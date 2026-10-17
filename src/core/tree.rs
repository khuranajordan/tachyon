use std::collections::BTreeMap;
use super::hash::Oid;
use super::index::Index;
use super::object::{ObjectStore, TreeEntry};

pub struct TreeBuilder;

impl TreeBuilder {
    /// Builds a hierarchical Git tree from the flat paths in the index and writes
    /// all necessary tree objects to `store`. Returns the root tree `Oid`.
    pub fn build_from_index(index: &Index, store: &ObjectStore) -> Result<Oid, String> {
        let mut root_node = TreeNode::Directory(BTreeMap::new());

        for (path, entry) in &index.entries {
            let parts: Vec<&str> = path.split('/').collect();
            Self::insert_path(&mut root_node, &parts, entry.mode, entry.oid)?;
        }

        Self::write_node(&root_node, store)
    }

    fn insert_path(
        current: &mut TreeNode,
        parts: &[&str],
        mode: u32,
        oid: Oid,
    ) -> Result<(), String> {
        if parts.is_empty() {
            return Ok(());
        }

        let name = parts[0].to_string();

        match current {
            TreeNode::Directory(children) => {
                if parts.len() == 1 {
                    children.insert(name, TreeNode::File(mode, oid));
                } else {
                    let entry = children.entry(name).or_insert_with(|| {
                        TreeNode::Directory(BTreeMap::new())
                    });
                    Self::insert_path(entry, &parts[1..], mode, oid)?;
                }
            }
            TreeNode::File(..) => {
                return Err("File path conflicts with existing directory".to_string());
            }
        }

        Ok(())
    }

    fn write_node(node: &TreeNode, store: &ObjectStore) -> Result<Oid, String> {
        match node {
            TreeNode::File(_, oid) => Ok(*oid),
            TreeNode::Directory(children) => {
                let mut entries = Vec::new();

                for (name, child) in children {
                    match child {
                        TreeNode::File(mode, oid) => {
                            entries.push(TreeEntry {
                                mode: *mode,
                                name: name.clone(),
                                oid: *oid,
                            });
                        }
                        TreeNode::Directory(..) => {
                            let child_tree_oid = Self::write_node(child, store)?;
                            entries.push(TreeEntry {
                                mode: 0o040000, // Directory mode
                                name: name.clone(),
                                oid: child_tree_oid,
                            });
                        }
                    }
                }

                store.write_tree(&entries)
            }
        }
    }

    /// Reads a tree recursively and returns a map of all files: `path -> (mode, oid)`
    pub fn read_tree_recursive(
        store: &ObjectStore,
        tree_oid: &Oid,
        prefix: &str,
    ) -> Result<BTreeMap<String, (u32, Oid)>, String> {
        let mut map = BTreeMap::new();
        let entries = store.parse_tree(tree_oid)?;

        for entry in entries {
            let path = if prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{}/{}", prefix, entry.name)
            };

            if entry.mode == 0o040000 {
                // Subtree
                let sub_map = Self::read_tree_recursive(store, &entry.oid, &path)?;
                map.extend(sub_map);
            } else {
                map.insert(path, (entry.mode, entry.oid));
            }
        }

        Ok(map)
    }
}

enum TreeNode {
    File(u32, Oid),
    Directory(BTreeMap<String, TreeNode>),
}
