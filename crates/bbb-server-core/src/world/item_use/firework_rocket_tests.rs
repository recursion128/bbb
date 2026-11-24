use crate::connection::ConnectionId;
use crate::entity::leash::{is_leashed, set_leashed_to_with, LeashAttachment};
use crate::entity::{EntityData, GeneralLivingData};
use crate::world::{ServerWorld, ServerWorldConfig};
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{InteractionHand, ItemStackSummary, PlayServerbound, UseItem, Vec3d};
use bbb_server_types::save::Java26BuiltinDimension;

const PLAYER: ConnectionId = ConnectionId(1);
const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

/// FireworkRocketItem.java:62-64: a gliding boost drops every lead the
/// player holds.
#[test]
fn a_gliding_firework_boost_drops_the_players_leads() {
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        PLAYER,
        Vec3d {
            x: 0.5,
            y: 90.0,
            z: 0.5,
        },
    );
    world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
    let player = &world.levels[PLAYER];
    let (player_entity, player_uuid) = (player.entity_id(), player.profile.uuid);
    let (pig, _) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_PIG_ID,
        Vec3d {
            x: 2.5,
            y: 90.0,
            z: 0.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_PIG_ID).unwrap()),
    );
    let store = &mut world.level_mut(DIMENSION).entities;
    store.with_entity_mut(pig, |pig, store| {
        assert!(set_leashed_to_with(
            pig,
            store,
            player_entity,
            LeashAttachment::Holder(player_uuid),
            true,
            false,
        ));
    });
    assert!(is_leashed(store.get(pig).unwrap()));
    world.levels[PLAYER].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, true);
    world.stock_menu_slot_for_test(
        PLAYER,
        36,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_FIREWORK_ROCKET_ID),
            count: 2,
            component_patch: Default::default(),
        },
    );
    world.apply_play(
        PLAYER,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: 1,
            y_rot: 0.0,
            x_rot: 0.0,
        }),
        &mut Vec::new(),
    );
    assert!(!is_leashed(
        world.level(DIMENSION).entities.get(pig).unwrap()
    ));
    assert!(world
        .stored_items_for_test(DIMENSION)
        .into_iter()
        .any(|(_, _, id, count, _, _)| id == "minecraft:lead" && count == 1));
}
