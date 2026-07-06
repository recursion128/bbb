//! World -> renderer projection for bed block-entity models.
//!
//! Vanilla renders beds through the `BlockEntityRenderDispatcher` +
//! `BedRenderer` pair: per bed block entity, a `BedRenderState` carrying the
//! dye color, the block state's `FACING`/`PART`, and the light coords
//! combined with the other half (`DoubleBlockCombiner` + `BrightnessCombiner`
//! max). bbb has no separate BER dispatch; bed instances ride the existing
//! single entity-model submission stream as `EntityModelKind::Bed`, like the
//! chest and the sign.

use bbb_renderer::{BedModelPart, EntityDyeColor, EntityModelInstance};
use bbb_world::{
    BedColorKind as WorldBedColorKind, BedModelSourceState, BedPartKind as WorldBedPartKind,
    TerrainLight, WorldStore,
};

/// Like chests/signs, bed instances are projected from block states, not the
/// entity list, so they carry a sentinel id no server entity can use.
const BED_BLOCK_MODEL_ENTITY_ID: i32 = -1;

/// Projects every bed block in the loaded chunks into a bed model instance:
/// position at the block min corner, `180 + facing.toYRot()` as the
/// `BedRenderer.createModelTransform` Z-spin angle, and the block-position
/// light with the vanilla double-block `BrightnessCombiner` per-component
/// max.
pub(crate) fn bed_model_instances_from_world(world: &WorldStore) -> Vec<EntityModelInstance> {
    world
        .bed_model_source_states()
        .into_iter()
        .map(|source| bed_model_instance(&source, world))
        .collect()
}

fn bed_model_instance(source: &BedModelSourceState, world: &WorldStore) -> EntityModelInstance {
    let mut instance = EntityModelInstance::bed(
        BED_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        180.0 + source.facing.to_y_rot(),
        bed_model_color(source.color),
        bed_model_part(source.part),
    );
    let mut light = world.sample_block_light(source.pos);
    if let Some(partner_pos) = source.partner_pos {
        // Vanilla `BrightnessCombiner`: the bed's `extractRenderState` combine
        // renders both halves with `LightCoordsUtil.max(pack(first),
        // pack(second))`, i.e. the per-component max of the two samples.
        if let (Some(own), Some(partner)) = (light, world.sample_block_light(partner_pos)) {
            light = Some(TerrainLight {
                sky: own.sky.max(partner.sky),
                block: own.block.max(partner.block),
            });
        }
    }
    if let Some(light) = light {
        instance = instance.with_light_coords(bed_light_coords(light));
    }
    instance
}

fn bed_model_color(color: WorldBedColorKind) -> EntityDyeColor {
    match color {
        WorldBedColorKind::White => EntityDyeColor::White,
        WorldBedColorKind::Orange => EntityDyeColor::Orange,
        WorldBedColorKind::Magenta => EntityDyeColor::Magenta,
        WorldBedColorKind::LightBlue => EntityDyeColor::LightBlue,
        WorldBedColorKind::Yellow => EntityDyeColor::Yellow,
        WorldBedColorKind::Lime => EntityDyeColor::Lime,
        WorldBedColorKind::Pink => EntityDyeColor::Pink,
        WorldBedColorKind::Gray => EntityDyeColor::Gray,
        WorldBedColorKind::LightGray => EntityDyeColor::LightGray,
        WorldBedColorKind::Cyan => EntityDyeColor::Cyan,
        WorldBedColorKind::Purple => EntityDyeColor::Purple,
        WorldBedColorKind::Blue => EntityDyeColor::Blue,
        WorldBedColorKind::Brown => EntityDyeColor::Brown,
        WorldBedColorKind::Green => EntityDyeColor::Green,
        WorldBedColorKind::Red => EntityDyeColor::Red,
        WorldBedColorKind::Black => EntityDyeColor::Black,
    }
}

fn bed_model_part(part: WorldBedPartKind) -> BedModelPart {
    match part {
        WorldBedPartKind::Head => BedModelPart::Head,
        WorldBedPartKind::Foot => BedModelPart::Foot,
    }
}

/// Vanilla `LightCoordsUtil.pack(block, sky)` (`block << 4 | sky << 20`) over
/// the raw stored light sample — the bed render state's `lightCoords`.
fn bed_light_coords(light: TerrainLight) -> u32 {
    u32::from(light.block.min(15)) << 4 | u32::from(light.sky.min(15)) << 20
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn set_bed(world: &mut WorldStore, pos: BlockPos, name: &str, facing: &str, part: &str) {
        let properties: BTreeMap<String, String> = [
            ("facing".to_string(), facing.to_string()),
            ("part".to_string(), part.to_string()),
            ("occupied".to_string(), "false".to_string()),
        ]
        .into_iter()
        .collect();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name} {properties:?}"));
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_state_id: state_id,
            })
        );
    }

    #[test]
    fn projects_bed_halves_with_color_part_and_facing_angle() {
        let mut world = world_with_air_chunk();
        // A south-facing red bed: foot at z=5, head toward facing at z=6.
        let foot_pos = BlockPos { x: 3, y: 4, z: 5 };
        let head_pos = BlockPos { x: 3, y: 4, z: 6 };
        set_bed(&mut world, foot_pos, "minecraft:red_bed", "south", "foot");
        set_bed(&mut world, head_pos, "minecraft:red_bed", "south", "head");

        let instances = bed_model_instances_from_world(&world);
        assert_eq!(instances.len(), 2);
        let foot = &instances[0];
        let head = &instances[1];
        assert_eq!(foot.entity_id, BED_BLOCK_MODEL_ENTITY_ID);
        assert_eq!(foot.position, [3.0, 4.0, 5.0]);
        assert_eq!(
            foot.kind,
            EntityModelKind::Bed {
                color: EntityDyeColor::Red,
                part: BedModelPart::Foot,
            }
        );
        assert_eq!(
            head.kind,
            EntityModelKind::Bed {
                color: EntityDyeColor::Red,
                part: BedModelPart::Head,
            }
        );
        // SOUTH: toYRot = 0 -> the Rz spin angle is 180 + 0.
        assert_eq!(foot.render_state.body_rot, 180.0);
        assert_eq!(head.render_state.body_rot, 180.0);
        // Empty light data: sky falls back to 15, block to 0 -> pack(0, 15).
        assert_eq!(foot.render_state.light_coords, 15 << 20);
    }

    #[test]
    fn projects_facing_angles_like_vanilla_to_y_rot() {
        let mut world = world_with_air_chunk();
        set_bed(
            &mut world,
            BlockPos { x: 1, y: 2, z: 3 },
            "minecraft:lime_bed",
            "north",
            "foot",
        );
        set_bed(
            &mut world,
            BlockPos { x: 3, y: 2, z: 3 },
            "minecraft:blue_bed",
            "west",
            "foot",
        );
        set_bed(
            &mut world,
            BlockPos { x: 5, y: 2, z: 3 },
            "minecraft:black_bed",
            "east",
            "foot",
        );
        let instances = bed_model_instances_from_world(&world);
        assert_eq!(instances.len(), 3);
        // NORTH toYRot = 180 -> 360; WEST 90 -> 270; EAST 270 -> 450.
        assert_eq!(instances[0].render_state.body_rot, 360.0);
        assert_eq!(instances[1].render_state.body_rot, 270.0);
        assert_eq!(instances[2].render_state.body_rot, 450.0);
        assert_eq!(
            instances[0].kind,
            EntityModelKind::Bed {
                color: EntityDyeColor::Lime,
                part: BedModelPart::Foot,
            }
        );
        assert_eq!(
            instances[1].kind,
            EntityModelKind::Bed {
                color: EntityDyeColor::Blue,
                part: BedModelPart::Foot,
            }
        );
        assert_eq!(
            instances[2].kind,
            EntityModelKind::Bed {
                color: EntityDyeColor::Black,
                part: BedModelPart::Foot,
            }
        );
    }

    #[test]
    fn packs_bed_light_coords_like_vanilla() {
        assert_eq!(
            bed_light_coords(TerrainLight { sky: 15, block: 0 }),
            15 << 20
        );
        assert_eq!(
            bed_light_coords(TerrainLight { sky: 7, block: 9 }),
            9 << 4 | 7 << 20
        );
        assert_eq!(
            bed_light_coords(TerrainLight {
                sky: 200,
                block: 200
            }),
            15 << 4 | 15 << 20
        );
    }
}
