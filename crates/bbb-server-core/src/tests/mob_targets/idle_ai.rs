//! Idle movement and gaze of goal-driven mobs, and the target rules their
//! brains keep in Vanilla.

use super::*;
use crate::commands::CommandDamageSourceEntity;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_BREEZE_ID,
    VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_SHULKER_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};

const ALICE: ConnectionId = ConnectionId(1);

struct Arena {
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    world: ServerWorld,
    operation: u64,
}

impl Arena {
    /// A stone floor at noon with Alice in `game_type` at `alice`.
    fn new(game_type: GameType, alice: Vec3d) -> Self {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = game_type;
        config.difficulty = Difficulty::Normal;
        config.keep_alive_interval_ticks = 10_000;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        install_floor(&mut world);
        world.set_player_position_for_test(ALICE, alice);
        let mut arena = Self {
            sender,
            receiver,
            world,
            operation: 10,
        };
        for command in [
            "gamerule spawn_mobs false",
            "weather clear",
            "time set noon",
            "gamerule advance_time false",
        ] {
            arena.command(command);
        }
        arena
    }

    fn command(&mut self, command: &str) {
        self.operation += 1;
        let tick = console_command(
            &self.sender,
            &self.receiver,
            &mut self.world,
            self.operation,
            command,
        );
        assert!(console_command_answer(&tick).0.is_some(), "{command}");
    }

    fn spawn(&mut self, entity_type_id: i32, position: Vec3d) -> (i32, Uuid) {
        spawn(&mut self.world, entity_type_id, position)
    }

    /// Merges `entries` into the mob's saved record.
    fn merge(&mut self, id: i32, entries: Vec<NbtSummaryEntry>) {
        let Some(NbtSummaryValue::Compound(mut record)) = self
            .world
            .stored_entity_command_data_for_test(DIMENSION, id)
        else {
            panic!("a stored mob has a record");
        };
        record.retain(|entry| !entries.iter().any(|new| new.name == entry.name));
        record.extend(entries);
        self.world
            .load_stored_entity_record_for_test(DIMENSION, id, &record);
    }

    fn tick(&mut self) -> TickResult {
        self.world.tick(self.receiver.drain())
    }

    fn entity(&self, id: i32) -> &crate::entity::Entity {
        self.world.stored_entity_for_test(DIMENSION, id).unwrap()
    }

    fn position(&self, id: i32) -> Vec3d {
        self.entity(id).position()
    }

    fn alice(&self) -> CommandDamageSourceEntity {
        let position = self.world.publication().players[0].position;
        CommandDamageSourceEntity::Player {
            user: crate::admin::NameAndId {
                id: Uuid::from_u128(1),
                name: "alice".into(),
            }
            .into(),
            entity_id: self.world.player_entity_id_for_test(ALICE),
            position,
            instabuild: false,
            may_build: true,
            spectator: false,
            display_name: NbtSummaryValue::string("alice"),
        }
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn horizontal(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

fn at(x: f64, y: f64, z: f64) -> Vec3d {
    Vec3d { x, y, z }
}

fn speed(arena: &Arena, id: i32) -> f32 {
    arena.entity(id).living().unwrap().speed()
}

fn sound(tick: &TickResult, id: &str) -> bool {
    let expected = bbb_protocol::packets::SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(id).unwrap(),
    };
    tick.events.iter().any(|event| {
        matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::SoundEntity(sound), .. }
            if sound.sound == expected)
    })
}

/// `BeeWanderGoal` (`Bee.java:1251-1294`): a bee with nothing to forage
/// flies about.
#[test]
fn a_bee_without_flowers_or_a_hive_wanders() {
    let mut arena = Arena::new(GameType::Creative, at(1.5, 65.0, 1.5));
    let start = at(8.5, 67.0, 8.5);
    let (bee, _) = arena.spawn(VANILLA_ENTITY_TYPE_BEE_ID, start);
    let mut farthest = 0.0_f64;
    for _ in 0..400 {
        arena.tick();
        farthest = farthest.max(horizontal(arena.position(bee), start));
    }
    assert!(farthest > 3.0, "the bee wandered {farthest} blocks");
}

/// `BeeGoToKnownFlowerGoal` (`Bee.java:892-941`): 600 ticks without nectar
/// send a bee back to the flower it remembers, far beyond the five blocks it
/// searches around itself.
#[test]
fn a_bee_long_without_nectar_flies_back_to_its_remembered_flower() {
    let mut arena = Arena::new(GameType::Creative, at(14.5, 65.0, 1.5));
    let flower = BlockPos {
        x: 13,
        y: 65,
        z: 13,
    };
    assert!(arena.world.set_block(
        DIMENSION,
        flower,
        block_state_id("minecraft:dandelion"),
        SET_BLOCK_UPDATE_ALL
    ));
    let (bee, _) = arena.spawn(VANILLA_ENTITY_TYPE_BEE_ID, at(1.5, 66.0, 1.5));
    arena.merge(
        bee,
        vec![
            entry(
                "flower_pos",
                NbtSummaryValue::IntArray(vec![flower.x, flower.y, flower.z]),
            ),
            entry("TicksSincePollination", NbtSummaryValue::Int(700)),
        ],
    );
    let target = at(13.5, 65.5, 13.5);
    let mut nearest = f64::MAX;
    for _ in 0..600 {
        arena.tick();
        nearest = nearest.min(horizontal(arena.position(bee), target));
        if nearest < 2.0 {
            break;
        }
    }
    assert!(nearest < 2.0, "the bee came within {nearest} of its flower");
}

/// `BeePollinateGoal` (`Bee.java:1078-1248`): the bee flies at 1.2 towards a
/// flower it finds and, once there, drifts between hover spots up to a third
/// of a block from its centre.
#[test]
fn a_pollinating_bee_drifts_around_the_flower() {
    let mut arena = Arena::new(GameType::Creative, at(14.5, 65.0, 1.5));
    let flower = BlockPos { x: 8, y: 65, z: 8 };
    assert!(arena.world.set_block(
        DIMENSION,
        flower,
        block_state_id("minecraft:dandelion"),
        SET_BLOCK_UPDATE_ALL
    ));
    let (bee, _) = arena.spawn(VANILLA_ENTITY_TYPE_BEE_ID, at(5.5, 66.0, 8.5));
    let centre = at(8.5, 65.6, 8.5);
    let (mut min_x, mut max_x, mut min_z, mut max_z) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    let mut settled = 0;
    for _ in 0..500 {
        arena.tick();
        let position = arena.position(bee);
        settled = if horizontal(position, centre) < 0.5 {
            settled + 1
        } else {
            0
        };
        // Only the hovering after the approach has settled.
        if settled > 40 {
            min_x = min_x.min(position.x);
            max_x = max_x.max(position.x);
            min_z = min_z.min(position.z);
            max_z = max_z.max(position.z);
        }
    }
    assert!(
        max_x - min_x > 0.1 || max_z - min_z > 0.1,
        "x {min_x}..{max_x}, z {min_z}..{max_z}"
    );
    assert!(max_x - min_x < 0.8 && max_z - min_z < 0.8);
}

/// `SnifferAi.java:59`: a sniffer hit by a player panics.
#[test]
fn a_sniffer_hit_by_a_player_panics() {
    let mut arena = Arena::new(GameType::Survival, at(1.5, 65.0, 1.5));
    let (sniffer, _) = arena.spawn(VANILLA_ENTITY_TYPE_SNIFFER_ID, at(8.5, 65.0, 8.5));
    arena.tick();
    let alice = arena.alice();
    let outcome = arena
        .world
        .hurt_stored_entity_by_command_for_test(
            DIMENSION,
            sniffer,
            "minecraft:player_attack",
            1.0,
            Some(alice),
        )
        .unwrap();
    assert!(outcome.applied && outcome.succeeded);
    let mut fastest = 0.0_f32;
    for _ in 0..40 {
        arena.tick();
        fastest = fastest.max(speed(&arena, sniffer));
    }
    // Walking is `1.0 * MOVEMENT_SPEED` 0.1; panic runs at 2.0.
    assert!(fastest > 0.15, "the sniffer ran at {fastest}");
}

/// `Shulker.java:96` looks at players with `onlyHorizontal`: a shulker on a
/// wall does not turn its head towards a player straight above it.
#[test]
fn a_wall_shulker_ignores_the_height_of_a_player_above_it() {
    let mut arena = Arena::new(GameType::Creative, at(8.5, 69.0, 8.5));
    assert!(arena.world.set_block(
        DIMENSION,
        BlockPos { x: 9, y: 65, z: 8 },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL
    ));
    let (shulker, _) = arena.spawn(VANILLA_ENTITY_TYPE_SHULKER_ID, at(8.5, 65.0, 8.5));
    // `Direction.EAST` (3D data value 5): attached to the block east of it.
    arena.merge(shulker, vec![entry("AttachFace", NbtSummaryValue::Byte(5))]);
    let mut widest = 0.0_f32;
    for _ in 0..600 {
        arena.tick();
        let entity = arena.entity(shulker);
        assert_eq!(entity.x_rot(), 0.0);
        widest = widest.max(crate::entity::living::wrap_degrees(entity.y_head_rot()).abs());
    }
    // Looking at the player's eyes would turn the head a half turn; random
    // looks along the wall reach a quarter.
    assert!(widest < 135.0, "the head turned to {widest}");
}

/// `BreezeAi.SlideToTargetSink` (`BreezeAi.java:79-97`): an idle breeze
/// slides with its sound when it walks.
#[test]
fn an_idle_breeze_slides_when_it_wanders() {
    let mut arena = Arena::new(GameType::Creative, at(1.5, 65.0, 1.5));
    let (breeze, _) = arena.spawn(VANILLA_ENTITY_TYPE_BREEZE_ID, at(8.5, 65.0, 8.5));
    let (mut slid, mut heard) = (false, false);
    for _ in 0..400 {
        let tick = arena.tick();
        heard |= sound(&tick, "minecraft:entity.breeze.slide");
        slid |= arena.entity(breeze).pose_id()
            == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLIDING_ID;
        if slid && heard {
            break;
        }
    }
    assert!(slid && heard, "slid {slid}, heard {heard}");
}

/// `BreezeAi.java:44-75`: retaliation is an idle-activity behaviour, so a
/// fighting breeze keeps its target when something else hits it.
#[test]
fn a_fighting_breeze_keeps_its_target_when_another_mob_hits_it() {
    let mut arena = Arena::new(GameType::Survival, at(8.5, 65.0, 8.5));
    let (breeze, _) = arena.spawn(VANILLA_ENTITY_TYPE_BREEZE_ID, at(8.5, 65.0, 13.5));
    let zombie_at = at(2.5, 65.0, 2.5);
    let (zombie, zombie_uuid) = arena.spawn(VANILLA_ENTITY_TYPE_ZOMBIE_ID, zombie_at);
    arena.merge(zombie, vec![entry("NoAI", NbtSummaryValue::Byte(1))]);
    let alice_uuid = Uuid::from_u128(1);
    tick_until(&mut arena.world, 200, |world| {
        target_of(world, breeze) == Some(alice_uuid)
    });
    let outcome = arena
        .world
        .hurt_stored_entity_by_command_for_test(
            DIMENSION,
            breeze,
            "minecraft:mob_attack",
            1.0,
            Some(CommandDamageSourceEntity::Stored {
                entity_id: zombie,
                uuid: Some(zombie_uuid),
                position: Some(zombie_at),
                entity_resource_id: "minecraft:zombie".to_string(),
                is_mob: true,
                is_living_non_player: true,
                display_name: NbtSummaryValue::string("minecraft:zombie"),
            }),
        )
        .unwrap();
    assert!(outcome.applied && outcome.succeeded);
    for _ in 0..40 {
        arena.tick();
        assert_eq!(target_of(&arena.world, breeze), Some(alice_uuid));
    }
}

/// `PiglinAi.java:173-175`: a piglin with a golden spear raises it and
/// charges like any spear user.
#[test]
fn a_piglin_with_a_golden_spear_raises_it_at_the_player() {
    let mut arena = Arena::new(GameType::Survival, at(8.5, 65.0, 2.5));
    let (piglin, uuid) = arena.spawn(VANILLA_ENTITY_TYPE_PIGLIN_ID, at(8.5, 65.0, 9.5));
    arena.merge(
        piglin,
        vec![entry("IsImmuneToZombification", NbtSummaryValue::Byte(1))],
    );
    arena.command(&format!(
        "item replace entity {uuid} weapon.mainhand with minecraft:golden_spear"
    ));
    let mut raised = false;
    for _ in 0..300 {
        arena.tick();
        raised |= arena.entity(piglin).living().unwrap().living_entity_flags() & 1 != 0;
        if raised {
            break;
        }
    }
    assert!(raised, "the piglin raised its spear");
}

/// `PiglinAi.java:171`: `BackUpIfTooClose.create(5, 0.75F)` walks a crossbow
/// piglin back from a target it sees within five blocks.
#[test]
fn a_crossbow_piglin_backs_away_from_a_close_player() {
    let mut arena = Arena::new(GameType::Survival, at(8.5, 65.0, 5.5));
    let start = at(8.5, 65.0, 8.5);
    let (piglin, uuid) = arena.spawn(VANILLA_ENTITY_TYPE_PIGLIN_ID, start);
    arena.merge(
        piglin,
        vec![entry("IsImmuneToZombification", NbtSummaryValue::Byte(1))],
    );
    arena.command(&format!(
        "item replace entity {uuid} weapon.mainhand with minecraft:crossbow"
    ));
    let alice = at(8.5, 65.0, 5.5);
    let mut farthest = 0.0_f64;
    for _ in 0..60 {
        arena.tick();
        farthest = farthest.max(horizontal(arena.position(piglin), alice));
    }
    assert!(farthest > 4.5, "the piglin backed off to {farthest}");
}

/// `HoglinAi.java:94,161-163`: a breeding hoglin neither keeps nor takes a
/// target, while one not in love takes the player.
#[test]
fn breeding_hoglins_take_no_target() {
    let mut arena = Arena::new(GameType::Survival, at(8.5, 65.0, 2.5));
    let (lone, _) = arena.spawn(VANILLA_ENTITY_TYPE_HOGLIN_ID, at(2.5, 65.0, 8.5));
    let lovers = [at(10.5, 65.0, 9.5), at(12.5, 65.0, 9.5)].map(|position| {
        let (id, _) = arena.spawn(VANILLA_ENTITY_TYPE_HOGLIN_ID, position);
        arena.merge(id, vec![entry("InLove", NbtSummaryValue::Int(600))]);
        id
    });
    let alice_uuid = Uuid::from_u128(1);
    let mut lone_targeted = false;
    for tick in 0..50 {
        arena.tick();
        lone_targeted |= target_of(&arena.world, lone) == Some(alice_uuid);
        // The pair starts breeding on its first selector ticks.
        if tick >= 5 {
            for lover in lovers {
                assert_eq!(target_of(&arena.world, lover), None);
            }
        }
    }
    assert!(lone_targeted, "a hoglin out of love takes the player");
}

/// `CopperGolemAi.java:85-92`: with nothing to carry, a copper golem strolls
/// within two blocks of where it stands.
#[test]
fn an_idle_copper_golem_strolls_close_by() {
    let mut arena = Arena::new(GameType::Creative, at(1.5, 65.0, 1.5));
    let start = at(8.5, 65.0, 8.5);
    let (golem, _) = arena.spawn(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, start);
    // Each walk starts where the last one stopped, so measure single legs.
    let (mut previous, mut rest) = (start, start);
    let (mut travelled, mut longest_leg) = (0.0_f64, 0.0_f64);
    for _ in 0..600 {
        arena.tick();
        let position = arena.position(golem);
        let step = horizontal(position, previous);
        travelled += step;
        // A walking golem covers 0.088 blocks a tick and only slows where a
        // walk ends, even when the next one starts straight away.
        if step < 0.05 {
            rest = position;
        }
        longest_leg = longest_leg.max(horizontal(position, rest));
        previous = position;
    }
    assert!(travelled > 1.0, "the golem walked {travelled} blocks");
    assert!(longest_leg < 3.5, "one walk went {longest_leg} blocks");
}

/// `AllayAi.java:84-87`: an idle allay looks at what is near and flies up
/// to it.
#[test]
fn an_idle_allay_flies_up_to_a_nearby_player() {
    let alice = at(8.5, 65.0, 2.5);
    let mut arena = Arena::new(GameType::Creative, alice);
    let (allay, _) = arena.spawn(VANILLA_ENTITY_TYPE_ALLAY_ID, at(8.5, 65.0, 8.0));
    let mut nearest = f64::MAX;
    for _ in 0..400 {
        arena.tick();
        nearest = nearest.min(horizontal(arena.position(allay), alice));
        if nearest < 3.5 {
            break;
        }
    }
    assert!(nearest < 3.5, "the allay came within {nearest}");
}
