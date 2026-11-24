use super::*;
use crate::world::LevelId;
use bbb_server_types::item_stack::RuntimeItemStack;

impl ServerWorld {
    // DefaultDispenseItemBehavior.java:30-49: all callers share level RNG and no pickup delay.
    pub(in crate::world) fn spawn_dispensed_stack(
        &mut self,
        dimension: &LevelId,
        mut position: Vec3d,
        direction: Direction,
        accuracy: i32,
        item: &RuntimeItemStack,
    ) -> bbb_server_types::Result<()> {
        let step = bbb_server_types::directions::relative(BlockPos { x: 0, y: 0, z: 0 }, direction);
        position.y -= if matches!(direction, Direction::Up | Direction::Down) {
            0.125
        } else {
            0.15625
        };
        let random = &self.random;
        let power = random.next_double() * 0.1 + 0.2;
        let spread = 0.0172275 * f64::from(accuracy);
        let triangle = |mean| random.triangle(mean, spread);
        let velocity = Vec3d {
            x: triangle(f64::from(step.x) * power),
            y: triangle(0.2),
            z: triangle(f64::from(step.z) * power),
        };
        crate::entity::spawn_dispensed_item_entity(
            &mut self.level_mut_by_id(dimension).entities,
            position,
            item.clone(),
            velocity,
        )
        .ok_or_else(|| {
            bbb_server_types::ServerWorldError::InvalidConfiguration(
                "dispensed item spawn refused".into(),
            )
        })?;
        Ok(())
    }
}
