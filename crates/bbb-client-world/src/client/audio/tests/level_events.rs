use super::*;

#[test]
fn level_event_random_state_matches_java_float_and_double_samples() {
    let mut random = LevelEventSoundRandomState::with_seed(0);
    assert_close(random.next_float(), 0.730_967_76);
    assert_close(random.next_float(), 0.831_441);
    assert_close_f64(random.next_double(), 0.240_536_415_671_485_87);
    assert_close_f64(random.next_double(), 0.637_417_425_350_108_3);

    let mut random = LevelEventSoundRandomState::with_seed(0);
    assert_eq!(random.next_long(), -4_962_768_465_676_381_896);
    assert_eq!(random.next_long(), 4_437_113_781_045_784_766);

    let mut random = LevelEventSoundRandomState::with_seed(0);
    assert_eq!(random.next_int_bound(3), 0);
    assert_eq!(random.next_int_bound(3), 1);
    assert_eq!(random.next_int_bound(4), 0);
    assert_eq!(random.next_int_bound(5), 2);

    let mut random = LevelEventSoundRandomState::with_seed(0);
    assert_eq!(
        (0..8).map(|_| random.next_boolean()).collect::<Vec<_>>(),
        vec![true, true, false, true, true, false, true, false]
    );

    let mut random = LevelEventSoundRandomState::with_seed(0);
    assert_close_f64(random.next_gaussian(), 0.802_533_063_739_030_5);
    assert_close_f64(random.next_gaussian(), -0.901_546_088_417_512_2);
    assert_close_f64(random.next_gaussian(), 2.080_920_790_428_163);
}

#[test]
fn level_event_2001_maps_block_state_id_to_vanilla_break_sound() {
    let store = WorldStore::new();

    let sound = store
        .level_event_block_break_sound(LevelEvent {
            event_type: 2001,
            pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
            data: 9,
            global: false,
        })
        .unwrap();

    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.grass.break")
    );
    assert_eq!(sound.source, "block");
    assert_eq!(
        sound.position,
        ProtocolVec3d {
            x: 2.5,
            y: 3.5,
            z: -3.5,
        }
    );
    assert_close(sound.volume, 1.0);
    assert_close(sound.pitch, 0.8);
    assert_eq!(sound.seed, 0);
    assert_eq!(
        store.level_event_sound(LevelEvent {
            event_type: 2001,
            pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
            data: 9,
            global: false,
        }),
        Some(sound)
    );

    assert!(store
        .level_event_block_break_sound(LevelEvent {
            event_type: 1001,
            pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
            data: 9,
            global: false,
        })
        .is_none());
    assert!(store
        .level_event_block_break_sound(LevelEvent {
            event_type: 2001,
            pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
            data: 0,
            global: false,
        })
        .is_none());
    for data in [-1, i32::MAX] {
        assert!(store
            .level_event_sound(LevelEvent {
                event_type: 2001,
                pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
                data,
                global: false,
            })
            .is_none());
    }
    for (cracked, expected) in [
        ("false", "minecraft:block.decorated_pot.break"),
        ("true", "minecraft:block.decorated_pot.shatter"),
    ] {
        let data = vanilla_block_state_id(
            "minecraft:decorated_pot",
            [
                ("cracked", cracked),
                ("facing", "north"),
                ("waterlogged", "false"),
            ],
        );
        let sound = store
            .level_event_sound(LevelEvent {
                event_type: 2001,
                pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
                data,
                global: false,
            })
            .unwrap();
        assert_eq!(sound.sound.location.as_deref(), Some(expected));
        assert_close(sound.volume, 1.0);
        assert_close(sound.pitch, 0.8);
    }
}

#[test]
fn level_event_3008_maps_brushable_block_state_to_completion_sound() {
    let store = WorldStore::new();
    let suspicious_sand = vanilla_block_state_id("minecraft:suspicious_sand", [("dusted", "0")]);
    let suspicious_gravel =
        vanilla_block_state_id("minecraft:suspicious_gravel", [("dusted", "3")]);
    let stone = vanilla_block_state_id("minecraft:stone", []);

    let sand_sound = store
        .level_event_sound(LevelEvent {
            event_type: 3008,
            pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
            data: suspicious_sand,
            global: false,
        })
        .unwrap();
    assert_eq!(
        sand_sound.sound.location.as_deref(),
        Some("minecraft:item.brush.brushing.sand")
    );
    assert_eq!(sand_sound.source, "player");
    assert_eq!(sand_sound.position, vec3(2.5, 3.5, -3.5));
    assert_close(sand_sound.volume, 1.0);
    assert_close(sand_sound.pitch, 1.0);

    let gravel_sound = store
        .level_event_sound(LevelEvent {
            event_type: 3008,
            pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
            data: suspicious_gravel,
            global: false,
        })
        .unwrap();
    assert_eq!(
        gravel_sound.sound.location.as_deref(),
        Some("minecraft:item.brush.brushing.gravel")
    );
    assert_eq!(gravel_sound.source, "player");
    assert_eq!(gravel_sound.position, vec3(-0.5, 70.5, 4.5));

    assert!(store
        .level_event_sound(LevelEvent {
            event_type: 3008,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            data: stone,
            global: false,
        })
        .is_none());
}

#[test]
fn level_event_sound_maps_fixed_vanilla_level_event_audio() {
    let store = WorldStore::new();

    for (event_type, event_id, source, volume, pitch) in [
        (
            1000,
            "minecraft:block.dispenser.dispense",
            "block",
            1.0,
            1.0,
        ),
        (1001, "minecraft:block.dispenser.fail", "block", 1.0, 1.2),
        (1002, "minecraft:block.dispenser.launch", "block", 1.0, 1.2),
        (
            1004,
            "minecraft:entity.firework_rocket.shoot",
            "neutral",
            1.0,
            1.2,
        ),
        (
            1033,
            "minecraft:block.chorus_flower.grow",
            "block",
            1.0,
            1.0,
        ),
        (
            1034,
            "minecraft:block.chorus_flower.death",
            "block",
            1.0,
            1.0,
        ),
        (
            1035,
            "minecraft:block.brewing_stand.brew",
            "block",
            1.0,
            1.0,
        ),
        (1049, "minecraft:block.crafter.craft", "block", 1.0, 1.0),
        (1050, "minecraft:block.crafter.fail", "block", 1.0, 1.0),
        (
            1503,
            "minecraft:block.end_portal_frame.fill",
            "block",
            1.0,
            1.0,
        ),
        (1505, "minecraft:item.bone_meal.use", "block", 1.0, 1.0),
        (3003, "minecraft:item.honeycomb.wax_on", "block", 1.0, 1.0),
    ] {
        let sound = store
            .level_event_sound(LevelEvent {
                event_type,
                pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
                data: 7,
                global: false,
            })
            .unwrap();
        assert_eq!(sound.sound.location.as_deref(), Some(event_id));
        assert_eq!(sound.source, source);
        assert_eq!(sound.position, vec3(-0.5, 70.5, 4.5));
        assert_close(sound.volume, volume);
        assert_close(sound.pitch, pitch);
    }

    assert!(store
        .level_event_sound(LevelEvent {
            event_type: 1015,
            pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
            data: 0,
            global: false,
        })
        .is_none());
    assert!(store
        .level_event_sound(LevelEvent {
            event_type: 1501,
            pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
            data: 0,
            global: false,
        })
        .is_none());
}

#[test]
fn level_event_sound_with_random_maps_randomized_vanilla_audio() {
    let store = WorldStore::new();

    let fire_extinguish = random_level_event_sound(&store, 1009, 0, &[0.25, 0.75]);
    assert_eq!(
        fire_extinguish.sound.location.as_deref(),
        Some("minecraft:block.fire.extinguish")
    );
    assert_eq!(fire_extinguish.source, "block");
    assert_close(fire_extinguish.volume, 0.5);
    assert_close(fire_extinguish.pitch, 2.2);

    let generic_extinguish = random_level_event_sound(&store, 1009, 1, &[0.25, 0.75]);
    assert_eq!(
        generic_extinguish.sound.location.as_deref(),
        Some("minecraft:entity.generic.extinguish_fire")
    );
    assert_close(generic_extinguish.volume, 0.7);
    assert_close(generic_extinguish.pitch, 1.4);

    let ghast_warn = random_level_event_sound(&store, 1015, 0, &[0.75, 0.25]);
    assert_eq!(
        ghast_warn.sound.location.as_deref(),
        Some("minecraft:entity.ghast.warn")
    );
    assert_eq!(ghast_warn.source, "hostile");
    assert_close(ghast_warn.volume, 10.0);
    assert_close(ghast_warn.pitch, 1.1);

    let anvil_destroy = random_level_event_sound(&store, 1029, 0, &[0.5]);
    assert_eq!(
        anvil_destroy.sound.location.as_deref(),
        Some("minecraft:block.anvil.destroy")
    );
    assert_close(anvil_destroy.volume, 1.0);
    assert_close(anvil_destroy.pitch, 0.95);

    let wind_charge = random_level_event_sound(&store, 1051, 0, &[0.5]);
    assert_eq!(
        wind_charge.sound.location.as_deref(),
        Some("minecraft:entity.wind_charge.throw")
    );
    assert_eq!(wind_charge.source, "block");
    assert_close(wind_charge.volume, 0.5);
    assert_close(wind_charge.pitch, 0.4);

    let splash_potion_break = random_level_event_sound(&store, 2002, 0x3366cc, &[0.5]);
    assert_eq!(
        splash_potion_break.sound.location.as_deref(),
        Some("minecraft:entity.splash_potion.break")
    );
    assert_eq!(splash_potion_break.source, "neutral");
    assert_close(splash_potion_break.volume, 1.0);
    assert_close(splash_potion_break.pitch, 0.95);

    let instant_effect_potion_break = random_level_event_sound(&store, 2007, 0x3366cc, &[0.25]);
    assert_eq!(
        instant_effect_potion_break.sound.location.as_deref(),
        Some("minecraft:entity.splash_potion.break")
    );
    assert_eq!(instant_effect_potion_break.source, "neutral");
    assert_close(instant_effect_potion_break.volume, 1.0);
    assert_close(instant_effect_potion_break.pitch, 0.925);

    let dragon_fireball = random_level_event_sound(&store, 2006, 1, &[0.75]);
    assert_eq!(
        dragon_fireball.sound.location.as_deref(),
        Some("minecraft:entity.dragon_fireball.explode")
    );
    assert_eq!(dragon_fireball.source, "hostile");
    assert_close(dragon_fireball.volume, 1.0);
    assert_close(dragon_fireball.pitch, 0.975);

    assert!(store
        .level_event_sound_with_random(
            LevelEvent {
                event_type: 2006,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                data: 0,
                global: false,
            },
            || panic!("2006 without data=1 has no sound")
        )
        .is_none());

    for (event_type, data, event_id, volume, expected_pitch) in [
        (3012, 0, "minecraft:block.trial_spawner.spawn_mob", 1.0, 1.1),
        (
            3013,
            0,
            "minecraft:block.trial_spawner.detect_player",
            1.0,
            1.1,
        ),
        (
            3014,
            0,
            "minecraft:block.trial_spawner.eject_item",
            1.0,
            1.1,
        ),
        (
            3019,
            0,
            "minecraft:block.trial_spawner.detect_player",
            1.0,
            1.1,
        ),
        (
            3020,
            0,
            "minecraft:block.trial_spawner.ominous_activate",
            0.3,
            1.1,
        ),
        (
            3020,
            1,
            "minecraft:block.trial_spawner.ominous_activate",
            1.0,
            1.1,
        ),
        (
            3021,
            0,
            "minecraft:block.trial_spawner.spawn_item",
            1.0,
            1.1,
        ),
    ] {
        let sound = random_level_event_sound(&store, event_type, data, &[0.75, 0.25]);
        assert_eq!(sound.sound.location.as_deref(), Some(event_id));
        assert_eq!(sound.source, "block");
        assert_close(sound.volume, volume);
        assert_close(sound.pitch, expected_pitch);
        assert!(sound.distance_delay);
    }

    let end_gateway_spawn = random_level_event_sound(&store, 3000, 0, &[0.75, 0.25]);
    assert_eq!(
        end_gateway_spawn.sound.location.as_deref(),
        Some("minecraft:block.end_gateway.spawn")
    );
    assert_eq!(end_gateway_spawn.source, "block");
    assert_close(end_gateway_spawn.volume, 10.0);
    assert_close(end_gateway_spawn.pitch, 0.77);

    let dragon_growl = random_level_event_sound(&store, 3001, 0, &[0.5]);
    assert_eq!(
        dragon_growl.sound.location.as_deref(),
        Some("minecraft:entity.ender_dragon.growl")
    );
    assert_eq!(dragon_growl.source, "hostile");
    assert_close(dragon_growl.volume, 64.0);
    assert_close(dragon_growl.pitch, 0.95);

    let sculk_charge_pop = random_level_event_sound(&store, 3006, 0, &[]);
    assert_eq!(
        sculk_charge_pop.sound.location.as_deref(),
        Some("minecraft:block.sculk.charge")
    );
    assert_eq!(sculk_charge_pop.source, "block");
    assert_close(sculk_charge_pop.volume, 1.0);
    assert_close(sculk_charge_pop.pitch, 1.0);

    let sculk_charge = random_level_event_sound(&store, 3006, 2 << 6, &[0.25, 0.5, 0.25]);
    assert_eq!(
        sculk_charge.sound.location.as_deref(),
        Some("minecraft:block.sculk.charge")
    );
    assert_eq!(sculk_charge.source, "block");
    assert_close(sculk_charge.volume, 0.19);
    assert_close(sculk_charge.pitch, 0.55);

    let mut missed_samples = [0.5].into_iter();
    assert!(store
        .level_event_sound_with_random(
            LevelEvent {
                event_type: 3006,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                data: 1 << 6,
                global: false,
            },
            || missed_samples.next().unwrap(),
        )
        .is_none());

    assert!(store
        .level_event_sound_with_random(
            LevelEvent {
                event_type: 1032,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                data: 0,
                global: false,
            },
            || 0.5,
        )
        .is_none());
}

#[test]
fn cobweb_place_level_event_sound_uses_random_pitch_and_distance_delay() {
    let store = WorldStore::new();
    let event = LevelEvent {
        event_type: 3018,
        pos: ProtocolBlockPos { x: 2, y: 64, z: -5 },
        data: 0,
        global: false,
    };

    let sound = store
        .cobweb_place_level_event_sound_with_random(event, || 0.5)
        .unwrap();

    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.cobweb.place")
    );
    assert_eq!(sound.source, "block");
    assert_eq!(
        sound.position,
        ProtocolVec3d {
            x: 2.5,
            y: 64.5,
            z: -4.5,
        }
    );
    assert_close(sound.volume, 1.0);
    assert_close(sound.pitch, 1.0);
    assert_eq!(sound.seed, 0);
    assert!(sound.distance_delay);

    assert!(store
        .level_event_sound_with_random(event, || panic!("3018 uses its dedicated sound mapping"))
        .is_none());
}

#[test]
fn level_event_local_sound_with_random_maps_portal_travel_ambience() {
    let mut store = WorldStore::new();
    let sound = store
        .level_event_local_sound_with_random(
            LevelEvent {
                event_type: 1032,
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                data: 0,
                global: false,
            },
            || 0.5,
        )
        .unwrap();

    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.portal.travel")
    );
    assert_eq!(sound.source, "ambient");
    assert_close(sound.volume, 0.25);
    assert_close(sound.pitch, 1.0);
    assert_eq!(sound.seed, 0);
    assert_eq!(store.last_local_sound(), None);

    let recorded = store.record_local_sound(sound);
    assert_eq!(store.last_local_sound(), Some(&recorded));
}

#[test]
fn sculk_shrieker_level_event_sound_uses_top_y_and_random_pitch() {
    let store = WorldStore::new();
    let sound = store
        .sculk_shrieker_level_event_sound_with_random(
            LevelEvent {
                event_type: 3007,
                pos: ProtocolBlockPos { x: 2, y: 64, z: -5 },
                data: 0,
                global: false,
            },
            || 0.25,
        )
        .unwrap();

    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.sculk_shrieker.shriek")
    );
    assert_eq!(sound.source, "block");
    assert_eq!(
        sound.position,
        ProtocolVec3d {
            x: 2.5,
            y: 64.5,
            z: -4.5,
        }
    );
    assert_close(sound.volume, 2.0);
    assert_close(sound.pitch, 0.7);
    assert_eq!(sound.seed, 0);
    assert!(!sound.distance_delay);

    assert!(store
        .sculk_shrieker_level_event_sound_with_random(
            LevelEvent {
                event_type: 3008,
                pos: ProtocolBlockPos { x: 2, y: 64, z: -5 },
                data: 0,
                global: false,
            },
            || panic!("non-3007 should not sample random"),
        )
        .is_none());
}

#[test]
fn vault_level_event_sound_uses_distance_delay_and_block_entity_gate() {
    let store = WorldStore::new();
    let deactivate = store
        .vault_level_event_sound_with_random(
            LevelEvent {
                event_type: 3016,
                pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
                data: 0,
                global: false,
            },
            {
                let mut samples = [0.75, 0.25].into_iter();
                move || samples.next().unwrap()
            },
        )
        .unwrap();
    assert_eq!(
        deactivate.sound.location.as_deref(),
        Some("minecraft:block.vault.deactivate")
    );
    assert_eq!(deactivate.source, "block");
    assert_eq!(deactivate.position, vec3(-0.5, 70.5, 4.5));
    assert_close(deactivate.volume, 1.0);
    assert_close(deactivate.pitch, 1.1);
    assert!(deactivate.distance_delay);

    assert!(store
        .vault_level_event_sound_with_random(
            LevelEvent {
                event_type: 3015,
                pos: ProtocolBlockPos { x: -1, y: 70, z: 4 },
                data: 0,
                global: false,
            },
            || panic!("3015 without a vault block entity has no sound")
        )
        .is_none());
}

#[test]
fn level_event_1010_and_1011_track_jukebox_song_state() {
    let mut store = WorldStore::new();
    let pos = ProtocolBlockPos { x: 4, y: 65, z: -7 };

    store.apply_level_event(LevelEvent {
        event_type: 1010,
        pos,
        data: 12,
        global: false,
    });

    assert_eq!(
        store.playing_jukebox_songs(),
        &[JukeboxSongState {
            pos: crate::BlockPos { x: 4, y: 65, z: -7 },
            song_registry_id: 12,
        }]
    );
    assert_eq!(
        store.last_jukebox_event(),
        Some(&JukeboxLevelEventState {
            action: JukeboxLevelEventAction::Start,
            pos: crate::BlockPos { x: 4, y: 65, z: -7 },
            song_registry_id: Some(12),
            stopped_existing: false,
        })
    );
    assert_eq!(store.action_bar().unwrap().display_ticks, 60);
    assert!(store.action_bar().unwrap().animate_color);
    let first_narrations = store.take_pending_chat_narrations();
    assert_eq!(first_narrations.len(), 1);
    assert_eq!(
        first_narrations[0].kind,
        crate::ChatNarrationKind::SystemNow
    );
    assert_eq!(
        first_narrations[0].content_component.as_ref(),
        Some(store.action_bar().unwrap().content.component())
    );

    store.apply_level_event(LevelEvent {
        event_type: 1010,
        pos,
        data: 15,
        global: false,
    });

    assert_eq!(
        store.playing_jukebox_songs(),
        &[JukeboxSongState {
            pos: crate::BlockPos { x: 4, y: 65, z: -7 },
            song_registry_id: 15,
        }]
    );
    assert_eq!(
        store.last_jukebox_event().unwrap().action,
        JukeboxLevelEventAction::Start
    );
    assert!(store.last_jukebox_event().unwrap().stopped_existing);
    assert_eq!(store.take_pending_chat_narrations().len(), 1);

    store.apply_level_event(LevelEvent {
        event_type: 1011,
        pos,
        data: 0,
        global: false,
    });

    assert!(store.playing_jukebox_songs().is_empty());
    assert_eq!(
        store.last_jukebox_event(),
        Some(&JukeboxLevelEventState {
            action: JukeboxLevelEventAction::Stop,
            pos: crate::BlockPos { x: 4, y: 65, z: -7 },
            song_registry_id: None,
            stopped_existing: true,
        })
    );
    assert!(store.take_pending_chat_narrations().is_empty());
}

#[test]
fn jukebox_song_tracking_bounds_entries_and_prunes_on_chunk_forget() {
    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    for i in 0..300 {
        store.apply_level_event(LevelEvent {
            event_type: 1010,
            pos: BlockPos { x: i, y: 4, z: 0 },
            data: 1,
            global: false,
        });
    }
    assert!(store.playing_jukebox_songs().len() <= 256);

    let chunk = crate::ChunkPos { x: 0, z: 0 };
    store.insert_decoded_chunk(crate::ChunkColumn {
        pos: chunk,
        state: crate::ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![std::sync::Arc::new(crate::ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: crate::PalettedContainerData {
                domain: crate::PaletteDomain::BlockStates,
                bits_per_entry: 0,
                palette_kind: crate::PaletteKind::SingleValue,
                palette_global_ids: vec![0],
                packed_data: Vec::new(),
                entry_count: 4096,
            },
            biomes: crate::PalettedContainerData {
                domain: crate::PaletteDomain::Biomes,
                bits_per_entry: 0,
                palette_kind: crate::PaletteKind::SingleValue,
                palette_global_ids: vec![0],
                packed_data: Vec::new(),
                entry_count: 64,
            },
        })],
        block_entities: std::sync::Arc::new(Vec::new()),
        light: crate::LightData::default(),
    });
    store.apply_level_event(LevelEvent {
        event_type: 1010,
        pos: BlockPos { x: 2, y: 4, z: 3 },
        data: 1,
        global: false,
    });
    assert!(store
        .playing_jukebox_songs()
        .iter()
        .any(|song| song.pos == crate::BlockPos { x: 2, y: 4, z: 3 }));
    assert!(store.forget_chunk(chunk));
    assert!(!store
        .playing_jukebox_songs()
        .iter()
        .any(|song| song.pos.x.div_euclid(16) == 0 && song.pos.z.div_euclid(16) == 0));
}
