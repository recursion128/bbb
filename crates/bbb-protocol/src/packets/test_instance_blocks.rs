use serde::{Deserialize, Serialize};

use crate::{
    codec::{Encoder, ProtocolError, Result},
    ids,
};

use super::{chunks, normalize_resource_location, BlockPos, Vec3i};

pub const MAX_TEST_INSTANCE_ID_UTF16_UNITS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestInstanceBlockActionKind {
    Init,
    Query,
    Set,
    Reset,
    Save,
    Export,
    Run,
}

impl TestInstanceBlockActionKind {
    fn wire_id(self) -> i32 {
        match self {
            Self::Init => 0,
            Self::Query => 1,
            Self::Set => 2,
            Self::Reset => 3,
            Self::Save => 4,
            Self::Export => 5,
            Self::Run => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestInstanceBlockRotation {
    None,
    Clockwise90,
    Clockwise180,
    Counterclockwise90,
}

impl TestInstanceBlockRotation {
    fn wire_id(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Clockwise90 => 1,
            Self::Clockwise180 => 2,
            Self::Counterclockwise90 => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInstanceBlockAction {
    pub pos: BlockPos,
    pub action: TestInstanceBlockActionKind,
    pub test: Option<String>,
    pub size: Vec3i,
    pub rotation: TestInstanceBlockRotation,
    pub ignore_entities: bool,
}

pub fn encode_play_test_instance_block_action(
    packet: &TestInstanceBlockAction,
) -> Result<(i32, Vec<u8>)> {
    let test = packet
        .test
        .as_ref()
        .map(|test| {
            let len = test.encode_utf16().count();
            if len > MAX_TEST_INSTANCE_ID_UTF16_UNITS {
                return Err(ProtocolError::StringTooLong {
                    actual: len,
                    max: MAX_TEST_INSTANCE_ID_UTF16_UNITS,
                });
            }
            normalize_resource_location(test.clone())
        })
        .transpose()?;

    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(packet.pos));
    out.write_var_i32(packet.action.wire_id());
    out.write_bool(test.is_some());
    if let Some(test) = test {
        out.write_string(&test);
    }
    out.write_var_i32(packet.size.x);
    out.write_var_i32(packet.size.y);
    out.write_var_i32(packet.size.z);
    out.write_var_i32(packet.rotation.wire_id());
    out.write_bool(packet.ignore_entities);
    // TestInstanceBlockEditScreen uses the packet constructor that fixes the
    // outgoing Data status to CLEARED and its error Component to empty.
    out.write_var_i32(0);
    out.write_bool(false);
    Ok((
        ids::play::SERVERBOUND_TEST_INSTANCE_BLOCK_ACTION,
        out.into_inner(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Decoder;

    fn packet(
        action: TestInstanceBlockActionKind,
        rotation: TestInstanceBlockRotation,
    ) -> TestInstanceBlockAction {
        TestInstanceBlockAction {
            pos: BlockPos {
                x: -12,
                y: 70,
                z: 31,
            },
            action,
            test: Some("example".to_string()),
            size: Vec3i { x: -1, y: 48, z: 3 },
            rotation,
            ignore_entities: true,
        }
    }

    #[test]
    fn encodes_test_instance_action_wire_order_ids_and_fixed_editor_data() {
        let actions = [
            TestInstanceBlockActionKind::Init,
            TestInstanceBlockActionKind::Query,
            TestInstanceBlockActionKind::Set,
            TestInstanceBlockActionKind::Reset,
            TestInstanceBlockActionKind::Save,
            TestInstanceBlockActionKind::Export,
            TestInstanceBlockActionKind::Run,
        ];
        let rotations = [
            TestInstanceBlockRotation::None,
            TestInstanceBlockRotation::Clockwise90,
            TestInstanceBlockRotation::Clockwise180,
            TestInstanceBlockRotation::Counterclockwise90,
        ];

        for (action_id, action) in actions.into_iter().enumerate() {
            for (rotation_id, rotation) in rotations.into_iter().enumerate() {
                let (id, payload) =
                    encode_play_test_instance_block_action(&packet(action, rotation)).unwrap();
                assert_eq!(id, ids::play::SERVERBOUND_TEST_INSTANCE_BLOCK_ACTION);
                let mut decoder = Decoder::new(&payload);
                assert_eq!(
                    decoder.read_i64().unwrap(),
                    chunks::encode_block_pos(BlockPos {
                        x: -12,
                        y: 70,
                        z: 31,
                    })
                );
                assert_eq!(decoder.read_var_i32().unwrap(), action_id as i32);
                assert!(decoder.read_bool().unwrap());
                assert_eq!(decoder.read_string(128).unwrap(), "minecraft:example");
                assert_eq!(decoder.read_var_i32().unwrap(), -1);
                assert_eq!(decoder.read_var_i32().unwrap(), 48);
                assert_eq!(decoder.read_var_i32().unwrap(), 3);
                assert_eq!(decoder.read_var_i32().unwrap(), rotation_id as i32);
                assert!(decoder.read_bool().unwrap());
                assert_eq!(decoder.read_var_i32().unwrap(), 0);
                assert!(!decoder.read_bool().unwrap());
                assert!(decoder.is_empty());
            }
        }
    }

    #[test]
    fn test_instance_action_validates_optional_id_and_utf16_boundary() {
        let mut without_test = packet(
            TestInstanceBlockActionKind::Query,
            TestInstanceBlockRotation::None,
        );
        without_test.test = None;
        let (_, payload) = encode_play_test_instance_block_action(&without_test).unwrap();
        let mut decoder = Decoder::new(&payload);
        decoder.read_i64().unwrap();
        decoder.read_var_i32().unwrap();
        assert!(!decoder.read_bool().unwrap());

        let mut invalid = packet(
            TestInstanceBlockActionKind::Set,
            TestInstanceBlockRotation::None,
        );
        invalid.test = Some("minecraft:Invalid".to_string());
        assert!(encode_play_test_instance_block_action(&invalid)
            .unwrap_err()
            .to_string()
            .contains("invalid resource location"));

        invalid.test = Some("a".repeat(MAX_TEST_INSTANCE_ID_UTF16_UNITS + 1));
        assert!(matches!(
            encode_play_test_instance_block_action(&invalid),
            Err(ProtocolError::StringTooLong {
                actual: 129,
                max: MAX_TEST_INSTANCE_ID_UTF16_UNITS,
            })
        ));
    }
}
