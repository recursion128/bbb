//! The sculk catalyst's level seam over a real world
//! the `LivingEntity` experience
//! facts its listener body reads, the `skipDropExperience` write, the four
//! effects `bloom` raises, and the advancement award a death beside the
//! catalyst reaches.
//!
//! The deaths here are real: `LivingEntity.die`'s
//! `gameEvent(GameEvent.ENTITY_DIE)` is raised by
//! the kill itself and reaches the listener when the dying entity's own level
//! frame drains.

use std::sync::Arc;

use super::*;

use crate::block_entity::sculk_catalyst::PULSE_TICKS;
use crate::entity::LastHurtByMob;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::tests::parse_advancement;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_SHEEP_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::{LevelParticles, PlayerExperience, SoundEventHolder, SoundSource};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_SCULK_SOUL_ID;
use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
use bbb_server_datapack::startup::DatapackStartup;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::tests::entities::restore_entity_writes;

use super::helpers::catalyst_state;

const CATALYST: BlockPos = BlockPos { x: 0, y: 64, z: 0 };

fn spawn_mob(world: &mut ServerWorld, entity_type_id: i32) -> i32 {
    helpers::spawn_living_at(
        world,
        entity_type_id,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    )
    .0
}

/// `bloom` (`:111-112`) writes `PULSE = true` and schedules the eight-tick
/// reset `SculkCatalystBlock.tick` runs.
#[test]
fn bloom_writes_the_state_and_the_scheduled_tick_clears_it() {
    let (_sender, receiver, mut world) = helpers::joined_world(datapack_config());
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        CATALYST,
        catalyst_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));
    let mut events = Vec::new();
    world.catalyst_level_for_test(
        Java26BuiltinDimension::Overworld,
        CATALYST,
        &mut events,
        |level| {
            assert_eq!(level.block_state(CATALYST), Some(catalyst_state(false)));
            level.set_block_state(CATALYST, catalyst_state(true), SET_BLOCK_UPDATE_ALL);
            level.schedule_block_tick(CATALYST, PULSE_TICKS);
        },
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, CATALYST),
        Some(catalyst_state(true))
    );

    for _ in 0..(PULSE_TICKS - 1) {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, CATALYST),
        Some(catalyst_state(true)),
        "the reset is eight ticks out, not sooner"
    );
    world.tick(receiver.drain());
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, CATALYST),
        Some(catalyst_state(false)),
        "the scheduled tick cleared BLOOM"
    );
}

/// The catalyst block entity's own cursor list
/// (`SculkCatalystBlockEntity.getSpreader`), which `addCursors` writes.
fn catalyst_cursors(world: &mut ServerWorld) -> Vec<NbtSummaryValue> {
    helpers::sculk_catalyst_cursors(world, CATALYST)
}

/// The bloom sound of one tick, if it was broadcast at all.
fn bloom_sound(tick: &TickResult) -> Option<bbb_protocol::packets::SoundEvent> {
    let bloom = vanilla_sound_event_registry_id("minecraft:block.sculk_catalyst.bloom")
        .expect("the bloom sound is in the catalog");
    sounds_sent(tick).into_iter().find(|sound| {
        matches!(
            &sound.sound,
            SoundEventHolder::Reference { registry_id } if *registry_id == bloom
        )
    })
}

/// A world with a survival session beside an unbloomed catalyst, with the join
/// and the placement flushed so a scenario starts from a quiet sink.
fn catalyst_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    catalyst_world_with(None)
}

/// [`catalyst_world`] over a datapack generation whose advancement set is
/// `definitions` instead of the built-in one.
fn catalyst_world_with(
    definitions: Option<&[(&str, Value)]>,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let config = match definitions {
        None => datapack_config(),
        Some(definitions) => {
            let mut config = datapack_config();
            let mut startup: DatapackStartup = (**config
                .datapacks
                .as_ref()
                .expect("the fixture generation is loaded"))
            .clone();
            let mut advancements = definitions
                .iter()
                .map(|(id, body)| AdvancementElement {
                    id: (*id).into(),
                    source_pack: "test".into(),
                    advancement: parse_advancement(body).expect("the fixture advancement parses"),
                })
                .collect::<Vec<_>>();
            advancements.sort_by(|left, right| left.id.cmp(&right.id));
            startup.advancements = AdvancementSet {
                advancements,
                warnings: Vec::new(),
            };
            config.datapacks = Some(Arc::new(startup));
            config
        }
    };
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        CATALYST,
        catalyst_state(false),
        SET_BLOCK_UPDATE_ALL,
    ));
    world.tick(receiver.drain());
    (sender, receiver, world)
}

/// A lethal blow from `causing`, run through the command damage path the way
/// `/damage` does, so the target remembers it.
fn kill_stored_entity(
    world: &mut ServerWorld,
    mob: i32,
    causing: Option<crate::commands::CommandDamageSourceEntity>,
) {
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            mob,
            "minecraft:generic",
            40.0,
            causing,
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);
}

/// The `CommandDamageSourceEntity` a `/damage ... by <player>` resolves to,
/// carrying the session's own entity id so the source's causing slot resolves
/// back to that session.
fn player_damage_source(uuid: Uuid, entity_id: i32) -> crate::commands::CommandDamageSourceEntity {
    crate::commands::CommandDamageSourceEntity::Player {
        user: crate::admin::NameAndId {
            id: uuid,
            name: "alice".into(),
        }
        .into(),
        entity_id,
        position: Vec3d::default(),
        instabuild: false,
        may_build: true,
        spectator: false,
        display_name: NbtSummaryValue::string("alice"),
    }
}

/// The `CommandDamageSourceEntity` a `/damage ... by <stored mob>` resolves to.
fn stored_damage_source(uuid: Uuid, entity_id: i32) -> crate::commands::CommandDamageSourceEntity {
    helpers::stored_damage_source(uuid, entity_id, "minecraft:zombie", true, Vec3d::default())
}

/// Every `ClientboundSetExperiencePacket` one tick sent the session.
fn set_experiences(tick: &TickResult) -> Vec<PlayerExperience> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SetExperience(packet),
            } => Some(*packet),
            _ => None,
        })
        .collect()
}

/// A real death beside the catalyst: `LivingEntity.die` raises
/// `GameEvent.ENTITY_DIE`, the dispatcher delivers
/// it, and the catalyst's listener charges the spreader at the death's own
/// position, marks the mob's experience consumed, and raises the four `bloom`
/// effects.
#[test]
fn a_real_entity_die_charges_the_spreader_and_blooms() {
    for (entity_type, charge) in [
        (VANILLA_ENTITY_TYPE_ZOMBIE_ID, 5),
        (VANILLA_ENTITY_TYPE_BAT_ID, 0),
    ] {
        let (_sender, receiver, mut world) = catalyst_world();
        let zombie = spawn_mob(&mut world, entity_type);

        kill_stored_entity(&mut world, zombie, None);
        let tick = world.tick(receiver.drain());

        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, CATALYST),
            Some(catalyst_state(true))
        );
        let cursors = catalyst_cursors(&mut world);
        if charge == 0 {
            assert!(cursors.is_empty());
        } else {
            assert_eq!(cursors.len(), 1);
            let NbtSummaryValue::Compound(cursor) = &cursors[0] else {
                panic!("a cursor must be a compound");
            };
            assert_eq!(
                cursor
                    .iter()
                    .find(|entry| entry.name == "charge")
                    .map(|entry| &entry.value),
                Some(&NbtSummaryValue::Int(charge)),
            );
            assert_eq!(
                cursor
                    .iter()
                    .find(|entry| entry.name == "pos")
                    .map(|entry| &entry.value),
                Some(&NbtSummaryValue::IntArray(vec![0, 64, 0])),
            );
        }
        assert!(world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, zombie)
            .expect("the zombie is stored")
            .living()
            .expect("a zombie is living")
            .was_experience_consumed());
        let sound = bloom_sound(&tick).expect("the bloom sound was broadcast");
        assert_eq!(sound.source, SoundSource::Blocks);
        assert_eq!(sound.volume, 2.0);
        assert!(
            (0.6..=1.0).contains(&sound.pitch),
            "pitch {} is outside `0.6 + random * 0.4`",
            sound.pitch
        );
        let quiet = world.tick(receiver.drain());
        assert!(
            bloom_sound(&quiet).is_none(),
            "the bloom sound was broadcast once, not replayed"
        );
        let particles: Vec<LevelParticles> = tick
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::LevelParticles(particles),
                    ..
                } => Some(particles.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(particles.len(), 1, "one soul burst");
        assert_eq!(
            particles[0].particle.particle_type_id,
            VANILLA_PARTICLE_TYPE_SCULK_SOUL_ID
        );
        assert_eq!(particles[0].count, 2);
        assert_eq!(
            particles[0].position,
            Vec3d {
                x: 0.5,
                y: 65.15,
                z: 0.5
            }
        );
    }
}

/// `mob.skipDropExperience()` (`:95`) is read by `dropAllDeathLoot`'s
/// `dropExperience`: a player's kill beside the
/// catalyst must leave no XP orb, while the plain-kill coverage in
/// `mob_death_loot` proves the same kill without a catalyst awards one.
#[test]
fn a_player_kill_beside_the_catalyst_leaves_no_experience_orb() {
    let (_sender, receiver, mut world) = catalyst_world();
    let zombie = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let player = world.player_entity_id_for_test(ConnectionId(1));
    kill_stored_entity(
        &mut world,
        zombie,
        Some(player_damage_source(Uuid::from_u128(1), player)),
    );
    world.tick(receiver.drain());

    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, zombie)
        .and_then(crate::entity::Entity::living)
        .is_some_and(crate::entity::LivingEntityState::was_experience_consumed));
    assert!(
        world
            .stored_experience_orbs_for_test(Java26BuiltinDimension::Overworld)
            .is_empty(),
        "the catalyst consumed the reward before the orb was awarded"
    );
}

/// The mirror: a mob whose experience is already consumed
/// (`!mob.wasExperienceConsumed()`, `:87`) charges nothing and raises no
/// bloom.
#[test]
fn a_consumed_mob_die_blooms_nothing() {
    let (_sender, receiver, mut world) = catalyst_world();
    let zombie = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, zombie)
        .expect("the zombie is stored")
        .living_mut()
        .expect("a zombie is living")
        .skip_drop_experience();

    kill_stored_entity(&mut world, zombie, None);
    let tick = world.tick(receiver.drain());
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, CATALYST),
        Some(catalyst_state(false))
    );
    assert!(catalyst_cursors(&mut world).is_empty());
    let flushed = world.tick(receiver.drain());
    assert!(bloom_sound(&tick).is_none(), "no bloom sound");
    assert!(
        bloom_sound(&flushed).is_none(),
        "no bloom sound on the flushing tick either"
    );
}

/// `tryAwardItSpreadsAdvancement`:
/// a player's kill beside the catalyst offers that player the built-in
/// `minecraft:adventure/kill_mob_near_sculk_catalyst` criterion.
#[test]
fn a_player_kill_near_a_catalyst_awards_the_built_in_advancement() {
    let (_sender, receiver, mut world) = catalyst_world();
    let zombie = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let player = world.player_entity_id_for_test(ConnectionId(1));
    kill_stored_entity(
        &mut world,
        zombie,
        Some(player_damage_source(Uuid::from_u128(1), player)),
    );
    world.tick(receiver.drain());
    assert!(world.player_advancement_is_done(
        ConnectionId(1),
        "minecraft:adventure/kill_mob_near_sculk_catalyst"
    ));
}

/// `KilledTrigger.trigger` through the catalyst:
/// the criterion is offered the damage type `getLastDamageSource` remembered,
/// the mob the death event named, and the causing entity that source kept.
#[test]
fn a_catalyst_kill_matches_the_remembered_blow_and_the_mob_it_killed() {
    let (_sender, receiver, mut world) = catalyst_world_with(Some(&[
        (
            "test:melee",
            json!({"criteria":{"killed":{"trigger":"minecraft:kill_mob_near_sculk_catalyst",
                "conditions":{"killing_blow":{"tags":[{"id":"minecraft:is_projectile","expected":false}]}}}}}),
        ),
        (
            "test:projectile",
            json!({"criteria":{"killed":{"trigger":"minecraft:kill_mob_near_sculk_catalyst",
                "conditions":{"killing_blow":{"tags":[{"id":"minecraft:is_projectile","expected":true}]}}}}}),
        ),
        (
            "test:sheep",
            json!({"criteria":{"killed":{"trigger":"minecraft:kill_mob_near_sculk_catalyst",
                "conditions":{"entity":[{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"type_specific":{"type":"minecraft:sheep","sheared":false}}}]}}}}),
        ),
        (
            "test:source_sheep",
            json!({"criteria":{"killed":{"trigger":"minecraft:kill_mob_near_sculk_catalyst",
                "conditions":{"killing_blow":{"source_entity":{"type_specific":{"type":"minecraft:sheep"}}}}}}}),
        ),
    ]));
    let sheep = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SHEEP_ID);
    let player = world.player_entity_id_for_test(ConnectionId(1));
    kill_stored_entity(
        &mut world,
        sheep,
        Some(player_damage_source(Uuid::from_u128(1), player)),
    );
    world.tick(receiver.drain());
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:melee"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:projectile"));
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:sheep"));
    // The remembered source's causing entity is the session that dealt the
    // blow, so a sheep named there is answered false rather than refused.
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:source_sheep"));
}

/// The `EntityPredicate` terms the killed mob's own view answers
/// its entity type, its
/// distance from the session the criterion belongs to, and the
/// `LocationPredicate` its own coordinates are tested against.
///
/// The origin `distance` measures from is the triggering session's position
/// and the mob's is the catalyst's own column
/// at `(0.5, 64.0, 0.5)`, so the two bounds below bracket one real measurement.
#[test]
fn a_catalyst_kill_answers_the_mobs_type_distance_and_coordinates() {
    let (_sender, receiver, mut world) = catalyst_world_with(Some(&[
        (
            "test:sheep_type",
            json!({"criteria":{"killed":{"trigger":"minecraft:kill_mob_near_sculk_catalyst",
                "conditions":{"entity":[{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"type":"minecraft:sheep"}}]}}}}),
        ),
        (
            "test:zombie_type",
            json!({"criteria":{"killed":{"trigger":"minecraft:kill_mob_near_sculk_catalyst",
                "conditions":{"entity":[{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"type":"minecraft:zombie"}}]}}}}),
        ),
        (
            "test:in_the_catalysts_column",
            json!({"criteria":{"killed":{"trigger":"minecraft:kill_mob_near_sculk_catalyst",
                "conditions":{"entity":[{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"location":{"position":{"x":{"min":0.0,"max":1.0},"y":64.0,"z":{"min":0.0,"max":1.0}}}}}]}}}}),
        ),
        (
            "test:one_block_up",
            json!({"criteria":{"killed":{"trigger":"minecraft:kill_mob_near_sculk_catalyst",
                "conditions":{"entity":[{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"location":{"position":{"y":65.0}}}}]}}}}),
        ),
        (
            "test:within_reach",
            json!({"criteria":{"killed":{"trigger":"minecraft:kill_mob_near_sculk_catalyst",
                "conditions":{"entity":[{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"distance":{"horizontal":{"max":200.0}}}}]}}}}),
        ),
        (
            "test:out_of_reach",
            json!({"criteria":{"killed":{"trigger":"minecraft:kill_mob_near_sculk_catalyst",
                "conditions":{"entity":[{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"distance":{"horizontal":{"min":200.0}}}}]}}}}),
        ),
    ]));
    let spot = world.publication().players[0].position;
    let horizontal = ((spot.x - 0.5).powi(2) + (spot.z - 0.5).powi(2)).sqrt();
    assert!(
        horizontal < 200.0,
        "the fixture session stands inside the bracket: {horizontal}"
    );

    let sheep = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SHEEP_ID);
    let player = world.player_entity_id_for_test(ConnectionId(1));
    kill_stored_entity(
        &mut world,
        sheep,
        Some(player_damage_source(Uuid::from_u128(1), player)),
    );
    world.tick(receiver.drain());

    assert!(world.player_advancement_is_done(ConnectionId(1), "test:sheep_type"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:zombie_type"));
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:in_the_catalysts_column"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:one_block_up"));
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:within_reach"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:out_of_reach"));
}

/// `mob.getLastHurtByMob() instanceof ServerPlayer` (`:118`): a kill with no
/// player behind it awards nothing, and the charge still lands, so the refusal
/// is the award's and not the listener's.
#[test]
fn a_kill_with_no_player_behind_it_awards_nothing() {
    let (_sender, receiver, mut world) = catalyst_world();
    let attacker = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let victim = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    kill_stored_entity(
        &mut world,
        victim,
        Some(stored_damage_source(Uuid::from_u128(9), attacker)),
    );
    world.tick(receiver.drain());
    assert!(!world.player_advancement_is_done(
        ConnectionId(1),
        "minecraft:adventure/kill_mob_near_sculk_catalyst"
    ));
    assert!(
        !catalyst_cursors(&mut world).is_empty(),
        "the listener charged the spreader"
    );
}

/// The reference the listener reads (`:118`) is saved state
/// the player who hurt the mob
/// before the save is still its `getLastHurtByMob` after the reload, so a later
/// kill with no player behind it still credits that player.
#[test]
fn a_reloaded_mob_credits_the_killer_it_remembered_before_the_save() {
    let (sender, receiver, mut world) = catalyst_world();
    let victim = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let victim_uuid = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, victim)
        .expect("the victim is stored")
        .uuid();
    let alice = Uuid::from_u128(1);
    let player = world.player_entity_id_for_test(ConnectionId(1));
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            victim,
            "minecraft:generic",
            1.0,
            Some(player_damage_source(alice, player)),
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);
    {
        let living = world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, victim)
            .and_then(crate::entity::Entity::living_mut)
            .expect("a zombie is living");
        assert_eq!(living.last_hurt_by_mob(), Some(LastHurtByMob::Uuid(alice)));
        // Only the mob reference has to survive: the listener reads
        // `getLastHurtByMob`, never the player reference beside it.
        living.last_hurt_by_player = None;
        living.last_hurt_by_player_memory_time = 0;
    }

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the fixture column is written entire");
    let parsed = Java26ChunkData::parse(created.document.clone(), restore_chunk_context())
        .expect("the runtime's own document is a valid Java 26.1 chunk");
    let mut candidate = restore_candidate(world.game_time(), Some(parsed), Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(datapack_config(), &candidate);
    join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut restored,
        1,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let restored_victim = restored
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, victim_uuid)
        .expect("the victim reloads");
    assert_eq!(
        restored
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, restored_victim)
            .and_then(crate::entity::Entity::living)
            .and_then(crate::entity::LivingEntityState::last_hurt_by_mob),
        Some(LastHurtByMob::Uuid(alice)),
        "the saved reference survived the reload"
    );

    // The kill carries no player: the credit can only come from the reloaded
    // reference.
    kill_stored_entity(&mut restored, restored_victim, None);
    restored.tick(receiver.drain());
    assert!(restored.player_advancement_is_done(
        ConnectionId(1),
        "minecraft:adventure/kill_mob_near_sculk_catalyst"
    ));
}

/// The criterion's own advancement path: the matching criterion completes the
/// advancement and pays its reward once, and a second kill near the catalyst
/// pays nothing again.
#[test]
fn the_awarded_criterion_completes_the_advancement_and_pays_once() {
    let (_sender, receiver, mut world) = catalyst_world_with(Some(&[(
        "test:spread",
        json!({
            "criteria": {
                "killed": {
                    "trigger": "minecraft:kill_mob_near_sculk_catalyst",
                    "conditions": {},
                },
            },
            "rewards": {"experience": 10},
        }),
    )]));
    let player = Uuid::from_u128(1);
    let entity_id = world.player_entity_id_for_test(ConnectionId(1));
    let mut totals = Vec::new();
    for _ in 0..2 {
        let zombie = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        kill_stored_entity(
            &mut world,
            zombie,
            Some(player_damage_source(player, entity_id)),
        );
        // The award lands in the game-event flush and the experience packet
        // follows it in the same tick's session arm.
        let tick = world.tick(receiver.drain());
        assert!(world.player_advancement_is_done(ConnectionId(1), "test:spread"));
        totals.push(
            set_experiences(&tick)
                .into_iter()
                .map(|packet| packet.total)
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(
        totals,
        vec![vec![10], Vec::new()],
        "the reward is paid on the first award only"
    );
}
