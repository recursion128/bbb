use std::collections::BTreeSet;

use bbb_renderer::{ItemEntityBillboard, ItemEntityBillboardLayer, ItemEntityUvRect};
use bbb_world::{ItemEntityStackState, TerrainLight, WorldStore};

use crate::entity_scene::THROWN_ITEM_PROJECTILE_BILLBOARDS;
use bbb_item_model::{ItemAtlasIcon, ItemAtlasIconLayer, ItemAtlasUvRect, NativeItemRuntime};

/// Vanilla `ItemEntityRenderer` lifts the dropped item sprite to sit above the entity's ground
/// position; the thrown-item projectiles (`ThrownItemRenderer`) render centered on the entity, so they
/// use no offset.
const DROPPED_ITEM_ENTITY_BILLBOARD_Y_OFFSET: f32 = 0.25;
const THROWN_ITEM_PROJECTILE_BILLBOARD_Y_OFFSET: f32 = 0.0;

pub(crate) fn item_entity_billboards_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    rendered_as_models: &BTreeSet<i32>,
) -> Vec<ItemEntityBillboard> {
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };

    // Dropped items: the unit-scale sprite lifted above the ground position. Items already drawn as 3D
    // block-item models (see `item_models`) are skipped so they are not double-rendered.
    let mut billboards: Vec<ItemEntityBillboard> = world
        .item_entity_stacks()
        .into_iter()
        .filter(|state| !rendered_as_models.contains(&state.entity_id))
        .filter_map(|state| {
            let icon = item_runtime.icon_for_stack(&state.stack)?;
            Some(item_entity_billboard_from_icon(
                &state,
                icon,
                DROPPED_ITEM_ENTITY_BILLBOARD_Y_OFFSET,
                1.0,
            ))
        })
        .collect();

    // Thrown-item projectiles (snowball, egg, ender pearl, potions, fireballs, …) render the same item
    // sprite via vanilla `ThrownItemRenderer`, centered on the entity at that renderer's sprite scale.
    for &(type_id, scale) in THROWN_ITEM_PROJECTILE_BILLBOARDS {
        for state in world.item_stacks_for_entity_types(&[type_id]) {
            if let Some(icon) = item_runtime.icon_for_stack(&state.stack) {
                billboards.push(item_entity_billboard_from_icon(
                    &state,
                    icon,
                    THROWN_ITEM_PROJECTILE_BILLBOARD_Y_OFFSET,
                    scale,
                ));
            }
        }
    }

    billboards
}

fn item_entity_billboard_from_icon(
    state: &ItemEntityStackState,
    icon: ItemAtlasIcon,
    y_offset: f32,
    scale: f32,
) -> ItemEntityBillboard {
    ItemEntityBillboard {
        position: [
            state.position.x as f32,
            state.position.y as f32 + y_offset,
            state.position.z as f32,
        ],
        scale,
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
            stack: ItemStackSummary {
                item_id: Some(42),
                count: 3,
                component_patch: DataComponentPatchSummary::default(),
            },
        };
        let icon = ItemAtlasIcon {
            layers: vec![
                ItemAtlasIconLayer {
                    uv: ItemAtlasUvRect {
                        min: [0.25, 0.125],
                        max: [0.5, 0.375],
                    },
                    tint: [0.25, 0.5, 0.75, 1.0],
                },
                ItemAtlasIconLayer {
                    uv: ItemAtlasUvRect {
                        min: [0.5, 0.5],
                        max: [0.75, 0.75],
                    },
                    tint: [1.0, 1.0, 1.0, 0.5],
                },
            ],
        };

        let billboard = item_entity_billboard_from_icon(
            &state,
            icon,
            DROPPED_ITEM_ENTITY_BILLBOARD_Y_OFFSET,
            1.0,
        );

        // The dropped item is lifted 0.25 above its ground position.
        assert_eq!(billboard.position, [1.5, 64.25, -2.25]);
        assert_eq!(billboard.scale, 1.0);
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
            stack: ItemStackSummary {
                item_id: Some(11),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        };
        let icon = ItemAtlasIcon {
            layers: vec![ItemAtlasIconLayer {
                uv: ItemAtlasUvRect {
                    min: [0.0, 0.0],
                    max: [0.25, 0.25],
                },
                tint: [1.0, 1.0, 1.0, 1.0],
            }],
        };

        let billboard = item_entity_billboard_from_icon(
            &state,
            icon,
            THROWN_ITEM_PROJECTILE_BILLBOARD_Y_OFFSET,
            3.0,
        );

        assert_eq!(billboard.position, [2.0, 70.5, -4.0]);
        assert_eq!(billboard.scale, 3.0);
        assert_eq!(billboard.light, [1.0, 7.0 / 15.0]);
        assert_eq!(billboard.layers.len(), 1);
    }
}
