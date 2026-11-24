//! `AbstractVillager`'s merchant offers as live entity state.
//!
//! The offers are decoded on restore and re-encoded on save through
//! `MerchantOffers.CODEC`, so a stored merchant carries a list this server can
//! read rather than an opaque save field. `AbstractVillager.getOffers` fills a
//! missing list from `updateTrades`; only the merchants whose trade set is
//! absent are owned here, and every other one is refused rather than stored
//! without the offers vanilla would have generated.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_VILLAGER_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
};
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::save::EntitySaveError;
use crate::entity::save::{optional_bool, optional_field, optional_float, optional_int};
use crate::entity::GeneralLivingData;

/// `MerchantOffer.CODEC`'s optional-field defaults (`MerchantOffer.java:17-23`).
const DEFAULT_USES: i32 = 0;
const DEFAULT_MAX_USES: i32 = 4;
const DEFAULT_REWARD_EXP: bool = true;
const DEFAULT_SPECIAL_PRICE_DIFF: i32 = 0;
const DEFAULT_DEMAND: i32 = 0;
const DEFAULT_PRICE_MULTIPLIER: f32 = 0.0;
const DEFAULT_XP: i32 = 1;

/// `VillagerData.MAX_VILLAGER_LEVEL` (`VillagerData.java:15`), the last level
/// `VillagerProfession.tradeSetsByLevel` maps.
const MAX_VILLAGER_LEVEL: i32 = 5;

/// `ItemCost` (`ItemCost.java:17-25`): an item, a positive count and an exact
/// component predicate whose empty form the writer omits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerchantItemCost {
    item_resource_id: String,
    count: i32,
    components: Vec<NbtSummaryEntry>,
}

impl MerchantItemCost {
    pub fn new(
        item_resource_id: String,
        count: i32,
        components: Vec<NbtSummaryEntry>,
    ) -> Result<Self, EntitySaveError> {
        validated_item_cost(item_resource_id, count, components, "buy")
    }

    pub fn item_resource_id(&self) -> &str {
        &self.item_resource_id
    }

    pub const fn count(&self) -> i32 {
        self.count
    }

    pub fn components(&self) -> &[NbtSummaryEntry] {
        &self.components
    }
}

/// `MerchantOffer` (`MerchantOffer.java:30-39`).
#[derive(Debug, Clone, PartialEq)]
pub struct MerchantOffer {
    base_cost_a: MerchantItemCost,
    cost_b: Option<MerchantItemCost>,
    result: RuntimeItemStack,
    uses: i32,
    max_uses: i32,
    reward_exp: bool,
    special_price_diff: i32,
    demand: i32,
    price_multiplier: f32,
    xp: i32,
}

impl MerchantOffer {
    /// `MerchantOffer.java:69-97`.
    pub fn new(
        base_cost_a: MerchantItemCost,
        cost_b: Option<MerchantItemCost>,
        result: RuntimeItemStack,
        max_uses: i32,
        xp: i32,
        price_multiplier: f32,
    ) -> Self {
        Self {
            base_cost_a,
            cost_b,
            result,
            uses: DEFAULT_USES,
            max_uses,
            reward_exp: DEFAULT_REWARD_EXP,
            special_price_diff: DEFAULT_SPECIAL_PRICE_DIFF,
            demand: DEFAULT_DEMAND,
            price_multiplier,
            xp,
        }
    }

    pub const fn base_cost_a(&self) -> &MerchantItemCost {
        &self.base_cost_a
    }

    pub const fn cost_b(&self) -> Option<&MerchantItemCost> {
        self.cost_b.as_ref()
    }

    pub const fn result(&self) -> &RuntimeItemStack {
        &self.result
    }

    pub const fn uses(&self) -> i32 {
        self.uses
    }

    pub const fn max_uses(&self) -> i32 {
        self.max_uses
    }

    pub const fn reward_exp(&self) -> bool {
        self.reward_exp
    }

    pub const fn special_price_diff(&self) -> i32 {
        self.special_price_diff
    }

    pub const fn demand(&self) -> i32 {
        self.demand
    }

    pub const fn price_multiplier(&self) -> f32 {
        self.price_multiplier
    }

    pub const fn xp(&self) -> i32 {
        self.xp
    }

    /// `MerchantOffer.getModifiedCostCount` (`MerchantOffer.java:123-127`) for
    /// the first cost, whose stack caps the count at `max_stack_size`.
    pub fn modified_cost_a_count(&self, max_stack_size: i32) -> i32 {
        let base = self.base_cost_a.count;
        let demand_diff =
            ((base.wrapping_mul(self.demand) as f32 * self.price_multiplier).floor() as i32).max(0);
        base.wrapping_add(demand_diff)
            .wrapping_add(self.special_price_diff)
            .max(1)
            .min(max_stack_size)
    }

    /// `MerchantOffer.isOutOfStock` (`MerchantOffer.java:197-199`).
    pub const fn is_out_of_stock(&self) -> bool {
        self.uses >= self.max_uses
    }

    /// `MerchantOffer.increaseUses` (`MerchantOffer.java:165-167`).
    pub fn increase_uses(&mut self) {
        self.uses = self.uses.wrapping_add(1);
    }

    /// `MerchantOffer.updateDemand` (`MerchantOffer.java:145-147`).
    pub fn update_demand(&mut self) {
        self.demand = self
            .demand
            .wrapping_add(self.uses)
            .wrapping_sub(self.max_uses.wrapping_sub(self.uses));
    }

    pub fn reset_uses(&mut self) {
        self.uses = 0;
    }

    /// `MerchantOffer.needsRestock` (`MerchantOffer.java:205-207`).
    pub const fn needs_restock(&self) -> bool {
        self.uses > 0
    }

    pub fn add_to_special_price_diff(&mut self, add: i32) {
        self.special_price_diff = self.special_price_diff.wrapping_add(add);
    }

    pub fn reset_special_price_diff(&mut self) {
        self.special_price_diff = 0;
    }
}

/// `MerchantOffers` (`MerchantOffers.java:14-19`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MerchantOffers(Vec<MerchantOffer>);

impl MerchantOffers {
    pub fn new(offers: Vec<MerchantOffer>) -> Self {
        Self(offers)
    }

    pub fn offers(&self) -> &[MerchantOffer] {
        &self.0
    }

    pub fn offers_mut(&mut self) -> &mut [MerchantOffer] {
        &mut self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn extend(&mut self, offers: impl IntoIterator<Item = MerchantOffer>) {
        self.0.extend(offers);
    }
}

/// `AbstractVillager.offers` (`AbstractVillager.java:58`), absent until
/// `getOffers` generates it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AbstractVillagerRetainedState {
    offers: Option<MerchantOffers>,
    pub(crate) trading_player: Option<i32>,
    /// `AbstractVillager.DATA_UNHAPPY_COUNTER`, which the client reads to
    /// shake the head.
    pub(crate) unhappy_counter: i32,
}

impl AbstractVillagerRetainedState {
    pub(crate) const fn offers(&self) -> Option<&MerchantOffers> {
        self.offers.as_ref()
    }

    pub(crate) fn clear_offers(&mut self) {
        self.offers = None;
    }
}

/// The two `AbstractVillager` subclasses (`Villager.java:95`,
/// `WanderingTrader.java:50`).
pub(crate) const fn is_abstract_villager(entity_type_id: i32) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_VILLAGER_ID
        || entity_type_id == VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
}

/// Whether `AbstractVillager.getOffers` would leave the generated list empty.
///
/// `Villager.updateTrades` adds nothing when `VillagerProfession.getTrades`
/// finds no trade set for the current level (`Villager.java:802-812`,
/// `VillagerProfession.java:299-301`), which is every level of the two
/// professions bootstrapped without a trade map (`VillagerProfession.java:123`,
/// `VillagerProfession.java:257`) and every level past the mapped five.
/// `WanderingTrader.updateTrades` always draws three trade sets
/// (`WanderingTrader.java:130-135`); its loaded generation belongs to the world owner.
pub(crate) fn generated_offers_stay_empty(
    entity_type_id: i32,
    villager_data: Option<(&str, i32)>,
) -> bool {
    if entity_type_id != VANILLA_ENTITY_TYPE_VILLAGER_ID {
        return false;
    }
    let Some((profession, level)) = villager_data else {
        return false;
    };
    matches!(profession, "minecraft:none" | "minecraft:nitwit") || level > MAX_VILLAGER_LEVEL
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    name == "Offers"
}

/// `AbstractVillager.readAdditionalSaveData` (`AbstractVillager.java:185-189`).
pub(crate) fn read_save_data(
    entries: &[NbtSummaryEntry],
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> AbstractVillagerRetainedState {
    AbstractVillagerRetainedState {
        offers: optional_field(entries, "Offers").and_then(|value| read_offers(value, items)),
        trading_player: None,
        unhappy_counter: 0,
    }
}

/// `AbstractVillager.addAdditionalSaveData` (`AbstractVillager.java:172-182`),
/// which reads through the lazy `getOffers` generation at `:112-123` first and
/// then stores only a nonempty list. Mutable serialization entry points
/// materialize offers beforehand; immutable snapshots omit `Offers` when
/// uninitialized so loading re-triggers lazy generation.
pub(crate) fn write_save_data(
    entries: &mut Vec<NbtSummaryEntry>,
    state: &AbstractVillagerRetainedState,
    _entity_type_id: i32,
    _villager_data: Option<(&str, i32)>,
    items: crate::entity::item::ItemResolver<'_>,
) -> Result<(), EntitySaveError> {
    let Some(offers) = &state.offers else {
        return Ok(());
    };
    if offers.is_empty() {
        return Ok(());
    }
    entries.push(NbtSummaryEntry {
        name: "Offers".into(),
        value: write_offers(offers, items),
    });
    Ok(())
}

/// `MerchantOffers.CODEC` (`MerchantOffers.java:15-19`): offers that fail to
/// decode are dropped from the partial list.
pub(crate) fn read_offers(
    value: &NbtSummaryValue,
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Option<MerchantOffers> {
    const MAX_MERCHANT_OFFERS: usize = 1024;
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let recipes = match optional_field(entries, "Recipes") {
        None => &[][..],
        Some(NbtSummaryValue::List(recipes)) => recipes,
        Some(_) => return None,
    };
    Some(MerchantOffers(
        recipes
            .iter()
            .filter_map(|value| read_offer(value, items))
            .take(MAX_MERCHANT_OFFERS)
            .collect(),
    ))
}

pub(crate) fn write_offers(
    offers: &MerchantOffers,
    items: crate::entity::item::ItemResolver<'_>,
) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "Recipes".into(),
        value: NbtSummaryValue::List(
            offers
                .0
                .iter()
                .map(|offer| write_offer(offer, items))
                .collect(),
        ),
    }])
}

/// `MerchantOffer.CODEC` (`MerchantOffer.java:12-26`).
fn read_offer(
    value: &NbtSummaryValue,
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Option<MerchantOffer> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let base_cost_a = read_item_cost(optional_field(entries, "buy")?, items)?;
    let cost_b = optional_field(entries, "buyB").and_then(|value| read_item_cost(value, items));
    let result = crate::entity::item::read_saved_stack(optional_field(entries, "sell")?, items)
        .filter(|stack| !stack.is_empty())?;
    Some(MerchantOffer {
        base_cost_a,
        cost_b,
        result,
        uses: optional_int(entries, "uses").unwrap_or(DEFAULT_USES),
        max_uses: optional_int(entries, "maxUses").unwrap_or(DEFAULT_MAX_USES),
        reward_exp: optional_bool(entries, "rewardExp").unwrap_or(DEFAULT_REWARD_EXP),
        special_price_diff: optional_int(entries, "specialPrice")
            .unwrap_or(DEFAULT_SPECIAL_PRICE_DIFF),
        demand: optional_int(entries, "demand").unwrap_or(DEFAULT_DEMAND),
        price_multiplier: optional_float(entries, "priceMultiplier")
            .unwrap_or(DEFAULT_PRICE_MULTIPLIER),
        xp: optional_int(entries, "xp").unwrap_or(DEFAULT_XP),
    })
}

fn write_offer(
    offer: &MerchantOffer,
    items: crate::entity::item::ItemResolver<'_>,
) -> NbtSummaryValue {
    let mut entries = vec![NbtSummaryEntry {
        name: "buy".into(),
        value: write_item_cost(&offer.base_cost_a),
    }];
    if let Some(cost_b) = &offer.cost_b {
        entries.push(NbtSummaryEntry {
            name: "buyB".into(),
            value: write_item_cost(cost_b),
        });
    }
    entries.push(NbtSummaryEntry {
        name: "sell".into(),
        value: crate::entity::item::write_saved_stack(&offer.result, items),
    });
    // Every remaining field is an `optionalFieldOf` whose default encodes as
    // nothing, emitted in the codec's own group order.
    push_int_unless_default(&mut entries, "uses", offer.uses, DEFAULT_USES);
    push_int_unless_default(&mut entries, "maxUses", offer.max_uses, DEFAULT_MAX_USES);
    if offer.reward_exp != DEFAULT_REWARD_EXP {
        entries.push(NbtSummaryEntry {
            name: "rewardExp".into(),
            value: NbtSummaryValue::Byte(i8::from(offer.reward_exp)),
        });
    }
    push_int_unless_default(
        &mut entries,
        "specialPrice",
        offer.special_price_diff,
        DEFAULT_SPECIAL_PRICE_DIFF,
    );
    push_int_unless_default(&mut entries, "demand", offer.demand, DEFAULT_DEMAND);
    if offer.price_multiplier.to_bits() != DEFAULT_PRICE_MULTIPLIER.to_bits() {
        entries.push(NbtSummaryEntry {
            name: "priceMultiplier".into(),
            value: NbtSummaryValue::Float(offer.price_multiplier.to_bits()),
        });
    }
    push_int_unless_default(&mut entries, "xp", offer.xp, DEFAULT_XP);
    NbtSummaryValue::Compound(entries)
}

fn push_int_unless_default(
    entries: &mut Vec<NbtSummaryEntry>,
    name: &str,
    value: i32,
    default: i32,
) {
    if value != default {
        entries.push(NbtSummaryEntry {
            name: name.into(),
            value: NbtSummaryValue::Int(value),
        });
    }
}

fn read_item_cost(
    value: &NbtSummaryValue,
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Option<MerchantItemCost> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let Some(NbtSummaryValue::String(raw_id)) = optional_field(entries, "id") else {
        return None;
    };
    // `ExtraCodecs.POSITIVE_INT.fieldOf("count").orElse(1)` swallows a missing
    // or non-positive count (`ItemCost.java:21`).
    let count = optional_int(entries, "count").unwrap_or(1);
    let components = match optional_field(entries, "components") {
        None => Vec::new(),
        Some(NbtSummaryValue::Compound(components)) => {
            // `ItemCost.CODEC` (`ItemCost.java:17-28`): `DataComponentExactPredicate.CODEC`
            // rejects `!` removals and invalid component payloads, dropping only this offer.
            if !components.is_empty() {
                let patch = bbb_registry_codec::read_component_patch(
                    &NbtSummaryValue::Compound(components.clone()),
                    items,
                )
                .ok()?;
                if !patch.removed_type_ids.is_empty() {
                    return None;
                }
            }
            components.clone()
        }
        Some(_) => return None,
    };
    validated_item_cost_with_registry(
        raw_id.try_to_string().ok()?,
        count,
        components,
        "buy",
        items.content_registry(),
    )
    .ok()
}

fn validated_item_cost(
    raw_id: String,
    count: i32,
    components: Vec<NbtSummaryEntry>,
    field: &'static str,
) -> Result<MerchantItemCost, EntitySaveError> {
    validated_item_cost_with_registry(
        raw_id,
        count,
        components,
        field,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

fn validated_item_cost_with_registry(
    raw_id: String,
    count: i32,
    components: Vec<NbtSummaryEntry>,
    field: &'static str,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<MerchantItemCost, EntitySaveError> {
    let item_resource_id = normalize_resource_location(raw_id)
        .map_err(|error| field_error(field, format!("id is not a resource location: {error}")))?;
    if vanilla_item_type_for_resource_id(&item_resource_id).is_none()
        && content_registry
            .item_by_resource_id(&item_resource_id)
            .is_none()
    {
        return Err(field_error(
            field,
            format!("id {item_resource_id} is absent from the built-in item registry"),
        ));
    }
    Ok(MerchantItemCost {
        item_resource_id,
        count: if count > 0 { count } else { 1 },
        components,
    })
}

fn write_item_cost(cost: &MerchantItemCost) -> NbtSummaryValue {
    let mut entries = vec![
        NbtSummaryEntry {
            name: "id".into(),
            value: NbtSummaryValue::string(cost.item_resource_id.clone()),
        },
        NbtSummaryEntry {
            name: "count".into(),
            value: NbtSummaryValue::Int(cost.count),
        },
    ];
    if !cost.components.is_empty() {
        entries.push(NbtSummaryEntry {
            name: "components".into(),
            value: NbtSummaryValue::Compound(cost.components.clone()),
        });
    }
    NbtSummaryValue::Compound(entries)
}

impl crate::entity::Entity {
    pub fn merchant_offers(&self) -> Option<&MerchantOffers> {
        match &self.data {
            crate::entity::EntityData::GeneralLiving(data) => data.merchant_offers(),
            _ => None,
        }
    }

    pub fn merchant_offers_mut(&mut self) -> Option<&mut MerchantOffers> {
        match &mut self.data {
            crate::entity::EntityData::GeneralLiving(data) => data.merchant_offers_mut(),
            _ => None,
        }
    }

    pub fn merchant_trading_player(&self) -> Option<i32> {
        match &self.data {
            crate::entity::EntityData::GeneralLiving(data) => data.merchant_trading_player(),
            _ => None,
        }
    }

    pub fn initialize_merchant_offers(&mut self, offers: MerchantOffers) -> bool {
        match &mut self.data {
            crate::entity::EntityData::GeneralLiving(data) => {
                data.initialize_merchant_offers(offers)
            }
            _ => false,
        }
    }
}

impl GeneralLivingData {
    /// `AbstractVillager.java:112-123`: an initialized empty list is still authoritative.
    pub fn initialize_merchant_offers(&mut self, offers: MerchantOffers) -> bool {
        let Some(state) = self.abstract_villager_retained_mut() else {
            return false;
        };
        if state.offers.is_some() {
            return false;
        }
        state.offers = Some(offers);
        true
    }

    /// The merchant offers a stored villager or wandering trader carries.
    pub fn merchant_offers(&self) -> Option<&MerchantOffers> {
        self.abstract_villager_retained()?.offers()
    }

    pub fn merchant_offers_mut(&mut self) -> Option<&mut MerchantOffers> {
        self.abstract_villager_retained_mut()?.offers.as_mut()
    }

    /// `AbstractVillager.getTradingPlayer` (`AbstractVillager.java:103-105`).
    pub fn merchant_trading_player(&self) -> Option<i32> {
        self.abstract_villager_retained()?.trading_player
    }
}

fn field_error(field: &'static str, detail: impl Into<String>) -> EntitySaveError {
    EntitySaveError::Field {
        field,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests;
