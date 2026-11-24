use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAGS_DATA_ID, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
    VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
};

use bbb_protocol::packets::decode_play_serverbound;

use bbb_client_world::{CommandSuggestionRequestState, SoundHolderState};

use super::*;
use crate::types::{
    INTERACTION_DROP_ENTITY_TYPE_NAME, INTERACTION_DROP_SLOT_COUNT_AFTER_PICKUP,
    INTERACTION_HELD_STACK_COUNT_AFTER_CLOSE, INTERACTION_NOTE_BLOCK_TUNED_NOTE,
    INTERACTION_REDSTONE_WIRE_NAME, INTERACTION_REDSTONE_WIRE_STATE, NOTE_BLOCK_NOTE_PROPERTY,
};

#[test]
fn the_tick_barrier_asks_for_a_completion_rather_than_running_a_command() {
    let request = CommandSuggestionRequestState {
        id: 7,
        command: TICK_BARRIER_COMMAND.into(),
    };
    let (id, payload) = tick_barrier_request(request).unwrap();
    // A chat command would be answered off the tick, so the barrier has to
    // be a packet the server handles in its packet phase instead.
    assert_eq!(id, bbb_protocol::ids::play::SERVERBOUND_COMMAND_SUGGESTION);
    assert_ne!(id, bbb_protocol::ids::play::SERVERBOUND_CHAT_COMMAND);
    let bbb_protocol::packets::PlayServerbound::CommandSuggestion(suggestion) =
        decode_play_serverbound(id, &payload).unwrap()
    else {
        panic!("the barrier is not a completion request");
    };
    assert_eq!(suggestion.id, 7);
    assert_eq!(suggestion.command, TICK_BARRIER_COMMAND);
    // Every node the request can reach is a literal, so the dispatcher
    // answers it without a further round of its own.
    assert!(!TICK_BARRIER_COMMAND.trim_end().contains(' '));
}

/// The two malformed refusal cases damage a real packet, so nothing stops
/// an encoder change from turning the damage into something a decoder
/// happens to accept. Then the sessions would join, send a valid packet and
/// never be refused, and the gate would report a difference in the servers
/// rather than a probe that stopped asking the question. This asks the
/// question of the real decoder instead: the source decodes, the damaged
/// one does not.
#[test]
fn the_malformed_refusal_cases_carry_bodies_no_decoder_accepts() {
    for (case, source) in [
        (RefusalCase::MalformedPacketBody, malformed_body_source()),
        (RefusalCase::MalformedComponentNbt, malformed_nbt_source()),
    ] {
        let (source_id, source_payload) = source.unwrap();
        decode_play_serverbound(source_id, &source_payload).unwrap_or_else(|error| {
            panic!("{case:?} starts from a packet no server can read: {error}")
        });

        let (id, payload) = case.packet().unwrap();
        assert_eq!(id, source_id, "{case:?} must keep the packet id it damages");
        assert!(
            decode_play_serverbound(id, &payload).is_err(),
            "{case:?} sends a body the decoder accepts, so no server would refuse it"
        );
    }
}

/// The walk exists to leave the column the session spawned in, and it aims
/// at that column's centre. Both halves have to hold at once: a zero offset
/// would probe the spawn column under a different name, and a centre that
/// landed in a neighbouring column would probe a column the walk never
/// reached.
#[test]
fn the_outward_walk_targets_the_centre_of_a_column_it_is_not_already_in() {
    for start in [
        ChunkPos { x: 0, z: 0 },
        ChunkPos { x: -3, z: 7 },
        ChunkPos { x: 12, z: -9 },
    ] {
        let walked = ChunkPos {
            x: start.x + OUTWARD_WALK_CHUNKS,
            z: start.z,
        };
        assert_ne!(walked, start, "the walk must leave the column it starts in");
        let center = column_center_block(walked);
        assert_eq!(
            column_of(packets::Vec3d {
                x: f64::from(center.x) + 0.5,
                y: 0.0,
                z: f64::from(center.z) + 0.5,
            }),
            walked,
            "the walk must end inside the column it probes"
        );
    }
}

/// `SectionPos.blockToSectionCoord` is an arithmetic shift, so the column
/// west of the origin is -1 rather than the 0 a truncating division gives.
/// The official server spawns players either side of the origin, so a probe
/// that got this wrong would walk into the column it started in on half its
/// runs and into a column two over on the other half.
#[test]
fn column_of_floors_across_the_origin() {
    for (x, expected) in [
        (0.5_f64, 0),
        (15.9, 0),
        (16.0, 1),
        (-0.5, -1),
        (-16.0, -1),
        (-16.5, -2),
    ] {
        assert_eq!(
            column_of(packets::Vec3d { x, y: 0.0, z: x }),
            ChunkPos {
                x: expected,
                z: expected
            },
            "x = {x}"
        );
    }
}

#[test]
fn probe_drain_finishes_on_first_chunk_by_default() {
    let first_chunk = ChunkPos { x: 1, z: -2 };
    let mut drain = ProbeDrain::new(0);

    assert!(!drain.observe_packet(None));
    assert!(drain.observe_packet(Some(first_chunk)));
    assert_eq!(drain.first_chunk(), Some(first_chunk));
    assert_eq!(drain.packets_after_first_chunk, 0);
}

#[test]
fn probe_drain_counts_bounded_packets_after_first_chunk() {
    let first_chunk = ChunkPos { x: 1, z: -2 };
    let later_chunk = ChunkPos { x: 3, z: 4 };
    let mut drain = ProbeDrain::new(2);

    assert!(!drain.observe_packet(None));
    assert!(!drain.observe_packet(Some(first_chunk)));
    assert_eq!(drain.first_chunk(), Some(first_chunk));
    assert_eq!(drain.packets_after_first_chunk, 0);

    assert!(!drain.observe_packet(None));
    assert_eq!(drain.packets_after_first_chunk, 1);
    assert!(drain.observe_packet(Some(later_chunk)));
    assert_eq!(drain.packets_after_first_chunk, 2);
    assert_eq!(drain.first_chunk(), Some(first_chunk));
}

#[test]
fn interaction_replay_waits_for_the_first_chunk_batch_to_close() {
    let first_chunk = ChunkPos { x: 1, z: -2 };
    let mut drain = ProbeDrain::new(0);
    let mut delivery = ChunkDeliveryObserver::default();

    delivery.on_batch_start();
    delivery.on_chunk();
    assert!(drain.observe_packet(Some(first_chunk)));
    assert!(!interaction_replay_ready(&drain, &delivery));

    delivery.on_batch_finished(1);
    assert!(interaction_replay_ready(&drain, &delivery));
}

/// A replay report holding every result the replay produces, which is the
/// shape `completed` and `drop_finished` are asked about.
fn completed_replay_report() -> InteractionReplayReport {
    InteractionReplayReport {
        chat_content: INTERACTION_CHAT_MESSAGE.to_string(),
        chat_observed: true,
        broken_block: BlockPos { x: 0, y: 63, z: 0 },
        broken_block_name: Some("minecraft:air".to_string()),
        placed_block: BlockPos { x: 1, y: 64, z: 0 },
        placed_block_name: Some("minecraft:stone".to_string()),
        hotbar_slot: 1,
        hotbar_item_id: Some(VANILLA_ITEM_STONE_ID),
        hotbar_item_count: INTERACTION_HELD_STACK_COUNT_AFTER_CLOSE,
        acknowledged_sequence: Some(INTERACTION_USE_ITEM_SEQUENCE),
        list_command_response: Some(format!("{LIST_COMMAND_TRANSLATION_KEY} probe")),
        list_command_responses: 2,
        invalid_command_signature_refusals: 1,
        command_suggestions: vec![
            CommandSuggestionObservation {
                command: INTERACTION_ROOT_SUGGESTION_COMMAND.into(),
                start: 0,
                length: 3,
                suggestions: vec!["list".into()],
            },
            CommandSuggestionObservation {
                command: INTERACTION_CHILD_SUGGESTION_COMMAND.into(),
                start: 5,
                length: 1,
                suggestions: vec!["uuids".into()],
            },
        ],
        ping_response_time: Some(INTERACTION_PING_TIME),
        container_click_cursor_item_id: None,
        container_click_cursor_item_count: 0,
        container_click_slot_item_id: None,
        container_click_slot_item_count: 0,
        pick_parked_slot_item_id: None,
        pick_parked_slot_item_count: 0,
        award_stats_packets: 1,
        awarded_statistics: 0,
        permission_refusals: 2,
        permission_level: "NoPermissions".to_string(),
        advancement_tab_packets: 2,
        advancement_tab_selected: None,
        book_item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WRITTEN_BOOK_ID),
        book_title: Some(INTERACTION_BOOK_TITLE.to_string()),
        book_author: Some("server-probe".to_string()),
        book_generation: Some(0),
        book_resolved: Some(true),
        book_pages: vec![INTERACTION_BOOK_PAGE.to_string()],
        note_block_name: Some(INTERACTION_NOTE_BLOCK_NAME.to_string()),
        redstone_wire_name: Some(INTERACTION_REDSTONE_WIRE_NAME.to_string()),
        redstone_wire_properties: INTERACTION_REDSTONE_WIRE_STATE
            .iter()
            .map(|(property, value)| ((*property).to_string(), (*value).to_string()))
            .collect(),
        note_block_properties: BTreeMap::from([(
            NOTE_BLOCK_NOTE_PROPERTY.to_string(),
            INTERACTION_NOTE_BLOCK_TUNED_NOTE.to_string(),
        )]),
        block_events: vec![BlockEventObservation {
            block_name: Some(INTERACTION_NOTE_BLOCK_NAME.to_string()),
            param_a: 0,
            param_b: 0,
            at_note_block: true,
            positioned_sounds_before: 1,
        }],
        sounds_heard: Vec::new(),
        spawned_entities: vec![test_drop_observation()],
        spawned_entity_count: 1,
        drop_observed: true,
        drop_picked_up: true,
        drop_slot_item_id: Some(test_drop_item_id()),
        drop_slot_item_count: INTERACTION_DROP_SLOT_COUNT_AFTER_PICKUP,
    }
}

#[test]
fn normalized_interaction_report_requires_every_visible_result() {
    let mut report = completed_replay_report();
    assert!(report.completed());
    report.acknowledged_sequence = Some(2);
    assert!(!report.completed());
    report.acknowledged_sequence = Some(INTERACTION_USE_ITEM_SEQUENCE);
    report.list_command_response = None;
    assert!(!report.completed());
    report.list_command_response = Some(LIST_COMMAND_TRANSLATION_KEY.to_string());
    // The signed request reaches the same dispatcher, so the reply must
    // arrive twice; the request that signed an argument no command reads
    // must be refused instead of adding a third.
    report.list_command_responses = 1;
    assert!(!report.completed());
    report.list_command_responses = 3;
    assert!(!report.completed());
    report.list_command_responses = 2;
    report.invalid_command_signature_refusals = 0;
    assert!(!report.completed());
    report.invalid_command_signature_refusals = 2;
    assert!(!report.completed());
    report.invalid_command_signature_refusals = 1;
    // Both completion requests must have been answered; the second one
    // rides the wave the book edits do, so one answer is a report taken
    // before that wave landed.
    let answered = report.command_suggestions.clone();
    report.command_suggestions.truncate(1);
    assert!(!report.completed());
    report.command_suggestions = answered;
    report.ping_response_time = Some(0);
    assert!(!report.completed());
    report.ping_response_time = Some(INTERACTION_PING_TIME);
    // The container session is the last thing the replay sends, so a
    // report taken before its answers arrived is not a completed one: the
    // cursor is still loaded, the clicked slot is still full, or the held
    // slot has not yet grown by what the close handed back.
    report.container_click_cursor_item_id = Some(VANILLA_ITEM_STONE_ID);
    assert!(!report.completed());
    report.container_click_cursor_item_id = None;
    report.container_click_slot_item_id = Some(VANILLA_ITEM_STONE_ID);
    assert!(!report.completed());
    report.container_click_slot_item_id = None;
    // A pick that answered the empty stack would have stocked the slot the
    // replay parked on, which no server may do.
    report.pick_parked_slot_item_id = Some(VANILLA_ITEM_STONE_ID);
    assert!(!report.completed());
    report.pick_parked_slot_item_id = None;
    // Both command-block editors refuse, and nothing else in that surface
    // says a word.
    report.permission_refusals = 1;
    assert!(!report.completed());
    report.permission_refusals = 3;
    assert!(!report.completed());
    report.permission_refusals = 2;
    // The statistics answer is what orders the request block before this
    // report is taken; what it carried is each server's own business.
    report.award_stats_packets = 0;
    assert!(!report.completed());
    report.award_stats_packets = 1;
    report.permission_level = String::new();
    assert!(!report.completed());
    report.permission_level = "NoPermissions".to_string();
    // Exactly two advancement tab moves: the root taken and cleared again.
    // One would mean the clear never arrived, three that a tab the handler
    // should have dropped was taken.
    report.advancement_tab_packets = 1;
    assert!(!report.completed());
    report.advancement_tab_packets = 3;
    assert!(!report.completed());
    report.advancement_tab_packets = 2;
    // The last tab the replay opens is not a root, so no server may end up
    // holding it.
    report.advancement_tab_selected = Some(INTERACTION_CHILD_ADVANCEMENT_TAB.to_string());
    assert!(!report.completed());
    report.advancement_tab_selected = None;
    // The signed book is the last result the replay produces, so a report
    // taken before `signBook` ran still holds a writable book.
    report.book_item_id = Some(bbb_protocol::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID);
    assert!(!report.completed());
    report.book_item_id = Some(bbb_protocol::item_types::VANILLA_ITEM_WRITTEN_BOOK_ID);
    report.book_title = None;
    assert!(!report.completed());
    report.book_title = Some(INTERACTION_BOOK_TITLE.to_string());
    report.hotbar_item_count = INTERACTION_HELD_STACK_COUNT_AFTER_CLOSE;
    // The redstone-wire leg. A place that never landed leaves the cell
    // air -- which is exactly what a server with no `RedStoneWireBlock`
    // answers -- and a shape that is not the cross means the connection
    // walk ran differently.
    report.redstone_wire_name = None;
    assert!(!report.completed());
    report.redstone_wire_name = Some(INTERACTION_REDSTONE_WIRE_NAME.to_string());
    report.redstone_wire_properties.clear();
    assert!(!report.completed());
    // A dot rather than a cross: the four arms `getConnectionState`
    // grows are exactly what a server that
    // stopped at `getMissingConnections` would be missing.
    report.redstone_wire_properties = INTERACTION_REDSTONE_WIRE_STATE
        .iter()
        .map(|(property, value)| {
            let dotted = if *property == "power" { *value } else { "none" };
            ((*property).to_string(), dotted.to_string())
        })
        .collect();
    assert!(!report.completed());
    report.redstone_wire_properties = INTERACTION_REDSTONE_WIRE_STATE
        .iter()
        .map(|(property, value)| ((*property).to_string(), (*value).to_string()))
        .collect();
    // The block-event leg. A place that never landed leaves nothing to
    // right-click, a note that never moved is a server that placed a
    // second block instead of tuning the first, and an event that landed
    // anywhere but on the note block is not the one the right-click
    // queued.
    report.note_block_name = None;
    assert!(!report.completed());
    report.note_block_name = Some(INTERACTION_NOTE_BLOCK_NAME.to_string());
    report.note_block_properties.clear();
    assert!(!report.completed());
    report.note_block_properties =
        BTreeMap::from([(NOTE_BLOCK_NOTE_PROPERTY.to_string(), "0".to_string())]);
    assert!(!report.completed());
    report.note_block_properties = BTreeMap::from([(
        NOTE_BLOCK_NOTE_PROPERTY.to_string(),
        INTERACTION_NOTE_BLOCK_TUNED_NOTE.to_string(),
    )]);
    report.block_events = vec![BlockEventObservation {
        at_note_block: false,
        ..report.block_events[0].clone()
    }];
    assert!(!report.completed());
    report.block_events = Vec::new();
    assert!(!report.completed());
    report.hotbar_item_count = INTERACTION_HELD_STACK_COUNT;
    assert!(!report.completed());
}

/// A plan whose positions are all distinct, which is what
/// `find_interaction_replay_plan` guarantees against a real world: the
/// note block sits directly on the block the replay places.
fn test_replay_plan() -> InteractionReplayPlan {
    InteractionReplayPlan {
        broken_block: BlockPos { x: 0, y: 63, z: 0 },
        clicked_block: BlockPos { x: 1, y: 63, z: 0 },
        placed_block: BlockPos { x: 1, y: 64, z: 0 },
        note_block: BlockPos { x: 1, y: 65, z: 0 },
        drop_base: BlockPos { x: 0, y: 63, z: 1 },
        redstone_clicked_block: BlockPos { x: -1, y: 63, z: 0 },
        redstone_wire_base: BlockPos { x: -1, y: 64, z: 0 },
    }
}

/// The cell the carpet stands in, which is where its drop has to appear.
fn drop_cell(drop_base: BlockPos) -> BlockPos {
    BlockPos {
        x: drop_base.x + INTERACTION_DROP_CELL_OFFSET[0],
        y: drop_base.y + INTERACTION_DROP_CELL_OFFSET[1],
        z: drop_base.z + INTERACTION_DROP_CELL_OFFSET[2],
    }
}

/// The item id the drop leg's block resolves to, looked up the way the
/// leg looks it up rather than spelled as a number.
fn test_drop_item_id() -> i32 {
    bbb_protocol::item_types::vanilla_item_type_for_resource_id(INTERACTION_DROP_BLOCK_NAME)
        .expect("the generated catalog names the drop block's item")
        .id
}

/// The drop the leg's carpet has to produce, as the completed replay
/// reports it.
fn test_drop_observation() -> SpawnedEntityObservation {
    SpawnedEntityObservation {
        entity_type: Some(INTERACTION_DROP_ENTITY_TYPE_NAME.to_string()),
        cell_offset: INTERACTION_DROP_CELL_OFFSET,
        item_id: Some(INTERACTION_DROP_BLOCK_NAME.to_string()),
        item_count: INTERACTION_DROP_ITEM_COUNT,
        entity_id: 7,
        uuid: Uuid::from_u128(0x1234).to_string(),
        position: [0.5, 64.125, 1.5],
        delta_movement: [0.0, 0.2, 0.0],
    }
}

/// The leg breaks the surface it stands on, so it may not stand on either
/// of the surfaces the other two legs are still using: the first is
/// already air, and the second is carrying the stone and the note block
/// that `placed_block_name` and `note_block_properties` read afterwards.
/// It also may not take a surface with something over it, because the
/// session stands on the carpet it lays there.
#[test]
fn the_drop_leg_takes_a_surface_neither_other_leg_is_using() {
    let surface = |x: i32, headroom: bool| {
        (
            BlockPos { x, y: 63, z: 0 },
            BlockPos { x, y: 64, z: 0 },
            headroom,
        )
    };
    let surfaces = [
        surface(0, true),
        surface(1, true),
        surface(2, false),
        surface(3, true),
    ];
    let broken_block = surfaces[0].0;
    let clicked_block = surfaces[1].0;

    assert_eq!(
        find_drop_surface(&surfaces, broken_block, clicked_block),
        Some(surfaces[3].0),
        "the third surface has no room above it for the session to stand in"
    );
    assert_eq!(
        find_drop_surface(&surfaces[..3], broken_block, clicked_block),
        None,
        "the other two surfaces are the other two legs' own"
    );
}

/// Standing on the carpet cell keeps any random drop wander inside the 1-block pickup box through the 10-tick delay.
#[test]
fn standing_on_the_carpet_keeps_the_whole_drop_cell_within_reach() {
    const POP_SPREAD: f64 = 0.25;
    const WANDER_PER_TICK: f64 = 0.1;
    const WANDER_DECAY: f64 = 0.98;
    const PICKUP_DELAY_TICKS: u32 = 10;
    const COLLISION_CHECK_SLACK_TICKS: u32 = 2;
    const PICKUP_INFLATION: f64 = 1.0;
    const PLAYER_HALF_WIDTH: f64 = 0.6 / 2.0;
    const DROP_HALF_WIDTH: f64 = 0.25 / 2.0;

    let mut speed = WANDER_PER_TICK;
    let mut wander = 0.0;
    for _ in 0..PICKUP_DELAY_TICKS + COLLISION_CHECK_SLACK_TICKS {
        wander += speed;
        speed *= WANDER_DECAY;
    }
    let reach = PLAYER_HALF_WIDTH + PICKUP_INFLATION + DROP_HALF_WIDTH;

    let drop_base = BlockPos { x: 4, y: 63, z: -7 };
    let stand = drop_pickup_stand(drop_base);
    let cell = drop_cell(drop_base);
    assert_eq!(
        [
            stand.x.floor() as i32,
            stand.y.floor() as i32,
            stand.z.floor() as i32
        ],
        [cell.x, cell.y, cell.z],
        "the session stands in the cell the drop is projected into"
    );
    assert_eq!(
        [stand.x, stand.y, stand.z],
        [
            f64::from(cell.x) + 0.5,
            f64::from(cell.y) + CARPET_TOP,
            f64::from(cell.z) + 0.5
        ],
        "in the middle of it, standing on the carpet rather than inside it"
    );
    assert!(
        POP_SPREAD + wander < reach,
        "a drop that wandered {wander} from the far side of the cell is out of a {reach} reach"
    );
    assert!(
        1.0 + POP_SPREAD + wander > reach,
        "one cell away is close enough after all, so this leg has nothing to guard"
    );
}

/// The pickup wait must end on the packet, not on either half of it
/// alone. `ItemEntity.playerTouch` announces the take, and the slot the
/// stack landed in arrives from the menu broadcast that runs after it —
/// so a report holding one and not the other is a report taken between
/// two packets, and the wait has to keep running.
#[test]
fn the_drop_leg_is_finished_only_once_the_take_and_the_slot_have_both_landed() {
    let mut report = completed_replay_report();
    assert!(report.drop_finished());
    report.drop_picked_up = false;
    assert!(!report.drop_finished());
    report.drop_picked_up = true;
    report.drop_slot_item_count = INTERACTION_DROP_ITEM_COUNT;
    assert!(
        !report.drop_finished(),
        "the take arrived but the stack the pickup moved has not"
    );
    report.drop_slot_item_count = INTERACTION_DROP_SLOT_COUNT_AFTER_PICKUP;
    assert!(report.drop_finished());
}

/// The pickup is not part of `completed()`. Everything `completed()` waits
/// for is an answer to a packet the replay sent, and a server that never
/// takes the item answers all of them — so folding the pickup in would
/// turn "this server did not pick the item up" into a probe that hangs
/// until its own timeout and says nothing about why.
#[test]
fn a_replay_that_was_never_picked_up_still_completes() {
    let mut report = completed_replay_report();
    report.drop_picked_up = false;
    report.drop_slot_item_count = INTERACTION_DROP_ITEM_COUNT;
    assert!(report.completed());
    assert!(!report.drop_finished());
}

/// The wait ends on the pickup when the pickup comes, and on the world
/// clock when it does not — never on the wall, and never on nothing.
#[test]
fn the_pickup_wait_ends_on_the_packet_or_on_the_world_clock() {
    // The item was already taken before the wait was ever armed.
    let mut deadline = None;
    assert!(drop_pickup_wait_ended(&mut deadline, true, Some(1_000)));
    assert_eq!(deadline, None, "a wait that never ran must arm nothing");

    // A server that never told this session its tick count cannot be
    // waited on, so the report is taken rather than the probe hung.
    let mut deadline = None;
    assert!(drop_pickup_wait_ended(&mut deadline, false, None));

    // The ordinary case: the wait arms, holds, and ends the moment the
    // take arrives — well inside the budget.
    let mut deadline = None;
    assert!(!drop_pickup_wait_ended(&mut deadline, false, Some(1_000)));
    assert_eq!(deadline, Some(1_000 + DROP_PICKUP_TICK_BUDGET));
    assert!(!drop_pickup_wait_ended(&mut deadline, false, Some(1_010)));
    assert!(drop_pickup_wait_ended(&mut deadline, true, Some(1_010)));

    // The failing case: no pickup ever arrives, and the budget ends the
    // wait so the difference is reported instead of timing the probe out.
    let mut deadline = None;
    assert!(!drop_pickup_wait_ended(&mut deadline, false, Some(1_000)));
    assert!(!drop_pickup_wait_ended(
        &mut deadline,
        false,
        Some(1_000 + DROP_PICKUP_TICK_BUDGET - 1)
    ));
    assert!(drop_pickup_wait_ended(
        &mut deadline,
        false,
        Some(1_000 + DROP_PICKUP_TICK_BUDGET)
    ));
}

/// The budget has to leave the pickup room to happen on a server that is
/// going to do it at all: nothing between the drop and the take but
/// `setDefaultPickUpDelay`'s ten ticks, since the player does not move
/// while the wait runs. A budget at or below that would report a slow
/// server as a server that never picked the item up.
#[test]
fn the_pickup_budget_is_far_longer_than_the_delay_it_waits_out() {
    const VANILLA_DEFAULT_PICKUP_DELAY: i64 = 10;
    const {
        assert!(
            DROP_PICKUP_TICK_BUDGET >= VANILLA_DEFAULT_PICKUP_DELAY * 5,
            "pickup budget must far exceed the default pickup delay"
        );
    }
}

/// `drop_observed` must name the leg's own drop and nothing else. Every
/// one of the four facts it reads is one the leg owns — an item entity,
/// in the cell the carpet stood in, carrying one carpet — so an entity
/// that misses any of them is something the world produced rather than
/// something the leg did.
#[test]
fn the_drop_flag_names_the_leg_drop_and_nothing_else() {
    assert!(test_drop_observation().is_the_leg_drop());
    let mut wrong_cell = test_drop_observation();
    wrong_cell.cell_offset = [0, 0, 0];
    assert!(!wrong_cell.is_the_leg_drop());
    let mut wrong_type = test_drop_observation();
    wrong_type.entity_type = Some("minecraft:experience_orb".to_string());
    assert!(!wrong_type.is_the_leg_drop());
    let mut wrong_item = test_drop_observation();
    wrong_item.item_id = Some("minecraft:stone".to_string());
    assert!(!wrong_item.is_the_leg_drop());
    let mut wrong_count = test_drop_observation();
    wrong_count.item_count = INTERACTION_DROP_ITEM_COUNT + 1;
    assert!(!wrong_count.is_the_leg_drop());
}

/// `Block.popResource` offsets the drop off the cell
/// centre by `Mth.nextDouble(random, -0.25, 0.25)` on each axis, and on
/// the vertical also down by half of `EntityType.ITEM`'s 0.25 height.
/// Those bounds are what make the cell a fact
/// two servers owe while the doubles are not, so the projection has to
/// hold across the whole range the random can produce — and the range is
/// half-open, so the upper end is approached and never reached.
#[test]
fn a_drop_is_placed_by_its_cell_across_the_whole_pop_resource_range() {
    let plan = test_replay_plan();
    let cell = drop_cell(plan.drop_base);
    let lowest = [-0.25_f64, -0.25 - 0.125, -0.25];
    let highest = [
        0.25 - f64::EPSILON,
        0.25 - 0.125 - f64::EPSILON,
        0.25 - f64::EPSILON,
    ];
    for offsets in [lowest, highest] {
        let snapshot = SpawnedEntitySnapshot {
            entity_type_id: bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(
                INTERACTION_DROP_ENTITY_TYPE_NAME,
            )
            .expect("the generated catalog names the item entity"),
            uuid: Uuid::from_u128(1),
            position: [
                f64::from(cell.x) + 0.5 + offsets[0],
                f64::from(cell.y) + 0.5 + offsets[1],
                f64::from(cell.z) + 0.5 + offsets[2],
            ],
            delta_movement: [0.0, 0.2, 0.0],
            item: None,
        };

        let observation = spawned_entity_observation(3, &snapshot, plan.drop_base);

        assert_eq!(
            observation.entity_type.as_deref(),
            Some(INTERACTION_DROP_ENTITY_TYPE_NAME)
        );
        assert_eq!(
            observation.cell_offset, INTERACTION_DROP_CELL_OFFSET,
            "a drop offset by {offsets:?} left the cell the carpet stood in"
        );
    }
}

/// A drop the leg never produced still has to be reported as an entity,
/// with the stack it carries read out of the generated `ItemEntity`
/// accessor rather than assumed. An entity that carries no stack reports
/// none and a count of zero, so a server that spawned something else is
/// visible instead of being dropped from the list.
#[test]
fn a_spawn_reports_the_stack_it_carries_and_none_when_it_carries_one() {
    let plan = test_replay_plan();
    let item_type = bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(
        INTERACTION_DROP_ENTITY_TYPE_NAME,
    )
    .expect("the generated catalog names the item entity");
    let drop_item =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id(INTERACTION_DROP_BLOCK_NAME)
            .expect("the generated catalog names the drop block's item");
    let mut snapshot = SpawnedEntitySnapshot {
        entity_type_id: item_type,
        uuid: Uuid::from_u128(2),
        position: [
            f64::from(drop_cell(plan.drop_base).x) + 0.5,
            f64::from(drop_cell(plan.drop_base).y) + 0.5,
            f64::from(drop_cell(plan.drop_base).z) + 0.5,
        ],
        delta_movement: [0.0, 0.2, 0.0],
        item: None,
    };

    let carried_nothing = spawned_entity_observation(4, &snapshot, plan.drop_base);
    assert_eq!(carried_nothing.item_id, None);
    assert_eq!(carried_nothing.item_count, 0);

    snapshot.item = Some(ItemStackSummary {
        item_id: Some(drop_item.id),
        count: INTERACTION_DROP_ITEM_COUNT,
        component_patch: DataComponentPatchSummary::default().into(),
    });
    let carried_the_drop = spawned_entity_observation(4, &snapshot, plan.drop_base);
    assert_eq!(
        carried_the_drop.item_id.as_deref(),
        Some(INTERACTION_DROP_BLOCK_NAME)
    );
    assert_eq!(carried_the_drop.item_count, INTERACTION_DROP_ITEM_COUNT);
}

/// `ItemEntity.DATA_ITEM` is resolved through the generated metadata
/// catalog, which answers only for the entity type whose accessor is
/// `ItemEntity`'s own — so the same synched-data index on another entity
/// must not be read as a dropped stack.
///
/// Both entities are built the way the wire builds them, through the
/// spawn and the metadata the server sends after it, because that
/// ordering is exactly what the recorder depends on: the spawn carries
/// the position, and the stack only ever arrives in the packet behind it.
#[test]
fn only_an_item_entity_reports_a_carried_stack() {
    let entity_type_id = |resource_id: &str| {
        bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(resource_id)
            .unwrap_or_else(|| panic!("the generated catalog names {resource_id}"))
    };
    let item_type = entity_type_id(INTERACTION_DROP_ENTITY_TYPE_NAME);
    let data_id =
        bbb_client_world::vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
            item_type,
        )
        .expect("the generated catalog resolves the item entity's stack accessor");
    let stack = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    // The armor stand is the negative case, and it is the catalog rather
    // than the absence of data that decides it: nothing resolves an
    // `ItemEntity` stack accessor for a type that is not `ItemEntity`.
    let armor_stand = entity_type_id("minecraft:armor_stand");
    assert_eq!(
        bbb_client_world::vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
            armor_stand
        ),
        None
    );

    let mut world = WorldStore::new();
    // Spawned out of id order, so the accessor's ordering is its own work
    // rather than the store's arrival order.
    for (id, spawned_type) in [(12, armor_stand), (11, item_type)] {
        world.apply_add_entity(packets::AddEntity {
            id,
            uuid: Uuid::from_u128(u128::from(u32::try_from(id).unwrap())),
            entity_type_id: spawned_type,
            position: packets::Vec3d {
                x: 0.5,
                y: 64.125,
                z: 0.5,
            },
            delta_movement: packets::Vec3d {
                x: 0.0,
                y: 0.2,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
    }
    assert!(world.apply_set_entity_data(packets::SetEntityData {
        id: 11,
        values: vec![packets::EntityDataValue {
            data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: packets::EntityDataValueKind::ItemStack(stack.clone()),
        }],
    }));

    let tracked = world.probe_entities();
    assert_eq!(
        tracked.iter().map(|state| state.id).collect::<Vec<_>>(),
        vec![11, 12],
        "the accessor must enumerate every tracked entity in id order"
    );
    assert_eq!(item_entity_stack(&tracked[0]), Some(stack));
    assert_eq!(item_entity_stack(&tracked[1]), None);
    assert_eq!(world.world_apply_error_messages(), Vec::<String>::new());
}

/// Every block action the replay sends has to keep the use in the air the
/// highest sequence it ever sent, because `ackBlockChangesUpTo` keeps the
/// maximum and `acknowledged_sequence` is compared against that use. The
/// drop leg's two actions also have to be distinct from each other and
/// from the note-block leg's.
#[test]
fn the_drop_leg_acknowledges_below_the_use_in_the_air() {
    let sequences = [
        INTERACTION_REDSTONE_BASE_PLACE_SEQUENCE,
        INTERACTION_NOTE_BLOCK_PLACE_SEQUENCE,
        INTERACTION_NOTE_BLOCK_TUNE_SEQUENCE,
        INTERACTION_REDSTONE_WIRE_PLACE_SEQUENCE,
        INTERACTION_DROP_PLACE_SEQUENCE,
        INTERACTION_DROP_BREAK_SEQUENCE,
    ];
    for sequence in sequences {
        assert!(
            sequence < INTERACTION_USE_ITEM_SEQUENCE,
            "sequence {sequence} would outrank the use the report reads"
        );
    }
    assert_eq!(
        sequences.iter().collect::<BTreeSet<_>>().len(),
        sequences.len(),
        "two block actions sharing a sequence make one of the acks unreadable"
    );
}

/// The drop leg stocks and selects a slot of its own. Sharing one with
/// another leg would make that leg's result a function of this one: the
/// held, clicked, parked, book and note-block slots are all read after
/// the drop leg has run.
#[test]
fn the_drop_leg_owns_a_slot_no_other_observation_reads() {
    let inventory_slots = [
        INTERACTION_PICK_PARKED_SLOT,
        INTERACTION_NOTE_BLOCK_INVENTORY_SLOT,
        INTERACTION_DROP_INVENTORY_SLOT,
    ];
    assert_eq!(
        inventory_slots.iter().collect::<BTreeSet<_>>().len(),
        inventory_slots.len()
    );
    let menu_slots = [
        FIRST_HOTBAR_MENU_SLOT,
        INTERACTION_CLICKED_MENU_SLOT,
        INTERACTION_BOOK_MENU_SLOT,
        INTERACTION_NOTE_BLOCK_MENU_SLOT,
        INTERACTION_DROP_MENU_SLOT,
    ];
    assert_eq!(
        menu_slots.iter().collect::<BTreeSet<_>>().len(),
        menu_slots.len()
    );
    // The two numberings have to name the same slot, or the creative
    // write and the selection would be aimed at different places.
    assert_eq!(
        i16::from(INTERACTION_DROP_INVENTORY_SLOT) + FIRST_HOTBAR_MENU_SLOT,
        INTERACTION_DROP_MENU_SLOT
    );
}

/// The note block's own sound comes out of the block, not out of whoever
/// hears it, so the replay's position test is the one thing that has to be
/// taken against the block. `ClientboundSoundPacket` truncates the
/// position to eighths (:15, :25-43), which for a block centre is exact —
/// so a sound half a block off is a different cell rather than rounding.
#[test]
fn a_note_sound_is_placed_by_the_cell_it_came_out_of() {
    let world = WorldStore::new();
    let plan = test_replay_plan();
    let at_block = |x: f64, y: f64, z: f64| SoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some("minecraft:block.note_block.harp".to_string()),
            fixed_range: None,
        },
        source: "record".to_string(),
        position: packets::Vec3d { x, y, z },
        volume: 3.0,
        pitch: 1.0,
        seed: 0,
        distance_delay: false,
    };
    let center = [
        f64::from(plan.note_block.x) + 0.5,
        f64::from(plan.note_block.y) + 0.5,
        f64::from(plan.note_block.z) + 0.5,
    ];
    let report = observe_interaction_replay(
        &world,
        plan,
        &[],
        &[
            at_block(center[0], center[1], center[2]),
            at_block(center[0], center[1] + 1.0, center[2]),
        ],
        &[],
        &BTreeMap::new(),
        &BTreeSet::new(),
    );

    assert_eq!(report.sounds_heard.len(), 2);
    assert!(report.sounds_heard[0].at_note_block);
    assert!(!report.sounds_heard[1].at_note_block);
}

#[test]
fn interaction_replay_predicts_creative_hotbar_without_clientbound_echo() {
    let mut world = WorldStore::new();
    world.apply_player_abilities(packets::PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });

    predict_interaction_replay_state(&mut world).unwrap();

    let plan = test_replay_plan();
    let report = observe_interaction_replay(
        &world,
        plan,
        &[],
        &[],
        &[],
        &BTreeMap::new(),
        &BTreeSet::new(),
    );
    assert_eq!(report.hotbar_slot, 1);
    assert_eq!(report.hotbar_item_id, Some(VANILLA_ITEM_STONE_ID));
    assert_eq!(report.hotbar_item_count, 32);
    assert!(!report.completed());
}

#[test]
fn interaction_replay_report_uses_canonical_selected_hotbar_slot() {
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(3));
    let plan = test_replay_plan();

    let report = observe_interaction_replay(
        &world,
        plan,
        &[],
        &[],
        &[],
        &BTreeMap::new(),
        &BTreeSet::new(),
    );

    assert_eq!(report.hotbar_slot, 3);
    assert_eq!(report.hotbar_item_id, None);
    assert_eq!(report.hotbar_item_count, 0);
    assert!(!report.completed());
}

#[test]
fn session_authority_needs_both_server_announcements() {
    let mut world = WorldStore::new();
    // The login alone already gives the client abilities, derived from its
    // game mode, so a server that never announced them would otherwise look
    // identical to one that did.
    world.apply_player_abilities(packets::PlayerAbilities::for_game_type(
        bbb_protocol::game_types::GameType::Creative,
    ));
    assert!(observe_session_authority(&world).is_none());

    world.apply_change_difficulty(packets::ChangeDifficulty {
        difficulty: bbb_protocol::difficulties::Difficulty::Peaceful,
        locked: false,
    });
    let report = observe_session_authority(&world).expect("both halves arrived");
    assert_eq!(report.difficulty, "peaceful");
    assert!(!report.difficulty_locked);
    assert_eq!(report.difficulty_updates, 1);
    assert_eq!(report.ability_updates, 1);
    assert!(report.abilities.can_fly);
    assert!(!report.abilities.flying);

    // A second announcement is exactly what a server that granted the
    // replay's request would send, so the count has to keep growing.
    world.apply_change_difficulty(packets::ChangeDifficulty {
        difficulty: INTERACTION_REQUESTED_DIFFICULTY,
        locked: true,
    });
    let report = observe_session_authority(&world).expect("both halves arrived");
    assert_eq!(report.difficulty, "hard");
    assert!(report.difficulty_locked);
    assert_eq!(report.difficulty_updates, 2);
}

/// Builds the two packets a joining player produces for an observer, in
/// the order `ClientPacketListener` requires (`PlayerInfoUpdate` first, or
/// the spawn is dropped).
fn peer_join_packets(profile_id: Uuid, entity_id: i32) -> Vec<packets::PlayClientbound> {
    vec![
        packets::PlayClientbound::PlayerInfoUpdate(packets::PlayerInfoUpdate {
            actions: vec![
                bbb_protocol::player_info_actions::PlayerInfoAction::AddPlayer,
                bbb_protocol::player_info_actions::PlayerInfoAction::UpdateListed,
                bbb_protocol::player_info_actions::PlayerInfoAction::UpdateGameMode,
                bbb_protocol::player_info_actions::PlayerInfoAction::UpdateHat,
            ],
            entries: vec![packets::PlayerInfoEntry {
                profile_id,
                profile: Some(packets::GameProfile {
                    uuid: profile_id,
                    name: PEER_USERNAME.to_string(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: bbb_protocol::game_types::GameType::Creative,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            }],
        }),
        packets::PlayClientbound::AddEntity(packets::AddEntity {
            id: entity_id,
            uuid: profile_id,
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: packets::Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
            delta_movement: packets::Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        }),
    ]
}

/// Runs a whole walk on paper and hands back every position it claims.
fn walked_positions(
    from: packets::Vec3d,
    target: packets::Vec3d,
    keep_clear: f64,
) -> Vec<packets::Vec3d> {
    let mut position = from;
    let mut claimed = Vec::new();
    // Far more hops than any walk under this gate needs, so a planner that
    // stopped converging fails the length assertion below instead of
    // hanging the test.
    for _ in 0..64 {
        let Some(hop) = walk_hop(position, target, keep_clear) else {
            return claimed;
        };
        position = hop.position;
        claimed.push(position);
        if hop.arrives {
            return claimed;
        }
    }
    panic!("the walk from {from:?} to {target:?} never arrived");
}

fn apart(a: packets::Vec3d, b: packets::Vec3d) -> f64 {
    let delta = [a.x - b.x, a.y - b.y, a.z - b.z];
    (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt()
}

/// How far from the primary a hop has to land to be clear of everything
/// the interaction replay built around it.
///
/// The placed block and the note block on top of it are one cell from
/// where the replay stands, and a player's box is 0.6 wide, so two and a
/// bit blocks already clears them. Asserting against this rather than
/// against [`WALK_TARGET_STANDOFF`] keeps the test measuring the world
/// instead of measuring the constant it guards.
const REPLAY_STRUCTURE_REACH: f64 = 3.0;

/// The peer's approach is what puts it in reach of the primary, and every
/// hop before the last one lands somewhere nobody has vouched for. The
/// last hop lands on the primary itself, which is why it is allowed to
/// come that close.
#[test]
fn the_peer_approach_keeps_every_hop_but_the_last_clear_of_its_target() {
    let target = packets::Vec3d {
        x: 0.5,
        y: -59.9375,
        z: 0.5,
    };
    // Two players `PlayerSpawnFinder` dropped inside the same square
    // around spawn are at most the square's diagonal apart, and the
    // official replay's respawn radius is ten blocks.
    for tenths in 0..=290 {
        let from = packets::Vec3d {
            x: target.x + f64::from(tenths) / 10.0,
            y: -60.0,
            z: target.z,
        };
        let claimed = walked_positions(from, target, WALK_TARGET_STANDOFF);
        let Some((last, before)) = claimed.split_last() else {
            assert_eq!(from, target, "only an arrived walk claims nothing");
            continue;
        };
        assert_eq!(*last, target, "the walk from {from:?} must land on it");
        let mut previous = from;
        for hop in &claimed {
            assert!(
                apart(previous, *hop) <= WALK_HOP_BLOCKS + 1e-9,
                "hop {hop:?} of the walk from {from:?} is longer than one tick allows"
            );
            previous = *hop;
        }
        for hop in before {
            assert!(
                apart(*hop, target) >= REPLAY_STRUCTURE_REACH,
                "hop {hop:?} of the walk from {from:?} lands in what the replay built"
            );
        }
    }
}

/// The outward walk follows a pre-cleared route, so it asks for no
/// standoff and keeps taking full hops right up to its target.
#[test]
fn a_walk_without_a_standoff_takes_full_hops() {
    let from = packets::Vec3d {
        x: 0.0,
        y: 64.0,
        z: 0.0,
    };
    let target = packets::Vec3d {
        x: 32.0,
        y: 64.0,
        z: 0.0,
    };
    let claimed = walked_positions(from, target, 0.0);
    let xs = claimed.iter().map(|hop| hop.x).collect::<Vec<_>>();
    assert_eq!(xs, vec![4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 28.0, 32.0]);
}

#[test]
fn peer_observation_needs_both_halves_of_the_join_and_of_the_removal() {
    let profile_id = Uuid::from_u128(0x1234);
    let entity_id = 77;
    let mut world = WorldStore::new();
    let mut observer = PeerObserver {
        profile_id,
        joined: None,
    };

    let mut sounds: Vec<SoundEventState> = Vec::new();
    // A sound is compared against where the observer itself stands, so the
    // fixture world needs the observer's own pose before it hears one.
    world.apply_player_position(packets::PlayerPositionUpdate {
        id: 1,
        position: OBSERVER_POSITION,
        delta_movement: packets::Vec3d::default(),
        y_rot: 0.0,
        x_rot: 0.0,
        relatives_mask: 0,
    });
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Joining
    ));

    let packets = peer_join_packets(profile_id, entity_id);
    world.apply_player_info_update(match &packets[0] {
        packets::PlayClientbound::PlayerInfoUpdate(update) => update.clone(),
        _ => unreachable!(),
    });
    // The player list entry alone is not a visible peer: the avatar has
    // not spawned yet.
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Joining
    ));

    world.apply_add_entity(match &packets[1] {
        packets::PlayClientbound::AddEntity(entity) => entity.clone(),
        _ => unreachable!(),
    });
    // A visible avatar is the cue to start the movement-state replay, not
    // the end of the observation.
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Replay(PeerReplayStep::StartSprinting)
    ));

    // Until the flags byte actually changes there is nothing to record, so
    // the step must not advance on its own.
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Joining
    ));

    publish_shared_flags(&mut world, entity_id, VANILLA_ENTITY_SHARED_FLAG_SPRINTING);
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Replay(PeerReplayStep::ShiftKeyDown)
    ));

    publish_shared_flags(
        &mut world,
        entity_id,
        VANILLA_ENTITY_SHARED_FLAG_SPRINTING | VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
    );
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Replay(PeerReplayStep::StopSprintingAndFallFlying)
    ));

    publish_shared_flags(
        &mut world,
        entity_id,
        VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
    );
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Replay(PeerReplayStep::InteractPrimary)
    ));

    // The interaction clears the crouch bit the input packet set, which is
    // the same byte every earlier step settled on.
    publish_shared_flags(&mut world, entity_id, 0);
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Replay(PeerReplayStep::AttackPrimary)
    ));

    // The attack changes no flag at all, so only a sound can settle it.
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Joining
    ));
    sounds.push(peer_attack_sound());
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Joined
    ));

    // Removing only the list entry leaves the avatar behind, which is the
    // exact half-disconnect regression this observation guards.
    world.apply_player_info_remove(packets::PlayerInfoRemove {
        profile_ids: vec![profile_id],
    });
    assert!(matches!(
        observer.observe(&world, &sounds),
        PeerPhase::Joined
    ));

    world.apply_remove_entities(packets::RemoveEntities {
        entity_ids: vec![entity_id],
    });
    let PeerPhase::Left(report) = observer.observe(&world, &sounds) else {
        panic!("both removals were applied");
    };
    assert_eq!(report.name, PEER_USERNAME);
    assert!(report.listed);
    assert_eq!(report.game_mode, "creative");
    assert!(report.show_hat);
    assert_eq!(
        report.entity_type_id,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID
    );
    assert!(report.removed_from_player_list);
    assert!(report.entity_removed);
    assert_eq!(
        report.shared_flags,
        PeerSharedFlagsReport {
            at_join: 0,
            after_start_sprinting: VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
            after_shift_key_down: VANILLA_ENTITY_SHARED_FLAG_SPRINTING
                | VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            after_stop_sprinting_and_fall_flying: VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            after_interact: 0,
        }
    );
    assert_eq!(
        report.sounds_heard,
        vec![SoundObservation {
            sound: "minecraft:entity.player.attack.nodamage".to_string(),
            source: "players".to_string(),
            volume: 1.0,
            pitch: 1.0,
            at_local_player_position: true,
        }]
    );
}

/// Where the observer stands in the fixture world.
const OBSERVER_POSITION: packets::Vec3d = packets::Vec3d {
    x: 0.5,
    y: 65.0,
    z: 0.5,
};

/// The no-damage sound as the peer's own attack would deliver it: the peer
/// walked onto the observer first, so it is emitted where the observer is.
fn peer_attack_sound() -> SoundEventState {
    SoundEventState {
        sound: bbb_client_world::SoundHolderState {
            kind: "reference".to_string(),
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.player.attack.nodamage",
            ),
            location: None,
            fixed_range: None,
        },
        source: "players".to_string(),
        position: OBSERVER_POSITION,
        volume: 1.0,
        pitch: 1.0,
        seed: 0,
        distance_delay: false,
    }
}

/// Publishes what `ServerEntity.sendDirtyEntityData` would have flushed for
/// the shared-flags byte.
fn publish_shared_flags(world: &mut WorldStore, entity_id: i32, flags: i8) {
    assert!(world.apply_set_entity_data(packets::SetEntityData {
        id: entity_id,
        values: vec![packets::EntityDataValue {
            data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: packets::EntityDataValueKind::Byte(flags),
        }],
    }));
}
