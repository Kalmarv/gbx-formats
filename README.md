# gbx-formats

A Rust library that reads Nadeo's GameBox (`.Gbx`) files and the pack files (`.pak`) of TrackMania Nations Forever. It
is the GameBox reader behind [primase](https://github.com/Kalmarv/primase)'s TrackMania route.

## Features

- Reads the Gbx container (version 6, binary): header, header chunks, reference table and the LZO1X-compressed body
- Reads the body's nodes class by class (91 classes), following node references and lookback ids
- Reads maps (`CGameCtnChallenge`): header chunks (description, medal times, name, XML, thumbnail) and body (blocks, skins, waypoints, checkpoints, laps, parameters, music, password)
- Reads MediaTracker clips, tracks and blocks (cameras, text, images, sounds, fades, triangles, colour and blur effects)
- Reads ghosts, including their sample stream and validation inputs
- Reads TMUnlimiter's map chunks `0x3F001001` to `0x3F001003` (versions 1, 6 and 7, with embedded block models) and the encrypted `0x3F001000`
- Reads TMNF's pack files (NadeoPak version 3) with the keys from `packlist.dat`, decrypting each Gbx file as its nodes are read
- Reads Stadium's block infos, block units, scene objects, decorations, moods and collections
- Reads Plug classes: solids, trees, meshes, sprites, materials, bitmaps, shaders, lights and collision surfaces
- Resolves reference-table entries to file paths and finds pack files by real or hashed name
- Keeps skippable chunks it does not read as stored, and refuses unknown classes, unskippable chunks and unknown versions by name
- Every read is bounds-checked: bad bytes give an `Error` naming the byte offset, never a panic, and `unsafe` is denied
- Parses bytes you give it and does no file, environment or process access

## Getting Started

### Requirements

Rust 1.99 or later (edition 2024). The repository pins its toolchain in `rust-toolchain.toml`.

### Installation

The crate is not published on crates.io. Add it as a git dependency:

```toml
[dependencies]
gbx-formats = { git = "https://github.com/Kalmarv/gbx-formats" }
```

### Usage

Read a map and its main node. This is the crate's doc test, with a map built by hand:

```rust
use gbx_formats::Gbx;
use gbx_formats::game::challenge::Challenge;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A map built by hand: an uncompressed body holding one chunk, "made with the simple editor".
    let mut file = b"GBX\x06\x00BUUR".to_vec();
    for v in [0x0304_3000u32, 0, 1, 0, 0x0304_302A, 1, 0xFACA_DE01] {
        // class, no header chunks, one node, no reference table; then the body: chunk, bool, end marker
        file.extend_from_slice(&v.to_le_bytes());
    }
    let gbx = Gbx::parse(&file)?;
    let nodes = gbx.nodes()?;
    let map: &Challenge = nodes.main().try_into().map_err(|_| "the main node is not a map")?;
    assert_eq!(map.created_with_simple_editor, Some(true));
    Ok(())
}
```

A map's header chunks are read with `game::challenge::Header::read(&gbx)`.

List and read the files of an installed pack:

```rust
use gbx_formats::pak::{PackList, Pak};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let packs = Path::new("path/to/TmNationsForever/Packs");
    let list = PackList::parse(&std::fs::read(packs.join("packlist.dat"))?)?;
    let key = list.key("Stadium").ok_or("no key for Stadium")?;
    let pak = Pak::parse(std::fs::read(packs.join("Stadium.pak"))?, &key.key)?;
    for file in &pak.files {
        // a Gbx file holding a class this crate does not read is refused
        match pak.read(file) {
            Ok(bytes) => println!("{}: {} bytes", pak.path(file), bytes.len()),
            Err(e) => println!("{}: {e}", pak.path(file)),
        }
    }
    Ok(())
}
```

Run the tests with `cargo test --workspace`. It runs the tests that build their own Gbx bytes in memory; no game file is kept in the repository. The tests that compare
the crate with GBX.NET 2.5.0 over real maps and the TMNF packs skip unless `GAME_DATA` names a TOML file in the layout of
[primase-testdata](https://github.com/Kalmarv/primase-testdata) pointing at your own copies;
[tests/inputs/README.md](tests/inputs/README.md) lists what they read.

API documentation: `cargo doc --open`. See [CONTRIBUTING.md](CONTRIBUTING.md) to add a class or report a file the crate
cannot read. Licensed under the GNU General Public License, version 3 or later ([LICENSE](LICENSE)).
