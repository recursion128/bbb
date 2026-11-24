use super::*;

#[test]
fn generated_death_audio_policies_resolve_all_living_entity_types() {
    let mut store = WorldStore::new();
    let mut living_count = 0;
    for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
        if matches!(
            vanilla_entity_metadata_type_death_sound_policy(entry.id),
            Some(VanillaEntityDeathSoundPolicy::NonLiving)
        ) {
            continue;
        }
        living_count += 1;
        let entity_id = 20_000 + entry.id;
        store.apply_add_entity(protocol_add_entity_of_type(entity_id, entry.id));
        let mut samples = [0.75, 0.25].into_iter();
        let sound = store
            .living_death_sound_for_entity(entity_id, || samples.next().unwrap())
            .unwrap_or_else(|| panic!("{} death policy did not resolve", entry.resource_id));
        assert!(sound.sound.location.is_some(), "{}", entry.resource_id);
        assert_close(sound.pitch, 1.1);
        assert!(
            samples.next().is_none(),
            "{} death pitch consumed fewer than two samples",
            entry.resource_id
        );

        let expected_volume = match vanilla_entity_metadata_type_sound_volume_policy(entry.id)
            .expect("every fixed type has a generated volume policy")
        {
            VanillaEntitySoundVolumePolicy::Fixed(bits) => f32::from_bits(bits),
            VanillaEntitySoundVolumePolicy::SlimeSize => 0.4,
            VanillaEntitySoundVolumePolicy::HappyGhastAge => 4.0,
            VanillaEntitySoundVolumePolicy::NonLiving => {
                panic!("{} living row has non-living volume", entry.resource_id)
            }
        };
        assert_close(sound.volume, expected_volume);
    }
    assert_eq!(living_count, 92);
}

#[test]
fn player_powder_snow_fall_sounds_use_player_category_and_size_variants() {
    let mut store = WorldStore::new();
    for (big, expected_event) in [
        (false, "minecraft:entity.player.small_fall"),
        (true, "minecraft:entity.player.big_fall"),
    ] {
        let sound = store.record_entity_client_movement_sound(EntityClientMovementSoundState {
            entity_id: 19,
            kind: EntityClientMovementSoundKind::PlayerPowderSnowFall { big },
            position: crate::EntityVec3 {
                x: 4.5,
                y: 65.9,
                z: -5.5,
            },
        });
        assert_eq!(sound.sound.location.as_deref(), Some(expected_event));
        assert_eq!(sound.source, "player");
        assert_eq!(sound.volume, 1.0);
        assert_eq!(sound.pitch, 1.0);
        assert_eq!(
            sound.position,
            ProtocolVec3d {
                x: 4.5,
                y: 65.9,
                z: -5.5,
            }
        );
    }
}

#[test]
fn entity_client_movement_sound_maps_player_honey_block_slide() {
    let mut store = WorldStore::new();
    let sound = store.record_entity_client_movement_sound(EntityClientMovementSoundState {
        entity_id: 19,
        kind: EntityClientMovementSoundKind::PlayerHoneyBlockSlide,
        position: crate::EntityVec3 {
            x: 4.5,
            y: 65.9,
            z: -5.5,
        },
    });

    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.honey_block.slide")
    );
    assert_eq!(sound.source, "player");
    assert_eq!(sound.volume, 1.0);
    assert_eq!(sound.pitch, 1.0);
}
