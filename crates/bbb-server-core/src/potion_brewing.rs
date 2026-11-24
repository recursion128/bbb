//! `PotionBrewing` (`PotionBrewing.java:16-246`): the vanilla mix table the
//! brewing stand and its menu both read, with the three container items, the
//! two container mixes and the potion mixes `addVanillaMixes` (`:135-193`)
//! declares.
//!
//! The table is bootstrap data rather than datapack data: vanilla builds it
//! from `Items` and `Potions` holders at startup, so it is resolved at compile
//! time from the generated item and potion catalogs.

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID;
use bbb_protocol::item_types::{
    VANILLA_ITEM_BLAZE_POWDER_ID, VANILLA_ITEM_BREEZE_ROD_ID, VANILLA_ITEM_COBWEB_ID,
    VANILLA_ITEM_DRAGON_BREATH_ID, VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
    VANILLA_ITEM_GHAST_TEAR_ID, VANILLA_ITEM_GLISTERING_MELON_SLICE_ID,
    VANILLA_ITEM_GLOWSTONE_DUST_ID, VANILLA_ITEM_GOLDEN_CARROT_ID, VANILLA_ITEM_GUNPOWDER_ID,
    VANILLA_ITEM_LINGERING_POTION_ID, VANILLA_ITEM_MAGMA_CREAM_ID, VANILLA_ITEM_NETHER_WART_ID,
    VANILLA_ITEM_PHANTOM_MEMBRANE_ID, VANILLA_ITEM_POTION_ID, VANILLA_ITEM_PUFFERFISH_ID,
    VANILLA_ITEM_RABBIT_FOOT_ID, VANILLA_ITEM_REDSTONE_ID, VANILLA_ITEM_SLIME_BLOCK_ID,
    VANILLA_ITEM_SPIDER_EYE_ID, VANILLA_ITEM_SPLASH_POTION_ID, VANILLA_ITEM_STONE_ID,
    VANILLA_ITEM_SUGAR_ID, VANILLA_ITEM_TURTLE_HELMET_ID,
};
use bbb_protocol::packets::{DataComponentPatchSummary, ItemStackSummary};
use bbb_protocol::potion_types::VANILLA_POTIONS;

/// The one `PotionBrewing` every level serves (`ServerLevel.potionBrewing`,
/// `ServerLevel.java:1824-1825`, over `MinecraftServer.potionBrewing`).
static VANILLA: PotionBrewing = PotionBrewing {
    containers: &VANILLA_CONTAINERS,
    potion_mixes: &VANILLA_POTION_MIXES,
    container_mixes: &VANILLA_CONTAINER_MIXES,
};

/// `PotionBrewing` (`PotionBrewing.java:16-21`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PotionBrewing {
    /// The item a potion mix can be applied to (`:19`, `:219-224`).
    containers: &'static [i32],
    /// `PotionBrewing.Mix<Potion>` (`:20`, `:244-245`).
    potion_mixes: &'static [Mix],
    /// `PotionBrewing.Mix<Item>` (`:21`).
    container_mixes: &'static [Mix],
}

/// `PotionBrewing.Mix` (`PotionBrewing.java:244-245`): an ingredient that turns
/// one holder into another. Both mixes are the same shape here because a
/// potion and a container item are both item-registry ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mix {
    from: i32,
    ingredient: i32,
    to: i32,
}

impl PotionBrewing {
    /// `MinecraftServer.potionBrewing` (`:1824-1825`) over
    /// `PotionBrewing.bootstrap` (`PotionBrewing.java:129-133`), resolved from
    /// the generated item and potion catalogs at compile time.
    #[must_use]
    pub const fn vanilla() -> &'static Self {
        &VANILLA
    }

    /// `PotionBrewing.isIngredient` (`:31-33`).
    #[must_use]
    pub fn is_ingredient(&self, ingredient: &ItemStackSummary) -> bool {
        self.is_container_ingredient(ingredient) || self.is_potion_ingredient(ingredient)
    }

    /// `PotionBrewing.isContainerIngredient` (`:45-53`).
    #[must_use]
    pub fn is_container_ingredient(&self, ingredient: &ItemStackSummary) -> bool {
        let Some(item_id) = ingredient.item_id else {
            return false;
        };
        self.container_mixes
            .iter()
            .any(|mix| mix.ingredient == item_id)
    }

    /// `PotionBrewing.isPotionIngredient` (`:55-63`).
    #[must_use]
    pub fn is_potion_ingredient(&self, ingredient: &ItemStackSummary) -> bool {
        let Some(item_id) = ingredient.item_id else {
            return false;
        };
        self.potion_mixes
            .iter()
            .any(|mix| mix.ingredient == item_id)
    }

    /// `PotionBrewing.hasMix` (`:75-77`): a source that is not a container has
    /// no mix at all.
    #[must_use]
    pub fn has_mix(&self, source: &ItemStackSummary, ingredient: &ItemStackSummary) -> bool {
        if !self.is_container(source) {
            return false;
        }
        self.has_container_mix(source, ingredient) || self.has_potion_mix(source, ingredient)
    }

    /// `PotionBrewing.hasContainerMix` (`:79-87`).
    #[must_use]
    pub fn has_container_mix(
        &self,
        source: &ItemStackSummary,
        ingredient: &ItemStackSummary,
    ) -> bool {
        let (Some(from), Some(ingredient_item)) = (source.item_id, ingredient.item_id) else {
            return false;
        };
        self.container_mixes
            .iter()
            .any(|mix| mix.from == from && mix.ingredient == ingredient_item)
    }

    /// `PotionBrewing.hasPotionMix` (`:89-102`): a source whose potion contents
    /// name no potion has no mix.
    #[must_use]
    pub fn has_potion_mix(&self, source: &ItemStackSummary, ingredient: &ItemStackSummary) -> bool {
        let (Some(from), Some(ingredient_item)) = (potion_id(source), ingredient.item_id) else {
            return false;
        };
        self.potion_mixes
            .iter()
            .any(|mix| mix.from == from && mix.ingredient == ingredient_item)
    }

    /// `PotionBrewing.mix` (`:104-127`): a container mix replaces the item and
    /// keeps the potion, a potion mix keeps the item and replaces the potion,
    /// and a source with no mix comes back unchanged.
    ///
    /// `PotionContents.createItemStack` (`PotionContents.java:96-100`) builds a
    /// fresh stack, so every other component the source carried is dropped.
    #[must_use]
    pub fn mix(
        &self,
        ingredient: &ItemStackSummary,
        source: &ItemStackSummary,
    ) -> ItemStackSummary {
        if stack_is_empty(source) {
            return source.clone();
        }
        let Some(potion) = potion_id(source) else {
            return source.clone();
        };
        if let Some(item_id) = source.item_id {
            if let Some(ingredient_item) = ingredient.item_id {
                if let Some(mix) = self
                    .container_mixes
                    .iter()
                    .find(|mix| mix.from == item_id && mix.ingredient == ingredient_item)
                {
                    return potion_stack(mix.to, potion);
                }
            }
        }
        if let Some(ingredient_item) = ingredient.item_id {
            if let Some(mix) = self
                .potion_mixes
                .iter()
                .find(|mix| mix.from == potion && mix.ingredient == ingredient_item)
            {
                let Some(item_id) = source.item_id else {
                    return source.clone();
                };
                return potion_stack(item_id, mix.to);
            }
        }
        source.clone()
    }

    /// `PotionBrewing.isContainer` (`:35-43`).
    fn is_container(&self, source: &ItemStackSummary) -> bool {
        let Some(item_id) = source.item_id else {
            return false;
        };
        self.containers.contains(&item_id)
    }
}

/// `PotionContents.createItemStack` (`PotionContents.java:96-100`).
fn potion_stack(item_id: i32, potion_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
            potion_contents: bbb_protocol::packets::PotionContentsSummary {
                potion_id: Some(potion_id),
                ..Default::default()
            },
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

/// `ItemStack.getOrDefault(DataComponents.POTION_CONTENTS, PotionContents.EMPTY).potion()`
/// (`PotionBrewing.java:90`, `:109`).
fn potion_id(stack: &ItemStackSummary) -> Option<i32> {
    stack.component_patch.potion_contents.potion_id
}

fn stack_is_empty(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

const fn str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

const fn potion(resource_id: &str) -> i32 {
    let mut index = 0;
    while index < VANILLA_POTIONS.len() {
        if str_eq(VANILLA_POTIONS[index].resource_id, resource_id) {
            return VANILLA_POTIONS[index].id;
        }
        index += 1;
    }
    panic!("unknown built-in 26.1 potion")
}

/// `PotionBrewing.Builder.addMix` (`PotionBrewing.java:226-230`).
const fn mix(from: &str, ingredient: i32, to: &str) -> Mix {
    Mix {
        from: potion(from),
        ingredient,
        to: potion(to),
    }
}

const VANILLA_CONTAINERS: [i32; 3] = [
    VANILLA_ITEM_POTION_ID,
    VANILLA_ITEM_SPLASH_POTION_ID,
    VANILLA_ITEM_LINGERING_POTION_ID,
];

const VANILLA_CONTAINER_MIXES: [Mix; 2] = [
    Mix {
        from: VANILLA_ITEM_POTION_ID,
        ingredient: VANILLA_ITEM_GUNPOWDER_ID,
        to: VANILLA_ITEM_SPLASH_POTION_ID,
    },
    Mix {
        from: VANILLA_ITEM_SPLASH_POTION_ID,
        ingredient: VANILLA_ITEM_DRAGON_BREATH_ID,
        to: VANILLA_ITEM_LINGERING_POTION_ID,
    },
];

/// `PotionBrewing.addVanillaMixes` (`PotionBrewing.java:135-193`) in the
/// order the builder receives it, with `addStartMix` (`PotionBrewing.java:232-237`)
/// adding its water->mundane and awkward->target pair where it stands.
const VANILLA_POTION_MIXES: [Mix; 63] = [
    mix(
        "minecraft:water",
        VANILLA_ITEM_GLOWSTONE_DUST_ID,
        "minecraft:thick",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_NETHER_WART_ID,
        "minecraft:awkward",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_BREEZE_ROD_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_BREEZE_ROD_ID,
        "minecraft:wind_charged",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_SLIME_BLOCK_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_SLIME_BLOCK_ID,
        "minecraft:oozing",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_STONE_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_STONE_ID,
        "minecraft:infested",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_COBWEB_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_COBWEB_ID,
        "minecraft:weaving",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_GOLDEN_CARROT_ID,
        "minecraft:night_vision",
    ),
    mix(
        "minecraft:night_vision",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_night_vision",
    ),
    mix(
        "minecraft:night_vision",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:invisibility",
    ),
    mix(
        "minecraft:long_night_vision",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:long_invisibility",
    ),
    mix(
        "minecraft:invisibility",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_invisibility",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_MAGMA_CREAM_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_MAGMA_CREAM_ID,
        "minecraft:fire_resistance",
    ),
    mix(
        "minecraft:fire_resistance",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_fire_resistance",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_RABBIT_FOOT_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_RABBIT_FOOT_ID,
        "minecraft:leaping",
    ),
    mix(
        "minecraft:leaping",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_leaping",
    ),
    mix(
        "minecraft:leaping",
        VANILLA_ITEM_GLOWSTONE_DUST_ID,
        "minecraft:strong_leaping",
    ),
    mix(
        "minecraft:leaping",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:slowness",
    ),
    mix(
        "minecraft:long_leaping",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:long_slowness",
    ),
    mix(
        "minecraft:slowness",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_slowness",
    ),
    mix(
        "minecraft:slowness",
        VANILLA_ITEM_GLOWSTONE_DUST_ID,
        "minecraft:strong_slowness",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_TURTLE_HELMET_ID,
        "minecraft:turtle_master",
    ),
    mix(
        "minecraft:turtle_master",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_turtle_master",
    ),
    mix(
        "minecraft:turtle_master",
        VANILLA_ITEM_GLOWSTONE_DUST_ID,
        "minecraft:strong_turtle_master",
    ),
    mix(
        "minecraft:swiftness",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:slowness",
    ),
    mix(
        "minecraft:long_swiftness",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:long_slowness",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_SUGAR_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_SUGAR_ID,
        "minecraft:swiftness",
    ),
    mix(
        "minecraft:swiftness",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_swiftness",
    ),
    mix(
        "minecraft:swiftness",
        VANILLA_ITEM_GLOWSTONE_DUST_ID,
        "minecraft:strong_swiftness",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_PUFFERFISH_ID,
        "minecraft:water_breathing",
    ),
    mix(
        "minecraft:water_breathing",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_water_breathing",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_GLISTERING_MELON_SLICE_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_GLISTERING_MELON_SLICE_ID,
        "minecraft:healing",
    ),
    mix(
        "minecraft:healing",
        VANILLA_ITEM_GLOWSTONE_DUST_ID,
        "minecraft:strong_healing",
    ),
    mix(
        "minecraft:healing",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:harming",
    ),
    mix(
        "minecraft:strong_healing",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:strong_harming",
    ),
    mix(
        "minecraft:harming",
        VANILLA_ITEM_GLOWSTONE_DUST_ID,
        "minecraft:strong_harming",
    ),
    mix(
        "minecraft:poison",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:harming",
    ),
    mix(
        "minecraft:long_poison",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:harming",
    ),
    mix(
        "minecraft:strong_poison",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:strong_harming",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_SPIDER_EYE_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_SPIDER_EYE_ID,
        "minecraft:poison",
    ),
    mix(
        "minecraft:poison",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_poison",
    ),
    mix(
        "minecraft:poison",
        VANILLA_ITEM_GLOWSTONE_DUST_ID,
        "minecraft:strong_poison",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_GHAST_TEAR_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_GHAST_TEAR_ID,
        "minecraft:regeneration",
    ),
    mix(
        "minecraft:regeneration",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_regeneration",
    ),
    mix(
        "minecraft:regeneration",
        VANILLA_ITEM_GLOWSTONE_DUST_ID,
        "minecraft:strong_regeneration",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_BLAZE_POWDER_ID,
        "minecraft:mundane",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_BLAZE_POWDER_ID,
        "minecraft:strength",
    ),
    mix(
        "minecraft:strength",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_strength",
    ),
    mix(
        "minecraft:strength",
        VANILLA_ITEM_GLOWSTONE_DUST_ID,
        "minecraft:strong_strength",
    ),
    mix(
        "minecraft:water",
        VANILLA_ITEM_FERMENTED_SPIDER_EYE_ID,
        "minecraft:weakness",
    ),
    mix(
        "minecraft:weakness",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_weakness",
    ),
    mix(
        "minecraft:awkward",
        VANILLA_ITEM_PHANTOM_MEMBRANE_ID,
        "minecraft:slow_falling",
    ),
    mix(
        "minecraft:slow_falling",
        VANILLA_ITEM_REDSTONE_ID,
        "minecraft:long_slow_falling",
    ),
];
