# Clade

A tool for phylogenetic tree construction and pruning based on NCBI taxonomy data and GTDB data.

## Features

1. Fetch and process NCBI taxonomy data
2. Fetch and process GTDB (Genome Taxonomy Database) data
3. Parse taxonomy data into efficient vector structures
4. Prune phylogenetic trees based on user input
5. Generate Newick format output from pruned trees

## Workflow

1. **Data Retrieval**:
   - Fetch the latest taxonomy data from NCBI
   - Fetch the latest tree and taxonomy data from GTDB
2. **Data Processing**:
   - Decompress the downloaded data
   - Parse the taxonomy information into five synchronized vectors:
     - taxid vector
     - parentid vector
     - name vector
     - rank vector
     - parent_distances vector
   - Each vector has the same length, and their indices correspond to each other
3. **Tree Pruning and Newick Generation**:
   - Accept user input in the form of:
     - A list of taxids, or
     - A list of taxonomic names
   - Prune the phylogenetic tree to include only the branches related to the input
   - Generate a Newick format file representing the pruned phylogenetic tree

## Usage

The Clade tool supports the following commands:

1. `update`: Update taxdump files from NCBI
2. `generate`: Generate and print taxonomy summary from taxdump files
3. `prune`: Prune the taxonomy tree based on given taxids or names and generate a Newick format file
4. `gtdb`: Download and process GTDB data

Example usage:

## Installation

[Add installation instructions here]

## Contributing

[Add contribution guidelines here]

## License

[Add license information here]
