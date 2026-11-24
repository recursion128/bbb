use super::*;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;

// Entity.applyMovementEmissionAndPlaySound; Player.getMovementEmission.
pub(super) fn advance_local_player_movement_sounds(
    world: &mut WorldStore,
    pose: LocalPlayerPoseState,
    movement: EntityBlockEffectMovement,
) {
    world.local_player.movement_sound.tick_count = world
        .local_player
        .movement_sound
        .tick_count
        .saturating_add(1);
    let in_water_now = local_player_fluid_contact(world, pose).in_water();
    // Vanilla `Entity.updateInWaterStateAndDoFluidPushing` -> `doWaterSplashEffect`
    // (`Entity.java:1560-1608`): play splash sound + bubble/splash particles on
    // the !wasTouchingWater -> inWater rising edge.
    let was_in_water = world.local_player.movement_sound.was_touching_water;
    let initialized = world.local_player.movement_sound.water_state_initialized;
    world.local_player.movement_sound.was_touching_water = in_water_now;
    world.local_player.movement_sound.water_state_initialized = true;
    if !was_in_water
        && initialized
        && in_water_now
        && world.local_player_id.is_some()
        && !world.local_player_is_spectator()
        && world.local_player_vehicle_id().is_none()
    {
        play_local_player_water_splash_effect(world, pose);
    }

    if world.local_player_id.is_none()
        || world.local_player_is_spectator()
        || world.local_player_vehicle_id().is_some()
        || world
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.flying)
        || pose.on_ground && world.local_player.shift_key_down
    {
        return;
    }
    let position = EntityVec3 {
        x: pose.position.x,
        y: pose.position.y,
        z: pose.position.z,
    };
    let supporting_pos = pose
        .on_ground
        .then(|| world.local_player_supporting_block(pose, movement.to.y < movement.from.y))
        .flatten();
    let on_pos = supporting_pos.unwrap_or(BlockPos {
        x: local_player_block_floor(position.x),
        y: local_player_block_floor(position.y - 1.0e-5),
        z: local_player_block_floor(position.z),
    });
    let support = world.block_view(on_pos);
    let delta = [
        movement.to.x - movement.from.x,
        movement.to.y - movement.from.y,
        movement.to.z - movement.from.z,
    ];
    let climbing = support.is_some_and(|block| movement_sound_block_is_climbable(world, block));
    let squared_distance = delta[0] * delta[0]
        + delta[2] * delta[2]
        + if climbing { delta[1] * delta[1] } else { 0.0 };
    world.local_player.movement_sound.distance +=
        (squared_distance.sqrt() * f64::from(0.6f32)) as f32;
    let distance = world.local_player.movement_sound.distance;
    if distance <= world.local_player.movement_sound.next_step {
        return;
    }
    let Some(support) = support.filter(|block| !movement_sound_block_is_air(*block)) else {
        return;
    };
    let effect_pos = world.legacy_on_pos(supporting_pos, position);
    let effect = world.block_view(effect_pos);
    let can_step = |block: BlockView| {
        !movement_sound_block_is_air(block)
            && !world.local_player.swimming
            && (pose.on_ground
                || movement_sound_block_is_climbable(world, block)
                || pose.sneaking && delta[1] == 0.0)
    };
    let sound_from_effect = effect.is_some_and(can_step);
    let in_water = local_player_fluid_contact(world, pose).in_water();
    if !sound_from_effect && !can_step(support) && !in_water {
        return;
    }
    world.local_player.movement_sound.next_step = distance.floor() + 1.0;
    if sound_from_effect {
        play_local_player_step_sound(world, pose, effect.unwrap(), in_water);
    } else if in_water {
        play_local_player_swim_sound(world, pose);
    }
}

fn play_local_player_step_sound(
    world: &mut WorldStore,
    pose: LocalPlayerPoseState,
    effect: BlockView,
    in_water: bool,
) {
    // Player.playStepSound selects the surface above the legacy effect block.
    if in_water {
        play_local_player_swim_sound(world, pose);
        append_local_player_block_step(world, pose, effect, true);
        return;
    }
    let above = world.block_view(BlockPos {
        y: effect.pos.y + 1,
        ..effect.pos
    });
    if let Some(above) = above {
        let combined =
            movement_sound_block_is_in_tag(world, above, "minecraft:combination_step_sound_blocks");
        if combined
            || movement_sound_block_is_in_tag(world, above, "minecraft:inside_step_sound_blocks")
        {
            append_local_player_block_step(world, pose, above, false);
            if combined {
                append_local_player_block_step(world, pose, effect, true);
            }
            maybe_play_local_player_crystal_step_sound(world, pose, effect);
            return;
        }
    }
    append_local_player_block_step(world, pose, effect, false);
    maybe_play_local_player_crystal_step_sound(world, pose, effect);
}

fn maybe_play_local_player_crystal_step_sound(
    world: &mut WorldStore,
    pose: LocalPlayerPoseState,
    effect: BlockView,
) {
    // Vanilla `Entity.shouldPlayAmethystStepSound` / `playAmethystStepSound`
    // (`Entity.java:1380-1391`).
    if !movement_sound_block_is_in_tag(world, effect, "minecraft:crystal_sound_blocks") {
        return;
    }
    let tick_count = world.local_player.movement_sound.tick_count;
    let last_tick = world.local_player.movement_sound.last_crystal_sound_tick;
    if tick_count < last_tick.saturating_add(20) {
        return;
    }
    let Some(id) = world.local_player_id else {
        return;
    };
    let elapsed = (tick_count - last_tick).clamp(0, 10_000) as i32;
    let intensity = (world.local_player.movement_sound.crystal_sound_intensity
        * 0.997_f32.powi(elapsed)
        + 0.07)
        .min(1.0);
    world.local_player.movement_sound.crystal_sound_intensity = intensity;
    world.local_player.movement_sound.last_crystal_sound_tick = tick_count;
    let Some(rand_float) = world
        .entities
        .with_entity_random_mut(id, |random| random.next_float())
    else {
        return;
    };
    let pitch = 0.5 + intensity * rand_float * 1.2;
    let volume = 0.1 + intensity * 1.2;
    append_local_player_movement_sound(
        world,
        pose,
        EntityClientMovementSoundKind::PlayerCrystalChime { volume, pitch },
    );
}

fn play_local_player_water_splash_effect(world: &mut WorldStore, pose: LocalPlayerPoseState) {
    let Some(id) = world.local_player_id else {
        return;
    };
    let delta = pose.delta_movement;
    let speed = (((delta.x * delta.x + delta.z * delta.z) * 0.2 + delta.y * delta.y).sqrt() as f32
        * 0.2)
        .min(1.0);
    let width = world.local_player_dimensions(pose).width;
    let particle_count = (1.0 + width * 20.0).ceil().max(1.0) as usize;
    let yt = pose.position.y.floor() + 1.0;
    let Some((pitch, bubbles, splashes)) = world.entities.with_entity_random_mut(id, |random| {
        let pitch = 1.0 + (random.next_float() - random.next_float()) * 0.4;
        let mut bubbles = Vec::with_capacity(particle_count);
        for _ in 0..particle_count {
            let xo = (random.next_double() * 2.0 - 1.0) * width;
            let zo = (random.next_double() * 2.0 - 1.0) * width;
            let y_jitter = random.next_double() * 0.2;
            bubbles.push((xo, zo, y_jitter));
        }
        let mut splashes = Vec::with_capacity(particle_count);
        for _ in 0..particle_count {
            let xo = (random.next_double() * 2.0 - 1.0) * width;
            let zo = (random.next_double() * 2.0 - 1.0) * width;
            splashes.push((xo, zo));
        }
        (pitch, bubbles, splashes)
    }) else {
        return;
    };
    append_local_player_movement_sound(
        world,
        pose,
        EntityClientMovementSoundKind::PlayerSplash {
            high_speed: speed >= 0.25,
            volume: speed,
            pitch,
        },
    );
    for (xo, zo, y_jitter) in bubbles {
        world.push_pending_client_particle_spawn(crate::particles::ParticleSpawn::new(
            crate::particles::ParticleKind::BUBBLE,
            [pose.position.x + xo, yt, pose.position.z + zo],
            [delta.x, delta.y - y_jitter, delta.z],
        ));
    }
    for (xo, zo) in splashes {
        world.push_pending_client_particle_spawn(crate::particles::ParticleSpawn::new(
            crate::particles::ParticleKind::SPLASH,
            [pose.position.x + xo, yt, pose.position.z + zo],
            [delta.x, delta.y, delta.z],
        ));
    }
}

fn append_local_player_block_step(
    world: &mut WorldStore,
    pose: LocalPlayerPoseState,
    block: BlockView,
    muffled: bool,
) {
    append_local_player_movement_sound(
        world,
        pose,
        EntityClientMovementSoundKind::PlayerBlockStep {
            block_state_id: block.block_state_id,
            muffled,
        },
    );
}

fn play_local_player_swim_sound(world: &mut WorldStore, pose: LocalPlayerPoseState) {
    let Some(id) = world.local_player_id else {
        return;
    };
    let delta = pose.delta_movement;
    let volume =
        (((delta.x * delta.x + delta.z * delta.z) * 0.2 + delta.y * delta.y).sqrt() as f32 * 0.35)
            .min(1.0);
    let Some(pitch) = world.entities.with_entity_random_mut(id, |random| {
        1.0 + (random.next_float() - random.next_float()) * 0.4
    }) else {
        return;
    };
    append_local_player_movement_sound(
        world,
        pose,
        EntityClientMovementSoundKind::PlayerSwim { volume, pitch },
    );
}

fn movement_sound_block_is_air(block: BlockView) -> bool {
    block
        .block_name
        .is_some_and(crate::terrain::block_name_is_air)
}

fn movement_sound_block_is_climbable(world: &WorldStore, block: BlockView) -> bool {
    block.block_name == Some(LOCAL_PLAYER_POWDER_SNOW_BLOCK_NAME)
        || movement_sound_block_is_in_tag(world, block, "minecraft:climbable")
}

fn movement_sound_block_is_in_tag(world: &WorldStore, block: BlockView, tag: &str) -> bool {
    let Some(block_id) = vanilla_block_state_block_type_id(block.block_state_id).or_else(|| {
        world
            .content_registry()
            .block_by_state_id(block.block_state_id)
            .map(|(spec, _)| spec.block_id)
    }) else {
        return false;
    };
    world.registry_holder_is_in_tag("minecraft:block", tag, block_id)
}
