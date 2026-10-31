# Real-data inputs

What this crate's real-data tests read, as tests/inputs/mod.rs declares them: the keys of the configuration file
`GAME_DATA` names (its layout is that of [primase-testdata](https://github.com/Kalmarv/primase-testdata), which the
tests use), the oracle sets and the files of each, and the debug keys `GAME_DATA_DEBUG` sets. A test whose input is not
configured skips with a line naming its key; the "gate" column marks the inputs that fail instead of skipping when the
file says `[run] require = 'gate'`. The tables below are generated from the declarations (the `real_data_inputs` test
fails when they differ); [the README](../../README.md#running-the-tests) says how to set the inputs up.

<!-- real-data inputs: made by primase_testdata::doc::tables from tests/inputs/mod.rs -->

#### Configuration keys

| key | gate | about | source |
| --- | --- | --- | --- |
| `scratch.root` | - | where real-data tests write their outputs (the OS temporary folder when unset) | - |

#### Oracle sets (`<oracles.root>/<set>/<game>/`)

| set | about | source |
| --- | --- | --- |
| `oracles.corpus` | files as stored, gathered from where they are published or installed | TrackMania Exchange downloads (user-made maps) |
| `oracles.gbxnet` | what GBX.NET 2.5.0 (third-party, MIT) reads from the corpus, written by a thin dump harness | GBX.NET 2.5.0 with GBX.NET.LZO 2.1.6, run over the corpus files |

#### Declared files

| key | status | gate | about | source |
| --- | --- | --- | --- | --- |
| `oracles.corpus.tmnf/tmx-2026-top` | live | - | the 100 most downloaded TrackMania Nations Forever maps uploaded to TrackMania Exchange in 2026 (`<TrackId>.Gbx`, 32 MB) | tmnf.exchange `api/tracks` ordered by downloads, uploaded after 2026-01-01; each from `trackgbx/<TrackId>` |
| `oracles.gbxnet.tmnf/tmx-2026-top` | live | - | GBX.NET 2.5.0's reading of each corpus map: `maps.jsonl` (one line per map: container, chunk ids, node and chunk census, map fields, blocks) and `decompressed/<file>` (the map with its body decompressed by GBX.NET.LZO) | the gbxnet-dump harness (GBX.NET 2.5.0, GBX.NET.LZO 2.1.6) over the corpus maps |
| `oracles.corpus.tmnf/tmx-alltime-top` | live | - | the 100 most downloaded TrackMania Nations Forever maps of all time on TrackMania Exchange (`<TrackId>.Gbx`) | tmnf.exchange `api/tracks` with `order1=18`, `count=100`; each from `trackgbx/<TrackId>` |
| `oracles.gbxnet.tmnf/tmx-alltime-top` | live | - | GBX.NET 2.5.0's reading of each map of `tmx-alltime-top`: `maps.jsonl` (one line per map: container, chunk ids, node and chunk census, map fields, blocks) and `decompressed/<file>` (the map with its body decompressed by GBX.NET.LZO) | the gbxnet-dump harness (GBX.NET 2.5.0, GBX.NET.LZO 2.1.6) over the corpus maps |
| `oracles.corpus.tmnf/tmx-newest` | live | - | the 100 most recently uploaded TrackMania Nations Forever maps on TrackMania Exchange when it was gathered (`<TrackId>.Gbx`) | tmnf.exchange `api/tracks` with `order1=2` (upload date, newest first), `count=100`; each from `trackgbx/<TrackId>` |
| `oracles.gbxnet.tmnf/tmx-newest` | live | - | GBX.NET 2.5.0's reading of each map of `tmx-newest`: `maps.jsonl` (one line per map: container, chunk ids, node and chunk census, map fields, blocks) and `decompressed/<file>` (the map with its body decompressed by GBX.NET.LZO) | the gbxnet-dump harness (GBX.NET 2.5.0, GBX.NET.LZO 2.1.6) over the corpus maps |
| `oracles.corpus.tmnf/tmnf-stock` | live | - | every `*.Challenge.Gbx` of a TrackMania Nations Forever install (72 maps, under `GameData/Tracks/...`) | the install's GameData folder, copied with its relative paths |
| `oracles.gbxnet.tmnf/tmnf-stock` | live | - | GBX.NET 2.5.0's reading of each map of `tmnf-stock`: `maps.jsonl` (one line per map: container, chunk ids, node and chunk census, map fields, blocks) and `decompressed/<file>` (the map with its body decompressed by GBX.NET.LZO) | the gbxnet-dump harness (GBX.NET 2.5.0, GBX.NET.LZO 2.1.6) over the corpus maps |

<!-- end of real-data inputs -->

<!-- end of real-data inputs -->
