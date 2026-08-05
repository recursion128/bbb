//! Vanilla `ScreenEffectRenderer.renderScreenEffect` extraction: the gates and
//! view-space quad geometry for the view-blocking block, underwater and fire
//! first-person overlays, resolved against the terrain atlas / underwater
//! texture so the renderer pass only projects and rasterizes.

use bbb_renderer::{ScreenEffectQuad, ScreenEffectTexture};
use bbb_world::WorldStore;

use crate::terrain_runtime::TerrainTextureState;

/// Vanilla `ModelBakery.FIRE_1`, the sprite consumed by
/// `ScreenEffectRenderer.renderFire`. Shared with the entity flame feature.
const FIRE_SCREEN_EFFECT_SPRITE: &str = bbb_renderer::ENTITY_FLAME_FIRE_1_SPRITE;

#[cfg(test)]
mod tests;

/// Vanilla `ScreenEffectRenderer.renderScreenEffect` without the item
/// activation branch: `isFirstPerson && !isSleeping` wraps every overlay,
/// `!player.noPhysics` gates the view-blocking block and `!isSpectator()`
/// gates water and fire. A client player's `noPhysics` is exactly the
/// spectator flag (no other client-side writer), so one spectator gate covers
/// both conditions. The overlays sample the local player; only the sleeping
/// gate reads the camera entity.
pub(crate) fn screen_effect_overlay_quads(
    world: &WorldStore,
    textures: &TerrainTextureState,
    first_person: bool,
    sky_darken: u8,
) -> Vec<ScreenEffectQuad> {
    let mut quads = Vec::new();
    if !first_person {
        return quads;
    }
    let camera = world.local_player().camera;
    let camera_entity_sleeping = if camera.follows_player {
        world.local_player_is_sleeping()
    } else {
        camera
            .entity_id
            .is_some_and(|id| world.entity_is_sleeping(id))
    };
    if camera_entity_sleeping || world.local_player_is_spectator() {
        return quads;
    }
    if let Some(state_id) = world.local_player_view_blocking_state() {
        // Vanilla `getParticleMaterial(state).sprite()` always resolves: an
        // undeclared sprite falls back through `TextureAtlas.getSprite` to the
        // atlas missing sprite, which `sprite_uv_rect` reproduces.
        let sprite_id = textures.particle_sprite_id_for_block_state(state_id);
        let rect = textures.sprite_uv_rect(sprite_id.as_deref().unwrap_or("minecraft:missingno"));
        quads.push(full_screen_quad(
            // Vanilla `renderTex`: u1..u0 left to right, v1..v0 bottom to top.
            [
                [rect.max[0], rect.max[1]],
                [rect.min[0], rect.max[1]],
                [rect.min[0], rect.min[1]],
                [rect.max[0], rect.min[1]],
            ],
            [0.1, 0.1, 0.1, 1.0],
            ScreenEffectTexture::TerrainAtlas,
        ));
    }
    if world.local_player_eye_in_water() {
        if let Some(brightness) = world.local_player_eye_brightness(sky_darken) {
            let pose = world.local_player_pose();
            let (y_rot, x_rot) = pose.map_or((0.0, 0.0), |pose| (pose.y_rot, pose.x_rot));
            // Vanilla `renderWater`: UVs tile 0..4 and pan with the head
            // rotation (`-yRot / 64`, `xRot / 64`).
            let uo = -y_rot / 64.0;
            let vo = x_rot / 64.0;
            quads.push(full_screen_quad(
                [
                    [4.0 + uo, 4.0 + vo],
                    [uo, 4.0 + vo],
                    [uo, vo],
                    [4.0 + uo, vo],
                ],
                [brightness, brightness, brightness, 0.1],
                ScreenEffectTexture::Underwater,
            ));
        }
    }
    if world.local_player_is_on_fire() {
        let rect = textures.sprite_uv_rect(FIRE_SCREEN_EFFECT_SPRITE);
        // Vanilla `renderFire`: two half-size quads translated to either side
        // (`-(i * 2 - 1) * 0.24`, `-0.3`) and yawed outward by `(i * 2 - 1) * 10`
        // degrees before projection.
        for side in [-1.0_f32, 1.0_f32] {
            let angle = (side * 10.0_f32).to_radians();
            let (sin, cos) = angle.sin_cos();
            let corner = |x: f32, y: f32| {
                let z = -0.5_f32;
                [
                    -side * 0.24 + x * cos + z * sin,
                    -0.3 + y,
                    -x * sin + z * cos,
                ]
            };
            quads.push(ScreenEffectQuad {
                positions: [
                    corner(-0.5, -0.5),
                    corner(0.5, -0.5),
                    corner(0.5, 0.5),
                    corner(-0.5, 0.5),
                ],
                uvs: [
                    [rect.max[0], rect.max[1]],
                    [rect.min[0], rect.max[1]],
                    [rect.min[0], rect.min[1]],
                    [rect.max[0], rect.min[1]],
                ],
                color: [1.0, 1.0, 1.0, 0.9],
                texture: ScreenEffectTexture::TerrainAtlas,
            });
        }
    }
    quads
}

/// The vanilla block/water overlay quad: x, y in `[-1, 1]` at view z `-0.5`,
/// which always covers the whole screen under the 3D-HUD perspective.
fn full_screen_quad(
    uvs: [[f32; 2]; 4],
    color: [f32; 4],
    texture: ScreenEffectTexture,
) -> ScreenEffectQuad {
    ScreenEffectQuad {
        positions: [
            [-1.0, -1.0, -0.5],
            [1.0, -1.0, -0.5],
            [1.0, 1.0, -0.5],
            [-1.0, 1.0, -0.5],
        ],
        uvs,
        color,
        texture,
    }
}
