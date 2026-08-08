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
import re
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
    selected_methods: tuple[str, ...] = ()


@dataclass(frozen=True)
class MethodRoute:
    category: str
    domain: str
    mechanism_id: str
    scope: str = "external-client"
    role: str = "decision"


@dataclass(frozen=True)
class OwnerScopeRuling:
    conclusion: str
    vanilla_basis: tuple[str, ...]
    current_scope_reason: str
    reopen_condition: str


@dataclass(frozen=True)
class OwnerOpenGap:
    ledger: str
    artifact: str
    reproduce_command: str
    reason: str
    close_condition: str


@dataclass(frozen=True)
class OwnerEvidence:
    implementation: tuple[str, ...]
    tests: tuple[str, ...]
    scope_ruling: OwnerScopeRuling | None = None


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
        MethodRoute("platform", "platform", "platform-window-chrome"),
        ("updateTitle", "createTitle"),
    ),
    (
        MethodRoute("platform", "platform", "platform-window-display-mode"),
        ("onFullscreenError",),
    ),
    (
        MethodRoute("platform", "platform", "platform-window-surface-state"),
        ("pauseIfInactive", "resizeGui", "cursorEntered", "isWindowActive"),
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
        MethodRoute("render-step", "rendering", "rendering-frame-order"),
        ("renderFrame",),
    ),
    (
        MethodRoute("render-step", "rendering", "rendering-extract-surface"),
        (
            "renderNames",
            "useShaderTransparency",
            "getEntityModels",
        ),
    ),
    (
        MethodRoute("render-step", "rendering", "rendering-level-pass"),
        (
            "getCameraEntity",
            "setCameraEntity",
            "shouldEntityAppearGlowing",
        ),
    ),
    (
        MethodRoute("render-step", "ui", "hud-frame-extract-order"),
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


MAIN_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("startup", "startup", "startup-client-bootstrap"),
        (
            "main",
            "getQuickPlayVariant",
            "unescapeJavaArgument",
            "ofNullable",
            "parseArgument",
            "stringHasValue",
        ),
    ),
    (
        MethodRoute("startup", "identity", "startup-session-identity"),
        (
            "emptyStringToEmptyOptional",
            "hasValidUuid",
            "isUuidValid",
        ),
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
            "processOptions",
            "processDumpedOptions",
            "process",
            "isTrue",
            "isFalse",
            "readListOfStrings",
        ),
    ),
    (
        MethodRoute("persistence", "persistence", "persistence-options-derived"),
        (
            "maxAnisotropyValue",
            "getCloudStatus",
            "getEffectiveRenderDistance",
            "setServerRenderDistance",
            "getFinalSoundSourceVolume",
            "getSoundSourceVolume",
            "getBackgroundOpacity",
            "getBackgroundColor",
            "getMenuBackgroundBlurriness",
            "japaneseGlyphVariantsDefault",
            "isModelPartEnabled",
            "setCameraType",
            "logMouse",
            "unlogMouse",
        ),
    ),
    (
        MethodRoute("persistence", "persistence", "persistence-options-apply"),
        (
            "buildPlayerInformation",
            "broadcastOptions",
            "dumpOptionsForReport",
            "getFullscreenVideoModeString",
            "updateFontOptions",
            "applyGraphicsPreset",
            "loadSelectedResourcePacks",
            "isSoundDeviceDefault",
        ),
    ),
    (
        MethodRoute("persistence", "persistence", "persistence-options-unconsumed"),
        ("dataFix",),
    ),
    (
        # Caption/label factories consumed only by `OptionsSubScreen` widgets,
        # plus the owners whose only caller is an options screen or an
        # `OptionInstance.onValueUpdate` lambda; the in-game options UI is a
        # declared non-goal.
        MethodRoute("out-of-scope", "options", "scope-options-ui", "out-of-scope", "out-of-scope"),
        (
            "pixelValueLabel",
            "percentValueLabel",
            "genericValueLabel",
            "genericValueOrOffLabel",
            "percentValueOrOffLabel",
            "getSoundSourceOptionInstance",
            "createSoundSliderOptionInstance",
            "setModelPart",
            "onboardingAccessibilityFinished",
            "setGraphicsPresetToCustom",
            "operateOnLevelRenderer",
            "updateResourcePacks",
        ),
    ),
)


# `ServerList` is the multiplayer screen's list model that also owns
# `servers.dat`. Only the file half has a command-line caller —
# `QuickPlay.joinMultiplayerWorld` loads the list, looks the address up, adds a
# hidden entry when it is new, and saves; `ClientPacketListener` then updates
# that entry through `saveSingleServer`. Everything else is reached only from
# `JoinMultiplayerScreen`/`ServerSelectionList`, and `replace` has no caller at
# all in 26.1.
SERVER_LIST_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("persistence", "persistence", "persistence-server-list"),
        ("ServerList", "load", "save", "saveSingleServer", "set", "add", "get"),
    ),
    (
        MethodRoute(
            "out-of-scope",
            "ui",
            "scope-multiplayer-server-list",
            "out-of-scope",
            "out-of-scope",
        ),
        ("size", "swap", "replace", "remove", "unhide"),
    ),
)


# `Gui` is one class holding every HUD layer, so it is split by the state each
# owner decides rather than by the frame it is drawn in. The seven direct field
# returns fall through to `DIRECT_SUPPORT_ROUTE`; anything else must be named
# here, so a new Vanilla HUD owner fails the build instead of landing silently.
GUI_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("render-step", "ui", "hud-frame-extract-order"),
        (
            "Gui",
            "extractRenderState",
            "extractHotbarAndDecorations",
            "extractCameraOverlays",
            "tick",
            "clearCache",
            "onDisconnected",
            "getCameraPlayer",
            "showOnlyReducedInfo",
        ),
    ),
    (
        MethodRoute("render-step", "ui", "hud-player-vitals"),
        (
            "extractPlayerHealth",
            "extractHearts",
            "extractHeart",
            "extractArmor",
            "extractFood",
            "extractAirBubbles",
            "extractVehicleHealth",
            "HeartType",
            "forPlayer",
            "getVehicleMaxHearts",
            "getVisibleVehicleHeartRows",
            "getPlayerVehicleWithHealth",
            "getAirBubbleYLine",
            "getCurrentAirSupplyBubble",
            "getEmptyBubbleDelayDuration",
            "playAirBubblePoppedSound",
            "willPrioritizeExperienceInfo",
            "willPrioritizeJumpInfo",
            "getSprite",
        ),
    ),
    (
        MethodRoute("render-step", "ui", "hud-hotbar-selection"),
        (
            "extractItemHotbar",
            "extractSlot",
            "extractSelectedItemName",
            "extractCrosshair",
            "canRenderCrosshairForSpectator",
            "nextContextualInfoState",
        ),
    ),
    (
        MethodRoute("render-step", "ui", "hud-overlay-effects"),
        (
            "extractVignette",
            "updateVignetteBrightness",
            "extractPortalOverlay",
            "extractConfusionOverlay",
            "extractSpyglassOverlay",
            "extractTextureOverlay",
            "extractSleepOverlay",
            "extractDemoOverlay",
            "extractEffects",
            "getMobEffectSprite",
        ),
    ),
    (
        MethodRoute("render-step", "ui", "hud-text-and-lists"),
        (
            "extractChat",
            "extractTitle",
            "setTitle",
            "setSubtitle",
            "setTimes",
            "clearTitles",
            "resetTitleTimes",
            "extractOverlayMessage",
            "setOverlayMessage",
            "setNowPlaying",
            "extractSubtitleOverlay",
            "extractDeferredSubtitles",
            "extractTabList",
            "extractScoreboardSidebar",
            "displayScoreboardSidebar",
            "extractBossOverlay",
            "extractDebugOverlay",
            "extractSavingIndicator",
            "tickAutosaveIndicator",
        ),
    ),
)


# `KeyboardHandler` mixes the raw GLFW key/char/preedit routing with the F3
# debug-key action surface and the clipboard bridge, so it is split by the state
# each owner decides. Nothing here is a direct field return, so every method must
# be named and a new Vanilla owner fails the build instead of landing silently.
GAME_RENDERER_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("render-step", "rendering", "rendering-frame-order"),
        (
            "GameRenderer",
            "close",
            "tick",
            "update",
            "extract",
            "render",
            "resize",
            "resetData",
            "setLevel",
        ),
    ),
    (
        MethodRoute("render-step", "rendering", "rendering-extract-surface"),
        (
            "extractWindow",
            "extractOptions",
            "extractCamera",
            "extractGui",
            "renderActiveTextDebug",
            "getGameRenderState",
            "getSubmitNodeStorage",
            "getFeatureRenderDispatcher",
        ),
    ),
    (
        MethodRoute("render-step", "rendering", "rendering-level-pass"),
        (
            "renderLevel",
            "renderItemInHand",
            "bobHurt",
            "bobView",
            "shouldRenderBlockOutline",
            "setRenderBlockOutline",
            "getNightVisionScale",
            "getBossOverlayWorldDarkening",
            "projectPointToScreen",
            "projectHorizonToScreen",
            "getMainCamera",
        ),
    ),
    (
        MethodRoute("render-step", "rendering", "rendering-effects-screenshot"),
        (
            "checkEntityPostEffect",
            "setPostEffect",
            "clearPostEffect",
            "togglePostEffect",
            "currentPostEffect",
            "processBlurEffect",
            "displayItemActivation",
            "tryTakeScreenshotIfNeeded",
            "takeAutoScreenshot",
            "preloadUiShader",
            "lightmap",
            "levelLightmap",
            "overlayTexture",
            "getGlobalSettingsUniform",
            "getLighting",
        ),
    ),
    (
        MethodRoute("resource-consumer", "resources", "resources-indexed-panorama"),
        ("getPanorama", "registerPanoramaTextures"),
    ),
)


KEYBOARD_HANDLER_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("input", "input", "input-keyboard-routing"),
        (
            "KeyboardHandler",
            "setup",
            "keyPress",
            "charTyped",
            "preeditCallback",
            "submitPreeditEvent",
            "resubmitLastPreeditEvent",
        ),
    ),
    (
        MethodRoute("input", "input", "input-debug-key-actions"),
        (
            "handleDebugKeys",
            "handleChunkDebugKeys",
            "tick",
            "copyRecreateCommand",
            "copyCreateBlockCommand",
            "copyCreateEntityCommand",
            "debugFeedback",
            "debugFeedbackComponent",
            "debugFeedbackEnabledStatus",
            "debugFeedbackTranslated",
            "debugWarningComponent",
            "decorateDebugComponent",
            "showDebugChat",
        ),
    ),
    (
        MethodRoute("input", "accessibility", "ui-clipboard-surface"),
        ("getClipboard", "setClipboard"),
    ),
)


# `ClipboardManager` is the platform half of the same clipboard bridge.
CLIPBOARD_MANAGER_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("accessibility", "accessibility", "ui-clipboard-surface"),
        ("getClipboard", "setClipboard", "pushClipboard"),
    ),
)


# `Window` owns creation, display mode, surface state and window chrome in one
# class, so it is split by the state each owner decides. The 17 direct field
# returns fall through to `DIRECT_SUPPORT_ROUTE`.
WINDOW_METHOD_ROUTES = _method_routes(
    (
        MethodRoute("platform", "platform", "platform-window-lifecycle"),
        (
            "Window",
            "createWindow",
            "createGlfwWindow",
            "allowedWindowMinSize",
            "close",
            "shouldClose",
            "setWindowCloseCallback",
            "bootCrash",
            "checkGlfwError",
            "defaultErrorCallback",
            "setBootErrorCallback",
            "setDefaultErrorCallback",
            "setErrorSection",
            "WindowInitFailed",
            "getPlatform",
            "findBestMonitor",
        ),
    ),
    (
        MethodRoute("platform", "platform", "platform-window-display-mode"),
        (
            "setMode",
            "toggleFullScreen",
            "setWindowed",
            "updateFullscreen",
            "updateFullscreenIfChanged",
            "changeFullscreenVideoMode",
            "setPreferredFullscreenVideoMode",
            "updateVsync",
            "updateRawMouseInput",
            "getRefreshRate",
        ),
    ),
    (
        MethodRoute("platform", "platform", "platform-window-surface-state"),
        (
            "onResize",
            "onFramebufferResize",
            "refreshFramebufferSize",
            "resetIsResized",
            "setWidth",
            "setHeight",
            "calculateScale",
            "setGuiScale",
            "getAppropriateLineWidth",
            "onMove",
            "onIconify",
            "onFocus",
            "onEnter",
        ),
    ),
    (
        MethodRoute("platform", "platform", "platform-window-chrome"),
        ("setTitle", "selectCursor", "setAllowCursorChanges"),
    ),
    (
        MethodRoute("resource-consumer", "resources", "resources-window-icons"),
        ("setIcon",),
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


CLIENT_PACK_SOURCE_METHOD_ROUTES = _method_routes(
    (
        MethodRoute(
            "resource-provider",
            "resources",
            "resources-indexed-vanilla-provider",
        ),
        (
            "ClientPackSource",
            "createVanillaPackSource",
            "createVanillaPack",
        ),
    ),
)


def _owner_evidence(
    *groups: tuple[OwnerEvidence | OwnerOpenGap, tuple[str, ...]],
) -> dict[str, OwnerEvidence | OwnerOpenGap]:
    evidence: dict[str, OwnerEvidence | OwnerOpenGap] = {}
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
            (
                "crates/bbb-world/src/terrain.rs#pub(crate) fn mark_section_from_player",
                "crates/bbb-world/src/chunks/store.rs#fn mark_terrain_block_changed",
                "crates/bbb-native/src/terrain_runtime.rs#queue_terrain_chunk_mesh",
            ),
            ("crates/bbb-native/src/terrain_runtime.rs#prioritize_chunk_updates_matches_vanilla_sync_decision",),
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
                "crates/bbb-renderer/src/terrain/mesh/tests.rs#ambient_cubic_face_offset_samples_neighbor_brightness_and_state",
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

OPTIONS_CATALOG_IMPLEMENTATION = (
    "crates/bbb-native/src/client_options.rs#CLIENT_OPTIONS",
    "scripts/client_options_catalog_extractor.py#client_option_entries",
    "scripts/generate-client-options-catalog.py#render_rust_catalog",
)

OPTIONS_FILE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            OPTIONS_CATALOG_IMPLEMENTATION
            + ("crates/bbb-native/src/client_options.rs#default_text",),
            (
                "crates/bbb-native/src/client_options.rs"
                "#defaults_carry_the_text_vanilla_writes_for_every_catalogued_key",
                "crates/bbb-native/src/client_options.rs"
                "#host_derived_defaults_follow_the_host",
                "scripts/tests/test_client_options_catalog_extractor.py"
                "#test_committed_catalog_matches_the_vanilla_source",
            ),
        ),
        ("Options",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/client_options.rs#set_raw",),
            (
                "crates/bbb-native/src/client_options.rs"
                "#a_line_splits_on_its_first_colon_only_and_bad_lines_are_skipped",
                "crates/bbb-native/src/client_options.rs"
                "#a_missing_file_leaves_the_defaults_in_place",
            ),
        ),
        ("load",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options.rs#rendered",
                "crates/bbb-native/src/client_options.rs#pub(crate) fn apply(",
            ),
            (
                "crates/bbb-native/src/client_options.rs"
                "#save_writes_every_catalogued_key_in_vanilla_order",
                "crates/bbb-native/src/client_options.rs"
                "#save_round_trips_and_drops_keys_this_client_does_not_know",
                "crates/bbb-native/src/client_options.rs"
                "#recipe_book_language_write_stores_the_bare_language_code",
                "crates/bbb-native/src/simulation/input_commands.rs"
                "#recipe_book_language_transition_reloads_saves_and_broadcasts_in_order",
                "crates/bbb-native/src/main.rs"
                "#client_options_save_completion_follows_missing_or_failed_file_owner",
            ),
        ),
        ("save",),
    ),
    (
        # `process`/`isTrue`/`isFalse`/`readListOfStrings` are the overload set
        # that decides how one line is written and read back; BBB resolves the
        # same choice from the generated per-key encoding.
        OwnerEvidence(
            OPTIONS_CATALOG_IMPLEMENTATION
            + ("crates/bbb-native/src/client_options/args.rs#bind",),
            (
                "crates/bbb-native/src/client_options.rs"
                "#every_encoding_parses_its_own_default",
                "scripts/tests/test_client_options_catalog_extractor.py"
                "#test_catalog_covers_every_options_txt_line",
                "crates/bbb-native/src/client_options/args.rs"
                "#binding_a_key_through_the_wrong_encoding_fails_closed",
            ),
        ),
        ("processOptions", "processDumpedOptions", "process", "isTrue", "isFalse"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/client_options.rs#TextList",),
            (
                "crates/bbb-native/src/client_options.rs"
                "#every_encoding_parses_its_own_default",
            ),
        ),
        ("readListOfStrings",),
    ),
)


OPTIONS_DATAFIX_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options.rs#ClientOptions::load",
                "crates/bbb-native/src/client_options/datafix.rs#update_to_current_version",
                "crates/bbb-native/src/client_options/datafix.rs#apply_fix",
                "crates/bbb-native/src/client_options/datafix_26_1.rs#CLIENT_OPTIONS_DATA_FIXES",
                "scripts/client_options_datafix_extractor.py#client_options_datafix_catalog",
                "scripts/generate-client-options-catalog.py#render_rust_datafix",
            ),
            (
                "crates/bbb-native/src/client_options/datafix.rs"
                "#complete_version_zero_chain_applies_in_registration_order",
                "crates/bbb-native/src/client_options/datafix.rs"
                "#each_schema_boundary_is_exclusive_of_its_own_fixes",
                "crates/bbb-native/src/client_options.rs"
                "#old_options_are_data_fixed_before_current_fields_are_decoded",
                "crates/bbb-native/src/client_options.rs"
                "#a_throwing_legacy_key_fix_retains_all_defaults_like_options_load",
                "scripts/tests/test_client_options_catalog_extractor.py"
                "#test_datafix_catalog_covers_every_registered_options_fix",
            ),
        ),
        ("dataFix",),
    ),
)


OPTIONS_DERIVED_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/renderer.rs#fn effective_terrain_max_anisotropy",),
            (
                "crates/bbb-renderer/src/gpu.rs"
                "#anisotropic_filtering_keeps_the_nearest_shader_and_scales_the_sampler",
            ),
        ),
        ("maxAnisotropyValue",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/renderer.rs#pub fn set_cloud_status",
                "crates/bbb-native/src/startup.rs#fn parse_stored_render_clouds",
            ),
            (
                "crates/bbb-renderer/src/render.rs#clouds_off_exits_the_pass_before_any_encoding",
                "crates/bbb-native/src/startup.rs#stored_render_clouds_reads_legacy_names_and_booleans",
            ),
        ),
        ("getCloudStatus",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options/derived.rs#effective_render_distance",
                "crates/bbb-native/src/runtime.rs#update_renderer_from_publication",
            ),
            (
                "crates/bbb-native/src/client_options/derived.rs"
                "#the_server_radius_only_clamps_when_it_is_set_and_smaller",
            ),
        ),
        ("getEffectiveRenderDistance",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/chunks/store.rs#apply_set_chunk_cache_radius",),
            ("crates/bbb-world/src/chunks/store.rs#chunk_cache_radius",),
        ),
        ("setServerRenderDistance",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-audio/src/command.rs#final_source_volume",),
            (
                "crates/bbb-audio/src/command.rs"
                "#final_source_volume_matches_vanilla_master_and_category_mix",
            ),
        ),
        ("getFinalSoundSourceVolume", "getSoundSourceVolume"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/startup.rs#parse_text_background_opacity",),
            (
                "crates/bbb-native/src/startup.rs"
                "#args_reject_text_background_opacity_outside_unit_range",
            ),
        ),
        ("getBackgroundOpacity", "getBackgroundColor"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/screen_blur.rs#sanitize_menu_background_blurriness",),
            (
                "crates/bbb-native/src/startup.rs"
                "#args_accept_vanilla_menu_background_blurriness_range",
                "crates/bbb-native/src/startup.rs"
                "#args_reject_menu_background_blurriness_outside_vanilla_range",
            ),
        ),
        ("getMenuBackgroundBlurriness",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options.rs#LocaleIsJapanese",
                "crates/bbb-native/src/client_options.rs#locale_is_japanese",
            ),
            (
                "crates/bbb-native/src/client_options.rs#host_derived_defaults_follow_the_host",
                "crates/bbb-native/src/client_options.rs"
                "#japanese_font_default_uses_only_the_system_language_subtag",
            ),
        ),
        ("japaneseGlyphVariantsDefault",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/startup.rs#client_information_from_args",),
            (
                "crates/bbb-native/src/startup.rs"
                "#args_build_client_information_from_startup_options",
            ),
        ),
        ("isModelPartEnabled",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#cycle_camera_type",),
            ("crates/bbb-native/src/input.rs#camera_type",),
        ),
        ("setCameraType",),
    ),
    (
        # The log/unlog pair is only the `sensitivity` codec, so the generated
        # encoding is what has to round-trip it.
        OwnerEvidence(
            OPTIONS_CATALOG_IMPLEMENTATION,
            (
                "crates/bbb-native/src/client_options.rs"
                "#every_encoding_parses_its_own_default",
            ),
        ),
        ("logMouse", "unlogMouse"),
    ),
)


OPTIONS_APPLY_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-native/src/startup.rs#client_information_from_args",),
            (
                "crates/bbb-native/src/startup.rs"
                "#args_build_client_information_from_startup_options",
                "crates/bbb-native/src/startup.rs"
                "#stored_options_fill_arguments_the_user_did_not_type",
            ),
        ),
        ("buildPlayerInformation",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/commands.rs"
                "#queue_broadcast_client_information_command",
                "crates/bbb-net/src/driver/commands.rs#send_play_client_information",
                "crates/bbb-native/src/simulation/input_commands.rs"
                "#apply_client_options_save_completed",
            ),
            (
                "crates/bbb-net/src/driver/commands/tests.rs"
                "#broadcast_client_information_keeps_each_phase_own_packet_id",
                "crates/bbb-native/src/input/tests.rs"
                "#focus_pause_toggle_queues_the_option_line_vanilla_saves",
                "crates/bbb-native/src/simulation/input_commands.rs"
                "#recipe_book_language_transition_reloads_saves_and_broadcasts_in_order",
                "crates/bbb-native/src/main.rs"
                "#client_options_save_completion_follows_missing_or_failed_file_owner",
            ),
        ),
        ("broadcastOptions",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/client_options.rs#dump_for_report",),
            (
                "crates/bbb-native/src/client_options.rs"
                "#the_report_dump_covers_the_dumped_block_and_the_keys_it_names",
                "crates/bbb-native/src/crash_report.rs"
                "#the_options_captured_at_startup_reach_the_report",
            ),
        ),
        ("dumpOptionsForReport",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options.rs#FULLSCREEN_RESOLUTION_KEY",
                "crates/bbb-native/src/startup.rs#parse_client_fullscreen_resolution",
                "crates/bbb-native/src/main.rs#fn set_window_fullscreen",
                "crates/bbb-native/src/window_surface.rs#find_exact_video_mode",
            ),
            (
                "crates/bbb-native/src/client_options.rs"
                "#the_fullscreen_resolution_line_is_retained_across_load_and_save",
                "crates/bbb-native/src/startup.rs"
                "#a_typed_fullscreen_resolution_wins_over_the_stored_line",
                "crates/bbb-native/src/window_surface.rs"
                "#video_mode_facts_exact_match_includes_refresh_and_total_bits",
                "crates/bbb-native/src/main.rs"
                "#fullscreen_transitions_share_one_deterministic_mode_policy",
            ),
        ),
        ("getFullscreenVideoModeString",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-item-model/src/font/providers.rs#pub struct FontOptions",
                "crates/bbb-item-model/src/font/providers.rs#fn provider_enabled",
                "crates/bbb-native/src/startup.rs#client_japanese_glyph_variants",
                "crates/bbb-native/src/resource_reload.rs#font_options: FontOptions",
            ),
            (
                "crates/bbb-item-model/src/font/providers.rs"
                "#font_options_filter_providers_before_reference_expansion",
                "crates/bbb-native/src/startup.rs"
                "#the_stored_window_family_options_feed_the_window_owner",
                "crates/bbb-native/src/resource_reload.rs"
                "#renderer_resource_owners_commit_inside_one_reload_barrier",
            ),
        ),
        ("updateFontOptions",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/startup.rs#fn apply_graphics_preset",
                "crates/bbb-world/src/terrain.rs#pub(crate) fn mark_section_from_player",
                "crates/bbb-native/src/terrain_runtime.rs#fn section_rebuilds_synchronously",
                "crates/bbb-native/src/terrain_runtime/textures.rs#pub(crate) fn with_max_mipmap_levels",
                "crates/bbb-native/src/entity_scene.rs#fn entity_renderer_should_render_with_scaling",
                "crates/bbb-native/src/runtime/render_extract.rs#pub(crate) fn weather_render_state_for_world_with_radius",
                "crates/bbb-renderer/src/renderer.rs#fn resolve_fabulous_terrain_texture_filtering",
            ),
            (
                "crates/bbb-native/src/startup.rs#stored_graphics_presets_overwrite_every_owned_value_after_load",
                "crates/bbb-native/src/terrain_runtime.rs#prioritize_chunk_updates_matches_vanilla_sync_decision",
                "crates/bbb-native/src/terrain_runtime/textures/tests.rs#terrain_generation_retains_the_selected_mipmap_limit_for_reload",
                "crates/bbb-native/src/biome_tint.rs#configured_biome_blend_radius_controls_window_geometry",
                "crates/bbb-native/src/entity_scene/tests.rs#entity_distance_scaling_multiplies_the_shared_vanilla_view_scale",
                "crates/bbb-native/src/runtime/tests.rs#weather_tick_uses_the_selected_graphics_radius_for_sampling",
                "crates/bbb-renderer/src/renderer.rs#fabulous_filtering_uses_the_selected_adapter_renderer_like_vanilla",
            ),
        ),
        ("applyGraphicsPreset",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-pack/src/selected_packs.rs#pub fn from_options_repository",
                "crates/bbb-native/src/startup.rs#fn stored_text_list",
                "crates/bbb-native/src/startup.rs#pub(crate) fn load_pack_roots_with_options",
                "crates/bbb-native/src/client_options.rs#apply_resource_pack_reconciliation",
            ),
            (
                "crates/bbb-pack/src/selected_packs.rs#options_repository_reconciles_missing_compatibility_legacy_ids_and_order",
                "crates/bbb-native/src/startup.rs#stored_resource_pack_lists_reconcile_the_game_directory_repository",
                "crates/bbb-native/src/client_options.rs#a_later_hotkey_save_uses_the_reconciled_resource_pack_lists",
            ),
        ),
        ("loadSelectedResourcePacks",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/device.rs#pub fn is_sound_device_default",
                "crates/bbb-native/src/startup.rs#parse_stored_sound_device",
                "crates/bbb-audio/src/runtime.rs#pub fn new_with_device_preference",
                "crates/bbb-native/src/audio_runtime.rs#pub(crate) fn load",
            ),
            (
                "crates/bbb-audio/src/device.rs#only_the_empty_name_selects_the_default_device",
                "crates/bbb-audio/src/device.rs#reload_decision_preserves_vanilla_branch_order",
                "crates/bbb-native/src/startup.rs"
                "#stored_sound_device_decodes_codec_string_and_explicit_cli_wins",
                "crates/bbb-native/src/audio_runtime.rs"
                "#device_reload_invalidates_backend_tickable_identity_only_after_success",
            ),
        ),
        ("isSoundDeviceDefault",),
    ),
)


SERVER_LIST_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-native/src/server_list.rs#ServerListPersistence",),
            (
                "crates/bbb-native/src/startup.rs"
                "#server_list_owner_requires_explicit_store_name_and_live_connection",
            ),
        ),
        ("ServerList",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/server_list.rs#ServerListFile",
                "crates/bbb-native/src/server_list.rs#MAX_SERVER_LIST_FILE_BYTES",
            ),
            (
                "crates/bbb-native/src/server_list.rs"
                "#update_preserves_other_entries_unknown_fields_and_omitted_icon",
            ),
        ),
        ("load",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/server_list.rs#safely_replace",),
            (
                "crates/bbb-native/src/server_list.rs"
                "#update_preserves_other_entries_unknown_fields_and_omitted_icon",
            ),
        ),
        ("save",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/server_list.rs#persist_current_server_data",
                "crates/bbb-native/src/server_list.rs#save_server_icon",
            ),
            (
                "crates/bbb-native/src/server_list.rs"
                "#configured_owner_persists_valid_icon_in_vanilla_servers_dat_shape",
                "crates/bbb-native/src/runtime/events/tests/session_hud_and_vehicle_events.rs"
                "#server_data_event_persists_configured_vanilla_server_list_owner",
            ),
        ),
        ("saveSingleServer",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/server_list.rs#find_or_insert",),
            (
                "crates/bbb-native/src/server_list.rs"
                "#an_entry_the_player_already_named_for_the_address_is_reused",
            ),
        ),
        ("set", "get"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/server_list.rs#entry_is_hidden",
                "crates/bbb-native/src/server_list.rs#MAX_HIDDEN_SERVERS",
            ),
            (
                "crates/bbb-native/src/server_list.rs"
                "#a_created_entry_heads_the_hidden_block_and_trims_it_to_sixteen",
            ),
        ),
        ("add",),
    ),
)


HUD_FRAME_EXTRACT_ORDER_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#HudRenderRuntime",
                "crates/bbb-world/src/client/hud.rs#HudTitleState",
            ),
            ("crates/bbb-world/src/client/hud.rs#title_packets_match_vanilla_timing_rules",),
        ),
        ("Gui",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#hud_level_loading_screen",
                "crates/bbb-renderer/src/hud.rs#collect_hud_draws",
            ),
            (
                "crates/bbb-renderer/src/hud.rs#the_level_loading_screen_replaces_every_hud_draw_and_centers_the_status_line",
                "crates/bbb-native/src/runtime/tests.rs#renderer_frame_hud_extracts_after_input_and_use_item_tick",
            ),
        ),
        ("extractRenderState",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime/render_extract.rs#hud_hotbar_mode_from_world",),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_spectator_projection_preserves_ordinary_and_hidden_hotbar_modes",
                "crates/bbb-native/src/runtime/tests.rs#a_non_player_camera_entity_hides_the_ordinary_item_hotbar",
            ),
        ),
        ("extractHotbarAndDecorations",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hud_camera_overlays",),
            (
                "crates/bbb-native/src/runtime/tests.rs#camera_overlays_split_first_person_equippables_from_the_freeze_outline",
                "crates/bbb-renderer/src/hud.rs#camera_overlays_blit_full_screen_in_order_and_skip_unknown_textures",
            ),
        ),
        ("extractCameraOverlays",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/hud.rs#advance_hud_text_ticks",),
            (
                "crates/bbb-world/src/client/hud.rs#hud_text_ticks_count_down_like_vanilla_gui_tick",
                "crates/bbb-world/src/client/hud.rs#hud_text_tick_zero_and_partial_advances_keep_title_visible",
            ),
        ),
        ("tick",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/level.rs#clear_level_bound_state",),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_debug_overlay_projects_custom_chunk_render_stats_under_reduced_debug_info",
            ),
        ),
        ("clearCache",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/hud.rs#clear_hud_for_level_teardown",),
            (
                "crates/bbb-world/src/client/hud.rs#level_teardown_resets_the_tab_list_boss_bars_and_titles_like_gui_on_disconnected",
            ),
        ),
        ("onDisconnected",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#hud_camera_player_id",
                "crates/bbb-world/src/client/hud_camera_player.rs#hud_camera_player_vitals",
            ),
            (
                "crates/bbb-world/src/client/hud_camera_player.rs#the_camera_player_defaults_to_the_local_player_until_set_camera_retargets_it",
                "crates/bbb-world/src/client/hud_camera_player.rs#a_remote_camera_player_projects_its_own_synced_vitals_and_vanilla_food_defaults",
            ),
        ),
        ("getCameraPlayer",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#show_only_reduced_info",),
            (
                "crates/bbb-native/src/input/tests.rs#show_only_reduced_info_ors_the_reduced_debug_info_option_with_the_player_flag",
            ),
        ),
        ("showOnlyReducedInfo",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hud_debug_overlay",),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_debug_overlay_projects_version_and_camera_position_lines",
            ),
        ),
        ("getDebugOverlay",),
    ),
)


HUD_PLAYER_VITALS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#player_health_frame",),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_heart_health_frame_matches_vanilla_damage_hold_and_blink_phase",
                "crates/bbb-native/src/runtime/tests.rs#hud_heart_health_frame_uses_ten_tick_heal_and_invulnerability_gate",
            ),
        ),
        ("extractPlayerHealth",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud/layout.rs#hud_player_heart_instances",),
            (
                "crates/bbb-renderer/src/hud/layout.rs#hud_player_heart_instances_order_damage_and_heal_blink_strata",
                "crates/bbb-renderer/src/hud/layout.rs#hud_player_heart_instances_append_absorption_hearts",
            ),
        ),
        ("extractHearts",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud.rs#hud_heart_sprite",),
            (
                "crates/bbb-renderer/src/hud.rs#hud_heart_sprite_names_match_vanilla_assets",
                "crates/bbb-renderer/src/hud.rs#player_heart_damage_blink_offscreen_frame_consumes_ordered_sprite_variants",
            ),
        ),
        ("extractHeart", "getSprite"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#camera_player_heart_type",
                "crates/bbb-world/src/client/hud_camera_player.rs#fully_frozen",
            ),
            (
                "crates/bbb-renderer/src/hud.rs#poison_heart_offscreen_frame_uses_the_poisoned_sprite",
                "crates/bbb-renderer/src/hud/layout.rs#hud_player_heart_instances_follow_the_base_heart_type",
            ),
        ),
        ("HeartType", "forPlayer"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/local_player.rs#player_armor_value",),
            ("crates/bbb-renderer/src/hud.rs#armor_bar_offscreen_frame_draws_only_when_armor_is_positive",),
        ),
        ("extractArmor",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud.rs#hud_food_sprite_variant",),
            (
                "crates/bbb-renderer/src/hud.rs#food_sprite_variant_prefers_the_hunger_icon_only_under_the_effect",
                "crates/bbb-renderer/src/hud/layout.rs#hud_layout_places_food_icons_above_hotbar",
            ),
        ),
        ("extractFood",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#player_air_supply",
                "crates/bbb-renderer/src/hud/layout.rs#hud_air_bubble_icons",
            ),
            (
                "crates/bbb-renderer/src/hud.rs#air_bubbles_offscreen_frame_draw_only_underwater_or_below_max",
                "crates/bbb-renderer/src/hud/layout.rs#hud_air_bubble_icons_mirror_the_vanilla_bubble_formulas",
            ),
        ),
        ("extractAirBubbles",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud/layout.rs#hud_air_bubbles_visible",),
            ("crates/bbb-renderer/src/hud/layout.rs#hud_air_bubbles_visible_only_underwater_or_below_max",),
        ),
        ("getCurrentAirSupplyBubble", "getEmptyBubbleDelayDuration"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud/layout.rs#air_bubble_hud_rect",),
            ("crates/bbb-renderer/src/hud/layout.rs#hud_layout_places_air_bubbles_one_row_above_the_food_row",),
        ),
        ("getAirBubbleYLine",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#air_bubble_pop_sound",
                "crates/bbb-renderer/src/hud/layout.rs#hud_air_bubble_pop",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_air_bubble_pop_sound_matches_vanilla_dedup_reset_and_gain_formulas",
                "crates/bbb-renderer/src/hud/layout.rs#hud_air_bubble_pop_exposes_the_sound_inputs_from_the_same_formulas",
            ),
        ),
        ("playAirBubblePoppedSound",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/entities.rs#player_vehicle_health",),
            (
                "crates/bbb-renderer/src/hud.rs#vehicle_hearts_offscreen_frame_replace_the_food_row",
                "crates/bbb-renderer/src/hud/layout.rs#hud_layout_places_vehicle_hearts_on_the_food_row",
            ),
        ),
        ("extractVehicleHealth", "getPlayerVehicleWithHealth"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/hud/layout.rs#hud_vehicle_max_hearts",
                "crates/bbb-renderer/src/hud/layout.rs#vehicle_heart_hud_rect",
            ),
            (
                "crates/bbb-renderer/src/hud/layout.rs#hud_vehicle_max_hearts_rounds_and_caps_like_the_gui",
                "crates/bbb-renderer/src/hud/layout.rs#hud_vehicle_heart_fill_splits_rows_on_the_20_half_heart_base",
            ),
        ),
        ("getVehicleMaxHearts", "getVisibleVehicleHeartRows"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hud_jump_bar",),
            ("crates/bbb-renderer/src/hud.rs#contextual_bar_jump_replaces_experience_and_uses_cooldown_overlay",),
        ),
        ("willPrioritizeExperienceInfo", "willPrioritizeJumpInfo"),
    ),
)


HUD_HOTBAR_SELECTION_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hotbar_item_icons_with_input_context",),
            (
                "crates/bbb-native/src/runtime/tests.rs#a_remote_camera_player_keeps_the_hotbar_frame_but_projects_no_local_inventory",
                "crates/bbb-renderer/src/hud/layout.rs#hud_layout_matches_vanilla_hotbar_positions",
            ),
        ),
        ("extractItemHotbar",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud/layout.rs#hud_item_pop_placement",),
            (
                "crates/bbb-renderer/src/hud.rs#the_slot_pop_pose_moves_the_item_quad_but_never_its_decorations",
                "crates/bbb-renderer/src/hud/layout.rs#hud_layout_matches_vanilla_hotbar_item_positions",
            ),
        ),
        ("extractSlot",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#selected_item_name_frame",),
            (
                "crates/bbb-native/src/runtime/tests.rs#the_selected_item_name_timer_restarts_on_a_new_item_and_counts_down_to_zero",
                "crates/bbb-renderer/src/hud.rs#selected_item_name_draw_centers_above_the_hotbar_and_drops_without_hearts",
            ),
        ),
        ("extractSelectedItemName",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hud_crosshair_visible",),
            (
                "crates/bbb-renderer/src/hud.rs#crosshair_visibility_gate_retains_the_uploaded_sprite",
                "crates/bbb-renderer/src/hud.rs#crosshair_attack_indicator_draws_after_crosshair_and_obeys_debug_replacement",
            ),
        ),
        ("extractCrosshair",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/crosshair.rs#can_render_crosshair_for_spectator",),
            (
                "crates/bbb-native/src/crosshair.rs#spectator_mode_only_outlines_and_crosshairs_menu_provider_blocks",
                "crates/bbb-native/src/crosshair.rs#spectator_crosshair_follows_the_hit_entity_menu_provider",
            ),
        ),
        ("canRenderCrosshairForSpectator",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hud_contextual_bar",),
            ("crates/bbb-renderer/src/hud.rs#contextual_bar_jump_replaces_experience_and_uses_cooldown_overlay",),
        ),
        ("nextContextualInfoState",),
    ),
)


HUD_OVERLAY_EFFECTS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime/vignette.rs#VignetteRuntime",),
            (
                "crates/bbb-native/src/runtime/vignette.rs#the_vignette_option_hides_the_draw_while_the_tick_state_keeps_smoothing",
                "crates/bbb-renderer/src/vignette.rs#vignette_pipeline_matches_vanilla_blend_and_blurred_texture_sampling",
            ),
        ),
        ("extractVignette",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime/vignette.rs#vignette_target_brightness",),
            (
                "crates/bbb-native/src/runtime/vignette.rs#vignette_brightness_curve_and_tick_smoothing_match_vanilla",
                "crates/bbb-native/src/runtime/vignette.rs#vignette_runtime_keeps_brightness_without_camera_and_ticks_while_hidden",
            ),
        ),
        ("updateVignetteBrightness",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hud_portal_overlay",),
            ("crates/bbb-renderer/src/hud.rs#the_portal_overlay_eases_partial_intensity_and_samples_the_blocks_atlas",),
        ),
        ("extractPortalOverlay",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hud_confusion_overlay",),
            (
                "crates/bbb-renderer/src/hud.rs#the_confusion_overlay_scales_about_the_centre_and_blends_additively",
                "crates/bbb-native/src/runtime/tests.rs#the_confusion_overlay_needs_nausea_and_a_reduced_screen_effect_scale",
            ),
        ),
        ("extractConfusionOverlay",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud.rs#push_hud_spyglass_scope",),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_spyglass_scope_matches_vanilla_frame_interpolation_and_visibility",
                "crates/bbb-renderer/src/hud.rs#spyglass_scope_draws_texture_and_masks_before_crosshair_and_obeys_hide_gui",
            ),
        ),
        ("extractSpyglassOverlay",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hud_camera_overlays",),
            (
                "crates/bbb-renderer/src/hud.rs#camera_overlays_blit_full_screen_in_order_and_skip_unknown_textures",
                "crates/bbb-native/src/runtime/tests.rs#camera_overlays_split_first_person_equippables_from_the_freeze_outline",
            ),
        ),
        ("extractTextureOverlay",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud.rs#push_hud_sleep_overlay",),
            (
                "crates/bbb-native/src/runtime/tests.rs#animate_player_wake_sleep_overlay_frame_wiring_is_hide_gui_independent",
                "crates/bbb-renderer/src/hud.rs#sleep_overlay_submits_after_boss_bars_and_before_later_hud_content",
            ),
        ),
        ("extractSleepOverlay",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#HudDemoPopupScreen",),
            ("crates/bbb-native/src/runtime/tests.rs#hud_demo_popup_projects_exact_fallback_copy_and_initial_focus",),
        ),
        ("extractDemoOverlay",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hud_mob_effects_from_world",),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_mob_effect_projection_uses_catalog_category_flags_level_and_duration",
                "crates/bbb-renderer/src/hud.rs#hud_mob_effect_overlay_matches_vanilla_order_rows_and_icon_alpha",
            ),
        ),
        ("extractEffects",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud.rs#hud_mob_effect_overlay_instances",),
            ("crates/bbb-native/src/runtime/tests.rs#hud_mob_effect_projection_requires_local_player_and_formats_vanilla_time_ranges",),
        ),
        ("getMobEffectSprite",),
    ),
)


HUD_TEXT_AND_LISTS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime/render_extract.rs#hud_chat_messages_from_world",),
            (
                "crates/bbb-renderer/src/hud.rs#chat_alpha_matches_vanilla_focus_and_two_hundred_tick_fade",
                "crates/bbb-native/src/runtime/tests.rs#chat_render_cache_rebuilds_only_for_display_revision_or_level",
            ),
        ),
        ("extractChat",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime/render_extract.rs#hud_title_text_from_world",),
            (
                "crates/bbb-renderer/src/hud.rs#title_alpha_follows_the_fade_in_stay_fade_out_windows",
                "crates/bbb-renderer/src/hud.rs#title_draws_center_and_scale_title_4x_and_subtitle_2x",
            ),
        ),
        ("extractTitle",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/hud.rs#apply_title_text",
                "crates/bbb-world/src/client/hud.rs#apply_subtitle_text",
            ),
            ("crates/bbb-world/src/client/hud.rs#title_packets_match_vanilla_timing_rules",),
        ),
        ("setTitle", "setSubtitle"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/hud.rs#apply_titles_animation",
                "crates/bbb-world/src/client/hud.rs#apply_clear_titles",
            ),
            (
                "crates/bbb-world/src/client/hud.rs#title_packets_match_vanilla_timing_rules",
                "crates/bbb-world/src/client/hud.rs#hud_text_packets_update_canonical_state",
            ),
        ),
        ("setTimes", "clearTitles", "resetTitleTimes"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime/render_extract.rs#hud_action_bar_text_from_world",),
            (
                "crates/bbb-renderer/src/hud.rs#overlay_message_alpha_matches_vanilla_ramp_and_cap",
                "crates/bbb-renderer/src/hud.rs#overlay_message_rainbow_rgb_is_deterministic_and_matches_the_hsv_quirk",
            ),
        ),
        ("extractOverlayMessage",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/hud.rs#set_overlay_message",),
            ("crates/bbb-world/src/client/hud.rs#hud_text_components_remain_exact_across_all_overlay_producers",),
        ),
        ("setOverlayMessage",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/hud.rs#set_jukebox_now_playing",),
            (
                "crates/bbb-world/src/client/hud.rs#jukebox_now_playing_wraps_one_exact_component_for_overlay_and_narration",
                "crates/bbb-native/src/runtime/tests.rs#jukebox_now_playing_localizes_for_animated_action_bar_and_system_now_narration",
            ),
        ),
        ("setNowPlaying",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud.rs#hud_sound_subtitle_placement",),
            ("crates/bbb-renderer/src/hud.rs#sound_subtitle_placement_obeys_screen_and_hide_gui_rules",),
        ),
        ("extractSubtitleOverlay",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud.rs#HudSoundSubtitlePlacement",),
            ("crates/bbb-renderer/src/hud.rs#sound_subtitle_commands_precede_screens_or_defer_past_debug_and_toasts",),
        ),
        ("extractDeferredSubtitles",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime/render_extract.rs#hud_player_list_is_visible",),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_player_list_projection_uses_scores_rich_names_and_vanilla_visibility",
                "crates/bbb-renderer/src/hud/player_list.rs#grid_and_ping_thresholds_match_vanilla",
            ),
        ),
        ("extractTabList",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-world/src/client/scoreboard.rs#sidebar_state",),
            ("crates/bbb-world/src/client/scoreboard/tests.rs#scoreboard_sidebar_prefers_team_slot_and_filters_sorts_and_limits_scores",),
        ),
        ("extractScoreboardSidebar",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/hud/scoreboard.rs#push_hud_scoreboard_sidebar",),
            (
                "crates/bbb-renderer/src/hud/scoreboard.rs#scoreboard_layout_matches_vanilla_right_edge_and_vertical_formula",
                "crates/bbb-renderer/src/hud/scoreboard.rs#scoreboard_layout_counts_spacer_only_for_visible_scores",
            ),
        ),
        ("displayScoreboardSidebar",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime/render_extract.rs#hud_boss_bars_from_world",),
            (
                "crates/bbb-renderer/src/hud.rs#boss_bar_layers_follow_vanilla_background_then_cropped_fill_order",
                "crates/bbb-world/src/client/hud.rs#boss_events_add_update_remove_and_ignore_unknown_updates",
            ),
        ),
        ("extractBossOverlay",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/runtime.rs#hud_debug_overlay_at_partial_tick",),
            ("crates/bbb-native/src/runtime/tests.rs#hud_debug_overlay_projects_vanilla_default_profile_entries",),
        ),
        ("extractDebugOverlay",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/client_options/catalog_26_1.rs#showAutosaveIndicator",),
            ("crates/bbb-native/src/client_options.rs#defaults_carry_the_text_vanilla_writes_for_every_catalogued_key",),
        ),
        ("extractSavingIndicator", "tickAutosaveIndicator"),
    ),
)


INPUT_KEYBOARD_ROUTING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#pub(crate) struct ClientInputState",
                "crates/bbb-native/src/main.rs#window.set_ime_allowed(false)",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#debug_advanced_item_tooltips_startup_state_is_runtime_initial_value",
            ),
        ),
        ("KeyboardHandler",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn handle_key_input_with_item_runtime",
                "crates/bbb-native/src/simulation/input_commands.rs#fn apply_key",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#f3_release_toggles_debug_overlay_without_gameplay_command",
                "crates/bbb-native/src/input/tests.rs#the_fullscreen_key_toggles_the_window_and_saves_the_option",
                "crates/bbb-native/src/input/tests.rs#the_screenshot_key_requests_one_grab_per_press_and_announces_the_saved_name",
            ),
        ),
        ("keyPress",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn handle_text_input_with_item_runtime",
                "crates/bbb-native/src/simulation/input_commands.rs#fn apply_ime_commit",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#text_input_requires_focus_and_leading_slash",
                "crates/bbb-native/src/input/tests.rs#sign_editor_text_input_changes_lines_and_queues_sign_update",
            ),
        ),
        ("charTyped",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/simulation/input_commands.rs#fn apply_ime_preedit",
                "crates/bbb-native/src/input.rs#fn handle_dialog_ime_preedit_at",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#dialog_ime_preedit_maps_utf8_cursor_ranges_and_blinks_at_vanilla_period",
            ),
        ),
        ("preeditCallback",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/simulation/input_commands.rs#fn sync_ime_preedit_focus",),
            (
                "crates/bbb-native/src/simulation/input_commands.rs#an_open_composition_hands_off_to_a_newly_focused_sign_editor",
            ),
        ),
        ("resubmitLastPreeditEvent",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn handle_dialog_ime_preedit_at",
                "crates/bbb-native/src/input.rs#fn handle_sign_editor_ime_preedit_at",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#sign_editor_done_mouse_and_ime_preedit_share_one_submit_path",
            ),
        ),
        ("submitPreeditEvent",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/main.rs#fn enqueue_simulation_window_event",),
            (
                "crates/bbb-native/src/simulation.rs#all_external_commands_enter_one_mailbox_and_apply_without_a_second_queue",
                "crates/bbb-native/src/simulation/commands.rs#saturated_mailbox_never_drops_key_mouse_or_control_releases",
            ),
        ),
        ("setup",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn handle_key_input_with_item_runtime",
                "crates/bbb-native/src/input/commands.rs#fn hotbar_slot_for_key",
                "crates/bbb-native/src/input.rs#fn toggled_down",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#digit_key_selects_hotbar_slot_updates_world_and_queues_command",
                "crates/bbb-native/src/input/tests.rs#camera_perspective_f5_consumes_one_cycle_per_press_edge",
                "crates/bbb-native/src/input/tests.rs#drop_key_queues_drop_item_action",
                "crates/bbb-native/src/input/social.rs#social_interactions_default_p_key_opens_and_closes_away_from_search",
            ),
        ),
        ("handleKeybinds",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn shift_down",),
            (
                "crates/bbb-native/src/input/tests.rs#release_active_input_keeps_shift_modifier_for_inventory_clicks",
            ),
        ),
        ("hasShiftDown",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn control_down",),
            (
                "crates/bbb-native/src/input/tests.rs#control_drop_key_queues_drop_all_items_action",
            ),
        ),
        ("hasControlDown",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn alt_down",),
            (
                "crates/bbb-native/src/input/tests.rs#text_edit_shortcut_predicate_requires_modifier_without_shift_or_alt",
            ),
        ),
        ("hasAltDown",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn note_key_input_type",
                "crates/bbb-native/src/input.rs#enum LastInputType",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#the_last_input_type_follows_the_screen_gated_keyboard_and_mouse_events",
                "crates/bbb-native/src/input/tests.rs#opening_a_screen_from_no_screen_resets_the_last_input_type",
            ),
        ),
        ("setLastInputType",),
    ),
)


INPUT_DEBUG_KEY_ACTIONS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn debug_copy_recreate_command",
                "crates/bbb-native/src/input.rs#fn queue_debug_recreate_server_query_request",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#f3_i_without_shift_records_server_recreate_query_request_shell",
                "crates/bbb-native/src/input/tests.rs#f3_i_without_gamemaster_permission_copies_client_recreate_command_without_query",
            ),
        ),
        ("copyRecreateCommand",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn debug_copy_recreate_block_command",),
            (
                "crates/bbb-native/src/input/tests.rs#shift_f3_i_copies_block_recreate_command_to_clipboard_and_reports_feedback",
                "crates/bbb-native/src/input/tests.rs#shift_f3_i_with_permission_copies_local_block_entity_nbt_to_clipboard",
            ),
        ),
        ("copyCreateBlockCommand",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn debug_copy_recreate_entity_command",),
            (
                "crates/bbb-native/src/input/tests.rs#shift_f3_i_copies_entity_recreate_command_to_clipboard_and_reports_feedback",
                "crates/bbb-native/src/input/tests.rs#f3_i_server_entity_tag_response_copies_recreate_command_without_uuid_or_pos",
            ),
        ),
        ("copyCreateEntityCommand",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn push_debug_feedback_chat_message",),
            (
                "crates/bbb-native/src/input/tests.rs#f3_t_records_resource_pack_reload_request_and_feedback_without_toggling_overlay",
            ),
        ),
        ("debugFeedback", "debugFeedbackTranslated"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn push_debug_feedback_chat_runs",),
            (
                "crates/bbb-native/src/input/tests.rs#f3_s_records_dynamic_texture_dump_request_and_feedback_without_toggling_overlay",
            ),
        ),
        ("debugFeedbackComponent",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn handle_shared_debug_hotkey",),
            (
                "crates/bbb-native/src/input/tests.rs#shared_debug_hotkeys_toggle_dev_state_when_startup_flag_is_enabled",
                "crates/bbb-native/src/runtime/tests.rs#f3_p_emits_pause_on_lost_focus_debug_feedback_with_world",
            ),
        ),
        ("debugFeedbackEnabledStatus",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn push_debug_warning_chat_message",
                "crates/bbb-native/src/input.rs#VANILLA_DEBUG_WARNING_COLOR",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#f3_c_hold_reports_manual_crash_warning_countdown_without_toggling_overlay",
            ),
        ),
        ("debugWarningComponent",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn decorated_debug_styled_runs",
                "crates/bbb-native/src/input.rs#VANILLA_DEBUG_FEEDBACK_COLOR",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#f3_t_records_resource_pack_reload_request_and_feedback_without_toggling_overlay",
                "crates/bbb-native/src/input/tests.rs#f3_c_hold_reports_manual_crash_warning_countdown_without_toggling_overlay",
            ),
        ),
        ("decorateDebugComponent",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn push_debug_feedback_chat_runs",
                "crates/bbb-world/src/client/chat.rs#fn push_styled_client_system_chat_message",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#f3_v_dumps_version_debug_chat_without_toggling_overlay_on_release",
            ),
        ),
        ("showDebugChat",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn handle_shared_debug_hotkey",
                "crates/bbb-native/src/startup.rs#debug_hotkeys",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#shared_debug_hotkeys_toggle_dev_state_when_startup_flag_is_enabled",
                "crates/bbb-native/src/input/tests.rs#shared_debug_hotkeys_without_player_only_handle_global_dev_toggles",
            ),
        ),
        ("handleChunkDebugKeys",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn handle_debug_overlay_key_inner",
                "crates/bbb-native/src/input.rs#fn handle_debug_feature_count_key",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#debug_hotkeys_take_priority_over_feature_count_and_regular_keymap",
                "crates/bbb-native/src/input/tests.rs#f3_release_toggles_debug_overlay_without_gameplay_command",
                "crates/bbb-native/src/input/tests.rs#f3_n_queues_spectator_game_mode_with_permission",
            ),
        ),
        ("handleDebugKeys",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn advance_debug_crash_hold",
                "crates/bbb-native/src/input.rs#DEBUG_CRASH_TIME",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#f3_c_hold_reports_manual_crash_warning_countdown_without_toggling_overlay",
                "crates/bbb-native/src/input/tests.rs#f3_c_hold_panics_after_vanilla_manual_crash_delay",
            ),
        ),
        ("tick",),
    ),
)


INPUT_MOUSE_ROUTING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#MouseSensitivity",
                "crates/bbb-native/src/input.rs#scroll_accumulated_x",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#the_mouse_turn_delta_follows_the_vanilla_sensitivity_curve",
                "crates/bbb-native/src/input/tests.rs#the_scroll_delta_follows_the_discrete_and_wheel_sensitivity_options",
            ),
        ),
        ("MouseHandler",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#WindowEvent::MouseInput",
                "crates/bbb-native/src/simulation/input_commands.rs#fn apply_mouse_button",
                "crates/bbb-native/src/input/mouse.rs#handle_mouse_input_at_partial_tick_with_effects_and_presentation",
            ),
            (
                "crates/bbb-native/src/input/mouse.rs#left_mouse_press_queues_main_hand_swing",
                "crates/bbb-native/src/input/mouse.rs#right_mouse_press_without_block_queues_use_item",
                "crates/bbb-native/src/input/mouse.rs#sign_editor_mouse_press_does_not_queue_gameplay_input",
            ),
        ),
        ("onButton",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/mouse.rs#InputQuirks.SIMULATE_RIGHT_CLICK_WITH_LONG_LEFT_CLICK",
            ),
            ("crates/bbb-native/src/input/mouse.rs#left_mouse_press_queues_main_hand_swing",),
        ),
        ("simulateRightClick",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input/mouse.rs#fillMousePositionDetails",),
            (
                "crates/bbb-native/src/crash_report.rs#friendly_report_has_vanilla_sections_and_bounded_accounting",
            ),
        ),
        ("fillMousePositionDetails",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn scale_scroll_delta",
                "crates/bbb-native/src/input/mouse.rs#pub(crate) fn handle_mouse_wheel",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#the_scroll_delta_follows_the_discrete_and_wheel_sensitivity_options",
                "crates/bbb-native/src/input/mouse.rs#mouse_wheel_selects_hotbar_slot_updates_world_and_queues_command",
                "crates/bbb-native/src/input/mouse.rs#spectator_mouse_wheel_adjusts_local_flying_speed_without_hotbar_command",
            ),
        ),
        ("onScroll",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/main.rs#WindowEvent::DroppedFile",),
            (
                "crates/bbb-native/src/simulation/commands.rs#commands_from_winit_and_control_share_one_monotonic_sequence",
            ),
        ),
        ("onDrop",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#fn enqueue_simulation_window_event",
                "crates/bbb-native/src/main.rs#DeviceEvent::MouseMotion",
            ),
            (
                "crates/bbb-native/src/main.rs#cursor_events_cache_physical_and_reproject_at_the_gui_boundary",
                "crates/bbb-native/src/main.rs#cursor_capture_prefers_locked_and_falls_back_to_confined_independently_of_raw_input",
            ),
        ),
        ("setup",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#WindowEvent::CursorMoved",
                "crates/bbb-native/src/window_surface.rs#fn on_cursor_moved",
                "crates/bbb-native/src/input/mouse.rs#pub(crate) fn handle_mouse_motion",
            ),
            (
                "crates/bbb-native/src/window_surface.rs#cursor_cache_stays_physical_and_reprojects_after_gui_projection_changes",
                "crates/bbb-native/src/runtime/control_requests.rs#pump_control_input_requests_replays_keys_through_the_real_key_handler",
            ),
        ),
        ("onMove",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input/movement.rs#pub(crate) fn advance_player_input_for_client_ticks",),
            (
                "crates/bbb-native/src/input/movement.rs#client_tick_catch_up_preserves_each_tutorial_movement_sample",
                "crates/bbb-native/src/input/movement.rs#mounted_mouse_motion_updates_look_and_queues_riding_rotation",
            ),
        ),
        ("handleAccumulatedMovement",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#pub(crate) fn mouse_turn_delta",),
            (
                "crates/bbb-native/src/input/tests.rs#the_mouse_turn_delta_follows_the_vanilla_sensitivity_curve",
                "crates/bbb-native/src/input/movement.rs#local_input_projection_includes_focus_keys_and_mouse_delta",
            ),
        ),
        ("turnPlayer",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/window_surface.rs#pub(crate) fn project_cursor_to_gui",),
            (
                "crates/bbb-native/src/window_surface.rs#cursor_projection_uses_the_vanilla_ratio_not_integer_division",
                "crates/bbb-native/src/main.rs#resize_gui_surface_projects_1280x720_at_scale_three_to_427x240",
            ),
        ),
        ("getScaledXPos", "getScaledYPos"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#mouseHandler.setIgnoreFirstMove",
                "crates/bbb-native/src/window_surface.rs#fn on_cursor_left",
            ),
            (
                "crates/bbb-native/src/main.rs#cursor_capture_prefers_locked_and_falls_back_to_confined_independently_of_raw_input",
            ),
        ),
        ("setIgnoreFirstMove",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#fn set_cursor_capture",
                "crates/bbb-native/src/simulation/input_commands.rs#fn request_cursor_capture",
            ),
            (
                "crates/bbb-native/src/main.rs#cursor_capture_prefers_locked_and_falls_back_to_confined_independently_of_raw_input",
                "crates/bbb-native/src/input/mouse.rs#local_attack_miss_time_uses_screen_sentinel_and_clears_on_release",
            ),
        ),
        ("grabMouse",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#pub(crate) fn runtime_wants_cursor",
                "crates/bbb-native/src/simulation/input_commands.rs#fn simulation_wants_cursor",
            ),
            ("crates/bbb-native/src/main.rs#runtime_wants_cursor_for_advancements_screen",),
        ),
        ("releaseMouse",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/main.rs#WindowEvent::CursorEntered",),
            (
                "crates/bbb-native/src/window_surface.rs#cursor_cache_stays_physical_and_reprojects_after_gui_projection_changes",
            ),
        ),
        ("cursorEntered",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input/mouse.rs#SharedConstants.DEBUG_CURSOR_POS",),
            (
                "crates/bbb-native/src/input/tests.rs#f3_release_toggles_debug_overlay_without_gameplay_command",
            ),
        ),
        ("drawDebugMouseInfo",),
    ),
)


INPUT_KEY_MAPPING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options/key_mappings_26_1.rs#VANILLA_KEY_MAPPINGS",
                "crates/bbb-native/src/client_options/key_bindings.rs#impl Default for KeyBindings",
            ),
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#the_catalog_covers_every_binding_default_with_a_named_key",
                "crates/bbb-native/src/client_options/key_bindings.rs#every_binding_resolves_to_a_key_the_window_can_report",
            ),
        ),
        ("KeyMapping", "registerMapping"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#let just_pressed = pressed && !input.pressed_keys.contains(&code);",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#toggle_gui_key_flips_hide_gui_once_per_press_edge",
                "crates/bbb-native/src/input/tests.rs#camera_perspective_f5_consumes_one_cycle_per_press_edge",
            ),
        ),
        ("click", "consumeClick"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn set_key_down",
                "crates/bbb-native/src/input.rs#fn set_mouse_button_down",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#item_model_keybind_context_tracks_default_key_and_mouse_state",
            ),
        ),
        ("set",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn binding_down",),
            (
                "crates/bbb-native/src/input/tests.rs#a_rebound_jump_key_moves_both_the_action_and_the_key_it_left",
            ),
        ),
        ("forAllKeyMappings",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#KeyMapping.setAll",
                "crates/bbb-native/src/input.rs#pub(crate) fn handle_focus_change",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#focus_loss_clears_pressed_input_and_queues_release",
            ),
        ),
        ("setAll",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#pub(crate) fn release_active_input",
                "crates/bbb-native/src/input.rs#fn clear_pressed",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#focus_loss_clears_pressed_input_and_queues_release",
                "crates/bbb-native/src/input/tests.rs#focus_loss_aborts_destroying_block",
            ),
        ),
        ("releaseAll",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn restore_toggle_states_on_screen_closed",),
            (
                "crates/bbb-native/src/input/tests.rs#closing_a_screen_restores_a_keyboard_bound_toggle_it_released",
            ),
        ),
        ("restoreToggleStatesOnScreenClosed",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn reset_toggle_keys",
                "crates/bbb-native/src/input.rs#fn sync_respawn_toggle_reset",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#a_queued_respawn_resets_every_toggle_key_like_local_player_respawn",
            ),
        ),
        ("resetToggleKeys",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#pub(crate) fn from_options",
                "crates/bbb-native/src/input.rs#pub(crate) fn set_key_bindings",
            ),
            ("crates/bbb-native/src/client_options/key_bindings.rs#a_rebound_key_moves_the_resolved_input",),
        ),
        ("resetMapping",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn clear_pressed",
                "crates/bbb-native/src/input.rs#sneak_released_by_screen_when_down",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#focus_loss_clears_pressed_input_and_queues_release",
            ),
        ),
        ("release",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn restores_toggle_on_screen_closed",
                "crates/bbb-native/src/main.rs#KeyMapping.setAll",
            ),
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#every_binding_resolves_to_a_key_the_window_can_report",
            ),
        ),
        ("shouldSetOnIngameFocus",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#pub(crate) fn bound_key",
                "crates/bbb-native/src/client_options/key_bindings.rs#pub(crate) fn named_key",
            ),
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#a_stored_binding_overrides_the_default_and_a_bad_name_does_not",
            ),
        ),
        ("setKey",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#KeyMapping.compareTo",
                "crates/bbb-native/src/client_options/key_mappings_26_1.rs#sort_order",
            ),
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#the_catalog_covers_every_binding_default_with_a_named_key",
            ),
        ),
        ("compareTo",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#fn install_bound_key_display_keys",
                "crates/bbb-native/src/component_text.rs#KeyMapping.createNameSupplier",
            ),
            (
                "crates/bbb-native/src/component_text.rs#keybind_component_resolves_the_currently_bound_key_with_id_fallback",
                "crates/bbb-native/src/component_text.rs#bound_key_display_keys_follow_rebinds_and_defaults",
            ),
        ),
        ("createNameSupplier",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/client_options/key_bindings.rs#Category.label",),
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#every_binding_resolves_to_a_key_the_window_can_report",
            ),
        ),
        ("same", "label"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#pub(crate) fn is_unbound",
                "crates/bbb-native/src/client_options/key_bindings.rs#pub(crate) enum BoundInput",
            ),
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#every_binding_resolves_to_a_key_the_window_can_report",
                "crates/bbb-native/src/input/tests.rs#item_model_keybind_context_accepts_vanilla_default_unbound_keymappings",
            ),
        ),
        ("isUnbound",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#pub(crate) fn binding_matches",),
            (
                "crates/bbb-native/src/input/tests.rs#a_rebound_jump_key_moves_both_the_action_and_the_key_it_left",
            ),
        ),
        ("matches",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn binding_down",
                "crates/bbb-native/src/client_options/key_mappings_26_1.rs#VANILLA_MOUSE_BUTTONS",
            ),
            ("crates/bbb-native/src/client_options/key_bindings.rs#a_rebound_key_moves_the_resolved_input",),
        ),
        ("matchesMouse",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/client_options/key_bindings.rs#fn bound_key_display_keys",),
            (
                "crates/bbb-native/src/component_text.rs#bound_key_display_keys_follow_rebinds_and_defaults",
            ),
        ),
        ("getTranslatedKeyMessage",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/client_options/key_bindings.rs#pub(crate) fn rebound_keys",),
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#rebound_keys_lists_only_bindings_that_left_their_default",
            ),
        ),
        ("isDefault",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options/key_mappings_26_1.rs#VANILLA_NAMED_KEYS",
                "crates/bbb-native/src/client_options.rs#CLIENT_OPTION_PREFIXES",
            ),
            (
                "crates/bbb-native/src/client_options.rs#save_round_trips_and_drops_keys_this_client_does_not_know",
                "crates/bbb-native/src/client_options/key_bindings.rs#the_catalog_covers_every_binding_default_with_a_named_key",
            ),
        ),
        ("saveString",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn toggled_down",
                "crates/bbb-native/src/input.rs#pub(crate) fn set_toggle_key_options",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#toggle_crouch_flips_sneak_on_press_and_survives_the_release",
            ),
        ),
        ("setDown",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#pub(crate) fn input",
                "crates/bbb-item-model/src/item_runtime.rs#pub fn keybind_down",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#item_model_keybind_context_accepts_vanilla_default_unbound_keymappings",
                "crates/bbb-native/src/runtime/tests.rs#hotbar_item_icons_use_keybind_down_condition_when_default_key_is_down",
            ),
        ),
        ("get",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/client_options/key_mappings_26_1.rs#category",),
            (
                "crates/bbb-native/src/client_options/key_bindings.rs#the_catalog_covers_every_binding_default_with_a_named_key",
            ),
        ),
        ("register",),
    ),
)


UI_ACCESSIBILITY_SURFACE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/narrator_runtime.rs#NativeNarratorRuntime",
                "crates/bbb-native/src/narrator_runtime.rs#struct CommandTextToSpeech",
            ),
            (
                "crates/bbb-native/src/startup.rs#args_accept_narrator_status_command_and_repeated_arguments",
                "crates/bbb-native/src/narrator_runtime.rs#narrator_status_filters_interrupting_system_messages",
            ),
        ),
        ("GameNarrator",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/narrator_runtime.rs#fn say_chat_queued",
                "crates/bbb-native/src/narrator_runtime.rs#should_narrate_chat",
            ),
            (
                "crates/bbb-native/src/narrator_runtime.rs#chat_narration_modes_queue_without_interrupting_current_speech",
                "crates/bbb-world/src/client/chat.rs#delayed_player_chat_defers_display_reporting_and_last_seen_until_acceptance",
            ),
        ),
        ("sayChatQueued",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/narrator_runtime.rs#fn say_system_chat_queued",
                "crates/bbb-world/src/client/chat.rs#fn push_rich_server_system_chat_message",
            ),
            (
                "crates/bbb-native/src/narrator_runtime.rs#chat_narration_modes_queue_without_interrupting_current_speech",
                "crates/bbb-native/src/runtime/tests.rs#game_event_chat_components_localize_for_hud_and_system_narration",
            ),
        ),
        ("saySystemChatQueued",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/narrator_runtime.rs#fn say_system_queued",
                "crates/bbb-world/src/client/chat.rs#fn push_overlay_chat_narration",
            ),
            (
                "crates/bbb-native/src/narrator_runtime.rs#chat_narration_modes_queue_without_interrupting_current_speech",
                "crates/bbb-world/src/level.rs#demo_hint_game_events_record_rich_translations_and_system_narration",
            ),
        ),
        ("saySystemQueued",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#world.take_pending_chat_narrations()",
                "crates/bbb-native/src/runtime/render_extract.rs#fn chat_narration_text",
            ),
            (
                "crates/bbb-native/src/simulation.rs#chat_narration_enters_the_lossless_effect_gate_before_publication",
                "crates/bbb-native/src/runtime/tests.rs#chat_narration_text_uses_styled_content_without_duplicate_projection",
            ),
        ),
        ("narrateNotInterruptingMessage",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/narrator_runtime.rs#fn say_system_now",
                "crates/bbb-world/src/client/chat.rs#fn push_interrupting_system_component_narration",
            ),
            (
                "crates/bbb-native/src/narrator_runtime.rs#narrator_status_filters_interrupting_system_messages",
                "crates/bbb-world/src/client/hud.rs#jukebox_now_playing_wraps_one_exact_component_for_overlay_and_narration",
            ),
        ),
        ("saySystemNow",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/narrator_runtime.rs#fn narrator_command_arguments",
                "crates/bbb-native/src/startup.rs#fn stored_audio_volume_settings",
            ),
            (
                "crates/bbb-native/src/narrator_runtime.rs#command_arguments_substitute_the_final_voice_volume_placeholder",
                "crates/bbb-native/src/startup.rs#stored_audio_volumes_cover_every_vanilla_source_with_safe_defaults",
            ),
        ),
        ("narrateMessage",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/narrator_runtime.rs#enum NarratorStatus",
                "crates/bbb-native/src/input.rs#fn set_narrator_status",
            ),
            (
                "crates/bbb-native/src/client_options.rs#an_enum_option_write_stores_the_numeric_id",
                "crates/bbb-native/src/input/tests.rs#the_narrator_hotkey_cycles_the_status_and_saves_the_enum_id",
            ),
        ),
        ("getStatus",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/narrator_runtime.rs#IS_RUNNING_IN_IDE",),
            (
                "crates/bbb-native/src/narrator_runtime.rs#chat_narration_modes_queue_without_interrupting_current_speech",
            ),
        ),
        ("logNarratedMessage",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/narrator_runtime.rs#fn update_status",
                "crates/bbb-native/src/input.rs#fn handle_narrator_hotkey",
                "crates/bbb-world/src/client/ui.rs#fn push_narrator_toggle_toast",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#the_narrator_hotkey_cycles_the_status_and_saves_the_enum_id",
                "crates/bbb-native/src/runtime/tests.rs#narrator_toggle_projects_enabled_and_disabled_system_toasts",
                "crates/bbb-native/src/narrator_runtime.rs#a_keyboard_opened_screen_suppresses_narration_until_the_next_action",
            ),
        ),
        ("updateNarratorStatus",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input.rs#fn set_narrator_available",),
            (
                "crates/bbb-native/src/input/tests.rs#the_narrator_hotkey_stays_out_of_text_entry_and_unavailable_narrators",
            ),
        ),
        ("isActive",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/narrator_runtime.rs#fn clear",),
            (
                "crates/bbb-native/src/narrator_runtime.rs#chat_narration_modes_queue_without_interrupting_current_speech",
            ),
        ),
        ("clear",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/narrator_runtime.rs#impl Drop for CommandTextToSpeech",),
            (
                "crates/bbb-native/src/narrator_runtime.rs#dropping_the_narrator_runtime_kills_and_reaps_the_speaking_child",
            ),
        ),
        ("destroy",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/main.rs#MessageBox.errorWithContinue",),
            (
                "crates/bbb-native/src/startup.rs#args_accept_narrator_status_command_and_repeated_arguments",
            ),
        ),
        ("checkStatus",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/main.rs#NarratorInitException",),
            (
                "crates/bbb-native/src/crash_report.rs#production_fatal_paths_cannot_restore_log_only_exit_or_string_worker_errors",
            ),
        ),
        ("NarratorInitException",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/simulation/input_commands.rs#fn apply_ime_preedit",
                "crates/bbb-native/src/input.rs#fn clear_dialog_ime_preedit",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#dialog_ime_preedit_maps_utf8_cursor_ranges_and_blinks_at_vanilla_period",
                "crates/bbb-native/src/input/tests.rs#sign_editor_done_mouse_and_ime_preedit_share_one_submit_path",
            ),
        ),
        ("onTextInputFocusChange",),
    ),
)


UI_CLIPBOARD_SURFACE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#fn get_debug_clipboard_text",
                "crates/bbb-native/src/input.rs#trait DebugClipboard",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#chat_entry_copies_cuts_and_pastes_through_the_clipboard",
                "crates/bbb-native/src/input/tests.rs#absent_or_failing_clipboard_is_a_safe_noop_for_edit_shortcuts",
                "crates/bbb-native/src/input/tests.rs#sign_editor_clipboard_copy_cut_and_sanitized_paste_match_text_field_helper",
            ),
        ),
        ("getClipboard",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#fn set_debug_clipboard_text",
                "crates/bbb-native/src/input.rs#fn copy_selection_to_clipboard",
                "crates/bbb-native/src/runtime/dialog_screen.rs#KeyboardHandler.setClipboard intentionally ignores empty text.",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#chat_entry_copies_cuts_and_pastes_through_the_clipboard",
                "crates/bbb-native/src/input/tests.rs#anvil_rename_clipboard_shortcuts_select_copy_cut_paste_and_emit_only_on_change",
                "crates/bbb-native/src/runtime/dialog_screen.rs#copy_to_clipboard_action_writes_non_empty_text_before_transition",
            ),
        ),
        ("setClipboard",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/main.rs#pushClipboard",),
            (
                "crates/bbb-native/src/input/tests.rs#absent_or_failing_clipboard_is_a_safe_noop_for_edit_shortcuts",
            ),
        ),
        ("pushClipboard",),
    ),
)


PLATFORM_WINDOW_LIFECYCLE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/startup.rs#build_window",
                "crates/bbb-native/src/main.rs#native platform window created",
            ),
            (
                "crates/bbb-native/src/startup.rs#window_config_clamps_dimensions_to_minimum_nonzero",
                "crates/bbb-native/src/window_icon.rs#production_source_has_one_ordered_startup_call_and_no_icon_path_resolver",
            ),
        ),
        ("Window",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/startup.rs#fn build_window",
                "crates/bbb-native/src/startup.rs#create native window",
            ),
            (
                "crates/bbb-native/src/startup.rs#window_config_clamps_dimensions_to_minimum_nonzero",
            ),
        ),
        ("createGlfwWindow", "createWindow"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/window_icon.rs#platform_identity",
                "crates/bbb-native/src/window_icon.rs#NativeWindowBackend",
            ),
            ("crates/bbb-native/src/window_icon.rs#platform_identity_matches_glfw_platform_names",),
        ),
        ("getPlatform",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#WindowEvent::CloseRequested",
                "crates/bbb-native/src/simulation/input_commands.rs#request_network_terminal_shutdown",
            ),
            (
                "crates/bbb-native/src/simulation/input_commands.rs#network_terminal_shutdown_is_idempotent_and_requests_disconnect",
                "crates/bbb-native/src/main.rs#window_title_follows_the_published_connection_state_and_diffs_applications",
            ),
        ),
        ("shouldClose", "setWindowCloseCallback"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/startup.rs#checkGlfwError",),
            (
                "crates/bbb-native/src/crash_report.rs#guarded_error_and_panic_become_typed_failures_but_success_stays_clean",
            ),
        ),
        (
            "checkGlfwError",
            "defaultErrorCallback",
            "setBootErrorCallback",
            "setDefaultErrorCallback",
        ),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#crash_reports.guard",
                "crates/bbb-native/src/crash_report.rs#exit_after_failure",
            ),
            (
                "crates/bbb-native/src/crash_report.rs#guarded_error_and_panic_become_typed_failures_but_success_stays_clean",
                "crates/bbb-native/src/crash_report.rs#process_failure_writes_report_exits_nonzero_and_next_process_recovers",
            ),
        ),
        ("bootCrash",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/crash_report.rs#RuntimeFailure",
                "crates/bbb-native/src/crash_report.rs#phase: String",
            ),
            (
                "crates/bbb-native/src/crash_report.rs#friendly_report_has_vanilla_sections_and_bounded_accounting",
            ),
        ),
        ("setErrorSection",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#Event::LoopExiting",
                "crates/bbb-native/src/simulation/thread.rs#shutdown_and_join",
            ),
            (
                "crates/bbb-native/src/network_runtime.rs#production_shutdown_has_one_owner_and_frozen_order",
            ),
        ),
        ("close",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#window.current_monitor()",
                "crates/bbb-native/src/window_surface.rs#find_exact_video_mode",
            ),
            (
                "crates/bbb-native/src/window_surface.rs#fullscreen_choice_selects_exclusive_only_with_exact_mode_and_backend_support",
            ),
        ),
        ("findBestMonitor",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/window_surface.rs#allowed_window_min_size",),
            (
                "crates/bbb-native/src/window_surface.rs#allowed_window_min_size_clamps_both_dimensions_to_one",
                "crates/bbb-native/src/startup.rs#window_config_clamps_dimensions_to_minimum_nonzero",
            ),
        ),
        ("allowedWindowMinSize",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/startup.rs#create native window",
                "crates/bbb-native/src/crash_report.rs#exit_after_failure",
            ),
            (
                "crates/bbb-native/src/crash_report.rs#production_fatal_paths_cannot_restore_log_only_exit_or_string_worker_errors",
            ),
        ),
        ("WindowInitFailed",),
    ),
)


PLATFORM_WINDOW_DISPLAY_MODE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/renderer.rs#set_vsync_enabled",
                "crates/bbb-native/src/main.rs#self.renderer.set_vsync_enabled(self.vsync_enabled)",
            ),
            (
                "crates/bbb-renderer/src/renderer.rs#set_vsync_enabled_renegotiates_present_mode_without_touching_targets",
                "crates/bbb-renderer/src/renderer.rs#set_vsync_enabled_survives_zero_size_without_reconfigure",
                "crates/bbb-native/src/startup.rs#args_accept_vanilla_default_vsync_toggle",
            ),
        ),
        ("updateVsync",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn take_fullscreen_requests",
                "crates/bbb-native/src/simulation/input_commands.rs#PlatformEffect::SetFullscreen(fullscreen)",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#the_fullscreen_key_toggles_the_window_and_saves_the_option",
                "crates/bbb-native/src/main.rs#fullscreen_transitions_share_one_deterministic_mode_policy",
            ),
        ),
        ("updateFullscreenIfChanged",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/startup.rs#parse_client_fullscreen_resolution",
                "crates/bbb-native/src/client_options/args.rs#bind_fullscreen_resolution",
            ),
            (
                "crates/bbb-native/src/startup.rs#a_typed_fullscreen_resolution_wins_over_the_stored_line",
                "crates/bbb-native/src/startup.rs#an_invalid_stored_fullscreen_resolution_keeps_the_current_value",
            ),
        ),
        ("setPreferredFullscreenVideoMode",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/main.rs#Window.changeFullscreenVideoMode",),
            ("crates/bbb-native/src/main.rs#fullscreen_transitions_share_one_deterministic_mode_policy",),
        ),
        ("changeFullscreenVideoMode",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#fn set_window_fullscreen",
                "crates/bbb-native/src/window_surface.rs#fn fullscreen_choice",
            ),
            (
                "crates/bbb-native/src/main.rs#fullscreen_transitions_share_one_deterministic_mode_policy",
                "crates/bbb-native/src/window_surface.rs#fullscreen_choice_selects_exclusive_only_with_exact_mode_and_backend_support",
                "crates/bbb-native/src/window_surface.rs#video_mode_facts_exact_match_includes_refresh_and_total_bits",
            ),
        ),
        ("setMode",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn toggle_fullscreen",
                "crates/bbb-native/src/input.rs#binding_matches(\"key.fullscreen\", code)",
            ),
            ("crates/bbb-native/src/input/tests.rs#the_fullscreen_key_toggles_the_window_and_saves_the_option",),
        ),
        ("toggleFullScreen",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#fn set_window_fullscreen",
                "crates/bbb-native/src/main.rs#Windowed restoration is winit-owned",
            ),
            ("crates/bbb-native/src/main.rs#fullscreen_transitions_share_one_deterministic_mode_policy",),
        ),
        ("setWindowed",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#apply_startup_fullscreen",
                "crates/bbb-native/src/simulation/input_commands.rs#PlatformEffect::SetFullscreen(fullscreen)",
            ),
            ("crates/bbb-native/src/main.rs#fullscreen_transitions_share_one_deterministic_mode_policy",),
        ),
        ("updateFullscreen",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#rawMouseInput=false has no effect on the winit backend",
                "crates/bbb-native/src/startup.rs#client_raw_mouse_input",
            ),
            (
                "crates/bbb-native/src/startup.rs#the_stored_window_family_options_feed_the_window_owner",
                "crates/bbb-native/src/main.rs#cursor_capture_prefers_locked_and_falls_back_to_confined_independently_of_raw_input",
            ),
        ),
        ("updateRawMouseInput",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/window_surface.rs#refresh_rate_hz",
                "crates/bbb-native/src/main.rs#monitor_refresh_rate_hz",
            ),
            ("crates/bbb-native/src/window_surface.rs#refresh_rate_hz_rounds_millihertz_to_glfw_hertz",),
        ),
        ("getRefreshRate",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#`Minecraft.onFullscreenError` has no runtime trigger on this backend",
                "crates/bbb-renderer/src/renderer.rs#failure, so `Minecraft.onFullscreenError` has no trigger on this",
            ),
            ("crates/bbb-native/src/main.rs#fullscreen_transitions_share_one_deterministic_mode_policy",),
        ),
        ("onFullscreenError",),
    ),
)


PLATFORM_WINDOW_SURFACE_STATE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            ("crates/bbb-native/src/main.rs#Windowed restoration is winit-owned",),
            ("crates/bbb-native/src/main.rs#fullscreen_transitions_share_one_deterministic_mode_policy",),
        ),
        ("onMove",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#WindowEvent::Resized(size)",
                "crates/bbb-native/src/simulation/input_commands.rs#set_surface_size",
                "crates/bbb-renderer/src/renderer.rs#resize_with_prepare_check",
            ),
            (
                "crates/bbb-native/src/main.rs#resize_arm_publishes_logical_gui_size_before_queuing_surface_resized",
                "crates/bbb-native/src/simulation/input_commands.rs#zero_size_resize_marks_minimized_and_keeps_last_usable_surface",
                "crates/bbb-renderer/src/renderer.rs#gui_surface_size_stays_logical_while_surface_size_stays_physical",
            ),
        ),
        ("onFramebufferResize",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#window.inner_size()",
                "crates/bbb-native/src/simulation/input_commands.rs#SimulationInputRuntime",
            ),
            (
                "crates/bbb-native/src/simulation/input_commands.rs#surface_resize_recomputes_gui_scale_and_projection",
            ),
        ),
        ("refreshFramebufferSize",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-renderer/src/renderer.rs#physical_surface_size",),
            ("crates/bbb-renderer/src/renderer.rs#gui_surface_size_stays_logical_while_surface_size_stays_physical",),
        ),
        ("onResize",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#WindowEvent::Focused(focused)",
                "crates/bbb-native/src/input.rs#pub(crate) fn handle_focus_change",
                "crates/bbb-native/src/simulation/input_commands.rs#fn apply_focus",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#focus_loss_clears_pressed_input_and_queues_release",
                "crates/bbb-native/src/simulation/input_commands.rs#focus_regained_in_the_same_step_prevents_the_pause_screen",
            ),
        ),
        ("onFocus",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/main.rs#WindowEvent::CursorEntered",),
            (
                "crates/bbb-native/src/window_surface.rs#cursor_cache_stays_physical_and_reprojects_after_gui_projection_changes",
            ),
        ),
        ("onEnter", "cursorEntered"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/simulation/input_commands.rs#pub(crate) minimized: bool",),
            (
                "crates/bbb-native/src/simulation/input_commands.rs#zero_size_resize_marks_minimized_and_keeps_last_usable_surface",
            ),
        ),
        ("onIconify",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/window_surface.rs#calculate_gui_scale",
                "crates/bbb-native/src/simulation/input_commands.rs#project_gui_scale",
            ),
            (
                "crates/bbb-native/src/window_surface.rs#calculate_gui_scale_matches_vanilla_progression",
                "crates/bbb-native/src/window_surface.rs#calculate_gui_scale_enforce_unicode_rounds_odd_scale_up",
                "crates/bbb-native/src/simulation/input_commands.rs#manual_gui_scale_cap_and_unicode_rounding_flow_into_projection",
            ),
        ),
        ("calculateScale",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/window_surface.rs#gui_scaled_size",
                "crates/bbb-renderer/src/renderer.rs#set_gui_surface_size",
            ),
            (
                "crates/bbb-native/src/window_surface.rs#gui_scaled_size_uses_ceiling_division",
                "crates/bbb-renderer/src/renderer.rs#set_gui_surface_size_refreshes_dependent_camera_state_and_no_ops_when_unchanged",
            ),
        ),
        ("setGuiScale",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/renderer.rs#apply_resize",
                "crates/bbb-renderer/src/renderer.rs#physical_surface_size",
            ),
            ("crates/bbb-renderer/src/renderer.rs#gui_surface_size_stays_logical_while_surface_size_stays_physical",),
        ),
        ("setWidth", "setHeight"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#Window.resetIsResized",
                "crates/bbb-renderer/src/counters.rs#surface_resizes",
            ),
            ("crates/bbb-native/src/main.rs#resize_arm_publishes_logical_gui_size_before_queuing_surface_resized",),
        ),
        ("resetIsResized",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/window_surface.rs#appropriate_line_width",
                "crates/bbb-native/src/runtime.rs#crate::window_surface::appropriate_line_width",
            ),
            (
                "crates/bbb-native/src/window_surface.rs#appropriate_line_width_scales_from_1920_and_never_drops_below_2_5",
                "crates/bbb-native/src/runtime/tests.rs#block_selection_projection_uses_current_material_catalog_surface_width_and_style",
            ),
        ),
        ("getAppropriateLineWidth",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#pub(crate) fn handle_focus_change",
                "crates/bbb-native/src/main.rs#ClientInputState::new(window.has_focus())",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#unfocused_movement_key_does_not_queue_player_input",
                "crates/bbb-native/src/input/tests.rs#focus_loss_clears_pressed_input_and_queues_release",
            ),
        ),
        ("isWindowActive",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn pause_if_inactive",
                "crates/bbb-native/src/simulation/input_commands.rs#context.input.pause_if_inactive(context.now, context.world)",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#pause_if_inactive_opens_pause_screen_after_500ms_without_a_screen",
                "crates/bbb-native/src/input/tests.rs#pause_if_inactive_respects_chat_entry_as_an_open_screen",
                "crates/bbb-native/src/simulation/input_commands.rs#pause_if_inactive_is_evaluated_after_focus_changed_commands_in_the_step",
            ),
        ),
        ("pauseIfInactive",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#fn resize_gui_surface",
                "crates/bbb-native/src/simulation/input_commands.rs#set_gui_scale_options",
            ),
            (
                "crates/bbb-native/src/main.rs#resize_gui_surface_projects_1280x720_at_scale_three_to_427x240",
                "crates/bbb-native/src/simulation/input_commands.rs#surface_resize_recomputes_gui_scale_and_projection",
            ),
        ),
        ("resizeGui",),
    ),
)


WINDOW_ICON_OWNER_CENSUS_TEST = (
    "scripts/tests/test_client_mechanism_inventory.py"
    "#test_window_icon_owner_census_is_exact_and_has_per_owner_evidence"
)

WINDOW_ICON_RESOURCE_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-pack/src/window_icons.rs#pub enum WindowIconSet",
        "crates/bbb-pack/src/window_icons.rs#pub enum WindowIconCatalog",
        "crates/bbb-native/src/window_icon.rs#pub(crate) fn install_startup_window_icon",
    ),
    (
        "crates/bbb-pack/src/window_icons.rs#loads_release_and_snapshot_standard_paths_in_vanilla_order",
        "crates/bbb-pack/src/window_icons.rs#standard_prepare_is_all_or_nothing_and_ignores_selected_root_overrides",
        "crates/bbb-pack/src/window_icons.rs#indexed_icons_validate_integrity_and_do_not_open_unrelated_objects",
        "crates/bbb-native/src/window_icon.rs#x11_property_preserves_all_five_icons_and_argb_order",
        WINDOW_ICON_OWNER_CENSUS_TEST,
    ),
)

WINDOW_ICON_SET_OWNER_EVIDENCE = _owner_evidence(
    (
        WINDOW_ICON_RESOURCE_OWNER_EVIDENCE,
        ("IconSet", "getStandardIcons", "getMacIcon", "getFile"),
    ),
)

WINDOW_ICON_WINDOW_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/window_icon.rs#install_startup_window_icon",
                "crates/bbb-native/src/window_icon.rs#prepare_standard_icons",
                "crates/bbb-native/src/window_icon.rs#apply_x11_icon_property",
            ),
            (
                "crates/bbb-native/src/window_icon.rs#platform_target_preserves_vanilla_family_and_wayland_skip",
                "crates/bbb-native/src/window_icon.rs#x11_property_preserves_all_five_icons_and_argb_order",
                "crates/bbb-native/src/window_icon.rs#production_source_has_one_ordered_startup_call_and_no_icon_path_resolver",
                "crates/bbb-pack/src/window_icons.rs#standard_prepare_is_all_or_nothing_and_ignores_selected_root_overrides",
                WINDOW_ICON_OWNER_CENSUS_TEST,
            ),
        ),
        ("setIcon",),
    ),
)

PLATFORM_WINDOW_CHROME_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#fn update_window_title",
                "crates/bbb-native/src/main.rs#window.set_title(&next)",
            ),
            ("crates/bbb-native/src/main.rs#window_title_follows_the_published_connection_state_and_diffs_applications",),
        ),
        ("setTitle",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/startup.rs#client_allow_cursor_changes",
                "crates/bbb-native/src/client_options/catalog_26_1.rs#allowCursorChanges",
            ),
            (
                "crates/bbb-native/src/startup.rs#the_stored_window_family_options_feed_the_window_owner",
                "crates/bbb-native/src/main.rs#cursor_icon_changes_flow_through_select_cursor_gating",
            ),
        ),
        ("setAllowCursorChanges",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/window_surface.rs#fn select_cursor",
                "crates/bbb-native/src/main.rs#fn apply_window_cursor_icon",
            ),
            (
                "crates/bbb-native/src/window_surface.rs#select_cursor_forces_default_unless_allowed_and_skips_redundant_changes",
                "crates/bbb-native/src/main.rs#cursor_icon_changes_flow_through_select_cursor_gating",
            ),
        ),
        ("selectCursor",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/window_surface.rs#fn create_title",),
            (
                "crates/bbb-native/src/window_surface.rs#create_title_marks_modified_client_and_appends_multiplayer_while_connected",
            ),
        ),
        ("createTitle",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#fn update_window_title",
                "crates/bbb-native/src/main.rs#render_publication.net_counters.connected",
            ),
            ("crates/bbb-native/src/main.rs#window_title_follows_the_published_connection_state_and_diffs_applications",),
        ),
        ("updateTitle",),
    ),
)



RENDERING_FRAME_ORDER_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/main.rs#WindowEvent::RedrawRequested => {",
                "crates/bbb-native/src/startup.rs#pub(crate) fn effective_framerate_limit",
            ),
            (
                "crates/bbb-native/src/main.rs#redraw_arm_orders_update_render_and_framerate_resample_like_render_frame",
                "crates/bbb-native/src/startup.rs#effective_framerate_limit_matches_the_vanilla_tracker",
                "crates/bbb-native/src/startup.rs#client_framerate_limit_accepts_the_vanilla_codec_range",
            ),
        ),
        ("renderFrame",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#let camera_pose = render_camera_pose_from_world_at_partial_tick(",
                "crates/bbb-native/src/runtime.rs#terrain_upload.refresh_frame_visibility(terrain_visibility_context);",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#terrain_visibility_wiring_commits_only_through_renderer_frame",
                "crates/bbb-native/src/runtime/tests.rs#camera_fov_frame_wiring_routes_one_snapshot_to_each_projection_owner",
            ),
        ),
        ("update",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#pub(crate) fn update_renderer_from_publication",
                "crates/bbb-native/src/runtime/render_extract.rs#pub(crate) fn apply_renderer_frame",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#level_diagnostics_have_one_current_frame_collection_and_finalize_path",
                "crates/bbb-native/src/runtime/tests.rs#renderer_frame_sky_flash_environment_extracts_after_client_level_tick",
                "crates/bbb-native/src/simulation.rs#render_and_event_loop_cannot_read_the_live_simulation_owner",
            ),
        ),
        ("extract",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/render.rs#pub fn render(&mut self, screenshot: Option<&Path>) -> Result<()>",
                "crates/bbb-renderer/src/render.rs#fn finish_frame(",
            ),
            (
                "crates/bbb-renderer/src/render.rs#render_steps_are_defined_in_frame_execution_order",
                "crates/bbb-renderer/src/render.rs#render_calls_exactly_one_step_per_frame_steps_entry",
                "crates/bbb-renderer/src/render.rs#transparency_combine_and_gui_stay_internal_until_vanilla_final_present_blit",
            ),
        ),
        ("render",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#camera_spinning_ticks.advance_for_world(advanced_ticks, world);",
                "crates/bbb-native/src/runtime.rs#world.advance_item_activation_ticks(running_ticks);",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#camera_spinning_effect_ticks_and_interpolates_nausea_like_vanilla",
                "crates/bbb-native/src/runtime/tests.rs#lightmap_tick_state_projects_boss_overlay_world_darkening",
                "crates/bbb-world/src/client/screen_effects.rs#item_activation_ticks_count_down_and_clear_at_zero",
            ),
        ),
        ("tick",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/renderer.rs#pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()>",
                "crates/bbb-native/src/main.rs#WindowEvent::Resized(size) => {",
            ),
            (
                "crates/bbb-renderer/src/renderer.rs#production_resize_has_one_prepare_apply_path",
                "crates/bbb-renderer/src/renderer.rs#zero_size_resize_skips_prepare_and_preserves_backend_owners",
                "crates/bbb-native/src/main.rs#resize_arm_publishes_logical_gui_size_before_queuing_surface_resized",
            ),
        ),
        ("resize",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/renderer.rs#pub struct Renderer {",
                "crates/bbb-native/src/main.rs#simulation_thread.shutdown_and_join()",
            ),
            (
                "crates/bbb-renderer/src/renderer.rs#renderer_backend_gpu_owners_share_one_struct_raii_lifetime",
                "crates/bbb-native/src/network_runtime.rs#production_shutdown_has_one_owner_and_frozen_order",
            ),
        ),
        ("close",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/renderer.rs#pub async fn new_with_vsync_and_terrain_filtering",
                "crates/bbb-renderer/src/renderer.rs#pub(super) fn with_gpu",
            ),
            (
                "crates/bbb-renderer/src/renderer.rs#renderer_backend_gpu_owners_share_one_struct_raii_lifetime",
                "crates/bbb-renderer/src/renderer.rs#choose_format_requires_vanilla_rgba8_numeric_surface_behavior",
            ),
        ),
        ("GameRenderer",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#pub fn clear_client_level",
                "crates/bbb-native/src/runtime/render_extract.rs#pub(crate) fn synchronize_map_texture_level",
            ),
            (
                "crates/bbb-world/src/client/screen_effects.rs#clear_client_level_resets_the_item_activation",
                "crates/bbb-world/src/client/local_player.rs#clear_client_level_resets_the_camera_to_the_local_player",
                "crates/bbb-native/src/runtime/tests.rs#map_texture_cache_invalidates_only_on_level_generation_edge",
            ),
        ),
        ("resetData",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/level.rs#pub fn apply_respawn",
                "crates/bbb-native/src/runtime.rs#fn dimension_lightmap_environment",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#lightmap_environment_uses_vanilla_dimension_attributes",
                "crates/bbb-world/src/level.rs#client_level_generation_production_source_has_one_mutator_and_exact_edges",
                "crates/bbb-world/src/level.rs#spawn_info_uses_builtin_cardinal_lighting_when_dimension_type_registry_is_missing",
            ),
        ),
        ("setLevel",),
    ),
)


RENDERING_EXTRACT_SURFACE_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/simulation/input_commands.rs#fn set_surface_size",
                "crates/bbb-native/src/runtime.rs#let physical_surface_size = publication.input_runtime.surface_size;",
            ),
            (
                "crates/bbb-native/src/simulation/input_commands.rs#surface_resize_recomputes_gui_scale_and_projection",
                "crates/bbb-native/src/simulation/input_commands.rs#zero_size_resize_marks_minimized_and_keeps_last_usable_surface",
                "crates/bbb-renderer/src/renderer.rs#gui_surface_size_stays_logical_while_surface_size_stays_physical",
            ),
        ),
        ("extractWindow",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/startup.rs#fn bind_stored_client_options",
                "crates/bbb-renderer/src/renderer.rs#pub fn set_glint_settings",
            ),
            (
                "crates/bbb-native/src/startup.rs#every_binding_names_a_real_argument_and_a_catalogued_key",
                "crates/bbb-native/src/startup.rs#stored_options_fill_arguments_the_user_did_not_type",
                "crates/bbb-native/src/runtime/tests.rs#panorama_rotation_scales_with_the_scroll_speed_option",
            ),
        ),
        ("extractOptions",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/camera_pose.rs#pub(crate) fn render_camera_pose_from_world_at_partial_tick",
                "crates/bbb-native/src/runtime/render_extract.rs#pub(crate) fn fog_environment_for_world_at_camera",
            ),
            (
                "crates/bbb-native/src/camera_pose.rs#render_camera_uses_local_player_tick_endpoints",
                "crates/bbb-native/src/camera_pose.rs#camera_eye_height_is_applied_before_detached_camera_offset",
                "crates/bbb-native/src/runtime/tests.rs#camera_fluid_sample_is_shared_by_same_frame_fog_and_fov_consumers",
            ),
        ),
        ("extractCamera",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#let hud_debug_overlay = hud_debug_overlay_with_profiler_chart(",
                "crates/bbb-native/src/runtime/render_extract.rs#pub(crate) fn apply_renderer_frame",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#renderer_frame_hud_extracts_after_input_and_use_item_tick",
                "crates/bbb-native/src/runtime/tests.rs#hud_text_timers_tick_before_projection_like_gui_tick",
                "crates/bbb-native/src/runtime/tests.rs#hide_gui_frame_policy_precedes_residency_and_keeps_toasts_independent",
            ),
        ),
        ("extractGui",),
    ),
    (
        OwnerEvidence(
            (
                "docs/unsupported-features.md#`renderActiveTextDebug` is `not-needed`",
            ),
            (
                "crates/bbb-native/src/startup.rs#args_accept_sharedconstants_debug_flag_startup_options",
            ),
        ),
        ("renderActiveTextDebug",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/render_extract.rs#pub(crate) struct RendererFrame",
                "crates/bbb-native/src/runtime.rs#pub(crate) fn update_renderer_from_publication",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#level_diagnostics_have_one_current_frame_collection_and_finalize_path",
                "crates/bbb-native/src/runtime/tests.rs#hide_gui_frame_policy_precedes_residency_and_keeps_toasts_independent",
            ),
        ),
        ("getGameRenderState",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/render_extract.rs#pub(crate) fn apply_renderer_frame",
                "crates/bbb-renderer/src/entity_models/textured.rs#struct EntityModelRenderSubmission",
            ),
            (
                "crates/bbb-renderer/src/entity_models/tests/allay.rs#translucent_draw_plan_sorts_order_then_camera_distance_then_insertion",
                "crates/bbb-renderer/src/entity_models/tests.rs#texture_backed_blended_model_uploads_sort_by_order_then_camera_distance",
            ),
        ),
        ("getSubmitNodeStorage",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/render.rs#fn entity_translucent_feature_pass",
                "crates/bbb-renderer/src/render.rs#fn ordered_custom_and_text_feature_pass",
            ),
            (
                "crates/bbb-renderer/src/render.rs#render_calls_exactly_one_step_per_frame_steps_entry",
                "crates/bbb-renderer/src/render.rs#entity_translucent_features_draw_after_depth_copies_and_before_translucent_terrain",
            ),
        ),
        ("getFeatureRenderDispatcher",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/entity_models.rs#mod model_layers;",
                "crates/bbb-renderer/src/entity_models/textured.rs#fn entity_model_textured_meshes_with_equipment_for_camera",
            ),
            (
                "crates/bbb-renderer/src/entity_models/tests.rs#equipment_layer_pass_records_vanilla_model_layer_metadata",
                "crates/bbb-renderer/src/entity_models/tests.rs#textured_layer_render_type_names_match_vanilla_render_types",
            ),
        ),
        ("getEntityModels",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/renderer.rs#pub fn for_options",
                "crates/bbb-renderer/src/render.rs#self.level_frame_route.uses_improved_transparency()",
            ),
            (
                "crates/bbb-renderer/src/renderer.rs#level_frame_route_matches_vanilla_option_and_panoramic_suppression",
                "crates/bbb-renderer/src/render.rs#level_frame_route_is_the_only_feature_target_and_combine_decision_owner",
                "crates/bbb-native/src/runtime/tests.rs#startup_transparency_option_has_one_native_to_renderer_projection",
            ),
        ),
        ("useShaderTransparency",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/entity_name_tags.rs#fn entity_name_tag_is_visible",
                "crates/bbb-native/src/runtime.rs#let entity_name_tag_surfaces = entity_name_tag_surfaces(",
            ),
            (
                "crates/bbb-native/src/entity_name_tags.rs#entity_name_visibility_applies_vanilla_distance_camera_and_team_order",
                "crates/bbb-world/src/entities/tests/name_tag_projection.rs#entity_name_tag_living_visibility_preserves_team_early_returns_and_vehicle_facts",
            ),
        ),
        ("renderNames",),
    ),
)


RENDERING_LEVEL_PASS_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/render.rs#pub fn render(&mut self, screenshot: Option<&Path>) -> Result<()>",
                "crates/bbb-native/src/runtime.rs#pub(crate) fn update_renderer_from_publication",
            ),
            (
                "crates/bbb-renderer/src/render.rs#render_steps_are_defined_in_frame_execution_order",
                "crates/bbb-renderer/src/render.rs#first_person_item_pass_runs_after_world_composite_and_before_hud",
            ),
        ),
        ("renderLevel",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/item_models/first_person.rs#fn append_first_person_hand_item_models",
                "crates/bbb-renderer/src/render.rs#fn first_person_item_pass(",
            ),
            (
                "crates/bbb-native/src/item_models/tests.rs#first_person_item_models_bake_ordinary_local_hand_stack",
                "crates/bbb-renderer/src/render.rs#camera_fov_first_person_pass_uses_hud3d_camera_bindings",
                "crates/bbb-renderer/src/render.rs#hide_gui_exits_first_person_pass_before_collecting_geometry",
            ),
        ),
        ("renderItemInHand",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/camera.rs#fn camera_hurt_transform",
                "crates/bbb-native/src/camera_pose.rs#fn camera_hurt_effect_from_world",
            ),
            (
                "crates/bbb-renderer/src/camera.rs#camera_hurt_transform_matches_vanilla_bob_hurt_order",
                "crates/bbb-renderer/src/camera.rs#camera_uniform_applies_hurt_after_projection_without_changing_view",
                "crates/bbb-native/src/runtime/tests.rs#camera_hurt_effect_follows_the_active_living_camera_entity",
            ),
        ),
        ("bobHurt",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/camera.rs#fn camera_view_bob_transform",
                "crates/bbb-native/src/camera_pose.rs#fn camera_view_bob_effect_from_world",
            ),
            (
                "crates/bbb-renderer/src/camera.rs#camera_view_bob_transform_matches_vanilla_translate_and_rotation_order",
                "crates/bbb-renderer/src/camera.rs#camera_view_bob_uniform_applies_between_hurt_and_spinning_to_world_and_hud",
                "crates/bbb-native/src/runtime/tests.rs#camera_view_bob_effect_follows_active_player_and_startup_gate",
            ),
        ),
        ("bobView",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/crosshair.rs#pub(crate) fn block_selection_from_camera",
                "crates/bbb-native/src/runtime.rs#let block_selection = (!input.hide_gui())",
            ),
            (
                "crates/bbb-native/src/crosshair.rs#a_non_player_camera_entity_suppresses_the_block_outline",
                "crates/bbb-native/src/crosshair.rs#spectator_mode_only_outlines_and_crosshairs_menu_provider_blocks",
                "crates/bbb-native/src/crosshair.rs#adventure_mode_outline_requires_matching_selected_item_predicate",
            ),
        ),
        ("shouldRenderBlockOutline",),
    ),
    (
        OwnerEvidence(
            (
                "docs/unsupported-features.md#`setRenderBlockOutline` is `not-needed`",
            ),
            (
                "crates/bbb-native/src/crosshair.rs#a_non_player_camera_entity_suppresses_the_block_outline",
            ),
        ),
        ("setRenderBlockOutline",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#fn vanilla_night_vision_scale",
                "crates/bbb-native/src/runtime.rs#const VANILLA_NIGHT_VISION_FULL_STRENGTH_TICKS",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#lightmap_environment_projects_local_player_night_vision",
                "crates/bbb-native/src/runtime/tests.rs#lightmap_tick_state_advances_night_vision_duration_before_sampling",
                "crates/bbb-native/src/runtime/tests.rs#lightmap_tick_state_prefers_night_vision_over_conduit_water_vision",
            ),
        ),
        ("getNightVisionScale",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#fn boss_overlay_world_darkening",
                "crates/bbb-native/src/runtime.rs#fn tick_boss_overlay_world_darkening",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#lightmap_tick_state_projects_boss_overlay_world_darkening",
            ),
        ),
        ("getBossOverlayWorldDarkening",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/waypoint_locator.rs#fn point_pitch_direction",
            ),
            (
                "crates/bbb-native/src/runtime/waypoint_locator.rs#waypoint_locator_projects_all_renderable_data_forms_far_first",
                "crates/bbb-native/src/runtime/waypoint_locator.rs#waypoint_locator_uses_exact_visibility_and_pitch_boundaries",
            ),
        ),
        ("projectPointToScreen",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/waypoint_locator.rs#fn horizon_pitch_direction",
            ),
            (
                "crates/bbb-native/src/runtime/waypoint_locator.rs#waypoint_locator_uses_exact_visibility_and_pitch_boundaries",
            ),
        ),
        ("projectHorizonToScreen",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/camera_pose.rs#pub(crate) fn render_camera_pose_from_world_at_partial_tick",
                "crates/bbb-renderer/src/renderer.rs#pub fn set_camera_pose",
            ),
            (
                "crates/bbb-native/src/camera_pose.rs#render_camera_uses_local_player_tick_endpoints",
                "crates/bbb-native/src/camera_pose.rs#camera_perspective_detached_pose_matches_back_and_mirrored_front_offsets",
            ),
        ),
        ("getMainCamera",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#pub fn camera_entity_id",
            ),
            (
                "crates/bbb-world/src/client/hud_camera_player.rs#the_camera_player_defaults_to_the_local_player_until_set_camera_retargets_it",
                "crates/bbb-world/src/client/local_player.rs#the_hud_camera_player_only_resolves_for_the_local_player_or_a_player_entity",
            ),
        ),
        ("getCameraEntity",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#pub fn apply_set_camera",
                "crates/bbb-native/src/camera_pose.rs#pub(crate) fn current_post_effect_id_from_world",
            ),
            (
                "crates/bbb-world/src/client/local_player.rs#camera_updates_only_for_local_or_known_entities",
                "crates/bbb-native/src/runtime/tests.rs#hud_debug_post_effect_tracks_vanilla_camera_entity_families",
            ),
        ),
        ("setCameraEntity",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities.rs#fn entity_outline_color",
                "crates/bbb-world/src/client/local_player.rs#pub fn set_spectator_outlines_key_down",
            ),
            (
                "crates/bbb-world/src/entities/tests/visibility_equipment_and_mount_projection.rs#spectator_outline_key_glows_every_player_for_a_spectating_local_player",
                "crates/bbb-world/src/entities/tests/visibility_equipment_and_mount_projection.rs#entity_model_sources_project_glowing_shared_flag",
                "crates/bbb-world/src/entities/tests/visibility_equipment_and_mount_projection.rs#entity_model_sources_project_team_outline_color",
            ),
        ),
        ("shouldEntityAppearGlowing",),
    ),
)


PANORAMA_GAME_RENDERER_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#fn panorama_rotation_degrees",
                "crates/bbb-renderer/src/panorama.rs#fn panorama_uniform",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#panorama_rotation_scales_with_the_scroll_speed_option",
                "crates/bbb-native/src/runtime/tests.rs#panorama_spin_uses_realtime_ticks_only_while_visible_and_vanilla_wrap",
                "crates/bbb-renderer/src/offscreen.rs#offscreen_disconnected_panorama_draws_front_face_before_overlay_blur_and_rolls_back",
            ),
        ),
        ("getPanorama",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/hud_assets.rs#pub(crate) fn load_panorama",
                "crates/bbb-native/src/hud_assets.rs#pub(crate) fn upload_panorama",
            ),
            (
                "crates/bbb-pack/src/panorama.rs#loads_vanilla_layer_order_and_selected_override",
                "crates/bbb-renderer/src/offscreen.rs#offscreen_disconnected_panorama_draws_front_face_before_overlay_blur_and_rolls_back",
            ),
        ),
        ("registerPanoramaTextures",),
    ),
)


RENDERING_EFFECTS_SCREENSHOT_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/entities/schema.rs#pub fn vanilla_entity_camera_post_effect_id",
                "crates/bbb-native/src/camera_pose.rs#pub(crate) fn current_post_effect_id_from_world",
            ),
            (
                "crates/bbb-world/src/entities/schema.rs#generated_camera_post_effects_follow_game_renderer_class_patterns",
                "crates/bbb-native/src/runtime/tests.rs#hud_debug_post_effect_tracks_vanilla_camera_entity_families",
                "crates/bbb-native/src/input/tests.rs#camera_post_effect_selection_and_f4_edges_preserve_vanilla_active_lifecycle",
            ),
        ),
        ("checkEntityPostEffect",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#pub(crate) fn synchronize_camera_post_effect",
                "crates/bbb-renderer/src/renderer.rs#pub fn set_camera_post_effect",
            ),
            (
                "crates/bbb-renderer/src/renderer.rs#camera_post_effect_setter_commits_selection_and_active_state_together",
                "crates/bbb-native/src/runtime/tests.rs#camera_post_effect_frame_commits_the_same_selection_and_active_state_used_by_hud",
            ),
        ),
        ("setPostEffect",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn cycle_camera_type",
                "crates/bbb-renderer/src/renderer.rs#pub(super) camera_post_effect: CameraPostEffectGpu,",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#camera_perspective_cycles_and_applies_only_first_person_post_effect_boundaries",
                "crates/bbb-renderer/src/offscreen.rs#camera_post_effect_inactive_and_unknown_ids_leave_the_frame_untouched",
            ),
        ),
        ("clearPostEffect",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#fn toggle_camera_post_effect",
                "crates/bbb-native/src/input.rs#key.toggleSpectatorShaderEffects",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#camera_post_effect_selection_and_f4_edges_preserve_vanilla_active_lifecycle",
                "crates/bbb-native/src/input/tests.rs#camera_post_effect_f3_f4_chord_preserves_active_state",
            ),
        ),
        ("togglePostEffect",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input.rs#pub(crate) fn camera_post_effect_id",
                "crates/bbb-renderer/src/renderer.rs#pub fn current_post_effect_id",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_debug_overlay_suppresses_post_effect_without_current_effect",
                "crates/bbb-native/src/runtime/tests.rs#hud_debug_post_effect_is_filtered_by_reduced_debug_info",
            ),
        ),
        ("currentPostEffect",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/render.rs#fn screen_blur_passes",
                "crates/bbb-renderer/src/screen_blur.rs#pub(super) fn sanitize_menu_background_blurriness",
            ),
            (
                "crates/bbb-renderer/src/screen_blur.rs#menu_blur_radius_matches_vanilla_option_range",
                "crates/bbb-renderer/src/hud.rs#dialog_screen_box_blur_softens_prior_hud_but_keeps_screen_fill_sharp",
                "crates/bbb-renderer/src/post_effect_graph/tests.rs#selected_generation_drives_live_menu_blur_graph_and_pass_count",
            ),
        ),
        ("processBlurEffect",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/screen_effects.rs#pub(crate) fn display_local_player_item_activation",
                "crates/bbb-native/src/item_models/first_person.rs#fn append_item_activation_item_model",
            ),
            (
                "crates/bbb-world/src/client/screen_effects.rs#totem_event_for_the_local_player_starts_the_item_activation",
                "crates/bbb-native/src/item_models/tests.rs#first_person_item_models_render_the_item_activation_totem_pop",
                "crates/bbb-native/src/item_models/tests.rs#item_activation_view_transform_matches_the_vanilla_pose_stack",
            ),
        ),
        ("displayItemActivation",),
    ),
    (
        OwnerEvidence(
            (
                "docs/unsupported-features.md#The auto world-icon screenshot pair",
            ),
            (
                "crates/bbb-native/src/screenshot.rs#the_first_free_screenshot_name_follows_the_vanilla_suffix_sequence",
            ),
        ),
        ("tryTakeScreenshotIfNeeded", "takeAutoScreenshot"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/renderer.rs#let hud_pipeline = create_hud_pipeline(&device, format, &hud_bind_group_layout, None)?;",
                "crates/bbb-renderer/src/resource_reload.rs#impl GuiCorePipelines",
            ),
            (
                "crates/bbb-renderer/src/core_shaders.rs#selected_sky_and_position_tex_color_families_preload_and_filter_rolls_back",
                "crates/bbb-renderer/src/resource_reload.rs#every_renderer_pipeline_owner_has_one_shader_reload_classification",
            ),
        ),
        ("preloadUiShader",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/lightmap.rs#pub(super) fn create_lightmap_gpu",
                "crates/bbb-renderer/src/renderer.rs#pub(super) lightmap: LightmapGpu,",
            ),
            (
                "crates/bbb-renderer/src/lightmap.rs#lightmap_gpu_matches_vanilla_formula",
                "crates/bbb-renderer/src/lightmap.rs#lightmap_texture_shape_matches_vanilla",
                "crates/bbb-renderer/src/renderer.rs#lightmap_dirty_state_tracks_tick_owned_environment_changes",
            ),
        ),
        ("lightmap",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/lightmap.rs#pub(super) fn create_lightmap_sample_bind_group_layout",
                "crates/bbb-renderer/src/item_models.rs#pub const ITEM_MODEL_FULL_BRIGHT_LIGHT",
            ),
            (
                "crates/bbb-renderer/src/lightmap.rs#lightmap_pass_is_consumed_once_per_environment_change",
                "crates/bbb-renderer/src/item_models.rs#baking_a_quad_emits_two_triangles_normalized_to_the_unit_cube",
            ),
        ),
        ("levelLightmap",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/entity_models/geometry.rs#pub(super) fn fill_entity_model_overlay",
                "crates/bbb-renderer/src/item_models.rs#pub const ITEM_MODEL_NO_OVERLAY",
            ),
            (
                "crates/bbb-renderer/src/entity_models/tests.rs#entity_shaders_apply_vanilla_overlay_texture_mix",
                "crates/bbb-world/src/entities/tests/movement_combat_and_swing_projection.rs#entity_model_sources_project_hurt_overlay_for_ten_ticks",
            ),
        ),
        ("overlayTexture",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/camera.rs#pub(crate) fn with_glint_texture_time",
                "crates/bbb-renderer/src/camera.rs#pub(crate) fn with_viewport_size",
            ),
            (
                "crates/bbb-renderer/src/camera.rs#camera_uniform_stores_vanilla_glint_offsets",
                "crates/bbb-renderer/src/camera.rs#camera_uniform_stores_viewport_size_for_line_widths",
                "crates/bbb-renderer/src/camera.rs#camera_uniform_stores_vanilla_shader_game_time",
            ),
        ),
        ("getGlobalSettingsUniform",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-renderer/src/camera.rs#fn lighting_entry_directions",
                "crates/bbb-renderer/src/camera.rs#pub(crate) fn with_lighting_entry",
            ),
            (
                "crates/bbb-renderer/src/camera.rs#camera_uniform_diffuse_lighting_entries_match_vanilla_directions",
                "crates/bbb-renderer/src/camera.rs#camera_uniform_uses_nether_level_lighting_from_environment",
            ),
        ),
        ("getLighting",),
    ),
)



SECURITY_SECURE_CHAT_REPORTING_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "docs/unsupported-features.md#scope-multiplayer-server-list",
                "docs/unsupported-features.md#The CLI-direct client has no title screen.",
            ),
            (
                "crates/bbb-native/src/server_list.rs#an_entry_the_player_already_named_for_the_address_is_reused",
                "crates/bbb-native/src/server_list.rs#a_created_entry_heads_the_hidden_block_and_trims_it_to_sixteen",
            ),
        ),
        ("allowsMultiplayer",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/chat/presentation.rs#pub(super) fn can_receive_player_messages(&self) -> bool {",
                "crates/bbb-native/src/input.rs#fn chat_messages_allowed(world: &WorldStore) -> bool {",
            ),
            (
                "crates/bbb-native/src/input/tests.rs#chat_visibility_permissions_gate_messages_commands_and_message_arguments",
                "crates/bbb-native/src/runtime/tests.rs#restrictions_projection_uses_the_canonical_chat_visibility_permissions",
                "crates/bbb-world/src/client/chat.rs#system_visibility_and_matched_hidden_name_follow_chat_options",
            ),
        ),
        ("computeChatAbilities",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/social.rs#pub fn social_should_hide_message_from(&self, profile_id: Uuid) -> bool {",
                "crates/bbb-world/src/client/social.rs#pub fn social_player_is_blocked(&self, profile_id: Uuid) -> bool {",
            ),
            (
                "crates/bbb-world/src/client/social.rs#player_social_services_block_list_unions_configured_and_excludes_local_and_nil",
                "crates/bbb-world/src/client/chat.rs#player_visibility_and_blocking_suppress_display_but_process_valid_signature",
            ),
        ),
        ("isBlocked",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/social.rs#if profile_id.is_nil() || Some(profile_id) == self.local_player_profile_id() {",
                "crates/bbb-world/src/client/reporting.rs#if target.is_nil() || Some(target) == self.local_player_profile_id() {",
            ),
            (
                "crates/bbb-world/src/client/social.rs#player_social_services_block_list_unions_configured_and_excludes_local_and_nil",
            ),
        ),
        ("isLocalOrUnknownPlayer",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/local_player.rs#pub fn local_player_profile_id(&self) -> Option<Uuid> {",
                "crates/bbb-native/src/main.rs#world.set_local_player_profile_id(client_identity.profile_id);",
            ),
            (
                "crates/bbb-protocol/src/codec.rs#offline_uuid_matches_java_name_uuid_shape",
                "crates/bbb-world/src/client/social.rs#player_social_services_block_list_unions_configured_and_excludes_local_and_nil",
            ),
        ),
        ("isLocalPlayer",),
    ),
    (
        OwnerEvidence(
            (
                "docs/wiki/architecture/offline-runtime.md#`UserApiService.OFFLINE`、",
                "docs/unsupported-features.md#Online mode (authenticated Services and encrypted transport) is",
            ),
            (
                "crates/bbb-native/src/services_block_list_runtime.rs#services_block_list_offline_play_and_disconnect_stay_inactive",
            ),
        ),
        ("isNameBanned",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/startup.rs#text_filtering_enabled: args.client_text_filtering,",
                "crates/bbb-protocol/src/packets/connection.rs#out.write_bool(information.text_filtering_enabled);",
            ),
            (
                "crates/bbb-native/src/startup.rs#args_build_client_information_from_startup_options",
                "crates/bbb-native/src/runtime/book_screen.rs#rich_book_projection_preserves_style_interaction_and_filtered_page",
            ),
        ),
        ("isTextFilteringEnabled",),
    ),
    (
        OwnerEvidence(
            (
                "docs/wiki/architecture/offline-runtime.md#UserApiService.OFFLINE_PROPERTIES",
                "crates/bbb-net/src/types/connection.rs#pub fn offline(address: impl Into<String>, username: impl Into<String>) -> Result<Self> {",
            ),
            (
                "crates/bbb-net/src/types/connection.rs#offline_connection_options_delegate_login_username_validation",
            ),
        ),
        ("multiplayerBan", "userProperties"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/social.rs#pub struct PlayerSocialState {",
                "crates/bbb-world/src/store.rs#pub(crate) player_social: PlayerSocialState,",
            ),
            (
                "crates/bbb-world/src/client/social.rs#player_social_discovery_hidden_blocked_and_sorting_share_chat_owner",
                "crates/bbb-native/src/services_block_list_runtime.rs#services_block_list_offline_play_and_disconnect_stay_inactive",
            ),
        ),
        ("PlayerSocialManager",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/social.rs#pub fn hide_social_player(&mut self, profile_id: Uuid) -> bool {",
                "crates/bbb-native/src/input.rs#if input.binding_matches(\"key.socialInteractions\", code) {",
            ),
            (
                "crates/bbb-native/src/input/social.rs#social_interactions_default_p_key_opens_and_closes_away_from_search",
                "crates/bbb-native/src/runtime/tests.rs#hud_social_interactions_projection_tracks_faces_removed_status_links_and_narration",
            ),
        ),
        ("hidePlayer",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/social.rs#pub fn show_social_player(&mut self, profile_id: Uuid) -> bool {",
            ),
            (
                "crates/bbb-world/src/client/social.rs#player_social_discovery_hidden_blocked_and_sorting_share_chat_owner",
                "crates/bbb-native/src/input/social.rs#social_interactions_search_mouse_actions_links_and_pause_return_are_generation_safe",
            ),
        ),
        ("showPlayer",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/social.rs#self.social_player_is_hidden(profile_id) || self.social_player_is_blocked(profile_id)",
                "crates/bbb-world/src/client/chat.rs#.is_some_and(|sender| self.social_should_hide_message_from(sender));",
            ),
            (
                "crates/bbb-world/src/client/chat.rs#player_visibility_and_blocking_suppress_display_but_process_valid_signature",
                "crates/bbb-world/src/client/chat.rs#system_visibility_and_matched_hidden_name_follow_chat_options",
            ),
        ),
        ("shouldHideMessageFrom",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/social.rs#pub fn social_player_is_hidden(&self, profile_id: Uuid) -> bool {",
                "crates/bbb-world/src/client/chat/presentation.rs#pub hidden_players: Vec<Uuid>,",
            ),
            (
                "crates/bbb-world/src/client/social.rs#player_social_discovery_hidden_blocked_and_sorting_share_chat_owner",
                "crates/bbb-world/src/client/chat.rs#reset_chat_preserves_startup_presentation_options",
            ),
        ),
        ("isHidden",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/services_block_list_runtime.rs#fn begin_online_mode(&mut self, world: &mut WorldStore, now_epoch_seconds: i64) {",
                "crates/bbb-world/src/client/social.rs#pub fn begin_services_block_list_session(&mut self, generation: u64) {",
            ),
            (
                "crates/bbb-native/src/services_block_list_runtime.rs#services_block_list_success_and_strict_cooldown_apply_through_mailbox",
                "crates/bbb-native/src/services_block_list_runtime.rs#services_block_list_transfer_reconnect_rejects_stale_completion",
            ),
        ),
        ("startOnlineMode",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/services_block_list_runtime.rs#fn stop_online_mode(&mut self, world: &mut WorldStore) {",
                "crates/bbb-world/src/client/social.rs#pub fn stop_services_block_list_session(&mut self, generation: u64) {",
            ),
            (
                "crates/bbb-native/src/services_block_list_runtime.rs#services_block_list_play_reconfiguration_invalidates_pending_generation",
                "crates/bbb-native/src/services_block_list_runtime.rs#services_block_list_offline_play_and_disconnect_stay_inactive",
                "crates/bbb-world/src/client/social.rs#player_social_services_block_list_rejects_stale_or_inactive_replacement",
            ),
        ),
        ("stopOnlineMode",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/social.rs#pub fn social_players(&self, page: SocialInteractionsPage) -> Vec<SocialPlayerState> {",
            ),
            (
                "crates/bbb-world/src/client/social.rs#player_social_discovery_hidden_blocked_and_sorting_share_chat_owner",
                "crates/bbb-world/src/client/social.rs#player_info_remove_preserves_seen_and_recent_social_rows_as_removed",
            ),
        ),
        ("getHiddenPlayers",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/social.rs#pub fn social_discovered_uuid(&self, name: &str) -> Option<Uuid> {",
                "crates/bbb-world/src/client/chat.rs#fn system_chat_matches_hidden_player(&self, content: &str) -> bool {",
            ),
            (
                "crates/bbb-world/src/client/social.rs#player_social_discovery_hidden_blocked_and_sorting_share_chat_owner",
                "crates/bbb-world/src/client/chat.rs#system_visibility_and_matched_hidden_name_follow_chat_options",
            ),
        ),
        ("getDiscoveredUUID",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/social.rs#pub(crate) fn record_social_player_profile(&mut self, profile: &ProtocolGameProfile) {",
                "crates/bbb-world/src/client/player_info.rs#self.record_social_player_profile(profile);",
            ),
            (
                "crates/bbb-world/src/client/player_info.rs#player_info_adds_player_with_profile_and_fields",
                "crates/bbb-native/src/input/social.rs#player_info_remove_marks_retained_social_row_and_readd_restores_it",
            ),
        ),
        ("addPlayer",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/player_info.rs#pub fn apply_player_info_remove(&mut self, packet: ProtocolPlayerInfoRemove) -> usize {",
                "crates/bbb-native/src/input/social.rs#fn synchronize_rows(&mut self, world: &WorldStore) {",
            ),
            (
                "crates/bbb-world/src/client/social.rs#player_info_remove_preserves_seen_and_recent_social_rows_as_removed",
                "crates/bbb-native/src/input/social.rs#player_info_remove_marks_retained_social_row_and_readd_restores_it",
                "crates/bbb-world/src/client/player_info.rs#player_info_remove_clears_entry_and_listed_tracking",
            ),
        ),
        ("removePlayer",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/reporting.rs#pub struct PlayerReportingState {",
                "crates/bbb-world/src/client/chat/presentation.rs#pub(super) const MAX_CHAT_REPORT_EVENTS: usize = 1_024;",
            ),
            (
                "crates/bbb-world/src/client/reporting.rs#player_reporting_reasons_match_vanilla_type_exclusions",
                "crates/bbb-world/src/client/chat.rs#publication_snapshot_isolates_chat_delivery_queue_and_reporting",
            ),
        ),
        ("ReportingContext",),
    ),
    (
        OwnerEvidence(
            (
                "docs/unsupported-features.md#Online mode (authenticated Services and encrypted transport) is",
                "crates/bbb-world/src/client/reporting.rs#pub fn set_player_report_environment(",
            ),
            (
                "crates/bbb-world/src/client/reporting.rs#player_reporting_environment_change_invalidates_screen_and_previous_screen_state",
                "crates/bbb-world/src/client/server_presentation.rs#transfer_stores_latest_server_target",
            ),
        ),
        ("create",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/input/pause_screen.rs#fn discard_pause_draft_and_disconnect(&mut self, world: &mut WorldStore) {",
                "crates/bbb-native/src/input/reporting.rs#pub(super) fn open_player_reporting_draft_from_pause(",
            ),
            (
                "crates/bbb-native/src/input/pause_screen.rs#disconnect_with_draft_asks_before_disconnecting_and_discard_drops_the_draft",
                "crates/bbb-native/src/input/pause_screen.rs#disconnect_without_draft_requests_disconnect_directly",
                "crates/bbb-native/src/input/death_screen.rs#draft_discard_escape_continues_normal_and_hardcore_exit_paths",
            ),
        ),
        ("draftReportHandled",),
    ),
    (
        OwnerEvidence(
            (
                "docs/unsupported-features.md#Online mode (authenticated Services and encrypted transport) is",
                "crates/bbb-native/src/services_block_list_runtime.rs#pub(crate) fn synchronize_player_report_submissions(",
            ),
            (
                "crates/bbb-native/src/services_block_list_runtime.rs#player_reporting_submission_runs_async_and_applies_only_matching_attempt",
                "crates/bbb-native/src/services_block_list_runtime.rs#player_reporting_submission_cancel_aborts_task_and_rejects_late_completion",
            ),
        ),
        ("sender",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/chat/presentation.rs#pub(super) fn push(&mut self, mut event: ChatReportEventState) {",
                "crates/bbb-world/src/client/reporting/evidence.rs#pub(super) fn build_chat_evidence(",
            ),
            (
                "crates/bbb-world/src/client/reporting.rs#player_reporting_chat_builder_collects_chain_and_last_seen_oldest_first",
                "crates/bbb-world/src/client/reporting.rs#player_reporting_accepts_exact_evidence_limit_and_rejects_stale_selection",
                "crates/bbb-world/src/client/chat.rs#delayed_player_chat_defers_display_reporting_and_last_seen_until_acceptance",
            ),
        ),
        ("chatLog",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/reporting.rs#pub fn set_player_report_environment(",
                "crates/bbb-world/src/client/reporting.rs#if self.player_reporting.environment == environment {",
            ),
            (
                "crates/bbb-world/src/client/reporting.rs#player_reporting_environment_change_invalidates_screen_and_previous_screen_state",
            ),
        ),
        ("matches",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-world/src/client/reporting.rs#fn start_player_report_draft(",
                "crates/bbb-world/src/client/reporting.rs#pub fn discard_player_report_draft(&mut self, report_id: Uuid) -> bool {",
            ),
            (
                "crates/bbb-world/src/client/reporting.rs#player_reporting_skin_username_copy_discard_reset_and_publication_isolate",
                "crates/bbb-world/src/client/reporting.rs#player_reporting_rejects_offline_local_zero_and_wrong_report_mutations",
                "crates/bbb-native/src/input/reporting/tests.rs#player_reporting_chooser_editor_comment_limit_and_draft_closeout_are_generation_safe",
            ),
        ),
        ("setReportDraft",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/render_extract.rs#has_draft_report: world.player_reporting().draft.is_some(),",
                "crates/bbb-renderer/src/hud.rs#pub has_draft_report: bool,",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#hud_pause_screen_projects_draft_report_icon_and_quit_confirmation",
                "crates/bbb-native/src/runtime/tests.rs#hud_death_screen_projection_and_narration_track_main_draft_and_quit_focus",
            ),
        ),
        ("hasDraftReport",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime/render_extract.rs#.is_some_and(|draft| draft.common().reported_profile_id == player.profile.uuid),",
                "crates/bbb-renderer/src/hud/social.rs#pub has_report_draft: bool,",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs#player_reporting_projection_exposes_generation_rows_and_username_editor_controls",
                "crates/bbb-renderer/src/offscreen.rs#offscreen_social_interactions_draws_shared_rows_faces_scroll_empty_and_link_surfaces",
            ),
        ),
        ("hasDraftReportFor",),
    ),
)


AUDIO_INTEGRATED_PAUSE_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=(
        "net/minecraft/client/Minecraft.java:1156",
        "net/minecraft/client/Minecraft.java:1292-1298",
        "net/minecraft/client/sounds/SoundEngine.java:227-231,303-317,471-484",
    ),
    current_scope_reason=(
        "Vanilla only enters the paused sound branch for an unpublished integrated "
        "server; BBB is an external client and never owns or pauses that server. "
        "Closing an external-client screen therefore has no paused audio channels to resume."
    ),
    reopen_condition="BBB gains an in-process integrated server or another audio-pause owner.",
)

AUDIO_MANUAL_LOOP_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=(
        "net/minecraft/client/sounds/SoundEngine.java:262-269,318-327",
        "net/minecraft/client/resources/sounds/AbstractSoundInstance.java:69-75",
        "net/minecraft/client/resources/sounds/AbstractTickableSoundInstance.java:15-25",
        "net/minecraft/client/resources/sounds/BeeSoundInstance.java:17-27",
        "net/minecraft/client/resources/sounds/BeeAggressiveSoundInstance.java:5-13",
        "net/minecraft/client/resources/sounds/BiomeAmbientSoundsHandler.java:112-122",
        "net/minecraft/client/resources/sounds/ElytraOnPlayerSoundInstance.java:11-21",
        "net/minecraft/client/resources/sounds/GuardianAttackSoundInstance.java:13-23",
        "net/minecraft/client/resources/sounds/MinecartSoundInstance.java:16-26",
        "net/minecraft/client/resources/sounds/RidingEntitySoundInstance.java:30-40",
        "net/minecraft/client/resources/sounds/SimpleSoundInstance.java:20-119",
        "net/minecraft/client/resources/sounds/SnifferSoundInstance.java:11-21",
        "net/minecraft/client/resources/sounds/UnderwaterAmbientSoundInstances.java:10-42",
        "net/minecraft/client/multiplayer/ClientLevel.java:625-634",
        "net/minecraft/client/multiplayer/MultiPlayerGameMode.java:248-270",
    ),
    current_scope_reason=(
        "The official 26.1 client requires looping with a positive SoundInstance delay "
        "to enter manual requeue; every client-created looping instance uses delay zero, "
        "while the public SimpleSoundInstance call sites are non-looping."
    ),
    reopen_condition=(
        "A supported official-client sound instance or BBB-owned sound surface combines "
        "looping with a positive replay delay."
    ),
)

MUSIC_OPTIONS_RUNTIME_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=(
        "net/minecraft/client/Options.java:909-924,1248-1260",
        "net/minecraft/client/sounds/MusicManager.java:83-88,152-155,187-189",
        "net/minecraft/client/MusicToastDisplayState.java:23-29",
    ),
    current_scope_reason=(
        "These callbacks and captions are reached only through the in-game sound/options widgets. "
        "BBB intentionally exposes configuration at startup and consumes music frequency, toast "
        "mode, and final music volume before constructing their runtime owners."
    ),
    reopen_condition="BBB gains an in-game options UI or another runtime music-option mutator.",
)

MUSIC_MANAGER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/music_runtime.rs#pub(crate) struct MusicRuntime",
        "crates/bbb-native/src/audio_runtime.rs#pub(crate) fn counters",
        "crates/bbb-audio/src/runtime.rs#pub fn active_music_sound_name",
        "crates/bbb-world/src/toasts.rs#pub fn show_now_playing_music",
    ),
    (
        "crates/bbb-native/src/music_runtime.rs"
        "#background_selection_prefers_underwater_then_creative_then_default",
        "crates/bbb-native/src/music_runtime.rs"
        "#gain_fades_follow_vanilla_rise_fall_and_zero_stop_rules",
        "crates/bbb-native/src/music_runtime.rs"
        "#default_frequency_clamps_ranges_and_mth_inclusive_order",
        "crates/bbb-native/src/music_runtime.rs"
        "#confirmed_resolved_music_identity_drives_now_playing_show_and_explicit_hide",
        "crates/bbb-native/src/music_runtime.rs"
        "#silently_started_music_retains_identity_without_showing_now_playing",
    ),
)

MUSIC_MANAGER_RUNTIME_OPTION_EVIDENCE = OwnerEvidence(
    MUSIC_MANAGER_EVIDENCE.implementation,
    MUSIC_MANAGER_EVIDENCE.tests
    + (
        "crates/bbb-native/src/startup.rs"
        "#stored_music_options_reach_startup_and_explicit_cli_wins",
    ),
    MUSIC_OPTIONS_RUNTIME_SCOPE,
)

MUSIC_MANAGER_OWNER_EVIDENCE = _owner_evidence(
    (
        MUSIC_MANAGER_EVIDENCE,
        (
            "MusicManager",
            "tick",
            "canReplace",
            "startPlaying",
            "stopPlaying",
            "fadePlaying",
            "isPlayingMusic",
            "getCurrentMusicTranslationKey",
            "MusicFrequency",
            "getNextSongDelay",
            "getSerializedName",
        ),
    ),
    (
        MUSIC_MANAGER_RUNTIME_OPTION_EVIDENCE,
        ("showNowPlayingToastIfNeeded", "setMinutesBetweenSongs", "caption"),
    ),
)

MUSIC_TOAST_DISPLAY_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/startup.rs#pub(crate) enum MusicToastDisplayStateArg",
        "crates/bbb-native/src/runtime/toast.rs#impl NowPlayingToastRuntime",
    ),
    (
        "crates/bbb-native/src/startup.rs"
        "#startup_music_options_use_vanilla_defaults_and_cli_names",
        "crates/bbb-native/src/startup.rs"
        "#stored_music_options_reach_startup_and_explicit_cli_wins",
        "crates/bbb-native/src/runtime/toast.rs"
        "#now_playing_uses_actual_sound_key_independent_timing_and_pause_projection",
    ),
)

MUSIC_TOAST_DISPLAY_UI_EVIDENCE = OwnerEvidence(
    MUSIC_TOAST_DISPLAY_EVIDENCE.implementation,
    MUSIC_TOAST_DISPLAY_EVIDENCE.tests,
    MUSIC_OPTIONS_RUNTIME_SCOPE,
)

MUSIC_TOAST_DISPLAY_OWNER_EVIDENCE = _owner_evidence(
    (
        MUSIC_TOAST_DISPLAY_EVIDENCE,
        (
            "MusicToastDisplayState",
            "getSerializedName",
            "renderInPauseScreen",
            "renderToast",
        ),
    ),
    (MUSIC_TOAST_DISPLAY_UI_EVIDENCE, ("text", "tooltip")),
)

AUDIO_SOUND_MANAGER_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/music_runtime.rs#fn situational_music",
                "crates/bbb-native/src/music_runtime.rs#fn music_volume",
            ),
            (
                "crates/bbb-native/src/music_runtime.rs"
                "#background_selection_prefers_underwater_then_creative_then_default",
                "crates/bbb-native/src/music_runtime.rs"
                "#gain_fades_follow_vanilla_rise_fall_and_zero_stop_rules",
            ),
        ),
        ("getMusicVolume", "getSituationalMusic"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/audio_runtime.rs#pub(crate) fn load(",
                "crates/bbb-audio/src/runtime.rs#pub fn new_with_device_preference",
                "crates/bbb-native/src/main.rs#NativeAudioRuntime::load(",
            ),
            (
                "crates/bbb-native/src/audio_runtime.rs"
                "#native_audio_catalog_loader_errors_before_kira_when_catalog_is_empty",
                "crates/bbb-pack/src/sounds.rs"
                "#required_sound_catalog_loads_minimal_valid_catalog",
                "crates/bbb-audio/src/device.rs#only_the_empty_name_selects_the_default_device",
            ),
        ),
        ("SoundEngine", "SoundManager"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/runtime.rs#playback_events: VecDeque<AudioPlaybackEvent>",
                "crates/bbb-native/src/audio_runtime.rs#pub(crate) fn drain_subtitle_events",
                "crates/bbb-native/src/main.rs#if args.client_show_subtitles {",
            ),
            (
                "crates/bbb-audio/src/runtime.rs"
                "#playback_event_local_and_spatial_ranges_match_vanilla_subtitle_listener",
                "crates/bbb-native/src/audio_runtime.rs"
                "#audio_runtime_maps_playback_root_event_to_current_subtitle_key",
                "crates/bbb-native/src/sound_subtitle_runtime.rs"
                "#sound_subtitle_projection_uses_strict_range_and_listener_directions",
            ),
        ),
        ("addEventListener", "addListener", "removeEventListener", "removeListener"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/runtime.rs#fn channel_playback_rate",
                "crates/bbb-native/src/audio_runtime/resolver.rs#playback_rate:",
            ),
            (
                "crates/bbb-audio/src/runtime.rs#playback_rate_clamps_to_vanilla_pitch_range",
                "crates/bbb-native/src/audio_runtime/resolver.rs"
                "#event_reference_modifiers_wrap_resolved_file_sound",
            ),
        ),
        ("calculatePitch",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/command.rs#pub fn final_source_volume",
                "crates/bbb-native/src/startup.rs#fn stored_audio_volume_settings",
                "crates/bbb-native/src/audio_runtime.rs#volume_settings: AudioVolumeSettings",
                "crates/bbb-native/src/audio_runtime/resolver.rs#pub fn with_jukebox_registry",
            ),
            (
                "crates/bbb-audio/src/command.rs"
                "#final_source_volume_matches_vanilla_master_and_category_mix",
                "crates/bbb-native/src/startup.rs"
                "#stored_audio_volumes_cover_every_vanilla_source_with_safe_defaults",
                "crates/bbb-native/src/audio_runtime/resolver.rs"
                "#category_volume_settings_affect_channel_gain_not_instance_gain",
                "crates/bbb-native/src/audio_runtime.rs"
                "#every_native_audio_resolution_uses_the_startup_volume_settings",
            ),
        ),
        ("calculateVolume",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/audio_runtime.rs#fn stop_all_sounds",
                "crates/bbb-audio/src/runtime.rs#fn stop_sounds",
                "crates/bbb-audio/src/preload.rs#impl Drop for AudioPreloadWorker",
                "crates/bbb-native/src/main.rs#Event::LoopExiting",
            ),
            (
                "crates/bbb-audio/src/runtime.rs"
                "#stop_sound_only_stop_all_clears_delayed_positioned_sounds",
                "crates/bbb-native/src/runtime/tests.rs"
                "#remote_disconnect_publishes_terminal_screen_before_input_without_exit_fallback",
                "crates/bbb-native/src/network_runtime.rs"
                "#production_shutdown_has_one_owner_and_frozen_order",
            ),
        ),
        ("destroy", "emergencyShutdown", "stopAll"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/device.rs#pub struct AudioDeviceObservation",
                "crates/bbb-native/src/audio_runtime.rs#fn project_audio_device_observation",
            ),
            (
                "crates/bbb-audio/src/device.rs#only_the_empty_name_selects_the_default_device",
                "crates/bbb-native/src/audio_runtime.rs"
                "#device_observation_projects_cached_receipt_fields",
            ),
        ),
        ("getAvailableSoundDevices",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/runtime.rs#impl AudioChannelLimits",
                "crates/bbb-audio/src/runtime.rs#fn try_admit_channel",
                "crates/bbb-native/src/audio_runtime.rs#fn project_audio_channel_observation",
                "crates/bbb-native/src/audio_runtime.rs#fn project_audio_preload_observation",
                "crates/bbb-native/src/runtime.rs#fn hud_debug_sound_cache_line",
            ),
            (
                "crates/bbb-audio/src/runtime.rs"
                "#channel_limits_match_vanilla_formula_and_isolate_pools",
                "crates/bbb-audio/src/runtime.rs"
                "#kira_capacity_has_headroom_for_both_logical_pools",
                "crates/bbb-native/src/audio_runtime.rs"
                "#audio_channel_diagnostics_project_independent_pool_receipt",
                "crates/bbb-native/src/runtime/tests.rs"
                "#hud_debug_overlay_projects_custom_sound_cache_under_reduced_debug_info",
            ),
        ),
        ("getChannelDebugString", "getSoundCacheDebugStats"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/runtime.rs#listener_position:",
                "crates/bbb-audio/src/runtime.rs#fn listener_orientation",
                "crates/bbb-native/src/simulation/effects.rs#fn tick_entity_sound_positions",
            ),
            (
                "crates/bbb-native/src/runtime/tests.rs"
                "#audio_scene_command_tracks_listener_and_entity_positions",
                "crates/bbb-audio/src/runtime.rs"
                "#listener_orientation_matches_kira_forward_at_minecraft_north",
            ),
        ),
        ("getListenerTransform", "updateSource"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/runtime.rs#fn stopped_sound_is_retained",
                "crates/bbb-audio/src/runtime.rs#fn latest_retained_music_event_id",
                "crates/bbb-native/src/audio_runtime.rs#active_tickable_sounds:",
            ),
            (
                "crates/bbb-audio/src/runtime.rs"
                "#stopped_instance_shell_remains_projected_until_twentieth_tick",
                "crates/bbb-audio/src/runtime.rs"
                "#backend_music_identity_retains_stopped_instance_until_delete_tick",
                "crates/bbb-native/src/audio_runtime.rs"
                "#tickable_sound_owner_requires_start_after_event_switch_or_reload",
            ),
        ),
        ("isActive",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/runtime.rs#pub fn reset_resource_generation",
                "crates/bbb-native/src/audio_runtime.rs#pub(crate) fn commit_reload",
                "crates/bbb-native/src/resource_reload.rs#audio.commit_reload(reload);",
            ),
            (
                "crates/bbb-audio/src/runtime.rs"
                "#generation_reset_cancels_pending_preload_and_rejects_old_requests",
                "crates/bbb-audio/src/runtime.rs"
                "#resource_generation_reset_preserves_engine_clock_and_diagnostics",
                "crates/bbb-native/src/resource_reload.rs"
                "#renderer_resource_owners_commit_inside_one_reload_barrier",
            ),
        ),
        ("loadLibrary", "reload"),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-native/src/input/pause_screen.rs#native_pause_freezes_client_ticks",),
            (
                "crates/bbb-native/src/runtime/tests.rs"
                "#native_pause_screens_never_freeze_external_client_or_running_level_ticks",
            ),
            AUDIO_INTEGRATED_PAUSE_SCOPE,
        ),
        ("pauseAllExcept", "resume", "tickMusicWhenPaused"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/audio_runtime/resolver.rs#pub fn play_positioned_sound",
                "crates/bbb-audio/src/runtime.rs#fn play_spatial_sound",
                "crates/bbb-audio/src/runtime.rs#fn play_local_sound_with_role",
            ),
            (
                "crates/bbb-native/src/audio_runtime/resolver.rs"
                "#resolves_direct_positioned_sound_through_event_reference",
                "crates/bbb-native/src/audio_runtime/resolver.rs"
                "#resolves_direct_local_sound_without_spatial_position",
                "crates/bbb-audio/src/runtime.rs#non_music_zero_volume_sounds_do_not_start",
                "crates/bbb-audio/src/runtime.rs"
                "#playback_event_local_and_spatial_ranges_match_vanilla_subtitle_listener",
            ),
        ),
        ("play",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/runtime.rs#fn vanilla_distance_delay_ticks",
                "crates/bbb-audio/src/runtime.rs#fn advance_delayed_positioned_sounds",
            ),
            (
                "crates/bbb-audio/src/runtime.rs"
                "#distance_delay_ticks_match_vanilla_threshold_and_flooring",
                "crates/bbb-audio/src/runtime.rs"
                "#delayed_positioned_sounds_release_after_remaining_ticks",
            ),
        ),
        ("playDelayed",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/audio_runtime.rs#active_tickable_sounds:",
                "crates/bbb-audio/src/runtime.rs#fn update_tickable_entity_sound",
            ),
            (
                "crates/bbb-native/src/audio_runtime.rs"
                "#tickable_sound_owner_requires_start_after_event_switch_or_reload",
                "crates/bbb-native/src/audio_runtime/resolver.rs"
                "#resolves_tickable_entity_sound_updates_with_current_gain_pitch_and_position",
            ),
        ),
        ("queueTickingSound",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/audio_runtime.rs#fn set_music_gain",
                "crates/bbb-audio/src/runtime.rs#fn set_music_gain",
            ),
            (
                "crates/bbb-audio/src/runtime.rs"
                "#music_gain_clamps_and_only_scales_music_category_handles",
                "crates/bbb-native/src/music_runtime.rs"
                "#gain_fades_follow_vanilla_rise_fall_and_zero_stop_rules",
            ),
        ),
        ("refreshCategoryVolume", "updateCategoryVolume"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-pack/src/sounds.rs#pub fn preload_files",
                "crates/bbb-audio/src/preload.rs#pub(crate) fn submit",
                "crates/bbb-native/src/audio_runtime.rs#fn audio_preload_requests",
            ),
            (
                "crates/bbb-pack/src/sounds.rs"
                "#preload_plan_matches_vanilla_complete_buffer_queue_and_deduplicates_resource_ids",
                "crates/bbb-audio/src/preload.rs"
                "#latest_job_cancels_or_supersedes_older_generation_without_publishing_it",
                "crates/bbb-audio/src/runtime.rs"
                "#preload_completion_installs_current_generation_without_a_playback_decode_miss",
            ),
        ),
        ("requestPreload",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/runtime.rs#if looping { data.loop_region(..) } else { data }",
                "crates/bbb-native/src/environment_audio_runtime.rs#fn advance_loop_sounds",
            ),
            (
                "crates/bbb-audio/src/runtime.rs"
                "#ambient_loop_sync_updates_retained_stops_omitted_and_starts_new",
                "crates/bbb-native/src/environment_audio_runtime.rs"
                "#environment_audio_cross_fades_loops_over_40_ticks_and_reuses_a_fading_holder",
            ),
        ),
        ("shouldLoopAutomatically",),
    ),
    (
        OwnerEvidence(
            ("crates/bbb-audio/src/runtime.rs#if looping { data.loop_region(..) } else { data }",),
            (
                "crates/bbb-audio/src/runtime.rs"
                "#ambient_loop_sync_updates_retained_stops_omitted_and_starts_new",
            ),
            AUDIO_MANUAL_LOOP_SCOPE,
        ),
        ("requiresManualLooping", "shouldLoopManually"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-audio/src/device.rs#pub fn audio_device_reload_reason",
                "crates/bbb-audio/src/runtime.rs#pub fn poll_audio_device",
            ),
            (
                "crates/bbb-audio/src/device.rs#reload_decision_preserves_vanilla_branch_order",
                "crates/bbb-audio/src/device.rs"
                "#a_changed_irrelevant_device_does_not_reload_the_current_output",
            ),
        ),
        ("shouldChangeDevice",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/audio_runtime/resolver.rs#pub fn stop_sound",
                "crates/bbb-audio/src/runtime.rs#fn stop_sounds",
                "crates/bbb-native/src/audio_runtime.rs#fn stop_all_sounds",
            ),
            (
                "crates/bbb-native/src/audio_runtime/resolver.rs#represents_stop_sound_commands",
                "crates/bbb-audio/src/runtime.rs#stop_filter_matches_source_name_and_all",
                "crates/bbb-audio/src/runtime.rs"
                "#stop_sound_only_stop_all_clears_delayed_positioned_sounds",
            ),
        ),
        ("stop",),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/runtime.rs#audio_events.tick_entity_sound_positions(",
                "crates/bbb-native/src/audio_runtime.rs#fn tick_entity_sound_positions",
                "crates/bbb-audio/src/runtime.rs#fn update_tickable_entity_sound",
            ),
            (
                "crates/bbb-native/src/music_runtime.rs"
                "#music_effects_enter_fifo_before_entity_sound_position_tick",
                "crates/bbb-native/src/environment_audio_runtime.rs"
                "#environment_audio_ticks_after_player_movement_and_before_music",
                "crates/bbb-audio/src/runtime.rs"
                "#stopped_instance_shell_remains_projected_until_twentieth_tick",
                "crates/bbb-native/src/audio_runtime/resolver.rs"
                "#resolves_tickable_entity_sound_updates_with_current_gain_pitch_and_position",
            ),
        ),
        ("tick", "tickInGameSound"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-native/src/audio_runtime.rs#pub(crate) fn available_sound_ids",
                "crates/bbb-world/src/commands.rs#pub fn set_available_command_sounds",
            ),
            (
                "crates/bbb-pack/src/sounds.rs"
                "#indexed_vanilla_26_1_sounds_catalog_loads_representative_events",
                "crates/bbb-world/src/commands.rs"
                "#resource_sound_completion_view_is_publication_shared_and_generation_isolated",
            ),
        ),
        ("getAvailableSounds", "getSoundEvent"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-pack/src/sounds.rs#fn merge_selected_resource_stack",
                "crates/bbb-native/src/audio_runtime.rs#pub(crate) fn prepare_reload",
                "crates/bbb-native/src/resource_reload.rs#fn prepare_native_resource_reload",
            ),
            (
                "crates/bbb-pack/src/sounds.rs"
                "#sound_catalog_merges_resource_stack_definitions_in_pack_order",
                "crates/bbb-pack/src/sounds.rs"
                "#sound_catalog_replace_clears_lower_priority_sounds",
                "crates/bbb-native/src/resource_reload.rs"
                "#renderer_resource_owners_commit_inside_one_reload_barrier",
            ),
        ),
        ("apply", "prepare", "handleRegistration", "listResources"),
    ),
    (
        OwnerEvidence(
            (
                "crates/bbb-pack/src/sounds.rs#fn sound_ogg_resource",
                "crates/bbb-pack/src/sounds.rs#fn sound_resource_location",
            ),
            (
                "crates/bbb-pack/src/sounds.rs"
                "#sound_catalog_skips_invalid_resource_stack_definitions",
                "crates/bbb-pack/src/sounds.rs"
                "#sound_catalog_skips_missing_file_sounds_from_resource_stack",
                "crates/bbb-pack/src/sounds.rs#rejects_invalid_sound_entry_values",
            ),
        ),
        ("validateSoundResource",),
    ),
)


AUDIO_OPENAL_CONTEXT_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-pack/src/openal_native.rs#pub fn materialize_verified_openal_library",
        "crates/bbb-audio/src/openal.rs#pub struct OpenAlDeviceContext",
        "crates/bbb-audio/src/openal.rs#fn finish_context_initialization",
        "crates/bbb-audio/src/runtime.rs#pub fn new_with_device_preference",
        "crates/bbb-native/src/startup.rs#client_directional_audio",
    ),
    (
        "crates/bbb-audio/src/openal.rs"
        "#context_attributes_match_vanilla_supported_hrtf_fallback",
        "crates/bbb-audio/src/openal.rs"
        "#verified_openal_library_executes_both_hrtf_context_branches",
        "crates/bbb-audio/src/openal.rs"
        "#verified_openal_sources_restore_their_context_across_device_reload",
        "crates/bbb-pack/src/openal_native.rs"
        "#verified_workspace_materializes_manifest_owned_openal",
        "crates/bbb-native/src/startup.rs"
        "#stored_directional_audio_reaches_startup_and_explicit_cli_wins",
    ),
)

AUDIO_OPENAL_CHANNEL_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-audio/src/openal.rs#pub(crate) struct OpenAlBackend",
        "crates/bbb-audio/src/openal.rs#pub(crate) struct OpenAlSoundSource",
        "crates/bbb-audio/src/openal.rs#fn upload_pcm_buffer",
        "crates/bbb-audio/src/residency.rs#pub(crate) struct OpenAlStreamDecoder",
        "crates/bbb-audio/src/runtime.rs#fn play_spatial_sound",
    ),
    (
        "crates/bbb-audio/src/openal.rs"
        "#verified_openal_channel_executes_static_and_four_buffer_stream_owners",
        "crates/bbb-audio/src/runtime.rs"
        "#stopped_openal_channel_returns_pool_slot_while_instance_shell_remains",
        "crates/bbb-audio/src/residency.rs"
        "#pcm16_conversion_matches_vanilla_chunked_sample_buffer",
        "crates/bbb-audio/src/runtime.rs"
        "#stopped_instance_shell_remains_projected_until_twentieth_tick",
        "crates/bbb-audio/src/runtime.rs"
        "#channel_limits_match_vanilla_formula_and_isolate_pools",
    ),
)

AUDIO_OPENAL_DEVICE_TRACKER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-audio/src/openal.rs"
        "#pub(crate) fn install_system_event_tracker",
        "crates/bbb-audio/src/device.rs#pub(crate) struct AudioDeviceTracker",
        "crates/bbb-audio/src/runtime.rs#pub fn poll_audio_device",
    ),
    (
        "crates/bbb-audio/src/openal.rs"
        "#system_event_callback_requests_only_playback_updates",
        "crates/bbb-audio/src/openal.rs"
        "#verified_openal_device_tracker_executes_callback_or_polling_selection",
        "crates/bbb-audio/src/device.rs"
        "#verified_device_tracker_runs_selected_mode_and_forced_refresh",
        "crates/bbb-audio/src/device.rs"
        "#reload_decision_preserves_vanilla_branch_order",
    ),
)

AUDIO_OPENAL_LISTENER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-audio/src/openal.rs#pub(crate) fn set_listener",
        "crates/bbb-audio/src/runtime.rs#fn replace_openal_backend",
        "crates/bbb-audio/src/runtime.rs#fn tick_entity_sound_positions",
    ),
    (
        "crates/bbb-audio/src/openal.rs"
        "#verified_openal_channel_executes_static_and_four_buffer_stream_owners",
        "crates/bbb-audio/src/runtime.rs"
        "#listener_orientation_matches_kira_forward_at_minecraft_north",
    ),
)

AUDIO_OPENAL_CLEANUP_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-audio/src/openal.rs#impl Drop for OpenAlBackend",
        "crates/bbb-audio/src/openal.rs#impl Drop for OpenAlSoundSource",
        "crates/bbb-audio/src/openal.rs#impl Drop for OpenAlDeviceContext",
        "crates/bbb-audio/src/runtime.rs#fn replace_openal_backend",
    ),
    (
        "crates/bbb-audio/src/openal.rs"
        "#verified_openal_sources_restore_their_context_across_device_reload",
        "crates/bbb-audio/src/runtime.rs"
        "#stopped_openal_channel_returns_pool_slot_while_instance_shell_remains",
    ),
)

AUDIO_OPENAL_POOL_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-audio/src/runtime.rs#struct AudioChannelLimits",
        "crates/bbb-audio/src/runtime.rs#fn try_admit_channel",
        "crates/bbb-audio/src/runtime.rs#fn release_stopped_channel",
        "crates/bbb-native/src/audio_runtime.rs#fn project_audio_channel_observation",
    ),
    (
        "crates/bbb-audio/src/runtime.rs"
        "#channel_limits_match_vanilla_formula_and_isolate_pools",
        "crates/bbb-audio/src/runtime.rs"
        "#stopped_openal_channel_returns_pool_slot_while_instance_shell_remains",
        "crates/bbb-native/src/audio_runtime.rs"
        "#audio_channel_diagnostics_project_independent_pool_receipt",
    ),
)

AUDIO_LIBRARY_OWNER_EVIDENCE = _owner_evidence(
    (
        AUDIO_OPENAL_CONTEXT_EVIDENCE,
        ("init", "createAttributes", "getChannelCount"),
    ),
    (
        AUDIO_SOUND_MANAGER_OWNER_EVIDENCE["getAvailableSoundDevices"],
        (
            "currentDeviceName",
            "queryDeviceName",
            "openDeviceOrFallback",
            "tryOpenDevice",
        ),
    ),
    (
        AUDIO_OPENAL_DEVICE_TRACKER_EVIDENCE,
        ("isCurrentDeviceDisconnected", "createDeviceTracker"),
    ),
    (
        AUDIO_OPENAL_CLEANUP_EVIDENCE,
        ("cleanup",),
    ),
    (
        AUDIO_OPENAL_LISTENER_EVIDENCE,
        ("getListener",),
    ),
    (
        AUDIO_OPENAL_POOL_EVIDENCE,
        (
            "acquireChannel",
            "releaseChannel",
            "getChannelDebugString",
            "acquire",
            "release",
            "getMaxCount",
            "getUsedCount",
            "CountingChannelPool",
        ),
    ),
)

AUDIO_CHANNEL_OWNER_EVIDENCE = _owner_evidence(
    (
        AUDIO_OPENAL_CHANNEL_EVIDENCE,
        (
            "create",
            "Channel",
            "destroy",
            "play",
            "getState",
            "stop",
            "playing",
            "stopped",
            "setSelfPosition",
            "setPitch",
            "setLooping",
            "setVolume",
            "disableAttenuation",
            "linearAttenuation",
            "setRelative",
            "attachStaticBuffer",
            "attachBufferStream",
            "calculateBufferSize",
            "pumpBuffers",
            "updateStream",
            "removeProcessedBuffers",
        ),
    ),
    (
        AUDIO_SOUND_MANAGER_OWNER_EVIDENCE["resume"],
        ("pause", "unpause"),
    ),
)

AUDIO_LISTENER_OWNER_EVIDENCE = _owner_evidence(
    (
        AUDIO_OPENAL_LISTENER_EVIDENCE,
        ("setTransform", "reset", "getTransform"),
    ),
)

AUDIO_OPENAL_SOUND_BUFFER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-audio/src/openal.rs#struct OpenAlStaticBuffer",
        "crates/bbb-audio/src/openal.rs#fn upload_pcm_buffer",
        "crates/bbb-audio/src/openal.rs#fn ensure_static_buffer_capacity",
        "crates/bbb-audio/src/residency.rs#pub(crate) struct OpenAlStaticSound",
    ),
    (
        "crates/bbb-audio/src/openal.rs"
        "#verified_openal_channel_executes_static_and_four_buffer_stream_owners",
        "crates/bbb-audio/src/residency.rs"
        "#pcm16_conversion_matches_vanilla_chunked_sample_buffer",
        "crates/bbb-audio/src/residency.rs"
        "#capacity_evicts_only_inactive_lru_and_rejects_when_all_are_pinned",
    ),
)

AUDIO_SOUND_BUFFER_OWNER_EVIDENCE = _owner_evidence(
    (
        AUDIO_OPENAL_SOUND_BUFFER_EVIDENCE,
        (
            "SoundBuffer",
            "getAlBuffer",
            "discardAlBuffer",
            "releaseAlBuffer",
            "format",
            "size",
            "isValid",
        ),
    ),
)

AUDIO_OPENAL_CHANNEL_ACCESS_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-audio/src/runtime.rs#enum KiraSoundHandle",
        "crates/bbb-audio/src/runtime.rs#fn release_stopped_channel",
        "crates/bbb-audio/src/runtime.rs#fn update_stream",
        "crates/bbb-audio/src/runtime.rs#fn replace_openal_backend",
        "crates/bbb-audio/src/openal.rs#impl Drop for OpenAlSoundSource",
    ),
    (
        "crates/bbb-audio/src/runtime.rs"
        "#stopped_openal_channel_returns_pool_slot_while_instance_shell_remains",
        "crates/bbb-audio/src/openal.rs"
        "#verified_openal_channel_executes_static_and_four_buffer_stream_owners",
        "crates/bbb-audio/src/runtime.rs"
        "#stopped_instance_shell_remains_projected_until_twentieth_tick",
    ),
)

AUDIO_CHANNEL_ACCESS_OWNER_EVIDENCE = _owner_evidence(
    (
        AUDIO_OPENAL_CHANNEL_ACCESS_EVIDENCE,
        (
            "ChannelAccess",
            "createHandle",
            "executeOnChannels",
            "scheduleTick",
            "clear",
            "isStopped",
            "ChannelHandle",
            "execute",
            "release",
        ),
    ),
)

AUDIO_SOUND_BUFFER_LIBRARY_OWNER_EVIDENCE = _owner_evidence(
    (
        AUDIO_SOUND_MANAGER_OWNER_EVIDENCE["requestPreload"],
        (
            "SoundBufferLibrary",
            "getCompleteBuffer",
            "getStream",
            "clear",
            "preload",
            "enumerate",
            "accountBuffer",
            "totalCount",
            "totalSize",
        ),
    ),
)

SERVER_PACK_LOCAL_WORLD_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=(
        "net/minecraft/client/resources/server/DownloadedPackSource.java:313-315,369-389",
        "net/minecraft/client/resources/server/ServerPackManager.java:58-74",
        "net/minecraft/client/gui/screens/worldselection/WorldOpenFlows.java:595-597",
        "com/mojang/realmsclient/util/task/GetServerDetailsTask.java:149-153",
    ),
    current_scope_reason=(
        "The local-path pack entry is called by integrated-world loading, while the only "
        "other wait-for-feedback caller is Realms. Both are outside the declared external "
        "multiplayer client surface; ordinary server-controlled packs use pushPack and packet feedback."
    ),
    reopen_condition="BBB adds integrated-world or Realms connection support.",
)

HTTP_UTIL_CONSTRUCTOR_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=("net/minecraft/util/HttpUtil.java:32-33",),
    current_scope_reason=(
        "The private utility constructor has an empty body and no state projection; the "
        "reachable static download methods are inventoried separately."
    ),
    reopen_condition="The constructor gains a body or becomes reachable.",
)

SERVER_PACK_SCOPE_TEST = (
    "scripts/tests/test_client_mechanism_inventory.py"
    "#test_server_pack_owner_census_has_complete_evidence_and_scope_rulings"
)

SERVER_PACK_TOAST_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/resource_pack_runtime.rs#fn apply_download_progress",
        "crates/bbb-world/src/client/server_presentation.rs"
        "#pub fn set_resource_pack_download_state",
        "crates/bbb-native/src/runtime/toast.rs#impl ClientToastRuntime",
        "crates/bbb-renderer/src/hud.rs#pub enum HudToast",
    ),
    (
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#download_progress_marks_only_vanilla_toast_mutation_callbacks",
        "crates/bbb-world/src/client/server_presentation.rs"
        "#download_toast_events_dedupe_snapshots_and_bind_notifier_generation",
        "crates/bbb-native/src/runtime/tests.rs"
        "#server_pack_download_progress_projects_vanilla_system_toast_and_forced_hide",
        "crates/bbb-native/src/runtime/tests.rs"
        "#mixed_system_toasts_share_fifo_and_multiline_contiguous_slots",
        "crates/bbb-native/src/runtime/tests.rs"
        "#system_add_or_update_keeps_slot_and_construction_width_while_resetting_lifetime",
        "crates/bbb-native/src/runtime/tests.rs"
        "#shared_toast_projection_preserves_mixed_family_insertion_order",
        "crates/bbb-renderer/src/hud.rs"
        "#unified_toast_sanitizer_preserves_mixed_order_and_rejects_overlap",
        "crates/bbb-renderer/src/hud.rs"
        "#multiline_system_toast_uses_frozen_lines_and_height_scaled_slot_position",
        "crates/bbb-world/src/level.rs"
        "#play_login_retains_toast_manager_but_level_clear_drops_it",
    ),
)

TOAST_OWNER_CENSUS_TEST = (
    "scripts/tests/test_client_mechanism_inventory.py"
    "#test_toast_owner_census_has_complete_evidence_scope_and_no_open_gaps"
)

HUD_TOAST_SHARED_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-world/src/toasts.rs#pub enum ClientToastEventKind",
        "crates/bbb-native/src/runtime/toast.rs#impl ClientToastRuntime",
        "crates/bbb-native/src/runtime/render_extract.rs#pub(crate) struct RendererFrame",
        "crates/bbb-renderer/src/hud.rs#pub enum HudToast",
    ),
    (
        "crates/bbb-world/src/toasts.rs#all_toast_families_share_one_call_site_order",
        "crates/bbb-native/src/runtime/tests.rs"
        "#shared_toast_projection_preserves_mixed_family_insertion_order",
        "crates/bbb-native/src/runtime/tests.rs"
        "#mixed_system_toasts_share_fifo_and_multiline_contiguous_slots",
        "crates/bbb-native/src/runtime/tests.rs"
        "#advancement_toast_runtime_admits_five_fifo_slots_and_matches_vanilla_timing",
        "crates/bbb-renderer/src/hud.rs"
        "#unified_toast_sanitizer_preserves_mixed_order_and_rejects_overlap",
    ),
)

HUD_SYSTEM_TOAST_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime/toast.rs#struct SystemToastState",
        "crates/bbb-native/src/runtime/toast.rs#fn system_toast_state",
        "crates/bbb-renderer/src/hud.rs#pub struct HudSystemToast",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs"
        "#system_add_or_update_keeps_slot_and_construction_width_while_resetting_lifetime",
        "crates/bbb-native/src/runtime/tests.rs"
        "#low_disk_space_toast_uses_ten_second_visible_window_and_square_slides",
        "crates/bbb-renderer/src/hud.rs"
        "#multiline_system_toast_uses_frozen_lines_and_height_scaled_slot_position",
    ),
)

HUD_DYNAMIC_SYSTEM_TITLE_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime/toast.rs#struct SystemToastTitle",
        "crates/bbb-native/src/runtime/toast.rs#fn project_system",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs"
        "#system_toast_title_relocalizes_while_message_lines_and_width_stay_frozen",
        "crates/bbb-native/src/runtime/tests.rs"
        "#system_toast_title_arguments_relocalize_without_rebuilding_download_content",
    ),
)

HUD_ADVANCEMENT_TOAST_EVIDENCE = OwnerEvidence(
    ("crates/bbb-native/src/runtime/toast.rs#fn project_advancement",),
    (
        "crates/bbb-native/src/runtime/tests.rs"
        "#advancement_toast_runtime_admits_five_fifo_slots_and_matches_vanilla_timing",
        "crates/bbb-native/src/runtime/tests.rs"
        "#advancement_toast_and_screen_preserve_rich_title_runs",
    ),
)

HUD_RECIPE_TOAST_EVIDENCE = OwnerEvidence(
    ("crates/bbb-native/src/runtime/toast.rs#struct RecipeToastState",),
    (
        "crates/bbb-native/src/runtime/tests.rs"
        "#recipe_toast_runtime_coalesces_resets_lifetime_cycles_and_revives",
        "crates/bbb-native/src/runtime/tests.rs"
        "#recipe_toast_runtime_coalesces_while_queued_and_keeps_global_fifo_order",
    ),
)

HUD_TUTORIAL_TOAST_EVIDENCE = OwnerEvidence(
    ("crates/bbb-native/src/runtime/toast.rs#struct TutorialToastState",),
    (
        "crates/bbb-native/src/runtime/tests.rs"
        "#tutorial_toast_uses_contiguous_slots_allows_later_small_toast_bypass_and_smooths_progress",
        "crates/bbb-native/src/runtime/tests.rs"
        "#social_tutorial_toast_freezes_localized_key_content_and_hides_strictly_after_8000ms",
        "crates/bbb-native/src/runtime/tests.rs"
        "#social_interactions_hide_mutates_the_same_visible_tutorial_before_screen_open",
        "crates/bbb-renderer/src/hud.rs"
        "#tutorial_toast_height_slot_layout_and_progress_follow_vanilla",
    ),
)

HUD_TIMED_TUTORIAL_TOAST_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-world/src/toasts.rs"
        "#pub fn show_social_interactions_tutorial_if_first_join",
        "crates/bbb-native/src/runtime.rs"
        "#world.show_social_interactions_tutorial_if_first_join()",
        "crates/bbb-native/src/runtime/toast.rs#struct TutorialToastState",
        "crates/bbb-native/src/input.rs#fn open_social_interactions_screen",
        "crates/bbb-native/src/startup.rs#pub(crate) client_joined_first_server",
        "crates/bbb-native/src/client_options.rs#pub(crate) struct ClientOptionWrite",
        "crates/bbb-renderer/src/hud.rs#pub enum HudTutorialToastIcon",
    ),
    (
        "crates/bbb-world/src/tutorial.rs"
        "#first_multiplayer_tick_owns_one_timed_social_toast_and_persists_across_disconnect",
        "crates/bbb-native/src/runtime/tests.rs"
        "#social_tutorial_toast_freezes_localized_key_content_and_hides_strictly_after_8000ms",
        "crates/bbb-native/src/runtime/tests.rs"
        "#social_interactions_hide_mutates_the_same_visible_tutorial_before_screen_open",
        "crates/bbb-native/src/runtime/tests.rs"
        "#first_multiplayer_tutorial_queues_the_joined_sentinel_in_the_same_effect_batch_once",
        "crates/bbb-native/src/input/social.rs"
        "#social_interactions_default_p_key_opens_and_closes_away_from_search",
        "crates/bbb-native/src/startup.rs"
        "#joined_first_server_is_consumed_at_startup_and_explicit_cli_wins",
        "crates/bbb-native/src/client_options.rs"
        "#first_server_tutorial_sentinel_rewrites_and_survives_reload",
        "crates/bbb-native/src/component_text.rs"
        "#bound_key_display_keys_follow_rebinds_and_defaults",
    ),
)

HUD_TUTORIAL_CONSTRUCTOR_EVIDENCE = OwnerEvidence(
    HUD_TUTORIAL_TOAST_EVIDENCE.implementation
    + HUD_TIMED_TUTORIAL_TOAST_EVIDENCE.implementation,
    HUD_TUTORIAL_TOAST_EVIDENCE.tests + HUD_TIMED_TUTORIAL_TOAST_EVIDENCE.tests,
)

HUD_PERIODIC_TOAST_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime/periodic_notifications.rs"
        "#struct PeriodicNotificationRuntime",
        "crates/bbb-native/src/runtime/toast.rs#fn ingest_periodic_notification",
    ),
    (
        "crates/bbb-native/src/runtime/periodic_notifications.rs"
        "#due_notification_projects_through_the_shared_system_toast_surface",
        "crates/bbb-native/src/runtime/tests.rs"
        "#periodic_add_queues_distinct_toasts_and_display_multiplier_scales_system_lifetime",
    ),
)

HUD_NOW_PLAYING_TOAST_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-world/src/toasts.rs#pub struct ClientNowPlayingToastState",
        "crates/bbb-native/src/music_runtime.rs#pub(crate) struct MusicRuntime",
        "crates/bbb-native/src/runtime/toast.rs#pub(super) struct NowPlayingToastRuntime",
        "crates/bbb-native/src/hud_assets.rs#toast/now_playing",
        "crates/bbb-renderer/src/hud.rs#pub struct HudNowPlayingToast",
    ),
    (
        "crates/bbb-pack/tests/vanilla_gui_music_assets.rs"
        "#vanilla_now_playing_gui_assets_preserve_animation_and_nine_slice_metadata",
        "crates/bbb-audio/src/runtime.rs"
        "#stopped_instance_shell_remains_projected_until_twentieth_tick",
        "crates/bbb-native/src/music_runtime.rs"
        "#confirmed_resolved_music_identity_drives_now_playing_show_and_explicit_hide",
        "crates/bbb-native/src/music_runtime.rs"
        "#silently_started_music_retains_identity_without_showing_now_playing",
        "crates/bbb-native/src/runtime/toast.rs"
        "#now_playing_uses_actual_sound_key_independent_timing_and_pause_projection",
        "crates/bbb-native/src/runtime/toast.rs"
        "#now_playing_translation_and_music_note_palette_match_vanilla",
        "crates/bbb-renderer/src/hud.rs"
        "#now_playing_uses_left_slide_static_pause_and_dynamic_vanilla_width",
        "crates/bbb-renderer/src/hud.rs"
        "#hide_gui_suppresses_ingame_draws_and_toasts_but_keeps_sleep_and_debug",
    ),
)

HUD_TOAST_MANAGER_WITH_NOW_PLAYING_EVIDENCE = OwnerEvidence(
    HUD_TOAST_SHARED_EVIDENCE.implementation + HUD_NOW_PLAYING_TOAST_EVIDENCE.implementation,
    HUD_TOAST_SHARED_EVIDENCE.tests + HUD_NOW_PLAYING_TOAST_EVIDENCE.tests,
)

MUSIC_TOAST_RUNTIME_OPTION_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=(
        "net/minecraft/client/gui/components/toasts/ToastManager.java:174-187",
        "net/minecraft/client/Options.java:909-924",
    ),
    current_scope_reason=(
        "The runtime state setter is invoked by the in-game options callback; BBB intentionally "
        "has no in-game options UI and consumes the same value once at command-line startup."
    ),
    reopen_condition="BBB gains an in-game options surface or another runtime musicToast mutator.",
)

HUD_NOW_PLAYING_RUNTIME_OPTION_EVIDENCE = OwnerEvidence(
    HUD_NOW_PLAYING_TOAST_EVIDENCE.implementation,
    HUD_NOW_PLAYING_TOAST_EVIDENCE.tests
    + (
        "crates/bbb-native/src/startup.rs"
        "#stored_music_options_reach_startup_and_explicit_cli_wins",
    ),
    MUSIC_TOAST_RUNTIME_OPTION_SCOPE,
)

HUD_FILE_DROP_TOAST_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=(
        "net/minecraft/client/MouseHandler.java:227-260",
    ),
    current_scope_reason=(
        "Official 26.1 runs Java 25 and selects LWJGL's multi-release Java-11 UTF-8 "
        "decoder, which replaces malformed input after NUL-terminated GLFW conversion; "
        "the resulting String cannot trip UnixPath's InvalidPathException conditions. "
        "The base Screen drop callback is empty and the only override is the excluded "
        "in-game PackSelectionScreen."
    ),
    reopen_condition=(
        "The Java/LWJGL/GLFW/JDK conversion chain changes, a declared platform can produce "
        "InvalidPathException, or BBB adds PackSelectionScreen/another onFilesDrop consumer."
    ),
)

HUD_FILE_DROP_TOAST_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/main.rs#enqueue_simulation_window_event",
        "docs/evidence/vanilla-26.1/file-drop-capability.json#not-needed-declared-linux-wayland",
    ),
    (
        "scripts/tests/test_file_drop_capability.py#test_checked_receipt_matches_verified_vanilla_and_pinned_winit",
    ),
    HUD_FILE_DROP_TOAST_SCOPE,
)

SYSTEM_TOAST_WORLD_UI_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=(
        "net/minecraft/client/gui/components/toasts/SystemToast.java:132-153",
        "net/minecraft/client/gui/screens/TitleScreen.java:246-262,340-340",
        "net/minecraft/client/gui/screens/worldselection/EditWorldScreen.java:168-168",
        "net/minecraft/client/gui/screens/packs/PackSelectionScreen.java:225-225",
    ),
    current_scope_reason=(
        "World access/delete and pack-copy failure helpers are called only by title/world-selection "
        "or in-game pack-selection surfaces, which are excluded by the external multiplayer and "
        "command-line-only configuration contract."
    ),
    reopen_condition=(
        "BBB adds singleplayer world-selection or an in-game pack-selection/file-copy surface."
    ),
)

SYSTEM_TOAST_WORLD_UI_SCOPE_EVIDENCE = OwnerEvidence(
    ("docs/unsupported-features.md",),
    (TOAST_OWNER_CENSUS_TEST,),
    SYSTEM_TOAST_WORLD_UI_SCOPE,
)

SYSTEM_TOAST_INTEGRATED_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=(
        "net/minecraft/client/gui/components/toasts/SystemToast.java:172-188",
        "net/minecraft/client/server/IntegratedServer.java:352-363",
    ),
    current_scope_reason=(
        "Chunk load/save failure toasts are emitted only by IntegratedServer callbacks; the "
        "declared client connects to external multiplayer servers and owns no server chunk storage."
    ),
    reopen_condition="BBB adds an integrated server or client-owned server chunk storage.",
)

SYSTEM_TOAST_INTEGRATED_SCOPE_EVIDENCE = OwnerEvidence(
    ("docs/unsupported-features.md",),
    (TOAST_OWNER_CENSUS_TEST,),
    SYSTEM_TOAST_INTEGRATED_SCOPE,
)

SERVER_PACK_QUEUE_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-net/src/resource_pack.rs#pub fn initialize_server_pack_download_cache",
        "crates/bbb-net/src/resource_pack.rs#pub fn append_server_pack_download_log",
        "crates/bbb-native/src/resource_pack_runtime.rs#fn start_pack_batch",
    ),
    (
        "crates/bbb-net/src/resource_pack.rs"
        "#server_pack_cache_initialization_vacuums_old_duplicates_and_preserves_log",
        "crates/bbb-net/src/resource_pack.rs"
        "#server_pack_download_log_records_vanilla_file_and_error_shapes",
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#failed_runtime_download_writes_vanilla_download_queue_event",
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#accepted_packs_are_tracked_in_push_order",
    ),
)

SERVER_PACK_HTTP_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-net/src/resource_pack.rs#pub async fn download_server_pack_with_progress",
        "crates/bbb-net/src/resource_pack.rs#fn download_server_pack_blocking",
    ),
    (
        "crates/bbb-net/src/resource_pack.rs"
        "#server_pack_download_verifies_hash_headers_and_reuses_cache",
        "crates/bbb-net/src/resource_pack.rs"
        "#requested_hash_mismatch_preserves_vanilla_target_for_next_validation",
        "crates/bbb-net/src/resource_pack.rs"
        "#requested_hash_target_race_preserves_vanilla_non_truncating_create",
        "crates/bbb-net/src/resource_pack.rs"
        "#server_pack_download_enforces_stream_limit_without_content_length",
        "crates/bbb-net/src/resource_pack.rs"
        "#server_pack_download_removes_bad_requested_cache_before_network_failure",
        "crates/bbb-net/src/resource_pack.rs"
        "#unhashed_download_touches_an_existing_content_addressed_file",
        "crates/bbb-net/src/resource_pack.rs"
        "#matching_requested_cache_bypasses_the_network_download_size_limit",
        "crates/bbb-net/src/resource_pack.rs"
        "#server_pack_download_uses_vanilla_socks_proxy_and_redacts_credentials",
    ),
)

SERVER_PACK_HTTP_PROGRESS_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-net/src/resource_pack.rs#pub async fn download_server_pack_with_progress",
        "crates/bbb-net/src/resource_pack.rs#fn download_server_pack_blocking",
    ),
    (
        "crates/bbb-net/src/resource_pack.rs"
        "#server_pack_download_reports_vanilla_progress_order_for_network_and_cache",
    ),
)

SERVER_PACK_STATE_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-world/src/client/server_presentation.rs#pub fn apply_resource_pack_push",
        "crates/bbb-native/src/resource_pack_runtime.rs#fn reconcile_world",
        "crates/bbb-native/src/resource_pack_runtime.rs#fn resolve_download_batches",
    ),
    (
        "crates/bbb-world/src/client/server_presentation.rs"
        "#resource_pack_push_stores_and_upserts_by_id",
        "crates/bbb-world/src/client/server_presentation.rs"
        "#resource_pack_pop_without_id_clears_all_packs",
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#accepted_packs_are_tracked_in_push_order",
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#one_download_failure_discards_every_other_inactive_pack_across_queued_batches",
    ),
)

SERVER_PACK_RELOAD_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/resource_pack_runtime.rs#pub(crate) fn take_reload",
        "crates/bbb-native/src/resource_pack_runtime.rs#pub(crate) fn finish_reload",
        "crates/bbb-pack/src/server_pack.rs#pub fn prepare_pack_archive",
    ),
    (
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#accepted_pack_runs_download_apply_pop_restore_lifecycle",
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#recovery_reload_runs_once_and_second_failure_does_not_hot_loop",
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#failed_reload_keeps_previous_active_stack_and_fails_only_new_pack",
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#one_preparation_failure_fails_every_new_pack_across_queued_batches",
        "crates/bbb-pack/src/server_pack.rs#prepares_archive_provider_without_eager_extraction",
    ),
)

SERVER_PACK_PROVIDER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-pack/src/server_pack.rs#pub fn prepare_pack_archive",
        "crates/bbb-pack/src/zip_pack.rs#impl ZipPackResources",
        "crates/bbb-pack/src/resources.rs#impl PackEntry",
        "crates/bbb-pack/src/selected_packs.rs#pub fn server_archive",
    ),
    (
        "crates/bbb-pack/src/server_pack.rs"
        "#prepares_archive_provider_without_eager_extraction",
        "crates/bbb-pack/src/zip_pack.rs"
        "#duplicate_lookup_uses_last_entry_while_listing_keeps_central_order",
        "crates/bbb-pack/src/zip_pack.rs"
        "#materialization_is_flat_lazy_and_preserves_useful_extension",
        "crates/bbb-pack/src/selected_packs.rs"
        "#options_zip_is_indexed_directly_and_materializes_only_consumed_entries",
        "crates/bbb-pack/src/selected_packs.rs"
        "#server_packs_follow_vanilla_reverse_push_repository_order",
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#failed_reload_keeps_previous_active_stack_and_fails_only_new_pack",
    ),
)

SERVER_PACK_SHUTDOWN_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/resource_pack_runtime.rs#impl Drop for ServerResourcePackRuntime",
        "crates/bbb-native/src/resource_pack_runtime.rs#fn cleanup_removed_packs",
    ),
    (
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#dropping_runtime_removes_materialized_session_directory",
        "crates/bbb-native/src/resource_pack_runtime.rs"
        "#transfer_keeps_active_pack_while_terminal_disconnect_unloads_it",
    ),
)

SERVER_PACK_LOCAL_SCOPE_EVIDENCE = OwnerEvidence(
    ("docs/unsupported-features.md",),
    (SERVER_PACK_SCOPE_TEST,),
    SERVER_PACK_LOCAL_WORLD_SCOPE,
)

HTTP_UTIL_CONSTRUCTOR_EVIDENCE = OwnerEvidence(
    ("docs/unsupported-features.md",),
    (SERVER_PACK_SCOPE_TEST,),
    HTTP_UTIL_CONSTRUCTOR_SCOPE,
)

DOWNLOADED_PACK_SOURCE_OWNER_EVIDENCE = _owner_evidence(
    (
        SERVER_PACK_QUEUE_EVIDENCE,
        ("DownloadedPackSource",),
    ),
    (
        SERVER_PACK_TOAST_EVIDENCE,
        ("createDownloadNotifier",),
    ),
    (
        SERVER_PACK_QUEUE_EVIDENCE,
        ("createUpdateScheduler",),
    ),
    (
        SERVER_PACK_HTTP_EVIDENCE,
        ("createDownloader", "tryParseSha1Hash"),
    ),
    (
        SERVER_PACK_PROVIDER_EVIDENCE,
        (
            "loadRequestedPacks",
            "createRepositorySource",
            "configureSource",
        ),
    ),
    (
        SERVER_PACK_RELOAD_EVIDENCE,
        (
            "createReloadConfig",
            "startReload",
            "onRecovery",
            "onRecoveryFailure",
            "onReloadSuccess",
        ),
    ),
    (
        SERVER_PACK_STATE_EVIDENCE,
        (
            "pushPack",
            "popPack",
            "popAll",
            "createPackResponseSender",
            "configureForServerControl",
            "allowServerPacks",
            "rejectServerPacks",
        ),
    ),
    (
        SERVER_PACK_LOCAL_SCOPE_EVIDENCE,
        ("pushLocalPack", "configureForLocalWorld", "waitForPackFeedback"),
    ),
    (
        SERVER_PACK_SHUTDOWN_EVIDENCE,
        ("cleanupAfterDisconnect", "close"),
    ),
)

SERVER_PACK_MANAGER_OWNER_EVIDENCE = _owner_evidence(
    (
        SERVER_PACK_QUEUE_EVIDENCE,
        (
            "ServerPackManager",
            "registerForUpdate",
            "tick",
            "onDownload",
            "updateDownloads",
        ),
    ),
    (
        SERVER_PACK_STATE_EVIDENCE,
        (
            "markExistingPacksAsRemoved",
            "pushPack",
            "pushNewPack",
            "acceptPack",
            "findPackInfo",
            "popPack",
            "popAll",
            "allowServerPacks",
            "rejectServerPacks",
            "resetPromptStatus",
            "cleanupRemovedPacks",
            "RemovalReason",
            "ServerPackData",
            "setRemovalReasonIfNotSet",
            "isRemoved",
        ),
    ),
    (SERVER_PACK_RELOAD_EVIDENCE, ("triggerReloadIfNeeded",)),
    (SERVER_PACK_LOCAL_SCOPE_EVIDENCE, ("pushLocalPack",)),
)

DOWNLOAD_QUEUE_OWNER_EVIDENCE = _owner_evidence(
    (SERVER_PACK_QUEUE_EVIDENCE, ("DownloadQueue", "runDownload", "getFileInfo", "downloadBatch", "BatchResult")),
    (SERVER_PACK_SHUTDOWN_EVIDENCE, ("close",)),
)

DOWNLOAD_CACHE_CLEANER_OWNER_EVIDENCE = _owner_evidence(
    (
        SERVER_PACK_QUEUE_EVIDENCE,
        ("vacuumCacheDir", "listFilesWithModificationTimes", "prioritizeFilesInDirs"),
    ),
)

HTTP_UTIL_OWNER_EVIDENCE = _owner_evidence(
    (HTTP_UTIL_CONSTRUCTOR_EVIDENCE, ("HttpUtil",)),
    (
        SERVER_PACK_HTTP_EVIDENCE,
        (
            "downloadFile",
            "updateModificationTime",
            "hashFile",
            "checkExistingFile",
            "cachedFilePath",
            "downloadAndHash",
        ),
    ),
    (
        SERVER_PACK_HTTP_PROGRESS_EVIDENCE,
        ("requestStart", "downloadStart", "downloadedBytes", "requestFinished"),
    ),
)

TOAST_OWNER_EVIDENCE = _owner_evidence(
    (
        HUD_TOAST_SHARED_EVIDENCE,
        (
            "getWantedVisibility",
            "update",
            "getSoundEvent",
            "extractRenderState",
            "getToken",
            "xPos",
            "yPos",
            "width",
            "height",
            "occcupiedSlotCount",
            "onFinishedRendering",
            "Visibility",
            "playSound",
        ),
    ),
)

TOAST_MANAGER_OWNER_EVIDENCE = _owner_evidence(
    (
        HUD_TOAST_MANAGER_WITH_NOW_PLAYING_EVIDENCE,
        (
            "ToastManager",
            "update",
            "extractRenderState",
            "showNowPlayingToast",
            "hideNowPlayingToast",
            "initializeMusicToast",
        ),
    ),
    (HUD_NOW_PLAYING_RUNTIME_OPTION_EVIDENCE, ("setMusicToastDisplayState",)),
    (
        HUD_TOAST_SHARED_EVIDENCE,
        (
            "findFreeSlotsIndex",
            "freeSlotCount",
            "getToast",
            "clear",
            "addToast",
            "getMinecraft",
            "getNotificationDisplayTimeMultiplier",
            "ToastInstance",
            "resetToast",
            "hasFinishedRendering",
            "calculateVisiblePortion",
        ),
    ),
)

SYSTEM_TOAST_OWNER_EVIDENCE = _owner_evidence(
    (
        HUD_DYNAMIC_SYSTEM_TITLE_EVIDENCE,
        ("SystemToast", "extractRenderState", "reset"),
    ),
    (HUD_FILE_DROP_TOAST_EVIDENCE, ("onFileDropFailure",)),
    (
        SYSTEM_TOAST_WORLD_UI_SCOPE_EVIDENCE,
        (
            "onWorldAccessFailure",
            "onWorldDeleteFailure",
            "onPackCopyFailure",
        ),
    ),
    (
        SYSTEM_TOAST_INTEGRATED_SCOPE_EVIDENCE,
        ("onChunkLoadFailure", "onChunkSaveFailure"),
    ),
    (
        HUD_SYSTEM_TOAST_EVIDENCE,
        (
            "multiline",
            "nullToEmpty",
            "width",
            "height",
            "forceHide",
            "getWantedVisibility",
            "update",
            "getToken",
            "add",
            "addOrUpdate",
            "onLowDiskSpace",
            "SystemToastId",
        ),
    ),
)

ADVANCEMENT_TOAST_OWNER_EVIDENCE = _owner_evidence(
    (
        HUD_ADVANCEMENT_TOAST_EVIDENCE,
        (
            "AdvancementToast",
            "getWantedVisibility",
            "update",
            "getSoundEvent",
            "isChallengeAdvancement",
            "extractRenderState",
        ),
    ),
)

RECIPE_TOAST_OWNER_EVIDENCE = _owner_evidence(
    (
        HUD_RECIPE_TOAST_EVIDENCE,
        (
            "RecipeToast",
            "getWantedVisibility",
            "update",
            "extractRenderState",
            "addItem",
            "addOrUpdate",
        ),
    ),
)

TUTORIAL_TOAST_OWNER_EVIDENCE = _owner_evidence(
    (HUD_TUTORIAL_CONSTRUCTOR_EVIDENCE, ("TutorialToast",)),
    (
        HUD_TUTORIAL_TOAST_EVIDENCE,
        (
            "getWantedVisibility",
            "update",
            "height",
            "contentHeight",
            "extractRenderState",
            "hide",
            "updateProgress",
            "Icons",
        ),
    ),
)

NOW_PLAYING_TOAST_OWNER_EVIDENCE = _owner_evidence(
    (
        HUD_NOW_PLAYING_TOAST_EVIDENCE,
        (
            "NowPlayingToast",
            "extractToast",
            "getCurrentSongName",
            "tickMusicNotes",
            "getNowPlayingString",
            "showToast",
            "update",
            "extractRenderState",
            "onFinishedRendering",
            "width",
            "getWidth",
            "height",
            "xPos",
            "yPos",
            "getWantedVisibility",
            "setWantedVisibility",
        ),
    ),
)

PERIODIC_NOTIFICATION_OWNER_EVIDENCE = _owner_evidence(
    (
        HUD_PERIODIC_TOAST_EVIDENCE,
        (
            "PeriodicNotificationManager",
            "prepare",
            "apply",
            "close",
            "stopTimer",
            "calculateOptimalPeriod",
            "calculateInitialDelay",
            "NotificationTask",
            "reset",
            "run",
        ),
    ),
)

SERVER_PACK_OWNER_EVIDENCE_BY_SOURCE = {
    "net/minecraft/client/resources/server/DownloadedPackSource.java": DOWNLOADED_PACK_SOURCE_OWNER_EVIDENCE,
    "net/minecraft/client/resources/server/ServerPackManager.java": SERVER_PACK_MANAGER_OWNER_EVIDENCE,
    "net/minecraft/server/packs/DownloadQueue.java": DOWNLOAD_QUEUE_OWNER_EVIDENCE,
    "net/minecraft/server/packs/DownloadCacheCleaner.java": DOWNLOAD_CACHE_CLEANER_OWNER_EVIDENCE,
    "net/minecraft/util/HttpUtil.java": HTTP_UTIL_OWNER_EVIDENCE,
}

HUD_TOAST_OWNER_EVIDENCE_BY_SOURCE = {
    "net/minecraft/client/gui/components/toasts/Toast.java": TOAST_OWNER_EVIDENCE,
    "net/minecraft/client/gui/components/toasts/ToastManager.java": TOAST_MANAGER_OWNER_EVIDENCE,
    "net/minecraft/client/gui/components/toasts/SystemToast.java": SYSTEM_TOAST_OWNER_EVIDENCE,
    "net/minecraft/client/gui/components/toasts/AdvancementToast.java": ADVANCEMENT_TOAST_OWNER_EVIDENCE,
    "net/minecraft/client/gui/components/toasts/RecipeToast.java": RECIPE_TOAST_OWNER_EVIDENCE,
    "net/minecraft/client/gui/components/toasts/TutorialToast.java": TUTORIAL_TOAST_OWNER_EVIDENCE,
    "net/minecraft/client/gui/components/toasts/NowPlayingToast.java": NOW_PLAYING_TOAST_OWNER_EVIDENCE,
    "net/minecraft/client/PeriodicNotificationManager.java": PERIODIC_NOTIFICATION_OWNER_EVIDENCE,
}

SERVER_PACK_ROOT_OWNER_EVIDENCE = _owner_evidence(
    (SERVER_PACK_SHUTDOWN_EVIDENCE, ("clearDownloadedResourcePacks",)),
)

LANGUAGE_MANAGER_SELECTION_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/simulation/input_commands.rs#apply_recipe_book_language_transition",
        "crates/bbb-native/src/main.rs#save_client_options_and_enqueue_broadcast",
    ),
    (
        "crates/bbb-native/src/input/inventory/tests.rs#recipe_book_excitedze_search_requests_the_en_pt_language_decision",
        "crates/bbb-native/src/simulation/input_commands.rs#recipe_book_language_transition_reloads_saves_and_broadcasts_in_order",
        "crates/bbb-native/src/simulation/input_commands.rs#mouse_button_receipts_distinguish_delivery_inventory_routing_and_consumption",
        "crates/bbb-native/src/input/tests.rs#closing_recipe_book_screen_revokes_stale_search_text_input_focus",
        "crates/bbb-native/src/client_options.rs#recipe_book_language_write_stores_the_bare_language_code",
        "crates/bbb-native/src/main.rs#client_options_save_completion_follows_missing_or_failed_file_owner",
    ),
)

LANGUAGE_MANAGER_METADATA_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-pack/src/metadata.rs#PackMetadataCatalog",
        "crates/bbb-pack/src/language.rs#pub fn has_available_language(",
    ),
    (
        "crates/bbb-pack/src/metadata.rs#pack_metadata_catalog_loads_default_language_and_pack_languages",
        "crates/bbb-pack/src/metadata.rs#pack_metadata_catalog_skips_invalid_language_metadata_pack",
        "crates/bbb-pack/src/language.rs#pack_roots_client_language_catalog_uses_pack_metadata_for_selected_language",
    ),
)

LANGUAGE_MANAGER_RELOAD_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-pack/src/roots.rs#pub fn load_client_language_catalog(",
        "crates/bbb-native/src/resource_reload.rs#apply_server_resource_pack_reload_effect",
    ),
    (
        "crates/bbb-pack/src/language.rs#pack_roots_client_language_catalog_uses_pack_metadata_for_selected_language",
        "crates/bbb-pack/src/language.rs#indexed_selected_locale_honors_pack_override_and_filter",
        "crates/bbb-native/src/simulation/input_commands.rs#recipe_book_language_transition_reloads_saves_and_broadcasts_in_order",
    ),
)

LANGUAGE_MANAGER_OWNER_EVIDENCE = _owner_evidence(
    (
        LANGUAGE_MANAGER_SELECTION_EVIDENCE,
        ("LanguageManager", "setSelected", "getSelected"),
    ),
    (
        LANGUAGE_MANAGER_METADATA_EVIDENCE,
        ("extractLanguages", "getLanguages", "getLanguage"),
    ),
    (LANGUAGE_MANAGER_RELOAD_EVIDENCE, ("onResourceManagerReload",)),
)

CLIENT_LANGUAGE_LOAD_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-pack/src/language.rs#pub fn load_resource_stack(",
        "crates/bbb-pack/src/language.rs#fn append_resource_stack(",
        "crates/bbb-pack/src/roots.rs#pub fn load_client_language_catalog(",
    ),
    (
        "crates/bbb-pack/src/language.rs#language_catalog_resource_stack_merges_namespaces_and_pack_precedence",
        "crates/bbb-pack/src/language.rs#language_catalog_skips_bad_pack_resource_and_keeps_other_namespaces",
        "crates/bbb-pack/src/language.rs#malformed_language_json_stops_only_its_namespace_resource_stack",
        "crates/bbb-pack/src/language.rs#pack_roots_language_catalog_applies_default_deprecated_translations",
    ),
)

CLIENT_LANGUAGE_LOOKUP_EVIDENCE = OwnerEvidence(
    ("crates/bbb-pack/src/language.rs#LanguageCatalog",),
    (
        "crates/bbb-pack/src/language.rs#language_catalog_parses_json_and_normalizes_numeric_formats",
    ),
)

CLIENT_LANGUAGE_DIRECTION_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-pack/src/language.rs#pub fn is_bidirectional(",
        "crates/bbb-item-model/src/item_runtime.rs#pub fn language_bidirectional(",
    ),
    (
        "crates/bbb-pack/src/language.rs#pack_roots_client_language_catalog_uses_pack_metadata_for_selected_language",
    ),
)

CLIENT_LANGUAGE_BIDI_EVIDENCE = OwnerEvidence(
    ("crates/bbb-render-types/src/text_visual_order.rs#visual_order",),
    (
        "crates/bbb-render-types/src/text_visual_order.rs#arabic_shaping_matches_icu4j_77_1_fixed_vectors",
        "crates/bbb-render-types/src/text_visual_order.rs#visual_order_matches_icu_mixed_direction_and_mirroring_vectors",
        "crates/bbb-render-types/src/text_visual_order.rs#visual_order_carries_utf16_metadata_like_vanilla_substring_source",
    ),
)

CLIENT_LANGUAGE_OWNER_EVIDENCE = _owner_evidence(
    (
        CLIENT_LANGUAGE_LOAD_EVIDENCE,
        ("ClientLanguage", "loadFrom", "appendFrom"),
    ),
    (CLIENT_LANGUAGE_LOOKUP_EVIDENCE, ("getOrDefault", "has")),
    (CLIENT_LANGUAGE_DIRECTION_EVIDENCE, ("isDefaultRightToLeft",)),
    (CLIENT_LANGUAGE_BIDI_EVIDENCE, ("getVisualOrder",)),
)

RECIPE_BOOK_LANGUAGE_OWNER_EVIDENCE = _owner_evidence(
    (
        LANGUAGE_MANAGER_SELECTION_EVIDENCE,
        ("checkSearchStringUpdate", "pirateSpeechForThePeople"),
    ),
)

CHUNK_LAYER_BUILD_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-renderer/src/terrain.rs#build_terrain_chunk_section_subset_mesh_layers_with_atlas_and_camera",
        "crates/bbb-renderer/src/terrain/mesh.rs#build_chunk_section_subset_mesh_layers_with_lookup",
    ),
    (
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#layer_builder_splits_opaque_and_cutout_meshes",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#forced_translucent_cube_faces_emit_in_translucent_layer",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#transparent_cube_faces_emit_in_cutout_layer",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#section_layer_builder_reuses_texture_layer_routing_without_changing_meshes",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#waterlogged_block_emits_block_and_fluid_layers",
        "crates/bbb-native/src/terrain_runtime/textures/tests.rs#fast_leaves_force_root_and_nested_model_transparency_opaque",
    ),
)

CHUNK_LAYER_SORT_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-renderer/src/terrain/mesh.rs#sort_translucent_quads_by_distance",
        "crates/bbb-renderer/src/renderer.rs#TerrainTranslucentSortState",
    ),
    (
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#translucent_layer_sorts_quad_indices_by_distance_from_camera_like_vanilla",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#translucent_layer_sort_uses_world_camera_equivalent_to_vanilla_section_local_sorting",
        "crates/bbb-renderer/src/renderer.rs#translucent_sort_state_rebuilds_vanilla_indices_after_camera_move",
        "crates/bbb-renderer/src/renderer.rs#translucent_sort_state_uses_vanilla_block_and_point_of_view_triggers",
    ),
)

SECTION_COMPILER_LAYER_OWNER_EVIDENCE = _owner_evidence(
    (CHUNK_LAYER_BUILD_OWNER_EVIDENCE, ("compile", "getOrBeginLayer")),
)

MESH_DATA_SORT_OWNER_EVIDENCE = _owner_evidence(
    (CHUNK_LAYER_SORT_OWNER_EVIDENCE, ("sortQuads", "buildSortedIndexBuffer")),
)

VERTEX_SORTING_OWNER_EVIDENCE = _owner_evidence(
    (CHUNK_LAYER_SORT_OWNER_EVIDENCE, ("byDistance", "sort", "apply")),
)

SECTION_RENDER_SORT_OWNER_EVIDENCE = _owner_evidence(
    (CHUNK_LAYER_SORT_OWNER_EVIDENCE, ("createVertexSorting",)),
)

RENDERING_CHUNK_LAYER_OWNER_EVIDENCE = {
    **SECTION_COMPILER_LAYER_OWNER_EVIDENCE,
    **MESH_DATA_SORT_OWNER_EVIDENCE,
    **VERTEX_SORTING_OWNER_EVIDENCE,
    **SECTION_RENDER_SORT_OWNER_EVIDENCE,
}

CAMERA_FLUID_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/camera_fov.rs#camera_fluid_in_camera",
        "crates/bbb-native/src/runtime.rs#camera_environment_colors_from_attributes",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs#camera_fluid_sampler_uses_water_priority_viewport_base_fov_and_inclusive_lava_surface",
        "crates/bbb-native/src/runtime/tests.rs#camera_powder_snow_near_plane_matches_vanilla_sample_order_without_fov_scaling",
        "crates/bbb-native/src/runtime/tests.rs#camera_fluid_sample_is_shared_by_same_frame_fog_and_fov_consumers",
    ),
)

CAMERA_FOV_OWNER_EVIDENCE = OwnerEvidence(
    ("crates/bbb-native/src/camera_fov.rs#CameraFovTickState",),
    (
        "crates/bbb-native/src/runtime/tests.rs#camera_fov_frame_samples_water_at_eye_and_lava_on_base_fov_near_plane",
        "crates/bbb-native/src/runtime/tests.rs#camera_powder_snow_near_plane_matches_vanilla_sample_order_without_fov_scaling",
    ),
)

FOG_COLOR_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime/render_extract.rs#clear_color_for_world_with_environment_colors_and_water_vision",
        "crates/bbb-native/src/runtime/render_extract.rs#apply_camera_mob_effect_fog_color",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs#clear_color_samples_camera_biome_fog_and_sky_color_attributes",
        "crates/bbb-native/src/runtime/tests.rs#blindness_overrides_atmospheric_fog_and_darkens_its_color",
        "crates/bbb-native/src/runtime/tests.rs#night_vision_brightens_atmospheric_fog_unless_darkness_is_present",
    ),
)

FOG_DISTANCE_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime/render_extract.rs#fog_environment_for_world_with_environment_colors",
        "crates/bbb-native/src/runtime/render_extract.rs#camera_mob_effect_fog_environment",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs#fog_environment_uses_vanilla_render_distance_range_and_dimension_fog_distances",
        "crates/bbb-native/src/runtime/tests.rs#fog_environment_applies_vanilla_rain_fog_distance_multiplier",
        "crates/bbb-native/src/runtime/tests.rs#blindness_takes_fog_precedence_over_darkness_and_blocks_sky",
        "crates/bbb-native/src/runtime/tests.rs#darkness_fog_uses_canonical_blend_factor",
    ),
)

FOG_SELECTION_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime.rs#camera_environment_colors_from_cached_sample",
        "crates/bbb-native/src/runtime/render_extract.rs#fog_environment_for_world_with_environment_colors",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs#camera_fluid_sample_is_shared_by_same_frame_fog_and_fov_consumers",
        "crates/bbb-native/src/runtime/tests.rs#lava_fog_precedes_mob_effect_distances_and_suppresses_the_sky_pass",
        "crates/bbb-native/src/runtime/tests.rs#powder_snow_fog_precedes_mob_effect_distances_preserves_color_and_suppresses_sky",
        "crates/bbb-native/src/runtime/tests.rs#blindness_takes_fog_precedence_over_darkness_and_blocks_sky",
    ),
)

ATMOSPHERIC_FOG_COLOR_OWNER_EVIDENCE = OwnerEvidence(
    FOG_COLOR_OWNER_EVIDENCE.implementation,
    (
        "crates/bbb-native/src/runtime/tests.rs#clear_color_samples_camera_biome_fog_and_sky_color_attributes",
        "crates/bbb-native/src/runtime/tests.rs#clear_color_mixes_sunrise_sunset_color_when_camera_faces_sun",
    ),
)

ATMOSPHERIC_FOG_DISTANCE_OWNER_EVIDENCE = OwnerEvidence(
    ("crates/bbb-native/src/runtime/render_extract.rs#fog_environment_for_world_with_environment_colors",),
    (
        "crates/bbb-native/src/runtime/tests.rs#fog_environment_uses_vanilla_render_distance_range_and_dimension_fog_distances",
        "crates/bbb-native/src/runtime/tests.rs#fog_environment_applies_vanilla_rain_fog_distance_multiplier",
        "crates/bbb-native/src/runtime/tests.rs#fog_environment_clamps_atmospheric_distance_for_boss_world_fog",
    ),
)

WATER_FOG_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime/render_extract.rs#clear_color_for_world_with_environment_colors_and_water_vision",
        "crates/bbb-native/src/runtime/render_extract.rs#fog_environment_for_world_with_environment_colors",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs#clear_color_brightens_water_fog_with_vanilla_water_vision",
        "crates/bbb-native/src/runtime/tests.rs#fog_environment_uses_water_fog_distances_when_eye_is_in_water",
        "crates/bbb-native/src/runtime/tests.rs#fog_environment_applies_biome_water_fog_end_distance_modifier",
    ),
)

LAVA_FOG_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime/render_extract.rs#clear_color_for_world_with_environment_colors_and_water_vision",
        "crates/bbb-native/src/runtime/render_extract.rs#fog_environment_for_world_with_environment_colors",
        "crates/bbb-native/src/runtime/render_extract.rs#sky_environment_for_world_with_environment_colors",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs#lava_fog_uses_normal_fire_resistance_and_spectator_distance_precedence",
        "crates/bbb-native/src/runtime/tests.rs#lava_fog_precedes_mob_effect_distances_and_suppresses_the_sky_pass",
    ),
)

POWDER_SNOW_FOG_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime/render_extract.rs#clear_color_for_world_with_environment_colors_and_water_vision",
        "crates/bbb-native/src/runtime/render_extract.rs#fog_environment_for_world_with_environment_colors",
        "crates/bbb-native/src/runtime/render_extract.rs#sky_environment_for_world_with_environment_colors",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs#camera_powder_snow_near_plane_matches_vanilla_sample_order_without_fov_scaling",
        "crates/bbb-native/src/runtime/tests.rs#powder_snow_fog_precedes_mob_effect_distances_preserves_color_and_suppresses_sky",
    ),
)

BLINDNESS_FOG_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime/render_extract.rs#apply_camera_mob_effect_fog_color",
        "crates/bbb-native/src/runtime/render_extract.rs#camera_mob_effect_fog_environment",
        "crates/bbb-native/src/runtime/render_extract.rs#sky_environment_for_world_with_environment_colors",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs#blindness_overrides_atmospheric_fog_and_darkens_its_color",
        "crates/bbb-native/src/runtime/tests.rs#blindness_takes_fog_precedence_over_darkness_and_blocks_sky",
    ),
)

DARKNESS_FOG_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime/render_extract.rs#apply_camera_mob_effect_fog_color",
        "crates/bbb-native/src/runtime/render_extract.rs#camera_mob_effect_fog_environment",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs#darkness_fog_uses_canonical_blend_factor",
        "crates/bbb-native/src/runtime/tests.rs#night_vision_brightens_atmospheric_fog_unless_darkness_is_present",
    ),
)

CAMERA_OWNER_EVIDENCE = _owner_evidence(
    (CAMERA_FLUID_OWNER_EVIDENCE, ("getFluidInCamera",)),
    (CAMERA_FOV_OWNER_EVIDENCE, ("modifyFovBasedOnDeathOrFluid",)),
)

FOG_RENDERER_OWNER_EVIDENCE = _owner_evidence(
    (FOG_COLOR_OWNER_EVIDENCE, ("computeFogColor",)),
    (FOG_DISTANCE_OWNER_EVIDENCE, ("setupFog",)),
    (FOG_SELECTION_OWNER_EVIDENCE, ("getFogType",)),
)

FOG_ENVIRONMENT_OWNER_EVIDENCE = _owner_evidence(
    (FOG_DISTANCE_OWNER_EVIDENCE, ("setupFog",)),
    (
        FOG_COLOR_OWNER_EVIDENCE,
        ("providesColor", "getBaseColor", "modifiesDarkness", "getModifiedDarkness"),
    ),
    (FOG_SELECTION_OWNER_EVIDENCE, ("isApplicable",)),
)

MOB_EFFECT_FOG_ENVIRONMENT_OWNER_EVIDENCE = _owner_evidence(
    (FOG_DISTANCE_OWNER_EVIDENCE, ("getMobEffect", "isApplicable")),
    (FOG_COLOR_OWNER_EVIDENCE, ("providesColor", "modifiesDarkness")),
)

ATMOSPHERIC_FOG_ENVIRONMENT_OWNER_EVIDENCE = _owner_evidence(
    (ATMOSPHERIC_FOG_COLOR_OWNER_EVIDENCE, ("getBaseColor", "applyWeatherDarken")),
    (ATMOSPHERIC_FOG_DISTANCE_OWNER_EVIDENCE, ("setupFog", "updateRainFogState")),
    (FOG_SELECTION_OWNER_EVIDENCE, ("isApplicable",)),
)

WATER_FOG_ENVIRONMENT_OWNER_EVIDENCE = _owner_evidence(
    (WATER_FOG_OWNER_EVIDENCE, ("setupFog", "isApplicable", "getBaseColor")),
)

LAVA_FOG_ENVIRONMENT_OWNER_EVIDENCE = _owner_evidence(
    (LAVA_FOG_OWNER_EVIDENCE, ("getBaseColor", "setupFog", "isApplicable")),
)

POWDER_SNOW_FOG_ENVIRONMENT_OWNER_EVIDENCE = _owner_evidence(
    (POWDER_SNOW_FOG_OWNER_EVIDENCE, ("getBaseColor", "setupFog", "isApplicable")),
)

BLINDNESS_FOG_ENVIRONMENT_OWNER_EVIDENCE = _owner_evidence(
    (BLINDNESS_FOG_OWNER_EVIDENCE, ("getMobEffect", "setupFog", "getModifiedDarkness")),
)

DARKNESS_FOG_ENVIRONMENT_OWNER_EVIDENCE = _owner_evidence(
    (DARKNESS_FOG_OWNER_EVIDENCE, ("getMobEffect", "setupFog", "getModifiedDarkness")),
)

CAMERA_FOG_OWNER_EVIDENCE_BY_SOURCE = {
    "net/minecraft/client/Camera.java": CAMERA_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/fog/FogRenderer.java": FOG_RENDERER_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/fog/environment/FogEnvironment.java": FOG_ENVIRONMENT_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/fog/environment/MobEffectFogEnvironment.java": MOB_EFFECT_FOG_ENVIRONMENT_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/fog/environment/AtmosphericFogEnvironment.java": ATMOSPHERIC_FOG_ENVIRONMENT_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/fog/environment/WaterFogEnvironment.java": WATER_FOG_ENVIRONMENT_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/fog/environment/LavaFogEnvironment.java": LAVA_FOG_ENVIRONMENT_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/fog/environment/PowderedSnowFogEnvironment.java": POWDER_SNOW_FOG_ENVIRONMENT_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/fog/environment/BlindnessFogEnvironment.java": BLINDNESS_FOG_ENVIRONMENT_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/fog/environment/DarknessFogEnvironment.java": DARKNESS_FOG_ENVIRONMENT_OWNER_EVIDENCE,
}

AUDIO_OWNER_EVIDENCE_BY_SOURCE = {
    "com/mojang/blaze3d/audio/Library.java": AUDIO_LIBRARY_OWNER_EVIDENCE,
    "com/mojang/blaze3d/audio/Channel.java": AUDIO_CHANNEL_OWNER_EVIDENCE,
    "com/mojang/blaze3d/audio/Listener.java": AUDIO_LISTENER_OWNER_EVIDENCE,
    "com/mojang/blaze3d/audio/SoundBuffer.java": AUDIO_SOUND_BUFFER_OWNER_EVIDENCE,
    "net/minecraft/client/sounds/ChannelAccess.java": AUDIO_CHANNEL_ACCESS_OWNER_EVIDENCE,
    "net/minecraft/client/sounds/SoundBufferLibrary.java": AUDIO_SOUND_BUFFER_LIBRARY_OWNER_EVIDENCE,
    "net/minecraft/client/sounds/MusicManager.java": MUSIC_MANAGER_OWNER_EVIDENCE,
}

OFFLINE_ASSET_TRUST_OWNER_CENSUS_TEST = (
    "scripts/tests/test_client_mechanism_inventory.py"
    "#test_offline_asset_trust_owner_census_is_exact_and_has_per_owner_evidence"
)

INDEXED_VANILLA_PROVIDER_OWNER_CENSUS_TEST = (
    "scripts/tests/test_client_mechanism_inventory.py"
    "#test_indexed_vanilla_provider_owner_census_is_exact_and_has_per_owner_evidence"
)

STARTUP_BOOTSTRAP_OWNER_CENSUS_TEST = (
    "scripts/tests/test_client_mechanism_inventory.py"
    "#test_startup_bootstrap_owner_census_is_exact_and_has_per_owner_evidence"
)

STARTUP_IDENTITY_OWNER_CENSUS_TEST = (
    "scripts/tests/test_client_mechanism_inventory.py"
    "#test_startup_identity_owner_census_is_exact_and_has_per_owner_evidence"
)

PANORAMA_OWNER_CENSUS_TEST = (
    "scripts/tests/test_client_mechanism_inventory.py"
    "#test_panorama_owner_census_is_exact_and_has_per_owner_evidence"
)

PANORAMA_RESOURCE_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-pack/src/panorama.rs#pub struct PanoramaCatalog",
        "crates/bbb-native/src/hud_assets.rs#pub(crate) fn upload_panorama",
        "crates/bbb-renderer/src/panorama.rs#pub fn upload_hud_panorama",
    ),
    (
        "crates/bbb-pack/src/panorama.rs#loads_vanilla_layer_order_and_selected_override",
        "crates/bbb-pack/src/panorama.rs#filter_missing_corrupt_and_mismatched_faces_reject_the_whole_catalog",
        "crates/bbb-pack/src/panorama.rs#indexed_objects_are_lazy_but_each_consumed_object_is_integrity_checked",
        "crates/bbb-renderer/src/offscreen.rs#offscreen_panorama_preserves_vanilla_face_row_orientation",
        PANORAMA_OWNER_CENSUS_TEST,
    ),
)

PANORAMA_RENDER_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-renderer/src/panorama.rs#struct PanoramaGpu",
        "crates/bbb-renderer/src/render.rs#fn panorama_pass",
        "crates/bbb-renderer/src/hud/disconnected_screen.rs#pub(super) fn push_panorama_overlay",
    ),
    (
        "crates/bbb-renderer/src/panorama.rs#uniform_preserves_vanilla_fov_aspect_and_rotation_order",
        "crates/bbb-renderer/src/panorama.rs#shader_uses_one_cube_texture_and_fullscreen_perspective_ray",
        "crates/bbb-renderer/src/offscreen.rs#offscreen_panorama_preserves_vanilla_face_row_orientation",
        "crates/bbb-renderer/src/offscreen.rs#offscreen_disconnected_panorama_draws_front_face_before_overlay_blur_and_rolls_back",
        PANORAMA_OWNER_CENSUS_TEST,
    ),
)

PANORAMA_SCREEN_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/runtime.rs#fn panorama_rotation_degrees",
        "crates/bbb-native/src/runtime/render_extract.rs#pub(crate) fn hud_disconnected_screen",
        "crates/bbb-renderer/src/render.rs#fn panorama_pass",
    ),
    (
        "crates/bbb-native/src/runtime/tests.rs#panorama_rotation_scales_with_the_scroll_speed_option",
        "crates/bbb-native/src/runtime/tests.rs#panorama_spin_uses_realtime_ticks_only_while_visible_and_vanilla_wrap",
        "crates/bbb-renderer/src/offscreen.rs#offscreen_disconnected_panorama_draws_front_face_before_overlay_blur_and_rolls_back",
        PANORAMA_OWNER_CENSUS_TEST,
    ),
)

PANORAMA_RELOAD_OWNER_EVIDENCE = OwnerEvidence(
    (
        "crates/bbb-native/src/resource_reload.rs#fn apply_native_resource_reload",
        "crates/bbb-renderer/src/resource_reload.rs#pub(crate) fn transactional_resource_reload",
    ),
    (
        "crates/bbb-native/src/resource_reload.rs#renderer_resource_owners_commit_inside_one_reload_barrier",
        "crates/bbb-renderer/src/offscreen.rs#offscreen_disconnected_panorama_draws_front_face_before_overlay_blur_and_rolls_back",
        PANORAMA_OWNER_CENSUS_TEST,
    ),
)

PANORAMA_GUI_RENDERER_OWNER_EVIDENCE = _owner_evidence(
    (PANORAMA_RESOURCE_OWNER_EVIDENCE, ("GuiRenderer", "registerPanoramaTextures")),
    (PANORAMA_RENDER_OWNER_EVIDENCE, ("render",)),
)

PANORAMA_CUBE_MAP_OWNER_EVIDENCE = _owner_evidence(
    (PANORAMA_RENDER_OWNER_EVIDENCE, ("CubeMap", "render", "initializeVertices")),
    (PANORAMA_RESOURCE_OWNER_EVIDENCE, ("registerTextures",)),
    (PANORAMA_RELOAD_OWNER_EVIDENCE, ("close",)),
)

PANORAMA_CUBE_TEXTURE_OWNER_EVIDENCE = _owner_evidence(
    (
        PANORAMA_RESOURCE_OWNER_EVIDENCE,
        ("CubeMapTexture", "loadContents", "doLoad"),
    ),
)

PANORAMA_STATE_OWNER_EVIDENCE = _owner_evidence(
    (PANORAMA_SCREEN_OWNER_EVIDENCE, ("extractRenderState",)),
)

STARTUP_ARGUMENT_IMPLEMENTATION = (
    "crates/bbb-native/src/startup.rs#pub(crate) fn parse_args",
    "crates/bbb-native/src/startup.rs#fn apply_vanilla_bootstrap_arguments",
    "crates/bbb-native/src/startup.rs#fn apply_stored_window_override",
    "crates/bbb-native/src/startup.rs#fn unescape_java_argument",
    "crates/bbb-native/src/startup.rs#pub(crate) fn name_current_render_thread",
    "crates/bbb-native/src/startup.rs#impl WindowConfig",
)

STARTUP_ARGUMENT_TESTS = (
    "crates/bbb-native/src/startup.rs"
    "#vanilla_bootstrap_arguments_feed_the_single_external_startup_path",
    "crates/bbb-native/src/startup.rs"
    "#vanilla_quick_play_unescapes_java_and_cli_fullscreen_dimensions_are_fallback_only",
    "crates/bbb-native/src/startup.rs"
    "#window_config_clamps_dimensions_to_minimum_nonzero",
    "crates/bbb-native/src/startup.rs"
    "#stored_positive_window_override_beats_main_display_arguments_only_as_a_pair",
    "crates/bbb-native/src/startup.rs#render_thread_name_is_visible_to_linux_profilers",
    "scripts/tests/test_official_client_benchmark.py"
    "#test_bbb_launch_arguments_pin_the_same_external_settings",
    STARTUP_BOOTSTRAP_OWNER_CENSUS_TEST,
)

STARTUP_RUNTIME_IMPLEMENTATION = (
    "crates/bbb-native/src/main.rs#fn run_client",
    "crates/bbb-native/src/network_runtime.rs#pub(crate) fn activate",
    "crates/bbb-native/src/client_options.rs#pub(crate) fn save_started_cleanly",
    "crates/bbb-native/src/client_options.rs#pub(crate) fn clear_startup_fullscreen_resolution",
    "crates/bbb-native/src/startup.rs#pub(crate) fn build_window",
)

STARTUP_RUNTIME_TESTS = (
    "crates/bbb-native/src/main.rs"
    "#startup_cleanliness_brackets_native_bootstrap_and_successful_initial_load",
    "crates/bbb-native/src/client_options.rs"
    "#startup_cleanliness_is_false_until_initial_load_finishes",
    "crates/bbb-native/src/client_options.rs"
    "#incomplete_startup_clears_fullscreen_resolution_only_on_successful_final_save",
    "crates/bbb-native/src/network_runtime.rs"
    "#external_connection_is_not_polled_until_initial_load_activation",
    *STARTUP_ARGUMENT_TESTS,
)

STARTUP_PROXY_IMPLEMENTATION = (
    "crates/bbb-native/src/startup.rs#pub(crate) fn server_resource_pack_proxy_from_args",
    "crates/bbb-net/src/resource_pack.rs#pub fn from_vanilla_args",
)

STARTUP_PROXY_TESTS = (
    "crates/bbb-native/src/startup.rs"
    "#args_project_vanilla_socks_proxy_without_exposing_password",
    "crates/bbb-net/src/resource_pack.rs"
    "#vanilla_proxy_arguments_fall_back_and_require_both_credentials",
    STARTUP_BOOTSTRAP_OWNER_CENSUS_TEST,
)

MAIN_STARTUP_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(STARTUP_RUNTIME_IMPLEMENTATION, STARTUP_RUNTIME_TESTS),
        ("main",),
    ),
    (
        OwnerEvidence(STARTUP_ARGUMENT_IMPLEMENTATION, STARTUP_ARGUMENT_TESTS),
        (
            "getQuickPlayVariant",
            "unescapeJavaArgument",
            "ofNullable",
            "parseArgument",
        ),
    ),
    (
        OwnerEvidence(STARTUP_PROXY_IMPLEMENTATION, STARTUP_PROXY_TESTS),
        ("stringHasValue",),
    ),
)

MINECRAFT_STARTUP_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(STARTUP_RUNTIME_IMPLEMENTATION, STARTUP_RUNTIME_TESTS),
        ("Minecraft", "onGameLoadFinished"),
    ),
    (
        OwnerEvidence(
            ("docs/unsupported-features.md",),
            (STARTUP_BOOTSTRAP_OWNER_CENSUS_TEST,),
            OwnerScopeRuling(
                "not-reachable-in-current-scope",
                (
                    "net/minecraft/client/Minecraft.java:2425-2429",
                    "net/minecraft/client/Minecraft.java:2903-2905",
                    "net/minecraft/client/telemetry/ClientTelemetryManager.java:47",
                ),
                "The repo-native executable has no JVM minecraft.launcher.brand property; the optional value only decorates crash/telemetry output and is absent in the repo-owned launcher contract.",
                "Reopen if BBB adds a generic launcher-brand startup field, exports launcher telemetry, or the official shared-launch contract sets minecraft.launcher.brand.",
            ),
        ),
        ("getLauncherBrand",),
    ),
)

STARTUP_IDENTITY_IMPLEMENTATION = (
    "crates/bbb-net/src/types/connection.rs#pub struct ClientIdentity",
    "crates/bbb-net/src/types/connection.rs#pub fn offline_with_identity",
    "crates/bbb-native/src/startup.rs#pub(crate) fn client_identity_from_args",
    "crates/bbb-native/src/startup.rs#fn parse_vanilla_launcher_uuid",
    "crates/bbb-native/src/main.rs#fn run_client",
    "crates/bbb-world/src/client/local_player.rs#pub fn set_local_player_profile_id",
    "crates/bbb-net/src/resource_pack.rs#fn download_headers",
)

STARTUP_IDENTITY_TESTS = (
    "crates/bbb-native/src/startup.rs"
    "#vanilla_launcher_uuid_is_lenient_and_one_identity_feeds_login",
    "crates/bbb-native/src/startup.rs"
    "#invalid_launcher_uuid_falls_back_to_the_username_offline_identity",
    "crates/bbb-net/src/types/connection.rs"
    "#explicit_launcher_identity_is_preserved_for_login_instead_of_rederived",
    "crates/bbb-native/src/main.rs"
    "#one_startup_identity_feeds_network_world_and_server_pack_headers",
    "crates/bbb-net/src/resource_pack.rs"
    "#server_pack_download_verifies_hash_headers_and_reuses_cache",
    STARTUP_IDENTITY_OWNER_CENSUS_TEST,
)

STARTUP_OFFLINE_SERVICE_IMPLEMENTATION = (
    "crates/bbb-native/src/services_block_list_runtime.rs#ServicesBlockListRuntime::new",
    "crates/bbb-net/src/chat_session.rs",
    "docs/unsupported-features.md",
)

STARTUP_OFFLINE_SERVICE_TESTS = (
    "crates/bbb-native/src/services_block_list_runtime.rs"
    "#services_block_list_offline_play_and_disconnect_stay_inactive",
    "crates/bbb-net/src/chat_session.rs#outbound_requests_are_unsigned_without_a_session",
    STARTUP_IDENTITY_OWNER_CENSUS_TEST,
)

MAIN_IDENTITY_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(STARTUP_IDENTITY_IMPLEMENTATION, STARTUP_IDENTITY_TESTS),
        ("emptyStringToEmptyOptional", "hasValidUuid", "isUuidValid"),
    ),
)

USER_IDENTITY_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(STARTUP_IDENTITY_IMPLEMENTATION, STARTUP_IDENTITY_TESTS),
        ("User", "getProfileId", "getName"),
    ),
    (
        OwnerEvidence(
            ("docs/unsupported-features.md",),
            (STARTUP_IDENTITY_OWNER_CENSUS_TEST,),
            OwnerScopeRuling(
                "not-reachable-in-current-scope",
                (
                    "net/minecraft/client/User.java:22-23",
                    "net/minecraft/client/Minecraft.java:429-430",
                ),
                "getSessionId only decorates Vanilla's debug log with the access token and undashed profile id; BBB deliberately retains no session secret or equivalent observable product surface.",
                "Reopen if BBB exposes an official session-id diagnostic contract or the offline-only scope decision is reversed.",
            ),
        ),
        ("getSessionId",),
    ),
    (
        OwnerEvidence(
            ("docs/unsupported-features.md",),
            (STARTUP_IDENTITY_OWNER_CENSUS_TEST,),
            OwnerScopeRuling(
                "not-reachable-in-current-scope",
                (
                    "net/minecraft/client/User.java:34-35",
                    "net/minecraft/client/Minecraft.java:832-834",
                ),
                "getAccessToken is consumed only by the online UserApiService branch; the declared offline-only client selects the OFFLINE service and owns no access-token startup field.",
                "Reopen only if the offline-only scope decision is reversed and the online authentication service is restored.",
            ),
        ),
        ("getAccessToken",),
    ),
    (
        OwnerEvidence(
            ("docs/unsupported-features.md",),
            (STARTUP_IDENTITY_OWNER_CENSUS_TEST,),
            OwnerScopeRuling(
                "not-reachable-in-current-scope",
                (
                    "net/minecraft/client/User.java:38-43",
                    "net/minecraft/client/telemetry/ClientTelemetryManager.java:36-41",
                ),
                "getClientId and getXuid only populate Mojang telemetry properties; BBB has no telemetry sender in the declared scope.",
                "Reopen if BBB gains a Mojang telemetry upload surface or the scope-telemetry-upload decision changes.",
            ),
        ),
        ("getClientId", "getXuid"),
    ),
)

MINECRAFT_IDENTITY_OWNER_EVIDENCE = _owner_evidence(
    (
        OwnerEvidence(
            STARTUP_OFFLINE_SERVICE_IMPLEMENTATION,
            STARTUP_OFFLINE_SERVICE_TESTS,
        ),
        ("createUserApiService",),
    ),
    (
        OwnerEvidence(STARTUP_IDENTITY_IMPLEMENTATION, STARTUP_IDENTITY_TESTS),
        ("getGameProfile",),
    ),
)

INDEXED_VANILLA_PROVIDER_IMPLEMENTATION = (
    "crates/bbb-pack/src/roots.rs#pub fn from_root",
    "crates/bbb-pack/src/selected_packs.rs#fn vanilla",
    "crates/bbb-pack/src/resources.rs#fn resource_file",
    "crates/bbb-pack/src/resources.rs#fn root_resource",
    "crates/bbb-pack/src/resources.rs#fn list_resources",
)

INDEXED_VANILLA_PROVIDER_TESTS = (
    "crates/bbb-pack/src/roots.rs"
    "#indexed_vanilla_provider_replaces_source_stub_but_selected_pack_wins",
    "crates/bbb-pack/src/selected_packs.rs"
    "#startup_stack_records_identity_policy_and_compatibility",
    "crates/bbb-pack/src/resources.rs"
    "#single_resource_and_list_resources_use_highest_precedence_metadata",
    "crates/bbb-pack/src/resources.rs"
    "#resource_location_matches_vanilla_identifier_validation",
    INDEXED_VANILLA_PROVIDER_OWNER_CENSUS_TEST,
)

INDEXED_VANILLA_PROVIDER_OWNER_EVIDENCE = OwnerEvidence(
    INDEXED_VANILLA_PROVIDER_IMPLEMENTATION,
    INDEXED_VANILLA_PROVIDER_TESTS,
)

CLIENT_PACK_SOURCE_PROVIDER_OWNER_EVIDENCE = _owner_evidence(
    (
        INDEXED_VANILLA_PROVIDER_OWNER_EVIDENCE,
        ("ClientPackSource", "createVanillaPackSource", "createVanillaPack"),
    ),
)

VANILLA_PACK_BUILDER_OWNER_EVIDENCE = _owner_evidence(
    (
        INDEXED_VANILLA_PROVIDER_OWNER_EVIDENCE,
        (
            "validateDirPath",
            "pushRootPath",
            "pushPathForType",
            "pushJarResources",
            "pushAssetPath",
            "setMetadata",
            "exposeNamespace",
            "build",
            "copyAndReverse",
        ),
    ),
    (
        OwnerEvidence(
            ("docs/unsupported-features.md",),
            (INDEXED_VANILLA_PROVIDER_OWNER_CENSUS_TEST,),
            OwnerScopeRuling(
                "not-reachable-in-current-scope",
                (
                    "net/minecraft/server/packs/VanillaPackResourcesBuilder.java:31",
                    "net/minecraft/server/packs/VanillaPackResourcesBuilder.java:121-123",
                ),
                "Vanilla 26.1 production keeps developmentConfig as the empty consumer, so the call contributes no additional provider path.",
                "Reopen if the official production client assigns developmentConfig or BBB adds a development-resource launch mode.",
            ),
        ),
        ("applyDevelopmentConfig",),
    ),
)

VANILLA_PACK_RESOURCES_OWNER_EVIDENCE = _owner_evidence(
    (
        INDEXED_VANILLA_PROVIDER_OWNER_EVIDENCE,
        (
            "VanillaPackResources",
            "getRootResource",
            "listResources",
            "getResources",
            "getResource",
            "getNamespaces",
            "getMetadataSection",
            "asProvider",
        ),
    ),
)

GAME_CONFIG_ASSET_TRUST_OWNER_EVIDENCE = {
    "getExternalAssetSource": OwnerEvidence(
        (
            "crates/bbb-pack/src/roots.rs#pub fn from_root",
            "crates/bbb-pack/src/asset_index.rs#pub(crate) fn discover",
        ),
        (
            "crates/bbb-pack/src/asset_index.rs"
            "#missing_version_metadata_keeps_external_asset_source_optional",
            "crates/bbb-pack/src/asset_index.rs"
            "#rejects_oversized_index_before_opening_or_hashing_it",
            OFFLINE_ASSET_TRUST_OWNER_CENSUS_TEST,
        ),
    ),
}

INDEXED_ASSET_SOURCE_OWNER_EVIDENCE = {
    "createIndexFs": OwnerEvidence(
        (
            "crates/bbb-pack/src/asset_index.rs#pub(crate) fn discover",
            "crates/bbb-pack/src/asset_index.rs#pub(crate) fn resource",
            "crates/bbb-pack/src/asset_index.rs#pub(crate) fn root_resource",
            "crates/bbb-pack/src/asset_index.rs#pub(crate) fn list_resources",
            "crates/bbb-pack/src/resources.rs#impl ResourceIntegrity",
        ),
        (
            "crates/bbb-pack/src/asset_index.rs#discovers_and_verifies_indexed_object",
            "crates/bbb-pack/src/asset_index.rs#rejects_wrong_object_size_and_sha1",
            "crates/bbb-pack/src/asset_index.rs"
            "#rejects_wrong_index_metadata_and_missing_object",
            "crates/bbb-pack/src/asset_index.rs"
            "#rejects_path_escape_index_id_and_asset_key",
            "crates/bbb-pack/src/asset_index.rs"
            "#rejects_unbounded_declared_object_size_without_opening_an_object",
            "crates/bbb-pack/src/asset_index.rs"
            "#rejects_unbounded_object_count_and_total_declared_bytes",
            OFFLINE_ASSET_TRUST_OWNER_CENSUS_TEST,
        ),
    ),
}

METHOD_OWNER_EVIDENCE_BY_SOURCE = {
    **AUDIO_OWNER_EVIDENCE_BY_SOURCE,
    **CAMERA_FOG_OWNER_EVIDENCE_BY_SOURCE,
    **SERVER_PACK_OWNER_EVIDENCE_BY_SOURCE,
    **HUD_TOAST_OWNER_EVIDENCE_BY_SOURCE,
    "net/minecraft/client/gui/render/GuiRenderer.java": PANORAMA_GUI_RENDERER_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/CubeMap.java": PANORAMA_CUBE_MAP_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/texture/CubeMapTexture.java": PANORAMA_CUBE_TEXTURE_OWNER_EVIDENCE,
    "net/minecraft/client/renderer/Panorama.java": PANORAMA_STATE_OWNER_EVIDENCE,
    "net/minecraft/client/main/Main.java": {
        **MAIN_STARTUP_OWNER_EVIDENCE,
        **MAIN_IDENTITY_OWNER_EVIDENCE,
    },
    "net/minecraft/client/User.java": USER_IDENTITY_OWNER_EVIDENCE,
    "net/minecraft/client/main/GameConfig.java": GAME_CONFIG_ASSET_TRUST_OWNER_EVIDENCE,
    "net/minecraft/client/resources/ClientPackSource.java": CLIENT_PACK_SOURCE_PROVIDER_OWNER_EVIDENCE,
    "net/minecraft/client/resources/IndexedAssetSource.java": INDEXED_ASSET_SOURCE_OWNER_EVIDENCE,
    "net/minecraft/server/packs/VanillaPackResourcesBuilder.java": VANILLA_PACK_BUILDER_OWNER_EVIDENCE,
    "net/minecraft/server/packs/VanillaPackResources.java": VANILLA_PACK_RESOURCES_OWNER_EVIDENCE,
    "net/minecraft/client/MusicToastDisplayState.java": MUSIC_TOAST_DISPLAY_OWNER_EVIDENCE,
    "net/minecraft/client/resources/language/LanguageManager.java": LANGUAGE_MANAGER_OWNER_EVIDENCE,
    "net/minecraft/client/resources/language/ClientLanguage.java": CLIENT_LANGUAGE_OWNER_EVIDENCE,
    "net/minecraft/client/gui/screens/recipebook/RecipeBookComponent.java": RECIPE_BOOK_LANGUAGE_OWNER_EVIDENCE,
    "com/mojang/blaze3d/platform/IconSet.java": WINDOW_ICON_SET_OWNER_EVIDENCE,
}


METHOD_OWNER_EVIDENCE = {
    "startup-client-bootstrap": {
        **MAIN_STARTUP_OWNER_EVIDENCE,
        **MINECRAFT_STARTUP_OWNER_EVIDENCE,
    },
    "startup-session-identity": {
        **MAIN_IDENTITY_OWNER_EVIDENCE,
        **USER_IDENTITY_OWNER_EVIDENCE,
        **MINECRAFT_IDENTITY_OWNER_EVIDENCE,
    },
    "resources-offline-asset-trust": {
        **GAME_CONFIG_ASSET_TRUST_OWNER_EVIDENCE,
        **INDEXED_ASSET_SOURCE_OWNER_EVIDENCE,
    },
    "resources-indexed-vanilla-provider": {
        **CLIENT_PACK_SOURCE_PROVIDER_OWNER_EVIDENCE,
        **VANILLA_PACK_BUILDER_OWNER_EVIDENCE,
        **VANILLA_PACK_RESOURCES_OWNER_EVIDENCE,
    },
    "resources-language-manager": {
        **LANGUAGE_MANAGER_OWNER_EVIDENCE,
        **CLIENT_LANGUAGE_OWNER_EVIDENCE,
        **RECIPE_BOOK_LANGUAGE_OWNER_EVIDENCE,
    },
    "ui-bidirectional-shaping": CLIENT_LANGUAGE_OWNER_EVIDENCE,
    "resources-server-pack-lifecycle": SERVER_PACK_ROOT_OWNER_EVIDENCE,
    "audio-sound-manager-engine": AUDIO_SOUND_MANAGER_OWNER_EVIDENCE,
    "security-secure-chat-reporting": SECURITY_SECURE_CHAT_REPORTING_OWNER_EVIDENCE,
    "rendering-frame-order": RENDERING_FRAME_ORDER_OWNER_EVIDENCE,
    "rendering-extract-surface": RENDERING_EXTRACT_SURFACE_OWNER_EVIDENCE,
    "rendering-level-pass": RENDERING_LEVEL_PASS_OWNER_EVIDENCE,
    "rendering-effects-screenshot": RENDERING_EFFECTS_SCREENSHOT_OWNER_EVIDENCE,
    "resources-indexed-panorama": PANORAMA_GAME_RENDERER_OWNER_EVIDENCE,
    "resources-window-icons": WINDOW_ICON_WINDOW_OWNER_EVIDENCE,
    "rendering-camera-fluid-fog": {},
    "rendering-chunk-layer-routing": RENDERING_CHUNK_LAYER_OWNER_EVIDENCE,
    "input-keyboard-routing": INPUT_KEYBOARD_ROUTING_OWNER_EVIDENCE,
    "input-debug-key-actions": INPUT_DEBUG_KEY_ACTIONS_OWNER_EVIDENCE,
    "input-mouse-routing": INPUT_MOUSE_ROUTING_OWNER_EVIDENCE,
    "input-key-mapping": INPUT_KEY_MAPPING_OWNER_EVIDENCE,
    "ui-accessibility-surface": UI_ACCESSIBILITY_SURFACE_OWNER_EVIDENCE,
    "ui-clipboard-surface": UI_CLIPBOARD_SURFACE_OWNER_EVIDENCE,
    "platform-window-lifecycle": PLATFORM_WINDOW_LIFECYCLE_OWNER_EVIDENCE,
    "platform-window-display-mode": PLATFORM_WINDOW_DISPLAY_MODE_OWNER_EVIDENCE,
    "platform-window-surface-state": PLATFORM_WINDOW_SURFACE_STATE_OWNER_EVIDENCE,
    "platform-window-chrome": PLATFORM_WINDOW_CHROME_OWNER_EVIDENCE,
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
    "persistence-server-list": SERVER_LIST_OWNER_EVIDENCE,
    "persistence-options-file": OPTIONS_FILE_OWNER_EVIDENCE,
    "persistence-options-unconsumed": OPTIONS_DATAFIX_OWNER_EVIDENCE,
    "persistence-options-derived": OPTIONS_DERIVED_OWNER_EVIDENCE,
    "persistence-options-apply": OPTIONS_APPLY_OWNER_EVIDENCE,
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
    "hud-frame-extract-order": HUD_FRAME_EXTRACT_ORDER_OWNER_EVIDENCE,
    "hud-player-vitals": HUD_PLAYER_VITALS_OWNER_EVIDENCE,
    "hud-hotbar-selection": HUD_HOTBAR_SELECTION_OWNER_EVIDENCE,
    "hud-overlay-effects": HUD_OVERLAY_EFFECTS_OWNER_EVIDENCE,
    "hud-text-and-lists": HUD_TEXT_AND_LISTS_OWNER_EVIDENCE,
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


RELOAD_LISTENER_OWNER_CENSUS_TEST = (
    "scripts/tests/test_client_mechanism_inventory.py"
    "#test_reload_listener_owner_census_has_complete_evidence_and_scope_rulings"
)

RELOAD_LISTENER_BARRIER_IMPLEMENTATION = (
    "crates/bbb-native/src/resource_reload.rs#fn prepare_native_resource_reload",
    "crates/bbb-native/src/resource_reload.rs#fn apply_native_resource_reload",
    "crates/bbb-renderer/src/resource_reload.rs#pub(crate) fn transactional_resource_reload",
)

RELOAD_LISTENER_BARRIER_TESTS = (
    "crates/bbb-renderer/src/resource_reload.rs"
    "#injected_failure_after_each_reload_stage_restores_exact_accepted_owners",
    "crates/bbb-native/src/resource_pack_runtime.rs"
    "#failed_reload_keeps_previous_active_stack_and_fails_only_new_pack",
)


def _reload_listener_evidence(
    implementation: tuple[str, ...],
    tests: tuple[str, ...],
) -> OwnerEvidence:
    return OwnerEvidence(
        RELOAD_LISTENER_BARRIER_IMPLEMENTATION + implementation,
        RELOAD_LISTENER_BARRIER_TESTS + tests + (RELOAD_LISTENER_OWNER_CENSUS_TEST,),
    )


SPLASH_RELOAD_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=(
        "net/minecraft/client/resources/SplashManager.java:39-64",
        "net/minecraft/client/gui/screens/TitleScreen.java:95-99",
    ),
    current_scope_reason=(
        "SplashManager prepares texts/splashes.txt for TitleScreen.init; the declared client "
        "starts directly from command-line configuration and has no title-screen surface."
    ),
    reopen_condition="BBB adds a title-screen surface.",
)

GPU_WARNLIST_RELOAD_SCOPE = OwnerScopeRuling(
    conclusion="not-reachable-in-current-scope",
    vanilla_basis=(
        "net/minecraft/client/renderer/GpuWarnlistManager.java:76-94,137-155",
        "net/minecraft/client/Options.java:197-207",
        "net/minecraft/client/gui/screens/options/VideoSettingsScreen.java:77-87",
    ),
    current_scope_reason=(
        "Vanilla emits warnlist matches only for the OpenGL backend and consumes them through "
        "the improved-transparency options flow. BBB ships wgpu Vulkan/Metal/DX12 backends and "
        "has no in-game options UI."
    ),
    reopen_condition=(
        "BBB supports an OpenGL backend or the improved-transparency warning/options surface."
    ),
)

RELOAD_LISTENER_OWNER_EVIDENCE = _registration_evidence(
    (
        _reload_listener_evidence(
            (
                "crates/bbb-item-model/src/item_runtime.rs"
                "#pub fn load_with_locale_and_font_options",
            ),
            (
                "crates/bbb-pack/src/language.rs"
                "#pack_roots_client_language_catalog_uses_pack_metadata_for_selected_language",
                "crates/bbb-item-model/src/item_runtime/tests.rs"
                "#world_text_font_direction_tracks_the_selected_language_owner",
            ),
        ),
        ("this.languageManager",),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-pack/src/resource_textures.rs"
                "#pub fn load_resource_texture_image",
            ),
            (
                "crates/bbb-pack/src/resource_textures.rs"
                "#loads_namespaced_resource_textures_with_pack_precedence_and_real_dimensions",
            ),
        ),
        ("this.textureManager",),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-renderer/src/core_shaders.rs"
                "#pub(crate) fn compile_selected_shader_stages",
                "crates/bbb-renderer/src/resource_reload.rs"
                "#pub fn transactional_shader_resource_reload",
            ),
            (
                "crates/bbb-renderer/src/resource_reload.rs"
                "#local_shader_resource_generation_commits_and_rolls_back_as_one_owner",
            ),
        ),
        ("this.shaderManager",),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-native/src/audio_runtime.rs#pub(crate) fn prepare_reload",
                "crates/bbb-native/src/audio_runtime.rs#pub(crate) fn commit_reload",
            ),
            (
                "crates/bbb-native/src/audio_runtime.rs"
                "#preload_plan_stamps_catalog_generation_and_receipt_is_machine_readable",
                "crates/bbb-pack/src/sounds.rs"
                "#pack_roots_stamp_sound_catalog_with_exact_resource_generation",
            ),
        ),
        ("this.soundManager",),
    ),
    (
        OwnerEvidence(
            ("docs/unsupported-features.md",),
            (RELOAD_LISTENER_OWNER_CENSUS_TEST,),
            SPLASH_RELOAD_SCOPE,
        ),
        ("this.splashManager",),
    ),
    (
        _reload_listener_evidence(
            ("crates/bbb-native/src/resource_reload.rs#fn upload_reload_atlases",),
            (
                "crates/bbb-pack/src/roots.rs"
                "#separate_atlases_share_one_generation_image_budget",
                "crates/bbb-pack/src/atlas.rs"
                "#atlas_mips_are_generated_per_sprite_without_cross_sprite_blending",
            ),
        ),
        ("this.atlasManager",),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-item-model/src/font.rs"
                "#pub fn load_hud_font_atlas_with_options",
            ),
            (
                "crates/bbb-item-model/src/font.rs"
                "#font_book_stacks_fonts_into_one_atlas_and_keeps_per_font_metrics",
            ),
        ),
        ("this.fontManager",),
    ),
    (
        _reload_listener_evidence(
            ("crates/bbb-pack/src/roots.rs#pub fn load_terrain_colormaps",),
            (
                "crates/bbb-pack/src/roots.rs"
                "#pack_roots_terrain_colormaps_use_resource_pack_precedence",
            ),
        ),
        (
            "new GrassColorReloadListener ()",
            "new FoliageColorReloadListener ()",
            "new DryFoliageColorReloadListener ()",
        ),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-item-model/src/item_runtime.rs"
                "#pub fn load_with_locale_and_font_options",
            ),
            (
                "crates/bbb-pack/src/selected_packs.rs"
                "#one_selected_generation_composes_custom_model_texture_font_sound_and_data_resources",
                "crates/bbb-pack/src/block_models/tests.rs"
                "#block_model_catalog_uses_resource_pack_precedence_for_blockstates_and_models",
            ),
        ),
        ("this.modelManager",),
    ),
    (
        _reload_listener_evidence(
            ("crates/bbb-pack/src/roots.rs#pub fn load_equipment_asset_catalog",),
            (
                "crates/bbb-pack/src/equipment_assets.rs"
                "#equipment_asset_catalog_uses_resource_pack_precedence",
            ),
        ),
        ("equipmentAssets",),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-native/src/entity_assets.rs"
                "#pub(crate) fn try_load_entity_model_textures",
            ),
            (
                "crates/bbb-native/src/entity_assets.rs"
                "#loads_pack_defined_entity_texture_resolution",
            ),
        ),
        ("this.entityRenderDispatcher",),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-renderer/src/resource_reload.rs"
                "#pub(crate) fn transactional_resource_reload",
            ),
            (
                "crates/bbb-renderer/src/resource_reload.rs"
                "#successful_reload_commits_new_owner_and_discards_checkpoint",
            ),
        ),
        ("this.blockEntityRenderDispatcher",),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-native/src/particle_runtime.rs"
                "#pub(crate) fn load",
            ),
            (
                "crates/bbb-pack/src/particle_definitions.rs"
                "#particle_definition_catalog_uses_resource_pack_overrides",
                "crates/bbb-pack/src/particle_sprites.rs"
                "#particle_sprite_catalog_loads_particle_atlas_images",
            ),
        ),
        ("this.particleResources",),
    ),
    (
        _reload_listener_evidence(
            ("crates/bbb-pack/src/roots.rs#pub fn load_waypoint_style_catalog",),
            (
                "crates/bbb-pack/src/waypoint_styles.rs"
                "#waypoint_style_catalog_uses_resource_pack_overrides",
                "crates/bbb-renderer/src/resource_reload.rs"
                "#failed_waypoint_locator_reload_restores_background_style_and_marker_sprite",
            ),
        ),
        ("this.waypointStyles",),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-native/src/sky_assets.rs"
                "#pub(crate) fn try_load_sky_textures",
                "crates/bbb-native/src/resource_reload.rs"
                "#terrain_upload.request_reload_all_chunks();",
            ),
            (
                "crates/bbb-native/src/sky_assets.rs"
                "#loads_vanilla_end_sky_texture_from_resource_stack",
            ),
        ),
        ("this.levelRenderer",),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-native/src/sky_assets.rs"
                "#fn try_load_cloud_texture",
            ),
            (
                "crates/bbb-native/src/sky_assets.rs"
                "#loads_vanilla_cloud_texture_from_resource_stack",
            ),
        ),
        ("this.levelRenderer.getCloudRenderer ()",),
    ),
    (
        OwnerEvidence(
            ("docs/unsupported-features.md",),
            (RELOAD_LISTENER_OWNER_CENSUS_TEST,),
            GPU_WARNLIST_RELOAD_SCOPE,
        ),
        ("this.gpuWarnlistManager",),
    ),
    (
        _reload_listener_evidence(
            (
                "crates/bbb-native/src/runtime.rs"
                "#pub(crate) fn prepare_periodic_notifications",
                "crates/bbb-native/src/runtime.rs"
                "#pub(crate) fn install_periodic_notifications",
            ),
            (
                "crates/bbb-native/src/runtime/periodic_notifications.rs"
                "#reload_preserves_elapsed_but_restarts_selected_initial_delay",
                "crates/bbb-native/src/runtime/periodic_notifications.rs"
                "#due_notification_projects_through_the_shared_system_toast_surface",
            ),
        ),
        ("this.regionalCompliancies",),
    ),
)


REGISTRATION_OWNER_EVIDENCE = {
    "rendering-block-entity-registration": BLOCK_ENTITY_RENDERER_OWNER_EVIDENCE,
    "resources-reload-listeners": RELOAD_LISTENER_OWNER_EVIDENCE,
}


OWNER_EVIDENCE_MECHANISMS = tuple(METHOD_OWNER_EVIDENCE) + tuple(REGISTRATION_OWNER_EVIDENCE)


METHOD_SURFACES = (
    MethodSurface("startup", "startup", "startup-client-bootstrap", "net/minecraft/client/main/Main.java"),
    MethodSurface("startup", "startup", "startup-session-identity", "net/minecraft/client/User.java"),
    MethodSurface("lifecycle", "lifecycle", "lifecycle-client-runtime", "net/minecraft/client/Minecraft.java"),
    MethodSurface(
        "resources",
        "resources",
        "resources-offline-asset-trust",
        "net/minecraft/client/main/GameConfig.java",
        selected_methods=("getExternalAssetSource",),
    ),
    MethodSurface("resources", "resources", "resources-default-pack-stack", "net/minecraft/client/resources/ClientPackSource.java"),
    MethodSurface(
        "resource-consumer",
        "resources",
        "resources-window-icons",
        "com/mojang/blaze3d/platform/IconSet.java",
        selected_methods=("IconSet", "getStandardIcons", "getMacIcon", "getFile"),
    ),
    MethodSurface("resources", "resources", "resources-offline-asset-trust", "net/minecraft/client/resources/IndexedAssetSource.java"),
    MethodSurface(
        "resource-provider",
        "resources",
        "resources-indexed-vanilla-provider",
        "net/minecraft/server/packs/VanillaPackResourcesBuilder.java",
        selected_methods=(
            "validateDirPath",
            "pushRootPath",
            "pushPathForType",
            "pushJarResources",
            "applyDevelopmentConfig",
            "pushAssetPath",
            "setMetadata",
            "exposeNamespace",
            "build",
            "copyAndReverse",
        ),
    ),
    MethodSurface(
        "resource-provider",
        "resources",
        "resources-indexed-vanilla-provider",
        "net/minecraft/server/packs/VanillaPackResources.java",
        selected_methods=(
            "VanillaPackResources",
            "getRootResource",
            "listResources",
            "getResources",
            "getResource",
            "getNamespaces",
            "getMetadataSection",
            "asProvider",
        ),
    ),
    MethodSurface("resources", "resources", "resources-default-pack-stack", "net/minecraft/server/packs/resources/FallbackResourceManager.java"),
    MethodSurface("resources", "resources", "resources-default-pack-stack", "net/minecraft/server/packs/resources/MultiPackResourceManager.java"),
    MethodSurface("resources", "resources", "resources-server-pack-lifecycle", "net/minecraft/client/resources/server/DownloadedPackSource.java"),
    MethodSurface("resources", "resources", "resources-server-pack-lifecycle", "net/minecraft/client/resources/server/ServerPackManager.java"),
    MethodSurface("resources", "resources", "resources-server-pack-lifecycle", "net/minecraft/server/packs/DownloadQueue.java"),
    MethodSurface("resources", "resources", "resources-server-pack-lifecycle", "net/minecraft/server/packs/DownloadCacheCleaner.java"),
    MethodSurface("resources", "resources", "resources-server-pack-lifecycle", "net/minecraft/util/HttpUtil.java"),
    MethodSurface("protocol-handler", "protocol", "protocol-login-handler-decisions", "net/minecraft/client/multiplayer/ClientHandshakePacketListenerImpl.java"),
    MethodSurface("protocol-handler", "protocol", "protocol-configuration-handler-decisions", "net/minecraft/client/multiplayer/ClientConfigurationPacketListenerImpl.java"),
    MethodSurface("protocol-handler", "protocol", "protocol-play-handler-decisions", "net/minecraft/client/multiplayer/ClientCommonPacketListenerImpl.java"),
    MethodSurface("protocol-handler", "protocol", "protocol-play-handler-decisions", "net/minecraft/client/multiplayer/ClientPacketListener.java"),
    MethodSurface("world", "world", "world-client-level-decisions", "net/minecraft/client/multiplayer/ClientLevel.java"),
    MethodSurface("movement", "movement", "player-local-input-movement", "net/minecraft/client/player/LocalPlayer.java"),
    MethodSurface("interaction", "interaction", "interaction-primary-action-routing", "net/minecraft/client/multiplayer/MultiPlayerGameMode.java"),
    MethodSurface("render-step", "ui", "hud-frame-extract-order", "net/minecraft/client/gui/Gui.java"),
    MethodSurface("render-step", "ui", "hud-frame-extract-order", "net/minecraft/client/gui/components/toasts/Toast.java"),
    MethodSurface("render-step", "ui", "hud-frame-extract-order", "net/minecraft/client/gui/components/toasts/ToastManager.java"),
    MethodSurface("render-step", "ui", "hud-frame-extract-order", "net/minecraft/client/gui/components/toasts/SystemToast.java"),
    MethodSurface("render-step", "ui", "hud-frame-extract-order", "net/minecraft/client/gui/components/toasts/AdvancementToast.java"),
    MethodSurface("render-step", "ui", "hud-frame-extract-order", "net/minecraft/client/gui/components/toasts/RecipeToast.java"),
    MethodSurface("render-step", "ui", "hud-frame-extract-order", "net/minecraft/client/gui/components/toasts/TutorialToast.java"),
    MethodSurface("render-step", "ui", "hud-frame-extract-order", "net/minecraft/client/gui/components/toasts/NowPlayingToast.java"),
    MethodSurface("render-step", "ui", "hud-frame-extract-order", "net/minecraft/client/PeriodicNotificationManager.java"),
    MethodSurface(
        "resource-consumer",
        "resources",
        "resources-indexed-panorama",
        "net/minecraft/client/gui/render/GuiRenderer.java",
        selected_methods=("GuiRenderer", "render", "registerPanoramaTextures"),
    ),
    MethodSurface(
        "resource-consumer",
        "resources",
        "resources-indexed-panorama",
        "net/minecraft/client/renderer/CubeMap.java",
        selected_methods=(
            "CubeMap",
            "render",
            "initializeVertices",
            "registerTextures",
            "close",
        ),
    ),
    MethodSurface(
        "resource-consumer",
        "resources",
        "resources-indexed-panorama",
        "net/minecraft/client/renderer/texture/CubeMapTexture.java",
        selected_methods=("CubeMapTexture", "loadContents", "doLoad"),
    ),
    MethodSurface(
        "resource-consumer",
        "resources",
        "resources-indexed-panorama",
        "net/minecraft/client/renderer/Panorama.java",
        selected_methods=("extractRenderState",),
    ),
    MethodSurface("render-step", "rendering", "rendering-frame-order", "net/minecraft/client/renderer/GameRenderer.java"),
    MethodSurface("render-step", "rendering", "rendering-level-frame-graph", "net/minecraft/client/renderer/LevelRenderer.java"),
    MethodSurface(
        "render-layer",
        "rendering",
        "rendering-chunk-layer-routing",
        "net/minecraft/client/renderer/chunk/SectionCompiler.java",
        selected_methods=("compile", "getOrBeginLayer"),
    ),
    MethodSurface(
        "render-layer",
        "rendering",
        "rendering-chunk-layer-routing",
        "com/mojang/blaze3d/vertex/MeshData.java",
        selected_methods=("sortQuads", "buildSortedIndexBuffer"),
    ),
    MethodSurface(
        "render-layer",
        "rendering",
        "rendering-chunk-layer-routing",
        "com/mojang/blaze3d/vertex/VertexSorting.java",
    ),
    MethodSurface(
        "render-layer",
        "rendering",
        "rendering-chunk-layer-routing",
        "net/minecraft/client/renderer/chunk/SectionRenderDispatcher.java",
        selected_methods=("createVertexSorting",),
    ),
    MethodSurface(
        "camera-fog",
        "rendering",
        "rendering-camera-fluid-fog",
        "net/minecraft/client/Camera.java",
        selected_methods=("getFluidInCamera", "modifyFovBasedOnDeathOrFluid"),
    ),
    MethodSurface(
        "camera-fog",
        "rendering",
        "rendering-camera-fluid-fog",
        "net/minecraft/client/renderer/fog/FogRenderer.java",
        selected_methods=("computeFogColor", "setupFog", "getFogType"),
    ),
    MethodSurface(
        "camera-fog",
        "rendering",
        "rendering-camera-fluid-fog",
        "net/minecraft/client/renderer/fog/environment/FogEnvironment.java",
        selected_methods=(
            "setupFog",
            "providesColor",
            "getBaseColor",
            "modifiesDarkness",
            "getModifiedDarkness",
            "isApplicable",
        ),
    ),
    MethodSurface(
        "camera-fog",
        "rendering",
        "rendering-camera-fluid-fog",
        "net/minecraft/client/renderer/fog/environment/MobEffectFogEnvironment.java",
        selected_methods=(
            "getMobEffect",
            "providesColor",
            "modifiesDarkness",
            "isApplicable",
        ),
    ),
    MethodSurface(
        "camera-fog",
        "rendering",
        "rendering-camera-fluid-fog",
        "net/minecraft/client/renderer/fog/environment/AtmosphericFogEnvironment.java",
        selected_methods=(
            "getBaseColor",
            "applyWeatherDarken",
            "setupFog",
            "updateRainFogState",
            "isApplicable",
        ),
    ),
    MethodSurface(
        "camera-fog",
        "rendering",
        "rendering-camera-fluid-fog",
        "net/minecraft/client/renderer/fog/environment/WaterFogEnvironment.java",
        selected_methods=("setupFog", "isApplicable", "getBaseColor"),
    ),
    MethodSurface(
        "camera-fog",
        "rendering",
        "rendering-camera-fluid-fog",
        "net/minecraft/client/renderer/fog/environment/LavaFogEnvironment.java",
        selected_methods=("getBaseColor", "setupFog", "isApplicable"),
    ),
    MethodSurface(
        "camera-fog",
        "rendering",
        "rendering-camera-fluid-fog",
        "net/minecraft/client/renderer/fog/environment/PowderedSnowFogEnvironment.java",
        selected_methods=("getBaseColor", "setupFog", "isApplicable"),
    ),
    MethodSurface(
        "camera-fog",
        "rendering",
        "rendering-camera-fluid-fog",
        "net/minecraft/client/renderer/fog/environment/BlindnessFogEnvironment.java",
        selected_methods=("getMobEffect", "setupFog", "getModifiedDarkness"),
    ),
    MethodSurface(
        "camera-fog",
        "rendering",
        "rendering-camera-fluid-fog",
        "net/minecraft/client/renderer/fog/environment/DarknessFogEnvironment.java",
        selected_methods=("getMobEffect", "setupFog", "getModifiedDarkness"),
    ),
    MethodSurface("resource-consumer", "language", "resources-language-manager", "net/minecraft/client/resources/language/LanguageManager.java"),
    MethodSurface("resource-consumer", "language", "resources-language-manager", "net/minecraft/client/resources/language/ClientLanguage.java"),
    MethodSurface(
        "resource-consumer",
        "language",
        "resources-language-manager",
        "net/minecraft/client/gui/screens/recipebook/RecipeBookComponent.java",
        selected_methods=("checkSearchStringUpdate", "pirateSpeechForThePeople"),
    ),
    MethodSurface("resource-consumer", "text", "ui-font-manager", "net/minecraft/client/gui/font/FontManager.java"),
    MethodSurface("resource-consumer", "audio", "audio-sound-manager-engine", "net/minecraft/client/sounds/SoundManager.java"),
    MethodSurface("resource-consumer", "audio", "audio-sound-manager-engine", "net/minecraft/client/sounds/MusicManager.java"),
    MethodSurface("resource-consumer", "audio", "audio-sound-manager-engine", "net/minecraft/client/sounds/SoundEngine.java"),
    MethodSurface("audio-backend", "audio", "audio-sound-manager-engine", "com/mojang/blaze3d/audio/Library.java"),
    MethodSurface("audio-backend", "audio", "audio-sound-manager-engine", "com/mojang/blaze3d/audio/Channel.java"),
    MethodSurface("audio-backend", "audio", "audio-sound-manager-engine", "com/mojang/blaze3d/audio/Listener.java"),
    MethodSurface("audio-backend", "audio", "audio-sound-manager-engine", "com/mojang/blaze3d/audio/SoundBuffer.java"),
    MethodSurface("audio-backend", "audio", "audio-sound-manager-engine", "net/minecraft/client/sounds/ChannelAccess.java"),
    MethodSurface("audio-backend", "audio", "audio-sound-manager-engine", "net/minecraft/client/sounds/SoundBufferLibrary.java"),
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
    MethodSurface("persistence", "persistence", "persistence-options-apply", "net/minecraft/client/MusicToastDisplayState.java"),
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
    "startup-client-bootstrap": (
        "crates/bbb-native/src/startup.rs#fn apply_vanilla_bootstrap_arguments",
        "crates/bbb-native/src/startup.rs#fn apply_stored_window_override",
        "crates/bbb-native/src/startup.rs#pub(crate) fn name_current_render_thread",
        "crates/bbb-native/src/startup.rs#pub(crate) fn build_window",
        "crates/bbb-native/src/main.rs#fn run_client",
        "crates/bbb-native/src/network_runtime.rs#pub(crate) fn activate",
        "crates/bbb-native/src/client_options.rs#pub(crate) fn save_started_cleanly",
        "crates/bbb-native/src/client_options.rs#pub(crate) fn clear_startup_fullscreen_resolution",
        STARTUP_BOOTSTRAP_OWNER_CENSUS_TEST,
    ),
    "startup-session-identity": (
        "crates/bbb-net/src/types/connection.rs#pub struct ClientIdentity",
        "crates/bbb-native/src/startup.rs#pub(crate) fn client_identity_from_args",
        "crates/bbb-native/src/startup.rs#fn parse_vanilla_launcher_uuid",
        "crates/bbb-native/src/main.rs#fn run_client",
        "crates/bbb-net/src/resource_pack.rs#fn download_headers",
        STARTUP_IDENTITY_OWNER_CENSUS_TEST,
    ),
    "lifecycle-client-runtime": (
        "crates/bbb-native/src/network_runtime.rs#production_shutdown_has_one_owner_and_frozen_order",
        "crates/bbb-native/src/simulation/thread.rs#shutdown_and_join",
        "crates/bbb-native/src/runtime/tests.rs#entity_client_tick_cadence_tracks_normal_world_tick_rate",
        "crates/bbb-native/src/runtime/tests.rs#native_pause_screens_never_freeze_external_client_or_running_level_ticks",
        "crates/bbb-native/src/runtime/tests.rs#first_multiplayer_tutorial_queues_the_joined_sentinel_in_the_same_effect_batch_once",
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
    "resources-offline-asset-trust": (
        "crates/bbb-pack/src/asset_index.rs",
        "crates/bbb-pack/src/roots.rs#pub fn from_root",
        "crates/bbb-pack/src/resources.rs#impl ResourceIntegrity",
        "docs/wiki/mechanisms/resources/offline-asset-trust.md",
    ),
    "resources-indexed-vanilla-provider": (
        *INDEXED_VANILLA_PROVIDER_IMPLEMENTATION,
        *INDEXED_VANILLA_PROVIDER_TESTS,
        "docs/wiki/mechanisms/resources/offline-asset-trust.md",
    ),
    "resources-indexed-panorama": (
        "crates/bbb-pack/src/panorama.rs#pub struct PanoramaCatalog",
        "crates/bbb-native/src/hud_assets.rs#pub(crate) fn upload_panorama",
        "crates/bbb-native/src/resource_reload.rs#fn apply_native_resource_reload",
        "crates/bbb-renderer/src/panorama.rs#struct PanoramaGpu",
        "crates/bbb-renderer/src/render.rs#fn panorama_pass",
        "crates/bbb-renderer/src/offscreen.rs#offscreen_panorama_preserves_vanilla_face_row_orientation",
        PANORAMA_OWNER_CENSUS_TEST,
        "docs/wiki/mechanisms/resources/offline-asset-trust.md",
    ),
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
        "crates/bbb-native/src/terrain_runtime/textures.rs#pub(crate) fn with_max_mipmap_levels",
        "crates/bbb-native/src/terrain_runtime/textures/tests.rs#terrain_generation_retains_the_selected_mipmap_limit_for_reload",
        "docs/unsupported-features.md",
    ),
    "resources-server-pack-lifecycle": (
        "crates/bbb-pack/src/server_pack.rs#prepares_archive_provider_without_eager_extraction",
        "crates/bbb-pack/src/selected_packs.rs#options_zip_is_indexed_directly_and_materializes_only_consumed_entries",
        "crates/bbb-net/src/resource_pack.rs#server_pack_download_reports_vanilla_progress_order_for_network_and_cache",
        "crates/bbb-net/src/resource_pack.rs#server_pack_download_verifies_hash_headers_and_reuses_cache",
        "crates/bbb-net/src/resource_pack.rs#server_pack_cache_initialization_vacuums_old_duplicates_and_preserves_log",
        "crates/bbb-net/src/resource_pack.rs#server_pack_download_log_records_vanilla_file_and_error_shapes",
        "crates/bbb-net/src/resource_pack.rs#requested_hash_mismatch_preserves_vanilla_target_for_next_validation",
        "crates/bbb-net/src/resource_pack.rs#requested_hash_target_race_preserves_vanilla_non_truncating_create",
        "crates/bbb-net/src/resource_pack.rs#server_pack_download_enforces_stream_limit_without_content_length",
        "crates/bbb-net/src/resource_pack.rs#server_pack_download_removes_bad_requested_cache_before_network_failure",
        "crates/bbb-net/src/resource_pack.rs#unhashed_download_touches_an_existing_content_addressed_file",
        "crates/bbb-net/src/resource_pack.rs#matching_requested_cache_bypasses_the_network_download_size_limit",
        "crates/bbb-net/src/resource_pack.rs#server_pack_download_uses_vanilla_socks_proxy_and_redacts_credentials",
        "crates/bbb-net/src/resource_pack.rs#vanilla_proxy_arguments_fall_back_and_require_both_credentials",
        "crates/bbb-native/src/startup.rs#args_project_vanilla_socks_proxy_without_exposing_password",
        "crates/bbb-native/src/resource_pack_runtime.rs#default_server_pack_cache_uses_vanilla_game_directory_downloads",
        "crates/bbb-native/src/resource_pack_runtime.rs#server_pack_runtime_initializes_the_exact_vanilla_download_queue_root",
        "crates/bbb-native/src/resource_pack_runtime.rs#failed_runtime_download_writes_vanilla_download_queue_event",
        "crates/bbb-net/src/event_stream.rs#configuration_resource_pack_push_emits_push_and_response_events",
        "crates/bbb-net/src/event_stream.rs#play_resource_pack_push_emits_push_and_response_events",
        "crates/bbb-world/src/client/server_presentation.rs#resource_pack_push_stores_and_upserts_by_id",
        "crates/bbb-world/src/client/server_presentation.rs#resource_pack_pop_without_id_clears_all_packs",
        "crates/bbb-native/src/resource_pack_runtime.rs#accepted_packs_are_tracked_in_push_order",
        "crates/bbb-native/src/resource_pack_runtime.rs#download_progress_preserves_batch_counts_and_terminal_failure",
        "crates/bbb-native/src/resource_pack_runtime.rs#download_progress_ignores_a_pack_from_another_sequential_batch",
        "crates/bbb-native/src/resource_pack_runtime.rs#accepted_pack_runs_download_apply_pop_restore_lifecycle",
        "crates/bbb-native/src/resource_pack_runtime.rs#one_download_failure_discards_every_other_inactive_pack_across_queued_batches",
        "crates/bbb-native/src/resource_pack_runtime.rs#one_preparation_failure_fails_every_new_pack_across_queued_batches",
        "crates/bbb-native/src/resource_pack_runtime.rs#recovery_reload_runs_once_and_second_failure_does_not_hot_loop",
        "crates/bbb-native/src/resource_pack_runtime.rs#transfer_keeps_active_pack_while_terminal_disconnect_unloads_it",
        "crates/bbb-native/src/resource_pack_runtime.rs#dropping_runtime_removes_materialized_session_directory",
        "crates/bbb-native/src/runtime/tests.rs#server_pack_download_progress_projects_vanilla_system_toast_and_forced_hide",
        "crates/bbb-renderer/src/hud.rs#system_toast_sanitizes_text_and_visible_portion",
        "crates/bbb-renderer/src/hud.rs#system_toast_instances_use_distinct_vanilla_slots",
        "scripts/playable_gate.py",
        "scripts/tests/test_client_mechanism_inventory.py#test_server_pack_owner_census_has_complete_evidence_and_scope_rulings",
    ),
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
    "hud-frame-extract-order": ("crates/bbb-native/src/runtime/render_extract.rs",),
    "hud-player-vitals": ("crates/bbb-native/src/runtime.rs",),
    "hud-hotbar-selection": ("crates/bbb-native/src/runtime.rs",),
    "hud-overlay-effects": ("crates/bbb-native/src/runtime.rs",),
    "hud-text-and-lists": ("crates/bbb-native/src/runtime.rs",),
    "rendering-frame-order": (
        "crates/bbb-renderer/src/render.rs#FRAME_STEPS",
        "crates/bbb-native/src/runtime.rs",
    ),
    "rendering-extract-surface": (
        "crates/bbb-native/src/runtime/render_extract.rs",
        "crates/bbb-native/src/runtime.rs",
    ),
    "rendering-level-pass": (
        "crates/bbb-renderer/src/render.rs",
        "crates/bbb-native/src/item_models/first_person.rs",
    ),
    "rendering-effects-screenshot": (
        "crates/bbb-renderer/src/camera_post_effect.rs",
        "crates/bbb-native/src/screen_effects.rs",
    ),
    "rendering-camera-fluid-fog": (
        "crates/bbb-native/src/camera_fov.rs",
        "crates/bbb-native/src/runtime.rs",
        "crates/bbb-native/src/runtime/render_extract.rs",
        "crates/bbb-renderer/src/camera.rs",
        "crates/bbb-native/src/runtime/tests.rs#camera_fluid_sampler_uses_water_priority_viewport_base_fov_and_inclusive_lava_surface",
        "crates/bbb-native/src/runtime/tests.rs#camera_powder_snow_near_plane_matches_vanilla_sample_order_without_fov_scaling",
        "crates/bbb-native/src/runtime/tests.rs#camera_fluid_sample_is_shared_by_same_frame_fog_and_fov_consumers",
        "crates/bbb-native/src/runtime/tests.rs#fog_environment_uses_water_fog_distances_when_eye_is_in_water",
        "crates/bbb-native/src/runtime/tests.rs#lava_fog_uses_normal_fire_resistance_and_spectator_distance_precedence",
        "crates/bbb-native/src/runtime/tests.rs#powder_snow_fog_precedes_mob_effect_distances_preserves_color_and_suppresses_sky",
        "crates/bbb-native/src/runtime/tests.rs#blindness_takes_fog_precedence_over_darkness_and_blocks_sky",
        "crates/bbb-native/src/runtime/tests.rs#night_vision_brightens_atmospheric_fog_unless_darkness_is_present",
    ),
    "rendering-chunk-layer-routing": (
        "crates/bbb-renderer/src/terrain.rs#build_terrain_chunk_section_subset_mesh_layers_with_atlas_and_camera",
        "crates/bbb-renderer/src/terrain/mesh.rs#build_chunk_section_subset_mesh_layers_with_lookup",
        "crates/bbb-renderer/src/terrain/mesh.rs#sort_translucent_quads_by_distance",
        "crates/bbb-renderer/src/renderer.rs#TerrainTranslucentSortState",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#layer_builder_splits_opaque_and_cutout_meshes",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#forced_translucent_cube_faces_emit_in_translucent_layer",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#transparent_cube_faces_emit_in_cutout_layer",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#section_layer_builder_reuses_texture_layer_routing_without_changing_meshes",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#translucent_layer_sorts_quad_indices_by_distance_from_camera_like_vanilla",
        "crates/bbb-renderer/src/terrain/mesh/tests.rs#translucent_layer_sort_uses_world_camera_equivalent_to_vanilla_section_local_sorting",
        "crates/bbb-renderer/src/renderer.rs#translucent_sort_state_rebuilds_vanilla_indices_after_camera_move",
        "crates/bbb-renderer/src/renderer.rs#translucent_sort_state_uses_vanilla_block_and_point_of_view_triggers",
    ),
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
    "resources-language-manager": (
        "crates/bbb-pack/src/language.rs",
        "crates/bbb-pack/src/metadata.rs",
        "crates/bbb-native/src/simulation/input_commands.rs",
    ),
    "resources-window-icons": (
        "crates/bbb-pack/src/window_icons.rs#pub enum WindowIconCatalog",
        "crates/bbb-native/src/window_icon.rs#pub(crate) fn install_startup_window_icon",
        "crates/bbb-native/src/window_icon.rs#fn apply_x11_icon_property",
        WINDOW_ICON_OWNER_CENSUS_TEST,
        "docs/wiki/mechanisms/resources/offline-asset-trust.md",
    ),
    "ui-bidirectional-shaping": (
        "crates/bbb-render-types/src/text_visual_order.rs",
    ),
    "ui-font-manager": ("crates/bbb-item-model/src/font.rs",),
    "audio-sound-manager-engine": (
        "crates/bbb-pack/src/sounds.rs",
        "crates/bbb-pack/src/openal_native.rs",
        "crates/bbb-audio/src/device.rs",
        "crates/bbb-audio/src/openal.rs",
        "crates/bbb-audio/src/runtime.rs",
        "crates/bbb-native/src/audio_runtime.rs",
    ),
    "input-keyboard-routing": ("crates/bbb-native/src/input.rs",),
    "input-debug-key-actions": ("crates/bbb-native/src/input.rs",),
    "input-mouse-routing": ("crates/bbb-native/src/input/mouse.rs",),
    "input-key-mapping": ("crates/bbb-native/src/input.rs",),
    "platform-window-lifecycle": ("crates/bbb-native/src/platform.rs",),
    "platform-window-display-mode": ("crates/bbb-native/src/main.rs",),
    "platform-window-surface-state": ("crates/bbb-native/src/main.rs",),
    "platform-window-chrome": ("crates/bbb-native/src/window_icon.rs",),
    "ui-clipboard-surface": ("crates/bbb-native/src/input/text_edit.rs",),
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
    "persistence-options-file": ("crates/bbb-native/src/client_options.rs",),
    "persistence-options-derived": ("crates/bbb-native/src/client_options.rs",),
    "persistence-options-apply": (
        "crates/bbb-native/src/client_options.rs",
        "crates/bbb-native/src/startup.rs",
        "crates/bbb-native/src/terrain_runtime.rs",
        "crates/bbb-native/src/terrain_runtime/textures.rs",
        "crates/bbb-native/src/entity_scene.rs",
        "crates/bbb-native/src/runtime/render_extract.rs",
        "crates/bbb-renderer/src/renderer.rs",
        "crates/bbb-pack/src/selected_packs.rs",
    ),
    "persistence-options-unconsumed": (
        "crates/bbb-native/src/client_options/datafix.rs",
        "crates/bbb-native/src/client_options/datafix_26_1.rs",
        "docs/evidence/vanilla-26.1/client-options-catalog.json",
    ),
    "persistence-server-list": ("crates/bbb-native/src/server_list.rs",),
    "identity-profile-key-lifecycle": ("docs/unsupported-features.md",),
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
    "scope-multiplayer-server-list": ("docs/unsupported-features.md",),
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
        evidence: OwnerEvidence | OwnerOpenGap | None = None,
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
        if isinstance(evidence, OwnerOpenGap):
            owner["bbb_evidence"] = {
                "open_gap": {
                    "ledger": evidence.ledger,
                    "artifact": evidence.artifact,
                    "reproduce_command": evidence.reproduce_command,
                    "reason": evidence.reason,
                    "close_condition": evidence.close_condition,
                }
            }
        elif evidence is not None:
            owner["bbb_evidence"] = {
                "implementation": list(evidence.implementation),
                "tests": list(evidence.tests),
            }
            if evidence.scope_ruling is not None:
                for anchor in evidence.scope_ruling.vanilla_basis:
                    basis_path, separator, basis_lines = anchor.partition(":")
                    if (
                        not separator
                        or re.fullmatch(
                            r"\d+(?:-\d+)?(?:,\d+(?:-\d+)?)*", basis_lines
                        )
                        is None
                    ):
                        raise ClientMechanismInventoryError(
                            f"invalid Vanilla scope-ruling anchor: {anchor}"
                        )
                    self.source(basis_path)
                owner["bbb_evidence"]["scope_ruling"] = {
                    "conclusion": evidence.scope_ruling.conclusion,
                    "vanilla_basis": list(evidence.scope_ruling.vanilla_basis),
                    "current_scope_reason": evidence.scope_ruling.current_scope_reason,
                    "reopen_condition": evidence.scope_ruling.reopen_condition,
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
        methods = sorted(source.methods, key=lambda item: item.span.start_byte)
        if spec.selected_methods:
            counts = Counter(method.name for method in methods)
            invalid = {
                name: counts[name]
                for name in spec.selected_methods
                if counts[name] != 1
            }
            if invalid:
                raise ClientMechanismInventoryError(
                    f"{spec.source_path}: selected method census changed: {invalid}"
                )
            selected = set(spec.selected_methods)
            methods = [method for method in methods if method.name in selected]
        for method in methods:
            route = self.method_route(source, spec, method)
            signature = self.method_signature(method)
            evidence = None
            if route.mechanism_id in OWNER_EVIDENCE_MECHANISMS:
                source_evidence = METHOD_OWNER_EVIDENCE_BY_SOURCE.get(spec.source_path)
                evidence = (
                    source_evidence.get(method.name)
                    if source_evidence is not None
                    else METHOD_OWNER_EVIDENCE[route.mechanism_id].get(method.name)
                )
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
        elif spec.source_path == "net/minecraft/client/resources/ClientPackSource.java":
            route = CLIENT_PACK_SOURCE_METHOD_ROUTES.get(method.name)
            if route is not None:
                return route
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
        if spec.source_path == "net/minecraft/client/gui/Gui.java":
            route = GUI_METHOD_ROUTES.get(method.name)
            if route is not None:
                return route
            if InventoryBuilder.is_direct_support_accessor(source, method):
                return DIRECT_SUPPORT_ROUTE
            signature = InventoryBuilder.method_signature(method)
            raise ClientMechanismInventoryError(
                f"Gui method lacks a decision/support route: {signature}"
            )
        if spec.source_path == "net/minecraft/client/renderer/GameRenderer.java":
            route = GAME_RENDERER_METHOD_ROUTES.get(method.name)
            if route is not None:
                return route
            if InventoryBuilder.is_direct_support_accessor(source, method):
                return DIRECT_SUPPORT_ROUTE
            signature = InventoryBuilder.method_signature(method)
            raise ClientMechanismInventoryError(
                f"GameRenderer method lacks a decision/support route: {signature}"
            )
        if spec.source_path == "net/minecraft/client/KeyboardHandler.java":
            route = KEYBOARD_HANDLER_METHOD_ROUTES.get(method.name)
            if route is None:
                signature = InventoryBuilder.method_signature(method)
                raise ClientMechanismInventoryError(
                    f"KeyboardHandler method lacks a homogeneous route: {signature}"
                )
            return route
        if spec.source_path == "com/mojang/blaze3d/platform/ClipboardManager.java":
            route = CLIPBOARD_MANAGER_METHOD_ROUTES.get(method.name)
            if route is None:
                signature = InventoryBuilder.method_signature(method)
                raise ClientMechanismInventoryError(
                    f"ClipboardManager method lacks a homogeneous route: {signature}"
                )
            return route
        if spec.source_path == "com/mojang/blaze3d/platform/Window.java":
            route = WINDOW_METHOD_ROUTES.get(method.name)
            if route is not None:
                return route
            if InventoryBuilder.is_direct_support_accessor(source, method):
                return DIRECT_SUPPORT_ROUTE
            signature = InventoryBuilder.method_signature(method)
            raise ClientMechanismInventoryError(
                f"Window method lacks a decision/support route: {signature}"
            )
        if spec.source_path in {
            "net/minecraft/client/MouseHandler.java",
            "net/minecraft/client/KeyMapping.java",
        }:
            if InventoryBuilder.is_direct_support_accessor(source, method):
                return DIRECT_SUPPORT_ROUTE
            return MethodRoute(spec.category, spec.domain, spec.mechanism_id)
        if spec.source_path == "net/minecraft/client/multiplayer/ServerList.java":
            route = SERVER_LIST_METHOD_ROUTES.get(method.name)
            if route is None:
                signature = InventoryBuilder.method_signature(method)
                raise ClientMechanismInventoryError(
                    f"ServerList method lacks a homogeneous route: {signature}"
                )
            return route
        if (
            spec.source_path
            == "net/minecraft/client/resources/language/ClientLanguage.java"
            and method.name == "getVisualOrder"
        ):
            return MethodRoute(
                "resource-consumer",
                "text",
                "ui-bidirectional-shaping",
            )
        if spec.source_path == "net/minecraft/util/HttpUtil.java" and method.name in {
            "getAvailablePort",
            "isPortAvailable",
        }:
            return MethodRoute(
                "out-of-scope",
                "singleplayer",
                "scope-singleplayer-integrated",
                "out-of-scope",
                "out-of-scope",
            )
        if spec.source_path == "net/minecraft/client/main/Main.java":
            route = MAIN_METHOD_ROUTES.get(method.name)
            if route is None:
                signature = InventoryBuilder.method_signature(method)
                raise ClientMechanismInventoryError(
                    f"Main method lacks a homogeneous route: {signature}"
                )
            return route
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
        owner_gap_by_mechanism = Counter(
            str(item["mechanism_id"])
            for item in self.owners
            if isinstance(item.get("bbb_evidence"), dict)
            and "open_gap" in item["bbb_evidence"]
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
                    **(
                        {"owner_gap_count": owner_gap_by_mechanism[mechanism_id]}
                        if owner_gap_by_mechanism[mechanism_id]
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
