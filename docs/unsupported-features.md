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
- Two states are not todos, and deleting one silently re-opens a decision that
  was already made:
  - **Deferred** — a real gap, intentionally not scheduled. Carries the
    condition that turns it back into a todo.
  - **Not needed** — vanilla 26.1 has no such behavior, or bbb has no surface
    for it. Carries the vanilla evidence.
- This file does not overflow into sibling files. If it grows, the gap list grew.

---

# Todo

## Protocol coverage for remaining required 26.1 packet families

Owner: `bbb-protocol`

- Audit field order, nullability, enum ordinals, and serverbound encoders against
  `<MC_CODE_ROOT>/sources/26.1/`, adding focused encode/decode tests with each
  packet slice, including strict unit-payload, terminal packet/component roots,
  remaining bounded strings/byte arrays, typed resource-id normalization,
  bounded map count/key/value, untrusted URI validation and
  NBT-root/depth/count boundaries;
  keep handwritten handler coverage aligned with the generated packet catalog
  as unsupported families are implemented.
- Use `docs/packet-coverage-26.1.json` as the complete 256-entry inventory. A
  cell remains `partial` until it carries implementation and semantic-test
  evidence through the checked-in sparse overrides; catalog presence or a
  generic dispatch arm alone is not completion.
- Route a transferred target into production reconnect, send its Transfer-intent
  handshake, and lock the closing-to-handshake state transition.
- Complete authenticated login: perform session authentication/encryption from the
  typed `ClientboundHelloPacket`, generate the encrypted inputs for the existing
  `ServerboundKeyPacket` encoder, install the negotiated ciphers, lock the
  sent-hello to encryption-request transition, and reject duplicate or out-of-state
  login responses.
- Complete configuration resource-pack runtime behavior: download allowed packs,
  verify hashes, apply/reload them, enforce required-pack outcomes, emit the full
  response lifecycle, and unload one/all packs on pop.
- Build a local `KnownPacksManager`/server-data resource manager, select only
  offered packs whose registry data bbb can actually provide, and pass that
  intersection to the existing 64-entry response encoder instead of always
  returning an empty configuration response.
- Complete configuration/play transfer runtime behavior: enter a closing state,
  continue accepting late StoreCookie/Transfer packets, disconnect and make the
  current connection read-only, carry cookies/seen-player/insecure-chat transfer
  state, and reconnect with the Transfer intent instead of only recording the
  requested host and port.
- Decode configuration/play dialog holders into typed `Dialog` data, render the
  corresponding screens, restore the previous screen on clear, and encode
  resource-id plus bounded optional-NBT custom click actions in the active state.
- Decode block/chunk/entity/event debug subscription payloads through the synced
  `DEBUG_SUBSCRIPTION` registry codecs, retain typed values with vanilla game-time
  expiry/removal semantics, ignore entity updates whose target is absent, and feed
  the matching debug renderers. The fixed 16-entry vanilla registration-order and
  resource-id catalog now owns the serverbound tick-time subscription ID, but it
  does not implement the per-subscription value codecs. Bounded raw payload
  diagnostics are not typed subscription support.
- Build the received configuration registry entries and tags into a frozen,
  typed `RegistryAccess` on top of the checked holder-identity lookup: decode
  each synchronized registry with its element codec, merge known-pack stubs,
  replace resource-id-only entity-variant and derived audio projections with
  typed element codecs, materialize the checked tag-resolution view in the
  frozen lookup, extend the source-aware built-in fallback beyond the current
  item/block/entity-type catalogs where a consumer requires it, apply pending
  static tags and data-component initializers, and freeze typed lookup state.
  Raw NBT summaries, the narrow validated dimension shape/lighting projection,
  individual checked tag queries, and a few derived audio registries are not
  that runtime. Dimension-type direct visual environment attributes now drive
  custom fog/sky/water-fog colors and distances plus lightmap tint/factor
  without classifying an unknown resource as a vanilla dimension. Remaining
  work includes environment-attribute modifier compounds, syncable audio/gameplay
  attributes, timeline/clock holders and their layer ordering; typed `skybox`,
  direct `ambient_light`, and `has_skylight`/`has_ceiling` weather gating are
  complete.
- Complete play Login session initialization beyond the current player-id and
  partial dimension/gameplay projection: retain the initial level set, chunk
  radius, simulation distance, secure-chat enforcement, seed and portal
  cooldown; reset level-bound debug/chat/camera/player state; and enter the
  vanilla level-load readiness lifecycle without introducing an
  `IntegratedServer` owner.
- Complete play Respawn lifecycle from
  `ClientPacketListener.handleRespawn`: retain seed and portal cooldown in the
  canonical spawn state, reconstruct the local player according to both keep
  bits, preserve map data across dimension replacement, close the active
  container, reset camera/music/debug-subscriber state, and re-enter
  `LevelLoadTracker` readiness. The current world reset/keep-bit baseline is not
  the full client lifecycle.
- Recompute vanilla fuel values and creative search-tree tag membership after
  play UpdateTags reloads, following `ClientPacketListener.handleUpdateTags`.
- Complete StatsScreen presentation parity: add narration. General, Items and Mobs now leave
  loading only after the requested AwardStats response and use selected-pack language.
  Item/header/empty-tab tooltips and vanilla Ctrl+digit/Ctrl+Tab navigation are wired;
  Items mechanically maps block stats through checked-in `Block.asItem` evidence, renders
  flat/3D item icons and vanilla header/slot/sort sprites, preserves six-column no-value
  semantics and the descending/ascending/clear sort cycle. All three lists use vanilla
  half-row/pixel wheel amounts, content clamping, draggable nine-slice scrollbar and viewport
  scissor; the Items header scrolls as its first entry. Canonical state retains unresolved
  identities without guessing.
- Resolve UpdateAttributes holder ids through that session RegistryAccess and
  validate each snapshot against the target living entity's actual AttributeMap
  instance set; do not accept arbitrary fixed-id attributes on every living type.
- Complete SetEntityData semantic fidelity: preserve rich Component and full
  ItemStack/component values, resolve registry-holder ids through the session
  RegistryAccess, validate each data id/serializer against the generated entity
  accessor schema like `SynchedEntityData.assignValues`, and close the remaining
  family consumers through the entity matrix. The current flat summary merge is
  not proof of complete metadata behavior.
- Defer configuration/play keepalive responses while client event polling is
  render-frozen, with vanilla's one-minute expiry, instead of always replying
  immediately. Ping/pong remains immediate and is already separate.
- Drive `ServerboundClientTickEndPacket` from the actual native client-tick end,
  after that tick's player/vehicle movement has been queued, instead of an
  independent Tokio interval whose first deadline fires immediately. The current
  play-only 20 Hz sender keeps servers live but does not prove vanilla movement
  grouping semantics from `handleClientTickEnd`.
- Replace the current first-`ClientboundPlayerPositionPacket` trigger for
  `ServerboundPlayerLoadedPacket` with vanilla `LevelLoadTracker` readiness:
  wait for server loading packets, then for the local-player section to be
  compiled and visible (with the spectator/dead/out-of-height shortcuts and
  30-second timeout), and send exactly once. Also make teleport acknowledgement
  echo the unchanged mounted-player transform and advance the block-state
  prediction teleport sequence as `ClientPacketListener.handleMovePlayer` does.
- Then cover play, movement, inventory, chat, interaction and command suggestion
  end to end.
- Preserve rich Components for command-suggestion tooltips and render vanilla's
  scrollable suggestion list, selection suffix, hover tooltip and narration.
  Matching responses currently support safe first-result Tab replacement;
  stale/wrong transaction ids are ignored.
- Build a registry-aware repo-native command dispatcher from the Commands tree,
  matching `CommandBuildContext.simple` argument construction, local parsing,
  validation, redirects and completion rather than retaining only structural
  nodes and signed-message detection.

## Vanilla Java generator frontend migration

Owner: `scripts/` + each generated catalog's semantic crate

- Complete the remaining cross-crate registry/ordinal/serializer-id and
  resource-membership audit, and emit every new generated-catalog consumer guard
  or migrated generated-catalog consumer guard from that catalog's generator
  rather than hand-editing its checked-in output.
  Generate a membership only when an exact tag, class, property, or registry
  source fact owns that semantic; do not substitute approximate tags.
  Terrain render layer is deliberately excluded from this catalog work because
  vanilla 26.1 derives it per baked material/quad from the resolved sprite's
  transparency. Any newly found Java-source catalog extractor must consume the
  shared Tree-sitter frontend and checked-in deterministic output; ordinary
  build/test must not read local vanilla sources. Migrate each newly identified
  legacy parser as its own semantic-owner slice.

## Unknown clientbound packets in login, configuration, and play

Owner: `bbb-protocol` + `bbb-net` + `bbb-native`

- For each unsupported packet surfaced by probe/control diagnostics, verify it
  against local vanilla 26.1 sources, then either implement protocol decode plus
  world/runtime handling, or record why it is runtime-only.

## Native-owned business snapshots

Owner: `bbb-world` + `bbb-native` + `bbb-control`

- Move the remaining client-observable state into `WorldStore`, removing
  native-only `last_*` snapshots wherever a world owner exists or should exist.
- Keep `NetCounters` limited to connection/runtime status and command-queue
  projections.

## Signed chat and chat acknowledgement production

Owner: `bbb-protocol` + `bbb-net` + `bbb-world` + `bbb-native`

- Produce `ServerboundChatPacket` signatures. The protocol encoder accepts the
  optional 256-byte signature, but the production chat path still sends `None`.
- Produce non-empty `ServerboundChatCommandSignedPacket` argument signatures.
- Fetch and refresh profile key pairs, create the local chat session/message
  encoder, validate the resulting server broadcast, and automatically send
  `ServerboundChatSessionUpdatePacket`. Its protocol encoder and net send surface
  exist, but no production key/session owner drives them yet.
- On DeleteChat, disconnect for a packed signature that cannot be resolved,
  remove matching delayed-listener messages, and replace the visible message
  with vanilla's deleted marker no earlier than 60 GUI ticks after insertion.
  The current resolved-signature diagnostic record is not visible chat deletion.
- Decode PlayerChat/DisguisedChat trusted Components without flattening style,
  translation, siblings or interactions; resolve numeric `ChatType` holders from
  session RegistryAccess; apply chat/narration decoration; and run chat receive
  ability, delay queue, narration and logging behavior. Plain-text history with a
  numeric holder id is not the `ChatListener` presentation path.

## Crosshair entity interaction parity

Owner: `bbb-world` + `bbb-native` + `bbb-renderer`

- Validate any future `yRotA` source.
- Replace the debug target overlay with full entity model rendering and
  interaction feedback once renderer entity presentation exists.

## Native input, movement, interaction, inventory, and command flows

Owner: `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world`

Movement:

- UpdateMobEffect wire/state replacement, living-entity gate and common movement/
  render consumers are covered; remaining vanilla client behavior across all mob
  effects (attributes, particles, HUD and specialized movement) stays open.
- Complete the remaining camera-FOV inputs beyond synchronized player abilities:
  configurable base FOV/effect scale, bow-use/scoping zoom, and death/fluid
  modifiers. Vanilla keeps `walkingSpeed` as the FOV ratio denominator; it does
  not replace the separately synchronized movement-speed attribute base.
- Cover the remaining vanilla survival physics beyond the synchronized local
  movement cadence.
- Cover the remaining vanilla voxel collision shapes.
- Implement full vanilla post-`Entity.move` `deltaMovement` travel ordering.
- Add sprint-swim camera, animation, and presentation nuance.
- Implement the `minecraft:powder_snow` inside-block particle, extinguish, and
  fall-sound side effects.
- Handle the remaining `MovePlayer.Rot` vehicle movement send edge cases.
- Route relative entity Pos/PosRot/Rot targets through vanilla-equivalent
  `moveOrInterpolateTo` state instead of snapping the canonical transform, and
  resolve every entity-specific local-authority controller before deciding
  whether a packet only advances the `VecDeltaCodec` base.
- Complete `EntityPositionSync`/`TeleportEntity` ticking and distance-based
  interpolation, indirect-passenger reposition/old-pose reset, removed-player-
  vehicle teleport fallback, and locally-authoritative vehicle echo.
- Preserve `LivingEntity.lerpHeadTo(..., 3)` target/step state for RotateHead
  instead of snapping every entity's head yaw immediately; non-living Entity
  keeps the immediate-set behavior.
- Preserve `NewMinecartBehavior`'s pending/current lerp-step queues: along-track
  packets append pending steps and only the client tick advances the canonical
  transform. Do not replace active steps or snap the transform to the last step
  when the packet arrives.
- Gate SetEntityLink application on the source entity implementing vanilla
  `Leashable`; the generic entity component must not make non-leashable types
  accept a delayed holder id.
- Complete first-local-mount SetPassengers side effects: clear removed-vehicle
  recovery state, align player body/head yaw when mounting a boat, and surface
  vanilla's dismount-key overlay/narration once per new mount.
- Gate HurtAnimation's ten-tick red-overlay state on vanilla `LivingEntity`;
  base `Entity.animateHurt` is a no-op, and the client must not infer or apply
  authoritative damage from this presentation packet.
- Complete ClientboundAnimate action 2 (`Player.stopSleepInBed(false, false)`)
  and gate actions 0/3 on `LivingEntity` plus action 2 on `Player`; keep 4/5 as
  presentation-only crit tracking emitters, not authoritative attacks.
- Resolve DamageEvent's DamageType holder through session RegistryAccess and
  complete only vanilla client presentation state: LivingEntity gate, walk
  animation speed, invulnerability/hurt timers, hurt sound, and last-source
  timestamp. Never infer health loss, death, or authoritative combat results.
- Implement full vanilla boat physics, water/buoyancy/collision parity, and
  non-boat vehicle movement only for the locally controlled instance; resolve
  entity-specific `getControllingPassenger` gates (saddle/held item/armor and
  related state) rather than treating first passenger as sufficient, and cancel
  active interpolation on accepted corrections. Remote vehicles remain
  server-transform/interpolation driven.

Block destroy:

- Close destroy-profile gaps that `Blocks.java` property parsing cannot reach:
  constructor-level mutations and arbitrary helper/lambda evaluation
  (`InfestedBlock`, `InfestedRotatedPillarBlock`).
- Validate exact pose/fluid nuance for player destroy speed (standing-eye water
  probe).
- Implement block-specific `state.attack` callbacks and hit particles.
- Implement full model-shaped crack decals over `destroy_stage_0..9`.
- Close the remaining `STOP_DESTROY_BLOCK` sequencing gaps.

Commands:

- Add focused command-queue and encode tests for inventory, interaction, chat,
  command, and sign editing.
- Complete local attack input parity from `Minecraft.startAttack` and
  `MultiPlayerGameMode.attack`: hands-busy/feature/item gates, miss-time,
  carried-item ordering, client presentation from `Player.attack`, and attack
  strength reset. Damage and combat outcomes remain server-authoritative.
- Complete local entity-use parity from `Minecraft.startUseItem` and
  `MultiPlayerGameMode.interact`: exact main/offhand iteration until a local
  `InteractionResult.Success`, hands-busy and enabled-item gates, carried-item
  ordering, local `interactOn` prediction, swing-source handling, and held-item
  presentation. Entity state and inventory corrections remain server-authoritative.
- Implement `ServerboundSignUpdatePacket` clipboard copy/cut/paste parity.

Inventory:

- Complete component-rich creative pick-item response parity. Middle-click/Ctrl
  input, block/entity target selection and both serverbound packets are covered;
  the server remains authoritative for reach, creative/game-master data access,
  hotbar selection and inventory mutation. Remaining added-value ItemStack
  components in SetPlayerInventory/container broadcasts can still lose clone
  data and therefore keep the response family partial.
- Complete the remaining ItemStack added-value component semantics consumed by
  SetEquipment; wire-safe type presence is not sufficient for canonical item
  state or component-driven held/armor rendering.
- Implement the rich-tooltip `space` font provider, the remaining text styles,
  italic/complex component styles, and component-specific detail lines from
  `font/default.json`.
- Implement recipe book and creative inventory variants.
- When a highlighted recipe first becomes visible in a native recipe button,
  remove its local highlight and send `RecipeBookSeenRecipe` exactly once like
  vanilla `RecipeButton.init`/`RecipeBookComponent.recipeShown`. The generic
  encoder and maintenance command exist, but the automatic UI lifecycle does
  not; do not leave highlights permanently animated or treat manual control as
  client parity.
- Complete PlaceGhostRecipe presentation for every remaining SlotDisplay/item
  component semantic. Active Inventory/Crafting/Furnace/Blast/Smoker container
  gating and current crafting/furnace ghost overlays are covered; invalid or
  mismatched container packets are ignored without caching stale displays.
- Implement container `0` crafting-result parity: composite / component-aware /
  special recipe display matching, and recipe-specific remainder items.
- Add the dedicated server-opened menu layouts not covered by the
  `generic_9xN` / `generic_3x3` baseline families.

## Particle runtime vanilla parity

Owner: `bbb-renderer` + `bbb-native` + `bbb-pack`

The fixed vanilla 26.1 particle type id/resource/override-limiter catalog is
generated. Particle definition JSON and atlas sprites remain resource-pack-owned
runtime catalogs rather than fixed vanilla snapshots; the remaining items below
are provider and presentation behavior.

- Implement full vanilla provider behavior and presentation parity. New gaps
  enter through the per-provider coverage matrix at the end of this file: add
  the row, or flip the cell back to `todo`, before cutting the slice.
- Implement option-driven atlas rendering and provider-specific option effects
  beyond spell, for `dust` and `dust_color_transition`.
- Finish the terrain/block particle GPU presentation refinements around
  `addDestroyBlockEffect` and terrain tint.
- Keep missing definition/sprite diagnostics: native spawn resolution and
  renderer counters must record them rather than dropping otherwise valid spawns.

## Renderer scene parity

Owner: `bbb-renderer` + `bbb-native` + `bbb-pack` + `bbb-world`

- Implement GameTest block highlight marker presentation: retain multiple markers
  keyed by absolute position, refresh their 10-second expiry, render the padded
  translucent cuboid and always-on-top relative-position text, and clear them on
  level reset. The current last-position diagnostic snapshot is not a gizmo.
- Complete `ClientPacketListener.createEntityFromPacket` parity for AddEntity:
  resolve entity types through session RegistryAccess, enforce the PlayerInfo
  gate for player creation, and cover every type-specific `recreateFromPacket`
  initialization and post-add client sound path.
- Complete ClientLevel block-event dispatch for the remaining client
  `triggerEvent` families (including piston and note-block behavior), resolving
  the packet block holder through session RegistryAccess and validating it
  against the current block before applying the event.

Submission surfaces:

- Add renderer submission surfaces for generic name-tag text, sign/display text,
  and block-feature submissions, then wire their draw ordering into vanilla's
  `order(1).submitText` phase. Texture-backed special geometry added by these
  surfaces must record complete submission metadata before atlas lookup rather
  than reuse the no-texture custom-geometry path.

Living-entity render state:

- Track and apply the fall-flying and swimming `head.xRot` overrides in the
  head-look projection; both currently default upright.
  `LivingEntityRenderer.isShaking` reads it, but `conversionTime` is currently
  server-side only.
- Project guardian eye target tracking (`lookAtPosition` / `lookDirection` /
  `eyePosition`) from entity-side state.
- Implement armor-stand animation interpolation.

Models and layers:

- Replace the placeholder raw-cuboid `Humanoid` fallback with a vanilla-faithful
  part-list model.
- Implement the happy-ghast baby model (extra `inner_body` cube, `0.2375` baby
  scale), the `bodyItem` body squeeze (`0.9375` scale with harness), the harness
  equipment layer, and the rope/lead layer, per
  `HappyGhastModel.createBodyLayer(false, NONE)`.
- Port the keyframe `AnimationState` start-tick phase offset for the bat flap
  (`BatAnimation.BAT_FLYING`).
- Implement the remaining zombie-family and piglin-family armor nuances and
  held-item refinements.
- Implement player arrows/stingers stuck in body, name display, swim arm poses,
  and elytra `speedValue` poses. The client does not track them yet.
- Implement villager/illager live profiled-player skin presentation and the
  illager arm pose.
- Implement equine boost presentation and the remaining non-base-equine
  presentation.
- Implement slime and magma-cube particle/audio coupling and crumbling.

Attachment and spawner presentation:

- Wire the camel body-anchor passenger/leash attachment consumers. The anchor
  provider `Camel.getBodyAnchorAnimationYOffset` already exists.
- Implement the ominous-item-spawner about-to-spawn and spawn sounds.
- Add the exact falling-block `MovingBlockRenderState` biome / cardinal /
  random-seed details consumed by `FallingBlockRenderer`.

Environment:

- Complete custom-dimension `EnvironmentAttributes` beyond the current direct
  fog/lightmap visual projection: modifier compounds, remaining visual/audio/
  gameplay attributes, and timeline/default-clock layers are still open.
- Implement the creeper/spider/invert spectator post chains, the independent
  F4 `effectActive` toggle, and the perspective-change clear lifecycle. The
  renderer-owned current id currently drives only the debug entry mirror.

## HUD overlay and screen render surfaces

Owner: `bbb-renderer` + `bbb-native` + `bbb-world`

- Implement `PlayerSocialManager` state and the Social Interactions screen,
  including add/remove cleanup driven by PlayerInfo packets and blocked/hidden
  player filtering. Current player-info/listed state is not a social manager.
- Render tracked waypoints through the vanilla locator contextual bar: load
  waypoint-style assets, derive distance/yaw/pitch indicators, and apply
  `Gui.nextContextualInfoState` priority rules.
- Implement the multiplayer `SpectatorGui` menu surface: root/player/team
  categories, eligible-target filtering, paging, hotbar selection and fade/
  close behavior. Spectator crosshair-click + `ServerboundSpectateEntityPacket`
  and server-confirmed SetCamera handling are covered, but that shortcut and a
  maintenance command do not replace the vanilla spectator menu.
- Preserve written-book filtered pages and full Component/style/click-event
  semantics in `BookViewScreen`; the current OpenBook path gates on the held
  written/writable component and supports literal-page navigation, but flattens
  pages to strings and does not execute page click events.
- Implement the writable `BookEditScreen` owner: edit/paginate text, map the
  selected hotbar/offhand inventory slot, require the held writable-book
  component, submit unsigned/signed edits, and project the server-confirmed
  writable/written item. The current `net.edit_book` maintenance command is not
  an interactive editor or a held-item authority check.
- Implement the game-master `JigsawBlockEditScreen` owner and canonical jigsaw
  block-entity projection: preload/edit name, target, pool, final state, joint
  and priorities; validate resource ids; expose the 0..20 generation-level and
  keep-jigsaws controls; send SetJigsawBlock before JigsawGenerate; and let
  ordinary server block/chunk updates remain authoritative for generated output.
- Implement the game-master `CommandBlockEditScreen` owner and typed
  `ServerboundSetCommandBlockPacket`: preload the server-supplied command,
  previous output, mode/conditional/automatic/track-output state; preserve the
  vanilla permission and command-block-enabled diagnostics; encode BlockPos,
  bounded command text, mode ordinal and flag bits; and leave command execution,
  block replacement and block-entity updates server-authoritative. Packet catalog
  presence alone is not an editor or command-block state implementation.
- Implement the game-master `MinecartCommandBlockEditScreen` owner and typed
  `ServerboundSetCommandMinecartPacket`: consume the command/last-output synced
  metadata for the target minecart, preload command and track-output state,
  encode entity id + bounded command text + track-output, and retain server
  permission/entity-type/command-block-enabled authority. Generated metadata
  schema and packet catalog presence are not an interactive editor.
- Implement the game-master `StructureBlockEditScreen` owner and typed
  `ServerboundSetStructureBlockPacket`: preload the canonical block-entity
  fields, validate and clamp name/offset/size/data/integrity/seed, expose
  update/save/load/scan plus mirror/rotation/flag controls, and preserve server
  permission, structure storage, world mutation and feedback authority. The
  multiplayer client must not own structure save/load operations.
- Implement the game-master `TestBlockEditScreen` owner and typed
  `ServerboundSetTestBlockPacket`: preload mode/message from the canonical
  block entity, expose all TestBlockMode values, enforce the 128-character
  message limit and START visibility rule, and retain server permission,
  block-entity mutation and block-update authority.
- Complete `AbstractSignEditScreen` parity after the OpenSignEditor authority
  gate: choose filtered sign messages, enforce the 90-pixel font-width limit,
  support clipboard/IME preedit, and close when the sign is removed or the
  player is too far away. The current editor has front/back preload, editing,
  preview and serverbound submit, but retains a defensive character-count cap.
- Preserve and render full trusted Components for action-bar, title, and
  subtitle packets, including translation, siblings, style/color/font and text
  effects. Their wire is consumed and the vanilla timers/fades/scales are
  implemented, but canonical state currently projects only a readable String.
  Restart by sharing the repo-native rich Component/text layout path rather than
  adding packet-specific style fields.
- Preserve full trusted Components in the TabList header/footer and complete
  remaining player-list presentation parity: encrypted-session player heads,
  LIST objective scores/hearts, narrator visibility announcements, and vanilla
  ping sprites. The current overlay covers Tab visibility, vanilla ordering,
  80-entry/20-row columns, plain header/footer wrapping and ping thresholds.
- `chunk_section_paths`, `chunk_section_octree`, and
  `chunk_section_visibility` are `deferred` until the renderer retains
  per-section residency plus a repo-native `SectionOcclusionGraph`/octree and
  compiled-section face-visibility matrix. Restart when terrain upload no
  longer collapses each chunk into layer-wide meshes and can expose vanilla
  node step/source directions, octree node bounds/depth/frustum state, and
  `facesCanSeeEachother` without reconstructing them heuristically.
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
  branch are covered; locally controlled vehicle physics stays tracked under
  the existing full boat/vehicle movement item.
- Entity-hitbox local-server green boxes and delta arrows are `not-needed`:
  vanilla 26.1 `EntityHitboxDebugRenderer.getServerEntity` returns an entity
  only through `Minecraft.getSingleplayerServer()` -> `IntegratedServer` ->
  `ServerLevel.getEntity`; an external multiplayer client always receives
  `null` and emits the red `Missing Server Entity` 3D billboard instead. bbb
  owns no integrated server and implements that missing-label branch. Reopen
  with an in-process integrated server boundary.
- Extend Shift-local F3+I entity capture only for fields actually observed from
  synced metadata, spawn/equipment/components, client-owned runtime, and
  protocol-backed references. Ordinary F3+I already waits for the matching
  server tag-query response, which is the complete NBT authority; private
  owner/target/inventory/timer/RNG fields are not client work and must not be
  synthesized from defaults. Remaining observable work includes registry-backed
  entity-data projection (the fixed vanilla entity/block-entity type catalogs
  are generated; server dynamic-registry ordering remains), packed variant
  projection, remaining codec-backed SNBT projection (component removals are complete;
  added values still include `AbstractArrow`
  `inBlockState` and `item`, remaining data-component patch entries (including
  attribute-modifier override-display Component trees, direct instrument,
  direct trim material/pattern, direct provides-trim-material, direct jukebox-song,
  and direct painting-variant network holders; implemented
  through the protocol generated-metadata/handler descriptor catalog and native
  persistent SNBT encoder registry)
  beyond the supported custom-NBT/primitive (including repair-cost/enchantable/glider,
  minimum-attack-charge/potion-duration-scale)/
  enchantments/stored-enchantments/
  adventure-mode-predicate (block/state/NBT/exact/partial matchers; its 15
  predicate-type ids/resources are generated)/
  equippable (slot/sound/asset/entity-holder/default flags)/
  blocks-attacks (damage-reduction/item-damage/damage-type-holder/sound)/
  attribute-modifiers-named-holder-default-hidden-display/
  simple-resource (including tooltip-style/note-block-sound and vanilla/direct
  break-sound)/exact-component-tree (custom-name/item-name/lore)/
  food/consumable/death-protection/potion-contents/suspicious-stew-effects/
  damage-resistant-tag/weapon/
  tool-block-holder-rules/
  damage-type-named-holder (fixed vanilla bootstrap catalog is generated;
  dynamic-registry remapping remains)/
  piercing-weapon/kinetic-weapon/
  repairable/use-effects/use-remainder/
  use-cooldown/tooltip-display/attack-range/
  swing-animation/resource/color (including dye/base-color)/map/ominous/firework/custom-model-data/
  codec-backed-NBT (map-decorations/debug-stick/entity-data/bucket-data/block-entity-data/
  recipes/lock/container-loot)/bees/
  charged-projectile/bundle/writable-book/written-book-exact-component-trees/
  lodestone/profile/banner-pattern-provider-tag/
  banner-pattern-layers/
  trim-named-holders/
  instrument-named-holder/
  provides-trim-material-named-holder/
  jukebox-playable-named-holder/
  painting-variant-named-holder/
  pot-decoration/
  container/block-state/villager-variant/wolf-variant/wolf-sound-variant/
  pig-variant/pig-sound-variant/cow-variant/cow-sound-variant/
  chicken-variant/chicken-sound-variant/zombie-nautilus-variant/frog-variant/
  cat-variant/cat-sound-variant named holders/
  built-in animal/color enum (wolf/cat collars, fox,
  salmon, parrot, mooshroom, rabbit, horse, llama, axolotl, sheep, shulker,
  and tropical-fish) subset on throwable/firework/
  item-frame item stacks, falling-block `TileEntityData`,
  and non-default `VibrationSystem.Data` listener payloads),
  remaining entity-specific
  constructor/default save values, LivingEntity parent health/equipment/effect/attribute
  projection, observable non-empty entity inventory/container projection
  (including chest/hopper minecart `Items` / loot-table contents, and chest
  boat/raft `Items` / loot-table contents), non-default furnace minecart
  push/fuel fields,
  spawner minecart non-default delay/count/range fields and codec-backed
  `SpawnData` / `SpawnPotentials`, command-block minecart non-default success/
  track-output/custom-name/last-execution fields and exact styled `LastOutput`
  component projection, or entity-specific age and equipment projection before
  it can be emitted.
- Cover the full vanilla profiler section set, and add the profiling metrics
  recorder/output.
- Finish `DebugOptionsScreen` narration, focus, and widget styling.
- Implement native pause tick-freeze eligibility and the remaining `PauseScreen`
  actions and subscreens.

## Item model range-dispatch and select projection

Owner: `bbb-protocol` + `bbb-native` + `bbb-pack`

Each item below plugs into the existing value-aware `RangeDispatch` / `Select`
resolver by adding a value provider. No new selection machinery is required.

- Thread newly discovered ambient-context numeric `range_dispatch` properties
  through the icon resolver as that state becomes available to the GUI icon path.
- Wire `minecraft:context_entity_type` for non-GUI item consumers that gain a
  real living owner outside the current owner-backed generated held-item path.
- Complete `minecraft:local_time`: full localized symbols and the long-tail ICU
  pattern fields — locale-specific week data beyond the selected English
  regional groups, IANA long `z`, generic `v`, and one-/four-letter `V` widths.
- Extend `minecraft:component` select to complex object/list component values,
  style-sensitive `Component` equality, registry-backed component value codecs,
  custom/datapack component defaults, custom datapack component value decoding,
  and components without a persistent codec.
- Pass dynamic registry keys into the item resolver for the non-GUI item
  consumers that render component-bearing generated stacks where vanilla
  resolves registry-backed item-model properties. No-registry consumers still
  fall back.
- Parse custom enchantment effects, generalizing the registry/effect projection
  beyond `minecraft:quick_charge` and the direct/tag-key enchantment predicates.

## Audio runtime parity

Owner: `bbb-audio` + `bbb-native` + `bbb-pack` + `bbb-world`

- Validate remaining synthesized/game-event sound mappings against vanilla.
- Clientbound SoundEntity holder/category/entity/volume/pitch/seed handling,
  missing or silent entity gates, fixed range, entity following, and removal stop
  are covered.
- Clientbound Sound holder/category/fixed-point position/volume/pitch/seed handling,
  no-distance-delay behavior, fixed range, registry resolution, seeded selection,
  and spatial attenuation are covered.
- Clientbound StopSound covers source/name/all four filter combinations and unknown
  flag bits. Filtered stops affect active sounds only; stop-all also clears queued
  distance-delayed sounds, matching SoundEngine rather than over-stopping the queue.
- Cover device/runtime diagnostics.
- Keep unit tests independent of an audio device.

## Official 26.1 resource-pack coverage

Owner: `bbb-pack`

- Consume atlas mip animation metadata in the renderer, beyond the current
  static sprite use.
- Apply full item tint parity for dynamic sources across every item rendering
  path.
- Load equipment asset layers for armor/equipment rendering.

Loader invariants (not todos): keep the local vanilla coverage tests
(`loads_all_local_vanilla_atlases`, `loads_local_vanilla_item_model_catalog`,
`loads_local_vanilla_item_registry`,
`loads_local_vanilla_equipment_asset_catalog`) green when changing pack loaders;
add parser support only when an official asset or a focused resource-pack
fixture fails them; keep precedence/filter tests next to the loaders.

## Resource and dynamic texture generalization

Owner: `bbb-pack` + `bbb-native` + `bbb-renderer`

- Implement broader non-profile dynamic texture loading.
- Resolve player-head block-entity `profile` owner skins through that pipeline.
  Profileless player heads use the vanilla default player skin fallback.
- Compose resource-pack overrides, custom models, and datapack registry assets
  dynamically.
- Present custom/datapack `minecraft:chicken_variant` / `minecraft:pig_variant`
  / `minecraft:cow_variant` variant assets.
- Give failed, pending, and ready states an explicit fallback. Never draw a
  stale texture.

## Bundle selected-item icon state

Owner: `bbb-protocol` + `bbb-world` + `bbb-native` + `bbb-pack`

- Extend renderer/UI coverage of the selected-item icon beyond hotbar icon
  snapshots.

---

# Deferred

Real gaps, intentionally not scheduled. Each carries the condition that turns it
back into a todo. Do not delete these.

## Screens and presentation

- Complete client game-event types 4/5/13: render the typed invalid-respawn-block
  client-system message already stored by type 0, open WinScreen and send PerformRespawn on completion, implement
  demo intro/hints with chat+narration, and finish the level-load tracker's section-ready/30-second
  timeout/500ms close-delay stages after type 13 advances it to WaitingForPlayerChunk. The
  other ten fixed 26.1 event types have canonical/presentation owners; none of
  these four require an IntegratedServer.
- Implement the in-world GameRules screen request/response lifecycle: apply only
  the first GameRuleValues response while that screen is active, resolve keys
  through the built-in game-rule catalog, typed-deserialize valid values, ignore
  unknown/invalid entries, remove loading state, and submit changed rules. The
  current unconditional key/string diagnostic snapshot is not rule state.
- **Test-instance block status screen update.** Preserve the full status
  Component and apply the optional size only while a native
  `TestInstanceBlockEditScreen` equivalent is active, matching
  `ClientPacketListener.handleTestInstanceBlockStatus`. The world currently
  retains a plain-text diagnostic snapshot. Restart when the test-instance
  block edit screen has a native UI owner.
- **Server-data MOTD/icon parity.** Preserve the full trusted MOTD Component,
  validate optional icon bytes like `ServerData.validateIcon`, and update the
  persisted multiplayer server-list entry when that owner exists. The current
  client stores a readable MOTD plus bounded raw icon bytes and correctly keeps
  the prior icon when an update omits it. Restart with repo-native rich text and
  multiplayer server-list persistence rather than treating packet receipt as UI
  completion.
- **Code of Conduct screen presentation parity.** Canonical UI state and control
  requests are covered. Restart when the renderer UI stack grows fuller vanilla
  screen and font rendering.
- **Creative inventory-tab entity preview.** Restart when creative screen state
  exists.
- **Entity-in-UI preview `item_layers` GPU drawing**, per-preview override-camera
  orientation, and PIP-camera glint scroll animation time
  (`HudEntityPreviewItemLayer` / `GuiEntityRenderer`). Hand/head item models need
  a native-side baked item-quad handoff. Restart with GUI surface work.
- **Entity-in-UI item/entity lighting contexts.** Restart with GUI surface work.
- **Mount (`MountScreenOpen`) entity preview rendering.** Horse/Nautilus type
  gating, menu ownership and native inventory layouts are covered; restart with
  entity-in-UI presentation work for the remaining preview drawing.
- **Full Spectator GUI selection/menu behavior.** The wheel currently adjusts
  local flying speed while the Spectator GUI menu is inactive.

## Fonts and text

- **`unihex` / CJK.** The consumed assets tree ships no `font/unifont*.zip`
  archive, so the `include/unifont` reference cannot resolve its data;
  codepoints outside the bitmap pages degrade to the `?` replacement glyph.
  Restart when a unifont source exists in the consumed assets.
- **Bidirectional shaping.** Vanilla routes through ICU4J `ArabicShaping` /
  `Bidi` in `Font` / `StringSplitter`; current consumers render logical order
  only. Restart with the rest of rich text layout.

## Container menu presentation parity

- UpdateRecipes property sets and stonecutter replacement are covered; remaining
  SlotDisplay/component variants and recipe/menu presentation stay open.
- ContainerButtonClick reaches all four vanilla 26.1 menu owners (lectern,
  enchantment, loom, and stonecutter), with menu-specific hit testing and ordinary
  server container updates remaining authoritative. Finish the client-side
  `clickMenuButton` parity for lectern page state, stonecutter/loom result projection,
  and the complete enchantment item/currency/experience/creative eligibility path;
  the packet has no dedicated acknowledgement.
- ContainerClick encodes all seven generated ContainerInput ids and predicts common
  pickup, quick-move, swap, clone, throw, quick-craft, and pickup-all paths. Keep the
  conservative empty-delta/server-authoritative fallback for unhashable component
  patches and specialized result-slot semantics until their local menu rules are
  complete; ordinary ContainerSetContent/Slot/Data updates remain the correction
  path rather than a synthetic acknowledgement.
- Crafter slot-state requests cover the empty-grid pickup toggle, but still need
  vanilla's hotbar `SWAP` enable path plus immediate local menu data/audio mutation.
  The server's CrafterBlockEntity remains authoritative and has no dedicated ack.
- Preserve the full trusted OpenScreen title Component and finish dedicated
  presentation for every generated vanilla MenuType. The fixed 25-entry menu
  catalog and unknown-id rejection are covered; current canonical state keeps
  styled runs plus a plain title while remaining menu-specific UI gaps stay
  listed below.
- Keep clientbound container lifecycle audits aligned with vanilla even when a
  packet field is redundant: ContainerClose carries an id but the 26.1 client
  closes its current menu unconditionally; matching-id behavior is not an
  additional authority check.
- ContainerSetData is gated by the current `containerMenu` id in vanilla.
  Mismatched property updates must remain ignored rather than creating a future
  synthetic menu or replacing the active one.
- ContainerSetContent follows the same inventory-menu-or-matching-active-menu
  authority gate; a mismatched packet must not update the carried item or create
  synthetic slots. Remaining ItemStack added-value semantics stay tracked above.
- ContainerSetSlot also rejects mismatched nonzero menu ids. Remaining parity
  includes creative-inventory remote-slot synchronization, hotbar pop-time
  presentation, and the ItemStack added-value semantics tracked above.
- SetCursorItem updates carried state outside creative inventory; the creative
  screen suppression branch and remaining ItemStack added-value semantics are
  still open.
- SetPlayerInventory now limits canonical state to vanilla inventory/equipment
  slots 0..42. Tutorial pickup presentation and remaining ItemStack added-value
  semantics are still open.
- PlayerCombatEnter is a strict unit packet whose 26.1 client handler is empty;
  its diagnostic counter is not evidence of an additional combat-mode runtime.
- PlayerCombatEnd likewise has an empty 26.1 handler; its decoded duration may
  remain diagnostic but does not establish a client combat-mode transition.
- PlayerCombatKill still needs the local-player identity gate, DeathScreen versus
  respawn branch, and full rich death-message Component preservation.
- ResetScore owner/nullable-objective decode and all/single-score removal semantics
  are covered; remaining scoreboard gaps are objective/team text and presentation specific.
- SetDisplayObjective slot selection and clear/unknown-objective behavior are covered;
  rich objective/team content remains tracked by the packets that carry it.
- SetObjective still rejects unknown method bytes that vanilla decodes as handler no-ops,
  and its rich Component/NumberFormat values remain summary projections.
- SetScore preserves numeric state and unknown-objective gating, but optional display
  Components and NumberFormat values still need typed, presentation-capable semantics.
- SetPlayerTeam preserves add/change/remove/join/leave membership semantics, including
  moving a player out of the previous team. Unknown method bytes still differ from
  vanilla's handler no-op, and rich team Components remain plain-text projections.
- `BeaconMenu`: confirm/cancel hover state, effect highlighted hover state,
  labels/tooltips.
- `EnchantmentMenu`: animated book model, enchanting glyph text and its
  disabled/highlight coloring, hover tooltips.
- `CartographyTableMenu`: live map pixels and decorations in the preview area,
  invalid-transform prediction when item runtime or map state is unavailable.
- `LoomMenu`: pattern-item component/tag lists beyond the single-pattern vanilla
  items, highlighted pattern buttons, banner preview, max-pattern error overlay.
- `MerchantMenu`: generic button row backgrounds and hover/focus highlight,
  selected row state if a future vanilla source adds one, trade XP result bar,
  component-aware cost predicate rendering, full trade stack decorations and
  hover tooltips, deprecated tooltip behavior, discount strikethrough.
- `LecternMenu`: rich text styles and click events, exact vanilla font wrapping,
  text filtering toggle behavior.
- `SmithingMenu`: generic held-item / non-skull head-item projection for the
  armor stand preview, cycling empty-slot icons, tooltips.

## HUD and debug overlay

- Render the world-border warning vignette from camera distance,
  `warningBlocks`, `warningTime`, lerp speed and lerp target/size delta, matching
  `Gui.extractVignette`. Canonical border fields, tick lerp and forcefield render
  extraction are covered; storing warning fields alone is not their HUD consumer.
- **UpdateAdvancements rich display/toast parity.** Reset/remove/add/known-progress
  tree semantics are covered, but title/description and icon stacks remain summary
  projections, and completion telemetry/toast presentation is not complete.
- **SystemChat rich Component presentation.** Overlay=false now enters canonical
  server-system chat history and overlay=true follows the action-bar path, but the
  trusted Component is still flattened before style/click/hover rendering.
- **Advancement hover title/description boxes** and full scroll/scissor behavior.
- **Heart damage/heal blink flash.** Needs the untracked
  `player.invulnerableTime` (no client-side sync) and the wall-clock
  `displayHealth` / `lastHealthTime` hold, neither reproducible
  deterministically. `HudPlayerHealth` always draws `blinking = false`; sprite
  names are already complete for when it lands.
- **Air bubble-pop sound** (`playAirBubblePoppedSound`, `Gui.java:930-937`).
  Restart when a HUD-side sound sink exists.
- **Boss bar `darken_screen` / `create_world_fog` / `play_music`.** The first two
  sit behind `boss_overlay_should_*` queries with no sky/fog consumer; the third
  has no audio consumer.
- **Boss bar rich names, insertion order, and 100ms progress lerp.** Preserve
  trusted Component runs, retain packet-add order, and match
  `LerpingBossEvent` instead of projecting the latest value immediately.
- **Accessibility text backdrop** (`textWithBackdrop`, option default `0`).
- **Jukebox now-playing path** (`Gui.setNowPlaying`, the only `animate_color=true`
  producer). The flag is already carried end to end.
- **Sound-captions subtitle overlay** (`extractSubtitleOverlay`), distinct from
  the title subtitle drawn inside `extractTitle`.
- **Stratum/blur depth-clear** (`GuiRenderer.java`, before/after blur). Restart
  when a blur-backed screen exists.
- **Client-only debug entries with no HUD line**: LocalDifficulty,
  EntitySpawnCounts, ChunkGenerationStats. Restart when an integrated
  local-server mirror (`ServerLevel`, `NaturalSpawner.SpawnState`,
  `ChunkGenerator` / `RandomState` / `BiomeSource`) exists.

## Block entities and terrain

- **Fast-leaves graphics-option parity.** The current baked sprite-transparency path keeps
  leaves in the cutout-style behavior. Vanilla toggles both `LeavesBlock.skipRendering`
  between adjacent leaves and `ModelBlockRenderer.forceOpaque` from the effective graphics
  option; add the same resource-remesh boundary without reintroducing block-name render-layer
  guesses. The exact 11-block leaves membership is generated and shared already.
- **Light-update queue and section-neighbor rebuild parity.** Packet decode and
  canonical mask/empty-mask merging are covered, but bbb applies updates during
  net-event drain and uses a coarse terrain-upload counter. Vanilla queues them
  through `ClientLevel.queueLightUpdate`, consumes a bounded amount per frame,
  and calls `setSectionDirtyWithNeighbors` for changed standalone light sections.
- **ForgetLevelChunk runtime cache cleanup.** On unload, clear the renderer even
  when the last canonical chunk disappears, remove queued sky/block light for
  every section, and drop typed debug-subscription values for that chunk. World
  chunk-column removal is covered; add runtime tests that stale terrain, light and
  debug presentation cannot survive the packet.
- Decode synchronized biome environment-attribute modifier compounds and the
  remaining positional visual fields beyond current fog/sky/water-fog colors
  and water-fog end distance; then match vanilla's 3x3 neighboring-section
  rerender/tint invalidation. Direct custom climate and color effects already
  project through the active holder order; malformed synchronized data is
  omitted instead of falling back to a same-name pack default.
- **Block-entity `breakProgress` crumbling** and per-block-entity distance/frustum
  culling. A cross-cutting boundary shared by every block-entity renderer slice:
  block entities currently submit like entities, unculled.
- **Banner pattern no-depth-write pipeline.** Vanilla's `bannerPattern` pipeline
  disables depth writes; bbb's shared translucent pipeline keeps them on.
  Equal-depth `LessEqual` layering is unaffected. Restart only if a dedicated
  no-depth-write pass matters.
- **Sign glowing 8-way outline glyph pass.** Glowing text renders full-bright
  without outlines.
- **Sign underline/strikethrough effect bars.** They need a white pixel outside
  the single font-atlas draw.
- **Sign obfuscated glyph cycling.** The face currently draws literal glyphs.
- **Sign `filtered_messages` decoding.**
- **Sign POLYGON_OFFSET font display mode**, currently approximated by the fixed
  `TEXT_OFFSET` z gap.
- **General block-entity record removal on block change.** Only `sign_text`
  prunes today.
- **Chest christmas Dec 24-26 texture swap.** There is no wall-clock input;
  `bbb-world` serializable state must not introduce one.
- **Trial-spawner display-state behavior**, and custom `SpawnData` entity NBT
  variants, which render through default entity metadata. Restart when broader
  synthetic entity NBT projection exists.
- **Biome blend for block-break particles.** They still sample the single centre
  biome, unlike terrain tints which blend over `biomeBlendRadius`.

## Entity models and animation

- **Entity-event subclass dispatch audit.** Enumerate every vanilla 26.1
  client-side `handleEntityEvent(byte)` override and the listener-special cases,
  map each applicable event id to bbb's animation/particle/audio/presentation
  owner, and add a reverse coverage test. Existing focused mappings are not proof
  that every subclass event is covered; server-authoritative effects stay out of
  this client-only matrix.
- **Simulation-distance death-tick gate.** `ClientLevel.shouldTickDeath` only
  advances a dying LivingEntity's death timer when its chunk's chessboard
  distance from the local player's chunk is at most the server simulation
  distance. bbb stores and displays the synchronized distance, but currently
  advances every tracked death animation. Restart by threading the local-player
  chunk and synchronized distance into the death-timer branch only; do not gate
  unrelated client animation or remote interpolation ticks.
- **Baby `ageScale`** — the `0.5` proportions applied in model `setupAnim`,
  distinct from the projected `SCALE`-attribute `scale`.
- **Armor trims and the remaining mob-specific armor models.**
- **Datapack/custom item prototype default `SWING_ANIMATION`** beyond the
  resolved vanilla spear ids.
- **Exact humanoid arm-pose idle-bob ordering.** The halved `arm.xRot` folds
  bbb's idle bob in, where vanilla applies the bob afterwards. The full
  bob-reorder that makes the multiply exact is the shared deferred convention.
- **Off-arm `EMPTY` reset use-item arm-pose edge.** A near-no-op: at rest the off
  arm's `yRot` is already `0`.
- **GPU outline presentation for `submitOnlyOutline` entity-attached block-model
  layers** (snow-golem carved pumpkin, mooshroom mushroom). Outline-only
  attachment metadata is recorded and ordinary block-quad baking is suppressed.
- **Guardian out-of-water spike withdrawal.** The out-of-water branch
  (`spikesAnimation = random.nextFloat()` each tick) uses unseeded client RNG and
  is not reconstructable, so the value is held steady out of water rather than
  faked.
- **Frog tongue prey-targeting** (`DATA_TONGUE_TARGET_ID`). Not part of the model
  animation.
- **Creaking tearing-down death-flicker** (`hasGlowingEyes`, a client-tick
  toggle).
- **Armadillo roll-up / roll-out shell `cube` channels**, and the baby-specific
  roll keyframes. The shell `cube` stays static while hiding; baby rolls share
  the adult roll defs.
- **Baby axolotl swim/walk/idle keyframe animations.** Vanilla `BabyAxolotlModel`
  is a separate keyframe model.
- **`EnderDragonModel.setupAnim` procedural animation**: neck/tail re-placement
  from `DragonFlightHistory`, wing flap (`flapTime`), jaw open, and the root
  `bounce` y / `z = -48` / `xRot` adjustments. The model renders at the straight
  bind layout, pitch and bounce deferred to identity at rest.
- **Zombie-nautilus variant dynamic-registry reorder path.** The variant resolves
  from synced `DATA_VARIANT_ID` by bootstrap order.
- **Nautilus `NautilusAnimation.SWIMMING` looping undulation** and the
  `AgeableMobRenderer` baby render scale (`0.7`). Both need the keyframe
  machinery plus an `AnimationState`.
- **Rabbit `IDLE_HEAD_TILT` keyframe.** `shouldPlayIdleAnimation` is gated on a
  `random.nextInt(40) + 180` timeout and is not reconstructable client-side.
- **Local TNT movement / fuse countdown simulation.** The current slice consumes
  world entity position and synced/default fuse metadata. Restart with broader
  entity-physics parity.
- **Client projectile presentation movement / gravity / drag tick parity.** Canonical
  `Projectile` parent lifecycle now consumes the current transform, delta, real
  type bounds, and owner mount graph, but bbb does not yet advance every remote
  projectile subclass between server movement packets. This covers only the
  motion/interpolation/trail/lifecycle work performed by the multiplayer client;
  hit resolution, damage, explosions, block changes, and authoritative removal
  remain server-owned. `LeftOwner` is sticky
  and exact once the canonical projectile box separates; completing per-subclass
  client motion will make that separation occur at vanilla's intermediate tick
  instead of the next authoritative transform update.
- **Dedicated dragon-fireball / fishing-hook / experience-orb renderers**: their
  textured camera quads and renderer-specific lighting/tint/line behavior.
- **Minecart spawner animated block-entity display content.** Classified as
  block-entity special renderer presentation.
- **Painting custom geometry** and the exact entity-solid shader/cull split for
  block/painting atlas consumers.

## GPU and resources

- **Fine-grained/coalesced GPU render-state split.** The backend folds compatible
  submissions into atlas buckets. Later GPU work should split the remaining
  coalesced render-type state, dynamic `LightTexture` / darkness-adjusted gamma,
  and diffuse visual parity. Not a blocker for the CPU submission graph.
- **Standalone mip/sampler generalization**, full vanilla mip generation, and
  standalone texture sampler parity. Belongs to resource parity work.
## Movement and interaction

- **Non-player `POWDER_SNOW_WALKABLE_MOBS` entity collision.** Restart when
  locally controlled non-player entities are added.
- **Non-player freeze-immune entity type exceptions** (`canFreeze` nuance beyond
  the local player). Restart when locally controlled entity freezing is added.
- **No-collision hazard `entityInside` effects**: sweet berry bush age-gated
  damage, fox/bee exceptions. Restart when locally controlled non-player entities
  are added.
- **Full item component payload encoding for
  `ServerboundSetCreativeModeSlotPacket`.** The queue handles empty/componentless
  stacks only. Restart when component-rich creative stacks must be sent.

## Particles and level events

- **Explosion packet typed effects.** Decode the registry-dispatched explosion
  particle, sound holder and weighted block-particle list; play the local sound
  and primary particle; and implement `ClientLevel.explosionTracker` presentation.
  Center/radius/blockCount plus local-player knockback are covered, while the
  bounded raw tail is diagnostic only. Do not add client-authoritative damage or
  block mutation.
- **Broader collision-clipping parity** for special contexts beyond the covered
  `WakeParticle` case.
- **Player-coupled particle emitters** beyond the currently covered scoped cases.
- **Broader entity-event particle/audio parity** outside the covered fixed attack
  sounds and Fox family.
- **Broader firework presentation** outside the Starter particle/audio path and
  the `FireworkRocketEntity` client-tick trail.
- **Idle/tick block-entity client effects** (the vault event `3015` context).
- **LevelEvent-adjacent parity owned elsewhere**: terrain/item atlas rendering,
  block-entity client-effect presentation, audio-device/runtime parity. The
  `LevelEventHandler` switch cases themselves are covered.

---

# Not needed

Vanilla 26.1 has no such behavior, or bbb has no surface for it. Each carries the
vanilla evidence. Deleting one of these re-opens work already decided against.

## Packet canonical state

- **Mojang client telemetry callbacks.** `bbb` has no external telemetry sender.
  Vanilla `WorldSessionTelemetryManager.onServerBrandReceived` only stores the
  brand on `WorldLoadEvent` and attempts a Mojang telemetry send; server-brand
  protocol state and local diagnostics remain independently owned.

- **Bundle delimiter world state.** `ClientboundBundleDelimiterPacket` is
  consumed by `PacketBundlePacker` / `BundlerInfo` as connection framing before
  listener dispatch; it has no payload, listener handler, or canonical world
  mutation. Bundle assembly itself remains a connection-runtime todo above.

## Entity models and layers

- **Chicken head look.** `ChickenModel` never animates the head.
- **Freezing white overlay.** The creeper is the only vanilla
  `getWhiteOverlayProgress` producer; the base renderer returns `0`.
  `isFullyFrozen` drives the `setupRotations` body shake, not an overlay.
- **Zoglin conversion shake.** `ZoglinRenderer` has no conversion override.
- **Snow-golem walk-driven swing.** Its `setupAnim` is the head-yaw twist/orbit.
- **Piglin-brute `ADMIRING_ITEM` pose.** The brute has no admire branch.
- **Fox baby model scale.** `Fox.BABY_SCALE = 0.6` scales only the bounding box,
  so the baby uses the standard `0.5` `LivingEntity.getAgeScale`.
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
- **Separate glow-squid emissive overlay/texture.** 26.1 has none; the glow is
  purely a block-light override.
- **Exact guardian tail starting phase.** Vanilla seeds it with a per-spawn
  `random.nextFloat()`, which is non-deterministic. bbb starts at `0.0`; the sway
  dynamics are exact.
- **Exact enderman creepy root-jitter RNG seed.** Vanilla keeps that RNG inside
  the renderer and never exposes the seed over the protocol; bbb uses a
  deterministic Java-LCG-shaped seed from entity id + age while preserving the
  vanilla axis/scale behavior.
- **Exact horse `tailCounter` client seed.** Not protocol-visible; bbb uses a
  deterministic local Java LCG.

## Rendering and screenshots

- **Exact golden pixel parity to vanilla screenshots.** No golden PNG or
  full-frame hash: driver and float jitter make it unreliable.
- **Item-pipeline overlay-texture (`Sampler1`) sampling.** Vanilla's
  `ITEM_SNIPPET` does not bind `Sampler1`; `core/item` samples only the item
  atlas and lightmap, and `RenderTypes.itemCutout` / `itemTranslucent` never call
  `useOverlay()`.
- **Biome `BLOCK_LIGHT_TINT` / `NIGHT_VISION_COLOR` overrides.** Vanilla biome
  JSON does not provide them.
- **Lightning sky flash tinting water fog.** Vanilla does not tint it.

## Block entities

- **Sign `is_waxed` render effect.** Vanilla has none; it gates editing.
- **Banner item / shield pattern path** (`Sheets.SHIELD_PATTERN_BASE`,
  `BannerRenderer.submitSpecial`). Item-model scope, not the block-entity
  renderer.
- **Shulker box open/close sound and interaction blocking.** Gameplay-side, not
  the block-entity renderer.
- **Bell resonation particle/glow chain** (`resonationTicks`, raider search).
  Gameplay-side, belongs with the raid features.

## Particles

- **Spawn-filtering for `minecraft:block_marker`.** Vanilla `BlockMarker.Provider`
  does not reject air, `moving_piston`, or `shouldSpawnTerrainParticles=false`
  states the way `TerrainParticle.createTerrainParticle` does.
- **`moving_piston` rejection in the destroy-block effect** (events `2001` /
  `3008`). Vanilla `addDestroyBlockEffect` does not reject it, unlike
  `TerrainParticle.Provider`.
- **xAux handling in `GustParticle.Provider`.** Vanilla ignores that xAux; the
  `GustSeedParticle.Provider` child still feeds child xAux `age / lifetime`.

## Input, movement, and interaction

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
  Loaders reject them intentionally.

---

# Appendix: per-provider particle coverage matrix

Not history: this is the entry point for particle work. A new provider gap
is opened by adding a row or flipping a cell back to `todo`, before the
slice is cut. `not-needed` cells carry the vanilla evidence and must not be
deleted.


Usage: no open todo cells remain — every cell is `covered` or `not-needed`.
When a new provider behavior gap is found, first add the row (or flip the
affected cell back to `todo` with a root-cause tag), then cut a slice for
it; when the slice lands, flip the cell to `covered` and append the commit
hash to the row's notes. `not-needed` means vanilla 26.1 has no such
behavior for that provider (reason noted); do not pick those rows. Every row was judged
against the vanilla 26.1 class under
`~/Work/mc-code/sources/26.1/net/minecraft/client/particle/` (line references
in notes), cross-checked against the bbb runtime
(`crates/bbb-renderer/src/particles/descriptors.rs` — `collision_size()`,
`moves_without_collision()`, `drip_fluid()`, `required_fluid()`,
`air_downward_acceleration()`, `tick_motion()` —
`crates/bbb-renderer/src/particles/instance.rs`, and the completion history
earlier in this file).

Column definitions:

- collision: world collision/move path, collision AABB bounds
  (`setSize` / `Particle.scale`), `onGround` behavior, `stoppedByCollision`,
  `speedUpWhenYMotionIsBlocked`. `not-needed` when the particle has
  `hasPhysics=false` (clipping and `onGround` are dead in vanilla — `move`
  only translates), a collision-free `move` override, or a tick that never
  calls `move`.
- player-coupled: local player / nearest player / camera position, velocity,
  or view coupling inside the particle or provider tick.
- sounds: sounds triggered by the particle's own tick (`playLocalSound`).
  Spawn-site sounds emitted by packet / level-event / entity-event handlers
  are tracked by those histories, not here.
- removal-gates: removal (or motion gates) driven by block / fluid / entity
  state queries during tick. Collision-driven removal (e.g. leaves
  `onGround`) belongs to the collision column.

Shared todo root causes (one slice may clear several rows):

- `[bounds]`: vanilla per-provider collision AABB (`setSize` /
  `scale`-derived) vs the former default 0.2x0.2 fallback in
  `descriptors.rs collision_size()`. RESOLVED (this slice): the 24 static
  per-provider sizes are now table-driven in `collision_size()` — drip family
  0.01 (DripParticle.java:25), rain / splash 0.01 (WaterDropParticle.java:16),
  bubble / bubble column 0.02 (BubbleParticle.java:22 /
  BubbleColumnUpParticle.java:24), soul 0.3 and firefly 0.3 (`scale(1.5F)` →
  `Particle.scale` `setSize(0.2F * 1.5F)` = 0.3, Particle.java:77-80). The
  remaining collision `todo` rows are `[leaf-bounds]` and `[wake-grow]`.
- `[leaf-bounds]`: FallingLeaves uses a per-spawn random size
  (`setSize(size, size)`, `size = scale * (0.05F | 0.075F)`,
  FallingLeavesParticle.java:41-43) — needs per-instance collision size, not
  a static table entry. RESOLVED (this slice): `from_spawn_command_*` sets the
  collision box to the already-sampled `visual.base_quad_size` (= vanilla
  `size`) for all three leaf providers, reusing the existing `nextBoolean`
  draw so the spawn RNG sequence is unchanged.
- `[wake-grow]`: WakeParticle re-sizes its collision box every tick
  (`setSize(life * 0.001F)`, WakeParticle.java:46-47). RESOLVED (this slice):
  the Wake tick arm updates `collision_width`/`collision_height` to
  `life * 0.001` after `move`, so the growth trails the move exactly as in
  vanilla (initial 0.01 box still comes from `collision_size()`).
- `[nearest-player]`: PlayerCloudParticle pulls toward
  `level.getNearestPlayer(x, y, z, 2.0, false)` — the nearest of all
  players, not only the local one (PlayerCloudParticle.java:51-58).
  RESOLVED (this slice): the native pump now projects a candidate list
  (`particle_player_motion_contexts` — the local player pose plus every
  `VANILLA_ENTITY_TYPE_PLAYER_ID` entity transform, spectators excluded on
  both paths per `EntitySelector.NO_SPECTATORS`, creative kept because
  `filterOutCreative=false`, EntityGetter.java:95-98), and the renderer
  cloud/sneeze tick resolves the strict nearest candidate within 2.0 per
  particle (`dist < range * range`, minimum squared distance,
  EntityGetter.java:74-88; the former local-only path also accepted
  `dist == 4.0`, now excluded). Remote player `delta_movement` comes from
  the entity transform state, matching the pull's
  `player.getDeltaMovement().y` read.

| Provider | collision | player-coupled | sounds | removal-gates | Notes |
| --- | --- | --- | --- | --- | --- |
| AshParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (AshParticle.java:19, BaseAshSmoke last ctor arg); curves covered in history above |
| AttackSweepParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick never calls `move` (AttackSweepParticle.java:28-37) |
| BlockMarker.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` + zero velocity (BlockMarker.java:14-17); vanilla has no spawn filter here — bbb matches |
| BreakingItemParticle.Provider | covered | not-needed | not-needed | not-needed | no `setSize` → default 0.2 bounds correct; generic collision callback; gravity 1.0 |
| BreakingItemParticle.SlimeProvider | covered | not-needed | not-needed | not-needed | as BreakingItemParticle.Provider |
| BreakingItemParticle.CobwebProvider | covered | not-needed | not-needed | not-needed | as BreakingItemParticle.Provider |
| BreakingItemParticle.SnowballProvider | covered | not-needed | not-needed | not-needed | as BreakingItemParticle.Provider |
| BubbleColumnUpParticle.Provider | covered | not-needed | not-needed | covered | `[bounds]` `setSize(0.02F)` (BubbleColumnUpParticle.java:24) now in `collision_size()` (this slice); non-water removal :35-37 covered (`required_fluid`, descriptors.rs:2071-2078) |
| BubbleParticle.Provider | covered | not-needed | not-needed | covered | `[bounds]` `setSize(0.02F)` (BubbleParticle.java:22) now in `collision_size()` (this slice); +0.002 rise / 0.85 damping covered; non-water removal :43-45 covered |
| BubblePopParticle.Provider | covered | not-needed | not-needed | not-needed | no `setSize` → default bounds; direct-gravity-no-friction tick covered |
| CampfireSmokeParticle.CosyProvider | covered | not-needed | not-needed | not-needed | 0.25 bounds special-cased (descriptors.rs:2048-2050); `alpha<=0` removal + random drift covered |
| CampfireSmokeParticle.SignalProvider | covered | not-needed | not-needed | not-needed | as CosyProvider |
| CritParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (CritParticle.java:35) |
| CritParticle.DamageIndicatorProvider | not-needed | not-needed | not-needed | not-needed | as CritParticle.Provider |
| CritParticle.MagicProvider | not-needed | not-needed | not-needed | not-needed | as CritParticle.Provider |
| DragonBreathParticle.Provider | covered | not-needed | not-needed | not-needed | `hasPhysics=false` (DragonBreathParticle.java:34) — the `onGround`/`hasHitGround` branch (:48-55) is dead in vanilla because `move` never sets `onGround` without physics; bbb models the tick formula including the branch |
| DripParticle.LavaHangProvider | covered | not-needed | not-needed | covered | `[bounds]` `setSize(0.01F)` (DripParticle.java:25) now in `collision_size()` (this slice); lava-surface removal :58-64 covered (`drip_fluid`); cooling RGB + hang damping + fall child covered |
| DripParticle.LavaFallProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); `onGround` → land child covered; lava-surface removal covered |
| DripParticle.LavaLandProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); lava-surface removal covered |
| DripParticle.WaterHangProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); water-surface removal covered |
| DripParticle.WaterFallProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); `onGround` → splash child covered; water-surface removal covered |
| DripParticle.HoneyHangProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY` → no fluid gate (DripParticle.java:366-371) |
| DripParticle.HoneyFallProvider | covered | not-needed | covered | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `BEEHIVE_DRIP` on land (DripParticle.java:313-320) covered via renderer particle sound events; `Fluids.EMPTY` |
| DripParticle.HoneyLandProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY` |
| DripParticle.NectarFallProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `onGround` removal covered; `Fluids.EMPTY` |
| DripParticle.SporeBlossomFallProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY` |
| DripParticle.ObsidianTearHangProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY`; glow light covered |
| DripParticle.ObsidianTearFallProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY` |
| DripParticle.ObsidianTearLandProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY` |
| DripParticle.DripstoneWaterHangProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); water-surface removal covered |
| DripParticle.DripstoneWaterFallProvider | covered | not-needed | covered | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); `POINTED_DRIPSTONE_DRIP_WATER` on land (DripParticle.java:156-162) covered |
| DripParticle.DripstoneLavaHangProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); cooling RGB covered; lava-surface removal covered |
| DripParticle.DripstoneLavaFallProvider | covered | not-needed | covered | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); `POINTED_DRIPSTONE_DRIP_LAVA` on land covered |
| DustParticle.Provider | covered | not-needed | not-needed | not-needed | no `setSize` → default bounds; `hasPhysics=true` + `speedUpWhenYMotionIsBlocked` via generic path |
| DustColorTransitionParticle.Provider | covered | not-needed | not-needed | not-needed | as DustParticle.Provider; transition lerp covered |
| DustPlumeParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (DustPlumeParticle.java:22, BaseAshSmoke last arg); gravity/friction decay covered |
| ElderGuardianParticle.Provider | not-needed | not-needed | not-needed | not-needed | zero velocity, gravity 0 — base tick moves nothing; special-group render covered |
| EndRodParticle.Provider | not-needed | not-needed | not-needed | not-needed | collision-free `move` override (EndRodParticle.java:22-25); in `moves_without_collision` |
| ExplodeParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; physics, gravity -0.1, friction 0.9 |
| FallingDustParticle.Provider | covered | not-needed | not-needed | not-needed | custom tick move + `onGround` roll reset covered; spawn `INVISIBLE` rejection + tint covered; no removal gate in vanilla |
| FallingLeavesParticle.CherryProvider | covered | not-needed | not-needed | not-needed | `[leaf-bounds]` per-spawn `setSize(size, size)` (0.05/0.075) now driven by the sampled `base_quad_size` in `from_spawn_command_*` (this slice); `onGround` + horizontal-block removal + first-tick grace covered — removal is collision-driven, no state query |
| FallingLeavesParticle.PaleOakProvider | covered | not-needed | not-needed | not-needed | `[leaf-bounds]` (0.1/0.15) covered (this slice); rest as CherryProvider |
| FallingLeavesParticle.TintedLeavesProvider | covered | not-needed | not-needed | not-needed | `[leaf-bounds]` (0.1/0.15) covered (this slice); ARGB tint covered |
| FireflyParticle.FireflyProvider | covered | not-needed | not-needed | covered | `[bounds]` provider `scale(1.5F)` → `setSize(0.3F)` (FireflyParticle.java:94, Particle.java:77-80) now in `collision_size()` (this slice); generic move + non-air removal :50-51 covered |
| FireworkParticles.FlashProvider | not-needed | not-needed | not-needed | not-needed | zero velocity, lifetime 4; overlay alpha/size covered |
| FireworkParticles.SparkProvider | covered | not-needed | not-needed | not-needed | no `setSize` → default bounds; SimpleAnimated physics via generic path; trail/twinkle children + fade covered; sounds live on the Starter row |
| FlameParticle.Provider | not-needed | not-needed | not-needed | not-needed | collision-free `move` override (FlameParticle.java:29-32); in `moves_without_collision` |
| FlameParticle.SmallFlameProvider | not-needed | not-needed | not-needed | not-needed | as FlameParticle.Provider |
| FlyStraightTowardsParticle.OminousSpawnProvider | not-needed | not-needed | not-needed | not-needed | empty `move` override (FlyStraightTowardsParticle.java:56-57) + `hasPhysics=false`; curve covered |
| FlyTowardsPositionParticle.EnchantProvider | not-needed | not-needed | not-needed | not-needed | collision-free `move` (FlyTowardsPositionParticle.java:74-77) + `hasPhysics=false` |
| FlyTowardsPositionParticle.NautilusProvider | not-needed | not-needed | not-needed | not-needed | as EnchantProvider |
| FlyTowardsPositionParticle.VaultConnectionProvider | not-needed | not-needed | not-needed | not-needed | as EnchantProvider; `LifetimeAlpha` covered |
| GlowParticle.GlowSquidProvider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (GlowParticle.java:19) |
| GlowParticle.WaxOnProvider | not-needed | not-needed | not-needed | not-needed | as GlowSquidProvider |
| GlowParticle.WaxOffProvider | not-needed | not-needed | not-needed | not-needed | as GlowSquidProvider |
| GlowParticle.ElectricSparkProvider | not-needed | not-needed | not-needed | not-needed | as GlowSquidProvider |
| GlowParticle.ScrapeProvider | not-needed | not-needed | not-needed | not-needed | as GlowSquidProvider |
| GustParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick never calls `move` (GustParticle.java:30-36); `setSize(1.0F)` has no behavioral effect |
| GustParticle.SmallProvider | not-needed | not-needed | not-needed | not-needed | as GustParticle.Provider |
| GustSeedParticle.Provider | not-needed | not-needed | not-needed | not-needed | NoRender emitter, no move; child gust emission covered |
| HeartParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (HeartParticle.java:20); speed-up flag dead without collision |
| HeartParticle.AngryVillagerProvider | not-needed | not-needed | not-needed | not-needed | as HeartParticle.Provider; `y + 0.5` offset covered |
| HugeExplosionParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick never calls `move` |
| HugeExplosionSeedParticle.Provider | not-needed | not-needed | not-needed | not-needed | NoRender emitter; child explosion emission covered |
| LargeSmokeParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; smoke-family collision + blocked-Y speed-up covered |
| LavaParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds physics; child smoke emission covered |
| NoteParticle.Provider | covered | not-needed | not-needed | not-needed | `hasPhysics` stays true in vanilla; default bounds; generic collision + speed-up |
| PlayerCloudParticle.Provider | not-needed | covered | not-needed | not-needed | `hasPhysics=false` (PlayerCloudParticle.java:32); `[nearest-player]` nearest-of-all-players pull covered (this slice): native projects local + remote player candidates minus spectators, renderer picks the strict nearest within 2.0 per particle |
| PlayerCloudParticle.SneezeProvider | not-needed | covered | not-needed | not-needed | as PlayerCloudParticle.Provider |
| PortalParticle.Provider | not-needed | not-needed | not-needed | not-needed | collision-free `move` (PortalParticle.java:48-51); curve covered |
| ReversePortalParticle.ReversePortalProvider | not-needed | not-needed | not-needed | not-needed | inherits portal collision-free `move` |
| SculkChargeParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (SculkChargeParticle.java:18); roll covered |
| SculkChargePopParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (SculkChargePopParticle.java:18) |
| ShriekParticle.Provider | covered | not-needed | not-needed | not-needed | `hasPhysics` stays true, default bounds, `yd=0.1`; delay decoded into `initial_delay_ticks`; dual-quad extract covered |
| SimpleVerticalParticle.PauseMobGrowthProvider | covered | not-needed | not-needed | not-needed | default bounds physics; -0.03 y offset covered |
| SimpleVerticalParticle.ResetMobGrowthProvider | covered | not-needed | not-needed | not-needed | default bounds physics; +0.03 y offset covered |
| SmokeParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; blocked-Y speed-up via collision callback covered |
| SnowflakeParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; post-tick damping covered |
| SonicBoomParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick never calls `move` (inherits HugeExplosionParticle) |
| SoulParticle.Provider | covered | not-needed | not-needed | not-needed | `[bounds]` `scale(1.5F)` → `setSize(0.3F)` (SoulParticle.java:17 + Particle.java:77-80) now in `collision_size()` (this slice); rising motion covered |
| SoulParticle.EmissiveProvider | covered | not-needed | not-needed | not-needed | as SoulParticle.Provider; `[bounds]` 0.3 covered (this slice) |
| SpellParticle.Provider | not-needed | covered | not-needed | not-needed | `hasPhysics=false` (SpellParticle.java:33); scoping alpha (:35-37, :49-53, :62-69) covered — vanilla itself only checks the local player |
| SpellParticle.InstantProvider | not-needed | covered | not-needed | not-needed | as SpellParticle.Provider; RGB + power covered |
| SpellParticle.MobEffectProvider | not-needed | covered | not-needed | not-needed | as SpellParticle.Provider; ARGB covered |
| SpellParticle.WitchProvider | not-needed | covered | not-needed | not-needed | as SpellParticle.Provider; magenta brightness covered |
| SpitParticle.Provider | covered | not-needed | not-needed | not-needed | ExplodeParticle physics, gravity 0.5, default bounds |
| SplashParticle.Provider | covered | not-needed | not-needed | covered | `[bounds]` inherits WaterDrop `setSize(0.01F)` (WaterDropParticle.java:16 via `super`) now in `collision_size()` (this slice); `onGround` 50% removal + collision-shape/fluid-surface gate :39-55 covered |
| SquidInkParticle.Provider | not-needed | not-needed | not-needed | covered | `hasPhysics=false`; in-air downward drift gate :43-45 covered (`air_downward_acceleration`) |
| SquidInkParticle.GlowInkProvider | not-needed | not-needed | not-needed | covered | as SquidInkParticle.Provider |
| SuspendedParticle.UnderwaterProvider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false`; `setSize(0.01F)` has no behavioral effect without physics |
| SuspendedParticle.SporeBlossomAirProvider | not-needed | not-needed | not-needed | not-needed | as UnderwaterProvider; `ParticleLimit.SPORE_BLOSSOM` covered |
| SuspendedParticle.CrimsonSporeProvider | not-needed | not-needed | not-needed | not-needed | as UnderwaterProvider |
| SuspendedParticle.WarpedSporeProvider | not-needed | not-needed | not-needed | not-needed | as UnderwaterProvider; `setSize(0.001F)` no effect |
| SuspendedTownParticle.Provider | not-needed | not-needed | not-needed | not-needed | collision-free `move` (SuspendedTownParticle.java:38-41); in `moves_without_collision` |
| SuspendedTownParticle.HappyVillagerProvider | not-needed | not-needed | not-needed | not-needed | as SuspendedTownParticle.Provider |
| SuspendedTownParticle.ComposterFillProvider | not-needed | not-needed | not-needed | not-needed | as SuspendedTownParticle.Provider |
| SuspendedTownParticle.DolphinSpeedProvider | not-needed | not-needed | not-needed | not-needed | as SuspendedTownParticle.Provider |
| SuspendedTownParticle.EggCrackProvider | not-needed | not-needed | not-needed | not-needed | as SuspendedTownParticle.Provider |
| TerrainParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds, gravity 1.0, generic collision; spawn filter (air / moving_piston / `shouldSpawnTerrainParticles`, TerrainParticle.java:92) covered |
| TerrainParticle.DustPillarProvider | covered | not-needed | not-needed | not-needed | as TerrainParticle.Provider; pillar speed/lifetime covered |
| TerrainParticle.CrumblingProvider | covered | not-needed | not-needed | not-needed | as TerrainParticle.Provider; zero speed + lifetime covered |
| TotemParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; SimpleAnimated physics, friction 0.6 / gravity 1.25 |
| TrailParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick lerps toward fixed target, no `move` |
| TrialSpawnerDetectionParticle.Provider | covered | not-needed | not-needed | not-needed | `hasPhysics=true` (TrialSpawnerDetectionParticle.java:39), default bounds; LOOKAT_Y covered |
| VibrationSignalParticle.Provider | not-needed | not-needed | not-needed | covered | no `move`; entity-target-missing removal :77-79 covered via native entity target context |
| WakeParticle.Provider | covered | not-needed | not-needed | not-needed | `[wake-grow]` initial `setSize(0.01F)` covered (descriptors.rs:2051) + per-tick `setSize(life * 0.001F)` growth :46-47 now applied after `move` in the Wake tick arm (this slice) |
| WaterCurrentDownParticle.Provider | not-needed | not-needed | not-needed | covered | `hasPhysics=false` (WaterCurrentDownParticle.java:17) — the `onGround` half of :45 is dead in vanilla; non-water removal covered |
| WaterDropParticle.Provider | covered | not-needed | not-needed | covered | `[bounds]` `setSize(0.01F)` (WaterDropParticle.java:16) now in `collision_size()` (this slice); move + `onGround` 50% removal + collision-shape/fluid-surface gate covered |
| WhiteAshParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (WhiteAshParticle.java:22) |
| WhiteSmokeParticle.Provider | covered | not-needed | not-needed | not-needed | as SmokeParticle.Provider |
| TrackingEmitter (code-spawned) | not-needed | not-needed | not-needed | not-needed | NoRender emitter, no `move`; entity-AABB sampling covered for the wired paths (entity event 35, animate 4/5); other vanilla construction sites belong to their entity-event slices |
| FireworkParticles.Starter (code-spawned) | not-needed | covered | covered | not-needed | no `move`; camera-distance far-variant selection (FireworkParticles.java:284-287) covered; life-0 blast / large_blast (:216-238) + delayed twinkle (:273-281) covered |
| ItemPickupParticle (code-spawned) | not-needed | covered | not-needed | not-needed | follows target entity midpoint (ItemPickupParticle.java:45-49); covered incl. 3-tick lifetime and quadratic extract |

Row and cell counts (2026-07-05; player-coupled counts updated after the
nearest-player slice cleared `[nearest-player]`, collision counts after the
dynamic-collision-size slice cleared `[leaf-bounds]` + `[wake-grow]`):

- 113 rows: 110 distinct provider classes registered by vanilla 26.1
  `ParticleResources.registerProviders()` (lines 56-172; 117 registrations
  minus 7 duplicate-class registrations) plus 3 code-spawned particles
  (`TrackingEmitter`, `FireworkParticles.Starter`, `ItemPickupParticle`).
- collision: 56 covered / 57 not-needed / 0 todo (the dynamic-collision-size
  slice flipped the last 4: 3 `[leaf-bounds]` leaf providers now sized from the
  sampled `base_quad_size`, and `[wake-grow]` Wake now grows `life * 0.001`
  per tick)
- player-coupled: 8 covered / 105 not-needed / 0 todo (the nearest-player
  slice flipped the last 2: PlayerCloud + Sneeze now pull toward the
  nearest of all players, not only the local one)
- sounds: 4 covered / 109 not-needed / 0 todo
- removal-gates: 18 covered / 95 not-needed / 0 todo
- 0 todo cells — the table is fully `covered` / `not-needed`; new provider
  behavior gaps re-enter as new `todo` cells (see Usage above).
