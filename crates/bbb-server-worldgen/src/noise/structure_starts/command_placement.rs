//! Placement of a transient forced structure start.

use bbb_registry_codec::random_source::RandomSource;

use super::*;
use crate::noise::generated_structure_piece::bounding_box_from_array;
use crate::noise::mineshaft::MineshaftPlacementLevel;
use crate::noise::stronghold::geometry::union_boxes;

impl NoiseStructureChunkData {
    /// `StructureStart.getBoundingBox` and `Structure.adjustBoundingBox`
    /// (`StructureStart.java:69-77`, `Structure.java:79-81`).
    pub fn forced_structure_bounding_box(
        &self,
        adjustment: StructureTerrainAdjustment,
    ) -> Option<JigsawBoundingBox> {
        let mut boxes = Vec::new();
        for start in &self.starts {
            boxes.extend(
                start
                    .pieces()
                    .iter()
                    .map(|piece| box_array(piece.bounding_box)),
            );
        }
        for start in self.buried_treasure_start.iter() {
            boxes.push(box_array(treasure_box(start)));
        }
        for start in &self.mineshaft_starts {
            boxes.extend(
                start
                    .pieces()
                    .iter()
                    .map(|piece| box_array(piece.bounding_box)),
            );
        }
        for start in &self.shipwreck_starts {
            boxes.push(box_array(shipwreck_box(start)));
        }
        for start in &self.ocean_ruin_starts {
            boxes.extend(start.pieces().iter().map(|piece| {
                crate::noise::ocean_ruin::persistence::piece_bounding_box(
                    piece.template_position(),
                    piece.rotation(),
                    piece.template_size(),
                )
            }));
        }
        for start in &self.igloo_starts {
            boxes.extend(start.pieces().iter().map(|piece| {
                crate::noise::igloo::persistence::piece_bounding_box(
                    piece.template(),
                    piece.template_position(),
                    piece.rotation(),
                    piece.template_size(),
                )
            }));
        }
        for start in &self.scattered_starts {
            boxes.push(start.piece().bounding_box());
        }
        for start in &self.ruined_portal_starts {
            let piece = start.piece();
            let size = piece.template_size();
            boxes.push(crate::noise::ruined_portal::template_bounding_box(
                piece.template_position(),
                piece.rotation(),
                BlockPos {
                    x: size[0] / 2,
                    y: 0,
                    z: size[2] / 2,
                },
                piece.mirror(),
                size,
            ));
        }
        for start in &self.ocean_monument_starts {
            boxes.push(start.piece().bounding_box());
        }
        for start in &self.woodland_mansion_starts {
            boxes.extend(start.pieces().iter().map(|piece| {
                crate::noise::woodland_mansion::persistence::piece_bounding_box(
                    piece.template_position(),
                    piece.rotation(),
                    piece.mirror(),
                    piece.template_size(),
                )
            }));
        }
        for start in &self.stronghold_starts {
            boxes.extend(start.pieces().iter().map(|piece| piece.bounding_box()));
        }
        for start in &self.fortress_starts {
            boxes.extend(
                start
                    .pieces()
                    .iter()
                    .map(|piece| box_array(piece.bounding_box)),
            );
        }
        for start in &self.nether_fossil_starts {
            boxes.push(box_array(start.piece().bounding_box()));
        }
        for start in &self.end_city_starts {
            for piece in start.pieces() {
                boxes.push(box_array(piece.bounding_box()));
            }
        }
        let bounds = bounding_box_from_array(union_boxes(&boxes)?);
        Some(if adjustment == StructureTerrainAdjustment::None {
            bounds
        } else {
            inflate(bounds, 12)
        })
    }

    /// `StructureStart.placeInChunk` (`StructureStart.java:79-99`).
    pub fn place_forced_structure_in_chunk<L: MineshaftPlacementLevel>(
        &mut self,
        startup: &DatapackStartup,
        level: &mut L,
        bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) {
        for start in &self.starts {
            start.place_in_chunk(startup, level, bounds, random);
        }
        for start in self.buried_treasure_start.iter_mut() {
            if treasure_box(start).intersects(bounds) {
                start.place_in_chunk_with_bounds(level, bounds, random);
            }
        }
        for start in &mut self.mineshaft_starts {
            start.place_in_chunk_with_bounds(level, random, bounds);
        }
        for start in &mut self.shipwreck_starts {
            if shipwreck_box(start).intersects(bounds) {
                start.place_in_chunk(&startup.structures, level, bounds, random);
            }
        }
        for start in &mut self.ocean_ruin_starts {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        }
        for start in &mut self.igloo_starts {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        }
        for start in &mut self.scattered_starts {
            start.place_in_chunk(level, bounds, random);
        }
        for start in &mut self.ruined_portal_starts {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        }
        for start in &mut self.ocean_monument_starts {
            start.place_in_chunk(level, bounds, random);
        }
        for start in &mut self.woodland_mansion_starts {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        }
        for start in &mut self.stronghold_starts {
            start.place_in_chunk(level, bounds, random);
        }
        for start in &mut self.fortress_starts {
            start.place_in_chunk(level, bounds, random);
        }
        for start in &mut self.nether_fossil_starts {
            if start.piece().bounding_box().intersects(bounds) {
                start.place_in_chunk(&startup.structures, level, bounds, random);
            }
        }
        for start in &mut self.end_city_starts {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        }
    }
}

fn treasure_box(start: &BuriedTreasureStructureStart) -> JigsawBoundingBox {
    let position = start.piece().position();
    JigsawBoundingBox {
        min: position,
        max: position,
    }
}

fn shipwreck_box(start: &ShipwreckStructureStart) -> JigsawBoundingBox {
    let piece = start.piece();
    bounding_box_from_array(crate::noise::shipwreck::persistence::piece_bounding_box(
        piece.template_position(),
        piece.rotation(),
        piece.template_size(),
    ))
}

fn box_array(bounds: JigsawBoundingBox) -> [i32; 6] {
    [
        bounds.min.x,
        bounds.min.y,
        bounds.min.z,
        bounds.max.x,
        bounds.max.y,
        bounds.max.z,
    ]
}
