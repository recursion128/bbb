use super::*;
use crate::entity::BoatData;
use crate::world::simulation_level::LevelCtx;

pub(super) struct EntityRecordFields<'a> {
    pub(super) resource_id: &'a str,
    pub(super) uuid: Uuid,
    pub(super) position: [f64; 3],
    pub(super) entries: &'a [NbtSummaryEntry],
    pub(super) command_existing: Option<&'a Entity>,
}

pub(super) fn read_entity_record(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    context: Option<EntityNbtReadContext<'_>>,
    registry_tag_membership: &mut impl FnMut(&str, &str, &str) -> bool,
    items: crate::entity::item::ItemResolver<'_>,
) -> Result<i32, EntitySaveError> {
    read_entity_record_fields(
        store,
        EntityRecordFields {
            resource_id: record.id(),
            uuid: record.uuid(),
            position: record.position(),
            entries: record.raw(),
            command_existing: None,
        },
        None,
        context,
        registry_tag_membership,
        items,
    )
}

pub(super) fn read_entity_record_with_post_load(
    store: &mut EntityStore,
    record: Java26EntityRecordRef<'_>,
    context: EntityNbtReadContext<'_>,
    registry_tag_membership: &mut impl FnMut(&str, &str, &str) -> bool,
    items: crate::entity::item::ItemResolver<'_>,
    allocations: &mut usize,
) -> Result<crate::entity::store::EntityRestoreOutcome, EntitySaveError> {
    read_entity_record_fields_for_publication(
        store,
        EntityRecordFields {
            resource_id: record.id(),
            uuid: record.uuid(),
            position: record.position(),
            entries: record.raw(),
            command_existing: None,
        },
        None,
        Some(context),
        registry_tag_membership,
        items,
        Some(allocations),
    )
}

pub(super) fn read_entity_record_fields(
    store: &mut EntityStore,
    fields: EntityRecordFields<'_>,
    level: Option<&LevelCtx<'_, '_>>,
    context: Option<EntityNbtReadContext<'_>>,
    registry_tag_membership: &mut impl FnMut(&str, &str, &str) -> bool,
    items: crate::entity::item::ItemResolver<'_>,
) -> Result<i32, EntitySaveError> {
    match read_entity_record_fields_for_publication(
        store,
        fields,
        level,
        context,
        registry_tag_membership,
        items,
        None,
    )? {
        crate::entity::store::EntityRestoreOutcome::Published(id) => Ok(id),
        crate::entity::store::EntityRestoreOutcome::RejectedRemoved
        | crate::entity::store::EntityRestoreOutcome::RejectedDuplicateUuid => Err(field_error(
            "id",
            "entity was refused by its post-load consumer",
        )),
    }
}

fn read_entity_record_fields_for_publication(
    store: &mut EntityStore,
    fields: EntityRecordFields<'_>,
    level: Option<&LevelCtx<'_, '_>>,
    context: Option<EntityNbtReadContext<'_>>,
    registry_tag_membership: &mut impl FnMut(&str, &str, &str) -> bool,
    items: crate::entity::item::ItemResolver<'_>,
    post_load_allocations: Option<&mut usize>,
) -> Result<crate::entity::store::EntityRestoreOutcome, EntitySaveError> {
    let EntityRecordFields {
        resource_id,
        uuid,
        position,
        entries,
        command_existing,
    } = fields;
    let content_registry = if !store.plugins().content_registry().is_vanilla_only() {
        store.plugins().content_registry()
    } else {
        items.content_registry()
    };
    let entity_type_id = vanilla_entity_type_id_for_resource_id(resource_id)
        .or_else(|| {
            content_registry
                .entity_by_resource_id(resource_id)
                .map(|spec| spec.id)
        })
        .ok_or_else(|| EntitySaveError::UnsupportedEntityType(resource_id.to_string()))?;
    let custom_name = optional_field(entries, "CustomName")
        .cloned()
        .and_then(|component| {
            RichTextComponent::try_from_component_nbt_with_registry(component, content_registry)
                .ok()
        });
    let custom_data = marker::read_save_data(entries).data().to_vec();
    let custom_name_visible = optional_bool(entries, "CustomNameVisible").unwrap_or(false);
    let silent = optional_bool(entries, "Silent").unwrap_or(false);
    let glowing = optional_bool(entries, "Glowing").unwrap_or(false);
    let ticks_frozen = optional_int(entries, "TicksFrozen").unwrap_or(0);
    let has_visual_fire = optional_bool(entries, "HasVisualFire").unwrap_or(false);
    let tags = optional_string_list(entries, "Tags").unwrap_or_default();
    let mut discard_item = false;
    let data = match resource_id {
        "minecraft:falling_block" => EntityData::FallingBlock(read_falling_block(
            entries,
            registry_tag_membership,
            content_registry,
        )),
        "minecraft:item" => {
            let bob_offs = store.random().next_float();
            let data = read_item_entity(entries, bob_offs, items);
            if data.item().is_empty() {
                if command_existing.is_none()
                    && post_load_allocations.is_none()
                    && !context.is_some_and(EntityNbtReadContext::registers_discarded_items)
                {
                    return Err(field_error(
                        "Item",
                        "empty Item needs the caller's removal disposition",
                    ));
                }
                discard_item = true;
            }
            EntityData::Item(data)
        }
        "minecraft:experience_orb" => {
            EntityData::ExperienceOrb(experience_orb::read_save_data(entries))
        }
        "minecraft:tnt" => EntityData::Tnt(read_tnt(entries, content_registry)),
        "minecraft:leash_knot" => EntityData::LeashKnot(crate::entity::LeashKnotData::new(
            crate::entity::containing_block(Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            }),
        )),
        "minecraft:painting" => {
            EntityData::Painting(read_painting(entries, position, command_existing, items))
        }
        "minecraft:item_frame" | "minecraft:glow_item_frame" => {
            let frame = read_item_frame(entries, position, items);
            if resource_id == "minecraft:item_frame" {
                EntityData::ItemFrame(frame)
            } else {
                EntityData::GlowItemFrame(frame)
            }
        }
        "minecraft:armor_stand" => {
            EntityData::ArmorStand(ArmorStandData::new().ok_or_else(|| {
                field_error("id", "no default attributes for minecraft:armor_stand")
            })?)
        }
        "minecraft:marker" => EntityData::Marker(crate::entity::MarkerData::new()),
        "minecraft:interaction" => EntityData::Interaction(interaction::read_save_data(entries)),
        "minecraft:block_display" => EntityData::BlockDisplay(
            display::read_block_display_with_registry(entries, content_registry),
        ),
        "minecraft:item_display" => {
            EntityData::ItemDisplay(display::read_item_display(entries, items))
        }
        "minecraft:text_display" => EntityData::TextDisplay(display::read_text_display(entries)),
        "minecraft:arrow" => EntityData::Arrow(read_arrow(entries, items)),
        "minecraft:spectral_arrow" => {
            EntityData::SpectralArrow(read_spectral_arrow(entries, items))
        }
        "minecraft:trident" => EntityData::Trident(read_trident(entries, items)),
        "minecraft:fishing_bobber" => EntityData::FishingBobber(read_fishing_bobber(entries)),
        "minecraft:snowball" => {
            EntityData::Snowball(read_throwable_item(entries, items, "minecraft:snowball"))
        }
        "minecraft:egg" => EntityData::Egg(read_throwable_item(entries, items, "minecraft:egg")),
        "minecraft:ender_pearl" => {
            EntityData::EnderPearl(read_throwable_item(entries, items, "minecraft:ender_pearl"))
        }
        "minecraft:end_crystal" => EntityData::EndCrystal(end_crystal::read_save_data(
            entries,
            store.random().next_int(100_000),
        )),
        "minecraft:eye_of_ender" => {
            EntityData::EyeOfEnder(eye_of_ender::read_save_data(entries, items))
        }
        "minecraft:experience_bottle" => EntityData::ExperienceBottle(read_throwable_item(
            entries,
            items,
            "minecraft:experience_bottle",
        )),
        "minecraft:firework_rocket" => {
            EntityData::FireworkRocket(read_firework_rocket(entries, items))
        }
        "minecraft:llama_spit" => EntityData::LlamaSpit(read_projectile(entries)),
        "minecraft:evoker_fangs" => EntityData::EvokerFangs(read_evoker_fangs(entries)),
        "minecraft:lightning_bolt" => match command_existing.map(Entity::data) {
            Some(EntityData::LightningBolt(data)) => EntityData::LightningBolt(data.clone()),
            _ => EntityData::LightningBolt(LightningBoltData::create(store.random())),
        },
        "minecraft:fireball" => EntityData::LargeFireball(read_large_fireball(entries, items)),
        "minecraft:small_fireball" => {
            EntityData::SmallFireball(read_small_fireball(entries, items))
        }
        "minecraft:dragon_fireball" => {
            EntityData::DragonFireball(DragonFireballData::new(read_hurting_projectile(entries)))
        }
        "minecraft:wither_skull" => EntityData::WitherSkull(read_wither_skull(entries)),
        "minecraft:wind_charge" => {
            EntityData::WindCharge(WindChargeData::new(read_hurting_projectile(entries), 5))
        }
        "minecraft:breeze_wind_charge" => {
            EntityData::BreezeWindCharge(WindChargeData::new(read_hurting_projectile(entries), 0))
        }
        "minecraft:area_effect_cloud" => {
            EntityData::AreaEffectCloud(area_effect_cloud::read_save_data(entries))
        }
        "minecraft:ominous_item_spawner" => {
            EntityData::OminousItemSpawner(read_ominous_item_spawner(entries, items))
        }
        "minecraft:splash_potion" => EntityData::SplashPotion(read_thrown_potion(
            entries,
            items,
            crate::entity::ThrownPotionKind::Splash,
        )),
        "minecraft:lingering_potion" => EntityData::LingeringPotion(read_thrown_potion(
            entries,
            items,
            crate::entity::ThrownPotionKind::Lingering,
        )),
        "minecraft:shulker_bullet" => EntityData::ShulkerBullet(read_shulker_bullet(entries)),
        _ if boat::is_boat_type(entity_type_id) => EntityData::Boat(boat::read_save_data(
            entries,
            BoatData::for_entity_type(entity_type_id).unwrap(),
        )),
        _ if chest_boat::is_chest_boat_type(entity_type_id) => {
            EntityData::ChestBoat(read_chest_boat(entries, items, entity_type_id))
        }
        "minecraft:minecart" => EntityData::Minecart(minecart::read_save_data_with_registry(
            entries,
            content_registry,
        )),
        "minecraft:furnace_minecart" => {
            EntityData::FurnaceMinecart(minecart::read_furnace_save_data(entries, content_registry))
        }
        "minecraft:chest_minecart" => EntityData::ChestMinecart(read_minecart_container(
            entries,
            MinecartContainerKind::Chest,
            items,
        )),
        "minecraft:hopper_minecart" => EntityData::HopperMinecart(read_minecart_container(
            entries,
            MinecartContainerKind::Hopper,
            items,
        )),
        "minecraft:command_block_minecart" => {
            let minecart = minecart::read_save_data_with_registry(entries, content_registry);
            EntityData::CommandBlockMinecart(
                crate::entity::CommandBlockMinecartData::read_save_data(minecart, entries),
            )
        }
        "minecraft:spawner_minecart" => EntityData::SpawnerMinecart(
            crate::entity::SpawnerMinecartData::read_save_data_with_registry(
                entries,
                content_registry,
            ),
        ),
        "minecraft:tnt_minecart" => {
            let minecart = minecart::read_save_data_with_registry(entries, content_registry);
            EntityData::TntMinecart(crate::entity::TntMinecartData::from_saved(
                minecart, entries,
            ))
        }
        other => {
            let Some(mut data) =
                crate::entity::GeneralLivingData::new_with_plugins(entity_type_id, store.plugins())
            else {
                return Err(EntitySaveError::UnsupportedEntityType(other.to_string()));
            };
            data.read_save(entity_type_id, entries, position, items);
            if let Some(bee) = data.bee_retained_mut() {
                let anger = crate::entity::ai::mob::target::anger::read_saved_fields(
                    entries,
                    store.game_time(),
                );
                bee.set_persistent_anger(anger.end_time, anger.target);
            } else if let Some(anger) = data.neutral_anger_mut() {
                *anger = crate::entity::ai::mob::target::anger::read_saved_fields(
                    entries,
                    store.game_time(),
                );
            }
            let horse = data.horse_phenotype();
            if let Some(inventory) = &mut data.internal_inventory {
                internal_inventory::read_save_data(inventory, entries, horse, items)?;
            }
            EntityData::GeneralLiving(data)
        }
    };
    let motion = optional_vec3(entries, "Motion").unwrap_or_default();
    let rotation = optional_float_pair(entries, "Rotation").unwrap_or_default();
    let fall_distance = optional_double(entries, "fall_distance").unwrap_or(0.0);
    let remaining_fire_ticks = i32::from(optional_short(entries, "Fire").unwrap_or(0));
    let air_supply = optional_int(entries, "Air");
    let on_ground = optional_bool(entries, "OnGround").unwrap_or(false);
    let invulnerable = optional_bool(entries, "Invulnerable").unwrap_or(false);
    let portal_cooldown = optional_int(entries, "PortalCooldown").unwrap_or(0);
    let no_gravity = optional_bool(entries, "NoGravity").unwrap_or(false);
    let [x, y, z] = position;
    let reject_removed = post_load_allocations.is_some();
    let mut unused_allocations = 0;
    let allocations = post_load_allocations.unwrap_or(&mut unused_allocations);
    let outcome = store
        .restore_initialized_with_effects(
            uuid,
            entity_type_id,
            Vec3d { x, y, z },
            data,
            (reject_removed, allocations),
            |entity, store| {
                if discard_item {
                    entity.discard();
                }
                if let Some(existing) = command_existing {
                    entity.tick_count = existing.tick_count;
                    entity.first_tick = existing.first_tick;
                    entity.no_physics = existing.no_physics;
                    entity.shared_flags = existing.shared_flags;
                    entity.pose_id = existing.pose_id;
                    entity.set_dimensions_and_eye(
                        existing.bb_width(),
                        existing.bb_height(),
                        existing.eye_height(),
                    );
                    if let (Some(living), Some(previous)) = (entity.living_mut(), existing.living())
                    {
                        *living.attributes_mut() = previous.attributes().clone();
                        living.set_all_equipment(previous.equipment());
                    }
                    if let (EntityData::ArmorStand(data), EntityData::ArmorStand(previous)) =
                        (&mut entity.data, existing.data())
                    {
                        data.prepare_command_flags(previous);
                    }
                }
                // `Entity.load` clamps the motion at `:2053-2055` before
                // storing it; the clamp is per-component and zeroes anything
                // beyond ten blocks per tick.
                entity.set_delta_movement(Vec3d {
                    x: clamp_loaded_motion(motion.x),
                    y: clamp_loaded_motion(motion.y),
                    z: clamp_loaded_motion(motion.z),
                });
                entity.set_needs_sync(true);
                // Entity.java:3811-3829: discard non-finite rotations and wrap/clamp pitch.
                let y_rot = if !rotation[0].is_finite() {
                    0.0
                } else if rotation[0].abs() >= 360.0 {
                    rotation[0] % 360.0
                } else {
                    rotation[0]
                };
                let x_rot = if !rotation[1].is_finite() {
                    0.0
                } else {
                    (rotation[1] % 360.0).clamp(-90.0, 90.0)
                };
                entity.set_rot(y_rot, x_rot);
                entity.load_custom_name(custom_name);
                entity.load_custom_data(custom_data);
                entity.set_custom_name_visible(custom_name_visible);
                entity.set_silent(silent);
                entity.set_glowing_tag(glowing);
                entity.set_ticks_frozen(ticks_frozen);
                entity.set_has_visual_fire(has_visual_fire);
                // `:2089-2090`.
                entity.set_entity_tags(tags);
                // `:2062-2063`.
                entity.set_y_head_rot(y_rot);
                entity.fall_distance = fall_distance;
                entity.remaining_fire_ticks = remaining_fire_ticks;
                entity.air_supply =
                    air_supply.unwrap_or_else(|| crate::entity::living::max_air_supply(entity));
                entity.on_ground = on_ground;
                entity.invulnerable = invulnerable;
                entity.portal_cooldown = portal_cooldown;
                entity.no_gravity = no_gravity;
                if matches!(entity.data(), EntityData::LeashKnot(_)) {
                    crate::entity::hanging::set_attached_pos(entity, Vec3d { x, y, z });
                }
                if matches!(
                    entity.data(),
                    EntityData::Painting(_)
                        | EntityData::ItemFrame(_)
                        | EntityData::GlowItemFrame(_)
                ) {
                    crate::entity::hanging::apply_direction(entity);
                }
                finish_item_frame_load(entity);
                crate::entity::slime::finish_load(entity, level);
                if entity.living().is_some() {
                    crate::entity::shulker::restore_armor(entity);
                    load_living_entity(entity, entries, level, items);
                    crate::entity::mannequin::finish_load(entity, level);
                }
                crate::entity::zombie::villager::finish_load(entity, store);
                let baby = match entity.data() {
                    EntityData::GeneralLiving(data) => data.is_baby(),
                    _ => false,
                };
                let previous_baby = command_existing.is_some_and(|entity| match entity.data() {
                    EntityData::GeneralLiving(data) => data.is_baby(),
                    _ => false,
                });
                // Tracked baby changes refresh after Living attributes, including non-Ageable mobs.
                if baby != previous_baby {
                    let _ =
                        crate::entity::living::refresh_dimensions_with_context(entity, level, None);
                }
                if let EntityData::GeneralLiving(data) = entity.data() {
                    if let Some(baby) = data
                        .zombie_retained()
                        .map(crate::entity::zombie::ZombieRetainedState::is_baby)
                    {
                        let _ = crate::entity::zombie::set_baby(entity, baby);
                    }
                }
                crate::entity::general_living::finish_baby_load(entity);
                if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID
                {
                    crate::entity::configure_fox_target_goals(entity);
                }
                crate::entity::fish::finish_load(entity, command_existing, level);
                crate::entity::rabbit::finish_load(entity);
                crate::entity::phantom::finish_load(entity, command_existing, level);
                crate::entity::goat::finish_load(entity);
                crate::entity::camel::finish_load(entity, level);
                if matches!(entity.data(), EntityData::ArmorStand(_)) {
                    load_armor_stand(entity, entries, level);
                }
                if matches!(entity.data(), EntityData::Interaction(_)) {
                    interaction::finish_load(entity);
                }
                if matches!(
                    entity.data(),
                    EntityData::BlockDisplay(_)
                        | EntityData::ItemDisplay(_)
                        | EntityData::TextDisplay(_)
                ) {
                    display::finish_load(entity);
                }
                if matches!(entity.data(), EntityData::AreaEffectCloud(_)) {
                    area_effect_cloud::finish_load(entity);
                }
                if matches!(entity.data(), EntityData::WitherSkull(_)) {
                    crate::entity::hurting_projectile::wither_skull::finish_load(entity);
                }
                true
            },
        )
        .ok_or(EntitySaveError::DuplicateUuid(uuid))?;
    Ok(outcome)
}
