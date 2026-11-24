use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FixedLevelEventSound {
    pub(super) event_id: &'static str,
    pub(super) source: &'static str,
    pub(super) volume: f32,
    pub(super) pitch: f32,
}

pub(super) fn fixed_level_event_sound(event_type: i32) -> Option<FixedLevelEventSound> {
    let sound = match event_type {
        1000 => FixedLevelEventSound {
            event_id: "minecraft:block.dispenser.dispense",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1001 => FixedLevelEventSound {
            event_id: "minecraft:block.dispenser.fail",
            source: "block",
            volume: 1.0,
            pitch: 1.2,
        },
        1002 => FixedLevelEventSound {
            event_id: "minecraft:block.dispenser.launch",
            source: "block",
            volume: 1.0,
            pitch: 1.2,
        },
        1004 => FixedLevelEventSound {
            event_id: "minecraft:entity.firework_rocket.shoot",
            source: "neutral",
            volume: 1.0,
            pitch: 1.2,
        },
        1033 => FixedLevelEventSound {
            event_id: "minecraft:block.chorus_flower.grow",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1034 => FixedLevelEventSound {
            event_id: "minecraft:block.chorus_flower.death",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1035 => FixedLevelEventSound {
            event_id: "minecraft:block.brewing_stand.brew",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1049 => FixedLevelEventSound {
            event_id: "minecraft:block.crafter.craft",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1050 => FixedLevelEventSound {
            event_id: "minecraft:block.crafter.fail",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1503 => FixedLevelEventSound {
            event_id: "minecraft:block.end_portal_frame.fill",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        1505 => FixedLevelEventSound {
            event_id: "minecraft:item.bone_meal.use",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        3003 => FixedLevelEventSound {
            event_id: "minecraft:item.honeycomb.wax_on",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        },
        _ => return None,
    };
    Some(sound)
}

pub(super) fn random_level_event_sound(
    event_type: i32,
    data: i32,
    next_float: &mut impl FnMut() -> f32,
) -> Option<FixedLevelEventSound> {
    let sound = match event_type {
        1009 if data == 0 => FixedLevelEventSound {
            event_id: "minecraft:block.fire.extinguish",
            source: "block",
            volume: 0.5,
            pitch: triangle_pitch(2.6, 0.8, next_float),
        },
        1009 if data == 1 => FixedLevelEventSound {
            event_id: "minecraft:entity.generic.extinguish_fire",
            source: "block",
            volume: 0.7,
            pitch: triangle_pitch(1.6, 0.4, next_float),
        },
        1015 => hostile_triangle("minecraft:entity.ghast.warn", 10.0, next_float),
        1016 => hostile_triangle("minecraft:entity.ghast.shoot", 10.0, next_float),
        1017 => hostile_triangle("minecraft:entity.ender_dragon.shoot", 10.0, next_float),
        1018 => hostile_triangle("minecraft:entity.blaze.shoot", 2.0, next_float),
        1019 => hostile_triangle(
            "minecraft:entity.zombie.attack_wooden_door",
            2.0,
            next_float,
        ),
        1020 => hostile_triangle("minecraft:entity.zombie.attack_iron_door", 2.0, next_float),
        1021 => hostile_triangle("minecraft:entity.zombie.break_wooden_door", 2.0, next_float),
        1022 => hostile_triangle("minecraft:entity.wither.break_block", 2.0, next_float),
        1024 => hostile_triangle("minecraft:entity.wither.shoot", 2.0, next_float),
        1025 => FixedLevelEventSound {
            event_id: "minecraft:entity.bat.takeoff",
            source: "neutral",
            volume: 0.05,
            pitch: triangle_pitch(1.0, 0.2, next_float),
        },
        1026 => hostile_triangle("minecraft:entity.zombie.infect", 2.0, next_float),
        1027 => hostile_triangle(
            "minecraft:entity.zombie_villager.converted",
            2.0,
            next_float,
        ),
        1029 => block_ranged("minecraft:block.anvil.destroy", 1.0, next_float),
        1030 => block_ranged("minecraft:block.anvil.use", 1.0, next_float),
        1031 => block_ranged("minecraft:block.anvil.land", 0.3, next_float),
        1039 => FixedLevelEventSound {
            event_id: "minecraft:entity.phantom.bite",
            source: "hostile",
            volume: 0.3,
            pitch: ranged_pitch(0.9, 0.1, next_float),
        },
        1040 => hostile_triangle(
            "minecraft:entity.zombie.converted_to_drowned",
            2.0,
            next_float,
        ),
        1041 => hostile_triangle("minecraft:entity.husk.converted_to_zombie", 2.0, next_float),
        1042 => block_ranged("minecraft:block.grindstone.use", 1.0, next_float),
        1043 => block_ranged("minecraft:item.book.page_turn", 1.0, next_float),
        1044 => block_ranged("minecraft:block.smithing_table.use", 1.0, next_float),
        1045 => block_ranged("minecraft:block.pointed_dripstone.land", 2.0, next_float),
        1046 => block_ranged(
            "minecraft:block.pointed_dripstone.drip_lava_into_cauldron",
            2.0,
            next_float,
        ),
        1047 => block_ranged(
            "minecraft:block.pointed_dripstone.drip_water_into_cauldron",
            2.0,
            next_float,
        ),
        1048 => hostile_triangle(
            "minecraft:entity.skeleton.converted_to_stray",
            2.0,
            next_float,
        ),
        1051 => FixedLevelEventSound {
            event_id: "minecraft:entity.wind_charge.throw",
            source: "block",
            volume: 0.5,
            pitch: 0.4 / (next_float().clamp(0.0, 1.0) * 0.4 + 0.8),
        },
        1501 => FixedLevelEventSound {
            event_id: "minecraft:block.lava.extinguish",
            source: "block",
            volume: 0.5,
            pitch: triangle_pitch(2.6, 0.8, next_float),
        },
        1502 => FixedLevelEventSound {
            event_id: "minecraft:block.redstone_torch.burnout",
            source: "block",
            volume: 0.5,
            pitch: triangle_pitch(2.6, 0.8, next_float),
        },
        2002 | 2007 => FixedLevelEventSound {
            event_id: "minecraft:entity.splash_potion.break",
            source: "neutral",
            volume: 1.0,
            pitch: ranged_pitch(0.9, 0.1, next_float),
        },
        2006 if data == 1 => FixedLevelEventSound {
            event_id: "minecraft:entity.dragon_fireball.explode",
            source: "hostile",
            volume: 1.0,
            pitch: ranged_pitch(0.9, 0.1, next_float),
        },
        3000 => FixedLevelEventSound {
            event_id: "minecraft:block.end_gateway.spawn",
            source: "block",
            volume: 10.0,
            pitch: triangle_pitch(1.0, 0.2, next_float) * 0.7,
        },
        3001 => FixedLevelEventSound {
            event_id: "minecraft:entity.ender_dragon.growl",
            source: "hostile",
            volume: 64.0,
            pitch: ranged_pitch(0.8, 0.3, next_float),
        },
        3006 => sculk_charge_sound(data, next_float)?,
        _ => return None,
    };
    Some(sound)
}

pub(super) fn random_distance_delayed_level_event_sound(
    event_type: i32,
    data: i32,
    next_float: &mut impl FnMut() -> f32,
) -> Option<FixedLevelEventSound> {
    let sound = match event_type {
        3012 => trial_spawner_triangle("minecraft:block.trial_spawner.spawn_mob", 1.0, next_float),
        3013 | 3019 => trial_spawner_triangle(
            "minecraft:block.trial_spawner.detect_player",
            1.0,
            next_float,
        ),
        3014 => trial_spawner_triangle("minecraft:block.trial_spawner.eject_item", 1.0, next_float),
        3020 => trial_spawner_triangle(
            "minecraft:block.trial_spawner.ominous_activate",
            if data == 0 { 0.3 } else { 1.0 },
            next_float,
        ),
        3021 => trial_spawner_triangle("minecraft:block.trial_spawner.spawn_item", 1.0, next_float),
        _ => return None,
    };
    Some(sound)
}

pub(super) fn local_level_event_sound(
    event_type: i32,
    next_float: &mut impl FnMut() -> f32,
) -> Option<FixedLevelEventSound> {
    let sound = match event_type {
        1032 => FixedLevelEventSound {
            event_id: "minecraft:block.portal.travel",
            source: "ambient",
            volume: 0.25,
            pitch: ranged_pitch(0.8, 0.4, next_float),
        },
        _ => return None,
    };
    Some(sound)
}

pub(super) fn global_level_event_sound_position(
    pos: crate::BlockPos,
    camera_position: ProtocolVec3d,
) -> ProtocolVec3d {
    let event_center = ProtocolVec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    };
    let delta = ProtocolVec3d {
        x: event_center.x - camera_position.x,
        y: event_center.y - camera_position.y,
        z: event_center.z - camera_position.z,
    };
    let distance = (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt();
    if distance < VANILLA_VEC3_NORMALIZE_EPSILON {
        return camera_position;
    }

    ProtocolVec3d {
        x: camera_position.x + delta.x / distance * GLOBAL_LEVEL_EVENT_SOUND_DISTANCE,
        y: camera_position.y + delta.y / distance * GLOBAL_LEVEL_EVENT_SOUND_DISTANCE,
        z: camera_position.z + delta.z / distance * GLOBAL_LEVEL_EVENT_SOUND_DISTANCE,
    }
}

pub(super) fn hostile_triangle(
    event_id: &'static str,
    volume: f32,
    next_float: &mut impl FnMut() -> f32,
) -> FixedLevelEventSound {
    FixedLevelEventSound {
        event_id,
        source: "hostile",
        volume,
        pitch: triangle_pitch(1.0, 0.2, next_float),
    }
}

pub(super) fn block_ranged(
    event_id: &'static str,
    volume: f32,
    next_float: &mut impl FnMut() -> f32,
) -> FixedLevelEventSound {
    FixedLevelEventSound {
        event_id,
        source: "block",
        volume,
        pitch: ranged_pitch(0.9, 0.1, next_float),
    }
}

pub(super) fn trial_spawner_triangle(
    event_id: &'static str,
    volume: f32,
    next_float: &mut impl FnMut() -> f32,
) -> FixedLevelEventSound {
    FixedLevelEventSound {
        event_id,
        source: "block",
        volume,
        pitch: triangle_pitch(1.0, 0.2, next_float),
    }
}

pub(super) fn sculk_charge_sound(
    data: i32,
    next_float: &mut impl FnMut() -> f32,
) -> Option<FixedLevelEventSound> {
    let count = data >> 6;
    if count <= 0 {
        return Some(FixedLevelEventSound {
            event_id: "minecraft:block.sculk.charge",
            source: "block",
            volume: 1.0,
            pitch: 1.0,
        });
    }

    let count = count as f32;
    if next_float().clamp(0.0, 1.0) >= 0.3 + count * 0.1 {
        return None;
    }

    Some(FixedLevelEventSound {
        event_id: "minecraft:block.sculk.charge",
        source: "block",
        volume: 0.15 + 0.02 * count * count * next_float().clamp(0.0, 1.0),
        pitch: 0.4 + 0.3 * count * next_float().clamp(0.0, 1.0),
    })
}

pub(super) fn triangle_pitch(mean: f32, spread: f32, next_float: &mut impl FnMut() -> f32) -> f32 {
    mean + (next_float().clamp(0.0, 1.0) - next_float().clamp(0.0, 1.0)) * spread
}

pub(super) fn horizontal_speed(delta: crate::EntityVec3) -> f32 {
    delta.x.hypot(delta.z) as f32
}

pub(super) fn sound_lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

pub(super) fn ranged_pitch(min: f32, range: f32, next_float: &mut impl FnMut() -> f32) -> f32 {
    next_float().clamp(0.0, 1.0) * range + min
}

pub(super) fn block_sound_state(
    pos: crate::BlockPos,
    sound: &str,
    volume: f32,
    pitch: f32,
    source: &str,
) -> SoundEventState {
    block_sound_state_with_distance_delay(pos, sound, volume, pitch, source, false)
}

pub(super) fn local_player_sound_state(
    position: ProtocolVec3d,
    sound: &str,
    volume: f32,
    pitch: f32,
    source: &str,
) -> SoundEventState {
    SoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(sound.to_string()),
            fixed_range: None,
        },
        source: source.to_string(),
        position,
        volume,
        pitch,
        seed: 0,
        distance_delay: false,
    }
}

pub(super) fn ui_sound_state(sound: &str, volume: f32, pitch: f32) -> LocalSoundEventState {
    LocalSoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(sound.to_string()),
            fixed_range: None,
        },
        source: SoundSource::Ui.as_str().to_string(),
        volume,
        pitch,
        // `SimpleSoundInstance.forUI` owns an unseeded random source rather
        // than advancing ClientLevel's shared sound seed.
        seed: 0,
    }
}

pub(super) fn block_sound_state_with_distance_delay(
    pos: crate::BlockPos,
    sound: &str,
    volume: f32,
    pitch: f32,
    source: &str,
    distance_delay: bool,
) -> SoundEventState {
    SoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(sound.to_string()),
            fixed_range: None,
        },
        source: source.to_string(),
        position: ProtocolVec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        },
        volume,
        pitch,
        seed: 0,
        distance_delay,
    }
}
