mod selection;

pub(in crate::entity_models) use selection::{
    boat_texture_ref, camel_texture_ref, chicken_texture_ref, cow_texture_ref,
    horse_markings_texture_ref, llama_texture_ref, mooshroom_texture_ref, pig_texture_ref,
    sheep_wool_render_color, squid_texture_ref, villager_level_texture_ref,
    villager_profession_texture_ref, villager_type_texture_ref, wolf_texture_ref,
    zombie_villager_level_texture_ref, zombie_villager_profession_texture_ref,
    zombie_villager_type_texture_ref,
};
#[cfg(test)]
pub(in crate::entity_models) use selection::{
    player_texture_ref, sheep_jeb_wool_layer_color, sheep_wool_layer_color,
};

/// Vanilla `SkullBlock.Type` values currently rendered by bbb's `CustomHeadLayer` skull branch.
///
/// Static mob heads share the same 8x8x8 mob-head layer and differ only by entity texture. Player
/// heads use the humanoid head layer (base head plus hat). Profileless heads use vanilla's fixed
/// `DefaultPlayerSkin.getDefaultTexture()` skin; profiled heads can use the UUID/default-skin fallback
/// and can now carry a dynamic skin handle/model while live texture upload stays deferred. Dragon and
/// piglin heads use their specialized animated skull models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityCustomHeadSkull {
    Skeleton,
    WitherSkeleton,
    Player(EntityPlayerSkin),
    Zombie,
    Creeper,
    Dragon,
    Piglin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPlayerSkin {
    /// Profileless `player_head` fallback: vanilla `DefaultPlayerSkin.getDefaultTexture()`.
    Default(EntityDefaultPlayerSkin),
    /// A profile-backed player skin that resolves to a built-in default skin for now.
    ProfiledDefault(EntityDefaultPlayerSkin),
    Dynamic(EntityDynamicPlayerSkin),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityDynamicPlayerSkin {
    pub handle: u64,
    pub fallback: EntityDefaultPlayerSkin,
    pub model: EntityPlayerSkinModel,
    pub status: EntityDynamicPlayerSkinStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityDynamicPlayerTexture {
    pub handle: u64,
    pub kind: EntityDynamicPlayerTextureKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDynamicPlayerTextureKind {
    Cape,
    Elytra,
}

/// One equipment-asset layer texture projected onto an entity layer renderer. This mirrors one
/// `EquipmentClientInfo.Layer` after the native side has resolved its pack texture path to a renderer
/// atlas texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityEquipmentLayerTexture {
    pub texture: EntityModelTextureRef,
    pub use_player_texture: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDynamicPlayerSkinStatus {
    Loading,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPlayerSkinModel {
    Slim,
    Wide,
}

impl EntityPlayerSkinModel {
    pub const fn is_slim(self) -> bool {
        matches!(self, Self::Slim)
    }
}

impl EntityPlayerSkin {
    pub const fn fallback(self) -> EntityDefaultPlayerSkin {
        match self {
            Self::Default(skin) => skin,
            Self::ProfiledDefault(skin) => skin,
            Self::Dynamic(skin) => skin.fallback,
        }
    }

    pub const fn model(self) -> EntityPlayerSkinModel {
        match self {
            Self::Default(skin) => skin.model(),
            Self::ProfiledDefault(skin) => skin.model(),
            Self::Dynamic(skin) => skin.model,
        }
    }

    pub const fn default_for_model(slim: bool) -> Self {
        if slim {
            Self::Default(EntityDefaultPlayerSkin::SlimSteve)
        } else {
            Self::Default(EntityDefaultPlayerSkin::WideSteve)
        }
    }

    pub const fn is_slim(self) -> bool {
        self.model().is_slim()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDefaultPlayerSkin {
    SlimAlex,
    SlimAri,
    SlimEfe,
    SlimKai,
    SlimMakena,
    SlimNoor,
    SlimSteve,
    SlimSunny,
    SlimZuri,
    WideAlex,
    WideAri,
    WideEfe,
    WideKai,
    WideMakena,
    WideNoor,
    WideSteve,
    WideSunny,
    WideZuri,
}

impl EntityDefaultPlayerSkin {
    pub fn from_vanilla_index(index: usize) -> Self {
        match index % DEFAULT_PLAYER_SKINS.len() {
            0 => Self::SlimAlex,
            1 => Self::SlimAri,
            2 => Self::SlimEfe,
            3 => Self::SlimKai,
            4 => Self::SlimMakena,
            5 => Self::SlimNoor,
            6 => Self::SlimSteve,
            7 => Self::SlimSunny,
            8 => Self::SlimZuri,
            9 => Self::WideAlex,
            10 => Self::WideAri,
            11 => Self::WideEfe,
            12 => Self::WideKai,
            13 => Self::WideMakena,
            14 => Self::WideNoor,
            15 => Self::WideSteve,
            16 => Self::WideSunny,
            _ => Self::WideZuri,
        }
    }

    pub fn from_texture_path(path: &str) -> Option<Self> {
        let path = path.strip_prefix("minecraft:").unwrap_or(path);
        DEFAULT_PLAYER_SKINS
            .iter()
            .copied()
            .find(|skin| skin.texture_path() == path)
    }

    pub const fn texture_path(self) -> &'static str {
        match self {
            Self::SlimAlex => "textures/entity/player/slim/alex.png",
            Self::SlimAri => "textures/entity/player/slim/ari.png",
            Self::SlimEfe => "textures/entity/player/slim/efe.png",
            Self::SlimKai => "textures/entity/player/slim/kai.png",
            Self::SlimMakena => "textures/entity/player/slim/makena.png",
            Self::SlimNoor => "textures/entity/player/slim/noor.png",
            Self::SlimSteve => "textures/entity/player/slim/steve.png",
            Self::SlimSunny => "textures/entity/player/slim/sunny.png",
            Self::SlimZuri => "textures/entity/player/slim/zuri.png",
            Self::WideAlex => "textures/entity/player/wide/alex.png",
            Self::WideAri => "textures/entity/player/wide/ari.png",
            Self::WideEfe => "textures/entity/player/wide/efe.png",
            Self::WideKai => "textures/entity/player/wide/kai.png",
            Self::WideMakena => "textures/entity/player/wide/makena.png",
            Self::WideNoor => "textures/entity/player/wide/noor.png",
            Self::WideSteve => "textures/entity/player/wide/steve.png",
            Self::WideSunny => "textures/entity/player/wide/sunny.png",
            Self::WideZuri => "textures/entity/player/wide/zuri.png",
        }
    }

    pub const fn model(self) -> EntityPlayerSkinModel {
        match self {
            Self::SlimAlex
            | Self::SlimAri
            | Self::SlimEfe
            | Self::SlimKai
            | Self::SlimMakena
            | Self::SlimNoor
            | Self::SlimSteve
            | Self::SlimSunny
            | Self::SlimZuri => EntityPlayerSkinModel::Slim,
            Self::WideAlex
            | Self::WideAri
            | Self::WideEfe
            | Self::WideKai
            | Self::WideMakena
            | Self::WideNoor
            | Self::WideSteve
            | Self::WideSunny
            | Self::WideZuri => EntityPlayerSkinModel::Wide,
        }
    }
}

const DEFAULT_PLAYER_SKINS: [EntityDefaultPlayerSkin; 18] = [
    EntityDefaultPlayerSkin::SlimAlex,
    EntityDefaultPlayerSkin::SlimAri,
    EntityDefaultPlayerSkin::SlimEfe,
    EntityDefaultPlayerSkin::SlimKai,
    EntityDefaultPlayerSkin::SlimMakena,
    EntityDefaultPlayerSkin::SlimNoor,
    EntityDefaultPlayerSkin::SlimSteve,
    EntityDefaultPlayerSkin::SlimSunny,
    EntityDefaultPlayerSkin::SlimZuri,
    EntityDefaultPlayerSkin::WideAlex,
    EntityDefaultPlayerSkin::WideAri,
    EntityDefaultPlayerSkin::WideEfe,
    EntityDefaultPlayerSkin::WideKai,
    EntityDefaultPlayerSkin::WideMakena,
    EntityDefaultPlayerSkin::WideNoor,
    EntityDefaultPlayerSkin::WideSteve,
    EntityDefaultPlayerSkin::WideSunny,
    EntityDefaultPlayerSkin::WideZuri,
];

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
    HappyGhast,
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
    /// animation, and the head yaw/pitch. The dance pose (`isDancing`/`isSpinning`) and
    /// held-item arm poses are deferred entity-side state.
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
    /// animation, water, and ground state. The egg-laying leg amplitude (`isLayingEgg`) and the
    /// `egg_belly` shell (`hasEgg`) are deferred entity-side state.
    Turtle {
        baby: bool,
    },
    /// Bat (`BatModel`, `BatRenderer`). The first keyframe-animated entity: the looping
    /// `BatAnimation.BAT_FLYING` wing flap / body bob is sampled from `EntityRenderState`'s
    /// `age_in_ticks`. While the synced `isResting` is set the model swaps to the static
    /// `BatAnimation.BAT_RESTING` hanging pose and `applyHeadRotation` turns the head by the
    /// look yaw.
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
    /// translucent wind layer, the emissive eyes, and the shoot/slide/inhale/jump action animations
    /// are deferred entity-side state.
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
    /// withdrawal, tail sway, and attack beam are projected; eye tracking stays deferred.
    Guardian {
        elder: bool,
    },
    /// `FrogModel` at its `createBodyLayer` rest pose, textured by temperature variant
    /// (`FrogVariant` asset). The keyframe animations (jump, croak, tongue, swim/walk,
    /// idle-in-water) are deferred.
    Frog {
        variant: FrogModelVariant,
    },
    /// `CreakingModel` at its `createBodyLayer` rest pose. The head look, walk, attack,
    /// invulnerable, and death keyframe animations are deferred. The emissive eyes layer IS projected:
    /// `eyes_glowing` (the synced `isActive()` flag) shows the `creaking_eyes.png` overlay; the
    /// tearing-down death-flicker (`hasGlowingEyes`) stays deferred.
    Creaking {
        eyes_glowing: bool,
    },
    /// `SnifferModel` at its `createBodyLayer` rest pose. The head look, search/walk, and the
    /// dig / long-sniff / stand-up / happy / scenting keyframe animations are deferred.
    Sniffer,
    /// `WardenModel` at its `createBodyLayer` rest pose. The head look, walk, idle wobble,
    /// tendril sway, the attack / sonic-boom / digging / emerge / roar / sniff keyframe
    /// animations, and the four emissive overlay layers are deferred.
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
    /// (`AxolotlRenderer.TEXTURE_BY_TYPE`). The body yaw, the swimming / water-hovering /
    /// ground-crawling / lay-still procedural sways and baby keyframe animations, the play-dead
    /// pose, and the mirror-leg copy are deferred.
    Axolotl {
        baby: bool,
        variant: AxolotlModelVariant,
    },
    /// `TadpoleModel` at its `createBodyLayer` rest pose. The tail yaw sway (`tail.yRot`) is
    /// deferred.
    Tadpole,
    /// `ParrotModel` at its `createBodyLayer` STANDING rest pose, textured per the five
    /// `Parrot.Variant` colours (`variant`, the synced `DATA_VARIANT_ID`). The head look, the per-pose
    /// `prepare` offsets, the leg walk swing, the wing flap, the flap bob, and the PARTY dance are
    /// deferred.
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
    /// `EvokerFangsModel` at its `createBodyLayer` closed-jaw rest pose (the base block plus the two
    /// jaws). The bite open/close, the base drop, and the emerge scale are deferred.
    EvokerFangs,
    /// `LeashKnotModel` — the single 6×8×6 knot box. The model has no `setupAnim`, so the geometry
    /// is complete; only the texture-backed path is deferred.
    LeashKnot,
    /// `ArrowModel` at its `createBodyLayer` rest pose (the arrowhead plane plus the two crossed
    /// fletching planes, the whole mesh scaled 0.9). `texture` picks the normal / tipped / spectral
    /// image (`TippableArrowRenderer` swaps to `arrow_tipped.png` when the arrow carries a potion;
    /// the spectral arrow binds `arrow_spectral.png`). The impact-shake wobble is deferred;
    /// `ArrowRenderer` orients it along its flight.
    Arrow {
        texture: ArrowModelTexture,
    },
    /// `TridentModel` — the pole, crossguard, and three spikes. The model has no animation, so the
    /// geometry and base texture are complete; only the enchant-foil overlay is deferred.
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
    /// The `setupAnim` counter-rotation, the translucent scrolling `breezeWind` texture, and the
    /// texture-backed path are deferred.
    WindCharge,
    /// `EnderDragonModel` at its `createBodyLayer` straight bind layout (head + jaw, five neck and
    /// twelve tail segments, body with wings and four legs). The fully procedural `setupAnim` (the
    /// flight-history neck/tail placement, the wing flap, the jaw, the root bounce), the dying
    /// dissolve and nearest-crystal healing beam are deferred; the emissive eyes layer is projected.
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
    /// `QuadrupedModel.setupAnim` head look and four-leg walk swing are reproduced; every panda-specific
    /// pose (`isUnhappy`, `isSneezing`, `sitAmount`, `lieOnBackAmount`, `rollAmount`) reads un-projected
    /// `PandaRenderState` state and stays deferred. The gene-driven base texture IS projected: `variant`
    /// is the displayed `Panda.Gene` (`PandaRenderer.getTextureLocation` keys the 7-gene × age texture
    /// matrix off it).
    Panda {
        baby: bool,
        variant: PandaModelVariant,
    },
    /// `AdultFelineModel` / `BabyFelineModel` at their `createBodyMesh` rest pose, shared by the ocelot
    /// (`cat = false`) and the cat (`cat = true`). The adult cat scales the shared adult mesh 0.8 (via
    /// `AdultCatModel.CAT_TRANSFORMER` in the root transform); the ocelot and both babies are unscaled.
    /// `baby` selects the flatter `BabyFelineModel` layout. The shared `setupAnim` head look is
    /// reproduced, plus the adult's standing `tail2` droop (`xRot = 1.7278761` — the baby's `tail2` is
    /// cubeless, so vanilla's identical assignment is invisible). The bespoke walk leg swing / tail
    /// wobble and every feline pose (`isCrouching`, `isSprinting`, `isSitting`, `lieDownAmount`,
    /// `relaxStateOneAmount`) read un-projected `FelineRenderState` state and stay deferred. The textured
    /// path binds the cat-breed texture (`cat_variant`, the eleven `CatVariant`s) for cats and the
    /// `ocelot` texture for ocelots (`cat_variant` is ignored when `!cat`). `collar` is the tame cat's
    /// dyed collar overlay (`CatCollarLayer`, `Some` only for a tame cat — the ocelot never has one).
    Feline {
        cat: bool,
        baby: bool,
        cat_variant: CatModelVariant,
        collar: Option<EntityDyeColor>,
    },
    /// `AdultFoxModel` / `BabyFoxModel` (custom `EntityModel`s) at their `createBodyLayer` rest pose
    /// (`baby` selects the baby body layout). The `setupAnim` head look and the curled sleeping pose
    /// are reproduced, and the texture switches on the `variant` (red/snow) × age × sleeping matrix
    /// (`FoxRenderer.getTextureLocation`). The walk leg swing, the `headRollAngle` tilt, and the
    /// remaining poses (`isCrouching`, `isSitting`, `isPouncing`, `isFaceplanted`) stay deferred.
    Fox {
        baby: bool,
        variant: FoxModelVariant,
    },
    /// `NautilusModel` (a custom `EntityModel`, the new rideable nautilus) at its `createBodyMesh` /
    /// `createBabyBodyLayer` rest pose (`baby` selects the smaller hatchling geometry, the same
    /// `root → shell + body → mouths` structure). The `setupAnim` body look — clamped to ±10° — is
    /// reproduced; the looping `NautilusAnimation.SWIMMING` keyframe undulation and body armor
    /// equipment layer are deferred. The adult saddle equipment layer is driven through render state.
    Nautilus {
        baby: bool,
    },
    /// `ZombieNautilusRenderer` (a plain `MobRenderer`, never a baby), selected by the synced
    /// `ZombieNautilusVariant` holder. `coral = false` (the `NORMAL`/`TEMPERATE` default) reuses the
    /// living adult `NautilusModel` body textured by `zombie_nautilus.png`; `coral = true` (the `WARM`
    /// variant) renders the `ZombieNautilusCoralModel` (the same body plus the `corals` subtree) over
    /// `zombie_nautilus_coral.png`. The adult saddle equipment layer is driven through render state;
    /// the body armor equipment layer stays deferred.
    ZombieNautilus {
        coral: bool,
    },
    /// `AdultRabbitModel` / `BabyRabbitModel` at their `createBodyLayer` rest pose (`baby` selects the
    /// baby body layout — a deeper `_r1`-nested hierarchy whose head is `body`'s third child),
    /// textured by the seven `Rabbit.Variant` colours × age, with the `toast` named-rabbit override
    /// (`RabbitRenderer.getTextureLocation`). `RabbitModel.setupAnim` turns the head by the look
    /// angles (reproduced). The looping `RabbitAnimation.HOP` / `BabyRabbitAnimation` and
    /// `IDLE_HEAD_TILT` keyframe animations need un-projected `AnimationState`s and stay deferred.
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
    /// The standard held-item layer is projected; interaction keyframes, custom head, and the antenna
    /// block decoration stay deferred entity-side layers.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZombieVariantModelFamily {
    Husk,
    Drowned,
    ZombieVillager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillagerModelType {
    Desert,
    Jungle,
    Plains,
    Savanna,
    Snow,
    Swamp,
    Taiga,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillagerModelProfession {
    None,
    Armorer,
    Butcher,
    Cartographer,
    Cleric,
    Farmer,
    Fisherman,
    Fletcher,
    Leatherworker,
    Librarian,
    Mason,
    Nitwit,
    Shepherd,
    Toolsmith,
    Weaponsmith,
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

impl VillagerModelType {
    pub const fn hat(self) -> VillagerModelHat {
        match self {
            // Vanilla villager type mcmeta declares `hat: full` only for desert and snow.
            Self::Desert | Self::Snow => VillagerModelHat::Full,
            Self::Jungle | Self::Plains | Self::Savanna | Self::Swamp | Self::Taiga => {
                VillagerModelHat::None
            }
        }
    }
}

impl VillagerModelProfession {
    pub const fn hat(self) -> VillagerModelHat {
        match self {
            // Vanilla profession mcmeta declares butcher as partial and these six as full.
            Self::Butcher => VillagerModelHat::Partial,
            Self::Farmer | Self::Fisherman | Self::Fletcher | Self::Librarian | Self::Shepherd => {
                VillagerModelHat::Full
            }
            Self::None
            | Self::Armorer
            | Self::Cartographer
            | Self::Cleric
            | Self::Leatherworker
            | Self::Mason
            | Self::Nitwit
            | Self::Toolsmith
            | Self::Weaponsmith => VillagerModelHat::None,
        }
    }

    pub const fn renders_level_badge(self) -> bool {
        !matches!(self, Self::None | Self::Nitwit)
    }
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

    /// Vanilla `TropicalFish.getPattern(packedVariant).base()`: the body shape is decoded
    /// from the synced packed variant. `Pattern.byId(packed & 0xFFFF)` is a sparse lookup
    /// over the twelve patterns (each packed as `base.id | index << 8`, base `0`/`1`, index
    /// `0..=5`) defaulting to `KOB` (small) for any unrecognized id. So the shape is `Large`
    /// only when the low pattern byte is `1` (a `LARGE` base) and the index byte is in
    /// range; every other packed value — including the default `0` (`KOB`/white/white) —
    /// resolves to the small body.
    pub fn from_vanilla_packed_variant(packed_variant: i32) -> Self {
        let pattern_id = packed_variant & 0xFFFF;
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
    /// Vanilla `TropicalFish.Pattern.byId(packed & 0xFFFF)`, a sparse lookup over the twelve
    /// patterns keyed on `packedId = base.id | index << 8` (so `KOB=0`, `SUNSTREAK=256`, …,
    /// `FLOPPER=1`, `STRIPEY=257`, …). Any unrecognized id falls back to `KOB`, exactly like
    /// `ByIdMap.sparse(..., KOB)`.
    pub fn from_vanilla_packed_variant(packed_variant: i32) -> Self {
        match packed_variant & 0xFFFF {
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
}

/// Vanilla `WolfVariant` (the synced `Wolf.DATA_VARIANT_ID` registry holder): the nine registered
/// biome wolf coats, sharing one `WolfModel` and differing only by texture. Each variant supplies a
/// wild/tame/angry × adult/baby texture set (`WolfVariants.register`). `Pale` is the vanilla
/// `WolfVariants.DEFAULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WolfModelVariant {
    Pale,
    Spotted,
    Snowy,
    Black,
    Ashen,
    Rusty,
    Woods,
    Chestnut,
    Striped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChickenModelVariant {
    Temperate,
    Warm,
    Cold,
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
    /// Vanilla `Parrot.Variant.byId` (clamped via `byId`'s modulo): maps the synced `DATA_VARIANT_ID`
    /// int to a colour, defaulting to `RED_BLUE` for the out-of-range ids vanilla folds back to `0`.
    pub fn from_id(id: i32) -> Self {
        match id {
            1 => Self::Blue,
            2 => Self::Green,
            3 => Self::YellowBlue,
            4 => Self::Gray,
            _ => Self::RedBlue,
        }
    }
}

/// Vanilla `FrogVariant` (the synced `DATA_VARIANT_ID` `Holder<FrogVariant>`): the three temperature
/// frogs, sharing one `FrogModel` and differing only by texture (`FrogRenderer.getTextureLocation`
/// reads `state.texture` from the variant's `assetInfo`). `TEMPERATE` is the vanilla default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrogModelVariant {
    Temperate,
    Warm,
    Cold,
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

/// Vanilla `CatVariant` (the synced `DATA_VARIANT_ID` `Holder<CatVariant>`): the eleven cat breeds,
/// sharing one `CatModel` and differing by texture × age (`CatRenderer.getTextureLocation` reads
/// `state.texture` from the variant's `assetInfo(isBaby)`). `BLACK` is the vanilla default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatModelVariant {
    Tabby,
    Black,
    Red,
    Siamese,
    BritishShorthair,
    Calico,
    Persian,
    Ragdoll,
    White,
    Jellie,
    AllBlack,
}

/// Which arrow image to bind: the plain `arrow.png`, the `arrow_tipped.png` (`TippableArrowRenderer`
/// when the arrow carries a potion, `getColor() > 0`), or the spectral arrow's `arrow_spectral.png`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowModelTexture {
    Normal,
    Tipped,
    Spectral,
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
pub enum PigModelVariant {
    Temperate,
    Warm,
    Cold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CowModelVariant {
    Temperate,
    Warm,
    Cold,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityModelTextureRef {
    pub path: &'static str,
    pub size: [u32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityModelTextureImage {
    pub texture: EntityModelTextureRef,
    pub rgba: Vec<u8>,
}

impl EntityModelTextureImage {
    pub fn new(texture: EntityModelTextureRef, rgba: Vec<u8>) -> Self {
        Self { texture, rgba }
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
pub struct EntityModelTextureAtlasLayout {
    pub width: u32,
    pub height: u32,
    pub entries: Vec<EntityModelTextureAtlasEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityDynamicPlayerSkinAtlasEntry {
    pub(crate) handle: u64,
    pub(crate) uv: EntityModelUvRect,
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
    pub(crate) size: [u32; 2],
    pub(crate) uv: EntityModelUvRect,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityDynamicPlayerTextureAtlasLayout {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) entries: Vec<EntityDynamicPlayerTextureAtlasEntry>,
}
