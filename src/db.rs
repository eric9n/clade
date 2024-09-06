use rusqlite::{params, Connection, Result};

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
            node TEXT NOT NULL UNIQUE,
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

/// Inserts taxonomy data into the specified table in batch, ignoring conflicts.
pub fn batch_insert_taxonomy(
    conn: &mut Connection,
    taxonomies: &Vec<(String, String, Option<i64>, String, String, String, String)>,
) -> Result<()> {
    let tx = conn.transaction()?;

    {
        let mut stmt = tx.prepare(
            "
            INSERT OR IGNORE INTO genome_taxonomy
            (node, parent, ncbi_taxid, ancestor_sequence, ncbi_id, rank, domain)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ",
        )?;

        for taxonomy in taxonomies {
            stmt.execute(params![
                &taxonomy.0, // node
                &taxonomy.1, // parent
                taxonomy.2,  // ncbi_taxid
                &taxonomy.3, // ancestor_sequence
                &taxonomy.4, // ncbi_id
                &taxonomy.5, // rank
                &taxonomy.6, // domain
            ])?;
        }
    }

    tx.commit()?;
    Ok(())
}

/// Inserts GTDB tree data into the specified table in batch, ignoring conflicts.
pub fn batch_insert_gtdb_tree(
    conn: &mut Connection,
    table_name: &str,
    trees: &Vec<(usize, usize, String, f64, f64)>,
) -> Result<()> {
    let tx = conn.transaction()?;

    {
        let query = format!(
            "INSERT OR IGNORE INTO {} (node, parent, name, length, bootstrap) VALUES (?1, ?2, ?3, ?4, ?5)",
            table_name
        );

        let mut stmt = tx.prepare(&query)?;

        for tree in trees {
            stmt.execute(params![
                tree.0,  // node
                tree.1,  // parent
                &tree.2, // name
                tree.3,  // length
                tree.4,  // bootstrap
            ])?;
        }
    }

    tx.commit()?;
    Ok(())
}
