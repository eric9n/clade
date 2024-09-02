use clade::ncbi;
use clade::taxo::{prune_taxonomy, prune_taxonomy_by_names, Taxonomy};
use clade::update::update_taxdump;
use clap::{Parser, Subcommand};
use std::{error::Error, fs::File, io::Write, path::PathBuf};

#[derive(clap::Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(subcommand)]
    pub cmd: Command,

    #[clap(short, long, help = "The path to store the taxdump files")]
    pub taxo_path: PathBuf,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[clap(about = "Update taxdump files")]
    Update,
    #[clap(about = "Generate and print taxonomy summary from taxdump files")]
    Generate,
    #[clap(about = "Prune the taxonomy tree and generate Newick format")]
    Prune {
        #[clap(short, long, help = "List of taxids to keep")]
        taxids: Option<Vec<String>>,
        #[clap(short, long, help = "List of names to keep")]
        names: Option<Vec<String>>,
        #[clap(short, long, help = "Output file path for Newick format")]
        output: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let taxo_path = args.taxo_path;

    let start = std::time::Instant::now();
    match args.cmd {
        Command::Update => update_taxdump(&taxo_path)?,
        Command::Generate => ncbi::print_taxonomy_summary(&taxo_path)?,
        Command::Prune {
            taxids,
            names,
            output,
        } => {
            let (taxid_vec, parentid_vec, name_vec, rank_vec, parent_distances) =
                ncbi::load(&taxo_path)?;
            let taxonomy = Taxonomy::new(
                taxid_vec,
                parentid_vec,
                name_vec,
                rank_vec,
                parent_distances,
            );

            let pruned_taxonomy = if let Some(taxids) = taxids {
                prune_taxonomy(&taxonomy, &taxids)
            } else if let Some(names) = names {
                prune_taxonomy_by_names(&taxonomy, &names)
            } else {
                return Err("Either taxids or names must be provided for pruning".into());
            };

            println!("Original taxonomy size: {}", taxonomy.taxid_vec.len());
            println!("Pruned taxonomy size: {}", pruned_taxonomy.taxid_vec.len());

            let newick = pruned_taxonomy.to_newick();
            let mut file = File::create(output)?;
            file.write_all(newick.as_bytes())?;
            println!("Pruned taxonomy in Newick format with distances written to file.");
        }
    }
    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);

    Ok(())
}
