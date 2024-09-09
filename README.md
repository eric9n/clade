# Clade

A tool for phylogenetic tree construction and pruning based on NCBI taxonomy data and GTDB (Genome Taxonomy Database) data.

## Features

1. Fetch and process NCBI taxonomy data
2. Fetch and process GTDB data
3. Parse taxonomy data into efficient vector structures
4. Prune phylogenetic trees based on user input
5. Generate Newick format output from pruned trees
6. Support for both NCBI and GTDB data sources


## Installation

### Homebrew
```
brew install eric9n/tap/clade
```

## Usage

The Clade tool supports the following commands:

1. `update`: Update NCBI taxdump files
2. `gtdb`: GTDB related operations
   - `list`: List all GTDB release versions
   - `sync`: Download GTDB data files and parse metadata
   - `download`: Download GTDB data files
   - `parse`: Parse GTDB metadata and create database
   - `newick`: Generate Newick format from GTDB database
3. `generate`: Generate and print taxonomy summary from taxdump files
4. `prune`: Prune the taxonomy tree and generate Newick format

### Examples

1. Update NCBI taxdump files:
   ```
   clade -t /path/to/taxo update
   ```

2. List GTDB release versions:
   ```
   clade -t /path/to/taxo gtdb list
   ```

3. Download and parse GTDB data:
   ```
   clade -t /path/to/taxo gtdb sync --version 220.0
   ```

4. Generate Newick format from GTDB database:
   ```
   clade -t /path/to/taxo gtdb newick --version 220.0 --domain bacteria --input input.txt --output output.newick
   ```

5. Prune taxonomy tree:
   ```
   clade -t /path/to/taxo prune --taxids 9606,9605 --output pruned.newick
   ```

## Workflow

1. **Data Retrieval**:
   - Fetch the latest taxonomy data from NCBI
   - Fetch the latest tree and taxonomy data from GTDB
2. **Data Processing**:
   - Decompress the downloaded data
   - Parse the taxonomy information into efficient vector structures
3. **Tree Pruning and Newick Generation**:
   - Accept user input in the form of taxids or taxonomic names
   - Prune the phylogenetic tree to include only the branches related to the input
   - Generate a Newick format file representing the pruned phylogenetic tree
