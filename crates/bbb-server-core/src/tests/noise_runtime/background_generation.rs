use super::*;
use crate::tests::entities::console_command;

fn command_result(tick: &TickResult) -> Option<i32> {
    let Ok(AdminResult::Command(report)) = &admin_outcome(tick).result else {
        panic!("a command.execute answers with a command report");
    };
    report.result
}

fn background_world() -> ServerWorld {
    ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(crate::chunk::noise_generator::overworld_startup(true)),
            background_generation: true,
            ..ServerWorldConfig::default()
        },
        &noise_candidate(),
    )
    .unwrap()
}

fn sent_chunks(events: &[WorldEvent]) -> Vec<(i32, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelChunkWithLight(chunk),
                ..
            } => Some((chunk.x, chunk.z)),
            _ => None,
        })
        .collect()
}

fn finished_batches(events: &[WorldEvent]) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::ChunkBatchFinished { .. },
                    ..
                }
            )
        })
        .count()
}

#[test]
fn the_sender_holds_back_columns_its_batches_have_not_committed() {
    let mut world = background_world();
    world.tick_rate_mut().set_frozen(true);
    let (sender, receiver) = mailbox();
    let joined = join_noise_player(&sender, &receiver, &mut world, 61);
    let spawn = world.publication().players[0].position;
    let center = (
        (spawn.x.floor() as i32).div_euclid(16),
        (spawn.z.floor() as i32).div_euclid(16),
    );
    let square = (center.0 - 1..=center.0 + 1)
        .flat_map(|x| (center.1 - 1..=center.1 + 1).map(move |z| (x, z)))
        .collect::<Vec<_>>();
    let mut delivered = sent_chunks(&joined.events);
    let mut pending_batches = finished_batches(&joined.events);
    let store = world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    assert!(
        delivered.iter().all(|&(x, z)| store.is_ticking_full(x, z)),
        "{delivered:?}"
    );
    assert!(
        square.iter().any(|chunk| !delivered.contains(chunk)),
        "the join tick generated the whole view on demand: {delivered:?}"
    );

    world.finish_generation_for_test();
    for _ in 0..4 {
        let tick = if pending_batches > 0 {
            pending_batches -= 1;
            play(
                &sender,
                &receiver,
                &mut world,
                61,
                PlayServerbound::ChunkBatchReceived {
                    desired_chunks_per_tick: 64.0,
                },
            )
        } else {
            world.tick(Vec::new())
        };
        assert!(closed_reasons(&tick).is_empty());
        pending_batches += finished_batches(&tick.events);
        delivered.extend(sent_chunks(&tick.events));
    }
    assert!(
        square.iter().all(|chunk| delivered.contains(chunk)),
        "{delivered:?}"
    );
}

#[test]
fn forceload_answers_at_once_and_its_columns_load_on_later_ticks() {
    let mut world = background_world();
    let (sender, receiver) = mailbox();
    let added = console_command(&sender, &receiver, &mut world, 1, "forceload add 320 320");
    assert_eq!(command_result(&added), Some(1));
    let loaded = |world: &mut ServerWorld, operation| {
        let tick = console_command(
            &sender,
            &receiver,
            world,
            operation,
            "execute if loaded 320 0 320",
        );
        command_result(&tick) == Some(1)
    };
    assert!(!loaded(&mut world, 2));
    let mut operation = 3;
    while !loaded(&mut world, operation) {
        assert!(operation < 20_000, "the forced column never loaded");
        operation += 1;
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(world
        .chunk_store_for_test(Java26BuiltinDimension::Overworld)
        .is_ticking_full(20, 20));
}
