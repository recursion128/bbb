mod selection;

pub(in crate::entity_models) use selection::{
    boat_texture_ref, camel_texture_ref, chicken_texture_ref, cow_texture_ref, llama_texture_ref,
    pig_texture_ref, player_texture_ref, sheep_wool_render_color, squid_texture_ref,
    wolf_texture_ref,
};
#[cfg(test)]
pub(in crate::entity_models) use selection::{sheep_jeb_wool_layer_color, sheep_wool_layer_color};

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
        slim: bool,
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
    Ghast,
    HappyGhast,
    Blaze,
    Endermite,
    Silverfish,
    /// Vex (`VexModel`, `VexRenderer`). The idle wing flap, arm bob, head look, and body
    /// tilt read `EntityRenderState.age_in_ticks` and the head yaw/pitch. The charging
    /// pose (`isCharging`) and held-item arm poses are deferred entity-side state.
    Vex,
    /// Allay (`AllayModel`, `AllayRenderer`). The idle/flying wing flap, arm bob, head look,
    /// body tilt, and vertical bob read `EntityRenderState.age_in_ticks`, the walk
    /// animation, and the head yaw/pitch. The dance pose (`isDancing`/`isSpinning`) and
    /// held-item arm poses are deferred entity-side state.
    Allay,
    /// Strider (`AdultStriderModel` / `BabyStriderModel`, `StriderRenderer`). The body sway,
    /// vertical bob, leg swing/lift, and bristle flow read `EntityRenderState.age_in_ticks`,
    /// the walk animation, and the look angles. The ridden pose (`isRidden` zeroing the body
    /// look), the saddle equipment layer, and the cold/suffocating texture and shake are
    /// deferred entity-side state.
    Strider {
        baby: bool,
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
    /// `age_in_ticks`. The resting pose (`isResting`, `BatAnimation.BAT_RESTING`) is deferred
    /// entity-side state.
    Bat,
    /// Bee (`AdultBeeModel` / `BabyBeeModel`, `BeeRenderer`). The procedural `BeeModel.setupAnim`
    /// airborne wing flap (`zRot = cos(ageInTicks · 120.32113°) · π · 0.15`) and the idle
    /// `bobUpAndDown` bone/leg/antenna bob, gated on the synced `Entity.onGround`. The anger pose
    /// (`isAngry`), the rolled-up fall pose (`rollAmount`), the stinger-loss (`hasStinger`) and the
    /// nectar/angry texture swaps are deferred entity-side state.
    Bee {
        baby: bool,
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
    /// layer. The held-item carry layer is deferred entity-side state.
    Dolphin {
        baby: bool,
    },
    /// `GuardianModel` (`elder = false`) or the same mesh scaled 2.35× by
    /// `GuardianModel.ELDER_GUARDIAN_SCALE` (`elder = true`). The procedural spike pulse /
    /// withdrawal, eye tracking, tail sway, and attack beam are deferred (rendered at the
    /// `createBodyLayer` rest pose).
    Guardian {
        elder: bool,
    },
    /// `FrogModel` at its `createBodyLayer` rest pose. The keyframe animations (jump, croak,
    /// tongue, swim/walk, idle-in-water) and the three texture variants are deferred.
    Frog,
    /// `CreakingModel` at its `createBodyLayer` rest pose. The head look, walk, attack,
    /// invulnerable, and death keyframe animations and the emissive eyes layer are deferred.
    Creaking,
    /// `SnifferModel` at its `createBodyLayer` rest pose. The head look, search/walk, and the
    /// dig / long-sniff / stand-up / happy / scenting keyframe animations are deferred.
    Sniffer,
    /// `WardenModel` at its `createBodyLayer` rest pose. The head look, walk, idle wobble,
    /// tendril sway, the attack / sonic-boom / digging / emerge / roar / sniff keyframe
    /// animations, and the four emissive overlay layers are deferred.
    Warden,
    /// `AdultArmadilloModel` / `BabyArmadilloModel` at their `createBodyLayer` rest pose (`baby`
    /// selects the baby body layer). The clamped head look, the `applyWalk` leg sway, the
    /// roll-out / roll-up / peek keyframe animations, and the `isHidingInShell` shell-ball swap
    /// are deferred.
    Armadillo {
        baby: bool,
    },
    /// `AdultAxolotlModel` / `BabyAxolotlModel` at their `createBodyLayer` rest pose (`baby`
    /// selects the baby body layer). The body yaw, the swimming / water-hovering / ground-crawling
    /// / lay-still procedural sways and baby keyframe animations, the play-dead pose, the
    /// mirror-leg copy, and the five color variants are deferred.
    Axolotl {
        baby: bool,
    },
    /// `TadpoleModel` at its `createBodyLayer` rest pose. The tail yaw sway (`tail.yRot`) is
    /// deferred.
    Tadpole,
    /// `ParrotModel` at its `createBodyLayer` STANDING rest pose. The head look, the per-pose
    /// `prepare` offsets, the leg walk swing, the wing flap, the flap bob, the PARTY dance, and the
    /// five color variants are deferred.
    Parrot,
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
    Sheep {
        baby: bool,
        sheared: bool,
        wool_color: SheepWoolColor,
        invisible: bool,
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
        invisible: bool,
        collar_color: Option<EntityDyeColor>,
    },
    Horse {
        baby: bool,
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
    Quadruped {
        family: QuadrupedModelFamily,
        baby: bool,
    },
    Creeper,
    Spider,
    CaveSpider,
    Enderman,
    IronGolem,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CamelModelFamily {
    Camel,
    CamelHusk,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChickenModelVariant {
    Temperate,
    Warm,
    Cold,
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
