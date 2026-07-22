use bbb_pack::{builtin_block_piston_profiles, BlockPistonProfileCatalog, BlockPistonPushReaction};
use serde::{Deserialize, Serialize};

use crate::{block_name_is_air, BlockPos, WorldStore};

const MAX_PUSH_DEPTH: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PistonDirection {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PistonAxis {
    X,
    Y,
    Z,
}

impl PistonDirection {
    const VALUES: [Self; 6] = [
        Self::Down,
        Self::Up,
        Self::North,
        Self::South,
        Self::West,
        Self::East,
    ];

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "down" => Some(Self::Down),
            "up" => Some(Self::Up),
            "north" => Some(Self::North),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "east" => Some(Self::East),
            _ => None,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    pub fn step(self) -> [i32; 3] {
        match self {
            Self::Down => [0, -1, 0],
            Self::Up => [0, 1, 0],
            Self::North => [0, 0, -1],
            Self::South => [0, 0, 1],
            Self::West => [-1, 0, 0],
            Self::East => [1, 0, 0],
        }
    }

    fn axis(self) -> PistonAxis {
        match self {
            Self::Down | Self::Up => PistonAxis::Y,
            Self::North | Self::South => PistonAxis::Z,
            Self::West | Self::East => PistonAxis::X,
        }
    }

    pub fn relative(self, pos: BlockPos, distance: i32) -> BlockPos {
        let [x, y, z] = self.step();
        BlockPos {
            x: pos.x.wrapping_add(x.wrapping_mul(distance)),
            y: pos.y.wrapping_add(y.wrapping_mul(distance)),
            z: pos.z.wrapping_add(z.wrapping_mul(distance)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PistonStructureResolution {
    pub push_direction: PistonDirection,
    pub to_push: Vec<BlockPos>,
    pub to_destroy: Vec<BlockPos>,
}

impl WorldStore {
    /// Resolves one piston move without mutating chunks. This is a direct
    /// transcription of Vanilla's `PistonStructureResolver`; `None` is its
    /// `resolve() == false` result.
    pub fn resolve_piston_structure(
        &self,
        piston_pos: BlockPos,
        piston_direction: PistonDirection,
        extending: bool,
    ) -> Option<PistonStructureResolution> {
        let profiles = builtin_block_piston_profiles().ok()?;
        PistonStructureResolver::new(self, profiles, piston_pos, piston_direction, extending)
            .resolve()
    }
}

struct PistonStructureResolver<'a> {
    world: &'a WorldStore,
    profiles: &'a BlockPistonProfileCatalog,
    piston_pos: BlockPos,
    extending: bool,
    start_pos: BlockPos,
    push_direction: PistonDirection,
    piston_direction: PistonDirection,
    to_push: Vec<BlockPos>,
    to_destroy: Vec<BlockPos>,
}

impl<'a> PistonStructureResolver<'a> {
    fn new(
        world: &'a WorldStore,
        profiles: &'a BlockPistonProfileCatalog,
        piston_pos: BlockPos,
        piston_direction: PistonDirection,
        extending: bool,
    ) -> Self {
        let push_direction = if extending {
            piston_direction
        } else {
            piston_direction.opposite()
        };
        let start_pos = piston_direction.relative(piston_pos, if extending { 1 } else { 2 });
        Self {
            world,
            profiles,
            piston_pos,
            extending,
            start_pos,
            push_direction,
            piston_direction,
            to_push: Vec::new(),
            to_destroy: Vec::new(),
        }
    }

    fn resolve(mut self) -> Option<PistonStructureResolution> {
        if !self.is_pushable(
            self.start_pos,
            self.push_direction,
            false,
            self.piston_direction,
        ) {
            if self.extending
                && self.push_reaction(self.start_pos) == Some(BlockPistonPushReaction::Destroy)
            {
                self.to_destroy.push(self.start_pos);
            } else {
                return None;
            }
        } else {
            if !self.add_block_line(self.start_pos, self.push_direction) {
                return None;
            }
            let mut index = 0;
            while index < self.to_push.len() {
                let pos = self.to_push[index];
                if self.is_sticky(pos) && !self.add_branching_blocks(pos) {
                    return None;
                }
                index += 1;
            }
        }

        Some(PistonStructureResolution {
            push_direction: self.push_direction,
            to_push: self.to_push,
            to_destroy: self.to_destroy,
        })
    }

    fn add_block_line(&mut self, start: BlockPos, direction: PistonDirection) -> bool {
        if self.is_air(start) {
            return true;
        }
        if !self.is_pushable(start, self.push_direction, false, direction) {
            return true;
        }
        if start == self.piston_pos || self.to_push.contains(&start) {
            return true;
        }

        let mut block_count = 1usize;
        if block_count + self.to_push.len() > MAX_PUSH_DEPTH {
            return false;
        }

        let mut next_pos = start;
        while self.is_sticky(next_pos) {
            let pos = self
                .push_direction
                .opposite()
                .relative(start, block_count as i32);
            let previous_pos = next_pos;
            next_pos = pos;
            if self.is_air(pos)
                || !self.can_stick_to_each_other(previous_pos, pos)
                || !self.is_pushable(
                    pos,
                    self.push_direction,
                    false,
                    self.push_direction.opposite(),
                )
                || pos == self.piston_pos
            {
                break;
            }
            block_count += 1;
            if block_count + self.to_push.len() > MAX_PUSH_DEPTH {
                return false;
            }
        }

        let mut blocks_added = 0usize;
        for offset in (0..block_count).rev() {
            self.to_push.push(
                self.push_direction
                    .opposite()
                    .relative(start, offset as i32),
            );
            blocks_added += 1;
        }

        let mut distance = 1;
        loop {
            let pos = self.push_direction.relative(start, distance);
            if let Some(collision_pos) = self.to_push.iter().position(|candidate| *candidate == pos)
            {
                self.reorder_list_at_collision(blocks_added, collision_pos);
                let inclusive_end = collision_pos + blocks_added;
                for index in 0..=inclusive_end {
                    let branch_pos = self.to_push[index];
                    if self.is_sticky(branch_pos) && !self.add_branching_blocks(branch_pos) {
                        return false;
                    }
                }
                return true;
            }

            if self.is_air(pos) {
                return true;
            }
            if !self.is_pushable(pos, self.push_direction, true, self.push_direction)
                || pos == self.piston_pos
            {
                return false;
            }
            if self.push_reaction(pos) == Some(BlockPistonPushReaction::Destroy) {
                self.to_destroy.push(pos);
                return true;
            }
            if self.to_push.len() >= MAX_PUSH_DEPTH {
                return false;
            }
            self.to_push.push(pos);
            blocks_added += 1;
            distance += 1;
        }
    }

    fn reorder_list_at_collision(&mut self, blocks_added: usize, collision_pos: usize) {
        let last_line_start = self.to_push.len() - blocks_added;
        let head = self.to_push[..collision_pos].to_vec();
        let last_line = self.to_push[last_line_start..].to_vec();
        let collision_to_line = self.to_push[collision_pos..last_line_start].to_vec();
        self.to_push.clear();
        self.to_push.extend(head);
        self.to_push.extend(last_line);
        self.to_push.extend(collision_to_line);
    }

    fn add_branching_blocks(&mut self, from_pos: BlockPos) -> bool {
        for direction in PistonDirection::VALUES {
            if direction.axis() == self.push_direction.axis() {
                continue;
            }
            let neighbour_pos = direction.relative(from_pos, 1);
            if self.can_stick_to_each_other(neighbour_pos, from_pos)
                && !self.add_block_line(neighbour_pos, direction)
            {
                return false;
            }
        }
        true
    }

    fn is_pushable(
        &self,
        pos: BlockPos,
        direction: PistonDirection,
        allow_destroyable: bool,
        connection_direction: PistonDirection,
    ) -> bool {
        let max_y = self
            .world
            .dimension
            .min_y
            .saturating_add(self.world.dimension.height)
            .saturating_sub(1);
        if pos.y < self.world.dimension.min_y
            || pos.y > max_y
            || !self.world.world_border.contains_block_pos(pos)
        {
            return false;
        }
        if self.is_air(pos) {
            return true;
        }
        let Some(name) = self.block_name(pos) else {
            return false;
        };
        if matches!(
            name,
            "minecraft:obsidian"
                | "minecraft:crying_obsidian"
                | "minecraft:respawn_anchor"
                | "minecraft:reinforced_deepslate"
        ) {
            return false;
        }
        if (direction == PistonDirection::Down && pos.y == self.world.dimension.min_y)
            || (direction == PistonDirection::Up && pos.y == max_y)
        {
            return false;
        }

        let Some(profile) = self.profiles.profile(name) else {
            return false;
        };
        if matches!(name, "minecraft:piston" | "minecraft:sticky_piston") {
            if self
                .world
                .probe_block(pos)
                .and_then(|block| block.block_properties.get("extended").cloned())
                .as_deref()
                == Some("true")
            {
                return false;
            }
        } else {
            if profile.unbreakable {
                return false;
            }
            match profile.push_reaction {
                BlockPistonPushReaction::Block => return false,
                BlockPistonPushReaction::Destroy => return allow_destroyable,
                BlockPistonPushReaction::PushOnly => {
                    return direction == connection_direction;
                }
                BlockPistonPushReaction::Normal | BlockPistonPushReaction::Ignore => {}
            }
        }
        !profile.has_block_entity
    }

    fn block_name(&self, pos: BlockPos) -> Option<&str> {
        // This borrow cannot escape the temporary BlockProbe, so resolve from
        // the stable registry by state id instead.
        let state_id = self.world.probe_block(pos)?.block_state_id;
        self.world
            .registries
            .block_state(state_id)
            .map(|state| state.name.as_str())
    }

    fn is_air(&self, pos: BlockPos) -> bool {
        self.world
            .probe_block(pos)
            .is_none_or(|block| block.block_name.as_deref().is_some_and(block_name_is_air))
    }

    fn push_reaction(&self, pos: BlockPos) -> Option<BlockPistonPushReaction> {
        self.profiles
            .profile(self.block_name(pos)?)
            .map(|profile| profile.push_reaction)
    }

    fn is_sticky(&self, pos: BlockPos) -> bool {
        matches!(
            self.block_name(pos),
            Some("minecraft:slime_block" | "minecraft:honey_block")
        )
    }

    fn can_stick_to_each_other(&self, first: BlockPos, second: BlockPos) -> bool {
        let first = self.block_name(first);
        let second = self.block_name(second);
        if matches!(
            (first, second),
            (Some("minecraft:honey_block"), Some("minecraft:slime_block"))
                | (Some("minecraft:slime_block"), Some("minecraft:honey_block"))
        ) {
            return false;
        }
        matches!(
            first,
            Some("minecraft:slime_block" | "minecraft:honey_block")
        ) || matches!(
            second,
            Some("minecraft:slime_block" | "minecraft:honey_block")
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate};

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    crate::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str, properties: &[(&str, &str)]) {
        let properties = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>();
        let state_id = world
            .registries
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("missing state {name} {properties:?}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    #[test]
    fn resolves_linear_push_and_destroy_lists_in_vanilla_order() {
        let mut world = world_with_air_chunk();
        let piston = pos(1, 5, 5);
        set_block(&mut world, pos(2, 5, 5), "minecraft:stone", &[]);
        set_block(&mut world, pos(3, 5, 5), "minecraft:stone", &[]);
        set_block(&mut world, pos(4, 5, 5), "minecraft:torch", &[]);

        assert_eq!(
            world.resolve_piston_structure(piston, PistonDirection::East, true),
            Some(PistonStructureResolution {
                push_direction: PistonDirection::East,
                to_push: vec![pos(2, 5, 5), pos(3, 5, 5)],
                to_destroy: vec![pos(4, 5, 5)],
            })
        );
    }

    #[test]
    fn enforces_the_twelve_block_push_limit() {
        let mut world = world_with_air_chunk();
        let piston = pos(0, 5, 5);
        for x in 1..=12 {
            set_block(&mut world, pos(x, 5, 5), "minecraft:stone", &[]);
        }
        assert_eq!(
            world
                .resolve_piston_structure(piston, PistonDirection::East, true)
                .unwrap()
                .to_push
                .len(),
            12
        );

        set_block(&mut world, pos(13, 5, 5), "minecraft:stone", &[]);
        assert_eq!(
            world.resolve_piston_structure(piston, PistonDirection::East, true),
            None
        );
    }

    #[test]
    fn sticky_branching_follows_direction_order_and_slime_does_not_stick_to_honey() {
        let mut world = world_with_air_chunk();
        let piston = pos(1, 5, 5);
        set_block(&mut world, pos(2, 5, 5), "minecraft:slime_block", &[]);
        set_block(&mut world, pos(2, 4, 5), "minecraft:stone", &[]);
        set_block(&mut world, pos(2, 6, 5), "minecraft:stone", &[]);
        set_block(&mut world, pos(2, 5, 4), "minecraft:stone", &[]);
        set_block(&mut world, pos(2, 5, 6), "minecraft:honey_block", &[]);

        let resolved = world
            .resolve_piston_structure(piston, PistonDirection::East, true)
            .unwrap();
        assert_eq!(
            resolved.to_push,
            [pos(2, 5, 5), pos(2, 4, 5), pos(2, 6, 5), pos(2, 5, 4),]
        );
        assert!(!resolved.to_push.contains(&pos(2, 5, 6)));
    }

    #[test]
    fn push_only_block_moves_inline_but_not_as_a_sticky_side_branch() {
        let glazed = [("facing", "north")];
        let mut inline = world_with_air_chunk();
        set_block(
            &mut inline,
            pos(2, 5, 5),
            "minecraft:white_glazed_terracotta",
            &glazed,
        );
        assert_eq!(
            inline
                .resolve_piston_structure(pos(1, 5, 5), PistonDirection::East, true)
                .unwrap()
                .to_push,
            [pos(2, 5, 5)]
        );

        let mut branch = world_with_air_chunk();
        set_block(&mut branch, pos(2, 5, 5), "minecraft:slime_block", &[]);
        set_block(
            &mut branch,
            pos(2, 6, 5),
            "minecraft:white_glazed_terracotta",
            &glazed,
        );
        assert_eq!(
            branch
                .resolve_piston_structure(pos(1, 5, 5), PistonDirection::East, true)
                .unwrap()
                .to_push,
            [pos(2, 5, 5)]
        );
    }

    #[test]
    fn rejects_immovable_unbreakable_extended_and_block_entity_states() {
        for (name, properties) in [
            ("minecraft:obsidian", &[][..]),
            ("minecraft:bedrock", &[][..]),
            (
                "minecraft:sticky_piston",
                &[("extended", "true"), ("facing", "east")][..],
            ),
            (
                "minecraft:chest",
                &[
                    ("facing", "north"),
                    ("type", "single"),
                    ("waterlogged", "false"),
                ][..],
            ),
        ] {
            let mut world = world_with_air_chunk();
            set_block(&mut world, pos(2, 5, 5), name, properties);
            assert_eq!(
                world.resolve_piston_structure(pos(1, 5, 5), PistonDirection::East, true),
                None,
                "{name} should be immovable"
            );
        }
    }

    #[test]
    fn treats_unloaded_positions_as_air_but_rejects_unknown_loaded_states() {
        let unloaded = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        assert_eq!(
            unloaded.resolve_piston_structure(pos(1, 5, 5), PistonDirection::East, true),
            Some(PistonStructureResolution {
                push_direction: PistonDirection::East,
                to_push: Vec::new(),
                to_destroy: Vec::new(),
            })
        );

        let mut unknown = world_with_air_chunk();
        assert!(unknown.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 2, y: 5, z: 5 },
            block_state_id: i32::MAX,
        }));
        assert_eq!(
            unknown.resolve_piston_structure(pos(1, 5, 5), PistonDirection::East, true),
            None
        );
    }

    #[test]
    fn rejects_vertical_and_world_border_boundaries() {
        let mut vertical = world_with_air_chunk();
        set_block(&mut vertical, pos(5, 15, 5), "minecraft:stone", &[]);
        assert_eq!(
            vertical.resolve_piston_structure(pos(5, 14, 5), PistonDirection::Up, true),
            None
        );

        let mut border = world_with_air_chunk();
        border.world_border.center_x = 0.0;
        border.world_border.center_z = 0.0;
        border.world_border.size = 4.0;
        border.world_border.lerp_target = 4.0;
        border.world_border.current_size = 4.0;
        border.world_border.previous_size = 4.0;
        set_block(&mut border, pos(2, 5, 0), "minecraft:stone", &[]);
        assert_eq!(
            border.resolve_piston_structure(pos(1, 5, 0), PistonDirection::East, true),
            None
        );
    }

    #[test]
    fn sticky_retraction_pulls_normal_blocks_but_not_destroy_reaction_blocks() {
        let piston = pos(1, 5, 5);
        let mut normal = world_with_air_chunk();
        set_block(&mut normal, pos(3, 5, 5), "minecraft:stone", &[]);
        assert_eq!(
            normal.resolve_piston_structure(piston, PistonDirection::East, false),
            Some(PistonStructureResolution {
                push_direction: PistonDirection::West,
                to_push: vec![pos(3, 5, 5)],
                to_destroy: Vec::new(),
            })
        );

        let mut destroy = world_with_air_chunk();
        set_block(&mut destroy, pos(3, 5, 5), "minecraft:torch", &[]);
        assert_eq!(
            destroy.resolve_piston_structure(piston, PistonDirection::East, false),
            None
        );
    }
}
