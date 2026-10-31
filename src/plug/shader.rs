//! Shader requirements, passes, GPU constants and texture bindings.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};
/// ShaderRequirements fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderRequirements {
    /// Flags.
    pub flags: u64,
    /// Lightmap quality.
    pub lightmap_quality: f32,
    /// Material fx.
    pub material_fx: Option<NodeIndex>,
}
impl ShaderRequirements {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            flags: r.u64()?,
            lightmap_quality: r.f32()?,
            material_fx: r.node_ref()?,
        })
    }
}
/// ShaderMaterial fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderMaterial {
    /// Ambient.
    pub ambient: [f32; 3],
    /// Diffuse.
    pub diffuse: [f32; 3],
    /// Diffuse alpha.
    pub diffuse_alpha: f32,
    /// Specular.
    pub specular: [f32; 3],
    /// Specular exponent.
    pub specular_exponent: f32,
    /// Emissive.
    pub emissive: [f32; 3],
    /// Emissive modifier.
    pub emissive_modifier: [f32; 3],
    /// Wire.
    pub wire: [f32; 3],
    /// Wire alpha.
    pub wire_alpha: f32,
    /// Flags.
    pub flags: u32,
}
impl ShaderMaterial {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            ambient: r.vec3()?,
            diffuse: r.vec3()?,
            diffuse_alpha: r.f32()?,
            specular: r.vec3()?,
            specular_exponent: r.f32()?,
            emissive: r.vec3()?,
            emissive_modifier: r.vec3()?,
            wire: r.vec3()?,
            wire_alpha: r.f32()?,
            flags: r.u32()?,
        })
    }
}
/// ShaderApply fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShaderApply {
    /// Function.
    pub function: Option<Option<NodeIndex>>,
    /// Passes.
    pub passes: Option<Vec<Option<NodeIndex>>>,
    /// Solid wired apply.
    pub solid_wired_apply: Option<Option<NodeIndex>>,
    /// Legacy custom shaders.
    pub legacy_custom_shaders: Option<Vec<Option<NodeIndex>>>,
    /// Requirements.
    pub requirements: Option<ShaderRequirements>,
    /// Visible id flags.
    pub visible_id_flags: Option<u16>,
    /// Material.
    pub material: Option<ShaderMaterial>,
    /// Bitmap addresses.
    pub bitmap_addresses: Option<Vec<Option<NodeIndex>>>,
    /// Texop constant.
    pub texop_constant: Option<u32>,
    /// Apply field.
    pub apply_field: Option<u32>,
    /// Apply field2.
    pub apply_field2: Option<u32>,
    /// Chunks.
    pub chunks: Chunks,
}
impl ShaderApply {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugShaderApply", |r, id| {
            match id {
                0x0900200E => {
                    n.function = Some(r.node_ref()?);
                    n.passes = Some(r.list(4, |r| r.node_ref())?);
                    n.solid_wired_apply = Some(r.node_ref()?);
                    n.legacy_custom_shaders = Some(r.list(4, |r| r.node_ref())?);
                }
                0x09002016 => {
                    n.requirements = Some(ShaderRequirements::read(r)?);
                    n.visible_id_flags = Some(r.u16()?);
                }
                0x09004003 => n.material = Some(ShaderMaterial::read(r)?),
                0x09026002 => n.bitmap_addresses = Some(r.list(4, |r| r.node_ref())?),
                0x09026004 => n.texop_constant = Some(r.u32()?),
                0x09026008 => {
                    n.apply_field = Some(r.u32()?);
                    n.apply_field2 = Some(r.u32()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// GpuLoadFx fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuLoadFx {
    /// Name.
    pub name: Id,
    /// Value kind.
    pub value_kind: i32,
    /// Registers per element.
    pub registers_per_element: i32,
    /// Element count.
    pub element_count: i32,
    /// Value index.
    pub value_index: i32,
}
impl GpuLoadFx {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            name: r.id()?,
            value_kind: r.i32()?,
            registers_per_element: r.i32()?,
            element_count: r.i32()?,
            value_index: r.i32()?,
        })
    }
}
/// GpuPipeline fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuPipeline {
    /// Has external file.
    pub has_external_file: bool,
    /// File gpu.
    pub file_gpu: Option<NodeIndex>,
    /// Load fxs.
    pub load_fxs: Option<Vec<GpuLoadFx>>,
    /// Values.
    pub values: Option<Vec<[f32; 4]>>,
}
impl GpuPipeline {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let has_external_file = r.bool()?;
        let file_gpu = r.node_ref()?;
        let (load_fxs, values) = if file_gpu.is_some() {
            (
                Some(r.list(20, GpuLoadFx::read)?),
                Some(r.list(16, |r| r.vec4())?),
            )
        } else {
            (None, None)
        };
        Ok(Self {
            has_external_file,
            file_gpu,
            load_fxs,
            values,
        })
    }
}
/// ShaderPass fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShaderPass {
    /// Vertex textures.
    pub vertex_textures: Option<Vec<Option<NodeIndex>>>,
    /// Flags.
    pub flags: Option<u32>,
    /// Skip samplers.
    pub skip_samplers: Option<Vec<Id>>,
    /// Vertex pipeline.
    pub vertex_pipeline: Option<GpuPipeline>,
    /// Pixel pipeline.
    pub pixel_pipeline: Option<GpuPipeline>,
    /// Chunks.
    pub chunks: Chunks,
}
impl ShaderPass {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CPlugShaderPass", |r, id| {
            match id {
                0x09067006 => n.vertex_textures = Some(r.list(4, |r| r.node_ref())?),
                0x09067007 => n.flags = Some(r.u32()?),
                0x0906700A => {
                    n.skip_samplers = Some(r.list(4, |r| r.id())?);
                    n.vertex_pipeline = Some(GpuPipeline::read(r)?);
                    n.pixel_pipeline = Some(GpuPipeline::read(r)?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
