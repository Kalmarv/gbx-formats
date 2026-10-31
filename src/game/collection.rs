//! A collection (`CGameCtnCollection`, here Stadium) and its zones (`CGameCtnZoneFlat`, `CGameCtnZoneFrontier`): the
//! terrain a map is built on.
//!
//! A pack file's header chunks (the collection's description and folders) are kept as stored by the container.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Id, Ident};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};

/// A node reference.
type Node = Option<NodeIndex>;

/// A deprecated node list: the version the game wrote it with (always 10), the count and the references.
fn deprecated_list(r: &mut Archive<'_, '_>) -> Result<Vec<Node>> {
    r.version("deprecated node list", &[10])?;
    r.list(4, |r| r.node_ref())
}

/// The chunks of `CGameCtnZone` (class `0x0305C000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Zone {
    /// Chunk `0x0305C003`: the height, the zone's id, the surface's id and an `i32` GBX.NET does not name.
    pub zone: Option<(i32, Id, Id, i32)>,
    /// Chunk `0x0305C004` (skippable): the depth and whether the zone is old.
    pub depth: Option<(i32, bool)>,
    /// Chunk `0x0305C005`: whether the zone has water.
    pub has_water: Option<bool>,
}

impl Zone {
    fn chunk(&mut self, r: &mut Archive<'_, '_>, id: u32) -> Result<bool> {
        match id {
            0x0305_C003 => self.zone = Some((r.i32()?, r.id()?, r.id()?, r.i32()?)),
            0x0305_C004 => self.depth = Some(r.skippable(|r| Ok((r.i32()?, r.bool()?)))?),
            0x0305_C005 => self.has_water = Some(r.bool()?),
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// A flat zone (`CGameCtnZoneFlat`, class `0x0305D000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ZoneFlat {
    /// The zone chunks.
    pub zone: Zone,
    /// Chunk `0x0305D001`: the block infos of the flat terrain (`CGameCtnBlockInfoFlat`), the clip, the road and the
    /// pylon (`CGameCtnBlockInfoPylon`).
    pub block_infos: Option<[Node; 4]>,
    /// Chunk `0x0305D002` (skippable): an `i32` GBX.NET does not name, whether the zone is for the ground only, and a
    /// bool GBX.NET does not name.
    pub ground_only: Option<(i32, bool, bool)>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl ZoneFlat {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<ZoneFlat> {
        let mut n = ZoneFlat::default();
        n.chunks = r.chunks("CGameCtnZoneFlat", |r, id| {
            if n.zone.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x0305_D001 => {
                    n.block_infos = Some([r.node_ref()?, r.node_ref()?, r.node_ref()?, r.node_ref()?]);
                }
                0x0305_D002 => n.ground_only = Some(r.skippable(|r| Ok((r.i32()?, r.bool()?, r.bool()?)))?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A frontier zone (`CGameCtnZoneFrontier`, class `0x0305E000`): the border between two zones.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ZoneFrontier {
    /// The zone chunks.
    pub zone: Zone,
    /// Chunk `0x0305E001`: the frontier's block info (`CGameCtnBlockInfoFrontier`) and the ids of the parent and the
    /// child zones.
    pub frontier: Option<(Node, Id, Id)>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl ZoneFrontier {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<ZoneFrontier> {
        let mut n = ZoneFrontier::default();
        n.chunks = r.chunks("CGameCtnZoneFrontier", |r, id| {
            if n.zone.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x0305_E001 => n.frontier = Some((r.node_ref()?, r.id()?, r.id()?)),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A collection (`CGameCtnCollection`, class `0x03033000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Collection {
    /// Chunk `0x03033009`: the zones and the collection's size.
    pub zones: Option<CollectionZones>,
    /// Chunk `0x0303300C`: whether the collection is editable, and its type (`ECollectionType`).
    pub editable: Option<(bool, i32)>,
    /// Chunk `0x0303300D`: the menu icon and the small icon (`CPlugBitmap`), when stored.
    pub icons: Option<(Option<Node>, Option<Node>)>,
    /// Chunk `0x0303300E`: the sort index.
    pub sort_index: Option<i32>,
    /// Chunk `0x03033011`: the default decoration (`CGameCtnDecoration`).
    pub default_decoration: Option<Node>,
    /// Chunk `0x03033019`: the load screen (`CPlugBitmap`).
    pub load_screen: Option<Node>,
    /// Chunk `0x0303301A`: the map picture (`CPlugBitmap`), its rectangle, the coordinates of the element, the icon and
    /// the description on it, and the long description.
    pub map: Option<MapPicture>,
    /// Chunk `0x0303301D`: the zone ids that are replaced, and the ids that replace them.
    pub zone_strings: Option<Vec<(Id, Id)>>,
    /// Chunk `0x0303301D`: the terrain modifiers that replace zones (`CGameCtnDecorationTerrainModifier`).
    pub replacement_terrain_modifiers: Option<Vec<Node>>,
    /// Chunk `0x0303301E`: the height of the water's top and bottom, the least height of the camera, and whether water
    /// is outside the play field.
    pub camera: Option<(f32, f32, f32, bool)>,
    /// Chunk `0x0303301F`: the particle emitter models (`CPlugParticleEmitterModel`).
    pub particle_emitter_models: Option<Vec<Node>>,
    /// Chunk `0x03033020`: the folders of the block infos, the items, the decorations and the menu icons.
    pub folders: Option<[String; 4]>,
    /// Chunk `0x03033021`: the name shown.
    pub display_name: Option<String>,
    /// Chunk `0x03033022`: whether the water has several heights.
    pub is_water_multi_height: Option<bool>,
    /// Chunk `0x03033023`: whether the car can get dirty.
    pub car_can_be_dirty: Option<bool>,
    /// Chunk `0x03033024`: the lighting.
    pub lighting: Option<Lighting>,
    /// The node's chunks.
    pub chunks: Chunks,
}

/// Chunk `0x0303301A` of a collection.
#[derive(Clone, Debug, PartialEq)]
pub struct MapPicture {
    /// The picture (`CPlugBitmap`).
    pub bitmap: Node,
    /// The rectangle of the map in the picture: the minimum then the maximum corner, as stored.
    pub rect: [f32; 4],
    /// Where the collection's element sits on the map.
    pub coord_elem: [f32; 2],
    /// Where the collection's icon sits on the map.
    pub coord_icon: [f32; 2],
    /// Where the collection's description sits on the map.
    pub coord_desc: [f32; 2],
    /// The long description.
    pub long_desc: String,
}

/// Chunk `0x03033024` of a collection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lighting {
    /// The number of blocks that cast shadows.
    pub blocks_shadow: i32,
    /// The background shadow (`EBackgroundShadow`: 0 none, 1 receive, 2 cast and receive).
    pub background_shadow: i32,
    /// Whether shadows are cast backwards.
    pub shadow_cast_back: bool,
    /// How soft the shadows are, in world units.
    pub shadow_soft_size_in_world: f32,
    /// The vertex lighting (`EVertexLighting`: 0 none, 1 sunrise, 2 nations).
    pub vertex_lighting: i32,
    /// The least vertex color.
    pub color_vertex_min: f32,
    /// The greatest vertex color.
    pub color_vertex_max: f32,
}

/// Chunk `0x03033009` of a collection.
#[derive(Clone, Debug, PartialEq)]
pub struct CollectionZones {
    /// The collection's id.
    pub collection: Id,
    /// Every zone (`CGameCtnZone`).
    pub complete_zone_list: Vec<Node>,
    /// The default zone.
    pub default_zone: Node,
    /// Whether the collection needs unlocking.
    pub need_unlock: bool,
    /// The size of a square of the terrain.
    pub square_size: f32,
    /// The height of a square of the terrain.
    pub square_height: f32,
    /// The collection's vehicle.
    pub vehicle: Ident,
}

impl Collection {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Collection> {
        let mut n = Collection::default();
        n.chunks = r.chunks("CGameCtnCollection", |r, id| {
            match id {
                0x0303_3009 => {
                    n.zones = Some(CollectionZones {
                        collection: r.id()?,
                        complete_zone_list: deprecated_list(r)?,
                        default_zone: r.node_ref()?,
                        need_unlock: r.bool()?,
                        square_size: r.f32()?,
                        square_height: r.f32()?,
                        vehicle: r.ident()?,
                    });
                }
                0x0303_300C => n.editable = Some((r.bool()?, r.i32()?)),
                0x0303_300D => {
                    let icon = if r.bool()? { Some(r.node_ref()?) } else { None };
                    let small = if r.bool()? { Some(r.node_ref()?) } else { None };
                    n.icons = Some((icon, small));
                }
                0x0303_300E => n.sort_index = Some(r.i32()?),
                0x0303_3011 => n.default_decoration = Some(r.node_ref()?),
                0x0303_3019 => n.load_screen = Some(r.node_ref()?),
                0x0303_301A => {
                    n.map = Some(MapPicture {
                        bitmap: r.node_ref()?,
                        rect: r.vec4()?,
                        coord_elem: r.vec2()?,
                        coord_icon: r.vec2()?,
                        coord_desc: r.vec2()?,
                        long_desc: r.string()?,
                    });
                }
                0x0303_301D => {
                    n.zone_strings = Some(r.list(8, |r| Ok((r.id()?, r.id()?)))?);
                    n.replacement_terrain_modifiers = Some(deprecated_list(r)?);
                }
                0x0303_301E => n.camera = Some((r.f32()?, r.f32()?, r.f32()?, r.bool()?)),
                0x0303_301F => n.particle_emitter_models = Some(r.list(4, |r| r.node_ref())?),
                0x0303_3020 => n.folders = Some([r.string()?, r.string()?, r.string()?, r.string()?]),
                0x0303_3021 => n.display_name = Some(r.string()?),
                0x0303_3022 => n.is_water_multi_height = Some(r.bool()?),
                0x0303_3023 => n.car_can_be_dirty = Some(r.bool()?),
                0x0303_3024 => {
                    n.lighting = Some(Lighting {
                        blocks_shadow: r.i32()?,
                        background_shadow: r.i32()?,
                        shadow_cast_back: r.bool()?,
                        shadow_soft_size_in_world: r.f32()?,
                        vertex_lighting: r.i32()?,
                        color_vertex_min: r.f32()?,
                        color_vertex_max: r.f32()?,
                    });
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
