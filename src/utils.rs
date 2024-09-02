use reqwest::blocking::get;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

/// Downloads a file from the given URL and saves it to the specified output path.
pub fn download_file(url: &str, output_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut response = get(url).expect("Failed to download file");
    let mut file = BufWriter::new(File::create(&output_path).expect("Failed to create file")); // Use BufWriter for better performance
    response.copy_to(&mut file).expect("Failed to write file"); // Copy response directly to the file
    Ok(())
}
