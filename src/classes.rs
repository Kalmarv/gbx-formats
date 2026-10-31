//! Every class whose nodes this crate reads, by class id: a node of any other class is refused
//! ([`crate::Error::UnknownClass`]).

use crate::archive::Archive;
use crate::error::{Error, Result};
use crate::game;
use crate::node::Chunks;

macro_rules! classes {
    ($($(#[$doc:meta])* $variant:ident($ty:path) = $id:literal, $name:literal;)*) => {
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

        /// Reads a node of class `class` (its id read at `at`).
        pub(crate) fn read_node(r: &mut Archive<'_, '_>, class: u32, at: usize) -> Result<Node> {
            match class {
                $($id => Ok(Node::$variant(Box::new(<$ty>::read(r)?))),)*
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
    Challenge(game::challenge::Challenge) = 0x0304_3000, "CGameCtnChallenge";
    /// The block models a map's editor offers.
    CollectorList(game::collector::CollectorList) = 0x0301_B000, "CGameCtnCollectorList";
    /// A map's tips, medal times and time limit.
    ChallengeParameters(game::collector::ChallengeParameters) = 0x0305_B000, "CGameCtnChallengeParameters";
    /// A block's skin.
    BlockSkin(game::block::BlockSkin) = 0x0305_9000, "CGameCtnBlockSkin";
    /// CGameCtnMediaClipGroup.
    MediaClipGroup(game::media::MediaClipGroup) = 0x0307a000, "CGameCtnMediaClipGroup";
    /// CGameCtnMediaClip.
    MediaClip(game::media::MediaClip) = 0x03079000, "CGameCtnMediaClip";
    /// CGameCtnMediaTrack.
    MediaTrack(game::media::MediaTrack) = 0x03078000, "CGameCtnMediaTrack";
    /// CGameCtnMediaBlockText.
    MediaBlockText(game::media::MediaBlockText) = 0x030a8000, "CGameCtnMediaBlockText";
    /// CGameCtnMediaBlockCameraCustom.
    MediaBlockCameraCustom(game::media::MediaBlockCameraCustom) = 0x030a2000, "CGameCtnMediaBlockCameraCustom";
    /// CGameCtnMediaBlockTransitionFade.
    MediaBlockTransitionFade(game::media::MediaBlockTransitionFade) = 0x030ab000, "CGameCtnMediaBlockTransitionFade";
    /// CGameCtnMediaBlockMusicEffect.
    MediaBlockMusicEffect(game::media::MediaBlockMusicEffect) = 0x030a6000, "CGameCtnMediaBlockMusicEffect";
    /// CGameCtnMediaBlockCameraGame.
    MediaBlockCameraGame(game::media::MediaBlockCameraGame) = 0x03084000, "CGameCtnMediaBlockCameraGame";
    /// CGameCtnMediaBlockTriangles.
    MediaBlockTriangles(game::media::MediaBlockTriangles) = 0x03029000, "CGameCtnMediaBlockTriangles";
    /// CGameCtnMediaBlockTriangles2D.
    MediaBlockTriangles2D(game::media::MediaBlockTriangles2D) = 0x0304b000, "CGameCtnMediaBlockTriangles2D";
    /// CGameCtnMediaBlockTriangles3D.
    MediaBlockTriangles3D(game::media::MediaBlockTriangles3D) = 0x0304c000, "CGameCtnMediaBlockTriangles3D";
    /// CGameCtnMediaBlockImage.
    MediaBlockImage(game::media::MediaBlockImage) = 0x030a5000, "CGameCtnMediaBlockImage";
    /// CGameCtnMediaBlockSound.
    MediaBlockSound(game::media::MediaBlockSound) = 0x030a7000, "CGameCtnMediaBlockSound";
    /// CGameCtnMediaBlockFxColors.
    MediaBlockFxColors(game::media::MediaBlockFxColors) = 0x03080000, "CGameCtnMediaBlockFxColors";
    /// CGameCtnMediaBlockFxBlurDepth.
    MediaBlockFxBlurDepth(game::media::MediaBlockFxBlurDepth) = 0x03081000, "CGameCtnMediaBlockFxBlurDepth";
    /// CGameCtnMediaBlockFxBloom.
    MediaBlockFxBloom(game::media::MediaBlockFxBloom) = 0x03083000, "CGameCtnMediaBlockFxBloom";
    /// CGameCtnMediaBlockFxBlurMotion.
    MediaBlockFxBlurMotion(game::media::MediaBlockFxBlurMotion) = 0x03082000, "CGameCtnMediaBlockFxBlurMotion";
    /// CGameCtnMediaBlockCameraPath.
    MediaBlockCameraPath(game::media::MediaBlockCameraPath) = 0x030a1000, "CGameCtnMediaBlockCameraPath";
    /// CGameCtnMediaBlockCameraEffectShake.
    MediaBlockCameraEffectShake(game::media::MediaBlockCameraEffectShake) = 0x030a4000, "CGameCtnMediaBlockCameraEffectShake";
    /// CControlEffectSimi.
    ControlEffectSimi(game::media::ControlEffectSimi) = 0x07010000, "CControlEffectSimi";
    /// CFuncKeys.
    FuncKeys(game::media::FuncKeys) = 0x05002000, "CFuncKeys";
    /// CFuncKeysReal.
    FuncKeysReal(game::media::FuncKeysReal) = 0x0501a000, "CFuncKeysReal";
    /// A ghost.
    Ghost(game::ghost::Ghost) = 0x0309_2000, "CGameCtnGhost";
    /// The MediaTracker block that plays a ghost.
    MediaBlockGhost(game::ghost::MediaBlockGhost) = 0x030E_5000, "CGameCtnMediaBlockGhost";
}
