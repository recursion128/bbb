use super::Vec3d;
use crate::entity::{EntityData, GeneralLivingData};
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::ServerWorld;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_DYING_ID, VANILLA_ENTITY_POSE_SITTING_ID, VANILLA_ENTITY_POSE_SLEEPING_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::Java26BuiltinDimension;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn fixture() -> (ServerWorld, i32) {
    let config = crate::tests::datapack_config_with_loot_table(
        "minecraft:entities/camel",
        &serde_json::json!({
            "type":"minecraft:entity", "random_sequence":"test:death_growth",
            "pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:diamond",
            "functions":[{"function":"minecraft:set_count","count":{"type":"minecraft:uniform","min":1,"max":4}}]}]}]
        }),
    );
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "death_growth", 1);
    world.level_mut(DIM).chunks.load_column(0, 0);
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        Vec3d {
            x: 8.5,
            y: 128.2,
            z: 8.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_CAMEL_ID).unwrap()),
    );
    let NbtSummaryValue::Compound(mut record) =
        world.stored_entity_command_data_for_test(DIM, id).unwrap()
    else {
        panic!("record");
    };
    for name in ["NoAI", "NoGravity", "PersistenceRequired"] {
        record.retain(|field| field.name != name);
        record.push(NbtSummaryEntry {
            name: name.into(),
            value: NbtSummaryValue::Byte(1),
        });
    }
    record.push(NbtSummaryEntry {
        name: "last_hurt_by_player".into(),
        value: NbtSummaryValue::IntArray(vec![0, 0, 0, 1]),
    });
    record.push(NbtSummaryEntry {
        name: "last_hurt_by_player_memory_time".into(),
        value: NbtSummaryValue::Int(100),
    });
    world.load_stored_entity_record_for_test(DIM, id, &record);
    world.tick(Vec::new());
    let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
    assert!(!body.is_first_tick());
    body.set_pose_id(VANILLA_ENTITY_POSE_SITTING_ID);
    world.level_mut(DIM).entities.drain_lifecycle_events();
    (world, id)
}

fn hurt(world: &mut ServerWorld, id: i32) -> Option<crate::entity::StoredEntityHurtOutcome> {
    world.hurt_stored_entity_by_command(
        DIM.into(),
        id,
        "minecraft:generic",
        1000.0,
        None,
        None,
        None,
        &mut Vec::new(),
    )
}

#[test]
fn death_growth_world_lethal_sitting_camel_moves_and_refiles() {
    let (mut world, id) = fixture();
    let body = world.stored_entity_for_test(DIM, id).unwrap();
    let (old_y, old_height) = (body.position().y, body.bb_height());
    assert!(hurt(&mut world, id).is_some_and(|outcome| outcome.succeeded));
    let body = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(body.pose_id(), VANILLA_ENTITY_POSE_DYING_ID);
    assert!(body.living().unwrap().is_dead_or_dying());
    assert!(body.bb_height() > old_height);
    assert!(
        (body.position().y - (old_y + f64::from(old_height - body.bb_height()) / 2.0)).abs()
            < 1.0e-6
    );
    assert_eq!(body.bounding_box().min_y, body.position().y);
    assert_eq!(
        body.filed_section(),
        crate::entity::SectionKey::new(0, 7, 0)
    );
    assert!(world
        .level(DIM)
        .entities
        .ids_in_section(crate::entity::SectionKey::new(0, 7, 0))
        .contains(&id));
    assert!(!world
        .level(DIM)
        .entities
        .ids_in_section(crate::entity::SectionKey::new(0, 8, 0))
        .contains(&id));
    assert!(world
        .level(DIM)
        .entities
        .ids()
        .iter()
        .any(|other| *other != id));
    let drops: Vec<_> = world
        .level(DIM)
        .entities
        .ids()
        .into_iter()
        .filter_map(|other| world.stored_entity_for_test(DIM, other))
        .filter(|body| matches!(body.data(), EntityData::Item(_)))
        .collect();
    assert!(!drops.is_empty());
    assert!(world
        .level(DIM)
        .entities
        .ids()
        .into_iter()
        .filter_map(|other| world.stored_entity_for_test(DIM, other))
        .any(|body| matches!(body.data(), EntityData::ExperienceOrb(_))));
}

#[test]
fn death_growth_world_external_die_releases_real_occupied_bed_before_dying() {
    use bbb_protocol::packets::BlockPos;
    let (mut world, id) = fixture();
    let head = BlockPos { x: 8, y: 128, z: 8 };
    let stone = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:stone",
        &Default::default(),
    )
    .unwrap();
    for x in 6..=10 {
        for z in 6..=10 {
            world.set_raw_block_state_for_test(DIM, BlockPos { x, y: 127, z }, stone);
        }
    }
    let bed = |part: &str| {
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:red_bed",
            &std::collections::BTreeMap::from([
                ("facing".into(), "north".into()),
                ("part".into(), part.into()),
                ("occupied".into(), "true".into()),
            ]),
        )
        .unwrap()
    };
    world.set_raw_block_state_for_test(DIM, head, bed("head"));
    world.set_raw_block_state_for_test(DIM, BlockPos { z: 9, ..head }, bed("foot"));
    let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
    body.set_sleeping_pos(Some(head));
    body.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    body.set_pos(Vec3d {
        x: 8.5,
        y: 128.6875,
        z: 8.5,
    });
    let result = world.with_stored_hurt_level(&DIM.into(), |store, level| {
        let mut body = store.detach(id).unwrap();
        let damage = bbb_server_datapack::damage::builtin_damage_type_registry()
            .iter()
            .find(|damage| damage.resource_id == "minecraft:generic")
            .unwrap();
        let result = crate::entity::general_living::external_die(
            &mut body,
            store,
            level,
            crate::entity::damage::StoredEntityDamageSource::without_attacker_views(
                damage, None, None,
            ),
        );
        store.attach(body);
        result
    });
    assert!(result);
    let body = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(body.sleeping_pos(), None);
    assert_eq!(body.pose_id(), VANILLA_ENTITY_POSE_DYING_ID);
    assert_eq!(
        world
            .level(DIM)
            .chunks
            .get_block_state(head.x, head.y, head.z),
        crate::fluid::with_property(bed("head"), "occupied", "false")
    );
    assert_eq!(body.bounding_box().min_y, body.position().y);
}
