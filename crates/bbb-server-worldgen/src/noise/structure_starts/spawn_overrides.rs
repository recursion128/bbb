use bbb_protocol::entity_types::VanillaEntityMobCategory;
use bbb_protocol::packets::BlockPos;
use bbb_server_datapack::mob_spawn::{StructureMobSpawnOverride, StructureSpawnBoundingBox};

use crate::noise::end_city::EndCityPiece;
use crate::noise::generated_structure_piece::bounding_box_from_array;
use crate::noise::igloo::persistence::piece_bounding_box as igloo_piece_box;
use crate::noise::ocean_ruin::persistence::piece_bounding_box as ocean_ruin_piece_box;
use crate::noise::ruined_portal::template_bounding_box as ruined_portal_piece_box;
use crate::noise::shipwreck::persistence::piece_bounding_box as shipwreck_piece_box;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::noise::structures::{NoiseStructurePlan, StructureTerrainAdjustment};
use crate::noise::woodland_mansion::persistence::piece_bounding_box as mansion_piece_box;

use super::nbt::unpack_chunk;
use super::NoiseStructureChunkData;

const FORTRESS_ID: &str = "minecraft:fortress";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StructureSpawnOverrideMatch<'a> {
    pub structure_id: &'a str,
    pub override_: &'a StructureMobSpawnOverride,
}

impl NoiseStructureChunkData {
    // `ChunkGenerator.java:437-462`.
    pub fn spawn_override_at<'a, 's>(
        &'s self,
        plan: &'a NoiseStructurePlan,
        category: VanillaEntityMobCategory,
        position: BlockPos,
        mut source_at: impl FnMut(i32, i32) -> Option<&'s NoiseStructureChunkData>,
    ) -> Option<StructureSpawnOverrideMatch<'a>> {
        for structure_id in plan.structure_order() {
            let Some(definition) = plan.structures().get(structure_id) else {
                continue;
            };
            let Some(spawn_override) = definition
                .spawn_overrides
                .iter()
                .find(|entry| entry.category == category)
            else {
                continue;
            };
            let contains = self.any_referenced_start(structure_id, &mut source_at, |pieces| {
                match spawn_override.bounding_box {
                    StructureSpawnBoundingBox::Piece => {
                        pieces.iter().any(|bounds| bounds.contains(position))
                    }
                    StructureSpawnBoundingBox::Full => {
                        adjusted_full_box(pieces, definition.terrain_adjustment)
                            .is_some_and(|bounds| bounds.contains(position))
                    }
                }
            });
            if contains {
                return Some(StructureSpawnOverrideMatch {
                    structure_id: &definition.id,
                    override_: spawn_override,
                });
            }
        }
        None
    }

    // `NaturalSpawner.java:318-339` keeps this membership query separate from
    // the category and nether-bricks checks owned by the natural-spawn caller.
    pub fn fortress_full_box_contains<'s>(
        &'s self,
        plan: &NoiseStructurePlan,
        position: BlockPos,
        mut source_at: impl FnMut(i32, i32) -> Option<&'s NoiseStructureChunkData>,
    ) -> bool {
        let Some(definition) = plan.structures().get(FORTRESS_ID) else {
            return false;
        };
        self.any_referenced_start(FORTRESS_ID, &mut source_at, |pieces| {
            adjusted_full_box(pieces, definition.terrain_adjustment)
                .is_some_and(|bounds| bounds.contains(position))
        })
    }

    // StructureManager.java:109-148: membership tests the actual pieces.
    pub fn referenced_piece_contains<'s>(
        &'s self,
        position: BlockPos,
        mut structure_matches: impl FnMut(&str) -> bool,
        mut source_at: impl FnMut(i32, i32) -> Option<&'s NoiseStructureChunkData>,
    ) -> bool {
        self.references.keys().any(|id| {
            structure_matches(id)
                && self.any_referenced_start(id, &mut source_at, |pieces| {
                    pieces.iter().any(|bounds| bounds.contains(position))
                })
        })
    }

    fn any_referenced_start<'s>(
        &'s self,
        structure_id: &str,
        source_at: &mut impl FnMut(i32, i32) -> Option<&'s NoiseStructureChunkData>,
        mut matches: impl FnMut(&[JigsawBoundingBox]) -> bool,
    ) -> bool {
        self.references
            .get(structure_id)
            .into_iter()
            .flatten()
            .any(|&packed| {
                let source = unpack_chunk(packed);
                let data = if source == self.chunk {
                    Some(self)
                } else {
                    source_at(source[0], source[1])
                };
                data.and_then(|data| data.raw_start_geometry(structure_id))
                    .is_some_and(|pieces| matches(&pieces))
            })
    }

    pub(super) fn raw_start_geometry(&self, structure_id: &str) -> Option<Vec<JigsawBoundingBox>> {
        if let Some(start) = self
            .starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            return Some(
                start
                    .pieces()
                    .iter()
                    .map(|piece| piece.bounding_box)
                    .collect(),
            );
        }
        if let Some(start) = self
            .buried_treasure_start
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            return Some(vec![point_box(start.piece().position())]);
        }
        if let Some(start) = self
            .mineshaft_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            return Some(
                start
                    .pieces()
                    .iter()
                    .map(|piece| piece.bounding_box)
                    .collect(),
            );
        }
        if let Some(start) = self
            .shipwreck_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            let piece = start.piece();
            return Some(vec![template_or_point_box(
                piece.template_position(),
                piece.template_size(),
                || {
                    shipwreck_piece_box(
                        piece.template_position(),
                        piece.rotation(),
                        piece.template_size(),
                    )
                },
            )]);
        }
        if let Some(start) = self
            .ocean_ruin_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            let boxes = start
                .pieces()
                .iter()
                .map(|piece| {
                    template_or_point_box(piece.template_position(), piece.template_size(), || {
                        ocean_ruin_piece_box(
                            piece.template_position(),
                            piece.rotation(),
                            piece.template_size(),
                        )
                    })
                })
                .collect();
            return Some(boxes);
        }
        if let Some(start) = self
            .igloo_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            let boxes = start
                .pieces()
                .iter()
                .map(|piece| {
                    template_or_point_box(piece.template_position(), piece.template_size(), || {
                        igloo_piece_box(
                            piece.template(),
                            piece.template_position(),
                            piece.rotation(),
                            piece.template_size(),
                        )
                    })
                })
                .collect();
            return Some(boxes);
        }
        if let Some(start) = self
            .scattered_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            return Some(vec![bounding_box_from_array(start.piece().bounding_box())]);
        }
        if let Some(start) = self
            .ruined_portal_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            let piece = start.piece();
            return Some(vec![template_or_point_box(
                piece.template_position(),
                piece.template_size(),
                || {
                    let size = piece.template_size();
                    ruined_portal_piece_box(
                        piece.template_position(),
                        piece.rotation(),
                        BlockPos {
                            x: size[0] / 2,
                            y: 0,
                            z: size[2] / 2,
                        },
                        piece.mirror(),
                        size,
                    )
                },
            )]);
        }
        if let Some(start) = self
            .ocean_monument_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            return Some(vec![bounding_box_from_array(start.piece().bounding_box())]);
        }
        if let Some(start) = self
            .woodland_mansion_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            let boxes = start
                .pieces()
                .iter()
                .map(|piece| {
                    template_or_point_box(piece.template_position(), piece.template_size(), || {
                        mansion_piece_box(
                            piece.template_position(),
                            piece.rotation(),
                            piece.mirror(),
                            piece.template_size(),
                        )
                    })
                })
                .collect();
            return Some(boxes);
        }
        if let Some(start) = self
            .stronghold_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            return Some(
                start
                    .pieces()
                    .iter()
                    .map(|piece| bounding_box_from_array(piece.bounding_box()))
                    .collect(),
            );
        }
        if let Some(start) = self
            .fortress_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            return Some(
                start
                    .pieces()
                    .iter()
                    .map(|piece| piece.bounding_box)
                    .collect(),
            );
        }
        if let Some(start) = self
            .nether_fossil_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            return Some(vec![start.piece().bounding_box()]);
        }
        if let Some(start) = self
            .end_city_starts
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            let boxes = start
                .pieces()
                .iter()
                .map(EndCityPiece::bounding_box)
                .collect::<Vec<_>>();
            return Some(boxes);
        }
        if let Some(restored) = self.restored_bounds.get(structure_id) {
            return Some(restored.pieces.clone());
        }
        None
    }
}

// `Structure.java:75-81`.
fn adjusted_full_box(
    pieces: &[JigsawBoundingBox],
    terrain_adjustment: StructureTerrainAdjustment,
) -> Option<JigsawBoundingBox> {
    let mut bounds = *pieces.first()?;
    for piece in pieces {
        if piece.min.x > piece.max.x || piece.min.y > piece.max.y || piece.min.z > piece.max.z {
            return None;
        }
        bounds.min.x = bounds.min.x.min(piece.min.x);
        bounds.min.y = bounds.min.y.min(piece.min.y);
        bounds.min.z = bounds.min.z.min(piece.min.z);
        bounds.max.x = bounds.max.x.max(piece.max.x);
        bounds.max.y = bounds.max.y.max(piece.max.y);
        bounds.max.z = bounds.max.z.max(piece.max.z);
    }
    if terrain_adjustment != StructureTerrainAdjustment::None {
        bounds.min.x = bounds.min.x.checked_sub(12)?;
        bounds.min.y = bounds.min.y.checked_sub(12)?;
        bounds.min.z = bounds.min.z.checked_sub(12)?;
        bounds.max.x = bounds.max.x.checked_add(12)?;
        bounds.max.y = bounds.max.y.checked_add(12)?;
        bounds.max.z = bounds.max.z.checked_add(12)?;
    }
    Some(bounds)
}

fn template_or_point_box(
    position: BlockPos,
    size: [i32; 3],
    template_box: impl FnOnce() -> [i32; 6],
) -> JigsawBoundingBox {
    if size == [0, 0, 0] {
        point_box(position)
    } else {
        bounding_box_from_array(template_box())
    }
}

const fn point_box(position: BlockPos) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: position,
        max: position,
    }
}

#[cfg(test)]
mod tests;
