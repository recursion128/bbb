//! The values commands resolve and the world state rows they read.
//!
//! A command reads `ServerWorld` directly and writes it through
//! [`super::CommandCx`]; [`CommandOrigin`] is where its source stands.

use std::sync::Arc;

use bbb_facts::boss_bar_styles::{BossBarColor, BossBarOverlay};
use bbb_protocol::{
    entity_anchors::EntityAnchor,
    game_types::GameType,
    packets::{BlockPos, Vec3d, WaypointIcon},
    NbtSummaryValue, RichTextComponent,
};

use crate::admin::NameAndId;
use crate::entity::living::mob_effects::ActiveMobEffects;
use crate::entity::{AttributeMap, Entity, EntityRegistryTagView};
use crate::registry_tags::RegistryTagSnapshot;
use bbb_server_datapack::damage::PlayerDamageState;
use bbb_server_datapack::pack::ServerPackSource;
use bbb_server_types::experience::PlayerExperience;
use bbb_server_types::save::Java26LevelKey;

use crate::world::ServerWorld;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPlayerTarget {
    pub profile: NameAndId,
}

impl std::ops::Deref for CommandPlayerTarget {
    type Target = NameAndId;

    fn deref(&self) -> &Self::Target {
        &self.profile
    }
}

impl From<NameAndId> for CommandPlayerTarget {
    fn from(profile: NameAndId) -> Self {
        Self { profile }
    }
}

/// One online player borrowed from the live session.
#[derive(Debug, Clone, Copy)]
pub struct CommandPlayer<'a> {
    pub id: uuid::Uuid,
    pub name: &'a str,
    pub game_type: GameType,
    pub position: [f64; 3],
    pub rotation: [f32; 2],
    pub pose_id: i32,
    pub eye_height: f32,
    pub dimension: &'a Java26LevelKey,
    pub address: Option<&'a str>,
    pub entity_id: i32,
    pub damage: PlayerDamageState,
    pub removed: bool,
    pub tags: &'a [bbb_protocol::JavaString],
    pub camera: Option<&'a CommandCameraTarget>,
    pub experience: PlayerExperience,
    pub attributes: &'a AttributeMap,
    pub inventory: CommandInventory<'a>,
    pub effects: &'a ActiveMobEffects,
    pub advancements: &'a bbb_server_datapack::player_advancements::PlayerAdvancements,
}

/// A player's inventory and carried stack.
#[derive(Debug, Clone, Copy)]
pub struct CommandInventory<'a>(pub(crate) &'a crate::inventory::PlayerInventoryMenu);

impl CommandInventory<'_> {
    /// Player.java:1705-1742.
    pub(crate) fn slot(self, slot: i32) -> Option<bbb_server_types::item_stack::RuntimeItemStack> {
        use super::PlayerSlot;
        let inventory = self.0;
        match super::player_slot(slot, inventory.selected_slot())? {
            PlayerSlot::Cursor => Some(inventory.carried().clone()),
            PlayerSlot::Crafting(index) => inventory.crafting_slot(index),
            PlayerSlot::EnderChest(index) => inventory.ender_chest_slot(index),
            PlayerSlot::Inventory(index) => inventory.slot(index).cloned(),
        }
    }
}

impl CommandPlayer<'_> {
    pub fn user(&self) -> NameAndId {
        NameAndId {
            id: self.id,
            name: self.name.into(),
        }
    }

    pub fn target(&self) -> CommandPlayerTarget {
        CommandPlayerTarget {
            profile: self.user(),
        }
    }
}

/// One entity borrowed from a served level.
#[derive(Debug, Clone, Copy)]
pub struct CommandStoredEntity<'a> {
    pub entity: &'a Entity,
    pub dimension: &'a Java26LevelKey,
    /// The level's datapack registry tags; `None` reads the built-in 26.1 set.
    pub registry_tags: Option<&'a RegistryTagSnapshot>,
}

impl CommandStoredEntity<'_> {
    pub(crate) fn tag_contains(&self, registry: &str, tag: &str, element: &str) -> bool {
        match self.registry_tags {
            Some(tags) => tags.contains(registry, tag, element),
            None => EntityRegistryTagView::Builtin26_1.contains(registry, tag, element),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecuteRelation {
    Owner,
    Leasher,
    Target,
    Attacker,
    Vehicle,
    Controller,
    Origin,
    Passengers,
}

#[derive(Clone, Copy)]
pub(crate) enum CommandDamageEntityRef<'a> {
    Player(CommandPlayer<'a>),
    Stored(CommandStoredEntity<'a>),
}

impl CommandDamageEntityRef<'_> {
    pub(crate) fn uuid(self) -> uuid::Uuid {
        match self {
            Self::Player(player) => player.id,
            Self::Stored(stored) => stored.entity.uuid(),
        }
    }

    /// `Entity.getScoreboardName` (Entity.java:3190-3192) and its `Player`
    /// override (Player.java:1689-1691).
    pub(crate) fn with_scoreboard_name<T>(self, apply: impl FnOnce(&str) -> T) -> T {
        match self {
            Self::Player(player) => apply(player.name),
            Self::Stored(stored) => {
                let mut buffer = uuid::Uuid::encode_buffer();
                apply(stored.entity.uuid().hyphenated().encode_lower(&mut buffer))
            }
        }
    }
}

/// Canonical command NBT for one entity a selector matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEntityDataState {
    pub uuid: uuid::Uuid,
    pub data: NbtSummaryValue,
}

/// One entity occupying a role in the `DamageSource` built by `/damage`.
///
/// Player identity is retained because creative `instabuild` bypasses an
/// entity's base invulnerability check. Stored sources retain both their
/// runtime class and resource identity for mob-griefing and subtype checks.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandDamageSourceEntity {
    Player {
        user: CommandPlayerTarget,
        entity_id: i32,
        /// `Entity.position()` captured with the source identity.
        position: Vec3d,
        instabuild: bool,
        /// `Abilities.mayBuild` as `GameType.updatePlayerAbilities` derives it
        /// (`GameType.java:79-84`).
        may_build: bool,
        spectator: bool,
        display_name: NbtSummaryValue,
    },
    Stored {
        entity_id: i32,
        /// Normal command resolution always captures both. `None` only lets a
        /// live effect fail closed after an older projectile source vanished.
        uuid: Option<uuid::Uuid>,
        position: Option<Vec3d>,
        /// The source snapshot's canonical entity-type resource id.
        entity_resource_id: String,
        /// Whether the source is a `Mob`; mannequin is living but false.
        is_mob: bool,
        /// `DamageSource.scalesWithDifficulty` (`DamageSource.java:92-97`).
        is_living_non_player: bool,
        /// `Entity.getDisplayName()` as captured by command resolution. The
        /// live damage owner needs this even though stored source entities are
        /// not projected into the player-only PVP/creative/kill-credit facts.
        display_name: NbtSummaryValue,
    },
}

impl CommandDamageSourceEntity {
    pub const fn entity_id(&self) -> i32 {
        match self {
            Self::Player { entity_id, .. } | Self::Stored { entity_id, .. } => *entity_id,
        }
    }

    pub const fn display_name(&self) -> &NbtSummaryValue {
        match self {
            Self::Player { display_name, .. } | Self::Stored { display_name, .. } => display_name,
        }
    }
}

/// `ServerPlayerGameMode.changeGameModeForPlayer`'s early return
/// (ServerPlayerGameMode.java:56-58): a session already in the mode is not
/// moved, and every count of moves is a count of exactly this.
///
/// Both readers go through here rather than spelling the comparison out twice.
/// `/defaultgamemode` returns the count
/// (DefaultGameModeCommands.java:23, `MinecraftServer.java:1396-1399`) and the
/// world performs the moves it counted, so the number the command reports
/// cannot drift from the number of sessions that actually moved.
pub fn game_mode_change_moves_session(current: GameType, target: GameType) -> bool {
    current != target
}

/// The function library as a command reads it.
///
/// `ServerFunctionManager` answers four questions about it and every one of
/// them is a read: `get` (ServerFunctionManager.java:92-94), `getTag`
/// (`:96-98`), `getFunctionNames` (`:100-102`) and `getTagNames` (`:104-106`).
/// `FunctionArgument` asks the first two when the command runs
/// (FunctionArgument.java:76-87) and `SUGGEST_FUNCTION` asks the other two
/// while it is being typed (FunctionCommand.java:52-56). A macro body is made
/// by the command that instantiates it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandFunctionTemplate {
    pub source: Arc<[u16]>,
    pub segments: Vec<std::ops::Range<usize>>,
    pub parameter_slots: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandFunctionEntry {
    Plain(Arc<[u16]>),
    Macro(CommandFunctionTemplate),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandFunctionBody {
    Plain(Vec<Arc<[u16]>>),
    Macro {
        entries: Vec<CommandFunctionEntry>,
        parameters: Vec<bbb_protocol::JavaString>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandFunctionDefinition {
    pub id: String,
    pub body: CommandFunctionBody,
}

impl CommandFunctionDefinition {
    pub fn is_macro(&self) -> bool {
        matches!(self.body, CommandFunctionBody::Macro { .. })
    }

    pub fn plain_entries(&self) -> &[Arc<[u16]>] {
        match &self.body {
            CommandFunctionBody::Plain(entries) => entries,
            CommandFunctionBody::Macro { .. } => {
                unreachable!("macros are substituted before execution")
            }
        }
    }
}

/// One `Pack` as the `/datapack` readers see it (`Pack.java:21-122`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDatapack {
    pub id: String,
    pub source: ServerPackSource,
    pub description: CommandPackDescription,
    /// `Pack.getRequestedFeatures` (`Pack.java:92-94`), which `getPack`
    /// (`DataPackCommand.java:289-294`) compares against the source's own set.
    pub requested_features: Vec<String>,
}

/// Hover description for `Pack.getChatLink` (`Pack.java:84-86`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPackDescription {
    /// `dataPack.vanilla.name` / `dataPack.vanilla.description`
    /// (`ServerPacksSource.java:25-34`).
    Vanilla,
    Literal(bbb_protocol::JavaString),
}

/// `MinecraftServer` TimeProfiler as `/debug start` / `stop` read it
/// (`MinecraftServer.java:2022-2037`, `:2234-2280`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommandDebugProfilerView {
    pub running: bool,
    pub sample: Option<CommandDebugProfilerSample>,
}

/// The live TimeProfiler sample `stopTimeProfiler` reads when it is not
/// `EmptyProfileResults.EMPTY` (`MinecraftServer.java:2030-2033`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandDebugProfilerSample {
    pub start_nanos: i64,
    pub start_tick: i32,
    pub now_nanos: i64,
    pub now_tick: i32,
}

/// Where the source of a command stands: `CommandSourceStack`'s position,
/// rotation, level and anchor, which a function called from it inherits
/// (`FunctionCommand.modifySenderForExecution`, FunctionCommand.java:106-108).
#[derive(Debug, Clone, PartialEq)]
pub struct CommandOrigin {
    pub position: [f64; 3],
    pub rotation: [f32; 2],
    pub level: Java26LevelKey,
    /// `CommandSourceStack.anchor` (CommandSourceStack.java:64), which every
    /// other builder copies (`:145`, `:166`, `:326`) and only `withAnchor`
    /// replaces (`:286-305`).
    pub anchor: EntityAnchor,
    /// `getEyeHeight()` for `CommandSourceStack.entity` (`:62`), which travels
    /// with the anchor because `Anchor.apply` reads both together
    /// (EntityAnchorArgument.java:85-88). `None` is a source with no entity.
    pub eye_height: Option<f32>,
}

impl CommandOrigin {
    /// `CommandSourceStack.withLevel` (`CommandSourceStack.java:307-329`): a
    /// cross-level switch scales X/Z by `DimensionType.getTeleportationScale`
    /// (`DimensionType.java:109-113`).
    #[must_use]
    pub(crate) fn with_level(mut self, world: &ServerWorld, level: &Java26LevelKey) -> Self {
        if *level == self.level {
            return self;
        }
        let scale = world
            .command_dimension_type(&self.level)
            .1
            .shape
            .coordinate_scale
            / world.command_dimension_type(level).1.shape.coordinate_scale;
        self.position[0] *= scale;
        self.position[2] *= scale;
        self.level = level.clone();
        self
    }

    /// `CommandSourceStack.withPosition` (`CommandSourceStack.java:172-191`).
    #[must_use]
    pub(crate) fn with_position(mut self, position: [f64; 3]) -> Self {
        self.position = position;
        self
    }

    /// `CommandSourceStack.withRotation` (`CommandSourceStack.java:193-212`).
    #[must_use]
    pub(crate) fn with_rotation(mut self, rotation: [f32; 2]) -> Self {
        self.rotation = rotation;
        self
    }

    /// `CommandSourceStack.withAnchor` (`CommandSourceStack.java:286-305`).
    #[must_use]
    pub(crate) fn with_anchor(mut self, anchor: EntityAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// `EntityAnchorArgument.Anchor.apply(CommandSourceStack)`
    /// (`EntityAnchorArgument.java:85-88`): `EYES` rises by the entity's
    /// `getEyeHeight` (`:62`); a source with no entity stays put.
    #[must_use]
    pub(crate) fn anchored_position(&self) -> [f64; 3] {
        let [x, y, z] = self.position;
        match (self.anchor, self.eye_height) {
            (EntityAnchor::Eyes, Some(eye_height)) => [x, y + f64::from(eye_height), z],
            _ => [x, y, z],
        }
    }

    /// `CommandSourceStack.facing(Vec3)` (`CommandSourceStack.java:336-345`).
    #[must_use]
    pub(crate) fn facing(self, target: [f64; 3]) -> Self {
        let [from_x, from_y, from_z] = self.anchored_position();
        let xd = target[0] - from_x;
        let yd = target[1] - from_y;
        let zd = target[2] - from_z;
        let sd = xd.hypot(zd);
        let x_rot =
            crate::world::wrap_degrees((-(yd.atan2(sd) * 180.0 / std::f64::consts::PI)) as f32);
        let y_rot =
            crate::world::wrap_degrees((zd.atan2(xd) * 180.0 / std::f64::consts::PI - 90.0) as f32);
        self.with_rotation([y_rot, x_rot])
    }

    /// `Level.getMinY()` and `Level.getMaxY()` of the source level.
    pub(crate) fn y_bounds(&self, world: &ServerWorld) -> (i32, i32) {
        let shape = world.command_dimension_type(&self.level).1.shape;
        (shape.min_y, shape.min_y + shape.height - 1)
    }
}

/// `CustomBossEvent` state `/bossbar` reads (CustomBossEvent.java:20-190).
#[derive(Debug, Clone, PartialEq)]
pub struct CommandBossBarState {
    pub id: String,
    pub name: RichTextComponent,
    pub value: i32,
    pub max: i32,
    pub color: BossBarColor,
    pub overlay: BossBarOverlay,
    pub visible: bool,
    pub players: Vec<uuid::Uuid>,
    pub online_players: Vec<NameAndId>,
}

/// One living transmitter as `/waypoint` sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandWaypointState {
    pub target: CommandAttributeTarget,
    pub display_name: NbtSummaryValue,
    pub position: BlockPos,
    pub icon: WaypointIcon,
}

/// One `TimerQueue.Event` as `/schedule` reads it (TimerQueue.java:95).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandScheduledEvent {
    pub id: bbb_protocol::JavaString,
    pub trigger_time: i64,
}

pub use bbb_server_persistence::scheduled_events_schema::Java26ScheduledCallback as CommandScheduledCallback;

/// One `ServerClockManager` writer `modifyClock` runs
/// (`ServerClockManager.java:80-110`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandClockChange {
    /// `setTotalTicks` (`:80-85`), which `moveToTimeMarker` (`:87-98`) reaches
    /// with the resolved marker time; both drop the fractional tick.
    SetTotalTicks(i64),
    /// `addTicks` (`:100-102`), which keeps it and floors the sum at zero.
    AddTicks(i32),
    /// `setPaused` (`:104-106`).
    SetPaused(bool),
    /// `setRate` (`:108-110`).
    SetRate(f32),
}

pub use bbb_server_types::block_state_parse::BlockStateInput as CommandBlockInput;

/// One position a block command resolved, and the input it will be written
/// with.
///
/// `/setblock` produces one of these and `/fill` one per position its loop
/// admitted, in `BlockPos.betweenClosed`'s own order (`Cursor3D.advance`,
/// Cursor3D.java:30-40: x fastest, then y, then z), because the writes are
/// applied in that order and a shape update sees what the writes before it
/// left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandBlockPlacement {
    pub pos: BlockPos,
    /// The `BlockInput` this position is written with, or `None` for a
    /// position the command touched without placing anything.
    ///
    /// Vanilla's own nullable: `mode.filter.filter` (FillCommand.java:176)
    /// answers null for the inside of an `outline`, and
    /// `SetBlockCommand.setBlock`'s `placeNeeded`
    /// (SetBlockCommand.java:116-122) is false when a
    /// `destroy` had nothing to put back. Both still ran the affector, which
    /// is why the position is here at all rather than left out.
    pub block: Option<CommandBlockInput>,
}

/// Which positions the `updateNeighboursOnBlockSet` pass covers.
///
/// The two commands disagree: `SetBlockCommand` (SetBlockCommand.java:129-131)
/// runs the pass unless its write refused -- a `destroy` that had nothing to
/// place still reaches that line -- while `FillCommand`
/// (FillCommand.java:186-188) runs it after the loop over only the positions
/// whose write returned true.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBlockNeighbourPass {
    /// `SetBlockCommand.java:129-131`.
    EveryPosition,
    /// `FillCommand.java:186-188`.
    PositionsThatChanged,
    /// The `strict` literal, which is `UPDATE_SKIP_ALL_SIDEEFFECTS` on the
    /// write and no pass at all after it (SetBlockCommand.java:125-131,
    /// FillCommand.java:181-197).
    None,
}

/// Which of `WeatherCommand`'s three literals was typed
/// (`WeatherCommand.java:20-40`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandWeatherKind {
    Clear,
    Rain,
    Thunder,
}

impl CommandWeatherKind {
    /// The key the matching body reports (`WeatherCommand.java:50`, `:56`,
    /// `:62`).
    pub(super) fn feedback_key(self) -> &'static str {
        match self {
            Self::Clear => "commands.weather.set.clear",
            Self::Rain => "commands.weather.set.rain",
            Self::Thunder => "commands.weather.set.thunder",
        }
    }
}

/// `AdvancementCommands.Action` (`AdvancementCommands.java:369-442`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAdvancementAction {
    Grant,
    Revoke,
}

impl CommandAdvancementAction {
    /// `Action.getKey` (`AdvancementCommands.java:440-442`).
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Grant => "commands.advancement.grant",
            Self::Revoke => "commands.advancement.revoke",
        }
    }
}

/// The execution instance retained by a command block's result callback.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandBlockResultTarget {
    Block {
        level: Java26LevelKey,
        instance: Box<crate::block_entity::BlockEntity>,
    },
    Minecart {
        level: Java26LevelKey,
        entity_id: i32,
        entity_uuid: uuid::Uuid,
    },
}

/// A mutable `DataAccessor` after command argument resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandDataTarget {
    Block {
        level: Java26LevelKey,
        pos: BlockPos,
    },
    StoredEntity {
        level: Java26LevelKey,
        entity_id: i32,
    },
    Storage {
        id: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandBossBarMutation {
    Create {
        id: String,
        name: RichTextComponent,
        source_level: Java26LevelKey,
    },
    Remove {
        id: String,
    },
    SetName {
        id: String,
        name: RichTextComponent,
    },
    SetColor {
        id: String,
        color: BossBarColor,
    },
    SetOverlay {
        id: String,
        overlay: BossBarOverlay,
    },
    SetValue {
        id: String,
        value: i32,
    },
    SetMax {
        id: String,
        max: i32,
    },
    SetVisible {
        id: String,
        visible: bool,
    },
    SetPlayers {
        id: String,
        players: Vec<uuid::Uuid>,
    },
}

/// A camera remembers an entity identity, not its display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandCameraTarget {
    Player {
        uuid: uuid::Uuid,
    },
    Stored {
        dimension: Java26LevelKey,
        entity_id: i32,
        uuid: uuid::Uuid,
    },
}

/// One `Entity.teleportTo` a `/teleport` makes (TeleportCommand.java:252):
/// the axes `relatives_mask` marks carry the difference from where the
/// victim stands, and both rotations are already wrapped.
#[derive(Debug, Clone, PartialEq)]
pub struct TeleportMove {
    pub victim: CommandAttributeTarget,
    pub level: Java26LevelKey,
    pub position: [f64; 3],
    pub rotation: [f32; 2],
    pub relatives_mask: i32,
    pub look_at: Option<CommandLookAt>,
    /// `TeleportCommand.performTeleport`'s clean-up after the move
    /// (`:257-264`), which `/spreadplayers` does not run.
    pub teleport_command_tail: bool,
}

/// An entity resolved for a command to read or mutate.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandAttributeTarget {
    Player(CommandPlayerTarget),
    Stored {
        dimension: Java26LevelKey,
        entity_id: i32,
    },
}

/// The facing request `/teleport` and `/rotate` perform (`LookAt.java:13-29`).
#[derive(Debug, Clone, PartialEq)]
pub enum CommandLookAt {
    Entity {
        target: CommandAttributeTarget,
        anchor: EntityAnchor,
        from_anchor: EntityAnchor,
    },
    Position {
        position: [f64; 3],
        from_anchor: EntityAnchor,
    },
}

/// `ExperienceCommand.Type` (ExperienceCommand.java:175-202): which of the two
/// experience counters a `/experience` arm reads or writes, and the word the
/// translation keys spell it with (`:176`, `:184`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandExperienceKind {
    Points,
    Levels,
}

impl CommandExperienceKind {
    /// `Type.name` (`:191`), the segment `"commands.experience." + type.name`
    /// builds the keys from (`:127`, `:140`, `:166`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Points => "points",
            Self::Levels => "levels",
        }
    }
}

/// One player `PlaySoundCommand.playSound` sends to, and what the packet it
/// gets says (PlaySoundCommand.java:173-188).
#[derive(Debug, Clone, PartialEq)]
pub struct CommandSoundRecipient {
    pub player: CommandPlayerTarget,
    /// `localPosition`: the position the command was given, or -- for a
    /// listener past the range that asked for a `minVolume` -- a point two
    /// blocks from the listener towards it (`:181-183`).
    pub position: [f64; 3],
    /// `localVolume`: the volume the command was given, or the `minVolume`
    /// for that same far listener (`:184`).
    pub volume: f32,
}
