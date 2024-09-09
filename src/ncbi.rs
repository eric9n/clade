use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;

pub fn load(
    taxo_path: &PathBuf,
) -> io::Result<(Vec<String>, Vec<usize>, Vec<String>, Vec<String>, Vec<f64>)> {
    let names_path = taxo_path.join("names.dmp");
    let nodes_path = taxo_path.join("nodes.dmp");

    let names_file = File::open(names_path)?;
    let nodes_file = File::open(nodes_path)?;

    let names_reader = BufReader::new(names_file);
    let nodes_reader = BufReader::new(nodes_file);

    let mut taxid_to_index = HashMap::new();
    let mut taxid_vec = Vec::new();
    let mut parent_taxid_vec = Vec::new();
    let mut name_vec = Vec::new();
    let mut rank_vec = Vec::new();
    let mut parent_distances = Vec::new();

    // Read nodes.dmp file
    for (index, line) in nodes_reader.lines().enumerate() {
        let line = line?;
        let parts: Vec<&str> = line.split("\t|\t").collect();
        if parts.len() >= 3 {
            let taxid = parts[0].to_string();
            let parent_taxid = parts[1].to_string();
            let rank = parts[2].to_string();

            taxid_to_index.insert(taxid.clone(), index);
            taxid_vec.push(taxid);
            parent_taxid_vec.push(parent_taxid);
            rank_vec.push(rank);
            name_vec.push(String::new()); // Initialize with empty string
            parent_distances.push(1.0); // Default distance
        }
    }

    // Read names.dmp file
    for line in names_reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 7 && parts[6] == "scientific name" {
            let taxid = parts[0].to_string();
            if let Some(&index) = taxid_to_index.get(&taxid) {
                name_vec[index] = parts[2].to_string();
            }
        }
    }

    // Convert parent_taxid_vec to parentid_vec using indices
    let parentid_vec: Vec<usize> = parent_taxid_vec
        .iter()
        .map(|parent_taxid| taxid_to_index.get(parent_taxid).cloned().unwrap_or(0))
        .collect();

    Ok((
        taxid_vec,
        parentid_vec,
        name_vec,
        rank_vec,
        parent_distances,
    ))
}

pub fn print_taxonomy_summary(taxo_path: &PathBuf) -> io::Result<()> {
    let (taxid_vec, parentid_vec, name_vec, rank_vec, parent_distances) = load(taxo_path)?;

    println!("Generated taxonomy summary:");
    println!("Number of taxa: {}", taxid_vec.len());
    println!("First 5 entries:");
    for i in 0..5.min(taxid_vec.len()) {
        println!(
            "Index: {}, TaxID: {}, ParentIndex: {}, Name: {}, Rank: {}, ParentDistance: {}",
            i, taxid_vec[i], parentid_vec[i], name_vec[i], rank_vec[i], parent_distances[i]
        );
    }

    Ok(())
}
