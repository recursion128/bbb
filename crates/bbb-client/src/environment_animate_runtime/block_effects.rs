use super::*;

pub(super) fn append_brewing_stand_animate_effect(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(ParticleKind::SMOKE),
        position: [
            f64::from(pos.x) + 0.4 + f64::from(random.next_float() * 0.2),
            f64::from(pos.y) + 0.7 + f64::from(random.next_float() * 0.3),
            f64::from(pos.z) + 0.4 + f64::from(random.next_float() * 0.2),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_torch_animate_effects(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    profile: TorchAnimateParticleProfile,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let mut offset = [0.0, 0.0, 0.0];
    if profile.wall {
        let Some([x, 0, z]) = properties
            .get("facing")
            .and_then(|facing| block_state_direction(facing))
        else {
            return;
        };
        offset = [-0.27 * f64::from(x), 0.22, -0.27 * f64::from(z)];
    }
    let position = [
        f64::from(pos.x) + 0.5 + offset[0],
        f64::from(pos.y) + 0.7 + offset[1],
        f64::from(pos.z) + 0.5 + offset[2],
    ];
    for particle in [ParticleKind::SMOKE, profile.particle] {
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(particle),
            position,
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
}

pub(super) fn append_lever_animate_effect(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("powered").map(String::as_str) != Some("true") || random.next_float() >= 0.25
    {
        return;
    }
    let Some(facing) = properties
        .get("facing")
        .and_then(|facing| block_state_direction(facing))
    else {
        return;
    };
    let connected_opposite = match properties.get("face").map(String::as_str) {
        Some("floor") => [0, -1, 0],
        Some("ceiling") => [0, 1, 0],
        Some("wall") => [-facing[0], -facing[1], -facing[2]],
        _ => return,
    };
    let facing_opposite = [-facing[0], -facing[1], -facing[2]];
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::dust(REDSTONE_PARTICLE_COLOR, 0.5),
        position: [
            f64::from(pos.x)
                + 0.5
                + 0.1 * f64::from(facing_opposite[0])
                + 0.2 * f64::from(connected_opposite[0]),
            f64::from(pos.y)
                + 0.5
                + 0.1 * f64::from(facing_opposite[1])
                + 0.2 * f64::from(connected_opposite[1]),
            f64::from(pos.z)
                + 0.5
                + 0.1 * f64::from(facing_opposite[2])
                + 0.2 * f64::from(connected_opposite[2]),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_mycelium_animate_effect(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if random.next_int_bound(10) != 0 {
        return;
    }
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(ParticleKind::MYCELIUM),
        position: [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + 1.1,
            f64::from(pos.z) + random.next_double(),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_wither_rose_animate_effects(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for _ in 0..3 {
        if random.next_boolean() {
            spawns.push(EnvironmentParticleSpawn {
                particle: ClientParticle::simple(ParticleKind::SMOKE),
                position: [
                    f64::from(pos.x) + 0.5 + random.next_double() / 5.0,
                    f64::from(pos.y) + 0.5 - random.next_double(),
                    f64::from(pos.z) + 0.5 + random.next_double() / 5.0,
                ],
                velocity: [0.0; 3],
                always_show: false,
                color_source: None,
            });
        }
    }
}

pub(super) fn append_bubble_column_animate_effects(
    pos: BlockPos,
    drag_down: bool,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<(&'static str, f32, f32)> {
    let x = f64::from(pos.x);
    let y = f64::from(pos.y);
    let z = f64::from(pos.z);
    let sound_id = if drag_down {
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::CURRENT_DOWN),
            position: [x + 0.5, y + 0.8, z],
            velocity: [0.0; 3],
            always_show: true,
            color_source: None,
        });
        SOUND_EVENT_BUBBLE_COLUMN_WHIRLPOOL_AMBIENT
    } else {
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::BUBBLE_COLUMN_UP),
            position: [x + 0.5, y, z + 0.5],
            velocity: [0.0, 0.04, 0.0],
            always_show: true,
            color_source: None,
        });
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::BUBBLE_COLUMN_UP),
            position: [
                x + f64::from(random.next_float()),
                y + f64::from(random.next_float()),
                z + f64::from(random.next_float()),
            ],
            velocity: [0.0, 0.04, 0.0],
            always_show: true,
            color_source: None,
        });
        SOUND_EVENT_BUBBLE_COLUMN_UPWARDS_AMBIENT
    };

    if random.next_int_bound(BUBBLE_COLUMN_SOUND_CHANCE_BOUND) != 0 {
        return None;
    }
    Some((
        sound_id,
        0.2 + random.next_float() * 0.2,
        0.9 + random.next_float() * 0.15,
    ))
}

pub(super) fn append_water_fluid_animate_effects(
    pos: BlockPos,
    fluid: TerrainFluidState,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> FluidAnimateSounds {
    if fluid.amount < 8 && !fluid.falling {
        if random.next_int_bound(64) == 0 {
            return [
                Some((
                    SOUND_EVENT_WATER_AMBIENT,
                    [
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y) + 0.5,
                        f64::from(pos.z) + 0.5,
                    ],
                    random.next_float() * 0.25 + 0.75,
                    random.next_float() + 0.5,
                )),
                None,
            ];
        }
    } else if random.next_int_bound(10) == 0 {
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::UNDERWATER),
            position: [
                f64::from(pos.x) + random.next_double(),
                f64::from(pos.y) + random.next_double(),
                f64::from(pos.z) + random.next_double(),
            ],
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
    [None; 2]
}

pub(super) fn append_lava_fluid_animate_effects(
    pos: BlockPos,
    exposed: bool,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> FluidAnimateSounds {
    if !exposed {
        return [None; 2];
    }
    let mut sounds = [None; 2];
    let mut next_sound = 0;
    if random.next_int_bound(100) == 0 {
        let position = [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + 1.0,
            f64::from(pos.z) + random.next_double(),
        ];
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::LAVA),
            position,
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
        sounds[next_sound] = Some((
            SOUND_EVENT_LAVA_POP,
            position,
            0.2 + random.next_float() * 0.2,
            0.9 + random.next_float() * 0.15,
        ));
        next_sound += 1;
    }
    if random.next_int_bound(200) == 0 {
        sounds[next_sound] = Some((
            SOUND_EVENT_LAVA_AMBIENT,
            block_position(pos),
            0.2 + random.next_float() * 0.2,
            0.9 + random.next_float() * 0.15,
        ));
    }
    sounds
}

pub(super) fn append_fluid_drip_particle(
    world: &WorldStore,
    fluid_pos: BlockPos,
    fluid_kind: TerrainFluidKind,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let Some(fluid_block) = world.environment_animate_block_sample(fluid_pos) else {
        return;
    };
    let below_pos = BlockPos {
        y: fluid_pos.y.saturating_sub(1),
        ..fluid_pos
    };
    let Some(below) = world.environment_animate_block_sample(below_pos) else {
        return;
    };
    if below.fluid.is_some() {
        return;
    }

    let below_top = below
        .collision_bounds
        .map_or(f64::NEG_INFINITY, |bounds| bounds.max_y);
    let range_and_y = if below_top < 1.0 {
        fluid_block.full_face_sturdy[0].then_some((
            [f64::from(below_pos.x), f64::from(below_pos.x) + 1.0],
            [f64::from(below_pos.z), f64::from(below_pos.z) + 1.0],
            f64::from(below_pos.y) + 0.95,
        ))
    } else if below.impermeable {
        None
    } else {
        let Some(bounds) = below.collision_bounds else {
            return;
        };
        if bounds.min_y > 0.0 {
            Some((
                [
                    f64::from(below_pos.x) + bounds.min_x,
                    f64::from(below_pos.x) + bounds.max_x,
                ],
                [
                    f64::from(below_pos.z) + bounds.min_z,
                    f64::from(below_pos.z) + bounds.max_z,
                ],
                f64::from(below_pos.y) + bounds.min_y - 0.05,
            ))
        } else {
            let two_below_pos = BlockPos {
                y: below_pos.y.saturating_sub(1),
                ..below_pos
            };
            let Some(two_below) = world.environment_animate_block_sample(two_below_pos) else {
                return;
            };
            let two_below_top = two_below
                .collision_bounds
                .map_or(f64::NEG_INFINITY, |bounds| bounds.max_y);
            (two_below_top < 1.0 && two_below.fluid.is_none()).then_some((
                [
                    f64::from(below_pos.x) + bounds.min_x,
                    f64::from(below_pos.x) + bounds.max_x,
                ],
                [
                    f64::from(below_pos.z) + bounds.min_z,
                    f64::from(below_pos.z) + bounds.max_z,
                ],
                f64::from(below_pos.y) - 0.05,
            ))
        }
    };
    let Some((x_range, z_range, y)) = range_and_y else {
        return;
    };
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(match fluid_kind {
            TerrainFluidKind::Water => ParticleKind::DRIPPING_WATER,
            TerrainFluidKind::Lava => ParticleKind::DRIPPING_LAVA,
        }),
        position: [
            x_range[0] + random.next_double() * (x_range[1] - x_range[0]),
            y,
            z_range[0] + random.next_double() * (z_range[1] - z_range[0]),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_furnace_animate_effects(
    pos: BlockPos,
    kind: FurnaceAnimateKind,
    properties: &std::collections::BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<&'static str> {
    if properties.get("lit").map(String::as_str) != Some("true") {
        return None;
    }

    let sound = (random.next_double() < 0.1).then_some(match kind {
        FurnaceAnimateKind::Furnace => SOUND_EVENT_FURNACE_FIRE_CRACKLE,
        FurnaceAnimateKind::BlastFurnace => SOUND_EVENT_BLAST_FURNACE_FIRE_CRACKLE,
        FurnaceAnimateKind::Smoker => SOUND_EVENT_SMOKER_SMOKE,
    });
    let center = [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y),
        f64::from(pos.z) + 0.5,
    ];
    if kind == FurnaceAnimateKind::Smoker {
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::SMOKE),
            position: [center[0], center[1] + 1.1, center[2]],
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
        return sound;
    }

    let facing = properties.get("facing").map(String::as_str);
    let side_offset = random.next_double() * 0.6 - 0.3;
    let vertical_range = if kind == FurnaceAnimateKind::Furnace {
        6.0 / 16.0
    } else {
        9.0 / 16.0
    };
    let vertical_offset = random.next_double() * vertical_range;
    let (dx, dz) = match facing {
        Some("east") => (0.52, side_offset),
        Some("west") => (-0.52, side_offset),
        Some("south") => (side_offset, 0.52),
        _ => (side_offset, -0.52),
    };
    let position = [center[0] + dx, center[1] + vertical_offset, center[2] + dz];
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(ParticleKind::SMOKE),
        position,
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
    if kind == FurnaceAnimateKind::Furnace {
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::FLAME),
            position,
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
    sound
}

pub(super) fn append_campfire_animate_effects(
    pos: BlockPos,
    spawn_particles: bool,
    properties: &std::collections::BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<(f32, f32)> {
    if properties.get("lit").map(String::as_str) != Some("true") {
        return None;
    }

    let sound = (random.next_int_bound(10) == 0)
        .then(|| (0.5 + random.next_float(), random.next_float() * 0.7 + 0.6));
    if spawn_particles && random.next_int_bound(5) == 0 {
        let particle_count = random.next_int_bound(1) + 1;
        for _ in 0..particle_count {
            spawns.push(EnvironmentParticleSpawn {
                particle: ClientParticle::simple(ParticleKind::LAVA),
                position: [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y) + 0.5,
                    f64::from(pos.z) + 0.5,
                ],
                velocity: [
                    f64::from(random.next_float() / 2.0),
                    5.0e-5,
                    f64::from(random.next_float() / 2.0),
                ],
                always_show: false,
                color_source: None,
            });
        }
    }
    sound
}

pub(super) fn append_fire_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    burns_all_blocks: bool,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<(f32, f32)> {
    let sound = (random.next_int_bound(24) == 0)
        .then(|| (1.0 + random.next_float(), random.next_float() * 0.7 + 0.3));
    let can_burn = |target| {
        burns_all_blocks
            || world
                .environment_animate_block_sample(target)
                .is_some_and(|sample| sample.fire_flammable)
    };
    let below = BlockPos {
        y: pos.y.saturating_sub(1),
        ..pos
    };
    let below_sample = world.environment_animate_block_sample(below);
    let below_burns = burns_all_blocks || below_sample.is_some_and(|sample| sample.fire_flammable);
    let below_has_sturdy_top = below_sample.is_some_and(|sample| sample.full_face_sturdy[1]);
    if below_burns || below_has_sturdy_top {
        for _ in 0..3 {
            spawns.push(EnvironmentParticleSpawn {
                particle: ClientParticle::simple(ParticleKind::LARGE_SMOKE),
                position: [
                    f64::from(pos.x) + random.next_double(),
                    f64::from(pos.y) + random.next_double() * 0.5 + 0.5,
                    f64::from(pos.z) + random.next_double(),
                ],
                velocity: [0.0; 3],
                always_show: false,
                color_source: None,
            });
        }
        return sound;
    }

    for (target, side) in [
        (
            BlockPos {
                x: pos.x.saturating_sub(1),
                ..pos
            },
            FireSmokeFace::West,
        ),
        (
            BlockPos {
                x: pos.x.saturating_add(1),
                ..pos
            },
            FireSmokeFace::East,
        ),
        (
            BlockPos {
                z: pos.z.saturating_sub(1),
                ..pos
            },
            FireSmokeFace::North,
        ),
        (
            BlockPos {
                z: pos.z.saturating_add(1),
                ..pos
            },
            FireSmokeFace::South,
        ),
        (
            BlockPos {
                y: pos.y.saturating_add(1),
                ..pos
            },
            FireSmokeFace::Up,
        ),
    ] {
        if !can_burn(target) {
            continue;
        }
        for _ in 0..2 {
            let position = match side {
                FireSmokeFace::West => [
                    f64::from(pos.x) + random.next_double() * 0.1,
                    f64::from(pos.y) + random.next_double(),
                    f64::from(pos.z) + random.next_double(),
                ],
                FireSmokeFace::East => [
                    f64::from(pos.x) + 1.0 - random.next_double() * 0.1,
                    f64::from(pos.y) + random.next_double(),
                    f64::from(pos.z) + random.next_double(),
                ],
                FireSmokeFace::North => [
                    f64::from(pos.x) + random.next_double(),
                    f64::from(pos.y) + random.next_double(),
                    f64::from(pos.z) + random.next_double() * 0.1,
                ],
                FireSmokeFace::South => [
                    f64::from(pos.x) + random.next_double(),
                    f64::from(pos.y) + random.next_double(),
                    f64::from(pos.z) + 1.0 - random.next_double() * 0.1,
                ],
                FireSmokeFace::Up => [
                    f64::from(pos.x) + random.next_double(),
                    f64::from(pos.y) + 1.0 - random.next_double() * 0.1,
                    f64::from(pos.z) + random.next_double(),
                ],
            };
            spawns.push(EnvironmentParticleSpawn {
                particle: ClientParticle::simple(ParticleKind::LARGE_SMOKE),
                position,
                velocity: [0.0; 3],
                always_show: false,
                color_source: None,
            });
        }
    }
    sound
}

pub(super) fn append_respawn_anchor_animate_effects(
    pos: BlockPos,
    properties: &std::collections::BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> bool {
    if !properties
        .get("charges")
        .is_some_and(|charges| charges != "0")
    {
        return false;
    }
    let play_sound = random.next_int_bound(100) == 0;
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(ParticleKind::REVERSE_PORTAL),
        position: [
            f64::from(pos.x) + 0.5 + (0.5 - random.next_double()),
            f64::from(pos.y) + 1.0,
            f64::from(pos.z) + 0.5 + (0.5 - random.next_double()),
        ],
        velocity: [0.0, f64::from(random.next_float()) * 0.04, 0.0],
        always_show: false,
        color_source: None,
    });
    play_sound
}

pub(super) fn append_nether_portal_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<f32> {
    let pitch = (random.next_int_bound(100) == 0).then(|| random.next_float() * 0.4 + 0.8);
    let portal_at = |target| {
        world
            .probe_block(target)
            .is_some_and(|block| block.block_name.as_deref() == Some("minecraft:nether_portal"))
    };
    let pin_x = !portal_at(BlockPos {
        x: pos.x.saturating_sub(1),
        ..pos
    }) && !portal_at(BlockPos {
        x: pos.x.saturating_add(1),
        ..pos
    });
    for _ in 0..4 {
        let mut position = [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + random.next_double(),
            f64::from(pos.z) + random.next_double(),
        ];
        let mut velocity = [
            (f64::from(random.next_float()) - 0.5) * 0.5,
            (f64::from(random.next_float()) - 0.5) * 0.5,
            (f64::from(random.next_float()) - 0.5) * 0.5,
        ];
        let flip = random.next_int_bound(2) * 2 - 1;
        if pin_x {
            position[0] = f64::from(pos.x) + 0.5 + 0.25 * f64::from(flip);
            velocity[0] = f64::from(random.next_float() * 2.0 * flip as f32);
        } else {
            position[2] = f64::from(pos.z) + 0.5 + 0.25 * f64::from(flip);
            velocity[2] = f64::from(random.next_float() * 2.0 * flip as f32);
        }
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::PORTAL),
            position,
            velocity,
            always_show: false,
            color_source: None,
        });
    }
    pitch
}

pub(super) fn append_dried_ghast_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    waterlogged: bool,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<&'static str> {
    let center = [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y) + 0.5,
        f64::from(pos.z) + 0.5,
    ];
    if waterlogged {
        let sound =
            (random.next_int_bound(40) == 0).then_some(SOUND_EVENT_DRIED_GHAST_AMBIENT_WATER);
        if random.next_int_bound(6) == 0 {
            spawns.push(EnvironmentParticleSpawn {
                particle: ClientParticle::simple(ParticleKind::HAPPY_VILLAGER),
                position: [
                    center[0] + f64::from((random.next_float() * 2.0 - 1.0) / 3.0),
                    center[1] + 0.4,
                    center[2] + f64::from((random.next_float() * 2.0 - 1.0) / 3.0),
                ],
                velocity: [0.0, f64::from(random.next_float()), 0.0],
                always_show: false,
                color_source: None,
            });
        }
        return sound;
    }

    let below = BlockPos {
        y: pos.y.saturating_sub(1),
        ..pos
    };
    let sound = (random.next_int_bound(40) == 0
        && world
            .environment_animate_block_sample(below)
            .is_some_and(|sample| sample.triggers_dried_ghast_ambient_sound))
    .then_some(SOUND_EVENT_DRIED_GHAST_AMBIENT);
    if random.next_int_bound(6) == 0 {
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::WHITE_SMOKE),
            position: center,
            velocity: [0.0, 0.02, 0.0],
            always_show: false,
            color_source: None,
        });
    }
    sound
}

pub(super) fn append_pointed_dripstone_animate_effect(
    world: &WorldStore,
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("vertical_direction").map(String::as_str) != Some("down")
        || properties.get("thickness").map(String::as_str) != Some("tip")
        || properties.get("waterlogged").map(String::as_str) != Some("false")
    {
        return;
    }

    let random_value = random.next_float();
    if random_value > 0.12 {
        return;
    }
    let Some(context) = pointed_dripstone_drip_context(world, pos) else {
        return;
    };
    if random_value >= 0.02 && context.fluid_kind.is_none() {
        return;
    }
    let Some(particle) = context.particle else {
        return;
    };

    spawns.push(EnvironmentParticleSpawn {
        particle,
        position: pointed_dripstone_particle_position(pos),
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_beehive_animate_effect(
    world: &WorldStore,
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    animate_random: &mut LevelEventSoundRandomState,
    level_random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties
        .get("honey_level")
        .and_then(|value| value.parse::<u8>().ok())
        .is_none_or(|honey_level| honey_level < 5)
    {
        return;
    }

    let count = animate_random.next_int_bound(1) + 1;
    for _ in 0..count {
        let Some(sample) = world.environment_animate_block_sample(pos) else {
            continue;
        };
        if sample.fluid.is_some() || level_random.next_float() < 0.3 {
            continue;
        }
        let Some(bounds) = sample.collision_bounds else {
            continue;
        };
        if bounds.max_y < 1.0 || sample.impermeable {
            continue;
        }

        let height = if bounds.min_y > 0.0 {
            f64::from(pos.y) + bounds.min_y - 0.05
        } else {
            let below = BlockPos {
                y: pos.y.saturating_sub(1),
                ..pos
            };
            let Some(below_sample) = world.environment_animate_block_sample(below) else {
                continue;
            };
            let below_top = below_sample
                .collision_bounds
                .map_or(0.0, |bounds| bounds.max_y);
            let Some(below_is_full_block) = world.sample_block_collision_shape_is_full_block(below)
            else {
                continue;
            };
            if (below_top >= 1.0 && below_is_full_block) || below_sample.fluid.is_some() {
                continue;
            }
            f64::from(pos.y) - 0.05
        };

        let x1 = f64::from(pos.x) + bounds.min_x;
        let x2 = f64::from(pos.x) + bounds.max_x;
        let z1 = f64::from(pos.z) + bounds.min_z;
        let z2 = f64::from(pos.z) + bounds.max_z;
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::DRIPPING_HONEY),
            position: [
                x1 + level_random.next_double() * (x2 - x1),
                height,
                z1 + level_random.next_double() * (z2 - z1),
            ],
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
}

pub(super) fn append_spore_blossom_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(ParticleKind::FALLING_SPORE_BLOSSOM),
        position: [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + 0.7,
            f64::from(pos.z) + random.next_double(),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });

    for _ in 0..14 {
        let ambient_pos = BlockPos {
            x: pos.x + random.next_int_bound(21) - 10,
            y: pos.y - random.next_int_bound(10),
            z: pos.z + random.next_int_bound(21) - 10,
        };
        if world.sample_block_collision_shape_is_full_block(ambient_pos) != Some(false) {
            continue;
        }
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::SPORE_BLOSSOM_AIR),
            position: [
                f64::from(ambient_pos.x) + random.next_double(),
                f64::from(ambient_pos.y) + random.next_double(),
                f64::from(ambient_pos.z) + random.next_double(),
            ],
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
}

pub(super) fn append_enchanting_table_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for (offset_x, offset_y, offset_z) in enchanting_table_bookshelf_offsets() {
        if random.next_int_bound(16) != 0 {
            continue;
        }
        let provider_pos = BlockPos {
            x: pos.x + offset_x,
            y: pos.y + offset_y,
            z: pos.z + offset_z,
        };
        if !world
            .environment_animate_block_sample(provider_pos)
            .is_some_and(|sample| sample.enchantment_power_provider)
        {
            continue;
        }
        let transmitter_pos = BlockPos {
            x: pos.x + offset_x / 2,
            y: pos.y + offset_y,
            z: pos.z + offset_z / 2,
        };
        if !world
            .environment_animate_block_sample(transmitter_pos)
            .is_some_and(|sample| sample.enchantment_power_transmitter)
        {
            continue;
        }

        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::ENCHANT),
            position: [
                f64::from(pos.x) + 0.5,
                f64::from(pos.y) + 2.0,
                f64::from(pos.z) + 0.5,
            ],
            velocity: [
                f64::from(offset_x as f32 + random.next_float()) - 0.5,
                f64::from(offset_y as f32 - random.next_float() - 1.0),
                f64::from(offset_z as f32 + random.next_float()) - 0.5,
            ],
            always_show: false,
            color_source: None,
        });
    }
}

pub(super) fn enchanting_table_bookshelf_offsets() -> impl Iterator<Item = (i32, i32, i32)> {
    (-2_i32..=2).flat_map(|z| {
        (0_i32..=1).flat_map(move |y| {
            (-2_i32..=2).filter_map(move |x| (x.abs() == 2 || z.abs() == 2).then_some((x, y, z)))
        })
    })
}

pub(super) fn append_end_rod_animate_effect(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let Some(direction) = properties
        .get("facing")
        .and_then(|facing| block_state_direction(facing))
    else {
        return;
    };
    let x = f64::from(pos.x) + 0.55 - f64::from(random.next_float() * 0.1_f32);
    let y = f64::from(pos.y) + 0.55 - f64::from(random.next_float() * 0.1_f32);
    let z = f64::from(pos.z) + 0.55 - f64::from(random.next_float() * 0.1_f32);
    let r = f64::from(0.4_f32 - (random.next_float() + random.next_float()) * 0.4_f32);
    if random.next_int_bound(5) != 0 {
        return;
    }

    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(ParticleKind::END_ROD),
        position: [
            x + f64::from(direction[0]) * r,
            y + f64::from(direction[1]) * r,
            z + f64::from(direction[2]) * r,
        ],
        velocity: [
            random.next_gaussian() * 0.005,
            random.next_gaussian() * 0.005,
            random.next_gaussian() * 0.005,
        ],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_candle_animate_effects(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    on_cake: bool,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Vec<CandleAnimateSound> {
    if properties.get("lit").map(String::as_str) != Some("true") {
        return Vec::new();
    }
    let candles = if on_cake {
        1
    } else {
        let Some(candles) = properties
            .get("candles")
            .and_then(|candles| candles.parse::<u8>().ok())
            .filter(|candles| (1..=4).contains(candles))
        else {
            return Vec::new();
        };
        candles
    };

    let mut sounds = Vec::new();
    for offset in candle_wick_offsets(candles, on_cake) {
        let position = [
            f64::from(pos.x) + offset[0] / 16.0,
            f64::from(pos.y) + offset[1] / 16.0,
            f64::from(pos.z) + offset[2] / 16.0,
        ];
        let chance = random.next_float();
        if chance < 0.3 {
            spawns.push(EnvironmentParticleSpawn {
                particle: ClientParticle::simple(ParticleKind::SMOKE),
                position,
                velocity: [0.0; 3],
                always_show: false,
                color_source: None,
            });
            if chance < 0.17 {
                sounds.push((
                    position.map(|coordinate| coordinate + 0.5),
                    1.0_f32 + random.next_float(),
                    random.next_float() * 0.7_f32 + 0.3_f32,
                ));
            }
        }
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::SMALL_FLAME),
            position,
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
    sounds
}

pub(super) fn append_lightning_rod_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if !world.is_thundering() {
        return;
    }
    let game_time = world.world_time().map_or(0, |time| time.game_time);
    if i64::from(random.next_int_bound(LIGHTNING_ROD_SPARK_CYCLE))
        > game_time % i64::from(LIGHTNING_ROD_SPARK_CYCLE)
        || world.sample_heightmap_first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, pos.x, pos.z)
            != pos.y.checked_add(1)
    {
        return;
    }
    let Some([step_x, step_y, _]) = properties
        .get("facing")
        .and_then(|facing| block_state_direction(facing))
    else {
        return;
    };
    let axis = if step_x != 0 {
        0
    } else if step_y != 0 {
        1
    } else {
        2
    };
    let particle_count = random.next_int_bound(2) + 1;
    for _ in 0..particle_count {
        let (position, velocity) =
            particle_utils_axis_sample(pos, axis, ELECTRIC_SPARK_AXIS_RADIUS, random);
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::ELECTRIC_SPARK),
            position: [position.x, position.y, position.z],
            velocity: [velocity.x, velocity.y, velocity.z],
            always_show: false,
            color_source: None,
        });
    }
}

pub(super) fn append_ender_chest_animate_effects(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for _ in 0..3 {
        let flip_x = random.next_int_bound(2) * 2 - 1;
        let flip_z = random.next_int_bound(2) * 2 - 1;
        let position = [
            f64::from(pos.x) + 0.5 + 0.25 * f64::from(flip_x),
            f64::from(pos.y) + f64::from(random.next_float()),
            f64::from(pos.z) + 0.5 + 0.25 * f64::from(flip_z),
        ];
        let velocity = [
            f64::from(random.next_float() * flip_x as f32),
            f64::from((random.next_float() - 0.5_f32) * 0.125_f32),
            f64::from(random.next_float() * flip_z as f32),
        ];
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::PORTAL),
            position,
            velocity,
            always_show: false,
            color_source: None,
        });
    }
}

pub(super) fn append_end_portal_animate_effect(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(ParticleKind::SMOKE),
        position: [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + 0.8,
            f64::from(pos.z) + random.next_double(),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_end_gateway_animate_effects(
    pos: BlockPos,
    particle_count: usize,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for _ in 0..particle_count {
        let mut position = [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + random.next_double(),
            f64::from(pos.z) + random.next_double(),
        ];
        let mut velocity = [
            (random.next_double() - 0.5) * 0.5,
            (random.next_double() - 0.5) * 0.5,
            (random.next_double() - 0.5) * 0.5,
        ];
        let flip = random.next_int_bound(2) * 2 - 1;
        if random.next_boolean() {
            position[2] = f64::from(pos.z) + 0.5 + 0.25 * f64::from(flip);
            velocity[2] = f64::from(random.next_float() * 2.0 * flip as f32);
        } else {
            position[0] = f64::from(pos.x) + 0.5 + 0.25 * f64::from(flip);
            velocity[0] = f64::from(random.next_float() * 2.0 * flip as f32);
        }
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::PORTAL),
            position,
            velocity,
            always_show: false,
            color_source: None,
        });
    }
}

/// Vanilla `ClientLevel.doAnimateTick` marker branch: a matching sampled block
/// spawns one zero-velocity BLOCK_MARKER particle carrying the sampled state at
/// the block center.
pub(super) fn append_block_marker_animate_effect(
    pos: BlockPos,
    block_state_id: i32,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::block(ParticleKind::BLOCK_MARKER, block_state_id),
        position: [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y) + 0.5,
            f64::from(pos.z) + 0.5,
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_falling_dust_animate_effect(
    world: &WorldStore,
    pos: BlockPos,
    block_state_id: i32,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if random.next_int_bound(FALLING_DUST_CHANCE_BOUND) != 0
        || world.falling_block_is_free(BlockPos {
            y: pos.y.saturating_sub(1),
            ..pos
        }) != Some(true)
    {
        return;
    }
    append_particle_below(
        pos,
        ClientParticle::block(ParticleKind::FALLING_DUST, block_state_id),
        None,
        random,
        spawns,
    );
}
