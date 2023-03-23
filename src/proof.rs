//! A proof is a collection of hashes needed to hash up to the tree hash. Given the follow
//! tree with 8 leaves:
//! ```!
//! 14
//! |-----------------\
//! 12                13
//! |--------\        |--------\
//! 08       09       10       11
//! |---\    |---\    |---\    |----\
//! 00  01   02  03   04  05   06   07
//! ```
//! to prove 00, you need `[01, 09, 13]`. Assuming you known `00`, you use sha256(00|01) to
//! compute 08, sha256(08|09) to compute 12 and so on. If you can provide a set of hashes that
//! can be used to reproduce the root, assuming the hash function is secure, the object must
//! be in the original set.
//!
use crate::{
    node::{BranchNode, LeafNode, MSSMTNode, Node},
    node_hash::NodeHash,
};
/// The actual proof, just a list of nodes
#[derive(Debug)]
pub struct Proof {
    nodes: Vec<Node>,
}
/// A compact proof is a proof that omits empty branches. In a sparse tree, there will be
/// tons of empty branches, especially if there's only a handful of elements. We signal empty
/// nodes by setting the corresponding bits in a bitmap.
pub struct CompactProof {
    _bits: [bool; 256],
    _nodes: [NodeHash; 256],
}
impl Proof {
    pub fn new(nodes: Vec<Node>) -> Proof {
        Proof { nodes }
    }
}
/// Objects that can produce proofs, like a full tree
pub trait Provable {
    type Error;
    fn prove(&self, key: NodeHash) -> Result<Proof, Self::Error>;
}
/// Things that can be verified, like Proofs
pub trait Verifiable {
    type Error;
    fn verify(self, target_leaf: &LeafNode, key: &NodeHash) -> Result<NodeHash, Self::Error>;
}

impl Verifiable for Proof {
    type Error = String;
    fn verify(mut self, target_leaf: &LeafNode, key: &NodeHash) -> Result<NodeHash, Self::Error> {
        let mut current_node = Node::Leaf(target_leaf.to_owned());

        for idx in (0..=255).rev() {
            let node = self.nodes.pop().unwrap();
            current_node = if key.bit_index(idx) {
                Node::Branch(BranchNode::new(current_node, node).into())
            } else {
                Node::Branch(BranchNode::new(node, current_node).into())
            }
        }
        Ok(current_node.node_hash())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        memory_db::MemoryDatabase,
        node::LeafNode,
        node_hash::NodeHash,
        tree::{MSSMTree, Tree},
    };

    use super::{Provable, Verifiable};

    #[test]
    fn test_proof() {
        let database = MemoryDatabase::new();
        let mut tree = MSSMTree::new(database);
        tree.insert(
            NodeHash::from([0; 32]),
            vec![b'S', b'a', b't', b'o', b's', b'h', b'i'],
            1984,
        )
        .unwrap();
        let leaf = LeafNode::new(vec![b'S', b'a', b't', b'o', b's', b'h', b'i'], 1984);
        let proof = tree.prove(NodeHash::from([0; 32])).unwrap();
        let root = proof.verify(&leaf, &NodeHash::from([0; 32])).unwrap();
        assert_eq!(
            NodeHash::try_from("fe7917b2f00e3192692c0b1411cfe1d5527ab0e34bf76cde295417b558045cd5")
                .unwrap(),
            root
        )
    }
}
