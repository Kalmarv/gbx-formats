//! The Plug classes (engine `0x09`) that TMUnlimiter embeds in a map and the game's block packs hold: a block model
//! is a tree ([`tree::Tree`]) whose leaves carry indexed triangle meshes ([`visual::VisualIndexedTriangles`]), a
//! material ([`material::Material`]) and a collision surface ([`surface::Surface`]).
//!
//! Each class here is a node of its own class id, except that a class's base classes are read as part of it: a
//! node of class `CPlugVisualIndexedTriangles` stores the chunks of `CPlugVisual`, `CPlugVisual3D` and
//! `CPlugVisualIndexed` ahead of its own, and the node carries one struct per base ([`visual`]).
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

pub mod material;
pub mod solid;
pub mod surface;
pub mod tree;
pub mod visual;

use crate::archive::Archive;
use crate::error::Result;

/// A transform: a 3x3 matrix and a translation (`Iso4` in GBX.NET).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Iso4 {
    /// The nine matrix floats in the order stored, row by row (GBX.NET's XX, XY, XZ, YX, YY, YZ, ZX, ZY, ZZ).
    pub matrix: [[f32; 3]; 3],
    /// The translation (GBX.NET's TX, TY, TZ).
    pub translation: [f32; 3],
}

/// An axis-aligned box as stored (`BoxAligned` in GBX.NET): its center and its half-sizes, six floats.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoxAligned {
    /// The center.
    pub center: [f32; 3],
    /// The half-size along each axis: the box spans `center - half_size` to `center + half_size`.
    pub half_size: [f32; 3],
}

pub(crate) fn iso4(r: &mut Archive<'_, '_>) -> Result<Iso4> {
    Ok(Iso4 {
        matrix: [r.vec3()?, r.vec3()?, r.vec3()?],
        translation: r.vec3()?,
    })
}

pub(crate) fn box_aligned(r: &mut Archive<'_, '_>) -> Result<BoxAligned> {
    Ok(BoxAligned {
        center: r.vec3()?,
        half_size: r.vec3()?,
    })
}
