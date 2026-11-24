//! Immediate body hazards shared by stored entities and player sessions.

use bbb_protocol::block_types::{
    VANILLA_BLOCK_CACTUS_ID, VANILLA_BLOCK_CAMPFIRE_ID, VANILLA_BLOCK_CLOSED_EYEBLOSSOM_ID,
    VANILLA_BLOCK_COBWEB_ID, VANILLA_BLOCK_FROGSPAWN_ID, VANILLA_BLOCK_HONEY_BLOCK_ID,
    VANILLA_BLOCK_LILY_PAD_ID, VANILLA_BLOCK_OPEN_EYEBLOSSOM_ID, VANILLA_BLOCK_PITCHER_CROP_ID,
    VANILLA_BLOCK_SOUL_CAMPFIRE_ID, VANILLA_BLOCK_SWEET_BERRY_BUSH_ID,
    VANILLA_BLOCK_WITHER_ROSE_ID,
};
use bbb_protocol::packets::Vec3d;

use super::sweet_berry_bush;

/// The body facts the immediate contact rules read.
#[derive(Debug, Clone, Copy)]
pub struct ContactBody {
    pub entity_type_id: i32,
    pub living: bool,
    pub weaving: bool,
}

/// What one immediate inside-block contact asks of the body owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImmediateContact {
    Unhandled,
    Resolved,
    Stuck(Vec3d),
    Hurt(&'static str, f32),
    /// `SweetBerryBushBlock.entityInside`: slowed, then hurt by its own
    /// movement once the bush has grown.
    SweetBerry {
        hurts_when_moving: bool,
    },
    WitherRose(i32),
    Honey,
    BeeEyeblossom(i32),
    LilyPad,
    RavagerCrop,
    DestroyWithoutDrops,
}

// WebBlock.java:27-36.
pub fn cobweb_stuck_multiplier(weaving: bool) -> Vec3d {
    if weaving {
        Vec3d {
            x: 0.5,
            y: 0.25,
            z: 0.5,
        }
    } else {
        Vec3d {
            x: 0.25,
            y: f64::from(0.05_f32),
            z: 0.25,
        }
    }
}

pub fn contact(state: i32, body: ContactBody) -> Option<ImmediateContact> {
    let block = super::block_type_of(state)?;
    // CropBlock.java:154-162; PitcherCropBlock.java:120-126.
    if super::crop::is_crop(state) || block == VANILLA_BLOCK_PITCHER_CROP_ID {
        return Some(
            if body.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RAVAGER_ID {
                ImmediateContact::RavagerCrop
            } else {
                ImmediateContact::Resolved
            },
        );
    }
    Some(match block {
        // EyeblossomBlock.java:101-115.
        VANILLA_BLOCK_OPEN_EYEBLOSSOM_ID | VANILLA_BLOCK_CLOSED_EYEBLOSSOM_ID
            if body.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID =>
        {
            ImmediateContact::BeeEyeblossom(25)
        }
        // LilyPadBlock.java:34-39.
        VANILLA_BLOCK_LILY_PAD_ID => ImmediateContact::LilyPad,
        // FrogspawnBlock.java:94-99,114-115.
        VANILLA_BLOCK_FROGSPAWN_ID
            if body.entity_type_id
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID =>
        {
            ImmediateContact::DestroyWithoutDrops
        }
        VANILLA_BLOCK_FROGSPAWN_ID => ImmediateContact::Resolved,
        VANILLA_BLOCK_HONEY_BLOCK_ID => ImmediateContact::Honey,
        VANILLA_BLOCK_COBWEB_ID => {
            ImmediateContact::Stuck(cobweb_stuck_multiplier(body.living && body.weaving))
        }
        VANILLA_BLOCK_CAMPFIRE_ID | VANILLA_BLOCK_SOUL_CAMPFIRE_ID => {
            match super::campfire::inside_damage(state, body.living)? {
                Some(amount) => ImmediateContact::Hurt("minecraft:campfire", amount),
                None => ImmediateContact::Resolved,
            }
        }
        // CactusBlock.java:126-130.
        VANILLA_BLOCK_CACTUS_ID => ImmediateContact::Hurt("minecraft:cactus", 1.0),
        VANILLA_BLOCK_SWEET_BERRY_BUSH_ID
            if sweet_berry_bush::inside_applies(body.living, body.entity_type_id) =>
        {
            ImmediateContact::SweetBerry {
                hurts_when_moving: sweet_berry_bush::age(state)? != 0,
            }
        }
        // WitherRoseBlock.java:67-80.
        VANILLA_BLOCK_WITHER_ROSE_ID if body.living => ImmediateContact::WitherRose(40),
        VANILLA_BLOCK_OPEN_EYEBLOSSOM_ID
        | VANILLA_BLOCK_CLOSED_EYEBLOSSOM_ID
        | VANILLA_BLOCK_SWEET_BERRY_BUSH_ID
        | VANILLA_BLOCK_WITHER_ROSE_ID => ImmediateContact::Resolved,
        _ => ImmediateContact::Unhandled,
    })
}

#[cfg(test)]
mod tests;
