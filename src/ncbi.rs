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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taxonomy_structure() -> io::Result<()> {
        let taxo_path =
            PathBuf::from(std::env::var("TAXO_PATH").unwrap_or_else(|_| "taxo".to_string())); // 使用正确的 taxo 路径
        let (taxid_vec, parentid_vec, name_vec, rank_vec, parent_distances) = load(&taxo_path)?;

        // 检查所有向量的长度是否相同
        assert_eq!(taxid_vec.len(), parentid_vec.len());
        assert_eq!(taxid_vec.len(), name_vec.len());
        assert_eq!(taxid_vec.len(), rank_vec.len());
        assert_eq!(taxid_vec.len(), parent_distances.len());

        let root_index = taxid_vec
            .iter()
            .position(|id| id == "1")
            .expect("Root taxid not found");
        assert_eq!(
            parentid_vec[root_index], root_index,
            "Root should be its own parent"
        );
        assert_eq!(name_vec[root_index], "root", "Root should be named 'root'");
        assert_eq!(
            rank_vec[root_index], "no rank",
            "Root should have 'no rank'"
        );

        // 检查一些已知的分类关系
        // 例如，检查类（Homo sapiens）的分类
        if let Some(human_index) = taxid_vec.iter().position(|id| id == "9606") {
            assert_eq!(name_vec[human_index], "Homo sapiens");
            assert_eq!(rank_vec[human_index], "species");

            let parent_index = parentid_vec[human_index];
            assert_eq!(name_vec[parent_index], "Homo");
            assert_eq!(rank_vec[parent_index], "genus");
        } else {
            println!("Warning: Human taxid not found in test data");
        }

        // 添加更多的检查...

        Ok(())
    }
}
