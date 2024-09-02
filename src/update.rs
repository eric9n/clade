use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;

const TAXDUMP_URL: &str = "https://ftp.ncbi.nlm.nih.gov/pub/taxonomy/taxdump.tar.gz";
const ETAG_FILE: &str = "etag.txt";
const TAXDUMP_FILE: &str = "taxdump.tar.gz";

fn get_etag(response: &reqwest::blocking::Response) -> String {
    response
        .headers()
        .get(reqwest::header::ETAG)
        .map(|tag| tag.to_str().unwrap_or(""))
        .unwrap_or("")
        .to_string()
}

/// Updates the taxdump files if the local version is outdated or does not exist.
///
/// This function checks the ETag of the remote taxdump file against the local ETag.
/// If the local ETag does not match the remote or does not exist, it downloads the
/// latest taxdump file, updates the local ETag, and extracts specific files from
/// the archive.
///
/// # Arguments
///
/// * `taxo_path` - A string slice that represents the path where the taxdump files
///   should be stored.
///
/// # Errors
///
/// This function will return an error if:
/// - The directory `taxo_path` cannot be created or accessed.
/// - The HTTP request to fetch the taxdump file fails.
/// - The taxdump file cannot be created or written to.
/// - The ETag file cannot be written to.
/// - The taxdump archive cannot be opened or read.
/// - The specific files within the archive cannot be extracted or written to.
pub fn update_taxdump(taxo_path: &PathBuf) -> io::Result<()> {
    // Ensure the taxo directory exists
    if !std::path::Path::new(taxo_path).exists() {
        fs::create_dir_all(taxo_path)?;
    }

    let client = Client::new();
    let response = client.head(TAXDUMP_URL).send().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("HTTP HEAD request failed: {}", e),
        )
    })?;

    let remote_etag = get_etag(&response);

    let etag_file_path = taxo_path.join(ETAG_FILE);
    let local_etag = if etag_file_path.exists() {
        fs::read_to_string(&etag_file_path).expect("Failed to read etag file")
    } else {
        String::new()
    };

    if local_etag == remote_etag
        && taxo_path.join("names.dmp").exists()
        && taxo_path.join("nodes.dmp").exists()
    {
        println!("Taxdump is up to date.");
        return Ok(());
    }

    println!("Updating taxdump...");
    let mut response = client
        .get(TAXDUMP_URL)
        .send()
        .expect("Failed to download taxdump");
    let taxdump_file_path = taxo_path.join(TAXDUMP_FILE);
    let mut file =
        BufWriter::new(File::create(&taxdump_file_path).expect("Failed to create taxdump file"));

    response
        .copy_to(&mut file)
        .expect("Failed to write taxdump file");
    fs::write(&etag_file_path, remote_etag).expect("Failed to write etag file");

    let taxdump_file_path = taxo_path.join(TAXDUMP_FILE);
    let tar_gz = File::open(&taxdump_file_path).expect("Failed to open taxdump file");
    let tar = GzDecoder::new(BufReader::new(tar_gz));
    let mut archive = tar::Archive::new(tar);

    for entry in archive
        .entries()
        .expect("Failed to get entries from archive")
    {
        let mut entry = entry.expect("Failed to get entry from archive");
        let path = entry.path().expect("Failed to get path from entry");
        if path.ends_with("names.dmp") || path.ends_with("nodes.dmp") {
            let output_file_path = PathBuf::from(taxo_path).join(path);
            entry
                .unpack(output_file_path)
                .expect("Failed to unpack file");
        }
    }

    fs::remove_file(&taxdump_file_path).expect("Failed to remove taxdump file");

    println!("Update completed.");
    Ok(())
}
