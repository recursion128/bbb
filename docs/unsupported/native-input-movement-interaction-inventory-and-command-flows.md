# Native Input, Movement, Interaction, Inventory, And Command Flows — detailed ledger

- Next action:
  - Movement:
    - Extend the current basic AABB collision and gravity/jump slice to cover:
      - remaining vanilla survival physics details beyond the current native
        fixed 20Hz local movement cadence
      - remaining vanilla voxel collision shapes
      - remaining fluid movement work beyond current still water/lava support:
        - sprint-swim camera, animation, and presentation nuance beyond
          canonical swimming pose selection and pitch-coupled vertical travel
      - remaining block-state movement factors beyond the synced frozen-tick
        and local powder-snow contact slowdown/collision slices:
        - remaining powder-snow behavior beyond local player collision:
          - non-player `POWDER_SNOW_WALKABLE_MOBS` entity collision if locally
            controlled non-player entities are added later
          - inside-block particle/extinguish/fall-sound side effects
        - remaining powder-snow `canFreeze` nuance beyond local player:
          - non-player freeze-immune entity type exceptions if locally
            controlled entity freezing is added later
        - remaining no-collision hazard `entityInside` effects:
          - sweet berry bush age-gated damage and fox/bee exceptions if locally
            controlled non-player entities are added later
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
    - full model-shaped crack decals beyond the current cube overlay; vanilla
      crumbling blend/cull/depth-bias behavior is now covered by the renderer
      pipeline state, with the local cube overlay using outward winding
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
      - clipboard parity for copy/cut/paste editing
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
      - Main-hand `attack_range` item components:
        - are decoded from network item patches
        - are parsed from official 26.1 default item declarations
        - participate in crosshair entity selection for extended custom reach
        - use custom min/max reach, creative reach, hitbox margin, and block
          outline clipping before entity hits
        - follow vanilla `AttackRange.getClosesetHit` movement extension by
          adding the positive local movement component along the look vector to
          crosshair entity search reach
        - suppress out-of-range `AttackEntity` packets for the current entity
          hit
  - Inventory:
    - Implement:
      - remaining rich tooltip behavior:
        - non-ASCII font providers
        - bidirectional text shaping
        - official tooltip background/frame sprites
        - italic and complex component styles
        - component-specific detail lines
      - remaining dedicated server-opened menu layouts not already covered by
        these baseline menu families:
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
        - composite/component-aware/special recipe display matching beyond the
          current direct-item and crafting-requirement shaped/shapeless 2x2
          subset
        - recipe-specific remainder items
    - Higher-value next slices should prioritize missing player-visible flows
      such as recipe book/creative variants or container `0` crafting result
      parity over additional baseline layout/probe coverage for the menu
      families listed above.
- Evidence / boundary:
  - Movement:
    - Native movement projects world-computed `on_ground` and
      `horizontal_collision` into serverbound move commands.
    - Native unmounted local player movement accumulates elapsed time into
      fixed 20Hz physics steps while preserving per-frame look updates.
    - Serverbound movement packets use vanilla's strict position threshold and
      20-sendPosition-call position reminder semantics; rotation-only and
      status-only packets do not reset the reminder.
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
      - mangrove roots / muddy mangrove roots
      - bamboo stalk
      - chorus plant / chorus flower
      - conduit
      - azalea / flowering azalea
      - snow layer
      - flat carpet
      - pale moss carpet base
      - flowers / bushes / crops / thin ground overlays
      - cactus
      - farmland / dirt path
      - soul sand / mud
      - honey block
      - cake
      - lily pad
      - amethyst cluster / bud
      - flower pot
      - candle / candle cake
      - cocoa
      - shelf
      - sea pickle
      - pointed dripstone
      - skull / wall skull
      - turtle egg
      - sniffer egg
      - dried ghast
      - chain
      - ladder
      - rod
      - campfire
      - fire / redstone wire / tripwire
      - copper grate
      - piston / sticky piston
      - piston head
      - big dripleaf leaf
      - end portal frame
      - daylight detector
      - sculk sensor / calibrated sculk sensor / sculk shrieker
      - heavy core
      - copper golem statue
      - dragon egg
      - decorated pot
      - rail / powered rail / detector rail / activator rail
      - torch / wall torch
      - lever / button
      - sign / hanging sign
      - banner / wall banner
      - nether portal plane
      - end portal / end gateway
      - structure void / light
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
      - `moving_piston` without a canonical moving-piston block entity is
        treated as empty, matching vanilla's empty fallback when no block entity
        supplies dynamic collision.
      - `big_dripleaf_stem` is treated as empty, matching vanilla
        `noCollision`; `big_dripleaf` leaf collision follows vanilla
        `tilt=none|unstable|partial|full` heights.
      - `pale_moss_carpet` follows vanilla `MossyCarpetBlock` collision:
        `bottom=true` uses the default 1px carpet base and `bottom=false`
        is empty even when side outline faces are present.
      - `bamboo` follows vanilla `BambooStalkBlock` 3px offset collision
        column; `bamboo_sapling` is empty, matching its vanilla
        `noCollision` registration.
      - `mangrove_roots` and `muddy_mangrove_roots` use vanilla default
        full-block collision even though their render material is cutout.
      - `chorus_plant` follows vanilla `PipeBlock(10.0F)` center cube plus
        connected six-direction arms; `chorus_flower` uses vanilla default
        full-block collision.
      - `conduit` follows vanilla `Block.cube(6.0)` collision.
      - `azalea` and `flowering_azalea` follow vanilla `AzaleaBlock` collision:
        an 8px-tall full-width crown plus a 4px center stem.
      - `cocoa` follows vanilla `CocoaBlock` age-scaled wall-pod collision:
        age `0..2` uses a north-facing baseline at z `1..5/7/9` px and is
        rotated by the `facing` property.
      - `shelf` follows vanilla `ShelfBlock` collision:
        a horizontal-facing backboard plus top and bottom lips from
        `Shapes.rotateHorizontal`.
      - `amethyst_cluster`, `large_amethyst_bud`, `medium_amethyst_bud`, and
        `small_amethyst_bud` follow vanilla `AmethystClusterBlock` collision:
        `Block.boxZ(width, 16 - height, 16)` rotated with `Shapes.rotateAll`.
      - `copper_golem_statue` variants follow vanilla
        `CopperGolemStatueBlock` collision: fixed
        `Block.column(10.0, 0.0, 14.0)` independent of pose, facing,
        waterlogged, weathered, or waxed state.
      - common vanilla `.noCollision()` vegetation, crops, thin ground
        overlays, fire, redstone wire, and tripwire are classified as cutout
        non-blocking terrain for local movement. Covered examples include:
        - ordinary flowers and dry vegetation
        - bushes, fungi, sprouts, propagules, and dripleaf stems
        - wheat, carrots, potatoes, beetroots, nether wart, stems, and
          sugar cane
        - pink petals, leaf litter, sculk vein, glow lichen, resin clump, and
          frogspawn
      - standing signs, wall signs, ceiling hanging signs, and wall hanging
        signs follow vanilla `.noCollision()` registration; their outline
        shapes remain renderer/crosshair selection data only.
      - standing banners and wall banners follow vanilla `.noCollision()`
        registration; their banner pole/cloth outline remains selection data
        and does not block local movement.
      - Rails, torches, wall torches, levers, and buttons follow vanilla
        `.noCollision()` registration; their outline shapes remain
        renderer/crosshair selection data only.
      - `nether_portal` is translucent terrain and follows vanilla
        `.noCollision()` local movement. Portal travel remains server/runtime
        behavior driven by entity-inside semantics, not terrain collision.
      - `end_portal` and `end_gateway` are invisible portal blocks and follow
        vanilla `.noCollision()` local movement. End portal/gateway travel,
        smoke/portal particles, and gateway cooldown remain server/runtime or
        entity-inside behavior, not terrain collision.
      - `structure_void` and `light` are invisible non-blocking blocks for local
        movement. `barrier` remains a full collision block.
      - `cobweb` and `sweet_berry_bush` use vanilla `noCollision`
        registration so they do not block local movement.
      - `cobweb` applies vanilla `entityInside` stuck movement multipliers:
        normal players use `(0.25, 0.05, 0.25)` and players with the synced
        Weaving effect use `(0.5, 0.25, 0.5)`.
      - `sweet_berry_bush` applies vanilla local-player stuck movement
        multiplier `(0.8, 0.75, 0.8)`; server-authored damage remains
        authoritative.
    - It applies:
      - synced local player `gravity` attribute id `14` and basic gravity
      - synced local player `NoGravity` entity metadata data id `5`, which
        suppresses local gravity in air, water, and lava travel
      - synced local player `TicksFrozen` entity metadata data id `7` as the
        vanilla `minecraft:powder_snow` movement-speed `ADD_VALUE` modifier:
        - amount is `-0.05 * min(ticks_frozen, 140) / 140`
        - skipped when the server already syncs the `minecraft:powder_snow`
          movement-speed modifier
      - local player in-world `minecraft:powder_snow` contact updates synced
        `TicksFrozen` metadata:
        - increments by `1` per local physics step while inside powder snow
        - thaws by `2` per local physics step outside powder snow
        - clamps to `0..140`
        - respects vanilla local player `canFreeze` immunity:
          - spectator mode thaws instead of freezing
          - armor/body items in the official
            `minecraft:freeze_immune_wearables` item tag thaw instead of
            freezing
      - vanilla block speed factor for local walking prediction:
        - `minecraft:soul_sand` and `minecraft:honey_block` apply `0.4`
        - synced local player `movement_efficiency` attribute id `21`
          interpolates the block factor back toward `1.0`
        - `minecraft:water` and `minecraft:bubble_column` at the current
          player block position do not fall back to the block below
      - vanilla block jump factor for local jumps:
        - `minecraft:honey_block` applies `0.5` to the base jump strength
        - Jump Boost remains additive after the block jump factor
      - vanilla slime block landing bounce:
        - downward collision reverses local player Y velocity
        - sneak input suppresses the bounce
      - basic vanilla climbable movement for ladder and vine-family blocks:
        - resets fall distance while inside the climbable block
        - clamps horizontal velocity to `0.15` per tick
        - clamps downward velocity to `-0.15` per tick
        - sneak input suppresses non-scaffolding climbable sliding
        - jump or horizontal collision applies the vanilla upward climb velocity
      - scaffolding climbable movement and context-sensitive collision:
        - counts as climbable for fall-distance reset and velocity clamps
        - sneak input does not suppress downward slide
        - side entry no longer treats scaffolding as a full block
        - stable top shape supports the player when above and not descending
        - sneak/descending input bypasses the stable top shape
        - unstable bottom support shape applies for bottom scaffolding with
          non-zero distance
      - vanilla local player powder snow context-sensitive collision:
        - ordinary local player movement sinks through `minecraft:powder_snow`
        - feet slot item must be the injected official
          `minecraft:leather_boots` protocol id to stand on top
        - freeze-immune wearable items do not imply powder-snow walkability
        - sneak/descending input bypasses the top collision
        - `fall_distance > 2.5` uses the vanilla full-width `0.9`-high falling
          collision shape
        - leather boots plus jump or horizontal collision while inside powder
          snow applies vanilla `0.2` climb-out Y velocity before gravity/friction
      - vanilla open trapdoor-as-ladder climbable behavior when:
        - the current block is an open trapdoor
        - the block below is a ladder
        - both blocks have the same horizontal facing
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
      - vanilla passenger sprint eligibility for local-authoritative vehicles:
        - camel and camel_husk can sprint
        - horse-family and boat mounts cannot sprint
        - the local player must be the controlling passenger
        - player food is ignored in the mounted vehicle branch
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
      - vanilla default 7 tick double-tap-forward sprint window is supported
      - low food suppresses unmounted `START_SPRINTING`
      - camel and camel_husk mounts can queue `START_SPRINTING` while horse and
        boat mounts only queue raw `PlayerInput`
      - releasing focus or input queues `STOP_SPRINTING` when effective sprint
        was active
    - Existing input modules queue many serverbound packets, including
      vanilla-shaped boat/raft paddle-state packets from local mounted input.
    - They queue `START_RIDING_JUMP` player commands for vanilla
      `PlayerRideableJumping` vehicle types using:
      - the 26.1 charge scale
      - jump release, including focus/UI release cleanup
    - They toggle local creative/spectator-style flying with the vanilla
      double-jump window when synced abilities allow flight, then queue
      `ServerboundPlayerAbilitiesPacket`.
    - Creative middle-click clone in local and server-opened containers queues
      `ServerboundContainerClickPacket` with `ContainerInput.CLONE`, an empty
      changed-slot set, and a carried item copied to the vanilla max stack
      size.
    - Control/native can queue `ServerboundSetCreativeModeSlotPacket` for:
      - creative inventory slot updates
      - creative drop requests
      - empty or componentless item stacks
      Full item component payload encoding remains follow-up before
      component-rich creative stacks can be sent.
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
        - after middle-of-line edits that change a slash command
      - submit `ServerboundChatCommandPacket` payloads without:
        - the leading slash
      - edit chat and command text before submit with:
        - left/right/home/end cursor movement
        - delete/backspace
        - Ctrl+left/right word movement
        - Ctrl+delete/backspace word deletion
        - Ctrl+A selection replacement before chat or slash-command submit
      - queue explicit `ServerboundClientCommandPacket` commands:
        - perform-respawn
        - request-stats
        - request-game-rule-values
        - from native/control input
      - avoid auto-respawning on dead health
    - Sign editor input paths:
      - open from `ClientboundOpenSignEditorPacket`
      - initialize from canonical sign block entity front/back text when
        available
      - edit four pending lines
      - move the line cursor with left/right/home/end and edit at the cursor
        with text input/backspace/delete
      - Ctrl+A selects the current line, and text/backspace/delete replace or
        clear the selected range
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
        - result-slot prediction only when default crafting remainders are
          known and involved inputs have no recipe-specific crafting remainder;
          default item crafting remainders are locally placed back into the
          crafting grid or visible player inventory, otherwise the click is
          queued server-authoritatively without mutating the local snapshot
        - local 2x2 input clicks recompute the result slot from server-authored
          recipe-book shaped/shapeless displays when the result is a direct
          item or item-stack display and inputs are direct items or
          `crafting_requirements` holder sets, including item tags and mirrored
          shaped patterns
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
      - hashes local container-click stacks with empty/default component
        patches, removed-only component patches, and scalar integer
        component patches (`max_stack_size`, `max_damage`, `damage`, and
        `map_id`), including integer patches plus removed components
      - supports bundle wheel selection on hovered local inventory and
        server-opened container slots
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
        - Ctrl+A selection replacement/clearing in the rename text field
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
    - `ClientboundOpenBook` opens a canonical active book screen when the
      requested hand holds decoded writable/written book item components.
      The native client renders the book background/page text, handles local
      page buttons and PageUp/PageDown, and closes it with Escape/E/Done
      without queuing container commands.
    - MountScreenOpen opens a server-controlled container and hit-tests:
      - saddle slot
      - body armor slot
      - horse-family chest slots from `inventoryColumns`
      - player inventory slots
      - hotbar slots
    - Mount saddle/body slot rendering and hit-testing respect vanilla
      active-slot conditions from entity type tags plus baby/tame metadata.
    - Mount Shift-click routes locally for:
      - mount-owned slots back to the player inventory with vanilla
        reverse-fill ordering
      - default saddle items into an active saddle slot
      - known default horse/llama/nautilus body armor into matching active
        body slots
      - non-equipment player items into mount chest slots when present
      - player main-inventory/hotbar range movement when no mount slot accepts
        the stack
    - Mount component-patched or otherwise entity-specific equippable
      predicates remain server-authoritative.
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
    - Opening a server-driven runtime screen, including containers, dialogs,
      sign editors, and book views, releases active movement, item use, and
      block destroy input through the native runtime, matching vanilla
      `Minecraft.setScreen` screen-open input release.
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
        - input slots quick-move back to the player inventory with vanilla
          forward-fill ordering
        - player-inventory quick-move into the two input slots is locally
          predicted with vanilla `ItemCombinerMenu` ordering
        - result-slot quick-move into the player inventory is locally predicted
          when the current server result fully fits, cost plus local
          experience/creative state allow pickup, the additional slot is empty,
          and the single input is consumed to empty.
        - result-slot primary pickup with an empty cursor is locally predicted
          from the current server result when cost plus local experience/creative
          state allow pickup, the additional slot is empty, and the single input
          is consumed to empty.
        - result-slot paths are kept server-authoritative until:
          - carried-cursor or secondary result-slot pickup
          - repair/enchantment result prediction
          - repair-cost component updates
          - material/sacrifice consumption
          - XP and cost data side effects
          - blocked or partial result transfers
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
        - quick-move paths are locally predicted for:
          - vanilla slot ranges
          - single-item payment routing from `minecraft:beacon_payment_items`
          - max-stack-one payment slot behavior
        - confirm locally predicts the single payment-slot consumption and
          primary/secondary effect data update before closing the container.
        - beacon block entity updates and active player effects remain
          server-authoritative.
        - deferred presentation parity work:
          - confirm/cancel hover state
          - effect highlighted hover state
          - labels/tooltips
      - CraftingMenu:
        - non-result slots
        - result-slot primary pickup with an empty cursor and result-slot
          quick-move are locally predicted from the current server result when
          default crafting-remainder metadata is loaded and every non-empty
          crafting-grid input is not one of the known vanilla recipe-specific
          remainder inputs for banner duplication or book cloning. Default item
          crafting remainders are locally placed back into the crafting grid
          when that is sufficient; cases that require adding a remainder to
          hidden player inventory slots remain server-authoritative. Empty-cursor
          primary pickup leaves the current result in place when the same input
          slots remain populated; quick-move repeats while all resulting stacks
          and default remainders stay representable in the active container.
        - result-slot paths are kept server-authoritative until:
          - carried-cursor or secondary result-slot pickup
          - blocked or partial result transfers
          - recipe recomputation
          - recipe-specific remainders are locally modeled
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
        - input and lapis slots quick-move back to the player inventory with
          vanilla reverse-fill ordering
        - player-inventory quick-move routing is locally predicted when
          native item runtime provides the vanilla lapis lazuli protocol id:
          - lapis lazuli stacks route to the lapis slot
          - other item stacks move one item into an empty enchantment input
            slot, matching vanilla slot0 behavior
        - enchantment result side effects remain server-authoritative.
        - deferred presentation parity work:
          - animated book model
          - enchanting glyph text and its disabled/highlight coloring
          - hover tooltips
      - BrewingStandMenu:
        - quick-move paths are locally predicted for:
          - vanilla slot ranges
          - brewing fuel item tag routing
          - default potion/glass-bottle item routing
          - default vanilla ingredient item routing
          - max-stack-one bottle slots
        - remaining brewing parity work:
          - full feature-flag-sensitive `PotionBrewing` bootstrap parity
          - local brew-result stack prediction
          - brew completion side effects
      - CartographyTableMenu:
        - shift-click from the map and additional slots back to the player
          inventory is locally predicted when changed stacks have no component
          hash requirements.
        - player-inventory quick-move paths are locally predicted for:
          - filled-map routing into the map slot when the decoded stack summary
            contains hashable scalar integer component patches such as
            `map_id` for serverbound changed slots
          - paper/map/glass-pane additional input routing when native item
            runtime provides the vanilla protocol ids
          - ordinary player inventory/hotbar range movement
        - result-slot quick-move into the player inventory is locally predicted
          when the current server result fully fits and the single map and
          additional inputs are consumed to empty.
        - result-slot primary pickup with an empty cursor is locally predicted
          from the current server result when the single map and additional
          inputs are consumed to empty.
        - result-slot and component-hash-unsupported filled-map paths are kept
          server-authoritative until:
          - carried-cursor or secondary result-slot pickup
          - component-hash-unsupported result stacks
          - local result stack prediction
          - result recomputation after partial input consumption
          - blocked or partial result transfers
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
        - input slots back to player inventory
        - default damageable player inventory items route into open input slots
          when the native item registry provides vanilla max-damage ids
        - player inventory/hotbar range movement when both input slots are
          occupied
        - result-slot quick-move into the player inventory is locally
          predicted from the current server-provided result stack when the
          transfer fully fits and the input/result stacks are hashable.
        - result-slot primary pickup with an empty cursor is locally predicted
          from the current server-provided result stack; the vanilla take path
          clears both input slots while XP and level-event side effects remain
          server-authoritative.
        - component/enchantment-only player-to-input, secondary or carried-cursor
          result-slot pickup, partial/full-inventory result transfers, XP, and
          level-event side effects are kept server-authoritative
          until:
          - component-hashable grindstone item/result prediction
          - full result recomputation parity
      - HopperMenu:
        - quick-move paths are locally predicted for:
          - hopper slots back to player inventory with vanilla reverse-fill
            ordering
          - player inventory/hotbar slots into the hopper with vanilla
            forward-fill ordering
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
        - quick-move paths are locally predicted for:
          - input slots back to player inventory
          - player inventory/hotbar range movement
          - banner/dye/pattern input routing from vanilla item tags
          - result-slot quick-move into the player inventory when the current
            server result fully fits, the pattern slot is empty, and the single
            banner/dye inputs are consumed to empty
          - result-slot primary pickup with an empty cursor when the current
            server result is used, the pattern slot is empty, and the single
            banner/dye inputs are consumed to empty
        - result-slot paths are kept server-authoritative until:
          - carried-cursor or secondary result-slot pickup
          - component-aware selectable banner pattern state
          - result recomputation after partial input consumption
          - pattern-item result quick-move
          - pattern-item result pickup
          - blocked or partial result transfers
          - take-result sound side effects
        - deferred presentation parity work:
          - pattern-item component/tag lists beyond the single-pattern vanilla
            items
          - highlighted pattern buttons
          - banner preview
          - max-pattern error overlay
      - MerchantMenu:
        - payment slots quick-move back to the player inventory
        - player inventory/hotbar quick-move range movement
        - trade-row clicks locally autofill payment slots from
          component-predicate-free `MerchantOffer` costs after returning
          existing payment items to player inventory
        - result-slot quick-move locally predicts selected
          component-predicate-free `MerchantOffer`s when the current result
          matches the selected sell stack, payment slots satisfy the modified
          cost counts in vanilla normal or swapped order, and the result fully
          fits the player inventory; payment remainders are kept and the result
          slot is repopulated when the same selected offer remains in stock and
          payable
        - result-slot primary pickup with an empty cursor locally predicts the
          same component-predicate-free selected-offer payment/remainder path
          and places the sold item on the cursor
        - result-slot secondary pickup, carried-cursor pickup, alternate-offer
          payment remainder recomputation, and component-predicate
          trade-row/payment result prediction are kept server-authoritative
          until:
          - full active-offer search across remaining payment slots
          - trade sound/XP side effects
          - component-aware `ItemCost` predicate matching
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
      - ShulkerBoxMenu:
        - quick-move paths are locally predicted for:
          - shulker slots back to player inventory with vanilla reverse-fill
            ordering
          - player inventory/hotbar slots into the shulker box with vanilla
            forward-fill ordering
      - SmithingMenu:
        - input slots quick-move back to the player inventory with vanilla
          forward-fill ordering
        - player-inventory quick-move paths are locally predicted when
          `UpdateRecipes` provides the three vanilla smithing property sets:
          - template/base/addition input slot routing
          - ordinary player inventory/hotbar range movement
        - result-slot quick-move into the player inventory is locally predicted
          when the current server result fully fits and the single
          template/base/addition inputs are consumed to empty.
        - result-slot primary pickup with an empty cursor is locally predicted
          from the current server result when the single template/base/addition
          inputs are consumed to empty.
        - result-slot clicks are kept server-authoritative until:
          - carried-cursor or secondary result-slot pickup
          - result prediction
          - result recomputation after partial input consumption
          - partial/full-inventory result transfers
          - level event side effects
        - deferred presentation parity work:
          - generic held-item / non-skull head-item projection for the armor
            stand preview
          - cycling empty-slot icons
          - tooltips
      - StonecutterMenu:
        - non-result slots
        - vanilla slot ranges
        - decoded item-id recipe input routing
        - result-slot quick-move into the player inventory is locally
          predicted from the current server-provided result stack when the
          transfer fully fits and the input/result stacks are hashable.
        - result-slot primary pickup with an empty cursor is locally predicted
          from the current server-provided result stack when the single input is
          consumed to empty and the input/result stacks are hashable.
        - recipe grid wheel scroll and button clicks queue
          `ServerboundContainerButtonClickPacket`
        - recipe option icons and enabled/disabled scroller sprites are rendered
          from native HUD state
        - result-slot pickups with remaining input, carried-cursor pickup,
          partial/full-inventory result transfers, recipe used bookkeeping,
          input-specific result recomputation, and take-result sound side
          effects remain server-authoritative.
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
      - merging local stages and server progress per block position before
        keeping the highest renderer-visible stage
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
