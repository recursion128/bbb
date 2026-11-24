//! Projection of a FULL column into the owned clientbound packet payload.

use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
};
use bbb_protocol::packets::{ChunkHeightmapData, LightUpdateData};
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::{ChunkColumn, ChunkGenerationState};
use bbb_server_types::save::{SaveError, SaveResult};

use super::full::invalid_full;
use super::{ChunkSource, ChunkStore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullChunkPacketData {
    pub heightmaps: Vec<ChunkHeightmapData>,
    pub light_data: LightUpdateData,
}

impl ChunkStore {
    pub fn full_chunk_packet_data(&self, cx: i32, cz: i32) -> SaveResult<FullChunkPacketData> {
        if !self.is_ordinary_source() {
            return self
                .noise_full_chunk_packet_data(cx, cz)?
                .ok_or_else(|| invalid_packet((cx, cz), "noise projection is absent"));
        }
        let center = (cx, cz);
        self.validate_ordinary_light(center)?;
        let chunk = self.loaded(center);
        self.packet_data(center, &chunk.column, chunk.heightmaps())
    }

    /// `ClientboundLevelChunkPacketData` sends only client heightmaps
    /// (`ClientboundLevelChunkPacketData.java:27-40`) and couples them with the
    /// complete light-section projection (`ClientboundLightUpdatePacketData.java:26-47`).
    pub fn noise_full_chunk_packet_data(
        &self,
        cx: i32,
        cz: i32,
    ) -> SaveResult<Option<FullChunkPacketData>> {
        if matches!(self.source, ChunkSource::Flat(_) | ChunkSource::Debug(_)) {
            return Ok(None);
        }
        let center = (cx, cz);
        self.validate_servable_noise_full_center(center)?;
        let Some(ChunkGenerationState::NoiseFull(full)) = self.generation(center) else {
            return Err(invalid_full(
                center,
                "clientbound projection requires typed noise FULL",
            ));
        };
        let column = self
            .column_at(center)
            .ok_or_else(|| invalid_full(center, "clientbound projection has no loaded column"))?;
        let maps = full.light().initialized().features().final_heightmaps();
        self.packet_data(center, column, maps).map(Some)
    }

    fn packet_data(
        &self,
        center: (i32, i32),
        column: &ChunkColumn,
        maps: &FeatureHeightmaps,
    ) -> SaveResult<FullChunkPacketData> {
        let world_height = column
            .section_count()
            .checked_mul(16)
            .ok_or_else(|| invalid_packet(center, "column height overflows"))?;
        let heightmaps = [
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
        ]
        .into_iter()
        .map(|kind_id| {
            let path = format!(
                "clientbound chunk [{},{}].heightmap[{kind_id}]",
                center.0, center.1
            );
            Ok(ChunkHeightmapData {
                kind_id,
                data: maps.packed_values(kind_id, column.min_y(), world_height, &path)?,
            })
        })
        .collect::<SaveResult<Vec<_>>>()?;
        Ok(FullChunkPacketData {
            heightmaps,
            light_data: self.column_packet_light_data(center)?,
        })
    }

    pub(super) fn column_packet_light_data(
        &self,
        center: (i32, i32),
    ) -> SaveResult<LightUpdateData> {
        let column = self
            .column_at(center)
            .ok_or_else(|| invalid_packet(center, "light projection has no loaded column"))?;
        let min_light_section = column
            .min_y()
            .div_euclid(16)
            .checked_sub(1)
            .ok_or_else(|| invalid_packet(center, "minimum light section overflows"))?;
        let light_section_count = column
            .section_count()
            .checked_add(2)
            .ok_or_else(|| invalid_packet(center, "light section count overflows"))?;
        self.packet_light_data(center, min_light_section, light_section_count)
    }

    pub(super) fn packet_light_data(
        &self,
        center: (i32, i32),
        min_light_section: i32,
        light_section_count: usize,
    ) -> SaveResult<LightUpdateData> {
        let mut light_data =
            self.light_storage
                .packet_data(center, min_light_section, light_section_count)?;
        if !self.has_skylight {
            // The dummy sky listener returns no layers, so Vanilla sets no sky packet bits
            // (`LevelLightEngine.java:94-99`; `LayerLightEventListener.java:14-25`).
            light_data.sky_y_mask.clear();
            light_data.empty_sky_y_mask.clear();
            light_data.sky_updates.clear();
        }
        Ok(light_data)
    }
}

fn invalid_packet(center: (i32, i32), detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!(
            "clientbound chunk [{},{}]: {}",
            center.0,
            center.1,
            detail.into()
        ),
    }
}
