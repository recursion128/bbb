use std::sync::Arc;

use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::structure_pool::StructureLiquidSettings;
use bbb_server_datapack::structure_template::{
    MaterializedStructureTemplate, StructureTemplatePalette, StructureTemplateSet,
};

use super::{EndCityPiece, EndCityStructureStart, END_CITY_LOOT};
use crate::noise::ruined_portal::RuinedPortalMirror;
use crate::noise::structure_pieces::{JigsawBoundingBox, JigsawRotation};
use crate::noise::woodland_mansion::direction::{
    rotate_block_pos, rotate_direction, MansionDirection,
};
use crate::structure_processor::StructureProcessor;
use crate::structure_template_placement::{
    place_materialized_template_with_palette, MaterializedTemplatePlacement, TemplateSettingsRandom,
};
use crate::{block_is, catalog_default_state, WorldGenLevel};

const UPDATE_CLIENTS: i32 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EndCityChunkPlacement {
    pub attempted_pieces: usize,
    pub successful_pieces: usize,
}

impl EndCityStructureStart {
    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        templates: &StructureTemplateSet,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> EndCityChunkPlacement {
        let mut report = EndCityChunkPlacement::default();
        for piece in self.pieces_mut() {
            let template_id = piece.template().to_string();
            let live_template = level.structure_template(&template_id);
            let Some(template) = live_template.as_ref().or_else(|| {
                templates
                    .template(&template_id)
                    .map(|element| &element.template)
            }) else {
                continue;
            };
            piece.set_template_size(template.size);
            if !piece.bounding_box().intersects(chunk_bounds) {
                continue;
            }
            report.attempted_pieces += 1;
            if place_template(piece, template.materialized(), level, chunk_bounds, random) {
                report.successful_pieces += 1;
            }
        }
        report
    }
}

fn place_template<L: WorldGenLevel>(
    piece: &EndCityPiece,
    template: &MaterializedStructureTemplate,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> bool {
    let origin = piece.template_position();
    let mut ignored = vec![catalog_default_state("minecraft:structure_block")];
    if !piece.overwrite() {
        ignored.push(catalog_default_state("minecraft:air"));
    }
    let processors = [StructureProcessor::BlockIgnore { blocks: ignored }];
    // End City pieces ignore template entities and select either the structure-only
    // or structure-and-air processor (`EndCityPieces.java:383-389`).
    let Some(palette) = place_materialized_template_with_palette(
        template,
        level,
        MaterializedTemplatePlacement {
            position: origin,
            reference_pos: origin,
            rotation: piece.rotation(),
            mirror: RuinedPortalMirror::None,
            pivot: BlockPos { x: 0, y: 0, z: 0 },
            finalize_entities: false,
            ignore_entities: true,
            bounding_box: chunk_bounds,
            processors: &processors,
            liquid_settings: StructureLiquidSettings::ApplyWaterlogging,
            update_mode: UPDATE_CLIENTS,
            known_shape: true,
        },
        random,
        TemplateSettingsRandom::Positional,
    ) else {
        return false;
    };
    handle_data_markers(piece, palette, chunk_bounds, level, random);
    true
}

fn handle_data_markers<L: WorldGenLevel>(
    piece: &EndCityPiece,
    palette: &StructureTemplatePalette,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
    random: &mut dyn RandomSource,
) {
    for (position, nbt) in filter_structure_blocks(piece, palette, chunk_bounds) {
        let Some(nbt) = nbt.as_deref() else {
            continue;
        };
        if !compound_string(nbt, "mode").is_some_and(|mode| mode.eq_ignore_ascii_case("data")) {
            continue;
        }
        let marker = compound_string(nbt, "metadata").unwrap_or_default();
        if marker.starts_with("Chest") {
            let chest = BlockPos {
                y: position.y.wrapping_sub(1),
                ..position
            };
            if chunk_bounds.contains(chest) {
                level.set_block_entity_loot_table(chest, END_CITY_LOOT, random.next_long());
            }
            continue;
        }
        if !chunk_bounds.contains(position)
            || position.y < level.get_min_y()
            || position.y > level.get_max_y()
        {
            continue;
        }
        if marker.starts_with("Sentry") {
            let snapped = [
                f64::from(position.x) + 0.5,
                f64::from(position.y),
                f64::from(position.z) + 0.5,
            ];
            level.add_structure_mob("minecraft:shulker", snapped);
        } else if marker.starts_with("Elytra") {
            let facing = rotated_south(piece.rotation());
            level.add_structure_item_frame(position, facing, "minecraft:elytra");
        }
    }
}

fn filter_structure_blocks(
    piece: &EndCityPiece,
    palette: &StructureTemplatePalette,
    chunk_bounds: JigsawBoundingBox,
) -> Vec<(BlockPos, Option<Arc<NbtSummaryValue>>)> {
    palette
        .blocks
        .iter()
        .filter(|block| block_is(block.state, "minecraft:structure_block"))
        .filter_map(|block| {
            let position = add(
                rotate_block_pos(block.position, piece.rotation()),
                piece.template_position(),
            );
            chunk_bounds
                .contains(position)
                .then_some((position, block.nbt.clone()))
        })
        .collect()
}

fn rotated_south(rotation: JigsawRotation) -> Direction {
    match rotate_direction(MansionDirection::South, rotation) {
        MansionDirection::North => Direction::North,
        MansionDirection::South => Direction::South,
        MansionDirection::West => Direction::West,
        MansionDirection::East => Direction::East,
        MansionDirection::Up => unreachable!("horizontal rotation cannot produce up"),
    }
}

fn compound_string(value: &NbtSummaryValue, name: &str) -> Option<String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries.iter().find_map(|entry| {
        (entry.name == name)
            .then_some(&entry.value)
            .and_then(|value| match value {
                NbtSummaryValue::String(value) => value.try_to_string().ok(),
                _ => None,
            })
    })
}

fn add(left: BlockPos, right: BlockPos) -> BlockPos {
    BlockPos {
        x: left.x.wrapping_add(right.x),
        y: left.y.wrapping_add(right.y),
        z: left.z.wrapping_add(right.z),
    }
}
