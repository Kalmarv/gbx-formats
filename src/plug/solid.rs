//! Solids: `CPlugSolid`, the model a block, a vehicle or a scenery item is made of: a tree ([`super::tree`]) with
//! the physical parameters of the whole.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::Archive;
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};

/// The mass and friction of a solid (chunk `0x0900500E`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SolidPhysics {
    /// The mass.
    pub mass: f32,
    /// The center of mass, in the solid's space.
    pub center_of_mass: [f32; 3],
    /// The inverse inertia matrix, nine floats in the order stored, row by row.
    pub inverse_inertia: [f32; 9],
    /// The linear fluid friction.
    pub linear_fluid_friction: f32,
    /// The angular fluid friction.
    pub angular_fluid_friction: f32,
    /// The farthest the solid may move in one physics step.
    pub max_dist_per_step: f32,
}

/// What a solid is made of (chunk `0x09005011`): another solid, or a tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolidContent {
    /// A model: another solid (`CPlugSolid`), a node of the file or the file it names.
    Model {
        /// Whether the model is a file the solid names (`ModelIsExternal`).
        external: bool,
        /// The model.
        model: Option<NodeIndex>,
    },
    /// A tree (`CPlugTree`): the solid's own hierarchy.
    Tree(Option<NodeIndex>),
}

/// A solid (`CPlugSolid`, class `0x09005000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solid {
    /// Chunk `0x09005000`: the counter of the plug ids the solid's nodes were given.
    pub plug_id_counter: Option<i32>,
    /// Chunk `0x0900500E`: mass, inertia and friction.
    pub physics: Option<SolidPhysics>,
    /// Chunk `0x09005010`: a node of unknown use (`CSceneVehicleEnvironment` in GBX.NET).
    pub vehicle_environment: Option<Option<NodeIndex>>,
    /// Chunk `0x09005011`: `UseModel`, a bool of unknown meaning.
    pub use_model: Option<bool>,
    /// Chunk `0x09005011`: the content.
    pub content: Option<SolidContent>,
    /// Chunk `0x09005012`: the tile count of the light map's U axis.
    pub tile_count_u: Option<u8>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl Solid {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Solid> {
        let mut n = Solid::default();
        n.chunks = r.chunks("CPlugSolid", |r, id| {
            match id {
                0x0900_5000 => n.plug_id_counter = Some(r.i32()?),
                0x0900_500E => {
                    let mass = r.f32()?;
                    let center_of_mass = r.vec3()?;
                    let mut inverse_inertia = [0.0; 9];
                    for v in &mut inverse_inertia {
                        *v = r.f32()?;
                    }
                    n.physics = Some(SolidPhysics {
                        mass,
                        center_of_mass,
                        inverse_inertia,
                        linear_fluid_friction: r.f32()?,
                        angular_fluid_friction: r.f32()?,
                        max_dist_per_step: r.f32()?,
                    });
                }
                0x0900_5010 => n.vehicle_environment = Some(r.node_ref()?),
                0x0900_5011 => {
                    n.use_model = Some(r.bool()?);
                    n.content = Some(if r.bool()? {
                        SolidContent::Model {
                            external: r.bool()?,
                            model: r.node_ref()?,
                        }
                    } else {
                        SolidContent::Tree(r.node_ref()?)
                    });
                }
                0x0900_5012 => n.tile_count_u = Some(r.u8()?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
