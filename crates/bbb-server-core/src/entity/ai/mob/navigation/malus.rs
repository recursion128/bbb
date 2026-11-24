use super::GroundPathType;
use crate::entity::ai::mob::spec::classes;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PathfindingMalus {
    values: [Option<f32>; GroundPathType::BigMobsCloseToDanger as usize + 1],
}

impl Default for PathfindingMalus {
    fn default() -> Self {
        Self {
            values: [None; GroundPathType::BigMobsCloseToDanger as usize + 1],
        }
    }
}

impl PathfindingMalus {
    pub(crate) fn get(self, path_type: GroundPathType) -> f32 {
        self.values[path_type as usize].unwrap_or_else(|| path_type.default_malus())
    }

    pub(crate) fn set(&mut self, path_type: GroundPathType, value: f32) {
        self.values[path_type as usize] = Some(value);
    }

    /// More specific classes supply each override before their ancestors.
    pub(crate) fn for_entity_type(entity_type_id: i32) -> Self {
        let mut malus = Self::default();
        for class in classes(entity_type_id) {
            for &(path_type, value) in Self::class_overrides(class) {
                malus.values[path_type as usize].get_or_insert(value);
            }
        }
        malus
    }

    fn class_overrides(class: &str) -> &'static [(GroundPathType, f32)] {
        use GroundPathType::*;
        match class {
            "Animal" | "AbstractVillager" | "AbstractPiglin" => {
                &[(FireInNeighbor, 16.0), (Fire, -1.0)]
            }
            "CopperGolem" => &[
                (FireInNeighbor, 16.0),
                (DamagingInNeighbor, 16.0),
                (Fire, -1.0),
            ],
            "AgeableWaterCreature"
            | "WaterAnimal"
            | "AbstractNautilus"
            | "Drowned"
            | "Guardian"
            | "Axolotl"
            | "Chicken" => &[(Water, 0.0)],
            "Ravager" => &[(Leaves, 0.0)],
            "Parrot" => &[(FireInNeighbor, -1.0), (Fire, -1.0), (Cocoa, -1.0)],
            "Bee" => &[
                (FireInNeighbor, -1.0),
                (Water, -1.0),
                (WaterBorder, 16.0),
                (Cocoa, -1.0),
                (Fence, -1.0),
            ],
            // Frog. AmphibiousNodeEvaluator.prepare later makes WATER=0.
            "Frog" => &[(Water, 4.0), (Trapdoor, -1.0)],
            "Strider" => &[
                (Water, -1.0),
                (Lava, 0.0),
                (FireInNeighbor, 0.0),
                (Fire, 0.0),
            ],
            "Turtle" => &[
                (Water, 0.0),
                (DoorIronClosed, -1.0),
                (DoorWoodClosed, -1.0),
                (DoorOpen, -1.0),
            ],
            "Goat" | "Wolf" => &[(PowderSnow, -1.0), (OnTopOfPowderSnow, -1.0)],
            "Sniffer" => &[
                (Water, -1.0),
                (OnTopOfPowderSnow, -1.0),
                (DamageCautious, -1.0),
            ],
            "Fox" => &[(DamagingInNeighbor, 0.0), (Damaging, 0.0)],
            "EnderMan" => &[(Water, -1.0)],
            "WitherSkeleton" | "ZombifiedPiglin" => &[(Lava, 8.0)],
            "Blaze" => &[
                (Water, -1.0),
                (Lava, 8.0),
                (FireInNeighbor, 0.0),
                (Fire, 0.0),
            ],
            "Warden" => &[
                (UnpassableRail, 0.0),
                (Damaging, 8.0),
                (PowderSnow, 8.0),
                (Lava, 8.0),
                (Fire, 0.0),
                (FireInNeighbor, 0.0),
            ],
            "Breeze" => &[(OnTopOfTrapdoor, -1.0), (Fire, -1.0)],
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests;
