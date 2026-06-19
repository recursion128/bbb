# Unsupported And Deferred Feature Ledger

This is the project ledger for native Minecraft Java 26.1 features known to be:

- unsupported
- partial
- diagnostic-only
- intentionally deferred

Each remaining item must have an owner, status, and next action before the
rewrite can be considered complete.

When an agent does any of the following, update this file in the same slice:

- Adds a new unsupported behavior.
- Preserves a diagnostic-only path.
- Discovers a vanilla feature gap that cannot be closed in the current slice.

## Status Key

- `covered`:
  - implemented for the currently claimed scope
  - future work stays derived from the documented owner
- `partial`:
  - some behavior exists
  - vanilla parity is not complete
- `diagnostic`:
  - unsupported input is intentionally reported
  - unsupported input is not implemented
- `deferred`:
  - intentionally left for a later owner
  - current runtime surface is not ready

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
    - custom payload brand/unknown presentation state
    - item cooldown add/remove state
    - delete/disguised chat state
    - entity motion/head rotation/animation/hurt transient state
    - scoreboard objective/display/score/team updates
    - advancement update/select-tab state
    - recipe book add/remove/settings updates
    - recipe access `UpdateRecipes` property-set and stonecutter updates
    - world border initialize/center/size/warning updates
    - level chunk insert, block/section updates, chunk view, and chunk forgets
    - terrain light and biome updates
    - inventory, container, cursor, and merchant-offer updates
    - entity position, metadata, equipment, attributes, status, link,
      passenger, take-item, and remove updates
    - minecart along-track lerp updates
  - The final criterion:
    - every supported decoded packet stays aligned

### Native-Owned Business Snapshots

- Owner: `bbb-world` + `bbb-native` + `bbb-control`
- Status: `partial`
- Next action:
  - Continue moving client-observable state into `WorldStore`.
  - Keep `NetCounters` only for:
    - connection/runtime status
    - command queue projections
- Evidence / boundary:
  - The architecture plan still calls out removing native-only `last_*`
    snapshots when:
    - a world owner exists
    - a world owner should exist

### Code Of Conduct Presentation

- Owner: `bbb-world` + `bbb-native` + `bbb-renderer`
- Status: `deferred`
- Next action:
  - Replace the native bitmap prompt when the renderer UI stack is mature with:
    - fuller vanilla screen rendering
    - fuller vanilla font rendering
- Evidence / boundary:
  - Canonical Code of Conduct UI state and control requests are covered.
  - Presentation parity is explicitly deferred in
    `docs/full-native-rewrite-plan.md`.

### Crosshair Entity Interaction Parity

- Owner: `bbb-world` + `bbb-native` + `bbb-renderer`
- Status: `partial`
- Next action:
  - Validate any future `yRotA` source.
  - Replace the debug target overlay when renderer entity presentation exists
    with:
    - full entity model rendering
    - interaction feedback
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
  - Continue validating audio behavior against vanilla.
  - Keep unit tests independent of an audio device.
  - Cover:
    - source/category mapping
    - spatial/entity-following sounds
    - stop semantics
    - remaining level-event audio:
      - jukebox song start/stop
      - portal local ambience
      - particle-coupled level-event side effects
    - device/runtime diagnostics
- Evidence / boundary:
  - `bbb-audio` has:
    - Kira-backed command/runtime boundaries
    - pack-driven sound lookup
    - native dispatcher playback for fixed-pitch vanilla `LevelEventHandler`
      sounds:
      - dispenser dispense/fail/launch
      - firework rocket shoot
      - chorus flower grow/death
      - brewing stand brew
      - crafter craft/fail
      - end portal frame fill
      - bone meal use
    - native dispatcher playback for randomized vanilla `LevelEventHandler`
      sounds using a runtime-local `LegacyRandomSource`-shaped `nextFloat()`:
      - fire extinguish / generic extinguish
      - ghast/blaze/dragon/wither/zombie/skeleton/phantom hostile effects
      - anvil, grindstone, book, smithing table, dripstone, wind charge
      - lava extinguish and redstone torch burnout sounds
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
    - clear selected bundle items before `QUICK_MOVE` and `SWAP`
      container clicks
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
      - remaining fluid movement work beyond current still water/lava support:
        - sprint-swim camera, animation, and presentation nuance beyond
          canonical swimming pose selection and pitch-coupled vertical travel
      - remaining block-state movement factors beyond the synced frozen-tick
        powder-snow slowdown:
        - in-world powder snow contact freezing/unfreezing tick updates
        - full vanilla post-`Entity.move` `deltaMovement` travel ordering
          beyond the current direct local walking prediction
      - remaining vehicle movement send edge cases beyond the vanilla-shaped
        walking and passenger player packet thresholds
  - Block destroy:
    - Close remaining block destroy profile gaps outside the mechanically
      parsed `Blocks.java` property declarations:
      - constructor-level mutations beyond the covered `InfestedBlock` /
        `InfestedRotatedPillarBlock` host destroy-time rule
      - arbitrary helper/lambda evaluation not covered by the current parser
    - Close remaining vanilla player destroy-speed gaps:
      - validate exact pose/fluid nuance beyond the current standing-eye
        water probe
    - remaining hit effects beyond local block hit/break sounds:
      - block-specific `state.attack` callbacks
      - hit particles
    - full model-shaped crack decals with vanilla crumbling blend/depth-bias
      behavior
    - any remaining `STOP_DESTROY_BLOCK` sequencing gaps
  - Commands:
    - Continue adding focused command queue and encode tests for:
      - inventory
      - interaction
      - chat
      - command
      - sign editing
    - Sign editing follow-up work:
      - renderer presentation for the vanilla sign edit screen
      - selection and clipboard parity once modifier-aware sign editor key
        events are available
    - Gameplay Q/Ctrl+Q drop input now follows vanilla 26.1 modifier
      semantics:
      - Ctrl, not sprint, selects `DROP_ALL_ITEMS`.
      - The selected hotbar stack is locally predicted in canonical inventory
        state.
      - Main-hand swing is queued only when a non-empty stack was dropped.
      - Spectator mode suppresses gameplay drop and swap-offhand actions.
      - Spectator left-click on an entity queues `SpectateEntity`; spectator
        left-click on blocks does not attack, destroy, or swing.
      - Middle-click pick block/entity uses Ctrl, not sprint, for include-data.
      - Spectator hotbar number keys and wheel do not send held-slot packets;
        wheel input adjusts local flying speed when the Spectator GUI menu is
        inactive. Full Spectator GUI selection/menu behavior remains follow-up
        presentation/control work.
      - Spectator mode automatically enables local flying when server-synced
        abilities allow flight, and jump double-tap does not toggle spectator
        flying off.
      - Spectator right-click without a target does not send `UseItem`.
      - Spectator right-click on a block sends main-hand `UseItemOn` even when
        offhand would be preferred for non-spectator item use.
  - Inventory:
    - Implement:
      - remaining rich tooltip behavior:
        - non-ASCII font providers
        - bidirectional text shaping
        - official tooltip background/frame sprites
        - italic and complex component styles
        - component-specific detail lines
      - remaining dedicated server-opened menu layouts beyond:
        - `generic_9xN`
        - `generic_3x3`
        - anvil
        - beacon
        - brewing stand
        - cartography table
        - enchanting table
        - furnace/blast furnace/smoker
        - crafting table
        - grindstone
        - hopper
        - lectern
        - loom
        - merchant/villager
        - shulker box
        - smithing table
        - stonecutter
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
      - cactus
      - farmland / dirt path
      - soul sand / mud
      - honey block
      - cake
      - lily pad
      - flower pot
      - candle / candle cake
      - sea pickle
      - skull / wall skull
      - turtle egg
      - sniffer egg
      - dried ghast
      - chain
      - ladder
      - rod
      - campfire
      - copper grate
      - end portal frame
      - daylight detector
      - sculk sensor / calibrated sculk sensor / sculk shrieker
      - heavy core
      - dragon egg
      - decorated pot
      - chest
      - bed
      - cauldron
      - hopper
      - composter
      - lectern
      - grindstone
      - brewing stand
      - bell
      - enchanting table
      - stonecutter
      - anvil
    - It applies:
      - synced local player `gravity` attribute id `14` and basic gravity
      - synced local player `NoGravity` entity metadata data id `5`, which
        suppresses local gravity in air, water, and lava travel
      - synced local player `TicksFrozen` entity metadata data id `7` as the
        vanilla `minecraft:powder_snow` movement-speed `ADD_VALUE` modifier:
        - amount is `-0.05 * min(ticks_frozen, 140) / 140`
        - skipped when the server already syncs the `minecraft:powder_snow`
          movement-speed modifier
      - vanilla block speed factor for local walking prediction:
        - `minecraft:soul_sand` and `minecraft:honey_block` apply `0.4`
        - synced local player `movement_efficiency` attribute id `21`
          interpolates the block factor back toward `1.0`
        - `minecraft:water` and `minecraft:bubble_column` at the current
          player block position do not fall back to the block below
      - vanilla block jump factor for local jumps:
        - `minecraft:honey_block` applies `0.5` to the base jump strength
        - Jump Boost remains additive after the block jump factor
      - jumps only from ground
      - vanilla sprint jump horizontal impulse:
        - adds `0.2` horizontal velocity in the local yaw forward direction
        - uses the same effective sprint predicate as local movement
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
      - synced local player Blindness effect id `14` as the vanilla mobility
        restriction that prevents sprinting
      - vanilla local sprint eligibility for player food and forward impulse:
        - requires food level above `6`
        - treats synced `mayfly` / `can_fly` abilities as enough food
        - suppresses local sprint speed, sprint-swim pose, and sprinting fluid
          drag when sprinting is not eligible
      - abilities-driven flying movement with no ordinary gravity
      - vanilla flying horizontal input:
        - uses synced abilities `flying_speed` as per-tick acceleration
        - doubles acceleration while sprinting
        - applies vanilla `0.91` post-move air drag to horizontal velocity
      - jump/sneak vertical controls while flying
      - vanilla 0.6 Y-velocity damping while flying
      - vanilla default 0.6 step-up onto bottom slabs/stairs and low ground
        shapes without auto-stepping full blocks
      - a basic vanilla-shaped sneak edge backoff
      - the vanilla near-ground `fallDistance < maxUpStep` branch of sneak edge
        backoff, backed by canonical local pose `fall_distance`
      - canonical local player `sneaking` pose state when:
        - focused sneak is active and the player is not flying
        - a low ceiling blocks standing bounds but allows crouching bounds
      - canonical local player `swimming` visual pose state when:
        - sprinting while underwater in water contact
        - standing and crouching bounds are blocked but swimming bounds fit
      - vanilla-shaped sprint-swim vertical travel:
        - adjusts water swimming Y velocity toward `getLookAngle().y`
        - uses approach `0.085` when looking down below `-0.2`
        - otherwise uses approach `0.06`
        - upward look only auto-rises while jumping or when fluid remains at
          the vanilla `y + 0.9` swim-head probe
      - vanilla local player standing/crouching/swimming eye heights:
        - standing `1.62`
        - crouching `1.27`
        - swimming/crawling `0.4`
      - vanilla local player standing/crouching/swimming body heights:
        - standing `1.8`
        - crouching `1.5`
        - swimming/crawling `0.6`
      - local player collision, step-up, support probing, fluid body contact,
        fluid jump-out clearance, and bubble-column contact all consume the
        canonical local body height
      - local player camera pose, audio listener, crosshair ray, fluid eye probing,
        and `LookAt` eye anchor all consume the canonical local eye height
      - vanilla-shaped local player fluid contact probing for water/lava
        height and eye-in-fluid checks:
        - `FlowingFluid` height uses `amount / 9.0`
        - same-kind fluid above makes the lower fluid column full height
        - the local player fluid interaction box is deflated by `0.001`
      - water contact resets canonical local player `fall_distance`
      - basic still-fluid travel for local players affected by fluids:
        - water and lava `moveRelative(0.02)` acceleration
        - water drag `0.8`
        - sprinting-water drag `0.9`
        - lava drag `0.5`
        - liquid jump impulse `+0.04`
        - water sneak descent impulse `-0.04`
        - vanilla-shaped fluid gravity scaling
        - creative/spectator flying movement ignores fluid travel
        - water movement efficiency attribute id `32`:
          - interpolates water input speed toward local player movement speed
          - interpolates horizontal water drag toward `0.54600006`
          - applies at half strength when not on ground
        - Dolphin's Grace effect id `29` overrides horizontal water drag to
          `0.96`
        - water/lava `jumpOutOfFluid(oldY)` collision assist:
          - checks post-move `horizontalCollision`
          - probes the current player bounds offset by:
            `delta_movement.y + 0.6 - current_y + old_y`
          - requires the probed bounds to be free of block collision and any
            fluid block
          - sets vertical velocity to `0.3` when clear
        - `minecraft:bubble_column` local player velocity:
          - reads vanilla `drag` block-state property
          - inside upward columns add `0.06` Y velocity capped at `0.7`
          - above upward columns add `0.1` Y velocity capped at `1.8`
          - inside drag-down columns subtract `0.03` Y velocity capped at
            `-0.3`
          - above drag-down columns subtract `0.03` Y velocity capped at
            `-0.9`
          - players with flying abilities active ignore the bubble-column
            push, matching vanilla `Player` overrides
      - vanilla-shaped flow current push for local players affected by fluids:
        - `FlowingFluid.getFlow` horizontal own-height gradients
        - empty-neighbor falling-hole lookup with the vanilla `8/9` offset
        - falling-fluid downward current against solid neighbor faces
        - per-fluid accumulated current averaged for players
        - low contact height `< 0.4` current scaling
        - water current scale `0.014`
        - normal lava current scale `0.0023333333333333335`
        - Nether fast-lava current scale `0.007`
        - vanilla minimum current push `0.0045` when horizontal velocity is
          below `0.003`
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
    - Native sprint command queuing is derived from the same world-owned
      effective sprint predicate used by local movement:
      - forward impulse is required before `START_SPRINTING`
      - low food suppresses `START_SPRINTING`
      - releasing focus or input queues `STOP_SPRINTING` when effective sprint
        was active
    - Existing input modules queue many serverbound packets, including
      vanilla-shaped boat/raft paddle-state packets from local mounted input.
    - They queue `START_RIDING_JUMP` player commands for vanilla
      `PlayerRideableJumping` vehicle types using:
      - the 26.1 charge scale
      - jump release
    - They toggle local creative/spectator-style flying with the vanilla
      double-jump window when synced abilities allow flight, then queue
      `ServerboundPlayerAbilitiesPacket`.
    - Creative middle-click clone in local and server-opened containers queues
      `ServerboundContainerClickPacket` with `ContainerInput.CLONE`, an empty
      changed-slot set, and a carried item copied to the vanilla max stack
      size.
    - They queue `START_FALL_FLYING` player commands when an airborne local
      player has an elytra-equipped chest slot.
    - They queue `STOP_SLEEPING` player commands when wake-up input is pressed
      while the local player entity has sleeping pose metadata.
    - They queue `OPEN_INVENTORY` player commands instead of opening local
      inventory when the local player is riding a vanilla
      `HasCustomInventoryScreen` vehicle:
      - `AbstractHorse`
      - `AbstractNautilus`
      - `AbstractChestBoat`
    - Block-target right-click queues `ServerboundUseItemOnPacket` with:
      - main hand when the selected hotbar slot is non-empty
      - offhand when the selected hotbar slot is empty and offhand has an item
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
    - Sign editor input paths:
      - open from `ClientboundOpenSignEditorPacket`
      - initialize from canonical sign block entity front/back text when
        available
      - edit four pending lines
      - move the line cursor with left/right/home/end and edit at the cursor
        with text input/backspace/delete
      - cycle lines with vanilla-shaped up/down/confirmation keys
      - close by queueing `ServerboundSignUpdatePacket`
      - release cursor capture and suppress gameplay mouse input while open
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
      - renders vanilla-shaped durability bars for item icons with
        `damage` / `max_damage` component summaries:
        - in the hotbar
        - in local inventory screens
        - in supported server-opened screens
      - renders vanilla-shaped cooldown overlays for item icons from canonical
        cooldown groups and client tick progress:
        - in the hotbar
        - in local inventory screens
        - in supported server-opened screens
      - updates cursor/slot state
      - fills `ServerboundContainerClickPacket(0)` changed-slot hashes
      - supports bundle wheel selection on hovered local inventory slots
      - clears selected bundle items before local and server-opened
        `QUICK_MOVE` / `SWAP` container clicks
      - projects hovered-slot default item tooltip names from official language
        assets and protocol item ids
      - projects hovered-slot custom tooltip names from decoded
        `custom_name`, `item_name`, and written-book title component summaries
      - projects decoded lore component summaries as basic tooltip lines
      - applies vanilla-shaped base tooltip colors for:
        - rarity-colored item names
        - enchanted rarity upgrades
        - lore dark-purple lines
        - written-book author and generation detail lines
        - unbreakable component detail line
      - renders hovered-slot item tooltip names as basic ASCII text using
        official 26.1 `font/ascii.png` glyphs
    - It renders and hit-tests supported server-opened screens:
      - `generic_9x1` through `generic_9x6` ChestMenu screens with official
        `generic_54.png` background slices
      - `generic_3x3` DispenserMenu screens with official `dispenser.png`
      - CrafterMenu screens with official:
        - `crafter.png` background
        - `container/crafter/disabled_slot`
        - powered/unpowered redstone sprites
      - AnvilMenu screens with official:
        - `anvil.png` background
        - `container/anvil/text_field`
        - `text_field_disabled`
        - error sprites
        - rename text input that queues `ServerboundRenameItemPacket`
      - BeaconMenu screens with official:
        - `beacon.png` background
        - payment slot layout
        - confirm/cancel button sprites
        - beacon effect button sprites loaded for future effect selection
      - BrewingStandMenu screens with official `brewing_stand.png`
      - CartographyTableMenu screens with official:
        - `cartography_table.png` background
        - result map sprite
        - scaled-map sprite
        - duplicated-map sprites
        - locked-map overlay sprite
        - invalid-transform error sprite
      - CraftingMenu screens with official `crafting_table.png`
      - EnchantmentMenu screens with official:
        - `enchanting_table.png` background
        - empty lapis slot sprite
        - enchantment option slot sprites
        - level cost sprites
      - FurnaceMenu/BlastFurnaceMenu/SmokerMenu screens with official
        furnace-family backgrounds
      - GrindstoneMenu screens with official `grindstone.png` background and
        `container/grindstone/error` overlay sprite
      - HopperMenu screens with official `hopper.png`
      - MountScreenOpen screens with official:
        - `horse.png` background for horse-family mounts
        - `nautilus.png` background for nautilus-family mounts
        - generic mount equipment slot sprite
        - saddle/body empty-slot sprites:
          - `container/slot/saddle`
          - `container/slot/horse_armor`
          - `container/slot/llama_armor`
          - `container/slot/nautilus_armor_inventory`
        - horse chest-slot sprite clipped by `inventoryColumns`
      - LecternMenu screens with official:
        - `book.png` background
        - backward/forward page button sprites
      - LoomMenu screens with official:
        - `loom.png` background
        - empty banner/dye/pattern slot sprites
        - disabled scroller sprite
      - MerchantMenu screens with official:
        - `villager.png` background
        - payment/result slot layout
        - villager trade-list sprites loaded for future trade presentation
      - ShulkerBoxMenu screens with official `shulker_box.png`
      - SmithingMenu screens with official:
        - `smithing.png` background
        - `container/smithing/error` overlay sprite
      - StonecutterMenu screens with official `stonecutter.png`
    - LecternMenu page-button clicks queue vanilla container-button ids:
      - `1` for previous page
      - `2` for next page
    - LecternMenu Done closes the active container.
    - LecternMenu Take Book queues vanilla container-button id `3`.
    - LecternMenu renders the current book page as basic ASCII text from
      decoded writable/written book item components and server page data.
    - MountScreenOpen opens a server-controlled container and hit-tests:
      - saddle slot
      - body armor slot
      - horse-family chest slots from `inventoryColumns`
      - player inventory slots
      - hotbar slots
    - Mount saddle/body slot rendering and hit-testing respect vanilla
      active-slot conditions from entity type tags plus baby/tame metadata.
    - Mount Shift-click queues server-authoritative `QUICK_MOVE` clicks.
    - Mount entity preview rendering remains follow-up presentation work.
    - FurnaceMenu/BlastFurnaceMenu/SmokerMenu screens also render official
      progress sprites:
      - lit-progress
      - burn-progress
    - BrewingStandMenu screens also render official progress sprites:
      - fuel-length
      - brew-progress
      - bubbles
    - Those progress sprites use:
      - canonical `ContainerSetData` values
      - vanilla `AbstractFurnaceMenu` / `BrewingStandScreen` progress formulas
    - It queues basic left/right `PICKUP` container clicks for those supported
      fixed-slot screens.
    - It queues server-authoritative keyboard container clicks for those
      supported fixed-slot screens:
      - hovered-slot Q/Ctrl+Q as `THROW`
      - hovered-slot number keys and F as `SWAP`
    - MerchantMenu visible trade-row clicks queue
      `ServerboundSelectTradePacket` for the current visible offer window.
    - MerchantMenu mouse wheel input updates the local trade-list scroll offset
      for offer lists longer than seven rows.
    - MerchantMenu scroller drag updates the local trade-list scroll offset for
      offer lists longer than seven rows.
    - MerchantMenu renders the current visible server-provided offer window with:
      - cost/result item icons
      - normal/out-of-stock trade arrows
      - enabled/disabled scroller sprite at the current local scroll offset
      - selected-offer out-of-stock overlay
      - current villager XP bar
    - It also queues Shift-click `QUICK_MOVE` container clicks for:
      - supported generic containers
      - `generic_3x3`
      - AnvilMenu:
        - rename field text renders from native HUD text labels
        - rename input initializes from the default item hover name when item
          runtime assets are available
        - XP cost and `Too Expensive!` labels render from server
          `ContainerSetData` cost plus local game mode / experience state
        - quick-move and result-slot paths are kept server-authoritative until:
          - repair/enchantment result prediction
          - repair-cost component updates
          - material consumption
          - anvil damage side effects
      - BeaconMenu:
        - primary/secondary effect button grid renders official button sprites
          and mob-effect icons.
        - effect buttons track vanilla disabled and selected states from beacon
          level data plus local primary/secondary selection.
        - effect clicks update local primary/secondary selection.
        - confirm/cancel buttons render official sprites.
        - cancel closes the active container.
        - confirm queues `ServerboundSetBeaconPacket` from the current local
          primary/secondary selection when payment and primary selection make
          the vanilla button active, then closes the container.
        - quick-move paths are kept server-authoritative until:
          - beacon payment item tag routing
          - max-stack-one payment slot prediction
          - payment consumption from `SetBeacon`
        - deferred presentation parity work:
          - confirm/cancel hover state
          - effect highlighted hover state
          - labels/tooltips
      - CraftingMenu:
        - non-result slots
        - result-slot clicks kept server-authoritative until:
          - recipe recomputation
          - remainders are locally modeled
      - CrafterMenu grid/player inventory transfers:
        - vanilla slot ranges
        - disabled grid slots from `ContainerSetData`
        - result-preview slot 45 kept server-authoritative until crafter recipe
          preview and crafting side effects are locally modeled
      - EnchantmentMenu:
        - option button clicks queue `ServerboundContainerButtonClickPacket`
          when the server-provided option cost is nonzero
        - option slots render enabled/disabled level sprites and cost text from
          server `ContainerSetData`, lapis slot count, and local experience
        - quick-move paths are kept server-authoritative until:
          - lapis routing
          - enchantable-item checks
          - single-item movement into the enchantment input
          - enchantment result side effects
        - deferred presentation parity work:
          - animated book model
          - enchanting glyph text and its disabled/highlight coloring
          - hover tooltips
      - BrewingStandMenu:
        - quick-move paths are kept server-authoritative until:
          - brewing fuel tags
          - potion bottle ids
          - ingredient parity
          - component-hashable potion stack prediction
      - CartographyTableMenu:
        - shift-click from the additional slot back to the player inventory is
          locally predicted when the stack has no component hash requirements.
        - result-slot and remaining quick-move paths are kept
          server-authoritative until:
          - full cartography input item/tag routing
          - hashed component patch support for map-id stacks
          - local result stack prediction
          - input consumption
          - take-result sound side effects
        - result-state sprites are projected from:
          - input map id components
          - result-slot map post-processing components
          - known paper/map/glass-pane additional inputs
          - known native map locked/scale state
        - deferred presentation parity work:
          - live map pixels and decorations in the preview area
          - invalid-transform prediction when item runtime or map state is not
            available
      - GrindstoneMenu:
        - player inventory/hotbar range movement when both input slots are
          occupied
        - input/result-side paths are kept server-authoritative until:
          - component-hashable grindstone item prediction
      - hopper
      - LoomMenu:
        - selectable pattern grid renders official pattern button sprites when
          banner and dye slots are populated:
          - 32 vanilla no-item-required patterns when the pattern slot is empty
          - one selectable pattern when a vanilla pattern item is present
        - pattern clicks queue `ServerboundContainerButtonClickPacket` with the
          vanilla selectable pattern index.
        - pattern grid mouse wheel input and scroller drag update the local
          visible pattern window.
        - selected pattern buttons and active/disabled scroller state render
          from native HUD state.
        - quick-move and result-slot paths are kept server-authoritative until:
          - banner/dye/pattern item routing
          - component-aware selectable banner pattern state
          - result prediction
          - input consumption
          - take-result sound side effects
        - deferred presentation parity work:
          - pattern-item component/tag lists beyond the single-pattern vanilla
            items
          - highlighted pattern buttons
          - banner preview
          - max-pattern error overlay
      - MerchantMenu:
        - result-slot pickup and quick-move paths are kept server-authoritative
          until:
          - payment slot routing from `MerchantOffer` costs
          - result prediction
          - input consumption
          - trade sound/XP side effects
        - deferred presentation parity work:
          - rendered generic button row backgrounds and hover/focus highlight
          - selected row state if a future vanilla source adds one
          - future trade XP result bar from payment/result prediction
          - component-aware cost predicate rendering
          - full trade stack decorations and hover tooltips
          - deprecated tooltip behavior
          - discount strikethrough
      - LecternMenu:
        - page buttons queue vanilla previous/next button ids.
        - Done closes the active container.
        - Take Book queues vanilla button id `3`.
        - current page text renders from decoded book components.
        - deferred presentation parity work:
          - rich text styles and click events
          - exact vanilla font wrapping
          - text filtering toggle behavior
      - shulker box
      - SmithingMenu:
        - quick-move paths are kept server-authoritative until:
          - smithing recipe property sets
          - input slot routing
          - result prediction
          - input consumption
          - level event side effects
        - result-slot clicks are kept server-authoritative until smithing
          `onTake` side effects are modeled
        - deferred presentation parity work:
          - armor stand preview
          - cycling empty-slot icons
          - tooltips
      - StonecutterMenu:
        - non-result slots
        - vanilla slot ranges
        - decoded item-id recipe input routing
        - recipe grid wheel scroll and button clicks queue
          `ServerboundContainerButtonClickPacket`
        - recipe option icons and enabled/disabled scroller sprites are rendered
          from native HUD state
        - result-slot clicks kept server-authoritative until:
          - recipe result side effects are locally modeled
      - FurnaceMenu/BlastFurnaceMenu/SmokerMenu, including:
        - vanilla slot ranges
        - result-to-player transfer order
        - recipe-property-set input routing
        - official `FuelValues` fuel routing when vanilla assets are loaded
    - Control/native can still build a basic `ServerboundContainerClickPacket`
      from the active container id, state id, slot id, and cursor item for
      server-opened containers when the carried stack is hash-safe.
    - CrafterMenu empty grid-slot toggles queue
      `ServerboundContainerSlotStateChangedPacket`; rendered disabled/powered
      state remains driven by server `ContainerSetData`.
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
      - `InfestedBlock` / `InfestedRotatedPillarBlock` host destroy-time
        halving
      - common helper registrations for:
        - logs
        - stems
        - leaves
        - buttons
        - flower pots
        - candles
        - beds
        - stained glass
        - shulker boxes
        - pistons
        - stairs
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
    - Left-click attack and block destroy input is suppressed while canonical
      local `using_item` is active.
    - Block destroy and block-target item use are suppressed when the hit block
      position is outside the canonical world border bounds.
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
      - storing the prediction-start local player position
      - snapping the local player back to that position when a rejected ack
        restores a colliding block, matching vanilla `ClientLevel.syncBlockState`
    - Local destroy ticking emits vanilla-shaped block hit sounds through the
      native audio runtime:
      - `SoundType.getHitSound()`
      - `SoundSource.BLOCKS`
      - `(volume + 1) / 8`
      - `pitch * 0.5`
      - every 4 destroy ticks
    - Level event `2001` emits vanilla-shaped block break sounds through the
      native audio runtime:
      - `Block.stateById(data)`
      - `SoundType.getBreakSound()`
      - `SoundSource.BLOCKS`
      - `(volume + 1) / 2`
      - `pitch * 0.8`
  - Completion requires full vanilla movement and these flows to work through
    encoded serverbound packets end to end.

### Signed Chat And Chat Acknowledgement Production

- Owner: `bbb-protocol` + `bbb-net` + `bbb-world` + `bbb-native`
- Status: `partial`
- Next action:
  - Implement remaining signed chat payload production:
    - `ServerboundChatPacket` signatures
    - non-empty `ServerboundChatCommandSignedPacket` argument signatures
    - session/key handling if offline-compatible servers require it
- Evidence / boundary:
  - Covered pieces:
    - `ServerboundChatAckPacket` id 6 and VarInt `offset` encoding
    - offline unsigned `ServerboundChatPacket` encoding/sending
    - `NetCommand::ChatAcknowledgement` sending
    - canonical processed-signature offset tracking
    - online drain queueing
    - offline probe ack sending after vanilla's `offset > 64` threshold
    - play -> configuration re-entry flushes pending signed-chat
      acknowledgement before `ServerboundConfigurationAcknowledgedPacket`
    - canonical outbound last-seen tracker for unsigned chat messages:
      - 20-entry vanilla ring order
      - offset clearing
      - fixed 20-bit acknowledgement bitset
      - checksum byte
      - full-signature pending delete ignore
    - native normal chat submission consumes the canonical last-seen update
    - slash commands keep vanilla string-only `ServerboundChatCommandPacket`
      when the canonical command tree has no signable argument path
    - slash commands with a `minecraft:message` signable argument path send
      `ServerboundChatCommandSignedPacket` with:
      - timestamp
      - salt
      - empty argument signatures for offline/no-profile-key mode
      - canonical outbound last-seen update
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
