use super::*;
use crate::entity::{EntityData, FallingBlockData};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const LAND: BlockPos = BlockPos { x: 8, y: 64, z: 8 };

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn payload() -> Vec<NbtSummaryEntry> {
    vec![
        entry("id", NbtSummaryValue::String("minecraft:banner".into())),
        entry("x", NbtSummaryValue::Int(-99)),
        entry(
            "Items",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("Slot", NbtSummaryValue::Byte(0)),
                entry("id", NbtSummaryValue::String("minecraft:apple".into())),
                entry("count", NbtSummaryValue::Int(5)),
            ])]),
        ),
    ]
}

fn spawn(store: &mut EntityStore, name: &str, payload: Option<Vec<NbtSummaryEntry>>) -> i32 {
    let id = store.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 8.5,
            y: 64.2,
            z: 8.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(state(name), LAND).with_block_data(payload)),
    );
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    });
    id
}

fn floor(level: &mut LevelCtx<'_, '_>) {
    level.write_block(LAND, 0).unwrap();
    level
        .write_block(BlockPos { y: 63, ..LAND }, state("minecraft:stone"))
        .unwrap();
}

#[test]
fn falling_block_data_actual_tick_preserves_chest_position_state_and_items() {
    with_config(crate::tests::datapack_config(), |level| {
        floor(level);
        let mut store = batch_store();
        let id = spawn(&mut store, "minecraft:chest", Some(payload()));
        crate::entity::tick_one_entity(&mut store, level, id);
        assert!(store.get(id).is_none());
        let block_entity = level.chunks.block_entities().get(LAND).unwrap();
        assert_eq!(block_entity.block_pos(), LAND);
        assert_eq!(block_entity.block_state(), state("minecraft:chest"));
        assert_eq!(block_entity.chest().unwrap().items()[0].count(), 5);
        assert_eq!(
            block_entity.chest().unwrap().items()[0].item_id(),
            Some(bbb_protocol::item_types::VANILLA_ITEM_APPLE_ID)
        );
        assert!(level.chunks.save_dirty_epoch((0, 0)).is_some());
    });
}

#[test]
fn falling_block_data_sand_lands_without_loading_block_entity_payload() {
    with_config(crate::tests::datapack_config(), |level| {
        floor(level);
        let mut store = batch_store();
        let id = spawn(&mut store, "minecraft:sand", Some(payload()));
        crate::entity::tick_one_entity(&mut store, level, id);
        assert!(store.get(id).is_none());
        assert_eq!(level.block_state(LAND), state("minecraft:sand"));
        assert!(level.chunks.block_entities().get(LAND).is_none());
    });
}
