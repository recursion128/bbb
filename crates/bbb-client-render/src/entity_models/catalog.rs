mod selection;

use bbb_render_types::StandaloneTextureSampling;
use glam::Mat4;

use crate::item_models::SpecialItemCopperGolemPose;

use super::{
    BannerPatternKind, CatModelVariant, ChickenModelVariant, CowModelVariant, DecoratedPotPattern,
    FrogModelVariant, PigModelVariant, VillagerModelProfession, VillagerModelType,
    WolfModelVariant, ZombieNautilusModelVariant,
};
pub use crate::render_core::entity_catalog::{
    BedModelPart, ChestModelHalf, EntityDyeColor, EntityModelOwnedTextureRef, SignModelAttachment,
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

// Pure value types that also project into the item model runtime live in `bbb-render-types`; re-export
// them here so every existing `crate::entity_models::...` path keeps working.
pub use bbb_render_types::{
    BoatModelFamily, DragonFlightHistorySample, EndCrystalBeamRenderState,
    EnderDragonAnimationRenderState, EnderDragonBeamRenderState, EntityArmorTrim,
    EntityAttachmentFace, EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDynamicPlayerSkin,
    EntityDynamicPlayerSkinStatus, EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind,
    EntityDynamicTexture, EntityEquipmentAssetId, EntityLeashRopesRenderState,
    EntityModelTextureRef, EntityPlayerSkin, EntityPlayerSkinModel, GuardianBeamRenderState,
    GuardianEyeTrackingRenderState, LeashRopeRenderState, LlamaModelFamily, ParrotModelVariant,
    PiglinModelFamily, UndeadHorseModelFamily, VanillaDyeColor as SheepWoolColor,
    VillagerModelData, WolfArmorCrackiness,
};

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
    North,
    South,
    West,
    East,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndPortalModelFaces(u8);

impl EndPortalModelFaces {
    pub const EMPTY: Self = Self(0);
    pub const Y_AXIS: Self = Self::from_faces(&[EndPortalModelFace::Down, EndPortalModelFace::Up]);
    pub const ALL: Self = Self::from_faces(&[
        EndPortalModelFace::Down,
        EndPortalModelFace::Up,
        EndPortalModelFace::North,
        EndPortalModelFace::South,
        EndPortalModelFace::West,
        EndPortalModelFace::East,
    ]);

    const fn bit(face: EndPortalModelFace) -> u8 {
        match face {
            EndPortalModelFace::Down => 1 << 0,
            EndPortalModelFace::Up => 1 << 1,
            EndPortalModelFace::North => 1 << 2,
            EndPortalModelFace::South => 1 << 3,
            EndPortalModelFace::West => 1 << 4,
            EndPortalModelFace::East => 1 << 5,
        }
    }

    pub const fn from_faces(faces: &[EndPortalModelFace]) -> Self {
        let mut bits = 0u8;
        let mut index = 0;
        while index < faces.len() {
            bits |= Self::bit(faces[index]);
            index += 1;
        }
        Self(bits)
    }

    pub const fn for_kind(kind: EndPortalModelKind) -> Self {
        match kind {
            EndPortalModelKind::EndPortal => Self::Y_AXIS,
            EndPortalModelKind::EndGateway => Self::ALL,
        }
    }

    pub const fn with(self, face: EndPortalModelFace) -> Self {
        Self(self.0 | Self::bit(face))
    }

    pub const fn contains(self, face: EndPortalModelFace) -> bool {
        (self.0 & Self::bit(face)) != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = EndPortalModelFace> {
        [
            EndPortalModelFace::Down,
            EndPortalModelFace::Up,
            EndPortalModelFace::North,
            EndPortalModelFace::South,
            EndPortalModelFace::West,
            EndPortalModelFace::East,
        ]
        .into_iter()
        .filter(move |&face| self.contains(face))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpawnerDisplayRenderState {
    pub spin_degrees: f32,
    pub scale: f32,
}

#[allow(clippy::large_enum_variant)]
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
    /// Vanilla 26.1 `BeaconRenderer` beam source. The beacon block itself is
    /// ordinary terrain geometry; this kind carries only one beam section, so a
    /// beacon submits one instance per `BeaconBeamOwner.Section` exactly as
    /// vanilla loops over them.
    BeaconBeam,
    /// Vanilla 26.1 `TheEndPortalRenderer` / `TheEndGatewayRenderer` block-entity cube source.
    /// End portals submit only the Y-axis faces (`TheEndPortalBlockEntity.shouldRenderFace`) and
    /// apply the `T(0, 0.375, 0) * S(1, 0.375, 1)` cube transform; end gateways submit up to all
    /// six faces (`TheEndGatewayBlockEntity.shouldRenderFace`) on the unit cube and may add the
    /// spawn/cooldown beacon-style beam through `BlockEntityRenderState.end_gateway_beam`.
    EndPortalBlock {
        kind: EndPortalModelKind,
        faces: EndPortalModelFaces,
    },
    /// Vanilla 26.1 `ChestModel` submitted by the `ChestRenderer` block-entity renderer — the first
    /// (and so far only) block-entity model in the scene. `half` selects the single or the
    /// double-chest left/right mesh (`MultiblockChestResources.select`), `texture` the
    /// `Sheets.chooseSprite` material family; the lid/lock openness rotation reads
    /// `BlockEntityRenderState.chest_openness`. Instances are projected from chest block states, not the
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
    /// `BlockEntityRenderState.bell_ticks` / `bell_shake_direction`.
    Bell,
    /// Vanilla 26.1 `ShulkerBoxRenderer.ShulkerBoxModel` — the shulker mob's shell mesh
    /// (`ShulkerModel.createBoxLayer`: `lid` + `base`, no head) submitted per shulker box block
    /// entity. Instances are projected from shulker box block states (`entity_id` is a sentinel).
    /// `color` selects the `Sheets.getShulkerBoxSprite` texture (`None` -> the undyed
    /// `entity/shulker/shulker`); `facing` is the block's six-way `ShulkerBoxBlock.FACING`, folded
    /// into the root transform via `Direction.getRotation()`
    /// (`ShulkerBoxRenderer.createModelTransform`). The lid lift/twist reads
    /// `BlockEntityRenderState.shulker_box_progress`.
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
    /// `BlockEntityRenderState.decorated_pot_wobble`.
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
    /// `BlockEntityRenderState.banner_flag_phase`. The layer array clamps at the vanilla
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
    /// carried by `EntityRenderState.kind.fishing_hook().line_origin_offset`.
    FishingHook,
    /// Vanilla 26.1 `BookModel` (`ModelLayers.BOOK`) submitted by the
    /// `EnchantTableRenderer` block-entity renderer — the enchanting table's
    /// hovering, page-flipping book on the `entity/enchantment/
    /// enchanting_table_book` sheet. Instances are projected from enchanting
    /// table block states, not the entity list (`entity_id` is a sentinel). The
    /// book turns to face the nearest player, bobs, opens/closes, and flips its
    /// pages: `body_rot` carries the lerped book yaw (degrees), the hover offset
    /// rides `BlockEntityRenderState.book_float_y`, and the `BookModel.setupAnim`
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
    /// temperate cow, `MOOSHROOM_BABY` to `BabyCowModel.createBodyLayer()`). `variant` drives the vanilla
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
    /// `CopperGolemStatueBlockRenderer`: the statue block entity submits one of
    /// four baked pose layers through `CopperGolemStatueModel`, textured by the
    /// block's own `WeatheringCopper.WeatherState`. Unlike the living golem it
    /// has no emissive eyes layer and no animation beyond `setupAnim`'s fixed
    /// root reset.
    CopperGolemStatueBlock {
        pose: SpecialItemCopperGolemPose,
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
    /// Data-driven Blockbench/Bedrock-compatible articulated entity rig indexed into
    /// `Renderer`'s baked rig catalog (`anim_state`: `0` = idle/walk, `1+` = enraged).
    DataDriven {
        rig_id: u32,
        anim_state: u8,
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
pub enum HoglinModelFamily {
    Hoglin,
    Zoglin,
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
pub enum DonkeyModelFamily {
    Donkey,
    Mule,
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

/// One `BeaconRenderer.submitBeaconBeam` call: a single beam section drawn
/// from `beam_start` for `height` blocks in the block-entity's colour, with
/// the camera-distance radius scale and the shared `floorMod(gameTime, 40) +
/// partialTick` scroll clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeaconBeamRenderState {
    pub beam_start: i32,
    pub height: i32,
    pub color_argb: u32,
    /// `BeaconRenderState.beamRadiusScale`.
    pub radius_scale: f32,
    pub animation_time: f32,
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
/// `DecoratedPotRenderer`). `positive` selects the `WobbleStyle.POSITIVE` X/Z tilt
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
