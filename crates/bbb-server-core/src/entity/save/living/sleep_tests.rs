use super::*;
use crate::entity::store::EntityIdCounter;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::packets::BlockPos;

fn entry(value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: "sleeping_pos".into(),
        value,
    }
}

fn fixture(armor_stand: bool) -> (EntityStore, i32) {
    let mut store = EntityStore::new(47, EntityIdCounter::new());
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let id = if armor_stand {
        crate::entity::spawn_armor_stand(&mut store, position)
    } else {
        store.spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            position,
        )
    }
    .unwrap();
    (store, id)
}

fn command_entries(store: &EntityStore, id: i32) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(entries) = store.command_data(id).unwrap() else {
        panic!("command data is a compound")
    };
    entries
}

fn assert_saved(entries: &[NbtSummaryEntry], expected: Option<BlockPos>) {
    let fields: Vec<_> = entries
        .iter()
        .filter(|entry| entry.name == "sleeping_pos")
        .collect();
    if let Some(position) = expected {
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].value, write_block_pos(position));
    } else {
        assert!(fields.is_empty());
    }
}

#[test]
fn stored_sleep_save_cold_reload_preserves_position_and_sets_pose() {
    let bed = BlockPos {
        x: -17,
        y: 70,
        z: 33,
    };
    for armor_stand in [false, true] {
        let (mut store, id) = fixture(armor_stand);
        let position = store.get(id).unwrap().position();
        store.get_mut(id).unwrap().set_sleeping_pos(Some(bed));
        assert_saved(&command_entries(&store, id), Some(bed));
        let chunk = write_entity_chunk(&store, (0, 0)).unwrap();
        assert_saved(chunk.entities().next().unwrap().raw(), Some(bed));
        let mut loaded = EntityStore::new(53, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut loaded, &chunk).unwrap();
        let entity = loaded.get(ids[0]).unwrap();
        assert_eq!(entity.sleeping_pos(), Some(bed));
        assert_eq!(entity.pose_id(), VANILLA_ENTITY_POSE_SLEEPING_ID);
        assert_eq!(entity.position(), position);
        assert!(entity.is_first_tick());
        assert_saved(&command_entries(&loaded, ids[0]), Some(bed));
    }
}

#[test]
fn stored_sleep_save_command_applies_bed_position_only_after_first_tick() {
    let bed = BlockPos {
        x: -17,
        y: 70,
        z: 33,
    };
    for armor_stand in [false, true] {
        for first_tick in [true, false] {
            let (mut store, id) = fixture(armor_stand);
            store.get_mut(id).unwrap().first_tick = first_tick;
            let original = store.get(id).unwrap().position();
            let mut entries = command_entries(&store, id);
            entries.push(entry(write_block_pos(bed)));
            store.load_command_data(id, &entries).unwrap();
            let entity = store.get(id).unwrap();
            assert_eq!(entity.sleeping_pos(), Some(bed));
            assert_eq!(entity.pose_id(), VANILLA_ENTITY_POSE_SLEEPING_ID);
            assert_eq!(entity.is_first_tick(), first_tick);
            assert_eq!(
                entity.position(),
                if first_tick {
                    original
                } else {
                    Vec3d {
                        x: -16.5,
                        y: 70.6875,
                        z: 33.5,
                    }
                }
            );
            assert_saved(&command_entries(&store, id), Some(bed));
        }
    }
}

#[test]
fn stored_sleep_save_command_clear_keeps_pose_and_drops_old_opaque_value() {
    for armor_stand in [false, true] {
        let (mut store, id) = fixture(armor_stand);
        let mut entries = command_entries(&store, id);
        entries.push(entry(write_block_pos(BlockPos { x: 1, y: 70, z: 3 })));
        store.load_command_data(id, &entries).unwrap();
        entries.retain(|field| field.name != "sleeping_pos");
        store.load_command_data(id, &entries).unwrap();
        let entity = store.get(id).unwrap();
        assert_eq!(entity.sleeping_pos(), None);
        assert_eq!(entity.pose_id(), VANILLA_ENTITY_POSE_SLEEPING_ID);
        assert_saved(&command_entries(&store, id), None);
        let chunk = write_entity_chunk(&store, (0, 0)).unwrap();
        assert_saved(chunk.entities().next().unwrap().raw(), None);
        let mut loaded = EntityStore::new(53, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut loaded, &chunk).unwrap();
        assert_eq!(loaded.get(ids[0]).unwrap().sleeping_pos(), None);
        assert_eq!(
            loaded.get(ids[0]).unwrap().pose_id(),
            VANILLA_ENTITY_POSE_STANDING_ID
        );
    }
}
