//! Decorators of a solid: `CPlugDecoratorSolid` and the `CPlugDecoratorTree`s it holds, which attach a material and a
//! light to a tree of a solid by its plug id, under conditions.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};

/// The decoration of one tree of a solid (`CPlugDecoratorTree`, class `0x090A2000`). Each condition is an integer
/// whose meaning is not known.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecoratorTree {
    /// The plug id of the tree decorated.
    pub tree_id: Option<Id>,
    /// The material (`CPlugMaterial`).
    pub material: Option<Option<NodeIndex>>,
    /// The tree with a light (`CPlugTreeLight`).
    pub tree_light: Option<Option<NodeIndex>>,
    /// Chunks `0x090A2007` and later: the condition of the tree's existence.
    pub exist_cond: Option<i32>,
    /// The condition of the tree's visibility.
    pub visible_cond: Option<i32>,
    /// Whether the visibility applies to the tree's children.
    pub visible_apply_on_childs: Option<bool>,
    /// The condition of the tree casting shadows.
    pub shadow_caster_cond: Option<i32>,
    /// Whether the shadow casting applies to the tree's children.
    pub shadow_caster_apply_on_childs: Option<bool>,
    /// Whether the visual becomes the surface (collision).
    pub transform_visual_to_surface: Option<bool>,
    /// Chunks `0x090A2008` and `0x090A2009`: whether the decoration has no location.
    pub no_location: Option<bool>,
    /// Chunk `0x090A2009`: the condition of the tree being collidable.
    pub collidable_cond: Option<i32>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl DecoratorTree {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<DecoratorTree> {
        let mut n = DecoratorTree::default();
        n.chunks = r.chunks("CPlugDecoratorTree", |r, id| {
            if !(0x090A_2006..=0x090A_2009).contains(&id) {
                return Ok(false);
            }
            // each chunk of the 7 to 9 stores what the one before it does, then more
            n.tree_id = Some(r.id()?);
            n.material = Some(r.node_ref()?);
            n.tree_light = Some(r.node_ref()?);
            if id != 0x090A_2006 {
                n.exist_cond = Some(r.i32()?);
            }
            n.visible_cond = Some(r.i32()?);
            n.visible_apply_on_childs = Some(r.bool()?);
            n.shadow_caster_cond = Some(r.i32()?);
            n.shadow_caster_apply_on_childs = Some(r.bool()?);
            n.transform_visual_to_surface = Some(r.bool()?);
            if id >= 0x090A_2008 {
                n.no_location = Some(r.bool()?);
            }
            if id == 0x090A_2009 {
                n.collidable_cond = Some(r.i32()?);
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// The decorations of a solid (`CPlugDecoratorSolid`, class `0x090A3000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecoratorSolid {
    /// Chunk `0x090A3000`: the trees' decorators (`CPlugDecoratorTree`). The chunk stores a deprecated-list version
    /// (`u32`) before the count.
    pub tree_decorators: Option<Vec<Option<NodeIndex>>>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl DecoratorSolid {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<DecoratorSolid> {
        let mut n = DecoratorSolid::default();
        n.chunks = r.chunks("CPlugDecoratorSolid", |r, id| {
            match id {
                0x090A_3000 => {
                    r.version("tree decorators", &[10])?;
                    n.tree_decorators = Some(r.list(4, |r| r.node_ref())?);
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
