//! `PistonMovingBlockEntity` (`PistonMovingBlockEntity.java:32-404`).
//!
//! The moved state's identity, facing, progress, and the two booleans.

use crate::fluid::with_property;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_property_values,
    vanilla_block_state_block_type_id as block_type_of,
    vanilla_block_state_id_by_name_and_properties, vanilla_block_state_identity,
};
use bbb_protocol::block_types::{
    vanilla_block_type_for_resource_id, VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_HONEY_BLOCK_ID,
    VANILLA_BLOCK_PISTON_HEAD_ID, VANILLA_BLOCK_PISTON_ID, VANILLA_BLOCK_SLIME_BLOCK_ID,
    VANILLA_BLOCK_STICKY_PISTON_ID,
};
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::directions::opposite;
use bbb_server_types::geometry::{static_block_collision_boxes, Aabb};

use bbb_server_types::block_queries::direction_serialized_name;

use super::bell::{direction_3d_data_value, direction_from_3d_data_value};
use super::value_input::{boolean_or, codec_byte, float_or};
use super::{entry, unique_field};

const TAG_BLOCK_STATE: &str = "blockState";
const TAG_FACING: &str = "facing";
const TAG_PROGRESS: &str = "progress";
const TAG_EXTENDING: &str = "extending";
const TAG_SOURCE: &str = "source";

/// `PistonMovingBlockEntity` (`PistonMovingBlockEntity.java:32-48`).
#[derive(Debug, Clone, PartialEq)]
pub struct PistonMovingBlockEntity {
    moved_state: i32,
    facing: Direction,
    last_progress: f32,
    progress: f32,
    extending: bool,
    is_source: bool,
    last_ticked: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PistonMovingSnapshot {
    pub moved_state: i32,
    pub facing: Direction,
    pub last_progress: f32,
    pub progress: f32,
    pub extending: bool,
    pub is_source: bool,
    pub last_ticked: i64,
}

impl PistonMovingSnapshot {
    /// `PistonMovingBlockEntity.getMovementDirection` (`:224-226`).
    #[must_use]
    pub fn movement_direction(&self) -> Direction {
        if self.extending {
            self.facing
        } else {
            opposite(self.facing)
        }
    }

    /// `PistonMovingBlockEntity.getExtendedProgress` (`:104-106`).
    #[must_use]
    pub fn extended_progress(&self, progress: f32) -> f64 {
        if self.extending {
            f64::from(progress - 1.0)
        } else {
            f64::from(1.0 - progress)
        }
    }

    /// `PistonMovingBlockEntity.getCollisionRelatedBlockState`
    /// (`:108-118`): a retracting source piston answers the head it is
    /// leaving behind, whose `short` flag follows the progress.
    #[must_use]
    pub fn collision_related_block_state(&self, progress: f32) -> i32 {
        if self.extending
            || !self.is_source
            || !matches!(
                block_type_of(self.moved_state),
                Some(VANILLA_BLOCK_PISTON_ID | VANILLA_BLOCK_STICKY_PISTON_ID)
            )
        {
            return self.moved_state;
        }
        let Some(state) = vanilla_block_default_state_id(VANILLA_BLOCK_PISTON_HEAD_ID) else {
            return self.moved_state;
        };
        let sticky = block_type_of(self.moved_state) == Some(VANILLA_BLOCK_STICKY_PISTON_ID);
        let Some(state) = with_property(state, "facing", direction_serialized_name(self.facing))
        else {
            return self.moved_state;
        };
        let Some(state) = with_property(state, "type", if sticky { "sticky" } else { "normal" })
        else {
            return self.moved_state;
        };
        with_property(
            state,
            "short",
            if progress > 0.25 { "true" } else { "false" },
        )
        .unwrap_or(self.moved_state)
    }

    /// `PistonMovingBlockEntity.moveCollidedEntities`'s bounce test
    /// (`:132`): a moved slime block hands the entity its own velocity.
    #[must_use]
    pub fn causes_bounce(&self) -> bool {
        block_type_of(self.moved_state) == Some(VANILLA_BLOCK_SLIME_BLOCK_ID)
    }

    /// `PistonMovingBlockEntity.getCollisionShape` (`:363-400`) as the boxes
    /// the moving block occupies right now.
    #[must_use]
    pub fn collision_boxes(&self) -> Vec<Aabb> {
        self.collision_boxes_with(None)
    }

    /// [`Self::collision_boxes`] with the `NOCLIP` direction a piston push
    /// carries (`:44`, `:194`, `:374-377`): while a piston is moving `no_clip`
    /// and has not finished, its own moved-state shape is empty for the entity
    /// it is pushing, which leaves only the head it is leaving behind.
    #[must_use]
    pub fn collision_boxes_with(&self, no_clip: Option<Direction>) -> Vec<Aabb> {
        self.collision_boxes_with_checked(no_clip)
            .unwrap_or_default()
    }

    /// An unavailable moved-state shape remains distinct from Native empty geometry.
    #[must_use]
    pub fn collision_boxes_with_checked(&self, no_clip: Option<Direction>) -> Option<Vec<Aabb>> {
        self.collision_boxes_with_resolver(no_clip, static_block_collision_boxes)
    }

    /// PistonMovingBlockEntity.java:386-391: moved geometry reads the live level at piston pos.
    pub fn collision_boxes_with_resolver(
        &self,
        no_clip: Option<Direction>,
        mut shape: impl FnMut(i32) -> Option<Vec<Aabb>>,
    ) -> Option<Vec<Aabb>> {
        let mut boxes = Vec::new();
        // `:365-368`: a retracting source piston keeps the base it is leaving.
        if !self.extending
            && self.is_source
            && matches!(
                block_type_of(self.moved_state),
                Some(VANILLA_BLOCK_PISTON_ID | VANILLA_BLOCK_STICKY_PISTON_ID)
            )
        {
            if let Some(extended) = with_property(self.moved_state, "extended", "true") {
                boxes.extend(shape(extended)?);
            }
        }
        // `:374-377`.
        if self.progress < 1.0 && no_clip == Some(self.movement_direction()) {
            return Some(boxes);
        }
        // `:386-391`: the source piston shows a head, whose `short` flag is the
        // one this arm computes rather than [`Self::collision_related_block_state`]'s.
        let state = if self.is_source {
            let state = vanilla_block_default_state_id(VANILLA_BLOCK_PISTON_HEAD_ID)?;
            let state = with_property(state, "facing", direction_serialized_name(self.facing))?;
            let short = self.extending != (1.0 - f64::from(self.progress) < 0.25);
            with_property(state, "short", if short { "true" } else { "false" })
                .unwrap_or(self.moved_state)
        } else {
            self.moved_state
        };
        let shape = shape(state)?;
        let current = self.extended_progress(self.progress);
        let (step_x, step_y, step_z) = direction_step(self.facing);
        boxes.extend(shape.into_iter().map(|part| {
            part.moved(Vec3d {
                x: current * step_x,
                y: current * step_y,
                z: current * step_z,
            })
        }));
        Some(boxes)
    }

    /// `PistonMovingBlockEntity.isStickyForEntities` (`:218-220`).
    #[must_use]
    pub fn is_sticky_for_entities(&self) -> bool {
        block_type_of(self.moved_state) == Some(VANILLA_BLOCK_HONEY_BLOCK_ID)
    }

    /// `PistonMovingBlockEntity.moveByPositionAndProgress` (`:264-272`).
    #[must_use]
    pub fn move_by_position_and_progress(&self, pos: BlockPos, aabb: Aabb) -> Aabb {
        let current = self.extended_progress(self.progress);
        let (step_x, step_y, step_z) = direction_step(self.facing);
        aabb.moved(Vec3d {
            x: f64::from(pos.x) + current * step_x,
            y: f64::from(pos.y) + current * step_y,
            z: f64::from(pos.z) + current * step_z,
        })
    }
}

/// `Direction.getStepX/Y/Z` (`Direction.java:33-38`).
fn direction_step(direction: Direction) -> (f64, f64, f64) {
    match direction {
        Direction::Down => (0.0, -1.0, 0.0),
        Direction::Up => (0.0, 1.0, 0.0),
        Direction::North => (0.0, 0.0, -1.0),
        Direction::South => (0.0, 0.0, 1.0),
        Direction::West => (-1.0, 0.0, 0.0),
        Direction::East => (1.0, 0.0, 0.0),
    }
}

/// `Direction.AxisDirection.getStep` (`Direction.java:69-80`).
fn direction_axis_step(direction: Direction) -> f64 {
    match direction {
        Direction::Down | Direction::North | Direction::West => -1.0,
        Direction::Up | Direction::South | Direction::East => 1.0,
    }
}

/// `PistonMath.getMovementArea` (`PistonMath.java:7-26`).
#[must_use]
pub fn piston_movement_area(aabb: Aabb, direction: Direction, amount: f64) -> Aabb {
    let delta = amount * direction_axis_step(direction);
    let min = delta.min(0.0);
    let max = delta.max(0.0);
    match direction {
        Direction::West => Aabb {
            min_x: aabb.min_x + min,
            max_x: aabb.min_x + max,
            ..aabb
        },
        Direction::East => Aabb {
            min_x: aabb.max_x + min,
            max_x: aabb.max_x + max,
            ..aabb
        },
        Direction::Down => Aabb {
            min_y: aabb.min_y + min,
            max_y: aabb.min_y + max,
            ..aabb
        },
        Direction::Up => Aabb {
            min_y: aabb.max_y + min,
            max_y: aabb.max_y + max,
            ..aabb
        },
        Direction::North => Aabb {
            min_z: aabb.min_z + min,
            max_z: aabb.min_z + max,
            ..aabb
        },
        Direction::South => Aabb {
            min_z: aabb.max_z + min,
            max_z: aabb.max_z + max,
            ..aabb
        },
    }
}

/// `PistonMovingBlockEntity.getMovement` (`:236-262`): how far `aabb` has to
/// move to clear `aabb_to_be_outside_of` along `movement`.
#[must_use]
pub fn piston_movement_to_clear(
    aabb_to_be_outside_of: Aabb,
    movement: Direction,
    aabb: Aabb,
) -> f64 {
    match movement {
        Direction::East => aabb_to_be_outside_of.max_x - aabb.min_x,
        Direction::West => aabb.max_x - aabb_to_be_outside_of.min_x,
        Direction::Up => aabb_to_be_outside_of.max_y - aabb.min_y,
        Direction::Down => aabb.max_y - aabb_to_be_outside_of.min_y,
        Direction::South => aabb_to_be_outside_of.max_z - aabb.min_z,
        Direction::North => aabb.max_z - aabb_to_be_outside_of.min_z,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PistonTickResult {
    Advanced,
    Finalize {
        state: i32,
        push_direction: Direction,
    },
}

impl Default for PistonMovingBlockEntity {
    /// `new PistonMovingBlockEntity(pos, state)` (`:50-52`) plus the
    /// `loadAdditional` defaults (`:345-350`).
    fn default() -> Self {
        Self {
            moved_state: air_state(),
            facing: Direction::Down,
            last_progress: 0.0,
            progress: 0.0,
            extending: false,
            is_source: false,
            last_ticked: i64::MIN,
        }
    }
}

impl PistonMovingBlockEntity {
    #[must_use]
    pub const fn moved_state(&self) -> i32 {
        self.moved_state
    }

    #[must_use]
    pub const fn facing(&self) -> Direction {
        self.facing
    }

    #[must_use]
    pub const fn progress(&self) -> f32 {
        self.progress
    }

    #[must_use]
    pub const fn last_progress(&self) -> f32 {
        self.last_progress
    }

    #[must_use]
    pub const fn extending(&self) -> bool {
        self.extending
    }

    #[must_use]
    pub const fn is_source(&self) -> bool {
        self.is_source
    }

    #[must_use]
    pub const fn last_ticked(&self) -> i64 {
        self.last_ticked
    }

    #[must_use]
    pub const fn snapshot(&self) -> PistonMovingSnapshot {
        PistonMovingSnapshot {
            moved_state: self.moved_state,
            facing: self.facing,
            last_progress: self.last_progress,
            progress: self.progress,
            extending: self.extending,
            is_source: self.is_source,
            last_ticked: self.last_ticked,
        }
    }

    /// `PistonMovingBlockEntity` full constructor (`PistonMovingBlockEntity.java:54-65`).
    pub fn configure(
        &mut self,
        moved_state: i32,
        facing: Direction,
        extending: bool,
        is_source: bool,
    ) {
        self.moved_state = moved_state;
        self.facing = facing;
        self.last_progress = 0.0;
        self.progress = 0.0;
        self.extending = extending;
        self.is_source = is_source;
        self.last_ticked = i64::MIN;
    }

    /// Server half of `PistonMovingBlockEntity.tick` (`:307-339`). Entity
    /// movement is preflighted by the world adapter before this is called.
    pub fn tick_without_entities(&mut self, game_time: i64) -> PistonTickResult {
        self.last_ticked = game_time;
        self.last_progress = self.progress;
        if self.last_progress >= 1.0 {
            return PistonTickResult::Finalize {
                state: self.moved_state,
                push_direction: if self.extending {
                    self.facing
                } else {
                    opposite(self.facing)
                },
            };
        }
        self.progress = (self.progress + 0.5).min(1.0);
        PistonTickResult::Advanced
    }

    pub fn final_tick_state(&mut self) -> (i32, Direction) {
        self.progress = 1.0;
        self.last_progress = 1.0;
        (
            if self.is_source {
                air_state()
            } else {
                self.moved_state
            },
            if self.extending {
                self.facing
            } else {
                opposite(self.facing)
            },
        )
    }

    /// `PistonMovingBlockEntity.saveAdditional` (`:354-361`).
    pub(super) fn save_additional(
        &self,
        save: &mut super::item_projection::BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        let content_registry = save.resolver().content_registry();
        Ok(vec![
            entry(
                TAG_BLOCK_STATE,
                save_block_state(self.moved_state, content_registry),
            ),
            entry(
                TAG_FACING,
                NbtSummaryValue::Byte(direction_3d_data_value(self.facing) as i8),
            ),
            entry(
                TAG_PROGRESS,
                NbtSummaryValue::Float(self.last_progress.to_bits()),
            ),
            entry(TAG_EXTENDING, byte_bool(self.extending)),
            entry(TAG_SOURCE, byte_bool(self.is_source)),
        ])
    }

    /// `PistonMovingBlockEntity.loadAdditional` (`:343-351`).
    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut super::item_projection::BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        let content_registry = load.resolver().content_registry();
        self.moved_state = unique_field(record, TAG_BLOCK_STATE)
            .and_then(|value| load_block_state(value, content_registry))
            .unwrap_or_else(air_state);
        self.facing = unique_field(record, TAG_FACING)
            .and_then(codec_byte)
            .map(|id| {
                direction_from_3d_data_value(i32::from(id).wrapping_rem(6).unsigned_abs() as i32)
            })
            .unwrap_or(Direction::Down);
        self.progress = float_or(record, TAG_PROGRESS, 0.0);
        self.last_progress = self.progress;
        self.extending = boolean_or(record, TAG_EXTENDING, false);
        self.is_source = boolean_or(record, TAG_SOURCE, false);
        self.last_ticked = i64::MIN;
        Ok(())
    }
}

impl super::BlockEntity {
    #[must_use]
    pub fn piston_snapshot(&self) -> Option<PistonMovingSnapshot> {
        match &self.payload {
            super::BlockEntityPayload::Piston(piston) => Some(piston.snapshot()),
            _ => None,
        }
    }

    pub fn configure_piston(
        &mut self,
        moved_state: i32,
        facing: Direction,
        extending: bool,
        is_source: bool,
    ) -> bool {
        let super::BlockEntityPayload::Piston(piston) = &mut self.payload else {
            return false;
        };
        piston.configure(moved_state, facing, extending, is_source);
        true
    }

    pub fn tick_piston_without_entities(&mut self, game_time: i64) -> Option<PistonTickResult> {
        let super::BlockEntityPayload::Piston(piston) = &mut self.payload else {
            return None;
        };
        Some(piston.tick_without_entities(game_time))
    }

    pub fn final_tick_piston(&mut self) -> Option<(i32, Direction)> {
        let super::BlockEntityPayload::Piston(piston) = &mut self.payload else {
            return None;
        };
        Some(piston.final_tick_state())
    }
}

fn air_state() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap_or(0)
}

fn byte_bool(value: bool) -> NbtSummaryValue {
    NbtSummaryValue::Byte(i8::from(value))
}

/// `BlockState.CODEC` writer (`StateHolder.java:148-161`): `Name`, then the
/// full `Properties` map when the block is not a singleton.
fn save_block_state(
    state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> NbtSummaryValue {
    if let Some(identity) = vanilla_block_state_identity(state_id) {
        let mut fields = vec![entry(
            "Name",
            NbtSummaryValue::string(identity.name.clone()),
        )];
        if !identity.properties.is_empty() {
            fields.push(entry(
                "Properties",
                NbtSummaryValue::Compound(
                    identity
                        .properties
                        .iter()
                        .map(|(name, value)| entry(name, NbtSummaryValue::string(value.clone())))
                        .collect(),
                ),
            ));
        }
        return NbtSummaryValue::Compound(fields);
    }
    if let Some((custom, offset)) = content_registry.block_by_state_id(state_id) {
        let mut fields = vec![entry(
            "Name",
            NbtSummaryValue::string(custom.resource_id().to_owned()),
        )];
        if custom.state_count > 1 || offset > 0 {
            fields.push(entry(
                "Properties",
                NbtSummaryValue::Compound(vec![entry(
                    bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY,
                    NbtSummaryValue::string(offset.to_string()),
                )]),
            ));
        }
        return NbtSummaryValue::Compound(fields);
    }
    let identity =
        vanilla_block_state_identity(air_state()).expect("minecraft:air is in the catalog");
    NbtSummaryValue::Compound(vec![entry(
        "Name",
        NbtSummaryValue::string(identity.name.clone()),
    )])
}

/// `input.read("blockState", BlockState.CODEC).orElse(AIR)` (`:345`).
fn load_block_state(
    value: &NbtSummaryValue,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<i32> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let NbtSummaryValue::String(raw) = unique_field(fields, "Name")? else {
        return None;
    };
    let name = raw
        .try_to_string()
        .ok()
        .and_then(|raw| normalize_resource_location(raw).ok())?;
    if let Some(block) = vanilla_block_type_for_resource_id(&name) {
        let default_id = vanilla_block_default_state_id(block.id)?;
        let mut properties = vanilla_block_state_identity(default_id)?.properties.clone();
        if let Some(NbtSummaryValue::Compound(declared)) = unique_field(fields, "Properties") {
            for property in declared {
                let NbtSummaryValue::String(value) = &property.value else {
                    continue;
                };
                // A block property name keys with `String`; the state table owns the bound.
                let Ok(property_name) = property.name.try_to_string() else {
                    continue;
                };
                if properties.contains_key(&property_name)
                    && vanilla_block_property_values(&name, &property_name)
                        .is_some_and(|values| values.iter().any(|known| value == *known))
                {
                    properties.insert(
                        property_name,
                        value
                            .try_to_string()
                            .expect("matched a static block property"),
                    );
                }
            }
        }
        return vanilla_block_state_id_by_name_and_properties(&name, &properties);
    }
    let custom = content_registry.block_by_resource_id(&name)?;
    let offset = unique_field(fields, "Properties")
        .and_then(|props| match props {
            NbtSummaryValue::Compound(declared) => unique_field(
                declared,
                bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY,
            ),
            _ => None,
        })
        .and_then(|value| match value {
            NbtSummaryValue::String(raw) => raw.try_to_string().ok()?.parse::<i32>().ok(),
            _ => None,
        })
        .filter(|offset| *offset >= 0 && *offset < i32::from(custom.state_count))
        .unwrap_or(0);
    Some(custom.first_state_id + offset)
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::{VANILLA_BLOCK_MOVING_PISTON_ID, VANILLA_BLOCK_OAK_LOG_ID};
    use bbb_protocol::packets::{BlockPos, Direction};
    use bbb_protocol::NbtSummaryEntry;

    use crate::block_entity::{new_block_entity, BlockEntityPayload};

    use super::*;

    fn pos() -> BlockPos {
        BlockPos { x: 1, y: 2, z: 3 }
    }

    fn default_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_MOVING_PISTON_ID)
            .expect("minecraft:moving_piston has a default state")
    }

    fn oak_log() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_OAK_LOG_ID)
            .expect("minecraft:oak_log has a default state")
    }

    fn upsert(record: &mut Vec<NbtSummaryEntry>, extra: Vec<NbtSummaryEntry>) {
        for field in extra {
            if let Some(existing) = record.iter_mut().find(|entry| entry.name == field.name) {
                existing.value = field.value;
            } else {
                record.push(field);
            }
        }
    }

    #[test]
    fn piston_save_load_round_trips_moved_state_and_flags() {
        let mut piston = PistonMovingBlockEntity {
            moved_state: oak_log(),
            facing: Direction::East,
            last_progress: 0.5,
            progress: 0.5,
            extending: true,
            is_source: true,
            last_ticked: i64::MIN,
        };
        let mut record = new_block_entity(pos(), default_state())
            .expect("a moving piston builds an object")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let mut save = super::super::item_projection::BlockEntityItemSave::new(
            super::super::item_projection::BlockEntityItemProjection::with_resolver(
                crate::block_entity::test_item_resolver(),
            ),
        );
        let mut load = super::super::item_projection::BlockEntityItemLoad::new(
            super::super::item_projection::BlockEntityItemProjection::with_resolver(
                crate::block_entity::test_item_resolver(),
            ),
        );
        upsert(
            &mut record,
            piston.save_additional(&mut save).expect("piston saves"),
        );
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture codec succeeds")
            .expect("the record names a piston");
        let BlockEntityPayload::Piston(loaded_piston) = loaded.payload() else {
            panic!("payload is a piston");
        };
        assert_eq!(loaded_piston.moved_state(), oak_log());
        assert_eq!(loaded_piston.facing(), Direction::East);
        assert_eq!(loaded_piston.progress(), 0.5);
        assert!(loaded_piston.extending());
        assert!(loaded_piston.is_source());
        let again = crate::block_entity::load_test_fixture(
            pos(),
            default_state(),
            &loaded
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
                .expect("fixture codec succeeds"),
        )
        .expect("fixture codec succeeds")
        .expect("the round-trip still names a piston");
        let BlockEntityPayload::Piston(again) = again.payload() else {
            panic!("payload is a piston");
        };
        assert_eq!(again.moved_state(), oak_log());
        assert_eq!(again.facing(), Direction::East);
        assert_eq!(again.progress(), 0.5);
        assert!(again.extending());
        assert!(again.is_source());
        let saved_again = piston.save_additional(&mut save).expect("piston saves");
        piston
            .load_additional(&saved_again, &mut load)
            .expect("piston loads");
        assert_eq!(piston.moved_state(), oak_log());
    }

    #[test]
    fn moving_piston_advances_twice_then_finalizes_on_the_third_tick() {
        let mut piston = PistonMovingBlockEntity::default();
        piston.configure(oak_log(), Direction::East, true, false);

        assert_eq!(piston.tick_without_entities(10), PistonTickResult::Advanced);
        assert_eq!(piston.progress(), 0.5);
        assert_eq!(piston.last_progress(), 0.0);
        assert_eq!(piston.tick_without_entities(11), PistonTickResult::Advanced);
        assert_eq!(piston.progress(), 1.0);
        assert_eq!(
            piston.tick_without_entities(12),
            PistonTickResult::Finalize {
                state: oak_log(),
                push_direction: Direction::East,
            }
        );
        assert_eq!(piston.last_ticked(), 12);
    }

    /// `PistonMath.getMovementArea` (`PistonMath.java:7-26`) over one block's
    /// own box.
    #[test]
    fn the_movement_area_is_the_swept_face_band() {
        let unit = Aabb {
            min_x: 0.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 1.0,
            max_y: 1.0,
            max_z: 1.0,
        };
        assert_eq!(
            piston_movement_area(unit, Direction::East, 0.5),
            Aabb {
                min_x: 1.0,
                max_x: 1.5,
                ..unit
            }
        );
        assert_eq!(
            piston_movement_area(unit, Direction::Up, -0.5),
            Aabb {
                min_y: 0.5,
                max_y: 1.0,
                ..unit
            }
        );
        assert_eq!(
            piston_movement_area(unit, Direction::North, 0.25),
            Aabb {
                min_z: -0.25,
                max_z: 0.0,
                ..unit
            }
        );
    }

    /// `PistonMovingBlockEntity.getMovement` (`:236-262`).
    #[test]
    fn the_clear_distance_is_the_overlap_along_the_movement_axis() {
        let band = Aabb {
            min_x: 1.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 1.5,
            max_y: 1.0,
            max_z: 1.0,
        };
        let entity = Aabb {
            min_x: 1.2,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 1.8,
            max_y: 1.0,
            max_z: 1.0,
        };
        assert!((piston_movement_to_clear(band, Direction::East, entity) - 0.3).abs() < 1.0e-9);
        assert_eq!(piston_movement_to_clear(band, Direction::West, entity), 0.8);
    }

    /// `PistonMovingBlockEntity.getCollisionRelatedBlockState` (`:108-118`).
    #[test]
    fn a_retracting_source_piston_answers_its_head_state() {
        let mut piston = PistonMovingBlockEntity::default();
        piston.configure(
            vanilla_block_state_id_by_name_and_properties(
                "minecraft:sticky_piston",
                &std::collections::BTreeMap::from([
                    ("facing".to_string(), "east".to_string()),
                    ("extended".to_string(), "true".to_string()),
                ]),
            )
            .expect("a sticky piston facing east"),
            Direction::East,
            false,
            true,
        );
        let snapshot = piston.snapshot();
        assert!(!snapshot.is_sticky_for_entities());
        let head = snapshot.collision_related_block_state(0.3);
        let identity = vanilla_block_state_identity(head).expect("a piston head state");
        assert_eq!(identity.name, "minecraft:piston_head");
        assert_eq!(
            identity.properties.get("facing").map(String::as_str),
            Some("east")
        );
        assert_eq!(
            identity.properties.get("type").map(String::as_str),
            Some("sticky")
        );
        assert_eq!(
            identity.properties.get("short").map(String::as_str),
            Some("true")
        );
        let short = snapshot.collision_related_block_state(0.2);
        let identity = vanilla_block_state_identity(short).expect("a piston head state");
        assert_eq!(
            identity.properties.get("short").map(String::as_str),
            Some("false")
        );
    }

    /// `PistonMovingBlockEntity.isStickyForEntities` (`:218-220`).
    #[test]
    fn only_a_moving_honey_block_carries_entities() {
        let mut piston = PistonMovingBlockEntity::default();
        piston.configure(
            vanilla_block_default_state_id(bbb_protocol::block_types::VANILLA_BLOCK_HONEY_BLOCK_ID)
                .expect("honey has a default state"),
            Direction::Up,
            true,
            false,
        );
        assert!(piston.snapshot().is_sticky_for_entities());
        piston.configure(oak_log(), Direction::Up, true, false);
        assert!(!piston.snapshot().is_sticky_for_entities());
    }

    /// `PistonMovingBlockEntity.moveCollidedEntities`'s bounce test (`:132`).
    #[test]
    fn only_a_moved_slime_block_bounces_entities() {
        let mut piston = PistonMovingBlockEntity::default();
        piston.configure(
            vanilla_block_default_state_id(VANILLA_BLOCK_SLIME_BLOCK_ID)
                .expect("slime has a default state"),
            Direction::East,
            true,
            false,
        );
        assert!(piston.snapshot().causes_bounce());
        piston.configure(oak_log(), Direction::East, true, false);
        assert!(!piston.snapshot().causes_bounce());
    }

    /// `PistonMovingBlockEntity.getCollisionShape` (`:363-400`).
    #[test]
    fn the_collision_shape_follows_the_progress() {
        let mut piston = PistonMovingBlockEntity::default();
        piston.configure(oak_log(), Direction::East, true, false);
        // `getExtendedProgress` for an extending move is `progress - 1`, so a
        // move that has not started yet still occupies its source cell.
        assert_eq!(
            piston.snapshot().collision_boxes(),
            vec![Aabb {
                min_x: -1.0,
                min_y: 0.0,
                min_z: 0.0,
                max_x: 0.0,
                max_y: 1.0,
                max_z: 1.0,
            }]
        );
        piston.progress = 0.5;
        let boxes = piston.snapshot().collision_boxes();
        assert_eq!(boxes.len(), 1);
        // `getExtendedProgress` for an extending move is `progress - 1`.
        assert!((boxes[0].min_x + 0.5).abs() < 1.0e-9);
        assert!((boxes[0].max_x - 0.5).abs() < 1.0e-9);
        assert!((boxes[0].max_y - 1.0).abs() < 1.0e-9);
    }

    /// `PistonMovingBlockEntity.getCollisionShape`'s `NOCLIP` arm
    /// (`:374-377`): the piston doing the pushing contributes no moved-state
    /// shape to the entity it is moving, but a piston moving another way still
    /// does, and a finished move does too.
    #[test]
    fn a_push_ignores_the_shape_of_the_piston_doing_the_pushing() {
        let mut piston = PistonMovingBlockEntity::default();
        piston.configure(oak_log(), Direction::East, true, false);
        piston.progress = 0.5;
        let full = piston.snapshot().collision_boxes();
        assert_eq!(full.len(), 1);
        assert_eq!(piston.snapshot().collision_boxes_with(None), full);
        assert_eq!(
            piston
                .snapshot()
                .collision_boxes_with(Some(Direction::West)),
            full
        );
        assert!(piston
            .snapshot()
            .collision_boxes_with(Some(Direction::East))
            .is_empty());
        // `:374` also wants `progress < 1.0`: the arrived block is solid again.
        piston.progress = 1.0;
        assert_eq!(
            piston
                .snapshot()
                .collision_boxes_with(Some(Direction::East)),
            piston.snapshot().collision_boxes()
        );

        // A retracting source piston keeps the base it is leaving (`:365-368`),
        // which is what `pistonHeadShape` is in the `NOCLIP` arm.
        let mut source = PistonMovingBlockEntity::default();
        source.configure(
            piston_state_id("minecraft:piston", "east", "true"),
            Direction::East,
            false,
            true,
        );
        source.progress = 0.5;
        let base = source
            .snapshot()
            .collision_boxes_with(Some(Direction::West));
        assert_eq!(base.len(), 1);
        assert!(source.snapshot().collision_boxes().len() > base.len());
    }

    fn piston_state_id(name: &str, facing: &str, extended: &str) -> i32 {
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            name,
            &std::collections::BTreeMap::from([
                ("facing".to_string(), facing.to_string()),
                ("extended".to_string(), extended.to_string()),
            ]),
        )
        .expect("a built-in piston state")
    }
}
