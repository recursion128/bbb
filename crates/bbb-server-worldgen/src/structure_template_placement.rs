//! Shared `StructureTemplate.placeInWorld` block/entity publication.

use crate::random::WorldgenRandom;
use bbb_protocol::block_state_types::{
    vanilla_block_state_mirror, vanilla_block_state_name, vanilla_block_state_rotate_y,
    VanillaBlockStateMirror,
};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::structure_pool::StructureLiquidSettings;
use bbb_server_datapack::structure_template::{
    MaterializedStructureTemplate, StructureTemplateBlock, StructureTemplatePalette,
};

use crate::noise::ruined_portal::{transform, RuinedPortalMirror};
use crate::noise::structure_pieces::{JigsawBoundingBox, JigsawRotation};
use crate::noise::template_entities::place_template_entities_with_settings;
use crate::structure_processor::{
    block_pos_seed, process_block_infos_with_positional_random, StructureBlockInfo,
    StructureProcessor,
};
use crate::template_waterlogging::StructureTemplateWaterlogging;
use crate::tree::voxel_shape::BitSetDiscreteVoxelShape;
use crate::{catalog_default_state, WorldGenLevel};

pub enum TemplateSettingsRandom<'a> {
    Shared,
    Positional,
    Separate(&'a mut dyn RandomSource),
}

pub struct MaterializedTemplatePlacement<'a> {
    pub position: BlockPos,
    pub reference_pos: BlockPos,
    pub rotation: JigsawRotation,
    pub mirror: RuinedPortalMirror,
    pub pivot: BlockPos,
    pub finalize_entities: bool,
    pub ignore_entities: bool,
    pub bounding_box: JigsawBoundingBox,
    pub processors: &'a [StructureProcessor],
    pub liquid_settings: StructureLiquidSettings,
    pub update_mode: i32,
    pub known_shape: bool,
}

/// `StructureTemplate.placeInWorld` (`StructureTemplate.java:251-410`).
pub fn place_materialized_template<L: WorldGenLevel>(
    template: &MaterializedStructureTemplate,
    level: &mut L,
    settings: MaterializedTemplatePlacement<'_>,
    random: &mut dyn RandomSource,
    settings_random: TemplateSettingsRandom<'_>,
) -> bool {
    place_materialized_template_with_palette(template, level, settings, random, settings_random)
        .is_some()
}

/// `StructureTemplate.placeInWorld` (`StructureTemplate.java:251-410`), returning
/// the selected palette when placement ran so `TemplateStructurePiece` callers
/// can run data-marker and jigsaw-finalization passes against the same palette.
pub fn place_materialized_template_with_palette<'a, L: WorldGenLevel>(
    template: &'a MaterializedStructureTemplate,
    level: &mut L,
    settings: MaterializedTemplatePlacement<'_>,
    random: &mut dyn RandomSource,
    mut settings_random: TemplateSettingsRandom<'_>,
) -> Option<&'a StructureTemplatePalette> {
    if template.palettes.is_empty() {
        return None;
    }
    let palette_index = match &mut settings_random {
        TemplateSettingsRandom::Shared => random.next_int(template.palettes.len() as i32) as usize,
        TemplateSettingsRandom::Separate(settings_random) => {
            settings_random.next_int(template.palettes.len() as i32) as usize
        }
        TemplateSettingsRandom::Positional => {
            let mut palette_random = WorldgenRandom::with_seed(block_pos_seed(settings.position));
            palette_random.next_int(template.palettes.len() as i32) as usize
        }
    };
    let palette = &template.palettes[palette_index];
    if (palette.blocks.is_empty() && (settings.ignore_entities || template.entities.is_empty()))
        || template.size.iter().any(|axis| *axis < 1)
    {
        return None;
    }
    let blocks = palette
        .blocks
        .iter()
        .map(structure_block_info)
        .collect::<Vec<_>>();
    let calculate_relative_position = |block_pos| {
        wrapping_add(
            transform(
                block_pos,
                settings.mirror,
                settings.rotation,
                settings.pivot,
            ),
            settings.position,
        )
    };
    let processed = (match &mut settings_random {
        TemplateSettingsRandom::Shared => crate::structure_processor::process_block_infos(
            level,
            settings.position,
            settings.reference_pos,
            settings.processors,
            &blocks,
            random,
            calculate_relative_position,
        ),
        TemplateSettingsRandom::Separate(settings_random) => {
            crate::structure_processor::process_block_infos(
                level,
                settings.position,
                settings.reference_pos,
                settings.processors,
                &blocks,
                *settings_random,
                calculate_relative_position,
            )
        }
        TemplateSettingsRandom::Positional => process_block_infos_with_positional_random(
            level,
            settings.position,
            settings.reference_pos,
            settings.processors,
            &blocks,
            calculate_relative_position,
        ),
    })?;

    let mut writes = Vec::new();
    let mut waterlogging = StructureTemplateWaterlogging::default();
    for info in processed {
        if !settings.bounding_box.contains(info.pos) {
            continue;
        }
        // StructureTemplate.java:285: mirror the state before rotating it.
        let mirror = match settings.mirror {
            RuinedPortalMirror::None => VanillaBlockStateMirror::None,
            RuinedPortalMirror::LeftRight => VanillaBlockStateMirror::LeftRight,
            RuinedPortalMirror::FrontBack => VanillaBlockStateMirror::FrontBack,
        };
        let mirrored = vanilla_block_state_mirror(info.state, mirror).unwrap_or(info.state);
        let state = vanilla_block_state_rotate_y(mirrored, settings.rotation.quarter_turns())
            .unwrap_or(mirrored);
        let prepared = waterlogging.prepare(
            info.pos,
            state,
            settings.liquid_settings == StructureLiquidSettings::ApplyWaterlogging,
        );
        writes.push((info.pos, state, info.nbt, prepared));
    }

    let mut placed = Vec::with_capacity(writes.len());
    let mut placed_block_entities = Vec::new();
    let (mut min_x, mut min_y, mut min_z) = (i32::MAX, i32::MAX, i32::MAX);
    let (mut max_x, mut max_y, mut max_z) = (i32::MIN, i32::MIN, i32::MIN);
    let barrier = catalog_default_state("minecraft:barrier");
    for (position, state, nbt, prepared) in writes {
        level.prepare_structure_block(position);
        let previous = waterlogging.capture_previous(level, &prepared);
        if nbt.is_some() {
            level.set_block(position, barrier, 820);
        }
        if !level.set_block(position, state, settings.update_mode) {
            continue;
        }
        min_x = min_x.min(position.x);
        min_y = min_y.min(position.y);
        min_z = min_z.min(position.z);
        max_x = max_x.max(position.x);
        max_y = max_y.max(position.y);
        max_z = max_z.max(position.z);
        placed.push(position);
        if let Some(NbtSummaryValue::Compound(record)) = nbt.as_deref() {
            placed_block_entities.push(position);
            let mut record = record.clone();
            if randomizable_container(state) {
                put_long(&mut record, "LootTableSeed", random.next_long());
            }
            level.load_structure_block_entity(position, state, &record);
        }
        waterlogging.record_success(level, prepared, previous);
    }
    waterlogging.refill(level);

    if !settings.known_shape && min_x <= max_x {
        let mut shape =
            BitSetDiscreteVoxelShape::new(max_x - min_x + 1, max_y - min_y + 1, max_z - min_z + 1);
        for pos in &placed {
            shape.fill(pos.x - min_x, pos.y - min_y, pos.z - min_z);
        }
        level.update_template_shape_at_edge(settings.update_mode, &shape, min_x, min_y, min_z);
        for &pos in &placed {
            let state = level.get_block(pos);
            let new_state = level.update_from_neighbour_shapes(state, pos);
            if state != new_state {
                level.set_block(pos, new_state, settings.update_mode & -2 | 16);
            }
            level.update_structure_neighbors(pos, new_state);
        }
    }
    for pos in placed_block_entities {
        level.structure_block_entity_changed(pos);
    }

    // Blocks and their shape updates precede template entities
    // (`StructureTemplate.java:356-403`).
    if !settings.ignore_entities {
        place_template_entities_with_settings(
            level,
            &template.entities,
            settings.position,
            settings.rotation,
            settings.pivot,
            settings.mirror,
            settings.bounding_box,
            settings.finalize_entities,
        );
    }
    Some(palette)
}

fn randomizable_container(state: i32) -> bool {
    vanilla_block_state_name(state).is_some_and(|name| {
        matches!(
            name,
            "minecraft:barrel"
                | "minecraft:chest"
                | "minecraft:copper_chest"
                | "minecraft:crafter"
                | "minecraft:decorated_pot"
                | "minecraft:dispenser"
                | "minecraft:dropper"
                | "minecraft:hopper"
                | "minecraft:shulker_box"
                | "minecraft:trapped_chest"
        ) || name.ends_with("_copper_chest")
            || name.ends_with("_shulker_box")
    })
}

fn put_long(record: &mut Vec<NbtSummaryEntry>, name: &str, value: i64) {
    if let Some(entry) = record.iter_mut().find(|entry| entry.name == name) {
        entry.value = NbtSummaryValue::Long(value);
    } else {
        record.push(NbtSummaryEntry {
            name: name.into(),
            value: NbtSummaryValue::Long(value),
        });
    }
}

fn structure_block_info(block: &StructureTemplateBlock) -> StructureBlockInfo {
    StructureBlockInfo {
        pos: block.position,
        state: block.state,
        nbt: block.nbt.clone(),
    }
}

fn wrapping_add(left: BlockPos, right: BlockPos) -> BlockPos {
    BlockPos {
        x: left.x.wrapping_add(right.x),
        y: left.y.wrapping_add(right.y),
        z: left.z.wrapping_add(right.z),
    }
}

impl JigsawRotation {
    pub(crate) const fn quarter_turns(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Clockwise90 => 1,
            Self::Clockwise180 => 2,
            Self::CounterClockwise90 => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use bbb_server_datapack::structure_template::{
        StructureTemplateBlock, StructureTemplatePalette,
    };

    use super::*;

    struct BlockEntityLevel {
        blocks: BTreeMap<(i32, i32, i32), i32>,
        loaded: Vec<(BlockPos, i32, Vec<NbtSummaryEntry>)>,
    }

    impl WorldGenLevel for BlockEntityLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.blocks
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or_else(|| catalog_default_state("minecraft:air"))
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, _update_flags: i32) -> bool {
            self.blocks.insert((pos.x, pos.y, pos.z), state) != Some(state)
        }

        fn load_structure_block_entity(
            &mut self,
            pos: BlockPos,
            state: i32,
            record: &[NbtSummaryEntry],
        ) {
            self.loaded.push((pos, state, record.to_vec()));
        }

        fn get_min_y(&self) -> i32 {
            -64
        }

        fn get_max_y(&self) -> i32 {
            319
        }

        fn get_sea_level(&self) -> i32 {
            63
        }

        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}
    }

    #[test]
    fn block_entity_nbt_loads_after_the_block_with_a_fresh_loot_seed() {
        assert_fresh_loot_seed("minecraft:chest");
        assert_fresh_loot_seed("minecraft:copper_chest");
        assert_fresh_loot_seed("minecraft:waxed_oxidized_copper_chest");
        assert_fresh_loot_seed("minecraft:decorated_pot");
        assert_fresh_loot_seed("minecraft:shulker_box");

        let brewing = catalog_default_state("minecraft:brewing_stand");
        let template = MaterializedStructureTemplate {
            size: [1, 1, 1],
            palettes: vec![StructureTemplatePalette {
                blocks: vec![StructureTemplateBlock {
                    position: BlockPos { x: 0, y: 0, z: 0 },
                    state: brewing,
                    nbt: Some(Arc::new(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "Fuel".into(),
                        value: NbtSummaryValue::Byte(20),
                    }]))),
                }],
                jigsaws: Vec::new(),
            }],
            entities: Vec::new(),
        };
        let mut level = BlockEntityLevel {
            blocks: BTreeMap::new(),
            loaded: Vec::new(),
        };
        let mut random = WorldgenRandom::with_seed(7);
        let position = BlockPos { x: 1, y: 2, z: 3 };
        assert!(place_materialized_template(
            &template,
            &mut level,
            MaterializedTemplatePlacement {
                position,
                reference_pos: position,
                rotation: JigsawRotation::None,
                mirror: crate::noise::ruined_portal::RuinedPortalMirror::None,
                pivot: BlockPos { x: 0, y: 0, z: 0 },
                finalize_entities: false,
                ignore_entities: false,
                bounding_box: JigsawBoundingBox {
                    min: position,
                    max: position,
                },
                processors: &[],
                liquid_settings: StructureLiquidSettings::IgnoreWaterlogging,
                update_mode: 2,
                known_shape: true,
            },
            &mut random,
            TemplateSettingsRandom::Positional,
        ));
        assert_eq!(level.loaded.len(), 1);
        assert!(level.loaded[0]
            .2
            .iter()
            .any(|entry| entry.name == "Fuel" && entry.value == NbtSummaryValue::Byte(20)));
        assert!(!level.loaded[0]
            .2
            .iter()
            .any(|entry| entry.name == "LootTableSeed"));
    }

    fn assert_fresh_loot_seed(block_id: &'static str) {
        let chest = catalog_default_state(block_id);
        let template = MaterializedStructureTemplate {
            size: [1, 1, 1],
            palettes: vec![StructureTemplatePalette {
                blocks: vec![StructureTemplateBlock {
                    position: BlockPos { x: 0, y: 0, z: 0 },
                    state: chest,
                    nbt: Some(Arc::new(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "LootTable".into(),
                        value: NbtSummaryValue::String("minecraft:chests/simple_dungeon".into()),
                    }]))),
                }],
                jigsaws: Vec::new(),
            }],
            entities: Vec::new(),
        };
        let mut level = BlockEntityLevel {
            blocks: BTreeMap::new(),
            loaded: Vec::new(),
        };
        let mut expected_random = WorldgenRandom::with_seed(42);
        let expected_seed = expected_random.next_long();
        let mut random = WorldgenRandom::with_seed(42);
        let position = BlockPos { x: 3, y: 4, z: 5 };

        assert!(place_materialized_template(
            &template,
            &mut level,
            MaterializedTemplatePlacement {
                position,
                reference_pos: position,
                rotation: JigsawRotation::None,
                mirror: crate::noise::ruined_portal::RuinedPortalMirror::None,
                pivot: BlockPos { x: 0, y: 0, z: 0 },
                finalize_entities: false,
                ignore_entities: false,
                bounding_box: JigsawBoundingBox {
                    min: position,
                    max: position,
                },
                processors: &[],
                liquid_settings: StructureLiquidSettings::IgnoreWaterlogging,
                update_mode: 18,
                known_shape: true,
            },
            &mut random,
            TemplateSettingsRandom::Positional,
        ));
        assert_eq!(level.get_block(position), chest);
        assert_eq!(level.loaded.len(), 1);
        assert!(level.loaded[0].2.iter().any(|entry| {
            entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(expected_seed)
        }));
    }
}
