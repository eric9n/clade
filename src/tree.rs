use rusqlite::{params, params_from_iter, Connection, Result};
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct Node {
    pub node: usize,
    pub parent: usize,
    pub name: String,
    pub length: f64,
    pub bootstrap: f64,
    pub rank: Option<String>,
    pub children: Vec<Node>,
}

impl Node {
    fn from_row(row: &rusqlite::Row<'_>) -> Result<Self> {
        Ok(Node {
            node: row.get(0)?,
            parent: row.get(1)?,
            name: row.get(2)?,
            length: row.get(3)?,
            bootstrap: row.get(4)?,
            rank: row.get(5)?,
            children: Vec::new(),
        })
    }
}

pub fn write_node_to_newick(node: &Node, newick: &mut String) {
    if !node.children.is_empty() {
        newick.push('(');
        for (i, child) in node.children.iter().enumerate() {
            if i > 0 {
                newick.push(',');
            }
            write_node_to_newick(child, newick);
        }
        newick.push(')');
    }

    // 写入节点名称, 如果name中包含; 则用+代替
    let rank = node.rank.clone().unwrap_or("".to_string());
    let name = if rank.starts_with("s__") {
        rank
    } else {
        node.name.clone()
    };

    let label = name.replace(';', "+").replace(' ', "_");
    write!(newick, "{}", label).unwrap();

    // 写入分支长度
    if node.length != 0.0 {
        write!(newick, ":{:.6}", node.length).unwrap();
    }

    // 写入 bootstrap 值
    if node.bootstrap != 0.0 {
        write!(newick, "[{:.2}]", node.bootstrap).unwrap();
    }
}

pub fn get_leaf_nodes_by_rank(
    conn: &Connection,
    table_name: &str,
    ranks: &Vec<String>,
) -> Result<Vec<usize>> {
    let placeholders = vec!["?"; ranks.len()].join(",");
    let query = format!(
        "SELECT node FROM {} WHERE rank IN ({})",
        table_name, placeholders
    );
    let mut stmt = conn.prepare(&query)?;

    let nodes_iter = stmt.query_map(params_from_iter(ranks), |row| row.get::<_, usize>(0))?;

    let mut nodes = Vec::new();
    for node_result in nodes_iter {
        nodes.push(node_result?);
    }

    Ok(nodes)
}

pub fn _build_pruned_tree(
    conn: &Connection,
    table_name: &str,
    node: usize,
    leaf_nodes: &Vec<usize>,
) -> Result<Option<Node>> {
    let mut stmt = conn.prepare(
        format!(
            "SELECT node, parent, name, length, bootstrap, rank FROM {} WHERE node = ?",
            table_name
        )
        .as_str(),
    )?;
    let mut current_node = stmt.query_row(params![node], |row| Node::from_row(row))?;

    let mut stmt =
        conn.prepare(format!("SELECT node FROM {} WHERE parent = ?", table_name).as_str())?;
    let children_iter = stmt.query_map(params![node], |row| row.get::<_, usize>(0))?;

    let mut keep_node = leaf_nodes.contains(&node);

    for child_result in children_iter {
        let child_id = child_result?;
        if let Some(child_node) = build_pruned_tree(conn, table_name, child_id, leaf_nodes)? {
            current_node.children.push(child_node);
            keep_node = true;
        }
    }

    if keep_node {
        Ok(Some(current_node))
    } else {
        Ok(None)
    }
}
pub fn build_pruned_tree(
    conn: &Connection,
    table_name: &str,
    node: usize,
    leaf_nodes: &Vec<usize>,
) -> Result<Option<Node>> {
    let mut stmt = conn.prepare(
        format!(
            "SELECT node, parent, name, length, bootstrap, rank FROM {} WHERE node = ?",
            table_name
        )
        .as_str(),
    )?;
    let mut current_node = stmt.query_row(params![node], |row| Node::from_row(row))?;

    let is_specified_leaf = leaf_nodes.contains(&node);

    let mut stmt =
        conn.prepare(format!("SELECT node FROM {} WHERE parent = ?", table_name).as_str())?;
    let children_iter = stmt.query_map(params![node], |row| row.get::<_, usize>(0))?;

    let mut keep_node = false;
    for child_result in children_iter {
        let child_id = child_result?;
        if let Some(child_node) = build_pruned_tree(conn, table_name, child_id, leaf_nodes)? {
            current_node.children.push(child_node);
            keep_node = true;
        }
    }

    if is_specified_leaf && current_node.children.is_empty() {
        // 如果是指定的叶子节点，并且没有子节点，则保留
        Ok(Some(current_node))
    } else if keep_node {
        // 如果有需要保留的子节点，则保留当前节点
        Ok(Some(current_node))
    } else {
        // 否则不保留
        Ok(None)
    }
}
