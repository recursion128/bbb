# 客户端功能差异

本账本仅登记 `bbb` 客户端与 Minecraft Java 26.1 之间的未关闭功能缺口（active / 带 `Reopen when ...` 的 deferred）以及 `# Not needed` 范围裁决；不保留完成历史，功能落地时直接删除对应条目，历史由 Git 维护。

维护规则见[文档工作流](wiki/development/documentation-workflow.md#ledger-与接手信息)。

# Todo

以下只登记主分支生产路径中玩家可观察的功能缺口，验收按 [AGENTS.md](../AGENTS.md) 的推进原则。

# Not needed

以下保留 Vanilla 26.1 行为依据、产品范围裁决及重开条件。

## Official player reporting

- **Official player reporting is `not-needed`.** The client intentionally neither
  integrates Mojang's player-reporting service nor keeps a local mock surface,
  covering chat/skin/name report entry points, reason and message selection,
  comments, drafts and exit confirmation, evidence building, asynchronous
  submission, result screens, dedicated report assets, and control projections
  (`ReportPlayerScreen`, `ReportingContext`). Inbound message-signature
  validation, last-seen acknowledgements, chat trust tags, social hide/mute, and
  recent-player history remain supported independently. Reopen only if full
  official player-reporting service integration enters the declared product scope.

## Session, identity, and telemetry

- **Online mode (authenticated Services and encrypted transport) is
  `not-needed`.** The declared client scope is offline-only. A server that opens
  the vanilla encryption handshake fails the connection with an attributable
  "online-mode servers are unsupported" error before any cipher could be
  negotiated. Remote players' signed chat and announced sessions are still
  decoded and validated with the public Services key set, and signable command
  arguments keep the vanilla signed-command packet shape with an empty
  signature list — matching a vanilla client that has no key pair. Vanilla
  `Main --uuid` is distinct from those online credentials: it selects the offline
  launch identity, so BBB retains that exact CLI surface with Authlib/JDK lenient
  parsing and username-derived fallback, feeding login, canonical local-player
  state, and server-pack headers. The offline service boundary follows vanilla's
  `UserApiService.OFFLINE_PROPERTIES` semantics (empty privilege flags and ban
  scopes), so profile-flag chat restrictions stay inert just like vanilla's
  offline fallback. Reopen only if the offline-only scope decision is reversed
  and encrypted login, session join, and the profile-key lifecycle are brought
  into scope.
- **Mojang client telemetry callbacks are `not-needed`.** `bbb` has no external
  telemetry sender. Vanilla `WorldSessionTelemetryManager.onServerBrandReceived`
  only stores the brand on `WorldLoadEvent` and attempts a Mojang telemetry send;
  server-brand protocol state and local diagnostics remain independently owned.
  Reopen if the offline external-client scope gains a telemetry sender.

## Platform release gates

- **An X11 release row is `not-needed`.** The declared Linux release row is
  Wayland. `bbb` keeps winit's X11 branch as a reachable implementation detail
  but does not issue an X11 platform receipt. Reopen only if the scope decision
  is reversed.
- **The macOS aarch64 release receipt (`platform-macos-release`) is
  `not-needed`.** Client delivery covers Linux x86_64 and Windows x86_64. The
  client still builds for macOS and `scripts/cross_check.py` keeps compiling that
  target, but no maintained macOS runner exists, so the Cocoa/Metal/Quartz/CoreAudio
  host-native receipt is out of delivery scope. Reopen only if that ruling is
  reversed and a maintained macOS aarch64 runner can earn the receipt.
- **The Windows x86_64 automated host receipt (`platform-windows-release`) is
  `not-needed`.** Windows x86_64 / DX12 native client builds, startup, software-GPU
  WARP E2E (`cargo dev e2e client --software-gpu`), and `cargo dev` workflows are
  supported; `scripts/cross_check.py` defaults to `aarch64-apple-darwin` on non-macOS
  hosts (and selects installed `x86_64-unknown-linux-gnu` or `x86_64-pc-windows-msvc`
  on macOS hosts, or accepts explicit `--target x86_64-pc-windows-msvc` when installed).
  Because `scripts/platform-acceptance-gate.sh` only drives Unix hosts and no maintained
  automated Windows physical-GUI runner exists, the Win32/DX12/IME/WASAPI host-native
  receipt is out of automated release-gate scope. Reopen only if that ruling is reversed
  and a maintained Windows x86_64 physical-GUI runner is added to earn the receipt.

## Input, movement, menus, and interaction

- **Touchscreen split-stack and snapback animation are `not-needed`.** Vanilla
  gates both behind `Options.touchscreen` (`AbstractContainerScreen.java:336,
  342, 411, 489`; snapback interpolation `:146-158`) and `bbb` has no touch input
  mode.

## Rendering, HUD, debug overlays, and configuration

- **Auto world-icon screenshots (`tryTakeScreenshotIfNeeded` /
  `takeAutoScreenshot`) are `not-needed`.** Both gate on
  `Minecraft.isLocalServer()` and the integrated server's
  `getWorldScreenshotFile()`, so a multiplayer-only external client never reaches
  the 1000 ms retry latch, rendered-sections readiness gate, or 64x64 world-icon
  write. Manual F2 screenshots keep their own owners. Reopen if `bbb` grows an
  integrated-server surface.
- **`setRenderBlockOutline` is `not-needed`.** Its only vanilla callers disable
  and restore the outline around `Minecraft.grabHugeScreenshot`'s 4096-wide
  panoramic capture, and `bbb` has no panoramic/huge-screenshot mode (keeping the
  live `LevelFrameRoute` panoramic flag hard-false). Reopen with any panoramic
  capture surface.
- **`visualize_chunks_on_server` is `not-needed`.** Vanilla 26.1
  `ChunkDebugRenderer.emitGizmos` refreshes data only when
  `Minecraft.getSingleplayerServer()` returns an `IntegratedServer` and reads
  `ServerLevel` / `ServerChunkCache` on that server's executor; an external
  multiplayer client has no integrated server, so vanilla-equivalent behavior is
  empty. Reopen if the repo gains an in-process integrated server boundary.
- **Entity-hitbox local-server green boxes and delta arrows are `not-needed`.**
  Vanilla 26.1 `EntityHitboxDebugRenderer.getServerEntity` resolves an entity
  only through `Minecraft.getSingleplayerServer()` -> `IntegratedServer` ->
  `ServerLevel.getEntity`; an external multiplayer client always receives `null`
  and emits the red `Missing Server Entity` 3D billboard implemented by `bbb`.
  Reopen with an in-process integrated server boundary.
- **Client-only debug entries with no external-client HUD line are `not-needed`.**
  `LocalDifficulty`, `EntitySpawnCounts`, and `ChunkGenerationStats` read an
  integrated local-server mirror (`ServerLevel`, `NaturalSpawner.SpawnState`,
  `ChunkGenerator` / `RandomState` / `BiomeSource`) that an external client does
  not own.
- **Exact golden pixel parity to vanilla screenshots is `not-needed`.** Driver
  and floating-point raster jitter make full-frame pixel hashes across independent
  GPU stacks unreliable. Reopen only if a same-backend, same-driver deterministic
  raster contract becomes available.
- **`F3+F4` `GameModeSwitcherScreen` mouse-release modifier path is
  `not-needed`.** Vanilla only uses `mouseReleased` for
  `keyDebugModifier.matchesMouse(event)` (`KeyMapping.matchesMouse` is true only
  for a `MOUSE`-type mapping), whereas `bbb`'s debug modifier is the hard-coded
  `F3` keyboard path.
- **In-game options UI and UI-driven runtime option mutators are `not-needed`.**
  Configuration stays at command-line / URL-query startup (for example
  `--advanced-item-tooltips`) alongside Vanilla `options.txt` (and browser
  `localStorage["bbb.options.txt"]`) load/migration and hotkey saves (`F11`,
  `Ctrl+B`, `F3+H`, `F3+P`); `bbb` does not ship an in-game configuration UI.
  This also covers `Minecraft.updateMaxMipLevel`, whose only Vanilla 26.1 caller
  is `VideoSettingsScreen` while `bbb` uses the Vanilla default mip level 4.
  Reopen if a CLI or supported runtime surface changes mipmap levels after startup
  (rebuilding the complete resource/GPU generation atomically).
- **World-access/delete, pack-copy, and integrated chunk-storage `SystemToast`
  helpers are `not-needed`.** `SystemToast.onWorldAccessFailure`,
  `onWorldDeleteFailure`, and `onPackCopyFailure` are called only from
  title/world-selection or in-game pack selection, and
  `onChunkLoadFailure`/`onChunkSaveFailure` are called only by `IntegratedServer`
  (`SystemToast.java:132-188`, `IntegratedServer.java:352-363`). Reopen the first
  group with a supported world/pack-selection UI, and the second with an
  integrated server.
- **Vanilla OpenGL GPU warnlist reload listener is `not-needed`.**
  `GpuWarnlistManager.Preparations.apply` returns warnings only when
  `GpuDevice.getBackendName()` is exactly `OpenGL`, feeding the
  improved-transparency branch in `Options`/`VideoSettingsScreen` and an optional
  copy in `Minecraft.fillSystemReport`. `bbb` ships `wgpu` Vulkan/Metal/DX12/WebGPU
  backends and has no in-game options UI, so the Vanilla predicate is always
  empty. Reopen only if an OpenGL backend and improved-transparency warning enter
  scope.

## Resource packs, screens, and static registries

- **Server-list editing UI owners are `not-needed`.**
  `ServerList.size`/`swap`/`replace`/`remove`/`unhide` serve only
  `ServerSelectionList` and `JoinMultiplayerScreen` (`replace` has no 26.1 caller
  at all), whereas CLI / Web `QuickPlay.joinMultiplayerWorld` uses only
  `load`/`get(String)`/`add`/`save`, owned by `bbb_client::server_list` (persisted
  to `servers.dat` on desktop and `localStorage["bbb.servers.dat"]` on Web). Reopen
  with a server-selection screen.
- **Title splash reload listener is `not-needed`.** Vanilla
  `SplashManager.prepare` reads `minecraft:texts/splashes.txt` exclusively for
  `TitleScreen.init` (`Minecraft.getSplashManager().getSplash()`), and the
  CLI-direct client has no title screen. Reopen when a title screen enters scope.
- **Indexed built-in resource-pack ZIPs (`programmer_art`, `high_contrast`) are
  `not-needed`.** `ClientPackSource` exposes them through pack-repository/options
  UI selection; `bbb` accepts explicit startup/server packs (plus Web `?pack=`,
  drag-and-drop, and `#bbb-pack-picker` `.zip` mounting) and has no built-in
  pack-id selection UI or CLI flag. Reopen when a built-in pack-id selection
  surface is added.
- **Indexed Realms language and carousel assets are `not-needed`.** Vanilla's
  shared language manager sees `realms:lang/en_us` and `AddRealmPopupScreen`
  enumerates `realms:textures/gui/images`, while Realms service and screens lie
  outside the external multiplayer client boundary. Reopen if Realms enters scope.
- **Server/world-list favicon and pack-selection dynamic textures are
  `not-needed`.** `bbb` configures startup/server/Web packs without those
  selection screens, while owning map textures, profile skin/cape/elytra, and
  synchronized pack-backed presentation. Reopen if one of those screens enters
  scope.
- **Synchronized custom block or entity-type registry identities are
  `not-needed`.** Vanilla 26.1 `RegistryDataLoader.SYNCHRONIZED_REGISTRIES`
  contains neither `BLOCK` nor `ENTITY_TYPE`; their protocol identities remain
  fixed and resource packs cannot add entries. Reopen if the target protocol
  synchronizes either registry.
- **Custom/datapack attribute tooltip definitions are `not-needed`.** Vanilla
  26.1 registers `minecraft:attribute` via `BuiltInRegistries.registerSimple`
  outside `WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`; the fixed 35-entry
  vanilla catalog is generated. Reopen if attributes become network-synchronized.
- **Custom/datapack mob-effect definitions are `not-needed`.** Vanilla 26.1
  registers `minecraft:mob_effect` via `BuiltInRegistries.registerSimple` outside
  `WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`; the fixed 40-entry catalog
  is generated. Reopen if mob effects become network-synchronized.
- **Custom/datapack potion registry definitions are `not-needed`.** Vanilla 26.1
  registers `minecraft:potion` via `BuiltInRegistries.registerSimple`
  (`Potions::bootstrap`) outside `WORLDGEN_REGISTRIES` and
  `SYNCHRONIZED_REGISTRIES`. Reopen if potions become network-synchronized.
- **Custom/datapack villager-type and villager-profession definitions are
  `not-needed`.** Both defaulted registries live in `BuiltInRegistries`
  (`VillagerType::bootstrap`, `VillagerProfession::bootstrap`) outside
  `WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`, and the fixed 7-entry and
  15-entry catalogs are generated. Reopen if either registry becomes
  network-synchronized.
- **Custom/datapack decorated-pot pattern definitions are `not-needed`.** Vanilla
  26.1 registers `minecraft:decorated_pot_pattern` via
  `BuiltInRegistries.registerSimple` (`DecoratedPotPatterns::bootstrap`) outside
  `WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`. Reopen if pot patterns
  become network-synchronized.
- **Custom/datapack menu-type definitions are `not-needed`.** Vanilla 26.1
  registers `minecraft:menu` via `BuiltInRegistries.registerSimple` outside
  `WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`; the fixed 25-entry menu
  catalog is generated. Reopen if menu types become network-synchronized.
- **Custom difficulty, equipment-slot/slot-group, entity-pose, and sound-source
  enum values are `not-needed`.** Vanilla 26.1 defines `Difficulty` (4 entries),
  `EquipmentSlot` / `EquipmentSlotGroup`, `Pose` (18 entries), and `SoundSource`
  (11 entries) as closed Java enums with generated fixed catalogs and no
  server/datapack extension surface.
- **Custom entity-data serializers and client game-event packet types are
  `not-needed`.** Vanilla 26.1 assigns the 43 `EntityDataSerializers` entries and
  14 `ClientboundGameEventPacket.Type` byte ids statically in code with no
  synchronized registry surface.
- **Custom item, block, or block-state registry definitions are `not-needed`.**
  Vanilla 26.1 item protocol ids (1506 entries) and block / global block-state
  ids (29,873 states) come from fixed built-in client registration rather than
  synchronized server/datapack registries; modded clients that alter built-in
  registration order are outside the Vanilla 26.1 wire target.