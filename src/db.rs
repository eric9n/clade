use rusqlite::{params, Connection, Error, Result};

pub fn create_tables(conn: &Connection) -> Result<()> {
    create_genome_taxonomy_table(conn)?;
    create_gtdb_tree_tables(conn, &["archaea", "bacteria"])?;
    Ok(())
}

pub fn create_genome_taxonomy_table(conn: &Connection) -> Result<()> {
    conn.execute("DROP TABLE IF EXISTS genome_taxonomy", [])?;
    conn.execute(
        "CREATE TABLE genome_taxonomy (
            id INTEGER PRIMARY KEY,
            node TEXT NOT NULL,
            parent TEXT NOT NULL,
            ncbi_taxid INTEGER,
            ancestor_sequence TEXT NOT NULL,
            ncbi_id TEXT,
            rank TEXT NOT NULL,
            domain TEXT NOT NULL
        )",
        [],
    )?;
    // Create indexes for specified fields
    conn.execute(
        "CREATE INDEX idx_genome_taxonomy_node ON genome_taxonomy (node)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX idx_genome_taxonomy_parent ON genome_taxonomy (parent)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX idx_genome_taxonomy_ncbi_taxid ON genome_taxonomy (ncbi_taxid)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX idx_genome_taxonomy_ncbi_id ON genome_taxonomy (ncbi_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX idx_genome_taxonomy_rank ON genome_taxonomy (rank)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX idx_genome_taxonomy_domain ON genome_taxonomy (domain)",
        [],
    )?;

    println!("Created table: genome_taxonomy");
    Ok(())
}

pub fn create_gtdb_tree_tables(conn: &Connection, table_names: &[&str]) -> Result<()> {
    // Drop existing tables
    for table_name in table_names {
        conn.execute(
            &format!("DROP TABLE IF EXISTS gtdb_tree_{}", table_name),
            [],
        )?;
        create_gtdb_tree_table(conn, table_name)?;
    }

    Ok(())
}

fn create_gtdb_tree_table(conn: &Connection, table_name: &str) -> Result<()> {
    conn.execute(
        &format!(
            "CREATE TABLE gtdb_tree_{} (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            node INTEGER NOT NULL UNIQUE,
            parent INTEGER NOT NULL,
            name TEXT NOT NULL,
            length REAL DEFAULT 0.0,
            bootstrap REAL DEFAULT 0.0
        )",
            table_name
        ),
        [],
    )?;
    // Create index for node and parent in gtdb_tree_archaea
    conn.execute(
        &format!(
            "CREATE INDEX idx_gtdb_tree_{}_node ON gtdb_tree_{} (node)",
            table_name, table_name
        ),
        [],
    )?;
    conn.execute(
        &format!(
            "CREATE INDEX idx_gtdb_tree_{}_parent ON gtdb_tree_{} (parent)",
            table_name, table_name
        ),
        [],
    )?;

    println!("Created table: gtdb_tree_{}", table_name);
    Ok(())
}

/// Inserts taxonomy data into the specified table.
pub fn insert_taxonomy(
    conn: &Connection,
    domain: &str,
    node_id: &str,
    parent: &str,
    ncbi_taxid: Option<i64>,
    ancestor_sequence: &str,
    ncbi_id: &str,
    rank: &str,
) -> Result<()> {
    let query = "INSERT INTO genome_taxonomy (node, parent, ncbi_taxid, ancestor_sequence, ncbi_id, rank, domain)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

    conn.execute(
        query,
        params![
            node_id,
            parent,
            ncbi_taxid,
            ancestor_sequence,
            ncbi_id,
            rank,
            domain
        ],
    )?;
    Ok(())
}

/// Inserts gtdb_tree data into the specified table.
pub fn insert_gtdb_tree(
    conn: &Connection,
    table_name: &str,
    node: usize,
    parent: usize,
    name: &str,
    length: f64,
    bootstrap: f64,
) -> Result<()> {
    let query = format!(
        "INSERT INTO {} (node, parent, name, length, bootstrap) VALUES (?1, ?2, ?3, ?4, ?5)",
        table_name
    );

    conn.execute(&query, params![node, parent, name, length, bootstrap])?;
    Ok(())
}

pub fn node_exists(conn: &Connection, domain: &str, node: &str) -> Result<bool> {
    let query = "SELECT 1 FROM genome_taxonomy WHERE domain = ?1 AND node = ?2 LIMIT 1";
    match conn.query_row(query, params![domain, node], |_| Ok(())) {
        Ok(_) => Ok(true),
        Err(Error::QueryReturnedNoRows) => Ok(false),
        Err(Error::SqliteFailure(_, Some(msg))) if msg.contains("no such column: node") => {
            Ok(false)
        }
        Err(e) => Err(e),
    }
}
