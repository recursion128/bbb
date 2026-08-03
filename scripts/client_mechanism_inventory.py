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
        MethodRoute("interaction", "interaction", "interaction-game-mode-decisions"),
        (
            "continueAttack",
            "startAttack",
            "startUseItem",
            "canSwitchGameMode",
            "pickBlockOrEntity",
            "pick",
        ),
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

MINECRAFT_SUPPORT_ROUTE = MethodRoute(
    "support-accessor",
    "support",
    "scope-support-accessors",
    "out-of-scope",
    "support-non-decision",
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
    MethodSurface("movement", "movement", "world-local-player-decisions", "net/minecraft/client/player/LocalPlayer.java"),
    MethodSurface("interaction", "interaction", "interaction-game-mode-decisions", "net/minecraft/client/multiplayer/MultiPlayerGameMode.java"),
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
    MethodSurface("persistence", "persistence", "persistence-client-files", "net/minecraft/client/Options.java"),
    MethodSurface("persistence", "persistence", "persistence-client-files", "net/minecraft/client/multiplayer/ServerList.java"),
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
    "protocol-packet-tables": ("docs/packet-coverage-26.1.json",),
    "protocol-listener-contracts": ("docs/packet-coverage-26.1.json",),
    "protocol-login-handler-decisions": ("crates/bbb-net/src/event_stream.rs",),
    "protocol-configuration-handler-decisions": ("crates/bbb-net/src/event_stream.rs",),
    "protocol-play-handler-decisions": ("crates/bbb-world/src/play_apply.rs", "docs/packet-coverage-26.1.json"),
    "registries-built-in-catalogs": ("docs/registry-holder-consumer-coverage-26.1.json",),
    "registries-synchronized-data": ("docs/registry-holder-consumer-coverage-26.1.json",),
    "world-client-level-decisions": ("crates/bbb-world/src/lib.rs",),
    "world-local-player-decisions": ("crates/bbb-native/src/input/movement.rs",),
    "interaction-game-mode-decisions": ("crates/bbb-native/src/input.rs",),
    "entities-fixed-type-surface": ("docs/entity-coverage-26.1.json",),
    "rendering-entity-renderer-registration": ("docs/entity-coverage-26.1.json",),
    "rendering-entity-layer-registration": ("docs/entity-coverage-26.1.json",),
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
        "crates/bbb-native/src/runtime.rs#block_destroy_stages_from_world",
        "crates/bbb-renderer/src/block_destroy.rs",
    ),
    "rendering-level-diagnostics-gizmos": (
        "crates/bbb-native/src/terrain_runtime/occlusion/debug.rs",
        "crates/bbb-renderer/src/debug_filled_boxes.rs",
    ),
    "rendering-level-light-query": (
        "crates/bbb-world/src/chunks/light.rs",
        "crates/bbb-native/src/runtime/render_extract.rs#particle_light_for_world",
    ),
    "resources-language-manager": ("crates/bbb-pack/src/language.rs",),
    "ui-font-manager": ("crates/bbb-item-model/src/font.rs",),
    "audio-sound-manager-engine": ("crates/bbb-pack/src/sounds.rs", "crates/bbb-audio/src/runtime.rs"),
    "input-keyboard-routing": ("crates/bbb-native/src/input.rs",),
    "input-mouse-routing": ("crates/bbb-native/src/input/mouse.rs",),
    "input-key-mapping": ("crates/bbb-native/src/input.rs",),
    "platform-window-lifecycle": ("crates/bbb-native/src/platform.rs",),
    "protocol-serverbound-call-sites": ("crates/bbb-net/src/event_stream.rs", "docs/packet-coverage-26.1.json"),
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
    "persistence-client-files": ("crates/bbb-native/src/server_list.rs",),
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
            )

    @staticmethod
    def method_route(
        source: JavaSource,
        spec: MethodSurface,
        method: JavaMethod,
    ) -> MethodRoute:
        if spec.source_path == "net/minecraft/client/renderer/LevelRenderer.java":
            route = LEVEL_RENDERER_METHOD_ROUTES.get(method.name)
            if route is None:
                signature = InventoryBuilder.method_signature(method)
                raise ClientMechanismInventoryError(
                    f"LevelRenderer method lacks a homogeneous route: {signature}"
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
            return MINECRAFT_SUPPORT_ROUTE
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
        for ordinal, call in enumerate(calls):
            detail = ",".join(argument.canonical for argument in call.arguments)
            owner = call.owner or source.package
            self.add_owner(
                owner_id=f"registration:{category}:{owner}#{call.name}:{ordinal}",
                category=category,
                domain=domain,
                mechanism_id=mechanism_id,
                vanilla_owner=f"{owner}#{call.name}({detail})",
                path=path,
                line=call.span.start_point[0] + 1,
                kind="registration-call",
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
                }
                for mechanism_id in mechanisms
            },
            "owners": self.owners,
            "schema_version": 1,
            "selection_contract": {
                "java_frontend": "scripts/vanilla_java_source.py",
                "level_renderer_method_partition": "explicit homogeneous routes; unclassified methods fail closed",
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
