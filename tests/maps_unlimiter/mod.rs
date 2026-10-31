//! The comparison of a map's TMUnlimiter data with what GBX.NET 2.5.0 read (`map.tmunlimiter_data` and each block's
//! `tmu` in the dump). GBX.NET keeps floats here as their IEEE bits, so they compare exactly, and hands the data out
//! by kind of block (game, trigger, external, embedded), each list in file order.
#![allow(
    clippy::unwrap_used,
    reason = "a test reading the files you point it at; the library does no I/O"
)]

use super::{author_skin, id, ident};
use gbx_formats::game::challenge::Challenge;
use gbx_formats::game::unlimiter::{
    BlockKind, BlockProperties, ClipResource, Condition, EmbeddedBlockData, EncryptedUnlimiter, Event,
    Motion, Operation, Parameter, ParameterValue, Unlimiter, UnlimiterBlock,
};
use gbx_formats::{Id, Node, NodeIndex, Nodes};
use serde_json::{Value, json};

type Check<'a> = dyn FnMut(&str, Value, &Value) + 'a;

fn bits(f: f32) -> Value {
    json!(f.to_bits())
}

fn vec3(v: [f32; 3]) -> Value {
    json!(v.map(f32::to_bits))
}

fn vec3_or(v: Option<[f32; 3]>, default: [f32; 3]) -> Value {
    vec3(v.unwrap_or(default))
}

fn vec3_opt(v: Option<[f32; 3]>) -> Value {
    v.map_or(Value::Null, vec3)
}

fn motion(m: &Option<Motion>) -> Value {
    match m {
        None => Value::Null,
        // GBX.NET's manually controlled motion keeps an empty point list
        Some(m) => json!({
            "wave": m.wave_type,
            "points": m.points.as_ref().map_or_else(Vec::new, |p| {
                p.iter().map(|p| json!([p.time, vec3(p.offset), vec3(p.rotation)])).collect()
            }),
        }),
    }
}

fn groups(u: &Unlimiter, p: &BlockProperties) -> Value {
    json!(
        p.block_groups
            .iter()
            .map(|&g| u.block_groups[g as usize].clone())
            .collect::<Vec<_>>()
    )
}

const ZERO: [f32; 3] = [0.0; 3];
const ONE: [f32; 3] = [1.0; 3];

/// The fields of GBX.NET's `TMUnlimiter.Block` that every kind of block shares.
fn common(
    u: &Unlimiter,
    coord: [i32; 3],
    direction: u8,
    p: &BlockProperties,
) -> serde_json::Map<String, Value> {
    let Value::Object(map) = json!({
        "coord": coord,
        "dir": direction,
        "offset": vec3_or(p.offset, ZERO),
        "rotation": vec3_or(p.rotation, ZERO),
        "scale": vec3_or(p.scale, ONE),
        "motion": motion(&p.motion),
        "origin_offset": vec3_opt(p.origin_offset),
        "block_groups": groups(u, p),
        "spawn_method": p.spawn_method,
        "spawn_offset": vec3_opt(p.spawn_offset),
        "spawn_rotation": vec3_opt(p.spawn_rotation),
        "non_collidable": p.non_collidable,
        "dynamic": p.dynamic,
        "invisible": p.invisible,
        "inverted": p.inverted,
        "classic_terrain": p.classic_terrain,
        "respawn": p.respawn,
        "trigger_activation": p.trigger_activation,
    }) else {
        unreachable!()
    };
    map
}

/// The fields of GBX.NET's `CGameCtnBlock.TMUnlimiter` of a game block.
fn game_tmu(u: &Unlimiter, p: &BlockProperties) -> Value {
    json!({
        "outside": [0, 0, 0],
        "offset": vec3_or(p.offset, ZERO),
        "rotation": vec3_or(p.rotation, ZERO),
        "scale": vec3_or(p.scale, ONE),
        "group": null,
        "inverted": p.inverted,
        "vanilla_terrain": false,
        "spawn_point_fix": false,
        "dynamic": p.dynamic,
        "invisible": p.invisible,
        "collision_disabled": false,
        "classic_mode": false,
        "classic_terrain": p.classic_terrain,
        "motion": motion(&p.motion),
        "origin_offset": vec3_opt(p.origin_offset),
        "block_groups": groups(u, p),
        "spawn_method": p.spawn_method,
        "spawn_offset": vec3_opt(p.spawn_offset),
        "spawn_rotation": vec3_opt(p.spawn_rotation),
        "non_collidable": p.non_collidable,
        "respawn": p.respawn,
        "trigger_activation": p.trigger_activation,
    })
}

fn event(e: &Option<Event>) -> Value {
    match e {
        None => Value::Null,
        Some(Event::ParameterSet(i)) => json!([0, i, null, null]),
        Some(Event::AngelScript { module, function }) => json!([2, null, module, function]),
    }
}

fn trigger_groups(u: &Unlimiter, indices: &[u32]) -> Value {
    json!(
        indices
            .iter()
            .map(|&i| {
                let g = &u.trigger_groups[i as usize];
                let condition = match g.condition {
                    None => Value::Null,
                    Some(Condition::MediaTracker {
                        condition_type,
                        value,
                    }) => {
                        json!([0, condition_type, value.map(f32::to_bits), null, null])
                    }
                    Some(Condition::AngelScript { module, function }) => {
                        json!([1, null, null, module, function])
                    }
                };
                let flags = u8::from(g.condition.is_some())
                    | u8::from(g.on_enter.is_some()) << 1
                    | u8::from(g.on_inside.is_some()) << 2
                    | u8::from(g.on_leave.is_some()) << 3;
                json!([
                    g.name,
                    flags,
                    condition,
                    event(&g.on_enter),
                    event(&g.on_inside),
                    event(&g.on_leave)
                ])
            })
            .collect::<Vec<_>>()
    )
}

fn parameter(p: &Parameter) -> Value {
    let value = match &p.value {
        ParameterValue::None => Value::Null,
        ParameterValue::Float(f) => bits(*f),
        ParameterValue::String(s) => json!(s),
        // GBX.NET keeps a natural parameter's multiplier as the whole number it truncates to, and a buffer
        // parameter's as a one-element list
        ParameterValue::Multiplier(f) => match gbx_formats::game::parameters::kind(p.function) {
            Some(gbx_formats::game::parameters::ValueKind::Natural) => json!(*f as u32),
            Some(gbx_formats::game::parameters::ValueKind::FastBufferReal) => json!([f.to_bits()]),
            _ => bits(*f),
        },
        ParameterValue::Keys(k) => {
            let list = |v: &Option<Vec<f32>>| {
                v.as_ref()
                    .map(|v| v.iter().map(|f| f.to_bits()).collect::<Vec<_>>())
            };
            json!([list(&k.xs), list(&k.ys)])
        }
        ParameterValue::Natural(n) => json!(n),
        ParameterValue::Buffer(b) => json!(b.iter().map(|f| f.to_bits()).collect::<Vec<_>>()),
        ParameterValue::Bool(b) => json!(b),
    };
    let operation = match p.operation {
        Operation::Execute => 0,
        Operation::Set => 1,
        Operation::Multiply => 2,
    };
    json!({"function": p.function, "operation": operation, "value": value})
}

fn embedded_data(d: &EmbeddedBlockData) -> Value {
    // twelve lists: ground and air of each of six variants; absent ones are empty in GBX.NET
    let mut lists = vec![Vec::new(); 12];
    for (pair, lists2) in d.variations.iter().zip(lists.chunks_mut(2)) {
        for (list, slot) in pair.iter().zip(lists2) {
            *slot = list
                .iter()
                .map(|v| {
                    json!([
                        v.tree.is_some(),
                        v.trigger_tree.flatten().is_some(),
                        v.pre_light_gen_tile_count_u
                    ])
                })
                .collect();
        }
    }
    let units = |u: &Option<Vec<[i32; 3]>>| u.as_ref().map_or(Value::Null, |u| json!(u));
    json!({
        "id": id(&d.id),
        "author": id(&d.author),
        "u01": d.u01.unwrap_or(0),
        "block_type": d.block_type,
        "waypoint": d.waypoint,
        "icon": d.icon.as_ref().map(|i| json!([i.width, i.height, i.pixels.len()])),
        "sub_variations": lists,
        "spawn_offset_ground": vec3(d.spawn_offset_ground),
        "spawn_rotation_ground": vec3(d.spawn_rotation_ground),
        "spawn_offset_air": vec3(d.spawn_offset_air),
        "spawn_rotation_air": vec3(d.spawn_rotation_air),
        "ground_units": units(&d.ground_block_units),
        "air_units": units(&d.air_block_units),
    })
}

/// GBX.NET's game blocks (`map.blocks`, which TMUnlimiter's replace the map's): its fields and its `tmu`.
fn game_block(u: &Unlimiter, b: &UnlimiterBlock, nodes: &Nodes) -> Value {
    let BlockKind::Game(g) = &b.kind else {
        unreachable!()
    };
    let (author, skin) = author_skin(g.skin.as_ref(), nodes);
    json!({"name": id(&g.name), "dir": g.direction, "coord": g.coord, "flags": g.flags, "author": author,
           "skin": skin, "tmu": game_tmu(u, &b.properties)})
}

/// Whether the map's block data is TMUnlimiter's stand-in, replaced by the blocks of the chunk.
pub fn replaces_blocks(map: &Challenge) -> bool {
    map.unlimiter.as_ref().is_some_and(|u| u.version != 0)
}

/// Compares the map's TMUnlimiter data with the dump's. Chunk `0x3F001001` to `0x3F001003` of version 0 hold nothing
/// and GBX.NET reads no data from them.
pub fn compare(map: &Challenge, nodes: &Nodes, want: &Value, check: &mut Check<'_>) {
    let theirs = &want["map"]["tmunlimiter_data"];
    match (&map.unlimiter, &map.unlimiter_encrypted) {
        (Some(u), None) if u.version != 0 => {
            check(
                "unlimiter: GBX.NET read it",
                json!(true),
                &json!(!theirs.is_null()),
            );
            if !theirs.is_null() {
                compare_chunk(u, nodes, want, check);
            }
        }
        (None, Some(e)) => {
            check(
                "unlimiter: GBX.NET read it",
                json!(true),
                &json!(!theirs.is_null()),
            );
            if !theirs.is_null() {
                compare_encrypted(e, map, nodes, want, check);
            }
        }
        (None, None) | (Some(_), None) => {
            check(
                "unlimiter: GBX.NET read none",
                json!(true),
                &json!(theirs.is_null()),
            );
        }
        (Some(_), Some(_)) => unreachable!("one map holds both"),
    }
}

fn compare_chunk(u: &Unlimiter, nodes: &Nodes, want: &Value, check: &mut Check<'_>) {
    let t = &want["map"]["tmunlimiter_data"];
    check(
        "unlimiter chunk",
        json!(format!("0x{:08X}", u.chunk)),
        &t["chunk"],
    );
    check("unlimiter version", json!(u.version), &t["version"]);
    let d = u.decoration.clone().unwrap_or_default();
    check("decoration visibility", json!(d.visibility), &t["visibility"]);
    check(
        "decoration offset",
        vec3_or(d.offset, ZERO),
        &t["decoration_offset"],
    );
    check("decoration scale", vec3_or(d.scale, ONE), &t["decoration_scale"]);
    check(
        "track base empty",
        json!(u.track_base_empty),
        &t["track_base_empty"],
    );
    check("pylons disabled", json!(u.pylons_disabled), &t["pylons_disabled"]);
    check("vanilla mode", json!(false), &t["vanilla_mode"]);
    check(
        "vehicle collisions",
        json!(u.vehicle_collisions),
        &t["vehicle_collisions"],
    );
    check(
        "random start line",
        json!(u.random_start_line),
        &t["random_start_line"],
    );
    check(
        "server communication",
        json!(u.server_communication),
        &t["server_communication"],
    );
    check(
        "ignore time sync",
        json!(u.ignore_multiplayer_time_sync),
        &t["ignore_time_sync"],
    );
    let modules: Vec<Value> = u
        .angel_script_modules
        .iter()
        .map(|m| json!([1 | if m.module_name.is_none() { 2 } else { 0 }, m.module_name]))
        .collect();
    check("angel script modules", json!(modules), &t["angel_script_modules"]);
    let sets: Vec<Value> = u
        .parameter_sets
        .iter()
        .map(
            |s| json!({"name": s.name, "parameters": s.parameters.iter().map(parameter).collect::<Vec<_>>()}),
        )
        .collect();
    check("parameter sets", json!(sets), &t["parameter_sets"]);
    let images: Vec<Value> = u
        .embedded_images
        .iter()
        .map(|i| {
            let pairs: Vec<Value> = i
                .bitmap_pairs
                .iter()
                .map(|b| json!([b.tex_filter, b.tex_address]))
                .collect();
            json!([i.class_id, i.relative_path, i.data.len(), pairs])
        })
        .collect();
    check("embedded images", json!(images), &t["embedded_images"]);
    let vehicles: Vec<Value> = u.vehicle_identifiers.iter().map(ident).collect();
    check("vehicle identifiers", json!(vehicles), &t["vehicle_identifiers"]);

    let (mut games, mut triggers, mut externals, mut embeddeds) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    // GBX.NET hands out only the embedded models some block uses, in the order the blocks first use them
    let mut used: Vec<u32> = Vec::new();
    for b in &u.blocks {
        let p = &b.properties;
        match &b.kind {
            BlockKind::Game(_) => games.push(game_block(u, b, nodes)),
            BlockKind::Trigger {
                coord,
                direction,
                trigger_groups: g,
            } => {
                let mut c = common(u, *coord, *direction, p);
                c.insert("trigger_groups".into(), trigger_groups(u, g));
                triggers.push(Value::Object(c));
            }
            BlockKind::External {
                name,
                author,
                coord,
                direction,
                flags,
            } => {
                let mut c = common(u, *coord, *direction, p);
                c.insert("name".into(), id(name));
                c.insert("author".into(), id(author));
                c.insert("flags".into(), json!(flags));
                externals.push(Value::Object(c));
            }
            BlockKind::Embedded {
                data_index,
                coord,
                direction,
                flags,
            } => {
                if !used.contains(data_index) {
                    used.push(*data_index);
                }
                let mut c = common(u, *coord, *direction, p);
                c.insert("data".into(), json!(used.iter().position(|d| d == data_index)));
                c.insert("flags".into(), json!(flags));
                embeddeds.push(Value::Object(c));
            }
        }
    }
    let theirs: Vec<Value> = t_blocks(want);
    check("game block count", json!(games.len()), &json!(theirs.len()));
    for (i, (o, t)) in games.iter().zip(&theirs).enumerate() {
        check(&format!("game block {i}"), o.clone(), t);
    }
    check("trigger blocks", json!(triggers), &t["trigger_blocks"]);
    check("external blocks", json!(externals), &t["external_blocks"]);
    check("embedded blocks", json!(embeddeds), &t["embedded_blocks"]);
    let data: Vec<Value> = used
        .iter()
        .map(|&i| embedded_data(&u.embedded_blocks[i as usize]))
        .collect();
    check("embedded block data", json!(data), &t["embedded_data"]);

    // the clips the legacy resources are mapped to
    let clips: Vec<Value> = u
        .clip_mappings
        .iter()
        .map(|m| match m.resource {
            ClipResource::ParameterSet(i) => {
                let s = &u.legacy_parameter_sets[i as usize];
                json!([m.clip_index, "parameter_set", s.name, s.parameters.len()])
            }
            ClipResource::LegacyScript(i) => {
                let s = &u.legacy_scripts[i as usize];
                json!([m.clip_index, "script", s.name, s.byte_code.len()])
            }
        })
        .collect();
    check("clip resources", json!(clips), &t["clip_resources"]);
}

/// The oracle's game blocks, as GBX.NET lists them for a map whose blocks TMUnlimiter replaced.
fn t_blocks(want: &Value) -> Vec<Value> {
    want["map"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            json!({"name": b["name"], "dir": b["dir"], "coord": b["coord"], "flags": b["flags"],
                   "author": b["author"], "skin": b["skin"], "tmu": b["tmu"]})
        })
        .collect()
}

fn compare_encrypted(
    e: &EncryptedUnlimiter,
    map: &Challenge,
    nodes: &Nodes,
    want: &Value,
    check: &mut Check<'_>,
) {
    let _ = nodes;
    let t = &want["map"]["tmunlimiter_data"];
    check("unlimiter chunk", json!("0x3F001000"), &t["chunk"]);
    check("unlimiter u01", json!(e.u01), &t["u01"]);
    // GBX.NET's visibility is the flags masked with 0x105
    check("decoration visibility", json!(e.flags & 0x105), &t["visibility"]);
    check(
        "decoration offset",
        vec3_or(e.decoration_offset, ZERO),
        &t["decoration_offset"],
    );
    check(
        "decoration scale",
        vec3_or(e.decoration_scale, ONE),
        &t["decoration_scale"],
    );
    check(
        "track base empty",
        json!(e.flags & (1 << 4) != 0),
        &t["track_base_empty"],
    );
    check("vanilla mode", json!(e.flags & (1 << 5) != 0), &t["vanilla_mode"]);
    check(
        "pylons disabled",
        json!(e.flags & (1 << 9) != 0),
        &t["pylons_disabled"],
    );

    // each block of the map's block data carries the properties of its entry, if any
    let blocks = want["map"]["blocks"].as_array().unwrap();
    let data = map
        .block_data
        .as_ref()
        .or(map.legacy_block_data.as_ref())
        .unwrap();
    check("block count", json!(data.blocks.len()), &json!(blocks.len()));
    for (i, theirs) in blocks.iter().enumerate() {
        let entry = e.blocks.iter().find(|b| b.index as usize == i);
        let ours = entry.map_or(Value::Null, |b| {
            let f = |bit: u16| b.flags & (1 << bit) != 0;
            json!({
                "outside": b.outside_boundaries.unwrap_or([0; 3]),
                "offset": vec3_or(b.offset, ZERO),
                "rotation": vec3_or(b.rotation, ZERO),
                "scale": vec3_or(b.scale, ONE),
                "group": b.group,
                "inverted": f(4),
                "vanilla_terrain": f(5),
                "spawn_point_fix": f(6),
                "dynamic": f(7),
                "invisible": f(8),
                "collision_disabled": f(9),
                "classic_mode": f(10),
                "classic_terrain": f(11),
                "motion": null,
                "origin_offset": null,
                "block_groups": [],
                "spawn_method": 0,
                "spawn_offset": null,
                "spawn_rotation": null,
                "non_collidable": false,
                "respawn": 0,
                "trigger_activation": 0,
            })
        });
        check(&format!("block {i} unlimiter properties"), ours, &theirs["tmu"]);
    }
    let _: Option<Id> = None;
}

/// The nodes a node refers to (the Plug classes).
fn refs(node: &Node) -> Vec<NodeIndex> {
    let mut out = Vec::new();
    let tree = |t: &gbx_formats::plug::tree::TreeData, out: &mut Vec<NodeIndex>| {
        out.extend(t.children.iter().flatten().flatten());
        out.extend(t.model_solid.flatten());
        out.extend(t.func_tree.flatten());
        if let Some(c) = &t.content {
            out.extend([c.visual, c.shader, c.surface, c.generator].into_iter().flatten());
        }
    };
    match node {
        Node::Tree(t) => tree(&t.tree, &mut out),
        Node::TreeVisualMip(t) => {
            tree(&t.tree, &mut out);
            out.extend(t.levels.iter().flatten().filter_map(|l| l.tree));
        }
        Node::VisualIndexedTriangles(v) => {
            out.extend(v.visual.func_visual.flatten());
            out.extend(v.visual_3d.blend_shapes.flatten());
            if let Some(g) = &v.visual.geometry {
                out.extend(g.vertex_streams.iter().flatten());
            }
        }
        Node::Material(m) => {
            out.extend(m.unknown_node.flatten());
            out.extend(m.custom_material.flatten());
            out.extend(m.shader);
        }
        Node::MaterialCustom(m) => out.extend(m.textures.iter().flatten().filter_map(|b| b.texture)),
        Node::Surface(s) => out.extend(s.geom.flatten()),
        _ => {}
    }
    out
}

/// The indices of the nodes reachable from the trees of the embedded block models `models` (indices into the
/// chunk's list).
fn reachable(
    u: &Unlimiter,
    nodes: &Nodes,
    models: impl Iterator<Item = usize>,
) -> std::collections::BTreeSet<u32> {
    let mut seen = std::collections::BTreeSet::new();
    let mut todo: Vec<NodeIndex> = models
        .flat_map(|m| u.embedded_blocks[m].variations.iter().flatten().flatten())
        .flat_map(|v| v.tree.into_iter().chain(v.trigger_tree.flatten()))
        .collect();
    while let Some(i) = todo.pop() {
        if !seen.insert(i.0) {
            continue;
        }
        if let Some(n) = nodes.get(i) {
            todo.extend(refs(n));
        }
    }
    seen
}

/// The node census as GBX.NET's dump makes it: GBX.NET reaches the embedded models some block uses and drops the
/// others, so the nodes only the others reach are left out; the inline `CFuncKeysReal` nodes of the parameter sets,
/// which are in no node table, are counted.
pub fn census_adjust(map: &Challenge, nodes: &Nodes, census: &mut std::collections::BTreeMap<String, u64>) {
    let Some(u) = &map.unlimiter else { return };
    let used: std::collections::BTreeSet<usize> = u
        .blocks
        .iter()
        .filter_map(|b| match b.kind {
            BlockKind::Embedded { data_index, .. } => Some(data_index as usize),
            _ => None,
        })
        .collect();
    let all = reachable(u, nodes, 0..u.embedded_blocks.len());
    let kept = reachable(u, nodes, used.iter().copied());
    for i in all.difference(&kept) {
        // an external (a file the map names) has no node
        let Some(n) = nodes.get(NodeIndex(*i)) else {
            continue;
        };
        for c in &n.chunks().ids {
            let key = format!("{} 0x{c:08X}", n.class_name());
            *census.get_mut(&key).unwrap() -= 1;
        }
    }
    // the index buffers of the visuals GBX.NET reaches are in place too
    for i in &kept {
        if let Some(Node::VisualIndexedTriangles(v)) = nodes.get(NodeIndex(*i)) {
            for c in v.indexed.index_buffer.iter().flat_map(|b| &b.chunks.ids) {
                *census.entry(format!("CPlugIndexBuffer 0x{c:08X}")).or_insert(0) += 1;
            }
        }
    }
    for p in u.parameter_sets.iter().flat_map(|s| &s.parameters) {
        if let ParameterValue::Keys(k) = &p.value {
            for c in &k.chunks.ids {
                *census.entry(format!("CFuncKeysReal 0x{c:08X}")).or_insert(0) += 1;
            }
        }
    }
    census.retain(|_, n| *n > 0);
}
