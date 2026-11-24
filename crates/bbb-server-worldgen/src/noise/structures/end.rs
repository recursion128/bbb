use super::*;
use crate::noise::end_city::END_CITY_TYPE_ID;

pub(super) fn owns_set(plan: &NoiseStructurePlan, set: &NoiseStructureSet) -> bool {
    !set.structures.is_empty()
        && set.structures.iter().all(|selection| {
            plan.structures
                .get(&selection.structure)
                .is_some_and(|definition| {
                    definition.type_id == END_CITY_TYPE_ID
                        && definition.jigsaw.is_none()
                        && definition.mineshaft.is_none()
                        && definition.shipwreck.is_none()
                        && definition.ocean_ruin.is_none()
                        && definition.ruined_portal.is_none()
                        && definition.nether_fossil_height.is_none()
                })
        })
}
