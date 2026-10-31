//! Materials: `CPlugMaterial` and the `CPlugMaterialCustom` it points to.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};

/// A material (`CPlugMaterial`, class `0x09079000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Material {
    /// Chunk `0x09079001`: a node of unknown use (`CMwNod`).
    pub unknown_node: Option<Option<NodeIndex>>,
    /// Chunk `0x09079007`: the custom material (`CPlugMaterialCustom`): the textures and samplers.
    pub custom_material: Option<Option<NodeIndex>>,
    /// Chunk `0x0907900D`: the shader (`CPlugShader`), a file the material names. A material without one stores
    /// device materials inline, which are not read.
    pub shader: Option<NodeIndex>,
    /// Chunk `0x0907900E`: the surface (physics) material id, a `CPlugSurface.MaterialId`
    /// (see [`super::surface::material_id_name`]).
    pub surface_id: Option<i16>,
    /// Chunk `0x0907900E`: a 16-bit value of unknown meaning stored after the surface id.
    pub u02: Option<i16>,
    /// Chunk `0x0907900F`: an integer of unknown meaning.
    pub u03: Option<i32>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl Material {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Material> {
        let mut n = Material::default();
        n.chunks = r.chunks("CPlugMaterial", |r, id| {
            match id {
                0x0907_9001 => n.unknown_node = Some(r.node_ref()?),
                0x0907_9007 => n.custom_material = Some(r.node_ref()?),
                0x0907_900D => {
                    let at = r.here();
                    n.shader = r.node_ref()?;
                    if n.shader.is_none() {
                        return Err(r.bad_at(at, "a material with inline device materials is not read"));
                    }
                }
                0x0907_900E => {
                    n.surface_id = Some(r.i16()?);
                    n.u02 = Some(r.i16()?);
                }
                0x0907_900F => n.u03 = Some(r.i32()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// A texture slot of a custom material (`CPlugMaterialCustom.Bitmap`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bitmap {
    /// The slot's name (`Diffuse`, `Specular`, `Normal`, `Lighting`, `Occlusion`...).
    pub name: Id,
    /// An integer of unknown meaning.
    pub u01: i32,
    /// The texture (`CPlugBitmap` or another `CMwNod`): usually a file the material names.
    pub texture: Option<NodeIndex>,
}

/// A GPU effect parameter table (`CPlugMaterialCustom.GpuFx`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GpuFx {
    /// The effect's name.
    pub name: Id,
    /// A bool of unknown meaning.
    pub u01: bool,
    /// The number of floats in each row of `values`.
    pub width: u32,
    /// The rows of floats.
    pub values: Vec<Vec<f32>>,
}

/// A sampler a material skips (`CPlugMaterialCustom.BitmapSkip`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkipSampler {
    /// The sampler's name.
    pub name: Id,
    /// A bool of unknown meaning.
    pub skip: bool,
}

/// A custom material (`CPlugMaterialCustom`, class `0x0903A000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MaterialCustom {
    /// Chunk `0x0903A004`: integers of unknown meaning.
    pub u01: Option<Vec<i32>>,
    /// Chunk `0x0903A006`: the texture slots.
    pub textures: Option<Vec<Bitmap>>,
    /// Chunk `0x0903A00A`: the first list of GPU effect tables.
    pub gpu_fxs_1: Option<Vec<GpuFx>>,
    /// Chunk `0x0903A00A`: the second list of GPU effect tables.
    pub gpu_fxs_2: Option<Vec<GpuFx>>,
    /// Chunk `0x0903A00C`: the samplers.
    pub skip_samplers: Option<Vec<SkipSampler>>,
    /// Chunk `0x0903A00D`: two 64-bit values of unknown meaning; bit 0 of the first says the visibility filter is
    /// stored.
    pub u02: Option<(u64, u64)>,
    /// Chunk `0x0903A00D`: the visibility filter's two 16-bit values.
    pub visible_filter: Option<(i16, i16)>,
    /// The node's chunks.
    pub chunks: Chunks,
}

fn gpu_fxs(r: &mut Archive<'_, '_>) -> Result<Vec<GpuFx>> {
    r.list(16, |r| {
        let name = r.id()?;
        let width_at = r.here();
        let width = r.u32()?;
        let rows = r.count(1)?;
        let u01 = r.bool()?;
        let width_n = usize::try_from(width).unwrap_or(usize::MAX);
        let floats = width_n.checked_mul(rows).and_then(|n| n.checked_mul(4));
        if floats.is_none_or(|bytes| bytes > r.remaining()) {
            return Err(r.bad_at(
                width_at,
                format!("{rows} rows of {width} floats do not fit in the chunk"),
            ));
        }
        let values = (0..rows)
            .map(|_| (0..width_n).map(|_| r.f32()).collect::<Result<Vec<_>>>())
            .collect::<Result<Vec<_>>>()?;
        Ok(GpuFx {
            name,
            u01,
            width,
            values,
        })
    })
}

impl MaterialCustom {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<MaterialCustom> {
        let mut n = MaterialCustom::default();
        n.chunks = r.chunks("CPlugMaterialCustom", |r, id| {
            match id {
                0x0903_A004 => n.u01 = Some(r.list(4, |r| r.i32())?),
                0x0903_A006 => {
                    n.textures = Some(r.list(12, |r| {
                        Ok(Bitmap {
                            name: r.id()?,
                            u01: r.i32()?,
                            texture: r.node_ref()?,
                        })
                    })?);
                }
                0x0903_A00A => {
                    n.gpu_fxs_1 = Some(gpu_fxs(r)?);
                    n.gpu_fxs_2 = Some(gpu_fxs(r)?);
                }
                0x0903_A00C => {
                    n.skip_samplers = Some(r.list(8, |r| {
                        Ok(SkipSampler {
                            name: r.id()?,
                            skip: r.bool()?,
                        })
                    })?);
                }
                0x0903_A00D => {
                    let first = r.u64()?;
                    let second = r.u64()?;
                    n.u02 = Some((first, second));
                    if first & 1 != 0 {
                        n.visible_filter = Some((r.i16()?, r.i16()?));
                    }
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
