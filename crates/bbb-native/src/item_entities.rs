use bbb_renderer::{ItemEntityBillboard, ItemEntityBillboardLayer, ItemEntityUvRect};
use bbb_world::{ItemEntityStackState, WorldStore};

use crate::item_runtime::{ItemAtlasIcon, ItemAtlasIconLayer, ItemAtlasUvRect, NativeItemRuntime};

const ITEM_ENTITY_BILLBOARD_Y_OFFSET: f32 = 0.25;

pub(crate) fn item_entity_billboards_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<ItemEntityBillboard> {
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };

    world
        .item_entity_stacks()
        .into_iter()
        .filter_map(|state| {
            let icon = item_runtime.icon_for_stack(&state.stack)?;
            Some(item_entity_billboard_from_icon(&state, icon))
        })
        .collect()
}

fn item_entity_billboard_from_icon(
    state: &ItemEntityStackState,
    icon: ItemAtlasIcon,
) -> ItemEntityBillboard {
    ItemEntityBillboard {
        position: [
            state.position.x as f32,
            state.position.y as f32 + ITEM_ENTITY_BILLBOARD_Y_OFFSET,
            state.position.z as f32,
        ],
        layers: icon
            .layers
            .into_iter()
            .map(item_entity_billboard_layer)
            .collect(),
    }
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
            item_entity_billboards_from_world(&WorldStore::new(), None),
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

        let billboard = item_entity_billboard_from_icon(&state, icon);

        assert_eq!(billboard.position, [1.5, 64.25, -2.25]);
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
}
