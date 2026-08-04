//! World -> renderer projection for copper golem statue block-entity models.
//!
//! `CopperGolemStatueBlockRenderer` submits a single baked pose model per
//! statue, so each block becomes one block-sentinel instance carrying the
//! block state's facing, pose and weathering.

use bbb_renderer::{CopperGolemWeathering, EntityModelInstance, SpecialItemCopperGolemPose};
use bbb_world::{
    CopperGolemStatueModelSourceState, CopperGolemStatuePoseKind, CopperGolemStatueWeatherState,
    WorldStore,
};

/// Block-entity model sentinel id; no vanilla network entity id is negative.
const COPPER_GOLEM_STATUE_BLOCK_MODEL_ENTITY_ID: i32 = -1;

pub(crate) fn copper_golem_statue_model_instances_from_world(
    world: &WorldStore,
) -> Vec<EntityModelInstance> {
    world
        .copper_golem_statue_model_source_states()
        .into_iter()
        .map(|source| copper_golem_statue_instance(source, world))
        .collect()
}

fn copper_golem_statue_instance(
    source: CopperGolemStatueModelSourceState,
    world: &WorldStore,
) -> EntityModelInstance {
    let instance = EntityModelInstance::copper_golem_statue_block(
        COPPER_GOLEM_STATUE_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        source.facing_y_rot,
        statue_pose(source.pose),
        statue_weathering(source.weather_state),
    );
    match world.sample_render_light(source.pos) {
        Some(light) => instance.with_light_coords(light.packed()),
        None => instance,
    }
}

fn statue_pose(pose: CopperGolemStatuePoseKind) -> SpecialItemCopperGolemPose {
    match pose {
        CopperGolemStatuePoseKind::Standing => SpecialItemCopperGolemPose::Standing,
        CopperGolemStatuePoseKind::Sitting => SpecialItemCopperGolemPose::Sitting,
        CopperGolemStatuePoseKind::Running => SpecialItemCopperGolemPose::Running,
        CopperGolemStatuePoseKind::Star => SpecialItemCopperGolemPose::Star,
    }
}

fn statue_weathering(weather_state: CopperGolemStatueWeatherState) -> CopperGolemWeathering {
    match weather_state {
        CopperGolemStatueWeatherState::Unaffected => CopperGolemWeathering::Unaffected,
        CopperGolemStatueWeatherState::Exposed => CopperGolemWeathering::Exposed,
        CopperGolemStatueWeatherState::Weathered => CopperGolemWeathering::Weathered,
        CopperGolemStatueWeatherState::Oxidized => CopperGolemWeathering::Oxidized,
    }
}

#[cfg(test)]
mod tests {
    use bbb_renderer::EntityModelKind;
    use bbb_world::BlockPos;

    use super::*;

    fn source(
        pose: CopperGolemStatuePoseKind,
        weather_state: CopperGolemStatueWeatherState,
    ) -> CopperGolemStatueModelSourceState {
        CopperGolemStatueModelSourceState {
            pos: BlockPos { x: 4, y: 5, z: 6 },
            facing_y_rot: 90.0,
            pose,
            weather_state,
        }
    }

    #[test]
    fn statues_project_their_pose_weathering_and_facing() {
        let instance = copper_golem_statue_instance(
            source(
                CopperGolemStatuePoseKind::Running,
                CopperGolemStatueWeatherState::Oxidized,
            ),
            &WorldStore::new(),
        );

        assert_eq!(
            instance.kind,
            EntityModelKind::CopperGolemStatueBlock {
                pose: SpecialItemCopperGolemPose::Running,
                weathering: CopperGolemWeathering::Oxidized,
            }
        );
        assert_eq!(instance.position, [4.0, 5.0, 6.0]);
        // `body_rot` carries `FACING.toYRot()`; the renderer takes the opposite.
        assert_eq!(instance.render_state.body_rot, 90.0);
    }

    #[test]
    fn every_pose_and_weathering_maps_one_to_one() {
        for pose in [
            CopperGolemStatuePoseKind::Standing,
            CopperGolemStatuePoseKind::Sitting,
            CopperGolemStatuePoseKind::Running,
            CopperGolemStatuePoseKind::Star,
        ] {
            for weather_state in [
                CopperGolemStatueWeatherState::Unaffected,
                CopperGolemStatueWeatherState::Exposed,
                CopperGolemStatueWeatherState::Weathered,
                CopperGolemStatueWeatherState::Oxidized,
            ] {
                let instance =
                    copper_golem_statue_instance(source(pose, weather_state), &WorldStore::new());
                let EntityModelKind::CopperGolemStatueBlock {
                    pose: model_pose,
                    weathering,
                } = instance.kind
                else {
                    panic!("statue blocks submit the statue model kind");
                };
                assert_eq!(model_pose, statue_pose(pose));
                assert_eq!(weathering, statue_weathering(weather_state));
            }
        }
    }
}
