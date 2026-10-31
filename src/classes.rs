//! Every class whose nodes this crate reads, by class id: a node of any other class is refused
//! ([`crate::Error::UnknownClass`]).

use crate::archive::Archive;
use crate::error::{Error, Result};
use crate::game;
use crate::node::Chunks;

macro_rules! classes {
    ($($(#[$doc:meta])* $variant:ident($ty:path) = $id:literal, $name:literal;)*) => {
        /// A node of a class this crate reads.
        #[derive(Clone, Debug)]
        pub enum Node {
            $($(#[$doc])* $variant(Box<$ty>),)*
        }

        impl Node {
            /// The class id.
            pub fn class_id(&self) -> u32 {
                match self {
                    $(Node::$variant(_) => $id,)*
                }
            }

            /// The class name, as Nadeo's engine names it.
            pub fn class_name(&self) -> &'static str {
                match self {
                    $(Node::$variant(_) => $name,)*
                }
            }

            /// The node's chunks: their ids in order, and the skippable ones not read.
            pub fn chunks(&self) -> &Chunks {
                match self {
                    $(Node::$variant(n) => &n.chunks,)*
                }
            }
        }

        $(
            impl<'n> TryFrom<&'n Node> for &'n $ty {
                type Error = &'n Node;
                fn try_from(node: &'n Node) -> std::result::Result<Self, &'n Node> {
                    match node {
                        Node::$variant(n) => Ok(n),
                        #[allow(unreachable_patterns, reason = "a list of one class has no other")]
                        other => Err(other),
                    }
                }
            }
        )*

        /// The name of a class this crate reads, by id.
        pub fn class_name(id: u32) -> Option<&'static str> {
            match id {
                $($id => Some($name),)*
                _ => None,
            }
        }

        /// Reads a node of class `class` (its id read at `at`).
        pub(crate) fn read_node(r: &mut Archive<'_, '_>, class: u32, at: usize) -> Result<Node> {
            match class {
                $($id => Ok(Node::$variant(Box::new(<$ty>::read(r)?))),)*
                _ => Err(Error::UnknownClass {
                    space: r.space(),
                    offset: at as u128,
                    class,
                }),
            }
        }
    };
}

classes! {
    /// A map.
    Challenge(game::challenge::Challenge) = 0x0304_3000, "CGameCtnChallenge";
    /// The block models a map's editor offers.
    CollectorList(game::collector::CollectorList) = 0x0301_B000, "CGameCtnCollectorList";
    /// A map's tips, medal times and time limit.
    ChallengeParameters(game::collector::ChallengeParameters) = 0x0305_B000, "CGameCtnChallengeParameters";
    /// A block's skin.
    BlockSkin(game::block::BlockSkin) = 0x0305_9000, "CGameCtnBlockSkin";
}
