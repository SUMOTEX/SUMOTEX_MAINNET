use tiny_keccak::{Keccak, Hasher};
use std::collections::HashMap;

type Hash = [u8; 32];

enum VerkleNode {
    Leaf(Vec<u8>, Vec<u8>),
    InnerNode(Vec<Hash>),
}

pub struct VerkleTree {
    root: Hash,
    nodes: HashMap<Hash, VerkleNode>,
}

impl VerkleTree {
    pub fn new() -> Self {
        let root_hash: Hash = [0; 32];
        let mut nodes = HashMap::new();
        nodes.insert(root_hash, VerkleNode::InnerNode(Vec::new()));
        VerkleTree {
            root: root_hash,
            nodes,
        }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let leaf = VerkleNode::Leaf(key.clone(), value);
        let leaf_hash = self.calculate_hash(&leaf);
        self.nodes.insert(leaf_hash, leaf);
        self.root = self.insert_recursive(&key, leaf_hash, self.root, 0);
    }

    fn insert_recursive(&mut self, key: &[u8], leaf_hash: Hash, current_hash: Hash, depth: usize) -> Hash {
        if depth == key.len() {
            return leaf_hash;
        }

        if let Some(VerkleNode::InnerNode(child_hashes)) = self.nodes.get(&current_hash) {
            let mut child_hashes = child_hashes.clone(); // Clone the Vec<[u8; 32]>
            let bit = (key[depth] >> 7) as usize;

            if bit >= child_hashes.len() {
                child_hashes.resize(bit + 1, [0; 32]);
            }

            child_hashes[bit] = self.insert_recursive(key, leaf_hash, child_hashes[bit], depth + 1);
            
            let new_hash = self.calculate_hash(&VerkleNode::InnerNode(child_hashes.clone()));
            self.nodes.insert(new_hash, VerkleNode::InnerNode(child_hashes));
            return new_hash;
        }

        current_hash
    }
    
    fn calculate_hash_static(node: &VerkleNode, nodes: &HashMap<Hash, VerkleNode>) -> Hash {
        let mut hasher = Keccak::v256();
        match node {
            VerkleNode::Leaf(key, value) => {
                hasher.update(key);
                hasher.update(value);
            }
            VerkleNode::InnerNode(child_hashes) => {
                for hash in child_hashes {
                    if let Some(child_node) = nodes.get(hash) {
                        let child_hash = VerkleTree::calculate_hash_static(child_node, nodes);
                        hasher.update(&child_hash);
                    }
                }
            }
        }
        let mut hash = [0; 32];
        hasher.finalize(&mut hash);
        hash
    }

    pub fn node_exists_with_root(&self, root_hash: Hash, key: &[u8]) -> bool {
        self.node_exists_recursive(root_hash, key, 0)
    }

    fn node_exists_recursive(&self, current_hash: Hash, key: &[u8], depth: usize) -> bool {
        if let Some(node) = self.nodes.get(&current_hash) {
            match node {
                VerkleNode::Leaf(leaf_key, _) if leaf_key == key => return true,
                VerkleNode::InnerNode(child_hashes) if depth < key.len() => {
                    let bit = (key[depth] >> 7) as usize; // Corrected line
                    if bit < child_hashes.len() {
                        return self.node_exists_recursive(child_hashes[bit], key, depth + 1);
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn calculate_hash(&self, node: &VerkleNode) -> Hash {
        let mut hasher = Keccak::v256();
        match node {
            VerkleNode::Leaf(key, value) => {
                hasher.update(key);
                hasher.update(value);
            }
            VerkleNode::InnerNode(child_hashes) => {
                for hash in child_hashes {
                    hasher.update(hash);
                }
            }
        }
        let mut hash = [0; 32];
        hasher.finalize(&mut hash);
        hash
    }

    pub fn get_root(&self) -> Hash {
        self.root
    }
    pub fn get_root_string(&self) -> String {
        hex::encode(&self.root)
    }
    pub fn print_nodes(&self) {
        println!("Nodes HashMap: {:?}", self.nodes);
    }
    
}

impl std::fmt::Debug for VerkleNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerkleNode::Leaf(key, value) => write!(f, "Leaf({:?}, {:?})", key, value),
            VerkleNode::InnerNode(child_hashes) => write!(f, "InnerNode({:?})", child_hashes),
        }
    }
}
