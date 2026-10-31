//! The real-data inputs this crate's tests read, each with what it is and where it comes from
//! (`primase_testdata::inputs!`; README.md here lists them). A test whose input is not configured skips with a line
//! naming its key. An input declared `gate: true` is one a complete run needs: with `[run] require = 'gate'` in the
//! `GAME_DATA` file, a missing one fails the test instead of skipping it. Integration tests reach this module with
//! `mod inputs;`, and tests/real_data_inputs.rs checks that every query the crate's sources write is declared here.
//! Declare an input here when a test first asks for it.

primase_testdata::inputs! {
    games {
        Tmnf = "tmnf",
    }
    installs {
        Tmnf {
            about: "TrackMania Nations Forever's install folder (`Packs/`: packlist.dat and the Game, Resource and Stadium packs; `GameData/`)",
            source: "the user's own install (Steam, TrackMania Nations Forever, Nadeo.ini Version=2.11.16)",
            gate: false,
        },
    }
    keys {}
    tools {}
    oracle_sets {
        Corpus = "corpus" {
            about: "files as stored, gathered from where they are published or installed",
            source: "TrackMania Exchange downloads (user-made maps)",
        },
        Gbxnet = "gbxnet" {
            about: "what GBX.NET 2.5.0 (third-party, MIT) reads from the corpus, written by a thin dump harness",
            source: "GBX.NET 2.5.0 with GBX.NET.LZO 2.1.6, run over the corpus files",
        },
    }
    declared {
        Query::Install(Game::Tmnf, Some("GameData")) => {
            status: Live,
            gate: false,
            about: r#"the install's `GameData` folder: the files the game reads loose, beside the packs (textures `.dds`/`.tga`, `.zip` archives, sounds, the stock maps under `Tracks`)"#,
            source: r#"the user's own install"#,
        },
        Query::Install(Game::Tmnf, Some("Packs")) => {
            status: Live,
            gate: false,
            about: r#"the install's `Packs` folder: `packlist.dat` (the packs' keys) and `Game.pak`, `Resource.pak`, `Stadium.pak` (NadeoPak version 3; 614, 27 and 1,931 files)"#,
            source: r#"the user's own install"#,
        },
        Query::Oracle(OracleSet::Corpus, Game::Tmnf, "tmx-2026-top") => {
            status: Live,
            gate: false,
            about: r#"the 100 most downloaded TrackMania Nations Forever maps uploaded to TrackMania Exchange in 2026 (`<TrackId>.Gbx`, 32 MB)"#,
            source: r#"tmnf.exchange `api/tracks` ordered by downloads, uploaded after 2026-01-01; each from `trackgbx/<TrackId>`"#,
        },
        Query::Oracle(OracleSet::Gbxnet, Game::Tmnf, "tmx-2026-top") => {
            status: Live,
            gate: false,
            about: r#"GBX.NET 2.5.0's reading of each corpus map: `maps.jsonl` (one line per map: container, chunk ids, node and chunk census, map fields, blocks) and `decompressed/<file>` (the map with its body decompressed by GBX.NET.LZO)"#,
            source: r#"the gbxnet-dump harness (GBX.NET 2.5.0, GBX.NET.LZO 2.1.6) over the corpus maps"#,
        },
        Query::Oracle(OracleSet::Corpus, Game::Tmnf, "tmx-alltime-top") => {
            status: Live,
            gate: false,
            about: r#"the 100 most downloaded TrackMania Nations Forever maps of all time on TrackMania Exchange (`<TrackId>.Gbx`)"#,
            source: r#"tmnf.exchange `api/tracks` with `order1=18`, `count=100`; each from `trackgbx/<TrackId>`"#,
        },
        Query::Oracle(OracleSet::Gbxnet, Game::Tmnf, "tmx-alltime-top") => {
            status: Live,
            gate: false,
            about: r#"GBX.NET 2.5.0's reading of each map of `tmx-alltime-top`: `maps.jsonl` (one line per map: container, chunk ids, node and chunk census, map fields, blocks) and `decompressed/<file>` (the map with its body decompressed by GBX.NET.LZO)"#,
            source: r#"the gbxnet-dump harness (GBX.NET 2.5.0, GBX.NET.LZO 2.1.6) over the corpus maps"#,
        },
        Query::Oracle(OracleSet::Corpus, Game::Tmnf, "tmx-newest") => {
            status: Live,
            gate: false,
            about: r#"the 100 most recently uploaded TrackMania Nations Forever maps on TrackMania Exchange when it was gathered (`<TrackId>.Gbx`)"#,
            source: r#"tmnf.exchange `api/tracks` with `order1=2` (upload date, newest first), `count=100`; each from `trackgbx/<TrackId>`"#,
        },
        Query::Oracle(OracleSet::Gbxnet, Game::Tmnf, "tmx-newest") => {
            status: Live,
            gate: false,
            about: r#"GBX.NET 2.5.0's reading of each map of `tmx-newest`: `maps.jsonl` (one line per map: container, chunk ids, node and chunk census, map fields, blocks) and `decompressed/<file>` (the map with its body decompressed by GBX.NET.LZO)"#,
            source: r#"the gbxnet-dump harness (GBX.NET 2.5.0, GBX.NET.LZO 2.1.6) over the corpus maps"#,
        },
        Query::Oracle(OracleSet::Corpus, Game::Tmnf, "tmnf-stock") => {
            status: Live,
            gate: false,
            about: r#"every `*.Challenge.Gbx` of a TrackMania Nations Forever install (72 maps, under `GameData/Tracks/...`)"#,
            source: r#"the install's GameData folder, copied with its relative paths"#,
        },
        Query::Oracle(OracleSet::Gbxnet, Game::Tmnf, "tmnf-packs") => {
            status: Live,
            gate: false,
            about: r#"GBX.NET 2.5.0's extraction of the install's three packs (2,572 files): `<pack>/<path>` (each file's plaintext, under its real name where GBX.NET resolves the stored hash), `list.txt` (pack, size, class id, path, kind per file) and `writes.tsv` (every dummy write its node reading made: pack, path, plaintext position, `class` with the class id and the parent its reflection gives, or `data` with the 4 bytes)"#,
            source: r#"the gbxnet-pak harness (GBX.NET 2.5.0's NadeoPak with a recording IClassEncryptionInitializer) over `Packs/*.pak` with `packlist.dat`'s keys"#,
        },
        Query::Oracle(OracleSet::Gbxnet, Game::Tmnf, "tmnf-stock") => {
            status: Live,
            gate: false,
            about: r#"GBX.NET 2.5.0's reading of each map of `tmnf-stock`: `maps.jsonl` (one line per map: container, chunk ids, node and chunk census, map fields, blocks) and `decompressed/<file>` (the map with its body decompressed by GBX.NET.LZO)"#,
            source: r#"the gbxnet-dump harness (GBX.NET 2.5.0, GBX.NET.LZO 2.1.6) over the corpus maps"#,
        },
    }
    debug {}
}
