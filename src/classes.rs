//! Every class whose nodes this crate reads, by class id: a node of any other class is refused
//! ([`crate::Error::UnknownClass`]).
//!
//! Each class also names the id a pack file's reader writes as it starts one of its nodes (see [`crate::pak`]): its
//! parent class's id as the game knew it when TrackMania Nations Forever's packs were made, which for most classes is
//! the old id of the class it derives from (`CGameCtnMediaBlockText`, `0x030A8000`, derives from
//! `CGameCtnMediaBlock`, whose old id is `0x2406A000`).

use crate::archive::Archive;
use crate::error::{Error, Result};
use crate::game;
use crate::node::Chunks;
use crate::plug;

macro_rules! classes {
    ($($(#[$doc:meta])* $variant:ident($ty:path) = $id:literal, $name:literal, parent $parent:literal;)*) => {
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

        /// Reads a node of class `class` (its id read at `at`), starting with the dummy write a pack file's reader
        /// makes for it: the id of its parent class.
        pub(crate) fn read_node(r: &mut Archive<'_, '_>, class: u32, at: usize) -> Result<Node> {
            match class {
                $($id => {
                    r.dummy_write(u32::to_le_bytes($parent));
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
    /// A block.
    BlockInfoClassic(game::blockinfo::BlockInfoClassic) = 0x0305_1000, "CGameCtnBlockInfoClassic", parent 0x2400_5000;
    /// A block info for flat terrain.
    BlockInfoFlat(game::blockinfo::BlockInfoFlat) = 0x0304_F000, "CGameCtnBlockInfoFlat", parent 0x2400_5000;
    /// A block info for a rectangular asymmetric block.
    BlockInfoRectAsym(game::blockinfo::BlockInfoRectAsym) = 0x0305_6000, "CGameCtnBlockInfoRectAsym", parent 0x2400_5000;
    /// A road block info.
    BlockInfoRoad(game::blockinfo::BlockInfoRoad) = 0x0305_2000, "CGameCtnBlockInfoRoad", parent 0x2400_5000;
    /// A clip block info.
    BlockInfoClip(game::blockinfo::BlockInfoClip) = 0x0305_3000, "CGameCtnBlockInfoClip", parent 0x2400_5000;
    /// A frontier block info.
    BlockInfoFrontier(game::blockinfo::BlockInfoFrontier) = 0x0305_0000, "CGameCtnBlockInfoFrontier", parent 0x2400_5000;
    /// A pylon block info.
    BlockInfoPylon(game::blockinfo::BlockInfoPylon) = 0x0305_5000, "CGameCtnBlockInfoPylon", parent 0x2400_5000;
    /// A block unit.
    BlockUnitInfo(game::blockinfo::BlockUnitInfo) = 0x0303_6000, "CGameCtnBlockUnitInfo", parent 0x0100_1000;
    /// A mobil.
    Mobil(game::scene::Mobil) = 0x0A01_1000, "CSceneMobil", parent 0x0A00_5000;
    /// An item.
    HmsItem(game::scene::HmsItem) = 0x0600_3000, "CHmsItem", parent 0x0100_1000;
    /// A link from a mobil to a child object.
    ObjectLink(game::scene::ObjectLink) = 0x0A01_4000, "CSceneObjectLink", parent 0x0100_1000;
    /// A sound source in the scene.
    SceneSoundSource(game::scene::SceneSoundSource) = 0x0A00_E000, "CSceneSoundSource", parent 0x0A00_9000;
    /// A sound source.
    HmsSoundSource(game::scene::HmsSoundSource) = 0x0600_D000, "CHmsSoundSource", parent 0x0600_B000;
    /// A motion player.
    MotionPlayer(game::scene::MotionPlayer) = 0x0803_4000, "CMotionPlayer", parent 0x0800_1000;
    /// A motion command.
    MotionCmdBase(game::scene::MotionCmdBase) = 0x0802_9000, "CMotionCmdBase", parent 0x0100_5000;
    /// A shader track.
    MotionShader(game::scene::MotionShader) = 0x0802_B000, "CMotionShader", parent 0x0803_3000;
    /// A decoration.
    Decoration(game::decoration::Decoration) = 0x0303_8000, "CGameCtnDecoration", parent 0x2400_A000;
    /// The size of a decoration.
    DecorationSize(game::decoration::DecorationSize) = 0x0303_B000, "CGameCtnDecorationSize", parent 0x0100_1000;
    /// The mood of a decoration.
    DecorationMood(game::decoration::DecorationMood) = 0x0303_A000, "CGameCtnDecorationMood", parent 0x0100_1000;
    /// A terrain modifier of a decoration.
    DecorationTerrainModifier(game::decoration::DecorationTerrainModifier) = 0x0303_C000, "CGameCtnDecorationTerrainModifier", parent 0x0100_1000;
    /// The audio of a decoration.
    DecorationAudio(game::decoration::DecorationAudio) = 0x0303_9000, "CGameCtnDecorationAudio", parent 0x0100_1000;
    /// A tree of a decorator.
    DecoratorTree(game::decoration::DecoratorTree) = 0x090A_2000, "CPlugDecoratorTree", parent 0x0100_1000;
    /// A decorator solid.
    DecoratorSolid(game::decoration::DecoratorSolid) = 0x090A_3000, "CPlugDecoratorSolid", parent 0x0100_1000;
    /// A flat zone.
    ZoneFlat(game::collection::ZoneFlat) = 0x0305_D000, "CGameCtnZoneFlat", parent 0x2401_C000;
    /// A frontier zone.
    ZoneFrontier(game::collection::ZoneFrontier) = 0x0305_E000, "CGameCtnZoneFrontier", parent 0x2401_C000;
    /// A collection.
    Collection(game::collection::Collection) = 0x0303_3000, "CGameCtnCollection", parent 0x0100_1000;
    /// A solid.
    Solid(plug::solid::Solid) = 0x0900_5000, "CPlugSolid", parent 0x0902_B000;
}
