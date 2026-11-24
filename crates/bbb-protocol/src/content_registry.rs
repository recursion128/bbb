//! Session-frozen modded content registry synced during configuration.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::spin_lock::SpinLock;

use bbb_facts::attribute_types::VANILLA_ATTRIBUTE_TYPES;
use bbb_facts::block_state_types::VANILLA_BLOCK_STATE_COUNT;
use bbb_facts::block_types::VANILLA_BLOCK_TYPES;
use bbb_facts::entity_types::{
    vanilla_entity_mob_category_for_serialized_name, VanillaEntityMobCategory,
    VANILLA_ENTITY_MOB_CATEGORIES, VANILLA_ENTITY_TYPES,
};
use bbb_facts::item_types::VANILLA_ITEM_TYPES;
use bbb_facts::particle_types::VANILLA_PARTICLE_TYPES;
use bbb_facts::sound_event_types::VANILLA_SOUND_EVENT_RESOURCE_IDS;

use crate::codec::{Decoder, Encoder, ProtocolError, Result};
use crate::packets::entities::MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID;
use crate::packets::normalize_resource_location;

pub const BBB_CONTENT_REGISTRY_CHANNEL: &str = "bbb:content_registry";
const WIRE_MAGIC: [u8; 4] = *b"BBCR";
const CONTENT_REGISTRY_WIRE_VERSION: u32 = 1;
const MAX_MOD_ENTRIES_PER_KIND: usize = 4096;
const MAX_CUSTOM_SYNCHED_FIELD_COUNT: usize = 128;
const MAX_DEFAULT_ATTRIBUTES_PER_ENTITY: usize = 64;
const MAX_RESOURCE_STRING_BYTES: usize = 256;
// Sized above a maximal server registry (`5 * MAX_MOD_ENTRIES_PER_KIND = 20_480`) so startup freeze
// never hits the cap; wire-decoded client registries do not intern into this pool.
const MAX_INTERNED_MOD_RESOURCE_IDS: usize = 32_768;

static MOD_RESOURCE_ID_POOL: SpinLock<Option<HashSet<&'static str>>> = SpinLock::new(None);

#[derive(Debug, Clone, Copy, Default)]
struct InternedStaticStr(&'static str);

impl PartialEq for InternedStaticStr {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for InternedStaticStr {}

fn with_intern_pool<R>(f: impl FnOnce(&mut HashSet<&'static str>) -> R) -> R {
    MOD_RESOURCE_ID_POOL.with(|slot| f(slot.get_or_insert_with(HashSet::new)))
}

/// Validates and interns `<namespace>:<path>` (with `namespace != "minecraft"`)
/// into process-lifetime storage so `&'static str` server callers can borrow modded
/// resource IDs lock-free across threads without per-entity string clones.
pub fn intern_mod_resource_id(raw: &str) -> Result<&'static str> {
    let normalized = normalize_mod_resource_location(raw)?;
    with_intern_pool(|set| {
        if let Some(&existing) = set.get(normalized.as_str()) {
            return Ok(existing);
        }
        if set.len() >= MAX_INTERNED_MOD_RESOURCE_IDS {
            return Err(ProtocolError::InvalidData(format!(
                "modded resource intern pool exceeded {MAX_INTERNED_MOD_RESOURCE_IDS} entries"
            )));
        }
        let leaked: &'static str = Box::leak(normalized.into_boxed_str());
        set.insert(leaked);
        Ok(leaked)
    })
}

/// Resolves a `&'static str` for an already-validated mod resource id without
/// panicking or bypassing [`MAX_INTERNED_MOD_RESOURCE_IDS`] if the global
/// deduplication table is full. Prefer `resource_id()` wherever a borrowed
/// `&str` suffices.
fn resolve_static_resource_id(interned: InternedStaticStr, resource_id: &str) -> &'static str {
    if !interned.0.is_empty() {
        return interned.0;
    }
    intern_mod_resource_id(resource_id).unwrap_or("bbb:uninterned")
}

fn try_resolve_static_resource_id(
    interned: InternedStaticStr,
    resource_id: &str,
) -> Result<&'static str> {
    if !interned.0.is_empty() {
        return Ok(interned.0);
    }
    intern_mod_resource_id(resource_id)
}

fn normalize_mod_resource_location(raw: &str) -> Result<String> {
    let normalized = normalize_resource_location(raw)?;
    if normalized.len() > MAX_RESOURCE_STRING_BYTES {
        return Err(ProtocolError::InvalidData(format!(
            "mod resource location exceeds {MAX_RESOURCE_STRING_BYTES} bytes"
        )));
    }
    let Some((namespace, path)) = normalized.split_once(':') else {
        return Err(ProtocolError::InvalidData(format!(
            "mod resource location `{raw}` must contain a namespace"
        )));
    };
    if namespace.is_empty() || path.is_empty() || namespace == "minecraft" {
        return Err(ProtocolError::InvalidData(format!(
            "mod resource location `{raw}` must use a non-minecraft namespace and non-empty path"
        )));
    }
    Ok(normalized)
}

fn path_namespace_key(resource_id: &str) -> (&str, &str) {
    let (ns, path) = resource_id.split_once(':').unwrap_or(("", resource_id));
    (path, ns)
}

mod mob_category_serde {
    use super::*;

    pub fn serialize<S>(
        category: &VanillaEntityMobCategory,
        serializer: S,
    ) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let name = VANILLA_ENTITY_MOB_CATEGORIES
            .iter()
            .find_map(|(cat, s)| (*cat == *category).then_some(*s))
            .unwrap_or("misc");
        serializer.serialize_str(name)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> core::result::Result<VanillaEntityMobCategory, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        vanilla_entity_mob_category_for_serialized_name(&name).ok_or_else(|| {
            serde::de::Error::custom(format!("unknown entity mob category `{name}`"))
        })
    }
}

fn encode_mob_category(category: VanillaEntityMobCategory) -> u8 {
    match category {
        VanillaEntityMobCategory::Monster => 0,
        VanillaEntityMobCategory::Creature => 1,
        VanillaEntityMobCategory::Ambient => 2,
        VanillaEntityMobCategory::Axolotls => 3,
        VanillaEntityMobCategory::UndergroundWaterCreature => 4,
        VanillaEntityMobCategory::WaterCreature => 5,
        VanillaEntityMobCategory::WaterAmbient => 6,
        VanillaEntityMobCategory::Misc => 7,
    }
}

fn decode_mob_category(tag: u8) -> Result<VanillaEntityMobCategory> {
    match tag {
        0 => Ok(VanillaEntityMobCategory::Monster),
        1 => Ok(VanillaEntityMobCategory::Creature),
        2 => Ok(VanillaEntityMobCategory::Ambient),
        3 => Ok(VanillaEntityMobCategory::Axolotls),
        4 => Ok(VanillaEntityMobCategory::UndergroundWaterCreature),
        5 => Ok(VanillaEntityMobCategory::WaterCreature),
        6 => Ok(VanillaEntityMobCategory::WaterAmbient),
        7 => Ok(VanillaEntityMobCategory::Misc),
        _ => Err(ProtocolError::InvalidData(format!(
            "invalid mob category ordinal {tag}"
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModBossBarSpec {
    pub color_id: i32,
    pub overlay_id: i32,
    pub darken_screen: bool,
    pub play_boss_music: bool,
    pub create_world_fog: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModSynchedFieldSpec {
    pub data_id: u8,
    pub serializer_id: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModEntityRegistration {
    pub resource_id: String,
    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
    pub client_tracking_range: i32,
    pub update_interval: i32,
    #[serde(with = "mob_category_serde")]
    pub category: VanillaEntityMobCategory,
    pub is_living: bool,
    pub is_mob: bool,
    pub fire_immune: bool,
    pub can_summon: bool,
    pub serializable: bool,
    pub allowed_in_peaceful: bool,
    pub default_attributes: Vec<(i32, f64)>,
    pub synched_fields: Vec<ModSynchedFieldSpec>,
    pub ambient_sound: Option<String>,
    pub hurt_sound: Option<String>,
    pub death_sound: Option<String>,
    pub default_loot_table: Option<String>,
    pub xp_reward: i32,
    pub rig_id: Option<String>,
    pub dynamic_light_rgb: Option<[f32; 3]>,
    pub boss_bar: Option<ModBossBarSpec>,
}

impl Default for ModEntityRegistration {
    fn default() -> Self {
        Self {
            resource_id: String::new(),
            width: 0.6,
            height: 1.8,
            eye_height: 1.62,
            client_tracking_range: 8,
            update_interval: 3,
            category: VanillaEntityMobCategory::Creature,
            is_living: true,
            is_mob: true,
            fire_immune: false,
            can_summon: true,
            serializable: true,
            allowed_in_peaceful: true,
            default_attributes: Vec::new(),
            synched_fields: Vec::new(),
            ambient_sound: None,
            hurt_sound: None,
            death_sound: None,
            default_loot_table: None,
            xp_reward: 5,
            rig_id: None,
            dynamic_light_rgb: None,
            boss_bar: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModEntityTypeSpec {
    pub id: i32,
    pub resource_id: String,
    #[serde(skip)]
    interned_id: InternedStaticStr,
    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
    pub client_tracking_range: i32,
    pub update_interval: i32,
    #[serde(with = "mob_category_serde")]
    pub category: VanillaEntityMobCategory,
    pub is_living: bool,
    pub is_mob: bool,
    pub fire_immune: bool,
    pub can_summon: bool,
    pub serializable: bool,
    pub allowed_in_peaceful: bool,
    pub default_attributes: Vec<(i32, f64)>,
    pub synched_fields: Vec<ModSynchedFieldSpec>,
    pub ambient_sound: Option<String>,
    pub hurt_sound: Option<String>,
    pub death_sound: Option<String>,
    pub default_loot_table: Option<String>,
    pub xp_reward: i32,
    pub rig_id: Option<String>,
    pub dynamic_light_rgb: Option<[f32; 3]>,
    pub boss_bar: Option<ModBossBarSpec>,
}

impl ModEntityTypeSpec {
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    pub fn try_resource_id_static(&self) -> Result<&'static str> {
        try_resolve_static_resource_id(self.interned_id, &self.resource_id)
    }

    pub fn resource_id_static(&self) -> &'static str {
        resolve_static_resource_id(self.interned_id, &self.resource_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModItemRegistration {
    pub resource_id: String,
    pub max_stack_size: u8,
    pub is_block_item: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModItemSpec {
    pub id: i32,
    pub resource_id: String,
    #[serde(skip)]
    interned_id: InternedStaticStr,
    pub max_stack_size: u8,
    pub is_block_item: bool,
}

impl ModItemSpec {
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    pub fn try_resource_id_static(&self) -> Result<&'static str> {
        try_resolve_static_resource_id(self.interned_id, &self.resource_id)
    }

    pub fn resource_id_static(&self) -> &'static str {
        resolve_static_resource_id(self.interned_id, &self.resource_id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModBlockRegistration {
    pub resource_id: String,
    pub state_count: u16,
    pub light_emission: u8,
    pub destroy_speed: f32,
    pub full_cube_collision: bool,
    pub dynamic_light_rgb: Option<[f32; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModBlockSpec {
    pub block_id: i32,
    pub first_state_id: i32,
    pub state_count: u16,
    pub resource_id: String,
    #[serde(skip)]
    interned_id: InternedStaticStr,
    pub light_emission: u8,
    pub destroy_speed: f32,
    pub full_cube_collision: bool,
    pub dynamic_light_rgb: Option<[f32; 3]>,
}

impl ModBlockSpec {
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    pub fn try_resource_id_static(&self) -> Result<&'static str> {
        try_resolve_static_resource_id(self.interned_id, &self.resource_id)
    }

    pub fn resource_id_static(&self) -> &'static str {
        resolve_static_resource_id(self.interned_id, &self.resource_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModSoundEventSpec {
    pub id: i32,
    pub resource_id: String,
    #[serde(skip)]
    interned_id: InternedStaticStr,
}

impl ModSoundEventSpec {
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    pub fn try_resource_id_static(&self) -> Result<&'static str> {
        try_resolve_static_resource_id(self.interned_id, &self.resource_id)
    }

    pub fn resource_id_static(&self) -> &'static str {
        resolve_static_resource_id(self.interned_id, &self.resource_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModParticleRegistration {
    pub resource_id: String,
    pub override_limiter: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModParticleSpec {
    pub id: i32,
    pub resource_id: String,
    #[serde(skip)]
    interned_id: InternedStaticStr,
    pub override_limiter: bool,
}

impl ModParticleSpec {
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    pub fn try_resource_id_static(&self) -> Result<&'static str> {
        try_resolve_static_resource_id(self.interned_id, &self.resource_id)
    }

    pub fn resource_id_static(&self) -> &'static str {
        resolve_static_resource_id(self.interned_id, &self.resource_id)
    }
}

#[derive(Debug, Default, Clone)]
pub struct ContentRegistryBuilder {
    entities: Vec<ModEntityRegistration>,
    items: Vec<ModItemRegistration>,
    blocks: Vec<ModBlockRegistration>,
    sound_events: Vec<String>,
    particles: Vec<ModParticleRegistration>,
}

impl ContentRegistryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_entity(&mut self, spec: ModEntityRegistration) -> &mut Self {
        self.entities.push(spec);
        self
    }

    pub fn register_item(&mut self, spec: ModItemRegistration) -> &mut Self {
        self.items.push(spec);
        self
    }

    pub fn register_block(&mut self, spec: ModBlockRegistration) -> &mut Self {
        self.blocks.push(spec);
        self
    }

    pub fn register_sound_event(&mut self, resource_id: impl Into<String>) -> &mut Self {
        self.sound_events.push(resource_id.into());
        self
    }

    pub fn register_particle(&mut self, spec: ModParticleRegistration) -> &mut Self {
        self.particles.push(spec);
        self
    }

    pub fn freeze(self) -> Result<ContentRegistry> {
        self.freeze_internal(true)
    }

    /// Freezes a client/session registry without interning resource IDs into
    /// process-global static memory.
    pub fn freeze_for_session(self) -> Result<ContentRegistry> {
        self.freeze_internal(false)
    }

    fn freeze_internal(mut self, intern_static: bool) -> Result<ContentRegistry> {
        check_collection_limit(self.entities.len(), "mod entities")?;
        check_collection_limit(self.items.len(), "mod items")?;
        check_collection_limit(self.blocks.len(), "mod blocks")?;
        check_collection_limit(self.sound_events.len(), "mod sound events")?;
        check_collection_limit(self.particles.len(), "mod particles")?;

        for entity in &mut self.entities {
            entity.resource_id = normalize_mod_resource_location(&entity.resource_id)?;
            validate_entity_registration(entity)?;
        }
        self.entities.sort_by(|a, b| {
            path_namespace_key(&a.resource_id).cmp(&path_namespace_key(&b.resource_id))
        });

        for item in &mut self.items {
            item.resource_id = normalize_mod_resource_location(&item.resource_id)?;
            if item.max_stack_size == 0 || item.max_stack_size > 99 {
                return Err(ProtocolError::InvalidData(format!(
                    "mod item `{}` stack size {} outside 1..=99",
                    item.resource_id, item.max_stack_size
                )));
            }
        }
        self.items.sort_by(|a, b| {
            path_namespace_key(&a.resource_id).cmp(&path_namespace_key(&b.resource_id))
        });

        for block in &mut self.blocks {
            block.resource_id = normalize_mod_resource_location(&block.resource_id)?;
            validate_block_registration(block)?;
        }
        self.blocks.sort_by(|a, b| {
            path_namespace_key(&a.resource_id).cmp(&path_namespace_key(&b.resource_id))
        });

        for sound in &mut self.sound_events {
            *sound = normalize_mod_resource_location(sound)?;
        }
        self.sound_events
            .sort_by(|a, b| path_namespace_key(a).cmp(&path_namespace_key(b)));

        for particle in &mut self.particles {
            particle.resource_id = normalize_mod_resource_location(&particle.resource_id)?;
        }
        self.particles.sort_by(|a, b| {
            path_namespace_key(&a.resource_id).cmp(&path_namespace_key(&b.resource_id))
        });

        let entity_base = VANILLA_ENTITY_TYPES.len() as i32;
        let mut entities = Vec::with_capacity(self.entities.len());
        for (idx, reg) in self.entities.into_iter().enumerate() {
            let interned_id = if intern_static {
                InternedStaticStr(intern_mod_resource_id(&reg.resource_id)?)
            } else {
                InternedStaticStr::default()
            };
            entities.push(ModEntityTypeSpec {
                id: entity_base + idx as i32,
                resource_id: reg.resource_id,
                interned_id,
                width: reg.width,
                height: reg.height,
                eye_height: reg.eye_height,
                client_tracking_range: reg.client_tracking_range,
                update_interval: reg.update_interval,
                category: reg.category,
                is_living: reg.is_living,
                is_mob: reg.is_mob,
                fire_immune: reg.fire_immune,
                can_summon: reg.can_summon,
                serializable: reg.serializable,
                allowed_in_peaceful: reg.allowed_in_peaceful,
                default_attributes: reg.default_attributes,
                synched_fields: reg.synched_fields,
                ambient_sound: reg.ambient_sound,
                hurt_sound: reg.hurt_sound,
                death_sound: reg.death_sound,
                default_loot_table: reg.default_loot_table,
                xp_reward: reg.xp_reward,
                rig_id: reg.rig_id,
                dynamic_light_rgb: reg.dynamic_light_rgb,
                boss_bar: reg.boss_bar,
            });
        }

        let item_base = VANILLA_ITEM_TYPES.len() as i32;
        let mut items = Vec::with_capacity(self.items.len());
        for (idx, reg) in self.items.into_iter().enumerate() {
            let interned_id = if intern_static {
                InternedStaticStr(intern_mod_resource_id(&reg.resource_id)?)
            } else {
                InternedStaticStr::default()
            };
            items.push(ModItemSpec {
                id: item_base + idx as i32,
                resource_id: reg.resource_id,
                interned_id,
                max_stack_size: reg.max_stack_size,
                is_block_item: reg.is_block_item,
            });
        }

        let block_base = VANILLA_BLOCK_TYPES.len() as i32;
        let mut next_state_id = VANILLA_BLOCK_STATE_COUNT;
        let mut blocks = Vec::with_capacity(self.blocks.len());
        for (idx, reg) in self.blocks.into_iter().enumerate() {
            let interned_id = if intern_static {
                InternedStaticStr(intern_mod_resource_id(&reg.resource_id)?)
            } else {
                InternedStaticStr::default()
            };
            let first_state_id = next_state_id;
            next_state_id = next_state_id
                .checked_add(i32::from(reg.state_count))
                .ok_or_else(|| {
                    ProtocolError::InvalidData("mod block state ID space overflow".to_string())
                })?;
            blocks.push(ModBlockSpec {
                block_id: block_base + idx as i32,
                first_state_id,
                state_count: reg.state_count,
                resource_id: reg.resource_id,
                interned_id,
                light_emission: reg.light_emission,
                destroy_speed: reg.destroy_speed,
                full_cube_collision: reg.full_cube_collision,
                dynamic_light_rgb: reg.dynamic_light_rgb,
            });
        }

        let sound_base = VANILLA_SOUND_EVENT_RESOURCE_IDS.len() as i32;
        let mut sound_events = Vec::with_capacity(self.sound_events.len());
        for (idx, resource_id) in self.sound_events.into_iter().enumerate() {
            let interned_id = if intern_static {
                InternedStaticStr(intern_mod_resource_id(&resource_id)?)
            } else {
                InternedStaticStr::default()
            };
            sound_events.push(ModSoundEventSpec {
                id: sound_base + idx as i32,
                resource_id,
                interned_id,
            });
        }

        let particle_base = VANILLA_PARTICLE_TYPES.len() as i32;
        let mut particles = Vec::with_capacity(self.particles.len());
        for (idx, reg) in self.particles.into_iter().enumerate() {
            let interned_id = if intern_static {
                InternedStaticStr(intern_mod_resource_id(&reg.resource_id)?)
            } else {
                InternedStaticStr::default()
            };
            particles.push(ModParticleSpec {
                id: particle_base + idx as i32,
                resource_id: reg.resource_id,
                interned_id,
                override_limiter: reg.override_limiter,
            });
        }

        ContentRegistry::from_validated_parts(
            entities,
            items,
            blocks,
            sound_events,
            particles,
            next_state_id,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContentRegistry {
    entities: Vec<ModEntityTypeSpec>,
    items: Vec<ModItemSpec>,
    blocks: Vec<ModBlockSpec>,
    sound_events: Vec<ModSoundEventSpec>,
    particles: Vec<ModParticleSpec>,
    total_block_state_count: i32,
    #[serde(skip)]
    entity_index: HashMap<String, usize>,
    #[serde(skip)]
    item_index: HashMap<String, usize>,
    #[serde(skip)]
    block_index: HashMap<String, usize>,
    #[serde(skip)]
    sound_index: HashMap<String, usize>,
    #[serde(skip)]
    particle_index: HashMap<String, usize>,
}

#[derive(Deserialize)]
struct ContentRegistryWireSerde {
    entities: Vec<ModEntityTypeSpec>,
    items: Vec<ModItemSpec>,
    blocks: Vec<ModBlockSpec>,
    sound_events: Vec<ModSoundEventSpec>,
    particles: Vec<ModParticleSpec>,
}

impl<'de> Deserialize<'de> for ContentRegistry {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ContentRegistryWireSerde::deserialize(deserializer)?;
        let mut builder = ContentRegistryBuilder::new();
        for e in wire.entities {
            builder.register_entity(ModEntityRegistration {
                resource_id: e.resource_id,
                width: e.width,
                height: e.height,
                eye_height: e.eye_height,
                client_tracking_range: e.client_tracking_range,
                update_interval: e.update_interval,
                category: e.category,
                is_living: e.is_living,
                is_mob: e.is_mob,
                fire_immune: e.fire_immune,
                can_summon: e.can_summon,
                serializable: e.serializable,
                allowed_in_peaceful: e.allowed_in_peaceful,
                default_attributes: e.default_attributes,
                synched_fields: e.synched_fields,
                ambient_sound: e.ambient_sound,
                hurt_sound: e.hurt_sound,
                death_sound: e.death_sound,
                default_loot_table: e.default_loot_table,
                xp_reward: e.xp_reward,
                rig_id: e.rig_id,
                dynamic_light_rgb: e.dynamic_light_rgb,
                boss_bar: e.boss_bar,
            });
        }
        for item in wire.items {
            builder.register_item(ModItemRegistration {
                resource_id: item.resource_id,
                max_stack_size: item.max_stack_size,
                is_block_item: item.is_block_item,
            });
        }
        for block in wire.blocks {
            builder.register_block(ModBlockRegistration {
                resource_id: block.resource_id,
                state_count: block.state_count,
                light_emission: block.light_emission,
                destroy_speed: block.destroy_speed,
                full_cube_collision: block.full_cube_collision,
                dynamic_light_rgb: block.dynamic_light_rgb,
            });
        }
        for sound in wire.sound_events {
            builder.register_sound_event(sound.resource_id);
        }
        for particle in wire.particles {
            builder.register_particle(ModParticleRegistration {
                resource_id: particle.resource_id,
                override_limiter: particle.override_limiter,
            });
        }
        builder
            .freeze_for_session()
            .map_err(serde::de::Error::custom)
    }
}

impl Default for ContentRegistry {
    fn default() -> Self {
        Self::empty()
    }
}

impl ContentRegistry {
    fn empty() -> Self {
        Self {
            entities: Vec::new(),
            items: Vec::new(),
            blocks: Vec::new(),
            sound_events: Vec::new(),
            particles: Vec::new(),
            total_block_state_count: VANILLA_BLOCK_STATE_COUNT,
            entity_index: HashMap::new(),
            item_index: HashMap::new(),
            block_index: HashMap::new(),
            sound_index: HashMap::new(),
            particle_index: HashMap::new(),
        }
    }

    pub fn vanilla() -> &'static Arc<Self> {
        static VANILLA: OnceLock<Arc<ContentRegistry>> = OnceLock::new();
        VANILLA.get_or_init(|| Arc::new(Self::empty()))
    }

    pub fn is_vanilla_only(&self) -> bool {
        self.entities.is_empty()
            && self.items.is_empty()
            && self.blocks.is_empty()
            && self.sound_events.is_empty()
            && self.particles.is_empty()
    }

    pub fn entity_base_id() -> i32 {
        VANILLA_ENTITY_TYPES.len() as i32
    }

    pub fn item_base_id() -> i32 {
        VANILLA_ITEM_TYPES.len() as i32
    }

    pub fn block_base_id() -> i32 {
        VANILLA_BLOCK_TYPES.len() as i32
    }

    pub fn sound_event_base_id() -> i32 {
        VANILLA_SOUND_EVENT_RESOURCE_IDS.len() as i32
    }

    pub fn particle_base_id() -> i32 {
        VANILLA_PARTICLE_TYPES.len() as i32
    }

    pub fn entities(&self) -> &[ModEntityTypeSpec] {
        &self.entities
    }

    pub fn items(&self) -> &[ModItemSpec] {
        &self.items
    }

    pub fn blocks(&self) -> &[ModBlockSpec] {
        &self.blocks
    }

    pub fn sound_events(&self) -> &[ModSoundEventSpec] {
        &self.sound_events
    }

    pub fn particles(&self) -> &[ModParticleSpec] {
        &self.particles
    }

    pub fn total_entity_count(&self) -> usize {
        VANILLA_ENTITY_TYPES.len() + self.entities.len()
    }

    pub fn total_item_count(&self) -> usize {
        VANILLA_ITEM_TYPES.len() + self.items.len()
    }

    pub fn total_block_count(&self) -> usize {
        VANILLA_BLOCK_TYPES.len() + self.blocks.len()
    }

    pub fn total_block_state_count(&self) -> i32 {
        self.total_block_state_count
    }

    pub fn total_sound_event_count(&self) -> usize {
        VANILLA_SOUND_EVENT_RESOURCE_IDS.len() + self.sound_events.len()
    }

    pub fn total_particle_count(&self) -> usize {
        VANILLA_PARTICLE_TYPES.len() + self.particles.len()
    }

    pub fn entity_by_id(&self, id: i32) -> Option<&ModEntityTypeSpec> {
        let idx = usize::try_from(id.checked_sub(Self::entity_base_id())?).ok()?;
        self.entities.get(idx)
    }

    pub fn entity_by_resource_id(&self, resource_id: &str) -> Option<&ModEntityTypeSpec> {
        let idx = *self.entity_index.get(resource_id)?;
        self.entities.get(idx)
    }

    pub fn item_by_id(&self, id: i32) -> Option<&ModItemSpec> {
        let idx = usize::try_from(id.checked_sub(Self::item_base_id())?).ok()?;
        self.items.get(idx)
    }

    pub fn item_by_resource_id(&self, resource_id: &str) -> Option<&ModItemSpec> {
        let idx = *self.item_index.get(resource_id)?;
        self.items.get(idx)
    }

    pub fn block_by_id(&self, block_id: i32) -> Option<&ModBlockSpec> {
        let idx = usize::try_from(block_id.checked_sub(Self::block_base_id())?).ok()?;
        self.blocks.get(idx)
    }

    pub fn block_by_resource_id(&self, resource_id: &str) -> Option<&ModBlockSpec> {
        let idx = *self.block_index.get(resource_id)?;
        self.blocks.get(idx)
    }

    pub fn block_by_state_id(&self, state_id: i32) -> Option<(&ModBlockSpec, u16)> {
        if state_id < VANILLA_BLOCK_STATE_COUNT || state_id >= self.total_block_state_count {
            return None;
        }
        let pos = self
            .blocks
            .partition_point(|b| b.first_state_id <= state_id)
            .checked_sub(1)?;
        let spec = &self.blocks[pos];
        let offset = state_id - spec.first_state_id;
        u16::try_from(offset)
            .ok()
            .filter(|&o| o < spec.state_count)
            .map(|o| (spec, o))
    }

    pub fn sound_event_by_id(&self, id: i32) -> Option<&ModSoundEventSpec> {
        let idx = usize::try_from(id.checked_sub(Self::sound_event_base_id())?).ok()?;
        self.sound_events.get(idx)
    }

    pub fn sound_event_by_resource_id(&self, resource_id: &str) -> Option<&ModSoundEventSpec> {
        let idx = *self.sound_index.get(resource_id)?;
        self.sound_events.get(idx)
    }

    pub fn particle_by_id(&self, id: i32) -> Option<&ModParticleSpec> {
        let idx = usize::try_from(id.checked_sub(Self::particle_base_id())?).ok()?;
        self.particles.get(idx)
    }

    pub fn particle_by_resource_id(&self, resource_id: &str) -> Option<&ModParticleSpec> {
        let idx = *self.particle_index.get(resource_id)?;
        self.particles.get(idx)
    }

    pub fn encode_wire(&self) -> Vec<u8> {
        let mut encoder = Encoder::new();
        encoder.write_bytes(&WIRE_MAGIC);
        encoder.write_var_i32(CONTENT_REGISTRY_WIRE_VERSION as i32);

        encoder.write_var_i32(self.entities.len() as i32);
        for entity in &self.entities {
            encoder.write_string(&entity.resource_id);
            encoder.write_f32(entity.width);
            encoder.write_f32(entity.height);
            encoder.write_f32(entity.eye_height);
            encoder.write_var_i32(entity.client_tracking_range);
            encoder.write_var_i32(entity.update_interval);
            encoder.write_u8(encode_mob_category(entity.category));
            let flags = u8::from(entity.is_living)
                | (u8::from(entity.is_mob) << 1)
                | (u8::from(entity.fire_immune) << 2)
                | (u8::from(entity.can_summon) << 3)
                | (u8::from(entity.serializable) << 4)
                | (u8::from(entity.allowed_in_peaceful) << 5);
            encoder.write_u8(flags);
            encoder.write_var_i32(entity.default_attributes.len() as i32);
            for &(attr_id, value) in &entity.default_attributes {
                encoder.write_var_i32(attr_id);
                encoder.write_f64(value);
            }
            encoder.write_var_i32(entity.synched_fields.len() as i32);
            for field in &entity.synched_fields {
                encoder.write_u8(field.data_id);
                encoder.write_var_i32(field.serializer_id);
            }
            write_opt_string(&mut encoder, entity.ambient_sound.as_deref());
            write_opt_string(&mut encoder, entity.hurt_sound.as_deref());
            write_opt_string(&mut encoder, entity.death_sound.as_deref());
            write_opt_string(&mut encoder, entity.default_loot_table.as_deref());
            encoder.write_var_i32(entity.xp_reward);
            write_opt_string(&mut encoder, entity.rig_id.as_deref());
            write_opt_rgb(&mut encoder, entity.dynamic_light_rgb);
            encoder.write_bool(entity.boss_bar.is_some());
            if let Some(bar) = &entity.boss_bar {
                encoder.write_var_i32(bar.color_id);
                encoder.write_var_i32(bar.overlay_id);
                let bar_flags = u8::from(bar.darken_screen)
                    | (u8::from(bar.play_boss_music) << 1)
                    | (u8::from(bar.create_world_fog) << 2);
                encoder.write_u8(bar_flags);
            }
        }

        encoder.write_var_i32(self.items.len() as i32);
        for item in &self.items {
            encoder.write_string(&item.resource_id);
            encoder.write_u8(item.max_stack_size);
            encoder.write_bool(item.is_block_item);
        }

        encoder.write_var_i32(self.blocks.len() as i32);
        for block in &self.blocks {
            encoder.write_string(&block.resource_id);
            encoder.write_u16(block.state_count);
            encoder.write_u8(block.light_emission);
            encoder.write_f32(block.destroy_speed);
            encoder.write_bool(block.full_cube_collision);
            write_opt_rgb(&mut encoder, block.dynamic_light_rgb);
        }

        encoder.write_var_i32(self.sound_events.len() as i32);
        for sound in &self.sound_events {
            encoder.write_string(&sound.resource_id);
        }

        encoder.write_var_i32(self.particles.len() as i32);
        for particle in &self.particles {
            encoder.write_string(&particle.resource_id);
            encoder.write_bool(particle.override_limiter);
        }

        encoder.into_inner()
    }

    pub fn decode_wire(bytes: &[u8]) -> Result<Self> {
        let mut decoder = Decoder::new(bytes);
        if decoder.read_exact(4, "content registry magic")? != WIRE_MAGIC {
            return Err(ProtocolError::InvalidData(
                "invalid content registry magic header".to_string(),
            ));
        }
        let version = decoder.read_var_i32()?;
        if version != CONTENT_REGISTRY_WIRE_VERSION as i32 {
            return Err(ProtocolError::InvalidData(format!(
                "unrecognized content registry wire version {version}"
            )));
        }

        let mut builder = ContentRegistryBuilder::new();

        let entity_count =
            read_bounded_count(&mut decoder, MAX_MOD_ENTRIES_PER_KIND, "mod entities")?;
        for _ in 0..entity_count {
            let resource_id = decoder.read_string(MAX_RESOURCE_STRING_BYTES)?;
            let width = decoder.read_f32()?;
            let height = decoder.read_f32()?;
            let eye_height = decoder.read_f32()?;
            let client_tracking_range = decoder.read_var_i32()?;
            let update_interval = decoder.read_var_i32()?;
            let category = decode_mob_category(decoder.read_u8()?)?;
            let flags = decoder.read_u8()?;
            let attr_count = read_bounded_count(
                &mut decoder,
                MAX_DEFAULT_ATTRIBUTES_PER_ENTITY,
                "entity default attributes",
            )?;
            let mut default_attributes = Vec::with_capacity(attr_count);
            for _ in 0..attr_count {
                let attr_id = decoder.read_var_i32()?;
                let value = decoder.read_f64()?;
                default_attributes.push((attr_id, value));
            }
            let synched_count = read_bounded_count(
                &mut decoder,
                MAX_CUSTOM_SYNCHED_FIELD_COUNT,
                "entity synched fields",
            )?;
            let mut synched_fields = Vec::with_capacity(synched_count);
            for _ in 0..synched_count {
                synched_fields.push(ModSynchedFieldSpec {
                    data_id: decoder.read_u8()?,
                    serializer_id: decoder.read_var_i32()?,
                });
            }
            let ambient_sound = read_opt_string(&mut decoder)?;
            let hurt_sound = read_opt_string(&mut decoder)?;
            let death_sound = read_opt_string(&mut decoder)?;
            let default_loot_table = read_opt_string(&mut decoder)?;
            let xp_reward = decoder.read_var_i32()?;
            let rig_id = read_opt_string(&mut decoder)?;
            let dynamic_light_rgb = read_opt_rgb(&mut decoder)?;
            let boss_bar = if decoder.read_bool()? {
                let color_id = decoder.read_var_i32()?;
                let overlay_id = decoder.read_var_i32()?;
                let bar_flags = decoder.read_u8()?;
                Some(ModBossBarSpec {
                    color_id,
                    overlay_id,
                    darken_screen: (bar_flags & 1) != 0,
                    play_boss_music: (bar_flags & 2) != 0,
                    create_world_fog: (bar_flags & 4) != 0,
                })
            } else {
                None
            };
            builder.register_entity(ModEntityRegistration {
                resource_id,
                width,
                height,
                eye_height,
                client_tracking_range,
                update_interval,
                category,
                is_living: (flags & 1) != 0,
                is_mob: (flags & 2) != 0,
                fire_immune: (flags & 4) != 0,
                can_summon: (flags & 8) != 0,
                serializable: (flags & 16) != 0,
                allowed_in_peaceful: (flags & 32) != 0,
                default_attributes,
                synched_fields,
                ambient_sound,
                hurt_sound,
                death_sound,
                default_loot_table,
                xp_reward,
                rig_id,
                dynamic_light_rgb,
                boss_bar,
            });
        }

        let item_count = read_bounded_count(&mut decoder, MAX_MOD_ENTRIES_PER_KIND, "mod items")?;
        for _ in 0..item_count {
            let resource_id = decoder.read_string(MAX_RESOURCE_STRING_BYTES)?;
            let max_stack_size = decoder.read_u8()?;
            let is_block_item = decoder.read_bool()?;
            builder.register_item(ModItemRegistration {
                resource_id,
                max_stack_size,
                is_block_item,
            });
        }

        let block_count = read_bounded_count(&mut decoder, MAX_MOD_ENTRIES_PER_KIND, "mod blocks")?;
        for _ in 0..block_count {
            let resource_id = decoder.read_string(MAX_RESOURCE_STRING_BYTES)?;
            let state_count = decoder.read_u16()?;
            let light_emission = decoder.read_u8()?;
            let destroy_speed = decoder.read_f32()?;
            let full_cube_collision = decoder.read_bool()?;
            let dynamic_light_rgb = read_opt_rgb(&mut decoder)?;
            builder.register_block(ModBlockRegistration {
                resource_id,
                state_count,
                light_emission,
                destroy_speed,
                full_cube_collision,
                dynamic_light_rgb,
            });
        }

        let sound_count = read_bounded_count(&mut decoder, MAX_MOD_ENTRIES_PER_KIND, "mod sounds")?;
        for _ in 0..sound_count {
            builder.register_sound_event(decoder.read_string(MAX_RESOURCE_STRING_BYTES)?);
        }

        let particle_count =
            read_bounded_count(&mut decoder, MAX_MOD_ENTRIES_PER_KIND, "mod particles")?;
        for _ in 0..particle_count {
            let resource_id = decoder.read_string(MAX_RESOURCE_STRING_BYTES)?;
            let override_limiter = decoder.read_bool()?;
            builder.register_particle(ModParticleRegistration {
                resource_id,
                override_limiter,
            });
        }

        if !decoder.is_empty() {
            return Err(ProtocolError::InvalidData(
                "trailing bytes after content registry payload".to_string(),
            ));
        }

        builder.freeze_for_session()
    }

    fn from_validated_parts(
        entities: Vec<ModEntityTypeSpec>,
        items: Vec<ModItemSpec>,
        blocks: Vec<ModBlockSpec>,
        sound_events: Vec<ModSoundEventSpec>,
        particles: Vec<ModParticleSpec>,
        total_block_state_count: i32,
    ) -> Result<Self> {
        let entity_index = build_unique_index(
            entities
                .iter()
                .enumerate()
                .map(|(i, e)| (e.resource_id.clone(), i)),
            "mod entity",
        )?;
        let item_index = build_unique_index(
            items
                .iter()
                .enumerate()
                .map(|(i, e)| (e.resource_id.clone(), i)),
            "mod item",
        )?;
        let block_index = build_unique_index(
            blocks
                .iter()
                .enumerate()
                .map(|(i, e)| (e.resource_id.clone(), i)),
            "mod block",
        )?;
        let sound_index = build_unique_index(
            sound_events
                .iter()
                .enumerate()
                .map(|(i, e)| (e.resource_id.clone(), i)),
            "mod sound event",
        )?;
        let particle_index = build_unique_index(
            particles
                .iter()
                .enumerate()
                .map(|(i, e)| (e.resource_id.clone(), i)),
            "mod particle",
        )?;

        Ok(Self {
            entities,
            items,
            blocks,
            sound_events,
            particles,
            total_block_state_count,
            entity_index,
            item_index,
            block_index,
            sound_index,
            particle_index,
        })
    }
}

fn build_unique_index(
    entries: impl Iterator<Item = (String, usize)>,
    kind: &'static str,
) -> Result<HashMap<String, usize>> {
    let mut index = HashMap::new();
    for (resource_id, pos) in entries {
        if index.insert(resource_id.clone(), pos).is_some() {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate {kind} resource id `{resource_id}`"
            )));
        }
    }
    Ok(index)
}

fn check_collection_limit(len: usize, what: &'static str) -> Result<()> {
    if len > MAX_MOD_ENTRIES_PER_KIND {
        return Err(ProtocolError::PacketTooLarge(len, MAX_MOD_ENTRIES_PER_KIND));
    }
    let _ = what;
    Ok(())
}

fn validate_entity_registration(entity: &mut ModEntityRegistration) -> Result<()> {
    if !entity.width.is_finite()
        || !entity.height.is_finite()
        || !entity.eye_height.is_finite()
        || entity.width <= 0.0
        || entity.height <= 0.0
        || entity.eye_height < 0.0
    {
        return Err(ProtocolError::InvalidData(format!(
            "mod entity `{}` has invalid dimensions",
            entity.resource_id
        )));
    }
    if entity.client_tracking_range < 0 || entity.update_interval < 1 || entity.xp_reward < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "mod entity `{}` has invalid tracking/xp parameters",
            entity.resource_id
        )));
    }
    if entity.default_attributes.len() > MAX_DEFAULT_ATTRIBUTES_PER_ENTITY {
        return Err(ProtocolError::PacketTooLarge(
            entity.default_attributes.len(),
            MAX_DEFAULT_ATTRIBUTES_PER_ENTITY,
        ));
    }
    let mut seen_attrs = HashSet::new();
    for &(attr_id, value) in &entity.default_attributes {
        if attr_id < 0
            || (attr_id as usize) >= VANILLA_ATTRIBUTE_TYPES.len()
            || !value.is_finite()
            || !seen_attrs.insert(attr_id)
        {
            return Err(ProtocolError::InvalidData(format!(
                "mod entity `{}` has invalid attribute ({attr_id}, {value})",
                entity.resource_id
            )));
        }
    }
    if entity.synched_fields.len() > MAX_CUSTOM_SYNCHED_FIELD_COUNT {
        return Err(ProtocolError::PacketTooLarge(
            entity.synched_fields.len(),
            MAX_CUSTOM_SYNCHED_FIELD_COUNT,
        ));
    }
    let mut seen_data_ids = HashSet::new();
    for field in &entity.synched_fields {
        if !(16..=254).contains(&field.data_id) {
            return Err(ProtocolError::InvalidData(format!(
                "mod entity `{}` synched data_id {} must be in 16..=254",
                entity.resource_id, field.data_id
            )));
        }
        if !((0..=42).contains(&field.serializer_id)
            || field.serializer_id == MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID)
        {
            return Err(ProtocolError::InvalidData(format!(
                "mod entity `{}` synched serializer_id {} is invalid",
                entity.resource_id, field.serializer_id
            )));
        }
        if !seen_data_ids.insert(field.data_id) {
            return Err(ProtocolError::InvalidData(format!(
                "mod entity `{}` duplicates synched data_id {}",
                entity.resource_id, field.data_id
            )));
        }
    }
    if let Some(rgb) = entity.dynamic_light_rgb {
        validate_rgb(rgb, &entity.resource_id)?;
    }
    for val in [
        &mut entity.ambient_sound,
        &mut entity.hurt_sound,
        &mut entity.death_sound,
        &mut entity.default_loot_table,
        &mut entity.rig_id,
    ]
    .into_iter()
    .flatten()
    {
        *val = normalize_resource_location(val.as_str())?;
    }
    Ok(())
}

fn validate_block_registration(block: &ModBlockRegistration) -> Result<()> {
    if block.state_count == 0 {
        return Err(ProtocolError::InvalidData(format!(
            "mod block `{}` must declare at least 1 block state",
            block.resource_id
        )));
    }
    if block.light_emission > 15 || !block.destroy_speed.is_finite() {
        return Err(ProtocolError::InvalidData(format!(
            "mod block `{}` has invalid light_emission or destroy_speed",
            block.resource_id
        )));
    }
    if let Some(rgb) = block.dynamic_light_rgb {
        validate_rgb(rgb, &block.resource_id)?;
    }
    Ok(())
}

fn validate_rgb(rgb: [f32; 3], resource_id: &str) -> Result<()> {
    if !rgb.iter().all(|c| c.is_finite() && *c >= 0.0) {
        return Err(ProtocolError::InvalidData(format!(
            "mod entry `{resource_id}` has invalid dynamic_light_rgb"
        )));
    }
    Ok(())
}

fn read_bounded_count(decoder: &mut Decoder<'_>, max: usize, _what: &'static str) -> Result<usize> {
    let count = decoder.read_len()?;
    if count > max {
        return Err(ProtocolError::PacketTooLarge(count, max));
    }
    Ok(count)
}

fn write_opt_string(encoder: &mut Encoder, value: Option<&str>) {
    encoder.write_bool(value.is_some());
    if let Some(s) = value {
        encoder.write_string(s);
    }
}

fn read_opt_string(decoder: &mut Decoder<'_>) -> Result<Option<String>> {
    if decoder.read_bool()? {
        Ok(Some(decoder.read_string(MAX_RESOURCE_STRING_BYTES)?))
    } else {
        Ok(None)
    }
}

fn write_opt_rgb(encoder: &mut Encoder, rgb: Option<[f32; 3]>) {
    encoder.write_bool(rgb.is_some());
    if let Some([r, g, b]) = rgb {
        encoder.write_f32(r);
        encoder.write_f32(g);
        encoder.write_f32(b);
    }
}

fn read_opt_rgb(decoder: &mut Decoder<'_>) -> Result<Option<[f32; 3]>> {
    if decoder.read_bool()? {
        Ok(Some([
            decoder.read_f32()?,
            decoder.read_f32()?,
            decoder.read_f32()?,
        ]))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::client_common::{
        encode_play_clientbound_custom_payload, CustomPayload, CustomPayloadBody,
    };
    use crate::packets::connection::{
        decode_configuration_clientbound, encode_client_configuration_custom_payload,
        ConfigurationClientbound,
    };
    use crate::packets::entities::{
        EntityDataEnumSerializer, EntityDataValue, EntityDataValueKind, GlobalPosData,
        SetEntityData,
    };
    use crate::packets::play_clientbound::{decode_play_clientbound, PlayClientbound};
    use crate::packets::server_connection::{
        decode_configuration_serverbound, encode_configuration_custom_payload,
        ConfigurationServerbound,
    };
    use crate::packets::server_play::encode_play_clientbound_set_entity_data;
    use crate::packets::BlockPos;
    use bbb_facts::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID;
    use bbb_facts::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;

    #[test]
    fn content_registry_freeze_is_deterministic_and_round_trips_wire() {
        let mut b1 = ContentRegistryBuilder::new();
        b1.register_entity(ModEntityRegistration {
            resource_id: "bbb_demo:zeta_beast".to_string(),
            width: 1.5,
            height: 2.5,
            eye_height: 2.1,
            default_attributes: vec![(VANILLA_ATTRIBUTE_MAX_HEALTH_ID, 80.0)],
            synched_fields: vec![
                ModSynchedFieldSpec {
                    data_id: 16,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                },
                ModSynchedFieldSpec {
                    data_id: 17,
                    serializer_id: MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID,
                },
            ],
            rig_id: Some("bbb_demo:zeta_beast".to_string()),
            dynamic_light_rgb: Some([1.0, 0.5, 0.1]),
            boss_bar: Some(ModBossBarSpec {
                color_id: 2,
                overlay_id: 1,
                darken_screen: true,
                play_boss_music: false,
                create_world_fog: false,
            }),
            ..Default::default()
        });
        b1.register_entity(ModEntityRegistration {
            resource_id: "alpha_mod:alpha_imp".to_string(),
            ..Default::default()
        });

        let mut b2 = ContentRegistryBuilder::new();
        b2.register_entity(ModEntityRegistration {
            resource_id: "alpha_mod:alpha_imp".to_string(),
            ..Default::default()
        });
        b2.register_entity(ModEntityRegistration {
            resource_id: "bbb_demo:zeta_beast".to_string(),
            width: 1.5,
            height: 2.5,
            eye_height: 2.1,
            default_attributes: vec![(VANILLA_ATTRIBUTE_MAX_HEALTH_ID, 80.0)],
            synched_fields: vec![
                ModSynchedFieldSpec {
                    data_id: 16,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                },
                ModSynchedFieldSpec {
                    data_id: 17,
                    serializer_id: MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID,
                },
            ],
            rig_id: Some("bbb_demo:zeta_beast".to_string()),
            dynamic_light_rgb: Some([1.0, 0.5, 0.1]),
            boss_bar: Some(ModBossBarSpec {
                color_id: 2,
                overlay_id: 1,
                darken_screen: true,
                play_boss_music: false,
                create_world_fog: false,
            }),
            ..Default::default()
        });

        let r1 = b1.freeze().expect("valid freeze");
        let r2 = b2.freeze().expect("valid freeze");
        assert_eq!(r1.entities().len(), 2);
        assert_eq!(r1.entities()[0].resource_id, "alpha_mod:alpha_imp");
        assert_eq!(r1.entities()[0].id, ContentRegistry::entity_base_id());
        assert_eq!(r1.entities()[1].id, ContentRegistry::entity_base_id() + 1);
        assert_eq!(r1.encode_wire(), r2.encode_wire());

        let decoded = ContentRegistry::decode_wire(&r1.encode_wire()).expect("wire round-trip");
        assert_eq!(r1, decoded);
        assert_eq!(
            decoded
                .entity_by_id(ContentRegistry::entity_base_id() + 1)
                .unwrap()
                .resource_id(),
            "bbb_demo:zeta_beast"
        );
        assert!(decoded
            .entity_by_resource_id("alpha_mod:alpha_imp")
            .is_some());
        assert!(ContentRegistry::vanilla().is_vanilla_only());
        assert!(!decoded.is_vanilla_only());
    }

    #[test]
    fn decode_wire_does_not_intern_or_exhaust_global_pool_across_sessions() {
        for session in 0..10usize {
            let mut builder = ContentRegistryBuilder::new();
            for idx in 0..4096usize {
                builder.register_item(ModItemRegistration {
                    resource_id: format!("session_wire_{session}:item_{idx}"),
                    max_stack_size: 64,
                    is_block_item: false,
                });
            }
            let wire = builder.freeze_for_session().unwrap().encode_wire();
            let decoded = ContentRegistry::decode_wire(&wire).expect("session wire decode");
            assert_eq!(
                decoded.items()[0].resource_id(),
                format!("session_wire_{session}:item_0")
            );
            with_intern_pool(|set| {
                assert!(!set.contains(format!("session_wire_{session}:item_0").as_str()));
            });
        }
    }

    #[test]
    fn content_registry_rejects_minecraft_namespace_and_reserved_synched_slots() {
        let mut bad_ns = ContentRegistryBuilder::new();
        bad_ns.register_entity(ModEntityRegistration {
            resource_id: "minecraft:custom_boss".to_string(),
            ..Default::default()
        });
        assert!(bad_ns.freeze().is_err());

        let mut bad_slot = ContentRegistryBuilder::new();
        bad_slot.register_entity(ModEntityRegistration {
            resource_id: "bbb_demo:boss".to_string(),
            synched_fields: vec![ModSynchedFieldSpec {
                data_id: 15,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            }],
            ..Default::default()
        });
        assert!(bad_slot.freeze().is_err());
    }

    #[test]
    fn bidirectional_custom_payload_and_set_entity_data_round_trip() {
        let mut builder = ContentRegistryBuilder::new();
        builder.register_entity(ModEntityRegistration {
            resource_id: "bbb_demo:warden".to_string(),
            ..Default::default()
        });
        let raw = builder.freeze().unwrap().encode_wire();

        let (cfg_s2c_id, cfg_s2c_bytes) =
            encode_configuration_custom_payload(BBB_CONTENT_REGISTRY_CHANNEL, &raw).unwrap();
        let decoded_cfg = decode_configuration_clientbound(cfg_s2c_id, &cfg_s2c_bytes).unwrap();
        match decoded_cfg {
            ConfigurationClientbound::CustomPayload(p) => {
                assert_eq!(p.id, BBB_CONTENT_REGISTRY_CHANNEL);
                assert_eq!(
                    p.payload,
                    CustomPayloadBody::Unknown {
                        raw_payload: raw.clone()
                    }
                );
            }
            other => panic!("unexpected packet {other:?}"),
        }

        let (cfg_c2s_id, cfg_c2s_bytes) =
            encode_client_configuration_custom_payload("bbb_demo:hello", b"hi").unwrap();
        match decode_configuration_serverbound(cfg_c2s_id, &cfg_c2s_bytes).unwrap() {
            ConfigurationServerbound::CustomPayload(p) => {
                assert_eq!(p.id, "bbb_demo:hello");
                assert_eq!(
                    p.payload,
                    CustomPayloadBody::Unknown {
                        raw_payload: b"hi".to_vec()
                    }
                );
            }
            other => panic!("unexpected packet {other:?}"),
        }

        let play_payload = CustomPayload {
            id: "bbb_demo:event".to_string(),
            payload: CustomPayloadBody::Unknown {
                raw_payload: vec![1, 2, 3, 4],
            },
        };
        let (play_id, play_bytes) = encode_play_clientbound_custom_payload(&play_payload).unwrap();
        match decode_play_clientbound(play_id, &play_bytes).unwrap() {
            PlayClientbound::CustomPayload(p) => assert_eq!(p, play_payload),
            other => panic!("unexpected packet {other:?}"),
        }

        let entity_data = SetEntityData {
            id: 42,
            values: vec![
                EntityDataValue {
                    data_id: 16,
                    serializer_id: bbb_facts::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_POS_ID,
                    value: EntityDataValueKind::BlockPos(BlockPos { x: 1, y: 64, z: -2 }),
                },
                EntityDataValue {
                    data_id: 17,
                    serializer_id: bbb_facts::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_GLOBAL_POS_ID,
                    value: EntityDataValueKind::OptionalGlobalPos(Some(GlobalPosData {
                        dimension: "minecraft:overworld".to_string(),
                        pos: BlockPos { x: 5, y: 70, z: 9 },
                    })),
                },
                EntityDataValue {
                    data_id: 18,
                    serializer_id: bbb_facts::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_SNIFFER_STATE_ID,
                    value: EntityDataValueKind::EnumId {
                        serializer: EntityDataEnumSerializer::SnifferState,
                        id: 3,
                    },
                },
                EntityDataValue {
                    data_id: 19,
                    serializer_id: MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID,
                    value: EntityDataValueKind::ModCustom(vec![0xaa, 0xbb, 0xcc]),
                },
            ],
        };
        let (ed_id, ed_bytes) = encode_play_clientbound_set_entity_data(&entity_data).unwrap();
        match decode_play_clientbound(ed_id, &ed_bytes).unwrap() {
            PlayClientbound::SetEntityData(decoded) => assert_eq!(decoded, entity_data),
            other => panic!("unexpected packet {other:?}"),
        }
    }

    #[test]
    fn clientbound_item_equipment_merchant_and_recipe_encoders_accept_mod_content() {
        use crate::packets::{
            decode_play_clientbound_with_content_registry,
            encode_play_clientbound_merchant_offers_with_content_registry,
            encode_play_clientbound_set_equipment_with_content_registry,
            encode_play_clientbound_set_player_inventory,
            encode_play_clientbound_set_player_inventory_with_content_registry,
            encode_play_clientbound_update_recipes_with_content_registry,
            DataComponentPatchSummary, DebugStickPropertySummary, EquipmentSlot,
            EquipmentSlotUpdate, IngredientSummary, ItemCostSummary, ItemStackSummary,
            ItemStackTemplateSummary, MerchantOffer, MerchantOffers, RecipePropertySetSummary,
            SetEquipment, SetPlayerInventory, SlotDisplaySummary, SlotDisplayValue,
            StonecutterSelectableRecipeSummary, UpdateRecipes,
        };

        let mut builder = ContentRegistryBuilder::new();
        builder.register_item(ModItemRegistration {
            resource_id: "bbb_demo:ruby".to_string(),
            max_stack_size: 64,
            is_block_item: false,
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "bbb_demo:ruby_ore".to_string(),
            state_count: 2,
            light_emission: 0,
            destroy_speed: 3.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        let registry = builder.freeze().unwrap();
        let mod_item_id = registry.item_by_resource_id("bbb_demo:ruby").unwrap().id;
        let mod_block_id = registry
            .block_by_resource_id("bbb_demo:ruby_ore")
            .unwrap()
            .block_id;

        let mut patch = DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID);
        patch.use_remainder = Some(Box::new(ItemStackTemplateSummary {
            item_id: mod_item_id,
            count: 1,
            component_patch: Default::default(),
        }));
        patch
            .added_type_ids
            .push(bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID);
        patch.pot_decorations_item_ids = vec![mod_item_id, mod_item_id, mod_item_id, mod_item_id];
        patch.added_type_ids.push(
            bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
        );
        patch.debug_stick_properties = vec![DebugStickPropertySummary {
            block_registry_id: mod_block_id,
            property: "variant".to_string(),
        }];

        let mod_stack = ItemStackSummary {
            item_id: Some(mod_item_id),
            count: 3,
            component_patch: patch.into(),
        };

        let inv = SetPlayerInventory {
            slot: 0,
            item: mod_stack.clone(),
        };
        assert!(encode_play_clientbound_set_player_inventory(&inv).is_err());
        let (id, bytes) =
            encode_play_clientbound_set_player_inventory_with_content_registry(&inv, &registry)
                .unwrap();
        assert_eq!(
            decode_play_clientbound_with_content_registry(id, &bytes, &registry).unwrap(),
            PlayClientbound::SetPlayerInventory(inv)
        );

        let equip = SetEquipment {
            entity_id: 9,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: mod_stack.clone(),
            }],
        };
        let (id, bytes) =
            encode_play_clientbound_set_equipment_with_content_registry(&equip, &registry).unwrap();
        assert_eq!(
            decode_play_clientbound_with_content_registry(id, &bytes, &registry).unwrap(),
            PlayClientbound::SetEquipment(equip)
        );

        let offers = MerchantOffers {
            container_id: 2,
            offers: vec![MerchantOffer {
                buy_a: ItemCostSummary {
                    item_id: mod_item_id,
                    count: 1,
                    component_predicate: Default::default(),
                },
                sell: mod_stack.clone(),
                buy_b: None,
                is_out_of_stock: false,
                uses: 0,
                max_uses: 8,
                xp: 5,
                special_price_diff: 0,
                price_multiplier: 0.05,
                demand: 0,
            }],
            villager_level: 1,
            villager_xp: 0,
            show_progress: true,
            can_restock: true,
        };
        let (id, bytes) =
            encode_play_clientbound_merchant_offers_with_content_registry(&offers, &registry)
                .unwrap();
        assert_eq!(
            decode_play_clientbound_with_content_registry(id, &bytes, &registry).unwrap(),
            PlayClientbound::MerchantOffers(offers)
        );

        let slot_display = SlotDisplaySummary::from_value_with_content_registry(
            SlotDisplayValue::ItemStack {
                stack: Box::new(mod_stack),
            },
            &registry,
        )
        .unwrap();
        let update_recipes = UpdateRecipes {
            property_sets: vec![RecipePropertySetSummary {
                key: "bbb_demo:gems".to_string(),
                item_ids: vec![mod_item_id],
            }],
            stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                input: IngredientSummary {
                    tag: None,
                    item_ids: vec![mod_item_id],
                },
                option_display: slot_display,
            }],
        };
        let (id, bytes) = encode_play_clientbound_update_recipes_with_content_registry(
            &update_recipes,
            &registry,
        )
        .unwrap();
        assert_eq!(
            decode_play_clientbound_with_content_registry(id, &bytes, &registry).unwrap(),
            PlayClientbound::UpdateRecipes(update_recipes)
        );
    }
}
