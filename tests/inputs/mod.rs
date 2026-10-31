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
    installs {}
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
    }
    debug {}
}
