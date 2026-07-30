mod selection;

use std::sync::Arc;

use bbb_render_types::StandaloneTextureSampling;
use glam::Mat4;

use super::{
    BannerPatternKind, CatModelVariant, ChickenModelVariant, CowModelVariant, DecoratedPotPattern,
    FrogModelVariant, PigModelVariant, VillagerModelProfession, VillagerModelType,
    WolfModelVariant, ZombieNautilusModelVariant,
};

pub(in crate::entity_models) use selection::{
    boat_texture_ref, camel_texture_ref, chicken_texture_ref, cow_texture_ref,
    horse_coat_texture_ref, horse_markings_texture_ref, llama_texture_ref, mooshroom_texture_ref,
    pig_texture_ref, sheep_wool_render_color, squid_texture_ref, villager_level_texture_ref,
    villager_profession_texture_ref, villager_type_texture_ref, wolf_texture_ref,
    zombie_villager_level_texture_ref, zombie_villager_profession_texture_ref,
    zombie_villager_type_texture_ref,
};
#[cfg(test)]
pub(in crate::entity_models) use selection::{
    player_texture_ref, sheep_jeb_wool_layer_color, sheep_wool_layer_color,
};

// Pure value types that also project into `bbb-item-model` live in `bbb-render-types`; re-export
// them here so every existing `crate::entity_models::...` path keeps working.
pub use bbb_render_types::{
    EntityArmorTrim, EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDynamicPlayerSkin,
    EntityDynamicPlayerSkinStatus, EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind,
    EntityDynamicTexture, EntityEquipmentAssetId, EntityEquipmentLayerTexture,
    EntityModelTextureRef, EntityPlayerSkin, EntityPlayerSkinModel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityAttachmentFace {
    #[default]
    Down,
    Up,
    North,
    South,
    West,
    East,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkullBlockModelAttachment {
    Ground,
    Wall { facing: EntityAttachmentFace },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndPortalModelKind {
    EndPortal,
    EndGateway,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndPortalModelFace {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpawnerDisplayRenderState {
    pub spin_degrees: f32,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntityModelKind {
    Chicken {
        variant: ChickenModelVariant,
        baby: bool,
    },
    Pig {
        variant: PigModelVariant,
        baby: bool,
    },
    Player {
        skin: EntityPlayerSkin,
        parts: PlayerModelPartVisibility,
    },
    Humanoid {
        family: HumanoidModelFamily,
        baby: bool,
    },
    ArmorStand {
        small: bool,
        marker: bool,
        show_arms: bool,
        show_base_plate: bool,
        pose: ArmorStandModelPose,
    },
    Slime {
        size: i32,
    },
    MagmaCube {
        size: i32,
    },
    Ghast {
        /// Vanilla `GhastRenderer.getTextureLocation`: `isCharging` swaps the base texture to
        /// `ghast_shooting.png` (the open-mouth shooting face). Same `GhastModel`, only the texture differs.
        charging: bool,
    },
    /// `HappyGhastModel` adult/baby baked layer. BODY equipment state stays in render state because
    /// it changes the body subtree scale and optional harness overlay without changing entity identity.
    HappyGhast {
        baby: bool,
    },
    Blaze,
    Endermite,
    Silverfish,
    /// Vex (`VexModel`, `VexRenderer`). The idle wing flap, arm bob, head look, and body
    /// tilt read `EntityRenderState.age_in_ticks` and the head yaw/pitch. `charging`
    /// (`Vex.isCharging`, the synced `DATA_FLAGS_ID & 1`) swaps the texture to
    /// `vex_charging.png` (vanilla `VexRenderer.getTextureLocation`); hand item presence lives
    /// in `EntityRenderState` because it changes only the charging arm pose, not the model key.
    Vex {
        charging: bool,
    },
    /// Allay (`AllayModel`, `AllayRenderer`). The idle/flying wing flap, arm bob, head look,
    /// body tilt, and vertical bob read `EntityRenderState.age_in_ticks`, the walk
    /// animation, and the head yaw/pitch. Projected `isDancing` / `isSpinning` state drives the
    /// dance pose, and held-item progress raises both arms through the same model owner.
    Allay,
    /// Strider (`AdultStriderModel` / `BabyStriderModel`, `StriderRenderer`). The body sway,
    /// vertical bob, leg swing/lift, and bristle flow read `EntityRenderState.age_in_ticks`,
    /// the walk animation, and the look angles. The `cold` flag (the synced `isSuffocating()`) swaps
    /// to the `strider_cold` texture × age (`StriderRenderer.getTextureLocation`). The generic render
    /// state carries `isRidden` and the saddle equipment-layer flag; native folds the suffocating
    /// shake into `body_rot` like vanilla `StriderRenderer.isShaking`.
    Strider {
        baby: bool,
        cold: bool,
    },
    /// Turtle (`AdultTurtleModel` / `BabyTurtleModel`, `TurtleRenderer`). The
    /// `QuadrupedModel` head look plus the `TurtleModel.setupAnim` land walk / water swim leg
    /// branch (`isOnLand = !isInWater && onGround`) read the projected look angles, walk
    /// animation, water, and ground state. Projected `isLayingEgg` amplifies the land gait and
    /// adult `hasEgg` selects the lowered `egg_belly` shell.
    Turtle {
        baby: bool,
    },
    /// Bat (`BatModel`, `BatRenderer`). The first keyframe-animated entity: the looping
    /// `BatAnimation.BAT_FLYING` wing flap / body bob is sampled from the projected
    /// `flyAnimationState` elapsed time. While the synced `isResting` is set the model swaps to
    /// the static `BatAnimation.BAT_RESTING` hanging pose and `applyHeadRotation` turns the head
    /// by the look yaw.
    Bat,
    /// Bee (`AdultBeeModel` / `BabyBeeModel`, `BeeRenderer`). The procedural `BeeModel.setupAnim`
    /// airborne wing flap (`zRot = cos(ageInTicks · 120.32113°) · π · 0.15`) and the idle
    /// `bobUpAndDown` bone/leg/antenna bob, gated on the synced `Entity.onGround`. The anger pose
    /// (`isAngry`), the rolled-up fall pose (`rollAmount`) and the stinger-loss (`hasStinger`) are
    /// projected as render-state. `angry`/`has_nectar` drive the vanilla
    /// `BeeRenderer.getTextureLocation` eight-way texture swap (`bee[_angry][_nectar][_baby].png`).
    Bee {
        baby: bool,
        angry: bool,
        has_nectar: bool,
    },
    /// Breeze (`BreezeModel`, `BreezeRenderer`). The base body layer (head + three rods) driven by
    /// the looping `BreezeAnimation.IDLE` keyframe animation (the second keyframe entity, and the
    /// first to use CATMULLROM), sampled from `EntityRenderState`'s `age_in_ticks`. The swirling
    /// translucent wind layer, emissive eyes, and shoot/slide/inhale/jump action animations are
    /// projected and submitted by their typed model/layer owners.
    Breeze,
    /// Dolphin (`DolphinModel`, `DolphinRenderer`). The procedural `DolphinModel.setupAnim` steers
    /// the body by the look pitch/yaw and, while moving (`isMoving`, the synced velocity), adds the
    /// swim body tilt and tail/tail-fin wave. The baby uses the `MeshTransformer.scaling(0.5)` body
    /// layer. The held-item carry layer is baked by the native item-model pass from the renderer-owned
    /// `dolphin_carried_item_transform`.
    Dolphin {
        baby: bool,
    },
    /// `GuardianModel` (`elder = false`) or the same mesh scaled 2.35× by
    /// `GuardianModel.ELDER_GUARDIAN_SCALE` (`elder = true`). The procedural spike pulse /
    /// withdrawal, eye tracking, tail sway, and attack beam are projected.
    Guardian {
        elder: bool,
    },
    /// `FrogModel` textured by temperature variant (`FrogVariant` asset). Projected client
    /// animation states drive the exact jump, croak, tongue, swim/walk, and idle-in-water
    /// definitions; prey-target aiming is a separate non-model edge.
    Frog {
        variant: FrogModelVariant,
    },
    /// `CreakingModel` with head look, gated walk, and exact attack, invulnerable, and death
    /// keyframes. `eyes_glowing` shows the `creaking_eyes.png` emissive overlay: native projection
    /// selects synced `isActive()` normally and the client-ticked `hasGlowingEyes()` flicker while
    /// tearing down.
    Creaking {
        eyes_glowing: bool,
    },
    /// `SnifferModel` on the adult `ModelLayers.SNIFFER` or baby `ModelLayers.SNIFFER_BABY` baked
    /// layer. Vanilla `SnifferRenderer` uses `snifflet.png` for babies while still applying
    /// `SnifferModel.setupAnim` to the baby layer tree.
    Sniffer {
        baby: bool,
    },
    /// `WardenModel` with projected head look, walk, idle wobble, tendril sway, attack / sonic-boom /
    /// digging / emerge / roar / sniff keyframes, and the four emissive overlay layers.
    Warden,
    /// `AdultArmadilloModel` / `BabyArmadilloModel` (`baby` selects the baby body layer). When
    /// `rolled_up`, `setupAnim`'s `isHidingInShell` swap is emitted — the body cubes, tail, and
    /// hind legs hide and the shell-ball `cube` shows. The clamped head look, `applyWalk`, roll-out /
    /// roll-up, and peek keyframe animations are projected through render-state fields.
    Armadillo {
        baby: bool,
        rolled_up: bool,
    },
    /// `AdultAxolotlModel` / `BabyAxolotlModel` at their `createBodyLayer` rest pose (`baby`
    /// selects the baby body layer), textured by the five `Axolotl.Variant` colors × age
    /// (`AxolotlRenderer.TEXTURE_BY_TYPE`). The body yaw, swimming / water-hovering /
    /// ground-crawling / lay-still procedural sways, baby keyframe definitions, play-dead pose,
    /// and mirror-leg copy are all applied by the selected model.
    Axolotl {
        baby: bool,
        variant: AxolotlModelVariant,
    },
    /// `TadpoleModel` with the age/in-water-scaled `tail.yRot` swim sway from `setupAnim`.
    Tadpole,
    /// `ParrotModel` textured per the five `Parrot.Variant` colours (`variant`, the synced
    /// `DATA_VARIANT_ID`). Head look, per-pose `prepare` offsets, leg walk, wing flap/bob, and PARTY
    /// dance consume projected pose state; the player shoulder layer explicitly selects ON_SHOULDER.
    Parrot {
        variant: ParrotModelVariant,
    },
    /// `ShulkerModel` at its `createBodyLayer` closed rest pose, textured by dye colour
    /// (`ShulkerRenderer.getTextureLocation`: `None` → the default `shulker.png`, else
    /// `shulker_<color>.png`). The peek open/close, head look, and
    /// `ShulkerRenderer.setupRotations` attach-face/body-yaw root transform are reproduced.
    Shulker {
        color: Option<EntityDyeColor>,
    },
    /// `WitherBossModel` at its `createBodyLayer` bind rest pose, rendered at the vanilla 2.0× scale
    /// with the spawn-charge shrink and `wither_invulnerable.png` texture flicker. The procedural
    /// ribcage/tail breathing sway, the center/side head look, side-head target tracking, and the
    /// `WITHER_ARMOR` powered energy-swirl overlay layer are reproduced.
    Wither,
    /// `GiantZombieModel` — the standard humanoid (zombie) body layer `MeshTransformer`-scaled 6×.
    /// The head look, limb swing, zombie texture, `ItemInHandLayer`, and `HumanoidArmorLayer` match
    /// vanilla `GiantMobRenderer`.
    Giant,
    /// `EndCrystalModel` (the base slab plus the nested glass / core boxes, scaled 2× by
    /// `EndCrystalRenderer`). The diagonal spin, vertical bob, `showsBottom` base toggle, and
    /// `DATA_BEAM_TARGET` custom beam are projected.
    EndCrystal,
    /// `EvokerFangsModel` (the base block plus two jaws). Projected bite progress drives the exact
    /// jaw open/close, base drop, emerge scale, and terminal visibility gate.
    EvokerFangs,
    /// `LeashKnotModel` — the single 6×8×6 knot box. The model has no `setupAnim`, so the geometry
    /// is complete; only the texture-backed path is deferred.
    LeashKnot,
    /// Vanilla 26.1 `ConduitRenderer` submitted per conduit block entity. The inactive form renders
    /// only the 6x6x6 shell on `entity/conduit/base`; the active form expands into four part
    /// instances so each vanilla submit can keep its own root transform and texture: cage, outer wind,
    /// inner wind, and camera-facing eye. Instances are projected from conduit block states
    /// (`entity_id` is a sentinel), not the entity list.
    Conduit {
        part: ConduitModelPart,
    },
    /// Vanilla 26.1 `SkullBlockRenderer` / `BuiltInBlockModels` skull and
    /// head block-entity model. The static skull models/textures are shared
    /// with `CustomHeadLayer`; `attachment` selects the ground 16-segment
    /// transform or the wall-facing offset transform. Instances are projected
    /// from skull/head block states, not the entity list.
    SkullBlock {
        skull: EntityCustomHeadSkull,
        attachment: SkullBlockModelAttachment,
    },
    /// Vanilla 26.1 `TheEndPortalRenderer` / `TheEndGatewayRenderer` block-entity cube source.
    /// Both submit only the Y-axis faces from `AbstractEndPortalRenderer` (`shouldRenderFace`
    /// returns true for `Direction.Axis.Y`). End portals apply the vanilla
    /// `T(0, 0.375, 0) * S(1, 0.375, 1)` cube transform; gateways keep the unit cube and may add
    /// the spawn/cooldown beacon-style beam through `EntityRenderState.end_gateway_beam`.
    EndPortalBlock {
        kind: EndPortalModelKind,
        faces: [EndPortalModelFace; 2],
    },
    /// Vanilla 26.1 `ChestModel` submitted by the `ChestRenderer` block-entity renderer — the first
    /// (and so far only) block-entity model in the scene. `half` selects the single or the
    /// double-chest left/right mesh (`MultiblockChestResources.select`), `texture` the
    /// `Sheets.chooseSprite` material family; the lid/lock openness rotation reads
    /// `EntityRenderState.chest_openness`. Instances are projected from chest block states, not the
    /// entity list, so `entity_id` is a sentinel.
    Chest {
        texture: ChestModelTexture,
        half: ChestModelHalf,
    },
    /// Vanilla 26.1 sign meshes submitted by the `StandingSignRenderer` /
    /// `HangingSignRenderer` block-entity renderers — like the chest, sign
    /// instances are projected from sign block states, not the entity list
    /// (`entity_id` is a sentinel). `attachment` selects the mesh
    /// (`createSignLayer(standing)` board±stick,
    /// `createHangingSignLayer(attachment)` board+chains/plank), `wood` the
    /// `entity/signs[/hanging]/<wood>` sprite. The body yaw rides
    /// `body_rot` (`-angle` degrees, `Axis.YP.rotationDegrees(-angle)`); the
    /// vanilla `scale(s, -s, -s)` body flip and the wall/hanging offsets live
    /// in the sign root transform. The sign face text is rendered by the
    /// separate sign-text quad path, not this model.
    Sign {
        wood: SignModelWood,
        attachment: SignModelAttachment,
    },
    /// Vanilla 26.1 bed meshes submitted by the `BedRenderer` block-entity renderer — like the
    /// chest, bed instances are projected from bed block states, not the entity list (`entity_id`
    /// is a sentinel). `part` selects the `createHeadLayer` / `createFootLayer` mesh (each bed
    /// block renders only its own half), `color` the `Sheets.getBedSprite` `entity/bed/<color>`
    /// sprite (derived from the `minecraft:<color>_bed` block id). `body_rot` carries the
    /// `Axis.ZP.rotationDegrees(180 + facing.toYRot())` angle of
    /// `BedRenderer.createModelTransform`; the model itself has no animation.
    Bed {
        color: EntityDyeColor,
        part: BedModelPart,
    },
    /// Vanilla 26.1 `BellModel` submitted by the `BellRenderer` block-entity renderer — the bell
    /// body + base only (the bar/post support frame is part of the `bell_floor/wall/ceiling/
    /// between_walls` block models the terrain path already draws). Instances are projected from
    /// bell block states (`entity_id` is a sentinel); the shake rotation reads
    /// `EntityRenderState.bell_ticks` / `bell_shake_direction`.
    Bell,
    /// Vanilla 26.1 `ShulkerBoxRenderer.ShulkerBoxModel` — the shulker mob's shell mesh
    /// (`ShulkerModel.createBoxLayer`: `lid` + `base`, no head) submitted per shulker box block
    /// entity. Instances are projected from shulker box block states (`entity_id` is a sentinel).
    /// `color` selects the `Sheets.getShulkerBoxSprite` texture (`None` -> the undyed
    /// `entity/shulker/shulker`); `facing` is the block's six-way `ShulkerBoxBlock.FACING`, folded
    /// into the root transform via `Direction.getRotation()`
    /// (`ShulkerBoxRenderer.createModelTransform`). The lid lift/twist reads
    /// `EntityRenderState.shulker_box_progress`.
    ShulkerBox {
        color: Option<EntityDyeColor>,
        /// The block's `FACING` value reusing the six-way [`EntityAttachmentFace`] direction enum
        /// (`Down`/`Up`/`North`/`South`/`West`/`East`); the vanilla default is `Up`.
        facing: EntityAttachmentFace,
    },
    /// Vanilla 26.1 `DecoratedPotRenderer` — the `DECORATED_POT_BASE` layer (`neck`/`top`/`bottom`
    /// on the 32×32 `decorated_pot_base` sheet) plus the `DECORATED_POT_SIDES` layer (four 14×16
    /// north-face-only planes on 16×16 side sheets). Instances are projected from decorated pot
    /// block states (`entity_id` is a sentinel). Each side carries its sherd's
    /// `DecoratedPotPattern` (`Sheets.getDecoratedPotSprite`), `None` rendering the plain
    /// `decorated_pot_side`. `body_rot` carries the `180 - HORIZONTAL_FACING.toYRot()` yaw of
    /// `DecoratedPotRenderer.createModelTransformation`; the wobble reads
    /// `EntityRenderState.decorated_pot_wobble`.
    DecoratedPot {
        back: Option<DecoratedPotPattern>,
        left: Option<DecoratedPotPattern>,
        right: Option<DecoratedPotPattern>,
        front: Option<DecoratedPotPattern>,
    },
    /// Vanilla 26.1 `BannerRenderer` — the `STANDING_BANNER`/`WALL_BANNER` frame layer (pole +
    /// bar; the wall form has no pole) plus the `STANDING_BANNER_FLAG`/`WALL_BANNER_FLAG` flag
    /// layer, all on the 64×64 `entity/banner/banner_base` sheet, followed by the tinted pattern
    /// layers over the same flag geometry (`BannerRenderer.submitPatterns`: the `entity/banner/
    /// base` sheet tinted by `base_color`, then each `BannerPatternLayers.Layer`'s
    /// `entity/banner/<pattern>` sheet tinted by its dye — `DyeColor.getTextureDiffuseColor`).
    /// Instances are projected from banner block states (`entity_id` is a sentinel). `body_rot`
    /// carries the `-angle` degrees of `BannerRenderer.modelTransformation`
    /// (`Axis.YP.rotationDegrees(-angle)`; ground `RotationSegment.convertToDegrees(ROTATION)`,
    /// wall `FACING.toYRot()`); the flag swing phase rides
    /// `EntityRenderState.banner_flag_phase`. The layer array clamps at the vanilla
    /// `BannerRenderer.MAX_PATTERNS = 16` render cap, `None` past the stored stack.
    Banner {
        wall: bool,
        /// GUI Loom result previews submit only the flag model; world block
        /// entities submit the standing/wall frame first.
        flag_only: bool,
        base_color: EntityDyeColor,
        layers: [Option<BannerPatternLayer>; 16],
    },
    /// One 1x1 segment of vanilla 26.1 `PaintingRenderer` custom geometry. Native expands a
    /// painting entity into `width * height` instances so each segment retains the independently
    /// sampled `PaintingRenderState.lightCoordsPerBlock` value without bloating every entity
    /// instance. `front` is the variant asset in the synchronized `paintings` atlas; `back` is
    /// that atlas's `minecraft:back` sprite. Boundary segments add the corresponding thin edge.
    PaintingTile {
        width: i32,
        height: i32,
        segment_x: i32,
        segment_y: i32,
        facing: EntityAttachmentFace,
        front: EntityDynamicTexture,
        back: EntityDynamicTexture,
    },
    /// Vanilla 26.1 `ExperienceOrbRenderer`: a camera-facing quad on the 4×4 icon sheet
    /// `entity/experience/experience_orb.png`, submitted as
    /// `RenderTypes.entityTranslucentCullItemTarget` with the red/blue sine tint cycling over the
    /// orb's age. `icon` is `ExperienceOrb.getIcon()` derived from the synced orb value.
    ExperienceOrb {
        icon: i32,
    },
    /// Vanilla 26.1 `DragonFireballRenderer`: a camera-facing quad over the full
    /// `entity/enderdragon/dragon_fireball.png` sheet at scale 2, submitted as
    /// `RenderTypes.entityCutout`. Its constant `getBlockLightLevel` = 15 rides the generated
    /// static-full-bright renderer catalog.
    DragonFireball,
    /// Vanilla 26.1 `FishingHookRenderer`: a camera-facing half-scale quad on
    /// `entity/fishing/fishing_hook.png`, plus the 16-segment black parabola from
    /// the hook to its owning player's rendered hand. The line origin offset is
    /// carried by `EntityRenderState.fishing_line_origin_offset`.
    FishingHook,
    /// Vanilla 26.1 `BookModel` (`ModelLayers.BOOK`) submitted by the
    /// `EnchantTableRenderer` block-entity renderer — the enchanting table's
    /// hovering, page-flipping book on the `entity/enchantment/
    /// enchanting_table_book` sheet. Instances are projected from enchanting
    /// table block states, not the entity list (`entity_id` is a sentinel). The
    /// book turns to face the nearest player, bobs, opens/closes, and flips its
    /// pages: `body_rot` carries the lerped book yaw (degrees), the hover offset
    /// rides `EntityRenderState.book_float_y`, and the `BookModel.setupAnim`
    /// animation reads `book_progress` / `book_open` / `book_page_flip_1` /
    /// `book_page_flip_2`.
    EnchantingBook,
    /// Vanilla 26.1 `BookModel` submitted by the `LecternRenderer` block-entity
    /// renderer — the lectern's static open book on the same
    /// `enchanting_table_book` sheet, rendered only while the block's `HAS_BOOK`
    /// state is set. Instances are projected from lectern block states
    /// (`entity_id` is a sentinel). `body_rot` carries the
    /// `FACING.getClockWise().toYRot()` yaw; the fixed
    /// `BookModel.State.forAnimation(0, 0.1, 0.9, 1.2)` openness / page flips
    /// ride the shared book render state.
    LecternBook,
    /// `ArrowModel` at its `createBodyLayer` rest pose (the arrowhead plane plus the two crossed
    /// fletching planes, the whole mesh scaled 0.9). `texture` picks the normal / tipped / spectral
    /// image (`TippableArrowRenderer` swaps to `arrow_tipped.png` when the arrow carries a potion;
    /// the spectral arrow binds `arrow_spectral.png`). Projected shake time applies the impact
    /// wobble and `ArrowRenderer` orientation follows flight.
    Arrow {
        texture: ArrowModelTexture,
    },
    /// `TridentModel` — the pole, crossguard, and three spikes. The model has no animation, so the
    /// geometry and base texture are complete; foil overlay submits and folded GPU glint geometry use
    /// the `entityGlint` render type.
    /// `ThrownTridentRenderer` orients it along its flight.
    Trident,
    /// `SkullModel` at `WitherSkullRenderer.createSkullLayer` — a single 8×8×8 `head` box. `setupAnim`
    /// turns the head by the projectile's flight `yRot`/`xRot` (reproduced via the root transform, since
    /// the part sits at ZERO). `WitherSkullRenderer` orients it along its flight; `dangerous` selects
    /// vanilla `wither_invulnerable.png` instead of the normal `wither.png`.
    WitherSkull {
        dangerous: bool,
    },
    /// `LlamaSpitModel` — a single `main` part of seven 2×2×2 boxes forming a cross. The model has
    /// no `setupAnim`, so the geometry is complete; only the texture-backed path is deferred.
    /// `LlamaSpitRenderer` orients it along its flight.
    LlamaSpit,
    /// `ShulkerBulletModel` — a single `main` part of three interlocking slabs. `setupAnim` orients
    /// it by the bullet's facing (reproduced). `ShulkerBulletRenderer`'s age-driven tumble and
    /// translucent 1.5× outer-shell pass are reproduced on the textured path.
    ShulkerBullet,
    /// `WindChargeModel` — the `bone` root parenting the `wind` shell (a fixed `-π/4` bind rotation,
    /// two boxes) and the `wind_charge` core box. Shared by the wind charge and breeze wind charge.
    /// `setupAnim` counter-rotates the shell/core, and the textured path submits the translucent
    /// scrolling `breezeWind` layer.
    WindCharge,
    /// `EnderDragonModel` (head + jaw, five neck and twelve tail segments, body with wings and four
    /// legs). Procedural `setupAnim` consumes flight history, flap time, phase, and exact podium
    /// distance for neck/tail placement, wings, jaw, and root bounce; dying dissolve, death rays,
    /// nearest-crystal beam, and emissive eyes use their dedicated submissions.
    EnderDragon,
    /// Entities whose vanilla renderer is `NoopRenderer` — the area effect cloud (its potion cloud
    /// is particles, not a model), the marker (a pure data entity), and the interaction (an
    /// invisible hitbox). Vanilla renders no model for these, so this kind emits no geometry, which
    /// is exact parity (not a deferral).
    NoRender,
    Phantom {
        size: i32,
    },
    Pufferfish {
        puff_state: i32,
    },
    Zombie {
        baby: bool,
    },
    ZombieVariant {
        family: ZombieVariantModelFamily,
        baby: bool,
    },
    Piglin {
        family: PiglinModelFamily,
        baby: bool,
    },
    Hoglin {
        family: HoglinModelFamily,
        baby: bool,
    },
    Ravager,
    Skeleton,
    SkeletonVariant {
        family: SkeletonModelFamily,
    },
    Cow {
        variant: CowModelVariant,
        baby: bool,
    },
    /// The mooshroom (`MushroomCow`) body, which vanilla `MushroomCowRenderer` renders with the shared
    /// `CowModel` / `BabyCowModel` mesh (`ModelLayers.MOOSHROOM` bakes to the same `cowBodyLayer` as the
    /// temperate cow, `MOOSHROOM_BABY` to `BabyCowModel.createBodyLayer()`). Rendered on the colored path
    /// as the temperate [`Cow`](EntityModelKind::Cow) geometry (`baby` selecting the baby layout), so it
    /// is the real cow body rather than the generic quadruped stand-in. `variant` drives the vanilla
    /// `MushroomCowRenderer` red/brown body-texture swap (`mooshroom_<variant>[_baby].png`) and the
    /// adult-only `MushroomCowMushroomLayer` block-model variant.
    Mooshroom {
        baby: bool,
        variant: MooshroomVariant,
    },
    Sheep {
        baby: bool,
        sheared: bool,
        wool_color: SheepWoolColor,
        jeb: bool,
        age_ticks: f32,
    },
    Villager {
        baby: bool,
    },
    WanderingTrader,
    Wolf {
        baby: bool,
        tame: bool,
        angry: bool,
        collar_color: Option<EntityDyeColor>,
        variant: WolfModelVariant,
    },
    Horse {
        baby: bool,
        variant: HorseColorVariant,
        markings: HorseMarkings,
    },
    Donkey {
        family: DonkeyModelFamily,
        baby: bool,
        has_chest: bool,
    },
    UndeadHorse {
        family: UndeadHorseModelFamily,
        baby: bool,
    },
    Camel {
        family: CamelModelFamily,
        baby: bool,
    },
    Llama {
        family: LlamaModelFamily,
        variant: LlamaVariant,
        baby: bool,
        has_chest: bool,
    },
    Goat {
        baby: bool,
        left_horn: bool,
        right_horn: bool,
    },
    PolarBear {
        baby: bool,
    },
    /// `PandaModel` / `BabyPandaModel` (both `QuadrupedModel`s) at their `createBodyLayer` rest pose
    /// (`baby` selects the baby body layout — body-first, no body pitch). The shared
    /// `QuadrupedModel.setupAnim` head look and four-leg walk swing are reproduced; projected unhappy,
    /// sneeze, sit/eat/scared, lie-on-back, and roll state drives every panda-specific branch. The
    /// gene-driven base texture uses displayed `Panda.Gene` (`variant`) across the 7-gene × age matrix.
    Panda {
        baby: bool,
        variant: PandaModelVariant,
    },
    /// `AdultFelineModel` / `BabyFelineModel` at their `createBodyMesh` rest pose, shared by the ocelot
    /// (`cat = false`) and the cat (`cat = true`). The adult cat scales the shared adult mesh 0.8 (via
    /// `AdultCatModel.CAT_TRANSFORMER` in the root transform); the ocelot and both babies are unscaled.
    /// `baby` selects the flatter `BabyFelineModel` layout. The shared `setupAnim` head look is
    /// reproduced, plus the bespoke crouch/sprint body-tail setup, walk leg swing, and adult lower-tail
    /// droop/wobble (`tail2.xRot = 1.7278761 + amplitude·cos(walkAnimationPos)·walkAnimationSpeed`,
    /// with standing `π/4`, crouch `0.47123894`, and sprint `π/10`; the baby's `tail2` is cubeless, so
    /// vanilla's identical assignment is invisible). Cat-only sitting, lie-down, lie-down-tail, and
    /// relax-head amount branches are projected from `FelineRenderState` and applied by the model;
    /// `CatRenderer.setupRotations`'s lie-down root translate / roll and sleeping-player extra
    /// translate are applied before the model flip and adult-cat scale. The
    /// textured path binds the cat-breed texture
    /// (`cat_variant`, the eleven `CatVariant`s)
    /// for cats and the `ocelot` texture for ocelots (`cat_variant` is ignored when `!cat`). `collar`
    /// is the tame cat's dyed collar overlay (`CatCollarLayer`, `Some` only for a tame cat — the ocelot
    /// never has one).
    Feline {
        cat: bool,
        baby: bool,
        cat_variant: CatModelVariant,
        collar: Option<EntityDyeColor>,
    },
    /// `AdultFoxModel` / `BabyFoxModel` (custom `EntityModel`s) at their `createBodyLayer` rest pose
    /// (`baby` selects the baby body layout). The exact walking, crouching, sleeping, sitting,
    /// pouncing, and faceplanted branches plus head-roll interpolation are reproduced, and the
    /// texture switches on the `variant` (red/snow) × age × sleeping matrix.
    Fox {
        baby: bool,
        variant: FoxModelVariant,
    },
    /// `NautilusModel` (a custom `EntityModel`, the new rideable nautilus) at its `createBodyMesh` /
    /// `createBabyBodyLayer` rest pose (`baby` selects the smaller hatchling geometry, the same
    /// `root → shell + body → mouths` structure). The clamped ±10° body look and looping
    /// `NautilusAnimation.SWIMMING` definition are reproduced. Body armor remains an equipment-layer
    /// gap; the adult saddle equipment layer is driven through render state.
    Nautilus {
        baby: bool,
    },
    /// `ZombieNautilusRenderer` (a plain `MobRenderer`, never a baby), selected by the synced
    /// `ZombieNautilusVariant` holder. The known vanilla `TEMPERATE` identity reuses the living adult
    /// `NautilusModel` body textured by `zombie_nautilus.png`; `WARM` renders the
    /// `ZombieNautilusCoralModel` (the same body plus the `corals` subtree) over
    /// `zombie_nautilus_coral.png`. The synchronized registry element separately owns the authoritative
    /// model and texture payload. The adult saddle equipment layer is driven through render state; the
    /// body armor equipment layer stays deferred.
    ZombieNautilus {
        variant: ZombieNautilusModelVariant,
    },
    /// `AdultRabbitModel` / `BabyRabbitModel` at their `createBodyLayer` rest pose (`baby` selects the
    /// baby body layout — a deeper `_r1`-nested hierarchy whose head is `body`'s third child),
    /// textured by the seven `Rabbit.Variant` colours × age, with the `toast` named-rabbit override
    /// (`RabbitRenderer.getTextureLocation`). `RabbitModel.setupAnim` turns the head and applies the
    /// event-driven adult/baby HOP definition. `IDLE_HEAD_TILT` starts from client-local entity RNG,
    /// so a remote official-client phase is intentionally not fabricated.
    Rabbit {
        baby: bool,
        variant: RabbitModelVariant,
        toast: bool,
    },
    Quadruped {
        family: QuadrupedModelFamily,
        baby: bool,
    },
    Creeper,
    Spider,
    CaveSpider,
    Enderman,
    /// `CopperGolemModel` at its vanilla `createBodyLayer` rest pose, textured by
    /// `WeatheringCopper.WeatherState` (`CopperGolemRenderer.getTextureLocation`) and re-rendered
    /// with the matching emissive eyes texture through vanilla `LivingEntityEmissiveLayer`.
    /// Walking, walking-with-item, idle, and all four chest-interaction keyframes are projected. The
    /// standard held-item and antenna block attachment use typed layers; custom-head presentation is
    /// owned separately from model animation.
    CopperGolem {
        weathering: CopperGolemWeathering,
    },
    /// Iron golem (`IronGolemModel`, `IronGolemRenderer`). `crackiness` drives the vanilla
    /// `IronGolemCrackinessLayer` damage-crack overlay (`iron_golem_crackiness_{low,medium,high}.png`),
    /// selected by `IronGolem.getCrackiness()`. Attack and offer-flower arm poses are render-state
    /// driven, the renderer walk wobble is applied at the root, and the held poppy is rendered by the
    /// entity-attached block-model path.
    IronGolem {
        crackiness: IronGolemCrackiness,
    },
    SnowGolem,
    Witch,
    /// Squid and glow squid (`SquidModel`, `SquidRenderer` / `GlowSquidRenderer`).
    /// `glow` selects the glow-squid texture/color variant; `baby` selects the
    /// `BABY_TRANSFORMER` 0.5-scaled body layer.
    Squid {
        glow: bool,
        baby: bool,
    },
    /// Cod (`CodModel`, `CodRenderer`). The tail-fin sway and the
    /// `CodRenderer.setupRotations` body wiggle / out-of-water flop read
    /// `EntityRenderState.in_water` and `age_in_ticks`.
    Cod,
    /// Salmon (`SalmonModel`, `SalmonRenderer`). `size` selects the small/medium/large
    /// `MeshTransformer`-scaled body layer (the medium layer is the unscaled base). The
    /// body-back sway and `SalmonRenderer.setupRotations` wiggle / out-of-water flop read
    /// `EntityRenderState.in_water` and `age_in_ticks`.
    Salmon {
        size: SalmonModelSize,
    },
    /// Tropical fish (`TropicalFishSmallModel`/`TropicalFishLargeModel`,
    /// `TropicalFishRenderer`). `shape` selects the kob-style small body or the
    /// flopper-style large body (vanilla `TropicalFish.Pattern.base()`). The tail sway and
    /// `TropicalFishRenderer.setupRotations` wiggle / out-of-water flop read
    /// `EntityRenderState.in_water` and `age_in_ticks`. `base_color` is the body tint
    /// (vanilla `getModelTint` = `getBaseColor().getTextureDiffuseColor()`); `pattern` selects
    /// the `TropicalFishPatternLayer` overlay and `pattern_color` tints it
    /// (`getPatternColor().getTextureDiffuseColor()`). All three are decoded from the same
    /// synced packed variant, so `shape == pattern.shape()` always holds.
    TropicalFish {
        shape: TropicalFishModelShape,
        base_color: EntityDyeColor,
        pattern: TropicalFishPattern,
        pattern_color: EntityDyeColor,
    },
    Illager {
        family: IllagerModelFamily,
    },
    Minecart,
    Boat {
        family: BoatModelFamily,
        chest: bool,
    },
    Placeholder {
        name: &'static str,
        bounds: EntityModelBounds,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConduitModelPart {
    Shell,
    Cage,
    OuterWind { phase: u8 },
    InnerWind { vertical: bool },
    Eye { open: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerModelPartVisibility {
    pub cape: bool,
    pub jacket: bool,
    pub left_sleeve: bool,
    pub right_sleeve: bool,
    pub left_pants: bool,
    pub right_pants: bool,
    pub hat: bool,
}

impl PlayerModelPartVisibility {
    pub const CAPE_MASK: u8 = 1 << 0;
    pub const JACKET_MASK: u8 = 1 << 1;
    pub const LEFT_SLEEVE_MASK: u8 = 1 << 2;
    pub const RIGHT_SLEEVE_MASK: u8 = 1 << 3;
    pub const LEFT_PANTS_MASK: u8 = 1 << 4;
    pub const RIGHT_PANTS_MASK: u8 = 1 << 5;
    pub const HAT_MASK: u8 = 1 << 6;
    pub const ALL_MASK: u8 = Self::CAPE_MASK
        | Self::JACKET_MASK
        | Self::LEFT_SLEEVE_MASK
        | Self::RIGHT_SLEEVE_MASK
        | Self::LEFT_PANTS_MASK
        | Self::RIGHT_PANTS_MASK
        | Self::HAT_MASK;

    pub const fn from_vanilla_mask(mask: u8) -> Self {
        Self {
            cape: (mask & Self::CAPE_MASK) == Self::CAPE_MASK,
            jacket: (mask & Self::JACKET_MASK) == Self::JACKET_MASK,
            left_sleeve: (mask & Self::LEFT_SLEEVE_MASK) == Self::LEFT_SLEEVE_MASK,
            right_sleeve: (mask & Self::RIGHT_SLEEVE_MASK) == Self::RIGHT_SLEEVE_MASK,
            left_pants: (mask & Self::LEFT_PANTS_MASK) == Self::LEFT_PANTS_MASK,
            right_pants: (mask & Self::RIGHT_PANTS_MASK) == Self::RIGHT_PANTS_MASK,
            hat: (mask & Self::HAT_MASK) == Self::HAT_MASK,
        }
    }

    pub const fn vanilla_mask(self) -> u8 {
        (if self.cape { Self::CAPE_MASK } else { 0 })
            | (if self.jacket { Self::JACKET_MASK } else { 0 })
            | (if self.left_sleeve {
                Self::LEFT_SLEEVE_MASK
            } else {
                0
            })
            | (if self.right_sleeve {
                Self::RIGHT_SLEEVE_MASK
            } else {
                0
            })
            | (if self.left_pants {
                Self::LEFT_PANTS_MASK
            } else {
                0
            })
            | (if self.right_pants {
                Self::RIGHT_PANTS_MASK
            } else {
                0
            })
            | (if self.hat { Self::HAT_MASK } else { 0 })
    }
}

pub const PLAYER_MODEL_PARTS_ALL_VISIBLE: PlayerModelPartVisibility =
    PlayerModelPartVisibility::from_vanilla_mask(PlayerModelPartVisibility::ALL_MASK);
pub const PLAYER_MODEL_PARTS_ALL_HIDDEN: PlayerModelPartVisibility =
    PlayerModelPartVisibility::from_vanilla_mask(0);

/// One first-person local-player arm submission, matching vanilla
/// `ItemInHandRenderer.renderPlayerArm` / `renderMapHand` plus
/// `AvatarRenderer.renderRightHand` / `renderLeftHand`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstPersonPlayerArm {
    pub left: bool,
    pub skin: EntityPlayerSkin,
    pub sleeve_visible: bool,
    pub transform: Mat4,
    pub light: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZombieVariantModelFamily {
    Husk,
    Drowned,
    ZombieVillager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillagerModelHat {
    None,
    Partial,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VillagerModelData {
    pub villager_type: VillagerModelType,
    pub profession: VillagerModelProfession,
    pub level: i32,
}

impl VillagerModelData {
    pub const DEFAULT: Self = Self {
        villager_type: VillagerModelType::Plains,
        profession: VillagerModelProfession::None,
        level: 1,
    };

    pub const fn new(
        villager_type: VillagerModelType,
        profession: VillagerModelProfession,
        level: i32,
    ) -> Self {
        Self {
            villager_type,
            profession,
            level,
        }
    }
}

pub(in crate::entity_models) const fn villager_type_hat(
    villager_type: VillagerModelType,
) -> VillagerModelHat {
    match villager_type {
        // Vanilla villager type mcmeta declares `hat: full` only for desert and snow.
        VillagerModelType::Desert | VillagerModelType::Snow => VillagerModelHat::Full,
        VillagerModelType::Jungle
        | VillagerModelType::Plains
        | VillagerModelType::Savanna
        | VillagerModelType::Swamp
        | VillagerModelType::Taiga => VillagerModelHat::None,
    }
}

pub(in crate::entity_models) const fn villager_profession_hat(
    profession: VillagerModelProfession,
) -> VillagerModelHat {
    match profession {
        // Vanilla profession mcmeta declares butcher as partial and these six as full.
        VillagerModelProfession::Butcher => VillagerModelHat::Partial,
        VillagerModelProfession::Farmer
        | VillagerModelProfession::Fisherman
        | VillagerModelProfession::Fletcher
        | VillagerModelProfession::Librarian
        | VillagerModelProfession::Shepherd => VillagerModelHat::Full,
        VillagerModelProfession::None
        | VillagerModelProfession::Armorer
        | VillagerModelProfession::Cartographer
        | VillagerModelProfession::Cleric
        | VillagerModelProfession::Leatherworker
        | VillagerModelProfession::Mason
        | VillagerModelProfession::Nitwit
        | VillagerModelProfession::Toolsmith
        | VillagerModelProfession::Weaponsmith => VillagerModelHat::None,
    }
}

pub(in crate::entity_models) const fn villager_profession_renders_level_badge(
    profession: VillagerModelProfession,
) -> bool {
    !matches!(
        profession,
        VillagerModelProfession::None | VillagerModelProfession::Nitwit
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiglinModelFamily {
    Piglin,
    PiglinBrute,
    ZombifiedPiglin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoglinModelFamily {
    Hoglin,
    Zoglin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanoidModelFamily {
    Player,
    Zombie,
    Skeleton,
    Villager,
    Illager,
    ArmorStand,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmorStandModelPose {
    pub head: [f32; 3],
    pub body: [f32; 3],
    pub left_arm: [f32; 3],
    pub right_arm: [f32; 3],
    pub left_leg: [f32; 3],
    pub right_leg: [f32; 3],
}

pub const DEFAULT_ARMOR_STAND_MODEL_POSE: ArmorStandModelPose = ArmorStandModelPose {
    head: [0.0, 0.0, 0.0],
    body: [0.0, 0.0, 0.0],
    left_arm: [-10.0, 0.0, -10.0],
    right_arm: [-15.0, 0.0, 10.0],
    left_leg: [-1.0, 0.0, -1.0],
    right_leg: [1.0, 0.0, 1.0],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonModelFamily {
    Stray,
    Parched,
    WitherSkeleton,
    Bogged { sheared: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllagerModelFamily {
    Evoker,
    Illusioner,
    Pillager,
    Vindicator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuadrupedModelFamily {
    Pig,
    Cow,
    Sheep,
    Horse,
    Wolf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonkeyModelFamily {
    Donkey,
    Mule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndeadHorseModelFamily {
    Skeleton,
    Zombie,
}

/// The living horse's coat color (vanilla `equine.Variant`). `getTextureLocation` picks one of seven
/// `horse_<color>(_baby).png` base coats; the markings overlay (`HorseMarkingLayer`) is a separate
/// deferred layer. The shared `HorseModel` geometry is variant-agnostic, so this only chooses the
/// texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorseColorVariant {
    White,
    Creamy,
    Chestnut,
    Brown,
    Black,
    Gray,
    DarkBrown,
}

/// The living horse's white-markings overlay (vanilla `equine.Markings`). `HorseMarkingLayer` draws a
/// translucent `horse_markings_*(_baby).png` on top of the base coat; `None` maps to vanilla's
/// `INVISIBLE_TEXTURE` (no overlay). Selected from the `(DATA_ID_TYPE_VARIANT & 0xFF00) >> 8` nibble.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorseMarkings {
    None,
    White,
    WhiteField,
    WhiteDots,
    BlackDots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CamelModelFamily {
    Camel,
    CamelHusk,
}

/// A humanoid armor material (vanilla `ArmorMaterials.<MAT>` → `EquipmentAssets.<MAT>`), selecting the
/// `HumanoidArmorLayer` equipment-asset textures. Projected per slot onto the entity render state from
/// the worn item's armor material; `TurtleScute` only ever fills the head slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityArmorMaterial {
    Leather,
    Copper,
    Chainmail,
    Iron,
    Gold,
    Diamond,
    TurtleScute,
    Netherite,
    ArmadilloScute,
}

/// Vanilla `Crackiness.Level` for wolf armor (`Crackiness.WOLF_ARMOR.byDamage`): selected by the
/// body armor item's remaining durability and rendered as an `armorTranslucent` overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WolfArmorCrackiness {
    Low,
    Medium,
    High,
}

/// Vanilla `GuardianRenderer` attack-beam render state (`GuardianRenderState.attackTargetPosition`
/// present): a guardian firing its beam at a target. `eye_to_target` is the world-space vector from the
/// guardian's eye to the target center (`attackTargetPosition − eyePosition`, vanilla `beamVector`);
/// `eye_height` raises the beam origin from the entity feet to the eye; `attack_time` is the lerped
/// `clientSideAttackTime + partialTicks` (drives the texture V-scroll and the prism twist); and
/// `attack_scale` is `getAttackAnimationScale` (`0..1`, drives the beam color ramp). Projected as
/// `Option`, `None` for a guardian with no active target and every other entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuardianBeamRenderState {
    pub eye_to_target: [f32; 3],
    pub eye_height: f32,
    pub attack_time: f32,
    pub attack_scale: f32,
}

/// Vanilla `GuardianRenderState` eye-tracking inputs. `GuardianModel.setupAnim` keeps these in
/// world-space doubles while deriving the eye part's horizontal and vertical model-space offsets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuardianEyeTrackingRenderState {
    pub eye_position: [f64; 3],
    pub look_direction: [f64; 3],
    pub look_at_position: [f64; 3],
}

/// Vanilla `EndCrystalRenderState.beamOffset`: target block center relative to the crystal position.
/// The renderer combines this with the crystal's `ageInTicks` bob (`EndCrystalRenderer.getY`) when
/// submitting the `endCrystalBeam` custom geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndCrystalBeamRenderState {
    pub beam_offset: [f32; 3],
}

/// Vanilla `EnderDragonRenderState.beamOffset`: nearest end-crystal position (including the crystal
/// bob from `EndCrystalRenderer.getY`) relative to the dragon position. The dragon renderer submits the
/// same `endCrystalBeam` custom geometry after the body and eyes passes when this is present.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnderDragonBeamRenderState {
    pub beam_offset: [f32; 3],
}

/// One interpolated `DragonFlightHistory.Sample` (`y` world height, `y_rot` body yaw degrees).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnderDragonFlightSample {
    pub y: f64,
    pub y_rot: f32,
}

/// Vanilla `EnderDragonRenderState`'s procedural-animation inputs for `EnderDragonModel.setupAnim`
/// and the renderer's flight-history root yaw/pitch. `historical` carries `getHistoricalPos(delay)`
/// pre-interpolated for every delay the model samples (`0..=23`), with the dead-dragon
/// `partialTicks = 0` freeze already applied by the world projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnderDragonAnimationRenderState {
    pub flap_time: f32,
    pub is_landing_or_taking_off: bool,
    pub is_sitting: bool,
    pub distance_to_egg: f64,
    pub historical: [EnderDragonFlightSample; 24],
}

impl EnderDragonAnimationRenderState {
    /// Vanilla `EnderDragonRenderState.getHistoricalPos(delay)` over the projected samples.
    pub fn historical_pos(&self, delay: usize) -> EnderDragonFlightSample {
        self.historical[delay]
    }

    /// Vanilla `EnderDragonRenderState.getHeadPartYOffset(part, bodyPos, partPos)`: the landing /
    /// takeoff droop divides the part index by the egg distance, sitting droops linearly, the head
    /// itself (`part == 6`) stays level, and free flight reads the sample height difference.
    pub fn head_part_y_offset(
        &self,
        part: i32,
        body_pos: EnderDragonFlightSample,
        part_pos: EnderDragonFlightSample,
    ) -> f32 {
        let result = if self.is_landing_or_taking_off {
            f64::from(part) / (self.distance_to_egg / 4.0).max(1.0)
        } else if self.is_sitting {
            f64::from(part)
        } else if part == 6 {
            0.0
        } else {
            part_pos.y - body_pos.y
        };
        result as f32
    }
}

/// One rope of a leash connection (vanilla `EntityRenderState.LeashState`): `offset` is the rotated
/// attach offset (the rope start relative to the entity render position), `delta` is the world-space
/// `end − start` span (`LeashFeatureRenderer.renderLeash`'s `dx/dy/dz` after its `float` casts), the
/// endpoint block/sky lights feed the per-step light lerp, and `slack` selects the sag curve over
/// the taut line a quad connection uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeashRopeRenderState {
    pub offset: [f32; 3],
    pub delta: [f32; 3],
    pub start_block_light: u8,
    pub start_sky_light: u8,
    pub end_block_light: u8,
    pub end_sky_light: u8,
    pub slack: bool,
}

/// Vanilla `EntityRenderState.leashStates`: the ropes attached to a leashed entity — one for a
/// normal leash, four for a ghast-to-boat quad connection. Fixed-size because `EntityRenderState`
/// is `Copy`; `count` bounds the live prefix of `ropes`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityLeashRopesRenderState {
    pub ropes: [LeashRopeRenderState; 4],
    pub count: u8,
}

impl EntityLeashRopesRenderState {
    /// Packs up to four ropes; `None` for an empty slice (an unleashed entity).
    pub fn from_ropes(source: &[LeashRopeRenderState]) -> Option<Self> {
        let (&first, _) = source.split_first()?;
        let count = source.len().min(4);
        let mut ropes = [first; 4];
        ropes[..count].copy_from_slice(&source[..count]);
        Some(Self {
            ropes,
            count: count as u8,
        })
    }

    /// The live ropes.
    pub fn ropes(&self) -> &[LeashRopeRenderState] {
        &self.ropes[..usize::from(self.count).min(self.ropes.len())]
    }
}

/// Vanilla `TheEndGatewayRenderer` beam render state. `scale` is the
/// `sin(percent * PI)` envelope, `height` is the positive block distance used
/// to submit a beam from `-height` to `+height`, `color_argb` is magenta while
/// spawning or purple while cooling down, and `animation_time` is
/// `floorMod(gameTime, 40) + partialTick`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndGatewayBeamRenderState {
    pub scale: f32,
    pub height: i32,
    pub color_argb: u32,
    pub animation_time: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaModelFamily {
    Llama,
    TraderLlama,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaVariant {
    Creamy,
    White,
    Brown,
    Gray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SalmonModelSize {
    Small,
    Medium,
    Large,
}

impl SalmonModelSize {
    /// Vanilla `Salmon.Variant` ids (`SMALL=0`, `MEDIUM=1`, `LARGE=2`), clamped like
    /// `ByIdMap.continuous(..., CLAMP)`.
    pub fn from_vanilla_id(id: i32) -> Self {
        match id {
            i32::MIN..=0 => Self::Small,
            1 => Self::Medium,
            _ => Self::Large,
        }
    }

    /// Vanilla `SalmonModel` `MeshTransformer` scale: small `0.5`, medium `1.0` (the
    /// unscaled base layer), large `1.5`.
    pub fn scale(self) -> f32 {
        match self {
            Self::Small => 0.5,
            Self::Medium => 1.0,
            Self::Large => 1.5,
        }
    }
}

/// Vanilla `TropicalFish.Base` body shape (`TropicalFish.Pattern.base()`): the kob-style
/// `Small` body (`TropicalFishSmallModel`) or the flopper-style `Large` body
/// (`TropicalFishLargeModel`). Each of the twelve patterns maps to one of these two
/// shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalFishModelShape {
    Small,
    Large,
}

impl TropicalFishModelShape {
    /// Vanilla `TropicalFish.Base` ids (`SMALL=0`, `LARGE=1`). Any other id falls back to
    /// the small body, matching the pattern-decode default.
    pub fn from_vanilla_base_id(id: i32) -> Self {
        match id {
            1 => Self::Large,
            _ => Self::Small,
        }
    }

    /// Vanilla `TropicalFish.Pattern.base()` for an already decoded pattern id. The sparse
    /// pattern ids are packed as `base.id | index << 8`, base `0`/`1`, index `0..=5`, and
    /// default to `KOB` (small) when unrecognized. So the shape is `Large`
    /// only when the low pattern byte is `1` (a `LARGE` base) and the index byte is in
    /// range; every other pattern id — including the default `0` (`KOB`) —
    /// resolves to the small body.
    pub fn from_vanilla_pattern_id(pattern_id: i32) -> Self {
        let base_id = pattern_id & 0xFF;
        let index = (pattern_id >> 8) & 0xFF;
        if base_id == 1 && index <= 5 {
            Self::Large
        } else {
            Self::Small
        }
    }
}

/// Vanilla `TropicalFish.Pattern`: the twelve named patterns, six on the kob-style `Small`
/// body and six on the flopper-style `Large` body, selecting the
/// `TropicalFishPatternLayer` overlay texture. Each pattern is packed as
/// `base.id | index << 8` (base `SMALL=0`/`LARGE=1`, index `0..=5`) in the low 16 bits of
/// the synced variant (`TropicalFish.getPattern(packed & 0xFFFF)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalFishPattern {
    Kob,
    Sunstreak,
    Snooper,
    Dasher,
    Brinely,
    Spotty,
    Flopper,
    Stripey,
    Glitter,
    Blockfish,
    Betty,
    Clayfish,
}

impl TropicalFishPattern {
    /// Vanilla `TropicalFish.Pattern.byId(patternId)`, a sparse lookup over the twelve patterns
    /// keyed on `packedId = base.id | index << 8` (so `KOB=0`, `SUNSTREAK=256`, …,
    /// `FLOPPER=1`, `STRIPEY=257`, …). Any unrecognized id falls back to `KOB`, exactly like
    /// `ByIdMap.sparse(..., KOB)`.
    pub fn from_vanilla_pattern_id(pattern_id: i32) -> Self {
        match pattern_id {
            0 => Self::Kob,
            256 => Self::Sunstreak,
            512 => Self::Snooper,
            768 => Self::Dasher,
            1024 => Self::Brinely,
            1280 => Self::Spotty,
            1 => Self::Flopper,
            257 => Self::Stripey,
            513 => Self::Glitter,
            769 => Self::Blockfish,
            1025 => Self::Betty,
            1281 => Self::Clayfish,
            _ => Self::Kob,
        }
    }

    /// Vanilla `TropicalFish.Pattern.base()`: the first six patterns ride the kob-style
    /// `Small` body, the last six the flopper-style `Large` body.
    pub fn shape(self) -> TropicalFishModelShape {
        match self {
            Self::Kob
            | Self::Sunstreak
            | Self::Snooper
            | Self::Dasher
            | Self::Brinely
            | Self::Spotty => TropicalFishModelShape::Small,
            Self::Flopper
            | Self::Stripey
            | Self::Glitter
            | Self::Blockfish
            | Self::Betty
            | Self::Clayfish => TropicalFishModelShape::Large,
        }
    }

    /// The pattern's index within its base (`0..=5`); the `TropicalFishPatternLayer` texture
    /// is `tropical_{a,b}_pattern_{index + 1}.png`.
    pub fn pattern_index(self) -> u8 {
        match self {
            Self::Kob | Self::Flopper => 0,
            Self::Sunstreak | Self::Stripey => 1,
            Self::Snooper | Self::Glitter => 2,
            Self::Dasher | Self::Blockfish => 3,
            Self::Brinely | Self::Betty => 4,
            Self::Spotty | Self::Clayfish => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoatModelFamily {
    Acacia,
    Bamboo,
    Birch,
    Cherry,
    DarkOak,
    Jungle,
    Mangrove,
    Oak,
    PaleOak,
    Spruce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDyeColor {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

impl EntityDyeColor {
    pub fn from_vanilla_id(id: i32) -> Self {
        match id {
            0 => Self::White,
            1 => Self::Orange,
            2 => Self::Magenta,
            3 => Self::LightBlue,
            4 => Self::Yellow,
            5 => Self::Lime,
            6 => Self::Pink,
            7 => Self::Gray,
            8 => Self::LightGray,
            9 => Self::Cyan,
            10 => Self::Purple,
            11 => Self::Blue,
            12 => Self::Brown,
            13 => Self::Green,
            14 => Self::Red,
            15 => Self::Black,
            _ => Self::White,
        }
    }

    pub fn vanilla_id(self) -> i32 {
        match self {
            Self::White => 0,
            Self::Orange => 1,
            Self::Magenta => 2,
            Self::LightBlue => 3,
            Self::Yellow => 4,
            Self::Lime => 5,
            Self::Pink => 6,
            Self::Gray => 7,
            Self::LightGray => 8,
            Self::Cyan => 9,
            Self::Purple => 10,
            Self::Blue => 11,
            Self::Brown => 12,
            Self::Green => 13,
            Self::Red => 14,
            Self::Black => 15,
        }
    }

    pub fn texture_diffuse_color(self) -> [f32; 4] {
        let [red, green, blue] = match self {
            Self::White => [249, 255, 254],
            Self::Orange => [249, 128, 29],
            Self::Magenta => [199, 78, 189],
            Self::LightBlue => [58, 179, 218],
            Self::Yellow => [254, 216, 61],
            Self::Lime => [128, 199, 31],
            Self::Pink => [243, 139, 170],
            Self::Gray => [71, 79, 82],
            Self::LightGray => [157, 157, 151],
            Self::Cyan => [22, 156, 156],
            Self::Purple => [137, 50, 184],
            Self::Blue => [60, 68, 170],
            Self::Brown => [131, 84, 50],
            Self::Green => [94, 124, 22],
            Self::Red => [176, 46, 38],
            Self::Black => [29, 29, 33],
        };
        [
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            1.0,
        ]
    }
}

/// Vanilla `Crackiness.Level` for the iron golem (`IronGolem.getCrackiness()`): the damage-crack
/// overlay tier, selected by `Crackiness.GOLEM.byFraction(health / maxHealth)`. `None` draws no
/// overlay; `Low`/`Medium`/`High` overlay the matching `iron_golem_crackiness_*` texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IronGolemCrackiness {
    None,
    Low,
    Medium,
    High,
}

impl IronGolemCrackiness {
    /// Vanilla `Crackiness.GOLEM.byFraction` with `(fractionLow, fractionMedium, fractionHigh) =
    /// (0.75, 0.5, 0.25)`: a health fraction below `0.25` is `High`, below `0.5` `Medium`, below
    /// `0.75` `Low`, otherwise `None`.
    pub fn from_health_fraction(fraction: f32) -> Self {
        if fraction < 0.25 {
            Self::High
        } else if fraction < 0.5 {
            Self::Medium
        } else if fraction < 0.75 {
            Self::Low
        } else {
            Self::None
        }
    }
}

/// Vanilla `WeatheringCopper.WeatherState` for the copper golem's body and emissive-eye texture
/// selection. The stream codec maps ordinals 0..=3 with CLAMP, so out-of-range synced ids clamp to
/// the nearest weathering state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopperGolemWeathering {
    Unaffected,
    Exposed,
    Weathered,
    Oxidized,
}

impl CopperGolemWeathering {
    pub fn from_vanilla_id(id: i32) -> Self {
        match id {
            i32::MIN..=0 => Self::Unaffected,
            1 => Self::Exposed,
            2 => Self::Weathered,
            _ => Self::Oxidized,
        }
    }
}

/// Vanilla `MushroomCow.Variant` (the synced `MushroomCow.DATA_TYPE` int): the two mooshroom coats,
/// sharing the `CowModel` body and differing only by texture (`MushroomCowRenderer`). `Red` is the
/// vanilla `Variant.DEFAULT` (id 0); `Brown` is id 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MooshroomVariant {
    Red,
    Brown,
}

impl MooshroomVariant {
    /// Vanilla `MushroomCow.Variant.byId` (`ByIdMap.continuous(..., CLAMP)`): clamps the synced
    /// `DATA_TYPE` id, so `<= 0` resolves to `Red` (id 0) and `>= 1` to `Brown` (id 1).
    pub fn from_vanilla_id(id: i32) -> Self {
        if id >= 1 {
            Self::Brown
        } else {
            Self::Red
        }
    }

    /// Vanilla `StringRepresentable` name used by the save-data codec.
    pub const fn serialized_name(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Brown => "brown",
        }
    }
}

/// Vanilla `Parrot.Variant` (the synced `DATA_VARIANT_ID` int): the five parrot colours, sharing one
/// `ParrotModel` and differing only by texture (`ParrotRenderer.getVariantTexture`). `RED_BLUE` is the
/// vanilla `DEFAULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParrotModelVariant {
    RedBlue,
    Blue,
    Green,
    YellowBlue,
    Gray,
}

impl ParrotModelVariant {
    /// Vanilla `Parrot.Variant.byId`, backed by `ByIdMap.continuous(..., CLAMP)`.
    pub fn from_id(id: i32) -> Self {
        match id {
            i32::MIN..=0 => Self::RedBlue,
            1 => Self::Blue,
            2 => Self::Green,
            3 => Self::YellowBlue,
            4..=i32::MAX => Self::Gray,
        }
    }

    /// Vanilla `Parrot.Variant.id`, used by the save-data debug projection.
    pub const fn vanilla_id(self) -> i32 {
        match self {
            Self::RedBlue => 0,
            Self::Blue => 1,
            Self::Green => 2,
            Self::YellowBlue => 3,
            Self::Gray => 4,
        }
    }
}

/// Vanilla `Fox.Variant` (the synced `DATA_TYPE_ID` int): the two fox colours, sharing one
/// `FoxModel` and differing only by texture (`FoxRenderer.TEXTURES_BY_VARIANT`). `RED` is the
/// vanilla `DEFAULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoxModelVariant {
    Red,
    Snow,
}

impl FoxModelVariant {
    /// Vanilla `Fox.Variant.byId` (`ByIdMap.continuous` with `OutOfBoundsStrategy.ZERO`): maps the
    /// synced `DATA_TYPE_ID` int to a colour, folding out-of-range ids back to `RED`.
    pub fn from_id(id: i32) -> Self {
        match id {
            1 => Self::Snow,
            _ => Self::Red,
        }
    }

    /// Vanilla `StringRepresentable` name used by the save-data codec.
    pub const fn serialized_name(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Snow => "snow",
        }
    }
}

/// Vanilla `Axolotl.Variant` (the synced `DATA_VARIANT` int): the five axolotl colours, sharing one
/// `AxolotlModel` and differing by texture × age (`AxolotlRenderer.TEXTURE_BY_TYPE`). `LUCY` is the
/// vanilla `DEFAULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxolotlModelVariant {
    Lucy,
    Wild,
    Gold,
    Cyan,
    Blue,
}

impl AxolotlModelVariant {
    /// Vanilla `Axolotl.Variant.byId` (`ByIdMap.continuous` with `OutOfBoundsStrategy.ZERO`): maps
    /// the synced `DATA_VARIANT` int to a colour, folding out-of-range ids back to `LUCY`.
    pub fn from_id(id: i32) -> Self {
        match id {
            1 => Self::Wild,
            2 => Self::Gold,
            3 => Self::Cyan,
            4 => Self::Blue,
            _ => Self::Lucy,
        }
    }

    /// Vanilla `Axolotl.Variant.getId`, used by the save-data debug projection.
    pub const fn vanilla_id(self) -> i32 {
        match self {
            Self::Lucy => 0,
            Self::Wild => 1,
            Self::Gold => 2,
            Self::Cyan => 3,
            Self::Blue => 4,
        }
    }
}

/// Vanilla `Rabbit.Variant` (the synced `DATA_TYPE_ID` int): the seven rabbit colours, sharing one
/// `RabbitModel` and differing by texture × age (`RabbitRenderer.RABBIT_LOCATIONS`). `EVIL` (the
/// killer bunny) uses the `caerbannog` texture. `BROWN` is the vanilla `DEFAULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RabbitModelVariant {
    Brown,
    White,
    Black,
    WhiteSplotched,
    Gold,
    Salt,
    Evil,
}

impl RabbitModelVariant {
    /// Vanilla `Rabbit.Variant.byId` (`ByIdMap.sparse` over the discontinuous ids, defaulting to
    /// `BROWN`): note `EVIL` is id `99`, the others `0..=5`.
    pub fn from_id(id: i32) -> Self {
        match id {
            1 => Self::White,
            2 => Self::Black,
            3 => Self::WhiteSplotched,
            4 => Self::Gold,
            5 => Self::Salt,
            99 => Self::Evil,
            _ => Self::Brown,
        }
    }
}

/// Which arrow image to bind: the plain `arrow.png`, the `arrow_tipped.png` (`TippableArrowRenderer`
/// when the arrow carries a potion, `getColor() > 0`), or the spectral arrow's `arrow_spectral.png`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowModelTexture {
    Normal,
    Tipped,
    Spectral,
}

/// Vanilla `ChestType`: which of the three `ChestModel` meshes a chest block renders
/// (`MultiblockChestResources.select` — `single`, the double chest's `left`, or its `right` half).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChestModelHalf {
    Single,
    Left,
    Right,
}

/// Vanilla `ChestRenderState.ChestMaterialType`: the `Sheets.chooseSprite` texture family a chest
/// renders with. Each non-ender family carries `entity/chest/<prefix>{,_left,_right}.png`; the
/// ender chest is always single (`entity/chest/ender.png`). Native presentation owns the
/// date-gated selection of `Christmas`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChestModelTexture {
    Normal,
    Trapped,
    Christmas,
    Ender,
    Copper,
    CopperExposed,
    CopperWeathered,
    CopperOxidized,
}

/// Vanilla `WoodType.values()` families with sign blocks (26.1): the twelve
/// `entity/signs/<wood>.png` + `entity/signs/hanging/<wood>.png` sprite pairs
/// (`Sheets.SIGN_MAPPER` / `HANGING_SIGN_MAPPER`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignModelWood {
    Oak,
    Spruce,
    Birch,
    Acacia,
    Cherry,
    Jungle,
    DarkOak,
    PaleOak,
    Crimson,
    Warped,
    Mangrove,
    Bamboo,
}

/// The five sign meshes: vanilla `PlainSignBlock.Attachment` (GROUND / WALL —
/// `StandingSignRenderer.createSignLayer(standing)`) and
/// `HangingSignBlock.Attachment` (CEILING / CEILING_MIDDLE / WALL —
/// `HangingSignRenderer.createHangingSignLayer(attachment)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignModelAttachment {
    Standing,
    Wall,
    HangingCeiling,
    HangingCeilingMiddle,
    HangingWall,
}

impl SignModelAttachment {
    /// Whether this mesh is a hanging-sign form (`HangingSignRenderer`
    /// transforms, 60px text lines) rather than a plain sign
    /// (`StandingSignRenderer` transforms, 90px text lines).
    pub fn is_hanging(self) -> bool {
        matches!(
            self,
            Self::HangingCeiling | Self::HangingCeilingMiddle | Self::HangingWall
        )
    }
}

/// Vanilla `BedPart` (`BedBlock.PART`): which of the two `BedRenderer` meshes a bed block renders
/// (`createHeadLayer` / `createFootLayer` — each half is its own block and renders only its own
/// piece, `BedRenderer.submit` → `getPieceModel(part)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedModelPart {
    Head,
    Foot,
}

/// Vanilla `BellBlockEntity.clickDirection` (`Direction.from3DDataValue(b1)` of the
/// `BlockEvent(1, direction)` payload): the face the bell was rung from, deciding the
/// `BellModel.setupAnim` swing axis. `DOWN`/`UP` are representable on the wire but produce no
/// rotation (the vanilla switch only handles the four horizontal directions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BellShakeDirection {
    Down,
    Up,
    North,
    South,
    East,
    West,
}

/// Texture source for one banner-pattern sheet. Known bootstrap patterns retain a static fallback
/// for pack-unavailable/offline paths; synchronized and resource-pack sprites use the shared
/// runtime entity atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerPatternTexture {
    Builtin(BannerPatternKind),
    Runtime(EntityDynamicTexture),
}

/// One banner pattern layer — vanilla `BannerPatternLayers.Layer`: the pattern sprite plus the dye
/// whose `getTextureDiffuseColor` tints the layer's flag-geometry submit
/// (`BannerRenderer.submitPatternLayer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BannerPatternLayer {
    pub texture: BannerPatternTexture,
    pub color: EntityDyeColor,
}

/// Vanilla `DecoratedPotRenderState.wobbleStyle` + `wobbleProgress`: the running pot wobble the
/// renderer folds into the root transform (`DecoratedPotRenderer.submit`,
/// `DecoratedPotRenderer.java:150-169`). `positive` selects the `WobbleStyle.POSITIVE` X/Z tilt
/// (an item was stored/taken) over the `NEGATIVE` Y twist (a failed interaction); `progress` is
/// `(gameTime - wobbleStartedAtTick + partialTicks) / style.duration`, applied only inside
/// `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecoratedPotWobble {
    pub positive: bool,
    pub progress: f32,
}

/// Vanilla `Panda.Gene` (the displayed variant from `Panda.getVariant()`): the seven panda genes,
/// sharing the `PandaModel` / `BabyPandaModel` mesh and differing only by texture × age
/// (`PandaRenderer.getTextureLocation`). `NORMAL` is the vanilla default; `BROWN`/`WEAK` are the two
/// recessive genes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PandaModelVariant {
    Normal,
    Lazy,
    Worried,
    Playful,
    Brown,
    Weak,
    Aggressive,
}

impl PandaModelVariant {
    /// Vanilla `Panda.Gene.byId` (`ByIdMap.continuous` with `OutOfBoundsStrategy.ZERO`): the gene id
    /// `0..=6` maps directly, an out-of-range id folding back to `NORMAL`.
    pub fn from_id(id: i32) -> Self {
        match id {
            1 => Self::Lazy,
            2 => Self::Worried,
            3 => Self::Playful,
            4 => Self::Brown,
            5 => Self::Weak,
            6 => Self::Aggressive,
            _ => Self::Normal,
        }
    }

    /// The two recessive genes (`Panda.Gene.isRecessive`).
    fn is_recessive(self) -> bool {
        matches!(self, Self::Brown | Self::Weak)
    }

    /// Vanilla `Panda.Gene.getVariantFromGenes`: a recessive main gene shows only when both genes
    /// match (otherwise `NORMAL`); a dominant main gene always shows.
    pub fn from_genes(main_id: i32, hidden_id: i32) -> Self {
        let main = Self::from_id(main_id);
        if main.is_recessive() {
            if main == Self::from_id(hidden_id) {
                main
            } else {
                Self::Normal
            }
        } else {
            main
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheepWoolColor {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

impl LlamaVariant {
    pub fn from_vanilla_id(id: i32) -> Self {
        match id.clamp(0, 3) {
            0 => Self::Creamy,
            1 => Self::White,
            2 => Self::Brown,
            _ => Self::Gray,
        }
    }
}

impl SheepWoolColor {
    pub fn from_vanilla_id(id: u8) -> Self {
        match id {
            0 => Self::White,
            1 => Self::Orange,
            2 => Self::Magenta,
            3 => Self::LightBlue,
            4 => Self::Yellow,
            5 => Self::Lime,
            6 => Self::Pink,
            7 => Self::Gray,
            8 => Self::LightGray,
            9 => Self::Cyan,
            10 => Self::Purple,
            11 => Self::Blue,
            12 => Self::Brown,
            13 => Self::Green,
            14 => Self::Red,
            15 => Self::Black,
            _ => Self::White,
        }
    }

    pub fn vanilla_id(self) -> u8 {
        match self {
            Self::White => 0,
            Self::Orange => 1,
            Self::Magenta => 2,
            Self::LightBlue => 3,
            Self::Yellow => 4,
            Self::Lime => 5,
            Self::Pink => 6,
            Self::Gray => 7,
            Self::LightGray => 8,
            Self::Cyan => 9,
            Self::Purple => 10,
            Self::Blue => 11,
            Self::Brown => 12,
            Self::Green => 13,
            Self::Red => 14,
            Self::Black => 15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelBounds {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityModelTextureImage {
    pub texture: EntityModelTextureRef,
    /// Physical pack-image resolution; distinct from the model's logical UV
    /// extent in `texture.size`.
    pub size: [u32; 2],
    /// Selected-pack sampling metadata retained for standalone consumers such
    /// as the item and armor glint owners. The shared entity atlas deliberately
    /// keeps its own fixed sampler.
    pub sampling: StandaloneTextureSampling,
    pub rgba: Vec<u8>,
}

/// Owned pack texture reference admitted to the shared entity atlas for
/// resource-defined special item models. Static entity layers retain their
/// compact `&'static str` references; only pack-defined paths pay for `Arc`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityModelOwnedTextureRef {
    pub path: Arc<str>,
    /// Logical UV extent used by the model layer, independent of the physical
    /// pack image resolution.
    pub size: [u32; 2],
}

impl EntityModelOwnedTextureRef {
    pub fn new(path: impl Into<Arc<str>>, size: [u32; 2]) -> Self {
        Self {
            path: path.into(),
            size,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityModelOwnedTextureImage {
    pub texture: EntityModelOwnedTextureRef,
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
}

impl EntityModelOwnedTextureImage {
    pub fn new(texture: EntityModelOwnedTextureRef, rgba: Vec<u8>) -> Self {
        Self {
            size: texture.size,
            texture,
            rgba,
        }
    }

    pub fn with_physical_size(
        texture: EntityModelOwnedTextureRef,
        size: [u32; 2],
        rgba: Vec<u8>,
    ) -> Self {
        Self {
            texture,
            size,
            rgba,
        }
    }
}

impl EntityModelTextureImage {
    pub fn new(texture: EntityModelTextureRef, rgba: Vec<u8>) -> Self {
        Self {
            size: texture.size,
            texture,
            sampling: StandaloneTextureSampling::default(),
            rgba,
        }
    }

    pub fn with_physical_size(
        texture: EntityModelTextureRef,
        size: [u32; 2],
        rgba: Vec<u8>,
    ) -> Self {
        Self {
            texture,
            size,
            sampling: StandaloneTextureSampling::default(),
            rgba,
        }
    }

    pub fn with_physical_size_and_sampling(
        texture: EntityModelTextureRef,
        size: [u32; 2],
        rgba: Vec<u8>,
        sampling: StandaloneTextureSampling,
    ) -> Self {
        Self {
            texture,
            size,
            sampling,
            rgba,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelTextureAtlasEntry {
    pub texture: EntityModelTextureRef,
    pub uv: EntityModelUvRect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityModelOwnedTextureAtlasEntry {
    pub texture: EntityModelOwnedTextureRef,
    pub uv: EntityModelUvRect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityModelTextureAtlasLayout {
    pub width: u32,
    pub height: u32,
    pub entries: Vec<EntityModelTextureAtlasEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityDynamicPlayerSkinAtlasEntry {
    pub(crate) handle: u64,
    pub(crate) pixel_origin: [u32; 2],
    pub(crate) size: [u32; 2],
    pub(crate) uv: EntityModelUvRect,
    pub(crate) standalone_sampling: Option<StandaloneTextureSampling>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityDynamicPlayerSkinAtlasLayout {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) entries: Vec<EntityDynamicPlayerSkinAtlasEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityDynamicPlayerTextureAtlasEntry {
    pub(crate) handle: u64,
    pub(crate) pixel_origin: [u32; 2],
    pub(crate) size: [u32; 2],
    pub(crate) uv: EntityModelUvRect,
    pub(crate) standalone_sampling: Option<StandaloneTextureSampling>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityDynamicPlayerTextureAtlasLayout {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) entries: Vec<EntityDynamicPlayerTextureAtlasEntry>,
}
