//! Client-side dynamic point-light extractor for held torches/lanterns, burning entities, and glowing mobs.

use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{
    ArrowModelTexture, CameraPose, DynamicLightsMode, DynamicPointLight, EntityModelInstance,
    EntityModelKind, MAX_DYNAMIC_POINT_LIGHTS,
};
use bbb_client_world::{EntityRenderCandidateState, WorldStore};
use bbb_protocol::entity_types::vanilla_entity_type;
use bbb_protocol::packets::ItemStackSummary;

fn item_light_spec(resource_id: &str) -> Option<(f32, [f32; 3], f32)> {
    match resource_id {
        "minecraft:torch"
        | "minecraft:wall_torch"
        | "minecraft:lantern"
        | "minecraft:glowstone"
        | "minecraft:glowstone_dust"
        | "minecraft:shroomlight"
        | "minecraft:sea_lantern"
        | "minecraft:jack_o_lantern"
        | "minecraft:ochre_froglight"
        | "minecraft:verdant_froglight"
        | "minecraft:pearlescent_froglight"
        | "minecraft:lava_bucket"
        | "minecraft:blaze_rod"
        | "minecraft:end_rod"
        | "minecraft:campfire" => Some((14.0, [1.0, 0.78, 0.45], 0.95)),
        "minecraft:soul_torch" | "minecraft:soul_lantern" | "minecraft:soul_campfire" => {
            Some((10.0, [0.45, 0.85, 1.0], 0.90))
        }
        "minecraft:redstone_torch"
        | "minecraft:redstone_lamp"
        | "minecraft:magma_block"
        | "minecraft:magma_cream" => Some((8.0, [1.0, 0.35, 0.2], 0.80)),
        "minecraft:beacon" | "minecraft:conduit" | "minecraft:nether_star" => {
            Some((14.0, [0.85, 0.96, 1.0], 1.0))
        }
        _ => None,
    }
}

fn stack_light_spec(
    stack: &ItemStackSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<(f32, [f32; 3], f32)> {
    let runtime = item_runtime?;
    let item_id = stack.item_id?;
    if stack.count <= 0 {
        return None;
    }
    let resource_id = runtime.item_resource_id(item_id)?;
    item_light_spec(resource_id)
}

fn best_light(
    a: Option<(f32, [f32; 3], f32)>,
    b: Option<(f32, [f32; 3], f32)>,
) -> Option<(f32, [f32; 3], f32)> {
    match (a, b) {
        (Some(la), Some(lb)) => {
            if la.0 * la.2 >= lb.0 * lb.2 {
                Some(la)
            } else {
                Some(lb)
            }
        }
        (Some(la), None) => Some(la),
        (None, Some(lb)) => Some(lb),
        (None, None) => None,
    }
}

fn dist_sq(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
pub(crate) fn extract_dynamic_point_lights(
    mode: DynamicLightsMode,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_instances: &[EntityModelInstance],
    camera_pose: Option<CameraPose>,
) -> Vec<DynamicPointLight> {
    extract_dynamic_point_lights_with_occluded(
        mode,
        world,
        item_runtime,
        entity_instances,
        &[],
        camera_pose,
    )
}

pub(crate) fn extract_dynamic_point_lights_with_occluded(
    mode: DynamicLightsMode,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_instances: &[EntityModelInstance],
    occluded_candidates: &[EntityRenderCandidateState],
    camera_pose: Option<CameraPose>,
) -> Vec<DynamicPointLight> {
    if !mode.is_enabled() {
        return Vec::new();
    }
    let camera_eye = camera_pose.map_or([0.0; 3], |pose| {
        [
            f64::from(pose.position[0]),
            f64::from(pose.position[1] + pose.eye_height),
            f64::from(pose.position[2]),
        ]
    });
    let mut candidates: Vec<(f64, DynamicPointLight)> = Vec::new();

    if let Some(pose) = camera_pose {
        let hand_light = best_light(
            world
                .inventory()
                .hotbar_item_states()
                .get(usize::from(
                    world.local_player().selected_hotbar_slot.min(8),
                ))
                .and_then(|slot| slot.as_ref())
                .and_then(|hotbar| stack_light_spec(hotbar.item, item_runtime)),
            world
                .inventory()
                .offhand_item_state()
                .and_then(|hotbar| stack_light_spec(hotbar.item, item_runtime)),
        );
        let fire_light =
            world
                .local_player_is_on_fire()
                .then_some((12.0_f32, [1.0_f32, 0.55, 0.2], 0.95_f32));
        if let Some((radius, color, intensity)) = best_light(hand_light, fire_light) {
            let pos = [
                f64::from(pose.position[0]),
                f64::from(pose.position[1]) + f64::from(pose.eye_height) * 0.65,
                f64::from(pose.position[2]),
            ];
            candidates.push((
                dist_sq(pos, camera_eye),
                DynamicPointLight {
                    position: pos,
                    radius,
                    color,
                    intensity,
                },
            ));
        }
    }

    let local_id = world.local_player_id();
    for instance in entity_instances {
        if Some(instance.entity_id) == local_id {
            continue;
        }
        let hand_light = best_light(
            world
                .held_item(instance.entity_id, false)
                .and_then(|stack| stack_light_spec(stack, item_runtime)),
            world
                .held_item(instance.entity_id, true)
                .and_then(|stack| stack_light_spec(stack, item_runtime)),
        );
        let entity_type_id = world.entity_type_id(instance.entity_id);
        let mod_light_rgb = entity_type_id
            .and_then(|type_id| world.content_registry().entity_by_id(type_id))
            .and_then(|entry| entry.dynamic_light_rgb);
        let inherent_light = if let Some(rgb) = mod_light_rgb {
            let anim_state = match instance.kind {
                EntityModelKind::DataDriven { anim_state, .. } => anim_state,
                _ => 0,
            };
            let intensity = if anim_state >= 1 { 1.0 } else { 0.85 };
            Some((15.0_f32, rgb, intensity))
        } else if instance.render_state.flame_dimensions.is_some()
            || matches!(
                instance.kind,
                EntityModelKind::Blaze | EntityModelKind::MagmaCube { .. }
            )
        {
            Some((13.0_f32, [1.0_f32, 0.58, 0.22], 0.95_f32))
        } else if let (None, EntityModelKind::DataDriven { anim_state, .. }) =
            (entity_type_id, instance.kind)
        {
            let intensity = if anim_state >= 1 { 1.0 } else { 0.85 };
            Some((15.0_f32, [1.0_f32, 0.52, 0.18], intensity))
        } else if matches!(instance.kind, EntityModelKind::Squid { glow: true, .. }) {
            Some((9.0_f32, [0.4_f32, 0.9, 0.85], 0.75_f32))
        } else if matches!(
            instance.kind,
            EntityModelKind::Arrow {
                texture: ArrowModelTexture::Spectral
            }
        ) {
            Some((7.0_f32, [1.0_f32, 0.9, 0.4], 0.70_f32))
        } else {
            None
        };

        if let Some((radius, color, intensity)) = best_light(hand_light, inherent_light) {
            let pos = [
                f64::from(instance.position[0]),
                f64::from(instance.position[1]) + 1.0,
                f64::from(instance.position[2]),
            ];
            candidates.push((
                dist_sq(pos, camera_eye),
                DynamicPointLight {
                    position: pos,
                    radius,
                    color,
                    intensity,
                },
            ));
        }
    }

    for candidate in occluded_candidates {
        if Some(candidate.entity_id) == local_id {
            continue;
        }
        let hand_light = best_light(
            world
                .held_item(candidate.entity_id, false)
                .and_then(|stack| stack_light_spec(stack, item_runtime)),
            world
                .held_item(candidate.entity_id, true)
                .and_then(|stack| stack_light_spec(stack, item_runtime)),
        );
        let mod_light_rgb = world
            .content_registry()
            .entity_by_id(candidate.entity_type_id)
            .and_then(|entry| entry.dynamic_light_rgb);
        let resource_id =
            vanilla_entity_type(candidate.entity_type_id).map(|entry| entry.resource_id);
        let inherent_light = if let Some(rgb) = mod_light_rgb {
            Some((15.0_f32, rgb, 0.90_f32))
        } else if world.entity_is_on_fire(candidate.entity_id)
            || matches!(
                resource_id,
                Some("minecraft:blaze" | "minecraft:magma_cube")
            )
        {
            Some((13.0_f32, [1.0_f32, 0.58, 0.22], 0.95_f32))
        } else if matches!(resource_id, Some("minecraft:glow_squid")) {
            Some((9.0_f32, [0.4_f32, 0.9, 0.85], 0.75_f32))
        } else if matches!(resource_id, Some("minecraft:spectral_arrow")) {
            Some((7.0_f32, [1.0_f32, 0.9, 0.4], 0.70_f32))
        } else {
            None
        };
        if let Some((radius, color, intensity)) = best_light(hand_light, inherent_light) {
            let pos = [
                candidate.position.x,
                candidate.position.y + 1.0,
                candidate.position.z,
            ];
            candidates.push((
                dist_sq(pos, camera_eye),
                DynamicPointLight {
                    position: pos,
                    radius,
                    color,
                    intensity,
                },
            ));
        }
    }

    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    candidates.truncate(MAX_DYNAMIC_POINT_LIGHTS);
    candidates.into_iter().map(|(_, light)| light).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_render::{EntityModelInstance, EntityRenderState};

    #[test]
    fn dynamic_lights_off_mode_fast_returns_empty() {
        let world = WorldStore::new();
        let mut blaze = EntityModelInstance::new(42, EntityModelKind::Blaze, [4.0, 64.0, 4.0], 0.0);
        blaze.render_state = EntityRenderState::default();
        let lights =
            extract_dynamic_point_lights(DynamicLightsMode::Off, &world, None, &[blaze], None);
        assert!(lights.is_empty());
    }

    #[test]
    fn dynamic_lights_emits_closest_lights_for_fire_blaze_and_data_driven_boss() {
        let world = WorldStore::new();
        let pose = CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        };
        let far_blaze = EntityModelInstance::new(1, EntityModelKind::Blaze, [30.0, 64.0, 0.0], 0.0);
        let near_boss = EntityModelInstance::new(
            2,
            EntityModelKind::DataDriven {
                rig_id: 0,
                anim_state: 1,
            },
            [5.0, 64.0, 0.0],
            0.0,
        );
        let burning_zombie = EntityModelInstance::new(
            3,
            EntityModelKind::Zombie { baby: false },
            [12.0, 64.0, 0.0],
            0.0,
        )
        .with(|state| state.flame_dimensions = Some([0.6, 1.8]));

        let lights = extract_dynamic_point_lights(
            DynamicLightsMode::Fancy,
            &world,
            None,
            &[far_blaze, burning_zombie, near_boss],
            Some(pose),
        );
        assert_eq!(lights.len(), 3);
        assert_eq!(lights[0].position, [5.0, 65.0, 0.0]);
        assert_eq!(lights[0].intensity, 1.0);
        assert_eq!(lights[1].position, [12.0, 65.0, 0.0]);
        assert_eq!(lights[2].position, [30.0, 65.0, 0.0]);
    }

    #[test]
    fn dynamic_lights_respects_content_registry_entity_color() {
        use bbb_protocol::content_registry::{
            ContentRegistryBuilder, ModEntityRegistration, BBB_CONTENT_REGISTRY_CHANNEL,
        };
        let mut builder = ContentRegistryBuilder::new();
        builder.register_entity(ModEntityRegistration {
            resource_id: "bbb_demo:azure_wisp".to_string(),
            dynamic_light_rgb: Some([0.2, 0.65, 1.0]),
            ..Default::default()
        });
        builder.register_entity(ModEntityRegistration {
            resource_id: "bbb_demo:unlit_crawler".to_string(),
            dynamic_light_rgb: None,
            ..Default::default()
        });
        let registry = builder.freeze().expect("freeze registry");
        let wisp_type_id = registry
            .entity_by_resource_id("bbb_demo:azure_wisp")
            .unwrap()
            .id;
        let unlit_type_id = registry
            .entity_by_resource_id("bbb_demo:unlit_crawler")
            .unwrap()
            .id;

        let mut world = WorldStore::new();
        world.apply_custom_payload(bbb_protocol::packets::CustomPayload {
            id: BBB_CONTENT_REGISTRY_CHANNEL.to_string(),
            payload: bbb_protocol::packets::CustomPayloadBody::Unknown {
                raw_payload: registry.encode_wire(),
            },
        });
        world.apply_add_entity(bbb_protocol::packets::AddEntity {
            id: 201,
            uuid: uuid::Uuid::from_u128(201),
            entity_type_id: wisp_type_id,
            position: bbb_protocol::packets::Vec3d {
                x: 2.0,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: bbb_protocol::packets::Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world.apply_add_entity(bbb_protocol::packets::AddEntity {
            id: 202,
            uuid: uuid::Uuid::from_u128(202),
            entity_type_id: unlit_type_id,
            position: bbb_protocol::packets::Vec3d {
                x: 3.0,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: bbb_protocol::packets::Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });

        let wisp = EntityModelInstance::new(
            201,
            EntityModelKind::DataDriven {
                rig_id: 0,
                anim_state: 0,
            },
            [2.0, 64.0, 0.0],
            0.0,
        );
        let unlit = EntityModelInstance::new(
            202,
            EntityModelKind::DataDriven {
                rig_id: 0,
                anim_state: 0,
            },
            [3.0, 64.0, 0.0],
            0.0,
        );
        let lights = extract_dynamic_point_lights(
            DynamicLightsMode::Fancy,
            &world,
            None,
            &[wisp, unlit],
            None,
        );
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].color, [0.2, 0.65, 1.0]);
        assert_eq!(lights[0].intensity, 0.85);
    }

    #[test]
    fn occluded_emissive_candidates_still_contribute_dynamic_lights() {
        let mut world = WorldStore::new();
        let blaze_type =
            bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id("minecraft:blaze")
                .expect("vanilla blaze type");
        world.apply_add_entity(bbb_protocol::packets::AddEntity {
            id: 301,
            uuid: uuid::Uuid::from_u128(301),
            entity_type_id: blaze_type,
            position: bbb_protocol::packets::Vec3d {
                x: 4.0,
                y: 64.0,
                z: 8.0,
            },
            delta_movement: bbb_protocol::packets::Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        let candidate = world
            .entity_render_candidates_at_partial_tick(1.0)
            .into_iter()
            .find(|c| c.entity_id == 301)
            .expect("blaze candidate");
        let lights = extract_dynamic_point_lights_with_occluded(
            DynamicLightsMode::Fancy,
            &world,
            None,
            &[],
            &[candidate],
            None,
        );
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].position, [4.0, 65.0, 8.0]);
        assert_eq!(lights[0].radius, 13.0);
    }
}
