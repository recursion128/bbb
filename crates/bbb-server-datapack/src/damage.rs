//! The player damage pipeline: `hurtServer` and everything it settles.
//!
//! `Entity.hurtServer` is abstract (Entity.java:1842) and a player reaches it
//! through three overrides, each refusing before the one below it:
//! `ServerPlayer.hurtServer` (ServerPlayer.java:971-984), then
//! `Player.hurtServer` (Player.java:679-709), then `LivingEntity.hurtServer`
//! (LivingEntity.java:1155-1281), which is where the invulnerability window,
//! the absorption and the death transfer live. The four `isInvulnerableTo`
//! answers stack the same way (`Entity.isInvulnerableToBase`,
//! Entity.java:2941-2946; `LivingEntity.isInvulnerableTo`,
//! LivingEntity.java:3933-3935; `Player.isInvulnerableTo`, Player.java:664-676;
//! `ServerPlayer.isInvulnerableTo`, ServerPlayer.java:1265-1267).
//!
//! The whole ladder is one pure function here, [`hurt_player`], because two
//! callers need the same answer and neither may guess it: `/damage` reports
//! `commands.damage.success` or `commands.damage.invulnerable` from
//! `hurtServer`'s own boolean (DamageCommand.java:103-110), and the world then
//! applies the state that same call produced. It is the shape
//! `bbb_server_core::commands::game_mode_change_moves_session` already uses for the
//! other command whose feedback predicts what the world is about to do.
//!
//! Only players are hurt here. Vanilla's `/damage` names any entity through
//! `EntityArgument.entity()` (DamageCommand.java:26); this server's non-player
//! entities are falling blocks, and the command's target resolution
//! (`bbb_server_core::commands::run`) is where that narrowing is declared.

use bbb_protocol::{
    component_literal_nbt, component_translatable_nbt,
    content_registry::ContentRegistry,
    difficulties::Difficulty,
    game_rules::{vanilla_game_rule, GameRuleValue},
    ComponentClickEvent, ComponentHoverEvent, ComponentStyle, JavaString, NbtSummaryValue,
    RichTextComponent,
};

pub use bbb_registry_codec::registry_codec::direct_leaf::{
    DamageEffects as DamageEffect, DamageScaling, DeathMessageType,
};
use bbb_registry_codec::{
    decode_direct_registry_json,
    registry_codec::direct_leaf::{decode_damage_type, DamageTypeElement},
    synchronized_registry_element_codec,
};

/// `SoundEvents.PLAYER_DEATH`, the one sound `Player.getDeathSound`
/// (Player.java:575-577) answers with whatever killed the player.
pub const PLAYER_DEATH_SOUND: &str = "minecraft:entity.player.death";

/// The damage-type tags the player hurt path reads.
///
/// Each is a `DamageTypeTags` constant one of the four overrides tests; the
/// membership is loaded rather than written down, so a datapack that moves a
/// type between tags moves this with it.
const READ_DAMAGE_TYPE_TAGS: &[&str] = &[
    // `Entity.isInvulnerableToBase` (Entity.java:2943) and `Player.hurtServer`
    // (Player.java:684).
    "minecraft:bypasses_invulnerability",
    // `LivingEntity.hurtServer` (LivingEntity.java:1196).
    "minecraft:bypasses_cooldown",
    // `Player.isInvulnerableTo` (Player.java:667-674).
    "minecraft:is_drowning",
    "minecraft:is_fall",
    "minecraft:is_fire",
    "minecraft:is_freezing",
    // `ArmorStand.hurtServer` (ArmorStand.java:296-316).
    "minecraft:is_explosion",
    "minecraft:ignites_armor_stands",
    "minecraft:burns_armor_stands",
    "minecraft:can_break_armor_stand",
    "minecraft:always_kills_armor_stands",
    // `CombatTracker.getMostSignificantFall` (CombatTracker.java:116).
    "minecraft:always_most_significant_fall",
];

/// Everything one `DamageType` says that the player hurt path reads.
///
/// `minecraft:damage_type` is a datapack registry in 26.1, so every field here
/// comes from the loaded generation rather than from a table written in this
/// crate.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageTypeFacts {
    /// The registry id, which is what `/damage <damageType>` names.
    pub resource_id: String,
    /// The synced registry index `ClientboundDamageEventPacket` writes
    /// (`DamageType.STREAM_CODEC`, DamageType.java:24).
    pub registry_id: i32,
    /// `DamageType.msgId` (DamageType.java:15), the `death.attack.<id>` stem
    /// `DamageSource.getLocalizedDeathMessage` builds on (DamageSource.java:72).
    pub message_id: JavaString,
    pub scaling: DamageScaling,
    /// `DamageType.exhaustion` (DamageType.java:12), which
    /// `DamageSource.getFoodExhaustion` (DamageSource.java:26-28) is the whole
    /// of and which `Player.actuallyHurt` spends on a landed hit
    /// (`Player.java:761`).
    pub exhaustion: f32,
    pub effects: DamageEffect,
    pub death_message_type: DeathMessageType,
    /// `DamageTypeTags.BYPASSES_INVULNERABILITY`.
    pub bypasses_invulnerability: bool,
    /// `DamageTypeTags.BYPASSES_COOLDOWN`.
    pub bypasses_cooldown: bool,
    /// `DamageTypeTags.IS_DROWNING`.
    pub is_drowning: bool,
    /// `DamageTypeTags.IS_FALL`.
    pub is_fall: bool,
    /// `DamageTypeTags.IS_FIRE`.
    pub is_fire: bool,
    /// `DamageTypeTags.IS_FREEZING`.
    pub is_freezing: bool,
    /// `DamageTypeTags.IS_EXPLOSION`.
    pub is_explosion: bool,
    /// `DamageTypeTags.IGNITES_ARMOR_STANDS`.
    pub ignites_armor_stands: bool,
    /// `DamageTypeTags.BURNS_ARMOR_STANDS`.
    pub burns_armor_stands: bool,
    /// `DamageTypeTags.CAN_BREAK_ARMOR_STAND`.
    pub can_break_armor_stand: bool,
    /// `DamageTypeTags.ALWAYS_KILLS_ARMOR_STANDS`.
    pub always_kills_armor_stands: bool,
    /// `DamageTypeTags.ALWAYS_MOST_SIGNIFICANT_FALL`.
    pub always_most_significant_fall: bool,
}

/// The facts of the whole `minecraft:damage_type` registry, in registry order.
///
/// `elements` is the loaded registry's own element run: `RegistryDataLoader`
/// registers one element per `damage_type/<path>.json` in the order the merged
/// resource view lists them, and that order is what the registry ids the wire
/// carries mean.
///
/// `tag_members` answers `HolderSet.contains` for one tag id.
pub fn damage_type_registry(
    elements: impl IntoIterator<Item = (String, DamageTypeElement)>,
    tag_members: impl Fn(&str) -> Vec<String>,
) -> Vec<DamageTypeFacts> {
    let tags: Vec<(&str, Vec<String>)> = READ_DAMAGE_TYPE_TAGS
        .iter()
        .map(|tag| (*tag, tag_members(tag)))
        .collect();
    let member = |tag_id: &str, resource_id: &str| {
        tags.iter()
            .find(|(tag, _)| *tag == tag_id)
            .is_some_and(|(_, members)| members.iter().any(|id| id == resource_id))
    };
    elements
        .into_iter()
        .enumerate()
        .map(|(index, (resource_id, element))| DamageTypeFacts {
            registry_id: i32::try_from(index).expect("a registry is shorter than i32::MAX"),
            message_id: element.message_id,
            scaling: element.scaling,
            exhaustion: element.exhaustion,
            effects: element.effects,
            death_message_type: element.death_message_type,
            bypasses_invulnerability: member("minecraft:bypasses_invulnerability", &resource_id),
            bypasses_cooldown: member("minecraft:bypasses_cooldown", &resource_id),
            is_drowning: member("minecraft:is_drowning", &resource_id),
            is_fall: member("minecraft:is_fall", &resource_id),
            is_fire: member("minecraft:is_fire", &resource_id),
            is_freezing: member("minecraft:is_freezing", &resource_id),
            is_explosion: member("minecraft:is_explosion", &resource_id),
            ignites_armor_stands: member("minecraft:ignites_armor_stands", &resource_id),
            burns_armor_stands: member("minecraft:burns_armor_stands", &resource_id),
            can_break_armor_stand: member("minecraft:can_break_armor_stand", &resource_id),
            always_kills_armor_stands: member("minecraft:always_kills_armor_stands", &resource_id),
            always_most_significant_fall: member(
                "minecraft:always_most_significant_fall",
                &resource_id,
            ),
            resource_id,
        })
        .collect()
}

/// The `minecraft:damage_type` registry of the built-in datapack alone.
///
/// This is what a world the runtime wired no datapack generation into reads.
/// It goes to the same embedded built-in pack `DatapackStartup::load` starts
/// from (`VanillaDatapackSnapshot::embedded`), rather than to a table written
/// here, and lists it the way `RegistryDataLoader` lists a registry directory:
/// by path, which is the order the ids follow.
pub fn builtin_damage_type_registry() -> &'static [DamageTypeFacts] {
    static BUILTIN: std::sync::OnceLock<Vec<DamageTypeFacts>> = std::sync::OnceLock::new();
    BUILTIN.get_or_init(load_builtin_damage_type_registry)
}

fn load_builtin_damage_type_registry() -> Vec<DamageTypeFacts> {
    let snapshot = crate::vanilla_snapshot::VanillaDatapackSnapshot::embedded_shared()
        .expect("the embedded built-in datapack decodes");
    let mut paths: Vec<&str> = snapshot
        .paths()
        .filter(|path| path.starts_with("damage_type/") && path.ends_with(".json"))
        .collect();
    paths.sort_unstable();
    let elements = paths.into_iter().map(|path| {
        let name = path
            .trim_start_matches("damage_type/")
            .trim_end_matches(".json");
        let bytes = snapshot
            .read_path(path)
            .expect("an embedded damage type path exists");
        let data =
            decode_direct_registry_json(bytes).expect("an embedded damage type is valid JSON");
        let codec = synchronized_registry_element_codec(DAMAGE_TYPE_REGISTRY, data.source)
            .expect("damage types have a generated codec route");
        let element = decode_damage_type(data.root(), codec)
            .expect("an embedded damage type satisfies its codec");
        (format!("minecraft:{name}"), element)
    });
    damage_type_registry(elements, builtin_damage_type_tag_members)
}

/// The `damage_type` tag membership `bbb_protocol` generated from the built-in
/// pack, with no world datapack overlay.
///
/// It is the same content `TagLoader` builds out of the built-in pack's
/// `tags/damage_type` files, which is what a world the runtime wired a datapack
/// generation into reads instead. This is the answer for the worlds that have
/// none.
pub fn builtin_damage_type_tag_members(tag_id: &str) -> Vec<String> {
    match bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1(
        "minecraft:damage_type",
        tag_id,
    ) {
        bbb_protocol::PersistentRegistryTagMembers::Present(members) => members,
        // A tag the generated facts do not carry is an empty holder set, which
        // is how `HolderSet.contains` answers for a tag nothing declares.
        bbb_protocol::PersistentRegistryTagMembers::Absent
        | bbb_protocol::PersistentRegistryTagMembers::Unavailable => Vec::new(),
    }
}

/// The source `/damage <target> <amount>` hurts with
/// (`damageSources().generic()`, DamageCommand.java:34).
pub const GENERIC_DAMAGE_TYPE: &str = "minecraft:generic";

/// The source `Entity.kill` hurts with (`LivingEntity.kill`,
/// LivingEntity.java:303-305, through `DamageSources.genericKill`,
/// DamageSources.java:271-273).
///
/// Its element is in `minecraft:bypasses_invulnerability` and
/// `minecraft:bypasses_resistance`, which is why `/kill` reaches a creative
/// player where `/damage <target> <amount>` does not.
pub const GENERIC_KILL_DAMAGE_TYPE: &str = "minecraft:generic_kill";

/// `Registries.DAMAGE_TYPE`, the registry `/damage`'s `<damageType>` resolves
/// against (DamageCommand.java:38) and the one this module reads elements and
/// tags out of.
pub const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";

/// The four game rules `Player.isInvulnerableTo` (Player.java:667-674) reads,
/// plus the one `ServerPlayer.hurtServer` reaches through `canHarmPlayer`
/// (ServerPlayer.java:987-993, `ServerLevel.isPvpAllowed`, ServerLevel.java:1875-1877).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageGameRules {
    pub drowning_damage: bool,
    pub fall_damage: bool,
    pub fire_damage: bool,
    pub freeze_damage: bool,
    pub pvp: bool,
}

impl Default for DamageGameRules {
    /// All five are registered defaulted to on, which is what a projection
    /// carrying no rule at all reads back through the generated catalog.
    fn default() -> Self {
        Self {
            drowning_damage: true,
            fall_damage: true,
            fire_damage: true,
            freeze_damage: true,
            pvp: true,
        }
    }
}

impl DamageGameRules {
    /// Reads the five out of whatever holds this world's rules.
    ///
    /// A rule the holder does not carry falls back to the generated catalog's
    /// own default, which is what `GameRules` answers for it: the map is built
    /// with every enabled rule already at its registered default
    /// (`gamerules/GameRules.java:99-110`).
    ///
    /// Both readers go through here — the command runs the hurt over its
    /// projection and the world runs it again over its own state — so the two
    /// cannot disagree about which rule id feeds which branch.
    pub fn from_rules(values: impl Fn(&str) -> Option<GameRuleValue>) -> Self {
        let read = |id: &str| {
            let value = values(id)
                .or_else(|| vanilla_game_rule(id).map(|entry| entry.default_value))
                .expect("the five damage rules are in the generated catalog");
            match value {
                GameRuleValue::Boolean(flag) => flag,
                GameRuleValue::Integer(_) => {
                    unreachable!("the five damage rules are registered as boolean rules")
                }
            }
        };
        Self {
            drowning_damage: read("minecraft:drowning_damage"),
            fall_damage: read("minecraft:fall_damage"),
            fire_damage: read("minecraft:fire_damage"),
            freeze_damage: read("minecraft:freeze_damage"),
            pvp: read("minecraft:pvp"),
        }
    }
}

pub use bbb_server_types::player_damage::{
    player_max_absorption, player_max_health, PlayerDamageState, DEATH_TICKS, HURT_DURATION,
    INVULNERABLE_TICKS, INVULNERABLE_TICKS_HALF, LAST_HURT_BY_PLAYER_MEMORY_TICKS,
};

/// The target of one hurt, as far as the four overrides read it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HurtTarget {
    pub state: PlayerDamageState,
    /// `Abilities.invulnerable` (Player.java:684), which
    /// `GameType.updatePlayerAbilities` sets for creative and spectator.
    pub abilities_invulnerable: bool,
    /// `Entity.invulnerable` (Entity.java:2943), the `Invulnerable` byte the
    /// playerdata document carries.
    pub entity_invulnerable: bool,
    /// `this.connection.hasClientLoaded()` (ServerPlayer.java:1266).
    pub client_loaded: bool,
    /// `ServerPlayer.isChangingDimension()` (ServerPlayer.java:1958-1960), the
    /// other arm `ServerPlayer.isInvulnerableTo` ors on (`:1266`).
    pub changing_dimension: bool,
    /// `getMaxHealth()` and `getMaxAbsorption()` (LivingEntity.java:1954-1960):
    /// the two attributes with their modifiers, which the caller reads off the
    /// player's own map.
    pub max_health: f32,
    pub max_absorption: f32,
}

/// The player-relevant facts of one `DamageSource`.
///
/// Stored non-player source ids stay on the command/world effect for
/// the clientbound packet; only actual players enter these two fields, because
/// creative bypass, PVP and kill credit test that runtime kind explicitly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HurtSource<'a> {
    pub damage_type: &'a DamageTypeFacts,
    /// `DamageSource.getEntity()` (DamageSource.java:63-65).
    pub causing_player: Option<HurtEntity>,
    /// `DamageSource.getDirectEntity()` (DamageSource.java:59-61).
    pub direct_player: Option<HurtEntity>,
    /// `DamageSource.sourcePositionRaw()` (DamageSource.java:112-114): only a
    /// source built with an explicit position carries one.
    pub source_position: Option<[f64; 3]>,
    /// `source.getEntity()` is an `AbstractArrow` whose owner is a player
    /// (`ServerPlayer.java:978-980`).
    pub arrow_owner_player: bool,
    /// `Player.canHarmPlayer`'s scoreboard-team arm (`Player.java:727-735`)
    /// refusing the causing player or the arrow's owner, answered by the
    /// caller holding the scoreboard.
    pub team_refuses_harm: bool,
}

/// One player named by a `DamageSource`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HurtEntity {
    pub entity_id: i32,
    /// `Abilities.instabuild`, which is the whole of
    /// `DamageSource.isCreativePlayer` (DamageSource.java:100-102) once the
    /// entity is known to be a player.
    pub instabuild: bool,
}

impl HurtSource<'_> {
    /// `DamageSource.isCreativePlayer` (DamageSource.java:100-102).
    fn is_creative_player(&self) -> bool {
        self.causing_player.is_some_and(|player| player.instabuild)
    }

    /// `DamageSource.scalesWithDifficulty` (DamageSource.java:92-98).
    fn scales_with_difficulty(&self, causing_living_non_player: bool) -> bool {
        match self.damage_type.scaling {
            DamageScaling::Never => false,
            DamageScaling::WhenCausedByLivingNonPlayer => causing_living_non_player,
            DamageScaling::Always => true,
        }
    }
}

/// What one hurt did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HurtOutcome {
    /// `hurtServer`'s own return value, which is what `/damage` reports on.
    pub succeeded: bool,
    /// The state the target now holds.
    pub state: PlayerDamageState,
    /// Whether `ServerLevel.broadcastDamageEvent` ran
    /// (LivingEntity.java:1219, ServerLevel.java:1186-1188). A blocked hit
    /// answers false; it still runs when blocking broke the item in use
    /// (`:1215-1217`), which only the caller holding that item can tell.
    pub broadcast_damage_event: bool,
    /// The sound `makeSound` played: the death sound when the hit killed
    /// (LivingEntity.java:1248) and the hurt sound otherwise (`:1255`).
    pub sound: Option<&'static str>,
    /// Whether the hit reached `isDeadOrDying` (LivingEntity.java:1245).
    /// `checkTotemDeathProtection` (`:1246`) reads the held stacks, so the
    /// caller runs it and skips `sound` and `die` when it cancels the death.
    pub died: bool,
    /// The `dmg` that reached `setHealth`, before its clamp at zero, which is
    /// why the health delta is not a substitute for it: a forty-damage blow on
    /// five health moves the health by five and is applied as forty.
    ///
    /// Vanilla awards `Stats.DAMAGE_TAKEN` from it as `Math.round(dmg * 10.0F)`
    /// under `dmg != 0.0F` and `dmg < 3.4028235E37F` (Player.java:760-766).
    /// Both guards are functions of this value alone, so a caller that keeps
    /// statistics has everything it needs; this crate keeps none and awards
    /// neither.
    pub applied_damage: f32,
    /// `applyItemBlocking`'s answer (LivingEntity.java:1178-1180) when it got
    /// past its own refusals and resolved a blocked amount, which is when
    /// `hurtBlockingItem` runs (`:1310`) even for a zero answer.
    pub item_blocking: Option<f32>,
    /// `tookFullDamage` (`:1195`) of a hit that got past the cooldown refusal
    /// (`:1196-1199`); `None` for one `hurtServer` returned on before it.
    pub took_full_damage: Option<bool>,
    /// The damage left after blocking (`:1180`) as `markHurt` reads it
    /// (`:1222`).
    pub damage_after_blocking: f32,
    /// Damage after difficulty scaling and the negative clamp, before blocking
    /// (`LivingEntity.java:1177`).
    pub original_damage: f32,
    /// `absorbedDamage` (Player.java:755): the part of the same hit the
    /// absorption hearts took instead. `Stats.DAMAGE_ABSORBED` is awarded from
    /// it on the same decidecimal scaling, under `> 0.0F && < 3.4028235E37F`
    /// (`:756-758`).
    pub absorbed_damage: f32,
    /// The amount `causeFoodExhaustion` was called with (`Player.java:761`),
    /// which is the damage type's own `exhaustion` on a hit that put damage
    /// through and nothing on one that did not.
    ///
    /// Carried rather than spent for the same reason the two above are: the
    /// food data belongs to the caller. `causeFoodExhaustion`'s own
    /// `abilities.invulnerable` guard (`Player.java:1561-1567`) is not applied
    /// here either -- [`bbb_server_types::food::cause_food_exhaustion`] is what applies
    /// it, and it is the function this value is meant for.
    pub food_exhaustion: f32,
}

/// `Entity.hurtServer` for a player: the three overrides, top down.
///
/// Every branch of the three that this server cannot reach says so where it
/// would have been, rather than being dropped: a reader comparing this against
/// the vanilla method should find one comment per missing line.
pub fn hurt_player(
    target: HurtTarget,
    source: HurtSource<'_>,
    damage: f32,
    difficulty: Difficulty,
    rules: DamageGameRules,
) -> HurtOutcome {
    hurt_player_with_causing_living_non_player(target, source, false, damage, difficulty, rules)
}

/// Applies hurt after resolving the causing entity's live kind.
pub fn hurt_player_with_causing_living_non_player(
    target: HurtTarget,
    source: HurtSource<'_>,
    causing_living_non_player: bool,
    damage: f32,
    difficulty: Difficulty,
    rules: DamageGameRules,
) -> HurtOutcome {
    hurt_player_with_item_blocking(
        target,
        source,
        causing_living_non_player,
        damage,
        difficulty,
        rules,
        &mut |_| None,
    )
}

/// [`hurt_player_with_causing_living_non_player`] for a target that may be
/// blocking: `item_blocking` is `applyItemBlocking` past its `damage <= 0`
/// guard (LivingEntity.java:1283-1320), answering the blocked amount or
/// `None` where it returns zero without resolving one.
pub fn hurt_player_with_item_blocking(
    target: HurtTarget,
    source: HurtSource<'_>,
    causing_living_non_player: bool,
    damage: f32,
    difficulty: Difficulty,
    rules: DamageGameRules,
    item_blocking: &mut dyn FnMut(f32) -> Option<f32>,
) -> HurtOutcome {
    struct ItemBlockingOnly<'a>(&'a mut dyn FnMut(f32) -> Option<f32>);
    impl PlayerHurtHooks for ItemBlockingOnly<'_> {
        fn immune_to_damage(&mut self) -> bool {
            false
        }
        fn item_blocking(&mut self, damage: f32) -> Option<f32> {
            (self.0)(damage)
        }
        fn hurt_helmet(&mut self, _damage: f32) -> bool {
            false
        }
        fn absorb(&mut self, damage: f32) -> f32 {
            damage
        }
    }
    hurt_player_with_hooks(
        target,
        source,
        causing_living_non_player,
        damage,
        difficulty,
        rules,
        &mut ItemBlockingOnly(item_blocking),
    )
}

/// The parts of a player's `hurtServer` that read its equipment and effects,
/// which the caller holds.
pub trait PlayerHurtHooks {
    fn hurt_admitted(&mut self) {}
    /// Whether the target has Fire Resistance (`LivingEntity.java:1164-1166`).
    fn fire_resistant(&mut self) -> bool {
        false
    }
    /// `EnchantmentHelper.isImmuneToDamage` (`EnchantmentHelper.java:135-164`),
    /// the arm `LivingEntity.isInvulnerableTo` (`LivingEntity.java:3934`) adds.
    fn immune_to_damage(&mut self) -> bool;
    /// `applyItemBlocking` past its `damage <= 0` guard
    /// (`LivingEntity.java:1283-1320`): the blocked amount, or `None` where it
    /// returns zero without resolving one.
    fn item_blocking(&mut self, damage: f32) -> Option<f32>;
    /// `:1186-1188`: `hurtHelmet` for a `DAMAGES_HELMET` source on an occupied
    /// head, answering whether it ran.
    fn hurt_helmet(&mut self, damage: f32) -> bool;
    /// `getDamageAfterArmorAbsorb` then `getDamageAfterMagicAbsorb`
    /// (`LivingEntity.java:1869-1919`), as `Player.actuallyHurt` runs them
    /// (`Player.java:750-751`).
    fn absorb(&mut self, damage: f32) -> f32;
}

/// [`hurt_player_with_item_blocking`] with every equipment and effect read
/// answered by `hooks`.
pub fn hurt_player_with_hooks(
    target: HurtTarget,
    source: HurtSource<'_>,
    causing_living_non_player: bool,
    damage: f32,
    difficulty: Difficulty,
    rules: DamageGameRules,
    hooks: &mut dyn PlayerHurtHooks,
) -> HurtOutcome {
    let mut outcome = HurtOutcome {
        succeeded: false,
        state: target.state,
        broadcast_damage_event: false,
        sound: None,
        died: false,
        item_blocking: None,
        took_full_damage: None,
        damage_after_blocking: 0.0,
        original_damage: 0.0,
        applied_damage: 0.0,
        absorbed_damage: 0.0,
        food_exhaustion: 0.0,
    };

    // `ServerPlayer.hurtServer` (ServerPlayer.java:971-984). The immunity
    // cannot change before `actuallyHurt` asks again, so it is asked once.
    if player_is_invulnerable_to(&target, &source, rules) || hooks.immune_to_damage() {
        return outcome;
    }
    // `:976-981`: the two `canHarmPlayer` refusals. `ServerPlayer.canHarmPlayer`
    // (ServerPlayer.java:987-989) is `isPvpAllowed() && super.canHarmPlayer`.
    if (source.causing_player.is_some() || source.arrow_owner_player)
        && (!rules.pvp || source.team_refuses_harm)
    {
        return outcome;
    }

    // `Player.hurtServer` (Player.java:679-709). Its own `isInvulnerableTo`
    // call (:680) is the same one already made above, `isInvulnerableTo` being
    // a single virtual answer.
    if target.abilities_invulnerable && !source.damage_type.bypasses_invulnerability {
        return outcome;
    }
    // `:688` sets `noActionTime`, which only the idle-timeout kick reads
    // (`ServerGamePacketListenerImpl.tick`); this server has no wall clock to
    // measure that timeout against, which is the same blocker `/setidletimeout`
    // is held out of the command catalog by.
    if outcome.state.is_dead_or_dying() {
        return outcome;
    }
    hooks.hurt_admitted();
    let mut damage = damage;
    if source.scales_with_difficulty(causing_living_non_player) {
        // `:694-706`.
        if difficulty == Difficulty::Peaceful {
            damage = 0.0;
        }
        if difficulty == Difficulty::Easy {
            damage = (damage / 2.0 + 1.0).min(damage);
        }
        if difficulty == Difficulty::Hard {
            damage = damage * 3.0 / 2.0;
        }
    }
    if damage == 0.0 {
        // `:708`.
        return outcome;
    }

    // `LivingEntity.hurtServer` (LivingEntity.java:1155-1281). The
    // `isInvulnerableTo` (`:1156`) and `isDeadOrDying` (`:1160`) guards are the
    // two already answered above; Fire Resistance refuses only after the
    // shoulder entities came off.
    if source.damage_type.is_fire && hooks.fire_resistant() {
        return outcome;
    }
    if damage < 0.0 {
        // `:1173-1175`. `/damage` cannot reach it -- its `<amount>` is a
        // `floatArg(0.0F)` (DamageCommand.java:28) -- but the clamp belongs to
        // `hurtServer` rather than to its caller.
        damage = 0.0;
    }
    outcome.original_damage = damage;
    // `:1178-1181`.
    let damage_blocked = if damage > 0.0 {
        outcome.item_blocking = hooks.item_blocking(damage);
        outcome.item_blocking.unwrap_or(0.0)
    } else {
        0.0
    };
    damage -= damage_blocked;
    let blocked = damage_blocked > 0.0;
    // `:1182-1184` multiplies freezing damage for the entity types in
    // `FREEZE_HURTS_EXTRA_TYPES`, which a player is not in.
    if hooks.hurt_helmet(damage) {
        // `:1188`.
        damage *= 0.75;
    }
    if damage.is_nan() || damage.is_infinite() {
        // `:1191-1193`.
        damage = f32::MAX;
    }
    outcome.damage_after_blocking = damage;

    let mut took_full_damage = true;
    if outcome.state.invulnerable_time > INVULNERABLE_TICKS_HALF
        && !source.damage_type.bypasses_cooldown
    {
        // `:1196-1203`.
        if damage <= outcome.state.last_hurt {
            return outcome;
        }
        let above_the_last = damage - outcome.state.last_hurt;
        actually_hurt(&mut outcome, &target, &source, rules, hooks, above_the_last);
        outcome.state.last_hurt = damage;
        took_full_damage = false;
    } else {
        // `:1205-1209`.
        outcome.state.last_hurt = damage;
        outcome.state.invulnerable_time = INVULNERABLE_TICKS;
        actually_hurt(&mut outcome, &target, &source, rules, hooks, damage);
        outcome.state.hurt_time = HURT_DURATION;
    }
    outcome.took_full_damage = Some(took_full_damage);

    // `:1212-1213` (the responsible mob and player) belong to the caller.
    if took_full_damage {
        // `:1215-1220`. The blocked arm's `onBlocked`, the `markHurt` of
        // `:1222-1224` and the knockback of `:1226-1242` belong to the caller,
        // which holds the item in use and the velocity.
        outcome.broadcast_damage_event = !blocked;
    }

    if outcome.state.is_dead_or_dying() {
        // `:1245-1253`; `died` documents the caller's totem check.
        if took_full_damage {
            outcome.sound = Some(PLAYER_DEATH_SOUND);
            // `playSecondaryHurtSound` (`:1322-1327`) only fires for
            // `minecraft:thorns`, which nothing here can deal.
        }
        outcome.died = true;
    } else if took_full_damage {
        // `:1254-1257`.
        outcome.sound = Some(source.damage_type.effects.sound_resource_id());
    }

    // `:1259-1267`. The `lastDamageSource` stamp it sets is read by
    // `getLastDamageSource` (LivingEntity.java:1394-1400), whose only
    // consumers are the client HUD and mob AI; the effect loop has no effect
    // to run.
    outcome.succeeded = !blocked || damage > 0.0;
    // `:1269-1278` awards the advancement criteria and the shield statistic;
    // the statistic is the caller's, from `item_blocking`.
    outcome
}

/// `Player.actuallyHurt` (Player.java:748-770), which overrides
/// `LivingEntity.actuallyHurt` (LivingEntity.java:1921-1939) with no `super`
/// call, so a player never runs the `LivingEntity` body. The two bodies differ
/// in three lines: the absorbed-damage statistic is the victim's own rather
/// than the attacker's, the hit costs food exhaustion, and `LivingEntity`'s
/// second `setAbsorptionAmount` (`:1936`) has no counterpart here.
fn actually_hurt(
    outcome: &mut HurtOutcome,
    target: &HurtTarget,
    source: &HurtSource<'_>,
    rules: DamageGameRules,
    hooks: &mut dyn PlayerHurtHooks,
    damage: f32,
) {
    // `Player.java:749`, which every bare reference below is relative to. The
    // guard is re-asked here because vanilla re-asks it; nothing between the
    // two calls can change the answer for a player.
    if player_is_invulnerable_to(target, source, rules) {
        return;
    }
    // `:750-751`.
    let mut damage = hooks.absorb(damage);
    // `:752-754`.
    let original_damage = damage;
    let state = &mut outcome.state;
    damage = (damage - state.absorption_amount).max(0.0);
    state.set_absorption_amount(
        state.absorption_amount - (original_damage - damage),
        target.max_absorption,
    );
    // `:755`.
    let absorbed_damage = original_damage - damage;
    let mut food_exhaustion = 0.0;
    if damage != 0.0 {
        // `:761`.
        food_exhaustion = source.damage_type.exhaustion;
        // `:762-768`. The live world caller raises `ENTITY_DAMAGE` for this
        // nonzero applied amount after writing the projected state.
        state.set_health(state.health - damage, target.max_health);
    }
    // The two statistic awards (`:756-758` and `:764-766`) are the caller's,
    // which is why both quantities leave here rather than the health delta.
    outcome.applied_damage = damage;
    outcome.absorbed_damage = absorbed_damage;
    outcome.food_exhaustion = food_exhaustion;
}

/// `ServerPlayer.isInvulnerableTo` (ServerPlayer.java:1265-1267) down through
/// `Player.isInvulnerableTo` (Player.java:664-676),
/// `LivingEntity.isInvulnerableTo` (LivingEntity.java:3933-3935) and
/// `Entity.isInvulnerableToBase` (Entity.java:2941-2946).
pub fn player_is_invulnerable_to(
    target: &HurtTarget,
    source: &HurtSource<'_>,
    rules: DamageGameRules,
) -> bool {
    // `Entity.isInvulnerableToBase` (Entity.java:2942-2945). `isRemoved` is
    // false for a session the world still holds; `fireImmune` reads the entity
    // type's own flag, which `minecraft:player` does not carry; and
    // `FALL_DAMAGE_IMMUNE` is an entity-type tag a player is not in.
    if target.entity_invulnerable
        && !source.damage_type.bypasses_invulnerability
        && !source.is_creative_player()
    {
        return true;
    }
    // `LivingEntity.isInvulnerableTo` (LivingEntity.java:3934) adds
    // `EnchantmentHelper.isImmuneToDamage`; equipment damage immunity is not
    // represented by this projection yet.

    // `Player.isInvulnerableTo` (Player.java:667-674). Only the first tag that
    // matches decides, and a type in none of the four is not invulnerable here
    // (`:674`'s `false`).
    let by_game_rule = if source.damage_type.is_drowning {
        !rules.drowning_damage
    } else if source.damage_type.is_fall {
        !rules.fall_damage
    } else if source.damage_type.is_fire {
        !rules.fire_damage
    } else if source.damage_type.is_freezing {
        !rules.freeze_damage
    } else {
        false
    };
    // `ServerPlayer.isInvulnerableTo` (ServerPlayer.java:1266) *ors* its own
    // two arms onto whatever `Player` answered, so a rule that let the source
    // through still has to survive them. The dimension-change arm is open from
    // the moment a cross-level teleport starts until the session acknowledges
    // the position it landed at, and the client-loaded arm is the join window
    // before the session reports itself ready.
    by_game_rule
        || (target.changing_dimension && source.damage_type.resource_id != "minecraft:ender_pearl")
        || !target.client_loaded
}

/// `LivingEntity.baseTick`'s counters (LivingEntity.java:448-458) plus
/// `ServerPlayer.tick`'s own invulnerability decrement
/// (ServerPlayer.java:572-578).
///
/// The split is vanilla's: `LivingEntity` decrements `invulnerableTime` for
/// everything *except* a `ServerPlayer` (`:452-454`), and `ServerPlayer.tick`
/// (`:576-578`) does it for the player instead. One counter, two owners, and
/// the same single step a tick either way.
///
/// A frozen server still runs this: `TickRateManager.isEntityFrozen`
/// (TickRateManager.java:63-65) answers false for every `Player`, so the entity
/// tick list still reaches them (ServerLevel.java:412-434).
///
/// `tickDeath` is carried out in two pieces: the counter step is here, and the
/// removal it ends with is [`death_tick_removes_body`], which the world asks
/// after taking the step because only the world can announce and unpair the
/// avatar. The counter stops at twenty here for the reason vanilla's stops
/// there: the removed body leaves the tick list (ServerLevel.java:412-413).
pub fn tick_damage_state(mut state: PlayerDamageState) -> PlayerDamageState {
    if state.invulnerable_time > 0 {
        state.invulnerable_time -= 1;
    }
    tick_living_damage_state(state)
}

/// LivingEntity.baseTick counters; ServerPlayer decrements invulnerability in its level tick.
pub fn tick_living_damage_state(mut state: PlayerDamageState) -> PlayerDamageState {
    if state.hurt_time > 0 {
        state.hurt_time -= 1;
    }
    // `:456-458` over `Level.shouldTickDeath` (Level.java:562-564), which is
    // unconditionally true on the server.
    if state.is_dead_or_dying() && state.death_time < DEATH_TICKS {
        state.death_time += 1;
    }
    state
}

/// `LivingEntity.tickDeath`'s second half (LivingEntity.java:551-553): whether
/// this is the tick the body is announced with `EntityEvent.POOF` and removed.
///
/// It reads the counter [`tick_damage_state`] has just stepped, so the two
/// cannot disagree about which tick that is. `already_removed` is
/// `this.isRemoved()` (Entity.java:3843-3845), which is what keeps a body that
/// is already gone from being announced twice -- vanilla gets that for free by
/// dropping the entity out of the tick list, and this world gets it by asking.
///
/// `!level().isClientSide()` is the third arm of the same test and is always
/// true here.
pub fn death_tick_removes_body(state: &PlayerDamageState, already_removed: bool) -> bool {
    state.is_dead_or_dying() && state.death_time >= DEATH_TICKS && !already_removed
}

/// `CommonLinks.INTENTIONAL_GAME_DESIGN_BUG` (CommonLinks.java:31) and its id
/// (`:30`), which `CombatTracker.INTENTIONAL_GAME_DESIGN_STYLE`
/// (CombatTracker.java:22-24) puts on the link the message carries.
const INTENTIONAL_GAME_DESIGN_BUG_URL: &str = "https://bugs.mojang.com/browse/MCPE-28723";
const INTENTIONAL_GAME_DESIGN_BUG_ID: &str = "MCPE-28723";

/// `FallLocation` (FallLocation.java:11-19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallLocation {
    Generic,
    Ladder,
    Vines,
    WeepingVines,
    TwistingVines,
    Scaffolding,
    OtherClimbable,
    Water,
}

impl FallLocation {
    /// `FallLocation.blockToFallLocation` (FallLocation.java:21-33).
    pub fn of_climbable_block(resource_id: &str, is_trapdoor: bool) -> Self {
        match resource_id {
            "minecraft:ladder" => Self::Ladder,
            _ if is_trapdoor => Self::Ladder,
            "minecraft:vine" => Self::Vines,
            "minecraft:weeping_vines" | "minecraft:weeping_vines_plant" => Self::WeepingVines,
            "minecraft:twisting_vines" | "minecraft:twisting_vines_plant" => Self::TwistingVines,
            "minecraft:scaffolding" => Self::Scaffolding,
            _ => Self::OtherClimbable,
        }
    }

    fn language_key(self) -> &'static str {
        match self {
            Self::Generic => "death.fell.accident.generic",
            Self::Ladder => "death.fell.accident.ladder",
            Self::Vines => "death.fell.accident.vines",
            Self::WeepingVines => "death.fell.accident.weeping_vines",
            Self::TwistingVines => "death.fell.accident.twisting_vines",
            Self::Scaffolding => "death.fell.accident.scaffolding",
            Self::OtherClimbable => "death.fell.accident.other_climbable",
            Self::Water => "death.fell.accident.water",
        }
    }
}

/// `CombatEntry` (CombatEntry.java:5) with the source reduced to what the
/// death message reads.
#[derive(Debug, Clone, PartialEq)]
pub struct CombatEntry {
    pub is_fall: bool,
    pub always_most_significant_fall: bool,
    /// The display name of `source.getEntity()`.
    pub attacker: Option<NbtSummaryValue>,
    /// That entity's custom-named main-hand stack's display name, for the
    /// `.item` message variants (`DamageSource.java:80-83`,
    /// `CombatTracker.java:56-63`).
    pub attacker_weapon: Option<NbtSummaryValue>,
    pub damage: f32,
    pub fall_location: Option<FallLocation>,
    pub fall_distance: f32,
}

/// Combat transition signals emitted by [`CombatTracker::record_damage`] and
/// [`CombatTracker::recheck_status`] (`CombatTracker.java:37-50`, `:147-159`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatTransition {
    pub left_duration: Option<i32>,
    pub entered: bool,
}

/// `CombatTracker` (`CombatTracker.java:19-160`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CombatTracker {
    entries: Vec<CombatEntry>,
    last_damage_time: i32,
    combat_start_time: i32,
    combat_end_time: i32,
    in_combat: bool,
    taking_damage: bool,
}

/// `CombatTracker.RESET_DAMAGE_STATUS_TIME` and `RESET_COMBAT_STATUS_TIME`
/// (CombatTracker.java:20-21).
const RESET_DAMAGE_STATUS_TIME: i32 = 100;
const RESET_COMBAT_STATUS_TIME: i32 = 300;

impl CombatTracker {
    /// `getCombatDuration` (`CombatTracker.java:104-106`).
    pub fn combat_duration(&self, tick_count: i32) -> i32 {
        if self.in_combat {
            tick_count.wrapping_sub(self.combat_start_time)
        } else {
            self.combat_end_time.wrapping_sub(self.combat_start_time)
        }
    }

    /// `recordDamage` (CombatTracker.java:37-50) for a living victim;
    /// `enters_combat` is `source.getEntity() instanceof LivingEntity`.
    pub fn record_damage(
        &mut self,
        entry: CombatEntry,
        tick_count: i32,
        enters_combat: bool,
    ) -> CombatTransition {
        self.record_damage_with_alive(entry, tick_count, enters_combat, true)
    }

    /// `recordDamage` (`CombatTracker.java:37-50`) with the victim's live
    /// `isAlive()` state after `setHealth`.
    pub fn record_damage_with_alive(
        &mut self,
        entry: CombatEntry,
        tick_count: i32,
        enters_combat: bool,
        alive: bool,
    ) -> CombatTransition {
        let left_duration = self.recheck_status(tick_count, true);
        self.entries.push(entry);
        self.last_damage_time = tick_count;
        self.taking_damage = true;
        let entered = if !self.in_combat && alive && enters_combat {
            self.in_combat = true;
            self.combat_start_time = tick_count;
            self.combat_end_time = tick_count;
            true
        } else {
            false
        };
        CombatTransition {
            left_duration,
            entered,
        }
    }

    /// `recheckStatus` (CombatTracker.java:147-159). Returns the completed
    /// combat duration when `in_combat` transitions from `true` to `false`.
    pub fn recheck_status(&mut self, tick_count: i32, alive: bool) -> Option<i32> {
        let reset = if self.in_combat {
            RESET_COMBAT_STATUS_TIME
        } else {
            RESET_DAMAGE_STATUS_TIME
        };
        if self.taking_damage && (!alive || tick_count.wrapping_sub(self.last_damage_time) > reset)
        {
            let was_in_combat = self.in_combat;
            self.taking_damage = false;
            self.in_combat = false;
            self.combat_end_time = tick_count;
            self.entries.clear();
            if was_in_combat {
                return Some(self.combat_end_time.wrapping_sub(self.combat_start_time));
            }
        }
        None
    }

    /// `getMostSignificantFall` (CombatTracker.java:108-141).
    fn most_significant_fall(&self) -> Option<&CombatEntry> {
        let mut result = None;
        let mut alternative: Option<&CombatEntry> = None;
        let mut alt_damage = 0.0_f32;
        let mut best_fall = 0.0_f32;
        for (index, entry) in self.entries.iter().enumerate() {
            let fall_distance = if entry.always_most_significant_fall {
                f32::MAX
            } else {
                entry.fall_distance
            };
            if (entry.is_fall || entry.always_most_significant_fall)
                && fall_distance > 0.0
                && (result.is_none() || fall_distance > best_fall)
            {
                result = Some(&self.entries[index.saturating_sub(1)]);
                best_fall = fall_distance;
            }
            if entry.fall_location.is_some() && (alternative.is_none() || entry.damage > alt_damage)
            {
                alternative = Some(entry);
                alt_damage = entry.damage;
            }
        }
        if best_fall > 5.0 && result.is_some() {
            result
        } else if alt_damage > 5.0 {
            alternative
        } else {
            None
        }
    }
}

/// `CombatTracker.getDeathMessage` (CombatTracker.java:88-106) for the blow
/// `source` that `combat` recorded last.
///
/// `kill_credit` is `getKillCredit()`, `causing` and `direct` are the display
/// names of the source's two entities. The `.item` variants read the
/// weapon the killing blow's entry recorded.
pub fn death_message(
    combat: &CombatTracker,
    source: &HurtSource<'_>,
    victim: NbtSummaryValue,
    kill_credit: Option<NbtSummaryValue>,
    causing: Option<NbtSummaryValue>,
    direct: Option<NbtSummaryValue>,
) -> RichTextComponent {
    death_message_with_registry(
        combat,
        source,
        victim,
        kill_credit,
        causing,
        direct,
        ContentRegistry::vanilla(),
    )
}

/// Content-registry-aware form of [`death_message`], accepting custom entities
/// in `show_entity` hover events and falling back to plain text instead of
/// panicking if a component fails validation.
pub fn death_message_with_registry(
    combat: &CombatTracker,
    source: &HurtSource<'_>,
    victim: NbtSummaryValue,
    kill_credit: Option<NbtSummaryValue>,
    causing: Option<NbtSummaryValue>,
    direct: Option<NbtSummaryValue>,
    content_registry: &ContentRegistry,
) -> RichTextComponent {
    let mut units = Vec::with_capacity(13 + source.damage_type.message_id.units().len());
    units.extend("death.attack.".encode_utf16());
    units.extend_from_slice(source.damage_type.message_id.units());
    let death_msg = JavaString::from(units);
    let death_key = |suffix: &str| {
        let mut key = death_msg.clone();
        key.push_str(suffix);
        key
    };
    let knock_off = combat.most_significant_fall();
    let killer_weapon = combat
        .entries
        .last()
        .and_then(|entry| entry.attacker_weapon.clone());
    let value = if combat.entries.is_empty() {
        translatable_with("death.attack.generic", vec![victim])
    } else if let (DeathMessageType::FallVariants, Some(knock_off)) =
        (source.damage_type.death_message_type, knock_off)
    {
        fall_message(knock_off, causing, killer_weapon, victim)
    } else if source.damage_type.death_message_type == DeathMessageType::IntentionalGameDesign {
        // `:98-101`.
        let link = component_translatable_nbt(
            "chat.square_brackets",
            vec![translatable(death_key(".link"))],
            &ComponentStyle {
                click_event: Some(ComponentClickEvent::OpenUrl {
                    url: INTENTIONAL_GAME_DESIGN_BUG_URL.into(),
                }),
                hover_event: Some(ComponentHoverEvent::ShowText {
                    value: Vec::new(),
                    component: Some(component_literal_nbt(INTENTIONAL_GAME_DESIGN_BUG_ID)),
                }),
                ..ComponentStyle::default()
            },
        )
        .expect("the intentional-game-design link style is representable");
        translatable_with(death_key(".message"), vec![victim, link])
    } else {
        // `DamageSource.getLocalizedDeathMessage` (DamageSource.java:71-86).
        match (
            causing.is_some().then_some(killer_weapon).flatten(),
            causing.or(direct),
        ) {
            (Some(weapon), Some(name)) => {
                translatable_with(death_key(".item"), vec![victim, name, weapon])
            }
            (None, Some(name)) => translatable_with(&death_msg, vec![victim, name]),
            (_, None) => match kill_credit {
                Some(credit) => translatable_with(death_key(".player"), vec![victim, credit]),
                None => translatable_with(&death_msg, vec![victim]),
            },
        }
    };
    RichTextComponent::try_from_component_nbt_with_registry(value.clone(), content_registry)
        .unwrap_or_else(|_| {
            RichTextComponent::plain(String::from_utf16_lossy(
                &crate::language::component_plain_text(&value),
            ))
        })
}

/// `CombatTracker.getFallMessage` (CombatTracker.java:62-80); `killer` is
/// the killing source's `getEntity()`.
fn fall_message(
    knock_off: &CombatEntry,
    killer: Option<NbtSummaryValue>,
    killer_weapon: Option<NbtSummaryValue>,
    victim: NbtSummaryValue,
) -> NbtSummaryValue {
    if knock_off.is_fall || knock_off.always_most_significant_fall {
        let location = knock_off.fall_location.unwrap_or(FallLocation::Generic);
        return translatable_with(location.language_key(), vec![victim]);
    }
    // `getMessageForAssistedFall` (CombatTracker.java:56-63).
    let assisted = |key: &str, name: NbtSummaryValue, weapon: Option<NbtSummaryValue>| match weapon
    {
        Some(weapon) => {
            translatable_with(format!("{key}.item"), vec![victim.clone(), name, weapon])
        }
        None => translatable_with(key, vec![victim.clone(), name]),
    };
    match (&knock_off.attacker, killer) {
        (Some(attacker), killer) if killer.as_ref() != Some(attacker) => assisted(
            "death.fell.assist",
            attacker.clone(),
            knock_off.attacker_weapon.clone(),
        ),
        (_, Some(killer)) => assisted("death.fell.finish", killer, killer_weapon),
        (_, None) => translatable_with("death.fell.killer", vec![victim]),
    }
}

fn translatable(key: impl Into<JavaString>) -> NbtSummaryValue {
    translatable_with(key, Vec::new())
}

fn translatable_with(
    key: impl Into<JavaString>,
    arguments: Vec<NbtSummaryValue>,
) -> NbtSummaryValue {
    component_translatable_nbt(key, arguments, &ComponentStyle::default())
        .expect("an unstyled translatable is always representable")
}

#[cfg(test)]
#[path = "damage/tests.rs"]
mod tests;
