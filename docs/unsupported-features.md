# Unsupported And Deferred Feature Ledger

This is the project ledger for native Minecraft Java 26.1 features that are
known to be unsupported, partial, diagnostic-only, or intentionally deferred.
Each remaining item must have an owner, status, and next action before the
rewrite can be considered complete.

When an agent does any of the following, update this file in the same slice:

- Adds a new unsupported behavior.
- Preserves a diagnostic-only path.
- Discovers a vanilla feature gap that cannot be closed in the current slice.

## Status Key

- `covered`: implemented for the currently claimed scope; keep future work
  derived from the documented owner.
- `partial`: some behavior exists, but vanilla parity is not complete.
- `diagnostic`: unsupported input is intentionally reported, not implemented.
- `deferred`: intentionally left for a later owner because the current runtime
  surface is not ready.

## Ledger

### Unknown Clientbound Packets In Login, Configuration, And Play

- Owner: `bbb-protocol` + `bbb-net` + `bbb-native`
- Status: `diagnostic`
- Next action:
  - When an unsupported packet appears in probe/control diagnostics, verify it
    against local vanilla 26.1 sources.
  - Then either:
    - implement protocol decode plus world/runtime handling
    - record why it is runtime-only
- Evidence / boundary:
  - Unknown login/config/play packets:
    - preserve `packet_id` and `len`
    - emit `NetEvent::UnsupportedPacket`
    - project into `NetCounters` / `ProbeReport`

### Protocol Coverage For Remaining Required 26.1 Packet Families

- Owner: `bbb-protocol`
- Status: `partial`
- Next action:
  - Continue auditing protocol details against
    `<MC_CODE_ROOT>/sources/26.1/`:
    - packet ids
    - field order
    - nullability
    - enum ordinals
    - serverbound encoders
  - Add focused encode/decode tests with each packet slice.
- Evidence / boundary:
  - Phase 2 in `docs/full-native-rewrite-plan.md` remains open until required
    paths are covered:
    - login
    - configuration
    - play
    - movement
    - inventory
    - chat
    - resource-pack
    - interaction
    - command suggestion

### Offline Probe And Online Dispatcher Parity

- Owner: `bbb-net` + `bbb-native` + `bbb-world`
- Status: `partial`
- Next action:
  - Keep adding parity regression tests for every decoded packet that has a
    `WorldStore` apply API.
  - Prefer shared semantics or focused paired tests when probe and online
    handling differ.
- Evidence / boundary:
  - Probe and online paths now cover many packet families, including:
    - unsupported diagnostics
    - play -> configuration teardown
  - The final criterion:
    - every supported decoded packet stays aligned

### Native-Owned Business Snapshots

- Owner: `bbb-world` + `bbb-native` + `bbb-control`
- Status: `partial`
- Next action:
  - Continue moving client-observable state into `WorldStore`.
  - Keep `NetCounters` for connection/runtime status and command queue
    projections only.
- Evidence / boundary:
  - The architecture plan still calls out removal of native-only `last_*`
    snapshots where a world owner exists or should exist.

### Code Of Conduct Presentation

- Owner: `bbb-world` + `bbb-native` + `bbb-renderer`
- Status: `deferred`
- Next action:
  - Replace the native bitmap prompt with fuller vanilla screen/font rendering
    when the renderer UI stack is mature.
- Evidence / boundary:
  - Canonical Code of Conduct UI state and control requests are covered.
  - Presentation parity is explicitly deferred in
    `docs/full-native-rewrite-plan.md`.

### Crosshair Entity Interaction Parity

- Owner: `bbb-world` + `bbb-native` + `bbb-renderer`
- Status: `partial`
- Next action:
  - Validate any future `yRotA` source.
  - Replace the debug target overlay with full entity model rendering and
    interaction feedback when renderer entity presentation exists.
- Evidence / boundary:
  - `bbb-world` and `bbb-native` expose many verified pick bounds and
    interaction packets.
  - Native projects the current crosshair entity pick target into a
    renderer-visible wire outline.
  - The outline uses the same pick AABB as raycast selection.

### Particle Runtime Vanilla Parity

- Owner: `bbb-renderer` + `bbb-native` + `bbb-pack`
- Status: `partial`
- Next action:
  - Implement remaining renderer slices for:
    - provider-specific behavior
    - light curves
    - particle sorting
    - collision/player-coupled physics
    - particle limits/settings
    - atlas mip animation
    - terrain/item particle option rendering
  - Preserve missing definition/sprite diagnostics.
- Evidence / boundary:
  - Current runtime:
    - Drains level-particle spawn batches.
    - Advances CPU-side common particles.
    - Samples vanilla-shaped curves for common particle providers:
      - size
      - color
      - age-size
    - Uploads a stitched official particle atlas when assets are available.
    - Draws active particles as camera-facing textured billboards.
  - Follow-up work in the plan:
    - full vanilla provider behavior
    - presentation parity

### Renderer Scene Parity

- Owner: `bbb-renderer` + `bbb-native` + `bbb-pack` + `bbb-world`
- Status: `partial`
- Next action:
  - Replace proxies with full extraction from canonical world and pack data:
    - entity bounds
    - dropped-item icons
  - Extract complete entity presentation data:
    - model
    - equipment
    - skin
    - lighting
    - animation
    - culling
    - ordering
  - Implement vanilla dropped-item follow-up rendering:
    - ground-context model rendering
    - bobbing
    - Y spin
    - count-based multiple copies
    - lighting
  - Continue renderer presentation work with deterministic tests or explicit
    manual comparison notes:
    - HUD
    - overlays
    - screenshots
    - interaction feedback
- Evidence / boundary:
  - Renderer draws:
    - terrain
    - HUD
    - particles
    - selection/block-destroy overlays
    - crosshair entity target outlines
    - a basic hecs-derived entity bounds scene proxy
    - dropped item entities as camera-facing item-icon billboards from:
      - canonical item entity stack metadata
      - the native item atlas
  - Backend GPU resources stay outside `WorldStore`.
  - Full entity presentation remains phase 6 work.

### Audio Runtime Parity

- Owner: `bbb-audio` + `bbb-native` + `bbb-pack` + `bbb-world`
- Status: `partial`
- Next action:
  - Continue validating audio behavior against vanilla without requiring an
    audio device in unit tests:
    - source/category mapping
    - spatial/entity-following sounds
    - stop semantics
    - device/runtime diagnostics
- Evidence / boundary:
  - `bbb-audio` has:
    - Kira-backed command/runtime boundaries
    - pack-driven sound lookup
  - Full vanilla playback parity remains phase 7 work.

### Official 26.1 Resource-Pack Coverage

- Owner: `bbb-pack`
- Status: `partial`
- Next action:
  - Implement unsupported declaration shapes as official assets or resource
    packs require them:
    - atlas
    - item model
    - item tint
    - registry declaration
  - Keep resource-pack precedence/filter tests close to loaders.
- Evidence / boundary:
  - Loaders report unsupported atlas/item declarations.
  - Current audio use is covered by:
    - sounds
    - generated vanilla fallback
    - resource-pack filters

### Bundle Selected-Item Icon State

- Owner: `bbb-protocol` + `bbb-world` + `bbb-native` + `bbb-pack`
- Status: `partial`
- Next action:
  - Connect native bundle slot mouse helpers to screen slot hit-testing when
    that UI exists:
    - inventory
    - container
  - Expand renderer/UI coverage beyond hotbar icon snapshots.
- Evidence / boundary:
  - Vanilla `BundleHasSelectedItem` checks
    `BundleItem.getSelectedItem(itemStack) != null`.
  - `BundleContents.STREAM_CODEC` sends the item template list but not the
    selected index.
  - `bbb-protocol` preserves bundle item-template summaries.
  - `bbb-world` stores the local selected index per inventory/container slot.
  - Control pumping and native bundle mouse helpers:
    - update canonical state
    - queue `ServerboundSelectBundleItemPacket`
  - The GUI item icon runtime:
    - evaluates `minecraft:bundle/has_selected_item`
    - resolves `minecraft:bundle/selected_item` from the selected template

### Native Input, Movement, Interaction, Inventory, And Command Flows

- Owner: `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world`
- Status: `partial`
- Next action:
  - Movement:
    - Extend the current basic AABB collision and gravity/jump slice to cover:
      - full fixed 20Hz survival physics
      - remaining vanilla voxel collision shapes
      - fluids
      - remaining status-effect movement cases beyond local support for:
        - local Speed/Slowness movement-speed modifiers
        - local Jump Boost jump impulse
        - local Slow Falling gravity clamp
        - local Levitation vertical velocity targeting
      - sneak pose details
      - the near-ground/fallDistance branch of sneak edge backoff
      - full flying friction
      - remaining vehicle movement send edge cases beyond the vanilla-shaped
        walking and passenger player packet thresholds
  - Block destroy: close:
    - remaining block destroy profile gaps outside the mechanically parsed
      `Blocks.java` property declarations:
      - constructor-level mutations such as `InfestedBlock`
      - arbitrary helper/lambda evaluation not covered by the current parser
    - remaining vanilla player destroy-speed gaps:
      - validate exact pose/fluid nuance beyond the current standing-eye
        water probe
    - collision-aware rollback position handling
    - hit effects
    - full model-shaped crack decals with vanilla crumbling blend/depth-bias
      behavior
    - any remaining `STOP_DESTROY_BLOCK` sequencing gaps
  - Commands: continue adding focused command queue and encode tests for
    missing flows:
    - inventory
    - interaction
    - chat
    - command
  - Inventory: implement:
    - tooltips
    - item durability/cooldown decorations
    - remaining dedicated server-opened menu layouts beyond:
      - `generic_9xN`
      - `generic_3x3`
      - furnace/blast furnace/smoker
      - hopper
      - shulker box
    - recipe book/creative variants
    - remaining local crafting result parity for container `0`:
      - server-authored result recomputation from local 2x2 inputs
      - recipe-specific remainder items
- Evidence / boundary:
  - Movement:
    - Native movement projects world-computed `on_ground` and
      `horizontal_collision` into serverbound move commands.
    - It clips local player movement with a basic AABB solver against simple
      full-block terrain plus common non-full-block shapes:
      - slab
      - stair
      - door
      - trapdoor
      - fence
      - fence gate
      - bars/pane
      - wall
      - leaves
      - snow layer
      - flat carpet
      - chain
      - ladder
      - rod
      - campfire
      - copper grate
      - chest
      - bed
      - cauldron
      - hopper
      - composter
      - enchanting table
      - stonecutter
      - anvil
    - It applies:
      - synced local player `gravity` attribute id `14` and basic gravity
      - jumps only from ground
      - local player `movement_speed` / `sneaking_speed` attributes with the
        vanilla default sneaking-speed reduction
      - synced local player Speed effect id `0` and Slowness effect id `1`
        as `movement_speed` `ADD_MULTIPLIED_TOTAL` modifiers scaled by
        `amplifier + 1`
      - synced local player Jump Boost effect id `7` as a jump impulse bonus
        of `0.1 * (amplifier + 1)`
      - synced local player Slow Falling effect id `27` as the vanilla falling
        gravity clamp
      - synced local player Levitation effect id `24` as the vanilla vertical
        velocity target
      - basic abilities-driven flying movement with no ordinary gravity
      - jump/sneak vertical controls while flying
      - vanilla 0.6 Y-velocity damping while flying
      - vanilla default 0.6 step-up onto bottom slabs/stairs and low ground
        shapes without auto-stepping full blocks
      - a basic vanilla-shaped sneak edge backoff
      - vanilla-shaped walking player movement packet thresholds:
        - position delta squared greater than `(2.0E-4)^2`
        - rotation/status-only packets when only:
          - look changes
          - collision flags change
        - 20 tick position reminder that resets only when a position packet is
          sent
      - vanilla-shaped passenger `MovePlayer.Rot` packets while mounted:
        - sent on the local movement tick even when look did not change
        - forced to the Rot-only packet variant instead of position/status
          variants
  - Commands:
    - Existing input modules queue many serverbound packets, including
      vanilla-shaped boat/raft paddle-state packets from local mounted input.
    - They queue `START_RIDING_JUMP` player commands for vanilla
      `PlayerRideableJumping` vehicle types using:
      - the 26.1 charge scale
      - jump release
    - They queue `START_FALL_FLYING` player commands when an airborne local
      player has an elytra-equipped chest slot.
    - They queue `STOP_SLEEPING` player commands when wake-up input is pressed
      while the local player entity has sleeping pose metadata.
    - Chat entry paths:
      - send offline unsigned `ServerboundChatPacket` messages
      - request `ServerboundCommandSuggestionPacket` completions:
        - with the leading slash
        - while typing slash commands
      - submit `ServerboundChatCommandPacket` payloads without:
        - the leading slash
      - queue explicit `ServerboundClientCommandPacket` commands:
        - perform-respawn
        - from native/control input
      - avoid auto-respawning on dead health
  - Inventory:
    - Native opens the ordinary local inventory as container `0`.
    - While the local inventory is open, it:
      - releases cursor capture
      - closes with E/Esc by queueing `ServerboundContainerClosePacket(0)`
    - For container `0`, it:
      - renders the centered vanilla survival inventory background with item
        icons and slot hover highlights
      - hit-tests the fixed slot layout
      - routes left/right pickup and outside-drop clicks through a basic local
        `PICKUP` simulation
      - routes Shift-click slots through a local `QUICK_MOVE` simulation for:
        - armor/offhand auto-equip from official item default equipment slots
        - main-inventory/hotbar/container-zero ranges
        - repeated crafting result movement while the result stack remains the
          same item/components and the original input slots can still be
          consumed
        - vanilla-shaped result-to-player transfer order
      - routes hovered-slot Q/Ctrl+Q through a basic local `THROW` simulation
      - routes hovered-slot number/F keys through a basic local `SWAP`
        simulation
      - routes rapid same-slot left double-clicks through a basic local
        `PICKUP_ALL` simulation
      - routes local left/right drag distribution through vanilla-shaped
        `QUICK_CRAFT` start/add/end clicks
    - It also:
      - renders stack count labels:
        - for hotbar item icons
        - for local inventory item icons
        - using official 26.1 `font/ascii.png` digit glyphs
        - with vanilla item-count placement
      - updates cursor/slot state
      - fills `ServerboundContainerClickPacket(0)` changed-slot hashes
      - supports bundle wheel selection on hovered local inventory slots
    - It renders and hit-tests supported server-opened screens:
      - `generic_9x1` through `generic_9x6` ChestMenu screens with official
        `generic_54.png` background slices
      - `generic_3x3` DispenserMenu screens with official `dispenser.png`
      - FurnaceMenu/BlastFurnaceMenu/SmokerMenu screens with official
        furnace-family backgrounds
      - HopperMenu screens with official `hopper.png`
      - ShulkerBoxMenu screens with official `shulker_box.png`
    - FurnaceMenu/BlastFurnaceMenu/SmokerMenu screens also render official
      progress sprites:
      - lit-progress
      - burn-progress
    - Those progress sprites use:
      - canonical `ContainerSetData` values
      - vanilla `AbstractFurnaceMenu` progress formulas
    - It queues basic left/right `PICKUP` container clicks for those supported
      fixed-slot screens.
    - It also queues Shift-click `QUICK_MOVE` container clicks for:
      - supported generic containers
      - `generic_3x3`
      - hopper
      - shulker box
      - FurnaceMenu/BlastFurnaceMenu/SmokerMenu, including:
        - vanilla slot ranges
        - result-to-player transfer order
        - recipe-property-set input routing
        - official `FuelValues` fuel routing when vanilla assets are loaded
    - Control/native can still build a basic `ServerboundContainerClickPacket`
      from the active container id, state id, slot id, and cursor item for
      server-opened containers when the carried stack is hash-safe.
  - Block destroy:
    - Native block destroy progress records the starting main-hand item
      signature and restarts the destroy sequence when the selected
      item/components change.
    - Default item mining profiles are derived from official 26.1 item
      declarations and block tags for vanilla tool-like items:
      - pickaxe
      - axe
      - hoe
      - shovel
      - sword
      - shears
    - Default block destroy profiles are derived from official 26.1
      `Blocks.java` declarations for:
      - direct `strength` / `destroyTime` / `instabreak` chains
      - `requiresCorrectToolForDrops`
      - `ofLegacyCopy` / `ofFullCopy` inheritance
      - common helper registrations for logs, stems, leaves, buttons, flower
        pots, candles, beds, stained glass, shulker boxes, pistons, and stairs
    - Local destroy progress applies:
      - selected main-hand item profile
      - vanilla-shaped mining speed
      - `correct_for_drops` rule order
    - It applies synced local player destroy-speed state:
      - `mining_efficiency` attribute id `20` when item speed is above `1`
      - Haste effect id `2` and Conduit Power effect id `28`, using the max
        amplifier
      - Mining Fatigue effect id `3` with vanilla scale
      - `block_break_speed` attribute id `5`
      - `submerged_mining_speed` attribute id `29` when the local player's
        standing eye position is in water
      - airborne slowdown
    - Vanilla Efficiency and Aqua Affinity affect destroy speed through
      synced attributes, not local component fallback:
      - Efficiency contributes to `mining_efficiency`.
      - Aqua Affinity contributes to `submerged_mining_speed`.
      - Native keeps `UpdateAttributes` as the authoritative path.
    - It tracks vanilla-shaped local destroy stages in canonical interaction
      state and clears them on completion/abort/restart.
    - It projects destroy progress to batched renderer-visible cube crack
      overlays from:
      - local stages
      - server `BlockDestruction` progress
    - Those overlays use:
      - official `destroy_stage_0..9` block atlas textures
      - keeping the highest stage when multiple overlays target one block
        position
      - expiring server destruction entries after:
        - the vanilla-shaped 400 render tick window
    - It predicts the locally destroyed block before queuing:
      - stop destroy packets
      - instant destroy packets
    - Prediction targets:
      - air
      - legacy water/lava state
    - It reconciles local block-destroy predictions by:
      - deferring server block updates into pending prediction state
      - resolving predictions on `BlockChangedAck`
  - Completion requires full vanilla movement and these flows to work through
    encoded serverbound packets end to end.

### Signed Chat And Chat Acknowledgement Production

- Owner: `bbb-protocol` + `bbb-net` + `bbb-world` + `bbb-native`
- Status: `partial`
- Next action:
  - Implement signed chat/chat-command last-seen updates and any remaining
    vanilla last-seen message entries needed for outbound signed payloads.
- Evidence / boundary:
  - Covered pieces:
    - `ServerboundChatAckPacket` id 6 and VarInt `offset` encoding
    - offline unsigned `ServerboundChatPacket` encoding/sending
    - `NetCommand::ChatAcknowledgement` sending
    - canonical processed-signature offset tracking
    - online drain queueing
    - offline probe ack sending after vanilla's `offset > 64` threshold
  - Full signed chat payload generation remains follow-up work.

### Manual Visual/Audio Comparisons

- Owner: relevant runtime owner
- Status: `deferred`
- Next action:
  - Whenever visual or audio behavior cannot be proven by automated tests,
    record the required proof to close the slice:
    - vanilla source path
    - asset path
    - screenshot
    - smoke test
    - manual comparison
- Evidence / boundary:
  - The project gate allows manual or screenshot/audio smoke checks outside
    normal unit tests, but they must be documented when required.

Mounted boat input now has a basic locally authoritative path:

- Updates local look while mounted.
- Advances a simple root-boat transform from local input.
- Queues both paddle-state and `MoveVehicle` commands.
- Leaves the following covered by the native input/movement ledger row above:
  - full vanilla boat physics
  - water/buoyancy/collision parity
  - non-boat vehicle movement

## Update Rules

- Do not remove a row unless the current slice proves the feature is fully
  covered for its stated scope and the proof is referenced in code or tests.
- Prefer splitting a broad row into narrower rows as soon as a feature has a
  concrete owner and testable next action.
- Keep rows scoped to semantic ownership. Do not add arbitrary line-count or
  agent-parallelism work here.
