use super::*;
use crate::admin::NameAndId;
use crate::entity::{EntityData, GeneralLivingData};
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id;

fn world() -> ServerWorld {
    world_with_config(crate::tests::creative_fixture_config())
}

fn world_with_config(config: ServerWorldConfig) -> ServerWorld {
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world
}

fn mob(world: &mut ServerWorld, dimension: LevelId, position: Vec3d) -> (i32, Uuid) {
    let type_id = vanilla_entity_type_id_for_resource_id("minecraft:cow").unwrap();
    world.spawn_stored_entity_for_test_by_key(
        &dimension,
        type_id,
        position,
        EntityData::GeneralLiving(GeneralLivingData::new(type_id).unwrap()),
    )
}

fn cameras(events: &[WorldEvent]) -> Vec<(ConnectionId, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::SetCamera(packet),
            } => Some((*connection_id, packet.camera_id)),
            _ => None,
        })
        .collect()
}

#[test]
fn spectate_requires_loaded_spectator_current_level_reach_border_and_pickability() {
    let mut world = world();
    let id = ConnectionId(1);
    let dimension = world.levels[id].dimension;
    let position = world.levels[id].position();
    let (target, _) = mob(&mut world, dimension, position);
    let mut events = Vec::new();
    world.apply_spectate_entity(id, target, &mut events);
    assert!(cameras(&events).is_empty());
    world.set_player_game_mode(id, GameType::Spectator, &mut events);
    let foreign = Java26LevelKey::from(Java26BuiltinDimension::Nether);
    let foreign_id = world.require_level_id(&foreign);
    let (foreign_target, _) = mob(&mut world, foreign_id, position);
    world.apply_spectate_entity(id, foreign_target, &mut events);
    assert!(world.levels[id].cold.camera.is_none());
    let far = Vec3d {
        x: position.x + 30.0,
        ..position
    };
    let (far_target, _) = mob(&mut world, dimension, far);
    world.apply_spectate_entity(id, far_target, &mut events);
    assert!(world.levels[id].cold.camera.is_none());
    world.apply_spectate_entity(id, target, &mut events);
    assert_eq!(cameras(&events), [(id, target)]);
    events.clear();
    world.apply_spectate_entity(id, target, &mut events);
    assert!(cameras(&events).is_empty());
}

#[test]
fn stored_camera_follows_rotation_then_sneak_or_death_releases_it() {
    let mut world = world();
    let id = ConnectionId(1);
    let dimension = world.levels[id].dimension;
    let position = world.levels[id].position();
    let (target, uuid) = mob(&mut world, dimension, position);
    let camera = CommandCameraTarget::Stored {
        dimension: world.level_key(dimension).clone(),
        entity_id: target,
        uuid,
    };
    let mut events = Vec::new();
    world.set_player_camera(id, Some(camera.clone()), &mut events);
    let entity = world
        .level_mut_by_id(&dimension)
        .entities
        .get_mut(target)
        .unwrap();
    let position = Vec3d {
        x: position.x + 4.0,
        ..position
    };
    entity.set_pos(position);
    entity.set_rot(45.0, 12.0);
    world.tick_spectator_cameras(&mut events);
    assert_eq!(world.levels[id].position(), position);
    assert_eq!(
        world.levels[id].rotation(),
        PlayerRotation {
            y_rot: 45.0,
            x_rot: 12.0
        }
    );
    world.levels[id].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, true);
    world.tick_spectator_cameras(&mut events);
    assert!(world.levels[id].cold.camera.is_none());
    world.levels[id].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, false);
    world.set_player_camera(id, Some(camera), &mut events);
    world
        .level_mut_by_id(&dimension)
        .entities
        .get_mut(target)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    world.tick_spectator_cameras(&mut events);
    assert!(world.levels[id].cold.camera.is_none());
    assert_eq!(
        cameras(&events).last(),
        Some(&(id, world.levels[id].entity_id()))
    );
}

#[test]
fn camera_switch_crosses_dimensions_before_set_camera_and_game_mode_clears_it() {
    let mut world = world();
    let id = ConnectionId(1);
    let dimension = Java26LevelKey::from(Java26BuiltinDimension::Nether);
    let position = Vec3d {
        x: 10.0,
        y: 70.0,
        z: 5.0,
    };
    let dimension_id = world.require_level_id(&dimension);
    let (target, uuid) = mob(&mut world, dimension_id, position);
    let mut events = Vec::new();
    world.set_player_game_mode(id, GameType::Spectator, &mut events);
    events.clear();
    world.set_player_camera(
        id,
        Some(CommandCameraTarget::Stored {
            dimension: dimension.clone(),
            entity_id: target,
            uuid,
        }),
        &mut events,
    );
    assert_eq!(
        world.levels[id].dimension,
        world.require_level_id(&dimension)
    );
    assert_eq!(world.levels[id].position(), position);
    let respawn = events.iter().position(|event| matches!(event, WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::Respawn(_) } if *connection_id == id)).unwrap();
    let camera = events.iter().position(|event| matches!(event, WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::SetCamera(_) } if *connection_id == id)).unwrap();
    assert!(respawn < camera);
    world.set_player_game_mode(id, GameType::Creative, &mut events);
    assert!(world.levels[id].cold.camera.is_none());
}

#[test]
fn spectator_menu_teleports_by_uuid_without_reach_and_uses_target_rotation() {
    let mut world = world();
    let id = ConnectionId(1);
    let dimension = Java26LevelKey::from(Java26BuiltinDimension::Nether);
    let position = Vec3d {
        x: 40.0,
        y: 70.0,
        z: 5.0,
    };
    let dimension_id = world.require_level_id(&dimension);
    let (target, uuid) = mob(&mut world, dimension_id, position);
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .entities
        .get_mut(target)
        .unwrap()
        .set_rot(80.0, -15.0);
    let mut events = Vec::new();
    world.apply_teleport_to_entity(id, uuid, &mut events);
    assert_ne!(world.levels[id].dimension, dimension_id);
    world.set_player_game_mode(id, GameType::Spectator, &mut events);
    world.apply_teleport_to_entity(id, uuid, &mut events);
    assert_eq!(
        world.levels[id].dimension,
        world.require_level_id(&dimension)
    );
    assert_eq!(world.levels[id].position(), position);
    assert_eq!(
        world.levels[id].rotation(),
        PlayerRotation {
            y_rot: 80.0,
            x_rot: -15.0
        }
    );
    assert!(world.levels[id].cold.camera.is_none());
}

#[test]
fn camera_target_player_changing_dimension_moves_watcher_then_clears_camera() {
    let mut world = world();
    let id = ConnectionId(1);
    let target = ConnectionId(2);
    let mut events = Vec::new();
    world.set_player_camera(
        id,
        Some(CommandCameraTarget::Player {
            uuid: world.levels[target].profile.uuid,
        }),
        &mut events,
    );
    let transition = TeleportTransition {
        new_level: Java26BuiltinDimension::Nether.into(),
        position: Vec3d {
            x: 5.0,
            y: 70.0,
            z: 9.0,
        },
        y_rot: 20.0,
        x_rot: 3.0,
        relatives_mask: 0,
    };
    world.teleport_player(target, transition.clone(), &mut events);
    assert_eq!(
        world.levels[id].dimension,
        world.require_level_id(&transition.new_level)
    );
    assert_eq!(world.levels[id].position(), transition.position);
    assert!(world.levels[id].cold.camera.is_none());
}

#[test]
fn pick_mob_creates_default_egg_but_survival_only_selects_existing_stack() {
    let mut world = world();
    let id = ConnectionId(1);
    let dimension = world.levels[id].dimension;
    let position = world.levels[id].position();
    let (target, _) = mob(&mut world, dimension, position);
    let packet = PickItemFromEntity {
        entity_id: target,
        include_data: false,
    };
    let mut events = Vec::new();
    world.set_player_game_mode(id, GameType::Survival, &mut events);
    world.apply_pick_item_from_entity(id, packet, &mut events);
    assert!(world.levels[id]
        .inventory
        .inventory_slots()
        .iter()
        .all(|stack| stack.item_id().is_none()));
    world.set_player_game_mode(id, GameType::Creative, &mut events);
    world.apply_pick_item_from_entity(id, packet, &mut events);
    let inventory = &world.levels[id].inventory;
    let stack = inventory.inventory_slots()[usize::from(inventory.selected_slot())].clone();
    assert_eq!(
        stack.item_id(),
        Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:cow_spawn_egg")
                .unwrap()
                .id
        )
    );
    world.set_player_game_mode(id, GameType::Survival, &mut events);
    world.levels[id].inventory.set_selected_slot(4);
    world.apply_pick_item_from_entity(id, packet, &mut events);
    assert_eq!(world.levels[id].inventory.selected_slot(), 0);
}

#[test]
fn spectate_loaded_and_pickable_gates_do_not_apply_to_menu_teleport() {
    let mut world = world();
    let id = ConnectionId(1);
    let other = ConnectionId(2);
    let mut events = Vec::new();
    world.set_player_game_mode(id, GameType::Spectator, &mut events);
    world.set_player_game_mode(other, GameType::Spectator, &mut events);
    let target = world.levels[other].entity_id();
    world.apply_play(
        id,
        PlayServerbound::SpectateEntity(bbb_protocol::packets::SpectateEntity {
            entity_id: target,
        }),
        &mut events,
    );
    assert!(
        world.levels[id].cold.camera.is_none(),
        "spectator players are not pickable"
    );
    world.set_player_game_mode(other, GameType::Creative, &mut events);
    world.levels[id].restart_client_load_timer_after_respawn();
    world.apply_spectate_entity(id, target, &mut events);
    assert!(
        world.levels[id].cold.camera.is_none(),
        "loading prevents click spectate"
    );
    {
        let mut position = world.levels[other].position();
        position.x += 40.0;
        world.levels[other].set_position(position);
    }
    let uuid = world.levels[other].profile.uuid;
    world.apply_play(
        id,
        PlayServerbound::TeleportToEntity(bbb_protocol::packets::TeleportToEntity { uuid }),
        &mut events,
    );
    assert_eq!(world.levels[id].position(), world.levels[other].position());
}

#[test]
fn spectate_uses_live_border_while_entity_pick_ignores_it() {
    let mut world = world();
    let id = ConnectionId(1);
    let dimension = world.levels[id].dimension;
    let position = world.levels[id].position();
    let (target, _) = mob(&mut world, dimension, position);
    world.level_mut_by_id(&dimension).world_border =
        WorldBorder::from_settings(100.0, 100.0, 0.2, 5.0, 5, 15, 1.0, 0, 1.0, 0);
    let mut events = Vec::new();
    world.set_player_game_mode(id, GameType::Spectator, &mut events);
    world.apply_spectate_entity(id, target, &mut events);
    assert!(world.levels[id].cold.camera.is_none());
    world.set_player_game_mode(id, GameType::Creative, &mut events);
    world.apply_pick_item_from_entity(
        id,
        PickItemFromEntity {
            entity_id: target,
            include_data: false,
        },
        &mut events,
    );
    assert!(world.levels[id]
        .inventory
        .inventory_slots()
        .iter()
        .any(|stack| stack.item_id().is_some()));
}

#[test]
fn entity_selection_uses_runtime_range_strict_boundary_and_pose_eye_height() {
    let mut world = world();
    let id = ConnectionId(1);
    let dimension = world.levels[id].dimension;
    let position = world.levels[id].position();
    let (target_id, _) = mob(&mut world, dimension, position);
    let mut target = world.camera_entity_by_id(&dimension, target_id).unwrap();
    // The creative fixture adds `+2.0` (ServerPlayer.java:620-627), so 5 + 2 + 3 = 10.
    world.levels[id]
        .attributes
        .get_instance(VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID)
        .unwrap()
        .set_base_value(5.0);
    let eye = world.levels[id].eye_position().unwrap();
    target.bounds = Aabb::of_entity(
        Vec3d {
            x: eye.x + 10.0,
            y: eye.y,
            z: eye.z,
        },
        0.0,
        0.0,
    );
    assert!(
        !world.player_reaches_camera_entity(id, &target),
        "exact boundary is excluded"
    );
    target.bounds = Aabb::of_entity(
        Vec3d {
            x: eye.x + 9.999,
            y: eye.y,
            z: eye.z,
        },
        0.0,
        0.0,
    );
    assert!(world.player_reaches_camera_entity(id, &target));
    world.levels[id].set_pose(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID);
    assert!(
        !world.player_reaches_camera_entity(id, &target),
        "the lower eye changes the distance"
    );
}

#[test]
fn a_dead_camera_target_releases_observers_and_respawn_clears_own_camera() {
    let mut world = world();
    let id = ConnectionId(1);
    let other = ConnectionId(2);
    let mut events = Vec::new();
    world.set_player_camera(
        id,
        Some(CommandCameraTarget::Player {
            uuid: world.levels[other].profile.uuid,
        }),
        &mut events,
    );
    world.set_player_camera(
        other,
        Some(CommandCameraTarget::Player {
            uuid: world.levels[id].profile.uuid,
        }),
        &mut events,
    );
    world.levels[other].damage.health = 0.0;
    world.tick_spectator_cameras(&mut events);
    assert!(world.levels[id].cold.camera.is_none());
    assert!(world.levels[other].cold.camera.is_some());
    world.perform_respawn(other, &mut events);
    assert!(world.levels[other].cold.camera.is_none());
    assert!(world.levels[other].damage.health > 0.0);
}

#[test]
fn stored_cross_dimension_transfer_moves_observer_then_releases_old_identity() {
    let mut world = world();
    let id = ConnectionId(1);
    let source = world.levels[id].dimension;
    let position = world.levels[id].position();
    let target = Java26LevelKey::from(Java26BuiltinDimension::Nether);
    let (entity_id, uuid) = mob(&mut world, source, position);
    let source_key = world.level_key(source).clone();
    let mut events = Vec::new();
    world.set_player_camera(
        id,
        Some(CommandCameraTarget::Stored {
            dimension: source_key.clone(),
            entity_id,
            uuid,
        }),
        &mut events,
    );
    let transition = crate::entity::StoredEntityTransition {
        position: Vec3d {
            x: 64.0,
            y: 72.0,
            z: 64.0,
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 33.0,
        x_rot: 4.0,
        relatives_mask: 0,
    };
    world.ensure_entity_transfer_full_chunk_for_test_by_key(
        &world.require_level_id(&target),
        (4, 4),
    );
    let moved = world
        .transfer_stored_entity_tree_cross_level(source_key, target.clone(), entity_id, transition)
        .unwrap();
    assert_ne!(moved.new_root_id, entity_id);
    let tick = world.tick(Vec::new());
    assert_eq!(world.levels[id].dimension, world.require_level_id(&target));
    assert_eq!(world.levels[id].position(), transition.position);
    assert!(world.levels[id].cold.camera.is_none());
    assert!(cameras(&tick.events).contains(&(id, world.levels[id].entity_id())));
}

#[test]
fn avatar_pick_profile_requires_creative_operator_and_respects_system_feedback() {
    use crate::admin::{OperatorEntry, StoredUserLists};
    use crate::commands::CommandPermission;
    use bbb_protocol::packets::ClientChatVisibility;
    let mut lists = StoredUserLists::default();
    let user = NameAndId {
        id: Uuid::from_u128(1),
        name: "alice".into(),
    };
    lists.operators.insert(
        user.id,
        OperatorEntry {
            user,
            level: CommandPermission::Owner,
            bypasses_player_limit: false,
        },
    );
    let mut world = world_with_config(ServerWorldConfig {
        admin_lists: lists,
        ..crate::tests::creative_fixture_config()
    });
    let id = ConnectionId(1);
    let target = world.levels[ConnectionId(2)].entity_id();
    let pick = PlayServerbound::PickItemFromEntity(PickItemFromEntity {
        entity_id: target,
        include_data: true,
    });
    let mut events = Vec::new();
    world.apply_play(id, pick.clone(), &mut events);
    let messages: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::SystemChat(message),
            } => Some((*connection_id, message)),
            _ => None,
        })
        .collect();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].0, id);
    let content = bbb_protocol::canonical_snbt_string(messages[0].1.content.component());
    assert!(content.contains("commands.fetchprofile.entity.success"));
    assert!(content.contains("copy_to_clipboard"));
    assert!(content.contains("minecraft:player_head"));
    assert!(
        !events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetHeldSlot(_),
                ..
            }
        )),
        "player pick has no item"
    );
    events.clear();
    world.levels[id].cold.client_information.chat_visibility = ClientChatVisibility::Hidden;
    world.apply_play(id, pick.clone(), &mut events);
    assert!(events.is_empty());
    world.levels[id].cold.client_information.chat_visibility = ClientChatVisibility::System;
    world.apply_play(id, pick.clone(), &mut events);
    assert_eq!(events.len(), 1);
    world.admin.set_game_rule(
        RULE_SEND_COMMAND_FEEDBACK,
        bbb_protocol::game_rules::GameRuleValue::Boolean(false),
    );
    events.clear();
    world.apply_play(id, pick.clone(), &mut events);
    assert!(events.is_empty(), "success feedback respects the gamerule");
    world.admin.set_game_rule(
        RULE_SEND_COMMAND_FEEDBACK,
        bbb_protocol::game_rules::GameRuleValue::Boolean(true),
    );
    world.set_player_game_mode(id, GameType::Survival, &mut events);
    events.clear();
    world.apply_play(id, pick, &mut events);
    assert!(
        events.is_empty(),
        "operator permission alone does not allow profile pick"
    );
    world.apply_play(
        ConnectionId(2),
        PlayServerbound::PickItemFromEntity(PickItemFromEntity {
            entity_id: target,
            include_data: true,
        }),
        &mut events,
    );
    assert!(
        events.is_empty(),
        "creative ability alone does not allow profile pick"
    );
}

#[test]
fn stored_passenger_camera_transfer_preserves_per_node_offsets_and_rotation() {
    let mut world = world();
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26LevelKey::from(Java26BuiltinDimension::Nether);
    let base = world.levels[ConnectionId(1)].position();
    let passenger_pos = Vec3d {
        x: base.x + 1.0,
        y: base.y + 1.0,
        z: base.z,
    };
    let (root, passenger) = world
        .spawn_stored_entity_transfer_tree_for_test(source, base, passenger_pos)
        .unwrap();
    let source_key = Java26LevelKey::from(source);
    world
        .level_mut_by_id(&world.require_level_id(&source_key))
        .entities
        .get_mut(root)
        .unwrap()
        .set_rot(10.0, 2.0);
    world
        .level_mut_by_id(&world.require_level_id(&source_key))
        .entities
        .get_mut(passenger)
        .unwrap()
        .set_rot(30.0, 5.0);
    let mut events = Vec::new();
    for (id, entity_id) in [(ConnectionId(1), root), (ConnectionId(2), passenger)] {
        let uuid = world
            .level_by_id(&world.require_level_id(&source_key))
            .entities
            .get(entity_id)
            .unwrap()
            .uuid();
        world.set_player_camera(
            id,
            Some(CommandCameraTarget::Stored {
                dimension: source_key.clone(),
                entity_id,
                uuid,
            }),
            &mut events,
        );
    }
    world.ensure_entity_transfer_full_chunk_for_test_by_key(
        &world.require_level_id(&target),
        (4, 4),
    );
    world
        .transfer_stored_entity_tree_cross_level(
            source_key,
            target.clone(),
            root,
            crate::entity::StoredEntityTransition {
                position: Vec3d {
                    x: 64.0,
                    y: 72.0,
                    z: 64.0,
                },
                delta_movement: Vec3d::default(),
                y_rot: 90.0,
                x_rot: 10.0,
                relatives_mask: 0,
            },
        )
        .unwrap();
    world.flush_camera_transitions(&mut events);
    assert_eq!(
        world.levels[ConnectionId(1)].rotation(),
        PlayerRotation {
            y_rot: 90.0,
            x_rot: 10.0
        }
    );
    assert_eq!(
        world.levels[ConnectionId(2)].rotation(),
        PlayerRotation {
            y_rot: 110.0,
            x_rot: 13.0
        }
    );
    assert_eq!(
        world.levels[ConnectionId(2)].position(),
        Vec3d {
            x: 65.0,
            y: 73.0,
            z: 64.0
        }
    );
    for id in [ConnectionId(1), ConnectionId(2)] {
        assert_eq!(world.levels[id].dimension, world.require_level_id(&target));
        assert!(world.levels[id].cold.camera.is_none());
    }
}

#[test]
fn spectate_command_resolves_stored_uuid_and_rejects_zero_tracking_range() {
    use crate::admin::{OperatorEntry, StoredUserLists};
    use crate::commands::CommandPermission;
    let mut lists = StoredUserLists::default();
    let user = NameAndId {
        id: Uuid::from_u128(1),
        name: "alice".into(),
    };
    lists.operators.insert(
        user.id,
        OperatorEntry {
            user,
            level: CommandPermission::Owner,
            bypasses_player_limit: false,
        },
    );
    let mut world = world_with_config(ServerWorldConfig {
        admin_lists: lists,
        ..crate::tests::creative_fixture_config()
    });
    let id = ConnectionId(1);
    let dimension = world.levels[id].dimension;
    let position = world.levels[id].position();
    let (entity_id, uuid) = mob(&mut world, dimension, position);
    let mut events = Vec::new();
    world.set_player_game_mode(id, GameType::Spectator, &mut events);
    world.apply_play(
        id,
        PlayServerbound::ChatCommand {
            command: format!("spectate {uuid}"),
        },
        &mut events,
    );
    assert_eq!(cameras(&events).last(), Some(&(id, entity_id)));
    world.apply_play(
        id,
        PlayServerbound::ChatCommand {
            command: "spectate".into(),
        },
        &mut events,
    );
    assert!(world.levels[id].cold.camera.is_none());
    world.spawn_stored_entity_for_test_by_key(
        &dimension,
        vanilla_entity_type_id_for_resource_id("minecraft:marker").unwrap(),
        position,
        EntityData::Marker(crate::entity::MarkerData::new()),
    );
    events.clear();
    world.apply_play(
        id,
        PlayServerbound::ChatCommand {
            command: "spectate @e[type=minecraft:marker,limit=1]".into(),
        },
        &mut events,
    );
    assert!(world.levels[id].cold.camera.is_none());
    assert!(events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::SystemChat(message), .. } if bbb_protocol::canonical_snbt_string(message.content.component()).contains("commands.spectate.cannot_spectate"))), "{events:?}");
}

#[test]
fn session_eye_height_matches_source_asserted_avatar_poses_and_sleeping_is_unscaled() {
    use bbb_protocol::entity_poses::*;
    let mut world = world();
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    player
        .attributes
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(2.0);
    for (pose, height, scale) in [
        (VANILLA_ENTITY_POSE_STANDING_ID, 1.62_f32, 2.0_f32),
        (VANILLA_ENTITY_POSE_CROUCHING_ID, 1.27, 2.0),
        (VANILLA_ENTITY_POSE_FALL_FLYING_ID, 0.4, 2.0),
        (VANILLA_ENTITY_POSE_SWIMMING_ID, 0.4, 2.0),
        (VANILLA_ENTITY_POSE_SPIN_ATTACK_ID, 0.4, 2.0),
        (VANILLA_ENTITY_POSE_DYING_ID, 1.62, 1.0),
        (VANILLA_ENTITY_POSE_SLEEPING_ID, 0.2, 1.0),
    ] {
        player.set_pose(pose);
        assert_eq!(
            player.eye_position().unwrap().y,
            player.position().y + f64::from(height * scale)
        );
    }
}
