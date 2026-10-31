//! MediaTracker clips, tracks, blocks and their animation functions in TrackMania Forever.
//! Chunk layouts follow GBX.NET 2.5.0. Derived triangle and function nodes read their base-class chunk ids.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
use crate::archive::{Archive, Id, PackDesc};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};

fn refs(r: &mut Archive<'_, '_>) -> Result<Vec<Option<NodeIndex>>> {
    r.version("deprecated node list", &[10])?;
    r.list(4, |r| r.node_ref())
}
/// Trigger data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct Trigger {
    /// Ref coord.
    pub ref_coord: [i32; 3],
    /// Ref dir.
    pub ref_dir: i32,
    /// Condition.
    pub condition: i32,
    /// Condition value.
    pub condition_value: f32,
    /// Coords.
    pub coords: Vec<[i32; 3]>,
}
impl Trigger {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            ref_coord: r.int3()?,
            ref_dir: r.i32()?,
            condition: r.i32()?,
            condition_value: r.f32()?,
            coords: r.list(12, |r| r.int3())?,
        })
    }
}
/// CGameCtnMediaClipGroup (`0x0307a000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaClipGroup {
    /// Clips.
    pub clips: Option<Vec<Option<NodeIndex>>>,
    /// Triggers.
    pub triggers: Option<Vec<Trigger>>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaClipGroup {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            clips: None,
            triggers: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaClipGroup", |r, id| {
            match id {
                0x0307A003 => {
                    n.clips = Some(refs(r)?);
                    n.triggers = Some(r.list(28, Trigger::read)?);
                    if n.clips.as_ref().map(Vec::len) != n.triggers.as_ref().map(Vec::len) {
                        return Err(r.bad("clip and trigger counts differ"));
                    }
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CGameCtnMediaClip (`0x03079000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaClip {
    /// Tracks.
    pub tracks: Option<Vec<Option<NodeIndex>>>,
    /// Name.
    pub name: Option<String>,
    /// Scene.
    pub scene: Option<Option<NodeIndex>>,
    /// Local player clip ent index.
    pub local_player_clip_ent_index: Option<i32>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaClip {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            tracks: None,
            name: None,
            scene: None,
            local_player_clip_ent_index: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaClip", |r, id| {
            match id {
                0x03079004 => n.scene = Some(r.node_ref()?),
                0x03079005 => {
                    n.tracks = Some(refs(r)?);
                    n.name = Some(r.string()?);
                }
                0x03079007 => n.local_player_clip_ent_index = Some(r.i32()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CGameCtnMediaTrack (`0x03078000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaTrack {
    /// Name.
    pub name: Option<String>,
    /// Blocks.
    pub blocks: Option<Vec<Option<NodeIndex>>>,
    /// U01.
    pub u01: Option<i32>,
    /// Is keep playing.
    pub is_keep_playing: Option<bool>,
    /// Is read only.
    pub is_read_only: Option<bool>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaTrack {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            name: None,
            blocks: None,
            u01: None,
            is_keep_playing: None,
            is_read_only: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaTrack", |r, id| {
            match id {
                0x03078001 => {
                    n.name = Some(r.string()?);
                    n.blocks = Some(refs(r)?);
                    n.u01 = Some(r.i32()?);
                }
                0x03078004 => {
                    n.is_keep_playing = Some(r.bool()?);
                    n.is_read_only = Some(r.bool()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CGameCtnMediaBlockText (`0x030a8000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockText {
    /// Text.
    pub text: Option<String>,
    /// Effect.
    pub effect: Option<Option<NodeIndex>>,
    /// Color.
    pub color: Option<[f32; 3]>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockText {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            text: None,
            effect: None,
            color: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockText", |r, id| {
            match id {
                0x030A8001 => {
                    n.text = Some(r.string()?);
                    n.effect = Some(r.node_ref()?);
                }
                0x030A8002 => n.color = Some(r.vec3()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CameraCustomKey data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraCustomKey {
    /// Time.
    pub time: f32,
    /// Interpolation.
    pub interpolation: i32,
    /// U01.
    pub u01: i32,
    /// U02.
    pub u02: i32,
    /// Position.
    pub position: [f32; 3],
    /// Pitch yaw roll.
    pub pitch_yaw_roll: [f32; 3],
    /// Fov.
    pub fov: f32,
    /// Anchor rot.
    pub anchor_rot: bool,
    /// Anchor.
    pub anchor: i32,
    /// Anchor vis.
    pub anchor_vis: bool,
    /// Target.
    pub target: i32,
    /// Target position.
    pub target_position: [f32; 3],
    /// Left tangent.
    pub left_tangent: [f32; 3],
    /// Right tangent.
    pub right_tangent: [f32; 3],
}
impl CameraCustomKey {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            time: r.f32()?,
            interpolation: r.i32()?,
            u01: r.i32()?,
            u02: r.i32()?,
            position: r.vec3()?,
            pitch_yaw_roll: r.vec3()?,
            fov: r.f32()?,
            anchor_rot: r.bool()?,
            anchor: r.i32()?,
            anchor_vis: r.bool()?,
            target: r.i32()?,
            target_position: r.vec3()?,
            left_tangent: r.vec3()?,
            right_tangent: r.vec3()?,
        })
    }
}
/// CGameCtnMediaBlockCameraCustom (`0x030a2000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockCameraCustom {
    /// Keys.
    pub keys: Option<Vec<CameraCustomKey>>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockCameraCustom {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            keys: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockCameraCustom", |r, id| {
            match id {
                0x030A2005 => n.keys = Some(r.list(96, CameraCustomKey::read)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// FadeKey data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct FadeKey {
    /// Time.
    pub time: f32,
    /// Opacity.
    pub opacity: f32,
}
impl FadeKey {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            time: r.f32()?,
            opacity: r.f32()?,
        })
    }
}
/// CGameCtnMediaBlockTransitionFade (`0x030ab000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockTransitionFade {
    /// Keys.
    pub keys: Option<Vec<FadeKey>>,
    /// Color.
    pub color: Option<[f32; 4]>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockTransitionFade {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            keys: None,
            color: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockTransitionFade", |r, id| {
            match id {
                0x030AB000 => {
                    n.keys = Some(r.list(8, FadeKey::read)?);
                    n.color = Some(r.vec4()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CGameCtnMediaBlockCameraGame (`0x03084000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockCameraGame {
    /// Start.
    pub start: Option<f32>,
    /// End.
    pub end: Option<f32>,
    /// Game cam id.
    pub game_cam_id: Option<Id>,
    /// Clip ent id.
    pub clip_ent_id: Option<i32>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockCameraGame {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            start: None,
            end: None,
            game_cam_id: None,
            clip_ent_id: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockCameraGame", |r, id| {
            match id {
                0x03084003 => {
                    n.start = Some(r.f32()?);
                    n.end = Some(r.f32()?);
                    n.game_cam_id = Some(r.id()?);
                    n.clip_ent_id = Some(r.i32()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// TriangleGeometry data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct TriangleGeometry {
    /// Times.
    pub times: Vec<f32>,
    /// Positions.
    pub positions: Vec<Vec<[f32; 3]>>,
    /// Vertices.
    pub vertices: Vec<[f32; 4]>,
    /// Triangles.
    pub triangles: Vec<[i32; 3]>,
    /// Pick mode 3d.
    pub pick_mode_3d: i32,
    /// Pick plane normal.
    pub pick_plane_normal: [f32; 3],
    /// Pick plane position.
    pub pick_plane_position: [f32; 3],
}
impl TriangleGeometry {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let times = r.list(4, |r| r.f32())?;
        let rows = r.count(0)?;
        let columns = r.count(0)?;
        if rows != times.len() {
            return Err(r.bad("triangle position row count differs from key count"));
        }
        let count = rows
            .checked_mul(columns)
            .and_then(|x| x.checked_mul(12))
            .ok_or_else(|| r.bad("triangle position matrix size overflow"))?;
        if count > r.remaining() {
            return Err(r.bad("triangle position matrix exceeds remaining bytes"));
        }
        let mut positions = Vec::with_capacity(rows);
        for _ in 0..rows {
            let mut row = Vec::with_capacity(columns);
            for _ in 0..columns {
                row.push(r.vec3()?);
            }
            positions.push(row);
        }
        let vertices = r.list(16, |r| r.vec4())?;
        if vertices.len() != columns {
            return Err(r.bad("triangle vertex count differs from matrix width"));
        }
        let triangles = r.list(12, |r| r.int3())?;
        for triangle in &triangles {
            for i in triangle {
                if usize::try_from(*i).map_or(true, |i| i >= columns) {
                    return Err(r.bad("triangle vertex index out of range"));
                }
            }
        }
        Ok(Self {
            times,
            positions,
            vertices,
            triangles,
            pick_mode_3d: r.i32()?,
            pick_plane_normal: r.vec3()?,
            pick_plane_position: r.vec3()?,
        })
    }
}
/// CGameCtnMediaBlockTriangles (`0x03029000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockTriangles {
    /// Geometry.
    pub geometry: Option<TriangleGeometry>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockTriangles {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            geometry: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockTriangles", |r, id| {
            match id {
                0x03029001 => n.geometry = Some(TriangleGeometry::read(r)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CGameCtnMediaBlockTriangles2D (`0x0304b000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockTriangles2D {
    /// Geometry.
    pub geometry: Option<TriangleGeometry>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockTriangles2D {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            geometry: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockTriangles2D", |r, id| {
            match id {
                0x03029001 => n.geometry = Some(TriangleGeometry::read(r)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CGameCtnMediaBlockTriangles3D (`0x0304c000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockTriangles3D {
    /// Geometry.
    pub geometry: Option<TriangleGeometry>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockTriangles3D {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            geometry: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockTriangles3D", |r, id| {
            match id {
                0x03029001 => n.geometry = Some(TriangleGeometry::read(r)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CGameCtnMediaBlockImage (`0x030a5000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockImage {
    /// Effect.
    pub effect: Option<Option<NodeIndex>>,
    /// Image.
    pub image: Option<PackDesc>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockImage {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            effect: None,
            image: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockImage", |r, id| {
            match id {
                0x030A5000 => {
                    n.effect = Some(r.node_ref()?);
                    n.image = Some(r.pack_desc()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// SoundKey data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundKey {
    /// Time.
    pub time: f32,
    /// Volume.
    pub volume: f32,
    /// Pan.
    pub pan: f32,
    /// Position.
    pub position: [f32; 3],
}
impl SoundKey {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            time: r.f32()?,
            volume: r.f32()?,
            pan: r.f32()?,
            position: r.vec3()?,
        })
    }
}
/// SoundSettings data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundSettings {
    /// Play count.
    pub play_count: i32,
    /// Is looping.
    pub is_looping: bool,
    /// Is music.
    pub is_music: bool,
}
impl SoundSettings {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            play_count: r.i32()?,
            is_looping: r.bool()?,
            is_music: r.bool()?,
        })
    }
}
/// CGameCtnMediaBlockSound (`0x030a7000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockSound {
    /// Settings.
    pub settings: Option<SoundSettings>,
    /// Sound.
    pub sound: Option<PackDesc>,
    /// Keys.
    pub keys: Option<Vec<SoundKey>>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockSound {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            settings: None,
            sound: None,
            keys: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockSound", |r, id| {
            match id {
                0x030A7003 => {
                    r.version("sound settings", &[0])?;
                    n.settings = Some(SoundSettings::read(r)?);
                }
                0x030A7004 => {
                    n.sound = Some(r.pack_desc()?);
                    r.version("sound keys", &[1])?;
                    n.keys = Some(r.list(24, SoundKey::read)?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// ColorsKey data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorsKey {
    /// Time.
    pub time: f32,
    /// Intensity.
    pub intensity: f32,
    /// Far intensity.
    pub far_intensity: f32,
    /// Distance.
    pub distance: f32,
    /// Far distance.
    pub far_distance: f32,
    /// Inverse.
    pub inverse: f32,
    /// Hue.
    pub hue: f32,
    /// Saturation.
    pub saturation: f32,
    /// Brightness.
    pub brightness: f32,
    /// Contrast.
    pub contrast: f32,
    /// Modulate rgb.
    pub modulate_rgb: [f32; 3],
    /// Blend rgb.
    pub blend_rgb: [f32; 3],
    /// Blend alpha.
    pub blend_alpha: f32,
    /// Far inverse.
    pub far_inverse: f32,
    /// Far hue.
    pub far_hue: f32,
    /// Far saturation.
    pub far_saturation: f32,
    /// Far brightness.
    pub far_brightness: f32,
    /// Far contrast.
    pub far_contrast: f32,
    /// Far modulate rgb.
    pub far_modulate_rgb: [f32; 3],
    /// Far blend rgb.
    pub far_blend_rgb: [f32; 3],
    /// Far blend alpha.
    pub far_blend_alpha: f32,
}
impl ColorsKey {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            time: r.f32()?,
            intensity: r.f32()?,
            far_intensity: r.f32()?,
            distance: r.f32()?,
            far_distance: r.f32()?,
            inverse: r.f32()?,
            hue: r.f32()?,
            saturation: r.f32()?,
            brightness: r.f32()?,
            contrast: r.f32()?,
            modulate_rgb: r.vec3()?,
            blend_rgb: r.vec3()?,
            blend_alpha: r.f32()?,
            far_inverse: r.f32()?,
            far_hue: r.f32()?,
            far_saturation: r.f32()?,
            far_brightness: r.f32()?,
            far_contrast: r.f32()?,
            far_modulate_rgb: r.vec3()?,
            far_blend_rgb: r.vec3()?,
            far_blend_alpha: r.f32()?,
        })
    }
}
/// CGameCtnMediaBlockFxColors (`0x03080000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockFxColors {
    /// Keys.
    pub keys: Option<Vec<ColorsKey>>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockFxColors {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            keys: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockFxColors", |r, id| {
            match id {
                0x03080003 => n.keys = Some(r.list(116, ColorsKey::read)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// DepthKey data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct DepthKey {
    /// Time.
    pub time: f32,
    /// Lens size.
    pub lens_size: f32,
    /// Force focus.
    pub force_focus: bool,
    /// Focus z.
    pub focus_z: f32,
}
impl DepthKey {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            time: r.f32()?,
            lens_size: r.f32()?,
            force_focus: r.bool()?,
            focus_z: r.f32()?,
        })
    }
}
/// CGameCtnMediaBlockFxBlurDepth (`0x03081000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockFxBlurDepth {
    /// Keys.
    pub keys: Option<Vec<DepthKey>>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockFxBlurDepth {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            keys: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockFxBlurDepth", |r, id| {
            match id {
                0x03081001 => n.keys = Some(r.list(16, DepthKey::read)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// BloomKey data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct BloomKey {
    /// Time.
    pub time: f32,
    /// Intensity.
    pub intensity: f32,
    /// Sensitivity.
    pub sensitivity: f32,
}
impl BloomKey {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            time: r.f32()?,
            intensity: r.f32()?,
            sensitivity: r.f32()?,
        })
    }
}
/// CGameCtnMediaBlockFxBloom (`0x03083000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockFxBloom {
    /// Keys.
    pub keys: Option<Vec<BloomKey>>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockFxBloom {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            keys: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockFxBloom", |r, id| {
            match id {
                0x03083001 => n.keys = Some(r.list(12, BloomKey::read)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CGameCtnMediaBlockTrails (`0x030A9000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockTrails {
    /// Start.
    pub start: Option<f32>,
    /// End.
    pub end: Option<f32>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockTrails {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            start: None,
            end: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockTrails", |r, id| {
            match id {
                0x030A9000 => {
                    n.start = Some(r.f32()?);
                    n.end = Some(r.f32()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CGameCtnMediaBlockFxBlurMotion (`0x03082000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockFxBlurMotion {
    /// Start.
    pub start: Option<f32>,
    /// End.
    pub end: Option<f32>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockFxBlurMotion {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            start: None,
            end: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockFxBlurMotion", |r, id| {
            match id {
                0x03082000 => {
                    n.start = Some(r.f32()?);
                    n.end = Some(r.f32()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CameraPathKey data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraPathKey {
    /// Time.
    pub time: f32,
    /// Position.
    pub position: [f32; 3],
    /// Pitch yaw roll.
    pub pitch_yaw_roll: [f32; 3],
    /// Fov.
    pub fov: f32,
    /// Anchor rot.
    pub anchor_rot: bool,
    /// Anchor.
    pub anchor: i32,
    /// Anchor vis.
    pub anchor_vis: bool,
    /// Target.
    pub target: i32,
    /// Target position.
    pub target_position: [f32; 3],
    /// Weight.
    pub weight: f32,
    /// Rotation.
    pub rotation: [f32; 4],
}
impl CameraPathKey {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            time: r.f32()?,
            position: r.vec3()?,
            pitch_yaw_roll: r.vec3()?,
            fov: r.f32()?,
            anchor_rot: r.bool()?,
            anchor: r.i32()?,
            anchor_vis: r.bool()?,
            target: r.i32()?,
            target_position: r.vec3()?,
            weight: r.f32()?,
            rotation: r.vec4()?,
        })
    }
}
/// CGameCtnMediaBlockCameraPath (`0x030a1000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockCameraPath {
    /// Keys.
    pub keys: Option<Vec<CameraPathKey>>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockCameraPath {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            keys: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockCameraPath", |r, id| {
            match id {
                0x030A1002 => n.keys = Some(r.list(80, CameraPathKey::read)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// ShakeKey data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct ShakeKey {
    /// Time.
    pub time: f32,
    /// Intensity.
    pub intensity: f32,
    /// Speed.
    pub speed: f32,
}
impl ShakeKey {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            time: r.f32()?,
            intensity: r.f32()?,
            speed: r.f32()?,
        })
    }
}
/// CGameCtnMediaBlockCameraEffectShake (`0x030a4000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockCameraEffectShake {
    /// Keys.
    pub keys: Option<Vec<ShakeKey>>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockCameraEffectShake {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            keys: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockCameraEffectShake", |r, id| {
            match id {
                0x030A4000 => n.keys = Some(r.list(12, ShakeKey::read)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// SimiKey data, kept as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct SimiKey {
    /// Time.
    pub time: f32,
    /// Position.
    pub position: [f32; 2],
    /// Rotation.
    pub rotation: f32,
    /// Scale.
    pub scale: [f32; 2],
    /// Opacity.
    pub opacity: f32,
    /// Depth.
    pub depth: f32,
    /// U01.
    pub u01: f32,
    /// U02.
    pub u02: f32,
    /// U03.
    pub u03: f32,
    /// U04.
    pub u04: f32,
}
impl SimiKey {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            time: r.f32()?,
            position: r.vec2()?,
            rotation: r.f32()?,
            scale: r.vec2()?,
            opacity: r.f32()?,
            depth: r.f32()?,
            u01: r.f32()?,
            u02: r.f32()?,
            u03: r.f32()?,
            u04: r.f32()?,
        })
    }
}
/// CControlEffectSimi (`0x07010000`).
#[derive(Clone, Debug, PartialEq)]
pub struct ControlEffectSimi {
    /// Keys.
    pub keys: Option<Vec<SimiKey>>,
    /// Centered.
    pub centered: Option<bool>,
    /// Color blend mode.
    pub color_blend_mode: Option<i32>,
    /// Is continuous effect.
    pub is_continuous_effect: Option<bool>,
    /// Is interpolated.
    pub is_interpolated: Option<bool>,
    /// Chunks.
    pub chunks: Chunks,
}
impl ControlEffectSimi {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            keys: None,
            centered: None,
            color_blend_mode: None,
            is_continuous_effect: None,
            is_interpolated: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CControlEffectSimi", |r, id| {
            match id {
                0x07010005 => {
                    n.keys = Some(r.list(48, SimiKey::read)?);
                    n.centered = Some(r.bool()?);
                    n.color_blend_mode = Some(r.i32()?);
                    n.is_continuous_effect = Some(r.bool()?);
                    n.is_interpolated = Some(r.bool()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CFuncKeys (`0x05002000`).
#[derive(Clone, Debug, PartialEq)]
pub struct FuncKeys {
    /// Xs.
    pub xs: Option<Vec<f32>>,
    /// Id.
    pub id: Option<Id>,
    /// Chunks.
    pub chunks: Chunks,
}
impl FuncKeys {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            xs: None,
            id: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CFuncKeys", |r, id| {
            match id {
                0x05002001 => n.xs = Some(r.list(4, |r| r.f32())?),
                0x05002003 => n.id = Some(r.id()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// CFuncKeysReal (`0x0501a000`).
#[derive(Clone, Debug, PartialEq)]
pub struct FuncKeysReal {
    /// Xs.
    pub xs: Option<Vec<f32>>,
    /// Id.
    pub id: Option<Id>,
    /// Ys.
    pub ys: Option<Vec<f32>>,
    /// Real interp.
    pub real_interp: Option<i32>,
    /// Chunks.
    pub chunks: Chunks,
}
impl FuncKeysReal {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            xs: None,
            id: None,
            ys: None,
            real_interp: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CFuncKeysReal", |r, id| {
            match id {
                0x05002001 => n.xs = Some(r.list(4, |r| r.f32())?),
                0x05002003 => n.id = Some(r.id()?),
                0x0501A001 => {
                    n.ys = Some(r.list(4, |r| r.f32())?);
                    n.real_interp = Some(r.i32()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// A music effect key: the music's volume (and the sounds') from this time.
#[derive(Clone, Debug, PartialEq)]
pub struct MusicEffectKey {
    /// Time.
    pub time: f32,
    /// Music volume.
    pub music_volume: f32,
    /// Sound volume.
    pub sound_volume: f32,
}
/// CGameCtnMediaBlockMusicEffect (`0x030A6000`).
#[derive(Clone, Debug, PartialEq)]
pub struct MediaBlockMusicEffect {
    /// Keys.
    pub keys: Option<Vec<MusicEffectKey>>,
    /// Chunks.
    pub chunks: Chunks,
}
impl MediaBlockMusicEffect {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self {
            keys: None,
            chunks: Chunks::default(),
        };
        n.chunks = r.chunks("CGameCtnMediaBlockMusicEffect", |r, id| {
            match id {
                0x030A6001 => {
                    n.keys = Some(r.list(12, |r| {
                        Ok(MusicEffectKey {
                            time: r.f32()?,
                            music_volume: r.f32()?,
                            sound_volume: r.f32()?,
                        })
                    })?)
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
