//! Collision surfaces: `CPlugSurface` and the `CPlugSurfaceGeom` it points to.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use super::{BoxAligned, box_aligned};
use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};

/// The names of the physics material ids (`CPlugSurface.MaterialId`), by value, as GBX.NET lists them.
const MATERIAL_IDS: &[&str] = &[
    "Concrete",
    "Pavement",
    "Grass",
    "Ice",
    "Metal",
    "Sand",
    "Dirt",
    "Turbo_Deprecated",
    "DirtRoad",
    "Rubber",
    "SlidingRubber",
    "Test",
    "Rock",
    "Water",
    "Wood",
    "Danger",
    "Asphalt",
    "WetDirtRoad",
    "WetAsphalt",
    "WetPavement",
    "WetGrass",
    "Snow",
    "ResonantMetal",
    "GolfBall",
    "GolfWall",
    "GolfGround",
    "Turbo2_Deprecated",
    "Bumper_Deprecated",
    "NotCollidable",
    "FreeWheeling_Deprecated",
    "TurboRoulette_Deprecated",
    "WallJump",
    "MetalTrans",
    "Stone",
    "Player",
    "Trunk",
    "TechLaser",
    "SlidingWood",
    "PlayerOnly",
    "Tech",
    "TechArmor",
    "TechSafe",
    "OffZone",
    "Bullet",
    "TechHook",
    "TechGround",
    "TechWall",
    "TechArrow",
    "TechHook2",
    "Forest",
    "Wheat",
    "TechTarget",
    "PavementStair",
    "TechTeleport",
    "Energy",
    "TechMagnetic",
    "TurboTechMagnetic_Deprecated",
    "Turbo2TechMagnetic_Deprecated",
    "TurboWood_Deprecated",
    "Turbo2Wood_Deprecated",
    "FreeWheelingTechMagnetic_Deprecated",
    "FreeWheelingWood_Deprecated",
    "TechSuperMagnetic",
    "TechNucleus",
    "TechMagneticAccel",
    "MetalFence",
    "TechGravityChange",
    "TechGravityReset",
    "RubberBand",
    "Gravel",
    "Hack_NoGrip_Deprecated",
    "Bumper2_Deprecated",
    "NoSteering_Deprecated",
    "NoBrakes_Deprecated",
    "RoadIce",
    "RoadSynthetic",
    "Green",
    "Plastic",
    "DevDebug",
    "Free3",
    "XXX_Null",
];

/// The name of a physics material id (`CPlugSurface.MaterialId`), as GBX.NET lists them.
pub fn material_id_name(id: i16) -> Option<&'static str> {
    usize::try_from(id)
        .ok()
        .and_then(|i| MATERIAL_IDS.get(i))
        .copied()
}

/// One triangle of a cooked collision mesh (`CPlugSurface.Mesh.CookedTriangle`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CookedTriangle {
    /// Four floats of unknown meaning (a plane: normal and distance).
    pub u01: [f32; 4],
    /// The three vertex indices.
    pub indices: [i32; 3],
    /// The index into the surface's materials.
    pub surface_index: i16,
    /// A byte of unknown meaning.
    pub u04: u8,
    /// A byte of unknown meaning.
    pub u05: u8,
}

/// One cell of a cooked collision mesh's octree (`CPlugSurface.Mesh.OctreeCell`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OctreeCell {
    /// An integer of unknown meaning.
    pub u01: i32,
    /// A vector of unknown meaning (a bound).
    pub u02: [f32; 3],
    /// A vector of unknown meaning (a bound).
    pub u03: [f32; 3],
    /// An integer of unknown meaning.
    pub u04: i32,
}

/// A cooked triangle mesh (`CPlugSurface.Mesh`, version 3).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollisionMesh {
    /// The vertices.
    pub vertices: Vec<[f32; 3]>,
    /// The triangles.
    pub triangles: Vec<CookedTriangle>,
    /// The octree's version (3 in every file read).
    pub octree_version: u32,
    /// The octree's cells.
    pub octree_cells: Vec<OctreeCell>,
}

/// A collision shape (`CPlugSurface.ISurf`), by the type id stored before it. Of the shapes the engine has, only
/// meshes occur in the files read.
#[derive(Clone, Debug, PartialEq)]
pub enum Surf {
    /// Type 7: a triangle mesh.
    Mesh(CollisionMesh),
}

/// A collision geometry (`CPlugSurfaceGeom`, class `0x0900F000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceGeom {
    /// Chunk `0x0900F004`: an id of unknown use.
    pub u01: Option<Id>,
    /// Chunk `0x0900F004`: the bounding box.
    pub bounding_box: Option<BoxAligned>,
    /// Chunk `0x0900F004`: the shape, absent when its type is -1.
    pub surf: Option<Surf>,
    /// Chunk `0x0900F004`: the physics material id (see [`material_id_name`]).
    pub surface_id: Option<i16>,
    /// The node's chunks.
    pub chunks: Chunks,
}

/// A surface's material: a `CPlugMaterial` or a physics material id (`CPlugSurface.SurfMaterial`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfMaterial {
    /// A material node (`CPlugMaterial`).
    Material(Option<NodeIndex>),
    /// A physics material id (see [`material_id_name`]).
    SurfaceId(i16),
}

/// A collision surface (`CPlugSurface`, class `0x0900C000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Surface {
    /// Chunk `0x0900C000`: the geometry (`CPlugSurfaceGeom`).
    pub geom: Option<Option<NodeIndex>>,
    /// Chunk `0x0900C000`: the materials the geometry's triangles index.
    pub materials: Option<Vec<SurfMaterial>>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl Surface {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Surface> {
        let mut n = Surface::default();
        n.chunks = r.chunks("CPlugSurface", |r, id| {
            match id {
                0x0900_C000 => {
                    n.geom = Some(r.node_ref()?);
                    n.materials = Some(r.list(6, |r| {
                        if r.bool()? {
                            Ok(SurfMaterial::Material(r.node_ref()?))
                        } else {
                            Ok(SurfMaterial::SurfaceId(r.i16()?))
                        }
                    })?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

fn mesh(r: &mut Archive<'_, '_>) -> Result<CollisionMesh> {
    r.version("collision mesh", &[3])?;
    let vertices = r.list(12, |r| r.vec3())?;
    let triangles = r.list(32, |r| {
        Ok(CookedTriangle {
            u01: r.vec4()?,
            indices: r.int3()?,
            surface_index: r.i16()?,
            u04: r.u8()?,
            u05: r.u8()?,
        })
    })?;
    let octree_version = r.version("collision octree", &[3])?;
    let octree_cells = r.list(32, |r| {
        Ok(OctreeCell {
            u01: r.i32()?,
            u02: r.vec3()?,
            u03: r.vec3()?,
            u04: r.i32()?,
        })
    })?;
    Ok(CollisionMesh {
        vertices,
        triangles,
        octree_version,
        octree_cells,
    })
}

fn surf(r: &mut Archive<'_, '_>) -> Result<Option<Surf>> {
    let at = r.here();
    Ok(Some(match r.i32()? {
        -1 => return Ok(None),
        7 => Surf::Mesh(mesh(r)?),
        other => {
            return Err(r.bad_at(
                at,
                format!("collision shape type {other}: only meshes (7) are read"),
            ));
        }
    }))
}

impl SurfaceGeom {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<SurfaceGeom> {
        let mut n = SurfaceGeom::default();
        n.chunks = r.chunks("CPlugSurfaceGeom", |r, id| {
            match id {
                0x0900_F004 => {
                    n.u01 = Some(r.id()?);
                    n.bounding_box = Some(box_aligned(r)?);
                    n.surf = surf(r)?;
                    n.surface_id = Some(r.i16()?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
