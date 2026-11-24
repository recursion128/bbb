use super::*;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::packets::{MovePlayer, PlayServerbound};
use bbb_server_types::save::Java26BuiltinDimension;
use std::sync::Arc;

const PLAYER: ConnectionId = ConnectionId(1);
const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const FEET: BlockPos = BlockPos { x: 8, y: 65, z: 8 };
const START: Vec3d = Vec3d {
    x: 8.4,
    y: 65.25,
    z: 8.4,
};

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn property(state: i32, name: &str, value: &str) -> i32 {
    crate::fluid::with_property(state, name, value).unwrap()
}

fn write(world: &mut ServerWorld, pos: BlockPos, state: i32) {
    assert!(world
        .level_mut(DIMENSION)
        .chunks
        .set_block_state(pos.x, pos.y, pos.z, state)
        .is_some());
}

fn fixture() -> ServerWorld {
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        game_type: GameType::Creative,
        ..crate::tests::datapack_config()
    })
    .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
    for x in 6..=10 {
        for z in 6..=10 {
            for y in 63..=70 {
                write(&mut world, BlockPos { x, y, z }, state("minecraft:air"));
            }
        }
    }
    write(
        &mut world,
        BlockPos { y: 64, ..FEET },
        state("minecraft:stone"),
    );
    world.set_player_position_for_test(PLAYER, START);
    assert!(world.levels[PLAYER].abilities.can_fly);
    assert!(!world.levels[PLAYER].abilities.flying);
    world
}

fn play(world: &mut ServerWorld, packet: PlayServerbound) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    world.apply_command(
        crate::world::WorldCommand::Play {
            connection_id: PLAYER,
            packet,
        },
        &mut events,
    );
    events
}

fn rise(world: &mut ServerWorld) {
    let from = world.levels[PLAYER].position();
    let to = Vec3d {
        y: from.y + 0.1,
        ..from
    };
    let events = play(
        world,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(to),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.position(), to);
    assert_eq!(player.last_good, to);
    assert_eq!(player.awaiting_teleport, None);
    assert!(!events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: PLAYER,
            packet: ClientboundPlayPacket::PlayerPosition(_),
            ..
        }
    )));
}

fn stat(world: &ServerWorld, name: &str) -> i32 {
    world.player_stat_for_test(PLAYER, custom_stat(name))
}

fn assert_only_climb(world: &ServerWorld, centimeters: i32) {
    assert_eq!(stat(world, "minecraft:climb_one_cm"), centimeters);
    for name in [
        "minecraft:walk_one_cm",
        "minecraft:fly_one_cm",
        "minecraft:aviate_one_cm",
        "minecraft:swim_one_cm",
        "minecraft:walk_under_water_one_cm",
        "minecraft:walk_on_water_one_cm",
    ] {
        assert_eq!(stat(world, name), 0, "{name}");
    }
}

fn edit_block_tag(world: &mut ServerWorld, tag_to_edit: &str, edit: impl Fn(&mut Vec<String>)) {
    let config = crate::tests::datapack_config();
    let registries = &config.datapacks.as_ref().unwrap().tags.registries;
    let tags = RegistryTagSnapshot::new(registries.iter().flat_map(|registry| {
        registry.tags.iter().map(|(tag, members)| {
            let mut members = members.clone();
            if registry.registry_id == "minecraft:block" && tag == tag_to_edit {
                edit(&mut members);
            }
            (registry.registry_id.clone(), tag.clone(), members)
        })
    }));
    world.install_level_registry_tags(Arc::new(tags));
}

#[test]
fn climbable_stats_tests_ladder_flight_toggle_uses_actual_abilities_packet() {
    let mut world = fixture();
    write(&mut world, FEET, state("minecraft:ladder"));
    rise(&mut world);
    assert_only_climb(&world, 10);

    play(
        &mut world,
        PlayServerbound::PlayerAbilities { flying: true },
    );
    assert!(world.levels[PLAYER].abilities.flying);
    rise(&mut world);
    assert_only_climb(&world, 10);

    play(
        &mut world,
        PlayServerbound::PlayerAbilities { flying: false },
    );
    assert!(!world.levels[PLAYER].abilities.flying);
    rise(&mut world);
    assert_only_climb(&world, 20);
}

#[test]
fn climbable_stats_tests_live_trapdoors_tag_cannot_replace_block_class() {
    let mut world = fixture();
    edit_block_tag(&mut world, "minecraft:trapdoors", |members| {
        members.retain(|member| member != "minecraft:oak_trapdoor");
        members.push("minecraft:oak_fence_gate".to_owned());
    });
    let tags = world.player_registry_tags(PLAYER).unwrap();
    assert!(!tags.contains(
        "minecraft:block",
        "minecraft:trapdoors",
        "minecraft:oak_trapdoor"
    ));
    assert!(tags.contains(
        "minecraft:block",
        "minecraft:trapdoors",
        "minecraft:oak_fence_gate"
    ));
    write(
        &mut world,
        BlockPos { y: 64, ..FEET },
        state("minecraft:ladder"),
    );
    let trapdoor = property(state("minecraft:oak_trapdoor"), "open", "true");
    write(&mut world, FEET, trapdoor);
    rise(&mut world);
    assert_only_climb(&world, 10);

    write(
        &mut world,
        FEET,
        property(state("minecraft:oak_fence_gate"), "open", "true"),
    );
    rise(&mut world);
    assert_only_climb(&world, 10);

    write(&mut world, FEET, trapdoor);
    write(
        &mut world,
        BlockPos { y: 64, ..FEET },
        property(state("minecraft:ladder"), "facing", "south"),
    );
    rise(&mut world);
    assert_only_climb(&world, 10);
}

#[test]
fn climbable_stats_tests_live_climbable_tag_precedes_trapdoor_geometry() {
    let mut world = fixture();
    edit_block_tag(&mut world, "minecraft:climbable", |members| {
        members.push("minecraft:oak_fence_gate".to_owned());
    });
    write(
        &mut world,
        FEET,
        property(state("minecraft:oak_fence_gate"), "open", "true"),
    );
    // The supporting cell is stone, so only the live CLIMBABLE membership permits climbing.
    rise(&mut world);
    assert_only_climb(&world, 10);
}
