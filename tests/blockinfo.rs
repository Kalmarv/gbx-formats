//! Stadium's game classes built by hand, byte by byte, from the layouts the crate documents: a block with its unit and
//! mobil, the other block infos, the scene objects of a mobil, a decoration with its parts, zones and a collection, and
//! the refusals of each class.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test building and reading hand-made bytes"
)]

use gbx_formats::game::blockinfo::{
    BlockInfoClassic, BlockInfoClip, BlockInfoFrontier, BlockInfoPylon, BlockInfoRoad, BlockUnitInfo,
};
use gbx_formats::game::collection::{Collection, ZoneFlat, ZoneFrontier};
use gbx_formats::game::decoration::{
    Decoration, DecorationAudio, DecorationMood, DecorationSize, DecorationTerrainModifier,
};
use gbx_formats::game::scene::{
    HmsItem, HmsSoundSource, LinkTarget, Mobil, MobilPtr, MotionCmdBase, MotionPlayer, MotionShader,
    ObjectLink, SceneSoundSource,
};
use gbx_formats::{Gbx, Id, NodeIndex, Nodes};

const FACADE: u32 = 0xFACA_DE01;
const NONE: u32 = u32::MAX;

/// A body under construction: words, strings and lookback ids (every name a new string, the scope's version first).
#[derive(Default)]
struct Body {
    bytes: Vec<u8>,
    ids: u32,
}

impl Body {
    fn u32s(&mut self, values: &[u32]) -> &mut Self {
        for v in values {
            self.bytes.extend_from_slice(&v.to_le_bytes());
        }
        self
    }

    fn f32s(&mut self, values: &[f32]) -> &mut Self {
        for v in values {
            self.bytes.extend_from_slice(&v.to_le_bytes());
        }
        self
    }

    fn string(&mut self, s: &str) -> &mut Self {
        self.u32s(&[u32::try_from(s.len()).unwrap()]);
        self.bytes.extend_from_slice(s.as_bytes());
        self
    }

    fn id(&mut self, name: &str) -> &mut Self {
        if self.ids == 0 {
            self.u32s(&[3]);
        }
        self.ids += 1;
        self.u32s(&[0x4000_0000]);
        self.string(name)
    }

    fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.bytes.extend_from_slice(bytes);
        self
    }

    fn skippable(&mut self, id: u32, data: &[u8]) -> &mut Self {
        self.u32s(&[id, 0x534B_4950, u32::try_from(data.len()).unwrap()]);
        self.raw(data)
    }
}

/// A file with `body` as its one node of class `class`, in a scene of `nodes` nodes.
fn file(class: u32, nodes: u32, body: &[u8]) -> Vec<u8> {
    let mut out = b"GBX\x06\x00BUUR".to_vec();
    for w in [class, 0, nodes, 0] {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out.extend_from_slice(body);
    out
}

fn read(class: u32, nodes: u32, body: &[u8]) -> gbx_formats::Result<Nodes> {
    Gbx::parse(&file(class, nodes, body)).unwrap().nodes()
}

fn get<'n, T>(nodes: &'n Nodes, index: u32) -> &'n T
where
    &'n T: TryFrom<&'n gbx_formats::Node>,
{
    match <&T>::try_from(nodes.get(NodeIndex(index)).unwrap()) {
        Ok(t) => t,
        Err(_) => panic!("node {index} is not the class asked for"),
    }
}

fn main_of<'n, T>(nodes: &'n Nodes) -> &'n T
where
    &'n T: TryFrom<&'n gbx_formats::Node>,
{
    get(nodes, 0)
}

fn name(s: &str) -> Id {
    Id::Name(s.into())
}

/// The refusal of `body` as a node of `class`, as text.
fn refused(class: u32, nodes: u32, body: &[u8]) -> String {
    read(class, nodes, body).unwrap_err().to_string()
}

/// Every truncation of the body is refused.
fn truncations_are_refused(class: u32, nodes: u32, body: &[u8]) {
    for n in 0..body.len() {
        assert!(read(class, nodes, &body[..n]).is_err(), "prefix {n}");
    }
}

/// A block (class `0x03051000`, node 0) with a unit (node 1) whose mobil (node 2) shows an item with no solid.
fn block_body() -> Vec<u8> {
    let mut b = Body::default();
    // the collector: an id
    b.u32s(&[0x0301_A00A]).id("StadiumRoad");
    // the block info: the way point, a spawn, the flags
    b.u32s(&[0x0304_E00A, 2, NONE, NONE]);
    let mut spawn = [0.0f32; 24];
    spawn[0] = 1.0;
    spawn[23] = 7.5;
    b.u32s(&[0x0304_E00C]).f32s(&spawn);
    b.u32s(&[0x0304_E00D, 0, 0x0304_E00F, 1]);
    // the body: name, size, the pillar, a unit, a mobil
    b.u32s(&[0x0304_E005]).id("RoadMain");
    b.u32s(&[1, 2, 3, 0, 1, 0, NONE]);
    // ground units: one, in place
    b.u32s(&[1, 1, 0x0303_6000, 0x0303_6000, 2, 1, 0, 4, 5, 6, 0, FACADE]);
    // air units: none
    b.u32s(&[0]);
    // ground mobils: one unit with one mobil, in place
    b.u32s(&[1, 1, 2, 0x0A01_1000]);
    b.u32s(&[0x0A00_5001]).id("Mobil1");
    b.u32s(&[0x0A01_1003, 0]);
    // the item, in place without a class: no solid, flags and the visible id
    b.u32s(&[0x0A01_1005, 0x0600_3001, NONE, 0x0600_3011]);
    b.raw(&0x0123_4567_89AB_CDEFu64.to_le_bytes())
        .raw(&0x0102u16.to_le_bytes());
    b.u32s(&[FACADE, FACADE]);
    // air mobils: none; then the bytes after the lists
    b.u32s(&[0]);
    b.raw(&[7])
        .u32s(&[9])
        .raw(&1i16.to_le_bytes())
        .raw(&2i16.to_le_bytes());
    b.u32s(&[FACADE]);
    b.bytes
}

const CLASSIC: u32 = 0x0305_1000;

#[test]
fn a_block_reads_with_its_unit_and_mobil() {
    let nodes = read(CLASSIC, 3, &block_body()).unwrap();
    let block: &BlockInfoClassic = main_of(&nodes);
    assert_eq!(block.collector.u0a, Some(name("StadiumRoad")));
    assert!(block.chunks.unread.is_empty());
    assert_eq!(
        block.chunks.ids,
        [
            0x0301_A00A,
            0x0304_E00A,
            0x0304_E00C,
            0x0304_E00D,
            0x0304_E00F,
            0x0304_E005
        ]
    );
    let way = block.info.waypoint.as_ref().unwrap();
    assert_eq!(
        (
            way.kind,
            way.ground_helper,
            way.air_helper,
            way.construction_helper
        ),
        (2, None, None, None)
    );
    let (spawn, _) = block.info.spawn.as_ref().unwrap();
    assert_eq!(
        (spawn.matrix[0], spawn.translation),
        ([1.0, 0.0, 0.0], [0.0, 0.0, 0.0])
    );
    assert_eq!(block.info.is_replacement, Some(false));
    assert_eq!(block.info.no_respawn, Some(true));
    let body = block.info.body.as_ref().unwrap();
    assert_eq!(body.name, name("RoadMain"));
    assert_eq!(
        (body.size, body.is_pillar, body.selection, body.u01, body.pillar),
        ([1, 2, 3], false, 1, 0, None)
    );
    assert_eq!(body.ground_units, [Some(NodeIndex(1))]);
    assert!(body.air_units.is_empty() && body.air_mobils.is_empty());
    assert_eq!(body.ground_mobils, [vec![Some(NodeIndex(2))]]);
    assert_eq!((body.u02, body.u03, body.u04), (7, 9, [1, 2]));

    let unit: &BlockUnitInfo = get(&nodes, 1);
    let place = unit.place.as_ref().unwrap();
    assert_eq!((place.place_pylons, place.u1, place.u2), (2, true, false));
    assert_eq!((place.relative_offset, place.clips.len()), ([4, 5, 6], 0));

    let mobil: &Mobil = get(&nodes, 2);
    assert_eq!(mobil.object.name, Some(name("Mobil1")));
    assert_eq!(mobil.links, Some(vec![]));
    let item: &HmsItem = mobil.item.as_ref().unwrap();
    assert_eq!(item.solid, Some(None));
    let flags = item.flags.unwrap();
    assert_eq!(
        (flags.chunk, flags.flags, flags.visible_id),
        (0x0600_3011, 0x0123_4567_89AB_CDEF, 0x0102)
    );
}

#[test]
fn a_block_is_refused_when_it_is_not_read_in_full() {
    let body = block_body();
    truncations_are_refused(CLASSIC, 3, &body);
    // a chunk this crate does not read (the units chunk of the later games)
    let mut b = Body::default();
    b.u32s(&[0x0304_E01D, 0, FACADE]);
    let e = refused(CLASSIC, 1, &b.bytes);
    assert!(
        e.contains("0x0304e01d") && e.contains("CGameCtnBlockInfoClassic"),
        "{e}"
    );
    // a chunk twice
    let mut b = Body::default();
    b.u32s(&[0x0304_E00D, 0, 0x0304_E00D, 0, FACADE]);
    assert!(refused(CLASSIC, 1, &b.bytes).contains("a second time"));
    // a unit's chunk it does not read
    let mut b = Body::default();
    b.u32s(&[0x0303_6007, FACADE]);
    assert!(refused(0x0303_6000, 1, &b.bytes).contains("0x03036007"));
}

#[test]
fn the_other_block_infos_read() {
    let mut b = Body::default();
    b.u32s(&[0x0305_2000, NONE, FACADE]);
    let nodes = read(0x0305_2000, 1, &b.bytes).unwrap();
    assert_eq!(main_of::<BlockInfoRoad>(&nodes).slope, Some(None));

    let mut b = Body::default();
    b.skippable(0x0305_3000, &NONE.to_le_bytes());
    b.u32s(&[0x0305_3002]).id("ClipB");
    b.u32s(&[0x0305_3003, 1, FACADE]);
    let nodes = read(0x0305_3000, 1, &b.bytes).unwrap();
    let clip: &BlockInfoClip = main_of(&nodes);
    assert_eq!(clip.legacy_symmetrical_clip, Some(None));
    assert_eq!(clip.symmetrical_clip_id, Some(name("ClipB")));
    assert_eq!(clip.is_free_clip, Some(true));
    // a clip's chunk of the later games
    let mut b = Body::default();
    b.u32s(&[0x0305_3005, 0, FACADE]);
    assert!(refused(0x0305_3000, 1, &b.bytes).contains("0x03053005"));

    let mut b = Body::default();
    b.u32s(&[FACADE]);
    let nodes = read(0x0305_0000, 1, &b.bytes).unwrap();
    assert!(main_of::<BlockInfoFrontier>(&nodes).chunks.ids.is_empty());
    let mut b = Body::default();
    b.u32s(&[0x0305_0000, 1, FACADE]);
    assert!(refused(0x0305_0000, 1, &b.bytes).contains("0x03050000"));

    let mut b = Body::default();
    b.u32s(&[0x0305_5000, NONE, NONE, NONE, FACADE]);
    let nodes = read(0x0305_5000, 1, &b.bytes).unwrap();
    assert_eq!(main_of::<BlockInfoPylon>(&nodes).nodes, Some([None, None, None]));
    let mut b = Body::default();
    b.u32s(&[0x0305_5002, 0, 0, 0, 0, FACADE]);
    assert!(refused(0x0305_5000, 1, &b.bytes).contains("0x03055002"));
}

/// A link (class `0x0A014000`, node 0) to a model instance whose model (node 1) is a mobil with one link to nothing.
fn link_body(data_version: u32, terminator: u32) -> Vec<u8> {
    let mut b = Body::default();
    b.u32s(&[0x0A01_4001, 1, 1, 0x0A01_1000, 0x0A01_1003, 0, FACADE]);
    b.u32s(&[data_version, terminator]).id("Instance");
    b.u32s(&[1, NONE]);
    let mut matrix = [0.0f32; 12];
    matrix[0] = 1.0;
    matrix[11] = 2.0;
    b.f32s(&matrix).u32s(&[1]);
    b.u32s(&[0x0A01_4002]).id("Tree");
    b.u32s(&[1, 0, 0x0A01_4003]).id("Other");
    b.u32s(&[FACADE]);
    b.bytes
}

#[test]
fn a_link_reads_a_model_instance() {
    let nodes = read(0x0A01_4000, 2, &link_body(2, NONE)).unwrap();
    let link: &ObjectLink = main_of(&nodes);
    let (target, location, u06) = link.target.as_ref().unwrap();
    assert_eq!(
        *target,
        LinkTarget::Mobil(MobilPtr::Model {
            model: NodeIndex(1),
            name: name("Instance"),
            links: vec![None]
        })
    );
    assert_eq!(
        (location.matrix[0], location.translation[2], *u06),
        ([1.0, 0.0, 0.0], 2.0, true)
    );
    assert_eq!(link.mobil_tree, Some((name("Tree"), true, false)));
    assert_eq!(link.u03, Some(name("Other")));
    assert!(matches!(
        nodes.get(NodeIndex(1)),
        Some(gbx_formats::Node::Mobil(_))
    ));
}

#[test]
fn a_link_is_refused_when_it_is_not_read_in_full() {
    truncations_are_refused(0x0A01_4000, 2, &link_body(2, NONE));
    // the data versions whose layout depends on the model's class
    for v in [0, 1, 3] {
        let e = refused(0x0A01_4000, 2, &link_body(v, NONE));
        assert!(e.contains("mobil pointer"), "{e}");
    }
    let e = refused(0x0A01_4000, 2, &link_body(2, 5));
    assert!(e.contains("expected -1"), "{e}");
}

#[test]
fn a_sound_source_and_a_motion_read() {
    // a sound source (node 0) with its source in place
    let mut b = Body::default();
    b.u32s(&[0x0A00_9000, 1, 0x0A00_E000]);
    b.u32s(&[0x0600_D003]).f32s(&[0.5, 1.0, 2.0]);
    b.u32s(&[0x0600_D004]).f32s(&[3.0, 4.0, 5.0]);
    b.u32s(&[0x0600_D005, NONE])
        .f32s(&[0.25])
        .u32s(&[1, FACADE, FACADE]);
    let nodes = read(0x0A00_E000, 1, &b.bytes).unwrap();
    let sound: &SceneSoundSource = main_of(&nodes);
    assert_eq!(sound.is_active, Some(true));
    let source: &HmsSoundSource = sound.source.as_ref().unwrap();
    assert_eq!(source.volume_pitch_speed, Some([0.5, 1.0, 2.0]));
    assert_eq!(source.volumic_size, Some([3.0, 4.0, 5.0]));
    assert_eq!(source.sound, Some((None, 0.25, true)));
    // chunk 003 and 004 together, and a chunk of the earlier games
    let mut b = Body::default();
    b.u32s(&[0x0A00_E000, 0x0600_D003]).f32s(&[0.5, 1.0, 2.0]);
    b.u32s(&[0x0600_D001, 0, 0, FACADE, FACADE]);
    let e = refused(0x0A00_E000, 1, &b.bytes);
    assert!(e.contains("0x0600d001") && e.contains("CHmsSoundSource"), "{e}");

    // a motion player (node 0): the command in place, a state saved as "save the current state", one track (node 1)
    let mut b = Body::default();
    b.u32s(&[0x0800_1000]).id("Spin");
    b.u32s(&[0x0803_4004, 0x0802_9002]);
    b.u32s(&[1000]).f32s(&[0.5]).u32s(&[3, 1, 1, NONE, FACADE]);
    b.u32s(&[3, 2]).u32s(&[0]).id("Spin");
    b.u32s(&[1, 1, 0x0802_B000, 0x0802_B000, NONE, NONE, NONE, FACADE, FACADE]);
    let nodes = read(0x0803_4000, 2, &b.bytes).unwrap();
    let player: &MotionPlayer = main_of(&nodes);
    assert_eq!(player.motion_name, Some(name("Spin")));
    let cmd: &MotionCmdBase = player.command.as_ref().unwrap();
    assert_eq!(cmd.wave, Some((1000, 0.5, 3, true)));
    assert_eq!((cmd.is_absolute_phase, cmd.params), (Some(true), Some(None)));
    assert_eq!((player.play_state, player.is_physics), (Some(2), Some(false)));
    assert_eq!(player.name, Some(name("Spin")));
    assert_eq!(player.tracks, Some(vec![Some(NodeIndex(1))]));
    let shader: &MotionShader = get(&nodes, 1);
    assert_eq!(shader.shaders, Some((None, None, None)));
    // the earlier player chunk
    let mut b = Body::default();
    b.u32s(&[0x0803_4003, FACADE]);
    assert!(refused(0x0803_4000, 1, &b.bytes).contains("0x08034003"));
}

/// A decoration (node 0) with its size (1), audio (2), mood (3) and terrain modifiers (4, 5).
fn decoration_body() -> Vec<u8> {
    let mut b = Body::default();
    b.u32s(&[0x0301_A006, 0]);
    b.u32s(&[0x0303_8011, 1, 0x0303_B000, 0x0303_B000]);
    b.f32s(&[-1.0, -2.0, 3.0, 4.0, 0.5]);
    b.u32s(&[0x0303_B001, 8, 32, 8, 32, 0xFFFF_FFFF, FACADE]);
    // the audio: eleven sounds (the first from a file), three musics, two environments, an attenuation
    b.u32s(&[0x0303_8012, 2, 0x0303_9000, 0x0303_9000, 1])
        .id("Ambient");
    b.u32s(&[1]);
    b.u32s(&[0]);
    b.u32s(&[0x0303_9001, NONE, NONE, 0x0303_9002]).f32s(&[1.0]);
    b.u32s(&[FACADE]);
    b.u32s(&[0x0303_8013, 3, 0x0303_A000, 0x0303_A000]);
    b.f32s(&[48.0, 2.0, 1.0]).u32s(&[21_600_000, 64_800_000]);
    b.u32s(&[0x0303_A001]).f32s(&[0.25]).u32s(&[NONE]).string("Remap");
    b.u32s(&[0x0303_A002, 4, 5]).f32s(&[0.75]).u32s(&[1, 0]);
    b.u32s(&[0x0303_A004, NONE, 0x0303_A005, NONE, 0x0303_A006])
        .f32s(&[2.5]);
    b.u32s(&[FACADE]);
    b.u32s(&[0x0303_8015, 4, 0x0303_C000, 0x0303_C000, NONE])
        .string("Dirt");
    b.u32s(&[0x0303_C001]).id("Dirt");
    b.u32s(&[FACADE, 0x0303_8016, NONE, FACADE]);
    b.bytes
}

#[test]
fn a_decoration_reads_with_its_parts() {
    let nodes = read(0x0303_8000, 5, &decoration_body()).unwrap();
    let deco: &Decoration = main_of(&nodes);
    assert_eq!(deco.collector.skin_kind, Some(0));
    assert_eq!(deco.size, Some(Some(NodeIndex(1))));
    assert_eq!(deco.audio, Some(Some(NodeIndex(2))));
    assert_eq!(deco.mood, Some(Some(NodeIndex(3))));
    assert_eq!(deco.terrain_modifier_covered, Some(Some(NodeIndex(4))));
    assert_eq!(deco.terrain_modifier_base, Some(None));
    assert_eq!(deco.decorator_solid_warp, None);

    let size: &DecorationSize = get(&nodes, 1);
    assert_eq!(size.edition_zone, Some(([-1.0, -2.0], [3.0, 4.0], 0.5)));
    assert_eq!(size.base, Some((8, [32, 8, 32], None)));

    let audio: &DecorationAudio = get(&nodes, 2);
    assert_eq!(audio.sounds, Some(vec![(name("Ambient"), Some(NodeIndex(1)))]));
    assert_eq!(audio.musics, Some(vec![]));
    assert_eq!(audio.environments, Some((None, None)));
    assert_eq!(audio.sound_attenuation_in_editor, Some(1.0));

    let mood: &DecorationMood = get(&nodes, 3);
    let sun = mood.sun.unwrap();
    assert_eq!((sun.latitude, sun.longitude, sun.delta_gmt), (48.0, 2.0, 1.0));
    assert_eq!((sun.time_sun_rise, sun.time_sun_fall), (21_600_000, 64_800_000));
    assert_eq!(mood.remapping, Some((0.25, None, "Remap".into())));
    let shadows = mood.shadows.unwrap();
    assert_eq!(
        (
            shadows.count_car_human,
            shadows.count_car_opponent,
            shadows.car_intensity
        ),
        (4, 5, 0.75)
    );
    assert_eq!(
        (shadows.scene, shadows.background_is_locally_lighted),
        (true, false)
    );
    assert_eq!(
        (mood.light_map, mood.ambient_occlusion, mood.sun_moon_intensity),
        (Some(None), Some(None), Some(2.5))
    );

    let modifier: &DecorationTerrainModifier = get(&nodes, 4);
    assert_eq!(modifier.remapping, Some((None, "Dirt".into())));
    assert_eq!(modifier.name, Some(name("Dirt")));
}

#[test]
fn a_decoration_is_refused_when_it_is_not_read_in_full() {
    truncations_are_refused(0x0303_8000, 5, &decoration_body());
    // a decoration's chunk of the later games
    let mut b = Body::default();
    b.u32s(&[0x0303_8017, 1, NONE, FACADE]);
    assert!(refused(0x0303_8000, 1, &b.bytes).contains("0x03038017"));
    // a size's, a mood's, a modifier's and an audio's
    for (class, chunk) in [
        (0x0303_B000, 0x0303_B003),
        (0x0303_A000, 0x0303_A012),
        (0x0303_C000, 0x0303_C002),
        (0x0303_9000, 0x0303_9005),
    ] {
        let mut b = Body::default();
        b.u32s(&[chunk, 0, FACADE]);
        let e = refused(class, 1, &b.bytes);
        assert!(e.contains(&format!("{chunk:#010x}")), "{e}");
    }
}

/// A collection (node 0) with its zones (nodes 1 to 3: a flat zone, a frontier zone and the default zone).
fn collection_body(zone_list_version: u32) -> Vec<u8> {
    let mut b = Body::default();
    b.u32s(&[0x0303_3009]).id("Stadium");
    b.u32s(&[zone_list_version, 2]);
    // the flat zone
    b.u32s(&[1, 0x0305_D000, 0x0305_D001, NONE, NONE, NONE, NONE]);
    b.u32s(&[0x0305_C003, 2]).id("Grass").id("Grass").u32s(&[0]);
    b.skippable(0x0305_C004, &[1, 0, 0, 0, 0, 0, 0, 0]);
    b.u32s(&[0x0305_C005, 1]);
    b.skippable(0x0305_D002, &[3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
    b.u32s(&[FACADE]);
    // the frontier zone
    b.u32s(&[2, 0x0305_E000, 0x0305_E001, NONE])
        .id("Dirt")
        .id("Grass");
    b.u32s(&[FACADE]);
    // the default zone, whether the collection needs unlocking, the square and the vehicle
    b.u32s(&[1, 1]).f32s(&[32.0, 8.0]);
    b.id("StadiumCar").id("Vehicles").id("Nadeo");
    b.u32s(&[0x0303_300C, 1, 0, 0x0303_300E, 5, 0x0303_3021])
        .string("Stadium");
    b.u32s(&[FACADE]);
    b.bytes
}

#[test]
fn a_collection_reads_with_its_zones() {
    let nodes = read(0x0303_3000, 3, &collection_body(10)).unwrap();
    let collection: &Collection = main_of(&nodes);
    let zones = collection.zones.as_ref().unwrap();
    assert_eq!(zones.collection, name("Stadium"));
    assert_eq!(zones.complete_zone_list, [Some(NodeIndex(1)), Some(NodeIndex(2))]);
    assert_eq!(zones.default_zone, Some(NodeIndex(1)));
    assert_eq!(
        (zones.need_unlock, zones.square_size, zones.square_height),
        (true, 32.0, 8.0)
    );
    assert_eq!(zones.vehicle.id, name("StadiumCar"));
    assert_eq!(
        (zones.vehicle.collection.clone(), zones.vehicle.author.clone()),
        (name("Vehicles"), name("Nadeo"))
    );
    assert_eq!(collection.editable, Some((true, 0)));
    assert_eq!(collection.sort_index, Some(5));
    assert_eq!(collection.display_name.as_deref(), Some("Stadium"));

    let flat: &ZoneFlat = get(&nodes, 1);
    assert_eq!(flat.zone.zone, Some((2, name("Grass"), name("Grass"), 0)));
    assert_eq!(flat.zone.depth, Some((1, false)));
    assert_eq!(flat.zone.has_water, Some(true));
    assert_eq!(flat.block_infos, Some([None, None, None, None]));
    assert_eq!(flat.ground_only, Some((3, true, false)));

    let frontier: &ZoneFrontier = get(&nodes, 2);
    assert_eq!(frontier.frontier, Some((None, name("Dirt"), name("Grass"))));
}

#[test]
fn a_collection_is_refused_when_it_is_not_read_in_full() {
    truncations_are_refused(0x0303_3000, 3, &collection_body(10));
    // a deprecated list written by another version of the game
    let e = refused(0x0303_3000, 3, &collection_body(9));
    assert!(e.contains("deprecated node list"), "{e}");
    // chunks of the later games, in the collection and in a zone
    let mut b = Body::default();
    b.u32s(&[0x0303_3027, 0, FACADE]);
    assert!(refused(0x0303_3000, 1, &b.bytes).contains("0x03033027"));
    let mut b = Body::default();
    b.u32s(&[0x0305_C006, 0, FACADE]);
    assert!(refused(0x0305_D000, 1, &b.bytes).contains("0x0305c006"));
}
