//! Writer-canonical ProtoChunk entities emitted by a mineshaft corridor.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use bbb_server_types::save::{SaveError, SaveResult};

const LOOT_TABLE: &str = "minecraft:chests/abandoned_mineshaft";

/// The entity record created by `MineshaftPieces.createChest`.
///
/// `MineshaftPieces.java:356-383` consumes the rail-direction draw before the
/// minecart loot seed; this record only retains the already-consumed seed.
#[derive(Clone, Debug, PartialEq)]
pub struct MineshaftChestMinecartRecord {
    position: [f64; 3],
    uuid: [i32; 4],
    loot_table_seed: i64,
}

impl MineshaftChestMinecartRecord {
    pub fn new(position: [f64; 3], uuid: [i32; 4], loot_table_seed: i64) -> SaveResult<Self> {
        if position.iter().any(|value| !value.is_finite()) {
            return Err(SaveError::InvalidNbt {
                path: Some("minecraft:chest_minecart.Pos".into()),
                detail: "all coordinates must be finite".to_string(),
            });
        }
        Ok(Self {
            position,
            uuid,
            loot_table_seed,
        })
    }

    pub fn entries(&self) -> Vec<NbtSummaryEntry> {
        let doubles = |values: [f64; 3]| {
            NbtSummaryValue::List(
                values
                    .into_iter()
                    .map(|value| NbtSummaryValue::Double(value.to_bits()))
                    .collect(),
            )
        };
        let mut entries = vec![
            entry("id", NbtSummaryValue::string("minecraft:chest_minecart")),
            entry("Pos", doubles(self.position)),
            entry("Motion", doubles([0.0, 0.0, 0.0])),
            entry(
                "Rotation",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
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
            // `AbstractMinecart.java:476-493`: these are always written and
            // the default display offset 8 is omitted by the writer.
            entry("FlippedRotation", NbtSummaryValue::Byte(0)),
            entry("HasTicked", NbtSummaryValue::Byte(1)),
            entry("LootTable", NbtSummaryValue::string(LOOT_TABLE)),
        ];
        if self.loot_table_seed != 0 {
            entries.push(entry(
                "LootTableSeed",
                NbtSummaryValue::Long(self.loot_table_seed),
            ));
        }
        entries
    }

    pub fn loot_table_seed(&self) -> i64 {
        self.loot_table_seed
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_zero_seed_without_seed_leaf() {
        let record =
            MineshaftChestMinecartRecord::new([0.5, 40.5, -15.5], [1, 2, 3, 4], 0).unwrap();
        let entries = record.entries();
        assert!(entries.iter().any(|entry| entry.name == "LootTable"));
        assert!(!entries.iter().any(|entry| entry.name == "LootTableSeed"));
        assert!(!entries.iter().any(|entry| entry.name == "Items"));
        assert!(!entries.iter().any(|entry| entry.name == "DisplayOffset"));
    }

    #[test]
    fn emits_nonzero_seed_and_rejects_nonfinite_position() {
        let record = MineshaftChestMinecartRecord::new([0.5, 40.5, 0.5], [1, 2, 3, 4], -7).unwrap();
        assert!(record.entries().iter().any(|entry| {
            entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(-7)
        }));
        assert!(MineshaftChestMinecartRecord::new([f64::NAN, 0.0, 0.0], [0, 0, 0, 1], 1,).is_err());
    }
}
