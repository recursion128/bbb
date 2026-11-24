use crate::chunk::noise_generator::features::StructurePlacementFamily;
use bbb_server_worldgen::noise::structures::NoiseStructureDecorationInvocation;

use super::*;

impl<'resources> StructureGenerationContext<'resources> {
    pub(crate) fn structure_owner(&self, id: &str) -> Option<StructureStartSetOwner> {
        self.plan.registered_structure_owner(id)
    }

    pub(crate) fn placement_family(&self, id: &str) -> Option<StructurePlacementFamily> {
        Some(match self.structure_owner(id)? {
            StructureStartSetOwner::Jigsaw => StructurePlacementFamily::Jigsaw,
            StructureStartSetOwner::BuriedTreasure => StructurePlacementFamily::BuriedTreasure,
            StructureStartSetOwner::Mineshaft => StructurePlacementFamily::Mineshaft,
            StructureStartSetOwner::Shipwreck => StructurePlacementFamily::Shipwreck,
            StructureStartSetOwner::OceanRuin => StructurePlacementFamily::OceanRuin,
            StructureStartSetOwner::Igloo => StructurePlacementFamily::Igloo,
            StructureStartSetOwner::DesertPyramid => StructurePlacementFamily::DesertPyramid,
            StructureStartSetOwner::JungleTemple => StructurePlacementFamily::JungleTemple,
            StructureStartSetOwner::SwampHut => StructurePlacementFamily::SwampHut,
            StructureStartSetOwner::RuinedPortal => StructurePlacementFamily::RuinedPortal,
            StructureStartSetOwner::OceanMonument => StructurePlacementFamily::OceanMonument,
            StructureStartSetOwner::WoodlandMansion => StructurePlacementFamily::WoodlandMansion,
            StructureStartSetOwner::Stronghold => StructurePlacementFamily::Stronghold,
            StructureStartSetOwner::NetherFossil => StructurePlacementFamily::NetherFossil,
            StructureStartSetOwner::EndCity => StructurePlacementFamily::EndCity,
            StructureStartSetOwner::NetherComplex => {
                let definition = self.plan.structures().get(id)?;
                if definition.jigsaw.is_some() {
                    StructurePlacementFamily::Jigsaw
                } else if definition.type_id == FORTRESS_TYPE_ID {
                    StructurePlacementFamily::Fortress
                } else {
                    return None;
                }
            }
            _ => return None,
        })
    }

    pub(crate) fn structure_invocation(
        &self,
        target: [i32; 2],
        id: &str,
        family: StructurePlacementFamily,
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let path = format!(
            "structure_generation.{}.invocation[{},{}].{id}",
            self.label, target[0], target[1]
        );
        if self.placement_family(id) != Some(family) {
            return Err(invalid_schema(
                &path,
                "structure does not belong to the requested typed placement owner",
            ));
        }
        self.plan
            .decoration_invocations_for_chunk(self.world_seed, target[0], target[1])
            .into_iter()
            .find(|invocation| invocation.structure == id)
            .ok_or_else(|| {
                invalid_schema(&path, "structure is absent from frozen decoration order")
            })
    }
}
