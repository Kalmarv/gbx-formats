//! Visuals (geometry): `CPlugVisualIndexedTriangles` and its bases `CPlugVisual`, `CPlugVisual3D` and
//! `CPlugVisualIndexed`, and the index buffer (`CPlugIndexBuffer`) it holds.
//!
//! A mesh is [`Visual3DData::vertices`] (positions, normals, colors, one entry per vertex), one
//! [`TexCoordSet`] per UV channel (in [`Geometry::texcoords`]), and [`IndexBuffer::indices`] (three per triangle).
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use super::{BoxAligned, box_aligned};
use crate::archive::{Archive, Id};
use crate::error::Result;
use crate::node::{Chunks, NodeIndex};

/// The geometry header's flags, as chunk `0x0900600E` stores them (a `u32`), with their meanings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GeometryFlags(pub u32);

impl GeometryFlags {
    /// Bits 0 to 2: the number of skin weights per vertex (0 for an unskinned mesh).
    pub fn skin_index_count(self) -> u32 {
        self.0 & 7
    }
    /// Bit 3: the vertices never change.
    pub fn geometry_static(self) -> bool {
        self.0 & (1 << 3) != 0
    }
    /// Bit 4: the indices never change.
    pub fn indexation_static(self) -> bool {
        self.0 & (1 << 4) != 0
    }
    /// Bit 5: the vertices have normals.
    pub fn has_vertex_normals(self) -> bool {
        self.0 & (1 << 5) != 0
    }
    /// Bit 6: the vertices have colors.
    pub fn has_vertex_colors(self) -> bool {
        self.0 & (1 << 6) != 0
    }
    /// Bit 7: normals are packed into a `u32`.
    pub fn packed_normals(self) -> bool {
        self.0 & (1 << 7) != 0
    }
    /// Bit 8: colors are packed into a `u32`.
    pub fn packed_colors(self) -> bool {
        self.0 & (1 << 8) != 0
    }
    /// Bit 9: the vertices store normals and colors only as bits 5 and 6 say (without it both are stored).
    pub fn explicit_attributes(self) -> bool {
        self.0 & (1 << 9) != 0
    }
}

/// One UV channel (`CPlugVisual.TexCoordSet`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TexCoordSet {
    /// The UV of each vertex. (The set's version is 0 in every file read: two floats per vertex; later versions
    /// add floats and are refused.)
    pub uv: Vec<[f32; 2]>,
}

/// A range of the index buffer drawn together (`CPlugVisual.Split`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Split {
    /// The first index.
    pub index_offset: i32,
    /// The first vertex.
    pub vertex_offset: i32,
    /// The range's bounds.
    pub bounding_box: BoxAligned,
}

/// A UV transform of a bitmap packed into an atlas (`CPlugVisual.BitmapElemToPack`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BitmapElemToPack {
    /// The UV channel's index.
    pub mapper_index: i32,
    /// The scale.
    pub scale: [f32; 2],
    /// The translation.
    pub translation: [f32; 2],
}

/// The geometry header and the vertex-independent data of chunk `0x0900600E`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Geometry {
    /// The flags.
    pub flags: GeometryFlags,
    /// The number of vertices.
    pub vertex_count: u32,
    /// The vertex streams (`CPlugVertexStream`); a visual with streams keeps its vertices there, not in chunk
    /// `0x0902C004`.
    pub vertex_streams: Vec<Option<NodeIndex>>,
    /// The UV channels, each with `vertex_count` entries.
    pub texcoords: Vec<TexCoordSet>,
    /// The visual's bounds.
    pub bounding_box: BoxAligned,
    /// The UV transforms for packed bitmaps.
    pub bitmap_elem_to_packs: Vec<BitmapElemToPack>,
}

/// What `CPlugVisual` stores.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VisualData {
    /// Chunk `0x09006001`.
    pub visual_id: Option<Id>,
    /// Chunk `0x09006004`: the function visual (`CMwNod`).
    pub func_visual: Option<Option<NodeIndex>>,
    /// Chunk `0x09006005`: the sub-visuals, three integers each.
    pub sub_visuals: Option<Vec<[i32; 3]>>,
    /// Chunk `0x09006009`: the N-patch tessellation level.
    pub n_patch_tess_level: Option<f32>,
    /// Chunk `0x0900600B`: the splits.
    pub splits: Option<Vec<Split>>,
    /// Chunk `0x0900600E`.
    pub geometry: Option<Geometry>,
}

/// The vertices of chunk `0x0902C004`, a field per attribute, each with one entry per vertex.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Vertices {
    /// The positions.
    pub positions: Vec<[f32; 3]>,
    /// The normals.
    pub normals: Vec<[f32; 3]>,
    /// The colors (RGBA, floats).
    pub colors: Vec<[f32; 4]>,
}

/// What `CPlugVisual3D` stores.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Visual3DData {
    /// Chunk `0x0902C002`: the blend shapes (`CMwNod`).
    pub blend_shapes: Option<Option<NodeIndex>>,
    /// Chunk `0x0902C004`: the vertices, absent when the visual has vertex streams.
    pub vertices: Option<Vertices>,
    /// Chunk `0x0902C004`: the tangents, empty or one per vertex.
    pub tangents: Vec<[f32; 3]>,
    /// Chunk `0x0902C004`: the bitangents, empty or one per vertex.
    pub bitangents: Vec<[f32; 3]>,
}

/// What `CPlugVisualIndexed` stores.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VisualIndexedData {
    /// Chunk `0x0906A001`: the index buffer, stored in place after a bool; `None` when the bool is false.
    pub index_buffer: Option<IndexBuffer>,
}

/// An index buffer (`CPlugIndexBuffer`, class `0x09057000`), stored in place and not as a node of the file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IndexBuffer {
    /// Chunk `0x09057000`: the flags.
    pub flags: Option<u32>,
    /// Chunk `0x09057000`: the vertex indices, three per triangle.
    pub indices: Vec<u16>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl IndexBuffer {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<IndexBuffer> {
        let mut n = IndexBuffer::default();
        n.chunks = r.chunks("CPlugIndexBuffer", |r, id| {
            match id {
                0x0905_7000 => {
                    n.flags = Some(r.u32()?);
                    let count = r.count(2)?;
                    n.indices = Vec::with_capacity(count);
                    for _ in 0..count {
                        n.indices.push(r.u16()?);
                    }
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// An indexed triangle mesh (`CPlugVisualIndexedTriangles`, class `0x0901E000`) and the chunks of its bases.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VisualIndexedTriangles {
    /// The `CPlugVisual` base.
    pub visual: VisualData,
    /// The `CPlugVisual3D` base.
    pub visual_3d: Visual3DData,
    /// The `CPlugVisualIndexed` base.
    pub indexed: VisualIndexedData,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl VisualIndexedTriangles {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<VisualIndexedTriangles> {
        let mut n = VisualIndexedTriangles::default();
        n.chunks = r.chunks("CPlugVisualIndexedTriangles", |r, id| {
            match id {
                0x0900_6001 => n.visual.visual_id = Some(r.id()?),
                0x0900_6004 => n.visual.func_visual = Some(r.node_ref()?),
                0x0900_6005 => n.visual.sub_visuals = Some(r.list(12, |r| r.int3())?),
                0x0900_6009 => n.visual.n_patch_tess_level = Some(r.f32()?),
                0x0900_600B => {
                    n.visual.splits = Some(r.list(32, |r| {
                        Ok(Split {
                            index_offset: r.i32()?,
                            vertex_offset: r.i32()?,
                            bounding_box: box_aligned(r)?,
                        })
                    })?);
                }
                0x0900_600E => n.visual.geometry = Some(geometry(r)?),
                0x0902_C002 => n.visual_3d.blend_shapes = Some(r.node_ref()?),
                0x0902_C004 => vertices(r, n.visual.geometry.as_ref(), &mut n.visual_3d)?,
                0x0906_A001 => {
                    n.indexed.index_buffer = if r.bool()? {
                        Some(IndexBuffer::read(r)?)
                    } else {
                        None
                    };
                }
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// Chunk `0x0900600E`: the geometry header, UV channels, bounds and UV transforms.
fn geometry(r: &mut Archive<'_, '_>) -> Result<Geometry> {
    let flags_at = r.here();
    let flags = r.u32()?;
    if flags & !0x3FF != 0 {
        return Err(r.bad_at(
            flags_at,
            format!("geometry flags {flags:#x}: bits above 9 are not read"),
        ));
    }
    let flags = GeometryFlags(flags);
    if flags.skin_index_count() != 0 {
        return Err(r.bad_at(
            flags_at,
            format!("geometry flags {:#x}: skinned visuals are not read", flags.0),
        ));
    }
    if flags.packed_normals() || flags.packed_colors() || flags.explicit_attributes() {
        return Err(r.bad_at(
            flags_at,
            format!(
                "geometry flags {:#x}: packed vertex attributes are not read",
                flags.0
            ),
        ));
    }
    let set_count = r.count(4)?;
    let count_at = r.here();
    let vertex_count = r.i32()?;
    let vertex_count = u32::try_from(vertex_count)
        .map_err(|_| r.bad_at(count_at, format!("vertex count {vertex_count}")))?;
    let vertex_streams = r.list(4, |r| r.node_ref())?;
    let mut texcoords = Vec::with_capacity(set_count);
    for _ in 0..set_count {
        texcoords.push(tex_coord_set(r, vertex_count)?);
    }
    let bounding_box = box_aligned(r)?;
    let bitmap_elem_to_packs = r.list(20, |r| {
        Ok(BitmapElemToPack {
            mapper_index: r.i32()?,
            scale: r.vec2()?,
            translation: r.vec2()?,
        })
    })?;
    Ok(Geometry {
        flags,
        vertex_count,
        vertex_streams,
        texcoords,
        bounding_box,
        bitmap_elem_to_packs,
    })
}

fn tex_coord_set(r: &mut Archive<'_, '_>, vertex_count: u32) -> Result<TexCoordSet> {
    r.version("UV channel", &[0])?;
    let n = usize::try_from(vertex_count).unwrap_or(usize::MAX);
    if n.checked_mul(8).is_none_or(|bytes| bytes > r.remaining()) {
        return Err(r.bad(format!("{vertex_count} UV entries do not fit in the chunk")));
    }
    let uv = (0..n).map(|_| r.vec2()).collect::<Result<Vec<_>>>()?;
    Ok(TexCoordSet { uv })
}

/// Chunk `0x0902C004`: the vertices, tangents and bitangents.
fn vertices(r: &mut Archive<'_, '_>, geometry: Option<&Geometry>, out: &mut Visual3DData) -> Result<()> {
    let Some(g) = geometry else {
        return Err(r.bad("vertices (chunk 0x0902C004) before the geometry header (chunk 0x0900600E)"));
    };
    let n = usize::try_from(g.vertex_count).unwrap_or(usize::MAX);
    if g.vertex_streams.is_empty() {
        // Each vertex is a position, a normal and a color: the flags' bits 5 and 6 only matter with bit 9, which
        // is refused above.
        let each = 12 + 12 + 16;
        if n.checked_mul(each).is_none_or(|bytes| bytes > r.remaining()) {
            return Err(r.bad(format!("{n} vertices do not fit in the chunk")));
        }
        let mut v = Vertices {
            positions: Vec::with_capacity(n),
            normals: Vec::with_capacity(n),
            colors: Vec::with_capacity(n),
        };
        for _ in 0..n {
            v.positions.push(r.vec3()?);
            v.normals.push(r.vec3()?);
            v.colors.push(r.vec4()?);
        }
        out.vertices = Some(v);
    }
    out.tangents = tangents(r, n)?;
    out.bitangents = tangents(r, n)?;
    Ok(())
}

/// A tangent array: a count that is 0 or the vertex count, and that many vectors.
fn tangents(r: &mut Archive<'_, '_>, vertex_count: usize) -> Result<Vec<[f32; 3]>> {
    let at = r.here();
    let count = r.count(12)?;
    if count != 0 && count != vertex_count {
        return Err(r.bad_at(at, format!("{count} tangents for {vertex_count} vertices")));
    }
    (0..count).map(|_| r.vec3()).collect()
}
