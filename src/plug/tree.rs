//! Scene trees: `CPlugTree` and `CPlugTreeVisualMip`.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use super::{Iso4, iso4};
use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};

/// Flag bit 2: the tree has a location (a transform) relative to its parent. Chunk `0x0904F01A` stores the
/// location only when it is set.
pub const FLAG_LOCATION: u32 = 1 << 2;

/// What a `CPlugTree` stores, which `CPlugTreeVisualMip` stores too (as its base class).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeData {
    /// Chunk `0x0904F006`: the children, each a `CPlugTree` (or a `CPlugTreeVisualMip`). The chunk stores a
    /// deprecated-list version (`u32`, 10 in every file read) before the count.
    pub children: Option<Vec<Option<NodeIndex>>>,
    /// Chunk `0x0904F00D`: the tree's plug id.
    pub plug_id: Option<Id>,
    /// Chunk `0x0904F00D`: the solid model (`CPlugSolid`); when present, chunk `0x0904F016` stores nothing.
    pub model_solid: Option<Option<NodeIndex>>,
    /// Chunk `0x0904F00D`: the model's plug id, stored only when a solid model is present.
    pub model_plug_id: Option<Id>,
    /// Chunk `0x0904F011`: the function tree (`CFuncTree`).
    pub func_tree: Option<Option<NodeIndex>>,
    /// Chunk `0x0904F016`: the tree's visual, shader and surface, absent when the tree has a solid model.
    pub content: Option<TreeContent>,
    /// Chunk `0x0904F01A`: the flags (bit 2 is [`FLAG_LOCATION`]; GBX.NET names the others: bit 3 visible, bit 7
    /// collidable, bit 14 shadow caster).
    pub flags: Option<u32>,
    /// Chunk `0x0904F01A`: the location, stored when [`FLAG_LOCATION`] is set in the flags.
    pub location: Option<Iso4>,
}

/// What chunk `0x0904F016` stores for a tree without a solid model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeContent {
    /// The visual (`CPlugVisual` of some subclass): the geometry.
    pub visual: Option<NodeIndex>,
    /// The shader: a `CPlugMaterial`, or a `CPlugShader`.
    pub shader: Option<NodeIndex>,
    /// The collision surface (`CPlugSurface`).
    pub surface: Option<NodeIndex>,
    /// The generator (`CPlugTreeGenerator`).
    pub generator: Option<NodeIndex>,
}

impl TreeData {
    /// Reads chunk `id` if it is a `CPlugTree` chunk this crate reads.
    pub(crate) fn chunk(&mut self, r: &mut Archive<'_, '_>, id: u32) -> Result<bool> {
        match id {
            0x0904_F006 => {
                r.version("tree children", &[10])?;
                self.children = Some(r.list(4, |r| r.node_ref())?);
            }
            0x0904_F00D => {
                self.plug_id = Some(r.id()?);
                let solid = r.node_ref()?;
                if solid.is_some() {
                    self.model_plug_id = Some(r.id()?);
                }
                self.model_solid = Some(solid);
            }
            0x0904_F011 => self.func_tree = Some(r.node_ref()?),
            0x0904_F016 => {
                if self.model_solid.is_some_and(|s| s.is_some()) {
                    return Ok(true);
                }
                self.content = Some(TreeContent {
                    visual: r.node_ref()?,
                    shader: r.node_ref()?,
                    surface: r.node_ref()?,
                    generator: r.node_ref()?,
                });
            }
            0x0904_F01A => {
                let flags = r.u32()?;
                if flags & FLAG_LOCATION != 0 {
                    self.location = Some(iso4(r)?);
                }
                self.flags = Some(flags);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// A scene tree (`CPlugTree`, class `0x0904F000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tree {
    /// The tree's fields.
    pub tree: TreeData,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl Tree {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Tree> {
        let mut n = Tree::default();
        n.chunks = r.chunks("CPlugTree", |r, id| n.tree.chunk(r, id))?;
        Ok(n)
    }
}

/// One level of a [`TreeVisualMip`]: a tree shown from the previous level's far distance up to `far_z`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MipLevel {
    /// The distance up to which the level is shown.
    pub far_z: f32,
    /// The level's tree (`CPlugTree`).
    pub tree: Option<NodeIndex>,
}

/// A tree with levels of detail (`CPlugTreeVisualMip`, class `0x09015000`, a subclass of `CPlugTree`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeVisualMip {
    /// The `CPlugTree` base's fields.
    pub tree: TreeData,
    /// Chunk `0x09015002`: the levels.
    pub levels: Option<Vec<MipLevel>>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl TreeVisualMip {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<TreeVisualMip> {
        let mut n = TreeVisualMip::default();
        n.chunks = r.chunks("CPlugTreeVisualMip", |r, id| {
            match id {
                0x0901_5002 => {
                    n.levels = Some(r.list(8, |r| {
                        Ok(MipLevel {
                            far_z: r.f32()?,
                            tree: r.node_ref()?,
                        })
                    })?);
                }
                _ => return n.tree.chunk(r, id),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
