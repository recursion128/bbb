use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, ProtocolError, Result},
    packets::inventory::{self, ItemStackSummary},
};

const MAX_MERCHANT_OFFERS: usize = 1024;
const MAX_ITEM_COST_COMPONENTS: usize = 1024;

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
    pub component_count: usize,
}

pub(super) fn decode_merchant_offers(decoder: &mut Decoder<'_>) -> Result<MerchantOffers> {
    let container_id = decoder.read_var_i32()?;
    let offer_count = decoder.read_len()?;
    if offer_count > MAX_MERCHANT_OFFERS {
        return Err(ProtocolError::PacketTooLarge(
            offer_count,
            MAX_MERCHANT_OFFERS,
        ));
    }

    let mut offers = Vec::with_capacity(offer_count);
    for _ in 0..offer_count {
        offers.push(decode_merchant_offer(decoder)?);
    }

    Ok(MerchantOffers {
        container_id,
        offers,
        villager_level: decoder.read_var_i32()?,
        villager_xp: decoder.read_var_i32()?,
        show_progress: decoder.read_bool()?,
        can_restock: decoder.read_bool()?,
    })
}

fn decode_merchant_offer(decoder: &mut Decoder<'_>) -> Result<MerchantOffer> {
    Ok(MerchantOffer {
        buy_a: decode_item_cost_summary(decoder)?,
        sell: decode_non_empty_item_stack_summary(decoder)?,
        buy_b: decode_optional_item_cost_summary(decoder)?,
        is_out_of_stock: decoder.read_bool()?,
        uses: decoder.read_i32()?,
        max_uses: decoder.read_i32()?,
        xp: decoder.read_i32()?,
        special_price_diff: decoder.read_i32()?,
        price_multiplier: decoder.read_f32()?,
        demand: decoder.read_i32()?,
    })
}

fn decode_optional_item_cost_summary(decoder: &mut Decoder<'_>) -> Result<Option<ItemCostSummary>> {
    if decoder.read_bool()? {
        Ok(Some(decode_item_cost_summary(decoder)?))
    } else {
        Ok(None)
    }
}

fn decode_item_cost_summary(decoder: &mut Decoder<'_>) -> Result<ItemCostSummary> {
    let item_id = decoder.read_var_i32()?;
    let count = decoder.read_var_i32()?;
    let component_predicate = decode_item_cost_component_predicate_summary(decoder)?;
    Ok(ItemCostSummary {
        item_id,
        count,
        component_predicate,
    })
}

fn decode_item_cost_component_predicate_summary(
    decoder: &mut Decoder<'_>,
) -> Result<ItemCostComponentPredicateSummary> {
    let component_count = decoder.read_len()?;
    if component_count > MAX_ITEM_COST_COMPONENTS {
        return Err(ProtocolError::PacketTooLarge(
            component_count,
            MAX_ITEM_COST_COMPONENTS,
        ));
    }
    if component_count != 0 {
        return Err(ProtocolError::InvalidData(format!(
            "unsupported item cost component predicate with {component_count} component(s)"
        )));
    }

    Ok(ItemCostComponentPredicateSummary { component_count })
}

fn decode_non_empty_item_stack_summary(decoder: &mut Decoder<'_>) -> Result<ItemStackSummary> {
    let item = inventory::decode_item_stack_summary(decoder)?;
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
        write_item_stack(&mut payload, 99, 1);
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

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_MERCHANT_OFFERS,
            &payload.into_inner(),
        )
        .unwrap();

        assert_eq!(
            packet,
            PlayClientbound::MerchantOffers(MerchantOffers {
                container_id: 7,
                offers: vec![MerchantOffer {
                    buy_a: item_cost(42, 3),
                    sell: ItemStackSummary {
                        item_id: Some(99),
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    buy_b: Some(item_cost(43, 2)),
                    is_out_of_stock: true,
                    uses: 4,
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
    }

    #[test]
    fn rejects_merchant_offer_with_component_predicate() {
        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(1);
        payload.write_var_i32(42);
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(5);

        let error = decode_play_clientbound(
            ids::play::CLIENTBOUND_MERCHANT_OFFERS,
            &payload.into_inner(),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported item cost component predicate"));
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

    fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
        ItemCostSummary {
            item_id,
            count,
            component_predicate: ItemCostComponentPredicateSummary::default(),
        }
    }
}
