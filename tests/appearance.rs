//! Hand-built appearance archives from GBX.NET's TMF layouts; no game bytes.
#![allow(clippy::unwrap_used, reason = "hand-built fixture assertions")]
use gbx_formats::plug::{bitmap::*, func_shader::*, light::*, material::Material, shader::*};
use gbx_formats::{ExternalTarget, Gbx, Node, NodeIndex};
const END: u32 = 0xFACA_DE01;
const NULL: u32 = u32::MAX;
fn w(b: &mut Vec<u8>, xs: &[u32]) {
    for x in xs {
        b.extend_from_slice(&x.to_le_bytes());
    }
}
fn f(b: &mut Vec<u8>, xs: &[f32]) {
    for x in xs {
        b.extend_from_slice(&x.to_le_bytes());
    }
}
fn s(b: &mut Vec<u8>, text: &str) {
    w(b, &[u32::try_from(text.len()).unwrap()]);
    b.extend_from_slice(text.as_bytes());
}
fn skip(b: &mut Vec<u8>, id: u32, data: &[u8]) {
    w(b, &[id, 0x534B4950, u32::try_from(data.len()).unwrap()]);
    b.extend_from_slice(data);
}
fn file(class: u32, body: &[u8]) -> Vec<u8> {
    let mut b = b"GBX\x06\x00BUUR".to_vec();
    w(&mut b, &[class, 0, 32, 2, 0, 0]);
    for (name, index) in [("Diffuse.dds", 30), ("Pixel.Gpu.Gbx", 31)] {
        w(&mut b, &[0]);
        s(&mut b, name);
        w(&mut b, &[index, 1, 0]);
    }
    b.extend_from_slice(body);
    b
}
fn check(class: u32, body: &[u8], ids: &[u32], verify: impl FnOnce(&Node)) {
    let bytes = file(class, body);
    let gbx = Gbx::parse(&bytes).unwrap();
    let ns = gbx.nodes().unwrap();
    assert_eq!(ns.main().chunks().ids, ids);
    assert!(ns.main().chunks().unread.is_empty());
    verify(ns.main());
    for n in 0..body.len() {
        assert!(
            Gbx::parse(&file(class, &body[..n])).unwrap().nodes().is_err(),
            "{class:#x} prefix {n}"
        );
    }
    let mut bad = Vec::new();
    w(&mut bad, &[class | 0xFFF, 0, END]);
    assert!(Gbx::parse(&file(class, &bad)).unwrap().nodes().is_err());
}
fn bitmap_body(mode: u64) -> Vec<u8> {
    let mut b = Vec::new();
    w(&mut b, &[0x09011019]);
    f(&mut b, &[0.5]);
    w(&mut b, &[0x0901101C]);
    f(&mut b, &[2.0, 3.0, 0.25, 0.5, 0.75]);
    w(&mut b, &[0x0901101E, 1]);
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&3u16.to_le_bytes());
    w(&mut b, &[0x09011020, 2]);
    f(&mut b, &[0.25, 0.75]);
    w(&mut b, &[0x09011022, 30]);
    b.extend_from_slice(&mode.to_le_bytes());
    f(&mut b, &[0.1, 2.0, -0.5]);
    w(&mut b, &[0xFF123456]);
    match (mode >> 8) & 255 {
        4 => w(&mut b, &[NULL]),
        7 => {
            w(&mut b, &[1]);
            f(&mut b, &[1.0, 0.5, 0.25, 16.0]);
        }
        8 => w(&mut b, &[0xFFABCDEF]),
        _ => {}
    }
    w(&mut b, &[0x09011024, 0x02000A00, END]);
    b
}
#[test]
fn bitmap() {
    let body = bitmap_body(0x1122000000000001);
    check(
        0x09011000,
        &body,
        &[
            0x09011019, 0x0901101C, 0x0901101E, 0x09011020, 0x09011022, 0x09011024,
        ],
        |node| {
            let n: &TextureBitmap = node.try_into().unwrap();
            let image = n.image.as_ref().unwrap();
            assert_eq!(image.image, Some(NodeIndex(30)));
            assert_eq!(image.flags, 0x1122000000000001);
            assert_eq!(image.mipmap_lod_bias, -0.5);
            assert_eq!(n.atlas_counts.as_ref().unwrap(), &[[2, 3]]);
            assert_eq!(n.texcoord_scale, Some([2.0, 3.0]));
        },
    );
    let bytes = file(0x09011000, &body);
    let gbx = Gbx::parse(&bytes).unwrap();
    assert_eq!(gbx.ref_table.external[0].node_index, 30);
    assert_eq!(
        gbx.ref_table.external[0].target,
        ExternalTarget::File {
            name: "Diffuse.dds".into(),
            folder: None
        }
    );
    for mode in [4, 7, 8] {
        let body = bitmap_body(mode << 8);
        let ns = Gbx::parse(&file(0x09011000, &body)).unwrap().nodes().unwrap();
        let n: &TextureBitmap = ns.main().try_into().unwrap();
        if mode == 7 {
            assert_eq!(
                n.image.as_ref().unwrap().specular_submaps.as_ref().unwrap()[0].exponent,
                16.0
            );
        }
        if mode == 8 {
            assert_eq!(n.image.as_ref().unwrap().clear_color, Some(0xFFABCDEF));
        }
    }
}
#[test]
fn bitmap_apply() {
    let mut b = Vec::new();
    w(&mut b, &[0x0907E008, 3, 0x40000000]);
    s(&mut b, "Diffuse");
    w(&mut b, &[30, 0x18]);
    f(&mut b, &[0.5]);
    w(&mut b, &[0xFF010203, 0x09047007, 0x00201000, 7]);
    b.push(3);
    f(&mut b, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    f(
        &mut b,
        &[
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ],
    );
    w(&mut b, &[0x09047009]);
    f(&mut b, &[2.0]);
    w(&mut b, &[0x09012004, 0x4567, END]);
    check(
        0x09012000,
        &b,
        &[0x0907E008, 0x09047007, 0x09047009, 0x09012004],
        |node| {
            let n: &BitmapApply = node.try_into().unwrap();
            let sampler = n.sampler.as_ref().unwrap();
            assert_eq!(sampler.bitmap, Some(NodeIndex(30)));
            assert_eq!(sampler.border_color, Some(0xFF010203));
            let a = n.address.as_ref().unwrap();
            assert_eq!(a.texcoord_index, 7);
            assert_eq!(a.iso3.unwrap()[2], [5.0, 6.0]);
            assert_eq!(a.matrix.unwrap()[3], [13.0, 14.0, 15.0, 16.0]);
            assert_eq!(n.apply_flags, Some(0x4567));
        },
    );
}
fn render_base() -> Vec<u8> {
    let mut b = Vec::new();
    w(&mut b, &[0x09086003]);
    for v in [1u16, 2, 3, 4] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    let mut p = Vec::new();
    w(&mut p, &[2, 1]);
    skip(&mut b, 0x09086008, &p);
    w(&mut b, &[0x0908600A]);
    f(&mut b, &[500.0]);
    w(&mut b, &[0x0908600B, 0xFF010203, 0x2E8007E2, 30]);
    f(&mut b, &[0.25, 0.75]);
    w(&mut b, &[0x0908600C, NULL, 0x0908600E, 0x7800]);
    b
}
fn render_ids(own: &[u32]) -> Vec<u32> {
    let mut ids = vec![
        0x09086003, 0x09086008, 0x0908600A, 0x0908600B, 0x0908600C, 0x0908600E,
    ];
    ids.extend_from_slice(own);
    ids
}
fn render_check(n: &RenderSettings) {
    assert_eq!(n.item_visibility_mask, Some(1));
    assert_eq!(n.shader_visibility_ref, Some(4));
    assert_eq!(n.render_path, Some(2));
    assert_eq!(n.bitmap_clear, Some(Some(NodeIndex(30))));
    assert_eq!(n.bitmap_clear_uv, Some([0.25, 0.75]));
    assert_eq!(n.render_flags2, Some(0x7800));
}
#[test]
fn render_water() {
    let mut b = render_base();
    let mut p = Vec::new();
    w(&mut p, &[1]);
    f(&mut p, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    f(
        &mut p,
        &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 7.0, 8.0, 9.0],
    );
    w(&mut p, &[0x17]);
    f(&mut p, &[10.0, 11.0, -1.0]);
    skip(&mut b, 0x09087005, &p);
    let mut p = Vec::new();
    f(&mut p, &[64.0]);
    skip(&mut b, 0x09087006, &p);
    w(&mut b, &[END]);
    check(0x09087000, &b, &render_ids(&[0x09087005, 0x09087006]), |node| {
        let n: &BitmapRenderWater = node.try_into().unwrap();
        render_check(&n.base);
        let p = n.water.as_ref().unwrap();
        assert!(p.is_orthographic);
        assert_eq!(p.frustum.half_size, [4.0, 5.0, 6.0]);
        assert_eq!(p.transform.translation, [7.0, 8.0, 9.0]);
        assert_eq!(p.flags, 0x17);
        assert_eq!(p.mirror_scale_y, -1.0);
        assert_eq!(n.fog_max_depth, Some(64.0));
    });
}
#[test]
fn render_overlay() {
    let mut b = render_base();
    w(&mut b, &[0x0909E000, 1, END]);
    check(0x0909E000, &b, &render_ids(&[0x0909E000]), |node| {
        let n: &BitmapRenderOverlay = node.try_into().unwrap();
        render_check(&n.base);
        assert_eq!(n.is_overlay_self_blur, Some(true));
    });
}
#[test]
fn render_light_occ() {
    let mut b = render_base();
    w(&mut b, &[0x0909F000]);
    f(&mut b, &[90.0, 0.5]);
    w(&mut b, &[30, END]);
    check(0x0909F000, &b, &render_ids(&[0x0909F000]), |node| {
        let n: &BitmapRenderLightOcc = node.try_into().unwrap();
        render_check(&n.base);
        assert_eq!(n.opacity, Some(0.5));
        assert_eq!(n.bitmap_to_modulate, Some(Some(NodeIndex(30))));
    });
}
#[test]
fn render_camera() {
    let mut b = render_base();
    let mut p = Vec::new();
    w(&mut p, &[123]);
    skip(&mut b, 0x09089001, &p);
    w(&mut b, &[END]);
    check(0x09089000, &b, &render_ids(&[0x09089001]), |node| {
        let n: &BitmapRenderCamera = node.try_into().unwrap();
        render_check(&n.base);
        assert_eq!(n.u01, Some(123));
    });
}
#[test]
fn render_sub() {
    let mut b = render_base();
    w(&mut b, &[0x09091000, 31, END]);
    check(0x09091000, &b, &render_ids(&[0x09091000]), |node| {
        let n: &BitmapRenderSub = node.try_into().unwrap();
        render_check(&n.base);
        assert_eq!(n.shader_to_force, Some(Some(NodeIndex(31))));
    });
}
#[test]
fn render_hemisphere() {
    let mut b = render_base();
    w(&mut b, &[0x09058001, 1]);
    f(&mut b, &[20.0, 40.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    w(&mut b, &[END]);
    check(0x09058000, &b, &render_ids(&[0x09058001]), |node| {
        let n: &BitmapRenderHemisphere = node.try_into().unwrap();
        render_check(&n.base);
        assert_eq!(n.specular.as_ref().unwrap().power1, 40.0);
        assert_eq!(n.specular.as_ref().unwrap().u06, 6.0);
    });
}
#[test]
fn render_light_from_map() {
    let mut b = render_base();
    w(&mut b, &[0x09021001, 2, 4]);
    f(&mut b, &[0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]);
    w(&mut b, &[END]);
    check(0x09021000, &b, &render_ids(&[0x09021001]), |node| {
        let n: &BitmapRenderLightFromMap = node.try_into().unwrap();
        render_check(&n.base);
        assert_eq!(n.settings.as_ref().unwrap().object_count_max, 4);
        assert_eq!(n.settings.as_ref().unwrap().remap_max_day_directional, 0.8);
    });
}
#[test]
fn shader_apply() {
    let mut b = Vec::new();
    w(&mut b, &[0x0900200E, NULL, 1, 31, NULL, 1, 31, 0x09002016]);
    b.extend_from_slice(&0x0102030405060708u64.to_le_bytes());
    f(&mut b, &[0.5]);
    w(&mut b, &[NULL]);
    b.extend_from_slice(&0x1234u16.to_le_bytes());
    w(&mut b, &[0x09004003]);
    f(
        &mut b,
        &[
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.5, 7.0, 8.0, 9.0, 20.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
            17.0, 18.0, 0.75,
        ],
    );
    w(
        &mut b,
        &[
            5, 0x09026002, 1, 30, 0x09026004, 0x01010101, 0x09026008, 0x06000001, 0x007F007F, END,
        ],
    );
    check(
        0x09026000,
        &b,
        &[
            0x0900200E, 0x09002016, 0x09004003, 0x09026002, 0x09026004, 0x09026008,
        ],
        |node| {
            let n: &ShaderApply = node.try_into().unwrap();
            assert_eq!(n.passes.as_ref().unwrap(), &[Some(NodeIndex(31))]);
            assert_eq!(n.bitmap_addresses.as_ref().unwrap(), &[Some(NodeIndex(30))]);
            assert_eq!(n.requirements.as_ref().unwrap().flags, 0x0102030405060708);
            assert_eq!(n.material.as_ref().unwrap().emissive_modifier, [13.0, 14.0, 15.0]);
            assert_eq!(n.material.as_ref().unwrap().wire_alpha, 0.75);
            assert_eq!(n.visible_id_flags, Some(0x1234));
        },
    );
}
#[test]
fn shader_pass() {
    let mut b = Vec::new();
    w(
        &mut b,
        &[0x09067006, 1, 30, 0x09067007, 0x123, 0x0906700A, 1, 3, 0x40000000],
    );
    s(&mut b, "Skip");
    w(&mut b, &[0, NULL, 1, 31, 1, 0x40000000]);
    s(&mut b, "Color");
    w(&mut b, &[2, 1, 2, 0, 2]);
    f(&mut b, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    w(&mut b, &[END]);
    check(0x09067000, &b, &[0x09067006, 0x09067007, 0x0906700A], |node| {
        let n: &ShaderPass = node.try_into().unwrap();
        assert_eq!(n.vertex_textures.as_ref().unwrap(), &[Some(NodeIndex(30))]);
        assert!(n.vertex_pipeline.as_ref().unwrap().values.is_none());
        let p = n.pixel_pipeline.as_ref().unwrap();
        assert!(p.has_external_file);
        assert_eq!(p.file_gpu, Some(NodeIndex(31)));
        assert_eq!(p.load_fxs.as_ref().unwrap()[0].element_count, 2);
        assert_eq!(p.values.as_ref().unwrap()[1], [5.0, 6.0, 7.0, 8.0]);
    });
}
fn timing(b: &mut Vec<u8>) {
    w(b, &[0x0500B005]);
    f(b, &[3.0, 0.5]);
    w(b, &[1, 0, 3, 0x40000000]);
    s(b, "RaceTime");
}
#[test]
fn function_layer_uv() {
    let mut b = Vec::new();
    timing(&mut b);
    w(&mut b, &[0x05015005]);
    s(&mut b, "Diffuse");
    w(&mut b, &[7, 0x05015009]);
    f(&mut b, &[1.0, 2.0, 3.0, 4.0]);
    for id in [0x0501500A, 0x0501500D, 0x05015013] {
        w(&mut b, &[id]);
        f(&mut b, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }
    w(&mut b, &[0x05015014]);
    f(&mut b, &[2.0, 3.0, 4.0, 5.0]);
    w(&mut b, &[0x05015015, 1, 2, 3, 4, 0x05015016, 0x1234, END]);
    check(
        0x05015000,
        &b,
        &[
            0x0500B005, 0x05015005, 0x05015009, 0x0501500A, 0x0501500D, 0x05015013, 0x05015014, 0x05015015,
            0x05015016,
        ],
        |node| {
            let n: &FuncShaderLayerUv = node.try_into().unwrap();
            assert_eq!(n.timing.as_ref().unwrap().period, 3.0);
            assert_eq!(n.layer_name.as_deref(), Some("Diffuse"));
            assert_eq!(n.uv13.unwrap()[2], [5.0, 6.0]);
            assert_eq!(n.u15, Some([1, 2, 3, 4]));
        },
    );
}
#[test]
fn functions() {
    let mut b = Vec::new();
    timing(&mut b);
    w(&mut b, &[0x05014000, 2, NULL, 31, END]);
    check(0x05014000, &b, &[0x0500B005, 0x05014000], |node| {
        let n: &FuncShaders = node.try_into().unwrap();
        assert_eq!(n.functions.as_ref().unwrap(), &[None, Some(NodeIndex(31))]);
        assert_eq!(n.timing.as_ref().unwrap().phase, 0.5);
    });
}
fn gx_common() -> Vec<u8> {
    let mut b = Vec::new();
    w(&mut b, &[0x04001009]);
    f(&mut b, &[1.0, 0.5, 0.25]);
    w(&mut b, &[0x1FF]);
    f(&mut b, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.1, 0.2, 0.3]);
    b
}
fn gx_ball() -> Vec<u8> {
    let mut b = gx_common();
    w(&mut b, &[0x04003004]);
    f(&mut b, &[1.0, 2.0]);
    w(&mut b, &[0x04002006, 0x10]);
    f(&mut b, &[10.0, 11.0, 12.0, 13.0, 0.5, -1.0, -2.0, 0.1, 0.2, 0.3]);
    b
}
fn gx_check(n: &BallLightData) {
    assert_eq!(n.base.common.as_ref().unwrap().flags, 0x1FF);
    assert_eq!(n.point.as_ref().unwrap().flare_bias_z, 2.0);
    let p = n.ball.as_ref().unwrap();
    assert_eq!(p.radius_flare, 13.0);
    assert_eq!(p.attenuation1, -1.0);
}
#[test]
fn light_ball() {
    let mut b = gx_ball();
    w(&mut b, &[END]);
    check(0x04002000, &b, &[0x04001009, 0x04003004, 0x04002006], |node| {
        let n: &LightBall = node.try_into().unwrap();
        gx_check(&n.base);
    });
}
#[test]
fn light_spot() {
    let mut b = gx_ball();
    w(&mut b, &[0x0400B001]);
    f(&mut b, &[30.0, 45.0, 60.0, 1.0]);
    w(&mut b, &[0x0400B002, 3]);
    f(&mut b, &[10.0, 20.0, 30.0, 40.0, 50.0, 2.0]);
    w(&mut b, &[END]);
    check(
        0x0400B000,
        &b,
        &[0x04001009, 0x04003004, 0x04002006, 0x0400B001, 0x0400B002],
        |node| {
            let n: &LightSpot = node.try_into().unwrap();
            gx_check(&n.base);
            assert_eq!(n.legacy_angles.as_ref().unwrap().outer, 45.0);
            assert_eq!(n.spot.as_ref().unwrap().outer_shadow, 50.0);
        },
    );
}
#[test]
fn light_frustum() {
    let mut b = gx_ball();
    w(&mut b, &[0x0400A006, 1]);
    f(&mut b, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    w(&mut b, &[0x80010, END]);
    check(
        0x0400A000,
        &b,
        &[0x04001009, 0x04003004, 0x04002006, 0x0400A006],
        |node| {
            let n: &LightFrustum = node.try_into().unwrap();
            gx_check(&n.base);
            let p = n.frustum.as_ref().unwrap();
            assert!(p.is_orthographic);
            assert_eq!(p.z2, 6.0);
            assert_eq!(p.flags, 0x80010);
        },
    );
}
#[test]
fn light_directional() {
    let mut b = gx_common();
    w(&mut b, &[0x04007002, 1]);
    f(&mut b, &[1.0, 2.0, 3.0, 30.0, 0.5]);
    w(&mut b, &[0x04007003]);
    f(&mut b, &[0.1, 0.2, 0.3]);
    w(&mut b, &[0x04007004]);
    f(&mut b, &[0.4, 0.5, 0.6, 2.0]);
    w(&mut b, &[0x04007005]);
    f(&mut b, &[0.75, 0.9]);
    w(&mut b, &[END]);
    check(
        0x04007000,
        &b,
        &[0x04001009, 0x04007002, 0x04007003, 0x04007004, 0x04007005],
        |node| {
            let n: &LightDirectional = node.try_into().unwrap();
            assert_eq!(n.boundary.as_ref().unwrap().hint_position, [1.0, 2.0, 3.0]);
            assert_eq!(n.reverse_intensity, Some(2.0));
            assert_eq!(n.flare_angular_size, Some(0.9));
        },
    );
}
#[test]
fn plug_light() {
    let mut b = Vec::new();
    w(&mut b, &[0x0901D002, NULL, NULL, 30, 31, 0xF, END]);
    check(0x0901D000, &b, &[0x0901D002], |node| {
        let n: &Light = node.try_into().unwrap();
        assert_eq!(n.bitmap_flare, Some(Some(NodeIndex(30))));
        assert_eq!(n.flags, Some(0xF));
    });
}
fn hms() -> Vec<u8> {
    let mut b = Vec::new();
    w(&mut b, &[0x0600C003, 17, NULL, 30, 31, END]);
    b
}
#[test]
fn hms_light() {
    check(0x0600C000, &hms(), &[0x0600C003], |node| {
        let n: &HmsLight = node.try_into().unwrap();
        assert_eq!(n.u01, Some(17));
        assert_eq!(n.bitmap_sprite, Some(Some(NodeIndex(31))));
    });
}
#[test]
fn scene_light() {
    let mut b = Vec::new();
    w(&mut b, &[0x0A005001, 3, 0x40000000]);
    s(&mut b, "Sun");
    w(&mut b, &[0x0A005003, NULL, 0x0A009000, 1, 0x0A00B000]);
    b.extend_from_slice(&hms());
    w(&mut b, &[END]);
    check(
        0x0A00B000,
        &b,
        &[0x0A005001, 0x0A005003, 0x0A009000, 0x0A00B000],
        |node| {
            let n: &SceneLight = node.try_into().unwrap();
            assert_eq!(n.is_active, Some(true));
            assert_eq!(n.light.as_ref().unwrap().u01, Some(17));
        },
    );
}
#[test]
fn lightmap_mood() {
    let mut b = Vec::new();
    w(&mut b, &[0x06023000]);
    f(&mut b, &[2.0, 3.0, 0.35, 1.0, 0.5]);
    w(&mut b, &[END]);
    check(0x06023000, &b, &[0x06023000], |node| {
        let n: &HmsLightMapMood = node.try_into().unwrap();
        assert_eq!(n.u01, Some([2.0, 3.0]));
        assert_eq!(n.ambient_range, Some([0.35, 1.0]));
        assert_eq!(n.directional_scale, Some(0.5));
    });
}
#[test]
fn lightmap() {
    let mut b = Vec::new();
    w(
        &mut b,
        &[0x06021001, 30, 0x06021002, NULL, 0x06021003, 31, 0x06021004],
    );
    w(&mut b, &[NULL; 15]);
    w(&mut b, &[END]);
    check(
        0x06021000,
        &b,
        &[0x06021001, 0x06021002, 0x06021003, 0x06021004],
        |node| {
            let n: &HmsLightMap = node.try_into().unwrap();
            assert_eq!(n.mood, Some(Some(NodeIndex(31))));
            assert_eq!(n.legacy_nodes, Some([None; 15]));
        },
    );
}
#[test]
fn image_generator() {
    let mut b = Vec::new();
    w(&mut b, &[0x80000002, 2, 2, 64, 32, 1]);
    f(&mut b, &[1.0, 0.5, 0.25, 1.0]);
    w(&mut b, &[2]);
    f(&mut b, &[0.25, 0.75]);
    check(0x0902F000, &b, &[], |node| {
        let n: &FileGenerator = node.try_into().unwrap();
        assert_eq!(n.version, 2);
        assert_eq!(n.kind, 2);
        assert_eq!(n.int_params, [64, 32]);
        assert_eq!(n.color_params, [[1.0, 0.5, 0.25, 1.0]]);
        assert_eq!(n.real_params, [0.25, 0.75]);
    });
    let mut bad = Vec::new();
    w(&mut bad, &[0x80000003, 2, 0, 0, 0]);
    assert!(
        Gbx::parse(&file(0x0902F000, &bad))
            .unwrap()
            .nodes()
            .unwrap_err()
            .to_string()
            .contains("version")
    );
}
#[test]
fn inline_device_material() {
    let mut b = Vec::new();
    w(&mut b, &[0x0907900D, NULL, 1]);
    b.extend_from_slice(&2i16.to_le_bytes());
    b.extend_from_slice(&3i16.to_le_bytes());
    w(&mut b, &[1, 31, NULL, 30, 2, 7, 8, END]);
    check(0x09079000, &b, &[0x0907900D], |node| {
        let n: &Material = node.try_into().unwrap();
        assert_eq!(n.shader, None);
        let d = &n.device_materials.as_ref().unwrap()[0];
        assert_eq!((d.u01, d.u02, d.u03), (2, 3, true));
        assert_eq!(d.shader1, Some(NodeIndex(31)));
        assert_eq!(n.device_material_values.as_ref().unwrap(), &[7, 8]);
    });
}

#[test]
fn directional_light_refuses_unrelated_ball_chunks() {
    let mut body = Vec::new();
    w(&mut body, &[0x04002006, 0]);
    f(&mut body, &[0.0; 10]);
    w(&mut body, &[END]);
    let err = Gbx::parse(&file(0x04007000, &body))
        .unwrap()
        .nodes()
        .unwrap_err()
        .to_string();
    assert!(err.contains("chunk 0x04002006 of GxLightDirectional"), "{err}");
}
