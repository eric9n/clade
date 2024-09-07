use clade::gtdb::{
    download_gtdb_data, get_sub_version_files, list_releases, parse_domain_files, parse_metadata,
    parse_tree,
};
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
    #[clap(about = "Update NCBI taxdump files")]
    Update,
    #[clap(about = "GTDB related operations")]
    Gtdb {
        #[clap(subcommand)]
        subcmd: GtdbSubCommand,
    },
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

#[derive(Subcommand, Debug)]
enum GtdbSubCommand {
    #[clap(about = "List all GTDB release versions")]
    List,
    #[clap(about = "Download GTDB data files and parse metadata")]
    #[clap(about = "Download and parse GTDB data")]
    Sync {
        #[clap(
            long = "version",
            help = "The version of the GTDB release to download and parse"
        )]
        version: Option<String>,
    },
    Download {
        #[clap(
            long = "version",
            help = "The ersion of the GTDB release to download, default to the latest version, format: 220.0, 214.0, etc. must be a sub-version"
        )]
        version: Option<String>,
    },
    #[clap(about = "Parse GTDB metadata and create database")]
    Parse {
        #[clap(long = "version", help = "The version of the GTDB release to parse")]
        version: String,
    },
    #[clap(about = "Generate Newick format from GTDB database")]
    Newick {
        #[clap(short, long, help = "GTDB tree version to generate Newick format")]
        version: String,

        #[clap(
            short,
            long,
            help = "Input file path to the data that needs to be analyzed, format: GCF_7312312.0,s__Fen731 sp002068775"
        )]
        input_file: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let taxo_path = args.taxo_path;

    let start = std::time::Instant::now();
    match args.cmd {
        Command::Update => update_taxdump(&taxo_path)?,
        Command::Gtdb { subcmd } => match subcmd {
            GtdbSubCommand::Sync { version } => {
                let sub_version_info = list_releases(false, version)?;
                println!(
                    "Downloading sub-version: {}, url: {}",
                    &sub_version_info.version, sub_version_info.url
                );
                let sub_version_path = taxo_path.join(&sub_version_info.version);
                let files = get_sub_version_files(&sub_version_info.url)?;

                download_gtdb_data(&sub_version_path, &files)?;
                let domain_files = parse_domain_files(&sub_version_path)?;
                let db = taxo_path.join(format!("{}.db", sub_version_info.version));
                parse_metadata(&db, &domain_files)?;
                parse_tree(&db, &domain_files)?;
            }
            GtdbSubCommand::Download { version } => {
                let sub_version_info = list_releases(false, version)?;
                println!(
                    "Downloading sub-version: {}, url: {}",
                    sub_version_info.version, sub_version_info.url
                );
                let sub_version_path = taxo_path.join(sub_version_info.version);
                let files = get_sub_version_files(&sub_version_info.url)?;

                download_gtdb_data(&sub_version_path, &files)?;
            }
            GtdbSubCommand::Parse { version } => {
                let db = taxo_path.join(format!("{version}.db"));
                let domain_files = parse_domain_files(&taxo_path.join(version))?;
                parse_metadata(&db, &domain_files)?;
                parse_tree(&db, &domain_files)?;
            }
            GtdbSubCommand::List => {
                let _ = list_releases(true, None)?;
            }
            GtdbSubCommand::Newick {
                version,
                input_file,
            } => {
                let db = taxo_path.join(format!("{version}.db"));
                println!("Generating Newick format for GTDB version: {version}");
                // Read the input file and parse it into a list based on commas
                let data = std::fs::read_to_string(&input_file)?
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<String>>();

                let newick = clade::generate::process_data(data, &db)?;
                println!("{:?}", newick);
            }
        },
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
