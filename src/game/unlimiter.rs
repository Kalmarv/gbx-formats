//! TMUnlimiter's data in a map: chunks `0x3F001001` to `0x3F001003` (one layout, selected by a version byte) and
//! the encrypted `0x3F001000`.
//!
//! TMUnlimiter 2.x writes a map's real blocks in these chunks and leaves a single stand-in block in the map's block
//! data (a block named "TMUnlimiter 2.0.0 is required to load this challenge..."). Version 7 also embeds block
//! models: [`EmbeddedBlockData`] holds trees ([`crate::plug`]) whose materials and textures are files the map
//! names ([`Unlimiter::material_model_refs`], [`Unlimiter::embedded_images`]); the indices of those files are
//! registered in the node table as nodes the body never reads, so references to them resolve to no node
//! ([`crate::Nodes::get`] returns `None`).
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use super::media::FuncKeysReal;
use super::parameters::{self, ValueKind};
use crate::archive::{Archive, Id, Ident};
use crate::error::Result;
use crate::node::NodeIndex;

/// How much of the sky and decoration is drawn: 0 everything, 1 the sky only, 2 the background, 3 nothing.
pub type Visibility = u8;

/// The decoration's visibility, offset and scale.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Decoration {
    /// 0 everything, 1 sky only, 2 background, 3 nothing.
    pub visibility: Visibility,
    /// The offset, stored unless the visibility is 3 (nothing) and the flag is set.
    pub offset: Option<[f32; 3]>,
    /// The scale, stored under the same conditions.
    pub scale: Option<[f32; 3]>,
}

/// A legacy script (version 6): a name and its bytecode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyScript {
    /// The script's name.
    pub name: String,
    /// Its bytecode, as stored.
    pub byte_code: Vec<u8>,
}

/// A value of a legacy (version 6) parameter.
#[derive(Clone, Debug, PartialEq)]
pub enum LegacyValue {
    /// A string, stored with a one-byte length.
    String(String),
    /// A float.
    Float(f32),
}

/// A legacy parameter: a function ([`parameters`]) and its value.
#[derive(Clone, Debug, PartialEq)]
pub struct LegacyParameter {
    /// The function's index in [`parameters::NAMES`] (any `i32` is stored; the legacy layout does not check it).
    pub function: i32,
    /// The value.
    pub value: LegacyValue,
}

/// A legacy parameter set (version 6).
#[derive(Clone, Debug, PartialEq)]
pub struct LegacyParameterSet {
    /// The set's name.
    pub name: String,
    /// Its parameters.
    pub parameters: Vec<LegacyParameter>,
}

/// What a mediatracker clip is mapped to (version 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipResource {
    /// Type 0: a legacy parameter set, by index.
    ParameterSet(i32),
    /// Type 1: a legacy script, by index.
    LegacyScript(i32),
}

/// A mediatracker clip mapped to a parameter set or script (version 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipMapping {
    /// The clip's index in the in-game clip group.
    pub clip_index: i32,
    /// What it runs.
    pub resource: ClipResource,
}

/// An AngelScript module (version 7). Only empty ones occur: GBX.NET refuses modules with bytecode, and so does
/// this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AngelScriptModule {
    /// The module's name; absent for a core module (flag bit 1).
    pub module_name: Option<String>,
}

/// How a parameter acts on its function (`ParameterOperation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// 0.
    Execute,
    /// 1.
    Set,
    /// 2.
    Multiply,
}

/// A parameter's value, by the kind of its function ([`parameters::kind`]).
#[derive(Clone, Debug, PartialEq)]
pub enum ParameterValue {
    /// A function that stores nothing beyond its operation.
    None,
    /// A float.
    Float(f32),
    /// A string.
    String(String),
    /// A float to multiply by.
    Multiplier(f32),
    /// A `CFuncKeysReal` stored in place (no node index or class id).
    Keys(Box<FuncKeysReal>),
    /// An unsigned integer.
    Natural(u32),
    /// A list of floats.
    Buffer(Vec<f32>),
    /// A bool.
    Bool(bool),
}

/// A parameter of a parameter set (version 7).
#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    /// The function's index in [`parameters::NAMES`].
    pub function: u32,
    /// The operation.
    pub operation: Operation,
    /// The value.
    pub value: ParameterValue,
}

/// A named list of parameters (version 7).
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterSet {
    /// The set's name.
    pub name: String,
    /// Its parameters.
    pub parameters: Vec<Parameter>,
}

/// A trigger group's condition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Condition {
    /// Target 0: a mediatracker condition of this type (0 none) with a value when the type is not 0.
    MediaTracker {
        /// The condition type.
        condition_type: u8,
        /// The value, stored unless the type is 0.
        value: Option<f32>,
    },
    /// Target 1: an AngelScript function.
    AngelScript {
        /// The module's index.
        module: u32,
        /// The function's index.
        function: u32,
    },
}

/// What a trigger group runs on an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Target 0: a parameter set, by index.
    ParameterSet(u32),
    /// Target 2: an AngelScript function.
    AngelScript {
        /// The module's index.
        module: u32,
        /// The function's index.
        function: u32,
    },
}

/// A trigger group (version 7).
#[derive(Clone, Debug, PartialEq)]
pub struct TriggerGroup {
    /// The group's name.
    pub name: String,
    /// Its condition (flag bit 0).
    pub condition: Option<Condition>,
    /// What runs on entering (flag bit 1).
    pub on_enter: Option<Event>,
    /// What runs while inside (flag bit 2).
    pub on_inside: Option<Event>,
    /// What runs on leaving (flag bit 3).
    pub on_leave: Option<Event>,
}

/// A material model file the embedded blocks use (version 7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialModelRef {
    /// The node index references to the file use.
    pub instance_index: i32,
    /// The file's path, relative to the map's folder.
    pub relative_path: String,
}

/// The node indices of the replacement textures (version 7): each a file the body never reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReplacementTextures {
    /// Flag bit 0.
    pub specular: Option<i32>,
    /// Flag bit 1.
    pub normal: Option<i32>,
    /// Flag bit 2.
    pub white: Option<i32>,
    /// Flag bit 3.
    pub black: Option<i32>,
}

/// A bitmap an embedded image is used as (version 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitmapPair {
    /// The node index references to the bitmap use.
    pub instance_index: i32,
    /// The texture filter.
    pub tex_filter: u8,
    /// The texture address mode.
    pub tex_address: u8,
}

/// An image file stored in the map (version 7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddedImage {
    /// The class id of the file.
    pub class_id: u32,
    /// The file's path, relative to the map's folder.
    pub relative_path: String,
    /// The file's bytes.
    pub data: Vec<u8>,
    /// The bitmaps that use it.
    pub bitmap_pairs: Vec<BitmapPair>,
}

/// One variation of an embedded block (version 7): a tree and, for a waypoint, a trigger tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubVariation {
    /// The tree (`CPlugTree`).
    pub tree: Option<NodeIndex>,
    /// The trigger tree, stored for the waypoint types Finish (1), Checkpoint (2) and StartFinish (4).
    pub trigger_tree: Option<Option<NodeIndex>>,
    /// A byte stored from data version 2 (GBX.NET: the pre-light-generation tile count).
    pub pre_light_gen_tile_count_u: Option<u8>,
}

/// The icon of an embedded block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Icon {
    /// The width in pixels.
    pub width: u8,
    /// The height in pixels.
    pub height: u8,
    /// `width * height * 4` bytes of pixels.
    pub pixels: Vec<u8>,
}

/// An embedded block model (`EmbeddedBlockData`, version 7): the model many blocks of the map can use.
#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddedBlockData {
    /// The model's name.
    pub id: Id,
    /// Its author.
    pub author: Id,
    /// A `u32` stored by data version 0 only.
    pub u01: Option<u32>,
    /// Waypoint type: 0 start, 1 finish, 2 checkpoint, 3 none, 4 start-finish, 5 dispenser.
    pub waypoint: u8,
    /// Block type: 2 classic, 3 road (the types that store variations); others occur without.
    pub block_type: u8,
    /// The icon (data version 1 and later, flag bit 6).
    pub icon: Option<Icon>,
    /// The variations: ground then air for each of the (up to) six variants; the first pair for types 2 and 3, the
    /// rest for type 3 only.
    pub variations: Vec<[Vec<SubVariation>; 2]>,
    /// The spawn offset on the ground.
    pub spawn_offset_ground: [f32; 3],
    /// The spawn rotation on the ground.
    pub spawn_rotation_ground: [f32; 3],
    /// The spawn offset in the air.
    pub spawn_offset_air: [f32; 3],
    /// The spawn rotation in the air.
    pub spawn_rotation_air: [f32; 3],
    /// The ground block units (data version 1 and later).
    pub ground_block_units: Option<Vec<[i32; 3]>>,
    /// The air block units (data version 1 and later).
    pub air_block_units: Option<Vec<[i32; 3]>>,
}

/// A game block as the chunk stores it.
#[derive(Clone, Debug, PartialEq)]
pub struct GameBlock {
    /// The block model's name.
    pub name: Id,
    /// The block model's collection.
    pub collection: Id,
    /// The coordinate (version 1 stores bytes; they are widened here).
    pub coord: [i32; 3],
    /// The direction.
    pub direction: u8,
    /// The flags (bit 15: an author and a skin follow).
    pub flags: i32,
    /// The author and the skin node (`CGameCtnBlockSkin`), when flag bit 15 is set.
    pub skin: Option<(Id, Option<NodeIndex>)>,
}

/// What kind of block an [`UnlimiterBlock`] is.
#[derive(Clone, Debug, PartialEq)]
pub enum BlockKind {
    /// Type 0: a block of the game.
    Game(GameBlock),
    /// Type 1 (version 7): a trigger volume.
    Trigger {
        /// The coordinate.
        coord: [i32; 3],
        /// The direction.
        direction: u8,
        /// The trigger groups it belongs to, by index into [`Unlimiter::trigger_groups`].
        trigger_groups: Vec<u32>,
    },
    /// Type 2: a block model in a file (a block pack) rather than in the game.
    External {
        /// The model's name.
        name: Id,
        /// Its author.
        author: Id,
        /// The coordinate.
        coord: [i32; 3],
        /// The direction.
        direction: u8,
        /// The flags.
        flags: i32,
    },
    /// Type 3 (version 7): a block using an embedded model.
    Embedded {
        /// The model, by index into [`Unlimiter::embedded_blocks`].
        data_index: u32,
        /// The coordinate.
        coord: [i32; 3],
        /// The direction.
        direction: u8,
        /// The flags.
        flags: i32,
    },
}

/// A block's motion.
#[derive(Clone, Debug, PartialEq)]
pub struct Motion {
    /// The wave type (the low three bits of the motion flags).
    pub wave_type: u8,
    /// The motion points; absent when the motion is controlled manually (flag bit 3).
    pub points: Option<Vec<MotionPoint>>,
}

/// A point of a motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionPoint {
    /// The time in milliseconds.
    pub time: i32,
    /// The offset.
    pub offset: [f32; 3],
    /// The rotation.
    pub rotation: [f32; 3],
}

/// The properties TMUnlimiter adds to a block. Version 6 stores `inverted`, `automatic_spawn` and
/// `classic_terrain`, and one block group; version 7 stores the rest.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockProperties {
    /// The block is inverted (version 6).
    pub inverted: bool,
    /// The offset.
    pub offset: Option<[f32; 3]>,
    /// The rotation.
    pub rotation: Option<[f32; 3]>,
    /// The scale.
    pub scale: Option<[f32; 3]>,
    /// The motion (version 7).
    pub motion: Option<Motion>,
    /// The origin offset (version 7).
    pub origin_offset: Option<[f32; 3]>,
    /// The block groups, by index into [`Unlimiter::block_groups`].
    pub block_groups: Vec<u32>,
    /// The block moves.
    pub dynamic: bool,
    /// The block is not drawn.
    pub invisible: bool,
    /// The block has no collisions.
    pub non_collidable: bool,
    /// How the block's spawn point is altered: 0 not, 1 manually (version 7, with the offset and rotation below),
    /// 2 automatically (version 6 stores only this bit).
    pub spawn_method: u8,
    /// The spawn offset, for the manual method.
    pub spawn_offset: Option<[f32; 3]>,
    /// The spawn rotation, for the manual method.
    pub spawn_rotation: Option<[f32; 3]>,
    /// 0 default, 1 forced respawnable, 2 forced non-respawnable (version 7).
    pub respawn: u8,
    /// 0 default, 1 disabled, 2 manually triggered (version 7).
    pub trigger_activation: u8,
    /// The block uses the classic terrain (version 6).
    pub classic_terrain: bool,
}

/// A block of the chunk: what it is and its properties.
#[derive(Clone, Debug, PartialEq)]
pub struct UnlimiterBlock {
    /// The block.
    pub kind: BlockKind,
    /// Its properties.
    pub properties: BlockProperties,
}

/// The data of chunks `0x3F001001` to `0x3F001003` (`TMUnlimiterData` in GBX.NET).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Unlimiter {
    /// The chunk it was read from: `0x3F001001`, `0x3F001002` or `0x3F001003` (they differ in the version of the
    /// embedded block data: 0, 1 or 2).
    pub chunk: u32,
    /// The version byte: 0 (vanilla: nothing follows), 1, 6 or 7.
    pub version: u8,
    /// The decoration (versions 6 and 7).
    pub decoration: Option<Decoration>,
    /// Pylons are disabled (versions 6 and 7).
    pub pylons_disabled: bool,
    /// The track base is empty (versions 6 and 7).
    pub track_base_empty: bool,
    /// Vehicles collide (version 7).
    pub vehicle_collisions: bool,
    /// The start line is chosen at random (version 7).
    pub random_start_line: bool,
    /// The map talks to a server (version 7).
    pub server_communication: bool,
    /// Motion blocks ignore the multiplayer time sync (version 7).
    pub ignore_multiplayer_time_sync: bool,
    /// The legacy scripts (version 6).
    pub legacy_scripts: Vec<LegacyScript>,
    /// The legacy parameter sets (version 6).
    pub legacy_parameter_sets: Vec<LegacyParameterSet>,
    /// The clip mappings (version 6).
    pub clip_mappings: Vec<ClipMapping>,
    /// The AngelScript modules (version 7).
    pub angel_script_modules: Vec<AngelScriptModule>,
    /// The parameter sets (version 7).
    pub parameter_sets: Vec<ParameterSet>,
    /// The trigger groups (version 7).
    pub trigger_groups: Vec<TriggerGroup>,
    /// The block groups' names (versions 6 and 7).
    pub block_groups: Vec<String>,
    /// The material model files (version 7); each index is registered as a node the body never reads.
    pub material_model_refs: Vec<MaterialModelRef>,
    /// The replacement textures (version 7); each index is registered likewise.
    pub replacement_textures: ReplacementTextures,
    /// The embedded images (version 7); each bitmap's index is registered likewise.
    pub embedded_images: Vec<EmbeddedImage>,
    /// The embedded block models (version 7).
    pub embedded_blocks: Vec<EmbeddedBlockData>,
    /// The vehicles the map offers (version 7).
    pub vehicle_identifiers: Vec<Ident>,
    /// The blocks (all versions but 0).
    pub blocks: Vec<UnlimiterBlock>,
}

fn vec3(r: &mut Archive<'_, '_>) -> Result<[f32; 3]> {
    r.vec3()
}

/// An `i32` that must not be negative.
fn index(r: &mut Archive<'_, '_>, what: &str) -> Result<u32> {
    let at = r.here();
    let v = r.i32()?;
    u32::try_from(v).map_err(|_| r.bad_at(at, format!("{what} {v}")))
}

fn decoration(
    r: &mut Archive<'_, '_>,
    visibility: Visibility,
    moved: bool,
    scaled: bool,
) -> Result<Decoration> {
    let mut d = Decoration {
        visibility,
        ..Decoration::default()
    };
    if visibility != 3 {
        if moved {
            d.offset = Some(vec3(r)?);
        }
        if scaled {
            d.scale = Some(vec3(r)?);
        }
    }
    Ok(d)
}

fn game_block(r: &mut Archive<'_, '_>, byte_coord: bool) -> Result<GameBlock> {
    let name = r.id()?;
    let collection = r.id()?;
    let coord = if byte_coord {
        r.byte3()?.map(i32::from)
    } else {
        r.int3()?
    };
    let direction = r.u8()?;
    let flags = r.i32()?;
    let skin = if flags & (1 << 15) != 0 {
        Some((r.id()?, r.node_ref()?))
    } else {
        None
    };
    Ok(GameBlock {
        name,
        collection,
        coord,
        direction,
        flags,
        skin,
    })
}

fn external_block(r: &mut Archive<'_, '_>) -> Result<BlockKind> {
    Ok(BlockKind::External {
        name: r.id()?,
        author: r.id()?,
        coord: r.int3()?,
        direction: r.u8()?,
        flags: r.i32()?,
    })
}

fn legacy_parameter(r: &mut Archive<'_, '_>) -> Result<LegacyParameter> {
    let function = r.i32()?;
    let is_string = u32::try_from(function).is_ok_and(parameters::legacy_is_string);
    let value = if is_string {
        LegacyValue::String(r.string8()?)
    } else {
        LegacyValue::Float(r.f32()?)
    };
    Ok(LegacyParameter { function, value })
}

fn block_group_names(r: &mut Archive<'_, '_>) -> Result<Vec<String>> {
    r.list(4, |r| r.string())
}

fn blocks_v6(r: &mut Archive<'_, '_>, groups: usize) -> Result<Vec<UnlimiterBlock>> {
    r.list(1, |r| {
        let kind = match r.u8()? {
            0 => BlockKind::Game(game_block(r, false)?),
            2 => external_block(r)?,
            t => return Err(r.bad(format!("block type {t}: expected 0 or 2"))),
        };
        let flags_at = r.here();
        let flags = r.u16()?;
        if flags & !0x3FF != 0 {
            return Err(r.bad_at(
                flags_at,
                format!("block flags {flags:#x}: bits above 9 are not read"),
            ));
        }
        let mut p = BlockProperties {
            inverted: flags & 1 != 0,
            ..BlockProperties::default()
        };
        if flags & (1 << 1) != 0 {
            p.offset = Some(vec3(r)?);
        }
        if flags & (1 << 2) != 0 {
            p.rotation = Some(vec3(r)?);
        }
        if flags & (1 << 3) != 0 {
            p.scale = Some(vec3(r)?);
        }
        p.dynamic = flags & (1 << 4) != 0;
        p.invisible = flags & (1 << 5) != 0;
        p.non_collidable = flags & (1 << 6) != 0;
        p.spawn_method = if flags & (1 << 7) != 0 { 2 } else { 0 };
        if flags & (1 << 8) != 0 {
            p.block_groups = vec![group(r, groups)?];
        }
        p.classic_terrain = flags & (1 << 9) != 0;
        Ok(UnlimiterBlock { kind, properties: p })
    })
}

/// A block group index that must name one of the `groups` groups.
fn group(r: &mut Archive<'_, '_>, groups: usize) -> Result<u32> {
    let at = r.here();
    let i = index(r, "block group")?;
    if usize::try_from(i).map_or(true, |i| i >= groups) {
        return Err(r.bad_at(at, format!("block group {i}: the map has {groups}")));
    }
    Ok(i)
}

/// An `i32` array (count, then values) of indices that must each be below `limit`.
fn index_array(r: &mut Archive<'_, '_>, what: &str, limit: usize) -> Result<Vec<u32>> {
    let at = r.here();
    let v = r.list(4, |r| index(r, what))?;
    if let Some(bad) = v
        .iter()
        .find(|&&i| usize::try_from(i).map_or(true, |i| i >= limit))
    {
        return Err(r.bad_at(at, format!("{what} {bad}: there are {limit}")));
    }
    Ok(v)
}

/// The positional part of a block's flags (bits 0 to 3): offset, rotation, scale, motion.
fn positional(r: &mut Archive<'_, '_>, flags: u16, p: &mut BlockProperties) -> Result<()> {
    if flags & 1 != 0 {
        p.offset = Some(vec3(r)?);
    }
    if flags & (1 << 1) != 0 {
        p.rotation = Some(vec3(r)?);
    }
    if flags & (1 << 2) != 0 {
        p.scale = Some(vec3(r)?);
    }
    if flags & (1 << 3) != 0 {
        let at = r.here();
        let flags3 = r.u8()?;
        if flags3 & 0xF0 != 0 {
            return Err(r.bad_at(at, format!("motion flags {flags3:#x}: bits above 3 are not read")));
        }
        let points = if flags3 & (1 << 3) == 0 {
            Some(r.list(28, |r| {
                Ok(MotionPoint {
                    time: r.i32()?,
                    offset: vec3(r)?,
                    rotation: vec3(r)?,
                })
            })?)
        } else {
            None
        };
        p.motion = Some(Motion {
            wave_type: flags3 & 7,
            points,
        });
    }
    Ok(())
}

struct Counts {
    trigger_groups: usize,
    block_groups: usize,
    embedded: usize,
}

fn blocks_v7(r: &mut Archive<'_, '_>, c: &Counts) -> Result<Vec<UnlimiterBlock>> {
    r.list(1, |r| {
        let kind = match r.u8()? {
            0 => BlockKind::Game(game_block(r, false)?),
            1 => BlockKind::Trigger {
                coord: r.int3()?,
                direction: r.u8()?,
                trigger_groups: index_array(r, "trigger group", c.trigger_groups)?,
            },
            2 => external_block(r)?,
            3 => {
                let data_index = index(r, "embedded block data")?;
                if usize::try_from(data_index).map_or(true, |i| i >= c.embedded) {
                    return Err(r.bad(format!(
                        "embedded block data {data_index}: the map has {}",
                        c.embedded
                    )));
                }
                BlockKind::Embedded {
                    data_index,
                    coord: r.int3()?,
                    direction: r.u8()?,
                    flags: r.i32()?,
                }
            }
            t => return Err(r.bad(format!("block type {t}: expected 0 to 3"))),
        };
        let mut p = BlockProperties::default();
        if matches!(kind, BlockKind::Trigger { .. }) {
            let at = r.here();
            let flags = r.u8()?;
            if flags & 0xC0 != 0 {
                return Err(r.bad_at(
                    at,
                    format!("trigger block flags {flags:#x}: bits 6 and 7 are not read"),
                ));
            }
            positional(r, u16::from(flags), &mut p)?;
            if flags & (1 << 4) != 0 {
                p.block_groups = index_array(r, "block group", c.block_groups)?;
            }
            p.non_collidable = flags & (1 << 5) != 0;
        } else {
            let at = r.here();
            let flags = r.u16()?;
            if flags & (1 << 15) != 0 {
                return Err(r.bad_at(at, format!("block flags {flags:#x}: bit 15 is not read")));
            }
            positional(r, flags, &mut p)?;
            if flags & (1 << 4) != 0 {
                p.origin_offset = Some(vec3(r)?);
            }
            if flags & (1 << 5) != 0 {
                p.block_groups = index_array(r, "block group", c.block_groups)?;
            }
            p.dynamic = flags & (1 << 6) != 0;
            p.invisible = flags & (1 << 7) != 0;
            p.non_collidable = flags & (1 << 8) != 0;
            p.spawn_method = u8::try_from((flags >> 9) & 3).unwrap_or(3);
            if p.spawn_method == 3 {
                return Err(r.bad_at(at, format!("block flags {flags:#x}: spawn method 3")));
            }
            if p.spawn_method == 1 {
                p.spawn_offset = Some(vec3(r)?);
                p.spawn_rotation = Some(vec3(r)?);
            }
            p.respawn = u8::try_from((flags >> 11) & 3).unwrap_or(3);
            p.trigger_activation = u8::try_from((flags >> 13) & 3).unwrap_or(3);
            if p.respawn == 3 || p.trigger_activation == 3 {
                return Err(r.bad_at(
                    at,
                    format!("block flags {flags:#x}: respawn capability or trigger activation 3"),
                ));
            }
        }
        Ok(UnlimiterBlock { kind, properties: p })
    })
}

fn operation(r: &mut Archive<'_, '_>) -> Result<Operation> {
    let at = r.here();
    match r.u8()? {
        0 => Ok(Operation::Execute),
        1 => Ok(Operation::Set),
        2 => Ok(Operation::Multiply),
        o => Err(r.bad_at(at, format!("parameter operation {o}: expected 0 to 2"))),
    }
}

fn parameter(r: &mut Archive<'_, '_>) -> Result<Parameter> {
    let at = r.here();
    let function = r.u32()?;
    let Some(kind) = parameters::kind(function) else {
        return Err(r.bad_at(at, format!("parameter function {function}: not in the enum")));
    };
    let operation = operation(r)?;
    let multiply = operation == Operation::Multiply;
    let value = match kind {
        ValueKind::None => ParameterValue::None,
        ValueKind::Float => ParameterValue::Float(r.f32()?),
        ValueKind::String => ParameterValue::String(r.string()?),
        ValueKind::KeysReal if multiply => ParameterValue::Multiplier(r.f32()?),
        ValueKind::KeysReal => ParameterValue::Keys(Box::new(FuncKeysReal::read(r)?)),
        ValueKind::Natural if multiply => ParameterValue::Multiplier(r.f32()?),
        ValueKind::Natural => ParameterValue::Natural(r.u32()?),
        ValueKind::FastBufferReal if multiply => ParameterValue::Multiplier(r.f32()?),
        ValueKind::FastBufferReal => ParameterValue::Buffer(r.list(4, |r| r.f32())?),
        ValueKind::Bool => ParameterValue::Bool(r.bool8()?),
    };
    Ok(Parameter {
        function,
        operation,
        value,
    })
}

fn trigger_group(r: &mut Archive<'_, '_>) -> Result<TriggerGroup> {
    let name = r.string()?;
    let at = r.here();
    let flags = r.u8()?;
    if flags & 0xF0 != 0 {
        return Err(r.bad_at(
            at,
            format!("trigger group flags {flags:#x}: bits above 3 are not read"),
        ));
    }
    let condition = if flags & 1 != 0 {
        let at = r.here();
        Some(match r.u8()? {
            0 => {
                let condition_type = r.u8()?;
                Condition::MediaTracker {
                    condition_type,
                    value: if condition_type != 0 { Some(r.f32()?) } else { None },
                }
            }
            1 => Condition::AngelScript {
                module: r.u32()?,
                function: r.u32()?,
            },
            t => return Err(r.bad_at(at, format!("trigger condition target {t}: expected 0 or 1"))),
        })
    } else {
        None
    };
    let event = |r: &mut Archive<'_, '_>, bit: u8| -> Result<Option<Event>> {
        if flags & bit == 0 {
            return Ok(None);
        }
        let at = r.here();
        Ok(Some(match r.u8()? {
            0 => Event::ParameterSet(r.u32()?),
            2 => Event::AngelScript {
                module: r.u32()?,
                function: r.u32()?,
            },
            t => return Err(r.bad_at(at, format!("trigger event target {t}: expected 0 or 2"))),
        }))
    };
    Ok(TriggerGroup {
        name,
        condition,
        on_enter: event(r, 2)?,
        on_inside: event(r, 4)?,
        on_leave: event(r, 8)?,
    })
}

/// Registers a node index the body never reads.
fn register(r: &mut Archive<'_, '_>, at: usize, index: i32, what: &str) -> Result<()> {
    let ok = u32::try_from(index).is_ok_and(|i| r.st.nodes.add_external(i));
    if ok {
        Ok(())
    } else {
        Err(r.bad_at(
            at,
            format!("{what} node index {index}: not a free index below the header's node count"),
        ))
    }
}

fn replacement_textures(r: &mut Archive<'_, '_>) -> Result<ReplacementTextures> {
    let at = r.here();
    let flags = r.u8()?;
    if flags & 0xF0 != 0 {
        return Err(r.bad_at(
            at,
            format!("replacement texture flags {flags:#x}: bits above 3 are not read"),
        ));
    }
    let mut t = ReplacementTextures::default();
    for (bit, slot, what) in [
        (1, &mut t.specular, "specular texture"),
        (2, &mut t.normal, "normal texture"),
        (4, &mut t.white, "white texture"),
        (8, &mut t.black, "black texture"),
    ] {
        if flags & bit != 0 {
            let at = r.here();
            let i = r.i32()?;
            register(r, at, i, what)?;
            *slot = Some(i);
        }
    }
    Ok(t)
}

fn embedded_image(r: &mut Archive<'_, '_>) -> Result<EmbeddedImage> {
    let class_id = r.u32()?;
    let relative_path = r.string()?;
    let size = r.count(1)?;
    let data = r.bytes(size)?.to_vec();
    let bitmap_pairs = r.list(6, |r| {
        let at = r.here();
        let instance_index = r.i32()?;
        register(r, at, instance_index, "bitmap")?;
        Ok(BitmapPair {
            instance_index,
            tex_filter: r.u8()?,
            tex_address: r.u8()?,
        })
    })?;
    Ok(EmbeddedImage {
        class_id,
        relative_path,
        data,
        bitmap_pairs,
    })
}

/// A list with a one-byte count.
fn byte_list<T>(
    r: &mut Archive<'_, '_>,
    mut f: impl FnMut(&mut Archive<'_, '_>) -> Result<T>,
) -> Result<Vec<T>> {
    let n = usize::from(r.u8()?);
    (0..n).map(|_| f(r)).collect()
}

fn sub_variations(r: &mut Archive<'_, '_>, waypoint: u8, data_version: u32) -> Result<Vec<SubVariation>> {
    byte_list(r, |r| {
        let tree = r.node_ref()?;
        let trigger_tree = if matches!(waypoint, 1 | 2 | 4) {
            Some(r.node_ref()?)
        } else {
            None
        };
        let pre_light_gen_tile_count_u = if data_version >= 2 { Some(r.u8()?) } else { None };
        Ok(SubVariation {
            tree,
            trigger_tree,
            pre_light_gen_tile_count_u,
        })
    })
}

fn embedded_block_data(r: &mut Archive<'_, '_>, data_version: u32) -> Result<EmbeddedBlockData> {
    let id = r.id()?;
    let author = r.id()?;
    let (mut u01, mut icon) = (None, None);
    let (waypoint, block_type);
    if data_version == 0 {
        u01 = Some(r.u32()?);
        let at = r.here();
        waypoint = r.u8()?;
        if waypoint > 5 {
            return Err(r.bad_at(at, format!("waypoint type {waypoint}: expected 0 to 5")));
        }
        block_type = r.u8()?;
    } else {
        let flags = r.u8()?;
        block_type = flags & 7;
        waypoint = (flags >> 3) & 7;
        if waypoint > 5 {
            return Err(r.bad(format!("waypoint type {waypoint}: expected 0 to 5")));
        }
        if flags & (1 << 6) != 0 {
            let width = r.u8()?;
            let height = r.u8()?;
            let n = usize::from(width)
                .saturating_mul(usize::from(height))
                .saturating_mul(4);
            icon = Some(Icon {
                width,
                height,
                pixels: r.bytes(n)?.to_vec(),
            });
        }
        if flags & (1 << 7) != 0 {
            return Err(r.bad("embedded block flags: bit 7 is not read"));
        }
    }
    let mut variations = Vec::new();
    let pairs = match block_type {
        2 => 1,
        3 => 6,
        _ => 0,
    };
    for _ in 0..pairs {
        variations.push([
            sub_variations(r, waypoint, data_version)?,
            sub_variations(r, waypoint, data_version)?,
        ]);
    }
    let spawn_offset_ground = vec3(r)?;
    let spawn_rotation_ground = vec3(r)?;
    let spawn_offset_air = vec3(r)?;
    let spawn_rotation_air = vec3(r)?;
    let (mut ground_block_units, mut air_block_units) = (None, None);
    if data_version >= 1 {
        let at = r.here();
        let ground = r.u32()?;
        let air = r.u32()?;
        let total = u64::from(ground)
            .saturating_add(u64::from(air))
            .saturating_mul(12);
        if usize::try_from(total).map_or(true, |t| t > r.remaining()) {
            return Err(r.bad_at(
                at,
                format!("{ground} and {air} block units do not fit in the chunk"),
            ));
        }
        let ground = usize::try_from(ground).unwrap_or(0);
        let air = usize::try_from(air).unwrap_or(0);
        ground_block_units = Some((0..ground).map(|_| r.int3()).collect::<Result<Vec<_>>>()?);
        air_block_units = Some((0..air).map(|_| r.int3()).collect::<Result<Vec<_>>>()?);
    }
    Ok(EmbeddedBlockData {
        id,
        author,
        u01,
        waypoint,
        block_type,
        icon,
        variations,
        spawn_offset_ground,
        spawn_rotation_ground,
        spawn_offset_air,
        spawn_rotation_air,
        ground_block_units,
        air_block_units,
    })
}

impl Unlimiter {
    /// Reads chunk `chunk` (`0x3F001001` to `0x3F001003`) of a map; `data_version` is the version of its embedded
    /// block data (0, 1 or 2 by the chunk's id).
    pub(crate) fn read(r: &mut Archive<'_, '_>, chunk: u32, data_version: u32) -> Result<Unlimiter> {
        let version = r.version8("TMUnlimiter data", &[0, 1, 6, 7])?;
        let mut n = Unlimiter {
            chunk,
            version,
            ..Unlimiter::default()
        };
        match version {
            0 => {}
            1 => {
                n.blocks = r.list(25, |r| {
                    Ok(UnlimiterBlock {
                        kind: BlockKind::Game(game_block(r, true)?),
                        properties: BlockProperties::default(),
                    })
                })?;
            }
            6 => n.read_v6(r)?,
            _ => n.read_v7(r, data_version)?,
        }
        Ok(n)
    }

    fn read_v6(&mut self, r: &mut Archive<'_, '_>) -> Result<()> {
        let at = r.here();
        let flags = r.u8()?;
        if flags & 0xC0 != 0 {
            return Err(r.bad_at(at, format!("flags {flags:#x}: bits 6 and 7 are not read")));
        }
        let visibility = (flags >> 2) & 3;
        self.decoration = Some(decoration(r, visibility, flags & 1 != 0, flags & 2 != 0)?);
        self.pylons_disabled = flags & (1 << 4) != 0;
        self.track_base_empty = flags & (1 << 5) != 0;
        self.legacy_scripts = r.list(8, |r| {
            Ok(LegacyScript {
                name: r.string()?,
                byte_code: r.data()?.to_vec(),
            })
        })?;
        self.legacy_parameter_sets = r.list(8, |r| {
            Ok(LegacyParameterSet {
                name: r.string()?,
                parameters: r.list(5, legacy_parameter)?,
            })
        })?;
        let (scripts, sets) = (self.legacy_scripts.len(), self.legacy_parameter_sets.len());
        self.clip_mappings = r.list(9, |r| {
            let clip_index = r.i32()?;
            let at = r.here();
            let kind = r.u8()?;
            let at_index = r.here();
            let i = r.i32()?;
            let (resource, limit) = match kind {
                0 => (ClipResource::ParameterSet(i), sets),
                1 => (ClipResource::LegacyScript(i), scripts),
                t => return Err(r.bad_at(at, format!("clip mapping resource type {t}: expected 0 or 1"))),
            };
            if usize::try_from(i).map_or(true, |i| i >= limit) {
                return Err(r.bad_at(at_index, format!("clip mapping resource {i}: there are {limit}")));
            }
            Ok(ClipMapping { clip_index, resource })
        })?;
        self.block_groups = block_group_names(r)?;
        self.blocks = blocks_v6(r, self.block_groups.len())?;
        Ok(())
    }

    fn read_v7(&mut self, r: &mut Archive<'_, '_>, data_version: u32) -> Result<()> {
        let at = r.here();
        let flags = r.u16()?;
        if flags & 0xFC00 != 0 {
            return Err(r.bad_at(at, format!("flags {flags:#x}: bits above 9 are not read")));
        }
        let visibility = u8::try_from(flags & 3).unwrap_or(3);
        self.decoration = Some(decoration(
            r,
            visibility,
            flags & (1 << 2) != 0,
            flags & (1 << 3) != 0,
        )?);
        self.pylons_disabled = flags & (1 << 4) != 0;
        self.track_base_empty = flags & (1 << 5) != 0;
        self.vehicle_collisions = flags & (1 << 6) != 0;
        self.random_start_line = flags & (1 << 7) != 0;
        self.server_communication = flags & (1 << 8) != 0;
        self.ignore_multiplayer_time_sync = flags & (1 << 9) != 0;
        self.angel_script_modules = r.list(1, |r| {
            let at = r.here();
            let flags = r.u8()?;
            if flags & 0xFC != 0 {
                return Err(r.bad_at(
                    at,
                    format!("AngelScript module flags {flags:#x}: bits above 1 are not read"),
                ));
            }
            let module_name = if flags & 2 == 0 { Some(r.string()?) } else { None };
            if flags & 1 == 0 {
                return Err(r.bad("an AngelScript module with bytecode is not read"));
            }
            Ok(AngelScriptModule { module_name })
        })?;
        self.parameter_sets = r.list(8, |r| {
            Ok(ParameterSet {
                name: r.string()?,
                parameters: r.list(5, parameter)?,
            })
        })?;
        self.trigger_groups = r.list(5, trigger_group)?;
        self.block_groups = block_group_names(r)?;
        let at = r.here();
        let embedded_count = r.i32()?;
        let embedded_count = usize::try_from(embedded_count)
            .ok()
            .filter(|&c| c.saturating_mul(EMBEDDED_BLOCK_MIN) <= r.remaining())
            .ok_or_else(|| {
                r.bad_at(
                    at,
                    format!("{embedded_count} embedded block models do not fit in the chunk"),
                )
            })?;
        self.material_model_refs = r.list(8, |r| {
            let at = r.here();
            let instance_index = r.i32()?;
            register(r, at, instance_index, "material model")?;
            Ok(MaterialModelRef {
                instance_index,
                relative_path: r.string()?,
            })
        })?;
        self.replacement_textures = replacement_textures(r)?;
        self.embedded_images = r.list(16, embedded_image)?;
        self.embedded_blocks = (0..embedded_count)
            .map(|_| embedded_block_data(r, data_version))
            .collect::<Result<Vec<_>>>()?;
        self.vehicle_identifiers = r.list(12, |r| r.ident())?;
        let counts = Counts {
            trigger_groups: self.trigger_groups.len(),
            block_groups: self.block_groups.len(),
            embedded: self.embedded_blocks.len(),
        };
        self.blocks = blocks_v7(r, &counts)?;
        Ok(())
    }
}

/// The least bytes an embedded block model takes: two ids, a flags byte and the four spawn vectors.
const EMBEDDED_BLOCK_MIN: usize = 4 + 4 + 1 + 48;

/// A block of the encrypted chunk `0x3F001000`.
#[derive(Clone, Debug, PartialEq)]
pub struct EncryptedBlock {
    /// The block's index in the map's block data.
    pub index: u32,
    /// The flags: bit 0 outside the boundaries, 1 moved, 2 rotated, 3 scaled, 4 inverted, 5 vanilla terrain, 6 spawn
    /// point fix, 7 dynamic, 8 invisible, 9 collision disabled, 10 classic mode, 11 classic terrain, 14 has a group.
    pub flags: u16,
    /// The block's coordinate outside the boundaries (bit 0).
    pub outside_boundaries: Option<[u8; 3]>,
    /// The offset (bit 1), unless the block is vanilla terrain.
    pub offset: Option<[f32; 3]>,
    /// The rotation (bit 2), unless the block is vanilla terrain.
    pub rotation: Option<[f32; 3]>,
    /// The scale (bit 3), unless the block is vanilla terrain.
    pub scale: Option<[f32; 3]>,
    /// The block group's name (bit 14), unless the block is vanilla terrain.
    pub group: Option<String>,
}

/// The data of chunk `0x3F001000`, which TMUnlimiter 1.x writes encrypted (`TMUnlimiterData` in GBX.NET).
#[derive(Clone, Debug, PartialEq)]
pub struct EncryptedUnlimiter {
    /// The challenge flags: bit 0 sky only, 1 decoration moved, 2 nothing, 3 decoration scaled, 4 track base empty,
    /// 5 vanilla mode, 8 warp, 9 pylons disabled.
    pub flags: u16,
    /// The decoration's offset.
    pub decoration_offset: Option<[f32; 3]>,
    /// The decoration's scale.
    pub decoration_scale: Option<[f32; 3]>,
    /// The blocks.
    pub blocks: Vec<EncryptedBlock>,
    /// An integer of unknown meaning.
    pub u01: i32,
}

/// The chunk's byte crypt, which is its own inverse.
pub fn crypt(data: &mut [u8]) {
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    for (offset, byte) in (0u32..).zip(data.iter_mut()) {
        let mut hash = len.wrapping_mul(len.wrapping_mul(2).wrapping_sub(offset)) ^ 0xEAD9_C8B3;
        hash = hash.wrapping_add(offset.wrapping_mul(3) % 0x7F);
        if offset % 5 < 2 {
            hash = !hash;
        }
        let [low, ..] = (!(u32::from(*byte) ^ hash)).to_le_bytes();
        *byte = low;
    }
}

impl EncryptedUnlimiter {
    /// Reads the rest of the chunk: it is decrypted, then read to its end.
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<EncryptedUnlimiter> {
        let origin = r.here();
        let mut data = r.bytes(r.remaining())?.to_vec();
        crypt(&mut data);
        let space = r.space();
        let mut inner = Archive::new(&data, origin, space, &mut *r.st);
        let n = EncryptedUnlimiter::read_decrypted(&mut inner)?;
        inner.end("the decrypted chunk")?;
        Ok(n)
    }

    fn read_decrypted(r: &mut Archive<'_, '_>) -> Result<EncryptedUnlimiter> {
        let at = r.here();
        let flags = r.u16()?;
        if flags & !0x33F != 0 {
            return Err(r.bad_at(
                at,
                format!("challenge flags {flags:#x}: only bits 0 to 5, 8 and 9 are read"),
            ));
        }
        let moved = flags & (1 << 1) != 0;
        let scaled = flags & (1 << 3) != 0;
        if flags & (1 << 2) != 0 && (moved || scaled) {
            return Err(r.bad_at(
                at,
                format!("challenge flags {flags:#x}: a moved or scaled decoration with visibility nothing"),
            ));
        }
        let decoration_offset = if moved { Some(vec3(r)?) } else { None };
        let decoration_scale = if scaled { Some(vec3(r)?) } else { None };
        let blocks = r.list(6, |r| {
            let index = index(r, "block")?;
            let at = r.here();
            let flags = r.u16()?;
            if flags & 0xB000 != 0 {
                return Err(r.bad_at(
                    at,
                    format!("block flags {flags:#x}: bits 12, 13 and 15 are not read"),
                ));
            }
            let mut b = EncryptedBlock {
                index,
                flags,
                outside_boundaries: None,
                offset: None,
                rotation: None,
                scale: None,
                group: None,
            };
            if flags & 1 != 0 {
                b.outside_boundaries = Some(r.byte3()?);
            }
            if flags & (1 << 5) == 0 {
                if flags & (1 << 1) != 0 {
                    b.offset = Some(vec3(r)?);
                }
                if flags & (1 << 2) != 0 {
                    b.rotation = Some(vec3(r)?);
                }
                if flags & (1 << 3) != 0 {
                    b.scale = Some(vec3(r)?);
                }
                if flags & (1 << 14) != 0 {
                    b.group = Some(r.string()?);
                }
            }
            Ok(b)
        })?;
        Ok(EncryptedUnlimiter {
            flags,
            decoration_offset,
            decoration_scale,
            blocks,
            u01: r.i32()?,
        })
    }
}
