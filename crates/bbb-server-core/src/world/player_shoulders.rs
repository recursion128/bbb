//! Player shoulder entities own their complete saved bodies and metadata projection.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityRecord;
use uuid::Uuid;

use super::{LevelId, PlayerState, ServerWorld, WorldEvent};
use crate::connection::ConnectionId;
use crate::entity::save::{read_player_entity_tree, write_player_entity_tree};
use crate::entity::store::EntityBodyReadView;
use crate::entity::RemovalReason;

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct PlayerShoulders {
    pub(super) entities: [Vec<NbtSummaryEntry>; 2],
    sat_at: i64,
    pub(super) dirty: bool,
}

impl PlayerShoulders {
    pub(super) fn from_saved(entities: [Vec<NbtSummaryEntry>; 2]) -> Self {
        let dirty = entities.iter().any(|entries| !entries.is_empty());
        Self {
            entities,
            dirty,
            sat_at: 0,
        }
    }

    fn ready_to_release(&self, time: i64) -> bool {
        self.entities.iter().any(|entries| !entries.is_empty())
            && self.sat_at.saturating_add(20) < time
    }

    pub(super) fn variants(&self) -> [Option<i32>; 2] {
        self.entities.each_ref().map(|entries| {
            let field = |name: &str| {
                entries
                    .iter()
                    .rev()
                    .find(|entry| entry.name == name)
                    .map(|entry| &entry.value)
            };
            let id = match field("id")? {
                NbtSummaryValue::String(id) => id,
                _ => return None,
            };
            if id != "minecraft:parrot" && id != "parrot" {
                return None;
            }
            let variant = bbb_protocol::nbt_summary::codec_int_value(field("Variant")?)?;
            let variants =
                bbb_protocol::builtin_enum_component_values::VANILLA_PARROT_VARIANT_VALUES;
            // `Parrot.Variant.LEGACY_CODEC` clamps numeric ids (`Parrot.java:522-524`).
            Some(variant.clamp(variants.first()?.id, variants.last()?.id))
        })
    }
}

impl PlayerState {
    pub(crate) fn can_host_shoulder(&self) -> bool {
        !self.is_spectator()
            && !self.abilities.flying
            && !self.was_touching_water
            && !self.block_contact.in_powder_snow
    }
}

impl ServerWorld {
    pub(super) fn land_on_player_shoulder(
        &mut self,
        dimension: LevelId,
        entity_id: i32,
        owner: Uuid,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(connection) = self.connection_of(owner) else {
            return;
        };
        let store = &self.level_by_id(&dimension).entities;
        let Some(entity) = store.get(entity_id).filter(|entity| {
            entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PARROT_ID
        }) else {
            return;
        };
        if crate::entity::ai::mob::parrot::landing_owner(entity, store) != Some(owner) {
            return;
        }
        let Some(player) = store.player_by_uuid(owner) else {
            return;
        };
        if !player.on_ground() || store.vehicle(player.entity_id()).is_some() {
            return;
        }
        let Some(side) = player
            .cold
            .shoulders
            .entities
            .iter()
            .position(Vec::is_empty)
        else {
            return;
        };
        let Ok(record) =
            write_player_entity_tree(EntityBodyReadView::new(store, None, None), entity)
        else {
            return;
        };
        let time = store.game_time();
        let player = &mut self.levels[connection];
        player.cold.shoulders.entities[side] = record.into_raw();
        player.cold.shoulders.sat_at = time;
        player.cold.shoulders.dirty = true;
        self.level_mut_by_id(&dimension)
            .entities
            .remove(entity_id, RemovalReason::Discarded);
        self.apply_level_entity_lifecycle_by_key(&dimension, events);
    }

    // `ServerPlayer.removeEntitiesOnShoulder` (`ServerPlayer.java:804-809`).
    pub(super) fn remove_player_shoulders(
        &mut self,
        connection: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection) else {
            return;
        };
        let dimension = player.dimension;
        let time = self.level_by_id(&dimension).entities.game_time();
        if !player.cold.shoulders.ready_to_release(time) {
            return;
        }
        let owner = player.profile.uuid;
        let position = player.position();
        let records = std::mem::take(&mut self.levels[connection].cold.shoulders.entities);
        self.levels[connection].cold.shoulders.dirty = true;
        let input = self.entity_nbt_read_snapshot();
        for entries in records {
            if entries.is_empty() {
                continue;
            }
            let Some(record) = Java26EntityRecord::parse(&entries) else {
                continue;
            };
            let store = &mut self.level_mut_by_id(&dimension).entities;
            let Some(id) = read_player_entity_tree(store, record.as_ref(), input.context()) else {
                continue;
            };
            let Some(mut entity) = store.take_fresh(id) else {
                continue;
            };
            crate::entity::ai::mob::parrot::restore_owner(&mut entity, owner);
            entity.set_pos(bbb_protocol::packets::Vec3d {
                x: position.x,
                y: position.y + f64::from(0.7_f32),
                z: position.z,
            });
            store.add_fresh(entity);
        }
        self.apply_level_entity_lifecycle_by_key(&dimension, events);
    }

    pub(super) fn tick_player_shoulders(&mut self, events: &mut Vec<WorldEvent>) {
        let difficulty = self.admin.settings().difficulty;
        let mut sounds = Vec::new();
        for player in self.levels.players() {
            let level = self.level_by_id(&player.dimension);
            let bodies = EntityBodyReadView::new(&level.entities, None, None);
            for entries in &player.cold.shoulders.entities {
                if let Some(sound) = crate::entity::parrot_sounds::shoulder_sound(
                    entries,
                    player.body(),
                    player.is_alive() && !player.silent,
                    bodies,
                    &level.random,
                    difficulty,
                ) {
                    sounds.push((player.dimension, sound));
                }
            }
        }
        for (dimension, sound) in sounds {
            self.level_mut_by_id(&dimension)
                .entities
                .play_entity_sound(sound);
        }
        let connections = self
            .levels
            .players()
            .filter(|player| {
                player
                    .cold
                    .shoulders
                    .ready_to_release(self.level_by_id(&player.dimension).entities.game_time())
                    && (player.block_contact.fall_distance > 0.5
                        || player.was_touching_water
                        || player.abilities.flying
                        || player.cold.sleeping_pos.is_some()
                        || player.block_contact.in_powder_snow)
            })
            .map(|player| player.connection_id)
            .collect::<Vec<_>>();
        for connection in connections {
            self.remove_player_shoulders(connection, events);
        }
    }
}
