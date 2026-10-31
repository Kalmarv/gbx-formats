# gbx-formats

A Rust library that reads Nadeo's GameBox (`.Gbx`) files: the container, its LZO1X-compressed body, and the nodes of
the classes TrackMania Nations Forever maps (`.Challenge.Gbx`) hold.

It reads, from bytes you give it:

- **The container**, version 6, binary: header, header chunks (kept as stored, for the main class's header reader),
  reference table and body, which it decompresses (LZO1X, through
  [primase-foundations](https://github.com/Kalmarv/primase-foundations)).
- **The body's nodes**, class by class: the main node and every node a reference reads, by the index the file gives
  each, with the id and node-reference tables the format uses. A chunk this crate does not read is kept as stored
  when the file marks it skippable and refused otherwise.
- **Maps** (`CGameCtnChallenge`): the header chunks (description and medal times, map info and name, version, XML,
  thumbnail and comments) and the body: blocks with their skins and waypoints, block stock and parameters (medal
  times, tips, stunts), vehicle, checkpoints, laps, mediatracker clip references, music, thumbnail camera, password.

It was written against the 100 most downloaded TrackMania Nations Forever maps uploaded to TrackMania Exchange in
2026, checked field by field against [GBX.NET](https://github.com/BigBang1112/gbx-net) 2.5.0. Used by
[primase](https://github.com/Kalmarv/primase), a game-asset conversion library.

## Example

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

This is the crate's doc test (`src/lib.rs`), so it is compiled and run by `cargo test`. A map's header chunks are read
with `game::challenge::Header::read(&gbx)`.

## What it guarantees

- Every read is bounds-checked. Malformed or hostile bytes give an `Err` that names the byte offset (in the file, or
  in the decompressed body) and what was expected, never a panic or an out-of-range index (`unsafe` is denied;
  arithmetic, indexing and truncating casts are linted against in the parsers). Counts cannot promise more items than
  there are bytes left, nothing is allocated for the header's node count, and a body's stated size is refused past
  255 times its compressed length (LZO1X's largest expansion).
- What it does not read it refuses by name instead of guessing: a Gbx version other than 6, the text format, a
  compressed reference table, a node of a class it does not read (`Error::UnknownClass`), a chunk it does not read
  that the file does not mark skippable (`Error::UnknownChunk`), a chunk version it does not know, block flags naming
  data it does not read.
- Node references are checked: an index at or past the header's node count is refused, and a reference to a node read
  earlier (or being read) is kept as that index.
- Values are kept as stored: ids as numbers or strings, block coordinates and directions as the file holds them.

## What it does not do

- It does not write Gbx files, or read the text format or Gbx versions other than 6.
- It reads only the classes maps hold, and not yet all of them: mediatracker clips (`CGameCtnMediaClip`,
  `CGameCtnMediaClipGroup` and their tracks and blocks) and ghosts are refused as unknown classes, and the custom
  objects some maps embed are not read.
- TMUnlimiter's chunks (`0x3F001000`, `0x3F001003`), which hold the real blocks of maps made with it, are kept as
  stored; those maps' block data is the stand-in TMUnlimiter writes for the game.
- It does not read pack files (`.pak`) or the block models a map names.

## Running the tests

```sh
cargo test --workspace
```

runs the tests that build their own small Gbx files in memory (a hand-built map read field by field, a
truncated-at-every-length test, refusals with their offsets, flipped and random bytes), plus the doc test above. No
game file is kept in this repository. The test that reads real maps, `tests/maps.rs`, skips with a line naming what it
needs when `GAME_DATA` is not set. To run it, point the `GAME_DATA` environment variable at a TOML file naming your own
copies, in the layout of [primase-testdata](https://github.com/Kalmarv/primase-testdata): a folder of maps, and
GBX.NET's reading of them, written by a small dumper over GBX.NET 2.5.0.
[tests/inputs/README.md](tests/inputs/README.md) lists every input the tests can read (generated from
`tests/inputs/mod.rs`).

## Documentation, contributing, license

API documentation: `cargo doc --open`. See [CONTRIBUTING.md](CONTRIBUTING.md) to build or to report a file the crate
cannot read.

Licensed under the GNU General Public License, version 3 or later (see [LICENSE](LICENSE)).
