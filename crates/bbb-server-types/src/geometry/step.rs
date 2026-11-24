use super::Aabb;
use bbb_protocol::packets::Vec3d;

// Entity.java:1083-1127: step candidates reuse the initial entity list.
pub fn collide_with_step(
    bounds: Aabb,
    delta: Vec3d,
    clipped: Vec3d,
    on_ground: bool,
    max_step: f32,
    mut collect: impl FnMut(Aabb) -> Option<Vec<Aabb>>,
) -> Option<Vec3d> {
    bbb_facts::collision::collide_with_step(
        bounds,
        [delta.x, delta.y, delta.z],
        [clipped.x, clipped.y, clipped.z],
        on_ground,
        max_step,
        |step_bounds| collect(Aabb::from(step_bounds)),
    )
    .map(|[x, y, z]| Vec3d { x, y, z })
}
