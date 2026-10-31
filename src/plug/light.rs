//! Plug, scene and Hms lights, including their inherited Gx lighting data.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};
/// LightProperties fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct LightProperties {
    /// Color.
    pub color: [f32; 3],
    /// Flags.
    pub flags: u32,
    /// Intensity.
    pub intensity: f32,
    /// Diffuse intensity.
    pub diffuse_intensity: f32,
    /// Specular intensity.
    pub specular_intensity: f32,
    /// Specular power.
    pub specular_power: f32,
    /// Shadow intensity.
    pub shadow_intensity: f32,
    /// Flare intensity.
    pub flare_intensity: f32,
    /// Shadow rgb.
    pub shadow_rgb: [f32; 3],
}
impl LightProperties {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            color: r.vec3()?,
            flags: r.u32()?,
            intensity: r.f32()?,
            diffuse_intensity: r.f32()?,
            specular_intensity: r.f32()?,
            specular_power: r.f32()?,
            shadow_intensity: r.f32()?,
            flare_intensity: r.f32()?,
            shadow_rgb: r.vec3()?,
        })
    }
}
/// PointProperties fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct PointProperties {
    /// Flare size.
    pub flare_size: f32,
    /// Flare bias z.
    pub flare_bias_z: f32,
}
impl PointProperties {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            flare_size: r.f32()?,
            flare_bias_z: r.f32()?,
        })
    }
}
/// BallProperties fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct BallProperties {
    /// Flags.
    pub flags: u32,
    /// Radius.
    pub radius: f32,
    /// Radius specular.
    pub radius_specular: f32,
    /// Radius shadow.
    pub radius_shadow: f32,
    /// Radius flare.
    pub radius_flare: f32,
    /// Emitting radius.
    pub emitting_radius: f32,
    /// Attenuation1.
    pub attenuation1: f32,
    /// Attenuation2.
    pub attenuation2: f32,
    /// Ambient rgb.
    pub ambient_rgb: [f32; 3],
}
impl BallProperties {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            flags: r.u32()?,
            radius: r.f32()?,
            radius_specular: r.f32()?,
            radius_shadow: r.f32()?,
            radius_flare: r.f32()?,
            emitting_radius: r.f32()?,
            attenuation1: r.f32()?,
            attenuation2: r.f32()?,
            ambient_rgb: r.vec3()?,
        })
    }
}
/// SpotAngles fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct SpotAngles {
    /// Inner.
    pub inner: f32,
    /// Outer.
    pub outer: f32,
    /// Flare.
    pub flare: f32,
    /// Falloff exponent.
    pub falloff_exponent: f32,
}
impl SpotAngles {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            inner: r.f32()?,
            outer: r.f32()?,
            flare: r.f32()?,
            falloff_exponent: r.f32()?,
        })
    }
}
/// SpotProperties fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct SpotProperties {
    /// Flags.
    pub flags: u32,
    /// Inner.
    pub inner: f32,
    /// Outer.
    pub outer: f32,
    /// Flare.
    pub flare: f32,
    /// Inner shadow.
    pub inner_shadow: f32,
    /// Outer shadow.
    pub outer_shadow: f32,
    /// Falloff exponent.
    pub falloff_exponent: f32,
}
impl SpotProperties {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            flags: r.u32()?,
            inner: r.f32()?,
            outer: r.f32()?,
            flare: r.f32()?,
            inner_shadow: r.f32()?,
            outer_shadow: r.f32()?,
            falloff_exponent: r.f32()?,
        })
    }
}
/// Fields inherited directly from GxLight.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GxLightData {
    /// Common lighting properties, chunk `0x04001009`.
    pub common: Option<LightProperties>,
}
impl GxLightData {
    fn chunk(&mut self, r: &mut Archive<'_, '_>, id: u32) -> Result<bool> {
        if id != 0x04001009 {
            return Ok(false);
        }
        self.common = Some(LightProperties::read(r)?);
        Ok(true)
    }
}
/// Fields inherited by GxLightBall and its spot/frustum derivatives.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BallLightData {
    /// Common GxLight fields.
    pub base: GxLightData,
    /// Point-light flare properties, chunk `0x04003004`.
    pub point: Option<PointProperties>,
    /// Ball-light attenuation properties, chunk `0x04002006`.
    pub ball: Option<BallProperties>,
}
impl BallLightData {
    fn chunk(&mut self, r: &mut Archive<'_, '_>, id: u32) -> Result<bool> {
        if self.base.chunk(r, id)? {
            return Ok(true);
        }
        match id {
            0x04003004 => self.point = Some(PointProperties::read(r)?),
            0x04002006 => self.ball = Some(BallProperties::read(r)?),
            _ => return Ok(false),
        }
        Ok(true)
    }
}
/// LightBall fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LightBall {
    /// Base.
    pub base: BallLightData,
    /// Chunks.
    pub chunks: Chunks,
}
impl LightBall {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("GxLightBall", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            Ok(false)
        })?;
        Ok(n)
    }
}
/// LightSpot fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LightSpot {
    /// Legacy angles.
    pub legacy_angles: Option<SpotAngles>,
    /// Spot.
    pub spot: Option<SpotProperties>,
    /// Base.
    pub base: BallLightData,
    /// Chunks.
    pub chunks: Chunks,
}
impl LightSpot {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("GxLightSpot", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x0400B001 => n.legacy_angles = Some(SpotAngles::read(r)?),
                0x0400B002 => n.spot = Some(SpotProperties::read(r)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// FrustumProperties fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct FrustumProperties {
    /// Is orthographic.
    pub is_orthographic: bool,
    /// X.
    pub x: f32,
    /// Y.
    pub y: f32,
    /// Z.
    pub z: f32,
    /// X2.
    pub x2: f32,
    /// Y2.
    pub y2: f32,
    /// Z2.
    pub z2: f32,
    /// Flags.
    pub flags: u32,
}
impl FrustumProperties {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            is_orthographic: r.bool()?,
            x: r.f32()?,
            y: r.f32()?,
            z: r.f32()?,
            x2: r.f32()?,
            y2: r.f32()?,
            z2: r.f32()?,
            flags: r.u32()?,
        })
    }
}
/// LightFrustum fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LightFrustum {
    /// Frustum.
    pub frustum: Option<FrustumProperties>,
    /// Base.
    pub base: BallLightData,
    /// Chunks.
    pub chunks: Chunks,
}
impl LightFrustum {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("GxLightFrustum", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x0400A006 => n.frustum = Some(FrustumProperties::read(r)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// DirectionalBoundary fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct DirectionalBoundary {
    /// Use hint.
    pub use_hint: bool,
    /// Hint position.
    pub hint_position: [f32; 3],
    /// Dazzle angle max.
    pub dazzle_angle_max: f32,
    /// Dazzle intensity.
    pub dazzle_intensity: f32,
}
impl DirectionalBoundary {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            use_hint: r.bool()?,
            hint_position: r.vec3()?,
            dazzle_angle_max: r.f32()?,
            dazzle_intensity: r.f32()?,
        })
    }
}
/// LightDirectional fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LightDirectional {
    /// Boundary.
    pub boundary: Option<DirectionalBoundary>,
    /// Double sided rgb.
    pub double_sided_rgb: Option<[f32; 3]>,
    /// Reverse rgb.
    pub reverse_rgb: Option<[f32; 3]>,
    /// Reverse intensity.
    pub reverse_intensity: Option<f32>,
    /// Emitting angular size.
    pub emitting_angular_size: Option<f32>,
    /// Flare angular size.
    pub flare_angular_size: Option<f32>,
    /// Base.
    pub base: GxLightData,
    /// Chunks.
    pub chunks: Chunks,
}
impl LightDirectional {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("GxLightDirectional", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x04007002 => n.boundary = Some(DirectionalBoundary::read(r)?),
                0x04007003 => n.double_sided_rgb = Some(r.vec3()?),
                0x04007004 => {
                    n.reverse_rgb = Some(r.vec3()?);
                    n.reverse_intensity = Some(r.f32()?);
                }
                0x04007005 => {
                    n.emitting_angular_size = Some(r.f32()?);
                    n.flare_angular_size = Some(r.f32()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// Light fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Light {
    /// Light.
    pub light: Option<Option<NodeIndex>>,
    /// Function.
    pub function: Option<Option<NodeIndex>>,
    /// Bitmap flare.
    pub bitmap_flare: Option<Option<NodeIndex>>,
    /// Bitmap projector.
    pub bitmap_projector: Option<Option<NodeIndex>>,
    /// Flags.
    pub flags: Option<u32>,
    /// Chunks.
    pub chunks: Chunks,
}
impl Light {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugLight", |r, id| {
            match id {
                0x0901D002 => {
                    n.light = Some(r.node_ref()?);
                    n.function = Some(r.node_ref()?);
                    n.bitmap_flare = Some(r.node_ref()?);
                    n.bitmap_projector = Some(r.node_ref()?);
                    n.flags = Some(r.u32()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// HmsLight fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HmsLight {
    /// U01.
    pub u01: Option<i32>,
    /// Main light.
    pub main_light: Option<Option<NodeIndex>>,
    /// Bitmap flare.
    pub bitmap_flare: Option<Option<NodeIndex>>,
    /// Bitmap sprite.
    pub bitmap_sprite: Option<Option<NodeIndex>>,
    /// Chunks.
    pub chunks: Chunks,
}
impl HmsLight {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CHmsLight", |r, id| {
            match id {
                0x0600C003 => {
                    n.u01 = Some(r.i32()?);
                    n.main_light = Some(r.node_ref()?);
                    n.bitmap_flare = Some(r.node_ref()?);
                    n.bitmap_sprite = Some(r.node_ref()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// SceneLight fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneLight {
    /// Name.
    pub name: Option<Id>,
    /// Motion.
    pub motion: Option<Option<NodeIndex>>,
    /// Is active.
    pub is_active: Option<bool>,
    /// Light.
    pub light: Option<HmsLight>,
    /// Chunks.
    pub chunks: Chunks,
}
impl SceneLight {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CSceneLight", |r, id| {
            match id {
                0x0A005001 => n.name = Some(r.id()?),
                0x0A005003 => n.motion = Some(r.node_ref()?),
                0x0A009000 => n.is_active = Some(r.bool()?),
                0x0A00B000 => {
                    r.dummy_write(0x0600B000u32.to_le_bytes());
                    n.light = Some(HmsLight::read(r)?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// HmsLightMapMood fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HmsLightMapMood {
    /// U01.
    pub u01: Option<[f32; 2]>,
    /// Ambient range.
    pub ambient_range: Option<[f32; 2]>,
    /// Directional scale.
    pub directional_scale: Option<f32>,
    /// Chunks.
    pub chunks: Chunks,
}
impl HmsLightMapMood {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CHmsLightMapMood", |r, id| {
            match id {
                0x06023000 => {
                    n.u01 = Some(r.vec2()?);
                    n.ambient_range = Some(r.vec2()?);
                    n.directional_scale = Some(r.f32()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// HmsLightMap fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HmsLightMap {
    /// Points in sphere.
    pub points_in_sphere: Option<Option<NodeIndex>>,
    /// U01.
    pub u01: Option<Option<NodeIndex>>,
    /// Mood.
    pub mood: Option<Option<NodeIndex>>,
    /// Legacy nodes.
    pub legacy_nodes: Option<[Option<NodeIndex>; 15]>,
    /// Chunks.
    pub chunks: Chunks,
}
impl HmsLightMap {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CHmsLightMap", |r, id| {
            match id {
                0x06021001 => n.points_in_sphere = Some(r.node_ref()?),
                0x06021002 => n.u01 = Some(r.node_ref()?),
                0x06021003 => n.mood = Some(r.node_ref()?),
                0x06021004 => {
                    let mut nodes = [None; 15];
                    for node in &mut nodes {
                        *node = r.node_ref()?;
                    }
                    n.legacy_nodes = Some(nodes);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
