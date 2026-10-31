//! Hand-built MediaTracker nodes, independently assembled from GBX.NET's documented layouts.
#![allow(clippy::unwrap_used, reason = "hand-built test fixtures")]
use gbx_formats::game::media::*;
use gbx_formats::{Gbx, Node, Nodes};

fn words(out: &mut Vec<u8>, values: &[u32]) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}
fn floats(out: &mut Vec<u8>, values: &[f32]) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}
fn string(out: &mut Vec<u8>, text: &str) {
    words(out, &[u32::try_from(text.len()).unwrap()]);
    out.extend_from_slice(text.as_bytes());
}
fn pack(out: &mut Vec<u8>) {
    out.push(1);
    string(out, "image.zip");
    string(out, "https://example.org/image.zip");
}
fn file(class: u32, body: &[u8]) -> Vec<u8> {
    let mut out = b"GBX\x06\x00BUUR".to_vec();
    words(&mut out, &[class, 0, 16, 0]);
    out.extend_from_slice(body);
    out
}
fn read(class: u32, body: &[u8]) -> Nodes {
    Gbx::parse(&file(class, body)).unwrap().nodes().unwrap()
}
fn check(class: u32, body: &[u8], chunks: &[u32], verify: impl FnOnce(&Node)) {
    let nodes = read(class, body);
    assert_eq!(nodes.main().chunks().ids, chunks);
    assert!(nodes.main().chunks().unread.is_empty());
    verify(nodes.main());
    // Every truncation must refuse, including missing fields and the incomplete end marker.
    for n in 0..body.len() {
        assert!(
            Gbx::parse(&file(class, &body[..n])).unwrap().nodes().is_err(),
            "class {class:#x}, prefix {n}"
        );
    }
    let mut unknown = Vec::new();
    words(&mut unknown, &[class | 0xFFF, 0, 0xFACA_DE01]);
    assert!(Gbx::parse(&file(class, &unknown)).unwrap().nodes().is_err());
}

#[test]
fn clip_group() {
    let mut b = Vec::new();
    words(&mut b, &[0x0307A003, 10, 1, u32::MAX, 1, 1, 2, 3, 2, 5]);
    floats(&mut b, &[42.0]);
    words(&mut b, &[1, 4, 5, 6, 0xFACA_DE01]);
    check(0x0307A000, &b, &[0x0307A003], |node| {
        let n: &MediaClipGroup = node.try_into().unwrap();
        assert_eq!(n.clips.as_ref().unwrap(), &[None]);
        let t = &n.triggers.as_ref().unwrap()[0];
        assert_eq!(t.ref_coord, [1, 2, 3]);
        assert_eq!(t.condition_value, 42.0);
        assert_eq!(t.coords, [[4, 5, 6]]);
    });
}
#[test]
fn clip() {
    let mut b = Vec::new();
    words(&mut b, &[0x03079004, u32::MAX, 0x03079005, 10, 1, u32::MAX]);
    string(&mut b, "Intro");
    words(&mut b, &[0x03079007, 7, 0xFACA_DE01]);
    check(0x03079000, &b, &[0x03079004, 0x03079005, 0x03079007], |node| {
        let n: &MediaClip = node.try_into().unwrap();
        assert_eq!(n.name.as_deref(), Some("Intro"));
        assert_eq!(n.tracks.as_ref().unwrap(), &[None]);
        assert_eq!(n.scene, Some(None));
        assert_eq!(n.local_player_clip_ent_index, Some(7));
    });
}
#[test]
fn track() {
    let mut b = Vec::new();
    words(&mut b, &[0x03078001]);
    string(&mut b, "Camera");
    words(
        &mut b,
        &[10, 1, u32::MAX, u32::MAX, 0x03078004, 1, 0, 0xFACA_DE01],
    );
    check(0x03078000, &b, &[0x03078001, 0x03078004], |node| {
        let n: &MediaTrack = node.try_into().unwrap();
        assert_eq!(n.name.as_deref(), Some("Camera"));
        assert_eq!(n.blocks.as_ref().unwrap(), &[None]);
        assert_eq!(n.u01, Some(-1));
        assert_eq!(n.is_keep_playing, Some(true));
        assert_eq!(n.is_read_only, Some(false));
    });
}
#[test]
fn text() {
    let mut b = Vec::new();
    words(&mut b, &[0x030A8001]);
    string(&mut b, "Hello");
    words(&mut b, &[u32::MAX, 0x030A8002]);
    floats(&mut b, &[1.0, 0.5, 0.25]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x030A8000, &b, &[0x030A8001, 0x030A8002], |node| {
        let n: &MediaBlockText = node.try_into().unwrap();
        assert_eq!(n.text.as_deref(), Some("Hello"));
        assert_eq!(n.effect, Some(None));
        assert_eq!(n.color, Some([1.0, 0.5, 0.25]));
    });
}
#[test]
fn camera_custom() {
    let mut b = Vec::new();
    words(&mut b, &[0x030A2005, 1]);
    floats(&mut b, &[2.0]);
    words(&mut b, &[2, 11, 12]);
    floats(&mut b, &[1.0, 2.0, 3.0, 0.1, 0.2, 0.3, 90.0]);
    words(&mut b, &[1, 5, 0, u32::MAX]);
    floats(&mut b, &[4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x030A2000, &b, &[0x030A2005], |node| {
        let n: &MediaBlockCameraCustom = node.try_into().unwrap();
        let k = &n.keys.as_ref().unwrap()[0];
        assert_eq!(k.u01, 11);
        assert_eq!(k.position, [1.0, 2.0, 3.0]);
        assert_eq!(k.target, -1);
        assert_eq!(k.right_tangent, [10.0, 11.0, 12.0]);
    });
}
#[test]
fn transition_fade() {
    let mut b = Vec::new();
    words(&mut b, &[0x030AB000, 1]);
    floats(&mut b, &[2.0, 0.5, 0.1, 0.2, 0.3, 0.4]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x030AB000, &b, &[0x030AB000], |node| {
        let n: &MediaBlockTransitionFade = node.try_into().unwrap();
        assert_eq!(n.keys.as_ref().unwrap()[0].opacity, 0.5);
        assert_eq!(n.color, Some([0.1, 0.2, 0.3, 0.4]));
    });
}
#[test]
fn camera_game() {
    let mut b = Vec::new();
    words(&mut b, &[0x03084003]);
    floats(&mut b, &[1.0, 2.0]);
    words(&mut b, &[3, 0x40000000]);
    string(&mut b, "Internal");
    words(&mut b, &[5, 0xFACA_DE01]);
    check(0x03084000, &b, &[0x03084003], |node| {
        let n: &MediaBlockCameraGame = node.try_into().unwrap();
        assert_eq!(n.start, Some(1.0));
        assert_eq!(n.game_cam_id, Some(gbx_formats::Id::Name("Internal".into())));
        assert_eq!(n.clip_ent_id, Some(5));
    });
}
fn triangle_body() -> Vec<u8> {
    let mut b = Vec::new();
    words(&mut b, &[0x03029001, 1]);
    floats(&mut b, &[2.0]);
    words(&mut b, &[1, 3]);
    floats(&mut b, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    words(&mut b, &[3]);
    floats(
        &mut b,
        &[1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0],
    );
    words(&mut b, &[1, 0, 1, 2, 1]);
    floats(&mut b, &[0.0, 0.0, 1.0, 4.0, 5.0, 6.0]);
    words(&mut b, &[0xFACA_DE01]);
    b
}
fn triangle(g: &TriangleGeometry) {
    assert_eq!(g.times, [2.0]);
    assert_eq!(g.positions[0][2], [7.0, 8.0, 9.0]);
    assert_eq!(g.vertices.len(), 3);
    assert_eq!(g.triangles, [[0, 1, 2]]);
    assert_eq!(g.pick_plane_position, [4.0, 5.0, 6.0]);
}
#[test]
fn triangles() {
    check(0x03029000, &triangle_body(), &[0x03029001], |node| {
        let n: &MediaBlockTriangles = node.try_into().unwrap();
        triangle(n.geometry.as_ref().unwrap());
    });
}
#[test]
fn triangles_2d() {
    check(0x0304B000, &triangle_body(), &[0x03029001], |node| {
        let n: &MediaBlockTriangles2D = node.try_into().unwrap();
        triangle(n.geometry.as_ref().unwrap());
    });
}
#[test]
fn triangles_3d() {
    check(0x0304C000, &triangle_body(), &[0x03029001], |node| {
        let n: &MediaBlockTriangles3D = node.try_into().unwrap();
        triangle(n.geometry.as_ref().unwrap());
    });
}
#[test]
fn image() {
    let mut b = Vec::new();
    words(&mut b, &[0x030A5000, u32::MAX]);
    pack(&mut b);
    words(&mut b, &[0xFACA_DE01]);
    check(0x030A5000, &b, &[0x030A5000], |node| {
        let n: &MediaBlockImage = node.try_into().unwrap();
        assert_eq!(n.effect, Some(None));
        assert_eq!(n.image.as_ref().unwrap().file_path, "image.zip");
    });
}
#[test]
fn sound() {
    let mut b = Vec::new();
    words(&mut b, &[0x030A7003, 0, 3, 1, 0, 0x030A7004]);
    pack(&mut b);
    words(&mut b, &[1, 1]);
    floats(&mut b, &[2.0, 0.75, 0.5, 1.0, 2.0, 3.0]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x030A7000, &b, &[0x030A7003, 0x030A7004], |node| {
        let n: &MediaBlockSound = node.try_into().unwrap();
        assert_eq!(n.settings.as_ref().unwrap().play_count, 3);
        assert_eq!(n.keys.as_ref().unwrap()[0].position, [1.0, 2.0, 3.0]);
    });
}
#[test]
fn fx_colors() {
    let mut b = Vec::new();
    words(&mut b, &[0x03080003, 1]);
    floats(
        &mut b,
        &[
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0,
            18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0,
        ],
    );
    words(&mut b, &[0xFACA_DE01]);
    check(0x03080000, &b, &[0x03080003], |node| {
        let n: &MediaBlockFxColors = node.try_into().unwrap();
        let k = &n.keys.as_ref().unwrap()[0];
        assert_eq!(k.distance, 4.0);
        assert_eq!(k.blend_rgb, [14.0, 15.0, 16.0]);
        assert_eq!(k.far_blend_alpha, 29.0);
    });
}
#[test]
fn fx_blur_depth() {
    let mut b = Vec::new();
    words(&mut b, &[0x03081001, 1]);
    floats(&mut b, &[1.0, 2.0]);
    words(&mut b, &[1]);
    floats(&mut b, &[3.0]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x03081000, &b, &[0x03081001], |node| {
        let n: &MediaBlockFxBlurDepth = node.try_into().unwrap();
        assert!(n.keys.as_ref().unwrap()[0].force_focus);
        assert_eq!(n.keys.as_ref().unwrap()[0].focus_z, 3.0);
    });
}
#[test]
fn fx_bloom() {
    let mut b = Vec::new();
    words(&mut b, &[0x03083001, 1]);
    floats(&mut b, &[1.0, 2.0, 3.0]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x03083000, &b, &[0x03083001], |node| {
        let n: &MediaBlockFxBloom = node.try_into().unwrap();
        assert_eq!(n.keys.as_ref().unwrap()[0].sensitivity, 3.0);
    });
}
#[test]
fn fx_blur_motion() {
    let mut b = Vec::new();
    words(&mut b, &[0x03082000]);
    floats(&mut b, &[1.0, 2.0]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x03082000, &b, &[0x03082000], |node| {
        let n: &MediaBlockFxBlurMotion = node.try_into().unwrap();
        assert_eq!(n.start, Some(1.0));
        assert_eq!(n.end, Some(2.0));
    });
}
#[test]
fn trails() {
    let mut b = Vec::new();
    words(&mut b, &[0x030A9000]);
    floats(&mut b, &[1.0, 2.0]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x030A9000, &b, &[0x030A9000], |node| {
        let n: &MediaBlockTrails = node.try_into().unwrap();
        assert_eq!(n.start, Some(1.0));
        assert_eq!(n.end, Some(2.0));
    });
}
#[test]
fn camera_path() {
    let mut b = Vec::new();
    words(&mut b, &[0x030A1002, 1]);
    floats(&mut b, &[1.0, 2.0, 3.0, 4.0, 0.1, 0.2, 0.3, 90.0]);
    words(&mut b, &[1, 7, 0, u32::MAX]);
    floats(&mut b, &[5.0, 6.0, 7.0, 0.5, 0.0, 0.0, 0.0, 1.0]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x030A1000, &b, &[0x030A1002], |node| {
        let n: &MediaBlockCameraPath = node.try_into().unwrap();
        let k = &n.keys.as_ref().unwrap()[0];
        assert_eq!(k.anchor, 7);
        assert_eq!(k.target_position, [5.0, 6.0, 7.0]);
        assert_eq!(k.rotation, [0.0, 0.0, 0.0, 1.0]);
    });
}
#[test]
fn camera_shake() {
    let mut b = Vec::new();
    words(&mut b, &[0x030A4000, 1]);
    floats(&mut b, &[1.0, 2.0, 3.0]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x030A4000, &b, &[0x030A4000], |node| {
        let n: &MediaBlockCameraEffectShake = node.try_into().unwrap();
        assert_eq!(n.keys.as_ref().unwrap()[0].speed, 3.0);
    });
}
#[test]
fn effect_simi() {
    let mut b = Vec::new();
    words(&mut b, &[0x07010005, 1]);
    floats(
        &mut b,
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.5, 0.25, 7.0, 8.0, 9.0, 10.0],
    );
    words(&mut b, &[1, 1, 0, 1, 0xFACA_DE01]);
    check(0x07010000, &b, &[0x07010005], |node| {
        let n: &ControlEffectSimi = node.try_into().unwrap();
        let k = &n.keys.as_ref().unwrap()[0];
        assert_eq!(k.position, [2.0, 3.0]);
        assert_eq!(k.u04, 10.0);
        assert_eq!(n.is_interpolated, Some(true));
    });
}
fn function_body(real: bool) -> Vec<u8> {
    let mut b = Vec::new();
    words(&mut b, &[0x05002001, 2]);
    floats(&mut b, &[1.0, 2.0]);
    words(&mut b, &[0x05002003, 3, 0x40000000]);
    string(&mut b, "Function");
    if real {
        words(&mut b, &[0x0501A001, 2]);
        floats(&mut b, &[3.0, 4.0]);
        words(&mut b, &[2]);
    }
    words(&mut b, &[0xFACA_DE01]);
    b
}
#[test]
fn func_keys() {
    check(
        0x05002000,
        &function_body(false),
        &[0x05002001, 0x05002003],
        |node| {
            let n: &FuncKeys = node.try_into().unwrap();
            assert_eq!(n.xs.as_ref().unwrap(), &[1.0, 2.0]);
        },
    );
}
#[test]
fn func_keys_real() {
    check(
        0x0501A000,
        &function_body(true),
        &[0x05002001, 0x05002003, 0x0501A001],
        |node| {
            let n: &FuncKeysReal = node.try_into().unwrap();
            assert_eq!(n.xs.as_ref().unwrap(), &[1.0, 2.0]);
            assert_eq!(n.ys.as_ref().unwrap(), &[3.0, 4.0]);
            assert_eq!(n.real_interp, Some(2));
        },
    );
}
#[test]
fn named_refusals() {
    // Unknown explicit sound versions refuse before interpreting their dependent layouts.
    for chunk in [0x030A7003, 0x030A7004] {
        let mut b = Vec::new();
        words(&mut b, &[chunk]);
        if chunk == 0x030A7004 {
            pack(&mut b);
        }
        words(&mut b, &[99, 0xFACA_DE01]);
        let e = Gbx::parse(&file(0x030A7000, &b))
            .unwrap()
            .nodes()
            .unwrap_err()
            .to_string();
        assert!(e.contains("version 99"), "{e}");
    }
    let mut b = Vec::new();
    words(&mut b, &[0x03078004, 2, 0, 0xFACA_DE01]);
    assert!(
        Gbx::parse(&file(0x03078000, &b))
            .unwrap()
            .nodes()
            .unwrap_err()
            .to_string()
            .contains("bool 2")
    );
    let mut b = Vec::new();
    words(&mut b, &[0x03079005, 11, 0, 0, 0xFACA_DE01]);
    assert!(
        Gbx::parse(&file(0x03079000, &b))
            .unwrap()
            .nodes()
            .unwrap_err()
            .to_string()
            .contains("deprecated node list version 11")
    );
    let mut b = Vec::new();
    words(&mut b, &[0x0307A003, 10, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0xFACA_DE01]);
    assert!(
        Gbx::parse(&file(0x0307A000, &b))
            .unwrap()
            .nodes()
            .unwrap_err()
            .to_string()
            .contains("clip and trigger counts differ")
    );
    let mut b = Vec::new();
    words(&mut b, &[0x03029001, 1]);
    floats(&mut b, &[1.0]);
    words(&mut b, &[2, 0, 0xFACA_DE01]);
    assert!(
        Gbx::parse(&file(0x0304B000, &b))
            .unwrap()
            .nodes()
            .unwrap_err()
            .to_string()
            .contains("row count")
    );
    let mut b = triangle_body(); // Triangle indices are after the three vertex colors.
    b[116..120].copy_from_slice(&3u32.to_le_bytes());
    assert!(
        Gbx::parse(&file(0x0304C000, &b))
            .unwrap()
            .nodes()
            .unwrap_err()
            .to_string()
            .contains("vertex index")
    );
}

#[test]
fn challenge_references_resolve_typed_clips() {
    use gbx_formats::NodeIndex;
    use gbx_formats::game::challenge::Challenge;
    let mut b = Vec::new();
    // Challenge's intro introduces a clip; both group references share one node.
    words(&mut b, &[0x03043021, 1, 0x03079000, 0x03079005, 10, 0]);
    string(&mut b, "Intro");
    words(
        &mut b,
        &[0xFACA_DE01, 2, 0x0307A000, 0x0307A003, 10, 1, 1, 1, 0, 0, 0, 0, 0],
    );
    floats(&mut b, &[0.0]);
    words(&mut b, &[0, 0xFACA_DE01, 2, 0x03043026, 1, 0xFACA_DE01]);
    let nodes = read(0x03043000, &b);
    let map: &Challenge = nodes.main().try_into().unwrap();
    let refs = map.clips.as_ref().unwrap();
    assert_eq!(refs.intro(&nodes).unwrap().name.as_deref(), Some("Intro"));
    assert!(std::ptr::eq(
        refs.intro(&nodes).unwrap(),
        map.clip_global(&nodes).unwrap()
    ));
    assert!(std::ptr::eq(
        refs.in_game(&nodes).unwrap(),
        refs.end_race(&nodes).unwrap()
    ));
    assert_eq!(
        refs.in_game(&nodes).unwrap().clips.as_ref().unwrap(),
        &[Some(NodeIndex(1))]
    );
    let absent = gbx_formats::game::challenge::Clips {
        intro: None,
        in_game: Some(NodeIndex(1)),
        end_race: Some(NodeIndex(15)),
    };
    assert!(absent.intro(&nodes).is_none());
    assert!(absent.in_game(&nodes).is_none());
    assert!(absent.end_race(&nodes).is_none());
}
#[test]
fn music_effect() {
    let mut b = Vec::new();
    words(&mut b, &[0x030A6001, 1]);
    floats(&mut b, &[2.0, 0.5, 0.25]);
    words(&mut b, &[0xFACA_DE01]);
    check(0x030A6000, &b, &[0x030A6001], |node| {
        let n: &MediaBlockMusicEffect = node.try_into().unwrap();
        let k = &n.keys.as_ref().unwrap()[0];
        assert_eq!((k.time, k.music_volume, k.sound_volume), (2.0, 0.5, 0.25));
    });
}
