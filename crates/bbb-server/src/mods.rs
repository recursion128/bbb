//! Built-in server mod registration and manifest loading for CLI `--mod` / `--mod-pack`.

use std::sync::Arc;

use bbb_facts::boss_bar_styles::{
    VANILLA_BOSS_BAR_COLOR_RED_ORDINAL, VANILLA_BOSS_BAR_OVERLAY_NOTCHED_10_ORDINAL,
};
use bbb_facts::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;
use bbb_facts::entity_types::VanillaEntityMobCategory;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID,
    VANILLA_ATTRIBUTE_MAX_HEALTH_ID, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
};
use bbb_protocol::content_registry::{
    ContentRegistry, ModBlockRegistration, ModBossBarSpec, ModEntityRegistration,
    ModEntityTypeSpec, ModItemRegistration, ModParticleRegistration, ModSynchedFieldSpec,
};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use bbb_server_core::entity::store::EntityStore;
use bbb_server_core::entity::Entity;
use bbb_server_core::plugin::{
    DefaultPluginMobBehavior, MobBehaviorSpec, MobGoalSpec, PluginMobBehavior,
    ServerPluginRegistry, ServerPluginRegistryBuilder,
};
use bbb_server_core::world::simulation_level::LevelCtx;

use crate::ServerOptions;

#[derive(Debug)]
struct InfernoWardenBehavior;

impl PluginMobBehavior for InfernoWardenBehavior {
    fn initial_synched_values(&self, _spec: &ModEntityTypeSpec) -> Vec<EntityDataValue> {
        vec![EntityDataValue {
            data_id: 16,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(0),
        }]
    }

    fn on_tick(
        &self,
        entity: &mut Entity,
        _store: &mut EntityStore,
        _level: &mut LevelCtx<'_, '_>,
    ) {
        let phase = entity
            .living()
            .filter(|living| living.health() <= living.max_health() * 0.5)
            .map_or(0_i8, |_| 1_i8);
        let current_phase = entity
            .plugin_mob()
            .and_then(|plugin| plugin.synched_value(16))
            .and_then(|entry| match entry.value {
                EntityDataValueKind::Byte(val) => Some(val),
                _ => None,
            })
            .unwrap_or(0);
        if phase != current_phase {
            entity.set_plugin_synched_value(EntityDataValue {
                data_id: 16,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(phase),
            });
        }
        if let Some(plugin) = entity.plugin_mob_mut() {
            plugin.timers[0] = plugin.timers[0].saturating_add(1);
        }
    }
}

fn register_demo_boss(builder: &mut ServerPluginRegistryBuilder) {
    builder.register_mob(
        ModEntityRegistration {
            resource_id: "bbb_demo:inferno_warden".to_string(),
            width: 1.2,
            height: 3.2,
            eye_height: 2.8,
            client_tracking_range: 10,
            update_interval: 2,
            category: VanillaEntityMobCategory::Monster,
            is_living: true,
            is_mob: true,
            fire_immune: true,
            can_summon: true,
            serializable: true,
            allowed_in_peaceful: false,
            default_attributes: vec![
                (VANILLA_ATTRIBUTE_MAX_HEALTH_ID, 250.0),
                (VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, 14.0),
                (VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, 0.30),
                (VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID, 40.0),
            ],
            synched_fields: vec![ModSynchedFieldSpec {
                data_id: 16,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            }],
            rig_id: Some("bbb_demo:inferno_warden".to_string()),
            dynamic_light_rgb: Some([1.0, 0.42, 0.08]),
            boss_bar: Some(ModBossBarSpec {
                color_id: VANILLA_BOSS_BAR_COLOR_RED_ORDINAL,
                overlay_id: VANILLA_BOSS_BAR_OVERLAY_NOTCHED_10_ORDINAL,
                darken_screen: true,
                play_boss_music: true,
                create_world_fog: false,
            }),
            ..Default::default()
        },
        MobBehaviorSpec {
            goals: vec![
                (0, MobGoalSpec::Float),
                (
                    2,
                    MobGoalSpec::MeleeAttack {
                        speed_modifier: 1.15,
                        following_target_even_if_not_seen: true,
                    },
                ),
                (
                    5,
                    MobGoalSpec::WaterAvoidingRandomStroll {
                        speed_modifier: 0.8,
                    },
                ),
                (
                    6,
                    MobGoalSpec::LookAtPlayer {
                        look_distance: 16.0,
                    },
                ),
                (6, MobGoalSpec::RandomLookAround),
            ],
            targets: vec![
                (
                    1,
                    MobGoalSpec::HurtByTarget {
                        alert_others: false,
                    },
                ),
                (2, MobGoalSpec::NearestAttackablePlayer { must_see: false }),
            ],
        },
        Arc::new(InfernoWardenBehavior),
    );
}

/// Builds the frozen [`ServerPluginRegistry`] enabled by CLI `--mod` / `--mod-pack` flags.
pub fn build_server_plugins(options: &ServerOptions) -> Result<Arc<ServerPluginRegistry>, String> {
    if options.mods.is_empty() && options.mod_packs.is_empty() {
        return Ok(Arc::clone(ServerPluginRegistry::vanilla()));
    }
    let mut builder = ServerPluginRegistryBuilder::new();
    for mod_id in &options.mods {
        match mod_id.as_str() {
            "demo_boss" => register_demo_boss(&mut builder),
            other => return Err(format!("unknown server mod `{other}`")),
        }
    }
    for pack_path in &options.mod_packs {
        if !pack_path.exists() {
            return Err(format!(
                "mod pack path `{}` does not exist",
                pack_path.display()
            ));
        }
        let manifest_path = if pack_path.is_dir() {
            pack_path.join("content_registry.json")
        } else {
            pack_path.clone()
        };
        let bytes = std::fs::read(&manifest_path).map_err(|error| {
            format!(
                "failed to read mod pack manifest `{}`: {error}",
                manifest_path.display()
            )
        })?;
        let pack: ContentRegistry = serde_json::from_slice(&bytes).map_err(|error| {
            format!(
                "failed to parse mod pack manifest `{}`: {error}",
                manifest_path.display()
            )
        })?;
        for entity in pack.entities() {
            builder.register_mob(
                ModEntityRegistration {
                    resource_id: entity.resource_id.clone(),
                    width: entity.width,
                    height: entity.height,
                    eye_height: entity.eye_height,
                    client_tracking_range: entity.client_tracking_range,
                    update_interval: entity.update_interval,
                    category: entity.category,
                    is_living: entity.is_living,
                    is_mob: entity.is_mob,
                    fire_immune: entity.fire_immune,
                    can_summon: entity.can_summon,
                    serializable: entity.serializable,
                    allowed_in_peaceful: entity.allowed_in_peaceful,
                    default_attributes: entity.default_attributes.clone(),
                    synched_fields: entity.synched_fields.clone(),
                    ambient_sound: entity.ambient_sound.clone(),
                    hurt_sound: entity.hurt_sound.clone(),
                    death_sound: entity.death_sound.clone(),
                    default_loot_table: entity.default_loot_table.clone(),
                    xp_reward: entity.xp_reward,
                    rig_id: entity.rig_id.clone(),
                    dynamic_light_rgb: entity.dynamic_light_rgb,
                    boss_bar: entity.boss_bar.clone(),
                },
                MobBehaviorSpec::default(),
                Arc::new(DefaultPluginMobBehavior),
            );
        }
        for item in pack.items() {
            builder.register_item(ModItemRegistration {
                resource_id: item.resource_id.clone(),
                max_stack_size: item.max_stack_size,
                is_block_item: item.is_block_item,
            });
        }
        for block in pack.blocks() {
            builder.register_block(ModBlockRegistration {
                resource_id: block.resource_id.clone(),
                state_count: block.state_count,
                light_emission: block.light_emission,
                destroy_speed: block.destroy_speed,
                full_cube_collision: block.full_cube_collision,
                dynamic_light_rgb: block.dynamic_light_rgb,
            });
        }
        for sound in pack.sound_events() {
            builder.register_sound_event(sound.resource_id.clone());
        }
        for particle in pack.particles() {
            builder.register_particle(ModParticleRegistration {
                resource_id: particle.resource_id.clone(),
                override_limiter: particle.override_limiter,
            });
        }
    }
    builder
        .freeze()
        .map(Arc::new)
        .map_err(|error| format!("failed to freeze server plugins: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn default_server_options_build_vanilla_plugins() {
        let options = ServerOptions::parse_from(["bbb-server"]);
        let plugins = build_server_plugins(&options).expect("vanilla plugins");
        assert!(plugins.is_vanilla_only());
    }

    #[test]
    fn demo_boss_flag_freezes_inferno_warden_at_expected_id() {
        let options = ServerOptions::parse_from(["bbb-server", "--mod", "demo_boss"]);
        let plugins = build_server_plugins(&options).expect("demo boss plugins");
        assert!(!plugins.is_vanilla_only());
        let entry = plugins
            .mob_entry_by_resource_id("bbb_demo:inferno_warden")
            .expect("inferno_warden registered");
        assert_eq!(entry.spec.id, 157);
        assert!(entry.spec.boss_bar.is_some());
        assert_eq!(entry.spec.synched_fields.len(), 1);
        assert_eq!(entry.spec.synched_fields[0].data_id, 16);
    }
}
