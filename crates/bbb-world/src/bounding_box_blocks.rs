//! Client-side `BoundingBoxRenderable` projection.
//!
//! `BlockEntityWithBoundingBoxRenderer` draws one gizmo cuboid per block entity
//! implementing `BoundingBoxRenderable`, plus per-block markers for the
//! invisible blocks inside the box when the owner asks for them. The mode and
//! box come entirely from decoded block-entity state, so this module owns the
//! two vanilla implementations (`StructureBlockEntity`, `TestInstanceBlockEntity`)
//! and the shared invisible-block classification.

use bbb_protocol::{
    block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
        VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
    },
    packets::{
        StructureBlockMirror, StructureBlockMode, StructureBlockRotation, TestInstanceBlockRotation,
    },
};

use crate::{
    BlockPos, StructureBlockEntityDataState, TestInstanceBlockEntityDataState, WorldStore,
};

/// `BoundingBoxRenderable.Mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundingBoxRenderMode {
    None,
    Box,
    BoxAndInvisibleBlocks,
}

/// `BoundingBoxRenderable.RenderableBox`: an offset from the block entity plus
/// a size, both already normalised by `fromCorners`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderableBoxState {
    pub local_pos: BlockPos,
    pub size: [i32; 3],
}

impl RenderableBoxState {
    /// `BoundingBoxRenderable.RenderableBox.fromCorners`.
    fn from_corners(first: [i32; 3], second: [i32; 3]) -> Self {
        let min = [
            first[0].min(second[0]),
            first[1].min(second[1]),
            first[2].min(second[2]),
        ];
        Self {
            local_pos: BlockPos {
                x: min[0],
                y: min[1],
                z: min[2],
            },
            size: [
                first[0].max(second[0]) - min[0],
                first[1].max(second[1]) - min[1],
                first[2].max(second[2]) - min[2],
            ],
        }
    }

    /// `submit` skips degenerate boxes before drawing anything.
    pub fn is_drawable(&self) -> bool {
        self.size.iter().all(|extent| *extent >= 1)
    }
}

/// `BlockEntityWithBoundingBoxRenderState.InvisibleBlockType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvisibleBlockType {
    Air,
    StructureVoid,
    Barrier,
    Light,
}

/// One extracted `BlockEntityWithBoundingBoxRenderState`.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundingBoxRenderableSourceState {
    pub pos: BlockPos,
    /// `true` when the owner is a test-instance block, whose
    /// `TestInstanceRenderer` widens the bounding-box `shouldRender` gate.
    pub owner_is_test_instance: bool,
    pub mode: BoundingBoxRenderMode,
    pub renderable_box: RenderableBoxState,
    /// `state.invisibleBlocks`, indexed `z * sizeX * sizeY + y * sizeX + x`.
    /// Empty for every mode but `BoxAndInvisibleBlocks`.
    pub invisible_blocks: Vec<Option<InvisibleBlockType>>,
}

impl WorldStore {
    /// `BlockEntityWithBoundingBoxRenderer.extract` over every loaded owner.
    /// The whole surface is gated on `state.isVisible`, so a player who can
    /// neither use game-master blocks nor spectate sees nothing.
    pub fn bounding_box_renderable_source_states(&self) -> Vec<BoundingBoxRenderableSourceState> {
        if !self.local_player_can_use_gamemaster_blocks() && !self.local_player_is_spectator() {
            return Vec::new();
        }
        let mut states = Vec::new();
        for (pos, record) in self.loaded_block_entity_records() {
            let (mode, renderable_box, owner_is_test_instance) = match record.type_id {
                VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID => {
                    let Some(data) = record.structure_block.as_ref() else {
                        continue;
                    };
                    (
                        structure_block_render_mode(data),
                        structure_block_renderable_box(data),
                        false,
                    )
                }
                VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID => {
                    let Some(data) = record.test_instance_block.as_ref() else {
                        continue;
                    };
                    (
                        // `TestInstanceBlockEntity.renderMode`.
                        BoundingBoxRenderMode::Box,
                        self.test_instance_renderable_box(data),
                        true,
                    )
                }
                _ => continue,
            };
            let invisible_blocks = if mode == BoundingBoxRenderMode::BoxAndInvisibleBlocks {
                self.invisible_blocks_in(pos, renderable_box)
            } else {
                Vec::new()
            };
            states.push(BoundingBoxRenderableSourceState {
                pos,
                owner_is_test_instance,
                mode,
                renderable_box,
                invisible_blocks,
            });
        }
        states.sort_by_key(|state| state.pos);
        states
    }

    /// The `BOX_AND_INVISIBLE_BLOCKS` scan: classify every position in the box
    /// that vanilla renders no geometry for.
    fn invisible_blocks_in(
        &self,
        pos: BlockPos,
        renderable_box: RenderableBoxState,
    ) -> Vec<Option<InvisibleBlockType>> {
        let [size_x, size_y, size_z] = renderable_box.size;
        let Ok(len) = usize::try_from(size_x.max(0) * size_y.max(0) * size_z.max(0)) else {
            return Vec::new();
        };
        let start = BlockPos {
            x: pos.x + renderable_box.local_pos.x,
            y: pos.y + renderable_box.local_pos.y,
            z: pos.z + renderable_box.local_pos.z,
        };
        let mut blocks = vec![None; len];
        for x in 0..size_x {
            for y in 0..size_y {
                for z in 0..size_z {
                    let index = (z * size_x * size_y + y * size_x + x) as usize;
                    blocks[index] = self.invisible_block_type_at(BlockPos {
                        x: start.x + x,
                        y: start.y + y,
                        z: start.z + z,
                    });
                }
            }
        }
        blocks
    }

    fn invisible_block_type_at(&self, pos: BlockPos) -> Option<InvisibleBlockType> {
        let block = self.probe_block(pos)?;
        match block.block_name.as_deref()? {
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air" => {
                Some(InvisibleBlockType::Air)
            }
            "minecraft:structure_void" => Some(InvisibleBlockType::StructureVoid),
            "minecraft:barrier" => Some(InvisibleBlockType::Barrier),
            "minecraft:light" => Some(InvisibleBlockType::Light),
            _ => None,
        }
    }

    /// `TestInstanceBlockEntity.getRenderableBox`: the fixed `STRUCTURE_OFFSET`
    /// grown by the test's padding, sized by the rotated structure size.
    fn test_instance_renderable_box(
        &self,
        data: &TestInstanceBlockEntityDataState,
    ) -> RenderableBoxState {
        let padding = self.test_instance_padding(data);
        let rotation = self.test_instance_rotation(data);
        let axes_switched = matches!(
            rotation,
            TestInstanceBlockRotation::Clockwise90 | TestInstanceBlockRotation::Counterclockwise90
        );
        RenderableBoxState {
            local_pos: BlockPos {
                x: TEST_INSTANCE_STRUCTURE_OFFSET[0] + padding,
                y: TEST_INSTANCE_STRUCTURE_OFFSET[1] + padding,
                z: TEST_INSTANCE_STRUCTURE_OFFSET[2] + padding,
            },
            size: [
                if axes_switched {
                    data.size.z
                } else {
                    data.size.x
                },
                data.size.y,
                if axes_switched {
                    data.size.x
                } else {
                    data.size.z
                },
            ],
        }
    }
}

/// `TestInstanceBlockEntity.STRUCTURE_OFFSET`.
const TEST_INSTANCE_STRUCTURE_OFFSET: [i32; 3] = [0, 1, 1];

/// `StructureBlockEntity.renderMode`.
fn structure_block_render_mode(data: &StructureBlockEntityDataState) -> BoundingBoxRenderMode {
    match data.mode {
        StructureBlockMode::Save if data.show_air => BoundingBoxRenderMode::BoxAndInvisibleBlocks,
        StructureBlockMode::Save => BoundingBoxRenderMode::Box,
        StructureBlockMode::Load if data.show_bounding_box => BoundingBoxRenderMode::Box,
        _ => BoundingBoxRenderMode::None,
    }
}

/// `StructureBlockEntity.getRenderableBox`: the saved offset/size rotated and
/// mirrored around the block entity.
fn structure_block_renderable_box(data: &StructureBlockEntityDataState) -> RenderableBoxState {
    let origin = data.offset;
    let y0 = origin.y;
    let y1 = y0 + data.size.y;
    let (x_diff, z_diff) = match data.mirror {
        StructureBlockMirror::LeftRight => (data.size.x, -data.size.z),
        StructureBlockMirror::FrontBack => (-data.size.x, data.size.z),
        StructureBlockMirror::None => (data.size.x, data.size.z),
    };
    let (x0, z0, x1, z1) = match data.rotation {
        StructureBlockRotation::Clockwise90 => {
            let x0 = if z_diff < 0 { origin.x } else { origin.x + 1 };
            let z0 = if x_diff < 0 { origin.z + 1 } else { origin.z };
            (x0, z0, x0 - z_diff, z0 + x_diff)
        }
        StructureBlockRotation::Clockwise180 => {
            let x0 = if x_diff < 0 { origin.x } else { origin.x + 1 };
            let z0 = if z_diff < 0 { origin.z } else { origin.z + 1 };
            (x0, z0, x0 - x_diff, z0 - z_diff)
        }
        StructureBlockRotation::Counterclockwise90 => {
            let x0 = if z_diff < 0 { origin.x + 1 } else { origin.x };
            let z0 = if x_diff < 0 { origin.z } else { origin.z + 1 };
            (x0, z0, x0 + z_diff, z0 - x_diff)
        }
        StructureBlockRotation::None => {
            let x0 = if x_diff < 0 { origin.x + 1 } else { origin.x };
            let z0 = if z_diff < 0 { origin.z + 1 } else { origin.z };
            (x0, z0, x0 + x_diff, z0 + z_diff)
        }
    };
    RenderableBoxState::from_corners([x0, y0, z0], [x1, y1, z1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StructureBlockSizeState;

    fn save_block(size: [i32; 3]) -> StructureBlockEntityDataState {
        StructureBlockEntityDataState {
            mode: StructureBlockMode::Save,
            offset: BlockPos { x: 0, y: 1, z: 0 },
            size: StructureBlockSizeState {
                x: size[0],
                y: size[1],
                z: size[2],
            },
            ..StructureBlockEntityDataState::default()
        }
    }

    #[test]
    fn structure_render_mode_follows_the_save_load_split() {
        let mut data = save_block([3, 4, 5]);
        assert_eq!(
            structure_block_render_mode(&data),
            BoundingBoxRenderMode::Box
        );
        data.show_air = true;
        assert_eq!(
            structure_block_render_mode(&data),
            BoundingBoxRenderMode::BoxAndInvisibleBlocks
        );
        // `showBoundingBox` only gates LOAD; SAVE always draws.
        data.show_bounding_box = false;
        assert_eq!(
            structure_block_render_mode(&data),
            BoundingBoxRenderMode::BoxAndInvisibleBlocks
        );

        data.mode = StructureBlockMode::Load;
        data.show_air = false;
        assert_eq!(
            structure_block_render_mode(&data),
            BoundingBoxRenderMode::None
        );
        data.show_bounding_box = true;
        assert_eq!(
            structure_block_render_mode(&data),
            BoundingBoxRenderMode::Box
        );

        for mode in [StructureBlockMode::Corner, StructureBlockMode::Data] {
            data.mode = mode;
            assert_eq!(
                structure_block_render_mode(&data),
                BoundingBoxRenderMode::None
            );
        }
    }

    #[test]
    fn unrotated_structure_box_is_the_offset_plus_size() {
        let renderable = structure_block_renderable_box(&save_block([3, 4, 5]));
        assert_eq!(renderable.local_pos, BlockPos { x: 0, y: 1, z: 0 });
        assert_eq!(renderable.size, [3, 4, 5]);
        assert!(renderable.is_drawable());
    }

    #[test]
    fn rotation_swaps_the_horizontal_extents_and_shifts_the_corner() {
        let mut data = save_block([3, 4, 5]);
        data.rotation = StructureBlockRotation::Clockwise90;
        let renderable = structure_block_renderable_box(&data);
        // `x0 = origin.x + 1`, `x1 = x0 - zDiff` → the box spans x ∈ [-4, 1].
        assert_eq!(renderable.local_pos, BlockPos { x: -4, y: 1, z: 0 });
        assert_eq!(renderable.size, [5, 4, 3]);

        data.rotation = StructureBlockRotation::Counterclockwise90;
        let renderable = structure_block_renderable_box(&data);
        // `z0 = origin.z + 1`, `z1 = z0 - xDiff` → the box spans z ∈ [-2, 1].
        assert_eq!(renderable.local_pos, BlockPos { x: 0, y: 1, z: -2 });
        assert_eq!(renderable.size, [5, 4, 3]);
    }

    #[test]
    fn mirroring_flips_the_matching_axis_before_rotation() {
        let mut data = save_block([3, 4, 5]);
        data.mirror = StructureBlockMirror::LeftRight;
        let renderable = structure_block_renderable_box(&data);
        // `zDiff = -5` moves the origin corner to `z + 1` and spans backwards.
        assert_eq!(renderable.local_pos, BlockPos { x: 0, y: 1, z: -4 });
        assert_eq!(renderable.size, [3, 4, 5]);

        data.mirror = StructureBlockMirror::FrontBack;
        let renderable = structure_block_renderable_box(&data);
        assert_eq!(renderable.local_pos, BlockPos { x: -2, y: 1, z: 0 });
        assert_eq!(renderable.size, [3, 4, 5]);
    }

    #[test]
    fn extraction_is_gated_on_game_master_access_and_scans_invisible_blocks() {
        let mut world = structure_block_world(StructureBlockEntityDataState {
            show_air: true,
            ..save_block([2, 1, 1])
        });
        // `state.isVisible` is false for an ordinary survival player.
        assert!(world.bounding_box_renderable_source_states().is_empty());

        grant_gamemaster_block_access(&mut world);
        let states = world.bounding_box_renderable_source_states();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].mode, BoundingBoxRenderMode::BoxAndInvisibleBlocks);
        assert_eq!(states[0].renderable_box.size, [2, 1, 1]);
        // The box starts one block above the structure block: air then barrier.
        assert_eq!(
            states[0].invisible_blocks,
            vec![
                Some(InvisibleBlockType::Air),
                Some(InvisibleBlockType::Barrier)
            ]
        );
    }

    #[test]
    fn boxes_without_the_invisible_block_mode_carry_no_scan() {
        let mut world = structure_block_world(save_block([2, 1, 1]));
        grant_gamemaster_block_access(&mut world);
        let states = world.bounding_box_renderable_source_states();
        assert_eq!(states[0].mode, BoundingBoxRenderMode::Box);
        assert!(states[0].invisible_blocks.is_empty());
    }

    fn structure_block_world(data: StructureBlockEntityDataState) -> WorldStore {
        let mut store = WorldStore::new();
        store.dimension = crate::WorldDimension {
            min_y: 0,
            height: 16,
        };
        let air = crate::vanilla_air_block_state_id();
        let barrier = first_state_id("minecraft:barrier");
        let structure_block = first_state_id("minecraft:structure_block");
        let mut values = vec![air; 4096];
        // The structure block sits at (0, 0, 0); its box covers (0, 1, 0)-(1, 1, 0).
        values[0] = structure_block;
        values[256 + 1] = barrier;

        let mut palette_ids = values.clone();
        palette_ids.sort_unstable();
        palette_ids.dedup();
        let mut packed = vec![0_i64; 4096 / 2];
        for (index, value) in values.iter().enumerate() {
            let palette_index = palette_ids.iter().position(|id| id == value).unwrap() as u64;
            packed[index / 2] |= (palette_index << ((index % 2) * 32)) as i64;
        }
        store.insert_decoded_chunk(crate::ChunkColumn {
            pos: crate::ChunkPos { x: 0, z: 0 },
            state: crate::ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![crate::ChunkSection {
                non_empty_block_count: 2,
                fluid_count: 0,
                block_states: crate::PalettedContainerData {
                    domain: crate::PaletteDomain::BlockStates,
                    bits_per_entry: 32,
                    palette_kind: crate::PaletteKind::Local,
                    palette_global_ids: palette_ids,
                    packed_data: packed,
                    entry_count: 4096,
                },
                biomes: crate::PalettedContainerData {
                    domain: crate::PaletteDomain::Biomes,
                    bits_per_entry: 0,
                    palette_kind: crate::PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 64,
                },
            }],
            block_entities: vec![crate::BlockEntityRecord {
                structure_block: Some(data),
                ..crate::BlockEntityRecord::empty(
                    0,
                    0,
                    0,
                    VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
                )
            }],
            light: crate::LightData::default(),
        });
        store
    }

    /// The registry's first state for a block, whatever its property defaults.
    fn first_state_id(name: &str) -> i32 {
        crate::BlockStateRegistry::vanilla_26_1()
            .iter()
            .find(|state| state.name == name)
            .unwrap_or_else(|| panic!("no registered state for {name}"))
            .id
    }

    fn grant_gamemaster_block_access(store: &mut WorldStore) {
        store.local_player_id = Some(7);
        assert!(
            store.apply_entity_event(bbb_protocol::packets::EntityEvent {
                entity_id: 7,
                event_id: 26,
            })
        );
        store.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
    }

    #[test]
    fn empty_structures_are_not_drawable() {
        assert!(!structure_block_renderable_box(&save_block([0, 4, 5])).is_drawable());
        assert!(!structure_block_renderable_box(&save_block([3, 0, 5])).is_drawable());
    }
}
