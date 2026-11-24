//! One hive occupant, created fresh for `addFreshEntity`.

use crate::block_entity::beehive::BeehiveReleaseEntry;
use crate::random::GameRandom;
use bbb_protocol::packets::{BlockPos, Vec3d};

use super::{load_entity_creation, Entity, EntityNbtReadContext, EntitySaveError};
use crate::entity::EntityStore;

const IGNORED_BEE_TAGS: [&str; 26] = [
    "Air",
    "drop_chances",
    "equipment",
    "Brain",
    "CanPickUpLoot",
    "DeathTime",
    "fall_distance",
    "FallFlying",
    "Fire",
    "HurtByTimestamp",
    "HurtTime",
    "LeftHanded",
    "Motion",
    "NoGravity",
    "OnGround",
    "PortalCooldown",
    "Pos",
    "Rotation",
    "sleeping_pos",
    "CannotEnterHiveTicks",
    "TicksSincePollination",
    "CropsGrownSincePollination",
    "hive_pos",
    "Passengers",
    "leash",
    "UUID",
];

/// `Occupant.of` (`BeehiveBlockEntity.java:386-395`).
pub(crate) fn hive_occupant_record(
    entity: &Entity,
    store: &EntityStore,
) -> Result<Vec<bbb_protocol::NbtSummaryEntry>, EntitySaveError> {
    let mut record = super::entity_record(entity, false, Some(store))?;
    record.retain(|entry| !IGNORED_BEE_TAGS.iter().any(|name| entry.name == *name));
    Ok(record)
}

/// `Occupant.createEntity` (`BeehiveBlockEntity.java:401-429`). The caller
/// snaps the fresh body to the exit and adds it after the release sound and
/// game event have been raised.
pub fn create_beehive_occupant(
    store: &mut EntityStore,
    occupant: &BeehiveReleaseEntry,
    hive_pos: BlockPos,
    saved_flower_pos: Option<BlockPos>,
    world_random: &GameRandom,
    is_inhabitor: bool,
) -> Result<Option<Box<Entity>>, EntitySaveError> {
    create_beehive_occupant_with_optional_context(
        store,
        occupant,
        hive_pos,
        saved_flower_pos,
        world_random,
        is_inhabitor,
        None,
    )
}

pub fn create_beehive_occupant_with_context(
    store: &mut EntityStore,
    occupant: &BeehiveReleaseEntry,
    hive_pos: BlockPos,
    saved_flower_pos: Option<BlockPos>,
    world_random: &GameRandom,
    is_inhabitor: bool,
    context: EntityNbtReadContext<'_>,
) -> Result<Option<Box<Entity>>, EntitySaveError> {
    create_beehive_occupant_with_optional_context(
        store,
        occupant,
        hive_pos,
        saved_flower_pos,
        world_random,
        is_inhabitor,
        Some(context),
    )
}

fn create_beehive_occupant_with_optional_context(
    store: &mut EntityStore,
    occupant: &BeehiveReleaseEntry,
    hive_pos: BlockPos,
    saved_flower_pos: Option<BlockPos>,
    world_random: &GameRandom,
    is_inhabitor: bool,
    context: Option<EntityNbtReadContext<'_>>,
) -> Result<Option<Box<Entity>>, EntitySaveError> {
    let record = occupant
        .entity_data
        .iter()
        .filter(|entry| !IGNORED_BEE_TAGS.iter().any(|name| entry.name == *name))
        .cloned()
        .collect::<Vec<_>>();
    // `Entity` starts at the origin; only Bee receives the exit snap below.
    let id = load_entity_creation(store, &record, Vec3d::default(), context)?;
    let mut entity = store
        .take_fresh(id)
        .expect("created hive occupant is fresh");
    if !is_inhabitor {
        store.discard_fresh(entity);
        return Ok(None);
    }
    entity.set_no_gravity(true);
    crate::entity::bee::prepare_hive_release(
        &mut entity,
        hive_pos,
        occupant.ticks_in_hive,
        saved_flower_pos,
        world_random,
    );
    Ok(Some(entity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityData, EntityIdCounter};
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    fn field(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn occupant(id: &str, extra: Vec<NbtSummaryEntry>, ticks: i32) -> BeehiveReleaseEntry {
        let mut entity_data = vec![field("id", NbtSummaryValue::String(id.into()))];
        entity_data.extend(extra);
        BeehiveReleaseEntry {
            saved: NbtSummaryValue::Compound(Vec::new()),
            entity_resource_id: id.into(),
            entity_data,
            ticks_in_hive: ticks,
            min_ticks_in_hive: 600,
        }
    }

    #[test]
    fn bee_load_strips_ignored_identity_and_preserves_love_cause_while_aging() {
        let mut store = EntityStore::new(0x42_45_45, EntityIdCounter::new());
        let old_uuid = NbtSummaryValue::IntArray(vec![0, 0, 0, 1]);
        let bee = occupant(
            "minecraft:bee",
            vec![
                field("UUID", old_uuid.clone()),
                field("Age", NbtSummaryValue::Int(-40)),
                field("InLove", NbtSummaryValue::Int(80)),
                field("LoveCause", old_uuid),
                field("NoGravity", NbtSummaryValue::Byte(0)),
                field(
                    "Pos",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Double(123.0_f64.to_bits()); 3]),
                ),
            ],
            25,
        );
        let hive_pos = BlockPos { x: 7, y: 81, z: 9 };
        let random = GameRandom::with_seed(22);
        let mut staged = create_beehive_occupant(&mut store, &bee, hive_pos, None, &random, true)
            .unwrap()
            .unwrap();
        assert_ne!(staged.uuid().as_u128(), 1);
        assert_eq!(staged.position(), Vec3d::default());
        assert!(staged.no_gravity());
        let EntityData::GeneralLiving(data) = &mut staged.data else {
            panic!("Bee living data")
        };
        assert_eq!(data.ageable_retained_mut().unwrap().age(), -15);
        let love = data.animal_retained().unwrap();
        assert_eq!(love.in_love(), 55);
        assert_eq!(love.love_cause().unwrap().as_u128(), 1);
        let record = super::super::entity_record(&staged, false, Some(&store)).unwrap();
        assert_eq!(
            record
                .iter()
                .find(|entry| entry.name == "hive_pos")
                .map(|entry| &entry.value),
            Some(&NbtSummaryValue::IntArray(vec![7, 81, 9]))
        );
    }

    #[test]
    fn age_lock_and_existing_flower_are_kept() {
        let mut store = EntityStore::new(7, EntityIdCounter::new());
        let bee = occupant(
            "minecraft:bee",
            vec![
                field("Age", NbtSummaryValue::Int(-40)),
                field("AgeLocked", NbtSummaryValue::Byte(1)),
                field("flower_pos", NbtSummaryValue::IntArray(vec![1, 2, 3])),
            ],
            25,
        );
        let random = GameRandom::with_seed(91);
        let mut staged = create_beehive_occupant(
            &mut store,
            &bee,
            BlockPos { x: 7, y: 81, z: 9 },
            Some(BlockPos { x: 4, y: 5, z: 6 }),
            &random,
            true,
        )
        .unwrap()
        .unwrap();
        let EntityData::GeneralLiving(data) = &mut staged.data else {
            panic!("Bee living data")
        };
        assert_eq!(data.ageable_retained_mut().unwrap().age(), -40);
        let record = super::super::entity_record(&staged, false, Some(&store)).unwrap();
        assert_eq!(
            record
                .iter()
                .find(|entry| entry.name == "flower_pos")
                .map(|entry| &entry.value),
            Some(&NbtSummaryValue::IntArray(vec![1, 2, 3]))
        );
    }

    #[test]
    fn missing_flower_takes_the_hive_flower_and_age_transition_changes_exit_dimensions() {
        let mut store = EntityStore::new(7, EntityIdCounter::new());
        let adult = occupant(
            "minecraft:bee",
            vec![field("Age", NbtSummaryValue::Int(-1))],
            1,
        );
        let baby = occupant(
            "minecraft:bee",
            vec![field("Age", NbtSummaryValue::Int(-40))],
            1,
        );
        let flower = BlockPos { x: 4, y: 5, z: 6 };
        let random = GameRandom::with_seed(0);
        let expected = random.clone();
        assert!(expected.next_float() < 0.9);
        let staged_adult =
            create_beehive_occupant(&mut store, &adult, flower, Some(flower), &random, true)
                .unwrap()
                .unwrap();
        let record = super::super::entity_record(&staged_adult, false, Some(&store)).unwrap();
        assert_eq!(
            record
                .iter()
                .find(|entry| entry.name == "flower_pos")
                .map(|entry| &entry.value),
            Some(&NbtSummaryValue::IntArray(vec![4, 5, 6]))
        );
        let staged_baby = create_beehive_occupant(&mut store, &baby, flower, None, &random, true)
            .unwrap()
            .unwrap();
        assert!(staged_adult.bb_width() > staged_baby.bb_width());
        assert!(staged_adult.bb_height() > staged_baby.bb_height());
    }
}
