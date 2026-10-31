//! Read Nadeo's GameBox (`.Gbx`) files: the container, its LZO1X-compressed body, and the nodes of the classes
//! TrackMania Nations Forever maps hold.
//!
//! - [`Gbx`]: the container (version 6, binary): header, header chunks, reference table, body.
//! - [`Gbx::nodes`]: the body's nodes, read class by class: the main node and every node its references read, by
//!   the index the file gives each ([`Nodes`], [`Node`]). A node is a stream of chunks; a chunk this crate does not
//!   read is kept as stored when the file marks it skippable ([`Chunks::unread`]) and refused otherwise, since its
//!   size is then unknown ([`Error::UnknownChunk`]). A class it does not read is refused ([`Error::UnknownClass`]).
//! - [`game::challenge`]: a map's header chunks ([`game::challenge::Header`]) and body ([`game::challenge::Challenge`]):
//!   its blocks, checkpoints, medal times, mediatracker clips and the rest.
//!
//! Values are kept as stored: block coordinates as the file holds them, ids as numbers or strings, node references
//! as indices. Every read is checked: untrusted bytes give an [`Error`] naming the offset (in the file, or in the
//! decompressed body) and what was expected, never a panic.
//!
//! ```
//! use gbx_formats::Gbx;
//! use gbx_formats::game::challenge::Challenge;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // A map built by hand: an uncompressed body holding one chunk, "made with the simple editor".
//! let mut file = b"GBX\x06\x00BUUR".to_vec();
//! for v in [0x0304_3000u32, 0, 1, 0, 0x0304_302A, 1, 0xFACA_DE01] {
//!     // class, no header chunks, one node, no reference table; then the body: chunk, bool, end marker
//!     file.extend_from_slice(&v.to_le_bytes());
//! }
//! let gbx = Gbx::parse(&file)?;
//! let nodes = gbx.nodes()?;
//! let map: &Challenge = nodes.main().try_into().map_err(|_| "the main node is not a map")?;
//! assert_eq!(map.created_with_simple_editor, Some(true));
//! # Ok(())
//! # }
//! ```
#![deny(clippy::disallowed_methods, clippy::disallowed_types)]
#![deny(missing_docs)]

mod archive;
mod classes;
mod error;
pub mod game;
mod gbx;
mod node;
pub mod plug;

pub use archive::{Id, Ident, PackDesc};
pub use classes::{Node, class_name};
pub use error::{Error, Result, Space};
pub use gbx::{Body, Compression, External, ExternalTarget, Folder, Gbx, HeaderChunk, RefTable};
pub use node::{Chunks, NodeIndex, Nodes, RawChunk};
