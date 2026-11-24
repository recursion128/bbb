//! Per-session online and restored player state owned by [`super::ServerWorld`].

use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

use super::debug_subscriptions::PlayerDebugSubscriptions;
use super::simulation_level::CLIENT_LOADED_TIMEOUT_TICKS;
use super::{
    commands, game_type_may_build, player_drop, player_environment, player_movement,
    player_shoulders, player_tick::ChunkTrackingView, score_criteria, using_item, LevelId,
    LAST_SEEN_MESSAGES_MAX_LENGTH,
};
use crate::block_destroy::DestroyProgress;
use crate::chunk_sender::PlayerChunkSender;
use crate::connection::ConnectionId;
use crate::inventory::{minimum_attack_charge, PlayerInventoryMenu};
use crate::item_cooldowns::ItemCooldowns;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_SPEED_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    BlockPos, ClientInformation, EditBook, GameProfile, ItemStackSummary, PlayerAbilities,
    PlayerInput, PlayerRotation, RecipeBookChangeSettingsCommand, Vec3d,
};
use bbb_protocol::recipe_wire_types::VANILLA_RECIPE_BOOK_TYPES;
use bbb_server_datapack::damage::PlayerDamageState;
use bbb_server_types::experience::PlayerExperience as ExperienceState;
use bbb_server_types::food::FoodData;
use bbb_server_types::save::Java26LevelKey;
use bbb_server_types::stats::ServerStatsCounter;
use bbb_server_types::Result;

/// What one saved `playerdata` document says about a player who is not online,
/// as far as this world projects it back at the next join.
///
#[derive(Debug, Clone, PartialEq)]
pub(in crate::world) struct RestoredPlayer {
    pub(in crate::world) shoulders: player_shoulders::PlayerShoulders,
    pub(in crate::world) portal_cooldown: i32,
    pub(in crate::world) sleep_timer: i16,
    pub(in crate::world) sleeping_pos: Option<BlockPos>,
    pub(in crate::world) last_hurt_by: bbb_server_persistence::Java26PlayerLastHurtBy,
    pub(in crate::world) spawn_extra_particles_on_fall: bool,
    pub(in crate::world) seen_credits: bool,
    pub(in crate::world) fall_impulse: crate::entity::living::FallImpulseContext,
    pub(in crate::world) has_visual_fire: bool,
    pub(in crate::world) silent: bool,
    pub(in crate::world) glowing: bool,
    pub(in crate::world) custom_name: Option<bbb_protocol::NbtSummaryValue>,
    pub(in crate::world) custom_data: Vec<bbb_protocol::NbtSummaryEntry>,
    pub(in crate::world) fall_distance: f64,
    pub(in crate::world) remaining_fire_ticks: i32,
    pub(in crate::world) air_supply: i32,
    pub(in crate::world) ticks_frozen: i32,
    pub(in crate::world) dimension: Option<Java26LevelKey>,
    pub(in crate::world) position: Vec3d,
    pub(in crate::world) rotation: PlayerRotation,
    pub(in crate::world) on_ground: bool,
    pub(in crate::world) game_type: Option<GameType>,
    pub(in crate::world) previous_game_type: Option<GameType>,
    pub(in crate::world) entered_nether_position: Option<Vec3d>,
    pub(in crate::world) raid_omen_position: Option<BlockPos>,
    pub(in crate::world) damage: PlayerDamageState,
    pub(in crate::world) attributes: Vec<crate::entity::PackedAttribute>,
    pub(in crate::world) locator_bar_icon: bbb_protocol::packets::WaypointIcon,
    /// The `active_effects` list it reads right after (`:783-789`), whose
    /// attribute modifiers are already in `attributes` rather than re-added
    /// here.
    pub(in crate::world) effects: crate::entity::living::mob_effects::ActiveMobEffects,
    /// `Entity.invulnerable` as the `Invulnerable` byte carries it.
    pub(in crate::world) entity_invulnerable: bool,
    pub(in crate::world) food: FoodData,
    pub(in crate::world) experience: ExperienceState,
    pub(in crate::world) tags: Vec<bbb_protocol::JavaString>,
    pub(in crate::world) abilities: PlayerAbilities,
    pub(in crate::world) inventory: PlayerInventoryMenu,
    pub(in crate::world) fall_flying: bool,
    pub(in crate::world) recipe_book_settings:
        [RecipeBookTypeSettings; VANILLA_RECIPE_BOOK_TYPES.len()],
    pub(in crate::world) root_vehicle: Option<bbb_server_persistence::Java26RootVehicle>,
    pub(in crate::world) ender_pearls:
        Vec<bbb_server_persistence::player_schema::ender_pearls::Java26PlayerEnderPearl>,
    pub(in crate::world) respawn: Option<PlayerRespawnConfig>,
    pub(in crate::world) last_death_location: Option<bbb_protocol::packets::GlobalPos>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerRespawnConfig {
    pub(crate) dimension: Java26LevelKey,
    pub(crate) pos: BlockPos,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) forced: bool,
}

/// The connection-scoped half of an online player, kept outside the level
/// stores so chunk delivery can be read while a level is borrowed.
#[derive(Clone)]
pub(in crate::world) struct PlayerSession {
    /// `ServerGamePacketListenerImpl.chunkSender`: chunk delivery is paced per
    /// player by what that player's client acknowledges.
    pub(in crate::world) chunk_sender: PlayerChunkSender,
    /// `ServerPlayer.chunkTrackingView` (`ChunkMap.setChunkTrackingView`), the
    /// square of columns this session is allowed to hold. `None` is
    /// `ChunkTrackingView.EMPTY`, which is what a player starts at before
    /// `updatePlayerStatus` gives them one.
    pub(in crate::world) chunk_tracking_view: Option<ChunkTrackingView>,
}

/// Cold, rarely accessed per-session fields boxed out of [`PlayerState`] to keep
/// per-tick player iteration compact.
#[derive(Clone)]
pub(in crate::world) struct PlayerCold {
    pub(in crate::world) shoulders: player_shoulders::PlayerShoulders,
    /// `chatSpamThrottler` (ServerGamePacketListenerImpl.java:243).
    pub(in crate::world) chat_spam: i32,
    /// `nextChatIndex` (ServerGamePacketListenerImpl.java:271).
    pub(in crate::world) next_chat_index: i32,
    pub(in crate::world) portal_cooldown: i32,
    pub(in crate::world) portal_process: Option<crate::entity::portal::PortalProcessor>,
    pub(in crate::world) sleep_timer: i16,
    pub(in crate::world) sleeping_pos: Option<BlockPos>,
    pub(in crate::world) sleeping_pos_dirty: bool,
    pub(in crate::world) arrows: crate::entity::living::StuckCount,
    pub(in crate::world) arrow_count_dirty: bool,
    pub(in crate::world) stingers: crate::entity::living::StuckCount,
    pub(in crate::world) stinger_count_dirty: bool,
    pub(in crate::world) spawn_extra_particles_on_fall: bool,
    /// `ServerPlayer.seenCredits`: the End exit portal shows the poem first.
    pub(in crate::world) seen_credits: bool,
    /// `ServerPlayer.wonGame`: the next `PERFORM_RESPAWN` leaves the credits.
    pub(in crate::world) won_game: bool,
    pub(in crate::world) fall_impulse: crate::entity::living::FallImpulseContext,
    pub(in crate::world) impulse_explosion_cause: Option<(
        crate::explosion::ExplosionSourceEntity,
        Option<Arc<bbb_protocol::NbtSummaryValue>>,
    )>,
    /// `Entity.hasGlowingTag`, only ever set from the `Glowing` save field.
    pub(in crate::world) glowing_tag: bool,
    pub(in crate::world) custom_name: Option<bbb_protocol::NbtSummaryValue>,
    pub(in crate::world) custom_data: Vec<bbb_protocol::NbtSummaryEntry>,
    /// `ServerGamePacketListenerImpl.dropSpamThrottler` (`:244`), which guards
    /// the creative-mode drop path only (`:2016-2022`).
    pub(in crate::world) drop_spam_throttler: player_drop::DropSpamThrottler,
    /// `ServerPlayer.enteredNetherPosition`, saved as `entered_nether_pos`
    /// (`ServerPlayer.java:388`, `:411`).
    pub(in crate::world) entered_nether_position: Option<Vec3d>,
    /// `ServerPlayer.raidOmenPosition`, saved as `raid_omen_position`
    /// (`ServerPlayer.java:398`, `:417`).
    pub(in crate::world) raid_omen_position: Option<BlockPos>,
    /// `Player.lastDeathLocation` (`Player.java:170`).
    pub(in crate::world) last_death_location: Option<bbb_protocol::packets::GlobalPos>,
    /// `LivingEntity.skipDropExperience` (`LivingEntity.java:263`), which a
    /// sculk catalyst sets when it takes the death's experience.
    pub(in crate::world) skip_drop_experience: bool,
    pub(in crate::world) combat: bbb_server_datapack::damage::CombatTracker,
    pub(in crate::world) levitation_start_position: Option<Vec3d>,
    pub(in crate::world) entered_lava_on_vehicle_position: Option<Vec3d>,
    pub(in crate::world) levitation_start_tick: i32,
    pub(in crate::world) fall_fly_ticks: i32,
    pub(in crate::world) respawn: Option<PlayerRespawnConfig>,
    pub(in crate::world) pending_block_ack: Option<i32>,
    pub(in crate::world) pending_keep_alive: Option<i64>,
    pub(in crate::world) keep_alive_sent_millis: i64,
    /// `ServerCommonPacketListenerImpl.latency`, the smoothed keep-alive
    /// round trip the player list shows.
    pub(in crate::world) latency: i32,
    pub(in crate::world) last_action_millis: i64,
    /// `ServerPlayer.getIpAddress`, which is what
    /// `PlayerList.getPlayersWithAddress` matches an address ban against.
    pub(in crate::world) remote_address: Option<String>,
    pub(in crate::world) client_information: ClientInformation,
    pub(in crate::world) main_hand_dirty: bool,
    pub(in crate::world) model_customization_dirty: bool,
    /// `ServerPlayerGameMode.previousGameModeForPlayer` (`:40`).
    pub(in crate::world) previous_game_type: Option<GameType>,
    /// `ServerPlayer.lastSectionPos` (`ChunkMap.updatePlayerPos`), the 3D
    /// Filed SectionPos; chunk-view updates do not refile player tickets.
    pub(in crate::world) last_section_pos: (i32, i32, i32),
    pub(in crate::world) filed_ticket_ignored: bool,
    ///
    /// The validator starts with `LAST_SEEN_MESSAGES_MAX_LENGTH` empty entries
    /// and grows one per player chat message the session is *sent*
    /// (`addPending`, :20-25). This server broadcasts chat as a system message
    /// -- the same consequence the signed-chat removal declared -- so nothing
    /// ever pends here and the window never has slack to give away.
    pub(in crate::world) last_seen_tracked: usize,
    pub(in crate::world) requested_debug_subscriptions: PlayerDebugSubscriptions,
    pub(in crate::world) last_vehicle: Option<i32>,
    pub(in crate::world) vehicle_first_good: Vec3d,
    pub(in crate::world) vehicle_last_good: Vec3d,
    pub(in crate::world) client_vehicle_is_floating: bool,
    pub(in crate::world) above_ground_vehicle_tick_count: i32,
    /// `ServerPlayer.getRecipeBook().getBookSettings()`
    /// (`RecipeBookSettings`, :35-38): one open/filter pair per book type, in
    /// the book type's own ordinal order, all clear on a fresh book
    /// (`TypeSettings.DEFAULT`, :115).
    pub(in crate::world) recipe_book_settings:
        [RecipeBookTypeSettings; VANILLA_RECIPE_BOOK_TYPES.len()],
    pub(in crate::world) advancements: bbb_server_datapack::player_advancements::PlayerAdvancements,
    /// A wire projection that refused the last dirty flush; the session sends
    /// nothing for that update.
    pub(in crate::world) advancement_update_refusal: Option<String>,
    /// The book edits this session has asked for and the server has not run
    /// yet. See [`ServerWorld::run_server_tasks`].
    pub(in crate::world) pending_book_edits: Vec<PendingBookEdit>,
    pub(in crate::world) destroy: DestroyProgress,
    pub(in crate::world) locator_bar_icon: bbb_protocol::packets::WaypointIcon,
    pub(in crate::world) last_hurt_by_player: Option<Uuid>,
    pub(in crate::world) last_hurt_by_player_memory_time: i32,
    pub(in crate::world) last_hurt_by_mob: Option<Uuid>,
    pub(in crate::world) last_hurt_by_mob_timestamp: i32,
    pub(in crate::world) last_hurt_mob: Option<(i32, Uuid)>,
    pub(in crate::world) last_hurt_mob_timestamp: i32,
    pub(in crate::world) stats: ServerStatsCounter,
    pub(in crate::world) warden_tracker: crate::entity::WardenSpawnTracker,
    /// `Player.lastLevelUpTime` (`Player.java:165`).
    pub(in crate::world) last_level_up_time: i32,
    pub(in crate::world) recorded_criteria: score_criteria::RecordedCriteria,
    ///
    /// Vanilla's is a `HashSet`; this keeps the order tags were added in, which
    /// is the divergence [`crate::commands::CommandPlayer::tags`] spells
    /// out.
    pub(in crate::world) tags: Vec<bbb_protocol::JavaString>,
    ///
    /// Stored targets retain level, id and UUID so dimension replacements do
    /// not inherit the old object's observers.
    pub(in crate::world) camera: Option<commands::CommandCameraTarget>,
    pub(in crate::world) cooldowns: ItemCooldowns,
    pub(in crate::world) last_equipment_items: BTreeMap<
        bbb_protocol::equipment_slots::EquipmentSlot,
        bbb_server_types::item_stack::RuntimeItemStack,
    >,
    pub(in crate::world) location_effects: crate::entity::location_effects::LocationEffectsState,
}

/// One online player: its world body (filed in its level's [`EntityStore`]
/// under the body's entity id) and the session state riding along with it.
#[derive(Clone)]
pub(crate) struct PlayerState {
    pub(in crate::world) connection_id: ConnectionId,
    pub(in crate::world) body: crate::entity::Entity,
    pub(in crate::world) has_visual_fire: bool,
    pub(in crate::world) silent: bool,
    pub(in crate::world) silent_dirty: bool,
    pub(in crate::world) environment_first_tick: bool,
    pub(in crate::world) was_touching_water: bool,
    pub(in crate::world) block_contact: player_environment::contact::PlayerBlockContact,
    pub(in crate::world) remaining_fire_ticks: i32,
    pub(in crate::world) air_supply: i32,
    pub(in crate::world) air_supply_dirty: bool,
    pub(in crate::world) ticks_frozen: i32,
    pub(in crate::world) ticks_frozen_dirty: bool,
    pub(in crate::world) fluid_interaction: crate::entity::FluidInteraction,
    pub(in crate::world) body_tick_enabled: bool,
    pub(in crate::world) profile: GameProfile,
    pub(in crate::world) dimension: LevelId,
    pub(in crate::world) avatar_removed: bool,
    pub(in crate::world) is_changing_dimension: bool,
    /// `ServerGamePacketListenerImpl.waitingForRespawn` (:275), which
    /// `markClientUnloadedAfterDeath` (:2212-2214) raises when the player dies
    /// and `restartClientLoadTimerAfterRespawn` (:2216-2219) lowers when the
    /// respawn is carried out.
    pub(in crate::world) waiting_for_respawn: bool,
    /// `ServerGamePacketListenerImpl.clientLoadedTimeoutTimer` (:276).
    ///
    /// The listener's own constructor opens it (`:280`) and so does every
    /// respawn (`:2218`), both at [`CLIENT_LOADED_TIMEOUT_TICKS`];
    /// `handleAcceptPlayerLoad` (`:549-553`) closes it the moment the client
    /// says it is ready, and `tickClientLoadTimeout` (`:2202-2206`) counts it
    /// down otherwise. So a session that never answers is treated as loaded
    /// once the timer runs out rather than being stuck for good.
    pub(in crate::world) client_loaded_timeout_timer: i32,
    pub(in crate::world) awaiting_teleport: Option<i32>,
    /// Server tick of the last teleport, or of the last move packet while none
    /// was outstanding (`awaitingTeleportTime`).
    pub(in crate::world) awaiting_teleport_tick: u64,
    /// `ServerPlayerGameMode.gameModeForPlayer` (`ServerPlayer.gameMode`), the
    /// mode this session runs at.
    pub(in crate::world) game_type: GameType,
    /// `ServerPlayer.getAbilities`: the server owns every ability bit; the
    /// client may only ask for the flying toggle
    /// (`ServerGamePacketListenerImpl.handlePlayerAbilities`).
    pub(in crate::world) abilities: PlayerAbilities,
    /// `Entity.DATA_SHARED_FLAGS_ID`: the one synched byte this server owns so
    /// far. Vanilla stores it inside `SynchedEntityData`, which is why the
    /// dirty bit lives beside it rather than being derived from a last-sent
    /// snapshot -- `stopFallFlying` deliberately dirties the item without
    /// changing its value.
    pub(in crate::world) shared_flags: i8,
    pub(in crate::world) shared_flags_dirty: bool,
    pub(in crate::world) pose_id: i32,
    pub(in crate::world) pose_dirty: bool,
    pub(in crate::world) boarding_cooldown: i32,
    pub(in crate::world) tick_count: i32,
    /// `ServerGamePacketListenerImpl.firstGoodX/Y/Z` (`:245-247`): the
    /// position the current tick's move packets are measured against, which
    /// `resetPosition` (`:372-377`) sets from the session's own position.
    pub(in crate::world) first_good: Vec3d,
    /// `ServerGamePacketListenerImpl.lastGoodX/Y/Z` (`:248-250`), which an
    /// accepted move (`:1176-1178`) and an acknowledged teleport (`:541-543`)
    /// both move forward.
    pub(in crate::world) last_good: Vec3d,
    /// `ServerGamePacketListenerImpl.receivedMovePacketCount` (`:265`).
    pub(in crate::world) received_move_packets: i32,
    /// `ServerGamePacketListenerImpl.knownMovePacketCount` (`:266`), synced to
    /// the received count once per tick (`:317`).
    pub(in crate::world) known_move_packets: i32,
    /// The walking probe's own listener state (`:261-262`, `:1120`).
    pub(in crate::world) movement: player_movement::MovementProbeState,
    /// `ServerPlayer.setLastClientInput` (`handlePlayerInput`, :417-424). The
    /// server keeps the latest declared input even before the client reports
    /// itself loaded; only the derived state is gated.
    pub(in crate::world) last_client_input: PlayerInput,
    pub(in crate::world) inventory: PlayerInventoryMenu,
    pub(in crate::world) attack_strength_ticker: i32,
    pub(in crate::world) speed: f32,
    pub(in crate::world) item_swap_ticker: i32,
    pub(in crate::world) last_item_in_main_hand: bbb_server_types::item_stack::RuntimeItemStack,
    /// The `LivingEntity` damage fields (`bbb_server_datapack::damage`), which `/damage`
    /// writes and `tick_players` counts down.
    pub(in crate::world) damage: PlayerDamageState,
    ///
    /// Dirty syncable instances are projected by the player's
    /// [`ServerEntity`]; the update set independently drives server-side
    /// clamps and waypoint range changes.
    pub(in crate::world) attributes: crate::entity::AttributeMap,
    pub(in crate::world) effects: crate::entity::living::mob_effects::ActiveMobEffects,
    pub(in crate::world) effect_particles:
        crate::entity::living::mob_effects::effect_particles::SynchedEffectParticles,
    pub(in crate::world) entity_invulnerable: bool,
    pub(in crate::world) food: FoodData,
    pub(in crate::world) last_sent_health: f32,
    pub(in crate::world) last_sent_food: i32,
    pub(in crate::world) last_food_saturation_zero: bool,
    pub(in crate::world) experience: ExperienceState,
    pub(in crate::world) last_sent_exp: i32,
    /// The `DATA_SCORE_ID` value this session's client holds.
    pub(in crate::world) last_sent_score: i32,
    pub(in crate::world) take_xp_delay: i32,
    pub(in crate::world) swinging: bool,
    pub(in crate::world) swing_time: i32,
    /// `LivingEntity.swingingArm`, `MAIN_HAND` until the first swing.
    pub(in crate::world) swinging_arm: bbb_protocol::packets::InteractionHand,
    pub(in crate::world) known_movement: Vec3d,
    pub(in crate::world) needs_sync: bool,
    pub(in crate::world) hurt_marked: bool,
    pub(in crate::world) received_movement_this_tick: bool,
    pub(in crate::world) using_item: using_item::UsingItemState,
    pub(in crate::world) cold: Box<PlayerCold>,
}

/// `ClientboundAnimatePacket.SWING_MAIN_HAND` and `SWING_OFF_HAND`
/// (ClientboundAnimatePacket.java:13, `:15`), which `LivingEntity.swing` picks
/// between on the hand it was given (LivingEntity.java:1999).
pub(in crate::world) const ANIMATE_SWING_MAIN_HAND: u8 = 0;
pub(in crate::world) const ANIMATE_SWING_OFF_HAND: u8 = 3;

/// `Entity.MAX_ENTITY_TAG_COUNT` (Entity.java:193), the ceiling `Entity.addTag`
/// refuses above (`:391-393`).
pub(in crate::world) const MAX_ENTITY_TAG_COUNT: usize = 1024;

/// `ServerPlayer.lastSentHealth`'s initial value (ServerPlayer.java:238), which
/// no health can equal so that the first `doTick` always announces one.
pub(in crate::world) const UNSENT_HEALTH: f32 = -1.0E8;

/// `ServerPlayer.lastSentExp`'s initial value (ServerPlayer.java:241), which no
/// total experience can equal so that the first `doTick` always announces one.
pub(in crate::world) const UNSENT_EXPERIENCE: i32 = -99_999_999;

/// One accepted `handleEditBook` request waiting on the server's task queue.
#[derive(Debug, Clone)]
pub(in crate::world) struct PendingBookEdit {
    pub(in crate::world) slot: usize,
    pub(in crate::world) packet: EditBook,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RecipeBookTypeSettings {
    pub(crate) open: bool,
    pub(crate) filtering: bool,
}

/// The eight `recipeBook` flags a playerdata document carries, out of the four
/// book types' open/filter pairs.
///
pub(in crate::world) fn recipe_book_save_flags(
    settings: &[RecipeBookTypeSettings; VANILLA_RECIPE_BOOK_TYPES.len()],
) -> [bool; RECIPE_BOOK_SAVE_FLAGS] {
    let mut flags = [false; RECIPE_BOOK_SAVE_FLAGS];
    for (index, pair) in settings.iter().enumerate() {
        flags[index * 2] = pair.open;
        flags[index * 2 + 1] = pair.filtering;
    }
    flags
}

/// One open flag and one filter flag per book type, which is the width
/// `Java26CreatedPlayer::recipe_book_settings` is declared at.
const RECIPE_BOOK_SAVE_FLAGS: usize = 8;
const _: () = assert!(
    RECIPE_BOOK_SAVE_FLAGS == VANILLA_RECIPE_BOOK_TYPES.len() * 2,
    "the playerdata recipe-book flags are one open/filter pair per book type",
);

impl PlayerState {
    pub(crate) fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }

    pub(crate) fn body(&self) -> &crate::entity::Entity {
        &self.body
    }

    pub(crate) fn is_alive(&self) -> bool {
        !self.avatar_removed && !self.damage.is_dead_or_dying()
    }

    pub(crate) fn body_mut(&mut self) -> &mut crate::entity::Entity {
        &mut self.body
    }

    pub(in crate::world) fn entity_id(&self) -> i32 {
        self.body.id()
    }

    pub(in crate::world) fn position(&self) -> Vec3d {
        self.body.position()
    }

    pub(in crate::world) fn set_position(&mut self, position: Vec3d) {
        self.body.set_pos(position);
    }

    pub(in crate::world) fn rotation(&self) -> PlayerRotation {
        PlayerRotation {
            y_rot: self.body.y_rot(),
            x_rot: self.body.x_rot(),
        }
    }

    pub(in crate::world) fn set_rotation(&mut self, rotation: PlayerRotation) {
        self.body.set_rot(rotation.y_rot, rotation.x_rot);
    }

    pub(in crate::world) fn set_y_rot(&mut self, y_rot: f32) {
        self.body.set_rot(y_rot, self.body.x_rot());
    }

    pub(in crate::world) fn set_x_rot(&mut self, x_rot: f32) {
        self.body.set_rot(self.body.y_rot(), x_rot);
    }

    pub(in crate::world) fn on_ground(&self) -> bool {
        self.body.on_ground()
    }

    pub(in crate::world) fn set_on_ground(&mut self, on_ground: bool) {
        self.body.set_on_ground(on_ground);
        // Entity.java:680-700: leaving the ground releases the previous supporting block.
        if !on_ground {
            self.block_contact.supporting = None;
            self.block_contact.on_ground_no_blocks = false;
        }
    }

    pub(in crate::world) fn delta_movement(&self) -> Vec3d {
        self.body.delta_movement()
    }

    pub(in crate::world) fn set_delta_movement(&mut self, delta: Vec3d) {
        self.body.set_delta_movement(delta);
    }

    /// `ServerGamePacketListenerImpl.hasClientLoaded` (:2198-2200), the one
    /// answer every handler that waits for the client gates on.
    pub(in crate::world) fn client_loaded(&self) -> bool {
        !self.waiting_for_respawn && self.client_loaded_timeout_timer <= 0
    }

    /// `ServerGamePacketListenerImpl.markClientLoaded` (:2208-2210).
    pub(in crate::world) fn mark_client_loaded(&mut self) {
        self.client_loaded_timeout_timer = 0;
    }

    /// `ServerGamePacketListenerImpl.markClientUnloadedAfterDeath` (:2212-2214).
    pub(in crate::world) fn mark_client_unloaded_after_death(&mut self) {
        self.waiting_for_respawn = true;
    }

    /// `ServerGamePacketListenerImpl.restartClientLoadTimerAfterRespawn`
    /// (:2216-2219).
    pub(in crate::world) fn restart_client_load_timer_after_respawn(&mut self) {
        self.waiting_for_respawn = false;
        self.client_loaded_timeout_timer = CLIENT_LOADED_TIMEOUT_TICKS;
    }

    pub(in crate::world) fn tick_client_load_timeout(&mut self) {
        if self.client_loaded_timeout_timer > 0 {
            self.client_loaded_timeout_timer -= 1;
        }
    }

    ///
    /// It is derived from the game type rather than stored beside the other
    /// abilities because [`PlayerAbilities`] deliberately does not carry the
    /// bit: `ClientboundPlayerAbilitiesPacket` never writes it. Deriving it
    /// reads the same input `updatePlayerAbilities` did, so the two cannot
    /// drift apart.
    pub(in crate::world) fn may_build(&self) -> bool {
        game_type_may_build(self.game_type)
    }

    pub(in crate::world) fn apply_last_seen_offset(&mut self, offset: i32) -> bool {
        let Ok(offset) = usize::try_from(offset) else {
            return false;
        };
        let max_offset = self.cold.last_seen_tracked - LAST_SEEN_MESSAGES_MAX_LENGTH;
        if offset > max_offset {
            return false;
        }
        self.cold.last_seen_tracked -= offset;
        true
    }

    /// `LastSeenMessagesValidator.applyUpdate` (`LastSeenMessagesValidator.java:40-76`)
    /// and `LastSeenMessages.Update.verifyChecksum` (`LastSeenMessages.java:81-83`).
    pub(in crate::world) fn apply_last_seen_update(
        &mut self,
        update: bbb_protocol::packets::LastSeenMessagesUpdate,
    ) -> bool {
        self.apply_last_seen_offset(update.offset)
            && update.acknowledged == 0
            && (update.checksum == 0
                || update.checksum
                    == bbb_protocol::packets::LastSeenMessagesUpdate::default().checksum)
    }

    pub(in crate::world) fn set_recipe_book_setting(
        &mut self,
        command: RecipeBookChangeSettingsCommand,
    ) {
        let index = VANILLA_RECIPE_BOOK_TYPES
            .iter()
            .position(|entry| entry.book_type == command.book_type)
            .expect("the generated catalog covers every recipe book type");
        self.cold.recipe_book_settings[index] = RecipeBookTypeSettings {
            open: command.open,
            filtering: command.filtering,
        };
    }

    pub(crate) fn is_swimming(&self) -> bool {
        self.shared_flag(bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SWIMMING)
    }

    pub(crate) fn is_sprinting(&self) -> bool {
        self.shared_flag(bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING)
    }

    // Entity.java:561-568; Entity.java:879-881.
    pub(crate) fn has_moved_horizontally_recently(&self) -> bool {
        // Mob AI runs before tick_players advances the existing position baseline.
        let position = self.position();
        (position.x - self.first_good.x).hypot(position.z - self.first_good.z)
            > f64::from(1.0e-5_f32)
    }

    pub(in crate::world) fn shared_flag(&self, mask: i8) -> bool {
        self.shared_flags & mask != 0
    }

    pub(in crate::world) fn set_shared_flag(&mut self, mask: i8, value: bool) {
        let next = if value {
            self.shared_flags | mask
        } else {
            self.shared_flags & !mask
        };
        if next != self.shared_flags {
            self.shared_flags = next;
            self.shared_flags_dirty = true;
        }
    }

    pub(in crate::world) fn set_pose(&mut self, pose_id: i32) {
        if self.pose_id != pose_id {
            self.pose_id = pose_id;
            self.pose_dirty = true;
        }
    }

    pub(in crate::world) fn pose_scale(&self) -> f32 {
        self.attributes
            .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
            .unwrap_or(1.0) as f32
    }

    pub(in crate::world) fn current_item_attack_strength_delay(&self) -> Result<f32> {
        Ok((1.0 / self.attribute_value(VANILLA_ATTRIBUTE_ATTACK_SPEED_ID) * 20.0) as f32)
    }

    pub(in crate::world) fn attack_strength_scale(&self, partial: f32) -> Result<f32> {
        Ok(((self.attack_strength_ticker as f32 + partial)
            / self.current_item_attack_strength_delay()?)
        .clamp(0.0, 1.0))
    }

    pub(in crate::world) fn cannot_attack_with_item(
        &self,
        item: &ItemStackSummary,
        tolerance: i32,
    ) -> Result<bool> {
        let required_strength = minimum_attack_charge(item)?;
        let optimistic_strength = (self.attack_strength_ticker + tolerance) as f32
            / self.current_item_attack_strength_delay()?;
        Ok(required_strength > 0.0 && optimistic_strength < required_strength)
    }

    pub(in crate::world) fn on_attack(&mut self) {
        self.attack_strength_ticker = 0;
    }

    pub(in crate::world) fn reset_attack_strength_ticker(&mut self) {
        self.attack_strength_ticker = 0;
        self.item_swap_ticker = 0;
    }

    ///
    /// `attackAnim` (`:2157`) is the client-side interpolation of the same
    /// counter and is not kept here; nothing server-side reads it.
    pub(in crate::world) fn tick_swing_time(&mut self) {
        if self.swinging {
            self.swing_time += 1;
            if self.swing_time >= self.current_swing_duration() {
                self.swing_time = 0;
                self.swinging = false;
            }
        } else {
            self.swing_time = 0;
        }
    }

    pub(in crate::world) fn tick_stuck_counts(&mut self) {
        self.cold.arrow_count_dirty |= self.cold.arrows.tick();
        self.cold.stinger_count_dirty |= self.cold.stingers.tick();
    }

    pub(in crate::world) fn add_stuck_arrow(&mut self) {
        self.cold.arrows.count += 1;
        self.cold.arrow_count_dirty = true;
    }

    pub(in crate::world) fn add_stinger(&mut self) {
        self.cold.stingers.count += 1;
        self.cold.stinger_count_dirty = true;
    }

    pub(in crate::world) fn tick_attack_strength(&mut self) {
        self.attack_strength_ticker += 1;
        self.item_swap_ticker += 1;
        let main_hand = self.inventory.selected_stack().clone();
        if !self.last_item_in_main_hand.matches(&main_hand) {
            let same_item = self.last_item_in_main_hand.item_id() == main_hand.item_id();
            let main_hand = main_hand.clone();
            if !same_item {
                self.reset_attack_strength_ticker();
            }
            self.last_item_in_main_hand = main_hand;
        }
    }

    pub(in crate::world) fn stop_fall_flying(&mut self) {
        self.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, true);
        self.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, false);
    }
}

pub(in crate::world) fn players_in_connection_order<'a>(
    players: impl Iterator<Item = &'a PlayerState>,
) -> std::vec::IntoIter<&'a PlayerState> {
    let mut entries = players.collect::<Vec<_>>();
    entries.sort_unstable_by_key(|player| player.connection_id);
    entries.into_iter()
}
