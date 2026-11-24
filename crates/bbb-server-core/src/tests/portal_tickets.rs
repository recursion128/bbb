//! Live portal-ticket placement, persistence-facing state and rollback.

use super::*;

#[test]
fn portal_ticket_placement_loads_full_reach_without_aging_the_new_ticket() {
    let candidate = restore_candidate_without_tickets(10, None, Vec::new());
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();

    world
        .place_portal_ticket(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 7, y: 64, z: 9 },
        )
        .unwrap();

    assert_eq!(
        world.saved_chunk_tickets_for_test(Java26BuiltinDimension::Overworld),
        vec![((0, 0), 30, Some(300))]
    );
    let square = (-3..=3).flat_map(|x| (-3..=3).map(move |z| (x, z)));
    for chunk in square {
        let chunks = world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
        assert!(chunks.is_ticking_full(chunk.0, chunk.1), "{chunk:?}");
        assert!(world
            .chunk_tick_containers_registered_for_test(Java26BuiltinDimension::Overworld, chunk));
    }

    let (_sender, receiver) = mailbox();
    world.tick(receiver.drain());
    assert_eq!(
        world.saved_chunk_tickets_for_test(Java26BuiltinDimension::Overworld),
        vec![((0, 0), 30, Some(299))]
    );
}

/// `TicketStorage.addTicket` (`TicketStorage.java:147-178`) resets an equal
/// type-and-level ticket and leaves every other ticket in the column alone.
#[test]
fn portal_ticket_refresh_resets_only_its_identity() {
    let mut candidate = restore_candidate_without_tickets(10, None, Vec::new());
    candidate.chunk_tickets.insert(
        level_key(Java26BuiltinDimension::Overworld),
        vec![
            bbb_server_persistence::Java26SavedChunkTicket {
                chunk_x: 0,
                chunk_z: 0,
                level: 30,
                ticks_left: None,
            },
            bbb_server_persistence::Java26SavedChunkTicket {
                chunk_x: 0,
                chunk_z: 0,
                level: 29,
                ticks_left: Some(41),
            },
            bbb_server_persistence::Java26SavedChunkTicket {
                chunk_x: 0,
                chunk_z: 0,
                level: 30,
                ticks_left: Some(17),
            },
        ],
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();

    world
        .place_portal_ticket(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 7, y: 64, z: 9 },
        )
        .unwrap();

    assert_eq!(
        world.saved_chunk_tickets_for_test(Java26BuiltinDimension::Overworld),
        vec![
            ((0, 0), 30, None),
            ((0, 0), 29, Some(41)),
            ((0, 0), 30, Some(300)),
        ]
    );
}
