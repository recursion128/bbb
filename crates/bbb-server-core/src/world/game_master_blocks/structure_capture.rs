use crate::world::LevelId;
use bbb_server_types::Result as CoreResult;
use std::sync::Arc;

use crate::block_entity::structure_block::StructureBlockEntity;
use crate::entity::store::EntityBodyReadView;
use bbb_protocol::{packets::BlockPos, NbtSummaryValue};
use bbb_server_datapack::structure_template::{
    MaterializedStructureTemplate, StructureTemplate, StructureTemplateBlock,
    StructureTemplateEntity, StructureTemplatePalette,
};
use bbb_server_types::geometry::Aabb;

use super::structure_block::offset;
use crate::world::{registry_runtime::PlayerItemStackResolver, ServerWorld};

pub(in crate::world) enum StructureCaptureTarget<'a, 'l, 'w> {
    World(WorldCaptureHost<'a>),
    Level(&'a mut crate::world::simulation_level::LevelCtx<'l, 'w>),
}

impl StructureCaptureTarget<'_, '_, '_> {
    fn block(&mut self, pos: BlockPos) -> CoreResult<(i32, Option<Arc<NbtSummaryValue>>)> {
        match self {
            Self::World(host) => host.block(pos),
            Self::Level(level) => {
                if !level.is_in_world_bounds(pos) {
                    return Ok((level.chunks.air_state_id(), None));
                }
                level.chunks.load_column(pos.x >> 4, pos.z >> 4);
                let state = level.block_state(pos);
                if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
                    == Some(bbb_protocol::block_types::VANILLA_BLOCK_STRUCTURE_VOID_ID)
                {
                    return Ok((state, None));
                }
                let facts = &level.loot_level_facts;
                let nbt = level
                    .chunks
                    .block_entities()
                    .get(pos)
                    .map(|entity| {
                        let mut entries = entity.save_with_full_metadata_with_resolver(
                            facts
                                .item_stack_resolver
                                .as_ref()
                                .expect("live structure resolver"),
                        )?;
                        entries.retain(|entry| {
                            entry.name != "x" && entry.name != "y" && entry.name != "z"
                        });
                        Ok::<_, bbb_server_types::ServerWorldError>(Arc::new(
                            NbtSummaryValue::Compound(entries),
                        ))
                    })
                    .transpose()?;
                Ok((state, nbt))
            }
        }
    }

    fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        match self {
            Self::World(host) => host.world.registries.content_registry(),
            Self::Level(level) => level.chunks.content_registry(),
        }
    }

    fn entities(
        &mut self,
        origin: BlockPos,
        size: BlockPos,
    ) -> Option<Vec<StructureTemplateEntity>> {
        match self {
            Self::World(host) => host.entities(origin, size),
            Self::Level(level) => {
                let context = level.structure_entities.as_ref()?;
                capture_structure_entities(
                    crate::entity::store::EntityBodyReadView::new(
                        context.store,
                        context.current,
                        context.ancestors,
                    ),
                    origin,
                    size,
                )
            }
        }
    }
}

pub(in crate::world) fn capture_structure_template(
    host: &mut StructureCaptureTarget<'_, '_, '_>,
    origin: BlockPos,
    block: &StructureBlockEntity,
    mut template: StructureTemplate,
) -> CoreResult<Option<StructureTemplate>> {
    let size = block.size();
    if size.x >= 1 && size.y >= 1 && size.z >= 1 {
        let mut blocks = Vec::new();
        for y in 0..size.y {
            for z in 0..size.z {
                for x in 0..size.x {
                    let relative = BlockPos { x, y, z };
                    let (state, nbt) = host.block(offset(origin, relative))?;
                    if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
                        == Some(bbb_protocol::block_types::VANILLA_BLOCK_STRUCTURE_VOID_ID)
                    {
                        continue;
                    }
                    blocks.push(StructureTemplateBlock {
                        position: relative,
                        state,
                        nbt,
                    });
                }
            }
        }
        let entities = if block.ignore_entities() {
            Vec::new()
        } else {
            let Some(entities) = host.entities(origin, size) else {
                return Ok(None);
            };
            entities
        };
        template = match StructureTemplate::from_materialized_with_content_registry(
            MaterializedStructureTemplate {
                size: [size.x, size.y, size.z],
                palettes: vec![StructureTemplatePalette {
                    blocks,
                    jigsaws: Vec::new(),
                }],
                entities,
            },
            host.content_registry(),
        ) {
            Ok(template) => template,
            Err(error) => {
                tracing::warn!(%error, "could not capture structure template");
                return Ok(None);
            }
        };
    }
    template.set_author(block.author().to_owned());
    Ok(Some(template))
}

pub(in crate::world) fn capture_structure_entities(
    view: EntityBodyReadView<'_>,
    origin: BlockPos,
    size: BlockPos,
) -> Option<Vec<StructureTemplateEntity>> {
    let bounds = Aabb {
        min_x: origin.x as f64,
        min_y: origin.y as f64,
        min_z: origin.z as f64,
        max_x: origin.x as f64 + size.x as f64,
        max_y: origin.y as f64 + size.y as f64,
        max_z: origin.z as f64 + size.z as f64,
    };
    let mut entities = Vec::new();
    let ids = view.ids_in_box_sections(bounds);
    for entity in ids
        .into_iter()
        .filter_map(|id| view.get(id))
        .filter(|entity| entity.bounding_box().intersects(bounds))
    {
        let position = entity.position();
        let relative = [
            position.x - origin.x as f64,
            position.y - origin.y as f64,
            position.z - origin.z as f64,
        ];
        let block_position = if let crate::entity::EntityData::Painting(painting) = entity.data() {
            let attached = painting.pose().attached();
            BlockPos {
                x: attached.x - origin.x,
                y: attached.y - origin.y,
                z: attached.z - origin.z,
            }
        } else {
            BlockPos {
                x: relative[0].floor() as i32,
                y: relative[1].floor() as i32,
                z: relative[2].floor() as i32,
            }
        };
        let payload = crate::entity::save::write_structure_template_entity(view, entity);
        let nbt = match payload {
            Ok(nbt) => nbt,
            Err(error) => {
                tracing::warn!(%error, "could not capture structure template entity");
                return None;
            }
        };
        entities.push(StructureTemplateEntity {
            position_bits: relative.map(f64::to_bits),
            block_position,
            nbt,
        });
    }
    Some(entities)
}

pub(in crate::world) struct WorldCaptureHost<'a> {
    world: &'a mut ServerWorld,
    dimension: &'a LevelId,
    resolver: PlayerItemStackResolver,
}

impl WorldCaptureHost<'_> {
    fn block(&mut self, pos: BlockPos) -> CoreResult<(i32, Option<Arc<NbtSummaryValue>>)> {
        let state = self
            .world
            .block_state_by_key(self.dimension, pos)
            .unwrap_or(0);
        if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
            == Some(bbb_protocol::block_types::VANILLA_BLOCK_STRUCTURE_VOID_ID)
        {
            return Ok((state, None));
        }
        let nbt = self
            .world
            .level_by_id(self.dimension)
            .chunks
            .block_entities()
            .get(pos)
            .map(|entity| {
                let mut entries = entity.save_with_full_metadata_with_resolver(&self.resolver)?;
                entries.retain(|entry| entry.name != "x" && entry.name != "y" && entry.name != "z");
                Ok::<_, bbb_server_types::ServerWorldError>(Arc::new(NbtSummaryValue::Compound(
                    entries,
                )))
            })
            .transpose()?;
        Ok((state, nbt))
    }

    fn entities(
        &mut self,
        origin: BlockPos,
        size: BlockPos,
    ) -> Option<Vec<StructureTemplateEntity>> {
        if self.world.datapacks().is_some() {
            let bounds = Aabb {
                min_x: origin.x as f64,
                min_y: origin.y as f64,
                min_z: origin.z as f64,
                max_x: origin.x as f64 + size.x as f64,
                max_y: origin.y as f64 + size.y as f64,
                max_z: origin.z as f64 + size.z as f64,
            };
            let roots: Vec<i32> = {
                let store = &self.world.level_by_id(self.dimension).entities;
                store
                    .ids_in_box_sections(bounds)
                    .into_iter()
                    .filter(|&id| {
                        store
                            .get(id)
                            .is_some_and(|entity| entity.bounding_box().intersects(bounds))
                    })
                    .collect()
            };
            if let Err(error) = self
                .world
                .materialize_merchant_offers_in_trees(self.dimension, roots)
            {
                tracing::warn!(%error, "could not materialize merchant offers for structure capture");
            }
        }
        capture_structure_entities(
            EntityBodyReadView::new(&self.world.level_by_id(self.dimension).entities, None, None),
            origin,
            size,
        )
    }
}

impl ServerWorld {
    pub(super) fn save_structure_block(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        block: &StructureBlockEntity,
        save_to_disk: bool,
    ) -> bool {
        if block.name().is_empty() {
            return false;
        }
        let Some(template) = self.cold.structure_templates.get_or_create(
            block.name(),
            self.datapacks().map(|packs| &packs.structures),
        ) else {
            return false;
        };
        let origin = offset(pos, block.offset());
        let size = block.size();
        if size.x >= 1 && size.y >= 1 && size.z >= 1 {
            let max_x = origin.x.saturating_add(size.x - 1);
            let max_z = origin.z.saturating_add(size.z - 1);
            let mut chunks = Vec::new();
            for cz in (origin.z >> 4)..=(max_z >> 4) {
                for cx in (origin.x >> 4)..=(max_x >> 4) {
                    chunks.push((cx, cz));
                }
            }
            let _ = self.install_full_chunks_by_key(dimension, &chunks);
        }
        let resolver = self.item_stack_resolver();
        let mut host = StructureCaptureTarget::World(WorldCaptureHost {
            world: self,
            dimension,
            resolver,
        });
        let template = capture_structure_template(&mut host, origin, block, template);
        let template = match template {
            Ok(Some(template)) => template,
            Ok(None) => return false,
            Err(error) => {
                tracing::warn!("structure capture: {error}");
                return false;
            }
        };
        self.cold.structure_templates.put(block.name(), template)
            && (!save_to_disk || self.cold.structure_templates.save(block.name()))
    }
}
