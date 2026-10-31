//! The nodes of a body: the main node and every node a reference reads, by the index the file gives each.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::classes::Node;
use std::collections::BTreeMap;

/// The index a file gives a node: what a node reference stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeIndex(pub u32);

/// A skippable chunk this crate does not read, kept as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawChunk {
    /// The chunk id.
    pub id: u32,
    /// The offset of its data (after the skip marker and size), in the space its node was read from.
    pub offset: usize,
    /// Its data.
    pub bytes: Vec<u8>,
}

/// A node's chunks: the ids in the order stored, and the skippable ones this crate does not read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Chunks {
    /// Every chunk id, in order (the end marker not included).
    pub ids: Vec<u32>,
    /// The skippable chunks not read, as stored.
    pub unread: Vec<RawChunk>,
}

#[derive(Debug)]
enum Slot {
    /// Being read: a reference to it from within its own chunks is to the node itself.
    Reading,
    Read(Node),
}

/// The nodes being read: the indices below the header's node count, of which only those read or being read are
/// stored (the count is the file's claim, so nothing is allocated for it), and the indices the reference table lists.
#[derive(Debug)]
pub(crate) struct NodeTable {
    count: u32,
    slots: BTreeMap<u32, Slot>,
    external: Vec<u32>,
}

impl NodeTable {
    pub(crate) fn new(count: u32, external: Vec<u32>) -> NodeTable {
        NodeTable {
            count,
            slots: BTreeMap::new(),
            external,
        }
    }

    /// The header's node count.
    pub(crate) fn count(&self) -> u32 {
        self.count
    }

    /// The index is a node read or being read, or one the reference table lists.
    pub(crate) fn is_taken(&self, index: u32) -> bool {
        self.external.contains(&index) || self.slots.contains_key(&index)
    }

    /// The index is below the header's node count.
    pub(crate) fn has_slot(&self, index: u32) -> bool {
        index < self.count
    }

    pub(crate) fn reserve(&mut self, index: u32) {
        self.slots.insert(index, Slot::Reading);
    }

    pub(crate) fn fill(&mut self, index: u32, node: Node) {
        self.slots.insert(index, Slot::Read(node));
    }

    pub(crate) fn into_nodes(self, main: Node) -> Nodes {
        let others = self
            .slots
            .into_iter()
            .filter_map(|(i, s)| match s {
                Slot::Read(n) => Some((NodeIndex(i), n)),
                Slot::Reading => None,
            })
            .collect();
        Nodes { main, others }
    }
}

/// Every node a body holds: the main node (index 0) and the nodes its references read, by index.
#[derive(Debug)]
pub struct Nodes {
    main: Node,
    others: Vec<(NodeIndex, Node)>,
}

impl Nodes {
    /// The main node: the class the header names.
    pub fn main(&self) -> &Node {
        &self.main
    }

    /// The node a reference names: `None` for an index this body did not read (one the reference table lists).
    pub fn get(&self, index: NodeIndex) -> Option<&Node> {
        if index == NodeIndex(0) {
            return Some(&self.main);
        }
        self.others
            .binary_search_by_key(&index, |(i, _)| *i)
            .ok()
            .and_then(|k| self.others.get(k))
            .map(|(_, n)| n)
    }

    /// Every node, the main one first, then by index.
    pub fn iter(&self) -> impl Iterator<Item = (NodeIndex, &Node)> {
        std::iter::once((NodeIndex(0), &self.main)).chain(self.others.iter().map(|(i, n)| (*i, n)))
    }
}
