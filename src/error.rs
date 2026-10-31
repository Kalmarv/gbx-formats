//! Why a read was refused.

use primase_foundations::bytes::BoundsError;

/// Which bytes an offset counts in: the file as given, or the body once decompressed (a compressed body's offsets
/// are those of [`crate::Gbx::body`]'s bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    /// The file's own bytes.
    File,
    /// The decompressed body.
    Body,
    /// A pack file (`.pak`) or key list, as stored.
    Pack,
}

impl std::fmt::Display for Space {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Space::File => "Gbx file",
            Space::Body => "Gbx body (decompressed)",
            Space::Pack => "pack",
        })
    }
}

/// Why a read was refused. Each message names the byte offset and what was expected or found.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes break the format at this offset.
    #[error("{space} at byte {offset}: {detail}")]
    Format {
        /// Which bytes the offset counts in.
        space: Space,
        /// The byte offset.
        offset: u128,
        /// What was expected or found.
        detail: String,
    },
    /// A chunk this crate does not read, stored without the skip marker: its size is not stored, so nothing after it
    /// can be read.
    #[error(
        "{space} at byte {offset}: chunk {chunk:#010x} of {class} is not read by this crate and is not skippable"
    )]
    UnknownChunk {
        /// Which bytes the offset counts in.
        space: Space,
        /// The offset of the chunk's id.
        offset: u128,
        /// The chunk id as stored.
        chunk: u32,
        /// The class of the node holding it.
        class: &'static str,
    },
    /// A node of a class this crate does not read.
    #[error("{space} at byte {offset}: node of class {class:#010x}, which this crate does not read")]
    UnknownClass {
        /// Which bytes the offset counts in.
        space: Space,
        /// The offset of the class id.
        offset: u128,
        /// The class id as stored.
        class: u32,
    },
    /// The compressed body does not decompress to its stated size.
    #[error("Gbx body: {0}")]
    Decompress(#[from] primase_foundations::compress::Error),
}

/// A result with this crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn bad(space: Space, offset: usize, detail: impl Into<String>) -> Error {
    Error::Format {
        space,
        offset: offset as u128,
        detail: detail.into(),
    }
}

pub(crate) fn bounds(space: Space, e: BoundsError) -> Error {
    Error::Format {
        space,
        offset: e.absolute(),
        detail: e.to_string(),
    }
}
