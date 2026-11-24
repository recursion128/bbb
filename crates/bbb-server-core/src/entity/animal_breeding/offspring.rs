//!
//! [`OFFSPRING_FACTORIES`] is the whole set of subtypes this runtime can build
//! a child for, and `ai/mob/breed.rs` installs a `BreedGoal` only for those: a
//! type outside the table has no child to make, so its goal is skipped rather
//! than left to seek a partner it can never conceive with.
//!
//!
//!

use crate::random::GameRandom;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_BEE_ID,
    VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID,
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_FOX_ID,
    VANILLA_ENTITY_TYPE_GOAT_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
    VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_MOOSHROOM_ID, VANILLA_ENTITY_TYPE_MULE_ID,
    VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_OCELOT_ID, VANILLA_ENTITY_TYPE_PANDA_ID,
    VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_RABBIT_ID,
    VANILLA_ENTITY_TYPE_SHEEP_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};
use uuid::Uuid;

use crate::entity::general_living::{
    apply_parent_variant_copy, parent_variant_copy, ParentVariantCopy,
};
use crate::entity::store::EntityStore;
use crate::entity::Entity;

/// `AgeableMob.BABY_START_AGE` (`AgeableMob.java:31`), the age
/// `offspring.setBaby(true)` writes (`AgeableMob.java:246-248`).
pub(crate) const BABY_START_AGE: i32 = -24000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OffspringPlan {
    /// `EntityType.X.create(level, EntitySpawnReason.BREEDING)` with no state
    /// copied off either parent.
    Plain {
        entity_type_id: i32,
    },
    /// The same create with one parent's variant, the one
    /// `this.random.nextBoolean()` picked.
    Variant {
        entity_type_id: i32,
        from_partner: bool,
    },
    Persistent {
        entity_type_id: i32,
    },
    /// The same create with one value the factory already resolved, rather
    /// than a copy of whichever parent a draw picked.
    Resolved {
        entity_type_id: i32,
        value: ResolvedChildState,
    },
    Nautilus {
        owner: Option<Uuid>,
        tame: bool,
    },
    Tamable {
        entity_type_id: i32,
        from_partner: bool,
        tame: Option<TamedChildState>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TamedChildState {
    owner: Option<Uuid>,
    collar_color: i8,
}

/// The one piece of state a [`OffspringPlan::Resolved`] child is born with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedChildState {
    PandaGenes(crate::entity::panda::PandaGenes),
    SheepColor(i8),
    MooshroomVariant(i32),
    AxolotlVariant(i32),
    GoatScreaming(bool),
}

/// `Axolotl.Variant.BLUE`, the only rare variant (`Axolotl.java:627`).
const AXOLOTL_BLUE: i32 = 4;

impl OffspringPlan {
    pub(crate) const fn entity_type_id(self) -> i32 {
        match self {
            Self::Nautilus { .. } => VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            Self::Plain { entity_type_id }
            | Self::Variant { entity_type_id, .. }
            | Self::Persistent { entity_type_id }
            | Self::Resolved { entity_type_id, .. }
            | Self::Tamable { entity_type_id, .. } => entity_type_id,
        }
    }

    /// `Hoglin`, `TraderLlama` and `Axolotl.getBreedOffspring`'s
    /// `setPersistenceRequired` (`Axolotl.java:348`).
    const fn persistent(self) -> bool {
        matches!(self, Self::Persistent { .. })
            || self.entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID
    }
}

///
/// `mix_dyes` is `DyeColor.getMixedColor` (`DyeColor.java:120-127`).
pub(crate) fn plan_offspring(
    body: &Entity,
    store: &EntityStore,
    mix_dyes: &mut dyn FnMut(i32, i32) -> i32,
    partner_id: i32,
    random: &GameRandom,
) -> Option<OffspringPlan> {
    let entity_type_id = body.entity_type_id();
    Some(match entity_type_id {
        // `Axolotl.useRareVariant` (`Axolotl.java:304-306`, `:341-345`).
        VANILLA_ENTITY_TYPE_AXOLOTL_ID if random.next_int(1200) == 0 => OffspringPlan::Resolved {
            entity_type_id,
            value: ResolvedChildState::AxolotlVariant(AXOLOTL_BLUE),
        },
        // `Goat.getBreedOffspring` (`Goat.java:145-152`).
        VANILLA_ENTITY_TYPE_GOAT_ID => {
            let parent = if random.next_boolean() {
                body
            } else {
                store.get(partner_id)?
            };
            let screaming = matches!(parent.data(), crate::entity::EntityData::GeneralLiving(data)
                if data.goat_retained().is_some_and(|goat| goat.is_screaming()))
                || random.next_double() < 0.02;
            OffspringPlan::Resolved {
                entity_type_id,
                value: ResolvedChildState::GoatScreaming(screaming),
            }
        }
        VANILLA_ENTITY_TYPE_COW_ID
        | VANILLA_ENTITY_TYPE_PIG_ID
        | VANILLA_ENTITY_TYPE_CHICKEN_ID
        | VANILLA_ENTITY_TYPE_AXOLOTL_ID => {
            // `this.random.nextBoolean() ? this.getVariant() : partner.getVariant()`.
            OffspringPlan::Variant {
                entity_type_id,
                from_partner: random.next_boolean(),
            }
        }
        VANILLA_ENTITY_TYPE_PANDA_ID => OffspringPlan::Resolved {
            entity_type_id,
            value: ResolvedChildState::PandaGenes(crate::entity::panda::offspring_genes(
                body,
                store.get(partner_id)?,
                random,
            )?),
        },
        VANILLA_ENTITY_TYPE_SHEEP_ID => OffspringPlan::Resolved {
            entity_type_id,
            value: ResolvedChildState::SheepColor(sheep_color(body, store, mix_dyes, partner_id)?),
        },
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID => OffspringPlan::Resolved {
            entity_type_id,
            value: ResolvedChildState::MooshroomVariant(mooshroom_variant(
                body, store, partner_id, random,
            )?),
        },
        // Nautilus.java:41-49: only the creating parent supplies tame and owner.
        VANILLA_ENTITY_TYPE_NAUTILUS_ID => {
            let crate::entity::EntityData::GeneralLiving(data) = body.data() else {
                return None;
            };
            let tamable = data.tamable_retained()?;
            OffspringPlan::Nautilus {
                owner: tamable.owner(),
                tame: tamable.is_tame(),
            }
        }
        VANILLA_ENTITY_TYPE_CAT_ID | VANILLA_ENTITY_TYPE_WOLF_ID => {
            let from_partner = random.next_boolean();
            let tame = tamed_child_state(body, store, mix_dyes, partner_id)?;
            OffspringPlan::Tamable {
                entity_type_id,
                from_partner,
                tame,
            }
        }
        VANILLA_ENTITY_TYPE_BEE_ID
        | VANILLA_ENTITY_TYPE_CAMEL_ID
        | VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
        | VANILLA_ENTITY_TYPE_OCELOT_ID
        | VANILLA_ENTITY_TYPE_RABBIT_ID
        | VANILLA_ENTITY_TYPE_FOX_ID
        | VANILLA_ENTITY_TYPE_SNIFFER_ID
        | VANILLA_ENTITY_TYPE_STRIDER_ID
        | VANILLA_ENTITY_TYPE_ARMADILLO_ID => OffspringPlan::Plain { entity_type_id },
        VANILLA_ENTITY_TYPE_HOGLIN_ID => OffspringPlan::Persistent { entity_type_id },
        // `Horse.java:186-194`, `Donkey.java:59-60`: a horse and a donkey make a mule.
        VANILLA_ENTITY_TYPE_HORSE_ID | VANILLA_ENTITY_TYPE_DONKEY_ID => OffspringPlan::Plain {
            entity_type_id: if store.get(partner_id)?.entity_type_id() == entity_type_id {
                entity_type_id
            } else {
                VANILLA_ENTITY_TYPE_MULE_ID
            },
        },
        VANILLA_ENTITY_TYPE_LLAMA_ID => OffspringPlan::Plain { entity_type_id },
        // `TraderLlama.makeNewLlama` (`TraderLlama.java:40-46`).
        VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => OffspringPlan::Persistent { entity_type_id },
        _ => return None,
    })
}

fn sheep_color(
    body: &Entity,
    store: &EntityStore,
    mix_dyes: &mut dyn FnMut(i32, i32) -> i32,
    partner_id: i32,
) -> Option<i8> {
    let first = crate::entity::sheep::color_of(body)?;
    let second = crate::entity::sheep::color_of(store.get(partner_id)?)?;
    i8::try_from(mix_dyes(i32::from(first), i32::from(second))).ok()
}

///
/// `None` is a read the factory needs and cannot make, which refuses the whole
/// mating the way a missing wool colour does.
fn tamed_child_state(
    body: &Entity,
    store: &EntityStore,
    mix_dyes: &mut dyn FnMut(i32, i32) -> i32,
    partner_id: i32,
) -> Option<Option<TamedChildState>> {
    let crate::entity::EntityData::GeneralLiving(data) = body.data() else {
        return None;
    };
    let tamable = data.tamable_retained()?;
    if !tamable.is_tame() {
        return Some(None);
    }
    let first = crate::entity::collar::color_of(body)?;
    let second = crate::entity::collar::color_of(store.get(partner_id)?)?;
    let collar_color = i8::try_from(mix_dyes(i32::from(first), i32::from(second))).ok()?;
    Some(Some(TamedChildState {
        owner: tamable.owner(),
        collar_color,
    }))
}

///
/// The draw order is the one Java's `&&` gives: two parents of the same variant
/// spend a `nextInt(1024)` first and, unless it lands on the 1-in-1024 flip,
/// a `nextBoolean` after it; two parents of different variants short-circuit
/// past the `nextInt` and spend only the `nextBoolean`.
fn mooshroom_variant(
    body: &Entity,
    store: &EntityStore,
    partner_id: i32,
    random: &GameRandom,
) -> Option<i32> {
    let variant = crate::entity::mooshroom::variant_of(body)?;
    let mate_variant = crate::entity::mooshroom::variant_of(store.get(partner_id)?)?;
    if variant == mate_variant && random.next_int(1024) == 0 {
        return Some(crate::entity::mooshroom::other_variant(variant));
    }
    Some(if random.next_boolean() {
        variant
    } else {
        mate_variant
    })
}

/// Spawns the child and answers its id; `None` is a child the store could not
/// make or write, before either parent was touched. `biome_has_tag` answers
/// for the biome at the parent.
pub(crate) fn spawn_offspring(
    plan: OffspringPlan,
    variant: Option<&ParentVariant>,
    body: &Entity,
    partner_id: i32,
    store: &mut EntityStore,
    biome_has_tag: &dyn Fn(&str) -> bool,
    random: &GameRandom,
) -> Option<i32> {
    let partner = store.get(partner_id)?;
    let inherited = inherited_state(body, partner, random, || {
        crate::entity::general_living::random_rabbit_variant(random.next_int(100), biome_has_tag)
    });
    // `Wolf.java:580-600`: offspring chooses a sound variant independently.
    let sound_variant = if plan.entity_type_id() == VANILLA_ENTITY_TYPE_WOLF_ID {
        Some(
            crate::entity::general_living::pick_random_wolf_sound_variant(
                random,
                store.item_resolver(),
            )?,
        )
    } else {
        None
    };
    let child_id = store.spawn_general_living(plan.entity_type_id(), body.position())?;
    let child = store.get_mut(child_id)?;
    if let Some(variant) = variant {
        variant.apply(child);
    }
    if let Some(inherited) = inherited {
        inherited.apply(child);
    }
    if let OffspringPlan::Resolved { value, .. } = plan {
        match value {
            ResolvedChildState::PandaGenes(genes) => {
                if !crate::entity::panda::apply_spawn_genes(child, genes) {
                    return None;
                }
            }
            ResolvedChildState::SheepColor(color) => {
                if !crate::entity::sheep::set_spawn_color(child, color) {
                    return None;
                }
            }
            ResolvedChildState::MooshroomVariant(variant) => {
                if !crate::entity::mooshroom::set_variant(child, variant) {
                    return None;
                }
            }
            ResolvedChildState::AxolotlVariant(variant) => {
                if !crate::entity::set_spawner_simple_variant(child, variant) {
                    return None;
                }
            }
            ResolvedChildState::GoatScreaming(screaming) => {
                if !crate::entity::goat::apply_spawn_traits(
                    child,
                    screaming,
                    crate::entity::goat::SpawnHornRemoval::None,
                ) {
                    return None;
                }
            }
        }
    }
    if let OffspringPlan::Nautilus { owner, tame: true } = plan {
        if !crate::entity::taming::set_tame_offspring(child, owner) {
            return None;
        }
    }
    if let OffspringPlan::Tamable { tame, .. } = plan {
        if let Some(tame) = tame {
            if !crate::entity::taming::set_tame_offspring(child, tame.owner) {
                return None;
            }
            if !crate::entity::collar::set_collar_color(child, tame.collar_color) {
                return None;
            }
        }
        if let Some((resource_id, holder_id)) = sound_variant {
            if !crate::entity::general_living::set_spawner_registry_value(
                child,
                "minecraft:wolf_sound_variant",
                &resource_id,
                holder_id,
            ) {
                return None;
            }
        }
    }
    if !crate::entity::set_spawner_age(child, BABY_START_AGE) {
        return None;
    }
    if plan.persistent() {
        if let crate::entity::EntityData::GeneralLiving(data) = &mut child.data {
            if let Some(mob) = data.mob.as_mut() {
                mob.set_persistence_required(true);
            }
        }
    }
    child.set_rot(0.0, 0.0);
    Some(child_id)
}

/// The per-type state a child copies off one parent: a rabbit's or fox's
/// variant (`Rabbit.java:340-356`, `Fox.java:318`), or an equine's coat and
/// attributes. A rabbit child takes its biome's variant one time in 20.
enum InheritedState {
    RabbitVariant(i32),
    Fox(crate::entity::fox::FoxVariant),
    Equine(crate::entity::horse_phenotype::EquineOffspring),
}

impl InheritedState {
    fn apply(self, child: &mut Entity) {
        match self {
            Self::RabbitVariant(variant) => {
                let _ = crate::entity::rabbit::set_variant(child, variant);
            }
            Self::Fox(variant) => {
                let _ = crate::entity::fox::set_spawn_variant(
                    child,
                    variant == crate::entity::fox::FoxVariant::Snow,
                );
            }
            Self::Equine(offspring) => offspring.apply(child),
        }
    }
}

fn inherited_state(
    body: &Entity,
    partner: &Entity,
    random: &GameRandom,
    biome_rabbit_variant: impl FnOnce() -> i32,
) -> Option<InheritedState> {
    let crate::entity::EntityData::GeneralLiving(body_data) = body.data() else {
        return None;
    };
    let crate::entity::EntityData::GeneralLiving(partner_data) = partner.data() else {
        return None;
    };
    if let (Some(first), Some(second)) =
        (body_data.rabbit_retained(), partner_data.rabbit_retained())
    {
        let biome = biome_rabbit_variant();
        if random.next_int(20) == 0 {
            return Some(InheritedState::RabbitVariant(biome));
        }
        let pick = if random.next_boolean() { second } else { first };
        return Some(InheritedState::RabbitVariant(pick.variant()));
    }
    if let (Some(first), Some(second)) = (body_data.fox_retained(), partner_data.fox_retained()) {
        let pick = if random.next_boolean() { first } else { second };
        return Some(InheritedState::Fox(pick.variant()));
    }
    crate::entity::horse_phenotype::equine_offspring(body, partner, random)
        .map(InheritedState::Equine)
}

/// The variant one parent hands the child (`getBreedOffspring`'s
/// `baby.setVariant(...)`), read before the finalize writes so a partner the
/// store no longer holds refuses the child instead of half a mating.
pub(crate) enum ParentVariant {
    Copy(ParentVariantCopy),
    /// The subtype copies no parent state.
    None,
}

impl ParentVariant {
    fn apply(&self, child: &mut Entity) {
        if let Self::Copy(copy) = self {
            apply_parent_variant_copy(child, copy);
        }
    }
}

/// The variant half of `getBreedOffspring` for one plan, or nothing when the
/// plan's source parent is not in the store.
pub(crate) fn parent_variant(
    plan: OffspringPlan,
    body: &Entity,
    partner_id: i32,
    store: &EntityStore,
) -> Option<ParentVariant> {
    match plan {
        OffspringPlan::Variant {
            from_partner: false,
            ..
        }
        | OffspringPlan::Tamable {
            from_partner: false,
            ..
        } => Some(ParentVariant::Copy(parent_variant_copy(body))),
        OffspringPlan::Variant {
            from_partner: true, ..
        }
        | OffspringPlan::Tamable {
            from_partner: true, ..
        } => store
            .get(partner_id)
            .map(|partner| ParentVariant::Copy(parent_variant_copy(partner))),
        OffspringPlan::Nautilus { .. }
        | OffspringPlan::Plain { .. }
        | OffspringPlan::Persistent { .. }
        | OffspringPlan::Resolved { .. } => Some(ParentVariant::None),
    }
}
