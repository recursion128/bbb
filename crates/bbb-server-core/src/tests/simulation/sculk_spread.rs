//! `SculkCatalystBlockEntity.serverTick`
//! driving `SculkSpreader.updateCursors` on a
//! real level: the cursor's `updateDelay`/`decayDelay` clock, the charge the
//! `SculkBehaviour` owners spend, the vein/sculk/growth blocks they place, the
//! level events the spreader raises, and what a save and a reload do to the
//! list.

use super::*;

use crate::entity::{ArmorStandData, EntityData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::random::GameRandom;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID;
use bbb_protocol::packets::{
    Direction, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_DELTA_Z,
    PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y, PLAYER_RELATIVE_Y_ROT,
    PLAYER_RELATIVE_Z,
};
use bbb_server_types::sculk_cursor::{parse_cursors_nbt, SculkCursorFacts};
use bbb_server_worldgen::sculk_spreader::SCULK_BLOCK_SPREAD_SOUND;

use super::helpers::{at, sculk_cursor as cursor, state_with};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const CATALYST: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
/// `LevelEvent.PARTICLES_SCULK_CHARGE` (`LevelEvent.java:71`).
const PARTICLES_SCULK_CHARGE: i32 = 3006;

fn state(name: &str) -> i32 {
    block_state_id(name)
}

/// `minecraft:sculk_vein` with exactly the down face.
fn vein_down() -> i32 {
    state_with(
        "minecraft:sculk_vein",
        &[
            ("down", "true"),
            ("east", "false"),
            ("north", "false"),
            ("south", "false"),
            ("up", "false"),
            ("waterlogged", "false"),
            ("west", "false"),
        ],
    )
}

fn catalyst_state() -> i32 {
    helpers::catalyst_state(false)
}

/// A world whose chunk `(0, 0)` is forced into block-ticking range
/// (`restore_candidate` files the ticket), carrying the datapacks a live tag
/// lookup needs, with one sculk catalyst at [`CATALYST`] and no session.
fn catalyst_world() -> ServerWorld {
    let mut world =
        spawn_pinned_restore(datapack_config(), &restore_candidate(10, None, Vec::new()));
    assert!(world.set_block(DIM, CATALYST, catalyst_state(), SET_BLOCK_UPDATE_ALL));
    world
}

fn catalyst_session_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    helpers::catalyst_session_world(CATALYST)
}

/// `SculkSpreader`'s cursor list off the block entity, read back through the
/// same codec `load`/`save` use.
fn cursors(world: &ServerWorld) -> Vec<SculkCursorFacts> {
    let catalyst = world
        .get_block_entity(DIM, CATALYST)
        .expect("the catalyst's block entity")
        .sculk_catalyst()
        .expect("a sculk catalyst payload");
    parse_cursors_nbt(catalyst.cursors()).expect("the catalyst's cursor list decodes")
}

fn charge_spreader(world: &mut ServerWorld, facts: &[SculkCursorFacts]) {
    helpers::charge_spreader(world, CATALYST, facts);
}

/// The `3006` level events of one tick, as `(pos, data)`.
fn charge_particles(tick: &TickResult) -> Vec<(BlockPos, i32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelEvent(level_event),
                ..
            } if level_event.event_type == PARTICLES_SCULK_CHARGE => {
                Some((level_event.pos, level_event.data))
            }
            _ => None,
        })
        .collect()
}

/// The `minecraft:block.sculk.spread` sounds of one tick, with the volume and
/// pitch `SculkSpreader` plays them at (`:269`).
fn spread_sounds(tick: &TickResult) -> Vec<(BlockPos, f32, f32)> {
    let sound = vanilla_sound_event_registry_id(SCULK_BLOCK_SPREAD_SOUND)
        .expect("the spread sound is in the catalog");
    sounds_sent(tick)
        .into_iter()
        .filter(|sent| {
            matches!(
                &sent.sound,
                bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                    if *registry_id == sound
            )
        })
        .map(|sent| {
            (
                BlockPos {
                    x: sent.position.x.floor() as i32,
                    y: sent.position.y.floor() as i32,
                    z: sent.position.z.floor() as i32,
                },
                sent.volume,
                sent.pitch,
            )
        })
        .collect()
}

/// The seed whose first spreader draw — after `all_shuffled`'s five swaps —
/// answers `wanted`.
fn seed_with_charge_draw(wanted: impl Fn(i32) -> bool) -> i64 {
    (0_i64..)
        .find(|seed| {
            let draws = GameRandom::with_seed(*seed);
            for bound in [6, 5, 4, 3, 2] {
                draws.next_int(bound);
            }
            wanted(draws.next_int(10))
        })
        .expect("some seed draws the wanted value from nextInt(10)")
}

/// One cursor in an air pocket over the flat floor: `updateDelay` 1 spends a
/// tick counting down, the next tick spreads the vein the same-space spreader
/// attaches to the stone below, converts that stone to sculk, and moves the
/// cursor onto the new sculk (`SculkSpreader`).
#[test]
fn a_real_tick_advances_a_charged_cursor_through_the_vanilla_arithmetic() {
    let mut world = catalyst_world();
    let start = at(2, 64, 0);
    charge_spreader(&mut world, &[cursor(start, 5, 1, 1)]);

    // `if (this.updateDelay > 0) this.updateDelay--` (`:258-259`): nothing else
    // moves, not even the charge.
    world.tick(Vec::new());
    assert_eq!(cursors(&world), vec![cursor(start, 5, 1, 0)]);

    // The next tick runs the body: the same-space spreader attaches a vein to
    // the stone below, `attemptPlaceSculk` turns that stone into sculk, and the
    // cursor walks onto the patch, which discharges the vein it left behind
    world.tick(Vec::new());
    assert_eq!(world.block_state(DIM, start), Some(state("minecraft:air")));
    assert_eq!(
        world.block_state(DIM, at(2, 63, 0)),
        Some(state("minecraft:sculk"))
    );
    // `attemptUseCharge` spends one charge per placed sculk
    // the movement keeps the decay at 1
    // and arms one tick of delay.
    let after = cursors(&world);
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].charge, 4);
    assert_eq!(after[0].decay_delay, 1);
    assert_eq!(after[0].update_delay, 1);
    // `getValidMovementPos` (`:313-329`) walks the eighteen non-corner
    // neighbours and takes a `SculkBehaviour` cell, which here is the sculk it
    // just placed or one of the veins `attemptPlaceSculk`'s own spread left
    // around the patch.
    let moved = after[0].pos;
    assert_ne!(moved, start, "the cursor left the discharged vein");
    assert!(
        (moved.x - start.x)
            .abs()
            .max((moved.y - start.y).abs())
            .max((moved.z - start.z).abs())
            == 1,
        "the cursor moved to a neighbour: {moved:?}"
    );
    assert!(
        matches!(
            world.block_state(DIM, moved),
            Some(landed) if landed == state("minecraft:sculk") || landed == vein_down()
        ),
        "the cursor stands on a SculkBehaviour cell"
    );

    // The third tick is another pure countdown.
    world.tick(Vec::new());
    let counted = cursors(&world);
    assert_eq!(counted[0].charge, 4);
    assert_eq!(counted[0].decay_delay, 1);
    assert_eq!(counted[0].update_delay, 0);
}

/// The same-space spreader's vein write on a support the level can attach to
/// but not replace: the vein stays, `attemptPlaceSculk` finds no replaceable
/// support, and the charge survives the `chargeDecayRate` draw the vein
/// behaviour makes.
#[test]
fn a_spread_vein_persists_where_nothing_replaceable_supports_it() {
    let seed = seed_with_charge_draw(|draw| draw != 0);
    let mut world = catalyst_world();
    let start = at(2, 64, 0);
    assert!(world.set_block(
        DIM,
        at(2, 63, 0),
        state("minecraft:bedrock"),
        SET_BLOCK_UPDATE_ALL
    ));
    charge_spreader(&mut world, &[cursor(start, 5, 1, 0)]);

    world.tick_sculk_catalyst_seeded_for_test(&DIM, CATALYST, seed);
    assert_eq!(
        world.block_state(DIM, start),
        Some(vein_down()),
        "the vein attached to the only support, the bedrock below"
    );
    assert_eq!(
        world.block_state(DIM, at(2, 63, 0)),
        Some(state("minecraft:bedrock"))
    );
    // `availableFaces` of the vein the cursor now stands in (`:288-290`).
    assert_eq!(
        cursors(&world),
        vec![SculkCursorFacts {
            pos: start,
            charge: 5,
            decay_delay: 1,
            update_delay: 1,
            facings: Some(vec![Direction::Down]),
        }],
        "the cursor cannot walk anywhere and the draw kept the charge"
    );
}

/// `attemptUseCharge`'s `DEFAULT` arm answers 0
/// once `decayDelay` has aged out, and `updateCursors` then drops the cursor
/// with `LevelEvent.PARTICLES_SCULK_CHARGE` at data 0
#[test]
fn an_exhausted_cursor_leaves_the_list_and_raises_the_discharge_event() {
    let (_sender, receiver, mut world) = catalyst_session_world();
    // An air cell with nothing to attach a vein to: `attemptSpreadVein` fails,
    // so the behaviour stays `DEFAULT` and its `decayDelay == 0` arm answers 0.
    let stranded = at(2, 70, 0);
    charge_spreader(&mut world, &[cursor(stranded, 5, 0, 0)]);

    let tick = world.tick(receiver.drain());
    assert!(cursors(&world).is_empty(), "the cursor discharged");
    assert_eq!(
        world.block_state(DIM, stranded),
        Some(state("minecraft:air"))
    );
    assert_eq!(
        charge_particles(&tick),
        vec![(stranded, 0)],
        "the discharge carries data 0 in the tick the body raised it"
    );
}

/// `ChargeCursor.isPosUnreasonable`: a cursor
/// more than 1024 blocks away on the chessboard is dropped before it updates,
/// and that arm raises no event.
#[test]
fn a_cursor_past_the_maximum_distance_is_dropped_without_touching_the_level() {
    let (_sender, receiver, mut world) = catalyst_session_world();
    let far = at(1025, 64, 0);
    charge_spreader(&mut world, &[cursor(far, 5, 1, 0)]);

    world.tick(receiver.drain());
    assert!(cursors(&world).is_empty());
    let flushed = world.tick(receiver.drain());
    assert!(charge_particles(&flushed).is_empty());
    assert_eq!(world.block_state(DIM, far), Some(state("minecraft:air")));
}

/// `SculkSpreader`: at level time two cursors that end the tick
/// on the same cell merge while their charges stay at or below `MAX_CHARGE`,
/// and stay separate past it — the burst still reports their sum.
#[test]
fn cursors_on_one_cell_merge_up_to_the_charge_ceiling() {
    let cell = at(2, 70, 0);
    let (_sender, receiver, mut world) = catalyst_session_world();
    let mut first = cursor(cell, 600, 1, 1);
    let mut second = cursor(cell, 300, 1, 1);
    // `facings` present and empty is the state a cursor that walked onto sculk
    // carries (`availableFaces` of a non-multiface block), which is what makes
    // the position's burst fire at all (`:174-175`).
    first.facings = Some(Vec::new());
    second.facings = Some(Vec::new());
    charge_spreader(&mut world, &[first, second]);

    world.tick(receiver.drain());
    assert_eq!(
        cursors(&world),
        vec![SculkCursorFacts {
            pos: cell,
            charge: 900,
            decay_delay: 1,
            update_delay: 0,
            facings: Some(Vec::new()),
        }],
        "the merged cursor keeps the representative's delays"
    );
    // `(int)(Math.log1p(900) / 2.3F) + 1` is 3.
    let flushed = world.tick(receiver.drain());
    assert_eq!(charge_particles(&flushed), vec![(cell, 3 << 6)]);

    // 600 + 600 is over `MAX_CHARGE`, so neither cursor absorbs the other.
    let (_sender, receiver, mut world) = catalyst_session_world();
    let mut left = cursor(cell, 600, 1, 1);
    let mut right = cursor(cell, 600, 1, 1);
    left.facings = Some(Vec::new());
    right.facings = Some(Vec::new());
    charge_spreader(&mut world, &[left, right]);
    world.tick(receiver.drain());
    assert_eq!(
        cursors(&world),
        vec![
            SculkCursorFacts {
                pos: cell,
                charge: 600,
                decay_delay: 1,
                update_delay: 0,
                facings: Some(Vec::new()),
            },
            SculkCursorFacts {
                pos: cell,
                charge: 600,
                decay_delay: 1,
                update_delay: 0,
                facings: Some(Vec::new()),
            },
        ]
    );
    let flushed = world.tick(receiver.drain());
    // `(int)(Math.log1p(1200) / 2.3F) + 1` is 4.
    assert_eq!(charge_particles(&flushed), vec![(cell, 4 << 6)]);
}

/// A placement the level refuses: the cursor stands one block above the top of
/// the world, where the same-space spreader still finds the stone below as an
/// attachable support, so the write is attempted at a cell `Level.setBlock`
/// refuses — the charge is kept and only the delay clock moves
#[test]
fn a_placement_the_level_refuses_keeps_the_charge() {
    let mut world = catalyst_world();
    let support = at(2, 319, 0);
    assert!(world.set_block(DIM, support, state("minecraft:stone"), SET_BLOCK_UPDATE_ALL));
    let refused = at(2, 320, 0);
    charge_spreader(&mut world, &[cursor(refused, 5, 1, 0)]);

    world.tick(Vec::new());
    assert_eq!(
        cursors(&world),
        vec![cursor(refused, 5, 0, 1)],
        "no charge was spent and the decay aged one step"
    );
    assert_eq!(
        world.block_state(DIM, refused),
        None,
        "the cell is outside the build height, so nothing could be written"
    );
    assert_eq!(
        world.block_state(DIM, support),
        Some(state("minecraft:stone")),
        "the refused write left the support alone"
    );

    // The blocked-target mirror: a loaded cell with no attachable support fails
    // `attemptSpreadVein` the same way and keeps the charge too.
    let mut world = catalyst_world();
    let floating = at(2, 70, 0);
    charge_spreader(&mut world, &[cursor(floating, 5, 1, 0)]);
    world.tick(Vec::new());
    assert_eq!(cursors(&world), vec![cursor(floating, 5, 0, 1)]);
    assert_eq!(
        world.block_state(DIM, floating),
        Some(state("minecraft:air"))
    );
}

/// `SculkVeinBlock.attemptPlaceSculk` replaces
/// the replaceable support a vein's face reaches, and lifts whatever stands on
/// the shape the new state adds (`Block.pushEntitiesUp`).
///
/// Soul sand is the one `sculk_replaceable` block whose collision shape is not
/// a full cube (14/16 high), so replacing it with sculk adds the eighth of a
/// block an entity standing on it rises by.
mod push_entities_up {
    use super::*;

    fn entity_position(world: &ServerWorld, entity_id: i32) -> Vec3d {
        world
            .stored_entity_for_test(DIM, entity_id)
            .expect("the test entity is stored")
            .position()
    }

    fn spawn_armor_stand(world: &mut ServerWorld, position: Vec3d) -> i32 {
        world
            .spawn_stored_entity_for_test(
                DIM,
                VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                position,
                EntityData::ArmorStand(
                    ArmorStandData::new().expect("the armor stand has attributes"),
                ),
            )
            .0
    }

    /// The world, the replaced support and the cursor cell above it, with the
    /// soul sand in place and the cursor armed to place sculk on it next tick.
    fn world_with_soul_sand_support() -> ServerWorld {
        let mut world = catalyst_world();
        assert!(
            world.set_block(
                DIM,
                at(2, 63, 0),
                state("minecraft:soul_sand"),
                SET_BLOCK_UPDATE_ALL
            ),
            "the flat floor's stone becomes soul sand"
        );
        charge_spreader(&mut world, &[cursor(at(2, 64, 0), 5, 1, 1)]);
        world
    }

    /// One entity in the new shape rises by the collision the placement added;
    /// one whose box misses it does not move.
    #[test]
    fn the_entity_in_the_new_shape_rises_and_the_one_outside_it_does_not() {
        let mut world = world_with_soul_sand_support();
        let standing = spawn_armor_stand(
            &mut world,
            Vec3d {
                x: 2.5,
                y: 63.875,
                z: 0.5,
            },
        );
        // On the stone floor beside the support, a quarter block past the
        // shape's `x` face.
        let beside = spawn_armor_stand(
            &mut world,
            Vec3d {
                x: 3.5,
                y: 64.0,
                z: 0.5,
            },
        );
        for (dimension, chunk) in world.pending_runtime_entity_chunks() {
            world
                .complete_runtime_entity_chunk(&dimension, chunk, None)
                .unwrap();
        }
        world.tick(Vec::new());
        let standing_before = entity_position(&world, standing);
        let beside_before = entity_position(&world, beside);
        assert_eq!(standing_before.y, 63.875, "the entity stands on the sand");

        world.tick_sculk_catalyst_seeded_for_test(&DIM, CATALYST, 0);

        assert_eq!(
            world.block_state(DIM, at(2, 63, 0)),
            Some(state("minecraft:sculk")),
            "`attemptPlaceSculk` replaced the support"
        );
        let standing_after = entity_position(&world, standing);
        assert_eq!(
            standing_after.y - standing_before.y,
            0.125,
            "`1.0 + collide(Y, box.move(0, 1, 0), [offsetShape], -1.0)` (`Block.java:157-158`)"
        );
        assert_eq!(standing_after.x, standing_before.x);
        assert_eq!(standing_after.z, standing_before.z);
        assert_eq!(entity_position(&world, beside), beside_before);
    }

    /// A support whose collision shape already covers the new state's adds
    /// nothing, so `offsetShape.isEmpty()` returns before the entity query
    #[test]
    fn a_placement_that_adds_no_collision_shape_pushes_nothing() {
        let mut world = catalyst_world();
        assert_eq!(
            world.block_state(DIM, at(2, 63, 0)),
            Some(state("minecraft:stone"))
        );
        let standing = spawn_armor_stand(
            &mut world,
            Vec3d {
                x: 2.5,
                y: 64.0,
                z: 0.5,
            },
        );
        charge_spreader(&mut world, &[cursor(at(2, 64, 0), 5, 1, 1)]);
        for (dimension, chunk) in world.pending_runtime_entity_chunks() {
            world
                .complete_runtime_entity_chunk(&dimension, chunk, None)
                .unwrap();
        }
        world.tick(Vec::new());
        let before = entity_position(&world, standing);

        world.tick_sculk_catalyst_seeded_for_test(&DIM, CATALYST, 0);

        assert_eq!(
            world.block_state(DIM, at(2, 63, 0)),
            Some(state("minecraft:sculk")),
            "the full-cube support was still replaced"
        );
        assert_eq!(entity_position(&world, standing), before);
    }

    /// `getEntities(null, bb)` is `NO_SPECTATORS`
    /// over a `ServerLevel`'s entity view, which
    /// includes session players, so the lift reaches a player as
    /// `ServerPlayer.teleportRelative`.
    #[test]
    fn a_player_standing_on_the_support_is_lifted_by_a_relative_teleport() {
        let (sender, receiver, mut world) = catalyst_session_world();
        assert!(world.set_block(
            DIM,
            at(2, 63, 0),
            state("minecraft:soul_sand"),
            SET_BLOCK_UPDATE_ALL
        ));
        stand_at(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 2.5,
                y: 63.875,
                z: 0.5,
            },
        );
        assert_eq!(world.publication().players[0].position.y, 63.875);
        charge_spreader(&mut world, &[cursor(at(2, 64, 0), 5, 1, 0)]);

        let tick = world.tick(receiver.drain());

        assert_eq!(
            world.block_state(DIM, at(2, 63, 0)),
            Some(state("minecraft:sculk"))
        );
        let update = tick
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                    ..
                } => Some(*update),
                _ => None,
            })
            .expect("the lifted player is told its new position");
        assert_eq!(update.position.y, 0.125);
        assert_eq!(
            update.relatives_mask,
            PLAYER_RELATIVE_X
                | PLAYER_RELATIVE_Y
                | PLAYER_RELATIVE_Z
                | PLAYER_RELATIVE_DELTA_X
                | PLAYER_RELATIVE_DELTA_Y
                | PLAYER_RELATIVE_DELTA_Z
                | PLAYER_RELATIVE_Y_ROT
                | PLAYER_RELATIVE_X_ROT
                | bbb_protocol::packets::PLAYER_RELATIVE_ROTATE_DELTA
        );
        assert_eq!(world.publication().players[0].position.y, 64.0);
    }
}

/// The spreader's own `levelEvent` and `playSound` calls reach a session in
/// range: the `SCULK_BLOCK_SPREAD` sound at the cell the vein grew in
/// and the charge burst at the cursor's cell with
/// `(numParticles << 6) + MultifaceBlock.pack(faces)`.
#[test]
fn the_spread_sound_and_charge_particles_reach_a_session() {
    let (_sender, receiver, mut world) = catalyst_session_world();
    let start = at(2, 64, 0);
    charge_spreader(&mut world, &[cursor(start, 5, 1, 0)]);

    // Both effects leave in the tick that raised them, after the block-entity
    // ticker that reached the spreader.
    let tick = world.tick(receiver.drain());
    let landed = cursors(&world);

    // Two calls in the order vanilla makes them: the vein the cursor grew in
    // and the sculk `attemptPlaceSculk` put down
    assert_eq!(
        spread_sounds(&tick),
        vec![(start, 1.0, 1.0), (at(2, 63, 0), 1.0, 1.0)],
        "`playSound(null, pos, SCULK_BLOCK_SPREAD, BLOCKS, 1.0F, 1.0F)`"
    );
    // The burst is the cursor's own cell and charge after the tick:
    // `(int)(Math.log1p(charge) / 2.3F) + 1` particles plus the six-bit face
    // code of `MultifaceBlock.pack` (`:176-177`).
    let packed = landed[0]
        .facings
        .as_deref()
        .unwrap_or_default()
        .iter()
        .fold(0, |code, direction| code | (1 << direction.id()));
    let particles = (f64::ln_1p(f64::from(landed[0].charge)) / f64::from(2.3_f32)) as i32 + 1;
    assert_eq!(
        charge_particles(&tick),
        vec![(landed[0].pos, (particles << 6) + packed)]
    );
}

/// `ChargeCursor.shouldUpdate`'s level-time threshold is read at `originPos`
/// (`SculkSpreader`, whose `:257` call passes the `originPos` the
/// catalyst's own `serverTick` handed `updateCursors`), not at the
/// cursor's cell: a cursor standing outside the catalyst's block-ticking ring
/// still charges. The ticker's gate is that same read
/// (`Level.tickBlockEntities` gates on `shouldTickBlocksAt(ticker.getPos())`,
/// `Level`, over the cell `LevelChunk`/`:776` take off the
/// object), so the body carries no second copy of it.
#[test]
fn a_cursor_outside_the_catalysts_block_ticking_ring_still_charges() {
    let mut world = catalyst_world();
    // Chunk (2, 0) is two columns from the only ticketed one, so its
    // `SimulationChunkTracker` level is 33 and `inBlockTickingRange`
    // answers false for it.
    let far = at(34, 100, 0);
    let ticking = world.block_ticking_chunks(DIM);
    assert!(ticking.contains(&(0, 0)), "the catalyst's column ticks");
    assert!(
        !ticking.contains(&(2, 0)),
        "the cursor's column is outside the ring the ticker runs in"
    );
    charge_spreader(&mut world, &[cursor(far, 5, 1, 1)]);

    // `if (this.updateDelay > 0) this.updateDelay--` (`:258-259`) runs ahead of
    // every level read (`:261`), so the countdown is the body and not the gate:
    // a cursor the level-time threshold had refused would keep its delay.
    world.tick(Vec::new());
    assert_eq!(cursors(&world), vec![cursor(far, 5, 1, 0)]);
}

/// `MultifaceBlock.availableFaces` answers an empty set for a cell that is not
/// a multiface, and vanilla keeps that set apart
/// from the null one `updateCursors` reads at and the `DEFAULT`
/// spreader reads at `SculkBehaviour`. The cursor list carries the
/// difference as a present empty list against an absent field, so the
/// per-tick rebuild from `cursors` has to keep it.
#[test]
fn an_empty_facing_set_is_not_the_same_cursor_as_no_facing_set() {
    let start = at(0, 100, 0);
    let with_faces = SculkCursorFacts {
        facings: Some(Vec::new()),
        ..cursor(start, 5, 1, 0)
    };
    let (_sender, receiver, mut world) = catalyst_session_world();
    assert_eq!(
        world.block_state(DIM, start),
        Some(state("minecraft:air")),
        "the cursor stands in open air, where no face attaches"
    );
    charge_spreader(&mut world, &[with_faces]);
    let tick = world.tick(receiver.drain());
    // The charge burst is raised for a non-null set only, and its data is
    // `(numParticles << 6) + MultifaceBlock.pack(faces)` (`:176-177`): one
    // particle for a charge of five, and no face bits.
    assert_eq!(charge_particles(&tick), vec![(start, 64)]);
    assert_eq!(
        cursors(&world),
        vec![SculkCursorFacts {
            facings: Some(Vec::new()),
            ..cursor(start, 5, 0, 1)
        }],
        "the emptied set is still a set after the rebuild"
    );

    // The same cursor with the field absent raises nothing: the cell is air
    // with no support, so no face is gained and the null set skips `:175`.
    let (_sender, receiver, mut world) = catalyst_session_world();
    charge_spreader(&mut world, &[cursor(start, 5, 1, 0)]);
    let tick = world.tick(receiver.drain());
    assert_eq!(charge_particles(&tick), Vec::new());
    assert_eq!(cursors(&world), vec![cursor(start, 5, 0, 1)]);
}

/// A cursor list that survives a save keeps travelling: the reloaded catalyst
/// holds the same list, and a seeded tick on both worlds leaves the same cursor
/// state and the same blocks.
#[test]
fn a_saved_cursor_list_resumes_after_a_reload() {
    let mut world = catalyst_world();
    let start = at(2, 64, 0);
    charge_spreader(&mut world, &[cursor(start, 5, 1, 1)]);
    world.tick(Vec::new());
    assert_eq!(cursors(&world), vec![cursor(start, 5, 1, 0)]);

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the fixture column is written");
    let parsed = Java26ChunkData::parse(created.document.clone(), restore_chunk_context())
        .expect("the runtime's own document is a valid Java 26.1 chunk");
    let reloaded = restore_candidate(world.game_time(), Some(parsed), Vec::new());
    let mut restored = spawn_pinned_restore(datapack_config(), &reloaded);
    assert_eq!(
        cursors(&restored),
        vec![cursor(start, 5, 1, 0)],
        "`SculkSpreader.save`/`load` round-tripped the cursor"
    );

    // Both worlds now run the same seeded tick, so the reloaded one has to
    // travel the same way the original does.
    let seed = 7;
    world.tick_sculk_catalyst_seeded_for_test(&DIM, CATALYST, seed);
    restored.tick_sculk_catalyst_seeded_for_test(&DIM, CATALYST, seed);

    let travelled = cursors(&world);
    assert_eq!(travelled.len(), 1);
    assert_eq!(travelled[0].charge, 4, "the reloaded cursor spent a charge");
    assert_eq!(travelled[0].update_delay, 1);
    assert_eq!(
        cursors(&restored),
        travelled,
        "the reload resumed the travel"
    );
    assert_eq!(
        restored.block_state(DIM, at(2, 63, 0)),
        Some(state("minecraft:sculk"))
    );
    assert_eq!(
        restored.block_state(DIM, start),
        Some(state("minecraft:air"))
    );
}

/// `CatalystListener.handleGameEvent`
/// hears a session's `ENTITY_DIE` and takes the experience `dropExperience`
/// would have dropped as orbs.
#[test]
fn a_session_dying_beside_a_catalyst_charges_it_instead_of_dropping_orbs() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    assert!(world.set_block(DIM, CATALYST, catalyst_state(), SET_BLOCK_UPDATE_ALL));
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "experience set alice 10 levels",
    );
    chat_command(&sender, &receiver, &mut world, 1, "kill alice");
    world.tick(receiver.drain());

    let orbs = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|&id| {
            world
                .stored_entity_for_test(DIM, id)
                .is_some_and(|entity| matches!(entity.data(), EntityData::ExperienceOrb(_)))
        })
        .count();
    assert_eq!(orbs, 0);
    // `addCursors` takes the reward the orbs would have carried; the
    // catalyst's own tick may already have spent some of it.
    let charge: i32 = cursors(&world).iter().map(|cursor| cursor.charge).sum();
    assert!((1..=70).contains(&charge), "{charge}");
    assert_eq!(
        world.block_state(DIM, CATALYST),
        Some(state_with("minecraft:sculk_catalyst", &[("bloom", "true")]))
    );
}
