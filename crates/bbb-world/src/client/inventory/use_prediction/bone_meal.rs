use super::*;
use crate::{block_name_is_air, BlockProbe, TerrainFluidKind};
use bbb_protocol::block_types::{
    VanillaBonemealTargetOverride, VANILLA_BLOCK_PALE_MOSS_CARPET_ID, VANILLA_BLOCK_WATER_ID,
};

impl WorldStore {
    /// Mirrors the final client-side result of `BoneMealItem.useOn`.
    ///
    /// Vanilla evaluates a bonemealable target first, but a valid target
    /// returns `PASS` on the client because growth and stack shrink are
    /// server-only. The client only returns `SUCCESS` for the separate full
    /// water branch, and that branch is unreachable when the clicked target is
    /// itself valid.
    pub(super) fn predict_local_bone_meal_item_use_on(
        &self,
        owner: VanillaItemUseOnOverride,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let Some(clicked) = self.probe_block(context.pos) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };

        // A bonemealable target and a non-sturdy clicked face both produce the
        // same client-visible `PASS`, so target validity is only needed after
        // the sturdy-face and adjacent-water gates have succeeded.
        if !self.debug_block_sturdy_faces(&clicked)[sturdy_face_index(context.face)] {
            return pass();
        }

        let water_pos = relative(context.pos, context.face);
        let Some(water) = self.probe_block(water_pos) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        let full_water = block_has_type(&water, VANILLA_BLOCK_WATER_ID) == Some(true)
            && water
                .fluid
                .is_some_and(|fluid| fluid.kind == TerrainFluidKind::Water && fluid.amount == 8);
        if !full_water {
            return pass();
        }

        let Some(clicked_name) = clicked.block_name.as_deref() else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        let Some(block_type) = vanilla_block_type_for_resource_id(clicked_name) else {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        };
        match self.bonemeal_target_is_valid(&clicked, block_type.bonemeal_target_override) {
            Some(true) => pass(),
            Some(false) => LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            )),
            None => LocalTargetUsePrediction::UnsupportedItemUseOn(owner),
        }
    }

    /// Evaluates every bonemealable owner whose vanilla block states can expose
    /// a full sturdy face. The other generated owners have no sturdy faces, so
    /// `BoneMealItem.useOn` has already returned `PASS` before reaching here;
    /// a catalog regression test locks that reachability fact.
    fn bonemeal_target_is_valid(
        &self,
        block: &BlockProbe,
        target_owner: Option<VanillaBonemealTargetOverride>,
    ) -> Option<bool> {
        let owner = match target_owner {
            None => return Some(false),
            Some(owner) => owner,
        };
        match owner {
            // These vanilla implementations require `ServerLevel`; a client
            // `Level` therefore always reports an invalid target.
            VanillaBonemealTargetOverride::AzaleaBlock
            | VanillaBonemealTargetOverride::MushroomBlock
            | VanillaBonemealTargetOverride::SaplingBlock => Some(false),
            VanillaBonemealTargetOverride::BonemealableFeaturePlacerBlock => {
                self.block_is_air(above(block.pos))
            }
            VanillaBonemealTargetOverride::GrassBlock
            | VanillaBonemealTargetOverride::NyliumBlock => {
                let growth_pos = above(block.pos);
                Some(self.is_inside_build_height(growth_pos) && self.block_is_air(growth_pos)?)
            }
            VanillaBonemealTargetOverride::NetherrackBlock => {
                self.netherrack_bonemeal_target_is_valid(block.pos)
            }
            VanillaBonemealTargetOverride::MossyCarpetBlock => {
                self.mossy_carpet_bonemeal_target_is_valid(block)
            }
            VanillaBonemealTargetOverride::RootedDirtBlock => {
                let growth_pos = below(block.pos);
                Some(self.is_inside_build_height(growth_pos) && self.block_is_air(growth_pos)?)
            }
            _ => None,
        }
    }

    fn netherrack_bonemeal_target_is_valid(&self, pos: BlockPos) -> Option<bool> {
        let above_pos = above(pos);
        let above_block = self.probe_block(above_pos)?;
        if !crate::client::block_state_propagates_skylight_down(&above_block, above_pos)? {
            return Some(false);
        }

        let nylium = self
            .registry_resource_ids_for_tag("minecraft:block", "minecraft:nylium")
            .ok()??;
        for x_offset in -1..=1 {
            for y_offset in -1..=1 {
                for z_offset in -1..=1 {
                    let neighbour_pos = offset(pos, x_offset, y_offset, z_offset);
                    let neighbour = self.probe_block(neighbour_pos)?;
                    let neighbour_is_nylium = neighbour
                        .block_name
                        .as_deref()
                        .is_some_and(|name| nylium.contains(&name));
                    if neighbour_is_nylium {
                        return Some(true);
                    }
                }
            }
        }
        Some(false)
    }

    fn mossy_carpet_bonemeal_target_is_valid(&self, block: &BlockProbe) -> Option<bool> {
        if !block_boolean_property(block, "bottom")? {
            return Some(false);
        }

        let topper_pos = above(block.pos);
        let previous_topper = self.probe_block(topper_pos)?;
        let previous_is_carpet =
            block_has_type(&previous_topper, VANILLA_BLOCK_PALE_MOSS_CARPET_ID)?;
        if previous_is_carpet {
            if block_boolean_property(&previous_topper, "bottom")? {
                return Some(false);
            }
        } else {
            let previous_name = previous_topper.block_name.as_deref()?;
            if !vanilla_block_type_for_resource_id(previous_name)?.replaceable {
                return Some(false);
            }
        }

        let two_above = self.probe_block(above(topper_pos))?;
        let two_above_is_topper = block_has_type(&two_above, VANILLA_BLOCK_PALE_MOSS_CARPET_ID)?
            && !block_boolean_property(&two_above, "bottom")?;
        let mut candidate_sides = ["none"; 4];
        for (index, (direction, property)) in HORIZONTAL_CARPET_SIDES.into_iter().enumerate() {
            let support_pos = relative(topper_pos, direction);
            let support = self.probe_block(support_pos)?;
            if !self.block_has_full_support_or_collision_face(
                &support,
                sturdy_face_index(opposite(direction)),
            ) {
                continue;
            }

            let mut side = "low";
            if two_above_is_topper
                && two_above
                    .block_properties
                    .get(property)
                    .is_some_and(|value| value != "none")
            {
                side = "tall";
            }
            if block
                .block_properties
                .get(property)
                .is_some_and(|value| value == "none")
            {
                side = "none";
            }
            candidate_sides[index] = side;
        }
        if candidate_sides.iter().all(|side| *side == "none") {
            return Some(false);
        }
        if !previous_is_carpet {
            return Some(true);
        }
        Some(
            HORIZONTAL_CARPET_SIDES
                .into_iter()
                .zip(candidate_sides)
                .any(|((_, property), candidate)| {
                    previous_topper
                        .block_properties
                        .get(property)
                        .is_none_or(|previous| previous != candidate)
                }),
        )
    }

    fn block_is_air(&self, pos: BlockPos) -> Option<bool> {
        self.probe_block(pos)
            .map(|block| block.block_name.as_deref().is_some_and(block_name_is_air))
    }

    fn is_inside_build_height(&self, pos: BlockPos) -> bool {
        let dimension = self.dimension();
        pos.y >= dimension.min_y && pos.y < dimension.min_y.saturating_add(dimension.height)
    }
}

const HORIZONTAL_CARPET_SIDES: [(ProtocolDirection, &str); 4] = [
    (ProtocolDirection::North, "north"),
    (ProtocolDirection::East, "east"),
    (ProtocolDirection::South, "south"),
    (ProtocolDirection::West, "west"),
];

fn pass() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
}

fn block_boolean_property(block: &BlockProbe, property: &str) -> Option<bool> {
    match block.block_properties.get(property).map(String::as_str) {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    }
}

fn block_has_type(block: &BlockProbe, expected_type_id: i32) -> Option<bool> {
    let name = block.block_name.as_deref()?;
    Some(vanilla_block_type_for_resource_id(name)?.id == expected_type_id)
}

fn opposite(direction: ProtocolDirection) -> ProtocolDirection {
    match direction {
        ProtocolDirection::Down => ProtocolDirection::Up,
        ProtocolDirection::Up => ProtocolDirection::Down,
        ProtocolDirection::North => ProtocolDirection::South,
        ProtocolDirection::South => ProtocolDirection::North,
        ProtocolDirection::West => ProtocolDirection::East,
        ProtocolDirection::East => ProtocolDirection::West,
    }
}

fn sturdy_face_index(direction: ProtocolDirection) -> usize {
    match direction {
        ProtocolDirection::West => 0,
        ProtocolDirection::South => 1,
        ProtocolDirection::East => 2,
        ProtocolDirection::North => 3,
        ProtocolDirection::Down => 4,
        ProtocolDirection::Up => 5,
    }
}

fn relative(pos: BlockPos, direction: ProtocolDirection) -> BlockPos {
    match direction {
        ProtocolDirection::Down => offset(pos, 0, -1, 0),
        ProtocolDirection::Up => offset(pos, 0, 1, 0),
        ProtocolDirection::North => offset(pos, 0, 0, -1),
        ProtocolDirection::South => offset(pos, 0, 0, 1),
        ProtocolDirection::West => offset(pos, -1, 0, 0),
        ProtocolDirection::East => offset(pos, 1, 0, 0),
    }
}

fn above(pos: BlockPos) -> BlockPos {
    offset(pos, 0, 1, 0)
}

fn below(pos: BlockPos) -> BlockPos {
    offset(pos, 0, -1, 0)
}

fn offset(pos: BlockPos, x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos {
        x: pos.x.saturating_add(x),
        y: pos.y.saturating_add(y),
        z: pos.z.saturating_add(z),
    }
}
