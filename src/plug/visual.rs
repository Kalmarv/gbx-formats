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

/// The normals of a mesh: three floats each, or packed into a `u32` (geometry flag bit 7).
#[derive(Clone, Debug, PartialEq)]
pub enum VertexNormals {
    /// Three floats per vertex.
    Floats(Vec<[f32; 3]>),
    /// One packed value per vertex; [`unpack_normal`] gives the vector.
    Packed(Vec<u32>),
}

/// The colors of a mesh: four floats each, or packed into a `u32` (geometry flag bit 8).
#[derive(Clone, Debug, PartialEq)]
pub enum VertexColors {
    /// Red, green, blue and alpha as floats.
    Floats(Vec<[f32; 4]>),
    /// One packed value per vertex; [`unpack_color`] gives the color.
    Packed(Vec<u32>),
}

/// The tangents (or bitangents) of a mesh, empty or one per vertex.
#[derive(Clone, Debug, PartialEq)]
pub enum Tangents {
    /// Three floats per vertex.
    Floats(Vec<[f32; 3]>),
    /// One `u32` per vertex, stored when geometry flag bit 7 is set; how the value is packed is not known.
    Packed(Vec<u32>),
}

impl Default for Tangents {
    fn default() -> Self {
        Tangents::Floats(Vec::new())
    }
}

/// The normal in a packed value: three signed 10-bit components, x in bits 0 to 9, y in 10 to 19 and z in 20 to 29,
/// each divided by 511 (the two top bits are not used).
pub fn unpack_normal(packed: u32) -> [f32; 3] {
    let component = |shift: u32| {
        let v = i32::from_le_bytes((packed << shift).to_le_bytes()) >> 22;
        v as f32 / 511.0
    };
    [component(22), component(12), component(2)]
}

/// The color in a packed value: red in bits 16 to 23, green in 8 to 15, blue in 0 to 7 and alpha in 24 to 31, each
/// divided by 255.
pub fn unpack_color(packed: u32) -> [f32; 4] {
    let component = |shift: u32| ((packed >> shift) & 255) as f32 / 255.0;
    [component(16), component(8), component(0), component(24)]
}

/// The vertices of chunk `0x0902C004`, a field per attribute, each with one entry per vertex.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Vertices {
    /// The positions.
    pub positions: Vec<[f32; 3]>,
    /// The normals, when the geometry flags say the vertices have them. (A sprite stores its size and rotation
    /// in the normal.)
    pub normals: Option<VertexNormals>,
    /// The colors, when the geometry flags say the vertices have them.
    pub colors: Option<VertexColors>,
}

/// What `CPlugVisual3D` stores.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Visual3DData {
    /// Chunk `0x0902C002`: the blend shapes (`CMwNod`).
    pub blend_shapes: Option<Option<NodeIndex>>,
    /// Chunk `0x0902C004`: the vertices, absent when the visual has vertex streams.
    pub vertices: Option<Vertices>,
    /// Chunk `0x0902C004`: the tangents, empty or one per vertex.
    pub tangents: Tangents,
    /// Chunk `0x0902C004`: the bitangents, empty or one per vertex.
    pub bitangents: Tangents,
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
        // stored in place, but a pack file's reader starts it like a node of its class
        r.dummy_write(u32::to_le_bytes(0x0902_B000));
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

impl VisualData {
    /// Reads chunk `id` if it is a `CPlugVisual` chunk this crate reads.
    pub(crate) fn chunk(&mut self, r: &mut Archive<'_, '_>, id: u32) -> Result<bool> {
        match id {
            0x0900_6001 => self.visual_id = Some(r.id()?),
            0x0900_6004 => self.func_visual = Some(r.node_ref()?),
            0x0900_6005 => self.sub_visuals = Some(r.list(12, |r| r.int3())?),
            0x0900_6009 => self.n_patch_tess_level = Some(r.f32()?),
            0x0900_600B => {
                self.splits = Some(r.list(32, |r| {
                    Ok(Split {
                        index_offset: r.i32()?,
                        vertex_offset: r.i32()?,
                        bounding_box: box_aligned(r)?,
                    })
                })?);
            }
            0x0900_600E => self.geometry = Some(geometry(r)?),
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl Visual3DData {
    /// Reads chunk `id` if it is a `CPlugVisual3D` chunk this crate reads. `visual` is the `CPlugVisual` base read so
    /// far, `sprite` whether the node is a `CPlugVisualSprite`.
    pub(crate) fn chunk(
        &mut self,
        r: &mut Archive<'_, '_>,
        id: u32,
        visual: &VisualData,
        sprite: bool,
    ) -> Result<bool> {
        match id {
            0x0902_C002 => self.blend_shapes = Some(r.node_ref()?),
            0x0902_C004 => vertices(r, visual.geometry.as_ref(), sprite, self)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl VisualIndexedTriangles {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<VisualIndexedTriangles> {
        let mut n = VisualIndexedTriangles::default();
        n.chunks = r.chunks("CPlugVisualIndexedTriangles", |r, id| {
            if n.visual.chunk(r, id)? || n.visual_3d.chunk(r, id, &n.visual, false)? {
                return Ok(true);
            }
            match id {
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

/// How a sprite is drawn (`CPlugVisualSprite.ERenderMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// Value 0: a quad facing the camera.
    Quad,
    /// Value 1: a quad facing the camera and rotated.
    RotatedQuad,
}

/// A sprite mesh (`CPlugVisualSprite`, class `0x09010000`): the vertices are the sprites' centers, and their normal
/// slot holds each sprite's size and rotation (the geometry flags' packed-normal bit does not apply to it).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VisualSprite {
    /// The `CPlugVisual` base.
    pub visual: VisualData,
    /// The `CPlugVisual3D` base.
    pub visual_3d: Visual3DData,
    /// Chunks `0x09010000` and `0x09010001`: the render mode.
    pub render_mode: Option<RenderMode>,
    /// Chunk `0x09010001`: whether the sprites are sorted back to front.
    pub sort_back_to_front: Option<bool>,
    /// Chunks `0x09010002` and `0x09010005`: the sprite flags. GBX.NET names bits 0 to 2 the render mode, 4 sort back to
    /// front, 3 radius in screen space, 5 use the global direction and 6 use the texture atlas.
    pub sprite_flags: Option<u32>,
    /// Chunk `0x09010005`: the global direction, used with flag bit 5.
    pub global_direction: Option<[f32; 3]>,
    /// Chunk `0x09010005`: the pivot point.
    pub pivot_point: Option<[f32; 2]>,
    /// Chunk `0x09010006`: the texture atlas' grid size, in cells along U and along V.
    pub atlas_grid: Option<[u16; 2]>,
    /// The node's chunks.
    pub chunks: Chunks,
}

fn render_mode(r: &mut Archive<'_, '_>) -> Result<RenderMode> {
    let at = r.here();
    match r.u32()? {
        0 => Ok(RenderMode::Quad),
        1 => Ok(RenderMode::RotatedQuad),
        other => Err(r.bad_at(at, format!("sprite render mode {other}"))),
    }
}

impl VisualSprite {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<VisualSprite> {
        let mut n = VisualSprite::default();
        n.chunks = r.chunks("CPlugVisualSprite", |r, id| {
            if n.visual.chunk(r, id)? || n.visual_3d.chunk(r, id, &n.visual, true)? {
                return Ok(true);
            }
            match id {
                0x0901_0000 => n.render_mode = Some(render_mode(r)?),
                0x0901_0001 => {
                    n.render_mode = Some(render_mode(r)?);
                    n.sort_back_to_front = Some(r.bool()?);
                }
                0x0901_0002 => n.sprite_flags = Some(r.u32()?),
                0x0901_0005 => {
                    n.sprite_flags = Some(r.u32()?);
                    n.global_direction = Some(r.vec3()?);
                    n.pivot_point = Some(r.vec2()?);
                }
                0x0901_0006 => n.atlas_grid = Some([r.u16()?, r.u16()?]),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// One vertex of a 2D mesh (`CPlugVisual2D.Vertex2D`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex2D {
    /// The position on the screen.
    pub position: [f32; 2],
    /// The normal slot's two floats, as stored.
    pub normal: [f32; 2],
    /// The color: red, green, blue and alpha.
    pub color: [f32; 4],
}

/// A mesh of 2D quads (`CPlugVisualQuads2D`, class `0x0904B000`, a subclass of `CPlugVisual2D`, which holds the
/// vertices).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VisualQuads2D {
    /// The `CPlugVisual` base.
    pub visual: VisualData,
    /// `CPlugVisual2D` chunk `0x0904A000`: the vertices.
    pub vertices: Option<Vec<Vertex2D>>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl VisualQuads2D {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<VisualQuads2D> {
        let mut n = VisualQuads2D::default();
        n.chunks = r.chunks("CPlugVisualQuads2D", |r, id| {
            if n.visual.chunk(r, id)? {
                return Ok(true);
            }
            match id {
                0x0904_A000 => {
                    n.vertices = Some(r.list(32, |r| {
                        Ok(Vertex2D {
                            position: r.vec2()?,
                            normal: r.vec2()?,
                            color: r.vec4()?,
                        })
                    })?);
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

/// Chunk `0x0902C004`: the vertices, tangents and bitangents. Which attributes a vertex stores, and how, follow the
/// geometry flags: without bit 9 normals and colors are both stored, with it only those bits 5 and 6 name.
fn vertices(
    r: &mut Archive<'_, '_>,
    geometry: Option<&Geometry>,
    sprite: bool,
    out: &mut Visual3DData,
) -> Result<()> {
    let Some(g) = geometry else {
        return Err(r.bad("vertices (chunk 0x0902C004) before the geometry header (chunk 0x0900600E)"));
    };
    let n = usize::try_from(g.vertex_count).unwrap_or(usize::MAX);
    let flags = g.flags;
    if g.vertex_streams.is_empty() {
        let has_normals = !flags.explicit_attributes() || flags.has_vertex_normals();
        let has_colors = !flags.explicit_attributes() || flags.has_vertex_colors();
        // a sprite's normal slot holds floats whatever bit 7 says
        let packed_normals = flags.packed_normals() && !sprite;
        let packed_colors = flags.packed_colors();
        let normal_size = if packed_normals { 4 } else { 12 };
        let color_size = if packed_colors { 4 } else { 16 };
        let each = 12usize
            .saturating_add(if has_normals { normal_size } else { 0 })
            .saturating_add(if has_colors { color_size } else { 0 });
        if n.checked_mul(each).is_none_or(|bytes| bytes > r.remaining()) {
            return Err(r.bad(format!("{n} vertices do not fit in the chunk")));
        }
        let mut v = Vertices {
            positions: Vec::with_capacity(n),
            normals: has_normals.then(|| {
                if packed_normals {
                    VertexNormals::Packed(Vec::with_capacity(n))
                } else {
                    VertexNormals::Floats(Vec::with_capacity(n))
                }
            }),
            colors: has_colors.then(|| {
                if packed_colors {
                    VertexColors::Packed(Vec::with_capacity(n))
                } else {
                    VertexColors::Floats(Vec::with_capacity(n))
                }
            }),
        };
        for _ in 0..n {
            v.positions.push(r.vec3()?);
            match &mut v.normals {
                Some(VertexNormals::Floats(f)) => f.push(r.vec3()?),
                Some(VertexNormals::Packed(p)) => p.push(r.u32()?),
                None => {}
            }
            match &mut v.colors {
                Some(VertexColors::Floats(f)) => f.push(r.vec4()?),
                Some(VertexColors::Packed(p)) => p.push(r.u32()?),
                None => {}
            }
        }
        out.vertices = Some(v);
    }
    out.tangents = tangents(r, n, flags.packed_normals())?;
    out.bitangents = tangents(r, n, flags.packed_normals())?;
    Ok(())
}

/// A tangent array: a count that is 0 or the vertex count, and that many vectors (or packed values).
fn tangents(r: &mut Archive<'_, '_>, vertex_count: usize, packed: bool) -> Result<Tangents> {
    let at = r.here();
    let count = r.count(if packed { 4 } else { 12 })?;
    if count != 0 && count != vertex_count {
        return Err(r.bad_at(at, format!("{count} tangents for {vertex_count} vertices")));
    }
    Ok(if packed {
        Tangents::Packed((0..count).map(|_| r.u32()).collect::<Result<_>>()?)
    } else {
        Tangents::Floats((0..count).map(|_| r.vec3()).collect::<Result<_>>()?)
    })
}
