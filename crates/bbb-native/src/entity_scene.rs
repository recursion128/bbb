use bbb_renderer::{SelectionBox, SelectionOutline};
use bbb_world::{EntityPickTargetState, WorldStore};

pub(crate) fn entity_scene_outline_from_world_at_partial_tick(
    world: &WorldStore,
    entity_partial_tick: f32,
) -> Option<SelectionOutline> {
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    let boxes: Vec<_> = world
        .entity_pick_targets_at_partial_tick(entity_partial_tick.clamp(0.0, 1.0))
        .into_iter()
        .filter(|target| {
            local_player_id != Some(target.entity_id) && camera_entity_id != Some(target.entity_id)
        })
        .map(entity_pick_target_box)
        .collect();
    (!boxes.is_empty()).then(|| SelectionOutline::from_boxes(boxes))
}

fn entity_pick_target_box(target: EntityPickTargetState) -> SelectionBox {
    SelectionBox {
        min: [
            (target.position.x + f64::from(target.bounds.min[0])) as f32,
            (target.position.y + f64::from(target.bounds.min[1])) as f32,
            (target.position.z + f64::from(target.bounds.min[2])) as f32,
        ],
        max: [
            (target.position.x + f64::from(target.bounds.max[0])) as f32,
            (target.position.y + f64::from(target.bounds.max[1])) as f32,
            (target.position.z + f64::from(target.bounds.max[2])) as f32,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{AddEntity, CommonPlayerSpawnInfo, PlayLogin, SetCamera, Vec3d};
    use bbb_world::{EntityPickBoundsState, EntityVec3};
    use uuid::Uuid;

    const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;

    #[test]
    fn entity_scene_outline_is_none_without_visible_entity_targets() {
        assert_eq!(
            entity_scene_outline_from_world_at_partial_tick(&WorldStore::new(), 1.0),
            None
        );
    }

    #[test]
    fn entity_scene_outline_projects_pick_bounds_for_all_visible_targets() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            11,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [2.0, 1.0, 3.0],
        ));

        let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.5)
            .expect("expected entity scene outline");

        assert_eq!(outline.boxes.len(), 2);
        assert_selection_box_close(outline.boxes[0].min, [-0.49, 1.0, 2.51]);
        assert_selection_box_close(outline.boxes[0].max, [0.49, 1.7, 3.49]);
        assert_selection_box_close(outline.boxes[1].min, [1.51, 1.0, 2.51]);
        assert_selection_box_close(outline.boxes[1].max, [2.49, 1.7, 3.49]);
    }

    #[test]
    fn entity_scene_outline_uses_bounds_without_pick_radius_inflation() {
        let outline_box = entity_pick_target_box(EntityPickTargetState {
            entity_id: 7,
            position: EntityVec3 {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
            bounds: EntityPickBoundsState::from_centered_size(2.0, 4.0, 6.0, 1.5),
        });

        assert_selection_box_close(outline_box.min, [9.0, 18.0, 27.0]);
        assert_selection_box_close(outline_box.max, [11.0, 22.0, 33.0]);
    }

    #[test]
    fn entity_scene_outline_filters_local_player_and_camera_entity() {
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(10));
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            11,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [2.0, 1.0, 3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [4.0, 1.0, 3.0],
        ));
        assert!(world.apply_set_camera(SetCamera { camera_id: 11 }));

        let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
            .expect("expected non-camera entity scene outline");

        assert_eq!(outline.boxes.len(), 1);
        assert_selection_box_close(outline.boxes[0].min, [3.51, 1.0, 2.51]);
        assert_selection_box_close(outline.boxes[0].max, [4.49, 1.7, 3.49]);
    }

    fn protocol_add_entity(id: i32, entity_type_id: i32, position: [f64; 3]) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678 + id as u128),
            entity_type_id,
            position: Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn protocol_play_login(player_id: i32) -> PlayLogin {
        PlayLogin {
            player_id,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        }
    }

    fn assert_selection_box_close(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "expected {expected}, got {actual}"
            );
        }
    }
}
