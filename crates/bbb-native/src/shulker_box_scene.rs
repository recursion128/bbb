//! World -> renderer projection for shulker box block-entity models.
//!
//! Vanilla renders shulker boxes through the `BlockEntityRenderDispatcher` +
//! `ShulkerBoxRenderer` pair: per box block entity, a `ShulkerBoxRenderState`
//! carrying the block state's six-way `FACING`, the block-id dye color
//! (`Sheets.getShulkerBoxSprite`), the lerped lid progress
//! (`ShulkerBoxBlockEntity.getProgress(partialTicks)`), and the light coords
//! sampled at the block position. bbb has no separate BER dispatch; box
//! instances ride the existing single entity-model submission stream as
//! `EntityModelKind::ShulkerBox`, like the chest and the bell.

use bbb_renderer::{EntityAttachmentFace, EntityDyeColor, EntityModelInstance};
use bbb_world::{
    ShulkerBoxColorKind as WorldShulkerBoxColorKind, ShulkerBoxFacing as WorldShulkerBoxFacing,
    ShulkerBoxModelSourceState, TerrainLight, WorldStore,
};

/// Like chests/bells, shulker box instances are projected from block states,
/// not the entity list, so they carry a sentinel id no server entity can use.
const SHULKER_BOX_BLOCK_MODEL_ENTITY_ID: i32 = -1;

/// Projects every shulker box block in the loaded chunks into a shulker box
/// model instance: position at the block min corner, the block-id color and
/// six-way facing (folded into the root transform by the renderer via
/// `Direction.getRotation()`), the lerped lid progress, and the
/// block-position light.
pub(crate) fn shulker_box_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    partial_tick: f32,
) -> Vec<EntityModelInstance> {
    world
        .shulker_box_model_source_states(partial_tick)
        .into_iter()
        .map(|source| shulker_box_model_instance(&source, world))
        .collect()
}

fn shulker_box_model_instance(
    source: &ShulkerBoxModelSourceState,
    world: &WorldStore,
) -> EntityModelInstance {
    let mut instance = EntityModelInstance::shulker_box(
        SHULKER_BOX_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        source.color.map(shulker_box_dye_color),
        shulker_box_facing(source.facing),
    )
    .with_shulker_box_progress(source.progress);
    if let Some(light) = world.sample_block_light(source.pos) {
        instance = instance.with_light_coords(shulker_box_light_coords(light));
    }
    instance
}

fn shulker_box_dye_color(color: WorldShulkerBoxColorKind) -> EntityDyeColor {
    match color {
        WorldShulkerBoxColorKind::White => EntityDyeColor::White,
        WorldShulkerBoxColorKind::Orange => EntityDyeColor::Orange,
        WorldShulkerBoxColorKind::Magenta => EntityDyeColor::Magenta,
        WorldShulkerBoxColorKind::LightBlue => EntityDyeColor::LightBlue,
        WorldShulkerBoxColorKind::Yellow => EntityDyeColor::Yellow,
        WorldShulkerBoxColorKind::Lime => EntityDyeColor::Lime,
        WorldShulkerBoxColorKind::Pink => EntityDyeColor::Pink,
        WorldShulkerBoxColorKind::Gray => EntityDyeColor::Gray,
        WorldShulkerBoxColorKind::LightGray => EntityDyeColor::LightGray,
        WorldShulkerBoxColorKind::Cyan => EntityDyeColor::Cyan,
        WorldShulkerBoxColorKind::Purple => EntityDyeColor::Purple,
        WorldShulkerBoxColorKind::Blue => EntityDyeColor::Blue,
        WorldShulkerBoxColorKind::Brown => EntityDyeColor::Brown,
        WorldShulkerBoxColorKind::Green => EntityDyeColor::Green,
        WorldShulkerBoxColorKind::Red => EntityDyeColor::Red,
        WorldShulkerBoxColorKind::Black => EntityDyeColor::Black,
    }
}

fn shulker_box_facing(facing: WorldShulkerBoxFacing) -> EntityAttachmentFace {
    match facing {
        WorldShulkerBoxFacing::Down => EntityAttachmentFace::Down,
        WorldShulkerBoxFacing::Up => EntityAttachmentFace::Up,
        WorldShulkerBoxFacing::North => EntityAttachmentFace::North,
        WorldShulkerBoxFacing::South => EntityAttachmentFace::South,
        WorldShulkerBoxFacing::West => EntityAttachmentFace::West,
        WorldShulkerBoxFacing::East => EntityAttachmentFace::East,
    }
}

/// Vanilla `LightCoordsUtil.pack(block, sky)` (`block << 4 | sky << 20`) over
/// the raw stored light sample — the shulker box render state's
/// `lightCoords`.
fn shulker_box_light_coords(light: TerrainLight) -> u32 {
    u32::from(light.block.min(15)) << 4 | u32::from(light.sky.min(15)) << 20
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos};
    use bbb_renderer::EntityModelKind;
    use bbb_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension,
    };
    use std::collections::BTreeMap;

    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;

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
                    VANILLA_AIR_BLOCK_STATE_ID,
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

    fn set_shulker_box(world: &mut WorldStore, pos: BlockPos, name: &str, facing: &str) {
        let properties: BTreeMap<String, String> = [("facing".to_string(), facing.to_string())]
            .into_iter()
            .collect();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name} {properties:?}"));
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_state_id: state_id,
            })
        );
    }

    fn send_open_count(world: &mut WorldStore, pos: BlockPos, count: u8) {
        world.apply_block_event(ProtocolBlockEvent {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            b0: 1,
            b1: count,
            block_id: 0,
        });
    }

    #[test]
    fn projects_opening_and_resting_boxes_with_color_facing_and_light() {
        let mut world = world_with_air_chunk();
        let opening_pos = BlockPos { x: 3, y: 4, z: 5 };
        let resting_pos = BlockPos { x: 6, y: 4, z: 5 };
        set_shulker_box(&mut world, opening_pos, "minecraft:shulker_box", "north");
        set_shulker_box(
            &mut world,
            resting_pos,
            "minecraft:magenta_shulker_box",
            "down",
        );
        send_open_count(&mut world, opening_pos, 1);
        world.advance_shulker_box_lid_ticks(2);

        let instances = shulker_box_model_instances_from_world_at_partial_tick(&world, 0.5);
        assert_eq!(instances.len(), 2);
        let opening = &instances[0];
        let resting = &instances[1];
        assert_eq!(
            opening.kind,
            EntityModelKind::ShulkerBox {
                color: None,
                facing: EntityAttachmentFace::North,
            }
        );
        assert_eq!(opening.entity_id, SHULKER_BOX_BLOCK_MODEL_ENTITY_ID);
        assert_eq!(opening.position, [3.0, 4.0, 5.0]);
        // getProgress(0.5) = lerp(0.5, 0.1, 0.2); the facing rides the kind,
        // not the yaw.
        let expected = 0.1 + (0.2 - 0.1) * 0.5;
        assert!((opening.render_state.shulker_box_progress - expected).abs() < 1e-6);
        assert_eq!(opening.render_state.body_rot, 0.0);
        // Empty light data: sky falls back to 15, block to 0 -> pack(0, 15).
        assert_eq!(opening.render_state.light_coords, 15 << 20);
        assert_eq!(
            resting.kind,
            EntityModelKind::ShulkerBox {
                color: Some(EntityDyeColor::Magenta),
                facing: EntityAttachmentFace::Down,
            }
        );
        assert_eq!(resting.render_state.shulker_box_progress, 0.0);
    }

    #[test]
    fn packs_shulker_box_light_coords_like_vanilla() {
        assert_eq!(
            shulker_box_light_coords(TerrainLight { sky: 15, block: 0 }),
            15 << 20
        );
        assert_eq!(
            shulker_box_light_coords(TerrainLight { sky: 7, block: 9 }),
            9 << 4 | 7 << 20
        );
    }
}
