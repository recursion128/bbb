//! Native resolution for dispatcher-owned generic entity features.

use bbb_renderer::{
    bake_entity_flame_surface, bake_entity_shadow_surface, CameraPose, EntityFlameSurface,
    EntityModelInstance, EntityShadowPiece, EntityShadowSurface, ENTITY_FLAME_FIRE_0_SPRITE,
    ENTITY_FLAME_FIRE_1_SPRITE,
};
use bbb_world::EntityShadowRenderSourceState;

use crate::terrain_runtime::TerrainTextureState;

pub(crate) fn entity_flame_surfaces(
    instances: &[EntityModelInstance],
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
) -> Vec<EntityFlameSurface> {
    let Some(camera_pose) = camera_pose else {
        return Vec::new();
    };
    let fire_0 = terrain_textures.sprite_uv_rect(ENTITY_FLAME_FIRE_0_SPRITE);
    let fire_1 = terrain_textures.sprite_uv_rect(ENTITY_FLAME_FIRE_1_SPRITE);
    instances
        .iter()
        .filter_map(|instance| bake_entity_flame_surface(*instance, camera_pose, fire_0, fire_1))
        .collect()
}

pub(crate) fn entity_shadow_surfaces(
    sources: &[EntityShadowRenderSourceState],
    instances: &[EntityModelInstance],
) -> Vec<EntityShadowSurface> {
    sources
        .iter()
        .filter_map(|source| {
            let instance = instances
                .iter()
                .copied()
                .find(|instance| instance.entity_id == source.entity_id)?;
            let pieces = source
                .pieces
                .iter()
                .map(|piece| EntityShadowPiece {
                    relative_position: piece.relative_position,
                    shape_min: piece.shape_min,
                    shape_max: piece.shape_max,
                    alpha: piece.alpha,
                })
                .collect::<Vec<_>>();
            bake_entity_shadow_surface(
                instance,
                [
                    source.origin.x as f32,
                    source.origin.y as f32,
                    source.origin.z as f32,
                ],
                source.radius,
                &pieces,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bbb_pack::{AtlasLayout, AtlasRect, AtlasSprite, SpriteTransparency};
    use bbb_renderer::{terrain::TerrainUvRect, EntityModelKind};
    use bbb_world::{EntityShadowPieceState, EntityVec3};

    use super::*;

    #[test]
    fn flame_resolution_uses_named_blocks_atlas_sprites_and_skips_non_burning_instances() {
        let layout = AtlasLayout {
            width: 64,
            height: 16,
            padding: 0,
            sprites: vec![
                atlas_sprite("minecraft:missingno", 0),
                atlas_sprite(ENTITY_FLAME_FIRE_0_SPRITE, 16),
                atlas_sprite(ENTITY_FLAME_FIRE_1_SPRITE, 32),
            ],
        };
        let textures = TerrainTextureState::from_layout(&layout, None, None, None);
        let burning = EntityModelInstance::new(1, EntityModelKind::NoRender, [0.0; 3], 0.0)
            .with_flame_dimensions(Some([1.0, 1.0]));
        let idle = EntityModelInstance::new(2, EntityModelKind::NoRender, [0.0; 3], 0.0);
        let surfaces = entity_flame_surfaces(
            &[burning, idle],
            &textures,
            Some(CameraPose {
                position: [0.0; 3],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: CameraPose::STANDING_EYE_HEIGHT,
            }),
        );

        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0].submission.entity_id, 1);
        assert_eq!(
            textures.sprite_uv_rect(ENTITY_FLAME_FIRE_0_SPRITE),
            TerrainUvRect {
                min: [16.0 / 64.0, 0.0],
                max: [32.0 / 64.0, 1.0],
            }
        );
    }

    #[test]
    fn shadow_resolution_joins_world_pieces_to_the_matching_model_instance() {
        let source = EntityShadowRenderSourceState {
            entity_id: 7,
            origin: EntityVec3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            radius: 0.5,
            pieces: vec![EntityShadowPieceState {
                relative_position: [0.0; 3],
                shape_min: [0.0; 3],
                shape_max: [1.0; 3],
                alpha: 1.0,
            }],
        };
        let instance = EntityModelInstance::new(7, EntityModelKind::NoRender, [4.0, 5.0, 6.0], 0.0);

        let surfaces = entity_shadow_surfaces(std::slice::from_ref(&source), &[instance]);

        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0].submission.entity_id, 7);
        assert_eq!(surfaces[0].submission.root_position, [4.0, 5.0, 6.0]);
        assert!(entity_shadow_surfaces(&[source], &[]).is_empty());
    }

    fn atlas_sprite(id: &str, x: u32) -> AtlasSprite {
        AtlasSprite {
            id: id.to_string(),
            source_width: 16,
            source_height: 16,
            transparency: SpriteTransparency {
                has_transparent: true,
                has_translucent: false,
            },
            animation: None,
            texture_metadata: Default::default(),
            gui_metadata: Default::default(),
            content: AtlasRect {
                x,
                y: 0,
                width: 16,
                height: 16,
            },
            padded: AtlasRect {
                x,
                y: 0,
                width: 16,
                height: 16,
            },
        }
    }
}
