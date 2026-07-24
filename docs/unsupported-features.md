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
  - **Deferred** — a real gap, intentionally not scheduled. Carries the
    condition that turns it back into a todo.
  - **Not needed** — vanilla 26.1 has no such behavior, or bbb has no surface
    for it. Carries the vanilla evidence.
- This file does not overflow into sibling files. If it grows, the gap list grew.

---

# Todo

## Runtime performance parity and stable frame tails

Owner: `bbb-native` + `bbb-renderer` + `bbb-world`

- Reuse the remaining heap-heavy non-entity `RendererFrame` extraction owners
  and revision-cache stable HUD/block-entity/debug projections, and replace the
  exact scissored GUI triangle-clipping fallback for floating/stats items,
  without caching partial-tick or animation-dependent state.
- Measure and reduce the remaining dynamic item-model property-selection,
  per-frame instance upload, draw/pipeline state churn, and driver-deferred
  resident-release tails. Any change must preserve exact phase/translucent
  order and the explicit bounded-owner fallback rather than dropping visible
  instances.
- Narrow Guardian/EndCrystal/Shulker specialized culling to their exact
  vanilla beam/interpolation/metadata boxes, and add section-granular compiled
  visibility; follow `LevelRenderer.extractVisibleEntities` and the registered
  renderer overrides without a parallel entity-type table.
- Complete equal-load official Java 26.1 CPU p50/p95/p99, GPU-frame, 1%-low,
  and chunk-update comparison captures; report any losing scene by measured
  owner instead of reducing visible work.

## Protocol coverage for remaining required 26.1 packet families

Owner: `bbb-protocol`

- Audit remaining play-family field order, nullability, enum ordinals, and
  serverbound encoders against
  `<MC_CODE_ROOT>/sources/26.1/`, adding focused encode/decode tests with each
  packet slice, including strict unit-payload, terminal packet/component roots,
  remaining bounded strings/byte arrays, typed resource-id normalization,
  bounded map count/key/value, untrusted URI validation and
  NBT-root/depth/count boundaries;
  keep handwritten handler coverage aligned with the generated packet catalog
  as unsupported families are implemented.
- Use `docs/packet-coverage-26.1.json` as the complete 256-entry inventory. Every
  remaining `partial` cell must carry implementation and semantic-test
  evidence through the checked-in sparse overrides; catalog presence or a
  generic dispatch arm alone is not completion.
- Use `docs/item-component-coverage-26.1.json` as the complete 110-entry
  ItemStack component semantic inventory. Each `partial` cell must close
  through its sparse evidence override; consuming the right wire length or
  retaining only a component type id does not establish typed decode,
  canonical retention, re-encoding, persistent projection, or a consumer.
- Thread the world/session owner into generic packet Components whose nested
  player, dialog-holder, or show-item payloads require registry context, while
  preserving an explicit unknown result in context-free callers.
- Preserve or explicitly reject lone-surrogate `\u`/`\U`/`\N{name}`
  SNBT escapes instead of passing them through the current UTF-8-backed canonical
  string state; follow `TagParser` and the Java UTF-16 string model.
- Extend byte-denominated stack budgets to the remaining recursive untrusted
  decode paths (component trees other than nested ItemStack templates, SNBT,
  and holder sets) the way the dialog depth-64 and ItemStack-template depth-32
  tests pin their reference walks; a depth constant alone does not bound stack
  use across build profiles.
- Then cover the remaining play, movement, inventory, chat and interaction
  families end to end.
## Vanilla Java generator frontend migration

Owner: `scripts/` + each generated catalog's semantic crate

- Complete the remaining cross-crate ordinal/serializer-id and entity
  renderer-class membership discovery audit.
  Migrate each homogeneous family through schema-owned accessors and generator-emitted reverse guards.
  The 155 explicit
  `EntityRenderers` provider registrations (PLAYER/MANNEQUIN use the separate avatar
  path), their static-full-block-light behavior, and exact `NoopRenderer` model suppression
  plus `AbstractFelineModel` / `AbstractHorseRenderer` hierarchy memberships and the
  `MobRenderer` + inherited/direct `ItemInHandLayer` intersection, along with inherited
  `extractRenderState` Mob-aggressive, unhappy-counter and spell-casting projections, are now
  source-generated. `ThrownItemRenderer` registration membership and constructor sprite scale
  must likewise remain renderer-generated; native billboard/model suppression must not restore a
  parallel entity-type/scale table. `LivingEntityRenderer.isUpsideDownName` membership must remain
  renderer-generated and shared by player-profile and non-player custom-name projection. Native
  entity-model suppression for `FallingBlockRenderer`, `FireworkEntityRenderer`, `ItemFrameRenderer`,
  `ItemEntityRenderer`, `LightningBoltRenderer`, `OminousItemSpawnerRenderer`, and `TntRenderer`
  must remain derived from generated provider registrations rather than an entity-type switch.
  `ItemFrameRenderer`'s glow-frame render-state membership and minimum block-light clamp must remain
  extracted together; world/native consumers must not restore the GlowItemFrame type id or literal
  clamp as parallel facts. Sheep head-eat pose membership must remain the intersection of both
  `SheepRenderer.extractRenderState` head-eat assignments rather than a native Sheep type gate.
  Zombie underwater
  conversion shaking must remain derived from the registered renderer hierarchy's
  `extractRenderState` assignment rather than a Zombie/Husk/Drowned type switch. Emit every new
  generated-catalog consumer guard or migrated generated-catalog consumer guard
  from that catalog's generator rather than hand-editing its checked-in output.
  Generated-catalog consumers must import the semantic owner directly; remove
  compatibility lookup functions whose only behavior is forwarding an
  id/resource lookup across an unrelated crate boundary, and have the owner
  generator reject those bridges from production sources.
  Native, world, and net semantic test setup must consume generated entity-type constants rather
  than restoring numeric catalog ids; unknown-id fixtures derive the first invalid id from the
  generated catalog length. The owner-generated reverse guard excludes generated schema/renderer
  catalogs and separately owned protocol wire samples.
  Semantic block-entity packet/state fixtures and item-component BlockEntityData summaries must
  likewise use the generated BlockEntityType constants for their actual sign, banner, spawner,
  command-block, decorated-pot, end-gateway, jukebox, piston, or vault role instead of retaining
  numeric placeholders; the core-registry generator owns the reverse guard.
  Semantic OpenScreen, container-state, input, and HUD fixtures must use the generated `MenuType`
  constants for their fixed built-in menu role; unknown menu ids derive the first invalid catalog
  index. The gameplay-id generator rejects numeric `menu_type_id` construction and comparison
  across semantic consumers while leaving protocol wire samples under the packet codec owner.
  Semantic particle packet/state/runtime/renderer fixtures must use generated ParticleType
  constants or the generated resource-id lookup instead of numeric catalog ids; unknown ids derive
  the first invalid catalog index, while the non-Vanilla item-pickup sentinel remains explicitly
  named. The core-registry generator owns the reverse guard and leaves protocol wire samples under
  their packet-codec owner.
  Dialog component predicate, item-patch entry/reference, and patch-summary fixtures must use
  generated DataComponentType and DataComponentPredicateType constants that match their resource
  ids; the core-registry generator owns both named predicate constants and the reverse guard over
  this semantic owner.
  Semantic tropical-fish pattern item-component fixtures must use the generated sparse packed-id
  constants, and their base/pattern colors must use generated DyeColor value constants; the
  gameplay-id generator owns both reverse guards across world, item-model, native, net, and
  renderer consumers while protocol packet codec samples remain separately owned. Unknown-color
  fixtures derive the first invalid id from the generated catalog length.
  Other semantic DyeColor consumers, including dye/base-color components and banner pattern
  layers, must use the same generated value constants and are covered by that generator guard.
  Built-in enum component maps must pair generated DataComponentType keys with the generated enum
  value constants on both tuple sides; the gameplay-id generator guards semantic fixtures.
  Known vanilla recipe-book category fixtures must use the generated `RecipeBookCategories`
  constants in protocol, world, net, control, and native consumers; the recipe catalog generator
  rejects numeric field construction and semantic comparisons while leaving raw wire payload
  samples and intentionally open server-defined category ids under their existing owners.
  Semantic recipe-display type projections must likewise use the generated `RecipeDisplays`
  constants; the same generator rejects numeric `recipe_display_type_id` construction and
  comparison outside its generated owner.
  Known vanilla slot-display branches and semantic fixtures must use the generated
  `SlotDisplays.bootstrap` constants across protocol, world, net, control, and native consumers;
  the recipe catalog generator rejects numeric `display_type_id` construction, matching, and
  comparison while leaving raw wire payload samples under the protocol codec owner.
  Heightmap sampler membership, semantic packet/state fixtures, queries, and comparisons must
  consume the generated `Heightmap.Types` catalog and named ids; the gameplay-id generator rejects
  numeric kind references outside protocol-owned raw codec samples.
  Command-tree semantic fixtures must use constants generated from
  `ArgumentTypeInfos.bootstrap` for every `CommandArgumentParser.type_id`; the dedicated command
  argument generator owns the public 57-entry catalog and reverse guard, while protocol decoder
  wire samples remain separately owned.
  AwardStats semantic fixtures, queries, and control projections must use the named StatType ids
  generated from `Stats.java`; unknown holders derive the first invalid catalog id, while custom
  registry-order fixtures derive ids from their explicit entry order. The gameplay-id generator
  guards world, net, control, and native consumers; protocol wire samples remain separately owned.
  All `bbb-native` semantic consumers must use generated entity-data serializer constants rather
  than numeric fields or local aliases across presentation, runtime, event, and input fixtures.
  All `bbb-world` semantic consumers must follow the same rule across inline tests and test
  submodules, as must `bbb-net` event/probe fixtures; protocol wire samples remain separately
  owned evidence. The generator-emitted reverse guard must reject numeric serializer construction,
  assignment, and comparison even when formatting splits a field access across lines. Numeric
  `serializer_id` fields are confined to the 18 audited raw-decoder
  assertions in `bbb-protocol/src/packets/entities/tests.rs`, identified by its wire-sample marker.
  Attribute-modifier semantic consumers and fixtures must likewise use the generated
  `AttributeModifier.Operation` constants; the generator guard must cover numeric construction,
  assignment, comparison, optional wrapper forms, and fixture helper arguments outside separately
  owned protocol wire samples. Dialog attribute-modifier fixtures must also use the generated
  EquipmentSlotGroup and display-type ids; the same generator guard owns their construction and
  comparison paths. Custom registry-order fixtures must derive their holder ids from the explicit
  key order instead of mixing those ids with built-in constants.
  `UpdateAttributes` semantic snapshots in net, world entity/level, and native runtime must use
  generated Attribute registry ids; custom registry-order fixtures stay explicitly scoped to
  their resolver tests rather than being mistaken for built-in holder ids.
  Native component serialization and world dialog component fixtures that use the built-in
  Attribute registry must follow the same generated-id rule; explicit resolver reorder/unknown-id
  fixtures remain outside that built-in guard.
  Built-in mob-effect holder construction, lookup, assignment, and dialog projection fixtures must
  likewise use the generated MobEffect registry constants; unknown-holder fixtures derive the
  first invalid id from the generated catalog length, while protocol wire samples remain separately
  owned evidence.
  Baby-capable entity membership and the per-family wire index/default must remain derived from
  the generated `DATA_BABY_ID` accessor hierarchy rather than an entity-type table.
  Zoglin's independently declared `DATA_BABY_ID` must use that nearest-owner resolver in native
  model/debug consumers and semantic fixtures; they must not restore fixed Zoglin accessor/default
  aliases or substitute the different AgeableMob wire index.
  Zombie baby/special-type/drowned-conversion membership, inherited wire indices and typed
  defaults must remain derived together from generated `Zombie.DATA_BABY_ID`,
  `DATA_SPECIAL_TYPE_ID` and `DATA_DROWNED_CONVERSION_ID`; native model, renderer shake and F3+I
  save-debug consumers must not restore fixed Zombie accessor/default aliases.
  ZombieNautilus variant membership, wire index and registry-resource virtual default must remain
  derived from generated `ZombieNautilus.DATA_VARIANT_ID`; native model/ready-texture/F3+I
  consumers and fixtures must not restore fixed accessor/default aliases. Session holder order and
  typed variant payload decoding remain owned by the dynamic `RegistryAccess` work above.
  Frog variant membership, wire index and registry-resource virtual default must likewise remain
  derived from generated `Frog.DATA_VARIANT_ID`; native model/ready-texture/F3+I consumers and
  fixtures must not restore fixed accessor/default aliases. Frog tongue-target entity resolution is
  separate unfinished metadata behavior and must use generated `DATA_TONGUE_TARGET_ID` when added.
  Goat and PolarBear specialized bounds must query that generated accessor instead of restoring
  fixed `AgeableMob.DATA_BABY_ID` aliases; their vanilla 0.55/0.5 age scales and Goat long-jump /
  PolarBear standing composition remain dimensions-owned behavior.
  Per-entity metadata accessor-id consumers must likewise query the generated schema hierarchy;
  in particular, villager model projection must resolve `DATA_VILLAGER_DATA` there rather than
  restoring a Villager/ZombieVillager type-to-index switch, and Skeleton freeze-conversion shake
  plus Strider suffocating and ZombieVillager curing-conversion shake must resolve their generated
  boolean accessors there rather than entity-type branches. Piglin/PiglinBrute/Hoglin conversion
  shake must likewise resolve the nearest generated `DATA_IMMUNE_TO_ZOMBIFICATION` accessor rather
  than split class membership and Hoglin into parallel dispatch arms.
  Spellcaster render-state projection must intersect generated renderer assignment with the
  generated `SpellcasterIllager.DATA_SPELL_CASTING_ID` accessor; it must not restore a fixed byte
  slot/default or substitute a parallel Evoker/Illusioner type predicate.
  Villager unhappy render-state projection must intersect generated renderer assignment with the
  generated `AbstractVillager.DATA_UNHAPPY_COUNTER` accessor; it must not restore a fixed int
  slot/default or confuse Panda's separate `UNHAPPY_COUNTER` with AbstractVillager membership.
  EndCrystal beam-target/show-bottom membership, wire indices and typed defaults must remain
  derived together from generated `DATA_BEAM_TARGET` and `DATA_SHOW_BOTTOM`; world beam, native
  model/debug consumers and semantic fixtures must not restore fixed accessor/default aliases or a
  parallel crystal type gate. EnderDragon's nearby-crystal search remains a separate runtime-instance
  type check.
  Ghast charging texture membership, wire index and typed default must remain derived from
  generated `Ghast.DATA_IS_CHARGING`; HappyGhast leash-holder/stays-still plus inherited baby
  collision state must use generated accessors for `IS_LEASH_HOLDER`, `STAYS_STILL`, and
  `AgeableMob.DATA_BABY_ID`. Native model and world collision consumers/fixtures must not restore
  fixed accessor/default aliases or read same-index booleans from another entity family.
  GlowSquid dark-ticks light/debug membership, wire index and typed default must remain derived
  from generated `GlowSquid.DATA_DARK_TICKS_REMAINING`; native consumers must not restore fixed
  accessor/default aliases or a separate GlowSquid type gate for renderer light projection.
  Goat screaming/left-horn/right-horn membership, wire indices and typed defaults must remain
  derived together from generated `DATA_IS_SCREAMING_GOAT`, `DATA_HAS_LEFT_HORN` and
  `DATA_HAS_RIGHT_HORN`; native model/debug consumers and semantic fixtures must not restore fixed
  accessor/default aliases.
  Turtle has-egg/laying-egg membership, wire indices and typed defaults must remain derived together
  from generated `HAS_EGG` and `LAYING_EGG`; native model/debug consumers and semantic fixtures must
  not restore fixed accessor/default aliases or apply either field to another boolean family.
  WitherSkull dangerous texture/debug membership, wire index and typed default must remain derived
  from generated `WitherSkull.DATA_DANGEROUS`; native consumers must not restore fixed
  accessor/default aliases or apply the field to another hurting-projectile family.
  HangingEntity direction membership, inherited wire index and typed default must remain derived
  from generated `DATA_DIRECTION`; ItemFrame item/rotation membership, wire indices and rotation
  default must remain derived together from generated `DATA_ITEM`/`DATA_ROTATION`. World bounds,
  center and render-state projection plus native ItemFrame/Painting debug consumers, local-save and
  map-frame fixtures must not restore fixed accessor/default aliases. Missing direction metadata
  must continue falling back to the add-entity packet `data`, matching vanilla recreation behavior.
  Painting variant metadata membership and wire index must remain derived from generated
  `DATA_PAINTING_VARIANT_ID`, including validation of its registry-any virtual default. World
  metadata projection, native debug and semantic fixtures must not restore a fixed accessor alias
  or apply the Painting holder projection to another entity family that reuses slot 9.
  Spectator camera post-effect selection must remain generated from
  `GameRenderer.checkEntityPostEffect` pattern arms plus the entity class hierarchy; native debug
  projection must not restore a Creeper/Spider/Enderman type switch or resource-id literals. This
  only owns selection metadata; executing the post chains remains in the renderer queue below.
  AbstractMinecart custom-display-block/display-offset membership and wire indices must remain
  derived together from generated `DATA_ID_CUSTOM_DISPLAY_BLOCK` and `DATA_ID_DISPLAY_OFFSET`;
  per-minecart offset defaults must remain generated from the nearest `getDefaultDisplayOffset`
  class override. World render and native save projection must not restore fixed accessor aliases,
  Chest/Hopper exception arms or standalone virtual-default constants.
  FurnaceMinecart fuel membership, wire index and default must remain derived from generated
  `MinecartFurnace.DATA_ID_FUEL`; default display-block projection must not restore a furnace type
  predicate or fixed accessor/default aliases, and semantic fixtures must use the same accessor.
  PrimedTnt fuse/block-state membership, wire indices and typed defaults must remain derived
  together from generated `DATA_FUSE_ID` and `DATA_BLOCK_STATE_ID`; model, fuse interpolation,
  smoke, native F3+I and semantic fixtures must not restore a TNT type predicate or fixed
  accessor/default aliases.
  FireworkRocket item/attachment/shot-angle membership, wire indices and typed defaults must
  remain derived together from generated `DATA_ID_FIREWORKS_ITEM`, `DATA_ATTACHED_TO_TARGET` and
  `DATA_SHOT_AT_ANGLE`; world item/trail/explosion projection, native F3+I and semantic fixtures
  must not restore a FireworkRocket type predicate or fixed accessor/default aliases.
  ItemEntity item membership, ITEM_STACK wire index and empty default must remain derived from
  generated `ItemEntity.DATA_ITEM`; world render/pickup/mutation and native bake fixtures must not
  restore fixed slot aliases, while thrown-item projectiles resolve their own generated accessor.
  OminousItemSpawner item membership, ITEM_STACK wire index and empty default must remain derived
  from generated `DATA_ITEM`; world/native item-cluster projection and semantic fixtures must not
  restore a type predicate or fixed accessor alias, or read another entity family's slot-8 field.
  ExperienceOrb value membership, wire index and typed default must remain derived from generated
  `ExperienceOrb.DATA_VALUE`; world pickup-icon projection, native F3+I and semantic fixtures must
  not restore fixed accessor/default aliases or read the same wire slot from another entity family.
  Living-entity scale sanitization must remain generated from the nearest `sanitizeScale`
  override, including HappyGhast/Shulker maximum clamps and EnderDragon's fixed 1.0 result;
  dimensions/render projection must not restore a type switch or local cap constants.
  Wither invulnerability/render membership, wire index and default must remain derived from the
  generated `WitherBoss.DATA_ID_INV` accessor; world/native debug consumers and fixtures must not
  restore a Wither type predicate or fixed accessor/default aliases.
  EnderMan carried-block/creepy render membership, wire indices and defaults must remain derived
  together from generated `DATA_CARRY_STATE`/`DATA_CREEPY`; world/native projection and item-model
  fixtures must not restore an EnderMan type predicate or fixed accessor/default aliases.
  Bat resting/debug membership, wire index, default and named resting mask must remain derived from
  generated `Bat.DATA_ID_FLAGS`; world projection and native debug must not restore a Bat type
  predicate, fixed accessor/default aliases or a hand-copied flag mask.
  Bee rolling/stinger/nectar/anger/debug membership, wire indices, typed defaults and named masks
  must remain derived together from generated `Bee.DATA_FLAGS_ID`, `FLAG_ROLL`, `FLAG_HAS_STUNG`,
  `FLAG_HAS_NECTAR` and `DATA_ANGER_END_TIME`; world/native consumers and fixtures must not restore
  a Bee type predicate, fixed accessor/default aliases or hand-copied flag masks.
  Dolphin got-fish/moistness debug membership, wire indices and typed defaults must remain derived
  together from generated `Dolphin.GOT_FISH` and `MOISTNESS_LEVEL`; native F3+I and its fixture must
  not restore fixed accessor/default aliases or read same-index metadata from another family.
  Fox variant/pose/debug membership, wire indices, typed defaults, trusted-entity slots and seven
  named masks must remain derived together from generated `Fox.DATA_TYPE_ID`, `DATA_FLAGS_ID`,
  `DATA_TRUSTED_ID_0/1` and `FLAG_*`; world/native consumers and fixtures must not restore a Fox
  type predicate, fixed accessor/default aliases, trusted-slot ids or hand-copied flag masks, and
  invalid variant ids must continue following vanilla ZERO fallback.
  Vex charging membership, wire index, default and named charging mask must remain derived from
  generated `Vex.DATA_FLAGS_ID`; world/native render consumers and fixtures must not restore a Vex
  type predicate, fixed accessor/default aliases or a hand-copied flag mask.
  Sniffer state and seed-drop tick membership, wire indices and typed defaults must remain derived
  from generated `DATA_STATE` and `DATA_DROP_SEED_AT_TICK`; animation/bounds and native debug
  fixtures must not restore fixed accessor/default aliases.
  Panda counter/gene/pose/sneeze membership, wire indices, typed defaults and four named masks must
  remain derived together from generated `UNHAPPY_COUNTER`, `SNEEZE_COUNTER`, `EAT_COUNTER`,
  `MAIN_GENE_ID`, `HIDDEN_GENE_ID`, `DATA_ID_FLAGS` and `FLAG_*`; world/native consumers and
  fixtures must not restore fixed accessor/default aliases or hand-copied flag masks.
  AbstractHorse pose/tame/bred/debug membership, wire index, typed default and five named masks must
  remain derived together from generated `AbstractHorse.DATA_ID_FLAGS` and `FLAG_*`; animation,
  mount inventory, native debug and their fixtures must not restore fixed accessor/default aliases
  or hand-copied flag masks.
  Horse variant, AbstractChestedHorse chest and Llama strength/variant membership, wire indices and
  typed defaults must remain derived from their generated accessor bundles; native model/debug
  consumers and semantic fixtures must not restore fixed accessor/default aliases.
  Chicken/Cow/Pig variant and sound-variant membership, wire indices and registry-resource virtual
  defaults must remain derived together from the generated farm-animal bundle; world/native
  consumers and semantic fixtures must not restore fixed accessor/default aliases.
  Cat/Wolf variant and sound-variant membership, wire indices and registry-resource virtual defaults
  must remain derived together from the generated companion bundle; native dynamic-texture and
  world audio consumers/fixtures must not restore fixed accessor/default aliases.
  TamableAnimal sitting/tame/owner membership, inherited wire indices, typed defaults and two named
  masks must remain derived together from generated `TamableAnimal.DATA_FLAGS_ID`,
  `DATA_OWNERUUID_ID` and their `FLAG_*`; world/native presentation, mount inventory, native debug
  and their fixtures must not restore fixed accessor/default aliases or hand-copied masks.
  AbstractArrow flags, pierce-level and in-ground membership, inherited wire indices, defaults and
  crit/no-physics named masks must remain derived together from generated `AbstractArrow.ID_FLAGS`,
  `FLAG_CRIT`, `FLAG_NOPHYSICS`, `PIERCE_LEVEL` and `IN_GROUND`;
  Arrow effect color plus ThrownTrident loyalty/foil identity must likewise come from generated
  `Arrow.ID_EFFECT_COLOR`, `ThrownTrident.ID_LOYALTY` and `ThrownTrident.ID_FOIL`; animation,
  projectile lifecycle, pickup/particle, native render and debug consumers must not restore fixed
  accessor/default/mask aliases or confuse sibling ownership.
  ArmorStand client flags, four named masks and six pose accessors, wire indices and typed defaults
  must remain derived together from the generated `ArmorStand` schema; world bounds/pickability,
  native model/debug and their fixtures must not restore fixed metadata aliases, hand-copied masks
  or use the renderer preview pose as wire truth.
  Interaction width, height and response membership, wire indices and typed defaults must remain
  derived from generated `DATA_WIDTH_ID`, `DATA_HEIGHT_ID` and `DATA_RESPONSE_ID`; world bounds and
  native debug/crosshair consumers and their fixtures must not restore fixed metadata aliases or
  hand-written wire indices.
  AbstractBoat paddle and bubble-time membership, inherited wire indices and typed defaults must
  remain derived from generated `DATA_ID_PADDLE_LEFT`, `DATA_ID_PADDLE_RIGHT` and
  `DATA_ID_BUBBLE_TIME`; world/native animation consumers and their fixtures must not restore fixed
  metadata aliases or hand-written wire indices.
  VehicleEntity hurt-time, hurt-direction and damage membership, inherited wire indices and typed
  defaults must remain derived from generated `DATA_ID_HURT`, `DATA_ID_HURTDIR` and
  `DATA_ID_DAMAGE`; vehicle animation, event dispatch and world/native fixtures must not restore
  fixed metadata aliases, hand-written wire indices or a parallel boat/minecart membership union.
  Creeper swell-direction, powered and ignited membership, wire indices and typed defaults must
  remain derived together from generated `DATA_SWELL_DIR`, `DATA_IS_POWERED` and
  `DATA_IS_IGNITED`; world animation, native model/debug consumers and their fixtures must not
  restore fixed metadata aliases or hand-written wire indices, and the native powered-model
  projection must not restore a Creeper type predicate alongside the accessor membership.
  Guardian moving and attack-target membership, inherited wire indices and typed defaults must
  remain derived together from generated `DATA_ID_MOVING` and `DATA_ID_ATTACK_TARGET`; world
  animation/beam consumers and their semantic fixtures must not restore fixed metadata aliases,
  hand-written wire indices or reads without generated Guardian membership.
  Shulker attach-face, peek and color membership, wire indices and typed defaults must remain
  derived together from generated `DATA_ATTACH_FACE_ID`, `DATA_PEEK_ID` and `DATA_COLOR_ID`;
  world bounds/animation/state and native model/debug consumers plus their semantic fixtures must
  not restore fixed metadata aliases, hand-written wire indices, version-number entity aliases or
  reads without generated Shulker membership.
  SnowGolem pumpkin membership, wire index and typed default must remain derived from generated
  `DATA_PUMPKIN_ID` and `PUMPKIN_FLAG`; world model projection and world/native semantic fixtures
  must not restore a SnowGolem type predicate, fixed accessor/default/mask aliases or hand-written
  wire indices alongside accessor membership.
  Sheep wool membership, wire index, typed default, color mask and sheared mask must remain derived
  together from generated `DATA_WOOL_ID`/method evidence; native model/debug consumers and semantic
  fixtures must not restore fixed accessor/default/mask aliases or hand-written wire indices.
  Rabbit variant membership, wire index and typed default must remain derived from generated
  `DATA_TYPE_ID`; native model/debug consumers and semantic fixtures must not restore fixed
  accessor/default aliases or hand-written wire indices, and invalid variant ids must continue
  falling back through the generated default.
  Pufferfish puff-state membership, wire index and typed default must remain derived from generated
  `PUFF_STATE`; world bounds, native model/debug consumers and semantic fixtures must not restore
  fixed accessor/default aliases, hand-written wire indices or read the slot without generated
  Pufferfish membership.
  Slime size membership, inherited wire index and typed default must remain derived from generated
  `Slime.ID_SIZE`; world bounds/passenger attachment and native Slime/MagmaCube model/debug
  consumers and semantic fixtures must not restore fixed accessor/default aliases, hand-written
  wire/entity type ids or parallel family predicates.
  Phantom size membership, wire index and typed default must remain derived from generated
  `Phantom.ID_SIZE`; world bounds, native model/debug consumers and semantic fixtures must not
  restore fixed accessor/default aliases, hand-written wire/entity type ids or confuse it with the
  same-named Slime accessor.
  Salmon variant membership, wire index and typed default must remain derived from generated
  `Salmon.DATA_TYPE`; world bounds, native model/debug consumers and semantic fixtures must not
  restore fixed accessor/default aliases, hand-written wire/entity type ids or bypass the existing
  clamped variant mapping.
  TropicalFish packed-variant membership, wire index and typed default must remain derived from
  generated `TropicalFish.DATA_ID_TYPE_VARIANT`; native model/debug consumers and semantic fixtures
  must not restore fixed accessor/default/mask/shift aliases or hand-written wire indices.
  Parrot variant membership, wire index and typed default must remain derived from generated
  `Parrot.DATA_VARIANT_ID`; native model/debug consumers and semantic fixtures must not restore
  fixed accessor/default aliases, hand-written wire indices or a local ordinal bound, and ids must
  follow vanilla `ByIdMap` endpoint clamping.
  Axolotl variant membership, wire index and typed default must remain derived from generated
  `Axolotl.DATA_VARIANT`; native model/debug consumers and semantic fixtures must not restore fixed
  accessor/default aliases, hand-written wire indices or a parallel save-id normalizer, and invalid
  ids must follow vanilla ZERO fallback.
  Axolotl from-bucket membership, wire index and typed default must likewise remain derived from
  generated `Axolotl.FROM_BUCKET`; native debug and semantic fixtures must not restore fixed
  accessor/default aliases or reuse AbstractFish's separately owned field.
  AbstractFish-family from-bucket membership, inherited wire index and typed default must remain
  derived from generated `AbstractFish.FROM_BUCKET`; native debug and semantic fixtures must not
  restore fixed accessor/default aliases or reuse Axolotl's separately owned field.
  Ocelot trusting membership, wire index and typed default must remain derived from generated
  `Ocelot.DATA_TRUSTING`; native debug and semantic fixtures must not restore fixed
  accessor/default aliases or treat another feline family as a member.
  MushroomCow variant membership, wire index and typed default must remain derived from generated
  `MushroomCow.DATA_TYPE`; native model/debug consumers and semantic fixtures must not restore fixed
  accessor/default aliases, hand-written wire indices or a parallel serialized-name mapper, and
  invalid ids must follow vanilla CLAMP behavior.
  Bogged sheared membership, wire index and typed default must remain derived from generated
  `Bogged.DATA_SHEARED`; native model/debug consumers and semantic fixtures must not restore fixed
  accessor/default aliases, hand-written wire indices or read the boolean slot without generated
  Bogged membership.
  Allay dancing membership, wire index and typed default must remain derived from generated
  `Allay.DATA_DANCING`; world animation consumers and semantic fixtures must not restore fixed
  accessor/default aliases, hand-written wire indices or read another entity family's slot 16 as
  the dancing flag. The native F3+I can-duplicate fixture must likewise derive generated
  `Allay.DATA_CAN_DUPLICATE` membership, wire index and default rather than restore slot 17.
  Strider boost-time fixture membership, wire index and typed default must remain derived from its
  generated `Strider.DATA_BOOST_TIME` owner rather than Pig's separate same-name accessor; existing
  suffocating render/debug consumers must remain generated as a separate BOOLEAN field.
  Witch using-item fixture membership, wire index and typed default must remain derived from the
  generated Witch-only `DATA_USING_ITEM` BOOLEAN accessor rather than a fixed slot or another
  Raider-family entity's metadata layout.
  Player shoulder-parrot world/native fixtures must use the generated left/right
  `OPTIONAL_UNSIGNED_INT` accessor ids; local slot 19/20 aliases must not return.
  Local-player air/frozen/vehicle-health fixtures must use generated Entity/LivingEntity
  accessors, and absorption projection must resolve the generated Player-only FLOAT accessor;
  their local fixed slot aliases must not return.
  Entity scale and local-player max-health fixtures must consume the generated attribute catalog
  ids; local `SCALE=25` or `MAX_HEALTH=19` aliases must not return.
  Player-negative Chicken and EnderDragon part/event fixtures must consume the generated entity
  type catalog constants; local numeric type-id aliases must not return.
  Avatar main-arm/model-customization fixtures must consume generated accessor, HumanoidArm and
  cape-part ids; fixture aliases must not return, while Mannequin's explicit runtime default remains
  a separate native projection boundary.
  Swing-duration mob-effect fixtures must consume the generated Haste, Mining Fatigue and Conduit
  Power ids; local 2/3/28 aliases must not return.
  Local-player movement's Bad Omen negative fixture must consume the generated mob-effect catalog
  id; a local numeric Bad Omen alias must not return.
  Net probe merchant-container fixtures must consume the generated MenuType catalog id; a local
  merchant menu id alias must not return.
  Inventory profile and native attack-range component fixtures must consume the generated
  DataComponentType catalog ids; local profile/attack-range component aliases must not return.
  Native inventory hash/prediction fixtures must consume generated max-damage, damage and map-id
  component ids; local 2/3/46 component aliases must not return.
  Player inventory equipment-slot indices must remain derived from `Inventory.java` through the
  generated equipment-slot catalog; world/native 36-42 aliases (including movement/destroy
  fixtures) and offhand button literals must not return.
  World/native air block-state fixtures must resolve the embedded 26.1 block-state report through
  `vanilla_air_block_state_id`; local fixed `AIR=0` aliases must not return.
  World and item-model max-stack-size logic must consume the generated `Item.java` default and
  absolute limits; local 64/99 stack-size owners must not return.
  Axolotl playing-dead membership, wire index and typed default must remain derived from generated
  `Axolotl.DATA_PLAYING_DEAD`; world animation consumers and semantic fixtures must not restore
  fixed accessor/default aliases, hand-written wire indices or read another entity family's slot 19
  as the playing-dead flag.
  Camel dash and pose-tick membership, wire indices and typed defaults must remain derived from
  generated `Camel.DASH` and `Camel.LAST_POSE_CHANGE_TICK`, including inherited CamelHusk
  membership; world animation/dimensions consumers and semantic fixtures must not restore fixed
  accessor/default aliases, hand-written wire indices or confuse dash with `AbstractNautilus.DASH`.
  Camel-only baby scaling and its fixtures must read the generated inherited baby accessor while
  preserving CamelHusk's `isBaby() = false` override.
  PolarBear standing membership, wire index and typed default must remain derived from generated
  `PolarBear.DATA_STANDING_ID`; world animation consumers and semantic fixtures must not restore
  fixed accessor/default aliases, hand-written wire indices or read the standing flag without
  generated PolarBear membership.
  Wolf interested/collar-color/anger-end-time membership, wire indices and typed defaults must
  remain derived together from generated `Wolf.DATA_INTERESTED_ID`, `DATA_COLLAR_COLOR` and
  `DATA_ANGER_END_TIME`; world head-roll plus native model/tail/F3+I consumers and fixtures must not
  restore fixed accessor/default aliases or read same-index metadata from another entity family.
  Cat lie-down/relax-state-one/collar-color membership, wire indices and typed defaults must remain
  derived together from generated `Cat.IS_LYING`, `RELAX_STATE_ONE` and `DATA_COLLAR_COLOR`; world
  animation/sleeping-player plus native model/F3+I consumers and fixtures must not restore fixed
  accessor/default aliases or parallel Cat membership gates.
  Creaking can-move, active, tearing-down and home-position membership, wire indices and typed
  defaults must remain derived from generated `Creaking.CAN_MOVE`, `Creaking.IS_ACTIVE`,
  `Creaking.IS_TEARING_DOWN` and `Creaking.HOME_POS`; world animation/collision, native model and
  F3+I save-debug consumers must not restore fixed accessor/default aliases or a parallel Creaking
  type gate.
  AbstractPiglin zombification immunity plus Piglin baby/dancing membership, wire indices and typed
  defaults must remain derived from generated `AbstractPiglin.DATA_IMMUNE_TO_ZOMBIFICATION`,
  `Piglin.DATA_BABY_ID`, and `Piglin.DATA_IS_DANCING`; native model/pose and F3+I save-debug
  consumers must not restore fixed aliases or project Piglin-only fields onto PiglinBrute.
  Pillager and Piglin charging-crossbow membership, wire indices and typed defaults must remain
  derived from generated `Pillager.IS_CHARGING_CROSSBOW` and
  `Piglin.DATA_IS_CHARGING_CROSSBOW`; world animation/native hand-pose consumers and semantic
  fixtures must not restore fixed accessor/default aliases, hand-written wire indices or treat
  PiglinBrute as owning the regular Piglin accessor.
  CopperGolem weathering/animation-state membership, wire indices and typed defaults must remain
  derived as one bundle from generated `CopperGolem.DATA_WEATHER_STATE` and
  `CopperGolem.COPPER_GOLEM_STATE`; world animation, native render, F3+I save-debug consumers and
  semantic fixtures must not restore fixed accessor/default aliases or hand-written wire indices,
  and must keep generated enum ordinals.
  Sniffer animation-state membership, wire index and typed default must remain derived from
  generated `Sniffer.DATA_STATE`; world animation, dimensions, F3+I and semantic fixtures must not
  restore fixed accessor/default aliases, hand-written wire indices, or duplicate the generated
  `Sniffer.State` ordinals. Sniffer baby bounds and fixtures must use the generated inherited baby
  accessor rather than a fixed AgeableMob slot; the separate drop-seed accessor remains generated.
  Armadillo animation-state membership, wire index and typed default must remain derived from
  generated `Armadillo.ARMADILLO_STATE`; world metadata/event and native model/debug consumers
  and their semantic fixtures must not restore fixed accessor/default aliases, hand-written wire
  indices, or duplicate the generated `ArmadilloState` ordinals; event `64` remains behavioral.
  Warden client-anger membership, wire index and typed default must remain derived from generated
  `Warden.CLIENT_ANGER_LEVEL`; heartbeat production and fixtures must carry entity type, must not
  restore fixed accessor/default aliases or hand-written wire indices, and must not read another
  family's slot 16 as anger.
  EnderDragon phase membership, wire index and typed default must remain derived from generated
  `EnderDragon.DATA_PHASE`; animation/default-state consumers and world/native fixtures must not
  restore fixed accessor/default aliases or hand-written wire indices, and must not duplicate
  generated phase ordinals or sitting membership.
  Wither side-head target membership, wire indices and typed defaults must remain derived from
  generated `WitherBoss.DATA_TARGET_B/C`; world tick consumers and world/native fixtures must carry
  entity type, must not restore fixed accessor/default aliases or hand-written wire indices, and
  must not mix in center-head `DATA_TARGET_A`.
  IronGolem player-created debug membership, wire index, default and named mask must remain derived
  together from generated `IronGolem.DATA_FLAGS_ID`/`isPlayerCreated`; native debug production and
  fixtures must not restore fixed accessor/default/mask aliases or hand-written wire facts.
  Saveable projectile lifecycle bounds must remain composed from generated projectile/saveability
  membership and generated `EntityType.Builder.sized` dimensions rather than a projectile size
  table; production and fixtures must not restore fixed registry ranges or projectile-type tables.
  Projectile owner-collision pickability must consume the generated nearest-`isPickable` class
  behavior plus redirectable-projectile tag; production and fixtures must not restore representative
  entity-type tables, while spectator/marker remain per-instance gates with generated metadata.
  Remote-entity powder-snow collision must consume the generated
  `powder_snow_walkable_mobs` entity-type tag rather than a parallel entity-type set; production
  and fixtures must not restore representative tag members, while equipment/descending/falling-
  block state remains runtime instance behavior.
  Collision-debug pushability class dispatch must consume the generated nearest-`isPushable`
  behavior; production and class-dispatch fixtures must not restore representative entity-type
  tables, while health/climbable/mount/pose/metadata checks remain runtime instance gates.
  Collision-debug `canBeCollidedWith` dispatch must consume the generated nearest class behavior;
  production and class-dispatch fixtures must not restore representative entity-type tables, while
  Shulker health and HappyGhast age/health/other-entity/metadata checks remain instance gates.
  Ender-dragon sitting pose membership must consume the generated nearest `isSitting()` phase-class
  behavior, including `DragonHoverPhase`'s direct override; production and the full registered-phase
  fixture must not restore a parallel phase-id table or sitting-phase aliases.
  Hoglin/Zoglin shared attack-animation event and tick membership must remain derived from the
  generated `HoglinBase` interface graph; production and the full entity-catalog fixture must not
  restore parallel entity-type tables.
  Illager celebrating-arm-pose membership must combine the generated Raider `IS_CELEBRATING`
  accessor with the nearest generated `getArmPose` class behavior rather than fixed metadata aliases
  or an Evoker/Vindicator entity-type table. Spellcaster casting and Pillager charging fixtures must
  likewise use their generated accessors instead of hand-written wire indices; generated guards
  must inspect those specific fixtures rather than rely on whole-file matches.
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
  native-only `last_*` snapshots wherever a world owner exists or should exist;
  any new control exposure must derive from the paired immutable world/runtime
  generation rather than create an independent control-owned value.
- Keep `NetCounters` limited to connection/runtime status and command-queue
  projections.
## Signed chat and chat acknowledgement production

Owner: `bbb-protocol` + `bbb-net` + `bbb-world` + `bbb-native`

- Authenticated Services block-list synchronization and player-report
  composition/submission remain external account/social boundaries. The native
  `ChatListener` now owns receive-ability and secure-only gates, bounded delayed
  delivery, trust tags, the 1,024-entry reporting log, rich HUD interaction, and
  narrator delivery; startup hidden/blocked UUIDs are not a substitute for a
  signed-in Services client or reporting UI.
- Exact `show_item` / `show_entity` hover payloads and custom font ids are
  retained in canonical Components, but chat currently renders bounded
  name/id/count hover fallbacks and the active default font when a custom font
  provider is unavailable. Server-supplied `open_file` remains deliberately
  inert; completing reusable item/entity tooltip rendering or additional font
  providers must consume those retained values rather than add a parallel
  Component schema.

## Crosshair entity interaction parity

Owner: `bbb-world` + `bbb-native` + `bbb-renderer`

- Validate any future `yRotA` source.
- Replace the debug target overlay with full entity model rendering and
  interaction feedback once renderer entity presentation exists.

## Native input, movement, interaction, inventory, and command flows

Owner: `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world`

Movement:
- Apply the F1-backed `hideGui` state to the remaining HUD, crosshair, overlays,
  and first-person presentation. Generic entity-name visibility consumes it;
  the rest of the GUI still renders.

- Complete the remaining camera-FOV inputs beyond synchronized player abilities:
  configurable base FOV/effect scale, bow-use/scoping zoom, and death/fluid
  modifiers. Vanilla keeps `walkingSpeed` as the FOV ratio denominator; it does
  not replace the separately synchronized movement-speed attribute base.
- Cover the remaining vanilla survival physics beyond the synchronized local
  movement cadence.
- Cover the remaining vanilla voxel collision shapes.
- Implement full vanilla post-`Entity.move` `deltaMovement` travel ordering.
- Add sprint-swim camera, animation, and presentation nuance.
- Implement the `minecraft:powder_snow` fall-sound side effects (the
  inside-block snowflake particle and extinguish effects are wired through the
  block-effect evaluator and particle resolver).
- Handle the remaining `MovePlayer.Rot` vehicle movement send edge cases.

Commands:

- Add focused command-queue and encode tests for inventory, interaction, chat,
  command, and sign editing.
- Complete target-specific local attack prediction and presentation from
  `Player.attack`: synchronized enchantment effects, critical and magic-critical
  tracking emitters, special `Entity.hurtClient` families, and redirectable
  projectile deflection. Damage and combat outcomes remain server-authoritative.
- Complete local entity-use parity from `Minecraft.startUseItem` and
  `MultiPlayerGameMode.interact`: exact main/offhand iteration until a local
  `InteractionResult.Success`, hands-busy and enabled-item gates, local
  `interactOn` prediction, swing-source handling, and held-item presentation.
  Entity state and inventory corrections remain server-authoritative.
- Complete `InstrumentItem.use` prediction from named and direct synchronized
  holders: preserve sound/range/use-duration values in the ItemStack component,
  play the local records-source sound, start the use pose, and apply the matching
  cooldown without interpreting a custom holder through vanilla bootstrap order.

Inventory:

- Implement the creative-inventory screen item-add/item-drop interactions that
  drive `SetCreativeModeSlot`, including its local infinite-material,
  enabled-feature, slot, count, open-container, and drop-throttler gates. The server
  remains authoritative for reach, creative/game-master data access, hotbar
  selection and inventory mutation.
- Implement the rich-tooltip `space` font provider, the remaining text styles,
  italic/complex component styles, and component-specific detail lines from
  `font/default.json`.
- Implement the remaining RecipeBookAdd / RecipeBookRemove presentation variants
  (including RecipeToast, hover/tooltips, focus, and narration) and creative
  inventory variants.
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
generated, including its named type-id constants; native consumers must not restore
resource-to-id declaration macros or a second expected registry length. Particle definition JSON and atlas sprites remain resource-pack-owned
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

Submission surfaces:

- Add renderer submission surfaces for remaining block-feature submissions,
  then wire their draw ordering into vanilla's
  ordered text phase. Texture-backed special geometry added by these surfaces
  must record complete submission metadata before atlas lookup rather than
  reuse the no-texture custom-geometry path.

Living-entity render state:

- Project the remaining conversion-time shaking input;
  `LivingEntityRenderer.isShaking` reads it, but `conversionTime` is currently
  server-side only.
- Armor-stand pose interpolation is `not-needed`: vanilla 26.1
  `ArmorStandRenderer.extractRenderState` copies all six synced `Rotations`
  getters directly without partial-tick interpolation. Only `yRot` uses
  `Mth.rotLerp`, and hit `wiggle` adds `partialTicks`; bbb already covers both.

Attachment and spawner presentation:

- Wire the camel body-anchor passenger/leash attachment consumers. The anchor
  provider `Camel.getBodyAnchorAnimationYOffset` already exists.
- Implement the ominous-item-spawner about-to-spawn and spawn sounds.
Environment:

- Preserve complete `ParticleOptions` payloads when a custom dimension selects
  a complex default dripstone particle. The current level-event bridge carries
  the synchronized particle type but does not project block/item/color/vibration
  option state into the provider; item options also still need exact nested
  `DataComponentPatch.CODEC` value validation.
- Implement the creeper/spider/invert spectator post chains, the independent
  F4 `effectActive` toggle, and the perspective-change clear lifecycle. The
  renderer-owned current id currently drives only the debug entry mirror.

## HUD overlay and screen render surfaces

Owner: `bbb-renderer` + `bbb-native` + `bbb-world`

- Preserve full trusted Components in the TabList header/footer and complete
  remaining player-list presentation parity: encrypted-session player heads,
  narrator visibility announcements, and vanilla ping sprites.
- `chunk_section_paths`, `chunk_section_octree`, and
  `chunk_section_visibility` are `deferred`: stable per-section CPU/GPU
  residency now exists, but a repo-native `SectionOcclusionGraph`/octree and
  compiled-section face-visibility matrix do not. Restart by wiring the stable
  resident section owners into vanilla node step/source directions, octree node
  bounds/depth/frustum state, and `facesCanSeeEachother` without reconstructing
  them heuristically.
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
  other registry-backed entity-data projection (the fixed vanilla
  entity/block-entity type catalogs are generated),
  packed variant projection, remaining codec-backed SNBT projection (component removals are complete;
  added values still include `AbstractArrow`
  `inBlockState` and `item`, attribute-modifier override-display Component trees,
  and lossless persistent reconstruction for direct instrument and direct
  jukebox-song network holders. Painting direct network holders are intentionally
  omitted from persistent reconstruction because vanilla `PaintingVariant.CODEC`
  is a named-only `RegistryFixedCodec`. Keep the remaining dispatch
  owned by the protocol
  generated-metadata/handler descriptor catalog and native persistent SNBT encoder registry
  beyond the supported custom-NBT/primitive (including repair-cost/enchantable/glider/
  minimum-attack-charge)/
  enchantments/stored-enchantments/
  adventure-mode-predicate (block/state/NBT/exact/partial matchers; its 15
  predicate-type ids/resources are generated)/
  equippable (slot/sound/asset/entity-holder/default flags)/
  blocks-attacks (damage-reduction/item-damage/damage-type-holder/sound)/
  attribute-modifiers-named-holder-default-hidden-display/
  simple-resource (note-block-sound)/
  consumable/death-protection/potion-contents/
  weapon/
  tool-block-holder-rules/
  use-remainder/
  use-cooldown/tooltip-display/attack-range/
  swing-animation/resource/color (including dye/base-color)/map/ominous/firework/custom-model-data/
  codec-backed-NBT (flattened-SNBT alternatives for custom-data, entity-data,
  bucket-data, and block-entity-data/lock)/
  charged-projectile/bundle/written-book-exact-component-trees/
  lodestone/profile/
  container/block-state/
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
- Parse and apply the remaining synchronized enchantment effect component
  families beyond attribute modifiers and deterministic
  `minecraft:crossbow_charge_time`: conditional value/entity/location effects
  still need their loot-context requirements, and `remove_binomial` needs the
  holder entity's mutable `RandomSource` rather than a fabricated random result.

## Audio runtime parity

Owner: `bbb-audio` + `bbb-native` + `bbb-pack` + `bbb-world`

- Validate remaining synthesized/game-event sound mappings against vanilla.
- Add the client-owned animal ambient/step/eat and cat/wolf state-trigger loops
  before consuming the remaining already-decoded sound-set fields; do not
  synthesize those events from registry presence alone.
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

Owner: `bbb-pack` + `bbb-item-model` + `bbb-native` + `bbb-renderer`

- Consume atlas mip animation metadata in the renderer, beyond the current
  static sprite use.
- Apply full item tint parity for dynamic sources across every item rendering
  path.
- Execute parsed `minecraft:special` item models through their special renderer
  instead of resolving only the base baked model in GUI, hand, dropped-item,
  and item-frame paths. Follow `SpecialModelWrapper` / `SpecialModelRenderer`;
  `DecoratedPotSpecialRenderer` must consume `POT_DECORATIONS` face data.
- Load equipment asset layers for armor/equipment rendering.

Loader invariants (not todos): keep the local vanilla coverage tests
(`loads_all_local_vanilla_atlases`, `loads_local_vanilla_item_model_catalog`,
`loads_local_vanilla_item_registry`,
`loads_local_vanilla_equipment_asset_catalog`) green when changing pack loaders;
add parser support only when an official asset or a focused resource-pack
fixture fails them; keep precedence/filter tests next to the loaders.

## Playable live coverage beyond the session-lifecycle closure

Owner: `scripts/playable-gate.sh` + `scripts/playable_gate.py` + affected runtime
owners

- Add server-result assertions for entity interaction, hotbar, container,
  inventory, and chat/command flows beyond the existing movement and
  destroy/place closure.
- Add presentation evidence for representative audio/particle effects and a
  deterministic screenshot/readback. Renderer/audio counters must identify the
  exact lifecycle generation rather than merely proving that the processes
  stayed alive.
- Keep failure evidence bounded and stage-specific while adding those surfaces;
  do not turn the top-level opening `world_snapshot` into a misleading
  failure-time sample.

The current live gate already covers two official servers in one run: death and
respawn, both dimension replacements, production resource-pack push/pop,
Play-to-Configuration-to-Play, and same-process Transfer reconnect with a fresh
registry session. This boundary is an invariant for the remaining breadth; it
does not imply complete Play protocol, entity behavior, presentation, or
long-session coverage.

## Resource and dynamic texture generalization

Owner: `bbb-pack` + `bbb-native` + `bbb-renderer`

Lifecycle invariants (not todos): vanilla, startup and server packs share one
validated low-to-high selected stack and immutable generation; CPU consumers
prepare from that stack, renderer/audio owners commit atomically, and control
publishes source/priority/failure plus bounded dynamic-texture observations.
Skin/cape/elytra requests share one exact resource/session-generation cache.

- Implement broader non-profile dynamic texture loading.
- Connect synchronized custom block/entity registry identities to the existing
  resource-pack model/texture loaders. The selected-generation fixtures already
  cover custom item/cuboid model, PNG texture, font, sound and data-resource
  precedence; do not infer unsupported registry mappings from those loader tests.
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
## Fonts and text

- **`unihex` / CJK.** The consumed assets tree ships no `font/unifont*.zip`
  archive, so the `include/unifont` reference cannot resolve its data;
  codepoints outside the bitmap pages degrade to the `?` replacement glyph.
  Restart when a unifont source exists in the consumed assets.
- **Bidirectional shaping.** Vanilla routes through ICU4J `ArabicShaping` /
  `Bidi` in `Font` / `StringSplitter`; current consumers render logical order
  only. Restart with the rest of rich text layout.

## Container menu presentation parity

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
- ContainerSetSlot also rejects mismatched nonzero menu ids. Remaining parity
  includes creative-inventory remote-slot synchronization and hotbar pop-time
  presentation.
- SetCursorItem updates carried state outside creative inventory; the creative
  screen suppression branch is still open.
- SetPlayerInventory now limits canonical state to vanilla inventory/equipment
  slots 0..42. Tutorial pickup presentation is still open.
- PlayerCombatEnter is a strict unit packet whose 26.1 client handler is empty;
  its diagnostic counter is not evidence of an additional combat-mode runtime.
- PlayerCombatEnd likewise has an empty 26.1 handler; its decoded duration may
  remain diagnostic but does not establish a client combat-mode transition.
- `BeaconMenu`: render confirm/cancel hover state, selected-effect highlight and
  hover state, labels, and tooltips.
- `EnchantmentMenu`: animated book model, enchanting glyph text and its
  disabled/highlight coloring, hover tooltips.
- `CartographyTableMenu`: live map pixels and decorations in the preview area,
  invalid-transform prediction when item runtime or map state is unavailable.
- `LoomMenu`: render each ordered selectable-pattern banner thumbnail, the
  composited banner preview, and the max-pattern error overlay.
- `LecternMenu`: rich text styles and click events, exact vanilla font wrapping,
  text filtering toggle behavior.
- `SmithingMenu`: generic held-item / non-skull head-item projection for the
  armor stand preview, cycling empty-slot icons, tooltips.

## HUD and debug overlay

- **Heart damage/heal blink flash.** Needs the untracked
  `player.invulnerableTime` (no client-side sync) and the wall-clock
  `displayHealth` / `lastHealthTime` hold, neither reproducible
  deterministically. `HudPlayerHealth` always draws `blinking = false`; sprite
  names are already complete for when it lands.
- **Air bubble-pop sound** (`playAirBubblePoppedSound`, `Gui.java:930-937`).
  Restart when a HUD-side sound sink exists.
- **Accessibility text backdrop** (`textWithBackdrop`, option default `0`).
- **Jukebox now-playing path** (`Gui.setNowPlaying`, the only `animate_color=true`
  producer). The flag is already carried end to end.
- **Sound-captions subtitle overlay** (`extractSubtitleOverlay`), distinct from
  the title subtitle drawn inside `extractTitle`.
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
- **Per-block-entity distance/frustum culling.** Block entities currently submit
  like entities, unculled.
- **Banner pattern no-depth-write pipeline.** Vanilla's `bannerPattern` pipeline
  disables depth writes; bbb's shared translucent pipeline keeps them on.
  Equal-depth `LessEqual` layering is unaffected. Restart only if a dedicated
  no-depth-write pass matters.
- **Sign glowing 8-way outline glyph pass.** Glowing text renders full-bright
  without outlines.
- **Sign underline/strikethrough effect bars.** They need a white pixel outside
  the single font-atlas draw.
- **Sign obfuscated glyph cycling.** The face currently draws literal glyphs.
- **Sign POLYGON_OFFSET font display mode**, currently approximated by the fixed
  `TEXT_OFFSET` z gap.
- **Chest christmas Dec 24-26 texture swap.** There is no wall-clock input;
  `bbb-world` serializable state must not introduce one.
- **Trial-spawner display-state behavior**, and custom `SpawnData` entity NBT
  variants, which render through default entity metadata. Restart when broader
  synthetic entity NBT projection exists.
- **Biome blend for block-break particles.** They still sample the single centre
  biome, unlike terrain tints which blend over `biomeBlendRadius`.

## Entity models and animation

### Deferred model and animation details

- **Remaining mob-specific armor models.**
- **Datapack/custom item prototype default `SWING_ANIMATION`** beyond the
  resolved vanilla spear ids.
- **GPU outline presentation for `submitOnlyOutline` entity-attached block-model
  layers** (snow-golem carved pumpkin, mooshroom mushroom). Outline-only
  attachment metadata is recorded and ordinary block-quad baking is suppressed.
- **Frog tongue prey-targeting** (`DATA_TONGUE_TARGET_ID`). Not part of the model
  animation.
- **Zombie-nautilus variant dynamic-registry reorder path.** The variant resolves
  from synced `DATA_VARIANT_ID` by bootstrap order.
- **Rabbit `IDLE_HEAD_TILT` keyframe — not-needed.** Vanilla starts it from the
  entity's client-local `random.nextInt(40) + 180` timeout, so a remote client's
  particular phase is not reconstructable and bbb must not fabricate a seed.
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
- **Minecart spawner animated block-entity display content.** Classified as
  block-entity special renderer presentation.
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
## Particles and level events

- **Broader collision-clipping parity** for special contexts beyond the covered
  `WakeParticle` case.
- **Player-coupled particle emitters** beyond the currently covered scoped cases.
- **Idle/tick block-entity client effects** beyond packet-owned level events (the
  vault event `3015` context remains the audit anchor).
- **LevelEvent-adjacent parity owned elsewhere**: terrain/item atlas rendering,
  block-entity client-effect presentation, and audio-device/runtime parity. The
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
- **Separate glow-squid emissive overlay/texture.** 26.1 has none; the glow is
  purely a block-light override.
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

- **Exact golden pixel parity to vanilla screenshots.** No golden PNG or
  full-frame hash: driver and float jitter make it unreliable.
- **Fire-resistance lava fog — deferred.** Vanilla widens lava fog only when the
  active camera is actually in lava and has Fire Resistance. Restart when the
  camera environment owns a real lava fog medium; an effect-only approximation
  would incorrectly alter atmospheric and water fog.
- **Item-pipeline overlay-texture (`Sampler1`) sampling.** Vanilla's
  `ITEM_SNIPPET` does not bind `Sampler1`; `core/item` samples only the item
  atlas and lightmap, and `RenderTypes.itemCutout` / `itemTranslucent` never call
  `useOverlay()`.
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

- **Server-owned FishingHook catching/removal outcomes.** The client tick now
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
