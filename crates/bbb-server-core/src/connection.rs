//! The connection identity and the clientbound play packets a world publishes.

use std::sync::Arc;

use bbb_protocol::packets::GameRuleValues;
use bbb_protocol::packets::HurtAnimation;
use bbb_protocol::packets::{
    AddEntity, AwardStats, BlockChangedAck, BlockDestruction, BlockEntityData, BlockEvent,
    BlockUpdate, BossEvent, ChangeDifficulty, ChunksBiomes, ClearTitles, CommandSuggestions,
    ContainerClose, ContainerSetContent, ContainerSetData, ContainerSetSlot, DamageEvent,
    DebugSample, EntityAnimation, EntityEvent, EntityMove, EntityPositionSync, ForgetLevelChunk,
    GameEvent, LevelChunkWithLight, LevelParticles, LightUpdate, MapItemData, MountScreenOpen,
    MoveMinecartAlongTrack, MoveVehicle, OpenBook, OpenScreen, OpenSignEditor, PlaceGhostRecipe,
    PlayLogin, PlayTime, PlayerAbilities, PlayerCombatKill, PlayerExperience, PlayerHealth,
    PlayerInfoRemove, PlayerInfoUpdate, PlayerLookAt, PlayerPositionUpdate, PlayerRotationUpdate,
    PongResponse, ProjectilePower, RecipeBookAdd, RecipeBookRemove, RecipeBookSettings,
    RemoveEntities, Respawn, SectionBlocksUpdate, SelectAdvancementsTab, ServerData,
    SetActionBarText, SetBorderCenter, SetBorderLerpSize, SetBorderSize, SetBorderWarningDelay,
    SetBorderWarningDistance, SetCamera, SetChunkCacheCenter, SetChunkCacheRadius, SetCursorItem,
    SetDefaultSpawnPosition, SetEntityData, SetEntityMotion, SetHeldSlot, SetPlayerInventory,
    SetSimulationDistance, SetSubtitleText, SetTitleText, SetTitlesAnimation, SoundEntityEvent,
    SoundEvent, StopSound, SystemChat, TagQuery, TakeItemEntity, TeleportEntity, TickingState,
    TickingStep, TrackedWaypointPacket, Transfer, UpdateAdvancements, UpdateRecipes, UpdateTags,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionId(pub u64);

#[derive(Debug, Clone, PartialEq)]
pub enum ClientboundPlayPacket {
    /// `ClientboundBossEventPacket`, broadcast by `ServerBossEvent` mutations
    /// and player membership changes (`ServerBossEvent.java:25-135`).
    BossEvent(BossEvent),
    BlockChangedAck(BlockChangedAck),
    BlockDestruction(BlockDestruction),
    /// `ClientboundBlockEventPacket`, which `ServerLevel.runBlockEvents`
    /// (`ServerLevel.java:1248-1258`) broadcasts for every queued event whose
    /// block answered `triggerEvent` with true.
    BlockEvent(BlockEvent),
    /// `ClientboundBlockEntityDataPacket`, which
    /// `ChunkHolder.broadcastBlockEntity` (`ChunkHolder.java:226-233`) sends
    /// beside a block update whenever the block entity there answers
    /// `getUpdatePacket` (`BlockEntity.java:213-215`) with one.
    BlockEntityData(BlockEntityData),
    BlockUpdate(BlockUpdate),
    /// `ClientboundOpenSignEditorPacket`, whose one producer is
    /// `ServerPlayer.openTextEdit` (`ServerPlayer.java:1304-1307`).
    OpenSignEditor(OpenSignEditor),
    /// `ClientboundOpenBookPacket`, sent by `ServerPlayer.openItemGui`
    /// (`ServerPlayer.java:1383-1391`).
    OpenBook(OpenBook),
    /// `ClientboundMountScreenOpenPacket`, sent by
    /// `ServerPlayer.openHorseInventory` (`ServerPlayer.java:1357-1367`).
    MountScreenOpen(MountScreenOpen),
    ChangeDifficulty(ChangeDifficulty),
    ChunkBatchStart,
    ChunkBatchFinished {
        batch_size: i32,
    },
    Login(PlayLogin),
    KeepAlive {
        id: i64,
    },
    PlayerAbilities(PlayerAbilities),
    ContainerSetContent(ContainerSetContent),
    ContainerSetSlot(ContainerSetSlot),
    ContainerSetData(ContainerSetData),
    /// `ClientboundOpenScreenPacket`, whose one producer is
    /// `ServerPlayer.openMenu` (`ServerPlayer.java:1337`).
    OpenScreen(OpenScreen),
    /// `ClientboundContainerClosePacket`, whose one producer is
    /// `ServerPlayer.closeContainer` (`ServerPlayer.java:1400`) — the
    /// server-initiated close. A close the client asked for answers nothing.
    ContainerClose(ContainerClose),
    /// `ClientboundMerchantOffersPacket`, from `Merchant.openTradingScreen`
    /// (`Merchant.java:37-45`).
    MerchantOffers(bbb_protocol::packets::MerchantOffers),
    SetCursorItem(SetCursorItem),
    SetHeldSlot(SetHeldSlot),
    /// `ClientboundCooldownPacket`, sent by `ServerItemCooldowns`
    /// (`ServerItemCooldowns.java:15-24`) when a session's item cooldown
    /// starts or ends -- an ender pearl's `use_cooldown` is the one this world
    /// starts.
    Cooldown(bbb_protocol::packets::Cooldown),
    SetPlayerInventory(SetPlayerInventory),
    EntityEvent(EntityEvent),
    /// `ClientboundUpdateMobEffectPacket`, which `LivingEntity.onEffectAdded`
    /// and `onEffectUpdated` (`LivingEntity.java:1054-1060`, `:1070-1082`) send to every
    /// session tracking the entity, plus the passenger fan-out
    /// `sendEffectToPassengers` (`LivingEntity.java:1062-1068`) reads off the
    /// live passenger list rather than off the tracking set.
    UpdateMobEffect(bbb_protocol::packets::UpdateMobEffect),
    /// `ClientboundRemoveMobEffectPacket`, the matching send of
    /// `LivingEntity.onEffectsRemoved` (`LivingEntity.java:1084-1100`).
    RemoveMobEffect(bbb_protocol::packets::RemoveMobEffect),
    AwardStats(AwardStats),
    SelectAdvancementsTab(SelectAdvancementsTab),
    CommandSuggestions(CommandSuggestions),
    Commands(bbb_protocol::packets::Commands),
    SystemChat(SystemChat),
    PlayerChat(bbb_protocol::packets::UnsignedPlayerChat),
    DisguisedChat(bbb_protocol::packets::DisguisedChatMessage),
    DeleteChat(bbb_protocol::packets::DeleteChat),
    CustomChatCompletions(bbb_protocol::packets::CustomChatCompletions),
    PlayerPosition(PlayerPositionUpdate),
    /// `ClientboundPlayerLookAtPacket`, whose only producer in the 26.1 tree is
    /// `ServerPlayer.lookAt` (ServerPlayer.java:1575-1584). Both commands that
    /// reach it through `LookAt.perform` (LookAt.java:15-20, `:26-28`) are
    /// published: `/teleport .. facing ..` (TeleportCommand.java:253-255),
    /// which looks only after the move landed, and `/rotate .. facing ..`
    /// (RotateCommand.java:76), which has no move in front of it.
    PlayerLookAt(PlayerLookAt),
    SetChunkCacheCenter(SetChunkCacheCenter),
    LevelChunkWithLight(Arc<LevelChunkWithLight>),
    LightUpdate(Arc<LightUpdate>),
    ForgetLevelChunk(ForgetLevelChunk),
    SetTime(PlayTime),
    MapItemData(MapItemData),
    /// `ClientboundTrackedWaypointPacket.STREAM_CODEC`
    /// (`ClientboundTrackedWaypointPacket.java:20-28`).
    TrackedWaypoint(TrackedWaypointPacket),
    /// `ClientboundGameEventPacket`, whose one producer here is the
    /// `LEVEL_CHUNKS_LOAD_START` announcement `PlayerList.sendLevelInfo`
    /// (PlayerList.java:655) ends a join with.
    GameEvent(GameEvent),
    /// `ClientboundLevelEventPacket`, which `ServerLevel.levelEvent`
    /// (ServerLevel.java:1095-1107) broadcasts to every session within
    /// [`LEVEL_EVENT_BROADCAST_RANGE`]. This server's producers are the block
    /// break -- `Level.destroyBlock` (Level.java:283) and
    /// `Block.spawnDestroyParticles` (Block.java:514-516) -- and the lava
    /// fizz [`crate::fluid`] raises.
    LevelEvent(bbb_protocol::packets::LevelEvent),
    /// `ClientboundLevelParticlesPacket`, which `ServerLevel.sendParticles`
    /// (`ServerLevel.java:1321-1330`) broadcasts to every session within
    /// [`crate::level_event::LEVEL_PARTICLE_BROADCAST_RANGE`].
    LevelParticles(LevelParticles),
    /// `ClientboundExplodePacket`, which `ServerLevel.explode`
    /// (`ServerLevel.java:1224-1229`) sends to every session of the level
    /// within 64 blocks of the centre.
    Explode(bbb_protocol::packets::Explosion),
    /// `ClientboundTickingStatePacket`, which
    /// `ServerTickRateManager.updateStateToClients`
    /// (ServerTickRateManager.java:33-35) broadcasts whenever the rate or the
    /// frozen flag changes, and `updateJoiningPlayer` (:121-124) resends to an
    /// arriving session.
    TickingState(TickingState),
    /// `ClientboundTickingStepPacket`, from
    /// `ServerTickRateManager.updateStepTicks` (ServerTickRateManager.java:37-39)
    /// and the same join-time resend.
    TickingStep(TickingStep),
    /// `ClientboundSetChunkCacheRadiusPacket`, which `PlayerList.setViewDistance`
    /// (PlayerList.java:801-808) broadcasts before it moves the chunk source.
    SetChunkCacheRadius(SetChunkCacheRadius),
    /// `ClientboundSetSimulationDistancePacket`, which
    /// `PlayerList.setSimulationDistance` (PlayerList.java:810-817) broadcasts
    /// before it moves the chunk source.
    SetSimulationDistance(SetSimulationDistance),
    PongResponse(PongResponse),
    PlayerInfoUpdate(PlayerInfoUpdate),
    PlayerInfoRemove(PlayerInfoRemove),
    /// `ClientboundBundleDelimiterPacket`, emitted before and after the packet
    /// list `ServerEntity.addPairing` sends as one
    /// `ClientboundBundlePacket` (`ServerEntity.java:257-267`). The transport
    /// turns the logical bundle into delimiter, contents, delimiter as
    /// `BundlerInfo.unbundlePacket` specifies (`BundlerInfo.java:18-24`).
    BundleDelimiter,
    AddEntity(AddEntity),
    MoveEntity(EntityMove),
    /// `ClientboundMoveMinecartPacket`, sent by `ServerEntity.handleMinecartPosRot`
    /// (`ServerEntity.java:226-255`) when minecart track interpolation is active.
    MoveMinecartAlongTrack(MoveMinecartAlongTrack),
    MoveVehicle(MoveVehicle),
    EntityPositionSync(EntityPositionSync),
    TeleportEntity(TeleportEntity),
    RemoveEntities(RemoveEntities),
    SetEntityData(SetEntityData),
    DebugEntityValue(Arc<bbb_protocol::packets::DebugEntityValue>),
    /// `ClientboundDebugBlockValuePacket`, the block-keyed half of a level
    /// debug channel (`TrackingDebugSynchronizer.PoiSynchronizer`,
    /// TrackingDebugSynchronizer.java:104-137).
    DebugBlockValue(Arc<bbb_protocol::packets::DebugBlockValue>),
    /// `ClientboundDebugChunkValuePacket`, the chunk-keyed half
    /// (`TrackingDebugSynchronizer.SourceSynchronizer.pollAndSendUpdates`,
    /// TrackingDebugSynchronizer.java:157-164).
    DebugChunkValue(Arc<bbb_protocol::packets::DebugChunkValue>),
    /// `ClientboundDebugEventPacket`, the event-keyed half a game event or a
    /// neighbor update is sent through (`LevelDebugSynchronizers.java:205-209`).
    DebugEvent(bbb_protocol::packets::DebugEvent),
    /// `ClientboundSetEquipmentPacket`, from `ServerEntity.sendPairingData`
    /// (`ServerEntity.java:288-301`) and `LivingEntity.handleEquipmentChanges`
    /// (`LivingEntity.java:2943-2951`).
    SetEquipment(bbb_protocol::packets::SetEquipment),
    /// `ClientboundSetPassengersPacket`, published when a vehicle's direct
    /// passenger list changes (`ServerEntity.java:92-99`, `:303-309`).
    SetPassengers(bbb_protocol::packets::SetPassengers),
    /// `ClientboundSetEntityLinkPacket`, from `ServerEntity.sendPairingData`
    /// (`ServerEntity.java:311-313`) and the two `Leashable` edges that set and
    /// drop a holder (`Leashable.java:133`, `:322`).
    SetEntityLink(bbb_protocol::packets::SetEntityLink),
    /// `ClientboundUpdateAttributesPacket`, from `ServerEntity.sendPairingData`
    /// (`ServerEntity.java:281-286`) and `sendDirtyEntityData` (`:344-349`).
    UpdateAttributes(bbb_protocol::packets::UpdateAttributes),
    /// `ClientboundRotateHeadPacket`, from `ServerEntity.sendChanges`
    /// (`ServerEntity.java:210-214`).
    RotateHead(bbb_protocol::packets::RotateHead),
    /// `ClientboundTakeItemEntityPacket`, which `LivingEntity.take`
    /// (LivingEntity.java:3253-3261) sends to everyone tracking the picked-up
    /// entity. The pickup *sound* is not here and is not missing: the client
    /// plays `SoundEvents.ITEM_PICKUP` itself when this packet lands
    /// (`ClientPacketListener.handleTakeItemEntity`,
    /// ClientPacketListener.java:999-1009), so the server sends none.
    TakeItemEntity(TakeItemEntity),
    /// `ClientboundSetEntityMotionPacket`, which `ServerEntity.sendChanges`
    /// (ServerEntity.java:172-191) sends to every tracker whenever a tracked
    /// entity's velocity moved far enough to be worth predicting from.
    SetEntityMotion(SetEntityMotion),
    /// `ClientboundProjectilePowerPacket`, bundled immediately after motion
    /// for an `AbstractHurtingProjectile` (`ServerEntity.java:172-190`).
    ProjectilePower(ProjectilePower),
    Sound(SoundEvent),
    SoundEntity(SoundEntityEvent),
    /// `ClientboundSectionBlocksUpdatePacket`, several changes of one section
    /// (`ChunkHolder.java:205-207`).
    SectionBlocksUpdate(SectionBlocksUpdate),
    /// `ClientboundServerDataPacket`, sent on join (`PlayerList.java:202-205`).
    ServerData(ServerData),
    /// `ClientboundDebugSamplePacket` (`RemoteSampleLogger.java:21-25`).
    DebugSample(DebugSample),
    /// `ClientboundStopSoundPacket`, whose one producer is `/stopsound`
    /// (StopSoundCommand.java:52-54).
    StopSound(StopSound),
    /// `ClientboundUpdateAdvancementsPacket` (`ClientboundUpdateAdvancementsPacket.java:49-55`),
    /// the packet `PlayerAdvancements.flushDirty` sends (`PlayerAdvancements.java:247-268`).
    UpdateAdvancements(UpdateAdvancements),
    /// `ClientboundRecipeBookAddPacket.STREAM_CODEC` (`ClientboundRecipeBookAddPacket.java:12-18`),
    /// sent by `ServerRecipeBook.addRecipes` (`ServerRecipeBook.java:76`) and
    /// `sendInitialRecipeBook` (`ServerRecipeBook.java:120`).
    RecipeBookAdd(RecipeBookAdd),
    /// `ClientboundPlaceGhostRecipePacket`, the reply `handlePlaceRecipe`
    /// (`ServerGamePacketListenerImpl.java:1974-1976`) sends when the
    /// ingredients are missing.
    PlaceGhostRecipe(PlaceGhostRecipe),
    /// `ClientboundRecipeBookRemovePacket.STREAM_CODEC` (`ClientboundRecipeBookRemovePacket.java:12-14`),
    /// sent by `ServerRecipeBook.removeRecipes` (`ServerRecipeBook.java:94`).
    RecipeBookRemove(RecipeBookRemove),
    /// `ClientboundRecipeBookSettingsPacket.STREAM_CODEC` (`ClientboundRecipeBookSettingsPacket.java:10-12`),
    /// sent by `ServerRecipeBook.sendInitialRecipeBook` (`ServerRecipeBook.java:113`).
    RecipeBookSettings(RecipeBookSettings),
    /// `ClientboundUpdateRecipesPacket.STREAM_CODEC` (`ClientboundUpdateRecipesPacket.java:18-24`),
    /// sent by `PlayerList.placeNewPlayer` (`PlayerList.java:185-187`) and
    /// `reloadResources` (`PlayerList.java:848-853`).
    UpdateRecipes(UpdateRecipes),
    /// `ClientboundClearTitlesPacket.write` (`ClientboundClearTitlesPacket.java:22-24`).
    ClearTitles(ClearTitles),
    /// `ClientboundSetTitleTextPacket.STREAM_CODEC` (`ClientboundSetTitleTextPacket.java:11-13`).
    SetTitleText(SetTitleText),
    /// `ClientboundSetSubtitleTextPacket.STREAM_CODEC`
    /// (`ClientboundSetSubtitleTextPacket.java:11-13`).
    SetSubtitleText(SetSubtitleText),
    /// `ClientboundSetActionBarTextPacket.STREAM_CODEC`
    /// (`ClientboundSetActionBarTextPacket.java:11-13`).
    SetActionBarText(SetActionBarText),
    /// `ClientboundSetTitlesAnimationPacket.write`
    /// (`ClientboundSetTitlesAnimationPacket.java:28-32`).
    SetTitlesAnimation(SetTitlesAnimation),
    /// `ClientboundInitializeBorderPacket.write` (`ClientboundInitializeBorderPacket.java:44-53`).
    InitializeBorder(bbb_protocol::packets::InitializeBorder),
    SetBorderCenter(SetBorderCenter),
    SetBorderLerpSize(SetBorderLerpSize),
    SetBorderSize(SetBorderSize),
    SetBorderWarningDelay(SetBorderWarningDelay),
    SetBorderWarningDistance(SetBorderWarningDistance),
    /// `ClientboundSetExperiencePacket`, which `ServerPlayer.doTick`
    /// (ServerPlayer.java:710-713) sends whenever the total moved.
    SetExperience(PlayerExperience),
    /// `ClientboundSetHealthPacket`, which `ServerPlayer.doTick`
    /// (ServerPlayer.java:671-678) sends to the hurt player alone whenever the
    /// health it last sent no longer matches the health it holds.
    SetHealth(PlayerHealth),
    /// `ClientboundDamageEventPacket`, which `ServerLevel.broadcastDamageEvent`
    /// (ServerLevel.java:1186-1188) sends to everyone tracking the hurt entity
    /// and to the entity itself when it is a player.
    DamageEvent(DamageEvent),
    /// `ClientboundHurtAnimationPacket`, which `ServerPlayer.indicateDamage`
    /// (`ServerPlayer.java:2115-2118`) sends to the hurt player alone.
    HurtAnimation(HurtAnimation),
    /// `ClientboundGameRuleValuesPacket`, which `sendGameRuleValues`
    /// (`ServerGamePacketListenerImpl.java:1893-1906`) sends the gamemaster who
    /// asked.
    GameRuleValues(GameRuleValues),
    /// `ClientboundTagQueryPacket`, the answer `handleEntityTagQuery` and
    /// `handleBlockEntityTagQuery` (`ServerGamePacketListenerImpl.java:1017-1030`,
    /// `:1043-1050`) send the gamemaster who asked.
    TagQuery(TagQuery),
    /// `ClientboundPlayerCombatEnterPacket` (`ServerPlayer.onEnterCombat`).
    PlayerCombatEnter,
    /// `ClientboundPlayerCombatEndPacket` (`ServerPlayer.onLeaveCombat`).
    PlayerCombatEnd(bbb_protocol::packets::PlayerCombatEnd),
    /// `ClientboundPlayerCombatKillPacket`, which `ServerPlayer.die`
    /// (ServerPlayer.java:874-889) sends to the dead player.
    PlayerCombatKill(PlayerCombatKill),
    /// `ClientboundStartConfigurationPacket` (`ServerGamePacketListenerImpl.switchToConfig`).
    StartConfiguration,
    /// `ClientboundRespawnPacket`, which `PlayerList.respawn`
    /// (PlayerList.java:412) sends to the session it just rebuilt.
    Respawn(Respawn),
    /// `ClientboundSetDefaultSpawnPositionPacket`, which
    /// `MinecraftServer.setRespawnData` (MinecraftServer.java:1699-1706)
    /// broadcasts when the stored `RespawnData` changes.
    SetDefaultSpawnPosition(SetDefaultSpawnPosition),
    /// `ClientboundAnimatePacket`, which `LivingEntity.swing`
    /// (LivingEntity.java:1998-2006) sends to everyone tracking the swinging
    /// entity -- and to the entity itself when the caller asked for it, which
    /// `/swing` does (SwingCommand.java:40).
    EntityAnimation(EntityAnimation),
    /// `ClientboundPlayerRotationPacket`, whose only sender is
    /// `ServerPlayer.forceSetRotation` (ServerPlayer.java:1138-1141) -- and the
    /// only command that reaches that is `/rotate` (RotateCommand.java:70).
    PlayerRotation(PlayerRotationUpdate),
    /// `ClientboundSetCameraPacket`, which `ServerPlayer.setCamera`
    /// (ServerPlayer.java:1928) sends to the session whose camera moved.
    SetCamera(SetCamera),
    /// `ClientboundTransferPacket`, which `/transfer` sends to each player it
    /// names (TransferCommand.java:61).
    Transfer(Transfer),
    /// `ClientboundUpdateTagsPacket`, which `PlayerList.reloadResources`
    /// broadcasts to every online player after a successful resource reload
    /// (`PlayerList.java:841-856`). Registry entry catalogs remain fixed, so
    /// this packet can replace tag membership without changing holder ids.
    UpdateTags(UpdateTags),
    /// `ClientboundSetObjectivePacket`: `ServerScoreboard.onObjectiveChanged`
    /// (`ServerScoreboard.java:143-150`) broadcasts the change, while
    /// `getStartTrackingPackets` (`ServerScoreboard.java:196-198`) and
    /// `getStopTrackingPackets` (`ServerScoreboard.java:229-231`) carry the add
    /// and the remove.
    SetObjective(bbb_protocol::packets::SetObjective),
    /// `ClientboundSetScorePacket`, which `ServerScoreboard.onScoreChanged`
    /// (`ServerScoreboard.java:51-68`) broadcasts for a tracked objective and
    /// `getStartTrackingPackets` (`ServerScoreboard.java:206-212`) replays once
    /// per score a newly tracked objective already holds.
    SetScore(bbb_protocol::packets::SetScore),
    /// `ClientboundResetScorePacket`, broadcast by
    /// `ServerScoreboard.onPlayerRemoved` (`ServerScoreboard.java:77-81`) with
    /// no objective at all, and by `onPlayerScoreRemoved`
    /// (`ServerScoreboard.java:84-91`) with the one objective that lost it.
    ResetScore(bbb_protocol::packets::ResetScore),
    /// `ClientboundSetDisplayObjectivePacket`, which
    /// `ServerScoreboard.setDisplayObjective` (`ServerScoreboard.java:94-114`)
    /// broadcasts for a slot whose objective is already tracked.
    SetDisplayObjective(bbb_protocol::packets::SetDisplayObjective),
    /// `ClientboundSetPlayerTeamPacket`: `onTeamAdded`, `onTeamChanged` and
    /// `onTeamRemoved` (`ServerScoreboard.java:163-183`) carry the team itself,
    /// while `addPlayerToTeam` and `removePlayerFromTeam`
    /// (`ServerScoreboard.java:117-134`) carry one membership change.
    SetPlayerTeam(bbb_protocol::packets::SetPlayerTeam),
    /// `ClientboundClearDialogPacket.STREAM_CODEC`
    /// (`ClientboundClearDialogPacket.java:9-10`).
    ClearDialog,
    /// `ClientboundShowDialogPacket.STREAM_CODEC`
    /// (`ClientboundShowDialogPacket.java:12-14`).
    ShowDialog(bbb_protocol::packets::ShowDialog),
    /// `ClientboundChunksBiomesPacket` (`ClientboundChunksBiomesPacket.java:24-26`).
    ChunksBiomes(ChunksBiomes),
    CustomPayload(Arc<bbb_protocol::packets::client_common::CustomPayload>),
}
