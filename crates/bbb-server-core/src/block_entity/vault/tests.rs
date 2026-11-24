use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_VAULT_ID;
use bbb_protocol::item_types::{
    VANILLA_ITEM_ARROW_ID, VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_TRIAL_KEY_ID,
};
use bbb_protocol::packets::{BlockPos, ItemStackSummary};
use bbb_protocol::NbtSummaryValue;

use super::*;
use crate::block_entity::{entry, new_block_entity, unique_field};

fn pos() -> BlockPos {
    BlockPos { x: 7, y: 8, z: 9 }
}
fn vault_state() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_VAULT_ID).expect("vault default state")
}
fn item(id: i32, count: i32) -> RuntimeItemStack {
    RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(id),
        count,
        component_patch: Default::default(),
    })
}
fn save(be: &crate::block_entity::BlockEntity) -> Vec<NbtSummaryEntry> {
    be.save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds")
}

#[test]
fn a_vault_default_state_saves_a_trial_key() {
    let built = new_block_entity(pos(), vault_state()).expect("a vault builds");
    assert_eq!(built.type_id(), VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID);
    let saved = save(&built);
    let NbtSummaryValue::Compound(config) = unique_field(&saved, "config").unwrap() else {
        panic!()
    };
    assert!(unique_field(config, "key_item").is_some());
    let key = built.vault().unwrap().config().key_item();
    assert_eq!(key.item_id(), Some(VANILLA_ITEM_TRIAL_KEY_ID));
    assert_eq!(
        unique_field(&saved, "shared_data"),
        Some(&NbtSummaryValue::Compound(Vec::new()))
    );
    assert_eq!(
        unique_field(&saved, "server_data"),
        Some(&NbtSummaryValue::Compound(Vec::new()))
    );
}

#[test]
fn a_vault_round_trips_config_shared_data_and_server_data() {
    let state = vault_state();
    let mut record = save(&new_block_entity(pos(), state).unwrap());
    record.retain(|e| {
        !["config", "shared_data", "server_data"]
            .iter()
            .any(|name| e.name == *name)
    });
    record.push(entry(
        "config",
        NbtSummaryValue::Compound(vec![entry(
            "loot_table",
            NbtSummaryValue::String("minecraft:chests/trial_chambers/reward".into()),
        )]),
    ));
    record.push(entry(
        "shared_data",
        NbtSummaryValue::Compound(vec![
            entry(
                "display_item",
                NbtSummaryValue::Compound(vec![entry(
                    "id",
                    NbtSummaryValue::String("minecraft:arrow".into()),
                )]),
            ),
            entry(
                "connected_players",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::IntArray(vec![9, 8, 7, 6]),
                    NbtSummaryValue::IntArray(vec![6, 7, 8, 9]),
                ]),
            ),
        ]),
    ));
    record.push(entry(
        "server_data",
        NbtSummaryValue::Compound(vec![entry(
            "state_updating_resumes_at",
            NbtSummaryValue::Long(14),
        )]),
    ));
    let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
        .unwrap()
        .unwrap();
    let be = loaded.vault().unwrap();
    assert!(be.config().key_item().is_empty());
    assert_eq!(
        be.shared_data().display_item().item_id(),
        Some(VANILLA_ITEM_ARROW_ID)
    );
    assert_eq!(
        be.shared_data().connected_players(),
        &[
            crate::block_entity::codec_uuid(&NbtSummaryValue::IntArray(vec![9, 8, 7, 6])).unwrap(),
            crate::block_entity::codec_uuid(&NbtSummaryValue::IntArray(vec![6, 7, 8, 9])).unwrap(),
        ]
    );
    assert_eq!(be.server_data().state_updating_resumes_at(), 14);
    let saved = save(&loaded);
    let again = save(
        &crate::block_entity::load_test_fixture(pos(), state, &saved)
            .unwrap()
            .unwrap(),
    );
    for name in ["config", "shared_data", "server_data"] {
        assert_eq!(unique_field(&saved, name), unique_field(&again, name));
    }
}

fn world_with_vault(at: BlockPos) -> crate::world::ServerWorld {
    let mut world = crate::world::ServerWorld::new(crate::tests::datapack_config()).unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_state_with_property(
        vault_state(),
        "vault_state",
        "active",
    )
    .unwrap();
    assert!(world.set_block(
        bbb_server_types::save::Java26BuiltinDimension::Overworld,
        at,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world
}

#[test]
fn vault_unlock_pauses_then_ejects_rewards_and_rejects_a_rewarded_player() {
    use VaultState::*;
    let mut world = world_with_vault(pos());
    let dimension = bbb_server_types::save::Java26BuiltinDimension::Overworld.into();
    let mut events = Vec::new();
    let mut ctx = VaultLevel {
        world: &mut world,
        dimension: &dimension,
        pos: pos(),
        events: &mut events,
    };
    let mut be = VaultBlockEntity::default();
    let player = Uuid::from_u128(7);
    let key = item(VANILLA_ITEM_TRIAL_KEY_ID, 1);
    let wrong = item(VANILLA_ITEM_ARROW_ID, 1);
    assert_eq!(
        be.key_decision(
            crate::block_entity::test_item_resolver(),
            Active,
            15,
            player,
            &wrong
        )
        .unwrap(),
        VaultKeyDecision::Fail(VaultSound::InsertFail)
    );
    assert_eq!(
        be.key_decision(
            crate::block_entity::test_item_resolver(),
            Active,
            15,
            player,
            &key
        )
        .unwrap(),
        VaultKeyDecision::DrawReward
    );
    be.unlock(
        player,
        vec![
            item(VANILLA_ITEM_ARROW_ID, 2),
            item(VANILLA_ITEM_DIAMOND_ID, 1),
        ],
        1,
        Active,
        &mut ctx,
    )
    .unwrap();
    assert_eq!(be.server_data().rewarded_players(), &[player]);
    be.tick(Unlocking, 14, &mut ctx).unwrap();
    assert_eq!(be.server_data().items_to_eject().len(), 2);
    be.tick(Unlocking, 15, &mut ctx).unwrap();
    be.tick(Ejecting, 35, &mut ctx).unwrap();
    assert_eq!(
        be.server_data().items_to_eject(),
        &[item(VANILLA_ITEM_ARROW_ID, 2)]
    );
    let record = be
        .save_additional(&mut BlockEntityItemSave::for_test())
        .unwrap();
    let mut reload = VaultBlockEntity::default();
    reload
        .load_additional(&record, &mut BlockEntityItemLoad::for_test())
        .unwrap();
    assert_eq!(
        reload.server_data().items_to_eject(),
        &[item(VANILLA_ITEM_ARROW_ID, 2)]
    );
    assert_eq!(reload.server_data().state_updating_resumes_at(), 55);
    be.tick(Ejecting, 55, &mut ctx).unwrap();
    assert!(be.server_data().items_to_eject().is_empty());
    be.tick(Ejecting, 75, &mut ctx).unwrap();
    assert_eq!(
        be.key_decision(
            crate::block_entity::test_item_resolver(),
            Active,
            100,
            player,
            &key
        )
        .unwrap(),
        VaultKeyDecision::Fail(VaultSound::RejectRewardedPlayer)
    );
}

#[test]
fn vault_connections_refresh_excludes_rewarded_players() {
    let players: Vec<Uuid> = (1..=5_u128).map(Uuid::from_u128).collect();
    let mut world = crate::world::ServerWorld::new(crate::tests::datapack_config()).unwrap();
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    for (index, player) in players.iter().enumerate() {
        crate::tests::join_loaded(
            &sender,
            &receiver,
            &mut world,
            index as u64 + 1,
            &format!("vault-{index}"),
            player.as_u128(),
        );
    }
    let at = world.publication().players[0].position;
    let at = BlockPos {
        x: at.x.floor() as i32,
        y: at.y.floor() as i32,
        z: at.z.floor() as i32,
    };
    let state = bbb_protocol::block_state_types::vanilla_block_state_with_property(
        vault_state(),
        "vault_state",
        "active",
    )
    .unwrap();
    assert!(world.set_block(
        bbb_server_types::save::Java26BuiltinDimension::Overworld,
        at,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    let dimension = bbb_server_types::save::Java26BuiltinDimension::Overworld.into();
    let mut events = Vec::new();
    let mut ctx = VaultLevel {
        world: &mut world,
        dimension: &dimension,
        pos: at,
        events: &mut events,
    };
    let mut be = VaultBlockEntity::default();
    be.tick(VaultState::Active, 1, &mut ctx).unwrap();
    assert_eq!(be.shared_data().connected_players(), players);
    be.server_data.rewarded_players.push(players[3]);
    be.tick(VaultState::Active, 21, &mut ctx).unwrap();
    let expected: Vec<Uuid> = [0, 1, 2, 4].map(|i| players[i]).to_vec();
    assert_eq!(be.shared_data().connected_players(), expected);
}
