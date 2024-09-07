use crate::utils::download_file;
use chrono::{Datelike, NaiveDate};
use flate2::read::GzDecoder;
use gtdb_tree;
use regex::Regex;
use reqwest::blocking::Client;
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::fs::{self, File};
use std::io::{self, BufRead, Read};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub version: String,
    pub date: NaiveDate,
    pub sub_versions: Vec<SubVersionInfo>,
}

#[derive(Debug, Clone)]
pub struct SubVersionInfo {
    pub version: String,
    pub date: NaiveDate,
    pub url: String,
}

/// List all GTDB release versions and their sub-versions, and save to file if not exists.
pub fn list_releases(
    should_print: bool,
    target_sub_version: Option<String>,
) -> Result<SubVersionInfo, Box<dyn std::error::Error>> {
    let client = Client::new();
    let base_url = "https://data.gtdb.ecogenomic.org/releases/";
    let response = client.get(base_url).send()?.text()?;

    let re = Regex::new(
        r#"<tr>\s*<td><img[^>]*></td>\s*<td class="n">\s*<a href="([^"]+)/">[^<]*</a>\s*/\s*</td>\s*<td class="m">([^<]+)</td>"#,
    )?;

    let mut sub_versions = Vec::new();

    let mut releases: HashMap<String, ReleaseInfo> = re
        .captures_iter(&response)
        .filter_map(|cap| {
            let version = cap.get(1)?.as_str().to_string();
            let date_str = cap.get(2)?.as_str().trim();
            NaiveDate::parse_from_str(date_str, "%Y-%m-%d %H:%M")
                .ok()
                .map(|date| {
                    (
                        version.clone(),
                        ReleaseInfo {
                            version,
                            date,
                            sub_versions: Vec::new(),
                        },
                    )
                })
        })
        .filter(|(version, info)| version.starts_with("release") && info.date.year() >= 2021)
        .collect();

    for (version, info) in releases.iter_mut() {
        let release_url = format!("{}{}/", base_url, version);
        let sub = get_sub_versions(&client, &release_url)?;
        sub_versions.extend(sub.clone());
        info.sub_versions = sub.clone();
    }
    let mut sorted_releases: Vec<_> = releases.into_iter().collect();
    sorted_releases.sort_by(|a, b| b.1.date.cmp(&a.1.date));

    if should_print {
        print_releases(&sorted_releases);
    }

    let sub_version_info = if let Some(sub_version) = target_sub_version {
        sub_versions
            .iter()
            .find(|v| v.version == sub_version)
            .ok_or(format!("Sub-version {} not found", sub_version))?
            .clone() // Clone the URL to avoid moving out of the reference
    } else {
        sub_versions
            .iter()
            .max_by_key(|v| v.date)
            .ok_or("No sub-versions available")?
            .clone()
    };

    Ok(sub_version_info)
}

/// Get sub-versions for a specific release.
fn get_sub_versions(
    client: &Client,
    url: &str,
) -> Result<Vec<SubVersionInfo>, Box<dyn std::error::Error>> {
    let response = client.get(url).send()?.text()?;

    let re = Regex::new(
        r#"<tr>\s*<td><img[^>]*></td>\s*<td class="n">\s*<a href="([^"]+)/">[^<]*</a>\s*/\s*</td>\s*<td class="m">([^<]+)</td>"#,
    )?;

    let mut sub_versions: Vec<SubVersionInfo> = re
        .captures_iter(&response)
        .filter_map(|cap| {
            let version = cap.get(1)?.as_str().to_string();
            let date_str = cap.get(2)?.as_str().trim();
            NaiveDate::parse_from_str(date_str, "%Y-%m-%d %H:%M")
                .ok()
                .map(|date| SubVersionInfo {
                    version: version.clone(),
                    date,
                    url: format!("{}{}/", url, version),
                })
        })
        .collect();

    sub_versions.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(sub_versions)
}

#[derive(Debug)]
pub enum DomainFile {
    ArTree(String),
    BacTree(String),
    ArMetadata(String),
    BacMetadata(String),
}

pub fn get_sub_version_files(
    sub_version_url: &str,
) -> Result<Vec<DomainFile>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get(sub_version_url).send()?.text()?;

    let re = Regex::new(r#"<a class="plausible-event-name=Download" href="([^"]+)">"#)?;

    let mut files = HashMap::new();
    for cap in re.captures_iter(&response) {
        let file_name = cap.get(1).unwrap().as_str();
        if file_name.ends_with(".tree") || file_name.contains("metadata") {
            files.insert(
                file_name.to_string(),
                format!("{}{}", sub_version_url, file_name),
            );
        }
    }

    let ar_tree = files
        .iter()
        .find(|(k, _)| k.starts_with("ar") && k.ends_with(".tree"))
        .map(|(_, v)| v.clone())
        .ok_or("AR tree file not found")?;

    let bac_tree = files
        .iter()
        .find(|(k, _)| k.starts_with("bac") && k.ends_with(".tree"))
        .map(|(_, v)| v.clone())
        .ok_or("BAC tree file not found")?;

    let ar_metadata = files
        .iter()
        .find(|(k, _)| k.starts_with("ar") && k.contains("metadata"))
        .map(|(_, v)| v.clone())
        .ok_or("AR metadata file not found")?;

    let bac_metadata = files
        .iter()
        .find(|(k, _)| k.starts_with("bac") && k.contains("metadata"))
        .map(|(_, v)| v.clone())
        .ok_or("BAC metadata file not found")?;

    Ok(vec![
        DomainFile::ArTree(ar_tree),
        DomainFile::BacTree(bac_tree),
        DomainFile::ArMetadata(ar_metadata),
        DomainFile::BacMetadata(bac_metadata),
    ])
}

pub fn parse_domain_files(dir_path: &PathBuf) -> Result<Vec<DomainFile>, std::io::Error> {
    let mut domain_files = Vec::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                let file_path = path.to_string_lossy().into_owned();

                if file_name.starts_with("ar") && file_name.ends_with(".tree") {
                    domain_files.push(DomainFile::ArTree(file_path));
                } else if file_name.starts_with("bac") && file_name.ends_with(".tree") {
                    domain_files.push(DomainFile::BacTree(file_path));
                } else if file_name.starts_with("ar") && file_name.contains("metadata") {
                    domain_files.push(DomainFile::ArMetadata(file_path));
                } else if file_name.starts_with("bac") && file_name.contains("metadata") {
                    domain_files.push(DomainFile::BacMetadata(file_path));
                }
            }
        }
    }

    if domain_files.len() != 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Domain files not found",
        ));
    }
    Ok(domain_files)
}

/// Print releases to console.
fn print_releases(releases: &Vec<(String, ReleaseInfo)>) {
    println!("GTDB Release versions and sub-versions:");
    for (_, info) in releases {
        println!("{} ({})", info.version, info.date.format("%Y-%m-%d"));
        for sub_version in &info.sub_versions {
            println!(
                "  - {} ({}) - {}",
                sub_version.version,
                sub_version.date.format("%Y-%m-%d"),
                sub_version.url
            );
        }
        println!(); // Add a blank line between releases
    }
}

/// Downloads and extracts GTDB data files.
pub fn download_gtdb_data(taxo_path: &PathBuf, files: &Vec<DomainFile>) -> io::Result<()> {
    if !taxo_path.exists() {
        fs::create_dir_all(taxo_path)?;
    }

    for domain_file in files.iter() {
        let url = match domain_file {
            DomainFile::ArTree(url) => url,
            DomainFile::BacTree(url) => url,
            DomainFile::ArMetadata(url) => url,
            DomainFile::BacMetadata(url) => url,
        };
        let file_name = url.split('/').last().unwrap();
        let output_path = taxo_path.join(file_name);
        download_file(url, &output_path)?;

        // If the file is a .gz file, decompress it
        if file_name.ends_with(".gz") {
            if file_name.ends_with(".tar.gz") {
                let tar_gz_file = File::open(&output_path)?;
                let mut archive = tar::Archive::new(GzDecoder::new(tar_gz_file));
                archive.unpack(&taxo_path)?; // Extract to the specified directory
            } else {
                let gz_file = File::open(&output_path)?;
                let mut gz_decoder = GzDecoder::new(gz_file);
                let decompressed_file_name = file_name.trim_end_matches(".gz");
                let decompressed_path = taxo_path.join(decompressed_file_name);
                let mut decompressed_file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&decompressed_path)?;
                io::copy(&mut gz_decoder, &mut decompressed_file)?;
            }
            fs::remove_file(&output_path)?; // Remove the .gz file after extraction
        }
    }

    Ok(())
}

/// Parses the metadata files and inserts data into the SQLite database.
pub fn parse_metadata(db: &PathBuf, domain_files: &Vec<DomainFile>) -> io::Result<()> {
    println!("Parsing metadata");
    let mut conn = Connection::open(db).expect("failed to open database");
    // Create tables if they don't exist
    crate::db::create_genome_taxonomy_table(&mut conn).expect("failed to create tables");

    for domain_file in domain_files.iter() {
        let (path, domain) = match domain_file {
            DomainFile::ArMetadata(path) => (path, "archaea"),
            DomainFile::BacMetadata(path) => (path, "bacteria"),
            _ => continue,
        };
        // Truncate the table before inserting new data
        conn.execute("DELETE FROM genome_taxonomy WHERE domain = ?1", [domain])
            .expect(&format!("Failed to truncate table {}", domain));

        let file = File::open(path)?;
        let reader = io::BufReader::new(file);
        let mut lines = reader.lines();

        // Read and parse the header
        let header = lines
            .next()
            .expect("Empty file")
            .expect("Failed to read line");
        let header_fields: Vec<&str> = header.split('\t').collect();

        // Find the indices of the columns we're interested in
        let accession_index = header_fields
            .iter()
            .position(|&r| r == "accession")
            .expect("Accession column not found");
        let taxonomy_index = header_fields
            .iter()
            .position(|&r| r == "gtdb_taxonomy")
            .expect("GTDB taxonomy column not found");
        let taxid_index = header_fields
            .iter()
            .position(|&r| r == "ncbi_taxid")
            .expect("NCBI taxid column not found");

        let mut taxonomies = Vec::new();
        for line in lines {
            let line = line?;
            let fields: Vec<&str> = line.split('\t').collect();

            let accession = fields[accession_index];
            let gtdb_taxonomy = fields[taxonomy_index]; // gtdb_taxonomy is at index 19
            let ncbi_taxid: Option<i64> = fields[taxid_index].parse().ok(); // Assuming ncbi_taxid is at index 84

            // Parse gtdb_taxonomy
            let taxonomy_parts: Vec<&str> = gtdb_taxonomy.split(';').collect();
            let mut ancestor_sequence = String::new();

            // First part: process taxonomy_parts
            for (i, part) in taxonomy_parts.iter().enumerate() {
                let node = *part;
                let parent = if i > 0 { taxonomy_parts[i - 1] } else { "root" };
                let rank = &node[..3];

                // Directly push the record to the vector
                taxonomies.push((
                    node.to_string(),
                    parent.to_string(),
                    None::<i64>, // ncbi_taxid is None for internal nodes
                    ancestor_sequence.clone(),
                    "".to_string(),
                    rank.to_string(),
                    domain.to_string(),
                ));

                ancestor_sequence.push_str(node);
                ancestor_sequence.push(';');
            }

            // Second part: process accession
            let ncbi_id = accession.split('_').last().unwrap_or("");
            taxonomies.push((
                accession.to_string(),
                taxonomy_parts.last().unwrap().to_string(),
                ncbi_taxid,
                ancestor_sequence.clone(),
                ncbi_id.to_string(),
                "no rank".to_string(),
                domain.to_string(),
            ));

            // Batch insert every 1000 records
            if taxonomies.len() >= 1000 {
                crate::db::batch_insert_taxonomy(&mut conn, &taxonomies)
                    .expect("batch insert taxonomy failed");
                taxonomies.clear(); // Clear the vector after batch insert
            }
        }
        // Insert any remaining records
        if !taxonomies.is_empty() {
            crate::db::batch_insert_taxonomy(&mut conn, &taxonomies)
                .expect("batch insert taxonomy failed");
        }
    }

    Ok(())
}

/// Parses the tree files and inserts data into the SQLite database.
pub fn parse_tree(db: &PathBuf, domain_files: &Vec<DomainFile>) -> io::Result<()> {
    println!("Parsing tree");
    let mut conn = Connection::open(db).expect("failed to open database");

    crate::db::create_gtdb_tree_tables(&conn, &["archaea", "bacteria"])
        .expect("failed to create tables");

    for domain_file in domain_files.iter() {
        let (file_path, table_name) = match domain_file {
            DomainFile::ArTree(path) => (path, "gtdb_tree_archaea"),
            DomainFile::BacTree(path) => (path, "gtdb_tree_bacteria"),
            _ => continue,
        };
        let file = File::open(&file_path)?;
        let mut reader = io::BufReader::new(file);
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer)?;

        // Process each line in the tree file
        let nodes = gtdb_tree::tree::parse_tree(&buffer).expect("Failed to parse tree");
        conn.execute(&format!("DELETE FROM {}", table_name), [])
            .expect(&format!("Failed to truncate table {}", table_name));

        let mut batch = Vec::new();
        for node in nodes {
            batch.push((node.id, node.parent, node.name, node.length, node.bootstrap));
            // Batch insert every 1000 records
            if batch.len() >= 1000 {
                crate::db::batch_insert_gtdb_tree(&mut conn, table_name, &batch)
                    .expect("Failed to batch insert gtdb_tree nodes");
                batch.clear(); // Clear the batch after insertion
            }
        }
        // Insert any remaining records
        if !batch.is_empty() {
            crate::db::batch_insert_gtdb_tree(&mut conn, table_name, &batch)
                .expect("Failed to batch insert gtdb_tree nodes");
        }
        crate::db::update_gtdb_tree_species(&mut conn, table_name)
            .expect("Failed to update gtdb_tree species");
    }

    Ok(())
}
