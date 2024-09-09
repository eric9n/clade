use rusqlite::{params_from_iter, Connection, Result};
use std::path::PathBuf;

pub fn generate_newick_tree(db: &PathBuf, input_data: Vec<String>, domain: &str) -> Result<String> {
    let conn = Connection::open(db).expect("Failed to open database");

    let ranks = process_data(input_data, db).expect("Failed to process data");

    let table_name = format!("gtdb_tree_{domain}");
    let leaf_nodes = crate::tree::get_leaf_nodes_by_rank(&conn, &table_name, &ranks)?;
    let newick_tree = crate::tree::build_pruned_tree(&conn, &table_name, 1, &leaf_nodes)?;

    if let Some(root) = newick_tree {
        let mut newick = String::new();
        crate::tree::write_node_to_newick(&root, &mut newick);
        newick.push(';');
        Ok(newick)
    } else {
        Ok("".into())
    }
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
