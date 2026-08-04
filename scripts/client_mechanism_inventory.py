"""Tree-sitter-derived Vanilla 26.1 external-client mechanism inventory.

The selection manifest in this module freezes the bounded audit surface. Java
owners, declarations, registrations, call sites, source locations, and source
hashes are always derived from :mod:`vanilla_java_source`; the manifest does
not duplicate any Vanilla registration or packet table.
"""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
from typing import Iterable

from packet_catalog_extractor import packet_resource_ids, registered_packets
from vanilla_java_source import JavaMethod, JavaSource, descendants, parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
GENERATOR_INPUTS = (
    "scripts/client_mechanism_inventory.py",
    "scripts/generate-client-mechanism-inventory.py",
    "scripts/packet_catalog_extractor.py",
    "scripts/vanilla_java_source.py",
)
PROTOCOL_ROOT = Path("net/minecraft/network/protocol")
PROTOCOL_SPECS = (
    ("handshake", "handshake/HandshakeProtocols.java", ("serverbound",)),
    ("status", "status/StatusProtocols.java", ("serverbound", "clientbound")),
    ("login", "login/LoginProtocols.java", ("serverbound", "clientbound")),
    (
        "configuration",
        "configuration/ConfigurationProtocols.java",
        ("serverbound", "clientbound"),
    ),
    ("play", "game/GameProtocols.java", ("serverbound", "clientbound")),
)


@dataclass(frozen=True)
class MethodSurface:
    category: str
    domain: str
    mechanism_id: str
    source_path: str
    scope: str = "external-client"


@dataclass(frozen=True)
class MethodRoute:
    category: str
    domain: str
    mechanism_id: str
    scope: str = "external-client"
    role: str = "decision"


@dataclass(frozen=True)
class OwnerEvidence:
    implementation: tuple[str, ...]
    tests: tuple[str, ...]


def _method_routes(
    *groups: tuple[MethodRoute, tuple[str, ...]],
) -> dict[str, MethodRoute]:
    routes: dict[str, MethodRoute] = {}
    for route, method_names in groups:
        for method_name in method_names:
            if method_name in routes:
                raise ValueError(f"duplicate method route: {method_name}")
            routes[method_name] = route
    return routes


MINECRAFT_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("startup", "startup", "startup-client-bootstrap"),
        (
            "Minecraft",
            "onGameLoadFinished",
            "getLauncherBrand",
        ),
    ),
    (
        MethodRoute("startup", "identity", "startup-session-identity"),
        (
            "createUserApiService",
            "getGameProfile",
        ),
    ),
    (
        MethodRoute("lifecycle", "lifecycle", "lifecycle-client-runtime"),
        (
            "run",
            "runTick",
            "tick",
            "isLevelRunningNormally",
            "isMultiplayerServer",
            "destroy",
            "close",
            "stop",
            "getTickTargetMillis",
        ),
    ),
    (
        MethodRoute("lifecycle", "lifecycle", "lifecycle-session-replacement"),
        (
            "setLevel",
            "disconnectFromWorld",
            "disconnectWithProgressScreen",
            "disconnect",
            "clearClientLevel",
            "setScreenAndShow",
            "updateLevelInEngines",
            "getConnection",
            "getCurrentServer",
        ),
    ),
    (
        MethodRoute("lifecycle", "lifecycle", "lifecycle-client-task-handoff"),
        (
            "getRunningThread",
            "wrapRunnable",
            "shouldRun",
        ),
    ),
    (
        MethodRoute("resources", "resources", "resources-reload-recovery"),
        (
            "onResourceLoadFinished",
            "rollbackResourcePacks",
            "clearResourcePacksOnError",
            "abortResourcePackRecovery",
            "addResourcePackLoadFailToast",
            "triggerResourcePackRecovery",
            "reloadResourcePacks",
            "delayTextureReload",
        ),
    ),
    (
        MethodRoute("resources", "resources", "resources-runtime-mipmap-update"),
        ("updateMaxMipLevel",),
    ),
    (
        MethodRoute("resources", "resources", "resources-server-pack-lifecycle"),
        ("clearDownloadedResourcePacks",),
    ),
    (
        MethodRoute("text", "text", "ui-font-manager"),
        (
            "updateFontOptions",
            "isEnforceUnicode",
        ),
    ),
    (
        MethodRoute("audio", "audio", "audio-sound-manager-engine"),
        (
            "getSituationalMusic",
            "getMusicVolume",
        ),
    ),
    (
        MethodRoute("input", "input", "input-keyboard-routing"),
        (
            "hasShiftDown",
            "hasControlDown",
            "hasAltDown",
            "handleKeybinds",
            "setLastInputType",
        ),
    ),
    (
        MethodRoute(
            "interaction-routing",
            "interaction",
            "interaction-primary-action-routing",
        ),
        ("continueAttack", "startAttack", "startUseItem"),
    ),
    (
        MethodRoute("game-mode", "interaction", "interaction-game-mode-lifecycle"),
        ("canSwitchGameMode",),
    ),
    (
        MethodRoute("pick-action", "interaction", "interaction-pick-actions"),
        ("pickBlockOrEntity",),
    ),
    (
        MethodRoute("targeting", "interaction", "player-local-camera-targeting"),
        ("pick",),
    ),
    (
        MethodRoute("platform", "platform", "platform-window-lifecycle"),
        (
            "updateTitle",
            "createTitle",
            "onFullscreenError",
            "pauseIfInactive",
            "resizeGui",
            "cursorEntered",
            "isWindowActive",
        ),
    ),
    (
        MethodRoute("ui", "ui", "ui-chat-screen-surface"),
        ("openChatScreen",),
    ),
    (
        MethodRoute("ui", "ui", "ui-screen-lifecycle"),
        (
            "setScreen",
            "setOverlay",
            "canInterruptScreen",
        ),
    ),
    (
        MethodRoute("ui", "ui", "ui-pause-screen-surface"),
        ("pauseGame",),
    ),
    (
        MethodRoute("render-step", "rendering", "rendering-game-step-surface"),
        (
            "renderFrame",
            "renderNames",
            "useShaderTransparency",
            "getCameraEntity",
            "setCameraEntity",
            "shouldEntityAppearGlowing",
            "getEntityModels",
        ),
    ),
    (
        MethodRoute("render-step", "ui", "rendering-hud-step-surface"),
        (
            "showOnlyReducedInfo",
            "getDebugOverlay",
        ),
    ),
    (
        MethodRoute("input", "input", "ui-accessibility-surface"),
        ("onTextInputFocusChange",),
    ),
    (
        MethodRoute("identity", "identity", "identity-profile-key-lifecycle"),
        ("prepareForMultiplayer",),
    ),
    (
        MethodRoute("security", "security", "security-secure-chat-reporting"),
        (
            "userProperties",
            "allowsMultiplayer",
            "multiplayerBan",
            "isNameBanned",
            "isBlocked",
            "isLocalOrUnknownPlayer",
            "computeChatAbilities",
            "isLocalPlayer",
            "isTextFilteringEnabled",
        ),
    ),
    (
        MethodRoute("diagnostics", "diagnostics", "diagnostics-crash-runtime"),
        (
            "checkModStatus",
            "delayCrash",
            "emergencySaveAndCrash",
            "saveReport",
            "crash",
            "saveReportAndShutdownSoundManager",
            "selfTest",
            "sendLowDiskSpaceWarning",
            "emergencySave",
            "debugClientMetricsStart",
            "debugClientMetricsStop",
            "debugClientMetricsCancel",
            "archiveProfilingReport",
            "constructProfiler",
            "finishProfilers",
            "fillReport",
            "fillUptime",
            "fillSystemReport",
            "formatSeconds",
            "updateReportEnvironment",
            "collectPerTickGizmos",
        ),
    ),
    (
        MethodRoute("out-of-scope", "ui", "scope-title-screen", "out-of-scope", "out-of-scope"),
        (
            "buildInitialScreens",
            "addInitialScreens",
            "countryEqualsISO3",
            "getQuickActionsDialog",
        ),
    ),
    (
        MethodRoute(
            "out-of-scope",
            "singleplayer",
            "scope-singleplayer-integrated",
            "out-of-scope",
            "out-of-scope",
        ),
        (
            "createWorldOpenFlows",
            "doWorldLoad",
            "disconnectWithSavingScreen",
            "hasSingleplayerServer",
            "isSingleplayer",
        ),
    ),
    (
        MethodRoute("out-of-scope", "realms", "scope-realms", "out-of-scope", "out-of-scope"),
        ("allowsRealms",),
    ),
    (
        MethodRoute(
            "out-of-scope",
            "telemetry",
            "scope-telemetry-upload",
            "out-of-scope",
            "out-of-scope",
        ),
        (
            "telemetryOptInExtra",
            "extraTelemetryAvailable",
            "allowsTelemetry",
        ),
    ),
    (
        MethodRoute(
            "out-of-scope",
            "rendering",
            "rendering-nondeterministic-golden-exclusions",
            "out-of-scope",
            "out-of-scope",
        ),
        ("grabPanoramixScreenshot",),
    ),
)

DIRECT_SUPPORT_ROUTE = MethodRoute(
    "support-accessor",
    "support",
    "scope-support-accessors",
    "out-of-scope",
    "support-non-decision",
)


# `Options` keeps its 94 `OptionInstance` getters as direct field returns, so
# they fall through to `DIRECT_SUPPORT_ROUTE`; the persisted semantics live in
# the field initializers that `processOptions` names and clamps. Only the 51
# methods with a body decision are routed here.
OPTIONS_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("persistence", "persistence", "persistence-options-file"),
        (
            "Options",
            "load",
            "save",
            "dataFix",
            "processOptions",
            "processDumpedOptions",
            "process",
            "isTrue",
            "isFalse",
            "readListOfStrings",
            "getFullscreenVideoModeString",
        ),
    ),
    (
        MethodRoute("persistence", "persistence", "persistence-options-derived"),
        (
            "getEffectiveRenderDistance",
            "setServerRenderDistance",
            "getFinalSoundSourceVolume",
            "getSoundSourceVolume",
            "getSoundSourceOptionInstance",
            "createSoundSliderOptionInstance",
            "isSoundDeviceDefault",
            "getBackgroundOpacity",
            "getBackgroundColor",
            "getMenuBackgroundBlurriness",
            "maxAnisotropyValue",
            "japaneseGlyphVariantsDefault",
            "getCloudStatus",
            "isModelPartEnabled",
            "setModelPart",
            "setCameraType",
            "onboardingAccessibilityFinished",
            "logMouse",
            "unlogMouse",
            "applyGraphicsPreset",
            "setGraphicsPresetToCustom",
        ),
    ),
    (
        MethodRoute("persistence", "persistence", "persistence-options-apply"),
        (
            "buildPlayerInformation",
            "broadcastOptions",
            "operateOnLevelRenderer",
            "updateResourcePacks",
            "loadSelectedResourcePacks",
            "updateFontOptions",
            "dumpOptionsForReport",
        ),
    ),
    (
        # Caption/label factories consumed only by `OptionsSubScreen` widgets;
        # the in-game options UI is a declared non-goal.
        MethodRoute("out-of-scope", "options", "scope-options-ui", "out-of-scope", "out-of-scope"),
        (
            "pixelValueLabel",
            "percentValueLabel",
            "genericValueLabel",
            "genericValueOrOffLabel",
            "percentValueOrOffLabel",
        ),
    ),
)


LOCAL_PLAYER_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("player-lifecycle", "movement", "player-local-session-respawn"),
        ("LocalPlayer", "respawn", "resetPos", "tickDeath"),
    ),
    (
        MethodRoute("player-state", "movement", "player-local-vitals-status"),
        ("heal", "hurtTo", "setExperienceValues", "setExperienceDisplayStartTickToTickCount"),
    ),
    (
        MethodRoute("movement-report", "movement", "player-local-movement-reporting"),
        ("tick", "sendPosition", "sendIsSprintingIfNeeded", "onUpdateAbilities"),
    ),
    (
        MethodRoute("movement", "movement", "player-local-input-movement"),
        (
            "isSuppressingSlidingDownLadder",
            "canSpawnSprintParticle",
            "moveTowardsClosestSpace",
            "suffocatesAt",
            "isShiftKeyDown",
            "isMovingSlowly",
            "applyInput",
            "modifyInput",
            "modifyInputSpeedForSquareMovement",
            "distanceToUnitSquare",
            "aiStep",
            "shouldStopRunSprinting",
            "shouldStopSwimSprinting",
            "move",
            "updateAutoJump",
            "isHorizontalCollisionMinor",
            "canAutoJump",
            "isMoving",
            "isSprintingPossible",
            "canStartSprinting",
        ),
    ),
    (
        MethodRoute("vehicle-control", "movement", "player-local-vehicle-control"),
        (
            "startRiding",
            "removeVehicle",
            "sendRidingJump",
            "jumpableVehicle",
            "rideTick",
            "shouldRotateWithMinecart",
            "vehicleCanSprint",
        ),
    ),
    (
        MethodRoute("player-state", "identity", "player-local-permission-chat-state"),
        (
            "setPermissions",
            "refreshChatAbilities",
            "sendSystemMessage",
            "sendOverlayMessage",
            "handleEntityEvent",
            "isTextFilteringEnabled",
        ),
    ),
    (
        MethodRoute("player-state", "ui", "player-local-client-status-options"),
        (
            "setReducedDebugInfo",
            "setShowDeathScreen",
            "setDoLimitedCrafting",
            "onGameModeChanged",
        ),
    ),
    (
        MethodRoute("item-use", "interaction", "player-local-item-use-state"),
        (
            "isSlowDueToUsingItem",
            "itemUseSpeedMultiplier",
            "startUsingItem",
            "stopUsingItem",
            "getUsedItemHand",
            "onSyncedDataUpdated",
        ),
    ),
    (
        MethodRoute("inventory-action", "interaction", "player-local-inventory-actions"),
        (
            "drop",
            "swing",
            "closeContainer",
            "clientSideCloseContainer",
            "sendOpenInventory",
            "removeRecipeHighlight",
            "updateTutorialInventoryAction",
            "handleCreativeModeItemDrop",
            "canDropItems",
        ),
    ),
    (
        MethodRoute("screen-open", "ui", "player-local-world-edit-ui"),
        (
            "openTextEdit",
            "openMinecartCommandBlock",
            "openCommandBlock",
            "openStructureBlock",
            "openTestBlock",
            "openTestInstanceBlock",
            "openJigsawBlock",
            "openDialog",
            "openItemGui",
        ),
    ),
    (
        MethodRoute("local-effect", "audio", "player-local-environment-effects"),
        (
            "getCurrentMood",
            "playSound",
            "crit",
            "magicCrit",
            "getActivePortalLocalTransition",
            "handlePortalTransitionEffect",
            "getWaterVision",
            "updateIsUnderwater",
        ),
    ),
    (
        MethodRoute("targeting", "interaction", "player-local-camera-targeting"),
        (
            "getViewXRot",
            "getViewYRot",
            "isControlledCamera",
            "getRopeHoldPosition",
            "getVisualRotationYInDegrees",
            "raycastHitResult",
            "pick",
            "filterHitResult",
        ),
    ),
    (
        DIRECT_SUPPORT_ROUTE,
        (
            "isLocalPlayer",
            "getStats",
            "getRecipeBook",
            "permissions",
            "chatAbilities",
            "shouldShowDeathScreen",
            "getDoLimitedCrafting",
            "isUsingItem",
            "getJumpRidingScale",
            "isCrouching",
            "isHandsBusy",
            "isAutoJumpEnabled",
            "isUnderWater",
            "getDropSpamThrottler",
            "getLastSentInput",
        ),
    ),
)


MULTIPLAYER_GAME_MODE_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("game-mode", "interaction", "interaction-game-mode-lifecycle"),
        ("MultiPlayerGameMode", "adjustPlayer", "setLocalMode", "createPlayer"),
    ),
    (
        MethodRoute("game-mode", "interaction", "interaction-game-mode-guards"),
        (
            "canHurtPlayer",
            "hasExperience",
            "hasMissTime",
            "isServerControlledInventory",
            "isSpectator",
        ),
    ),
    (
        MethodRoute("block-breaking", "interaction", "interaction-block-breaking"),
        (
            "destroyBlock",
            "startDestroyBlock",
            "stopDestroyBlock",
            "continueDestroyBlock",
            "sameDestroyTarget",
            "getDestroyStage",
        ),
    ),
    (
        MethodRoute("prediction", "interaction", "interaction-prediction-sequence"),
        ("startPrediction",),
    ),
    (
        MethodRoute("carried-slot", "interaction", "interaction-carried-slot-sync"),
        ("tick", "ensureHasSentCarriedItem"),
    ),
    (
        MethodRoute("item-use", "interaction", "interaction-item-use"),
        ("useItemOn", "performUseItemOn", "useItem", "releaseUsingItem"),
    ),
    (
        MethodRoute("entity-action", "interaction", "interaction-entity-actions"),
        ("attack", "spectate", "interact", "piercingAttack"),
    ),
    (
        MethodRoute("container-action", "interaction", "interaction-container-actions"),
        (
            "handleContainerInput",
            "handlePlaceRecipe",
            "handleInventoryButtonClick",
            "handleSlotStateChanged",
        ),
    ),
    (
        MethodRoute("creative-action", "interaction", "interaction-creative-actions"),
        ("handleCreativeModeItemAdd", "handleCreativeModeItemDrop"),
    ),
    (
        MethodRoute("pick-action", "interaction", "interaction-pick-actions"),
        ("handlePickItemFromBlock", "handlePickItemFromEntity"),
    ),
    (
        DIRECT_SUPPORT_ROUTE,
        ("getPreviousPlayerMode", "getPlayerMode", "isDestroying"),
    ),
)


CLIENT_LEVEL_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("level-lifecycle", "world", "world-client-level-lifecycle"),
        ("ClientLevel", "disconnect", "registerForCleaning"),
    ),
    (
        MethodRoute(
            "block-prediction",
            "interaction",
            "interaction-prediction-sequence",
        ),
        (
            "handleBlockChangedAck",
            "setServerVerifiedBlockState",
            "syncBlockState",
            "setBlock",
        ),
    ),
    (
        MethodRoute(
            "block-entity-render-tracking",
            "rendering",
            "rendering-client-block-entity-tracking",
        ),
        ("onBlockEntityAdded",),
    ),
    (
        MethodRoute("clock-weather", "world", "world-client-clock-weather"),
        (
            "addEnvironmentAttributeLayers",
            "tick",
            "tickTime",
            "setTimeFromServer",
            "clockManager",
            "getSkyFlashTime",
            "setSkyFlashTime",
            "setGameTime",
        ),
    ),
    (
        MethodRoute(
            "chunk-light-lifecycle",
            "world",
            "world-client-chunk-light-lifecycle",
        ),
        (
            "queueLightUpdate",
            "pollLightUpdates",
            "update",
            "unload",
            "onChunkLoaded",
        ),
    ),
    (
        MethodRoute(
            "render-dirty-handoff",
            "rendering",
            "rendering-client-level-dirty-handoff",
        ),
        (
            "onSectionBecomingNonEmpty",
            "sendBlockUpdated",
            "setBlocksDirty",
            "setSectionDirtyWithNeighbors",
            "setSectionRangeDirty",
            "destroyBlockProgress",
        ),
    ),
    (
        MethodRoute("entity-lifecycle", "world", "world-client-entity-lifecycle"),
        (
            "entitiesForRendering",
            "tickEntities",
            "isTickingEntity",
            "shouldTickDeath",
            "tickNonPassenger",
            "tickPassenger",
            "getEntityCount",
            "addEntity",
            "removeEntity",
            "getPushableEntities",
            "getEntity",
            "getEntities",
            "setServerSimulationDistance",
            "onTickingStart",
            "onTickingEnd",
            "onTrackingStart",
            "onTrackingEnd",
        ),
    ),
    (
        MethodRoute(
            "ambient-animation",
            "effects",
            "world-client-ambient-animation",
        ),
        (
            "animateTick",
            "getMarkerParticleTarget",
            "doAnimateTick",
            "trySpawnDripParticles",
            "spawnParticle",
            "spawnFluidParticle",
            "createFireworks",
            "addDestroyBlockEffect",
            "addBreakingBlockEffect",
        ),
    ),
    (
        MethodRoute(
            "particle-routing",
            "effects",
            "rendering-client-particle-routing",
        ),
        (
            "addParticle",
            "addAlwaysVisibleParticle",
            "doAddParticle",
            "calculateParticleLevel",
        ),
    ),
    (
        MethodRoute(
            "sound-routing",
            "audio",
            "audio-client-level-sound-routing",
        ),
        (
            "playSeededSound",
            "playLocalSound",
            "playPlayerSound",
            "playSound",
        ),
    ),
    (
        MethodRoute(
            "session-services",
            "world",
            "world-client-session-services",
        ),
        (
            "sendPacketToServer",
            "recipeAccess",
            "getBlockTicks",
            "getFluidTicks",
            "getScoreboard",
            "enabledFeatures",
            "potionBrewing",
            "fuelValues",
        ),
    ),
    (
        MethodRoute("map-data", "world", "world-client-map-data"),
        ("getMapData", "overrideMapData", "getAllMapData", "addMapData"),
    ),
    (
        MethodRoute("level-events", "effects", "world-client-level-events"),
        ("globalLevelEvent", "levelEvent", "trackExplosionEffects"),
    ),
    (
        MethodRoute(
            "biome-tint",
            "rendering",
            "rendering-client-biome-tint",
        ),
        (
            "clearTintCaches",
            "getUncachedNoiseBiome",
            "cardinalLighting",
            "getBlockTint",
            "calculateBlockTint",
            "getClientLeafTintColor",
        ),
    ),
    (
        MethodRoute("respawn-data", "world", "world-client-respawn-data"),
        (
            "setRespawnData",
            "getRespawnData",
            "ClientLevelData",
            "setSpawn",
            "setDifficulty",
            "setDifficultyLocked",
            "getHorizonHeight",
            "voidDarknessOnsetRange",
        ),
    ),
    (
        MethodRoute(
            "level-diagnostics",
            "diagnostics",
            "diagnostics-client-level-report",
        ),
        (
            "fillReportDetails",
            "fillCrashReportCategory",
            "gatherChunkSourceStats",
            "toString",
        ),
    ),
    (
        MethodRoute(
            "client-noop",
            "world",
            "scope-client-level-noop-hooks",
        ),
        ("gameEvent", "explode", "onCreated", "onDestroyed", "onSectionChange"),
    ),
    (
        DIRECT_SUPPORT_ROUTE,
        (
            "getGloballyRenderedBlockEntities",
            "getBlockStatePredictionHandler",
            "endFlashState",
            "hasChunk",
            "getWorldBorder",
            "tickRateManager",
            "environmentAttributes",
            "getChunkSource",
            "players",
            "dragonParts",
            "getLevelData",
            "getServerSimulationDistance",
            "getSeaLevel",
            "getGameTime",
            "isHardcore",
            "getDifficulty",
            "isDifficultyLocked",
        ),
    ),
)


LEVEL_RENDERER_METHOD_ROUTES = _method_routes(
    (
        MethodRoute(
            "render-lifecycle",
            "rendering",
            "rendering-level-backend-lifecycle",
        ),
        (
            "LevelRenderer",
            "close",
            "onResourceManagerReload",
            "initOutline",
            "setLevel",
            "allChanged",
            "resize",
            "resetSampler",
            "needsUpdate",
        ),
    ),
    (
        MethodRoute(
            "render-section",
            "rendering",
            "rendering-level-section-lifecycle",
        ),
        (
            "clearVisibleSections",
            "cullTerrain",
            "offsetFrustum",
            "applyFrustum",
            "addRecentlyCompiledSection",
            "update",
            "scheduleTranslucentSectionResort",
            "scheduleResort",
            "compileSections",
            "blockChanged",
            "setBlockDirty",
            "setBlocksDirty",
            "setSectionDirtyWithNeighbors",
            "setSectionRangeDirty",
            "setSectionDirty",
            "onSectionBecomingNonEmpty",
            "hasRenderedAllSections",
            "onChunkReadyToRender",
            "isSectionCompiledAndVisible",
            "prepareChunkRenders",
            "getSectionRenderDispatcher",
            "getTotalSections",
            "getLastViewDistance",
            "countRenderedSections",
            "getVisibleSections",
            "getSectionOcclusionGraph",
            "getSectionStatistics",
        ),
    ),
    (
        MethodRoute("render-step", "rendering", "rendering-level-frame-graph"),
        (
            "getTransparencyChain",
            "renderLevel",
            "addMainPass",
            "addCloudsPass",
            "addWeatherPass",
            "addLateDebugPass",
            "addSkyPass",
            "endFrame",
            "getTranslucentTarget",
            "getItemEntityTarget",
            "getParticlesTarget",
            "getWeatherTarget",
            "getCloudsTarget",
            "getCloudRenderer",
        ),
    ),
    (
        MethodRoute(
            "render-scene",
            "rendering",
            "rendering-level-scene-projection",
        ),
        (
            "doEntityOutline",
            "shouldShowEntityOutlines",
            "extractLevel",
            "extractVisibleEntities",
            "submitEntities",
            "extractVisibleBlockEntities",
            "submitBlockEntities",
            "extractBlockDestroyAnimation",
            "submitBlockDestroyAnimation",
            "extractBlockOutline",
            "renderBlockOutline",
            "checkPoseStack",
            "extractEntity",
            "renderHitOutline",
            "entityOutlineTarget",
        ),
    ),
    (
        MethodRoute(
            "render-effect",
            "rendering",
            "rendering-level-effect-lifecycle",
        ),
        (
            "tick",
            "removeBlockBreakingProgress",
            "removeProgress",
            "destroyBlockProgress",
        ),
    ),
    (
        MethodRoute(
            "render-diagnostics",
            "rendering",
            "rendering-level-diagnostics-gizmos",
        ),
        (
            "getEntityStatistics",
            "collectPerFrameGizmos",
            "finalizeGizmoCollection",
        ),
    ),
    (
        MethodRoute(
            "render-light",
            "rendering",
            "rendering-level-light-query",
        ),
        (
            "getLightCoords",
            "packedBrightness",
        ),
    ),
)


def _owner_evidence(
    *groups: tuple[OwnerEvidence, tuple[str, ...]],
) -> dict[str, OwnerEvidence]:
    evidence: dict[str, OwnerEvidence] = {}
    for owner_evidence, method_names in groups:
        for method_name in method_names:
            if method_name in evidence:
                raise ValueError(f"duplicate owner evidence: {method_name}")
            evidence[method_name] = owner_evidence
    return evidence


LEVEL_RENDERER_FRAME_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/render.rs#render",),
            (
                "crates/bbb-renderer/src/render.rs#render_steps_are_defined_in_frame_execution_order",
                "crates/bbb-renderer/src/render.rs#render_calls_exactly_one_step_per_frame_steps_entry",
            ),
        ),
        ("renderLevel",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/render.rs#main_world_pass",
                "crates/bbb-renderer/src/render.rs#copy_main_depth_to_feature_targets",
                "crates/bbb-renderer/src/render.rs#item_entity_target_pass",
                "crates/bbb-renderer/src/render.rs#translucent_target_pass",
                "crates/bbb-renderer/src/render.rs#particle_target_pass",
            ),
            (
                "crates/bbb-renderer/src/render.rs#level_frame_route_is_the_only_feature_target_and_combine_decision_owner",
                "crates/bbb-renderer/src/offscreen.rs#offscreen_block_selection_renders_both_material_phase_paths",
            ),
        ),
        ("addMainPass",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/render.rs#main_world_pass",),
            ("crates/bbb-renderer/src/render.rs#sky_disc_draws_before_sunrise_and_celestial_layers",),
        ),
        ("addSkyPass",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/render.rs#clouds_pass",),
            (
                "crates/bbb-renderer/src/render.rs#cloud_target_feeds_transparency_combine_after_world_passes",
                "crates/bbb-renderer/src/offscreen.rs#offscreen_regular_and_improved_routes_preserve_clear_color_identity",
            ),
        ),
        ("addCloudsPass", "getCloudsTarget"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/render.rs#weather_target_pass",),
            (
                "crates/bbb-renderer/src/render.rs#weather_target_uses_independently_cleared_depth_before_combine",
                "crates/bbb-renderer/src/render.rs#world_border_draws_into_weather_target_after_rain_snow_and_before_combine",
            ),
        ),
        ("addWeatherPass", "getWeatherTarget"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/render.rs#debug_always_on_top_text_pass",),
            ("crates/bbb-renderer/src/render.rs#always_on_top_debug_text_runs_after_composition_with_cleared_depth",),
        ),
        ("addLateDebugPass",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/renderer.rs#LevelFrameRoute::for_options",
                "crates/bbb-renderer/src/render.rs#transparency_combine_pass",
                "crates/bbb-renderer/src/render.rs#presentation_color_target",
            ),
            (
                "crates/bbb-renderer/src/renderer.rs#level_frame_route_matches_vanilla_option_and_panoramic_suppression",
                "crates/bbb-renderer/src/post_effect_graph/tests.rs#selected_generation_drives_live_transparency_graph",
            ),
        ),
        ("getTransparencyChain",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/render.rs#item_entity_target_pass",),
            (
                "crates/bbb-renderer/src/render.rs#item_entity_target_copies_main_depth_and_splits_item_features_from_line_draws",
                "crates/bbb-renderer/src/render.rs#level_frame_route_is_the_only_feature_target_and_combine_decision_owner",
            ),
        ),
        ("getItemEntityTarget",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/render.rs#particle_target_pass",),
            (
                "crates/bbb-renderer/src/render.rs#particle_target_copies_main_depth_and_clears_before_combine",
                "crates/bbb-renderer/src/render.rs#level_frame_route_is_the_only_feature_target_and_combine_decision_owner",
            ),
        ),
        ("getParticlesTarget",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/render.rs#translucent_target_pass",),
            (
                "crates/bbb-renderer/src/render.rs#translucent_target_copies_main_depth_and_clears_color_before_draws",
                "crates/bbb-renderer/src/render.rs#level_frame_route_is_the_only_feature_target_and_combine_decision_owner",
            ),
        ),
        ("getTranslucentTarget",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/renderer.rs#set_cloud_frame",),
            ("crates/bbb-renderer/src/render.rs#cloud_presentation_binds_cloud_offset_uniform",),
        ),
        ("getCloudRenderer",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/render.rs#finish_frame",),
            (
                "crates/bbb-renderer/src/render.rs#surface_image_is_acquired_at_vanilla_screen_blit_boundary",
                "crates/bbb-renderer/src/render.rs#cloud_presentation_binds_cloud_offset_uniform",
            ),
        ),
        ("endFrame",),
    ),
)


LEVEL_RENDERER_SECTION_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#publish_terrain_frustum_visibility",),
            ("crates/bbb-native/src/terrain_runtime.rs#frame_visibility_matches_vanilla_two_degree_rotation_gate",),
        ),
        ("clearVisibleSections", "applyFrustum", "offsetFrustum"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#refresh_terrain_visibility",),
            ("crates/bbb-native/src/terrain_runtime/occlusion.rs#terrain_runtime_occlusion_partial_compile_propagates_without_full_rebuild",),
        ),
        ("cullTerrain", "getSectionOcclusionGraph"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#accept_terrain_mesh_result",),
            ("crates/bbb-native/src/terrain_runtime.rs#partial_worker_result_preserves_unchanged_section_mesh_and_residency",),
        ),
        ("addRecentlyCompiledSection", "update", "onChunkReadyToRender"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/renderer.rs#resort_translucent_terrain_for_camera",),
            ("crates/bbb-renderer/src/renderer.rs#translucent_sort_state_uses_vanilla_block_and_point_of_view_triggers",),
        ),
        ("scheduleTranslucentSectionResort", "scheduleResort"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#queue_terrain_chunk_mesh",),
            ("crates/bbb-native/src/terrain_runtime.rs#production_compile_order_keeps_vanilla_default_none_policy",),
        ),
        ("compileSections",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#dirty_terrain_sections",),
            ("crates/bbb-native/src/terrain_runtime.rs#block_update_packets_rebuild_dirty_sections_and_reuse_stable_layers",),
        ),
        (
            "blockChanged",
            "setBlockDirty",
            "setBlocksDirty",
            "setSectionDirtyWithNeighbors",
            "setSectionRangeDirty",
            "setSectionDirty",
        ),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#synchronize_chunk_section_residency",),
            ("crates/bbb-native/src/terrain_runtime.rs#nearby_and_previously_empty_sections_do_not_restart_fade_on_recompile",),
        ),
        ("onSectionBecomingNonEmpty",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#terrain_residency_counters",),
            ("crates/bbb-native/src/terrain_runtime.rs#terrain_residency_counters_require_every_selected_section_and_no_pending_job",),
        ),
        ("hasRenderedAllSections", "getSectionStatistics"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#section_residency_is_consumer_visible",),
            ("crates/bbb-native/src/terrain_runtime.rs#player_entity_block_entity_and_readiness_share_the_point_three_gate",),
        ),
        ("isSectionCompiledAndVisible",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/renderer.rs#set_terrain_section_states",),
            ("crates/bbb-renderer/src/renderer.rs#terrain_visibility_filter_changes_deterministic_frame_pixels",),
        ),
        ("prepareChunkRenders",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#selected_terrain_chunks",),
            ("crates/bbb-native/src/terrain_runtime.rs#terrain_selection_keeps_every_server_loaded_chunk_in_distance_order",),
        ),
        ("getLastViewDistance",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/renderer.rs#refresh_terrain_counters",),
            ("crates/bbb-renderer/src/renderer.rs#terrain_visibility_filter_limits_resident_section_draws_and_counter",),
        ),
        ("countRenderedSections",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#terrain_residency_counters",),
            ("crates/bbb-native/src/terrain_runtime.rs#terrain_residency_counters_require_every_selected_section_and_no_pending_job",),
        ),
        ("getTotalSections",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime/mesh_worker.rs#pub(super) struct TerrainMeshWorker",),
            ("crates/bbb-native/src/terrain_runtime/mesh_worker.rs#worker_builds_from_immutable_values_and_joins_on_drop",),
        ),
        ("getSectionRenderDispatcher",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#renderer_sections",),
            ("crates/bbb-native/src/terrain_runtime.rs#ordered_frame_projection_refreshes_fade_and_consumer_gate_together",),
        ),
        ("getVisibleSections",),
    ),
)

LEVEL_RENDERER_BACKEND_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/renderer.rs#pub struct Renderer",
                "crates/bbb-renderer/src/renderer.rs#pub async fn new",
            ),
            ("crates/bbb-renderer/src/renderer.rs#renderer_backend_gpu_owners_share_one_struct_raii_lifetime",),
        ),
        ("LevelRenderer", "close"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/resource_reload.rs#pub fn transactional_shader_resource_reload",),
            ("crates/bbb-renderer/src/resource_reload.rs#local_shader_resource_generation_commits_and_rolls_back_as_one_owner",),
        ),
        ("onResourceManagerReload",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/outline.rs#create_entity_outline_target",),
            ("crates/bbb-renderer/src/post_effect_graph/tests.rs#selected_generation_drives_live_entity_outline_graph_before_composite",),
        ),
        ("initOutline",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#synchronize_terrain_level_generation",),
            ("crates/bbb-native/src/terrain_runtime.rs#level_generation_change_drops_old_residency_without_reusing_revision_ids",),
        ),
        ("setLevel",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#request_reload_all_chunks",),
            ("crates/bbb-native/src/terrain_runtime.rs#terrain_reload_request_marks_all_cached_chunks_for_rebuild",),
        ),
        ("allChanged",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/renderer.rs#fn prepare_resize",),
            ("crates/bbb-renderer/src/renderer.rs#resize_prepare_failures_keep_the_previous_complete_backend_generation",),
        ),
        ("resize",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/renderer.rs#upload_terrain_texture_atlas_mips",),
            ("crates/bbb-renderer/src/resource_reload.rs#injected_failure_after_each_reload_stage_restores_exact_accepted_owners",),
        ),
        ("resetSampler",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#terrain_frustum_visibility_key",),
            ("crates/bbb-native/src/terrain_runtime.rs#frame_visibility_matches_vanilla_two_degree_rotation_gate",),
        ),
        ("needsUpdate",),
    ),
)

LEVEL_RENDERER_SCENE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/entity_scene.rs#entity_scene_outline_from_world_at_partial_tick",
                "crates/bbb-renderer/src/render.rs#entity_outline_target_pass",
            ),
            (
                "crates/bbb-native/src/entity_scene/tests.rs#entity_scene_outline_projects_pick_bounds_for_all_visible_targets",
                "crates/bbb-renderer/src/render.rs#entity_outline_target_stays_in_main_pass_and_post_chain_runs_before_clouds",
            ),
        ),
        ("doEntityOutline", "shouldShowEntityOutlines", "entityOutlineTarget"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#update_renderer_from_publication",
                "crates/bbb-native/src/runtime/render_extract.rs#apply_renderer_frame",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#renderer_frame_crosshair_feedback_and_outlines_share_post_input_camera_state",
                "crates/bbb-native/src/runtime/tests.rs#block_selection_has_one_production_projection_and_no_legacy_resolver_path",
            ),
        ),
        ("extractLevel", "checkPoseStack"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/entity_scene.rs#visible_entity_model_instances_from_world_with_variant_textures_at_partial_tick",),
            ("crates/bbb-native/src/entity_scene/tests.rs#visible_entity_source_orders_dispatcher_before_current_section_gate",),
        ),
        ("extractVisibleEntities", "extractEntity"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/render_extract.rs#apply_renderer_frame",
                "crates/bbb-renderer/src/render.rs#main_world_pass",
            ),
            ("crates/bbb-renderer/src/render.rs#entity_translucent_features_draw_after_depth_copies_and_before_translucent_terrain",),
        ),
        ("submitEntities",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/block_entity_visibility.rs#retain_visible_block_entity_instances",),
            (
                "crates/bbb-native/src/block_entity_visibility.rs#compiled_section_gate_precedes_camera_frustum_submission",
                "crates/bbb-native/src/block_entity_visibility.rs#ordinary_models_use_the_containing_section_as_the_frustum_unit",
            ),
        ),
        ("extractVisibleBlockEntities",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/render_extract.rs#apply_renderer_frame",
                "crates/bbb-renderer/src/render.rs#main_world_pass",
            ),
            ("crates/bbb-renderer/src/render.rs#lit_entity_draws_sample_dynamic_lightmap_texture",),
        ),
        ("submitBlockEntities",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#block_destroy_overlays_from_stages",
                "crates/bbb-native/src/runtime.rs#apply_block_entity_crumbling",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#block_destroy_overlays_use_camera_eye_and_vanilla_thirty_two_block_center_bound",
                "crates/bbb-native/src/runtime/tests.rs#block_destroy_overlays_include_server_progress_and_keep_highest_per_position",
            ),
        ),
        ("extractBlockDestroyAnimation",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/render.rs#block_destroy_overlay_pass",),
            ("crates/bbb-renderer/src/render.rs#block_destroy_overlays_draw_in_translucent_feature_phase_before_translucent_terrain",),
        ),
        ("submitBlockDestroyAnimation",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/crosshair.rs#block_selection_from_camera",
                "crates/bbb-native/src/runtime.rs#block_selection_render_state",
            ),
            (
                "crates/bbb-native/src/crosshair.rs#block_selection_rejects_target_outside_world_border",
                "crates/bbb-native/src/runtime/tests.rs#block_selection_projection_uses_current_material_catalog_surface_width_and_style",
            ),
        ),
        ("extractBlockOutline",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/render.rs#block_selection_before_translucent_pass",
                "crates/bbb-renderer/src/render.rs#item_entity_line_target_pass",
                "crates/bbb-renderer/src/selection.rs#create_block_selection_gpu",
            ),
            (
                "crates/bbb-renderer/src/render.rs#block_selection_phase_brackets_translucent_terrain_and_has_one_draw_lane",
                "crates/bbb-renderer/src/offscreen.rs#offscreen_block_selection_renders_both_material_phase_paths",
                "crates/bbb-renderer/src/selection.rs#high_contrast_block_selection_emits_secondary_before_cyan_primary",
            ),
        ),
        ("renderBlockOutline", "renderHitOutline"),
    ),
)

LEVEL_RENDERER_EFFECT_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/weather_effect_runtime.rs#advance_for_ticks",
                "crates/bbb-world/src/block_events.rs#advance_block_destruction_render_ticks",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#weather_tick_effects_preserve_level_renderer_order_before_later_effect_drains",
                "crates/bbb-native/src/runtime/tests.rs#block_destroy_render_ticks_respect_frozen_world_ticking_state",
            ),
        ),
        ("tick",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/block_events.rs#advance_block_destruction_render_ticks",),
            (
                "crates/bbb-world/src/block_events.rs#block_destruction_progress_expires_after_vanilla_render_tick_window",
                "crates/bbb-world/src/block_events.rs#block_destruction_batched_tick_advance_expires_at_scan_tick",
            ),
        ),
        ("removeBlockBreakingProgress",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/block_events.rs#apply_block_destruction",),
            ("crates/bbb-world/src/block_events.rs#tracks_block_destruction_progress_by_id",),
        ),
        ("removeProgress",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/block_events.rs#apply_block_destruction",
                "crates/bbb-world/src/block_events.rs#block_destruction_render_stages",
            ),
            (
                "crates/bbb-world/src/block_events.rs#block_destruction_render_stages_merge_server_and_local_progress",
                "crates/bbb-world/src/block_events.rs#block_destruction_budget_rejects_only_new_ids",
                "crates/bbb-native/src/runtime/tests.rs#block_destroy_stage_projection_has_one_world_owner_and_no_native_resolver",
            ),
        ),
        ("destroyBlockProgress",),
    ),
)

LEVEL_RENDERER_DIAGNOSTICS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/entity_scene.rs#visible_entity_render_state_count",
                "crates/bbb-native/src/runtime.rs#hud_debug_entity_render_stats_line",
            ),
            (
                "crates/bbb-native/src/entity_scene/tests.rs#visible_entity_render_state_count_reads_the_current_survivor_owner",
                "crates/bbb-native/src/runtime/tests.rs#hud_debug_overlay_projects_current_frame_entity_render_stats_under_reduced_debug_info",
                "crates/bbb-native/src/runtime/tests.rs#level_diagnostics_have_one_current_frame_collection_and_finalize_path",
            ),
        ),
        ("getEntityStatistics",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/render_extract.rs#begin_debug_frame",
                "crates/bbb-native/src/runtime.rs#update_renderer_from_publication",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#dynamic_debug_buffers_retain_capacity_and_count_only_growth_edges",
                "crates/bbb-native/src/runtime/tests.rs#level_diagnostics_have_one_current_frame_collection_and_finalize_path",
            ),
        ),
        ("collectPerFrameGizmos",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/render_extract.rs#apply_renderer_frame",
                "crates/bbb-renderer/src/render.rs#debug_filled_box_pass",
                "crates/bbb-renderer/src/render.rs#debug_always_on_top_text_pass",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#renderer_frame_reuses_dynamic_debug_buffers_and_keeps_stable_game_test_lane",
                "crates/bbb-renderer/src/render.rs#debug_gizmo_passes_run_after_translucent_features_before_translucent_terrain",
                "crates/bbb-renderer/src/render.rs#always_on_top_debug_text_runs_after_composition_with_cleared_depth",
            ),
        ),
        ("finalizeGizmoCollection",),
    ),
)

LEVEL_RENDERER_LIGHT_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-render-types/src/light_coords.rs#from_stored",
                "crates/bbb-world/src/chunks/store.rs#sample_render_light",
                "crates/bbb-world/src/chunks/store.rs#sample_render_light_for_state",
            ),
            (
                "crates/bbb-render-types/src/light_coords.rs#state_override_order_matches_level_renderer",
                "crates/bbb-world/src/chunks/tests.rs#samples_block_light_at_world_position_and_reports_unloaded",
                "crates/bbb-renderer/src/terrain/mesh/tests.rs#cubic_face_offset_samples_neighbor_brightness_with_current_state",
                "crates/bbb-native/src/runtime/tests.rs#level_renderer_light_consumers_cannot_restore_raw_pack_resolvers",
            ),
        ),
        ("getLightCoords",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-render-types/src/light_coords.rs#packed",
                "crates/bbb-world/src/chunks/store.rs#sample_render_light_for_state",
            ),
            (
                "crates/bbb-render-types/src/light_coords.rs#component_max_pack_and_normalize_share_one_projection",
                "crates/bbb-native/src/runtime/tests.rs#level_renderer_light_consumers_cannot_restore_raw_pack_resolvers",
            ),
        ),
        ("packedBrightness",),
    ),
)

LEVEL_RENDERER_OWNER_EVIDENCE = {
    "rendering-level-backend-lifecycle": LEVEL_RENDERER_BACKEND_OWNER_EVIDENCE,
    "rendering-level-diagnostics-gizmos": LEVEL_RENDERER_DIAGNOSTICS_OWNER_EVIDENCE,
    "rendering-level-effect-lifecycle": LEVEL_RENDERER_EFFECT_OWNER_EVIDENCE,
    "rendering-level-frame-graph": LEVEL_RENDERER_FRAME_OWNER_EVIDENCE,
    "rendering-level-light-query": LEVEL_RENDERER_LIGHT_OWNER_EVIDENCE,
    "rendering-level-scene-projection": LEVEL_RENDERER_SCENE_OWNER_EVIDENCE,
    "rendering-level-section-lifecycle": LEVEL_RENDERER_SECTION_OWNER_EVIDENCE,
}

INTERACTION_BLOCK_BREAKING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/chunks/store.rs#predict_local_destroy_block",
                "crates/bbb-world/src/client/local_player_destroy.rs#local_selected_main_hand_can_destroy_block",
                "crates/bbb-world/src/client/local_player_destroy.rs#local_destroy_block_gamemaster_gate_allows",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#survival_instant_left_mouse_press_predicts_destroy_without_delay",
                "crates/bbb-native/src/input/mouse.rs#creative_debug_stick_sends_start_without_local_destroy_prediction",
                "crates/bbb-native/src/input/mouse.rs#creative_game_master_prediction_requires_permission_but_start_still_sends",
                "crates/bbb-protocol/src/item_types.rs#destroy_block_consumer_uses_generated_direct_deny_catalog",
                "crates/bbb-protocol/src/block_types.rs#block_destroy_consumer_uses_generated_game_master_membership",
            ),
        ),
        ("destroyBlock",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/mouse.rs#start_destroy_block",
                "crates/bbb-native/src/input/mouse.rs#apply_start_destroy_block",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#left_mouse_press_on_block_queues_start_destroy_and_swing",
                "crates/bbb-native/src/input/mouse.rs#repeated_survival_press_on_same_target_keeps_prediction_and_only_swings_again",
                "crates/bbb-native/src/input/mouse.rs#full_or_closed_start_queue_keeps_prediction_and_destroy_state_unchanged",
            ),
        ),
        ("startDestroyBlock",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input/mouse.rs#abort_destroy_block",),
            (
                "crates/bbb-native/src/input/mouse.rs#left_mouse_release_aborts_destroying_block",
                "crates/bbb-native/src/input/mouse.rs#completion_and_abort_retry_without_publishing_partial_state_when_queue_is_full",
                "crates/bbb-native/src/input/tests.rs#focus_loss_retries_destroy_abort_without_clearing_state_when_queue_is_full",
            ),
        ),
        ("stopDestroyBlock",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input/mouse.rs#continue_destroy_block_tick",),
            (
                "crates/bbb-native/src/input/mouse.rs#survival_same_target_continues_after_world_border_moves_past_block",
                "crates/bbb-native/src/input/mouse.rs#held_left_mouse_same_target_finishes_destroy_with_stop",
            ),
        ),
        ("continueDestroyBlock",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#local_destroying_block_matches_current_item",
            ),
            (
                "crates/bbb-world/src/client/local_player_destroy.rs#local_destroy_target_item_signature_follows_vanilla_same_item_same_components",
            ),
        ),
        ("sameDestroyTarget",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/local_player_destroy.rs#local_destroy_stage",),
            (
                "crates/bbb-world/src/client/local_player_destroy.rs#local_destroy_stage_tracks_active_progress_range",
            ),
        ),
        ("getDestroyStage",),
    ),
)

INTERACTION_PREDICTION_SEQUENCE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/block_events.rs#next_local_prediction_sequence",
                "crates/bbb-world/src/block_events.rs#apply_block_changed_ack",
                "crates/bbb-world/src/chunks/store.rs#sync_ended_local_block_predictions",
                "crates/bbb-native/src/input/mouse.rs#start_use_item_with_effects",
                "crates/bbb-native/src/input/commands.rs#send_reserved_use_item_command",
                "crates/bbb-native/src/input/commands.rs#send_reserved_use_item_on_command",
            ),
            (
                "crates/bbb-world/src/block_events.rs#prediction_sequence_uses_java_signed_wrap_and_ack_order",
                "crates/bbb-world/src/chunks/tests.rs#one_prediction_sequence_tracks_each_changed_position",
                "crates/bbb-world/src/chunks/tests.rs#teleport_prevents_older_prediction_rejection_from_moving_player_back",
                "crates/bbb-world/src/level.rs#respawn_updates_dimension_and_clears_old_chunks",
                "crates/bbb-native/src/input/mouse.rs#unavailable_item_use_command_keeps_sequence_and_world_unchanged",
                "crates/bbb-native/src/input/mouse.rs#unavailable_block_use_command_keeps_prediction_and_item_unchanged",
                "crates/bbb-native/src/input/mouse.rs#unavailable_block_use_command_does_not_open_prediction_owned_editor",
                "crates/bbb-native/src/input/mouse.rs#admitted_block_use_does_not_start_unadmitted_item_fallback",
                "crates/bbb-native/src/input/commands.rs#prediction_commands_cannot_restore_unreserved_queue_helpers",
            ),
        ),
        ("startPrediction",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/block_events.rs#apply_block_changed_ack",
                "crates/bbb-world/src/chunks/store.rs#sync_ended_local_block_predictions",
            ),
            (
                "crates/bbb-world/src/block_events.rs#prediction_sequence_uses_java_signed_wrap_and_ack_order",
                "crates/bbb-world/src/chunks/tests.rs#teleport_prevents_older_prediction_rejection_from_moving_player_back",
            ),
        ),
        ("handleBlockChangedAck",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/chunks/store.rs#sync_ended_local_block_predictions",
                "crates/bbb-world/src/chunks/store.rs#set_block_state_id",
            ),
            (
                "crates/bbb-world/src/chunks/tests.rs#one_prediction_sequence_tracks_each_changed_position",
                "crates/bbb-world/src/chunks/tests.rs#teleport_prevents_older_prediction_rejection_from_moving_player_back",
            ),
        ),
        ("setServerVerifiedBlockState", "syncBlockState"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/chunks/store.rs#predict_local_block_state",
                "crates/bbb-world/src/chunks/store.rs#predict_local_block_states",
            ),
            (
                "crates/bbb-world/src/chunks/tests.rs#one_prediction_sequence_tracks_each_changed_position",
                "crates/bbb-native/src/input/mouse.rs#unavailable_block_use_command_keeps_prediction_and_item_unchanged",
            ),
        ),
        ("setBlock",),
    ),
)

CLIENT_LEVEL_ENTITY_LIFECYCLE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities.rs#entity_model_sources_at_partial_tick",
                "crates/bbb-world/src/entities.rs#entity_count",
                "crates/bbb-world/src/entities.rs#probe_entity",
                "crates/bbb-world/src/entities/store.rs#states",
            ),
            (
                "crates/bbb-world/src/entities/tests/lifecycle_and_queries.rs#tracks_entity_lifecycle_and_absolute_state_updates",
                "crates/bbb-world/src/entities/tests/lifecycle_and_queries.rs#partial_tick_queries_share_canonical_transform_render_endpoints",
                "crates/bbb-world/src/entities/tests/creature_pick_bounds_projection.rs#ender_dragon_pick_targets_use_vanilla_part_ids_and_bounds",
            ),
        ),
        ("entitiesForRendering", "getEntityCount", "getEntity", "getEntities"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/animations.rs#advance_entity_client_animation_tick",
                "crates/bbb-world/src/entities/store.rs#fill_client_tick_schedule",
                "crates/bbb-world/src/entities/store.rs#apply_local_player_push_impulse",
                "crates/bbb-world/src/entities/store.rs#within_death_tick_distance",
            ),
            (
                "crates/bbb-world/src/entities/tests/passenger_and_vehicle_movement.rs#client_tick_schedule_follows_loaded_roots_and_recursive_player_exception",
                "crates/bbb-world/src/entities/tests/passenger_and_vehicle_movement.rs#frozen_client_tick_schedule_keeps_player_roots_and_player_passenger_trees_running",
                "crates/bbb-world/src/entities/tests/passenger_and_vehicle_movement.rs#ordinary_living_tick_uses_scaled_local_bounds_and_one_immutable_push_generation",
                "crates/bbb-world/src/entities/tests/passenger_and_vehicle_movement.rs#ordinary_living_push_rejects_same_vehicle_and_spectator_local_player",
                "crates/bbb-world/src/entities/tests/combat_state_and_mob_flag_projection.rs#simulation_distance_only_gates_living_death_ticks_by_chunk_distance",
            ),
        ),
        (
            "tickEntities",
            "isTickingEntity",
            "shouldTickDeath",
            "tickNonPassenger",
            "tickPassenger",
            "getPushableEntities",
            "onTickingStart",
            "onTickingEnd",
        ),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/spawn.rs#apply_add_entity_packet",
                "crates/bbb-world/src/entities.rs#apply_remove_entities",
                "crates/bbb-world/src/entities/store.rs#insert_or_replace",
                "crates/bbb-world/src/entities/store.rs#remove_ids",
            ),
            (
                "crates/bbb-world/src/entities/tests/lifecycle_and_queries.rs#tracks_entity_lifecycle_and_absolute_state_updates",
                "crates/bbb-world/src/entities/tests/lifecycle_and_queries.rs#level_reset_and_id_reuse_restore_generated_attribute_defaults",
                "crates/bbb-world/src/entities/tests/lifecycle_and_queries.rs#turtle_pairing_state_uses_generated_defaults_and_resets_on_readd",
                "crates/bbb-world/src/entities/tests/passenger_and_vehicle_movement.rs#removed_player_vehicle_teleport_recovers_the_local_player",
            ),
        ),
        (
            "addEntity",
            "removeEntity",
            "onTrackingStart",
            "onTrackingEnd",
        ),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/store.rs#within_death_tick_distance",
                "crates/bbb-world/src/entities/animations.rs#advance_entity_client_animation_tick",
            ),
            (
                "crates/bbb-world/src/entities/tests/combat_state_and_mob_flag_projection.rs#simulation_distance_only_gates_living_death_ticks_by_chunk_distance",
            ),
        ),
        ("setServerSimulationDistance",),
    ),
)

CLIENT_LEVEL_IDENTITY_LIFECYCLE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#apply_login",
                "crates/bbb-world/src/level.rs#apply_respawn",
                "crates/bbb-world/src/level.rs#clear_client_level",
            ),
            (
                "crates/bbb-world/src/level.rs#client_level_generation_follows_construct_replace_and_clear_identity_only",
                "crates/bbb-world/src/level.rs#client_level_generation_production_source_has_one_mutator_and_exact_edges",
                "crates/bbb-native/src/level_load_runtime.rs#same_dimension_respawn_restarts_load_timeout_without_replacing_level_identity",
            ),
        ),
        ("ClientLevel",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/server_presentation.rs#end_connection_session",
                "crates/bbb-native/src/runtime.rs#advance_simulation",
            ),
            (
                "crates/bbb-world/src/client/server_presentation.rs#terminal_connection_session_drops_transfer_only_carry",
                "crates/bbb-native/src/runtime/tests.rs#remote_disconnect_publishes_terminal_screen_before_input_without_exit_fallback",
            ),
        ),
        ("disconnect",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#client_level_generation",
                "crates/bbb-native/src/terrain_runtime.rs#synchronize_terrain_level_generation",
                "crates/bbb-native/src/particle_runtime/explosion.rs#track",
                "crates/bbb-native/src/runtime/render_extract.rs#synchronize_map_texture_level",
            ),
            (
                "crates/bbb-native/src/terrain_runtime.rs#level_generation_change_drops_old_residency_without_reusing_revision_ids",
                "crates/bbb-native/src/particle_runtime/tests.rs#explosion_tracker_drops_pending_work_across_level_generations",
                "crates/bbb-native/src/runtime/tests.rs#map_texture_cache_invalidates_only_on_level_generation_edge",
                "crates/bbb-native/src/level_load_runtime.rs#respawn_local_consumers_cannot_restore_level_generation_as_their_only_identity",
            ),
        ),
        ("registerForCleaning",),
    ),
)

CLIENT_LEVEL_CHUNK_LIGHT_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/chunks/store.rs#insert_level_chunk_with_light",
                "crates/bbb-world/src/chunks/store.rs#apply_biome_update",
                "crates/bbb-world/src/entities/movement.rs#entity_is_ticking",
            ),
            (
                "crates/bbb-world/src/chunks/tests.rs#full_chunk_biome_reload_and_unload_invalidate_full_neighbor_height",
                "crates/bbb-world/src/chunks/tests.rs#pending_initial_light_lifecycle_follows_load_apply_prune_and_direct_insert",
            ),
        ),
        ("onChunkLoaded",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/chunks/store.rs#forget_chunk",
                "crates/bbb-world/src/chunks/store.rs#discard_queued_light_updates_for_chunk",
            ),
            (
                "crates/bbb-world/src/chunks/tests.rs#forget_and_level_clear_discard_pending_light_without_stale_application",
                "crates/bbb-world/src/chunks/tests.rs#light_and_chunk_removal_invalidate_neighbor_meshes_without_fabricating_content",
            ),
        ),
        ("unload",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/chunks/store.rs#queue_light_update",
                "crates/bbb-world/src/chunks/store.rs#insert_level_chunk_with_light",
            ),
            (
                "crates/bbb-world/src/chunks/tests.rs#queued_light_updates_preserve_fifo_order_and_vanilla_poll_budget",
                "crates/bbb-world/src/chunks/tests.rs#light_validation_rejects_wrong_dimension_domains_before_mutation",
            ),
        ),
        ("queueLightUpdate",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/chunks/store.rs#poll_queued_light_updates",
                "crates/bbb-native/src/terrain_runtime.rs#next_terrain_cache_work",
            ),
            (
                "crates/bbb-world/src/chunks/tests.rs#terrain_revision_tracks_view_selection_and_level_clear_lifecycle",
                "crates/bbb-native/src/terrain_runtime.rs#mesh_targets_withhold_a_chunk_until_its_initial_light_applies",
            ),
        ),
        ("pollLightUpdates",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#advance_simulation",),
            (
                "crates/bbb-native/src/simulation.rs#queued_light_poll_follows_the_single_network_drain_before_presentation_sync",
            ),
        ),
        ("update",),
    ),
)

CLIENT_LEVEL_DIRTY_HANDOFF_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/chunks/store.rs#set_block_state_id",
                "crates/bbb-world/src/chunks/store.rs#mark_terrain_block_changed",
            ),
            (
                "crates/bbb-world/src/chunks/tests.rs#identical_server_block_update_marks_nothing_like_vanilla_null_set",
                "crates/bbb-world/src/chunks/tests.rs#terrain_revisions_separate_chunk_content_from_neighbor_mesh_invalidation",
                "crates/bbb-native/src/terrain_runtime.rs#player_adjacent_dirty_chunk_schedules_before_a_distant_dirty_chunk",
            ),
        ),
        ("sendBlockUpdated", "setBlocksDirty"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/chunks/store.rs#mark_terrain_light_sections_changed",),
            (
                "crates/bbb-world/src/chunks/tests.rs#standalone_light_revisions_touch_only_masked_section_neighbors",
            ),
        ),
        ("setSectionDirtyWithNeighbors",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/chunks/store.rs#apply_light_data",
                "crates/bbb-world/src/chunks/store.rs#mark_terrain_chunk_and_neighbors_changed",
            ),
            (
                "crates/bbb-world/src/chunks/tests.rs#terrain_revision_tracks_view_selection_and_level_clear_lifecycle",
                "crates/bbb-world/src/chunks/tests.rs#full_chunk_biome_reload_and_unload_invalidate_full_neighbor_height",
            ),
        ),
        ("setSectionRangeDirty",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/block_events.rs#apply_block_destruction",
                "crates/bbb-world/src/block_events.rs#block_destruction_render_stages",
            ),
            (
                "crates/bbb-world/src/block_events.rs#tracks_block_destruction_progress_by_id",
                "crates/bbb-world/src/block_events.rs#block_destruction_render_stages_merge_server_and_local_progress",
                "crates/bbb-world/src/block_events.rs#block_destruction_progress_expires_after_vanilla_render_tick_window",
            ),
        ),
        ("destroyBlockProgress",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/terrain_runtime.rs#refresh_terrain_visibility",
                "crates/bbb-native/src/terrain_runtime/occlusion.rs#update_sections",
            ),
            (
                "crates/bbb-native/src/terrain_runtime/occlusion.rs#terrain_runtime_occlusion_partial_compile_propagates_without_full_rebuild",
                "crates/bbb-native/src/terrain_runtime/occlusion.rs#terrain_runtime_occlusion_loaded_empty_sections_propagate_but_skip_octree",
            ),
        ),
        ("onSectionBecomingNonEmpty",),
    ),
)

CLIENT_LEVEL_CLOCK_WEATHER_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/animations.rs#advance_client_level_entity_ticks_with_observation",
                "crates/bbb-world/src/level.rs#consume_running_render_ticks",
            ),
            (
                "crates/bbb-world/src/level.rs#frozen_ticking_gates_production_time_advance_but_not_set_time_resync",
                "crates/bbb-world/src/level.rs#consume_running_render_ticks_respects_frozen_tick_steps",
            ),
        ),
        ("tick",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/level.rs#advance_client_time",),
            (
                "crates/bbb-world/src/level.rs#advance_client_time_ticks_game_time_and_clock_instances_like_vanilla",
            ),
        ),
        ("tickTime",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/level.rs#apply_world_time",),
            (
                "crates/bbb-world/src/level.rs#world_time_weather_and_ticking_are_canonical_state",
                "crates/bbb-world/src/level.rs#frozen_ticking_gates_production_time_advance_but_not_set_time_resync",
            ),
        ),
        ("setTimeFromServer", "setGameTime"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#apply_world_time",
                "crates/bbb-world/src/level.rs#advance_client_time",
            ),
            (
                "crates/bbb-world/src/level.rs#world_time_merges_partial_clock_updates_like_client_clock_manager",
                "crates/bbb-world/src/level.rs#advance_client_time_preserves_paused_clock_updates",
                "crates/bbb-world/src/level.rs#world_time_selects_dimension_default_clock_by_session_holder_identity",
            ),
        ),
        ("clockManager",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime/render_extract.rs#clear_color_with_sky_flash",),
            (
                "crates/bbb-native/src/runtime/tests.rs#lightmap_environment_hides_client_sky_flash_when_option_is_enabled",
                "crates/bbb-native/src/runtime/tests.rs#clear_color_hides_client_sky_flash_when_option_is_enabled",
            ),
        ),
        ("getSkyFlashTime",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#set_sky_flash_time",
                "crates/bbb-world/src/level.rs#trigger_sky_flash",
            ),
            (
                "crates/bbb-world/src/level.rs#sky_flash_time_decrements_on_client_ticks_and_clears_with_level",
            ),
        ),
        ("setSkyFlashTime",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#entity_shadow_sky_darken",
                "crates/bbb-native/src/runtime/render_extract.rs#clear_color_with_sky_flash",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#lightmap_environment_applies_client_sky_flash_layer_after_timeline",
                "crates/bbb-native/src/runtime/tests.rs#sky_environment_applies_client_sky_flash_layer_and_dimension_gate",
            ),
        ),
        ("addEnvironmentAttributeLayers",),
    ),
)

CLIENT_LEVEL_AMBIENT_ANIMATION_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-native/src/environment_animate_runtime.rs#tick",),
            (
                "crates/bbb-native/src/environment_animate_runtime.rs#animate_tick_runs_exactly_667_near_far_pairs",
                "crates/bbb-native/src/environment_animate_runtime.rs#ambient_particles_observe_chunks_biomes_replacement_on_the_next_tick",
                "crates/bbb-native/src/environment_animate_runtime.rs#environment_animate_ticks_after_movement_and_before_music_and_particle_intake",
            ),
        ),
        ("animateTick",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/environment_animate_runtime.rs#do_animate_tick",),
            (
                "crates/bbb-native/src/environment_animate_runtime.rs#environment_dispatches_end_rod_animate_effect",
                "crates/bbb-native/src/environment_animate_runtime.rs#full_block_collision_suppresses_ambient_particles",
                "crates/bbb-native/src/environment_animate_runtime.rs#marker_target_carries_the_sampled_state_and_ignores_non_matching_blocks",
            ),
        ),
        ("doAnimateTick",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/inventory.rs#local_marker_particle_target_block",),
            (
                "crates/bbb-world/src/client/inventory/tests.rs#marker_particle_target_requires_creative_and_main_hand_barrier_or_light",
                "crates/bbb-native/src/environment_animate_runtime.rs#marker_target_spawns_centered_block_marker_even_for_a_full_collision_block",
            ),
        ),
        ("getMarkerParticleTarget",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/environment_animate_runtime.rs#append_fluid_drip_particle",),
            (
                "crates/bbb-native/src/environment_animate_runtime.rs#fluid_drips_use_vanilla_collision_branches_tags_and_level_random_order",
            ),
        ),
        ("trySpawnDripParticles", "spawnParticle", "spawnFluidParticle"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/particle_runtime/resolver_block_events.rs#destroy_block_particle_batch",
            ),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#level_event_destroy_block_particles_use_block_particle_options",
            ),
        ),
        ("addDestroyBlockEffect",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/particle_runtime/resolver_player_actions.rs#local_destroy_hit_particle_state",
                "crates/bbb-native/src/particle_runtime/resolver_player_actions.rs#local_destroy_hit_particle_batch",
            ),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#local_destroy_hit_particle_uses_vanilla_shape_face_power_and_scale",
            ),
        ),
        ("addBreakingBlockEffect",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/particle_runtime/resolver_firework.rs#firework_explosion_particle_batch",
                "crates/bbb-native/src/particle_runtime/resolver_firework.rs#append_firework_explosion_sparks",
            ),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#level_event_particles_map_vanilla_simple_side_effects",
            ),
        ),
        ("createFireworks",),
    ),
)

CLIENT_LEVEL_RESPAWN_DATA_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/local_player.rs#apply_default_spawn_position",),
            (
                "crates/bbb-world/src/chunks/tests.rs#out_of_border_default_spawn_falls_back_to_the_border_center_heightmap_position",
            ),
        ),
        ("setRespawnData", "setSpawn"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/local_player.rs#local_player",),
            (
                "crates/bbb-world/src/level.rs#respawn_updates_dimension_and_clears_old_chunks",
                "crates/bbb-native/src/runtime/events/client_state/tests.rs#player_abilities_spawn_distance_and_chat_update_world_counters",
            ),
        ),
        ("getRespawnData",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#apply_login",
                "crates/bbb-world/src/level.rs#apply_spawn_info",
            ),
            (
                "crates/bbb-world/src/level.rs#apply_login_records_the_hardcore_flag",
                "crates/bbb-world/src/level.rs#horizon_height_and_void_darkness_range_follow_the_flat_level_flag",
            ),
        ),
        ("ClientLevelData",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/hud.rs#apply_change_difficulty",),
            (
                "crates/bbb-world/src/client/hud.rs#change_difficulty_updates_client_level_data_state",
            ),
        ),
        ("setDifficulty", "setDifficultyLocked"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#client_horizon_height",
                "crates/bbb-world/src/level.rs#local_player_eye_below_horizon",
            ),
            (
                "crates/bbb-world/src/level.rs#dark_disc_gate_compares_the_player_eye_against_the_horizon_height",
                "crates/bbb-native/src/runtime/tests.rs#sky_environment_renders_the_dark_disc_only_below_the_horizon",
                "crates/bbb-renderer/src/sky.rs#dark_sky_disc_vertices_mirror_the_bottom_disc_with_the_baked_translate",
            ),
        ),
        ("getHorizonHeight",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#client_void_darkness_onset_range",
                "crates/bbb-native/src/runtime/render_extract.rs#apply_camera_mob_effect_fog_color",
            ),
            (
                "crates/bbb-world/src/level.rs#horizon_height_and_void_darkness_range_follow_the_flat_level_flag",
                "crates/bbb-native/src/runtime/tests.rs#void_darkness_below_the_level_floor_darkens_the_fog_color",
            ),
        ),
        ("voidDarknessOnsetRange",),
    ),
)

CLIENT_LEVEL_SESSION_SERVICES_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-net/src/connection.rs#send_packet",),
            (
                "crates/bbb-net/src/driver.rs#drive_connection_sends_movement_net_commands_in_play",
                "crates/bbb-world/src/level.rs#client_world_owns_no_scheduled_tick_simulation_or_socket",
            ),
        ),
        ("sendPacketToServer",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/recipes.rs#apply_update_recipes",
                "crates/bbb-world/src/recipes.rs#reset_recipes_for_new_listener",
            ),
            (
                "crates/bbb-world/src/level.rs#connection_services_survive_respawn_and_reset_only_with_the_listener",
            ),
        ),
        ("recipeAccess",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/chunks/store.rs#set_block_state_id",),
            (
                "crates/bbb-world/src/level.rs#client_world_owns_no_scheduled_tick_simulation_or_socket",
            ),
        ),
        ("getBlockTicks", "getFluidTicks"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/scoreboard.rs#apply_set_objective",
                "crates/bbb-world/src/level.rs#clear_client_level",
            ),
            (
                "crates/bbb-world/src/level.rs#connection_services_survive_respawn_and_reset_only_with_the_listener",
            ),
        ),
        ("getScoreboard",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/features.rs#is_feature_enabled",),
            (
                "crates/bbb-world/src/level.rs#connection_services_survive_respawn_and_reset_only_with_the_listener",
            ),
        ),
        ("enabledFeatures",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/item_profiles.rs#set_brewing_potion_item_ids",
                "crates/bbb-native/src/resource_reload.rs#apply",
            ),
            (
                "crates/bbb-world/src/level.rs#connection_services_survive_respawn_and_reset_only_with_the_listener",
                "crates/bbb-world/src/client/inventory/tests.rs#apply_local_brewing_stand_pickup_all_collects_matching_menu_and_player_stacks",
            ),
        ),
        ("potionBrewing",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/item_profiles.rs#set_furnace_fuel_item_ids",
                "crates/bbb-world/src/client/inventory.rs#furnace_fuel_item_ids",
            ),
            (
                "crates/bbb-world/src/level.rs#connection_services_survive_respawn_and_reset_only_with_the_listener",
                "crates/bbb-world/src/client/inventory/tests.rs#local_container_slot_access_matches_specialized_input_predicates",
            ),
        ),
        ("fuelValues",),
    ),
)

CLIENT_LEVEL_BIOME_TINT_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/biome_tint.rs#build_biome_blend",
                "crates/bbb-native/src/terrain_runtime.rs#build_chunk_biome_blend",
            ),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#terrain_particle_biome_blend_is_centered_at_each_spawn_position",
                "crates/bbb-native/src/particle_runtime/tests.rs#local_destroy_hit_particle_uses_captured_biome_blend",
            ),
        ),
        ("calculateBlockTint",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/particle_runtime/environment.rs#environment_particle_color",
                "crates/bbb-native/src/terrain_runtime/textures.rs#terrain_tint_particle_rgb",),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#terrain_particle_commands_use_installed_block_tint_colors",
                "crates/bbb-native/src/particle_runtime/tests.rs#falling_dust_block_tint_uses_biome_blend_at_spawn_position",
            ),
        ),
        ("getBlockTint",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime.rs#next_terrain_cache_work",),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#terrain_particle_tint_snapshot_accepts_session_biome_remap",
            ),
        ),
        ("clearTintCaches",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/chunks/store.rs#biome_id_at",),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#falling_dust_tint_samples_biome_foliage_color_at_spawn_position",
            ),
        ),
        ("getUncachedNoiseBiome",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#cardinal_lighting",
                "crates/bbb-native/src/terrain_runtime.rs#terrain_cardinal_lighting",
            ),
            (
                "crates/bbb-renderer/src/terrain/mesh/tests.rs#terrain_diffuse_uses_vanilla_nether_cardinal_shade",
                "crates/bbb-native/src/terrain_runtime/textures/tests.rs#moving_block_quads_use_start_seed_and_cardinal_context",
            ),
        ),
        ("cardinalLighting",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/terrain_runtime/textures.rs#client_leaf_tint_color",),
            (
                "crates/bbb-native/src/environment_animate_runtime.rs#leaf_particles_preserve_untinted_and_fixed_color_profiles",
                "crates/bbb-native/src/particle_runtime/tests.rs#tinted_leaves_particle_options_decode_argb_color_into_spawn_command",
            ),
        ),
        ("getClientLeafTintColor",),
    ),
)

CLIENT_LEVEL_SOUND_ROUTING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/audio.rs#apply_sound_event",
                "crates/bbb-world/src/client/audio.rs#apply_sound_entity_event",
            ),
            (
                "crates/bbb-world/src/client/audio.rs#minecart_and_bee_tickable_sounds_match_client_instance_math_and_generation_isolation",
                "crates/bbb-native/src/runtime/events/tests/audio_and_death_sounds.rs#remote_armor_stand_death_event_waits_for_server_owned_sound_packet",
            ),
        ),
        ("playSeededSound",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/audio.rs#guardian_attack_sound_for_entity",
                "crates/bbb-native/src/environment_animate_runtime.rs#local_block_sound",
            ),
            (
                "crates/bbb-world/src/client/audio.rs#entity_client_movement_sounds_use_canonical_seed_and_vanilla_categories",
                "crates/bbb-native/src/environment_animate_runtime.rs#local_block_sounds_preserve_position_source_and_shared_level_seed",
            ),
        ),
        ("playLocalSound",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/audio.rs#begin_local_player_ambient_audio_tick",),
            (
                "crates/bbb-world/src/client/audio.rs#hud_air_bubble_pop_sound_is_player_positioned_and_advances_the_sound_seed",
                "crates/bbb-world/src/client/audio.rs#sound_seed_random_is_deterministic_and_non_repeating",
            ),
        ),
        ("playPlayerSound",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-audio/src/runtime.rs#vanilla_distance_delay_ticks",),
            (
                "crates/bbb-audio/src/runtime.rs#distance_delay_ticks_match_vanilla_threshold_and_flooring",
                "crates/bbb-audio/src/runtime.rs#delayed_positioned_sounds_release_after_remaining_ticks",
            ),
        ),
        ("playSound",),
    ),
)

CLIENT_LEVEL_PARTICLE_ROUTING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/particle_runtime/resolver_core.rs#should_spawn_level_particle",
            ),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#level_particles_drop_non_override_spawns_beyond_vanilla_camera_distance",
                "crates/bbb-native/src/particle_runtime/tests.rs#level_particles_override_limiter_bypasses_distance_and_particle_status",
            ),
        ),
        ("doAddParticle",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/particle_runtime/resolver_core.rs#calculate_particle_level",
            ),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#decreased_particle_status_uses_vanilla_next_int_three",
                "crates/bbb-native/src/particle_runtime/tests.rs#minimal_particle_status_only_keeps_always_show_promoted_particles",
            ),
        ),
        ("calculateParticleLevel",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/particle_runtime/resolver_core.rs#resolve_level_particles",
                "crates/bbb-native/src/particle_runtime/environment.rs#environment_particle_batch",
            ),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#level_particles_override_limiter_bypasses_distance_and_particle_status",
                "crates/bbb-native/src/environment_animate_runtime.rs#bubble_column_animate_effects_preserve_vanilla_particles_and_random_order",
            ),
        ),
        ("addParticle", "addAlwaysVisibleParticle"),
    ),
)

LOCAL_PLAYER_INPUT_MOVEMENT_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player_movement.rs#integrate_local_player_input_pose_with_world_effects",
                "crates/bbb-world/src/client/local_player_movement.rs#local_player_modified_input",
                "crates/bbb-world/src/client/local_player_movement.rs#local_player_unit_square_input",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_input_stops_at_full_block_wall_and_reports_collision",
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_using_item_slows_input_without_stopping_an_active_sprint_latch",
            ),
        ),
        (
            "aiStep",
            "applyInput",
            "modifyInput",
            "modifyInputSpeedForSquareMovement",
            "isMoving",
        ),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player_movement.rs#update_local_player_auto_jump",
                "crates/bbb-world/src/client/local_player_movement.rs#can_local_player_auto_jump",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_auto_jump_arms_on_block_and_forces_next_tick_jump",
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_auto_jump_disabled_option_and_low_jump_factor_do_not_arm",
            ),
        ),
        ("move", "canAutoJump", "updateAutoJump"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player_movement.rs#local_player_can_start_sprinting",
                "crates/bbb-world/src/client/local_player_movement.rs#local_player_should_stop_sprinting",
                "crates/bbb-world/src/client/local_player_movement.rs#local_player_sprint_particle_state",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_shallow_water_stops_run_sprint_before_fluid_travel",
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_sprint_particle_state_applies_vanilla_gates_and_on_pos_facts",
                "crates/bbb-native/src/particle_runtime/tests.rs#local_sprint_particle_batch_clamps_into_on_pos_column_with_trailing_velocity",
            ),
        ),
        (
            "canStartSprinting",
            "isSprintingPossible",
            "shouldStopRunSprinting",
            "shouldStopSwimSprinting",
            "canSpawnSprintParticle",
        ),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player_movement.rs#move_local_player_towards_closest_space",
                "crates/bbb-world/src/client/local_player_collision.rs#local_player_suffocates_at",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_suffocation_uses_live_state_fact_and_collision_shape",
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_suffocation_escape_selects_closest_open_direction",
            ),
        ),
        ("suffocatesAt", "moveTowardsClosestSpace", "distanceToUnitSquare"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player_movement.rs#local_player_climbable_limited_velocity",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_sneak_suppresses_climbable_sliding",
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_sneak_uses_default_sneaking_speed_multiplier",
            ),
        ),
        ("isShiftKeyDown", "isMovingSlowly", "isSuppressingSlidingDownLadder"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player_movement.rs#local_player_horizontal_collision_is_minor",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_minor_horizontal_collision_angle_threshold_matches_vanilla",
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_minor_horizontal_collision_tracks_tangential_input_in_air_and_fluid",
            ),
        ),
        ("isHorizontalCollisionMinor",),
    ),
)

LOCAL_PLAYER_ENVIRONMENT_EFFECTS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/play_apply.rs#tracking_emitter_for_entity_animation",
                "crates/bbb-native/src/particle_runtime/resolver_entity_events.rs#tracking_emitter_particle_batch",
            ),
            (
                "crates/bbb-world/src/play_apply.rs#entity_animation_forwards_crit_tracking_emitters",
                "crates/bbb-native/src/particle_runtime/tests.rs#tracking_emitter_batch_spawns_totem_particles_around_entity_bounds",
            ),
        ),
        ("crit", "magicCrit"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/audio.rs#hud_air_bubble_pop_sound",
                "crates/bbb-world/src/client/audio.rs#record_positioned_sound",
            ),
            (
                "crates/bbb-world/src/client/audio.rs#hud_air_bubble_pop_sound_is_player_positioned_and_advances_the_sound_seed",
            ),
        ),
        ("playSound",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#advance_local_player_portal_transition_effect",
                "crates/bbb-world/src/client/local_player_collision.rs#local_player_inside_nether_portal",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_portal_transition_effect_tracks_vanilla_intensity_curve",
                "crates/bbb-native/src/runtime/tests.rs#camera_spinning_effect_blends_portal_intensity_like_vanilla",
            ),
        ),
        ("getActivePortalLocalTransition", "handlePortalTransitionEffect"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/environment_audio_runtime.rs#advance_mood",
            ),
            (
                "crates/bbb-native/src/environment_audio_runtime.rs#environment_audio_projects_vanilla_rounded_mood_percent",
            ),
        ),
        ("getCurrentMood",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#tick_water_vision",
                "crates/bbb-native/src/runtime.rs#water_vision",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#lightmap_tick_state_water_vision_fades_out_when_eye_leaves_water",
            ),
        ),
        ("getWaterVision",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/audio.rs#begin_local_player_ambient_audio_tick",
                "crates/bbb-native/src/environment_audio_runtime.rs#apply_underwater_tick",
            ),
            (
                "crates/bbb-native/src/environment_audio_runtime.rs#underwater_audio_plays_transition_sounds_and_fades_loop_at_vanilla_rates",
            ),
        ),
        ("updateIsUnderwater",),
    ),
)

LOCAL_PLAYER_WORLD_EDIT_UI_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/ui.rs#apply_open_sign_editor",
                "crates/bbb-native/src/input.rs#sync_sign_editor",
            ),
            (
                "crates/bbb-world/src/client/ui.rs#open_sign_editor_ignores_missing_sign_without_replacing_screen",
                "crates/bbb-native/src/input/tests.rs#sign_editor_tick_range_guard_submits_once",
            ),
        ),
        ("openTextEdit",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/book.rs#sync_book_screen",
            ),
            (
                "crates/bbb-native/src/input/book/tests.rs#book_screen_editor_applies_text_rules_and_done_sends_unsigned_submission",
            ),
        ),
        ("openItemGui",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#sync_command_block_editor",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#command_block_editor_waits_for_authoritative_data_and_done_sends_exact_state_once",
            ),
        ),
        ("openCommandBlock",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/minecart_command_block.rs#sync_minecart_command_block_editor",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#minecart_command_block_editor_preserves_unsent_command_and_done_sends_exact_state_once",
            ),
        ),
        ("openMinecartCommandBlock",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/structure_block.rs#sync_structure_block_editor",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#structure_block_editor_preloads_and_done_sends_clamped_exact_packet",
            ),
        ),
        ("openStructureBlock",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/jigsaw_block.rs#sync_jigsaw_block_editor",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#jigsaw_block_editor_preloads_and_done_sends_exact_set_once",
            ),
        ),
        ("openJigsawBlock",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/test_block.rs#sync_test_block_editor",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#test_block_editor_preserves_hidden_message_cycles_modes_and_done_sends_once",
            ),
        ),
        ("openTestBlock",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/test_instance_block.rs#sync_test_instance_block_editor",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#test_instance_block_editor_initializes_queries_and_applies_only_present_response_size",
            ),
        ),
        ("openTestInstanceBlock",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/ui.rs#apply_show_dialog",
                "crates/bbb-native/src/input.rs#sync_dialog_native_previous_screen",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#dialog_stack_suspends_and_restores_the_native_chat_screen",
            ),
        ),
        ("openDialog",),
    ),
)

LOCAL_PLAYER_CAMERA_TARGETING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/crosshair.rs#crosshair_target_from_world",
                "crates/bbb-native/src/crosshair.rs#active_crosshair_attack_range",
            ),
            (
                "crates/bbb-native/src/crosshair.rs#crosshair_target_uses_selected_item_attack_range_for_extended_entity_hit",
                "crates/bbb-native/src/crosshair.rs#crosshair_raycast_uses_vanilla_default_block_interaction_range",
                "crates/bbb-native/src/crosshair.rs#crosshair_target_skips_entity_beyond_vanilla_default_entity_interaction_range",
            ),
        ),
        ("pick", "filterHitResult", "raycastHitResult"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/camera_pose.rs#camera_pose_from_world",
            ),
            (
                "crates/bbb-native/src/camera_pose.rs#render_camera_uses_local_player_tick_endpoints",
            ),
        ),
        ("getViewXRot", "getViewYRot"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/leash.rs#rope_hold_position",
            ),
            (
                "crates/bbb-world/src/entities/leash.rs#flying_player_roll_uses_full_view_rotation_not_body_rotation",
            ),
        ),
        ("getVisualRotationYInDegrees", "getRopeHoldPosition"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#apply_set_camera",
                "crates/bbb-native/src/runtime.rs#camera_entity_id",
            ),
            (
                "crates/bbb-world/src/client/local_player.rs#camera_updates_only_for_local_or_known_entities",
            ),
        ),
        ("isControlledCamera",),
    ),
)

LOCAL_PLAYER_INVENTORY_ACTIONS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/inventory.rs#drop_local_selected_hotbar_item",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#drop_key_predicts_one_selected_item_and_swings_when_non_empty",
                "crates/bbb-native/src/input/tests.rs#control_drop_key_predicts_selected_stack_and_swings_when_non_empty",
            ),
        ),
        ("drop",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_swing_command",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#spectator_drop_key_does_not_drop_or_swing",
            ),
        ),
        ("swing",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/inventory/creative.rs#submit_action_inner",
            ),
            (
                "crates/bbb-native/src/input/inventory/creative.rs#creative_drop_throttler_limits_decays_and_resets_for_a_new_level_owner",
            ),
        ),
        ("canDropItems", "handleCreativeModeItemDrop"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/inventory.rs#close_local_container",
                "crates/bbb-native/src/input/commands.rs#queue_container_close_command",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#escape_key_closes_local_inventory_and_queues_container_zero",
            ),
        ),
        ("closeContainer", "clientSideCloseContainer"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#handle_key_input_with_item_runtime",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#inventory_key_on_server_controlled_mount_queues_open_inventory_command",
                "crates/bbb-native/src/input/tests.rs#inventory_key_closes_open_container_before_open_inventory_command",
            ),
        ),
        ("sendOpenInventory",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/recipe_book.rs#remove_recipe_highlight",
            ),
            (
                "crates/bbb-world/src/recipe_book.rs#local_recipe_highlight_removal_is_guarded_and_updates_counts",
            ),
        ),
        ("removeRecipeHighlight",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/inventory.rs#apply_local_container_click_slot",
            ),
            (
                "crates/bbb-world/src/client/inventory/tests.rs#apply_local_mount_quick_move_moves_mount_slots_to_player_reverse",
            ),
        ),
        ("updateTutorialInventoryAction",),
    ),
)

LOCAL_PLAYER_VEHICLE_CONTROL_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/store.rs#prepare_scheduled_passengers_for_ride_tick",
            ),
            (
                "crates/bbb-world/src/entities/tests/passenger_and_vehicle_movement.rs#passenger_ride_tick_clears_motion_before_client_entity_tick",
            ),
        ),
        ("rideTick",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/passengers.rs#local_player_rideable_jumping_vehicle_id",
                "crates/bbb-native/src/input/movement.rs#maybe_queue_riding_jump_command",
            ),
            (
                "crates/bbb-native/src/input/movement.rs#mounted_jumpable_vehicle_release_queues_riding_jump_command",
                "crates/bbb-native/src/input/movement.rs#mounted_non_jumpable_vehicle_release_does_not_queue_riding_jump",
            ),
        ),
        ("sendRidingJump", "jumpableVehicle"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/passengers.rs#local_player_sprintable_vehicle_id",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_sprint_latch_uses_sprintable_vehicle_instead_of_food_when_mounted",
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_sprint_latch_requires_controlling_sprintable_vehicle_passenger",
            ),
        ),
        ("vehicleCanSprint",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/passengers.rs#apply_set_passengers",
                "crates/bbb-world/src/entities/passengers.rs#clear_local_player_mount",
            ),
            (
                "crates/bbb-world/src/entities/tests/passenger_and_vehicle_movement.rs#tracks_entity_passenger_updates",
            ),
        ),
        ("startRiding", "removeVehicle"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/features.rs#is_feature_enabled",
            ),
            (
                "crates/bbb-world/src/client/features.rs#update_enabled_features_replaces_known_feature_set_and_ignores_unknowns",
            ),
        ),
        ("shouldRotateWithMinecart",),
    ),
)

LOCAL_PLAYER_ITEM_USE_STATE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#set_local_using_item_with_hand",
                "crates/bbb-world/src/client/local_player.rs#take_local_using_item",
            ),
            (
                "crates/bbb-world/src/client/local_player.rs#local_interaction_state_tracks_prediction_destroy_and_use_item",
                "crates/bbb-world/src/client/inventory/use_prediction.rs#trident_release_obeys_hold_threshold_and_keeps_throw_damage_server_owned",
            ),
        ),
        ("startUsingItem", "stopUsingItem", "getUsedItemHand"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player_movement.rs#local_player_is_slow_due_to_using_item",
                "crates/bbb-world/src/client/local_player_movement.rs#local_player_using_item_speed_multiplier",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_using_item_slows_input_without_stopping_an_active_sprint_latch",
            ),
        ),
        ("isSlowDueToUsingItem", "itemUseSpeedMultiplier"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/store.rs#model_source",
            ),
            (
                "crates/bbb-world/src/entities/schema.rs#generated_living_entity_accessors_own_membership_indices_defaults_and_masks",
            ),
        ),
        ("onSyncedDataUpdated",),
    ),
)

LOCAL_PLAYER_PERMISSION_CHAT_STATE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#apply_local_player_permission_entity_event",
            ),
            (
                "crates/bbb-world/src/client/local_player.rs#local_player_permission_entity_events_update_gamemaster_gate_without_entity_projection",
            ),
        ),
        ("setPermissions", "handleEntityEvent"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/hud.rs#apply_system_chat_at",
                "crates/bbb-world/src/client/chat.rs#push_rich_server_system_chat_message",
            ),
            (
                "crates/bbb-world/src/client/hud.rs#hud_text_packets_update_canonical_state",
                "crates/bbb-world/src/client/chat.rs#client_system_chat_tracks_local_display_message_without_protocol_counters",
            ),
        ),
        ("sendSystemMessage", "sendOverlayMessage"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/chat/presentation.rs#can_receive_system_messages",
            ),
            (
                "crates/bbb-world/src/client/chat.rs#disguised_chat_tracks_system_like_message",
            ),
        ),
        ("refreshChatAbilities",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#client_text_filtering_enabled",
            ),
            (
                "crates/bbb-native/src/startup.rs#args_build_client_information_from_startup_options",
            ),
        ),
        ("isTextFilteringEnabled",),
    ),
)

INTERACTION_GAME_MODE_LIFECYCLE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#set_game_type_from_game_event_param",
                "crates/bbb-world/src/client/local_player.rs#reset_local_player_for_login",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_spectator_skips_suffocation_escape",
                "crates/bbb-world/src/level.rs#respawn_without_keep_data_resets_local_player_runtime_state",
            ),
        ),
        ("MultiPlayerGameMode", "setLocalMode"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#game_type_player_abilities",
            ),
            (
                "crates/bbb-world/src/client/local_player.rs#login_player_abilities_match_vanilla_game_types",
            ),
        ),
        ("adjustPlayer",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#reset_local_player_for_respawn",
            ),
            (
                "crates/bbb-world/src/level.rs#respawn_keep_all_data_preserves_entity_data_pose_and_attribute_modifiers",
                "crates/bbb-world/src/level.rs#respawn_without_keep_data_resets_local_player_runtime_state",
            ),
        ),
        ("createPlayer",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#debug_game_mode_switcher_can_open",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#f3_f4_game_mode_switcher_does_not_open_over_existing_screen",
            ),
        ),
        ("canSwitchGameMode",),
    ),
)

INTERACTION_PRIMARY_ACTION_ROUTING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/mouse.rs#local_player_attack_is_blocked",
                "crates/bbb-native/src/input/mouse.rs#handle_mouse_input_at_partial_tick_with_effects_and_presentation",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#left_mouse_press_on_entity_queues_attack_and_swing",
                "crates/bbb-native/src/input/mouse.rs#local_attack_miss_time_uses_screen_sentinel_and_clears_on_release",
            ),
        ),
        ("startAttack",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/mouse.rs#handle_mouse_input_at_partial_tick_with_effects_and_presentation",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#held_left_mouse_while_using_item_does_not_continue_or_abort_destroy",
                "crates/bbb-native/src/input/mouse.rs#held_left_mouse_with_piercing_weapon_does_not_continue_or_abort_destroy",
            ),
        ),
        ("continueAttack",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/mouse.rs#start_use_item",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#right_mouse_press_without_block_queues_use_item",
                "crates/bbb-native/src/input/mouse.rs#right_mouse_bucket_prediction_uses_the_queued_use_item_sequence",
            ),
        ),
        ("startUseItem",),
    ),
)

INTERACTION_GAME_MODE_GUARDS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#update_renderer_from_publication",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_heart_health_frame_matches_vanilla_damage_hold_and_blink_phase",
            ),
        ),
        ("canHurtPlayer", "hasExperience"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#advance_attack_miss_time",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#local_attack_miss_time_uses_screen_sentinel_and_clears_on_release",
            ),
        ),
        ("hasMissTime",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#local_player_is_spectator",
            ),
            (
                "crates/bbb-world/src/client/local_player_movement/tests.rs#local_player_spectator_skips_suffocation_escape",
            ),
        ),
        ("isSpectator",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/passengers.rs#local_player_server_controlled_inventory_vehicle_id",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#inventory_key_on_server_controlled_mount_queues_open_inventory_command",
            ),
        ),
        ("isServerControlledInventory",),
    ),
)

LOCAL_PLAYER_SESSION_RESPAWN_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#reset_local_player_for_login",
                "crates/bbb-world/src/level.rs#reset_local_player_for_respawn",
            ),
            (
                "crates/bbb-world/src/level.rs#respawn_without_keep_data_resets_local_player_runtime_state",
            ),
        ),
        ("LocalPlayer", "resetPos"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_perform_respawn_command",
            ),
            (
                "crates/bbb-native/src/input/commands.rs#queues_perform_respawn_command",
                "crates/bbb-native/src/input/death_screen.rs#death_screen_buttons_unlock_after_twenty_ticks_and_respawn_once",
            ),
        ),
        ("respawn",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/animations.rs#advance_client_tick",
            ),
            (
                "crates/bbb-world/src/entities/tests/combat_state_and_mob_flag_projection.rs#entity_model_sources_project_death_animation_counter",
            ),
        ),
        ("tickDeath",),
    ),
)

LOCAL_PLAYER_VITALS_STATUS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#apply_player_health",
                "crates/bbb-world/src/entities/store.rs#apply_local_living_heal",
            ),
            (
                "crates/bbb-world/src/client/local_player.rs#local_player_packets_update_canonical_state",
                "crates/bbb-world/src/client/inventory/use_prediction.rs#iron_golem_interaction_applies_client_heal_before_selecting_result",
            ),
        ),
        ("heal", "hurtTo"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#apply_player_experience",
            ),
            (
                "crates/bbb-world/src/client/local_player.rs#local_player_packets_update_canonical_state",
            ),
        ),
        ("setExperienceValues",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/waypoint_locator.rs#hud_locator_bar_from_world",
            ),
            (
                "crates/bbb-native/src/runtime/waypoint_locator.rs#contextual_bar_uses_vanilla_idle_and_jump_priority",
            ),
        ),
        ("setExperienceDisplayStartTickToTickCount",),
    ),
)

LOCAL_PLAYER_MOVEMENT_REPORTING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/movement.rs#advance_player_input_for_client_ticks",
            ),
            (
                "crates/bbb-native/src/input/movement.rs#move_command_forces_position_after_vanilla_reminder_interval",
                "crates/bbb-native/src/input/movement.rs#move_command_position_reminder_is_not_reset_by_rotation_only_commands",
            ),
        ),
        ("tick", "sendPosition"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/movement.rs#maybe_queue_local_player_sprint_transition",
            ),
            (
                "crates/bbb-native/src/input/movement.rs#sprint_command_queue_failure_keeps_baseline_and_retries_next_tick",
            ),
        ),
        ("sendIsSprintingIfNeeded",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_player_abilities_command",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#creative_flight_toggle_suppresses_same_jump_fall_flying_command",
            ),
        ),
        ("onUpdateAbilities",),
    ),
)

LOCAL_PLAYER_CLIENT_STATUS_OPTIONS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#set_game_type_from_game_event_param",
                "crates/bbb-world/src/level.rs#apply_game_event",
            ),
            (
                "crates/bbb-world/src/level.rs#game_events_update_local_gameplay_state",
            ),
        ),
        ("onGameModeChanged", "setShowDeathScreen", "setDoLimitedCrafting"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#set_local_player_reduced_debug_info_from_entity_event",
            ),
            (
                "crates/bbb-world/src/level.rs#game_events_update_local_gameplay_state",
            ),
        ),
        ("setReducedDebugInfo",),
    ),
)

INTERACTION_ITEM_USE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#send_reserved_use_item_command",
                "crates/bbb-native/src/input/commands.rs#send_reserved_use_item_on_command",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#right_mouse_press_without_block_queues_use_item",
                "crates/bbb-native/src/input/mouse.rs#right_mouse_bucket_prediction_uses_the_queued_use_item_sequence",
            ),
        ),
        ("useItem", "useItemOn", "performUseItemOn"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#send_reserved_player_action_command",
            ),
            (
                "crates/bbb-world/src/client/inventory/use_prediction.rs#trident_release_obeys_hold_threshold_and_keeps_throw_damage_server_owned",
            ),
        ),
        ("releaseUsingItem",),
    ),
)

INTERACTION_ENTITY_ACTIONS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_attack_entity_command",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#left_mouse_press_on_entity_queues_attack_and_swing",
                "crates/bbb-native/src/input/mouse.rs#left_mouse_press_with_piercing_weapon_queues_stab_and_swing_on_entity",
            ),
        ),
        ("attack", "piercingAttack"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_interact_entity_command",
            ),
            (
                "crates/bbb-native/src/input/commands.rs#queues_entity_attack_and_interact_commands",
            ),
        ),
        ("interact",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_spectate_entity_command",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#spectator_left_mouse_press_with_piercing_weapon_queues_spectate_only",
            ),
        ),
        ("spectate",),
    ),
)

INTERACTION_CONTAINER_ACTIONS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_container_click_command",
            ),
            (
                "crates/bbb-native/src/input/commands.rs#queues_container_click_command",
            ),
        ),
        ("handleContainerInput",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_container_button_click_command",
            ),
            (
                "crates/bbb-native/src/input/commands.rs#queues_container_button_click_command",
            ),
        ),
        ("handleInventoryButtonClick",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_place_recipe_command",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#recipe_book_recipe_button_click_queues_place_recipe_command",
            ),
        ),
        ("handlePlaceRecipe",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_container_slot_state_changed_command",
            ),
            (
                "crates/bbb-native/src/input/commands.rs#queues_container_slot_state_changed_command",
                "crates/bbb-native/src/input/commands.rs#queues_container_slot_state_and_click_atomically",
            ),
        ),
        ("handleSlotStateChanged",),
    ),
)

INTERACTION_CREATIVE_ACTIONS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/inventory/creative.rs#submit_action_inner",
            ),
            (
                "crates/bbb-native/src/input/inventory/creative.rs#creative_drop_throttler_limits_decays_and_resets_for_a_new_level_owner",
            ),
        ),
        ("handleCreativeModeItemAdd", "handleCreativeModeItemDrop"),
    ),
)

INTERACTION_CARRIED_SLOT_SYNC_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#select_hotbar_slot",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#digit_key_selects_hotbar_slot_updates_world_and_queues_command",
            ),
        ),
        ("ensureHasSentCarriedItem", "tick"),
    ),
)

INTERACTION_PICK_ACTIONS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs#queue_pick_item_from_block_command",
                "crates/bbb-native/src/input/commands.rs#queue_pick_item_from_entity_command",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#control_middle_mouse_press_on_block_queues_pick_block_with_data",
                "crates/bbb-native/src/input/mouse.rs#control_middle_mouse_press_on_entity_queues_pick_entity_with_data",
                "crates/bbb-native/src/input/mouse.rs#sprint_middle_mouse_press_does_not_include_pick_data",
            ),
        ),
        (
            "pickBlockOrEntity",
            "handlePickItemFromBlock",
            "handlePickItemFromEntity",
        ),
    ),
)

UI_SCREEN_LIFECYCLE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#runtime_wants_cursor",
                "crates/bbb-world/src/level.rs#clear_level_bound_state",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#dialog_stack_suspends_and_restores_the_native_chat_screen",
                "crates/bbb-native/src/main.rs#runtime_wants_cursor_for_debug_game_mode_switcher",
            ),
        ),
        ("setScreen",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/resource_reload.rs#apply_server_resource_pack_reload_effect",
            ),
            (
                "crates/bbb-native/src/resource_reload.rs#renderer_resource_owners_commit_inside_one_reload_barrier",
            ),
        ),
        ("setOverlay",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#open_debug_options_screen",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#f3_f4_game_mode_switcher_does_not_open_over_existing_screen",
            ),
        ),
        ("canInterruptScreen",),
    ),
)

CLIENT_LEVEL_BLOCK_ENTITY_TRACKING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/block_entity_visibility.rs#visible_block_position",
            ),
            (
                "crates/bbb-native/src/block_entity_visibility.rs#default_and_end_gateway_renderers_use_vanilla_view_distances",
                "crates/bbb-native/src/block_entity_visibility.rs#ordinary_models_use_the_containing_section_as_the_frustum_unit",
                "crates/bbb-native/src/block_entity_visibility.rs#compiled_section_gate_precedes_camera_frustum_submission",
            ),
        ),
        ("onBlockEntityAdded",),
    ),
)

CLIENT_LEVEL_LEVEL_EVENTS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/audio.rs#global_level_event_sound",
            ),
            (
                "crates/bbb-world/src/client/audio.rs#global_level_event_sound_maps_vanilla_camera_relative_audio",
            ),
        ),
        ("globalLevelEvent",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/play_apply.rs#apply_level_event_with_effects",
                "crates/bbb-world/src/block_events.rs#apply_level_event",
            ),
            (
                "crates/bbb-world/src/block_events.rs#tracks_transient_block_and_level_events",
                "crates/bbb-world/src/play_apply.rs#sink_less_level_event_advances_particle_randoms_like_online_dispatcher",
            ),
        ),
        ("levelEvent",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/particle_runtime/explosion.rs#track",
            ),
            (
                "crates/bbb-native/src/particle_runtime/tests.rs#explosion_tracker_is_deterministic_caps_at_512_and_clears_after_tick",
                "crates/bbb-world/src/play_apply.rs#apply_play_packet_orders_typed_explosion_effects_before_knockback",
            ),
        ),
        ("trackExplosionEffects",),
    ),
)

CLIENT_LEVEL_MAP_DATA_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/maps.rs#map_item",
                "crates/bbb-native/src/item_frames.rs#item_frame_models_with_map_cache",
            ),
            (
                "crates/bbb-native/src/item_frames.rs#filled_map_frame_renders_decoded_full_frame_map_when_data_exists",
            ),
        ),
        ("getMapData",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/maps.rs#apply_map_item_data",
            ),
            (
                "crates/bbb-world/src/maps.rs#map_item_data_rejects_invalid_patch_without_replacing_last_patch",
                "crates/bbb-world/src/client/component_owner.rs#advancement_and_map_batches_reject_atomically_and_keep_old_publication",
            ),
        ),
        ("overrideMapData",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#apply_respawn",
                "crates/bbb-world/src/maps.rs#map_items",
            ),
            (
                "crates/bbb-world/src/level.rs#respawn_updates_dimension_and_clears_old_chunks",
                "crates/bbb-world/src/level.rs#login_resets_map_store_while_dimension_respawn_carries_it_over",
            ),
        ),
        ("addMapData", "getAllMapData"),
    ),
)

CLIENT_LEVEL_DIAGNOSTICS_REPORT_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/level_report.rs#client_level_report_details",
                "crates/bbb-native/src/crash_report.rs#attach_level_details",
            ),
            (
                "crates/bbb-world/src/client/level_report.rs#report_details_follow_the_current_world_generation",
                "crates/bbb-native/src/crash_report.rs#level_details_render_as_the_affected_level_category",
            ),
        ),
        ("fillReportDetails",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/level_report.rs#chunk_source_stats_line",
            ),
            (
                "crates/bbb-world/src/client/level_report.rs#chunk_source_stats_line_matches_the_vanilla_shape",
                "crates/bbb-native/src/runtime/tests.rs#hud_debug_overlay_projects_custom_chunk_source_stats_under_reduced_debug_info",
            ),
        ),
        ("gatherChunkSourceStats",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/level_report.rs#client_level_identity",
            ),
            (
                "crates/bbb-world/src/client/level_report.rs#client_level_identity_is_the_vanilla_to_string_constant",
            ),
        ),
        ("toString",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/level_report.rs#level_spawn_location_detail",
            ),
            (
                "crates/bbb-world/src/client/level_report.rs#report_details_assemble_live_values_in_vanilla_order",
            ),
        ),
        ("fillCrashReportCategory",),
    ),
)

METHOD_OWNER_EVIDENCE = {
    **LEVEL_RENDERER_OWNER_EVIDENCE,
    "world-client-level-lifecycle": CLIENT_LEVEL_IDENTITY_LIFECYCLE_OWNER_EVIDENCE,
    "world-client-chunk-light-lifecycle": CLIENT_LEVEL_CHUNK_LIGHT_OWNER_EVIDENCE,
    "rendering-client-level-dirty-handoff": CLIENT_LEVEL_DIRTY_HANDOFF_OWNER_EVIDENCE,
    "world-client-clock-weather": CLIENT_LEVEL_CLOCK_WEATHER_OWNER_EVIDENCE,
    "world-client-ambient-animation": CLIENT_LEVEL_AMBIENT_ANIMATION_OWNER_EVIDENCE,
    "world-client-respawn-data": CLIENT_LEVEL_RESPAWN_DATA_OWNER_EVIDENCE,
    "world-client-session-services": CLIENT_LEVEL_SESSION_SERVICES_OWNER_EVIDENCE,
    "rendering-client-biome-tint": CLIENT_LEVEL_BIOME_TINT_OWNER_EVIDENCE,
    "audio-client-level-sound-routing": CLIENT_LEVEL_SOUND_ROUTING_OWNER_EVIDENCE,
    "rendering-client-particle-routing": CLIENT_LEVEL_PARTICLE_ROUTING_OWNER_EVIDENCE,
    "world-client-map-data": CLIENT_LEVEL_MAP_DATA_OWNER_EVIDENCE,
    "world-client-level-events": CLIENT_LEVEL_LEVEL_EVENTS_OWNER_EVIDENCE,
    "rendering-client-block-entity-tracking": CLIENT_LEVEL_BLOCK_ENTITY_TRACKING_OWNER_EVIDENCE,
    "player-local-input-movement": LOCAL_PLAYER_INPUT_MOVEMENT_OWNER_EVIDENCE,
    "player-local-environment-effects": LOCAL_PLAYER_ENVIRONMENT_EFFECTS_OWNER_EVIDENCE,
    "player-local-world-edit-ui": LOCAL_PLAYER_WORLD_EDIT_UI_OWNER_EVIDENCE,
    "player-local-camera-targeting": LOCAL_PLAYER_CAMERA_TARGETING_OWNER_EVIDENCE,
    "player-local-inventory-actions": LOCAL_PLAYER_INVENTORY_ACTIONS_OWNER_EVIDENCE,
    "player-local-vehicle-control": LOCAL_PLAYER_VEHICLE_CONTROL_OWNER_EVIDENCE,
    "player-local-item-use-state": LOCAL_PLAYER_ITEM_USE_STATE_OWNER_EVIDENCE,
    "player-local-permission-chat-state": LOCAL_PLAYER_PERMISSION_CHAT_STATE_OWNER_EVIDENCE,
    "interaction-game-mode-lifecycle": INTERACTION_GAME_MODE_LIFECYCLE_OWNER_EVIDENCE,
    "interaction-primary-action-routing": INTERACTION_PRIMARY_ACTION_ROUTING_OWNER_EVIDENCE,
    "interaction-game-mode-guards": INTERACTION_GAME_MODE_GUARDS_OWNER_EVIDENCE,
    "player-local-session-respawn": LOCAL_PLAYER_SESSION_RESPAWN_OWNER_EVIDENCE,
    "player-local-vitals-status": LOCAL_PLAYER_VITALS_STATUS_OWNER_EVIDENCE,
    "player-local-movement-reporting": LOCAL_PLAYER_MOVEMENT_REPORTING_OWNER_EVIDENCE,
    "player-local-client-status-options": LOCAL_PLAYER_CLIENT_STATUS_OPTIONS_OWNER_EVIDENCE,
    "interaction-item-use": INTERACTION_ITEM_USE_OWNER_EVIDENCE,
    "interaction-entity-actions": INTERACTION_ENTITY_ACTIONS_OWNER_EVIDENCE,
    "interaction-container-actions": INTERACTION_CONTAINER_ACTIONS_OWNER_EVIDENCE,
    "interaction-creative-actions": INTERACTION_CREATIVE_ACTIONS_OWNER_EVIDENCE,
    "interaction-carried-slot-sync": INTERACTION_CARRIED_SLOT_SYNC_OWNER_EVIDENCE,
    "ui-screen-lifecycle": UI_SCREEN_LIFECYCLE_OWNER_EVIDENCE,
    "interaction-pick-actions": INTERACTION_PICK_ACTIONS_OWNER_EVIDENCE,
    "diagnostics-client-level-report": CLIENT_LEVEL_DIAGNOSTICS_REPORT_OWNER_EVIDENCE,
    "world-client-entity-lifecycle": CLIENT_LEVEL_ENTITY_LIFECYCLE_OWNER_EVIDENCE,
    "interaction-block-breaking": INTERACTION_BLOCK_BREAKING_OWNER_EVIDENCE,
    "interaction-prediction-sequence": INTERACTION_PREDICTION_SEQUENCE_OWNER_EVIDENCE,
}

def _registration_evidence(
    *groups: tuple[OwnerEvidence, tuple[str, ...]],
) -> dict[str, OwnerEvidence]:
    evidence: dict[str, OwnerEvidence] = {}
    for owner_evidence, registered in groups:
        for key in registered:
            if key in evidence:
                raise ValueError(f"duplicate registration evidence: {key}")
            evidence[key] = owner_evidence
    return evidence


BLOCK_ENTITY_RENDERER_OWNER_EVIDENCE = _registration_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-native/src/sign_scene.rs#sign_scene_from_world",),
            (
                "crates/bbb-native/src/sign_scene.rs#projects_sign_instances_with_kind_rotation_and_light",
                "crates/bbb-native/src/sign_scene.rs#bakes_text_surfaces_only_for_faces_with_text",
            ),
        ),
        ("BlockEntityType.SIGN", "BlockEntityType.HANGING_SIGN"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/spawner_scene.rs#spawner_display_entity_submissions_from_world_at_partial_tick",
            ),
            (
                "crates/bbb-native/src/spawner_scene.rs#projects_spawner_display_entity_as_wrapped_entity_model_instance",
                "crates/bbb-native/src/spawner_scene.rs#projects_trial_spawner_custom_entity_through_shared_entity_world",
            ),
        ),
        ("BlockEntityType.MOB_SPAWNER", "BlockEntityType.TRIAL_SPAWNER"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/item_models/dropped.rs#entity_block_attachments",),
            (
                "crates/bbb-native/src/item_models/tests.rs#entity_block_attachments_submit_interpolated_moving_piston_blocks",
                "crates/bbb-native/src/item_models/tests.rs#moving_piston_blocks_go_through_the_block_entity_visibility_gate",
            ),
        ),
        ("BlockEntityType.PISTON",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/chest_scene.rs#chest_model_instances_from_world_at_partial_tick",),
            (
                "crates/bbb-native/src/chest_scene.rs#projects_single_chest_instance_with_facing_openness_and_light",
                "crates/bbb-native/src/chest_scene.rs#projects_double_chest_halves_with_shared_max_openness",
            ),
        ),
        (
            "BlockEntityType.CHEST",
            "BlockEntityType.ENDER_CHEST",
            "BlockEntityType.TRAPPED_CHEST",
        ),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/enchanting_table_book_scene.rs#enchanting_table_book_model_instances_from_world_at_partial_tick",
            ),
            (
                "crates/bbb-native/src/enchanting_table_book_scene.rs#projects_a_closed_book_for_an_untouched_table",
                "crates/bbb-native/src/enchanting_table_book_scene.rs#extracts_partial_tick_lerp_and_book_state_fields",
            ),
        ),
        ("BlockEntityType.ENCHANTING_TABLE",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/lectern_book_scene.rs#lectern_book_model_instances_from_world",),
            (
                "crates/bbb-native/src/lectern_book_scene.rs#projects_only_lecterns_with_a_book_with_fixed_state_and_facing_yaw",
                "crates/bbb-native/src/lectern_book_scene.rs#facing_maps_to_the_clockwise_yaw",
            ),
        ),
        ("BlockEntityType.LECTERN",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/end_portal_scene.rs#end_portal_model_instances_from_world_at_partial_tick",
            ),
            (
                "crates/bbb-native/src/end_portal_scene.rs#end_portal_and_gateway_project_to_renderer_block_instances",
                "crates/bbb-native/src/end_portal_scene.rs#active_gateway_projects_beam_render_state",
            ),
        ),
        ("BlockEntityType.END_PORTAL", "BlockEntityType.END_GATEWAY"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/beacon_scene.rs#beacon_beam_model_instances_from_world_at_partial_tick",
                "crates/bbb-world/src/beacon_blocks.rs#beacon_beam_source_states",
            ),
            (
                "crates/bbb-world/src/beacon_blocks.rs#beam_publishes_sections_once_the_scan_reaches_the_surface",
                "crates/bbb-native/src/beacon_scene.rs#each_section_becomes_one_beam_instance_at_the_block_corner",
                "crates/bbb-renderer/src/entity_models/tests/beacon.rs#beacon_beam_spans_its_section_and_carries_the_vanilla_radii",
            ),
        ),
        ("BlockEntityType.BEACON",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/skull_scene.rs#skull_model_instances_from_world_at_partial_tick",),
            (
                "crates/bbb-native/src/skull_scene.rs#ground_skull_projects_rotation_and_static_kind",
                "crates/bbb-native/src/skull_scene.rs#powered_dragon_head_projects_animation_progress",
            ),
        ),
        ("BlockEntityType.SKULL",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/banner_scene.rs#banner_model_instances_from_world_at_partial_tick",),
            (
                "crates/bbb-native/src/banner_scene.rs#projects_banner_instances_with_color_patterns_yaw_phase_and_light",
                "crates/bbb-native/src/banner_scene.rs#named_block_entity_layer_uses_synchronized_pattern_value_not_static_resource_name",
            ),
        ),
        ("BlockEntityType.BANNER",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/bounding_box_gizmos.rs#bounding_box_gizmo_lines",
                "crates/bbb-world/src/bounding_box_blocks.rs#bounding_box_renderable_source_states",
            ),
            (
                "crates/bbb-world/src/bounding_box_blocks.rs#extraction_is_gated_on_game_master_access_and_scans_invisible_blocks",
                "crates/bbb-native/src/bounding_box_gizmos.rs#the_bounding_box_marks_its_three_axis_edges_at_the_min_corner",
                "crates/bbb-native/src/bounding_box_gizmos.rs#invisible_block_markers_use_the_vanilla_colors_and_air_inflation",
            ),
        ),
        ("BlockEntityType.STRUCTURE_BLOCK",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/test_instance_gizmos.rs#test_instance_error_gizmos",
                "crates/bbb-world/src/test_instance_blocks.rs#test_instance_beam_sections",
            ),
            (
                "crates/bbb-native/src/test_instance_gizmos.rs#error_markers_match_vanilla_geometry_color_and_label",
                "crates/bbb-world/src/test_instance_blocks.rs#beam_sections_and_boxes_follow_the_run_status_and_definition_defaults",
                "crates/bbb-native/src/block_entity_visibility.rs#off_screen_renderers_use_their_own_should_render_rules",
            ),
        ),
        ("BlockEntityType.TEST_INSTANCE_BLOCK",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/shulker_box_scene.rs#shulker_box_model_instances_from_world_at_partial_tick",
            ),
            (
                "crates/bbb-native/src/shulker_box_scene.rs#projects_opening_and_resting_boxes_with_color_facing_and_light",
            ),
        ),
        ("BlockEntityType.SHULKER_BOX",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/bed_scene.rs#bed_model_instances_from_world",),
            (
                "crates/bbb-native/src/bed_scene.rs#projects_bed_halves_with_color_part_and_facing_angle",
                "crates/bbb-native/src/bed_scene.rs#projects_facing_angles_like_vanilla_to_y_rot",
            ),
        ),
        ("BlockEntityType.BED",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/conduit_scene.rs#conduit_model_instances_from_world_at_partial_tick",),
            (
                "crates/bbb-native/src/conduit_scene.rs#inactive_conduit_projects_only_the_shell_part",
                "crates/bbb-native/src/conduit_scene.rs#active_conduit_projects_cage_winds_and_camera_facing_eye",
            ),
        ),
        ("BlockEntityType.CONDUIT",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/bell_scene.rs#bell_model_instances_from_world_at_partial_tick",),
            (
                "crates/bbb-native/src/bell_scene.rs#projects_shaking_and_resting_bells_with_partial_ticks_and_light",
                "crates/bbb-native/src/bell_scene.rs#shake_ends_after_the_vanilla_duration",
            ),
        ),
        ("BlockEntityType.BELL",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/item_models/block_entity.rs#campfire_item_transform",),
            (
                "crates/bbb-native/src/item_models/block_entity.rs#campfire_slots_rotate_through_the_block_facing_data_value",
                "crates/bbb-native/src/item_models/block_entity.rs#campfire_items_lie_flat_in_their_quadrant_at_the_vanilla_scale",
            ),
        ),
        ("BlockEntityType.CAMPFIRE",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/item_models/block_entity.rs#brushable_item_transform",),
            (
                "crates/bbb-native/src/item_models/block_entity.rs#brushable_items_slide_out_of_the_brushed_face_by_the_dust_stage",
            ),
        ),
        ("BlockEntityType.BRUSHABLE_BLOCK",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/decorated_pot_scene.rs#decorated_pot_model_instances_from_world_at_partial_tick",
            ),
            (
                "crates/bbb-native/src/decorated_pot_scene.rs#projects_pot_instances_with_patterns_facing_wobble_and_light",
                "crates/bbb-native/src/decorated_pot_scene.rs#facing_angles_match_vanilla_model_transformation",
            ),
        ),
        ("BlockEntityType.DECORATED_POT",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/item_models/block_entity.rs#vault_item_transform",),
            (
                "crates/bbb-native/src/item_models/block_entity.rs#vault_items_spin_above_the_cage_floor_from_the_ticked_client_data",
            ),
        ),
        ("BlockEntityType.VAULT",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/copper_golem_statue_scene.rs#copper_golem_statue_model_instances_from_world",
            ),
            (
                "crates/bbb-native/src/copper_golem_statue_scene.rs#statues_project_their_pose_weathering_and_facing",
                "crates/bbb-renderer/src/entity_models/tests/copper_golem.rs#copper_golem_statue_block_root_transform_faces_away_from_the_block_facing",
            ),
        ),
        ("BlockEntityType.COPPER_GOLEM_STATUE",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/item_models/block_entity.rs#shelf_item_transform",),
            (
                "crates/bbb-native/src/item_models/block_entity.rs#shelf_slots_spread_along_the_board_and_face_the_block_facing",
            ),
        ),
        ("BlockEntityType.SHELF",),
    ),
)


REGISTRATION_OWNER_EVIDENCE = {
    "rendering-block-entity-registration": BLOCK_ENTITY_RENDERER_OWNER_EVIDENCE,
}


OWNER_EVIDENCE_MECHANISMS = tuple(METHOD_OWNER_EVIDENCE) + tuple(REGISTRATION_OWNER_EVIDENCE)


METHOD_SURFACES = (
    MethodSurface("startup", "startup", "startup-client-bootstrap", "net/minecraft/client/main/Main.java"),
    MethodSurface("startup", "startup", "startup-session-identity", "net/minecraft/client/User.java"),
    MethodSurface("lifecycle", "lifecycle", "lifecycle-client-runtime", "net/minecraft/client/Minecraft.java"),
    MethodSurface("resources", "resources", "resources-default-pack-stack", "net/minecraft/client/resources/ClientPackSource.java"),
    MethodSurface("resources", "resources", "resources-default-pack-stack", "net/minecraft/client/resources/IndexedAssetSource.java"),
    MethodSurface("resources", "resources", "resources-default-pack-stack", "net/minecraft/server/packs/resources/FallbackResourceManager.java"),
    MethodSurface("resources", "resources", "resources-default-pack-stack", "net/minecraft/server/packs/resources/MultiPackResourceManager.java"),
    MethodSurface("resources", "resources", "resources-server-pack-lifecycle", "net/minecraft/client/resources/server/DownloadedPackSource.java"),
    MethodSurface("protocol-handler", "protocol", "protocol-login-handler-decisions", "net/minecraft/client/multiplayer/ClientHandshakePacketListenerImpl.java"),
    MethodSurface("protocol-handler", "protocol", "protocol-configuration-handler-decisions", "net/minecraft/client/multiplayer/ClientConfigurationPacketListenerImpl.java"),
    MethodSurface("protocol-handler", "protocol", "protocol-play-handler-decisions", "net/minecraft/client/multiplayer/ClientCommonPacketListenerImpl.java"),
    MethodSurface("protocol-handler", "protocol", "protocol-play-handler-decisions", "net/minecraft/client/multiplayer/ClientPacketListener.java"),
    MethodSurface("world", "world", "world-client-level-decisions", "net/minecraft/client/multiplayer/ClientLevel.java"),
    MethodSurface("movement", "movement", "player-local-input-movement", "net/minecraft/client/player/LocalPlayer.java"),
    MethodSurface("interaction", "interaction", "interaction-primary-action-routing", "net/minecraft/client/multiplayer/MultiPlayerGameMode.java"),
    MethodSurface("render-step", "ui", "rendering-hud-step-surface", "net/minecraft/client/gui/Gui.java"),
    MethodSurface("render-step", "rendering", "rendering-game-step-surface", "net/minecraft/client/renderer/GameRenderer.java"),
    MethodSurface("render-step", "rendering", "rendering-level-frame-graph", "net/minecraft/client/renderer/LevelRenderer.java"),
    MethodSurface("resource-consumer", "language", "resources-language-manager", "net/minecraft/client/resources/language/LanguageManager.java"),
    MethodSurface("resource-consumer", "text", "ui-font-manager", "net/minecraft/client/gui/font/FontManager.java"),
    MethodSurface("resource-consumer", "audio", "audio-sound-manager-engine", "net/minecraft/client/sounds/SoundManager.java"),
    MethodSurface("resource-consumer", "audio", "audio-sound-manager-engine", "net/minecraft/client/sounds/SoundEngine.java"),
    MethodSurface("input", "input", "input-keyboard-routing", "net/minecraft/client/KeyboardHandler.java"),
    MethodSurface("input", "input", "input-mouse-routing", "net/minecraft/client/MouseHandler.java"),
    MethodSurface("input", "input", "input-key-mapping", "net/minecraft/client/KeyMapping.java"),
    MethodSurface("platform", "platform", "platform-window-lifecycle", "com/mojang/blaze3d/platform/Window.java"),
    MethodSurface("accessibility", "accessibility", "ui-accessibility-surface", "net/minecraft/client/GameNarrator.java"),
    MethodSurface("accessibility", "accessibility", "ui-accessibility-surface", "com/mojang/blaze3d/platform/ClipboardManager.java"),
    MethodSurface("ui", "ui", "ui-chat-screen-surface", "net/minecraft/client/gui/screens/ChatScreen.java"),
    MethodSurface("ui", "ui", "ui-death-screen-surface", "net/minecraft/client/gui/screens/DeathScreen.java"),
    MethodSurface("ui", "ui", "ui-disconnected-screen-surface", "net/minecraft/client/gui/screens/DisconnectedScreen.java"),
    MethodSurface("ui", "ui", "ui-pause-screen-surface", "net/minecraft/client/gui/screens/PauseScreen.java"),
    MethodSurface("persistence", "persistence", "persistence-options-file", "net/minecraft/client/Options.java"),
    MethodSurface("persistence", "persistence", "persistence-server-list", "net/minecraft/client/multiplayer/ServerList.java"),
    MethodSurface("identity", "identity", "identity-skin-presentation", "net/minecraft/client/resources/SkinManager.java"),
    MethodSurface("identity", "identity", "identity-profile-key-lifecycle", "net/minecraft/client/multiplayer/AccountProfileKeyPairManager.java"),
    MethodSurface("security", "security", "security-secure-chat-reporting", "net/minecraft/client/multiplayer/chat/report/ReportingContext.java"),
    MethodSurface("security", "security", "security-secure-chat-reporting", "net/minecraft/client/gui/screens/social/PlayerSocialManager.java"),
    MethodSurface("diagnostics", "diagnostics", "diagnostics-crash-runtime", "net/minecraft/CrashReport.java"),
    MethodSurface("out-of-scope", "ui", "scope-title-screen", "net/minecraft/client/gui/screens/TitleScreen.java", "out-of-scope"),
    MethodSurface("out-of-scope", "singleplayer", "scope-singleplayer-integrated", "net/minecraft/client/gui/screens/worldselection/SelectWorldScreen.java", "out-of-scope"),
    MethodSurface("out-of-scope", "singleplayer", "scope-singleplayer-integrated", "net/minecraft/client/server/IntegratedServer.java", "out-of-scope"),
    MethodSurface("out-of-scope", "realms", "scope-realms", "com/mojang/realmsclient/RealmsMainScreen.java", "out-of-scope"),
    MethodSurface("out-of-scope", "options", "scope-options-ui", "net/minecraft/client/gui/screens/options/OptionsScreen.java", "out-of-scope"),
    MethodSurface("out-of-scope", "telemetry", "scope-telemetry-upload", "net/minecraft/client/telemetry/ClientTelemetryManager.java", "out-of-scope"),
    MethodSurface("out-of-scope", "telemetry", "scope-telemetry-upload", "net/minecraft/client/telemetry/WorldSessionTelemetryManager.java", "out-of-scope"),
)


LISTENER_PATHS = (
    "net/minecraft/network/protocol/common/ClientCommonPacketListener.java",
    "net/minecraft/network/protocol/configuration/ClientConfigurationPacketListener.java",
    "net/minecraft/network/protocol/cookie/ClientCookiePacketListener.java",
    "net/minecraft/network/protocol/game/ClientGamePacketListener.java",
    "net/minecraft/network/protocol/login/ClientLoginPacketListener.java",
    "net/minecraft/network/protocol/ping/ClientPongPacketListener.java",
    "net/minecraft/network/protocol/status/ClientStatusPacketListener.java",
)


REGISTRATION_SURFACES = (
    ("reload-listener", "resources", "resources-reload-listeners", "net/minecraft/client/Minecraft.java", ("registerReloadListener",)),
    ("entity-renderer", "entities", "rendering-entity-renderer-registration", "net/minecraft/client/renderer/entity/EntityRenderers.java", ("register", "registerPlayerItems")),
    ("block-entity-renderer", "rendering", "rendering-block-entity-registration", "net/minecraft/client/renderer/blockentity/BlockEntityRenderers.java", ("register",)),
    ("particle-provider", "rendering", "rendering-particle-provider-registration", "net/minecraft/client/particle/ParticleResources.java", ("register",)),
    ("menu-screen", "ui", "ui-menu-screen-registration", "net/minecraft/client/gui/screens/MenuScreens.java", ("register",)),
)


SERVERBOUND_SCAN_PATHS = (
    "net/minecraft/client/Minecraft.java",
    "net/minecraft/client/KeyboardHandler.java",
    "net/minecraft/client/player/LocalPlayer.java",
    "net/minecraft/client/multiplayer/MultiPlayerGameMode.java",
    "net/minecraft/client/multiplayer/ClientPacketListener.java",
)


MECHANISM_EVIDENCE = {
    "startup-client-bootstrap": ("crates/bbb-native/src/startup.rs",),
    "startup-session-identity": ("crates/bbb-net/src/types/connection.rs", "docs/wiki/architecture/offline-runtime.md"),
    "lifecycle-client-runtime": (
        "crates/bbb-native/src/network_runtime.rs#production_shutdown_has_one_owner_and_frozen_order",
        "crates/bbb-native/src/simulation/thread.rs#shutdown_and_join",
        "crates/bbb-native/src/runtime/tests.rs#entity_client_tick_cadence_tracks_normal_world_tick_rate",
        "crates/bbb-native/src/runtime/tests.rs#native_pause_screens_never_freeze_external_client_or_running_level_ticks",
        "scripts/playable_gate.py",
    ),
    "lifecycle-session-replacement": (
        "crates/bbb-native/src/runtime/events/dispatcher.rs#session_replacement_has_one_connection_owner_and_frozen_transfer_order",
        "crates/bbb-native/src/runtime/events/tests/world_state_lifecycle.rs#play_reconfiguration_protocol_switch_does_not_clear_the_level_twice",
        "crates/bbb-world/src/client/server_presentation.rs#new_connection_session_keeps_only_transfer_carry_and_server_data",
        "crates/bbb-native/src/resource_pack_runtime.rs#transfer_keeps_active_pack_while_terminal_disconnect_unloads_it",
        "crates/bbb-net/src/event_stream.rs#transfer_reconnect_uses_transfer_intent_and_carries_session_cookies",
        "crates/bbb-native/src/network_runtime.rs#terminal_close_does_not_fabricate_remote_disconnect",
        "scripts/playable_gate.py",
    ),
    "lifecycle-client-task-handoff": (
        "crates/bbb-native/src/simulation/thread.rs#run_scheduled",
        "crates/bbb-native/src/simulation/thread.rs#producer_wake_without_work_emits_no_output_and_preserves_deadline",
        "crates/bbb-native/src/simulation/commands.rs#lifecycle_partition_reserves_shutdown_behind_saturated_input",
        "crates/bbb-native/src/simulation/commands.rs#close_unblocks_a_saturated_producer_and_rejects_control",
        "crates/bbb-native/src/simulation/commands.rs#production_producers_cannot_restore_ignored_mailbox_results",
    ),
    "resources-default-pack-stack": ("crates/bbb-pack/src/resources.rs", "docs/wiki/mechanisms/resources/offline-asset-trust.md"),
    "resources-reload-listeners": ("crates/bbb-native/src/resource_reload.rs",),
    "resources-reload-recovery": (
        "crates/bbb-native/src/resource_reload.rs#apply_native_resource_reload",
        "crates/bbb-native/src/resource_reload.rs#renderer_resource_owners_commit_inside_one_reload_barrier",
        "crates/bbb-native/src/simulation.rs#render_owner_commits_each_terminal_reload_before_consuming_the_next_batch",
        "crates/bbb-native/src/resource_pack_runtime.rs#failed_reload_keeps_previous_active_stack_and_fails_only_new_pack",
        "crates/bbb-native/src/resource_pack_runtime.rs#recovery_reload_runs_once_and_second_failure_does_not_hot_loop",
        "crates/bbb-native/src/resource_pack_runtime.rs#stale_reload_ack_does_not_commit_or_discard_the_current_barrier",
        "crates/bbb-native/src/resource_pack_runtime.rs#replaced_uuid_drops_old_result_without_mutating_new_request",
        "crates/bbb-native/src/runtime/tests.rs#completed_resource_pack_recovery_projects_pack_load_failure_system_toast",
        "crates/bbb-native/src/simulation.rs#failed_resource_pack_recovery_terminates_the_session_once",
    ),
    "resources-runtime-mipmap-update": (
        "crates/bbb-native/src/terrain_runtime/textures.rs#VANILLA_DEFAULT_MIPMAP_LEVELS",
        "docs/unsupported-features.md",
    ),
    "resources-server-pack-lifecycle": ("crates/bbb-pack/src/server_pack.rs",),
    "protocol-packet-tables": ("docs/evidence/vanilla-26.1/packet-coverage.json",),
    "protocol-listener-contracts": ("docs/evidence/vanilla-26.1/packet-coverage.json",),
    "protocol-login-handler-decisions": ("crates/bbb-net/src/event_stream.rs",),
    "protocol-configuration-handler-decisions": ("crates/bbb-net/src/event_stream.rs",),
    "protocol-play-handler-decisions": (
        "crates/bbb-world/src/play_apply.rs",
        "docs/evidence/vanilla-26.1/packet-coverage.json",
    ),
    "registries-built-in-catalogs": (
        "docs/evidence/vanilla-26.1/registry-holder-consumer-coverage.json",
    ),
    "registries-synchronized-data": (
        "docs/evidence/vanilla-26.1/registry-holder-consumer-coverage.json",
    ),
    "world-client-level-lifecycle": ("crates/bbb-world/src/level.rs",),
    "rendering-client-block-entity-tracking": (
        "crates/bbb-native/src/block_entity_visibility.rs",
    ),
    "world-client-clock-weather": ("crates/bbb-world/src/level.rs",),
    "world-client-chunk-light-lifecycle": ("crates/bbb-world/src/chunks/store.rs",),
    "rendering-client-level-dirty-handoff": (
        "crates/bbb-native/src/terrain_runtime.rs",
    ),
    "world-client-entity-lifecycle": ("crates/bbb-world/src/entities/store.rs",),
    "world-client-ambient-animation": (
        "crates/bbb-native/src/environment_animate_runtime.rs",
    ),
    "rendering-client-particle-routing": (
        "crates/bbb-native/src/particle_runtime.rs",
    ),
    "audio-client-level-sound-routing": (
        "crates/bbb-world/src/client/audio.rs",
        "crates/bbb-native/src/environment_audio_runtime.rs",
    ),
    "world-client-session-services": ("crates/bbb-world/src/registries.rs",),
    "world-client-map-data": ("crates/bbb-world/src/maps.rs",),
    "world-client-level-events": (
        "crates/bbb-world/src/play_apply.rs",
        "crates/bbb-native/src/runtime/events.rs",
    ),
    "rendering-client-biome-tint": ("crates/bbb-native/src/biome_tint.rs",),
    "world-client-respawn-data": ("crates/bbb-world/src/level.rs",),
    "diagnostics-client-level-report": (
        "crates/bbb-world/src/client/level_report.rs",
        "crates/bbb-native/src/crash_report.rs",
    ),
    "scope-client-level-noop-hooks": ("docs/unsupported-features.md",),
    "player-local-session-respawn": (
        "crates/bbb-native/src/input/death_screen.rs",
        "crates/bbb-world/src/client/combat.rs",
    ),
    "player-local-vitals-status": ("crates/bbb-world/src/client/local_player.rs",),
    "player-local-movement-reporting": ("crates/bbb-native/src/input/movement.rs",),
    "player-local-input-movement": (
        "crates/bbb-native/src/input/movement.rs",
        "crates/bbb-world/src/client/local_player_movement.rs",
    ),
    "player-local-vehicle-control": (
        "crates/bbb-native/src/input/movement.rs",
        "crates/bbb-world/src/entities/client_movement.rs",
    ),
    "player-local-permission-chat-state": (
        "crates/bbb-world/src/client/local_player.rs",
        "crates/bbb-native/src/input.rs",
    ),
    "player-local-client-status-options": (
        "crates/bbb-world/src/client/local_player.rs",
        "crates/bbb-native/src/input/death_screen.rs",
    ),
    "player-local-item-use-state": (
        "crates/bbb-world/src/client/local_player.rs",
        "crates/bbb-world/src/client/inventory/use_prediction.rs",
    ),
    "player-local-inventory-actions": (
        "crates/bbb-native/src/input/inventory.rs",
        "crates/bbb-world/src/client/inventory.rs",
    ),
    "player-local-world-edit-ui": ("crates/bbb-native/src/input.rs",),
    "player-local-environment-effects": (
        "crates/bbb-world/src/client/local_player_fluid.rs",
        "crates/bbb-native/src/input/movement.rs",
    ),
    "player-local-camera-targeting": (
        "crates/bbb-native/src/crosshair.rs",
        "crates/bbb-native/src/runtime.rs",
    ),
    "interaction-game-mode-lifecycle": (
        "crates/bbb-world/src/client/local_player.rs",
        "crates/bbb-native/src/input/movement.rs",
    ),
    "interaction-game-mode-guards": (
        "crates/bbb-world/src/client/local_player.rs",
        "crates/bbb-native/src/input/mouse.rs",
    ),
    "interaction-block-breaking": (
        "crates/bbb-world/src/client/local_player_destroy.rs",
        "crates/bbb-native/src/input/mouse.rs",
    ),
    "interaction-prediction-sequence": (
        "crates/bbb-world/src/block_events.rs",
        "crates/bbb-world/src/chunks/store.rs",
        "crates/bbb-native/src/input/commands.rs",
        "crates/bbb-native/src/input/mouse.rs",
    ),
    "interaction-carried-slot-sync": (
        "crates/bbb-native/src/input/commands.rs",
        "crates/bbb-world/src/client/inventory.rs",
    ),
    "interaction-item-use": (
        "crates/bbb-world/src/client/inventory/use_prediction.rs",
        "crates/bbb-native/src/input/mouse.rs",
    ),
    "interaction-entity-actions": (
        "crates/bbb-world/src/client/combat.rs",
        "crates/bbb-native/src/input/mouse.rs",
    ),
    "interaction-container-actions": (
        "crates/bbb-world/src/client/inventory.rs",
        "crates/bbb-native/src/input/inventory.rs",
    ),
    "interaction-creative-actions": (
        "crates/bbb-world/src/creative_inventory/action.rs",
        "crates/bbb-native/src/input/inventory/creative.rs",
    ),
    "interaction-pick-actions": (
        "crates/bbb-native/src/input/commands.rs",
        "crates/bbb-native/src/input/mouse.rs",
    ),
    "interaction-primary-action-routing": ("crates/bbb-native/src/input/mouse.rs",),
    "entities-fixed-type-surface": ("docs/evidence/vanilla-26.1/entity-coverage.json",),
    "rendering-entity-renderer-registration": (
        "docs/evidence/vanilla-26.1/entity-coverage.json",
    ),
    "rendering-entity-layer-registration": (
        "docs/evidence/vanilla-26.1/entity-coverage.json",
    ),
    "rendering-block-entity-registration": ("crates/bbb-native/src/block_entity_visibility.rs",),
    "rendering-particle-provider-registration": ("crates/bbb-native/src/particle_runtime.rs",),
    "ui-menu-screen-registration": ("crates/bbb-native/src/input/inventory.rs",),
    "rendering-hud-step-surface": ("crates/bbb-native/src/runtime/render_extract.rs",),
    "rendering-game-step-surface": ("crates/bbb-renderer/src/lib.rs",),
    "rendering-level-backend-lifecycle": (
        "crates/bbb-renderer/src/resource_reload.rs",
        "crates/bbb-native/src/resource_reload.rs",
    ),
    "rendering-level-section-lifecycle": (
        "crates/bbb-native/src/terrain_runtime.rs",
        "crates/bbb-renderer/src/terrain.rs",
    ),
    "rendering-level-frame-graph": (
        "crates/bbb-renderer/src/render.rs#FRAME_STEPS",
    ),
    "rendering-level-scene-projection": (
        "crates/bbb-native/src/runtime/render_extract.rs",
        "crates/bbb-renderer/src/render.rs",
    ),
    "rendering-level-effect-lifecycle": (
        "crates/bbb-native/src/weather_effect_runtime.rs#WeatherEffectRuntime",
        "crates/bbb-world/src/block_events.rs#block_destruction_render_stages",
        "crates/bbb-renderer/src/block_destroy.rs",
    ),
    "rendering-level-diagnostics-gizmos": (
        "crates/bbb-native/src/terrain_runtime/occlusion/debug.rs",
        "crates/bbb-renderer/src/debug_filled_boxes.rs",
    ),
    "rendering-level-light-query": (
        "crates/bbb-render-types/src/light_coords.rs#VanillaLightCoords",
        "crates/bbb-world/src/chunks/store.rs#sample_render_light_for_state",
        "crates/bbb-native/src/runtime/render_extract.rs#particle_light_for_world",
    ),
    "resources-language-manager": ("crates/bbb-pack/src/language.rs",),
    "ui-font-manager": ("crates/bbb-item-model/src/font.rs",),
    "audio-sound-manager-engine": ("crates/bbb-pack/src/sounds.rs", "crates/bbb-audio/src/runtime.rs"),
    "input-keyboard-routing": ("crates/bbb-native/src/input.rs",),
    "input-mouse-routing": ("crates/bbb-native/src/input/mouse.rs",),
    "input-key-mapping": ("crates/bbb-native/src/input.rs",),
    "platform-window-lifecycle": ("crates/bbb-native/src/platform.rs",),
    "protocol-serverbound-call-sites": (
        "crates/bbb-net/src/event_stream.rs",
        "docs/evidence/vanilla-26.1/packet-coverage.json",
    ),
    "ui-chat-screen-surface": (
        "crates/bbb-native/src/input.rs",
        "crates/bbb-native/src/runtime/render_extract.rs",
    ),
    "ui-death-screen-surface": (
        "crates/bbb-native/src/input/death_screen.rs",
        "crates/bbb-native/src/runtime/render_extract.rs",
    ),
    "ui-disconnected-screen-surface": (
        "crates/bbb-native/src/network_runtime.rs",
        "crates/bbb-native/src/runtime.rs",
    ),
    "ui-pause-screen-surface": (
        "crates/bbb-native/src/input/pause_screen.rs",
        "crates/bbb-native/src/runtime/render_extract.rs",
    ),
    "ui-screen-lifecycle": (
        "crates/bbb-native/src/input.rs",
        "crates/bbb-native/src/runtime/render_extract.rs",
    ),
    "ui-accessibility-surface": ("crates/bbb-native/src/narrator_runtime.rs",),
    "persistence-options-file": ("crates/bbb-native/src/startup.rs",),
    "persistence-options-derived": ("crates/bbb-native/src/startup.rs",),
    "persistence-options-apply": ("crates/bbb-native/src/startup.rs",),
    "persistence-server-list": ("crates/bbb-native/src/server_list.rs",),
    "identity-profile-key-lifecycle": (
        "crates/bbb-net/src/chat_session.rs",
        "crates/bbb-net/src/chat_session/runtime.rs",
        "crates/bbb-net/src/event_stream.rs",
    ),
    "identity-skin-presentation": ("crates/bbb-item-model/src/skin_runtime.rs",),
    "security-secure-chat-reporting": ("crates/bbb-native/src/input/reporting.rs",),
    "diagnostics-crash-runtime": (
        "crates/bbb-native/src/crash_report.rs#first_failure_wins_and_suppressed_failures_are_bounded",
        "crates/bbb-native/src/crash_report.rs#save_is_atomic_no_clobber_and_once_per_report",
        "crates/bbb-native/src/crash_report.rs#production_fatal_paths_cannot_restore_log_only_exit_or_string_worker_errors",
        "crates/bbb-native/src/simulation/thread.rs#worker_panic_is_reported_as_a_join_failure",
        "crates/bbb-native/src/network_runtime.rs#task_panic_is_returned_after_the_other_task_joins",
        "crates/bbb-native/src/network_runtime.rs#shared_deadline_aborts_and_reaps_a_blocked_relay",
    ),
    "scope-title-screen": ("docs/unsupported-features.md",),
    "scope-singleplayer-integrated": ("docs/unsupported-features.md",),
    "scope-realms": ("docs/unsupported-features.md",),
    "scope-options-ui": ("docs/unsupported-features.md",),
    "scope-telemetry-upload": ("docs/unsupported-features.md",),
    "scope-support-accessors": (
        "scripts/client_mechanism_inventory.py#is_direct_support_accessor",
    ),
    "rendering-nondeterministic-golden-exclusions": ("docs/unsupported-features.md",),
}


class ClientMechanismInventoryError(ValueError):
    """The selected source surface no longer has the expected CST shape."""


class InventoryBuilder:
    def __init__(self, source_root: Path) -> None:
        self.source_root = source_root
        self.sources: dict[str, JavaSource] = {}
        self.hashes: dict[str, str] = {}
        self.owners: list[dict[str, object]] = []

    def source(self, relative_path: str) -> JavaSource:
        if relative_path not in self.sources:
            path = self.source_root / relative_path
            raw = path.read_bytes()
            self.sources[relative_path] = parse_source(relative_path, raw)
            self.hashes[relative_path] = hashlib.sha256(raw).hexdigest()
        return self.sources[relative_path]

    def add_owner(
        self,
        *,
        owner_id: str,
        category: str,
        domain: str,
        mechanism_id: str,
        vanilla_owner: str,
        path: str,
        line: int,
        kind: str,
        scope: str = "external-client",
        role: str | None = None,
        evidence: OwnerEvidence | None = None,
    ) -> None:
        owner = {
            "category": category,
            "domain": domain,
            "kind": kind,
            "mechanism_id": mechanism_id,
            "owner_id": owner_id,
            "scope": scope,
            "source": {"line": line, "path": path},
            "vanilla_owner": vanilla_owner,
        }
        if role is not None:
            owner["role"] = role
        if evidence is not None:
            owner["bbb_evidence"] = {
                "implementation": list(evidence.implementation),
                "tests": list(evidence.tests),
            }
        self.owners.append(owner)

    @staticmethod
    def method_signature(method: JavaMethod) -> str:
        parameters = ",".join(parameter.canonical for parameter in method.parameters)
        return f"{method.owner}#{method.name}({parameters})"

    def add_method_surface(self, spec: MethodSurface) -> None:
        source = self.source(spec.source_path)
        if not source.types:
            raise ClientMechanismInventoryError(f"{spec.source_path}: no declared Java type")
        for method in sorted(source.methods, key=lambda item: item.span.start_byte):
            route = self.method_route(source, spec, method)
            signature = self.method_signature(method)
            evidence = None
            if route.mechanism_id in OWNER_EVIDENCE_MECHANISMS:
                evidence = METHOD_OWNER_EVIDENCE[route.mechanism_id].get(method.name)
                if evidence is None:
                    raise ClientMechanismInventoryError(
                        f"method owner lacks BBB implementation/test evidence: {signature}"
                    )
            self.add_owner(
                owner_id=f"method:{signature}",
                category=route.category,
                domain=route.domain,
                mechanism_id=route.mechanism_id,
                vanilla_owner=signature,
                path=spec.source_path,
                line=method.span.start_point[0] + 1,
                kind="constructor" if method.is_constructor else "method",
                scope=route.scope,
                role=route.role,
                evidence=evidence,
            )

    @staticmethod
    def method_route(
        source: JavaSource,
        spec: MethodSurface,
        method: JavaMethod,
    ) -> MethodRoute:
        exact_routes = None
        owner_name = None
        if spec.source_path == "net/minecraft/client/player/LocalPlayer.java":
            exact_routes = LOCAL_PLAYER_METHOD_ROUTES
            owner_name = "LocalPlayer"
        elif (
            spec.source_path
            == "net/minecraft/client/multiplayer/MultiPlayerGameMode.java"
        ):
            exact_routes = MULTIPLAYER_GAME_MODE_METHOD_ROUTES
            owner_name = "MultiPlayerGameMode"
        elif spec.source_path == "net/minecraft/client/multiplayer/ClientLevel.java":
            exact_routes = CLIENT_LEVEL_METHOD_ROUTES
            owner_name = "ClientLevel"
        if exact_routes is not None:
            route = exact_routes.get(method.name)
            signature = InventoryBuilder.method_signature(method)
            if route is None:
                raise ClientMechanismInventoryError(
                    f"{owner_name} method lacks a homogeneous route: {signature}"
                )
            if route == DIRECT_SUPPORT_ROUTE and not InventoryBuilder.is_direct_support_accessor(
                source, method
            ):
                raise ClientMechanismInventoryError(
                    f"{owner_name} support route is not a direct accessor: {signature}"
                )
            if (
                route.mechanism_id == "scope-client-level-noop-hooks"
                and not InventoryBuilder.is_empty_method_body(method)
            ):
                raise ClientMechanismInventoryError(
                    f"ClientLevel no-op route is not source-empty: {signature}"
                )
            return route
        if spec.source_path == "net/minecraft/client/renderer/LevelRenderer.java":
            route = LEVEL_RENDERER_METHOD_ROUTES.get(method.name)
            if route is None:
                signature = InventoryBuilder.method_signature(method)
                raise ClientMechanismInventoryError(
                    f"LevelRenderer method lacks a homogeneous route: {signature}"
                )
            return route
        if spec.source_path == "net/minecraft/client/Options.java":
            route = OPTIONS_METHOD_ROUTES.get(method.name)
            if route is not None:
                return route
            if InventoryBuilder.is_direct_support_accessor(source, method):
                return DIRECT_SUPPORT_ROUTE
            signature = InventoryBuilder.method_signature(method)
            raise ClientMechanismInventoryError(
                f"Options method lacks a decision/support route: {signature}"
            )
        if spec.source_path != "net/minecraft/client/Minecraft.java":
            return MethodRoute(
                spec.category,
                spec.domain,
                spec.mechanism_id,
                spec.scope,
                "out-of-scope" if spec.scope == "out-of-scope" else "decision",
            )
        if method.name in MINECRAFT_METHOD_ROUTES:
            return MINECRAFT_METHOD_ROUTES[method.name]
        if InventoryBuilder.is_direct_support_accessor(source, method):
            return DIRECT_SUPPORT_ROUTE
        signature = InventoryBuilder.method_signature(method)
        raise ClientMechanismInventoryError(
            f"Minecraft method lacks a decision/support route: {signature}"
        )

    @staticmethod
    def is_direct_support_accessor(source: JavaSource, method: JavaMethod) -> bool:
        body = method.node.child_by_field_name("body")
        if body is None or len(body.named_children) != 1:
            return False
        statement = body.named_children[0]
        if statement.type != "return_statement" or len(statement.named_children) != 1:
            return False
        expression = statement.named_children[0]
        if expression.type not in {"field_access", "identifier", "true"}:
            return False
        # A direct field/parameter/constant return has no independent decision,
        # mutation, allocation, or downstream call. Source hashes and this CST
        # predicate make any later body change fail the checked inventory.
        return bool(source.text(expression).strip())

    @staticmethod
    def is_empty_method_body(method: JavaMethod) -> bool:
        body = method.node.child_by_field_name("body")
        return body is not None and not body.named_children

    def add_listener_contracts(self) -> None:
        for path in LISTENER_PATHS:
            source = self.source(path)
            for method in sorted(source.methods, key=lambda item: item.span.start_byte):
                signature = self.method_signature(method)
                self.add_owner(
                    owner_id=f"listener:{signature}",
                    category="listener-contract",
                    domain="protocol",
                    mechanism_id="protocol-listener-contracts",
                    vanilla_owner=signature,
                    path=path,
                    line=method.span.start_point[0] + 1,
                    kind="listener-method",
                )

    def add_packet_tables(self) -> None:
        packet_type_sources = [
            self.source(path.relative_to(self.source_root).as_posix())
            for path in sorted((self.source_root / PROTOCOL_ROOT).rglob("*PacketTypes.java"))
        ]
        resources = packet_resource_ids(packet_type_sources)
        for state, relative, directions in PROTOCOL_SPECS:
            path = (PROTOCOL_ROOT / relative).as_posix()
            source = self.source(path)
            for direction in directions:
                packets = registered_packets(source, direction, resources)
                marker = f"{direction.upper()}_TEMPLATE"
                fields = [field for field in source.fields if field.name == marker]
                initializer = fields[0].initializer if len(fields) == 1 else None
                if initializer is None:
                    raise ClientMechanismInventoryError(f"{path}: missing {marker}")
                calls = [
                    call
                    for call in source.calls
                    if initializer.span.start_byte <= call.span.start_byte
                    and call.span.end_byte <= initializer.span.end_byte
                    and call.name in {"addPacket", "withBundlePacket"}
                ]
                calls.sort(key=lambda call: (call.span.end_byte, call.span.start_byte))
                if len(calls) != len(packets):
                    raise ClientMechanismInventoryError(
                        f"{path}: {direction} packet/call count differs"
                    )
                for packet_id, (packet, call) in enumerate(zip(packets, calls)):
                    owner_id = f"packet:{state}:{direction}:{packet.resource_id}"
                    self.add_owner(
                        owner_id=owner_id,
                        category="packet-table",
                        domain="protocol",
                        mechanism_id="protocol-packet-tables",
                        vanilla_owner=f"{state}/{direction}[{packet_id}] {packet.class_name}",
                        path=path,
                        line=call.span.start_point[0] + 1,
                        kind="packet-registration",
                    )

    def add_registration_surface(
        self,
        category: str,
        domain: str,
        mechanism_id: str,
        path: str,
        call_names: tuple[str, ...],
    ) -> None:
        source = self.source(path)
        calls = [call for call in source.calls if call.name in call_names]
        calls.sort(key=lambda item: item.span.start_byte)
        registration_evidence = REGISTRATION_OWNER_EVIDENCE.get(mechanism_id)
        for ordinal, call in enumerate(calls):
            detail = ",".join(argument.canonical for argument in call.arguments)
            owner = call.owner or source.package
            evidence: OwnerEvidence | None = None
            if registration_evidence is not None:
                registered = call.arguments[0].canonical if call.arguments else ""
                evidence = registration_evidence.get(registered)
                if evidence is None:
                    raise ClientMechanismInventoryError(
                        "registration owner lacks BBB implementation/test evidence: "
                        f"{owner}#{call.name}({detail})"
                    )
            self.add_owner(
                owner_id=f"registration:{category}:{owner}#{call.name}:{ordinal}",
                category=category,
                domain=domain,
                mechanism_id=mechanism_id,
                vanilla_owner=f"{owner}#{call.name}({detail})",
                path=path,
                line=call.span.start_point[0] + 1,
                kind="registration-call",
                evidence=evidence,
            )

    def add_entity_layers(self) -> None:
        root = self.source_root / "net/minecraft/client/renderer/entity"
        for source_path in sorted(root.rglob("*.java")):
            path = source_path.relative_to(self.source_root).as_posix()
            source = self.source(path)
            calls = sorted(
                (call for call in source.calls if call.name == "addLayer"),
                key=lambda item: item.span.start_byte,
            )
            for ordinal, call in enumerate(calls):
                detail = ",".join(argument.canonical for argument in call.arguments)
                owner = call.owner or source.package
                self.add_owner(
                    owner_id=f"registration:entity-layer:{owner}:{ordinal}",
                    category="entity-layer",
                    domain="entities",
                    mechanism_id="rendering-entity-layer-registration",
                    vanilla_owner=f"{owner}#addLayer({detail})",
                    path=path,
                    line=call.span.start_point[0] + 1,
                    kind="registration-call",
                )

    def add_builtin_registries(self) -> None:
        path = "net/minecraft/core/registries/BuiltInRegistries.java"
        source = self.source(path)
        for field in source.fields:
            if field.initializer is None:
                continue
            calls = [
                node
                for node in (field.initializer.node, *descendants(field.initializer.node))
                if node.type == "method_invocation"
            ]
            names = []
            for call in calls:
                name = call.child_by_field_name("name")
                if name is not None:
                    names.append(source.text(name))
            if not any(name.startswith("register") for name in names):
                continue
            self.add_owner(
                owner_id=f"registry:built-in:{field.name}",
                category="built-in-registry",
                domain="registries",
                mechanism_id="registries-built-in-catalogs",
                vanilla_owner=f"{field.owner}#{field.name}",
                path=path,
                line=field.span.start_point[0] + 1,
                kind="registry-declaration",
            )

    def add_synchronized_registries(self) -> None:
        path = "net/minecraft/resources/RegistryDataLoader.java"
        source = self.source(path)
        fields = [field for field in source.fields if field.name == "SYNCHRONIZED_REGISTRIES"]
        if len(fields) != 1 or fields[0].initializer is None:
            raise ClientMechanismInventoryError(f"{path}: expected SYNCHRONIZED_REGISTRIES initializer")
        initializer = fields[0].initializer
        list_calls = []
        for node in (initializer.node, *descendants(initializer.node)):
            if node.type != "method_invocation":
                continue
            name = node.child_by_field_name("name")
            if name is not None and source.text(name) == "of":
                list_calls.append(node)
        if len(list_calls) != 1:
            raise ClientMechanismInventoryError(f"{path}: expected one List.of synchronized registry call")
        arguments = list_calls[0].child_by_field_name("arguments")
        if arguments is None:
            raise ClientMechanismInventoryError(f"{path}: List.of has no arguments")
        for ordinal, argument in enumerate(arguments.named_children):
            detail = source.expression(argument).canonical
            self.add_owner(
                owner_id=f"registry:synchronized:{ordinal}:{detail}",
                category="synchronized-registry",
                domain="registries",
                mechanism_id="registries-synchronized-data",
                vanilla_owner=detail,
                path=path,
                line=argument.start_point.row + 1,
                kind="registry-data-entry",
            )

    def add_entity_types(self) -> None:
        path = "net/minecraft/world/entity/EntityType.java"
        source = self.source(path)
        for field in source.fields:
            if not field.type_spelling.replace(" ", "").startswith("EntityType<") or field.initializer is None:
                continue
            self.add_owner(
                owner_id=f"entity-type:{field.name}",
                category="entity-type",
                domain="entities",
                mechanism_id="entities-fixed-type-surface",
                vanilla_owner=f"{field.owner}#{field.name}",
                path=path,
                line=field.span.start_point[0] + 1,
                kind="entity-type-declaration",
            )

    @staticmethod
    def enclosing_method(source: JavaSource, start_byte: int) -> JavaMethod | None:
        candidates = [
            method
            for method in source.methods
            if method.span.start_byte <= start_byte < method.span.end_byte
        ]
        return min(candidates, key=lambda item: item.span.end_byte - item.span.start_byte) if candidates else None

    def add_serverbound_calls(self) -> None:
        for path in SERVERBOUND_SCAN_PATHS:
            source = self.source(path)
            for call in sorted(source.calls, key=lambda item: item.span.start_byte):
                packet_types: set[str] = set()
                for node in descendants(call.node):
                    if node.type not in {"identifier", "type_identifier", "scoped_identifier"}:
                        continue
                    spelling = source.text(node).split(".")[-1]
                    if spelling.startswith("Serverbound") and spelling.endswith("Packet"):
                        packet_types.add(spelling)
                if not packet_types:
                    continue
                method = self.enclosing_method(source, call.span.start_byte)
                method_owner = self.method_signature(method) if method is not None else (call.owner or source.package)
                for packet_type in sorted(packet_types):
                    self.add_owner(
                        owner_id=f"serverbound-call:{path}:{call.span.start_byte}:{packet_type}",
                        category="serverbound-call-site",
                        domain="protocol",
                        mechanism_id="protocol-serverbound-call-sites",
                        vanilla_owner=f"{method_owner} -> {packet_type}",
                        path=path,
                        line=call.span.start_point[0] + 1,
                        kind="packet-send-call",
                    )

    def finish(self) -> dict[str, object]:
        self.owners.sort(key=lambda item: str(item["owner_id"]))
        owner_ids = [str(item["owner_id"]) for item in self.owners]
        duplicates = sorted(owner_id for owner_id, count in Counter(owner_ids).items() if count != 1)
        if duplicates:
            raise ClientMechanismInventoryError(f"duplicate owner ids: {duplicates[:5]}")
        mechanisms = sorted({str(item["mechanism_id"]) for item in self.owners})
        missing_evidence = sorted(set(mechanisms) - set(MECHANISM_EVIDENCE))
        if missing_evidence:
            raise ClientMechanismInventoryError(f"mechanisms lack BBB evidence routes: {missing_evidence}")
        by_category = Counter(str(item["category"]) for item in self.owners)
        by_domain = Counter(str(item["domain"]) for item in self.owners)
        by_mechanism = Counter(str(item["mechanism_id"]) for item in self.owners)
        owner_evidence_by_mechanism = Counter(
            str(item["mechanism_id"])
            for item in self.owners
            if isinstance(item.get("bbb_evidence"), dict)
        )
        incomplete_owner_evidence = sorted(
            mechanism_id
            for mechanism_id in OWNER_EVIDENCE_MECHANISMS
            if owner_evidence_by_mechanism[mechanism_id] != by_mechanism[mechanism_id]
        )
        if incomplete_owner_evidence:
            raise ClientMechanismInventoryError(
                "mechanisms lack complete per-owner BBB evidence: "
                f"{incomplete_owner_evidence}"
            )
        by_role = Counter(
            str(item["role"])
            for item in self.owners
            if isinstance(item.get("role"), str)
        )
        return {
            "generated_by": "scripts/generate-client-mechanism-inventory.py",
            "generator_inputs": [
                {
                    "path": path,
                    "sha256": hashlib.sha256((REPO_ROOT / path).read_bytes()).hexdigest(),
                }
                for path in GENERATOR_INPUTS
            ],
            "mechanisms": {
                mechanism_id: {
                    "bbb_evidence": list(MECHANISM_EVIDENCE[mechanism_id]),
                    "owner_count": by_mechanism[mechanism_id],
                    **(
                        {"owner_evidence_count": owner_evidence_by_mechanism[mechanism_id]}
                        if mechanism_id in OWNER_EVIDENCE_MECHANISMS
                        else {}
                    ),
                }
                for mechanism_id in mechanisms
            },
            "owners": self.owners,
            "schema_version": 1,
            "selection_contract": {
                "client_level_method_partition": "explicit homogeneous routes plus CST-derived direct-return support owners; unclassified methods fail closed",
                "java_frontend": "scripts/vanilla_java_source.py",
                "level_renderer_method_partition": "explicit homogeneous routes; unclassified methods fail closed",
                "owner_evidence_mechanisms": list(OWNER_EVIDENCE_MECHANISMS),
                "minecraft_method_partition": "explicit decision/out-of-scope routes plus CST-derived direct-return support owners; unclassified methods fail closed",
                "policy": "bounded external-client decision, registration, listener, consumer, lifecycle, and explicit out-of-scope owner surfaces",
                "serverbound_scan_paths": list(SERVERBOUND_SCAN_PATHS),
            },
            "source_files": [
                {"path": path, "sha256": self.hashes[path]}
                for path in sorted(self.hashes)
            ],
            "summary": {
                "by_category": dict(sorted(by_category.items())),
                "by_domain": dict(sorted(by_domain.items())),
                "by_role": dict(sorted(by_role.items())),
                "mapped_owner_count": len(self.owners),
                "owner_count": len(self.owners),
                "unmapped_owner_count": 0,
            },
            "vanilla_version": "26.1",
        }


def build_inventory(source_root: Path) -> dict[str, object]:
    builder = InventoryBuilder(source_root)
    builder.add_packet_tables()
    builder.add_listener_contracts()
    builder.add_builtin_registries()
    builder.add_synchronized_registries()
    builder.add_entity_types()
    for spec in METHOD_SURFACES:
        builder.add_method_surface(spec)
    for surface in REGISTRATION_SURFACES:
        builder.add_registration_surface(*surface)
    builder.add_entity_layers()
    builder.add_serverbound_calls()
    return builder.finish()


def render_inventory(source_root: Path) -> str:
    return json.dumps(build_inventory(source_root), indent=2, sort_keys=True) + "\n"


def source_hashes(value: dict[str, object]) -> Iterable[tuple[str, str]]:
    for item in value.get("source_files", []):
        if isinstance(item, dict) and isinstance(item.get("path"), str) and isinstance(item.get("sha256"), str):
            yield item["path"], item["sha256"]
