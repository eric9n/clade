use reqwest::blocking::get;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

/// Downloads a file from the given URL and saves it to the specified output path.
pub fn download_file(url: &str, output_path: &PathBuf) -> std::io::Result<()> {
    let mut attempts = 0;
    let mut response = loop {
        attempts += 1;
        match get(url) {
            Ok(resp) => break resp,
            Err(e) if attempts < 3 => {
                eprintln!("Attempt {} failed: {}. Retrying...", attempts, e);
                continue;
            }
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to download {} after {} attempts: {}",
                        url, attempts, e
                    ),
                ));
            }
        }
    };

    let mut file = BufWriter::new(File::create(&output_path).expect("Failed to create file")); // Use BufWriter for better performance
    response.copy_to(&mut file).expect("Failed to write file"); // Copy response directly to the file
    Ok(())
}
