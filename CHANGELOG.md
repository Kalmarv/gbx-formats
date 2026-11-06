# Changelog

## 0.2.0 (2026-11-06)

- **Breaking:** `unlimiter::AngelScriptModule` gains `byte_code`: a TMUnlimiter 2.0 script module with code is read
  (refused before, as GBX.NET refuses it). `game::angelscript::SavedModule` walks AngelScript 2.37.0's saved bytecode
  to its end (the stream has no length) and keeps it as stored, with the module's enums, classes, funcdefs, typedefs,
  globals, functions, imports and string constants. Nine of the 100 tmx-2026-top maps hold such a module.
- `Vehicle_M4AccelFromSlipAngle` and `Vehicle_M4SteerRadiusCoefFromSlipAngle` carry a float, as TMUnlimiter's script
  interface declares them (GBX.NET's table, and this crate's before, read a curve and refused the map that sets one).

## 0.1.0 (2026-11-06)

- First version: the Gbx container (version 6, binary, LZO1X bodies), node and id references, and TrackMania Nations
  Forever maps' header chunks and body (blocks, skins, parameters, medal times).
