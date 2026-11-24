use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_server_datapack::damage::builtin_damage_type_registry;

use crate::entity::damage::{
    explosion_source_entity, StoredAttackerViews, StoredDamageCausingEntity,
    StoredEntityDamageContext, StoredEntityDamageRules, StoredEntityDamageSource,
};
use crate::entity::level::block_state_tag_membership;
use crate::entity::projectile_action::{hurt_stored_entity, StoredHurtRequest};
use crate::entity::{Entity, EntityData, EntityStore};

#[cfg(test)]
mod tests;

pub(crate) fn cause_fall_damage_inner(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    distance: f64,
    _modifier: f32,
    _incoming_type: &'static str,
) -> Option<bool> {
    let EntityData::FallingBlock(data) = entity.data() else {
        return None;
    };
    if !data.hurt_entities() {
        return Some(false);
    }
    let distance = (distance - 1.0).ceil() as i32;
    if distance < 0 {
        return Some(false);
    }
    let damage = ((distance as f32 * data.fall_damage_per_distance()).floor() as i32)
        .min(data.fall_damage_max()) as f32;
    let block =
        vanilla_block_state_block_type_id(data.block_state_id()).and_then(vanilla_block_type)?;
    let damage_resource = match block.get_state_for_placement_override {
        Some(VanillaGetStateForPlacementOverride::AnvilBlock) => "minecraft:falling_anvil",
        Some(VanillaGetStateForPlacementOverride::PointedDripstoneBlock) => {
            "minecraft:falling_stalactite"
        }
        _ => "minecraft:falling_block",
    };
    let damage_type = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == damage_resource)?;
    let area = entity.bounding_box();
    let targets: Vec<_> = store
        .ids_in_box_sections(area)
        .into_iter()
        .filter(|id| {
            store.get(*id).is_some_and(|target| {
                target.is_alive()
                    && target.living().is_some_and(|living| living.health() > 0.0)
                    && target.bounding_box().intersects(area)
            })
        })
        .collect();
    let registry_tags = level.capture_registry_tags();
    level.with_detached_entity(entity, |level| {
        let source_view = crate::loot_runtime::StoredLootEntity {
            entity,
            bodies: Some(crate::entity::store::EntityBodyReadView::new(
                store,
                Some(entity),
                level.detached_bodies(),
            )),
            raid_at_position: None,
        }
        .snapshot(&bbb_registry_codec::UnavailableHolderResolver);
        let name = entity.custom_name().map_or_else(
            || {
                let (namespace, path) =
                    block.resource_id.split_once(':').expect("catalog block id");
                let path = if path == "wall_torch" {
                    "torch".to_owned()
                } else if path.ends_with("_wall_torch")
                    || path.ends_with("_wall_sign")
                    || path.ends_with("_wall_hanging_sign")
                    || path.ends_with("_wall_banner")
                {
                    path.replace("_wall_", "_")
                } else {
                    path.to_owned()
                };
                let block_name = bbb_protocol::component_translatable_nbt(
                    format!("block.{namespace}.{}", path.replace('/', ".")),
                    Vec::new(),
                    &Default::default(),
                )
                .expect("block description component");
                bbb_protocol::component_translatable_nbt(
                    "entity.minecraft.falling_block_type",
                    vec![block_name],
                    &Default::default(),
                )
                .expect("falling block description component")
            },
            |name| name.component().clone(),
        );
        let display_name = std::sync::Arc::new(
            bbb_protocol::component_appended_nbt(
                &bbb_protocol::ComponentStyle {
                    hover_event: Some(bbb_protocol::ComponentHoverEvent::ShowEntity {
                        id: "minecraft:falling_block".into(),
                        uuid: bbb_protocol::component_hover_uuid_nbt(entity.uuid()),
                        name: None,
                        name_component: Some(name.clone()),
                    }),
                    insertion: Some(entity.uuid().to_string().into()),
                    ..Default::default()
                },
                vec![name],
            )
            .expect("falling block display component"),
        );
        let source = crate::level_event::PendingHurtEntity::FallingBlock {
            entity_id: entity.id(),
            uuid: entity.uuid(),
            position: entity.position(),
            display_name,
        };
        for player in level.players_in_box(area).filter(|player| {
            !player.spectator && !player.instabuild && !player.dead_or_dying && player.health > 0.0
        }) {
            level.hurt_player(crate::level_event::PendingPlayerHurt {
                player: player.uuid,
                damage_type: damage_resource,
                amount: damage,
                causing: Some(source.clone()),
                direct: Some(source.clone()),
                post_attack: false,
                impulse: None,
            });
        }
        for target_id in targets {
            let Some(target) = store.get(target_id) else {
                continue;
            };
            if target.entity_type_id() == VANILLA_ENTITY_TYPE_PLAYER_ID
                || !matches!(
                    target.data(),
                    EntityData::GeneralLiving(_) | EntityData::ArmorStand(_)
                )
            {
                continue;
            }
            let source = StoredEntityDamageSource {
                damage_type,
                direct_entity_id: Some(entity.id()),
                causing_entity: Some(StoredDamageCausingEntity::Other {
                    entity_id: entity.id(),
                }),
                attacker_views: StoredAttackerViews {
                    causing: Some(&source_view),
                    direct: Some(&source_view),
                },
            };
            let request = StoredHurtRequest {
                direct_entity_snapshot: None,
                target_entity_id: target_id,
                damage_type_resource_id: &damage_type.resource_id,
                amount: damage,
                direct_resource_id: Some(entity.data().entity_resource_id()),
                registry_tags: &registry_tags,
                causing_entity_snapshot: Some(explosion_source_entity(entity)),
                source_position: Some(entity.position()),
                explicit_source_position: None,
            };
            let rules = StoredEntityDamageRules {
                mob_griefing: level.mob_griefing(),
                entity_drops: level.entity_drops(),
                tnt_explodes: level.tnt_explodes(),
                ..Default::default()
            };
            let context = StoredEntityDamageContext {
                block_drops: level.block_drops(),
                ..Default::default()
            };
            let _ = hurt_stored_entity(request, None, source, rules, context, store, level);
        }
    });
    let EntityData::FallingBlock(data) = &mut entity.data else {
        return None;
    };
    if block_state_tag_membership(level, data.block_state_id(), "minecraft:anvil")
        && damage > 0.0
        && store.random().next_float() < 0.05_f32 + distance as f32 * 0.05_f32
    {
        match crate::block_behavior::anvil::damage(data.block_state_id())? {
            Some(state) => data.block_state_id = state,
            None => data.cancel_drop = true,
        }
    }
    Some(false)
}
