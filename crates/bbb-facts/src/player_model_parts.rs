//! The `PlayerModelPart` bit layout carried by `ClientInformation`.
//!
//! Vanilla `PlayerModelPart` (`PlayerModelPart.java:8-14`) assigns each part a
//! bit and exposes `getMask() = 1 << bit`; the client packs those bits into the
//! `modelCustomisation` byte and the server reads them back through
//! `Player.isModelPartShown`. The bit values themselves are extracted into
//! [`crate::entity_synched_data`] as `VANILLA_AVATAR_MODEL_PART_*`, so this
//! module owns only the named projection over them.
//!
//! `bbb-client-renderer` keeps its own copy of this projection because it deliberately
//! does not depend on `bbb-facts` or `bbb-protocol` at build time (the protocol
//! crate is a dev-dependency there). The two stay pinned together by a guard test in
//! `bbb-client-renderer` that compares its constants against this module, so the
//! layering boundary does not become a second source of truth.

use crate::entity_synched_data::{
    VANILLA_AVATAR_MODEL_PART_CAPE, VANILLA_AVATAR_MODEL_PART_HAT,
    VANILLA_AVATAR_MODEL_PART_JACKET, VANILLA_AVATAR_MODEL_PART_LEFT_PANTS_LEG,
    VANILLA_AVATAR_MODEL_PART_LEFT_SLEEVE, VANILLA_AVATAR_MODEL_PART_RIGHT_PANTS_LEG,
    VANILLA_AVATAR_MODEL_PART_RIGHT_SLEEVE,
};

/// Which optional player model parts the client asked to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerModelPartVisibility {
    pub cape: bool,
    pub jacket: bool,
    pub left_sleeve: bool,
    pub right_sleeve: bool,
    pub left_pants: bool,
    pub right_pants: bool,
    pub hat: bool,
}

impl PlayerModelPartVisibility {
    pub const CAPE_MASK: u8 = VANILLA_AVATAR_MODEL_PART_CAPE as u8;
    pub const JACKET_MASK: u8 = VANILLA_AVATAR_MODEL_PART_JACKET as u8;
    pub const LEFT_SLEEVE_MASK: u8 = VANILLA_AVATAR_MODEL_PART_LEFT_SLEEVE as u8;
    pub const RIGHT_SLEEVE_MASK: u8 = VANILLA_AVATAR_MODEL_PART_RIGHT_SLEEVE as u8;
    pub const LEFT_PANTS_MASK: u8 = VANILLA_AVATAR_MODEL_PART_LEFT_PANTS_LEG as u8;
    pub const RIGHT_PANTS_MASK: u8 = VANILLA_AVATAR_MODEL_PART_RIGHT_PANTS_LEG as u8;
    pub const HAT_MASK: u8 = VANILLA_AVATAR_MODEL_PART_HAT as u8;
    pub const ALL_MASK: u8 = Self::CAPE_MASK
        | Self::JACKET_MASK
        | Self::LEFT_SLEEVE_MASK
        | Self::RIGHT_SLEEVE_MASK
        | Self::LEFT_PANTS_MASK
        | Self::RIGHT_PANTS_MASK
        | Self::HAT_MASK;

    pub const fn from_vanilla_mask(mask: u8) -> Self {
        Self {
            cape: (mask & Self::CAPE_MASK) == Self::CAPE_MASK,
            jacket: (mask & Self::JACKET_MASK) == Self::JACKET_MASK,
            left_sleeve: (mask & Self::LEFT_SLEEVE_MASK) == Self::LEFT_SLEEVE_MASK,
            right_sleeve: (mask & Self::RIGHT_SLEEVE_MASK) == Self::RIGHT_SLEEVE_MASK,
            left_pants: (mask & Self::LEFT_PANTS_MASK) == Self::LEFT_PANTS_MASK,
            right_pants: (mask & Self::RIGHT_PANTS_MASK) == Self::RIGHT_PANTS_MASK,
            hat: (mask & Self::HAT_MASK) == Self::HAT_MASK,
        }
    }

    pub const fn vanilla_mask(self) -> u8 {
        (if self.cape { Self::CAPE_MASK } else { 0 })
            | (if self.jacket { Self::JACKET_MASK } else { 0 })
            | (if self.left_sleeve {
                Self::LEFT_SLEEVE_MASK
            } else {
                0
            })
            | (if self.right_sleeve {
                Self::RIGHT_SLEEVE_MASK
            } else {
                0
            })
            | (if self.left_pants {
                Self::LEFT_PANTS_MASK
            } else {
                0
            })
            | (if self.right_pants {
                Self::RIGHT_PANTS_MASK
            } else {
                0
            })
            | (if self.hat { Self::HAT_MASK } else { 0 })
    }
}

pub const PLAYER_MODEL_PARTS_ALL_VISIBLE: PlayerModelPartVisibility =
    PlayerModelPartVisibility::from_vanilla_mask(PlayerModelPartVisibility::ALL_MASK);
pub const PLAYER_MODEL_PARTS_ALL_HIDDEN: PlayerModelPartVisibility =
    PlayerModelPartVisibility::from_vanilla_mask(0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_match_the_vanilla_bit_assignment() {
        // `PlayerModelPart.java:8-14` declares the parts in bit order with
        // `mask = 1 << bit`, so the whole byte is a dense 0..=6 range.
        assert_eq!(PlayerModelPartVisibility::HAT_MASK, 1 << 6);
        assert_eq!(PlayerModelPartVisibility::ALL_MASK, 0x7f);
    }

    #[test]
    fn mask_round_trips_through_the_projection() {
        for mask in 0..=PlayerModelPartVisibility::ALL_MASK {
            let parts = PlayerModelPartVisibility::from_vanilla_mask(mask);
            assert_eq!(parts.vanilla_mask(), mask, "mask {mask:#04x}");
        }
        // Bits outside the declared range are not part of the projection.
        assert_eq!(
            PlayerModelPartVisibility::from_vanilla_mask(0xff).vanilla_mask(),
            PlayerModelPartVisibility::ALL_MASK
        );
    }
}
