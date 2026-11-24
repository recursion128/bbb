use super::*;
use crate::entity::{EntityData, EntityTickOutcome, ItemEntityData};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_RAVAGER_ID,
};
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const CROP: BlockPos = BlockPos { x: 8, y: 65, z: 8 };
const CROPS: [&str; 6] = [
    "minecraft:wheat",
    "minecraft:carrots",
    "minecraft:potatoes",
    "minecraft:beetroots",
    "minecraft:torchflower_crop",
    "minecraft:pitcher_crop",
];

#[derive(Clone, Copy)]
enum Kind {
    Cow,
    Item,
    Ravager,
}

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn write(level: &mut LevelCtx<'_, '_>, pos: BlockPos, value: i32) {
    if level.block_state(pos) != value {
        assert!(level.write_block(pos, value).is_some());
    }
    assert_eq!(level.block_state(pos), value);
}

fn arena(level: &mut LevelCtx<'_, '_>, crop: &str) {
    level.chunks.try_ensure_served_full_column(0, 0).unwrap();
    assert!(level.chunks.is_ticking_full(0, 0));
    for x in 5..=11 {
        for z in 6..=11 {
            for y in 63..=70 {
                write(level, BlockPos { x, y, z }, state("minecraft:air"));
            }
            write(level, BlockPos { x, y: 64, z }, state("minecraft:farmland"));
        }
    }
    write(level, CROP, state(crop));
}

fn griefing(level: &mut LevelCtx<'_, '_>, value: bool) {
    level
        .game_rules
        .set("minecraft:mob_griefing", GameRuleValue::Boolean(value));
    assert_eq!(LevelCtx::mob_griefing(level), value);
}

fn spawn(store: &mut EntityStore, kind: Kind) -> i32 {
    let x = if matches!(kind, Kind::Ravager) {
        7.0
    } else {
        7.5
    };
    // PitcherCropBlock.java:40-41: enter above both the bulb and older crop collision shapes.
    let position = Vec3d { x, y: 65.5, z: 8.5 };
    let id = match kind {
        Kind::Cow => store
            .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position)
            .unwrap(),
        Kind::Ravager => store
            .spawn_general_living(VANILLA_ENTITY_TYPE_RAVAGER_ID, position)
            .unwrap(),
        Kind::Item => store.spawn(
            VANILLA_ENTITY_TYPE_ITEM_ID,
            position,
            EntityData::Item(
                ItemEntityData::new(
                    crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
                    0.0,
                )
                .with_saved_fields(5, 41, 17, None, None),
            ),
        ),
    };
    let NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    entries.retain(|entry| entry.name != "NoAI" && entry.name != "NoGravity");
    entries.push(NbtSummaryEntry {
        name: "NoGravity".into(),
        value: NbtSummaryValue::Byte(1),
    });
    if !matches!(kind, Kind::Item) {
        entries.push(NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(0),
        });
    }
    store.load_command_data(id, &entries).unwrap();
    let body = store.get_mut(id).unwrap();
    assert!(body.bounding_box().max_x < f64::from(CROP.x));
    body.set_delta_movement(Vec3d {
        x: 8.5 - x,
        y: 0.0,
        z: 0.0,
    });
    store.drain_lifecycle_events();
    id
}

fn complete(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    id: i32,
    kind: Kind,
    crop: &str,
) {
    let health = store
        .get(id)
        .unwrap()
        .living()
        .map(|living| living.health());
    assert!(matches!(
        crate::entity::tick_one_entity(store, level, id),
        EntityTickOutcome::Complete
    ));
    let body = store.get(id).unwrap();
    assert_eq!(
        body.position(),
        Vec3d {
            x: 8.5,
            y: 65.5,
            z: 8.5
        }
    );
    assert_eq!(body.tick_count(), 1);
    assert!(!body.on_ground());
    let initial_x = if matches!(kind, Kind::Ravager) {
        1.5
    } else {
        1.0
    };
    let friction = if matches!(kind, Kind::Item) {
        0.98_f32
    } else {
        0.91_f32
    };
    assert_eq!(body.delta_movement().x, initial_x * f64::from(friction));
    assert_eq!(body.delta_movement().y, 0.0);
    assert_eq!(body.delta_movement().z, 0.0);
    match body.data() {
        EntityData::Item(item) => {
            assert_eq!(item.age(), 42);
            assert_eq!(item.pickup_delay(), 16);
            assert_eq!(item.health(), 5);
        }
        _ => assert_eq!(body.living().map(|living| living.health()), health),
    }
    assert_eq!(level.block_state(CROP), state(crop));
    assert_eq!(
        level.block_state(BlockPos { y: 64, ..CROP }),
        state("minecraft:farmland")
    );
}

#[test]
fn crop_contacts_cow_and_item_cross_crop_and_pitcher_with_griefing_enabled() {
    // CropBlock.java:154-162; PitcherCropBlock.java:120-126: only Ravager enters the destroy branch.
    for crop in CROPS {
        for kind in [Kind::Cow, Kind::Item] {
            with_config(crate::tests::datapack_config(), |level| {
                arena(level, crop);
                griefing(level, true);
                let mut store = batch_store();
                let id = spawn(&mut store, kind);
                complete(&mut store, level, id, kind, crop);
            });
        }
    }
}

#[test]
fn crop_contacts_ravager_crosses_both_crop_classes_when_griefing_disabled() {
    for crop in CROPS {
        with_config(crate::tests::datapack_config(), |level| {
            arena(level, crop);
            griefing(level, false);
            let mut store = batch_store();
            let id = spawn(&mut store, Kind::Ravager);
            complete(&mut store, level, id, Kind::Ravager, crop);
        });
    }
}

#[test]
fn crop_contacts_ravager_breaks_crops_with_drops_when_griefing_enabled() {
    for crop in CROPS {
        with_config(crate::tests::datapack_config(), |level| {
            arena(level, crop);
            let pitcher = crop == "minecraft:pitcher_crop";
            let upper = BlockPos {
                y: CROP.y + 1,
                ..CROP
            };
            if pitcher {
                // PitcherCropBlock.java:35-39; blocks/pitcher_crop.json: age four drops one plant from its lower half.
                write(
                    level,
                    CROP,
                    crate::fluid::with_property(state(crop), "age", "4").unwrap(),
                );
                write(
                    level,
                    upper,
                    crate::fluid::with_property(level.block_state(CROP), "half", "upper").unwrap(),
                );
            }
            griefing(level, true);
            let mut store = batch_store();
            let id = spawn(&mut store, Kind::Ravager);
            assert!(matches!(
                crate::entity::tick_one_entity(&mut store, level, id),
                EntityTickOutcome::Complete
            ));
            let body = store.get(id).unwrap();
            assert!(body.is_alive());
            assert!(body.position().x > f64::from(CROP.x));
            assert!(body.delta_movement().x > 0.0);
            assert_eq!(level.block_state(CROP), state("minecraft:air"));
            assert!(level
                .event_sink
                .recent_game_events()
                .any(|event| event.game_event == VanillaGameEvent::BLOCK_DESTROY
                    && event.source_entity_id == Some(id)
                    && (event.pos == CROP || event.pos == upper)));
            assert!(!level.item_spawns.items.is_empty());
            if pitcher {
                assert_eq!(level.block_state(upper), state("minecraft:air"));
                assert_eq!(
                    level
                        .item_spawns
                        .items
                        .iter()
                        .filter(
                            |drop| drop.item.item_resource_id() == Some("minecraft:pitcher_plant")
                        )
                        .map(|drop| drop.item.count())
                        .sum::<i32>(),
                    1
                );
                assert_eq!(level.item_spawns.items.len(), 1);
            }
        });
    }
}
