use crate::level_event::{LevelEventSink, VanillaGameEvent};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID, VANILLA_ABSTRACT_HORSE_FLAG_BRED,
    VANILLA_ABSTRACT_HORSE_FLAG_EATING, VANILLA_ABSTRACT_HORSE_FLAG_STANDING,
    VANILLA_AGEABLE_MOB_BABY_DATA_ID, VANILLA_BAT_FLAGS_DATA_ID, VANILLA_ENTITY_NO_GRAVITY_DATA_ID,
    VANILLA_ENTITY_SHARED_FLAG_GLOWING, VANILLA_ENTITY_SHARED_FLAG_ON_FIRE,
    VANILLA_ENTITY_SHARED_FLAG_SPRINTING, VANILLA_ENTITY_SILENT_DATA_ID,
    VANILLA_ENTITY_TICKS_FROZEN_DATA_ID, VANILLA_GHAST_CHARGING_DATA_ID,
    VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID, VANILLA_MOB_FLAG_LEFT_HANDED, VANILLA_MOB_FLAG_NO_AI,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_BAT_ID,
    VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID,
    VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COD_ID,
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    VANILLA_ENTITY_TYPE_ENDERMAN_ID, VANILLA_ENTITY_TYPE_GHAST_ID,
    VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
    VANILLA_ENTITY_TYPE_MANNEQUIN_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    VANILLA_ENTITY_TYPE_OCELOT_ID, VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
    VANILLA_ENTITY_TYPE_SALMON_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    VANILLA_ENTITY_TYPE_SQUID_ID, VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TADPOLE_ID,
    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};
use bbb_protocol::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::packets::{BlockPos, SoundEventHolder, SoundSource, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
use bbb_server_datapack::damage::builtin_damage_type_registry;
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};
use uuid::Uuid;

use super::native_level::{NativeLevel, DIMENSION};
use super::{registry_tag_snapshot, state_id};
use crate::entity::damage::{
    StoredDamageCausingEntity, StoredEntityDamageRules, StoredEntityDamageSource,
};
use crate::entity::general_living::{
    apply_abstract_horse_random_stand, hurt_server_with_runtime, AbstractHorseRetainedState,
    AgeableMobRetainedState, AgeableWaterCreatureRetainedState, AnimalRetainedState,
    BatRetainedState, CreeperRetainedState, VexRetainedState,
};
use crate::entity::ghast::GhastRetainedState;
use crate::entity::save::{
    read_entity_chunk, resolve_saved_last_hurt_by_mob_references, write_entity_chunk,
};
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{
    tick_entities, EntityData, EntityIdCounter, EntityLifecycleEvent, EntityStore,
    GeneralLivingData,
};

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn base_entity(resource_id: &str, uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let bits = uuid.as_u128();
    vec![
        entry("id", NbtSummaryValue::string(resource_id)),
        entry(
            "Pos",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Double(0.5_f64.to_bits()),
                NbtSummaryValue::Double(64.0_f64.to_bits()),
                NbtSummaryValue::Double(0.5_f64.to_bits()),
            ]),
        ),
        entry(
            "Motion",
            NbtSummaryValue::List(vec![NbtSummaryValue::Double(0); 3]),
        ),
        entry(
            "Rotation",
            NbtSummaryValue::List(vec![NbtSummaryValue::Float(0); 2]),
        ),
        entry("fall_distance", NbtSummaryValue::Double(0)),
        entry("Fire", NbtSummaryValue::Short(0)),
        entry("Air", NbtSummaryValue::Short(300)),
        entry("OnGround", NbtSummaryValue::Byte(0)),
        entry("Invulnerable", NbtSummaryValue::Byte(0)),
        entry("PortalCooldown", NbtSummaryValue::Int(0)),
        entry(
            "UUID",
            NbtSummaryValue::IntArray(vec![
                (bits >> 96) as u32 as i32,
                (bits >> 64) as u32 as i32,
                (bits >> 32) as u32 as i32,
                bits as u32 as i32,
            ]),
        ),
    ]
}

fn zombie_record(uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let mut entries = base_entity("minecraft:zombie", uuid);
    entries.extend([
        entry("Health", NbtSummaryValue::Float(17.0_f32.to_bits())),
        entry("HurtTime", NbtSummaryValue::Short(2)),
        entry("HurtByTimestamp", NbtSummaryValue::Int(42)),
        entry("DeathTime", NbtSummaryValue::Short(0)),
        entry(
            "AbsorptionAmount",
            NbtSummaryValue::Float(0.0_f32.to_bits()),
        ),
        entry(
            "current_impulse_context_reset_grace_time",
            NbtSummaryValue::Int(4),
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
        entry(
            "FutureZombieAlpha",
            NbtSummaryValue::Compound(vec![entry("value", NbtSummaryValue::Long(11))]),
        ),
        entry("PersistenceRequired", NbtSummaryValue::Byte(0)),
        entry("LeftHanded", NbtSummaryValue::Byte(0)),
        entry("IsBaby", NbtSummaryValue::Byte(0)),
        entry("CanBreakDoors", NbtSummaryValue::Byte(0)),
        entry("InWaterTime", NbtSummaryValue::Int(-1)),
        entry("DrownedConversionTime", NbtSummaryValue::Int(-1)),
        entry(
            "FutureZombieOmega",
            NbtSummaryValue::List(vec![NbtSummaryValue::string("kept")]),
        ),
    ]);
    entries
}

fn bat_record(uuid: Uuid, flags: Option<NbtSummaryValue>) -> Vec<NbtSummaryEntry> {
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string("minecraft:bat");
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby"
                | "CanBreakDoors"
                | "InWaterTime"
                | "DrownedConversionTime"
                | "FutureZombieAlpha"
                | "FutureZombieOmega"
        )
    });
    if let Some(flags) = flags {
        entries.push(entry("BatFlags", flags));
    }
    entries
}

fn ghast_record(uuid: Uuid, explosion_power: Option<NbtSummaryValue>) -> Vec<NbtSummaryEntry> {
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string("minecraft:ghast");
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby"
                | "CanBreakDoors"
                | "InWaterTime"
                | "DrownedConversionTime"
                | "FutureZombieAlpha"
                | "FutureZombieOmega"
        )
    });
    if let Some(explosion_power) = explosion_power {
        entries.push(entry("ExplosionPower", explosion_power));
    }
    entries
}

fn phantom_record(
    uuid: Uuid,
    size: Option<NbtSummaryValue>,
    anchor: Option<NbtSummaryValue>,
) -> Vec<NbtSummaryEntry> {
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string("minecraft:phantom");
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby"
                | "CanBreakDoors"
                | "InWaterTime"
                | "DrownedConversionTime"
                | "FutureZombieAlpha"
                | "FutureZombieOmega"
        )
    });
    if let Some(anchor) = anchor {
        entries.push(entry("anchor_pos", anchor));
    }
    if let Some(size) = size {
        entries.push(entry("size", size));
    }
    entries
}

fn vex_record(
    uuid: Uuid,
    bound_origin: Option<NbtSummaryValue>,
    limited_life_ticks: Option<NbtSummaryValue>,
    owner: Option<NbtSummaryValue>,
) -> Vec<NbtSummaryEntry> {
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string("minecraft:vex");
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby"
                | "CanBreakDoors"
                | "InWaterTime"
                | "DrownedConversionTime"
                | "FutureZombieAlpha"
                | "FutureZombieOmega"
        )
    });
    entries
        .iter_mut()
        .find(|entry| entry.name == "HurtTime")
        .unwrap()
        .value = NbtSummaryValue::Short(0);
    for (name, value) in [
        ("bound_pos", bound_origin),
        ("life_ticks", limited_life_ticks),
        ("owner", owner),
    ] {
        if let Some(value) = value {
            entries.push(entry(name, value));
        }
    }
    entries
}

fn creeper_record(
    uuid: Uuid,
    powered: bool,
    fuse: i16,
    explosion_radius: i8,
    ignited: bool,
) -> Vec<NbtSummaryEntry> {
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string("minecraft:creeper");
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby"
                | "CanBreakDoors"
                | "InWaterTime"
                | "DrownedConversionTime"
                | "FutureZombieAlpha"
                | "FutureZombieOmega"
        )
    });
    entries.extend([
        entry("powered", NbtSummaryValue::Byte(i8::from(powered))),
        entry("Fuse", NbtSummaryValue::Short(fuse)),
        entry("ExplosionRadius", NbtSummaryValue::Byte(explosion_radius)),
        entry("ignited", NbtSummaryValue::Byte(i8::from(ignited))),
        entry(
            "FutureCreeperPayload",
            NbtSummaryValue::Compound(vec![entry("value", NbtSummaryValue::Int(7))]),
        ),
    ]);
    entries
}

fn bucketable_record(
    resource_id: &str,
    uuid: Uuid,
    from_bucket: Option<bool>,
) -> Vec<NbtSummaryEntry> {
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string(resource_id);
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby" | "CanBreakDoors" | "InWaterTime" | "DrownedConversionTime"
        )
    });
    if let Some(from_bucket) = from_bucket {
        entries.push(entry(
            "FromBucket",
            NbtSummaryValue::Byte(i8::from(from_bucket)),
        ));
    }
    match resource_id {
        "minecraft:axolotl" => entries.extend([
            entry("Age", NbtSummaryValue::Int(0)),
            entry("ForcedAge", NbtSummaryValue::Int(0)),
            entry("AgeLocked", NbtSummaryValue::Byte(0)),
            entry("InLove", NbtSummaryValue::Int(0)),
            entry("Variant", NbtSummaryValue::Int(0)),
        ]),
        "minecraft:cod" => {}
        "minecraft:pufferfish" => entries.push(entry("PuffState", NbtSummaryValue::Int(0))),
        "minecraft:salmon" => entries.push(entry("type", NbtSummaryValue::string("medium"))),
        "minecraft:tadpole" => entries.extend([
            entry("Age", NbtSummaryValue::Int(0)),
            entry("AgeLocked", NbtSummaryValue::Byte(0)),
        ]),
        "minecraft:tropical_fish" => entries.push(entry("Variant", NbtSummaryValue::Int(0x0500))),
        _ => unreachable!("bucketable test resource"),
    }
    entries
}

fn ageable_mob_record(
    uuid: Uuid,
    age: i32,
    forced_age: i32,
    age_locked: bool,
) -> Vec<NbtSummaryEntry> {
    let mut entries = bucketable_record("minecraft:axolotl", uuid, Some(false));
    for (name, value) in [
        ("Age", NbtSummaryValue::Int(age)),
        ("ForcedAge", NbtSummaryValue::Int(forced_age)),
        ("AgeLocked", NbtSummaryValue::Byte(i8::from(age_locked))),
    ] {
        entries
            .iter_mut()
            .find(|entry| entry.name == name)
            .unwrap()
            .value = value;
    }
    entries
}

fn ageable_water_record(
    resource_id: &str,
    uuid: Uuid,
    got_fish: bool,
    moistness: i32,
    dark_ticks: i32,
) -> Vec<NbtSummaryEntry> {
    let mut entries = ageable_mob_record(uuid, 0, 0, false);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string(resource_id);
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "FromBucket" | "InLove" | "Variant"
        )
    });
    match resource_id {
        "minecraft:dolphin" => entries.extend([
            entry("GotFish", NbtSummaryValue::Byte(i8::from(got_fish))),
            entry("Moistness", NbtSummaryValue::Int(moistness)),
        ]),
        "minecraft:glow_squid" => entries.push(entry(
            "DarkTicksRemaining",
            NbtSummaryValue::Int(dark_ticks),
        )),
        "minecraft:squid" => {}
        _ => unreachable!("AgeableWaterCreature test resource"),
    }
    entries
}

fn animal_record(
    uuid: Uuid,
    age: i32,
    in_love: i32,
    love_cause: Option<Uuid>,
) -> Vec<NbtSummaryEntry> {
    let mut entries = ageable_mob_record(uuid, age, 0, false);
    entries
        .iter_mut()
        .find(|entry| entry.name == "InLove")
        .unwrap()
        .value = NbtSummaryValue::Int(in_love);
    if let Some(love_cause) = love_cause {
        entries.push(entry("LoveCause", saved_uuid(love_cause)));
    }
    entries
}

fn abstract_horse_record(
    uuid: Uuid,
    eating: Option<bool>,
    bred: Option<bool>,
    temper: Option<i32>,
    tame: Option<bool>,
    owner: Option<Uuid>,
) -> Vec<NbtSummaryEntry> {
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string("minecraft:zombie_horse");
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby" | "CanBreakDoors" | "InWaterTime" | "DrownedConversionTime"
        )
    });
    entries.extend([
        entry("Age", NbtSummaryValue::Int(0)),
        entry("ForcedAge", NbtSummaryValue::Int(0)),
        entry("AgeLocked", NbtSummaryValue::Byte(0)),
        entry("InLove", NbtSummaryValue::Int(0)),
    ]);
    for (name, value) in [
        eating.map(|value| ("EatingHaystack", NbtSummaryValue::Byte(i8::from(value)))),
        bred.map(|value| ("Bred", NbtSummaryValue::Byte(i8::from(value)))),
        temper.map(|value| ("Temper", NbtSummaryValue::Int(value))),
        tame.map(|value| ("Tame", NbtSummaryValue::Byte(i8::from(value)))),
    ]
    .into_iter()
    .flatten()
    {
        entries.push(entry(name, value));
    }
    if let Some(owner) = owner {
        entries.push(entry("Owner", saved_uuid(owner)));
    }
    entries
}

fn saved_uuid(uuid: Uuid) -> NbtSummaryValue {
    let bits = uuid.as_u128();
    NbtSummaryValue::IntArray(vec![
        (bits >> 96) as u32 as i32,
        (bits >> 64) as u32 as i32,
        (bits >> 32) as u32 as i32,
        bits as u32 as i32,
    ])
}

fn tamable_record(
    resource_id: &str,
    uuid: Uuid,
    owner: Option<Uuid>,
    sitting: Option<bool>,
) -> Vec<NbtSummaryEntry> {
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string(resource_id);
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby" | "CanBreakDoors" | "InWaterTime" | "DrownedConversionTime"
        )
    });
    entries.extend([
        entry("Age", NbtSummaryValue::Int(0)),
        entry("ForcedAge", NbtSummaryValue::Int(0)),
        entry("AgeLocked", NbtSummaryValue::Byte(0)),
        entry("InLove", NbtSummaryValue::Int(0)),
    ]);
    if let Some(owner) = owner {
        entries.push(entry("Owner", saved_uuid(owner)));
    }
    if let Some(sitting) = sitting {
        entries.push(entry("Sitting", NbtSummaryValue::Byte(i8::from(sitting))));
    }
    match resource_id {
        "minecraft:cat" => entries.extend([
            entry("variant", NbtSummaryValue::string("minecraft:black")),
            entry(
                "sound_variant",
                NbtSummaryValue::string("minecraft:classic"),
            ),
            entry("CollarColor", NbtSummaryValue::Byte(14)),
        ]),
        "minecraft:nautilus" => {}
        "minecraft:parrot" => entries.push(entry("Variant", NbtSummaryValue::Int(0))),
        "minecraft:wolf" => entries.extend([
            entry("CollarColor", NbtSummaryValue::Byte(14)),
            entry("variant", NbtSummaryValue::string("minecraft:pale")),
            entry(
                "sound_variant",
                NbtSummaryValue::string("minecraft:classic"),
            ),
            entry("anger_end_time", NbtSummaryValue::Long(-1)),
        ]),
        "minecraft:zombie_nautilus" => entries.push(entry(
            "variant",
            NbtSummaryValue::string("minecraft:temperate"),
        )),
        _ => unreachable!("tamable test resource"),
    }
    entries
}

fn conditional_animal_record(
    resource_id: &str,
    uuid: Uuid,
    retained: Option<bool>,
) -> Vec<NbtSummaryEntry> {
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string(resource_id);
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby" | "CanBreakDoors" | "InWaterTime" | "DrownedConversionTime"
        )
    });
    entries.extend([
        entry("Age", NbtSummaryValue::Int(0)),
        entry("ForcedAge", NbtSummaryValue::Int(0)),
        entry("AgeLocked", NbtSummaryValue::Byte(0)),
        entry("InLove", NbtSummaryValue::Int(0)),
    ]);
    match resource_id {
        "minecraft:chicken" => {
            if let Some(jockey) = retained {
                entries.push(entry(
                    "IsChickenJockey",
                    NbtSummaryValue::Byte(i8::from(jockey)),
                ));
            }
            entries.extend([
                entry("EggLayTime", NbtSummaryValue::Int(6000)),
                entry("variant", NbtSummaryValue::string("minecraft:temperate")),
                entry(
                    "sound_variant",
                    NbtSummaryValue::string("minecraft:classic"),
                ),
            ]);
        }
        "minecraft:ocelot" => {
            if let Some(trusting) = retained {
                entries.push(entry("Trusting", NbtSummaryValue::Byte(i8::from(trusting))));
            }
        }
        _ => unreachable!("conditional animal test resource"),
    }
    entries
}

fn enderman_record(
    uuid: Uuid,
    carried_block_state: Option<NbtSummaryValue>,
) -> Vec<NbtSummaryEntry> {
    let mut entries = zombie_record(uuid);
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string("minecraft:enderman");
    entries.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby" | "CanBreakDoors" | "InWaterTime" | "DrownedConversionTime"
        )
    });
    entries.push(entry("anger_end_time", NbtSummaryValue::Long(-1)));
    if let Some(state) = carried_block_state {
        entries.push(entry("carriedBlockState", state));
    }
    entries
}

fn saved_block_state(resource_id: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![entry("Name", NbtSummaryValue::string(resource_id))])
}

fn chunk_document(entries: Vec<NbtSummaryEntry>) -> NamedCompound {
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
            entry(
                "Entities",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entries)]),
            ),
        ],
    )
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(chunk_document(entries), 0, 0)
        .expect("the common Java 26.1 entity record is valid")
}

fn entity_type_id(resource_id: &str) -> i32 {
    VANILLA_ENTITY_TYPES
        .iter()
        .find(|facts| facts.resource_id == resource_id)
        .map(|facts| facts.id)
        .expect("test entity belongs to the generated catalog")
}

fn restore_test_horse(
    store: &mut EntityStore,
    uuid: Uuid,
    entity_type_id: i32,
    retained: AbstractHorseRetainedState,
) -> i32 {
    let mut data = GeneralLivingData::new(entity_type_id).unwrap();
    data.set_no_ai(true);
    *data.abstract_horse_retained_mut().unwrap() = retained;
    store
        .restore(
            uuid,
            entity_type_id,
            position(),
            EntityData::GeneralLiving(data),
        )
        .unwrap()
}

#[test]
fn catalog_supplier_qualifies_zombie_but_not_session_specialized_or_nonliving_types() {
    let zombie = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).expect("zombie is living");
    assert_eq!(zombie.entity_resource_id(), "minecraft:zombie");
    assert!(GeneralLivingData::new(VANILLA_ENTITY_TYPE_PLAYER_ID).is_none());
    assert!(GeneralLivingData::new(VANILLA_ENTITY_TYPE_ARMOR_STAND_ID).is_none());
    assert!(GeneralLivingData::new(VANILLA_ENTITY_TYPE_ITEM_ID).is_none());

    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .expect("the qualified type spawns");
    let entity = store.get(id).unwrap();
    let EntityData::GeneralLiving(data) = entity.data() else {
        panic!("spawn publishes the general variant")
    };
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert_eq!(data.entity_resource_id(), "minecraft:zombie");
    assert!(entity.is_pickable());

    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    assert_eq!(saved.entities().next().unwrap().id(), "minecraft:zombie");
}

#[test]
fn mannequin_profile_accessor_exposes_the_default_empty_profile_only() {
    let mannequin = GeneralLivingData::new(VANILLA_ENTITY_TYPE_MANNEQUIN_ID).unwrap();
    assert_eq!(
        mannequin.mannequin_profile(),
        Some(&NbtSummaryValue::Compound(Vec::new()))
    );
    let zombie = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    assert_eq!(zombie.mannequin_profile(), None);
}

#[test]
fn zombified_piglin_missing_anger_end_time_normalizes_before_rewrite() {
    let mut entries = zombie_record(Uuid::from_u128(0x92));
    entries
        .iter_mut()
        .find(|entry| entry.name == "id")
        .unwrap()
        .value = NbtSummaryValue::string("minecraft:zombified_piglin");

    let mut store = EntityStore::new(2, EntityIdCounter::new());
    read_entity_chunk(&mut store, &chunk(entries)).unwrap();

    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    assert_eq!(
        saved
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .find(|entry| entry.name == "anger_end_time")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Long(-1))
    );
}

mod ageable_runtime;
mod ambient_sound;
mod autonomous_runtime;
mod core_runtime;
mod despawn;
mod horse_runtime;
mod prepared_immunity;

#[test]
fn a_perched_ender_dragon_takes_no_knockback() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            Vec3d {
                x: 0.5,
                y: 80.0,
                z: 0.5,
            },
        )
        .unwrap();
    let mut dragon = store.detach(id).unwrap();
    // `EnderDragonPhase` ids: HOLDING_PATTERN 0, SITTING_SCANNING 6.
    for (phase, moves) in [(0, true), (6, false)] {
        let EntityData::GeneralLiving(data) = &mut dragon.data else {
            unreachable!()
        };
        crate::entity::ender_dragon::read_save_data(
            data.ender_dragon_mut().unwrap(),
            &[bbb_protocol::NbtSummaryEntry {
                name: "DragonPhase".into(),
                value: bbb_protocol::NbtSummaryValue::Int(phase),
            }],
        );
        dragon.set_delta_movement(Vec3d::default());
        crate::entity::living::knockback(&mut dragon, &mut store, 0.4, 1.0, 0.0);
        assert_eq!(
            dragon.delta_movement() != Vec3d::default(),
            moves,
            "phase {phase}"
        );
    }
}
