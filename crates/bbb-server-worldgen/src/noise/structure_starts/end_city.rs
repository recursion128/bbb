use super::*;
use crate::noise::end_city::EndCityStructureStart;
use bbb_server_datapack::startup::DatapackStartup;

impl NoiseStructureChunkData {
    pub(super) fn decode_end_city_start(
        id: &str,
        value: &NbtSummaryValue,
        terrain_adjustment: crate::noise::structures::StructureTerrainAdjustment,
        startup: &DatapackStartup,
    ) -> Option<EndCityStructureStart> {
        let mut decoded =
            EndCityStructureStart::from_nbt(id, value)?.with_terrain_adjustment(terrain_adjustment);
        for piece in decoded.pieces_mut() {
            let element = startup.structures.template(piece.template())?;
            piece.set_template_size(element.template.size);
        }
        Some(decoded)
    }

    pub fn end_city_starts(&self) -> &[EndCityStructureStart] {
        &self.end_city_starts
    }

    pub fn with_end_city_starts(
        mut self,
        starts: impl IntoIterator<Item = EndCityStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.end_city_starts,
            starts,
            EndCityStructureStart::structure_id,
        );
        self
    }
}
