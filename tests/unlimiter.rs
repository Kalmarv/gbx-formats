//! TMUnlimiter's chunks of a map built by hand, byte by byte, from the layouts the crate documents: versions 1, 6 and
//! 7 of chunks `0x3F001001` to `0x3F001003`, and the encrypted chunk `0x3F001000` (encrypted here by a second
//! implementation of the byte crypt), each read in full and then refused when a byte is made wrong.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "a test building and reading hand-made bytes"
)]

use gbx_formats::game::challenge::Challenge;
use gbx_formats::game::parameters::{NAMES, ValueKind, kind};
use gbx_formats::game::unlimiter::{
    BlockKind, ClipMapping, ClipResource, Condition, Event, LegacyValue, Operation, ParameterValue,
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

fn string8(out: &mut Vec<u8>, s: &str) {
    out.push(s.len() as u8);
    out.extend_from_slice(s.as_bytes());
}

/// The index of a parameter function.
fn function(name: &str) -> u32 {
    NAMES.iter().position(|n| *n == name).unwrap() as u32
}

/// A skippable chunk.
fn skippable(id: u32, data: &[u8]) -> Vec<u8> {
    let mut c = Vec::new();
    u32s(&mut c, &[id, 0x534B_4950, data.len() as u32]);
    c.extend_from_slice(data);
    c
}

/// A map whose body is `chunks` and the end marker, in a container that counts `nodes` nodes.
fn file(chunks: &[u8], nodes: u32) -> Vec<u8> {
    let mut body = chunks.to_vec();
    u32s(&mut body, &[FACADE]);
    let mut f = b"GBX".to_vec();
    f.extend_from_slice(&6u16.to_le_bytes());
    f.extend_from_slice(b"BUUR");
    u32s(&mut f, &[0x0304_3000, 16, 1, 0x0304_3004, 4, 6, nodes, 0]);
    f.extend_from_slice(&body);
    f
}

fn read(chunks: &[u8], nodes: u32) -> Result<Nodes, gbx_formats::Error> {
    Gbx::parse(&file(chunks, nodes)).unwrap().nodes()
}

fn map(nodes: &Nodes) -> &Challenge {
    nodes.main().try_into().unwrap()
}

fn refused(chunks: &[u8], nodes: u32, expect: &str) {
    let e = read(chunks, nodes).unwrap_err().to_string();
    assert!(e.contains(expect), "{e:?} does not contain {expect:?}");
}

// ---- version 1 ----

fn v1() -> Vec<u8> {
    let mut d = vec![1u8];
    u32s(&mut d, &[1]);
    // a game block: a new name, the collection 26, byte coordinates, direction, flags
    u32s(&mut d, &[3, 0x4000_0000]);
    string(&mut d, "StadiumRoadMainStraight");
    u32s(&mut d, &[26]);
    d.extend_from_slice(&[4, 0, 9, 2]);
    u32s(&mut d, &[0x10]);
    skippable(0x3F00_1001, &d)
}

#[test]
fn version_1_reads_blocks_with_byte_coordinates() {
    let nodes = read(&v1(), 1).unwrap();
    let u = map(&nodes).unlimiter.as_ref().unwrap();
    assert_eq!((u.chunk, u.version), (0x3F00_1001, 1));
    assert_eq!(u.blocks.len(), 1);
    let BlockKind::Game(g) = &u.blocks[0].kind else {
        panic!()
    };
    assert_eq!(g.name, Id::Name("StadiumRoadMainStraight".into()));
    assert_eq!(g.collection, Id::Number(26));
    assert_eq!((g.coord, g.direction, g.flags), ([4, 0, 9], 2, 0x10));
    assert_eq!(map(&nodes).chunks.ids, [0x3F00_1001]);
}

#[test]
fn version_0_holds_nothing_and_other_versions_are_refused() {
    let nodes = read(&skippable(0x3F00_1003, &[0]), 1).unwrap();
    let u = map(&nodes).unlimiter.as_ref().unwrap();
    assert_eq!((u.version, u.blocks.len()), (0, 0));
    refused(&skippable(0x3F00_1003, &[0, 0]), 1, "bytes left");
    for v in [2u8, 5, 8, 255] {
        refused(&skippable(0x3F00_1003, &[v]), 1, "TMUnlimiter data version");
    }
}

// ---- version 6 ----

fn v6(clip_type: u8, clip_resource: i32, group: i32, block_type: u8, flags2: u16) -> Vec<u8> {
    let mut d = vec![6u8];
    // offset and scale, visibility 1 (sky only), pylons disabled
    d.push(1 | 2 | (1 << 2) | (1 << 4));
    f32s(&mut d, &[1.0, 2.0, 3.0, 0.5, 0.5, 0.5]);
    // one legacy script: a name and two bytes of code
    u32s(&mut d, &[1]);
    string(&mut d, "S");
    u32s(&mut d, &[2]);
    d.extend_from_slice(&[0xAA, 0xBB]);
    // one parameter set of two parameters: a string one (one-byte length) and a float
    u32s(&mut d, &[1]);
    string(&mut d, "P");
    u32s(&mut d, &[2, function("World_ExecuteParameterSet")]);
    string8(&mut d, "set");
    u32s(&mut d, &[function("Vehicle_Scale")]);
    f32s(&mut d, &[2.0]);
    // two clip mappings
    u32s(&mut d, &[2, 0]);
    d.push(0);
    u32s(&mut d, &[0, 3]);
    d.push(clip_type);
    d.extend_from_slice(&clip_resource.to_le_bytes());
    // one block group
    u32s(&mut d, &[1]);
    string(&mut d, "G");
    // two blocks: a game block and an external one
    u32s(&mut d, &[2]);
    d.push(0);
    u32s(&mut d, &[3, 0x4000_0000]);
    string(&mut d, "Block");
    u32s(&mut d, &[26, 1, 2, 3]);
    d.push(1);
    u32s(&mut d, &[0x8000]);
    u32s(&mut d, &[0x4000_0000 | 1, NONE]);
    // flags 2: inverted, offset, scale, dynamic, a block group
    d.extend_from_slice(&(1u16 | 2 | 8 | 16 | 256).to_le_bytes());
    f32s(&mut d, &[10.0, 20.0, 30.0, 2.0, 2.0, 2.0]);
    d.extend_from_slice(&group.to_le_bytes());
    d.push(block_type);
    u32s(&mut d, &[0x4000_0000]);
    string(&mut d, "Ext");
    u32s(&mut d, &[NONE, 5, 6, 7]);
    d.push(3);
    u32s(&mut d, &[0]);
    d.extend_from_slice(&flags2.to_le_bytes());
    skippable(0x3F00_1003, &d)
}

#[test]
fn version_6_reads() {
    let nodes = read(&v6(1, 0, 0, 2, (1 << 7) | (1 << 9)), 1).unwrap();
    let u = map(&nodes).unlimiter.as_ref().unwrap();
    assert_eq!(u.version, 6);
    let d = u.decoration.as_ref().unwrap();
    assert_eq!(d.visibility, 1);
    assert_eq!((d.offset, d.scale), (Some([1.0, 2.0, 3.0]), Some([0.5; 3])));
    assert!(u.pylons_disabled && !u.track_base_empty);
    assert_eq!(u.legacy_scripts[0].name, "S");
    assert_eq!(u.legacy_scripts[0].byte_code, [0xAA, 0xBB]);
    let p = &u.legacy_parameter_sets[0];
    assert_eq!(p.name, "P");
    assert_eq!(p.parameters[0].value, LegacyValue::String("set".into()));
    assert_eq!(p.parameters[1].value, LegacyValue::Float(2.0));
    assert_eq!(
        u.clip_mappings,
        [
            ClipMapping {
                clip_index: 0,
                resource: ClipResource::ParameterSet(0)
            },
            ClipMapping {
                clip_index: 3,
                resource: ClipResource::LegacyScript(0)
            },
        ]
    );
    assert_eq!(u.block_groups, ["G"]);
    let BlockKind::Game(g) = &u.blocks[0].kind else {
        panic!()
    };
    assert_eq!(
        (g.name.clone(), g.coord, g.direction),
        (Id::Name("Block".into()), [1, 2, 3], 1)
    );
    assert_eq!(
        (g.flags, g.skin.clone()),
        (0x8000, Some((Id::Name("Block".into()), None)))
    );
    let p = &u.blocks[0].properties;
    assert!(p.inverted && p.dynamic && !p.invisible && !p.classic_terrain);
    assert_eq!(
        (p.offset, p.scale, p.rotation),
        (Some([10.0, 20.0, 30.0]), Some([2.0; 3]), None)
    );
    assert_eq!(p.block_groups, [0]);
    let BlockKind::External {
        name,
        author,
        coord,
        flags,
        ..
    } = &u.blocks[1].kind
    else {
        panic!()
    };
    assert_eq!(
        (name.clone(), author.clone(), *coord, *flags),
        (Id::Name("Ext".into()), Id::Empty, [5, 6, 7], 0)
    );
    let p = &u.blocks[1].properties;
    assert_eq!((p.spawn_method, p.classic_terrain), (2, true));
}

#[test]
fn version_6_refusals() {
    // a clip mapping of type 2, or to a resource that is not there
    refused(&v6(2, 0, 0, 2, 0), 1, "clip mapping resource type 2");
    refused(&v6(1, 1, 0, 2, 0), 1, "clip mapping resource 1: there are 1");
    refused(&v6(0, -1, 0, 2, 0), 1, "clip mapping resource -1");
    // a block group that is not there
    refused(&v6(1, 0, 1, 2, 0), 1, "block group 1: the map has 1");
    // a block type this version does not have, and a bit it does not read
    refused(&v6(1, 0, 0, 1, 0), 1, "block type 1");
    refused(&v6(1, 0, 0, 2, 1 << 10), 1, "bits above 9");
    // every truncation of the chunk
    let whole = v6(1, 0, 0, 2, 0);
    for n in 1..whole.len() {
        assert!(
            Gbx::parse(&file(&whole[..n], 1)).and_then(|g| g.nodes()).is_err(),
            "prefix {n}"
        );
    }
}

// ---- version 7 ----

/// The ways a version 7 chunk is made wrong.
#[derive(Clone)]
struct Wrong {
    flags: u16,
    module_flags: u8,
    function: u32,
    operation: u8,
    trigger_group: u32,
    block_group: u32,
    embedded: u32,
    specular: u32,
    block_type: u8,
    block_flags: u16,
    condition_target: u8,
    waypoint: u8,
    extra: Vec<u8>,
}

impl Wrong {
    fn good() -> Wrong {
        Wrong {
            flags: 2 | 4 | 16 | 64 | 512,
            module_flags: 1,
            function: function("Vehicle_Scale"),
            operation: 1,
            trigger_group: 0,
            block_group: 1,
            embedded: 0,
            specular: 4,
            block_type: 0,
            block_flags: 1 | (1 << 3) | (1 << 5) | (1 << 6) | (1 << 9) | (2 << 11) | (1 << 13),
            condition_target: 0,
            waypoint: 2,
            extra: Vec::new(),
        }
    }
}

/// The chunk `id` of a version 7 map of nodes 0 (the map), 1 (a tree) and the files 3 to 6.
fn v7(id: u32, w: &Wrong) -> Vec<u8> {
    let mut d = vec![7u8];
    d.extend_from_slice(&w.flags.to_le_bytes());
    f32s(&mut d, &[5.0, 6.0, 7.0]);
    // two AngelScript modules, both empty: a named one and a core one
    u32s(&mut d, &[2]);
    d.push(w.module_flags);
    if w.module_flags & 2 == 0 {
        string(&mut d, "Main");
    }
    d.push(3);
    // one parameter set, one parameter of each kind of value
    u32s(&mut d, &[1]);
    string(&mut d, "Boost");
    u32s(&mut d, &[8]);
    u32s(&mut d, &[w.function]);
    d.push(w.operation);
    f32s(&mut d, &[1.5]);
    u32s(&mut d, &[function("World_ExecuteScript")]);
    d.push(0);
    string(&mut d, "go");
    // a curve stored in place: its ids start the id scope
    u32s(&mut d, &[function("Vehicle_AccelerationCurve")]);
    d.push(1);
    u32s(&mut d, &[0x0500_2001, 2]);
    f32s(&mut d, &[0.0, 1.0]);
    u32s(&mut d, &[0x0500_2003, 3, NONE, 0x0501_A001, 2]);
    f32s(&mut d, &[5.0, 7.0]);
    u32s(&mut d, &[0, FACADE]);
    // the same function multiplying
    u32s(&mut d, &[function("Vehicle_AccelerationCurve")]);
    d.push(2);
    f32s(&mut d, &[0.5]);
    u32s(&mut d, &[function("Vehicle_GearCount")]);
    d.push(1);
    u32s(&mut d, &[5]);
    u32s(&mut d, &[function("Vehicle_M6GearRatio")]);
    d.push(1);
    u32s(&mut d, &[2]);
    f32s(&mut d, &[3.0, 2.0]);
    u32s(&mut d, &[function("Vehicle_IsFakeEngine")]);
    d.extend_from_slice(&[1, 1]);
    u32s(&mut d, &[function("Reset_Everything")]);
    d.push(0);
    // one trigger group: a media tracker condition, an event running a parameter set and one running a script
    u32s(&mut d, &[1]);
    string(&mut d, "Zone");
    d.push(1 | 2 | 4);
    d.push(w.condition_target);
    d.push(1);
    f32s(&mut d, &[2.0]);
    d.push(0);
    u32s(&mut d, &[0]);
    d.push(2);
    u32s(&mut d, &[1, 2]);
    // two block groups
    u32s(&mut d, &[2]);
    string(&mut d, "A");
    string(&mut d, "B");
    // one embedded block model; one material model file (node 3)
    u32s(&mut d, &[1, 1, 3]);
    string(&mut d, "Materials\\Stone.Material.Gbx");
    // replacement textures: specular and white (nodes 4 and 5)
    d.push(1 | 4);
    u32s(&mut d, &[w.specular, 5]);
    // one embedded image used as a bitmap (node 6)
    u32s(&mut d, &[1, 0x0900_B000]);
    string(&mut d, "Img\\a.dds");
    u32s(&mut d, &[3]);
    d.extend_from_slice(&[7, 8, 9]);
    u32s(&mut d, &[1, 6]);
    d.extend_from_slice(&[1, 2]);
    // the embedded block model (data version 2 for chunk 0x3F001003): a classic block with a checkpoint trigger
    u32s(&mut d, &[0x4000_0000]);
    string(&mut d, "MyBlock");
    u32s(&mut d, &[NONE]);
    d.push(2 | (w.waypoint << 3) | 64);
    d.extend_from_slice(&[1, 1, 1, 1, 1, 1]);
    // ground: one variation (a tree, node 1; the trigger tree, file node 3; a byte), air: none
    d.push(1);
    u32s(&mut d, &[1, 0x0904_F000, FACADE, 3]);
    if id == 0x3F00_1003 {
        d.push(9);
    }
    d.push(0);
    f32s(
        &mut d,
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
    );
    u32s(&mut d, &[1, 0, 1, 2, 3]);
    // one vehicle
    u32s(&mut d, &[1, 0x4000_0000]);
    string(&mut d, "CarSport");
    u32s(&mut d, &[NONE, NONE]);
    // four blocks
    u32s(&mut d, &[4]);
    // a game block with an offset, a motion, groups, a manual spawn point, respawn and trigger settings
    d.push(w.block_type);
    u32s(&mut d, &[0x4000_0001, 26, 1, 2, 3]);
    d.push(1);
    u32s(&mut d, &[0]);
    d.extend_from_slice(&w.block_flags.to_le_bytes());
    f32s(&mut d, &[10.0, 20.0, 30.0]);
    d.push(2);
    u32s(&mut d, &[1, 250]);
    f32s(&mut d, &[1.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
    u32s(&mut d, &[1, w.block_group]);
    f32s(&mut d, &[0.5, 0.5, 0.5, 0.25, 0.25, 0.25]);
    // a trigger block
    d.push(1);
    u32s(&mut d, &[4, 5, 6]);
    d.push(0);
    u32s(&mut d, &[1, w.trigger_group]);
    d.push(2 | 16 | 32);
    f32s(&mut d, &[9.0, 9.0, 9.0]);
    u32s(&mut d, &[1, 0]);
    // an external block
    d.push(2);
    u32s(&mut d, &[0x4000_0000]);
    string(&mut d, "Ext");
    u32s(&mut d, &[NONE, 0, 0, 0]);
    d.push(0);
    u32s(&mut d, &[7]);
    d.extend_from_slice(&0u16.to_le_bytes());
    // an embedded block with a scale and an origin offset
    d.push(3);
    u32s(&mut d, &[w.embedded, 8, 8, 8]);
    d.push(3);
    u32s(&mut d, &[0]);
    d.extend_from_slice(&(4u16 | 16).to_le_bytes());
    f32s(&mut d, &[3.0, 3.0, 3.0, 0.0, 1.0, 0.0]);
    d.extend_from_slice(&w.extra);
    skippable(id, &d)
}

const NODES: u32 = 7;

#[test]
fn version_7_reads() {
    let nodes = read(&v7(0x3F00_1003, &Wrong::good()), NODES).unwrap();
    let u = map(&nodes).unlimiter.as_ref().unwrap();
    assert_eq!((u.chunk, u.version), (0x3F00_1003, 7));
    let d = u.decoration.as_ref().unwrap();
    assert_eq!(
        (d.visibility, d.offset, d.scale),
        (2, Some([5.0, 6.0, 7.0]), None)
    );
    assert!(u.pylons_disabled && u.vehicle_collisions && u.ignore_multiplayer_time_sync);
    assert!(!u.track_base_empty && !u.random_start_line && !u.server_communication);
    let names: Vec<_> = u
        .angel_script_modules
        .iter()
        .map(|m| m.module_name.clone())
        .collect();
    assert_eq!(names, [Some("Main".to_owned()), None]);

    let p = &u.parameter_sets[0].parameters;
    assert_eq!(u.parameter_sets[0].name, "Boost");
    assert_eq!(
        (p[0].operation, &p[0].value),
        (Operation::Set, &ParameterValue::Float(1.5))
    );
    assert_eq!(
        (p[1].operation, &p[1].value),
        (Operation::Execute, &ParameterValue::String("go".into()))
    );
    let ParameterValue::Keys(keys) = &p[2].value else {
        panic!()
    };
    assert_eq!(
        (keys.xs.clone(), keys.ys.clone(), keys.real_interp),
        (Some(vec![0.0, 1.0]), Some(vec![5.0, 7.0]), Some(0))
    );
    assert_eq!(keys.id, Some(Id::Empty));
    assert_eq!(
        (p[3].operation, &p[3].value),
        (Operation::Multiply, &ParameterValue::Multiplier(0.5))
    );
    assert_eq!(p[4].value, ParameterValue::Natural(5));
    assert_eq!(p[5].value, ParameterValue::Buffer(vec![3.0, 2.0]));
    assert_eq!(p[6].value, ParameterValue::Bool(true));
    assert_eq!(
        (p[7].operation, &p[7].value),
        (Operation::Execute, &ParameterValue::None)
    );
    assert_eq!(kind(p[7].function), Some(ValueKind::None));

    let g = &u.trigger_groups[0];
    assert_eq!(g.name, "Zone");
    assert_eq!(
        g.condition,
        Some(Condition::MediaTracker {
            condition_type: 1,
            value: Some(2.0)
        })
    );
    assert_eq!(g.on_enter, Some(Event::ParameterSet(0)));
    assert_eq!(
        g.on_inside,
        Some(Event::AngelScript {
            module: 1,
            function: 2
        })
    );
    assert_eq!(g.on_leave, None);
    assert_eq!(u.block_groups, ["A", "B"]);

    // the files the blocks name are nodes the body never reads
    assert_eq!(u.material_model_refs[0].instance_index, 3);
    assert_eq!(
        u.material_model_refs[0].relative_path,
        "Materials\\Stone.Material.Gbx"
    );
    assert_eq!(
        (u.replacement_textures.specular, u.replacement_textures.normal),
        (Some(4), None)
    );
    assert_eq!(
        (u.replacement_textures.white, u.replacement_textures.black),
        (Some(5), None)
    );
    let image = &u.embedded_images[0];
    assert_eq!(
        (image.class_id, image.relative_path.as_str(), &image.data[..]),
        (0x0900_B000, "Img\\a.dds", &[7, 8, 9][..])
    );
    assert_eq!(
        (
            image.bitmap_pairs[0].instance_index,
            image.bitmap_pairs[0].tex_filter,
            image.bitmap_pairs[0].tex_address
        ),
        (6, 1, 2)
    );
    for external in [3, 4, 5, 6] {
        assert!(nodes.get(NodeIndex(external)).is_none(), "node {external}");
    }
    assert!(nodes.get(NodeIndex(1)).is_some());

    let m = &u.embedded_blocks[0];
    assert_eq!(
        (m.id.clone(), m.author.clone()),
        (Id::Name("MyBlock".into()), Id::Empty)
    );
    assert_eq!((m.block_type, m.waypoint), (2, 2));
    assert_eq!(
        m.icon.as_ref().map(|i| (i.width, i.height, i.pixels.len())),
        Some((1, 1, 4))
    );
    assert_eq!(m.variations.len(), 1);
    let v = &m.variations[0][0][0];
    assert_eq!(
        (v.tree, v.trigger_tree, v.pre_light_gen_tile_count_u),
        (Some(NodeIndex(1)), Some(Some(NodeIndex(3))), Some(9))
    );
    assert!(m.variations[0][1].is_empty());
    assert_eq!(m.spawn_offset_air, [7.0, 8.0, 9.0]);
    assert_eq!(m.ground_block_units, Some(vec![[1, 2, 3]]));
    assert_eq!(m.air_block_units, Some(vec![]));
    assert_eq!(u.vehicle_identifiers[0].id, Id::Name("CarSport".into()));

    assert_eq!(u.blocks.len(), 4);
    let p = &u.blocks[0].properties;
    assert_eq!(p.offset, Some([10.0, 20.0, 30.0]));
    let motion = p.motion.as_ref().unwrap();
    assert_eq!((motion.wave_type, motion.points.as_ref().unwrap().len()), (2, 1));
    assert_eq!(motion.points.as_ref().unwrap()[0].time, 250);
    assert_eq!(p.block_groups, [1]);
    assert!(p.dynamic && !p.invisible && !p.non_collidable);
    assert_eq!(
        (p.spawn_method, p.spawn_offset, p.spawn_rotation),
        (1, Some([0.5; 3]), Some([0.25; 3]))
    );
    assert_eq!((p.respawn, p.trigger_activation), (2, 1));
    let BlockKind::Trigger {
        coord,
        direction,
        trigger_groups,
    } = &u.blocks[1].kind
    else {
        panic!()
    };
    assert_eq!(
        (*coord, *direction, trigger_groups.clone()),
        ([4, 5, 6], 0, vec![0])
    );
    let p = &u.blocks[1].properties;
    assert_eq!(
        (p.rotation, p.block_groups.clone(), p.non_collidable),
        (Some([9.0; 3]), vec![0], true)
    );
    let BlockKind::External { name, flags, .. } = &u.blocks[2].kind else {
        panic!()
    };
    assert_eq!((name.clone(), *flags), (Id::Name("Ext".into()), 7));
    let BlockKind::Embedded {
        data_index,
        coord,
        direction,
        flags,
    } = &u.blocks[3].kind
    else {
        panic!()
    };
    assert_eq!((*data_index, *coord, *direction, *flags), (0, [8, 8, 8], 3, 0));
    let p = &u.blocks[3].properties;
    assert_eq!(
        (p.scale, p.origin_offset),
        (Some([3.0; 3]), Some([0.0, 1.0, 0.0]))
    );
    assert_eq!(
        nodes.get(NodeIndex(0)).map(|n| n.class_name()),
        Some("CGameCtnChallenge")
    );
}

#[test]
fn chunk_1002_stores_embedded_blocks_without_the_byte_after_a_variation() {
    let nodes = read(&v7(0x3F00_1002, &Wrong::good()), NODES).unwrap();
    let u = map(&nodes).unlimiter.as_ref().unwrap();
    assert_eq!((u.chunk, u.version), (0x3F00_1002, 7));
    assert_eq!(
        u.embedded_blocks[0].variations[0][0][0].pre_light_gen_tile_count_u,
        None
    );
    // the chunk of data version 2 read as that of data version 1 is refused, the bytes being those of the other
    let mut d = v7(0x3F00_1003, &Wrong::good());
    d[0..4].copy_from_slice(&0x3F00_1002u32.to_le_bytes());
    assert!(read(&d, NODES).is_err());
}

#[test]
fn version_7_refusals() {
    let ok = Wrong::good();
    let mut w;
    macro_rules! wrong {
        ($($field:ident: $value:expr),+; $expect:expr) => {
            w = ok.clone();
            $(w.$field = $value;)+
            refused(&v7(0x3F00_1003, &w), NODES, $expect);
        };
    }
    wrong!(flags: 1 << 10; "bits above 9");
    wrong!(module_flags: 0; "bytecode");
    wrong!(function: NAMES.len() as u32; "not in the enum");
    wrong!(operation: 3; "parameter operation 3");
    wrong!(trigger_group: 1; "trigger group 1: there are 1");
    wrong!(block_group: 2; "block group 2: there are 2");
    wrong!(embedded: 1; "embedded block data 1: the map has 1");
    wrong!(block_type: 4; "block type 4");
    wrong!(block_flags: 1 << 15; "bit 15");
    wrong!(block_flags: 3 << 9; "spawn method 3");
    wrong!(block_flags: 3 << 11; "respawn capability or trigger activation 3");
    wrong!(condition_target: 2; "trigger condition target 2");
    wrong!(waypoint: 6; "waypoint type 6");
    // a replacement texture whose index is a node the body reads, or one the header does not count
    wrong!(specular: 3; "specular texture node index 3");
    wrong!(specular: 7; "specular texture node index 7");
    wrong!(specular: u32::MAX; "specular texture node index -1");
    // bytes left in the chunk
    w = ok.clone();
    w.extra = vec![0];
    refused(&v7(0x3F00_1003, &w), NODES, "bytes left");
    // every truncation of the chunk
    let whole = v7(0x3F00_1003, &ok);
    for n in 1..whole.len() {
        assert!(
            Gbx::parse(&file(&whole[..n], NODES))
                .and_then(|g| g.nodes())
                .is_err(),
            "prefix {n}"
        );
    }
}

// ---- the encrypted chunk ----

/// The crypt of chunk `0x3F001000`, written again here as GBX.NET writes it (with 64-bit arithmetic where it has C#'s
/// widening).
fn crypt(data: &[u8]) -> Vec<u8> {
    let len = data.len() as u64;
    data.iter()
        .enumerate()
        .map(|(i, &byte)| {
            let offset = i as u64;
            let mut hash = ((len * (len * 2 - offset)) & 0xFFFF_FFFF) as u32;
            hash ^= 0xEAD9_C8B3;
            hash = hash.wrapping_add((offset * 3 % 0x7F) as u32);
            if offset % 5 < 2 {
                hash = !hash;
            }
            !(byte ^ hash as u8)
        })
        .collect()
}

/// The legacy block data of a map of three blocks, then the encrypted chunk made of `plain`.
fn encrypted_map(plain: &[u8]) -> Vec<u8> {
    let mut c = Vec::new();
    u32s(&mut c, &[0x0304_3013, 3, 0x4000_0000]);
    string(&mut c, "uid");
    u32s(&mut c, &[26, NONE]);
    string(&mut c, "Map");
    u32s(&mut c, &[NONE, 26, NONE, 32, 32, 32, 0, 3]);
    for i in 0..3u8 {
        u32s(&mut c, &[if i == 0 { 0x4000_0000 } else { 0x4000_0002 }]);
        if i == 0 {
            string(&mut c, "Block");
        }
        c.extend_from_slice(&[i, i, i, 1]);
        c.extend_from_slice(&0u16.to_le_bytes());
    }
    c.extend_from_slice(&skippable(0x3F00_1000, &crypt(plain)));
    c
}

fn plain(block_index: u32, block_flags: u16, challenge_flags: u16, trailing: &[u8]) -> Vec<u8> {
    let mut p = Vec::new();
    p.extend_from_slice(&challenge_flags.to_le_bytes());
    f32s(&mut p, &[1.0, 2.0, 3.0]);
    u32s(&mut p, &[2]);
    // a block outside the boundaries, moved, rotated, inverted, in a group
    u32s(&mut p, &[block_index]);
    p.extend_from_slice(&block_flags.to_le_bytes());
    p.extend_from_slice(&[9, 8, 7]);
    f32s(&mut p, &[4.0, 5.0, 6.0, 0.0, 1.0, 0.0]);
    string(&mut p, "Grp");
    // a vanilla terrain block: nothing more, whatever else its flags say
    u32s(&mut p, &[2]);
    p.extend_from_slice(&((1u16 << 5) | (1 << 7) | (1 << 1)).to_le_bytes());
    u32s(&mut p, &[42]);
    p.extend_from_slice(trailing);
    p
}

const BLOCK_FLAGS: u16 = 1 | 2 | 4 | 16 | (1 << 14);
const CHALLENGE_FLAGS: u16 = 1 | 2 | 16 | 512;

#[test]
fn the_crypt_is_its_own_inverse() {
    let data: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
    assert_ne!(crypt(&data), data);
    assert_eq!(crypt(&crypt(&data)), data);
}

#[test]
fn the_encrypted_chunk_reads() {
    let nodes = read(&encrypted_map(&plain(0, BLOCK_FLAGS, CHALLENGE_FLAGS, &[])), 1).unwrap();
    let m = map(&nodes);
    assert!(m.unlimiter.is_none());
    let e = m.unlimiter_encrypted.as_ref().unwrap();
    assert_eq!(e.flags, CHALLENGE_FLAGS);
    assert_eq!(
        (e.decoration_offset, e.decoration_scale),
        (Some([1.0, 2.0, 3.0]), None)
    );
    assert_eq!(e.u01, 42);
    assert_eq!(e.blocks.len(), 2);
    let b = &e.blocks[0];
    assert_eq!((b.index, b.flags), (0, BLOCK_FLAGS));
    assert_eq!(b.outside_boundaries, Some([9, 8, 7]));
    assert_eq!(
        (b.offset, b.rotation, b.scale),
        (Some([4.0, 5.0, 6.0]), Some([0.0, 1.0, 0.0]), None)
    );
    assert_eq!(b.group.as_deref(), Some("Grp"));
    let b = &e.blocks[1];
    assert_eq!((b.index, b.offset, b.group.clone()), (2, None, None));
    assert_eq!(m.chunks.ids, [0x0304_3013, 0x3F00_1000]);
}

#[test]
fn the_encrypted_chunk_refusals() {
    // a block the map does not have
    refused(
        &encrypted_map(&plain(3, BLOCK_FLAGS, CHALLENGE_FLAGS, &[])),
        1,
        "block 3: the map has 3 blocks",
    );
    // flags the reader does not know
    refused(
        &encrypted_map(&plain(0, BLOCK_FLAGS | (1 << 15), CHALLENGE_FLAGS, &[])),
        1,
        "bits 12, 13 and 15",
    );
    refused(
        &encrypted_map(&plain(0, BLOCK_FLAGS | (1 << 12), CHALLENGE_FLAGS, &[])),
        1,
        "bits 12, 13 and 15",
    );
    refused(
        &encrypted_map(&plain(0, BLOCK_FLAGS, CHALLENGE_FLAGS | (1 << 6), &[])),
        1,
        "challenge flags",
    );
    // visibility "nothing" with a moved decoration: GBX.NET and the flag's name disagree about it
    refused(
        &encrypted_map(&plain(0, BLOCK_FLAGS, CHALLENGE_FLAGS | 4, &[])),
        1,
        "visibility nothing",
    );
    // a byte too many, and a byte too few
    refused(
        &encrypted_map(&plain(0, BLOCK_FLAGS, CHALLENGE_FLAGS, &[0])),
        1,
        "left",
    );
    let short = plain(0, BLOCK_FLAGS, CHALLENGE_FLAGS, &[]);
    refused(&encrypted_map(&short[..short.len() - 1]), 1, "");
    // a chunk stored as it would be without the crypt
    let mut c = encrypted_map(&[]);
    let n = c.len();
    let plain_chunk = skippable(0x3F00_1000, &plain(0, BLOCK_FLAGS, CHALLENGE_FLAGS, &[]));
    c.truncate(n - skippable(0x3F00_1000, &[]).len());
    c.extend_from_slice(&plain_chunk);
    assert!(read(&c, 1).is_err());
}
