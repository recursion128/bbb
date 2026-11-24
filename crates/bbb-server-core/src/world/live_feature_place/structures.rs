//! Live template and jigsaw command placement.

use super::*;
use crate::block_entity::structure_block::StructureBlockEntity;
use crate::world::LevelId;
use bbb_protocol::NbtSummaryValue;
use bbb_server_datapack::structure_pool::{StructureLiquidSettings, StructurePoolSet};
use bbb_server_datapack::structure_template::MaterializedStructureTemplate;
use bbb_server_worldgen::noise::ruined_portal::RuinedPortalMirror;
use bbb_server_worldgen::noise::structure_pieces::{
    jigsaw_pool_piece_graph, JigsawBoundingBox, JigsawRotation, JigsawTemplateAccess,
};
use bbb_server_worldgen::structure_processor::StructureProcessor;
use bbb_server_worldgen::structure_template_placement::{
    place_materialized_template, MaterializedTemplatePlacement, TemplateSettingsRandom,
};

use crate::commands::translatable;

pub(in crate::world) struct LiveTemplateSettings {
    pub(in crate::world) rotation: JigsawRotation,
    pub(in crate::world) mirror: RuinedPortalMirror,
    pub(in crate::world) integrity: f32,
    pub(in crate::world) seed: i64,
    pub(in crate::world) strict: bool,
    pub(in crate::world) ignore_entities: bool,
}

impl LiveTemplateSettings {
    pub(in crate::world) fn for_structure_block(block: &StructureBlockEntity) -> Self {
        use bbb_protocol::packets::{StructureBlockMirror, StructureBlockRotation};
        Self {
            rotation: match block.rotation() {
                StructureBlockRotation::None => JigsawRotation::None,
                StructureBlockRotation::Clockwise90 => JigsawRotation::Clockwise90,
                StructureBlockRotation::Clockwise180 => JigsawRotation::Clockwise180,
                StructureBlockRotation::Counterclockwise90 => JigsawRotation::CounterClockwise90,
            },
            mirror: match block.mirror() {
                StructureBlockMirror::None => RuinedPortalMirror::None,
                StructureBlockMirror::LeftRight => RuinedPortalMirror::LeftRight,
                StructureBlockMirror::FrontBack => RuinedPortalMirror::FrontBack,
            },
            integrity: block.integrity().clamp(0.0, 1.0),
            seed: block.seed(),
            strict: block.strict(),
            ignore_entities: block.ignore_entities(),
        }
    }
}

pub(in crate::world) fn template_world_chunks(
    position: BlockPos,
    size: [i32; 3],
    settings: &LiveTemplateSettings,
) -> Vec<(i32, i32)> {
    if size[0] < 1 || size[1] < 1 || size[2] < 1 {
        return Vec::new();
    }
    let pivot = BlockPos { x: 0, y: 0, z: 0 };
    let corners = [
        BlockPos { x: 0, y: 0, z: 0 },
        BlockPos {
            x: size[0] - 1,
            y: 0,
            z: 0,
        },
        BlockPos {
            x: 0,
            y: 0,
            z: size[2] - 1,
        },
        BlockPos {
            x: size[0] - 1,
            y: 0,
            z: size[2] - 1,
        },
    ];
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_z = i32::MAX;
    let mut max_z = i32::MIN;
    for corner in corners {
        let transformed = bbb_server_worldgen::noise::ruined_portal::transform(
            corner,
            settings.mirror,
            settings.rotation,
            pivot,
        );
        let wx = position.x.wrapping_add(transformed.x);
        let wz = position.z.wrapping_add(transformed.z);
        min_x = min_x.min(wx);
        max_x = max_x.max(wx);
        min_z = min_z.min(wz);
        max_z = max_z.max(wz);
    }
    let mut chunks = Vec::new();
    for cz in (min_z >> 4)..=(max_z >> 4) {
        for cx in (min_x >> 4)..=(max_x >> 4) {
            chunks.push((cx, cz));
        }
    }
    chunks
}

pub(in crate::world) fn place_live_materialized_template(
    template: &MaterializedStructureTemplate,
    level: &mut LevelCtx<'_, '_>,
    position: BlockPos,
    settings: &LiveTemplateSettings,
) -> bool {
    // A zero seed is a fresh random placement.
    let seed = match settings.seed {
        0 => level.level_random().next_long(),
        seed => seed,
    };
    let mut placement_random = &GameRandom::with_seed(seed);
    let mut settings_random = &GameRandom::with_seed(seed);
    let processors = if settings.integrity < 1.0 {
        vec![StructureProcessor::BlockRot {
            rottable_blocks: None,
            integrity: settings.integrity,
        }]
    } else {
        Vec::new()
    };
    place_materialized_template(
        template,
        level,
        MaterializedTemplatePlacement {
            position,
            reference_pos: position,
            rotation: settings.rotation,
            mirror: settings.mirror,
            pivot: BlockPos { x: 0, y: 0, z: 0 },
            finalize_entities: false,
            ignore_entities: settings.ignore_entities,
            bounding_box: infinite_bounds(),
            processors: &processors,
            liquid_settings: StructureLiquidSettings::ApplyWaterlogging,
            update_mode: 2 | if settings.strict { 816 } else { 0 },
            known_shape: settings.strict,
        },
        &mut placement_random,
        if settings.integrity < 1.0 {
            TemplateSettingsRandom::Separate(&mut settings_random)
        } else {
            TemplateSettingsRandom::Positional
        },
    )
}

impl ServerWorld {
    pub(crate) fn place_live_command(
        &mut self,
        request: &crate::commands::PlaceCommandRequest,
    ) -> Result<(), NbtSummaryValue> {
        match &request.kind {
            crate::commands::PlaceCommandKind::Structure { id } => self.place_live_structure(
                self.require_level_id(&request.level),
                id,
                request.pos,
                request.check_position,
            ),
            crate::commands::PlaceCommandKind::Jigsaw {
                pool,
                target,
                max_depth,
            } => self.place_live_jigsaw(
                self.require_level_id(&request.level),
                pool,
                target,
                *max_depth,
                request.pos,
                request.check_position,
            ),
            crate::commands::PlaceCommandKind::Template {
                id,
                rotation,
                mirror,
                integrity,
                seed,
                strict,
            } => self.place_live_template(
                self.require_level_id(&request.level),
                id,
                request.pos,
                *rotation,
                *mirror,
                *integrity,
                *seed,
                *strict,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn place_live_template(
        &mut self,
        dimension: LevelId,
        id: &str,
        position: BlockPos,
        rotation: JigsawRotation,
        mirror: RuinedPortalMirror,
        integrity: f32,
        seed: i32,
        strict: bool,
    ) -> Result<(), NbtSummaryValue> {
        self.place_live_template_inner(
            dimension,
            id,
            position,
            &LiveTemplateSettings {
                rotation,
                mirror,
                integrity,
                seed: i64::from(seed),
                strict,
                ignore_entities: false,
            },
            true,
        )
    }

    pub(in crate::world) fn place_structure_block_template(
        &mut self,
        dimension: LevelId,
        position: BlockPos,
        block: &StructureBlockEntity,
    ) {
        let settings = LiveTemplateSettings::for_structure_block(block);
        let _ = self.place_live_template_inner(dimension, block.name(), position, &settings, false);
    }

    fn place_live_template_inner(
        &mut self,
        dimension: LevelId,
        id: &str,
        position: BlockPos,
        settings: &LiveTemplateSettings,
        require_loaded: bool,
    ) -> Result<(), NbtSummaryValue> {
        let template = self
            .cold
            .structure_templates
            .get(id, self.datapacks().map(|packs| &packs.structures))
            .ok_or_else(|| {
                translatable(
                    "commands.place.template.invalid",
                    vec![bbb_protocol::component_literal_nbt(id)],
                )
            })?;
        // PlaceCommand.java:344: the loaded gate uses the unrotated size.
        let far = BlockPos {
            x: position.x.wrapping_add(template.size[0]),
            y: position.y.wrapping_add(template.size[1]),
            z: position.z.wrapping_add(template.size[2]),
        };
        if require_loaded {
            self.check_place_loaded(&dimension, position, far)?;
        }
        let target_chunks = template_world_chunks(position, template.size, settings);
        let _ = self.install_full_chunks_by_key(&dimension, &target_chunks);
        let placed = self.with_live_placement(dimension, |level, _| {
            place_live_materialized_template(template.materialized(), level, position, settings)
        });
        if placed {
            Ok(())
        } else {
            Err(translatable("commands.place.template.failed", Vec::new()))
        }
    }

    pub(in crate::world) fn place_live_jigsaw(
        &mut self,
        dimension: LevelId,
        pool: &str,
        target: &str,
        max_depth: i32,
        position: BlockPos,
        check_position: bool,
    ) -> Result<(), NbtSummaryValue> {
        self.place_live_jigsaw_inner(
            dimension,
            pool,
            target,
            max_depth,
            position,
            Some(check_position),
            false,
        )
    }

    pub(in crate::world) fn generate_live_jigsaw(
        &mut self,
        dimension: LevelId,
        pool: &str,
        target: &str,
        max_depth: i32,
        position: BlockPos,
        keep_jigsaws: bool,
    ) -> Result<(), NbtSummaryValue> {
        self.place_live_jigsaw_inner(
            dimension,
            pool,
            target,
            max_depth,
            position,
            None,
            keep_jigsaws,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn place_live_jigsaw_inner(
        &mut self,
        dimension: LevelId,
        pool: &str,
        target: &str,
        max_depth: i32,
        position: BlockPos,
        command_position: Option<bool>,
        keep_jigsaws: bool,
    ) -> Result<(), NbtSummaryValue> {
        let datapacks = self
            .datapacks_shared()
            .ok_or_else(|| translatable("commands.place.jigsaw.failed", Vec::new()))?;
        let pools = StructurePoolSet::from_generation(&datapacks.registries)
            .map_err(|_| translatable("commands.place.jigsaw.failed", Vec::new()))?;
        if !pools.contains(pool) {
            return Err(translatable(
                "commands.place.jigsaw.invalid",
                vec![bbb_protocol::component_literal_nbt(pool)],
            ));
        }
        if let Some(check_position) = command_position {
            self.check_explicit_place_position(&dimension, position, check_position)?;
            self.check_place_loaded(&dimension, position, position)?;
        }
        let graph = {
            let chunks = &self.level_by_id(&dimension).chunks;
            jigsaw_pool_piece_graph(
                &pools,
                &CommandTemplates(self),
                pool,
                target,
                max_depth,
                position,
                self.cold.seed,
                chunks.min_y(),
                chunks.section_count() as i32 * 16,
                chunks,
            )
        }
        .ok_or_else(|| translatable("commands.place.jigsaw.failed", Vec::new()))?;
        let mut target_chunks = std::collections::BTreeSet::new();
        for bounds in std::iter::once(graph.center_bounding_box)
            .chain(graph.pieces.iter().map(|piece| piece.bounding_box))
        {
            for cz in (bounds.min.z >> 4)..=(bounds.max.z >> 4) {
                for cx in (bounds.min.x >> 4)..=(bounds.max.x >> 4) {
                    target_chunks.insert((cx, cz));
                }
            }
        }
        let target_chunks: Vec<(i32, i32)> = target_chunks.into_iter().collect();
        let _ = self.install_full_chunks_by_key(&dimension, &target_chunks);
        self.with_live_placement(dimension, |level, random| {
            graph.place_jigsaw_with_keep_jigsaws(&datapacks, level, position, random, keep_jigsaws)
        });
        Ok(())
    }

    fn place_live_structure(
        &mut self,
        dimension: LevelId,
        id: &str,
        position: BlockPos,
        check_position: bool,
    ) -> Result<(), NbtSummaryValue> {
        let failed = || translatable("commands.place.structure.failed", Vec::new());
        let datapacks = self.datapacks_shared().ok_or_else(failed)?;
        if datapacks
            .registries
            .element("minecraft:worldgen/structure", id)
            .is_none()
        {
            return Err(translatable(
                "commands.place.structure.invalid",
                vec![bbb_protocol::component_literal_nbt(id)],
            ));
        }
        self.check_explicit_place_position(&dimension, position, check_position)?;
        let chunks = &self.level_by_id(&dimension).chunks;
        let adjustment = chunks
            .command_structure_terrain_adjustment(id)
            .map_err(|_| failed())?;
        let mut start = chunks
            .generate_forced_structure_start(
                id,
                position.x >> 4,
                position.z >> 4,
                &CommandTemplates(self),
            )
            .map_err(|_| failed())?;
        let bounds = start
            .forced_structure_bounding_box(adjustment)
            .ok_or_else(failed)?;
        self.check_place_loaded(&dimension, bounds.min, bounds.max)?;
        // PlaceCommand.java:300-313: one LevelRandom across the closed chunk range.
        self.with_live_placement(dimension, |level, random| {
            for z in (bounds.min.z >> 4)..=(bounds.max.z >> 4) {
                for x in (bounds.min.x >> 4)..=(bounds.max.x >> 4) {
                    let chunk_bounds = JigsawBoundingBox {
                        min: BlockPos {
                            x: x << 4,
                            y: level.get_min_y(),
                            z: z << 4,
                        },
                        max: BlockPos {
                            x: (x << 4) + 15,
                            y: level.get_max_y() + 1,
                            z: (z << 4) + 15,
                        },
                    };
                    start.place_forced_structure_in_chunk(&datapacks, level, chunk_bounds, random);
                }
            }
            Ok(())
        })
    }

    fn check_explicit_place_position(
        &self,
        dimension: &LevelId,
        position: BlockPos,
        explicit: bool,
    ) -> Result<(), NbtSummaryValue> {
        if explicit {
            self.check_place_loaded(dimension, position, position)?;
            if !crate::world::level_blocks::is_in_world_bounds(
                &self.level_by_id(dimension).chunks,
                position,
            ) {
                return Err(translatable("argument.pos.outofworld", Vec::new()));
            }
        }
        Ok(())
    }

    fn check_place_loaded(
        &self,
        dimension: &LevelId,
        min: BlockPos,
        max: BlockPos,
    ) -> Result<(), NbtSummaryValue> {
        for z in (min.z >> 4)..=(max.z >> 4) {
            for x in (min.x >> 4)..=(max.x >> 4) {
                if !self.level_by_id(dimension).chunks.is_ticking_full(x, z) {
                    return Err(translatable("argument.pos.unloaded", Vec::new()));
                }
            }
        }
        Ok(())
    }
}

struct CommandTemplates<'a>(&'a ServerWorld);

impl JigsawTemplateAccess for CommandTemplates<'_> {
    fn structure_template(
        &self,
        id: &str,
    ) -> Option<bbb_server_datapack::structure_template::StructureTemplate> {
        self.0
            .cold
            .structure_templates
            .get(id, self.0.datapacks().map(|packs| &packs.structures))
    }
}

fn infinite_bounds() -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: i32::MIN,
            y: i32::MIN,
            z: i32::MIN,
        },
        max: BlockPos {
            x: i32::MAX,
            y: i32::MAX,
            z: i32::MAX,
        },
    }
}
