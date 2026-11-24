//! Entity NBT fixtures shared by the world-storage publication tests.

use super::Java26LoadedEntityChunk;
use crate::Java26EntityChunkData;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

pub fn nbt_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

pub fn uuid_nbt(uuid: Uuid) -> NbtSummaryValue {
    let value = uuid.as_u128();
    NbtSummaryValue::IntArray(vec![
        (value >> 96) as i32,
        (value >> 64) as i32,
        (value >> 32) as i32,
        value as i32,
    ])
}

pub fn entity_nbt(resource_id: &str, uuid: Uuid, x: f64) -> Vec<NbtSummaryEntry> {
    vec![
        nbt_entry("id", NbtSummaryValue::string(resource_id)),
        nbt_entry("UUID", uuid_nbt(uuid)),
        nbt_entry(
            "Pos",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Double(x.to_bits()),
                NbtSummaryValue::Double(64.0f64.to_bits()),
                NbtSummaryValue::Double(0.5f64.to_bits()),
            ]),
        ),
        nbt_entry(
            "Motion",
            NbtSummaryValue::List(vec![NbtSummaryValue::Double(0); 3]),
        ),
        nbt_entry(
            "Rotation",
            NbtSummaryValue::List(vec![NbtSummaryValue::Float(0); 2]),
        ),
        nbt_entry("fall_distance", NbtSummaryValue::Double(0)),
        nbt_entry("Fire", NbtSummaryValue::Short(-20)),
        nbt_entry("Air", NbtSummaryValue::Short(300)),
        nbt_entry("OnGround", NbtSummaryValue::Byte(0)),
        nbt_entry("Invulnerable", NbtSummaryValue::Byte(0)),
        nbt_entry("PortalCooldown", NbtSummaryValue::Int(0)),
    ]
}

pub fn entity_chunk(chunk_x: i32, resource_id: &str, uuid: Uuid) -> Java26LoadedEntityChunk {
    let x = f64::from(chunk_x * 16) + 0.5;
    Java26LoadedEntityChunk {
        timestamp: 0,
        data: Java26EntityChunkData::parse(
            crate::NamedCompound::new(
                "",
                vec![
                    nbt_entry(
                        "DataVersion",
                        NbtSummaryValue::Int(bbb_server_types::save::JAVA_26_1_DATA_VERSION),
                    ),
                    nbt_entry("Position", NbtSummaryValue::IntArray(vec![chunk_x, 0])),
                    nbt_entry(
                        "Entities",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entity_nbt(
                            resource_id,
                            uuid,
                            x,
                        ))]),
                    ),
                ],
            ),
            chunk_x,
            0,
        )
        .unwrap(),
    }
}

pub fn item_entity_chunk(uuid: Uuid, component: NbtSummaryEntry) -> Java26LoadedEntityChunk {
    let mut item = entity_nbt("minecraft:item", uuid, 0.5);
    item.extend([
        nbt_entry("Health", NbtSummaryValue::Short(5)),
        nbt_entry("Age", NbtSummaryValue::Short(i16::MIN)),
        nbt_entry("PickupDelay", NbtSummaryValue::Short(i16::MAX)),
        nbt_entry("Thrower", uuid_nbt(Uuid::from_u128(8))),
        nbt_entry("Owner", uuid_nbt(Uuid::from_u128(9))),
        nbt_entry(
            "Item",
            NbtSummaryValue::Compound(vec![
                nbt_entry("id", NbtSummaryValue::string("minecraft:diamond")),
                nbt_entry("count", NbtSummaryValue::Int(1)),
                nbt_entry("components", NbtSummaryValue::Compound(vec![component])),
            ]),
        ),
    ]);
    Java26LoadedEntityChunk {
        timestamp: 0,
        data: Java26EntityChunkData::parse(
            crate::NamedCompound::new(
                "",
                vec![
                    nbt_entry(
                        "DataVersion",
                        NbtSummaryValue::Int(bbb_server_types::save::JAVA_26_1_DATA_VERSION),
                    ),
                    nbt_entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                    nbt_entry(
                        "Entities",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(item)]),
                    ),
                ],
            ),
            0,
            0,
        )
        .unwrap(),
    }
}

fn living_entity_entries(id: &str, uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = entity_nbt(id, uuid, 0.5);
    entity.extend([
        nbt_entry("Health", NbtSummaryValue::Float(20.0_f32.to_bits())),
        nbt_entry("HurtTime", NbtSummaryValue::Short(0)),
        nbt_entry("HurtByTimestamp", NbtSummaryValue::Int(0)),
        nbt_entry("DeathTime", NbtSummaryValue::Short(0)),
        nbt_entry(
            "AbsorptionAmount",
            NbtSummaryValue::Float(0.0_f32.to_bits()),
        ),
        nbt_entry(
            "current_impulse_context_reset_grace_time",
            NbtSummaryValue::Int(0),
        ),
        nbt_entry("attributes", NbtSummaryValue::List(Vec::new())),
        nbt_entry("FallFlying", NbtSummaryValue::Byte(0)),
        nbt_entry(
            "Brain",
            NbtSummaryValue::Compound(vec![nbt_entry(
                "memories",
                NbtSummaryValue::Compound(Vec::new()),
            )]),
        ),
    ]);
    entity
}

pub fn end_crystal_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = entity_nbt("minecraft:end_crystal", uuid, 0.5);
    entity.extend([
        nbt_entry(
            "beam_target",
            NbtSummaryValue::IntArray(vec![i32::MIN, 0, i32::MAX]),
        ),
        nbt_entry("ShowBottom", NbtSummaryValue::Byte(1)),
    ]);
    entity
}

pub fn evoker_fangs_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = entity_nbt("minecraft:evoker_fangs", uuid, 0.5);
    entity.extend([
        nbt_entry("Warmup", NbtSummaryValue::Int(i32::MIN)),
        nbt_entry("Owner", uuid_nbt(Uuid::from_u128(101))),
    ]);
    entity
}

pub fn painting_entity_entries(uuid: Uuid, facing: i8, variant: &str) -> Vec<NbtSummaryEntry> {
    let mut entity = entity_nbt("minecraft:painting", uuid, 0.5);
    entity.extend([
        nbt_entry("block_pos", NbtSummaryValue::IntArray(vec![0, 64, 0])),
        nbt_entry("facing", NbtSummaryValue::Byte(facing)),
        nbt_entry("variant", NbtSummaryValue::string(variant)),
        nbt_entry(
            "FuturePaintingPayload",
            NbtSummaryValue::Compound(Vec::new()),
        ),
    ]);
    entity
}

pub fn riding_painting_entity_entries(
    vehicle_uuid: Uuid,
    painting_uuid: Uuid,
) -> Vec<NbtSummaryEntry> {
    let mut painting = painting_entity_entries(painting_uuid, 0, "minecraft:kebab");
    let NbtSummaryValue::List(position) = &mut painting
        .iter_mut()
        .find(|entry| entry.name == "Pos")
        .unwrap()
        .value
    else {
        unreachable!("painting fixture has a position list");
    };
    position[0] = NbtSummaryValue::Double(15.5_f64.to_bits());

    let mut vehicle = entity_nbt("minecraft:experience_orb", vehicle_uuid, 15.5);
    vehicle.extend([
        nbt_entry("Health", NbtSummaryValue::Short(-1)),
        nbt_entry("Age", NbtSummaryValue::Short(i16::MAX)),
        nbt_entry("Value", NbtSummaryValue::Short(i16::MIN)),
        nbt_entry("Count", NbtSummaryValue::Int(1)),
        nbt_entry(
            "Passengers",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(painting)]),
        ),
    ]);
    vehicle
}

fn living_mob_entity_entries(
    id: &str,
    uuid: Uuid,
    include_left_handed: bool,
) -> Vec<NbtSummaryEntry> {
    let mut entity = living_entity_entries(id, uuid);
    entity.extend([
        nbt_entry("CanPickUpLoot", NbtSummaryValue::Byte(0)),
        nbt_entry("PersistenceRequired", NbtSummaryValue::Byte(0)),
    ]);
    if include_left_handed {
        entity.push(nbt_entry("LeftHanded", NbtSummaryValue::Byte(0)));
    }
    entity
}

fn vibration_info(game_event: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        nbt_entry("game_event", NbtSummaryValue::string(game_event)),
        nbt_entry("distance", NbtSummaryValue::Float(0.0_f32.to_bits())),
        nbt_entry(
            "pos",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Double(f64::NAN.to_bits()),
                NbtSummaryValue::Double(f64::INFINITY.to_bits()),
                NbtSummaryValue::Double(f64::NEG_INFINITY.to_bits()),
            ]),
        ),
        nbt_entry("source", uuid_nbt(Uuid::from_u128(410))),
        nbt_entry("projectile_owner", uuid_nbt(Uuid::from_u128(411))),
    ])
}

fn vibration_listener(game_event: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        nbt_entry("event", vibration_info(game_event)),
        nbt_entry(
            "selector",
            NbtSummaryValue::Compound(vec![
                nbt_entry("event", vibration_info("minecraft:step")),
                nbt_entry("tick", NbtSummaryValue::Long(i64::MAX)),
            ]),
        ),
        nbt_entry("event_delay", NbtSummaryValue::Int(i32::MAX)),
    ])
}

pub fn allay_warden_entity_entries(id: &str, uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries(id, uuid, true);
    entity.push(nbt_entry("listener", vibration_listener("minecraft:step")));
    match id {
        "minecraft:allay" => {
            entity.extend([
                nbt_entry("Inventory", NbtSummaryValue::List(Vec::new())),
                nbt_entry(
                    "DuplicationCooldown",
                    NbtSummaryValue::Long(i32::MIN.into()),
                ),
            ]);
            entity
                .iter_mut()
                .find(|entry| entry.name == "Brain")
                .unwrap()
                .value = NbtSummaryValue::Compound(vec![nbt_entry(
                "memories",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "minecraft:liked_player",
                    NbtSummaryValue::Compound(vec![nbt_entry(
                        "value",
                        uuid_nbt(Uuid::from_u128(412)),
                    )]),
                )]),
            )]);
        }
        "minecraft:warden" => {
            entity.push(nbt_entry(
                "anger",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "suspects",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        nbt_entry("uuid", uuid_nbt(Uuid::from_u128(413))),
                        nbt_entry("anger", NbtSummaryValue::Int(i32::MAX)),
                    ])]),
                )]),
            ));
            entity
                .iter_mut()
                .find(|entry| entry.name == "Brain")
                .unwrap()
                .value = NbtSummaryValue::Compound(vec![nbt_entry(
                "memories",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "minecraft:vibration_cooldown",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry("value", NbtSummaryValue::Compound(Vec::new())),
                        nbt_entry("ttl", NbtSummaryValue::Long(-1)),
                    ]),
                )]),
            )]);
        }
        _ => unreachable!("allay/warden fixture id"),
    }
    entity
}

pub fn area_effect_cloud_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = entity_nbt("minecraft:area_effect_cloud", uuid, 0.5);
    entity.extend([
        nbt_entry("Age", NbtSummaryValue::Int(0)),
        nbt_entry("Duration", NbtSummaryValue::Int(-1)),
        nbt_entry("WaitTime", NbtSummaryValue::Int(20)),
        nbt_entry("ReapplicationDelay", NbtSummaryValue::Int(20)),
        nbt_entry("DurationOnUse", NbtSummaryValue::Int(0)),
        nbt_entry("RadiusOnUse", NbtSummaryValue::Float(0.0_f32.to_bits())),
        nbt_entry("RadiusPerTick", NbtSummaryValue::Float(0.0_f32.to_bits())),
        nbt_entry("Radius", NbtSummaryValue::Float(3.0_f32.to_bits())),
        nbt_entry("Owner", uuid_nbt(Uuid::from_u128(430))),
        nbt_entry(
            "custom_particle",
            NbtSummaryValue::Compound(vec![
                nbt_entry("type", NbtSummaryValue::string("minecraft:item")),
                nbt_entry(
                    "item",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry("id", NbtSummaryValue::string("minecraft:stick")),
                        nbt_entry(
                            "components",
                            NbtSummaryValue::Compound(vec![nbt_entry(
                                "minecraft:enchantments",
                                NbtSummaryValue::Compound(vec![nbt_entry(
                                    "minecraft:sharpness",
                                    NbtSummaryValue::Int(1),
                                )]),
                            )]),
                        ),
                    ]),
                ),
            ]),
        ),
        nbt_entry(
            "potion_contents",
            NbtSummaryValue::Compound(vec![nbt_entry(
                "custom_effects",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    nbt_entry("id", NbtSummaryValue::string("minecraft:speed")),
                    nbt_entry("show_icon", NbtSummaryValue::Byte(1)),
                ])]),
            )]),
        ),
        nbt_entry(
            "potion_duration_scale",
            NbtSummaryValue::Float(0.5_f32.to_bits()),
        ),
    ]);
    entity
}

pub fn fixed_living_mob_entity_entries(id: &str, uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries(id, uuid, true);
    if matches!(
        id,
        "minecraft:dolphin"
            | "minecraft:glow_squid"
            | "minecraft:hoglin"
            | "minecraft:squid"
            | "minecraft:strider"
    ) {
        entity.extend([
            nbt_entry("Age", NbtSummaryValue::Int(0)),
            nbt_entry("ForcedAge", NbtSummaryValue::Int(0)),
            nbt_entry("AgeLocked", NbtSummaryValue::Byte(0)),
        ]);
    }
    if matches!(id, "minecraft:hoglin" | "minecraft:strider") {
        entity.push(nbt_entry("InLove", NbtSummaryValue::Int(0)));
    }
    match id {
        "minecraft:bat" => entity.push(nbt_entry("BatFlags", NbtSummaryValue::Byte(i8::MIN))),
        "minecraft:dolphin" => entity.extend([
            nbt_entry("GotFish", NbtSummaryValue::Byte(1)),
            nbt_entry("Moistness", NbtSummaryValue::Int(i32::MIN)),
        ]),
        "minecraft:endermite" => entity.push(nbt_entry("Lifetime", NbtSummaryValue::Int(i32::MAX))),
        "minecraft:ender_dragon" => entity.extend([
            nbt_entry("DragonPhase", NbtSummaryValue::Int(10)),
            nbt_entry("DragonDeathTime", NbtSummaryValue::Int(i32::MIN)),
        ]),
        "minecraft:ghast" => {
            entity.push(nbt_entry("ExplosionPower", NbtSummaryValue::Byte(i8::MAX)))
        }
        "minecraft:glow_squid" => entity.push(nbt_entry(
            "DarkTicksRemaining",
            NbtSummaryValue::Int(i32::MIN),
        )),
        "minecraft:hoglin" => {
            entity.extend([
                nbt_entry("IsImmuneToZombification", NbtSummaryValue::Byte(0)),
                nbt_entry("TimeInOverworld", NbtSummaryValue::Int(i32::MAX)),
                nbt_entry("CannotBeHunted", NbtSummaryValue::Byte(1)),
            ]);
            entity
                .iter_mut()
                .find(|entry| entry.name == "Brain")
                .unwrap()
                .value = NbtSummaryValue::Compound(vec![nbt_entry(
                "memories",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "minecraft:is_panicking",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry("value", NbtSummaryValue::Byte(1)),
                        nbt_entry("ttl", NbtSummaryValue::Long(-1)),
                    ]),
                )]),
            )]);
        }
        "minecraft:shulker" => entity.extend([
            nbt_entry("AttachFace", NbtSummaryValue::Byte(5)),
            nbt_entry("Peek", NbtSummaryValue::Byte(i8::MIN)),
            nbt_entry("Color", NbtSummaryValue::Byte(i8::MAX)),
        ]),
        "minecraft:snow_golem" => entity.push(nbt_entry("Pumpkin", NbtSummaryValue::Byte(1))),
        "minecraft:squid" | "minecraft:strider" => {}
        "minecraft:wither" => entity.push(nbt_entry("Invul", NbtSummaryValue::Int(i32::MIN))),
        "minecraft:zoglin" => entity.push(nbt_entry("IsBaby", NbtSummaryValue::Byte(0))),
        _ => unreachable!("fixed living/mob fixture id"),
    }
    entity
}

pub fn armor_stand_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_entity_entries("minecraft:armor_stand", uuid);
    entity.extend([
        nbt_entry("Invisible", NbtSummaryValue::Byte(0)),
        nbt_entry("Small", NbtSummaryValue::Byte(0)),
        nbt_entry("ShowArms", NbtSummaryValue::Byte(0)),
        nbt_entry("DisabledSlots", NbtSummaryValue::Int(0)),
        nbt_entry("NoBasePlate", NbtSummaryValue::Byte(0)),
        nbt_entry("Pose", NbtSummaryValue::Compound(Vec::new())),
    ]);
    entity
}

pub fn rotations(values: [f32; 3]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .into_iter()
            .map(|value| NbtSummaryValue::Float(value.to_bits()))
            .collect(),
    )
}

pub fn loaded_entity_chunk(entity: Vec<NbtSummaryEntry>) -> Java26LoadedEntityChunk {
    Java26LoadedEntityChunk {
        timestamp: 0,
        data: Java26EntityChunkData::parse(
            crate::NamedCompound::new(
                "",
                vec![
                    nbt_entry(
                        "DataVersion",
                        NbtSummaryValue::Int(bbb_server_types::save::JAVA_26_1_DATA_VERSION),
                    ),
                    nbt_entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                    nbt_entry(
                        "Entities",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entity)]),
                    ),
                ],
            ),
            0,
            0,
        )
        .unwrap(),
    }
}

pub fn living_mob_entity_chunk(uuid: Uuid, include_left_handed: bool) -> Java26LoadedEntityChunk {
    loaded_entity_chunk(living_mob_entity_entries(
        "minecraft:blaze",
        uuid,
        include_left_handed,
    ))
}

pub fn parrot_entity_entries(uuid: Uuid, variant: i32) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:parrot", uuid, true);
    entity.extend([
        nbt_entry("Age", NbtSummaryValue::Int(0)),
        nbt_entry("ForcedAge", NbtSummaryValue::Int(0)),
        nbt_entry("AgeLocked", NbtSummaryValue::Byte(0)),
        nbt_entry("InLove", NbtSummaryValue::Int(0)),
        nbt_entry("Sitting", NbtSummaryValue::Byte(0)),
        nbt_entry("Variant", NbtSummaryValue::Int(variant)),
    ]);
    entity
}

pub fn abstract_horse_entity_chunk(uuid: Uuid, strength: i32) -> Java26LoadedEntityChunk {
    let mut entity = living_mob_entity_entries("minecraft:trader_llama", uuid, true);
    entity.extend([
        nbt_entry("Age", NbtSummaryValue::Int(i32::MIN)),
        nbt_entry("ForcedAge", NbtSummaryValue::Int(i32::MAX)),
        nbt_entry("AgeLocked", NbtSummaryValue::Byte(0)),
        nbt_entry("InLove", NbtSummaryValue::Int(i32::MIN)),
        nbt_entry("EatingHaystack", NbtSummaryValue::Byte(0)),
        nbt_entry("Bred", NbtSummaryValue::Byte(1)),
        nbt_entry("Temper", NbtSummaryValue::Int(i32::MAX)),
        nbt_entry("Tame", NbtSummaryValue::Byte(1)),
        nbt_entry("ChestedHorse", NbtSummaryValue::Byte(1)),
        nbt_entry("Items", NbtSummaryValue::List(Vec::new())),
        nbt_entry("Variant", NbtSummaryValue::Int(3)),
        nbt_entry("Strength", NbtSummaryValue::Int(strength)),
        nbt_entry("DespawnDelay", NbtSummaryValue::Int(i32::MAX)),
    ]);
    loaded_entity_chunk(entity)
}

pub fn tamable_nautilus_entity_chunk(uuid: Uuid, variant: &str) -> Java26LoadedEntityChunk {
    let mut entity = living_mob_entity_entries("minecraft:zombie_nautilus", uuid, true);
    entity.extend([
        nbt_entry("Age", NbtSummaryValue::Int(i32::MIN)),
        nbt_entry("ForcedAge", NbtSummaryValue::Int(i32::MAX)),
        nbt_entry("AgeLocked", NbtSummaryValue::Byte(0)),
        nbt_entry("InLove", NbtSummaryValue::Int(0)),
        nbt_entry("Sitting", NbtSummaryValue::Byte(0)),
        nbt_entry("variant", NbtSummaryValue::string(variant)),
    ]);
    entity
        .iter_mut()
        .find(|entry| entry.name == "Brain")
        .unwrap()
        .value = NbtSummaryValue::Compound(vec![nbt_entry(
        "memories",
        NbtSummaryValue::Compound(vec![nbt_entry(
            "minecraft:angry_at",
            NbtSummaryValue::Compound(vec![nbt_entry("value", uuid_nbt(Uuid::from_u128(10)))]),
        )]),
    )]);
    loaded_entity_chunk(entity)
}

pub fn abstract_fish_entity_chunk(uuid: Uuid, age_locked: i8) -> Java26LoadedEntityChunk {
    let mut entity = living_mob_entity_entries("minecraft:tadpole", uuid, true);
    entity.extend([
        nbt_entry("FromBucket", NbtSummaryValue::Byte(1)),
        nbt_entry("Age", NbtSummaryValue::Int(23_999)),
        nbt_entry("AgeLocked", NbtSummaryValue::Byte(age_locked)),
    ]);
    entity
        .iter_mut()
        .find(|entry| entry.name == "Brain")
        .unwrap()
        .value = NbtSummaryValue::Compound(vec![nbt_entry(
        "memories",
        NbtSummaryValue::Compound(vec![nbt_entry(
            "minecraft:is_panicking",
            NbtSummaryValue::Compound(vec![nbt_entry("value", NbtSummaryValue::Byte(1))]),
        )]),
    )]);
    loaded_entity_chunk(entity)
}

pub fn piglin_brute_entity_chunk(uuid: Uuid, immune: i8) -> Java26LoadedEntityChunk {
    let mut entity = living_mob_entity_entries("minecraft:piglin_brute", uuid, true);
    entity.extend([
        nbt_entry("IsImmuneToZombification", NbtSummaryValue::Byte(immune)),
        nbt_entry("TimeInOverworld", NbtSummaryValue::Int(i32::MAX)),
    ]);
    entity
        .iter_mut()
        .find(|entry| entry.name == "Brain")
        .unwrap()
        .value = NbtSummaryValue::Compound(vec![nbt_entry(
        "memories",
        NbtSummaryValue::Compound(vec![
            nbt_entry(
                "minecraft:home",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "value",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry("dimension", NbtSummaryValue::string("minecraft:overworld")),
                        nbt_entry("pos", NbtSummaryValue::IntArray(vec![0, -64, 0])),
                    ]),
                )]),
            ),
            nbt_entry(
                "minecraft:angry_at",
                NbtSummaryValue::Compound(vec![
                    nbt_entry("value", uuid_nbt(Uuid::from_u128(11))),
                    nbt_entry("ttl", NbtSummaryValue::Long(600)),
                ]),
            ),
        ]),
    )]);
    loaded_entity_chunk(entity)
}

pub fn slime_entity_chunk(id: &str, uuid: Uuid, size: i32) -> Java26LoadedEntityChunk {
    let mut entity = living_mob_entity_entries(id, uuid, true);
    entity.extend([
        nbt_entry("Size", NbtSummaryValue::Int(size)),
        nbt_entry("wasOnGround", NbtSummaryValue::Byte(1)),
    ]);
    loaded_entity_chunk(entity)
}

pub fn piglin_entity_entries(uuid: Uuid, cannot_hunt: i8) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:piglin", uuid, true);
    entity.extend([
        nbt_entry("IsImmuneToZombification", NbtSummaryValue::Byte(0)),
        nbt_entry("TimeInOverworld", NbtSummaryValue::Int(i32::MAX)),
        nbt_entry("IsBaby", NbtSummaryValue::Byte(1)),
        nbt_entry("CannotHunt", NbtSummaryValue::Byte(cannot_hunt)),
        nbt_entry(
            "Inventory",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                nbt_entry("id", NbtSummaryValue::string("minecraft:gold_ingot")),
                nbt_entry("count", NbtSummaryValue::Int(3)),
            ])]),
        ),
    ]);
    entity
        .iter_mut()
        .find(|entry| entry.name == "Brain")
        .unwrap()
        .value = NbtSummaryValue::Compound(vec![nbt_entry(
        "memories",
        NbtSummaryValue::Compound(vec![nbt_entry(
            "minecraft:admiring_item",
            NbtSummaryValue::Compound(vec![
                nbt_entry("value", NbtSummaryValue::Byte(1)),
                nbt_entry("ttl", NbtSummaryValue::Long(600)),
            ]),
        )]),
    )]);
    entity
}

pub fn mooshroom_entity_entries(uuid: Uuid, variant: &str) -> Vec<NbtSummaryEntry> {
    let mut entity = animal_variant_entity_entries("minecraft:mooshroom", uuid);
    entity.extend([
        nbt_entry("Type", NbtSummaryValue::string(variant)),
        nbt_entry(
            "stew_effects",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                nbt_entry("id", NbtSummaryValue::string("minecraft:poison")),
                nbt_entry("duration", NbtSummaryValue::Int(7)),
            ])]),
        ),
    ]);
    entity
}

pub fn copper_golem_entity_entries(uuid: Uuid, weather_state: &str) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:copper_golem", uuid, true);
    entity.extend([
        nbt_entry("next_weather_age", NbtSummaryValue::Long(-1)),
        nbt_entry("weather_state", NbtSummaryValue::string(weather_state)),
    ]);
    entity
        .iter_mut()
        .find(|entry| entry.name == "Brain")
        .unwrap()
        .value = NbtSummaryValue::Compound(vec![nbt_entry(
        "memories",
        NbtSummaryValue::Compound(vec![nbt_entry(
            "minecraft:gaze_cooldown_ticks",
            NbtSummaryValue::Compound(vec![nbt_entry("value", NbtSummaryValue::Int(i32::MAX))]),
        )]),
    )]);
    entity
}

pub fn creaking_entity_entries(uuid: Uuid, home_pos: NbtSummaryValue) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:creaking", uuid, true);
    entity.extend([
        nbt_entry("home_pos", home_pos),
        nbt_entry("home_radius", NbtSummaryValue::Int(16)),
    ]);
    entity
}

pub fn mannequin_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_entity_entries("minecraft:mannequin", uuid);
    entity.extend([
        nbt_entry(
            "profile",
            NbtSummaryValue::Compound(vec![nbt_entry("name", NbtSummaryValue::string("Alex"))]),
        ),
        nbt_entry("hidden_layers", NbtSummaryValue::List(Vec::new())),
        nbt_entry("main_hand", NbtSummaryValue::string("left")),
        nbt_entry("pose", NbtSummaryValue::string("crouching")),
        nbt_entry("immovable", NbtSummaryValue::Byte(1)),
        nbt_entry("description", NbtSummaryValue::string("greeter")),
    ]);
    entity
}

fn zombie_entity_entries(id: &str, uuid: Uuid, is_baby: i8) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries(id, uuid, true);
    entity.extend([
        nbt_entry("IsBaby", NbtSummaryValue::Byte(is_baby)),
        nbt_entry("CanBreakDoors", NbtSummaryValue::Byte(1)),
        nbt_entry("InWaterTime", NbtSummaryValue::Int(i32::MIN)),
        nbt_entry("DrownedConversionTime", NbtSummaryValue::Int(i32::MAX)),
    ]);
    entity
}

fn villager_data(profession: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        nbt_entry("type", NbtSummaryValue::string("minecraft:plains")),
        nbt_entry("profession", NbtSummaryValue::string(profession)),
        nbt_entry("level", NbtSummaryValue::Int(1)),
    ])
}

fn villager_offers(component_id: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![nbt_entry(
        "Recipes",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            nbt_entry(
                "buy",
                NbtSummaryValue::Compound(vec![
                    nbt_entry("id", NbtSummaryValue::string("minecraft:emerald")),
                    nbt_entry("count", NbtSummaryValue::Int(1)),
                    nbt_entry(
                        "components",
                        NbtSummaryValue::Compound(vec![nbt_entry(
                            component_id,
                            if component_id == "minecraft:max_stack_size" {
                                NbtSummaryValue::Int(16)
                            } else {
                                NbtSummaryValue::string("blocked")
                            },
                        )]),
                    ),
                ]),
            ),
            nbt_entry(
                "sell",
                NbtSummaryValue::Compound(vec![
                    nbt_entry("id", NbtSummaryValue::string("minecraft:diamond")),
                    nbt_entry("count", NbtSummaryValue::Int(1)),
                ]),
            ),
        ])]),
    )])
}

pub fn villager_merchant_entity_entries(
    id: &str,
    uuid: Uuid,
    profession: &str,
    component_id: &str,
) -> Vec<NbtSummaryEntry> {
    let mut entity = if id == "minecraft:zombie_villager" {
        zombie_entity_entries(id, uuid, 0)
    } else {
        let mut entity = living_mob_entity_entries(id, uuid, true);
        entity.extend([
            nbt_entry("Age", NbtSummaryValue::Int(0)),
            nbt_entry("ForcedAge", NbtSummaryValue::Int(0)),
            nbt_entry("AgeLocked", NbtSummaryValue::Byte(0)),
            nbt_entry("Inventory", NbtSummaryValue::List(Vec::new())),
        ]);
        entity
    };
    match id {
        "minecraft:villager" => {
            entity.extend([
                nbt_entry("VillagerData", villager_data(profession)),
                nbt_entry("VillagerDataFinalized", NbtSummaryValue::Byte(1)),
                nbt_entry("Offers", villager_offers(component_id)),
                nbt_entry("FoodLevel", NbtSummaryValue::Byte(0)),
                nbt_entry("Gossips", NbtSummaryValue::List(Vec::new())),
                nbt_entry("Xp", NbtSummaryValue::Int(0)),
                nbt_entry("LastRestock", NbtSummaryValue::Long(0)),
                nbt_entry("LastGossipDecay", NbtSummaryValue::Long(0)),
                nbt_entry("RestocksToday", NbtSummaryValue::Int(0)),
            ]);
            entity
                .iter_mut()
                .find(|entry| entry.name == "Brain")
                .unwrap()
                .value = NbtSummaryValue::Compound(vec![nbt_entry(
                "memories",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "minecraft:item_pickup_cooldown_ticks",
                    NbtSummaryValue::Compound(vec![nbt_entry("value", NbtSummaryValue::Int(20))]),
                )]),
            )]);
        }
        "minecraft:wandering_trader" => {
            entity.push(nbt_entry("Offers", villager_offers(component_id)));
            entity.push(nbt_entry("DespawnDelay", NbtSummaryValue::Int(i32::MIN)));
        }
        "minecraft:zombie_villager" => entity.extend([
            nbt_entry("VillagerData", villager_data(profession)),
            nbt_entry("VillagerDataFinalized", NbtSummaryValue::Byte(1)),
            nbt_entry("Offers", NbtSummaryValue::Compound(Vec::new())),
            nbt_entry("Gossips", NbtSummaryValue::List(Vec::new())),
            nbt_entry("ConversionTime", NbtSummaryValue::Int(-1)),
            nbt_entry("Xp", NbtSummaryValue::Int(0)),
        ]),
        _ => unreachable!("villager merchant fixture id"),
    }
    entity
}

pub fn zombie_entity_chunk(id: &str, uuid: Uuid, is_baby: i8) -> Java26LoadedEntityChunk {
    loaded_entity_chunk(zombie_entity_entries(id, uuid, is_baby))
}

pub fn zombified_piglin_entity_entries(uuid: Uuid, angry_at: Option<Uuid>) -> Vec<NbtSummaryEntry> {
    let mut entity = zombie_entity_entries("minecraft:zombified_piglin", uuid, 0);
    entity.push(nbt_entry("anger_end_time", NbtSummaryValue::Long(0)));
    if let Some(angry_at) = angry_at {
        entity.push(nbt_entry("angry_at", uuid_nbt(angry_at)));
    }
    entity
}

pub fn bee_entity_entries(uuid: Uuid, angry_at: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:bee", uuid, true);
    entity.extend([
        nbt_entry("Age", NbtSummaryValue::Int(0)),
        nbt_entry("ForcedAge", NbtSummaryValue::Int(0)),
        nbt_entry("AgeLocked", NbtSummaryValue::Byte(0)),
        nbt_entry("InLove", NbtSummaryValue::Int(0)),
        nbt_entry("hive_pos", NbtSummaryValue::IntArray(vec![0, -64, 0])),
        nbt_entry("flower_pos", NbtSummaryValue::IntArray(vec![1, 80, -1])),
        nbt_entry("HasNectar", NbtSummaryValue::Byte(1)),
        nbt_entry("HasStung", NbtSummaryValue::Byte(0)),
        nbt_entry("TicksSincePollination", NbtSummaryValue::Int(0)),
        nbt_entry("CannotEnterHiveTicks", NbtSummaryValue::Int(0)),
        nbt_entry("CropsGrownSincePollination", NbtSummaryValue::Int(0)),
        nbt_entry("anger_end_time", NbtSummaryValue::Long(1_200)),
        nbt_entry("angry_at", uuid_nbt(angry_at)),
    ]);
    entity
}

pub fn enderman_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:enderman", uuid, true);
    entity.extend([
        nbt_entry(
            "carriedBlockState",
            NbtSummaryValue::Compound(vec![
                nbt_entry("Name", NbtSummaryValue::string("minecraft:tnt")),
                nbt_entry(
                    "Properties",
                    NbtSummaryValue::Compound(vec![nbt_entry(
                        "unstable",
                        NbtSummaryValue::string("false"),
                    )]),
                ),
            ]),
        ),
        nbt_entry("anger_end_time", NbtSummaryValue::Long(-1)),
    ]);
    entity
}

pub fn iron_golem_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:iron_golem", uuid, true);
    entity.extend([
        nbt_entry("PlayerCreated", NbtSummaryValue::Byte(1)),
        nbt_entry("anger_end_time", NbtSummaryValue::Long(-1)),
    ]);
    entity
}

pub fn raider_entity_entries(id: &str, uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries(id, uuid, true);
    entity.extend([
        nbt_entry("patrol_target", NbtSummaryValue::IntArray(vec![0, -64, 0])),
        nbt_entry("PatrolLeader", NbtSummaryValue::Byte(0)),
        nbt_entry("Patrolling", NbtSummaryValue::Byte(1)),
        nbt_entry("Wave", NbtSummaryValue::Int(i32::MIN)),
        nbt_entry("CanJoinRaid", NbtSummaryValue::Byte(1)),
        nbt_entry("RaidId", NbtSummaryValue::Int(i32::MAX)),
    ]);
    match id {
        "minecraft:evoker" | "minecraft:illusioner" => {
            entity.push(nbt_entry("SpellTicks", NbtSummaryValue::Int(i32::MAX)));
        }
        "minecraft:pillager" => {
            entity.push(nbt_entry("Inventory", NbtSummaryValue::List(Vec::new())));
        }
        "minecraft:ravager" => entity.extend([
            nbt_entry("AttackTick", NbtSummaryValue::Int(i32::MIN)),
            nbt_entry("StunTick", NbtSummaryValue::Int(0)),
            nbt_entry("RoarTick", NbtSummaryValue::Int(i32::MAX)),
        ]),
        "minecraft:vindicator" => {
            entity.push(nbt_entry("Johnny", NbtSummaryValue::Byte(1)));
        }
        "minecraft:witch" => {}
        _ => unreachable!("raider fixture id"),
    }
    entity
}

pub fn fixed_animal_entity_entries(id: &str, uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries(id, uuid, true);
    entity.extend([
        nbt_entry("Age", NbtSummaryValue::Int(0)),
        nbt_entry("ForcedAge", NbtSummaryValue::Int(0)),
        nbt_entry("AgeLocked", NbtSummaryValue::Byte(0)),
        nbt_entry("InLove", NbtSummaryValue::Int(0)),
    ]);
    match id {
        "minecraft:ocelot" => {
            entity.push(nbt_entry("Trusting", NbtSummaryValue::Byte(1)));
        }
        "minecraft:panda" => entity.extend([
            nbt_entry("MainGene", NbtSummaryValue::string("normal")),
            nbt_entry("HiddenGene", NbtSummaryValue::string("brown")),
        ]),
        "minecraft:polar_bear" => {
            entity.push(nbt_entry("anger_end_time", NbtSummaryValue::Long(0)));
        }
        "minecraft:rabbit" => entity.extend([
            nbt_entry("RabbitType", NbtSummaryValue::Int(99)),
            nbt_entry("MoreCarrotTicks", NbtSummaryValue::Int(i32::MAX)),
        ]),
        "minecraft:sheep" => entity.extend([
            nbt_entry("Sheared", NbtSummaryValue::Byte(0)),
            nbt_entry("Color", NbtSummaryValue::Byte(15)),
        ]),
        "minecraft:turtle" => entity.extend([
            nbt_entry("home_pos", NbtSummaryValue::IntArray(vec![0, -64, 0])),
            nbt_entry("has_egg", NbtSummaryValue::Byte(1)),
        ]),
        _ => unreachable!("fixed animal fixture id"),
    }
    entity
}

pub fn ageable_brain_animal_entity_entries(id: &str, uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries(id, uuid, true);
    entity.extend([
        nbt_entry("Age", NbtSummaryValue::Int(i32::MIN)),
        nbt_entry("ForcedAge", NbtSummaryValue::Int(i32::MAX)),
        nbt_entry("AgeLocked", NbtSummaryValue::Byte(0)),
        nbt_entry("InLove", NbtSummaryValue::Int(0)),
    ]);
    let (memory_name, memory_value) = match id {
        "minecraft:armadillo" => {
            entity.extend([
                nbt_entry("state", NbtSummaryValue::string("rolling")),
                nbt_entry("scute_time", NbtSummaryValue::Int(i32::MIN)),
            ]);
            (
                "minecraft:danger_detected_recently",
                NbtSummaryValue::Byte(1),
            )
        }
        "minecraft:axolotl" => {
            entity.extend([
                nbt_entry("Variant", NbtSummaryValue::Int(4)),
                nbt_entry("FromBucket", NbtSummaryValue::Byte(1)),
            ]);
            ("minecraft:has_hunting_cooldown", NbtSummaryValue::Byte(1))
        }
        "minecraft:happy_ghast" => {
            entity.push(nbt_entry("still_timeout", NbtSummaryValue::Int(i32::MAX)));
            (
                "minecraft:temptation_cooldown_ticks",
                NbtSummaryValue::Int(0),
            )
        }
        "minecraft:goat" => {
            entity.extend([
                nbt_entry("IsScreamingGoat", NbtSummaryValue::Byte(1)),
                nbt_entry("HasLeftHorn", NbtSummaryValue::Byte(0)),
                nbt_entry("HasRightHorn", NbtSummaryValue::Byte(0)),
            ]);
            (
                "minecraft:ram_cooldown_ticks",
                NbtSummaryValue::Int(i32::MIN),
            )
        }
        "minecraft:sniffer" => (
            "minecraft:sniffer_explored_positions",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                nbt_entry("dimension", NbtSummaryValue::string("minecraft:overworld")),
                nbt_entry(
                    "pos",
                    NbtSummaryValue::IntArray(vec![i32::MIN, 0, i32::MAX]),
                ),
            ])]),
        ),
        _ => unreachable!("ageable Brain-animal fixture id"),
    };
    entity
        .iter_mut()
        .find(|entry| entry.name == "Brain")
        .unwrap()
        .value = NbtSummaryValue::Compound(vec![nbt_entry(
        "memories",
        NbtSummaryValue::Compound(vec![nbt_entry(
            memory_name,
            NbtSummaryValue::Compound(vec![
                nbt_entry("value", memory_value),
                nbt_entry("ttl", NbtSummaryValue::Long(-1)),
            ]),
        )]),
    )]);
    entity
}

fn animal_variant_entity_entries(id: &str, uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries(id, uuid, true);
    entity.extend([
        nbt_entry("Age", NbtSummaryValue::Int(0)),
        nbt_entry("ForcedAge", NbtSummaryValue::Int(0)),
        nbt_entry("AgeLocked", NbtSummaryValue::Byte(0)),
        nbt_entry("InLove", NbtSummaryValue::Int(0)),
    ]);
    entity
}

pub fn pig_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = animal_variant_entity_entries("minecraft:pig", uuid);
    entity.extend([
        nbt_entry("variant", NbtSummaryValue::string("minecraft:warm")),
        nbt_entry("sound_variant", NbtSummaryValue::string("minecraft:mini")),
        nbt_entry(
            "FutureVariantPayload",
            NbtSummaryValue::Compound(Vec::new()),
        ),
    ]);
    entity
}

pub fn cow_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = animal_variant_entity_entries("minecraft:cow", uuid);
    // Vanilla calls the superclass writer twice, but CompoundTag replacement
    // leaves one visible value for each canonical key.
    entity.extend([
        nbt_entry("variant", NbtSummaryValue::string("minecraft:cold")),
        nbt_entry("sound_variant", NbtSummaryValue::string("minecraft:moody")),
    ]);
    entity
}

pub fn chicken_entity_entries(uuid: Uuid, jockey: i8, egg_lay_time: i32) -> Vec<NbtSummaryEntry> {
    let mut entity = animal_variant_entity_entries("minecraft:chicken", uuid);
    entity.extend([
        nbt_entry("IsChickenJockey", NbtSummaryValue::Byte(jockey)),
        nbt_entry("EggLayTime", NbtSummaryValue::Int(egg_lay_time)),
        nbt_entry("variant", NbtSummaryValue::string("minecraft:temperate")),
        nbt_entry("sound_variant", NbtSummaryValue::string("minecraft:picky")),
    ]);
    entity
}

pub fn frog_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = animal_variant_entity_entries("minecraft:frog", uuid);
    entity.push(nbt_entry(
        "variant",
        NbtSummaryValue::string("minecraft:cold"),
    ));
    entity
        .iter_mut()
        .find(|entry| entry.name == "Brain")
        .unwrap()
        .value = NbtSummaryValue::Compound(vec![nbt_entry(
        "memories",
        NbtSummaryValue::Compound(vec![
            nbt_entry(
                "minecraft:temptation_cooldown_ticks",
                NbtSummaryValue::Compound(vec![nbt_entry("value", NbtSummaryValue::Int(i32::MIN))]),
            ),
            nbt_entry(
                "minecraft:long_jump_cooling_down",
                NbtSummaryValue::Compound(vec![nbt_entry("value", NbtSummaryValue::Int(i32::MAX))]),
            ),
            nbt_entry(
                "minecraft:is_in_water",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "value",
                    NbtSummaryValue::Compound(Vec::new()),
                )]),
            ),
            nbt_entry(
                "minecraft:is_pregnant",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "value",
                    NbtSummaryValue::Compound(Vec::new()),
                )]),
            ),
            nbt_entry(
                "minecraft:is_panicking",
                NbtSummaryValue::Compound(vec![nbt_entry("value", NbtSummaryValue::Byte(0))]),
            ),
            nbt_entry(
                "minecraft:is_tempted",
                NbtSummaryValue::Compound(vec![nbt_entry("value", NbtSummaryValue::Byte(1))]),
            ),
        ]),
    )]);
    entity
}

pub fn fox_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:fox", uuid, true);
    entity.extend([
        nbt_entry("Age", NbtSummaryValue::Int(0)),
        nbt_entry("ForcedAge", NbtSummaryValue::Int(0)),
        nbt_entry("AgeLocked", NbtSummaryValue::Byte(0)),
        nbt_entry("InLove", NbtSummaryValue::Int(0)),
        nbt_entry(
            "Trusted",
            NbtSummaryValue::List(vec![
                uuid_nbt(Uuid::from_u128(61)),
                uuid_nbt(Uuid::from_u128(62)),
            ]),
        ),
        nbt_entry("Sleeping", NbtSummaryValue::Byte(0)),
        nbt_entry("Type", NbtSummaryValue::string("snow")),
        nbt_entry("Sitting", NbtSummaryValue::Byte(1)),
        nbt_entry("Crouching", NbtSummaryValue::Byte(0)),
    ]);
    entity
}

pub fn vex_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:vex", uuid, true);
    entity.extend([
        nbt_entry(
            "bound_pos",
            NbtSummaryValue::IntArray(vec![i32::MIN, 0, i32::MAX]),
        ),
        nbt_entry("life_ticks", NbtSummaryValue::Int(i32::MIN)),
        nbt_entry("owner", uuid_nbt(Uuid::from_u128(71))),
    ]);
    entity
}

pub fn phantom_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:phantom", uuid, true);
    entity.extend([
        nbt_entry(
            "anchor_pos",
            NbtSummaryValue::IntArray(vec![i32::MIN, 0, i32::MAX]),
        ),
        nbt_entry("size", NbtSummaryValue::Int(64)),
    ]);
    entity
}

pub fn creeper_entity_entries(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entity = living_mob_entity_entries("minecraft:creeper", uuid, true);
    entity.extend([
        nbt_entry("powered", NbtSummaryValue::Byte(1)),
        nbt_entry("Fuse", NbtSummaryValue::Short(i16::MIN)),
        nbt_entry("ExplosionRadius", NbtSummaryValue::Byte(i8::MIN)),
        nbt_entry("ignited", NbtSummaryValue::Byte(0)),
    ]);
    entity
}

pub fn spawner_minecart_entity_chunk(uuid: Uuid, weight: i32) -> Java26LoadedEntityChunk {
    let mut entity = entity_nbt("minecraft:spawner_minecart", uuid, 0.5);
    entity.extend([
        nbt_entry("FlippedRotation", NbtSummaryValue::Byte(0)),
        nbt_entry("HasTicked", NbtSummaryValue::Byte(1)),
        nbt_entry("Delay", NbtSummaryValue::Short(20)),
        nbt_entry("MinSpawnDelay", NbtSummaryValue::Short(200)),
        nbt_entry("MaxSpawnDelay", NbtSummaryValue::Short(800)),
        nbt_entry("SpawnCount", NbtSummaryValue::Short(4)),
        nbt_entry("MaxNearbyEntities", NbtSummaryValue::Short(6)),
        nbt_entry("RequiredPlayerRange", NbtSummaryValue::Short(16)),
        nbt_entry("SpawnRange", NbtSummaryValue::Short(4)),
        nbt_entry(
            "SpawnData",
            NbtSummaryValue::Compound(vec![nbt_entry(
                "entity",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "id",
                    NbtSummaryValue::string("minecraft:pig"),
                )]),
            )]),
        ),
        nbt_entry(
            "SpawnPotentials",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                nbt_entry(
                    "data",
                    NbtSummaryValue::Compound(vec![nbt_entry(
                        "entity",
                        NbtSummaryValue::Compound(Vec::new()),
                    )]),
                ),
                nbt_entry("weight", NbtSummaryValue::Int(weight)),
            ])]),
        ),
    ]);
    Java26LoadedEntityChunk {
        timestamp: 0,
        data: Java26EntityChunkData::parse(
            crate::NamedCompound::new(
                "",
                vec![
                    nbt_entry(
                        "DataVersion",
                        NbtSummaryValue::Int(bbb_server_types::save::JAVA_26_1_DATA_VERSION),
                    ),
                    nbt_entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                    nbt_entry(
                        "Entities",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entity)]),
                    ),
                ],
            ),
            0,
            0,
        )
        .unwrap(),
    }
}

pub fn test_flat_column(
    short: bool,
    fixture: &bbb_server_types::fixture::FlatFixture,
) -> bbb_server_types::chunk_data::ChunkColumn {
    let (min_y, section_count) = if short { (0, 16) } else { (-64, 24) };
    let mut column = bbb_server_types::chunk_data::ChunkColumn::new(
        min_y,
        section_count,
        fixture.air_state_id(),
        fixture.biome_id(),
    );
    for y in min_y..=fixture.floor_y() {
        for z in 0..16 {
            for x in 0..16 {
                column
                    .set_block_state(x, y, z, fixture.stone_state_id())
                    .unwrap();
            }
        }
    }
    column
}

pub fn test_created_save(
    request: &crate::Java26WorldCreation,
    fixture: &bbb_server_types::fixture::FlatFixture,
    chunks: &[(i32, i32, &bbb_server_types::chunk_data::ChunkColumn)],
) -> crate::SaveResult<crate::Java26CreatedSave> {
    use crate::JAVA_MAIN_SERIES;
    use bbb_server_types::save::{
        default_overworld_level_stem, Java26BuiltinDimension, Java26CreatedChunk,
        Java26CreatedSave, NamedCompound, BUILTIN_ONLY_DISABLED_PACKS,
        BUILTIN_ONLY_ENABLED_FEATURES, BUILTIN_ONLY_ENABLED_PACKS, JAVA_26_1_DATA_VERSION,
        JAVA_26_1_NAME, JAVA_26_1_STORAGE_VERSION,
    };

    let strings = |items: &[&str]| {
        NbtSummaryValue::List(items.iter().map(|s| NbtSummaryValue::string(*s)).collect())
    };
    let level = NamedCompound::new(
        "",
        vec![nbt_entry(
            "Data",
            NbtSummaryValue::Compound(vec![
                nbt_entry("ServerBrands", NbtSummaryValue::List(Vec::new())),
                nbt_entry("WasModded", NbtSummaryValue::Byte(0)),
                nbt_entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                nbt_entry("GameType", NbtSummaryValue::Int(request.game_type)),
                nbt_entry(
                    "spawn",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry("dimension", NbtSummaryValue::string("minecraft:overworld")),
                        nbt_entry(
                            "pos",
                            NbtSummaryValue::IntArray(request.spawn_position.to_vec()),
                        ),
                        nbt_entry("yaw", NbtSummaryValue::Float(0.0_f32.to_bits())),
                        nbt_entry("pitch", NbtSummaryValue::Float(0.0_f32.to_bits())),
                    ]),
                ),
                nbt_entry("Time", NbtSummaryValue::Long(0)),
                nbt_entry("LastPlayed", NbtSummaryValue::Long(request.last_played)),
                nbt_entry(
                    "LevelName",
                    NbtSummaryValue::string(request.level_name.clone()),
                ),
                nbt_entry("version", NbtSummaryValue::Int(JAVA_26_1_STORAGE_VERSION)),
                nbt_entry("allowCommands", NbtSummaryValue::Byte(0)),
                nbt_entry("initialized", NbtSummaryValue::Byte(1)),
                nbt_entry(
                    "difficulty_settings",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry(
                            "difficulty",
                            NbtSummaryValue::string(request.difficulty.clone()),
                        ),
                        nbt_entry("hardcore", NbtSummaryValue::Byte(0)),
                        nbt_entry("locked", NbtSummaryValue::Byte(0)),
                    ]),
                ),
                nbt_entry(
                    "DataPacks",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry("Enabled", strings(BUILTIN_ONLY_ENABLED_PACKS)),
                        nbt_entry("Disabled", strings(BUILTIN_ONLY_DISABLED_PACKS)),
                    ]),
                ),
                nbt_entry("enabled_features", strings(BUILTIN_ONLY_ENABLED_FEATURES)),
                nbt_entry(
                    "Version",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry("Name", NbtSummaryValue::string(JAVA_26_1_NAME)),
                        nbt_entry("Id", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                        nbt_entry("Snapshot", NbtSummaryValue::Byte(0)),
                        nbt_entry("Series", NbtSummaryValue::string(JAVA_MAIN_SERIES)),
                    ]),
                ),
            ]),
        )],
    );

    let overworld_stem = default_overworld_level_stem()?;
    let noise_stem = |dim_type: &str, settings: &str, biome: &str| {
        NbtSummaryValue::Compound(vec![
            nbt_entry("type", NbtSummaryValue::string(dim_type)),
            nbt_entry(
                "generator",
                NbtSummaryValue::Compound(vec![
                    nbt_entry("type", NbtSummaryValue::string("minecraft:noise")),
                    nbt_entry("settings", NbtSummaryValue::string(settings)),
                    nbt_entry(
                        "biome_source",
                        NbtSummaryValue::Compound(vec![
                            nbt_entry("type", NbtSummaryValue::string("minecraft:fixed")),
                            nbt_entry("biome", NbtSummaryValue::string(biome)),
                        ]),
                    ),
                ]),
            ),
        ])
    };
    let world_gen_settings = NamedCompound::new(
        "",
        vec![
            nbt_entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            nbt_entry(
                "data",
                NbtSummaryValue::Compound(vec![
                    nbt_entry("bonus_chest", NbtSummaryValue::Byte(0)),
                    nbt_entry("generate_features", NbtSummaryValue::Byte(0)),
                    nbt_entry("seed", NbtSummaryValue::Long(0)),
                    nbt_entry(
                        "dimensions",
                        NbtSummaryValue::Compound(vec![
                            nbt_entry(
                                "minecraft:overworld",
                                NbtSummaryValue::Compound(vec![
                                    nbt_entry(
                                        "type",
                                        NbtSummaryValue::string(overworld_stem.dimension_type),
                                    ),
                                    nbt_entry("generator", overworld_stem.raw_generator),
                                ]),
                            ),
                            nbt_entry(
                                "minecraft:the_nether",
                                noise_stem(
                                    "minecraft:the_nether",
                                    "minecraft:nether",
                                    "minecraft:nether_wastes",
                                ),
                            ),
                            nbt_entry(
                                "minecraft:the_end",
                                noise_stem(
                                    "minecraft:the_end",
                                    "minecraft:end",
                                    "minecraft:the_end",
                                ),
                            ),
                        ]),
                    ),
                ]),
            ),
        ],
    );

    let chunks = chunks
        .iter()
        .map(|&(chunk_x, chunk_z, column)| {
            Ok(Java26CreatedChunk {
                dimension: Java26BuiltinDimension::Overworld.into(),
                chunk_x,
                chunk_z,
                document: crate::chunk_writer::full_chunk_document(
                    column,
                    chunk_x,
                    chunk_z,
                    &|_| Some("minecraft:plains".to_string()),
                    fixture.floor_y(),
                )?,
            })
        })
        .collect::<crate::SaveResult<Vec<_>>>()?;

    Ok(Java26CreatedSave {
        level,
        world_gen_settings,
        chunks,
    })
}
