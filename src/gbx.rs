//! The Gbx container: header, user data (the header chunks), reference table and body.
//!
//! Layout (version 6, the version read here): `GBX`; the version (`u16`); the format (`B`, binary: the text format
//! is refused); the reference table's and the body's compression (`U` or `C` each); a byte `R` or `E` (release or
//! editor); the main node's class id (`u32`); the user data: its length (`u32`), then, when it is not 0, the number
//! of header chunks, a table of their ids and sizes (bit 31 of a size marks a "heavy" chunk) and their data, which
//! must fill the length exactly; the number of nodes (`u32`); the reference table: the number of external nodes,
//! and when that is not 0 the ancestor level, a folder tree (each folder a name and its own folders, in depth-first
//! order) and each external node; then the body. A compressed body is its decompressed size (`u32`), its compressed
//! size (`u32`) and LZO1X data, which ends the file; an uncompressed body is the rest of the file.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, State};
use crate::error::{Result, Space, bad};
use crate::node::{NodeTable, Nodes};
use primase_foundations::compress::{Decompressor, Lzo1x};
use std::borrow::Cow;

/// The Gbx versions read.
const VERSIONS: [u16; 1] = [6];

/// Whether a part of the file is compressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    /// `U`.
    Uncompressed,
    /// `C`: LZO1X.
    Compressed,
}

/// A header chunk as stored: its data is read by the main class's header reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderChunk<'a> {
    /// The chunk id.
    pub id: u32,
    /// Bit 31 of its stored size.
    pub heavy: bool,
    /// The file offset of its data.
    pub offset: usize,
    /// Its data.
    pub bytes: &'a [u8],
}

/// A folder of the reference table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    /// Its name.
    pub name: String,
    /// The folder it is in (an index into [`RefTable::folders`]), `None` at the top.
    pub parent: Option<usize>,
}

/// What an external node is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalTarget {
    /// A file: its name and the folder it is in (an index into [`RefTable::folders`], `None` at the top).
    File {
        /// The file's name.
        name: String,
        /// Its folder.
        folder: Option<usize>,
    },
    /// A resource, by index (flag bit 2).
    Resource(u32),
}

/// A node the body refers to but does not hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct External {
    /// Its flags as stored (bit 2: a resource).
    pub flags: u32,
    /// The file or resource.
    pub target: ExternalTarget,
    /// The node index references to it use.
    pub node_index: u32,
    /// The stored "use file" bool.
    pub use_file: bool,
}

/// The reference table: the nodes a body names but other files hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RefTable {
    /// How many folders up from the file's own its paths start.
    pub ancestor_level: u32,
    /// Every folder, depth-first.
    pub folders: Vec<Folder>,
    /// The external nodes.
    pub external: Vec<External>,
}

/// The body as stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body<'a> {
    /// LZO1X data and the size it decompresses to.
    Compressed {
        /// The decompressed size.
        size: usize,
        /// The file offset of the compressed data.
        offset: usize,
        /// The compressed data.
        data: &'a [u8],
    },
    /// The body's own bytes.
    Uncompressed {
        /// The file offset of the body.
        offset: usize,
        /// The body.
        data: &'a [u8],
    },
}

/// A Gbx file's container, borrowing its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gbx<'a> {
    /// The container version (6).
    pub version: u16,
    /// The reference table's compression byte.
    pub ref_table_compression: Compression,
    /// `true` for `R` (release), `false` for `E` (editor).
    pub release: bool,
    /// The main node's class id, as stored.
    pub class_id: u32,
    /// The header chunks, in order.
    pub header_chunks: Vec<HeaderChunk<'a>>,
    /// The number of nodes the body and reference table index.
    pub nodes: u32,
    /// The reference table.
    pub ref_table: RefTable,
    /// The body.
    pub body: Body<'a>,
}

fn compression(r: &mut Archive<'_, '_>, what: &str) -> Result<Compression> {
    let at = r.here();
    match r.u8()? {
        b'U' => Ok(Compression::Uncompressed),
        b'C' => Ok(Compression::Compressed),
        b => Err(r.bad_at(at, format!("{what} compression {b:#04x}: expected U or C"))),
    }
}

/// The folders under one folder, depth-first, each followed by its own.
fn folders(r: &mut Archive<'_, '_>, parent: Option<usize>, out: &mut Vec<Folder>, depth: u32) -> Result<()> {
    if depth > 64 {
        return Err(r.bad("reference table folders nested more than 64 deep"));
    }
    let n = r.count(8)?;
    for _ in 0..n {
        let name = r.string()?;
        let index = out.len();
        out.push(Folder { name, parent });
        folders(r, Some(index), out, depth.saturating_add(1))?;
    }
    Ok(())
}

fn ref_table(r: &mut Archive<'_, '_>) -> Result<RefTable> {
    let n = r.count(16)?;
    if n == 0 {
        return Ok(RefTable::default());
    }
    let ancestor_level = r.u32()?;
    let mut table = RefTable {
        ancestor_level,
        ..RefTable::default()
    };
    folders(r, None, &mut table.folders, 0)?;
    for _ in 0..n {
        let flags = r.u32()?;
        let name_or_resource = if flags & 4 != 0 {
            Err(r.u32()?)
        } else {
            Ok(r.string()?)
        };
        let node_index = r.u32()?;
        let use_file = r.bool()?;
        let target = match name_or_resource {
            Err(resource) => ExternalTarget::Resource(resource),
            Ok(name) => {
                let at = r.here();
                let folder = match r.u32()? {
                    0 => None,
                    k => {
                        let k = usize::try_from(k).map_or(usize::MAX, |k| k.saturating_sub(1));
                        if k >= table.folders.len() {
                            return Err(
                                r.bad_at(at, format!("folder {k}: the table has {}", table.folders.len()))
                            );
                        }
                        Some(k)
                    }
                };
                ExternalTarget::File { name, folder }
            }
        };
        table.external.push(External {
            flags,
            target,
            node_index,
            use_file,
        });
    }
    Ok(table)
}

/// The container up to its body, read from any archive: the header chunks as spans of the file.
#[derive(Debug)]
pub(crate) struct Head {
    pub(crate) version: u16,
    pub(crate) ref_table_compression: Compression,
    pub(crate) body_compression: Compression,
    pub(crate) release: bool,
    pub(crate) class_id: u32,
    /// Each header chunk's id, heavy bit, and data's offset and length.
    pub(crate) header_chunks: Vec<(u32, bool, usize, usize)>,
    pub(crate) nodes: u32,
    pub(crate) ref_table: RefTable,
}

/// Reads the container from its first byte up to its body.
pub(crate) fn head(r: &mut Archive<'_, '_>) -> Result<Head> {
    if r.array::<3>()? != *b"GBX" {
        return Err(bad(Space::File, 0, "not a Gbx file: expected GBX"));
    }
    let at = r.here();
    let version = r.u16()?;
    if !VERSIONS.contains(&version) {
        return Err(r.bad_at(at, format!("Gbx version {version}: expected one of {VERSIONS:?}")));
    }
    let at = r.here();
    match r.u8()? {
        b'B' => {}
        b'T' => return Err(r.bad_at(at, "text-format Gbx: only the binary format is read")),
        b => return Err(r.bad_at(at, format!("format {b:#04x}: expected B"))),
    }
    let ref_table_compression = compression(r, "reference table")?;
    let body_compression = compression(r, "body")?;
    let at = r.here();
    let release = match r.u8()? {
        b'R' => true,
        b'E' => false,
        b => return Err(r.bad_at(at, format!("mode {b:#04x}: expected R or E"))),
    };
    let class_id = r.u32()?;
    let user_data = r.data()?;
    let header_chunks = header_chunks(&user_data, r.here().saturating_sub(user_data.len()))?;
    let nodes = r.u32()?;
    let ref_table = ref_table(r)?;
    if ref_table_compression == Compression::Compressed && !ref_table.external.is_empty() {
        return Err(r.bad("a compressed reference table: only uncompressed ones are read"));
    }
    Ok(Head {
        version,
        ref_table_compression,
        body_compression,
        release,
        class_id,
        header_chunks,
        nodes,
        ref_table,
    })
}

/// Reads the nodes of a body from `r`, which is at its start: the main node, of class `class_id`, and every node its
/// references read. The body must end with the main node.
pub(crate) fn body_nodes(r: &mut Archive<'_, '_>, class_id: u32) -> Result<crate::classes::Node> {
    let at = r.here();
    let main = crate::classes::read_node(r, class_id, at)?;
    r.end("the body")?;
    Ok(main)
}

/// The node table of a body, given the container's node count and reference table.
pub(crate) fn node_table(nodes: u32, ref_table: &RefTable) -> Result<NodeTable> {
    if nodes == 0 {
        return Err(bad(
            Space::File,
            0,
            "the header counts no nodes, not even the main one",
        ));
    }
    let external = ref_table.external.iter().map(|e| e.node_index).collect();
    let mut table = NodeTable::new(nodes, external);
    table.reserve(0);
    Ok(table)
}

impl<'a> Gbx<'a> {
    /// Reads the container. The header chunks and the body are kept as stored.
    pub fn parse(bytes: &'a [u8]) -> Result<Gbx<'a>> {
        let mut st = State::new(NodeTable::new(0, Vec::new()));
        let mut r = Archive::new(bytes, 0, Space::File, &mut st);
        let head = head(&mut r)?;
        let header_chunks = head
            .header_chunks
            .iter()
            .map(|&(id, heavy, offset, len)| HeaderChunk {
                id,
                heavy,
                offset,
                // within the user data, which was read from `bytes`
                bytes: bytes.get(offset..offset.saturating_add(len)).unwrap_or_default(),
            })
            .collect();
        let body = match head.body_compression {
            Compression::Compressed => {
                let size = r.count(0)?;
                let n = r.count(1)?;
                let offset = r.here();
                r.bytes(n)?;
                r.end("the compressed body")?;
                Body::Compressed {
                    size,
                    offset,
                    data: bytes.get(offset..offset.saturating_add(n)).unwrap_or_default(),
                }
            }
            Compression::Uncompressed => {
                let offset = r.here();
                Body::Uncompressed {
                    offset,
                    data: bytes.get(offset..).unwrap_or_default(),
                }
            }
        };
        Ok(Gbx {
            version: head.version,
            ref_table_compression: head.ref_table_compression,
            release: head.release,
            class_id: head.class_id,
            header_chunks,
            nodes: head.nodes,
            ref_table: head.ref_table,
            body,
        })
    }

    /// The body's bytes, decompressed when stored compressed.
    pub fn body(&self) -> Result<Cow<'a, [u8]>> {
        match self.body {
            Body::Compressed { size, data, .. } => Ok(Cow::Owned(Lzo1x.decompress(data, size)?)),
            Body::Uncompressed { data, .. } => Ok(Cow::Borrowed(data)),
        }
    }

    /// Reads the nodes of `body` (the bytes [`Gbx::body`] returns): the main node, of the class the header names,
    /// and every node its references read. The body must end with the main node.
    pub fn read_nodes(&self, body: &[u8]) -> Result<Nodes> {
        let mut st = State::new(node_table(self.nodes, &self.ref_table)?);
        let mut r = Archive::new(body, 0, Space::Body, &mut st);
        let main = body_nodes(&mut r, self.class_id)?;
        Ok(st.nodes.into_nodes(main))
    }

    /// [`Gbx::body`] and [`Gbx::read_nodes`] in one.
    pub fn nodes(&self) -> Result<Nodes> {
        self.read_nodes(&self.body()?)
    }
}

fn header_chunks(user_data: &[u8], origin: usize) -> Result<Vec<(u32, bool, usize, usize)>> {
    if user_data.is_empty() {
        return Ok(Vec::new());
    }
    let mut st = State::new(NodeTable::new(0, Vec::new()));
    let mut r = Archive::new(user_data, origin, Space::File, &mut st);
    let n = r.count(8)?;
    let mut table = Vec::with_capacity(n);
    for _ in 0..n {
        let id = r.u32()?;
        let size = r.u32()?;
        table.push((id, size & 0x8000_0000 != 0, size & 0x7FFF_FFFF));
    }
    let mut chunks = Vec::with_capacity(n);
    for (id, heavy, size) in table {
        let offset = r.here();
        let size = usize::try_from(size).unwrap_or(usize::MAX);
        r.bytes(size)?;
        chunks.push((id, heavy, offset, size));
    }
    r.end("the user data")?;
    Ok(chunks)
}
