use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::EntityStore;
use crate::entity::{EntityData, EntityIdCounter, GeneralLivingData};
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::ServerWorld;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::Java26BuiltinDimension;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

/// One mob hanging in loaded air with `fields` merged over its record.
fn fixture(entity_type_id: i32, fields: &[(&str, NbtSummaryValue)]) -> (ServerWorld, i32) {
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "gift_loot", 1);
    world.level_mut(DIM).chunks.load_column(0, 0);
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        entity_type_id,
        Vec3d {
            x: 8.5,
            y: 128.0,
            z: 8.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(entity_type_id).unwrap()),
    );
    let NbtSummaryValue::Compound(mut record) =
        world.stored_entity_command_data_for_test(DIM, id).unwrap()
    else {
        panic!("record");
    };
    let fields = [
        ("NoGravity", NbtSummaryValue::Byte(1)),
        ("PersistenceRequired", NbtSummaryValue::Byte(1)),
    ]
    .into_iter()
    .chain(fields.iter().cloned());
    for (name, value) in fields {
        record.retain(|field| field.name != name);
        record.push(NbtSummaryEntry {
            name: name.into(),
            value,
        });
    }
    world.load_stored_entity_record_for_test(DIM, id, &record);
    (world, id)
}

fn items(world: &ServerWorld) -> Vec<String> {
    world
        .stored_items_for_test(DIM)
        .into_iter()
        .map(|(_, _, item, _, _, _)| item)
        .collect()
}

fn int_field(world: &ServerWorld, id: i32, name: &str) -> i32 {
    let NbtSummaryValue::Compound(record) =
        world.stored_entity_command_data_for_test(DIM, id).unwrap()
    else {
        panic!("record");
    };
    match record
        .iter()
        .find(|field| field.name == name)
        .map(|f| &f.value)
    {
        Some(NbtSummaryValue::Int(value)) => *value,
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn chicken_lays_its_variant_egg_and_redraws_the_timer() {
    for (variant, egg) in [
        ("minecraft:temperate", "minecraft:egg"),
        ("minecraft:warm", "minecraft:brown_egg"),
    ] {
        let (mut world, id) = fixture(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            &[
                ("NoAI", NbtSummaryValue::Byte(1)),
                ("EggLayTime", NbtSummaryValue::Int(2)),
                ("variant", NbtSummaryValue::String(variant.into())),
            ],
        );
        world.tick(Vec::new());
        assert!(items(&world).is_empty());
        assert_eq!(int_field(&world, id, "EggLayTime"), 1);
        world.tick(Vec::new());
        assert_eq!(items(&world), [egg]);
        assert!((6000..12000).contains(&int_field(&world, id, "EggLayTime")));
    }
}

#[test]
fn baby_and_jockey_chickens_hold_their_eggs() {
    for field in [
        ("Age", NbtSummaryValue::Int(-24000)),
        ("IsChickenJockey", NbtSummaryValue::Byte(1)),
    ] {
        let (mut world, id) = fixture(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            &[
                ("NoAI", NbtSummaryValue::Byte(1)),
                ("EggLayTime", NbtSummaryValue::Int(1)),
                field,
            ],
        );
        for _ in 0..3 {
            world.tick(Vec::new());
        }
        assert!(items(&world).is_empty());
        assert_eq!(int_field(&world, id, "EggLayTime"), 1);
    }
}

#[test]
fn adult_and_baby_armadillos_shed_a_scute() {
    for age in [0, -24000] {
        let (mut world, id) = fixture(
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            &[
                ("scute_time", NbtSummaryValue::Int(1)),
                ("Age", NbtSummaryValue::Int(age)),
            ],
        );
        world.tick(Vec::new());
        assert_eq!(items(&world), ["minecraft:armadillo_scute"], "age {age}");
        assert!((6000..12000).contains(&int_field(&world, id, "scute_time")));
    }
}

#[test]
fn baby_turtle_drops_a_scute_when_it_grows_up() {
    let (mut world, id) = fixture(
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        &[
            ("NoAI", NbtSummaryValue::Byte(1)),
            ("Age", NbtSummaryValue::Int(-2)),
        ],
    );
    world.tick(Vec::new());
    assert!(items(&world).is_empty());
    world.tick(Vec::new());
    assert_eq!(items(&world), ["minecraft:turtle_scute"]);
    assert_eq!(int_field(&world, id, "Age"), 0);
}

#[test]
fn gift_timers_survive_a_chunk_save() {
    for (entity_type_id, name) in [
        (VANILLA_ENTITY_TYPE_CHICKEN_ID, "EggLayTime"),
        (VANILLA_ENTITY_TYPE_ARMADILLO_ID, "scute_time"),
    ] {
        let mut data = GeneralLivingData::new(entity_type_id).unwrap();
        *data.gift_drop_time_mut().unwrap() = 1234;
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        store.spawn(
            entity_type_id,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: 1.0,
            },
            EntityData::GeneralLiving(data),
        );
        let saved = write_entity_chunk(&store, (0, 0)).unwrap();
        let mut reloaded = EntityStore::new(2, EntityIdCounter::new());
        let id = read_entity_chunk(&mut reloaded, &saved).unwrap()[0];
        let resaved = write_entity_chunk(&reloaded, (0, 0)).unwrap();
        let fields = resaved.entities().next().unwrap().raw();
        let values: Vec<_> = fields.iter().filter(|field| field.name == name).collect();
        assert_eq!(values.len(), 1, "{name}");
        assert_eq!(values[0].value, NbtSummaryValue::Int(1234));
        let EntityData::GeneralLiving(mut data) = reloaded.get(id).unwrap().data().clone() else {
            unreachable!();
        };
        assert_eq!(data.gift_drop_time_mut().copied(), Some(1234));
    }
}
