use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::{
        decode_styled_component_summary_from_decoder, styled_runs_summary_text, StyledTextRun,
    },
    data_component_types::{
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
    },
    item_types::{vanilla_item_type, VANILLA_ITEM_AIR_ID},
    menu_types::vanilla_menu_type,
    packets::data_components,
};

pub use crate::item_component_value_types::{
    FireworkExplosionShapeSummary, ItemRaritySummary, ItemUseAnimationSummary,
    MapPostProcessingSummary, SwingAnimationTypeSummary,
};

const MAX_CONTAINER_ITEMS: usize = 1024;

pub use data_components::{
    decode_persistent_item_predicate_summary, decode_profile_textures_from_properties,
    encode_persistent_item_predicate_summary, AdventureModePredicateDataSummary,
    AdventureModePredicateSummary, AttackRangeSummary, AttributeModifierSummary,
    BannerPatternLayerSummary, BeeOccupantSummary, BlockHolderSetSummary, BlockPredicateSummary,
    BlocksAttacksSummary, ConsumableSummary, ConsumeEffectSummary, CustomModelDataFloats,
    DamageReductionSummary, DamageTypeHolderSetSummary, DataComponentPatchSummary,
    DataComponentPredicateTypeSummary, DebugStickPropertySummary, EntityTypeHolderSetSummary,
    EquippableSummary, FireworkExplosionSummary, GameProfilePropertySummary, InstrumentSummary,
    ItemContainerSlotSummary, ItemDamageFunctionSummary, ItemEnchantmentSummary,
    ItemHolderSetSummary, ItemMapDecorationSummary, ItemStackTemplateSummary, JukeboxSongSummary,
    KineticWeaponConditionSummary, KineticWeaponSummary, LodestoneTargetSummary,
    MobEffectDetailsSummary, MobEffectInstanceSummary, NbtSummaryEntry, NbtSummaryValue,
    PaintingVariantSummary, PartialDataComponentPredicateSummary,
    PersistentDataComponentValueSummary, PersistentItemPredicateSummary, PiercingWeaponSummary,
    PlayerModelTypeSummary, PlayerSkinPatchSummary, ProfileSkinTextureSummary,
    ProfileTextureSummary, ProfileTexturesSummary, ResolvableProfileKindSummary,
    ResolvableProfileSummary, ResourceTextureSummary, SoundEventSummary,
    StatePropertyMatcherSummary, StatePropertyValueMatcherSummary, SuspiciousStewEffectSummary,
    SwingAnimationSummary, ToolRuleSummary, ToolSummary, TrimMaterialSummary, TrimPatternSummary,
    UseEffectsSummary, WrittenBookContentSummary, MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
    MAX_POT_DECORATIONS,
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

    /// Component-summary equivalent of vanilla `ItemStack.hasFoil()` for the data retained by bbb:
    /// Removed patch entries win over retained projections. Otherwise
    /// `minecraft:enchantment_glint_override` wins when present, and decoded
    /// enchantments provide the fallback.
    pub fn has_foil(&self) -> bool {
        let patch = &self.component_patch;
        if !patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID)
        {
            if let Some(override_value) = patch.enchantment_glint_override {
                return override_value;
            }
        }
        !patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
            && !patch.enchantments.is_empty()
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
    /// Styled-run projection of `title` (same component decode, style kept);
    /// `title` is its plain-text concatenation.
    #[serde(default)]
    pub title_styled: Vec<StyledTextRun>,
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
    let packet = ContainerClose {
        container_id: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after container close packet".to_string(),
        ));
    }
    Ok(packet)
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
    let packet = ContainerSetContent {
        container_id,
        state_id,
        items,
        carried_item,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after container set content packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_container_set_data(decoder: &mut Decoder<'_>) -> Result<ContainerSetData> {
    let packet = ContainerSetData {
        container_id: decoder.read_var_i32()?,
        id: decoder.read_i16()?,
        value: decoder.read_i16()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after container set data packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_container_set_slot(decoder: &mut Decoder<'_>) -> Result<ContainerSetSlot> {
    let packet = ContainerSetSlot {
        container_id: decoder.read_var_i32()?,
        state_id: decoder.read_var_i32()?,
        slot: decoder.read_i16()?,
        item: decode_item_stack_summary(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after container set slot packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_open_screen(decoder: &mut Decoder<'_>) -> Result<OpenScreen> {
    let container_id = decoder.read_var_i32()?;
    let menu_type_id = decoder.read_var_i32()?;
    if vanilla_menu_type(menu_type_id).is_none() {
        return Err(ProtocolError::InvalidData(format!(
            "invalid vanilla menu type id {menu_type_id}"
        )));
    }
    let title_styled = decode_styled_component_summary_from_decoder(decoder)?;
    let packet = OpenScreen {
        container_id,
        menu_type_id,
        title: styled_runs_summary_text(&title_styled),
        title_styled,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after open screen packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_set_cursor_item(decoder: &mut Decoder<'_>) -> Result<SetCursorItem> {
    let packet = SetCursorItem {
        item: decode_item_stack_summary(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set cursor item packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_set_player_inventory(decoder: &mut Decoder<'_>) -> Result<SetPlayerInventory> {
    let packet = SetPlayerInventory {
        slot: decoder.read_var_i32()?,
        item: decode_item_stack_summary(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set player inventory packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_item_stack_summary(decoder: &mut Decoder<'_>) -> Result<ItemStackSummary> {
    let count = decoder.read_var_i32()?;
    if count <= 0 {
        return Ok(ItemStackSummary::empty());
    }

    let item_id = decode_item_holder_id(decoder)?;
    let component_patch = decode_data_component_patch_summary(decoder)?;
    Ok(ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch,
    })
}

pub(super) fn decode_item_holder_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    validate_item_holder_id(decoder.read_var_i32()?)
}

pub(super) fn decode_non_empty_item_holder_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let item_id = decode_item_holder_id(decoder)?;
    if item_id == VANILLA_ITEM_AIR_ID {
        return Err(ProtocolError::InvalidData(
            "item stack template holder must not be minecraft:air".to_string(),
        ));
    }
    Ok(item_id)
}

pub(super) fn validate_item_holder_id(item_id: i32) -> Result<i32> {
    if vanilla_item_type(item_id).is_none() {
        return Err(ProtocolError::InvalidData(format!(
            "invalid vanilla item holder id {item_id}"
        )));
    }
    Ok(item_id)
}

pub fn decode_data_component_patch_summary(
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
    fn item_stack_has_foil_uses_glint_override_before_enchantments() {
        let mut stack = ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        assert!(!stack.has_foil());

        stack
            .component_patch
            .enchantments
            .push(ItemEnchantmentSummary {
                holder_id: 7,
                level: 3,
            });
        assert!(stack.has_foil());

        stack.component_patch.enchantment_glint_override = Some(false);
        assert!(!stack.has_foil());

        stack
            .component_patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID);
        assert!(stack.has_foil());

        stack
            .component_patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
        assert!(!stack.has_foil());

        stack.component_patch.removed_type_ids.clear();
        stack.component_patch.enchantments.clear();
        stack.component_patch.enchantment_glint_override = Some(true);
        assert!(stack.has_foil());
    }

    #[test]
    fn item_stack_holder_rejects_ids_outside_the_builtin_registry() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        let invalid_item_id = crate::item_types::VANILLA_ITEM_TYPES.len() as i32;
        payload.write_var_i32(invalid_item_id);
        let error =
            decode_item_stack_summary(&mut Decoder::new(&payload.into_inner())).unwrap_err();
        assert!(error
            .to_string()
            .contains(&format!("invalid vanilla item holder id {invalid_item_id}")));
    }

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
                item_model: Some("minecraft:diamond_sword".to_string()),
                ..DataComponentPatchSummary::default()
            }
        );
    }

    #[test]
    fn item_stack_packets_share_component_rich_patch_decode() {
        let stack = component_rich_stack();

        let mut content = Encoder::new();
        content.write_var_i32(7);
        content.write_var_i32(12);
        content.write_var_i32(1);
        encode_item_stack_fixture(&mut content, &stack);
        content.write_var_i32(0);
        let PlayClientbound::ContainerSetContent(content) = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT,
            &content.into_inner(),
        )
        .unwrap() else {
            panic!("expected container content packet");
        };
        assert_eq!(content.items, [stack.clone()]);

        let mut slot = Encoder::new();
        slot.write_var_i32(7);
        slot.write_var_i32(13);
        slot.write_i16(5);
        encode_item_stack_fixture(&mut slot, &stack);
        let PlayClientbound::ContainerSetSlot(slot) = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_SET_SLOT,
            &slot.into_inner(),
        )
        .unwrap() else {
            panic!("expected container slot packet");
        };
        assert_eq!(slot.item, stack);

        let mut cursor = Encoder::new();
        encode_item_stack_fixture(&mut cursor, &stack);
        let PlayClientbound::SetCursorItem(cursor) =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_CURSOR_ITEM, &cursor.into_inner())
                .unwrap()
        else {
            panic!("expected cursor item packet");
        };
        assert_eq!(cursor.item, stack);

        let mut inventory = Encoder::new();
        inventory.write_var_i32(5);
        encode_item_stack_fixture(&mut inventory, &stack);
        let PlayClientbound::SetPlayerInventory(inventory) = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY,
            &inventory.into_inner(),
        )
        .unwrap() else {
            panic!("expected player inventory packet");
        };
        assert_eq!(inventory.item, stack);

        let mut equipment = Encoder::new();
        equipment.write_var_i32(123);
        equipment.write_u8(EquipmentSlot::Head.ordinal());
        encode_item_stack_fixture(&mut equipment, &stack);
        let PlayClientbound::SetEquipment(equipment) = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_EQUIPMENT,
            &equipment.into_inner(),
        )
        .unwrap() else {
            panic!("expected equipment packet");
        };
        assert_eq!(equipment.slots[0].item, stack);
    }

    fn component_rich_stack() -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(crate::item_types::VANILLA_ITEM_STONE_ID),
            count: 3,
            component_patch: DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                    crate::data_component_types::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
                ],
                max_stack_size: Some(16),
                lock_predicate: Some(PersistentItemPredicateSummary {
                    count_min: Some(1),
                    count_max: Some(3),
                    ..PersistentItemPredicateSummary::default()
                }),
                ..DataComponentPatchSummary::default()
            },
        }
    }

    fn encode_item_stack_fixture(out: &mut Encoder, stack: &ItemStackSummary) {
        out.write_var_i32(stack.count);
        out.write_var_i32(stack.item_id.unwrap());
        out.write_var_i32(2);
        out.write_var_i32(0);
        out.write_var_i32(
            crate::data_component_types::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        );
        out.write_var_i32(stack.component_patch.max_stack_size.unwrap());
        out.write_var_i32(crate::data_component_types::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID);
        let lock = encode_persistent_item_predicate_summary(
            stack.component_patch.lock_predicate.as_ref().unwrap(),
        )
        .unwrap();
        out.write_bytes(
            &crate::nbt_summary::encode_nbt_summary_bounded(
                Some(&lock),
                i32::MAX as usize,
                crate::nbt_summary::MAX_NBT_DEPTH,
            )
            .unwrap(),
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
        let mut trailing = Encoder::new();
        trailing.write_var_i32(7);
        trailing.write_u8(0);
        let error = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_CLOSE,
            &trailing.into_inner(),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after container close packet"));

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
        let mut trailing = Encoder::new();
        trailing.write_var_i32(7);
        trailing.write_i16(3);
        trailing.write_i16(42);
        trailing.write_u8(0);
        let error = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_SET_DATA,
            &trailing.into_inner(),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after container set data packet"));

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
                title_styled: vec![StyledTextRun {
                    text: "Chest".to_string(),
                    style: Default::default(),
                }],
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
        let mut bytes = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT, &bytes).unwrap();
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
        bytes.push(0);
        let error = decode_play_clientbound(ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT, &bytes)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after container set content packet"));

        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(13);
        payload.write_i16(5);
        payload.write_var_i32(1);
        payload.write_var_i32(99);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        let mut bytes = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_CONTAINER_SET_SLOT, &bytes).unwrap();
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
        bytes.push(0);
        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_CONTAINER_SET_SLOT, &bytes).unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after container set slot packet"));

        let mut bytes = Encoder::new();
        bytes.write_var_i32(0);
        let mut bytes = bytes.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_CURSOR_ITEM, &bytes).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetCursorItem(SetCursorItem {
                item: ItemStackSummary::empty(),
            })
        );
        bytes.push(0);
        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_CURSOR_ITEM, &bytes).unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after set cursor item packet"));

        let mut payload = Encoder::new();
        payload.write_var_i32(36);
        payload.write_var_i32(1);
        payload.write_var_i32(42);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        let mut bytes = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY, &bytes).unwrap();
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
        bytes.push(0);
        let error = decode_play_clientbound(ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY, &bytes)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after set player inventory packet"));
    }

    #[test]
    fn open_screen_rejects_unknown_menu_type_and_trailing_bytes() {
        let mut trailing = Encoder::new();
        trailing.write_var_i32(7);
        trailing.write_var_i32(2);
        trailing.write_bytes(&nbt_string_root("Chest"));
        trailing.write_u8(0);
        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_OPEN_SCREEN, &trailing.into_inner())
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after open screen packet"));

        let mut unknown = Encoder::new();
        unknown.write_var_i32(7);
        unknown.write_var_i32(25);
        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_OPEN_SCREEN, &unknown.into_inner())
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("invalid vanilla menu type id 25"));
    }

    fn nbt_string_root(text: &str) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload
    }
}
