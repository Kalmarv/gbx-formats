//! A map's block stock (`CGameCtnCollectorList`), parameters (`CGameCtnChallengeParameters`), and the chunks every
//! collectable (a block info, a decoration) shares (`CGameCtnCollector`).
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Id, Ident, State};
use crate::error::{Result, Space};
use crate::game::nullable;
use crate::gbx::HeaderChunk;
use crate::node::{Chunks, NodeIndex, NodeTable};

/// A node reference.
type Node = Option<NodeIndex>;

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

/// What a collector stores about its catalog entry (chunk `0x0301A007`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Catalog {
    /// Whether the entry is internal.
    pub is_internal: bool,
    /// Whether the entry needs unlocking.
    pub need_unlock: bool,
    /// Its position in the catalog.
    pub position: i32,
    /// The least number of the entry available.
    pub nb_available_min: i32,
    /// Its price in copper.
    pub copper_price: i32,
    /// The most of the entry available.
    pub nb_available_max: i32,
}

/// The chunks of `CGameCtnCollector` (class `0x0301A000`, id prefix `0x0301A`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Collector {
    /// Chunk `0x0301A006`: the skin kind (`ESkinKind`: 0 none, 1 text, 2 image, 3 vehicle).
    pub skin_kind: Option<i32>,
    /// Chunk `0x0301A007`: the catalog entry.
    pub catalog: Option<Catalog>,
    /// Chunk `0x0301A008`: the game skin (`CPlugGameSkin`).
    pub game_skin: Option<Node>,
    /// Chunk `0x0301A009`: the page name, the icon file reference (when stored), and the parent collector's id.
    pub page: Option<(String, Option<Node>, Id)>,
    /// Chunk `0x0301A00A`: an id GBX.NET does not name.
    pub u0a: Option<Id>,
    /// Chunk `0x0301A00B`: the collector's ident.
    pub ident: Option<Ident>,
}

impl Collector {
    /// Reads chunk `id` if it is one of the collector's.
    pub(crate) fn chunk(&mut self, r: &mut Archive<'_, '_>, id: u32) -> Result<bool> {
        match id {
            0x0301_A006 => self.skin_kind = Some(r.i32()?),
            0x0301_A007 => {
                self.catalog = Some(Catalog {
                    is_internal: r.bool()?,
                    need_unlock: r.bool()?,
                    position: r.i32()?,
                    nb_available_min: r.i32()?,
                    copper_price: r.i32()?,
                    nb_available_max: r.i32()?,
                });
            }
            0x0301_A008 => self.game_skin = Some(r.node_ref()?),
            0x0301_A009 => {
                let name = r.string()?;
                let icon = if r.bool()? { Some(r.node_ref()?) } else { None };
                self.page = Some((name, icon, r.id()?));
            }
            0x0301_A00A => self.u0a = Some(r.id()?),
            0x0301_A00B => self.ident = Some(r.ident()?),
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// Header chunk `0x0301A003` of a collector's file (a block info, a decoration, a collection item): what the game lists
/// before it reads the file, its ident first. GBX.NET's `SHeaderDesc`, versions 1 to 8; a field is `Some` in the
/// versions that store it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CollectorDesc {
    /// The collector's ident: its name, collection and author (the name a map's blocks use).
    pub ident: Ident,
    /// The chunk's version.
    pub version: u32,
    /// The editor page it is listed on.
    pub page_name: String,
    /// Version 5: an id GBX.NET does not name.
    pub u05: Option<Id>,
    /// Version 4 and later: the parent collector's id.
    pub parent: Option<Id>,
    /// Versions 1 and 2: a bool GBX.NET does not name.
    pub u01: Option<bool>,
    /// Version 2: whether the collector is internal.
    pub is_internal: Option<bool>,
    /// Version 3 and later: its flags (`ECollectorFlags`).
    pub flags: Option<i32>,
    /// Version 2 and later: its position in the catalog.
    pub catalog_position: Option<i16>,
    /// Versions 2 to 5: the least available, the price in copper and the most available.
    pub available: Option<(u8, i32, i16)>,
    /// Version 7 and later: its display name.
    pub name: Option<String>,
    /// Version 8 and later: its production state (`EProdState`).
    pub prod_state: Option<u8>,
}

impl CollectorDesc {
    /// The desc among a collector's file's header chunks ([`crate::Gbx::header_chunks`], [`crate::pak::Header`]),
    /// `None` when they hold none. Its chunk is a scope of its own for ids.
    pub fn read(header_chunks: &[HeaderChunk<'_>]) -> Result<Option<CollectorDesc>> {
        let Some(&HeaderChunk { offset, bytes, .. }) = header_chunks.iter().find(|c| c.id == 0x0301_A003)
        else {
            return Ok(None);
        };
        let mut st = State::new(NodeTable::new(0, Vec::new()));
        let mut r = Archive::new(bytes, offset, Space::File, &mut st);
        let ident = r.ident()?;
        let version = r.u32()?;
        if !(1..=8).contains(&version) {
            return Err(r.bad(format!("collector desc version {version}, which is not read")));
        }
        let mut d = CollectorDesc {
            ident,
            version,
            page_name: r.string()?,
            ..CollectorDesc::default()
        };
        if version == 5 {
            d.u05 = Some(r.id()?);
        }
        if version >= 4 {
            d.parent = Some(r.id()?);
        }
        if version <= 2 {
            d.u01 = Some(r.bool()?);
        }
        if version == 2 {
            d.is_internal = Some(r.bool()?);
        }
        if version >= 3 {
            d.flags = Some(r.i32()?);
        }
        if version >= 2 {
            d.catalog_position = Some(r.i16()?);
        }
        if (2..=5).contains(&version) {
            d.available = Some((r.u8()?, r.i32()?, r.i16()?));
        }
        if version >= 7 {
            d.name = Some(r.string()?);
        }
        if version >= 8 {
            d.prod_state = Some(r.u8()?);
        }
        r.end("header chunk 0x0301a003")?;
        Ok(Some(d))
    }
}
