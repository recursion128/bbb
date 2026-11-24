//! Fast-path Vanilla entity facts paired with session-registered modded entity specs.

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TYPES;
use bbb_protocol::content_registry::{ContentRegistry, ModEntityTypeSpec};
use bbb_protocol::entity_types::{
    vanilla_entity_type, vanilla_entity_type_is_fire_immune, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::{AttributeSnapshot, EntityDataValue};

use super::dimensions::{
    bounds_height, vanilla_bounds_for_entity_data, vanilla_eye_height_for_entity_data,
    vanilla_eye_height_for_model_source_bounds, vanilla_model_source_bounds_for_entity_data,
    vanilla_pick_bounds_for_entity_data, vanilla_render_scale, vanilla_scale_attribute,
};
use super::schema::{
    vanilla_entity_metadata_accessor_for_entity_type_and_data_id,
    vanilla_entity_metadata_living_entity_accessors_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_type_interpolation_steps, vanilla_entity_metadata_type_is_living,
    vanilla_entity_metadata_type_is_mob, vanilla_entity_type_has_attribute,
    VanillaLivingEntityMetadataAccessors, VanillaMobMetadataAccessors,
};
use super::{vanilla_living_entity_type, EntityClientAnimationState, EntityPickBoundsState};
use crate::PaintingVariantClientProjection;

#[inline]
pub(crate) fn mod_entity_spec(
    entity_type_id: i32,
    content_registry: &ContentRegistry,
) -> Option<&ModEntityTypeSpec> {
    if content_registry.is_vanilla_only() || vanilla_entity_type(entity_type_id).is_some() {
        return None;
    }
    content_registry.entity_by_id(entity_type_id)
}

#[inline]
pub(crate) fn entity_is_living(entity_type_id: i32, content_registry: &ContentRegistry) -> bool {
    vanilla_living_entity_type(entity_type_id)
        || mod_entity_spec(entity_type_id, content_registry).is_some_and(|spec| spec.is_living)
}

#[inline]
pub(crate) fn entity_metadata_is_living(
    entity_type_id: i32,
    content_registry: &ContentRegistry,
) -> bool {
    vanilla_entity_metadata_type_is_living(entity_type_id)
        || mod_entity_spec(entity_type_id, content_registry).is_some_and(|spec| spec.is_living)
}

#[inline]
pub(crate) fn entity_is_mob(entity_type_id: i32, content_registry: &ContentRegistry) -> bool {
    vanilla_entity_metadata_type_is_mob(entity_type_id)
        || mod_entity_spec(entity_type_id, content_registry).is_some_and(|spec| spec.is_mob)
}

#[inline]
pub(crate) fn entity_is_fire_immune(
    entity_type_id: i32,
    content_registry: &ContentRegistry,
) -> bool {
    vanilla_entity_type_is_fire_immune(entity_type_id)
        || mod_entity_spec(entity_type_id, content_registry).is_some_and(|spec| spec.fire_immune)
}

#[inline]
pub(crate) fn entity_living_accessors(
    entity_type_id: i32,
    content_registry: &ContentRegistry,
) -> Option<VanillaLivingEntityMetadataAccessors> {
    vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id).or_else(|| {
        mod_entity_spec(entity_type_id, content_registry)
            .filter(|spec| spec.is_living)
            .and_then(|_| {
                vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                )
            })
    })
}

#[inline]
pub(super) fn entity_mob_accessors(
    entity_type_id: i32,
    content_registry: &ContentRegistry,
) -> Option<VanillaMobMetadataAccessors> {
    vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id).or_else(|| {
        mod_entity_spec(entity_type_id, content_registry)
            .filter(|spec| spec.is_mob)
            .and_then(|_| {
                vanilla_entity_metadata_mob_accessors_for_entity_type(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
            })
    })
}

#[inline]
pub(super) fn entity_interpolation_steps(
    entity_type_id: i32,
    minecart_improvements_enabled: bool,
    content_registry: &ContentRegistry,
) -> Option<u8> {
    vanilla_entity_metadata_type_interpolation_steps(entity_type_id, minecart_improvements_enabled)
        .or_else(|| mod_entity_spec(entity_type_id, content_registry).map(|_| 3))
}

#[inline]
pub(super) fn entity_has_attribute(
    entity_type_id: i32,
    attribute_id: i32,
    content_registry: &ContentRegistry,
) -> bool {
    if vanilla_entity_type(entity_type_id).is_some() {
        return vanilla_entity_type_has_attribute(entity_type_id, attribute_id);
    }
    attribute_id >= 0
        && (attribute_id as usize) < VANILLA_ATTRIBUTE_TYPES.len()
        && mod_entity_spec(entity_type_id, content_registry).is_some_and(|spec| spec.is_living)
}

pub(super) fn validate_entity_data_field(
    entity_type_id: i32,
    data_id: u8,
    serializer_id: i32,
    content_registry: &ContentRegistry,
) -> Result<&'static str, String> {
    if vanilla_entity_type(entity_type_id).is_some() {
        let Some(accessor) =
            vanilla_entity_metadata_accessor_for_entity_type_and_data_id(entity_type_id, data_id)
        else {
            return Err(format!("unknown entity data field {data_id}"));
        };
        if accessor.serializer_id != serializer_id {
            return Err(format!(
                "invalid entity data item type for field {} (id {data_id}): expected serializer {} ({}), received {serializer_id}",
                accessor.java_name, accessor.serializer_id, accessor.serializer_name
            ));
        }
        return Ok(accessor.java_name);
    }

    let Some(spec) = mod_entity_spec(entity_type_id, content_registry) else {
        return Err(format!("unknown modded entity type {entity_type_id}"));
    };

    let base_allowed = data_id <= 7
        || (spec.is_living && (8..=14).contains(&data_id))
        || (spec.is_mob && data_id == 15);
    if base_allowed {
        let Some(accessor) = vanilla_entity_metadata_accessor_for_entity_type_and_data_id(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            data_id,
        ) else {
            return Err(format!("unknown base entity data field {data_id}"));
        };
        if accessor.serializer_id != serializer_id {
            return Err(format!(
                "invalid entity data item type for field {} (id {data_id}): expected serializer {} ({}), received {serializer_id}",
                accessor.java_name, accessor.serializer_id, accessor.serializer_name
            ));
        }
        return Ok(accessor.java_name);
    }

    let Some(field) = spec
        .synched_fields
        .iter()
        .find(|field| field.data_id == data_id)
    else {
        return Err(format!("undeclared modded entity data field {data_id}"));
    };
    if field.serializer_id != serializer_id {
        return Err(format!(
            "invalid modded entity data serializer for field {data_id}: expected {}, received {serializer_id}",
            field.serializer_id
        ));
    }
    Ok("MOD_SYNCHED_FIELD")
}

fn mod_scaled_bounds(
    spec: &ModEntityTypeSpec,
    attributes: &[AttributeSnapshot],
) -> EntityPickBoundsState {
    let bounds = EntityPickBoundsState::from_base_size(spec.width, spec.height, 0.0);
    if !spec.is_living {
        return bounds;
    }
    match vanilla_scale_attribute(attributes) {
        Some(scale) => bounds.scale_dimensions(scale),
        None => bounds,
    }
}

pub(super) fn pick_bounds_for_entity_data(
    entity_type_id: i32,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    painting_variant: Option<&PaintingVariantClientProjection>,
    content_registry: &ContentRegistry,
) -> Option<EntityPickBoundsState> {
    if vanilla_entity_type(entity_type_id).is_some() {
        return vanilla_pick_bounds_for_entity_data(
            entity_type_id,
            add_entity_data,
            data_values,
            attributes,
            client_animations,
            painting_variant,
        );
    }
    mod_entity_spec(entity_type_id, content_registry)
        .map(|spec| mod_scaled_bounds(spec, attributes))
}

pub(super) fn bounds_for_entity_data(
    entity_type_id: i32,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    painting_variant: Option<&PaintingVariantClientProjection>,
    content_registry: &ContentRegistry,
) -> Option<EntityPickBoundsState> {
    if vanilla_entity_type(entity_type_id).is_some() {
        return vanilla_bounds_for_entity_data(
            entity_type_id,
            add_entity_data,
            data_values,
            attributes,
            client_animations,
            painting_variant,
        );
    }
    mod_entity_spec(entity_type_id, content_registry)
        .map(|spec| mod_scaled_bounds(spec, attributes))
}

pub(super) fn model_source_bounds_for_entity_data(
    entity_type_id: i32,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    painting_variant: Option<&PaintingVariantClientProjection>,
    content_registry: &ContentRegistry,
) -> Option<EntityPickBoundsState> {
    if vanilla_entity_type(entity_type_id).is_some() {
        return vanilla_model_source_bounds_for_entity_data(
            entity_type_id,
            add_entity_data,
            data_values,
            attributes,
            client_animations,
            painting_variant,
        );
    }
    mod_entity_spec(entity_type_id, content_registry)
        .map(|spec| mod_scaled_bounds(spec, attributes))
}

pub(super) fn eye_height_for_entity_data(
    entity_type_id: i32,
    add_entity_data: i32,
    data_values: &[EntityDataValue],
    attributes: &[AttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    painting_variant: Option<&PaintingVariantClientProjection>,
    content_registry: &ContentRegistry,
) -> Option<f32> {
    if vanilla_entity_type(entity_type_id).is_some() {
        return vanilla_eye_height_for_entity_data(
            entity_type_id,
            add_entity_data,
            data_values,
            attributes,
            client_animations,
            painting_variant,
        );
    }
    let bounds = model_source_bounds_for_entity_data(
        entity_type_id,
        add_entity_data,
        data_values,
        attributes,
        client_animations,
        painting_variant,
        content_registry,
    );
    eye_height_for_model_source_bounds(entity_type_id, data_values, bounds, content_registry)
}

pub(super) fn eye_height_for_model_source_bounds(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    bounds: Option<EntityPickBoundsState>,
    content_registry: &ContentRegistry,
) -> Option<f32> {
    if vanilla_entity_type(entity_type_id).is_some() {
        return vanilla_eye_height_for_model_source_bounds(entity_type_id, data_values, bounds);
    }
    let spec = mod_entity_spec(entity_type_id, content_registry)?;
    let current_height = bounds_height(bounds?);
    if spec.height > 0.0 {
        Some(spec.eye_height * current_height / spec.height)
    } else {
        Some(spec.eye_height)
    }
}

pub(super) fn render_scale_for_entity(
    entity_type_id: i32,
    attributes: &[AttributeSnapshot],
    content_registry: &ContentRegistry,
) -> f32 {
    if vanilla_entity_type(entity_type_id).is_some() {
        if vanilla_entity_metadata_type_is_living(entity_type_id) {
            return vanilla_render_scale(entity_type_id, attributes);
        }
        return 1.0;
    }
    if entity_metadata_is_living(entity_type_id, content_registry) {
        vanilla_scale_attribute(attributes).unwrap_or(1.0)
    } else {
        1.0
    }
}

#[inline]
pub(super) fn is_non_dragon_living(
    entity_type_id: i32,
    content_registry: &ContentRegistry,
) -> bool {
    entity_type_id != VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
        && entity_is_living(entity_type_id, content_registry)
}

#[cfg(test)]
mod tests {
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID;
    use bbb_protocol::content_registry::{
        ContentRegistryBuilder, ModEntityRegistration, ModSynchedFieldSpec,
        BBB_CONTENT_REGISTRY_CHANNEL,
    };
    use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;
    use bbb_protocol::packets::entities::MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID;
    use bbb_protocol::packets::{
        AddEntity, AttributeSnapshot, CustomPayload, CustomPayloadBody, EntityDataValue,
        EntityDataValueKind, RegistryTags, SetEntityData, TagNetworkPayload, UpdateAttributes,
        UpdateTags, Vec3d,
    };
    use uuid::Uuid;

    use crate::WorldStore;

    #[test]
    fn content_registry_admits_modded_mob_metadata_tags_and_render_source() {
        let mut builder = ContentRegistryBuilder::new();
        builder.register_entity(ModEntityRegistration {
            resource_id: "bbb_demo:inferno_warden".to_string(),
            width: 1.2,
            height: 3.2,
            eye_height: 2.8,
            default_attributes: vec![(VANILLA_ATTRIBUTE_MAX_HEALTH_ID, 250.0)],
            synched_fields: vec![
                ModSynchedFieldSpec {
                    data_id: 16,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                },
                ModSynchedFieldSpec {
                    data_id: 17,
                    serializer_id: MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID,
                },
            ],
            ..Default::default()
        });
        let reg = builder.freeze().expect("valid modded registry");
        let entity_type_id = reg
            .entity_by_resource_id("bbb_demo:inferno_warden")
            .unwrap()
            .id;

        let mut world = WorldStore::new();
        world.begin_connection_session(false);
        world.begin_registry_configuration();
        world.apply_custom_payload(CustomPayload {
            id: BBB_CONTENT_REGISTRY_CHANNEL.to_string(),
            payload: CustomPayloadBody::Unknown {
                raw_payload: reg.encode_wire(),
            },
        });
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:entity_type".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "bbb_demo:bosses".to_string(),
                    entries: vec![entity_type_id],
                }],
            }],
        });
        world.finish_registry_configuration().unwrap();
        let boss_name = bbb_protocol::component_styled_literal_nbt(
            "Inferno Warden",
            &bbb_protocol::ComponentStyle {
                hover_event: Some(bbb_protocol::ComponentHoverEvent::ShowEntity {
                    id: "bbb_demo:inferno_warden".into(),
                    uuid: bbb_protocol::component_hover_uuid_nbt(Uuid::from_u128(0x1234)),
                    name: None,
                    name_component: None,
                }),
                ..Default::default()
            },
        )
        .unwrap();
        let content = bbb_protocol::RichTextComponent::try_from_component_nbt_with_registry(
            boss_name.clone(),
            &reg,
        )
        .unwrap();
        world.apply_system_chat(bbb_protocol::packets::SystemChat::from_component(
            content, false,
        ));
        assert_eq!(
            world.system_chat().unwrap().content_component.as_ref(),
            Some(&boss_name)
        );
        assert!(world.registries().holder_is_in_tag(
            "minecraft:entity_type",
            "bbb_demo:bosses",
            entity_type_id,
        ));

        assert!(world
            .apply_add_entity_packet(AddEntity {
                id: 77,
                uuid: Uuid::from_u128(0x1234),
                entity_type_id,
                position: Vec3d {
                    x: 4.0,
                    y: 64.0,
                    z: 8.0,
                },
                delta_movement: Vec3d::default(),
                x_rot: 0.0,
                y_rot: 90.0,
                y_head_rot: 90.0,
                data: 0,
            })
            .is_some());

        assert!(world.apply_update_attributes(UpdateAttributes {
            entity_id: 77,
            attributes: vec![AttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                base: 250.0,
                modifiers: Vec::new(),
            }],
        }));

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 77,
            values: vec![
                EntityDataValue {
                    data_id: 16,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(1),
                },
                EntityDataValue {
                    data_id: 17,
                    serializer_id: MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID,
                    value: EntityDataValueKind::ModCustom(vec![9, 8, 7]),
                },
            ],
        }));

        let bounds = world.entity_pick_bounds(77).expect("modded pick bounds");
        assert_eq!(bounds.max[1], 3.2);
        let candidates = world.entity_render_candidates_at_partial_tick(1.0);
        assert_eq!(candidates.len(), 1);
        let source = world
            .entity_model_source_at_partial_tick(77, 1.0)
            .expect("modded model source");
        assert_eq!(source.entity_type_id, entity_type_id);
        assert_eq!(source.data_values.len(), 2);
        assert_eq!(
            source.data_values[1].value,
            EntityDataValueKind::ModCustom(vec![9, 8, 7])
        );
    }

    #[test]
    fn content_registry_survives_login_and_plumbs_mod_blocks_items_particles_and_living_mobs() {
        use bbb_protocol::content_registry::{
            ModBlockRegistration, ModItemRegistration, ModParticleRegistration,
        };
        use bbb_protocol::packets::{
            CommonPlayerSpawnInfo, DamageEvent, HurtAnimation, InteractionHand, ItemStackSummary,
            MobEffectFlags, PlayLogin, RemoveMobEffect, SetPlayerInventory, UpdateMobEffect,
        };

        let mut builder = ContentRegistryBuilder::new();
        builder.register_entity(ModEntityRegistration {
            resource_id: "bbb_demo:boss_mob".to_string(),
            width: 1.4,
            height: 3.2,
            eye_height: 2.8,
            is_living: true,
            is_mob: true,
            can_summon: true,
            ..Default::default()
        });
        builder.register_item(ModItemRegistration {
            resource_id: "bbb_demo:relic".to_string(),
            max_stack_size: 16,
            is_block_item: false,
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "bbb_demo:ghost_lantern".to_string(),
            state_count: 1,
            light_emission: 15,
            destroy_speed: 1.0,
            full_cube_collision: false,
            dynamic_light_rgb: None,
        });
        builder.register_particle(ModParticleRegistration {
            resource_id: "bbb_demo:spark".to_string(),
            override_limiter: true,
        });
        let reg = builder.freeze().expect("valid modded registry");
        let mod_entity_id = reg.entity_by_resource_id("bbb_demo:boss_mob").unwrap().id;
        let mod_item_id = reg.item_by_resource_id("bbb_demo:relic").unwrap().id;
        let mod_block_state_id = reg
            .block_by_resource_id("bbb_demo:ghost_lantern")
            .unwrap()
            .first_state_id;
        let mod_particle_id = reg.particle_by_resource_id("bbb_demo:spark").unwrap().id;

        let mut world = WorldStore::new();
        world.begin_connection_session(false);
        world.begin_registry_configuration();
        world.apply_custom_payload(CustomPayload {
            id: BBB_CONTENT_REGISTRY_CHANNEL.to_string(),
            payload: CustomPayloadBody::Unknown {
                raw_payload: reg.encode_wire(),
            },
        });
        world.finish_registry_configuration().unwrap();

        // R14-05: Login clears level-bound entities while preserving EntityStore::content_registry.
        world.apply_login(&PlayLogin {
            player_id: 1,
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
                seed: 1,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });

        // Spawning mod entity AFTER Login still resolves custom bounds & living predicates (R14-05, R13-04).
        assert!(world
            .apply_add_entity_packet(AddEntity {
                id: 88,
                uuid: Uuid::from_u128(0x88),
                entity_type_id: mod_entity_id,
                position: Vec3d {
                    x: 2.0,
                    y: 64.0,
                    z: 2.0,
                },
                delta_movement: Vec3d::default(),
                x_rot: 0.0,
                y_rot: 0.0,
                y_head_rot: 0.0,
                data: 0,
            })
            .is_some());
        assert_eq!(world.entity_pick_bounds(88).unwrap().max[1], 3.2);
        assert!(world.apply_damage_event(DamageEvent {
            entity_id: 88,
            source_type_id: 0,
            source_cause_id: 0,
            source_direct_id: 0,
            source_position: None,
        }));
        assert!(world.apply_hurt_animation(HurtAnimation { id: 88, yaw: 0.0 }));
        assert!(world.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 88,
            effect_id: 1,
            amplifier: 0,
            duration_ticks: 100,
            flags: MobEffectFlags {
                raw: 0x06,
                ambient: false,
                visible: true,
                show_icon: true,
                blend: false,
            },
        }));
        assert!(world.apply_remove_mob_effect(RemoveMobEffect {
            entity_id: 88,
            effect_id: 1,
        }));

        // R3-23: custom item max_stack_size is honored.
        assert_eq!(
            world
                .item_profiles()
                .item_max_stack_size_for_protocol_id(mod_item_id),
            16
        );
        let mod_stack = ItemStackSummary {
            count: 4,
            item_id: Some(mod_item_id),
            component_patch: Default::default(),
        };
        assert_eq!(
            world.item_profiles().item_stack_max_stack_size(&mod_stack),
            16
        );

        // R3-18: custom ContentRegistry particle resolves and round-trips protocol_id.
        let particle_kind =
            crate::particles::ParticleKind::from_protocol_id_with_registry(mod_particle_id, &reg)
                .expect("custom particle kind");
        assert!(particle_kind.is_custom());
        assert_eq!(particle_kind.protocol_id(), Some(mod_particle_id));
        assert!(particle_kind.override_limiter_with_registry(&reg));
        assert_eq!(
            crate::particles::ParticleKind::from_resource_id_with_registry("bbb_demo:spark", &reg),
            Some(particle_kind)
        );

        // R9-13: stepping on a custom mod block never panics.
        let step_sound =
            world.record_entity_client_movement_sound(crate::EntityClientMovementSoundState {
                entity_id: 1,
                kind: crate::EntityClientMovementSoundKind::PlayerBlockStep {
                    block_state_id: mod_block_state_id,
                    muffled: false,
                },
                position: crate::EntityVec3::default(),
            });
        assert!(step_sound.sound.location.is_some());

        // R14-07: custom item in hand is enabled and predicts Pass (not UnknownItemType).
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: mod_stack,
        });
        assert!(world.local_item_in_hand_is_enabled(InteractionHand::MainHand));
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            crate::LocalItemUsePrediction::Result(crate::LocalInteractionResult::Pass)
        );

        // R15-03: persistent_item_stack_from_nbt decodes custom mod items.
        let nbt = bbb_protocol::NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "id".into(),
                value: bbb_protocol::NbtSummaryValue::string("bbb_demo:relic"),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "count".into(),
                value: bbb_protocol::NbtSummaryValue::Int(3),
            },
        ]);
        let decoded = world.persistent_item_stack_from_nbt(&nbt).unwrap();
        assert_eq!(decoded.item_id, Some(mod_item_id));
        assert_eq!(decoded.count, 3);
    }
}
