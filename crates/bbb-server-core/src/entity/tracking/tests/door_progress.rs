use super::*;
use bbb_protocol::packets::BlockDestruction;

fn progress(id: i32, stage: i32) -> EntityLifecycleEvent {
    EntityLifecycleEvent::BlockDestruction {
        entity_id: id,
        pos: BlockPos { x: 0, y: 64, z: 0 },
        stage,
    }
}
fn packet(id: i32, stage: u8) -> ClientboundPlayPacket {
    ClientboundPlayPacket::BlockDestruction(BlockDestruction {
        id,
        pos: BlockPos { x: 0, y: 64, z: 0 },
        progress: stage,
    })
}

#[test]
fn door_progress_reaches_nontracking_viewers_strictly_inside_three_dimensional_range() {
    let mut tracker = EntityTracker::new();
    let mut source = viewer(3, 0.0);
    source.entity_id = 42;
    let mut vertical_boundary = viewer(4, 0.0);
    vertical_boundary.position.y += 32.0;
    let mut depth_boundary = viewer(5, 0.0);
    depth_boundary.position.z = -32.0;
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &[progress(42, -1)],
        &[
            viewer(1, 31.99),
            viewer(2, 32.0),
            source,
            vertical_boundary,
            depth_boundary,
            viewer(6, -31.99),
        ],
        |_, _, _| false,
        &mut out,
    );
    assert_eq!(
        out,
        vec![TrackingPacket {
            recipients: vec![ConnectionId(1), ConnectionId(6)],
            packet: packet(42, 255),
        }]
    );
}
