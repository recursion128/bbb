use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::decode_component_summary_from_decoder,
    packets::data_components,
};

const MAX_CONTAINER_ITEMS: usize = 1024;

pub use data_components::{
    decode_profile_textures_from_properties, AttackRangeSummary, CustomModelDataFloats,
    DataComponentPatchSummary, GameProfilePropertySummary, ItemEnchantmentSummary,
    ItemRaritySummary, ItemStackTemplateSummary, MapPostProcessingSummary, PlayerModelTypeSummary,
    PlayerSkinPatchSummary, ProfileSkinTextureSummary, ProfileTextureSummary,
    ProfileTexturesSummary, ResolvableProfileKindSummary, ResolvableProfileSummary,
    ResourceTextureSummary, SwingAnimationSummary, SwingAnimationTypeSummary, UseEffectsSummary,
    WrittenBookContentSummary,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStackSummary {
    pub item_id: Option<i32>,
    pub count: i32,
    pub component_patch: DataComponentPatchSummary,
}

impl ItemStackSummary {
    pub fn empty() -> Self {
        Self {
            item_id: None,
            count: 0,
            component_patch: DataComponentPatchSummary::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerClose {
    pub container_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSetContent {
    pub container_id: i32,
    pub state_id: i32,
    pub items: Vec<ItemStackSummary>,
    pub carried_item: ItemStackSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSetData {
    pub container_id: i32,
    pub id: i16,
    pub value: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSetSlot {
    pub container_id: i32,
    pub state_id: i32,
    pub slot: i16,
    pub item: ItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenScreen {
    pub container_id: i32,
    pub menu_type_id: i32,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCursorItem {
    pub item: ItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPlayerInventory {
    pub slot: i32,
    pub item: ItemStackSummary,
}

pub(super) fn decode_container_close(decoder: &mut Decoder<'_>) -> Result<ContainerClose> {
    Ok(ContainerClose {
        container_id: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_container_set_content(
    decoder: &mut Decoder<'_>,
) -> Result<ContainerSetContent> {
    let container_id = decoder.read_var_i32()?;
    let state_id = decoder.read_var_i32()?;
    let item_count = decoder.read_len()?;
    if item_count > MAX_CONTAINER_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            item_count,
            MAX_CONTAINER_ITEMS,
        ));
    }
    let mut items = Vec::with_capacity(item_count);
    for _ in 0..item_count {
        items.push(decode_item_stack_summary(decoder)?);
    }
    let carried_item = decode_item_stack_summary(decoder)?;
    Ok(ContainerSetContent {
        container_id,
        state_id,
        items,
        carried_item,
    })
}

pub(super) fn decode_container_set_data(decoder: &mut Decoder<'_>) -> Result<ContainerSetData> {
    Ok(ContainerSetData {
        container_id: decoder.read_var_i32()?,
        id: decoder.read_i16()?,
        value: decoder.read_i16()?,
    })
}

pub(super) fn decode_container_set_slot(decoder: &mut Decoder<'_>) -> Result<ContainerSetSlot> {
    Ok(ContainerSetSlot {
        container_id: decoder.read_var_i32()?,
        state_id: decoder.read_var_i32()?,
        slot: decoder.read_i16()?,
        item: decode_item_stack_summary(decoder)?,
    })
}

pub(super) fn decode_open_screen(decoder: &mut Decoder<'_>) -> Result<OpenScreen> {
    Ok(OpenScreen {
        container_id: decoder.read_var_i32()?,
        menu_type_id: decoder.read_var_i32()?,
        title: decode_component_summary_from_decoder(decoder)?,
    })
}

pub(super) fn decode_set_cursor_item(decoder: &mut Decoder<'_>) -> Result<SetCursorItem> {
    Ok(SetCursorItem {
        item: decode_item_stack_summary(decoder)?,
    })
}

pub(super) fn decode_set_player_inventory(decoder: &mut Decoder<'_>) -> Result<SetPlayerInventory> {
    Ok(SetPlayerInventory {
        slot: decoder.read_var_i32()?,
        item: decode_item_stack_summary(decoder)?,
    })
}

pub(super) fn decode_item_stack_summary(decoder: &mut Decoder<'_>) -> Result<ItemStackSummary> {
    let count = decoder.read_var_i32()?;
    if count <= 0 {
        return Ok(ItemStackSummary::empty());
    }

    let item_id = decoder.read_var_i32()?;
    let component_patch = decode_data_component_patch_summary(decoder)?;
    Ok(ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch,
    })
}

pub(super) fn decode_data_component_patch_summary(
    decoder: &mut Decoder<'_>,
) -> Result<DataComponentPatchSummary> {
    data_components::decode_data_component_patch_summary(decoder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        codec::Encoder,
        ids,
        packets::{decode_play_clientbound, EquipmentSlot, PlayClientbound},
    };

    #[test]
    fn decodes_set_equipment_item_stack_with_supported_component_patch() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_u8(EquipmentSlot::MainHand.ordinal());
        payload.write_var_i32(1);
        payload.write_var_i32(42);
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_var_i32(64);
        payload.write_var_i32(10);
        payload.write_string("minecraft:diamond_sword");

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_EQUIPMENT, &payload.into_inner())
                .unwrap();
        let PlayClientbound::SetEquipment(update) = packet else {
            panic!("expected equipment packet");
        };
        assert_eq!(update.slots.len(), 1);
        assert_eq!(
            update.slots[0].item.component_patch,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![1, 10],
                removed_type_ids: Vec::new(),
                max_stack_size: Some(64),
                ..DataComponentPatchSummary::default()
            }
        );
    }

    #[test]
    fn decodes_item_stack_component_patch_color_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(23);
        payload.write_i16(5);
        payload.write_var_i32(1);
        payload.write_var_i32(99);
        payload.write_var_i32(8);
        payload.write_var_i32(0);

        payload.write_var_i32(2);
        payload.write_var_i32(432);

        payload.write_var_i32(3);
        payload.write_var_i32(431);

        payload.write_var_i32(4);

        payload.write_var_i32(17);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        payload.write_var_i32(2);
        payload.write_i32(0x112233);
        payload.write_i32(0x445566);

        payload.write_var_i32(44);
        payload.write_i32(0x778899);

        payload.write_var_i32(45);
        payload.write_i32(0x010203);

        payload.write_var_i32(51);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_i32(0x0a0b0c);
        payload.write_var_i32(0);
        payload.write_bool(false);

        payload.write_var_i32(68);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_i32(0x102030);
        payload.write_i32(0x405060);
        payload.write_var_i32(1);
        payload.write_i32(0x708090);
        payload.write_bool(true);
        payload.write_bool(false);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_SET_SLOT,
            &payload.into_inner(),
        )
        .unwrap();
        let PlayClientbound::ContainerSetSlot(update) = packet else {
            panic!("expected container set slot packet");
        };
        assert_eq!(
            update.item.component_patch.added_type_ids,
            vec![2, 3, 4, 17, 44, 45, 51, 68]
        );
        assert_eq!(update.item.component_patch.max_damage, Some(432));
        assert_eq!(update.item.component_patch.damage, Some(431));
        assert!(update.item.component_patch.unbreakable);
        assert_eq!(
            update.item.component_patch.custom_model_data_colors,
            vec![0x112233, 0x445566]
        );
        assert_eq!(update.item.component_patch.dyed_color, Some(0x778899));
        assert_eq!(update.item.component_patch.map_color, Some(0x010203));
        assert_eq!(
            update.item.component_patch.potion_custom_color,
            Some(0x0a0b0c)
        );
        assert_eq!(
            update.item.component_patch.firework_explosion_colors,
            vec![0x102030, 0x405060]
        );
    }

    #[test]
    fn decodes_container_and_inventory_item_updates() {
        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_CLOSE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ContainerClose(ContainerClose { container_id: 7 })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_i16(3);
        payload.write_i16(42);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_SET_DATA,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ContainerSetData(ContainerSetData {
                container_id: 7,
                id: 3,
                value: 42,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(2);
        payload.write_bytes(&nbt_string_root("Chest"));
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_OPEN_SCREEN, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::OpenScreen(OpenScreen {
                container_id: 7,
                menu_type_id: 2,
                title: "Chest".to_string(),
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(12);
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i32(64);
        payload.write_var_i32(42);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ContainerSetContent(ContainerSetContent {
                container_id: 7,
                state_id: 12,
                items: vec![
                    ItemStackSummary::empty(),
                    ItemStackSummary {
                        item_id: Some(42),
                        count: 64,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                carried_item: ItemStackSummary::empty(),
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(13);
        payload.write_i16(5);
        payload.write_var_i32(1);
        payload.write_var_i32(99);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_SET_SLOT,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ContainerSetSlot(ContainerSetSlot {
                container_id: 7,
                state_id: 13,
                slot: 5,
                item: ItemStackSummary {
                    item_id: Some(99),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(0);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_CURSOR_ITEM,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetCursorItem(SetCursorItem {
                item: ItemStackSummary::empty(),
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(36);
        payload.write_var_i32(1);
        payload.write_var_i32(42);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPlayerInventory(SetPlayerInventory {
                slot: 36,
                item: ItemStackSummary {
                    item_id: Some(42),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            })
        );
    }

    fn nbt_string_root(text: &str) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload
    }
}
