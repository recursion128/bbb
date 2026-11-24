//! `EnderEyeItem.useOn` and `EnderEyeItem.use`: an eye set into a portal
//! frame, and one thrown toward a stronghold.

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID;
use bbb_protocol::packets::{BlockHitResult, BlockPos, Direction, ItemStackSummary, Vec3d};

use super::*;

const STANDING: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

fn frame(facing: &str, eye: bool) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        "minecraft:end_portal_frame",
        &std::collections::BTreeMap::from([
            ("eye".to_owned(), eye.to_string()),
            ("facing".to_owned(), facing.to_owned()),
        ]),
    )
    .unwrap()
}

fn survival_world(
    config: ServerWorldConfig,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:ender_eye")),
            count: 2,
            ..ItemStackSummary::empty()
        },
    );
    (sender, receiver, world)
}

#[test]
fn the_last_eye_set_into_a_frame_ring_opens_the_end_portal() {
    let (sender, receiver, mut world) = survival_world(datapack_config());
    // The interior is x -1..=1, z 2..=4; every frame faces into it.
    let last = BlockPos { x: 0, y: 64, z: 1 };
    let mut ring = Vec::new();
    for i in -1..=1 {
        ring.push((BlockPos { x: i, y: 64, z: 1 }, "south"));
        ring.push((BlockPos { x: i, y: 64, z: 5 }, "north"));
        ring.push((
            BlockPos {
                x: -2,
                y: 64,
                z: 3 + i,
            },
            "east",
        ));
        ring.push((
            BlockPos {
                x: 2,
                y: 64,
                z: 3 + i,
            },
            "west",
        ));
    }
    for (pos, facing) in &ring {
        world.set_block(
            Java26BuiltinDimension::Overworld,
            *pos,
            frame(facing, *pos != last),
            SET_BLOCK_UPDATE_ALL,
        );
    }

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: last,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 0.8,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    );

    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, last),
        Some(frame("south", true))
    );
    for x in -1..=1 {
        for z in 2..=4 {
            assert_eq!(
                world.block_state(Java26BuiltinDimension::Overworld, BlockPos { x, y: 64, z }),
                Some(block_state_id("minecraft:end_portal")),
                "({x}, {z})"
            );
        }
    }
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 1);
}

fn compound_mut<'a>(
    entries: &'a mut [NbtSummaryEntry],
    name: &str,
) -> &'a mut Vec<NbtSummaryEntry> {
    let entry = entries.iter_mut().find(|entry| entry.name == name).unwrap();
    let NbtSummaryValue::Compound(value) = &mut entry.value else {
        panic!("{name} is not a compound")
    };
    value
}

fn set_entry(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    entries.retain(|entry| entry.name != name);
    entries.push(NbtSummaryEntry {
        name: name.into(),
        value,
    });
}

/// The fixture flat world with structures on and only the stronghold set.
fn stronghold_world(config: ServerWorldConfig) -> ServerWorld {
    let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
    let mut document = candidate.world_gen_settings.document.clone();
    let data = compound_mut(document.entries_mut(), "data");
    set_entry(data, "generate_structures", NbtSummaryValue::Byte(1));
    let dimensions = compound_mut(data, "dimensions");
    let overworld = compound_mut(dimensions, "minecraft:overworld");
    let generator = compound_mut(overworld, "generator");
    let settings = compound_mut(generator, "settings");
    set_entry(
        settings,
        "structure_overrides",
        NbtSummaryValue::List(vec![NbtSummaryValue::string("minecraft:strongholds")]),
    );
    candidate.world_gen_settings =
        bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
            document, 0,
        )
        .unwrap();
    spawn_pinned_restore(config, &candidate)
}

#[test]
fn a_thrown_eye_flies_toward_the_nearest_stronghold_and_fires_used_ender_eye() {
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::AdvancementElement;

    let mut config = datapack_config();
    let mut startup = (*config.datapacks.take().unwrap()).clone();
    startup.advancements.advancements.push(AdvancementElement {
        id: "test:eye".to_owned(),
        source_pack: "test".to_owned(),
        advancement: parse_advancement(&serde_json::json!({
            "criteria": {"eye": {
                "trigger": "minecraft:used_ender_eye",
                "conditions": {"distance": {"min": 100.0}}
            }}
        }))
        .unwrap(),
    });
    config.datapacks = Some(std::sync::Arc::new(startup));
    let (sender, receiver) = mailbox();
    let mut world = stronghold_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:ender_eye")),
            count: 2,
            ..ItemStackSummary::empty()
        },
    );

    let thrown = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);
    // `SUCCESS_SERVER` (EnderEyeItem.java:107): the server swings for the
    // thrower too (ServerGamePacketListenerImpl.java:1413-1416).
    assert!(thrown.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::EntityAnimation(update),
        } if update.action == 0
    )));

    let eyes: Vec<_> = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id))
        .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID)
        .cloned()
        .collect();
    assert_eq!(eyes.len(), 1);
    let crate::entity::EntityData::EyeOfEnder(eye) = eyes[0].data() else {
        unreachable!()
    };
    // The seed-0 fixture's nearest stronghold is far to -x/+z; the eye heads
    // for it, its target capped 12 blocks out (`EyeOfEnder.signalTo`).
    let target = eye.target().expect("the eye was signalled");
    let start = eyes[0].position();
    let (dx, dz) = (target.x - start.x, target.z - start.z);
    assert!(dx < 0.0 && dz > 0.0, "{target:?}");
    assert!((dx * dx + dz * dz).sqrt() <= 12.0 + 1e-6, "{target:?}");
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:eye"));
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 1);
}
