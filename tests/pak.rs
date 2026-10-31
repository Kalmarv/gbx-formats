//! Reads every file of the TrackMania Nations Forever packs (`Packs/*.pak` of the install, keys from its
//! `packlist.dat`) and compares it with GBX.NET 2.5.0's extraction of the same packs (`gbxnet/tmnf/tmnf-packs`).
//!
//! A pack's Gbx files decrypt only as their nodes are read: every node read folds its parent class id into the
//! cipher chain (the module documentation of `gbx_formats::pak`). So two readings are checked:
//!
//! - with the dummy writes GBX.NET made (`writes.tsv`, each at the position it made it), replayed on the crate's
//!   [`FileStream`]: every file must come out as GBX.NET extracted it, which checks the keys, the cipher, the
//!   inflater and where a write lands;
//! - with the writes the crate's own node reading makes ([`Pak::read`]): a file is either refused (a class or chunk
//!   the crate does not read yet, listed by class) or comes out exactly as GBX.NET extracted it.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test reading the files you point it at; the library does no I/O"
)]

mod inputs;

use gbx_formats::pak::{FileStream, PackList, Pak, PakFile};
use inputs::{Game, OracleSet, install_file, oracle};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// GBX.NET's dummy writes, by (pack, path): the plaintext position each was made at and its 4 bytes.
type Writes = HashMap<(String, String), Vec<(usize, [u8; 4])>>;

/// `writes.tsv`: `pack, path, position, class|data, class id|-, parent|bytes` per write. GBX.NET's harness logs a
/// class write with the parent its reflection gives; the pack version 3 overrides its encryption initializer applies
/// are applied here.
fn writes(oracle: &Path) -> Writes {
    let mut out = Writes::new();
    for line in std::fs::read_to_string(oracle.join("writes.tsv"))
        .unwrap()
        .lines()
    {
        let f: Vec<&str> = line.split('\t').collect();
        let bytes = if f[3] == "class" {
            let class = u32::from_str_radix(f[4], 16).unwrap();
            let parent = u32::from_str_radix(f[5], 16).unwrap();
            let parent = match (class, parent) {
                (0x0600_C000 | 0x0600_D000, _) => 0x0600_B000,
                (0x2400_1000, _) => 0x2403_3000,
                (0x090E_F000 | 0x090F_6000 | 0x0911_2000, 0x0910_C000) => 0x0100_1000,
                _ => parent,
            };
            parent.to_le_bytes()
        } else {
            let byte = |i: usize| u8::from_str_radix(&f[5][i * 2..i * 2 + 2], 16).unwrap();
            [byte(0), byte(1), byte(2), byte(3)]
        };
        out.entry((f[0].to_owned(), f[1].to_owned()))
            .or_default()
            .push((f[2].parse().unwrap(), bytes));
    }
    out
}

/// The plaintext with `writes` made at their positions: the stream is read up to each before it is made.
fn replay(pak: &Pak, file: &PakFile, writes: &[(usize, [u8; 4])]) -> gbx_formats::Result<Vec<u8>> {
    let mut s: FileStream = pak.open(file)?;
    for &(at, data) in writes {
        s.read(0, at)?;
        s.dummy_write(data);
    }
    s.finish()
}

/// One pack file and the oracle's extraction of it.
struct Entry<'a> {
    pak: &'a Pak,
    file: &'a PakFile,
    pack: String,
    /// The path GBX.NET wrote it under: its real name where `list.txt` resolves the stored hash.
    path: String,
    expected: PathBuf,
}

/// Every file of every installed pack, with where GBX.NET's extraction of it is.
fn each_file(packs: &Path, oracle: &Path, mut f: impl FnMut(Entry<'_>)) -> usize {
    let list = PackList::parse(&std::fs::read(packs.join("packlist.dat")).unwrap()).unwrap();
    // list.txt: pack, size, class, path, kind
    let mut names: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in std::fs::read_to_string(oracle.join("list.txt")).unwrap().lines() {
        let f: Vec<&str> = line.split('\t').collect();
        names.entry(f[0].to_owned()).or_default().push(f[3].to_owned());
    }
    let mut paks: Vec<PathBuf> = std::fs::read_dir(packs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pak")))
        .collect();
    paks.sort();
    let mut files = 0;
    for path in paks {
        let pack = path.file_stem().unwrap().to_string_lossy().into_owned();
        let key = list.key(&pack).unwrap();
        let pak = Pak::parse(std::fs::read(&path).unwrap(), &key.key).unwrap();
        let real: HashMap<String, String> = names
            .get(&pack)
            .into_iter()
            .flatten()
            .filter_map(|p| pak.find(p).map(|f| (pak.path(f), p.clone())))
            .collect();
        for file in &pak.files {
            let stored = pak.path(file);
            let path = real.get(&stored).cloned().unwrap_or(stored);
            let expected = oracle.join(&pack).join(path.replace('\\', "/"));
            files += 1;
            f(Entry {
                pak: &pak,
                file,
                pack: pack.clone(),
                path,
                expected,
            });
        }
    }
    files
}

#[test]
fn every_pack_file_decrypts_with_gbx_nets_writes() {
    let (Some(packs), Some(oracle)) = (
        install_file(Game::Tmnf, "Packs"),
        oracle(OracleSet::Gbxnet, Game::Tmnf, "tmnf-packs"),
    ) else {
        return;
    };
    let writes = writes(&oracle);
    let mut differ = Vec::new();
    let files = each_file(&packs, &oracle, |e| {
        let w = writes
            .get(&(e.pack.clone(), e.path.clone()))
            .map_or(&[][..], Vec::as_slice);
        let got = replay(e.pak, e.file, w).map_err(|err| err.to_string());
        let want = std::fs::read(&e.expected).unwrap();
        if got.as_ref() != Ok(&want) {
            differ.push(format!("{}\\{}: {:?}", e.pack, e.path, got.map(|g| g.len())));
        }
    });
    inputs::expect_found(
        inputs::Query::Install(Game::Tmnf, Some("Packs")),
        "pack files",
        files,
    );
    for d in &differ {
        eprintln!("DIFFERS {d}");
    }
    assert_eq!((files, differ.len()), (2_572, 0));
}

#[test]
fn pack_files_read_through_their_nodes_as_gbx_net_extracts_them() {
    let (Some(packs), Some(oracle)) = (
        install_file(Game::Tmnf, "Packs"),
        oracle(OracleSet::Gbxnet, Game::Tmnf, "tmnf-packs"),
    ) else {
        return;
    };
    let (mut same, mut differ, mut refused) = (0, Vec::new(), BTreeMap::<String, Vec<String>>::new());
    let files = each_file(&packs, &oracle, |e| {
        let want = std::fs::read(&e.expected).unwrap();
        match e.pak.read(e.file) {
            Ok(got) if got == want => same += 1,
            Ok(got) => {
                let first = got.iter().zip(&want).position(|(a, b)| a != b);
                differ.push(format!(
                    "{}\\{}: first difference at {first:?} ({} / {} bytes)",
                    e.pack,
                    e.path,
                    got.len(),
                    want.len()
                ));
            }
            Err(err) => {
                // the refusal's own words, without its position: one line per kind
                let s = err.to_string();
                let kind = s.split_once(": ").map_or(s.as_str(), |(_, d)| d);
                let kind = kind.split(" at byte").next().unwrap_or(kind).to_owned();
                refused
                    .entry(kind)
                    .or_default()
                    .push(format!("{}\\{}", e.pack, e.path));
            }
        }
    });
    let mut kinds: Vec<_> = refused.iter().collect();
    kinds.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));
    for (kind, paths) in &kinds {
        eprintln!("REFUSED {:5} {kind} (e.g. {})", paths.len(), paths[0]);
    }
    for d in &differ {
        eprintln!("DIFFERS {d}");
    }
    let refused: usize = refused.values().map(Vec::len).sum();
    eprintln!(
        "{files} pack files: {same} read and the same, {refused} refused, {} differ",
        differ.len()
    );
    assert!(
        differ.is_empty(),
        "a file read in full must be the one GBX.NET extracted"
    );
    assert_eq!(same + refused, files);
}

/// The game's files as its own file system sees them: the install's `GameData` folder and the packs over it, each
/// pack's paths from the same root (`Stadium\Media\...` may be in `Game.pak`, `Stadium.pak` or a loose file).
struct GameFiles {
    paks: Vec<(String, Pak)>,
    /// Every loose file under `GameData`, by its lowercased `\`-separated path.
    loose: HashMap<String, PathBuf>,
}

/// Where a path of the game's file system is.
enum Found<'a> {
    Pack(&'a Pak, &'a PakFile),
    Loose(&'a Path),
}

impl GameFiles {
    fn open(packs: &Path, game_data: &Path) -> GameFiles {
        let list = PackList::parse(&std::fs::read(packs.join("packlist.dat")).unwrap()).unwrap();
        let mut paks = Vec::new();
        for e in std::fs::read_dir(packs).unwrap() {
            let path = e.unwrap().path();
            if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pak")) {
                let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                let key = list.key(&name).unwrap();
                paks.push((name, Pak::parse(std::fs::read(&path).unwrap(), &key.key).unwrap()));
            }
        }
        paks.sort_by(|a, b| a.0.cmp(&b.0));
        let mut loose = HashMap::new();
        let mut stack = vec![game_data.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    let rel = p.strip_prefix(game_data).unwrap().to_string_lossy();
                    loose.insert(rel.replace('/', "\\").to_lowercase(), p.clone());
                }
            }
        }
        GameFiles { paks, loose }
    }

    fn find(&self, path: &str) -> Option<Found<'_>> {
        self.paks
            .iter()
            .find_map(|(_, p)| p.find(path).map(|f| Found::Pack(p, f)))
            .or_else(|| {
                self.loose
                    .get(&path.replace('/', "\\").to_lowercase())
                    .map(|p| Found::Loose(p))
            })
    }
}

/// A pack file's stored name is its real one (not [`gbx_formats::pak::name_hash`]'s 34 hex digits).
fn real_name(file: &PakFile) -> bool {
    !(file.name.len() == 34 && file.name.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Following the reference tables from every pack file stored under its real name, and from every file they reach:
/// each file named (resolved from the naming file's path, [`gbx_formats::RefTable::file_path`]) is in the game's
/// file system (a pack or a loose file of `GameData`).
#[test]
fn every_reference_of_the_packs_names_a_game_file() {
    let (Some(packs), Some(game_data)) = (
        install_file(Game::Tmnf, "Packs"),
        install_file(Game::Tmnf, "GameData"),
    ) else {
        return;
    };
    let files = GameFiles::open(&packs, &game_data);
    let mut queue: Vec<String> = files
        .paks
        .iter()
        .flat_map(|(_, p)| p.files.iter().filter(|f| real_name(f)).map(|f| p.path(f)))
        .collect();
    let mut seen: std::collections::HashSet<String> = queue.iter().map(|p| p.to_lowercase()).collect();
    let (mut tables, mut references, mut resources, mut missing) = (0, 0, 0, Vec::new());
    // the pack files reached, by their stored paths
    let mut reached = std::collections::HashSet::new();
    while let Some(from) = queue.pop() {
        let table = match files.find(&from) {
            Some(Found::Pack(pak, file)) => {
                reached.insert(pak.path(file));
                pak.ref_table(file).unwrap()
            }
            Some(Found::Loose(path)) => {
                let bytes = std::fs::read(path).unwrap();
                if bytes.starts_with(b"GBX") {
                    Some(gbx_formats::Gbx::parse(&bytes).unwrap().ref_table)
                } else {
                    None
                }
            }
            None => unreachable!("{from} was found before it was queued"),
        };
        let Some(table) = table else {
            continue;
        };
        tables += 1;
        for external in &table.external {
            let Some(path) = table.file_path(external, &from) else {
                resources += 1;
                continue;
            };
            references += 1;
            if files.find(&path).is_none() {
                missing.push(format!("{from} -> {path}"));
            } else if seen.insert(path.to_lowercase()) {
                queue.push(path);
            }
        }
    }
    let unreached = files
        .paks
        .iter()
        .flat_map(|(_, p)| p.files.iter().filter(|f| !real_name(f)).map(|f| p.path(f)))
        .filter(|stored| !reached.contains(stored))
        .count();
    for m in &missing {
        eprintln!("MISSING {m}");
    }
    eprintln!(
        "{tables} reference tables: {references} file references, {resources} resources, {} missing; \
         {unreached} pack files under a hashed name no reference reaches",
        missing.len()
    );
    inputs::expect_found(
        inputs::Query::Install(Game::Tmnf, Some("Packs")),
        "reference tables",
        tables,
    );
    assert!(missing.is_empty());
}

/// Every pack file's head reads alone ([`Pak::header`]), and every collector desc (header chunk `0x0301A003`) in
/// them reads to its end; a collector whose body reads names the same ident in its body (chunk `0x0301A00B`).
#[test]
fn every_collector_desc_reads_and_names_its_body_ident() {
    use gbx_formats::Node;
    use gbx_formats::game::collector::{Collector, CollectorDesc};
    let Some(packs) = install_file(Game::Tmnf, "Packs") else {
        return;
    };
    let list = PackList::parse(&std::fs::read(packs.join("packlist.dat")).unwrap()).unwrap();
    let mut paks: Vec<PathBuf> = std::fs::read_dir(&packs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pak")))
        .collect();
    paks.sort();
    let (mut heads, mut descs, mut compared, mut differ, mut versions) =
        (0, 0, 0, Vec::new(), BTreeMap::<u32, usize>::new());
    for path in paks {
        let pack = path.file_stem().unwrap().to_string_lossy().into_owned();
        let pak = Pak::parse(std::fs::read(&path).unwrap(), &list.key(&pack).unwrap().key).unwrap();
        for file in &pak.files {
            let Some(header) = pak.header(file).unwrap() else {
                continue;
            };
            heads += 1;
            let Some(desc) = CollectorDesc::read(&header.header_chunks()).unwrap() else {
                continue;
            };
            descs += 1;
            *versions.entry(desc.version).or_default() += 1;
            let Ok(bytes) = pak.read(file) else {
                continue;
            };
            let nodes = gbx_formats::Gbx::parse(&bytes).unwrap().nodes().unwrap();
            let collector: Option<&Collector> = match nodes.main() {
                Node::BlockInfoClassic(b) => Some(&b.collector),
                Node::BlockInfoFlat(b) => Some(&b.collector),
                Node::BlockInfoRectAsym(b) => Some(&b.collector),
                Node::BlockInfoRoad(b) => Some(&b.collector),
                Node::BlockInfoClip(b) => Some(&b.collector),
                Node::BlockInfoFrontier(b) => Some(&b.collector),
                Node::BlockInfoPylon(b) => Some(&b.collector),
                Node::Decoration(d) => Some(&d.collector),
                _ => None,
            };
            if let Some(ident) = collector.and_then(|c| c.ident.as_ref()) {
                compared += 1;
                if *ident != desc.ident {
                    differ.push(format!(
                        "{pack}\\{}: {ident:?} / {:?}",
                        pak.path(file),
                        desc.ident
                    ));
                }
            }
        }
    }
    for d in &differ {
        eprintln!("DIFFERS {d}");
    }
    eprintln!(
        "{heads} heads, {descs} collector descs (by version {versions:?}), {compared} compared with their body"
    );
    inputs::expect_found(
        inputs::Query::Install(Game::Tmnf, Some("Packs")),
        "collector descs",
        descs,
    );
    assert!(differ.is_empty());
    assert!(compared > 0);
}
