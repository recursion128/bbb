//! Sculk catalyst listener state and bloom.

use crate::fluid::with_property;
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_tick::vibration::WorldVibrationReads;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use bbb_server_types::sculk_cursor::{cursor_nbt, SculkCursorFacts, MAX_CHARGE, MAX_CURSORS};

use super::game_event::{containing_block_pos, GameEventContext};

use super::{entry, unique_field};

/// `SculkSpreader.load` / `save` (`SculkSpreader.java:108-114`).
const TAG_CURSORS: &str = "cursors";
/// `SculkCatalystBlock.PULSE = BlockStateProperties.BLOOM`
/// (`SculkCatalystBlock.java:24`).
const BLOOM_PROPERTY: &str = "bloom";
/// `CatalystListener.PULSE_TICKS` (`:58`), the delay `bloom` schedules
/// (`:112`).
pub const PULSE_TICKS: i32 = 8;
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SculkCatalystBlockEntity {
    cursors: Vec<NbtSummaryValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivingExperienceFacts {
    pub experience_consumed: bool,
    pub drops_experience: bool,
    pub last_damage_source_entity: Option<i32>,
    pub last_hurt_by_mob_player: Option<Uuid>,
    pub last_damage_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalystDamageSource {
    /// `DamageSource.getType()`'s registry id.
    pub damage_type: String,
    /// `DamageSource.getEntity()`, `None` for a source that carries no causing
    /// entity.
    pub causing_entity_id: Option<i32>,
}

impl SculkCatalystBlockEntity {
    #[must_use]
    pub fn cursors(&self) -> &[NbtSummaryValue] {
        &self.cursors
    }

    pub fn set_cursors(&mut self, cursors: Vec<NbtSummaryValue>) {
        self.cursors = cursors;
    }

    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        vec![entry(
            TAG_CURSORS,
            NbtSummaryValue::List(self.cursors.clone()),
        )]
    }

    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.cursors = match unique_field(record, TAG_CURSORS) {
            Some(NbtSummaryValue::List(values)) => values.clone(),
            _ => Vec::new(),
        };
    }

    fn add_cursors(&mut self, start_pos: BlockPos, mut charge: i32) {
        while charge > 0 {
            let current_charge = charge.min(MAX_CHARGE);
            if self.cursors.len() < MAX_CURSORS {
                self.cursors.push(cursor_value(start_pos, current_charge));
            }
            charge -= current_charge;
        }
    }

    pub(super) fn handle_game_event(
        &mut self,
        pos: BlockPos,
        level: &mut WorldVibrationReads<'_>,
        event: VanillaGameEvent,
        context: GameEventContext,
        source_position: Vec3d,
    ) -> bool {
        if event != VanillaGameEvent::ENTITY_DIE {
            return false;
        }
        let Some(entity_id) = context.source_entity_id else {
            return false;
        };
        let Some(mob) = level.living_experience_facts(entity_id) else {
            return false;
        };
        self.handle_mob_death(entity_id, mob, pos, level, source_position);
        true
    }

    fn handle_mob_death(
        &mut self,
        entity_id: i32,
        mob: LivingExperienceFacts,
        pos: BlockPos,
        catalyst: &mut WorldVibrationReads<'_>,
        source_position: Vec3d,
    ) {
        if mob.experience_consumed {
            return;
        }
        let experience_would_drop = catalyst
            .experience_reward(entity_id, mob.last_damage_source_entity)
            .unwrap_or(0);
        if mob.drops_experience && experience_would_drop > 0 {
            // `BlockPos.containing(sourcePosition.relative(UP, 0.5))`
            self.add_cursors(
                containing_block_pos(Vec3d {
                    x: source_position.x,
                    y: source_position.y + 0.5,
                    z: source_position.z,
                }),
                experience_would_drop,
            );
            self.try_award_it_spreads_advancement(entity_id, &mob, catalyst);
        }
        catalyst.skip_drop_experience(entity_id);
        self.bloom(pos, catalyst);
    }

    fn try_award_it_spreads_advancement(
        &mut self,
        entity_id: i32,
        facts: &LivingExperienceFacts,
        level: &mut WorldVibrationReads<'_>,
    ) {
        let Some(player) = facts.last_hurt_by_mob_player else {
            return;
        };
        let damage_source = match &facts.last_damage_type {
            Some(damage_type) => CatalystDamageSource {
                damage_type: damage_type.clone(),
                causing_entity_id: facts.last_damage_source_entity,
            },
            None => level.player_attack_source(player),
        };
        level.trigger_kill_mob_near_sculk_catalyst(player, entity_id, damage_source);
    }

    fn bloom(&self, pos: BlockPos, level: &mut WorldVibrationReads<'_>) {
        let Some(state) = level.block_state(pos) else {
            return;
        };
        if let Some(bloom) = with_property(state, BLOOM_PROPERTY, "true") {
            level.set_block_state(pos, bloom, UPDATE_ALL);
        }
        level.schedule_block_tick(pos, PULSE_TICKS);
        level.send_sculk_soul_particles(
            f64::from(pos.x) + 0.5,
            f64::from(pos.y) + 1.15,
            f64::from(pos.z) + 0.5,
        );
        let pitch = 0.6 + level.next_random_float() * 0.4;
        level.play_bloom_sound(pos, pitch);
    }
}

/// `Block.UPDATE_ALL` (`Block.java:89`), the flag `bloom` passes (`:111`).
const UPDATE_ALL: i32 = 3;

fn cursor_value(pos: BlockPos, charge: i32) -> NbtSummaryValue {
    cursor_nbt(&SculkCursorFacts {
        pos,
        charge,
        decay_delay: 1,
        update_delay: 0,
        facings: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{entry, SculkCatalystBlockEntity, MAX_CHARGE, MAX_CURSORS, TAG_CURSORS};
    use crate::block_entity::{new_block_entity, unique_field, BlockEntityPayload};
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_SCULK_CATALYST_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    fn pos() -> BlockPos {
        BlockPos { x: 6, y: 7, z: 8 }
    }

    fn sculk_catalyst() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_SCULK_CATALYST_ID)
            .expect("minecraft:sculk_catalyst has a default state")
    }

    fn sample_cursors() -> NbtSummaryValue {
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            entry("charge", NbtSummaryValue::Int(9)),
            entry("pos", NbtSummaryValue::IntArray(vec![1, 2, 3])),
        ])])
    }

    fn upsert(record: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
        if let Some(existing) = record.iter_mut().find(|entry| entry.name == name) {
            existing.value = value;
        } else {
            record.push(entry(name, value));
        }
    }

    fn cursor(start_pos: BlockPos, charge: i32) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry(
                "pos",
                NbtSummaryValue::IntArray(vec![start_pos.x, start_pos.y, start_pos.z]),
            ),
            entry("charge", NbtSummaryValue::Int(charge)),
            entry("decay_delay", NbtSummaryValue::Int(1)),
            entry("update_delay", NbtSummaryValue::Int(0)),
        ])
    }

    #[test]
    fn sculk_catalyst_save_load_round_trips_cursors() {
        let state = sculk_catalyst();
        let mut record = new_block_entity(pos(), state)
            .expect("a sculk catalyst builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        upsert(&mut record, TAG_CURSORS, sample_cursors());
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("catalyst loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(unique_field(&saved, TAG_CURSORS), Some(&sample_cursors()));
        let catalyst = match loaded.payload() {
            BlockEntityPayload::SculkCatalyst(payload) => payload,
            other => panic!("expected a sculk catalyst payload, got {other:?}"),
        };
        assert_eq!(catalyst.cursors().len(), 1);
        assert!(loaded
            .update_tag_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds")
            .is_empty());
        assert!(
            crate::block_entity::block_entity_update_packet_with_resolver(
                &loaded,
                crate::block_entity::test_item_resolver()
            )
            .expect("fixture codec succeeds")
            .is_none()
        );
    }

    #[test]
    fn a_charge_splits_at_a_thousand_and_the_cursor_list_stops_at_thirty_two() {
        let start = BlockPos { x: 1, y: 3, z: 3 };
        let mut catalyst = SculkCatalystBlockEntity::default();
        catalyst.add_cursors(start, 2500);
        assert_eq!(
            catalyst.cursors(),
            &[
                cursor(start, MAX_CHARGE),
                cursor(start, MAX_CHARGE),
                cursor(start, 500)
            ]
        );

        let mut full = SculkCatalystBlockEntity::default();
        full.add_cursors(start, MAX_CHARGE * (MAX_CURSORS as i32 + 1));
        assert_eq!(full.cursors().len(), MAX_CURSORS);
        assert!(full
            .cursors()
            .iter()
            .all(|written| *written == cursor(start, MAX_CHARGE)));
    }
}
