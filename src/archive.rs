//! The values chunks are made of, and the chunk stream of a node.
//!
//! Numbers are little-endian. A bool is a `u32` that is 0 or 1. A string is an `i32` byte length and that many bytes
//! of UTF-8. An [`Id`] is a "lookback string": the first id of a scope is preceded by the id version (`u32`, 3);
//! then each id is a `u32` that is `0xFFFFFFFF` for none, a plain number when its top two bits are neither `01` nor
//! `10`, a new string (which follows) when those bits are `01` or `10` and the low 30 bits are 0, and otherwise a
//! string added earlier: the low 30 bits are its 1-based position among the scope's strings, and the top two bits
//! those it was added with (`0x40000002`: the second string, added with bits `01`). Each header chunk is a scope of
//! its own; the body is one scope.
//!
//! A node reference is an `i32`: -1 for none, an index already read (or one the reference table lists) for that
//! node, or a new index followed by the node's class id and the node itself. A node is a stream of chunks, each a
//! `u32` id and its data, ended by `0xFACADE01`. A chunk stored "skippable" puts `PIKS` (`0x534B4950`) and its size
//! (`u32`) before its data; any other chunk's size is known only to a reader that understands it.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::error::{Error, Result, Space, bad, bounds};
use crate::node::{Chunks, NodeIndex, NodeTable, RawChunk};
use primase_foundations::bytes::{BoundsError, Cursor, Endian::Little, checked_span};
use std::borrow::Cow;

/// A span error from [`checked_span`] (which counts from 0) given the archive's origin, as a cursor reports it.
fn origin_error(e: BoundsError, origin: usize) -> BoundsError {
    match e {
        BoundsError::LengthOverflow {
            offset,
            count,
            stride,
            ..
        } => BoundsError::LengthOverflow {
            offset,
            origin,
            count,
            stride,
        },
        BoundsError::OutOfBounds {
            offset,
            length,
            total,
            ..
        } => BoundsError::OutOfBounds {
            offset,
            origin,
            length,
            total,
        },
    }
}

/// The skip marker before a skippable chunk's size: `PIKS` read as a little-endian `u32`.
pub(crate) const SKIP: u32 = 0x534B_4950;
/// The end of a node's chunks.
pub(crate) const FACADE: u32 = 0xFACA_DE01;
/// The id version every scope of the files read here begins with.
const ID_VERSION: u32 = 3;

/// A lookback id as stored (see the module documentation).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Id {
    /// `0xFFFFFFFF`: no id.
    #[default]
    Empty,
    /// A number (a collection's id, for instance).
    Number(u32),
    /// A string, stored here or earlier in the same scope.
    Name(String),
}

impl Id {
    /// The string, if the id is one.
    pub fn name(&self) -> Option<&str> {
        match self {
            Id::Name(s) => Some(s),
            _ => None,
        }
    }
}

/// Three ids naming an object: its id, its collection and its author.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Ident {
    /// The object's id.
    pub id: Id,
    /// The collection (environment) it belongs to.
    pub collection: Id,
    /// Its author.
    pub author: Id,
}

/// A reference to a pack file: a skin, a mod or a music file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackDesc {
    /// Its version byte.
    pub version: u8,
    /// A SHA-256 checksum, stored from version 3.
    pub checksum: Option<[u8; 32]>,
    /// The file's path.
    pub file_path: String,
    /// Where to download it, stored when the path is not empty (version 1 and 2) or always (version 3).
    pub locator_url: Option<String>,
}

/// The id lookback strings of one scope.
#[derive(Debug, Default)]
pub(crate) struct Ids {
    version: Option<u32>,
    /// Each string with the two flag bits it was added with.
    names: Vec<(u32, String)>,
}

/// What a read keeps between chunks: the id scope, the nodes read so far and, for a file read from a pack, the stream
/// its plaintext comes from.
#[derive(Debug)]
pub(crate) struct State {
    pub(crate) ids: Ids,
    pub(crate) nodes: NodeTable,
    /// The pack file whose plaintext a [`Source::Stream`] archive reads.
    pub(crate) stream: Option<crate::pak::FileStream>,
}

impl State {
    pub(crate) fn new(nodes: NodeTable) -> State {
        State {
            ids: Ids::default(),
            nodes,
            stream: None,
        }
    }
}

/// Where an archive's bytes come from.
#[derive(Debug)]
enum Source<'a> {
    /// Bytes given whole.
    Slice(Cursor<'a>),
    /// A pack file's plaintext, decrypted as the read reaches it ([`State::stream`]): `len` bytes from the archive's
    /// origin, `position` of them read.
    Stream { position: usize, len: usize },
}

/// A reader over one span of bytes (a header chunk, the body, a skippable chunk's data), sharing the read's
/// [`State`].
#[derive(Debug)]
pub(crate) struct Archive<'a, 's> {
    src: Source<'a>,
    /// The offset of the span's first byte in `space`.
    origin: usize,
    space: Space,
    pub(crate) st: &'s mut State,
}

impl<'a, 's> Archive<'a, 's> {
    pub(crate) fn new(bytes: &'a [u8], origin: usize, space: Space, st: &'s mut State) -> Self {
        Archive {
            src: Source::Slice(Cursor::with_origin(bytes, origin)),
            origin,
            space,
            st,
        }
    }

    /// An archive over the plaintext of the pack file in `st.stream`, from its first byte: `len` bytes.
    pub(crate) fn stream(len: usize, st: &'s mut State) -> Self {
        Archive {
            src: Source::Stream { position: 0, len },
            origin: 0,
            space: Space::File,
            st,
        }
    }

    fn position(&self) -> usize {
        match &self.src {
            Source::Slice(cur) => cur.position(),
            Source::Stream { position, .. } => *position,
        }
    }

    /// The offset of the next byte, in the archive's space.
    pub(crate) fn here(&self) -> usize {
        self.origin.saturating_add(self.position())
    }

    /// An error at the next byte.
    pub(crate) fn bad(&self, detail: impl Into<String>) -> Error {
        bad(self.space, self.here(), detail)
    }

    /// An error at `at`, an offset in the archive's space.
    pub(crate) fn bad_at(&self, at: usize, detail: impl Into<String>) -> Error {
        bad(self.space, at, detail)
    }

    pub(crate) fn space(&self) -> Space {
        self.space
    }

    pub(crate) fn remaining(&self) -> usize {
        match &self.src {
            Source::Slice(cur) => cur.remaining(),
            Source::Stream { position, len } => len.saturating_sub(*position),
        }
    }

    /// Refuses bytes left over: a span must be read to its end.
    pub(crate) fn end(&self, what: &str) -> Result<()> {
        match self.remaining() {
            0 => Ok(()),
            n => Err(self.bad(format!("{n} bytes left at the end of {what}"))),
        }
    }

    /// The next `n` bytes: borrowed from bytes given whole, copied from a stream.
    pub(crate) fn bytes(&mut self, n: usize) -> Result<Cow<'a, [u8]>> {
        let space = self.space;
        match &mut self.src {
            Source::Slice(cur) => cur.take(n).map(Cow::Borrowed).map_err(|e| bounds(space, e)),
            Source::Stream { position, len } => {
                let span = checked_span(*position, n, 1, *len)
                    .map_err(|e| bounds(space, origin_error(e, self.origin)))?;
                let (start, end) = (
                    self.origin.saturating_add(span.start),
                    self.origin.saturating_add(span.end),
                );
                let stream = self
                    .st
                    .stream
                    .as_mut()
                    .ok_or_else(|| bad(space, start, "no pack stream to read"))?;
                let bytes = stream.read(start, end)?.to_vec();
                *position = span.end;
                Ok(Cow::Owned(bytes))
            }
        }
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let space = self.space;
        if let Source::Slice(cur) = &mut self.src {
            return cur.array::<N>().map_err(|e| bounds(space, e));
        }
        let bytes = self.bytes(N)?;
        let mut out = [0; N];
        out.copy_from_slice(&bytes);
        Ok(out)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        self.array::<1>().map(|[b]| b)
    }

    pub(crate) fn u16(&mut self) -> Result<u16> {
        self.array().map(u16::from_le_bytes)
    }

    pub(crate) fn i16(&mut self) -> Result<i16> {
        self.array().map(i16::from_le_bytes)
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        self.array().map(u32::from_le_bytes)
    }

    pub(crate) fn i32(&mut self) -> Result<i32> {
        self.array().map(i32::from_le_bytes)
    }

    pub(crate) fn u64(&mut self) -> Result<u64> {
        self.array().map(u64::from_le_bytes)
    }

    pub(crate) fn f32(&mut self) -> Result<f32> {
        self.array().map(f32::from_le_bytes)
    }

    pub(crate) fn vec2(&mut self) -> Result<[f32; 2]> {
        Ok([self.f32()?, self.f32()?])
    }

    pub(crate) fn vec3(&mut self) -> Result<[f32; 3]> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }

    pub(crate) fn vec4(&mut self) -> Result<[f32; 4]> {
        Ok([self.f32()?, self.f32()?, self.f32()?, self.f32()?])
    }

    pub(crate) fn int3(&mut self) -> Result<[i32; 3]> {
        Ok([self.i32()?, self.i32()?, self.i32()?])
    }

    pub(crate) fn byte3(&mut self) -> Result<[u8; 3]> {
        self.array()
    }

    /// A `u32` bool: 0 or 1, anything else refused.
    pub(crate) fn bool(&mut self) -> Result<bool> {
        let at = self.here();
        match self.u32()? {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(self.bad_at(at, format!("bool {n}: expected 0 or 1"))),
        }
    }

    /// A `u32` count of entries of at least `each` bytes that the rest of the span can hold.
    pub(crate) fn count(&mut self, each: usize) -> Result<usize> {
        let space = self.space;
        if let Source::Slice(cur) = &mut self.src {
            return cur.count(Little, each).map_err(|e| bounds(space, e));
        }
        let start = self.position();
        let n = usize::try_from(self.u32()?).unwrap_or(usize::MAX);
        let after = self.position();
        let origin = self.origin;
        if let Source::Stream { position, len } = &mut self.src
            && let Err(e) = checked_span(after, n, each, *len)
        {
            // as a cursor does: the archive stays before the count
            *position = start;
            return Err(bounds(space, origin_error(e, origin)));
        }
        Ok(n)
    }

    /// A counted list, each entry at least `each` bytes.
    pub(crate) fn list<T>(
        &mut self,
        each: usize,
        mut f: impl FnMut(&mut Self) -> Result<T>,
    ) -> Result<Vec<T>> {
        let n = self.count(each)?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(f(self)?);
        }
        Ok(out)
    }

    /// Counted bytes: an `i32` length and the bytes.
    pub(crate) fn data(&mut self) -> Result<Cow<'a, [u8]>> {
        let n = self.count(1)?;
        self.bytes(n)
    }

    /// A string: an `i32` byte length and UTF-8.
    pub(crate) fn string(&mut self) -> Result<String> {
        let at = self.here();
        let bytes = self.data()?;
        std::str::from_utf8(&bytes)
            .map(str::to_owned)
            .map_err(|e| self.bad_at(at, format!("string of {} bytes is not UTF-8: {e}", bytes.len())))
    }

    /// A string with a one-byte length (`StringLengthPrefix.Byte` in GBX.NET): UTF-8.
    pub(crate) fn string8(&mut self) -> Result<String> {
        let at = self.here();
        let n = usize::from(self.u8()?);
        let bytes = self.bytes(n)?;
        std::str::from_utf8(&bytes)
            .map(str::to_owned)
            .map_err(|e| self.bad_at(at, format!("string of {n} bytes is not UTF-8: {e}")))
    }

    /// A bool stored as one byte: 0 or 1, anything else refused.
    pub(crate) fn bool8(&mut self) -> Result<bool> {
        let at = self.here();
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(self.bad_at(at, format!("bool {n}: expected 0 or 1"))),
        }
    }

    /// A version number (`u32`) that must be one of `known`.
    pub(crate) fn version(&mut self, what: &str, known: &[u32]) -> Result<u32> {
        let at = self.here();
        let v = self.u32()?;
        if known.contains(&v) {
            Ok(v)
        } else {
            Err(self.bad_at(at, format!("{what} version {v}: expected one of {known:?}")))
        }
    }

    /// A one-byte version number that must be one of `known`.
    pub(crate) fn version8(&mut self, what: &str, known: &[u8]) -> Result<u8> {
        let at = self.here();
        let v = self.u8()?;
        if known.contains(&v) {
            Ok(v)
        } else {
            Err(self.bad_at(at, format!("{what} version {v}: expected one of {known:?}")))
        }
    }

    /// A lookback id (see the module documentation).
    pub(crate) fn id(&mut self) -> Result<Id> {
        if self.st.ids.version.is_none() {
            let at = self.here();
            let v = self.u32()?;
            if v != ID_VERSION {
                return Err(self.bad_at(at, format!("id version {v}: expected {ID_VERSION}")));
            }
            self.st.ids.version = Some(v);
        }
        let at = self.here();
        let v = self.u32()?;
        if v == u32::MAX {
            return Ok(Id::Empty);
        }
        let flags = v & 0xC000_0000;
        if flags != 0x4000_0000 && flags != 0x8000_0000 {
            return Ok(Id::Number(v));
        }
        let back = v & 0x3FFF_FFFF;
        if back == 0 {
            let s = self.string()?;
            self.st.ids.names.push((flags, s.clone()));
            return Ok(Id::Name(s));
        }
        let names = &self.st.ids.names;
        let index = usize::try_from(back).map_or(usize::MAX, |b| b.saturating_sub(1));
        match names.get(index) {
            Some((f, s)) if *f == flags => Ok(Id::Name(s.clone())),
            _ => Err(self.bad_at(
                at,
                format!(
                    "id {v:#010x} refers back to a string not added with those flags ({} added)",
                    names.len()
                ),
            )),
        }
    }

    pub(crate) fn ident(&mut self) -> Result<Ident> {
        Ok(Ident {
            id: self.id()?,
            collection: self.id()?,
            author: self.id()?,
        })
    }

    pub(crate) fn pack_desc(&mut self) -> Result<PackDesc> {
        let version = self.version8("pack description", &[1, 2, 3])?;
        let checksum = if version >= 3 { Some(self.array()?) } else { None };
        let file_path = self.string()?;
        let locator_url = if (!file_path.is_empty() && version >= 1) || version >= 3 {
            Some(self.string()?)
        } else {
            None
        };
        Ok(PackDesc {
            version,
            checksum,
            file_path,
            locator_url,
        })
    }

    /// The dummy write a pack file's reader makes (see [`crate::pak`]): `data` folded into the cipher's chain, when
    /// this archive reads a pack file's stream directly (not a buffer of its own, as a skippable chunk is).
    pub(crate) fn dummy_write(&mut self, data: [u8; 4]) {
        if let (Source::Stream { .. }, Some(stream)) = (&self.src, self.st.stream.as_mut()) {
            stream.dummy_write(data);
        }
    }

    /// A node stored in place, without a class id or an index (the game archives it with the class known from the
    /// field): `read` reads its chunks. A pack file's reader makes the dummy write of `parent`, the id of the
    /// node's parent class (see [`crate::classes`]), as it does at the start of every node.
    pub(crate) fn direct<T>(&mut self, parent: u32, read: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        self.dummy_write(u32::to_le_bytes(parent));
        read(self)
    }

    /// A node reference (see the module documentation): `None` for -1. A new index reads its node into the table.
    pub(crate) fn node_ref(&mut self) -> Result<Option<NodeIndex>> {
        let at = self.here();
        let raw = self.i32()?;
        if raw == -1 {
            return Ok(None);
        }
        let index = u32::try_from(raw).map_err(|_| self.bad_at(at, format!("node index {raw}")))?;
        let table = &self.st.nodes;
        if table.is_taken(index) {
            return Ok(Some(NodeIndex(index)));
        }
        if !table.has_slot(index) {
            return Err(self.bad_at(
                at,
                format!("node index {index}: the header counts {} nodes", table.count()),
            ));
        }
        let class_at = self.here();
        let class = self.u32()?;
        self.st.nodes.reserve(index);
        let node = crate::classes::read_node(self, class, class_at)?;
        self.st.nodes.fill(index, node);
        Ok(Some(NodeIndex(index)))
    }

    /// A chunk stored skippable: the marker, its size, and `f` reading exactly that many bytes. The game archives a
    /// skippable chunk through a buffer of its own, so a pack file's reader makes no dummy writes within it.
    pub(crate) fn skippable<T>(
        &mut self,
        f: impl for<'b> FnOnce(&mut Archive<'b, '_>) -> Result<T>,
    ) -> Result<T> {
        let at = self.here();
        let marker = self.u32()?;
        if marker != SKIP {
            return Err(self.bad_at(at, format!("{marker:#010x}: expected the skip marker PIKS")));
        }
        let size = self.count(1)?;
        let origin = self.here();
        let bytes = self.bytes(size)?;
        let mut sub = Archive::new(&bytes, origin, self.space, &mut *self.st);
        let out = f(&mut sub)?;
        sub.end("the skippable chunk")?;
        Ok(out)
    }

    /// A node's chunks, up to and including `0xFACADE01`. `each` reads the chunk whose id it is given and returns
    /// true, or returns false having read nothing; such a chunk is kept as stored if it is skippable and refused
    /// otherwise. A chunk id may appear only once.
    pub(crate) fn chunks(
        &mut self,
        class: &'static str,
        mut each: impl FnMut(&mut Self, u32) -> Result<bool>,
    ) -> Result<Chunks> {
        let mut chunks = Chunks::default();
        loop {
            let at = self.here();
            let id = self.u32()?;
            if id == FACADE {
                return Ok(chunks);
            }
            if chunks.ids.contains(&id) {
                return Err(self.bad_at(at, format!("chunk {id:#010x} of {class} a second time")));
            }
            chunks.ids.push(id);
            if each(self, id)? {
                continue;
            }
            let marker_at = self.here();
            if self.u32()? != SKIP {
                return Err(Error::UnknownChunk {
                    space: self.space,
                    offset: at as u128,
                    chunk: id,
                    class,
                });
            }
            let size = self
                .count(1)
                .map_err(|_| self.bad_at(marker_at, "skippable chunk size"))?;
            let offset = self.here();
            let bytes = self.bytes(size)?.into_owned();
            chunks.unread.push(RawChunk { id, offset, bytes });
        }
    }
}
