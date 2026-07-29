use std::collections::BTreeSet;

use bbb_pack::BlockModelDisplayContext;
use bbb_renderer::{
    ItemEntityBillboard, ItemEntityBillboardLayer, ItemEntityBillboardOrientation, ItemEntityUvRect,
};
use bbb_world::{
    vanilla_entity_renderer_thrown_item_scales, FireworkRocketItemState, ItemEntityStackState,
    TerrainLight, WorldStore,
};

use bbb_item_model::{
    ItemAtlasIcon, ItemAtlasIconLayer, ItemAtlasUvRect, ItemModelKeybindContext,
    ItemModelUseContext, NativeItemRuntime,
};

use crate::item_models::{
    world_attribute_keys, world_damage_type_keys, world_enchantment_keys, world_trim_material_keys,
    world_trim_pattern_keys,
};

/// Vanilla `ItemEntityRenderer` lifts the dropped item sprite to sit above the entity's ground
/// position; the thrown-item projectiles (`ThrownItemRenderer`) render centered on the entity, so they
/// use no offset.
const DROPPED_ITEM_ENTITY_BILLBOARD_Y_OFFSET: f32 = 0.25;
const THROWN_ITEM_PROJECTILE_BILLBOARD_Y_OFFSET: f32 = 0.0;
const FIREWORK_ROCKET_BILLBOARD_Y_OFFSET: f32 = 0.0;

pub(crate) fn item_entity_billboards_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    rendered_as_models: &BTreeSet<i32>,
) -> Vec<ItemEntityBillboard> {
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let icon_context = WorldItemIconContext::new(world);

    // Dropped items: the unit-scale sprite lifted above the ground position. Items already drawn as 3D
    // block-item models (see `item_models`) are skipped so they are not double-rendered.
    let mut billboards: Vec<ItemEntityBillboard> = world
        .item_entity_stacks()
        .into_iter()
        .filter(|state| !rendered_as_models.contains(&state.entity_id))
        .filter_map(|state| {
            let icon = item_icon_for_world_stack(
                &icon_context,
                item_runtime,
                &state.stack,
                Some("minecraft:item"),
                state.entity_id,
            )?;
            Some(item_entity_billboard_from_icon(
                &state,
                icon,
                DROPPED_ITEM_ENTITY_BILLBOARD_Y_OFFSET,
                1.0,
                ItemEntityBillboardOrientation::CameraFacing,
            ))
        })
        .collect();

    // Thrown-item projectiles (snowball, egg, ender pearl, potions, fireballs, …) render the same item
    // sprite via vanilla `ThrownItemRenderer`, centered on the entity at that renderer's sprite scale.
    for (type_id, scale) in vanilla_entity_renderer_thrown_item_scales() {
        let context_entity_type =
            bbb_protocol::entity_types::vanilla_entity_resource_id_for_type_id(type_id);
        for state in world.item_stacks_for_entity_types(&[type_id]) {
            if let Some(icon) = item_icon_for_world_stack(
                &icon_context,
                item_runtime,
                &state.stack,
                context_entity_type.as_deref(),
                state.entity_id,
            ) {
                billboards.push(item_entity_billboard_from_icon(
                    &state,
                    icon,
                    THROWN_ITEM_PROJECTILE_BILLBOARD_Y_OFFSET,
                    scale,
                    ItemEntityBillboardOrientation::CameraFacing,
                ));
            }
        }
    }

    // Firework rockets use vanilla `FireworkEntityRenderer`: a camera-facing `ItemStackRenderState`
    // with an extra post-camera rotation when `FireworkRocketRenderState.isShotAtAngle` is true.
    for state in world.firework_rocket_item_states() {
        if let Some(icon) = item_icon_for_world_stack(
            &icon_context,
            item_runtime,
            &state.stack,
            Some("minecraft:firework_rocket"),
            state.entity_id,
        ) {
            billboards.push(firework_rocket_billboard_from_icon(&state, icon));
        }
    }

    billboards
}

struct WorldItemIconContext<'a> {
    world: &'a WorldStore,
    trim_material_keys: Option<Vec<String>>,
    trim_pattern_keys: Option<Vec<String>>,
    enchantment_keys: Option<Vec<String>>,
    damage_type_keys: Option<Vec<String>>,
    attribute_keys: Option<Vec<String>>,
}

impl<'a> WorldItemIconContext<'a> {
    fn new(world: &'a WorldStore) -> Self {
        Self {
            world,
            trim_material_keys: world_trim_material_keys(world),
            trim_pattern_keys: world_trim_pattern_keys(world),
            enchantment_keys: world_enchantment_keys(world),
            damage_type_keys: world_damage_type_keys(world),
            attribute_keys: world_attribute_keys(world),
        }
    }
}

fn item_icon_for_world_stack(
    context: &WorldItemIconContext<'_>,
    item_runtime: &NativeItemRuntime,
    stack: &bbb_protocol::packets::ItemStackSummary,
    context_entity_type: Option<&str>,
    item_model_seed: i32,
) -> Option<ItemAtlasIcon> {
    let effective_stack = context.world.effective_item_stack(stack);
    let stack = effective_stack.as_ref();
    item_runtime
        .icon_for_stack_with_context_and_use_context_time_state_and_fishing_rod_cast_with_registry_context(
            stack,
            None,
            false,
            ItemModelUseContext::inactive(),
            BlockModelDisplayContext::Ground,
            0.0,
            context.trim_material_keys.as_deref(),
            context.trim_pattern_keys.as_deref(),
            context.enchantment_keys.as_deref(),
            context.damage_type_keys.as_deref(),
            context.attribute_keys.as_deref(),
            None,
            context_entity_type,
            None,
            context
                .world
                .level_info()
                .map(|level| level.dimension.as_str()),
            None,
            None,
            false,
            false,
            false,
            false,
            ItemModelKeybindContext::default(),
            false,
            item_model_seed,
            context.world.frozen_registry_access(),
        )
}

fn item_entity_billboard_from_icon(
    state: &ItemEntityStackState,
    icon: ItemAtlasIcon,
    y_offset: f32,
    scale: f32,
    orientation: ItemEntityBillboardOrientation,
) -> ItemEntityBillboard {
    ItemEntityBillboard {
        position: [
            state.position.x as f32,
            state.position.y as f32 + y_offset,
            state.position.z as f32,
        ],
        scale,
        orientation,
        light: shader_light(state.light),
        layers: icon
            .layers
            .into_iter()
            .map(item_entity_billboard_layer)
            .collect(),
    }
}

fn firework_rocket_billboard_from_icon(
    state: &FireworkRocketItemState,
    icon: ItemAtlasIcon,
) -> ItemEntityBillboard {
    ItemEntityBillboard {
        position: [
            state.position.x as f32,
            state.position.y as f32 + FIREWORK_ROCKET_BILLBOARD_Y_OFFSET,
            state.position.z as f32,
        ],
        scale: 1.0,
        orientation: if state.shot_at_angle {
            ItemEntityBillboardOrientation::FireworkShotAtAngle
        } else {
            ItemEntityBillboardOrientation::CameraFacing
        },
        light: shader_light(state.light),
        layers: icon
            .layers
            .into_iter()
            .map(item_entity_billboard_layer)
            .collect(),
    }
}

fn shader_light(light: TerrainLight) -> [f32; 2] {
    [
        light.block.min(15) as f32 / 15.0,
        light.sky.min(15) as f32 / 15.0,
    ]
}

fn item_entity_billboard_layer(layer: ItemAtlasIconLayer) -> ItemEntityBillboardLayer {
    ItemEntityBillboardLayer::new(item_entity_uv_rect(layer.uv), layer.tint)
}

fn item_entity_uv_rect(uv: ItemAtlasUvRect) -> ItemEntityUvRect {
    ItemEntityUvRect {
        min: uv.min,
        max: uv.max,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{DataComponentPatchSummary, ItemStackSummary};
    use bbb_world::EntityVec3;

    #[test]
    fn item_entity_billboards_from_world_without_runtime_is_empty() {
        assert_eq!(
            item_entity_billboards_from_world(&WorldStore::new(), None, &BTreeSet::new()),
            Vec::new()
        );
    }

    #[test]
    fn item_entity_billboard_from_icon_projects_position_and_layers() {
        let state = ItemEntityStackState {
            entity_id: 7,
            position: EntityVec3 {
                x: 1.5,
                y: 64.0,
                z: -2.25,
            },
            light: TerrainLight { sky: 12, block: 5 },
            bob_offset: 0.0,
            stack: ItemStackSummary {
                item_id: Some(42),
                count: 3,
                component_patch: DataComponentPatchSummary::default(),
            },
        };
        let icon = ItemAtlasIcon {
            layers: vec![
                ItemAtlasIconLayer {
                    texture_index: 0,
                    uv: ItemAtlasUvRect {
                        min: [0.25, 0.125],
                        max: [0.5, 0.375],
                    },
                    tint: [0.25, 0.5, 0.75, 1.0],
                    translucent: false,
                },
                ItemAtlasIconLayer {
                    texture_index: 1,
                    uv: ItemAtlasUvRect {
                        min: [0.5, 0.5],
                        max: [0.75, 0.75],
                    },
                    tint: [1.0, 1.0, 1.0, 0.5],
                    translucent: false,
                },
            ],
        };

        let billboard = item_entity_billboard_from_icon(
            &state,
            icon,
            DROPPED_ITEM_ENTITY_BILLBOARD_Y_OFFSET,
            1.0,
            ItemEntityBillboardOrientation::CameraFacing,
        );

        // The dropped item is lifted 0.25 above its ground position.
        assert_eq!(billboard.position, [1.5, 64.25, -2.25]);
        assert_eq!(billboard.scale, 1.0);
        assert_eq!(
            billboard.orientation,
            ItemEntityBillboardOrientation::CameraFacing
        );
        assert_eq!(billboard.light, [5.0 / 15.0, 12.0 / 15.0]);
        assert_eq!(billboard.layers.len(), 2);
        assert_eq!(
            billboard.layers[0],
            ItemEntityBillboardLayer::new(
                ItemEntityUvRect {
                    min: [0.25, 0.125],
                    max: [0.5, 0.375],
                },
                [0.25, 0.5, 0.75, 1.0],
            )
        );
        assert_eq!(
            billboard.layers[1],
            ItemEntityBillboardLayer::new(
                ItemEntityUvRect {
                    min: [0.5, 0.5],
                    max: [0.75, 0.75],
                },
                [1.0, 1.0, 1.0, 0.5],
            )
        );
    }

    #[test]
    fn thrown_item_projectile_billboard_is_centered_on_the_entity() {
        // Unlike the dropped item, a thrown-item projectile (`ThrownItemRenderer`) renders centered on
        // the entity position, with no lift offset.
        let state = ItemEntityStackState {
            entity_id: 9,
            position: EntityVec3 {
                x: 2.0,
                y: 70.5,
                z: -4.0,
            },
            light: TerrainLight { sky: 7, block: 15 },
            bob_offset: 0.0,
            stack: ItemStackSummary {
                item_id: Some(11),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        };
        let icon = ItemAtlasIcon {
            layers: vec![ItemAtlasIconLayer {
                texture_index: 0,
                uv: ItemAtlasUvRect {
                    min: [0.0, 0.0],
                    max: [0.25, 0.25],
                },
                tint: [1.0, 1.0, 1.0, 1.0],
                translucent: false,
            }],
        };

        let billboard = item_entity_billboard_from_icon(
            &state,
            icon,
            THROWN_ITEM_PROJECTILE_BILLBOARD_Y_OFFSET,
            3.0,
            ItemEntityBillboardOrientation::CameraFacing,
        );

        assert_eq!(billboard.position, [2.0, 70.5, -4.0]);
        assert_eq!(billboard.scale, 3.0);
        assert_eq!(
            billboard.orientation,
            ItemEntityBillboardOrientation::CameraFacing
        );
        assert_eq!(billboard.light, [1.0, 7.0 / 15.0]);
        assert_eq!(billboard.layers.len(), 1);
    }

    #[test]
    fn firework_rocket_billboard_projects_shot_at_angle_orientation() {
        let state = FireworkRocketItemState {
            entity_id: 12,
            position: EntityVec3 {
                x: -1.0,
                y: 68.25,
                z: 3.5,
            },
            light: TerrainLight { sky: 9, block: 4 },
            stack: ItemStackSummary {
                item_id: Some(42),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
            shot_at_angle: true,
        };
        let icon = ItemAtlasIcon {
            layers: vec![ItemAtlasIconLayer {
                texture_index: 0,
                uv: ItemAtlasUvRect {
                    min: [0.25, 0.0],
                    max: [0.5, 0.25],
                },
                tint: [0.8, 0.7, 0.6, 1.0],
                translucent: false,
            }],
        };

        let billboard = firework_rocket_billboard_from_icon(&state, icon);

        assert_eq!(billboard.position, [-1.0, 68.25, 3.5]);
        assert_eq!(billboard.scale, 1.0);
        assert_eq!(
            billboard.orientation,
            ItemEntityBillboardOrientation::FireworkShotAtAngle
        );
        assert_eq!(billboard.light, [4.0 / 15.0, 9.0 / 15.0]);
        assert_eq!(billboard.layers.len(), 1);
    }
}
