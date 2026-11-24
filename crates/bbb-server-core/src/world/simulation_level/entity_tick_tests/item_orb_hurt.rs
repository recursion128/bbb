use super::*;
use crate::entity::{EntityData, EntityLifecycleEvent};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

// Keep the fluid scan margin inside the loaded chunk.
const FEET: Vec3d = Vec3d {
    x: 8.5,
    y: 64.0,
    z: 8.5,
};

pub(super) fn compound(entries: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}

pub(super) fn stack(name: &str, components: Vec<(&str, NbtSummaryValue)>) -> RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &compound(vec![
            ("id", NbtSummaryValue::String(name.into())),
            ("count", NbtSummaryValue::Int(1)),
            ("components", compound(components)),
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap()
}

fn body(store: &mut EntityStore, orb: bool) -> i32 {
    if orb {
        crate::entity::spawn_experience_orb(store, FEET, 1).unwrap()
    } else {
        crate::entity::spawn_item_entity(store, FEET, stack("minecraft:stone", vec![])).unwrap()
    }
}

fn health(store: &EntityStore, id: i32) -> i32 {
    match store.get(id).unwrap().data() {
        EntityData::Item(data) => data.health(),
        EntityData::ExperienceOrb(data) => data.health(),
        _ => unreachable!(),
    }
}

fn block(level: &mut LevelCtx<'_, '_>, name: &str) {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    level
        .write_block(BlockPos { x: 8, y: 64, z: 8 }, state)
        .unwrap();
}

fn contact(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) -> bool {
    let mut body = store.detach(id).unwrap();
    let ok = crate::entity::movement::move_entity_by_piston(
        &mut body,
        store,
        level,
        Vec3d::default(),
        Direction::East,
    );
    store.attach(body);
    ok
}

#[test]
fn item_orb_on_fire_tick_hurts_before_countdown_and_publishes_only_item_event() {
    for orb in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            let mut store = batch_store();
            let id = body(&mut store, orb);
            store.get_mut(id).unwrap().set_remaining_fire_ticks(20);
            store.drain_lifecycle_events();
            crate::entity::tick_one_entity(&mut store, level, id);
            assert_eq!(health(&store, id), 4);
            assert_eq!(store.get(id).unwrap().remaining_fire_ticks(), 19);
            assert_eq!(store.get(id).unwrap().tick_count(), 1);
            let events = level.event_sink.drain_game_events();
            assert_eq!(events.len(), usize::from(!orb));
            if !orb {
                assert_eq!(events[0].game_event, VanillaGameEvent::ENTITY_DAMAGE);
                assert_eq!(events[0].source_entity_id, None);
            }
        });
    }
}

#[test]
fn item_orb_lava_tick_preserves_first_tick_fire_gate_and_contact_hurt() {
    for orb in [false, true] {
        for first_tick in [false, true] {
            with_config(crate::tests::datapack_config(), |level| {
                let mut store = batch_store();
                let id = body(&mut store, orb);
                if !first_tick {
                    crate::entity::tick_one_entity(&mut store, level, id);
                    assert!(!store.get(id).unwrap().is_first_tick());
                }
                block(level, "minecraft:lava");
                let body = store.get_mut(id).unwrap();
                body.set_pos(FEET);
                body.set_delta_movement(Vec3d::default());
                body.set_remaining_fire_ticks(20);
                store.drain_lifecycle_events();
                level.event_sink.drain_game_events();
                crate::entity::tick_one_entity(&mut store, level, id);
                if first_tick {
                    assert!(store.get(id).is_none());
                } else {
                    assert_eq!(health(&store, id), 1);
                    assert_eq!(store.get(id).unwrap().remaining_fire_ticks(), 300);
                }
                assert_eq!(
                    level.event_sink.drain_game_events().len(),
                    if orb {
                        0
                    } else if first_tick {
                        2
                    } else {
                        1
                    }
                );
                let sounds = store
                    .drain_lifecycle_events()
                    .into_iter()
                    .filter(|event| matches!(event, EntityLifecycleEvent::Sound { .. }))
                    .count();
                assert_eq!(sounds, usize::from(orb || first_tick));
            });
        }
    }
}

#[test]
fn item_orb_piston_cactus_contact_commits_hurt_and_legal_refusal() {
    for orb in [false, true] {
        for invulnerable in [false, true] {
            with_config(crate::tests::datapack_config(), |level| {
                block(level, "minecraft:cactus");
                let mut store = batch_store();
                let id = body(&mut store, orb);
                store.get_mut(id).unwrap().set_invulnerable(invulnerable);
                let result = contact(&mut store, level, id).then_some(());
                assert_eq!(result, Some(()));
                assert_eq!(health(&store, id), if invulnerable { 5 } else { 4 });
                assert_eq!(
                    level.event_sink.drain_game_events().len(),
                    usize::from(!orb && !invulnerable)
                );
            });
        }
    }
}
