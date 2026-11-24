use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::entity::living::FallImpulseContext;
use crate::explosion::ExplosionSourceEntityKind;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::packets::{BlockPos, MovePlayer, PlayServerbound, Vec3d};
use bbb_server_types::save::Java26BuiltinDimension;
use std::sync::Arc;

const PLAYER: ConnectionId = ConnectionId(1);
const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const FEET: BlockPos = BlockPos { x: 8, y: 65, z: 8 };
const START: Vec3d = Vec3d {
    x: 8.5,
    y: 65.25,
    z: 8.5,
};

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn write(world: &mut ServerWorld, pos: BlockPos, value: i32) {
    if world.block_state(DIMENSION, pos) != Some(value) {
        assert!(world.set_block(DIMENSION, pos, value, SET_BLOCK_UPDATE_ALL));
    }
    assert_eq!(world.block_state(DIMENSION, pos), Some(value));
}

fn dispatch(world: &mut ServerWorld, packet: PlayServerbound) -> Vec<crate::world::WorldEvent> {
    let mut events = Vec::new();
    world.apply_command(
        crate::world::WorldCommand::Play {
            connection_id: PLAYER,
            packet,
        },
        &mut events,
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, crate::world::WorldEvent::Close { .. })));
    events
}

fn fixture() -> ServerWorld {
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert_eq!(world.levels.player_count(), 1);
    world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
    for x in 7..=10 {
        for z in 7..=10 {
            for y in 63..=69 {
                write(&mut world, BlockPos { x, y, z }, state("minecraft:air"));
            }
        }
    }
    if world.levels[PLAYER].game_type != GameType::Creative {
        assert!(world.set_player_game_mode(PLAYER, GameType::Creative, &mut Vec::new()));
    }
    assert_eq!(world.levels[PLAYER].game_type, GameType::Creative);
    assert!(world.levels[PLAYER].abilities.can_fly);
    assert!(!world.levels[PLAYER].abilities.flying);
    world.set_player_position_for_test(PLAYER, START);
    world.levels[PLAYER].set_on_ground(false);
    world
}

fn expired_explosion_context(world: &mut ServerWorld) -> ExplosionSourceEntity {
    let player = world.levels.player_mut(PLAYER).unwrap();
    let source = ExplosionSourceEntity {
        custom_name: None,
        entity_id: 10000,
        entity_resource_id: "minecraft:wind_charge".to_owned(),
        uuid: uuid::Uuid::from_u128(10000),
        position: player.position(),
        kind: ExplosionSourceEntityKind::Other,
    };
    player.on_explosion_impulse(Some(&source), None);
    assert_eq!(player.cold.fall_impulse.reset_grace_time, 40);
    for _ in 0..40 {
        player.cold.fall_impulse.tick();
    }
    assert_eq!(
        player.cold.fall_impulse,
        FallImpulseContext {
            reset_grace_time: 0,
            impact_position: Some(source.position),
        }
    );
    assert_eq!(
        player.cold.impulse_explosion_cause.as_ref(),
        Some(&(source.clone(), None))
    );
    source
}

fn accepted_airborne_move(world: &mut ServerWorld, source: &ExplosionSourceEntity, resets: bool) {
    let start = world.levels[PLAYER].position();
    let target = Vec3d {
        x: start.x + 0.05,
        ..start
    };
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.block_contact.fall_distance, 0.0);
    assert_eq!(player.cold.fall_impulse.reset_grace_time, 0);
    assert_eq!(
        player.cold.impulse_explosion_cause.as_ref(),
        Some(&(source.clone(), None))
    );
    assert!(!player.fluid_interaction.is_in_fluid(FluidTag::Water));
    assert!(!player.fluid_interaction.is_in_fluid(FluidTag::Lava));
    assert_ne!(player.game_type, GameType::Spectator);
    assert!(!player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING));
    let events = dispatch(
        world,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(target),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        crate::world::WorldEvent::Send {
            packet: ClientboundPlayPacket::PlayerPosition(_),
            ..
        }
    )));
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.position(), target);
    assert_eq!(player.last_good, target);
    assert!(player.awaiting_teleport.is_none());
    assert!(!player.on_ground());
    assert!(!player.fluid_interaction.is_in_fluid(FluidTag::Water));
    assert!(!player.fluid_interaction.is_in_fluid(FluidTag::Lava));
    assert_eq!(player.cold.fall_impulse.reset_grace_time, 0);
    assert_eq!(
        player.cold.fall_impulse.impact_position,
        if resets { None } else { Some(source.position) }
    );
    assert_eq!(
        player
            .cold
            .impulse_explosion_cause
            .as_ref()
            .map(|(source, nbt)| (source, nbt.as_ref())),
        if resets { None } else { Some((source, None)) }
    );
}

fn trapdoor_tag(world: &mut ServerWorld, members: &[&str]) {
    let config = crate::tests::datapack_config();
    let registries = &config.datapacks.as_ref().unwrap().tags.registries;
    let mut replaced = false;
    let tags: Vec<_> = registries
        .iter()
        .flat_map(|registry| {
            registry
                .tags
                .iter()
                .map(|(tag, existing)| {
                    let values = if registry.registry_id == "minecraft:block"
                        && tag == "minecraft:trapdoors"
                    {
                        replaced = true;
                        members.iter().map(|member| (*member).to_string()).collect()
                    } else {
                        existing.clone()
                    };
                    (registry.registry_id.clone(), tag.clone(), values)
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(replaced);
    let snapshot = Arc::new(RegistryTagSnapshot::new(tags));
    for name in ["minecraft:oak_trapdoor", "minecraft:oak_fence_gate"] {
        assert!(!snapshot.contains("minecraft:block", "minecraft:climbable", name));
        assert_eq!(
            snapshot.contains("minecraft:block", "minecraft:trapdoors", name),
            members.contains(&name)
        );
    }
    world.install_level_registry_tags(snapshot);
}

fn install_with_matching_ladder(world: &mut ServerWorld, current: i32) {
    let facing = crate::block_behavior::string_property(current, "facing").unwrap();
    let below = BlockPos {
        y: FEET.y - 1,
        ..FEET
    };
    write(
        world,
        BlockPos {
            z: below.z + 1,
            ..below
        },
        state("minecraft:stone"),
    );
    write(
        world,
        below,
        with_property(state("minecraft:ladder"), "facing", &facing).unwrap(),
    );
    write(world, FEET, current);
}

#[test]
fn climbable_packet_tests_ladder_resets_until_legal_abilities_packet_enables_flying() {
    // Player.java:2001-2003; ServerGamePacketListenerImpl.java:1166-1176.
    let mut world = fixture();
    write(
        &mut world,
        BlockPos {
            z: FEET.z + 1,
            ..FEET
        },
        state("minecraft:stone"),
    );
    write(&mut world, FEET, state("minecraft:ladder"));
    let source = expired_explosion_context(&mut world);
    accepted_airborne_move(&mut world, &source, true);
    let source = expired_explosion_context(&mut world);
    dispatch(
        &mut world,
        PlayServerbound::PlayerAbilities { flying: true },
    );
    assert!(world.levels[PLAYER].abilities.flying);
    assert_eq!(
        world.levels[PLAYER].cold.fall_impulse.impact_position,
        Some(source.position)
    );
    accepted_airborne_move(&mut world, &source, false);
    dispatch(
        &mut world,
        PlayServerbound::PlayerAbilities { flying: false },
    );
    assert!(!world.levels[PLAYER].abilities.flying);
    accepted_airborne_move(&mut world, &source, true);
}

#[test]
fn climbable_packet_tests_real_trapdoor_ignores_removed_trapdoors_tag_but_requires_open() {
    // LivingEntity.java:1696-1714: TrapDoorBlock class and facing own this branch, not its tag.
    for open in [true, false] {
        let mut world = fixture();
        trapdoor_tag(&mut world, &[]);
        let current = with_property(
            state("minecraft:oak_trapdoor"),
            "open",
            if open { "true" } else { "false" },
        )
        .unwrap();
        install_with_matching_ladder(&mut world, current);
        let source = expired_explosion_context(&mut world);
        accepted_airborne_move(&mut world, &source, open);
    }
}

#[test]
fn climbable_packet_tests_tagged_open_fence_gate_with_matching_ladder_cannot_reset() {
    let mut world = fixture();
    trapdoor_tag(&mut world, &["minecraft:oak_fence_gate"]);
    install_with_matching_ladder(
        &mut world,
        with_property(state("minecraft:oak_fence_gate"), "open", "true").unwrap(),
    );
    let source = expired_explosion_context(&mut world);
    accepted_airborne_move(&mut world, &source, false);
}
