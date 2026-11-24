use super::*;
use crate::entity::ActiveMobEffect;
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::{ServerWorldConfig, TickResult};
use bbb_protocol::game_types::GameType;
use bbb_server_types::food::FoodData;
use bbb_server_types::interaction::WorldBorder;

const PLAYER: ConnectionId = ConnectionId(1);

pub(super) fn world(game_type: GameType) -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player = world.levels.player_mut(PLAYER).unwrap();
    {
        let mut position = player.position();
        position.y = 80.0;
        player.set_position(position);
    }
    {
        let mut position = player.position();
        position.x = player.position().x.floor() + 0.5;
        player.set_position(position);
    }
    {
        let mut position = player.position();
        position.z = player.position().z.floor() + 0.5;
        player.set_position(position);
    }
    player.food = FoodData::from_saved(10, 0, 5.0, 0.0);
    world
}

pub(super) fn block(world: &mut ServerWorld, position: BlockPos, resource_id: &str) {
    let dimension = world.levels[PLAYER].dimension;
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(resource_id).unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    assert!(world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(position.x, position.y, position.z, state)
        .is_some());
}

fn eye_block(world: &ServerWorld) -> BlockPos {
    let eye = world.levels[PLAYER].eye_position().unwrap();
    BlockPos {
        x: eye.x.floor() as i32,
        y: eye.y.floor() as i32,
        z: eye.z.floor() as i32,
    }
}

fn pool(world: &mut ServerWorld) {
    let eye = eye_block(world);
    for x in eye.x - 2..=eye.x + 2 {
        for z in eye.z - 2..=eye.z + 2 {
            for y in 80..=82 {
                block(world, BlockPos { x, y, z }, "minecraft:water");
            }
        }
    }
}

fn effect(world: &mut ServerWorld, resource_id: &'static str, duration: i32) {
    let effect = ActiveMobEffect::new(resource_id, duration, 0).unwrap();
    assert_eq!(
        world.add_player_mob_effect(PLAYER, effect, &mut Vec::new()),
        Some(true)
    );
}

fn damage_id(resource_id: &str) -> i32 {
    bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == resource_id)
        .unwrap()
        .registry_id
}

fn damages(tick: &TickResult) -> Vec<i32> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::DamageEvent(data),
            } if *connection_id == PLAYER => Some(data.source_type_id),
            _ => None,
        })
        .collect()
}

#[test]
fn player_environment_fire_hurts_at_twenty_then_only_counts_down_at_nineteen() {
    let mut world = world(GameType::Survival);
    world.levels[PLAYER].remaining_fire_ticks = 20;
    let tick = world.tick(Vec::new());
    assert_eq!(damages(&tick), [damage_id("minecraft:on_fire")]);
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.damage.health, 19.0);
    assert_eq!(player.remaining_fire_ticks, 19);
    assert!(player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE));
    let tick = world.tick(Vec::new());
    assert!(damages(&tick).is_empty());
    assert_eq!(world.levels[PLAYER].damage.health, 19.0);
    assert_eq!(world.levels[PLAYER].remaining_fire_ticks, 18);
}

#[test]
fn player_environment_fire_resistance_protects_before_its_last_tick_expires() {
    let mut world = world(GameType::Survival);
    effect(&mut world, "minecraft:fire_resistance", 1);
    world.levels[PLAYER].remaining_fire_ticks = 20;
    let tick = world.tick(Vec::new());
    assert!(damages(&tick).is_empty());
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.damage.health, 20.0);
    assert_eq!(player.remaining_fire_ticks, 19);
    assert!(player.active_effect("minecraft:fire_resistance").is_none());
    world.levels[PLAYER].remaining_fire_ticks = 20;
    let tick = world.tick(Vec::new());
    assert_eq!(damages(&tick), [damage_id("minecraft:on_fire")]);
    assert_eq!(world.levels[PLAYER].damage.health, 19.0);
}

#[test]
fn player_environment_fatal_fire_clears_before_decrement_and_advances_death_once() {
    let mut world = world(GameType::Survival);
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.damage.health = 1.0;
    player.remaining_fire_ticks = 20;
    player.set_ticks_frozen(140);
    let tick = world.tick(Vec::new());
    assert_eq!(damages(&tick), [damage_id("minecraft:on_fire")]);
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.damage.health, 0.0);
    assert_eq!(player.damage.death_time, 1);
    assert_eq!(player.remaining_fire_ticks, -20);
    assert_eq!(player.ticks_frozen, 0);
    assert!(!player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE));
}

#[test]
fn player_environment_drowning_resets_air_and_sends_bubbles_before_damage() {
    let mut world = world(GameType::Survival);
    pool(&mut world);
    world.levels[PLAYER].set_air_supply(-19);
    let tick = world.tick(Vec::new());
    let player = world.levels.player_at(PLAYER);
    assert!(player.fluid_interaction.is_eye_in_fluid(FluidTag::Water));
    assert_eq!(player.air_supply, 0);
    assert_eq!(player.damage.health, 18.0);
    assert_eq!(damages(&tick), [damage_id("minecraft:drown")]);
    let mut sequence: Vec<_> = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet,
            } if *connection_id == PLAYER => match packet {
                ClientboundPlayPacket::EntityEvent(event)
                    if event.entity_id == player.entity_id() && event.event_id == 67 =>
                {
                    Some("bubbles")
                }
                ClientboundPlayPacket::DamageEvent(event)
                    if event.entity_id == player.entity_id() =>
                {
                    Some("damage")
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    sequence.sort();
    assert_eq!(sequence, ["bubbles", "damage"]);
}

#[test]
fn player_environment_oxygen_bonus_sometimes_keeps_air() {
    // `LivingEntity.decreaseAirSupply`: without a bonus every submerged tick
    // costs air; a bonus of 1 keeps it with chance 1/2.
    for (bonus, expected) in [(0.0, [299].as_slice()), (1.0, [299, 300].as_slice())] {
        let mut world = world(GameType::Survival);
        pool(&mut world);
        world.levels[PLAYER]
            .attributes
            .get_instance(VANILLA_ATTRIBUTE_OXYGEN_BONUS_ID)
            .unwrap()
            .set_base_value(bonus);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..20 {
            world.levels[PLAYER].set_air_supply(300);
            let tick = world.tick(Vec::new());
            assert!(damages(&tick).is_empty());
            seen.insert(world.levels[PLAYER].air_supply);
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            expected,
            "bonus={bonus}"
        );
    }
}

#[test]
fn player_environment_nautilus_preserves_air_but_water_breathing_allows_refill() {
    for (nautilus, water_breathing, expected) in [
        (false, false, 279),
        (true, false, 280),
        (false, true, 284),
        (true, true, 284),
    ] {
        let mut world = world(GameType::Survival);
        pool(&mut world);
        if nautilus {
            effect(&mut world, "minecraft:breath_of_the_nautilus", 100);
        }
        if water_breathing {
            effect(&mut world, "minecraft:water_breathing", 100);
        }
        world.levels[PLAYER].set_air_supply(280);
        let tick = world.tick(Vec::new());
        assert!(damages(&tick).is_empty());
        assert_eq!(
            world.levels[PLAYER].air_supply, expected,
            "nautilus={nautilus}, water_breathing={water_breathing}"
        );
    }
}

#[test]
fn player_environment_bubble_column_refills_air_despite_eye_water_and_nautilus() {
    let mut world = world(GameType::Survival);
    pool(&mut world);
    let eye = eye_block(&world);
    block(&mut world, eye, "minecraft:bubble_column");
    effect(&mut world, "minecraft:breath_of_the_nautilus", 100);
    world.levels[PLAYER].set_air_supply(298);
    let tick = world.tick(Vec::new());
    assert!(damages(&tick).is_empty());
    let player = world.levels.player_at(PLAYER);
    assert!(player.fluid_interaction.is_eye_in_fluid(FluidTag::Water));
    assert_eq!(player.air_supply, 300);
}

#[test]
fn player_environment_void_threshold_is_strict_and_damage_bypasses_creative_immunity() {
    for game_type in [GameType::Survival, GameType::Creative] {
        let mut world = world(game_type);
        let dimension = world.levels[PLAYER].dimension;
        let threshold = f64::from(world.level_by_id(&dimension).chunks.min_y() - 64);
        {
            let mut position = world.levels[PLAYER].position();
            position.y = threshold;
            world.levels[PLAYER].set_position(position);
        }
        let tick = world.tick(Vec::new());
        assert!(damages(&tick).is_empty(), "{game_type:?}");
        assert_eq!(world.levels[PLAYER].damage.health, 20.0);
        {
            let mut position = world.levels[PLAYER].position();
            position.y = threshold - 0.01;
            world.levels[PLAYER].set_position(position);
        }
        let tick = world.tick(Vec::new());
        assert_eq!(
            damages(&tick),
            [damage_id("minecraft:out_of_world")],
            "{game_type:?}"
        );
        assert_eq!(world.levels[PLAYER].damage.health, 16.0);
    }
}

#[test]
fn player_environment_suffocation_takes_precedence_over_larger_border_damage() {
    for in_wall in [true, false] {
        let mut world = world(GameType::Survival);
        let player = world.levels.player_at(PLAYER);
        let position = player.position();
        let dimension = player.dimension;
        world.level_mut_by_id(&dimension).world_border = WorldBorder::from_settings(
            position.x - 3.0,
            position.z,
            2.0,
            0.0,
            0,
            0,
            2.0,
            0,
            2.0,
            0,
        );
        if in_wall {
            let eye = eye_block(&world);
            block(&mut world, eye, "minecraft:stone");
        }
        let tick = world.tick(Vec::new());
        let (source, health) = if in_wall {
            ("minecraft:in_wall", 19.0)
        } else {
            ("minecraft:outside_border", 16.0)
        };
        assert_eq!(damages(&tick), [damage_id(source)]);
        assert_eq!(world.levels[PLAYER].damage.health, health);
    }
}

#[test]
fn player_environment_respects_datapack_breathing_tag() {
    for (member, expected) in [(true, -15), (false, 0)] {
        let mut world = world(GameType::Survival);
        pool(&mut world);
        let tags = vec![
            (
                "minecraft:fluid".to_string(),
                "minecraft:water".to_string(),
                vec![
                    "minecraft:water".to_string(),
                    "minecraft:flowing_water".to_string(),
                ],
            ),
            (
                "minecraft:entity_type".to_string(),
                "minecraft:can_breathe_under_water".to_string(),
                if member {
                    vec!["minecraft:player".to_string()]
                } else {
                    vec![]
                },
            ),
        ];
        world.install_level_registry_tags(std::sync::Arc::new(
            crate::registry_tags::RegistryTagSnapshot::new(tags),
        ));
        world.levels[PLAYER].set_air_supply(-19);
        world.tick(Vec::new());
        assert_eq!(world.levels[PLAYER].air_supply, expected, "member={member}");
        assert_eq!(
            world.levels[PLAYER].damage.health,
            if member { 20.0 } else { 18.0 }
        );
    }
}

#[test]
fn player_environment_submerged_horse_dismounts_even_with_water_breathing() {
    let mut world = world(GameType::Survival);
    let player = world.levels.player_at(PLAYER);
    let position = player.position();
    let dimension = player.dimension;
    let player_entity = player.entity_id();
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID;
    let horse = world
        .spawn_stored_entity_for_test(
            bbb_server_types::save::Java26BuiltinDimension::Overworld,
            kind,
            position,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(kind).unwrap(),
            ),
        )
        .0;
    assert!(world.attach_external_player_for_test(PLAYER, horse));
    for x in position.x.floor() as i32 - 2..=position.x.floor() as i32 + 2 {
        for z in position.z.floor() as i32 - 2..=position.z.floor() as i32 + 2 {
            for y in 79..=87 {
                block(&mut world, BlockPos { x, y, z }, "minecraft:water");
            }
        }
    }
    effect(&mut world, "minecraft:water_breathing", 100);
    world.tick(Vec::new());
    assert!(world.levels[PLAYER]
        .fluid_interaction
        .is_eye_in_fluid(FluidTag::Water));
    assert_eq!(
        world
            .level_by_id(&dimension)
            .entities
            .external_player_vehicle(player_entity),
        None
    );
    assert_eq!(world.levels[PLAYER].damage.health, 20.0);
    assert_eq!(world.levels[PLAYER].air_supply, 300);
}

#[test]
fn player_environment_lava_halves_fall_distance_each_tick() {
    let mut world = world(GameType::Creative);
    let eye = eye_block(&world);
    for x in eye.x - 2..=eye.x + 2 {
        for z in eye.z - 2..=eye.z + 2 {
            for y in 78..=82 {
                block(&mut world, BlockPos { x, y, z }, "minecraft:lava");
            }
        }
    }
    world.tick(Vec::new());
    world.levels[PLAYER].block_contact.fall_distance = 8.0;
    world.tick(Vec::new());
    assert!(world.levels[PLAYER]
        .fluid_interaction
        .is_in_fluid(FluidTag::Lava));
    assert_eq!(world.levels[PLAYER].block_contact.fall_distance, 4.0);
}

/// Player.java:693 drops the shoulder parrots before LivingEntity.java:1164-1166
/// refuses fire damage under Fire Resistance.
#[test]
fn player_environment_fire_resistance_still_knocks_shoulder_parrots_off() {
    let mut world = world(GameType::Survival);
    let dimension = world.levels[PLAYER].dimension;
    // Shoulder riders stay seated for their first twenty ticks.
    world
        .level_mut_by_id(&dimension)
        .entities
        .set_game_time(100);
    let parrot = vec![
        bbb_protocol::NbtSummaryEntry {
            name: "id".into(),
            value: bbb_protocol::NbtSummaryValue::String("minecraft:parrot".into()),
        },
        bbb_protocol::NbtSummaryEntry {
            name: "Variant".into(),
            value: bbb_protocol::NbtSummaryValue::Int(0),
        },
    ];
    world.levels[PLAYER].cold.shoulders =
        crate::world::player_shoulders::PlayerShoulders::from_saved([parrot, Vec::new()]);
    effect(&mut world, "minecraft:fire_resistance", 100);
    let mut events = Vec::new();
    assert!(!world.hurt_connected_player(
        PLAYER,
        "minecraft:in_fire",
        1.0,
        None,
        None,
        None,
        &mut events
    ));
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.damage.health, 20.0);
    assert!(player.cold.shoulders.entities.iter().all(Vec::is_empty));
}
