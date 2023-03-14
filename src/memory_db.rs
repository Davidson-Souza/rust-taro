//! A small, in-memory database backend for trees. This database is not intended to be used
//! in production. It's suitable for testing and writing small Proof-of-Concepts.
//! It keeps all data as a simple [HashMap], if you add too much data, this will take-up all
//! your system's RAM.
//!
//! # Usage:
//! ```
//!    use rust_taro::{memory_db::MemoryDatabase, node::{MSSMTNode, LeafNode}};
//!    use rust_taro::tree_backend::TreeStore;
//!
//!    let storage = MemoryDatabase::new();
//!
//!    let leaf1 = LeafNode::new(vec![0, 1, 2, 3], 10);
//!    storage.insert_leaf(leaf1.clone()).expect("Valid leaves");
//!
//!    let branch = storage
//!        .fetch_leaf(leaf1.node_hash())
//!        .unwrap()
//!        .unwrap();
//!
//!    assert_eq!(
//!        branch.node_hash().to_string().as_str(),
//!        "2ecf333bfc373434f47fdd8a7be8d9a693bb19bc09bd0af6edc98b24c51f8411"
//!    );
//!```

use std::{
    collections::HashMap,
    sync::{PoisonError, RwLock},
};

use crate::{
    node::MSSMTNode,
    node::{BranchNode, LeafNode, Node},
    node_hash::NodeHash,
    tree_backend::TreeStore,
};

pub struct MemoryDatabase {
    inner: RwLock<HashMap<NodeHash, Node>>,
}

impl MemoryDatabase {
    pub fn new() -> MemoryDatabase {
        MemoryDatabase {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl TreeStore for MemoryDatabase {
    type Error = MemoryDatabaseError;

    fn delete_branch(&self, hash: NodeHash) -> Result<(), Self::Error> {
        let mut inner = self.inner.write()?;
        inner.remove(&hash);
        Ok(())
    }

    fn delete_leaf(&self, hash: NodeHash) -> Result<(), Self::Error> {
        let mut inner = self.inner.write()?;
        inner.remove(&hash);
        Ok(())
    }

    fn insert_branch(&self, branch: BranchNode) -> Result<(), Self::Error> {
        let mut inner = self.inner.write()?;
        inner.insert(branch.node_hash(), Node::Branch(branch.into()));
        Ok(())
    }

    fn insert_leaf(&self, leaf: LeafNode) -> Result<(), Self::Error> {
        let mut inner = self.inner.write()?;
        inner.insert(leaf.node_hash(), Node::Leaf(leaf));
        Ok(())
    }
    fn fetch_branch(
        &self,
        hash: NodeHash,
    ) -> Result<Option<crate::node::DiskBranchNode>, Self::Error> {
        let inner = self.inner.read()?;
        let node = inner.get(&hash);
        match node {
            Some(Node::Branch(node)) => Ok(Some(node.to_owned())),
            Some(Node::Leaf(_)) => Ok(None),
            None => Ok(None),
        }
    }

    fn fetch_leaf(&self, hash: NodeHash) -> Result<Option<LeafNode>, Self::Error> {
        let inner = self.inner.read()?;
        let node = inner.get(&hash);
        match node {
            Some(Node::Branch(_)) => Ok(None),
            Some(Node::Leaf(leaf)) => Ok(Some(leaf.to_owned())),
            None => Ok(None),
        }
    }

    fn fetch_branch_recursive(&self, _: NodeHash) -> Result<Option<BranchNode>, Self::Error> {
        todo!()
    }
}

#[derive(Debug)]
pub enum MemoryDatabaseError {
    PoisonedLock,
}
impl<T> From<PoisonError<T>> for MemoryDatabaseError {
    fn from(_: PoisonError<T>) -> Self {
        Self::PoisonedLock
    }
}

#[cfg(test)]
mod test {
    use crate::{
        node::{BranchNode, LeafNode, MSSMTNode, Node},
        node_hash::NodeHash,
        tree_backend::TreeStore,
    };

    use super::MemoryDatabase;

    #[test]
    fn test_database() {
        let storage = MemoryDatabase::new();

        let leaf1 = LeafNode::new(vec![0, 1, 2, 3], 10);
        let leaf2 = LeafNode::new(vec![4, 5, 6], 100);

        storage.insert_leaf(leaf1.clone()).expect("Valid leaves");
        storage.insert_leaf(leaf2.clone()).expect("Valid leaves");

        let branch = BranchNode::new(Node::Leaf(leaf1), Node::Leaf(leaf2));

        storage.insert_branch(branch).expect("Valid branch");

        let branch = storage
            .fetch_branch(
                NodeHash::try_from(
                    "9b70d7de4fe4c5b40347333d664073277251690e26df3270e84f3c73b6eec03c",
                )
                .unwrap(),
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            branch.node_hash().to_string().as_str(),
            "9b70d7de4fe4c5b40347333d664073277251690e26df3270e84f3c73b6eec03c"
        );
        assert_eq!(
            branch.l_child().to_string().as_str(),
            "2ecf333bfc373434f47fdd8a7be8d9a693bb19bc09bd0af6edc98b24c51f8411"
        );
        assert_eq!(
            branch.r_child().to_string().as_str(),
            "a42280e0a6760328dfc8b4c494761c255c4aaa4f98d606eb52717dd872d3c15b"
        )
    }
}
