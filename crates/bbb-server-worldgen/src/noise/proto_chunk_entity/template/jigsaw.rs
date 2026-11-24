//! Jigsaw template-entity leaves (village, outpost, bastion).

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use bbb_server_types::save::{SaveError, SaveResult};

use super::{
    byte_bool, byte_field, compound, entry, field, float_list, int_field, long_field,
    resource_string, TemplateMobKind,
};

#[derive(Clone, Debug, PartialEq)]
pub enum JigsawExtras {
    Ageable {
        age: i32,
        forced_age: i32,
        age_locked: bool,
        in_love: i32,
        leaf: AgeableLeaf,
    },
    IronGolem {
        player_created: bool,
    },
    ArmorStand {
        invisible: bool,
        small: bool,
        show_arms: bool,
        disabled_slots: i32,
        no_base_plate: bool,
        marker: bool,
        pose: NbtSummaryValue,
    },
    Allay,
    Piglin {
        immune: bool,
        time_in_overworld: i32,
        is_baby: bool,
        cannot_hunt: bool,
    },
    PiglinBrute {
        immune: bool,
        time_in_overworld: i32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum AgeableLeaf {
    Cat {
        variant: String,
        sound_variant: String,
        collar_color: i8,
        sitting: bool,
    },
    VariantAnimal {
        variant: String,
        sound_variant: String,
    },
    Sheep {
        color: i8,
        sheared: bool,
    },
    Horse {
        eating: bool,
        bred: bool,
        temper: i32,
        tame: bool,
        variant: i32,
    },
    Camel {
        eating: bool,
        bred: bool,
        temper: i32,
        tame: bool,
        last_pose_tick: i64,
    },
    Hoglin {
        immune: bool,
        time_in_overworld: i32,
        cannot_be_hunted: bool,
    },
}

pub fn build(kind: TemplateMobKind, entries: &[NbtSummaryEntry]) -> SaveResult<JigsawExtras> {
    match kind {
        TemplateMobKind::Cat
        | TemplateMobKind::Cow
        | TemplateMobKind::Pig
        | TemplateMobKind::Sheep
        | TemplateMobKind::Horse
        | TemplateMobKind::Camel
        | TemplateMobKind::Hoglin => Ok(JigsawExtras::Ageable {
            age: int_field(entries, "Age").unwrap_or(0),
            forced_age: int_field(entries, "ForcedAge").unwrap_or(0),
            age_locked: byte_bool(entries, "AgeLocked").unwrap_or(false),
            in_love: int_field(entries, "InLove").unwrap_or(0),
            leaf: build_leaf(kind, entries)?,
        }),
        TemplateMobKind::IronGolem => Ok(JigsawExtras::IronGolem {
            player_created: byte_bool(entries, "PlayerCreated").unwrap_or(false),
        }),
        TemplateMobKind::ArmorStand => Ok(JigsawExtras::ArmorStand {
            invisible: byte_bool(entries, "Invisible").unwrap_or(false),
            small: byte_bool(entries, "Small").unwrap_or(false),
            show_arms: byte_bool(entries, "ShowArms").unwrap_or(false),
            disabled_slots: int_field(entries, "DisabledSlots").unwrap_or(0),
            no_base_plate: byte_bool(entries, "NoBasePlate").unwrap_or(false),
            marker: byte_bool(entries, "Marker").unwrap_or(false),
            pose: canonicalize_pose(field(entries, "Pose")).ok_or_else(|| {
                SaveError::InvalidNbt {
                    path: Some("minecraft:armor_stand.Pose".into()),
                    detail: "template pose is not ArmorStandPose.CODEC".to_string(),
                }
            })?,
        }),
        TemplateMobKind::Allay => Ok(JigsawExtras::Allay),
        TemplateMobKind::Piglin => Ok(JigsawExtras::Piglin {
            immune: byte_bool(entries, "IsImmuneToZombification").unwrap_or(false),
            time_in_overworld: int_field(entries, "TimeInOverworld").unwrap_or(0),
            is_baby: byte_bool(entries, "IsBaby").unwrap_or(false),
            cannot_hunt: byte_bool(entries, "CannotHunt").unwrap_or(false),
        }),
        TemplateMobKind::PiglinBrute => Ok(JigsawExtras::PiglinBrute {
            immune: byte_bool(entries, "IsImmuneToZombification").unwrap_or(false),
            time_in_overworld: int_field(entries, "TimeInOverworld").unwrap_or(0),
        }),
        TemplateMobKind::Villager | TemplateMobKind::ZombieVillager => {
            unreachable!("villager extras stay on the template owner")
        }
    }
}

pub fn build_leaf(kind: TemplateMobKind, entries: &[NbtSummaryEntry]) -> SaveResult<AgeableLeaf> {
    match kind {
        TemplateMobKind::Cat => Ok(AgeableLeaf::Cat {
            // `Cat.java:80-81,208-214`: default black / classic / red collar.
            variant: resource_string(entries, "variant")
                .unwrap_or_else(|| "minecraft:black".to_string()),
            sound_variant: resource_string(entries, "sound_variant")
                .unwrap_or_else(|| "minecraft:classic".to_string()),
            collar_color: {
                let color = byte_field(entries, "CollarColor").unwrap_or(14);
                if !(0..=15).contains(&color) {
                    return Err(SaveError::InvalidNbt {
                        path: Some("minecraft:cat.CollarColor".into()),
                        detail: "dye color id must be in 0..=15".to_string(),
                    });
                }
                color
            },
            sitting: byte_bool(entries, "Sitting").unwrap_or(false),
        }),
        TemplateMobKind::Cow | TemplateMobKind::Pig => Ok(AgeableLeaf::VariantAnimal {
            // `PigVariants.DEFAULT` / `CowVariants.DEFAULT` are temperate;
            // sound holders default to classic (`Pig.java:109-119`, `Cow.java:41-52`).
            variant: resource_string(entries, "variant")
                .unwrap_or_else(|| "minecraft:temperate".to_string()),
            sound_variant: resource_string(entries, "sound_variant")
                .unwrap_or_else(|| "minecraft:classic".to_string()),
        }),
        TemplateMobKind::Sheep => {
            let color = byte_field(entries, "Color").unwrap_or(0);
            if !(0..=15).contains(&color) {
                return Err(SaveError::InvalidNbt {
                    path: Some("minecraft:sheep.Color".into()),
                    detail: "dye color id must be in 0..=15".to_string(),
                });
            }
            Ok(AgeableLeaf::Sheep {
                color,
                sheared: byte_bool(entries, "Sheared").unwrap_or(false),
            })
        }
        TemplateMobKind::Horse => Ok(AgeableLeaf::Horse {
            eating: byte_bool(entries, "EatingHaystack").unwrap_or(false),
            bred: byte_bool(entries, "Bred").unwrap_or(false),
            temper: int_field(entries, "Temper").unwrap_or(0),
            tame: byte_bool(entries, "Tame").unwrap_or(false),
            variant: int_field(entries, "Variant").unwrap_or(0),
        }),
        TemplateMobKind::Camel => Ok(AgeableLeaf::Camel {
            eating: byte_bool(entries, "EatingHaystack").unwrap_or(false),
            bred: byte_bool(entries, "Bred").unwrap_or(false),
            temper: int_field(entries, "Temper").unwrap_or(0),
            tame: byte_bool(entries, "Tame").unwrap_or(false),
            last_pose_tick: long_field(entries, "LastPoseTick").unwrap_or(0),
        }),
        TemplateMobKind::Hoglin => Ok(AgeableLeaf::Hoglin {
            // `Hoglin.java:260-265`.
            immune: byte_bool(entries, "IsImmuneToZombification").unwrap_or(false),
            time_in_overworld: int_field(entries, "TimeInOverworld").unwrap_or(0),
            cannot_be_hunted: byte_bool(entries, "CannotBeHunted").unwrap_or(false),
        }),
        _ => unreachable!("ageable jigsaw leaf"),
    }
}

pub fn emit(extras: &JigsawExtras, entries: &mut Vec<NbtSummaryEntry>) {
    match extras {
        JigsawExtras::Ageable {
            age,
            forced_age,
            age_locked,
            in_love,
            leaf,
        } => {
            // `AgeableMob.java:163-166`, `Animal.java:97-99`.
            entries.extend([
                entry("Age", NbtSummaryValue::Int(*age)),
                entry("ForcedAge", NbtSummaryValue::Int(*forced_age)),
                entry("AgeLocked", NbtSummaryValue::Byte(i8::from(*age_locked))),
                entry("InLove", NbtSummaryValue::Int(*in_love)),
            ]);
            emit_leaf(leaf, entries);
        }
        JigsawExtras::IronGolem { player_created } => {
            // `IronGolem.java:144-147`, `NeutralMob.java:35`.
            entries.extend([
                entry(
                    "PlayerCreated",
                    NbtSummaryValue::Byte(i8::from(*player_created)),
                ),
                entry("anger_end_time", NbtSummaryValue::Long(0)),
            ]);
        }
        JigsawExtras::ArmorStand {
            invisible,
            small,
            show_arms,
            disabled_slots,
            no_base_plate,
            marker,
            pose,
        } => {
            // `ArmorStand.java:140-151`.
            entries.extend([
                entry("Invisible", NbtSummaryValue::Byte(i8::from(*invisible))),
                entry("Small", NbtSummaryValue::Byte(i8::from(*small))),
                entry("ShowArms", NbtSummaryValue::Byte(i8::from(*show_arms))),
                entry("DisabledSlots", NbtSummaryValue::Int(*disabled_slots)),
                entry(
                    "NoBasePlate",
                    NbtSummaryValue::Byte(i8::from(*no_base_plate)),
                ),
                entry("Pose", pose.clone()),
            ]);
            if *marker {
                entries.push(entry("Marker", NbtSummaryValue::Byte(1)));
            }
        }
        JigsawExtras::Allay => {
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
        JigsawExtras::Piglin {
            immune,
            time_in_overworld,
            is_baby,
            cannot_hunt,
        } => {
            // `AbstractPiglin.java:64-67`, `Piglin.java:106-110`.
            entries.extend([
                entry(
                    "IsImmuneToZombification",
                    NbtSummaryValue::Byte(i8::from(*immune)),
                ),
                entry("TimeInOverworld", NbtSummaryValue::Int(*time_in_overworld)),
                entry("IsBaby", NbtSummaryValue::Byte(i8::from(*is_baby))),
                entry("CannotHunt", NbtSummaryValue::Byte(i8::from(*cannot_hunt))),
                entry("Inventory", NbtSummaryValue::List(Vec::new())),
            ]);
        }
        JigsawExtras::PiglinBrute {
            immune,
            time_in_overworld,
        } => {
            // `AbstractPiglin.java:64-67`.
            entries.extend([
                entry(
                    "IsImmuneToZombification",
                    NbtSummaryValue::Byte(i8::from(*immune)),
                ),
                entry("TimeInOverworld", NbtSummaryValue::Int(*time_in_overworld)),
            ]);
        }
    }
}

pub fn emit_leaf(leaf: &AgeableLeaf, entries: &mut Vec<NbtSummaryEntry>) {
    match leaf {
        AgeableLeaf::Cat {
            variant,
            sound_variant,
            collar_color,
            sitting,
        } => {
            // `TamableAnimal.java:58-62`, `Cat.java:208-214`.
            entries.extend([
                entry("Sitting", NbtSummaryValue::Byte(i8::from(*sitting))),
                entry("variant", NbtSummaryValue::string(variant.clone())),
                entry(
                    "sound_variant",
                    NbtSummaryValue::string(sound_variant.clone()),
                ),
                entry("CollarColor", NbtSummaryValue::Byte(*collar_color)),
            ]);
        }
        AgeableLeaf::VariantAnimal {
            variant,
            sound_variant,
        } => {
            entries.extend([
                entry("variant", NbtSummaryValue::string(variant.clone())),
                entry(
                    "sound_variant",
                    NbtSummaryValue::string(sound_variant.clone()),
                ),
            ]);
        }
        AgeableLeaf::Sheep { color, sheared } => {
            // `Sheep.java:188-196`.
            entries.extend([
                entry("Sheared", NbtSummaryValue::Byte(i8::from(*sheared))),
                entry("Color", NbtSummaryValue::Byte(*color)),
            ]);
        }
        AgeableLeaf::Horse {
            eating,
            bred,
            temper,
            tame,
            variant,
        } => {
            // `AbstractHorse.java:788-794`, `Horse.java:65-67`.
            entries.extend([
                entry("EatingHaystack", NbtSummaryValue::Byte(i8::from(*eating))),
                entry("Bred", NbtSummaryValue::Byte(i8::from(*bred))),
                entry("Temper", NbtSummaryValue::Int(*temper)),
                entry("Tame", NbtSummaryValue::Byte(i8::from(*tame))),
                entry("Variant", NbtSummaryValue::Int(*variant)),
            ]);
        }
        AgeableLeaf::Camel {
            eating,
            bred,
            temper,
            tame,
            last_pose_tick,
        } => {
            // `AbstractHorse.java:788-794`, `Camel.java:99-101`.
            entries.extend([
                entry("EatingHaystack", NbtSummaryValue::Byte(i8::from(*eating))),
                entry("Bred", NbtSummaryValue::Byte(i8::from(*bred))),
                entry("Temper", NbtSummaryValue::Int(*temper)),
                entry("Tame", NbtSummaryValue::Byte(i8::from(*tame))),
                entry("LastPoseTick", NbtSummaryValue::Long(*last_pose_tick)),
            ]);
        }
        AgeableLeaf::Hoglin {
            immune,
            time_in_overworld,
            cannot_be_hunted,
        } => {
            // `Hoglin.java:260-265`.
            entries.extend([
                entry(
                    "IsImmuneToZombification",
                    NbtSummaryValue::Byte(i8::from(*immune)),
                ),
                entry("TimeInOverworld", NbtSummaryValue::Int(*time_in_overworld)),
                entry(
                    "CannotBeHunted",
                    NbtSummaryValue::Byte(i8::from(*cannot_be_hunted)),
                ),
            ]);
        }
    }
}

/// Writer-canonical `ArmorStandPose.CODEC` omits default members
/// (`ArmorStand.java:151`).
pub fn canonicalize_pose(value: Option<&NbtSummaryValue>) -> Option<NbtSummaryValue> {
    let Some(value) = value else {
        return Some(NbtSummaryValue::Compound(Vec::new()));
    };
    let pose = compound(value)?;
    const DEFAULTS: [(&str, [f32; 3]); 6] = [
        ("Head", [0.0, 0.0, 0.0]),
        ("Body", [0.0, 0.0, 0.0]),
        ("LeftArm", [-10.0, 0.0, -10.0]),
        ("RightArm", [-15.0, 0.0, 10.0]),
        ("LeftLeg", [-1.0, 0.0, -1.0]),
        ("RightLeg", [1.0, 0.0, 1.0]),
    ];
    let mut kept = Vec::new();
    for member in pose {
        let Some((_, default)) = DEFAULTS.iter().find(|(name, _)| member.name == *name) else {
            kept.push(member.clone());
            continue;
        };
        let rotation = float_list(&member.value, 3)?;
        let actual = [rotation[0], rotation[1], rotation[2]];
        if actual.map(f32::to_bits) != default.map(f32::to_bits) {
            kept.push(member.clone());
        }
    }
    Some(NbtSummaryValue::Compound(kept))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::proto_chunk_entity::TemplateMobRecord;

    fn id_only(id: &str) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![entry("id", NbtSummaryValue::string(id))])
    }

    fn record(nbt: &NbtSummaryValue) -> TemplateMobRecord {
        TemplateMobRecord::from_template_nbt(nbt, [0.5, 64.0, 0.5], [90.0, 0.0], [1, 2, 3, 4])
            .unwrap()
            .unwrap()
    }

    #[test]
    fn camel_id_only_emits_create_defaults() {
        let entries = record(&id_only("minecraft:camel")).entries();
        let last_pose = entries
            .iter()
            .find(|entry| entry.name == "LastPoseTick")
            .map(|entry| &entry.value);
        assert_eq!(last_pose, Some(&NbtSummaryValue::Long(0)));
        let variant = entries.iter().find(|entry| entry.name == "Variant");
        assert!(variant.is_none());
    }

    /// Mob.java:392: a template mob without the flag may despawn.
    #[test]
    fn a_template_mob_without_persistence_required_is_not_persistent() {
        let entries = record(&id_only("minecraft:camel")).entries();
        let persistent = entries
            .iter()
            .find(|entry| entry.name == "PersistenceRequired")
            .map(|entry| &entry.value);
        assert_eq!(persistent, Some(&NbtSummaryValue::Byte(0)));
    }

    #[test]
    fn built_in_template_ids_are_owned() {
        use bbb_server_datapack::pack::ServerPackResources;
        use bbb_server_datapack::structure_template::parse_structure_template;
        use bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot;

        let snapshot = std::sync::Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
        let vanilla = snapshot.vanilla_pack();
        let mut owned = 0usize;
        for path in vanilla.list_paths("minecraft", "structure") {
            let bytes = vanilla.read("minecraft", &path).unwrap();
            let template = parse_structure_template(&bytes)
                .unwrap_or_else(|error| panic!("{path} must parse: {error}"));
            for entity in &template.materialized().entities {
                let Some(id) = entity_id(&entity.nbt) else {
                    continue;
                };
                assert!(
                    TemplateMobRecord::owns_resource_id(&id),
                    "{path} built-in template id {id} must be owned"
                );
                owned += 1;
            }
        }
        assert_eq!(owned, 64);
    }

    fn entity_id(nbt: &NbtSummaryValue) -> Option<String> {
        let entries = compound(nbt)?;
        resource_string(entries, "id")
    }

    #[test]
    fn template_equipment_is_kept() {
        let sword = NbtSummaryValue::Compound(vec![
            entry("count", NbtSummaryValue::Int(1)),
            entry("id", NbtSummaryValue::string("minecraft:golden_sword")),
        ]);
        let nbt = NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("minecraft:piglin")),
            entry(
                "equipment",
                NbtSummaryValue::Compound(vec![entry("mainhand", sword.clone())]),
            ),
        ]);
        let entries = record(&nbt).entries();
        assert_eq!(
            field(&entries, "equipment"),
            Some(&NbtSummaryValue::Compound(vec![entry("mainhand", sword)]))
        );
        assert!(field(&record(&id_only("minecraft:piglin")).entries(), "equipment").is_none());
    }

    #[test]
    fn armor_stand_omits_mob_fields_and_default_pose() {
        let entries = record(&id_only("minecraft:armor_stand")).entries();
        let names: Vec<&bbb_protocol::JavaString> =
            entries.iter().map(|entry| &entry.name).collect();
        assert!(!names.iter().any(|name| **name == "PersistenceRequired"));
        assert!(!names.iter().any(|name| **name == "CanPickUpLoot"));
        assert!(!names.iter().any(|name| **name == "LeftHanded"));
        let pose = entries
            .iter()
            .find(|entry| entry.name == "Pose")
            .map(|entry| &entry.value);
        assert_eq!(pose, Some(&NbtSummaryValue::Compound(Vec::new())));
    }
}
