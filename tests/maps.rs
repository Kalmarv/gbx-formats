//! Reads every TrackMania Nations Forever map of each corpus (`corpus/tmnf/<name>` of the GAME_DATA oracle root:
//! tmx-2026-top, tmx-alltime-top, tmx-newest, tmnf-stock) and compares it with what GBX.NET 2.5.0, a third-party
//! reader, read from the same file (`gbxnet/tmnf/<name>`): the container, the decompressed body (GBX.NET.LZO's), the
//! main node's chunk ids, each class's chunk census, and the map's fields and blocks. A map GBX.NET could not read is
//! reported, not compared.
#![allow(
    clippy::unwrap_used,
    reason = "a test reading the files you point it at; the library does no I/O"
)]

mod inputs;

use gbx_formats::game::block::Block;
use gbx_formats::game::challenge::{Challenge, Header};
use gbx_formats::game::collector::ChallengeParameters;
use gbx_formats::{Gbx, Id, Ident, Nodes};
use inputs::{Game, OracleSet, oracle};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn id(i: &Id) -> Value {
    match i {
        Id::Empty => json!(""),
        Id::Number(n) => json!(n.to_string()),
        Id::Name(s) => json!(s),
    }
}

/// The collection ids TMNF maps store as numbers, by the names GBX.NET gives them.
fn collection(i: &Id) -> Value {
    match i {
        Id::Number(26) => json!("Stadium"),
        other => id(other),
    }
}

fn ident(i: &Ident) -> Value {
    json!({"id": id(&i.id), "collection": collection(&i.collection), "author": id(&i.author)})
}

fn block(b: &Block, version: u32, nodes: &Nodes) -> Value {
    let offset = if version >= 6 { [1, 0, 1] } else { [0, 0, 0] };
    let coord: Vec<i64> = b
        .coord
        .iter()
        .zip(offset)
        .map(|(&c, o)| i64::from(c) - o)
        .collect();
    let (author, skin) = match &b.skin {
        None => (Value::Null, Value::Null),
        Some((author, skin)) => {
            let skin = skin.map(|i| nodes.get(i).unwrap()).map(|n| {
                let s: &gbx_formats::game::block::BlockSkin = n.try_into().unwrap();
                let (text, pack, parent) = s.skin.as_ref().unwrap();
                json!({"text": text, "pack": pack.file_path, "pack_url": pack.locator_url.as_deref().unwrap_or(""),
                       "parent_pack": parent.file_path})
            });
            (id(author), skin.unwrap_or(Value::Null))
        }
    };
    // GBX.NET reads version 0's 16-bit flags as a signed short, so a skin (bit 15) makes them negative there
    let flags = if version == 0 {
        i32::from(b.flags as u16 as i16)
    } else {
        b.flags as i32
    };
    json!({"name": id(&b.name), "dir": b.direction, "coord": coord, "flags": flags, "author": author,
           "skin": skin})
}

/// The oracle's block fields this crate keeps (`ground`, `variant` and `sub_variant` are GBX.NET's readings of
/// flag bits; `waypoint` is compared by presence).
fn oracle_block(b: &Value) -> Value {
    json!({"name": b["name"], "dir": b["dir"], "coord": b["coord"], "flags": b["flags"], "author": b["author"],
           "skin": b["skin"]})
}

/// The node census GBX.NET's dump makes: (class, chunk id) counted over every node, the main node's header chunks
/// included.
fn census(gbx: &Gbx<'_>, nodes: &Nodes) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for c in &gbx.header_chunks {
        *out.entry(format!("CGameCtnChallenge 0x{:08X}", c.id))
            .or_insert(0) += 1;
    }
    for (_, n) in nodes.iter() {
        for chunk in &n.chunks().ids {
            *out.entry(format!("{} 0x{chunk:08X}", n.class_name()))
                .or_insert(0) += 1;
        }
    }
    out
}

/// Reads one map and compares it with GBX.NET's reading; when GBX.NET refused the map (`want["ok"]` false), only the
/// decompressed body is compared.
fn compare(path: &Path, want: &Value, decompressed: &Path, failures: &mut Vec<String>) -> Result<(), String> {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let bytes = std::fs::read(path).unwrap();
    let gbx = Gbx::parse(&bytes).map_err(|e| format!("container: {e}"))?;
    let header = Header::read(&gbx).map_err(|e| format!("header: {e}"))?;
    let body = gbx.body().map_err(|e| format!("body: {e}"))?;
    let mut check = |what: &str, ours: Value, theirs: &Value| {
        if ours != *theirs {
            let (o, t) = match (ours.as_object(), theirs.as_object()) {
                // a census: only the entries that differ
                (Some(a), Some(b)) => {
                    let keys: std::collections::BTreeSet<_> = a.keys().chain(b.keys()).collect();
                    let differ =
                        |m: &serde_json::Map<String, Value>, k: &String| m.get(k).map(Value::to_string);
                    let diff = |m: &serde_json::Map<String, Value>,
                                other: &serde_json::Map<String, Value>| {
                        keys.iter()
                            .filter(|k| differ(m, k) != differ(other, k))
                            .map(|k| format!("{k}={}", differ(m, k).unwrap_or("-".into())))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    (diff(a, b), diff(b, a))
                }
                _ => (ours.to_string(), theirs.to_string()),
            };
            failures.push(format!("{name}: {what}: ours {:.300} GBX.NET {:.300}", o, t));
        }
    };

    // the body GBX.NET.LZO decompressed, inside the file its Gbx.Decompress wrote
    let theirs = std::fs::read(decompressed.join(&name)).unwrap();
    let theirs = Gbx::parse(&theirs).map_err(|e| format!("GBX.NET's decompressed file: {e}"))?;
    match theirs.body {
        gbx_formats::Body::Uncompressed { data, .. } => {
            check("decompressed body", json!(body.as_ref() == data), &json!(true))
        }
        other => return Err(format!("GBX.NET's decompressed file has a {other:?} body")),
    }
    if want["ok"] != json!(true) {
        gbx.read_nodes(&body).map_err(|e| format!("nodes: {e}"))?;
        return Ok(());
    }
    check("version", json!(gbx.version), &want["header"]["version"]);
    check("nodes", json!(gbx.nodes), &want["header"]["num_nodes"]);
    check(
        "release",
        json!(gbx.release),
        &json!(want["header"]["mode"] == "Release"),
    );
    check("body size", json!(body.len()), &want["body"]["uncompressed_size"]);

    let nodes = gbx.read_nodes(&body).map_err(|e| format!("nodes: {e}"))?;
    let map: &Challenge = nodes
        .main()
        .try_into()
        .map_err(|_| "main node is not a map".to_string())?;
    let mut chunks: Vec<String> = gbx
        .header_chunks
        .iter()
        .map(|c| format!("0x{:08X}", c.id))
        .collect();
    chunks.extend(map.chunks.ids.iter().map(|c| format!("0x{c:08X}")));
    check("chunks", json!(chunks), &want["chunks"]);
    let census = census(&gbx, &nodes);
    check("node chunks", json!(census), &want["node_chunks"]);

    let m = &want["map"];
    let common = header.common.as_ref().ok_or("no header chunk 0x03043003")?;
    let desc = header.description.as_ref().ok_or("no header chunk 0x03043002")?;
    check("uid", id(&common.map_info.id), &m["uid"]);
    check("author", id(&common.map_info.author), &m["author"]);
    check("name", json!(common.name), &m["name"]);
    check("map info", ident(&common.map_info), &m["map_info"]);
    check("decoration", ident(&common.decoration), &m["decoration"]);
    // GBX.NET's medal times are the parameters node's when the body has one, the header's otherwise; the two
    // differ in a few maps
    let params: Option<&ChallengeParameters> = map
        .parameters
        .and_then(|p| p.challenge_parameters)
        .map(|p| nodes.get(p).unwrap().try_into().unwrap());
    let medals = params.and_then(|p| p.medals).map_or(desc.medals, |(m, _)| m);
    // GBX.NET reads the times as signed
    let time = |v: Option<u32>| v.map_or(Value::Null, |t| json!(t as i32));
    check("bronze", time(medals.bronze), &m["bronze"]);
    check("silver", time(medals.silver), &m["silver"]);
    check("gold", time(medals.gold), &m["gold"]);
    check("author time", time(medals.author), &m["author_time"]);
    if let Some(cps) = &map.checkpoints {
        check("checkpoints", json!(cps), &m["checkpoints"]);
    }
    if let Some((lap_race, laps)) = map.laps {
        check("lap race", json!(lap_race), &m["is_lap_race"]);
        check("laps", json!(laps), &m["nb_laps"]);
    }
    if let Some(pm) = &map.player_model {
        check("player model", ident(pm), &m["player_model"]);
    }
    let data = map.block_data.as_ref().or(map.legacy_block_data.as_ref());
    if let Some(data) = data {
        check("size", json!(data.size), &m["size"]);
        let ours: Vec<Value> = data
            .blocks
            .iter()
            .map(|b| block(b, data.version, &nodes))
            .collect();
        let theirs: Vec<Value> = m["blocks"].as_array().unwrap().iter().map(oracle_block).collect();
        check("block count", json!(ours.len()), &json!(theirs.len()));
        for (i, (o, t)) in ours.iter().zip(&theirs).enumerate() {
            check(&format!("block {i}"), o.clone(), t);
        }
        let waypoints = data.blocks.iter().filter(|b| b.waypoint.is_some()).count();
        let theirs = m["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|b| !b["waypoint"].is_null())
            .count();
        check("waypoints", json!(waypoints), &json!(theirs));
    }
    Ok(())
}

/// Every `.Gbx` under `dir`, in path order (the stock maps sit in folders; the Exchange corpora are flat).
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("gbx")) {
            out.push(p);
        }
    }
}

/// Reads every map of one corpus and compares it with GBX.NET's dump of it; returns whether all agree.
fn run_corpus(corpus: &str, maps: &Path, dump: &Path) -> bool {
    let mut want = BTreeMap::new();
    for line in std::fs::read_to_string(dump.join("maps.jsonl")).unwrap().lines() {
        let v: Value = serde_json::from_str(line).unwrap();
        want.insert(v["file"].as_str().unwrap().to_owned(), v);
    }
    let mut files = Vec::new();
    walk(maps, &mut files);
    files.sort();
    assert_eq!(files.len(), want.len(), "{corpus}: maps and oracle lines");
    let (mut read, mut both_refuse, mut failures, mut refused) = (0, 0, Vec::new(), Vec::new());
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let w = &want[&name];
        let result = compare(path, w, &dump.join("decompressed"), &mut failures);
        match (result, w["ok"].as_bool().unwrap()) {
            (Ok(()), true) => read += 1,
            (Err(e), true) => refused.push(format!("{name}: {e}")),
            (Ok(()), false) => eprintln!("{corpus}: {name}: read here; GBX.NET refused it: {}", w["error"]),
            (Err(e), false) => {
                both_refuse += 1;
                eprintln!(
                    "{corpus}: {name}: refused here ({e}) and by GBX.NET ({})",
                    w["error"]
                );
            }
        }
    }
    eprintln!(
        "{corpus}: {} maps: {read} read and compared, {both_refuse} refused by both, {} refused here only, {} mismatches",
        files.len(),
        refused.len(),
        failures.len()
    );
    for r in refused.iter().take(40) {
        eprintln!("REFUSED {corpus}: {r}");
    }
    for f in failures.iter() {
        eprintln!("MISMATCH {corpus}: {f}");
    }
    refused.is_empty() && failures.is_empty()
}

#[test]
fn every_map_reads_as_gbx_net_reads_it() {
    // the queries are literal: the inputs check reads them from this source
    let corpora = [
        (
            "tmx-2026-top",
            oracle(OracleSet::Corpus, Game::Tmnf, "tmx-2026-top"),
            oracle(OracleSet::Gbxnet, Game::Tmnf, "tmx-2026-top"),
        ),
        (
            "tmx-alltime-top",
            oracle(OracleSet::Corpus, Game::Tmnf, "tmx-alltime-top"),
            oracle(OracleSet::Gbxnet, Game::Tmnf, "tmx-alltime-top"),
        ),
        (
            "tmx-newest",
            oracle(OracleSet::Corpus, Game::Tmnf, "tmx-newest"),
            oracle(OracleSet::Gbxnet, Game::Tmnf, "tmx-newest"),
        ),
        (
            "tmnf-stock",
            oracle(OracleSet::Corpus, Game::Tmnf, "tmnf-stock"),
            oracle(OracleSet::Gbxnet, Game::Tmnf, "tmnf-stock"),
        ),
    ];
    let mut all = true;
    for (name, maps, dump) in &corpora {
        if let (Some(maps), Some(dump)) = (maps, dump) {
            all &= run_corpus(name, maps, dump);
        }
    }
    assert!(all, "see the lines above");
}
