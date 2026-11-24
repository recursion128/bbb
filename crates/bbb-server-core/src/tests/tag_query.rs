//! The F3+I tag queries (`ServerGamePacketListenerImpl.java:1017-1030`,
//! `:1043-1050`) through real session packets.

use super::*;

use crate::block_entity::BlockEntity;
use bbb_protocol::packets::TagQuery;

/// Alice is an owner, bob holds no operator entry and carol is a moderator,
/// one level short of the gamemaster permission both handlers ask for.
fn tag_query_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let mut config = operator_config(&[("alice", 1), ("carol", 3)]);
    config
        .admin_lists
        .operators
        .get_mut(&Uuid::from_u128(3))
        .expect("carol is on the operator list")
        .level = CommandPermission::Moderator;
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded_stocked(&sender, &receiver, &mut world, 2, "bob", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "carol", 3);
    (sender, receiver, world)
}

fn tag_query_answers(tick: &TickResult) -> Vec<(ConnectionId, &TagQuery)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::TagQuery(update),
            } => Some((*connection_id, update)),
            _ => None,
        })
        .collect()
}

/// The one answer a query produced, checked against the transaction it echoes
/// and decoded back out of the bytes the packet carries.
fn answered_tag(tick: &TickResult, transaction_id: i32) -> Option<Vec<NbtSummaryEntry>> {
    let answers = tag_query_answers(tick);
    let [(ConnectionId(1), update)] = answers.as_slice() else {
        panic!("one tag query answer to the asker: {answers:?}");
    };
    assert_eq!(update.transaction_id, transaction_id);
    match bbb_protocol::decode_nbt_summary(&update.raw_nbt).unwrap() {
        Some(NbtSummaryValue::Compound(entries)) => {
            assert!(update.tag_present);
            Some(entries)
        }
        None => {
            assert!(!update.tag_present);
            None
        }
        Some(other) => panic!("a tag query root is a compound: {other:?}"),
    }
}

fn has_field(entries: &[NbtSummaryEntry], name: &str) -> bool {
    entries.iter().any(|entry| entry.name == name)
}

fn chest_with_stone(world: &mut ServerWorld, pos: BlockPos) {
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("type".to_string(), "single".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    let chest = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:chest",
        &properties,
    )
    .expect("minecraft:chest is a vanilla 26.1 block");
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        chest,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let mut items = vec![bbb_server_types::item_stack::RuntimeItemStack::empty(); 27];
    items[4] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 5,
        component_patch: DataComponentPatchSummary::default().into(),
    });
    assert!(world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .and_then(BlockEntity::chest_mut)
        .expect("the write created the chest's block entity")
        .set_items(items));
}

fn spawn_pig(world: &mut ServerWorld, dimension: Java26BuiltinDimension) -> i32 {
    let pig_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
    let pig =
        crate::entity::GeneralLivingData::new(pig_type).expect("a pig owns living attributes");
    world
        .spawn_stored_entity_for_test(
            dimension,
            pig_type,
            Vec3d {
                x: 3.5,
                y: SPAWN_Y,
                z: 3.5,
            },
            crate::entity::EntityData::GeneralLiving(pig),
        )
        .0
}

#[test]
fn a_gamemaster_block_entity_tag_query_answers_the_record_without_metadata() {
    let (sender, receiver, mut world) = tag_query_world();
    let pos = BlockPos { x: 0, y: 64, z: 1 };
    chest_with_stone(&mut world, pos);
    let Some(NbtSummaryValue::Compound(full)) = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .map(|chest| {
            chest
                .command_data_with_resolver(
                    crate::world::registry_runtime::item_stack_resolver_for_test(),
                )
                .expect("the fixture operation succeeds")
        })
    else {
        panic!("the chest answers `/data get block`");
    };

    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::BlockEntityTagQuery(BlockEntityTagQuery {
            transaction_id: 5,
            pos,
        }),
    );
    let tag = answered_tag(&tick, 5).expect("a chest stands there");
    // `saveWithoutMetadata` is `/data get block`'s `saveWithFullMetadata`
    // without what `saveMetadata` adds (`BlockEntity.java:118-121`, `:161-166`).
    let expected = full
        .into_iter()
        .filter(|entry| !["id", "x", "y", "z"].iter().any(|key| entry.name == *key))
        .collect::<Vec<_>>();
    assert_eq!(tag, expected);
    assert!(has_field(&tag, "Items") && has_field(&tag, "components"));
}

#[test]
fn a_gamemaster_block_entity_tag_query_without_a_block_entity_answers_the_null_tag() {
    let (sender, receiver, mut world) = tag_query_world();
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::BlockEntityTagQuery(BlockEntityTagQuery {
            transaction_id: 6,
            pos: BlockPos { x: 0, y: 64, z: 1 },
        }),
    );
    assert_eq!(answered_tag(&tick, 6), None);
    assert_eq!(tag_query_answers(&tick)[0].1.raw_nbt, [0]);
}

#[test]
fn a_gamemaster_entity_tag_query_answers_a_stored_entity_save_without_id() {
    let (sender, receiver, mut world) = tag_query_world();
    let pig = spawn_pig(&mut world, Java26BuiltinDimension::Overworld);
    let Some(NbtSummaryValue::Compound(expected)) =
        world.stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, pig)
    else {
        panic!("the pig answers `/data get entity`");
    };

    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::EntityTagQuery(EntityTagQuery {
            transaction_id: 7,
            entity_id: pig,
        }),
    );
    let tag = answered_tag(&tick, 7).expect("the pig is in alice's level");
    assert_eq!(tag, expected);
    assert!(has_field(&tag, "UUID") && !has_field(&tag, "id"));
}

#[test]
fn a_gamemaster_entity_tag_query_answers_a_session_player_without_the_selected_item() {
    let (sender, receiver, mut world) = tag_query_world();
    let bob = entity_id_of(&world, 2);

    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::EntityTagQuery(EntityTagQuery {
            transaction_id: 8,
            entity_id: bob,
        }),
    );
    let tag = answered_tag(&tick, 8).expect("bob is in alice's level");
    assert_eq!(field(&tag, "UUID"), &nbt_uuid(Uuid::from_u128(2)));
    // Bob holds stone, but `SelectedItem` is `NbtPredicate`'s addition to
    // `saveWithoutId` (NbtPredicate.java:42-53), not part of the save itself.
    let NbtSummaryValue::List(inventory) = field(&tag, "Inventory") else {
        panic!("the player save carries its inventory");
    };
    assert_eq!(inventory.len(), 1, "{inventory:?}");
    assert!(!has_field(&tag, "SelectedItem") && !has_field(&tag, "id"));
}

#[test]
fn an_entity_tag_query_for_nothing_in_the_askers_level_is_not_answered() {
    let (sender, receiver, mut world) = tag_query_world();
    let nether_pig = spawn_pig(&mut world, Java26BuiltinDimension::Nether);
    let bob = entity_id_of(&world, 2);
    let mut moved = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(2),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 8.5,
                y: 70.0,
                z: 8.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut moved,
    ));

    for (transaction_id, entity_id) in [(9, i32::MAX), (10, nether_pig), (11, bob)] {
        let tick = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::EntityTagQuery(EntityTagQuery {
                transaction_id,
                entity_id,
            }),
        );
        assert!(
            tag_query_answers(&tick).is_empty(),
            "entity {entity_id}: {tick:?}"
        );
    }
}

#[test]
fn a_tag_query_below_the_gamemaster_permission_is_not_answered() {
    let (sender, receiver, mut world) = tag_query_world();
    let pos = BlockPos { x: 0, y: 64, z: 1 };
    chest_with_stone(&mut world, pos);
    let alice = entity_id_of(&world, 1);

    for connection_id in [2, 3] {
        for packet in [
            PlayServerbound::BlockEntityTagQuery(BlockEntityTagQuery {
                transaction_id: 12,
                pos,
            }),
            PlayServerbound::EntityTagQuery(EntityTagQuery {
                transaction_id: 13,
                entity_id: alice,
            }),
        ] {
            let tick = play(&sender, &receiver, &mut world, connection_id, packet);
            assert!(
                tag_query_answers(&tick).is_empty(),
                "connection {connection_id}: {tick:?}"
            );
        }
    }
}
