//! The per-type half of a mob: each entity carries only the state its type
//! declares, and save, load, synched data and command restore all read it here.

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID, VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
    VANILLA_ABSTRACT_HORSE_FLAG_BRED, VANILLA_ABSTRACT_HORSE_FLAG_EATING,
    VANILLA_ABSTRACT_HORSE_FLAG_TAME, VANILLA_ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID,
    VANILLA_AXOLOTL_FROM_BUCKET_DATA_ID, VANILLA_AXOLOTL_PLAYING_DEAD_DATA_ID,
    VANILLA_BAT_FLAGS_DATA_ID, VANILLA_CREEPER_IGNITED_DATA_ID, VANILLA_CREEPER_POWERED_DATA_ID,
    VANILLA_CREEPER_SWELL_DIR_DATA_ID, VANILLA_DOLPHIN_GOT_FISH_DATA_ID,
    VANILLA_DOLPHIN_MOISTNESS_DATA_ID, VANILLA_ENDERMAN_CARRY_STATE_DATA_ID,
    VANILLA_ENDERMAN_CREEPY_DATA_ID, VANILLA_ENDERMAN_STARED_AT_DATA_ID,
    VANILLA_GHAST_CHARGING_DATA_ID, VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID,
    VANILLA_GUARDIAN_ATTACK_TARGET_DATA_ID, VANILLA_GUARDIAN_MOVING_DATA_ID,
    VANILLA_HAPPY_GHAST_IS_LEASH_HOLDER_DATA_ID, VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID,
    VANILLA_OCELOT_TRUSTING_DATA_ID, VANILLA_POLAR_BEAR_STANDING_DATA_ID,
    VANILLA_STRIDER_BOOST_TIME_DATA_ID, VANILLA_STRIDER_SUFFOCATING_DATA_ID,
    VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID, VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID,
    VANILLA_VEX_FLAGS_DATA_ID,
};
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};

use super::type_variants::VillagerState;
use super::*;
use crate::entity::armadillo::ArmadilloState;
use crate::entity::bee::BeeRetainedState;
use crate::entity::blaze::BlazeRetainedState;
use crate::entity::camel::CamelState;
use crate::entity::collar::CollarRetainedState;
use crate::entity::creaking::CreakingRetainedState;
use crate::entity::endermite::EndermiteRetainedState;
use crate::entity::fish::{FishLeafState, SchoolingFishState};
use crate::entity::fox::{FoxRetainedState, FoxVariant};
use crate::entity::game_event_listener::EntityGameEventListenerState;
use crate::entity::ghast::GhastRetainedState;
use crate::entity::goat::GoatRetainedState;
use crate::entity::horse_phenotype::HorsePhenotypeState;
use crate::entity::living::SynchedValue;
use crate::entity::llama::CaravanState;
use crate::entity::merchant::AbstractVillagerRetainedState;
use crate::entity::panda::PandaRetainedState;
use crate::entity::phantom::PhantomRetainedState;
use crate::entity::piglin::CelebrationState;
use crate::entity::rabbit::RabbitRetainedState;
use crate::entity::raider::RaiderRetainedState;
use crate::entity::save::{optional_bool, optional_int, EntitySaveError};
use crate::entity::sheep::SheepRetainedState;
use crate::entity::skeleton::SkeletonRetainedState;
use crate::entity::skeleton_horse::SkeletonHorseTrapState;
use crate::entity::slime::SlimeRetainedState;
use crate::entity::strider::StriderState;
use crate::entity::tadpole::TadpoleRetainedState;
use crate::entity::turtle::TurtleRetainedState;
use crate::entity::wandering_trader::TraderState;
use crate::entity::witch::WitchState;
use crate::entity::wither::WitherRetainedState;
use crate::entity::zombie::villager::ZombieVillagerCure;
use crate::entity::zombie::ZombieRetainedState;

/// The fixed first countdown of a new chicken or armadillo; the redraws are
/// `nextInt(6000) + 6000` (`Chicken.java:75`, `Armadillo.java:157-159`).
pub(crate) const GIFT_DROP_INTERVAL: i32 = 6000;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CatRestState {
    lying: bool,
    relax: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MobVariantState {
    Plain,
    AgeableWater(AgeableWaterCreatureRetainedState),
    Armadillo(ArmadilloState),
    Axolotl {
        from_bucket: bool,
        playing_dead: bool,
    },
    Bat(BatRetainedState),
    Bee(BeeRetainedState),
    Blaze(BlazeRetainedState),
    /// `Chicken.isChickenJockey` and `Chicken.eggTime` (`Chicken.java:70-71`).
    Chicken {
        jockey: bool,
        egg_time: i32,
    },
    Ocelot {
        trusting: bool,
    },
    CopperGolem(crate::entity::copper_golem::CopperGolemState),
    Creaking(CreakingRetainedState),
    Creeper(CreeperRetainedState),
    EnderDragon(Box<crate::entity::ender_dragon::EnderDragonState>),
    Enderman(EndermanRetainedState),
    Endermite(EndermiteRetainedState),
    Fish(FishState),
    Fox(FoxRetainedState, Option<FoxVariant>),
    GameEventListener(Box<EntityGameEventListenerState>),
    Warden(Box<crate::entity::warden::anger::WardenState>),
    Ghast(GhastRetainedState),
    Goat(GoatRetainedState),
    Guardian(crate::entity::guardian::GuardianState),
    HappyGhast(HappyGhastState),
    Hoglin {
        experience_reward: i32,
        state: crate::entity::hoglin::HoglinState,
        zombification: crate::entity::zombification::ZombificationState,
        cannot_be_hunted: bool,
    },
    Horse(HorseState),
    /// `IronGolem.DATA_FLAGS_ID` bit 1 (`IronGolem.java:52`, `:286-297`).
    IronGolem {
        player_created: bool,
    },
    Panda(PandaRetainedState),
    Phantom(PhantomRetainedState),
    Pig(crate::entity::item_steering::ItemSteering),
    Piglin {
        baby: bool,
        zombification: crate::entity::zombification::ZombificationState,
        celebration: Option<CelebrationState>,
        /// `Piglin.DATA_IS_CHARGING_CROSSBOW` (`Piglin.java:65`), never saved.
        charging_crossbow: bool,
        cannot_hunt: bool,
    },
    PiglinBrute(crate::entity::zombification::ZombificationState),
    /// `PolarBear.DATA_STANDING_ID` (`PolarBear.java:56`), never saved.
    PolarBear {
        standing: bool,
    },
    Rabbit(RabbitRetainedState),
    Raider(RaiderRetainedState),
    Sheep(SheepRetainedState),
    Shulker(crate::entity::shulker::ShulkerState),
    Skeleton(SkeletonRetainedState),
    Slime(SlimeRetainedState),
    Sniffer(crate::entity::sniffer::SnifferState),
    Strider(StriderState),
    Tadpole(TadpoleRetainedState),
    Tamable(
        TamableRetainedState,
        Option<CollarRetainedState>,
        Option<WolfRetainedState>,
        Option<crate::entity::nautilus::NautilusDash>,
        Option<CatRestState>,
    ),
    Turtle(TurtleRetainedState),
    Vex(VexRetainedState),
    Villager(AbstractVillagerRetainedState, VillagerState),
    WanderingTrader(AbstractVillagerRetainedState, TraderState),
    Witch(RaiderRetainedState, WitchState),
    Wither(WitherRetainedState),
    Zoglin {
        baby: bool,
    },
    Zombie(ZombieRetainedState),
    ZombieVillager(ZombieRetainedState, VillagerState, ZombieVillagerCure),
    Plugin(crate::entity::plugin_mob::PluginMobState),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WolfRetainedState {
    interested: bool,
    pub(crate) shake: crate::entity::wolf::WolfShake,
}

/// `AbstractFish` and its leaves; Cod carries no leaf, Pufferfish no school.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FishState {
    pub(crate) from_bucket: bool,
    pub(crate) leaf: Option<FishLeafState>,
    pub(crate) school: Option<SchoolingFishState>,
    pub(crate) tropical_school: bool,
    pub(crate) puff: crate::entity::fish::pufferfish::PuffRuntime,
}

/// `AbstractHorse` and the pieces its leaves add.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HorseState {
    pub(crate) horse: AbstractHorseRetainedState,
    pub(crate) phenotype: Option<HorsePhenotypeState>,
    pub(crate) trap: Option<SkeletonHorseTrapState>,
    pub(crate) camel: Option<CamelState>,
    pub(crate) trader: Option<TraderState>,
    pub(crate) caravan: Option<CaravanState>,
}

impl MobVariantState {
    fn zombification(&self) -> Option<&crate::entity::zombification::ZombificationState> {
        match self {
            Self::Hoglin { zombification, .. }
            | Self::Piglin { zombification, .. }
            | Self::PiglinBrute(zombification) => Some(zombification),
            _ => None,
        }
    }

    fn zombification_mut(
        &mut self,
    ) -> Option<&mut crate::entity::zombification::ZombificationState> {
        match self {
            Self::Hoglin { zombification, .. }
            | Self::Piglin { zombification, .. }
            | Self::PiglinBrute(zombification) => Some(zombification),
            _ => None,
        }
    }

    pub(super) fn new(entity_type_id: i32) -> Self {
        use crate::entity::*;
        if let Some(horse) = AbstractHorseRetainedState::new(entity_type_id) {
            return Self::Horse(HorseState {
                horse,
                phenotype: horse_phenotype::HorsePhenotypeState::new(entity_type_id),
                trap: skeleton_horse::SkeletonHorseTrapState::new(entity_type_id),
                camel: camel::CamelState::new(entity_type_id),
                trader: (entity_type_id == VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID)
                    .then(|| TraderState::new(entity_type_id)),
                caravan: llama::CaravanState::new(entity_type_id),
            });
        }
        if let Some(tamable) = TamableRetainedState::new(entity_type_id) {
            return Self::Tamable(
                tamable,
                collar::CollarRetainedState::new(entity_type_id),
                (entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID).then(WolfRetainedState::default),
                crate::entity::nautilus::NautilusDash::new(entity_type_id),
                (entity_type_id == VANILLA_ENTITY_TYPE_CAT_ID).then(CatRestState::default),
            );
        }
        if let Some(state) = AgeableWaterCreatureRetainedState::new(entity_type_id) {
            return Self::AgeableWater(state);
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_CHICKEN_ID {
            return Self::Chicken {
                jockey: false,
                egg_time: GIFT_DROP_INTERVAL,
            };
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_OCELOT_ID {
            return Self::Ocelot { trusting: false };
        }
        if is_water_animal_entity_type(entity_type_id)
            && entity_type_id != VANILLA_ENTITY_TYPE_TADPOLE_ID
        {
            return Self::Fish(FishState {
                from_bucket: false,
                leaf: fish::FishLeafState::new(entity_type_id),
                school: (entity_type_id != VANILLA_ENTITY_TYPE_PUFFERFISH_ID)
                    .then(SchoolingFishState::new),
                tropical_school: entity_type_id == VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
                puff: Default::default(),
            });
        }
        if let Some(raider) = raider::RaiderRetainedState::new(entity_type_id) {
            return match witch::WitchState::new(entity_type_id) {
                Some(witch) => Self::Witch(raider, witch),
                None => Self::Raider(raider),
            };
        }
        if let Some(zombie) = zombie::ZombieRetainedState::new(entity_type_id) {
            return if entity_type_id == VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID {
                Self::ZombieVillager(
                    zombie,
                    VillagerState::zombie_villager(),
                    ZombieVillagerCure::default(),
                )
            } else {
                Self::Zombie(zombie)
            };
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID {
            return Self::Warden(Box::new(crate::entity::warden::anger::WardenState::new()));
        }
        if let Some(listener) =
            game_event_listener::EntityGameEventListenerState::new(entity_type_id)
        {
            return Self::GameEventListener(Box::new(listener));
        }
        match entity_type_id {
            VANILLA_ENTITY_TYPE_BAT_ID => Self::Bat(BatRetainedState::from_saved(1)),
            VANILLA_ENTITY_TYPE_BEE_ID => Self::Bee(Default::default()),
            VANILLA_ENTITY_TYPE_BLAZE_ID => Self::Blaze(blaze::BlazeRetainedState::new()),
            VANILLA_ENTITY_TYPE_CREAKING_ID => {
                Self::Creaking(creaking::CreakingRetainedState::from_saved(None))
            }
            VANILLA_ENTITY_TYPE_CREEPER_ID => {
                Self::Creeper(CreeperRetainedState::from_saved(false, 30, 3, false))
            }
            VANILLA_ENTITY_TYPE_ENDERMAN_ID => {
                Self::Enderman(EndermanRetainedState::from_saved(None))
            }
            VANILLA_ENTITY_TYPE_ENDERMITE_ID => {
                Self::Endermite(endermite::EndermiteRetainedState::from_saved(0))
            }
            VANILLA_ENTITY_TYPE_GHAST_ID => Self::Ghast(ghast::GhastRetainedState::from_saved(1)),
            VANILLA_ENTITY_TYPE_GOAT_ID => Self::Goat(Default::default()),
            VANILLA_ENTITY_TYPE_GUARDIAN_ID | VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => {
                Self::Guardian(guardian::GuardianState::new())
            }
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => Self::HappyGhast(Default::default()),
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => Self::IronGolem {
                player_created: false,
            },
            VANILLA_ENTITY_TYPE_PANDA_ID => Self::Panda(Default::default()),
            VANILLA_ENTITY_TYPE_PHANTOM_ID => {
                Self::Phantom(phantom::PhantomRetainedState::from_saved(0, None))
            }
            VANILLA_ENTITY_TYPE_RABBIT_ID => Self::Rabbit(rabbit::RabbitRetainedState::from_saved(
                bbb_protocol::builtin_enum_component_values::VANILLA_RABBIT_VARIANT_BROWN_ID,
                0,
            )),
            VANILLA_ENTITY_TYPE_SHEEP_ID => {
                Self::Sheep(sheep::SheepRetainedState::from_saved(0, false))
            }
            VANILLA_ENTITY_TYPE_SHULKER_ID => Self::Shulker(shulker::ShulkerState::new()),
            VANILLA_ENTITY_TYPE_SKELETON_ID => {
                Self::Skeleton(skeleton::SkeletonRetainedState::new())
            }
            VANILLA_ENTITY_TYPE_SLIME_ID | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => {
                Self::Slime(slime::SlimeRetainedState::from_saved(1, false))
            }
            VANILLA_ENTITY_TYPE_SNIFFER_ID => Self::Sniffer(sniffer::SnifferState::new()),
            VANILLA_ENTITY_TYPE_STRIDER_ID => Self::Strider(strider::StriderState::new()),
            VANILLA_ENTITY_TYPE_TADPOLE_ID => {
                Self::Tadpole(tadpole::TadpoleRetainedState::from_saved(0, false))
            }
            VANILLA_ENTITY_TYPE_TURTLE_ID => Self::Turtle(turtle::TurtleRetainedState::new()),
            VANILLA_ENTITY_TYPE_VEX_ID => Self::Vex(VexRetainedState::from_saved(None, None, None)),
            VANILLA_ENTITY_TYPE_WITHER_ID => {
                Self::Wither(wither::WitherRetainedState::from_saved(0))
            }
            VANILLA_ENTITY_TYPE_FOX_ID => Self::Fox(Default::default(), None),
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => Self::EnderDragon(Box::default()),
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => Self::CopperGolem(Default::default()),
            VANILLA_ENTITY_TYPE_AXOLOTL_ID => Self::Axolotl {
                from_bucket: false,
                playing_dead: false,
            },
            VANILLA_ENTITY_TYPE_ARMADILLO_ID => {
                Self::Armadillo(ArmadilloState::new(GIFT_DROP_INTERVAL))
            }
            VANILLA_ENTITY_TYPE_HOGLIN_ID => Self::Hoglin {
                experience_reward: 5,
                state: Default::default(),
                zombification: Default::default(),
                cannot_be_hunted: false,
            },
            VANILLA_ENTITY_TYPE_PIG_ID => {
                Self::Pig(crate::entity::item_steering::ItemSteering::new())
            }
            VANILLA_ENTITY_TYPE_PIGLIN_ID => Self::Piglin {
                baby: false,
                zombification: Default::default(),
                celebration: None,
                charging_crossbow: false,
                cannot_hunt: false,
            },
            VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => Self::PiglinBrute(Default::default()),
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => Self::PolarBear { standing: false },
            VANILLA_ENTITY_TYPE_ZOGLIN_ID => Self::Zoglin { baby: false },
            VANILLA_ENTITY_TYPE_VILLAGER_ID => {
                Self::Villager(Default::default(), VillagerState::villager())
            }
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID => {
                Self::WanderingTrader(Default::default(), TraderState::new(entity_type_id))
            }
            _ => Self::Plain,
        }
    }

    pub(super) fn read_save(
        &mut self,
        entity_type_id: i32,
        entries: &[NbtSummaryEntry],
        position: [f64; 3],
        items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) {
        use crate::entity::*;
        let id = entity_type_id;
        if let Some(state) = self.zombification_mut() {
            *state = crate::entity::zombification::ZombificationState::read_save(entries);
        }
        let from_bucket = || optional_bool(entries, "FromBucket").unwrap_or(false);
        match self {
            Self::Plain
            | Self::Sniffer(_)
            | Self::Guardian(_)
            | Self::Blaze(_)
            | Self::Strider(_)
            | Self::Pig(_)
            | Self::PolarBear { .. } => {}
            Self::CopperGolem(state) => state.read_save(entries),
            Self::Plugin(plugin) => plugin.read_save_entries(entries),
            Self::Hoglin {
                cannot_be_hunted, ..
            } => {
                *cannot_be_hunted = optional_bool(entries, "CannotBeHunted").unwrap_or(false);
            }
            Self::PiglinBrute(_) => {}
            Self::AgeableWater(state) => {
                *state = match state {
                    AgeableWaterCreatureRetainedState::Dolphin { .. } => {
                        AgeableWaterCreatureRetainedState::from_saved_dolphin(
                            optional_bool(entries, "GotFish").unwrap_or(false),
                            optional_int(entries, "Moistness").unwrap_or(2400),
                        )
                    }
                    AgeableWaterCreatureRetainedState::GlowSquid { .. } => {
                        AgeableWaterCreatureRetainedState::from_saved_glow_squid(
                            optional_int(entries, "DarkTicksRemaining").unwrap_or(0),
                        )
                    }
                    AgeableWaterCreatureRetainedState::Squid => {
                        AgeableWaterCreatureRetainedState::Squid
                    }
                }
            }
            Self::Axolotl {
                from_bucket: value, ..
            } => *value = from_bucket(),
            Self::Armadillo(state) => state.read_save(entries),
            Self::Chicken { jockey, egg_time } => {
                *jockey = optional_bool(entries, "IsChickenJockey").unwrap_or(false);
                if let Some(time) = optional_int(entries, "EggLayTime") {
                    *egg_time = time;
                }
            }
            Self::Bat(state) => *state = ai::mob::bat::read_save_data(entries),
            Self::Bee(state) => *state = bee::read_save_data(entries),
            Self::Ocelot { trusting } => {
                *trusting = optional_bool(entries, "Trusting").unwrap_or(false);
            }
            Self::Creaking(state) => *state = creaking::read_save_data(entries),
            Self::Creeper(state) => *state = ai::mob::creeper::read_save_data(entries),
            Self::Enderman(state) => {
                *state = EndermanRetainedState::from_saved(
                    crate::entity::save::optional_field(entries, "carriedBlockState")
                        .and_then(|value| {
                            crate::entity::save::read_block_state_from_value_with_registry(
                                value,
                                items.content_registry(),
                            )
                        })
                        .filter(|state| !block_state_is_air(*state)),
                );
            }
            Self::Endermite(state) => *state = endermite::read_save_data(entries),
            Self::Fish(fish) => {
                fish.from_bucket = from_bucket();
                if fish.leaf.is_some() {
                    fish.leaf = fish::read_save_data(id, entries);
                }
            }
            Self::Fox(state, _) => *state = fox::read_save_data(entries),
            Self::GameEventListener(state) => state.read_save_data(entries),
            Self::Warden(state) => state.read_save(entries),
            Self::Ghast(state) => *state = ghast::read_save_data(entries),
            Self::Goat(state) => *state = goat::read_save_data(entries),
            Self::HappyGhast(state) => {
                state.set_still_timeout(optional_int(entries, "still_timeout").unwrap_or(0));
            }
            Self::IronGolem { player_created } => {
                *player_created = optional_bool(entries, "PlayerCreated").unwrap_or(false);
            }
            Self::Horse(horse) => {
                horse.horse = AbstractHorseRetainedState::from_saved(
                    optional_bool(entries, "EatingHaystack").unwrap_or(false),
                    optional_bool(entries, "Bred").unwrap_or(false),
                    optional_int(entries, "Temper").unwrap_or(0),
                    optional_bool(entries, "Tame").unwrap_or(false),
                    crate::entity::save::optional_uuid(entries, "Owner"),
                );
                horse.phenotype = horse_phenotype::read_save_data(id, entries);
                horse.trap = skeleton_horse::read_save_data(id, entries);
                horse.camel = camel::read_save_data(id, entries);
                if let Some(trader) = &mut horse.trader {
                    *trader = wandering_trader::read_save_data(id, entries);
                }
            }
            Self::Panda(state) => *state = panda::read_save_data(entries),
            Self::Phantom(state) => *state = phantom::read_save_data(entries),
            Self::Piglin {
                baby, cannot_hunt, ..
            } => {
                *baby = optional_bool(entries, "IsBaby").unwrap_or(false);
                *cannot_hunt = optional_bool(entries, "CannotHunt").unwrap_or(false);
            }
            Self::Zoglin { baby } => {
                *baby = optional_bool(entries, "IsBaby").unwrap_or(false);
            }
            Self::Rabbit(state) => *state = rabbit::read_save_data(entries),
            Self::Raider(state) | Self::Witch(state, _) => {
                *state = raider::read_save_data(id, entries);
            }
            Self::Shulker(state) => *state = shulker::ShulkerState::read_save(entries),
            Self::Sheep(state) => *state = sheep::read_save_data(entries),
            Self::Skeleton(state) => *state = skeleton::read_save_data(entries),
            Self::Slime(state) => *state = slime::read_save_data(entries),
            Self::Tadpole(state) => *state = tadpole::read_save_data(entries),
            Self::Tamable(tamable, collar, _, _, _) => {
                *tamable = TamableRetainedState::from_saved(
                    crate::entity::save::optional_uuid(entries, "Owner"),
                    optional_bool(entries, "Sitting").unwrap_or(false),
                );
                if collar.is_some() {
                    *collar = collar::read_save_data(id, entries);
                }
            }
            Self::Turtle(state) => *state = turtle::read_save_data(entries, position),
            Self::Vex(state) => *state = ai::mob::vex::read_save_data(entries),
            Self::Villager(merchant, villager) => {
                *merchant = merchant::read_save_data(entries, items);
                villager.read_save(entries);
            }
            Self::WanderingTrader(merchant, trader) => {
                *merchant = merchant::read_save_data(entries, items);
                *trader = wandering_trader::read_save_data(id, entries);
            }
            Self::Wither(state) => *state = wither::read_save_data(entries),
            Self::EnderDragon(state) => ender_dragon::read_save_data(state, entries),
            Self::Zombie(state) => *state = zombie::read_save_data(entries),
            Self::ZombieVillager(state, villager, cure) => {
                *state = zombie::read_save_data(entries);
                villager.read_save(entries);
                *cure = ZombieVillagerCure::read_save(entries);
            }
        }
    }

    pub(super) fn write_save(
        &self,
        entries: &mut Vec<NbtSummaryEntry>,
        entity: &Entity,
        data: &GeneralLivingData,
        items: crate::entity::item::ItemResolver<'_>,
    ) -> Result<(), EntitySaveError> {
        use crate::entity::save::{byte_bool, uuid_int_array, write_block_pos};
        use crate::entity::*;
        if let Some(state) = self.zombification() {
            state.write_save(entries);
        }
        let mut push = |name: &str, value: NbtSummaryValue| {
            entries.push(NbtSummaryEntry {
                name: name.into(),
                value,
            })
        };
        match self {
            Self::Plain
            | Self::Sniffer(_)
            | Self::Guardian(_)
            | Self::Blaze(_)
            | Self::Strider(_)
            | Self::Pig(_)
            | Self::PiglinBrute(_)
            | Self::PolarBear { .. }
            | Self::IronGolem { .. } => {}
            Self::CopperGolem(state) => state.write_save(entries),
            Self::Hoglin {
                cannot_be_hunted, ..
            } => push("CannotBeHunted", byte_bool(*cannot_be_hunted)),
            Self::Plugin(plugin) => entries.push(plugin.write_save_entry()),
            Self::AgeableWater(state) => match *state {
                AgeableWaterCreatureRetainedState::Dolphin {
                    got_fish,
                    moistness,
                    ..
                } => {
                    push("GotFish", byte_bool(got_fish));
                    push("Moistness", NbtSummaryValue::Int(moistness));
                }
                AgeableWaterCreatureRetainedState::GlowSquid {
                    dark_ticks_remaining,
                } => push(
                    "DarkTicksRemaining",
                    NbtSummaryValue::Int(dark_ticks_remaining),
                ),
                AgeableWaterCreatureRetainedState::Squid => {}
            },
            Self::Axolotl { from_bucket, .. } => push("FromBucket", byte_bool(*from_bucket)),
            Self::Armadillo(state) => state.write_save(entries),
            Self::Chicken { jockey, egg_time } => {
                push("IsChickenJockey", byte_bool(*jockey));
                push("EggLayTime", NbtSummaryValue::Int(*egg_time));
            }
            Self::Bat(state) => ai::mob::bat::write_save_data(entries, *state),
            Self::Bee(state) => bee::write_save_data(entries, state),
            Self::Ocelot { trusting } => push("Trusting", byte_bool(*trusting)),
            Self::Creaking(state) => creaking::write_save_data(entries, state),
            Self::Creeper(creeper) => {
                push("powered", byte_bool(creeper.powered()));
                push("Fuse", NbtSummaryValue::Short(creeper.max_swell() as i16));
                push(
                    "ExplosionRadius",
                    NbtSummaryValue::Byte(creeper.explosion_radius() as i8),
                );
                push("ignited", byte_bool(creeper.ignited()));
            }
            Self::Enderman(state) => {
                if let Some(block_state_id) = state.carried_block_state_id() {
                    push(
                        "carriedBlockState",
                        crate::entity::save::block_state_value_with_registry(
                            block_state_id,
                            "carriedBlockState",
                            items.content_registry(),
                        )?,
                    );
                }
            }
            Self::Endermite(state) => endermite::write_save_data(entries, *state),
            Self::Fish(fish) => {
                push("FromBucket", byte_bool(fish.from_bucket));
                if let Some(leaf) = fish.leaf {
                    fish::write_save_data(entries, leaf);
                }
            }
            Self::Fox(state, _) => fox::write_save_data(entries, state),
            Self::GameEventListener(state) => game_event_listener::write_save_data(entries, state),
            Self::Warden(state) => state.write_save(entries),
            Self::Shulker(state) => state.write_save(entries),
            Self::Ghast(state) => ghast::write_save_data(entries, *state),
            Self::Goat(state) => goat::write_save_data(entries, state),
            Self::HappyGhast(state) => {
                push("still_timeout", NbtSummaryValue::Int(state.still_timeout))
            }
            Self::Horse(horse) => {
                if let Some(trader) = &horse.trader {
                    wandering_trader::write_save_data(entries, trader);
                }
                let mut push = |name: &str, value: NbtSummaryValue| {
                    entries.push(NbtSummaryEntry {
                        name: name.into(),
                        value,
                    })
                };
                let state = &horse.horse;
                push("EatingHaystack", byte_bool(state.is_eating()));
                push("Bred", byte_bool(state.is_bred()));
                push("Temper", NbtSummaryValue::Int(state.temper()));
                push("Tame", byte_bool(state.is_tame()));
                if let Some(owner) = state.owner() {
                    push("Owner", uuid_int_array(owner));
                }
                if let Some(phenotype) = horse.phenotype {
                    horse_phenotype::write_save_data(
                        entries,
                        phenotype,
                        data.internal_inventory()
                            .expect("horses own mount cargo")
                            .slots(),
                        items,
                    );
                }
                if let Some(trap) = horse.trap {
                    skeleton_horse::write_save_data(entries, trap);
                }
                if let Some(camel) = horse.camel {
                    camel::write_save_data(entries, camel);
                }
            }
            Self::Panda(state) => panda::write_save_data(entries, state),
            Self::Phantom(state) => phantom::write_save_data(entries, *state),
            Self::Piglin {
                baby, cannot_hunt, ..
            } => {
                push("IsBaby", NbtSummaryValue::Byte(i8::from(*baby)));
                push("CannotHunt", byte_bool(*cannot_hunt));
            }
            Self::Zoglin { baby } => {
                push("IsBaby", NbtSummaryValue::Byte(i8::from(*baby)));
            }
            Self::Rabbit(state) => rabbit::write_save_data(entries, *state),
            Self::Raider(state) | Self::Witch(state, _) => raider::write_save_data(entries, state),
            Self::Sheep(state) => sheep::write_save_data(entries, *state),
            Self::Skeleton(state) => skeleton::write_save_data(entries, *state),
            Self::Slime(state) => slime::write_save_data(entries, *state),
            Self::Tadpole(state) => {
                push("FromBucket", byte_bool(true));
                tadpole::write_save_data(entries, *state);
            }
            Self::Tamable(tamable, collar, _, _, _) => {
                if let Some(owner) = tamable.owner() {
                    push("Owner", uuid_int_array(owner));
                }
                push("Sitting", byte_bool(tamable.ordered_to_sit()));
                if let Some(collar) = collar {
                    collar::write_save_data(entries, *collar);
                }
            }
            Self::Turtle(state) => turtle::write_save_data(entries, state),
            Self::Vex(vex) => {
                if let Some(bound_origin) = vex.bound_origin() {
                    push("bound_pos", write_block_pos(bound_origin));
                }
                if let Some(limited_life_ticks) = vex.limited_life_ticks() {
                    push("life_ticks", NbtSummaryValue::Int(limited_life_ticks));
                }
                if let Some(owner) = vex.owner() {
                    push("owner", uuid_int_array(owner));
                }
            }
            Self::Villager(merchant, villager) => {
                villager.write_save(entries);
                merchant::write_save_data(
                    entries,
                    merchant,
                    entity.entity_type_id(),
                    Some((villager.profession(), villager.level())),
                    items,
                )?;
            }
            Self::WanderingTrader(merchant, trader) => {
                wandering_trader::write_save_data(entries, trader);
                merchant::write_save_data(entries, merchant, entity.entity_type_id(), None, items)?;
            }
            Self::Wither(state) => wither::write_save_data(entries, *state),
            Self::EnderDragon(state) => ender_dragon::write_save_data(entries, state),
            Self::Zombie(state) => {
                zombie::write_save_data(entries, *state, entity.was_touching_water());
            }
            Self::ZombieVillager(state, villager, cure) => {
                zombie::write_save_data(entries, *state, entity.was_touching_water());
                villager.write_save(entries);
                cure.write_save(entries);
            }
        }
        Ok(())
    }

    /// The save fields this state reads back, which the opaque remainder
    /// must not duplicate.
    pub(super) fn owns_save_field(&self, name: &str) -> bool {
        use crate::entity::*;
        if self.zombification().is_some()
            && crate::entity::zombification::ZombificationState::owns_save_field(name)
        {
            return true;
        }
        match self {
            Self::Plain
            | Self::Sniffer(_)
            | Self::Guardian(_)
            | Self::Blaze(_)
            | Self::Strider(_)
            | Self::Pig(_)
            | Self::PiglinBrute(_)
            | Self::PolarBear { .. }
            | Self::IronGolem { .. } => false,
            Self::CopperGolem(_) => {
                crate::entity::copper_golem::CopperGolemState::owns_save_field(name)
            }
            Self::Hoglin { .. } => name == "CannotBeHunted",
            Self::Plugin(_) => name == "bbb_plugin",
            // `home_pos` is the Mob field this state shares.
            Self::Creaking(_) => false,
            Self::AgeableWater(state) => matches!(
                (state, name),
                (
                    AgeableWaterCreatureRetainedState::Dolphin { .. },
                    "GotFish" | "Moistness"
                ) | (
                    AgeableWaterCreatureRetainedState::GlowSquid { .. },
                    "DarkTicksRemaining"
                )
            ),
            Self::Axolotl { .. } => name == "FromBucket",
            Self::Armadillo(_) => matches!(name, "state" | "scute_time"),
            Self::Chicken { .. } => matches!(name, "IsChickenJockey" | "EggLayTime"),
            Self::Bat(_) => name == "BatFlags",
            Self::Bee(_) => bee::owns_save_field(name),
            Self::Ocelot { .. } => name == "Trusting",
            Self::Creeper(_) => matches!(name, "powered" | "Fuse" | "ExplosionRadius" | "ignited"),
            Self::Enderman(_) => name == "carriedBlockState",
            Self::Endermite(_) => name == "Lifetime",
            Self::Fish(fish) => {
                name == "FromBucket" || (fish.leaf.is_some() && fish::owns_save_field(name))
            }
            Self::Fox(..) => matches!(
                name,
                "Trusted" | "Sleeping" | "Type" | "Sitting" | "Crouching"
            ),
            Self::GameEventListener(state) => {
                name == game_event_listener::TAG_LISTENER
                    || (state.allay.is_some() && name == "DuplicationCooldown")
            }
            Self::Warden(_) => name == "anger" || name == game_event_listener::TAG_LISTENER,
            Self::Shulker(_) => matches!(name, "AttachFace" | "Peek" | "Color"),
            Self::Ghast(_) => name == "ExplosionPower",
            Self::Goat(_) => matches!(name, "IsScreamingGoat" | "HasLeftHorn" | "HasRightHorn"),
            Self::HappyGhast(_) => name == "still_timeout",
            Self::Horse(horse) => {
                matches!(
                    name,
                    "EatingHaystack" | "Bred" | "Temper" | "Tame" | "Owner"
                ) || horse
                    .phenotype
                    .is_some_and(|state| horse_phenotype::owns_save_field(state, name))
                    || (horse.trap.is_some() && skeleton_horse::owns_save_field(name))
                    || (horse.camel.is_some() && camel::owns_save_field(name))
                    || (horse.trader.is_some() && name == "DespawnDelay")
            }
            Self::Panda(_) => matches!(name, "MainGene" | "HiddenGene"),
            Self::Phantom(_) => phantom::owns_save_field(name),
            Self::Piglin { .. } => matches!(name, "IsBaby" | "CannotHunt"),
            Self::Zoglin { .. } => name == "IsBaby",
            Self::Rabbit(_) => rabbit::owns_save_field(name),
            Self::Raider(state) | Self::Witch(state, _) => {
                raider::owns_save_field(name) || state.owns_leaf_save_field(name)
            }
            Self::Sheep(_) => sheep::owns_save_field(name),
            Self::Skeleton(_) => skeleton::owns_save_field(name),
            Self::Slime(_) => slime::owns_save_field(name),
            Self::Tadpole(_) => name == "FromBucket" || tadpole::owns_save_field(name),
            Self::Tamable(_, collar, _, _, _) => {
                matches!(name, "Owner" | "Sitting")
                    || (collar.is_some() && collar::owns_save_field(name))
            }
            Self::Turtle(_) => turtle::owns_save_field(name),
            Self::Vex(_) => matches!(name, "bound_pos" | "life_ticks" | "owner"),
            Self::Villager(..) => {
                merchant::owns_save_field(name)
                    || VillagerState::owns_save_field(name)
                    || matches!(
                        name,
                        "AssignProfessionWhenSpawned"
                            | "FoodLevel"
                            | "LastRestock"
                            | "RestocksToday"
                            | "LastGossipDecay"
                    )
            }
            Self::WanderingTrader(..) => {
                merchant::owns_save_field(name) || matches!(name, "DespawnDelay" | "wander_target")
            }
            Self::Wither(_) => name == "Invul",
            Self::EnderDragon(_) => ender_dragon::owns_save_field(name),
            Self::Zombie(_) => zombie::owns_save_field(name),
            Self::ZombieVillager(..) => {
                zombie::owns_save_field(name)
                    || VillagerState::owns_save_field(name)
                    || ZombieVillagerCure::owns_save_field(name)
            }
        }
    }

    pub(super) fn synched_values(&self, entity_type_id: i32, rows: &mut Vec<SynchedValue>) {
        use crate::entity::*;
        match self {
            Self::Plain | Self::Chicken { .. } => {}
            Self::Plugin(plugin) => {
                for value in &plugin.synched_values {
                    let is_default = plugin
                        .default_synched_values
                        .iter()
                        .any(|default| default == value);
                    rows.push(SynchedValue {
                        value: value.clone(),
                        is_default,
                    });
                }
            }
            // AbstractPiglin.java:58-61.
            Self::PiglinBrute(zombification) => rows.push(bool_row(
                bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID,
                zombification.immune,
            )),
            Self::Armadillo(state) => rows.push(state.synched_value()),
            Self::Hoglin { zombification, .. } => rows.push(bool_row(
                bbb_protocol::entity_synched_data::VANILLA_HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID,
                zombification.immune,
            )),
            Self::AgeableWater(state) => match *state {
                AgeableWaterCreatureRetainedState::Dolphin {
                    got_fish,
                    moistness,
                    ..
                } => {
                    // `Dolphin.defineSynchedData` (`Dolphin.java:127-131`).
                    rows.push(bool_row(VANILLA_DOLPHIN_GOT_FISH_DATA_ID, got_fish));
                    rows.push(int_row(VANILLA_DOLPHIN_MOISTNESS_DATA_ID, moistness, 2400));
                }
                AgeableWaterCreatureRetainedState::GlowSquid {
                    dark_ticks_remaining,
                } => rows.push(int_row(
                    VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID,
                    dark_ticks_remaining,
                    0,
                )),
                AgeableWaterCreatureRetainedState::Squid => {}
            },
            Self::Axolotl {
                from_bucket,
                playing_dead,
            } => {
                rows.push(bool_row(VANILLA_AXOLOTL_FROM_BUCKET_DATA_ID, *from_bucket));
                rows.push(bool_row(
                    VANILLA_AXOLOTL_PLAYING_DEAD_DATA_ID,
                    *playing_dead,
                ));
            }
            Self::Bat(state) => rows.push(byte_row(VANILLA_BAT_FLAGS_DATA_ID, state.flags())),
            Self::Bee(state) => rows.extend(bee::synched_values(state)),
            Self::Blaze(state) => rows.extend(blaze::synched_values(*state)),
            Self::CopperGolem(state) => rows.push(state.animation.synched_value()),
            Self::Ocelot { trusting } => {
                rows.push(bool_row(VANILLA_OCELOT_TRUSTING_DATA_ID, *trusting));
            }
            Self::Creaking(state) => rows.extend(creaking::synched_values(state)),
            Self::Creeper(creeper) => {
                // `Creeper.defineSynchedData` (`Creeper.java:98-103`).
                rows.push(int_row(
                    VANILLA_CREEPER_SWELL_DIR_DATA_ID,
                    creeper.swell_dir(),
                    -1,
                ));
                rows.push(bool_row(VANILLA_CREEPER_POWERED_DATA_ID, creeper.powered()));
                rows.push(bool_row(VANILLA_CREEPER_IGNITED_DATA_ID, creeper.ignited()));
            }
            Self::Enderman(state) => {
                let carried = state.carried_block_state_id();
                rows.push(SynchedValue {
                    value: EntityDataValue {
                        data_id: VANILLA_ENDERMAN_CARRY_STATE_DATA_ID,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
                        value: EntityDataValueKind::OptionalBlockState(carried),
                    },
                    is_default: carried.is_none(),
                });
                rows.push(bool_row(VANILLA_ENDERMAN_CREEPY_DATA_ID, state.creepy()));
                rows.push(bool_row(
                    VANILLA_ENDERMAN_STARED_AT_DATA_ID,
                    state.stared_at(),
                ));
            }
            Self::Endermite(_) => {}
            Self::Fish(fish) => {
                rows.push(bool_row(
                    VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID,
                    fish.from_bucket,
                ));
                if let Some(leaf) = fish.leaf {
                    rows.extend(fish::synched_values(leaf));
                }
            }
            Self::Fox(state, _) => rows.extend(fox::synched_values(state)),
            Self::Warden(state) => rows.push(state.synched_value()),
            Self::GameEventListener(state) => {
                if let Some(allay) = state.allay {
                    // `Allay.defineSynchedData`'s `DATA_DANCING` (`Allay.java:142-146`).
                    rows.push(bool_row(
                        bbb_protocol::entity_synched_data::VANILLA_ALLAY_DANCING_DATA_ID,
                        allay.dancing,
                    ));
                    let can_duplicate = allay.can_duplicate();
                    rows.push(SynchedValue {
                        value: EntityDataValue {
                            data_id: bbb_protocol::entity_synched_data::VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID,
                            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                            value: EntityDataValueKind::Boolean(can_duplicate),
                        },
                        is_default: can_duplicate,
                    });
                }
            }
            Self::Ghast(state) => {
                rows.push(bool_row(VANILLA_GHAST_CHARGING_DATA_ID, state.charging()))
            }
            Self::Shulker(state) => rows.extend(state.synched_values()),
            Self::Goat(state) => rows.extend(goat::synched_values(state)),
            Self::Guardian(state) => {
                rows.push(int_row(
                    VANILLA_GUARDIAN_ATTACK_TARGET_DATA_ID,
                    state.attack_target,
                    0,
                ));
                rows.push(bool_row(VANILLA_GUARDIAN_MOVING_DATA_ID, state.moving));
            }
            Self::HappyGhast(state) => {
                rows.push(bool_row(
                    VANILLA_HAPPY_GHAST_IS_LEASH_HOLDER_DATA_ID,
                    state.is_leash_holder,
                ));
                rows.push(bool_row(
                    VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID,
                    state.stays_still,
                ));
            }
            // `IronGolem.defineSynchedData` (`IronGolem.java:52`, `:85-88`).
            Self::IronGolem { player_created } => rows.push(byte_row(
                16,
                if *player_created {
                    bbb_protocol::entity_synched_data::VANILLA_IRON_GOLEM_FLAG_PLAYER_CREATED
                } else {
                    0
                },
            )),
            Self::Horse(horse) => {
                rows.push(byte_row(
                    VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
                    horse.horse.flags(),
                ));
                if let Some(phenotype) = horse.phenotype {
                    rows.extend(horse_phenotype::synched_values(phenotype));
                }
                if let Some(camel) = horse.camel {
                    rows.extend(camel::synched_values(camel));
                }
            }
            Self::Panda(state) => rows.extend(panda::synched_values(state)),
            Self::Phantom(state) => rows.extend(phantom::synched_values(*state)),
            // `Pig.defineSynchedData` (`Pig.java:105-111`).
            Self::Pig(steering) => rows.push(int_row(
                bbb_protocol::entity_synched_data::VANILLA_PIG_BOOST_TIME_DATA_ID,
                steering.boost_time_total(),
                0,
            )),
            Self::Piglin {
                baby,
                celebration,
                zombification,
                charging_crossbow,
                ..
            } => {
                rows.push(bool_row(
                    bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID,
                    zombification.immune,
                ));
                rows.push(bool_row(
                    bbb_protocol::entity_synched_data::VANILLA_PIGLIN_BABY_DATA_ID,
                    *baby,
                ));
                rows.push(bool_row(
                    bbb_protocol::entity_synched_data::VANILLA_PIGLIN_CHARGING_CROSSBOW_DATA_ID,
                    *charging_crossbow,
                ));
                // Piglin.java:148-153.
                rows.push(bool_row(
                    bbb_protocol::entity_synched_data::VANILLA_PIGLIN_DANCING_DATA_ID,
                    celebration.is_some_and(|state| state.dancing),
                ));
            }
            Self::PolarBear { standing } => {
                rows.push(bool_row(VANILLA_POLAR_BEAR_STANDING_DATA_ID, *standing));
            }
            Self::Zoglin { baby } => {
                rows.push(bool_row(
                    bbb_protocol::entity_synched_data::VANILLA_ZOGLIN_BABY_DATA_ID,
                    *baby,
                ));
            }
            Self::Rabbit(state) => rows.extend(rabbit::synched_values(*state)),
            Self::Raider(state) => rows.extend(raider::synched_values(*state)),
            Self::Witch(state, witch) => {
                rows.extend(raider::synched_values(*state));
                rows.extend(witch::synched_values(*witch));
            }
            Self::Sheep(state) => rows.extend(sheep::synched_values(*state)),
            Self::Skeleton(state) => rows.extend(skeleton::synched_values(*state)),
            Self::Slime(state) => rows.extend(slime::synched_values(*state)),
            Self::Sniffer(state) => rows.extend(state.synched_values()),
            Self::Strider(strider) => {
                // `Strider.defineSynchedData` (`Strider.java:122-126`).
                rows.push(int_row(
                    VANILLA_STRIDER_BOOST_TIME_DATA_ID,
                    strider.boost_time(),
                    0,
                ));
                rows.push(bool_row(
                    VANILLA_STRIDER_SUFFOCATING_DATA_ID,
                    strider.suffocating(),
                ));
            }
            Self::Tadpole(state) => {
                rows.push(bool_row(VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID, false));
                rows.extend(tadpole::synched_values(*state));
            }
            Self::Tamable(tamable, collar, wolf, nautilus, cat) => {
                if let Some(cat) = cat {
                    rows.push(bool_row(
                        bbb_protocol::entity_synched_data::VANILLA_CAT_IS_LYING_DATA_ID,
                        cat.lying,
                    ));
                    rows.push(bool_row(
                        bbb_protocol::entity_synched_data::VANILLA_CAT_RELAX_STATE_ONE_DATA_ID,
                        cat.relax,
                    ));
                }
                if let Some(nautilus) = nautilus {
                    rows.push(bool_row(
                        bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
                        nautilus.dashing,
                    ));
                }
                if let Some(wolf) = wolf {
                    rows.push(bool_row(
                        bbb_protocol::entity_synched_data::VANILLA_WOLF_INTERESTED_DATA_ID,
                        wolf.interested,
                    ));
                }
                // `TamableAnimal.defineSynchedData` (`TamableAnimal.java:40-54`).
                rows.push(byte_row(
                    VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID,
                    tamable.flags(),
                ));
                let owner = tamable.owner();
                rows.push(SynchedValue {
                    value: EntityDataValue {
                        data_id: VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID,
                        serializer_id:
                            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
                        value: EntityDataValueKind::OptionalLivingEntityReference(owner),
                    },
                    is_default: owner.is_none(),
                });
                if let Some(collar) = collar {
                    rows.extend(
                        collar::synched_values(entity_type_id, *collar)
                            .into_iter()
                            .flatten(),
                    );
                }
            }
            Self::Turtle(state) => rows.extend(turtle::synched_values(state)),
            Self::Vex(vex) => rows.push(byte_row(VANILLA_VEX_FLAGS_DATA_ID, vex.flags())),
            Self::Villager(merchant, _) | Self::WanderingTrader(merchant, _) => {
                rows.push(int_row(
                    VANILLA_ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID,
                    merchant.unhappy_counter,
                    0,
                ));
                if let Self::Villager(_, villager) = self {
                    villager.synched_values(rows);
                }
            }
            Self::ZombieVillager(zombie, villager, cure) => {
                villager.synched_values(rows);
                rows.extend(zombie::synched_values(*zombie));
                rows.push(cure.synched_value());
            }
            Self::Wither(state) => rows.extend(wither::synched_values(*state)),
            Self::EnderDragon(state) => rows.push(ender_dragon::synched_value(state)),
            Self::Zombie(state) => rows.extend(zombie::synched_values(*state)),
        }
    }

    /// Keeps the transient state a command's reload must not reset.
    pub(super) fn restore_command_transients(&mut self, previous: &Self) {
        match (self, previous) {
            (
                Self::Axolotl {
                    playing_dead: current,
                    ..
                },
                Self::Axolotl {
                    playing_dead: previous,
                    ..
                },
            ) => *current = *previous,
            (
                Self::Piglin {
                    celebration: current,
                    charging_crossbow: current_charging,
                    ..
                },
                Self::Piglin {
                    celebration: previous,
                    charging_crossbow: previous_charging,
                    ..
                },
            ) => {
                *current = *previous;
                *current_charging = *previous_charging;
            }
            (Self::CopperGolem(current), Self::CopperGolem(previous)) => {
                current.animation = previous.animation;
            }
            (Self::Vex(current), Self::Vex(previous)) => current.flags = previous.flags,
            (Self::Warden(current), Self::Warden(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::GameEventListener(current), Self::GameEventListener(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Creeper(current), Self::Creeper(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Horse(current), Self::Horse(previous)) => {
                if let (Some(current), Some(previous)) = (&mut current.camel, &previous.camel) {
                    current.restore_command_transients(previous);
                }
                if let (Some(current), Some(previous)) = (&mut current.caravan, previous.caravan) {
                    *current = previous;
                }
                let persisted = VANILLA_ABSTRACT_HORSE_FLAG_EATING
                    | VANILLA_ABSTRACT_HORSE_FLAG_BRED
                    | VANILLA_ABSTRACT_HORSE_FLAG_TAME;
                let (current, previous) = (&mut current.horse, &previous.horse);
                current.flags = (current.flags & persisted) | (previous.flags & !persisted);
                current.eating_counter = previous.eating_counter;
                current.mouth_counter = previous.mouth_counter;
                current.stand_counter = previous.stand_counter;
                current.tail_counter = previous.tail_counter;
            }
            (Self::Strider(current), Self::Strider(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Pig(current), Self::Pig(previous)) => *current = *previous,
            (Self::Guardian(current), Self::Guardian(previous)) => *current = *previous,
            (Self::Sniffer(current), Self::Sniffer(previous)) => *current = *previous,
            (
                Self::AgeableWater(AgeableWaterCreatureRetainedState::Dolphin {
                    got_fish: true,
                    treasure_pos: current,
                    ..
                }),
                Self::AgeableWater(AgeableWaterCreatureRetainedState::Dolphin {
                    treasure_pos: previous,
                    ..
                }),
            ) => *current = *previous,
            (Self::Ghast(current), Self::Ghast(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::HappyGhast(current), Self::HappyGhast(previous)) => {
                current.time = previous.time;
                current.is_leash_holder = previous.is_leash_holder;
                current.requires_precise_position = previous.requires_precise_position;
            }
            (Self::Shulker(current), Self::Shulker(previous)) => {
                current.restore_command_transients(*previous);
            }
            (
                Self::Villager(current, current_villager),
                Self::Villager(previous, previous_villager),
            ) => {
                current.trading_player = previous.trading_player;
                current.unhappy_counter = previous.unhappy_counter;
                current_villager.work.update_merchant_timer =
                    previous_villager.work.update_merchant_timer;
                current_villager.work.increase_level_on_update =
                    previous_villager.work.increase_level_on_update;
                current_villager.work.last_traded_player =
                    previous_villager.work.last_traded_player;
                current_villager.work.last_gossip_time = previous_villager.work.last_gossip_time;
                current_villager.work.last_restock_check_day =
                    previous_villager.work.last_restock_check_day;
            }
            (
                Self::WanderingTrader(current, trader),
                Self::WanderingTrader(previous, previous_trader),
            ) => {
                current.trading_player = previous.trading_player;
                current.unhappy_counter = previous.unhappy_counter;
                trader.restore_command_transients(previous_trader);
            }
            (Self::Blaze(current), Self::Blaze(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Raider(current), Self::Raider(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Witch(current_raider, current), Self::Witch(previous_raider, previous)) => {
                current_raider.restore_command_transients(previous_raider);
                current.restore_command_transients(previous);
            }
            (Self::Wither(current), Self::Wither(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::EnderDragon(current), Self::EnderDragon(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Panda(current), Self::Panda(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Goat(current), Self::Goat(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Fox(current, goals), Self::Fox(previous, previous_goals)) => {
                current.restore_command_transients(previous);
                *goals = previous_goals.or(*goals);
            }
            (Self::Bee(current), Self::Bee(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Turtle(current), Self::Turtle(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Skeleton(current), Self::Skeleton(previous)) => {
                current.restore_command_transients(previous);
            }
            (Self::Fish(current), Self::Fish(previous)) => {
                current.school = previous.school;
                current.tropical_school = previous.tropical_school;
            }
            (
                Self::Hoglin {
                    experience_reward,
                    state,
                    ..
                },
                Self::Hoglin {
                    experience_reward: previous,
                    state: previous_state,
                    ..
                },
            ) => {
                *experience_reward = *previous;
                *state = *previous_state;
            }
            _ => {}
        }
    }
}

pub(super) fn bool_row(data_id: u8, value: bool) -> SynchedValue {
    SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(value),
        },
        is_default: !value,
    }
}

fn byte_row(data_id: u8, value: i8) -> SynchedValue {
    SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(value),
        },
        is_default: value == 0,
    }
}

fn int_row(data_id: u8, value: i32, default: i32) -> SynchedValue {
    SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(value),
        },
        is_default: value == default,
    }
}

/// Accessors for the variants that hold one state: `copy` getters answer by
/// value, `ref` getters by reference, and a listed `mut` name gets the
/// mutable form.
macro_rules! variant_accessors {
    ($($(#[$attr:meta])* $mode:ident $variant:ident($ty:ty) => $get:ident $(, $get_mut:ident)?;)*) => {
        impl GeneralLivingData {
            $(
                $(#[$attr])*
                variant_accessors!(@get $mode $variant $ty, $get);
                $(pub(crate) fn $get_mut(&mut self) -> Option<&mut $ty> {
                    match &mut self.kind {
                        MobVariantState::$variant(state) => Some(state),
                        _ => None,
                    }
                })?
            )*
        }
    };
    (@get copy $variant:ident $ty:ty, $get:ident) => {
        pub(crate) fn $get(&self) -> Option<$ty> {
            match &self.kind {
                MobVariantState::$variant(state) => Some(*state),
                _ => None,
            }
        }
    };
    (@get ref $variant:ident $ty:ty, $get:ident) => {
        pub(crate) fn $get(&self) -> Option<&$ty> {
            match &self.kind {
                MobVariantState::$variant(state) => Some(state),
                _ => None,
            }
        }
    };
}

variant_accessors! {
    ref Armadillo(ArmadilloState) => armadillo_state, armadillo_state_mut;
    copy AgeableWater(AgeableWaterCreatureRetainedState) => ageable_water_retained, ageable_water_retained_mut;
    copy Bat(BatRetainedState) => bat_retained, bat_retained_mut;
    ref Bee(BeeRetainedState) => bee_retained, bee_retained_mut;
    copy Blaze(BlazeRetainedState) => blaze_retained, blaze_retained_mut;
    copy CopperGolem(crate::entity::copper_golem::CopperGolemState) => copper_golem_state, copper_golem_state_mut;
    ref Creaking(CreakingRetainedState) => creaking_retained, creaking_retained_mut;
    copy Creeper(CreeperRetainedState) => creeper_retained, creeper_retained_mut;
    copy Enderman(EndermanRetainedState) => enderman_retained, enderman_retained_mut;
    #[cfg(test)]
    copy Endermite(EndermiteRetainedState) => endermite_retained, endermite_retained_mut;
    copy Ghast(GhastRetainedState) => ghast_retained, ghast_retained_mut;
    copy Shulker(crate::entity::shulker::ShulkerState) => shulker_state, shulker_state_mut;
    ref Goat(GoatRetainedState) => goat_retained, goat_retained_mut;
    copy Guardian(crate::entity::guardian::GuardianState) => guardian_state, guardian_state_mut;
    copy HappyGhast(HappyGhastState) => happy_ghast_state, happy_ghast_state_mut;
    ref Panda(PandaRetainedState) => panda_retained, panda_retained_mut;
    copy Phantom(PhantomRetainedState) => phantom_retained, phantom_retained_mut;
    #[cfg(test)]
    copy Pig(crate::entity::item_steering::ItemSteering) => pig_steering, pig_steering_mut;
    copy Rabbit(RabbitRetainedState) => rabbit_retained, rabbit_retained_mut;
    copy Sheep(SheepRetainedState) => sheep_retained, sheep_retained_mut;
    #[cfg(test)]
    copy Skeleton(SkeletonRetainedState) => skeleton_retained, skeleton_retained_mut;
    copy Slime(SlimeRetainedState) => slime_retained, slime_retained_mut;
    copy Sniffer(crate::entity::sniffer::SnifferState) => sniffer_state, sniffer_state_mut;
    copy Strider(StriderState) => strider_state, strider_state_mut;
    copy Tadpole(TadpoleRetainedState) => tadpole_retained, tadpole_retained_mut;
    ref Turtle(TurtleRetainedState) => turtle_retained, turtle_retained_mut;
    ref Vex(VexRetainedState) => vex_retained, vex_retained_mut;
    copy Wither(WitherRetainedState) => wither_retained, wither_retained_mut;
}

impl GeneralLivingData {
    pub(crate) fn copper_golem_animation(
        &self,
    ) -> Option<crate::entity::copper_golem::AnimationState> {
        match &self.kind {
            MobVariantState::CopperGolem(state) => Some(state.animation),
            _ => None,
        }
    }

    pub(crate) fn piglin_cannot_hunt(&self) -> Option<bool> {
        match &self.kind {
            MobVariantState::Piglin { cannot_hunt, .. } => Some(*cannot_hunt),
            _ => None,
        }
    }

    pub(crate) fn hoglin_cannot_be_hunted(&self) -> Option<bool> {
        match &self.kind {
            MobVariantState::Hoglin {
                cannot_be_hunted, ..
            } => Some(*cannot_be_hunted),
            _ => None,
        }
    }

    pub(crate) fn nautilus_dash_mut(
        &mut self,
    ) -> Option<&mut crate::entity::nautilus::NautilusDash> {
        match &mut self.kind {
            MobVariantState::Tamable(_, _, _, state, _) => state.as_mut(),
            _ => None,
        }
    }

    pub(crate) fn ender_dragon(&self) -> Option<&crate::entity::ender_dragon::EnderDragonState> {
        match &self.kind {
            MobVariantState::EnderDragon(state) => Some(state),
            _ => None,
        }
    }

    pub(crate) fn ender_dragon_mut(
        &mut self,
    ) -> Option<&mut crate::entity::ender_dragon::EnderDragonState> {
        match &mut self.kind {
            MobVariantState::EnderDragon(state) => Some(state),
            _ => None,
        }
    }

    pub(crate) fn ocelot_trusting(&self) -> Option<bool> {
        match &self.kind {
            MobVariantState::Ocelot { trusting } => Some(*trusting),
            _ => None,
        }
    }

    pub(crate) fn iron_golem_player_created(&self) -> Option<bool> {
        match &self.kind {
            MobVariantState::IronGolem { player_created } => Some(*player_created),
            _ => None,
        }
    }

    pub(crate) fn set_iron_golem_player_created(&mut self, value: bool) -> bool {
        match &mut self.kind {
            MobVariantState::IronGolem { player_created } => {
                std::mem::replace(player_created, value) != value
            }
            _ => false,
        }
    }

    /// `Chicken.isChickenJockey`, `None` for every other type.
    pub(crate) fn chicken_jockey(&self) -> Option<bool> {
        match &self.kind {
            MobVariantState::Chicken { jockey, .. } => Some(*jockey),
            _ => None,
        }
    }

    pub(crate) fn set_chicken_jockey(&mut self, value: bool) -> bool {
        match &mut self.kind {
            MobVariantState::Chicken { jockey, .. } => {
                *jockey = value;
                true
            }
            _ => false,
        }
    }

    /// The countdown of `Chicken.eggTime` or `Armadillo.scuteTime`.
    pub(crate) fn gift_drop_time_mut(&mut self) -> Option<&mut i32> {
        match &mut self.kind {
            MobVariantState::Chicken { egg_time, .. } => Some(egg_time),
            MobVariantState::Armadillo(state) => Some(&mut state.scute_time),
            _ => None,
        }
    }

    pub(crate) fn axolotl_playing_dead(&self) -> bool {
        match &self.kind {
            MobVariantState::Axolotl { playing_dead, .. } => *playing_dead,
            _ => false,
        }
    }

    pub(crate) fn set_axolotl_playing_dead(&mut self, value: bool) -> bool {
        match &mut self.kind {
            MobVariantState::Axolotl { playing_dead, .. } if *playing_dead != value => {
                *playing_dead = value;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn is_from_bucket(&self) -> Option<bool> {
        match &self.kind {
            MobVariantState::Axolotl { from_bucket, .. } => Some(*from_bucket),
            MobVariantState::Fish(fish) => Some(fish.from_bucket),
            MobVariantState::Tadpole(_) => Some(true),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn set_from_bucket(&mut self, value: bool) -> bool {
        match &mut self.kind {
            MobVariantState::Axolotl { from_bucket, .. } => *from_bucket = value,
            MobVariantState::Fish(fish) => fish.from_bucket = value,
            MobVariantState::Tadpole(_) => {}
            _ => return false,
        }
        true
    }

    pub(crate) fn fish_state(&self) -> Option<&FishState> {
        match &self.kind {
            MobVariantState::Fish(fish) => Some(fish),
            _ => None,
        }
    }

    pub(crate) fn fish_state_mut(&mut self) -> Option<&mut FishState> {
        match &mut self.kind {
            MobVariantState::Fish(fish) => Some(fish),
            _ => None,
        }
    }

    pub(crate) fn fish_leaf(&self) -> Option<FishLeafState> {
        match &self.kind {
            MobVariantState::Fish(fish) => fish.leaf,
            _ => None,
        }
    }

    /// `Wolf.getMaxHeadXRot` (`Wolf.java:388-390`), `Mob.java:725-726`.
    pub(crate) fn tamable_max_head_x_rot(&self) -> i32 {
        match &self.kind {
            MobVariantState::Tamable(tamable, _, Some(_), _, _) if tamable.is_in_sitting_pose() => {
                20
            }
            _ => 40,
        }
    }

    pub(crate) fn cat_lying(&self) -> bool {
        matches!(&self.kind, MobVariantState::Tamable(_, _, _, _, Some(cat)) if cat.lying)
    }

    pub(crate) fn set_cat_lying(&mut self, value: bool) -> bool {
        match &mut self.kind {
            MobVariantState::Tamable(_, _, _, _, Some(cat)) => {
                std::mem::replace(&mut cat.lying, value) != value
            }
            _ => false,
        }
    }

    pub(crate) fn cat_relaxing(&self) -> bool {
        matches!(&self.kind, MobVariantState::Tamable(_, _, _, _, Some(cat)) if cat.relax)
    }

    pub(crate) fn set_cat_relaxing(&mut self, value: bool) -> bool {
        match &mut self.kind {
            MobVariantState::Tamable(_, _, _, _, Some(cat)) => {
                std::mem::replace(&mut cat.relax, value) != value
            }
            _ => false,
        }
    }

    pub(crate) fn wolf_shake_mut(&mut self) -> Option<&mut crate::entity::wolf::WolfShake> {
        match &mut self.kind {
            MobVariantState::Tamable(_, _, Some(wolf), _, _) => Some(&mut wolf.shake),
            _ => None,
        }
    }

    pub(super) fn set_wolf_interested(&mut self, value: bool) -> bool {
        match &mut self.kind {
            MobVariantState::Tamable(_, _, Some(wolf), _, _) => {
                std::mem::replace(&mut wolf.interested, value) != value
            }
            _ => false,
        }
    }

    pub(crate) fn tamable_retained(&self) -> Option<&TamableRetainedState> {
        match &self.kind {
            MobVariantState::Tamable(tamable, _, _, _, _) => Some(tamable),
            _ => None,
        }
    }

    pub(crate) fn tamable_retained_mut(&mut self) -> Option<&mut TamableRetainedState> {
        match &mut self.kind {
            MobVariantState::Tamable(tamable, _, _, _, _) => Some(tamable),
            _ => None,
        }
    }

    pub(crate) fn is_tame(&self) -> bool {
        self.tamable_retained()
            .is_some_and(TamableRetainedState::is_tame)
    }

    pub(crate) fn collar_retained(&self) -> Option<CollarRetainedState> {
        match &self.kind {
            MobVariantState::Tamable(_, collar, _, _, _) => *collar,
            _ => None,
        }
    }

    pub(crate) fn collar_retained_mut(&mut self) -> Option<&mut CollarRetainedState> {
        match &mut self.kind {
            MobVariantState::Tamable(_, collar, _, _, _) => collar.as_mut(),
            _ => None,
        }
    }

    pub(crate) fn horse_state_mut(&mut self) -> Option<&mut HorseState> {
        match &mut self.kind {
            MobVariantState::Horse(horse) => Some(horse),
            _ => None,
        }
    }

    pub(crate) fn abstract_horse_retained(&self) -> Option<&AbstractHorseRetainedState> {
        match &self.kind {
            MobVariantState::Horse(horse) => Some(&horse.horse),
            _ => None,
        }
    }

    pub(crate) fn abstract_horse_retained_mut(
        &mut self,
    ) -> Option<&mut AbstractHorseRetainedState> {
        self.horse_state_mut().map(|horse| &mut horse.horse)
    }

    pub(crate) fn horse_phenotype(&self) -> Option<HorsePhenotypeState> {
        match &self.kind {
            MobVariantState::Horse(horse) => horse.phenotype,
            _ => None,
        }
    }

    pub(crate) fn camel_state(&self) -> Option<CamelState> {
        match &self.kind {
            MobVariantState::Horse(horse) => horse.camel,
            _ => None,
        }
    }

    pub(crate) fn llama_caravan(&self) -> Option<CaravanState> {
        match &self.kind {
            MobVariantState::Horse(horse) => horse.caravan,
            _ => None,
        }
    }

    pub(crate) fn skeleton_horse_trap(&self) -> Option<SkeletonHorseTrapState> {
        match &self.kind {
            MobVariantState::Horse(horse) => horse.trap,
            _ => None,
        }
    }

    pub(crate) fn trader_state(&self) -> Option<&TraderState> {
        match &self.kind {
            MobVariantState::Horse(HorseState {
                trader: Some(trader),
                ..
            })
            | MobVariantState::WanderingTrader(_, trader) => Some(trader),
            _ => None,
        }
    }

    pub(crate) fn trader_state_mut(&mut self) -> Option<&mut TraderState> {
        match &mut self.kind {
            MobVariantState::Horse(HorseState {
                trader: Some(trader),
                ..
            })
            | MobVariantState::WanderingTrader(_, trader) => Some(trader),
            _ => None,
        }
    }

    pub(crate) fn abstract_villager_retained(&self) -> Option<&AbstractVillagerRetainedState> {
        match &self.kind {
            MobVariantState::Villager(merchant, _)
            | MobVariantState::WanderingTrader(merchant, _) => Some(merchant),
            _ => None,
        }
    }

    pub(crate) fn abstract_villager_retained_mut(
        &mut self,
    ) -> Option<&mut AbstractVillagerRetainedState> {
        match &mut self.kind {
            MobVariantState::Villager(merchant, _)
            | MobVariantState::WanderingTrader(merchant, _) => Some(merchant),
            _ => None,
        }
    }

    pub(super) fn villager_state(&self) -> Option<&VillagerState> {
        match &self.kind {
            MobVariantState::Villager(_, villager)
            | MobVariantState::ZombieVillager(_, villager, _) => Some(villager),
            _ => None,
        }
    }

    pub(super) fn villager_state_mut(&mut self) -> Option<&mut VillagerState> {
        match &mut self.kind {
            MobVariantState::Villager(_, villager)
            | MobVariantState::ZombieVillager(_, villager, _) => Some(villager),
            _ => None,
        }
    }

    pub(crate) fn raider_retained(&self) -> Option<&RaiderRetainedState> {
        match &self.kind {
            MobVariantState::Raider(raider) | MobVariantState::Witch(raider, _) => Some(raider),
            _ => None,
        }
    }

    pub(crate) fn raider_retained_mut(&mut self) -> Option<&mut RaiderRetainedState> {
        match &mut self.kind {
            MobVariantState::Raider(raider) | MobVariantState::Witch(raider, _) => Some(raider),
            _ => None,
        }
    }

    pub(crate) fn witch_state(&self) -> Option<WitchState> {
        match &self.kind {
            MobVariantState::Witch(_, witch) => Some(*witch),
            _ => None,
        }
    }

    pub(crate) fn witch_state_mut(&mut self) -> Option<&mut WitchState> {
        match &mut self.kind {
            MobVariantState::Witch(_, witch) => Some(witch),
            _ => None,
        }
    }

    pub(crate) fn zombie_retained(&self) -> Option<ZombieRetainedState> {
        match &self.kind {
            MobVariantState::Zombie(zombie) | MobVariantState::ZombieVillager(zombie, ..) => {
                Some(*zombie)
            }
            _ => None,
        }
    }

    pub(crate) fn zombie_villager_cure(&self) -> Option<ZombieVillagerCure> {
        match &self.kind {
            MobVariantState::ZombieVillager(_, _, cure) => Some(*cure),
            _ => None,
        }
    }

    pub(crate) fn zombie_villager_cure_mut(&mut self) -> Option<&mut ZombieVillagerCure> {
        match &mut self.kind {
            MobVariantState::ZombieVillager(_, _, cure) => Some(cure),
            _ => None,
        }
    }

    pub(crate) fn zombie_retained_mut(&mut self) -> Option<&mut ZombieRetainedState> {
        match &mut self.kind {
            MobVariantState::Zombie(zombie) | MobVariantState::ZombieVillager(zombie, ..) => {
                Some(zombie)
            }
            _ => None,
        }
    }

    pub(crate) fn fox_retained(&self) -> Option<&FoxRetainedState> {
        match &self.kind {
            MobVariantState::Fox(fox, _) => Some(fox),
            _ => None,
        }
    }

    pub(crate) fn fox_retained_mut(&mut self) -> Option<&mut FoxRetainedState> {
        match &mut self.kind {
            MobVariantState::Fox(fox, _) => Some(fox),
            _ => None,
        }
    }

    pub(crate) fn warden_state(&self) -> Option<&crate::entity::warden::anger::WardenState> {
        match &self.kind {
            MobVariantState::Warden(state) => Some(state),
            _ => None,
        }
    }

    pub(crate) fn warden_state_mut(
        &mut self,
    ) -> Option<&mut crate::entity::warden::anger::WardenState> {
        match &mut self.kind {
            MobVariantState::Warden(state) => Some(state),
            _ => None,
        }
    }

    pub(crate) fn game_event_listener_state(&self) -> Option<&EntityGameEventListenerState> {
        match &self.kind {
            MobVariantState::GameEventListener(state) => Some(state),
            MobVariantState::Warden(state) => Some(&state.listener),
            _ => None,
        }
    }

    pub(crate) fn game_event_listener_state_mut(
        &mut self,
    ) -> Option<&mut EntityGameEventListenerState> {
        match &mut self.kind {
            MobVariantState::GameEventListener(state) => Some(state),
            MobVariantState::Warden(state) => Some(&mut state.listener),
            _ => None,
        }
    }

    /// `LivingEntity.isBaby()` for the `DATA_BABY_ID` families that are not
    /// `AgeableMob` (`Piglin.java:254-256`, `Zoglin.java:243-245`).
    pub(crate) fn piglin_or_zoglin_baby(&self) -> Option<bool> {
        match &self.kind {
            MobVariantState::Piglin { baby, .. } | MobVariantState::Zoglin { baby } => Some(*baby),
            _ => None,
        }
    }

    pub(crate) fn piglin_celebration(&self) -> Option<CelebrationState> {
        match &self.kind {
            MobVariantState::Piglin { celebration, .. } => *celebration,
            _ => None,
        }
    }

    pub(crate) fn piglin_celebration_mut(&mut self) -> Option<&mut Option<CelebrationState>> {
        match &mut self.kind {
            MobVariantState::Piglin { celebration, .. } => Some(celebration),
            _ => None,
        }
    }

    pub(crate) fn piglin_charging_crossbow_mut(&mut self) -> Option<&mut bool> {
        match &mut self.kind {
            MobVariantState::Piglin {
                charging_crossbow, ..
            } => Some(charging_crossbow),
            _ => None,
        }
    }

    /// `Hoglin.xpReward` (`Hoglin.java:72`, `:162-170`), the one type whose
    /// reward this crate retains.
    pub(crate) fn spawn_experience_reward(&self) -> Option<i32> {
        match &self.kind {
            MobVariantState::Hoglin {
                experience_reward, ..
            } => Some(*experience_reward),
            _ => None,
        }
    }
}

impl GeneralLivingData {
    pub(crate) fn zombification(
        &self,
    ) -> Option<&crate::entity::zombification::ZombificationState> {
        self.kind.zombification()
    }

    pub(crate) fn zombification_mut(
        &mut self,
    ) -> Option<&mut crate::entity::zombification::ZombificationState> {
        self.kind.zombification_mut()
    }
}

impl GeneralLivingData {
    pub(crate) fn hoglin_state(&self) -> Option<&crate::entity::hoglin::HoglinState> {
        match &self.kind {
            MobVariantState::Hoglin { state, .. } => Some(state),
            _ => None,
        }
    }

    pub(crate) fn hoglin_state_mut(&mut self) -> Option<&mut crate::entity::hoglin::HoglinState> {
        match &mut self.kind {
            MobVariantState::Hoglin { state, .. } => Some(state),
            _ => None,
        }
    }
}
