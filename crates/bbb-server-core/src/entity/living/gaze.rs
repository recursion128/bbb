use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::level::NearbyPlayer;

pub(crate) fn not_wearing_disguise(level: &LevelCtx<'_, '_>, player: &NearbyPlayer) -> bool {
    let Some(item) = player.head_item_resource_id else {
        return true;
    };
    !level.registry_tag_membership("minecraft:item", "minecraft:gaze_disguise_equipment", item)
}

pub(crate) fn is_looking_at_me(
    level: &LevelCtx<'_, '_>,
    body: Vec3d,
    heights: &[f64],
    player: &NearbyPlayer,
    cone: f64,
    adjust_distance: bool,
    visual: bool,
) -> Option<bool> {
    let view = crate::entity::ai::mob::stroll::view_vector(player.x_rot, player.y_rot).normalize();
    let from = Vec3d {
        y: player.position.y + player.eye_height,
        ..player.position
    };
    for height in heights {
        let to = Vec3d {
            x: body.x,
            y: *height,
            z: body.z,
        };
        let direction = Vec3d {
            x: to.x - from.x,
            y: to.y - from.y,
            z: to.z - from.z,
        };
        let distance = direction.length_sqr().sqrt();
        let direction = direction.normalize();
        let dot = view.x * direction.x + view.y * direction.y + view.z * direction.z;
        if dot > 1.0 - cone / if adjust_distance { distance } else { 1.0 } && distance <= 128.0 {
            let hit = bbb_server_types::geometry::clip::clip_with(from, to, |pos| {
                Some((level.player_gaze_shape(player, pos, visual)?, None))
            })?;
            if hit.is_none() {
                return Some(true);
            }
        }
    }
    Some(false)
}

#[cfg(test)]
pub(crate) mod tests;
