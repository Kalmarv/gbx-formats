//! Stadium's block infos: the collector and block info chunks every block info shares, and the block info classes
//! (`CGameCtnBlockInfoClassic`, `Road`, `Slope`, `Clip`, `Flat`, `Frontier`, `RectAsym`, `Pylon`) with their block units
//! (`CGameCtnBlockUnitInfo`).
//!
//! A block info node stores the chunks of `CGameCtnCollector` (`0x0301A…`) and `CGameCtnBlockInfo` (`0x0304E…`) ahead of
//! its own class's; each class here carries a [`Collector`] and a [`BlockInfo`] for them.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Id, Ident};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};
use crate::plug::{Iso4, iso4};

/// A node reference list entry.
type Node = Option<NodeIndex>;

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

/// What a block info says about the checkpoint, start or finish it is (`EWayPointType`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WayPoint {
    /// The type: 0 start, 1 finish, 2 checkpoint, 3 none, 4 start and finish, 5 dispenser.
    pub kind: i32,
    /// The mobil shown for the ground variant.
    pub ground_helper: Node,
    /// The mobil shown for the air variant.
    pub air_helper: Node,
    /// The mobil shown in construction mode, when stored (chunk `0x0304E00B` and later).
    pub construction_helper: Option<Node>,
}

/// The chunks of `CGameCtnBlockInfo` (class `0x0304E000`, id prefix `0x0304E`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockInfo {
    /// Chunk `0x0304E009`: whether the block is a pillar.
    pub is_pillar: Option<bool>,
    /// Chunks `0x0304E00A` and `0x0304E00E`: the way point.
    pub waypoint: Option<WayPoint>,
    /// Chunk `0x0304E00C`: where a car spawns on the block: ground and air.
    pub spawn: Option<(Iso4, Iso4)>,
    /// Chunk `0x0304E00D`: whether the block replaces another.
    pub is_replacement: Option<bool>,
    /// Chunk `0x0304E00F`: whether a car is not respawned on the block.
    pub no_respawn: Option<bool>,
    /// Chunk `0x0304E005`: the name, the units and the mobils.
    pub body: Option<Body>,
}

/// The mobils of a block's variant, one list per unit.
pub type UnitMobils = Vec<Vec<Node>>;

/// Chunk `0x0304E005` of a block info: its name, units and mobils.
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    /// The block info's name.
    pub name: Id,
    /// The block's size in units: stored, and discarded by the game since Sunrise.
    pub size: [i32; 3],
    /// Whether the block is a pillar.
    pub is_pillar: bool,
    /// The selection (`ESelection`): 0 random, 1 weighted, 2 auto-rotate.
    pub selection: i32,
    /// An `i32` GBX.NET does not name.
    pub u01: i32,
    /// The pillar's block info (`CGameCtnBlockInfoClassic`).
    pub pillar: Node,
    /// The units of the ground variant (`CGameCtnBlockUnitInfo`).
    pub ground_units: Vec<Node>,
    /// The units of the air variant.
    pub air_units: Vec<Node>,
    /// The mobils (`CSceneMobil`) of the ground variant, one list per unit.
    pub ground_mobils: UnitMobils,
    /// The mobils of the air variant.
    pub air_mobils: UnitMobils,
    /// A byte GBX.NET does not name.
    pub u02: u8,
    /// An `i32` GBX.NET does not name.
    pub u03: i32,
    /// Two `i16`s GBX.NET does not name.
    pub u04: [i16; 2],
}

impl BlockInfo {
    /// Reads chunk `id` if it is one of the block info's.
    pub(crate) fn chunk(&mut self, r: &mut Archive<'_, '_>, id: u32) -> Result<bool> {
        match id {
            0x0304_E009 => self.is_pillar = Some(r.bool()?),
            0x0304_E00A => {
                self.waypoint = Some(WayPoint {
                    kind: r.i32()?,
                    ground_helper: r.node_ref()?,
                    air_helper: r.node_ref()?,
                    construction_helper: None,
                });
            }
            0x0304_E00C => self.spawn = Some((iso4(r)?, iso4(r)?)),
            0x0304_E00D => self.is_replacement = Some(r.bool()?),
            0x0304_E00E => {
                self.waypoint = Some(WayPoint {
                    kind: r.i32()?,
                    ground_helper: r.node_ref()?,
                    air_helper: r.node_ref()?,
                    construction_helper: Some(r.node_ref()?),
                });
            }
            0x0304_E00F => self.no_respawn = Some(r.bool()?),
            0x0304_E005 => {
                self.body = Some(Body {
                    name: r.id()?,
                    size: r.int3()?,
                    is_pillar: r.bool()?,
                    selection: r.i32()?,
                    u01: r.i32()?,
                    pillar: r.node_ref()?,
                    ground_units: r.list(4, |r| r.node_ref())?,
                    air_units: r.list(4, |r| r.node_ref())?,
                    ground_mobils: r.list(4, |r| r.list(4, |r| r.node_ref()))?,
                    air_mobils: r.list(4, |r| r.list(4, |r| r.node_ref()))?,
                    u02: r.u8()?,
                    u03: r.i32()?,
                    u04: [r.i16()?, r.i16()?],
                });
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// Reads the chunks a block info class shares and then `own`'s.
fn read_info(
    r: &mut Archive<'_, '_>,
    class: &'static str,
    collector: &mut Collector,
    info: &mut BlockInfo,
    mut own: impl FnMut(&mut Archive<'_, '_>, u32) -> Result<bool>,
) -> Result<Chunks> {
    r.chunks(class, |r, id| {
        Ok(collector.chunk(r, id)? || info.chunk(r, id)? || own(r, id)?)
    })
}

macro_rules! plain_block_info {
    ($(#[$doc:meta])* $name:ident, $class:literal) => {
        $(#[$doc])*
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct $name {
            /// The collector chunks.
            pub collector: Collector,
            /// The block info chunks.
            pub info: BlockInfo,
            /// The node's chunks.
            pub chunks: Chunks,
        }

        impl $name {
            pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<$name> {
                let mut n = $name::default();
                n.chunks = read_info(r, $class, &mut n.collector, &mut n.info, |_, _| Ok(false))?;
                Ok(n)
            }
        }
    };
}

plain_block_info!(
    /// A block (`CGameCtnBlockInfoClassic`, class `0x03051000`).
    BlockInfoClassic,
    "CGameCtnBlockInfoClassic"
);
plain_block_info!(
    /// A block info for flat terrain (`CGameCtnBlockInfoFlat`, class `0x0304F000`).
    BlockInfoFlat,
    "CGameCtnBlockInfoFlat"
);
plain_block_info!(
    /// A block info for a rectangular asymmetric block (`CGameCtnBlockInfoRectAsym`, class `0x03056000`).
    BlockInfoRectAsym,
    "CGameCtnBlockInfoRectAsym"
);

/// A block unit (`CGameCtnBlockUnitInfo`, class `0x03036000`): one cell of a block.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockUnitInfo {
    /// Chunk `0x03036000`: how many pylons the unit places, two bools GBX.NET does not name, the unit's offset in the
    /// block and its clips (also called junctions).
    pub place: Option<UnitPlace>,
    /// Chunk `0x03036001`: the surface id, the frontier (`EFrontier`) and the direction.
    pub surface: Option<(Id, i32, i32)>,
    /// Chunk `0x03036002`: whether the unit is underground.
    pub underground: Option<bool>,
    /// Chunk `0x03036003`: the block info that replaces this one, its id, the replacement direction (`ReplacementDir`)
    /// and an `i32` GBX.NET does not name.
    pub replacement: Option<(Node, Id, i32, i32)>,
    /// Chunk `0x03036004`: how many pylons the unit accepts.
    pub accept_pylons: Option<i32>,
    /// Chunk `0x03036005`: the terrain modifier's id.
    pub terrain_modifier: Option<Id>,
    /// Chunk `0x03036008`: the bottom and the top clip.
    pub bottom_top_clips: Option<(Node, Node)>,
    /// The node's chunks.
    pub chunks: Chunks,
}

/// Chunk `0x03036000` of a block unit.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitPlace {
    /// The number of pylons placed.
    pub place_pylons: i32,
    /// A bool GBX.NET does not name (it guesses "accept pylons").
    pub u1: bool,
    /// A bool GBX.NET does not name.
    pub u2: bool,
    /// The unit's offset from the block's origin, in units.
    pub relative_offset: [i32; 3],
    /// The clips (`CGameCtnBlockInfoClip`).
    pub clips: Vec<Node>,
}

impl BlockUnitInfo {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<BlockUnitInfo> {
        let mut n = BlockUnitInfo::default();
        n.chunks = r.chunks("CGameCtnBlockUnitInfo", |r, id| {
            match id {
                0x0303_6000 => {
                    n.place = Some(UnitPlace {
                        place_pylons: r.i32()?,
                        u1: r.bool()?,
                        u2: r.bool()?,
                        relative_offset: r.int3()?,
                        clips: r.list(4, |r| r.node_ref())?,
                    });
                }
                0x0303_6001 => n.surface = Some((r.id()?, r.i32()?, r.i32()?)),
                0x0303_6002 => n.underground = Some(r.skippable(|r| r.bool())?),
                0x0303_6003 => n.replacement = Some((r.node_ref()?, r.id()?, r.i32()?, r.i32()?)),
                0x0303_6004 => n.accept_pylons = Some(r.i32()?),
                0x0303_6005 => n.terrain_modifier = Some(r.id()?),
                0x0303_6008 => n.bottom_top_clips = Some((r.node_ref()?, r.node_ref()?)),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A road block info (`CGameCtnBlockInfoRoad`, class `0x03052000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockInfoRoad {
    /// The collector chunks.
    pub collector: Collector,
    /// The block info chunks.
    pub info: BlockInfo,
    /// Chunk `0x03052000`: the slope (`CGameCtnBlockInfoSlope`).
    pub slope: Option<Node>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl BlockInfoRoad {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<BlockInfoRoad> {
        let mut n = BlockInfoRoad::default();
        n.chunks = read_info(
            r,
            "CGameCtnBlockInfoRoad",
            &mut n.collector,
            &mut n.info,
            |r, id| {
                if id != 0x0305_2000 {
                    return Ok(false);
                }
                n.slope = Some(r.node_ref()?);
                Ok(true)
            },
        )?;
        Ok(n)
    }
}

/// A clip block info (`CGameCtnBlockInfoClip`, class `0x03053000`): the shape two blocks agree on to connect.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockInfoClip {
    /// The collector chunks.
    pub collector: Collector,
    /// The block info chunks.
    pub info: BlockInfo,
    /// Chunk `0x03053000` (skippable) or `0x03053001`: the legacy symmetrical clip (`CGameCtnCollector`).
    pub legacy_symmetrical_clip: Option<Node>,
    /// Chunk `0x03053002`: the symmetrical clip's id.
    pub symmetrical_clip_id: Option<Id>,
    /// Chunk `0x03053003`: whether the clip is a free clip (legacy flag).
    pub is_free_clip: Option<bool>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl BlockInfoClip {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<BlockInfoClip> {
        let mut n = BlockInfoClip::default();
        n.chunks = read_info(
            r,
            "CGameCtnBlockInfoClip",
            &mut n.collector,
            &mut n.info,
            |r, id| {
                match id {
                    0x0305_3000 => n.legacy_symmetrical_clip = Some(r.skippable(|r| r.node_ref())?),
                    0x0305_3001 => n.legacy_symmetrical_clip = Some(r.node_ref()?),
                    0x0305_3002 => n.symmetrical_clip_id = Some(r.id()?),
                    0x0305_3003 => n.is_free_clip = Some(r.bool()?),
                    _ => return Ok(false),
                }
                Ok(true)
            },
        )?;
        Ok(n)
    }
}

/// A frontier block info (`CGameCtnBlockInfoFrontier`, class `0x03050000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockInfoFrontier {
    /// The collector chunks.
    pub collector: Collector,
    /// The block info chunks.
    pub info: BlockInfo,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl BlockInfoFrontier {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<BlockInfoFrontier> {
        let mut n = BlockInfoFrontier::default();
        n.chunks = read_info(
            r,
            "CGameCtnBlockInfoFrontier",
            &mut n.collector,
            &mut n.info,
            |_, _| Ok(false),
        )?;
        Ok(n)
    }
}

/// A pylon block info (`CGameCtnBlockInfoPylon`, class `0x03055000`): the support under a raised block.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockInfoPylon {
    /// The collector chunks.
    pub collector: Collector,
    /// The block info chunks.
    pub info: BlockInfo,
    /// Chunk `0x03055000`: three `CMwNod` references.
    pub nodes: Option<[Node; 3]>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl BlockInfoPylon {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<BlockInfoPylon> {
        let mut n = BlockInfoPylon::default();
        n.chunks = read_info(
            r,
            "CGameCtnBlockInfoPylon",
            &mut n.collector,
            &mut n.info,
            |r, id| {
                if id != 0x0305_5000 {
                    return Ok(false);
                }
                n.nodes = Some([r.node_ref()?, r.node_ref()?, r.node_ref()?]);
                Ok(true)
            },
        )?;
        Ok(n)
    }
}
