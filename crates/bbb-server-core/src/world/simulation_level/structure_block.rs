use crate::block_entity::BlockEntity;
use crate::entity::{Entity, EntityStore};
use crate::neighbor_updater::NeighborUpdate;
use bbb_protocol::packets::{BlockPos, StructureBlockMode};

use super::LevelCtx;
use crate::world::{
    game_master_blocks::structure_capture::{capture_structure_template, StructureCaptureTarget},
    live_feature_place::{place_live_materialized_template, LiveTemplateSettings},
    registry_runtime::EntityNbtReadSnapshot,
    structure_templates::StructureTemplateManager,
};

pub(in crate::world) struct StructureBlockServices<'a> {
    pub(in crate::world) manager: &'a StructureTemplateManager,
    pub(in crate::world) entity_nbt: EntityNbtReadSnapshot,
    pub(in crate::world) fast_lava: Option<bool>,
    pub(in crate::world) piglins_zombify:
        Option<crate::world::environment_attributes::BiomeBoolean>,
    pub(in crate::world) monsters_burn: Option<crate::world::environment_attributes::BiomeBoolean>,
    pub(in crate::world) water_evaporates:
        Option<crate::world::environment_attributes::BiomeBoolean>,
    pub(in crate::world) bees_stay_in_hive:
        Option<crate::world::environment_attributes::BiomeBoolean>,
    /// The adult and baby villager schedules' activity this tick.
    pub(in crate::world) villager_activity: [Option<crate::entity::ai::Activity>; 2],
    /// `Timelines.OVERWORLD_DAY`'s period count: the overworld clock's days.
    pub(in crate::world) overworld_day: i64,
}

pub(in crate::world) struct StructureEntityContext<'a> {
    pub(in crate::world) store: &'a mut EntityStore,
    pub(in crate::world) current: Option<&'a Entity>,
    pub(in crate::world) ancestors: Option<&'a crate::entity::store::DetachedEntityBodies<'a>>,
}

impl StructureEntityContext<'_> {
    pub(super) fn reborrow(&mut self) -> StructureEntityContext<'_> {
        StructureEntityContext {
            store: self.store,
            current: self.current,
            ancestors: self.ancestors,
        }
    }
}

impl<'a> LevelCtx<'a, '_> {
    /// A body a caller frame lifted out of the store, such as a mob mid-attack.
    pub(crate) fn detached_entity(&self, id: i32) -> Option<&'a Entity> {
        let mut frame = self.detached_entities;
        while let Some(bodies) = frame {
            if bodies.body.id() == id {
                return Some(bodies.body);
            }
            frame = bodies.parent;
        }
        None
    }
}

impl<'w> LevelCtx<'_, 'w> {
    pub(crate) fn with_detached_entity<R>(
        &mut self,
        body: &Entity,
        run: impl FnOnce(&mut LevelCtx<'_, 'w>) -> R,
    ) -> R {
        let frame = crate::entity::store::DetachedEntityBodies {
            body,
            parent: self.detached_entities,
        };
        let mut level = self.reborrow();
        level.detached_entities = Some(&frame);
        run(&mut level)
    }

    pub(in crate::world) fn with_structure_entities<'a>(
        &'a mut self,
        store: &'a mut EntityStore,
        current: Option<&'a Entity>,
    ) -> LevelCtx<'a, 'w> {
        let mut level = self.reborrow();
        level.structure_entities = Some(StructureEntityContext {
            store,
            current,
            ancestors: None,
        });
        level
    }

    pub(crate) fn with_entity_context<'a>(
        &'a mut self,
        context: crate::entity::level::EntityBlockContext<'a>,
    ) -> LevelCtx<'a, 'w> {
        let ancestors = self.detached_entities.or(context.ancestors);
        let mut level = self.reborrow();
        if let Some(store) = context.store {
            level.structure_entities = Some(StructureEntityContext {
                store,
                current: context.current,
                ancestors,
            });
        }
        level
    }

    pub(super) fn dispatch_neighbors(
        &mut self,
        dispatch: impl FnOnce(&mut crate::neighbor_updater::NeighborUpdates),
    ) {
        dispatch(self.neighbours);
        if !self.neighbours.begin() {
            return;
        }
        while let Some(update) = self.neighbours.pop() {
            match update {
                NeighborUpdate::Block(update) => super::execute_neighbor_update(self, &update),
                NeighborUpdate::Shape(update) => super::execute_shape_update(self, &update),
            }
        }
        self.neighbours.finish();
    }
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if level.structure_services.is_none() || level.structure_entities.is_none() {
        return;
    }
    let powered = level.has_neighbor_signal(pos);
    let Some(block) = level
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .and_then(BlockEntity::structure_block_mut)
    else {
        return;
    };
    let previous = block.powered();
    if previous == powered {
        return;
    }
    block.set_powered(powered);
    let block = block.clone();
    if !powered {
        return;
    }
    let manager = level
        .structure_services
        .as_ref()
        .expect("structure services")
        .manager;
    let datapacks = level
        .loot_level_facts
        .datapacks
        .map(|packs| &packs.structures);
    let origin = BlockPos {
        x: pos.x.wrapping_add(block.offset().x),
        y: pos.y.wrapping_add(block.offset().y),
        z: pos.z.wrapping_add(block.offset().z),
    };
    match block.mode() {
        StructureBlockMode::Save => {
            if let Some(template) = manager.get_or_create(block.name(), datapacks) {
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
                    level.full_preparation.prepare_full_chunks(
                        level.chunks,
                        &chunks,
                        level.game_time,
                        level.item_spawns,
                    );
                }
                match capture_structure_template(
                    &mut StructureCaptureTarget::Level(level),
                    origin,
                    &block,
                    template,
                ) {
                    Ok(Some(template)) => {
                        manager.put(block.name(), template);
                    }
                    Ok(None) => {}
                    Err(error) => level.record_item_failure(pos, error),
                }
            }
        }
        StructureBlockMode::Load => {
            if let Some(template) = manager.get(block.name(), datapacks) {
                if let Some(entity) = level
                    .chunks
                    .block_entities_mut()
                    .get_mut(pos)
                    .and_then(BlockEntity::structure_block_mut)
                {
                    entity.set_author(template.author().to_owned());
                    entity.set_size(BlockPos {
                        x: template.size[0],
                        y: template.size[1],
                        z: template.size[2],
                    });
                }
                level.chunks.block_entities_mut().set_changed(pos);
                let settings = LiveTemplateSettings::for_structure_block(&block);
                let chunks = crate::world::live_feature_place::template_world_chunks(
                    origin,
                    template.size,
                    &settings,
                );
                level.full_preparation.prepare_full_chunks(
                    level.chunks,
                    &chunks,
                    level.game_time,
                    level.item_spawns,
                );
                place_live_materialized_template(template.materialized(), level, origin, &settings);
            }
        }
        StructureBlockMode::Corner => manager.remove(block.name()),
        StructureBlockMode::Data => {}
    }
}
