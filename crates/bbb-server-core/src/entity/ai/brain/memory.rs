use std::fmt;

use bbb_protocol::nbt_summary::{
    codec_int_value, codec_long_value, uuid_from_words, uuid_nbt as uuid_int_array,
};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryValueKind {
    Unit,
    Bool,
    Int,
    Long,
    Uuid,
    BlockPos,
    GlobalPos,
    GlobalPosList,
    /// A value kind no codec this crate reads carries: the two memories that
    /// hold one (`HURT_BY`, `HURT_BY_ENTITY`) have no codec in Vanilla
    /// either, so they never reach the pack.
    DamageSource,
    EntityRef,
    /// `NEAREST_LIVING_ENTITIES`, a `List<LivingEntity>`
    /// registered without a codec.
    EntityList,
    /// `NEAREST_VISIBLE_LIVING_ENTITIES`
    /// registered without a codec.
    VisibleEntities,
    /// `WALK_TARGET`, registered without a codec.
    WalkTarget,
    /// `UNREACHABLE_TONGUE_TARGETS`, a `List<UUID>`
    /// registered without a codec.
    UuidList,
    /// `LOOK_TARGET`, a `PositionTracker`,
    /// registered without a codec.
    PositionTracker,
    /// `PATH`, a `Path`, registered without a
    /// codec.
    Path,
}

macro_rules! memory_module_types {
    ($(
        $variant:ident = $resource:literal $(, $kind:ident)?
    );+ $(;)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum MemoryModuleType {
            $($variant,)+
        }

        impl MemoryModuleType {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];

            pub const fn resource_id(self) -> &'static str {
                match self {
                    $(Self::$variant => $resource,)+
                }
            }

            pub(crate) fn registry_path(self) -> &'static str {
                match self {
                    $(Self::$variant => &$resource["minecraft:".len()..],)+
                }
            }

            pub const fn codec_kind(self) -> Option<MemoryValueKind> {
                match self {
                    $(Self::$variant => memory_module_types!(@kind $($kind)?),)+
                }
            }

            pub fn from_resource_id(resource_id: &str) -> Option<Self> {
                match resource_id {
                    $($resource => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
    (@kind) => { None };
    (@kind $kind:ident) => { Some(MemoryValueKind::$kind) };
}

// Registration order.
memory_module_types! {
    Dummy = "minecraft:dummy";
    Home = "minecraft:home", GlobalPos;
    JobSite = "minecraft:job_site", GlobalPos;
    PotentialJobSite = "minecraft:potential_job_site", GlobalPos;
    MeetingPoint = "minecraft:meeting_point", GlobalPos;
    SecondaryJobSite = "minecraft:secondary_job_site";
    NearestLivingEntities = "minecraft:mobs";
    NearestVisibleLivingEntities = "minecraft:visible_mobs";
    VisibleVillagerBabies = "minecraft:visible_villager_babies";
    NearestPlayers = "minecraft:nearest_players";
    NearestVisiblePlayer = "minecraft:nearest_visible_player";
    NearestVisibleAttackablePlayer = "minecraft:nearest_visible_targetable_player";
    NearestVisibleAttackablePlayers = "minecraft:nearest_visible_targetable_players";
    WalkTarget = "minecraft:walk_target";
    LookTarget = "minecraft:look_target";
    AttackTarget = "minecraft:attack_target";
    AttackCoolingDown = "minecraft:attack_cooling_down";
    InteractionTarget = "minecraft:interaction_target";
    BreedTarget = "minecraft:breed_target";
    RideTarget = "minecraft:ride_target";
    Path = "minecraft:path";
    DoorsToClose = "minecraft:doors_to_close";
    NearestBed = "minecraft:nearest_bed";
    HurtBy = "minecraft:hurt_by";
    HurtByEntity = "minecraft:hurt_by_entity";
    AvoidTarget = "minecraft:avoid_target";
    NearestHostile = "minecraft:nearest_hostile";
    NearestAttackable = "minecraft:nearest_attackable";
    HidingPlace = "minecraft:hiding_place";
    HeardBellTime = "minecraft:heard_bell_time";
    CantReachWalkTargetSince = "minecraft:cant_reach_walk_target_since";
    GolemDetectedRecently = "minecraft:golem_detected_recently", Bool;
    DangerDetectedRecently = "minecraft:danger_detected_recently", Bool;
    LastSlept = "minecraft:last_slept", Long;
    LastWoken = "minecraft:last_woken", Long;
    LastWorkedAtPoi = "minecraft:last_worked_at_poi", Long;
    NearestVisibleAdult = "minecraft:nearest_visible_adult";
    NearestVisibleWantedItem = "minecraft:nearest_visible_wanted_item";
    NearestVisibleNemesis = "minecraft:nearest_visible_nemesis";
    PlayDeadTicks = "minecraft:play_dead_ticks", Int;
    TemptingPlayer = "minecraft:tempting_player";
    TemptationCooldownTicks = "minecraft:temptation_cooldown_ticks", Int;
    GazeCooldownTicks = "minecraft:gaze_cooldown_ticks", Int;
    IsTempted = "minecraft:is_tempted", Bool;
    LongJumpCooldownTicks = "minecraft:long_jump_cooling_down", Int;
    LongJumpMidJump = "minecraft:long_jump_mid_jump";
    HasHuntingCooldown = "minecraft:has_hunting_cooldown", Bool;
    RamCooldownTicks = "minecraft:ram_cooldown_ticks", Int;
    RamTarget = "minecraft:ram_target";
    IsInWater = "minecraft:is_in_water", Unit;
    IsPregnant = "minecraft:is_pregnant", Unit;
    IsPanicking = "minecraft:is_panicking", Bool;
    UnreachableTongueTargets = "minecraft:unreachable_tongue_targets";
    VisitedBlockPositions = "minecraft:visited_block_positions", GlobalPosList;
    UnreachableTransportBlockPositions = "minecraft:unreachable_transport_block_positions", GlobalPosList;
    TransportItemsCooldownTicks = "minecraft:transport_items_cooldown_ticks";
    ChargeCooldownTicks = "minecraft:charge_cooldown_ticks", Int;
    AttackTargetCooldown = "minecraft:attack_target_cooldown", Int;
    SpearFleeingTime = "minecraft:spear_fleeing_time";
    SpearFleeingPosition = "minecraft:spear_fleeing_position";
    SpearChargePosition = "minecraft:spear_charge_position";
    SpearEngageTime = "minecraft:spear_engage_time";
    SpearStatus = "minecraft:spear_status";
    AngryAt = "minecraft:angry_at", Uuid;
    UniversalAnger = "minecraft:universal_anger", Bool;
    AdmiringItem = "minecraft:admiring_item", Bool;
    TimeTryingToReachAdmireItem = "minecraft:time_trying_to_reach_admire_item";
    DisableWalkToAdmireItem = "minecraft:disable_walk_to_admire_item";
    AdmiringDisabled = "minecraft:admiring_disabled", Bool;
    HuntedRecently = "minecraft:hunted_recently", Bool;
    CelebrateLocation = "minecraft:celebrate_location";
    Dancing = "minecraft:dancing";
    NearestVisibleHuntableHoglin = "minecraft:nearest_visible_huntable_hoglin";
    NearestVisibleBabyHoglin = "minecraft:nearest_visible_baby_hoglin";
    NearestTargetablePlayerNotWearingGold = "minecraft:nearest_targetable_player_not_wearing_gold";
    NearbyAdultPiglins = "minecraft:nearby_adult_piglins";
    NearestVisibleAdultPiglins = "minecraft:nearest_visible_adult_piglins";
    NearestVisibleAdultHoglins = "minecraft:nearest_visible_adult_hoglins";
    NearestVisibleAdultPiglin = "minecraft:nearest_visible_adult_piglin";
    NearestVisibleZombified = "minecraft:nearest_visible_zombified";
    VisibleAdultPiglinCount = "minecraft:visible_adult_piglin_count";
    VisibleAdultHoglinCount = "minecraft:visible_adult_hoglin_count";
    NearestPlayerHoldingWantedItem = "minecraft:nearest_player_holding_wanted_item";
    AteRecently = "minecraft:ate_recently";
    NearestRepellent = "minecraft:nearest_repellent";
    Pacified = "minecraft:pacified";
    RoarTarget = "minecraft:roar_target";
    DisturbanceLocation = "minecraft:disturbance_location";
    RecentProjectile = "minecraft:recent_projectile", Unit;
    IsSniffing = "minecraft:is_sniffing", Unit;
    IsEmerging = "minecraft:is_emerging", Unit;
    RoarSoundDelay = "minecraft:roar_sound_delay", Unit;
    DigCooldown = "minecraft:dig_cooldown", Unit;
    RoarSoundCooldown = "minecraft:roar_sound_cooldown", Unit;
    SniffCooldown = "minecraft:sniff_cooldown", Unit;
    TouchCooldown = "minecraft:touch_cooldown", Unit;
    VibrationCooldown = "minecraft:vibration_cooldown", Unit;
    SonicBoomCooldown = "minecraft:sonic_boom_cooldown", Unit;
    SonicBoomSoundCooldown = "minecraft:sonic_boom_sound_cooldown", Unit;
    SonicBoomSoundDelay = "minecraft:sonic_boom_sound_delay", Unit;
    LikedPlayer = "minecraft:liked_player", Uuid;
    LikedNoteblockPosition = "minecraft:liked_noteblock", GlobalPos;
    LikedNoteblockCooldownTicks = "minecraft:liked_noteblock_cooldown_ticks", Int;
    ItemPickupCooldownTicks = "minecraft:item_pickup_cooldown_ticks", Int;
    SnifferExploredPositions = "minecraft:sniffer_explored_positions", GlobalPosList;
    SnifferSniffingTarget = "minecraft:sniffer_sniffing_target";
    SnifferDigging = "minecraft:sniffer_digging";
    SnifferHappy = "minecraft:sniffer_happy";
    BreezeJumpCooldown = "minecraft:breeze_jump_cooldown", Unit;
    BreezeShoot = "minecraft:breeze_shoot", Unit;
    BreezeShootCharging = "minecraft:breeze_shoot_charging", Unit;
    BreezeShootRecovering = "minecraft:breeze_shoot_recover", Unit;
    BreezeShootCooldown = "minecraft:breeze_shoot_cooldown", Unit;
    BreezeJumpInhaling = "minecraft:breeze_jump_inhaling", Unit;
    BreezeJumpTarget = "minecraft:breeze_jump_target", BlockPos;
    BreezeLeavingWater = "minecraft:breeze_leaving_water", Unit;
}

impl MemoryModuleType {
    pub const fn can_serialize(self) -> bool {
        self.codec_kind().is_some()
    }
}

impl fmt::Display for MemoryModuleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.resource_id())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryGlobalPos {
    pub dimension: String,
    pub pos: BlockPos,
}

/// `LivingEntity.getLastDamageSource()` as
/// much as this crate retains: the damage type's registry id and the causing
/// entity `DamageSource.getEntity()` answers. The direct entity has no owner
/// here, so `DamageSource.getDirectEntity()` is not part of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrainDamageSource {
    pub damage_type: String,
    /// `DamageSource.getEntity().getId()`, which this crate keeps as the
    /// level's entity id for a player and a stored mob alike.
    pub source_entity: Option<i32>,
}

/// `PositionTracker`, the thing a `WalkTarget`
/// holds instead of a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPositionTracker {
    /// `BlockPosTracker`, which keeps the
    /// position it was built with.
    BlockPos(BlockPos),
    /// `EntityTracker`, whose
    /// `currentBlockPosition()` reads the tracked entity on every call.
    /// Named by the level's entity id, as
    /// [`MemoryValue::EntityTracker`] is. Every tracker this runtime builds
    /// carries `targetEyeHeight` false, so the block position is the entity's
    /// own.
    ///
    /// `last_seen` is what the tracker answers once the level no longer places
    /// that entity: Vanilla holds the `Entity` object itself, which keeps
    /// answering its final `blockPosition()` after removal.
    Entity { entity_id: i32, last_seen: BlockPos },
}

/// `WalkTarget` as much as this crate stores: the
/// position tracker, the speed multiplier and the close-enough distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryWalkTarget {
    pub target: MemoryPositionTracker,
    pub speed_modifier: f32,
    pub close_enough_dist: i32,
}

impl MemoryWalkTarget {
    /// `new WalkTarget(BlockPos, float, int)` (`WalkTarget.java:15-17`), which
    /// wraps the block in a `BlockPosTracker`.
    #[must_use]
    pub const fn at(pos: BlockPos, speed_modifier: f32, close_enough_dist: i32) -> Self {
        Self {
            target: MemoryPositionTracker::BlockPos(pos),
            speed_modifier,
            close_enough_dist,
        }
    }
}

/// `Path` as much as a memory reader sees of it: the
/// target it was built for (`Path.getTarget`), its node count and whether the
/// search reached the target, which is what `Path.toString`
/// and `Path.canReach` expose. The nodes stay with the navigation that owns
/// them (`BrainWorld::body_path`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryPath {
    pub target: BlockPos,
    pub length: usize,
    pub reached: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemoryValue {
    Unit,
    Bool(bool),
    Int(i32),
    Long(i64),
    Uuid(Uuid),
    BlockPos(BlockPos),
    GlobalPos(MemoryGlobalPos),
    GlobalPosList(Vec<MemoryGlobalPos>),
    /// Unserialized per-level POI list (e.g. `SECONDARY_JOB_SITE`), storing the
    /// shared dimension once instead of cloning it per block position.
    SameDimensionPosList {
        dimension: String,
        positions: Vec<BlockPos>,
    },
    /// `MemoryModuleType.HURT_BY`'s `DamageSource`, which has no codec in
    /// Vanilla either, so it never reaches the pack.
    DamageSource(BrainDamageSource),
    /// A memory holding another entity, such as
    /// `MemoryModuleType.HURT_BY_ENTITY`, named by the level's entity id.
    EntityRef(i32),
    /// `NEAREST_LIVING_ENTITIES`, the living entities of the scan box in
    /// `distanceToSqr` order.
    EntityList(Vec<i32>),
    /// `NEAREST_VISIBLE_LIVING_ENTITIES`: Vanilla's
    /// `NearestVisibleLivingEntities` holds the same list plus a line-of-sight
    /// predicate that reads the level and the body, so this crate keeps the
    /// list and asks the world for the predicate when it is used
    VisibleEntities(Vec<i32>),
    /// `WALK_TARGET`.
    WalkTarget(MemoryWalkTarget),
    /// `UNREACHABLE_TONGUE_TARGETS`, the UUIDs `ShootTongue` refused
    UuidList(Vec<Uuid>),
    /// `LOOK_TARGET` as the `EntityTracker` `BehaviorUtils.lookAtEntity`
    /// writes, named by the level's entity id.
    EntityTracker(i32),
    /// `PATH`, the route the navigation is following.
    Path(MemoryPath),
}

impl MemoryValue {
    pub fn kind(&self) -> MemoryValueKind {
        match self {
            Self::Unit => MemoryValueKind::Unit,
            Self::Bool(_) => MemoryValueKind::Bool,
            Self::Int(_) => MemoryValueKind::Int,
            Self::Long(_) => MemoryValueKind::Long,
            Self::Uuid(_) => MemoryValueKind::Uuid,
            Self::BlockPos(_) => MemoryValueKind::BlockPos,
            Self::GlobalPos(_) => MemoryValueKind::GlobalPos,
            Self::GlobalPosList(_) | Self::SameDimensionPosList { .. } => {
                MemoryValueKind::GlobalPosList
            }
            Self::DamageSource(_) => MemoryValueKind::DamageSource,
            Self::EntityRef(_) => MemoryValueKind::EntityRef,
            Self::EntityList(_) => MemoryValueKind::EntityList,
            Self::VisibleEntities(_) => MemoryValueKind::VisibleEntities,
            Self::WalkTarget(_) => MemoryValueKind::WalkTarget,
            Self::UuidList(_) => MemoryValueKind::UuidList,
            Self::EntityTracker(_) => MemoryValueKind::PositionTracker,
            Self::Path(_) => MemoryValueKind::Path,
        }
    }

    /// The entity a memory names, for a value that holds one.
    pub const fn entity_ref(&self) -> Option<i32> {
        match self {
            Self::EntityRef(entity_id) | Self::EntityTracker(entity_id) => Some(*entity_id),
            _ => None,
        }
    }

    /// The entities a memory names, for a value that holds a list of them.
    pub fn entity_refs(&self) -> &[i32] {
        match self {
            Self::EntityList(ids) | Self::VisibleEntities(ids) => ids,
            _ => &[],
        }
    }

    fn is_empty_collection(&self) -> bool {
        match self {
            Self::GlobalPosList(values) => values.is_empty(),
            Self::SameDimensionPosList { positions, .. } => positions.is_empty(),
            // `Brain.isEmptyCollection` erases a slot a list would empty
            Self::EntityList(values) | Self::VisibleEntities(values) => values.is_empty(),
            Self::UuidList(values) => values.is_empty(),
            _ => false,
        }
    }

    fn encode(&self) -> NbtSummaryValue {
        match self {
            Self::Unit => NbtSummaryValue::Compound(Vec::new()),
            Self::Bool(value) => NbtSummaryValue::Byte(i8::from(*value)),
            Self::Int(value) => NbtSummaryValue::Int(*value),
            Self::Long(value) => NbtSummaryValue::Long(*value),
            Self::Uuid(value) => uuid_int_array(*value),
            Self::BlockPos(pos) => NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z]),
            Self::GlobalPos(value) => encode_global_pos(value),
            Self::GlobalPosList(values) => {
                NbtSummaryValue::List(values.iter().map(encode_global_pos).collect())
            }
            // `Brain.pack` skips a module whose `canSerialize` is false
            // which is every module these can
            // live in, so no pack asks for them.
            Self::SameDimensionPosList { .. }
            | Self::DamageSource(_)
            | Self::EntityRef(_)
            | Self::EntityList(_)
            | Self::VisibleEntities(_)
            | Self::WalkTarget(_)
            | Self::UuidList(_)
            | Self::EntityTracker(_)
            | Self::Path(_) => {
                unreachable!("a memory value with no codec kind is never packed")
            }
        }
    }

    fn decode(
        kind: MemoryValueKind,
        value: &NbtSummaryValue,
        path: &str,
    ) -> Result<Self, BrainCodecError> {
        match kind {
            MemoryValueKind::Unit => {
                let fields = expect_compound(value, path)?;
                if !fields.is_empty() {
                    return Err(BrainCodecError::new(
                        path,
                        "Unit memory value must be an empty compound",
                    ));
                }
                Ok(Self::Unit)
            }
            MemoryValueKind::Bool => match value {
                NbtSummaryValue::Byte(0) => Ok(Self::Bool(false)),
                NbtSummaryValue::Byte(1) => Ok(Self::Bool(true)),
                _ => Err(BrainCodecError::new(path, "must be boolean byte 0 or 1")),
            },
            MemoryValueKind::Int => codec_int_value(value)
                .map(Self::Int)
                .ok_or_else(|| BrainCodecError::new(path, "must be a number")),
            MemoryValueKind::Long => codec_long_value(value)
                .map(Self::Long)
                .ok_or_else(|| BrainCodecError::new(path, "must be a number")),
            MemoryValueKind::Uuid => Ok(Self::Uuid(decode_uuid(value, path)?)),
            MemoryValueKind::BlockPos => Ok(Self::BlockPos(decode_block_pos(value, path)?)),
            MemoryValueKind::GlobalPos => Ok(Self::GlobalPos(decode_global_pos(value, path)?)),
            MemoryValueKind::GlobalPosList => {
                let NbtSummaryValue::List(values) = value else {
                    return Err(BrainCodecError::new(path, "must be a list"));
                };
                let mut positions = Vec::with_capacity(values.len());
                for (index, entry) in values.iter().enumerate() {
                    positions.push(decode_global_pos(entry, &format!("{path}[{index}]"))?);
                }
                Ok(Self::GlobalPosList(positions))
            }
            MemoryValueKind::DamageSource
            | MemoryValueKind::EntityRef
            | MemoryValueKind::EntityList
            | MemoryValueKind::VisibleEntities
            | MemoryValueKind::WalkTarget
            | MemoryValueKind::UuidList
            | MemoryValueKind::PositionTracker
            | MemoryValueKind::Path => Err(BrainCodecError::new(
                path,
                "memory value has no persistent encoding",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryStatus {
    ValuePresent,
    ValueAbsent,
    Registered,
}

#[derive(Clone)]
pub(super) struct MemorySlot {
    value: Option<MemoryValue>,
    time_to_live: i64,
}

impl MemorySlot {
    const NEVER_EXPIRE: i64 = i64::MAX;

    pub(super) fn create() -> Self {
        Self {
            value: None,
            time_to_live: Self::NEVER_EXPIRE,
        }
    }

    // `MemorySlot.tick`.
    pub(super) fn tick(&mut self) {
        if self.has_value() && self.can_expire() {
            if self.has_expired() {
                self.clear();
            } else {
                self.time_to_live -= 1;
            }
        }
    }

    pub(super) fn set(&mut self, value: MemoryValue, time_to_live: i64) {
        self.value = Some(value);
        self.time_to_live = time_to_live;
    }

    pub(super) fn clear(&mut self) {
        self.value = None;
        self.time_to_live = Self::NEVER_EXPIRE;
    }

    pub(super) fn has_value(&self) -> bool {
        self.value.is_some()
    }

    pub(super) fn value(&self) -> Option<&MemoryValue> {
        self.value.as_ref()
    }

    pub(super) fn can_expire(&self) -> bool {
        self.time_to_live != Self::NEVER_EXPIRE
    }

    pub(super) fn has_expired(&self) -> bool {
        self.time_to_live <= 0
    }

    pub(super) fn time_to_live(&self) -> i64 {
        self.time_to_live
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackedMemory {
    pub memory_type: MemoryModuleType,
    pub value: MemoryValue,
    pub time_to_live: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BrainPacked {
    pub memories: Vec<PackedMemory>,
}

impl BrainPacked {
    pub const fn empty() -> Self {
        Self {
            memories: Vec::new(),
        }
    }

    // `Brain.Packed.CODEC`.
    // `TagValueInput.read` keeps the dispatched map's partial value, so an
    // unreadable memory is dropped and the rest load.
    pub fn decode(value: &NbtSummaryValue) -> Result<Self, BrainCodecError> {
        let fields = expect_compound(value, "Brain")?;
        reject_unknown_fields(fields, &["memories"], "Brain")?;
        let memories_value = required_field(fields, "memories", "Brain")?;
        let memories = expect_compound(memories_value, "Brain.memories")?;
        let mut packed: Vec<PackedMemory> = Vec::with_capacity(memories.len());
        for entry in memories {
            let memory_path = format!("Brain.memories.{}", entry.name.to_display_string());
            let Some(Ok(memory)) = entry
                .name
                .with_utf8(|name| decode_packed_memory(name, &entry.value, &memory_path))
            else {
                continue;
            };
            if packed
                .iter()
                .all(|kept| kept.memory_type != memory.memory_type)
            {
                packed.push(memory);
            }
        }
        Ok(Self { memories: packed })
    }

    pub fn encode(&self) -> NbtSummaryValue {
        let mut memories = self
            .memories
            .iter()
            .map(|memory| {
                (
                    memory.memory_type.resource_id(),
                    encode_expirable_value(&memory.value, memory.time_to_live),
                )
            })
            .collect::<Vec<_>>();
        memories.sort_by(|left, right| left.0.cmp(right.0));
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "memories".into(),
            value: NbtSummaryValue::Compound(
                memories
                    .into_iter()
                    .map(|(name, value)| NbtSummaryEntry {
                        name: name.into(),
                        value,
                    })
                    .collect(),
            ),
        }])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{path}: {detail}")]
pub struct BrainCodecError {
    pub path: String,
    pub detail: String,
}

impl BrainCodecError {
    fn new(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            detail: detail.into(),
        }
    }
}

pub(super) fn apply_value_to_slot(
    slot: &mut MemorySlot,
    value: MemoryValue,
    time_to_live: Option<i64>,
) {
    if value.is_empty_collection() {
        slot.clear();
        return;
    }
    match time_to_live {
        Some(ttl) => slot.set(value, ttl),
        None => slot.set(value, MemorySlot::NEVER_EXPIRE),
    }
}

fn decode_packed_memory(
    raw_id: &str,
    value: &NbtSummaryValue,
    path: &str,
) -> Result<PackedMemory, BrainCodecError> {
    let resource_id = normalize_resource_location(raw_id.to_string()).map_err(|error| {
        BrainCodecError::new(path, format!("invalid memory module resource id: {error}"))
    })?;
    let memory_type = MemoryModuleType::from_resource_id(&resource_id).ok_or_else(|| {
        BrainCodecError::new(path, format!("unknown memory module {resource_id}"))
    })?;
    let Some(kind) = memory_type.codec_kind() else {
        return Err(BrainCodecError::new(
            path,
            format!("Memory module {memory_type} cannot be encoded"),
        ));
    };
    let fields = expect_compound(value, path)?;
    reject_unknown_fields(fields, &["value", "ttl"], path)?;
    let decoded = MemoryValue::decode(
        kind,
        required_field(fields, "value", path)?,
        &format!("{path}.value"),
    )?;
    if decoded.kind() != kind {
        return Err(BrainCodecError::new(path, "memory value kind mismatch"));
    }
    let time_to_live = match unique_field(fields, "ttl", path)? {
        Some(ttl) => Some(
            codec_long_value(ttl)
                .ok_or_else(|| BrainCodecError::new(format!("{path}.ttl"), "must be a number"))?,
        ),
        None => None,
    };
    Ok(PackedMemory {
        memory_type,
        value: decoded,
        time_to_live,
    })
}

fn encode_expirable_value(value: &MemoryValue, time_to_live: Option<i64>) -> NbtSummaryValue {
    let mut fields = vec![NbtSummaryEntry {
        name: "value".into(),
        value: value.encode(),
    }];
    if let Some(ttl) = time_to_live.filter(|ttl| *ttl != MemorySlot::NEVER_EXPIRE) {
        fields.push(NbtSummaryEntry {
            name: "ttl".into(),
            value: NbtSummaryValue::Long(ttl),
        });
    }
    NbtSummaryValue::Compound(fields)
}

fn encode_global_pos(value: &MemoryGlobalPos) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "dimension".into(),
            value: NbtSummaryValue::string(value.dimension.clone()),
        },
        NbtSummaryEntry {
            name: "pos".into(),
            value: NbtSummaryValue::IntArray(vec![value.pos.x, value.pos.y, value.pos.z]),
        },
    ])
}

fn decode_global_pos(
    value: &NbtSummaryValue,
    path: &str,
) -> Result<MemoryGlobalPos, BrainCodecError> {
    let fields = expect_compound(value, path)?;
    reject_unknown_fields(fields, &["dimension", "pos"], path)?;
    let raw = expect_string(
        required_field(fields, "dimension", path)?,
        &format!("{path}.dimension"),
    )?;
    let dimension = normalize_resource_location(raw.to_string()).map_err(|error| {
        BrainCodecError::new(
            format!("{path}.dimension"),
            format!("invalid resource id: {error}"),
        )
    })?;
    Ok(MemoryGlobalPos {
        dimension,
        pos: decode_block_pos(required_field(fields, "pos", path)?, &format!("{path}.pos"))?,
    })
}

fn decode_block_pos(value: &NbtSummaryValue, path: &str) -> Result<BlockPos, BrainCodecError> {
    let NbtSummaryValue::IntArray(values) = value else {
        return Err(BrainCodecError::new(path, "BlockPos must be an int array"));
    };
    let [x, y, z] = values.as_slice() else {
        return Err(BrainCodecError::new(
            path,
            "BlockPos must contain exactly three ints",
        ));
    };
    Ok(BlockPos {
        x: *x,
        y: *y,
        z: *z,
    })
}

fn decode_uuid(value: &NbtSummaryValue, path: &str) -> Result<Uuid, BrainCodecError> {
    let NbtSummaryValue::IntArray(values) = value else {
        return Err(BrainCodecError::new(path, "must be a UUID int array"));
    };
    if values.len() != 4 {
        return Err(BrainCodecError::new(path, "must contain exactly four ints"));
    }
    Ok(uuid_from_words([
        values[0], values[1], values[2], values[3],
    ]))
}

fn expect_compound<'a>(
    value: &'a NbtSummaryValue,
    path: &str,
) -> Result<&'a [NbtSummaryEntry], BrainCodecError> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(BrainCodecError::new(path, "must be a compound"));
    };
    Ok(entries)
}

fn expect_string(value: &NbtSummaryValue, path: &str) -> Result<String, BrainCodecError> {
    let NbtSummaryValue::String(value) = value else {
        return Err(BrainCodecError::new(path, "must be a string"));
    };
    value
        .try_to_string()
        .map_err(|_| BrainCodecError::new(path, "must be Unicode text"))
}

fn unique_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
    parent_path: &str,
) -> Result<Option<&'a NbtSummaryValue>, BrainCodecError> {
    let mut matches = entries.iter().filter(|entry| entry.name == name);
    let value = matches.next().map(|entry| &entry.value);
    if matches.next().is_some() {
        return Err(BrainCodecError::new(
            format!("{parent_path}.{name}"),
            "field must occur exactly once",
        ));
    }
    Ok(value)
}

fn required_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
    parent_path: &str,
) -> Result<&'a NbtSummaryValue, BrainCodecError> {
    unique_field(entries, name, parent_path)?.ok_or_else(|| {
        BrainCodecError::new(format!("{parent_path}.{name}"), "required field is missing")
    })
}

fn reject_unknown_fields(
    entries: &[NbtSummaryEntry],
    allowed: &[&str],
    path: &str,
) -> Result<(), BrainCodecError> {
    for entry in entries {
        if !allowed.iter().any(|name| entry.name == *name) {
            return Err(BrainCodecError::new(
                format!("{path}.{}", entry.name.to_display_string()),
                "unknown codec field",
            ));
        }
    }
    Ok(())
}
