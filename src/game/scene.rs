//! The scene objects a block info shows: mobils (`CSceneMobil`) with their items (`CHmsItem`) and links
//! (`CSceneObjectLink`), sound sources, and the motions that move them (`CMotionPlayer` with its command and shader
//! tracks).
//!
//! A mobil is an object of the scene (`CSceneObject`, class `0x0A005000`, whose chunks come first); it holds an item
//! (the solid shown) and links to child objects, each with a local transform.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};
use crate::plug::{Iso4, iso4};

/// A node reference.
type Node = Option<NodeIndex>;

/// The chunks of `CSceneObject` (class `0x0A005000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneObject {
    /// Chunk `0x0A005001`: the object's name.
    pub name: Option<Id>,
    /// Chunk `0x0A005002`: a bool GBX.NET does not name.
    pub u02: Option<bool>,
    /// Chunk `0x0A005003`: the motion (`CMotion`) that moves the object.
    pub motion: Option<Node>,
}

impl SceneObject {
    fn chunk(&mut self, r: &mut Archive<'_, '_>, id: u32) -> Result<bool> {
        match id {
            0x0A00_5001 => self.name = Some(r.id()?),
            0x0A00_5002 => self.u02 = Some(r.bool()?),
            0x0A00_5003 => self.motion = Some(r.node_ref()?),
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// The flags of an item and its visibility id (chunks `0x0600300B` to `0x06003011`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemFlags {
    /// The chunk that stored them: later chunks mask the flags differently when the game loads them.
    pub chunk: u32,
    /// The flags (`FlagsItem`, or `FlagsLegacy64` before chunk `0x0600300E`).
    pub flags: u64,
    /// The visibility id (`VisibleId`).
    pub visible_id: u16,
}

/// An item (`CHmsItem`, class `0x06003000`): a solid in the scene.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HmsItem {
    /// Chunk `0x06003001`: the solid (`CPlugSolid`).
    pub solid: Option<Node>,
    /// The flags chunk.
    pub flags: Option<ItemFlags>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl HmsItem {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<HmsItem> {
        let mut n = HmsItem::default();
        n.chunks = r.chunks("CHmsItem", |r, id| {
            match id {
                0x0600_3001 => n.solid = Some(r.node_ref()?),
                0x0600_300B..=0x0600_3011 => {
                    n.flags = Some(ItemFlags {
                        chunk: id,
                        flags: r.u64()?,
                        visible_id: r.u16()?,
                    });
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// What a mobil pointer holds (`CSceneMobil.MobilPtr`): a mobil model with this instance's name and links, or a mobil.
#[derive(Clone, Debug, PartialEq)]
pub enum MobilPtr {
    /// A model (`CSceneMobil`, inline or an external file) instanced under a name.
    Model {
        /// The model.
        model: NodeIndex,
        /// The instance's name.
        name: Id,
        /// The instance's links (`CSceneObjectLink`).
        links: Vec<Node>,
    },
    /// A mobil.
    Mobil(Node),
}

impl MobilPtr {
    fn read(r: &mut Archive<'_, '_>) -> Result<MobilPtr> {
        let Some(model) = r.node_ref()? else {
            return Ok(MobilPtr::Mobil(r.node_ref()?));
        };
        // Older data versions store instance data that depends on the model's class (containers, menu frames).
        r.version("a mobil pointer's data", &[2])?;
        let at = r.here();
        let terminator = r.i32()?;
        if terminator != -1 {
            return Err(r.bad_at(
                at,
                format!("{terminator}: expected -1 after a mobil pointer's data version"),
            ));
        }
        Ok(MobilPtr::Model {
            model,
            name: r.id()?,
            links: r.list(4, |r| r.node_ref())?,
        })
    }
}

/// A link from a mobil to a child object (`CSceneObjectLink`, class `0x0A014000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectLink {
    /// Chunk `0x0A014001`: the child, where the object sits relative to its parent, and a bool GBX.NET does not name.
    pub target: Option<(LinkTarget, Iso4, bool)>,
    /// Chunk `0x0A014002`: the id of the mobil tree node the link attaches to, and two bools GBX.NET does not name.
    pub mobil_tree: Option<(Id, bool, bool)>,
    /// Chunk `0x0A014003`: an id GBX.NET does not name.
    pub u03: Option<Id>,
    /// The node's chunks.
    pub chunks: Chunks,
}

/// What a link points at.
#[derive(Clone, Debug, PartialEq)]
pub enum LinkTarget {
    /// A mobil pointer.
    Mobil(MobilPtr),
    /// An object (`CSceneObject`).
    Object(Node),
}

impl ObjectLink {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<ObjectLink> {
        let mut n = ObjectLink::default();
        n.chunks = r.chunks("CSceneObjectLink", |r, id| {
            match id {
                0x0A01_4001 => {
                    let target = if r.bool()? {
                        LinkTarget::Mobil(MobilPtr::read(r)?)
                    } else {
                        LinkTarget::Object(r.node_ref()?)
                    };
                    n.target = Some((target, iso4(r)?, r.bool()?));
                }
                0x0A01_4002 => n.mobil_tree = Some((r.id()?, r.bool()?, r.bool()?)),
                0x0A01_4003 => n.u03 = Some(r.id()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A mobil (`CSceneMobil`, class `0x0A011000`): an object showing an item, with links to child objects.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mobil {
    /// The object chunks.
    pub object: SceneObject,
    /// Chunk `0x0A011003`: the links (`CSceneObjectLink`).
    pub links: Option<Vec<Node>>,
    /// Chunk `0x0A011005`: the item, stored in place.
    pub item: Option<Box<HmsItem>>,
    /// Chunk `0x0A011006`: the message handler (`CSceneMessageHandler`).
    pub message_handler: Option<Node>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl Mobil {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Mobil> {
        let mut n = Mobil::default();
        n.chunks = r.chunks("CSceneMobil", |r, id| {
            if n.object.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x0A01_1003 => n.links = Some(r.list(4, |r| r.node_ref())?),
                0x0A01_1005 => n.item = Some(Box::new(r.direct(0x0100_1000, HmsItem::read)?)),
                0x0A01_1006 => n.message_handler = Some(r.node_ref()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A sound source (`CHmsSoundSource`, class `0x0600D000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HmsSoundSource {
    /// Chunk `0x0600D003`: the volume, the pitch and the rpm or speed the sound follows.
    pub volume_pitch_speed: Option<[f32; 3]>,
    /// Chunk `0x0600D004`: the volumic size.
    pub volumic_size: Option<[f32; 3]>,
    /// Chunk `0x0600D005`: the sound (`CPlugSound`), its priority adjustment and whether it uses low quality.
    pub sound: Option<(Node, f32, bool)>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl HmsSoundSource {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<HmsSoundSource> {
        let mut n = HmsSoundSource::default();
        n.chunks = r.chunks("CHmsSoundSource", |r, id| {
            match id {
                0x0600_D003 => n.volume_pitch_speed = Some(r.vec3()?),
                0x0600_D004 => n.volumic_size = Some(r.vec3()?),
                0x0600_D005 => n.sound = Some((r.node_ref()?, r.f32()?, r.bool()?)),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A sound source in the scene (`CSceneSoundSource`, class `0x0A00E000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneSoundSource {
    /// The object chunks.
    pub object: SceneObject,
    /// Chunk `0x0A009000` (`CScenePoc`): whether the source is active.
    pub is_active: Option<bool>,
    /// Chunk `0x0A00E000`: the sound source, stored in place.
    pub source: Option<Box<HmsSoundSource>>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl SceneSoundSource {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<SceneSoundSource> {
        let mut n = SceneSoundSource::default();
        n.chunks = r.chunks("CSceneSoundSource", |r, id| {
            if n.object.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x0A00_9000 => n.is_active = Some(r.bool()?),
                // `CHmsSoundSource` derives from `CHmsPoc` (`0x0600B000` in the game's packs).
                0x0A00_E000 => n.source = Some(Box::new(r.direct(0x0600_B000, HmsSoundSource::read)?)),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// How a motion command's wave runs (`EWaveType`).
pub type WaveType = i32;

/// A motion command (`CMotionCmdBase`, class `0x08029000`): a periodic wave.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionCmdBase {
    /// Chunks `0x08029000` to `0x08029002`: the period in milliseconds, the phase (a fraction of the period), the wave
    /// type and whether it runs once.
    pub wave: Option<(u32, f32, WaveType, bool)>,
    /// Chunks `0x08029001` and `0x08029002`: whether the phase is absolute.
    pub is_absolute_phase: Option<bool>,
    /// Chunk `0x08029002`: the command's parameters (`CMotionCmdBaseParams`).
    pub params: Option<Node>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl MotionCmdBase {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<MotionCmdBase> {
        let mut n = MotionCmdBase::default();
        n.chunks = r.chunks("CMotionCmdBase", |r, id| {
            match id {
                0x0802_9000..=0x0802_9002 => {
                    n.wave = Some((r.u32()?, r.f32()?, r.i32()?, r.bool()?));
                    if id >= 0x0802_9001 {
                        n.is_absolute_phase = Some(r.bool()?);
                    }
                    if id >= 0x0802_9002 {
                        n.params = Some(r.node_ref()?);
                    }
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A motion player (`CMotionPlayer`, class `0x08034000`): a command and the tracks it drives.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionPlayer {
    /// Chunk `0x08001000` (`CMotion`): the motion's name.
    pub motion_name: Option<Id>,
    /// Chunk `0x08034004`: the command, stored in place.
    pub command: Option<Box<MotionCmdBase>>,
    /// Chunk `0x08034004`: the play state (0 playing, 1 paused, 2 stopped).
    pub play_state: Option<u32>,
    /// Chunk `0x08034004`: whether the motion drives physics.
    pub is_physics: Option<bool>,
    /// Chunk `0x08034004`: the name stored again.
    pub name: Option<Id>,
    /// Chunk `0x08034004`: the tracks (`CMotionTrack`, such as `CMotionShader`).
    pub tracks: Option<Vec<Node>>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl MotionPlayer {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<MotionPlayer> {
        let mut n = MotionPlayer::default();
        n.chunks = r.chunks("CMotionPlayer", |r, id| {
            match id {
                0x0800_1000 => n.motion_name = Some(r.id()?),
                0x0803_4004 => {
                    n.command = Some(Box::new(r.direct(0x0100_5000, MotionCmdBase::read)?));
                    let saved = r.u32()?;
                    n.play_state = Some(if saved == 3 { r.u32()? } else { saved });
                    n.is_physics = Some(r.bool()?);
                    n.name = Some(r.id()?);
                    n.tracks = Some(r.list(4, |r| r.node_ref())?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A shader track (`CMotionShader`, class `0x0802B000`): the material and function shader a motion animates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionShader {
    /// Chunk `0x0802B000`: the material (`CPlugMaterial`), the shader (`CPlugShader`) and the function shader
    /// (`CFuncShader`).
    pub shaders: Option<(Node, Node, Node)>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl MotionShader {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<MotionShader> {
        let mut n = MotionShader::default();
        n.chunks = r.chunks("CMotionShader", |r, id| {
            match id {
                0x0802_B000 => n.shaders = Some((r.node_ref()?, r.node_ref()?, r.node_ref()?)),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
