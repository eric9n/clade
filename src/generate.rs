use gtdb_tree::node::Node;
use rusqlite::{params, params_from_iter, Connection, Result};
use std::path::PathBuf;

pub fn generate_newick_tree(db: &PathBuf) -> Result<String> {
    let mut newick = String::new();
    let conn = Connection::open(db).expect("Failed to open database");

    let mut stmt = conn.prepare(
        "select node, name, bootstrap, length, parent from gtdb_tree_archaea gta where parent = 0",
    )?;
    // Fetch a single row instead of using an iterator
    let node: Node = stmt.query_row(params![], |row| {
        let id: usize = row.get(0)?;
        let name: String = row.get(1)?;
        let bootstrap: f64 = row.get(2)?;
        let length: f64 = row.get(3)?;
        let parent: usize = row.get(4)?;
        Ok(Node {
            id,
            name,
            bootstrap,
            length,
            parent,
        })
    })?;

    build_tree(&mut newick, &node, &conn)?;

    Ok(newick)
}

fn build_tree(newick: &mut String, node: &Node, conn: &Connection) -> Result<()> {
    // Query to get children of the current node
    let mut stmt = conn.prepare(
        "SELECT node, name, bootstrap, length, parent FROM gtdb_tree_archaea WHERE parent = ?",
    )?;
    let child_iter = stmt.query_map(params![node.id], |row| {
        let id: usize = row.get(0)?;
        let name: String = row.get(1)?;
        let bootstrap: f64 = row.get(2)?;
        let length: f64 = row.get(3)?;
        let parent: usize = row.get(4)?;
        let node = Node {
            id,
            name,
            bootstrap,
            length,
            parent,
        };
        Ok(node)
    })?;

    let mut children = Vec::new();
    for child in child_iter {
        children.push(child?);
    }

    if !children.is_empty() {
        newick.push('(');
        for (i, child) in children.iter().enumerate() {
            build_tree(newick, child, conn)?;
            if i < children.len() - 1 {
                newick.push(',');
            }
        }
        newick.push_str(&format!("){}", node.name));
    }

    Ok(())
}

pub fn process_data(data: Vec<String>, db: &PathBuf) -> std::io::Result<Vec<String>> {
    let mut species = Vec::new();
    let mut ncbi_taxids = Vec::new();
    let mut ncbi_ids = Vec::new();

    let conn = Connection::open(db).expect("Failed to open database");
    let valid_prefixes = ["c__", "d__", "f__", "g__", "o__", "p__", "s__"];

    // Classify the input data
    for item in data {
        if valid_prefixes
            .iter()
            .any(|&prefix| item.starts_with(prefix))
        {
            species.push(item.to_string());
        } else if item.chars().all(char::is_numeric) {
            ncbi_taxids.push(item.to_string());
        } else if let Some(captures) = regex::Regex::new(r"(?:[A-Za-z]{2}_)?[A-Za-z]{3}_(\d+\.\d+)")
            .unwrap()
            .captures(&item)
        {
            let ncbi_id = captures.get(1).unwrap().as_str();
            ncbi_ids.push(ncbi_id.to_string());
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to process item: {}", item),
            ));
        }
    }

    let mut not_found = Vec::new();
    let mut results = Vec::new();

    if !species.is_empty() {
        let placeholders: Vec<String> = species.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT node FROM genome_taxonomy WHERE node IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = conn.prepare(&query).expect("Failed to prepare query");
        let rows: Vec<String> = stmt
            .query_map(
                params_from_iter(species.iter().map(|s| s.as_str())),
                |row| row.get::<_, String>(0),
            )
            .expect("Failed to query")
            .filter_map(Result::ok)
            .collect(); // Collect rows into a vector

        // Check if all species exist in the database
        if rows.len() != species.len() {
            // Calculate the difference between expected and found species
            let missing_species: Vec<&String> =
                species.iter().filter(|s| !rows.contains(s)).collect();

            not_found.extend(missing_species);
        } else {
            results.extend(species);
        }
    }

    // Batch query for ncbi_taxids
    if !ncbi_taxids.is_empty() {
        let placeholders: Vec<String> = ncbi_taxids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT ncbi_taxid, parent FROM genome_taxonomy WHERE ncbi_taxid IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = conn.prepare(&query).expect("Failed to prepare query");
        let rows: Vec<(String, String)> = stmt
            .query_map(
                params_from_iter(ncbi_taxids.iter().map(|s| s.as_str())),
                |row| {
                    let ncbi_taxid: String = row.get(0)?;
                    let parent: String = row.get(1)?;
                    Ok((ncbi_taxid, parent)) // Wrap the tuple in Ok
                },
            )
            .expect("Failed to query")
            .filter_map(Result::ok)
            .collect();

        if rows.len() != ncbi_taxids.len() {
            let selected: Vec<_> = rows.iter().map(|r| r.0.clone()).collect();
            let missing_species: Vec<&String> =
                ncbi_ids.iter().filter(|s| !selected.contains(s)).collect();
            not_found.extend(missing_species);
        } else {
            for row in rows {
                results.push(row.1);
            }
        }
    }

    // Batch query for ncbi_ids
    if !ncbi_ids.is_empty() {
        let placeholders: Vec<String> = ncbi_ids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT ncbi_id, parent FROM genome_taxonomy WHERE ncbi_id IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = conn.prepare(&query).expect("Failed to prepare query");
        let rows: Vec<(String, String)> = stmt
            .query_map(
                params_from_iter(ncbi_ids.iter().map(|s| s.as_str())),
                |row| {
                    let ncbi_id: String = row.get(0)?;
                    let parent: String = row.get(1)?;
                    Ok((ncbi_id, parent)) // Wrap the tuple in Ok
                },
            )
            .expect("Failed to query")
            .filter_map(Result::ok)
            .collect();

        if rows.len() != ncbi_ids.len() {
            let selected: Vec<_> = rows.iter().map(|r| r.0.clone()).collect();
            let missing_species: Vec<&String> =
                ncbi_ids.iter().filter(|s| !selected.contains(s)).collect();

            not_found.extend(missing_species);
        } else {
            for row in rows {
                results.push(row.1);
            }
        }
    }

    if !not_found.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Not found in the database. Missing: {}",
                not_found
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ")
            ),
        ));
    }

    Ok(results)
}
