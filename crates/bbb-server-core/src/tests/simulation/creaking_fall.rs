//! `LivingEntity.causeFallDamage` on the real
//! tick: it deals its damage through `this.hurt`, so a heart-bound
//! protector answers it with `Creaking.hurtServer`'s refusal
//! while an unbound one takes the damage.

use super::*;

use crate::entity::{EntityData, GeneralLivingData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID;
use uuid::Uuid;

use super::helpers::{
    creaking_fixture_with_teams as fixture_with_teams, creaking_lay_corridor as lay_corridor,
    creaking_move_player as move_player, creaking_place_heart,
    creaking_spawn_protector as spawn_protector, level_sounds, uuid_scanning_players_immediately,
    CREAKING_HEART as HEART, CREAKING_LOOKING_WEST as LOOKING_WEST,
    CREAKING_WATCHER_POS as WATCHER_POSITION,
};

fn place_heart(world: &mut ServerWorld, at: BlockPos) {
    creaking_place_heart(world, at, "awake");
}

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

/// The falling body's own column, clear of the corridor floor at y=64: a
/// four-block drop is `floor(4.0 - 3.0)` = one point of damage
/// (`LivingEntity.calculateFallDamage`, `LivingEntity.java:1813-1824`), which
/// is the whole of a creaking's health (`Creaking.java:129`).
const DROP: Vec3d = Vec3d {
    x: 3.5,
    y: 69.0,
    z: 4.5,
};

/// The corridor plus clear air above the whole of it, so the drop starts in
/// the air whatever the generator left there and cannot be caught by it.
fn lay_drop_corridor(world: &mut ServerWorld) {
    lay_corridor(world);
    let air = block_state_id("minecraft:air");
    for x in -2..=10 {
        for z in 3..=6 {
            for y in 65..=72 {
                // `Level.setBlock` answers false for a write that changes
                // nothing, which a generated cell
                // already at this state is.
                let _ = world.set_block(DIMENSION, BlockPos { x, y, z }, air, SET_BLOCK_UPDATE_ALL);
            }
        }
    }
}

/// The protector that falls, bound to the heart of `HEART` or not.
///
/// The unbound one is the same body without `Creaking.setTransient`
/// `getHomePos()` stays null, which is the
/// `super.hurtServer` arm of.
fn spawn_falling_protector(world: &mut ServerWorld, uuid: Uuid, bound: bool) -> i32 {
    if bound {
        return spawn_protector(world, DROP, uuid, HEART);
    }
    world
        .restore_stored_entity_for_test(
            DIMENSION,
            uuid,
            VANILLA_ENTITY_TYPE_CREAKING_ID,
            DROP,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_CREAKING_ID)
                    .expect("creaking is living"),
            ),
        )
        .expect("the protector is restored with its own uuid")
}

/// What the drop left behind.
struct Drop {
    /// Every sound of every tick, in order.
    sounds: Vec<String>,
    /// The protector's health after the last tick, and whether it is still in
    /// the store: the killing blow removes it in the same tick it lands on.
    health: Option<f32>,
    ground: Option<Vec3d>,
}

/// The watch both cases run: a protector dropped into the shaft with a watcher
/// four blocks east of it staring straight at it, so the body freezes on the
/// second tick and falls without the IDLE stroll pulling it sideways.
fn drop_protector(uuid: Uuid, bound: bool) -> Drop {
    let (sender, receiver, mut world) = fixture_with_teams(Vec::new());
    lay_drop_corridor(&mut world);
    place_heart(&mut world, HEART);
    let entity_id = spawn_falling_protector(&mut world, uuid, bound);

    // Tick one: the watcher's own move packet lands, the brain is built and
    // both sensors scan with a zero delay.
    let first = move_player(
        &sender,
        &receiver,
        &mut world,
        Some(WATCHER_POSITION),
        LOOKING_WEST,
    );
    let mut sounds = level_sounds(&first);
    for _ in 0..40 {
        let tick = world.tick(receiver.drain());
        sounds.extend(level_sounds(&tick));
        if let Some(entity) = world.stored_entity_for_test(DIMENSION, entity_id) {
            if entity.on_ground() {
                return Drop {
                    sounds,
                    health: entity.living().map(|living| living.health()),
                    ground: Some(entity.position()),
                };
            }
        }
    }
    Drop {
        sounds,
        health: world
            .stored_entity_for_test(DIMENSION, entity_id)
            .and_then(|entity| entity.living().map(|living| living.health())),
        ground: None,
    }
}

/// `causeFallDamage` on a bound protector: the sounds of `:1770-1772` play,
/// but the hurt at `:1773` is the refusal of `Creaking.hurtServer`'s
/// `:161-163`, so `setHealth` never runs.
#[test]
fn a_heart_bound_protector_lands_without_fall_damage() {
    let drop = drop_protector(uuid_scanning_players_immediately(), true);
    assert_eq!(
        drop.ground.expect("the protector lands inside the shaft"),
        Vec3d { y: 65.0, ..DROP }
    );
    assert_eq!(
        drop.health,
        Some(1.0),
        "the heart-bound protector keeps the whole of its one health"
    );
    assert!(
        !drop
            .sounds
            .contains(&"minecraft:entity.creaking.hurt".to_string()),
        "nothing hurt the body: {:?}",
        drop.sounds
    );
}

/// The contrast: the same fall on a protector whose `getHomePos()` is null
/// reaches `super.hurtServer` and kills it, one point
/// of damage against one point of health.
#[test]
fn an_unbound_protector_dies_of_the_same_fall() {
    let drop = drop_protector(uuid_scanning_players_immediately(), false);
    assert!(
        drop.health.is_none_or(|health| health <= 0.0),
        "the fall took the last of its health: {:?}",
        drop.health
    );
    assert!(
        drop.sounds
            .contains(&"minecraft:entity.hostile.small_fall".to_string()),
        "the same landing sound as the bound body: {:?}",
        drop.sounds
    );
}
