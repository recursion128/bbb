use super::*;
use crate::connection::ConnectionId;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
};

const MOUNT_POS: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

#[test]
fn an_interacted_undead_mount_never_despawns_far_from_players() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "rider", 1);
    level.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 500.5,
            ..MOUNT_POS
        },
    );
    let player = level.world.publication().players[0].clone();
    let ids = EntityIdCounter::new();
    let highest = level
        .world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .chain([player.entity_id])
        .max()
        .unwrap_or(0);
    for _ in 0..highest {
        ids.next_id();
    }
    let mut store = EntityStore::new(0x554e_4d54, ids);
    store.set_difficulty(Difficulty::Normal);
    let mut kept = Vec::new();
    let mut wild = Vec::new();
    for entity_type_id in [
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ] {
        let touched = store
            .spawn_general_living(entity_type_id, MOUNT_POS)
            .unwrap();
        let mut hand = ItemStackSummary::empty();
        crate::entity::leash::interact_leash(
            &mut store,
            touched,
            &mut crate::entity::leash::LeashInteractingPlayer {
                hand_stack: &mut hand,
                mob_item_used: false,
                entity_id: player.entity_id,
                uuid: player.uuid,
                position: player.position,
                secondary_use_active: false,
                infinite_materials: false,
            },
        );
        kept.push(touched);
        wild.push(
            store
                .spawn_general_living(entity_type_id, MOUNT_POS)
                .unwrap(),
        );
    }
    store.drain_lifecycle_events();
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    for id in kept {
        assert!(store.get(id).is_some());
    }
    for id in wild {
        assert!(store.get(id).is_none());
    }
}
