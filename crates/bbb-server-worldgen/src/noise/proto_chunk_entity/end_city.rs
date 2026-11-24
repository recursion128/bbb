use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::{SaveError, SaveResult};

use super::entry;

#[derive(Clone, Debug, PartialEq)]
pub struct EndCityItemFrameRecord {
    attachment: BlockPos,
    facing: Direction,
    item: String,
    uuid: [i32; 4],
}

impl EndCityItemFrameRecord {
    /// Canonical block attachment and subtype fields follow
    /// `ItemFrame.java:269-287,325-354` and `BlockAttachedEntity.java:119-131`.
    pub fn new(
        attachment: BlockPos,
        facing: Direction,
        item: impl Into<String>,
        uuid: [i32; 4],
    ) -> SaveResult<Self> {
        if !matches!(
            facing,
            Direction::North | Direction::South | Direction::West | Direction::East
        ) {
            return Err(SaveError::InvalidNbt {
                path: Some("minecraft:item_frame.Facing".into()),
                detail: "End City item frame facing must be horizontal".to_string(),
            });
        }
        let item = item.into();
        if item != "minecraft:elytra" {
            return Err(SaveError::InvalidNbt {
                path: Some("minecraft:item_frame.Item".into()),
                detail: format!("unowned End City frame item {item}"),
            });
        }
        Ok(Self {
            attachment,
            facing,
            item,
            uuid,
        })
    }

    pub fn entries(&self) -> Vec<NbtSummaryEntry> {
        let position = frame_position(self.attachment, self.facing);
        vec![
            entry("id", NbtSummaryValue::string("minecraft:item_frame")),
            entry(
                "Pos",
                NbtSummaryValue::List(
                    position
                        .into_iter()
                        .map(|value| NbtSummaryValue::Double(value.to_bits()))
                        .collect(),
                ),
            ),
            entry(
                "Motion",
                NbtSummaryValue::List(
                    [0.0_f64; 3]
                        .into_iter()
                        .map(|value| NbtSummaryValue::Double(value.to_bits()))
                        .collect(),
                ),
            ),
            entry(
                "Rotation",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(frame_y_rotation(self.facing).to_bits()),
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                ]),
            ),
            entry("fall_distance", NbtSummaryValue::Double(0.0_f64.to_bits())),
            entry("Fire", NbtSummaryValue::Short(0)),
            entry("Air", NbtSummaryValue::Short(300)),
            entry("OnGround", NbtSummaryValue::Byte(0)),
            entry("Invulnerable", NbtSummaryValue::Byte(0)),
            entry("PortalCooldown", NbtSummaryValue::Int(0)),
            entry(
                "UUID",
                NbtSummaryValue::IntArray(self.uuid.into_iter().collect()),
            ),
            entry(
                "block_pos",
                NbtSummaryValue::IntArray(vec![
                    self.attachment.x,
                    self.attachment.y,
                    self.attachment.z,
                ]),
            ),
            entry(
                "Item",
                NbtSummaryValue::Compound(vec![
                    entry("id", NbtSummaryValue::string(self.item.clone())),
                    entry("count", NbtSummaryValue::Int(1)),
                ]),
            ),
            entry("ItemRotation", NbtSummaryValue::Byte(0)),
            entry("ItemDropChance", NbtSummaryValue::Float(1.0_f32.to_bits())),
            entry("Facing", NbtSummaryValue::Byte(self.facing.id() as i8)),
            entry("Invisible", NbtSummaryValue::Byte(0)),
            entry("Fixed", NbtSummaryValue::Byte(0)),
        ]
    }

    pub fn position(&self) -> [f64; 3] {
        frame_position(self.attachment, self.facing)
    }
}

fn frame_position(pos: BlockPos, facing: Direction) -> [f64; 3] {
    let (step_x, step_y, step_z) = match facing {
        Direction::Down => (0, -1, 0),
        Direction::Up => (0, 1, 0),
        Direction::North => (0, 0, -1),
        Direction::South => (0, 0, 1),
        Direction::West => (-1, 0, 0),
        Direction::East => (1, 0, 0),
    };
    [
        f64::from(pos.x) + 0.5 - f64::from(step_x) * 0.46875,
        f64::from(pos.y) + 0.5 - f64::from(step_y) * 0.46875,
        f64::from(pos.z) + 0.5 - f64::from(step_z) * 0.46875,
    ]
}

fn frame_y_rotation(facing: Direction) -> f32 {
    match facing {
        Direction::South => 0.0,
        Direction::West => 90.0,
        Direction::North => 180.0,
        Direction::East => 270.0,
        Direction::Down | Direction::Up => 0.0,
    }
}
