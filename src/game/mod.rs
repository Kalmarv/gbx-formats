//! The game classes: maps and what they hold.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

pub mod block;
pub mod blockinfo;
pub mod challenge;
pub mod collection;
pub mod collector;
pub mod decoration;
pub mod ghost;
pub mod media;
pub mod parameters;
pub mod scene;
pub mod unlimiter;

use crate::archive::Archive;
use crate::error::Result;

/// A `u32` stored as `0xFFFFFFFF` when absent.
pub(crate) fn nullable(r: &mut Archive<'_, '_>) -> Result<Option<u32>> {
    Ok(Some(r.u32()?).filter(|&v| v != u32::MAX))
}
