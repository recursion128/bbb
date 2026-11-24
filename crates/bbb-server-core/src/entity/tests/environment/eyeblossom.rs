use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::difficulties::Difficulty;

const FLOWERS: [&str; 2] = ["minecraft:open_eyeblossom", "minecraft:closed_eyeblossom"];
const ATTRACTIVE: &str = "minecraft:bee_attractive";

fn flower_level(flower: &str, membership: bool) -> NativeLevel {
    let mut native = NativeLevel::new();
    native
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id(flower), 18);
    let members: &[&str] = if membership { &[flower] } else { &[] };
    native.install_tags(registry_tag_snapshot(&[(
        "minecraft:block",
        ATTRACTIVE,
        members,
    )]));
    native
}

fn scan(store: &mut EntityStore, native: &mut NativeLevel, id: i32) -> bool {
    let mut body = store.detach(id).unwrap();
    let resolved = native.with_level(|level| {
        crate::entity::living::apply_living_inside_block_effects_and_freezing(
            &mut body,
            store,
            level,
            crate::entity::general_living::hurt_from_environment_outcome,
        )
    });
    store.attach(body);
    resolved
}

fn poison(store: &EntityStore, id: i32) -> Option<(i32, i32)> {
    store
        .get(id)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:poison")
        .map(|effect| (effect.duration(), effect.amplifier()))
}

#[test]
fn eyeblossom_inside_bee_adds_poison_outside_peaceful() {
    for flower in FLOWERS {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        store.set_difficulty(Difficulty::Normal);
        let id = living_at(&mut store, "minecraft:bee", FEET);
        let mut level = flower_level(flower, true);
        assert!(scan(&mut store, &mut level, id));
        assert_eq!(poison(&store, id), Some((25, 0)));
    }
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    store.set_difficulty(Difficulty::Peaceful);
    let id = living_at(&mut store, "minecraft:bee", FEET);
    let mut level = flower_level(FLOWERS[0], true);
    assert!(scan(&mut store, &mut level, id));
    assert_eq!(poison(&store, id), None);
}

#[test]
fn eyeblossom_inside_needs_the_attractive_tag_and_never_refreshes_poison() {
    for membership in [true, false] {
        for existing in [None, Some((8, 2)), Some((70, 0))] {
            let mut store = EntityStore::new(1, EntityIdCounter::new());
            store.set_difficulty(Difficulty::Normal);
            let id = living_at(&mut store, "minecraft:bee", FEET);
            if let Some((duration, amplifier)) = existing {
                assert!(store
                    .get_mut(id)
                    .unwrap()
                    .living_mut()
                    .unwrap()
                    .add_active_effect(
                        crate::entity::ActiveMobEffect::new(
                            "minecraft:poison",
                            duration,
                            amplifier
                        )
                        .unwrap(),
                    ));
            }
            let mut level = flower_level(FLOWERS[0], membership);
            assert!(scan(&mut store, &mut level, id));
            let expected = existing.or_else(|| (membership).then_some((25, 0)));
            assert_eq!(poison(&store, id), expected);
        }
    }
}

#[test]
fn eyeblossom_inside_nonbee_continues_swept_scan_to_next_contact() {
    for flower in FLOWERS {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        store.set_difficulty(Difficulty::Normal);
        let id = living_at(&mut store, "minecraft:pig", FEET);
        let mut entity = store.detach(id).unwrap();
        let to = Vec3d { x: 1.5, ..FEET };
        entity.record_movement(crate::entity::inside_blocks::EntityMovement::direct(
            FEET, to,
        ));
        entity.set_pos(to);
        let mut level = flower_level(flower, true);
        level
            .world
            .set_block(DIMENSION, pos(1, 64, 0), state_id("minecraft:campfire"), 18);
        let before = entity.living().unwrap().health();
        assert!(level.with_level(|ctx| {
            crate::entity::living::apply_living_inside_block_effects_and_freezing(
                &mut entity,
                &mut store,
                ctx,
                crate::entity::general_living::hurt_from_environment_outcome,
            )
        }));
        store.attach(entity);
        assert_eq!(
            store.get(id).unwrap().living().unwrap().health(),
            before - 1.0
        );
        assert_eq!(poison(&store, id), None);
    }
}

#[test]
fn eyeblossom_piston_entry_poisons_a_bee() {
    for flower in FLOWERS {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        store.set_difficulty(Difficulty::Normal);
        let id = living_at(&mut store, "minecraft:bee", FEET);
        let mut entity = store.detach(id).unwrap();
        let mut level = flower_level(flower, true);
        assert!(level.with_level(|ctx| {
            crate::entity::apply_nonliving_inside_block_effects_between(
                &mut entity,
                ctx,
                Some(&mut store),
                FEET,
                FEET,
            )
        }));
        store.attach(entity);
        assert_eq!(poison(&store, id), Some((25, 0)));
    }
}
