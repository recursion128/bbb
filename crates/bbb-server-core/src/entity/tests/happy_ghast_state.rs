use crate::connection::ClientboundPlayPacket;
use bbb_protocol::entity_synched_data::VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID;
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};
use bbb_protocol::NbtSummaryValue;

use crate::entity::general_living::HappyGhastState;
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent};
use crate::entity::{EntityData, EntityStore};

fn fixture() -> (EntityStore, i32) {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    store.drain_lifecycle_events();
    (store, id)
}

fn state(store: &EntityStore, id: i32) -> HappyGhastState {
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        panic!()
    };
    data.happy_ghast_state().unwrap()
}

fn command_timeout(store: &mut EntityStore, id: i32, timeout: Option<i32>) {
    let NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap() else {
        panic!()
    };
    if let Some(timeout) = timeout {
        entries
            .iter_mut()
            .find(|entry| entry.name == "still_timeout")
            .unwrap()
            .value = NbtSummaryValue::Int(timeout);
    } else {
        entries.retain(|entry| entry.name != "still_timeout");
    }
    store.load_command_data(id, &entries).unwrap();
}

#[test]
fn happy_ghast_state_command_load_publishes_only_first_positive_entry_and_keeps_transients() {
    let (mut store, id) = fixture();
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
        panic!()
    };
    data.note_leash_holder();
    assert!(data.tick_leash_holder());
    data.happy_ghast_state_mut()
        .unwrap()
        .requires_precise_position = true;
    for (timeout, sync) in [(5, true), (7, false), (-1, false), (0, false), (3, true)] {
        command_timeout(&mut store, id, Some(timeout));
        assert_eq!(state(&store, id).still_timeout, timeout);
        assert_eq!(state(&store, id).stays_still, timeout > 0);
        assert!(state(&store, id).requires_precise_position);
        let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
            panic!()
        };
        assert_eq!(data.happy_ghast_is_leash_holder(), Some(true));
        let events = store.drain_lifecycle_events();
        let packets: Vec<_> = events
            .into_iter()
            .filter_map(|event| match event {
                EntityLifecycleEvent::Broadcast {
                    packet: ClientboundPlayPacket::EntityPositionSync(packet),
                    ..
                } => Some(packet),
                _ => None,
            })
            .collect();
        assert_eq!(packets.len(), usize::from(sync));
        if sync {
            assert_eq!(packets[0].position, store.get(id).unwrap().position());
            assert_eq!(packets[0].id, id);
        }
    }
    command_timeout(&mut store, id, None);
    assert_eq!(state(&store, id).still_timeout, 0);
    assert!(!state(&store, id).stays_still);
}

#[test]
fn happy_ghast_state_save_reload_has_one_timer_and_only_restores_persistent_state() {
    for timeout in [-7, 0, 10] {
        let (mut store, id) = fixture();
        command_timeout(&mut store, id, Some(timeout));
        let NbtSummaryValue::Compound(entries) = store.command_data(id).unwrap() else {
            panic!()
        };
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.name == "still_timeout")
                .count(),
            1
        );
        let chunk = crate::entity::save::write_entity_chunk(&store, (0, 0)).unwrap();
        let mut restored = EntityStore::new(18, EntityIdCounter::new());
        let id = crate::entity::save::read_entity_chunk(&mut restored, &chunk).unwrap()[0];
        assert_eq!(state(&restored, id).still_timeout, timeout);
        assert_eq!(state(&restored, id).stays_still, timeout > 0);
        assert!(!state(&restored, id).requires_precise_position);
        let entity = restored.get(id).unwrap();
        let tracker = crate::entity::tracking::ServerEntity::new(entity, 1, true);
        let pairing = tracker.send_pairing_data(entity, &[]);
        let flag = pairing
            .iter()
            .filter_map(|packet| match packet {
                ClientboundPlayPacket::SetEntityData(data) => Some(&data.values),
                _ => None,
            })
            .flatten()
            .find(|row| row.data_id == VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID);
        assert_eq!(
            flag.map(|row| &row.value),
            (timeout > 0).then_some(&EntityDataValueKind::Boolean(true))
        );
    }
}

#[test]
fn happy_ghast_state_tracking_keeps_precise_frame_after_timer_reaches_zero() {
    let (mut store, id) = fixture();
    let mut body = store.detach(id).unwrap();
    crate::entity::happy_ghast::set_server_still_timeout(&mut body, 1);
    let EntityData::GeneralLiving(data) = &mut body.data else {
        panic!()
    };
    data.happy_ghast_state_mut()
        .unwrap()
        .requires_precise_position = true;
    let mut tracker = crate::entity::tracking::ServerEntity::new(&body, 1, true);
    tracker.send_changes(&mut body);
    crate::entity::happy_ghast::set_server_still_timeout(&mut body, 0);
    assert!(tracker
        .send_changes(&mut body)
        .iter()
        .any(|packet| matches!(packet, ClientboundPlayPacket::EntityPositionSync(_))));
    let EntityData::GeneralLiving(data) = &mut body.data else {
        panic!()
    };
    data.happy_ghast_state_mut()
        .unwrap()
        .requires_precise_position = false;
    body.set_pos(Vec3d {
        x: 0.52,
        ..body.position()
    });
    assert!(tracker
        .send_changes(&mut body)
        .iter()
        .all(|packet| !matches!(packet, ClientboundPlayPacket::EntityPositionSync(_))));
}
