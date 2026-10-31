//! A solid (`CPlugSolid`) and what it holds beyond the block model of `tests/plug.rs`, built by hand, byte by byte, from
//! the layouts the crate documents: a tree with a light, leaves with a sprite mesh and a 2D quad mesh, a collision box,
//! and the function tree with its keys. One scene read in full, and the refusals.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test building and reading hand-made bytes"
)]

use gbx_formats::plug::BoxAligned;
use gbx_formats::plug::decorator::{DecoratorSolid, DecoratorTree};
use gbx_formats::plug::func_tree::{KeysNatural, TreeSubVisualSequence};
use gbx_formats::plug::solid::{Solid, SolidContent, SolidPhysics};
use gbx_formats::plug::surface::{Surf, Surface, SurfaceGeom};
use gbx_formats::plug::tree::{Tree, TreeLight};
use gbx_formats::plug::visual::{
    RenderMode, Vertex2D, VertexColors, VertexNormals, VisualQuads2D, VisualSprite,
};
use gbx_formats::{Gbx, Id, NodeIndex, Nodes};

const FACADE: u32 = 0xFACA_DE01;
const NONE: u32 = u32::MAX;

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

/// The body of a solid of ten nodes:
///
/// - 0: the solid, whose tree is 1;
/// - 1: a tree with a light (none) and a function tree (8), with the children 2 and 6;
/// - 2: a leaf with a sprite mesh (3) and a collision surface (4), whose geometry (5) is a box;
/// - 6: a leaf with a mesh of 2D quads (7);
/// - 8: a function choosing a sub-visual, with the keys 9.
fn body() -> Vec<u8> {
    let mut b = Vec::new();
    // node 0: CPlugSolid
    u32s(&mut b, &[0x0900_5000, 5]);
    u32s(&mut b, &[0x0900_500E]);
    f32s(
        &mut b,
        &[
            10.0, 1.0, 2.0, 3.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.1, 0.2, 7.0,
        ],
    );
    u32s(&mut b, &[0x0900_5010, NONE]);
    // not using a model, and no model: the tree is node 1
    u32s(&mut b, &[0x0900_5011, 0, 0, 1, 0x0906_2000]);
    // node 1: CPlugTreeLight, with the children 2 and 6
    u32s(&mut b, &[0x0904_F006, 10, 2]);
    // child 2: a leaf
    u32s(&mut b, &[2, 0x0904_F000, 0x0904_F00D, 3, 0x4000_0000]);
    string(&mut b, "Cloud");
    u32s(&mut b, &[NONE, 0x0904_F011, NONE]);
    // its visual: a sprite mesh (node 3)
    u32s(&mut b, &[0x0904_F016, 3, 0x0901_0000]);
    // the geometry: flags 0x3F8 (packed normals and colors, only those named), one UV channel, two vertices
    u32s(&mut b, &[0x0900_600E, 0x3F8, 1, 2, 0, 0]);
    f32s(&mut b, &[0.0, 0.0, 1.0, 1.0]);
    f32s(&mut b, &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]); // the bounding box
    u32s(&mut b, &[0]); // no bitmaps to pack
    // the vertices: a sprite's normal is three floats whatever the packed-normal bit says; colors are packed
    u32s(&mut b, &[0x0902_C004]);
    f32s(&mut b, &[1.0, 2.0, 3.0, 0.5, 0.25, 0.125]);
    u32s(&mut b, &[0xFF80_4020]);
    f32s(&mut b, &[4.0, 5.0, 6.0, 1.0, 0.0, 2.0]);
    u32s(&mut b, &[0x0000_00FF]);
    u32s(&mut b, &[0, 0]); // no tangents, no bitangents
    // rotated quads sorted back to front, then the flags, the direction and the pivot, then the atlas grid
    u32s(&mut b, &[0x0901_0001, 1, 1]);
    u32s(&mut b, &[0x0901_0005, 0x30]);
    f32s(&mut b, &[0.0, 1.0, 0.0, 0.5, 0.5]);
    u32s(&mut b, &[0x0901_0006, 0x0003_0002]);
    u32s(&mut b, &[FACADE]);
    // its shader: none; its surface: node 4, with the geometry 5
    u32s(
        &mut b,
        &[
            NONE,
            4,
            0x0900_C000,
            0x0900_C000,
            5,
            0x0900_F000,
            0x0900_F004,
            NONE,
        ],
    );
    f32s(&mut b, &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    u32s(&mut b, &[6]); // a box
    f32s(&mut b, &[1.0, 2.0, 3.0, 0.5, 0.5, 0.5]);
    b.extend_from_slice(&4i16.to_le_bytes()); // the physics material id
    u32s(&mut b, &[FACADE, 0, FACADE]); // the geometry ends, no materials, the surface ends
    u32s(&mut b, &[NONE, 0x0904_F01A, 0, FACADE]);
    // child 6: a leaf with a mesh of 2D quads (node 7)
    u32s(
        &mut b,
        &[6, 0x0904_F000, 0x0904_F00D, NONE, NONE, 0x0904_F011, NONE],
    );
    u32s(&mut b, &[0x0904_F016, 7, 0x0904_B000, 0x0904_A000, 2]);
    f32s(&mut b, &[0.0, 1.0, 0.5, 0.25, 1.0, 0.5, 0.25, 1.0]);
    f32s(&mut b, &[2.0, 3.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.5]);
    u32s(&mut b, &[FACADE, NONE, NONE, NONE]);
    u32s(&mut b, &[0x0904_F01A, 0, FACADE]);
    // the rest of node 1: its plug id, the function tree (node 8), no content, no flags, no light
    u32s(&mut b, &[0x0904_F00D, NONE, NONE, 0x0904_F011, 8, 0x0503_1000]);
    u32s(&mut b, &[0x0500_B005]);
    f32s(&mut b, &[2.0, 0.5]);
    u32s(&mut b, &[1, 0, NONE]); // automatic motion, no random phase, no input
    u32s(&mut b, &[0x0503_1001, NONE, 0x0503_1002, 9, 0x0503_0000]);
    u32s(&mut b, &[0x0500_2001, 2]);
    f32s(&mut b, &[0.0, 1.0]);
    u32s(&mut b, &[0x0500_2003, NONE, 0x0503_0000, 2, 4, 9, FACADE]);
    u32s(&mut b, &[0x0503_1003, 1, 0, 3, FACADE]);
    u32s(&mut b, &[0x0904_F016, NONE, NONE, NONE, NONE]);
    u32s(&mut b, &[0x0904_F01A, 0, 0x0906_2004, NONE, FACADE]);
    // the solid goes on: the tile count
    u32s(&mut b, &[0x0900_5012]);
    b.push(4);
    u32s(&mut b, &[FACADE]);
    b
}

fn file(body: &[u8]) -> Vec<u8> {
    file_of(0x0900_5000, body)
}

/// The container: the main node's class, ten nodes and no files.
fn file_of(class: u32, body: &[u8]) -> Vec<u8> {
    let mut f = b"GBX".to_vec();
    f.extend_from_slice(&6u16.to_le_bytes());
    f.extend_from_slice(b"BUUR");
    u32s(&mut f, &[class, 0, 10]);
    u32s(&mut f, &[0]); // no files
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
fn a_hand_built_solid_reads() {
    let nodes = read(&body()).unwrap();

    let solid: &Solid = nodes.main().try_into().unwrap();
    assert_eq!(solid.plug_id_counter, Some(5));
    assert_eq!(
        solid.physics,
        Some(SolidPhysics {
            mass: 10.0,
            center_of_mass: [1.0, 2.0, 3.0],
            inverse_inertia: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            linear_fluid_friction: 0.1,
            angular_fluid_friction: 0.2,
            max_dist_per_step: 7.0,
        })
    );
    assert_eq!(solid.vehicle_environment, Some(None));
    assert_eq!(solid.use_model, Some(false));
    assert_eq!(solid.content, Some(SolidContent::Tree(Some(NodeIndex(1)))));
    assert_eq!(solid.tile_count_u, Some(4));
    assert_eq!(
        solid.chunks.ids,
        [0x0900_5000, 0x0900_500E, 0x0900_5010, 0x0900_5011, 0x0900_5012]
    );

    let lamp: &TreeLight = get(&nodes, 1);
    assert_eq!(lamp.light, Some(None));
    assert_eq!(
        lamp.tree.children,
        Some(vec![Some(NodeIndex(2)), Some(NodeIndex(6))])
    );
    assert_eq!(lamp.tree.func_tree, Some(Some(NodeIndex(8))));
    assert_eq!(lamp.tree.plug_id, Some(Id::Empty));
    assert_eq!(lamp.tree.flags, Some(0));

    let leaf: &Tree = get(&nodes, 2);
    assert_eq!(leaf.tree.plug_id, Some(Id::Name("Cloud".into())));
    let content = leaf.tree.content.as_ref().unwrap();
    assert_eq!(
        (content.visual, content.surface),
        (Some(NodeIndex(3)), Some(NodeIndex(4)))
    );

    let sprite: &VisualSprite = get(&nodes, 3);
    assert_eq!(sprite.render_mode, Some(RenderMode::RotatedQuad));
    assert_eq!(sprite.sort_back_to_front, Some(true));
    assert_eq!(sprite.sprite_flags, Some(0x30));
    assert_eq!(sprite.global_direction, Some([0.0, 1.0, 0.0]));
    assert_eq!(sprite.pivot_point, Some([0.5, 0.5]));
    assert_eq!(sprite.atlas_grid, Some([2, 3]));
    let vertices = sprite.visual_3d.vertices.as_ref().unwrap();
    assert_eq!(vertices.positions, [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
    assert_eq!(
        vertices.normals,
        Some(VertexNormals::Floats(vec![[0.5, 0.25, 0.125], [1.0, 0.0, 2.0]]))
    );
    assert_eq!(
        vertices.colors,
        Some(VertexColors::Packed(vec![0xFF80_4020, 0xFF]))
    );
    assert_eq!(sprite.visual.geometry.as_ref().unwrap().texcoords[0].uv.len(), 2);

    let surface: &Surface = get(&nodes, 4);
    assert_eq!(surface.geom, Some(Some(NodeIndex(5))));
    let geom: &SurfaceGeom = get(&nodes, 5);
    assert_eq!(
        geom.surf,
        Some(Surf::Box {
            bounds: BoxAligned {
                center: [1.0, 2.0, 3.0],
                half_size: [0.5, 0.5, 0.5]
            }
        })
    );
    assert_eq!(geom.surface_id, Some(4));

    let quads: &VisualQuads2D = get(&nodes, 7);
    assert_eq!(
        quads.vertices,
        Some(vec![
            Vertex2D {
                position: [0.0, 1.0],
                normal: [0.5, 0.25],
                color: [1.0, 0.5, 0.25, 1.0]
            },
            Vertex2D {
                position: [2.0, 3.0],
                normal: [0.0, 0.0],
                color: [0.0, 1.0, 0.0, 0.5]
            },
        ])
    );

    let sequence: &TreeSubVisualSequence = get(&nodes, 8);
    let timing = sequence.timing.as_ref().unwrap();
    assert_eq!((timing.period, timing.phase), (2.0, 0.5));
    assert_eq!((timing.auto_create_motion, timing.randomize_phase), (true, false));
    assert_eq!(sequence.id, Some(Id::Empty));
    assert_eq!(sequence.sub_keys, Some(Some(NodeIndex(9))));
    assert_eq!(
        (
            sequence.simple_mode_is_looping,
            sequence.simple_mode_start_index,
            sequence.simple_mode_end_index
        ),
        (Some(true), Some(0), Some(3))
    );

    let keys: &KeysNatural = get(&nodes, 9);
    assert_eq!(keys.xs, Some(vec![0.0, 1.0]));
    assert_eq!(keys.naturals, Some(vec![4, 9]));
}

/// The body with `from` replaced by `to` where it first occurs.
fn patched(from: &[u32], to: &[u32]) -> Vec<u8> {
    let (mut find, mut put) = (Vec::new(), Vec::new());
    u32s(&mut find, from);
    u32s(&mut put, to);
    let b = body();
    let at = b
        .windows(find.len())
        .position(|w| w == find)
        .expect("the bytes to patch");
    [&b[..at], &put[..], &b[at + find.len()..]].concat()
}

#[test]
fn spheres_and_ellipsoids_read() {
    let bits = |v: &[f32]| v.iter().map(|f| f.to_bits()).collect::<Vec<_>>();
    let box_shape = [&[6u32][..], &bits(&[1.0, 2.0, 3.0, 0.5, 0.5, 0.5])].concat();
    let sphere = [&[0u32][..], &bits(&[2.5])].concat();
    let nodes = read(&patched(&box_shape, &sphere)).unwrap();
    let geom: &SurfaceGeom = get(&nodes, 5);
    assert_eq!(geom.surf, Some(Surf::Sphere { size: 2.5 }));
    assert_eq!(geom.surface_id, Some(4));
    let ellipsoid = [&[1u32][..], &bits(&[1.0, 2.0, 3.0])].concat();
    let nodes = read(&patched(&box_shape, &ellipsoid)).unwrap();
    let geom: &SurfaceGeom = get(&nodes, 5);
    assert_eq!(
        geom.surf,
        Some(Surf::Ellipsoid {
            size: [1.0, 2.0, 3.0]
        })
    );
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
    // a chunk of the solid this crate does not read
    refused(
        patched(&[0x0900_5010, NONE], &[0x0900_5999, NONE]),
        "chunk 0x09005999 of CPlugSolid",
    );
    // a sprite render mode that names no mode
    refused(
        patched(&[0x0901_0001, 1, 1], &[0x0901_0001, 2, 1]),
        "sprite render mode 2",
    );
    // a sprite's chunk before the geometry it needs
    refused(
        patched(&[0x0900_600E, 0x3F8], &[0x0900_6999, 0x3F8]),
        "chunk 0x09006999 of CPlugVisualSprite",
    );
    // a collision shape of a type that is not read
    refused(patched(&[6], &[5]), "collision shape type 5");
    // a function tree's keys of another chunk
    refused(
        patched(&[0x0500_2001, 2], &[0x0500_2002, 2]),
        "chunk 0x05002002 of CFuncKeysNatural",
    );
    // more 2D vertices than the chunk holds
    refused(
        patched(&[0x0904_A000, 2], &[0x0904_A000, 3_000_000]),
        "bytes outside",
    );
}

/// A decorator solid with two decorators: the first of the newest chunk, the second of the oldest.
fn decorators() -> Vec<u8> {
    let mut b = Vec::new();
    u32s(&mut b, &[0x090A_3000, 10, 2]);
    u32s(&mut b, &[1, 0x090A_2000, 0x090A_2009, 3, 0x4000_0000]);
    string(&mut b, "High");
    u32s(&mut b, &[NONE, NONE, 6, 4, 1, 4, 1, 1, 0, 2, FACADE]);
    u32s(
        &mut b,
        &[
            2,
            0x090A_2000,
            0x090A_2006,
            NONE,
            NONE,
            NONE,
            5,
            0,
            3,
            0,
            1,
            FACADE,
        ],
    );
    u32s(&mut b, &[FACADE]);
    b
}

#[test]
fn decorators_read() {
    let nodes = Gbx::parse(&file_of(0x090A_3000, &decorators()))
        .unwrap()
        .nodes()
        .unwrap();
    let solid: &DecoratorSolid = nodes.main().try_into().unwrap();
    assert_eq!(
        solid.tree_decorators,
        Some(vec![Some(NodeIndex(1)), Some(NodeIndex(2))])
    );
    let newest: &DecoratorTree = get(&nodes, 1);
    assert_eq!(newest.tree_id, Some(Id::Name("High".into())));
    assert_eq!(newest.exist_cond, Some(6));
    assert_eq!(newest.visible_cond, Some(4));
    assert_eq!(newest.visible_apply_on_childs, Some(true));
    assert_eq!(newest.shadow_caster_cond, Some(4));
    assert_eq!(newest.shadow_caster_apply_on_childs, Some(true));
    assert_eq!(newest.transform_visual_to_surface, Some(true));
    assert_eq!(newest.no_location, Some(false));
    assert_eq!(newest.collidable_cond, Some(2));
    let oldest: &DecoratorTree = get(&nodes, 2);
    assert_eq!(oldest.tree_id, Some(Id::Empty));
    assert_eq!(oldest.exist_cond, None);
    assert_eq!(oldest.visible_cond, Some(5));
    assert_eq!(oldest.transform_visual_to_surface, Some(true));
    assert_eq!((oldest.no_location, oldest.collidable_cond), (None, None));

    // every truncation, a deprecated-list version that is not 10, and a chunk the decorator does not have
    let whole = decorators();
    for n in 0..whole.len() {
        assert!(
            Gbx::parse(&file_of(0x090A_3000, &whole[..n]))
                .unwrap()
                .nodes()
                .is_err(),
            "prefix {n}"
        );
    }
    let mut other_version = whole.clone();
    other_version[4..8].copy_from_slice(&9u32.to_le_bytes());
    let e = Gbx::parse(&file_of(0x090A_3000, &other_version))
        .unwrap()
        .nodes()
        .unwrap_err();
    assert!(e.to_string().contains("tree decorators version 9"), "{e}");
    let mut other_chunk = whole;
    other_chunk[20..24].copy_from_slice(&0x090A_200Au32.to_le_bytes());
    let e = Gbx::parse(&file_of(0x090A_3000, &other_chunk))
        .unwrap()
        .nodes()
        .unwrap_err();
    assert!(
        e.to_string().contains("chunk 0x090a200a of CPlugDecoratorTree"),
        "{e}"
    );
}
