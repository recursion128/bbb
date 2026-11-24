//! Java 26.1 playerdata: a lenient reader over the retained raw document and
//! the writer that patches the live fields back into it.
//!
//! Reads follow `ValueInput.getXOr` / `read(..).orElse(..)`: a missing or
//! mistyped field takes the Vanilla default and the rest of the file loads.

use std::sync::OnceLock;

use bbb_protocol::{
    content_registry::ContentRegistry,
    game_types::GameType,
    packets::{BlockPos, GlobalPos, PlayerAbilities, WaypointIcon},
    NbtSummaryEntry, NbtSummaryValue,
};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use uuid::Uuid;

use crate::entity_schema::{Java26EntityRecord, Java26EntityRecordRef};

use super::schema_support::{invalid_schema, require_java_26_1_root, uuid_nbt, uuid_value};
use bbb_server_types::save::{NamedCompound, SaveResult};

pub mod document;
mod ender_items;
pub mod ender_pearls;
mod inventory;
pub use ender_items::Java26PlayerEnderItems;
pub use inventory::Java26PlayerInventory;
pub mod field_parsers;
pub mod field_patches;

#[cfg(test)]
pub mod tests;

pub use self::document::created_player_document;

use self::field_parsers::{
    bool_or, double_or, double_vec3, field, float_or, int_or, int_vec3, parse_abilities,
    parse_global_pos, parse_locator_bar_icon, parse_recipe_book, parse_respawn, parse_warden,
    resource_id, short_or,
};

const DEFAULT_LOCATOR_STYLE: &str = "minecraft:default";
/// The eight `recipeBook` flags, in `RecipeBookSettings`' own order: one
/// open/filter pair per `RecipeBookType`, in the enum's ordinal order.
const RECIPE_BOOK_SETTING_NAMES: [&str; 8] = [
    "isGuiOpen",
    "isFilteringCraftable",
    "isFurnaceGuiOpen",
    "isFurnaceFilteringCraftable",
    "isBlastingFurnaceGuiOpen",
    "isBlastingFurnaceFilteringCraftable",
    "isSmokerGuiOpen",
    "isSmokerFilteringCraftable",
];

pub(super) fn authored_recipe_id_list(ids: &[String]) -> NbtSummaryValue {
    NbtSummaryValue::List(ids.iter().cloned().map(NbtSummaryValue::string).collect())
}

#[derive(Debug, Clone, PartialEq)]
enum RetainedPlayerInputs {
    FullRaw(NamedCompound),
    ModeledInputs {
        shoulders: [Vec<NbtSummaryEntry>; 2],
        attributes: Option<NbtSummaryValue>,
        active_effects: Option<NbtSummaryValue>,
        inventory: Option<NbtSummaryValue>,
        equipment: Option<NbtSummaryValue>,
        ender_items: Option<NbtSummaryValue>,
    },
}

fn take_modeled_inputs(raw: NamedCompound) -> RetainedPlayerInputs {
    let (_root, entries) = raw.into_parts();
    let mut left_shoulder = Vec::new();
    let mut right_shoulder = Vec::new();
    let mut has_left_shoulder = false;
    let mut has_right_shoulder = false;
    let mut attributes = None;
    let mut active_effects = None;
    let mut inventory = None;
    let mut equipment = None;
    let mut ender_items = None;
    for entry in entries {
        if entry.name == "ShoulderEntityLeft" {
            if !has_left_shoulder {
                has_left_shoulder = true;
                if let NbtSummaryValue::Compound(fields) = entry.value {
                    left_shoulder = fields;
                }
            }
        } else if entry.name == "ShoulderEntityRight" {
            if !has_right_shoulder {
                has_right_shoulder = true;
                if let NbtSummaryValue::Compound(fields) = entry.value {
                    right_shoulder = fields;
                }
            }
        } else if entry.name == "attributes" && attributes.is_none() {
            attributes = Some(entry.value);
        } else if entry.name == "active_effects" && active_effects.is_none() {
            active_effects = Some(entry.value);
        } else if entry.name == "Inventory" && inventory.is_none() {
            inventory = Some(entry.value);
        } else if entry.name == "equipment" && equipment.is_none() {
            equipment = Some(entry.value);
        } else if entry.name == "EnderItems" && ender_items.is_none() {
            ender_items = Some(entry.value);
        }
    }
    RetainedPlayerInputs::ModeledInputs {
        shoulders: [left_shoulder, right_shoulder],
        attributes,
        active_effects,
        inventory,
        equipment,
        ender_items,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26PlayerData {
    retained: RetainedPlayerInputs,
    uuid: Uuid,
    dimension: String,
    position: [f64; 3],
    rotation: [f32; 2],
    fall_distance: f64,
    on_ground: bool,
    invulnerable: bool,
    fall_flying: bool,
    has_visual_fire: bool,
    silent: bool,
    glowing: bool,
    custom_name: Option<NbtSummaryValue>,
    custom_data: Vec<NbtSummaryEntry>,
    remaining_fire_ticks: i32,
    air_supply: i32,
    ticks_frozen: i32,
    portal_cooldown: i32,
    health: Option<f32>,
    absorption_amount: f32,
    hurt_time: i16,
    death_time: i16,
    impulse_reset_grace_time: i32,
    impulse_impact_position: Option<bbb_protocol::packets::Vec3d>,
    locator_bar_icon: WaypointIcon,
    selected_item_slot: u8,
    abilities: Java26PlayerAbilities,
    food: bbb_server_types::food::FoodData,
    experience: Java26PlayerExperience,
    game_mode: Option<GameType>,
    previous_game_mode: Option<GameType>,
    entered_nether_pos: Option<[f64; 3]>,
    raid_omen_position: Option<[i32; 3]>,
    recipe_book: Java26PlayerRecipeBook,
    respawn: Option<Java26PlayerRespawn>,
    last_death_location: Option<GlobalPos>,
    warden_spawn_tracker: Java26WardenSpawnTracker,
    sleep_timer: i16,
    sleeping_pos: Option<[i32; 3]>,
    last_hurt_by: Java26PlayerLastHurtBy,
    spawn_extra_particles_on_fall: bool,
    seen_credits: bool,
    root_vehicle: Option<Java26RootVehicle>,
    ender_pearls: Vec<ender_pearls::Java26PlayerEnderPearl>,
    tags: Vec<bbb_protocol::JavaString>,
}

/// `LivingEntity`'s attacker memory (`LivingEntity.java:726,739-747`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Java26PlayerLastHurtBy {
    /// `lastHurtByMobTimestamp`, written as `HurtByTimestamp`.
    pub hurt_by_timestamp: i32,
    /// `last_hurt_by_player` and `last_hurt_by_player_memory_time`.
    pub player: Option<(Uuid, i32)>,
    /// `last_hurt_by_mob` and `ticks_since_last_hurt_by_mob`.
    pub mob: Option<(Uuid, i32)>,
}

fn parse_last_hurt_by(entries: &[NbtSummaryEntry]) -> Java26PlayerLastHurtBy {
    let reference = |name| field(entries, name).and_then(uuid_value);
    Java26PlayerLastHurtBy {
        hurt_by_timestamp: int_or(entries, "HurtByTimestamp", 0),
        player: reference("last_hurt_by_player")
            .map(|uuid| (uuid, int_or(entries, "last_hurt_by_player_memory_time", 0))),
        mob: reference("last_hurt_by_mob")
            .map(|uuid| (uuid, int_or(entries, "ticks_since_last_hurt_by_mob", 0))),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26RootVehicle {
    attach: Option<NbtSummaryValue>,
    entity: Java26EntityRecord,
}

/// A `RootVehicle` whose entity tree cannot be read loads as no vehicle.
fn parse_root_vehicle(
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Option<Java26RootVehicle> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let Some(NbtSummaryValue::Compound(entity)) = field(entries, "Entity") else {
        return None;
    };
    let entity = Java26EntityRecord::parse_with_content_registry(entity, content_registry)?;
    Some(Java26RootVehicle {
        attach: field(entries, "Attach").cloned(),
        entity,
    })
}

impl Java26RootVehicle {
    pub fn new(attach: Uuid, entity: Java26EntityRecord) -> Self {
        Self {
            attach: Some(uuid_nbt(attach)),
            entity,
        }
    }

    pub fn attach(&self) -> SaveResult<Uuid> {
        self.attach
            .as_ref()
            .and_then(uuid_value)
            .ok_or_else(|| invalid_schema("player.RootVehicle.Attach", "is not a UUID"))
    }

    pub fn attach_input(&self) -> Option<&NbtSummaryValue> {
        self.attach.as_ref()
    }

    pub fn entity(&self) -> Java26EntityRecordRef<'_> {
        self.entity.as_ref()
    }

    pub fn entity_entries(&self) -> &[NbtSummaryEntry] {
        self.entity.as_ref().raw()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26PlayerAbilities {
    invulnerable: bool,
    flying: bool,
    may_fly: bool,
    instant_build: bool,
    may_build: bool,
    fly_speed: f32,
    walk_speed: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26PlayerExperience {
    progress: f32,
    level: i32,
    total: i32,
    seed: i32,
    score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Java26PlayerRecipeBook {
    recipes: Vec<String>,
    to_be_displayed: Vec<String>,
    settings: [bool; 8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26PlayerRespawn {
    global_pos: GlobalPos,
    yaw: f32,
    pitch: f32,
    forced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26WardenSpawnTracker {
    ticks_since_last_warning: i32,
    warning_level: i32,
    cooldown_ticks: i32,
}

/// The live player state a playerdata document is written from.
#[derive(Debug, Clone, PartialEq)]
pub struct Java26CreatedPlayer {
    pub sleep_timer: i16,
    pub sleeping_pos: Option<[i32; 3]>,
    pub last_hurt_by: Java26PlayerLastHurtBy,
    pub dimension: String,
    pub game_mode: GameType,
    /// `Entity.invulnerable`, persisted independently from the abilities bit.
    pub entity_invulnerable: bool,
    pub has_visual_fire: bool,
    pub silent: bool,
    /// `Entity.hasGlowingTag` (`Entity.java:2004-2006`, `:2085`).
    pub glowing: bool,
    pub fall_distance: f64,
    pub remaining_fire_ticks: i32,
    pub air_supply: i32,
    pub ticks_frozen: i32,
    pub portal_cooldown: i32,
    pub spawn_extra_particles_on_fall: bool,
    /// `ServerPlayer.seenCredits` (`ServerPlayer.java:389`, `:410`).
    pub seen_credits: bool,
    pub impulse_reset_grace_time: i32,
    pub impulse_impact_position: Option<bbb_protocol::packets::Vec3d>,
    pub abilities: PlayerAbilities,
    /// Storage slots 0..35 followed by the seven equipment-backed inventory
    /// slots 36..42 (`Inventory.java:28-53`).
    pub inventory: Java26PlayerInventory,
    pub ender_items: Java26PlayerEnderItems,
    pub fall_flying: bool,
    pub selected_item_slot: u8,
    /// The eight `recipeBook` flags in [`RECIPE_BOOK_SETTING_NAMES`] order.
    pub recipe_book_settings: [bool; 8],
    pub recipe_book_recipes: Vec<String>,
    pub recipe_book_to_be_displayed: Vec<String>,
    pub damage: bbb_server_types::player_damage::PlayerDamageState,
    pub food: bbb_server_types::food::FoodData,
    pub experience: bbb_server_types::experience::PlayerExperience,
    /// `AttributeInstance.Packed.LIST_CODEC` output, encoded above this crate.
    pub attributes: Vec<NbtSummaryValue>,
    /// `MobEffectInstance.CODEC.listOf()` output, written only when non-empty.
    pub active_effects: Vec<NbtSummaryValue>,
    pub locator_bar_icon: WaypointIcon,
}

/// One player's contribution to a save: the live fields patched over the
/// retained document, or over a fresh one for a player the save never held.
#[derive(Debug, Clone, PartialEq)]
pub struct Java26PlayerSavePatch {
    pub uuid: Uuid,
    pub position: [f64; 3],
    pub motion: [f64; 3],
    pub rotation: [f32; 2],
    pub on_ground: bool,
    /// `None` removes a previous name (`Entity.java:1991`).
    pub custom_name: Option<NbtSummaryValue>,
    pub custom_data: Vec<NbtSummaryEntry>,
    pub tags: Vec<bbb_protocol::JavaString>,
    /// `None` removes a retained `RootVehicle`.
    pub root_vehicle: Option<Java26RootVehicle>,
    pub ender_pearls: Vec<ender_pearls::Java26PlayerEnderPearl>,
    pub shoulder_entities: [Vec<NbtSummaryEntry>; 2],
    /// `None` removes a retained `respawn`.
    pub respawn: Option<Java26PlayerRespawn>,
    /// `Player.lastDeathLocation` (`Player.java:660`); `None` removes it.
    pub last_death_location: Option<GlobalPos>,
    /// `ServerPlayer.java:2038-2042`; `None` removes it.
    pub previous_game_mode: Option<GameType>,
    /// `ServerPlayer.java:411`; `None` removes it.
    pub entered_nether_pos: Option<[f64; 3]>,
    /// `ServerPlayer.java:417`; `None` removes it.
    pub raid_omen_position: Option<[i32; 3]>,
    pub created: Java26CreatedPlayer,
    /// The session's tracker (`ServerPlayer.java:408`); `None` keeps the
    /// document's own, or a fresh one for a created document.
    pub warden_spawn_tracker: Option<Java26WardenSpawnTracker>,
    /// `ServerStatsCounter`, written to its own file (`PlayerList.java:288-291`).
    pub stats: bbb_server_types::stats::ServerStatsCounter,
}

impl Java26PlayerData {
    pub fn parse(raw: NamedCompound, uuid: Uuid) -> SaveResult<Self> {
        Self::parse_with_content_registry(raw, uuid, ContentRegistry::vanilla())
    }

    pub fn parse_with_content_registry(
        raw: NamedCompound,
        uuid: Uuid,
        content_registry: &ContentRegistry,
    ) -> SaveResult<Self> {
        Self::parse_inner(raw, uuid, content_registry, true)
    }

    pub(crate) fn parse_candidate_with_content_registry(
        raw: NamedCompound,
        uuid: Uuid,
        content_registry: &ContentRegistry,
    ) -> SaveResult<Self> {
        Self::parse_inner(raw, uuid, content_registry, false)
    }

    fn parse_inner(
        raw: NamedCompound,
        uuid: Uuid,
        content_registry: &ContentRegistry,
        retain_raw: bool,
    ) -> SaveResult<Self> {
        let entries = require_java_26_1_root(&raw, "player")?;
        let position = double_vec3(field(entries, "Pos"))
            .filter(|position| position.iter().all(|value| value.is_finite()))
            .unwrap_or([0.0; 3]);
        let rotation = match field(entries, "Rotation") {
            Some(NbtSummaryValue::List(values)) if values.len() == 2 => {
                let mut rotation = [0.0; 2];
                for (slot, value) in rotation.iter_mut().zip(values) {
                    if let NbtSummaryValue::Float(bits) = value {
                        let angle = f32::from_bits(*bits);
                        if angle.is_finite() {
                            *slot = angle;
                        }
                    }
                }
                rotation
            }
            _ => [0.0; 2],
        };
        let custom_data = match field(entries, "data") {
            Some(NbtSummaryValue::Compound(value)) => value.clone(),
            Some(NbtSummaryValue::String(value)) => {
                match bbb_protocol::parse_snbt_compound_utf16(value.units()) {
                    Ok(NbtSummaryValue::Compound(entries)) => entries,
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        };
        // Entity.java:2081: a malformed component reads as no custom name.
        let custom_name = field(entries, "CustomName")
            .filter(|value| {
                bbb_protocol::validate_component_nbt(value, "player.CustomName").is_ok()
            })
            .cloned();
        let mut tags = Vec::new();
        if let Some(NbtSummaryValue::List(values)) = field(entries, "Tags") {
            for value in values {
                if let NbtSummaryValue::String(tag) = value {
                    if !tags.contains(tag) {
                        tags.push(tag.clone());
                    }
                }
            }
        }
        // Inventory.setSelectedSlot accepts only hotbar slots.
        let selected_item_slot = u8::try_from(int_or(entries, "SelectedItemSlot", 0))
            .ok()
            .filter(|slot| *slot < 9)
            .unwrap_or(0);
        let sleeping_pos = match field(entries, "sleeping_pos") {
            Some(NbtSummaryValue::IntArray(values)) => <[i32; 3]>::try_from(values.as_slice()).ok(),
            _ => None,
        };
        Ok(Self {
            uuid,
            dimension: field(entries, "Dimension")
                .and_then(resource_id)
                .unwrap_or_else(|| "minecraft:overworld".to_string()),
            position,
            rotation,
            fall_distance: double_or(entries, "fall_distance", 0.0),
            on_ground: bool_or(entries, "OnGround", false),
            invulnerable: bool_or(entries, "Invulnerable", false),
            fall_flying: bool_or(entries, "FallFlying", false),
            has_visual_fire: bool_or(entries, "HasVisualFire", false),
            silent: bool_or(entries, "Silent", false),
            glowing: bool_or(entries, "Glowing", false),
            custom_name,
            custom_data,
            remaining_fire_ticks: i32::from(short_or(entries, "Fire", 0)),
            air_supply: int_or(entries, "Air", 300),
            ticks_frozen: int_or(entries, "TicksFrozen", 0),
            portal_cooldown: int_or(entries, "PortalCooldown", 0),
            health: field(entries, "Health").and_then(field_parsers::number_f32),
            absorption_amount: float_or(entries, "AbsorptionAmount", 0.0),
            hurt_time: short_or(entries, "HurtTime", 0),
            death_time: short_or(entries, "DeathTime", 0),
            impulse_reset_grace_time: int_or(
                entries,
                "current_impulse_context_reset_grace_time",
                0,
            ),
            impulse_impact_position: double_vec3(field(entries, "current_explosion_impact_pos"))
                .map(|[x, y, z]| bbb_protocol::packets::Vec3d { x, y, z }),
            locator_bar_icon: parse_locator_bar_icon(field(entries, "locator_bar_icon")),
            selected_item_slot,
            abilities: parse_abilities(field(entries, "abilities")),
            food: bbb_server_types::food::FoodData::from_saved(
                int_or(entries, "foodLevel", 20),
                int_or(entries, "foodTickTimer", 0),
                float_or(entries, "foodSaturationLevel", 5.0),
                float_or(entries, "foodExhaustionLevel", 0.0),
            ),
            experience: Java26PlayerExperience {
                progress: float_or(entries, "XpP", 0.0),
                level: int_or(entries, "XpLevel", 0),
                total: int_or(entries, "XpTotal", 0),
                seed: int_or(entries, "XpSeed", 0),
                score: int_or(entries, "Score", 0),
            },
            // An absent field leaves the choice to the server default.
            game_mode: saved_game_mode(field(entries, "playerGameType")),
            // ServerPlayer.java:388,398,400.
            previous_game_mode: saved_game_mode(field(entries, "previousPlayerGameType")),
            entered_nether_pos: double_vec3(field(entries, "entered_nether_pos")),
            raid_omen_position: int_vec3(field(entries, "raid_omen_position")),
            recipe_book: field(entries, "recipeBook")
                .map(parse_recipe_book)
                .unwrap_or_default(),
            respawn: field(entries, "respawn").and_then(parse_respawn),
            last_death_location: field(entries, "LastDeathLocation").and_then(parse_global_pos),
            warden_spawn_tracker: parse_warden(field(entries, "warden_spawn_tracker")),
            sleep_timer: short_or(entries, "SleepTimer", 0),
            sleeping_pos,
            last_hurt_by: parse_last_hurt_by(entries),
            spawn_extra_particles_on_fall: bool_or(entries, "spawn_extra_particles_on_fall", false),
            seen_credits: bool_or(entries, "seenCredits", false),
            root_vehicle: field(entries, "RootVehicle")
                .and_then(|value| parse_root_vehicle(value, content_registry)),
            ender_pearls: ender_pearls::parse_with_content_registry(
                field(entries, "ender_pearls"),
                content_registry,
            ),
            tags,
            retained: if retain_raw {
                RetainedPlayerInputs::FullRaw(raw)
            } else {
                take_modeled_inputs(raw)
            },
        })
    }

    pub fn has_retained_raw(&self) -> bool {
        matches!(self.retained, RetainedPlayerInputs::FullRaw(_))
    }

    pub fn raw(&self) -> &NamedCompound {
        static EMPTY: OnceLock<NamedCompound> = OnceLock::new();
        match &self.retained {
            RetainedPlayerInputs::FullRaw(raw) => raw,
            RetainedPlayerInputs::ModeledInputs { .. } => {
                EMPTY.get_or_init(|| NamedCompound::new("", Vec::new()))
            }
        }
    }

    pub fn into_raw(self) -> NamedCompound {
        match self.retained {
            RetainedPlayerInputs::FullRaw(raw) => raw,
            RetainedPlayerInputs::ModeledInputs { .. } => NamedCompound::new("", Vec::new()),
        }
    }

    /// Rewrites every playerdata field the live player owns and preserves
    /// every other field of the document.
    pub fn raw_with_player_save_patch(
        &self,
        patch: &Java26PlayerSavePatch,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> SaveResult<NamedCompound> {
        let mut raw = match &self.retained {
            RetainedPlayerInputs::FullRaw(raw) => raw.clone(),
            RetainedPlayerInputs::ModeledInputs { .. } => created_player_document(patch, resolver)?,
        };
        patch_loaded_player_document(&mut raw, Some(&self.warden_spawn_tracker), patch, resolver)?;
        Ok(raw)
    }

    pub fn tags(&self) -> &[bbb_protocol::JavaString] {
        &self.tags
    }
    pub fn root_vehicle(&self) -> Option<&Java26RootVehicle> {
        self.root_vehicle.as_ref()
    }
    pub fn ender_pearls(&self) -> &[ender_pearls::Java26PlayerEnderPearl] {
        &self.ender_pearls
    }
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
    pub fn dimension(&self) -> &str {
        &self.dimension
    }
    pub fn position(&self) -> [f64; 3] {
        self.position
    }
    pub fn rotation(&self) -> [f32; 2] {
        self.rotation
    }
    pub fn fall_distance(&self) -> f64 {
        self.fall_distance
    }
    pub fn portal_cooldown(&self) -> i32 {
        self.portal_cooldown
    }
    pub fn fall_flying(&self) -> bool {
        self.fall_flying
    }
    pub fn shoulder_entities(&self) -> [&[NbtSummaryEntry]; 2] {
        match &self.retained {
            RetainedPlayerInputs::FullRaw(raw) => ["ShoulderEntityLeft", "ShoulderEntityRight"]
                .map(|name| match field(raw.entries(), name) {
                    Some(NbtSummaryValue::Compound(entries)) => entries.as_slice(),
                    _ => &[],
                }),
            RetainedPlayerInputs::ModeledInputs { shoulders, .. } => {
                [shoulders[0].as_slice(), shoulders[1].as_slice()]
            }
        }
    }

    pub fn custom_data(&self) -> &[NbtSummaryEntry] {
        &self.custom_data
    }
    pub fn custom_name(&self) -> Option<&NbtSummaryValue> {
        self.custom_name.as_ref()
    }
    pub fn silent(&self) -> bool {
        self.silent
    }
    pub fn glowing(&self) -> bool {
        self.glowing
    }
    pub fn has_visual_fire(&self) -> bool {
        self.has_visual_fire
    }
    pub fn remaining_fire_ticks(&self) -> i32 {
        self.remaining_fire_ticks
    }
    pub fn air_supply(&self) -> i32 {
        self.air_supply
    }
    pub fn ticks_frozen(&self) -> i32 {
        self.ticks_frozen
    }
    pub fn impulse_reset_grace_time(&self) -> i32 {
        self.impulse_reset_grace_time
    }
    pub fn impulse_impact_position(&self) -> Option<bbb_protocol::packets::Vec3d> {
        self.impulse_impact_position
    }
    pub fn locator_bar_icon(&self) -> &WaypointIcon {
        &self.locator_bar_icon
    }
    pub fn on_ground(&self) -> bool {
        self.on_ground
    }

    /// `LivingEntity.readAdditionalSaveData`: `Health` goes through the
    /// clamping `setHealth` (`LivingEntity.java:791`) and defaults to the max
    /// health; `AbsorptionAmount` is a raw field write (`:778`).
    pub fn damage(&self, max_health: f32) -> bbb_server_types::player_damage::PlayerDamageState {
        let mut state = bbb_server_types::player_damage::PlayerDamageState {
            health: 0.0,
            absorption_amount: self.absorption_amount,
            invulnerable_time: 0,
            last_hurt: 0.0,
            hurt_time: i32::from(self.hurt_time),
            death_time: i32::from(self.death_time),
        };
        state.set_health(self.health.unwrap_or(max_health), max_health);
        state
    }

    /// Raw input for the live attribute decoder.
    pub fn attributes_input(&self) -> Option<&NbtSummaryValue> {
        match &self.retained {
            RetainedPlayerInputs::FullRaw(raw) => field(raw.entries(), "attributes"),
            RetainedPlayerInputs::ModeledInputs { attributes, .. } => attributes.as_ref(),
        }
    }

    /// Raw input for the live effect decoder.
    pub fn active_effects_input(&self) -> Option<&NbtSummaryValue> {
        match &self.retained {
            RetainedPlayerInputs::FullRaw(raw) => field(raw.entries(), "active_effects"),
            RetainedPlayerInputs::ModeledInputs { active_effects, .. } => active_effects.as_ref(),
        }
    }

    pub fn attributes(&self) -> &[NbtSummaryValue] {
        match self.attributes_input() {
            Some(NbtSummaryValue::List(values)) => values,
            _ => &[],
        }
    }

    pub fn active_effects(&self) -> &[NbtSummaryValue] {
        match self.active_effects_input() {
            Some(NbtSummaryValue::List(values)) => values,
            _ => &[],
        }
    }

    /// `FoodData.readAdditionalSaveData` (`FoodData.java:74-79`).
    pub fn food_data(&self) -> bbb_server_types::food::FoodData {
        self.food
    }

    /// The `Invulnerable` byte (`Entity.java:2068`).
    pub fn invulnerable(&self) -> bool {
        self.invulnerable
    }

    // Player.java:623-626; TagValueInput.java:137-143: absent input is an empty slot list.
    pub fn inventory_input(&self) -> Option<&NbtSummaryValue> {
        match &self.retained {
            RetainedPlayerInputs::FullRaw(raw) => field(raw.entries(), "Inventory"),
            RetainedPlayerInputs::ModeledInputs { inventory, .. } => inventory.as_ref(),
        }
    }

    pub fn equipment_input(&self) -> Option<&NbtSummaryValue> {
        match &self.retained {
            RetainedPlayerInputs::FullRaw(raw) => field(raw.entries(), "equipment"),
            RetainedPlayerInputs::ModeledInputs { equipment, .. } => equipment.as_ref(),
        }
    }

    pub fn ender_items_input(&self) -> Option<&NbtSummaryValue> {
        match &self.retained {
            RetainedPlayerInputs::FullRaw(raw) => field(raw.entries(), "EnderItems"),
            RetainedPlayerInputs::ModeledInputs { ender_items, .. } => ender_items.as_ref(),
        }
    }

    pub fn selected_item_slot(&self) -> u8 {
        self.selected_item_slot
    }
    pub fn abilities(&self) -> &Java26PlayerAbilities {
        &self.abilities
    }
    pub fn experience(&self) -> &Java26PlayerExperience {
        &self.experience
    }
    /// `None` when the document has no `playerGameType`
    /// (`ServerPlayer.readPlayerMode`).
    pub fn game_mode(&self) -> Option<GameType> {
        self.game_mode
    }
    pub fn previous_game_mode(&self) -> Option<GameType> {
        self.previous_game_mode
    }
    pub fn entered_nether_pos(&self) -> Option<[f64; 3]> {
        self.entered_nether_pos
    }
    pub fn raid_omen_position(&self) -> Option<[i32; 3]> {
        self.raid_omen_position
    }
    pub fn recipe_book(&self) -> &Java26PlayerRecipeBook {
        &self.recipe_book
    }
    pub fn respawn(&self) -> Option<&Java26PlayerRespawn> {
        self.respawn.as_ref()
    }
    pub fn last_death_location(&self) -> Option<&GlobalPos> {
        self.last_death_location.as_ref()
    }
    pub fn warden_spawn_tracker(&self) -> &Java26WardenSpawnTracker {
        &self.warden_spawn_tracker
    }
    pub fn sleep_timer(&self) -> i16 {
        self.sleep_timer
    }
    pub fn sleeping_pos(&self) -> Option<[i32; 3]> {
        self.sleeping_pos
    }
    pub fn last_hurt_by(&self) -> Java26PlayerLastHurtBy {
        self.last_hurt_by
    }
    pub fn spawn_extra_particles_on_fall(&self) -> bool {
        self.spawn_extra_particles_on_fall
    }
    pub fn seen_credits(&self) -> bool {
        self.seen_credits
    }
}

pub(crate) fn patch_loaded_player_document(
    document: &mut NamedCompound,
    loaded_tracker: Option<&Java26WardenSpawnTracker>,
    patch: &Java26PlayerSavePatch,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> SaveResult<()> {
    let parsed_tracker;
    let tracker = match patch.warden_spawn_tracker.as_ref() {
        Some(tracker) => tracker,
        None => match loaded_tracker {
            Some(tracker) => tracker,
            None => {
                parsed_tracker = parse_warden(field(document.entries(), "warden_spawn_tracker"));
                &parsed_tracker
            }
        },
    };
    field_patches::write_save_patch(document.entries_mut(), patch, tracker, resolver)
}

/// `GameType.LEGACY_ID_CODEC` maps an unknown id to survival; an absent field
/// is no mode.
fn saved_game_mode(value: Option<&NbtSummaryValue>) -> Option<GameType> {
    value
        .and_then(field_parsers::number_i32)
        .map(GameType::from_id)
}

impl Java26PlayerAbilities {
    pub fn invulnerable(&self) -> bool {
        self.invulnerable
    }
    pub fn flying(&self) -> bool {
        self.flying
    }
    pub fn may_fly(&self) -> bool {
        self.may_fly
    }
    pub fn instant_build(&self) -> bool {
        self.instant_build
    }
    pub fn may_build(&self) -> bool {
        self.may_build
    }
    pub fn fly_speed(&self) -> f32 {
        self.fly_speed
    }
    pub fn walk_speed(&self) -> f32 {
        self.walk_speed
    }
}

impl Java26PlayerExperience {
    pub fn progress(&self) -> f32 {
        self.progress
    }
    pub fn level(&self) -> i32 {
        self.level
    }
    pub fn total(&self) -> i32 {
        self.total
    }
    pub fn seed(&self) -> i32 {
        self.seed
    }
    pub fn score(&self) -> i32 {
        self.score
    }
}

impl Java26PlayerRecipeBook {
    pub fn recipes(&self) -> &[String] {
        &self.recipes
    }
    pub fn to_be_displayed(&self) -> &[String] {
        &self.to_be_displayed
    }
    pub fn settings(&self) -> [bool; 8] {
        self.settings
    }
}

impl Java26PlayerRespawn {
    pub fn new(dimension: String, position: [i32; 3], yaw: f32, pitch: f32, forced: bool) -> Self {
        let [x, y, z] = position;
        Self {
            global_pos: GlobalPos {
                dimension,
                pos: BlockPos { x, y, z },
            },
            yaw,
            pitch,
            forced,
        }
    }

    pub fn global_pos(&self) -> &GlobalPos {
        &self.global_pos
    }
    pub fn yaw(&self) -> f32 {
        self.yaw
    }
    pub fn pitch(&self) -> f32 {
        self.pitch
    }
    pub fn forced(&self) -> bool {
        self.forced
    }
}

impl Java26WardenSpawnTracker {
    /// `WardenSpawnTracker.MAX_WARNING_LEVEL` (`WardenSpawnTracker.java:26`).
    pub const MAX_WARNING_LEVEL: i32 = 4;

    /// `WardenSpawnTracker()` (`WardenSpawnTracker.java:41-43`).
    pub const FRESH: Self = Self {
        ticks_since_last_warning: 0,
        warning_level: 0,
        cooldown_ticks: 0,
    };

    pub fn new(ticks_since_last_warning: i32, warning_level: i32, cooldown_ticks: i32) -> Self {
        Self {
            ticks_since_last_warning: ticks_since_last_warning.max(0),
            warning_level: warning_level.clamp(0, Self::MAX_WARNING_LEVEL),
            cooldown_ticks: cooldown_ticks.max(0),
        }
    }

    pub fn ticks_since_last_warning(&self) -> i32 {
        self.ticks_since_last_warning
    }
    pub fn warning_level(&self) -> i32 {
        self.warning_level
    }
    pub fn cooldown_ticks(&self) -> i32 {
        self.cooldown_ticks
    }
}
