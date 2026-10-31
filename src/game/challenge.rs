//! A map: `CGameCtnChallenge` (class `0x03043000`), its header chunks and the body chunks TrackMania Nations Forever
//! writes.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Ident, PackDesc, State};
use crate::error::{Result, Space};
use crate::game::block::Block;
use crate::game::collector::Medals;
use crate::game::nullable;
use crate::game::unlimiter::{EncryptedUnlimiter, Unlimiter};
use crate::gbx::{Gbx, HeaderChunk};
use crate::node::{Chunks, NodeIndex, NodeTable};

/// Header chunk `0x03043002`: the map's description (version 11 as TrackMania Forever writes it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Description {
    /// The chunk version.
    pub version: u8,
    /// Whether the map must be unlocked.
    pub need_unlock: bool,
    /// The medal times.
    pub medals: Medals,
    /// The map's cost (coppers).
    pub cost: u32,
    /// Whether it is a lap race.
    pub lap_race: bool,
    /// The play mode (0 race, 1 platform, 2 puzzle, 3 crazy, 4 shortcut, 5 stunts).
    pub mode: u32,
    /// A `u32` bool whose meaning is not known ("has clones" in GBX.NET's naming).
    pub has_clones: bool,
    /// The author's score, `None` for -1.
    pub author_score: Option<u32>,
    /// The editor it was made in (0 advanced, 1 simple).
    pub editor: u32,
}

/// Header chunk `0x03043003`: the map's name and identity (version 5 as TrackMania Forever writes it).
#[derive(Clone, Debug, PartialEq)]
pub struct Common {
    /// The chunk version.
    pub version: u8,
    /// The map's uid, environment and author login.
    pub map_info: Ident,
    /// The map's name (with its formatting codes).
    pub name: String,
    /// The map kind byte (6 in-play, 0 end marker, ...).
    pub kind: u8,
    /// A `u32` whose meaning is not known.
    pub u01: u32,
    /// The password, empty when none.
    pub password: String,
    /// The decoration (mood): its id, collection and author.
    pub decoration: Ident,
    /// The map's coordinate origin on its environment's map.
    pub coord_origin: [f32; 2],
    /// The map's coordinate target.
    pub coord_target: [f32; 2],
    /// The pack mask (16 bytes).
    pub pack_mask: [u8; 16],
}

/// Header chunk `0x03043007`: the thumbnail and the author's comments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thumbnail {
    /// The chunk version.
    pub version: u32,
    /// The JPEG bytes, between the markers `<Thumbnail.jpg>` and `</Thumbnail.jpg>`.
    pub jpeg: Vec<u8>,
    /// The comments, between `<Comments>` and `</Comments>`.
    pub comments: String,
}

/// A map's header chunks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Header {
    /// `0x03043002`.
    pub description: Option<Description>,
    /// `0x03043003`.
    pub common: Option<Common>,
    /// `0x03043004`: the version of the map format.
    pub version: Option<u32>,
    /// `0x03043005`: the map's description as XML.
    pub xml: Option<String>,
    /// `0x03043007`.
    pub thumbnail: Option<Thumbnail>,
    /// The header chunks of other ids, as stored.
    pub unread: Vec<(u32, Vec<u8>)>,
}

fn marker(r: &mut Archive<'_, '_>, text: &str) -> Result<()> {
    let at = r.here();
    if r.bytes(text.len())? != text.as_bytes() {
        return Err(r.bad_at(at, format!("expected the marker {text}")));
    }
    Ok(())
}

fn description(r: &mut Archive<'_, '_>) -> Result<Description> {
    let version = r.version8("map description", &[11])?;
    Ok(Description {
        version,
        need_unlock: r.bool()?,
        medals: Medals::read(r)?,
        cost: r.u32()?,
        lap_race: r.bool()?,
        mode: r.u32()?,
        has_clones: r.bool()?,
        author_score: nullable(r)?,
        editor: r.u32()?,
    })
}

fn common(r: &mut Archive<'_, '_>) -> Result<Common> {
    let version = r.version8("map common header", &[5])?;
    Ok(Common {
        version,
        map_info: r.ident()?,
        name: r.string()?,
        kind: r.u8()?,
        u01: r.u32()?,
        password: r.string()?,
        decoration: r.ident()?,
        coord_origin: r.vec2()?,
        coord_target: r.vec2()?,
        pack_mask: r.array()?,
    })
}

fn thumbnail(r: &mut Archive<'_, '_>) -> Result<Thumbnail> {
    let version = r.version("map thumbnail", &[1])?;
    let size = r.count(1)?;
    marker(r, "<Thumbnail.jpg>")?;
    let jpeg = r.bytes(size)?.to_vec();
    marker(r, "</Thumbnail.jpg>")?;
    marker(r, "<Comments>")?;
    let comments = r.string()?;
    marker(r, "</Comments>")?;
    Ok(Thumbnail {
        version,
        jpeg,
        comments,
    })
}

impl Header {
    /// Reads the header chunks of a map's container. Each is a scope of its own for ids.
    pub fn read(gbx: &Gbx<'_>) -> Result<Header> {
        let mut h = Header::default();
        for &HeaderChunk {
            id, offset, bytes, ..
        } in &gbx.header_chunks
        {
            let mut st = State::new(NodeTable::new(0, Vec::new()));
            let mut r = Archive::new(bytes, offset, Space::File, &mut st);
            match id {
                0x0304_3002 => h.description = Some(description(&mut r)?),
                0x0304_3003 => h.common = Some(common(&mut r)?),
                0x0304_3004 => h.version = Some(r.u32()?),
                0x0304_3005 => h.xml = Some(r.string()?),
                0x0304_3007 => h.thumbnail = Some(thumbnail(&mut r)?),
                _ => {
                    h.unread.push((id, bytes.to_vec()));
                    continue;
                }
            }
            r.end(&format!("header chunk {id:#010x}"))?;
        }
        Ok(h)
    }
}

/// A map's block data (chunk `0x0304301F`, or the older `0x03043013`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockData {
    /// The map's uid, environment and author login.
    pub map_info: Ident,
    /// The map's name.
    pub name: String,
    /// The decoration.
    pub decoration: Ident,
    /// The map's size in blocks.
    pub size: [u32; 3],
    /// Whether the map must be unlocked (in `0x03043013`, a bool GBX.NET leaves unnamed).
    pub need_unlock: bool,
    /// The block data version (chunk `0x0304301F` stores it; `0x03043013` is version 0).
    pub version: u32,
    /// The blocks.
    pub blocks: Vec<Block>,
}

/// Chunk `0x03043011`: the block stock, the parameters and the map kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parameters {
    /// The `CGameCtnCollectorList` node.
    pub block_stock: Option<NodeIndex>,
    /// The `CGameCtnChallengeParameters` node.
    pub challenge_parameters: Option<NodeIndex>,
    /// The map kind (6 in-play, ...).
    pub kind: u32,
}

/// Chunk `0x03043021`: the mediatracker clips of the older layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clips {
    /// The intro clip (`CGameCtnMediaClip`).
    pub intro: Option<NodeIndex>,
    /// The in-game clip group (`CGameCtnMediaClipGroup`).
    pub in_game: Option<NodeIndex>,
    /// The end-of-race clip group (`CGameCtnMediaClipGroup`).
    pub end_race: Option<NodeIndex>,
}

impl Clips {
    /// Resolves the intro reference to a clip; absent, external or differently typed references return `None`.
    pub fn intro<'n>(&self, nodes: &'n crate::Nodes) -> Option<&'n super::media::MediaClip> {
        nodes.get(self.intro?)?.try_into().ok()
    }
    /// Resolves the in-game reference to its clip group.
    pub fn in_game<'n>(&self, nodes: &'n crate::Nodes) -> Option<&'n super::media::MediaClipGroup> {
        nodes.get(self.in_game?)?.try_into().ok()
    }
    /// Resolves the end-of-race reference to its clip group.
    pub fn end_race<'n>(&self, nodes: &'n crate::Nodes) -> Option<&'n super::media::MediaClipGroup> {
        nodes.get(self.end_race?)?.try_into().ok()
    }
}

/// A custom camera for the thumbnail.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomCamera {
    /// A byte whose meaning is not known.
    pub u01: u8,
    /// The rotation (3x3, row by row).
    pub rotation: [f32; 9],
    /// The position.
    pub position: [f32; 3],
    /// The field of view.
    pub fov: f32,
    /// The near clip plane.
    pub near: f32,
    /// The far clip plane.
    pub far: f32,
}

/// Chunk `0x03043028` (`0x03043027` and comments): the camera the thumbnail was taken with, if a custom one.
#[derive(Clone, Debug, PartialEq)]
pub struct ThumbnailCamera {
    /// The camera, when the map has a custom one.
    pub camera: Option<CustomCamera>,
    /// The author's comments.
    pub comments: String,
}

/// A map (`CGameCtnChallenge`): the body chunks TrackMania Nations Forever writes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Challenge {
    /// `0x0304300D`: the vehicle.
    pub player_model: Option<Ident>,
    /// `0x03043011`.
    pub parameters: Option<Parameters>,
    /// `0x03043013`: block data in the older layout (version 0).
    pub legacy_block_data: Option<BlockData>,
    /// `0x03043017` (skippable): the checkpoints' block coordinates.
    pub checkpoints: Option<Vec<[i32; 3]>>,
    /// `0x03043018` (skippable): whether it is a lap race, and the number of laps.
    pub laps: Option<(bool, u32)>,
    /// `0x03043019` (skippable): the mod pack.
    pub mod_pack: Option<PackDesc>,
    /// `0x0304301C` (skippable): the play mode.
    pub play_mode: Option<u32>,
    /// `0x0304301F`: the block data.
    pub block_data: Option<BlockData>,
    /// `0x03043021`.
    pub clips: Option<Clips>,
    /// `0x03043022`: a `u32` whose meaning is not known.
    pub u022: Option<u32>,
    /// `0x03043024`: the custom music.
    pub music: Option<PackDesc>,
    /// `0x03043025`: the map's coordinate origin and target.
    pub coords: Option<([f32; 2], [f32; 2])>,
    /// `0x03043026`: the global clip (`CGameCtnMediaClip`).
    pub clip_global: Option<Option<NodeIndex>>,
    /// `0x03043028`.
    pub thumbnail_camera: Option<ThumbnailCamera>,
    /// `0x03043029` (skippable): the hashed password (16 bytes) and a CRC-32.
    pub password: Option<([u8; 16], u32)>,
    /// `0x0304302A`: whether the map was made in the simple editor.
    pub created_with_simple_editor: Option<bool>,
    /// `0x3F001001` to `0x3F001003` (skippable): TMUnlimiter's data.
    pub unlimiter: Option<Unlimiter>,
    /// `0x3F001000` (skippable, encrypted): TMUnlimiter 1.x's data.
    pub unlimiter_encrypted: Option<EncryptedUnlimiter>,
    /// The node's chunks.
    pub chunks: Chunks,
}

fn size(r: &mut Archive<'_, '_>) -> Result<[u32; 3]> {
    Ok([r.u32()?, r.u32()?, r.u32()?])
}

fn block_data(r: &mut Archive<'_, '_>) -> Result<BlockData> {
    let map_info = r.ident()?;
    let name = r.string()?;
    let decoration = r.ident()?;
    let size = size(r)?;
    let need_unlock = r.bool()?;
    let version = r.version("block data", &[1, 6])?;
    let blocks = r.list(9, |r| Block::read(r, version))?;
    Ok(BlockData {
        map_info,
        name,
        decoration,
        size,
        need_unlock,
        version,
        blocks,
    })
}

fn legacy_block_data(r: &mut Archive<'_, '_>) -> Result<BlockData> {
    let map_info = r.ident()?;
    let name = r.string()?;
    let decoration = r.ident()?;
    let size = size(r)?;
    let need_unlock = r.bool()?;
    let blocks = r.list(7, |r| Block::read(r, 0))?;
    Ok(BlockData {
        map_info,
        name,
        decoration,
        size,
        need_unlock,
        version: 0,
        blocks,
    })
}

fn thumbnail_camera(r: &mut Archive<'_, '_>) -> Result<ThumbnailCamera> {
    let camera = if r.bool()? {
        let u01 = r.u8()?;
        let mut rotation = [0.0; 9];
        for v in &mut rotation {
            *v = r.f32()?;
        }
        Some(CustomCamera {
            u01,
            rotation,
            position: r.vec3()?,
            fov: r.f32()?,
            near: r.f32()?,
            far: r.f32()?,
        })
    } else {
        None
    };
    Ok(ThumbnailCamera {
        camera,
        comments: r.string()?,
    })
}

impl Challenge {
    /// Resolves the global clip; absent, external or differently typed references return `None`.
    pub fn clip_global<'n>(&self, nodes: &'n crate::Nodes) -> Option<&'n super::media::MediaClip> {
        nodes.get(self.clip_global.flatten()?)?.try_into().ok()
    }
}

impl Challenge {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Challenge> {
        let mut n = Challenge::default();
        n.chunks = r.chunks("CGameCtnChallenge", |r, id| {
            match id {
                0x0304_300D => n.player_model = Some(r.ident()?),
                0x0304_3011 => {
                    n.parameters = Some(Parameters {
                        block_stock: r.node_ref()?,
                        challenge_parameters: r.node_ref()?,
                        kind: r.u32()?,
                    })
                }
                0x0304_3013 => n.legacy_block_data = Some(legacy_block_data(r)?),
                0x0304_3017 => n.checkpoints = Some(r.skippable(|r| r.list(12, |r| r.int3()))?),
                0x0304_3018 => n.laps = Some(r.skippable(|r| Ok((r.bool()?, r.u32()?)))?),
                0x0304_3019 => n.mod_pack = Some(r.skippable(|r| r.pack_desc())?),
                0x0304_301C => n.play_mode = Some(r.skippable(|r| r.u32())?),
                0x0304_301F => n.block_data = Some(block_data(r)?),
                0x0304_3021 => {
                    n.clips = Some(Clips {
                        intro: r.node_ref()?,
                        in_game: r.node_ref()?,
                        end_race: r.node_ref()?,
                    })
                }
                0x0304_3022 => n.u022 = Some(r.u32()?),
                0x0304_3024 => n.music = Some(r.pack_desc()?),
                0x0304_3025 => n.coords = Some((r.vec2()?, r.vec2()?)),
                0x0304_3026 => n.clip_global = Some(r.node_ref()?),
                0x0304_3028 => n.thumbnail_camera = Some(thumbnail_camera(r)?),
                0x0304_3029 => n.password = Some(r.skippable(|r| Ok((r.array()?, r.u32()?)))?),
                0x0304_302A => n.created_with_simple_editor = Some(r.bool()?),
                0x3F00_1000 => n.unlimiter_encrypted = Some(r.skippable(EncryptedUnlimiter::read)?),
                0x3F00_1001..=0x3F00_1003 => {
                    let data_version = id.saturating_sub(0x3F00_1001);
                    n.unlimiter = Some(r.skippable(|r| Unlimiter::read(r, id, data_version))?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        if let Some(e) = &n.unlimiter_encrypted {
            let blocks = n
                .block_data
                .as_ref()
                .or(n.legacy_block_data.as_ref())
                .map_or(0, |d| d.blocks.len());
            if let Some(b) = e
                .blocks
                .iter()
                .find(|b| usize::try_from(b.index).map_or(true, |i| i >= blocks))
            {
                return Err(r.bad(format!(
                    "TMUnlimiter block {}: the map has {blocks} blocks",
                    b.index
                )));
            }
        }
        Ok(n)
    }
}
