use super::*;
use crate::entity::damage::{StoredEntityDamageRules, StoredEntityDamageSource};
use crate::entity::projectile_action::{hurt_stored_entity, StoredHurtRequest, StoredHurtResult};
use crate::entity::{EntityData, ItemFrameData, StoredEntityDamageContext};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
};
use bbb_protocol::NbtSummaryValue;

const FEET: Vec3d = Vec3d {
    x: 8.5,
    y: 65.0,
    z: 8.5,
};
const ATTACHED: BlockPos = BlockPos { x: 8, y: 65, z: 8 };

fn frame(store: &mut EntityStore, glow: bool, chance: f32) -> i32 {
    let map = super::item_orb_hurt::stack(
        "minecraft:filled_map",
        vec![("minecraft:map_id", NbtSummaryValue::Int(7))],
    );
    let data = ItemFrameData::new(ATTACHED, Direction::North).with_saved_fields(
        Some(map),
        0,
        chance,
        false,
        false,
    );
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

fn hurt(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32, explosion: bool) -> bool {
    let resource = if explosion {
        "minecraft:explosion"
    } else {
        "minecraft:cactus"
    };
    let damage = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == resource)
        .unwrap();
    let tags = level.capture_registry_tags();
    let result = hurt_stored_entity(
        StoredHurtRequest {
            direct_entity_snapshot: None,
            target_entity_id: id,
            damage_type_resource_id: resource,
            amount: 1.0,
            direct_resource_id: None,
            registry_tags: &tags,
            causing_entity_snapshot: None,
            source_position: None,
            explicit_source_position: None,
        },
        None,
        StoredEntityDamageSource::without_attacker_views(damage, None, None),
        StoredEntityDamageRules {
            entity_drops: true,
            ..Default::default()
        },
        StoredEntityDamageContext::default(),
        store,
        level,
    );
    matches!(result,
        StoredHurtResult::Applied { outcome, effect_applied: false } if outcome.succeeded)
}

#[test]
fn hanging_drops_live_item_removal_clears_map_and_emits_block_change() {
    for glow in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            let mut store = batch_store();
            let id = frame(&mut store, glow, 0.0);
            store.drain_lifecycle_events();
            assert!(hurt(&mut store, level, id, false));
            let data = match store.get(id).unwrap().data() {
                EntityData::ItemFrame(data) | EntityData::GlowItemFrame(data) => data,
                _ => unreachable!(),
            };
            assert!(data.item().is_none());
            assert_eq!(store.len(), 1);
            let removals = store.drain_framed_map_removals();
            assert_eq!(removals.len(), 1);
            assert_eq!(removals[0].map_id, 7);
            assert_eq!(removals[0].entity_id, id);
            let events = level.event_sink.drain_game_events();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].game_event, VanillaGameEvent::BLOCK_CHANGE);
        });
    }
}

#[test]
fn hanging_drops_support_loss_publishes_frame_and_map_items() {
    with_config(crate::tests::datapack_config(), |level| {
        let mut store = batch_store();
        let id = frame(&mut store, false, 1.0);
        store.drain_lifecycle_events();
        for _ in 0..100 {
            crate::entity::tick_one_entity(&mut store, level, id);
        }
        assert!(store.get(id).is_some());
        crate::entity::tick_one_entity(&mut store, level, id);
        assert!(store.get(id).is_none());
        assert_eq!(store.len(), 2);
        let names: Vec<_> = store
            .ids()
            .into_iter()
            .map(|child| {
                let EntityData::Item(item) = store.get(child).unwrap().data() else {
                    panic!("non-item drop")
                };
                item.item().item_resource_id().unwrap().to_owned()
            })
            .collect();
        assert_eq!(names, vec!["minecraft:item_frame", "minecraft:filled_map"]);
        assert_eq!(store.drain_framed_map_removals().len(), 1);
        let events = level.event_sink.drain_game_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].game_event, VanillaGameEvent::BLOCK_CHANGE);
    });
}
