//! Every class whose nodes this crate reads, by class id: a node of any other class is refused
//! ([`crate::Error::UnknownClass`]).
//!
//! Each class also names the id a pack file's reader writes as it starts one of its nodes (see [`crate::pak`]): its
//! parent class's id as the game knew it when TrackMania Nations Forever's packs were made, which for most classes is
//! the old id of the class it derives from (`CGameCtnMediaBlockText`, `0x030A8000`, derives from
//! `CGameCtnMediaBlock`, whose old id is `0x2406A000`). An unchunked archive that overrides the chunk reader uses
//! `parent none`: it does not make that write.

use crate::archive::Archive;
use crate::error::{Error, Result};
use crate::game;
use crate::node::Chunks;
use crate::plug;

macro_rules! classes {
    (@write $r:ident, none) => {};
    (@write $r:ident, $parent:literal) => { $r.dummy_write(u32::to_le_bytes($parent)); };
    ($($(#[$doc:meta])* $variant:ident($ty:path) = $id:literal, $name:literal, parent $parent:tt;)*) => {
        /// A node of a class this crate reads.
        #[derive(Clone, Debug)]
        pub enum Node {
            $($(#[$doc])* $variant(Box<$ty>),)*
        }

        impl Node {
            /// The class id.
            pub fn class_id(&self) -> u32 {
                match self {
                    $(Node::$variant(_) => $id,)*
                }
            }

            /// The class name, as Nadeo's engine names it.
            pub fn class_name(&self) -> &'static str {
                match self {
                    $(Node::$variant(_) => $name,)*
                }
            }

            /// The node's chunks: their ids in order, and the skippable ones not read.
            pub fn chunks(&self) -> &Chunks {
                match self {
                    $(Node::$variant(n) => &n.chunks,)*
                }
            }
        }

        $(
            impl<'n> TryFrom<&'n Node> for &'n $ty {
                type Error = &'n Node;
                fn try_from(node: &'n Node) -> std::result::Result<Self, &'n Node> {
                    match node {
                        Node::$variant(n) => Ok(n),
                        #[allow(unreachable_patterns, reason = "a list of one class has no other")]
                        other => Err(other),
                    }
                }
            }
        )*

        /// The name of a class this crate reads, by id.
        pub fn class_name(id: u32) -> Option<&'static str> {
            match id {
                $($id => Some($name),)*
                _ => None,
            }
        }

        /// Reads a node of class `class` (its id read at `at`), making the parent-class dummy write for chunked
        /// nodes. An unchunked archive marked `parent none` reads without this write.
        pub(crate) fn read_node(r: &mut Archive<'_, '_>, class: u32, at: usize) -> Result<Node> {
            match class {
                $($id => {
                    classes!(@write r, $parent);
                    Ok(Node::$variant(Box::new(<$ty>::read(r)?)))
                })*
                _ => Err(Error::UnknownClass {
                    space: r.space(),
                    offset: at as u128,
                    class,
                }),
            }
        }
    };
}

classes! {
    /// A map.
    Challenge(game::challenge::Challenge) = 0x0304_3000, "CGameCtnChallenge", parent 0x0100_1000;
    /// The block models a map's editor offers.
    CollectorList(game::collector::CollectorList) = 0x0301_B000, "CGameCtnCollectorList", parent 0x0100_1000;
    /// A map's tips, medal times and time limit.
    ChallengeParameters(game::collector::ChallengeParameters) = 0x0305_B000, "CGameCtnChallengeParameters", parent 0x0100_1000;
    /// A block's skin.
    BlockSkin(game::block::BlockSkin) = 0x0305_9000, "CGameCtnBlockSkin", parent 0x0100_1000;
    /// CGameCtnMediaClipGroup.
    MediaClipGroup(game::media::MediaClipGroup) = 0x0307_A000, "CGameCtnMediaClipGroup", parent 0x0100_1000;
    /// CGameCtnMediaClip.
    MediaClip(game::media::MediaClip) = 0x0307_9000, "CGameCtnMediaClip", parent 0x0100_1000;
    /// CGameCtnMediaTrack.
    MediaTrack(game::media::MediaTrack) = 0x0307_8000, "CGameCtnMediaTrack", parent 0x0100_1000;
    /// CGameCtnMediaBlockText.
    MediaBlockText(game::media::MediaBlockText) = 0x030A_8000, "CGameCtnMediaBlockText", parent 0x2406_A000;
    /// CGameCtnMediaBlockCameraCustom.
    MediaBlockCameraCustom(game::media::MediaBlockCameraCustom) = 0x030A_2000, "CGameCtnMediaBlockCameraCustom", parent 0x2406_9000;
    /// CGameCtnMediaBlockTransitionFade.
    MediaBlockTransitionFade(game::media::MediaBlockTransitionFade) = 0x030A_B000, "CGameCtnMediaBlockTransitionFade", parent 0x2408_2000;
    /// CGameCtnMediaBlockMusicEffect.
    MediaBlockMusicEffect(game::media::MediaBlockMusicEffect) = 0x030A_6000, "CGameCtnMediaBlockMusicEffect", parent 0x2406_A000;
    /// CGameCtnMediaBlockCameraGame.
    MediaBlockCameraGame(game::media::MediaBlockCameraGame) = 0x0308_4000, "CGameCtnMediaBlockCameraGame", parent 0x2406_9000;
    /// CGameCtnMediaBlockTriangles.
    MediaBlockTriangles(game::media::MediaBlockTriangles) = 0x0302_9000, "CGameCtnMediaBlockTriangles", parent 0x2406_A000;
    /// CGameCtnMediaBlockTriangles2D.
    MediaBlockTriangles2D(game::media::MediaBlockTriangles2D) = 0x0304_B000, "CGameCtnMediaBlockTriangles2D", parent 0x0302_9000;
    /// CGameCtnMediaBlockTriangles3D.
    MediaBlockTriangles3D(game::media::MediaBlockTriangles3D) = 0x0304_C000, "CGameCtnMediaBlockTriangles3D", parent 0x0302_9000;
    /// CGameCtnMediaBlockImage.
    MediaBlockImage(game::media::MediaBlockImage) = 0x030A_5000, "CGameCtnMediaBlockImage", parent 0x2406_A000;
    /// CGameCtnMediaBlockSound.
    MediaBlockSound(game::media::MediaBlockSound) = 0x030A_7000, "CGameCtnMediaBlockSound", parent 0x2406_A000;
    /// CGameCtnMediaBlockFxColors.
    MediaBlockFxColors(game::media::MediaBlockFxColors) = 0x0308_0000, "CGameCtnMediaBlockFxColors", parent 0x2402_D000;
    /// CGameCtnMediaBlockFxBlurDepth.
    MediaBlockFxBlurDepth(game::media::MediaBlockFxBlurDepth) = 0x0308_1000, "CGameCtnMediaBlockFxBlurDepth", parent 0x2402_D000;
    /// CGameCtnMediaBlockFxBloom.
    MediaBlockFxBloom(game::media::MediaBlockFxBloom) = 0x0308_3000, "CGameCtnMediaBlockFxBloom", parent 0x2402_D000;
    /// CGameCtnMediaBlockTrails.
    MediaBlockTrails(game::media::MediaBlockTrails) = 0x030A_9000, "CGameCtnMediaBlockTrails", parent 0x2406_A000;
    /// CGameCtnMediaBlockFxBlurMotion.
    MediaBlockFxBlurMotion(game::media::MediaBlockFxBlurMotion) = 0x0308_2000, "CGameCtnMediaBlockFxBlurMotion", parent 0x2406_5000;
    /// CGameCtnMediaBlockCameraPath.
    MediaBlockCameraPath(game::media::MediaBlockCameraPath) = 0x030A_1000, "CGameCtnMediaBlockCameraPath", parent 0x2406_9000;
    /// CGameCtnMediaBlockCameraEffectShake.
    MediaBlockCameraEffectShake(game::media::MediaBlockCameraEffectShake) = 0x030A_4000, "CGameCtnMediaBlockCameraEffectShake", parent 0x2408_4000;
    /// CControlEffectSimi.
    ControlEffectSimi(game::media::ControlEffectSimi) = 0x0701_0000, "CControlEffectSimi", parent 0x0700_5000;
    /// CFuncKeys.
    FuncKeys(game::media::FuncKeys) = 0x0500_2000, "CFuncKeys", parent 0x0501_0000;
    /// CFuncKeysReal.
    FuncKeysReal(game::media::FuncKeysReal) = 0x0501_A000, "CFuncKeysReal", parent 0x0500_2000;
    /// A ghost.
    Ghost(game::ghost::Ghost) = 0x0309_2000, "CGameCtnGhost", parent 0x0303_F000;
    /// The MediaTracker block that plays a ghost.
    MediaBlockGhost(game::ghost::MediaBlockGhost) = 0x030E_5000, "CGameCtnMediaBlockGhost", parent 0x2406_A000;
    /// A scene tree: a block model's structure.
    Tree(plug::tree::Tree) = 0x0904_F000, "CPlugTree", parent 0x0902_B000;
    /// A scene tree with levels of detail.
    TreeVisualMip(plug::tree::TreeVisualMip) = 0x0901_5000, "CPlugTreeVisualMip", parent 0x0904_F000;
    /// An indexed triangle mesh.
    VisualIndexedTriangles(plug::visual::VisualIndexedTriangles) = 0x0901_E000, "CPlugVisualIndexedTriangles", parent 0x0906_A000;
    /// A material.
    Material(plug::material::Material) = 0x0907_9000, "CPlugMaterial", parent 0x0902_B000;
    /// A material's textures and samplers.
    MaterialCustom(plug::material::MaterialCustom) = 0x0903_A000, "CPlugMaterialCustom", parent 0x0902_B000;
    /// A collision surface.
    Surface(plug::surface::Surface) = 0x0900_C000, "CPlugSurface", parent 0x0902_B000;
    /// A collision geometry.
    SurfaceGeom(plug::surface::SurfaceGeom) = 0x0900_F000, "CPlugSurfaceGeom", parent 0x0902_B000;
    /// A scene tree holding a light.
    TreeLight(plug::tree::TreeLight) = 0x0906_2000, "CPlugTreeLight", parent 0x0904_F000;
    /// A sprite mesh.
    VisualSprite(plug::visual::VisualSprite) = 0x0901_0000, "CPlugVisualSprite", parent 0x0902_C000;
    /// A mesh of 2D quads.
    VisualQuads2D(plug::visual::VisualQuads2D) = 0x0904_B000, "CPlugVisualQuads2D", parent 0x0904_A000;
    /// A function of a sub-visual sequence.
    FuncTreeSubVisualSequence(plug::func_tree::TreeSubVisualSequence) = 0x0503_1000, "CFuncTreeSubVisualSequence", parent 0x0501_C000;
    /// Keys of natural numbers.
    FuncKeysNatural(plug::func_tree::KeysNatural) = 0x0503_0000, "CFuncKeysNatural", parent 0x0500_2000;
    /// A solid.
    Solid(plug::solid::Solid) = 0x0900_5000, "CPlugSolid", parent 0x0902_B000;
    /// CPlugBitmap.
    TextureBitmap(plug::bitmap::TextureBitmap) = 0x09011000, "CPlugBitmap", parent 0x0902b000;
    /// CPlugBitmapApply.
    BitmapApply(plug::bitmap::BitmapApply) = 0x09012000, "CPlugBitmapApply", parent 0x09047000;
    /// CPlugBitmapRenderWater.
    BitmapRenderWater(plug::bitmap::BitmapRenderWater) = 0x09087000, "CPlugBitmapRenderWater", parent 0x09086000;
    /// CPlugBitmapRenderOverlay.
    BitmapRenderOverlay(plug::bitmap::BitmapRenderOverlay) = 0x0909e000, "CPlugBitmapRenderOverlay", parent 0x09086000;
    /// CPlugBitmapRenderLightOcc.
    BitmapRenderLightOcc(plug::bitmap::BitmapRenderLightOcc) = 0x0909f000, "CPlugBitmapRenderLightOcc", parent 0x09086000;
    /// CPlugBitmapRenderCamera.
    BitmapRenderCamera(plug::bitmap::BitmapRenderCamera) = 0x09089000, "CPlugBitmapRenderCamera", parent 0x09086000;
    /// CPlugBitmapRenderSub.
    BitmapRenderSub(plug::bitmap::BitmapRenderSub) = 0x09091000, "CPlugBitmapRenderSub", parent 0x09086000;
    /// CPlugBitmapRenderHemisphere.
    BitmapRenderHemisphere(plug::bitmap::BitmapRenderHemisphere) = 0x09058000, "CPlugBitmapRenderHemisphere", parent 0x09086000;
    /// CPlugBitmapRenderLightFromMap.
    BitmapRenderLightFromMap(plug::bitmap::BitmapRenderLightFromMap) = 0x09021000, "CPlugBitmapRenderLightFromMap", parent 0x09086000;
    /// CPlugShaderApply.
    ShaderApply(plug::shader::ShaderApply) = 0x09026000, "CPlugShaderApply", parent 0x09004000;
    /// CPlugShaderPass.
    ShaderPass(plug::shader::ShaderPass) = 0x09067000, "CPlugShaderPass", parent 0x0902b000;
    /// CFuncShaderLayerUV.
    FuncShaderLayerUv(plug::func_shader::FuncShaderLayerUv) = 0x05015000, "CFuncShaderLayerUV", parent 0x05011000;
    /// CFuncShaders.
    FuncShaders(plug::func_shader::FuncShaders) = 0x05014000, "CFuncShaders", parent 0x05011000;
    /// GxLightBall.
    LightBall(plug::light::LightBall) = 0x04002000, "GxLightBall", parent 0x04003000;
    /// GxLightSpot.
    LightSpot(plug::light::LightSpot) = 0x0400b000, "GxLightSpot", parent 0x04002000;
    /// GxLightFrustum.
    LightFrustum(plug::light::LightFrustum) = 0x0400a000, "GxLightFrustum", parent 0x04002000;
    /// GxLightDirectional.
    LightDirectional(plug::light::LightDirectional) = 0x04007000, "GxLightDirectional", parent 0x04006000;
    /// CPlugLight.
    Light(plug::light::Light) = 0x0901d000, "CPlugLight", parent 0x0902b000;
    /// CHmsLight.
    HmsLight(plug::light::HmsLight) = 0x0600c000, "CHmsLight", parent 0x0600b000;
    /// CSceneLight.
    SceneLight(plug::light::SceneLight) = 0x0a00b000, "CSceneLight", parent 0x0a009000;
    /// CHmsLightMapMood.
    HmsLightMapMood(plug::light::HmsLightMapMood) = 0x06023000, "CHmsLightMapMood", parent 0x01001000;
    /// CHmsLightMap.
    HmsLightMap(plug::light::HmsLightMap) = 0x06021000, "CHmsLightMap", parent 0x01001000;
    /// CPlugFileGen overrides the chunk archive; GBX.NET makes no class write (also absent from writes.tsv).
    FileGenerator(plug::bitmap::FileGenerator) = 0x0902_F000, "CPlugFileGen", parent none;
}
