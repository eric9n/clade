use std::collections::{HashMap, HashSet};

pub struct Taxonomy {
    pub taxid_vec: Vec<String>,
    pub parentid_vec: Vec<usize>,
    pub name_vec: Vec<String>,
    pub rank_vec: Vec<String>,
    pub parent_distances: Vec<f64>,
}

impl Taxonomy {
    pub fn new(
        taxid_vec: Vec<String>,
        parentid_vec: Vec<usize>,
        name_vec: Vec<String>,
        rank_vec: Vec<String>,
        parent_distances: Vec<f64>,
    ) -> Self {
        Taxonomy {
            taxid_vec,
            parentid_vec,
            name_vec,
            rank_vec,
            parent_distances,
        }
    }

    pub fn prune_by_taxids(&self, taxids: &[String]) -> Self {
        let mut keep_indices = HashSet::new();
        let taxid_to_index: HashMap<&String, usize> = self
            .taxid_vec
            .iter()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();

        // 找出所有需要保留的节点（包括祖先）
        for taxid in taxids {
            if let Some(&index) = taxid_to_index.get(taxid) {
                self.add_ancestors_to_keep(&mut keep_indices, index);
            }
        }

        // 创建新的修剪后的向量
        let mut new_taxid_vec = Vec::new();
        let mut new_parentid_vec = Vec::new();
        let mut new_name_vec = Vec::new();
        let mut new_rank_vec = Vec::new();
        let mut new_parent_distances = Vec::new();
        let mut old_to_new_index = HashMap::new();

        for (old_index, taxid) in self.taxid_vec.iter().enumerate() {
            if keep_indices.contains(&old_index) {
                let new_index = new_taxid_vec.len();
                old_to_new_index.insert(old_index, new_index);
                new_taxid_vec.push(taxid.clone());
                new_name_vec.push(self.name_vec[old_index].clone());
                new_rank_vec.push(self.rank_vec[old_index].clone());
                new_parentid_vec.push(0); // 临时值，稍后更新
                new_parent_distances.push(self.parent_distances[old_index]);
            }
        }

        // 更新父节点索引
        for (old_index, &new_index) in old_to_new_index.iter() {
            let old_parent_index = self.parentid_vec[*old_index];
            new_parentid_vec[new_index] = *old_to_new_index
                .get(&old_parent_index)
                .unwrap_or(&new_index);
        }

        Taxonomy::new(
            new_taxid_vec,
            new_parentid_vec,
            new_name_vec,
            new_rank_vec,
            new_parent_distances,
        )
    }

    pub fn prune_by_names(&self, names: &[String]) -> Self {
        let taxids: Vec<String> = names
            .iter()
            .filter_map(|name| self.name_vec.iter().position(|n| n == name))
            .map(|index| self.taxid_vec[index].clone())
            .collect();
        self.prune_by_taxids(&taxids)
    }

    fn add_ancestors_to_keep(&self, keep_indices: &mut HashSet<usize>, index: usize) {
        let mut current_index = index;
        while !keep_indices.contains(&current_index) {
            keep_indices.insert(current_index);
            let parent_index = self.parentid_vec[current_index];
            if parent_index == current_index {
                break; // 到达根节点
            }
            current_index = parent_index;
        }
    }

    pub fn to_newick(&self) -> String {
        let root_index = self
            .parentid_vec
            .iter()
            .position(|&p| p == self.parentid_vec[p])
            .expect("Root node not found");

        self.newick_recursive(root_index)
    }

    fn newick_recursive(&self, node_index: usize) -> String {
        let children: Vec<usize> = self
            .parentid_vec
            .iter()
            .enumerate()
            .filter(|&(i, &p)| p == node_index && i != node_index)
            .map(|(i, _)| i)
            .collect();

        if children.is_empty() {
            format!(
                "{}_{}_{}",
                self.name_vec[node_index],
                self.taxid_vec[node_index],
                self.parent_distances[node_index]
            )
        } else {
            let child_strings: Vec<String> = children
                .iter()
                .map(|&child_index| self.newick_recursive(child_index))
                .collect();

            format!(
                "({}){}_{}:{}",
                child_strings.join(","),
                self.name_vec[node_index],
                self.taxid_vec[node_index],
                if node_index == self.parentid_vec[node_index] {
                    0.0
                } else {
                    self.parent_distances[node_index]
                }
            )
        }
    }
}

pub fn prune_taxonomy(taxonomy: &Taxonomy, taxids: &[String]) -> Taxonomy {
    taxonomy.prune_by_taxids(taxids)
}

pub fn prune_taxonomy_by_names(taxonomy: &Taxonomy, names: &[String]) -> Taxonomy {
    taxonomy.prune_by_names(names)
}

pub fn taxonomy_to_newick(taxonomy: &Taxonomy) -> String {
    taxonomy.to_newick()
}
