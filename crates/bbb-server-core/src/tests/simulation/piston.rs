//! `PistonMovingBlockEntity`'s entity half over a real level

use super::*;

use crate::entity::{AreaEffectCloudData, ArmorStandData, EntityData, MarkerData};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    VANILLA_ENTITY_TYPE_MARKER_ID,
};

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

/// The piston sits in chunk (0,0), which the session's spawn loads.
fn cell(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

const PISTON: (i32, i32, i32) = (4, 64, 0);
const ARM: (i32, i32, i32) = (5, 64, 0);
const POWER: (i32, i32, i32) = (3, 64, 0);

fn piston_state(facing: &str, extended: bool, sticky: bool) -> i32 {
    let name = if sticky {
        "minecraft:sticky_piston"
    } else {
        "minecraft:piston"
    };
    helpers::state_with(
        name,
        &[
            ("facing", facing),
            ("extended", if extended { "true" } else { "false" }),
        ],
    )
}

fn place(world: &mut ServerWorld, (x, y, z): (i32, i32, i32), state: i32) {
    let pos = cell(x, y, z);
    // `Level.setBlock` answers false for a write that changes nothing
    // which the generated floor already satisfies.
    if world.block_state(DIMENSION, pos) == Some(state) {
        return;
    }
    assert!(
        world.set_block(DIMENSION, pos, state, SET_BLOCK_UPDATE_ALL),
        "the cell at {pos:?} is writable"
    );
}

/// A floor at `y=63` so the test entity rests instead of falling.
fn floor(world: &mut ServerWorld, from_x: i32, to_x: i32) {
    let stone = block_state_id("minecraft:stone");
    for x in from_x..=to_x {
        place(world, (x, 63, 0), stone);
    }
}

fn spawn_armor_stand(world: &mut ServerWorld, position: Vec3d) -> i32 {
    world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            position,
            EntityData::ArmorStand(ArmorStandData::new().expect("the armor stand has attributes")),
        )
        .0
}

fn spawn_marker(world: &mut ServerWorld, position: Vec3d) -> i32 {
    world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_MARKER_ID,
            position,
            EntityData::Marker(MarkerData::new()),
        )
        .0
}

fn position_of(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .expect("the test entity is stored")
        .position()
}

fn piston_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // The piston cells live in chunk (0,0), which loads with the session.
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 4.5,
        },
    );
    (sender, receiver, world)
}

/// `PistonMovingBlockEntity.moveCollidedEntities` (`:120-189`): the entity in
/// the swept band leaves with the piston instead of stopping it.
#[test]
fn an_extending_piston_pushes_the_entity_in_its_path() {
    let (_, receiver, mut world) = piston_world();
    floor(&mut world, 3, 6);
    place(&mut world, PISTON, piston_state("east", false, false));
    let stand = spawn_armor_stand(
        &mut world,
        Vec3d {
            x: 5.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let before = position_of(&world, stand);
    place(
        &mut world,
        POWER,
        block_state_id("minecraft:redstone_block"),
    );

    for _ in 0..4 {
        world.tick(receiver.drain());
    }

    let after = position_of(&world, stand);
    assert!(
        after.x > before.x + 0.5,
        "the piston pushed the armor stand east: {before:?} -> {after:?}"
    );
    assert_eq!(after.y, before.y, "the push is horizontal");
    // The head finished in the arm cell (`PistonHeadBlock` facing east).
    let head = world
        .block_state(DIMENSION, cell(ARM.0, ARM.1, ARM.2))
        .expect("the arm cell is loaded");
    let identity = bbb_protocol::block_state_types::vanilla_block_state_identity(head)
        .expect("a built-in state");
    assert_eq!(identity.name, "minecraft:piston_head");
}

/// `:147-165`: a moved slime block hands the entity its own velocity rather
/// than displacing it, and a player is exempt.
#[test]
fn a_moving_slime_block_launches_the_entity_instead_of_pushing_it() {
    let (_, receiver, mut world) = piston_world();
    floor(&mut world, 3, 7);
    place(&mut world, PISTON, piston_state("east", false, false));
    // A slime block resting on the ground drags the ground line with it
    // (`PistonStructureResolver.addBranchingBlocks`, `:164-176`), so the
    // pushed block floats.
    place(
        &mut world,
        (ARM.0, ARM.1 - 1, ARM.2),
        block_state_id("minecraft:air"),
    );
    place(&mut world, ARM, block_state_id("minecraft:slime_block"));
    let stand = spawn_armor_stand(
        &mut world,
        Vec3d {
            x: 6.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let before = position_of(&world, stand);
    place(
        &mut world,
        POWER,
        block_state_id("minecraft:redstone_block"),
    );

    // The bounce lands in the block-entity phase, after the entity phase of
    // the same tick, so it shows up as the entity's own velocity at the tick's
    // end and the displacement it would have produced is never applied.
    let mut bounced = None;
    for _ in 0..4 {
        world.tick(receiver.drain());
        let entity = world
            .stored_entity_for_test(DIMENSION, stand)
            .expect("the armor stand is stored");
        if entity.delta_movement().x != 0.0 {
            bounced = Some((entity.position(), entity.delta_movement()));
            break;
        }
    }
    let (position, velocity) = bounced.expect("the moving slime block bounced the armor stand");
    assert_eq!(
        velocity.x, 1.0,
        "the slime block sets the eastward velocity (`:146-161`)"
    );
    // The `break` at `:162` leaves the inner loop for the displacement below,
    // so the same tick also pushes the entity out of the swept band.
    assert!(
        position.x > before.x,
        "the bounce does not replace the displacement: {before:?} -> {position:?}"
    );
    // The velocity is the entity's own from the next tick on.
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    assert!(position_of(&world, stand).x > before.x + 0.5);
}

/// `:141-142`: a `PushReaction.IGNORE` entity is skipped, so the piston
/// passes through it.
#[test]
fn an_ignored_entity_is_not_pushed_and_does_not_stop_the_piston() {
    let (_, receiver, mut world) = piston_world();
    floor(&mut world, 3, 6);
    place(&mut world, PISTON, piston_state("east", false, false));
    let at = Vec3d {
        x: 5.5,
        y: 64.0,
        z: 0.5,
    };
    let marker = spawn_marker(&mut world, at);
    let cloud = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
            at,
            EntityData::AreaEffectCloud(AreaEffectCloudData::new()),
        )
        .0;
    let before = position_of(&world, marker);
    place(
        &mut world,
        POWER,
        block_state_id("minecraft:redstone_block"),
    );

    for _ in 0..4 {
        world.tick(receiver.drain());
    }

    assert_eq!(position_of(&world, marker), before, "the marker stays put");
    assert_eq!(position_of(&world, cloud), before, "the cloud stays put");
    let head = world
        .block_state(DIMENSION, cell(ARM.0, ARM.1, ARM.2))
        .expect("the arm cell is loaded");
    let identity = bbb_protocol::block_state_types::vanilla_block_state_identity(head)
        .expect("a built-in state");
    assert_eq!(
        identity.name, "minecraft:piston_head",
        "the piston still extended through the ignored entity"
    );
}

/// `:200-216`: a moving honey block carries what stands on it.
#[test]
fn a_moving_honey_block_carries_the_entity_standing_on_it() {
    let (_, receiver, mut world) = piston_world();
    floor(&mut world, 3, 7);
    place(&mut world, PISTON, piston_state("east", false, false));
    // Free-floating, for the same branching reason as the slime block above.
    place(
        &mut world,
        (ARM.0, ARM.1 - 1, ARM.2),
        block_state_id("minecraft:air"),
    );
    place(&mut world, ARM, block_state_id("minecraft:honey_block"));
    // Standing on the honey block's top face, which is 15/16 of a block
    // (`HoneyBlock.getCollisionShape`).
    let stand = spawn_armor_stand(
        &mut world,
        Vec3d {
            x: 5.5,
            y: 64.9375,
            z: 0.5,
        },
    );
    // `matchesStickyCritera` (`:218-220`) wants the entity on the ground.
    for _ in 0..2 {
        world.tick(receiver.drain());
    }
    let before = position_of(&world, stand);
    place(
        &mut world,
        POWER,
        block_state_id("minecraft:redstone_block"),
    );

    for _ in 0..4 {
        world.tick(receiver.drain());
    }

    let after = position_of(&world, stand);
    assert!(
        after.x > before.x + 0.5,
        "the honey block carried the armor stand east: {before:?} -> {after:?}"
    );
}

/// A moving piston saved mid-animation resumes the same mechanism after a
/// restore: the block entity keeps its progress, advances and still pushes.
#[test]
fn a_saved_moving_piston_resumes_and_still_pushes() {
    let (_, receiver, mut world) = piston_world();
    floor(&mut world, 3, 6);
    place(&mut world, PISTON, piston_state("east", false, false));
    place(
        &mut world,
        POWER,
        block_state_id("minecraft:redstone_block"),
    );
    // One tick applies the plan and advances both moving block entities to 0.5.
    world.tick(receiver.drain());
    let live = world
        .level_block_entity_for_test(DIMENSION, cell(ARM.0, ARM.1, ARM.2))
        .as_deref()
        .and_then(BlockEntity::piston_snapshot)
        .expect("the moving piston is live at the arm cell");
    assert_eq!(live.progress, 0.5);

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the world saves");
    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the piston's column is written");
    let parsed = Java26ChunkData::parse(
        created.document.clone(),
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .expect("the runtime's own document is a valid Java 26.1 chunk");
    // `saveAdditional` (`:354-361`) writes `progressO`, the tick before the
    // live one, which is what the restored entity resumes from.
    assert!(
        parsed
            .block_entities()
            .iter()
            .any(|record| field(record, "progress") == &NbtSummaryValue::Float(0.0_f32.to_bits())),
        "the moving piston's progress is in the saved column: {:?}",
        parsed.block_entities()
    );

    let candidate = restore_candidate(world.game_time(), Some(parsed), Vec::new());
    let mut restored = ServerWorld::restore(ServerWorldConfig::default(), &candidate)
        .expect("the saved column restores");
    // The restored entity resumes at 0 and reaches 0.5 on its first tick, so
    // the entity enters the band the second tick sweeps.
    restored.tick(Vec::new());
    let stand = spawn_armor_stand(
        &mut restored,
        Vec3d {
            x: 6.0,
            y: 64.0,
            z: 0.5,
        },
    );
    let before = position_of(&restored, stand);
    for _ in 0..2 {
        restored.tick(Vec::new());
    }

    assert!(
        position_of(&restored, stand).x > before.x + 0.2,
        "the restored moving piston pushed the armor stand: {before:?} -> {:?}",
        position_of(&restored, stand)
    );
    let head = restored
        .block_state(DIMENSION, cell(ARM.0, ARM.1, ARM.2))
        .expect("the arm cell is loaded");
    let identity = bbb_protocol::block_state_types::vanilla_block_state_identity(head)
        .expect("a built-in state");
    assert_eq!(identity.name, "minecraft:piston_head");
}

// PistonMovingBlockEntity: movement does not teleport the player.
#[test]
fn an_extending_piston_pushes_a_standing_player() {
    let (_, receiver, mut world) = piston_world();
    floor(&mut world, 3, 6);
    place(&mut world, PISTON, piston_state("east", false, false));
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 5.5,
            y: 64.0,
            z: 0.5,
        },
    );
    place(
        &mut world,
        POWER,
        block_state_id("minecraft:redstone_block"),
    );

    for _ in 0..4 {
        let published = world.tick(receiver.drain());
        assert!(!published.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(_),
                ..
            }
        )));
    }
    let pushed = world.player_movement_validation_for_test(ConnectionId(1)).0;
    assert!(pushed.x > 5.5, "the player is pushed east: {pushed:?}");
    assert_eq!(pushed.y, 64.0, "the push is horizontal");
}

/// The second piston, facing back at the first, so one armor stand can be
/// pushed twice without either head ever entering the other's cell.
const RETURN_PISTON: (i32, i32, i32) = (7, 64, 0);
const RETURN_POWER: (i32, i32, i32) = (8, 64, 0);

/// A climbable floor, so the emission's predicate holds on
/// a push: a horizontal `Entity.move` clears `onGround` before the emission
/// reads it, and `isStateClimbable(effectState)` is the only arm
/// of that predicate a stored body can satisfy while being pushed.
fn scaffolding_floor(world: &mut ServerWorld, from_x: i32, to_x: i32) {
    let stone = block_state_id("minecraft:stone");
    for x in from_x..=to_x {
        place(world, (x, 62, 0), stone);
        place(world, (x, 63, 0), scaffolding_state());
    }
}

/// Resting on the stone below it, so `ScaffoldingBlock` keeps it standing.
fn scaffolding_state() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:scaffolding",
        &BTreeMap::from([
            ("bottom".to_owned(), "false".to_owned()),
            ("distance".to_owned(), "0".to_owned()),
            ("waterlogged".to_owned(), "false".to_owned()),
        ]),
    )
    .expect("a built-in Java 26.1 scaffolding state")
}

/// The `ClientboundSoundPacket`s one tick sent on `SoundSource.NEUTRAL`, which
/// is the channel `Entity.playStepSound` uses.
fn step_sounds(tick: &TickResult) -> Vec<(i32, Vec3d)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Sound(sound),
                ..
            } if sound.source == SoundSource::Neutral => match sound.sound {
                SoundEventHolder::Reference { registry_id } => Some((registry_id, sound.position)),
                SoundEventHolder::Direct { .. } => None,
            },
            _ => None,
        })
        .collect()
}

fn power(world: &mut ServerWorld, cell: (i32, i32, i32), on: bool) {
    let state = if on {
        block_state_id("minecraft:redstone_block")
    } else {
        block_state_id("minecraft:air")
    };
    place(world, cell, state);
}

/// `PistonMovingBlockEntity.moveEntityByPiston` (`:192-198`) is a real
/// `Entity.move(MoverType.PISTON, ..)`, so its tail runs
/// `applyMovementEmissionAndPlaySound` like any other
/// move: `moveDist` advances with the push and, once it passes `nextStep`, the
/// pushed body plays the floor's step sound and raises `GameEvent.STEP`.
///
/// One push clears the swept band and no more, which is about half a block, so
/// the two pistons hand the stand back and forth until the `0.6F`-scaled
/// travel passes the `1.0F` `nextStep` starts at.
/// An armor stand resting on the floor adds nothing of its own in between, so
/// every bit of that `moveDist` is the pushes'.
#[test]
fn a_piston_push_plays_the_step_sound_and_raises_step_once_move_dist_passes() {
    let (_, receiver, mut world) = piston_world();
    scaffolding_floor(&mut world, 2, 10);
    place(&mut world, PISTON, piston_state("east", false, false));
    place(
        &mut world,
        RETURN_PISTON,
        piston_state("west", false, false),
    );
    let stand = spawn_armor_stand(
        &mut world,
        Vec3d {
            x: 5.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let start = position_of(&world, stand);

    let scaffolding_step = vanilla_sound_event_registry_id("minecraft:block.scaffolding.step")
        .expect("the scaffolding step sound of the built-in catalog");
    let mut heard = None;
    let mut travelled = 0.0;
    let mut previous = start;
    'pushes: for cycle in 0..3 {
        for cell in [POWER, RETURN_POWER] {
            for on in [true, false] {
                power(&mut world, cell, on);
                for _ in 0..4 {
                    let published = world.tick(receiver.drain());
                    let now = position_of(&world, stand);
                    travelled += (now.x - previous.x).abs();
                    previous = now;
                    if let Some(sound) = step_sounds(&published)
                        .into_iter()
                        .find(|(id, _)| *id == scaffolding_step)
                    {
                        heard = Some((cycle, sound));
                        break 'pushes;
                    }
                }
            }
        }
    }

    let (_, (_, sound_position)) = heard.unwrap_or_else(|| {
        panic!("no push ever played the floor's step sound after {travelled} of travel")
    });
    // `Entity.playSound` plays at the entity, not at the block it walked.
    assert_eq!(sound_position, position_of(&world, stand));
    assert!(
        travelled > 1.0 / 0.6,
        "the step waited for `moveDist` to pass `nextStep`: {travelled}"
    );
    let (events, _) = world.game_events(DIMENSION);
    assert!(
        events.iter().any(
            |row| row.game_event == crate::level_event::VanillaGameEvent::STEP
                && row.source_entity_id == Some(stand)
                && row.affected_state == Some(scaffolding_state())
        ),
        "the same push raised STEP naming the stand and the floor it walked: {events:?}"
    );
}

/// `PistonMovingBlockEntity.moveCollidedEntities` (`:120-126`) reads the
/// empty-context shape, which for powder snow is empty: the push completes
/// and the snow lands in the next cell.
#[test]
fn an_extending_piston_moves_powder_snow() {
    let (_, receiver, mut world) = piston_world();
    floor(&mut world, 3, 7);
    place(&mut world, PISTON, piston_state("east", false, false));
    let snow = block_state_id("minecraft:powder_snow");
    place(&mut world, ARM, snow);
    place(
        &mut world,
        POWER,
        block_state_id("minecraft:redstone_block"),
    );

    for _ in 0..4 {
        world.tick(receiver.drain());
    }

    assert_eq!(world.block_state(DIMENSION, cell(6, 64, 0)), Some(snow));
}
