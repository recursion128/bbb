//! `SpawnerBlockEntity` (`SpawnerBlockEntity.java:20-86`) over the shared
//! [`BaseSpawnerData`] state machine.

use bbb_protocol::NbtSummaryEntry;

use super::base_spawner::BaseSpawnerData;

/// `SpawnerBlockEntity.getUpdateTag`'s omission (`SpawnerBlockEntity.java:66-69`).
const TAG_SPAWN_POTENTIALS: &str = "SpawnPotentials";

/// `SpawnerBlockEntity` (`SpawnerBlockEntity.java:20-35`) over `BaseSpawner`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpawnerBlockEntity {
    spawner: BaseSpawnerData,
}

impl SpawnerBlockEntity {
    /// `SpawnerBlockEntity.setEntityId` (`SpawnerBlockEntity.java:78-81`) over
    /// `BaseSpawner.setEntityId` (`BaseSpawner.java:55-57`) without the draw
    /// `getOrCreateNextSpawnData` would take, which is what the worldgen
    /// feature has to answer with; [`Self::set_entity_id_drawing`] is the
    /// level-backed half.
    pub fn set_entity_id(&mut self, resource_id: &str) {
        self.spawner
            .next_spawn_data_mut()
            .set_entity_resource_id(resource_id);
    }

    pub fn set_entity_id_drawing(&mut self, resource_id: &str, random: &crate::random::GameRandom) {
        self.spawner.set_entity_id(resource_id, random);
    }

    /// The state one `BaseSpawner.serverTick` reads and writes back.
    pub fn spawner_data(&self) -> &BaseSpawnerData {
        &self.spawner
    }

    pub fn spawner_data_mut(&mut self) -> &mut BaseSpawnerData {
        &mut self.spawner
    }

    /// `SpawnerBlockEntity.getUpdateTag` (`SpawnerBlockEntity.java:66-69`):
    /// `saveCustomOnly` minus `SpawnPotentials`.
    pub(super) fn save_items(&self) -> Vec<NbtSummaryEntry> {
        self.save_additional()
            .into_iter()
            .filter(|entry| entry.name != TAG_SPAWN_POTENTIALS)
            .collect()
    }

    /// `BaseSpawner.save` (`BaseSpawner.java:214-224`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = Vec::new();
        self.spawner.write_save_data(&mut entries);
        entries
    }

    /// `BaseSpawner.load` (`BaseSpawner.java:200-212`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.spawner = BaseSpawnerData::read_save_data(record);
    }

    /// `SpawnerBlockEntity.triggerEvent` (`SpawnerBlockEntity.java:73-75`)
    /// over `BaseSpawner.onEventTriggered` (`BaseSpawner.java:241-250`); the
    /// client-side delay reset belongs to a client this server does not run.
    pub fn trigger_event(&mut self, id: i32, data: i32) -> bool {
        let _ = data;
        u8::try_from(id).is_ok_and(|id| self.spawner.handle_event(id, false))
    }
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_SPAWNER_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::NbtSummaryValue;

    use crate::block_entity::entry;

    use super::TAG_SPAWN_POTENTIALS;

    use crate::block_entity::{new_block_entity, unique_field};

    const TAG_SPAWN_DATA: &str = "SpawnData";
    const TAG_DELAY: &str = "Delay";
    const TAG_MIN_SPAWN_DELAY: &str = "MinSpawnDelay";
    const TAG_MAX_SPAWN_DELAY: &str = "MaxSpawnDelay";
    const TAG_SPAWN_COUNT: &str = "SpawnCount";
    const TAG_MAX_NEARBY_ENTITIES: &str = "MaxNearbyEntities";
    const TAG_REQUIRED_PLAYER_RANGE: &str = "RequiredPlayerRange";
    const TAG_SPAWN_RANGE: &str = "SpawnRange";

    fn pos() -> BlockPos {
        BlockPos { x: 1, y: 2, z: 3 }
    }

    fn spawner_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_SPAWNER_ID)
            .expect("minecraft:spawner has a default state")
    }

    fn zombie_spawn_data() -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![entry(
            "entity",
            NbtSummaryValue::Compound(vec![entry(
                "id",
                NbtSummaryValue::string("minecraft:zombie"),
            )]),
        )])
    }

    fn potentials() -> NbtSummaryValue {
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            entry("data", zombie_spawn_data()),
            entry("weight", NbtSummaryValue::Int(2)),
        ])])
    }

    /// `WeightedList.of(spawnData)` (`WeightedList.java:41-43`) through
    /// `Weighted.codec`.
    fn weighted_spawn_data(data: NbtSummaryValue) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry("data", data),
            entry("weight", NbtSummaryValue::Int(1)),
        ])
    }

    #[test]
    fn a_spawner_round_trips_owned_base_spawner_fields() {
        let state = spawner_state();
        let mut record = new_block_entity(pos(), state)
            .expect("a spawner builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        for (name, value) in [
            (TAG_DELAY, NbtSummaryValue::Short(7)),
            (TAG_MIN_SPAWN_DELAY, NbtSummaryValue::Short(40)),
            (TAG_MAX_SPAWN_DELAY, NbtSummaryValue::Short(80)),
            (TAG_SPAWN_COUNT, NbtSummaryValue::Short(2)),
            (TAG_MAX_NEARBY_ENTITIES, NbtSummaryValue::Short(3)),
            (TAG_REQUIRED_PLAYER_RANGE, NbtSummaryValue::Short(8)),
            (TAG_SPAWN_RANGE, NbtSummaryValue::Short(5)),
        ] {
            record.retain(|entry| entry.name != name);
            record.push(entry(name, value));
        }
        record.retain(|entry| entry.name != TAG_SPAWN_DATA && entry.name != TAG_SPAWN_POTENTIALS);
        record.push(entry(TAG_SPAWN_DATA, zombie_spawn_data()));
        record.push(entry(TAG_SPAWN_POTENTIALS, potentials()));
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("a spawner loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let again = crate::block_entity::load_test_fixture(pos(), state, &saved)
            .expect("fixture codec succeeds")
            .expect("the round-trip still names a spawner")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        for name in [
            TAG_DELAY,
            TAG_MIN_SPAWN_DELAY,
            TAG_MAX_SPAWN_DELAY,
            TAG_SPAWN_COUNT,
            TAG_MAX_NEARBY_ENTITIES,
            TAG_REQUIRED_PLAYER_RANGE,
            TAG_SPAWN_RANGE,
            TAG_SPAWN_DATA,
            TAG_SPAWN_POTENTIALS,
        ] {
            assert_eq!(
                unique_field(&saved, name),
                unique_field(&again, name),
                "{name} must round-trip through the spawner owner"
            );
        }
        assert_eq!(
            unique_field(&saved, TAG_SPAWN_DATA),
            Some(&zombie_spawn_data())
        );
        assert_eq!(
            unique_field(&saved, TAG_SPAWN_POTENTIALS),
            Some(&potentials())
        );
        assert_eq!(
            unique_field(&saved, TAG_DELAY),
            Some(&NbtSummaryValue::Short(7))
        );
        assert_eq!(
            unique_field(
                &loaded
                    .update_tag_with_resolver(crate::block_entity::test_item_resolver())
                    .expect("fixture codec succeeds"),
                TAG_SPAWN_POTENTIALS
            ),
            None
        );
        assert_eq!(
            unique_field(
                &loaded
                    .update_tag_with_resolver(crate::block_entity::test_item_resolver())
                    .expect("fixture codec succeeds"),
                TAG_SPAWN_DATA
            ),
            Some(&zombie_spawn_data())
        );
    }

    #[test]
    fn missing_spawn_potentials_are_reconstructed_from_spawn_data() {
        let state = spawner_state();
        let mut record = new_block_entity(pos(), state)
            .expect("a spawner builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        record.retain(|entry| entry.name != TAG_SPAWN_POTENTIALS && entry.name != TAG_SPAWN_DATA);
        record.push(entry(TAG_SPAWN_DATA, zombie_spawn_data()));
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("a spawner loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(
            unique_field(&saved, TAG_SPAWN_POTENTIALS),
            Some(&NbtSummaryValue::List(vec![weighted_spawn_data(
                zombie_spawn_data()
            )]))
        );
    }

    #[test]
    fn worldgen_entity_id_mutation_round_trips_without_rng() {
        let state = spawner_state();
        let pos = pos();
        let mut built = new_block_entity(pos, state).unwrap();
        built
            .spawner_mut()
            .unwrap()
            .set_entity_id("minecraft:zombie");
        let saved = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let spawn_data = unique_field(&saved, TAG_SPAWN_DATA).unwrap();
        assert_eq!(
            unique_field(
                match spawn_data {
                    NbtSummaryValue::Compound(fields) => fields,
                    _ => panic!("spawn data is not a compound"),
                },
                "entity"
            ),
            Some(&NbtSummaryValue::Compound(vec![entry(
                "id",
                NbtSummaryValue::String("minecraft:zombie".into()),
            )]))
        );
        let loaded = crate::block_entity::load_test_fixture(pos, state, &saved)
            .expect("fixture codec succeeds")
            .unwrap();
        assert_eq!(
            loaded
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver(),)
                .expect("fixture codec succeeds"),
            saved
        );
        assert!(loaded.chest().is_none());
    }
}
