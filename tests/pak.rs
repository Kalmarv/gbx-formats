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
