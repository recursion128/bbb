use super::*;

pub(super) fn should_play_ambient_desert_sand_sound(world: &WorldStore, pos: BlockPos) -> bool {
    const DIRECTIONS: [[i32; 2]; 4] = [[0, -8], [8, 0], [0, 8], [-8, 0]];

    let mut matching_blocks_found = 0;
    let mut sides_checked = 0;
    for [dx, dz] in DIRECTIONS {
        let column_pos = BlockPos {
            x: pos.x.saturating_add(dx),
            y: pos.y,
            z: pos.z.saturating_add(dz),
        };
        if column_contains_ambient_desert_sand_trigger(world, column_pos) {
            let previous_matches = matching_blocks_found;
            matching_blocks_found += 1;
            if previous_matches >= 3 {
                return true;
            }
        }

        sides_checked += 1;
        let remaining_sides = 4 - sides_checked;
        if remaining_sides + matching_blocks_found < 3 {
            return false;
        }
    }
    false
}

pub(super) fn column_contains_ambient_desert_sand_trigger(
    world: &WorldStore,
    pos: BlockPos,
) -> bool {
    let Some(surface_y) = world
        .sample_heightmap_first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, pos.x, pos.z)
        .and_then(|first_available| first_available.checked_sub(1))
    else {
        return false;
    };

    if surface_y.abs_diff(pos.y) > 5 {
        let mut above_name = block_name_at(
            world,
            BlockPos {
                y: pos.y.saturating_add(6),
                ..pos
            },
        );
        for offset in (pos.y.saturating_sub(4)..=pos.y.saturating_add(5)).rev() {
            let current_name = block_name_at(world, BlockPos { y: offset, ..pos });
            if above_name.as_deref().is_some_and(block_name_is_air)
                && current_name
                    .as_deref()
                    .is_some_and(block_name_triggers_ambient_desert_sand_block_sounds)
            {
                return true;
            }
            above_name = current_name;
        }
        return false;
    }

    block_name_at(
        world,
        BlockPos {
            y: surface_y.saturating_add(1),
            ..pos
        },
    )
    .as_deref()
    .is_some_and(block_name_is_air)
        && block_name_at(
            world,
            BlockPos {
                y: surface_y,
                ..pos
            },
        )
        .as_deref()
        .is_some_and(block_name_triggers_ambient_desert_sand_block_sounds)
}

pub(super) fn supports_ambient_desert_dry_vegetation_sound(
    world: &WorldStore,
    pos: BlockPos,
) -> bool {
    let below = BlockPos {
        y: pos.y.saturating_sub(1),
        ..pos
    };
    let below_below = BlockPos {
        y: below.y.saturating_sub(1),
        ..below
    };
    [below, below_below].into_iter().all(|support| {
        block_name_at(world, support)
            .as_deref()
            .is_some_and(block_name_triggers_ambient_desert_dry_vegetation_block_sounds)
    })
}

pub(super) fn block_name_at(world: &WorldStore, pos: BlockPos) -> Option<String> {
    world.probe_block(pos)?.block_name
}

pub(super) fn append_leaves_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    profile: LeafAnimateParticleProfile,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let below = BlockPos {
        y: pos.y.saturating_sub(1),
        ..pos
    };
    let below_sample = world.environment_animate_block_sample(below);

    if world.is_raining_at(BlockPos {
        y: pos.y.saturating_add(1),
        ..pos
    }) && random.next_int_bound(15) == 1
        && below_sample.is_some_and(|sample| !sample.can_occlude || !sample.full_face_sturdy[1])
    {
        append_particle_below(
            pos,
            ClientParticle::simple(ParticleKind::DRIPPING_WATER),
            None,
            random,
            spawns,
        );
    }

    if random.next_float() >= profile.leaf_particle_chance()
        || below_sample.is_none_or(|sample| sample.collision_full_face[1])
    {
        return;
    }
    let (particle, color_source) = match profile.tint {
        LeafParticleTint::None => (ClientParticle::simple(profile.particle), None),
        LeafParticleTint::FixedArgb(argb) => (ClientParticle::color(profile.particle, argb), None),
        LeafParticleTint::ClientLeaf => (
            ClientParticle::color(profile.particle, u32::MAX),
            Some(EnvironmentParticleColorSource::ClientLeaf {
                block_pos: pos,
                block_resource_id: profile.block_resource_id,
            }),
        ),
    };
    append_particle_below(pos, particle, color_source, random, spawns);
}

pub(super) fn append_particle_below(
    pos: BlockPos,
    particle: ClientParticle,
    color_source: Option<EnvironmentParticleColorSource>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    spawns.push(EnvironmentParticleSpawn {
        particle,
        position: [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) - 0.05,
            f64::from(pos.z) + random.next_double(),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source,
    });
}

pub(super) fn block_state_direction(facing: &str) -> Option<[i32; 3]> {
    match facing {
        "down" => Some([0, -1, 0]),
        "up" => Some([0, 1, 0]),
        "north" => Some([0, 0, -1]),
        "south" => Some([0, 0, 1]),
        "west" => Some([-1, 0, 0]),
        "east" => Some([1, 0, 0]),
        _ => None,
    }
}

pub(super) fn append_wet_sponge_animate_effect(
    world: &WorldStore,
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let direction_index = random.next_int_bound(6) as usize;
    if direction_index == 1 || neighbor_has_sturdy_opposite_face(world, pos, direction_index) {
        return;
    }
    let direction = REDSTONE_EFFECT_DIRECTION_STEPS[direction_index];
    let mut x = f64::from(pos.x);
    let mut y = f64::from(pos.y);
    let mut z = f64::from(pos.z);
    if direction_index == 0 {
        y -= 0.05;
        x += random.next_double();
        z += random.next_double();
    } else {
        y += random.next_double() * 0.8;
        if direction[0] != 0 {
            z += random.next_double();
            x += if direction[0] > 0 { 1.1 } else { 0.05 };
        } else {
            x += random.next_double();
            z += if direction[2] > 0 { 1.1 } else { 0.05 };
        }
    }

    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(ParticleKind::DRIPPING_WATER),
        position: [x, y, z],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_crying_obsidian_animate_effect(
    world: &WorldStore,
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if random.next_int_bound(5) != 0 {
        return;
    }
    let direction_index = random.next_int_bound(6) as usize;
    if direction_index == 1 || neighbor_has_sturdy_opposite_face(world, pos, direction_index) {
        return;
    }
    let direction = REDSTONE_EFFECT_DIRECTION_STEPS[direction_index];
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(ParticleKind::DRIPPING_OBSIDIAN_TEAR),
        position: [
            f64::from(pos.x) + crying_obsidian_offset(direction[0], random),
            f64::from(pos.y) + crying_obsidian_offset(direction[1], random),
            f64::from(pos.z) + crying_obsidian_offset(direction[2], random),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn crying_obsidian_offset(step: i32, random: &mut LevelEventSoundRandomState) -> f64 {
    if step == 0 {
        random.next_double()
    } else {
        0.5 + f64::from(step) * 0.6
    }
}

pub(super) fn neighbor_has_sturdy_opposite_face(
    world: &WorldStore,
    pos: BlockPos,
    direction_index: usize,
) -> bool {
    let step = REDSTONE_EFFECT_DIRECTION_STEPS[direction_index];
    world
        .environment_animate_block_sample(BlockPos {
            x: pos.x + step[0],
            y: pos.y + step[1],
            z: pos.z + step[2],
        })
        .is_some_and(|sample| {
            sample.full_face_sturdy[OPPOSITE_EFFECT_DIRECTION_INDICES[direction_index]]
        })
}
