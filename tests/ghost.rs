//! Ghosts built by hand, byte by byte, from GBX.NET's documented layouts: a `CGameCtnGhost` with every chunk the corpus
//! holds (the sample stream a stored zlib block), the media block that plays one, and the refusals.
#![allow(clippy::unwrap_used, reason = "hand-built test fixtures")]
use gbx_formats::game::ghost::{Ghost, MediaBlockGhost, RecordSizes};
use gbx_formats::{Gbx, Id, Node, NodeIndex, Nodes};

fn words(out: &mut Vec<u8>, values: &[u32]) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}

fn string(out: &mut Vec<u8>, s: &str) {
    words(out, &[u32::try_from(s.len()).unwrap()]);
    out.extend_from_slice(s.as_bytes());
}

/// A file with `body` as its one node of class `class`.
fn file(class: u32, body: &[u8]) -> Vec<u8> {
    let mut out = b"GBX\x06\x00BUUR".to_vec();
    words(&mut out, &[class, 0, 16, 0]);
    out.extend_from_slice(body);
    out
}

fn read(class: u32, body: &[u8]) -> gbx_formats::Result<Nodes> {
    Gbx::parse(&file(class, body)).unwrap().nodes()
}

/// A car state of version 9: 61 bytes, a position and then byte `seed + i` at offset `12 + i`.
fn record(seed: u8) -> Vec<u8> {
    let mut r = Vec::new();
    for f in [1.5f32, 2.5, 3.5] {
        r.extend_from_slice(&f.to_le_bytes());
    }
    r.extend((0..49u8).map(|i| seed.wrapping_add(i)));
    assert_eq!(r.len(), 61);
    r
}

/// A zlib stream of one stored block.
fn zlib(data: &[u8]) -> Vec<u8> {
    let len = u16::try_from(data.len()).unwrap();
    let mut out = vec![0x78, 0x01, 0x01];
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(data);
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    out.extend_from_slice(&((b << 16) | a).to_be_bytes());
    out
}

/// A fixed-step sample stream of the given records: all of one size when `sizes` is `None`, listed otherwise.
fn samples(class: u32, version: u32, records: &[Vec<u8>], sizes: Option<&[u32]>) -> Vec<u8> {
    let mut s = Vec::new();
    words(&mut s, &[class, 1, 0, 100, version]);
    let buffer: Vec<u8> = records.concat();
    words(&mut s, &[u32::try_from(buffer.len()).unwrap()]);
    s.extend_from_slice(&buffer);
    words(&mut s, &[u32::try_from(records.len()).unwrap(), 0]);
    match sizes {
        None => words(&mut s, &[61]),
        Some(sizes) => {
            words(&mut s, &[u32::MAX]);
            words(&mut s, sizes);
        }
    }
    s
}

/// Chunk `0x0303F005` around a sample stream.
fn samples_chunk(stream: &[u8]) -> Vec<u8> {
    let packed = zlib(stream);
    let mut c = Vec::new();
    words(&mut c, &[0x0303_F005, u32::try_from(stream.len()).unwrap()]);
    words(&mut c, &[u32::try_from(packed.len()).unwrap()]);
    c.extend_from_slice(&packed);
    c
}

fn skippable(out: &mut Vec<u8>, id: u32, data: &[u8]) {
    words(out, &[id, 0x534B_4950, u32::try_from(data.len()).unwrap()]);
    out.extend_from_slice(data);
}

fn data_of(f: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut d = Vec::new();
    f(&mut d);
    d
}

/// The validation chunk: `duration` ms of events (two names, two inputs when not 0).
fn validation(duration: u32, store_version: u32) -> Vec<u8> {
    let mut b = Vec::new();
    words(&mut b, &[0x0309_2019, duration]);
    if duration == 0 {
        return b;
    }
    words(&mut b, &[store_version, 2, 0x4000_0000]);
    string(&mut b, "Accelerate");
    words(&mut b, &[0x4000_0000]);
    string(&mut b, "Steer");
    words(&mut b, &[2, 8]);
    // the events: time (stored with 100000 added), name index, data
    for (time, name, data) in [(100_010u32, 0u8, 1u32), (100_020, 1, 0xFFFF)] {
        words(&mut b, &[time]);
        b.push(name);
        words(&mut b, &[data]);
    }
    string(&mut b, "TmForever.2.11.26");
    words(&mut b, &[0xDEAD_BEEF, 10, 2]);
    string(&mut b, "<id>Unassigned</id>");
    words(&mut b, &[0]);
    b
}

/// A ghost's chunks as a TMF map stores them, ended by `0xFACADE01`.
fn ghost_body(stream: &[u8], validation: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    skippable(
        &mut b,
        0x0304_F001,
        &data_of(|d| {
            words(d, &[0x45AF_3775, 3]);
            d.extend_from_slice(&[9, 8, 7]);
        }),
    );
    b.extend_from_slice(&samples_chunk(stream));
    skippable(&mut b, 0x0309_2005, &12400u32.to_le_bytes());
    skippable(&mut b, 0x0309_2008, &u32::MAX.to_le_bytes());
    skippable(
        &mut b,
        0x0309_2009,
        &data_of(|d| {
            for f in [0.0f32, 0.5, 1.0] {
                d.extend_from_slice(&f.to_le_bytes());
            }
        }),
    );
    skippable(&mut b, 0x0309_200A, &5u32.to_le_bytes());
    skippable(
        &mut b,
        0x0309_200B,
        &data_of(|d| words(d, &[2, 3000, 0, 12400, u32::MAX])),
    );
    words(&mut b, &[0x0309_200C, 0]);
    words(&mut b, &[0x0309_200E, 3, 0x4000_0000]);
    string(&mut b, "uid");
    words(&mut b, &[0x0309_200F]);
    string(&mut b, "login");
    words(&mut b, &[0x0309_2010, 0x4000_0001]);
    words(&mut b, &[0x0309_2012, 7, 1, 2, 3, 4]);
    skippable(&mut b, 0x0309_2013, &[0; 8]);
    skippable(&mut b, 0x0309_2014, &7u32.to_le_bytes());
    words(&mut b, &[0x0309_2015, 0x4000_0000]);
    string(&mut b, "$fffNick");
    skippable(
        &mut b,
        0x0309_2017,
        &data_of(|d| {
            words(d, &[1]);
            d.push(2);
            string(d, "Skins\\Car.zip");
            string(d, "https://example.org/car.zip");
            string(d, "$fffNick");
            string(d, "avatar");
        }),
    );
    words(&mut b, &[0x0309_2018, 0x4000_0000]);
    string(&mut b, "StadiumCar");
    words(&mut b, &[u32::MAX, 0x4000_0003]);
    b.extend_from_slice(validation);
    words(&mut b, &[0xFACA_DE01]);
    b
}

fn good_stream() -> Vec<u8> {
    samples(0x0A02_B000, 9, &[record(0), record(1), record(2)], None)
}

const GHOST: u32 = 0x0309_2000;

#[test]
fn a_ghost_reads_field_by_field() {
    let body = ghost_body(&good_stream(), &validation(5000, 0));
    let nodes = read(GHOST, &body).unwrap();
    let g: &Ghost = nodes.main().try_into().unwrap();
    assert_eq!(
        g.chunks.ids,
        [
            0x0304_F001,
            0x0303_F005,
            0x0309_2005,
            0x0309_2008,
            0x0309_2009,
            0x0309_200A,
            0x0309_200B,
            0x0309_200C,
            0x0309_200E,
            0x0309_200F,
            0x0309_2010,
            0x0309_2012,
            0x0309_2013,
            0x0309_2014,
            0x0309_2015,
            0x0309_2017,
            0x0309_2018,
            0x0309_2019
        ]
    );
    assert!(g.chunks.unread.is_empty());
    let tmcp = g.tmcp.as_ref().unwrap();
    assert_eq!((tmcp.key, &tmcp.data[..]), (0x45AF_3775, &[9, 8, 7][..]));
    let s = g.samples.as_ref().unwrap();
    assert_eq!(
        (
            s.vehicle_class,
            s.fixed_time_step,
            s.period,
            s.state_version,
            s.first_offset
        ),
        (0x0A02_B000, true, 100, 9, Some(0))
    );
    assert_eq!(s.sizes, RecordSizes::Uniform(61));
    assert_eq!(s.times, None);
    assert_eq!(s.records.len(), 3);
    // record 1 holds bytes 1, 2, 3 ... after its position
    let r = &s.records[1];
    assert_eq!(r.position, [1.5, 2.5, 3.5]);
    assert_eq!((r.rotation_angle, r.rotation_axis), (0x0201, 0x0605_0403));
    assert_eq!((r.velocity, r.angular_velocity), (0x0A09_0807, 0x0E0D_0C0B));
    assert_eq!(
        (r.speed_forward, r.speed_sideward, r.rpm),
        (0x100F, 0x1211, 0x1413)
    );
    assert_eq!(r.wheel_rotation, [0x1615, 0x1817, 0x1A19, 0x1C1B]);
    assert_eq!(
        (r.steer, r.gas, r.brake, r.u11, r.u12),
        (0x1D, 0x1E, 0x1F, 0x20, 0x21)
    );
    assert_eq!(
        (r.u13, r.u14, r.turbo_strength, r.steer_front),
        (0x22, 0x23, 0x24, 0x25)
    );
    assert_eq!(
        (r.wheels[0].dampen_len, r.wheels[0].ground_contact_material),
        (0x26, 0x27)
    );
    assert_eq!(
        (r.wheels[3].dampen_len, r.wheels[3].ground_contact_material),
        (0x2C, 0x2D)
    );
    assert_eq!((r.u25, r.u26, r.u27, r.dirt_blend), (0x2E, 0x2F, 0x30, 0x31));
    assert_eq!(g.race_time, Some(Some(12400)));
    assert_eq!(g.respawns, Some(None));
    assert_eq!(g.light_trail_color, Some([0.0, 0.5, 1.0]));
    assert_eq!(g.stunt_score, Some(Some(5)));
    let cps = g.checkpoints.as_ref().unwrap();
    assert_eq!((cps[0].time, cps[0].stunts_score), (Some(3000), Some(0)));
    assert_eq!((cps[1].time, cps[1].stunts_score), (Some(12400), None));
    assert_eq!(g.u0c, Some(0));
    assert_eq!(g.ghost_uid, Some(Id::Name("uid".into())));
    assert_eq!(g.ghost_login.as_deref(), Some("login"));
    assert_eq!(g.validate_challenge_uid, Some(Id::Name("uid".into())));
    assert_eq!(
        g.security_key,
        Some((7, [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]))
    );
    assert_eq!(g.u13, Some([0, 0]));
    assert_eq!(g.ghost_version, Some(Some(7)));
    assert_eq!(g.ghost_nickname, Some(Id::Name("$fffNick".into())));
    let m = g.metadata.as_ref().unwrap();
    assert_eq!(
        (
            m.skin_packs[0].file_path.as_str(),
            m.nickname.as_str(),
            m.avatar_name.as_str()
        ),
        ("Skins\\Car.zip", "$fffNick", "avatar")
    );
    let model = g.player_model.as_ref().unwrap();
    assert_eq!(model.id, Id::Name("StadiumCar".into()));
    assert_eq!(model.author, Id::Name("StadiumCar".into()));
    let v = g.validation.as_ref().unwrap();
    assert_eq!((v.events_duration, v.seed), (5000, Some(0)));
    let e = v.events.as_ref().unwrap();
    assert_eq!(e.names, [Id::Name("Accelerate".into()), Id::Name("Steer".into())]);
    let last = e.inputs[1];
    assert_eq!(
        (e.inputs.len(), last.time, last.name, last.data),
        (2, 100_020, 1, 0xFFFF)
    );
    assert_eq!(
        (e.exe_version.as_str(), e.exe_checksum, e.os_kind, e.cpu_kind),
        ("TmForever.2.11.26", 0xDEAD_BEEF, 10, 2)
    );
    assert_eq!(e.race_settings, "<id>Unassigned</id>");
}

#[test]
fn a_ghost_without_events_stores_only_the_duration() {
    let body = ghost_body(&good_stream(), &validation(0, 0));
    let nodes = read(GHOST, &body).unwrap();
    let g: &Ghost = nodes.main().try_into().unwrap();
    let v = g.validation.as_ref().unwrap();
    assert_eq!((v.events_duration, &v.events, v.seed), (0, &None, None));
}

#[test]
fn listed_sizes_and_times_when_the_step_is_not_fixed() {
    let mut s = Vec::new();
    words(&mut s, &[0x0A10_3000, 0, 0, 100, 9]);
    words(&mut s, &[122]);
    s.extend_from_slice(&[record(0), record(1)].concat());
    // two samples, first offset 0, sizes listed (-1, then the first's), then the times
    words(&mut s, &[2, 0, u32::MAX, 61, 2, 0, 90]);
    let nodes = read(GHOST, &ghost_body(&s, &validation(0, 0))).unwrap();
    let g: &Ghost = nodes.main().try_into().unwrap();
    let s = g.samples.as_ref().unwrap();
    assert_eq!(s.vehicle_class, 0x0A10_3000);
    assert_eq!(s.sizes, RecordSizes::Each(vec![61]));
    assert_eq!(s.times, Some(vec![0, 90]));
    assert_eq!(s.records.len(), 2);
}

#[test]
fn a_media_block_ghost_holds_its_ghost() {
    let mut b = Vec::new();
    words(&mut b, &[0x030E_5001]);
    for f in [1.0f32, 12.5] {
        b.extend_from_slice(&f.to_le_bytes());
    }
    words(&mut b, &[1, GHOST]);
    b.extend_from_slice(&ghost_body(&good_stream(), &validation(0, 0)));
    b.extend_from_slice(&0.25f32.to_le_bytes());
    words(&mut b, &[0xFACA_DE01]);
    let nodes = read(0x030E_5000, &b).unwrap();
    let m: &MediaBlockGhost = nodes.main().try_into().unwrap();
    assert_eq!(m.ghost, Some((1.0, 12.5, Some(NodeIndex(1)), 0.25)));
    assert!(matches!(nodes.get(NodeIndex(1)), Some(Node::Ghost(_))));
    for n in 0..b.len() {
        assert!(read(0x030E_5000, &b[..n]).is_err(), "prefix {n}");
    }
}

fn refused(stream: &[u8], validation: &[u8]) -> String {
    read(GHOST, &ghost_body(stream, validation))
        .unwrap_err()
        .to_string()
}

#[test]
fn refusals() {
    let body = ghost_body(&good_stream(), &validation(5000, 0));
    // every truncation of the node
    for n in 0..body.len() {
        assert!(read(GHOST, &body[..n]).is_err(), "prefix {n}");
    }
    // a byte more
    let mut more = body.clone();
    more.push(0);
    assert!(read(GHOST, &more).is_err());
    // a vehicle state version, a vehicle class and a record size not seen
    let records = [record(0), record(1), record(2)];
    let e = refused(&samples(0x0A02_B000, 8, &records, None), &validation(0, 0));
    assert!(e.contains("state version 8"), "{e}");
    let e = refused(&samples(0x0A40_1000, 9, &records, None), &validation(0, 0));
    assert!(e.contains("vehicle class 0x0a401000"), "{e}");
    let e = refused(
        &samples(0x0A02_B000, 9, &records, Some(&[61, 60])),
        &validation(0, 0),
    );
    assert!(e.contains("ghost sample"), "{e}");
    // a sample stream with a byte more
    let mut stream = good_stream();
    stream.push(0);
    let e = refused(&stream, &validation(0, 0));
    assert!(e.contains("bytes left"), "{e}");
    // the input store version
    let e = refused(&good_stream(), &validation(5000, 1));
    assert!(e.contains("input store version 1"), "{e}");
    // the TMCP chunk without the skip marker
    let mut unskippable = body.clone();
    unskippable[4..8].copy_from_slice(&0u32.to_le_bytes());
    assert!(read(GHOST, &unskippable).is_err());
    // a chunk that appears twice
    let mut twice = ghost_body(&good_stream(), &validation(0, 0));
    twice.truncate(twice.len() - 4);
    words(&mut twice, &[0x0309_200C, 0, 0xFACA_DE01]);
    let e = read(GHOST, &twice).unwrap_err().to_string();
    assert!(e.contains("a second time"), "{e}");
}
