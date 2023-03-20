use std::slice;

use crate::{
    node::{BranchNode, DiskBranchNode, LeafNode, MSSMTNode, Node},
    node_hash::NodeHash,
    proof::{Proof, Provable, Verifiable},
    tree_backend::TreeStore,
};

/// Defines all operations in a full tree
pub trait Tree<E> {
    /// Inserts a new leaf into the tree
    fn insert(&mut self, key: NodeHash, data: Vec<u8>, sum: u64) -> Result<(), E>;
    /// Removes a node from the tree, indexed by a [NodeHash]
    fn delete(&mut self, key: NodeHash) -> Result<(), E>;
    /// Updates a node that already exists
    fn update(&mut self, key: NodeHash, data: Vec<u8>, sum: u64) -> Result<(), E>;
    /// Looks up a node and returns it's value
    fn lookup(&self, key: NodeHash) -> Result<Option<LeafNode>, E>;
}

/// A  full Merkle Sum Sparse Merkle Tree. A full Merkle Tree that virtually contains
/// all 2^256 nodes. This is made tractable by not storing empty nodes. In practice, we'll
/// never have more than 2^64 nodes anyways.
/// By being full, each element have exactly one possible position inside the tree, so you
/// can prove statements like proof of non-inclusion (or proof of emptiness).
/// This tree also commits to a value, and the root holds the sum of all leaves's values.
pub struct MSSMTree<Persistence: TreeStore> {
    /// A backend for our tree. We store nodes in key-value pairs.
    database: Persistence,
    /// Points to this tree's root
    root: NodeHash,
    /// This is used for optimization reasons. It contains the pre-computed values for
    /// an empty tree. So we can see what an empty value for each level looks like
    empty_tree: Vec<Node>,
}
impl<Persistence: TreeStore> MSSMTree<Persistence> {
    /// Returns this node's children hash. It can either be in an empty branch, so we return
    /// the corresponding hash from the empty_tree. If this node isn't empty, we then return
    /// it's actual child
    fn get_children_hash(&self, node: &Option<DiskBranchNode>, idx: u8) -> (NodeHash, NodeHash) {
        if node.is_none()
            || node.as_ref().unwrap().node_hash() == self.empty_tree[idx as usize].node_hash()
        {
            // If we call this method in a leaf
            if idx == 255 {
                let hash = NodeHash::from([0; 32]);
                return (hash, hash);
            }
            let hash = self.empty_tree[(idx + 1) as usize].node_hash();
            return (hash, hash);
        }

        let node = node.as_ref().unwrap();
        (*node.l_child(), *node.r_child())
    }
    pub fn new(database: Persistence) -> MSSMTree<Persistence> {
        let mut empty_tree: Vec<Node> = Vec::with_capacity(256);
        let mut node = Node::default();
        // Creates the empty tree
        for _ in 0..=255 {
            empty_tree.push(node.clone());

            let branch = Node::Branch(DiskBranchNode::new(0, node.node_hash(), node.node_hash()));
            node = branch;
        }
        // We build it in reverse order, from leaf to root. But in a tree, index 0 is the root
        // so we reverse that here.
        let empty_tree: Vec<Node> = empty_tree.iter().cloned().rev().collect();
        MSSMTree {
            database,
            root: empty_tree[0].node_hash(),
            empty_tree,
        }
    }
    fn get_node_hash(&self, node: Option<BranchNode>, idx: u8) -> NodeHash {
        if let Some(node) = node {
            return node.node_hash();
        }
        self.empty_tree[idx as usize].node_hash()
    }
}
impl<Persistence: TreeStore> Tree<Persistence::Error> for MSSMTree<Persistence> {
    fn insert(&mut self, key: NodeHash, data: Vec<u8>, sum: u64) -> Result<(), Persistence::Error> {
        let leaf = LeafNode::new(data, sum);

        let mut node = self.root;
        let mut parents = vec![];
        let mut siblings = vec![];
        // Walks down the tree and grabs all parents and siblings on the way down
        for idx in 0..=255 {
            let disk_node = self.database.fetch_branch(node)?;
            let (left, right) = self.get_children_hash(&disk_node, idx);

            let (next, sibling) = if key.bit_index(idx) {
                (left, right)
            } else {
                (right, left)
            };
            parents.push(node);
            siblings.push(sibling);
            node = next;
        }
        if leaf.node_hash() != self.empty_tree[255].node_hash() {
            self.database.insert_leaf(leaf.clone())?;
        } else {
            self.database.delete_leaf(leaf.node_hash())?;
        }
        let mut current_update: Node = Node::Leaf(leaf);
        // Actually update the tree
        for idx in (0..=255).rev() {
            let sibling = siblings[idx as usize];
            let (left, right) = if key.bit_index(idx) {
                (current_update.node_hash(), sibling)
            } else {
                (sibling, current_update.node_hash())
            };

            let sibling = self.database.fetch_branch(sibling)?;
            let sum = if let Some(sibling) = sibling {
                current_update.node_sum() + sibling.node_sum()
            } else {
                current_update.node_sum()
            };
            // If the old node isn't empty, delete it from the storage
            if parents[idx as usize] != self.empty_tree[idx as usize].node_hash() {
                self.database.delete_branch(parents[idx as usize])?;
            }
            let new_node = DiskBranchNode::new(sum, left, right);
            // If the new node isn't empty, add it into the storage
            if new_node.node_hash() != self.empty_tree[idx as usize].node_hash() {
                self.database.insert_branch(new_node.clone())?;
            }
            current_update = Node::Branch(new_node);
        }
        self.root = current_update.node_hash();
        Ok(())
    }

    fn delete(&mut self, key: NodeHash) -> Result<(), Persistence::Error> {
        self.insert(key, vec![], 0)
    }

    fn update(&mut self, key: NodeHash, data: Vec<u8>, sum: u64) -> Result<(), Persistence::Error> {
        self.insert(key, data, sum)
    }

    fn lookup(&self, key: NodeHash) -> Result<Option<LeafNode>, Persistence::Error> {
        let mut node = self.root;
        for idx in 0..=255 {
            let disk_node = self.database.fetch_branch(node)?;
            let (left, right) = self.get_children_hash(&disk_node, idx);
            let next = if key.bit_index(idx) { left } else { right };
            node = next;
        }
        Ok(self.database.fetch_leaf(node)?)
    }
}

impl<T: TreeStore> Provable for MSSMTree<T> {
    type Error = T::Error;

    fn prove(&self, key: NodeHash) -> Result<crate::proof::Proof, Self::Error> {
        let mut proof = Vec::new();
        let mut node = self.root;
        for idx in 0..=255 {
            let disk_node = self.database.fetch_branch(node)?;
            let (left, right) = self.get_children_hash(&disk_node, idx);

            let (next, sibling) = if key.bit_index(idx) {
                (left, right)
            } else {
                (right, left)
            };
            node = next;
            if idx < 255 {
                if let Some(sibling) = self.database.fetch_branch(sibling)? {
                    proof.push(Node::Branch(sibling));
                } else {
                    proof.push(self.empty_tree[idx as usize].clone());
                }
            } else {
                if let Some(sibling) = self.database.fetch_leaf(sibling)? {
                    proof.push(Node::Leaf(sibling));
                } else {
                    proof.push(self.empty_tree[idx as usize].clone());
                }
            }
        }
        Ok(Proof::new(proof))
    }
}

#[cfg(test)]
mod test {
    use crate::{
        memory_db::MemoryDatabase,
        node::{DiskBranchNode, LeafNode, MSSMTNode, Node},
        node_hash::NodeHash,
        proof::Provable,
    };
    fn get_test_tree() -> MSSMTree<MemoryDatabase> {
        let database = MemoryDatabase::new();

        MSSMTree::new(database)
    }
    use super::{MSSMTree, Tree};
    #[test]
    fn test_addition() {
        let expected_hash = LeafNode::new(vec![1], 99).node_hash();

        let mut tree = get_test_tree();

        tree.insert(NodeHash::from([0; 32]), vec![1], 99).unwrap();

        let leaf = tree
            .lookup([0; 32].into())
            .unwrap()
            .expect("We just inserted this");
        assert_eq!(leaf.node_sum(), 99);
        assert_eq!(leaf.node_hash(), expected_hash);
    }
    #[test]
    fn test_deletion() {
        let mut tree = get_test_tree();
        tree.insert(NodeHash::from([0; 32]), vec![1], 99)
            .expect("Should be able to add");

        tree.delete(NodeHash::from([0; 32]))
            .expect("Should be able to delete");
        let res = tree.lookup(NodeHash::from([0; 32])).unwrap();
        assert!(res.is_none());
    }
    #[test]
    fn test_update() {
        let mut tree = get_test_tree();
        let expected_hash = LeafNode::new(vec![2], 100).node_hash();

        tree.insert(NodeHash::from([0; 32]), vec![1], 99)
            .expect("Should be able to add");

        tree.update(NodeHash::from([0; 32]), vec![2], 100)
            .expect("Should be able to delete");

        let leaf = tree
            .lookup([0; 32].into())
            .unwrap()
            .expect("We just inserted this");
        assert_eq!(leaf.node_sum(), 100);
        assert_eq!(leaf.node_hash(), expected_hash);
    }
    #[test]
    fn test_empty_tree() {
        // Tests the sanity of our empty tree
        let mut hashes = vec![];
        let tree = get_test_tree();
        let mut node = Node::Leaf(LeafNode::new(vec![], 0));
        for _ in 0..=255 {
            hashes.push(node.node_hash());
            node = Node::Branch(DiskBranchNode::new(0, node.node_hash(), node.node_hash()));
        }

        let cmp = tree.empty_tree.iter().zip(hashes.iter().rev());
        for (left, right) in cmp {
            // assert that each i-th position is pairwise equal
            assert_eq!(left.node_hash(), *right);
        }
    }
}
