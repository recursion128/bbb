//! `TestBlockEntity` (`TestBlockEntity.java:18-127`) and
//! `TestInstanceBlockEntity` (`TestInstanceBlockEntity.java:55-503`).
//!
//! `/test` execution stays outside this slice; the objects still round-trip.

use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::TestBlockMode;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::value_input::{boolean_or, codec_boolean};
use super::{entry, unique_field};

const TAG_MODE: &str = "mode";
const TAG_MESSAGE: &str = "message";
const TAG_POWERED: &str = "powered";
const TAG_DATA: &str = "data";
const TAG_ERRORS: &str = "errors";
const TAG_TEST: &str = "test";
const TAG_SIZE: &str = "size";
const TAG_ROTATION: &str = "rotation";
const TAG_IGNORE_ENTITIES: &str = "ignore_entities";
const TAG_STATUS: &str = "status";
const TAG_ERROR_MESSAGE: &str = "error_message";
const TAG_POS: &str = "pos";
const TAG_TEXT: &str = "text";

/// `Rotation.CODEC` (`Rotation.java:24`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TestInstanceRotation {
    #[default]
    None,
    Clockwise90,
    Clockwise180,
    Counterclockwise90,
}

impl TestInstanceRotation {
    fn codec_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Clockwise90 => "clockwise_90",
            Self::Clockwise180 => "180",
            Self::Counterclockwise90 => "counterclockwise_90",
        }
    }

    fn from_codec(name: &str) -> Option<Self> {
        match name {
            "none" => Some(Self::None),
            "clockwise_90" => Some(Self::Clockwise90),
            "180" => Some(Self::Clockwise180),
            "counterclockwise_90" => Some(Self::Counterclockwise90),
            _ => None,
        }
    }
}

/// `TestInstanceBlockEntity.Status` (`TestInstanceBlockEntity.java:476-502`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TestInstanceStatus {
    #[default]
    Cleared,
    Running,
    Finished,
}

impl TestInstanceStatus {
    fn codec_name(self) -> &'static str {
        match self {
            Self::Cleared => "cleared",
            Self::Running => "running",
            Self::Finished => "finished",
        }
    }

    fn from_codec(name: &str) -> Option<Self> {
        match name {
            "cleared" => Some(Self::Cleared),
            "running" => Some(Self::Running),
            "finished" => Some(Self::Finished),
            _ => None,
        }
    }
}

/// `TestInstanceBlockEntity.Data` (`TestInstanceBlockEntity.java:415-463`).
#[derive(Debug, Clone, PartialEq, Default)]
struct TestInstanceData {
    test: Option<String>,
    size: (i32, i32, i32),
    rotation: TestInstanceRotation,
    ignore_entities: bool,
    status: TestInstanceStatus,
    error_message: Option<NbtSummaryValue>,
}

/// `TestInstanceBlockEntity.ErrorMarker` (`TestInstanceBlockEntity.java:465-474`).
#[derive(Debug, Clone, PartialEq)]
struct ErrorMarker {
    pos: (i32, i32, i32),
    text: NbtSummaryValue,
}

/// `TestBlockEntity` (`TestBlockEntity.java:22-25`).
#[derive(Debug, Clone, PartialEq)]
pub struct TestBlockEntity {
    mode: TestBlockMode,
    message: bbb_protocol::JavaString,
    powered: bool,
}

impl Default for TestBlockEntity {
    /// `new TestBlockEntity` (`TestBlockEntity.java:27-30`) on the default
    /// block state (`Block.java:247`, first `TestBlockMode` is `START`).
    fn default() -> Self {
        Self {
            mode: TestBlockMode::Start,
            message: Default::default(),
            powered: false,
        }
    }
}

impl TestBlockEntity {
    /// `TestBlockEntity.saveAdditional` (`TestBlockEntity.java:33-37`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        vec![
            entry(
                TAG_MODE,
                NbtSummaryValue::string(self.mode.serialized_name()),
            ),
            entry(TAG_MESSAGE, NbtSummaryValue::string(self.message.clone())),
            entry(TAG_POWERED, NbtSummaryValue::Byte(i8::from(self.powered))),
        ]
    }

    /// `TestBlockEntity.loadAdditional` (`TestBlockEntity.java:40-44`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.mode = match unique_field(record, TAG_MODE) {
            Some(NbtSummaryValue::String(value)) => {
                match value.try_to_string().as_deref().unwrap_or("") {
                    "start" => TestBlockMode::Start,
                    "log" => TestBlockMode::Log,
                    "accept" => TestBlockMode::Accept,
                    _ => TestBlockMode::Fail,
                }
            }
            _ => TestBlockMode::Fail,
        };
        self.message = match unique_field(record, TAG_MESSAGE) {
            Some(NbtSummaryValue::String(value)) => value.clone(),
            _ => Default::default(),
        };
        self.powered = boolean_or(record, TAG_POWERED, false);
    }
}

/// `TestInstanceBlockEntity` (`TestInstanceBlockEntity.java:64-65`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TestInstanceBlockEntity {
    data: TestInstanceData,
    error_markers: Vec<ErrorMarker>,
}

impl TestInstanceBlockEntity {
    /// `TestInstanceBlockEntity.saveAdditional` (`TestInstanceBlockEntity.java:168-173`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = vec![entry(
            TAG_DATA,
            NbtSummaryValue::Compound(save_data(&self.data)),
        )];
        if !self.error_markers.is_empty() {
            entries.push(entry(
                TAG_ERRORS,
                NbtSummaryValue::List(
                    self.error_markers
                        .iter()
                        .map(|marker| {
                            NbtSummaryValue::Compound(vec![
                                entry(
                                    TAG_POS,
                                    NbtSummaryValue::IntArray(vec![
                                        marker.pos.0,
                                        marker.pos.1,
                                        marker.pos.2,
                                    ]),
                                ),
                                entry(TAG_TEXT, marker.text.clone()),
                            ])
                        })
                        .collect(),
                ),
            ));
        }
        entries
    }

    /// `TestInstanceBlockEntity.loadAdditional` (`TestInstanceBlockEntity.java:161-165`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        if let Some(value) = unique_field(record, TAG_DATA) {
            if let Some(data) = parse_data(value) {
                self.data = data;
            }
        }
        self.error_markers = parse_errors(record);
    }
}

fn save_data(data: &TestInstanceData) -> Vec<NbtSummaryEntry> {
    let mut entries = Vec::new();
    if let Some(test) = &data.test {
        entries.push(entry(TAG_TEST, NbtSummaryValue::string(test.clone())));
    }
    entries.push(entry(
        TAG_SIZE,
        NbtSummaryValue::IntArray(vec![data.size.0, data.size.1, data.size.2]),
    ));
    entries.push(entry(
        TAG_ROTATION,
        NbtSummaryValue::string(data.rotation.codec_name()),
    ));
    entries.push(entry(
        TAG_IGNORE_ENTITIES,
        NbtSummaryValue::Byte(i8::from(data.ignore_entities)),
    ));
    entries.push(entry(
        TAG_STATUS,
        NbtSummaryValue::string(data.status.codec_name()),
    ));
    if let Some(message) = &data.error_message {
        entries.push(entry(TAG_ERROR_MESSAGE, message.clone()));
    }
    entries
}

fn parse_data(value: &NbtSummaryValue) -> Option<TestInstanceData> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let size = match unique_field(entries, TAG_SIZE) {
        Some(NbtSummaryValue::IntArray(values)) if values.len() == 3 => {
            (values[0], values[1], values[2])
        }
        _ => return None,
    };
    let rotation = match unique_field(entries, TAG_ROTATION) {
        Some(NbtSummaryValue::String(value)) => {
            TestInstanceRotation::from_codec(&value.try_to_string().ok()?)?
        }
        _ => return None,
    };
    let ignore_entities = unique_field(entries, TAG_IGNORE_ENTITIES).and_then(codec_boolean)?;
    let status = match unique_field(entries, TAG_STATUS) {
        Some(NbtSummaryValue::String(value)) => {
            TestInstanceStatus::from_codec(&value.try_to_string().ok()?)?
        }
        _ => return None,
    };
    let test = match unique_field(entries, TAG_TEST) {
        Some(NbtSummaryValue::String(value)) => {
            Some(normalize_resource_location(value.try_to_string().ok()?).ok()?)
        }
        Some(_) => return None,
        None => None,
    };
    Some(TestInstanceData {
        test,
        size,
        rotation,
        ignore_entities,
        status,
        error_message: unique_field(entries, TAG_ERROR_MESSAGE).cloned(),
    })
}

fn parse_errors(record: &[NbtSummaryEntry]) -> Vec<ErrorMarker> {
    let Some(NbtSummaryValue::List(markers)) = unique_field(record, TAG_ERRORS) else {
        return Vec::new();
    };
    markers.iter().filter_map(parse_error_marker).collect()
}

fn parse_error_marker(value: &NbtSummaryValue) -> Option<ErrorMarker> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let pos = match unique_field(entries, TAG_POS) {
        Some(NbtSummaryValue::IntArray(values)) if values.len() == 3 => {
            (values[0], values[1], values[2])
        }
        _ => return None,
    };
    let text = unique_field(entries, TAG_TEXT)?.clone();
    Some(ErrorMarker { pos, text })
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::{
        VANILLA_BLOCK_TEST_BLOCK_ID, VANILLA_BLOCK_TEST_INSTANCE_BLOCK_ID,
    };
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::NbtSummaryValue;

    use super::{
        entry, unique_field, TAG_DATA, TAG_ERRORS, TAG_ERROR_MESSAGE, TAG_IGNORE_ENTITIES,
        TAG_MESSAGE, TAG_MODE, TAG_POS, TAG_POWERED, TAG_ROTATION, TAG_SIZE, TAG_STATUS, TAG_TEST,
        TAG_TEXT,
    };
    use crate::block_entity::new_block_entity;

    fn pos() -> BlockPos {
        BlockPos { x: 2, y: 3, z: 4 }
    }

    fn test_block() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_TEST_BLOCK_ID)
            .expect("minecraft:test_block has a default state")
    }

    fn test_instance() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_TEST_INSTANCE_BLOCK_ID)
            .expect("minecraft:test_instance_block has a default state")
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
    fn test_block_save_load_round_trips_owned_fields() {
        let state = test_block();
        let mut record = new_block_entity(pos(), state)
            .expect("a test block builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        overlay(
            &mut record,
            vec![
                entry(TAG_MODE, NbtSummaryValue::string("log")),
                entry(TAG_MESSAGE, NbtSummaryValue::string("ok")),
                entry(TAG_POWERED, NbtSummaryValue::Byte(1)),
            ],
        );
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("test block loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(
            unique_field(&saved, TAG_MODE),
            Some(&NbtSummaryValue::string("log"))
        );
        assert_eq!(
            unique_field(&saved, TAG_MESSAGE),
            Some(&NbtSummaryValue::string("ok"))
        );
        assert_eq!(
            unique_field(&saved, TAG_POWERED),
            Some(&NbtSummaryValue::Byte(1))
        );
    }

    #[test]
    fn test_instance_save_load_round_trips_data_and_errors() {
        let state = test_instance();
        let mut record = new_block_entity(pos(), state)
            .expect("a test instance block builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let error_text = NbtSummaryValue::string("boom");
        overlay(
            &mut record,
            vec![
                entry(
                    TAG_DATA,
                    NbtSummaryValue::Compound(vec![
                        entry(TAG_TEST, NbtSummaryValue::string("minecraft:demo")),
                        entry(TAG_SIZE, NbtSummaryValue::IntArray(vec![2, 3, 4])),
                        entry(TAG_ROTATION, NbtSummaryValue::string("clockwise_90")),
                        entry(TAG_IGNORE_ENTITIES, NbtSummaryValue::Byte(1)),
                        entry(TAG_STATUS, NbtSummaryValue::string("finished")),
                        entry(TAG_ERROR_MESSAGE, NbtSummaryValue::string("failed")),
                    ]),
                ),
                entry(
                    TAG_ERRORS,
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry(TAG_POS, NbtSummaryValue::IntArray(vec![1, 2, 3])),
                        entry(TAG_TEXT, error_text.clone()),
                    ])]),
                ),
            ],
        );
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("test instance loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let data = unique_field(&saved, TAG_DATA).expect("data is written");
        let NbtSummaryValue::Compound(data) = data else {
            panic!("data is a compound");
        };
        assert_eq!(
            unique_field(data, TAG_TEST),
            Some(&NbtSummaryValue::string("minecraft:demo"))
        );
        assert_eq!(
            unique_field(data, TAG_SIZE),
            Some(&NbtSummaryValue::IntArray(vec![2, 3, 4]))
        );
        assert_eq!(
            unique_field(data, TAG_ROTATION),
            Some(&NbtSummaryValue::string("clockwise_90"))
        );
        assert_eq!(
            unique_field(data, TAG_IGNORE_ENTITIES),
            Some(&NbtSummaryValue::Byte(1))
        );
        assert_eq!(
            unique_field(data, TAG_STATUS),
            Some(&NbtSummaryValue::string("finished"))
        );
        assert_eq!(
            unique_field(data, TAG_ERROR_MESSAGE),
            Some(&NbtSummaryValue::string("failed"))
        );
        let errors = unique_field(&saved, TAG_ERRORS).expect("errors are written");
        let NbtSummaryValue::List(errors) = errors else {
            panic!("errors is a list");
        };
        let NbtSummaryValue::Compound(marker) = &errors[0] else {
            panic!("error marker is a compound");
        };
        assert_eq!(
            unique_field(marker, TAG_POS),
            Some(&NbtSummaryValue::IntArray(vec![1, 2, 3]))
        );
        assert_eq!(unique_field(marker, TAG_TEXT), Some(&error_text));
        let again = crate::block_entity::load_test_fixture(pos(), state, &saved)
            .expect("fixture codec succeeds")
            .expect("second load");
        assert_eq!(
            unique_field(
                &again
                    .save_with_full_metadata_with_resolver(
                        crate::block_entity::test_item_resolver(),
                    )
                    .expect("fixture codec succeeds"),
                TAG_ERRORS
            ),
            unique_field(&saved, TAG_ERRORS)
        );
    }
}
