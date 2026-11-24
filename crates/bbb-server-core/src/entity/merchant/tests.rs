use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_VILLAGER_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use super::*;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::{EntityData, EntityIdCounter, EntityStore};

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn set_field(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    match entries.iter_mut().find(|entry| entry.name == name) {
        Some(existing) => existing.value = value,
        None => entries.push(entry(name, value)),
    }
}

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(
        NamedCompound::new(
            "",
            vec![
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                entry(
                    "Entities",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entries)]),
                ),
            ],
        ),
        0,
        0,
    )
    .expect("merchant chunk document")
}

/// A villager the store produced itself, finalized the way `Villager.finalizeSpawn`
/// does (`Villager.java:729-731`).
fn finalized_villager_record() -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0x_11_00, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_VILLAGER_ID, position())
        .expect("a fresh villager trades nothing");
    assert!(crate::entity::finalize_spawner_villager_type(
        store.get_mut(id).expect("spawned villager"),
        "minecraft:plains",
    ));
    write_entity_chunk(&store, (0, 0))
        .expect("finalized villager record")
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

fn item_cost(id: &str, count: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("id", NbtSummaryValue::string(id)),
        entry("count", NbtSummaryValue::Int(count)),
    ])
}

/// One offer that leans on every `MerchantOffer.CODEC` default, and one that
/// overrides all of them.
fn offers_field() -> NbtSummaryValue {
    let defaulted = NbtSummaryValue::Compound(vec![
        entry("buy", item_cost("minecraft:emerald", 1)),
        entry("sell", item_cost("minecraft:diamond", 1)),
    ]);
    let overridden = NbtSummaryValue::Compound(vec![
        entry(
            "buy",
            NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::string("minecraft:emerald")),
                entry("count", NbtSummaryValue::Int(3)),
                entry(
                    "components",
                    NbtSummaryValue::Compound(vec![entry(
                        "minecraft:max_stack_size",
                        NbtSummaryValue::Int(16),
                    )]),
                ),
            ]),
        ),
        entry("buyB", item_cost("minecraft:book", 2)),
        entry("sell", item_cost("minecraft:diamond_sword", 1)),
        entry("uses", NbtSummaryValue::Int(3)),
        entry("maxUses", NbtSummaryValue::Int(7)),
        entry("rewardExp", NbtSummaryValue::Byte(0)),
        entry("specialPrice", NbtSummaryValue::Int(-2)),
        entry("demand", NbtSummaryValue::Int(5)),
        entry(
            "priceMultiplier",
            NbtSummaryValue::Float(0.05_f32.to_bits()),
        ),
        entry("xp", NbtSummaryValue::Int(9)),
    ]);
    NbtSummaryValue::Compound(vec![entry(
        "Recipes",
        NbtSummaryValue::List(vec![defaulted, overridden]),
    )])
}

fn stored_offers(store: &EntityStore, id: i32) -> Option<MerchantOffers> {
    let EntityData::GeneralLiving(data) = store.get(id).expect("stored merchant").data() else {
        unreachable!("the villager family uses GeneralLiving")
    };
    data.merchant_offers().cloned()
}

#[test]
fn abstract_villager_family_is_the_two_merchant_types_with_vanilla_trade_generation() {
    let owned = bbb_protocol::entity_types::VANILLA_ENTITY_TYPES
        .iter()
        .filter(|facts| is_abstract_villager(facts.id))
        .map(|facts| facts.resource_id)
        .collect::<Vec<_>>();
    assert_eq!(owned, ["minecraft:villager", "minecraft:wandering_trader"]);

    // `Villager.updateTrades` finds a trade set for every profession but the two
    // registered without one, and only for the five mapped levels.
    for profession in [
        "minecraft:armorer",
        "minecraft:butcher",
        "minecraft:cartographer",
        "minecraft:cleric",
        "minecraft:farmer",
        "minecraft:fisherman",
        "minecraft:fletcher",
        "minecraft:leatherworker",
        "minecraft:librarian",
        "minecraft:mason",
        "minecraft:shepherd",
        "minecraft:toolsmith",
        "minecraft:weaponsmith",
    ] {
        for level in 1..=5 {
            assert!(!generated_offers_stay_empty(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                Some((profession, level))
            ));
        }
        assert!(generated_offers_stay_empty(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            Some((profession, 6))
        ));
    }
    for profession in ["minecraft:none", "minecraft:nitwit"] {
        assert!(generated_offers_stay_empty(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            Some((profession, 1))
        ));
    }
    assert!(!generated_offers_stay_empty(
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        None
    ));

    // The `minecraft:none` villager the constructor produces generates an empty
    // list, which `addAdditionalSaveData` omits.
    let record = finalized_villager_record();
    assert_eq!(field(&record, "Offers"), None);
    assert_eq!(
        field(&record, "VillagerDataFinalized"),
        Some(&NbtSummaryValue::Byte(1))
    );
}

#[test]
fn merchant_offers_restore_decodes_every_codec_default_and_rewrites_them_exactly() {
    let mut record = finalized_villager_record();
    set_field(&mut record, "Offers", offers_field());
    let mut store = EntityStore::new(0x_11_02, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut store, &chunk(record.clone())).expect("villager with offers");
    let offers = stored_offers(&store, ids[0]).expect("restored offers");
    assert_eq!(offers.offers().len(), 2);

    // `MerchantOffer.CODEC`'s ten defaults (`MerchantOffer.java:17-23`).
    let defaulted = &offers.offers()[0];
    assert_eq!(
        defaulted.base_cost_a().item_resource_id(),
        "minecraft:emerald"
    );
    assert_eq!(defaulted.base_cost_a().count(), 1);
    assert!(defaulted.base_cost_a().components().is_empty());
    assert_eq!(defaulted.cost_b(), None);
    assert_eq!(
        defaulted
            .result()
            .write(&bbb_registry_codec::UnavailableHolderResolver)
            .unwrap(),
        Some(item_cost("minecraft:diamond", 1))
    );
    assert_eq!(defaulted.uses(), 0);
    assert_eq!(defaulted.max_uses(), 4);
    assert!(defaulted.reward_exp());
    assert_eq!(defaulted.special_price_diff(), 0);
    assert_eq!(defaulted.demand(), 0);
    assert_eq!(defaulted.price_multiplier().to_bits(), 0.0_f32.to_bits());
    assert_eq!(defaulted.xp(), 1);

    let overridden = &offers.offers()[1];
    assert_eq!(overridden.base_cost_a().count(), 3);
    assert_eq!(
        overridden.base_cost_a().components(),
        [entry("minecraft:max_stack_size", NbtSummaryValue::Int(16))]
    );
    assert_eq!(
        overridden.cost_b().map(MerchantItemCost::item_resource_id),
        Some("minecraft:book")
    );
    assert_eq!(overridden.cost_b().map(MerchantItemCost::count), Some(2));
    assert_eq!(overridden.uses(), 3);
    assert_eq!(overridden.max_uses(), 7);
    assert!(!overridden.reward_exp());
    assert_eq!(overridden.special_price_diff(), -2);
    assert_eq!(overridden.demand(), 5);
    assert_eq!(overridden.price_multiplier().to_bits(), 0.05_f32.to_bits());
    assert_eq!(overridden.xp(), 9);

    // The writer omits every field equal to its default and emits the rest in
    // the codec's group order.
    let written = write_entity_chunk(&store, (0, 0))
        .expect("merchant round trip")
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec();
    assert_eq!(field(&written, "Offers"), Some(&offers_field()));
    let names = |entries: &[NbtSummaryEntry]| {
        let mut names = entries
            .iter()
            .map(|entry| entry.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names
    };
    assert_eq!(names(&written), names(&record));
}

/// The saved shape of a trader the store cannot create itself: the shared
/// `AgeableMob` record with the villager leaf swapped for
/// `WanderingTrader.addAdditionalSaveData` (`WanderingTrader.java:138-142`).
fn wandering_trader_record() -> Vec<NbtSummaryEntry> {
    let mut record = finalized_villager_record();
    set_field(
        &mut record,
        "id",
        NbtSummaryValue::string("minecraft:wandering_trader"),
    );
    set_field(&mut record, "attributes", NbtSummaryValue::List(Vec::new()));
    record.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "VillagerData"
                | "VillagerDataFinalized"
                | "FoodLevel"
                | "Gossips"
                | "Xp"
                | "LastRestock"
                | "LastGossipDecay"
                | "RestocksToday"
        )
    });
    set_field(&mut record, "DespawnDelay", NbtSummaryValue::Int(1200));
    set_field(&mut record, "Offers", offers_field());
    record
}

#[test]
fn merchant_offers_restore_and_rewrite_the_wandering_trader_that_carries_them() {
    let record = wandering_trader_record();
    let mut store = EntityStore::new(0x_11_04, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut store, &chunk(record.clone())).expect("saved trader");
    let offers = stored_offers(&store, ids[0]).expect("restored trader offers");
    assert_eq!(offers.offers().len(), 2);
    assert_eq!(offers.offers()[1].base_cost_a().count(), 3);

    let written = write_entity_chunk(&store, (0, 0))
        .expect("trader round trip")
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec();
    assert_eq!(field(&written, "Offers"), field(&record, "Offers"));
    assert_eq!(
        field(&written, "DespawnDelay"),
        field(&record, "DespawnDelay")
    );
}

fn constructed_offers() -> MerchantOffers {
    let components = vec![entry("minecraft:damage", NbtSummaryValue::Int(2))];
    MerchantOffers::new(vec![MerchantOffer::new(
        MerchantItemCost::new("emerald".into(), 3, components).unwrap(),
        Some(MerchantItemCost::new("minecraft:book".into(), 2, Vec::new()).unwrap()),
        crate::entity::item::read_saved_stack(
            &item_cost("minecraft:diamond", 1),
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .unwrap(),
        12,
        5,
        0.2,
    )])
}

#[test]
fn merchant_typed_construction_preserves_codec_and_offer_defaults() {
    let offers = constructed_offers();
    let offer = &offers.offers()[0];
    assert_eq!(offer.base_cost_a().item_resource_id(), "minecraft:emerald");
    assert_eq!(offer.base_cost_a().count(), 3);
    assert_eq!(offer.uses(), 0);
    assert!(offer.reward_exp());
    assert_eq!(offer.special_price_diff(), 0);
    assert_eq!(offer.demand(), 0);
    assert_eq!(offer.max_uses(), 12);
    assert_eq!(offer.xp(), 5);
    assert_eq!(offer.price_multiplier(), 0.2);
    assert_eq!(
        read_offers(
            &write_offers(&offers, &bbb_registry_codec::UnavailableHolderResolver),
            &bbb_registry_codec::UnavailableHolderResolver
        ),
        Some(offers)
    );
    assert_eq!(
        MerchantItemCost::new("emerald".into(), 0, Vec::new())
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        MerchantItemCost::new("emerald".into(), -2, Vec::new())
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn merchant_initialization_is_once_even_for_empty_offers_and_rejects_non_merchants() {
    for entity_type in [
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
    ] {
        for first in [MerchantOffers::new(Vec::new()), constructed_offers()] {
            let mut data = GeneralLivingData::new(entity_type).unwrap();
            assert!(data.merchant_offers().is_none());
            assert!(data.initialize_merchant_offers(first.clone()));
            assert!(!data.initialize_merchant_offers(constructed_offers()));
            assert!(!data.initialize_merchant_offers(MerchantOffers::default()));
            assert_eq!(data.merchant_offers(), Some(&first));
        }
    }
    let mut pig =
        GeneralLivingData::new(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID).unwrap();
    assert!(!pig.initialize_merchant_offers(constructed_offers()));
    assert!(pig.merchant_offers().is_none());
}
