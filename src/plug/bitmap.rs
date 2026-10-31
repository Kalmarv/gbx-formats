//! Texture files, sampling and render-to-texture settings.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};
/// SpecularSubMap fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct SpecularSubMap {
    /// Rgb.
    pub rgb: [f32; 3],
    /// Exponent.
    pub exponent: f32,
}
impl SpecularSubMap {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            rgb: r.vec3()?,
            exponent: r.f32()?,
        })
    }
}
/// BitmapImage fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct BitmapImage {
    /// Image source: an external image file (such as a DDS) or an inline procedural generator.
    /// External file names and folders are in the Gbx reference table at this node index.
    pub image: Option<NodeIndex>,
    /// Serialized bitmap flags: usage in bits 0..8 and pixel-update mode in bits 8..16.
    pub flags: u64,
    /// Mipmap lower alpha.
    pub mipmap_lower_alpha: f32,
    /// Bump scale factor.
    pub bump_scale_factor: f32,
    /// Mipmap lod bias.
    pub mipmap_lod_bias: f32,
    /// Border rgb.
    pub border_rgb: u32,
    /// Render.
    pub render: Option<NodeIndex>,
    /// Specular submaps.
    pub specular_submaps: Option<Vec<SpecularSubMap>>,
    /// Clear color.
    pub clear_color: Option<u32>,
}
impl BitmapImage {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let image = r.node_ref()?;
        let flags = r.u64()?;
        let mipmap_lower_alpha = r.f32()?;
        let bump_scale_factor = r.f32()?;
        let mipmap_lod_bias = r.f32()?;
        let border_rgb = r.u32()?;
        let pixel_update = (flags >> 8) & 255;
        let mut render = None;
        let mut specular_submaps = None;
        let mut clear_color = None;
        match pixel_update {
            4 => render = r.node_ref()?,
            5 => return Err(r.bad("bitmap shader pixel-update mode 5 is not read")),
            7 => specular_submaps = Some(r.list(16, SpecularSubMap::read)?),
            8 => clear_color = Some(r.u32()?),
            _ => {}
        }
        Ok(Self {
            image,
            flags,
            mipmap_lower_alpha,
            bump_scale_factor,
            mipmap_lod_bias,
            border_rgb,
            render,
            specular_submaps,
            clear_color,
        })
    }
}
/// TextureBitmap fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextureBitmap {
    /// Image.
    pub image: Option<BitmapImage>,
    /// Bump scale mip level.
    pub bump_scale_mip_level: Option<f32>,
    /// Texcoord scale.
    pub texcoord_scale: Option<[f32; 2]>,
    /// Texcoord translation.
    pub texcoord_translation: Option<[f32; 2]>,
    /// Texcoord rotation.
    pub texcoord_rotation: Option<f32>,
    /// Atlas counts.
    pub atlas_counts: Option<Vec<[u16; 2]>>,
    /// Mipmap fade alphas.
    pub mipmap_fade_alphas: Option<Vec<f32>>,
    /// Flags2.
    pub flags2: Option<u32>,
    /// Chunks.
    pub chunks: Chunks,
}
impl TextureBitmap {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugBitmap", |r, id| {
            match id {
                0x09011022 => n.image = Some(BitmapImage::read(r)?),
                0x09011019 => n.bump_scale_mip_level = Some(r.f32()?),
                0x0901101C => {
                    n.texcoord_scale = Some(r.vec2()?);
                    n.texcoord_translation = Some(r.vec2()?);
                    n.texcoord_rotation = Some(r.f32()?);
                }
                0x0901101E => n.atlas_counts = Some(r.list(4, |r| Ok([r.u16()?, r.u16()?]))?),
                0x09011020 => n.mipmap_fade_alphas = Some(r.list(4, |r| r.f32())?),
                0x09011024 => n.flags2 = Some(r.u32()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// SamplerState fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct SamplerState {
    /// Name.
    pub name: Id,
    /// Bitmap.
    pub bitmap: Option<NodeIndex>,
    /// Serialized packed filtering and U/V/W address state; retained before legacy conversion.
    pub legacy_flags: u32,
    /// Mipmap lod bias.
    pub mipmap_lod_bias: f32,
    /// Border color.
    pub border_color: Option<u32>,
}
impl SamplerState {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let name = r.id()?;
        let bitmap = r.node_ref()?;
        let legacy_flags = r.u32()?;
        let mipmap_lod_bias = r.f32()?;
        let border_color = if legacy_flags & 0x18 == 0x18
            || legacy_flags & 0x60 == 0x60
            || legacy_flags & 0x1800 == 0x1800
        {
            Some(r.u32()?)
        } else {
            None
        };
        Ok(Self {
            name,
            bitmap,
            legacy_flags,
            mipmap_lod_bias,
            border_color,
        })
    }
}
/// AddressTransform fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct AddressTransform {
    /// Flags.
    pub flags: u32,
    /// Serialized texture-coordinate index, retained even if native UV-generation upgrades would replace it.
    pub texcoord_index: i32,
    /// Transform flags.
    pub transform_flags: u32,
    /// Iso3.
    pub iso3: Option<[[f32; 2]; 3]>,
    /// Matrix.
    pub matrix: Option<[[f32; 4]; 4]>,
}
impl AddressTransform {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let flags = r.u32()?;
        let texcoord_index = r.i32()?;
        let transform_flags = u32::from(r.u8()?);
        let iso3 = if transform_flags & 1 != 0 {
            Some([r.vec2()?, r.vec2()?, r.vec2()?])
        } else {
            None
        };
        let matrix = if transform_flags & 2 != 0 {
            Some([r.vec4()?, r.vec4()?, r.vec4()?, r.vec4()?])
        } else {
            None
        };
        Ok(Self {
            flags,
            texcoord_index,
            transform_flags,
            iso3,
            matrix,
        })
    }
}
/// BitmapApply fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BitmapApply {
    /// Sampler.
    pub sampler: Option<SamplerState>,
    /// Address.
    pub address: Option<AddressTransform>,
    /// Bump env scale.
    pub bump_env_scale: Option<f32>,
    /// Apply flags.
    pub apply_flags: Option<u32>,
    /// Chunks.
    pub chunks: Chunks,
}
impl BitmapApply {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugBitmapApply", |r, id| {
            match id {
                0x0907E008 => n.sampler = Some(SamplerState::read(r)?),
                0x09047007 => n.address = Some(AddressTransform::read(r)?),
                0x09047009 => n.bump_env_scale = Some(r.f32()?),
                0x09012004 => n.apply_flags = Some(r.u32()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// RenderSettings fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderSettings {
    /// Item visibility mask.
    pub item_visibility_mask: Option<u16>,
    /// Item visibility ref.
    pub item_visibility_ref: Option<u16>,
    /// Shader visibility mask.
    pub shader_visibility_mask: Option<u16>,
    /// Shader visibility ref.
    pub shader_visibility_ref: Option<u16>,
    /// Render path.
    pub render_path: Option<i32>,
    /// Render path fails.
    pub render_path_fails: Option<i32>,
    /// Fog far z.
    pub fog_far_z: Option<f32>,
    /// Clear color.
    pub clear_color: Option<u32>,
    /// Render flags.
    pub render_flags: Option<u32>,
    /// Bitmap clear.
    pub bitmap_clear: Option<Option<NodeIndex>>,
    /// Bitmap clear uv.
    pub bitmap_clear_uv: Option<[f32; 2]>,
    /// Render sub.
    pub render_sub: Option<Option<NodeIndex>>,
    /// Render flags2.
    pub render_flags2: Option<u32>,
}
impl RenderSettings {
    fn chunk(&mut self, r: &mut Archive<'_, '_>, id: u32) -> Result<bool> {
        match id {
            0x09086003 => {
                self.item_visibility_mask = Some(r.u16()?);
                self.item_visibility_ref = Some(r.u16()?);
                self.shader_visibility_mask = Some(r.u16()?);
                self.shader_visibility_ref = Some(r.u16()?);
            }
            0x09086008 => {
                let (a, b) = r.skippable(|r| Ok((r.i32()?, r.i32()?)))?;
                self.render_path = Some(a);
                self.render_path_fails = Some(b);
            }
            0x0908600A => self.fog_far_z = Some(r.f32()?),
            0x0908600B => {
                self.clear_color = Some(r.u32()?);
                self.render_flags = Some(r.u32()?);
                self.bitmap_clear = Some(r.node_ref()?);
                self.bitmap_clear_uv = Some(r.vec2()?);
            }
            0x0908600C => self.render_sub = Some(r.node_ref()?),
            0x0908600E => self.render_flags2 = Some(r.u32()?),
            _ => return Ok(false),
        }
        Ok(true)
    }
}
/// WaterRender fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterRender {
    /// Is orthographic.
    pub is_orthographic: bool,
    /// Frustum.
    pub frustum: super::BoxAligned,
    /// Transform.
    pub transform: super::Iso4,
    /// Flags.
    pub flags: u32,
    /// Clip plane height.
    pub clip_plane_height: f32,
    /// Margin.
    pub margin: f32,
    /// Mirror scale y.
    pub mirror_scale_y: f32,
}
impl WaterRender {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            is_orthographic: r.bool()?,
            frustum: super::box_aligned(r)?,
            transform: super::iso4(r)?,
            flags: r.u32()?,
            clip_plane_height: r.f32()?,
            margin: r.f32()?,
            mirror_scale_y: r.f32()?,
        })
    }
}
/// BitmapRenderWater fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BitmapRenderWater {
    /// Water.
    pub water: Option<WaterRender>,
    /// Fog max depth.
    pub fog_max_depth: Option<f32>,
    /// Base.
    pub base: RenderSettings,
    /// Chunks.
    pub chunks: Chunks,
}
impl BitmapRenderWater {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugBitmapRenderWater", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x09087005 => n.water = Some(r.skippable(WaterRender::read)?),
                0x09087006 => n.fog_max_depth = Some(r.skippable(|r| r.f32())?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// BitmapRenderOverlay fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BitmapRenderOverlay {
    /// Is overlay self blur.
    pub is_overlay_self_blur: Option<bool>,
    /// Base.
    pub base: RenderSettings,
    /// Chunks.
    pub chunks: Chunks,
}
impl BitmapRenderOverlay {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugBitmapRenderOverlay", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x0909E000 => n.is_overlay_self_blur = Some(r.bool()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// BitmapRenderLightOcc fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BitmapRenderLightOcc {
    /// Fov y.
    pub fov_y: Option<f32>,
    /// Opacity.
    pub opacity: Option<f32>,
    /// Bitmap to modulate.
    pub bitmap_to_modulate: Option<Option<NodeIndex>>,
    /// Base.
    pub base: RenderSettings,
    /// Chunks.
    pub chunks: Chunks,
}
impl BitmapRenderLightOcc {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugBitmapRenderLightOcc", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x0909F000 => {
                    n.fov_y = Some(r.f32()?);
                    n.opacity = Some(r.f32()?);
                    n.bitmap_to_modulate = Some(r.node_ref()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// BitmapRenderCamera fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BitmapRenderCamera {
    /// U01.
    pub u01: Option<u32>,
    /// Base.
    pub base: RenderSettings,
    /// Chunks.
    pub chunks: Chunks,
}
impl BitmapRenderCamera {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugBitmapRenderCamera", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x09089001 => n.u01 = Some(r.skippable(|r| r.u32())?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// BitmapRenderSub fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BitmapRenderSub {
    /// Shader to force.
    pub shader_to_force: Option<Option<NodeIndex>>,
    /// Base.
    pub base: RenderSettings,
    /// Chunks.
    pub chunks: Chunks,
}
impl BitmapRenderSub {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugBitmapRenderSub", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x09091000 => n.shader_to_force = Some(r.node_ref()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// HemisphereSpecular fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct HemisphereSpecular {
    /// Layout.
    pub layout: i32,
    /// Power0.
    pub power0: f32,
    /// Power1.
    pub power1: f32,
    /// U01.
    pub u01: f32,
    /// U02.
    pub u02: f32,
    /// U03.
    pub u03: f32,
    /// U04.
    pub u04: f32,
    /// U05.
    pub u05: f32,
    /// U06.
    pub u06: f32,
}
impl HemisphereSpecular {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            layout: r.i32()?,
            power0: r.f32()?,
            power1: r.f32()?,
            u01: r.f32()?,
            u02: r.f32()?,
            u03: r.f32()?,
            u04: r.f32()?,
            u05: r.f32()?,
            u06: r.f32()?,
        })
    }
}
/// BitmapRenderHemisphere fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BitmapRenderHemisphere {
    /// Specular.
    pub specular: Option<HemisphereSpecular>,
    /// Base.
    pub base: RenderSettings,
    /// Chunks.
    pub chunks: Chunks,
}
impl BitmapRenderHemisphere {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugBitmapRenderHemisphere", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x09058001 => n.specular = Some(HemisphereSpecular::read(r)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// LightFromMapSettings fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct LightFromMapSettings {
    /// Object count min.
    pub object_count_min: i32,
    /// Object count max.
    pub object_count_max: i32,
    /// Camera near z factor.
    pub camera_near_z_factor: f32,
    /// Camera far z to add.
    pub camera_far_z_to_add: f32,
    /// Remap min night.
    pub remap_min_night: f32,
    /// Remap max night.
    pub remap_max_night: f32,
    /// Remap min day ambient.
    pub remap_min_day_ambient: f32,
    /// Remap max day ambient.
    pub remap_max_day_ambient: f32,
    /// Remap min day directional.
    pub remap_min_day_directional: f32,
    /// Remap max day directional.
    pub remap_max_day_directional: f32,
}
impl LightFromMapSettings {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            object_count_min: r.i32()?,
            object_count_max: r.i32()?,
            camera_near_z_factor: r.f32()?,
            camera_far_z_to_add: r.f32()?,
            remap_min_night: r.f32()?,
            remap_max_night: r.f32()?,
            remap_min_day_ambient: r.f32()?,
            remap_max_day_ambient: r.f32()?,
            remap_min_day_directional: r.f32()?,
            remap_max_day_directional: r.f32()?,
        })
    }
}
/// BitmapRenderLightFromMap fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BitmapRenderLightFromMap {
    /// Settings.
    pub settings: Option<LightFromMapSettings>,
    /// Base.
    pub base: RenderSettings,
    /// Chunks.
    pub chunks: Chunks,
}
impl BitmapRenderLightFromMap {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugBitmapRenderLightFromMap", |r, id| {
            if n.base.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x09021001 => n.settings = Some(LightFromMapSettings::read(r)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A procedural image archive (`CPlugFileGen`), without chunks or an end marker.
#[derive(Clone, Debug, PartialEq)]
pub struct FileGenerator {
    /// Serialized archive version (the high prefix bit marks a versioned archive).
    pub version: u32,
    /// Generator kind, without applying legacy upgrades.
    pub kind: i32,
    /// Integer parameters.
    pub int_params: Vec<i32>,
    /// Color parameters.
    pub color_params: Vec<[f32; 4]>,
    /// Real parameters.
    pub real_params: Vec<f32>,
    /// This archive holds no chunks.
    pub chunks: Chunks,
}
impl FileGenerator {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let prefix = r.version("image generator prefix", &[0x8000_0002])?;
        let kind = r.i32()?;
        if kind == 9 {
            return Err(r.bad("pixel generator data is not read"));
        }
        Ok(Self {
            version: prefix & 0x7FFF_FFFF,
            kind,
            int_params: r.list(4, |r| r.i32())?,
            color_params: r.list(16, |r| r.vec4())?,
            real_params: r.list(4, |r| r.f32())?,
            chunks: Chunks::default(),
        })
    }
}
