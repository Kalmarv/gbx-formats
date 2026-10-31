//! TrackMania Nations Forever's pack files (`.pak`, "NadeoPak" version 3) and the key list that unlocks them
//! (`packlist.dat`).
//!
//! **The key list** (`packlist.dat`): a version byte, the number of packs (`u8`), a CRC-32 and a salt (`u32` each),
//! then per pack a flags byte, its name's length (`u8`), its name and a 32-character hexadecimal key string, then a
//! 16-byte signature. Each name is XORed with `MD5("6611992868945B0B59536FC3226F3FD0" + salt)` (the salt in decimal),
//! each key string with `MD5(name + salt + "B97C1205648A66E04F86A1B5D5AF9862")`, both cycling the 16 hash bytes. The
//! key string's 16 bytes are the pack's base key ([`PackKey`]).
//!
//! **A pack**: `NadeoPak`, its version (`i32`, 3), the header's IV (`u64`) and the header, encrypted with Blowfish
//! keyed by `MD5(hex(base key) + "NadeoPak")` (the hex in capitals) in the chained mode [`FileStream`] describes. The
//! header: an MD5 (16 bytes), where the Gbx headers start, where the file data starts ("header max size"), the Gbx
//! headers' size and compressed size (`u32` each), a 128-bit pack mask, flags (`u32`), the folders (a count, then each
//! folder's parent index, -1 at the top, and name) and the files (a count, then each file's folder index, name, an
//! unknown `u32`, its size and compressed size, its data's offset from the data start, its class id and its flags,
//! `u64`). After the folders, before the files, the header's reader makes a dummy write of bytes 4 to 8 of the third
//! folder's name (with a trailing `\`) in UTF-16, when that name is longer than 4 characters.
//!
//! **A file's data** starts with its IV (`u64`), unless its flags store it in the clear (bit 49 public, or bit 50 no
//! encryption); it is compressed (zlib) when any of bits 2 to 5 is set. A file's name is stored as a hash of it
//! ([`name_hash`]) unless the pack lists its real name.
//!
//! **Dummy writes.** The game reads a Gbx file from a pack straight through the cipher, and as it starts reading each
//! node it writes the 4 bytes of the node's parent class id into the cipher, which folds them into the chain at the
//! next 256-byte chunk ([`FileStream`]). So a Gbx file decrypts only when its nodes are read as it is decrypted:
//! [`Pak::read`] reads each Gbx file's nodes with this crate's classes as it decrypts it, and refuses a file holding a
//! node of a class it does not read. A file whose flags set bit 32 is read without dummy writes, as is a file that is
//! not a Gbx file.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

mod blowfish;
mod pi;
pub(crate) mod stream;

use crate::archive::{Archive, State};
use crate::error::{Result, Space, bad};
use crate::gbx::{Compression, HeaderChunk, RefTable, body_nodes, head, node_table};
use crate::node::NodeTable;
use blowfish::Blowfish;
use primase_foundations::hash::md5;
use std::collections::HashMap;
use std::sync::Arc;
pub use stream::FileStream;

/// The pack versions read: TrackMania Nations Forever's.
const VERSIONS: [u32; 1] = [3];
/// What a key list's names are masked with, before the salt.
const NAME_KEY_SALT: &str = "6611992868945B0B59536FC3226F3FD0";
/// What a key list's key strings are masked with, after the name and salt.
const KEY_STRING_SALT: &str = "B97C1205648A66E04F86A1B5D5AF9862";

/// A pack's entry in the key list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackKey {
    /// The pack's name (its file name without `.pak`).
    pub name: String,
    /// Its flags byte.
    pub flags: u8,
    /// Its base key.
    pub key: [u8; 16],
}

/// The key list, `packlist.dat`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackList {
    /// Its version byte.
    pub version: u8,
    /// The stored CRC-32.
    pub crc32: u32,
    /// The salt the names and keys are masked with.
    pub salt: u32,
    /// Each pack's name and key.
    pub packs: Vec<PackKey>,
    /// The closing 16 bytes.
    pub signature: [u8; 16],
}

/// Reads a key list's bytes in sequence.
struct Bytes<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize, what: &str) -> Result<&'a [u8]> {
        let span = self
            .bytes
            .get(self.at..self.at.saturating_add(n))
            .ok_or_else(|| bad(Space::Pack, self.at, format!("{what}: {n} bytes past the end")))?;
        self.at = self.at.saturating_add(n);
        Ok(span)
    }

    fn array<const N: usize>(&mut self, what: &str) -> Result<[u8; N]> {
        let mut out = [0; N];
        out.copy_from_slice(self.take(N, what)?);
        Ok(out)
    }
}

/// `bytes` XORed with `key`, cycling it.
fn unmask(bytes: &[u8], key: &[u8; 16]) -> Vec<u8> {
    bytes.iter().zip(key.iter().cycle()).map(|(b, k)| b ^ k).collect()
}

/// Two hexadecimal digits per byte.
fn parse_hex<const N: usize>(text: &[u8]) -> Option<[u8; N]> {
    let digit = |c: u8| char::from(c).to_digit(16).and_then(|d| u8::try_from(d).ok());
    let mut out = [0; N];
    if text.len() != N.checked_mul(2)? {
        return None;
    }
    for (o, &[hi, lo]) in out.iter_mut().zip(text.as_chunks::<2>().0) {
        *o = (digit(hi)? << 4) | digit(lo)?;
    }
    Some(out)
}

impl PackList {
    /// Reads a key list.
    pub fn parse(bytes: &[u8]) -> Result<PackList> {
        let mut r = Bytes { bytes, at: 0 };
        let [version, count] = r.array("version and pack count")?;
        let crc32 = u32::from_le_bytes(r.array("CRC-32")?);
        let salt = u32::from_le_bytes(r.array("salt")?);
        let name_key = md5(format!("{NAME_KEY_SALT}{salt}").as_bytes());
        let mut packs = Vec::with_capacity(usize::from(count));
        for i in 0..count {
            let [flags, len] = r.array("pack flags and name length")?;
            let at = r.at;
            let name = unmask(r.take(usize::from(len), "pack name")?, &name_key);
            let name = String::from_utf8(name)
                .map_err(|_| bad(Space::Pack, at, format!("pack {i}: its name is not text")))?;
            let at = r.at;
            let key_key = md5(format!("{name}{salt}{KEY_STRING_SALT}").as_bytes());
            let key_string = unmask(r.take(32, "pack key")?, &key_key);
            let key = parse_hex(&key_string).ok_or_else(|| {
                bad(
                    Space::Pack,
                    at,
                    format!("pack {name}: its key is not 32 hexadecimal digits"),
                )
            })?;
            packs.push(PackKey { name, flags, key });
        }
        let signature = r.array("signature")?;
        if r.at != bytes.len() {
            return Err(bad(
                Space::Pack,
                r.at,
                format!("{} bytes after the signature", bytes.len().saturating_sub(r.at)),
            ));
        }
        Ok(PackList {
            version,
            crc32,
            salt,
            packs,
            signature,
        })
    }

    /// The key of the pack named `name` (case-insensitive, as the game's file system is).
    pub fn key(&self, name: &str) -> Option<&PackKey> {
        self.packs.iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }
}

/// The name a pack stores for a file whose real name it hides: the name in lower case as UTF-8, hashed with MD5,
/// with its length (`u8`) before the 16 hash bytes, each of the 17 bytes written as two hexadecimal digits (capitals),
/// the low digit first.
pub fn name_hash(name: &str) -> String {
    let lower = name.to_lowercase();
    let bytes = lower.as_bytes();
    let mut out = String::with_capacity(34);
    // the length's low byte, as the game stores it
    let [len, ..] = bytes.len().to_le_bytes();
    for b in std::iter::once(len).chain(md5(bytes)) {
        for nibble in [b & 0xF, b >> 4] {
            out.push(char::from_digit(u32::from(nibble), 16).map_or('0', |c| c.to_ascii_uppercase()));
        }
    }
    out
}

/// A folder of a pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PakFolder {
    /// The folder it is in (an index into [`Pak::folders`]), `None` at the top.
    pub parent: Option<usize>,
    /// Its name as stored, with a trailing `\` added when it has none (as the game reads it).
    pub name: String,
}

/// A file of a pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PakFile {
    /// The folder it is in (an index into [`Pak::folders`]), `None` at the top.
    pub folder: Option<usize>,
    /// Its name as stored: its real name, or [`name_hash`] of it.
    pub name: String,
    /// The `u32` after the name, not understood.
    pub u01: u32,
    /// Its size.
    pub size: u32,
    /// Its compressed size.
    pub compressed_size: u32,
    /// Where its data starts, from the pack's data start ([`Pak::data_start`]).
    pub offset: u32,
    /// The class of its main node (0 for a file that holds none).
    pub class_id: u32,
    /// Its flags.
    pub flags: u64,
}

impl PakFile {
    /// Read without dummy writes (bit 32).
    pub fn dont_use_dummy_write(&self) -> bool {
        self.flags & 0x1_0000_0000 != 0
    }

    /// Stored in the clear: neither bit 49 (public) nor bit 50 (no encryption) is set.
    pub fn is_encrypted(&self) -> bool {
        self.flags & 0x6_0000_0000_0000 == 0
    }

    /// Compressed with zlib: one of bits 2 to 5 is set.
    pub fn is_compressed(&self) -> bool {
        self.flags & 0x3C != 0
    }
}

/// A pack: its header and the table of its files, over the pack's bytes.
#[derive(Clone, Debug)]
pub struct Pak {
    bytes: Arc<[u8]>,
    /// The data key (the header's key too).
    blowfish: Blowfish,
    /// The pack version (3).
    pub version: u32,
    /// The MD5 the header starts with.
    pub header_md5: [u8; 16],
    /// Where the Gbx headers start.
    pub gbx_headers_start: u32,
    /// Where the file data starts ("header max size").
    pub data_start: u32,
    /// The Gbx headers' size.
    pub gbx_headers_size: u32,
    /// The Gbx headers' compressed size.
    pub gbx_headers_compressed_size: u32,
    /// The pack mask.
    pub pack_mask: u128,
    /// The header's flags.
    pub flags: u32,
    /// Every folder, in the order stored.
    pub folders: Vec<PakFolder>,
    /// Every file, in the order stored.
    pub files: Vec<PakFile>,
    /// Each file's path in lower case, to its index.
    by_path: HashMap<String, usize>,
}

impl Pak {
    /// Reads a pack's header, decrypting it with the pack's base key (from its [`PackList`] entry).
    pub fn parse(bytes: impl Into<Arc<[u8]>>, base_key: &[u8; 16]) -> Result<Pak> {
        let bytes: Arc<[u8]> = bytes.into();
        let mut r = Bytes { bytes: &bytes, at: 0 };
        if r.take(8, "the magic")? != b"NadeoPak" {
            return Err(bad(Space::Pack, 0, "not a pack: expected NadeoPak"));
        }
        let version = u32::from_le_bytes(r.array("version")?);
        if !VERSIONS.contains(&version) {
            return Err(bad(
                Space::Pack,
                8,
                format!("pack version {version}: expected one of {VERSIONS:?}"),
            ));
        }
        let hex: String = base_key.iter().map(|b| format!("{b:02X}")).collect();
        let blowfish = Blowfish::new(&md5(format!("{hex}NadeoPak").as_bytes()));
        let iv = u64::from_le_bytes(r.array("header IV")?);
        let start = r.at;
        let len = bytes.len().saturating_sub(start);
        let stream = FileStream::new(
            Some(blowfish.clone()),
            Arc::clone(&bytes),
            start,
            bytes.len(),
            iv,
            len,
            false,
        )?;
        let mut st = State::new(NodeTable::new(0, Vec::new()));
        st.stream = Some(stream);
        let mut h = Archive::stream(len, &mut st);
        let header_md5 = h.array()?;
        let gbx_headers_start = h.u32()?;
        let data_start = h.u32()?;
        let gbx_headers_size = h.u32()?;
        let gbx_headers_compressed_size = h.u32()?;
        let pack_mask = u128::from_le_bytes(h.array()?);
        let flags = h.u32()?;
        let n = h.count(8)?;
        let mut folders = Vec::with_capacity(n);
        for i in 0..n {
            let at = h.here();
            let parent = h.i32()?;
            let parent =
                match parent {
                    -1 => None,
                    p => Some(usize::try_from(p).ok().filter(|&p| p < n).ok_or_else(|| {
                        bad(Space::Pack, at, format!("folder {i}: parent {p} of {n} folders"))
                    })?),
                };
            let mut name = h.string()?;
            if !name.ends_with('\\') && !name.ends_with('/') {
                name.push('\\');
            }
            folders.push(PakFolder { parent, name });
        }
        if let Some(third) = folders.get(2)
            && third.name.chars().count() > 4
        {
            let utf16: Vec<u8> = third.name.encode_utf16().flat_map(u16::to_le_bytes).collect();
            if let Some(&[a, b, c, d]) = utf16.get(4..8) {
                h.dummy_write([a, b, c, d]);
            }
        }
        let n = h.count(40)?;
        let mut files = Vec::with_capacity(n);
        for i in 0..n {
            let at = h.here();
            let folder = match h.i32()? {
                -1 => None,
                f => Some(
                    usize::try_from(f)
                        .ok()
                        .filter(|&f| f < folders.len())
                        .ok_or_else(|| {
                            bad(
                                Space::Pack,
                                at,
                                format!("file {i}: folder {f} of {}", folders.len()),
                            )
                        })?,
                ),
            };
            files.push(PakFile {
                folder,
                name: h.string()?,
                u01: h.u32()?,
                size: h.u32()?,
                compressed_size: h.u32()?,
                offset: h.u32()?,
                class_id: h.u32()?,
                flags: h.u64()?,
            });
        }
        let mut pak = Pak {
            bytes: Arc::clone(&bytes),
            blowfish,
            version,
            header_md5,
            gbx_headers_start,
            data_start,
            gbx_headers_size,
            gbx_headers_compressed_size,
            pack_mask,
            flags,
            folders,
            files,
            by_path: HashMap::new(),
        };
        pak.by_path = (0..pak.files.len())
            .filter_map(|i| Some((pak.path(pak.files.get(i)?).to_lowercase(), i)))
            .collect();
        Ok(pak)
    }

    /// A folder's path from the top, each name followed by `\`.
    pub fn folder_path(&self, folder: Option<usize>) -> String {
        let mut names = Vec::new();
        let mut at = folder;
        // a folder's parents cannot outnumber the folders (a cycle would)
        while let Some(f) = at.and_then(|i| self.folders.get(i))
            && names.len() < self.folders.len()
        {
            names.push(f.name.as_str());
            at = f.parent;
        }
        names.iter().rev().map(|n| n.replace('/', "\\")).collect()
    }

    /// A file's path as stored: its folder's path and its name (real or hashed).
    pub fn path(&self, file: &PakFile) -> String {
        self.folder_path(file.folder) + &file.name
    }

    /// The file at `path` (`\`-separated, case-insensitive): stored under that path, or under its folder's path and
    /// the [`name_hash`] of its name; failing both, the name is taken to include the last folders of the path, one
    /// more at a time (`a\b\c.Gbx` is looked for as `a\b\` and `c.Gbx`, then as `a\` and `b\c.Gbx`).
    pub fn find(&self, path: &str) -> Option<&PakFile> {
        let path = path.replace('/', "\\");
        let get = |p: &str| {
            self.by_path
                .get(&p.to_lowercase())
                .and_then(|&i| self.files.get(i))
        };
        if let Some(f) = get(&path) {
            return Some(f);
        }
        let mut split = path.len();
        while let Some(cut) = path.get(..split).and_then(|p| p.rfind('\\')) {
            let (folder, name) = path.split_at(cut.saturating_add(1));
            if let Some(f) = get(&(folder.to_string() + &name_hash(name))) {
                return Some(f);
            }
            split = cut;
        }
        get(&name_hash(&path))
    }

    /// A stream of a file's plaintext, decrypted as far as it is read ([`FileStream`]).
    pub fn open(&self, file: &PakFile) -> Result<FileStream> {
        let start = usize::try_from(self.data_start)
            .unwrap_or(usize::MAX)
            .saturating_add(usize::try_from(file.offset).unwrap_or(usize::MAX));
        let size = usize::try_from(file.size).unwrap_or(usize::MAX);
        let (blowfish, iv, start) = if file.is_encrypted() {
            let iv: [u8; 8] = self
                .bytes
                .get(start..start.saturating_add(8))
                .and_then(|b| b.try_into().ok())
                .ok_or_else(|| bad(Space::Pack, start, "a file's IV past the end of the pack"))?;
            (
                Some(self.blowfish.clone()),
                u64::from_le_bytes(iv),
                start.saturating_add(8),
            )
        } else {
            (None, 0, start)
        };
        FileStream::new(
            blowfish,
            Arc::clone(&self.bytes),
            start,
            self.bytes.len(),
            iv,
            size,
            file.is_compressed(),
        )
    }

    /// A Gbx file's reference table (the files its body names), read from its head alone: no node is read, so it
    /// needs none of the file's classes. `None` for a file that is not a Gbx file.
    pub fn ref_table(&self, file: &PakFile) -> Result<Option<RefTable>> {
        let mut stream = self.open(file)?;
        let size = stream.size();
        if file.class_id == 0 || size < 3 || stream.read(0, 3)? != b"GBX" {
            return Ok(None);
        }
        let mut st = State::new(NodeTable::new(0, Vec::new()));
        st.stream = Some(stream);
        let mut r = Archive::stream(size, &mut st);
        Ok(Some(head(&mut r)?.ref_table))
    }

    /// A Gbx file's header chunks (what the game lists before reading the file: a collector's desc, a map's
    /// description), read from its head alone like [`Pak::ref_table`]. `None` for a file that is not a Gbx file.
    pub fn header(&self, file: &PakFile) -> Result<Option<Header>> {
        let mut stream = self.open(file)?;
        let size = stream.size();
        if file.class_id == 0 || size < 3 || stream.read(0, 3)? != b"GBX" {
            return Ok(None);
        }
        let mut st = State::new(NodeTable::new(0, Vec::new()));
        st.stream = Some(stream);
        let mut r = Archive::stream(size, &mut st);
        let head = head(&mut r)?;
        let end = r.here();
        let mut stream = st
            .stream
            .take()
            .ok_or_else(|| bad(Space::File, size, "the pack stream was lost"))?;
        Ok(Some(Header {
            class_id: head.class_id,
            bytes: stream.read(0, end)?.to_vec(),
            chunks: head.header_chunks,
        }))
    }

    /// A file's plaintext. A Gbx file is decrypted as its nodes are read (see the module documentation), so this
    /// refuses one that holds a node of a class this crate does not read, or a chunk it cannot skip.
    pub fn read(&self, file: &PakFile) -> Result<Vec<u8>> {
        let mut stream = self.open(file)?;
        let size = stream.size();
        let gbx =
            file.class_id != 0 && !file.dont_use_dummy_write() && size >= 3 && stream.read(0, 3)? == b"GBX";
        if !gbx {
            return stream.finish();
        }
        let mut st = State::new(NodeTable::new(0, Vec::new()));
        st.stream = Some(stream);
        let mut r = Archive::stream(size, &mut st);
        let head = head(&mut r)?;
        if head.body_compression == Compression::Compressed {
            return Err(r.bad("a pack file with a compressed body: only uncompressed ones are read"));
        }
        r.st.nodes = node_table(head.nodes, &head.ref_table)?;
        body_nodes(&mut r, head.class_id)?;
        let stream = st
            .stream
            .take()
            .ok_or_else(|| bad(Space::File, size, "the pack stream was lost"))?;
        stream.finish()
    }
}

/// A pack file's head as [`Pak::header`] reads it: the container up to its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    /// The main node's class id, as stored.
    pub class_id: u32,
    /// The head's plaintext.
    bytes: Vec<u8>,
    /// Each header chunk's id, heavy bit, and data's offset and length.
    chunks: Vec<(u32, bool, usize, usize)>,
}

impl Header {
    /// The header chunks, in order.
    pub fn header_chunks(&self) -> Vec<HeaderChunk<'_>> {
        self.chunks
            .iter()
            .map(|&(id, heavy, offset, len)| HeaderChunk {
                id,
                heavy,
                offset,
                bytes: self
                    .bytes
                    .get(offset..offset.saturating_add(len))
                    .unwrap_or_default(),
            })
            .collect()
    }
}
