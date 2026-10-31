//! A decoration (`CGameCtnDecoration`): the setting a map is played in, made of a size, a mood, the audio and the
//! terrain modifiers, and the solids that decorate it (`CPlugDecoratorSolid` with its `CPlugDecoratorTree`s).
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::game::collector::Collector;
use crate::node::{Chunks, NodeIndex};

/// A node reference.
type Node = Option<NodeIndex>;

/// A deprecated node list: the version the game wrote it with (always 10), the count and the references.
fn deprecated_list(r: &mut Archive<'_, '_>) -> Result<Vec<Node>> {
    r.version("deprecated node list", &[10])?;
    r.list(4, |r| r.node_ref())
}

/// A node and the name it is stored under.
pub type NamedNode = (Id, Node);

/// A decoration (`CGameCtnDecoration`, class `0x03038000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Decoration {
    /// The collector chunks.
    pub collector: Collector,
    /// Chunk `0x03038011`: the size (`CGameCtnDecorationSize`).
    pub size: Option<Node>,
    /// Chunk `0x03038012`: the audio (`CGameCtnDecorationAudio`).
    pub audio: Option<Node>,
    /// Chunk `0x03038013`: the mood (`CGameCtnDecorationMood`).
    pub mood: Option<Node>,
    /// Chunk `0x03038014`: the decorator solid of the warp (`CPlugDecoratorSolid`).
    pub decorator_solid_warp: Option<Node>,
    /// Chunk `0x03038015`: the terrain modifier for covered terrain (`CGameCtnDecorationTerrainModifier`).
    pub terrain_modifier_covered: Option<Node>,
    /// Chunk `0x03038016`: the terrain modifier for the base.
    pub terrain_modifier_base: Option<Node>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl Decoration {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Decoration> {
        let mut n = Decoration::default();
        n.chunks = r.chunks("CGameCtnDecoration", |r, id| {
            if n.collector.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x0303_8011 => n.size = Some(r.node_ref()?),
                0x0303_8012 => n.audio = Some(r.node_ref()?),
                0x0303_8013 => n.mood = Some(r.node_ref()?),
                0x0303_8014 => n.decorator_solid_warp = Some(r.node_ref()?),
                0x0303_8015 => n.terrain_modifier_covered = Some(r.node_ref()?),
                0x0303_8016 => n.terrain_modifier_base = Some(r.node_ref()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// The size of a decoration (`CGameCtnDecorationSize`, class `0x0303B000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecorationSize {
    /// Chunk `0x0303B000`: the edition zone's corners and an `f32` GBX.NET does not name.
    pub edition_zone: Option<([f32; 2], [f32; 2], f32)>,
    /// Chunk `0x0303B001`: the base height, the size in blocks and the scene layout (`CSceneLayout`).
    pub base: Option<(i32, [i32; 3], Node)>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl DecorationSize {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<DecorationSize> {
        let mut n = DecorationSize::default();
        n.chunks = r.chunks("CGameCtnDecorationSize", |r, id| {
            match id {
                0x0303_B000 => n.edition_zone = Some((r.vec2()?, r.vec2()?, r.f32()?)),
                0x0303_B001 => n.base = Some((r.i32()?, r.int3()?, r.node_ref()?)),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// The mood of a decoration (`CGameCtnDecorationMood`, class `0x0303A000`): its sun, remapping and lighting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecorationMood {
    /// Chunk `0x0303A000`: the latitude, the longitude, the offset from GMT, and the times of sunrise and sunfall.
    pub sun: Option<Sun>,
    /// Chunk `0x0303A001`: the day time the remapping starts at, the remapping skin (`CPlugGameSkin`) and its folder.
    pub remapping: Option<(f32, Node, String)>,
    /// Chunk `0x0303A002`: the shadows.
    pub shadows: Option<Shadows>,
    /// Chunk `0x0303A004`: the light map (`CHmsLightMap`).
    pub light_map: Option<Node>,
    /// Chunk `0x0303A005`: the ambient occlusion (`CHmsAmbientOcc`).
    pub ambient_occlusion: Option<Node>,
    /// Chunk `0x0303A006`: the intensity of the sun and the moon.
    pub sun_moon_intensity: Option<f32>,
    /// The node's chunks.
    pub chunks: Chunks,
}

/// Chunk `0x0303A000` of a mood.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sun {
    /// The latitude.
    pub latitude: f32,
    /// The longitude.
    pub longitude: f32,
    /// The offset from GMT.
    pub delta_gmt: f32,
    /// The time of sunrise, in milliseconds.
    pub time_sun_rise: i32,
    /// The time of sunfall, in milliseconds.
    pub time_sun_fall: i32,
}

/// Chunk `0x0303A002` of a mood.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadows {
    /// How many shadows the player's car casts.
    pub count_car_human: i32,
    /// How many shadows the opponents' cars cast.
    pub count_car_opponent: i32,
    /// The cars' shadow intensity.
    pub car_intensity: f32,
    /// Whether the scene casts shadows.
    pub scene: bool,
    /// Whether the background is lit locally.
    pub background_is_locally_lighted: bool,
}

impl DecorationMood {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<DecorationMood> {
        let mut n = DecorationMood::default();
        n.chunks = r.chunks("CGameCtnDecorationMood", |r, id| {
            match id {
                0x0303_A000 => {
                    n.sun = Some(Sun {
                        latitude: r.f32()?,
                        longitude: r.f32()?,
                        delta_gmt: r.f32()?,
                        time_sun_rise: r.i32()?,
                        time_sun_fall: r.i32()?,
                    });
                }
                0x0303_A001 => n.remapping = Some((r.f32()?, r.node_ref()?, r.string()?)),
                0x0303_A002 => {
                    n.shadows = Some(Shadows {
                        count_car_human: r.i32()?,
                        count_car_opponent: r.i32()?,
                        car_intensity: r.f32()?,
                        scene: r.bool()?,
                        background_is_locally_lighted: r.bool()?,
                    });
                }
                0x0303_A004 => n.light_map = Some(r.node_ref()?),
                0x0303_A005 => n.ambient_occlusion = Some(r.node_ref()?),
                0x0303_A006 => n.sun_moon_intensity = Some(r.f32()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A terrain modifier of a decoration (`CGameCtnDecorationTerrainModifier`, class `0x0303C000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecorationTerrainModifier {
    /// Chunk `0x0303C000`: the remapping skin (`CPlugGameSkin`) and its folder.
    pub remapping: Option<(Node, String)>,
    /// Chunk `0x0303C001`: the modifier's name.
    pub name: Option<Id>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl DecorationTerrainModifier {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<DecorationTerrainModifier> {
        let mut n = DecorationTerrainModifier::default();
        n.chunks = r.chunks("CGameCtnDecorationTerrainModifier", |r, id| {
            match id {
                0x0303_C000 => n.remapping = Some((r.node_ref()?, r.string()?)),
                0x0303_C001 => n.name = Some(r.id()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// The audio of a decoration (`CGameCtnDecorationAudio`, class `0x03039000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecorationAudio {
    /// Chunk `0x03039000`: the sounds (`CPlugSound`) by name, such as `Ambient` or `RaceVictoryGold`; the musics
    /// follow. GBX.NET reads this chunk as empty.
    pub sounds: Option<Vec<NamedNode>>,
    /// Chunk `0x03039000`: the musics by name (`MusicRace`, `MusicEditor`, `MusicReplay`).
    pub musics: Option<Vec<NamedNode>>,
    /// Chunk `0x03039001`: the outside audio environments, open and enclosed (`CPlugAudioEnvironment`).
    pub environments: Option<(Node, Node)>,
    /// Chunk `0x03039002`: how the sound attenuates in the editor.
    pub sound_attenuation_in_editor: Option<f32>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl DecorationAudio {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<DecorationAudio> {
        let mut n = DecorationAudio::default();
        n.chunks = r.chunks("CGameCtnDecorationAudio", |r, id| {
            match id {
                0x0303_9000 => {
                    n.sounds = Some(r.list(8, |r| Ok((r.id()?, r.node_ref()?)))?);
                    n.musics = Some(r.list(8, |r| Ok((r.id()?, r.node_ref()?)))?);
                }
                0x0303_9001 => n.environments = Some((r.node_ref()?, r.node_ref()?)),
                0x0303_9002 => n.sound_attenuation_in_editor = Some(r.f32()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A tree of a decorator (`CPlugDecoratorTree`, class `0x090A2000`): a node of a solid's tree and the conditions it
/// is visible, exists and casts shadows under.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecoratorTree {
    /// Chunks `0x090A2007` to `0x090A2009`: the tree's properties.
    pub tree: Option<DecoratorTreeProperties>,
    /// The node's chunks.
    pub chunks: Chunks,
}

/// The properties of a decorator tree.
#[derive(Clone, Debug, PartialEq)]
pub struct DecoratorTreeProperties {
    /// The chunk that stored them.
    pub chunk: u32,
    /// The id of the solid's tree node.
    pub tree_id: Id,
    /// The material (`CPlugMaterial`).
    pub material: Node,
    /// The tree's light (`CPlugTreeLight`).
    pub tree_light: Node,
    /// The condition the tree exists under.
    pub exist_cond: i32,
    /// The condition the tree is visible under.
    pub visible_cond: i32,
    /// Whether the visibility condition applies to the children.
    pub visible_apply_on_childs: bool,
    /// The condition the tree casts shadows under.
    pub shadow_caster_cond: i32,
    /// Whether the shadow condition applies to the children.
    pub shadow_caster_apply_on_childs: bool,
    /// Whether the visual becomes the surface.
    pub transform_visual_to_surface: bool,
    /// Chunks `0x090A2008` and later: whether the tree has no location.
    pub no_location: Option<bool>,
    /// Chunk `0x090A2009`: the condition under which the tree collides.
    pub collidable_cond: Option<i32>,
}

impl DecoratorTree {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<DecoratorTree> {
        let mut n = DecoratorTree::default();
        n.chunks = r.chunks("CPlugDecoratorTree", |r, id| {
            match id {
                0x090A_2007..=0x090A_2009 => {
                    let mut tree = DecoratorTreeProperties {
                        chunk: id,
                        tree_id: r.id()?,
                        material: r.node_ref()?,
                        tree_light: r.node_ref()?,
                        exist_cond: r.i32()?,
                        visible_cond: r.i32()?,
                        visible_apply_on_childs: r.bool()?,
                        shadow_caster_cond: r.i32()?,
                        shadow_caster_apply_on_childs: r.bool()?,
                        transform_visual_to_surface: r.bool()?,
                        no_location: None,
                        collidable_cond: None,
                    };
                    if id >= 0x090A_2008 {
                        tree.no_location = Some(r.bool()?);
                    }
                    if id >= 0x090A_2009 {
                        tree.collidable_cond = Some(r.i32()?);
                    }
                    n.tree = Some(tree);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A decorator solid (`CPlugDecoratorSolid`, class `0x090A3000`): the trees decorating a solid.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecoratorSolid {
    /// Chunk `0x090A3000`: the trees (`CPlugDecoratorTree`).
    pub trees: Option<Vec<Node>>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl DecoratorSolid {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<DecoratorSolid> {
        let mut n = DecoratorSolid::default();
        n.chunks = r.chunks("CPlugDecoratorSolid", |r, id| {
            match id {
                0x090A_3000 => n.trees = Some(deprecated_list(r)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
