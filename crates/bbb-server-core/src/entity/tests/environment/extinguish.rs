use super::*;

use crate::entity::inside_blocks::extinguish::play_entity_extinguish_sound;
use crate::entity::projectile_shoot::ProjectileShot;
use bbb_protocol::builtin_enum_component_values::VANILLA_RABBIT_VARIANT_EVIL_ID;

fn sounds(store: &mut EntityStore) -> Vec<bbb_protocol::packets::SoundEvent> {
    store
        .drain_lifecycle_events()
        .into_iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Sound { sound } => Some(sound),
            _ => None,
        })
        .collect()
}

fn extinguish(store: &mut EntityStore, id: i32) -> Option<()> {
    let entity = store.detach(id).unwrap();
    let result = play_entity_extinguish_sound(&entity, store);
    store.attach(entity);
    result
}

fn shot() -> ProjectileShot {
    ProjectileShot {
        movement: Vec3d::default(),
        source_movement: Vec3d::default(),
    }
}

#[test]
fn extinguish_sound_plays_even_for_a_silent_item() {
    let mut store = EntityStore::new(82, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    store.get_mut(id).unwrap().set_silent(true);
    assert_eq!(extinguish(&mut store, id), Some(()));
    let actual = sounds(&mut store);
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].source, SoundSource::Ambient);
}

#[test]
fn extinguishing_a_fireball_plays_no_sound() {
    let mut store = EntityStore::new(83, EntityIdCounter::new());
    let large =
        crate::entity::spawn_large_fireball(&mut store, FEET, shot(), Uuid::nil(), 1).unwrap();
    let small = crate::entity::spawn_small_fireball(&mut store, FEET, shot(), None).unwrap();
    for id in [large, small] {
        assert_eq!(extinguish(&mut store, id), Some(()));
    }
    assert!(sounds(&mut store).is_empty());
}

#[test]
fn extinguish_sources_include_nonliving_overrides_and_dynamic_evil_rabbit() {
    let mut store = EntityStore::new(84, EntityIdCounter::new());
    let item = item_at(&mut store, FEET);
    let orb = crate::entity::experience_orb::spawn_experience_orb(&mut store, FEET, 1).unwrap();
    let lightning =
        crate::entity::lightning_bolt::spawn_lightning_bolt(&mut store, FEET, true, None).unwrap();
    let bullet = store.spawn(
        vanilla_entity_type_id_for_resource_id("minecraft:shulker_bullet").unwrap(),
        FEET,
        EntityData::ShulkerBullet(
            crate::entity::shulker_bullet::ShulkerBulletData::from_saved(
                crate::entity::hurting_projectile::ProjectileState::new(None),
                None,
                None,
                0,
                Vec3d::default(),
            ),
        ),
    );
    let dragon_fireball =
        crate::entity::hurting_projectile::dragon_fireball::spawn_dragon_fireball(
            &mut store,
            FEET,
            shot(),
            Uuid::nil(),
        )
        .unwrap();
    let zombie = living_at(&mut store, "minecraft:zombie", FEET);
    let rabbit = living_at(&mut store, "minecraft:rabbit", FEET);
    for (id, source) in [
        (item, SoundSource::Ambient),
        (orb, SoundSource::Ambient),
        (lightning, SoundSource::Weather),
        (bullet, SoundSource::Hostile),
        (dragon_fireball, SoundSource::Neutral),
        (zombie, SoundSource::Hostile),
        (rabbit, SoundSource::Neutral),
    ] {
        assert_eq!(extinguish(&mut store, id), Some(()));
        let emitted = sounds(&mut store);
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].source, source);
    }
    assert!(crate::entity::rabbit::set_variant(
        store.get_mut(rabbit).unwrap(),
        VANILLA_RABBIT_VARIANT_EVIL_ID
    ));
    assert_eq!(extinguish(&mut store, rabbit), Some(()));
    assert_eq!(sounds(&mut store)[0].source, SoundSource::Hostile);
}
