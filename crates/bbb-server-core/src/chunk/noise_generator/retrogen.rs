use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_BEDROCK_ID, VANILLA_BLOCK_DEEPSLATE_ID,
};
use bbb_server_persistence::chunk_schema::Java26BelowZeroRetrogen;
use bbb_server_persistence::Java26ChunkStatus;
use bbb_server_types::chunk_data::ChunkColumn;
use bbb_server_types::save::SaveResult;
use bbb_server_worldgen::noise::beardifier::JigsawBeardifier;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;

use super::{
    invalid_schema, prime_worldgen_heightmaps, NoiseFinalDensityProgram, NoiseGeneratedChunk,
};
use crate::chunk::blending::BlendingResolver;

const RETROGEN_MIN_Y: i32 = -64;
const RETROGEN_MAX_Y: i32 = 0;

impl NoiseFinalDensityProgram {
    pub(crate) fn filter_retrogen_structure_starts(
        &self,
        mut starts: NoiseStructureChunkData,
    ) -> SaveResult<NoiseStructureChunkData> {
        starts.retain_starts_within_y(RETROGEN_MIN_Y, RETROGEN_MAX_Y);
        Ok(starts)
    }

    pub(crate) fn resume_below_zero_retrogen_to_carvers(
        &self,
        saved: super::SavedChunkResume<'_>,
        retrogen: &Java26BelowZeroRetrogen,
        beardifier: &JigsawBeardifier,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<NoiseGeneratedChunk> {
        let super::SavedChunkResume {
            chunk: [chunk_x, chunk_z],
            saved_status,
            saved_column,
            saved_postprocessing,
        } = saved;
        let path = format!(
            "noise_settings.{}.below_zero_retrogen[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        if self.dimension_type_id != "minecraft:overworld"
            || self.source.min_y() > RETROGEN_MIN_Y
            || self.source.min_y().saturating_add(self.source.height()) < RETROGEN_MAX_Y
        {
            return Err(invalid_schema(
                &path,
                "below-zero retrogen requires an Overworld noise shape containing -64..0",
            ));
        }

        let mut chunk = if saved_status < Java26ChunkStatus::Noise {
            let mut generated = self.generate_fresh_base_chunk_with_generation_shape(
                chunk_x,
                chunk_z,
                beardifier,
                biome_blender,
                Some((RETROGEN_MIN_Y, RETROGEN_MAX_Y - RETROGEN_MIN_Y)),
            )?;
            let fresh = generated.column.clone();
            generated.column = saved_column.clone();
            for y in RETROGEN_MIN_Y..RETROGEN_MAX_Y {
                for local_z in 0..16 {
                    for local_x in 0..16 {
                        let state =
                            fresh.get_block_state(local_x, y, local_z).ok_or_else(|| {
                                invalid_schema(
                                    &path,
                                    "generated retrogen block range is incomplete",
                                )
                            })?;
                        generated
                            .column
                            .set_block_state(local_x, y, local_z, state)
                            .ok_or_else(|| {
                                invalid_schema(&path, "saved retrogen block range is incomplete")
                            })?;
                    }
                }
            }
            if saved_status < Java26ChunkStatus::Biomes {
                // `BelowZeroRetrogen.getBiomeResolver` (`BelowZeroRetrogen.java:98-107`).
                for quart_y in (RETROGEN_MIN_Y / 4)..(RETROGEN_MAX_Y / 4) {
                    for local_x in 0..4 {
                        for local_z in 0..4 {
                            let generated_biome = fresh
                                .get_noise_biome(local_x, quart_y, local_z)
                                .ok_or_else(|| {
                                    invalid_schema(
                                        &path,
                                        "generated retrogen biome range is incomplete",
                                    )
                                })?;
                            let retained = self.biome_name(generated_biome).is_some_and(|name| {
                                matches!(
                                    name.as_str(),
                                    "minecraft:lush_caves"
                                        | "minecraft:dripstone_caves"
                                        | "minecraft:deep_dark"
                                )
                            });
                            let biome = if retained {
                                generated_biome
                            } else {
                                saved_column
                                    .get_noise_biome(local_x, 0, local_z)
                                    .ok_or_else(|| {
                                        invalid_schema(
                                            &path,
                                            "old quart-Y zero biome witness is incomplete",
                                        )
                                    })?
                            };
                            generated
                                .column
                                .set_noise_biome(local_x, quart_y, local_z, biome)
                                .map_err(|error| invalid_schema(&path, error.to_string()))?;
                        }
                    }
                }
            }
            replace_old_bedrock_and_apply_mask(&path, &mut generated.column, retrogen)?;
            let (ocean_floor_wg, world_surface_wg) = prime_worldgen_heightmaps(&generated.column);
            generated.ocean_floor_wg = ocean_floor_wg;
            generated.world_surface_wg = world_surface_wg;
            generated
                .postprocessing_positions
                .retain(|&(_, y, _)| y < RETROGEN_MAX_Y);
            generated
                .postprocessing_positions
                .extend(saved_postprocessing.iter().copied());
            generated
        } else {
            let (ocean_floor_wg, world_surface_wg) = prime_worldgen_heightmaps(saved_column);
            NoiseGeneratedChunk {
                chunk_x,
                chunk_z,
                column: saved_column.clone(),
                ocean_floor_wg,
                world_surface_wg,
                postprocessing_positions: saved_postprocessing,
                carving_mask: None,
            }
        };

        if saved_status < Java26ChunkStatus::Surface {
            let before_surface = chunk.column.clone();
            let old_postprocessing = chunk.postprocessing_positions.clone();
            self.apply_retrogen_surface(&mut chunk, biome_blender, &path)?;
            restore_old_height(&path, &before_surface, &mut chunk.column)?;
            chunk
                .postprocessing_positions
                .retain(|&(_, y, _)| y < RETROGEN_MAX_Y);
            chunk.postprocessing_positions.extend(old_postprocessing);
            let (ocean_floor_wg, world_surface_wg) = prime_worldgen_heightmaps(&chunk.column);
            chunk.ocean_floor_wg = ocean_floor_wg;
            chunk.world_surface_wg = world_surface_wg;
        }

        if self.carvers_are_verified_empty {
            chunk.carving_mask = Some(Vec::new());
        } else {
            let carvers = self.carvers.as_ref().ok_or_else(|| {
                invalid_schema(&path, "retrogen requires a frozen carver program")
            })?;
            let surface = self.surface.as_ref().ok_or_else(|| {
                invalid_schema(&path, "retrogen requires a frozen surface program")
            })?;
            let biome_source = self
                .biome_source
                .as_ref()
                .ok_or_else(|| invalid_schema(&path, "retrogen requires a frozen biome source"))?;
            chunk.carving_mask = Some(
                carvers
                    .apply_retrogen(
                        &self.source,
                        biome_source,
                        surface,
                        chunk_x,
                        chunk_z,
                        &mut chunk.column,
                        &mut chunk.ocean_floor_wg,
                        &mut chunk.world_surface_wg,
                        &mut chunk.postprocessing_positions,
                    )
                    .map_err(|error| invalid_schema(&path, error.to_string()))?,
            );
        }
        chunk.postprocessing_positions.sort_unstable();
        chunk.postprocessing_positions.dedup();
        Ok(chunk)
    }

    fn apply_retrogen_surface(
        &self,
        chunk: &mut NoiseGeneratedChunk,
        biome_blender: Option<&BlendingResolver>,
        path: &str,
    ) -> SaveResult<()> {
        let surface = self
            .surface
            .as_ref()
            .ok_or_else(|| invalid_schema(path, "retrogen requires a frozen surface program"))?;
        let biome_source = self
            .biome_source
            .as_ref()
            .ok_or_else(|| invalid_schema(path, "retrogen requires a frozen biome source"))?;
        let min_quart_y = chunk.column.min_y().div_euclid(4);
        let quart_height = i32::try_from(chunk.column.section_count())
            .map_err(|_| invalid_schema(path, "column section count is outside i32"))?
            .checked_mul(4)
            .ok_or_else(|| invalid_schema(path, "column quart height overflows"))?;
        let max_quart_y = min_quart_y
            .checked_add(quart_height)
            .ok_or_else(|| invalid_schema(path, "column quart range overflows"))?;
        let chunk_quart_x = chunk
            .chunk_x
            .checked_mul(4)
            .ok_or_else(|| invalid_schema(path, "surface chunk quart x overflows"))?;
        let chunk_quart_z = chunk
            .chunk_z
            .checked_mul(4)
            .ok_or_else(|| invalid_schema(path, "surface chunk quart z overflows"))?;
        let mut assigned = Vec::with_capacity(
            usize::try_from(quart_height)
                .map_err(|_| invalid_schema(path, "column quart height is negative"))?
                * 16,
        );
        for quart_y in min_quart_y..max_quart_y {
            for local_x in 0..4 {
                for local_z in 0..4 {
                    assigned.push(
                        chunk
                            .column
                            .get_noise_biome(local_x, quart_y, local_z)
                            .ok_or_else(|| {
                                invalid_schema(path, "assigned retrogen BIOMES are incomplete")
                            })?,
                    );
                }
            }
        }
        surface.apply_retrogen(
            &self.source,
            biome_source,
            chunk.chunk_x,
            chunk.chunk_z,
            &mut chunk.column,
            &mut chunk.ocean_floor_wg,
            &mut chunk.world_surface_wg,
            &mut chunk.postprocessing_positions,
            |quart_x, quart_y, quart_z| {
                let local_x = quart_x - chunk_quart_x;
                let local_z = quart_z - chunk_quart_z;
                if (0..4).contains(&local_x) && (0..4).contains(&local_z) {
                    let clamped_y = quart_y.clamp(min_quart_y, max_quart_y - 1);
                    let index = usize::try_from(clamped_y - min_quart_y).unwrap() * 16
                        + usize::try_from(local_x).unwrap() * 4
                        + usize::try_from(local_z).unwrap();
                    Some(assigned[index])
                } else {
                    biome_blender.and_then(|blender| blender.blend(quart_x, quart_y, quart_z))
                }
            },
        );
        Ok(())
    }
}

fn replace_old_bedrock_and_apply_mask(
    path: &str,
    column: &mut ChunkColumn,
    retrogen: &Java26BelowZeroRetrogen,
) -> SaveResult<()> {
    let state = |block_id: i32, name: &str| {
        vanilla_block_default_state_id(block_id)
            .ok_or_else(|| invalid_schema(path, format!("Vanilla block {name} has no default")))
    };
    let bedrock = state(VANILLA_BLOCK_BEDROCK_ID, "minecraft:bedrock")?;
    let deepslate = state(VANILLA_BLOCK_DEEPSLATE_ID, "minecraft:deepslate")?;
    let air = state(VANILLA_BLOCK_AIR_ID, "minecraft:air")?;
    // `BelowZeroRetrogen.java:63-95`.
    for local_x in 0..16 {
        for local_z in 0..16 {
            for y in 0..=4 {
                if column.get_block_state(local_x, y, local_z) == Some(bedrock) {
                    let _ = column.set_block_state(local_x, y, local_z, deepslate);
                }
            }
            if retrogen.has_bedrock_hole(local_x, local_z) {
                for y in RETROGEN_MIN_Y..RETROGEN_MAX_Y {
                    column
                        .set_block_state(local_x, y, local_z, air)
                        .ok_or_else(|| {
                            invalid_schema(path, "missing-bedrock mask exceeds the restored column")
                        })?;
                }
            }
        }
    }
    Ok(())
}

fn restore_old_height(path: &str, before: &ChunkColumn, after: &mut ChunkColumn) -> SaveResult<()> {
    let max_y = before
        .min_y()
        .checked_add(
            i32::try_from(before.section_count())
                .map_err(|_| invalid_schema(path, "section count exceeds Java int"))?
                .checked_mul(16)
                .ok_or_else(|| invalid_schema(path, "section height overflows"))?,
        )
        .ok_or_else(|| invalid_schema(path, "column height overflows"))?;
    for y in RETROGEN_MAX_Y..max_y {
        for local_z in 0..16 {
            for local_x in 0..16 {
                let state = before.get_block_state(local_x, y, local_z).ok_or_else(|| {
                    invalid_schema(path, "old-height surface snapshot is incomplete")
                })?;
                after
                    .set_block_state(local_x, y, local_z, state)
                    .ok_or_else(|| invalid_schema(path, "surface result column is incomplete"))?;
            }
        }
    }
    Ok(())
}
