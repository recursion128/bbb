//! Living dimensions and eye height at the current pose.

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_DYING_ID, VANILLA_ENTITY_POSE_LONG_JUMPING_ID,
    VANILLA_ENTITY_POSE_SITTING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_types::{
    self as entity_types, vanilla_entity_type_dimensions, vanilla_entity_type_eye_height,
    vanilla_entity_type_vehicle_attachment, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
};

use crate::entity::{Entity, EntityData};

/// `LivingEntity.getAgeScale` (`LivingEntity.java:532-534`) and every override
/// of it (`Dolphin.java:102-104`, `HappyGhast.java:252-254`,
/// `Turtle.java:214-216`, `Goat.java:158-160`, `Armadillo.java:176-178`,
/// `Camel.java:508-510`).
fn baby_age_scale(entity_type_id: i32) -> f32 {
    match entity_type_id {
        entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => 0.2375,
        entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID => 0.3,
        entity_types::VANILLA_ENTITY_TYPE_GOAT_ID => 0.55,
        entity_types::VANILLA_ENTITY_TYPE_ARMADILLO_ID
        | entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID => 0.6,
        entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID => 0.65,
        _ => 0.5,
    }
}

/// The `getDefaultDimensions` override each baby type replaces
/// `LivingEntity.java:3667-3669` with. A factor multiplies the type's own box,
/// which is what `EntityDimensions.scale` does (`EntityDimensions.java:25-39`);
/// a pair is the absolute box the override names instead.
///
/// `AbstractCow`, `Squid` and `Llama` scale the base type's box rather than the
/// subtype's, and each pair declares the same box, so the factor stands for
/// every subtype (`EntityType.java:349-350`, `:706-708`, `:960-961`,
/// `:545-546`, `:653-656`, `:997-1000`).
fn baby_dimensions(entity_type_id: i32) -> Option<BabyDimensions> {
    let [_, height] = vanilla_entity_type_dimensions(entity_type_id)?;
    Some(match entity_type_id {
        // `AbstractCow.java:32`, `:102-104`.
        entity_types::VANILLA_ENTITY_TYPE_COW_ID
        | entity_types::VANILLA_ENTITY_TYPE_MOOSHROOM_ID => {
            BabyDimensions::Factor(0.5, Some(0.665), None)
        }
        // `Chicken.java:56`, `:104-106`.
        entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID => BabyDimensions::Box(0.3, 0.4, 0.28, 0.0),
        // `Axolotl.java:110`, `:569-571`.
        entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID => BabyDimensions::Box(0.5, 0.25, 0.2, 0.0),
        // `Fox.java:130`, `:377-379`.
        entity_types::VANILLA_ENTITY_TYPE_FOX_ID => BabyDimensions::Factor(0.6, Some(0.2975), None),
        // `Rabbit.java:81`, `:133-135`.
        entity_types::VANILLA_ENTITY_TYPE_RABBIT_ID => BabyDimensions::Box(0.24, 0.4, 0.39, 0.0),
        // `Squid.java:49`, `:235-237`.
        entity_types::VANILLA_ENTITY_TYPE_SQUID_ID
        | entity_types::VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => {
            BabyDimensions::Box(0.5, 0.63, 0.37, 0.0)
        }
        // `Turtle.java:69-72`, `:283-285`.
        entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID => {
            BabyDimensions::Factor(0.3, None, Some([0.0, height * 0.3, -0.25 * 0.3]))
        }
        // `Panda.java:82-85`, `:694-696`.
        entity_types::VANILLA_ENTITY_TYPE_PANDA_ID => {
            BabyDimensions::Factor(0.5, None, Some([0.0, 0.40625, 0.0]))
        }
        // `Horse.java:41-44`, `:254-256`.
        entity_types::VANILLA_ENTITY_TYPE_HORSE_ID => {
            BabyDimensions::Factor(0.7, None, Some([0.0, (height - 0.125) * 0.7, 0.0]))
        }
        // `SkeletonHorse.java:35-38`, `:121-123`.
        entity_types::VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => {
            BabyDimensions::Factor(0.7, None, Some([0.0, (height - 0.25) * 0.7, 0.0]))
        }
        // `ZombieHorse.java:43-46`, `:195-197`.
        entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => {
            BabyDimensions::Factor(0.7, None, Some([0.0, (height - 0.25) * 0.7, 0.0]))
        }
        // `Llama.java:71-74`, `:444-446`.
        entity_types::VANILLA_ENTITY_TYPE_LLAMA_ID
        | entity_types::VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => {
            BabyDimensions::Factor(0.5, None, Some([0.0, (height - 0.25) * 0.5, -0.3 * 0.5]))
        }
        // `AbstractChestedHorse.java:37-41`, `:68-70`.
        entity_types::VANILLA_ENTITY_TYPE_DONKEY_ID | entity_types::VANILLA_ENTITY_TYPE_MULE_ID => {
            BabyDimensions::Factor(
                0.5,
                None,
                Some([0.0, (height + 0.03125) * 0.5, -0.3125 * 0.5]),
            )
        }
        // `Villager.java:115`, `:365-367`.
        entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID => BabyDimensions::Box(0.49, 0.99, 0.63, 0.0),
        // Zombie.java:89-91; Drowned.java:69-71.
        entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID
        | entity_types::VANILLA_ENTITY_TYPE_DROWNED_ID => {
            BabyDimensions::Box(0.49, 0.99, 0.775, 0.1875)
        }
        // Husk.java:31-33; ZombieVillager.java:78-80.
        entity_types::VANILLA_ENTITY_TYPE_HUSK_ID => BabyDimensions::Box(0.49, 0.99, 0.825, 0.1875),
        entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => {
            BabyDimensions::Box(0.49, 0.99, 0.67, 0.125)
        }
        // ZombifiedPiglin.java:49-51; Piglin.java:77-79.
        entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
        | entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID => {
            BabyDimensions::Box(0.49, 0.99, 0.78, 0.1875)
        }
        _ => return None,
    })
}

#[derive(Clone, Copy)]
enum BabyDimensions {
    // The optional replacement passenger point already includes the baby factor.
    Factor(f32, Option<f32>, Option<[f32; 3]>),
    // Fresh boxes default their passenger point to height; the last field is vehicle Y.
    Box(f32, f32, f32, f32),
}

/// `Shulker.sanitizeScale` (`Shulker.java:540-542`), `HappyGhast.sanitizeScale`
/// (`HappyGhast.java:158-160`), and `EnderDragon.sanitizeScale` (`EnderDragon.java:865-867`).
pub(crate) fn sanitize_scale(entity_type_id: i32, scale: f32) -> f32 {
    match entity_type_id {
        entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID => {
            crate::entity::shulker::sanitize_scale(scale)
        }
        entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => scale.min(1.0),
        entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => 1.0,
        _ => scale,
    }
}

/// `Avatar.POSES` (`Avatar.java:17-37,65-68`) vehicle attachment point:
/// only `STANDING` and `CROUCHING` (and other default poses) carry `(0, 0.6, 0) * scale`.
pub(crate) fn avatar_vehicle_attachment(pose_id: i32, scale: f32) -> [f32; 3] {
    if matches!(
        pose_id,
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID
            | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_FALL_FLYING_ID
            | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID
            | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SPIN_ATTACK_ID
            | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_DYING_ID
    ) {
        [0.0, 0.0, 0.0]
    } else {
        [0.0, 0.6 * scale, 0.0]
    }
}

fn size_factor(living: &crate::entity::GeneralLivingData) -> f32 {
    if let Some(slime) = living.slime_retained() {
        slime.size() as f32
    } else if let Some(phantom) = living.phantom_retained() {
        1.0_f32 + 0.15_f32 * phantom.size() as f32
    } else if let Some(crate::entity::fish::FishLeafState::Salmon(variant)) = living.fish_leaf() {
        match variant {
            crate::entity::fish::SalmonVariant::Small => 0.5,
            crate::entity::fish::SalmonVariant::Medium => 1.0,
            crate::entity::fish::SalmonVariant::Large => 1.5,
        }
    } else if let Some(crate::entity::fish::FishLeafState::Pufferfish(state)) = living.fish_leaf() {
        // Pufferfish.java:177-187: dimensions read the retained PuffState.
        match state {
            0 => 0.5,
            1 => 0.7,
            _ => 1.0,
        }
    } else {
        1.0
    }
}

// LivingEntity.java:3663-3669; Slime.java:346-348; Phantom.java:201-205; Avatar.java:17-37,65-68.
pub(crate) fn geometry(data: &EntityData, entity_type_id: i32, pose_id: i32) -> Option<[f32; 3]> {
    let EntityData::GeneralLiving(living) = data else {
        return None;
    };
    let raw_scale = living.living().attribute_value(VANILLA_ATTRIBUTE_SCALE_ID) as f32;
    let scale = sanitize_scale(entity_type_id, raw_scale);
    if entity_type_id == entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID {
        return Some(match pose_id {
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID => [0.2, 0.2, 0.2],
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_DYING_ID => [0.2, 0.2, 1.62],
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID => {
                [0.6 * scale, 1.5 * scale, 1.27 * scale]
            }
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_FALL_FLYING_ID
            | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID
            | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SPIN_ATTACK_ID => {
                [0.6 * scale, 0.6 * scale, 0.4 * scale]
            }
            _ => [0.6 * scale, 1.8 * scale, 1.62 * scale],
        });
    }
    let [width, height, eye_height] = if let Some(plugin) = living.plugin_mob() {
        [plugin.width, plugin.height, plugin.eye_height]
    } else {
        let [width, height] = vanilla_entity_type_dimensions(entity_type_id)?;
        // Warden.java:512-514; EntityDimensions.java:29-39,45-46.
        if crate::entity::warden::is_digging_or_emerging(entity_type_id, pose_id) {
            return Some([width, 1.0, 0.85]);
        }
        let eye_height = vanilla_entity_type_eye_height(entity_type_id).unwrap_or(height * 0.85);
        [width, height, eye_height]
    };
    // Camel.java:84,160-162; Sniffer.java:73-75,122-124.
    let sitting_camel = matches!(
        entity_type_id,
        entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID
            | entity_types::VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
    ) && pose_id == VANILLA_ENTITY_POSE_SITTING_ID;
    let [width, height, eye_height] = if entity_type_id == entity_types::VANILLA_ENTITY_TYPE_GOAT_ID
        && pose_id == VANILLA_ENTITY_POSE_LONG_JUMPING_ID
    {
        // Goat.java:55,245-246; EntityDimensions.java:7-12,29-35.
        [0.9 * 0.7, 1.3 * 0.7, 1.3 * 0.85 * 0.7]
    } else if sitting_camel {
        [width, height - 1.43, 0.845]
    } else if living
        .sniffer_state()
        .is_some_and(|state| state.is_digging())
    {
        [width, height - 0.4, 0.81]
    } else {
        [width, height, eye_height]
    };
    // `CamelHusk.isBaby` is fixed false (`CamelHusk.java:76-78`).
    let baby = entity_type_id != VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID && living.is_baby();
    let [width, height, eye_height] = match baby.then(|| baby_dimensions(entity_type_id)).flatten()
    {
        Some(BabyDimensions::Box(width, height, eye, _)) => [width, height, eye],
        Some(BabyDimensions::Factor(factor, eye, _)) => [
            width * factor,
            height * factor,
            eye.unwrap_or(eye_height * factor),
        ],
        None if baby => {
            let factor = baby_age_scale(entity_type_id);
            [width * factor, height * factor, eye_height * factor]
        }
        None => [width, height, eye_height],
    };
    let factor = size_factor(living);
    let width = width * factor;
    let height = height * factor;
    let eye_height = eye_height * factor;
    Some([width * scale, height * scale, eye_height * scale])
}

pub(crate) fn generated_passenger_scale(entity: &Entity) -> Option<f32> {
    let id = entity.entity_type_id();
    let base_height = match entity.data() {
        EntityData::GeneralLiving(living) if living.plugin_mob().is_some() => {
            living.plugin_mob()?.height
        }
        _ => vanilla_entity_type_dimensions(id)?[1],
    };
    if base_height <= 0.0 {
        return Some(1.0);
    }
    let [_, height, _] = geometry(entity.data(), id, entity.pose_id())?;
    Some(height / base_height)
}

fn baby_attachment_dimensions(entity: &Entity) -> Option<(BabyDimensions, f32)> {
    let EntityData::GeneralLiving(living) = entity.data() else {
        return None;
    };
    if !living.is_baby()
        || !matches!(
            entity.pose_id(),
            VANILLA_ENTITY_POSE_STANDING_ID | VANILLA_ENTITY_POSE_DYING_ID
        )
    {
        return None;
    }
    let raw_scale = living.living().attribute_value(VANILLA_ATTRIBUTE_SCALE_ID) as f32;
    Some((
        baby_dimensions(entity.entity_type_id())?,
        sanitize_scale(entity.entity_type_id(), raw_scale),
    ))
}

pub(crate) fn baby_passenger_attachment(entity: &Entity) -> Option<[f32; 3]> {
    let (dimensions, scale) = baby_attachment_dimensions(entity)?;
    let point = match dimensions {
        BabyDimensions::Box(_, height, _, _) => [0.0, height, 0.0],
        BabyDimensions::Factor(_, _, point) => point?,
    };
    Some(point.map(|coordinate| coordinate * scale))
}

pub(crate) fn baby_vehicle_attachment(entity: &Entity) -> Option<[f32; 3]> {
    let (dimensions, scale) = baby_attachment_dimensions(entity)?;
    let point = match dimensions {
        BabyDimensions::Box(_, _, _, vehicle_y) => [0.0, vehicle_y, 0.0],
        // Replacing attachments resets unspecified kinds to their fresh-box defaults.
        BabyDimensions::Factor(_, _, Some(_)) => [0.0; 3],
        BabyDimensions::Factor(factor, _, None) => {
            vanilla_entity_type_vehicle_attachment(entity.entity_type_id())?
                .map(|coordinate| coordinate * factor)
        }
    };
    Some(point.map(|coordinate| coordinate * scale))
}

/// `Entity.getVehicleAttachmentPoint` (`Entity.java:2319-2333`; `LivingEntity.java:3663-3669`;
/// `EntityDimensions.java:29-39`) scaled by sanitized `scale`, age/size factor, and pose.
pub(crate) fn vehicle_attachment(entity: &Entity) -> Option<[f32; 3]> {
    let EntityData::GeneralLiving(living) = entity.data() else {
        return vanilla_entity_type_vehicle_attachment(entity.entity_type_id());
    };
    let id = entity.entity_type_id();
    let raw_scale = living.living().attribute_value(VANILLA_ATTRIBUTE_SCALE_ID) as f32;
    let scale = sanitize_scale(id, raw_scale);
    if id == entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID {
        return Some(avatar_vehicle_attachment(entity.pose_id(), scale));
    }
    if entity.pose_id() == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID {
        return Some([0.0, 0.0, 0.0]);
    }
    if let Some(point) = baby_vehicle_attachment(entity) {
        return Some(point);
    }
    let base = match vanilla_entity_type_vehicle_attachment(id) {
        Some(base) => base,
        None if living.plugin_mob().is_some() => [0.0, 0.0, 0.0],
        None => return None,
    };
    let baby = id != VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID && living.is_baby();
    let age_factor = if baby { baby_age_scale(id) } else { 1.0 };
    let factor = age_factor * size_factor(living) * scale;
    Some(base.map(|coordinate| coordinate * factor))
}

/// `Entity.refreshDimensions` (`Entity.java:3306-3322`), before collision relocation.
pub(crate) fn refresh(entity: &mut Entity) {
    let Some([width, height, eye_height]) =
        geometry(entity.data(), entity.entity_type_id(), entity.pose_id())
    else {
        return;
    };
    entity.set_dimensions_and_eye(width, height, eye_height);
}

#[cfg(test)]
mod tests;
