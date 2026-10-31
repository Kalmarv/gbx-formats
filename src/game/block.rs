//! A map's blocks (`CGameCtnBlock` as the map's block data stores it) and their skins (`CGameCtnBlockSkin`).
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Id, PackDesc};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};

/// Flag bit 15: the block stores an author and a skin.
pub const FLAG_SKIN: u32 = 1 << 15;
/// Flag bit 17: a decal (from block data version 2).
pub const FLAG_DECAL: u32 = 1 << 17;
/// Flag bit 18: square card event ids (from block data version 2).
pub const FLAG_CARD_EVENTS: u32 = 1 << 18;
/// Flag bit 19: a character physics property (from block data version 2).
pub const FLAG_PHY_CHAR: u32 = 1 << 19;
/// Flag bit 20: a waypoint property (from block data version 2).
pub const FLAG_WAYPOINT: u32 = 1 << 20;

/// One block of a map's block data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The block model's name.
    pub name: Id,
    /// Its direction: 0 north, 1 east, 2 south, 3 west. Other values occur (255 in the stand-in blocks of maps made
    /// with TMUnlimiter, 4 in one map) and are kept as stored.
    pub direction: u8,
    /// Its coordinate in block units, as stored: block data version 6 and later store it plus (1, 0, 1).
    pub coord: [u8; 3],
    /// Its flags: a `u16` widened in block data version 0, a `u32` from version 1.
    pub flags: u32,
    /// The skin's author and the skin node, when flag bit 15 is set.
    pub skin: Option<(Id, Option<NodeIndex>)>,
    /// The waypoint property node, when flag bit 20 is set (block data version 2 and later).
    pub waypoint: Option<Option<NodeIndex>>,
}

impl Block {
    /// A block of block data version `version`.
    pub(crate) fn read(r: &mut Archive<'_, '_>, version: u32) -> Result<Block> {
        let name = r.id()?;
        let direction = r.u8()?;
        let coord = r.byte3()?;
        let flags_at = r.here();
        let flags = if version == 0 {
            u32::from(r.u16()?)
        } else {
            r.u32()?
        };
        let skin = if flags & FLAG_SKIN != 0 {
            Some((r.id()?, r.node_ref()?))
        } else {
            None
        };
        let mut waypoint = None;
        if version >= 2 {
            for (bit, what) in [
                (FLAG_PHY_CHAR, "a character physics property"),
                (FLAG_CARD_EVENTS, "square card event ids"),
                (FLAG_DECAL, "a decal"),
            ] {
                if flags & bit != 0 {
                    return Err(r.bad_at(
                        flags_at,
                        format!("block flags {flags:#x}: {what}, which is not read"),
                    ));
                }
            }
            if flags & FLAG_WAYPOINT != 0 {
                waypoint = Some(r.node_ref()?);
            }
        }
        Ok(Block {
            name,
            direction,
            coord,
            flags,
            skin,
            waypoint,
        })
    }
}

/// A block's skin (`CGameCtnBlockSkin`, class `0x03059000`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockSkin {
    /// Chunk `0x03059002`: the text (a sign's, for instance), the skin's pack and its parent pack.
    pub skin: Option<(String, PackDesc, PackDesc)>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl BlockSkin {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<BlockSkin> {
        let mut n = BlockSkin::default();
        n.chunks = r.chunks("CGameCtnBlockSkin", |r, id| {
            match id {
                0x0305_9002 => n.skin = Some((r.string()?, r.pack_desc()?, r.pack_desc()?)),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
