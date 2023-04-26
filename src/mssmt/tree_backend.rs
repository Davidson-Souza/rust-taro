//! A Merkle-Sum Sparse Merkle Tree data can reside on memory only, but since the tree
//! is innevitably deep and keeping it in a structured way on memory requires holding all
//! branch nodes for a given branch, it's more feasible if nodes lives on a disk-first way.
//!
//! This trait abstracts the engine that actually holds on to data, it can be a simple
//! in-ram HashMap or a complicated distributed Database Engine like Postgres. Empty hashes
//! should be optimized out by removing it from the set, since it's value (and from nodes above it)
//! can be computed efficiently ahead of time, this saves up space and makes the tree more
//! tractable.
//!
use super::node::{BranchNode, DiskBranchNode, LeafNode};
use super::node_hash::NodeHash;

pub trait TreeStore {
    type Error;
    /// Stores a new branch keyed by its node_hash. Branch nodes are intermediate nodes
    /// that aren't a root or a leaf (i.e nodes in 1 <= i < 255).
    fn insert_branch(&self, branch: DiskBranchNode) -> Result<(), Self::Error>;
    /// Inserts a new leaf into our storage
    fn insert_leaf(&self, leaf: LeafNode) -> Result<(), Self::Error>;
    /// delete_branch deletes the branch node keyed by the given NodeHash.
    fn delete_branch(&self, hash: NodeHash) -> Result<(), Self::Error>;
    /// delete_leaf deletes the leaf node keyed by the given NodeHash.
    fn delete_leaf(&self, hash: NodeHash) -> Result<(), Self::Error>;
    /// Fetches a branch node from storage. This method only fetches one node and
    /// the id of it's children. To get the actual child, you need to fetch again.
    fn fetch_branch(&self, hash: NodeHash) -> Result<Option<DiskBranchNode>, Self::Error>;
    /// Fetches a branch node from storage. This method will also pull every children in
    /// the subtree. So if a node have subtree depth of 5, all 5 levels will be fetched.
    /// This might cause some memory issues for bigger subtrees.
    fn fetch_branch_recursive(&self, hash: NodeHash) -> Result<Option<BranchNode>, Self::Error>;
    /// Fetches a leaf node from internal storage.
    fn fetch_leaf(&self, hash: NodeHash) -> Result<Option<LeafNode>, Self::Error>;
}
