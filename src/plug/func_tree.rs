//! Functions of a scene tree: `CFuncTreeSubVisualSequence` and the keys it holds, `CFuncKeysNatural`.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use super::func_shader::FunctionTiming;
use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};

/// Keys of natural numbers (`CFuncKeysNatural`, class `0x05030000`, a subclass of `CFuncKeys`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeysNatural {
    /// `CFuncKeys` chunk `0x05002001`: the keys' positions along the function's input.
    pub xs: Option<Vec<f32>>,
    /// `CFuncKeys` chunk `0x05002003`: an id.
    pub id: Option<Id>,
    /// Chunk `0x05030000`: the natural number of each key.
    pub naturals: Option<Vec<i32>>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl KeysNatural {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<KeysNatural> {
        let mut n = KeysNatural::default();
        n.chunks = r.chunks("CFuncKeysNatural", |r, id| {
            match id {
                0x0500_2001 => n.xs = Some(r.list(4, |r| r.f32())?),
                0x0500_2003 => n.id = Some(r.id()?),
                0x0503_0000 => n.naturals = Some(r.list(4, |r| r.i32())?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A function choosing a sub-visual by an input (`CFuncTreeSubVisualSequence`, class `0x05031000`, a subclass of
/// `CFuncTree` and so of `CFuncPlug`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeSubVisualSequence {
    /// `CFuncPlug` chunk `0x0500B005`: the period, phase and input.
    pub timing: Option<FunctionTiming>,
    /// Chunk `0x05031001`: an id.
    pub id: Option<Id>,
    /// Chunk `0x05031002`: the keys (`CFuncKeysNatural`).
    pub sub_keys: Option<Option<NodeIndex>>,
    /// Chunk `0x05031003`: whether the simple mode loops.
    pub simple_mode_is_looping: Option<bool>,
    /// Chunk `0x05031003`: the first index of the simple mode.
    pub simple_mode_start_index: Option<i32>,
    /// Chunk `0x05031003`: the last index of the simple mode.
    pub simple_mode_end_index: Option<i32>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl TreeSubVisualSequence {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<TreeSubVisualSequence> {
        let mut n = TreeSubVisualSequence::default();
        n.chunks = r.chunks("CFuncTreeSubVisualSequence", |r, id| {
            match id {
                0x0500_B005 => n.timing = Some(FunctionTiming::read(r)?),
                0x0503_1001 => n.id = Some(r.id()?),
                0x0503_1002 => n.sub_keys = Some(r.node_ref()?),
                0x0503_1003 => {
                    n.simple_mode_is_looping = Some(r.bool()?);
                    n.simple_mode_start_index = Some(r.i32()?);
                    n.simple_mode_end_index = Some(r.i32()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
