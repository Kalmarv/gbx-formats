//! The Plug classes (a block model's tree, mesh, material and collision surface) built by hand, byte by byte, from the
//! layouts the crate documents: one scene read in full, and the refusals.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test building and reading hand-made bytes"
)]

use gbx_formats::plug::material::{Bitmap, Material, MaterialCustom, SkipSampler};
use gbx_formats::plug::surface::{CollisionMesh, Surf, SurfMaterial, Surface, SurfaceGeom};
use gbx_formats::plug::tree::{FLAG_LOCATION, MipLevel, Tree, TreeVisualMip};
use gbx_formats::plug::visual::VisualIndexedTriangles;
use gbx_formats::plug::{BoxAligned, Iso4};
use gbx_formats::{Gbx, Id, NodeIndex, Nodes};

const FACADE: u32 = 0xFACA_DE01;

fn u32s(out: &mut Vec<u8>, values: &[u32]) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}

fn f32s(out: &mut Vec<u8>, values: &[f32]) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}

fn string(out: &mut Vec<u8>, s: &str) {
    u32s(out, &[s.len() as u32]);
    out.extend_from_slice(s.as_bytes());
}

/// The body of a scene of ten nodes, two of them (6 and 7) standing for files named in the reference table:
///
/// - 0: a tree with two children: 1 and 9;
/// - 1: a leaf tree with a visual (2), a material (3) and a surface (4);
/// - 2: a mesh of three vertices and one triangle, with an index buffer stored in place;
/// - 3: a material whose custom material is 5 and whose shader is the file 6;
/// - 5: the custom material, with five texture slots of which one names the file 7;
/// - 4: a surface with the geometry 8;
/// - 9: a tree with levels of detail: node 1 up to 128 units.
fn body() -> Vec<u8> {
    let mut b = Vec::new();
    // node 0: CPlugTree
    u32s(&mut b, &[0x0904_F006, 10, 2]);
    // child 1: a leaf tree
    u32s(&mut b, &[1, 0x0904_F000, 0x0904_F006, 10, 0]);
    u32s(&mut b, &[0x0904_F00D, 3, 0x4000_0000]);
    string(&mut b, "Floor");
    u32s(&mut b, &[u32::MAX, 0x0904_F011, u32::MAX]);
    u32s(&mut b, &[0x0904_F016]);
    // the visual (node 2)
    u32s(&mut b, &[2, 0x0901_E000]);
    u32s(
        &mut b,
        &[0x0900_6001, u32::MAX, 0x0900_6004, u32::MAX, 0x0900_6005, 0],
    );
    b.extend_from_slice(&[0x0900_6009u32.to_le_bytes(), 0.5f32.to_bits().to_le_bytes()].concat());
    // one split: index offset 0, vertex offset 0, box
    u32s(&mut b, &[0x0900_600B, 1, 0, 0]);
    f32s(&mut b, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    // the geometry: flags 0x78, one UV channel, three vertices, no streams, UV version 0
    u32s(&mut b, &[0x0900_600E, 0x78, 1, 3, 0, 0]);
    f32s(&mut b, &[0.0, 1.0, 0.5, 0.25, 1.0, 0.0]);
    f32s(&mut b, &[1.0, 1.0, 1.0, 0.5, 0.5, 0.5]); // the bounding box
    u32s(&mut b, &[1, 7]); // one bitmap element to pack: mapper 7
    f32s(&mut b, &[2.0, 3.0, 4.0, 5.0]);
    u32s(&mut b, &[0x0902_C002, u32::MAX, 0x0902_C004]);
    for v in 0..3u32 {
        let v = v as f32;
        f32s(&mut b, &[v, v + 1.0, v + 2.0]); // position
        f32s(&mut b, &[0.0, 1.0, 0.0]); // normal
        f32s(&mut b, &[1.0, 0.5, 0.25, 1.0]); // color
    }
    u32s(&mut b, &[0, 0]); // no tangents, no bitangents
    // the index buffer, in place after a bool
    u32s(&mut b, &[0x0906_A001, 1, 0x0905_7000, 2, 3]);
    for i in [0u16, 2, 1] {
        b.extend_from_slice(&i.to_le_bytes());
    }
    u32s(&mut b, &[FACADE, FACADE]);
    // the shader: a material (node 3)
    u32s(&mut b, &[3, 0x0907_9000, 0x0907_9001, u32::MAX, 0x0907_9007]);
    // its custom material (node 5)
    u32s(&mut b, &[5, 0x0903_A000, 0x0903_A004, 0, 0x0903_A006, 5]);
    for (i, name) in ["Diffuse", "Specular", "Normal", "Lighting", "Occlusion"]
        .iter()
        .enumerate()
    {
        u32s(&mut b, &[0x4000_0000]);
        string(&mut b, name);
        // the first slot names the file 7, the others no texture
        u32s(&mut b, &[0, if i == 0 { 7 } else { u32::MAX }]);
    }
    u32s(&mut b, &[0x0903_A00A, 0, 0, 0x0903_A00C, 1, 0x4000_0000]);
    string(&mut b, "PreLightGen");
    u32s(&mut b, &[1]);
    u32s(&mut b, &[0x0903_A00D, 0, 0, 0, 0, FACADE]);
    // the material goes on: the shader is the file 6
    u32s(
        &mut b,
        &[0x0907_900D, 6, 0x0907_900E, 0x8012_0000, 0x0907_900F, 9, FACADE],
    );
    // the surface (node 4)
    u32s(&mut b, &[4, 0x0900_C000, 0x0900_C000]);
    // its geometry (node 8)
    u32s(&mut b, &[8, 0x0900_F000, 0x0900_F004, u32::MAX]);
    f32s(&mut b, &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    u32s(&mut b, &[7, 3, 3]); // a mesh, version 3, three vertices
    f32s(&mut b, &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
    u32s(&mut b, &[1]); // one cooked triangle
    f32s(&mut b, &[0.0, 1.0, 0.0, 0.0]);
    u32s(&mut b, &[0, 1, 2, 0x0102_0000]);
    u32s(&mut b, &[3, 1]); // the octree: version 3, one cell
    u32s(&mut b, &[5]);
    f32s(&mut b, &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    u32s(&mut b, &[6]);
    b.extend_from_slice(&0i16.to_le_bytes()); // the geometry's physics material id
    u32s(&mut b, &[FACADE]);
    // the surface's two materials: a physics id (a short), and a reference to no material
    u32s(&mut b, &[2, 0]);
    b.extend_from_slice(&5i16.to_le_bytes());
    u32s(&mut b, &[1, u32::MAX, FACADE]);
    // the generator, then the flags: a location (bit 2) and the visible bit
    u32s(&mut b, &[u32::MAX, 0x0904_F01A, 0x0001_E00C]);
    f32s(
        &mut b,
        &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 10.0, 20.0, 30.0],
    );
    u32s(&mut b, &[FACADE]);
    // child 9: a tree with levels of detail
    u32s(&mut b, &[9, 0x0901_5000, 0x0901_5002, 1]);
    f32s(&mut b, &[128.0]);
    u32s(&mut b, &[1]); // node 1, already read
    u32s(&mut b, &[0x0904_F006, 10, 0, 0x0904_F00D, 0x4000_0001, u32::MAX]);
    u32s(
        &mut b,
        &[
            0x0904_F011,
            u32::MAX,
            0x0904_F016,
            u32::MAX,
            u32::MAX,
            u32::MAX,
            u32::MAX,
        ],
    );
    u32s(&mut b, &[0x0904_F01A, 0, FACADE]);
    // back to node 0
    u32s(&mut b, &[0x0904_F00D, 0x4000_0001, u32::MAX]);
    u32s(
        &mut b,
        &[
            0x0904_F011,
            u32::MAX,
            0x0904_F016,
            u32::MAX,
            u32::MAX,
            u32::MAX,
            u32::MAX,
        ],
    );
    u32s(&mut b, &[0x0904_F01A, 0, FACADE]);
    b
}

/// The container: the main node's class, ten nodes, and a reference table naming the files of nodes 6 and 7.
fn file(body: &[u8]) -> Vec<u8> {
    let mut f = b"GBX".to_vec();
    f.extend_from_slice(&6u16.to_le_bytes());
    f.extend_from_slice(b"BUUR");
    u32s(&mut f, &[0x0904_F000, 0, 10]);
    u32s(&mut f, &[2, 0, 0]); // two files, no ancestors, no folders
    for (name, index) in [("Block.Shader.Gbx", 6), ("Block.Bitmap.Gbx", 7)] {
        u32s(&mut f, &[0]);
        string(&mut f, name);
        u32s(&mut f, &[index, 0, 0]);
    }
    f.extend_from_slice(body);
    f
}

fn read(body: &[u8]) -> Result<Nodes, gbx_formats::Error> {
    Gbx::parse(&file(body)).unwrap().nodes()
}

fn get<'n, T>(nodes: &'n Nodes, index: u32) -> &'n T
where
    &'n T: TryFrom<&'n gbx_formats::Node>,
{
    match <&T>::try_from(nodes.get(NodeIndex(index)).unwrap()) {
        Ok(t) => t,
        Err(_) => panic!("node {index} is not the class asked for"),
    }
}

#[test]
fn a_hand_built_scene_reads() {
    let nodes = read(&body()).unwrap();

    let root: &Tree = nodes.main().try_into().unwrap();
    assert_eq!(
        root.tree.children,
        Some(vec![Some(NodeIndex(1)), Some(NodeIndex(9))])
    );
    assert_eq!(root.tree.plug_id, Some(Id::Name("Floor".into())));
    assert_eq!(root.tree.flags, Some(0));
    assert_eq!(root.tree.location, None);
    assert_eq!(
        root.chunks.ids,
        [0x0904_F006, 0x0904_F00D, 0x0904_F011, 0x0904_F016, 0x0904_F01A]
    );

    let leaf: &Tree = get(&nodes, 1);
    assert_eq!(leaf.tree.plug_id, Some(Id::Name("Floor".into())));
    assert_eq!(leaf.tree.model_solid, Some(None));
    let content = leaf.tree.content.as_ref().unwrap();
    assert_eq!(
        (content.visual, content.shader, content.surface, content.generator),
        (Some(NodeIndex(2)), Some(NodeIndex(3)), Some(NodeIndex(4)), None)
    );
    assert_eq!(leaf.tree.flags, Some(0x1_E00C));
    assert_eq!(leaf.tree.flags.unwrap() & FLAG_LOCATION, FLAG_LOCATION);
    assert_eq!(
        leaf.tree.location,
        Some(Iso4 {
            matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [10.0, 20.0, 30.0]
        })
    );

    let mip: &TreeVisualMip = get(&nodes, 9);
    assert_eq!(
        mip.levels,
        Some(vec![MipLevel {
            far_z: 128.0,
            tree: Some(NodeIndex(1))
        }])
    );
    assert_eq!(mip.chunks.ids[0], 0x0901_5002);
    assert_eq!(mip.tree.plug_id, Some(Id::Name("Floor".into())));

    let visual: &VisualIndexedTriangles = get(&nodes, 2);
    assert_eq!(visual.visual.visual_id, Some(Id::Empty));
    assert_eq!(visual.visual.n_patch_tess_level, Some(0.5));
    assert_eq!(visual.visual.sub_visuals, Some(vec![]));
    let split = &visual.visual.splits.as_ref().unwrap()[0];
    assert_eq!(
        (split.index_offset, split.vertex_offset, split.bounding_box),
        (
            0,
            0,
            BoxAligned {
                center: [1.0, 2.0, 3.0],
                half_size: [4.0, 5.0, 6.0]
            }
        )
    );
    let g = visual.visual.geometry.as_ref().unwrap();
    assert_eq!(g.flags.0, 0x78);
    assert!(g.flags.geometry_static() && g.flags.indexation_static());
    assert!(g.flags.has_vertex_normals() && g.flags.has_vertex_colors());
    assert_eq!(g.flags.skin_index_count(), 0);
    assert_eq!(g.vertex_count, 3);
    assert_eq!(g.texcoords[0].uv, [[0.0, 1.0], [0.5, 0.25], [1.0, 0.0]]);
    assert_eq!(g.bitmap_elem_to_packs[0].mapper_index, 7);
    assert_eq!(g.bitmap_elem_to_packs[0].scale, [2.0, 3.0]);
    assert_eq!(g.bitmap_elem_to_packs[0].translation, [4.0, 5.0]);
    let vs = visual.visual_3d.vertices.as_ref().unwrap();
    assert_eq!(vs.positions, [[0.0, 1.0, 2.0], [1.0, 2.0, 3.0], [2.0, 3.0, 4.0]]);
    assert_eq!(vs.normals, [[0.0, 1.0, 0.0]; 3]);
    assert_eq!(vs.colors, [[1.0, 0.5, 0.25, 1.0]; 3]);
    assert_eq!(visual.visual_3d.blend_shapes, Some(None));
    let ib = visual.indexed.index_buffer.as_ref().unwrap();
    assert_eq!((ib.flags, &ib.indices[..]), (Some(2), &[0, 2, 1][..]));
    assert_eq!(
        visual.chunks.ids,
        [
            0x0900_6001,
            0x0900_6004,
            0x0900_6005,
            0x0900_6009,
            0x0900_600B,
            0x0900_600E,
            0x0902_C002,
            0x0902_C004,
            0x0906_A001
        ]
    );

    let material: &Material = get(&nodes, 3);
    assert_eq!(material.custom_material, Some(Some(NodeIndex(5))));
    assert_eq!(material.shader, Some(NodeIndex(6)));
    assert!(nodes.get(NodeIndex(6)).is_none(), "the shader is a file");
    assert_eq!(
        (material.surface_id, material.u02, material.u03),
        (Some(0), Some(-0x7FEE), Some(9))
    );

    let custom: &MaterialCustom = get(&nodes, 5);
    let textures = custom.textures.as_ref().unwrap();
    assert_eq!(textures.len(), 5);
    assert_eq!(
        textures[0],
        Bitmap {
            name: Id::Name("Diffuse".into()),
            u01: 0,
            texture: Some(NodeIndex(7))
        }
    );
    assert_eq!(textures[1].texture, None);
    assert_eq!(
        custom.skip_samplers,
        Some(vec![SkipSampler {
            name: Id::Name("PreLightGen".into()),
            skip: true
        }])
    );
    assert_eq!(
        (custom.gpu_fxs_1.as_deref(), custom.u02),
        (Some(&[][..]), Some((0, 0)))
    );

    let surface: &Surface = get(&nodes, 4);
    assert_eq!(surface.geom, Some(Some(NodeIndex(8))));
    assert_eq!(
        surface.materials,
        Some(vec![SurfMaterial::SurfaceId(5), SurfMaterial::Material(None)])
    );

    let geom: &SurfaceGeom = get(&nodes, 8);
    assert_eq!(geom.surface_id, Some(0));
    assert_eq!(
        geom.bounding_box,
        Some(BoxAligned {
            center: [0.0, 0.0, 0.0],
            half_size: [1.0, 1.0, 1.0]
        })
    );
    let Some(Surf::Mesh(CollisionMesh {
        vertices,
        triangles,
        octree_version,
        octree_cells,
    })) = &geom.surf
    else {
        panic!("not a mesh")
    };
    assert_eq!(vertices.len(), 3);
    assert_eq!(triangles[0].indices, [0, 1, 2]);
    assert_eq!(
        (triangles[0].surface_index, triangles[0].u04, triangles[0].u05),
        (0, 2, 1)
    );
    assert_eq!(
        (
            *octree_version,
            octree_cells.len(),
            octree_cells[0].u01,
            octree_cells[0].u04
        ),
        (3, 1, 5, 6)
    );
}

/// `body()` with `from` replaced by `to` (once; it must be found).
fn patched(from: &[u32], to: &[u32]) -> Vec<u8> {
    let mut find = Vec::new();
    u32s(&mut find, from);
    let mut put = Vec::new();
    u32s(&mut put, to);
    let b = body();
    let at = b
        .windows(find.len())
        .position(|w| w == find)
        .expect("the bytes to patch");
    [&b[..at], &put[..], &b[at + find.len()..]].concat()
}

#[test]
fn refusals() {
    // every truncation of the body
    let whole = body();
    for n in 0..whole.len() {
        assert!(read(&whole[..n]).is_err(), "prefix {n}");
    }
    let refused = |bytes: Vec<u8>, what: &str| {
        let e = read(&bytes).unwrap_err().to_string();
        assert!(e.contains(what), "{e}");
    };
    // a children list of another deprecated-list version
    refused(
        patched(&[0x0904_F006, 10, 2], &[0x0904_F006, 9, 2]),
        "tree children version 9",
    );
    // a UV channel of a version that adds floats
    refused(
        patched(&[0x0900_600E, 0x78, 1, 3, 0, 0], &[0x0900_600E, 0x78, 1, 3, 0, 1]),
        "UV channel version 1",
    );
    // a skinned mesh, packed vertex attributes, flags above bit 9
    refused(
        patched(&[0x0900_600E, 0x78], &[0x0900_600E, 0x79]),
        "skinned visuals are not read",
    );
    refused(
        patched(&[0x0900_600E, 0x78], &[0x0900_600E, 0xF8]),
        "packed vertex attributes",
    );
    refused(
        patched(&[0x0900_600E, 0x78], &[0x0900_600E, 0x478]),
        "bits above 9",
    );
    // a vertex count the chunk cannot hold
    refused(
        patched(&[0x0900_600E, 0x78, 1, 3], &[0x0900_600E, 0x78, 1, 3_000_000]),
        "do not fit",
    );
    // an inline device material with an invalid boolean
    refused(
        patched(
            &[0x0907_900D, 6],
            &[0x0907_900D, u32::MAX, 1, 0, 2, u32::MAX, u32::MAX, u32::MAX, 0],
        ),
        "bool 2",
    );
    // a collision shape other than a mesh, and a mesh or an octree of another version
    refused(patched(&[7, 3, 3], &[8, 3, 3]), "collision shape type 8");
    refused(patched(&[7, 3, 3], &[7, 4, 3]), "collision mesh version 4");
    refused(
        patched(&[0x0102_0000, 3, 1], &[0x0102_0000, 4, 1]),
        "collision octree version 4",
    );
    // tangents for some of the vertices only
    refused(
        patched(&[0, 0, 0x0906_A001], &[2, 0, 0x0906_A001]),
        "2 tangents for 3 vertices",
    );
    // an unknown chunk that is not skippable, and a bool that is not 0 or 1
    refused(
        patched(&[0x0900_6009], &[0x0900_6999]),
        "chunk 0x09006999 of CPlugVisualIndexedTriangles",
    );
    refused(patched(&[0x0906_A001, 1], &[0x0906_A001, 2]), "bool 2");
    // a node of a class the crate does not read where a visual is expected
    refused(
        patched(&[2, 0x0901_E000], &[2, 0x0901_F000]),
        "node of class 0x0901f000",
    );
    // a skippable chunk of an unknown id is kept as stored
    let skippable = patched(
        &[0x0900_6009],
        &[0x0900_6999, 0x534B_4950, 4, 0xDDCC_BBAA, 0x0900_6009],
    );
    let nodes = read(&skippable).unwrap();
    let visual: &VisualIndexedTriangles = get(&nodes, 2);
    assert_eq!(visual.chunks.unread.len(), 1);
    assert_eq!(visual.chunks.unread[0].id, 0x0900_6999);
}
