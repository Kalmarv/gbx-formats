//! A map's block stock (`CGameCtnCollectorList`) and parameters (`CGameCtnChallengeParameters`).
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Ident};
use crate::error::Result;
use crate::game::nullable;
use crate::node::Chunks;

/// The block models a map's editor offers (`CGameCtnCollectorList`, class `0x0301B000`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CollectorList {
    /// Chunk `0x0301B000`: each block model and how many of it.
    pub stock: Option<Vec<(Ident, u32)>>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl CollectorList {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<CollectorList> {
        let mut n = CollectorList::default();
        n.chunks = r.chunks("CGameCtnCollectorList", |r, id| {
            match id {
                0x0301_B000 => n.stock = Some(r.list(16, |r| Ok((r.ident()?, r.u32()?)))?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// Medal times in milliseconds, each `None` when stored as -1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Medals {
    /// Bronze.
    pub bronze: Option<u32>,
    /// Silver.
    pub silver: Option<u32>,
    /// Gold.
    pub gold: Option<u32>,
    /// The author's.
    pub author: Option<u32>,
}

impl Medals {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Medals> {
        Ok(Medals {
            bronze: nullable(r)?,
            silver: nullable(r)?,
            gold: nullable(r)?,
            author: nullable(r)?,
        })
    }
}

/// A map's parameters (`CGameCtnChallengeParameters`, class `0x0305B000`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChallengeParameters {
    /// Chunk `0x0305B001`: four tips.
    pub tips: Option<[String; 4]>,
    /// Chunk `0x0305B004`: the medal times, and a `u32` whose meaning is not known.
    pub medals: Option<(Medals, u32)>,
    /// Chunk `0x0305B008`: the time limit (milliseconds) and the author's score (`None` for -1).
    pub stunts: Option<(u32, Option<u32>)>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl ChallengeParameters {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<ChallengeParameters> {
        let mut n = ChallengeParameters::default();
        n.chunks = r.chunks("CGameCtnChallengeParameters", |r, id| {
            match id {
                0x0305_B001 => n.tips = Some([r.string()?, r.string()?, r.string()?, r.string()?]),
                0x0305_B004 => n.medals = Some((Medals::read(r)?, r.u32()?)),
                0x0305_B008 => n.stunts = Some((r.u32()?, nullable(r)?)),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
