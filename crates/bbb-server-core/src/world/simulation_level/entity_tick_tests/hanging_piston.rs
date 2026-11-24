use super::*;
use crate::entity::{EntityData, EntityLifecycleEvent, ItemFrameData};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
};
use bbb_protocol::packets::SoundEventHolder;
use bbb_protocol::NbtSummaryValue;

const FEET: Vec3d = Vec3d {
    x: 8.5,
    y: 65.5,
    z: 8.96875,
};
const ATTACHED: BlockPos = BlockPos { x: 8, y: 65, z: 8 };

fn frame(store: &mut EntityStore, glow: bool, fixed: bool, occupied: bool) -> i32 {
    let item = occupied.then(|| {
        super::item_orb_hurt::stack(
            "minecraft:filled_map",
            vec![("minecraft:map_id", NbtSummaryValue::Int(19))],
        )
    });
    let data = ItemFrameData::new(ATTACHED, Direction::North)
        .with_saved_fields(item, 0, 1.0, fixed, false);
    let (ty, data) = if glow {
        (
            VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
            EntityData::GlowItemFrame(data),
        )
    } else {
        (
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            EntityData::ItemFrame(data),
        )
    };
    store.spawn(ty, FEET, data)
}

fn block(level: &mut LevelCtx<'_, '_>, name: &str) {
    let ty = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(ty.id).unwrap();
    level.write_block(ATTACHED, state).unwrap();
}

fn contact(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32, delta: f64) -> bool {
    let mut body = store.detach(id).unwrap();
    let ok = crate::entity::movement::move_entity_by_piston(
        &mut body,
        store,
        level,
        Vec3d {
            x: delta,
            y: 0.0,
            z: 0.0,
        },
        Direction::East,
    );
    store.attach(body);
    ok
}

fn assert_removed_and_drops(events: &[EntityLifecycleEvent], id: i32, drops: usize) {
    assert!(events.iter().any(
        |event| matches!(event, EntityLifecycleEvent::Removed { entity, tracked: true }
        if entity.id() == id && entity.position() == FEET)
    ));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event,
        EntityLifecycleEvent::Removed { entity, .. } if entity.id() == id))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, EntityLifecycleEvent::Added { .. }))
            .count(),
        drops
    );
}

#[test]
fn hanging_piston_nonzero_move_kills_invulnerable_frame_without_translation() {
    for glow in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            let mut store = batch_store();
            let id = frame(&mut store, glow, false, true);
            store.get_mut(id).unwrap().set_invulnerable(true);
            store.drain_lifecycle_events();
            assert!(contact(&mut store, level, id, 0.1));
            assert!(store.get(id).is_none());
            assert_removed_and_drops(&store.drain_lifecycle_events(), id, 2);
            assert_eq!(store.drain_framed_map_removals().len(), 1);
            let events = level.event_sink.drain_positioned_game_events();
            let outputs = events
                .iter()
                .map(|event| (event.recorded.game_event, event.recorded.source_entity_id))
                .collect::<Vec<_>>();
            for expected in [
                (VanillaGameEvent::ENTITY_DIE, Some(id)),
                (VanillaGameEvent::BLOCK_CHANGE, None),
            ] {
                assert!(outputs.contains(&expected));
            }
            assert!(events.iter().any(|event| event.recorded.game_event
                == VanillaGameEvent::ENTITY_DIE
                && event.position == FEET));
        });
    }
}

#[test]
fn hanging_piston_zero_motion_still_runs_cactus_hurt_but_ordinary_tick_does_not() {
    with_config(crate::tests::datapack_config(), |level| {
        let mut store = batch_store();
        let id = frame(&mut store, false, false, true);
        block(level, "minecraft:cactus");
        store.drain_lifecycle_events();
        crate::entity::tick_one_entity(&mut store, level, id);
        assert!(
            matches!(store.get(id).unwrap().data(), EntityData::ItemFrame(frame) if frame.item().is_some())
        );
        assert!(store.drain_lifecycle_events().is_empty());
        assert!(contact(&mut store, level, id, 0.0));
        assert!(
            matches!(store.get(id).unwrap().data(), EntityData::ItemFrame(frame) if frame.item().is_none())
        );
        assert_eq!(store.len(), 2);
        assert_eq!(store.drain_framed_map_removals().len(), 1);
        let events = level.event_sink.drain_game_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].game_event, VanillaGameEvent::BLOCK_CHANGE);
        assert_eq!(events[0].source_entity_id, None);
    });
}

#[test]
fn hanging_piston_fixed_frame_ignores_move_and_refuses_contact_damage() {
    for name in ["minecraft:cactus", "minecraft:lava"] {
        for glow in [false, true] {
            with_config(crate::tests::datapack_config(), |level| {
                let mut store = batch_store();
                let id = frame(&mut store, glow, true, true);
                block(level, name);
                store.drain_lifecycle_events();
                assert!(contact(&mut store, level, id, 0.1));
                assert_eq!(store.get(id).unwrap().position(), FEET);
                let data = match store.get(id).unwrap().data() {
                    EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => frame,
                    _ => unreachable!(),
                };
                assert!(data.item().is_some());
                assert_eq!(store.len(), 1);
                assert!(store.drain_framed_map_removals().is_empty());
                assert!(level.event_sink.drain_game_events().is_empty());
                assert!(store.drain_lifecycle_events().is_empty());
            });
        }
    }
}

#[test]
fn hanging_piston_zero_motion_lava_removes_the_frame_and_emits_burn_sound() {
    with_config(crate::tests::datapack_config(), |level| {
        let mut store = batch_store();
        let id = frame(&mut store, false, false, false);
        block(level, "minecraft:lava");
        store.drain_lifecycle_events();
        assert!(contact(&mut store, level, id, 0.0));
        assert!(store.get(id).is_none());
        let events = store.drain_lifecycle_events();
        assert_removed_and_drops(&events, id, 1);
        let burn = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
            "minecraft:entity.generic.burn",
        )
        .unwrap();
        assert!(events.iter().any(
            |event| matches!(event, EntityLifecycleEvent::Sound { sound }
            if sound.sound == (SoundEventHolder::Reference { registry_id: burn })
            && (2.0..2.4).contains(&sound.pitch))
        ));
    });
}
