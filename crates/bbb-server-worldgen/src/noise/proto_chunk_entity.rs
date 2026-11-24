//! Writer-canonical ProtoChunk entities emitted by structure `postProcess`.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::noise::mineshaft::MineshaftChestMinecartRecord;
use bbb_server_types::save::{SaveError, SaveResult};

mod end_city;
pub mod template;
pub use end_city::EndCityItemFrameRecord;
pub use template::TemplateMobRecord;

/// Flags an in-memory ProtoChunk entity record that still owes
/// `Mob.finalizeSpawn(STRUCTURE)`. The server finalizes and strips it before
/// the record is saved or joins the entity store; it is never written to disk.
pub const STRUCTURE_FINALIZE_MARKER: &str = "bbb:structure_finalize";

pub fn mark_structure_finalize(record: &mut Vec<NbtSummaryEntry>) {
    record.push(entry(STRUCTURE_FINALIZE_MARKER, NbtSummaryValue::Byte(1)));
}

pub fn has_structure_finalize_marker(record: &[NbtSummaryEntry]) -> bool {
    record
        .last()
        .is_some_and(|entry| entry.name == STRUCTURE_FINALIZE_MARKER)
}

/// Removes the marker; whether the record carried it.
pub fn take_structure_finalize_marker(record: &mut Vec<NbtSummaryEntry>) -> bool {
    let marked = has_structure_finalize_marker(record);
    if marked {
        record.pop();
    }
    marked
}

/// STRUCTURE-spawned mob kinds that persist as ProtoChunk entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureMobKind {
    ElderGuardian,
    Witch,
    Cat,
    Evoker,
    Vindicator,
    Allay,
    Drowned,
    Shulker,
}

impl StructureMobKind {
    pub fn from_resource_id(id: &str) -> Option<Self> {
        Some(match id {
            "minecraft:elder_guardian" => Self::ElderGuardian,
            "minecraft:witch" => Self::Witch,
            "minecraft:cat" => Self::Cat,
            "minecraft:evoker" => Self::Evoker,
            "minecraft:vindicator" => Self::Vindicator,
            "minecraft:allay" => Self::Allay,
            "minecraft:drowned" => Self::Drowned,
            "minecraft:shulker" => Self::Shulker,
            _ => return None,
        })
    }

    pub const fn resource_id(self) -> &'static str {
        match self {
            Self::ElderGuardian => "minecraft:elder_guardian",
            Self::Witch => "minecraft:witch",
            Self::Cat => "minecraft:cat",
            Self::Evoker => "minecraft:evoker",
            Self::Vindicator => "minecraft:vindicator",
            Self::Allay => "minecraft:allay",
            Self::Drowned => "minecraft:drowned",
            Self::Shulker => "minecraft:shulker",
        }
    }

    /// Every piece but the End City sentry calls `finalizeSpawn`
    /// (`SwampHutPiece.java:105,124`, `WoodlandMansionPieces.java:1295`,
    /// `OceanMonumentPieces.java:1575`, `OceanRuinPieces.java:334`,
    /// `EndCityPieces.java:413-416`).
    pub const fn finalizes_spawn(self) -> bool {
        !matches!(self, Self::Shulker)
    }

    /// `LivingEntity.getMaxHealth` after `create` / `heal`
    /// (`ElderGuardian.java:34-36`, `Shulker.java:138-167`, and the other
    /// owned mobs' default suppliers).
    pub const fn max_health(self) -> f32 {
        match self {
            Self::ElderGuardian => 80.0,
            Self::Witch => 26.0,
            Self::Cat => 10.0,
            Self::Evoker | Self::Vindicator => 24.0,
            Self::Allay | Self::Drowned => 20.0,
            Self::Shulker => 30.0,
        }
    }
}

/// One ProtoChunk entity record: mineshaft minecarts or structure mobs.
#[derive(Clone, Debug, PartialEq)]
pub enum ProtoChunkEntityRecord {
    ChestMinecart(MineshaftChestMinecartRecord),
    EndCrystal(EndCrystalRecord),
    EndCityItemFrame(EndCityItemFrameRecord),
    StructureMob(StructureMobRecord),
    TemplateMob(TemplateMobRecord),
}

impl ProtoChunkEntityRecord {
    pub fn entries(&self) -> Vec<NbtSummaryEntry> {
        match self {
            Self::ChestMinecart(record) => record.entries(),
            Self::EndCrystal(record) => record.entries(),
            Self::EndCityItemFrame(record) => record.entries(),
            Self::StructureMob(record) => record.entries(),
            Self::TemplateMob(record) => record.entries(),
        }
    }
}

/// Fresh `EndCrystal` created by `EndSpikeFeature.placeSpike`; subtype fields
/// follow `EndCrystal.addAdditionalSaveData` (`EndSpikeFeature.java:110-115`,
/// `EndCrystal.java:66-69`).
#[derive(Clone, Debug, PartialEq)]
pub struct EndCrystalRecord {
    position: [f64; 3],
    y_rot: f32,
    beam_target: Option<[i32; 3]>,
    invulnerable: bool,
    uuid: [i32; 4],
}

impl EndCrystalRecord {
    pub fn new(
        position: [f64; 3],
        y_rot: f32,
        beam_target: Option<[i32; 3]>,
        invulnerable: bool,
        uuid: [i32; 4],
    ) -> SaveResult<Self> {
        if position.iter().any(|value| !value.is_finite()) || !y_rot.is_finite() {
            return Err(SaveError::InvalidNbt {
                path: Some("minecraft:end_crystal.Pos".into()),
                detail: "all coordinates and rotations must be finite".to_string(),
            });
        }
        Ok(Self {
            position,
            y_rot,
            beam_target,
            invulnerable,
            uuid,
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
            entry("id", NbtSummaryValue::string("minecraft:end_crystal")),
            entry("Pos", doubles(self.position)),
            entry("Motion", doubles([0.0, 0.0, 0.0])),
            entry(
                "Rotation",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(self.y_rot.to_bits()),
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                ]),
            ),
            entry("fall_distance", NbtSummaryValue::Double(0.0_f64.to_bits())),
            entry("Fire", NbtSummaryValue::Short(0)),
            entry("Air", NbtSummaryValue::Short(300)),
            entry("OnGround", NbtSummaryValue::Byte(0)),
            entry(
                "Invulnerable",
                NbtSummaryValue::Byte(i8::from(self.invulnerable)),
            ),
            entry("PortalCooldown", NbtSummaryValue::Int(0)),
            entry(
                "UUID",
                NbtSummaryValue::IntArray(self.uuid.into_iter().collect()),
            ),
        ];
        if let Some(beam_target) = self.beam_target {
            entries.push(entry(
                "beam_target",
                NbtSummaryValue::IntArray(beam_target.into_iter().collect()),
            ));
        }
        entries.push(entry("ShowBottom", NbtSummaryValue::Byte(1)));
        entries
    }
}

/// `Entity.save` of a freshly created STRUCTURE mob after `setPersistenceRequired`
/// and `snapTo` (`OceanMonumentPieces.java:1568-1578`, `SwampHutPiece.java:97-126`,
/// `WoodlandMansionPieces.java:1291-1298`) before `finalizeSpawn`, which the
/// server applies to the marked record.
#[derive(Clone, Debug, PartialEq)]
pub struct StructureMobRecord {
    kind: StructureMobKind,
    position: [f64; 3],
    uuid: [i32; 4],
}

impl StructureMobRecord {
    pub fn new(kind: StructureMobKind, position: [f64; 3], uuid: [i32; 4]) -> SaveResult<Self> {
        if position.iter().any(|value| !value.is_finite()) {
            return Err(SaveError::InvalidNbt {
                path: Some(format!("{}.Pos", kind.resource_id()).into()),
                detail: "all coordinates must be finite".to_string(),
            });
        }
        Ok(Self {
            kind,
            position,
            uuid,
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
            entry("id", NbtSummaryValue::string(self.kind.resource_id())),
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
            entry(
                "Health",
                NbtSummaryValue::Float(self.kind.max_health().to_bits()),
            ),
            entry("HurtTime", NbtSummaryValue::Short(0)),
            entry("HurtByTimestamp", NbtSummaryValue::Int(0)),
            entry("DeathTime", NbtSummaryValue::Short(0)),
            entry(
                "AbsorptionAmount",
                NbtSummaryValue::Float(0.0_f32.to_bits()),
            ),
            entry(
                "current_impulse_context_reset_grace_time",
                NbtSummaryValue::Int(0),
            ),
            entry("attributes", NbtSummaryValue::List(Vec::new())),
            entry("FallFlying", NbtSummaryValue::Byte(0)),
            entry(
                "Brain",
                NbtSummaryValue::Compound(vec![entry(
                    "memories",
                    NbtSummaryValue::Compound(Vec::new()),
                )]),
            ),
            entry("CanPickUpLoot", NbtSummaryValue::Byte(0)),
            entry("PersistenceRequired", NbtSummaryValue::Byte(1)),
            entry("LeftHanded", NbtSummaryValue::Byte(0)),
        ];
        match self.kind {
            StructureMobKind::ElderGuardian => {}
            StructureMobKind::Witch | StructureMobKind::Evoker | StructureMobKind::Vindicator => {
                // `PatrollingMonster.java:44-48`, `Raider.java:181-187`.
                entries.extend([
                    entry("PatrolLeader", NbtSummaryValue::Byte(0)),
                    entry("Patrolling", NbtSummaryValue::Byte(0)),
                    entry("Wave", NbtSummaryValue::Int(0)),
                    entry("CanJoinRaid", NbtSummaryValue::Byte(0)),
                ]);
                if self.kind == StructureMobKind::Evoker {
                    entries.push(entry("SpellTicks", NbtSummaryValue::Int(0)));
                }
            }
            StructureMobKind::Cat => {
                // `AgeableMob.java:163-166`, `Animal.java:97-99`,
                // `TamableAnimal.java:58-62`, `Cat.java:80-81,208-214`.
                entries.extend([
                    entry("Age", NbtSummaryValue::Int(0)),
                    entry("ForcedAge", NbtSummaryValue::Int(0)),
                    entry("AgeLocked", NbtSummaryValue::Byte(0)),
                    entry("InLove", NbtSummaryValue::Int(0)),
                    entry("Sitting", NbtSummaryValue::Byte(0)),
                    entry("variant", NbtSummaryValue::string("minecraft:black")),
                    entry(
                        "sound_variant",
                        NbtSummaryValue::string("minecraft:classic"),
                    ),
                    entry("CollarColor", NbtSummaryValue::Byte(14)),
                ]);
            }
            StructureMobKind::Allay => {
                // `Allay.java:427-431`.
                entries.extend([
                    entry("Inventory", NbtSummaryValue::List(Vec::new())),
                    entry(
                        "listener",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "selector",
                                NbtSummaryValue::Compound(vec![entry(
                                    "tick",
                                    NbtSummaryValue::Long(-1),
                                )]),
                            ),
                            entry("event_delay", NbtSummaryValue::Int(0)),
                        ]),
                    ),
                    entry("DuplicationCooldown", NbtSummaryValue::Long(0)),
                ]);
            }
            StructureMobKind::Drowned => {
                // `Zombie.java:397-402`.
                entries.extend([
                    entry("IsBaby", NbtSummaryValue::Byte(0)),
                    entry("CanBreakDoors", NbtSummaryValue::Byte(0)),
                    entry("InWaterTime", NbtSummaryValue::Int(-1)),
                    entry("DrownedConversionTime", NbtSummaryValue::Int(-1)),
                ]);
            }
            StructureMobKind::Shulker => {
                entries.extend([
                    entry("AttachFace", NbtSummaryValue::Byte(0)),
                    entry("Peek", NbtSummaryValue::Byte(0)),
                    entry("Color", NbtSummaryValue::Byte(16)),
                ]);
            }
        }
        entries
    }
}

pub fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(kind: StructureMobKind) -> StructureMobRecord {
        StructureMobRecord::new(kind, [0.5, 64.0, 0.5], [1, 2, 3, 4]).unwrap()
    }

    #[test]
    fn proto_record_wraps_structure_mob_id() {
        let wrapped = ProtoChunkEntityRecord::StructureMob(record(StructureMobKind::ElderGuardian));
        assert!(wrapped.entries().iter().any(|entry| {
            entry.name == "id" && entry.value == NbtSummaryValue::string("minecraft:elder_guardian")
        }));
    }
}
