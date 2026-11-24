//! Per-kind client animation state: an entity carries only the state of its own kind.

use super::super::{is_vanilla_abstract_horse_type, is_vanilla_minecart_type};
use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoatAnimation {
    pub damage: Option<BoatDamageAnimationState>,
    pub bubble: Option<BoatBubbleAnimationState>,
    pub paddles: Option<BoatPaddleAnimationState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct MinecartAnimation {
    pub damage: Option<BoatDamageAnimationState>,
    pub tnt_fuse: Option<MinecartTntFuseAnimationState>,
    pub spawner: Option<MinecartSpawnerClientState>,
    #[serde(skip)]
    pub(crate) furnace_random: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ShulkerAnimation {
    pub peek: Option<ShulkerPeekAnimationState>,
    #[serde(skip)]
    pub(crate) teleport: Option<ShulkerTeleportAnimationState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct WolfAnimation {
    pub wet: Option<WolfWetAnimationState>,
    pub head_roll: Option<WolfHeadRollAnimationState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct FrogAnimation {
    pub croak: Option<KeyframeAnimationState>,
    pub tongue: Option<KeyframeAnimationState>,
    pub jump: Option<KeyframeAnimationState>,
    pub swim_idle: Option<KeyframeAnimationState>,
}

/// Vanilla `AbstractHorse` client state; `tail` also owns the shared `AbstractHorse.random`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EquineAnimation {
    pub tail: Option<EquineTailAnimationState>,
    pub pose: Option<EquinePoseAnimationState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CamelAnimation {
    pub equine: EquineAnimation,
    pub dash: Option<KeyframeAnimationState>,
    pub idle: Option<CamelIdleAnimationState>,
    pub dash_cooldown: Option<CamelDashCooldownState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct NautilusAnimation {
    #[serde(skip)]
    pub(crate) dash_cooldown: Option<NautilusDashCooldownState>,
    #[serde(skip)]
    pub(crate) random: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CopperGolemAnimation {
    pub idle: Option<CopperGolemIdleAnimationState>,
    pub get_item: Option<KeyframeAnimationState>,
    pub get_no_item: Option<KeyframeAnimationState>,
    pub drop_item: Option<KeyframeAnimationState>,
    pub drop_no_item: Option<KeyframeAnimationState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EnderDragonKindAnimation {
    pub state: Option<EnderDragonAnimationState>,
    #[serde(skip)]
    pub(crate) random_seed: u64,
    #[serde(skip)]
    pub(crate) random_initialized: bool,
    #[serde(skip)]
    pub(crate) last_phase_id: i8,
    #[serde(skip)]
    pub(crate) flame_ticks: u16,
    #[serde(skip)]
    pub(crate) death_phase_ticks: u16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct WardenAnimation {
    pub tendril: Option<WardenTendrilAnimationState>,
    pub heart: Option<WardenHeartAnimationState>,
    pub combat: Option<WardenCombatAnimationState>,
    #[serde(skip)]
    pub(crate) random: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct SlimeKindAnimation {
    pub state: Option<SlimeAnimationState>,
    #[serde(skip)]
    pub(crate) random: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct AllayAnimation {
    pub dance: Option<AllayDanceAnimationState>,
    pub holding_item: Option<AllayHoldingItemAnimationState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct GuardianAnimation {
    pub tail: Option<GuardianTailAnimationState>,
    pub spikes: Option<GuardianSpikesAnimationState>,
    pub attack: Option<GuardianAttackAnimationState>,
    #[serde(skip)]
    pub(crate) random: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct LightningBoltAnimation {
    #[serde(skip)]
    pub(crate) random: Option<u64>,
    #[serde(skip)]
    pub(crate) render_seed: Option<i64>,
}

/// The kind-specific part of [`EntityClientAnimationState`].
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum EntityKindAnimation {
    #[default]
    None,
    /// Vanilla `ItemEntity.bobOffs`.
    Item(f32),
    /// Vanilla `EndCrystal.time` constructor offset.
    EndCrystal(u32),
    Boat(BoatAnimation),
    Minecart(MinecartAnimation),
    Arrow(Option<ArrowShakeAnimationState>),
    PolarBear(Option<PolarBearStandingAnimationState>),
    Shulker(ShulkerAnimation),
    Bee(Option<BeeRollAnimationState>),
    Panda(Option<PandaAnimationState>),
    Fox(Option<FoxAnimationState>),
    Wolf(WolfAnimation),
    Cat(Option<CatLieDownAnimationState>),
    Wither(Option<WitherHeadRotationsState>),
    Bat(Option<KeyframeAnimationState>),
    Frog(FrogAnimation),
    Equine(EquineAnimation),
    Camel(CamelAnimation),
    Nautilus(NautilusAnimation),
    CopperGolem(CopperGolemAnimation),
    Sniffer(Option<SnifferAnimationState>),
    Armadillo(Option<ArmadilloAnimationState>),
    EnderDragon(EnderDragonKindAnimation),
    Sheep(Option<SheepEatAnimationState>),
    Goat(Option<GoatRammingAnimationState>),
    IronGolem(Option<IronGolemAnimationState>),
    Ravager(Option<RavagerAnimationState>),
    Hoglin(Option<HoglinAnimationState>),
    Rabbit(Option<RabbitHopAnimationState>),
    Creaking(Option<CreakingAnimationState>),
    ArmorStand(Option<ArmorStandWiggleAnimationState>),
    Player(Option<PlayerCloakAnimationState>),
    Creeper(Option<CreeperSwellAnimationState>),
    Warden(WardenAnimation),
    Squid(Option<SquidAnimationState>),
    Chicken(Option<ChickenFlapAnimationState>),
    Slime(SlimeKindAnimation),
    EvokerFangs(Option<EvokerFangsAnimationState>),
    Allay(AllayAnimation),
    Axolotl(Option<AxolotlAnimationState>),
    Parrot(Option<ParrotFlapAnimationState>),
    Guardian(GuardianAnimation),
    Breeze(Option<BreezeAnimationState>),
    /// Client-local `Blaze.random` cursor.
    Blaze {
        #[serde(skip)]
        random: Option<u64>,
    },
    /// Client-local RNG shared by the Enderman and Endermite portal particles,
    /// plus Enderman's 400-tick local stare sound cooldown (`EnderMan.java:173-189`).
    EnderAmbient {
        #[serde(skip)]
        random: Option<u64>,
        #[serde(skip)]
        last_stare_sound_tick: Option<u32>,
    },
    LightningBolt(LightningBoltAnimation),
}

impl EntityKindAnimation {
    pub(crate) fn for_entity_type(entity_type_id: i32) -> Self {
        use EntityKindAnimation as K;
        match entity_type_id {
            VANILLA_ENTITY_TYPE_ITEM_ID => K::Item(0.0),
            VANILLA_ENTITY_TYPE_END_CRYSTAL_ID => K::EndCrystal(0),
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => K::PolarBear(None),
            VANILLA_ENTITY_TYPE_SHULKER_ID => K::Shulker(Default::default()),
            VANILLA_ENTITY_TYPE_BEE_ID => K::Bee(None),
            VANILLA_ENTITY_TYPE_PANDA_ID => K::Panda(None),
            VANILLA_ENTITY_TYPE_FOX_ID => K::Fox(None),
            VANILLA_ENTITY_TYPE_WOLF_ID => K::Wolf(Default::default()),
            VANILLA_ENTITY_TYPE_CAT_ID => K::Cat(None),
            VANILLA_ENTITY_TYPE_WITHER_ID => K::Wither(None),
            VANILLA_ENTITY_TYPE_BAT_ID => K::Bat(None),
            VANILLA_ENTITY_TYPE_FROG_ID => K::Frog(Default::default()),
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => K::CopperGolem(Default::default()),
            VANILLA_ENTITY_TYPE_SNIFFER_ID => K::Sniffer(None),
            VANILLA_ENTITY_TYPE_ARMADILLO_ID => K::Armadillo(None),
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => K::EnderDragon(Default::default()),
            VANILLA_ENTITY_TYPE_SHEEP_ID => K::Sheep(None),
            VANILLA_ENTITY_TYPE_GOAT_ID => K::Goat(None),
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => K::IronGolem(None),
            VANILLA_ENTITY_TYPE_RAVAGER_ID => K::Ravager(None),
            VANILLA_ENTITY_TYPE_RABBIT_ID => K::Rabbit(None),
            VANILLA_ENTITY_TYPE_CREAKING_ID => K::Creaking(None),
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID => K::ArmorStand(None),
            VANILLA_ENTITY_TYPE_PLAYER_ID => K::Player(None),
            VANILLA_ENTITY_TYPE_CREEPER_ID => K::Creeper(None),
            VANILLA_ENTITY_TYPE_WARDEN_ID => K::Warden(Default::default()),
            VANILLA_ENTITY_TYPE_CHICKEN_ID => K::Chicken(None),
            VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID => K::EvokerFangs(None),
            VANILLA_ENTITY_TYPE_ALLAY_ID => K::Allay(Default::default()),
            VANILLA_ENTITY_TYPE_AXOLOTL_ID => K::Axolotl(None),
            VANILLA_ENTITY_TYPE_PARROT_ID => K::Parrot(None),
            VANILLA_ENTITY_TYPE_BREEZE_ID => K::Breeze(None),
            VANILLA_ENTITY_TYPE_BLAZE_ID => K::Blaze { random: None },
            VANILLA_ENTITY_TYPE_ENDERMAN_ID | VANILLA_ENTITY_TYPE_ENDERMITE_ID => K::EnderAmbient {
                random: None,
                last_stare_sound_tick: None,
            },
            VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID => K::LightningBolt(Default::default()),
            t if is_vanilla_boat_type(t) => K::Boat(Default::default()),
            t if is_vanilla_minecart_type(t) => K::Minecart(Default::default()),
            t if vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(t).is_some() => {
                K::Arrow(None)
            }
            t if vanilla_entity_metadata_type_is_camel(t) => K::Camel(Default::default()),
            t if is_vanilla_abstract_horse_type(t) => K::Equine(Default::default()),
            t if is_vanilla_abstract_nautilus_type(t) => K::Nautilus(Default::default()),
            t if vanilla_entity_metadata_type_is_hoglin_base(t) => K::Hoglin(None),
            t if vanilla_entity_type_is_squid(t) => K::Squid(None),
            t if vanilla_entity_metadata_type_is_slime(t) => K::Slime(Default::default()),
            t if vanilla_entity_metadata_type_is_guardian(t) => K::Guardian(Default::default()),
            _ => K::None,
        }
    }
}

/// Generates the per-variant [`EntityKindAnimation`] accessors: `$get` returns the variant's
/// state (by value for a single `Option` state) and `$get_mut` borrows it mutably.
macro_rules! kind_accessors {
    (
        options { $($ovariant:ident: $oty:ty => $oget:ident, $oget_mut:ident;)* }
        groups { $($gvariant:ident: $gty:ty => $gget:ident, $gget_mut:ident;)* }
    ) => {
        impl EntityClientAnimationState {
            $(
                pub(crate) fn $oget(&self) -> Option<$oty> {
                    match self.kind {
                        EntityKindAnimation::$ovariant(state) => state,
                        _ => None,
                    }
                }

                #[allow(dead_code)]
                pub(crate) fn $oget_mut(&mut self) -> Option<&mut Option<$oty>> {
                    match &mut self.kind {
                        EntityKindAnimation::$ovariant(state) => Some(state),
                        _ => None,
                    }
                }
            )*
            $(
                pub(crate) fn $gget(&self) -> Option<&$gty> {
                    match &self.kind {
                        EntityKindAnimation::$gvariant(state) => Some(state),
                        _ => None,
                    }
                }

                #[allow(dead_code)]
                pub(crate) fn $gget_mut(&mut self) -> Option<&mut $gty> {
                    match &mut self.kind {
                        EntityKindAnimation::$gvariant(state) => Some(state),
                        _ => None,
                    }
                }
            )*
        }
    };
}

kind_accessors! {
    options {
        Arrow: ArrowShakeAnimationState => arrow_kind, arrow_kind_mut;
        PolarBear: PolarBearStandingAnimationState => polar_bear_kind, polar_bear_kind_mut;
        Bee: BeeRollAnimationState => bee_kind, bee_kind_mut;
        Panda: PandaAnimationState => panda_kind, panda_kind_mut;
        Fox: FoxAnimationState => fox_kind, fox_kind_mut;
        Cat: CatLieDownAnimationState => cat_kind, cat_kind_mut;
        Wither: WitherHeadRotationsState => wither_kind, wither_kind_mut;
        Bat: KeyframeAnimationState => bat_kind, bat_kind_mut;
        Sniffer: SnifferAnimationState => sniffer_kind, sniffer_kind_mut;
        Armadillo: ArmadilloAnimationState => armadillo_kind, armadillo_kind_mut;
        Sheep: SheepEatAnimationState => sheep_kind, sheep_kind_mut;
        Goat: GoatRammingAnimationState => goat_kind, goat_kind_mut;
        IronGolem: IronGolemAnimationState => iron_golem_kind, iron_golem_kind_mut;
        Ravager: RavagerAnimationState => ravager_kind, ravager_kind_mut;
        Hoglin: HoglinAnimationState => hoglin_kind, hoglin_kind_mut;
        Rabbit: RabbitHopAnimationState => rabbit_kind, rabbit_kind_mut;
        Creaking: CreakingAnimationState => creaking_kind, creaking_kind_mut;
        ArmorStand: ArmorStandWiggleAnimationState => armor_stand_kind, armor_stand_kind_mut;
        Player: PlayerCloakAnimationState => player_kind, player_kind_mut;
        Creeper: CreeperSwellAnimationState => creeper_kind, creeper_kind_mut;
        Squid: SquidAnimationState => squid_kind, squid_kind_mut;
        Chicken: ChickenFlapAnimationState => chicken_kind, chicken_kind_mut;
        EvokerFangs: EvokerFangsAnimationState => evoker_fangs_kind, evoker_fangs_kind_mut;
        Axolotl: AxolotlAnimationState => axolotl_kind, axolotl_kind_mut;
        Parrot: ParrotFlapAnimationState => parrot_kind, parrot_kind_mut;
        Breeze: BreezeAnimationState => breeze_kind, breeze_kind_mut;
    }
    groups {
        Boat: BoatAnimation => boat_kind, boat_kind_mut;
        Minecart: MinecartAnimation => minecart_kind, minecart_kind_mut;
        Shulker: ShulkerAnimation => shulker_kind, shulker_kind_mut;
        Wolf: WolfAnimation => wolf_kind, wolf_kind_mut;
        Frog: FrogAnimation => frog_kind, frog_kind_mut;
        Camel: CamelAnimation => camel_kind, camel_kind_mut;
        Nautilus: NautilusAnimation => nautilus_kind, nautilus_kind_mut;
        CopperGolem: CopperGolemAnimation => copper_golem_kind, copper_golem_kind_mut;
        EnderDragon: EnderDragonKindAnimation => ender_dragon_kind, ender_dragon_kind_mut;
        Warden: WardenAnimation => warden_kind, warden_kind_mut;
        Slime: SlimeKindAnimation => slime_kind, slime_kind_mut;
        Allay: AllayAnimation => allay_kind, allay_kind_mut;
        Guardian: GuardianAnimation => guardian_kind, guardian_kind_mut;
        LightningBolt: LightningBoltAnimation => lightning_bolt_kind, lightning_bolt_kind_mut;
    }
}

impl EntityClientAnimationState {
    /// Installs the kind state when the entity has none yet (records rebuilt without spawn
    /// defaults, or reset on respawn).
    pub(crate) fn ensure_kind(&mut self, entity_type_id: i32) {
        if matches!(self.kind, EntityKindAnimation::None) {
            self.kind = EntityKindAnimation::for_entity_type(entity_type_id);
        }
    }

    /// The `AbstractHorse` state shared by equines, llamas and camels.
    pub(crate) fn equine_kind(&self) -> Option<&EquineAnimation> {
        match &self.kind {
            EntityKindAnimation::Equine(equine) => Some(equine),
            EntityKindAnimation::Camel(camel) => Some(&camel.equine),
            _ => None,
        }
    }

    pub(crate) fn equine_kind_mut(&mut self) -> Option<&mut EquineAnimation> {
        match &mut self.kind {
            EntityKindAnimation::Equine(equine) => Some(equine),
            EntityKindAnimation::Camel(camel) => Some(&mut camel.equine),
            _ => None,
        }
    }

    /// Vanilla `VehicleEntity` hurt-roll state shared by boats and minecarts.
    pub(crate) fn vehicle_damage(&self) -> Option<BoatDamageAnimationState> {
        match &self.kind {
            EntityKindAnimation::Boat(boat) => boat.damage,
            EntityKindAnimation::Minecart(minecart) => minecart.damage,
            _ => None,
        }
    }

    pub(crate) fn vehicle_damage_mut(&mut self) -> Option<&mut Option<BoatDamageAnimationState>> {
        match &mut self.kind {
            EntityKindAnimation::Boat(boat) => Some(&mut boat.damage),
            EntityKindAnimation::Minecart(minecart) => Some(&mut minecart.damage),
            _ => None,
        }
    }
}
