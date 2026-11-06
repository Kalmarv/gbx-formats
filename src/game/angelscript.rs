//! AngelScript saved bytecode: a module as AngelScript 2.37.0's `asIScriptModule::SaveByteCode` writes it, which is how
//! TMUnlimiter 2.0 stores a script module in a map ([`super::unlimiter::AngelScriptModule`]).
//!
//! The stream has no length of its own: its end is found by walking it the way `asCReader` (`as_restore.cpp` of
//! 2.37.0) reads it, every entity in order. The bytes are kept as stored ([`SavedModule::bytes`], what
//! `LoadByteCode` takes back) along with the module's declarations. TMUnlimiter's DLL is 32-bit, so pointer operands of
//! the bytecode are 32-bit words; which AngelScript it links is told by its error texts: those added in 2.37.0 are in
//! it, none of 2.38.0's, and the maps' type tokens are 2.37's (`bool` 65, `void` 80; 2.38 renumbered them).
//!
//! The walk checks what the reader would: string, data type and function references point back at entries already
//! read, type-info and function tags are known, encoded numbers fit their width, counts are within AngelScript's own
//! sanity bounds, and every instruction is one the writer emits.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::Archive;
use crate::error::Result;

/// A declared entity's name and namespace ("" for the global one).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declared {
    /// Its name.
    pub name: String,
    /// Its namespace, `::`-separated.
    pub namespace: String,
}

/// A script function of the module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    /// Its name.
    pub name: String,
    /// Its namespace (a method's is its class's).
    pub namespace: String,
    /// The class it is a method of.
    pub class: Option<String>,
}

/// A function the module imports from another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Import {
    /// The imported function's name.
    pub name: String,
    /// The module it is imported from.
    pub module: String,
}

/// A module's saved bytecode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedModule {
    /// The stream as stored.
    pub bytes: Vec<u8>,
    /// Whether it was saved with debug information (line numbers, variable and parameter names, section names).
    pub debug_info: bool,
    /// The enums it declares.
    pub enums: Vec<Declared>,
    /// The classes and interfaces it declares.
    pub classes: Vec<Declared>,
    /// The funcdefs it declares.
    pub funcdefs: Vec<Declared>,
    /// The typedefs it declares.
    pub typedefs: Vec<Declared>,
    /// Its global variables.
    pub globals: Vec<Declared>,
    /// Its script functions (methods, global functions and the rest, in the module's order).
    pub functions: Vec<Function>,
    /// The functions it imports.
    pub imports: Vec<Import>,
    /// The string constants its code uses, as bytes (the host's string type decides their encoding: UTF-16LE in
    /// TMUnlimiter's maps).
    pub string_constants: Vec<Vec<u8>>,
}

/// An operand of an instruction: an encoded 16-bit word, 32-bit word or 64-bit word.
#[derive(Clone, Copy)]
enum Arg {
    W,
    D,
    Q,
}

/// The operands of each instruction (`asBCInfo` of 2.37.0 with 32-bit pointers), `None` for the codes the writer
/// never emits: the information pseudo-instructions and the unused codes.
const INSTRUCTIONS: [Option<(&[Arg], &str)>; 256] = [
    Some((&[], "PopPtr")),                            // 0
    Some((&[Arg::D], "PshGPtr")),                     // 1
    Some((&[Arg::D], "PshC4")),                       // 2
    Some((&[Arg::W], "PshV4")),                       // 3
    Some((&[Arg::W], "PSF")),                         // 4
    Some((&[], "SwapPtr")),                           // 5
    Some((&[Arg::W], "NOT")),                         // 6
    Some((&[Arg::D], "PshG4")),                       // 7
    Some((&[Arg::W, Arg::D], "LdGRdR4")),             // 8
    Some((&[Arg::D], "CALL")),                        // 9
    Some((&[Arg::W], "RET")),                         // 10
    Some((&[Arg::D], "JMP")),                         // 11
    Some((&[Arg::D], "JZ")),                          // 12
    Some((&[Arg::D], "JNZ")),                         // 13
    Some((&[Arg::D], "JS")),                          // 14
    Some((&[Arg::D], "JNS")),                         // 15
    Some((&[Arg::D], "JP")),                          // 16
    Some((&[Arg::D], "JNP")),                         // 17
    Some((&[], "TZ")),                                // 18
    Some((&[], "TNZ")),                               // 19
    Some((&[], "TS")),                                // 20
    Some((&[], "TNS")),                               // 21
    Some((&[], "TP")),                                // 22
    Some((&[], "TNP")),                               // 23
    Some((&[Arg::W], "NEGi")),                        // 24
    Some((&[Arg::W], "NEGf")),                        // 25
    Some((&[Arg::W], "NEGd")),                        // 26
    Some((&[], "INCi16")),                            // 27
    Some((&[], "INCi8")),                             // 28
    Some((&[], "DECi16")),                            // 29
    Some((&[], "DECi8")),                             // 30
    Some((&[], "INCi")),                              // 31
    Some((&[], "DECi")),                              // 32
    Some((&[], "INCf")),                              // 33
    Some((&[], "DECf")),                              // 34
    Some((&[], "INCd")),                              // 35
    Some((&[], "DECd")),                              // 36
    Some((&[Arg::W], "IncVi")),                       // 37
    Some((&[Arg::W], "DecVi")),                       // 38
    Some((&[Arg::W], "BNOT")),                        // 39
    Some((&[Arg::W, Arg::W, Arg::W], "BAND")),        // 40
    Some((&[Arg::W, Arg::W, Arg::W], "BOR")),         // 41
    Some((&[Arg::W, Arg::W, Arg::W], "BXOR")),        // 42
    Some((&[Arg::W, Arg::W, Arg::W], "BSLL")),        // 43
    Some((&[Arg::W, Arg::W, Arg::W], "BSRL")),        // 44
    Some((&[Arg::W, Arg::W, Arg::W], "BSRA")),        // 45
    Some((&[Arg::W, Arg::D], "COPY")),                // 46
    Some((&[Arg::Q], "PshC8")),                       // 47
    Some((&[Arg::W], "PshVPtr")),                     // 48
    Some((&[], "RDSPtr")),                            // 49
    Some((&[Arg::W, Arg::W], "CMPd")),                // 50
    Some((&[Arg::W, Arg::W], "CMPu")),                // 51
    Some((&[Arg::W, Arg::W], "CMPf")),                // 52
    Some((&[Arg::W, Arg::W], "CMPi")),                // 53
    Some((&[Arg::W, Arg::D], "CMPIi")),               // 54
    Some((&[Arg::W, Arg::D], "CMPIf")),               // 55
    Some((&[Arg::W, Arg::D], "CMPIu")),               // 56
    Some((&[Arg::W], "JMPP")),                        // 57
    Some((&[], "PopRPtr")),                           // 58
    Some((&[], "PshRPtr")),                           // 59
    Some((&[Arg::W], "STR")),                         // 60
    Some((&[Arg::D], "CALLSYS")),                     // 61
    Some((&[Arg::D], "CALLBND")),                     // 62
    Some((&[], "SUSPEND")),                           // 63
    Some((&[Arg::D, Arg::D], "ALLOC")),               // 64
    Some((&[Arg::W, Arg::D], "FREE")),                // 65
    Some((&[Arg::W], "LOADOBJ")),                     // 66
    Some((&[Arg::W], "STOREOBJ")),                    // 67
    Some((&[Arg::W], "GETOBJ")),                      // 68
    Some((&[Arg::D], "REFCPY")),                      // 69
    Some((&[], "CHKREF")),                            // 70
    Some((&[Arg::W], "GETOBJREF")),                   // 71
    Some((&[Arg::W], "GETREF")),                      // 72
    Some((&[], "PshNull")),                           // 73
    Some((&[Arg::W], "ClrVPtr")),                     // 74
    Some((&[Arg::D], "OBJTYPE")),                     // 75
    Some((&[Arg::D], "TYPEID")),                      // 76
    Some((&[Arg::W, Arg::D], "SetV4")),               // 77
    Some((&[Arg::W, Arg::Q], "SetV8")),               // 78
    Some((&[Arg::W, Arg::D], "ADDSi")),               // 79
    Some((&[Arg::W, Arg::W], "CpyVtoV4")),            // 80
    Some((&[Arg::W, Arg::W], "CpyVtoV8")),            // 81
    Some((&[Arg::W], "CpyVtoR4")),                    // 82
    Some((&[Arg::W], "CpyVtoR8")),                    // 83
    Some((&[Arg::W, Arg::D], "CpyVtoG4")),            // 84
    Some((&[Arg::W], "CpyRtoV4")),                    // 85
    Some((&[Arg::W], "CpyRtoV8")),                    // 86
    Some((&[Arg::W, Arg::D], "CpyGtoV4")),            // 87
    Some((&[Arg::W], "WRTV1")),                       // 88
    Some((&[Arg::W], "WRTV2")),                       // 89
    Some((&[Arg::W], "WRTV4")),                       // 90
    Some((&[Arg::W], "WRTV8")),                       // 91
    Some((&[Arg::W], "RDR1")),                        // 92
    Some((&[Arg::W], "RDR2")),                        // 93
    Some((&[Arg::W], "RDR4")),                        // 94
    Some((&[Arg::W], "RDR8")),                        // 95
    Some((&[Arg::D], "LDG")),                         // 96
    Some((&[Arg::W], "LDV")),                         // 97
    Some((&[Arg::D], "PGA")),                         // 98
    Some((&[Arg::W, Arg::W], "CmpPtr")),              // 99
    Some((&[Arg::W], "VAR")),                         // 100
    Some((&[Arg::W], "iTOf")),                        // 101
    Some((&[Arg::W], "fTOi")),                        // 102
    Some((&[Arg::W], "uTOf")),                        // 103
    Some((&[Arg::W], "fTOu")),                        // 104
    Some((&[Arg::W], "sbTOi")),                       // 105
    Some((&[Arg::W], "swTOi")),                       // 106
    Some((&[Arg::W], "ubTOi")),                       // 107
    Some((&[Arg::W], "uwTOi")),                       // 108
    Some((&[Arg::W, Arg::W], "dTOi")),                // 109
    Some((&[Arg::W, Arg::W], "dTOu")),                // 110
    Some((&[Arg::W, Arg::W], "dTOf")),                // 111
    Some((&[Arg::W, Arg::W], "iTOd")),                // 112
    Some((&[Arg::W, Arg::W], "uTOd")),                // 113
    Some((&[Arg::W, Arg::W], "fTOd")),                // 114
    Some((&[Arg::W, Arg::W, Arg::W], "ADDi")),        // 115
    Some((&[Arg::W, Arg::W, Arg::W], "SUBi")),        // 116
    Some((&[Arg::W, Arg::W, Arg::W], "MULi")),        // 117
    Some((&[Arg::W, Arg::W, Arg::W], "DIVi")),        // 118
    Some((&[Arg::W, Arg::W, Arg::W], "MODi")),        // 119
    Some((&[Arg::W, Arg::W, Arg::W], "ADDf")),        // 120
    Some((&[Arg::W, Arg::W, Arg::W], "SUBf")),        // 121
    Some((&[Arg::W, Arg::W, Arg::W], "MULf")),        // 122
    Some((&[Arg::W, Arg::W, Arg::W], "DIVf")),        // 123
    Some((&[Arg::W, Arg::W, Arg::W], "MODf")),        // 124
    Some((&[Arg::W, Arg::W, Arg::W], "ADDd")),        // 125
    Some((&[Arg::W, Arg::W, Arg::W], "SUBd")),        // 126
    Some((&[Arg::W, Arg::W, Arg::W], "MULd")),        // 127
    Some((&[Arg::W, Arg::W, Arg::W], "DIVd")),        // 128
    Some((&[Arg::W, Arg::W, Arg::W], "MODd")),        // 129
    Some((&[Arg::W, Arg::W, Arg::D], "ADDIi")),       // 130
    Some((&[Arg::W, Arg::W, Arg::D], "SUBIi")),       // 131
    Some((&[Arg::W, Arg::W, Arg::D], "MULIi")),       // 132
    Some((&[Arg::W, Arg::W, Arg::D], "ADDIf")),       // 133
    Some((&[Arg::W, Arg::W, Arg::D], "SUBIf")),       // 134
    Some((&[Arg::W, Arg::W, Arg::D], "MULIf")),       // 135
    Some((&[Arg::D, Arg::D], "SetG4")),               // 136
    Some((&[], "ChkRefS")),                           // 137
    Some((&[Arg::W], "ChkNullV")),                    // 138
    Some((&[Arg::D], "CALLINTF")),                    // 139
    Some((&[Arg::W], "iTOb")),                        // 140
    Some((&[Arg::W], "iTOw")),                        // 141
    Some((&[Arg::W, Arg::D], "SetV1")),               // 142
    Some((&[Arg::W, Arg::D], "SetV2")),               // 143
    Some((&[Arg::D], "Cast")),                        // 144
    Some((&[Arg::W, Arg::W], "i64TOi")),              // 145
    Some((&[Arg::W, Arg::W], "uTOi64")),              // 146
    Some((&[Arg::W, Arg::W], "iTOi64")),              // 147
    Some((&[Arg::W, Arg::W], "fTOi64")),              // 148
    Some((&[Arg::W], "dTOi64")),                      // 149
    Some((&[Arg::W, Arg::W], "fTOu64")),              // 150
    Some((&[Arg::W], "dTOu64")),                      // 151
    Some((&[Arg::W, Arg::W], "i64TOf")),              // 152
    Some((&[Arg::W, Arg::W], "u64TOf")),              // 153
    Some((&[Arg::W], "i64TOd")),                      // 154
    Some((&[Arg::W], "u64TOd")),                      // 155
    Some((&[Arg::W], "NEGi64")),                      // 156
    Some((&[], "INCi64")),                            // 157
    Some((&[], "DECi64")),                            // 158
    Some((&[Arg::W], "BNOT64")),                      // 159
    Some((&[Arg::W, Arg::W, Arg::W], "ADDi64")),      // 160
    Some((&[Arg::W, Arg::W, Arg::W], "SUBi64")),      // 161
    Some((&[Arg::W, Arg::W, Arg::W], "MULi64")),      // 162
    Some((&[Arg::W, Arg::W, Arg::W], "DIVi64")),      // 163
    Some((&[Arg::W, Arg::W, Arg::W], "MODi64")),      // 164
    Some((&[Arg::W, Arg::W, Arg::W], "BAND64")),      // 165
    Some((&[Arg::W, Arg::W, Arg::W], "BOR64")),       // 166
    Some((&[Arg::W, Arg::W, Arg::W], "BXOR64")),      // 167
    Some((&[Arg::W, Arg::W, Arg::W], "BSLL64")),      // 168
    Some((&[Arg::W, Arg::W, Arg::W], "BSRL64")),      // 169
    Some((&[Arg::W, Arg::W, Arg::W], "BSRA64")),      // 170
    Some((&[Arg::W, Arg::W], "CMPi64")),              // 171
    Some((&[Arg::W, Arg::W], "CMPu64")),              // 172
    Some((&[Arg::W], "ChkNullS")),                    // 173
    Some((&[], "ClrHi")),                             // 174
    Some((&[Arg::D], "JitEntry")),                    // 175
    Some((&[Arg::W], "CallPtr")),                     // 176
    Some((&[Arg::D], "FuncPtr")),                     // 177
    Some((&[Arg::W, Arg::D], "LoadThisR")),           // 178
    Some((&[Arg::W], "PshV8")),                       // 179
    Some((&[Arg::W, Arg::W, Arg::W], "DIVu")),        // 180
    Some((&[Arg::W, Arg::W, Arg::W], "MODu")),        // 181
    Some((&[Arg::W, Arg::W, Arg::W], "DIVu64")),      // 182
    Some((&[Arg::W, Arg::W, Arg::W], "MODu64")),      // 183
    Some((&[Arg::W, Arg::W, Arg::D], "LoadRObjR")),   // 184
    Some((&[Arg::W, Arg::W, Arg::D], "LoadVObjR")),   // 185
    Some((&[Arg::W, Arg::D], "RefCpyV")),             // 186
    Some((&[Arg::D], "JLowZ")),                       // 187
    Some((&[Arg::D], "JLowNZ")),                      // 188
    Some((&[Arg::W, Arg::D], "AllocMem")),            // 189
    Some((&[Arg::W, Arg::D, Arg::D], "SetListSize")), // 190
    Some((&[Arg::W, Arg::D], "PshListElmnt")),        // 191
    Some((&[Arg::W, Arg::D, Arg::D], "SetListType")), // 192
    Some((&[Arg::W, Arg::W, Arg::W], "POWi")),        // 193
    Some((&[Arg::W, Arg::W, Arg::W], "POWu")),        // 194
    Some((&[Arg::W, Arg::W, Arg::W], "POWf")),        // 195
    Some((&[Arg::W, Arg::W, Arg::W], "POWd")),        // 196
    Some((&[Arg::W, Arg::W, Arg::W], "POWdi")),       // 197
    Some((&[Arg::W, Arg::W, Arg::W], "POWi64")),      // 198
    Some((&[Arg::W, Arg::W, Arg::W], "POWu64")),      // 199
    Some((&[Arg::D], "Thiscall1")),                   // 200
    None,                                             // 201 unused
    None,                                             // 202 unused
    None,                                             // 203 unused
    None,                                             // 204 unused
    None,                                             // 205 unused
    None,                                             // 206 unused
    None,                                             // 207 unused
    None,                                             // 208 unused
    None,                                             // 209 unused
    None,                                             // 210 unused
    None,                                             // 211 unused
    None,                                             // 212 unused
    None,                                             // 213 unused
    None,                                             // 214 unused
    None,                                             // 215 unused
    None,                                             // 216 unused
    None,                                             // 217 unused
    None,                                             // 218 unused
    None,                                             // 219 unused
    None,                                             // 220 unused
    None,                                             // 221 unused
    None,                                             // 222 unused
    None,                                             // 223 unused
    None,                                             // 224 unused
    None,                                             // 225 unused
    None,                                             // 226 unused
    None,                                             // 227 unused
    None,                                             // 228 unused
    None,                                             // 229 unused
    None,                                             // 230 unused
    None,                                             // 231 unused
    None,                                             // 232 unused
    None,                                             // 233 unused
    None,                                             // 234 unused
    None,                                             // 235 unused
    None,                                             // 236 unused
    None,                                             // 237 unused
    None,                                             // 238 unused
    None,                                             // 239 unused
    None,                                             // 240 unused
    None,                                             // 241 unused
    None,                                             // 242 unused
    None,                                             // 243 unused
    None,                                             // 244 unused
    None,                                             // 245 unused
    None,                                             // 246 unused
    None,                                             // 247 unused
    None,                                             // 248 unused
    None,                                             // 249 unused
    Some((&[Arg::D], "TryBlock")),                    // 250
    Some((&[Arg::W], "VarDecl")),                     // 251
    None,                                             // 252 Block
    Some((&[Arg::W, Arg::D], "ObjInfo")),             // 253
    None,                                             // 254 LINE
    None,                                             // 255 LABEL
];

/// `asFUNC_SCRIPT`, `asFUNC_INTERFACE`, `asFUNC_VIRTUAL` and `asFUNC_FUNCDEF` of `asEFuncType`; 0 to 6 are defined.
const FUNC_SCRIPT: u32 = 1;
const FUNC_INTERFACE: u32 = 2;
const FUNC_VIRTUAL: u32 = 3;
const FUNC_FUNCDEF: u32 = 4;
const FUNC_LAST: u32 = 6;
/// `asOBJ_*` flags of a type declaration.
const OBJ_REF: u64 = 1 << 0;
const OBJ_VALUE: u64 = 1 << 1;
const OBJ_SCRIPT_OBJECT: u64 = 1 << 21;
const OBJ_SHARED: u64 = 1 << 22;
const OBJ_ENUM: u64 = 1 << 26;
const OBJ_TYPEDEF: u64 = 1 << 28;
/// `ttIdentifier` of 2.37's `eTokenType`: a data type naming a type rather than a primitive.
const TT_IDENTIFIER: u32 = 5;
/// The name `asCWriter` gives the delegate factory (`DELEGATE_FACTORY`); its signature is not stored.
const DELEGATE_FACTORY: &[u8] = b"$dlgte";
/// The highest `asEObjVarInfoOption` (`asOBJ_VARDECL`).
const OBJ_VAR_OPTION_LAST: u32 = 4;
/// AngelScript's `SanityCheck` bounds: on counts, and on a signature's parameters.
const MAX_COUNT: u32 = 1_000_000;
const MAX_PARAMETERS: u32 = 256;

/// A type a type-info record names: its tag and name (the name of the template for `a`, of the subtype for `s`, of
/// the type for `o` and `c`; empty for `l`).
struct TypeInfo {
    name: Vec<u8>,
    namespace: Vec<u8>,
}

/// What a function signature names: the function, its class, its type (`asEFuncType`) and namespace.
struct Signature {
    name: Vec<u8>,
    class: Option<TypeInfo>,
    kind: u32,
    namespace: Vec<u8>,
}

/// A class read in the first pass, for the passes after it.
struct Class {
    flags: u64,
    interface: bool,
    external: bool,
    name: Vec<u8>,
    namespace: Vec<u8>,
}

struct Walker<'r, 'a, 'b> {
    r: &'r mut Archive<'a, 'b>,
    bytes: Vec<u8>,
    strings: Vec<Vec<u8>>,
    data_types: usize,
    /// Every function read, in order: what `r` references index.
    functions: Vec<Function>,
    debug: bool,
    /// The external shared classes (declared by another module): their methods are not stored.
    external: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Walker<'_, '_, '_> {
    fn u8(&mut self) -> Result<u8> {
        let b = self.r.u8()?;
        self.bytes.push(b);
        Ok(b)
    }

    fn raw(&mut self, n: usize) -> Result<Vec<u8>> {
        let b = self.r.bytes(n)?.into_owned();
        self.bytes.extend_from_slice(&b);
        Ok(b)
    }

    /// `ReadEncodedUInt64`: a sign bit, then the value's length in the leading one bits of the first byte's other seven.
    fn encoded64(&mut self) -> Result<u64> {
        let first = self.u8()?;
        let negative = first & 0x80 != 0;
        let b = first & 0x7F;
        // (prefix, bytes after the first, bits of the first kept)
        let (more, keep) = match b {
            0x7F => (8, 0),
            0x7E => (6, 0x01),
            0x7C..=0x7D => (5, 0x03),
            0x78..=0x7B => (4, 0x07),
            0x70..=0x77 => (3, 0x0F),
            0x60..=0x6F => (2, 0x1F),
            0x40..=0x5F => (1, 0x3F),
            _ => (0, 0x7F),
        };
        let mut value = u64::from(b & keep);
        for _ in 0..more {
            value = (value << 8) | u64::from(self.u8()?);
        }
        Ok(if negative { value.wrapping_neg() } else { value })
    }

    /// `ReadEncodedUInt`: a 64-bit encoding whose high half is all zeros or all ones.
    fn encoded(&mut self) -> Result<u32> {
        let at = self.r.here();
        let q = self.encoded64()?;
        match q >> 32 {
            0 | 0xFFFF_FFFF => Ok(u32::try_from(q & 0xFFFF_FFFF).unwrap_or(u32::MAX)),
            _ => Err(self
                .r
                .bad_at(at, format!("encoded word {q:#x} does not fit 32 bits"))),
        }
    }

    /// `ReadEncodedUInt16`.
    fn encoded16(&mut self) -> Result<()> {
        let at = self.r.here();
        let d = self.encoded()?;
        match d >> 16 {
            0 | 0xFFFF => Ok(()),
            _ => Err(self
                .r
                .bad_at(at, format!("encoded word {d:#x} does not fit 16 bits"))),
        }
    }

    /// A count, within `max` and the bytes left (every entry takes one at least).
    fn count(&mut self, max: u32, what: &str) -> Result<usize> {
        let at = self.r.here();
        let n = self.encoded()?;
        let fits = usize::try_from(n).is_ok_and(|n| n <= self.r.remaining());
        if n > max || !fits {
            return Err(self
                .r
                .bad_at(at, format!("{n} {what}: more than the stream can hold")));
        }
        Ok(usize::try_from(n).unwrap_or(0))
    }

    /// `ReadString`: an encoded length doubled, or an odd number naming a string read before by its index.
    fn string(&mut self) -> Result<Vec<u8>> {
        let at = self.r.here();
        let n = self.encoded()?;
        let index = usize::try_from(n / 2).unwrap_or(usize::MAX);
        if n & 1 == 1 {
            return self.strings.get(index).cloned().ok_or_else(|| {
                self.r
                    .bad_at(at, format!("string {index}: {} were read", self.strings.len()))
            });
        }
        if index == 0 {
            return Ok(Vec::new());
        }
        let s = self.raw(index)?;
        self.strings.push(s.clone());
        Ok(s)
    }

    /// A name: AngelScript identifiers are ASCII, so anything not UTF-8 is refused.
    fn name(&mut self) -> Result<String> {
        let at = self.r.here();
        let s = self.string()?;
        String::from_utf8(s).map_err(|e| self.r.bad_at(at, format!("name is not UTF-8: {e}")))
    }

    fn text(&self, s: Vec<u8>, at: usize) -> Result<String> {
        String::from_utf8(s).map_err(|e| self.r.bad_at(at, format!("name is not UTF-8: {e}")))
    }

    /// `ReadDataType`: a reference to a data type read before (its index plus one), or 0 and a new one.
    fn data_type(&mut self) -> Result<()> {
        let at = self.r.here();
        let index = self.encoded()?;
        if index != 0 {
            if usize::try_from(index).map_or(true, |i| i > self.data_types) {
                return Err(self
                    .r
                    .bad_at(at, format!("data type {index}: {} were read", self.data_types)));
            }
            return Ok(());
        }
        let token = self.encoded()?;
        self.data_types = self.data_types.saturating_add(1);
        if token == TT_IDENTIFIER {
            self.type_info()?;
        }
        let flags_at = self.r.here();
        let flags = self.u8()?;
        if flags & 0xF0 != 0 {
            return Err(self.r.bad_at(
                flags_at,
                format!("data type flags {flags:#x}: bits above 3 are not defined"),
            ));
        }
        Ok(())
    }

    /// `ReadTypeInfo`: a tag (`a` template instance, `l` list pattern, `s` template subtype, `o` object type, `c` child
    /// funcdef, 0 none) and what it names.
    fn type_info(&mut self) -> Result<Option<TypeInfo>> {
        let at = self.r.here();
        Ok(Some(match self.u8()? {
            0 => return Ok(None),
            b'a' => {
                let name = self.string()?;
                let namespace = self.string()?;
                let n = self.count(MAX_COUNT, "template subtypes")?;
                for _ in 0..n {
                    if self.u8()? == b's' {
                        self.data_type()?;
                    } else {
                        self.encoded()?;
                    }
                }
                TypeInfo { name, namespace }
            }
            b'l' => {
                self.type_info()?
                    .ok_or_else(|| self.r.bad_at(at, "a list pattern of no type"))?;
                TypeInfo {
                    name: Vec::new(),
                    namespace: Vec::new(),
                }
            }
            b's' => TypeInfo {
                name: self.string()?,
                namespace: Vec::new(),
            },
            b'o' => {
                let name = self.string()?;
                let namespace = self.string()?;
                TypeInfo { name, namespace }
            }
            b'c' => {
                let name = self.string()?;
                self.type_info()?
                    .ok_or_else(|| self.r.bad_at(at, "a child funcdef of no class"))?;
                TypeInfo {
                    name,
                    namespace: Vec::new(),
                }
            }
            t => return Err(self.r.bad_at(at, format!("type info tag {t:#x}"))),
        }))
    }

    /// `ReadFunctionSignature`: the name, return and parameter types, the in/out flags and default arguments (stored
    /// only when there are parameters), the function type, the class or namespace. Returns the name, class and type.
    fn signature(&mut self) -> Result<Signature> {
        let name = self.string()?;
        if name == DELEGATE_FACTORY {
            return Ok(Signature {
                name,
                class: None,
                kind: 0,
                namespace: Vec::new(),
            });
        }
        self.data_type()?;
        let parameters = self.count(MAX_PARAMETERS, "parameters")?;
        for _ in 0..parameters {
            self.data_type()?;
        }
        if parameters > 0 {
            let flags = self.count(MAX_PARAMETERS, "in/out flags")?;
            if flags > parameters {
                return Err(self
                    .r
                    .bad(format!("{flags} in/out flags for {parameters} parameters")));
            }
            for _ in 0..flags {
                self.encoded()?;
            }
        }
        let at = self.r.here();
        let kind = self.encoded()?;
        if kind > FUNC_LAST {
            return Err(self
                .r
                .bad_at(at, format!("function type {kind}: 0 to {FUNC_LAST} are defined")));
        }
        if parameters > 0 {
            let defaults = self.count(MAX_PARAMETERS, "default arguments")?;
            if defaults > parameters {
                return Err(self.r.bad(format!(
                    "{defaults} default arguments for {parameters} parameters"
                )));
            }
            for _ in 0..defaults {
                self.string()?;
            }
        }
        let class = self.type_info()?;
        // the traits byte: a method's, or a global function's that could be a virtual property
        if class.is_some() || name.starts_with(b"get_") || name.starts_with(b"set_") {
            self.u8()?;
        }
        let mut namespace = Vec::new();
        if let Some(c) = &class {
            namespace.clone_from(&c.namespace);
        } else if kind == FUNC_FUNCDEF {
            let at = self.r.here();
            match self.u8()? {
                b'n' => namespace = self.string()?,
                b'o' => {
                    self.type_info()?;
                }
                t => {
                    return Err(self
                        .r
                        .bad_at(at, format!("funcdef owner tag {t:#x}: expected 'n' or 'o'")));
                }
            }
        } else {
            namespace = self.string()?;
        }
        Ok(Signature {
            name,
            class,
            kind,
            namespace,
        })
    }

    /// `ReadFunction`: 0 for none, `r` and the index of one read before, or `f` and a new one. Returns the function's
    /// index in [`Walker::functions`].
    fn function(&mut self) -> Result<Option<usize>> {
        let at = self.r.here();
        match self.u8()? {
            0 => return Ok(None),
            b'r' => {
                let index = self.encoded()?;
                return match usize::try_from(index) {
                    Ok(i) if i < self.functions.len() => Ok(Some(i)),
                    _ => Err(self.r.bad_at(
                        at,
                        format!("function {index}: {} were read", self.functions.len()),
                    )),
                };
            }
            b'f' => {}
            t => return Err(self.r.bad_at(at, format!("function tag {t:#x}"))),
        }
        // the slot is taken before the body: a function's own code may not reference it, but the indices are the writer's
        let index = self.functions.len();
        self.functions.push(Function {
            name: String::new(),
            namespace: String::new(),
            class: None,
        });
        let Signature {
            name,
            class,
            kind,
            namespace,
        } = self.signature()?;
        match kind {
            FUNC_SCRIPT => {
                let external = class.as_ref().is_some_and(|c| {
                    self.external
                        .iter()
                        .any(|(n, ns)| *n == c.name && *ns == c.namespace)
                });
                // a method of an external shared class is the other module's: nothing more is stored
                if !external {
                    let bits = self.u8()?;
                    // bit 2: an external shared function, whose body is the other module's
                    if bits & 4 == 0 {
                        self.body(bits)?;
                    }
                }
            }
            FUNC_VIRTUAL | FUNC_INTERFACE => {
                self.encoded()?;
            }
            FUNC_FUNCDEF => {
                self.u8()?;
            }
            _ => {}
        }
        let f = Function {
            name: self.text(name, at)?,
            namespace: self.text(namespace, at)?,
            class: class.map(|c| self.text(c.name, at)).transpose()?,
        };
        if let Some(slot) = self.functions.get_mut(index) {
            *slot = f;
        }
        Ok(Some(index))
    }

    /// A function that must be there: a funcdef, a script function, an import.
    fn required_function(&mut self, what: &str) -> Result<Function> {
        let at = self.r.here();
        let index = self.function()?;
        index
            .and_then(|i| self.functions.get(i).cloned())
            .ok_or_else(|| self.r.bad_at(at, format!("{what} of no function")))
    }

    /// A script function's body: bytecode, variable space, object variable information (bit 3), try/catch blocks (bit
    /// 4), line numbers and sections, variables, section name and parameter names.
    fn body(&mut self, bits: u8) -> Result<()> {
        self.bytecode()?;
        self.encoded()?; // variable space
        if bits & 8 != 0 {
            let n = self.count(MAX_COUNT, "object variable records")?;
            for _ in 0..n {
                self.encoded()?; // program position
                self.encoded()?; // variable offset
                let at = self.r.here();
                let option = self.encoded()?;
                if option > OBJ_VAR_OPTION_LAST {
                    return Err(self.r.bad_at(at, format!("object variable option {option}")));
                }
            }
        }
        if bits & 16 != 0 {
            let n = self.count(MAX_COUNT, "try blocks")?;
            for _ in 0..n {
                self.encoded()?;
                self.encoded()?;
            }
        }
        if self.debug {
            let n = self.count(MAX_COUNT, "line numbers")?;
            for _ in 0..n {
                self.encoded()?;
            }
            let n = self.count(MAX_COUNT, "section entries")?;
            for i in 0..n {
                if i & 1 == 0 {
                    self.encoded()?;
                } else {
                    self.string()?;
                }
            }
        }
        let n = self.count(MAX_COUNT, "variables")?;
        for _ in 0..n {
            if self.debug {
                self.encoded()?; // declared at
                self.string()?;
            }
            self.encoded()?; // stack offset and on-heap bit
            self.data_type()?;
        }
        if self.debug {
            self.string()?; // section
            self.encoded()?; // declared at
            let at = self.r.here();
            let n = self.encoded64()?;
            if n > u64::from(MAX_PARAMETERS) {
                return Err(self.r.bad_at(at, format!("{n} parameter names")));
            }
            for _ in 0..n {
                self.string()?;
            }
        }
        Ok(())
    }

    /// `ReadByteCode`: an instruction count, then each instruction's code and its operands, encoded.
    fn bytecode(&mut self) -> Result<()> {
        let n = self.count(MAX_COUNT, "instructions")?;
        for _ in 0..n {
            let at = self.r.here();
            let code = self.u8()?;
            let Some((args, _)) = INSTRUCTIONS.get(usize::from(code)).copied().flatten() else {
                return Err(self
                    .r
                    .bad_at(at, format!("instruction {code}: not one the writer stores")));
            };
            for arg in args {
                match arg {
                    Arg::W => self.encoded16()?,
                    Arg::D => {
                        self.encoded()?;
                    }
                    Arg::Q => {
                        self.encoded64()?;
                    }
                }
            }
        }
        Ok(())
    }

    /// `ReadTypeDeclaration` phase 1: name, flags, size, namespace and, for a shared type, whether it is external.
    fn type_head(&mut self) -> Result<Class> {
        let name = self.string()?;
        let flags = u64::from_le_bytes(
            <[u8; 8]>::try_from(self.raw(8)?.as_slice()).map_err(|_| self.r.bad("type flags"))?,
        );
        let size = self.encoded()?;
        let namespace = self.string()?;
        let mut external = false;
        if flags & OBJ_SHARED != 0 {
            let at = self.r.here();
            match self.u8()? {
                b'e' => external = true,
                b' ' => {}
                t => {
                    return Err(self
                        .r
                        .bad_at(at, format!("shared type tag {t:#x}: expected 'e' or ' '")));
                }
            }
        }
        Ok(Class {
            flags,
            interface: flags & OBJ_SCRIPT_OBJECT != 0 && size == 0,
            external,
            name,
            namespace,
        })
    }

    fn declared(&self, name: &[u8], namespace: &[u8], at: usize) -> Result<Declared> {
        Ok(Declared {
            name: self.text(name.to_vec(), at)?,
            namespace: self.text(namespace.to_vec(), at)?,
        })
    }

    /// Phase 2 of a class: base class, interfaces, and for a class (not an interface) the destructor and the
    /// constructors with their factories; then the methods and the virtual function table.
    fn class_body(&mut self, class: &Class) -> Result<()> {
        self.type_info()?;
        let n = self.count(MAX_COUNT, "interfaces")?;
        for _ in 0..n {
            self.type_info()?;
            if !class.interface {
                self.encoded()?; // offset of the interface's methods in the virtual table
            }
        }
        if !class.interface && class.flags != OBJ_TYPEDEF && class.flags != OBJ_ENUM {
            self.function()?; // destructor
            let n = self.count(MAX_COUNT, "constructors")?;
            for _ in 0..n {
                self.function()?;
                self.function()?; // its factory
            }
        }
        for what in ["methods", "virtual functions"] {
            let n = self.count(MAX_COUNT, what)?;
            for _ in 0..n {
                self.function()?;
            }
        }
        Ok(())
    }

    /// `asCReader::ReadInner`.
    fn module(&mut self) -> Result<SavedModule> {
        self.debug = self.encoded()? == 0;
        let mut m = SavedModule {
            bytes: Vec::new(),
            debug_info: self.debug,
            enums: Vec::new(),
            classes: Vec::new(),
            funcdefs: Vec::new(),
            typedefs: Vec::new(),
            globals: Vec::new(),
            functions: Vec::new(),
            imports: Vec::new(),
            string_constants: Vec::new(),
        };
        let n = self.count(MAX_COUNT, "enums")?;
        for _ in 0..n {
            let at = self.r.here();
            let e = self.type_head()?;
            if e.flags & OBJ_ENUM == 0 {
                return Err(self
                    .r
                    .bad_at(at, format!("enum flags {:#x} lack asOBJ_ENUM", e.flags)));
            }
            m.enums.push(self.declared(&e.name, &e.namespace, at)?);
            if !e.external {
                let values = self.count(MAX_COUNT, "enum values")?;
                for _ in 0..values {
                    self.string()?;
                    self.raw(4)?;
                }
            }
        }
        let n = self.count(MAX_COUNT, "classes")?;
        let mut classes = Vec::with_capacity(n);
        for _ in 0..n {
            let at = self.r.here();
            let c = self.type_head()?;
            if c.flags & (OBJ_REF | OBJ_VALUE) == 0 {
                return Err(self.r.bad_at(
                    at,
                    format!("class flags {:#x} are neither asOBJ_REF nor asOBJ_VALUE", c.flags),
                ));
            }
            if c.external {
                self.external.push((c.name.clone(), c.namespace.clone()));
            }
            m.classes.push(self.declared(&c.name, &c.namespace, at)?);
            classes.push(c);
        }
        let n = self.count(MAX_COUNT, "funcdefs")?;
        for _ in 0..n {
            let f = self.required_function("a funcdef")?;
            m.funcdefs.push(Declared {
                name: f.name,
                namespace: f.namespace,
            });
        }
        // interfaces first, then classes; then the classes' properties
        for interfaces in [true, false] {
            for c in classes
                .iter()
                .filter(|c| c.interface == interfaces && !c.external)
            {
                self.class_body(c)?;
            }
        }
        for _ in classes.iter().filter(|c| !c.interface && !c.external) {
            let n = self.count(MAX_COUNT, "properties")?;
            for _ in 0..n {
                self.string()?;
                self.data_type()?;
                self.encoded()?; // private, protected, inherited
            }
        }
        let n = self.count(MAX_COUNT, "typedefs")?;
        for _ in 0..n {
            let at = self.r.here();
            let t = self.type_head()?;
            m.typedefs.push(self.declared(&t.name, &t.namespace, at)?);
            self.encoded()?; // the primitive it names
        }
        let n = self.count(MAX_COUNT, "global variables")?;
        for _ in 0..n {
            let at = self.r.here();
            let name = self.string()?;
            let namespace = self.string()?;
            m.globals.push(self.declared(&name, &namespace, at)?);
            self.data_type()?;
            self.function()?; // its initialisation
        }
        let n = self.count(MAX_COUNT, "script functions")?;
        for _ in 0..n {
            let f = self.required_function("a script function")?;
            m.functions.push(f);
        }
        let n = self.count(MAX_COUNT, "global functions")?;
        for _ in 0..n {
            self.function()?;
        }
        let n = self.count(MAX_COUNT, "imported functions")?;
        for _ in 0..n {
            let f = self.required_function("an import")?;
            let module = self.name()?;
            m.imports.push(Import { name: f.name, module });
        }
        let n = self.count(MAX_COUNT, "used types")?;
        for _ in 0..n {
            self.type_info()?;
        }
        let n = self.count(MAX_COUNT, "used type ids")?;
        for _ in 0..n {
            self.data_type()?;
        }
        let n = self.count(MAX_COUNT, "used functions")?;
        for _ in 0..n {
            // 'n' for none, otherwise 'm' (the module's) or 'a' (the application's) and a signature
            if self.u8()? != b'n' {
                self.signature()?;
            }
        }
        let n = self.count(MAX_COUNT, "used global variables")?;
        for _ in 0..n {
            self.string()?;
            self.string()?;
            self.data_type()?;
            self.u8()?; // the module's (1) or the application's (0)
        }
        let n = self.count(MAX_COUNT, "string constants")?;
        for _ in 0..n {
            let s = self.string()?;
            m.string_constants.push(s);
        }
        let n = self.count(MAX_COUNT, "used object properties")?;
        for _ in 0..n {
            self.type_info()?;
            self.string()?;
        }
        Ok(m)
    }
}

impl SavedModule {
    /// Reads a saved module from `r`, which is left after its last byte.
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<SavedModule> {
        let mut w = Walker {
            r,
            bytes: Vec::new(),
            strings: Vec::new(),
            data_types: 0,
            functions: Vec::new(),
            debug: false,
            external: Vec::new(),
        };
        let mut m = w.module()?;
        m.bytes = w.bytes;
        Ok(m)
    }
}
