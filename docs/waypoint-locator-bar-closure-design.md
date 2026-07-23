# Waypoint locator-bar closure

Status: active engineering contract.

## Goal and completion contract

Close the remaining runtime/presentation gap for
`ClientboundTrackedWaypointPacket`. The exact waypoint set already owned by
`WorldStore` must produce Vanilla 26.1 locator-bar markers from the active
camera, selected resource generation, and current contextual-bar priority
state.

Completion requires:

- all four tracked-waypoint data forms (`empty`, block position, chunk, and
  azimuth) have deterministic Vanilla-equivalent distance, yaw, pitch, and
  visibility projection;
- a UUID block-position waypoint follows the matching entity's interpolated
  eye position only while that entity's current block position remains within
  Manhattan distance 3 of the transmitted vector;
- the active camera entity is excluded only when its UUID is the waypoint UUID;
  named identifiers remain renderable;
- markers are submitted farthest first so nearer markers retain Vanilla's
  later-draw overlap priority;
- explicit icon RGB is opaque, while absent color uses Java UUID/String
  `hashCode` followed by `ARGB.setBrightness(..., 0.9F)`;
- the selected client pack supplies `waypoint_style/*.json`, all referenced GUI
  atlas sprites, the missing-texture fallback, the 182x5 background, and the
  two 7x5 pitch arrows through the renderer's transactional resource owner;
- distance selects the same near/middle/far sprite as `WaypointStyle.sprite`,
  yaw is visible only in `(-60, 60]`, and renderer geometry uses the Vanilla
  173-pixel marker travel, 9x9 dot, and up/down arrow offsets;
- `Gui.nextContextualInfoState` priority is preserved: an active jump or
  cooldown temporarily beats locator, a recently changed experience bar beats
  locator for 100 player ticks, otherwise locator beats idle jump/experience;
- the first experience synchronization does not start the 100-tick priority
  window, while later progress changes and opening an anvil/enchantment screen
  do;
- experience-level text remains independent of the selected contextual bar;
- configuration reentry, dimension/session generation changes, camera absence,
  malformed/unknown style ids, missing sprites, empty waypoint data, and
  non-finite projection inputs fail closed without mutating canonical state;
- the `runtime_side_effect` cell for `CLIENTBOUND_WAYPOINT` reaches `complete`
  with world-to-frame and renderer-geometry evidence; and
- the waypoint locator-bar item is removed from
  `docs/unsupported-features.md`.

## Non-goals

- This slice does not change waypoint wire decode, canonical track/update/
  untrack behavior, server transmit range, or server-side waypoint production.
- It does not implement a waypoint configuration or editing UI.
- Third-person camera support remains outside the current first-person-only
  client boundary. The projection consumes the active camera pose already
  published by bbb and does not add a second perspective path.
- Generic GUI-atlas animation/mipmap consumption remains the existing P3
  resource-pack item. Animated locator arrows use the accepted sprite's first
  frame until that shared atlas owner lands; the locator does not add a private
  animation scheduler.
- Creative/spectator screen work and the spectator hotbar remain separate
  surface gaps.
- No unrelated contextual-bar, player experience, vehicle physics, or menu
  behavior is added.

## Boundaries and frozen integration API

Semantic owners:

- canonical waypoint/entity queries:
  `crates/bbb-world/src/client/waypoints.rs` and the existing entity store;
- camera projection, Java color fallback, and contextual priority:
  `crates/bbb-native/src/runtime/waypoint_locator.rs` plus
  `HudRenderRuntime` in `crates/bbb-native/src/runtime.rs`;
- selected-pack style/sprite loading:
  `crates/bbb-pack/src/waypoint_styles.rs` and
  `crates/bbb-native/src/hud_assets.rs`;
- pure frame values, resource ownership, layout, and draw submission:
  `crates/bbb-renderer/src/hud.rs`,
  `crates/bbb-renderer/src/hud/layout.rs`,
  `crates/bbb-renderer/src/renderer.rs`, and
  `crates/bbb-renderer/src/resource_reload.rs`;
- frame integration:
  `crates/bbb-native/src/runtime/render_extract.rs`;
- coverage and gap ownership:
  `scripts/data/packet-coverage-overrides-26.1.json`,
  `docs/packet-coverage-26.1.json`,
  `crates/bbb-protocol/src/packet_coverage.rs`, and
  `docs/unsupported-features.md`.

Frozen values and calls:

- `WorldStore` exposes one narrow read-only query for a UUID waypoint's
  qualifying interpolated entity eye position. UUID lookup and current-block
  Manhattan gating stay inside `bbb-world`; no entity map is exposed;
- `HudLocatorBar` owns ordered `HudLocatorMarker` values. Each marker carries
  angle, pitch direction, canonical distance, style id, and final opaque ARGB
  tint; it carries no GPU handle or decoded image;
- `HudContextualBar` is the one frame enum for `Empty`, `Experience`, `Locator`,
  and `Jump`. `RendererFrame` replaces the two independently selected
  jump/experience progress fields with that enum, while keeping experience
  level separate;
- `HudRenderRuntime` owns the presentation-only experience display clock and
  last priority-screen identity, resets them at client-level generation
  changes, and returns one `HudContextualBar` per frame;
- `Renderer` owns `HudLocatorStyle` definitions, fixed locator sprites, and a
  marker-sprite map. HUD loading installs them through
  `upload_hud_locator_background`, `upload_hud_locator_arrow_up`,
  `upload_hud_locator_arrow_down`, `upload_hud_locator_marker_sprite`, and
  `set_hud_locator_styles`;
- renderer resource reload checkpoints retain and restore every locator owner,
  so no mixed style/sprite generation can escape a failed reload; and
- world-to-renderer data continues to cross only the single `RendererFrame`
  submission.

Fallback behavior:

- absent camera entity/pose yields an empty marker list but does not suppress
  the locator background when tracked waypoints exist;
- `empty` data and non-finite yaw are not drawn;
- unknown or malformed style ids resolve to `WaypointStyle::missing`;
- an absent referenced GUI sprite resolves to the GUI atlas
  `minecraft:missingno` image;
- malformed angles/distances are sanitized away before geometry; and
- unavailable experience or jump state follows `nextContextualInfoState`
  exactly rather than inventing a bar.

## Vanilla 26.1 evidence

- `ClientWaypointManager.trackWaypoint`, `updateWaypoint`,
  `untrackWaypoint`, `hasWaypoints`, and `forEachWaypoint` define identity
  replacement, in-place same-type updates, membership, and reversed
  distance-squared draw ordering.
- `TrackedWaypoint.EmptyWaypoint`, `Vec3iWaypoint`, `ChunkWaypoint`, and
  `AzimuthWaypoint` define distance, target position, yaw, pitch, and update
  behavior. `Vec3iWaypoint.position` contains the UUID entity-follow and
  Manhattan-distance-3 rule.
- `GameRenderer.projectPointToScreen` uses the main camera's
  view-rotation-projection matrix; `projectHorizonToScreen` uses
  `tan(xRot) / tan(fov / 2)` with infinite endpoints at +/-90 degrees.
- `LocatorBarRenderer.extractBackground` and `extractRenderState` define the
  sprite ids, sizes, visible angle interval, 173-pixel travel, pitch offsets,
  self-UUID suppression, style selection, tint fallback, and draw order.
- `WaypointStyle`, `WaypointStyleManager`, and the two official
  `waypoint_style/*.json` resources define default distances, distance-index
  selection, pack override behavior, and missing-style fallback.
- `Gui.extractHotbarAndDecorations`,
  `willPrioritizeExperienceInfo`, `willPrioritizeJumpInfo`, and
  `nextContextualInfoState` define contextual selection and the independent
  experience-level draw.
- `LocalPlayer.setExperienceValues`, `AnvilScreen.init`, and
  `EnchantmentScreen.init` define the display-start clock, including the
  sentinel first synchronization.

## Serial stages and focused tests

The pre-start checkpoint is clean commit `71e964769`; its full gate, playable
fixture smoke, and P0 audit passed. The live playable gate was unavailable only
because the external `BBB_PLAYABLE_JAVA` prerequisite was absent.

### Stage 1: canonical entity query and pure locator projection

- add the narrow UUID waypoint eye-position query;
- add the dedicated native waypoint projection module;
- implement target positions, distances, wrapped yaw, point/horizon pitch,
  camera-UUID filtering, Java hash/brightness color, visibility, and far-first
  ordering; and
- add pure fixtures covering all four data forms, boundary angles, entity
  following/fallback, pitch, hashes/colors, and malformed inputs.

Focused commands:

- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-world waypoint`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-native waypoint_locator`

Commit the buildable stage after the ordinary per-commit checks. Projection
failures remain isolated from GPU/resource integration.

### Stage 2: contextual owner, selected resources, and renderer geometry

- replace independent jump/experience selection with `HudContextualBar`;
- implement the experience display clock, anvil/enchantment init edges, and
  exact contextual priority;
- load and transactionally own locator style/sprite resources;
- select distance sprites and emit background, tinted dots, and untinted pitch
  arrows in far-first order; and
- add resource override/missing fallback, priority-state, frame-integration,
  and actual `collect_hud_draws` geometry tests.

Focused commands:

- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-pack waypoint_style`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-native waypoint_locator`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-native contextual`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-renderer locator`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-renderer contextual`

Commit the stage clean, then run `scripts/cargo-dev.sh gate` as the planned
cross-owner checkpoint before documentation closeout.

### Stage 3: coverage and milestone closeout

- mark `CLIENTBOUND_WAYPOINT.runtime_side_effect` complete with semantic
  evidence and keep the reverse packet-coverage contract green;
- remove only the closed waypoint item from the unsupported ledger;
- move this contract to `docs/archive/` and append a concise milestone index
  entry; and
- run generated-ledger, docs, format, and focused regression checks.

Focused commands:

- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-protocol waypoint`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main scripts/cargo-dev.sh test -p bbb-protocol packet_coverage`
- `scripts/vanilla-source-tools.sh scripts/generate-packet-catalog.py --check`
- `scripts/docs-lint.sh`
- `cargo fmt --all -- --check`

After the clean closeout commit, run `scripts/cargo-dev.sh gate`,
`scripts/playable-gate.sh live`, `scripts/playable-gate.sh auto`, and
`scripts/audit-p0.sh`. A missing external Java/display/server prerequisite may
leave only the live portion not run; the fixture smoke, full gate, and P0 audit
must pass at the delivered HEAD.

## Failure localization and safe stopping

- Stage 1 failures are confined to canonical entity lookup or pure camera/
  waypoint projection.
- Stage 2 failures are confined to contextual presentation state, selected
  resource ownership, frame wiring, or HUD geometry.
- Stage 3 failures are confined to coverage and documentation closeout.
- Every stage ends in a buildable reviewed commit. Failures are fixed forward;
  no commit or history rewrite is allowed.
- Safe stopping points are the clean stage commits after their required
  focused checks and checkpoint gate. No unrelated slice starts before this
  contract closes or reaches a genuine blocker.
