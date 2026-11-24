use super::*;
use crate::world::TeleportTransition;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::save::Java26BuiltinDimension;

fn entering(conditions: Value) -> Value {
    json!({"criteria":{"enter":{"trigger":"enter_block","conditions":conditions}},"rewards":{"experience":7}})
}

#[test]
fn enter_block_stationary_contact_checks_block_state_player_and_rewards_once() {
    let mut world = world(&[(
        "test:enter",
        entering(
            json!({"block":"water","state":{"level":"0"},"player":{"flags":{"is_on_ground":true}}}),
        ),
    )]);
    let dimension = world.levels[PLAYER].dimension.builtin().unwrap();
    let feet = BlockPos { x: 0, y: 64, z: 0 };
    assert!(world.set_block(
        dimension,
        feet,
        state("minecraft:water", &[("level", "0")]),
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    // Afloat half a block over the floor, inside the same water cell: the
    // tick's own travel finds nothing under the body.
    let standing = world.levels[PLAYER].position();
    world.levels[PLAYER].set_position(Vec3d {
        y: standing.y + 0.5,
        ..standing
    });
    world.tick(Vec::new());
    assert!(!done(&world, "test:enter"));
    world.levels[PLAYER].set_position(standing);
    world.tick(Vec::new());
    assert!(done(&world, "test:enter"));
    assert_eq!(xp(&world), 7);
    world.tick(Vec::new());
    assert_eq!(xp(&world), 7);
}

#[test]
fn enter_block_pearl_gateway_contact_awards_online_owner_in_another_level_during_cooldown() {
    let mut world = world(&[(
        "test:gateway",
        entering(json!({"block":"end_gateway","player":{"location":{"position":{"y":80}}}})),
    )]);
    let source = world.levels[PLAYER].dimension;
    world.join(
        ConnectionId(2),
        GameProfile {
            uuid: Uuid::from_u128(2),
            name: "observer".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    world.levels[ConnectionId(2)].set_position(Vec3d {
        x: 8.5,
        y: 64.0,
        z: 0.5,
    });
    assert!(world.teleport_player_id(
        PLAYER,
        TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 0.5,
                y: 80.0,
                z: 0.5
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut Vec::new()
    ));
    world.ensure_entity_transfer_full_chunk_for_test_by_key(&source, (0, 0));
    let gateway = BlockPos { x: 1, y: 65, z: 0 };
    assert!(world.set_block(
        source.builtin().unwrap(),
        gateway,
        state("minecraft:end_gateway", &[]),
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    world
        .level_mut_by_id(&source)
        .chunks
        .block_entities_mut()
        .get_mut(gateway)
        .unwrap()
        .end_gateway_mut()
        .unwrap()
        .trigger_cooldown();
    crate::entity::spawn_ender_pearl(
        &mut world.level_mut_by_id(&source).entities,
        Vec3d {
            x: 1.5,
            y: 65.5,
            z: 0.5,
        },
        crate::entity::projectile_shoot::ProjectileShot::new(Vec3d::default()),
        crate::entity::item::plain_stack("minecraft:ender_pearl", 1).unwrap(),
        Some(Uuid::from_u128(1)),
    )
    .unwrap();
    world.tick(Vec::new());
    assert!(done(&world, "test:gateway"));
    assert_eq!(xp(&world), 7);
    assert_eq!(
        world.levels[PLAYER].dimension,
        Java26BuiltinDimension::Nether.into()
    );
    assert!(!world.player_advancement_is_done(ConnectionId(2), "test:gateway"));
}
