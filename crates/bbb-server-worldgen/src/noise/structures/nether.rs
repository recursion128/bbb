use super::*;

pub(super) fn structure_start_owner(
    plan: &NoiseStructurePlan,
    set: &NoiseStructureSet,
) -> Option<StructureStartSetOwner> {
    if !set.structures.is_empty()
        && set.structures.iter().any(|selection| {
            plan.structures
                .get(&selection.structure)
                .is_some_and(|definition| definition.type_id == "minecraft:fortress")
        })
        && set.structures.iter().all(|selection| {
            plan.structures
                .get(&selection.structure)
                .is_some_and(|definition| {
                    definition.type_id == "minecraft:fortress" || definition.jigsaw.is_some()
                })
        })
    {
        return Some(StructureStartSetOwner::NetherComplex);
    }
    if !set.structures.is_empty()
        && set.structures.iter().all(|selection| {
            plan.structures
                .get(&selection.structure)
                .is_some_and(|definition| {
                    definition.type_id == "minecraft:nether_fossil"
                        && definition.nether_fossil_height.is_some()
                })
        })
    {
        return Some(StructureStartSetOwner::NetherFossil);
    }
    None
}
