use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, ProtocolError, Result},
    content_registry::ContentRegistry,
    packets::{
        data_components,
        inventory::{self, ItemStackSummary, SparseDataComponentPatchSummary},
    },
};

const MAX_MERCHANT_OFFERS: usize = 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantOffers {
    pub container_id: i32,
    pub offers: Vec<MerchantOffer>,
    pub villager_level: i32,
    pub villager_xp: i32,
    pub show_progress: bool,
    pub can_restock: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantOffer {
    pub buy_a: ItemCostSummary,
    pub sell: ItemStackSummary,
    pub buy_b: Option<ItemCostSummary>,
    pub is_out_of_stock: bool,
    pub uses: i32,
    pub max_uses: i32,
    pub xp: i32,
    pub special_price_diff: i32,
    pub price_multiplier: f32,
    pub demand: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCostSummary {
    pub item_id: i32,
    pub count: i32,
    pub component_predicate: ItemCostComponentPredicateSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCostComponentPredicateSummary {
    #[serde(default)]
    pub components: SparseDataComponentPatchSummary,
}

impl ItemCostComponentPredicateSummary {
    pub fn is_empty(&self) -> bool {
        self.components.added_type_ids.is_empty()
    }

    pub fn display_patch(&self) -> SparseDataComponentPatchSummary {
        self.components.clone()
    }
}

/// `ClientboundMerchantOffersPacket.write` (`ClientboundMerchantOffersPacket.java:40-47`)
/// over `MerchantOffer.writeToStream` (`MerchantOffer.java:238-249`).
pub fn encode_play_clientbound_merchant_offers(update: &MerchantOffers) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_merchant_offers_with_content_registry(
        update,
        ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_merchant_offers_with_content_registry(
    update: &MerchantOffers,
    registry: &ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    if update.offers.len() > MAX_MERCHANT_OFFERS {
        return Err(ProtocolError::PacketTooLarge(
            update.offers.len(),
            MAX_MERCHANT_OFFERS,
        ));
    }
    let mut encoder = crate::codec::Encoder::new();
    encoder.write_var_i32(update.container_id);
    encoder.write_var_i32(update.offers.len() as i32);
    for offer in &update.offers {
        encode_item_cost_summary(&mut encoder, &offer.buy_a, registry)?;
        if offer.sell.item_id.is_none() || offer.sell.count <= 0 {
            return Err(ProtocolError::InvalidData(
                "merchant offer sell item stack must not be empty".to_string(),
            ));
        }
        super::serverbound::encode_item_stack_summary_with_content_registry(
            &mut encoder,
            &offer.sell,
            data_components::ComponentValueFraming::Direct,
            registry,
        )?;
        encoder.write_bool(offer.buy_b.is_some());
        if let Some(buy_b) = &offer.buy_b {
            encode_item_cost_summary(&mut encoder, buy_b, registry)?;
        }
        encoder.write_bool(offer.is_out_of_stock);
        encoder.write_i32(offer.uses);
        encoder.write_i32(offer.max_uses);
        encoder.write_i32(offer.xp);
        encoder.write_i32(offer.special_price_diff);
        encoder.write_f32(offer.price_multiplier);
        encoder.write_i32(offer.demand);
    }
    encoder.write_var_i32(update.villager_level);
    encoder.write_var_i32(update.villager_xp);
    encoder.write_bool(update.show_progress);
    encoder.write_bool(update.can_restock);
    Ok((
        crate::ids::play::CLIENTBOUND_MERCHANT_OFFERS,
        encoder.into_inner(),
    ))
}

/// `ItemCost.STREAM_CODEC` (`ItemCost.java:26-28`).
fn encode_item_cost_summary(
    encoder: &mut crate::codec::Encoder,
    cost: &ItemCostSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    encoder.write_var_i32(inventory::validate_item_holder_id_with_registry(
        cost.item_id,
        registry,
    )?);
    encoder.write_var_i32(cost.count);
    data_components::encode_data_component_exact_predicate_summary_with_registry(
        encoder,
        &cost.component_predicate.components,
        registry,
    )
}

pub(crate) fn decode_merchant_offers_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<MerchantOffers> {
    let container_id = decoder.read_var_i32()?;
    let offer_count = decoder.read_len()?;
    if offer_count > MAX_MERCHANT_OFFERS {
        return Err(ProtocolError::PacketTooLarge(
            offer_count,
            MAX_MERCHANT_OFFERS,
        ));
    }

    let mut offers = Vec::with_capacity(decoder.bounded_capacity(offer_count, 25));
    for _ in 0..offer_count {
        offers.push(decode_merchant_offer(decoder, registry)?);
    }

    let packet = MerchantOffers {
        container_id,
        offers,
        villager_level: decoder.read_var_i32()?,
        villager_xp: decoder.read_var_i32()?,
        show_progress: decoder.read_bool()?,
        can_restock: decoder.read_bool()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after merchant offers packet".to_string(),
        ));
    }
    Ok(packet)
}

fn decode_merchant_offer(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<MerchantOffer> {
    let buy_a = decode_item_cost_summary(decoder, registry)?;
    let sell = decode_non_empty_item_stack_summary(decoder, registry)?;
    let buy_b = decode_optional_item_cost_summary(decoder, registry)?;
    let encoded_out_of_stock = decoder.read_bool()?;
    let mut uses = decoder.read_i32()?;
    let max_uses = decoder.read_i32()?;
    let xp = decoder.read_i32()?;
    let special_price_diff = decoder.read_i32()?;
    let price_multiplier = decoder.read_f32()?;
    let demand = decoder.read_i32()?;
    if encoded_out_of_stock {
        uses = max_uses;
    }
    Ok(MerchantOffer {
        buy_a,
        sell,
        buy_b,
        is_out_of_stock: uses >= max_uses,
        uses,
        max_uses,
        xp,
        special_price_diff,
        price_multiplier,
        demand,
    })
}

fn decode_optional_item_cost_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<Option<ItemCostSummary>> {
    if decoder.read_bool()? {
        Ok(Some(decode_item_cost_summary(decoder, registry)?))
    } else {
        Ok(None)
    }
}

fn decode_item_cost_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<ItemCostSummary> {
    let item_id = inventory::decode_item_holder_id_with_registry(decoder, registry)?;
    let count = decoder.read_var_i32()?;
    let component_predicate = decode_item_cost_component_predicate_summary(decoder, registry)?;
    Ok(ItemCostSummary {
        item_id,
        count,
        component_predicate,
    })
}

fn decode_item_cost_component_predicate_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<ItemCostComponentPredicateSummary> {
    let components = data_components::decode_data_component_exact_predicate_summary_with_registry(
        decoder, registry,
    )?
    .into();
    Ok(ItemCostComponentPredicateSummary { components })
}

fn decode_non_empty_item_stack_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<ItemStackSummary> {
    let item = inventory::decode_item_stack_summary_with_registry(decoder, registry)?;
    if item.item_id.is_none() {
        return Err(ProtocolError::InvalidData(
            "merchant offer sell item stack must not be empty".to_string(),
        ));
    }
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        codec::Encoder,
        ids,
        packets::{decode_play_clientbound, DataComponentPatchSummary, PlayClientbound},
    };

    #[test]
    fn decodes_merchant_offers_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(1);
        write_item_cost(&mut payload, 42, 3);
        write_item_stack_with_additional_trade_cost(&mut payload, 99, 1, -7);
        payload.write_bool(true);
        write_item_cost(&mut payload, 43, 2);
        payload.write_bool(true);
        payload.write_i32(4);
        payload.write_i32(12);
        payload.write_i32(8);
        payload.write_i32(-2);
        payload.write_f32(0.05);
        payload.write_i32(6);
        payload.write_var_i32(3);
        payload.write_var_i32(120);
        payload.write_bool(true);
        payload.write_bool(false);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_MERCHANT_OFFERS, &payload).unwrap();

        assert_eq!(
            packet,
            PlayClientbound::MerchantOffers(MerchantOffers {
                container_id: 7,
                offers: vec![MerchantOffer {
                    buy_a: item_cost(42, 3),
                    sell: ItemStackSummary {
                        item_id: Some(99),
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![
                                crate::data_component_types::VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
                            ],
                            additional_trade_cost: Some(-7),
                            ..DataComponentPatchSummary::default()
                        }.into(),
                    },
                    buy_b: Some(item_cost(43, 2)),
                    is_out_of_stock: true,
                    uses: 12,
                    max_uses: 12,
                    xp: 8,
                    special_price_diff: -2,
                    price_multiplier: 0.05,
                    demand: 6,
                }],
                villager_level: 3,
                villager_xp: 120,
                show_progress: true,
                can_restock: false,
            })
        );

        let mut trailing = payload;
        trailing.push(0);
        assert!(
            decode_play_clientbound(ids::play::CLIENTBOUND_MERCHANT_OFFERS, &trailing)
                .unwrap_err()
                .to_string()
                .contains("trailing bytes after merchant offers packet")
        );
    }

    #[test]
    fn decodes_merchant_offer_with_supported_component_predicate() {
        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(1);
        payload.write_var_i32(42);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_var_i32(1);
        payload.write_var_i32(32);
        payload.write_var_i32(10);
        payload.write_string("minecraft:emerald");
        write_item_stack(&mut payload, 99, 1);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_i32(0);
        payload.write_i32(12);
        payload.write_i32(1);
        payload.write_i32(0);
        payload.write_f32(0.0);
        payload.write_i32(0);
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_bool(false);
        payload.write_bool(true);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_MERCHANT_OFFERS, &payload).unwrap();
        let PlayClientbound::MerchantOffers(offers) = packet else {
            panic!("expected merchant offers packet");
        };
        assert_eq!(
            encode_play_clientbound_merchant_offers(&offers).unwrap(),
            (ids::play::CLIENTBOUND_MERCHANT_OFFERS, payload)
        );
        assert_eq!(
            offers.offers[0].buy_a.component_predicate,
            ItemCostComponentPredicateSummary {
                components: DataComponentPatchSummary {
                    added_type_ids: vec![1, 10],
                    max_stack_size: Some(32),
                    item_model: Some("minecraft:emerald".to_string()),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            }
        );
    }

    #[test]
    fn merchant_offer_normalizes_wire_stock_state_like_vanilla() {
        let mut payload = Encoder::new();
        write_item_cost(&mut payload, 42, 3);
        write_item_stack(&mut payload, 99, 1);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_i32(4);
        payload.write_i32(12);
        payload.write_i32(8);
        payload.write_i32(0);
        payload.write_f32(0.05);
        payload.write_i32(0);

        let offer = decode_merchant_offer(
            &mut Decoder::new(&payload.into_inner()),
            ContentRegistry::vanilla(),
        )
        .unwrap();
        assert_eq!(offer.uses, 12);
        assert_eq!(offer.max_uses, 12);
        assert!(offer.is_out_of_stock);

        let mut payload = Encoder::new();
        write_item_cost(&mut payload, 42, 3);
        write_item_stack(&mut payload, 99, 1);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_i32(12);
        payload.write_i32(12);
        payload.write_i32(8);
        payload.write_i32(0);
        payload.write_f32(0.05);
        payload.write_i32(0);

        let offer = decode_merchant_offer(
            &mut Decoder::new(&payload.into_inner()),
            ContentRegistry::vanilla(),
        )
        .unwrap();
        assert_eq!(offer.uses, 12);
        assert!(offer.is_out_of_stock);
    }

    #[test]
    fn merchant_item_cost_rejects_oversized_component_predicates_before_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(42);
        payload.write_var_i32(1);
        payload.write_var_i32((data_components::MAX_DATA_COMPONENT_PREDICATE_ENTRIES + 1) as i32);

        let error = decode_item_cost_summary(
            &mut Decoder::new(&payload.into_inner()),
            ContentRegistry::vanilla(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(actual, maximum)
                if actual == data_components::MAX_DATA_COMPONENT_PREDICATE_ENTRIES + 1
                    && maximum == data_components::MAX_DATA_COMPONENT_PREDICATE_ENTRIES
        ));
    }

    #[test]
    fn merchant_item_cost_rejects_an_out_of_range_builtin_item_holder() {
        let mut payload = Encoder::new();
        let invalid_item_id = crate::item_types::VANILLA_ITEM_TYPES.len() as i32;
        payload.write_var_i32(invalid_item_id);
        let error = decode_item_cost_summary(
            &mut Decoder::new(&payload.into_inner()),
            ContentRegistry::vanilla(),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains(&format!("invalid vanilla item holder id {invalid_item_id}")));
    }

    fn write_item_cost(out: &mut Encoder, item_id: i32, count: i32) {
        out.write_var_i32(item_id);
        out.write_var_i32(count);
        out.write_var_i32(0);
    }

    fn write_item_stack(out: &mut Encoder, item_id: i32, count: i32) {
        out.write_var_i32(count);
        out.write_var_i32(item_id);
        out.write_var_i32(0);
        out.write_var_i32(0);
    }

    fn write_item_stack_with_additional_trade_cost(
        out: &mut Encoder,
        item_id: i32,
        count: i32,
        additional_trade_cost: i32,
    ) {
        out.write_var_i32(count);
        out.write_var_i32(item_id);
        out.write_var_i32(1);
        out.write_var_i32(0);
        out.write_var_i32(
            crate::data_component_types::VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
        );
        out.write_var_i32(additional_trade_cost);
    }

    fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
        ItemCostSummary {
            item_id,
            count,
            component_predicate: ItemCostComponentPredicateSummary::default(),
        }
    }
}
