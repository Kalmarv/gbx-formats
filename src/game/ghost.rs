//! A ghost (`CGameCtnGhost`, with the chunks of its base class `CGameGhost`) and the MediaTracker block that plays one
//! (`CGameCtnMediaBlockGhost`).
//!
//! A ghost's replay is the sample stream of `CGameGhost` chunk `0x0303F005`: a zlib stream that holds the vehicle
//! class, the sample period, and one record of vehicle state per sample. A record is read field by field in its stored
//! form (the packed fields are not unpacked) for the one vehicle state version the corpus holds.
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::archive::{Archive, Id, Ident, PackDesc, State};
use crate::error::{Error, Result, Space};
use crate::game::nullable;
use crate::node::{Chunks, NodeIndex, NodeTable};
use primase_foundations::compress::{Decompressor, Zlib};

/// The vehicle classes whose records this crate reads: `CSceneVehicleCar` of two engine versions.
const VEHICLE_CLASSES: [u32; 2] = [0x0A02_B000, 0x0A10_3000];
/// The vehicle state versions read: the one every ghost of the corpus stores (`SVehicleSimpleState_ReplayAfter081205`,
/// 61 bytes a record).
const STATE_VERSIONS: [i32; 1] = [9];
/// The size of a record of state version 9.
const RECORD_SIZE: usize = 61;

/// The MediaTracker block that plays a ghost (`CGameCtnMediaBlockGhost`, class `0x030E5000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaBlockGhost {
    /// Chunk `0x030E5001`: the time the block starts and ends, in seconds, the ghost it plays, and the offset
    /// into the ghost's run (seconds) where the block starts playing it.
    pub ghost: Option<(f32, f32, Option<NodeIndex>, f32)>,
    /// The node's chunks.
    pub chunks: Chunks,
}

impl MediaBlockGhost {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<MediaBlockGhost> {
        let mut n = MediaBlockGhost::default();
        n.chunks = r.chunks("CGameCtnMediaBlockGhost", |r, id| {
            match id {
                0x030E_5001 => n.ghost = Some((r.f32()?, r.f32()?, r.node_ref()?, r.f32()?)),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}

/// How the sizes of the records in the state buffer are stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordSizes {
    /// One sample: no sizes are stored, the record is the whole buffer.
    Single,
    /// Every record but the last has this size (stored as the size, which is not -1).
    Uniform(i32),
    /// The size of each record but the last (stored as -1 and then the sizes).
    Each(Vec<i32>),
}

/// A ghost's sample stream (`CGameGhost` chunk `0x0303F005`, read as GBX.NET's `Data.ReadNew` with version 0).
#[derive(Clone, Debug, PartialEq)]
pub struct Samples {
    /// The class of the vehicle the records are of: `0x0A02B000` or `0x0A103000` (`CSceneVehicleCar`).
    pub vehicle_class: u32,
    /// Whether the records are a fixed period apart; if not, `times` holds each one's time.
    pub fixed_time_step: bool,
    /// A `u32` GBX.NET reads as `U01` and does not interpret.
    pub u01: i32,
    /// The time between two records, in milliseconds.
    pub period: i32,
    /// The version of the vehicle state the records are of (`SVehicleSimpleState_Replay…`): 9.
    pub state_version: i32,
    /// The offset stored for the first record, when there is one.
    pub first_offset: Option<i32>,
    /// How the record sizes are stored.
    pub sizes: RecordSizes,
    /// Each record's time in milliseconds, stored when the step is not fixed.
    pub times: Option<Vec<i32>>,
    /// The vehicle's state at each sample.
    pub records: Vec<CarState>,
}

/// One wheel of a [`CarState`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WheelState {
    /// The suspension's length, `0..=255` for -2..2 (as stored).
    pub dampen_len: u8,
    /// The `CPlugSurface.MaterialId` of the ground under the wheel.
    pub ground_contact_material: u8,
}

/// A car's state at one sample, version 9: 61 bytes, every field as stored (the packed ones, a position's rotation and
/// the velocities, are not unpacked).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarState {
    /// The position.
    pub position: [f32; 3],
    /// The rotation's angle, `0..=65535` for `0..π`.
    pub rotation_angle: u16,
    /// The rotation's axis, a packed unit vector (GBX.NET's `Vec3Unit4`).
    pub rotation_axis: u32,
    /// The velocity, a packed vector (GBX.NET's `Vec3_4`).
    pub velocity: u32,
    /// The angular velocity, packed like the velocity.
    pub angular_velocity: u32,
    /// The speed along the car, `0..=65535` for -1000..10000 m/s.
    pub speed_forward: u16,
    /// The speed across the car, `0..=65535` for -1000..1000 m/s.
    pub speed_sideward: u16,
    /// The engine's rpm, `0..=65535` for 0..30000.
    pub rpm: u16,
    /// The rotation of the front left, front right, rear right and rear left wheels, `0..=65535` for 0..1608.495.
    pub wheel_rotation: [u16; 4],
    /// The steering, `0..=255` for -1..1.
    pub steer: u8,
    /// The throttle, `0..=255` for 0..1.
    pub gas: u8,
    /// The brake, `0..=255` for 0..1.
    pub brake: u8,
    /// A byte GBX.NET reads as `U11`.
    pub u11: u8,
    /// A byte GBX.NET reads as `U12`, "always 0 but sometimes it isn't".
    pub u12: u8,
    /// A byte GBX.NET reads as `U13`.
    pub u13: u8,
    /// A byte GBX.NET reads as `U14`.
    pub u14: u8,
    /// The turbo's strength, `0..=255` for 0..1.
    pub turbo_strength: u8,
    /// The front wheels' steering angle, `0..=255` for -π..π.
    pub steer_front: u8,
    /// The front left, front right, rear right and rear left wheels.
    pub wheels: [WheelState; 4],
    /// A byte of flags GBX.NET reads as `U25` (the horn is bits 3 and 4).
    pub u25: u8,
    /// A byte of flags: the front left wheel's sliding (bit 6) and on-ground (bit 7) among them.
    pub u26: u8,
    /// A byte of flags: the other wheels' sliding (bits 0, 2, 4) and on-ground (bits 1, 3, 5) among them.
    pub u27: u8,
    /// The dirt on the car, `0..=255` for 0..1.
    pub dirt_blend: u8,
}

impl CarState {
    fn read(r: &mut Archive<'_, '_>) -> Result<CarState> {
        let position = r.vec3()?;
        let rotation_angle = r.u16()?;
        let rotation_axis = r.u32()?;
        let velocity = r.u32()?;
        let angular_velocity = r.u32()?;
        let speed_forward = r.u16()?;
        let speed_sideward = r.u16()?;
        let rpm = r.u16()?;
        let wheel_rotation = [r.u16()?, r.u16()?, r.u16()?, r.u16()?];
        let steer = r.u8()?;
        let gas = r.u8()?;
        let brake = r.u8()?;
        let u11 = r.u8()?;
        let u12 = r.u8()?;
        let u13 = r.u8()?;
        let u14 = r.u8()?;
        let turbo_strength = r.u8()?;
        let steer_front = r.u8()?;
        let wheel = |r: &mut Archive<'_, '_>| -> Result<WheelState> {
            Ok(WheelState {
                dampen_len: r.u8()?,
                ground_contact_material: r.u8()?,
            })
        };
        let wheels = [wheel(r)?, wheel(r)?, wheel(r)?, wheel(r)?];
        let state = CarState {
            position,
            rotation_angle,
            rotation_axis,
            velocity,
            angular_velocity,
            speed_forward,
            speed_sideward,
            rpm,
            wheel_rotation,
            steer,
            gas,
            brake,
            u11,
            u12,
            u13,
            u14,
            turbo_strength,
            steer_front,
            wheels,
            u25: r.u8()?,
            u26: r.u8()?,
            u27: r.u8()?,
            dirt_blend: r.u8()?,
        };
        r.end("a ghost sample")?;
        Ok(state)
    }
}

impl Samples {
    /// Reads the decompressed stream of chunk `0x0303F005`.
    fn read(r: &mut Archive<'_, '_>) -> Result<Samples> {
        let vehicle_class = r.u32()?;
        if !VEHICLE_CLASSES.contains(&vehicle_class) {
            return Err(r.bad(format!(
                "ghost vehicle class {vehicle_class:#010x}: expected one of {VEHICLE_CLASSES:#010x?}"
            )));
        }
        let fixed_time_step = r.bool()?;
        let u01 = r.i32()?;
        let period = r.i32()?;
        let version_at = r.here();
        let state_version = r.i32()?;
        if !STATE_VERSIONS.contains(&state_version) {
            return Err(r.bad_at(
                version_at,
                format!("ghost vehicle state version {state_version}: expected one of {STATE_VERSIONS:?}"),
            ));
        }
        let buffer = r.data()?;
        let count_at = r.here();
        let count = r.i32()?;
        let count =
            usize::try_from(count).map_err(|_| r.bad_at(count_at, format!("{count} ghost samples")))?;
        let mut first_offset = None;
        let mut sizes = RecordSizes::Single;
        if count > 0 {
            first_offset = Some(r.i32()?);
            if count > 1 {
                let size_at = r.here();
                let size = r.i32()?;
                sizes = if size == -1 {
                    let each = (1..count).map(|_| r.i32()).collect::<Result<Vec<_>>>()?;
                    RecordSizes::Each(each)
                } else if size < 0 {
                    return Err(r.bad_at(size_at, format!("ghost sample size {size}")));
                } else {
                    RecordSizes::Uniform(size)
                };
            }
        }
        let times = if fixed_time_step {
            None
        } else {
            let at = r.here();
            let times = r.list(4, |r| r.i32())?;
            if times.len() != count {
                return Err(r.bad_at(
                    at,
                    format!("{} ghost sample times for {count} samples", times.len()),
                ));
            }
            Some(times)
        };
        r.end("the ghost samples")?;

        let mut records = Vec::new();
        let mut rest = buffer;
        for i in 0..count {
            let size = if i.saturating_add(1) == count {
                rest.len()
            } else {
                let size = match &sizes {
                    RecordSizes::Uniform(s) => Some(*s),
                    RecordSizes::Each(each) => each.get(i).copied(),
                    RecordSizes::Single => None,
                };
                size.and_then(|s| usize::try_from(s).ok())
                    .ok_or_else(|| r.bad(format!("ghost sample {i} has no stored size")))?
            };
            let (record, tail) = rest.split_at_checked(size).ok_or_else(|| {
                r.bad(format!(
                    "ghost sample {i}: {size} bytes, {} left in the state buffer",
                    rest.len()
                ))
            })?;
            if record.len() != RECORD_SIZE {
                return Err(r.bad(format!(
                    "ghost sample {i}: {} bytes, expected {RECORD_SIZE}",
                    record.len()
                )));
            }
            let origin = r.here();
            let mut one = Archive::new(record, origin, r.space(), &mut *r.st);
            records.push(CarState::read(&mut one)?);
            rest = tail;
        }
        if count == 0 && !buffer.is_empty() {
            return Err(r.bad(format!(
                "no ghost samples but a state buffer of {} bytes",
                buffer.len()
            )));
        }
        Ok(Samples {
            vehicle_class,
            fixed_time_step,
            u01,
            period,
            state_version,
            first_offset,
            sizes,
            times,
            records,
        })
    }

    /// Chunk `0x0303F005`'s data: the uncompressed size, the compressed bytes, which are then read as a sample stream.
    fn read_chunk(r: &mut Archive<'_, '_>) -> Result<Samples> {
        let at = r.here();
        let size = r.u32()?;
        let packed = r.data()?;
        let size = usize::try_from(size).map_err(|_| r.bad_at(at, "sample stream size"))?;
        let bytes = Zlib.decompress(packed, size)?;
        let mut st = State::new(NodeTable::new(0, Vec::new()));
        let mut sub = Archive::new(&bytes, 0, Space::Body, &mut st);
        Samples::read(&mut sub).map_err(|e| match e {
            Error::Format { offset, detail, .. } => r.bad_at(
                at,
                format!("in the sample stream of this chunk, byte {offset}: {detail}"),
            ),
            other => other,
        })
    }
}

/// The data of chunk `0x0304F001`, which only TMF stores: a `u32` and counted bytes that look like cipher text (their
/// entropy is that of random bytes, and two ghosts of a map have different ones). Kept as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tmcp {
    /// The `u32` before the bytes.
    pub key: u32,
    /// The counted bytes.
    pub data: Vec<u8>,
}

/// One checkpoint of a ghost's run (chunk `0x0309200B`, checkpoint version 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    /// The time in milliseconds, `None` for -1.
    pub time: Option<u32>,
    /// The stunt score, `None` for -1.
    pub stunts_score: Option<u32>,
}

/// A ghost (`CGameCtnGhost`, class `0x03092000`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ghost {
    /// `CGameGhost` chunk `0x0303F005`: the sample stream.
    pub samples: Option<Samples>,
    /// Chunk `0x0304F001`.
    pub tmcp: Option<Tmcp>,
    /// Chunk `0x03092005`: the race time in milliseconds, `None` for -1.
    pub race_time: Option<Option<u32>>,
    /// Chunk `0x03092008`: the respawn count, `None` for -1.
    pub respawns: Option<Option<u32>>,
    /// Chunk `0x03092009`: the light trail colour.
    pub light_trail_color: Option<[f32; 3]>,
    /// Chunk `0x0309200A`: the stunt score, `None` for -1.
    pub stunt_score: Option<Option<u32>>,
    /// Chunk `0x0309200B`: the checkpoints.
    pub checkpoints: Option<Vec<Checkpoint>>,
    /// Chunk `0x0309200C`: an `i32` GBX.NET does not interpret.
    pub u0c: Option<i32>,
    /// Chunk `0x0309200E`: the ghost's uid.
    pub ghost_uid: Option<Id>,
    /// Chunk `0x0309200F`: the player's login.
    pub ghost_login: Option<String>,
    /// Chunk `0x03092010`: the uid of the map the ghost was validated on.
    pub validate_challenge_uid: Option<Id>,
    /// Chunk `0x03092012`: a `u32` and the 128-bit security key.
    pub security_key: Option<(u32, [u8; 16])>,
    /// Chunk `0x03092013`: two `i32`s GBX.NET does not interpret.
    pub u13: Option<[i32; 2]>,
    /// Chunk `0x03092014`: the ghost version (`TMr.n`), `None` for -1.
    pub ghost_version: Option<Option<u32>>,
    /// Chunk `0x03092015`: the player's nickname (with Nadeo's formatting codes).
    pub ghost_nickname: Option<Id>,
    /// Chunk `0x03092017`: the skin packs, the nickname (formatted) and the avatar name.
    pub metadata: Option<GhostMetadata>,
    /// Chunk `0x03092018`: the player model.
    pub player_model: Option<Ident>,
    /// Chunk `0x03092019`: the validation data of the ghost's run.
    pub validation: Option<Validation>,
    /// The node's chunks.
    pub chunks: Chunks,
}

/// Chunk `0x03092017`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GhostMetadata {
    /// The skin packs.
    pub skin_packs: Vec<PackDesc>,
    /// The nickname, with formatting codes.
    pub nickname: String,
    /// The avatar's name.
    pub avatar_name: String,
}

/// One input event of a ghost, as stored (nine bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputEvent {
    /// The time as stored: GBX.NET subtracts 100000 to get milliseconds.
    pub time: u32,
    /// The index of the input's name in [`Events::names`].
    pub name: u8,
    /// The input's data: what the name's input makes of it (a key's state, a steering value).
    pub data: u32,
}

/// The input events and validation data of a ghost whose events last more than 0 milliseconds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Events {
    /// The input store version, always 0.
    pub input_store_version: i32,
    /// The names of the inputs the events are of.
    pub names: Vec<Id>,
    /// The most inputs the store holds.
    pub input_count_limit: i32,
    /// The events.
    pub inputs: Vec<InputEvent>,
    /// The game's version string.
    pub exe_version: String,
    /// The executable's checksum.
    pub exe_checksum: u32,
    /// The operating system's kind (`EKindOs`).
    pub os_kind: i32,
    /// The processor's kind (`EKindCpu`).
    pub cpu_kind: i32,
    /// The race settings.
    pub race_settings: String,
}

/// Chunk `0x03092019`: the validation data of the ghost's run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Validation {
    /// How long the events last, in milliseconds.
    pub events_duration: u32,
    /// The events and the validation data, stored when the duration is not 0.
    pub events: Option<Events>,
    /// The validation seed, stored when the duration is not 0.
    pub seed: Option<i32>,
}

impl Validation {
    fn read(r: &mut Archive<'_, '_>) -> Result<Validation> {
        let events_duration = r.u32()?;
        if events_duration == 0 {
            return Ok(Validation {
                events_duration,
                events: None,
                seed: None,
            });
        }
        let input_store_version = r.i32()?;
        if input_store_version != 0 {
            return Err(r.bad(format!("input store version {input_store_version}: expected 0")));
        }
        let names = r.list(4, |r| r.id())?;
        let count = r.count(9)?;
        let input_count_limit = r.i32()?;
        let inputs = (0..count)
            .map(|_| {
                let time = r.u32()?;
                let name = r.u8()?;
                if usize::from(name) >= names.len() {
                    return Err(r.bad(format!("input name {name} of {} names", names.len())));
                }
                Ok(InputEvent {
                    time,
                    name,
                    data: r.u32()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let events = Events {
            input_store_version,
            names,
            input_count_limit,
            inputs,
            exe_version: r.string()?,
            exe_checksum: r.u32()?,
            os_kind: r.i32()?,
            cpu_kind: r.i32()?,
            race_settings: r.string()?,
        };
        Ok(Validation {
            events_duration,
            events: Some(events),
            seed: Some(r.i32()?),
        })
    }
}

impl Ghost {
    pub(crate) fn read(r: &mut Archive<'_, '_>) -> Result<Ghost> {
        let mut n = Ghost::default();
        n.chunks = r.chunks("CGameCtnGhost", |r, id| {
            match id {
                0x0303_F005 => n.samples = Some(Samples::read_chunk(r)?),
                0x0304_F001 => {
                    r.skippable(|r| {
                        n.tmcp = Some(Tmcp {
                            key: r.u32()?,
                            data: r.data()?.to_vec(),
                        });
                        Ok(())
                    })?;
                }
                0x0309_2005 => n.race_time = Some(r.skippable(|r| nullable(r))?),
                0x0309_2008 => n.respawns = Some(r.skippable(|r| nullable(r))?),
                0x0309_2009 => n.light_trail_color = Some(r.skippable(|r| r.vec3())?),
                0x0309_200A => n.stunt_score = Some(r.skippable(|r| nullable(r))?),
                0x0309_200B => {
                    n.checkpoints = Some(r.skippable(|r| {
                        r.list(8, |r| {
                            Ok(Checkpoint {
                                time: nullable(r)?,
                                stunts_score: nullable(r)?,
                            })
                        })
                    })?);
                }
                0x0309_200C => n.u0c = Some(r.i32()?),
                0x0309_200E => n.ghost_uid = Some(r.id()?),
                0x0309_200F => n.ghost_login = Some(r.string()?),
                0x0309_2010 => n.validate_challenge_uid = Some(r.id()?),
                0x0309_2012 => n.security_key = Some((r.u32()?, r.array()?)),
                0x0309_2013 => n.u13 = Some(r.skippable(|r| Ok([r.i32()?, r.i32()?]))?),
                0x0309_2014 => n.ghost_version = Some(r.skippable(|r| nullable(r))?),
                0x0309_2015 => n.ghost_nickname = Some(r.id()?),
                0x0309_2017 => {
                    n.metadata = Some(r.skippable(|r| {
                        Ok(GhostMetadata {
                            skin_packs: r.list(1, |r| r.pack_desc())?,
                            nickname: r.string()?,
                            avatar_name: r.string()?,
                        })
                    })?);
                }
                0x0309_2018 => n.player_model = Some(r.ident()?),
                0x0309_2019 => n.validation = Some(Validation::read(r)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        Ok(n)
    }
}
