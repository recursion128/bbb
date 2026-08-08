# Feature Gap TODO List

What is still missing between `bbb` and Minecraft Java 26.1. Nothing else.

This file holds no completion history. When a slice lands, delete the todo it
closed — never rewrite it into a "done" note. Completed work is recorded by the
code, its tests, `docs/goal-archive.md`, and `git log`.

## Update Rules

- Every entry states what is still missing, positively. Never define remaining
  work by exclusion ("X beyond A, B, C"): the exclusion list grows without bound
  and hides the actual gap. If you need to know what already exists, read the
  code.
- The slice that closes a todo deletes that todo in the same commit.
- Never add an entry describing what was built.
- Every todo carries an owner crate and, where the behavior is vanilla-derived,
  the vanilla 26.1 anchor (class / method / constant) it must follow.
- File and heading order are an ownership inventory, not an execution schedule.
  Ready todos join the global hardest-ready-first selection in `goal.md`; blocked,
  deferred, or prerequisite-incomplete work does not enter that candidate set.
- A ready todo may become an atomic slice or a multi-stage engineering slice. Breadth,
  cross-crate ownership, implementation size, and multiple normal commits are not
  exclusion reasons when the semantic boundary, final completion criteria, phase
  validation, and failure-localizing checkpoints are explicit. Excluding a large
  target requires a real blocker under `goal.md`, not merely "scope too large".
- Two states are not todos, and deleting one silently re-opens a decision that
  was already made:
  - **Deferred** — a real gap, intentionally not scheduled. It stays under its
    owning `# Todo` domain, uses the literal `deferred` marker, and carries an
    explicit `Reopen when ...` condition.
  - **Not needed** — vanilla 26.1 has no such behavior, or bbb has no surface
    for it. Carries the vanilla evidence.
- This file does not overflow into sibling files. If it grows, the gap list grew.

---

# Todo

## Architecture drift candidates

Owner: workspace production crates; census evidence owned by `scripts/docs_wiki_lint.py`

<!-- architecture-candidates:start -->
<!-- architecture-candidates:end -->

## Machine-readable parity acceptance closure

Owner: `scripts` + the matrix-declared BBB owners

Matrix routes: `startup-client-bootstrap`, `startup-session-identity`, `resources-offline-asset-trust`, `resources-indexed-vanilla-provider`, `resources-reload-listeners`, `resources-reload-recovery`, `resources-server-pack-lifecycle`, `resources-language-manager`, `resources-window-icons`, `resources-indexed-panorama`, `protocol-login-handler-decisions`, `protocol-client-state-flow`
Matrix routes: `world-client-level-lifecycle`, `world-client-clock-weather`, `world-client-chunk-light-lifecycle`, `world-client-entity-lifecycle`, `world-client-ambient-animation`, `world-client-session-services`, `world-client-map-data`, `world-client-level-events`, `world-client-respawn-data`, `rendering-client-block-entity-tracking`, `rendering-client-level-dirty-handoff`, `audio-client-level-sound-routing`, `diagnostics-client-level-report`, `player-local-session-respawn`, `player-local-movement-reporting`, `player-local-input-movement`, `player-local-vehicle-control`, `player-local-item-use-state`, `player-local-camera-targeting`, `interaction-game-mode-lifecycle`, `interaction-game-mode-guards`, `interaction-block-breaking`, `interaction-prediction-sequence`, `interaction-carried-slot-sync`, `interaction-item-use`, `interaction-entity-actions`, `interaction-container-actions`, `interaction-creative-actions`, `interaction-pick-actions`, `interaction-primary-action-routing`
Matrix routes: `rendering-client-biome-tint`, `rendering-level-backend-lifecycle`
Matrix routes: `ui-font-manager`, `ui-font-provider-pipeline`, `ui-menu-screen-registration`, `ui-chat-screen-surface`, `ui-death-screen-surface`, `ui-disconnected-screen-surface`, `ui-pause-screen-surface`, `ui-bidirectional-shaping`, `ui-local-time-icu`, `ui-context-entity-type`, `input-keyboard-routing`, `input-debug-key-actions`, `input-mouse-routing`, `input-key-mapping`, `identity-skin-presentation`, `player-local-world-edit-ui`
Matrix routes: `docs-chinese-mechanism-wiki`, `entities-fixed-type-surface`, `hud-frame-extract-order`, `hud-hotbar-selection`, `hud-overlay-effects`, `hud-player-vitals`, `hud-text-and-lists`, `persistence-options-apply`, `platform-window-surface-state`, `player-local-environment-effects`, `protocol-configuration-handler-decisions`, `protocol-listener-contracts`, `protocol-packet-tables`, `protocol-play-handler-decisions`, `protocol-serverbound-call-sites`, `rendering-effects-screenshot`, `rendering-entity-layer-registration`, `rendering-entity-renderer-registration`, `rendering-extract-surface`, `rendering-frame-order`, `rendering-level-diagnostics-gizmos`, `rendering-level-effect-lifecycle`, `rendering-level-light-query`, `rendering-level-pass`, `rendering-level-scene-projection`, `security-secure-chat-reporting`, `ui-accessibility-surface`
Matrix routes: `coverage-external-client-census`, `persistence-options-derived`, `persistence-server-list`, `platform-window-chrome`, `platform-window-display-mode`, `platform-window-lifecycle`, `player-local-permission-chat-state`, `ui-clipboard-surface`, `ui-screen-lifecycle`
Matrix routes: `rendering-block-entity-registration`, `rendering-block-model-pipeline`, `rendering-camera-fluid-fog`, `rendering-chunk-layer-routing`, `rendering-fluid-mesh`, `rendering-level-frame-graph`
Matrix routes: `resources-default-pack-stack`, `rendering-level-section-lifecycle`
- Close the 102 records that remain `partial` in the current parity acceptance
  report. All 147 records and reverse artifact/scenario mappings validate against
  gated evidence commit `553d02405` with zero false `equivalent`, zero missing
  focused-decision evidence, and current Vanilla owner source hashes; the
  bidirectional-text owner is specifically bound to `Language`, `ClientLanguage`,
  `FormattedBidiReorder`, and `SubStringSource` instead of a prose-derived source
  fallback. Current visual/draw/performance receipts are rebound from capture
  `fa73aed19` by identical `elf-load-image-v1`; remaining closure requires the
  acceptance schema v2 selected-gate-receipt change to pass and bind its current
  full gate, an exact-commit real ALSA host-device receipt, plus per-mechanism matrix review. Exact-commit
  lifecycle and Linux platform receipts are now present; the platform seat,
  session-replacement, task-handoff and client-runtime rows completed independent owner/focused review.
  Preserve invalid receipts until replaced; the user-waived
  macOS `deferred-external` row is not a closure blocker but retains its source
  basis and reopen condition.
- The server-pack lifecycle census now covers all 68 reachable/helper owners in
  `DownloadedPackSource`, `ServerPackManager`, `DownloadQueue`,
  `DownloadCacheCleaner`, the `HttpUtil` download chain and the Minecraft clear
  call. Download/cache/log/vacuum ordering, one DownloadQueue batch per external
  push, cross-batch failure propagation and per-batch five-slot progress toasts
  now have decision-level tests. All 68 owners have implementation/test or
  explicit scope evidence; `createDownloadNotifier` is bound to the shared
  mixed-family five-slot manager, exact callback mutation actions and notifier
  generation. The pack provider now
  retains the downloaded ZIP as its canonical source, indexes the central
  directory without eager extraction, resolves duplicate lookup using the final
  central entry, preserves central-order overlay/list decisions, and materializes
  only consumed resources into a flat temporary cache. Focused tests cover
  duplicate entries, file/directory collisions, overlays, long paths, high-ratio
  content, lazy listing and persistent archive ownership. Runtime pack ids now
  use the per-load-attempt `server/%08X/%UUID` serial and the repository retains
  Vanilla's reverse-push ordering. Reopen the provider
  work if `FilePackResources`, JDK `ZipFile` lookup/enumeration, resource filter,
  overlay or reload ownership changes.
- The server-pack runtime/net/world/HUD implementation and per-owner evidence
  remain open until a current-HEAD live receipt is captured. Reopen runtime
  identity/order if `packIdSerialNumber`, repository
  sorting, required-top insertion, `ToastManager.update`, toast height/slot,
  notifier callback or diagnostics ownership changes.
- The generated HUD/toast census covers all 107 methods in `Toast`,
  `ToastManager`, `SystemToast`, `AdvancementToast`, `RecipeToast`,
  `TutorialToast`, `NowPlayingToast`, and `PeriodicNotificationManager`, in
  addition to the existing 11 `hud-frame-extract-order` owners. All 118 have
  machine-routed evidence; the startup-consumed non-`NEVER` `musicToast` and
  separate `NowPlayingToast` lifecycle are closed with actual resolved sound
  identity, STARTED/STARTED_SILENTLY, timing, pause/static, palette, animated
  sprite and draw-order tests. `SystemToast` now retains a structured
  translatable title through native state and resolves it against the current
  language at extraction, while construction/reset-time message lines and the
  original width remain frozen. The first-multiplayer-server owner now consumes
  `joinedFirstServer`, emits the 8000 ms social-interactions tutorial before the
  ordinary tutorial tick, persists the sentinel in the same effect batch, uses
  the rebound key component, and preserves Vanilla's strict `> 8000 ms` hide
  boundary. One owner remains honestly open. The generated
  [`file-drop-capability.json`](evidence/vanilla-26.1/file-drop-capability.json)
  derives Vanilla's single-batch `(valid_paths, failed_count)` contract and
  `SystemToast.onFileDropFailure` translation from source hashes, then pins
  winit 0.29.15 by Cargo.lock checksum and audits its public event plus
  Windows/macOS/X11/Wayland backends. The public API has no batch identity or
  failed count; X11 keeps fallible parsing internally but delivers only an
  `Ok` path list, so BBB cannot reconstruct the exact count after
  `WindowEvent`. Reproduce with
  `scripts/python scripts/generate-file-drop-capability.py --vanilla-root
  "$(scripts/vanilla path 26.1 --kind root)" --check`. Close only when a
  supported owned adapter preserves one batch's valid paths and exact failed
  conversion count before winit flattens/discards them, then routes
  `failedCount > 0` through the shared toast manager. Reopen when the winit
  event/backend contract or supported-platform scope changes. This external
  blocker remains recorded even though macOS-only deferred work is not a
  delivery blocker.

## Render-step and HUD presentation closure

Owner: `bbb-renderer` + `bbb-native` + `bbb-world` + `bbb-pack`

Matrix routes: `hud-frame-extract-order`, `hud-player-vitals`, `hud-hotbar-selection`,
`hud-overlay-effects`, `hud-text-and-lists`, `rendering-frame-order`,
`rendering-extract-surface`, `rendering-level-pass`, `rendering-effects-screenshot`

Structural invariant: one frame-extract/submission path, and `Gui`'s direct
support accessors stay accessors rather than becoming a second decision owner.

- The auto world-icon screenshot pair
  (`tryTakeScreenshotIfNeeded`/`takeAutoScreenshot`) is `not-needed`: both gate on
  `Minecraft.isLocalServer()` and the integrated server's
  `getWorldScreenshotFile()`, so a multiplayer-only external client never reaches
  the 1000 ms retry latch, the rendered-sections readiness gate, or the 64x64
  world-icon write. Manual F2 screenshots keep their own owners. Reopen if bbb
  grows an integrated-server surface.
- `setRenderBlockOutline` is `not-needed`: its only vanilla callers disable and
  restore the outline around `Minecraft.grabHugeScreenshot`'s 4096-wide panoramic
  capture, and bbb has no panoramic/huge-screenshot mode (the same absence keeps
  the live `LevelFrameRoute` panoramic flag hard-false). Reopen with any panoramic
  capture surface.
- `renderActiveTextDebug` is `not-needed`: vanilla only calls it under
  `SharedConstants.DEBUG_ACTIVE_TEXT_AREAS`, a dev-build `debugFlag` that is
  false whenever `DEBUG_ENABLED` is off, so no production 26.1 client renders the
  glyph-area markers. Reopen if the flag ever ships enabled.

## Persistence, profile and secure-session closure

Owner: `bbb-native` + `bbb-net` + `bbb-item-model`

Matrix routes: `persistence-options-file`, `persistence-options-derived`,
`persistence-options-apply`, `persistence-server-list`,
`identity-profile-key-lifecycle`, `identity-skin-presentation`,
`security-secure-chat-reporting`, `persistence-options-unconsumed`,
`audio-sound-manager-engine`

- Produce the formal Vanilla↔BBB default/ALSA host-device differential receipt
  for `audio-sound-manager-engine`, exercising both HRTF requests, device-derived
  pool capacity, static/streaming source state, buffer lifecycle, listener state,
  device reload and stopped-channel release. The 159-owner generated census has
  implementation/focused-test evidence; its twelve unreachable integrated-pause/
  manual-loop/runtime-options owners retain caller basis and reopen conditions. `--driver null`
  remains diagnostic only. The 2026-08-07 default-driver recapture is invalid:
  both HRTF branches fail in Vanilla `Library.openDeviceOrFallback`; UID 1000 is
  not in device group 29 and every `/dev/snd` character node is `crw-rw----`
  root:audio. Owner: `bbb-pack` + `bbb-audio` + `bbb-native`. Vanilla anchors:
  `Library.java:64-114,124-155`, `Channel.java:35-149`,
  `SoundEngine.java:101-109,215-327,402-443`, and `Sound.java:68-70`.
  Reopen the owner census if any source hash, OpenAL artifact or method set moves.

- **Online mode (authenticated Services and encrypted transport) is
  `not-needed`.** User scope decision 2026-08-05: this client is offline-only.
  The formerly implemented online surface — RSA/AES-CFB8 encrypted login with
  the sessionserver join, the `player/certificates` profile-key fetch, the
  local signed-message chain with `ChatSessionUpdate`, the bearer-token
  Services block-list/abuse-report REST client, and the
  `--profile-id`/`--access-token` startup credentials — was removed rather
  than left as permanently unverifiable dead code. A server that opens the
  vanilla encryption handshake now fails the connection with an attributable
  "online-mode servers are unsupported" error before any cipher could be
  negotiated. Remote players' signed chat and announced sessions are still
  decoded and validated with the public Services key set, and signable command
  arguments keep the vanilla signed-command packet shape with an empty
  signature list — matching a vanilla client that has no key pair. Reopen only
  if the offline-only scope decision itself is reversed; restoring the online
  surface means reimplementing encrypted login, session join, and the
  profile-key lifecycle from vanilla source.
The offline service boundary follows vanilla's `UserApiService.OFFLINE_PROPERTIES`
semantics — empty flags and ban scopes, so profile-flag chat restrictions, the
Services block list and the abuse-report sender stay inert exactly like
vanilla's auth-failure fallback; with online mode out of scope no activation
path exists at all.

## P4 lifecycle, soak, and supported-platform release gates

Owner: `bbb-native` + `bbb-net` + `bbb-renderer` + `bbb-audio` + `bbb-control`

Matrix routes: `platform-linux-seat-matrix`, `platform-macos-release`, `platform-windows-release`

- An X11 release row is `not-needed`: the user declared X11 a non-goal on
  2026-08-06 in favour of the declared Wayland row. bbb keeps its winit X11
  branches, and a measured X11 run does open an `x11` window on hardware Vulkan
  and reach `Play`. That run's live look steering did not converge, but that is
  not an X11 defect and must not be cited as one: the same symptom then
  reproduced on Wayland against a seat-backed compositor, where it was traced to
  the client pausing itself after entering Play unfocused — a display server
  that publishes a real seat sends `Focused(false)` until something activates
  the window, which overrides the control-plane focus stand-in. The row rests on
  the scope decision alone; reopen only if that decision is reversed.
- Windows is `not-needed`: the user declared it a non-goal on 2026-08-06 and
  then ruled that bbb supports macOS and Linux only, so the Windows surface was
  removed rather than left as permanently unverifiable code — the same
  treatment online mode got. Gone: `PlatformTarget::Windows` and with it the
  `rundll32 url.dll,FileProtocolHandler` URI opener, the `explorer` path
  opener and the `%APPDATA%` client-store root; `WindowIconTarget::Windows`
  and the `WindowExtWindows` taskbar-icon call; `Window.getPlatform`'s `win32`
  identity, which now has no target that could report it. `bbb-native` refuses
  a Windows build outright (`compile_error!` in `main.rs`), so the decision is
  enforced by the compiler rather than by this paragraph, and
  `test_dev_chain_portability.py` rejects a returning `cfg(windows)` arm or
  Windows-only API — necessary because nothing here compiles that target, so a
  regrown arm would otherwise be invisible. One derivation stays because it is
  a Vanilla options fact rather than platform support: `syncChunkWrites`
  defaults to `Util.getPlatform() == Util.OS.WINDOWS` (Options.java), and the
  generated catalog keeps reading it from `platform::HOST_IS_WINDOWS`, false by
  construction. Reopen only if the scope decision is reversed; restoring
  Windows means reimplementing that dispatch from Vanilla source.
- macOS release validation is `deferred`: bbb is developed on macOS and Linux,
  but no maintained macOS host/runner exists with the winit, wgpu, audio and
  input fixtures the declared platform matrix needs. Reopen when such a host is
  provisioned and can execute that matrix. What is attested without one is the
  build, not the run, and the distinction is mechanical: `cargo check --target
  aarch64-apple-darwin` (gate step `cross-check`, `scripts/cross_check.py`)
  type-checks every workspace crate whose closure cross-compiles from Linux,
  and `crates/bbb-platform-macos` exists so the AppKit calls are inside that
  set rather than in the one crate no host here compiles. Three crates stop at
  `ring` and one at `objc_exception`, both third-party build scripts that need
  an Apple toolchain; that boundary is measured per run, not asserted. Do not
  read a passing cross-check as release validation.

Nothing else on that row is deferred. Compositor-originated focus, keyboard,
pointer buttons, relative pointer motion, the clipboard, live DPI transitions,
fullscreen and IME composition are all attested: the release run is driven from a real `wl_seat` and
`scripts/platform-acceptance-gate.sh` requires its receipts, which retires the
former `physical-seat-input` entry. The gate
refuses receipts from a run that also wrote synthetic `input.*`, so they cannot
be manufactured by the control plane. What the seat proves: the compositor
grants focus, a seat Escape reopens the world a client paused itself into after
losing focus, a seat hotbar key moves the canonical selected slot with server
confirmation, a seat pointer button reaches the canonical swing, relative
pointer motion turns the canonical view, and text placed on the seat's
selection reaches the canonical chat draft through the paste chord.


## Native input, movement, interaction, inventory, and command flows

Owner: `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world`

Matrix route: `interaction-server-authority-exclusions`

- Bottle dragon-breath client prediction is `not-needed`: `AreaEffectCloud.owner`
  is absent from both `ClientboundAddEntityPacket` and synchronized metadata, so
  the remote 26.1 client leaves that branch server-authoritative. Reopen if the
  owner becomes client-visible on the wire.

## Renderer scene parity

Owner: `bbb-renderer` + `bbb-native` + `bbb-pack` + `bbb-world`

Matrix route: `rendering-official-noop-surfaces`

Living-entity render state:

- Armor-stand pose interpolation is `not-needed`: vanilla 26.1
  `ArmorStandRenderer.extractRenderState` copies all six synced `Rotations`
  getters directly without partial-tick interpolation. Only `yRot` uses
  `Mth.rotLerp`, and hit `wiggle` adds `partialTicks`; bbb already covers both.

Reopen `rendering-official-noop-surfaces` if any routed Vanilla no-op owner gains
a client-visible render state, submission, particle, or debug effect.

## HUD overlay and screen render surfaces

Owner: `bbb-renderer` + `bbb-native` + `bbb-world`

Matrix routes: `debug-integrated-server-mirrors`, `movement-remote-type-exceptions`

- `visualize_chunks_on_server` is `not-needed`: vanilla 26.1
  `ChunkDebugRenderer.emitGizmos` refreshes data only when
  `Minecraft.getSingleplayerServer()` returns an `IntegratedServer`, and its
  `ChunkData` reads `ServerLevel` / `ServerChunkCache` directly on that server's
  executor. bbb is an external native client and owns no integrated-server or
  server chunk-cache surface, so vanilla-equivalent behavior is an empty
  renderer. Reopen if the repo gains an in-process integrated server boundary.
- Remote `visualize_entity_supporting_blocks` `fallDistance > 2.5` powder-snow
  support is `not-needed`: vanilla position/move/teleport packets update a
  server-authoritative remote entity through `moveOrInterpolateTo` and
  `setOnGround`; interpolation uses `setPos`, and only locally authoritative
  `Entity.move` calls `checkFallDamage`, so remote `fallDistance` remains its
  default zero. Local-player high-fall powder snow and the falling-block type
  branch are covered; locally controlled vehicle physics is owned by
  `bbb-world::entities::passengers` and does not make a remote entity's
  server-authoritative `fallDistance` observable.
- Entity-hitbox local-server green boxes and delta arrows are `not-needed`:
  vanilla 26.1 `EntityHitboxDebugRenderer.getServerEntity` returns an entity
  only through `Minecraft.getSingleplayerServer()` -> `IntegratedServer` ->
  `ServerLevel.getEntity`; an external multiplayer client always receives
  `null` and emits the red `Missing Server Entity` 3D billboard instead. bbb
  owns no integrated server and implements that missing-label branch. Reopen
  with an in-process integrated server boundary.
## Item model range-dispatch and select projection

Owner: `bbb-protocol` + `bbb-native` + `bbb-pack`

Matrix routes: `items-server-owned-effects`

Each item below plugs into the existing value-aware `RangeDispatch` / `Select`
resolver by adding a value provider. No new selection machinery is required.

- `not-needed`: synchronized conditional value/entity/location enchantment
  effects, damage immunity, equipment-drop prevention, and crossbow charging
  sounds remain server-owned in Vanilla 26.1. Conditional execution enters
  through `ServerLevel`; crossbow loading sounds run only under
  `!level.isClientSide()`. Reopen if
  `test_enchantment_client_effect_partition.py` finds a client-side consumer.

## Audio runtime parity

Owner: `bbb-audio` + `bbb-native` + `bbb-pack` + `bbb-world`

Matrix route: `interaction-server-authority-exclusions`

- Client-owned animal ambient/step/eat and cat/wolf state-trigger loops are
  `not-needed`: vanilla `Entity.playSound` delegates through
  `Level.playSound(null, ...)`, `ClientLevel.playSeededSound` only submits when
  the excluded entity is its local player, and `ServerLevel` supplies remote
  listeners with Sound/SoundEntity packets. Reopen only if a target animal path
  is shown to call `playLocalSound` or stops being server-packet-owned.

Audio unit tests must remain independent of an audio device.

## Resource and dynamic texture generalization

Owner: `bbb-pack` + `bbb-native` + `bbb-renderer`

Matrix route: `registries-unsynchronized-fixed-identities`

Lifecycle invariants (not todos): vanilla, startup and server packs share one
validated low-to-high selected stack and immutable generation; CPU consumers
prepare from that stack, renderer/audio owners commit atomically, and control
publishes source/priority/failure plus bounded dynamic-texture observations.
Skin/cape/elytra requests share one process-lifetime CPU owner; resource/session
generations only own GPU delivery and cannot force a second HTTP/cache resolver.

- Synchronized custom block/entity registry identities are `not-needed`:
  Vanilla 26.1 `RegistryDataLoader.SYNCHRONIZED_REGISTRIES` contains neither
  `BLOCK` nor `ENTITY_TYPE`; their protocol identities remain fixed, and a
  resource pack cannot add either registry entry. Reopen if the target protocol
  begins synchronizing one of those registries.

## Container menu presentation parity

Owner: `bbb-native` + `bbb-renderer` + `bbb-world`

Matrix route: `ui-container-server-authority-exclusions`

- A matching-id authority check for ContainerClose is `not-needed`: vanilla
  26.1 carries an id but closes the current menu unconditionally.
- A synthetic menu fallback for mismatched ContainerSetData is `not-needed`:
  `ClientPacketListener.handleContainerSetData` accepts only the current
  `containerMenu` id.
- A synthetic menu fallback for mismatched nonzero ContainerSetSlot is
  `not-needed`: `ClientPacketListener.handleContainerSetSlot` rejects it.
- A combat-mode runtime transition for PlayerCombatEnter is `not-needed`: its
  26.1 client handler is empty.
- A combat-mode runtime transition for PlayerCombatEnd is `not-needed`: its
  decoded duration is diagnostic and its 26.1 client handler is empty.

Reopen this route if any cited 26.1 handler changes its id guard, empty-body
decision, or current-menu authority semantics.

## HUD and debug overlay

Owner: `bbb-native` + `bbb-renderer` + `bbb-audio`

Matrix route: `debug-integrated-server-mirrors`

- **Client-only debug entries with no HUD line** are `not-needed`:
  LocalDifficulty, EntitySpawnCounts, and ChunkGenerationStats read an
  integrated local-server mirror (`ServerLevel`,
  `NaturalSpawner.SpawnState`, `ChunkGenerator` / `RandomState` /
  `BiomeSource`) that an external native client does not own.

## Entity models and animation

Owner: `bbb-world` + `bbb-native` + `bbb-renderer`

Matrix route: `rendering-official-noop-surfaces`

### Model and animation details

- **Frog tongue prey-targeting is `not-needed`.**
  `DATA_TONGUE_TARGET_ID` is consumed by frog/AI behavior only. Vanilla
  `FrogRenderer.extractRenderState` does not project it, `FrogRenderState` has
  no target field, and `FrogModel.setupAnim` drives the tongue solely from the
  copied tongue `AnimationState`.
- **Rabbit `IDLE_HEAD_TILT` keyframe** is `not-needed`: vanilla starts it from
  the entity's client-local `random.nextInt(40) + 180` timeout, so a remote
  client's particular phase is not reconstructable and bbb must not fabricate
  a seed.
- **Minecart spawner animated block-entity display content is `not-needed`.**
  Vanilla renders a `MinecartSpawner` through the ordinary minecart display
  block only. No block-entity renderer calls
  `BaseSpawner.getOrCreateDisplayEntity` for that embedded spawner, so its
  `displayEntity` remains null and `clientTick` advances neither spin nor
  display particles.

## Particles and level events

Owner: `bbb-world` + `bbb-native` + `bbb-renderer`

Matrix route: `rendering-particle-provider-registration`

Routing invariant: terrain/item atlas rendering, block-entity client-effect
presentation, and audio-device/runtime parity stay with their owning domains;
the `LevelEventHandler` switch itself is not a second owner.

---

# Not needed

Vanilla 26.1 has no such behavior, or bbb has no surface for it. Each carries the
vanilla evidence. Deleting one of these re-opens work already decided against.

## Packet canonical state

Matrix routes: `scope-telemetry-upload`, `protocol-framing-no-world-state`

- **Mojang client telemetry callbacks.** `bbb` has no external telemetry sender.
  Vanilla `WorldSessionTelemetryManager.onServerBrandReceived` only stores the
  brand on `WorldLoadEvent` and attempts a Mojang telemetry send; server-brand
  protocol state and local diagnostics remain independently owned.

- **Bundle delimiter world state.** `ClientboundBundleDelimiterPacket` is
  consumed by `PacketBundlePacker` / `BundlerInfo` as connection framing before
  listener dispatch; it has no payload, listener handler, or canonical world
  mutation.

Reopen `scope-telemetry-upload` if the offline external-client scope gains a
telemetry sender; reopen `protocol-framing-no-world-state` if the delimiter gains
a payload, listener call, or observable canonical-world effect.

## Source inventory support owners

Matrix routes: `scope-support-accessors`, `scope-client-level-noop-hooks`

- **Direct accessors are not independent mechanisms.** The shared Tree-sitter
  inventory classifies only single-return field, parameter or constant bodies in
  `Minecraft`, `LocalPlayer`, `MultiPlayerGameMode` and `ClientLevel` as
  `support-non-decision`; every branch, allocation, call or mutation must have an
  explicit mechanism route. Reopen when the CST body of any of the 99 classified
  owners gains a decision or side effect.
- **Empty `ClientLevel` client hooks have no BBB consumer.** Vanilla
  `gameEvent`, `explode` and `EntityCallbacks.onCreated/onDestroyed/onSectionChange`
  have empty bodies and do not mutate client state or emit an effect. Reopen when
  any source body gains a statement or the enclosing lifecycle starts invoking a
  distinct client decision owner.

## Target-version item-model surface

Matrix route: `items-versioned-property-surface`

- **Additional ambient-context numeric `range_dispatch` properties.** Vanilla
  26.1 `RangeSelectItemModelProperties.bootstrap` registers exactly
  custom-model-data, bundle fullness, damage, cooldown, time, compass,
  crossbow pull, use cycle, use duration and count, all of which are present in
  the native resolver. Reopen only when the target version changes or that
  registration list differs.

## Resource and screen surface

Matrix routes: `resources-indexed-built-in-packs`, `resources-indexed-realms`, `resources-world-list-favicons`, `scope-title-screen`, `scope-realms`, `scope-multiplayer-server-list`

- **Server-list editing owners.** `ServerList.size`/`swap`/`replace`/`remove`/
  `unhide` mutate or index the in-memory list rather than the file. In 26.1
  their only callers are `ServerSelectionList` (`size`, `get(int)`, `swap` for
  the move-up/down buttons) and `JoinMultiplayerScreen` (`remove` for delete,
  `unhide` and `add` for the edit-server dialog); `replace` has no caller at
  all. The command-line entry point `QuickPlay.joinMultiplayerWorld` uses only
  `load`/`get(String)`/`add`/`save`, which is what
  `crates/bbb-native/src/server_list.rs` owns. Reopen with a
  server-selection screen, not before.
- **Title splash reload listener.** Vanilla `SplashManager.prepare` reads
  `minecraft:texts/splashes.txt`, but its only consumer is
  `TitleScreen.init` calling `Minecraft.getSplashManager().getSplash()`.
  The CLI-direct client has no title screen. Reopen when a title screen enters
  the product; merely exposing the resource through the default pack does not
  create a consumer.
- **Indexed built-in resource-pack ZIPs.** `ClientPackSource` exposes
  `programmer_art` and `high_contrast` through the pack repository/options
  selection surface. bbb accepts explicit startup/server packs but has no
  built-in pack-id selection or in-game options UI, so the two ZIPs have no
  current consumer. Reopen when a built-in pack-id UI or CLI surface is added.
- **Indexed Realms language and carousel assets.** Vanilla's shared language
  manager can see `realms:lang/en_us`, while `AddRealmPopupScreen` enumerates
  `realms:textures/gui/images`; Realms service and screens are outside the
  external multiplayer client boundary. The 16 objects remain
  provider-reachable. Reopen when Realms service/screen support enters scope.

- **Broader non-profile dynamic textures.** The remaining Vanilla 26.1
  external-image producers are server/world-list favicons and pack-selection
  presentation. bbb exposes startup/server-pack configuration through the CLI
  and has neither screen; its current client surfaces already own map textures,
  profile skin/cape/elytra and synchronized pack-backed presentation. Reopen
  only if one of those screens enters the declared client scope or a new
  current-surface producer is identified.

## Non-player movement authority

Matrix route: `movement-remote-type-exceptions`

- **`POWDER_SNOW_WALKABLE_MOBS` collision exceptions.** Vanilla 26.1's tag
  contains only rabbit, endermite, silverfish and fox. None is a locally
  client-authoritative ridden type in the current client, so their collision is
  server-owned. Reopen specifically if one of those four types gains local
  movement authority.
- **Freeze-immune entity-type exceptions.** Vanilla
  `LivingEntity.tick` applies freeze damage only in `ServerLevel`, and
  `FREEZE_IMMUNE_ENTITY_TYPES` contains only stray, polar bear, snow golem and
  wither. Their remote frozen presentation remains server synchronized. Reopen
  specifically if one of those four types gains local freezing authority.
- **Sweet-berry fox/bee exceptions.** Vanilla
  `SweetBerryBushBlock.entityInside` skips fox and bee slowing, while its
  age-gated damage executes only in `ServerLevel`. Neither fox nor bee is a
  locally client-authoritative ridden type in the current client. Reopen if
  either exact type gains local movement authority.

## Entity models and layers

Matrix routes: `rendering-entity-renderer-registration`, `rendering-entity-layer-registration`, `rendering-official-noop-surfaces`

### Entity coverage matrix vanilla noop renderers

- Area-effect-cloud, marker, and interaction have no model/texture/layer
  presentation: vanilla registers each with `NoopRenderer`. Their other
  observable cells remain independently classified.

- **Chicken head look.** `ChickenModel` never animates the head.
- **Freezing white overlay.** The creeper is the only vanilla
  `getWhiteOverlayProgress` producer; the base renderer returns `0`.
  `isFullyFrozen` drives the `setupRotations` body shake, not an overlay.
- **Zoglin conversion shake.** `ZoglinRenderer` has no conversion override.
- **Snow-golem walk-driven swing.** Its `setupAnim` is the head-yaw twist/orbit.
- **Piglin-brute `ADMIRING_ITEM` pose.** The brute has no admire branch.
- **Fox baby model scale.** `Fox.BABY_SCALE = 0.6` scales only the bounding box,
  so the baby uses the standard `0.5` `LivingEntity.getAgeScale`.
- **Nautilus baby `0.7F` model scale.** `NautilusRenderer` passes `0.7F` as the
  `shadow` argument to `AgeableMobRenderer`, whose `submit` only selects the
  adult or baby model. Vanilla has no additional `0.7` baby-model transform.
- **Axolotl baby as a scaled adult.** The `0.5F` constructor argument is the
  shadow radius, not a scale; the baby has its own 32x32 geometry.
- **`SniffletModel.setupAnim` static baby transform** (`BABY_TRANSFORM` /
  `SNIFFER_BABY_FALL`). Vanilla `SnifferRenderer` does not consume it.
- **Baby saddle models** for pig, horse, donkey, mule, camel, strider, and living
  nautilus. Vanilla supplies no baby saddle model (`null` for strider; no baby
  model on `SimpleEquipmentLayer` for nautilus).
- **Baby horse body-armor layer.** Vanilla supplies no baby armor model.
- **Skeleton-horse body armor.** Vanilla
  `EntityTypeTags.CAN_WEAR_HORSE_ARMOR` includes only horse and zombie_horse.
- **Baby wandering-trader model/texture.** `WanderingTraderRenderer` is a plain
  `MobRenderer` always using `ModelLayers.WANDERING_TRADER` and
  `textures/entity/wandering_trader/wandering_trader.png`, so the inherited
  `AgeableMob.DATA_BABY_ID` selects nothing.
- **Pig boost/ridden-speed renderer pose.** `PigRenderState` carries saddle and
  variant only; boost lives in `ItemBasedSteering` / `Pig.tickRidden` movement
  control.
- **Area-effect-cloud, marker, and interaction models.** `EntityRenderers`
  registers all three to `NoopRenderer`: the cloud draws as particles, the marker
  is a server-side data entity, the interaction is an invisible click hitbox.
- **Invisible-gating `WolfArmorLayer`.** Vanilla does not gate that layer on
  `state.isInvisible`, so bbb keeps its armor/crack submissions in
  hidden/self-visible/glowing invisible states.
- **`CapeLayer` for invisible players.** Vanilla explicitly gates it on
  `!state.isInvisible`.
- **Separate glow-squid emissive overlay/texture.** Vanilla 26.1
  `GlowSquidRenderer` only selects the adult/baby texture and overrides
  `getBlockLightLevel`; it registers no independent emissive layer.
- **Exact guardian client RNG seed.** Vanilla seeds the per-spawn entity RNG
  outside the protocol. bbb uses one deterministic UUID/type-derived Java-LCG
  cursor for the constructor tail phase and subsequent spike/bubble/flop draws;
  draw order and ranges are exact, but a particular official-client sample is
  not reconstructable from wire state.
- **Guardian powder-snow stand-on-face exception.** The normal loaded collision
  UP-face path is exact. Vanilla can additionally expose powder snow as a full
  collision shape from entity-local fall distance or leather-boots context;
  remote Guardian fall distance is not synchronized, so bbb conservatively
  fails that rare flop-sound arming probe closed.
- **Exact enderman creepy root-jitter RNG seed.** Vanilla keeps that RNG inside
  the renderer and never exposes the seed over the protocol; bbb uses a
  deterministic Java-LCG-shaped seed from entity id + age while preserving the
  vanilla axis/scale behavior.
- **Exact horse `tailCounter` client seed.** Not protocol-visible; bbb uses a
  deterministic local Java LCG.

## Rendering and screenshots

Matrix route: `rendering-nondeterministic-golden-exclusions`

- **Exact golden pixel parity to vanilla screenshots.** No golden PNG or
  full-frame hash: driver and float jitter make it unreliable.
- **Item-pipeline overlay-texture (`Sampler1`) sampling.** Vanilla's
  `ITEM_SNIPPET` does not bind `Sampler1`; `core/item` samples only the item
  atlas and lightmap, and `RenderTypes.itemCutout` / `itemTranslucent` never call
  `useOverlay()`.
- **Lightning sky flash tinting water fog.** Vanilla does not tint it.

Reopen `rendering-nondeterministic-golden-exclusions` if a same-backend,
same-driver deterministic raster contract becomes available, or if a cited
Vanilla pipeline begins consuming one of the currently absent bindings/effects.

## Block entities

Matrix routes: `rendering-block-entity-registration`, `rendering-official-noop-surfaces`

- **Sign `is_waxed` render effect.** Vanilla has none; it gates editing.
- **Banner item / shield pattern path** (`Sheets.SHIELD_PATTERN_BASE`,
  `BannerRenderer.submitSpecial`). Item-model scope, not the block-entity
  renderer.
- **Shulker box open/close sound and interaction blocking.** Vanilla
  `ShulkerBoxBlock.useWithoutItem` / `canOpen` own collision admission, while
  `ShulkerBoxBlockEntity.startOpen` / `stopOpen` own the sounds;
  `ShulkerBoxRenderer` only consumes render state.
- **Bell resonation particle/glow chain** (`resonationTicks`, raider search).
  Gameplay-side, belongs with the raid features.

## Particles

Matrix routes: `rendering-particle-provider-registration`, `rendering-official-noop-surfaces`

- **Spawn-filtering for `minecraft:block_marker`.** Vanilla `BlockMarker.Provider`
  does not reject air, `moving_piston`, or `shouldSpawnTerrainParticles=false`
  states the way `TerrainParticle.createTerrainParticle` does.
- **`moving_piston` rejection in the destroy-block effect** (events `2001` /
  `3008`). Vanilla `addDestroyBlockEffect` does not reject it, unlike
  `TerrainParticle.Provider`.
- **xAux handling in `GustParticle.Provider`.** Vanilla ignores that xAux; the
  `GustSeedParticle.Provider` child still feeds child xAux `age / lifetime`.

## Input, movement, and interaction

Matrix route: `interaction-server-authority-exclusions`

- **Server-owned FishingHook catching/removal outcomes.** The bbb client tick
  owns Vanilla flight, Bobbing, hooked-target following, collision/deflection,
  metadata biting impulse, and Breeze deflect audio. `catchingFish`, loot,
  open-water/server timers, rod durability/pull results, and canonical removal
  remain incoming server/packet outcomes; an invalid-owner client instance is
  frozen until that authoritative removal instead of creating a second local
  removal path.
- **Local component fallback for Efficiency and Aqua Affinity destroy speed.**
  Vanilla routes both through synced attributes: Efficiency contributes to
  `mining_efficiency` (attribute id `20`), Aqua Affinity to
  `submerged_mining_speed` (attribute id `29`). `UpdateAttributes` stays the
  authoritative path.
- **Local terrain-collision handling of portal travel** (`nether_portal`,
  `end_portal`, `end_gateway`). All three are vanilla `.noCollision()`; portal
  travel, smoke/portal particles, and gateway cooldown are server/runtime or
  entity-inside behavior.
- **Dynamic collision for `moving_piston`** when no canonical moving-piston block
  entity exists. Treated as empty, matching vanilla's empty fallback.
- **Local prediction of Mount component-patched or entity-specific `equippable`
  predicates, and `sweet_berry_bush` damage.** Both are server-authoritative.
- **Touchscreen split-stack and snapback animation.** Vanilla gates both entirely
  behind `Options.touchscreen` (`AbstractContainerScreen.java:336, 342, 411, 489`;
  snapback interpolation `:146-158`) and bbb has no touch input mode or plan.

## HUD and configuration

Matrix routes: `diagnostics-client-only-inspection-exclusions`, `scope-options-ui`,
`resources-runtime-mipmap-update`, `scope-singleplayer-integrated`,
`debug-integrated-server-mirrors`

- **Shift-local F3+I server-private entity save residuals.** Not needed on an
  external client: ordinary F3+I queries the authoritative server tag, while
  Shift-local capture can project only synchronized metadata, spawn/equipment
  components, client-owned runtime, and protocol-backed references. Unretained
  Living impulse context, mutable owner/target/RNG/timer fields, non-empty
  entity inventories or loot tables, server-loaded spawner-minecart
  `SpawnData` / `SpawnPotentials` and non-default counters, and command-block
  minecart execution results have no clientbound owner. Reopen a field only
  when a protocol or client-owned mutation path makes its exact value
  observable.
- **Shift-local F3+I fixed-registry holder reconstruction for direct
  jukebox-song and painting values.** Not needed: vanilla
  `JukeboxSong.CODEC` and `PaintingVariant.CODEC` are named-only
  `RegistryFixedCodec`s, while their network holders may be direct and have no
  persistent resource key. Unknown direct holders omit the containing value
  rather than guessing an identity.
- **F3+I non-default `PrimedTnt.explosionPower`, item-frame `dropChance` /
  `fixed`, and `SpectralArrow.duration`.** Not needed on an external client:
  vanilla 26.1 changes all four only in `readAdditionalSaveData`; none is in the
  add-entity packet or synched entity data, and client tick/interaction does not
  mutate them. A newly recreated remote `PrimedTnt` therefore always keeps
  `explosionPower = 4.0` (whose save field is conditionally omitted), item/glow
  frames keep `dropChance = 1.0` and `fixed = false` (both always written), and
  spectral arrows keep `duration = 200` (always written). Reopen only if a
  future protocol synchronizes one of these private fields.
- **F3+I local fishing-bobber save NBT.** Not needed: vanilla
  `EntityType.FISHING_BOBBER` is `.noSave().noSummon()` (`EntityType.java`),
  `FishingHook.addAdditionalSaveData` is an empty override that does not call
  `Projectile.addAdditionalSaveData`, and `Projectile.isPickable` only returns
  true for `EntityTypeTags.REDIRECTABLE_PROJECTILE`; the 26.1 bootstrap tag
  contains fireball/wind-charge variants, not `FISHING_BOBBER`.
- **F3+F4 `GameModeSwitcherScreen` mouse-release path.** It only uses
  `mouseReleased` for `keyDebugModifier.matchesMouse(event)`, and
  `KeyMapping.matchesMouse` is true only for a `MOUSE`-type mapping. bbb's debug
  modifier is the hard-coded F3 keyboard path.
- **Container-loot loot-table/seed NBT payload.** Vanilla tooltip output
  (`SeededContainerLoot.addToTooltip` -> `item.container.loot_table.unknown`)
  depends only on presence.
- **In-game options UI and vanilla options-file persistence.** Configuration
  stays at command-line startup (for example `--advanced-item-tooltips`); bbb
  does not add an in-game configuration UI or call vanilla `Options.save()`.
  This also owns `Minecraft.updateMaxMipLevel`: its only Vanilla 26.1 caller is
  `VideoSettingsScreen`, while BBB's terrain generation already uses the
  Vanilla default mip level 4. Reopen the runtime update mechanism when a CLI
  or supported in-game surface can change mipmap levels after startup; that
  slice must rebuild the complete resource/GPU generation atomically rather
  than mutate the atlas behind its owner.
- **World-access/delete, pack-copy and integrated chunk-storage SystemToast
  helpers.** `SystemToast.onWorldAccessFailure`, `onWorldDeleteFailure`, and
  `onPackCopyFailure` are called only from title/world-selection or in-game pack
  selection; `onChunkLoadFailure`/`onChunkSaveFailure` are called only by
  `IntegratedServer` (`SystemToast.java:132-188`, `IntegratedServer.java:352-363`).
  Those five owners have no observable effect in the external multiplayer,
  command-line-configuration scope. Reopen the first group with a supported
  world/pack-selection UI, and the second with integrated-server or
  client-owned server chunk storage.
- **Vanilla OpenGL GPU warnlist reload listener.**
  `GpuWarnlistManager.Preparations.apply` returns warnings only when
  `GpuDevice.getBackendName()` is exactly `OpenGL`; its behavioral consumer is
  the improved-transparency branch in `Options`/`VideoSettingsScreen`, with an
  optional copy in `Minecraft.fillSystemReport`. BBB ships the wgpu primary
  Vulkan/Metal/DX12 backends and has no in-game options UI, so the Vanilla
  predicate is always empty. Reopen when an OpenGL backend or the corresponding
  improved-transparency warning choice enters the supported product; then the
  diagnostics copy must be wired in the same slice.
- **Custom/datapack attribute tooltip definitions.** Vanilla 26.1 registers
  `minecraft:attribute` through `BuiltInRegistries.registerSimple`; `Attribute`
  has no direct element codec and the registry is absent from both
  `RegistryDataLoader.WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`.
  The fixed 35-entry vanilla catalog is generated; this item covers non-vanilla
  definitions rather than built-in id/resource metadata.
  Attribute holders therefore use the fixed client built-in registry, so no
  server/datapack description or sentiment payload exists to project.
- **Custom/datapack mob-effect definitions.** Vanilla 26.1 registers
  `minecraft:mob_effect` through `BuiltInRegistries.registerSimple`, and the
  registry is absent from both `RegistryDataLoader.WORLDGEN_REGISTRIES` and
  `SYNCHRONIZED_REGISTRIES`. The fixed 40-entry vanilla id/resource/category
  catalog is generated; effect holders therefore resolve against the built-in
  client registry rather than server-defined effect metadata.
- **Custom/datapack potion registry definitions.** Vanilla 26.1 registers
  `minecraft:potion` through `BuiltInRegistries.registerSimple` with
  `Potions::bootstrap`; the registry is absent from both
  `RegistryDataLoader.WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`.
  Potion holder ids and their base effect lists therefore come from the fixed
  client built-in registry rather than a server-remappable registry payload.
- **Custom/datapack villager-type and villager-profession definitions.**
  Vanilla 26.1 registers both defaulted registries in `BuiltInRegistries`
  through `VillagerType::bootstrap` / `VillagerProfession::bootstrap`; neither
  appears in `RegistryDataLoader.WORLDGEN_REGISTRIES` or
  `SYNCHRONIZED_REGISTRIES`. Their `VillagerData.STREAM_CODEC` holders resolve
  through the connection's static registry layer, whose ids/order come from
  the local built-in catalogs rather than configuration `RegistryData`. The
  fixed 7-entry type and 15-entry profession identity/ordinal catalogs are
  generated directly from those vanilla bootstrap registrations.
- **Custom/datapack decorated-pot pattern definitions.** Vanilla 26.1 registers
  `minecraft:decorated_pot_pattern` through `BuiltInRegistries.registerSimple`
  with `DecoratedPotPatterns::bootstrap`, and the registry is absent from both
  `RegistryDataLoader.WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`.
  Item-to-pattern identity and pattern asset ids therefore come from the fixed
  client built-in registry rather than a server-remappable registry payload.
- **Custom/datapack menu-type definitions.** Vanilla 26.1 registers
  `minecraft:menu` through `BuiltInRegistries.registerSimple`, and the registry
  is absent from both `RegistryDataLoader.WORLDGEN_REGISTRIES` and
  `SYNCHRONIZED_REGISTRIES`. The fixed 25-entry vanilla menu id/resource catalog
  is generated; OpenScreen menu ids therefore resolve against the built-in
  client registry rather than server-defined menu metadata.
- **Custom difficulty enum values.** Vanilla 26.1 `Difficulty` is a closed Java
  enum with four explicit ids/names and `ByIdMap.OutOfBoundsStrategy.WRAP`;
  protocol/world consume the generated catalog and there is no server/datapack
  definition surface to load.
- **Custom equipment-slot or slot-group enum values.** Vanilla 26.1 defines
  both as closed Java enums, not registries or datapack codecs. Their distinct
  declaration ordinals / explicit ids and serialized names are generated from
  `EquipmentSlot.java` and `EquipmentSlotGroup.java`; the packet-facing typed
  equipment-slot enum and wire ordinal lookup share that generated owner. There
  is no server-owned extension surface to load.
- **Custom entity-pose enum values.** Vanilla 26.1 `Pose` is a closed Java enum
  whose id-mapped stream codec uses its 18 explicit ids. The fixed id/name
  catalog is generated from `Pose.java`; it has no registry or datapack
  extension surface.
- **Custom entity-data serializers.** Vanilla 26.1 assigns the built-in ids in
  the static `EntityDataSerializers` registration block. Its public code-level
  registration helper is not a server- or datapack-synchronized registry. The
  fixed 43-entry vanilla id/order catalog is generated from that block;
  serializer wire codecs remain explicit protocol semantics rather than
  dynamically supplied definitions.
- **Custom client game-event packet types.** Vanilla 26.1 declares the 14
  `ClientboundGameEventPacket.Type` instances and their byte ids directly in
  the packet class; there is no server/datapack registration surface. The
  generated fixed catalog intentionally does not treat vibration game events
  or entity-event ids as members of this packet type map.
- **Custom sound-source enum values.** Vanilla 26.1 `SoundSource` is a closed
  Java enum whose wire value is its declaration ordinal. The generated fixed
  11-entry ordinal/name catalog has no server or datapack extension surface.
- **Custom item registry definitions.** Vanilla 26.1 item protocol ids come
  from the fixed client built-in `Items` registration order, not a synchronized
  server/datapack registry. The generated 1506-entry catalog covers the vanilla
  mapping; modded clients that change item registration order remain outside
  the native vanilla-client target.
- **Custom block or block-state registry definitions.** Vanilla 26.1 block and
  global block-state ids come from the fixed client built-in registration; the
  checked-in 29,873-state report is the native lookup authority. Modded clients
  that alter block registration or state definitions remain outside the vanilla
  client target.
- **Accepting malformed or unsupported custom resource-pack declarations.**
  Vanilla resource loaders parse with `StrictJsonParser` plus typed codecs and
  reject invalid declarations; bbb likewise fails the selected generation
  instead of inventing fallback schema.
