//! A Gbx file built by hand, byte by byte, from the layout the crate documents: the container, the id lookback, a
//! node reference read inline and then by index, a skippable chunk kept as stored, and the refusals.
use gbx_formats::game::block::BlockSkin;
use gbx_formats::game::challenge::Challenge;
use gbx_formats::{Body, Error, Gbx, Id, NodeIndex};

fn u32s(out: &mut Vec<u8>, values: &[u32]) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}

fn string(out: &mut Vec<u8>, s: &str) {
    u32s(out, &[s.len() as u32]);
    out.extend_from_slice(s.as_bytes());
}

/// A map body: the vehicle ident (three ids: a new string, an empty one, the first string again), a skippable chunk
/// this crate does not read, the global clip as a new node (a block skin, index 1) and the "simple editor" flag.
fn body() -> Vec<u8> {
    let mut b = Vec::new();
    u32s(&mut b, &[0x0304_300D, 3, 0x4000_0000]);
    string(&mut b, "StadiumCar");
    u32s(&mut b, &[u32::MAX, 0x4000_0001]);
    u32s(&mut b, &[0x0304_30F0, 0x534B_4950, 2]);
    b.extend_from_slice(&[0xAB, 0xCD]);
    u32s(&mut b, &[0x0304_3026, 1, 0x0305_9000, 0x0305_9002]);
    string(&mut b, "sign");
    b.push(3);
    b.extend_from_slice(&[0; 32]);
    string(&mut b, "Skins\\Sign.zip");
    string(&mut b, "");
    b.push(3);
    b.extend_from_slice(&[0; 32]);
    string(&mut b, "");
    string(&mut b, "");
    u32s(&mut b, &[0xFACA_DE01]);
    u32s(&mut b, &[0x0304_302A, 1, 0xFACA_DE01]);
    b
}

/// The container around an uncompressed body: one header chunk (the map format version, 6) and two nodes.
fn file(body: &[u8]) -> Vec<u8> {
    let mut f = b"GBX".to_vec();
    f.extend_from_slice(&6u16.to_le_bytes());
    f.extend_from_slice(b"BUUR");
    u32s(&mut f, &[0x0304_3000, 16, 1, 0x0304_3004, 4, 6, 2, 0]);
    f.extend_from_slice(body);
    f
}

#[test]
fn a_hand_built_map_reads() {
    let bytes = file(&body());
    let gbx = Gbx::parse(&bytes).unwrap();
    assert_eq!(
        (gbx.version, gbx.release, gbx.class_id, gbx.nodes),
        (6, true, 0x0304_3000, 2)
    );
    assert_eq!(gbx.header_chunks.len(), 1);
    assert_eq!(
        (gbx.header_chunks[0].id, gbx.header_chunks[0].bytes),
        (0x0304_3004, &6u32.to_le_bytes()[..])
    );
    assert!(matches!(gbx.body, Body::Uncompressed { offset: 41, .. }));

    let nodes = gbx.nodes().unwrap();
    let map: &Challenge = nodes.main().try_into().unwrap();
    let car = map.player_model.as_ref().unwrap();
    assert_eq!(car.id, Id::Name("StadiumCar".into()));
    assert_eq!(car.collection, Id::Empty);
    assert_eq!(car.author, Id::Name("StadiumCar".into()));
    assert_eq!(
        map.chunks.ids,
        [0x0304_300D, 0x0304_30F0, 0x0304_3026, 0x0304_302A]
    );
    assert_eq!(map.chunks.unread.len(), 1);
    assert_eq!(
        (map.chunks.unread[0].id, &map.chunks.unread[0].bytes[..]),
        (0x0304_30F0, &[0xAB, 0xCD][..])
    );
    assert_eq!(map.clip_global, Some(Some(NodeIndex(1))));
    assert_eq!(map.created_with_simple_editor, Some(true));
    let skin: &BlockSkin = nodes.get(NodeIndex(1)).unwrap().try_into().unwrap();
    let (text, pack, _) = skin.skin.as_ref().unwrap();
    assert_eq!(
        (text.as_str(), pack.file_path.as_str()),
        ("sign", "Skins\\Sign.zip")
    );
}

#[test]
fn refusals() {
    let good = body();
    // every truncation of the file
    let whole = file(&good);
    for n in 0..whole.len() {
        assert!(
            Gbx::parse(&whole[..n]).and_then(|g| g.nodes()).is_err(),
            "prefix {n}"
        );
    }
    // a byte after the main node
    let mut trailing = good.clone();
    trailing.push(0);
    let e = Gbx::parse(&file(&trailing))
        .unwrap()
        .nodes()
        .unwrap_err()
        .to_string();
    assert!(e.contains("1 bytes left"), "{e}");
    // an unknown chunk without the skip marker
    let mut unskippable = good.clone();
    unskippable[38..42].copy_from_slice(&0u32.to_le_bytes());
    let e = Gbx::parse(&file(&unskippable)).unwrap().nodes().unwrap_err();
    assert!(
        matches!(
            e,
            Error::UnknownChunk {
                chunk: 0x0304_30F0,
                offset: 34,
                ..
            }
        ),
        "{e}"
    );
    // a node of a class this crate does not read
    let mut class = good.clone();
    class[56..60].copy_from_slice(&0x0999_9000u32.to_le_bytes());
    let e = Gbx::parse(&file(&class)).unwrap().nodes().unwrap_err();
    assert!(
        matches!(
            e,
            Error::UnknownClass {
                class: 0x0999_9000,
                ..
            }
        ),
        "{e}"
    );
    // an id referring back to a string never added
    let mut back = good;
    back[30..34].copy_from_slice(&0x4000_0002u32.to_le_bytes());
    assert!(Gbx::parse(&file(&back)).unwrap().nodes().is_err());
    // a header counting 2^32 - 1 nodes reads the same nodes (and allocates nothing for the ones never read); a
    // reference to an index at or past the count is refused
    let mut many = whole.clone();
    many[33..37].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(Gbx::parse(&many).unwrap().nodes().unwrap().iter().count(), 2);
    let mut past = whole.clone();
    past[33..37].copy_from_slice(&1u32.to_le_bytes());
    let e = Gbx::parse(&past).unwrap().nodes().unwrap_err().to_string();
    assert!(e.contains("node index 1: the header counts 1 nodes"), "{e}");
    // a text-format file
    let mut text = whole.clone();
    text[5] = b'T';
    assert!(Gbx::parse(&text).unwrap_err().to_string().contains("text-format"));
}

/// Seeded random bytes after a valid header, and the hand-built file with each byte flipped, give a result or an
/// error, never a panic.
#[test]
fn arbitrary_bytes_never_panic() {
    let whole = file(&body());
    for i in 0..whole.len() {
        let mut flipped = whole.clone();
        flipped[i] ^= 0x5A;
        let _ = Gbx::parse(&flipped).and_then(|g| g.nodes());
    }
    let mut s = 0x2545_F491u32;
    let mut next = || {
        s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (s >> 16) as u8
    };
    for len in 0..96 {
        for _ in 0..100 {
            let mut f = whole[..41].to_vec();
            f.extend((0..len).map(|_| next()));
            let _ = Gbx::parse(&f).and_then(|g| g.nodes());
        }
    }
}
