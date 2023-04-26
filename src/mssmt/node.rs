//! The logic behind a single tree node. A node can be a Leaf or a Branch node.
//! Leaves contains actual data being committed to, and lives on the bottom of our tree.
//! Branch nodes are intermediate nodes that links the root to a leaf, and only contains
//! the hash of it's children and a value that represents the sum of all leaf values in
//! a given subtree

use sha2::Digest;

use crate::mssmt::node_hash::NodeHash;

/// A trait that must be implemented by all nodes in the tree
pub trait MSSMTNode {
    /// The node's associated `hash` value. For leafs, this is the hash of it's content.
    /// For branch nodes, sha256(l_child, r_child) where `[r|l]_child` is the child's hash
    fn node_hash(&self) -> NodeHash;

    /// A node's associated `sum` value
    fn node_sum(&self) -> u64;
}
#[derive(Debug, Clone)]
pub enum Node {
    Leaf(LeafNode),
    Branch(DiskBranchNode),
}
impl Default for Node {
    fn default() -> Self {
        Node::Leaf(LeafNode {
            data: vec![],
            sum: 0,
        })
    }
}
#[derive(Debug, Clone)]
pub struct BranchNode {
    sum: u64,
    hash: NodeHash,
    left: Node,
    right: Node,
}
/// A [DiskBranchNode] is a BranchNode, but we don't fetch it's children, just pull their
/// hashes. If we use BranchNode directly, we would be forced to fetch the whole subtree
/// to make the node type-complete.
#[derive(Debug, Clone)]
pub struct DiskBranchNode {
    /// The sum of all leaves in this subtree
    sum: u64,
    /// This node's hash
    _hash: NodeHash,
    /// Hash of the left child
    left: NodeHash,
    /// Hash of the right child
    right: NodeHash,
}

impl DiskBranchNode {
    pub fn l_child(&self) -> &NodeHash {
        &self.left
    }
    pub fn r_child(&self) -> &NodeHash {
        &self.right
    }
    pub fn new(sum: u64, left: NodeHash, right: NodeHash) -> DiskBranchNode {
        let _hash = BranchNode::parent_hash(left, right, sum);
        DiskBranchNode {
            sum,
            _hash,
            left,
            right,
        }
    }
}
impl BranchNode {
    pub fn new(left: Node, right: Node) -> BranchNode {
        let sum = left.node_sum() + right.node_sum();
        let hash = BranchNode::parent_hash(left.node_hash(), right.node_hash(), sum);

        BranchNode {
            sum,
            hash,
            left,
            right,
        }
    }
    fn parent_hash(left: NodeHash, right: NodeHash, sum: u64) -> NodeHash {
        let hash = sha2::Sha256::new()
            .chain_update(&left)
            .chain_update(&right)
            .chain_update(sum.to_be_bytes())
            .finalize();
        NodeHash::try_from(&*hash).unwrap()
    }
}
/// Leaves are nodes that contains the actual data being committed to, they sit at
/// the last row and don't have any descendants.
#[derive(Debug, Clone)]
pub struct LeafNode {
    data: Vec<u8>,
    sum: u64,
}

impl LeafNode {
    pub fn new(data: Vec<u8>, sum: u64) -> LeafNode {
        LeafNode { data, sum }
    }
}

impl MSSMTNode for LeafNode {
    fn node_hash(&self) -> NodeHash {
        let hash = sha2::Sha256::new()
            .chain_update(&self.data)
            .chain_update(self.sum.to_be_bytes())
            .finalize();
        NodeHash::try_from(&*hash).unwrap()
    }
    fn node_sum(&self) -> u64 {
        self.sum
    }
}

impl MSSMTNode for DiskBranchNode {
    fn node_hash(&self) -> NodeHash {
        let hash = sha2::Sha256::new()
            .chain_update(&self.left)
            .chain_update(&self.right)
            .chain_update(self.sum.to_be_bytes())
            .finalize();
        NodeHash::try_from(&*hash).unwrap()
    }
    fn node_sum(&self) -> u64 {
        self.sum
    }
}

impl MSSMTNode for Node {
    fn node_hash(&self) -> NodeHash {
        match self {
            Node::Branch(inner) => inner.node_hash(),
            Node::Leaf(inner) => inner.node_hash(),
        }
    }

    fn node_sum(&self) -> u64 {
        match self {
            Node::Branch(inner) => inner.node_sum(),
            Node::Leaf(inner) => inner.node_sum(),
        }
    }
}

impl MSSMTNode for BranchNode {
    fn node_hash(&self) -> NodeHash {
        let hash = sha2::Sha256::new()
            .chain_update(&self.left.node_hash())
            .chain_update(&self.right.node_hash())
            .chain_update(self.sum.to_be_bytes())
            .finalize();
        NodeHash::try_from(&*hash).unwrap()
    }
    fn node_sum(&self) -> u64 {
        self.sum
    }
}

impl From<BranchNode> for DiskBranchNode {
    fn from(value: BranchNode) -> Self {
        DiskBranchNode {
            sum: value.sum,
            _hash: value.hash,
            left: value.left.node_hash(),
            right: value.right.node_hash(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::mssmt::node_hash::NodeHash;

    use super::{LeafNode, MSSMTNode};

    #[test]
    fn test_node_hash() {
        let expected_hash =
            NodeHash::try_from("a8a978fd0d18e6d65c09a6771425d6e8cb7f8e7695cf178696c1b20d0e7d9edd")
                .unwrap();
        let node_hash = LeafNode {
            data: vec![b'B', b'i', b't', b'c', b'o', b'i', b'n'],
            sum: 99,
        }
        .node_hash();
        assert_eq!(expected_hash, node_hash)
    }
}
