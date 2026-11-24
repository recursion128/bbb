//! `StructureBlockEntity` (`StructureBlockEntity.java:39-553`).

use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::{
    BlockPos, SetStructureBlock, StructureBlockMirror, StructureBlockMode, StructureBlockRotation,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::value_input::{boolean_or, float_or, int_or, long_or};
use super::{entry, unique_field};

const TAG_NAME: &str = "name";
const TAG_AUTHOR: &str = "author";
const TAG_METADATA: &str = "metadata";
const TAG_POS_X: &str = "posX";
const TAG_POS_Y: &str = "posY";
const TAG_POS_Z: &str = "posZ";
const TAG_SIZE_X: &str = "sizeX";
const TAG_SIZE_Y: &str = "sizeY";
const TAG_SIZE_Z: &str = "sizeZ";
const TAG_ROTATION: &str = "rotation";
const TAG_MIRROR: &str = "mirror";
const TAG_MODE: &str = "mode";
const TAG_IGNORE_ENTITIES: &str = "ignoreEntities";
const TAG_STRICT: &str = "strict";
const TAG_POWERED: &str = "powered";
const TAG_SHOW_AIR: &str = "showair";
const TAG_SHOW_BOUNDING_BOX: &str = "showboundingbox";
const TAG_INTEGRITY: &str = "integrity";
const TAG_SEED: &str = "seed";

/// `StructureBlockEntity.MAX_OFFSET_PER_AXIS` (`StructureBlockEntity.java:41`).
const MAX_OFFSET_PER_AXIS: i32 = 48;
/// `StructureBlockEntity.MAX_SIZE_PER_AXIS` (`:42`).
const MAX_SIZE_PER_AXIS: i32 = 48;
/// `StructureBlockEntity.DEFAULT_POS` (`:46`).
const DEFAULT_POS: (i32, i32, i32) = (0, 1, 0);
/// `StructureBlockEntity.DEFAULT_INTEGRITY` (`:55`).
const DEFAULT_INTEGRITY_BITS: u32 = 1.0_f32.to_bits();

trait LegacyEnum: Sized {
    fn legacy_name(self) -> &'static str;
    fn from_legacy(name: &str) -> Self;
}

/// `Rotation.LEGACY_CODEC` (`Rotation.java:27`).
impl LegacyEnum for StructureBlockRotation {
    fn legacy_name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Clockwise90 => "CLOCKWISE_90",
            Self::Clockwise180 => "CLOCKWISE_180",
            Self::Counterclockwise90 => "COUNTERCLOCKWISE_90",
        }
    }

    fn from_legacy(name: &str) -> Self {
        match name {
            "CLOCKWISE_90" => Self::Clockwise90,
            "CLOCKWISE_180" => Self::Clockwise180,
            "COUNTERCLOCKWISE_90" => Self::Counterclockwise90,
            _ => Self::None,
        }
    }
}

/// `Mirror.LEGACY_CODEC` (`Mirror.java:17`).
impl LegacyEnum for StructureBlockMirror {
    fn legacy_name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::LeftRight => "LEFT_RIGHT",
            Self::FrontBack => "FRONT_BACK",
        }
    }

    fn from_legacy(name: &str) -> Self {
        match name {
            "LEFT_RIGHT" => Self::LeftRight,
            "FRONT_BACK" => Self::FrontBack,
            _ => Self::None,
        }
    }
}

/// `StructureMode.LEGACY_CODEC` (`StructureMode.java:15`).
impl LegacyEnum for StructureBlockMode {
    fn legacy_name(self) -> &'static str {
        match self {
            Self::Save => "SAVE",
            Self::Load => "LOAD",
            Self::Corner => "CORNER",
            Self::Data => "DATA",
        }
    }

    fn from_legacy(name: &str) -> Self {
        match name {
            "SAVE" => Self::Save,
            "LOAD" => Self::Load,
            "CORNER" => Self::Corner,
            _ => Self::Data,
        }
    }
}

/// `StructureBlockEntity` (`StructureBlockEntity.java:57-71`).
#[derive(Debug, Clone, PartialEq)]
pub struct StructureBlockEntity {
    name: String,
    author: bbb_protocol::JavaString,
    metadata: bbb_protocol::JavaString,
    pos: (i32, i32, i32),
    size: (i32, i32, i32),
    rotation: StructureBlockRotation,
    mirror: StructureBlockMirror,
    mode: StructureBlockMode,
    ignore_entities: bool,
    strict: bool,
    powered: bool,
    show_air: bool,
    show_bounding_box: bool,
    integrity_bits: u32,
    seed: i64,
}

impl Default for StructureBlockEntity {
    /// `new StructureBlockEntity` (`StructureBlockEntity.java:73-76`) on the
    /// default block state (`StructureBlock.java:34`, `MODE=LOAD`).
    fn default() -> Self {
        Self {
            name: String::new(),
            author: Default::default(),
            metadata: Default::default(),
            pos: DEFAULT_POS,
            size: (0, 0, 0),
            rotation: StructureBlockRotation::None,
            mirror: StructureBlockMirror::None,
            mode: StructureBlockMode::Load,
            ignore_entities: true,
            strict: false,
            powered: false,
            show_air: false,
            show_bounding_box: true,
            integrity_bits: DEFAULT_INTEGRITY_BITS,
            seed: 0,
        }
    }
}

impl StructureBlockEntity {
    /// `ServerGamePacketListenerImpl.handleSetStructureBlock`
    /// (`ServerGamePacketListenerImpl.java:825-837`).
    pub fn apply_edit(&mut self, packet: &SetStructureBlock) {
        self.mode = packet.mode;
        self.name = normalize_resource_location(packet.name.clone()).unwrap_or_default();
        self.set_offset(packet.offset);
        self.set_size(BlockPos {
            x: packet.size.x,
            y: packet.size.y,
            z: packet.size.z,
        });
        self.mirror = packet.mirror;
        self.rotation = packet.rotation;
        self.metadata = packet.data.clone();
        self.ignore_entities = packet.ignore_entities;
        self.strict = packet.strict;
        self.show_air = packet.show_air;
        self.show_bounding_box = packet.show_bounding_box;
        self.integrity_bits = packet.integrity.to_bits();
        self.seed = packet.seed;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn author(&self) -> &bbb_protocol::JavaString {
        &self.author
    }

    pub fn set_author(&mut self, author: impl Into<bbb_protocol::JavaString>) {
        self.author = author.into();
    }

    pub const fn offset(&self) -> BlockPos {
        BlockPos {
            x: self.pos.0,
            y: self.pos.1,
            z: self.pos.2,
        }
    }

    pub fn set_offset(&mut self, offset: BlockPos) {
        self.pos = (offset.x, offset.y, offset.z);
    }

    pub const fn size(&self) -> BlockPos {
        BlockPos {
            x: self.size.0,
            y: self.size.1,
            z: self.size.2,
        }
    }

    pub fn set_size(&mut self, size: BlockPos) {
        self.size = (size.x, size.y, size.z);
    }

    pub const fn mode(&self) -> StructureBlockMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: StructureBlockMode) {
        self.mode = mode;
    }

    pub const fn mirror(&self) -> StructureBlockMirror {
        self.mirror
    }

    pub const fn rotation(&self) -> StructureBlockRotation {
        self.rotation
    }

    pub const fn ignore_entities(&self) -> bool {
        self.ignore_entities
    }

    pub const fn strict(&self) -> bool {
        self.strict
    }

    pub const fn powered(&self) -> bool {
        self.powered
    }

    pub fn set_powered(&mut self, powered: bool) {
        self.powered = powered;
    }

    pub const fn integrity(&self) -> f32 {
        f32::from_bits(self.integrity_bits)
    }

    pub const fn seed(&self) -> i64 {
        self.seed
    }

    /// `StructureBlockEntity.saveAdditional` (`StructureBlockEntity.java:79-100`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        vec![
            entry(TAG_NAME, NbtSummaryValue::string(self.name.clone())),
            entry(TAG_AUTHOR, NbtSummaryValue::string(self.author.clone())),
            entry(TAG_METADATA, NbtSummaryValue::string(self.metadata.clone())),
            entry(TAG_POS_X, NbtSummaryValue::Int(self.pos.0)),
            entry(TAG_POS_Y, NbtSummaryValue::Int(self.pos.1)),
            entry(TAG_POS_Z, NbtSummaryValue::Int(self.pos.2)),
            entry(TAG_SIZE_X, NbtSummaryValue::Int(self.size.0)),
            entry(TAG_SIZE_Y, NbtSummaryValue::Int(self.size.1)),
            entry(TAG_SIZE_Z, NbtSummaryValue::Int(self.size.2)),
            entry(
                TAG_ROTATION,
                NbtSummaryValue::string(self.rotation.legacy_name()),
            ),
            entry(
                TAG_MIRROR,
                NbtSummaryValue::string(self.mirror.legacy_name()),
            ),
            entry(TAG_MODE, NbtSummaryValue::string(self.mode.legacy_name())),
            bool_entry(TAG_IGNORE_ENTITIES, self.ignore_entities),
            bool_entry(TAG_STRICT, self.strict),
            bool_entry(TAG_POWERED, self.powered),
            bool_entry(TAG_SHOW_AIR, self.show_air),
            bool_entry(TAG_SHOW_BOUNDING_BOX, self.show_bounding_box),
            entry(TAG_INTEGRITY, NbtSummaryValue::Float(self.integrity_bits)),
            entry(TAG_SEED, NbtSummaryValue::Long(self.seed)),
        ]
    }

    /// `StructureBlockEntity.loadAdditional` (`StructureBlockEntity.java:103-127`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.name = match unique_field(record, TAG_NAME) {
            Some(NbtSummaryValue::String(value)) if !value.is_empty() => value
                .try_to_string()
                .ok()
                .and_then(|raw| normalize_resource_location(raw).ok())
                .unwrap_or_default(),
            _ => String::new(),
        };
        self.author = nbt_string(record, TAG_AUTHOR).cloned().unwrap_or_default();
        self.metadata = nbt_string(record, TAG_METADATA)
            .cloned()
            .unwrap_or_default();
        self.pos = (
            int_or(record, TAG_POS_X, DEFAULT_POS.0)
                .clamp(-MAX_OFFSET_PER_AXIS, MAX_OFFSET_PER_AXIS),
            int_or(record, TAG_POS_Y, DEFAULT_POS.1)
                .clamp(-MAX_OFFSET_PER_AXIS, MAX_OFFSET_PER_AXIS),
            int_or(record, TAG_POS_Z, DEFAULT_POS.2)
                .clamp(-MAX_OFFSET_PER_AXIS, MAX_OFFSET_PER_AXIS),
        );
        self.size = (
            int_or(record, TAG_SIZE_X, 0).clamp(0, MAX_SIZE_PER_AXIS),
            int_or(record, TAG_SIZE_Y, 0).clamp(0, MAX_SIZE_PER_AXIS),
            int_or(record, TAG_SIZE_Z, 0).clamp(0, MAX_SIZE_PER_AXIS),
        );
        self.rotation = match unique_field(record, TAG_ROTATION) {
            Some(NbtSummaryValue::String(value)) => {
                StructureBlockRotation::from_legacy(&value.try_to_string().unwrap_or_default())
            }
            _ => StructureBlockRotation::None,
        };
        self.mirror = match unique_field(record, TAG_MIRROR) {
            Some(NbtSummaryValue::String(value)) => {
                StructureBlockMirror::from_legacy(&value.try_to_string().unwrap_or_default())
            }
            _ => StructureBlockMirror::None,
        };
        // Missing/invalid `mode` is `StructureMode.DATA` (`:118`).
        self.mode = match unique_field(record, TAG_MODE) {
            Some(NbtSummaryValue::String(value)) => {
                StructureBlockMode::from_legacy(&value.try_to_string().unwrap_or_default())
            }
            _ => StructureBlockMode::Data,
        };
        self.ignore_entities = boolean_or(record, TAG_IGNORE_ENTITIES, true);
        self.strict = boolean_or(record, TAG_STRICT, false);
        self.powered = boolean_or(record, TAG_POWERED, false);
        self.show_air = boolean_or(record, TAG_SHOW_AIR, false);
        self.show_bounding_box = boolean_or(record, TAG_SHOW_BOUNDING_BOX, true);
        self.integrity_bits = float_or(record, TAG_INTEGRITY, 1.0).to_bits();
        self.seed = long_or(record, TAG_SEED, 0);
    }
}

fn bool_entry(name: &str, value: bool) -> NbtSummaryEntry {
    entry(name, NbtSummaryValue::Byte(i8::from(value)))
}

fn nbt_string<'a>(
    record: &'a [NbtSummaryEntry],
    name: &str,
) -> Option<&'a bbb_protocol::JavaString> {
    match unique_field(record, name) {
        Some(NbtSummaryValue::String(value)) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_STRUCTURE_BLOCK_ID;
    use bbb_protocol::packets::{
        BlockPos, SetStructureBlock, StructureBlockMirror, StructureBlockMode,
        StructureBlockRotation, StructureBlockUpdateKind, Vec3i,
    };
    use bbb_protocol::NbtSummaryValue;

    use super::{
        TAG_AUTHOR, TAG_IGNORE_ENTITIES, TAG_INTEGRITY, TAG_METADATA, TAG_MIRROR, TAG_MODE,
        TAG_NAME, TAG_POS_X, TAG_ROTATION, TAG_SEED, TAG_SIZE_Y, TAG_STRICT,
    };
    use crate::block_entity::{new_block_entity, unique_field};

    fn pos() -> BlockPos {
        BlockPos { x: 4, y: 5, z: 6 }
    }

    fn structure_block() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_STRUCTURE_BLOCK_ID)
            .expect("minecraft:structure_block has a default state")
    }

    fn overlay(
        record: &mut Vec<bbb_protocol::NbtSummaryEntry>,
        fields: Vec<bbb_protocol::NbtSummaryEntry>,
    ) {
        for field in fields {
            if let Some(existing) = record.iter_mut().find(|entry| entry.name == field.name) {
                existing.value = field.value;
            } else {
                record.push(field);
            }
        }
    }

    #[test]
    fn structure_block_save_load_round_trips_owned_fields() {
        let state = structure_block();
        let mut record = new_block_entity(pos(), state)
            .expect("a structure block builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let name = NbtSummaryValue::string("minecraft:village/house");
        let integrity = NbtSummaryValue::Float(0.5_f32.to_bits());
        let mut edited = super::StructureBlockEntity::default();
        edited.set_author("Ada".to_string());
        edited.set_powered(true);
        edited.apply_edit(&SetStructureBlock {
            pos: pos(),
            update: StructureBlockUpdateKind::UpdateData,
            mode: StructureBlockMode::Save,
            name: "minecraft:village/house".to_string(),
            offset: BlockPos { x: -3, y: 8, z: 2 },
            size: Vec3i { x: 4, y: 5, z: 6 },
            mirror: StructureBlockMirror::LeftRight,
            rotation: StructureBlockRotation::Clockwise90,
            data: "meta".into(),
            ignore_entities: false,
            strict: true,
            show_air: true,
            show_bounding_box: false,
            integrity: 0.5,
            seed: 77,
        });
        overlay(&mut record, edited.save_additional());
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("structure block loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let mut restored = super::StructureBlockEntity::default();
        restored.load_additional(&saved);
        assert_eq!(restored, edited);
        assert_eq!(unique_field(&saved, TAG_NAME), Some(&name));
        assert_eq!(
            unique_field(&saved, TAG_AUTHOR),
            Some(&NbtSummaryValue::string("Ada"))
        );
        assert_eq!(
            unique_field(&saved, TAG_METADATA),
            Some(&NbtSummaryValue::string("meta"))
        );
        assert_eq!(
            unique_field(&saved, TAG_POS_X),
            Some(&NbtSummaryValue::Int(-3))
        );
        assert_eq!(
            unique_field(&saved, TAG_SIZE_Y),
            Some(&NbtSummaryValue::Int(5))
        );
        assert_eq!(
            unique_field(&saved, TAG_ROTATION),
            Some(&NbtSummaryValue::string("CLOCKWISE_90"))
        );
        assert_eq!(
            unique_field(&saved, TAG_MIRROR),
            Some(&NbtSummaryValue::string("LEFT_RIGHT"))
        );
        assert_eq!(
            unique_field(&saved, TAG_MODE),
            Some(&NbtSummaryValue::string("SAVE"))
        );
        assert_eq!(
            unique_field(&saved, TAG_IGNORE_ENTITIES),
            Some(&NbtSummaryValue::Byte(0))
        );
        assert_eq!(
            unique_field(&saved, TAG_STRICT),
            Some(&NbtSummaryValue::Byte(1))
        );
        assert_eq!(unique_field(&saved, TAG_INTEGRITY), Some(&integrity));
        assert_eq!(
            unique_field(&saved, TAG_SEED),
            Some(&NbtSummaryValue::Long(77))
        );
        assert_eq!(
            unique_field(
                &loaded
                    .update_tag_with_resolver(crate::block_entity::test_item_resolver())
                    .expect("fixture codec succeeds"),
                TAG_NAME
            ),
            Some(&name)
        );
    }
}
