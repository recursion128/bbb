use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_RABBIT_ID,
};

#[test]
fn living_sound_source_uses_actual_owner_for_fall_and_extinguish() {
    for (entity_type, source, event) in [
        (
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            SoundSource::Hostile,
            "minecraft:entity.hostile.small_fall",
        ),
        (
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            SoundSource::Neutral,
            "minecraft:entity.generic.small_fall",
        ),
        (
            VANILLA_ENTITY_TYPE_BAT_ID,
            SoundSource::Neutral,
            "minecraft:entity.generic.small_fall",
        ),
        (
            VANILLA_ENTITY_TYPE_HOGLIN_ID,
            SoundSource::Hostile,
            "minecraft:entity.generic.small_fall",
        ),
    ] {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        let at = Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        };
        let id = store.spawn_general_living(entity_type, at).unwrap();
        let entity = store.detach(id).unwrap();
        let mut level = {
            let mut native = NativeLevel::new();
            for x in 0..16 {
                for z in 0..16 {
                    native.world.set_block(
                        DIMENSION,
                        BlockPos { x, y: 63, z },
                        state_id("minecraft:stone"),
                        crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
                    );
                }
            }
            native
        };
        let fall = crate::entity::living::fall_sound_for(&entity, false);
        assert_eq!(fall, event, "type {entity_type}");
        level.with_level(|level| {
            crate::entity::living::fall_damage_sounds(&entity, &mut store, level, fall)
        });
        let sounds = queued_sounds(&mut store);
        assert_eq!(sounds.len(), 2);
        assert!(
            sounds.iter().all(|sound| sound.source == source),
            "type {entity_type}"
        );
        assert_eq!(
            sounds[0].registry_id,
            queued(event, at, 1.0, 1.0).registry_id
        );
        assert_eq!(
            sounds[1].registry_id,
            queued("minecraft:block.stone.fall", at, 0.5, 0.75).registry_id
        );
        assert!(
            crate::entity::inside_blocks::extinguish::play_entity_extinguish_sound(
                &entity, &mut store
            )
            .is_some()
        );
        assert_eq!(queued_sounds(&mut store)[0].source, source);
    }
}

#[test]
fn living_sound_source_rabbit_variant_changes_the_shared_callbacks_immediately() {
    use bbb_protocol::builtin_enum_component_values::{
        VANILLA_RABBIT_VARIANT_BROWN_ID, VANILLA_RABBIT_VARIANT_EVIL_ID,
    };
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_RABBIT_ID, position())
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    for (variant, source) in [
        (VANILLA_RABBIT_VARIANT_BROWN_ID, SoundSource::Neutral),
        (VANILLA_RABBIT_VARIANT_EVIL_ID, SoundSource::Hostile),
        (VANILLA_RABBIT_VARIANT_BROWN_ID, SoundSource::Neutral),
    ] {
        crate::entity::rabbit::set_variant(&mut entity, variant);
        assert_eq!(
            crate::entity::living::fall_sound_for(&entity, false),
            "minecraft:entity.generic.small_fall"
        );
        {
            let mut native = NativeLevel::new();
            native.with_level(|level| {
                crate::entity::living::fall_damage_sounds(
                    &entity,
                    &mut store,
                    level,
                    crate::entity::living::GENERIC_SMALL_FALL_SOUND,
                )
            })
        };
        assert_eq!(queued_sounds(&mut store)[0].source, source);
        assert!(
            crate::entity::inside_blocks::extinguish::play_entity_extinguish_sound(
                &entity, &mut store
            )
            .is_some()
        );
        assert_eq!(queued_sounds(&mut store)[0].source, source);
    }
}
