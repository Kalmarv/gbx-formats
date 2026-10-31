//! Shader animation functions.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};
/// Timing and input fields inherited from CFuncPlug chunk `0x0500B005`.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionTiming {
    /// Period, as stored.
    pub period: f32,
    /// Phase, as stored.
    pub phase: f32,
    /// Whether motion is created automatically.
    pub auto_create_motion: bool,
    /// Whether phase is randomized.
    pub randomize_phase: bool,
    /// Input value lookback id.
    pub input_value_id: Id,
}
impl FunctionTiming {
    fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        Ok(Self {
            period: r.f32()?,
            phase: r.f32()?,
            auto_create_motion: r.bool()?,
            randomize_phase: r.bool()?,
            input_value_id: r.id()?,
        })
    }
}
/// FuncShaderLayerUv fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FuncShaderLayerUv {
    /// Layer name.
    pub layer_name: Option<String>,
    /// U01.
    pub u01: Option<i32>,
    /// Uv09.
    pub uv09: Option<[[f32; 2]; 2]>,
    /// Uv0a.
    pub uv0a: Option<[[f32; 2]; 3]>,
    /// Uv0d.
    pub uv0d: Option<[[f32; 2]; 3]>,
    /// Uv13.
    pub uv13: Option<[[f32; 2]; 3]>,
    /// Uv14.
    pub uv14: Option<[f32; 2]>,
    /// U14.
    pub u14: Option<[f32; 2]>,
    /// U15.
    pub u15: Option<[i32; 4]>,
    /// U16.
    pub u16: Option<u32>,
    /// Inherited timing and input.
    pub timing: Option<FunctionTiming>,
    /// Chunks.
    pub chunks: Chunks,
}
impl FuncShaderLayerUv {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CFuncShaderLayerUV", |r, id| {
            match id {
                0x0500B005 => n.timing = Some(FunctionTiming::read(r)?),
                0x05015005 => {
                    n.layer_name = Some(r.string()?);
                    n.u01 = Some(r.i32()?);
                }
                0x05015009 => n.uv09 = Some([r.vec2()?, r.vec2()?]),
                0x0501500A => n.uv0a = Some([r.vec2()?, r.vec2()?, r.vec2()?]),
                0x0501500D => n.uv0d = Some([r.vec2()?, r.vec2()?, r.vec2()?]),
                0x05015013 => n.uv13 = Some([r.vec2()?, r.vec2()?, r.vec2()?]),
                0x05015014 => {
                    n.uv14 = Some(r.vec2()?);
                    n.u14 = Some(r.vec2()?);
                }
                0x05015015 => n.u15 = Some([r.i32()?, r.i32()?, r.i32()?, r.i32()?]),
                0x05015016 => n.u16 = Some(r.u32()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
/// FuncShaders fields as serialized by TrackMania Forever.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FuncShaders {
    /// Functions.
    pub functions: Option<Vec<Option<NodeIndex>>>,
    /// Inherited timing and input.
    pub timing: Option<FunctionTiming>,
    /// Chunks.
    pub chunks: Chunks,
}
impl FuncShaders {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Self> {
        let mut n = Self::default();
        n.chunks = r.chunks("CFuncShaders", |r, id| {
            match id {
                0x0500B005 => n.timing = Some(FunctionTiming::read(r)?),
                0x05014000 => n.functions = Some(r.list(4, |r| r.node_ref())?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
