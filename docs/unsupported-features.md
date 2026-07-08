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
- Status: `covered`
- Next action:
  - Preserve `WorldStore::apply_play_packet` as the single shared decoded
    play-packet application path. When adding a new play-packet side effect,
    keep sink-less `PlayApplyEffects` callers and native sink-driven callers on
    the same deterministic random/context path.
- Evidence / boundary:
  - `WorldStore::apply_play_packet` is the single shared clientbound
    play-packet -> canonical-world mapping. Both the offline probe
    (`bbb-net/src/probe/play.rs`) and the online dispatcher
    (`bbb-native/src/runtime/events/dispatcher.rs`, via `NetEvent::Play`)
    delegate to it; per-family packet arms exist only there.
  - Runtime side effects (audio/particle sinks, chat/vehicle
    acknowledgements) flow through the `PlayApplyEffects` trait; the
    deterministic level-event sound/particle random stream advances
    identically for sink-less callers, including potion break, dragon
    fireball, wax-on, vault, trial spawner, and sculk charge events that the
    probe previously skipped.
  - Sink-less LevelEvent random context now uses world-owned read-only chunk
    probes: sculk-charge pop asks the existing world collision shape path
    whether the event block is a full collision block, and plant-growth
    randoms use the same vanilla `BoneMealItem.addGrowthParticles` mode
    classification (`NEIGHBOR_SPREADER` / `GROWER` / water) before sound
    seeds are drawn. Probe tests cover both a loaded full-block sculk pop
    context and a loaded water growth context.
  - Connection-owned packets (keepalive/ping, chunk batch feedback, cookies,
    configuration handoff, resource-pack responses, movement responses,
    disconnects, unknown packets) are returned to the caller and stay
    net-context-owned by design.
  - The final criterion:
    - every supported decoded packet stays aligned

### Renderer Frame Extraction Timing

- Owner: `bbb-native`
- Status: `current field list audited`
- Next action:
  - No known RendererFrame / adjacent renderer-state extraction interleaves
    remain in the current audit. When adding a new world -> renderer field or
    state update, verify its extraction point against the vanilla tick -> render
    frame order in the same slice.
  - A verified field either keeps its position with a vanilla citation on the
    binding, or its `let` moves across the relevant tick advance with the same
    citation.
- Evidence / boundary:
  - `bbb-native/src/runtime/render_extract.rs` owns `RendererFrame` and
    `apply_renderer_frame`.
  - The sky-flash-dependent `lightmap_environment`, `clear_color`,
    `fog_environment`, `sky_environment`, and `cloud_environment` extraction
    now reads after `advance_sky_flash_time`, matching vanilla
    `Minecraft.tick` -> `ClientLevel.tick` -> `GameRenderer.extract` order:
    `ClientLevel.tick` decrements `skyFlashTime`, and render extraction then
    samples the resulting `EnvironmentAttributes` / lightmap state.
  - HUD local-player values (`hud_health`, `hud_food`,
    `hud_experience_progress`, `hud_selected_slot`, hotbar icons, and inventory
    screen projection) now have an explicit source-order test and binding
    comment: vanilla `Minecraft.tick` handles gameplay keybinds before
    `GameRenderer.extractGui` calls `Gui.extractRenderState` /
    `Gui.extractItemHotbar`, so bbb reads these fields after
    `advance_player_input`, destroy/use input advancement, and
    `advance_local_using_item_ticks`.
  - Dropped item models, item entity billboards, entity model instances, held
    item models, item-frame models, and entity block-item models now have a
    source-order test and binding comment: vanilla `Minecraft.tick` advances
    keybinds, `gameRenderer.tick`, and `level.tickEntities` before
    `GameRenderer.extract` calls `LevelRenderer.extractLevel` /
    `extractVisibleEntities`, so bbb reads these fields after entity animation,
    client-time, item-cooldown, input, and local use-item tick advancement.
  - Block-destroy overlays now have source-order and merge coverage: vanilla
    `LevelRenderer.extractBlockDestroyAnimation` reads block-breaking render
    state during render extract, after the client tick, and
    `destroyBlockProgress` stores local `MultiPlayerGameMode` stages and
    server progress in the same per-position sorted set before extracting the
    highest progress. bbb reads `block_destroy_overlays` after
    `advance_block_destruction_render_ticks` and projects the highest
    local/server stage through the official `destroy_stage_0..9` atlas
    entries.
  - Selection outline, entity-scene outline, and entity-target outline now have
    a source-order test and binding comment: vanilla `Minecraft.renderFrame`
    calls `pick(partialTicks)` before `GameRenderer.extract`, and
    `LevelRenderer.extractBlockOutline` then reads `hitResult` plus the current
    camera, so bbb extracts these outline fields after input/use-item/entity
    tick advancement and after the frame camera pose is bound.
  - `cloud_frame` now has a source-order test and binding comment: vanilla
    `LevelRenderer.renderLevel` samples `level.getGameTime()`, the frame
    partial tick, and `cameraRenderState.pos` for `addCloudsPass`, so bbb
    extracts `cloud_frame` after `advance_client_time`, after computing
    `entity_partial_tick`, and after binding the frame camera pose.
  - `weather_render_state` now has a source-order test and binding comment:
    vanilla `LevelRenderer.extractLevel` calls
    `WeatherEffectRenderer.extractRenderState(level, ticks, deltaPartialTick,
    cameraPos, ...)`, and that helper samples rain level, column animation
    ticks, terrain light, and precipitation around the camera, so bbb extracts
    weather after `advance_client_time`, after computing `entity_partial_tick`,
    and after binding the frame camera pose.
  - Particle light refresh now has a source-order test and binding comment:
    vanilla `Minecraft.tick` handles gameplay input before
    `ParticleEngine.tick`, then `LevelRenderer.extractLevel` calls
    `ParticleEngine.extract`; `SingleQuadParticle.extractRotatedQuad` samples
    `getLightCoords(partialTicks)` from the particle's current `BlockPos`.
    bbb now advances particles after input/use-item advancement and refreshes
    particle light after frame extraction inputs are bound, before the renderer
    can collect particle vertices.
  - The renderer receives the whole frame in one commit, so reorders are pure
    extraction-timing questions and cannot introduce partial-frame states.

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

### Vanilla Font Provider Coverage

- Owner: `bbb-item-model` + `bbb-renderer` + `bbb-native` + `bbb-protocol`
- Status: `partial`
- Next action:
  - Deferred only (defer criteria unchanged, documented under Evidence):
    `unihex`/CJK (assets tree ships no `font/unifont*.zip`, codepoints outside
    the bitmap pages degrade to `?`) and bidirectional shaping (vanilla routes
    through ICU4J `ArabicShaping`/`Bidi`). Italic shear and obfuscated
    random-glyph substitution are live (2026-07-05); no font-style work
    remains open.
- Evidence / boundary:
  - Input end is wired (2026-07-05): `bbb_protocol::decode_styled_component_summary`
    flattens chat components into `StyledTextRun`s carrying the vanilla
    `Style` subset (`bold`/`italic`/`underlined`/`strikethrough`/`obfuscated`
    as `Option<bool>`, `color` resolved from named `ChatFormatting` or
    `#RRGGBB` via `TextColor.parseColor`), with `Style.applyTo` inheritance
    down `extra`/`with` children. The plain-text decoder is a pure delegation
    (run concatenation), so every legacy consumer is byte-identical.
    Styled fields ride next to the plain ones with `#[serde(default)]`:
    `OpenScreen.title_styled`, `DataComponentPatchSummary.{custom_name_styled,
    item_name_styled, lore_styled}`, and `ContainerState.title_styled`
    (world projection; the container title itself has no HUD label consumer
    yet).
  - Tooltip projection applies vanilla default styles in `bbb-item-model`
    (`item_runtime/tooltip.rs`): lore lines merge `ItemLore.LORE_STYLE`
    (DARK_PURPLE + italic, `ComponentUtils.mergeStyles` semantics — explicit
    line keys win) and the hover name gets `ItemStack.getStyledHoverName`'s
    rarity-colour wrapper plus ITALIC when a `custom_name` is present.
  - Live rendering: HUD label/tooltip loops consume `HudStyledTextRun`s —
    bold double-quad + `extraThickness` + bold-aware advance, per-run colour
    tint, style-driven shadow colour (`ARGB.scaleRGB(textColor, 0.25)`,
    which also fixes the previously fixed-grey shadow under coloured tooltip
    lines), and underline/strikethrough bars drawn after the line's glyphs
    per pass (vanilla `StringRenderOutput.visit` order). All geometry comes
    from the locked `styled_quads`/`styled_effect_rects`; the item count
    label stays digit-only (vanilla renders counts unstyled).
  - Italic shear is live (2026-07-05): `hud_styled_text_pass_geometry` feeds
    `run.style` straight into `styled_quads`, so italic runs draw the sheared
    corners (top edge `1-0.25*up`, bottom `1-0.25*down`) instead of the old
    upright degrade; non-italic runs are byte-identical (shear no-op).
  - Obfuscated (`§k`) substitution is live (2026-07-05): non-space obfuscated
    glyphs draw a random equal-advance substitute (vanilla
    `Font.getGlyph`/`FontSet.getRandomGlyph`) from a `HudObfuscatedGlyphPool`
    built once per font upload (advance-bucketed `HudFontGlyphMap`, mirroring
    `glyphsByWidth`). Randomness is deterministic, not wall-clock: a
    `HudObfuscatedRandom` (LegacyRandomSource LCG) seeded from the render
    `frame_index` and reset per pass, advanced once per substituted glyph, so
    the shadow pass matches the main pass and a fixed frame yields a fixed
    glyph sequence (per-frame jitter as the counter advances). The pen advance
    always follows the original glyph, so substitution never shifts layout;
    spaces (codepoint 32) are never substituted.
  - Text-style width + draw geometry mechanism is implemented and test-locked
    in `bbb-render-types` (`hud_glyphs.rs`): `HudTextStyle`
    (bold/italic/underlined/strikethrough/obfuscated, all-false default) plus
    `HudDigitGlyph::styled_advance` (vanilla `GlyphInfo.getAdvance(bold)` =
    advance + `getBoldOffset()`=1 per bold glyph), `styled_quads`
    (`BakedSheetGlyph.renderChar` pass order: shadow at `+shadowOffset`=1,1
    first, then main; bold doubles each pass shifted `+boldOffset`=1 with
    `extraThickness`=0.1 on every side; italic shears the top edge by
    `1-0.25*up` and the bottom by `1-0.25*down`), and `styled_effect_rects`
    (`Font.StringRenderOutput.accept`: strikethrough bar `y+3.5..y+4.5`,
    underline bar `y+8.0..y+9.0`, both `effectX0`..`x+advance`, `effectX0` = one
    pixel left for the first glyph in a line). `bbb-renderer`'s width path
    (`hud_font_runs_width`) sums per-run bold-aware advances; the
    default-style path is byte-for-byte the old behavior. Advances stay
    integer (`u32`) so vanilla's `Mth.ceil` over fractional TTF advances is a
    no-op here.
  - The `bitmap` + `space` + `reference` providers of `font/default.json`
    are parsed and baked into one multi-page codepoint-keyed glyph atlas
    (`bbb-item-model/src/font.rs` + `font/providers.rs`), with vanilla
    `BitmapProvider` metrics (`height`/`ascent`/`pixelScale` advance
    formula) and `FontSet` first-provider-wins fallback order
    (`space` -> `nonlatin_european` -> `accented` -> `ascii`); HUD labels,
    tooltips, and map decoration text consume it with `7 - ascent` baseline
    alignment.
  - The `space` provider (`font/include/space.json`: `" "` = 4, `\u200c`
    (ZWNJ) = 0) bakes vanilla `EmptyGlyph` semantics — zero pixel size, pure
    advance — ahead of the bitmap pages, replacing the former hardcoded
    space-advance constant; codepoints resolve it directly rather than
    through the `?` replacement fallback. Advances are parsed as
    `SpaceProvider.Definition`'s `Map<Integer, Float>` but narrowed to `u32`
    since every vanilla entry is a whole number; a fractional/negative
    advance is rejected at parse time (none exist in the shipped assets).
  - `unihex` is deferred: the consumed assets tree ships no
    `font/unifont*.zip` archive, so the `include/unifont` reference cannot
    resolve its data; codepoints outside the bitmap pages (CJK etc.)
    degrade to the `?` replacement glyph until a unifont source exists.
  - Bidirectional text shaping (vanilla routes text through ICU4J
    `ArabicShaping`/`Bidi` in `Font`/`StringSplitter`) is deferred with the
    rest of rich text layout; current consumers render logical order only.

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
  - The per-provider tracking table established 2026-07-05 in the detail
    file (section "Per-provider tracking table") has 0 open todo cells on
    its 113 rows: the nearest-player slice flipped the last 2 player-coupled
    rows (`[nearest-player]` PlayerCloud + Sneeze now pull toward the
    nearest of all players — native projects local + remote player
    candidates minus spectators, the renderer resolves the strict nearest
    within 2.0 per particle), after the `[bounds]` slice closed 24 collision
    rows via `collision_size()` and the dynamic-collision-size slice cleared
    the last 4 collision todos (`[leaf-bounds]` x3, `[wake-grow]`), down
    from the original 30 todo. New provider behavior gaps: add the row /
    flip the cell to `todo` in the table first, then cut a slice; goal.md no
    longer duplicates the list.
  - Implement remaining renderer slices for provider-specific behavior,
    non-particle-atlas terrain/item particle layer sorting, and
    collision/player-coupled physics (world collision clipping, cloud/sneeze
    nearest-player context, and totem/crit/enchanted-hit/entity-event-driven
    tracking emitters are covered so far).
  - Preserve missing definition/sprite diagnostics.
  - Follow-up work in the plan: full vanilla provider behavior and
    presentation parity.
- Evidence / boundary:
  - Current runtime drains level-particle spawn batches, records vanilla
    particle render-plan metadata for covered single-quad providers (atlas
    opaque/translucent split, `ParticleEngine` group order, terrain/item atlas
    sub-rect UVs), uploads a stitched official particle atlas when assets are
    available, and draws active particles as camera-facing textured
    billboards. Per-target ownership follows vanilla
    `ParticleFeatureRenderer`'s
    `useParticleTarget = particleTarget != null && translucent`: opaque
    particles draw into the main color+depth target before the feature-target
    depth copies, and only translucent particles render into the dedicated
    particles target.
- Detailed per-slice history: docs/unsupported/particle-runtime-vanilla-parity.md

### Renderer Scene Parity

- Owner: `bbb-renderer` + `bbb-native` + `bbb-pack` + `bbb-world`
- Status: `partial`
- Next action:
  - Sheep/wolf presentation parity and vanilla dropped-item follow-up
    rendering remain open; continue renderer presentation work with
    deterministic tests or explicit manual-comparison notes.
  - Full entity presentation remains phase 6 work: texture assets, variants,
    equipment, skins, and animation for the remaining entity families
    (boat/raft visual parity beyond the completed water-mask GPU path,
    equine/camel presentation, villager/illager/zombie-family/piglin-family
    live-profile and armor presentation, skeleton armor/held-item/animation,
    creeper overlays, spider walk-animation, enderman primitive presentation,
    copper golem keyframe presentation, armor stand equipment/layers/wiggle,
    slime/magma-cube squish, and precise vanilla mesh parity for
    primitive/placeholder entity families), plus custom/datapack variant
    asset presentation (reclassified as P3 resource/datapack
    generalization).
- Evidence / boundary:
  - P0 GPU submission/pipeline closeout is largely audited: texture-backed
    residual mesh-emitting arms are gone (`entity_models/dispatch.rs` only
    documents non-textured colored fallback/debug geometry), the
    target/post-chain render-graph order (main/translucent/itemEntity/
    particles/weather/clouds/entity_outline) is pinned by tests, and cull /
    translucent / outline GPU submission buckets follow vanilla constants.
    Screenshot/readback now gates the surface format before the copy.
    The world border forcefield now draws into the weather target after
    rain/snow with vanilla `WorldBorderRenderer` mesh/alpha/tint/UV-scroll
    formulas and `RenderPipelines.WORLD_BORDER` state (2026-07-05, detail in
    the per-slice history file). Translucent terrain now draws sections
    back-to-front across chunks (camera-sorted `terrain_translucent_order`),
    matching vanilla's reversed TRANSLUCENT draw list
    (`ChunkSectionsToRender.java:55-56`, MC 26.1); the 2026-07-05 audit
    confirmed within-section/composite/particle/within-target orders were
    already consistent and only this segment order diverged.
    The dying ender dragon body is now GPU-eroded by the DISSOLVE mask: a
    dedicated dissolve mesh/pipeline (`RenderPipelines.ENTITY_CUTOUT_DISSOLVE`)
    samples `dragon_exploding.png` at the model's `texCoord0` and discards
    `if (tint.a < mask.a)` per `entity.fsh`, closing the last dragon-death
    visual-parity gap (deterministic headless GPU readback + CPU mask-UV tests).
  - Entity presentation migration off wrong-model proxies is complete; the
    `EntityRenderState` projection carries packed-light shading, the hurt red
    overlay, creeper swelling, death animation, riptide spin, the
    Dinnerbone/Grumm upside-down easter egg, sleeping pose, uniform model
    scale, walk-animation limb swing, body-shake rotation, and head-look —
    all implemented end to end.
  - GUI entity previews (local inventory player, horse/nautilus mount,
    smithing armor stand) now draw through an actual GPU picture-in-picture
    path (2026-07-05): a dedicated `entity_preview_pip_passes` frame step
    renders each sanitized `HudEntityPreview` entity model into a persistent
    private color+depth PIP target under the vanilla GUI-ortho pose chain and
    `ENTITY_IN_UI` lighting, and the HUD pass blits the texture in vanilla
    GUI submission order; headless GPU readback pins the pixels. Preview
    `item_layers` GPU drawing and the creative inventory tab remain deferred
    (detail in the per-slice history file).
  - Backend GPU resources stay outside `WorldStore`.
- Detailed per-slice history: docs/unsupported/renderer-scene-parity.md

### Terrain Block Presentation Parity

- Owner: `bbb-renderer` + `bbb-native` + `bbb-pack`
- Status: `partial`
- Next action (2026-07-05 entry audit; the umbrella claims in goal.md P2 were
  re-verified and most surfaces are already aligned — see Evidence):
  - `skipRendering` same-block adjacency culling (glass / iron bars): vanilla
    `HalfTransparentBlock.skipRendering` culls the shared face between two
    identical glass-family blocks (`neighborState.is(this)`), and
    `IronBarsBlock.skipRendering` culls between `BARS`-tag blocks per
    connection property (`Block.java:310`). bbb has no equivalent — non-opaque
    (`Cutout`) neighbors never occlude, so adjacent glass renders both internal
    faces. Deferred as a separate slice: it needs a cross-crate data-model
    change (block-family / connection tagging on `TerrainCell`, classified on
    the `bbb-world` side) that the geometry-only per-face occlusion work does
    not touch.
  - Block-entity special renderers (end portal/gateway shader parity;
    player-head owner skin remains under the
    broader dynamic profile/texture pipeline): the chest
    family (2026-07-06), the sign family incl. hanging signs + face text
    (2026-07-06), bed + bell (2026-07-06), shulker box + decorated pot
    (2026-07-06), banner (2026-07-06), the enchanting-table book +
    lectern book (2026-07-07), conduit (2026-07-08; see Evidence), and
    skull/head (2026-07-08; see Evidence), end portal/gateway
    (2026-07-08; see Evidence), and spawner display entity
    (2026-07-08; see Evidence) are DONE as the first ten BER sub-slices;
    BE-driven model sources are now clear. Boundary: end portal/gateway cubes
    currently preserve the vanilla face source, transform, render type
    metadata, and gateway beam geometry, but their cube surfaces use a
    position-color approximation until the renderer grows dedicated
    `RenderTypes.endPortal()` / `endGateway()` shader parity (`end_sky`,
    `end_portal`, `PORTAL_LAYERS` 15/16).
    Vanilla: `BlockEntityRenderDispatcher` + per-BE renderers. Continue by
    smallest sub-slice; audit the `Custom`→`Cube` shape fallback
    (`block_models/shape.rs` → `textures.rs`) alongside, since
    unclassifiable elements are mostly BE-driven models.
- Evidence / boundary:
  - Done 2026-07-08 — Ordinary spawner display entity renderer (tenth BER
    sub-slice). Vanilla facts were checked against `SpawnerRenderer`,
    `TrialSpawnerRenderer.extractSpawnerData`, `BaseSpawner`, `SpawnData`,
    and `SpawnerBlockEntity`: the display entity id comes from
    `SpawnData.entity.id`; `clientTick` advances `spawnDelay`, `oSpin`, and
    `spin = (spin + 1000/(spawnDelay+200)) % 360` only while a nearby player
    exists; BlockEvent(1) resets delay to `minSpawnDelay`; extraction uses
    `lerp(oSpin, spin, partial) * 10` and scale `0.53125/max(bbWidth,
    bbHeight)`. World now decodes spawner BE NBT (`Delay`,
    `MinSpawnDelay`, `RequiredPlayerRange`, and `SpawnData.entity.id`),
    owns flat per-position ticker state, handles BlockEvent(1), and projects
    display-entity source states. Native maps those sources to existing
    `EntityModelKind` values through the protocol entity-type constants and
    appends block-sentinel `EntityModelInstance`s. Renderer adds
    `SpawnerDisplayRenderState` and wraps colored entity root transforms with
    vanilla `T(0.5,0.4,0.5) * Ry(spin) * T(0,-0.2,0) * Rx(-30) * S(scale)`.
    Boundary: trial spawner display-state behavior is intentionally not part
    of this ordinary-spawner slice; custom SpawnData entity NBT variants still
    render through default entity metadata until broader synthetic entity NBT
    projection exists. Tests cover protocol resource-id lookup, world
    NBT/tick/source/event behavior, native instance projection, and renderer
    wrapper transform points.
  - Done 2026-07-08 — End portal/gateway block-entity renderer (ninth BER
    sub-slice). Vanilla facts were checked against
    `AbstractEndPortalRenderer`, `TheEndPortalRenderer`,
    `TheEndGatewayRenderer`, `TheEndPortalBlockEntity`,
    `TheEndGatewayBlockEntity`, `EndPortalBlock`, `EndGatewayBlock`,
    `RenderPipelines`, `RenderTypes`, `DyeColor`, and `BeaconRenderer`:
    both blocks submit only Y-axis faces; end portals apply
    `T(0,0.375,0) * S(1,0.375,1)`; gateways keep the unit cube and submit a
    beacon-style beam while spawning or cooling down. World now decodes
    gateway `Age` from BE NBT, owns flat gateway age/cooldown state, handles
    BlockEvent(1) cooldown, advances `beamAnimationTick` on running ticks,
    and projects source states with vanilla spawn/cooldown percent,
    `sin(percent*PI)` scale, height, magenta/purple colors, and
    `floorMod(gameTime,40)+partial` animation time. Native maps those
    sources to `EntityModelKind::EndPortalBlock` with optional
    `EndGatewayBeamRenderState` and joins the shared entity-model stream
    after held-item baking. Renderer adds `EndPortal`/`EndGateway` custom
    position-color render types for the cube faces, `EndGatewayBeam` scroll
    geometry using the vanilla `BeaconRenderer.renderPart` quad formula, and
    `textures/entity/end_portal/end_gateway_beam.png` in the entity atlas
    (`ENTITY_MODEL_TEXTURE_REFS` 688-count). Boundary: the portal/gateway
    cube shader itself is not yet the official 15/16-layer portal shader; the
    approximation is intentionally tracked in Next action. Tests cover world
    NBT/tick/source projection, native instance/beam projection, renderer
    cube transform/faces/beam geometry/sorted draw range, and runtime
    tick-before-extract ordering.
  - Done 2026-07-08 — Skull/head block-entity renderer (eighth BER
    sub-slice). Vanilla facts were checked against `SkullBlockRenderer`,
    `SkullBlockRenderState`, `SkullBlockEntity`, `AbstractSkullBlock`,
    `SkullBlock`, `WallSkullBlock`, and `BuiltInBlockModels`: ground skulls
    use `ROTATION_16` with `RotationSegment.convertToDegrees(segment)`,
    wall skulls use `FACING`, all variants apply the vanilla
    `WallAndGroundTransformations`, and only powered dragon/piglin heads tick
    `animationTickCount`. World now maps the seven vanilla skull/head
    families, owns flat `SkullBlockState`, advances powered dragon/piglin
    animation on running ticks, and projects source states with ground/wall
    attachment plus partial animation progress. Native maps those sources to
    `EntityModelKind::SkullBlock`, samples block+sky light, applies ground
    yaw `-RotationSegment` degrees or wall attachment, and joins the shared
    entity-model stream after held-item baking. Renderer reuses the existing
    custom-head `SkullModel`, `DragonHeadModel`, and `PiglinHeadModel`
    geometry, dispatches skeleton/wither/player/zombie/creeper/dragon/piglin
    textures with vanilla `entityCutoutZOffset`, submits no overlay, and keeps
    dragon/piglin animation progress in the skull model state. Boundary:
    player-head BE `profile` owner skins still need the broader dynamic
    profile/texture plumbing; profileless player heads currently render the
    vanilla default player skin fallback. Tests cover world family/state/tick
    projection, native static/wall/animated source projection, renderer model
    key/texture/root transform/mesh bucket behavior, and runtime
    tick-before-extract ordering.
  - Done 2026-07-08 — Conduit block-entity renderer (seventh BER
    sub-slice). Vanilla facts were checked against
    `ConduitBlockEntity.java`, `ConduitRenderer.java`, and
    `ConduitRenderState.java`: client tick increments `tickCount`, refreshes
    the water + prismarine/sea-lantern frame every `gameTime % 40 == 0`,
    sets active at 16 frame blocks and hunting at 42, and increments
    `activeRotation` while active; `getActiveRotation(partialTick)` is
    `(activeRotation + partialTick) * -0.0375`. World now owns a flat
    `ConduitBlockState` store plus source-state projection, including the
    3x3x3 water requirement and the 5x5x5 ring block count. Native advances
    this client BE ticker on `running_ticks`, projects inactive conduits as
    one shell instance, and active conduits as cage, outer wind, inner wind,
    and camera-facing eye instances with sampled block+sky light. Renderer
    adds `EntityModelKind::Conduit { part }`, the four vanilla model layers
    (`CONDUIT_EYE`, `WIND`, `SHELL`, `CAGE`), and the six textures
    `entity/conduit/{base,cage,wind,wind_vertical,open_eye,closed_eye}` into
    the shared entity atlas.
    Root transforms transcribe `ConduitRenderer.submit`: inactive shell
    centered with vanilla's `activeRotation * PI / 180` rotation quirk;
    active cage bob + `(0.5,1,0.5)` axis rotation; outer wind phase
    rotations; inner wind 0.875 scale + `rotationXYZ(π,0,π)`; eye bob +
    camera-facing orientation + 4/3 scale. Deferred boundary stays the
    cross-cutting BER break-progress crumbling and per-BE distance/frustum
    culling already noted for previous BER slices. Tests cover world shape
    refresh/activation/hunting/source projection, native inactive/active
    instance expansion + camera-facing eye fields, renderer cube/texture
    refs, layer pass metadata, root transform samples, mesh buckets, and
    runtime tick-before-extract ordering.
  - Done 2026-07-07 — Enchanting-table book + lectern book block-entity
    renderers (sixth BER sub-slice; both share vanilla `ModelLayers.BOOK`
    / `BookModel` + the single `entity/enchantment/enchanting_table_book`
    64×32 sprite). World: the enchanting table's hovering book is a per-
    block-entity animation (`EnchantingTableBlockEntity`: `time`, `flip`/
    `oFlip`/`flipT`/`flipA`, `open`/`oOpen`, `rot`/`oRot`/`tRot`) ticked
    every client tick by `bookAnimationTick` (`EnchantingTableBlockEntity
    .java:50-106`), transcribed in `bbb-world/src/enchanting_table_books
    .rs` as a flat `Vec<EnchantingTableBookState>` reconciled + advanced
    on running ticks in the runtime pump. The book turns to face the
    nearest player within 3 blocks
    (`level.getNearestPlayer(x+0.5,y+0.5,z+0.5,3.0,false)` →
    `EntitySelector.NO_SPECTATORS`, transcribed as the local player +
    remote player entities minus spectators, matching the particle
    nearest-player context), opens/closes 0.1/tick, and flips its pages
    toward a random `flipT` target. Vanilla's static wall-clock-seeded
    `RANDOM` becomes a single fixed-seed serializable `LegacyRandomSource`
    (`EnchantingBookRandom`) drawn in a deterministic per-position tick
    order — a faithful deterministic stand-in for the shared-static random
    (vanilla is itself non-deterministic here: wall-clock seed + block-
    entity-ticker order). The lectern book is pure block-state derivation
    (`bbb-world/src/lectern_books.rs`, no BE data): rendered only while
    `LecternBlock.HAS_BOOK` is set, yaw = `FACING.getClockWise().toYRot()`.
    Dispatch: `EntityModelKind::EnchantingBook` / `LecternBook` ride the
    single entity-model submission stream (`-1` sentinel, `block<<4 |
    sky<<20` light). `EnchantTableRenderer.extractRenderState`'s partial-
    tick lerp (`flip`/`open`/`time`, and the `(-π,π]`-folded
    `oRot + or·partialTick` yaw) runs in `enchanting_table_book_scene.rs`;
    the enchanting root transform transcribes `EnchantTableRenderer.submit`
    (`java:61-73`): `T(0.5,0.75,0.5) · T(0, 0.1 + sin(time·0.1)·0.01, 0) ·
    Ry(-yRot) · Rz(80°)`, the lectern transform `LecternRenderer.submit`
    (`java:46-50`): `T(0.5,1.0625,0.5) · Ry(-yRot) · Rz(67.5°) ·
    T(0,-0.125,0)` (no extra model scale — the mesh is 1/16-authored, baked
    into cube emission). Renderer (`model_layers/book.rs`) transcribes
    `BookModel.createBodyLayer` (`BookModel.java:35-53`, atlas 64×32: `left
    _lid` 6×10×0.005 texOffs(0,0) offset(0,0,-1); `right_lid` texOffs(16,0)
    offset(0,0,1); `seam` 2×10×0.005 texOffs(12,0) rotation(0,π/2,0); `left
    _pages` 5×8×1 texOffs(0,10) at -0.99z; `right_pages` texOffs(12,10) at
    -0.01z; `flip_page1`/`flip_page2` 5×8×0.005 texOffs(24,10)) and
    `BookModel.setupAnim` (`java:55-68`): `leftLid.yRot = π + openness`,
    `rightLid.yRot = -openness`, pages `±openness`, `flipPageN.yRot =
    openness − openness·2·pageFlipN`, all pages `x = sin(openness)`, over
    the derived `BookModel.State.forAnimation` openness `(sin(progress·
    0.02)·0.1 + 1.25)·open` (renderer-side, in `setup_anim`). The two page-
    flip fractions `clamp(frac(flip + {0.25,0.75})·1.6 − 0.3, 0, 1)` are
    `EnchantTableRenderer.submit` submit logic (native side); the lectern
    binds the fixed `BookModel.State.forAnimation(0, 0.1, 0.9, 1.2)`
    (openness 1.5). One new 64×32 `entity/enchantment/enchanting_table_book`
    sprite joins the shared entity atlas and `entity_assets.rs`. Deferred
    (honest): BER
    `breakProgress` crumbling and per-BE distance/frustum culling (same
    boundary as the previous five slices); the enchanting-table book's
    exact page-flip pattern differs from any given vanilla session (both
    use a random with no deterministic contract — bbb's is at least
    reproducible), and batching >1 running tick reuses the current player
    positions for all steps (indistinguishable at 0/1 ticks per frame).
    Tests: `bbb-world/src/enchanting_table_books.rs` (nearest-player 3-block
    range incl. the `< range²` boundary, open/rot chase-then-relax, page-
    flip re-roll + flip easing, new/pruned table tracking, source-state
    enumeration, random determinism), `bbb-world/src/lectern_books.rs`
    (has-book gate, facing→clockwise-yaw table, book-removal prune),
    `entity_models/tests/book.rs` (cubes/pivots vs `BookModel` incl. the
    static seam, `State.forAnimation` openness hand-calcs incl. the sin
    peak + lectern 1.5, `setupAnim` cover/page/flip hand-calcs +
    `prepare()` wiring, enchanting hover+tip and lectern transform point-
    mapping, model-key/texture-ref selection, single `entitySolid` layer
    pass, 7-box 42-face cutout-cull mesh bake),
    `bbb-native/src/enchanting_table_book_scene.rs` (closed-book default,
    partial-tick lerp/yaw extraction, fixed closed-book flip fractions,
    lerp/frac/wrap math, light packing),
    `bbb-native/src/lectern_book_scene.rs` (has-book gate + fixed state +
    facing yaw, light packing), plus the runtime pump tick-before-extract
    ordering assertions.
  - Done 2026-07-06 — Banner block-entity renderer (fifth BER sub-slice;
    all 32 blocks: 16 `minecraft:<color>_banner` (ground, `ROTATION_16`)
    + 16 `<color>_wall_banner` (`FACING`)). World: the BE NBT `patterns`
    list (`BannerPatternLayers.CODEC` — `{pattern: registry id, color:
    dye name}` compounds) decodes into
    `BlockEntityRecord.banner_patterns` (chunk-batch + single
    `BlockEntityData`, pruned on block change); one malformed entry
    folds the whole list away, matching
    `BannerBlockEntity.loadAdditional`'s `.orElse(EMPTY)` codec fold;
    the base color is a block-id fact (`AbstractBannerBlock.getColor`).
    The flag swing phase transcribes
    `BannerRenderer.extractRenderState`: `(floorMod(x*7 + y*9 + z*13 +
    gameTime, 100L) + partialTicks) / 100` with the deterministic
    `WorldTimeState.game_time` standing in for `Level.getGameTime()`
    (wrapping i32 position hash, `rem_euclid` floor-mod). Dispatch:
    `EntityModelKind::Banner { wall, base_color, layers:
    [Option<BannerPatternLayer>; 16] }` rides the single entity-model
    submission stream (`-1` sentinel, `block << 4 | sky << 20` light);
    the root transform transcribes `BannerRenderer.modelTransformation`:
    `T(0.5, 0, 0.5) · Ry(−angle) · S(⅔, −⅔, −⅔)` — ground angle
    `RotationSegment.convertToDegrees(ROTATION)` (22.5° segments folded
    into (−180, 180]), wall `FACING.toYRot()`. Renderer
    (`model_layers/banner.rs`) transcribes `BannerModel.createBodyLayer`
    (atlas 64×64: standing-only `pole` 2×42×2 texOffs(44,0) at
    (−1,−42,−1); `bar` 20×2×2 texOffs(0,42) at (−10,−44,−1) standing /
    (−10,−20.5,9.5) wall) and `BannerFlagModel.createFlagLayer` (`flag`
    20×40×1 texOffs(0,0) at (−10,0,−2), pivot offset(0,−44,0) standing /
    (0,−20.5,10.5) wall); `setupAnim`: `flag.xRot = (−0.0125 +
    0.01·cos(2π·phase))·π`. Pattern composition transcribes
    `submitBanner`/`submitPatterns` (`BannerRenderer.java:171-208`): the
    frame + flag submit untinted `entitySolid` over
    `entity/banner/banner_base`, then the same flag geometry re-submits
    per layer — `entity/banner/base` tinted by the base color first,
    then each layer's `entity/banner/<pattern>` tinted by
    `DyeColor.getTextureDiffuseColor()` (the tint rides the existing
    per-pass vertex tint, the tropical-fish mechanism), clamped at the
    `MAX_PATTERNS = 16` render cap; the pattern passes ride the
    translucent bucket standing in for `RenderPipelines.BANNER_PATTERN`
    (`RenderPipelines.java:282`: ENTITY_SNIPPET + NO_OVERLAY +
    TRANSLUCENT blend + LESS_THAN_OR_EQUAL, depth write off). The
    pattern registry is the transcribed 43-arm table
    (`BannerPatterns.bootstrap`, `BannerPatterns.java:60-105`; every
    vanilla `asset_id` equals its registry id; an unknown pattern id or
    dye name folds the stack empty like the registry-holder codec
    failure — a datapack-registered pattern bbb has no texture for lands
    in that fold). 44 new 64×64 `entity/banner/*` sprites (banner_base +
    base + 42 patterns — asset tree count verified: 44 files) join the
    shared entity atlas and `entity_assets.rs`. Deferred (honest): BER
    `breakProgress` crumbling and per-BE distance/frustum culling (same
    boundary as the previous four slices); vanilla's `bannerPattern`
    pipeline disables depth writes — bbb's shared translucent pipeline
    keeps them on (equal-depth `LessEqual` layering is unaffected;
    revisit only if a dedicated no-depth-write pass ever matters); the
    banner *item* / shield pattern path (`Sheets.SHIELD_PATTERN_BASE`,
    `BannerRenderer.submitSpecial`) is item-model scope, not this slice.
    Tests: `bbb-world/src/banner_blocks.rs` (32-block color/form table,
    patterns NBT layer order, malformed-entry fold, phase hand-calcs
    incl. negative floor-mod + gameTime step, rotation-segment/facing
    angles, prune on block change), `entity_models/tests/banner.rs`
    (cubes/pivots vs `BannerModel`/`BannerFlagModel` incl. the pole-less
    wall tree, swing xRot hand-calcs at phase 0/¼/½/1 + `prepare()`
    wiring, transform point-mapping incl. the pole-top → y 28 landing
    and the −90° yaw, the 5-pass layer stack (kinds/render types/layer
    ids/retained parts/tints/sequences) + the wall 3-pass variant,
    44-sprite selection + shared-atlas membership, mesh bake: 18-face
    cutout-cull frame+flag + 12-face translucent pattern re-bake with
    per-pass tints), `bbb-native/src/banner_scene.rs` (kind/base-color/
    yaw/phase/light projection, 43-pattern id table round-trip, unknown
    pattern/dye fold, the 16-layer render cap).
  - Done 2026-07-06 — Shulker box + decorated pot block-entity renderers
    (fourth BER sub-slice). Shulker box (all 17 blocks: `minecraft:
    shulker_box` + 16 `<color>_shulker_box` × 6-way `facing`): lid state
    machine transcribed in `bbb-world/src/shulker_box_blocks.rs` —
    `BlockEvent(1, count)` → `ShulkerBoxBlockEntity.triggerEvent`
    (`java:140-155`: count 1 → OPENING, count 0 → CLOSING, other counts
    only update the stored count) and `updateAnimation` (`java:66-101`:
    `progressOld = progress` then ±0.1/tick, latch at the 0/1 clamps,
    CLOSED prunes) advanced on running ticks in the runtime pump;
    `getProgress = lerp(partialTicks, progressOld, progress)`. Dispatch:
    `EntityModelKind::ShulkerBox { color, facing }` rides the single
    entity-model submission stream (`-1` sentinel, `block << 4 | sky <<
    20` light); the root transform transcribes
    `ShulkerBoxRenderer.createModelTransform` (`java:111-121`):
    `T(0.5,0.5,0.5) · S(0.9995) · R(FACING.getRotation()) · S(1,-1,-1) ·
    T(0,-1,0)` with the `Direction.getRotation()` table
    (`Direction.java:144-153`). Renderer (`model_layers/shulker_box.rs`):
    the box model is vanilla `ShulkerModel.createBoxLayer` = the mob's
    shell mesh minus the head (`lid` 16×12×16 texOffs(0,0), `base`
    16×8×16 texOffs(0,28), pivot offset(0,24,0), atlas 64×64 — the cube
    consts are shared with the shulker mob, and the 17
    `entity/shulker/shulker[_<color>].png` sprites were already
    registered, so the box adds zero texture refs); `setupAnim`
    (`ShulkerBoxRenderer.java:141-145`): `lid.setPos(0, 24 −
    progress·0.5·16, 0)`, `lid.yRot = 270°·progress`; render type
    `entityCutout` (`java:137`; the mob uses `entityCutoutZOffset`).
    Decorated pot (`minecraft:decorated_pot` × HORIZONTAL_FACING): the BE
    NBT `sherds` item-id list (`PotDecorations.java:23-52`; ≤4 entries in
    [back, left, right, front] order, `minecraft:brick`/missing = empty
    face) decodes into `BlockEntityRecord.decorated_pot_sherds`
    (chunk-batch + single `BlockEntityData`, pruned on block change);
    the sherd item → pattern mapping is the transcribed 23-arm table in
    `bbb-native/src/decorated_pot_scene.rs` citing
    `DecoratedPotPatterns.java:37-62/72-97` (every
    `minecraft:<name>_pottery_sherd` → `<name>_pottery_pattern`; unknown
    items → the plain `decorated_pot_side`, matching the vanilla
    null-pattern fallback). Dispatch: `EntityModelKind::DecoratedPot {
    back, left, right, front }`; root transform transcribes
    `DecoratedPotRenderer` (`java:175-177`): `rotateAround(Ry(180 −
    facing.toYRot()), 0.5, 0.5, 0.5)`. Wobble done bell-style
    (`bbb-world/src/decorated_pot_blocks.rs`): `BlockEvent(1,
    style.ordinal())` (`DecoratedPotBlockEntity.java:167-175`, `data < 2`
    gate) starts a tick counter standing in for vanilla's `gameTime −
    wobbleStartedAtTick` clock (POSITIVE 7 ticks / NEGATIVE 10,
    `WobbleStyle`), re-trigger restarts, expiry/destroy prunes; the
    render-side transform transcribes `java:150-169`: gate `0 ≤ progress
    ≤ 1`, POSITIVE `Rx(−1.5·(cos dt + 0.5)·sin(dt/2)·0.015625)` then
    `Rz(sin dt·0.015625)` with `dt = progress·2π` about (0.5, 0, 0.5),
    NEGATIVE `Ry(sin(−progress·3π)·0.125·(1 − progress))`. Renderer
    (`model_layers/decorated_pot.rs`) transcribes `createBaseLayer`
    (atlas 32×32, `java:83-101`): `neck` texOffs(0,0) box (4,17,4)+(8,3,8)
    deflate(−0.1) + texOffs(0,5) box (5,20,5)+(6,1,6) inflate(0.2) at
    offsetAndRotation(0,37,16,π,0,0) (CubeDeformation g: min−g, size+2g,
    UV keeps undeformed dims), `top`/`bottom` texOffs(−14,13) 14×0×14
    planes at (1,16,1)/(1,0,1); and `createSidesLayer` (atlas 16×16,
    `java:103-112`): one 14×16×0 plane texOffs(1,0) baked NORTH-face-only
    (`ModelCube::with_visible_faces`), posed back(15,16,1, 0,0,π)/
    left(1,16,1, 0,−π/2,π)/right(15,16,15, 0,π/2,π)/front(1,16,15,
    π,0,0). One 7-part model tree renders in 5 `entitySolid` passes via
    `RetainedParts` visibility (base sheet for neck/top/bottom + one pass
    per side with its pattern sprite); 25 new 16×16/32×32
    `entity/decorated_pot/*` sprites (base, side, 23 patterns — asset
    tree count verified) join the shared entity atlas and
    `entity_assets.rs`. Deferred (honest): BER `breakProgress` crumbling
    and per-BE distance/frustum culling (same boundary as chest/sign/
    bed/bell); the shulker box's `AABB`-based open/close *sound* +
    interaction blocking are gameplay-side; vanilla 26.1's decompiled
    `DecoratedPotRenderer.extractRenderState` never assigns
    `state.wobbleStyle` (decompiler artifact) — bbb carries the style
    through deliberately. Tests: `bbb-world/src/shulker_box_blocks.rs`
    (17-color block-id table, event gate/open-close/latch/saturation
    count, 0.1-step + lerp hand-calcs, destroyed prune, projection),
    `bbb-world/src/decorated_pot_blocks.rs` (wobble event gate + style
    table + restart + expiry prune + progress projection, facing table),
    `bbb-world/src/chunks/pot_decorations.rs` (sherds NBT order,
    brick/missing → empty face, prune on block change),
    `entity_models/tests/shulker_box.rs` (cubes vs `ShulkerModel`, lid
    pose hand-computed at progress 0/0.5/1 → (24,0°)/(20,135°)/(16,270°),
    six-way facing transform point-mapped incl. the 0.9995 shrink,
    17-sprite selection, `entityCutout` pass, 12-face cutout bake),
    `entity_models/tests/decorated_pot.rs` (deformed cubes + poses vs
    `DecoratedPotRenderer`, NORTH-only side faces, facing point-mapping,
    wobble POSITIVE/NEGATIVE hand-calcs + >1 gate, 25-sprite table,
    5-pass layers with per-side pattern/fallback selection, 28-face
    cutout-cull bake), `bbb-native/src/shulker_box_scene.rs` /
    `decorated_pot_scene.rs` (kind/color/facing/y-rot/light packing,
    sherd→pattern table round-trip incl. brick/unknown fallback, wobble
    style+progress projection) + the runtime pump ordering assertions.
  - Done 2026-07-06 — Bed + bell block-entity renderers (third BER
    sub-slice). Bed (all 16 `minecraft:<color>_bed` blocks × HEAD/FOOT ×
    facing): positions/color/part/facing derive per frame from block states
    (`bbb-world/src/bed_blocks.rs`, palette pre-check per section; the color
    is a block-id fact — `BedBlockEntity.getColor` never reads NBT on the
    render path), with the `DoubleBlockCombiner` partner
    (`BedBlock.getNeighbourDirection`: FOOT → facing, HEAD → opposite;
    pairing needs same block + other `part` + same `facing`,
    `DoubleBlockCombiner.java:42-46`) feeding the `BrightnessCombiner`
    per-component light max. Dispatch: `EntityModelKind::Bed { color, part }`
    rides the single entity-model submission stream (`-1` sentinel,
    `block << 4 | sky << 20` light); the root transform transcribes
    `BedRenderer.createModelTransform` (`BedRenderer.java:157-164`):
    `translation(0, 0.5625, 0) · Rx(90°) · rotateAround(Rz(180 +
    facing.toYRot()), 0.5, 0.5, 0.5)`, no entity flip. Renderer
    (`model_layers/bed.rs`) transcribes `createHeadLayer`/`createFootLayer`
    (atlas 64×64): `main` 16×16×6 texOffs(0,0)/(0,22), legs 3×3×3
    texOffs(50,6)/(50,18)/(50,0)/(50,12) at `PartPose.rotation(π/2, 0,
    {π/2, π, 0, 3π/2})`; the vanilla `visibleFaces` sets (head main hides
    UP, foot main hides DOWN — the two seam faces — legs hide DOWN, which
    is coplanar with the visible mattress underside and would z-fight) are
    now honoured exactly: the shared cube emitter gained a vanilla-shaped
    per-face visibility mask (`ModelCube::with_visible_faces`,
    `MODEL_CUBE_FACE_*` bits in `Direction.get3DDataValue` order; existing
    models are untouched — `addBox` default stays all-visible). 16
    `entity/bed/<DyeColor.getName()>.png` 64×64 sprites feed the shared
    entity atlas; passes use vanilla `entitySolid` (cull bucket). Bell
    (`minecraft:bell`, all 4 attachments): `BellRenderer.submit` applies no
    transform — the body renders identically for every attachment; the
    bar/post support frame is block-model geometry
    (`bell_floor/wall/ceiling/between_walls.json` carry the `#bar`/`#post`
    elements) the terrain path already draws, so the BER contributes only
    the body. Shake chain transcribed in `bbb-world/src/bell_blocks.rs`:
    `BlockEvent(1, direction)` → `BellBlockEntity.triggerEvent`
    (`clickDirection = Direction.from3DDataValue(b1)` — DOWN/UP
    wire-representable but swing nothing — `ticks = 0`, `shaking = true`,
    re-ring resets), `clientTick` `if (shaking) ticks++; if (ticks >= 50)
    { shaking = false; ticks = 0; }` (DURATION 50) advanced on running
    ticks in the runtime pump; destroyed bells and finished shakes prune.
    Renderer (`model_layers/bell.rs`) transcribes `BellModel.createBodyLayer`
    (atlas 32×32): `bell_body` 6×7×6 texOffs(0,0) box (-3,-6,-3) pivot
    offset(8,12,8) with child `bell_base` 8×2×8 texOffs(0,13) box (4,4,4)
    offset(-8,-12,-8); `setupAnim` swing `Mth.sin(ticks/π) / (4 + ticks/3)`
    with `ticks = blockEntity.ticks + partialTicks`, axis by click
    direction (NORTH `xRot=-r` / SOUTH `+r` / EAST `zRot=-r` / WEST `+r`);
    `entity/bell/bell_body.png` 32×32; passes use vanilla `entitySolid`
    (`BellModel`'s constructor). Deferred (honest): BER `breakProgress`
    crumbling; per-block-entity distance/frustum culling (both submit like
    entities, unculled — same boundary as chest/sign); the bell resonation
    particle/glow chain (`resonationTicks`, raider search) is
    gameplay-side, not render, and stays with the raid features. Tests:
    `bbb-world/src/bed_blocks.rs` (16-color block-id table, enumeration +
    pairing incl. wrong-color/same-part/facing-mismatch breaks, removal),
    `bbb-world/src/bell_blocks.rs` (event gate + trigger/re-ring + 50-tick
    end sequence + destroyed prune + login clear + `from3DDataValue`
    table + partial-tick projection), `entity_models/tests/bed.rs` (all 6
    cubes + visibleFaces masks + 4 leg poses vs `BedRenderer.java`,
    S/N/W/E facing transform point-mapped, 16-sprite table in DyeColor id
    order, `entitySolid` pass, 15-face cutout-cull bake proving the hidden
    faces), `entity_models/tests/bell.rs` (cubes/pivots vs
    `BellModel.java`, swing angle hand-computed at ticks 0/10/25 on all 4
    axes + DOWN/UP/None still, identity root transform, 12-face bake),
    `bbb-native/src/bed_scene.rs` / `bell_scene.rs` (kind/angle/light
    packing, double-half light max path, shake-direction projection) + the
    runtime pump ordering assertions.
  - Done 2026-07-06 — Sign + hanging sign block-entity renderer with face
    text (second BER sub-slice; all 12 woods incl. pale_oak × standing /
    wall / hanging-ceiling (± the `attached=true` vChains CEILING_MIDDLE
    variant) / hanging-wall). Data chain: sign BE NBT decodes in
    bbb-protocol (`decode_sign_block_entity_nbt`) so the
    `front_text`/`back_text` message components reuse the single
    `append_component_runs` styled-run traversal (`SignText.DIRECT_CODEC`
    shape: 4 messages, dye-name `color` defaulting black,
    `has_glowing_text`, root `is_waxed`);
    `bbb-world/src/chunks/sign_text.rs` maps it to
    `SignBlockEntityTextState` (missing side → vanilla
    `orElseGet(SignText::new)` default); ChunkData BE sections and
    `BlockEntityData` packets share the ingest, and `set_block_state_id`
    prunes the stored text when the block name changes. Positions derive
    per frame from block states (`bbb-world/src/sign_blocks.rs`: palette
    pre-check per section, `rotation16 = segment * 22.5°`, wall-family
    `facing.toYRot`, text sides gated on non-empty lines). Dispatch:
    `EntityModelKind::Sign { wood, attachment }` rides the single
    entity-model submission stream (`-1` sentinel, block-position light
    `block << 4 | sky << 20`); the root transform transcribes
    `StandingSignRenderer.bodyTransformation`
    (`translate(0.5,0.5,0.5)·Ry(-angle)·scale(2/3,-2/3,-2/3)`,
    RENDER_SCALE 0.6666667, wall extra `translate(0,-0.3125,-0.4375)`)
    and `HangingSignRenderer.bodyTransformation`
    (`translation(0.5,0.9375,0.5)·Ry(-angle)·translate(0,-0.3125,0)·
    scale(1,-1,-1)`). Renderer: `model_layers/sign.rs` transcribes
    `createSignLayer` / `createHangingSignLayer` (board 24×12×2
    texOffs(0,0) + stick 2×14×2 texOffs(0,14); hanging board 14×10×2
    texOffs(0,12), plank 16×2×4, chain planes texOffs(0,6)/(6,6) at
    offset(±5,-6,0) yRot ∓π/4, vChains 12×6 texOffs(14,6)); 24
    `entity/signs[/hanging]/<wood>.png` 64×32 sprites feed the shared
    entity atlas; passes use vanilla `entityCutout` (no cull). Face text
    bakes to world-space glyph quads
    (`item_models/sign_text.rs::bake_sign_text_surface`) drawn with the
    map-label `minecraft:font/default` atlas in the entity-translucent
    feature pass; the text transform is body · [back: Ry(π)] ·
    `TEXT_OFFSET (0, 0.33333334, 0.046666667)` (hanging
    `(0, -0.32, 0.073)`) · scale 0.010416667 / 0.0140625 with negated y;
    layout matches `SignRenderer` semantics: line height 10/9, max line
    width 90/60 (word-wrap at the last space, else hard break before the
    overflowing glyph), centering `x = -width/2` (Java int division),
    `y = i*lh - 4*lh/2`; colors via `getDarkColor`
    (`ARGB.scaleRGB(color, 0.4F)` truncating per channel; black + glowing
    → the -988212 / 0xF0EBCC cream), glowing faces render the raw dye
    `getTextColor` full-bright (15728880), and per-run component colors
    override the face base color. Deferred (honest): the glowing 8-way
    outline glyph pass (glowing text renders full-bright without
    outlines); underline/strikethrough effect bars (need a white pixel
    outside the single font-atlas draw); obfuscated glyph cycling (draws
    the literal glyphs); `is_waxed` stored without render effect (vanilla
    has none either — it gates editing); `filtered_messages` not decoded;
    general BE-record removal on block change (only `sign_text` prunes);
    per-BE distance culling; vanilla's POLYGON_OFFSET font display mode
    (approximated by the fixed TEXT_OFFSET z gap). Tests: protocol NBT
    decode (4 lines / color / glowing / waxed / missing side), world
    BE-section + packet updates + block-change prune, `sign_blocks`
    enumeration / rotation / facing / attached,
    `entity_models/tests/sign.rs` (all 7 cubes + chain poses vs vanilla,
    root-transform point mapping for standing/wall/hanging, model keys,
    the 24-sprite table, pass render type, cutout mesh bake),
    `item_models/sign_text.rs` (text-transform offsets/scale incl. the
    back face, dark-color formula, 90/60 truncation + word wrap + bold
    7px advances, hand-computed centering and line ys, run-color override
    + bold double draw, empty-face None),
    `bbb-native/src/sign_scene.rs` (kind/rotation/light packing, per-face
    gating, glowing full-bright).
  - Done 2026-07-06 — Chest block-entity renderer (first BER sub-slice, whole
    chest family: chest / trapped / ender / the 8 copper-chest blocks with
    waxed variants sharing their weathering stage's texture). Data chain:
    `bbb-world/src/chest_lids.rs` holds a flat `ChestLidState` tracker
    (vanilla `ChestLidController` transcribed — `BlockEvent(1, count)` →
    `shouldBeOpen(count > 0)` via the `Level.blockEvent` → `BaseEntityBlock` →
    `ChestBlockEntity.triggerEvent` dispatch on the *current* block state;
    `tickLid` 0.1/tick with `oOpenness` trailing, advanced on running ticks in
    the runtime pump; destroyed/unloaded chests and resting-closed lids
    prune). Chest positions are derived from chunk block states per frame
    (`chest_model_source_states`): sections whose block palette holds no
    chest state are skipped, so the scan cost tracks chest-bearing sections.
    Double chests pair per `ChestBlock.getConnectedDirection` (LEFT →
    `facing.getClockWise()`, same-block opposite-`type` check) and share
    `opennessCombiner` max of the two lerped opennesses. Dispatch: chest
    instances ride the existing single entity-model submission stream
    (`EntityModelKind::Chest { texture, half }` in
    `RendererFrame.entity_model_instances` — no parallel textured path;
    `entity_id` is a `-1` sentinel), with a BER-style root transform
    (`rotationAround(-facing.toYRot(), 0.5, 0, 0.5)`, no entity
    `scale(-1,-1,1)` flip) and block-position light packed
    `block << 4 | sky << 20` with the double-chest `BrightnessCombiner`
    per-component max. Renderer: `model_layers/chest.rs` transcribes
    `ChestModel` (single + double left/right `bottom`/`lid`/`lock`, lid+lock
    pivot `offset(0,9,1)`), `setup_anim` applies `1-(1-o)^3` easing and
    `xRot = -(o·π/2)`; 19 `entity/chest/*.png` 64×64 sprites feed the shared
    entity atlas; passes use vanilla `entityCutoutCull` (cull-enabled cutout
    bucket). Deferred (honest): the christmas Dec 24-26 texture swap (no
    wall-clock input); vanilla's seam-face `Util.allOfEnumExcept` visibility
    (bbb emits the seam quads — they sit enclosed inside the joined
    double-chest volume, with only an invisible internal coplanar pair);
    BER `breakProgress` crumbling on the chest model; per-block-entity
    distance/frustum culling (chests submit like entities, unculled).
    Tests: `bbb-world/src/chest_lids.rs` (event gate + tick sequence +
    clamp/prune + login clear + enumeration/pairing/openness-combine),
    `bbb-renderer/src/entity_models/tests/chest.rs` (all 9 cubes + pivots vs
    `ChestModel.java`, easing/rotation hand-computed, facing rotation matrix
    point-mapped, sprite selection incl. ender/copper, cutout-cull mesh
    bake), `bbb-native/src/chest_scene.rs` (projection facing/openness/light,
    double-half max, light packing) + the runtime pump ordering test.
  - Done 2026-07-06 — Per-face occlusion-shape culling (slab/stairs full
    faces): neighbour face culling was a cell-level boolean (any opaque
    neighbour with geometry culled every adjacent face), so slabs/stairs hid
    the full faces of adjacent blocks. It now follows vanilla
    `Block.shouldRenderFace` (`Block.java:304`) per-direction occlusion shapes.
    New pure fn `face_occludes(shape, direction)` (`terrain/mesh.rs`) derives a
    full-1×1 occlusion face from the render cuboids: `Cube` fills all six sides;
    `Cross`/`Crosses`/`Quads` never occlude (empty vanilla occlusion shape); a
    `Box` face is full when a single cuboid touches the boundary and spans the
    16×16 cross-section; `Boxes` rasterises every boundary-touching cuboid's
    cross-section onto a 16×16 grid and requires full cover (a straight stair's
    back face is full only via the union of its two boxes). This is exactly
    vanilla `Block.isFaceFull(getFaceShape)` for box unions and a strict subset
    of vanilla occlusion, so it never culls a face vanilla keeps. In bbb's
    full-face-only model, `shouldRenderFace`'s two-part join collapses to a
    one-way "neighbour presents a full opaque occlusion face" test (the own-face
    and partial-join branches only ever render *more*). `culls_face_between_cells`
    gained a `direction` arg; the four call sites (cube face loop, `emit_box`,
    `box_face_will_render`, `emit_quads`) pass their cull direction and test the
    neighbour's opposite-face occlusion. The material gate is unchanged
    (`Opaque` ≈ vanilla `canOcclude`), the fluid branch is unchanged, and
    AO/light sampling still uses the cell-level `is_occluded_by_cell` (a
    separate concern from face culling). Two identical adjacent partial faces
    (e.g. same-orientation slab sides) conservatively over-render rather than
    cull the matching halves — safe (back-to-back faces, no visual hole).
    Tests (`terrain/mesh/tests.rs`): `face_occludes` direct predicate (cube /
    top+bottom slab / stair union back / cross / quads / empty boxes), stacked
    top+bottom slab filled-boundary cull, same-orientation double slab keeps
    touching sides, stairs union back culls a neighbour cube, cutout (glass-like)
    slab does not occlude, cross neighbour does not occlude, cross-chunk partial
    slab keeps the cube's face; the pre-existing
    `box_model_culls_only_faces_marked_by_cullface` assertion was corrected
    (slab half-side no longer hides the neighbour cubes: culled 4→2, opaque
    14→16). `skipRendering` same-block glass/bars culling is recorded above as a
    separate deferred slice.
  - Done 2026-07-06 — Breaking crack decal follows render shape: the crumbling
    overlay now cracks over the block's real geometry instead of a constant unit
    cube. `BlockDestroyOverlay` carries the position's `TerrainRenderShape`,
    projected in `runtime.rs::block_destroy_render_shape` from the same
    `TerrainTextureState::block_render_shape` (a thin wrapper over the chunk
    mesher's `block_render_data`, so the model-variant seed matches the drawn
    chunk); no chunk loaded → full-cube fallback. `block_destroy.rs` emits faces
    by reusing the mesher's own `box_face_corners` / `FACES` / `CROSS_FACES`
    (promoted to `pub(crate)`) with the `[0,1,2,0,2,3]` winding — the same
    inward-RHR winding the terrain block faces use (fluid back-face note in
    `terrain/mesh/emitter.rs` is the ground truth for outside-visible winding),
    so the decal shows on exactly the sides the block faces do. This also
    corrects the prior overlay, which used the opposite (outward-RHR) winding.
    Covered: Cube, Box, Boxes (slabs/stairs/fences/walls), Cross/Crosses (two
    diagonal planes). UV follows vanilla `SheetedDecalTextureGenerator`
    (`BlockFeatureRenderer` feeds the block model's own quads through the
    crumbling buffer at `textureScale = 1.0`): the block-local vertex position is
    projected onto the axes perpendicular to the face's nearest `Direction`
    (`[px,1-pz]` down, `[px,pz]` up, `[px,1-py]` south, …), so partial boxes
    sample only the covered slice of the sprite (a bottom slab's sides show the
    lower half). Degradation (honest): `Quads` shapes degrade to the unit-cube
    crack (no crumbling-friendly box decomposition); cross uses a full-plane
    decal (the mesher's fixed `[0,1]` cross planes always span the whole sprite).
    z-fight defense is unchanged (per-vertex outward nudge + crumbling pipeline
    depth bias). Tests: `block_destroy.rs` (slab half-height side faces + partial
    sprite slice, multi-box stairs face count, cross two-plane emission,
    hand-computed decal UV → mesh vertex, cube non-regression, `Quads`→cube
    degradation, terrain-matching winding) + native
    `block_destroy_overlays_merge_local_stage...` asserts the shape field.
  - Done 2026-07-05 — Biome color blend radius: terrain grass/foliage/
    dry-foliage/water tints now average the biome `ColorResolver` over the
    `biomeBlendRadius` window (hard-coded to vanilla `Options.java` default 2 →
    5×5), matching `ClientLevel.calculateBlockTint` (x/z plane at fixed y,
    per-channel integer arithmetic mean; the swamp grass modifier is applied
    per sample inside the resolver, before averaging — verified against
    `BiomeColors`/`Biome.getGrassColor`). Cross-chunk correctness: a per-convert
    `WorldStore::chunk_biome_sampler` pre-resolves the 3×3 neighbour columns so
    edge columns pull real neighbour-chunk biomes; columns whose chunk is not
    loaded are dropped from the mean (honest window truncation at the render-
    distance edge, divided by the available count) rather than fabricated.
    Window build is limited to biome-resolver blocks (grass/foliage/water),
    skipping the stone/dirt/air interior. Spruce/birch leaves keep their vanilla
    constant (not resolver-driven, so not blended); block-break particles still
    sample the single centre biome (blend deferred there). Tests:
    `terrain_runtime/textures/tests.rs` (uniform=no-op, two-biome exact mean,
    swamp per-sample-before-average, unloaded-column truncation) +
    `bbb-world` `chunk_biome_sampler_reads_neighbourhood_and_truncates_...`.
  - Verified aligned on 2026-07-05 (no code gap): vanilla four-corner AO +
    smooth lighting (`terrain/mesh/emitter.rs` per
    `ModelBlockRenderer.AmbientOcclusionCalculator`), face culling between
    cells, model-JSON render shapes via `bbb-pack` blockstate/model baking
    (multipart, weighted variants, rotations, uvlock), fluid side overlays
    and inverted backfaces, selection outline (vanilla view-offset layering,
    alpha 102, LINES depth semantics, non-cube shapes from `block_outline/`),
    translucent sorting both within sections (camera resort with index
    re-upload) and across sections (back-to-front order), and atlas state:
    terrain = 4 mip levels + Nearest/Nearest/Linear-mip samplers
    (`TextureAtlas` blur=false mipmap=true), entity atlas = 1 level all-
    Nearest (vanilla entity textures are not mipmapped), lightmap =
    Linear/Linear (`LightTexture`). This asymmetry is vanilla-correct.
- Detailed audit anchors live in the 2026-07-05 goal-archive P2 entry.

### HUD Overlay And Screen Render Surfaces

- Owner: `bbb-renderer` + `bbb-native` + `bbb-world`
- Status: `partial`
- Next action (2026-07-05 entry audit; consume in this order):
  - Continue the recipe-book overlay after the completed shell and toggle
    buttons/search input: tabs, page recipe buttons, recipe placement, and
    ghost recipe slots.
  - Then implement the advancement screen (`ClientAdvancementsState` ready) and
    debug overlay (F3; large, low priority).
- Evidence / boundary:
  - Done 2026-07-08 — Recipe-book overlay shell for the vanilla
    recipe-capable inventory screens. Vanilla anchors:
    `AbstractRecipeBookScreen.init` uses `width < 379` as the narrow-screen
    gate, initializes `RecipeBookComponent`, then gets the shifted main GUI
    `leftPos` from `RecipeBookComponent.updateScreenPosition`; in the
    non-narrow visible case with a 176px main GUI, that seats the main GUI
    149px to the right of the recipe-book origin. `RecipeBookComponent`
    defines the 147x166 panel, `xOffset = 86`, and blits
    `textures/gui/recipe_book.png` at UV `(1,1)` with size 147x166.
    bbb now treats `RecipeBookSettings.open` as the visibility source for
    local inventory, crafting table, furnace, blast furnace, and smoker
    screens; `inventory_screen_layout` expands to the recipe-book origin width
    and offsets slots by 149px, so hover/tooltips align with the shifted main
    GUI. Runtime HUD projection prepends a `RecipeBook` background layer,
    shifts the main GUI backgrounds, labels, previews, and non-cursor floating
    items by the same offset, and leaves the cursor item in composite screen
    coordinates. HUD assets load the vanilla `textures/gui/recipe_book.png`.
  - Done 2026-07-08 — Recipe-book toggle button. Vanilla anchors:
    `RecipeBookComponent.RECIPE_BUTTON_SPRITES` uses `recipe_book/button` and
    `recipe_book/button_highlighted`; `InventoryScreen` places the button at
    `(leftPos + 104, topPos + 61)`, `CraftingScreen` at
    `(leftPos + 5, topPos + 34)`, and `AbstractFurnaceScreen` at
    `(leftPos + 20, topPos + 34)`. bbb now projects those sprites for local
    inventory, crafting table, furnace, blast furnace, and smoker screens,
    highlights them from the composite inventory cursor position, locally
    toggles the matching `RecipeBookSettings` type while preserving
    `filtering`, and queues `RecipeBookChangeSettingsCommand` with the same
    values sent by vanilla `ServerboundRecipeBookChangeSettingsPacket`.
  - Done 2026-07-08 — Recipe-book filter toggle. Vanilla anchors:
    `RecipeBookComponent.initVisuals` creates the filter `CycleButton` at
    `(xo + 110, yo + 12)` with size 26x16 and sends updated settings after
    `toggleFiltering`; `CraftingRecipeBookComponent` uses
    `recipe_book/filter_{enabled,disabled}` plus highlighted variants, while
    `FurnaceRecipeBookComponent` uses the `furnace_filter_*` sprites. bbb now
    renders and highlights those sprites when the recipe book is open, locally
    flips the matching `RecipeBookSettings.filtering` value while preserving
    `open`, and queues `RecipeBookChangeSettingsCommand`.
  - Done 2026-07-08 — Recipe-book search input shell. Vanilla anchors:
    `RecipeBookComponent.initVisuals` creates an `EditBox` at `(xo + 25,
    yo + 13)` with size 81x14, `maxLength = 50`, white text, and value
    preservation; `RecipeBookComponent.keyPressed` lets a focused visible
    search box consume non-Escape keys and `charTyped` updates the search
    string. bbb now loads vanilla `widget/text_field` /
    `widget/text_field_highlighted`, projects the search box on the
    recipe-book panel, keeps search text/focus locally in `ClientInputState`,
    edits printable text with backspace/delete/arrows/Ctrl+A, focuses on
    click or chat-key, and prevents focused search from closing the container
    on `E`.
  - Boundary: recipe-book tabs/page buttons, recipe placement commands from
    clicks, tab notification animation, recipe-search filtering of visible
    recipe buttons, cursor/selection rendering inside the search box, and
    ghost recipe slot rendering remain open. The filter toggle and search
    text states are live, but visible filtered recipe-grid contents wait for
    the page recipe button implementation. The first shell models the
    non-narrow layout; vanilla's narrow-screen overlap mode (`width < 379`)
    remains for the input/render follow-up.
  - Done 2026-07-08 — Jumpable-vehicle contextual bar. Vanilla anchors:
    `Gui.willPrioritizeJumpInfo` / `nextContextualInfoState` select
    `JUMPABLE_VEHICLE` when `player.getJumpRidingScale() > 0` or the
    jumpable mount has `getJumpCooldown() > 0`, and in the no-locator case
    `canShowVehicleJumpInfo()` takes the contextual slot before experience;
    `JumpableVehicleBarRenderer.extractBackground` draws
    `hud/jump_bar_background` at the 182x5 `ContextualBarRenderer` rect, then
    either full-width `hud/jump_bar_cooldown` while
    `PlayerRideableJumping.getJumpCooldown() > 0`, or the cropped
    `hud/jump_bar_progress` width from `Mth.lerpDiscrete(player.getJumpRidingScale(),0,182)`.
    bbb now exposes the existing native riding-jump charge curve as
    `ClientInputState::riding_jump_scale`; `WorldStore::local_player_rideable_jumping_vehicle_id()`
    now applies the first-passenger gate plus the shared saddle-item
    `canJump()` gate; `RendererFrame.hud_jump_bar`
    (`HudJumpBar { progress, cooldown }`) is projected when that query is
    present; renderer-side jump bars override the experience bar while
    preserving the independent experience level number. HUD assets load the
    three vanilla `hud/jump_bar_*` sprites. Cooldown overlay is wired for
    camel/camel husk from the existing client-side `Camel.DASH` cooldown
    reconstruction; horse, donkey, mule, llama and skeleton/zombie horse use
    the interface default cooldown `0`.
  - Boundary: nautilus / zombie-nautilus dash cooldown is not yet reconstructed
    in world state, so their jump bar currently shows charge progress but not
    the cooldown overlay after a dash. Camel `canJump()` also suppresses while
    `refuseToMove()` (sitting or pose transition); bbb applies the saddle gate
    but has not yet folded that camel-specific pose gate into the
    local-player jumpable-vehicle query. Locator-bar priority remains absent
    because waypoints/locator HUD state is not implemented.
  - Tests: `bbb-native` `riding_jump_scale_matches_vanilla_local_player_curve`;
    `bbb-world`
    `local_player_rideable_jumping_vehicle_cooldown_tracks_camel_dash_cooldown`;
    `bbb-renderer`
    `jump_bar_offscreen_frame_replaces_experience_bar_and_uses_cooldown_overlay`.
  - Done 2026-07-06 — Heart variants + multi-row health stacking. New heart
    projection `RendererFrame.hud_player_health` (`HudPlayerHealth`) replaces
    the old single-row `hud_health: f32`, carrying health, the MAX_HEALTH
    attribute, absorption, the base `HeartType`, the hardcore flag, the
    Regeneration gate and the client tick. World inputs:
    `WorldStore::local_player_max_health` (MAX_HEALTH attribute, registry
    index 19, default 20.0), `local_player_is_fully_frozen`
    (`EntityStore::is_fully_frozen` = `ticksFrozen >= 140`, refactored out of
    the shaking-body check), the already-stored `local_player_absorption`, and
    the login `hardcore` flag now stored on `WorldGameplayState`
    (`WorldStore::is_hardcore`). `HeartType.forPlayer` precedence
    (Gui.java:1438-1450) = poison > wither > fully-frozen > normal, with the
    MobEffect ids derived by 0-indexed registration order (MobEffects.java:
    regeneration=9, poison=18, wither=19; consistent with the sibling
    night_vision=15 / hunger=16). Renderer: `HudHeartKind`
    (Container/Normal/Poisoned/Withered/Absorbing/Frozen) with a
    `sprite_name(hardcore, half, blinking)` that reproduces vanilla's asset
    naming asymmetry (Normal prefixes `hardcore_`, typed kinds embed it after
    their own prefix, Container appends `_hardcore` and ignores half); sprites
    stored per `[kind][variant]` and loaded by walking every combination.
    `hud_player_heart_instances` replays `extractHearts` (Gui.java:820-873):
    the descending container loop draws Container then the absorbing overlay
    (`WITHERED` keeps its own sprite, else `ABSORBING`) then the base fill, at
    `xLeft = guiWidth/2 - 91` stacking up by `healthRowHeight`
    (`numHealthRows = ceil((maxHealth + ceil(absorption)) / 2 / 10)`,
    `healthRowHeight = max(10 - (numHealthRows - 2), 3)`). The Regeneration
    wave lifts container `tickCount % ceil(maxHealth + 5)` by 2px, and at
    `currentHealth + absorption <= 4` every heart shakes by `nextInt(2)` from a
    `tickCount * 312871`-seeded `LegacyRandomSource` — reproduced exactly
    (vanilla reseeds at Gui.java:764, so unlike the wall-clock food/air shakes
    this matches the vanilla sequence). `armor_hud_rect` now takes the
    projected `(numHealthRows, healthRowHeight)` so multi-row health pushes the
    armor row up with the hearts (`yLineBase - (numHealthRows-1)*healthRowHeight
    - 10`, Gui.java:801; single-row default keeps the prior 10px gap). Boundary:
    the damage/heal **blink** flash is deferred — it needs the untracked
    `player.invulnerableTime` (no client-side sync) and the wall-clock
    `displayHealth`/`lastHealthTime` hold, neither reproducible
    deterministically; `HudPlayerHealth` therefore always draws `blinking =
    false`, though `HeartType::sprite_name` and the uploaded `*_blinking` names
    remain complete (matrix-tested) for when it lands. Tests: bbb-world
    (hardcore login flag, MAX_HEALTH attribute + 20.0 fallback, 140-tick freeze
    threshold); bbb-renderer layout (`hud_health_rows` row/height, half/empty
    splits, base-type follow, absorption append incl. odd-half + withered
    override, 2-row stacking, regen 2px lift index, hand-replayed low-health
    shake sequence, armor multi-row shift) + the sprite-name matrix (every
    kind×hardcore×half×blink resolves to a real vanilla asset, plus the
    hardcore-naming asymmetry) + an offscreen readback sentinel (poison swaps
    the base fill sprite). Air/vehicle/armor linked-y layout tests unchanged.
  - Done 2026-07-06 — Air bubbles + vehicle hearts + the world metadata
    chain. Synched-data ids derived per inheritance chain (never from bbb
    tests): `Entity.DATA_AIR_SUPPLY_ID` = 1 (Entity.java:255-271 field
    order), `LivingEntity.DATA_HEALTH_ID` = 9 (LivingEntity.java:178-186),
    `Player.DATA_PLAYER_ABSORPTION_ID` = 17 (after Avatar's main-hand 15 /
    mode-customisation 16, Avatar.java:38-39; Player.java:134-139 —
    cross-checked against the existing shoulder-parrot ids 19/20).
    `EntityStore` gained `metadata_float` plus `air_supply` (default 300 =
    the define-time `getMaxAirSupply()`, Entity.java:312,2725-2727),
    `living_entity_health` (default 1.0F, LivingEntity.java:314) and
    `player_absorption` (default 0.0F, Player.java:224; stored + queryable
    via `local_player_absorption`, not yet drawn). Air bubbles
    (`Gui.extractAirBubbles`, Gui.java:887-928): visibility
    `isUnderWater || clamp(air) < max` (:891); `Mth.ceil((cur+off)*10/max)`
    bubble counts with off = -2 (full) / 0 (popping position) / the one-tick
    underwater refill delay (empty, :922-928); the popping `hud/air_bursting`
    frame draws only underwater (:906) and the delay gap draws nothing for a
    tick; the all-empty even-tick `nextInt(2)` wobble (:910) reuses the
    food-shake frame-seeded LCG. The y line replays
    `extractPlayerHealth`+`getAirBubbleYLine` (:772,784-792,917-920) —
    `guiHeight-49` on foot and on 1-row-heart vehicles, -10 per extra
    vehicle heart row. Vehicle hearts (`extractVehicleHealth` /
    `getVehicleMaxHearts`, Gui.java:709-741,974-1005): living direct vehicle
    only (base `showVehicleHealth()` = `instanceof LivingEntity`,
    Entity.java:2349-2351, no 26.1 overrides), hearts =
    `(int)(maxHealth+0.5F)/2` capped 30, `ceil(health)` against
    `i*2+1+baseHealth` per 20-half-heart row stacking up from
    `guiHeight-39`, replacing the food row while hearts > 0 (:784-788).
    Vehicle max health reads the MAX_HEALTH attribute (registry index 19,
    Attributes.java field order) from the already-stored per-entity
    `UpdateAttributes` (vanilla always pairs syncable attributes on tracking
    start, ServerEntity.java:282-284), falling back to the RangedAttribute
    default 20.0 (Attributes.java:58-60) in the unsynced window. Projected as
    `RendererFrame.hud_air`/`hud_vehicle_health` (one-submission invariant
    kept); sprites `hud/air{,_bursting,_empty}` +
    `hud/heart/vehicle_{container,full,half}` (Gui.java:103-108) via the gui
    atlas. Boundary: the bubble-pop sound (`playAirBubblePoppedSound`,
    Gui.java:930-937) is deferred (no HUD-side sound sink yet); heart
    variants + multi-row stacking are the next slice above. Tests: bbb-world
    metadata-chain derivations (air/absorption/vehicle health incl. boat
    `None`, attribute fallback 20, metadata default 1.0); bbb-renderer
    layout rows (671/661/651 air, 681 vehicle baseline) and hand-computed
    formula cases (150 → 5 full + delay gap + 4 empty, 61 → popping index 2
    suppressed on land, 0/negative all-empty, max-hearts 15→7 / 15.5→8 /
    100→30); two offscreen sentinels (underwater full bubbles vs hidden at
    full air on land; vehicle hearts replacing the food row and a 0-heart
    vehicle keeping it) (bbb-renderer `hud.rs`).
  - Done 2026-07-06 — Armor bar. `WorldStore::local_player_armor_value`
    (bbb-world `client/local_player.rs`) derives vanilla
    `LivingEntity.getArmorValue()` = `Mth.floor(getAttributeValue(Attributes.ARMOR))`
    (LivingEntity.java:1845-1846): the already-stored synced ARMOR attribute
    (`BuiltInRegistries.ATTRIBUTE` registration index `0`, Attributes.java:10)
    folded through the shared `AttributeInstance.calculateValue` implementation
    (`entities::store::vanilla_attribute_value`: add, then multiply_base, then
    multiply_total), floored to an int, `0` without the attribute. Projected as
    `RendererFrame.hud_armor` (native `runtime.rs` → `render_extract.rs` →
    `Renderer::set_hud_armor`), drawn in `collect_hud_draws` before the hearts
    (vanilla `Gui.extractPlayerHealth` order, Gui.java:779/781) only when
    `armor > 0` (`Gui.extractArmor`, Gui.java:800): 10 icons choosing
    full/half/empty per `hud_armor_fill` (`i*2+1` vs armor, Gui.java:805-814)
    from the `hud/armor_{full,half,empty}` sprites (Gui.java:94-96), placed by
    `armor_hud_rect` one row (10px) above the heart baseline
    (`yLineArmor = yLineBase - (numHealthRows-1)*rowHeight - 10`, Gui.java:801;
    bbb draws a single health row so `numHealthRows == 1` collapses the stacked
    term to 0). Boundary: multi-row health stacking (absorption/`maxHealth`
    rows shifting the armor row further up) still deferred with the rest of the
    heart-variant work above; the armor row uses the single-row offset. Tests:
    world attribute→armor derivation incl. floor + full modifier formula +
    zero-default (bbb-world `local_player.rs`); `hud_armor_fill` combos
    (armor 7 → 3 full/1 half/6 empty) and `armor_hud_rect` layout constants
    (bbb-renderer `hud/layout.rs`); offscreen whole-frame sentinel proving the
    armor row paints when `armor > 0` and stays background when `armor == 0`
    (bbb-renderer `hud.rs`).
  - Done 2026-07-06 — Offscreen whole-frame readback harness. `render()` no
    longer hard-requires a swapchain: frame acquisition moved into
    `RenderSurface::acquire_frame` (renderer.rs), an enum over the window
    surface and a `cfg(test)` injected offscreen texture, returning a
    `FrameTarget` whose `texture()`/`present()` the frame steps consume
    (`transparency_blit_pass` / `first_person_item_pass` / `hud_passes` /
    `finish_frame` now take `FrameTarget`; surface-path semantics are
    byte-identical — Lost/Outdated reconfigure+skip, Timeout skip, same
    present and screenshot flow). Renderer construction split:
    `Renderer::new` keeps window/adapter negotiation and delegates to
    `with_gpu` (all pipelines/targets), so `Renderer::new_offscreen(w, h)`
    (`cfg(test)`, adapter-or-skip → `None`) builds the full production
    pipeline set headless over a `Bgra8UnormSrgb` offscreen target, and
    `render_offscreen_frame()` runs the complete FRAME_STEPS frame and reads
    pixels back through the shared screenshot path — `prepare_screenshot_copy`
    plus the new `read_screenshot_pixels` split out of `finish_screenshot`
    (single home of the 256-byte padded-row and BGRA→RGBA handling; the PNG
    save is now a thin wrapper over it). All 42 render.rs source-order
    assertions and both FRAME_STEPS meta tests hold with only the
    acquisition line and one comment changed inside `render()`. Proof test
    `offscreen_frame_renders_hud_sentinel_over_clear_color` (offscreen.rs):
    a 4x4 red crosshair over a blue clear color at 320x240 — center pixel
    red, corner blue, counters prove the whole frame ran (`frame_index == 1`,
    `hud_draw_calls >= 1`, `draw_calls >= 4`); passes on llvmpipe. Migration
    example: `hud_block_item_renders_visible_pixels_in_its_slot` rewritten
    from ~230 lines of hand-rolled device/pipeline/pass/readback onto the
    harness via public state APIs (`update_terrain_texture_atlas`,
    `set_hud_hotbar_block_item_models`, `update_camera`), keeping the same
    slot-center/corner pixel assertions. Constructing the full pipeline set
    headless immediately caught two latent shader bugs no test had ever
    compiled (both would panic production startup on
    `create_shader_module`): the translucent-emissive entity shader used
    WGSL-invalid swizzle assignment (`texel.rgb = mix(...)`, shipped
    2026-06-30) and all four outline post-chain shaders (since 2026-06-29)
    indexed a `let` array with the dynamic vertex index (naga requires
    `var`); both fixed in place. Boundary: the remaining hand-rolled
    readback tests stay as-is and migrate mechanically when next touched —
    hud.rs `hud_entity_preview_pip_renders_and_blits_isolated_entity_pixels`,
    item_models.rs
    `first_person_held_item_renders_visible_pixels_and_swing_moves_them`,
    entity_models/tests/player.rs and ender_dragon.rs pixel tests (they
    exercise isolated sub-passes whose state lacks public upload paths).
    `resize()` is a no-op on the offscreen surface (tests pick the size at
    construction).
  - Done 2026-07-05 — Experience level number + hunger food-bar jitter. The
    level is projected (`experience.level`) into `RendererFrame.hud_experience_level`
    and gated `> 0` by `set_hud_experience_level` (vanilla
    `Gui.java:533`; `hasExperience()` game-mode gate not modeled — bbb draws
    XP HUD whenever the experience state exists, matching how the progress bar
    already projects). Drawn in `collect_hud_draws` after the food row and
    before the boss overlay (vanilla `Gui.extractRenderState` order), centered
    at `x=(guiWidth-font.width)/2`, `y=guiHeight-24-9-2` via a reused
    styled-text pass: four `-16777216` black `(±1,0)/(0,±1)` copies then the
    `-8323296` (0x80FF20) green center, all `dropShadow=false`
    (ContextualBarRenderer.java:35-44; lang `gui.experience.level = "%s"`).
    Vanilla draws the level independent of the contextual bar, so jump/locator
    bars need no suppression (bbb now implements the jumpable-vehicle bar;
    locator state remains absent).
    Food-bar shake mirrors `Gui.extractFood` (Gui.java:958-960): per-icon
    `yo += random.nextInt(3)-1` (∈{-1,0,1}) applied to both the empty
    background and the fill of each index, gated on `saturation<=0 &&
    tickCount % (foodLevel*3+1) == 0`. The tick modulo reads the real client
    tick (`LightmapTickState.client_tick_count`); the offset LCG is the exact
    `nextInt(3)` clone (`HudObfuscatedRandom`) but reseeded per frame from the
    render frame counter (vanilla's wall-clock `RandomSource` is
    unreproducible, so the shake flickers deterministically instead). Hunger
    potion swap: under `MobEffects.HUNGER` (registry id 16, derived from
    MobEffects.java:70 via the raw `holderRegistry` stream codec) the row draws
    the `food_{empty,half,full}_hunger` sprites (loaded in `hud_assets.rs`),
    falling back to the base sprite when a variant is unloaded. Boundary: no
    game-mode gate (creative still shows XP HUD); the shake seed diverges from
    vanilla's wall-clock sequence by design.
  - Done 2026-07-05 — Boss bars render: `ClientHudState.boss_bars` projects
    per frame as an ordered `Vec<HudBossBar>` (plain-run name + latest packet
    progress + `HudBossBarColor`/`HudBossBarOverlay` enums whose `name()`
    strings are the vanilla `BossEvent` getName vocabularies) and draws in
    `collect_hud_draws` between the status bars and the overlay message
    (vanilla `Gui.extractRenderState` order, Gui.java:203-217). Vanilla
    anchors (BossHealthOverlay.java): 182x5 sheets at `x = guiWidth/2 - 91`,
    stacked from y=12 stepping 10+9 with the draw-then-check `guiHeight/3`
    cutoff (:63-77, first bar always draws); per-bar layer order colored
    background → notched background → (only when width > 0) colored progress
    → notched progress, cropped to `Mth.lerpDiscrete(progress, 0, 182)` =
    `floor(p*181) + (p>0 ? 1 : 0)` with the UV taking the left `width/182`
    band (:84-106, Mth.java:527-531); the name centers at
    `(guiWidth/2 - w/2, y-9)` in opaque white with the default drop shadow
    (:71-73). All 22 `boss_bar/*` sprites load from the vanilla GUI atlas
    through the same single-texture upload path as the other HUD sprites.
    Boundary: bars project in UUID order (the world keys a BTreeMap;
    vanilla's LinkedHashMap packet-arrival order is not tracked); progress
    renders the latest packet value (`LerpingBossEvent`'s 100ms wall-clock
    lerp is not modeled); `darken_screen`/`create_world_fog` stay behind the
    world's `boss_overlay_should_*` queries with no sky/fog consumer yet,
    and `play_music` has no audio consumer — all three remain deferred here.
  - Done 2026-07-05 — Actionbar + titles + subtitles render: `Gui.tick`
    countdowns now advance in `WorldStore::advance_hud_text_ticks` (raw
    client ticks, outside the tick-rate freeze gate; titleTime→0 clears
    title+subtitle), projected per frame as
    `HudActionBarText`/`HudTitleText` (styled runs + post-tick remaining
    ticks + fade windows + partial tick + jukebox `animate_color` flag) and
    drawn in `collect_hud_draws` through the one styled-text pipeline
    (`hud_styled_text_pass_geometry` gained a pose-`scale` input; 1.0
    reproduces the label path bit-for-bit). Vanilla anchors: overlay fade
    `(int)(t*255/20)` cap 255, gate `alpha > 0`, pos `(guiWidth/2,
    guiHeight-68)` + `(-w/2, -4)` (Gui.java:308-336); title fade-in
    `(total-t)*255/fadeIn`, fade-out `t*255/fadeOut`, clamp 0..255, center
    pose + 4x `(-w/2, -10)` / subtitle 2x `(-w/2, 5)`, `ARGB.white(alpha)`
    (Gui.java:338-377); rainbow `Mth.hsvToArgb(t/50, 0.7, 0.6, alpha)` with
    its h-mod-6/f-unwrapped quirk kept (Mth.java:451-497) — hue is
    remaining-time-driven, so deterministic. 26.1 has no legacy `alpha < 8`
    discard and no low-alpha force-opaque (`Font.java` alpha passthrough).
    Boundary: protocol still flattens these components to plain text
    (`decode_component_summary_from_decoder`), so lines project as single
    unstyled runs; the accessibility text backdrop
    (`textWithBackdrop`, option default 0) is skipped; the jukebox
    now-playing path (`Gui.setNowPlaying`, the only `animate_color=true`
    producer) is not yet wired — the flag is carried end-to-end. Note
    `extractSubtitleOverlay` is the sound-captions overlay, not the title
    subtitle (which draws inside `extractTitle`), and stays deferred.
  - Verified aligned on 2026-07-05: crosshair, hotbar + selection + item
    icons (flat + 3D pass), hearts/food base tiers, experience progress bar,
    the 22-variant container screen family incl. merchant trading UI and
    lectern/held book reading, and HUD pass color/depth semantics (all HUD
    sub-passes Load over the world blit; the 3D-item depth clear mirrors
    vanilla `GuiItemAtlas` per-slot clears). Vanilla's stratum/blur
    depth-clear (`GuiRenderer.java` before/after-blur) only matters once a
    blur-backed screen exists — deferred with that trigger recorded.
  - HUD text drawing can reuse the completed vanilla font stack (styled
    runs, shadow, color) with no font-side prerequisites.

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
    - future level-event audio changes when vanilla sources or supported side
      effects expand
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
      - bone meal use for event `1505`, emitted after
        `BoneMealItem.addGrowthParticles` random consumption
      - honeycomb wax on for event `3003`, emitted after the vanilla
        block-face `minecraft:wax_on` particle random sequence
      - brush-block-complete event `3008` for suspicious sand/gravel, using
        the event-data `BrushableBlock.getBrushCompletedSound()` mapping and
        `SoundSource.PLAYERS`
    - native dispatcher playback for vanilla entity-event local positioned
      sounds:
      - totem use entity event `35`, emitted after the totem
        `TrackingEmitter` side effect at the entity position as
        `minecraft:item.totem.use`, with source/category mapped from
        `Entity.getSoundSource()`
      - armadillo peek entity event `64`, emitted at the entity position as
        `minecraft:entity.armadillo.peek`, with neutral source and fixed
        volume/pitch `1.0`
      - armor-stand hit entity event `32`, emitted at the entity position as
        `minecraft:entity.armor_stand.hit`, with neutral source, volume `0.3`,
        and pitch `1.0`
      - armor-stand death entity event `3`, emitted at the entity position as
        `minecraft:entity.armor_stand.break`, with the generic LivingEntity
        silent gate and death-event pitch randomization
      - zombie death entity event `3`, emitted at the entity position as
        `minecraft:entity.zombie.death`, with hostile source, the generic
        LivingEntity silent gate, and death-event pitch randomization
      - zombie-villager death entity event `3`, emitted at the entity position
        as `minecraft:entity.zombie_villager.death`, with hostile source, the
        generic LivingEntity silent gate, and death-event pitch randomization
      - ravager and iron-golem death entity event `3`, emitted at the entity
        position as `minecraft:entity.ravager.death` /
        `minecraft:entity.iron_golem.death`, with hostile / neutral source,
        the generic LivingEntity silent gate, and death-event pitch randomization
      - witch and villager death entity event `3`, emitted at the entity
        position as `minecraft:entity.witch.death` /
        `minecraft:entity.villager.death`, with hostile / neutral source, the
        generic LivingEntity silent gate, and death-event pitch randomization
      - skeleton, stray, and bogged death entity event `3`, emitted at the
        entity position as `minecraft:entity.skeleton.death` /
        `minecraft:entity.stray.death` / `minecraft:entity.bogged.death`, with
        hostile source, the generic LivingEntity silent gate, and death-event
        pitch randomization
    - native dispatcher playback for randomized vanilla `LevelEventHandler`
      sounds using a runtime-local `LegacyRandomSource`-shaped `nextFloat()`:
      - fire extinguish / generic extinguish
      - ghast/blaze/dragon/wither/zombie/skeleton/phantom hostile effects
      - anvil, grindstone, book, smithing table, dripstone, wind charge
      - lava extinguish and redstone torch burnout sounds
      - splash/instant-effect potion break sounds for events `2002` and
        `2007`, emitted after the item-break and spell-particle random draws
      - dragon fireball explode sound for event `2006` when `data == 1`,
        emitted after the 200 dragon-breath particle random draws
      - distance-delayed trial spawner sounds for events `3012`, `3013`,
        `3014`, `3019`, `3020`, and `3021`; audio-only dispatch advances the
        vanilla post-sound spawn / detect-player / eject-item /
        become-ominous particle random streams before later LevelEvent sound
        seeds
      - distance-delayed vault activate/deactivate sounds for events `3015`
        and `3016`, emitted after the local vault particle random sequence;
        `3015` is gated on a loaded vault block entity at the event position
      - end gateway spawn and ender dragon growl sounds
      - sculk charge sounds for event `3006`, including the fixed pop branch
        and the randomized charged branch; audio-only dispatch advances the
        post-sound charged block-face or pop-particle random stream before
        later LevelEvent sound seeds, including the full-block-context 40-pop
        branch
      - sculk-shrieker sound for event `3007`, gated off when the loaded event
        block state has `waterlogged=true`, using
        `SculkShriekerBlock.TOP_Y`, volume `2.0`, and
        `0.6 + random.nextFloat() * 0.4` pitch after the shriek particles
      - sculk charge pop particles for event `3006` `data >> 6 == 0`: native
        level-event particle context now carries the target block full-shape
        classification, and the resolver emits `minecraft:sculk_charge_pop`
        with vanilla `20` / `40` count, `0.25` / `0.45` spread, and `0.07`
        velocity scale
      - sculk-shrieker event `3007` now emits the vanilla ten
        `minecraft:shriek` particles at `SculkShriekerBlock.TOP_Y` with
        `ShriekParticleOption(i * 5)` delays, then records/plays the
        waterlogged-gated `SCULK_SHRIEKER_SHRIEK` positioned sound when the
        loaded event block state is not waterlogged
      - lava extinguish and redstone torch burnout now share the dispatcher
        path with renderer smoke side effects for events `1501` and `1502`;
        audio-only dispatch also advances the post-sound smoke particle random
        stream for events `1501`, `1502`, and `1503` before later LevelEvent
        sound seeds
      - particle-only events `2000`, `2003`, `2004`, `2009`, and `2010`
        advance their vanilla smoke / ender-eye item-break / blaze smoke /
        cloud / white-smoke random streams in audio-only dispatch before later
        LevelEvent sound seeds
      - particle-only block-face / axis events `3002`, `3004`, `3005`, and
        `3009` advance their vanilla electric-spark, wax-off, scrape, and
        egg-crack random streams in audio-only dispatch before later LevelEvent
        sound seeds
      - cobweb place event `3018` consumes the vanilla poof-particle random
        sequence before recording/playing:
        - `minecraft:block.cobweb.place`
        - `SoundSource.BLOCKS`
        - volume `1.0`
        - pitch `(nextFloat - nextFloat) * 0.2 + 1.0`
        - `distanceDelay=true`
      - particle descriptors map `SpitParticle.Provider` to the
        `ExplodeParticle` random gray tint, age sprite, quad-size and lifetime
        formulas while overriding gravity to `0.5`; `ExplodeParticle.Provider`
        / `minecraft:poof` uses vanilla `command + +/-0.05` velocity and
        `0.9` friction.
      - particle descriptors map `FireworkParticles.SparkProvider` for
        `minecraft:firework` to age sprites, vanilla `SimpleAnimatedParticle`
        friction `0.91`, gravity `0.1`, full-bright light, translucent
        particle layer, command velocity, `0.75` quad-size scale, fixed
        initial alpha `0.99`, `48 + random.nextInt(12)` lifetime, and the
        half-lifetime alpha fade formula. Firework rocket entity event `17`
        with empty/no explosions now emits vanilla `minecraft:poof` particles;
        non-empty explosions now project `FireworkParticles.Starter` small /
        large ball, star, creeper, and burst base spark shapes, center `flash`,
        per-spark fade-colors, trail child spark duplication, and twinkle
        visibility gating, plus the life-0 blast / large_blast local ambient
        sound and delayed twinkle / twinkle_far local ambient sound.
      - particle descriptors map `DripParticle.HoneyHangProvider`,
        `HoneyFallProvider`, and `HoneyLandProvider` for `dripping_honey`,
        `falling_honey`, and `landing_honey` to random sprites, vanilla
        DripParticle opaque layer, zero initial velocity, physics metadata,
        fixed honey tints, `0.98` friction, direct gravity motion,
        hang-particle `0.02` post-move damping, lifetimes `100`,
        `64/(random*.8+.2)`, and `128/(random*.8+.2)`, with gravity
        `0.000012`, `0.01`, and `0.06`. The falling provider now removes on
        `onGround` through the collision-backed `move` path. The landing
        provider now uses collision-backed `DripParticle` move/friction without
        `WaterDropParticle`'s random on-ground removal branch. Hang-to-fall
        and fall-to-land child spawning now use vanilla lifetime/on-ground
        triggers through renderer child templates. Falling honey ground hits
        now emit positioned audio through renderer particle sound events for
        `minecraft:block.beehive.drip`.
      - particle descriptors map `DripParticle.ObsidianTearHangProvider`,
        `ObsidianTearFallProvider`, and `ObsidianTearLandProvider` for
        `dripping_obsidian_tear`, `falling_obsidian_tear`, and
        `landing_obsidian_tear` to random sprites, vanilla DripParticle opaque
        layer, zero initial velocity, physics metadata, fixed purple tint,
        `0.98` friction, direct gravity motion, hang-particle `0.02`
        post-move damping, glowing block-light override, lifetimes `100`,
        `64/(random*.8+.2)`, and `28/(random*.8+.2)`, with gravity
        `0.000012`, `0.01`, and `0.06`. The falling provider now removes on
        `onGround` through the collision-backed `move` path. The landing
        provider now uses collision-backed `DripParticle` move/friction without
        `WaterDropParticle`'s random on-ground removal branch. Hang-to-fall
        and fall-to-land child spawning now use vanilla lifetime/on-ground
        triggers through renderer child templates.
      - particle descriptors map `BubblePopParticle.Provider` to fixed lifetime
        `4`, age sprites, command velocity, default `SingleQuadParticle` white
        tint / quad-size sampling, gravity `0.008`, and its custom tick path
        that subtracts full gravity without applying default friction.
      - particle descriptors map `AttackSweepParticle.Provider` to xAux-derived
        quad size, random gray tint, fixed lifetime `4`, age sprites, vanilla
        zero-aux `Particle` constructor velocity sampling, and its no-motion
        tick path plus full-bright light coords.
      - particle descriptors map `SuspendedParticle.UnderwaterProvider` to
        vanilla `y - 0.125` initial position, random sprite selection, fixed
        blue tint, `SingleQuadParticle` quad-size sampling times `0.2..0.8`,
        `8/(random*.8+.2)` lifetime, zero velocity, friction `1.0`, and
        no-physics metadata.
      - particle descriptors map `SculkChargeParticle.Provider` to command
        velocity, alpha `1.0`, `1.5` quad-size scaling, age sprites,
        `8..=19` lifetime, friction `0.96`, no-physics metadata,
        translucent particle layer, full-bright block override, and decoded
        `SculkChargeParticleOptions.roll` billboard rotation.
      - particle descriptors map `TrialSpawnerDetectionParticle.Provider` for
        `trial_spawner_detection` and `_ominous` to age sprites, the
        `BaseAshSmokeParticle` `dir=(0.0, 0.9, 0.0)` base-spread velocity with
        the command velocity threaded through (no offset),
        `scale(1.5)` over the vanilla `0.75` single-quad scale,
        `12/(0.5+random*.5)` lifetime, opaque particle layer, full-block
        light, grow-to-base size curve, physics metadata, and vanilla
        `FacingCameraMode.LOOKAT_Y` vertex orientation with world-Y up.
      - particle descriptors map `DustPlumeParticle.Provider` for
        `dust_plume` to age sprites, `BaseAshSmokeParticle` `0.75`
        quad-size scale, `7/(random*.8+.2)` lifetime, command velocity plus
        `0.15` y offset, opaque particle layer, no-physics metadata, `0.5`
        initial gravity, `0.96` friction, `ARGB(0xBAB1C2)-random*.2`
        tint, grow-to-base size curve, and provider tick motion that decays
        gravity by `0.88` and friction by `0.92` before default motion.
      - particle descriptors map `AshParticle.Provider` and
        `WhiteAshParticle.Provider` for `ash` / `white_ash` to age sprites,
        `BaseAshSmokeParticle` `0.75` quad-size scale, `20/(random*.8+.2)`
        lifetime, opaque particle layer, no-physics + speed-up-when-Y-blocked
        metadata, `0.96` friction, and the vanilla `BaseAshSmokeParticle`
        base-spread velocity: the `Particle` 7-arg zero-aux normalized spread
        scaled per axis by `dir = (0.1, -0.1, 0.1)` (`xd *= dirX; yd *= dirY;
        zd *= dirZ`) so the y component is negated and `0.1`-damped, then the
        provider velocity added (`xd += xa; yd += ya; zd += za`). `ash` adds
        `(0, 0, 0)` with `0.5` random-gray tint and `0.1` gravity; `white_ash`
        ignores the command velocity and adds its own
        `xa = rand*-1.9*rand*.1`, `ya = rand*-0.5*rand*.1*5.0`,
        `za = rand*-1.9*rand*.1` with fixed `ARGB(0xBAB1C2)` tint and
        `0.0125` gravity. Vanilla passes `hasPhysics=false` for both providers,
        so there is no provider-specific collision removal path.
      - particle descriptors map `WaterDropParticle.Provider` and
        `SplashParticle.Provider` for `rain` / `splash` to random sprites,
        vanilla single-quad size, `8/(random*.8+.2)` lifetime, opaque
        particle layer, physics metadata, `0.98` friction, direct gravity
        motion, and damping. `rain` uses constructor random x/z velocity
        damped by `0.3`, `0.1..0.3` y velocity, and `0.06` gravity;
        `splash` uses `0.04` gravity and the vanilla horizontal command
        branch `(xa, 0.1, za)`. Runtime ticks now use collision-backed `move`
        and apply vanilla `onGround` 50% random removal plus X/Z ground damping;
        block/fluid in-block removal now queries the world surface height as
        `max(collisionShape.max(Y, localX, localZ), fluidState.height)` and
        removes `rain` / `splash` below that surface.
      - particle descriptors map `WakeParticle.Provider` for `fishing` to
        first sprite initialization, vanilla single-quad size,
        `8/(random*.8+.2)` lifetime, command velocity, opaque particle layer,
        physics metadata, `0.98` friction, zero gravity,
        `setSize(0.01F,0.01F)` collision bounds, collision-backed `move`,
        damping, and wake sprite cycling via `SpriteSet.get((60-lifetime)%4, 4)`
        during runtime ticks.
      - particle descriptors map `FallingLeavesParticle.CherryProvider`,
        `PaleOakProvider`, and `TintedLeavesProvider` for `cherry_leaves` /
        `pale_oak_leaves` / `tinted_leaves` to random sprites, fixed `300`
        lifetime, opaque particle layer, `1.0` friction, physics metadata,
        vanilla `scale * (0.05 | 0.075)` quad-size choice, cherry flow-away
        parameters `(fall=0.25, side=2.0, startVelocity=0.0)`, pale/tinted
        swirl parameters `(fall=0.07, side=10.0, startVelocity=0.021)`,
        tinted `ColorParticleOption` ARGB command decode with renderer RGB
        tinting and default alpha preservation, gravity
        `fallAcceleration * 1.2 * 0.0025`, flow/swirl acceleration, and roll
        spin acceleration. Runtime ticking now routes the leaf move through the
        world collision callback, removes on `onGround`, removes on horizontal
        axis blocking after the first tick, and preserves the vanilla first-tick
        horizontal-block grace.
      - particle descriptors map
        `FlyStraightTowardsParticle.OminousSpawnProvider` for
        `ominous_spawning` to random sprites, command velocity, initial
        position at `spawn + velocity` with `spawn` retained as the
        interpolation start, vanilla `0.1*(random*.5+.2)` quad size followed
        by `scale(randomBetween(3,5))`, `25+random*5` lifetime, opaque
        particle layer, no-physics metadata, full-block light, and the
        straight-toward tick path plus `ARGB.srgbLerp` from `0xFF45AEFE` to
        white.
      - particle descriptors map `ShriekParticle.Provider` to
        `ShriekParticleOption.delay` carried in
        `ParticleSpawnCommand.initial_delay_ticks`; delayed instances do not
        tick or emit vertices while `delay > 0`, then use vanilla random sprite
        selection, fixed `0.85` quad size, `30` lifetime, fixed `(0, 0.1, 0)`
        velocity, translucent layer, full-block light override,
        `0.85 * clamp((age + partial) / lifetime * 0.75, 0, 1)` size curve,
        linear alpha fade, and the vanilla two rotated quads from
        `ShriekParticle.extract` (`rotationX(-1.0472)` and
        `rotationYXZ(-PI, 1.0472, 0)`).
      - particle descriptors map `EndRodParticle.Provider` to command velocity,
        `0.75` quad-size scaling, age sprites, `60..=71` lifetime, friction
        `0.91`, gravity `0.0125`, full-bright light coords, translucent
        particle layer, `SimpleAnimatedParticle` half-lifetime alpha fade, and
        `setFadeColor(15916745)` RGB fade toward `0xF2DEC9` by 20% per tick
        after half lifetime. Runtime ticks preserve vanilla `hasPhysics=true`
        metadata while using the EndRod-specific collision-free `move` override
        so world collision callbacks cannot stop its motion.
      - particle descriptors map `LavaParticle.Provider` to random sprite
        selection, constructor-random horizontal velocity damped by `0.8`,
        random upward velocity `0.05..0.45`, `0.2..2.2` quad-size scaling,
        vanilla `1 - progress^2` shrinking size curve,
        `16/(random*.8+.2)` lifetime, friction `0.999`, gravity `0.75`,
        physics metadata, full-bright block light, and child smoke emission
        using the command-carried smoke SpriteSet and vanilla
        `random.nextFloat() > age / lifetime` post-tick odds.
      - particle descriptors map `CampfireSmokeParticle.CosyProvider` and
        `SignalProvider` to random sprites, constructor `scale(3.0)`, alpha
        `0.9` / `0.95`, `80..129` / `280..329` lifetime, command x/z velocity
        plus `yAux + random.nextFloat() / 500.0`, gravity `3.0E-6`, physics
        metadata, translucent particle layer, vanilla `0.25` x `0.25`
        collision size, random x/z drift, collision-backed `move`, alpha-`<= 0`
        pre-motion removal, and final 60-tick alpha fade.
      - particle descriptors map `SnowflakeParticle.Provider` to age sprites,
        fixed pale-blue tint, `0.1 * (random * random + 1.0)` quad size,
        command velocity plus random `+-0.05` per axis,
        `16/(random*.8+.2)+2` lifetime, friction `1.0`, gravity `0.225`,
        physics metadata, opaque particle layer, and vanilla post-tick damping
        (`xd *= 0.95`, `yd *= 0.9`, `zd *= 0.95`).
      - particle descriptors map `SuspendedParticle.CrimsonSporeProvider` and
        `WarpedSporeProvider` to random sprites, `y - 0.125` initial position,
        `0.6..1.2` quad-size multiplier, `16/(random*.8+.2)` lifetime, no
        physics, friction `1.0`, zero gravity, opaque layer, crimson gaussian
        micro-drift with `[0.9, 0.4, 0.5]` tint, and warped downward random
        drift with `[0.1, 0.1, 0.3]` tint.
      - particle descriptors map `SquidInkParticle.Provider` and
        `GlowInkProvider` to age sprites, fixed `0.5` quad size, black /
        glow-ink tint, command velocity, `6/(random*.8+.2)` lifetime, friction
        `0.92`, zero gravity, no-physics metadata, and full-bright light coords.
        The translucent particle layer and `SimpleAnimatedParticle`
        half-lifetime runtime alpha fade are represented. Runtime ticks now
        apply the vanilla post-`super.tick()` `yd -= 0.0074F` downward drift
        when the post-move world block sample is air.
      - particle descriptors map `SimpleVerticalParticle.PauseMobGrowthProvider`
        and `ResetMobGrowthProvider` to random sprites, random `0.5..1.1`
        quad-size scaling, fixed lifetime `8`, command velocity with
        `-0.03` / `+0.03` y offset, default `0.98` friction, zero gravity, and
        physics metadata plus vanilla opaque particle layer.
      - particle descriptors map `FallingDustParticle.Provider` for
        `minecraft:falling_dust` to age sprites, zero constructor velocity,
        vanilla `32/(random*.8+.2)` lifetime scaled by `0.9`, `0.67499995`
        quad-size multiplier, grow-to-base size curve, default `0.98`
        friction, zero gravity, physics metadata, ordinary opaque particle
        atlas layer, sampled roll / rotSpeed, and tick motion that rotates,
        moves by current velocity, subtracts `0.003` from Y velocity, and clamps
        it to `-0.14`. Native spawn resolution now mirrors the provider's
        `!state.isAir() && state.getRenderShape() == RenderShape.INVISIBLE`
        rejection for water/lava, bubble column, barrier, structure void, end
        portal/gateway, light, and moving piston while preserving packet sample
        RNG consumption before the rejected provider result. Its
        `FallingBlock#getDustColor` branch is
        projected into `ParticleSpawnCommand.option_color` for sand/red_sand,
        gravel, dragon_egg, anvils, and concrete_powder states; non-FallingBlock
        vanilla `BlockColors.createDefault()` layer-0 tint is also installed for
        constant, default-colormap, redstone power, stem age, and lily pad
        world-color sources. Vanilla static mapColor fallback now covers
        foundational non-tinted stone/dirt/planks, wood/log/bamboo axis states,
        wooden stairs/slabs/pressure plates/doors/trapdoors/fences/fence gates/
        signs/hanging signs/shelves, banner/wall banner `WOOD`, button,
        glass/glass pane/iron bars/iron chain/copper bars/copper chains,
        redstone/slime/bone/frosted-ice/dirt-path/petrified-slab misc static
        blocks, ladder/torch/end rod, rail/redstone fixture, skull/head, non-tinted
        potted, cake, air / cave_air / void_air, and test_instance_block
        default-NONE groups, crimson/warped stem/hyphae colors,
        DyeColor / colored terracotta families, bed/candle/shulker decorative
        families, cave/emissive block families, copper weathering families,
        nether flora / blackstone static families, quartz/prismarine/End static families, construction
        stone/brick static families, deepslate construction variants,
        infested stone CLAY variants, resin/pale garden static families,
        plant/dripstone/moss/root/mud natural static families, non-tinted
        foliage static families, crop/succulent static families,
        produce/fungus static families, utility/mechanical static families
        including stone/weighted pressure plates, utility fixtures, functional
        blocks, and redstone utility/control blocks, aquatic/coral static
        families, bamboo/honey/campfire utility static families,
        water plant/egg static families, flower/tall flower static families,
        fire/cocoa/creaking heart static families, glowstone/enchanting/beacon
        static families,
        ore/deepslate/nether colors, mineral/natural static block families, and
        the final accepted vanilla static states (mycelium, packed mud, nether
        brick fence, nether portal default NONE, stripped pale oak wood, and
        copper lantern weathering/waxed variants). Full mapColor catalog
        coverage is now pinned by the registry-wide falling-dust color test;
        biome-aware per-spawn BlockColors now use the same spawn-position
        world probe path as terrain particles. On-ground roll reset is now
        covered by collision-backed particle ticking.
      - native spawn resolution mirrors `TerrainParticle.createTerrainParticle`
        for definition-less `minecraft:block`, `minecraft:dust_pillar`, and
        `minecraft:block_crumble` submissions: air, `moving_piston`, and
        `shouldSpawnTerrainParticles=false` block states return no particle
        while preserving packet sample RNG consumption. Their spawn commands now
        carry the block state's terrain particle material sprite id, matching
        vanilla `TerrainParticle` construction through
        `BlockStateModelSet.getParticleMaterial(blockState).sprite()`.
        The same providers also carry `0.6 *
        BlockTintSource.colorAsTerrainParticle` for vanilla layer-0 block color
        sources; `minecraft:block_marker` remains sprite-only, matching vanilla
        `BlockMarker`.
        LevelEvent `addDestroyBlockEffect` / brush-complete `minecraft:block`
        commands now reuse the same terrain tint catalog with the event block
        position as the vanilla `TerrainParticle` tint position.
        `minecraft:block_marker` remains unfiltered, matching vanilla
        `BlockMarker.Provider`, and its spawn commands now carry the same
        block-state terrain particle material sprite id because vanilla
        `BlockMarker` also constructs with
        `BlockStateModelSet.getParticleMaterial(blockState).sprite()`.
      - renderer fixed `BreakingItemParticle` providers resolve their vanilla
        `ItemStackTemplate` sprite ids from local 26.1 assets:
        `minecraft:item_slime` -> `minecraft:item/slime_ball`,
        `minecraft:item_cobweb` -> `minecraft:block/cobweb`, and
        `minecraft:item_snowball` -> `minecraft:item/snowball`. The generic
        `minecraft:item` provider now decodes the `ItemStackTemplate`
        `DataComponentPatch` into the protocol component summary and resolves
        concrete GROUND particle material active-layer sprite ids through the
        native item runtime, including component-driven root item-model
        changes. The renderer uses random sprite selection for
        `BreakingItemParticle` providers.
      - renderer descriptor tests now cover the full vanilla 26.1
        `ParticleResources.registerProviders()` id list and reject any entry
        that falls back to generic `Particle`; remaining particle gaps are
        terrain/item atlas rendering, world-coupled collision/tint, or
        LevelEvent branches. The component-rich item-stack branch is done: the
        picked-up stack's already-decoded `DataComponentPatchSummary` rides the
        pickup channel as an opaque blob and the native bake reuses the
        dropped-item GROUND projection, so the pickup carried bake is
        byte-identical to the dropped-item bake for the same stack. The
        arrow/trident carried-model branch is done too: world projects
        `TakeItemEntityPickupProjectileModel` (normal/tipped/spectral arrow,
        trident with `ID_FOIL`) plus the extracted `yRot`/`xRot`, native carries
        it on the pickup command, and the renderer bakes
        `ArrowModel`/`TridentModel` (foil included) at the interpolated pickup
        position inside the `ITEM_PICKUP` group, closing every picked-up entity
        family vanilla's generic `EntityRenderState` submit actually consumes.
      - particle descriptors map `ElderGuardianParticle.Provider` to
        definition-less `minecraft:elder_guardian`, fixed lifetime `30`, zero
        aux/motion/gravity provider metadata, translucent
        `entityTranslucent(textures/entity/guardian/guardian_elder.png)`
        intent, and `ParticleRenderType.ELDER_GUARDIANS`; atlas billboard
        vertices are limited to `SINGLE_QUADS`, and actual model drawing is now
        covered by particle-target entity translucent submission using the
        vanilla elder guardian texture, alpha curve, full-bright light, no
        overlay, camera-facing rotation, `0.42553192` particle scale, translated
        model pose, and `2.35` elder baked-layer scale.
    - native dispatcher and offline probe recording/playback for
      `LevelEventHandler` portal travel local ambience:
      - event `1032`
      - `minecraft:block.portal.travel`
      - ambient source
      - non-spatial local sound command
    - canonical world/probe/native observed audio state for LevelEvent-derived
      positioned sounds:
      - fixed-pitch and randomized `LevelEventHandler` positioned sounds update
        `world.client_audio.last_sound`
      - vanilla `globalLevelEvent` sounds update `world.client_audio.last_sound`
        when a camera pose is available
      - LevelEvent-derived sound recording does not increment Sound-packet
        counters
      - positioned sound state and audio commands preserve vanilla
        `distanceDelay` metadata
      - Kira runtime queues distance-delayed positioned sounds using the
        vanilla `distanceDelay && distance^2 > 100` threshold and
        `floor(distance / 2)` tick delay
    - native dispatcher playback for vanilla `globalLevelEvent` sounds:
      - wither spawn event `1023`
      - ender dragon death event `1028`
      - end portal spawn event `1038`
      - camera-relative sound position two blocks toward the event
    - canonical world/probe/native state for `LevelEventHandler` jukebox
      start/stop:
      - event `1010` records active jukebox song registry id at block position
      - event `1011` stops the active jukebox song at that block position
      - active jukebox songs are cleared with client-level teardown
    - Kira-backed jukebox playback commands for `LevelEventHandler` jukebox
      start/stop:
      - event `1010` resolves vanilla 26.1 `JukeboxSongs.bootstrap` registry
        ids to sound events
      - registry data for `minecraft:jukebox_song` updates the runtime mapping
        for known vanilla song entry ids
      - play commands use `SoundSource.RECORDS`, volume `4.0`, pitch `1.0`,
        and the jukebox block center
      - stop commands target the matching jukebox block position without
        stopping all record sounds
    - The 2026-07-02 LevelEvent audio audit rechecked the local vanilla
      `LevelEventHandler` switch against current world/native tests and
      confirmed the covered set includes fixed-pitch local sounds, randomized
      local sounds, particle-before-sound ordering for potion break, dragon
      fireball, wax-on, bone-meal, vault, sculk-shrieker, and cobweb place,
      post-sound particle random-stream advancement for smoke, trial-spawner,
      sculk, simple particle-only, and block-face events, camera-relative
      `globalLevelEvent` sounds, portal-travel local ambience, and jukebox
      start/stop. P1-5 no longer tracks a LevelEvent-audio-specific open item.
    - Totem entity event `35` local positioned audio is covered separately from
      LevelEvent audio, including native resolver coverage for the
      `minecraft:item.totem.use` command.
    - 2026-07-04 determinism: the client sound-seed source no longer draws from
      wall-clock time. Vanilla seeds `Level.soundSeedGenerator` /
      `ClientLevel.random` from `RandomSupport.generateUniqueSeed()`
      (`SEED_UNIQUIFIER ^ System.nanoTime()`) and derives per-sound seeds via
      `RandomSource.nextLong()`. `bbb-world` now keeps the vanilla
      `LegacyRandomSource.nextLong()` advancement but seeds it from a fixed local
      seed on `ClientAudioState.sound_seed_random` (`WorldStore::next_sound_seed`),
      so the serializable world state stays deterministic (same packet stream ->
      same seeds/pitches). `LevelEventSoundRandomState::default()` likewise uses a
      fixed seed; native's level-event pitch stream is seeded from
      `world.next_sound_seed()`. The field is `#[serde(default)]` for snapshot
      compatibility. This removes the last `SystemTime::now()` determinism leak in
      `bbb-world`.
  - Full vanilla playback parity remains phase 7 work.

### Official 26.1 Resource-Pack Coverage

- Owner: `bbb-pack`
- Status: `partial`
- Next action:
  - Keep parser coverage aligned with official assets by running local vanilla
    coverage tests when changing pack loaders:
    - `loads_all_local_vanilla_atlases`
    - `loads_local_vanilla_item_model_catalog`
    - `loads_local_vanilla_item_registry`
    - `loads_local_vanilla_equipment_asset_catalog`
  - Add new parser support only when an official asset or resource-pack
    declaration fails those coverage tests or a focused resource-pack fixture.
  - Advance renderer/runtime consumption of parsed assets:
    - atlas mip animation metadata beyond current static sprite use
    - full item tint parity for dynamic sources in every item rendering path
    - equipment asset layers for future armor/equipment rendering
  - Keep resource-pack precedence/filter tests close to loaders.
- Evidence / boundary:
  - Local vanilla 26.1 parser coverage passes for:
    - atlases
    - item model declarations
    - item tint source declarations
    - item registry declarations
    - equipment assets
  - Loaders still reject malformed or unsupported custom resource-pack
    declarations intentionally.
  - Equipment assets under `assets/minecraft/equipment/*.json` are parsed into
    `EquipmentAssetCatalog`, including:
    - dyeable armor layers
    - `use_player_texture` elytra layers
    - wolf/horse/body armor layer declarations
  - Current audio use is covered by:
    - sounds
    - generated vanilla fallback
    - resource-pack filters

### Bundle Selected-Item Icon State

- Owner: `bbb-protocol` + `bbb-world` + `bbb-native` + `bbb-pack`
- Status: `partial`
- Next action:
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
    - route mouse-wheel selection through hovered local inventory and
      supported server-opened container slot hit-tests
    - clear selected bundle items before `QUICK_MOVE` and `SWAP`
      container clicks
  - The GUI item icon runtime:
    - evaluates `minecraft:bundle/has_selected_item`
    - resolves `minecraft:bundle/selected_item` from the selected template

### Item Model Range-Dispatch And Select Projection

- Owner: `bbb-protocol` + `bbb-native` + `bbb-pack`
- Status: `partial`
- Next action:
  - Thread any newly discovered ambient-context numeric `range_dispatch`
    properties through the icon resolver as that state becomes available to the
    GUI icon path.
  - Wire the remaining ambient-context `select` properties onto the same
    resolver:
    - `minecraft:context_entity_type` for any future non-GUI item consumer that
      gains a real living owner but is not routed through the current
      owner-backed generated held-item path
    - remaining `minecraft:local_time` coverage beyond the supported
      root/en plus selected English regional week-data ICU numeric/date-time /
      timezone-offset subset (which now includes `u` proleptic year,
      supported-English `Y` week-year, `G` era, `D` day-of-year, and `Q`/`q`
      quarter fields, root/en `M`/`L` month widths 1..=5, `g` Julian day,
      `F`
      day-of-week-in-month, supported-English `w`/`W` week numbers,
      supported-English `e`/`c` local weekdays, `A` milliseconds-in-day,
      root/en `a` AM/PM widths
      1..=5, plus localized-GMT `O` offsets, `Z`/`X`/`x` offset widths
      1..=5, short `z` zone
      abbreviations, `VV` zone IDs, and `VVV` exemplar cities, plus
      fixed/UTC long `z` names, with root/en `w` year-end boundary coverage,
      Sunday-first regional week coverage, and Monday/minimal-days=4 Jan 1
      previous-week-year / previous-month `W` coverage):
      full localized symbols
      and long-tail ICU pattern fields (locale-specific week data beyond the
      selected English regional groups, IANA long `z`, generic `v`, and
      one-/four-letter `V` widths)
  - Audit remaining item consumers that vanilla renders with a living owner and
    pass that owner context into the item resolver. `minecraft:main_hand` and
    `minecraft:context_entity_type` are now wired for owner-backed generated
    item attachments and GUI/HUD item icons that use the local-player owner
    context.
  - Extend `minecraft:component` select beyond the current decoded scalar /
    enum / simple literal text subset plus vanilla item/block default-name
    translatable keys as protocol/runtime summaries become available: complex
    object/list component values, style-sensitive Component equality,
    registry-backed component value codecs, custom/datapack component defaults
    beyond parsed vanilla item properties, custom datapack component value
    decoding, and components without a persistent codec remain follow-up.
  - Audit remaining non-GUI item consumers that can render component-bearing
    generated item stacks and pass dynamic registry keys where vanilla resolves
    registry-backed item-model properties. Dropped-item `GROUND`, item-frame
    `FIXED`, owner-backed third-person held generated items, and GUI/HUD icons
    now carry trim-material keys and direct enchantment keys; no-registry
    consumers still fall back.
  - Vanilla Quick Charge-modified crossbow charge duration is now wired for
    GUI/HUD local-player icons and owner-backed third-person generated held-item
    paths when the synced `minecraft:enchantment` registry identifies
    `minecraft:quick_charge`. The same registry projection now feeds direct-key
    and vanilla/static tag-key enchantment component predicates for GUI/HUD,
    dropped `GROUND`, item-frame `FIXED`, and owner-backed third-person
    generated held-item icons; custom enchantment effect parsing remains later
    registry/effect generalization.
  - Each plugs into the existing value-aware `RangeDispatch` / `Select`
    resolver by adding a value provider; no new selection machinery is required.
- Evidence / boundary:
  - `bbb-protocol` now decodes the `minecraft:custom_model_data` `floats` list
    (`CustomModelDataFloats`, bit-exact `Eq`) plus the flags/strings/colors
    lists, the `minecraft:block_state` property map, the
    `minecraft:charged_projectiles` item templates
    (`charged_projectiles_items`), and the `minecraft:trim` material holder
    reference id (`armor_trim_material_id`), and the
    `minecraft:consumable` `consume_seconds` value (`consumable`), plus the
    `minecraft:item_model` resource id and `minecraft:lodestone_tracker`
    target `GlobalPos` / `tracked` flag, so the
    `CustomModelDataProperty.getFloat(index)`,
    `CustomModelData.getBoolean(index)`,
    `CustomModelDataProperty.getString(index)`, `ItemBlockState.get`,
    `Charge.get`, `TrimMaterialProperty.get`,
    `DataComponents.ITEM_MODEL`, `DataComponents.LODESTONE_TRACKER`, and
    `ItemStack.getUseDuration(owner)` consumable input are preserved on the wire.
  - `bbb-native` resolves `minecraft:range_dispatch` item models with the exact
    vanilla `RangeSelectItemModel.update` selection:
    - `value = property.get(...) * scale`
    - `NaN` selects the fallback
    - `lastIndexLessOrEqual` over thresholds sorted ascending (inclusive
      `<=`; `-1` selects the fallback)
  - `bbb-native` resolves value-aware `minecraft:select` item models by matching
    the projected property value against each case's `when` values (vanilla
    `SelectItemModel`), falling back when no case matches.
  - `bbb-native` resolves `minecraft:display_context` item-model selects from
    vanilla `DisplayContext.get`, matching the current `ItemDisplayContext`
    serialized name against `when` values. GUI/HUD item icons pass `gui`,
    dropped-item generated layers pass `ground`, item-frame generated layers
    pass `fixed`, owner-backed third-person held generated layers pass their
    hand display context, and nested bundle selected items inherit the parent
    context. Tests pin texture selection across those contexts.
  - `bbb-native` resolves `minecraft:using_item` conditions for
    third-person/entity-owned generated item attachments by matching the
    submitted logical hand to vanilla `LivingEntity.getUseItem()` (`isUsingItem`
    plus `getUsedItemHand`); using an item in the other hand keeps the submitted
    stack on the false branch.
  - `bbb-native` resolves `minecraft:has_component` conditions with vanilla
    `HasComponent.get`: ordinary conditions use `ItemStack.has` so prototype
    defaults such as `minecraft:max_stack_size`, `minecraft:item_model`,
    `minecraft:rarity`, and common empty `minecraft:enchantments` count as
    present; vanilla `minecraft:enchanted_book` also counts its default empty
    `minecraft:stored_enchantments` component unless removed. The
    `ignore_default=true` path uses `ItemStack.hasNonDefault` / patch presence
    so added and removed component patches both select the true branch. Tests
    pin texture selection for default, added, and removed cases.
  - `bbb-native` resolves the root item model from the effective
    `DataComponents.ITEM_MODEL` value before evaluating the item-model tree:
    unpatched stacks use the vanilla default item id initialized by
    `Item.Properties.finalizeInitializer`, patched stacks select the overridden
    root model like `ItemModelResolver.appendItemLayers`, and removed component
    id 10 yields no item layers. The prototype item id remains the source for
    default `max_damage` / `max_stack_size` context values. Tests pin default,
    alternate, and removed behavior through texture UV selection.
  - Retained item display transforms are read from the same effective
    `DataComponents.ITEM_MODEL` root before applying the current display
    context, matching `ItemModelResolver.appendItemLayers` feeding
    `ModelRenderProperties.applyToLayer`. Dropped-item `GROUND`, item-frame
    `FIXED`, owner-backed held-item contexts, and HUD GUI block-item icons all
    query stack-aware transforms. Tests pin the default item root, a patched
    alternate root, and removed component id 10 returning no retained transform.
  - The context-free properties are projected from the item stack with vanilla
    math:
    - `minecraft:damage` — `Damage.get` (`damage / max_damage` normalized, or
      `clamp(damage, 0, max_damage)`), reading the component patch over the item
      prototype `max_damage` default
    - `minecraft:custom_model_data` — `CustomModelDataProperty.get`
      (`floats[index]`, or `0.0` when absent)
    - `minecraft:count` — `Count.get`, reading `ItemStack.getCount()` and the
      effective max stack size (component patch over item prototype), with
      `normalize` defaulting to `true`
    - `minecraft:bundle/fullness` — `BundleItem.getFullnessDisplay`, summing
      `BundleContents` weights: ordinary entries weigh `count / max_stack_size`,
      nested bundle entries add the fixed `1/16` bundle item weight even when
      empty, and stacks with non-empty `minecraft:bees` count as full-weight
      bundle entries
    - `minecraft:block_state` — `ItemBlockState.get`, reading the selected
      property from the stack's `minecraft:block_state` property map and falling
      back when the component/property is absent
    - `minecraft:custom_model_data` string select —
      `CustomModelDataProperty.getString(index)`, matching `strings[index]`
      against the case values and falling back when absent/out of range
    - `minecraft:custom_model_data` condition —
      conditional `CustomModelDataProperty.get`, matching
      `flags[index] == true`; missing, false, out-of-range, or removed
      `minecraft:custom_model_data` component id 17 selects the false branch
    - `minecraft:selected` — `IsSelected.get`, for HUD hotbar item icons:
      the local selected hotbar slot resolves the true branch and non-selected
      hotbar slots resolve false. Local inventory GUI hotbar slots also resolve
      true for slot `36 + selected_hotbar_slot`, while same-stack non-selected
      slots remain false instead of matching by item contents. Recognized
      server-opened container GUI hotbar slots now resolve from the vanilla menu
      slot layout's player hotbar start plus selected hotbar index.
    - `minecraft:carried` — `IsCarried.get`, projected as an explicit
      local-player carried-stack context bit in the native item icon resolver:
      ordinary HUD/GUI slot and generated recipe/offer item icons keep the
      false branch, while call sites that own the actual
      `containerMenu.getCarried()` stack can resolve the true branch without
      matching by item contents. GUI inventory cursor-carried items now use
      that true branch while projecting vanilla
      `AbstractContainerScreen.extractCarriedItem`'s non-dragging
      `mouseX - 8`, `mouseY - 8` floating item position; local quick-craft
      drag preview now applies vanilla `getQuickCraftPlaceCount` /
      `quickCraftingRemainder` to the floating cursor stack. Touchscreen
      split-stack and snapback animation remain broader GUI surface follow-up.
    - `minecraft:component` condition — `ComponentMatches.get`, currently for
      the `DataComponentPredicate` component-type / `AnyValue` branch plus the
      concrete `minecraft:damage` predicate. Native maps the AnyValue
      component id through the same decoded component table as `has_component`:
      default prototype components such as `minecraft:rarity` count as present,
      removed components select false, and non-default patched components such
      as `minecraft:enchantment_glint_override` select true regardless of their
      boolean payload. The damage predicate matches vanilla
      `DamagePredicate.matches` over the stack's `minecraft:damage` and
      `durability = max_damage - damage` bounds. Empty single-component
      predicates for `minecraft:bundle_contents`, `minecraft:container`,
      `minecraft:trim`, `minecraft:firework_explosion`, `minecraft:fireworks`,
      and `minecraft:jukebox_playable` also match the vanilla
      component-present branch. `minecraft:firework_explosion` shape / trail /
      twinkle constraints are decoded from the component and matched against
      vanilla `FireworkExplosionPredicate`; `minecraft:fireworks` decodes and
      matches `FireworksPredicate.flightDuration` `MinMaxBounds.Ints` plus
      `explosions.size` `CollectionPredicate.size` against the decoded
      explosions count. `minecraft:trim`
      direct registry-key or trim-material-tag constraints match the decoded
      `ArmorTrim.material()` holder id through dynamic trim-material registry
      keys and native trim-material tag catalog. `minecraft:jukebox_playable`
      now matches the optional `song` HolderSet against the decoded
      `JukeboxPlayable.song()` holder id through vanilla
      `JukeboxSongs.bootstrap` order, including direct vanilla registry keys and
      native jukebox-song tag entries. `minecraft:potion_contents` now matches
      vanilla `PotionsPredicate` HolderSets against the decoded
      `PotionContents.potion()` holder id through vanilla `Potions`
      registration order, including direct vanilla registry keys and native
      potion tag entries. `minecraft:writable_book_content` now matches decoded
      raw writable-book pages with vanilla `CollectionPredicate` `contains` /
      `count` / `size`; `minecraft:written_book_content` now matches decoded
      written-book raw title, author, `generation` `MinMaxBounds.Ints`,
      `resolved`, and plain-string plus simple literal
      `ComponentSerialization` text-object page collection predicates.
      `minecraft:villager/variant` now matches decoded 0-based
      `VillagerType` holder ids against direct registry-key or
      villager-type-tag HolderSets using the vanilla `VillagerType.bootstrap`
      registry-key order. `minecraft:attribute_modifiers` now preserves decoded
      modifier entries and matches direct plus bundle/container-nested
      `modifiers` collection predicates over direct registry-key or
      attribute-tag `attribute` HolderSets when `minecraft:attribute` registry
      keys are available, plus `id`, `amount`, `operation`, `slot`, and `size`
      / `contains` / `count`. Tool, sword, spear, humanoid armor, wolf armor,
      horse armor, nautilus armor, mace, and trident item-prototype default
      modifiers now feed the same direct plus bundle/container-nested predicate
      path unless the stack explicitly removes or overrides
      `ATTRIBUTE_MODIFIERS`.
      `minecraft:custom_data` now preserves decoded custom-data NBT compound
      summaries and matches direct plus bundle/container-nested predicates with
      vanilla `NbtUtils.compareNbt(..., true)` subset-compound and partial-list
      semantics for JSON-object and SNBT-string compound predicate values.
      `minecraft:bundle_contents`
      `items.size` constraints match vanilla `CollectionPredicate.size` against
      the decoded bundle item count, and `items.contains` / `count` now support
      vanilla `ItemPredicate` direct item-key or item-tag HolderSets,
      stack-count bounds, exact scalar/default `DataComponentMatchers`
      components, and patch-backed simple literal `minecraft:custom_name` /
      `minecraft:item_name` / `minecraft:lore` exact components plus
      `minecraft:unbreakable` Unit exact components and exact
      `minecraft:custom_data` compound components (JSON-object or SNBT-string
      expected values), plus exact `minecraft:potion_contents` components for
      direct potion keys, optional `custom_color`, ordered direct mob-effect
      `custom_effects` payloads including amplifier / duration / ambient /
      particles / icon / recursive hidden-effect details, and optional
      `custom_name`, plus exact `minecraft:writable_book_content`
      ordered `Filterable<String>` page lists with raw and optional filtered
      strings, plus exact `minecraft:written_book_content` components for raw
      / filtered title strings and ordered simple literal plus styled / extra /
      translated component page text summaries, plus
      exact `minecraft:firework_explosion` components for
      `shape`, ordered `colors`, ordered `fade_colors`, `has_trail`, and
      `has_twinkle`, plus exact `minecraft:fireworks` components for
      `flight_duration` and ordered explosion lists, plus nested partial
      `minecraft:damage`, plus exact `minecraft:jukebox_playable` components
      for direct vanilla jukebox-song keys and inline direct-song objects with
      direct or registry sound-event holders, description text summaries,
      `length_in_seconds`, and `comparator_output`, plus exact `minecraft:trim`
      components for direct trim-material registry keys, inline trim-material
      payloads with asset name / override armor assets / description text,
      direct vanilla trim-pattern keys, and inline trim-pattern payloads with
      asset id / description text / decal, plus exact `minecraft:enchantments` and
      `minecraft:stored_enchantments` components for direct enchantment
      registry-key maps, plus exact `minecraft:villager/variant` components
      for direct vanilla villager-type registry keys, plus exact
      `minecraft:lodestone_tracker` components for optional target `GlobalPos`
      and `tracked`, plus exact `minecraft:attribute_modifiers` components for
      ordered modifier lists with direct attribute registry keys, `id`,
      `amount`, `operation`, `slot`, and default / hidden / simple literal
      override display text summaries, including styled / extra component text,
      `minecraft:firework_explosion`, `minecraft:fireworks`, `minecraft:trim`,
      `minecraft:jukebox_playable`, `minecraft:potion_contents`,
      `minecraft:writable_book_content`, `minecraft:written_book_content`,
      `minecraft:villager/variant`, `minecraft:attribute_modifiers`,
      `minecraft:custom_data`, and data-component AnyValue predicates over
      decoded bundle entries.
      `minecraft:container` now decodes non-empty container entries and matches
      the same direct item-key / item-tag / stack-count / exact scalar component
      / nested partial damage, enchantments, stored-enchantments,
      firework-explosion, fireworks, trim, jukebox-playable, potion-contents,
      writable-book-content, written-book-content, villager-variant,
      attribute-modifiers, and AnyValue predicate collection subset.
      `minecraft:fireworks`
      `explosions.contains` / `count` predicates now match decoded explosion
      shape / trail / twinkle summaries. `minecraft:trim` direct vanilla
      registry-key or trim-pattern-tag constraints now match decoded
      `ArmorTrim.pattern()` holder ids through vanilla `TrimPatterns.bootstrap`
      order and native trim-pattern tag catalog.
      `minecraft:enchantments` and patch-backed
      `minecraft:stored_enchantments` match decoded enchantment levels and
      direct registry-key or enchantment-tag HolderSet predicates when the
      `minecraft:enchantment` registry keys and native enchantment tag catalog
      are available to the icon resolver; GUI/HUD, dropped `GROUND`, item-frame
      `FIXED`, and owner-backed third-person generated held-item paths now
      thread that registry context. Empty
      `minecraft:enchantments` predicate lists honor vanilla's default empty
      `ENCHANTMENTS` component unless id 13 is removed, and vanilla
      `minecraft:enchanted_book` contributes its item-specific default empty
      `STORED_ENCHANTMENTS` component unless id 42 is removed. Exact
      `minecraft:enchantments` and `minecraft:stored_enchantments` component
      maps now compare decoded holder ids and levels against direct
      enchantment registry keys when that dynamic registry is available.
      Exact `minecraft:villager/variant` components now compare decoded holder
      ids against direct vanilla villager-type registry keys. Exact
      `minecraft:lodestone_tracker` components now compare the decoded optional
      target `GlobalPos` and `tracked` flag. Exact
      `minecraft:attribute_modifiers` components now compare decoded ordered
      modifier entries against direct attribute registry keys, numeric amount,
      operation, slot, and default / hidden / simple literal plus styled /
      extra override display text summaries. Exact
      `minecraft:written_book_content` components now compare decoded raw /
      filtered title strings, author, generation, resolved, and ordered simple
      literal plus styled / extra / translated raw / filtered page text
      summaries. Remaining
      constrained predicate types (inline enchantment holder payloads / server
      datapack tag remaps, broader NBT scalar typing, remaining concrete partial
      predicates and complex exact component codecs beyond the already covered
      simple literal name / lore, Unit `unbreakable`, compound `custom_data`,
      filterable-page-list `writable_book_content`, full-field
      `firework_explosion` / `fireworks`, direct-key `jukebox_playable`, and
      direct-key `trim` exact components, potion / mob-effect datapack registry
      remaps for `potion_contents`, attribute modifier inline / datapack
      attribute holder payloads, full style-sensitive written-book page
      `ComponentSerialization` equality, datapack trim-material or trim-pattern
      registry-key remaps, datapack
      villager-type registry remaps, jukebox-song datapack registry remaps, and
      similar) remain follow-up.
    - `minecraft:charge_type` — `Charge.get` (`ROCKET` when any charged
      projectile is `minecraft:firework_rocket`, `ARROW` when charged otherwise,
      else `NONE`), using the native item registry to identify the projectile
    - `minecraft:trim_material` — `TrimMaterialProperty.get`, projecting the
      armor trim material holder id through the `minecraft:trim_material` dynamic
      registry (`bbb-world` registry keys threaded into the GUI icon resolver,
      dropped-item `GROUND` flat model path, and item-frame `FIXED` flat model
      path) to the material key (e.g. `minecraft:quartz`) matched against each case
    - `minecraft:main_hand` — `MainHand.get`, matching the owner's
      `HumanoidArm` serialized name (`left` / `right`) for third-person
      entity-owned generated item attachments and GUI/HUD item icons that can
      use the local-player owner context. This matches
      `GuiGraphicsExtractor.item`, which passes `minecraft.player` to
      `ItemModelResolver.updateForTopItem`; null-owner/fake item consumers still
      fall back.
    - `minecraft:context_dimension` — `ContextDimension.get`, matching the
      current `ClientLevel.dimension()` resource key for GUI/HUD item icons and
      owner-backed third-person generated held-item paths from `bbb-world`'s
      `WorldLevelInfo.dimension`; no-level item consumers still fall back.
    - `minecraft:context_entity_type` — `ContextEntityType.get`, matching the
      owner entity type resource key for GUI/HUD item icons. This mirrors
      `GuiGraphicsExtractor.item`, which passes `minecraft.player` as the owner,
      so the GUI/HUD context value is `minecraft:player`. Owner-backed generated
      held-item paths project the renderer entity kind to the vanilla entity
      type key before resolving generated item layers; tests pin player vs witch
      branch selection. Null-owner/fake item consumers still fall back.
    - `minecraft:view_entity` — `IsViewEntity.get`, for GUI/HUD local-player
      item icons in the normal camera==player path. This mirrors
      `GuiGraphicsExtractor.item`, which passes `minecraft.player` as owner;
      native threads an explicit view-entity bit rather than comparing by item
      contents or entity type. Spectator camera identity and non-GUI
      owner-backed item consumers remain follow-up.
    - `minecraft:extended_view` — `ExtendedView.get`, for GUI/HUD local-player
      item icons when either Shift key is down. Native threads
      `ClientInputState::shift_down()` into the item icon resolver and keeps
      vanilla's GUI display-context gate, so non-GUI consumers select the false
      branch even while Shift is down.
    - `minecraft:keybind_down` — `IsKeybindDown.get`, for GUI/HUD local-player
      item icons whose condition references native-tracked default key names.
      Native projects pressed movement/gameplay/inventory/multiplayer/misc/
      creative non-debug keys plus mouse attack/use/pick buttons into the item
      icon resolver, covering the vanilla `Options.keyMappings` defaults such
      as social interactions, quick actions, screenshot, perspective, fullscreen,
      GUI / spectator-shader toggles, creative toolbar activators,
      spectator-hotbar, hotbar 1-9, and the valid default-unbound
      smooth-camera / spectator-outlines key names as false under the vanilla
      default keymap. User-rebound/custom key mappings and F3/debug modifier
      combos remain follow-up.
    - `minecraft:fishing_rod/cast` — `FishingRodCast.get`, for GUI/HUD
      local-player selected hotbar main-hand fishing rods when a fishing bobber
      add-entity has owner data equal to the local player id. Non-selected
      hotbar slots and no-bobber paths select false; offhand / cursor /
      inventory identity and fishing-hook billboard / line rendering remain
      follow-up.
    - `minecraft:local_time` — `LocalTime.get`, formatting wall-clock time for
      the vanilla 26.1 chest/trapped-chest `MM-dd` selector plus a
      root/en plus selected English regional week-data ICU `SimpleDateFormat`
      subset (`y`/`u` year, supported-English `Y` week-year, `G` era text,
      `Q`/`q` quarter, root/en `M`/`L` month widths 1..=5, `d` day, `D`
      day-of-year, `g` Julian day, supported-English `w`/`W`
      week-of-year / week-of-month, `F` day-of-week-in-month,
      supported-English `E`/`e`/`c` weekdays, 24/12-hour
      `H`/`k`/`K`/`h`, `m`/`s`/`S`, `A` milliseconds-in-day, root/en `a`
      AM/PM widths 1..=5, `Z`/`X`/`x` offset fields through width 5,
      localized-GMT `O` offsets, short `z` zone abbreviations, `VV` zone IDs,
      `VVV` exemplar cities, fixed/UTC long `z` names, and quoted literals).
      Explicit `GMT`/UTC offset and IANA `time_zone` IDs use that zone; absent
      `time_zone` uses the system local timezone like vanilla.
      Tests pin GMT `12-25` selecting the Christmas branch, `12-27` selecting
      the fallback, cross-midnight `UTC+02:30` plus `Asia/Tokyo` date-time /
      weekday / AM-PM branches, UTC `X`/`x` zero-offset formatting,
      `uuuu-DDD-G` proleptic-year / day-of-year / era, `Q`/`q` quarter,
      `g` Julian day, root/en narrow `M`/`L` month symbols,
      localized-GMT `O`, `F` day-of-week-in-month, `A` milliseconds-in-day,
      root/en `a` narrow AM/PM, supported-English `Y` week-year, and
      supported-English `w`/`W`
      week plus `e`/`c` local weekday branches, a root/en `w` year-end boundary
      branch, a Sunday-first regional branch, and a Monday/minimal-days=4
      Jan 1 previous-week-year / previous-month `W` branch; an IANA-zone short
      `z` / `VV` / `VVV` branch plus fixed-offset `zzzz`; UTC and
      `UTC+02:30` offset branches now pin `Z`/`X`/`x` width 4/5 output as well.
    - `minecraft:time` — `Time.get`, for GUI/HUD item icons with a local-player
      owner and `ClientLevel` context. Native projects the `daytime` target
      from the overworld sun angle and `moon_phase` from the vanilla
      eight-phase `day_time / 24000` cycle, applies vanilla default
      `wobble=true` standard wobbler smoothing
      (`NeedleDirectionHelper.standardWobbler(0.9F)`), and then applies
      vanilla range-dispatch threshold selection. Tests pin no-level `0.0`
      fallback, overworld day-time and moon-phase texture selection, and a
      default-wobbled first-tick branch that raw non-wobbled target selection
      would miss. `source=random` uses a persistent per-property Java
      LCG-shaped random source; vanilla seeds it with a client-local unique
      seed, so native uses a deterministic local seed while preserving
      per-property advancement. Tests pin the random branch selecting a texture
      instead of falling back.
    - `minecraft:compass` — `CompassAngle.get`, for GUI/HUD item icons with a
      local-player owner and `ClientLevel` context targeting spawn, lodestone,
      or recovery. Native projects the default-spawn, `LodestoneTracker.target`,
      or local-player `lastDeathLocation` `GlobalPos`, validates it against the
      current dimension, computes vanilla's owner-position / visual-yaw
      rotation toward the block center, applies exact non-wobbled target
      rotation for `wobble=false`, applies default `wobble=true`
      `NeedleDirectionHelper` smoothing factor `0.8` for valid local-player
      targets, and then applies vanilla range-dispatch threshold selection.
      Tests pin no-pose `0.0` fallback, missing-component / missing-recovery
      threshold behavior, same-dimension spawn/lodestone/recovery texture
      selection, compass invalid-target threshold behavior, and a
      default-wobbled valid-target spawn texture-selection branch. No-target /
      invalid-target rotation now follows vanilla
      `getRandomlySpinningRotation`: `target=none` is parsed, each baked
      property has a no-target wobbler/random state, `wobble=true` updates once
      per game tick with factor `0.8`,
      `wobble=false` uses the non-wobbler random value, and the item-model
      seed hash is added before positive modulo. HUD hotbar icons pass
      vanilla-shaped `slot_index + 1` seeds. Tests pin `target=none` and
      cross-dimension spawn invalid-target branches selecting random-spin
      textures instead of the old fixed `0.0` fallback.
    - `minecraft:component` — `ComponentContents.get`, currently matching
      decoded persistent scalar / enum components with typed `when` values:
      `minecraft:max_stack_size`, `minecraft:max_damage`, `minecraft:damage`,
      `minecraft:item_model`, `minecraft:rarity`, and
      `minecraft:enchantment_glint_override`, plus `minecraft:map_id` from the
      synced `MapId` int wrapper and the RGB int wrappers
      `minecraft:dyed_color` / `minecraft:map_color`, plus simple literal
      JSON-string / `{"text": ...}` `minecraft:custom_name` components, plus
      `minecraft:item_name` simple literal patch values and vanilla item/block
      default translatable description keys from
      `Item.Properties.finalizeInitializer`. Native item icons project vanilla
      common defaults (`max_stack_size=64`, `item_model=<item id>`,
      `rarity=common`) and damageable item defaults (`damage=0`,
      `max_damage=<item default>`), and removed component ids suppress the
      selected value before case matching. Tests pin texture selection for
      string, numeric, boolean, resource-id, default, patched, explicit map-id,
      explicit color, literal custom-name string / text-object, item/block
      default / literal item-name string / text-object, and removed component
      cases.
    - `minecraft:cooldown` — `Cooldown.get`, matching the local player's
      `ItemCooldowns.getCooldownPercent(itemStack, 0.0F)` for GUI/HUD item
      icons. The item model property intentionally uses vanilla's `0.0F`
      partial tick, while the separate HUD cooldown overlay still uses render
      partial tick.
    - `minecraft:use_duration` — `UseDuration.get`, for GUI/HUD local-player
      item icons and owner-backed third-person generated held items whose stack
      is the active `LivingEntity.getUseItem()`, using the local / entity use
      tick counter as elapsed ticks (`remaining=false`, vanilla bow asset path)
      and reading vanilla `Consumable.consumeTicks()` (`consume_seconds * 20`
      truncated to int) for ordinary consumable stacks when `remaining=true`;
      tests also pin the 26.1 `EnderEyeItem.getUseDuration` override to `0`.
    - `minecraft:use_cycle` — `UseCycle.get`, for GUI/HUD local-player item
      icons using the active stack's remaining ticks modulo the declared
      positive `period` (vanilla brush asset path, 200 tick brush duration)
    - `minecraft:crossbow/pull` — `CrossbowPull.get`, for GUI/HUD local-player
      item icons and owner-backed third-person generated held-item paths using
      elapsed ticks divided by `CrossbowItem.getChargeDuration`: default 25
      ticks, or Quick Charge's vanilla `-0.25F` per level when the stack
      enchantment holder id resolves to `minecraft:quick_charge` through the
      synced `minecraft:enchantment` registry. Already charged crossbows still
      return `0.0`.
    - `minecraft:display_context` — `DisplayContext.get`, returning the
      serialized `ItemDisplayContext` currently used by the consumer (`gui`,
      `ground`, `fixed`, `thirdperson_righthand`, etc.) before case matching.
  - A value-aware `RangeDispatch` / `Select` is treated as a runtime condition so
    it is resolved per stack rather than collapsed at model-build time.
  - The trim-material registry keys are projected into the GUI icon path
    (`hud_item_icon_for_stack`), dropped-item generated model path, item-frame
    generated model path, and owner-backed third-person held generated item
    path; no-registry / null-context consumers still fall back to the untrimmed
    model.
  - `bbb-protocol` now preserves the `minecraft:bees` component occupant count
    (`DataComponents.BEES`, id 77) so bundle-fullness weight can distinguish
    beehive-like full-weight entries from ordinary stack-size weighted entries.
  - The GUI/HUD numeric `minecraft:compass` path now covers spawn, lodestone,
    recovery, and `none` targets: valid targets project owner-position / yaw
    against the current default spawn, `LodestoneTracker.target`, or
    local-player `lastDeathLocation`, default valid-target wobble is applied
    when requested by the model, and no-target / invalid-target cases use the
    vanilla random-spin branch instead of a fixed `0.0`.
    `minecraft:time` projects GUI/HUD `daytime` / `moon_phase` target values
    from world time, applies the default `wobble=true` standard wobbler, and
    advances per-property `source=random` state instead of falling back.
    `minecraft:local_time` resolves the vanilla chest/trapped-chest `MM-dd`
    selector and common root/en plus selected English regional week-data ICU
    date-time patterns from wall-clock time, including `y`/`u` year,
    supported-English `Y` week-year, `G` era, `D` day-of-year, `Q`/`q` quarter
    widths 1..=5 for root/en, `g` Julian day, `F` day-of-week-in-month,
    supported-English `w`/`W` week numbers and `e`/`c` local weekdays,
    fixed-offset / IANA
    `time_zone` IDs, `Z`/`X`/`x` offset fields, localized-GMT `O` offset
    widths, the root/en `w` year-end boundary, the Sunday-first regional
    branch, and the Monday/minimal-days=4 Jan 1 previous-week-year /
    previous-month `W` branch; full localized symbols and long-tail ICU
    pattern fields (locale-specific week data beyond the selected English
    regional groups, IANA long `z`, generic `v`, and one-/four-letter `V` widths) remain
    follow-up. Short `z` zone abbreviations, `VV` explicit zone
    IDs, `VVV` exemplar cities, and fixed/UTC long `z` names now resolve for
    explicit zones.
    GUI/HUD use-tick properties are wired for the local active stack,
    owner-backed third-person generated held-item paths use the entity render
    state's shared use tick counter, and both paths apply vanilla Quick
    Charge-modified crossbow charge duration when the enchantment registry is
    available. Ordinary and active-use first-person generated item stacks now
    render in the depth-cleared hand pass, and the active-use path passes the
    local active-use tick context into `minecraft:use_duration` /
    `minecraft:use_cycle` item-model range-dispatch properties.
    `minecraft:main_hand` and `minecraft:context_entity_type` still fall back on
    native item consumers that do not pass a `LivingEntity` owner, such as
    fake/null-owner item surfaces. `minecraft:custom_model_data` condition is
    wired for the stack-local `flags` list, and `minecraft:selected` is wired
    for HUD hotbar selected-slot icons plus recognized local and server-opened
    GUI hotbar slots.
    `minecraft:view_entity` is wired for
    GUI/HUD local-player icons in the normal camera==player path, and
    `minecraft:extended_view` is wired for Shift-held GUI/HUD local-player
    icons while retaining vanilla's GUI display-context gate. `minecraft:keybind_down`
    is wired for native-tracked default non-debug `Options.keyMappings` names,
    including vanilla default-unbound names that resolve false until user key
    rebinding exists.
    `minecraft:fishing_rod/cast` is wired for GUI/HUD selected hotbar
    main-hand fishing rods while a local-player-owned fishing bobber exists.
    `minecraft:carried` is wired as an explicit resolver context bit for the
    GUI inventory cursor-carried item path, which now renders the world cursor
    stack as a floating GUI item at vanilla's non-dragging cursor offset and
    applies the vanilla quick-craft remainder count while drag-distributing
    across multiple slots. Touchscreen split-stack and snapback animation are
    not-needed until a touchscreen input mode exists: vanilla gates both
    entirely behind `Options.touchscreen` (`AbstractContainerScreen.java:336,
    342, 411, 489`; snapback interpolation `:146-158`), and bbb has no touch
    input mode or plan, so implementing them now would be dead code
    (adjudicated 2026-07-05).
    `minecraft:component` is wired for the scalar / enum / simple literal
    custom-name component select subset listed above, and the condition form
    covers component-type / AnyValue,
    `minecraft:damage`, empty single-component predicates, direct-key
    enchantment HolderSet predicates when the synced enchantment registry is
    available, and direct plus nested bundle/container writable/written-book
    predicates for decoded raw string fields/pages, plus direct and
    bundle/container-nested villager variant predicates for direct registry-key
    or villager-type-tag HolderSets, plus direct and bundle/container-nested
    attribute modifier predicates for decoded direct registry-key or
    attribute-tag attribute / id / amount / operation / slot entries when the
    synced attribute registry is available, plus direct and
    bundle/container-nested custom-data NBT compound predicates for JSON-object
    and SNBT-string compound expected values, plus patch-backed simple literal
    custom-name / item-name / lore exact components and `unbreakable` Unit exact
    components, plus exact custom-data compound components; broader
    component-codec and remaining constrained `DataComponentPredicate` parity
    remains the documented follow-up.

### Native Input, Movement, Interaction, Inventory, And Command Flows

- Owner: `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world`
- Status: `partial`
- Next action:
  - Movement: extend the native fixed-20Hz local movement / AABB collision /
    still-water-lava fluid slice toward remaining vanilla survival physics,
    voxel collision shapes, and fluid presentation nuance.
  - Block destroy: close remaining destroy-speed and destroy-profile gaps
    outside the mechanically parsed `Blocks.java` declarations.
  - Commands: continue adding focused command queue and encode tests for
    inventory, interaction, chat, command, and sign editing. Sign editing now
    includes renderer presentation for the vanilla sign edit screen; clipboard
    copy/cut/paste parity remains in the detailed ledger.
  - Inventory: implement remaining rich tooltip behavior (styled component
    runs — bold/colour/underline/strikethrough/shadow/italic-shear/obfuscated
    — now render live in tooltips and labels, see Vanilla Font Provider
    Coverage; remaining: bidirectional text shaping only); the official
    tooltip background/frame nine-slice sprites
    are now drawn.
  - Completion requires full vanilla movement and these flows to work
    through encoded serverbound packets end to end.
- Evidence / boundary:
  - Movement, block destroy, commands, and inventory each have a native
    implementation covering the currently supported vanilla-shaped behavior:
    serverbound movement/physics projection, sprint/destroy-speed derivation
    from world-owned predicates, command-queue packet encoding, and
    container/tooltip rendering for the local and opened containers. Sign edit
    renderer presentation now covers plain sign PIP previews, hanging-sign GUI
    backgrounds, vanilla titles, line placement, cursor blink, and selection
    overlays.
- Detailed per-slice history: docs/unsupported/native-input-movement-interaction-inventory-and-command-flows.md

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
- When a row's body grows past 800 lines, relocate its per-slice detail
  bullets as-is into `docs/unsupported/<kebab-case-row-name>.md` (starting
  with `# <Row Name> — detailed ledger`) and keep the row itself to owner,
  status, a concise next-action/evidence summary, and a
  `Detailed per-slice history:` link line.
