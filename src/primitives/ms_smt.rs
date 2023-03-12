//! A Merkle Sum, Sparse Merkle Tree is an authenticated data structure that allows key-data
//! storage in an efficient way. This type of tree allows proof of no-inclusion and data
//! commitment, useful for verifying whether there is no inflation during fungible assets
//! transfer.
//!
//! The Merkle Sum part of it means that each node commits to a data, and as you transverse
//! the tree towards the root, you add the data committed in the adjacent leaves, and
//! at the end you should have the exact same amount as committed in the root node.
//! Sparseness, on the other hands, means that this tree virtually have all 2^256 nodes, and
//! they are inserted in a deterministic way. Since we known where something should be, if
//! we find out that this position is e.g empty, we known that a given node isn't in the tree.
//! # Usage
//! ### Inserting
//! ```
//!   use rust_taro::primitives::ms_smt::Tree;
//!   let mut tree = Tree::new();
//! ```
//! ### Adding new elements
//! ```
//!  use rust_taro::primitives::{ms_smt::Tree, key::Key};
//!  let key = Key::from(0);
//!  let sum = 10;
//!  // This represents an empty value
//!  let value = [0; 32];
//!  let mut tree = Tree::new();
//!
//!  tree.insert(key, value.into(), sum).unwrap();
//!
//!  let root_hash = &tree.root_hash();
//!  assert_eq!(
//!          root_hash.to_string().as_str(),
//!          "0000000000000000000000000000000000000000000000000000000000000000"
//!  );
//! ```
//!
//! ### Deleting elements
//! ```
//!  use rust_taro::primitives::{ms_smt::Tree, key::Key};
//!  let key = Key::from(0);
//!  let sum = 10;
//!  // This represents an empty value
//!  let value = [0; 32];
//!  let mut tree = Tree::new();
//!
//!  tree.insert(key.clone(), value.into(), sum).unwrap();
//!  tree.delete(key);
//!  let root_hash = &tree.root_hash();
//!  assert_eq!(
//!          root_hash.to_string().as_str(),
//!          "0000000000000000000000000000000000000000000000000000000000000000"
//!  );
//! ```

use super::{key::Key, node_hash::NodeHash};
use sha2::Digest;

#[derive(Debug, Default, PartialEq, Clone)]
pub enum NodeType {
    Leaf(Key),
    #[default]
    Branch,
}
#[derive(Debug, Default, PartialEq, Clone)]
pub struct TreeNode {
    pub(super) node_type: NodeType,
    pub(super) data: NodeHash,
    pub(super) sum: u64,
    pub(super) left: Option<Box<TreeNode>>,
    pub(super) right: Option<Box<TreeNode>>,
}

impl TreeNode {
    pub fn new(data: NodeHash) -> Self {
        TreeNode {
            data,
            ..Default::default()
        }
    }
    fn is_left(idx: u8, key: Key) -> bool {
        !key.bit_index(idx)
    }
    #[inline(always)]
    pub fn insert(&mut self, new_node: TreeNode, key: Key, idx: u8) {
        if new_node == *self {
            return;
        }
        if self.is_target(key) {
            *self = new_node;
            return;
        }
        let target = if Self::is_left(idx, key) {
            &mut self.left
        } else {
            &mut self.right
        };
        match target {
            Some(ref mut node) => node.insert(new_node, key, idx + 1),
            None => {
                let sum = new_node.sum + self.sum;
                let this_node = std::mem::take(self);
                let new_parent = if Self::is_left(idx, key) {
                    TreeNode {
                        left: Some(Box::new(new_node)),
                        right: Some(Box::new(this_node)),
                        data: NodeHash::default(),
                        node_type: NodeType::Branch,
                        sum,
                    }
                } else {
                    TreeNode {
                        right: Some(Box::new(new_node)),
                        left: Some(Box::new(this_node)),
                        data: NodeHash::default(),
                        node_type: NodeType::Branch,
                        sum,
                    }
                };
                *self = new_parent;
            }
        }
        self.recompute_hash();
    }

    fn recompute_hash(&mut self) {
        let hash = sha2::Sha256::new()
            .chain_update(self.get_left_data())
            .chain_update(self.get_right_data())
            .finalize();
        self.sum = self.get_sum();
        self.data = (*hash).try_into().unwrap();
    }
    fn get_sum(&self) -> u64 {
        if let (Some(left), Some(right)) = (&self.left, &self.right) {
            return left.sum + right.sum;
        }
        0
    }
    fn get_left_data(&self) -> NodeHash {
        if let Some(ref left) = self.left {
            return left.data;
        }
        NodeHash::default()
    }
    fn get_right_data(&self) -> NodeHash {
        if let Some(ref right) = self.right {
            return right.data;
        }
        NodeHash::default()
    }
    pub fn is_target(&self, target: Key) -> bool {
        match self.node_type {
            NodeType::Branch => false,
            NodeType::Leaf(key) => target == key,
        }
    }
    #[inline(always)]
    pub fn delete(&mut self, key: Key, idx: u8) {
        let target = if Self::is_left(idx, key) {
            &mut self.left
        } else {
            &mut self.right
        };
        if let Some(ref mut target) = target {
            if target.is_target(key) {
                if Self::is_left(idx, key) {
                    self.left = None;
                    *self = *self.right.clone().unwrap();
                } else {
                    self.right = None;
                    *self = *self.left.clone().unwrap();
                }
            } else {
                target.delete(key, idx + 1);
            }
        }

        self.recompute_hash();
    }

    pub fn prove(&self, proof: &mut Vec<(NodeHash, u64)>, key: Key, level: u8) {
        if self.is_target(key) {
            return;
        }

        if let (Some(ref left), Some(ref right)) = (&self.left, &self.right) {
            if Self::is_left(level, key) {
                proof.push((right.data, right.sum));
                left.prove(proof, key, level + 1);
            } else {
                proof.push((left.data, left.sum));
                right.prove(proof, key, level + 1);
            }
        }
    }
}

pub struct Tree {
    root: Option<TreeNode>,
    leaves: u64,
}
impl Tree {
    pub fn new() -> Tree {
        Tree {
            root: None,
            leaves: 0,
        }
    }
    pub fn root_hash(&self) -> NodeHash {
        if let Some(ref root) = self.root {
            return root.data;
        }
        NodeHash::default()
    }
    pub fn lookup(&self, key: &u32) -> Result<&TreeNode, &'static str> {
        if self.root.is_none() {
            return Err("Empty root");
        }
        let mut node = self.root.as_ref().unwrap();
        for idx in 0..=31 {
            if (key & (1 << idx)) != 0 {
                if let Some(ref current_node) = node.right {
                    node = &*current_node;
                } else {
                    break;
                }
            } else {
                if let Some(ref current_node) = node.left {
                    node = &*current_node;
                } else {
                    break;
                }
            }
        }
        Ok(node)
    }
    pub fn insert(&mut self, key: Key, data: NodeHash, sum: u64) -> Result<(), &'static str> {
        let new_node = TreeNode {
            left: None,
            right: None,
            data,
            node_type: NodeType::Leaf(key),
            sum,
        };
        self.leaves += 1;
        if self.root.is_none() {
            self.root = Some(new_node);
            return Ok(());
        }
        self.root.as_mut().unwrap().insert(new_node, key, 0);
        Ok(())
    }
    pub fn prove(&self, key: Key) -> Vec<(NodeHash, u64)> {
        let mut proof = vec![];
        if self.root.is_none() {
            return proof;
        }
        self.root.as_ref().unwrap().prove(&mut proof, key, 0);
        proof
    }
    pub fn delete(&mut self, key: Key) {
        if self.root.is_none() {
            return;
        }
        if self.leaves == 1 {
            self.root = None;
            return;
        }
        self.root.as_mut().unwrap().delete(key, 0);
        self.leaves -= 1;
    }
}

#[cfg(test)]
mod test {
    use super::Tree;

    #[test]
    fn test_insertion() {
        let mut tree = Tree::new();
        tree.insert(0.into(), [0; 32].into(), 2).unwrap();
        tree.insert(1.into(), [2; 32].into(), 2).unwrap();
        tree.insert(2.into(), [3; 32].into(), 2).unwrap();
        tree.insert(3.into(), [4; 32].into(), 2).unwrap();
        tree.insert(4.into(), [5; 32].into(), 2).unwrap();
        tree.insert(5.into(), [6; 32].into(), 2).unwrap();
        assert_eq!(
            tree.root.unwrap().data.to_string().as_str(),
            "167a44398fe4ff4240c30507a63b00d29bfe79b4468f4207838ab24963454b4d",
        )
    }
    #[test]
    fn test_proof() {
        let mut tree = Tree::new();
        tree.insert(0.into(), [0; 32].into(), 2).unwrap();
        tree.insert(1.into(), [2; 32].into(), 2).unwrap();
        tree.insert(2.into(), [3; 32].into(), 2).unwrap();
        tree.insert(3.into(), [4; 32].into(), 2).unwrap();
        tree.insert(4.into(), [5; 32].into(), 2).unwrap();
        tree.insert(5.into(), [6; 32].into(), 2).unwrap();

        let proof = tree.prove(0.into());
        // This proof has 3 nodes
        assert_eq!(proof.len(), 3);
        println!("{:?}", proof);
        // Sum of each node
        assert_eq!(proof[0].1, 6);
        assert_eq!(proof[1].1, 2);
        assert_eq!(proof[2].1, 2);
        // Value (hash) of each node
        assert_eq!(
            proof[0].0.to_string().as_str(),
            "f220acac38d3866357bd5c2ee6c658808c1c34c21d094b70e14063e77adfce76",
        );
        assert_eq!(
            proof[1].0.to_string().as_str(),
            "0303030303030303030303030303030303030303030303030303030303030303",
        );
        assert_eq!(
            proof[2].0.to_string().as_str(),
            "0505050505050505050505050505050505050505050505050505050505050505",
        );
    }
}
