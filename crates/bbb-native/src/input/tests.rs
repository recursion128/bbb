use super::*;
use bbb_item_model::NativeItemRuntime;
use bbb_net::OutboundChatCommand;
use bbb_pack::{EquipmentAssetCatalog, ItemRegistryCatalog};
use bbb_protocol::block_entity_types::{
    VANILLA_BLOCK_ENTITY_TYPES, VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
    VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID, VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
    VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
    VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
};
use bbb_protocol::builtin_enum_component_values::{
    VANILLA_AXOLOTL_VARIANT_BLUE_ID, VANILLA_DYE_COLOR_BLACK_ID, VANILLA_DYE_COLOR_CYAN_ID,
    VANILLA_DYE_COLOR_LIGHT_BLUE_ID, VANILLA_DYE_COLOR_ORANGE_ID, VANILLA_DYE_COLOR_RED_ID,
    VANILLA_DYE_COLOR_VALUES, VANILLA_DYE_COLOR_WHITE_ID, VANILLA_FOX_VARIANT_SNOW_ID,
    VANILLA_HORSE_VARIANT_DARK_BROWN_ID, VANILLA_LLAMA_VARIANT_GRAY_ID,
    VANILLA_MOOSHROOM_VARIANT_BROWN_ID, VANILLA_PARROT_VARIANT_GRAY_ID,
    VANILLA_RABBIT_VARIANT_EVIL_ID, VANILLA_SALMON_VARIANT_LARGE_ID,
};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
    VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_ARMADILLO_ID,
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_ARROW_ID,
    VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_BEE_ID,
    VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
    VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_BREEZE_ID,
    VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID,
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID,
    VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
    VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_CREEPER_ID,
    VANILLA_ENTITY_TYPE_DOLPHIN_ID, VANILLA_ENTITY_TYPE_DONKEY_ID,
    VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID, VANILLA_ENTITY_TYPE_DROWNED_ID,
    VANILLA_ENTITY_TYPE_EGG_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID, VANILLA_ENTITY_TYPE_ENDERMITE_ID,
    VANILLA_ENTITY_TYPE_ENDER_PEARL_ID, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
    VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID, VANILLA_ENTITY_TYPE_EVOKER_ID,
    VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
    VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_FIREBALL_ID,
    VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID, VANILLA_ENTITY_TYPE_FOX_ID,
    VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
    VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_GIANT_ID,
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
    VANILLA_ENTITY_TYPE_GOAT_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
    VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
    VANILLA_ENTITY_TYPE_HUSK_ID, VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
    VANILLA_ENTITY_TYPE_INTERACTION_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_LINGERING_POTION_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
    VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID, VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
    VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_MOOSHROOM_ID, VANILLA_ENTITY_TYPE_MULE_ID,
    VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_OCELOT_ID,
    VANILLA_ENTITY_TYPE_PAINTING_ID, VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_PARCHED_ID,
    VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_PIG_ID,
    VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
    VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
    VANILLA_ENTITY_TYPE_RABBIT_ID, VANILLA_ENTITY_TYPE_RAVAGER_ID, VANILLA_ENTITY_TYPE_SALMON_ID,
    VANILLA_ENTITY_TYPE_SHEEP_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
    VANILLA_ENTITY_TYPE_SHULKER_ID, VANILLA_ENTITY_TYPE_SILVERFISH_ID,
    VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, VANILLA_ENTITY_TYPE_SKELETON_ID,
    VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
    VANILLA_ENTITY_TYPE_SNOWBALL_ID, VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
    VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
    VANILLA_ENTITY_TYPE_SPIDER_ID, VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
    VANILLA_ENTITY_TYPE_SQUID_ID, VANILLA_ENTITY_TYPE_STRAY_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
    VANILLA_ENTITY_TYPE_TADPOLE_ID, VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID,
    VANILLA_ENTITY_TYPE_TNT_ID, VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, VANILLA_ENTITY_TYPE_TRIDENT_ID,
    VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
    VANILLA_ENTITY_TYPE_VEX_ID, VANILLA_ENTITY_TYPE_VINDICATOR_ID,
    VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
    VANILLA_ENTITY_TYPE_WITCH_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
    VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
    VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_CHEST_SLOT;
use bbb_protocol::item_types::VANILLA_ITEM_ELYTRA_ID;
use bbb_protocol::menu_types::{
    VANILLA_MENU_TYPE_ANVIL_ID, VANILLA_MENU_TYPE_CRAFTING_ID, VANILLA_MENU_TYPE_FURNACE_ID,
    VANILLA_MENU_TYPE_GENERIC_9X1_ID, VANILLA_MENU_TYPE_GENERIC_9X3_ID,
    VANILLA_MENU_TYPE_MERCHANT_ID,
};
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_POISON_ID, VANILLA_MOB_EFFECT_REGENERATION_ID, VANILLA_MOB_EFFECT_SPEED_ID,
    VANILLA_MOB_EFFECT_TYPES,
};
use bbb_protocol::packets::AwardStats;
use bbb_protocol::packets::BlockEntityData;
use bbb_protocol::packets::{
    AddEntity, AdvancementDisplaySummary, AdvancementFrameType, AdvancementIconSummary,
    AdvancementSummary, BlockEntityTagQuery, BlockPos as ProtocolBlockPos,
    BlockUpdate as ProtocolBlockUpdate, CommandArgumentParser, CommandNode, CommandNodeType,
    CommandSuggestion, CommandSuggestionRequest, CommandSuggestions, Commands,
    CommonPlayerSpawnInfo, ContainerClick, ContainerCloseRequest, ContainerInput,
    ContainerSetContent as ProtocolContainerSetContent, CustomChatCompletions,
    CustomChatCompletionsAction, DataComponentPatchSummary, DeleteChat, DialogHolder,
    EntityDataEnumSerializer, EntityDataRegistryHolder, EntityDataRegistryIdentity,
    EntityDataRegistryReference, EntityDataRegistryRevision, EntityDataRegistrySource,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    EntityEvent as ProtocolEntityEvent, EntityTagQuery, EquipmentSlot, EquipmentSlotUpdate,
    FilterMask, FilterMaskKind, FireworkExplosionShapeSummary, FireworkExplosionSummary,
    GameEvent as ProtocolGameEvent, GameProfile, HashedComponentPatch, HashedItemStack,
    HashedStack, ItemRaritySummary, ItemStackSummary as ProtocolItemStackSummary, JigsawGenerate,
    JigsawJoint, LastSeenMessagesUpdate, MessageSignature, NbtSummaryEntry, OpenBook,
    OpenScreen as ProtocolOpenScreen, OpenSignEditor, PackedMessageSignature, PaddleBoat,
    PlayLogin, PlayerAbilities, PlayerAbilitiesCommand, PlayerAction, PlayerChat, PlayerCommand,
    PlayerHealth, PlayerInfoAction, PlayerInfoEntry, PlayerInfoUpdate, PlayerSkinPatchData,
    ProjectilePower as ProtocolProjectilePower, RemoveEntities as ProtocolRemoveEntities,
    RenameItem, ResolvableProfileData, ResolvableProfileKind, SeenAdvancements, SelectBundleItem,
    SetCommandMinecart, SetCursorItem as ProtocolSetCursorItem,
    SetEntityData as ProtocolSetEntityData, SetEntityLink, SetEquipment, SetJigsawBlock,
    SetPassengers, SetPlayerInventory as ProtocolSetPlayerInventory, SetStructureBlock,
    SetTestBlock, ShowDialog as ProtocolShowDialog, SignUpdate, SignedMessageBody, StatUpdate,
    StructureBlockMirror, StructureBlockMode, StructureBlockRotation, StructureBlockUpdateKind,
    TagQuery, TestBlockMode, TestInstanceBlockAction, TestInstanceBlockActionKind,
    TestInstanceBlockRotation, TestInstanceBlockStatus, UpdateAdvancements, Vec3d as ProtocolVec3d,
    Vec3i, WrittenBookContentSummary, VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
};
use bbb_protocol::packets::{ChatTypeBound, ChatTypeHolder};
use bbb_protocol::recipe_wire_types::{
    VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID, VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
    VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID, VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
};
use bbb_protocol::stat_types::{
    VANILLA_STAT_TYPE_ENTITY_KILLED_ID, VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
};
use bbb_protocol::tropical_fish_patterns::VANILLA_TROPICAL_FISH_PATTERN_STRIPEY_PACKED_ID;
use bbb_protocol::{
    RichTextComponent, MC_BUILD_TIME, MC_DATA_PACK_FORMAT, MC_DATA_VERSION, MC_DATA_VERSION_SERIES,
    MC_RESOURCE_PACK_FORMAT, MC_STABLE, MC_VERSION, PROTOCOL_VERSION,
};
use bbb_world::{
    vanilla_entity_metadata_baby_accessor_for_entity_type,
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_painting_variant_accessor_for_entity_type,
    vanilla_entity_metadata_sniffer_drop_seed_at_tick_accessor_for_entity_type,
    vanilla_entity_metadata_sniffer_state_accessor_for_entity_type,
    VANILLA_SNIFFER_STATE_DIGGING_ID,
};
use bbb_world::{
    BlockEntityRecord, BlockPos, BlockStateRegistry, ChatMessageKind, ChunkColumn, ChunkPos,
    ChunkSection, ChunkState, CommandBlockEntityDataState, ItemEquipmentSlot,
    JigsawBlockEntityDataState, LightData, LocalPlayerPoseState, PaletteDomain, PaletteKind,
    PalettedContainerData, RegistryPacketEntry, SignBlockEntityTextState,
    StructureBlockEntityDataState, StructureBlockSizeState, TestBlockEntityDataState,
    TestInstanceBlockEntityDataState, TestInstanceBlockRunStatus, TestInstanceBlockSizeState,
    WorldStore,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const TEST_SADDLE_ITEM_ID: i32 = 8_903;

fn entity_metadata_accessors() -> bbb_world::VanillaEntityMetadataAccessors {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert_eq!(accessors.shared_flags_default, 0);
    assert!(!accessors.custom_name_visible_default);
    assert!(!accessors.silent_default);
    assert!(!accessors.no_gravity_default);
    assert_eq!(accessors.ticks_frozen_default, 0);
    accessors
}

fn entity_pose_data(pose_id: i32) -> ProtocolEntityDataValue {
    let accessors = entity_metadata_accessors();
    ProtocolEntityDataValue {
        data_id: accessors.pose_data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
        value: EntityDataValueKind::Pose(pose_id),
    }
}

fn entity_invisible_data() -> ProtocolEntityDataValue {
    let accessors = entity_metadata_accessors();
    ProtocolEntityDataValue {
        data_id: accessors.shared_flags_data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(accessors.invisible_mask),
    }
}

fn tamable_animal_flags_data(
    entity_type_id: i32,
    tame: bool,
    sitting: bool,
) -> ProtocolEntityDataValue {
    let accessors =
        vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id)
            .expect("generated TamableAnimal metadata accessors must resolve");
    let flags = if tame { accessors.tame_mask } else { 0 }
        | if sitting { accessors.sitting_mask } else { 0 };
    ProtocolEntityDataValue {
        data_id: accessors.flags_data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(flags),
    }
}

fn shulker_metadata_accessors() -> bbb_world::VanillaShulkerMetadataAccessors {
    vanilla_entity_metadata_shulker_accessors_for_entity_type(VANILLA_ENTITY_TYPE_SHULKER_ID)
        .expect("generated Shulker metadata accessors must resolve")
}

fn sheep_wool_data(color_id: u8, sheared: bool) -> ProtocolEntityDataValue {
    let (data_id, _, sheared_mask, color_mask) =
        vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(VANILLA_ENTITY_TYPE_SHEEP_ID)
            .expect("generated Sheep wool accessor must resolve");
    assert_eq!(color_id & !color_mask, 0, "invalid Sheep wool color id");
    let value = color_id | if sheared { sheared_mask as u8 } else { 0 };
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(value as i8),
    }
}

fn rabbit_type_data(variant_id: i32) -> ProtocolEntityDataValue {
    let (data_id, _) =
        vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(VANILLA_ENTITY_TYPE_RABBIT_ID)
            .expect("generated Rabbit variant accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(variant_id),
    }
}

fn pufferfish_puff_state_data(puff_state: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
    )
    .expect("generated Pufferfish puff-state accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(puff_state),
    }
}

fn slime_size_data(entity_type_id: i32, size: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)
        .expect("generated Slime size accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(size),
    }
}

fn phantom_size_data(size: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_phantom_size_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PHANTOM_ID,
    )
    .expect("generated Phantom size accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(size),
    }
}

fn salmon_variant_data(variant: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_SALMON_ID,
    )
    .expect("generated Salmon variant accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(variant),
    }
}

fn tropical_fish_variant_data(packed: i32) -> ProtocolEntityDataValue {
    let accessors = vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
    )
    .expect("generated TropicalFish packed-variant accessor must resolve");
    ProtocolEntityDataValue {
        data_id: accessors.variant_data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(packed),
    }
}

fn parrot_variant_data(variant: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_parrot_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PARROT_ID,
    )
    .expect("generated Parrot variant accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(variant),
    }
}

fn axolotl_variant_data(variant: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    )
    .expect("generated Axolotl variant accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(variant),
    }
}

fn axolotl_from_bucket_data(from_bucket: bool) -> ProtocolEntityDataValue {
    let (data_id, default) = vanilla_entity_metadata_axolotl_from_bucket_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    )
    .expect("generated Axolotl from-bucket accessor must resolve");
    assert!(!default);
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(from_bucket),
    }
}

fn abstract_fish_from_bucket_data(
    entity_type_id: i32,
    from_bucket: bool,
) -> ProtocolEntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_abstract_fish_from_bucket_accessor_for_entity_type(entity_type_id)
            .expect("generated AbstractFish from-bucket accessor must resolve");
    assert!(!default);
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(from_bucket),
    }
}

fn ocelot_trusting_data(trusting: bool) -> ProtocolEntityDataValue {
    let (data_id, default) = vanilla_entity_metadata_ocelot_trusting_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_OCELOT_ID,
    )
    .expect("generated Ocelot trusting accessor must resolve");
    assert!(!default);
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(trusting),
    }
}

fn mushroom_cow_variant_data(variant: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
    )
    .expect("generated MushroomCow variant accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(variant),
    }
}

fn bogged_sheared_data(sheared: bool) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_BOGGED_ID,
    )
    .expect("generated Bogged sheared accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(sheared),
    }
}

fn allay_dancing_data(dancing: bool) -> ProtocolEntityDataValue {
    let (data_id, _) = bbb_world::vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ALLAY_ID,
    )
    .expect("generated Allay dancing accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(dancing),
    }
}

fn allay_can_duplicate_data(can_duplicate: bool) -> ProtocolEntityDataValue {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ALLAY_ID,
        )
        .expect("generated Allay can-duplicate accessor must resolve");
    assert!(default);
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(can_duplicate),
    }
}

fn sniffer_state_data(state_id: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_SNIFFER_ID,
    )
    .expect("generated Sniffer state accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_SNIFFER_STATE_ID,
        value: EntityDataValueKind::EnumId {
            serializer: EntityDataEnumSerializer::SnifferState,
            id: state_id,
        },
    }
}

fn sniffer_drop_seed_at_tick_data(
    world: &WorldStore,
    entity_id: i32,
    tick: i32,
) -> ProtocolEntityDataValue {
    let entity_type_id = world
        .entity_type_id(entity_id)
        .expect("Sniffer fixture entity must exist before metadata is constructed");
    let (data_id, default) =
        vanilla_entity_metadata_sniffer_drop_seed_at_tick_accessor_for_entity_type(entity_type_id)
            .expect("generated Sniffer drop-seed tick accessor must resolve");
    assert_eq!(default, 0);
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(tick),
    }
}

fn painting_variant_data(
    world: &WorldStore,
    entity_id: i32,
    registry_id: i32,
) -> ProtocolEntityDataValue {
    let entity_type_id = world
        .entity_type_id(entity_id)
        .expect("Painting fixture entity must exist before metadata is constructed");
    let data_id = vanilla_entity_metadata_painting_variant_accessor_for_entity_type(entity_type_id)
        .expect("generated Painting variant accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
        value: EntityDataValueKind::PaintingVariant(
            bbb_protocol::packets::PaintingVariantData {
                registry: Some(EntityDataRegistryReference::unresolved(registry_id)),
                direct: None,
            },
        ),
    }
}

fn item_frame_metadata_accessors(
    world: &WorldStore,
    entity_id: i32,
) -> bbb_world::VanillaItemFrameMetadataAccessors {
    let entity_type_id = world
        .entity_type_id(entity_id)
        .expect("ItemFrame fixture entity must exist before metadata is constructed");
    let accessors = vanilla_entity_metadata_item_frame_accessors_for_entity_type(entity_type_id)
        .expect("generated ItemFrame metadata accessors must resolve");
    assert_eq!(accessors.direction_default, 3);
    assert_eq!(accessors.rotation_default, 0);
    accessors
}

fn end_crystal_metadata_accessors(
    world: &WorldStore,
    entity_id: i32,
) -> bbb_world::VanillaEndCrystalMetadataAccessors {
    let entity_type_id = world
        .entity_type_id(entity_id)
        .expect("EndCrystal fixture entity must exist before metadata is constructed");
    let accessors = vanilla_entity_metadata_end_crystal_accessors_for_entity_type(entity_type_id)
        .expect("generated EndCrystal metadata accessors must resolve");
    assert!(accessors.show_bottom_default);
    accessors
}

fn goat_metadata_accessors() -> bbb_world::VanillaGoatMetadataAccessors {
    vanilla_entity_metadata_goat_accessors_for_entity_type(VANILLA_ENTITY_TYPE_GOAT_ID)
        .expect("generated Goat metadata accessors must resolve")
}

fn turtle_metadata_accessors(
    world: &WorldStore,
    entity_id: i32,
) -> bbb_world::VanillaTurtleMetadataAccessors {
    let entity_type_id = world
        .entity_type_id(entity_id)
        .expect("Turtle fixture entity must exist before metadata is constructed");
    let accessors = vanilla_entity_metadata_turtle_accessors_for_entity_type(entity_type_id)
        .expect("generated Turtle metadata accessors must resolve");
    assert!(!accessors.has_egg_default);
    assert!(!accessors.laying_egg_default);
    accessors
}

fn farm_animal_variant_metadata_accessors(
    entity_type_id: i32,
) -> bbb_world::VanillaFarmAnimalVariantMetadataAccessors {
    vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(entity_type_id)
        .expect("generated farm-animal variant metadata accessors must resolve")
}

fn baby_data(entity_type_id: i32, is_baby: bool) -> ProtocolEntityDataValue {
    let (data_id, default) = vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
        .expect("generated nearest baby accessor must resolve");
    assert!(!default, "vanilla AgeableMob baby default must be false");
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(is_baby),
    }
}

fn age_locked_data(
    world: &WorldStore,
    entity_id: i32,
    age_locked: bool,
) -> ProtocolEntityDataValue {
    let entity_type_id = world
        .entity_type_id(entity_id)
        .expect("age-locked fixture entity must exist before metadata is constructed");
    let (data_id, default) =
        vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)
            .expect("generated nearest age-locked accessor must resolve");
    assert!(!default, "vanilla age-locked default must be false");
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(age_locked),
    }
}

fn mob_flags_data(
    world: &WorldStore,
    entity_id: i32,
    no_ai: bool,
    left_handed: bool,
) -> ProtocolEntityDataValue {
    let entity_type_id = world
        .entity_type_id(entity_id)
        .expect("Mob fixture entity must exist before metadata is constructed");
    let accessors = vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)
        .expect("generated inherited Mob metadata accessors must resolve");
    assert_eq!(accessors.flags_default, 0);
    let flags = if no_ai { accessors.no_ai_mask } else { 0 }
        | if left_handed {
            accessors.left_handed_mask
        } else {
            0
        };
    ProtocolEntityDataValue {
        data_id: accessors.flags_data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(flags),
    }
}

fn bat_flags_data(world: &WorldStore, entity_id: i32) -> ProtocolEntityDataValue {
    let entity_type_id = world
        .entity_type_id(entity_id)
        .expect("Bat fixture entity must exist before metadata is constructed");
    let (data_id, default, resting_mask) =
        vanilla_entity_metadata_bat_flags_accessor_for_entity_type(entity_type_id)
            .expect("generated Bat flags accessor must resolve");
    assert_eq!(default, 0);
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(resting_mask),
    }
}

fn wither_invulnerability_data(
    world: &WorldStore,
    entity_id: i32,
    ticks: i32,
) -> ProtocolEntityDataValue {
    let entity_type_id = world
        .entity_type_id(entity_id)
        .expect("Wither fixture entity must exist before metadata is constructed");
    let (data_id, default) =
        vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(entity_type_id)
            .expect("generated WitherBoss invulnerability accessor must resolve");
    assert_eq!(default, 0);
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(ticks),
    }
}

fn sniffer_baby_data(is_baby: bool) -> ProtocolEntityDataValue {
    let (data_id, _) =
        vanilla_entity_metadata_baby_accessor_for_entity_type(VANILLA_ENTITY_TYPE_SNIFFER_ID)
            .expect("generated Sniffer inherited baby accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(is_baby),
    }
}

fn armadillo_state_data(state_id: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ARMADILLO_ID,
    )
    .expect("generated Armadillo state accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ARMADILLO_STATE_ID,
        value: EntityDataValueKind::EnumId {
            serializer: EntityDataEnumSerializer::ArmadilloState,
            id: state_id,
        },
    }
}

fn iron_golem_flags_data(player_created: bool) -> ProtocolEntityDataValue {
    let (data_id, default, player_created_mask) =
        vanilla_entity_metadata_iron_golem_flags_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        )
        .expect("generated IronGolem flags accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(if player_created {
            default | player_created_mask
        } else {
            default & !player_created_mask
        }),
    }
}

fn snow_golem_pumpkin_accessor() -> (u8, i8, i8) {
    vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
    )
    .expect("generated SnowGolem pumpkin accessor must resolve")
}

fn abstract_horse_metadata_accessors(
    entity_type_id: i32,
) -> bbb_world::VanillaAbstractHorseMetadataAccessors {
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)
        .expect("generated AbstractHorse metadata accessors must resolve")
}

fn horse_variant_data(entity_type_id: i32, packed_variant: i32) -> ProtocolEntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_horse_variant_accessor_for_entity_type(entity_type_id)
            .expect("generated Horse variant accessor must resolve");
    assert_eq!(default, 0);
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(packed_variant),
    }
}

fn chested_horse_chest_data(entity_type_id: i32, has_chest: bool) -> ProtocolEntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type(entity_type_id)
            .expect("generated AbstractChestedHorse chest accessor must resolve");
    assert!(!default);
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(has_chest),
    }
}

fn llama_metadata_accessors(entity_type_id: i32) -> bbb_world::VanillaLlamaMetadataAccessors {
    let accessors = vanilla_entity_metadata_llama_accessors_for_entity_type(entity_type_id)
        .expect("generated Llama metadata accessors must resolve");
    assert_eq!(accessors.strength_default, 0);
    assert_eq!(accessors.variant_default, 0);
    accessors
}

fn creeper_metadata_accessors() -> bbb_world::VanillaCreeperMetadataAccessors {
    vanilla_entity_metadata_creeper_accessors_for_entity_type(VANILLA_ENTITY_TYPE_CREEPER_ID)
        .expect("generated Creeper metadata accessors must resolve")
}

fn zombification_immunity_data_id(entity_type_id: i32) -> u8 {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated zombification-immunity accessor");
    assert!(!default);
    data_id
}

fn strider_suffocating_data_id() -> u8 {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
        )
        .expect("generated Strider suffocating accessor");
    assert!(!default);
    data_id
}

fn strider_boost_time_data(boost_time: i32) -> ProtocolEntityDataValue {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_strider_boost_time_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
        )
        .expect("generated Strider boost-time accessor");
    assert_eq!(default, 0);
    ProtocolEntityDataValue {
        data_id,
        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(boost_time),
    }
}

fn witch_using_item_data(using_item: bool) -> ProtocolEntityDataValue {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_witch_using_item_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_WITCH_ID,
        )
        .expect("generated Witch using-item accessor");
    assert!(!default);
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(using_item),
    }
}

fn glow_squid_dark_ticks_data_id() -> u8 {
    let (data_id, default) =
        vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
        )
        .expect("generated GlowSquid dark-ticks accessor");
    assert_eq!(default, 0);
    data_id
}

fn wither_skull_dangerous_data_id() -> u8 {
    let (data_id, default) =
        vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        )
        .expect("generated WitherSkull dangerous accessor");
    assert!(!default);
    data_id
}

fn piglin_baby_data_id() -> u8 {
    let (data_id, default) = bbb_world::vanilla_entity_metadata_baby_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
    )
    .expect("generated Piglin baby accessor");
    assert!(!default);
    data_id
}

fn spellcaster_casting_data_id(entity_type_id: i32) -> u8 {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated SpellcasterIllager casting accessor");
    assert_eq!(default, 0);
    data_id
}

fn raider_celebrating_data_id(entity_type_id: i32) -> u8 {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated Raider celebrating accessor");
    assert!(!default);
    data_id
}

fn creaking_home_pos_data_id() -> u8 {
    bbb_world::vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_CREAKING_ID,
    )
    .expect("generated Creaking home-position accessor")
}

fn handle_key_input_without_world(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    physical_key: PhysicalKey,
    state: ElementState,
) {
    let mut world = WorldStore::new();
    handle_key_input(
        input,
        counters,
        &mut world,
        net_commands,
        physical_key,
        state,
    );
}

fn world_with_debug_player(reduced_debug_info: bool) -> WorldStore {
    world_with_debug_player_in_game_mode(reduced_debug_info, 0, -1)
}

fn world_with_debug_player_in_game_mode(
    reduced_debug_info: bool,
    game_type: i8,
    previous_game_type: i8,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 12345,
            game_type,
            previous_game_type,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    world
}

fn grant_debug_recreate_nbt_permission(world: &mut WorldStore) {
    let player_id = world
        .local_player_id()
        .expect("debug recreate test world has a local player");
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: player_id,
        event_id: 26,
    }));
    assert!(world.local_player_has_gamemaster_permission());
}

fn handle_text_input_without_world(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    text: &str,
) {
    let mut world = WorldStore::new();
    handle_text_input(input, counters, &mut world, net_commands, text);
}

fn world_with_local_player_id(player_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    });
    world
}

#[derive(Default)]
struct MockDebugClipboard {
    text: Option<String>,
    accepts_text: bool,
}

impl MockDebugClipboard {
    fn accepting() -> Self {
        Self {
            text: None,
            accepts_text: true,
        }
    }

    fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            accepts_text: true,
        }
    }
}

impl DebugClipboard for MockDebugClipboard {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
        if self.accepts_text {
            self.text = Some(text.to_string());
            true
        } else {
            false
        }
    }

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        self.text.clone()
    }
}

fn assert_unpickable_debug_entity_copy(
    input: &mut ClientInputState,
    world: &WorldStore,
    clipboard: &MockDebugClipboard,
    entity_id: i32,
    expected_direct_command: &str,
) {
    assert_eq!(clipboard.text, None);
    assert_eq!(
        debug_copy_recreate_entity_command(world, None, entity_id, true)
            .expect("direct recreate command remains available")
            .command,
        expected_direct_command
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    assert!(world.client_chat().messages.is_empty());
}

struct VariableDebugOptionsSearchTextMeasurer;

impl DebugOptionsSearchTextMeasurer for VariableDebugOptionsSearchTextMeasurer {
    fn debug_options_search_display_start_for_width(
        &self,
        search_text: &str,
        scroll_to: usize,
        width: i32,
    ) -> usize {
        let scroll_to = scroll_to.min(search_text.chars().count());
        if variable_debug_options_search_prefix_width(search_text, scroll_to) <= width {
            return 0;
        }

        let chars = search_text.chars().take(scroll_to).collect::<Vec<_>>();
        let mut start = chars.len();
        let mut used_width = 0;
        while start > 0 {
            let advance = variable_debug_options_search_char_advance(chars[start - 1]);
            if used_width + advance > width {
                break;
            }
            used_width += advance;
            start -= 1;
        }
        start
    }

    fn debug_options_search_cursor_for_text_offset_from_display_start(
        &self,
        search_text: &str,
        display_start: usize,
        offset: i32,
    ) -> usize {
        let display_start = display_start.min(search_text.chars().count());
        let tail = search_text.chars().skip(display_start).collect::<String>();
        display_start + variable_debug_options_search_cursor_for_text_offset(&tail, offset)
    }
}

struct FixedCommandSuggestionTextMeasurer;

impl CommandSuggestionTextMeasurer for FixedCommandSuggestionTextMeasurer {
    fn command_suggestion_text_width(&self, text: &str) -> u32 {
        u32::try_from(text.chars().count())
            .unwrap_or(u32::MAX)
            .saturating_mul(6)
    }

    fn command_suggestion_display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize {
        scroll_to
            .min(text.chars().count())
            .saturating_sub(usize::try_from(width / 6).unwrap_or(usize::MAX))
    }

    fn command_suggestion_cursor_for_width(
        &self,
        text: &str,
        display_start: usize,
        width: u32,
    ) -> usize {
        display_start
            .saturating_add(usize::try_from(width / 6).unwrap_or(usize::MAX))
            .min(text.chars().count())
    }
}

fn variable_debug_options_search_cursor_for_text_offset(search_text: &str, offset: i32) -> usize {
    let mut width = 0;
    let mut cursor = 0;
    for ch in search_text.chars() {
        let advance = variable_debug_options_search_char_advance(ch);
        if width + advance > offset {
            break;
        }
        width += advance;
        cursor += 1;
    }
    cursor
}

fn variable_debug_options_search_char_advance(ch: char) -> i32 {
    match ch {
        'i' => 2,
        'w' => 7,
        ' ' => 4,
        _ => DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE,
    }
}

fn variable_debug_options_search_prefix_width(search_text: &str, char_count: usize) -> i32 {
    search_text
        .chars()
        .take(char_count)
        .map(variable_debug_options_search_char_advance)
        .sum()
}

fn world_with_local_boat(player_id: i32) -> WorldStore {
    world_with_local_vehicle(player_id, 10, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
}

fn world_with_local_vehicle(player_id: i32, vehicle_id: i32, entity_type_id: i32) -> WorldStore {
    let mut world = world_with_local_player_id(player_id);
    world.apply_add_entity(AddEntity {
        id: vehicle_id,
        uuid: Uuid::from_u128(vehicle_id as u128),
        entity_type_id,
        position: ProtocolVec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    if entity_type_id == VANILLA_ENTITY_TYPE_HORSE_ID {
        world.set_default_item_equipment_slots(BTreeMap::from([(
            TEST_SADDLE_ITEM_ID,
            ItemEquipmentSlot::Saddle,
        )]));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: vehicle_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: ProtocolItemStackSummary {
                    item_id: Some(TEST_SADDLE_ITEM_ID),
                    count: 1,
                    component_patch: Default::default(),
                },
            }],
        }));
    }
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id,
        passenger_ids: vec![player_id],
    }));
    world
}

fn sign_text_side(lines: [&str; 4]) -> bbb_world::SignTextSideState {
    bbb_world::SignTextSideState {
        lines: lines.map(|line| {
            if line.is_empty() {
                Vec::new()
            } else {
                vec![bbb_protocol::StyledTextRun {
                    text: line.to_string(),
                    style: bbb_protocol::ComponentStyle::default(),
                }]
            }
        }),
        filtered_lines: None,
        color: bbb_world::SignTextDyeColor::Black,
        has_glowing_text: false,
    }
}

fn sign_text_side_with_filtered(
    lines: [&str; 4],
    filtered: [&str; 4],
) -> bbb_world::SignTextSideState {
    let mut side = sign_text_side(lines);
    side.filtered_lines = Some(sign_text_side(filtered).lines);
    side
}

fn world_with_sign_text(pos: BlockPos, front: [&str; 4], back: [&str; 4]) -> WorldStore {
    let mut world = WorldStore::new();
    insert_sign_text(&mut world, pos, front, back);
    world
}

fn insert_sign_text(world: &mut WorldStore, pos: BlockPos, front: [&str; 4], back: [&str; 4]) {
    insert_sign_text_state(world, pos, sign_text_side(front), sign_text_side(back));
}

fn insert_sign_text_state(
    world: &mut WorldStore,
    pos: BlockPos,
    front: bbb_world::SignTextSideState,
    back: bbb_world::SignTextSideState,
) {
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: Vec::new(),
        block_entities: vec![BlockEntityRecord {
            local_x: pos.x.rem_euclid(16) as u8,
            y: i16::try_from(pos.y).unwrap(),
            local_z: pos.z.rem_euclid(16) as u8,
            type_id: 7,
            raw_nbt: vec![0],
            nbt: None,
            sign_text: Some(SignBlockEntityTextState {
                front,
                back,
                is_waxed: false,
            }),
            command_block: None,
            structure_block: None,
            jigsaw_block: None,
            test_block: None,
            test_instance_block: None,
            vault_shared_data: None,
            decorated_pot_sherds: None,
            banner_patterns: None,
            end_gateway: None,
            spawner: None,
            note_block_sound: None,
            skull_profile: None,
        }],
        light: LightData::default(),
    });
}

fn insert_empty_chunk_for_block(world: &mut WorldStore, pos: BlockPos) {
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24).map(|_| empty_chunk_section()).collect(),
        block_entities: Vec::new(),
        light: LightData::default(),
    });
}

fn world_with_command_block(pos: BlockPos, command: &str) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let block_state_id = bbb_world::vanilla_block_state_id(
        "minecraft:command_block",
        &BTreeMap::from([
            ("conditional".to_string(), "false".to_string()),
            ("facing".to_string(), "north".to_string()),
        ]),
    )
    .expect("vanilla command block state");
    let section_index = usize::try_from((pos.y + 64).div_euclid(16))
        .expect("test command block lies in the overworld build height");
    let record = BlockEntityRecord {
        local_x: pos.x.rem_euclid(16) as u8,
        y: i16::try_from(pos.y).unwrap(),
        local_z: pos.z.rem_euclid(16) as u8,
        type_id: VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
        raw_nbt: vec![0],
        nbt: None,
        sign_text: None,
        vault_shared_data: None,
        decorated_pot_sherds: None,
        banner_patterns: None,
        end_gateway: None,
        spawner: None,
        command_block: Some(CommandBlockEntityDataState {
            command: command.to_string(),
            ..CommandBlockEntityDataState::default()
        }),
        structure_block: None,
        jigsaw_block: None,
        test_block: None,
        test_instance_block: None,
        note_block_sound: None,
        skull_profile: None,
    };
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|index| ChunkSection {
                non_empty_block_count: if index == section_index { 4096 } else { 0 },
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    if index == section_index {
                        block_state_id
                    } else {
                        0
                    },
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })
            .collect(),
        block_entities: vec![record],
        light: LightData::default(),
    });
    world
}

fn world_with_command_minecart(entity_id: i32, command: &str, output: &str) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: entity_id,
        uuid: Uuid::from_u128(u128::try_from(entity_id).unwrap()),
        entity_type_id: VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    apply_command_minecart_metadata(&mut world, entity_id, command, output);
    world
}

fn apply_command_minecart_metadata(
    world: &mut WorldStore,
    entity_id: i32,
    command: &str,
    output: &str,
) {
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: entity_id,
        values: vec![
            ProtocolEntityDataValue {
                data_id: bbb_world::VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_STRING_ID,
                value: EntityDataValueKind::String(command.to_string()),
            },
            ProtocolEntityDataValue {
                data_id: bbb_world::VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID,
                value: EntityDataValueKind::Component(RichTextComponent::plain(output)),
            },
        ],
    }));
}

fn world_with_test_block(pos: BlockPos, mode: TestBlockMode, message: &str) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let block_state_id = bbb_world::vanilla_block_state_id(
        "minecraft:test_block",
        &BTreeMap::from([("mode".to_string(), mode.serialized_name().to_string())]),
    )
    .expect("vanilla test block state");
    let section_index = usize::try_from((pos.y + 64).div_euclid(16))
        .expect("test block lies in the overworld build height");
    let record = BlockEntityRecord {
        local_x: pos.x.rem_euclid(16) as u8,
        y: i16::try_from(pos.y).unwrap(),
        local_z: pos.z.rem_euclid(16) as u8,
        type_id: VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
        raw_nbt: vec![0],
        nbt: None,
        sign_text: None,
        vault_shared_data: None,
        decorated_pot_sherds: None,
        banner_patterns: None,
        end_gateway: None,
        spawner: None,
        command_block: None,
        structure_block: None,
        jigsaw_block: None,
        test_block: Some(TestBlockEntityDataState {
            mode,
            message: message.to_string(),
            powered: false,
        }),
        test_instance_block: None,
        note_block_sound: None,
        skull_profile: None,
    };
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|index| ChunkSection {
                non_empty_block_count: if index == section_index { 4096 } else { 0 },
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    if index == section_index {
                        block_state_id
                    } else {
                        0
                    },
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })
            .collect(),
        block_entities: vec![record],
        light: LightData::default(),
    });
    world
}

fn world_with_test_instance_block(pos: BlockPos) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let block_state_id =
        bbb_world::vanilla_block_state_id("minecraft:test_instance_block", &BTreeMap::new())
            .expect("vanilla test-instance block state");
    let section_index = usize::try_from((pos.y + 64).div_euclid(16))
        .expect("test-instance block lies in the overworld build height");
    let record = BlockEntityRecord {
        local_x: pos.x.rem_euclid(16) as u8,
        y: i16::try_from(pos.y).unwrap(),
        local_z: pos.z.rem_euclid(16) as u8,
        type_id: VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
        raw_nbt: vec![0],
        nbt: None,
        sign_text: None,
        vault_shared_data: None,
        decorated_pot_sherds: None,
        banner_patterns: None,
        end_gateway: None,
        spawner: None,
        command_block: None,
        structure_block: None,
        jigsaw_block: None,
        test_block: None,
        test_instance_block: Some(TestInstanceBlockEntityDataState {
            test: Some("example:initial".to_string()),
            size: TestInstanceBlockSizeState { x: 3, y: 4, z: 5 },
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: false,
            status: TestInstanceBlockRunStatus::Cleared,
            error_message: None,
        }),
        note_block_sound: None,
        skull_profile: None,
    };
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|index| ChunkSection {
                non_empty_block_count: if index == section_index { 4096 } else { 0 },
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    if index == section_index {
                        block_state_id
                    } else {
                        0
                    },
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })
            .collect(),
        block_entities: vec![record],
        light: LightData::default(),
    });
    world
}

fn world_with_structure_block(pos: BlockPos) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let block_state_id = bbb_world::vanilla_block_state_id(
        "minecraft:structure_block",
        &BTreeMap::from([("mode".to_string(), "load".to_string())]),
    )
    .expect("vanilla structure block state");
    let section_index = usize::try_from((pos.y + 64).div_euclid(16))
        .expect("structure block lies in the overworld build height");
    let record = BlockEntityRecord {
        local_x: pos.x.rem_euclid(16) as u8,
        y: i16::try_from(pos.y).unwrap(),
        local_z: pos.z.rem_euclid(16) as u8,
        type_id: VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
        raw_nbt: vec![0],
        nbt: None,
        sign_text: None,
        vault_shared_data: None,
        decorated_pot_sherds: None,
        banner_patterns: None,
        end_gateway: None,
        spawner: None,
        command_block: None,
        test_block: None,
        structure_block: Some(StructureBlockEntityDataState {
            name: "example:initial".to_string(),
            author: "builder".to_string(),
            metadata: "initial data".to_string(),
            offset: BlockPos { x: 1, y: -2, z: 3 },
            size: StructureBlockSizeState { x: 4, y: 5, z: 6 },
            mirror: StructureBlockMirror::FrontBack,
            rotation: StructureBlockRotation::Clockwise90,
            mode: StructureBlockMode::Load,
            ignore_entities: false,
            strict: true,
            powered: false,
            show_air: true,
            show_bounding_box: false,
            integrity_bits: 0.75_f32.to_bits(),
            seed: 9,
        }),
        jigsaw_block: None,
        test_instance_block: None,
        note_block_sound: None,
        skull_profile: None,
    };
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|index| ChunkSection {
                non_empty_block_count: if index == section_index { 4096 } else { 0 },
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    if index == section_index {
                        block_state_id
                    } else {
                        0
                    },
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })
            .collect(),
        block_entities: vec![record],
        light: LightData::default(),
    });
    world
}

fn world_with_jigsaw_block(pos: BlockPos, orientation: &str) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let block_state_id = bbb_world::vanilla_block_state_id(
        "minecraft:jigsaw",
        &BTreeMap::from([("orientation".to_string(), orientation.to_string())]),
    )
    .expect("vanilla jigsaw block state");
    let section_index = usize::try_from((pos.y + 64).div_euclid(16))
        .expect("jigsaw block lies in the overworld build height");
    let record = BlockEntityRecord {
        local_x: pos.x.rem_euclid(16) as u8,
        y: i16::try_from(pos.y).unwrap(),
        local_z: pos.z.rem_euclid(16) as u8,
        type_id: VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
        raw_nbt: vec![0],
        nbt: None,
        sign_text: None,
        vault_shared_data: None,
        decorated_pot_sherds: None,
        banner_patterns: None,
        end_gateway: None,
        spawner: None,
        command_block: None,
        structure_block: None,
        jigsaw_block: Some(JigsawBlockEntityDataState {
            name: "example:entry".to_string(),
            target: "example:target".to_string(),
            pool: "example:village/houses".to_string(),
            final_state: "minecraft:oak_planks[axis=y]".to_string(),
            joint: JigsawJoint::Rollable,
            selection_priority: -2,
            placement_priority: 3,
        }),
        test_block: None,
        test_instance_block: None,
        note_block_sound: None,
        skull_profile: None,
    };
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|index| ChunkSection {
                non_empty_block_count: if index == section_index { 4096 } else { 0 },
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    if index == section_index {
                        block_state_id
                    } else {
                        0
                    },
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })
            .collect(),
        block_entities: vec![record],
        light: LightData::default(),
    });
    world
}

fn click_structure_block_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    commands: &Option<mpsc::Sender<NetCommand>>,
    rect: bbb_renderer::HudCommandBlockEditorRect,
) -> bool {
    input.handle_structure_block_editor_mouse_input(
        counters,
        world,
        commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(rect.x + 1),
            f64::from(rect.y + 1),
        )),
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    )
}

fn click_jigsaw_block_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    commands: &Option<mpsc::Sender<NetCommand>>,
    rect: bbb_renderer::HudCommandBlockEditorRect,
) -> bool {
    input.handle_jigsaw_block_editor_mouse_input(
        counters,
        world,
        commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(rect.x + 1),
            f64::from(rect.y + 1),
        )),
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    )
}

fn click_test_instance_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    commands: &Option<mpsc::Sender<NetCommand>>,
    rect: bbb_renderer::HudCommandBlockEditorRect,
) -> bool {
    input.handle_test_instance_block_editor_mouse_input(
        counters,
        world,
        commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(rect.x + 1),
            f64::from(rect.y + 1),
        )),
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    )
}

fn click_test_block_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    commands: &Option<mpsc::Sender<NetCommand>>,
    rect: bbb_renderer::HudCommandBlockEditorRect,
) -> bool {
    input.handle_test_block_editor_mouse_input(
        counters,
        world,
        commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(rect.x + 1),
            f64::from(rect.y + 1),
        )),
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    )
}

fn empty_chunk_section() -> ChunkSection {
    ChunkSection {
        non_empty_block_count: 0,
        fluid_count: 0,
        block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
        biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
    }
}

fn single_value_container(
    domain: PaletteDomain,
    entry_count: usize,
    global_id: i32,
) -> PalettedContainerData {
    PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
    BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties(name, &string_props(props))
        .unwrap_or_else(|| panic!("missing vanilla block state {name} {props:?}"))
        .id
}

fn string_props<const N: usize>(props: [(&str, &str); N]) -> BTreeMap<String, String> {
    props
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn player_chat_with_signature(global_index: i32, signature: MessageSignature) -> PlayerChat {
    PlayerChat {
        global_index,
        sender: Uuid::from_u128(0x1234),
        index: global_index,
        signature: Some(signature),
        body: SignedMessageBody {
            content: format!("message {global_index}"),
            timestamp_millis: i64::from(global_index),
            salt: i64::from(global_index) + 1,
            last_seen: Vec::new(),
        },
        unsigned_content: None,
        unsigned_content_styled: None,
        unsigned_content_component: None,
        filter_mask: FilterMask {
            kind: FilterMaskKind::PassThrough,
            mask_words: Vec::new(),
        },
        chat_type: ChatTypeBound {
            chat_type: ChatTypeHolder::Registry { id: 0 },
            name: "Alice".to_string(),
            name_styled: Vec::new(),
            name_component: None,
            target_name: None,
            target_name_styled: None,
            target_name_component: None,
        },
    }
}

fn signable_message_command_tree() -> Commands {
    Commands {
        root_index: 0,
        nodes: vec![
            CommandNode {
                node_type: CommandNodeType::Root,
                flags: 0,
                children: vec![1],
                redirect: None,
                name: None,
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            CommandNode {
                node_type: CommandNodeType::Literal,
                flags: 1,
                children: vec![2],
                redirect: None,
                name: Some("say".to_string()),
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            CommandNode {
                node_type: CommandNodeType::Argument,
                flags: 22,
                children: Vec::new(),
                redirect: None,
                name: Some("message".to_string()),
                parser: Some(CommandArgumentParser {
                    type_id: VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
                    name: "minecraft:message".to_string(),
                    properties: Vec::new(),
                }),
                suggestions: Some("minecraft:ask_server".to_string()),
                executable: true,
                restricted: false,
            },
        ],
    }
}

fn ask_server_command_tree() -> Commands {
    Commands {
        root_index: 0,
        nodes: vec![
            CommandNode {
                node_type: CommandNodeType::Root,
                flags: 0,
                children: vec![1],
                redirect: None,
                name: None,
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            CommandNode {
                node_type: CommandNodeType::Argument,
                flags: 18,
                children: Vec::new(),
                redirect: None,
                name: Some("value".to_string()),
                parser: Some(CommandArgumentParser {
                    type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
                    name: "brigadier:string".to_string(),
                    properties: vec![2],
                }),
                suggestions: Some("minecraft:ask_server".to_string()),
                executable: true,
                restricted: false,
            },
        ],
    }
}

fn local_literal_command_tree(literals: &[&str]) -> Commands {
    let children = (1..=literals.len())
        .map(|index| i32::try_from(index).expect("fixture node index"))
        .collect();
    let mut nodes = vec![CommandNode {
        node_type: CommandNodeType::Root,
        flags: 0,
        children,
        redirect: None,
        name: None,
        parser: None,
        suggestions: None,
        executable: false,
        restricted: false,
    }];
    nodes.extend(literals.iter().map(|literal| CommandNode {
        node_type: CommandNodeType::Literal,
        flags: 5,
        children: Vec::new(),
        redirect: None,
        name: Some((*literal).to_string()),
        parser: None,
        suggestions: None,
        executable: true,
        restricted: false,
    }));
    Commands {
        root_index: 0,
        nodes,
    }
}

fn set_local_player_on_ground(world: &mut WorldStore, on_ground: bool) {
    world.set_local_player_pose(LocalPlayerPoseState {
        on_ground,
        ..LocalPlayerPoseState::default()
    });
}

fn set_player_abilities(world: &mut WorldStore, flying: bool, can_fly: bool) {
    world.apply_player_abilities(PlayerAbilities {
        invulnerable: false,
        flying,
        can_fly,
        instabuild: can_fly,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
}

fn equip_player_slot(world: &mut WorldStore, slot: i32, item_id: i32, count: i32) {
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot,
        item: ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        },
    });
}

fn set_local_spectator(world: &mut WorldStore) {
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    assert!(world.local_player_is_spectator());
}

fn world_with_sleeping_local_player(player_id: i32) -> WorldStore {
    let mut world = world_with_local_player_id(player_id);
    world.apply_add_entity(AddEntity {
        id: player_id,
        uuid: Uuid::from_u128(player_id as u128),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: player_id,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID)],
    }));
    world
}

#[test]
fn render_input_publication_projects_display_state_without_simulation_queues() {
    let mut input = ClientInputState::new(true);
    input.forward = true;
    input.mouse_delta_x = 12.5;
    input.shift_left_down = true;
    input.pressed_keys.insert(KeyCode::Tab);
    input.inventory_hovered_slot = Some(7);
    input.inventory_cursor_position = Some((123, 45));
    input.inventory_quick_craft_button_num = Some(1);
    input.inventory_quick_craft_slots = vec![2, 5, 7];
    input.recipe_book_search_text = "visible recipe".to_owned();
    input.advancement_scroll_deltas = BTreeMap::from([("minecraft:test".to_owned(), (2.0, 3.0))]);
    input.debug_profiler_chart_visible = true;
    input.debug_fog_enabled = false;
    input
        .debug_frustum_requests
        .push(DebugFrustumRequest::Capture);
    input.debug_resource_pack_reload_requests = 3;
    input.chat_entry = Some(ChatEntryState {
        text: "/help".to_owned(),
        ..ChatEntryState::default()
    });
    input.riding_jump_charge_seconds = Some(0.6);
    input.last_move_command_at = Some(Instant::now());
    input.move_position_reminder_ticks = 19;

    let publication = input.render_publication();
    let projected = publication.as_state();

    assert!(projected.focused);
    assert!(projected.shift_left_down);
    assert!(projected.pressed_keys.contains(&KeyCode::Tab));
    assert_eq!(projected.inventory_hovered_slot, Some(7));
    assert_eq!(projected.inventory_cursor_position, Some((123, 45)));
    assert_eq!(projected.inventory_quick_craft_slots, [2, 5, 7]);
    assert_eq!(projected.recipe_book_search_text, "visible recipe");
    assert_eq!(
        projected.advancement_scroll_deltas["minecraft:test"],
        (2.0, 3.0)
    );
    assert!(projected.debug_profiler_chart_visible);
    assert!(!projected.debug_fog_enabled);
    assert_eq!(
        projected
            .chat_entry
            .as_ref()
            .map(|entry| entry.text.as_str()),
        Some("/help")
    );
    assert_eq!(projected.riding_jump_charge_seconds, Some(0.6));

    assert!(!projected.forward);
    assert_eq!(projected.mouse_delta_x, 0.0);
    assert!(projected.debug_frustum_requests.is_empty());
    assert_eq!(projected.debug_resource_pack_reload_requests, 0);
    assert!(projected.last_move_command_at.is_none());
    assert_eq!(projected.move_position_reminder_ticks, 0);
}

#[test]
fn prediction_sequence_starts_at_one_and_wraps_positive() {
    let mut world = WorldStore::new();

    assert_eq!(world.next_local_prediction_sequence(), 1);
    assert_eq!(world.next_local_prediction_sequence(), 2);

    world.set_local_prediction_sequence(i32::MAX);
    assert_eq!(world.next_local_prediction_sequence(), 1);
}

#[test]
fn debug_advanced_item_tooltips_startup_state_is_runtime_initial_value() {
    let mut input = ClientInputState::new(true);
    input.set_debug_advanced_item_tooltips(true);

    assert!(input.debug_advanced_item_tooltips());

    let commands = None;
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyH),
        ElementState::Pressed,
    );

    assert!(!input.debug_advanced_item_tooltips());
    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(feedback, vec!["[Debug]: Advanced tooltips: hidden"]);
}

#[test]
fn digit_key_selects_hotbar_slot_updates_world_and_queues_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 4);
    assert_eq!(world.counters().held_slot_packets, 0);
    assert_eq!(counters.held_slot_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(4));
}

#[test]
fn spectator_digit_key_opens_menu_without_queuing_held_slot() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    assert!(world.set_local_selected_hotbar_slot(2));
    set_local_spectator(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 2);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(input.spectator_menu_is_active());
    assert!(rx.try_recv().is_err());
}

#[test]
fn unfocused_movement_key_does_not_queue_player_input() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(false);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn unfocused_hotbar_or_drop_key_does_not_queue_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(false);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_release_toggles_debug_overlay_without_gameplay_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    assert!(!input.debug_overlay_visible());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_digit_chart_keys_toggle_overlay_state_and_do_not_toggle_on_f3_release() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::Digit1),
        ElementState::Pressed,
    );
    assert!(input.debug_overlay_visible());
    assert!(input.debug_profiler_chart_visible());
    assert!(input
        .take_debug_profiler_chart_navigation_requests()
        .is_empty());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
    );
    assert!(input.debug_profiler_chart_visible());
    assert!(input.debug_fps_charts_visible());
    assert!(!input.debug_network_charts_visible());
    assert!(!input.debug_lightmap_texture_visible());
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());
    assert!(input.debug_fps_charts_visible());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::Digit3),
        ElementState::Pressed,
    );
    assert!(input.debug_network_charts_visible());
    assert!(!input.debug_fps_charts_visible());
    assert!(!input.debug_lightmap_texture_visible());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::Digit4),
        ElementState::Pressed,
    );
    assert!(input.debug_lightmap_texture_visible());
    assert!(!input.debug_fps_charts_visible());
    assert!(!input.debug_network_charts_visible());
}

#[test]
fn profiler_chart_digit_navigation_records_without_blocking_hotbar() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit1),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(input.debug_profiler_chart_visible());
    assert!(input
        .take_debug_profiler_chart_navigation_requests()
        .is_empty());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
    );

    assert_eq!(
        input.take_debug_profiler_chart_navigation_requests(),
        vec![2]
    );
    assert_eq!(world.local_player().selected_hotbar_slot, 1);
    assert_eq!(counters.held_slot_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(1));
}

#[test]
fn f3_debug_status_keys_toggle_state_without_forcing_overlay_visible() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
    );
    assert!(input.debug_entity_hitboxes_visible());
    assert!(!input.debug_overlay_visible());
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_entity_hitboxes_visible());
    assert!(!input.debug_overlay_visible());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyG),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyH),
        ElementState::Pressed,
    );
    assert!(input.debug_chunk_borders_visible());
    assert!(input.debug_advanced_item_tooltips());
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    for code in [KeyCode::KeyB, KeyCode::KeyG, KeyCode::KeyH] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }
    assert!(!input.debug_entity_hitboxes_visible());
    assert!(!input.debug_chunk_borders_visible());
    assert!(!input.debug_advanced_item_tooltips());
    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(
        feedback,
        vec![
            "[Debug]: Hitboxes: shown",
            "[Debug]: Chunk borders: shown",
            "[Debug]: Advanced tooltips: shown",
            "[Debug]: Hitboxes: hidden",
            "[Debug]: Chunk borders: hidden",
            "[Debug]: Advanced tooltips: hidden",
        ]
    );
    assert!(world
        .client_chat()
        .messages
        .iter()
        .all(|message| message.kind == ChatMessageKind::ClientSystem));
    assert_eq!(world.counters().chat_messages_tracked, 6);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn shared_debug_hotkeys_toggle_dev_state_when_startup_flag_is_enabled() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    input.set_debug_hotkeys_enabled(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    for code in [
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyL,
        KeyCode::KeyO,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
    ] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }
    input.shift_left_down = true;
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyU),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::ChunkSectionPaths),
        DebugScreenEntryStatus::AlwaysOn
    );
    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::ChunkSectionOctree),
        DebugScreenEntryStatus::AlwaysOn
    );
    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::ChunkSectionVisibility),
        DebugScreenEntryStatus::AlwaysOn
    );
    assert!(!input.debug_fog_enabled());
    assert!(!input.debug_smart_cull_enabled());
    assert!(input.debug_wireframe_enabled);
    assert_eq!(
        input.take_debug_frustum_requests(),
        vec![DebugFrustumRequest::Capture, DebugFrustumRequest::Kill]
    );
    assert!(input.take_debug_feature_count_requests().is_empty());
    assert!(input.take_debug_profiling_toggle_requests().is_empty());
    assert!(!input.debug_overlay_visible());

    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(
        feedback,
        vec![
            "[Debug]: SectionPath: shown",
            "[Debug]: Fog: disabled",
            "[Debug]: SmartCull: disabled",
            "[Debug]: Frustum culling Octree: enabled",
            "[Debug]: Captured frustum",
            "[Debug]: SectionVisibility: enabled",
            "[Debug]: WireFrame: enabled",
            "[Debug]: Killed frustum",
        ]
    );
}

#[test]
fn shared_debug_hotkeys_without_player_only_handle_global_dev_toggles() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    input.set_debug_hotkeys_enabled(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::ChunkSectionPaths),
        DebugScreenEntryStatus::Never
    );
    assert!(!input.debug_fog_enabled());
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_feature_count_hotkeys_are_gated_ahead_of_profiling() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    input.set_debug_feature_count_enabled(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyR),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(
        input.take_debug_feature_count_requests(),
        vec![
            DebugFeatureCountRequest::Log,
            DebugFeatureCountRequest::Clear
        ]
    );
    assert!(input.take_debug_profiling_toggle_requests().is_empty());
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_hotkeys_take_priority_over_feature_count_and_regular_keymap() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    input.set_debug_hotkeys_enabled(true);
    input.set_debug_feature_count_enabled(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(!input.debug_smart_cull_enabled());
    assert!(input.take_debug_feature_count_requests().is_empty());
    assert!(input.take_debug_profiling_toggle_requests().is_empty());
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_debug_status_key_uses_in_overlay_status_when_overlay_is_visible() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
    );

    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::EntityHitboxes),
        DebugScreenEntryStatus::InOverlay
    );
    assert!(input.debug_entity_hitboxes_visible());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());
    assert!(input.debug_entity_hitboxes_visible());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(!input.debug_overlay_visible());
    assert!(!input.debug_entity_hitboxes_visible());
}

#[test]
fn f3_debug_status_toggle_persists_debug_profile_store() {
    let commands = None;
    let path = unique_input_temp_dir("debug-profile-store").join("debug-profile.json");
    let mut input = ClientInputState::new(true);
    input.set_debug_profile_store_path(path.clone());
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
    );

    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
        .expect("debug profile json parses");
    assert!(value.get("profile").is_none());
    assert_eq!(
        value["custom"]["minecraft:entity_hitboxes"].as_str(),
        Some("alwaysOn")
    );

    let loaded =
        DebugScreenEntryList::load_from_debug_profile_file(&path, DebugScreenProfile::Default)
            .unwrap();
    assert_eq!(
        loaded.status(DebugScreenEntryId::EntityHitboxes),
        DebugScreenEntryStatus::AlwaysOn
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(path.parent().unwrap());
}

#[test]
fn custom_debug_screen_entry_status_drives_entry_visibility() {
    let mut input = ClientInputState::new(true);

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ChunkBorders,
        DebugScreenEntryStatus::AlwaysOn,
    );
    assert!(input.debug_chunk_borders_visible());

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ChunkBorders,
        DebugScreenEntryStatus::Never,
    );
    assert!(!input.debug_chunk_borders_visible());
}

#[test]
fn f3_debug_status_keys_follow_player_reduced_debug_gate() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
    );
    assert!(!input.debug_entity_hitboxes_visible());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyH),
        ElementState::Pressed,
    );
    assert!(input.debug_advanced_item_tooltips());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());

    let mut reduced_world = world_with_debug_player(true);
    let mut reduced_input = ClientInputState::new(true);
    handle_key_input(
        &mut reduced_input,
        &mut counters,
        &mut reduced_world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    for code in [KeyCode::KeyB, KeyCode::KeyG] {
        handle_key_input(
            &mut reduced_input,
            &mut counters,
            &mut reduced_world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }
    assert!(!reduced_input.debug_entity_hitboxes_visible());
    assert!(!reduced_input.debug_chunk_borders_visible());
    handle_key_input(
        &mut reduced_input,
        &mut counters,
        &mut reduced_world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(reduced_input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_d_clears_chat_display_without_resetting_signature_state() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    let _ = world.apply_player_chat(player_chat_with_signature(
        0,
        MessageSignature {
            bytes: vec![9; 256],
        },
    ));
    world.apply_delete_chat(DeleteChat {
        message_signature: PackedMessageSignature {
            cache_id: Some(0),
            full_signature: None,
        },
    });
    assert_eq!(world.client_chat().messages.len(), 1);
    assert_eq!(world.client_chat().deleted_messages.len(), 1);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyD),
        ElementState::Pressed,
    );
    assert!(world.client_chat().messages.is_empty());
    assert!(world.client_chat().deleted_messages.is_empty());
    assert_eq!(world.client_chat().expected_player_chat_global_index, 1);
    assert!(world
        .client_chat()
        .signature_cache
        .iter()
        .any(Option::is_some));
    assert_eq!(world.client_chat().acknowledgement.pending_offset, 1);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
    assert_eq!(world.counters().chat_messages_tracked, 0);
    assert_eq!(world.counters().deleted_chat_messages_tracked, 0);
    assert_eq!(world.counters().chat_signature_cache_entries, 1);
    assert_eq!(world.counters().reset_chat_packets, 0);
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_v_dumps_version_debug_chat_without_toggling_overlay_on_release() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
    );

    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 10);
    assert!(messages
        .iter()
        .all(|message| message.kind == ChatMessageKind::ClientSystem));
    assert_eq!(messages[0].content, "[Debug]: Client version info:");
    assert_eq!(messages[1].content, format!("id = {MC_VERSION}"));
    assert_eq!(messages[2].content, format!("name = {MC_VERSION}"));
    assert_eq!(messages[3].content, format!("data = {MC_DATA_VERSION}"));
    assert_eq!(
        messages[4].content,
        format!("series = {MC_DATA_VERSION_SERIES}")
    );
    assert_eq!(
        messages[5].content,
        format!("protocol = {PROTOCOL_VERSION} (0x{PROTOCOL_VERSION:x})")
    );
    assert_eq!(messages[6].content, format!("build_time = {MC_BUILD_TIME}"));
    assert_eq!(
        messages[7].content,
        format!(
            "pack_resource = {}",
            MC_RESOURCE_PACK_FORMAT.to_vanilla_string()
        )
    );
    assert_eq!(
        messages[8].content,
        format!("pack_data = {}", MC_DATA_PACK_FORMAT.to_vanilla_string())
    );
    assert_eq!(
        messages[9].content,
        if MC_STABLE {
            "stable = yes"
        } else {
            "stable = no"
        }
    );
    assert_eq!(world.counters().chat_messages_tracked, 10);
    assert_eq!(world.counters().player_chat_packets, 0);
    assert_eq!(world.counters().disguised_chat_packets, 0);
    assert_eq!(world.counters().system_chat_packets, 0);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_v_consumes_without_world_to_suppress_release_toggle() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_t_records_resource_pack_reload_request_and_feedback_without_toggling_overlay() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );

    assert_eq!(input.take_debug_resource_pack_reload_requests(), 1);
    assert_eq!(input.take_debug_resource_pack_reload_requests(), 0);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(messages[0].content, "[Debug]: Reloaded resource packs");
    assert_eq!(messages[0].styled_content.len(), 3);
    assert_eq!(messages[0].styled_content[0].text, "[Debug]:");
    assert_eq!(messages[0].styled_content[0].style.bold, Some(true));
    assert_eq!(
        messages[0].styled_content[0].style.color,
        Some(VANILLA_DEBUG_FEEDBACK_COLOR)
    );
    assert_eq!(messages[0].styled_content[1].text, " ");
    assert_eq!(
        messages[0].styled_content[1].style,
        ComponentStyle::default()
    );
    assert_eq!(
        messages[0].styled_content[2].text,
        "Reloaded resource packs"
    );
    assert_eq!(
        messages[0].styled_content[2].style,
        ComponentStyle::default()
    );
    assert_eq!(world.counters().chat_messages_tracked, 1);
    assert_eq!(world.counters().player_chat_packets, 0);
    assert_eq!(world.counters().disguised_chat_packets, 0);
    assert_eq!(world.counters().system_chat_packets, 0);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_t_consumes_without_world_and_records_reload_request() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_resource_pack_reload_requests(), 1);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_s_records_dynamic_texture_dump_request_and_feedback_without_toggling_overlay() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyS),
        ElementState::Pressed,
    );

    assert_eq!(input.take_debug_dynamic_texture_dump_requests(), 1);
    assert_eq!(input.take_debug_dynamic_texture_dump_requests(), 0);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Saved dynamic textures to screenshots/debug"
    );
    assert_eq!(messages[0].styled_content.len(), 4);
    assert_eq!(messages[0].styled_content[0].text, "[Debug]:");
    assert_eq!(messages[0].styled_content[0].style.bold, Some(true));
    assert_eq!(
        messages[0].styled_content[0].style.color,
        Some(VANILLA_DEBUG_FEEDBACK_COLOR)
    );
    assert_eq!(messages[0].styled_content[1].text, " ");
    assert_eq!(
        messages[0].styled_content[1].style,
        ComponentStyle::default()
    );
    assert_eq!(
        messages[0].styled_content[2].text,
        "Saved dynamic textures to "
    );
    assert_eq!(
        messages[0].styled_content[2].style,
        ComponentStyle::default()
    );
    assert_eq!(
        messages[0].styled_content[3].text,
        DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH
    );
    assert_eq!(messages[0].styled_content[3].style.underlined, Some(true));
    assert_eq!(
        messages[0].styled_content[3].style.click_event,
        Some(ComponentClickEvent::OpenFile {
            path: DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH.to_string(),
        })
    );
    assert_eq!(world.counters().chat_messages_tracked, 1);
    assert_eq!(world.counters().player_chat_packets, 0);
    assert_eq!(world.counters().disguised_chat_packets, 0);
    assert_eq!(world.counters().system_chat_packets, 0);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_s_consumes_without_world_and_records_dump_request() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyS),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_dynamic_texture_dump_requests(), 1);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_i_consumes_copy_data_modifier_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn shift_f3_i_copies_block_recreate_command_to_clipboard_and_reports_feedback() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]")
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn shift_f3_i_with_permission_copies_local_block_entity_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:spawner", []),
    }));
    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
            raw_nbt: nbt_compound(vec![nbt_string("Lock", "secret")]),
        })
        .expect("local block entity nbt should decode"));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:spawner{Lock:\"secret\"}")
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );
}

#[test]
fn shift_f3_i_without_permission_omits_local_block_entity_nbt() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:spawner", []),
    }));
    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
            raw_nbt: nbt_compound(vec![nbt_string("Lock", "secret")]),
        })
        .expect("local block entity nbt should decode"));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:spawner")
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );
}

#[test]
fn f3_i_without_shift_records_server_recreate_query_request_shell() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(clipboard.text, None);
    assert_eq!(
        input.pending_debug_recreate_server_query,
        Some(PendingDebugRecreateServerQuery {
            transaction_id: 0,
            client_level_generation: world.client_level_generation(),
            target: PendingDebugRecreateServerQueryTarget::Block {
                pos: target_pos,
                description: "minecraft:oak_log[axis=x]".to_string(),
            },
        })
    );
    assert_eq!(
        input.take_debug_recreate_server_query_requests(),
        vec![DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id: 0,
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
        }]
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    assert!(world.client_chat().messages.is_empty());

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_i_without_gamemaster_permission_copies_client_recreate_command_without_query() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]")
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );
}

#[test]
fn f3_i_server_block_tag_response_copies_recreate_command() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        input.take_debug_recreate_server_query_requests(),
        vec![DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id: 0,
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
        }]
    );
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    world.apply_tag_query(TagQuery {
        transaction_id: 0,
        tag_present: true,
        raw_nbt: nbt_compound(vec![
            nbt_string("id", "minecraft:chest"),
            nbt_string("Lock", "secret"),
        ]),
    });

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]{Lock:\"secret\",id:\"minecraft:chest\"}")
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied server-side block data to clipboard"
    );
}

#[test]
fn f3_i_server_entity_tag_response_copies_recreate_command_without_uuid_or_pos() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        input.take_debug_recreate_server_query_requests(),
        vec![DebugRecreateServerQueryRequest::EntityTag {
            transaction_id: 0,
            entity_id: 50,
        }]
    );
    world.apply_tag_query(TagQuery {
        transaction_id: 99,
        tag_present: true,
        raw_nbt: nbt_compound(vec![nbt_byte("Charged", 1)]),
    });
    assert!(!input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(clipboard.text, None);
    assert!(input.pending_debug_recreate_server_query.is_some());

    world.apply_tag_query(TagQuery {
        transaction_id: 0,
        tag_present: true,
        raw_nbt: nbt_compound(vec![
            nbt_long_array("UUID", &[1, 2]),
            nbt_list(
                "Pos",
                6,
                vec![
                    0.0f64.to_be_bytes().to_vec(),
                    0.0f64.to_be_bytes().to_vec(),
                    3.0f64.to_be_bytes().to_vec(),
                ],
            ),
            nbt_byte("Charged", 1),
            nbt_string("CustomName", "Boom"),
        ]),
    });

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(
        clipboard.text.as_deref(),
        Some("/summon minecraft:creeper 0.00 0.00 3.00 {Charged: 1b, CustomName: \"Boom\"}")
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied server-side entity data to clipboard"
    );
}

#[test]
fn turtle_server_query_preserves_authoritative_home_pos() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 57,
        uuid: Uuid::from_u128(57),
        entity_type_id: VANILLA_ENTITY_TYPE_TURTLE_ID,
        position: ProtocolVec3d {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    input.pending_debug_recreate_server_query = Some(PendingDebugRecreateServerQuery {
        transaction_id: 9,
        client_level_generation: world.client_level_generation(),
        target: PendingDebugRecreateServerQueryTarget::Entity {
            entity_id: 57,
            entity_uuid: Uuid::from_u128(57),
            entity_type: "minecraft:turtle".to_string(),
            position: [4.0, 5.0, 6.0],
        },
    });
    world.apply_tag_query(TagQuery {
        transaction_id: 9,
        tag_present: true,
        raw_nbt: nbt_compound(vec![
            nbt_long_array("UUID", &[1, 2]),
            nbt_list(
                "Pos",
                6,
                vec![
                    4.0f64.to_be_bytes().to_vec(),
                    5.0f64.to_be_bytes().to_vec(),
                    6.0f64.to_be_bytes().to_vec(),
                ],
            ),
            nbt_int_array("home_pos", &[12, 63, -8]),
            nbt_byte("has_egg", 1),
        ]),
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:turtle 4.00 5.00 6.00 \
             {home_pos: [I; 12, 63, -8], has_egg: 1b}"
        )
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    assert_eq!(
        world.client_chat().messages[0].content,
        "[Debug]: Copied server-side entity data to clipboard"
    );
}

#[test]
fn f3_i_null_server_tag_response_reports_failure_without_copying() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    input.take_debug_recreate_server_query_requests();
    world.apply_tag_query(TagQuery {
        transaction_id: 0,
        tag_present: false,
        raw_nbt: vec![0],
    });

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(clipboard.text, None);
    assert_eq!(input.pending_debug_recreate_server_query, None);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Server returned no recreate data"
    );
}

#[test]
fn f3_i_entity_disappearing_before_matching_response_does_not_copy() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 51,
        uuid: Uuid::from_u128(51),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    input.pending_debug_recreate_server_query = Some(PendingDebugRecreateServerQuery {
        transaction_id: 7,
        client_level_generation: world.client_level_generation(),
        target: PendingDebugRecreateServerQueryTarget::Entity {
            entity_id: 51,
            entity_uuid: Uuid::from_u128(51),
            entity_type: "minecraft:creeper".to_string(),
            position: [1.0, 2.0, 3.0],
        },
    });
    assert_eq!(
        world.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![51],
        }),
        1
    );
    world.apply_add_entity(AddEntity {
        id: 51,
        uuid: Uuid::from_u128(52),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.apply_tag_query(TagQuery {
        transaction_id: 7,
        tag_present: true,
        raw_nbt: nbt_compound(vec![nbt_byte("Charged", 1)]),
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(clipboard.text, None);
    assert_eq!(input.pending_debug_recreate_server_query, None);
    assert_eq!(
        world.client_chat().messages[0].content,
        "[Debug]: Debug recreate target disappeared before server response"
    );
}

#[test]
fn f3_i_rejects_matching_response_and_unsent_request_after_client_level_change() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let entity = AddEntity {
        id: 51,
        uuid: Uuid::from_u128(51),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    };
    world.apply_add_entity(entity.clone());
    let old_generation = world.client_level_generation();
    let pending = PendingDebugRecreateServerQuery {
        transaction_id: 7,
        client_level_generation: old_generation,
        target: PendingDebugRecreateServerQueryTarget::Entity {
            entity_id: 51,
            entity_uuid: Uuid::from_u128(51),
            entity_type: "minecraft:creeper".to_string(),
            position: [1.0, 2.0, 3.0],
        },
    };
    input.pending_debug_recreate_server_query = Some(pending.clone());
    input
        .debug_recreate_server_query_requests
        .push(DebugRecreateServerQueryRequest::EntityTag {
            transaction_id: 7,
            entity_id: 51,
        });

    world.clear_client_level();
    assert!(world.client_level_generation() > old_generation);
    assert!(input
        .take_debug_recreate_server_query_requests_for_level(world.client_level_generation())
        .is_empty());
    assert_eq!(input.pending_debug_recreate_server_query, None);

    world.apply_add_entity(entity);
    input.pending_debug_recreate_server_query = Some(pending);
    world.apply_tag_query(TagQuery {
        transaction_id: 7,
        tag_present: true,
        raw_nbt: nbt_compound(vec![nbt_byte("Charged", 1)]),
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(clipboard.text, None);
    assert_eq!(input.pending_debug_recreate_server_query, None);
    assert_eq!(
        world.client_chat().messages[0].content,
        "[Debug]: Debug recreate level changed before server response"
    );
}

#[test]
fn queues_debug_recreate_server_query_requests_as_tag_query_commands() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();

    queue_debug_recreate_server_query_request(
        &mut counters,
        &commands,
        DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id: 3,
            pos: ProtocolBlockPos { x: -1, y: 2, z: 7 },
        },
    );
    queue_debug_recreate_server_query_request(
        &mut counters,
        &commands,
        DebugRecreateServerQueryRequest::EntityTag {
            transaction_id: 4,
            entity_id: 88,
        },
    );

    assert_eq!(counters.block_entity_tag_query_commands_queued, 1);
    assert_eq!(counters.entity_tag_query_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::BlockEntityTagQuery(BlockEntityTagQuery {
            transaction_id: 3,
            pos: ProtocolBlockPos { x: -1, y: 2, z: 7 },
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::EntityTagQuery(EntityTagQuery {
            transaction_id: 4,
            entity_id: 88,
        })
    );
}

fn nbt_compound(entries: Vec<Vec<u8>>) -> Vec<u8> {
    let mut out = vec![10];
    for entry in entries {
        out.extend_from_slice(&entry);
    }
    out.push(0);
    out
}

fn nbt_byte(name: &str, value: i8) -> Vec<u8> {
    let mut out = vec![1];
    write_mutf8(&mut out, name);
    out.push(value as u8);
    out
}

fn nbt_string(name: &str, value: &str) -> Vec<u8> {
    let mut out = vec![8];
    write_mutf8(&mut out, name);
    write_mutf8(&mut out, value);
    out
}

fn nbt_list(name: &str, element_type: u8, values: Vec<Vec<u8>>) -> Vec<u8> {
    let mut out = vec![9];
    write_mutf8(&mut out, name);
    out.push(element_type);
    out.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value);
    }
    out
}

fn nbt_long_array(name: &str, values: &[i64]) -> Vec<u8> {
    let mut out = vec![12];
    write_mutf8(&mut out, name);
    out.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value.to_be_bytes());
    }
    out
}

fn nbt_int_array(name: &str, values: &[i32]) -> Vec<u8> {
    let mut out = vec![11];
    write_mutf8(&mut out, name);
    out.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value.to_be_bytes());
    }
    out
}

fn write_mutf8(out: &mut Vec<u8>, value: &str) {
    let mut bytes = Vec::new();
    for unit in value.encode_utf16() {
        if unit == 0 {
            bytes.extend_from_slice(&[0xc0, 0x80]);
        } else if unit <= 0x7f {
            bytes.push(unit as u8);
        } else if unit <= 0x7ff {
            bytes.push((0xc0 | ((unit >> 6) & 0x1f)) as u8);
            bytes.push((0x80 | (unit & 0x3f)) as u8);
        } else {
            bytes.push((0xe0 | ((unit >> 12) & 0x0f)) as u8);
            bytes.push((0x80 | ((unit >> 6) & 0x3f)) as u8);
            bytes.push((0x80 | (unit & 0x3f)) as u8);
        }
    }
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(&bytes);
}

#[test]
fn shift_f3_i_copies_entity_recreate_command_to_clipboard_and_reports_feedback() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/summon minecraft:creeper 0.00 0.00 3.00")
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn shift_f3_i_with_permission_copies_local_creeper_save_nbt_to_clipboard() {
    let accessors = creeper_metadata_accessors();
    let entity_accessors = entity_metadata_accessors();
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.25,
            y: -0.5,
            z: 0.75,
        },
        x_rot: 10.0,
        y_rot: 45.0,
        y_head_rot: 45.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: entity_accessors.shared_flags_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(entity_accessors.glowing_mask),
            },
            ProtocolEntityDataValue {
                data_id: entity_accessors.air_supply_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(123),
            },
            ProtocolEntityDataValue {
                data_id: entity_accessors.custom_name_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
                value: EntityDataValueKind::OptionalComponent(Some(
                    bbb_protocol::RichTextComponent::plain("Bob \"Prime\""),
                )),
            },
            ProtocolEntityDataValue {
                data_id: entity_accessors.custom_name_visible_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: entity_accessors.silent_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: entity_accessors.no_gravity_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: entity_accessors.ticks_frozen_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(42),
            },
            mob_flags_data(&world, 50, true, true),
            ProtocolEntityDataValue {
                data_id: accessors.powered_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: accessors.ignited_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:creeper 0.00 0.00 3.00 \
             {Motion: [0.25d, -0.5d, 0.75d], Rotation: [45.0f, 10.0f], \
             Air: 123s, \
             CustomName: 'Bob \"Prime\"', \
             CustomNameVisible: 1b, Silent: 1b, NoGravity: 1b, Glowing: 1b, TicksFrozen: 42, \
             LeftHanded: 1b, NoAI: 1b, \
             powered: 1b, ignited: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_mob_leash_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.apply_add_entity(AddEntity {
        id: 60,
        uuid: Uuid::from_u128(60),
        entity_type_id: VANILLA_ENTITY_TYPE_COW_ID,
        position: ProtocolVec3d {
            x: 8.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_link(SetEntityLink {
        source_id: 50,
        dest_id: 60,
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:creeper 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             leash: {UUID: [I; 0, 0, 0, 60]}, powered: 0b, ignited: 0b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
}

#[test]
fn shift_f3_i_with_permission_copies_local_creaking_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREAKING_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, false, true),
            ProtocolEntityDataValue {
                data_id: creaking_home_pos_data_id(),
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                value: EntityDataValueKind::OptionalBlockPos(Some(ProtocolBlockPos {
                    x: 4,
                    y: 5,
                    z: 6,
                })),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:creaking 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, home_pos: [I; 4, 5, 6]}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_shulker_save_nbt_to_clipboard() {
    let accessors = shulker_metadata_accessors();
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_SHULKER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, false, true),
            ProtocolEntityDataValue {
                data_id: accessors.attach_face_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
                value: EntityDataValueKind::Direction(5),
            },
            ProtocolEntityDataValue {
                data_id: accessors.peek_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(30),
            },
            ProtocolEntityDataValue {
                data_id: accessors.color_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(11),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.5,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:shulker 0.50 1.00 2.50 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, AttachFace: 5b, Peek: 30b, \
             Color: 11b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_slime_family_save_nbt_to_clipboard() {
    let mut matched_members = 0;
    for entity_type in VANILLA_ENTITY_TYPES {
        let entity_type_id = entity_type.id;
        if !bbb_world::vanilla_entity_metadata_type_is_slime(entity_type_id) {
            continue;
        }
        matched_members += 1;
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 50,
            uuid: Uuid::from_u128(50),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 50,
            values: vec![
                mob_flags_data(&world, 50, false, true),
                slime_size_data(entity_type_id, 4),
            ],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        let mut clipboard = MockDebugClipboard::accepting();
        input.set_shift_key(KeyCode::ShiftLeft, true);

        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));
        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::KeyI),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));

        let expected = format!(
            "/summon {} 0.00 0.00 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, Size: 3}}",
            entity_type.resource_id
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
    assert_ne!(
        matched_members, 0,
        "generated Slime class has no entity types"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_snow_golem_save_nbt_to_clipboard() {
    let (pumpkin_data_id, _pumpkin_default, _pumpkin_mask) = snow_golem_pumpkin_accessor();
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, false, true),
            ProtocolEntityDataValue {
                data_id: pumpkin_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(0),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:snow_golem 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, Pumpkin: 0b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_bat_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_BAT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, false, true),
            bat_flags_data(&world, 50),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:bat 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, BatFlags: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_armadillo_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_ARMADILLO_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.35,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, true, true),
            age_locked_data(&world, 50, true),
            armadillo_state_data(ARMADILLO_STATE_SCARED_ID),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:armadillo 0.00 1.35 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, state: \"scared\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_copper_golem_save_nbt_to_clipboard() {
    let accessors = vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    )
    .expect("generated CopperGolem metadata accessors");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, true, true),
            ProtocolEntityDataValue {
                data_id: accessors.weather_state_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_WEATHERING_COPPER_STATE_ID,
                value: EntityDataValueKind::EnumId {
                    serializer: EntityDataEnumSerializer::WeatheringCopperState,
                    id: COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID,
                },
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:copper_golem 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             weather_state: \"weathered\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_bee_save_nbt_to_clipboard() {
    let bee_accessors = bbb_world::vanilla_entity_metadata_bee_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_BEE_ID,
    )
    .expect("generated Bee metadata accessors");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_BEE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.35,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, true, true),
            age_locked_data(&world, 50, true),
            ProtocolEntityDataValue {
                data_id: bee_accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(
                    bee_accessors.has_nectar_mask | bee_accessors.has_stung_mask,
                ),
            },
            ProtocolEntityDataValue {
                data_id: bee_accessors.anger_end_time_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                value: EntityDataValueKind::Long(240),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:bee 0.00 1.35 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, HasNectar: 1b, HasStung: 1b, \
             anger_end_time: 240L}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_blaze_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_BLAZE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![mob_flags_data(&world, 50, true, true)],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:blaze 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_breeze_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_BREEZE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![mob_flags_data(&world, 50, true, true)],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:breeze 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_cod_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 52,
        uuid: Uuid::from_u128(52),
        entity_type_id: VANILLA_ENTITY_TYPE_COD_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.45,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 52,
        values: vec![
            mob_flags_data(&world, 52, true, true),
            abstract_fish_from_bucket_data(VANILLA_ENTITY_TYPE_COD_ID, true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:cod 0.00 1.45 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, FromBucket: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_pufferfish_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 53,
        uuid: Uuid::from_u128(53),
        entity_type_id: VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 53,
        values: vec![
            mob_flags_data(&world, 53, true, true),
            abstract_fish_from_bucket_data(VANILLA_ENTITY_TYPE_PUFFERFISH_ID, true),
            pufferfish_puff_state_data(2),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:pufferfish 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             FromBucket: 1b, PuffState: 2}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_salmon_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 54,
        uuid: Uuid::from_u128(54),
        entity_type_id: VANILLA_ENTITY_TYPE_SALMON_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.2,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 54,
        values: vec![
            mob_flags_data(&world, 54, true, true),
            abstract_fish_from_bucket_data(VANILLA_ENTITY_TYPE_SALMON_ID, true),
            salmon_variant_data(2),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:salmon 0.00 1.20 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             FromBucket: 1b, type: \"large\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_tropical_fish_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 55,
        uuid: Uuid::from_u128(55),
        entity_type_id: VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.3,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![
            mob_flags_data(&world, 55, true, true),
            abstract_fish_from_bucket_data(VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID, true),
            tropical_fish_variant_data(0x0405_0001),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:tropical_fish 0.00 1.30 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             FromBucket: 1b, Variant: 67436545}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_turtle_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_TURTLE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.35,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    let turtle_accessors = turtle_metadata_accessors(&world, 50);
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, true, true),
            age_locked_data(&world, 50, true),
            ProtocolEntityDataValue {
                data_id: turtle_accessors.has_egg_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:turtle 0.00 1.35 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, \
             home_pos: [I; 0, 0, 0], \
             has_egg: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_tadpole_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 56,
        uuid: Uuid::from_u128(56),
        entity_type_id: VANILLA_ENTITY_TYPE_TADPOLE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.4,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 56,
        values: vec![
            mob_flags_data(&world, 56, true, true),
            age_locked_data(&world, 56, true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:tadpole 0.00 1.40 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_axolotl_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 57,
        uuid: Uuid::from_u128(57),
        entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 57,
        values: vec![
            mob_flags_data(&world, 57, true, true),
            baby_data(VANILLA_ENTITY_TYPE_AXOLOTL_ID, true),
            age_locked_data(&world, 57, true),
            axolotl_variant_data(9),
            axolotl_from_bucket_data(true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:axolotl 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, Variant: 0, FromBucket: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_nautilus_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 88,
        uuid: Uuid::from_u128(88),
        entity_type_id: VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 88,
        values: vec![
            mob_flags_data(&world, 88, true, true),
            baby_data(VANILLA_ENTITY_TYPE_NAUTILUS_ID, true),
            age_locked_data(&world, 88, true),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_NAUTILUS_ID, false, true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:nautilus 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, Sitting: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_zombie_nautilus_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let (zombie_nautilus_variant_data_id, _) =
        vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        )
        .expect("generated ZombieNautilus variant metadata accessor must resolve");
    grant_debug_recreate_nbt_permission(&mut world);
    world.record_registry_entries(
        "minecraft:zombie_nautilus_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:warm"),
            RegistryPacketEntry::stub("minecraft:temperate"),
        ],
    );
    world.apply_add_entity(AddEntity {
        id: 89,
        uuid: Uuid::from_u128(89),
        entity_type_id: VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 89,
        values: vec![
            mob_flags_data(&world, 89, true, true),
            age_locked_data(&world, 89, true),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, false, true),
            ProtocolEntityDataValue {
                data_id: zombie_nautilus_variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ZOMBIE_NAUTILUS_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::ZombieNautilusVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:zombie_nautilus 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, Sitting: 1b, \
             variant: \"minecraft:warm\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn local_zombie_nautilus_save_uses_generated_default() {
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 89,
        uuid: Uuid::from_u128(89),
        entity_type_id: VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    let zombie_nautilus = debug_copy_recreate_entity_command(&world, None, 89, true)
        .expect("local zombie nautilus recreate command");
    assert!(zombie_nautilus
        .command
        .contains("variant: \"minecraft:temperate\""));
}

#[test]
fn shift_f3_i_with_permission_copies_local_dolphin_save_nbt_to_clipboard() {
    let accessors = bbb_world::vanilla_entity_metadata_dolphin_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    )
    .expect("generated Dolphin metadata accessors");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 66,
        uuid: Uuid::from_u128(66),
        entity_type_id: VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 66,
        values: vec![
            mob_flags_data(&world, 66, true, true),
            baby_data(VANILLA_ENTITY_TYPE_DOLPHIN_ID, true),
            age_locked_data(&world, 66, true),
            ProtocolEntityDataValue {
                data_id: accessors.got_fish_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: accessors.moistness_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(1234),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:dolphin 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, GotFish: 1b, Moistness: 1234}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_frog_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let (frog_variant_data_id, _) =
        vanilla_entity_metadata_frog_variant_accessor_for_entity_type(VANILLA_ENTITY_TYPE_FROG_ID)
            .expect("generated Frog variant metadata accessor must resolve");
    grant_debug_recreate_nbt_permission(&mut world);
    world.record_registry_entries(
        "minecraft:frog_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:cold"),
            RegistryPacketEntry::stub("minecraft:temperate"),
            RegistryPacketEntry::stub("minecraft:warm"),
        ],
    );
    world.apply_add_entity(AddEntity {
        id: 67,
        uuid: Uuid::from_u128(67),
        entity_type_id: VANILLA_ENTITY_TYPE_FROG_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 67,
        values: vec![
            mob_flags_data(&world, 67, true, true),
            baby_data(VANILLA_ENTITY_TYPE_FROG_ID, true),
            age_locked_data(&world, 67, true),
            ProtocolEntityDataValue {
                data_id: frog_variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FROG_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::FrogVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:frog 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, variant: \"minecraft:cold\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn local_frog_save_uses_generated_default() {
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 67,
        uuid: Uuid::from_u128(67),
        entity_type_id: VANILLA_ENTITY_TYPE_FROG_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    let frog = debug_copy_recreate_entity_command(&world, None, 67, true)
        .expect("local frog recreate command");
    assert!(frog.command.contains("variant: \"minecraft:temperate\""));
}

#[test]
fn shift_f3_i_with_permission_copies_local_rabbit_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 58,
        uuid: Uuid::from_u128(58),
        entity_type_id: VANILLA_ENTITY_TYPE_RABBIT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.3,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 58,
        values: vec![
            mob_flags_data(&world, 58, true, true),
            baby_data(VANILLA_ENTITY_TYPE_RABBIT_ID, true),
            age_locked_data(&world, 58, true),
            rabbit_type_data(99),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:rabbit 0.00 1.30 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, RabbitType: 99}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_ocelot_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 59,
        uuid: Uuid::from_u128(59),
        entity_type_id: VANILLA_ENTITY_TYPE_OCELOT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.3,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 59,
        values: vec![
            mob_flags_data(&world, 59, true, true),
            baby_data(VANILLA_ENTITY_TYPE_OCELOT_ID, true),
            age_locked_data(&world, 59, true),
            ocelot_trusting_data(true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:ocelot 0.00 1.30 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, Trusting: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_sheep_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 60,
        uuid: Uuid::from_u128(60),
        entity_type_id: VANILLA_ENTITY_TYPE_SHEEP_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.2,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![
            mob_flags_data(&world, 60, true, true),
            baby_data(VANILLA_ENTITY_TYPE_SHEEP_ID, true),
            age_locked_data(&world, 60, true),
            sheep_wool_data(14, true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:sheep 0.00 1.20 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, Sheared: 1b, Color: 14b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_pig_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let farm_accessors = farm_animal_variant_metadata_accessors(VANILLA_ENTITY_TYPE_PIG_ID);
    grant_debug_recreate_nbt_permission(&mut world);
    world.record_registry_entries(
        "minecraft:pig_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:cold"),
            RegistryPacketEntry::stub("minecraft:temperate"),
            RegistryPacketEntry::stub("minecraft:warm"),
        ],
    );
    world.record_registry_entries(
        "minecraft:pig_sound_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:big"),
            RegistryPacketEntry::stub("minecraft:classic"),
            RegistryPacketEntry::stub("minecraft:mini"),
        ],
    );
    world.apply_add_entity(AddEntity {
        id: 61,
        uuid: Uuid::from_u128(61),
        entity_type_id: VANILLA_ENTITY_TYPE_PIG_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.25,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![
            mob_flags_data(&world, 61, true, true),
            baby_data(VANILLA_ENTITY_TYPE_PIG_ID, true),
            age_locked_data(&world, 61, true),
            ProtocolEntityDataValue {
                data_id: farm_accessors.variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_PIG_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::PigVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
            ProtocolEntityDataValue {
                data_id: farm_accessors.sound_variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_PIG_SOUND_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::PigSoundVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:pig 0.00 1.25 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, variant: \"minecraft:cold\", \
             sound_variant: \"minecraft:big\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn debug_registry_resource_ids_use_the_value_local_session_identity() {
    let mut fallback = EntityDataRegistryReference::unresolved(1);
    fallback.bind(EntityDataRegistryIdentity::new(
        "minecraft:cat_sound_variant",
        "minecraft:royal",
        EntityDataRegistrySource::VanillaFallback,
        EntityDataRegistryRevision::default(),
    ));
    assert_eq!(
        debug_registry_resource_id(&fallback, "minecraft:cat_sound_variant"),
        Some("minecraft:royal")
    );

    let mut synchronized = EntityDataRegistryReference::unresolved(0);
    synchronized.bind(EntityDataRegistryIdentity::new(
        "minecraft:cat_sound_variant",
        "example:custom",
        EntityDataRegistrySource::Server,
        EntityDataRegistryRevision {
            session: 2,
            contents: 7,
            tags: 3,
        },
    ));
    assert_eq!(
        debug_registry_resource_id(&synchronized, "minecraft:cat_sound_variant"),
        Some("example:custom")
    );
    assert_eq!(
        debug_registry_resource_id(&synchronized, "minecraft:cat_variant"),
        None
    );
}

#[test]
fn local_pig_save_uses_generated_defaults() {
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 64,
        uuid: Uuid::from_u128(64),
        entity_type_id: VANILLA_ENTITY_TYPE_PIG_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    let pig = debug_copy_recreate_entity_command(&world, None, 64, true)
        .expect("local pig recreate command");
    assert!(pig.command.contains("variant: \"minecraft:temperate\""));
    assert!(pig.command.contains("sound_variant: \"minecraft:classic\""));
}

#[test]
fn shift_f3_i_with_permission_copies_local_chicken_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let farm_accessors = farm_animal_variant_metadata_accessors(VANILLA_ENTITY_TYPE_CHICKEN_ID);
    grant_debug_recreate_nbt_permission(&mut world);
    world.record_registry_entries(
        "minecraft:chicken_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:warm"),
            RegistryPacketEntry::stub("minecraft:temperate"),
            RegistryPacketEntry::stub("minecraft:cold"),
        ],
    );
    world.record_registry_entries(
        "minecraft:chicken_sound_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:picky"),
            RegistryPacketEntry::stub("minecraft:classic"),
        ],
    );
    world.apply_add_entity(AddEntity {
        id: 62,
        uuid: Uuid::from_u128(62),
        entity_type_id: VANILLA_ENTITY_TYPE_CHICKEN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.25,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 62,
        values: vec![
            mob_flags_data(&world, 62, true, true),
            baby_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, true),
            age_locked_data(&world, 62, true),
            ProtocolEntityDataValue {
                data_id: farm_accessors.variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::ChickenVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
            ProtocolEntityDataValue {
                data_id: farm_accessors.sound_variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_SOUND_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::ChickenSoundVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:chicken 0.00 1.25 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, variant: \"minecraft:warm\", \
             sound_variant: \"minecraft:picky\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn local_chicken_save_uses_generated_resource_defaults() {
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_CHICKEN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    let copy = debug_copy_recreate_entity_command(&world, None, 63, true)
        .expect("local chicken recreate command");
    assert!(copy.command.contains("variant: \"minecraft:temperate\""));
    assert!(copy
        .command
        .contains("sound_variant: \"minecraft:classic\""));
    assert!(!copy.command.contains("EggLayTime"));
    assert!(!copy.command.contains("IsChickenJockey"));
}

#[test]
fn shift_f3_i_with_permission_copies_local_cow_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let farm_accessors = farm_animal_variant_metadata_accessors(VANILLA_ENTITY_TYPE_COW_ID);
    grant_debug_recreate_nbt_permission(&mut world);
    world.record_registry_entries(
        "minecraft:cow_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:cold"),
            RegistryPacketEntry::stub("minecraft:temperate"),
            RegistryPacketEntry::stub("minecraft:warm"),
        ],
    );
    world.record_registry_entries(
        "minecraft:cow_sound_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:moody"),
            RegistryPacketEntry::stub("minecraft:classic"),
        ],
    );
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_COW_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 63,
        values: vec![
            mob_flags_data(&world, 63, true, true),
            baby_data(VANILLA_ENTITY_TYPE_COW_ID, true),
            age_locked_data(&world, 63, true),
            ProtocolEntityDataValue {
                data_id: farm_accessors.variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_COW_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::CowVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
            ProtocolEntityDataValue {
                data_id: farm_accessors.sound_variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_COW_SOUND_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::CowSoundVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:cow 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, variant: \"minecraft:cold\", \
             sound_variant: \"minecraft:moody\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn local_cow_branch_save_uses_generated_defaults() {
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 64,
        uuid: Uuid::from_u128(64),
        entity_type_id: VANILLA_ENTITY_TYPE_COW_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.apply_add_entity(AddEntity {
        id: 65,
        uuid: Uuid::from_u128(65),
        entity_type_id: VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    let cow = debug_copy_recreate_entity_command(&world, None, 64, true)
        .expect("local cow recreate command");
    assert!(cow.command.contains("variant: \"minecraft:temperate\""));
    assert!(cow.command.contains("sound_variant: \"minecraft:classic\""));
    let mushroom_cow = debug_copy_recreate_entity_command(&world, None, 65, true)
        .expect("local mushroom cow recreate command");
    assert!(mushroom_cow.command.contains("Type: \"red\""));
}

#[test]
fn shift_f3_i_with_permission_copies_local_mooshroom_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 64,
        uuid: Uuid::from_u128(64),
        entity_type_id: VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 64,
        values: vec![
            mob_flags_data(&world, 64, true, true),
            baby_data(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, true),
            age_locked_data(&world, 64, true),
            mushroom_cow_variant_data(1),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:mooshroom 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, Type: \"brown\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_goat_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let goat_accessors = goat_metadata_accessors();
    assert!(!goat_accessors.screaming_default);
    assert!(goat_accessors.left_horn_default);
    assert!(goat_accessors.right_horn_default);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 65,
        uuid: Uuid::from_u128(65),
        entity_type_id: VANILLA_ENTITY_TYPE_GOAT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 65,
        values: vec![
            mob_flags_data(&world, 65, true, true),
            baby_data(VANILLA_ENTITY_TYPE_GOAT_ID, true),
            age_locked_data(&world, 65, true),
            ProtocolEntityDataValue {
                data_id: goat_accessors.screaming_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: goat_accessors.left_horn_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(false),
            },
            ProtocolEntityDataValue {
                data_id: goat_accessors.right_horn_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:goat 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, IsScreamingGoat: 1b, \
             HasLeftHorn: 0b, HasRightHorn: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_polar_bear_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let (standing_data_id, _) =
        bbb_world::vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
        )
        .expect("generated PolarBear standing accessor must resolve");
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 68,
        uuid: Uuid::from_u128(68),
        entity_type_id: VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 68,
        values: vec![
            mob_flags_data(&world, 68, true, true),
            baby_data(VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, true),
            age_locked_data(&world, 68, true),
            ProtocolEntityDataValue {
                data_id: standing_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:polar_bear 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_panda_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors =
        vanilla_entity_metadata_panda_accessors_for_entity_type(VANILLA_ENTITY_TYPE_PANDA_ID)
            .expect("generated Panda metadata accessors must resolve");
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 69,
        uuid: Uuid::from_u128(69),
        entity_type_id: VANILLA_ENTITY_TYPE_PANDA_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 69,
        values: vec![
            mob_flags_data(&world, 69, true, true),
            baby_data(VANILLA_ENTITY_TYPE_PANDA_ID, true),
            age_locked_data(&world, 69, true),
            ProtocolEntityDataValue {
                data_id: accessors.unhappy_counter_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(12),
            },
            ProtocolEntityDataValue {
                data_id: accessors.eat_counter_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(4),
            },
            ProtocolEntityDataValue {
                data_id: accessors.main_gene_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(4),
            },
            ProtocolEntityDataValue {
                data_id: accessors.hidden_gene_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(1),
            },
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(
                    accessors.rolling_mask | accessors.sitting_mask | accessors.on_back_mask,
                ),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:panda 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, MainGene: \"brown\", \
             HiddenGene: \"lazy\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_parrot_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 71,
        uuid: Uuid::from_u128(71),
        entity_type_id: VANILLA_ENTITY_TYPE_PARROT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            // Keep the standing player's horizontal eye ray inside the parrot's generated baby
            // bounds (adult 0.5 × 0.9, scaled by AgeableMob's 0.5 age scale).
            y: 1.2,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 71,
        values: vec![
            mob_flags_data(&world, 71, true, true),
            baby_data(VANILLA_ENTITY_TYPE_PARROT_ID, true),
            age_locked_data(&world, 71, true),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_PARROT_ID, false, true),
            parrot_variant_data(3),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:parrot 0.00 1.20 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, Sitting: 1b, Variant: 3}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_allay_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 71,
        uuid: Uuid::from_u128(71),
        entity_type_id: VANILLA_ENTITY_TYPE_ALLAY_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.1,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 71,
        values: vec![
            mob_flags_data(&world, 71, true, true),
            allay_dancing_data(true),
            allay_can_duplicate_data(false),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:allay 0.00 1.10 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_sniffer_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 70,
        uuid: Uuid::from_u128(70),
        entity_type_id: VANILLA_ENTITY_TYPE_SNIFFER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![
            mob_flags_data(&world, 70, true, true),
            sniffer_baby_data(true),
            age_locked_data(&world, 70, true),
            sniffer_state_data(VANILLA_SNIFFER_STATE_DIGGING_ID),
            sniffer_drop_seed_at_tick_data(&world, 70, 1234),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:sniffer 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_strider_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 72,
        uuid: Uuid::from_u128(72),
        entity_type_id: VANILLA_ENTITY_TYPE_STRIDER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 72,
        values: vec![
            mob_flags_data(&world, 72, true, true),
            baby_data(VANILLA_ENTITY_TYPE_STRIDER_ID, true),
            age_locked_data(&world, 72, true),
            strider_boost_time_data(40),
            ProtocolEntityDataValue {
                data_id: strider_suffocating_data_id(),
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:strider 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_hoglin_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 73,
        uuid: Uuid::from_u128(73),
        entity_type_id: VANILLA_ENTITY_TYPE_HOGLIN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 73,
        values: vec![
            mob_flags_data(&world, 73, true, true),
            baby_data(VANILLA_ENTITY_TYPE_HOGLIN_ID, true),
            age_locked_data(&world, 73, true),
            ProtocolEntityDataValue {
                data_id: zombification_immunity_data_id(VANILLA_ENTITY_TYPE_HOGLIN_ID),
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:hoglin 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b, \
             IsImmuneToZombification: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_horse_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors = abstract_horse_metadata_accessors(VANILLA_ENTITY_TYPE_HORSE_ID);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 76,
        uuid: Uuid::from_u128(76),
        entity_type_id: VANILLA_ENTITY_TYPE_HORSE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.25,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 76,
        values: vec![
            mob_flags_data(&world, 76, true, true),
            age_locked_data(&world, 76, true),
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(
                    accessors.eating_mask | accessors.bred_mask | accessors.tame_mask,
                ),
            },
            horse_variant_data(VANILLA_ENTITY_TYPE_HORSE_ID, 4 | (2 << 8)),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:horse 0.00 0.25 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, EatingHaystack: 1b, \
             Bred: 1b, Tame: 1b, Variant: 516}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_camel_family_save_nbt_to_clipboard() {
    let mut matched_members = 0;
    for entity_type in VANILLA_ENTITY_TYPES {
        let entity_type_id = entity_type.id;
        if !bbb_world::vanilla_entity_metadata_type_is_camel(entity_type_id) {
            continue;
        }
        matched_members += 1;
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        let accessors = abstract_horse_metadata_accessors(entity_type_id);
        let (last_pose_change_tick_data_id, _) =
            vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
                entity_type_id,
            )
            .expect("generated Camel last-pose-change accessor must resolve");
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 79,
            uuid: Uuid::from_u128(79),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 79,
            values: vec![
                mob_flags_data(&world, 79, true, true),
                age_locked_data(&world, 79, true),
                ProtocolEntityDataValue {
                    data_id: accessors.flags_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(accessors.tame_mask),
                },
                ProtocolEntityDataValue {
                    data_id: last_pose_change_tick_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                    value: EntityDataValueKind::Long(140),
                },
            ],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        let mut clipboard = MockDebugClipboard::accepting();
        input.set_shift_key(KeyCode::ShiftLeft, true);

        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));
        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::KeyI),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));

        let expected = format!(
            "/summon {} 0.00 0.00 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, EatingHaystack: 0b, \
             Bred: 0b, Tame: 1b, LastPoseTick: 140L}}",
            entity_type.resource_id
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
    assert_ne!(
        matched_members, 0,
        "generated Camel class has no entity types"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_cat_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors =
        vanilla_entity_metadata_cat_accessors_for_entity_type(VANILLA_ENTITY_TYPE_CAT_ID)
            .expect("generated Cat metadata accessors must resolve");
    let variant_accessors = vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_CAT_ID,
    )
    .expect("generated Cat variant metadata accessors must resolve");
    grant_debug_recreate_nbt_permission(&mut world);
    world.record_registry_entries(
        "minecraft:cat_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:calico"),
            RegistryPacketEntry::stub("minecraft:black"),
        ],
    );
    world.record_registry_entries(
        "minecraft:cat_sound_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:royal"),
            RegistryPacketEntry::stub("minecraft:classic"),
        ],
    );
    world.apply_add_entity(AddEntity {
        id: 85,
        uuid: Uuid::from_u128(85),
        entity_type_id: VANILLA_ENTITY_TYPE_CAT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 85,
        values: vec![
            mob_flags_data(&world, 85, true, true),
            age_locked_data(&world, 85, true),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_CAT_ID, false, true),
            ProtocolEntityDataValue {
                data_id: variant_accessors.variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_CAT_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::CatVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
            ProtocolEntityDataValue {
                data_id: variant_accessors.sound_variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_CAT_SOUND_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::CatSoundVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
            ProtocolEntityDataValue {
                data_id: accessors.collar_color_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(5),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:cat 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, Sitting: 1b, \
             variant: \"minecraft:calico\", sound_variant: \"minecraft:royal\", \
             CollarColor: 5b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn local_cat_save_uses_generated_resource_and_collar_defaults() {
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 86,
        uuid: Uuid::from_u128(86),
        entity_type_id: VANILLA_ENTITY_TYPE_CAT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    let copy = debug_copy_recreate_entity_command(&world, None, 86, true)
        .expect("local cat recreate command");
    assert!(copy.command.contains("variant: \"minecraft:black\""));
    assert!(copy
        .command
        .contains("sound_variant: \"minecraft:classic\""));
    assert!(copy.command.contains("CollarColor: 14b"));
}

#[test]
fn shift_f3_i_with_permission_copies_local_wolf_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors =
        vanilla_entity_metadata_wolf_accessors_for_entity_type(VANILLA_ENTITY_TYPE_WOLF_ID)
            .expect("generated Wolf metadata accessors must resolve");
    let variant_accessors = vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_WOLF_ID,
    )
    .expect("generated Wolf variant metadata accessors must resolve");
    grant_debug_recreate_nbt_permission(&mut world);
    world.record_registry_entries(
        "minecraft:wolf_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:striped"),
            RegistryPacketEntry::stub("minecraft:pale"),
        ],
    );
    world.record_registry_entries(
        "minecraft:wolf_sound_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:big"),
            RegistryPacketEntry::stub("minecraft:classic"),
        ],
    );
    world.apply_add_entity(AddEntity {
        id: 86,
        uuid: Uuid::from_u128(86),
        entity_type_id: VANILLA_ENTITY_TYPE_WOLF_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 86,
        values: vec![
            mob_flags_data(&world, 86, true, true),
            age_locked_data(&world, 86, true),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, false, true),
            ProtocolEntityDataValue {
                data_id: accessors.collar_color_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(11),
            },
            ProtocolEntityDataValue {
                data_id: accessors.anger_end_time_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                value: EntityDataValueKind::Long(200),
            },
            ProtocolEntityDataValue {
                data_id: variant_accessors.variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_WOLF_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::WolfVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
            ProtocolEntityDataValue {
                data_id: variant_accessors.sound_variant_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_WOLF_SOUND_VARIANT_ID,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::WolfSoundVariant,
                    value: EntityDataRegistryReference::unresolved(0),
                },
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:wolf 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, Sitting: 1b, \
             CollarColor: 11b, variant: \"minecraft:striped\", \
             anger_end_time: 200L, sound_variant: \"minecraft:big\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn local_wolf_save_uses_generated_resource_collar_and_anger_defaults() {
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 87,
        uuid: Uuid::from_u128(87),
        entity_type_id: VANILLA_ENTITY_TYPE_WOLF_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    let copy = debug_copy_recreate_entity_command(&world, None, 87, true)
        .expect("local wolf recreate command");
    assert!(copy.command.contains("CollarColor: 14b"));
    assert!(copy.command.contains("variant: \"minecraft:pale\""));
    assert!(copy.command.contains("anger_end_time: -1L"));
    assert!(copy
        .command
        .contains("sound_variant: \"minecraft:classic\""));
}

#[test]
fn shift_f3_i_with_permission_copies_local_fox_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors =
        vanilla_entity_metadata_fox_accessors_for_entity_type(VANILLA_ENTITY_TYPE_FOX_ID)
            .expect("generated Fox metadata accessors must resolve");
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 87,
        uuid: Uuid::from_u128(87),
        entity_type_id: VANILLA_ENTITY_TYPE_FOX_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 87,
        values: vec![
            mob_flags_data(&world, 87, true, true),
            age_locked_data(&world, 87, true),
            ProtocolEntityDataValue {
                data_id: accessors.type_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(1),
            },
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(
                    accessors.sitting_mask | accessors.crouching_mask | accessors.sleeping_mask,
                ),
            },
            ProtocolEntityDataValue {
                data_id: accessors.trusted_data_ids[0],
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
                value: EntityDataValueKind::OptionalLivingEntityReference(Some(Uuid::from_u128(
                    0x00000001000000020000000300000004
                ),)),
            },
            ProtocolEntityDataValue {
                data_id: accessors.trusted_data_ids[1],
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
                value: EntityDataValueKind::OptionalLivingEntityReference(Some(Uuid::from_u128(
                    0x00000005000000060000000700000008
                ),)),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:fox 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, \
             Trusted: [[I; 1, 2, 3, 4], [I; 5, 6, 7, 8]], Sleeping: 1b, \
             Type: \"snow\", Sitting: 1b, Crouching: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn debug_save_data_abstract_chested_horse_class_dispatch_uses_generated_membership() {
    let mut matched_members = 0;
    for entity_type in VANILLA_ENTITY_TYPES {
        let entity_type_id = entity_type.id;
        if !bbb_world::vanilla_entity_metadata_type_is_abstract_chested_horse(entity_type_id) {
            continue;
        }
        matched_members += 1;
        let horse_accessors = abstract_horse_metadata_accessors(entity_type_id);
        let (chest_data_id, _) =
            vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type(entity_type_id)
                .expect("generated AbstractChestedHorse chest accessor must resolve");
        let llama_accessors =
            vanilla_entity_metadata_llama_accessors_for_entity_type(entity_type_id);
        let mut world = world_with_debug_player(false);
        world.apply_add_entity(AddEntity {
            id: 81,
            uuid: Uuid::from_u128(81),
            entity_type_id,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        let mut values = vec![
            mob_flags_data(&world, 81, true, true),
            age_locked_data(&world, 81, true),
            ProtocolEntityDataValue {
                data_id: horse_accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(
                    horse_accessors.eating_mask
                        | horse_accessors.bred_mask
                        | horse_accessors.tame_mask,
                ),
            },
            ProtocolEntityDataValue {
                data_id: chest_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ];
        if let Some(llama_accessors) = llama_accessors {
            values.extend([
                ProtocolEntityDataValue {
                    data_id: llama_accessors.strength_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    value: EntityDataValueKind::Int(5),
                },
                ProtocolEntityDataValue {
                    data_id: llama_accessors.variant_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    value: EntityDataValueKind::Int(2),
                },
            ]);
        }
        assert!(world.apply_set_entity_data(ProtocolSetEntityData { id: 81, values }));

        let entity = world.probe_entity(81).expect("catalog entity must exist");
        let mut fields = Vec::new();
        debug_push_entity_additional_save_data(&world, None, &entity, &mut fields);
        let mut expected = vec![
            "LeftHanded: 1b".to_string(),
            "NoAI: 1b".to_string(),
            "AgeLocked: 1b".to_string(),
            "EatingHaystack: 1b".to_string(),
            "Bred: 1b".to_string(),
            "Tame: 1b".to_string(),
            "ChestedHorse: 1b".to_string(),
        ];
        if llama_accessors.is_some() {
            expected.extend(["Variant: 2".to_string(), "Strength: 5".to_string()]);
        }
        assert_eq!(fields, expected, "{} save chain", entity_type.resource_id);
    }
    assert_ne!(
        matched_members, 0,
        "generated AbstractChestedHorse class has no entity types"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_donkey_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors = abstract_horse_metadata_accessors(VANILLA_ENTITY_TYPE_DONKEY_ID);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 81,
        uuid: Uuid::from_u128(81),
        entity_type_id: VANILLA_ENTITY_TYPE_DONKEY_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.25,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 81,
        values: vec![
            mob_flags_data(&world, 81, true, true),
            age_locked_data(&world, 81, true),
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.eating_mask | accessors.tame_mask),
            },
            chested_horse_chest_data(VANILLA_ENTITY_TYPE_DONKEY_ID, true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:donkey 0.00 0.25 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, EatingHaystack: 1b, \
             Bred: 0b, Tame: 1b, ChestedHorse: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_mule_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors = abstract_horse_metadata_accessors(VANILLA_ENTITY_TYPE_MULE_ID);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 82,
        uuid: Uuid::from_u128(82),
        entity_type_id: VANILLA_ENTITY_TYPE_MULE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.25,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![
            mob_flags_data(&world, 82, true, true),
            age_locked_data(&world, 82, true),
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.bred_mask),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:mule 0.00 0.25 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, EatingHaystack: 0b, \
             Bred: 1b, Tame: 0b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_llama_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors = abstract_horse_metadata_accessors(VANILLA_ENTITY_TYPE_LLAMA_ID);
    let llama_accessors = llama_metadata_accessors(VANILLA_ENTITY_TYPE_LLAMA_ID);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 83,
        uuid: Uuid::from_u128(83),
        entity_type_id: VANILLA_ENTITY_TYPE_LLAMA_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.25,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 83,
        values: vec![
            mob_flags_data(&world, 83, true, true),
            age_locked_data(&world, 83, true),
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.bred_mask | accessors.tame_mask),
            },
            chested_horse_chest_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true),
            ProtocolEntityDataValue {
                data_id: llama_accessors.strength_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(5),
            },
            ProtocolEntityDataValue {
                data_id: llama_accessors.variant_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(2),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:llama 0.00 0.25 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, EatingHaystack: 0b, \
             Bred: 1b, Tame: 1b, ChestedHorse: 1b, \
             Variant: 2, Strength: 5}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_trader_llama_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors = abstract_horse_metadata_accessors(VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID);
    let llama_accessors = llama_metadata_accessors(VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 84,
        uuid: Uuid::from_u128(84),
        entity_type_id: VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.25,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 84,
        values: vec![
            mob_flags_data(&world, 84, true, true),
            age_locked_data(&world, 84, true),
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.tame_mask),
            },
            ProtocolEntityDataValue {
                data_id: llama_accessors.strength_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(4),
            },
            ProtocolEntityDataValue {
                data_id: llama_accessors.variant_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(3),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:trader_llama 0.00 0.25 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, EatingHaystack: 0b, \
             Bred: 0b, Tame: 1b, Variant: 3, \
             Strength: 4}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_skeleton_horse_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors = abstract_horse_metadata_accessors(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 77,
        uuid: Uuid::from_u128(77),
        entity_type_id: VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.25,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 77,
        values: vec![
            mob_flags_data(&world, 77, true, true),
            age_locked_data(&world, 77, true),
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.tame_mask),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:skeleton_horse 0.00 0.25 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, EatingHaystack: 0b, \
             Bred: 0b, Tame: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_zombie_horse_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors = abstract_horse_metadata_accessors(VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 78,
        uuid: Uuid::from_u128(78),
        entity_type_id: VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.25,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 78,
        values: vec![
            mob_flags_data(&world, 78, true, true),
            age_locked_data(&world, 78, true),
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.bred_mask),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:zombie_horse 0.00 0.25 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, EatingHaystack: 0b, \
             Bred: 1b, Tame: 0b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_piglin_brute_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 74,
        uuid: Uuid::from_u128(74),
        entity_type_id: VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 74,
        values: vec![
            mob_flags_data(&world, 74, true, true),
            ProtocolEntityDataValue {
                data_id: zombification_immunity_data_id(VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID),
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:piglin_brute 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             IsImmuneToZombification: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_piglin_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 75,
        uuid: Uuid::from_u128(75),
        entity_type_id: VANILLA_ENTITY_TYPE_PIGLIN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 75,
        values: vec![
            mob_flags_data(&world, 75, true, true),
            ProtocolEntityDataValue {
                data_id: zombification_immunity_data_id(VANILLA_ENTITY_TYPE_PIGLIN_ID),
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: piglin_baby_data_id(),
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:piglin 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             IsImmuneToZombification: 1b, \
             IsBaby: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_spider_family_save_nbt_to_clipboard() {
    for (entity_type_id, entity_type) in [
        (VANILLA_ENTITY_TYPE_SPIDER_ID, "minecraft:spider"),
        (VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, "minecraft:cave_spider"),
    ] {
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 50,
            uuid: Uuid::from_u128(50),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.25,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 50,
            values: vec![mob_flags_data(&world, 50, true, true)],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        let mut clipboard = MockDebugClipboard::accepting();
        input.set_shift_key(KeyCode::ShiftLeft, true);

        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));
        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::KeyI),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));

        let expected = format!(
            "/summon {entity_type} 0.00 1.25 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}}"
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
}

#[test]
fn shift_f3_i_with_permission_copies_local_skeleton_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 55,
        uuid: Uuid::from_u128(55),
        entity_type_id: VANILLA_ENTITY_TYPE_SKELETON_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.25,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![mob_flags_data(&world, 55, true, true)],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:skeleton 0.00 1.25 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_nonconverting_skeleton_family_save_nbt_to_clipboard() {
    for (entity_type_id, entity_type) in [
        (VANILLA_ENTITY_TYPE_PARCHED_ID, "minecraft:parched"),
        (VANILLA_ENTITY_TYPE_STRAY_ID, "minecraft:stray"),
        (
            VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
            "minecraft:wither_skeleton",
        ),
    ] {
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 56,
            uuid: Uuid::from_u128(56),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.25,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 56,
            values: vec![mob_flags_data(&world, 56, true, true)],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        let mut clipboard = MockDebugClipboard::accepting();
        input.set_shift_key(KeyCode::ShiftLeft, true);

        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));
        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::KeyI),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));

        let expected = format!(
            "/summon {entity_type} 0.00 1.25 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}}"
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
}

#[test]
fn shift_f3_i_with_permission_copies_local_zombie_family_save_nbt_to_clipboard() {
    for (entity_type_id, entity_type) in [
        (VANILLA_ENTITY_TYPE_ZOMBIE_ID, "minecraft:zombie"),
        (VANILLA_ENTITY_TYPE_HUSK_ID, "minecraft:husk"),
        (VANILLA_ENTITY_TYPE_DROWNED_ID, "minecraft:drowned"),
    ] {
        let accessors = vanilla_entity_metadata_zombie_accessors_for_entity_type(entity_type_id)
            .expect("generated Zombie metadata accessors");
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 57,
            uuid: Uuid::from_u128(57),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.25,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 57,
            values: vec![
                mob_flags_data(&world, 57, true, true),
                ProtocolEntityDataValue {
                    data_id: accessors.baby_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: EntityDataValueKind::Boolean(true),
                },
            ],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        let mut clipboard = MockDebugClipboard::accepting();
        input.set_shift_key(KeyCode::ShiftLeft, true);

        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));
        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::KeyI),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));

        let expected = format!(
            "/summon {entity_type} 0.00 1.25 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             IsBaby: 1b}}"
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
}

#[test]
fn shift_f3_i_with_permission_copies_local_zombified_piglin_save_nbt_to_clipboard() {
    let accessors = vanilla_entity_metadata_zombie_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
    )
    .expect("generated ZombifiedPiglin inherited Zombie metadata accessors");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 58,
        uuid: Uuid::from_u128(58),
        entity_type_id: VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.25,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 58,
        values: vec![
            mob_flags_data(&world, 58, true, true),
            ProtocolEntityDataValue {
                data_id: accessors.baby_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:zombified_piglin 0.00 1.25 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             IsBaby: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_bogged_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_BOGGED_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, false, true),
            bogged_sheared_data(true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:bogged 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, sheared: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_end_crystal_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    let end_crystal_accessors = end_crystal_metadata_accessors(&world, 50);
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: end_crystal_accessors.beam_target_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                value: EntityDataValueKind::OptionalBlockPos(Some(ProtocolBlockPos {
                    x: 1,
                    y: 2,
                    z: 3,
                })),
            },
            ProtocolEntityDataValue {
                data_id: end_crystal_accessors.show_bottom_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(false),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:end_crystal 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             beam_target: [I; 1, 2, 3], \
             ShowBottom: 0b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_skips_unpickable_evoker_fangs_but_direct_copy_encodes_save_nbt() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 51,
        uuid: Uuid::from_u128(51),
        entity_type_id: VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_unpickable_debug_entity_copy(
        &mut input,
        &world,
        &clipboard,
        51,
        "/summon minecraft:evoker_fangs 0.00 1.50 2.00 \
         {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f]}",
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_fireball_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 52,
        uuid: Uuid::from_u128(52),
        entity_type_id: VANILLA_ENTITY_TYPE_FIREBALL_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_projectile_power(ProtocolProjectilePower {
        entity_id: 52,
        acceleration_power: 0.75,
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:fireball 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             HasBeenShot: 0b, \
             acceleration_power: 0.75d, Item: {id: \"minecraft:fire_charge\"}}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn local_projectile_save_tracks_spawn_owner_first_tick_and_default_item_stack() {
    let mut world = WorldStore::new();
    world.apply_add_entity(AddEntity {
        id: 70,
        uuid: Uuid::from_u128(70),
        entity_type_id: VANILLA_ENTITY_TYPE_COW_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    let cases = [
        (
            VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
            "minecraft:fire_charge",
        ),
        (VANILLA_ENTITY_TYPE_EGG_ID, "minecraft:egg"),
        (VANILLA_ENTITY_TYPE_ENDER_PEARL_ID, "minecraft:ender_pearl"),
        (
            VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID,
            "minecraft:experience_bottle",
        ),
        (
            VANILLA_ENTITY_TYPE_LINGERING_POTION_ID,
            "minecraft:lingering_potion",
        ),
        (VANILLA_ENTITY_TYPE_SNOWBALL_ID, "minecraft:snowball"),
        (
            VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
            "minecraft:splash_potion",
        ),
    ];
    for (index, (entity_type_id, _)) in cases.iter().enumerate() {
        let id = 71 + index as i32;
        world.apply_add_entity(AddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id: *entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 70,
        });
    }

    for (index, (_, item_id)) in cases.iter().enumerate() {
        let entity = world.probe_entity(71 + index as i32).unwrap();
        let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
        assert!(snbt.contains("Owner: [I; 0, 0, 0, 70], HasBeenShot: 0b"));
        assert!(snbt.contains(&format!("Item: {{id: \"{item_id}\"}}")));
    }

    world.advance_entity_client_animations(1);
    for index in 0..cases.len() {
        let entity = world.probe_entity(71 + index as i32).unwrap();
        let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
        assert!(snbt.contains("Owner: [I; 0, 0, 0, 70], HasBeenShot: 1b"));
        assert!(!snbt.contains("LeftOwner"));
    }

    assert_eq!(
        world.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![70],
        }),
        1
    );
    world.advance_entity_client_animations(1);
    for index in 0..cases.len() {
        let entity = world.probe_entity(71 + index as i32).unwrap();
        let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
        assert!(snbt.contains("Owner: [I; 0, 0, 0, 70], LeftOwner: 1b, HasBeenShot: 1b"));
    }
}

#[test]
fn local_firework_save_uses_parent_lifecycle_synced_item_codec_and_client_life() {
    let accessors = bbb_world::vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    )
    .expect("generated FireworkRocket metadata accessors");
    let registry: ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": ["minecraft:firework_rocket"],
        "protocol_ids": {"minecraft:firework_rocket": 42}
    }))
    .unwrap();
    let item_runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        EquipmentAssetCatalog::default(),
    );
    let mut world = WorldStore::new();
    world.apply_add_entity(AddEntity {
        id: 90,
        uuid: Uuid::from_u128(90),
        entity_type_id: VANILLA_ENTITY_TYPE_COW_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.apply_add_entity(AddEntity {
        id: 91,
        uuid: Uuid::from_u128(91),
        entity_type_id: VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 90,
    });
    let entity = world.probe_entity(91).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity).unwrap();
    assert!(snbt.contains("Owner: [I; 0, 0, 0, 90], HasBeenShot: 0b, Life: 0"));
    assert!(!snbt.contains("LifeTime"));
    assert!(!snbt.contains("FireworksItem"));
    assert!(!snbt.contains("ShotAtAngle"));

    let mut component_patch = DataComponentPatchSummary::default();
    component_patch.added = 1;
    component_patch.added_type_ids = vec![69];
    component_patch.fireworks_flight_duration = Some(2);
    component_patch.fireworks_explosions_count = Some(1);
    component_patch.fireworks_explosions = vec![FireworkExplosionSummary {
        shape: FireworkExplosionShapeSummary::Star,
        colors: vec![0x11_22_33, 0x44_55_66],
        fade_colors: vec![0x77_88_99],
        has_trail: true,
        has_twinkle: true,
    }];
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![
            ProtocolEntityDataValue {
                data_id: accessors.item_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                value: EntityDataValueKind::ItemStack(ProtocolItemStackSummary {
                    item_id: Some(42),
                    count: 2,
                    component_patch,
                }),
            },
            ProtocolEntityDataValue {
                data_id: accessors.shot_at_angle_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));

    let entity = world.probe_entity(91).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity).unwrap();
    assert!(snbt.contains(
        "Owner: [I; 0, 0, 0, 90], HasBeenShot: 0b, Life: 0, \
         FireworksItem: {id: \"minecraft:firework_rocket\", count: 2, components: \
         {\"minecraft:fireworks\": {flight_duration: 2, explosions: \
         [{shape: \"star\", colors: [1122867, 4478310], fade_colors: [7833753], \
         has_trail: 1b, has_twinkle: 1b}]}}}, ShotAtAngle: 1b"
    ));

    let removed_fireworks_patch = DataComponentPatchSummary {
        removed_type_ids: vec![69],
        ..DataComponentPatchSummary::default()
    };
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.item_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(ProtocolItemStackSummary {
                item_id: Some(42),
                count: 1,
                component_patch: removed_fireworks_patch,
            }),
        }],
    }));
    let entity = world.probe_entity(91).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity).unwrap();
    assert!(snbt.contains(
        "FireworksItem: {id: \"minecraft:firework_rocket\", components: \
         {\"!minecraft:fireworks\": {}}}, ShotAtAngle: 1b"
    ));

    world.advance_entity_client_animations(1);
    let entity = world.probe_entity(91).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity).unwrap();
    assert!(snbt.contains("Owner: [I; 0, 0, 0, 90], HasBeenShot: 1b, Life: 1"));
    assert!(!snbt.contains("LifeTime"));
}

#[test]
fn range_checked_float_component_snbt_matches_vanilla_float_comparison() {
    assert_eq!(
        debug_range_checked_float_component_snbt(0.0_f32.to_bits(), 0.0, Some(1.0)),
        Some("0.0f".to_string())
    );
    assert_eq!(
        debug_range_checked_float_component_snbt(1.0_f32.to_bits(), 0.0, Some(1.0)),
        Some("1.0f".to_string())
    );
    assert_eq!(
        debug_range_checked_float_component_snbt((-0.0_f32).to_bits(), 0.0, Some(1.0)),
        None
    );
    assert_eq!(
        debug_range_checked_float_component_snbt(f32::NAN.to_bits(), 0.0, None),
        None
    );
    assert_eq!(
        debug_range_checked_float_component_snbt(f32::INFINITY.to_bits(), 0.0, None),
        None
    );
}

#[test]
fn item_stack_snbt_validates_and_omits_default_weapon_fields() {
    let mut patch = DataComponentPatchSummary {
        weapon_item_damage_per_attack: Some(1),
        weapon_disable_blocking_seconds_bits: Some(0.0_f32.to_bits()),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(debug_weapon_component_snbt(&patch), Some("{}".to_string()));

    patch.weapon_item_damage_per_attack = Some(-1);
    assert_eq!(debug_weapon_component_snbt(&patch), None);

    patch.weapon_item_damage_per_attack = Some(1);
    patch.weapon_disable_blocking_seconds_bits = Some((-0.0_f32).to_bits());
    assert_eq!(debug_weapon_component_snbt(&patch), None);

    patch.weapon_disable_blocking_seconds_bits = Some(f32::INFINITY.to_bits());
    assert_eq!(debug_weapon_component_snbt(&patch), None);
}

#[test]
fn item_stack_snbt_preserves_effect_free_consumables_and_omits_defaults() {
    let mut patch = DataComponentPatchSummary {
        consumable: Some(bbb_protocol::packets::ConsumableSummary {
            consume_seconds: 1.6,
            animation: bbb_protocol::packets::ItemUseAnimationSummary::Eat,
        }),
        consumable_sound: Some(bbb_protocol::packets::SoundEventSummary {
            registry_id: Some(696),
            sound_id: None,
            fixed_range_bits: None,
        }),
        consumable_has_consume_particles: Some(true),
        consumable_effect_count: Some(0),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        debug_consumable_component_snbt(&patch),
        Some("{}".to_string())
    );

    patch.consumable_effect_count = Some(1);
    assert_eq!(debug_consumable_component_snbt(&patch), None);

    patch.consumable_effect_count = Some(0);
    patch.consumable.as_mut().unwrap().consume_seconds = -0.0;
    assert_eq!(debug_consumable_component_snbt(&patch), None);
}

#[test]
fn item_stack_snbt_preserves_vanilla_consume_effects() {
    let effects = vec![
        ConsumeEffectSummary::ApplyEffects {
            effects: vec![MobEffectInstanceSummary {
                effect_id: VANILLA_MOB_EFFECT_REGENERATION_ID,
                amplifier: 1,
                duration: 200,
                ambient: false,
                show_particles: true,
                show_icon: true,
                hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                    amplifier: 0,
                    duration: 40,
                    ambient: true,
                    show_particles: false,
                    show_icon: false,
                    hidden_effect: None,
                })),
            }],
            probability_bits: 0.75_f32.to_bits(),
        },
        ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: Vec::new(),
            effect_tag: Some("minecraft:test_effects".to_string()),
        },
        ConsumeEffectSummary::ClearAllEffects,
        ConsumeEffectSummary::TeleportRandomly {
            diameter_bits: 16.0_f32.to_bits(),
        },
        ConsumeEffectSummary::PlaySound {
            sound: bbb_protocol::packets::SoundEventSummary {
                registry_id: None,
                sound_id: Some("minecraft:item.honey_bottle.drink".to_string()),
                fixed_range_bits: None,
            },
        },
    ];
    let mut patch = DataComponentPatchSummary {
        consumable: Some(bbb_protocol::packets::ConsumableSummary {
            consume_seconds: 1.6,
            animation: bbb_protocol::packets::ItemUseAnimationSummary::Eat,
        }),
        consumable_sound: Some(bbb_protocol::packets::SoundEventSummary {
            registry_id: Some(696),
            sound_id: None,
            fixed_range_bits: None,
        }),
        consumable_has_consume_particles: Some(true),
        consumable_effect_count: Some(effects.len()),
        consumable_effects: effects.clone(),
        death_protection_effects: effects,
        ..DataComponentPatchSummary::default()
    };
    let effect_list = "[{type: \"minecraft:apply_effects\", effects: \
        [{id: \"minecraft:regeneration\", amplifier: 1, duration: 200, \
        show_icon: 1b, hidden_effect: {duration: 40, ambient: 1b, \
        show_particles: 0b, show_icon: 0b}}], probability: 0.75f}, \
        {type: \"minecraft:remove_effects\", effects: \
        \"#minecraft:test_effects\"}, \
        {type: \"minecraft:clear_all_effects\"}, \
        {type: \"minecraft:teleport_randomly\"}, \
        {type: \"minecraft:play_sound\", sound: \
        {sound_id: \"minecraft:item.honey_bottle.drink\"}}]";
    assert_eq!(
        debug_consumable_component_snbt(&patch),
        Some(format!("{{on_consume_effects: {effect_list}}}"))
    );
    assert_eq!(
        debug_death_protection_component_snbt(&patch),
        Some(format!("{{death_effects: {effect_list}}}"))
    );

    patch.consumable_effects[3] = ConsumeEffectSummary::TeleportRandomly {
        diameter_bits: 0.0_f32.to_bits(),
    };
    assert_eq!(debug_consumable_component_snbt(&patch), None);

    patch.consumable_effects[3] = ConsumeEffectSummary::TeleportRandomly {
        diameter_bits: 16.0_f32.to_bits(),
    };
    patch.consumable_effects[0] = ConsumeEffectSummary::ApplyEffects {
        effects: Vec::new(),
        probability_bits: (-0.0_f32).to_bits(),
    };
    assert_eq!(debug_consumable_component_snbt(&patch), None);

    let unknown_effect_id = i32::try_from(VANILLA_MOB_EFFECT_TYPES.len()).unwrap();
    patch.death_protection_effects = vec![ConsumeEffectSummary::ApplyEffects {
        effects: vec![MobEffectInstanceSummary {
            effect_id: unknown_effect_id,
            amplifier: 0,
            duration: 0,
            ambient: false,
            show_particles: true,
            show_icon: true,
            hidden_effect: None,
        }],
        probability_bits: 1.0_f32.to_bits(),
    }];
    assert_eq!(debug_death_protection_component_snbt(&patch), None);
}

#[test]
fn item_stack_snbt_preserves_potion_contents_and_omits_unknown_potions() {
    let mut patch = DataComponentPatchSummary {
        potion_custom_effect_count: Some(0),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        debug_potion_contents_component_snbt(&patch),
        Some("{}".to_string())
    );

    patch.potion_id = Some(46);
    assert_eq!(debug_potion_contents_component_snbt(&patch), None);

    patch.potion_id = None;
    patch.potion_custom_effect_count = Some(1);
    assert_eq!(debug_potion_contents_component_snbt(&patch), None);
}

#[test]
fn item_stack_snbt_projects_supported_component_patch_in_wire_order() {
    let registry: ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": [
            "minecraft:angler_pottery_sherd",
            "minecraft:archer_pottery_sherd",
            "minecraft:arrow",
            "minecraft:firework_star"
        ],
        "protocol_ids": {
            "minecraft:angler_pottery_sherd": 0,
            "minecraft:archer_pottery_sherd": 1,
            "minecraft:arrow": 2,
            "minecraft:firework_star": 3
        }
    }))
    .unwrap();
    let item_runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        EquipmentAssetCatalog::default(),
    );
    let unknown_dye_color_id = i32::try_from(VANILLA_DYE_COLOR_VALUES.len()).unwrap();
    let patch = DataComponentPatchSummary {
        added: 107,
        added_type_ids: vec![
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 42, 43, 44, 45, 46, 47,
            49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70,
            71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92,
            93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109,
        ],
        removed_type_ids: Vec::new(),
        custom_data: Some(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "answer".to_string(),
                value: NbtSummaryValue::Int(42),
            },
            NbtSummaryEntry {
                name: "quoted key".to_string(),
                value: NbtSummaryValue::String("A 'diamond'".to_string()),
            },
            NbtSummaryEntry {
                name: "arrays".to_string(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "bytes".to_string(),
                        value: NbtSummaryValue::ByteArray(vec![-1, 2]),
                    },
                    NbtSummaryEntry {
                        name: "ints".to_string(),
                        value: NbtSummaryValue::IntArray(vec![3, 4]),
                    },
                    NbtSummaryEntry {
                        name: "longs".to_string(),
                        value: NbtSummaryValue::LongArray(vec![5, 6]),
                    },
                ]),
            },
            NbtSummaryEntry {
                name: "numbers".to_string(),
                value: NbtSummaryValue::List(vec![
                    NbtSummaryValue::Double(f64::INFINITY.to_bits()),
                    NbtSummaryValue::Double((-0.0_f64).to_bits()),
                ]),
            },
            NbtSummaryEntry {
                name: "nan".to_string(),
                value: NbtSummaryValue::Float(f32::NAN.to_bits()),
            },
            NbtSummaryEntry {
                name: "short".to_string(),
                value: NbtSummaryValue::Short(7),
            },
            NbtSummaryEntry {
                name: "long".to_string(),
                value: NbtSummaryValue::Long(8),
            },
            NbtSummaryEntry {
                name: "byte".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
        ])),
        max_stack_size: Some(64),
        max_damage: Some(100),
        damage: Some(7),
        damage_type_id: Some(49),
        custom_name: Some("Custom sword".to_string()),
        custom_name_nbt: Some(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Custom sword".to_string()),
            },
            NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
        ])),
        item_name: Some("Diamond".to_string()),
        item_name_nbt: Some(NbtSummaryValue::String("Diamond".to_string())),
        lore: vec!["Forged".to_string(), "Sharp".to_string()],
        lore_nbt: vec![
            NbtSummaryValue::String("Forged".to_string()),
            NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "text".to_string(),
                    value: NbtSummaryValue::String("Sharp".to_string()),
                },
                NbtSummaryEntry {
                    name: "italic".to_string(),
                    value: NbtSummaryValue::Byte(0),
                },
            ]),
        ],
        unbreakable: true,
        use_effects: Some(bbb_protocol::packets::UseEffectsSummary {
            can_sprint: true,
            interact_vibrations: false,
            speed_multiplier: 1.0,
        }),
        minimum_attack_charge_bits: Some(0.75_f32.to_bits()),
        item_model: Some("minecraft:custom/diamond".to_string()),
        rarity: Some(ItemRaritySummary::Rare),
        enchantments: vec![
            bbb_protocol::packets::ItemEnchantmentSummary {
                // `Enchantments.bootstrap`: DENSITY is holder id 38 in 26.1.
                holder_id: 38,
                level: 4,
            },
            bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 12,
                level: 3,
            },
        ],
        can_place_on: Some(bbb_protocol::packets::AdventureModePredicateSummary {
            unknown: false,
            block_registry_ids: vec![1],
            unresolved_block_tag_count: 0,
        }),
        can_place_on_persistent: Some(bbb_protocol::packets::AdventureModePredicateDataSummary {
            predicates: vec![bbb_protocol::packets::BlockPredicateSummary {
                blocks: Some(bbb_protocol::packets::BlockHolderSetSummary {
                    registry_ids: vec![1],
                    tag: None,
                }),
                exact_components: Some(Box::new(DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![1],
                    max_stack_size: Some(16),
                    ..DataComponentPatchSummary::default()
                })),
                ..Default::default()
            }],
        }),
        can_break: Some(bbb_protocol::packets::AdventureModePredicateSummary {
            unknown: true,
            block_registry_ids: Vec::new(),
            unresolved_block_tag_count: 1,
        }),
        can_break_persistent: Some(bbb_protocol::packets::AdventureModePredicateDataSummary {
            predicates: vec![
                bbb_protocol::packets::BlockPredicateSummary::default(),
                bbb_protocol::packets::BlockPredicateSummary {
                    blocks: Some(bbb_protocol::packets::BlockHolderSetSummary {
                        registry_ids: Vec::new(),
                        tag: Some("minecraft:mineable/pickaxe".to_string()),
                    }),
                    ..Default::default()
                },
            ],
        }),
        attribute_modifiers: vec![
            bbb_protocol::packets::AttributeModifierSummary {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                modifier_id: "minecraft:base_attack_damage".to_string(),
                amount_bits: 3.5_f64.to_bits(),
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                slot_id: 0,
                display_id: 0,
                display_text: None,
                display_text_styled: None,
                display_text_nbt: None,
            },
            bbb_protocol::packets::AttributeModifierSummary {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
                modifier_id: "minecraft:test/hidden".to_string(),
                amount_bits: (-0.25_f64).to_bits(),
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                slot_id: 1,
                display_id: 1,
                display_text: None,
                display_text_styled: None,
                display_text_nbt: None,
            },
        ],
        custom_model_data_floats: vec![1.0, -0.0, f32::INFINITY, f32::NAN].into(),
        custom_model_data_flags: vec![true, false],
        custom_model_data_strings: vec!["variant".to_string(), "A 'quote'".to_string()],
        custom_model_data_colors: vec![0x11_22_33, 0x44_55_66],
        tooltip_hide_tooltip: true,
        tooltip_hidden_component_type_ids: vec![13, 16, 20],
        repair_cost: Some(5),
        enchantment_glint_override: Some(false),
        intangible_projectile: true,
        food_nutrition: Some(6),
        food_saturation_bits: Some(7.2_f32.to_bits()),
        food_can_always_eat: true,
        consumable: Some(bbb_protocol::packets::ConsumableSummary {
            consume_seconds: 0.8,
            animation: bbb_protocol::packets::ItemUseAnimationSummary::Drink,
        }),
        consumable_sound: Some(bbb_protocol::packets::SoundEventSummary {
            registry_id: None,
            sound_id: Some("minecraft:entity.generic.drink".to_string()),
            fixed_range_bits: None,
        }),
        consumable_has_consume_particles: Some(false),
        consumable_effect_count: Some(5),
        consumable_effects: vec![
            ConsumeEffectSummary::ApplyEffects {
                effects: vec![MobEffectInstanceSummary {
                    effect_id: VANILLA_MOB_EFFECT_REGENERATION_ID,
                    amplifier: 0,
                    duration: 100,
                    ambient: false,
                    show_particles: true,
                    show_icon: true,
                    hidden_effect: None,
                }],
                probability_bits: 1.0_f32.to_bits(),
            },
            ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids: vec![18, 19],
                effect_tag: None,
            },
            ConsumeEffectSummary::ClearAllEffects,
            ConsumeEffectSummary::TeleportRandomly {
                diameter_bits: 8.0_f32.to_bits(),
            },
            ConsumeEffectSummary::PlaySound {
                sound: bbb_protocol::packets::SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("minecraft:item.honey_bottle.drink".to_string()),
                    fixed_range_bits: None,
                },
            },
        ],
        use_remainder: Some(Box::new(bbb_protocol::packets::ItemStackTemplateSummary {
            item_id: 2,
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        })),
        use_cooldown_seconds_bits: Some(1.25_f32.to_bits()),
        use_cooldown_ticks: Some(25),
        use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
        damage_resistant_type_tag: Some("minecraft:is_fire".to_string()),
        tool: Some(bbb_protocol::packets::ToolSummary {
            rules: vec![
                bbb_protocol::packets::ToolRuleSummary {
                    block_registry_ids: Vec::new(),
                    block_tag: Some("minecraft:mineable/pickaxe".to_string()),
                    speed_bits: Some(8.0_f32.to_bits()),
                    correct_for_drops: Some(true),
                },
                bbb_protocol::packets::ToolRuleSummary {
                    block_registry_ids: vec![1, 193],
                    block_tag: None,
                    speed_bits: None,
                    correct_for_drops: Some(false),
                },
            ],
            default_mining_speed_bits: 1.5_f32.to_bits(),
            damage_per_block: 2,
            can_destroy_blocks_in_creative: false,
        }),
        weapon_item_damage_per_attack: Some(3),
        weapon_disable_blocking_seconds_bits: Some(5.0_f32.to_bits()),
        attack_range: Some(bbb_protocol::packets::AttackRangeSummary {
            min_reach: 2.0,
            max_reach: 4.5,
            min_creative_reach: 2.0,
            max_creative_reach: 6.5,
            hitbox_margin: 0.125,
            mob_factor: 0.5,
        }),
        enchantable_value: Some(15),
        equippable: Some(bbb_protocol::packets::EquippableSummary {
            slot_id: 5,
            equip_sound: bbb_protocol::packets::SoundEventSummary {
                registry_id: None,
                sound_id: Some("minecraft:item.armor.equip_generic".to_string()),
                fixed_range_bits: None,
            },
            asset_id: Some("minecraft:diamond".to_string()),
            camera_overlay: Some("minecraft:misc/pumpkinblur".to_string()),
            allowed_entities: Some(bbb_protocol::packets::EntityTypeHolderSetSummary {
                registry_ids: Vec::new(),
                tag: Some("minecraft:skeletons".to_string()),
            }),
            dispensable: true,
            swappable: false,
            damage_on_hurt: true,
            equip_on_interact: false,
            can_be_sheared: true,
            shearing_sound: bbb_protocol::packets::SoundEventSummary {
                registry_id: None,
                sound_id: Some("minecraft:item.shears.snip".to_string()),
                fixed_range_bits: None,
            },
        }),
        repairable_item_registry_ids: vec![0, 1],
        tooltip_style: Some("minecraft:rare".to_string()),
        blocks_attacks: Some(bbb_protocol::packets::BlocksAttacksSummary {
            block_delay_seconds_bits: 0.25_f32.to_bits(),
            disable_cooldown_scale_bits: 1.5_f32.to_bits(),
            damage_reductions: vec![bbb_protocol::packets::DamageReductionSummary {
                horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
                damage_types: Some(bbb_protocol::packets::DamageTypeHolderSetSummary {
                    registry_ids: Vec::new(),
                    tag: Some("minecraft:bypasses_shield".to_string()),
                }),
                base_bits: 1.0_f32.to_bits(),
                factor_bits: 0.5_f32.to_bits(),
            }],
            item_damage: bbb_protocol::packets::ItemDamageFunctionSummary {
                threshold_bits: 1.0_f32.to_bits(),
                base_bits: 0.0_f32.to_bits(),
                factor_bits: 1.0_f32.to_bits(),
            },
            bypassed_by: Some(bbb_protocol::packets::DamageTypeHolderSetSummary {
                registry_ids: vec![3],
                tag: None,
            }),
            block_sound: Some(bbb_protocol::packets::SoundEventSummary {
                registry_id: None,
                sound_id: Some("minecraft:item.shield.block".to_string()),
                fixed_range_bits: None,
            }),
            disable_sound: None,
        }),
        piercing_weapon: Some(bbb_protocol::packets::PiercingWeaponSummary {
            deals_knockback: false,
            dismounts: true,
            sound: Some(bbb_protocol::packets::SoundEventSummary {
                registry_id: None,
                sound_id: Some("minecraft:item.trident.throw".to_string()),
                fixed_range_bits: None,
            }),
            hit_sound: Some(bbb_protocol::packets::SoundEventSummary {
                registry_id: Some(7),
                sound_id: None,
                fixed_range_bits: None,
            }),
        }),
        kinetic_weapon: Some(bbb_protocol::packets::KineticWeaponSummary {
            contact_cooldown_ticks: 12,
            delay_ticks: 2,
            dismount_conditions: Some(bbb_protocol::packets::KineticWeaponConditionSummary {
                max_duration_ticks: 20,
                min_speed_bits: 0.25_f32.to_bits(),
                min_relative_speed_bits: 0.5_f32.to_bits(),
            }),
            knockback_conditions: None,
            damage_conditions: Some(bbb_protocol::packets::KineticWeaponConditionSummary {
                max_duration_ticks: 30,
                min_speed_bits: 0.0_f32.to_bits(),
                min_relative_speed_bits: 1.5_f32.to_bits(),
            }),
            forward_movement_bits: 0.2_f32.to_bits(),
            damage_multiplier_bits: 2.0_f32.to_bits(),
            sound: None,
            hit_sound: Some(bbb_protocol::packets::SoundEventSummary {
                registry_id: Some(7),
                sound_id: None,
                fixed_range_bits: None,
            }),
        }),
        swing_animation: Some(bbb_protocol::packets::SwingAnimationSummary {
            animation_type: bbb_protocol::packets::SwingAnimationTypeSummary::Stab,
            duration: 17,
        }),
        stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: 13,
            level: 5,
        }],
        dye_color_id: Some(VANILLA_DYE_COLOR_RED_ID),
        dyed_color: Some(0x11_22_33),
        map_color: Some(0x44_55_66),
        map_id: Some(42),
        codec_backed_nbt: BTreeMap::from([
            (47, NbtSummaryValue::Compound(Vec::new())),
            (
                57,
                NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:stone".to_string(),
                    value: NbtSummaryValue::String("facing".to_string()),
                }]),
            ),
            (
                59,
                NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "Health".to_string(),
                    value: NbtSummaryValue::Short(5),
                }]),
            ),
            (
                66,
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("minecraft:bread".to_string()),
                    NbtSummaryValue::String("minecraft:torch".to_string()),
                ]),
            ),
            (78, NbtSummaryValue::Compound(Vec::new())),
            (
                79,
                NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "loot_table".to_string(),
                        value: NbtSummaryValue::String(
                            "minecraft:chests/simple_dungeon".to_string(),
                        ),
                    },
                    NbtSummaryEntry {
                        name: "seed".to_string(),
                        value: NbtSummaryValue::Long(9),
                    },
                ]),
            ),
        ]),
        entity_data_entity_type_id: Some(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        entity_data_tag: Some(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "NoAI".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "Health".to_string(),
                value: NbtSummaryValue::Float(20.0_f32.to_bits()),
            },
        ])),
        block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID),
        block_entity_data_tag: Some(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "Delay".to_string(),
                value: NbtSummaryValue::Short(20),
            },
            NbtSummaryEntry {
                name: "SpawnCount".to_string(),
                value: NbtSummaryValue::Short(4),
            },
        ])),
        instrument_id: Some(5),
        provides_trim_material_id: Some(10),
        jukebox_song_id: Some(1),
        charged_projectiles_items: vec![
            bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 2,
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![5],
                    use_effects: Some(bbb_protocol::packets::UseEffectsSummary {
                        can_sprint: false,
                        interact_vibrations: true,
                        speed_multiplier: 0.2,
                    }),
                    ..DataComponentPatchSummary::default()
                },
            },
            bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 3,
                count: 2,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![3],
                    damage: Some(7),
                    ..DataComponentPatchSummary::default()
                },
            },
        ],
        bundle_contents_items: vec![bbb_protocol::packets::ItemStackTemplateSummary {
            item_id: 2,
            count: 3,
            component_patch: DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![30, 44],
                attack_range: Some(bbb_protocol::packets::AttackRangeSummary {
                    min_reach: 0.0,
                    max_reach: 3.0,
                    min_creative_reach: 0.0,
                    max_creative_reach: 5.0,
                    hitbox_margin: 0.3,
                    mob_factor: 1.0,
                }),
                dyed_color: Some(0x22_44_66),
                ..DataComponentPatchSummary::default()
            },
        }],
        bundle_contents_item_count: Some(1),
        potion_id: Some(31),
        potion_custom_color: Some(0x77_88_99),
        potion_custom_effect_count: Some(1),
        potion_custom_effects: vec![MobEffectInstanceSummary {
            effect_id: VANILLA_MOB_EFFECT_SPEED_ID,
            amplifier: 1,
            duration: 600,
            ambient: true,
            show_particles: false,
            show_icon: false,
            hidden_effect: None,
        }],
        potion_custom_name: Some("swift".to_string()),
        potion_duration_scale_bits: Some(1.5_f32.to_bits()),
        suspicious_stew_effects: vec![
            bbb_protocol::packets::SuspiciousStewEffectSummary {
                effect_id: VANILLA_MOB_EFFECT_SPEED_ID,
                duration: 160,
            },
            bbb_protocol::packets::SuspiciousStewEffectSummary {
                effect_id: VANILLA_MOB_EFFECT_POISON_ID,
                duration: 200,
            },
        ],
        writable_book_pages: vec!["first\npage".to_string(), "second raw".to_string()],
        writable_book_page_filters: vec![None, Some("second filtered".to_string())],
        written_book: Some(WrittenBookContentSummary {
            title: "Guide".to_string(),
            title_filter: Some("Filtered Guide".to_string()),
            author: "Alex".to_string(),
            generation: 2,
            pages: vec!["Chapter one".to_string(), "Raw chapter two".to_string()],
            page_filters: vec![None, Some("Filtered chapter two".to_string())],
            resolved: true,
        }),
        written_book_pages_nbt: vec![
            Some(NbtSummaryValue::String("Chapter one".to_string())),
            Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Raw chapter two".to_string()),
            }])),
        ],
        written_book_page_filters_nbt: vec![
            None,
            Some(NbtSummaryValue::String("Filtered chapter two".to_string())),
        ],
        armor_trim_material_id: Some(1),
        armor_trim_pattern_id: Some(17),
        ominous_bottle_amplifier: Some(3),
        provides_banner_pattern_tag: Some("minecraft:pattern_item/flower".to_string()),
        lodestone_target: Some(bbb_protocol::packets::LodestoneTargetSummary {
            dimension: "minecraft:overworld".to_string(),
            pos: ProtocolBlockPos {
                x: 10,
                y: 64,
                z: -4,
            },
        }),
        lodestone_tracked: Some(false),
        firework_explosion_shape: Some(FireworkExplosionShapeSummary::Burst),
        firework_explosion_colors: vec![0x77_88_99],
        firework_explosion_fade_colors: vec![0xAA_BB_CC],
        firework_explosion_has_trail: Some(true),
        firework_explosion_has_twinkle: Some(false),
        fireworks_flight_duration: Some(2),
        fireworks_explosions_count: Some(0),
        profile: Some(bbb_protocol::packets::ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(Uuid::from_u128(1)),
            name: Some("Steve".to_string()),
            properties: vec![bbb_protocol::packets::GameProfilePropertySummary {
                name: "textures".to_string(),
                value: "skin-value".to_string(),
                signature: Some("skin-signature".to_string()),
            }],
            profile_textures: None,
            skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
                body: Some(bbb_protocol::packets::ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/wide/steve".to_string(),
                    texture_path: "unused-derived-path".to_string(),
                }),
                cape: None,
                elytra: Some(bbb_protocol::packets::ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/elytra".to_string(),
                    texture_path: "unused-derived-path".to_string(),
                }),
                model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Slim),
            },
        }),
        note_block_sound: Some("minecraft:block.note_block.harp".to_string()),
        banner_pattern_layers: vec![
            bbb_protocol::packets::BannerPatternLayerSummary {
                registry_id: Some(5),
                asset_id: None,
                translation_key: None,
                color_id: VANILLA_DYE_COLOR_RED_ID,
            },
            bbb_protocol::packets::BannerPatternLayerSummary {
                registry_id: None,
                asset_id: Some("example:custom_banner/pattern".to_string()),
                translation_key: Some("block.example.banner.custom".to_string()),
                color_id: VANILLA_DYE_COLOR_LIGHT_BLUE_ID,
            },
        ],
        base_color_id: Some(VANILLA_DYE_COLOR_LIGHT_BLUE_ID),
        pot_decorations_item_ids: vec![0, 1],
        container_items: vec![bbb_protocol::packets::ItemStackTemplateSummary {
            item_id: 3,
            count: 4,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![2],
                max_damage: Some(512),
                ..DataComponentPatchSummary::default()
            },
        }],
        container_slots: vec![bbb_protocol::packets::ItemContainerSlotSummary {
            slot: 2,
            item: bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 3,
                count: 4,
                component_patch: DataComponentPatchSummary {
                    added: 2,
                    added_type_ids: vec![2, 40],
                    max_damage: Some(512),
                    swing_animation: Some(bbb_protocol::packets::SwingAnimationSummary {
                        animation_type: bbb_protocol::packets::SwingAnimationTypeSummary::Whack,
                        duration: 6,
                    }),
                    ..DataComponentPatchSummary::default()
                },
            },
        }],
        container_item_count: Some(1),
        block_state_properties: BTreeMap::from([
            ("facing".to_string(), "south".to_string()),
            ("quoted key".to_string(), "value with 'quote'".to_string()),
        ]),
        bees_present: true,
        bees_count: 1,
        bees: vec![bbb_protocol::packets::BeeOccupantSummary {
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID,
            entity_data_tag: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "HasNectar".to_string(),
                value: NbtSummaryValue::Byte(1),
            }])),
            ticks_in_hive: 40,
            min_ticks_in_hive: 600,
        }],
        tropical_fish_pattern_id: Some(VANILLA_TROPICAL_FISH_PATTERN_STRIPEY_PACKED_ID),
        tropical_fish_base_color_id: Some(VANILLA_DYE_COLOR_ORANGE_ID),
        tropical_fish_pattern_color_id: Some(unknown_dye_color_id),
        break_sound: Some(bbb_protocol::packets::SoundEventSummary {
            registry_id: Some(7),
            sound_id: None,
            fixed_range_bits: None,
        }),
        painting_variant_id: Some(50),
        villager_variant_id: Some(2),
        wolf_variant_id: Some(8),
        wolf_sound_variant_id: Some(6),
        frog_variant_id: Some(2),
        zombie_nautilus_variant_id: Some(1),
        pig_variant_id: Some(2),
        pig_sound_variant_id: Some(1),
        cow_variant_id: Some(2),
        cow_sound_variant_id: Some(1),
        chicken_variant_id: Some(2),
        chicken_sound_variant_id: Some(1),
        cat_variant_id: Some(10),
        cat_sound_variant_id: Some(1),
        builtin_enum_component_ids: BTreeMap::from([
            (
                VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
                VANILLA_DYE_COLOR_RED_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
                VANILLA_FOX_VARIANT_SNOW_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
                VANILLA_SALMON_VARIANT_LARGE_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
                VANILLA_PARROT_VARIANT_GRAY_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                VANILLA_MOOSHROOM_VARIANT_BROWN_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
                VANILLA_RABBIT_VARIANT_EVIL_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
                VANILLA_HORSE_VARIANT_DARK_BROWN_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                VANILLA_LLAMA_VARIANT_GRAY_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
                VANILLA_AXOLOTL_VARIANT_BLUE_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
                VANILLA_DYE_COLOR_BLACK_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
                VANILLA_DYE_COLOR_CYAN_ID,
            ),
            (
                VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
                VANILLA_DYE_COLOR_WHITE_ID,
            ),
        ]),
        ..DataComponentPatchSummary::default()
    };
    let stack = ProtocolItemStackSummary {
        item_id: Some(0),
        count: 2,
        component_patch: patch,
    };

    assert_eq!(
        debug_item_stack_snbt(
            "minecraft:diamond",
            stack.count,
            Some(&stack),
            Some(&item_runtime),
        ),
        Some(
            "{id: \"minecraft:diamond\", count: 2, components: {\
         \"minecraft:custom_data\": {answer: 42, \"quoted key\": \"A 'diamond'\", \
         arrays: {bytes: [B; -1B, 2B], ints: [I; 3, 4], longs: [L; 5L, 6L]}, \
         numbers: [Infinityd, -0.0d], nan: NaNf, short: 7s, long: 8L, byte: 1b}, \
         \"minecraft:max_stack_size\": 64, \"minecraft:max_damage\": 100, \
         \"minecraft:damage\": 7, \"minecraft:unbreakable\": {}, \
         \"minecraft:use_effects\": {can_sprint: 1b, interact_vibrations: 0b, \
         speed_multiplier: 1.0f}, \
         \"minecraft:custom_name\": {text: \"Custom sword\", bold: 1b}, \
         \"minecraft:minimum_attack_charge\": 0.75f, \
         \"minecraft:damage_type\": \"minecraft:mace_smash\", \
         \"minecraft:item_name\": \"Diamond\", \
         \"minecraft:item_model\": \"minecraft:custom/diamond\", \
         \"minecraft:lore\": [\"Forged\", {text: \"Sharp\", italic: 0b}], \
         \"minecraft:rarity\": \"rare\", \
         \"minecraft:enchantments\": {\"minecraft:density\": 4, \
         \"minecraft:swift_sneak\": 3}, \
         \"minecraft:can_place_on\": {blocks: \"minecraft:stone\", components: \
         {\"minecraft:max_stack_size\": 16}}, \
         \"minecraft:can_break\": [{}, {blocks: \"#minecraft:mineable/pickaxe\"}], \
         \"minecraft:attribute_modifiers\": [{type: \"minecraft:attack_damage\", \
         id: \"minecraft:base_attack_damage\", amount: 3.5d, operation: \"add_value\"}, \
         {type: \"minecraft:knockback_resistance\", id: \"minecraft:test/hidden\", \
         amount: -0.25d, operation: \"add_multiplied_total\", slot: \"mainhand\", \
         display: {type: \"hidden\"}}], \
         \"minecraft:custom_model_data\": {floats: [1.0f, -0.0f, Infinityf, NaNf], \
         flags: [1b, 0b], strings: [\"variant\", \"A 'quote'\"], \
         colors: [1122867, 4478310]}, \
         \"minecraft:tooltip_display\": {hide_tooltip: 1b, hidden_components: \
         [\"minecraft:enchantments\", \"minecraft:attribute_modifiers\", \
         \"minecraft:creative_slot_lock\"]}, \"minecraft:repair_cost\": 5, \
         \"minecraft:enchantment_glint_override\": 0b, \
         \"minecraft:intangible_projectile\": {}, \
         \"minecraft:food\": {nutrition: 6, saturation: 7.2f, can_always_eat: 1b}, \
         \"minecraft:consumable\": {consume_seconds: 0.8f, animation: \"drink\", \
         sound: {sound_id: \"minecraft:entity.generic.drink\"}, \
         has_consume_particles: 0b, on_consume_effects: \
         [{type: \"minecraft:apply_effects\", effects: \
         [{id: \"minecraft:regeneration\", duration: 100, show_icon: 1b}]}, \
         {type: \"minecraft:remove_effects\", effects: \
         [\"minecraft:poison\", \"minecraft:wither\"]}, \
         {type: \"minecraft:clear_all_effects\"}, \
         {type: \"minecraft:teleport_randomly\", diameter: 8.0f}, \
         {type: \"minecraft:play_sound\", sound: \
         {sound_id: \"minecraft:item.honey_bottle.drink\"}}]}, \
         \"minecraft:use_remainder\": {id: \"minecraft:arrow\"}, \
         \"minecraft:use_cooldown\": {seconds: 1.25f, \
         cooldown_group: \"minecraft:ender_pearl\"}, \
         \"minecraft:damage_resistant\": {types: \"#minecraft:is_fire\"}, \
         \"minecraft:tool\": {rules: [{blocks: \"#minecraft:mineable/pickaxe\", \
         speed: 8.0f, correct_for_drops: 1b}, {blocks: [\"minecraft:stone\", \
         \"minecraft:obsidian\"], correct_for_drops: 0b}], default_mining_speed: 1.5f, \
         damage_per_block: 2, can_destroy_blocks_in_creative: 0b}, \
         \"minecraft:weapon\": {item_damage_per_attack: 3, \
         disable_blocking_for_seconds: 5.0f}, \
         \"minecraft:attack_range\": {min_reach: 2.0f, max_reach: 4.5f, \
         min_creative_reach: 2.0f, max_creative_reach: 6.5f, \
         hitbox_margin: 0.125f, mob_factor: 0.5f}, \
         \"minecraft:enchantable\": {value: 15}, \
         \"minecraft:equippable\": {slot: \"offhand\", equip_sound: \
         {sound_id: \"minecraft:item.armor.equip_generic\"}, asset_id: \
         \"minecraft:diamond\", camera_overlay: \"minecraft:misc/pumpkinblur\", \
         allowed_entities: \"#minecraft:skeletons\", swappable: 0b, \
         can_be_sheared: 1b, shearing_sound: \
         {sound_id: \"minecraft:item.shears.snip\"}}, \
         \"minecraft:repairable\": {items: [\"minecraft:angler_pottery_sherd\", \
         \"minecraft:archer_pottery_sherd\"]}, \"minecraft:glider\": {}, \
         \"minecraft:tooltip_style\": \"minecraft:rare\", \
         \"minecraft:death_protection\": {}, \
         \"minecraft:blocks_attacks\": {block_delay_seconds: 0.25f, \
         disable_cooldown_scale: 1.5f, damage_reductions: \
         [{type: \"#minecraft:bypasses_shield\", base: 1.0f, factor: 0.5f}], \
         bypassed_by: \"minecraft:on_fire\", block_sound: \
         {sound_id: \"minecraft:item.shield.block\"}}, \
         \"minecraft:piercing_weapon\": {deals_knockback: 0b, dismounts: 1b, \
         sound: {sound_id: \"minecraft:item.trident.throw\"}, \
         hit_sound: \"minecraft:ambient.cave\"}, \
         \"minecraft:kinetic_weapon\": {contact_cooldown_ticks: 12, delay_ticks: 2, \
         dismount_conditions: {max_duration_ticks: 20, min_speed: 0.25f, \
         min_relative_speed: 0.5f}, damage_conditions: {max_duration_ticks: 30, \
         min_relative_speed: 1.5f}, forward_movement: 0.2f, \
         damage_multiplier: 2.0f, hit_sound: \"minecraft:ambient.cave\"}, \
         \"minecraft:swing_animation\": {type: \"stab\", duration: 17}, \
         \"minecraft:stored_enchantments\": {\"minecraft:sharpness\": 5}, \
         \"minecraft:dye\": \"red\", \"minecraft:dyed_color\": 1122867, \
         \"minecraft:map_color\": 4478310, \"minecraft:map_id\": 42, \
         \"minecraft:map_decorations\": {}, \
         \"minecraft:charged_projectiles\": [{id: \"minecraft:arrow\", \
         components: {\"minecraft:use_effects\": {}}}, \
         {id: \"minecraft:firework_star\", count: 2, \
         components: {\"minecraft:damage\": 7}}], \
         \"minecraft:bundle_contents\": [{id: \"minecraft:arrow\", count: 3, \
         components: {\"minecraft:attack_range\": {}, \
         \"minecraft:dyed_color\": 2245734}}], \
         \"minecraft:potion_contents\": {potion: \"minecraft:regeneration\", \
         custom_color: 7833753, custom_effects: [{id: \"minecraft:speed\", \
         amplifier: 1, duration: 600, ambient: 1b, show_particles: 0b, \
         show_icon: 0b}], custom_name: \"swift\"}, \
         \"minecraft:potion_duration_scale\": 1.5f, \
         \"minecraft:suspicious_stew_effects\": [{id: \"minecraft:speed\"}, \
         {id: \"minecraft:poison\", duration: 200}], \
         \"minecraft:writable_book_content\": {pages: [{raw: \"first\\npage\"}, \
         {raw: \"second raw\", filtered: \"second filtered\"}]}, \
         \"minecraft:written_book_content\": {title: {raw: \"Guide\", filtered: \
         \"Filtered Guide\"}, author: \"Alex\", generation: 2, pages: \
         [{raw: \"Chapter one\"}, {raw: {text: \"Raw chapter two\"}, filtered: \
         \"Filtered chapter two\"}], resolved: 1b}, \
         \"minecraft:trim\": {material: \"minecraft:iron\", pattern: \"minecraft:bolt\"}, \
         \"minecraft:debug_stick_state\": {\"minecraft:stone\": \"facing\"}, \
         \"minecraft:entity_data\": {NoAI: 1b, Health: 20.0f, \
         id: \"minecraft:zombie\"}, \
         \"minecraft:bucket_entity_data\": {Health: 5s}, \
         \"minecraft:block_entity_data\": {Delay: 20s, SpawnCount: 4s, \
         id: \"minecraft:mob_spawner\"}, \
         \"minecraft:instrument\": \"minecraft:call_goat_horn\", \
         \"minecraft:provides_trim_material\": \"minecraft:resin\", \
         \"minecraft:ominous_bottle_amplifier\": 3, \
         \"minecraft:jukebox_playable\": \"minecraft:cat\", \
         \"minecraft:provides_banner_patterns\": \"#minecraft:pattern_item/flower\", \
         \"minecraft:recipes\": [\"minecraft:bread\", \"minecraft:torch\"], \
         \"minecraft:lodestone_tracker\": {target: {dimension: \"minecraft:overworld\", \
         pos: [I; 10, 64, -4]}, tracked: 0b}, \
         \"minecraft:firework_explosion\": {shape: \"burst\", colors: [7833753], \
         fade_colors: [11189196], has_trail: 1b}, \
         \"minecraft:fireworks\": {flight_duration: 2}, \
         \"minecraft:profile\": {name: \"Steve\", id: [I; 0, 0, 0, 1], \
         properties: [{name: \"textures\", value: \"skin-value\", \
         signature: \"skin-signature\"}], \
         texture: \"minecraft:entity/player/wide/steve\", \
         elytra: \"minecraft:entity/player/elytra\", model: \"slim\"}, \
         \"minecraft:note_block_sound\": \"minecraft:block.note_block.harp\", \
         \"minecraft:banner_patterns\": [{pattern: \"minecraft:stripe_bottom\", \
         color: \"red\"}, {pattern: {asset_id: \"example:custom_banner/pattern\", \
         translation_key: \"block.example.banner.custom\"}, color: \"light_blue\"}], \
         \"minecraft:base_color\": \"light_blue\", \
         \"minecraft:pot_decorations\": [\"minecraft:angler_pottery_sherd\", \
         \"minecraft:archer_pottery_sherd\", \"minecraft:brick\", \"minecraft:brick\"], \
         \"minecraft:container\": [{slot: 2, item: {id: \"minecraft:firework_star\", \
         count: 4, components: {\"minecraft:max_damage\": 512, \
         \"minecraft:swing_animation\": {}}}}], \
         \"minecraft:block_state\": {facing: \"south\", \
         \"quoted key\": \"value with 'quote'\"}, \
         \"minecraft:bees\": [{entity_data: {HasNectar: 1b, id: \"minecraft:bee\"}, \
         ticks_in_hive: 40, min_ticks_in_hive: 600}], \
         \"minecraft:lock\": {}, \
         \"minecraft:container_loot\": {loot_table: \
         \"minecraft:chests/simple_dungeon\", seed: 9L}, \
         \"minecraft:break_sound\": \"minecraft:ambient.cave\", \
         \"minecraft:villager/variant\": \"minecraft:plains\", \
         \"minecraft:wolf/variant\": \"minecraft:striped\", \
         \"minecraft:wolf/sound_variant\": \"minecraft:cute\", \
         \"minecraft:wolf/collar\": \"red\", \
         \"minecraft:fox/variant\": \"snow\", \
         \"minecraft:salmon/size\": \"large\", \
         \"minecraft:parrot/variant\": \"gray\", \
         \"minecraft:tropical_fish/pattern\": \"stripey\", \
         \"minecraft:tropical_fish/base_color\": \"orange\", \
         \"minecraft:tropical_fish/pattern_color\": \"white\", \
         \"minecraft:mooshroom/variant\": \"brown\", \
         \"minecraft:rabbit/variant\": \"evil\", \
         \"minecraft:pig/variant\": \"minecraft:cold\", \
         \"minecraft:pig/sound_variant\": \"minecraft:big\", \
         \"minecraft:cow/variant\": \"minecraft:cold\", \
         \"minecraft:cow/sound_variant\": \"minecraft:moody\", \
         \"minecraft:chicken/variant\": \"minecraft:cold\", \
         \"minecraft:chicken/sound_variant\": \"minecraft:picky\", \
         \"minecraft:zombie_nautilus/variant\": \"minecraft:warm\", \
         \"minecraft:frog/variant\": \"minecraft:cold\", \
         \"minecraft:horse/variant\": \"dark_brown\", \
         \"minecraft:painting/variant\": \"minecraft:dennis\", \
         \"minecraft:llama/variant\": \"gray\", \
         \"minecraft:axolotl/variant\": \"blue\", \
         \"minecraft:cat/variant\": \"minecraft:all_black\", \
         \"minecraft:cat/sound_variant\": \"minecraft:royal\", \
         \"minecraft:cat/collar\": \"black\", \
         \"minecraft:sheep/color\": \"cyan\", \
         \"minecraft:shulker/color\": \"white\"}}"
                .to_string()
        )
    );
}

#[test]
fn item_stack_snbt_preserves_repairable_holder_set_shapes() {
    let registry: ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": ["minecraft:iron_ingot", "minecraft:diamond"],
        "protocol_ids": {
            "minecraft:iron_ingot": 0,
            "minecraft:diamond": 1
        }
    }))
    .unwrap();
    let item_runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        EquipmentAssetCatalog::default(),
    );

    let mut patch = DataComponentPatchSummary {
        repairable_item_tag: Some("minecraft:repairs_leather_armor".to_string()),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        debug_repairable_component_snbt(&patch, Some(&item_runtime)),
        Some("{items: \"#minecraft:repairs_leather_armor\"}".to_string())
    );

    patch.repairable_item_tag = None;
    patch.repairable_item_registry_ids = vec![0];
    assert_eq!(
        debug_repairable_component_snbt(&patch, Some(&item_runtime)),
        Some("{items: \"minecraft:iron_ingot\"}".to_string())
    );

    patch.repairable_item_registry_ids.clear();
    assert_eq!(
        debug_repairable_component_snbt(&patch, None),
        Some("{items: []}".to_string())
    );

    patch.repairable_item_registry_ids = vec![99];
    assert_eq!(
        debug_repairable_component_snbt(&patch, Some(&item_runtime)),
        None
    );
}

#[test]
fn item_stack_snbt_preserves_resolvable_damage_resistant_holder_sets() {
    let mut patch = DataComponentPatchSummary::default();
    assert_eq!(
        debug_damage_resistant_component_snbt(&patch),
        Some("{types: []}".to_string())
    );

    patch.damage_resistant_type_registry_ids = vec![0];
    assert_eq!(
        debug_damage_resistant_component_snbt(&patch),
        Some("{types: \"minecraft:in_fire\"}".to_string())
    );
}

#[test]
fn item_stack_snbt_preserves_resolvable_banner_pattern_holder_sets() {
    let mut patch = DataComponentPatchSummary::default();
    assert_eq!(
        debug_provides_banner_patterns_component_snbt(&patch),
        Some("[]".to_string())
    );

    patch.provides_banner_pattern_registry_ids = vec![0];
    assert_eq!(
        debug_provides_banner_patterns_component_snbt(&patch),
        Some("\"minecraft:base\"".to_string())
    );
}

#[test]
fn item_stack_snbt_preserves_break_sound_holder_shapes() {
    assert_eq!(
        debug_sound_event_component_snbt(&bbb_protocol::packets::SoundEventSummary {
            registry_id: None,
            sound_id: Some("example:item.custom_break".to_string()),
            fixed_range_bits: Some(16.0_f32.to_bits()),
        }),
        Some("{sound_id: \"example:item.custom_break\", range: 16.0f}".to_string())
    );
    assert_eq!(
        debug_sound_event_component_snbt(&bbb_protocol::packets::SoundEventSummary {
            registry_id: None,
            sound_id: Some("example:item.variable_break".to_string()),
            fixed_range_bits: None,
        }),
        Some("{sound_id: \"example:item.variable_break\"}".to_string())
    );
    assert_eq!(
        debug_sound_event_component_snbt(&bbb_protocol::packets::SoundEventSummary {
            registry_id: Some(1902),
            sound_id: None,
            fixed_range_bits: None,
        }),
        None
    );
}

#[test]
fn item_stack_snbt_omits_default_piercing_weapon_fields_and_unknown_sounds() {
    let mut weapon = bbb_protocol::packets::PiercingWeaponSummary {
        deals_knockback: true,
        dismounts: false,
        sound: None,
        hit_sound: None,
    };
    assert_eq!(
        debug_piercing_weapon_component_snbt(&weapon),
        Some("{}".to_string())
    );

    weapon.sound = Some(bbb_protocol::packets::SoundEventSummary {
        registry_id: Some(1902),
        sound_id: None,
        fixed_range_bits: None,
    });
    assert_eq!(debug_piercing_weapon_component_snbt(&weapon), None);
}

#[test]
fn item_stack_snbt_validates_kinetic_weapon_ticks_and_omits_defaults() {
    let mut weapon = bbb_protocol::packets::KineticWeaponSummary {
        contact_cooldown_ticks: 10,
        delay_ticks: 0,
        dismount_conditions: None,
        knockback_conditions: None,
        damage_conditions: None,
        forward_movement_bits: 0.0_f32.to_bits(),
        damage_multiplier_bits: 1.0_f32.to_bits(),
        sound: None,
        hit_sound: None,
    };
    assert_eq!(
        debug_kinetic_weapon_component_snbt(&weapon),
        Some("{}".to_string())
    );

    weapon.contact_cooldown_ticks = -1;
    assert_eq!(debug_kinetic_weapon_component_snbt(&weapon), None);

    weapon.contact_cooldown_ticks = 10;
    weapon.damage_conditions = Some(bbb_protocol::packets::KineticWeaponConditionSummary {
        max_duration_ticks: -1,
        min_speed_bits: 0.0_f32.to_bits(),
        min_relative_speed_bits: 0.0_f32.to_bits(),
    });
    assert_eq!(debug_kinetic_weapon_component_snbt(&weapon), None);
}

#[test]
fn item_stack_snbt_omits_registry_backed_components_with_unknown_ids() {
    let registry: ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": ["minecraft:brick"],
        "protocol_ids": {"minecraft:brick": 0}
    }))
    .unwrap();
    let item_runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        EquipmentAssetCatalog::default(),
    );
    let unknown_entity_type_id = i32::try_from(VANILLA_ENTITY_TYPES.len()).unwrap();
    let unknown_block_entity_type_id = i32::try_from(VANILLA_BLOCK_ENTITY_TYPES.len()).unwrap();
    let stack = ProtocolItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added: 9,
            added_type_ids: vec![18, 25, 49, 50, 58, 60, 74, 75, 77],
            tooltip_hidden_component_type_ids: vec![999],
            use_remainder: Some(Box::new(bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 99,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            })),
            charged_projectiles_items: vec![bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 99,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }],
            bundle_contents_items: vec![bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 99,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }],
            bundle_contents_item_count: Some(1),
            entity_data_entity_type_id: Some(unknown_entity_type_id),
            entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
            block_entity_data_type_id: Some(unknown_block_entity_type_id),
            block_entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
            bees_present: true,
            bees_count: 1,
            bees: vec![bbb_protocol::packets::BeeOccupantSummary {
                entity_type_id: unknown_entity_type_id,
                entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
                ticks_in_hive: 0,
                min_ticks_in_hive: 600,
            }],
            pot_decorations_item_ids: vec![99],
            container_items: vec![bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 99,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }],
            container_slots: vec![bbb_protocol::packets::ItemContainerSlotSummary {
                slot: 0,
                item: bbb_protocol::packets::ItemStackTemplateSummary {
                    item_id: 99,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
            container_item_count: Some(1),
            ..DataComponentPatchSummary::default()
        },
    };

    assert_eq!(
        debug_item_stack_snbt(
            "minecraft:decorated_pot",
            stack.count,
            Some(&stack),
            Some(&item_runtime),
        ),
        Some("{id: \"minecraft:decorated_pot\"}".to_string())
    );
}

#[test]
fn item_stack_snbt_omits_components_with_invalid_persistent_integer_ranges() {
    let stack = ProtocolItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added: 6,
            added_type_ids: vec![1, 2, 3, 19, 31, 63],
            max_stack_size: Some(100),
            max_damage: Some(0),
            damage: Some(-1),
            repair_cost: Some(-1),
            enchantable_value: Some(0),
            ominous_bottle_amplifier: Some(5),
            ..DataComponentPatchSummary::default()
        },
    };

    assert_eq!(
        debug_item_stack_snbt("minecraft:stone", stack.count, Some(&stack), None),
        Some("{id: \"minecraft:stone\"}".to_string())
    );
}

#[test]
fn item_stack_snbt_rejects_invalid_stack_and_template_shapes() {
    assert_eq!(debug_item_stack_snbt("minecraft:air", 1, None, None), None);
    assert_eq!(
        debug_item_stack_snbt("minecraft:stone", 100, None, None),
        None
    );

    let registry: ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": ["minecraft:air", "minecraft:arrow", "minecraft:stone"],
        "protocol_ids": {
            "minecraft:air": 0,
            "minecraft:arrow": 1,
            "minecraft:stone": 2
        }
    }))
    .unwrap();
    let item_runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        EquipmentAssetCatalog::default(),
    );
    let stack = ProtocolItemStackSummary {
        item_id: Some(2),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added: 4,
            added_type_ids: vec![25, 49, 50, 75],
            use_remainder: Some(Box::new(bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 0,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            })),
            charged_projectiles_items: vec![bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 1,
                count: 100,
                component_patch: DataComponentPatchSummary::default(),
            }],
            bundle_contents_items: vec![bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 1,
                count: 0,
                component_patch: DataComponentPatchSummary::default(),
            }],
            bundle_contents_item_count: Some(1),
            container_slots: vec![bbb_protocol::packets::ItemContainerSlotSummary {
                slot: 0,
                item: bbb_protocol::packets::ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
            container_item_count: Some(1),
            ..DataComponentPatchSummary::default()
        },
    };

    assert_eq!(
        debug_item_stack_snbt(
            "minecraft:stone",
            stack.count,
            Some(&stack),
            Some(&item_runtime),
        ),
        Some("{id: \"minecraft:stone\"}".to_string())
    );
}

#[test]
fn item_stack_snbt_bounds_recursive_template_projection() {
    const STACK_BUDGET_BYTES: usize = 4 * 1024 * 1024;

    fn use_remainder_patch(depth: usize) -> DataComponentPatchSummary {
        let mut patch = DataComponentPatchSummary::default();
        for _ in 0..depth {
            patch = DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
                ],
                use_remainder: Some(Box::new(
                    bbb_protocol::packets::ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: patch,
                    },
                )),
                ..DataComponentPatchSummary::default()
            };
        }
        patch
    }

    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let item_runtime =
                NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
            PERSISTENT_ITEM_TEMPLATE_STACK_EXTREMES.with(|cell| cell.set((0, usize::MAX)));
            assert!(debug_supported_component_patch_snbt(
                &use_remainder_patch(bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,),
                Some(&item_runtime),
            )
            .is_some());
            let (top, bottom) = PERSISTENT_ITEM_TEMPLATE_STACK_EXTREMES.with(std::cell::Cell::get);
            let used = top - bottom;
            assert!(
                used <= STACK_BUDGET_BYTES,
                "depth-{} persistent item template used {used} bytes of stack, over the \
                 {STACK_BUDGET_BYTES}-byte budget",
                bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH
            );

            assert_eq!(
                debug_supported_component_patch_snbt(
                    &use_remainder_patch(
                        bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH + 1,
                    ),
                    Some(&item_runtime),
                ),
                None
            );
        })
        .expect("spawn deep persistent item-template test thread")
        .join()
        .expect("deep persistent item-template test thread");
}

#[test]
fn item_stack_template_budget_preflight_walks_all_recursive_components() {
    fn nested_template(
        depth: usize,
        component: usize,
    ) -> bbb_protocol::packets::ItemStackTemplateSummary {
        let mut template = bbb_protocol::packets::ItemStackTemplateSummary {
            item_id: 1,
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        for _ in 1..depth {
            let mut patch = DataComponentPatchSummary::default();
            match component {
                0 => patch.use_remainder = Some(Box::new(template)),
                1 => patch.charged_projectiles_items = vec![template],
                2 => {
                    patch.bundle_contents_items = vec![template];
                    patch.bundle_contents_item_count = Some(1);
                }
                3 => {
                    patch.container_slots = vec![bbb_protocol::packets::ItemContainerSlotSummary {
                        slot: 0,
                        item: template,
                    }];
                    patch.container_item_count = Some(1);
                }
                4 => {
                    patch.container_items = vec![template];
                    patch.container_item_count = Some(1);
                }
                _ => unreachable!("five recursive summary paths"),
            }
            template = bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: 1,
                count: 1,
                component_patch: patch,
            };
        }
        template
    }

    for component in 0..5 {
        assert!(item_stack_template_nesting_within_budget(
            &nested_template(
                bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
                component,
            ),
            bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
        ));
        assert!(!item_stack_template_nesting_within_budget(
            &nested_template(
                bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH + 1,
                component,
            ),
            bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
        ));
    }
}

#[test]
fn item_stack_snbt_preserves_all_persistent_component_removals() {
    let stack = ProtocolItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            removed_type_ids: vec![8, 20, 41, 48, 80, 109, 110],
            ..DataComponentPatchSummary::default()
        },
    };

    assert_eq!(
        debug_item_stack_snbt("minecraft:stone", stack.count, Some(&stack), None),
        Some(
            "{id: \"minecraft:stone\", components: {\
             \"!minecraft:damage_type\": {}, \"!minecraft:break_sound\": {}, \
             \"!minecraft:shulker/color\": {}}}"
                .to_string()
        )
    );
}

#[test]
fn component_patch_dispatch_uses_encoder_registry_and_catalog_metadata() {
    let patch = DataComponentPatchSummary {
        added: 2,
        added_type_ids: vec![1, 20],
        removed_type_ids: vec![8, 20],
        max_stack_size: Some(64),
        ..DataComponentPatchSummary::default()
    };

    assert_eq!(
        debug_supported_component_patch_snbt(&patch, None),
        Some(
            "{\"minecraft:max_stack_size\": 64, \
             \"!minecraft:damage_type\": {}}"
                .to_string()
        )
    );
}

#[test]
fn shift_f3_i_skips_unpickable_dragon_fireball_but_direct_copy_encodes_save_nbt() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 54,
        uuid: Uuid::from_u128(54),
        entity_type_id: VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_projectile_power(ProtocolProjectilePower {
        entity_id: 54,
        acceleration_power: 0.25,
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_unpickable_debug_entity_copy(
        &mut input,
        &world,
        &clipboard,
        54,
        "/summon minecraft:dragon_fireball 0.00 1.00 2.00 \
         {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
         HasBeenShot: 0b, \
         acceleration_power: 0.25d}",
    );
}

fn assert_shift_f3_i_with_permission_copies_local_wind_charge_save_nbt_to_clipboard(
    entity_type_id: i32,
    entity_id: i32,
    summon_id: &str,
) {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: entity_id,
        uuid: Uuid::from_u128(entity_id as u128),
        entity_type_id,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    let expected = format!(
        "/summon {summon_id} 0.00 1.50 2.00 \
         {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
         HasBeenShot: 0b, \
         acceleration_power: 0.0d}}"
    );
    assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_wind_charge_save_nbt_to_clipboard() {
    assert_shift_f3_i_with_permission_copies_local_wind_charge_save_nbt_to_clipboard(
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        55,
        "minecraft:wind_charge",
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_breeze_wind_charge_save_nbt_to_clipboard() {
    assert_shift_f3_i_with_permission_copies_local_wind_charge_save_nbt_to_clipboard(
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
        56,
        "minecraft:breeze_wind_charge",
    );
}

#[test]
fn shift_f3_i_skips_unpickable_wither_skull_but_direct_copy_encodes_save_nbt() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 53,
        uuid: Uuid::from_u128(53),
        entity_type_id: VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_projectile_power(ProtocolProjectilePower {
        entity_id: 53,
        acceleration_power: 0.5,
    }));
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 53,
        values: vec![ProtocolEntityDataValue {
            data_id: wither_skull_dangerous_data_id(),
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_unpickable_debug_entity_copy(
        &mut input,
        &world,
        &clipboard,
        53,
        "/summon minecraft:wither_skull 0.00 1.50 2.00 \
         {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
         HasBeenShot: 0b, \
         acceleration_power: 0.5d, dangerous: 1b}",
    );
}

#[test]
fn shift_f3_i_skips_unpickable_llama_spit_but_direct_copy_encodes_save_nbt() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 57,
        uuid: Uuid::from_u128(57),
        entity_type_id: VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_unpickable_debug_entity_copy(
        &mut input,
        &world,
        &clipboard,
        57,
        "/summon minecraft:llama_spit 0.00 1.50 2.00 \
         {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
         HasBeenShot: 0b}",
    );
}

#[test]
fn shift_f3_i_skips_unpickable_arrow_but_direct_copy_encodes_save_nbt() {
    let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARROW_ID,
    )
    .expect("generated Arrow inherited metadata accessors");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 58,
        uuid: Uuid::from_u128(58),
        entity_type_id: VANILLA_ENTITY_TYPE_ARROW_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 58,
        values: vec![
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.flags_default | accessors.crit_mask),
            },
            ProtocolEntityDataValue {
                data_id: accessors.pierce_level_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(3),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_unpickable_debug_entity_copy(
        &mut input,
        &world,
        &clipboard,
        58,
        "/summon minecraft:arrow 0.00 1.50 2.00 \
         {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
         HasBeenShot: 0b, crit: 1b, PierceLevel: 3b}",
    );
}

#[test]
fn shift_f3_i_skips_unpickable_spectral_arrow_but_direct_copy_encodes_save_nbt() {
    let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
    )
    .expect("generated SpectralArrow inherited metadata accessors");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 59,
        uuid: Uuid::from_u128(59),
        entity_type_id: VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 59,
        values: vec![
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.flags_default | accessors.crit_mask),
            },
            ProtocolEntityDataValue {
                data_id: accessors.pierce_level_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(2),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_unpickable_debug_entity_copy(
        &mut input,
        &world,
        &clipboard,
        59,
        "/summon minecraft:spectral_arrow 0.00 1.50 2.00 \
         {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
         HasBeenShot: 0b, crit: 1b, PierceLevel: 2b}",
    );
}

#[test]
fn shift_f3_i_skips_unpickable_trident_but_direct_copy_encodes_save_nbt() {
    let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .expect("generated ThrownTrident inherited metadata accessors");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    insert_empty_chunk_for_block(&mut world, BlockPos { x: 0, y: 1, z: 2 });
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 60,
        uuid: Uuid::from_u128(60),
        entity_type_id: VANILLA_ENTITY_TYPE_TRIDENT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![
            ProtocolEntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.flags_default | accessors.crit_mask),
            },
            ProtocolEntityDataValue {
                data_id: accessors.pierce_level_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(4),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_unpickable_debug_entity_copy(
        &mut input,
        &world,
        &clipboard,
        60,
        "/summon minecraft:trident 0.00 1.50 2.00 \
         {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
         HasBeenShot: 0b, crit: 1b, PierceLevel: 4b, DealtDamage: 0b}",
    );

    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.in_ground_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    world.advance_entity_client_animations(5);
    let command = debug_copy_recreate_entity_command(&world, None, 60, true)
        .expect("trident recreate command")
        .command;
    assert!(command.ends_with("crit: 1b, PierceLevel: 4b, DealtDamage: 0b}"));

    world.advance_entity_client_animations(1);
    let command = debug_copy_recreate_entity_command(&world, None, 60, true)
        .expect("trident recreate command after sixth in-ground tick")
        .command;
    assert!(command.ends_with("crit: 1b, PierceLevel: 4b, DealtDamage: 1b}"));
}

#[test]
fn shift_f3_i_with_permission_copies_local_falling_block_save_nbt_to_clipboard() {
    let sand_state_id = BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:sand", &BTreeMap::new())
        .expect("vanilla 26.1 sand block state exists")
        .id;
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    insert_empty_chunk_for_block(&mut world, BlockPos { x: 0, y: 1, z: 2 });
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 61,
        uuid: Uuid::from_u128(61),
        entity_type_id: VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: sand_state_id,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:falling_block 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             BlockState: {Name: \"minecraft:sand\"}, \
             Time: 0}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );

    world.advance_entity_client_animations(7);
    let command = debug_copy_recreate_entity_command(&world, None, 61, true)
        .expect("falling-block recreate command after client ticks")
        .command;
    assert!(command.contains("BlockState: {Name: \"minecraft:sand\"}, Time: 7"));
}

#[test]
fn shift_f3_i_with_permission_copies_local_minecart_save_nbt_to_clipboard() {
    let accessors = vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_MINECART_ID,
    )
    .expect("generated Minecart metadata accessors");
    let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
    let grass_state_id = BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:grass_block", &grass_props)
        .expect("vanilla 26.1 grass block state exists")
        .id;
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    insert_empty_chunk_for_block(&mut world, BlockPos { x: 0, y: 1, z: 2 });
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 62,
        uuid: Uuid::from_u128(62),
        entity_type_id: VANILLA_ENTITY_TYPE_MINECART_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 62,
        values: vec![
            ProtocolEntityDataValue {
                data_id: accessors.custom_display_block_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
                value: EntityDataValueKind::OptionalBlockState(Some(grass_state_id)),
            },
            ProtocolEntityDataValue {
                data_id: accessors.display_offset_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(3),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:minecart 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             DisplayState: {Name: \"minecraft:grass_block\", Properties: {snowy: \"false\"}}, \
             DisplayOffset: 3, FlippedRotation: 0b, HasTicked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );

    world.advance_entity_client_animations(1);
    let command = debug_copy_recreate_entity_command(&world, None, 62, true)
        .expect("minecart recreate command after first client tick")
        .command;
    assert!(command.ends_with("DisplayOffset: 3, FlippedRotation: 0b, HasTicked: 0b}"));
}

#[test]
fn shift_f3_i_with_permission_copies_local_chest_minecart_save_nbt_to_clipboard() {
    let accessors = vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
    )
    .expect("generated ChestMinecart metadata accessors");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 63,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.display_offset_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(6),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:chest_minecart 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             DisplayOffset: 6, \
             FlippedRotation: 0b, HasTicked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_command_block_minecart_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 63,
        values: vec![ProtocolEntityDataValue {
            data_id: COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_STRING_ID,
            value: EntityDataValueKind::String("say hi".to_string()),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:command_block_minecart 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             FlippedRotation: 0b, HasTicked: 1b, Command: \"say hi\"}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_furnace_minecart_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:furnace_minecart 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             FlippedRotation: 0b, HasTicked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_hopper_minecart_save_nbt_to_clipboard() {
    let accessors = vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
    )
    .expect("generated HopperMinecart metadata accessors");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 63,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.display_offset_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(6),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:hopper_minecart 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             DisplayOffset: 6, FlippedRotation: 0b, HasTicked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_spawner_minecart_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:spawner_minecart 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             FlippedRotation: 0b, HasTicked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_tnt_minecart_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: 63,
        event_id: 10,
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:tnt_minecart 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             FlippedRotation: 0b, HasTicked: 1b, \
             fuse: 80}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_item_frame_save_nbt_to_clipboard() {
    let registry: ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": ["minecraft:diamond"],
        "protocol_ids": {"minecraft:diamond": 0}
    }))
    .unwrap();
    let item_runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        EquipmentAssetCatalog::default(),
    );
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![RegistryPacketEntry::stub("example:custom")],
    );
    world.record_registry_entries(
        "minecraft:instrument",
        0,
        vec![RegistryPacketEntry::stub("example:custom_horn")],
    );
    world.record_registry_entries(
        "minecraft:banner_pattern",
        0,
        vec![RegistryPacketEntry::stub("example:custom_pattern")],
    );
    world.record_registry_entries(
        "minecraft:wolf_variant",
        0,
        vec![RegistryPacketEntry::stub("example:custom_wolf")],
    );
    world.record_registry_entries(
        "minecraft:jukebox_song",
        0,
        vec![RegistryPacketEntry::stub("example:custom_song")],
    );
    world.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::stub("example:custom_enchantment")],
    );
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 3,
    });
    let item_frame_accessors = item_frame_metadata_accessors(&world, 63);
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 63,
        values: vec![
            ProtocolEntityDataValue {
                data_id: item_frame_accessors.direction_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
                value: EntityDataValueKind::Direction(item_frame_accessors.direction_default),
            },
            ProtocolEntityDataValue {
                data_id: item_frame_accessors.rotation_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(11),
            },
            ProtocolEntityDataValue {
                data_id: item_frame_accessors.item_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                value: EntityDataValueKind::ItemStack(ProtocolItemStackSummary {
                    item_id: Some(0),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added: 7,
                        added_type_ids: vec![
                            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
                            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
                            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
                            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
                            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
                            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
                        ],
                        damage_type_id: Some(0),
                        enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 0,
                            level: 3,
                        }],
                        instrument_id: Some(0),
                        provides_banner_pattern_registry_ids: vec![0],
                        banner_pattern_layers: vec![bbb_protocol::packets::BannerPatternLayerSummary {
                            registry_id: Some(0),
                            asset_id: None,
                            translation_key: None,
                            color_id: VANILLA_DYE_COLOR_RED_ID,
                        }],
                        wolf_variant_id: Some(0),
                        jukebox_song_id: Some(0),
                        ..Default::default()
                    },
                }),
            },
            entity_invisible_data(),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.5,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:item_frame 0.50 1.50 2.03 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             block_pos: [I; 0, 1, 2], \
             ItemRotation: 3b, Facing: 3b, Invisible: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );

    let entity = world.probe_entity(63).unwrap();
    assert_eq!(item_runtime.item_resource_id(0), Some("minecraft:diamond"));
    assert!(
        debug_entity_data_item_stack_present(&entity, item_frame_accessors.item_data_id).is_some()
    );
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity).unwrap();
    assert!(
        snbt.contains(
            "block_pos: [I; 0, 1, 2], Item: {id: \"minecraft:diamond\", components: \
         {\"minecraft:damage_type\": \"example:custom\", \
         \"minecraft:enchantments\": {\"example:custom_enchantment\": 3}, \
         \"minecraft:instrument\": \"example:custom_horn\", \
         \"minecraft:provides_banner_patterns\": \"example:custom_pattern\", \
         \"minecraft:banner_patterns\": [{pattern: \"example:custom_pattern\", color: \"red\"}], \
         \"minecraft:wolf/variant\": \"example:custom_wolf\", \
         \"minecraft:jukebox_playable\": \"example:custom_song\"}}, \
         ItemRotation: 3b"
        ),
        "unexpected item-frame SNBT: {snbt}"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_glow_item_frame_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 2,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.5,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:glow_item_frame 0.50 1.50 2.97 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             block_pos: [I; 0, 1, 2], Facing: 2b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn local_painting_save_nbt_and_bounds_follow_synchronized_holder_order() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:painting_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:dennis"),
            RegistryPacketEntry::stub("minecraft:kebab"),
        ],
    );
    world.apply_add_entity(AddEntity {
        id: 64,
        uuid: Uuid::from_u128(64),
        entity_type_id: VANILLA_ENTITY_TYPE_PAINTING_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 3,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 64,
        values: vec![painting_variant_data(&world, 64, 0)],
    }));

    assert_eq!(
        world.probe_entity_pick_bounds(64),
        Some(bbb_world::EntityPickBoundsState::from_centered_size(
            3.0, 3.0, 0.0625, 0.0,
        ))
    );
    assert_eq!(
        debug_copy_recreate_entity_command(&world, None, 64, true)
            .unwrap()
            .command,
        "/summon minecraft:painting 0.50 1.50 2.03 {Motion: [0.0d, 0.0d, 0.0d], \
         Rotation: [0.0f, 0.0f], block_pos: [I; 0, 1, 2], facing: 0b, \
         variant: \"minecraft:dennis\"}"
    );
}

#[test]
fn debug_boat_save_memberships_use_the_world_generated_catalog() {
    for entry in bbb_world::VANILLA_ENTITY_TYPE_METADATA_26_1 {
        assert_eq!(
            debug_entity_type_writes_abstract_boat_save_data(entry.entity_type_id),
            entry.is_boat,
            "boat membership for {}",
            entry.resource_id
        );
        assert_eq!(
            bbb_world::vanilla_entity_type_is_chest_boat(entry.entity_type_id),
            entry.is_abstract_chest_boat,
            "chest-boat membership for {}",
            entry.resource_id
        );
    }

    let source = include_str!("../input.rs");
    let body = source
        .split("fn debug_entity_type_writes_abstract_boat_save_data")
        .nth(1)
        .and_then(|source| source.split_once("\n}"))
        .map(|(body, _)| body)
        .expect("debug boat membership query source");
    assert!(!body.contains("matches!"));
}

#[test]
fn minecart_and_item_frame_dispatch_use_generated_catalogs() {
    for entry in bbb_world::VANILLA_ENTITY_TYPE_METADATA_26_1 {
        assert_eq!(
            bbb_world::vanilla_entity_type_is_minecart(entry.entity_type_id),
            entry.is_minecart,
            "minecart membership for {}",
            entry.resource_id
        );
        assert_eq!(
            bbb_world::vanilla_entity_type_is_item_frame(entry.entity_type_id),
            entry.is_item_frame,
            "item-frame membership for {}",
            entry.resource_id
        );
    }

    let input_source = include_str!("../input.rs");
    assert!(input_source
        .contains("if bbb_world::vanilla_entity_type_is_minecart(entity.entity_type_id)"));
    assert!(input_source
        .contains("_ if bbb_world::vanilla_entity_type_is_item_frame(entity.entity_type_id)"));
    let kind_source = include_str!("../entity_scene/kind.rs");
    assert!(kind_source.contains("_ if bbb_world::vanilla_entity_type_is_minecart(entity_type_id)"));
    assert!(kind_source.contains("vanilla_entity_renderer_presentation_owner"));
    assert!(!kind_source.contains("vanilla_entity_renderer_uses_native_non_model_pass"));
}

#[test]
fn piglin_and_spider_dispatch_use_the_world_generated_catalog() {
    for entry in bbb_world::VANILLA_ENTITY_TYPE_METADATA_26_1 {
        assert_eq!(
            bbb_world::vanilla_entity_type_is_abstract_piglin(entry.entity_type_id),
            entry.is_abstract_piglin,
            "abstract-piglin membership for {}",
            entry.resource_id
        );
        assert_eq!(
            bbb_world::vanilla_entity_type_is_spider(entry.entity_type_id),
            entry.is_spider,
            "spider membership for {}",
            entry.resource_id
        );
    }

    assert!(include_str!("../entity_scene/instance.rs").contains(
        "bbb_world::vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type"
    ));
    assert!(include_str!("../input.rs")
        .contains("bbb_world::vanilla_entity_type_is_spider(entity.entity_type_id)"));
    assert!(include_str!("../runtime.rs")
        .contains("bbb_world::vanilla_entity_camera_post_effect_id(entity_type_id)"));
}

#[test]
fn wind_charge_dispatch_uses_the_world_generated_catalog() {
    for entry in bbb_world::VANILLA_ENTITY_TYPE_METADATA_26_1 {
        assert_eq!(
            bbb_world::vanilla_entity_type_is_wind_charge(entry.entity_type_id),
            entry.is_wind_charge,
            "wind-charge membership for {}",
            entry.resource_id
        );
    }

    assert!(include_str!("../entity_scene/kind.rs")
        .contains("bbb_world::vanilla_entity_type_is_wind_charge(entity_type_id)"));
    let projectile_source = include_str!("../../../bbb-world/src/entities/projectiles.rs");
    let initial_state = projectile_source
        .split("fn initial_hurting_projectile_state")
        .nth(1)
        .and_then(|source| source.split_once("\n}"))
        .map(|(body, _)| body)
        .expect("hurting-projectile initial state source");
    assert!(initial_state.contains("is_vanilla_wind_charge_type(entity_type_id)"));
}

#[test]
fn hurting_projectile_save_dispatch_uses_the_world_generated_catalog() {
    for entry in bbb_world::VANILLA_ENTITY_TYPE_METADATA_26_1 {
        assert_eq!(
            bbb_world::vanilla_entity_type_is_hurting_projectile(entry.entity_type_id),
            entry.is_hurting_projectile,
            "hurting-projectile membership for {}",
            entry.resource_id
        );
    }

    assert!(include_str!("../input.rs").contains(
        "if bbb_world::vanilla_entity_type_is_hurting_projectile(entity.entity_type_id)"
    ));
}

#[test]
fn shift_f3_i_with_permission_copies_local_boat_leash_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.1,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.apply_add_entity(AddEntity {
        id: 64,
        uuid: Uuid::from_u128(64),
        entity_type_id: VANILLA_ENTITY_TYPE_COW_ID,
        position: ProtocolVec3d {
            x: 8.0,
            y: 0.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_link(SetEntityLink {
        source_id: 63,
        dest_id: 64,
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:oak_boat 0.00 1.10 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             leash: {UUID: [I; 0, 0, 0, 64]}}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_chest_boat_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 65,
        uuid: Uuid::from_u128(65),
        entity_type_id: VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.1,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:oak_chest_boat 0.00 1.10 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f]}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_primed_tnt_save_nbt_to_clipboard() {
    let accessors = bbb_world::vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TNT_ID,
    )
    .expect("generated PrimedTnt metadata accessors");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 61,
        uuid: Uuid::from_u128(61),
        entity_type_id: VANILLA_ENTITY_TYPE_TNT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.fuse_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(37),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:tnt 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fuse: 37s, \
             block_state: {Name: \"minecraft:tnt\", Properties: {unstable: \"false\"}}}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_shulker_bullet_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 58,
        uuid: Uuid::from_u128(58),
        entity_type_id: VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:shulker_bullet 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             HasBeenShot: 0b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_armor_stand_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    )
    .expect("generated ArmorStand metadata accessors must resolve");
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 58,
        uuid: Uuid::from_u128(58),
        entity_type_id: VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 58,
        values: vec![
            entity_invisible_data(),
            ProtocolEntityDataValue {
                data_id: accessors.client_flags_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(
                    accessors.small_mask
                        | accessors.show_arms_mask
                        | accessors.no_baseplate_mask,
                ),
            },
            ProtocolEntityDataValue {
                data_id: accessors.body_pose_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
                value: EntityDataValueKind::Rotations {
                    x: 0.0,
                    y: 15.0,
                    z: 0.0,
                },
            },
            ProtocolEntityDataValue {
                data_id: accessors.left_arm_pose_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
                value: EntityDataValueKind::Rotations {
                    x: -30.0,
                    y: 0.0,
                    z: -20.0,
                },
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:armor_stand 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             Invisible: 1b, Small: 1b, \
             ShowArms: 1b, NoBasePlate: 1b, \
             Pose: {Body: [0.0f, 15.0f, 0.0f], LeftArm: [-30.0f, 0.0f, -20.0f]}}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_skips_unpickable_experience_orb_but_direct_copy_encodes_save_nbt() {
    let (value_data_id, _) =
        bbb_world::vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        )
        .expect("generated ExperienceOrb value accessor");
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 59,
        uuid: Uuid::from_u128(59),
        entity_type_id: VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 59,
        values: vec![ProtocolEntityDataValue {
            data_id: value_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(149),
        }],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_unpickable_debug_entity_copy(
        &mut input,
        &world,
        &clipboard,
        59,
        "/summon minecraft:experience_orb 0.00 1.50 2.00 \
         {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
         Value: 149s}",
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_endermite_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_ENDERMITE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![mob_flags_data(&world, 50, true, true)],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:endermite 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_enderman_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![mob_flags_data(&world, 50, true, true)],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:enderman 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_silverfish_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_SILVERFISH_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![mob_flags_data(&world, 50, true, true)],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:silverfish 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_guardian_family_save_nbt_to_clipboard() {
    let mut matched_members = 0;
    for entity_type in VANILLA_ENTITY_TYPES {
        let entity_type_id = entity_type.id;
        if !bbb_world::vanilla_entity_metadata_type_is_guardian(entity_type_id) {
            continue;
        }
        matched_members += 1;
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 50,
            uuid: Uuid::from_u128(50),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.0,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 50,
            values: vec![mob_flags_data(&world, 50, true, true)],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        let mut clipboard = MockDebugClipboard::accepting();
        input.set_shift_key(KeyCode::ShiftLeft, true);

        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));
        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::KeyI),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));

        let expected = format!(
            "/summon {} 0.00 1.00 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}}",
            entity_type.resource_id
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
    assert_ne!(
        matched_members, 0,
        "generated Guardian class has no entity types"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_happy_ghast_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, true, true),
            age_locked_data(&world, 50, true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:happy_ghast 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_ghast_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_GHAST_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![mob_flags_data(&world, 50, false, true)],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:ghast 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_giant_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_GIANT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![mob_flags_data(&world, 50, true, true)],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:giant 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_glow_squid_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, true, true),
            age_locked_data(&world, 50, true),
            ProtocolEntityDataValue {
                data_id: glow_squid_dark_ticks_data_id(),
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(77),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:glow_squid 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, \
             AgeLocked: 1b, DarkTicksRemaining: 77}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_squid_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 51,
        uuid: Uuid::from_u128(51),
        entity_type_id: VANILLA_ENTITY_TYPE_SQUID_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![
            mob_flags_data(&world, 51, true, true),
            baby_data(VANILLA_ENTITY_TYPE_SQUID_ID, true),
            age_locked_data(&world, 51, true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:squid 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_interaction_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let accessors = vanilla_entity_metadata_interaction_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_INTERACTION_ID,
    )
    .expect("generated Interaction metadata accessors must resolve");
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_INTERACTION_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: accessors.width_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(1.5),
            },
            ProtocolEntityDataValue {
                data_id: accessors.height_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(2.0),
            },
            ProtocolEntityDataValue {
                data_id: accessors.response_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:interaction 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             width: 1.5f, height: 2.0f, \
             response: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_iron_golem_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, false, true),
            iron_golem_flags_data(true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:iron_golem 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, PlayerCreated: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_phantom_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_PHANTOM_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, false, true),
            phantom_size_data(5),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:phantom 0.00 1.50 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, size: 5}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_ravager_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_RAVAGER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![mob_flags_data(&world, 50, false, true)],
    }));
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: 50,
        event_id: 4,
    }));
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: 50,
        event_id: 39,
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:ravager 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, AttackTick: 10, StunTick: 40, RoarTick: 0}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_pillager_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 51,
        uuid: Uuid::from_u128(51),
        entity_type_id: VANILLA_ENTITY_TYPE_PILLAGER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![mob_flags_data(&world, 51, true, true)],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:pillager 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_wandering_trader_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 52,
        uuid: Uuid::from_u128(52),
        entity_type_id: VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 52,
        values: vec![
            mob_flags_data(&world, 52, true, true),
            baby_data(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, true),
            age_locked_data(&world, 52, true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:wandering_trader 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b, Age: -1, \
             AgeLocked: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_witch_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 51,
        uuid: Uuid::from_u128(51),
        entity_type_id: VANILLA_ENTITY_TYPE_WITCH_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![
            mob_flags_data(&world, 51, true, true),
            witch_using_item_data(true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:witch 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_spellcaster_illager_save_nbt_to_clipboard() {
    for (index, (entity_type_id, entity_type)) in [
        (VANILLA_ENTITY_TYPE_EVOKER_ID, "minecraft:evoker"),
        (VANILLA_ENTITY_TYPE_ILLUSIONER_ID, "minecraft:illusioner"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        let entity_id = 52 + i32::try_from(index).unwrap_or(0);
        world.apply_add_entity(AddEntity {
            id: entity_id,
            uuid: Uuid::from_u128(u128::try_from(entity_id).unwrap_or(52)),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: entity_id,
            values: vec![
                mob_flags_data(&world, entity_id, true, true),
                ProtocolEntityDataValue {
                    data_id: spellcaster_casting_data_id(entity_type_id),
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(2),
                },
            ],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        let mut clipboard = MockDebugClipboard::accepting();
        input.set_shift_key(KeyCode::ShiftLeft, true);

        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));
        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(KeyCode::KeyI),
            ElementState::Pressed,
            Some(&mut world),
            None,
            Some(&mut clipboard)
        ));

        let expected = format!(
            "/summon {entity_type} 0.00 0.00 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}}"
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
}

#[test]
fn shift_f3_i_with_permission_copies_local_vindicator_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 54,
        uuid: Uuid::from_u128(54),
        entity_type_id: VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 54,
        values: vec![
            mob_flags_data(&world, 54, true, true),
            ProtocolEntityDataValue {
                data_id: raider_celebrating_data_id(VANILLA_ENTITY_TYPE_VINDICATOR_ID),
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:vindicator 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_vex_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_VEX_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![mob_flags_data(&world, 50, true, true)],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:vex 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, NoAI: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_wither_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_WITHER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, false, true),
            wither_invulnerability_data(&world, 50, 220),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:wither 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, Invul: 220}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn shift_f3_i_with_permission_copies_local_zoglin_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_ZOGLIN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            mob_flags_data(&world, 50, false, true),
            baby_data(VANILLA_ENTITY_TYPE_ZOGLIN_ID, true),
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:zoglin 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             LeftHanded: 1b, IsBaby: 1b}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn debug_snbt_string_quotes_and_escapes_custom_names() {
    assert_eq!(debug_snbt_string("Bob \"Prime\""), "'Bob \"Prime\"'");
    assert_eq!(
        debug_snbt_string("Bob's \\ line\n"),
        "\"Bob's \\\\ line\\n\""
    );
}

#[test]
fn f3_i_is_consumed_but_does_not_copy_when_reduced_debug_info_blocks_inspect() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(true);
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(clipboard.text, None);
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    assert!(world.client_chat().messages.is_empty());

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_block_state_description_matches_vanilla_recreate_property_format() {
    assert_eq!(
        debug_block_state_description("minecraft:stone", &BTreeMap::new()),
        "minecraft:stone"
    );
    assert_eq!(
        debug_block_state_description(
            "minecraft:mangrove_propagule",
            &string_props([
                ("stage", "1"),
                ("hanging", "true"),
                ("waterlogged", "false"),
                ("age", "2"),
            ])
        ),
        "minecraft:mangrove_propagule[age=2,hanging=true,stage=1,waterlogged=false]"
    );
}

#[test]
fn f3_l_toggles_profiling_feedback_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert_eq!(
        input.take_debug_profiling_toggle_requests(),
        vec![DebugProfilingToggleRequest::Start]
    );
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert_eq!(
        input.take_debug_profiling_toggle_requests(),
        vec![DebugProfilingToggleRequest::Stop]
    );
    assert!(input.take_debug_profiling_toggle_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Profiling started for 10 seconds. Use F3 + L to stop early"
    );
    assert_eq!(messages[1].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[1].content,
        "[Debug]: Profiling ended. Results folder debug/profiling"
    );
    assert_eq!(messages[1].styled_content.len(), 4);
    assert_eq!(
        messages[1].styled_content[2].text,
        "Profiling ended. Results folder "
    );
    assert_eq!(
        messages[1].styled_content[2].style,
        ComponentStyle::default()
    );
    assert_eq!(
        messages[1].styled_content[3].text,
        DEBUG_PROFILING_RESULTS_RELATIVE_DIR
    );
    assert_eq!(messages[1].styled_content[3].style.underlined, Some(true));
    assert_eq!(
        messages[1].styled_content[3].style.click_event,
        Some(ComponentClickEvent::OpenFile {
            path: DEBUG_PROFILING_RESULTS_RELATIVE_DIR.to_string(),
        })
    );
    assert_eq!(world.counters().chat_messages_tracked, 2);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_f6_records_debug_options_screen_request_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F6),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_options_screen_requests(), 1);
    assert_eq!(input.take_debug_options_screen_requests(), 0);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_options_screen_projects_vanilla_order_and_search_filter() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    let surface = winit::dpi::PhysicalSize::new(420, 240);

    let state = input
        .debug_options_screen_hud_state(surface, false)
        .expect("screen should project");
    assert_eq!(state.total_rows, 47);
    assert_eq!(state.visible_rows, 7);
    assert!(!state.default_profile_active);
    assert!(state.performance_profile_active);
    assert_eq!(
        state.rows[..3],
        [
            DebugOptionsScreenHudRow::Category {
                label: "Debug Screen Text".to_string()
            },
            DebugOptionsScreenHudRow::Entry {
                entry: DebugScreenEntryId::Biome,
                path: "biome".to_string(),
                status: DebugScreenEntryStatus::Never,
                hovered_status: None,
                allowed: true,
            },
            DebugOptionsScreenHudRow::Entry {
                entry: DebugScreenEntryId::ChunkGenerationStats,
                path: "chunk_generation_stats".to_string(),
                status: DebugScreenEntryStatus::Never,
                hovered_status: None,
                allowed: true,
            },
        ]
    );

    assert!(input.handle_debug_options_screen_text_input("chunk"));
    let filtered = input
        .debug_options_screen_hud_state(surface, false)
        .expect("screen should project after search");
    let labels = filtered
        .rows
        .iter()
        .map(|row| match row {
            DebugOptionsScreenHudRow::Category { label } => label.as_str(),
            DebugOptionsScreenHudRow::Entry { path, .. } => path.as_str(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "Debug Screen Text",
            "chunk_generation_stats",
            "chunk_render_stats",
            "chunk_source_stats",
            "Debug Renderers",
            "chunk_borders",
            "chunk_section_octree",
        ]
    );
}

#[test]
fn debug_options_screen_search_tracks_editbox_cursor_selection_and_word_keys() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();

    assert!(input.handle_debug_options_screen_text_input("chunk stats"));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_text_input("_"));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "chunk stat_s");
        assert_eq!(screen.search_cursor, 11);
        assert_eq!(screen.search_selection, 11);
    }

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "chunk stat_s");
        assert_eq!(screen.search_cursor, 9);
        assert_eq!(screen.search_selection, 11);
    }

    assert!(input.handle_debug_options_screen_text_input("X"));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "chunk staXs");
        assert_eq!(screen.search_cursor, 10);
        assert_eq!(screen.search_selection, 10);
    }

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    ));
    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::KeyA), ElementState::Pressed,));
    assert!(input.handle_debug_options_screen_text_input("alpha beta gamma"));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "alpha beta ");
        assert_eq!(screen.search_cursor, 11);
        assert_eq!(screen.search_selection, 11);
    }

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    ));
    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::Home), ElementState::Pressed,));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    ));
    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::End), ElementState::Pressed,));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    ));
    let state = input
        .debug_options_screen_hud_state(winit::dpi::PhysicalSize::new(420, 240), false)
        .unwrap();
    assert_eq!(state.search_text, "lpha");
    assert_eq!(state.search_cursor, 4);
    assert_eq!(state.search_selection, 4);
}

#[test]
fn debug_options_screen_search_uses_editbox_filter_and_default_max_length() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();

    assert!(input.handle_debug_options_screen_text_input(
        "abc\u{a7}\ndefghijklmnopqrstuvwxyz0123456789more"
    ));

    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_text, "abcdefghijklmnopqrstuvwxyz012345");
    assert_eq!(debug_options_search_len(&screen.search_text), 32);
    assert_eq!(screen.search_cursor, 32);
    assert_eq!(screen.search_selection, 32);
}

#[test]
fn debug_options_screen_search_handles_editbox_clipboard_shortcuts() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("chunk stats"));

    let mut clipboard = MockDebugClipboard::with_text("stale");
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert_eq!(clipboard.text.as_deref(), Some("chunk stats"));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "chunk stats");
        assert_eq!(screen.search_cursor, 11);
        assert_eq!(screen.search_selection, 0);
    }

    clipboard.text = Some("entity_hitboxes".to_string());
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "entity_hitboxes");
        assert_eq!(screen.search_cursor, 15);
        assert_eq!(screen.search_selection, 15);
    }

    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyX),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert_eq!(clipboard.text.as_deref(), Some("entity_hitboxes"));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "");
        assert_eq!(screen.search_cursor, 0);
        assert_eq!(screen.search_selection, 0);
    }

    clipboard.text = Some("abc\u{a7}\ndef".to_string());
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    ));
    let state = input
        .debug_options_screen_hud_state(winit::dpi::PhysicalSize::new(420, 240), false)
        .unwrap();
    assert_eq!(state.search_text, "abcdef");
    assert_eq!(state.search_cursor, 6);
    assert_eq!(state.search_selection, 6);

    let mut rejecting_clipboard = MockDebugClipboard {
        text: None,
        accepts_text: false,
    };
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut rejecting_clipboard),
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyX),
        ElementState::Pressed,
        Some(&mut rejecting_clipboard),
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    ));
    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_text, "abcdef");
    assert_eq!(screen.search_cursor, 6);
    assert_eq!(screen.search_selection, 0);
}

#[test]
fn debug_options_screen_search_handles_mouse_click_and_drag_selection() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("alpha beta"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 5),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_cursor, 5);
        assert_eq!(screen.search_selection, 5);
    }

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 9),
            f64::from(search_y + 18)
        )),
        surface,
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_cursor, 9);
        assert_eq!(screen.search_selection, 5);
    }

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Released,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 9),
            f64::from(search_y + 18)
        )),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x),
            f64::from(search_y + 2)
        )),
        surface,
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_cursor, 9);
        assert_eq!(screen.search_selection, 5);
    }
}

#[test]
fn debug_options_screen_search_mouse_hit_testing_uses_variable_text_measurer() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("iwx"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;
    let measurer = VariableDebugOptionsSearchTextMeasurer;

    assert!(
        input.handle_debug_options_screen_mouse_input_with_text_measurer(
            winit::event::MouseButton::Left,
            ElementState::Pressed,
            Some(winit::dpi::PhysicalPosition::new(
                f64::from(text_x + 2),
                f64::from(search_y + 2)
            )),
            surface,
            false,
            &measurer,
        )
    );
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_cursor, 1);
        assert_eq!(screen.search_selection, 1);
    }

    assert!(
        input.handle_debug_options_screen_cursor_moved_with_text_measurer(
            Some(winit::dpi::PhysicalPosition::new(
                f64::from(text_x + 9),
                f64::from(search_y + 18)
            )),
            surface,
            &measurer,
        )
    );
    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_cursor, 2);
    assert_eq!(screen.search_selection, 1);
}

#[test]
fn debug_options_screen_search_mouse_hit_testing_uses_display_start() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("aaaaaaaaaaaaaaaaaaaa"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;
    let measurer = VariableDebugOptionsSearchTextMeasurer;

    assert!(
        input.handle_debug_options_screen_mouse_input_with_text_measurer(
            winit::event::MouseButton::Left,
            ElementState::Pressed,
            Some(winit::dpi::PhysicalPosition::new(
                f64::from(text_x),
                f64::from(search_y + 2)
            )),
            surface,
            false,
            &measurer,
        )
    );
    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_cursor, 2);
    assert_eq!(screen.search_selection, 2);
}

#[test]
fn debug_options_screen_search_double_click_selects_word() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("alpha beta"));
    input
        .debug_options_screen
        .as_mut()
        .unwrap()
        .last_left_click_at = Some(Instant::now());
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 8),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));
    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_selection, 6);
    assert_eq!(screen.search_cursor, 10);
}

#[test]
fn dialog_double_click_timing_requires_same_screen_button_and_strict_threshold() {
    let start = Instant::now();
    let mut input = ClientInputState::new(true);
    assert!(!input.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Pressed,
        7,
        start,
    ));
    assert!(input.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Pressed,
        7,
        start + DIALOG_DOUBLE_CLICK_THRESHOLD - Duration::from_millis(1),
    ));

    let mut exact = ClientInputState::new(true);
    assert!(!exact.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Pressed,
        7,
        start,
    ));
    assert!(!exact.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Pressed,
        7,
        start + DIALOG_DOUBLE_CLICK_THRESHOLD,
    ));

    let mut identity = ClientInputState::new(true);
    assert!(!identity.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Pressed,
        7,
        start,
    ));
    assert!(!identity.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Released,
        7,
        start + Duration::from_millis(50),
    ));
    assert!(!identity.dialog_mouse_click_is_double_at(
        MouseButton::Right,
        ElementState::Pressed,
        7,
        start + Duration::from_millis(100),
    ));
    assert!(!identity.dialog_mouse_click_is_double_at(
        MouseButton::Right,
        ElementState::Pressed,
        8,
        start + Duration::from_millis(150),
    ));
    identity.clear_pressed();
    assert!(!identity.dialog_mouse_click_is_double_at(
        MouseButton::Right,
        ElementState::Pressed,
        8,
        start + Duration::from_millis(200),
    ));
}

#[test]
fn dialog_ime_preedit_maps_utf8_cursor_ranges_and_blinks_at_vanilla_period() {
    let start = Instant::now();
    let mut input = ClientInputState::new(true);

    assert!(input.handle_dialog_ime_preedit_at("a\u{0007}中b", Some((2, 5)), 7, start,));
    assert_eq!(
        input.dialog_ime_preedit_hud_state_at(start + Duration::from_millis(299)),
        Some(DialogImePreeditHudState {
            text: "a中b".to_string(),
            caret: Some(2),
            focused_start: 1,
            focused_end: 2,
            screen_revision: 7,
            caret_visible: true,
        })
    );
    assert!(
        !input
            .dialog_ime_preedit_hud_state_at(start + Duration::from_millis(300))
            .unwrap()
            .caret_visible
    );
    assert!(
        input
            .dialog_ime_preedit_hud_state_at(start + Duration::from_millis(600))
            .unwrap()
            .caret_visible
    );

    assert!(input.handle_dialog_ime_preedit_at("compose", Some((3, 3)), 8, start));
    let point_cursor = input.dialog_ime_preedit_hud_state_at(start).unwrap();
    assert_eq!(point_cursor.caret, Some(3));
    assert_eq!(
        (point_cursor.focused_start, point_cursor.focused_end),
        (0, 7)
    );

    assert!(input.handle_dialog_ime_preedit_at("compose", None, 8, start));
    let hidden_cursor = input.dialog_ime_preedit_hud_state_at(start).unwrap();
    assert_eq!(hidden_cursor.caret, None);
    assert_eq!(
        (hidden_cursor.focused_start, hidden_cursor.focused_end),
        (0, 7)
    );

    input.clear_pressed();
    assert_eq!(input.dialog_ime_preedit_hud_state_at(start), None);
    assert!(input.handle_dialog_ime_preedit_at("", None, 8, start));
    assert_eq!(input.dialog_ime_preedit_hud_state_at(start), None);
}

#[test]
fn debug_options_screen_search_double_click_requires_vanilla_threshold() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("alpha beta"));
    input
        .debug_options_screen
        .as_mut()
        .unwrap()
        .last_left_click_at = Some(Instant::now() - DEBUG_OPTIONS_DOUBLE_CLICK_THRESHOLD);
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 8),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));
    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_cursor, 8);
    assert_eq!(screen.search_selection, 8);
}

#[test]
fn debug_options_screen_search_shift_click_extends_selection() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("chunk stats"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 5),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));

    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_cursor, 5);
    assert_eq!(screen.search_selection, 11);
}

#[test]
fn debug_options_screen_buttons_update_status_and_profiles() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("entity_hitboxes"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let content_x = debug_options_content_x(surface);
    let buttons_start = content_x + DEBUG_OPTIONS_ROW_WIDTH - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    let always_x = buttons_start + DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 2 + 2;
    let entry_y = DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT + 2;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(always_x),
            f64::from(entry_y)
        )),
        surface,
        false,
    ));
    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::EntityHitboxes),
        DebugScreenEntryStatus::AlwaysOn
    );
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(state.default_profile_active);
    assert!(state.performance_profile_active);

    let (_, performance_x, _) = debug_options_footer_button_xs(surface);
    let footer_y = debug_options_footer_button_y(surface) + 2;
    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(performance_x + 2),
            f64::from(footer_y)
        )),
        surface,
        false,
    ));
    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::Fps),
        DebugScreenEntryStatus::AlwaysOn
    );
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(state.default_profile_active);
    assert!(!state.performance_profile_active);
}

#[test]
fn debug_options_screen_projects_button_hover_state() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("entity_hitboxes"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let content_x = debug_options_content_x(surface);
    let buttons_start = content_x + DEBUG_OPTIONS_ROW_WIDTH - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    let always_x = buttons_start + DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 2 + 2;
    let entry_y = DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT + 2;

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(always_x),
            f64::from(entry_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    let hovered_status = state.rows.iter().find_map(|row| match row {
        DebugOptionsScreenHudRow::Entry {
            entry,
            hovered_status,
            ..
        } if *entry == DebugScreenEntryId::EntityHitboxes => *hovered_status,
        _ => None,
    });
    assert_eq!(hovered_status, Some(DebugScreenEntryStatus::AlwaysOn));
    assert!(!state.default_profile_hovered);
    assert!(!state.performance_profile_hovered);
    assert!(!state.done_hovered);

    let (default_x, performance_x, done_x) = debug_options_footer_button_xs(surface);
    let footer_y = debug_options_footer_button_y(surface) + 2;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(default_x + 2),
            f64::from(footer_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(state.default_profile_hovered);
    assert!(!state.performance_profile_hovered);
    assert!(!state.done_hovered);

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(performance_x + 2),
            f64::from(footer_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(!state.default_profile_hovered);
    assert!(state.performance_profile_hovered);
    assert!(!state.done_hovered);

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(done_x + 2),
            f64::from(footer_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(!state.default_profile_hovered);
    assert!(!state.performance_profile_hovered);
    assert!(state.done_hovered);
}

#[test]
fn debug_options_screen_projects_not_allowed_tooltip_on_entry_name_hover() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("biome"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let content_x = debug_options_content_x(surface);
    let name_x = content_x + 2;
    let entry_y = DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT + 2;

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(name_x),
            f64::from(entry_y)
        )),
        surface,
    ));
    let reduced = input
        .debug_options_screen_hud_state(surface, true)
        .expect("screen should project under reduced debug info");
    assert_eq!(
        reduced
            .tooltip
            .as_ref()
            .map(|tooltip| tooltip.text.as_str()),
        Some("Not visible when debug info is reduced")
    );
    assert_eq!(
        reduced
            .tooltip
            .as_ref()
            .map(|tooltip| (tooltip.x, tooltip.y)),
        Some((name_x, entry_y))
    );

    let full = input
        .debug_options_screen_hud_state(surface, false)
        .expect("screen should project without reduced debug info");
    assert_eq!(full.tooltip, None);

    let buttons_start = content_x + DEBUG_OPTIONS_ROW_WIDTH - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(buttons_start),
            f64::from(entry_y)
        )),
        surface,
    ));
    let over_button = input
        .debug_options_screen_hud_state(surface, true)
        .expect("screen should project with button hover");
    assert_eq!(over_button.tooltip, None);
}

#[test]
fn debug_options_screen_scrollbar_drag_updates_scroll_row() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let scrollbar_x = debug_options_scrollbar_x(surface) + 1;
    let start_y = DEBUG_OPTIONS_HEADER_HEIGHT + 29;

    assert_eq!(
        debug_options_scrollbar_drag_scroll_row(0, 47, surface, start_y, start_y + 3),
        1
    );
    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(start_y)
        )),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(start_y + 3)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.scroll_row, 1);

    let below_list_y = DEBUG_OPTIONS_HEADER_HEIGHT + debug_options_list_height(surface) + 1;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(below_list_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.scroll_row, 40);

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Released,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(below_list_y)
        )),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(DEBUG_OPTIONS_HEADER_HEIGHT)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.scroll_row, 40);
}

#[test]
fn debug_options_screen_consumes_keys_scrolls_and_allows_f3_global_keymap() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    let surface = winit::dpi::PhysicalSize::new(420, 161);

    assert!(input.handle_debug_options_screen_mouse_wheel(
        winit::event::MouseScrollDelta::LineDelta(0.0, -3.0),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.visible_rows, 3);
    assert_eq!(state.scroll_row, 3);

    assert!(!input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::F3), ElementState::Pressed));
    assert!(input.debug_options_screen_is_open());
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed
    ));
    assert!(!input.debug_options_screen_is_open());
}

#[test]
fn f3_escape_records_pause_without_menu_request_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_pause_without_menu_requests(), 1);
    assert_eq!(input.take_debug_pause_without_menu_requests(), 0);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn releasing_active_input_preserves_f3_escape_debug_action_state() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_pause_without_menu_requests(), 1);

    release_active_input(&mut input, &mut world, &mut counters, &None);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_pause_screen_consumes_gameplay_keys_and_escape_closes() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    input.open_debug_pause_screen_without_menu();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert!(input.debug_pause_screen_is_open());
    assert!(!input.forward);
    assert!(!input.pressed_keys.contains(&KeyCode::KeyW));

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.debug_pause_screen_is_open());
    assert!(!input.pressed_keys.contains(&KeyCode::Escape));
}

#[test]
fn escape_opens_pause_screen_with_menu() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(
        input.debug_pause_screen(),
        Some(DebugPauseScreenState {
            show_pause_menu: true,
            cursor_position: None
        })
    );
    assert!(!input.pressed_keys.contains(&KeyCode::Escape));
}

#[test]
fn debug_pause_screen_return_to_game_button_closes_menu_screen() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(input.handle_debug_pause_screen_cursor_moved(Some(PhysicalPosition::new(68.0, 78.0))));
    assert_eq!(
        input.debug_pause_screen(),
        Some(DebugPauseScreenState {
            show_pause_menu: true,
            cursor_position: Some((68, 78))
        })
    );
    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 78.0)),
        surface,
    ));

    assert!(!input.debug_pause_screen_is_open());
}

#[test]
fn debug_pause_screen_advancements_button_opens_advancements_screen() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_displayed_advancement("minecraft:story/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    input.open_debug_pause_screen_with_menu();

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 102.0)),
        surface,
    ));

    assert!(!input.debug_pause_screen_is_open());
    assert!(world.advancements_screen_is_open());
    assert_eq!(
        world.selected_advancements_tab(),
        Some("minecraft:story/root")
    );
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:story/root".to_string()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn debug_pause_screen_stats_button_opens_stats_screen_and_requests_stats() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(170.0, 102.0)),
        surface,
    ));

    assert!(!input.debug_pause_screen_is_open());
    assert!(world.stats_screen_is_open());
    assert_eq!(counters.request_stats_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::RequestStats);
    assert!(rx.try_recv().is_err());
}

#[test]
fn social_interactions_pause_player_reporting_button_opens_with_pause_return() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(pause_screen_player_reporting_button_enabled());
    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(170.0, 150.0)),
        surface,
    ));

    assert!(!input.debug_pause_screen_is_open());
    assert!(world.social_interactions_screen().is_some());
    assert!(input.social_interactions_runtime().is_some());
    assert!(input.handle_social_interactions_key(&mut world, KeyCode::Escape, true));
    assert!(world.social_interactions_screen().is_none());
    assert!(input.debug_pause_screen_is_open());
}

#[test]
fn debug_pause_screen_send_feedback_button_records_link_request() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 126.0)),
        surface,
    ));

    assert!(input.debug_pause_screen_is_open());
    assert_eq!(
        input.take_pause_screen_link_requests(),
        vec![PauseScreenLinkRequest::SendFeedback {
            url: "https://aka.ms/javafeedback?ref=game"
        }]
    );
}

#[test]
fn debug_pause_screen_report_bugs_button_records_link_request_when_enabled() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(pause_screen_report_bugs_button_enabled());
    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(170.0, 126.0)),
        surface,
    ));

    assert!(input.debug_pause_screen_is_open());
    assert_eq!(
        input.take_pause_screen_link_requests(),
        vec![PauseScreenLinkRequest::ReportBugs {
            url: "https://aka.ms/snapshotbugs?ref=game"
        }]
    );
}

#[test]
fn debug_pause_screen_disconnect_button_records_disconnect_request() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 198.0)),
        surface,
    ));

    assert!(input.debug_pause_screen_is_open());
    assert!(input.pause_screen_disconnect_requested());
    assert!(input.take_pause_screen_link_requests().is_empty());
}

#[test]
fn debug_pause_screen_still_allows_global_f3_overlay_toggle() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    input.open_debug_pause_screen_without_menu();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(input.debug_pause_screen_is_open());
    assert!(input.debug_overlay_visible());
}

#[test]
fn f3_c_copies_location_tp_command_to_clipboard_and_reports_feedback() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.25,
            y: 64.0,
            z: -5.75,
        },
        y_rot: 90.5,
        x_rot: -15.25,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/execute in minecraft:overworld run tp @s 10.25 64.00 -5.75 90.50 -15.25")
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(messages[0].content, "[Debug]: Copied location to clipboard");
    assert_eq!(world.counters().chat_messages_tracked, 1);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_c_reduced_debug_blocks_location_copy_but_still_consumes_crash_modifier() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(true);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.25,
            y: 64.0,
            z: -5.75,
        },
        y_rot: 90.5,
        x_rot: -15.25,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert_eq!(clipboard.text, None);
    assert!(world.client_chat().messages.is_empty());

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_c_hold_reports_manual_crash_warning_countdown_without_toggling_overlay() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    let start = Instant::now();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
    );

    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(999),
    );
    assert!(world.client_chat().messages.is_empty());

    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_secs(1),
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_secs(2),
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages[0].content,
        "[Debug]: F3 + C is held down. This will crash the game unless released."
    );
    assert_eq!(messages[1].content, "[Debug]: Crashing in 8...");

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Released,
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_secs(3),
    );
    assert_eq!(world.client_chat().messages.len(), 2);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
#[should_panic(expected = "Manually triggered debug crash")]
fn f3_c_hold_panics_after_vanilla_manual_crash_delay() {
    let (tx, _rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    let start = Instant::now();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
    );

    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(10_001),
    );
}

#[test]
fn f3_game_mode_keys_report_no_permission_without_gameplay_commands() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyN),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(!input.debug_overlay_visible());
    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(
        feedback,
        vec![
            "[Debug]: Unable to switch game mode; no permission",
            "[Debug]: Unable to open game mode switcher; no permission",
        ]
    );
    assert!(world
        .client_chat()
        .messages
        .iter()
        .all(|message| message.kind == ChatMessageKind::ClientSystem));
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(counters.change_game_mode_commands_queued, 0);
    assert_eq!(input.debug_game_mode_switcher_selected(), None);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_n_queues_spectator_game_mode_with_permission() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyN),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(!input.debug_overlay_visible());
    assert!(world.client_chat().messages.is_empty());
    assert_eq!(counters.change_game_mode_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
            game_mode: bbb_protocol::packets::GameType::Spectator,
        })
    );
}

#[test]
fn f3_n_queues_previous_or_creative_game_mode_when_already_spectator() {
    for (previous_game_type, expected_game_mode) in [
        (-1, bbb_protocol::packets::GameType::Creative),
        (0, bbb_protocol::packets::GameType::Survival),
        (2, bbb_protocol::packets::GameType::Adventure),
    ] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_debug_player_in_game_mode(false, 3, previous_game_type);
        grant_debug_recreate_nbt_permission(&mut world);
        assert!(world.local_player_is_spectator());

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
        );
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::KeyN),
            ElementState::Pressed,
        );

        assert_eq!(counters.change_game_mode_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
                game_mode: expected_game_mode,
            })
        );
    }
}

#[test]
fn f3_f4_game_mode_switcher_queues_default_selection_on_f3_release() {
    for (game_type, previous_game_type, expected_game_mode) in [
        (0, -1, bbb_protocol::packets::GameType::Creative),
        (1, -1, bbb_protocol::packets::GameType::Survival),
        (3, 0, bbb_protocol::packets::GameType::Survival),
    ] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_debug_player_in_game_mode(false, game_type, previous_game_type);
        grant_debug_recreate_nbt_permission(&mut world);

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
        );
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F4),
            ElementState::Pressed,
        );

        assert_eq!(
            input.debug_game_mode_switcher_selected(),
            Some(expected_game_mode)
        );
        assert_eq!(counters.change_game_mode_commands_queued, 0);
        assert!(rx.try_recv().is_err());

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Released,
        );

        assert_eq!(input.debug_game_mode_switcher_selected(), None);
        assert!(!input.debug_overlay_visible());
        assert_eq!(counters.change_game_mode_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
                game_mode: expected_game_mode,
            })
        );
    }
}

#[test]
fn f3_f4_game_mode_switcher_cycles_with_additional_f4_presses() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    for _ in 0..3 {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F4),
            ElementState::Pressed,
        );
    }
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Adventure)
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(counters.change_game_mode_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
            game_mode: bbb_protocol::packets::GameType::Adventure,
        })
    );
}

#[test]
fn f3_f4_game_mode_switcher_does_not_open_over_existing_screen() {
    #[derive(Debug, Clone, Copy)]
    enum ExistingScreen {
        Chat,
        LocalInventory,
        Book,
        Dialog,
        Advancements,
        SignEditor,
    }

    for existing_screen in [
        ExistingScreen::Chat,
        ExistingScreen::LocalInventory,
        ExistingScreen::Book,
        ExistingScreen::Dialog,
        ExistingScreen::Advancements,
        ExistingScreen::SignEditor,
    ] {
        let commands = None;
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);

        match existing_screen {
            ExistingScreen::Chat => handle_key_input(
                &mut input,
                &mut counters,
                &mut world,
                &commands,
                PhysicalKey::Code(KeyCode::KeyT),
                ElementState::Pressed,
            ),
            ExistingScreen::LocalInventory => {
                assert!(world.open_local_inventory());
            }
            ExistingScreen::Book => open_test_book_screen(&mut world, vec!["First"]),
            ExistingScreen::Dialog => world.apply_show_dialog(ProtocolShowDialog {
                dialog: DialogHolder::Reference { registry_id: 11 },
            }),
            ExistingScreen::Advancements => {
                assert!(world.open_advancements_screen());
            }
            ExistingScreen::SignEditor => {
                let pos = BlockPos { x: 1, y: 2, z: 3 };
                insert_sign_text(&mut world, pos, ["", "", "", ""], ["", "", "", ""]);
                world.apply_open_sign_editor(OpenSignEditor {
                    pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
                    is_front_text: true,
                });
            }
        }

        assert!(input.handle_debug_overlay_key(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
            Some(&mut world),
            None
        ));
        assert!(
            !input.handle_debug_overlay_key(
                PhysicalKey::Code(KeyCode::F4),
                ElementState::Pressed,
                Some(&mut world),
                None
            ),
            "F3+F4 should not be a debug action over {existing_screen:?}"
        );
        assert_eq!(input.debug_game_mode_switcher_selected(), None);
        assert!(world.client_chat().messages.is_empty());
        assert!(input.handle_debug_overlay_key(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Released,
            Some(&mut world),
            None
        ));
        assert!(
            input.debug_overlay_visible(),
            "blocked F4 should leave F3 release as an ordinary overlay toggle over {existing_screen:?}"
        );
    }
}

#[test]
fn f3_f4_game_mode_switcher_hover_uses_first_mouse_suppression() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let surface_size = PhysicalSize::new(320, 240);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Creative)
    );

    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(176.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Creative),
        "first mouse position only arms hover detection"
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(176.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Creative),
        "same as first mouse position stays suppressed"
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(207.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Spectator)
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(counters.change_game_mode_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
            game_mode: bbb_protocol::packets::GameType::Spectator,
        })
    );
}

#[test]
fn f3_f4_game_mode_switcher_f4_cycle_resets_first_mouse_suppression() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let surface_size = PhysicalSize::new(320, 240);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(145.0, 102.0)),
        surface_size
    ));
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(176.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Adventure)
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Spectator)
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(114.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Spectator),
        "cursor hover is suppressed again after F4 cycling"
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(145.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Survival)
    );
}

#[test]
fn slash_text_opens_command_entry_and_releases_pressed_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/");

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/")
    );
    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/".to_string(),
        })
    );
    assert_eq!(
        world.command_suggestions().last_request.as_ref(),
        Some(&bbb_world::CommandSuggestionRequestState {
            id: 0,
            command: "/".to_string(),
        })
    );
}

#[test]
fn command_entry_submits_slash_command_without_leading_slash() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/time set day",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/time set day".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(OutboundChatCommand::unsigned("time set day"))
    );
}

#[test]
fn command_entry_with_signable_message_argument_submits_signed_command() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(signable_message_command_tree());
    let _ = world.apply_player_chat(player_chat_with_signature(
        0,
        MessageSignature {
            bytes: vec![12; 256],
        },
    ));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/say hello",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/say hello".to_string(),
        })
    );
    match rx.try_recv().unwrap() {
        NetCommand::ChatCommand(packet) => {
            assert_eq!(packet.command, "say hello");
            let signing = packet.signing.expect("signable command signing context");
            assert!(signing.timestamp_millis > 0);
            assert_ne!(signing.salt, 0);
            assert_eq!(signing.arguments[0].name, "message");
            assert_eq!(signing.arguments[0].value, "hello");
            assert_eq!(signing.last_seen.update.offset, 1);
            // The fixture sender has no PlayerInfo entry, so vanilla presents
            // a validation error and advances only the hidden-message offset.
            assert_eq!(signing.last_seen.update.acknowledged, 0);
            assert_eq!(
                signing.last_seen.update.checksum,
                LastSeenMessagesUpdate::default().checksum
            );
            assert!(signing.last_seen.signatures.is_empty());
        }
        command => panic!("expected chat command request, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
    assert_eq!(
        world.counters().player_chat_acknowledgement_pending_offset,
        0
    );
}

#[test]
fn chat_key_opens_chat_entry_and_submits_unsigned_message() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("")
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "hello   world",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.chat_message_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert_eq!(counters.command_suggestion_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => {
            assert_eq!(packet.message, "hello world");
            assert!(packet.timestamp_millis > 0);
            assert_ne!(packet.salt, 0);
            assert_eq!(packet.last_seen.update, LastSeenMessagesUpdate::default());
            assert!(packet.last_seen.signatures.is_empty());
        }
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn dialog_stack_suspends_and_restores_the_native_chat_screen() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    open_chat_entry(&mut input, &mut world, &mut counters, &commands, false);
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "draft command",
    );
    let entry = input.chat_entry.as_mut().unwrap();
    entry.cursor = 5;
    entry.selection = 11;
    let expected = entry.clone();

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert!(!input.chat_entry_is_active());

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 12 },
    });
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert!(!input.chat_entry_is_active());

    world.apply_clear_dialog();
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert_eq!(input.chat_entry.as_ref(), Some(&expected));
    assert_eq!(world.counters().show_dialog_packets, 2);
    assert_eq!(world.counters().clear_dialog_packets, 1);
}

#[test]
fn native_chat_screen_is_not_restored_after_another_screen_replaces_dialog() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    open_chat_entry(&mut input, &mut world, &mut counters, &commands, false);
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "discarded draft",
    );
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert!(!input.chat_entry_is_active());

    open_test_book_screen(&mut world, vec!["Replacement"]);
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert!(!input.chat_entry_is_active());
    assert!(world.current_book().is_some());

    assert!(world.close_current_book());
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert!(!input.chat_entry_is_active());
}

#[test]
fn dialog_warning_restores_the_native_debug_options_screen_after_back() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let commands = None;
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(420, 161);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("chunk"));
    assert!(input
        .handle_debug_options_screen_mouse_wheel(MouseScrollDelta::LineDelta(0.0, -2.0), surface,));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(PhysicalPosition::new(210.0, 80.0)),
        surface,
    ));
    let expected = input.debug_options_screen.clone().unwrap();

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert!(!input.debug_options_screen_is_open());

    assert!(world.open_dialog_warning());
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 12 },
    });
    world.apply_clear_dialog();
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert!(world.dialog_warning_is_open());
    assert!(!input.debug_options_screen_is_open());

    assert!(world.close_dialog_warning());
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert_eq!(input.debug_options_screen.as_ref(), Some(&expected));
    assert_eq!(
        input
            .debug_options_screen_hud_state(surface, false)
            .unwrap()
            .search_text,
        "chunk"
    );
}

#[test]
fn dialog_clear_restores_the_native_pause_screen_state() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let commands = None;
    let mut world = WorldStore::new();
    input.open_debug_pause_screen_with_menu();
    assert!(input.handle_debug_pause_screen_cursor_moved(Some(PhysicalPosition::new(68.0, 198.0,))));
    input.pause_screen_disconnect_requested = true;
    let expected = input.debug_pause_screen.unwrap();

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert!(!input.debug_pause_screen_is_open());
    assert!(!input.pause_screen_disconnect_requested());

    world.apply_clear_dialog();
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert_eq!(input.debug_pause_screen(), Some(expected));
    assert!(input.pause_screen_disconnect_requested());
}

#[test]
fn chat_entry_limits_message_text_to_vanilla_length() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &"a".repeat(CHAT_ENTRY_MAX_LENGTH + 20),
    );

    let entry = input.chat_entry.as_ref().unwrap();
    assert_eq!(entry.text.chars().count(), CHAT_ENTRY_MAX_LENGTH);
    assert_eq!(entry.cursor, CHAT_ENTRY_MAX_LENGTH);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => {
            assert_eq!(packet.message.chars().count(), CHAT_ENTRY_MAX_LENGTH);
            assert_eq!(packet.message, "a".repeat(CHAT_ENTRY_MAX_LENGTH));
        }
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_mid_cursor_insert_only_uses_remaining_vanilla_length() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &"b".repeat(CHAT_ENTRY_MAX_LENGTH - 1),
    );
    let entry = input.chat_entry.as_mut().unwrap();
    entry.cursor = 10;
    entry.selection = 10;

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "XYZ");

    let entry = input.chat_entry.as_ref().unwrap();
    let expected = format!(
        "{}{}{}",
        "b".repeat(10),
        "X",
        "b".repeat(CHAT_ENTRY_MAX_LENGTH - 11)
    );
    assert_eq!(entry.text.chars().count(), CHAT_ENTRY_MAX_LENGTH);
    assert_eq!(entry.text, expected);
    assert_eq!(entry.cursor, 11);
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_filters_vanilla_disallowed_formatting_character() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &format!("he{}llo", '\u{a7}'),
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("hello")
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => assert_eq!(packet.message, "hello"),
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_filters_vanilla_disallowed_formatting_character() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &format!("/s{}eed", '\u{a7}'),
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/seed")
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string(),
        })
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(OutboundChatCommand::unsigned("seed"))
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_key_submits_message_with_pending_last_seen_update() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let _ = world.apply_player_chat(player_chat_with_signature(
        0,
        MessageSignature {
            bytes: vec![12; 256],
        },
    ));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "reply");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => {
            assert_eq!(packet.message, "reply");
            assert_eq!(packet.last_seen.update.offset, 1);
            // Unknown-sender validation errors are processed as not shown:
            // their offset is acknowledged but no signature enters last-seen.
            assert_eq!(packet.last_seen.update.acknowledged, 0);
            assert_eq!(
                packet.last_seen.update.checksum,
                LastSeenMessagesUpdate::default().checksum
            );
            assert!(packet.last_seen.signatures.is_empty());
        }
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert_eq!(
        world.counters().player_chat_acknowledgement_pending_offset,
        0
    );
}

#[test]
fn chat_entry_starting_with_slash_requests_suggestions_and_submits_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/seed");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert_eq!(counters.chat_message_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(OutboundChatCommand::unsigned("seed"))
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_blocks_movement_keys_and_backspace_edits_text() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/givw");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "e");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/give")
    );
    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.command_suggestion_commands_queued, 3);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/givw".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 1,
            command: "/giv".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 2,
            command: "/give".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_cursor_editing_updates_slash_suggestions() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/givve");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );

    let entry = input
        .chat_entry
        .as_ref()
        .expect("command entry should stay open");
    assert_eq!(entry.text, "/give");
    assert_eq!(entry.cursor, 4);
    assert_eq!(counters.command_suggestion_commands_queued, 2);
    for (id, command) in [(0, "/givve"), (1, "/give")] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
                id,
                command: command.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_control_word_keys_edit_text() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/say alpha beta gamma",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowRight),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );

    let entry = input
        .chat_entry
        .as_ref()
        .expect("command entry should stay open");
    assert_eq!(entry.text, "/say alpha ");
    assert_eq!(entry.cursor, 11);
    assert_eq!(counters.command_suggestion_commands_queued, 3);
    for (id, command) in [
        (0, "/say alpha beta gamma"),
        (1, "/say alpha gamma"),
        (2, "/say alpha "),
    ] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
                id,
                command: command.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_control_a_selection_replaces_and_submits_command() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/gamemode creative",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/time set day",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 2);
    assert_eq!(counters.chat_command_commands_queued, 1);
    for (id, command) in [(0, "/gamemode creative"), (1, "/time set day")] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
                id,
                command: command.to_string(),
            })
        );
    }
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(OutboundChatCommand::unsigned("time set day"))
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_control_a_selection_replaces_message_before_submit() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "wrong message",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "fixed message",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.chat_message_commands_queued, 1);
    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => assert_eq!(packet.message, "fixed message"),
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_cursor_editing_submits_corrected_message() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "helo");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "l");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 0);
    assert_eq!(counters.chat_message_commands_queued, 1);
    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => assert_eq!(packet.message, "hello"),
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_escape_cancels_without_queuing_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/seed");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn slash_only_enter_requests_suggestions_but_does_not_submit_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_multi_character_commit_requests_suggestions_once() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/seed");

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/seed")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_tab_applies_latest_suggestion_range() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(local_literal_command_tree(&["give"]));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/giv");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/give")
    );
    assert_eq!(
        input
            .chat_entry
            .as_ref()
            .and_then(|entry| entry.last_suggestion_request_text.as_deref()),
        Some("/give")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(OutboundChatCommand::unsigned("give"))
    );
}

#[test]
fn command_suggestion_list_cycles_applies_suffix_and_preserves_narration_state() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/ga");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 2,
        suggestions: vec![
            CommandSuggestion {
                text: "gamemode".to_string(),
                tooltip: None,
                tooltip_styled: None,
                tooltip_component: None,
            },
            CommandSuggestion {
                text: "gamerule".to_string(),
                tooltip: Some("Change a game rule".to_string()),
                tooltip_styled: Some(vec![StyledTextRun {
                    text: "Change a game rule".to_string(),
                    style: ComponentStyle {
                        bold: Some(true),
                        color: Some(0x55_AA_FF),
                        ..Default::default()
                    },
                }]),
                tooltip_component: None,
            },
            CommandSuggestion {
                text: "give".to_string(),
                tooltip: None,
                tooltip_styled: None,
                tooltip_component: None,
            },
        ],
    });

    let hud = input
        .command_suggestion_hud_state(
            &world,
            &FixedCommandSuggestionTextMeasurer,
            PhysicalSize::new(320, 240),
        )
        .expect("open command entry");
    assert_eq!(hud.suffix.as_deref(), Some("memode"));
    assert_eq!(hud.entries.len(), 3);
    assert!(hud.entries[0].selected);
    assert_eq!(input.command_suggestion_narration(), None);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowDown),
        ElementState::Pressed,
    );
    assert_eq!(
        input.command_suggestion_narration(),
        Some(&CommandSuggestionNarrationState {
            position: 2,
            total: 3,
            text: "gamerule".to_string(),
            tooltip: Some("Change a game rule".to_string()),
            tooltip_translation_key: None,
        })
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.chat_entry.as_ref().unwrap().text, "/gamerule");
    assert_eq!(input.chat_entry.as_ref().unwrap().cursor, 9);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.chat_entry.as_ref().unwrap().text, "/give");

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.chat_entry.as_ref().unwrap().text, "/gamerule");
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/ga".to_string(),
        })
    );
}

#[test]
fn command_suggestion_list_limits_rows_scrolls_clicks_and_shows_rich_tooltip() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/c");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 1,
        suggestions: (0..12)
            .map(|index| CommandSuggestion {
                text: format!("cmd{index:02}"),
                tooltip: (index == 2).then(|| "Rich help".to_string()),
                tooltip_styled: (index == 2).then(|| {
                    vec![StyledTextRun {
                        text: "Rich help".to_string(),
                        style: ComponentStyle {
                            italic: Some(true),
                            underlined: Some(true),
                            color: Some(0x12_34_56),
                            ..Default::default()
                        },
                    }]
                }),
                tooltip_component: None,
            })
            .collect(),
    });
    let surface = PhysicalSize::new(320, 240);
    let hud = input
        .command_suggestion_hud_state(&world, &FixedCommandSuggestionTextMeasurer, surface)
        .expect("open command entry");
    assert_eq!(hud.rect.x, 9);
    assert_eq!(hud.rect.y, 105);
    assert_eq!(hud.rect.width, 31);
    assert_eq!(hud.rect.height, 120);
    assert_eq!(hud.entries.len(), 10);
    assert!(!hud.has_previous);
    assert!(hud.has_next);

    assert!(input
        .handle_command_suggestion_cursor_moved(&world, Some(PhysicalPosition::new(11.0, 131.0)),));
    let hovered = input
        .command_suggestion_hud_state(&world, &FixedCommandSuggestionTextMeasurer, surface)
        .unwrap();
    let tooltip = hovered.tooltip.expect("selected rich tooltip");
    assert_eq!(tooltip.text, "Rich help");
    assert_eq!(tooltip.runs[0].style.italic, Some(true));
    assert_eq!(tooltip.runs[0].style.underlined, Some(true));
    assert_eq!(tooltip.runs[0].style.color, Some(0x12_34_56));
    assert_eq!(
        input.command_suggestion_narration(),
        Some(&CommandSuggestionNarrationState {
            position: 3,
            total: 12,
            text: "cmd02".to_string(),
            tooltip: Some("Rich help".to_string()),
            tooltip_translation_key: None,
        })
    );

    assert!(input.handle_command_suggestion_mouse_wheel(MouseScrollDelta::LineDelta(0.0, -1.0)));
    let scrolled = input
        .command_suggestion_hud_state(&world, &FixedCommandSuggestionTextMeasurer, surface)
        .unwrap();
    assert!(scrolled.has_previous);
    assert!(scrolled.has_next);
    assert_eq!(scrolled.entries[0].text, "cmd01");

    assert!(!input.handle_command_suggestion_mouse_input(
        &world,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(500.0, 500.0)),
    ));
    assert!(input.handle_command_suggestion_mouse_input(
        &world,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(11.0, 155.0)),
    ));
    assert_eq!(input.chat_entry.as_ref().unwrap().text, "/cmd05");
    assert_eq!(input.chat_entry.as_ref().unwrap().cursor, 6);
    assert!(input.replace_chat_entry_text("/seed".to_string()));
    assert_eq!(input.chat_entry.as_ref().unwrap().text, "/seed");
    assert_eq!(input.chat_entry.as_ref().unwrap().cursor, 5);
    assert!(input.chat_entry.as_ref().unwrap().suggestion_list.is_none());
}

#[test]
fn test_block_editor_preserves_hidden_message_cycles_modes_and_done_sends_once() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_test_block(pos, TestBlockMode::Log, "initial");
    let layout = bbb_renderer::hud_test_block_editor_layout(PhysicalSize::new(320, 240));
    assert!(world.try_open_test_block_editor(pos));
    input.sync_test_block_editor(&mut world, &mut counters, &commands);

    let hud = input.test_block_editor_hud_state().unwrap();
    assert_eq!(hud.mode, TestBlockMode::Log);
    assert_eq!(hud.message, "initial");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &"😀".repeat(100),
    );
    assert_eq!(
        input
            .test_block_editor_hud_state()
            .unwrap()
            .message
            .encode_utf16()
            .count(),
        128
    );

    for expected in [
        TestBlockMode::Fail,
        TestBlockMode::Accept,
        TestBlockMode::Start,
    ] {
        assert!(click_test_block_editor(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            layout.mode,
        ));
        assert_eq!(input.test_block_editor_hud_state().unwrap().mode, expected);
    }
    let hidden_message = input.test_block_editor_hud_state().unwrap().message;
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "ignored while hidden",
    );
    assert_eq!(
        input.test_block_editor_hud_state().unwrap().message,
        hidden_message
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );
    assert!(input.test_block_editor_is_active());
    assert!(rx.try_recv().is_err(), "Enter must not submit the editor");

    assert!(click_test_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.mode,
    ));
    assert_eq!(
        input.test_block_editor_hud_state().unwrap().mode,
        TestBlockMode::Log
    );
    assert!(click_test_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.done,
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetTestBlock(SetTestBlock {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            mode: TestBlockMode::Log,
            message: hidden_message,
        })
    );
    assert_eq!(counters.set_test_block_commands_queued, 1);
    assert!(!input.test_block_editor_is_active());
    assert!(world.test_block_editor().is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn test_block_editor_right_click_cancel_and_escape_never_send_editor_packet() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_test_block(pos, TestBlockMode::Fail, "failure");

    assert!(mouse::start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(CrosshairTarget::Block(
            crate::crosshair::CrosshairBlockHit {
                pos,
                face: ProtocolDirection::Up,
                cursor: [0.25, 0.5, 0.75],
                inside: false,
            }
        )),
        None,
    ));
    assert!(input.test_block_editor_is_active());
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.test_block_editor_is_active());
    assert!(rx.try_recv().is_err());

    assert!(world.try_open_test_block_editor(pos));
    input.sync_test_block_editor(&mut world, &mut counters, &commands);
    let cancel = bbb_renderer::hud_test_block_editor_layout(PhysicalSize::new(320, 240)).cancel;
    assert!(click_test_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        cancel,
    ));
    assert!(!input.test_block_editor_is_active());
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.set_test_block_commands_queued, 0);
    assert_eq!(counters.use_item_on_commands_queued, 1);
}

#[test]
fn test_instance_block_editor_initializes_queries_and_applies_only_present_response_size() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_test_instance_block(pos);
    assert!(world.try_open_test_instance_block_editor(pos));
    input.sync_test_instance_block_editor(&mut world, &mut counters, &commands);

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::TestInstanceBlockAction(TestInstanceBlockAction {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            action: TestInstanceBlockActionKind::Init,
            test: Some("example:initial".to_string()),
            size: Vec3i { x: 3, y: 4, z: 5 },
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: false,
        })
    );
    input.sync_test_instance_block_editor(&mut world, &mut counters, &commands);
    assert!(
        rx.try_recv().is_err(),
        "same open must not send a second INIT"
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Bad ID");
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::TestInstanceBlockAction(TestInstanceBlockAction {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            action: TestInstanceBlockActionKind::Query,
            test: None,
            size: Vec3i { x: 3, y: 4, z: 5 },
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: false,
        })
    );
    assert!(
        input
            .test_instance_block_editor_hud_state()
            .unwrap()
            .local_invalid_id
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "new:test");
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::TestInstanceBlockAction(TestInstanceBlockAction {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            action: TestInstanceBlockActionKind::Query,
            test: Some("new:test".to_string()),
            size: Vec3i { x: 3, y: 4, z: 5 },
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: false,
        })
    );
    assert!(
        !input
            .test_instance_block_editor_hud_state()
            .unwrap()
            .local_invalid_id
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "33");
    world.apply_test_instance_block_status(TestInstanceBlockStatus {
        status: RichTextComponent::plain("No size"),
        size: None,
    });
    input.sync_test_instance_block_editor(&mut world, &mut counters, &commands);
    assert_eq!(
        input.test_instance_block_editor_hud_state().unwrap().fields[1].text,
        "33"
    );

    world.apply_test_instance_block_status(TestInstanceBlockStatus {
        status: RichTextComponent::plain("Sized"),
        size: Some(Vec3i { x: 8, y: 9, z: 10 }),
    });
    input.sync_test_instance_block_editor(&mut world, &mut counters, &commands);
    let state = input.test_instance_block_editor_hud_state().unwrap();
    assert_eq!(state.fields[1].text, "8");
    assert_eq!(state.fields[2].text, "9");
    assert_eq!(state.fields[3].text, "10");
    assert_eq!(state.response_status.unwrap().text(), "Sized");
    assert_eq!(counters.test_instance_block_action_commands_queued, 3);
}

#[test]
fn test_instance_block_editor_buttons_send_clamped_exact_actions_and_close_once() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(16);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_test_instance_block(pos);
    let layout = bbb_renderer::hud_test_instance_block_editor_layout(PhysicalSize::new(320, 240));
    assert!(world.try_open_test_instance_block_editor(pos));
    input.sync_test_instance_block_editor(&mut world, &mut counters, &commands);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::TestInstanceBlockAction(
            TestInstanceBlockAction {
                action: TestInstanceBlockActionKind::Init,
                ..
            }
        ))
    ));

    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.rotation,
    ));
    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.save,
    ));
    assert!(input.test_instance_block_editor_is_active());
    assert!(rx.try_recv().is_err(), "rotated Save is disabled");
    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.include_entities,
    ));
    for _ in 0..3 {
        assert!(click_test_instance_editor(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            layout.rotation,
        ));
    }
    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.size[0],
    ));
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "99");
    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.run,
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::TestInstanceBlockAction(TestInstanceBlockAction {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            action: TestInstanceBlockActionKind::Run,
            test: Some("example:initial".to_string()),
            size: Vec3i { x: 48, y: 4, z: 5 },
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: true,
        })
    );
    assert!(!input.test_instance_block_editor_is_active());
    assert!(world.test_instance_block_editor().is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn test_instance_block_editor_reset_save_and_done_buttons_map_to_exact_actions() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let layout = bbb_renderer::hud_test_instance_block_editor_layout(PhysicalSize::new(320, 240));
    for (expected_action, rect) in [
        (TestInstanceBlockActionKind::Reset, layout.reset),
        (TestInstanceBlockActionKind::Save, layout.save),
        (TestInstanceBlockActionKind::Set, layout.done),
    ] {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_test_instance_block(pos);
        assert!(world.try_open_test_instance_block_editor(pos));
        input.sync_test_instance_block_editor(&mut world, &mut counters, &commands);
        assert!(matches!(
            rx.try_recv(),
            Ok(NetCommand::TestInstanceBlockAction(
                TestInstanceBlockAction {
                    action: TestInstanceBlockActionKind::Init,
                    ..
                }
            ))
        ));
        assert!(click_test_instance_editor(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            rect,
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::TestInstanceBlockAction(TestInstanceBlockAction {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                action: expected_action,
                test: Some("example:initial".to_string()),
                size: Vec3i { x: 3, y: 4, z: 5 },
                rotation: TestInstanceBlockRotation::None,
                ignore_entities: false,
            })
        );
        assert_eq!(counters.test_instance_block_action_commands_queued, 2);
        assert!(!input.test_instance_block_editor_is_active());
        assert!(world.test_instance_block_editor().is_none());
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn test_instance_block_editor_cancel_escape_and_right_click_use_are_exact() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(12);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_test_instance_block(pos);

    assert!(mouse::start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(CrosshairTarget::Block(
            crate::crosshair::CrosshairBlockHit {
                pos,
                face: ProtocolDirection::Up,
                cursor: [0.25, 0.5, 0.75],
                inside: false,
            }
        )),
        None,
    ));
    assert!(input.test_instance_block_editor_is_active());
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::TestInstanceBlockAction(
            TestInstanceBlockAction {
                action: TestInstanceBlockActionKind::Init,
                ..
            }
        ))
    ));
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.test_instance_block_editor_is_active());
    assert!(rx.try_recv().is_err());

    assert!(world.try_open_test_instance_block_editor(pos));
    input.sync_test_instance_block_editor(&mut world, &mut counters, &commands);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::TestInstanceBlockAction(
            TestInstanceBlockAction {
                action: TestInstanceBlockActionKind::Init,
                ..
            }
        ))
    ));
    let cancel =
        bbb_renderer::hud_test_instance_block_editor_layout(PhysicalSize::new(320, 240)).cancel;
    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        cancel,
    ));
    assert!(!input.test_instance_block_editor_is_active());
    assert!(rx.try_recv().is_err());

    assert!(world.try_open_test_instance_block_editor(pos));
    input.sync_test_instance_block_editor(&mut world, &mut counters, &commands);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::TestInstanceBlockAction(
            TestInstanceBlockAction {
                action: TestInstanceBlockActionKind::Init,
                ..
            }
        ))
    ));
    assert!(world.open_local_inventory());
    input.sync_test_instance_block_editor(&mut world, &mut counters, &commands);
    assert!(!input.test_instance_block_editor_is_active());
    assert!(world.test_instance_block_editor().is_none());
    assert_eq!(counters.test_instance_block_action_commands_queued, 3);
    assert_eq!(counters.use_item_on_commands_queued, 1);
}

#[test]
fn jigsaw_block_editor_preloads_and_done_sends_exact_set_once() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_jigsaw_block(pos, "up_north");
    assert!(world.try_open_jigsaw_block_editor(pos));
    input.sync_jigsaw_block_editor(&mut world, &mut counters, &commands);

    let hud = input.jigsaw_block_editor_hud_state().unwrap();
    assert_eq!(hud.fields[0].text, "example:village/houses");
    assert_eq!(hud.fields[1].text, "example:entry");
    assert_eq!(hud.fields[2].text, "example:target");
    assert_eq!(hud.fields[3].text, "minecraft:oak_planks[axis=y]");
    assert_eq!(hud.fields[4].text, "-2");
    assert_eq!(hud.fields[5].text, "3");
    assert_eq!(hud.focus_index, 0);
    assert_eq!(hud.joint, JigsawJoint::Rollable);
    assert!(hud.joint_editable);
    assert_eq!(hud.levels, 0);
    assert!(hud.keep_jigsaws);
    assert!(hud.controls_enabled);

    let editor = input.jigsaw_block_editor.as_mut().unwrap();
    for (field, value) in editor.fields.iter_mut().zip([
        "example:new_pool",
        "new_entry",
        "example:new_target",
        "minecraft:stone",
        "-7",
        "bad",
    ]) {
        field.text = value.to_string();
    }
    editor.joint = JigsawJoint::Aligned;
    let done = bbb_renderer::hud_jigsaw_block_editor_layout(PhysicalSize::new(320, 240)).done;
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        done,
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetJigsawBlock(SetJigsawBlock {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            name: "minecraft:new_entry".to_string(),
            target: "example:new_target".to_string(),
            pool: "example:new_pool".to_string(),
            final_state: "minecraft:stone".to_string(),
            joint: JigsawJoint::Aligned,
            selection_priority: -7,
            placement_priority: 0,
        })
    );
    assert_eq!(counters.set_jigsaw_block_commands_queued, 1);
    assert_eq!(counters.jigsaw_generate_commands_queued, 0);
    assert!(!input.jigsaw_block_editor_is_active());
    assert!(world.jigsaw_block_editor().is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn jigsaw_generate_uses_exact_slider_toggle_joint_and_set_first_order() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_jigsaw_block(pos, "down_south");
    assert!(world.try_open_jigsaw_block_editor(pos));
    input.sync_jigsaw_block_editor(&mut world, &mut counters, &commands);
    let layout = bbb_renderer::hud_jigsaw_block_editor_layout(PhysicalSize::new(320, 240));

    assert!(input.handle_jigsaw_block_editor_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.levels.x + 50),
            f64::from(layout.levels.y + 1),
        )),
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert_eq!(input.jigsaw_block_editor_hud_state().unwrap().levels, 10);
    assert!(input.handle_jigsaw_block_editor_cursor_moved(
        Some(PhysicalPosition::new(
            f64::from(layout.levels.x + 96),
            f64::from(layout.levels.y + 1),
        )),
        PhysicalSize::new(320, 240),
    ));
    assert_eq!(input.jigsaw_block_editor_hud_state().unwrap().levels, 20);
    assert!(input.handle_jigsaw_block_editor_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        None,
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.keep_jigsaws,
    ));
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.joint,
    ));
    let hud = input.jigsaw_block_editor_hud_state().unwrap();
    assert!(!hud.keep_jigsaws);
    assert_eq!(hud.joint, JigsawJoint::Aligned);

    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.generate,
    ));
    assert!(matches!(rx.try_recv(), Ok(NetCommand::SetJigsawBlock(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::JigsawGenerate(JigsawGenerate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            levels: 20,
            keep_jigsaws: false,
        })
    );
    assert_eq!(counters.set_jigsaw_block_commands_queued, 1);
    assert_eq!(counters.jigsaw_generate_commands_queued, 1);
    assert!(rx.try_recv().is_err());
}

#[test]
fn jigsaw_editor_validation_limits_cancel_escape_and_screen_replacement_are_local() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_jigsaw_block(pos, "up_north");
    assert!(world.try_open_jigsaw_block_editor(pos));
    input.sync_jigsaw_block_editor(&mut world, &mut counters, &commands);
    let layout = bbb_renderer::hud_jigsaw_block_editor_layout(PhysicalSize::new(320, 240));
    assert!(input.handle_jigsaw_block_editor_key(
        &mut counters,
        &mut world,
        &commands,
        KeyCode::Tab,
        true,
    ));
    assert_eq!(
        input.jigsaw_block_editor_hud_state().unwrap().focus_index,
        1
    );
    input.set_shift_key(KeyCode::ShiftLeft, true);
    assert!(input.handle_jigsaw_block_editor_key(
        &mut counters,
        &mut world,
        &commands,
        KeyCode::Tab,
        true,
    ));
    input.set_shift_key(KeyCode::ShiftLeft, false);
    assert_eq!(
        input.jigsaw_block_editor_hud_state().unwrap().focus_index,
        0
    );
    input.jigsaw_block_editor.as_mut().unwrap().fields[0].text = "Bad Pool".to_string();
    assert!(
        !input
            .jigsaw_block_editor_hud_state()
            .unwrap()
            .controls_enabled
    );
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.done,
    ));
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.generate,
    ));
    assert!(input.handle_jigsaw_block_editor_key(
        &mut counters,
        &mut world,
        &commands,
        KeyCode::Enter,
        true,
    ));
    assert!(input.jigsaw_block_editor_is_active());
    assert!(rx.try_recv().is_err());

    {
        let editor = input.jigsaw_block_editor.as_mut().unwrap();
        editor.focus = jigsaw_block::JigsawBlockEditorFocus::Pool;
        editor.fields[0].cursor = editor.fields[0].text.chars().count();
        editor.fields[0].selection = 0;
    }
    assert!(input.handle_jigsaw_block_editor_text(&world, &"a".repeat(200)));
    assert_eq!(
        input.jigsaw_block_editor_hud_state().unwrap().fields[0]
            .text
            .encode_utf16()
            .count(),
        128
    );
    {
        let editor = input.jigsaw_block_editor.as_mut().unwrap();
        editor.focus = jigsaw_block::JigsawBlockEditorFocus::FinalState;
        editor.fields[3].cursor = editor.fields[3].text.chars().count();
        editor.fields[3].selection = 0;
    }
    assert!(input.handle_jigsaw_block_editor_text(&world, &"😀".repeat(200)));
    assert_eq!(
        input.jigsaw_block_editor_hud_state().unwrap().fields[3]
            .text
            .encode_utf16()
            .count(),
        256
    );
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.cancel,
    ));
    assert!(rx.try_recv().is_err());

    assert!(mouse::start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(CrosshairTarget::Block(
            crate::crosshair::CrosshairBlockHit {
                pos,
                face: ProtocolDirection::Up,
                cursor: [0.25, 0.5, 0.75],
                inside: false,
            }
        )),
        None,
    ));
    assert!(input.jigsaw_block_editor_is_active());
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert!(input.handle_jigsaw_block_editor_key(
        &mut counters,
        &mut world,
        &commands,
        KeyCode::Escape,
        true,
    ));
    assert!(rx.try_recv().is_err());

    assert!(world.try_open_jigsaw_block_editor(pos));
    input.sync_jigsaw_block_editor(&mut world, &mut counters, &commands);
    assert!(world.open_local_inventory());
    input.sync_jigsaw_block_editor(&mut world, &mut counters, &commands);
    assert!(!input.jigsaw_block_editor_is_active());
    assert!(world.jigsaw_block_editor().is_none());
    assert_eq!(counters.set_jigsaw_block_commands_queued, 0);
    assert_eq!(counters.jigsaw_generate_commands_queued, 0);

    let mut horizontal_world = world_with_jigsaw_block(pos, "north_up");
    let mut horizontal_input = ClientInputState::new(true);
    assert!(horizontal_world.try_open_jigsaw_block_editor(pos));
    horizontal_input.sync_jigsaw_block_editor(&mut horizontal_world, &mut counters, &None);
    assert!(
        !horizontal_input
            .jigsaw_block_editor_hud_state()
            .unwrap()
            .joint_editable
    );
    assert!(click_jigsaw_block_editor(
        &mut horizontal_input,
        &mut counters,
        &mut horizontal_world,
        &None,
        layout.joint,
    ));
    assert_eq!(
        horizontal_input
            .jigsaw_block_editor_hud_state()
            .unwrap()
            .joint,
        JigsawJoint::Rollable
    );
}

#[test]
fn jigsaw_authoritative_refresh_preserves_unsent_native_fields_in_same_open() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_jigsaw_block(pos, "up_north");
    assert!(world.try_open_jigsaw_block_editor(pos));
    input.sync_jigsaw_block_editor(&mut world, &mut counters, &None);
    input.jigsaw_block_editor.as_mut().unwrap().fields[1].text = "example:unsent".to_string();

    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
            raw_nbt: vec![10, 0],
        })
        .unwrap());
    input.sync_jigsaw_block_editor(&mut world, &mut counters, &None);
    assert_eq!(
        input.jigsaw_block_editor_hud_state().unwrap().fields[1].text,
        "example:unsent"
    );
    assert_eq!(
        world.jigsaw_block_editor().unwrap().data.name,
        "minecraft:empty"
    );
}

#[test]
fn structure_block_editor_preloads_and_done_sends_clamped_exact_packet() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_structure_block(pos);
    assert!(world.try_open_structure_block_editor(pos));
    input.sync_structure_block_editor(&mut world, &mut counters, &commands);

    let hud = input.structure_block_editor_hud_state().unwrap();
    assert_eq!(hud.fields[0].text, "example:initial");
    assert_eq!(hud.fields[1].text, "1");
    assert_eq!(hud.fields[2].text, "-2");
    assert_eq!(hud.fields[3].text, "3");
    assert_eq!(hud.fields[4].text, "4");
    assert_eq!(hud.fields[5].text, "5");
    assert_eq!(hud.fields[6].text, "6");
    assert_eq!(hud.fields[7].text, "0.75");
    assert_eq!(hud.fields[8].text, "9");
    assert_eq!(hud.fields[9].text, "initial data");
    assert_eq!(hud.mode, StructureBlockMode::Load);
    assert_eq!(hud.mirror, StructureBlockMirror::FrontBack);
    assert_eq!(hud.rotation, StructureBlockRotation::Clockwise90);
    assert!(hud.include_entities);
    assert!(hud.strict);
    assert!(!hud.show_bounding_box);

    let editor = input.structure_block_editor.as_mut().unwrap();
    for (field, value) in editor.fields.iter_mut().zip([
        "example:saved",
        "99",
        "-99",
        "invalid",
        "99",
        "-1",
        "invalid",
        "2.5",
        "invalid",
        "payload",
    ]) {
        field.text = value.to_string();
    }
    editor.mode = StructureBlockMode::Load;
    editor.mirror = StructureBlockMirror::LeftRight;
    editor.rotation = StructureBlockRotation::Counterclockwise90;
    editor.include_entities = false;
    editor.strict = false;
    editor.show_air = true;
    editor.show_bounding_box = true;

    let done = bbb_renderer::hud_structure_block_editor_layout(PhysicalSize::new(320, 240)).done;
    assert!(click_structure_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        done,
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetStructureBlock(SetStructureBlock {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            update: StructureBlockUpdateKind::UpdateData,
            mode: StructureBlockMode::Load,
            name: "example:saved".to_string(),
            offset: ProtocolBlockPos {
                x: 48,
                y: -48,
                z: 0
            },
            size: Vec3i { x: 48, y: 0, z: 0 },
            mirror: StructureBlockMirror::LeftRight,
            rotation: StructureBlockRotation::Counterclockwise90,
            data: "payload".to_string(),
            ignore_entities: true,
            strict: false,
            show_air: true,
            show_bounding_box: true,
            integrity: 1.0,
            seed: 0,
        })
    );
    assert_eq!(counters.set_structure_block_commands_queued, 1);
    assert!(!input.structure_block_editor_is_active());
    assert!(world.structure_block_editor().is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn structure_block_editor_save_load_and_detect_buttons_map_to_exact_updates() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let layout = bbb_renderer::hud_structure_block_editor_layout(PhysicalSize::new(320, 240));
    for (mode, expected_update, rect) in [
        (
            StructureBlockMode::Save,
            StructureBlockUpdateKind::SaveArea,
            layout.action,
        ),
        (
            StructureBlockMode::Load,
            StructureBlockUpdateKind::LoadArea,
            layout.action,
        ),
        (
            StructureBlockMode::Save,
            StructureBlockUpdateKind::ScanArea,
            layout.detect,
        ),
    ] {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_structure_block(pos);
        assert!(world.try_open_structure_block_editor(pos));
        input.sync_structure_block_editor(&mut world, &mut counters, &commands);
        input.structure_block_editor.as_mut().unwrap().mode = mode;
        assert!(click_structure_block_editor(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            rect,
        ));
        let NetCommand::SetStructureBlock(packet) = rx.try_recv().unwrap() else {
            panic!("structure block action must queue its typed packet");
        };
        assert_eq!(packet.update, expected_update);
        assert_eq!(packet.mode, mode);
        assert_eq!(counters.set_structure_block_commands_queued, 1);
        assert!(!input.structure_block_editor_is_active());
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn structure_block_editor_mode_visibility_and_controls_are_exact() {
    use structure_block::StructureBlockEditorFocus as Focus;

    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_structure_block(pos);
    assert!(world.try_open_structure_block_editor(pos));
    input.sync_structure_block_editor(&mut world, &mut counters, &commands);

    for (mode, focuses) in [
        (
            StructureBlockMode::Save,
            vec![
                Focus::Name,
                Focus::OffsetX,
                Focus::OffsetY,
                Focus::OffsetZ,
                Focus::SizeX,
                Focus::SizeY,
                Focus::SizeZ,
            ],
        ),
        (
            StructureBlockMode::Load,
            vec![
                Focus::Name,
                Focus::OffsetX,
                Focus::OffsetY,
                Focus::OffsetZ,
                Focus::Integrity,
                Focus::Seed,
            ],
        ),
        (StructureBlockMode::Corner, vec![Focus::Name]),
        (StructureBlockMode::Data, vec![Focus::Data]),
    ] {
        input.structure_block_editor.as_mut().unwrap().mode = mode;
        input.structure_block_editor.as_mut().unwrap().focus = focuses[0];
        for expected in &focuses {
            assert_eq!(
                input.structure_block_editor.as_ref().unwrap().focus,
                *expected
            );
            input.handle_structure_block_editor_key(
                &mut counters,
                &mut world,
                &commands,
                KeyCode::Tab,
                true,
            );
        }
        assert_eq!(
            input.structure_block_editor.as_ref().unwrap().focus,
            focuses[0]
        );
    }

    let layout = bbb_renderer::hud_structure_block_editor_layout(PhysicalSize::new(320, 240));
    let editor = input.structure_block_editor.as_mut().unwrap();
    editor.mode = StructureBlockMode::Load;
    editor.focus = Focus::Name;
    for rect in [
        layout.mirror,
        layout.rotations[2],
        layout.include_entities,
        layout.strict,
        layout.option,
    ] {
        assert!(click_structure_block_editor(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            rect,
        ));
    }
    let hud = input.structure_block_editor_hud_state().unwrap();
    assert_eq!(hud.mirror, StructureBlockMirror::None);
    assert_eq!(hud.rotation, StructureBlockRotation::Clockwise180);
    assert!(!hud.include_entities);
    assert!(!hud.strict);
    assert!(hud.show_bounding_box);

    input.structure_block_editor.as_mut().unwrap().mode = StructureBlockMode::Save;
    assert!(click_structure_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.option,
    ));
    assert!(!input.structure_block_editor_hud_state().unwrap().show_air);
}

#[test]
fn structure_block_editor_filters_names_cycles_modes_and_cancels_without_packet() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_structure_block(pos);
    assert!(world.try_open_structure_block_editor(pos));
    input.sync_structure_block_editor(&mut world, &mut counters, &commands);
    {
        let editor = input.structure_block_editor.as_mut().unwrap();
        editor.focus = structure_block::StructureBlockEditorFocus::Name;
        editor.fields[0].cursor = editor.fields[0].text.chars().count();
        editor.fields[0].selection = 0;
    }
    assert!(input.handle_structure_block_editor_text(&world, "Bad Name"));
    assert!(input.handle_structure_block_editor_text(&world, ":path/sub-1"));
    assert_eq!(
        input.structure_block_editor_hud_state().unwrap().fields[0].text,
        "adame:path/sub-1"
    );

    let layout = bbb_renderer::hud_structure_block_editor_layout(PhysicalSize::new(320, 240));
    assert!(click_structure_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.mode,
    ));
    assert_eq!(
        input.structure_block_editor_hud_state().unwrap().mode,
        StructureBlockMode::Corner
    );
    input.structure_block_editor.as_mut().unwrap().mode = StructureBlockMode::Corner;
    input.pressed_keys.insert(KeyCode::AltLeft);
    assert!(click_structure_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.mode,
    ));
    let hud = input.structure_block_editor_hud_state().unwrap();
    assert_eq!(hud.mode, StructureBlockMode::Data);
    assert_eq!(hud.focus_index, 9);

    assert!(click_structure_block_editor(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        layout.cancel,
    ));
    assert!(!input.structure_block_editor_is_active());
    assert_eq!(counters.set_structure_block_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(mouse::start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(CrosshairTarget::Block(
            crate::crosshair::CrosshairBlockHit {
                pos,
                face: ProtocolDirection::Up,
                cursor: [0.25, 0.5, 0.75],
                inside: false,
            }
        )),
        None,
    ));
    assert!(input.structure_block_editor_is_active());
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.structure_block_editor_is_active());
    assert!(rx.try_recv().is_err());

    assert!(world.try_open_structure_block_editor(pos));
    input.sync_structure_block_editor(&mut world, &mut counters, &commands);
    assert!(world.open_local_inventory());
    input.sync_structure_block_editor(&mut world, &mut counters, &commands);
    assert!(!input.structure_block_editor_is_active());
    assert!(world.structure_block_editor().is_none());
    assert_eq!(counters.set_structure_block_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_block_editor_waits_for_authoritative_data_and_done_sends_exact_state_once() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_block(pos, "say initial");
    assert!(world.try_open_command_block_editor(pos));
    input.sync_command_block_editor(&mut world, &mut counters, &commands);
    assert!(input.command_block_editor_is_active());
    assert!(
        !input
            .command_block_editor_hud_state()
            .unwrap()
            .controls_enabled
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );
    assert!(rx.try_recv().is_err());
    assert!(input.command_block_editor_is_active());

    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
            raw_nbt: nbt_compound(vec![
                nbt_string("Command", "say authoritative"),
                nbt_byte("TrackOutput", 1),
                nbt_string("LastOutput", "previous"),
                nbt_byte("auto", 0),
            ]),
        })
        .unwrap());
    input.sync_command_block_editor(&mut world, &mut counters, &commands);
    let state = input.command_block_editor_hud_state().unwrap();
    assert!(state.controls_enabled);
    assert!(state.track_output);
    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("say authoritative")
    );
    assert!(input.replace_chat_entry_text("say saved".to_string()));

    let surface = PhysicalSize::new(320, 240);
    let layout = hud_command_block_editor_layout(surface);
    assert!(input.handle_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.command_input.x + 4 + 18),
            f64::from(layout.command_input.y + 6),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert_eq!(input.chat_entry.as_ref().unwrap().cursor, 3);
    for rect in [
        layout.mode,
        layout.conditional,
        layout.automatic,
        layout.track_output,
    ] {
        assert!(input.handle_command_block_editor_mouse_input(
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(
                f64::from(rect.x + 1),
                f64::from(rect.y + 1),
            )),
            surface,
            &FixedCommandSuggestionTextMeasurer,
        ));
    }
    assert!(input.handle_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.done.x + 1),
            f64::from(layout.done.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));

    assert_eq!(counters.set_command_block_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetCommandBlock(SetCommandBlock {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            command: "say saved".to_string(),
            mode: CommandBlockMode::Sequence,
            track_output: false,
            conditional: true,
            automatic: true,
        })
    );
    assert!(rx.try_recv().is_err());
    assert!(!input.command_block_editor_is_active());
    assert!(world.command_block_editor().is_none());
    let data = world.command_block_data_at(pos).unwrap();
    assert!(!data.track_output);
    assert_eq!(data.last_output, None);
}

#[test]
fn command_block_editor_cancel_and_escape_close_without_sending() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_block(pos, "say initial");

    assert!(world.try_open_command_block_editor(pos));
    input.sync_command_block_editor(&mut world, &mut counters, &commands);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.command_block_editor_is_active());
    assert!(world.command_block_editor().is_none());

    assert!(world.try_open_command_block_editor(pos));
    input.sync_command_block_editor(&mut world, &mut counters, &commands);
    let surface = PhysicalSize::new(320, 240);
    let cancel = hud_command_block_editor_layout(surface).cancel;
    assert!(input.handle_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(cancel.x + 1),
            f64::from(cancel.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert!(!input.command_block_editor_is_active());
    assert!(world.command_block_editor().is_none());
    assert_eq!(counters.set_command_block_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(world.try_open_command_block_editor(pos));
    input.sync_command_block_editor(&mut world, &mut counters, &commands);
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    input.sync_command_block_editor(&mut world, &mut counters, &commands);
    assert!(!input.command_entry_is_active());
    assert!(world.command_block_editor().is_some());
    world.apply_clear_dialog();
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    input.sync_command_block_editor(&mut world, &mut counters, &commands);
    assert!(input.command_block_editor_is_active());

    assert!(world.open_local_inventory());
    input.sync_command_block_editor(&mut world, &mut counters, &commands);
    assert!(!input.command_block_editor_is_active());
    assert!(world.command_block_editor().is_none());
}

#[test]
fn command_block_editor_uses_commands_only_seven_row_suggestions_and_utf16_limit() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_block(pos, "c");
    let literals = (0..12)
        .map(|index| format!("cmd{index:02}"))
        .collect::<Vec<_>>();
    world.apply_commands(local_literal_command_tree(
        &literals.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert!(world.try_open_command_block_editor(pos));
    input.sync_command_block_editor(&mut world, &mut counters, &commands);

    let hud = input
        .command_suggestion_hud_state(
            &world,
            &FixedCommandSuggestionTextMeasurer,
            PhysicalSize::new(320, 240),
        )
        .unwrap();
    assert!(hud.command_block_layout);
    assert_eq!(hud.input_text, "c");
    assert_eq!(hud.rect.y, 71);
    assert_eq!(hud.rect.height, 84);
    assert_eq!(hud.entries.len(), 7);
    assert!(!hud.has_previous);
    assert!(hud.has_next);

    assert!(input.replace_chat_entry_text("😀".repeat(16_250)));
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "a");
    assert_eq!(
        input
            .chat_entry
            .as_ref()
            .unwrap()
            .text
            .encode_utf16()
            .count(),
        MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS
    );
    assert!(!input.chat_entry.as_ref().unwrap().text.ends_with('a'));
    assert!(input.replace_chat_entry_text("😀".repeat(16_249)));
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "ab");
    assert_eq!(
        input
            .chat_entry
            .as_ref()
            .unwrap()
            .text
            .encode_utf16()
            .count(),
        MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS
    );
    assert!(input.chat_entry.as_ref().unwrap().text.ends_with("ab"));
}

#[test]
fn minecart_command_block_editor_preserves_unsent_command_and_done_sends_exact_state_once() {
    let entity_id = 300;
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_minecart(entity_id, "say initial", "first output");
    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.sync_minecart_command_block_editor(&mut world, &mut counters, &commands);
    assert!(input.minecart_command_block_editor_is_active());
    let hud = input.minecart_command_block_editor_hud_state().unwrap();
    assert!(hud.track_output);
    assert_eq!(hud.previous_output, "first output");

    assert!(input.replace_chat_entry_text("say unsent".to_string()));
    apply_command_minecart_metadata(&mut world, entity_id, "say server", "new output");
    input.sync_minecart_command_block_editor(&mut world, &mut counters, &commands);
    assert_eq!(input.chat_entry.as_ref().unwrap().text, "say unsent");
    assert_eq!(
        input
            .minecart_command_block_editor_hud_state()
            .unwrap()
            .previous_output,
        "new output"
    );

    let surface = PhysicalSize::new(320, 240);
    let layout = bbb_renderer::hud_minecart_command_block_editor_layout(surface);
    assert!(input.handle_minecart_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.track_output.x + 1),
            f64::from(layout.track_output.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert!(input.handle_minecart_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.done.x + 1),
            f64::from(layout.done.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));

    assert_eq!(counters.set_command_minecart_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetCommandMinecart(SetCommandMinecart {
            entity_id,
            command: "say unsent".to_string(),
            track_output: false,
        })
    );
    assert!(rx.try_recv().is_err());
    assert!(!input.minecart_command_block_editor_is_active());
    assert!(world.minecart_command_block_editor().is_none());
    let data = world.minecart_command_block_data(entity_id).unwrap();
    assert_eq!(data.command, "say server");
    assert!(!data.track_output);
    assert!(data.last_output.is_none());
}

#[test]
fn minecart_command_block_editor_cancel_escape_and_removal_close_without_sending() {
    let entity_id = 300;
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_minecart(entity_id, "say initial", "output");

    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.sync_minecart_command_block_editor(&mut world, &mut counters, &commands);
    let surface = PhysicalSize::new(320, 240);
    let layout = bbb_renderer::hud_minecart_command_block_editor_layout(surface);
    assert!(input.handle_minecart_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.track_output.x + 1),
            f64::from(layout.track_output.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.minecart_command_block_editor_is_active());
    assert!(world.minecart_command_block_editor().is_none());

    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.sync_minecart_command_block_editor(&mut world, &mut counters, &commands);
    assert!(
        !input
            .minecart_command_block_editor_hud_state()
            .unwrap()
            .track_output
    );
    let cancel = layout.cancel;
    assert!(input.handle_minecart_command_block_editor_mouse_input(
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(cancel.x + 1),
            f64::from(cancel.y + 1),
        )),
        surface,
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert!(!input.minecart_command_block_editor_is_active());
    assert!(world.minecart_command_block_editor().is_none());

    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.sync_minecart_command_block_editor(&mut world, &mut counters, &commands);
    assert_eq!(
        world.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![entity_id],
        }),
        1
    );
    input.sync_minecart_command_block_editor(&mut world, &mut counters, &commands);
    assert!(!input.minecart_command_block_editor_is_active());
    assert!(world.minecart_command_block_editor().is_none());

    let mut replacement_world = world_with_command_minecart(entity_id, "say initial", "output");
    let mut replacement_input = ClientInputState::new(true);
    assert!(replacement_world.try_open_minecart_command_block_editor(entity_id));
    replacement_input.sync_minecart_command_block_editor(
        &mut replacement_world,
        &mut counters,
        &commands,
    );
    assert!(replacement_world.open_local_inventory());
    replacement_input.sync_minecart_command_block_editor(
        &mut replacement_world,
        &mut counters,
        &commands,
    );
    assert!(!replacement_input.minecart_command_block_editor_is_active());
    assert!(replacement_world.minecart_command_block_editor().is_none());
    assert_eq!(counters.set_command_minecart_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn minecart_command_block_editor_enter_sends_once() {
    let entity_id = 300;
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_minecart(entity_id, "say initial", "output");
    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.sync_minecart_command_block_editor(&mut world, &mut counters, &commands);
    assert!(input.replace_chat_entry_text("say enter".to_string()));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert_eq!(counters.set_command_minecart_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetCommandMinecart(SetCommandMinecart {
            entity_id,
            command: "say enter".to_string(),
            track_output: true,
        })
    );
    assert!(rx.try_recv().is_err());
    assert!(!input.minecart_command_block_editor_is_active());
    assert!(world.minecart_command_block_editor().is_none());
    assert_eq!(
        world
            .minecart_command_block_data(entity_id)
            .unwrap()
            .command,
        "say initial"
    );
}

#[test]
fn minecart_command_block_editor_uses_commands_only_suggestions_and_utf16_limit() {
    let entity_id = 300;
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_command_minecart(entity_id, "c", "");
    let literals = (0..12)
        .map(|index| format!("cmd{index:02}"))
        .collect::<Vec<_>>();
    world.apply_commands(local_literal_command_tree(
        &literals.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert!(world.try_open_minecart_command_block_editor(entity_id));
    input.sync_minecart_command_block_editor(&mut world, &mut counters, &commands);

    let hud = input
        .command_suggestion_hud_state(
            &world,
            &FixedCommandSuggestionTextMeasurer,
            PhysicalSize::new(320, 240),
        )
        .unwrap();
    assert!(hud.command_block_layout);
    assert_eq!(hud.rect.y, 71);
    assert_eq!(hud.rect.height, 84);
    assert_eq!(hud.entries.len(), 7);
    assert!(hud.has_next);

    assert!(input.replace_chat_entry_text("😀".repeat(16_250)));
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "a");
    assert_eq!(
        input
            .chat_entry
            .as_ref()
            .unwrap()
            .text
            .encode_utf16()
            .count(),
        MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS
    );
    assert!(!input.chat_entry.as_ref().unwrap().text.ends_with('a'));
}

#[test]
fn command_suggestion_escape_hides_list_before_closing_command_entry() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/se");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 2,
        suggestions: vec![CommandSuggestion {
            text: "seed".to_string(),
            tooltip: None,
            tooltip_styled: None,
            tooltip_component: None,
        }],
    });
    input.command_suggestion_hud_state(
        &world,
        &FixedCommandSuggestionTextMeasurer,
        PhysicalSize::new(320, 240),
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(input.command_entry_is_active());
    assert!(input
        .command_suggestion_hud_state(
            &world,
            &FixedCommandSuggestionTextMeasurer,
            PhysicalSize::new(320, 240),
        )
        .unwrap()
        .entries
        .is_empty());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.chat_entry.as_ref().unwrap().text, "/se");
    assert_eq!(
        input.command_suggestion_narration(),
        Some(&CommandSuggestionNarrationState {
            position: 1,
            total: 1,
            text: "seed".to_string(),
            tooltip: None,
            tooltip_translation_key: None,
        })
    );
    assert_eq!(
        input
            .command_suggestion_hud_state(
                &world,
                &FixedCommandSuggestionTextMeasurer,
                PhysicalSize::new(320, 240),
            )
            .unwrap()
            .entries
            .len(),
        1
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(input.command_entry_is_active());
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.command_entry_is_active());
}

#[test]
fn chat_scroll_lines_clamp_and_return_to_latest() {
    let mut input = ClientInputState::new(true);
    input.scroll_chat_lines(7);
    assert_eq!(input.chat_scroll_lines(), 7);
    input.scroll_chat_lines(i32::MAX);
    assert_eq!(input.chat_scroll_lines(), 100);
    input.scroll_chat_lines(-19);
    assert_eq!(input.chat_scroll_lines(), 81);
    input.scroll_chat_lines(i32::MIN);
    assert_eq!(input.chat_scroll_lines(), 0);
}

#[test]
fn chat_entry_tab_applies_server_custom_completion_without_command_request() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_custom_chat_completions(CustomChatCompletions {
        action: CustomChatCompletionsAction::Set,
        entries: vec!["helper".to_string(), "hello".to_string()],
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "say hel");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    let entry = input.chat_entry.as_ref().unwrap();
    assert_eq!(entry.text, "say hello");
    assert_eq!(entry.cursor, 9);
    assert_eq!(entry.last_suggestion_request_text, None);
    assert_eq!(counters.command_suggestion_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn custom_chat_completion_candidates_merge_online_names_and_match_vanilla_boundaries() {
    let mut world = WorldStore::new();
    let profile_id = Uuid::from_u128(0x12345678123456781234567812345678);
    world.apply_player_info_update(PlayerInfoUpdate {
        actions: vec![PlayerInfoAction::AddPlayer],
        entries: vec![PlayerInfoEntry {
            profile_id,
            profile: Some(GameProfile {
                uuid: profile_id,
                name: "Alex".to_string(),
                properties: Vec::new(),
            }),
            listed: false,
            latency: 0,
            game_mode: Default::default(),
            display_name: None,
            show_hat: true,
            list_order: 0,
            chat_session: None,
        }],
    });
    world.apply_custom_chat_completions(CustomChatCompletions {
        action: CustomChatCompletionsAction::Set,
        entries: vec![
            "Alice".to_string(),
            "minecraft:hello_world".to_string(),
            "worldwide".to_string(),
        ],
    });

    assert_eq!(
        latest_custom_chat_completion_text(&world, "tell ali later", 8),
        Some(("tell Alice later".to_string(), 10))
    );
    assert_eq!(
        latest_custom_chat_completion_text(&world, "tell ALE", 8),
        Some(("tell Alex".to_string(), 9))
    );
    assert_eq!(
        latest_custom_chat_completion_text(&world, "say world", 9),
        Some(("say worldwide".to_string(), 13))
    );
    assert!(matches_suggestion_substring(
        "world",
        "minecraft:hello_world"
    ));
    assert_eq!(latest_custom_chat_completion_text(&world, "   ", 3), None);
}

#[test]
fn command_entry_tab_ignores_stale_suggestion_response() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/giv");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "e");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 3,
        suggestions: vec![CommandSuggestion {
            text: "gamemode".to_string(),
            tooltip: None,
            tooltip_styled: None,
            tooltip_component: None,
        }],
    });
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/give")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/giv".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 1,
            command: "/give".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_tab_ignores_invalid_suggestion_range() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/give");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 10,
        length: 1,
        suggestions: vec![CommandSuggestion {
            text: "gamemode".to_string(),
            tooltip: None,
            tooltip_styled: None,
            tooltip_component: None,
        }],
    });
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/give")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/give".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn text_input_requires_focus_and_leading_slash() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(false);
    let mut counters = NetCounters::default();

    handle_text_input_without_world(&mut input, &mut counters, &commands, "/seed");
    assert!(!input.chat_entry_is_active());

    input.focused = true;
    handle_text_input_without_world(&mut input, &mut counters, &commands, "seed");

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn drop_key_queues_drop_item_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn control_drop_key_queues_drop_all_items_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropAllItems,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_drop_key_still_queues_drop_item_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.sprint = true;
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn drop_key_predicts_one_selected_item_and_swings_when_non_empty() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    equip_player_slot(&mut world, 0, 42, 3);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(test_player_slot_item(&world, 0), test_item_stack(42, 2));
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn control_drop_key_predicts_selected_stack_and_swings_when_non_empty() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    equip_player_slot(&mut world, 0, 42, 3);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(
        test_player_slot_item(&world, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropAllItems,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn spectator_drop_key_does_not_drop_or_swing() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    equip_player_slot(&mut world, 0, 42, 3);
    set_local_spectator(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(test_player_slot_item(&world, 0), test_item_stack(42, 3));
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_hovered_drop_key_queues_throw_one_container_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(36);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: ProtocolItemStackSummary {
            item_id: Some(42),
            count: 3,
            component_patch: Default::default(),
        },
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Throw,
            changed_slots: [(
                36,
                HashedStack::Item(HashedItemStack {
                    item_id: 42,
                    count: 2,
                    components: HashedComponentPatch::default(),
                }),
            )]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
}

#[test]
fn local_inventory_hovered_control_drop_key_queues_throw_stack_container_click() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(36);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: ProtocolItemStackSummary {
            item_id: Some(42),
            count: 3,
            component_patch: Default::default(),
        },
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    assert!(input.control_down());
    assert!(!input.sprint);
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: 36,
            button_num: 1,
            input: ContainerInput::Throw,
            changed_slots: [(36, HashedStack::Empty)].into(),
            carried_item: HashedStack::Empty,
        })
    );
}

#[test]
fn local_inventory_hovered_empty_slot_drop_key_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(36);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_drop_key_without_hovered_slot_does_not_queue_player_drop() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.sprint = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_hovered_number_keys_queue_swap_container_clicks() {
    const HOTBAR_KEYS: [(KeyCode, i8); 9] = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (code, button_num) in HOTBAR_KEYS {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.inventory_hovered_slot = Some(9);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: i32::from(button_num),
            item: test_item_stack(42 + i32::from(button_num), 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: test_item_stack(99, 2),
        });
        assert!(world.open_local_inventory());

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(world.local_player().selected_hotbar_slot, 0);
        match rx.try_recv().unwrap() {
            NetCommand::ContainerClick(click) => {
                assert_eq!(click.container_id, 0);
                assert_eq!(click.state_id, 0);
                assert_eq!(click.slot_num, 9);
                assert_eq!(click.button_num, button_num);
                assert_eq!(click.input, ContainerInput::Swap);
                assert_eq!(
                    click.changed_slots,
                    [
                        (9, test_hashed_item_stack(42 + i32::from(button_num), 1)),
                        (36 + i16::from(button_num), test_hashed_item_stack(99, 2)),
                    ]
                    .into()
                );
                assert_eq!(click.carried_item, HashedStack::Empty);
            }
            command => panic!("expected container click command, got {command:?}"),
        }
        assert_eq!(
            test_player_slot_item(&world, 9),
            test_item_stack(42 + i32::from(button_num), 1)
        );
        assert_eq!(
            test_player_slot_item(&world, i32::from(button_num)),
            test_item_stack(99, 2)
        );
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn local_inventory_hovered_offhand_swap_key_queues_swap_container_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(9);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: test_item_stack(42, 1),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: test_item_stack(77, 1),
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.container_id, 0);
            assert_eq!(click.state_id, 0);
            assert_eq!(click.slot_num, 9);
            assert_eq!(click.button_num, 40);
            assert_eq!(click.input, ContainerInput::Swap);
            assert_eq!(
                click.changed_slots,
                [
                    (9, test_hashed_item_stack(77, 1)),
                    (45, test_hashed_item_stack(42, 1)),
                ]
                .into()
            );
            assert_eq!(click.carried_item, HashedStack::Empty);
        }
        command => panic!("expected container click command, got {command:?}"),
    }
    assert_eq!(test_player_slot_item(&world, 9), test_item_stack(77, 1));
    assert_eq!(test_player_slot_item(&world, 40), test_item_stack(42, 1));
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_swap_keys_without_hover_or_with_carried_item_are_no_ops() {
    for code in [KeyCode::Digit5, KeyCode::KeyF] {
        assert_local_inventory_swap_key_noop(code, None, ProtocolItemStackSummary::empty());
        assert_local_inventory_swap_key_noop(
            code,
            Some(36),
            ProtocolItemStackSummary {
                item_id: Some(42),
                count: 1,
                component_patch: Default::default(),
            },
        );
    }
}

fn assert_local_inventory_swap_key_noop(
    code: KeyCode,
    hovered_slot: Option<i16>,
    cursor_item: ProtocolItemStackSummary,
) {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = hovered_slot;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(ProtocolSetCursorItem { item: cursor_item });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: test_item_stack(42, 1),
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(code),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_opened_container_hovered_drop_key_queues_throw_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Throw,
            changed_slots: [].into(),
            carried_item: HashedStack::Empty,
        })
    );
    assert_eq!(
        world
            .inventory()
            .open_container
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .find(|slot| slot.slot == 0)
            .unwrap()
            .item,
        test_item_stack(42, 3)
    );
}

#[test]
fn server_opened_container_hovered_control_drop_key_queues_throw_stack_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    input.inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 1,
            input: ContainerInput::Throw,
            changed_slots: [].into(),
            carried_item: HashedStack::Empty,
        })
    );
}

#[test]
fn server_opened_container_empty_slot_drop_key_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, None);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_opened_container_hovered_number_and_offhand_keys_queue_swap_clicks() {
    for (code, button_num) in [(KeyCode::Digit5, 4), (KeyCode::KeyF, 40)] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.inventory_hovered_slot = Some(0);
        let mut counters = NetCounters::default();
        let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));
        assert_eq!(world.local_player().selected_hotbar_slot, 0);

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(world.local_player().selected_hotbar_slot, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num,
                input: ContainerInput::Swap,
                changed_slots: [].into(),
                carried_item: HashedStack::Empty,
            })
        );
    }
}

#[test]
fn server_opened_bundle_slot_swap_key_clears_selection_before_click() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_bundle_stack(42, 1, 3))));
    assert!(world.apply_local_select_bundle_item(0, 1));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    assert_eq!(counters.select_bundle_item_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 0,
            selected_item_index: -1,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 4,
            input: ContainerInput::Swap,
            changed_slots: [].into(),
            carried_item: HashedStack::Empty,
        })
    );
    assert_eq!(open_container_slot_bundle_selection(&world, 0), Some(-1));
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_opened_container_swap_key_with_carried_item_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));
    world.apply_set_cursor_item(ProtocolSetCursorItem {
        item: test_item_stack(99, 1),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_queues_rename_item_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "Sharp Pick",
    );

    assert_eq!(input.anvil_rename_text(), "Sharp Pick");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Sharp Pick".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_starts_from_default_hover_name_when_item_runtime_is_available() {
    let root = unique_input_temp_dir("anvil-rename-default");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(0, 1)));

    handle_text_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        "!",
        &FixedCommandSuggestionTextMeasurer,
    );

    assert_eq!(input.anvil_rename_text(), "Test Combo!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Test Combo!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn anvil_text_input_starts_from_decoded_custom_name_without_item_runtime() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(42, 1);
    stack.component_patch.custom_name = Some("Custom Pick".to_string());
    stack.component_patch.item_name = Some("Ignored Item Name".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "!");

    assert_eq!(input.anvil_rename_text(), "Custom Pick!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Custom Pick!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_starts_from_written_book_title_without_item_runtime() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(42, 1);
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Book Title".to_string(),
        title_filter: None,
        author: "Author".to_string(),
        generation: 0,
        pages: Vec::new(),
        page_filters: Vec::new(),
        resolved: true,
    });
    stack.component_patch.item_name = Some("Ignored Item Name".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "!");

    assert_eq!(input.anvil_rename_text(), "Book Title!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Book Title!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_starts_from_decoded_item_name_without_item_runtime() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(42, 1);
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "   ".to_string(),
        title_filter: None,
        author: "Author".to_string(),
        generation: 0,
        pages: Vec::new(),
        page_filters: Vec::new(),
        resolved: true,
    });
    stack.component_patch.item_name = Some("Component Item".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "!");

    assert_eq!(input.anvil_rename_text(), "Component Item!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Component Item!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_default_hover_name_is_sent_as_empty_rename() {
    let root = unique_input_temp_dir("anvil-rename-default-empty");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(0, 1)));

    handle_text_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        "!",
        &FixedCommandSuggestionTextMeasurer,
    );
    handle_key_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
        PhysicalSize::new(1280, 720),
        &FixedCommandSuggestionTextMeasurer,
        None,
    );

    assert_eq!(input.anvil_rename_text(), "Test Combo");
    assert_eq!(counters.rename_item_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Test Combo!".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: String::new(),
        })
    );
    assert!(rx.try_recv().is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn anvil_custom_name_matching_hover_name_is_sent_verbatim() {
    let root = unique_input_temp_dir("anvil-rename-custom-name");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(0, 1);
    stack.component_patch.custom_name = Some("Custom Combo".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        "!",
        &FixedCommandSuggestionTextMeasurer,
    );
    handle_key_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
        PhysicalSize::new(1280, 720),
        &FixedCommandSuggestionTextMeasurer,
        None,
    );

    assert_eq!(input.anvil_rename_text(), "Custom Combo");
    assert_eq!(counters.rename_item_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Custom Combo!".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Custom Combo".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn anvil_backspace_queues_updated_rename_item_command() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Axe");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );

    assert_eq!(input.anvil_rename_text(), "Ax");
    assert_eq!(counters.rename_item_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Axe".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Ax".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_cursor_keys_edit_rename_text_inside_line() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "abcd");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "X");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Home),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, ">");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::End),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "<");

    assert_eq!(input.anvil_rename_text(), ">abd<");
    assert_eq!(counters.rename_item_commands_queued, 6);
    for name in ["abcd", "abXcd", "abXd", "abd", ">abd", ">abd<"] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: name.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_control_word_keys_edit_rename_text() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "alpha beta gamma",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowRight),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );

    assert_eq!(input.anvil_rename_text(), "alpha ");
    assert_eq!(counters.rename_item_commands_queued, 3);
    for name in ["alpha beta gamma", "alpha gamma", "alpha "] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: name.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_control_a_selection_replaces_and_deletes_rename_text() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "replace me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "Diamond Pick",
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "delete me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );

    assert_eq!(input.anvil_rename_text(), "");
    assert_eq!(counters.rename_item_commands_queued, 5);
    for name in ["replace me", "Diamond Pick", "", "delete me", ""] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: name.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_delete_at_end_is_consumed_without_rename_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Axe");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );

    assert_eq!(input.anvil_rename_text(), "Axe");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Axe".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_rename_field_consumes_inventory_key_when_editable() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "e");

    assert!(world.inventory().open_container.is_some());
    assert_eq!(counters.container_close_commands_queued, 0);
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "e".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_filters_vanilla_invalid_chars_and_max_length() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));
    let text = format!("{}{}\u{a7}\n", "a".repeat(49), "🙂");

    handle_text_input(&mut input, &mut counters, &mut world, &commands, &text);

    let expected = "a".repeat(49);
    assert_eq!(input.anvil_rename_text(), expected);
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem { name: expected })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn pending_sign_editor_escape_queues_empty_sign_update() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos {
        x: -5,
        y: 70,
        z: 12,
    };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        is_front_text: false,
    });

    assert!(input.sign_editor_is_active_or_pending(&world));
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.sign_editor_is_active_or_pending(&world));
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
            lines: Default::default(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn book_screen_keys_turn_pages_close_and_block_gameplay_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_test_book_screen(&mut world, vec!["First", "Second"]);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::PageDown),
        ElementState::Pressed,
    );
    assert_eq!(world.current_book().unwrap().current_page(), 1);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    assert_eq!(counters.player_input_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert_eq!(world.current_book(), None);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    assert_eq!(counters.player_input_commands_queued, 1);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
}

#[test]
fn book_screen_mouse_clicks_turn_pages_and_close() {
    let mut world = WorldStore::new();
    open_test_book_screen(&mut world, vec!["First", "Second"]);
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - f64::from(BOOK_SCREEN_WIDTH)) * 0.5;
    let origin_y = 2.0;

    assert!(handle_book_screen_mouse_input(
        &mut world,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(BOOK_PAGE_FORWARD_BUTTON_X + 1),
            origin_y + f64::from(BOOK_PAGE_BUTTON_Y + 1),
        )),
        surface_size,
    ));
    assert_eq!(world.current_book().unwrap().current_page(), 1);

    assert!(handle_book_screen_mouse_input(
        &mut world,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(BOOK_MENU_DONE_BUTTON_X + 10),
            origin_y + f64::from(BOOK_MENU_BUTTON_Y + 1),
        )),
        surface_size,
    ));
    assert_eq!(world.current_book(), None);
}

#[test]
fn dialog_stack_submits_suspends_and_restores_the_native_sign_editor() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos {
        x: -5,
        y: 70,
        z: 12,
    };
    let mut world = WorldStore::new();
    insert_sign_text(&mut world, sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: sign_pos.x,
            y: sign_pos.y,
            z: sign_pos.z,
        },
        is_front_text: true,
    });
    assert!(sync_sign_editor_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
    ));
    assert!(handle_sign_editor_text(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "draft",
        &FixedCommandSuggestionTextMeasurer,
    ));
    let editor = input.sign_editor.as_mut().unwrap();
    editor.cursor = 2;
    editor.selection = 5;
    let expected = editor.clone();

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);

    assert!(!input.sign_editor_is_active_or_pending(&world));
    assert_eq!(input.sign_editor_hud_state(&world), None);
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: expected.signature.pos,
            is_front_text: true,
            lines: expected.lines.clone(),
        })
    );

    world.apply_clear_dialog();
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert_eq!(input.sign_editor.as_ref(), Some(&expected));
    assert_eq!(
        input.sign_editor_hud_state(&world),
        Some(sign_editor_hud_state_from_editor(&expected, None))
    );

    submit_sign_editor(&mut input, &mut counters, &commands);
    assert_eq!(counters.sign_update_commands_queued, 2);
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate { lines, .. }) if lines == expected.lines
    ));
    assert!(rx.try_recv().is_err());
}

#[test]
fn new_sign_editor_replaces_the_one_suspended_below_dialog() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let old_pos = BlockPos { x: 1, y: 2, z: 3 };
    let new_pos = BlockPos { x: 20, y: 5, z: 6 };
    let mut world = WorldStore::new();
    insert_sign_text(&mut world, old_pos, ["Old", "", "", ""], ["", "", "", ""]);
    insert_sign_text(&mut world, new_pos, ["New", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: old_pos.x,
            y: old_pos.y,
            z: old_pos.z,
        },
        is_front_text: true,
    });
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);
    assert_eq!(counters.sign_update_commands_queued, 1);

    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: new_pos.x,
            y: new_pos.y,
            z: new_pos.z,
        },
        is_front_text: true,
    });
    input.sync_dialog_native_previous_screen(&world, &mut counters, &commands);

    assert!(!world.dialog_screen_is_open());
    assert!(input.sign_editor_is_active_or_pending(&world));
    assert_eq!(
        input.sign_editor_hud_state(&world).map(|state| state.pos),
        Some(new_pos)
    );
    assert!(sync_sign_editor_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
    ));
    assert_eq!(
        input.sign_editor.as_ref().unwrap().signature.pos.x,
        new_pos.x
    );
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::SignUpdate(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn pending_sign_editor_escape_preserves_existing_sign_text() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 3, y: 64, z: -9 };
    let mut world = world_with_sign_text(
        sign_pos,
        ["Front 1", "Front 2", "Front 3", "Front 4"],
        ["Back 1", "Back 2", "Back 3", "Back 4"],
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: sign_pos.x,
            y: sign_pos.y,
            z: sign_pos.z,
        },
        is_front_text: false,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.sign_editor_is_active_or_pending(&world));
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos {
                x: sign_pos.x,
                y: sign_pos.y,
                z: sign_pos.z,
            },
            is_front_text: false,
            lines: [
                "Back 1".to_string(),
                "Back 2".to_string(),
                "Back 3".to_string(),
                "Back 4".to_string(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn invalid_sign_editor_packet_does_not_reopen_previous_editor() {
    let sign_pos = BlockPos { x: 3, y: 64, z: -9 };
    let mut world = world_with_sign_text(sign_pos, ["Front", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 3, y: 64, z: -9 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    assert!(sync_sign_editor_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
    ));
    let signature = input.sign_editor.as_ref().unwrap().signature.clone();

    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: 99,
            y: 64,
            z: 99,
        },
        is_front_text: false,
    });

    assert!(!sync_sign_editor_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
    ));
    assert_eq!(input.sign_editor.as_ref().unwrap().signature, signature);
    assert_eq!(world.counters().open_sign_editor_packets, 2);
    assert_eq!(world.counters().open_sign_editor_updates_applied, 1);
    assert_eq!(world.counters().open_sign_editor_updates_ignored, 1);
}

#[test]
fn sign_editor_text_input_changes_lines_and_queues_sign_update() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 3, y: 64, z: -9 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 3, y: 64, z: -9 },
        is_front_text: true,
    });

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Front");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Back");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowUp),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.forward);
    assert!(!input.sign_editor_is_active_or_pending(&world));
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: 3, y: 64, z: -9 },
            is_front_text: true,
            lines: [
                "Fron".to_string(),
                "Back".to_string(),
                String::new(),
                String::new(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_cursor_keys_edit_inside_current_line() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 8, y: 65, z: -3 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 8, y: 65, z: -3 },
        is_front_text: true,
    });

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "abcd");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "X");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Home),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, ">");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::End),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "<");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: 8, y: 65, z: -3 },
            is_front_text: true,
            lines: [
                ">abd<".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_control_word_keys_edit_current_line() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: -4, y: 70, z: 9 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
        is_front_text: false,
    });

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "aa bb cc");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowRight),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
            is_front_text: false,
            lines: [
                "aa ".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_control_a_selection_replaces_and_deletes_current_line() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 2, y: 68, z: -7 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 2, y: 68, z: -7 },
        is_front_text: true,
    });

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "replace me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "kept");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "remove me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "delete me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: 2, y: 68, z: -7 },
            is_front_text: true,
            lines: [
                "kept".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_text_input_filters_invalid_chars_and_measured_width() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 0, y: 1, z: 2 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 2 },
        is_front_text: true,
    });
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &"a".repeat(15),
    );
    // The 16th six-pixel glyph exceeds a standing sign's 90-pixel width and
    // the whole candidate is rejected. Invalid chat characters insert nothing.
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "b");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "\u{a7}\n");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    let expected = "a".repeat(15);
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 2 },
            is_front_text: true,
            lines: [expected, String::new(), String::new(), String::new(),],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_hanging_block_uses_60_pixel_candidate_limit() {
    let sign_pos = BlockPos { x: 0, y: 1, z: 2 };
    let mut world = WorldStore::new();
    insert_empty_chunk_for_block(&mut world, sign_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 2 },
        block_state_id: vanilla_block_state_id(
            "minecraft:bamboo_hanging_sign",
            [
                ("attached", "true"),
                ("rotation", "0"),
                ("waterlogged", "false"),
            ],
        ),
    }));
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 2 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    assert!(sync_sign_editor_input(
        &mut input,
        &mut NetCounters::default(),
        &mut world,
        &None,
    ));
    assert_eq!(
        input.sign_editor.as_ref().unwrap().max_line_width,
        HANGING_SIGN_MAX_LINE_WIDTH
    );

    let editor = input.sign_editor.as_mut().unwrap();
    insert_sign_line_text(editor, &"a".repeat(10), &FixedCommandSuggestionTextMeasurer);
    insert_sign_line_text(editor, "b", &FixedCommandSuggestionTextMeasurer);
    assert_eq!(editor.lines[0], "a".repeat(10));
}

#[test]
fn sign_editor_preload_uses_filtered_open_snapshot() {
    let sign_pos = BlockPos { x: 1, y: 64, z: 1 };
    let mut world = WorldStore::new();
    insert_sign_text_state(
        &mut world,
        sign_pos,
        sign_text_side_with_filtered(["raw", "", "", ""], ["filtered", "", "", ""]),
        sign_text_side(["", "", "", ""]),
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 1, y: 64, z: 1 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    input.set_client_text_filtering_enabled(true);
    assert!(input.render_publication().client_text_filtering_enabled());

    assert_eq!(
        input.sign_editor_hud_state(&world).unwrap().lines[0],
        "filtered"
    );
    assert!(sync_sign_editor_input(
        &mut input,
        &mut NetCounters::default(),
        &mut world,
        &None,
    ));
    assert_eq!(input.sign_editor.as_ref().unwrap().lines[0], "filtered");
}

#[test]
fn sign_editor_shift_selection_replaces_the_selected_range() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 2, y: 64, z: 2 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 2, y: 64, z: 2 },
        is_front_text: true,
    });
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "abcd");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    for _ in 0..2 {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::ArrowLeft),
            ElementState::Pressed,
        );
    }
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "X");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate { lines, .. }) if lines[0] == "abX"
    ));
}

#[test]
fn sign_editor_clipboard_copy_cut_and_sanitized_paste_match_text_field_helper() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 3, y: 64, z: 3 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 3, y: 64, z: 3 },
        is_front_text: true,
    });
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "copy");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    let mut clipboard = MockDebugClipboard::accepting();
    for code in [KeyCode::KeyC, KeyCode::KeyX] {
        handle_key_input_with_item_runtime(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            None,
            PhysicalKey::Code(code),
            ElementState::Pressed,
            PhysicalSize::new(320, 240),
            &FixedCommandSuggestionTextMeasurer,
            Some(&mut clipboard),
        );
        assert_eq!(clipboard.text.as_deref(), Some("copy"));
    }
    assert_eq!(input.sign_editor.as_ref().unwrap().lines[0], "");
    clipboard.text = Some("\u{a7}aP\raste\n".to_string());
    handle_key_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        None,
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
        Some(&mut clipboard),
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate { lines, .. }) if lines[0] == "Paste"
    ));
}

#[test]
fn sign_editor_tick_range_guard_submits_once() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 4, y: 64, z: 4 };
    let mut world = world_with_local_player_id(42);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 4.5,
            y: 63.0,
            z: 4.5,
        },
        ..Default::default()
    });
    insert_sign_text(
        &mut world,
        sign_pos,
        ["draft", "", "", ""],
        ["", "", "", ""],
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 4, y: 64, z: 4 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    assert!(input.sync_sign_editor(&mut world, &mut counters, &commands));
    assert!(input.sign_editor.is_some());

    let mut pose = world.local_player_pose().unwrap();
    pose.position.x = 100.0;
    world.set_local_player_pose(pose);
    assert!(input.sync_sign_editor(&mut world, &mut counters, &commands));
    assert!(!input.sync_sign_editor(&mut world, &mut counters, &commands));
    assert!(input.sign_editor.is_none());
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::SignUpdate(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_tick_removed_sign_submits_once() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 4, y: 64, z: 4 };
    let mut world = world_with_local_player_id(42);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 4.5,
            y: 63.0,
            z: 4.5,
        },
        ..Default::default()
    });
    insert_sign_text(
        &mut world,
        sign_pos,
        ["draft", "", "", ""],
        ["", "", "", ""],
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 4, y: 64, z: 4 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    assert!(input.sync_sign_editor(&mut world, &mut counters, &commands));

    assert!(world.forget_chunk(ChunkPos { x: 0, z: 0 }));
    assert!(input.sync_sign_editor(&mut world, &mut counters, &commands));
    assert!(!input.sync_sign_editor(&mut world, &mut counters, &commands));
    assert!(input.sign_editor.is_none());
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::SignUpdate(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_done_mouse_and_ime_preedit_share_one_submit_path() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 5, y: 64, z: 5 };
    let mut world = world_with_sign_text(sign_pos, ["", "", "", ""], ["", "", "", ""]);
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 5, y: 64, z: 5 },
        is_front_text: true,
    });
    let mut input = ClientInputState::new(true);
    assert!(sync_sign_editor_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
    ));
    assert!(input.handle_sign_editor_ime_preedit("候補", Some((0, 3))));
    let state = input.sign_editor_hud_state(&world).unwrap();
    assert_eq!(state.lines[0], "");
    assert_eq!(state.preedit.as_ref().unwrap().text, "候補");
    assert_eq!(state.preedit.as_ref().unwrap().focused_end, 1);

    input.clear_sign_editor_ime_preedit();
    assert!(handle_sign_editor_text(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "候",
        &FixedCommandSuggestionTextMeasurer,
    ));
    let surface = PhysicalSize::new(320, 240);
    let done = bbb_renderer::hud_sign_editor_done_button_rect(surface);
    assert!(input.handle_sign_editor_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(done.x + 1),
            f64::from(done.y + 1),
        )),
        surface,
    ));
    assert!(matches!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate { lines, .. }) if lines[0] == "候"
    ));
    assert!(rx.try_recv().is_err());
}

#[test]
fn swap_offhand_key_queues_swap_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::SwapItemWithOffhand,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn spectator_swap_offhand_key_does_not_queue_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_spectator(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn death_respawn_key_queues_perform_respawn() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_player_health(PlayerHealth {
        health: 0.0,
        food: 0,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert_eq!(counters.perform_respawn_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::PerformRespawn);
}

#[test]
fn death_state_consumes_gameplay_keys_without_queueing_actions() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_player_health(PlayerHealth {
        health: 0.0,
        food: 0,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.perform_respawn_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn sleeping_escape_key_queues_stop_sleeping_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_sleeping_local_player(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StopSleeping,
            data: 0,
        })
    );
}

#[test]
fn sleeping_state_consumes_gameplay_keys_without_queueing_actions() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_sleeping_local_player(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_key_opens_local_inventory_without_player_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), Some(0));
    assert_eq!(counters.player_command_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_key_on_server_controlled_mount_queues_open_inventory_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(!world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), None);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::OpenInventory,
            data: 0,
        })
    );
}

#[test]
fn escape_key_closes_local_inventory_and_queues_container_zero() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), None);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 0 })
    );
}

#[test]
fn inventory_key_closes_local_inventory_and_queues_container_zero() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(!world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), None);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 0 })
    );
}

#[test]
fn escape_key_closes_open_container_and_queues_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: "Chest".to_string(),
        title_styled: Vec::new(),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(world.inventory().open_container.is_none());
    assert_eq!(world.counters().container_close_updates_received, 0);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
    );
}

#[test]
fn escape_key_closes_narrow_recipe_book_before_container() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();

    handle_key_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
        PhysicalSize::new(378, 720),
        &FixedCommandSuggestionTextMeasurer,
        None,
    );

    assert!(world.inventory().open_container.is_some());
    assert!(!world.recipe_book().settings.crafting.open);
    assert_eq!(counters.container_close_commands_queued, 0);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(
            bbb_protocol::packets::RecipeBookChangeSettingsCommand {
                book_type: bbb_protocol::packets::RecipeBookType::Crafting,
                open: false,
                filtering: false,
            }
        )
    );
}

#[test]
fn inventory_key_closes_open_container_before_open_inventory_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 8,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: "Chest".to_string(),
        title_styled: Vec::new(),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(world.inventory().open_container.is_none());
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 8 })
    );
}

#[test]
fn inventory_key_is_consumed_by_focused_recipe_book_search() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 8,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert_eq!(world.open_container_id(), Some(8));
    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_page_arrow_click_updates_local_page_state() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    input.recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: (0..21)
            .map(|index| recipe_book_shapeless_entry(index, 2, 200 + index))
            .collect(),
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_PAGE_FORWARD_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_PAGE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(input.recipe_book_page_hud_state().crafting, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_PAGE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(input.recipe_book_page_hud_state().crafting, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_recipe_button_click_queues_place_recipe_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    input.recipe_book_search_focused = true;
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_shapeless_entry(42, 2, 200)],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 42,
            use_max_items: true,
        })
    );
}

#[test]
fn narrow_recipe_book_recipe_button_click_closes_book_and_queues_settings() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    input.recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_shapeless_entry(42, 2, 200)],
    });
    let surface_size = PhysicalSize::new(378, 720);
    let origin_x = (378.0 - 176.0) / 2.0;
    let origin_y = (720.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + 14.0 + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert!(!world.recipe_book().settings.crafting.open);
    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 42,
            use_max_items: false,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(
            bbb_protocol::packets::RecipeBookChangeSettingsCommand {
                book_type: bbb_protocol::packets::RecipeBookType::Crafting,
                open: false,
                filtering: false,
            }
        )
    );
}

#[test]
fn recipe_book_uncraftable_recipe_button_retry_is_guarded() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_shapeless_entry(42, 2, 200)],
    });
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: None,
            furnace: None,
        },
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;
    let recipe_button = Some(PhysicalPosition::new(
        origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
        origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
    ));

    assert!(world.last_ghost_recipe().is_some());
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        recipe_button,
        surface_size,
    ));
    assert!(world.last_ghost_recipe().is_none());
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        recipe_button,
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 42,
            use_max_items: false,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_craftable_recipe_button_retry_still_queues() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: test_item_stack(50, 1),
    });
    let mut entry = recipe_book_shapeless_entry(42, 2, 200);
    entry.contents.crafting_requirements = Some(vec![bbb_protocol::packets::IngredientSummary {
        tag: None,
        item_ids: vec![50],
    }]);
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;
    let recipe_button = Some(PhysicalPosition::new(
        origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
        origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
    ));

    for _ in 0..2 {
        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            recipe_button,
            surface_size,
        ));
    }

    assert_eq!(counters.place_recipe_commands_queued, 2);
    for _ in 0..2 {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
                container_id: 7,
                recipe_index: 42,
                use_max_items: false,
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_recipe_button_click_uses_search_filtered_collection() {
    let root = unique_input_temp_dir("recipe-book-search");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    input.recipe_book_search_text = "combo".to_string();
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry(42, 2, 200),
            recipe_book_shapeless_entry(43, 2, 0),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input_with_item_runtime(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(&item_runtime),
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn recipe_book_recipe_button_click_uses_current_multi_recipe_cycle() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_world_time(bbb_protocol::packets::PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry_with_group(42, 2, Some(7), 200),
            recipe_book_shapeless_entry_with_group(43, 2, Some(7), 201),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );
}

#[test]
fn recipe_book_right_click_overlay_queues_clicked_recipe() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry_with_group(42, 2, Some(7), 200),
            recipe_book_shapeless_entry_with_group(43, 2, Some(7), 201),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(
        input.recipe_book_overlay_hud_state(),
        Some(RecipeBookOverlayHudState {
            book_type: bbb_protocol::packets::RecipeBookType::Crafting,
            tab_index: 1,
            page_index: 0,
            button_index: 0,
            x: 11,
            y: 31,
        })
    );
    assert_eq!(counters.place_recipe_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(origin_x + 41.0, origin_y + 37.0)),
        surface_size,
    ));

    assert_eq!(input.recipe_book_overlay_hud_state(), None);
    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );
}

#[test]
fn recipe_book_overlay_uncraftable_recipe_retry_is_guarded() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry_with_group(42, 2, Some(7), 200),
            recipe_book_shapeless_entry_with_group(43, 2, Some(7), 201),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;
    let recipe_button = Some(PhysicalPosition::new(
        origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
        origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
    ));
    let overlay_second_recipe = Some(PhysicalPosition::new(origin_x + 41.0, origin_y + 37.0));

    for _ in 0..2 {
        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            recipe_button,
            surface_size,
        ));
        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            overlay_second_recipe,
            surface_size,
        ));
    }

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn furnace_recipe_book_recipe_button_click_queues_place_recipe_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_furnace_tab_index = 1;
    input.recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_furnace_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_furnace_entry(84, 4, 200)],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 84,
            use_max_items: false,
        })
    );
}

#[test]
fn gameplay_keys_are_consumed_while_unsupported_container_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(0));
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 9,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });

    for code in [KeyCode::Digit5, KeyCode::KeyQ] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }

    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(world.inventory().open_container.is_some());
    assert!(rx.try_recv().is_err());
}

#[test]
fn escape_key_without_open_container_does_not_queue_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_opens_local_screen_without_seen_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_advancement("minecraft:hidden/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(world.advancements_screen_is_open());
    assert_eq!(world.selected_advancements_tab(), None);
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_selects_first_root_tab_and_queues_opened_tab() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![
            input_advancement("minecraft:z/root", None),
            input_displayed_advancement("minecraft:y/root", None),
            input_displayed_advancement("minecraft:a/root", None),
        ],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(world.advancements_screen_is_open());
    assert_eq!(world.selected_advancements_tab(), Some("minecraft:y/root"));
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:y/root".to_string()
        })
    );
    assert!(rx.try_recv().is_err());
}

fn input_advancement(id: &str, parent: Option<&str>) -> AdvancementSummary {
    AdvancementSummary {
        id: id.to_string(),
        parent: parent.map(str::to_string),
        display: None,
        requirements: Vec::new(),
        sends_telemetry_event: false,
    }
}

fn input_displayed_advancement(id: &str, parent: Option<&str>) -> AdvancementSummary {
    let mut advancement = input_advancement(id, parent);
    advancement.display = Some(AdvancementDisplaySummary {
        title: bbb_protocol::RichTextComponent::plain(id),
        description: bbb_protocol::RichTextComponent::plain(""),
        icon: AdvancementIconSummary {
            item_id: 1,
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        },
        frame_type: AdvancementFrameType::Task,
        show_toast: false,
        hidden: false,
        background: None,
        x: 0.0,
        y: 0.0,
    });
    advancement
}

#[test]
fn advancements_key_is_consumed_while_container_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 9,
        menu_type_id: VANILLA_MENU_TYPE_MERCHANT_ID,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(!world.advancements_screen_is_open());
    assert!(world.inventory().open_container.is_some());
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_is_consumed_while_dialog_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(!world.advancements_screen_is_open());
    assert!(world.current_dialog().is_some());
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn escape_key_closes_advancements_screen_and_queues_seen_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_advancements_screen());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!world.advancements_screen_is_open());
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_closes_advancements_screen_and_queues_seen_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_advancements_screen());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(!world.advancements_screen_is_open());
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn stats_screen_done_button_closes_stats_screen() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    assert!(world.open_stats_screen());

    assert!(input.handle_stats_screen_cursor_moved(
        &world,
        Some(PhysicalPosition::new(70.0, 223.0)),
        surface,
    ));
    assert_eq!(input.stats_screen_cursor_position(), Some((70, 223)));
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(70.0, 223.0)),
        surface,
    ));

    assert!(!world.stats_screen_is_open());
    assert_eq!(input.stats_screen_cursor_position(), None);
}

#[test]
fn stats_screen_wheel_uses_half_row_pixels_and_resets_on_close() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_stats_screen());
    input.sync_stats_screen_scroll_metrics(100, 32);

    assert!(input.handle_stats_screen_mouse_wheel(&world, MouseScrollDelta::LineDelta(0.0, -3.0),));
    assert_eq!(input.stats_screen_scroll_amount(), 21.0);
    assert!(input.handle_stats_screen_mouse_wheel(
        &world,
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -2.5)),
    ));
    assert_eq!(input.stats_screen_scroll_amount(), 23.5);
    assert!(input.handle_stats_screen_mouse_wheel(&world, MouseScrollDelta::LineDelta(0.0, 1.0),));
    assert_eq!(input.stats_screen_scroll_amount(), 16.5);

    input.sync_stats_screen_scroll_metrics(10, 32);
    assert_eq!(input.stats_screen_scroll_amount(), 10.0);
    assert!(input.handle_stats_screen_mouse_wheel(&world, MouseScrollDelta::LineDelta(0.0, 10.0),));
    assert_eq!(input.stats_screen_scroll_amount(), 0.0);

    close_stats_screen(&mut input, &mut counters, &mut world, &None);
    assert_eq!(input.stats_screen_scroll_amount(), 0.0);
    assert!(!input.handle_stats_screen_mouse_wheel(&world, MouseScrollDelta::LineDelta(0.0, -1.0),));
}

#[test]
fn stats_screen_scrollbar_drag_tracks_pixel_scroll_bounds() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    assert!(world.open_stats_screen());
    input.sync_stats_screen_scroll_metrics(140, 80);
    assert!(input.handle_stats_screen_cursor_moved(
        &world,
        Some(PhysicalPosition::new(308.0, 60.0)),
        surface,
    ));
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(308.0, 60.0)),
        surface,
    ));

    assert!(input.handle_stats_screen_cursor_moved(
        &world,
        Some(PhysicalPosition::new(308.0, 70.0)),
        surface,
    ));
    assert!((input.stats_screen_scroll_amount() - 17.283_950_617).abs() < 1e-6);
    assert!(input.handle_stats_screen_cursor_moved(
        &world,
        Some(PhysicalPosition::new(308.0, 20.0)),
        surface,
    ));
    assert_eq!(input.stats_screen_scroll_amount(), 0.0);
    assert!(input.handle_stats_screen_cursor_moved(
        &world,
        Some(PhysicalPosition::new(308.0, 220.0)),
        surface,
    ));
    assert_eq!(input.stats_screen_scroll_amount(), 140.0);

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Released,
        Some(PhysicalPosition::new(308.0, 220.0)),
        surface,
    ));
    assert!(input.handle_stats_screen_cursor_moved(
        &world,
        Some(PhysicalPosition::new(308.0, 100.0)),
        surface,
    ));
    assert_eq!(input.stats_screen_scroll_amount(), 140.0);
}

#[test]
fn stats_screen_tab_hitboxes_match_renderer_column_widths() {
    let surface = PhysicalSize::new(320, 240);
    assert_eq!(
        stats_screen_tab_at_position(Some((112, 30)), surface),
        Some(0)
    );
    assert_eq!(
        stats_screen_tab_at_position(Some((113, 30)), surface),
        Some(1)
    );
    assert_eq!(
        stats_screen_tab_at_position(Some((205, 30)), surface),
        Some(1)
    );
    assert_eq!(
        stats_screen_tab_at_position(Some((206, 30)), surface),
        Some(2)
    );
    assert_eq!(stats_screen_tab_at_position(Some((20, 42)), surface), None);
    assert_eq!(
        stats_screen_item_column_at_position(Some((77, 49)), surface, 0),
        Some(0)
    );
    assert_eq!(
        stats_screen_item_column_at_position(Some((94, 66)), surface, 0),
        Some(0)
    );
    assert_eq!(
        stats_screen_item_column_at_position(Some((95, 50)), surface, 0),
        None
    );
    assert_eq!(
        stats_screen_item_column_at_position(Some((157, 50)), surface, 0),
        Some(2)
    );
    assert_eq!(
        stats_screen_item_column_at_position(Some((157, 50)), surface, 10),
        Some(2)
    );
    assert_eq!(
        stats_screen_item_column_at_position(Some((157, 50)), surface, 22),
        None
    );
}

#[test]
fn stats_screen_control_tab_and_digits_skip_disabled_tabs() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_stats_screen());
    world.apply_award_stats(AwardStats {
        stats: vec![
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                value_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:apple",
                )
                .unwrap()
                .id,
                amount: 1,
            },
            StatUpdate {
                stat_type_id: VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                value_id: VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                amount: 1,
            },
        ],
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
    );
    assert_eq!(input.stats_screen_tab(), 1);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.stats_screen_tab(), 2);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Numpad1),
        ElementState::Pressed,
    );
    assert_eq!(input.stats_screen_tab(), 0);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Digit9),
        ElementState::Pressed,
    );
    assert_eq!(input.stats_screen_tab(), 2);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.stats_screen_tab(), 1);

    let mut empty_world = WorldStore::new();
    assert!(empty_world.open_stats_screen());
    empty_world.apply_award_stats(AwardStats { stats: Vec::new() });
    input.stats_screen_tab = 0;
    handle_key_input(
        &mut input,
        &mut counters,
        &mut empty_world,
        &None,
        PhysicalKey::Code(KeyCode::Digit3),
        ElementState::Pressed,
    );
    assert_eq!(input.stats_screen_tab(), 0);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut empty_world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.stats_screen_tab(), 0);
}

#[test]
fn stats_screen_focus_matches_loading_replacement_and_widget_tab_order() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_stats_screen(&mut input, &mut counters, &mut world, &None);

    input.sync_stats_screen_interaction(true, 0, 0);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::None);
    input.sync_stats_screen_interaction(false, 0, 3);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::Tabs);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::GeneralRow(0));
    stats_screen_move_tab_focus(&mut input, true);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::Done);
    stats_screen_move_tab_focus(&mut input, true);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::Tabs);

    input.stats_screen_tab = 1;
    input.sync_stats_screen_interaction(false, 1, 2);
    input.stats_screen_focus = StatsScreenFocus::Tabs;
    stats_screen_move_tab_focus(&mut input, true);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::ItemHeader(0));
    for column in 1..6 {
        stats_screen_move_tab_focus(&mut input, true);
        assert_eq!(
            input.stats_screen_focus(),
            StatsScreenFocus::ItemHeader(column)
        );
    }
    stats_screen_move_tab_focus(&mut input, true);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::ItemRow(0));
    stats_screen_move_tab_focus(&mut input, true);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::ItemRow(1));
    stats_screen_move_tab_focus(&mut input, true);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::Done);
    stats_screen_move_tab_focus(&mut input, false);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::ItemRow(1));
}

#[test]
fn stats_screen_focus_arrow_scroll_and_activation_follow_widget_paths() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_stats_screen(&mut input, &mut counters, &mut world, &None);
    input.stats_screen_was_loading = false;
    input.stats_screen_tab = 1;
    input.sync_stats_screen_interaction(false, 1, 12);
    input.stats_screen_focus = StatsScreenFocus::ItemHeader(0);

    stats_screen_move_arrow_focus(&mut input, &world, KeyCode::ArrowRight);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::ItemHeader(1));
    assert!(!stats_screen_activate_focus(&mut input));
    assert_eq!(input.stats_screen_item_sort(), (Some(1), -1));
    assert!(!stats_screen_activate_focus(&mut input));
    assert_eq!(input.stats_screen_item_sort(), (Some(1), 1));
    assert!(!stats_screen_activate_focus(&mut input));
    assert_eq!(input.stats_screen_item_sort(), (None, 0));

    stats_screen_move_arrow_focus(&mut input, &world, KeyCode::ArrowDown);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::ItemRow(0));
    for _ in 0..11 {
        stats_screen_move_arrow_focus(&mut input, &world, KeyCode::ArrowDown);
    }
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::ItemRow(11));
    input.sync_stats_screen_scroll_metrics(200, 40);
    input.keep_stats_screen_focus_visible(66, 22);
    assert_eq!(input.stats_screen_scroll_amount(), 222.0_f64.min(200.0));
    stats_screen_move_arrow_focus(&mut input, &world, KeyCode::ArrowDown);
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::Done);
    assert!(stats_screen_activate_focus(&mut input));
}

#[test]
fn stats_screen_focus_mouse_clicks_rows_headers_and_tabs() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    assert!(world.open_stats_screen());
    world.apply_award_stats(AwardStats { stats: Vec::new() });
    input.stats_screen_tab = 0;
    input.sync_stats_screen_interaction(false, 0, 20);

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(160.0, 64.0)),
        surface,
    ));
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::GeneralRow(1));

    input.stats_screen_tab = 1;
    input.sync_stats_screen_interaction(false, 1, 3);
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(117.0, 50.0)),
        surface,
    ));
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::ItemHeader(1));

    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(70.0, 30.0)),
        surface,
    ));
    assert_eq!(input.stats_screen_focus(), StatsScreenFocus::Tabs);
}

#[test]
fn escape_key_closes_stats_screen_without_requesting_stats_again() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_stats_screen());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!world.stats_screen_is_open());
    assert_eq!(counters.request_stats_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_done_button_click_closes_screen_and_queues_seen_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_advancements_screen());

    let handled = handle_advancements_screen_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(400.0, 580.0)),
        PhysicalSize::new(800, 600),
    );

    assert!(handled);
    assert!(!world.advancements_screen_is_open());
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_tab_click_selects_tab_and_queues_opened_tab() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![
            input_displayed_advancement("minecraft:y/root", None),
            input_displayed_advancement("minecraft:a/root", None),
        ],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    assert!(world.open_advancements_screen());
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:y/root".to_string())
    );

    let handled = handle_advancements_screen_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(316.0, 212.0)),
        PhysicalSize::new(800, 600),
    );

    assert!(handled);
    assert!(world.advancements_screen_is_open());
    assert_eq!(world.selected_advancements_tab(), Some("minecraft:a/root"));
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:a/root".to_string()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_mouse_wheel_scrolls_selected_tab_locally() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_displayed_advancement("minecraft:story/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    assert!(world.open_advancements_screen());
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:story/root".to_string())
    );

    let handled = handle_advancements_screen_mouse_wheel(
        &mut input,
        &world,
        MouseScrollDelta::LineDelta(0.0, -1.0),
    );

    assert!(handled);
    assert_eq!(
        input.advancement_scroll_delta(Some("minecraft:story/root")),
        Some((0.0, -16.0))
    );
}

#[test]
fn gameplay_mouse_wheel_is_consumed_while_advancements_screen_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(0));
    assert!(world.open_advancements_screen());

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_left_drag_scrolls_after_first_drag_event_until_release() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_displayed_advancement("minecraft:story/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    assert!(world.open_advancements_screen());
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:story/root".to_string())
    );

    assert!(handle_advancements_screen_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        None,
        PhysicalSize::new(800, 600),
    ));
    assert!(handle_advancements_screen_cursor_moved(
        &mut input,
        &world,
        Some(PhysicalPosition::new(100.0, 100.0)),
        Some(PhysicalPosition::new(110.0, 120.0)),
    ));
    assert_eq!(input.inventory_cursor_position(), Some((110, 120)));
    assert_eq!(input.inventory_hovered_slot(), None);
    assert_eq!(
        input.advancement_scroll_delta(Some("minecraft:story/root")),
        None
    );

    assert!(handle_advancements_screen_cursor_moved(
        &mut input,
        &world,
        Some(PhysicalPosition::new(110.0, 120.0)),
        Some(PhysicalPosition::new(115.0, 116.0)),
    ));
    assert_eq!(
        input.advancement_scroll_delta(Some("minecraft:story/root")),
        Some((5.0, -4.0))
    );

    assert!(handle_advancements_screen_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        None,
        PhysicalSize::new(800, 600),
    ));
    assert!(handle_advancements_screen_cursor_moved(
        &mut input,
        &world,
        Some(PhysicalPosition::new(115.0, 116.0)),
        Some(PhysicalPosition::new(120.0, 150.0)),
    ));
    assert_eq!(
        input.advancement_scroll_delta(Some("minecraft:story/root")),
        Some((5.0, -4.0))
    );
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn movement_key_changes_queue_player_input_commands() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            ..PlayerInput::default()
        })
    );

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.player_input_commands_queued, 1);

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );

    assert_eq!(counters.player_input_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
}

#[test]
fn release_active_input_keeps_shift_modifier_for_inventory_clicks() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    assert!(input.shift_down());
    assert!(input.sneak);

    release_active_input(&mut input, &mut world, &mut counters, &None);

    assert!(input.shift_down());
    assert!(!input.sneak);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    );
    assert!(!input.shift_down());
}

#[test]
fn item_model_keybind_context_tracks_default_key_and_mouse_state() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    let context = input.item_model_keybind_context();
    assert!(context.keybind_down("key.forward"));
    assert!(context.keybind_down("key.hotbar.5"));
    assert!(!context.keybind_down("key.use"));

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Released,
    );

    let context = input.item_model_keybind_context();
    assert!(!context.keybind_down("key.forward"));
    assert!(!context.keybind_down("key.hotbar.5"));

    let mut world = WorldStore::new();
    handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
    );
    assert!(input.item_model_keybind_context().keybind_down("key.use"));
    assert!(!input
        .item_model_keybind_context()
        .keybind_down("key.attack"));

    handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseButton::Right,
        ElementState::Released,
        1.0,
    );
    assert!(!input.item_model_keybind_context().keybind_down("key.use"));
}

#[test]
fn item_model_keybind_context_tracks_non_debug_default_keymappings() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    for (code, keybind) in [
        (KeyCode::F2, "key.screenshot"),
        (KeyCode::F5, "key.togglePerspective"),
        (KeyCode::F11, "key.fullscreen"),
        (KeyCode::KeyG, "key.quickActions"),
        (KeyCode::F1, "key.toggleGui"),
        (KeyCode::F4, "key.toggleSpectatorShaderEffects"),
        (KeyCode::KeyC, "key.saveToolbarActivator"),
        (KeyCode::KeyX, "key.loadToolbarActivator"),
    ] {
        handle_key_input_without_world(
            &mut input,
            &mut counters,
            &None,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
        assert!(input.item_model_keybind_context().keybind_down(keybind));
        handle_key_input_without_world(
            &mut input,
            &mut counters,
            &None,
            PhysicalKey::Code(code),
            ElementState::Released,
        );
        assert!(!input.item_model_keybind_context().keybind_down(keybind));
    }

    // Screen-opening bindings release all active input after their key press,
    // so exercise their raw keymapping projection without opening the screen.
    input.set_key_down(KeyCode::KeyP, true);
    assert!(input
        .item_model_keybind_context()
        .keybind_down("key.socialInteractions"));
    input.set_key_down(KeyCode::KeyP, false);
    assert!(!input
        .item_model_keybind_context()
        .keybind_down("key.socialInteractions"));

    input.set_key_down(KeyCode::KeyL, true);
    assert!(input
        .item_model_keybind_context()
        .keybind_down("key.advancements"));
    input.set_key_down(KeyCode::KeyL, false);
    assert!(!input
        .item_model_keybind_context()
        .keybind_down("key.advancements"));

    let mut world = WorldStore::new();
    handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseButton::Middle,
        ElementState::Pressed,
        1.0,
    );
    let context = input.item_model_keybind_context();
    assert!(context.keybind_down("key.pickItem"));
    assert!(context.keybind_down("key.spectatorHotbar"));
}

#[test]
fn toggle_gui_key_flips_hide_gui_once_per_press_edge() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    assert!(!input.hide_gui());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert!(input.hide_gui());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert!(input.hide_gui());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Released,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F1),
        ElementState::Pressed,
    );
    assert!(!input.hide_gui());
}

#[test]
fn item_model_keybind_context_accepts_vanilla_default_unbound_keymappings() {
    // Vanilla Options registers these with InputConstants.UNKNOWN, so
    // IsKeybindDown can decode the key names but KeyMapping.isDown() is false
    // under the default keymap.
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    for code in [KeyCode::F4, KeyCode::F5] {
        handle_key_input_without_world(
            &mut input,
            &mut counters,
            &None,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }

    let context = input.item_model_keybind_context();
    assert!(!context.keybind_down("key.smoothCamera"));
    assert!(!context.keybind_down("key.spectatorOutlines"));
}

fn assert_sprint_key_on_mount_only_queues_raw_player_input(entity_type_id: i32) {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, entity_type_id);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

fn assert_sprint_key_on_camel_type_queues_start_sprinting(entity_type_id: i32) {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, entity_type_id);
    world.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 6,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_with_forward_input_queues_player_input_and_sprint_commands() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        })
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );

    assert_eq!(counters.player_input_commands_queued, 2);
    assert_eq!(counters.player_command_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StopSprinting,
            data: 0,
        })
    );
}

#[test]
fn double_tap_forward_within_sprint_window_queues_start_sprinting() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn double_tap_forward_after_sprint_window_expires_does_not_start_sprinting() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );

    let start = Instant::now();
    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(250),
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(350),
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_without_forward_input_does_not_start_sprinting_until_forward_pressed() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 2);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        })
    );
}

#[test]
fn sprint_key_with_low_food_only_queues_raw_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    world.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 6,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_while_using_slow_item_only_queues_raw_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: test_item_stack(42, 1),
    });
    world.set_local_using_item(true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_on_horse_mount_only_queues_raw_player_input() {
    assert_sprint_key_on_mount_only_queues_raw_player_input(VANILLA_ENTITY_TYPE_HORSE_ID);
}

#[test]
fn sprint_key_on_boat_mount_only_queues_raw_player_input() {
    assert_sprint_key_on_mount_only_queues_raw_player_input(VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
}

#[test]
fn sprint_key_on_camel_mount_queues_player_input_and_sprint_command_with_low_food() {
    for entity_type_id in [
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ] {
        assert_sprint_key_on_camel_type_queues_start_sprinting(entity_type_id);
    }
}

#[test]
fn sprint_key_camel_mount_focus_loss_queues_stop_sprinting() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    input.sprint = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, VANILLA_ENTITY_TYPE_CAMEL_ID);

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StopSprinting,
            data: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_without_local_player_id_only_queues_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_double_tap_toggles_creative_flight() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 1);
    assert!(world.local_player().abilities.unwrap().flying);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAbilities(PlayerAbilitiesCommand { flying: true })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_double_tap_without_can_fly_only_queues_player_input() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 0);
    assert!(!world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn spectator_jump_key_double_tap_does_not_toggle_flying_off() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_spectator(&mut world);
    set_player_abilities(&mut world, true, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 0);
    assert!(world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn creative_flight_jump_trigger_expires_after_vanilla_window() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let start = Instant::now();
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(250),
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(350),
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 0);
    assert!(!world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn creative_flight_toggle_suppresses_same_jump_fall_flying_command() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, true);
    set_local_player_on_ground(&mut world, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );

    set_local_player_on_ground(&mut world, false);
    equip_player_slot(
        &mut world,
        VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        VANILLA_ITEM_ELYTRA_ID,
        1,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert!(world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAbilities(PlayerAbilitiesCommand { flying: true })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_queues_start_fall_flying_when_airborne_with_elytra() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_on_ground(&mut world, false);
    equip_player_slot(
        &mut world,
        VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        VANILLA_ITEM_ELYTRA_ID,
        1,
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartFallFlying,
            data: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_on_ground_with_elytra_only_queues_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_on_ground(&mut world, true);
    equip_player_slot(
        &mut world,
        VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        VANILLA_ITEM_ELYTRA_ID,
        1,
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_airborne_without_elytra_only_queues_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_on_ground(&mut world, false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn focus_loss_clears_pressed_input_and_queues_release() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    input.jump = true;
    input.sprint = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
}

#[test]
fn focus_loss_stops_active_sprinting() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    input.sprint = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StopSprinting,
            data: 0,
        })
    );
}

#[test]
fn focus_loss_stops_boat_paddles() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.left = true;
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_boat(77);

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.paddle_boat_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PaddleBoat(PaddleBoat {
            left: false,
            right: false,
        })
    );
}

#[test]
fn focus_loss_releases_charged_riding_jump() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.jump = true;
    input.riding_jump_charge_seconds = Some(0.2);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, VANILLA_ENTITY_TYPE_HORSE_ID);

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(input.riding_jump_charge_seconds, None);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartRidingJump,
            data: 40,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn focus_loss_aborts_destroying_block() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_destroying_block(BlockPos { x: 4, y: 70, z: -6 });
    let mut counters = NetCounters::default();

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::AbortDestroyBlock,
            pos: ProtocolBlockPos { x: 4, y: 70, z: -6 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn focus_loss_releases_using_item() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!world.local_player().interaction.using_item);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::ReleaseUseItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

fn test_item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn open_test_book_screen(world: &mut WorldStore, pages: Vec<&str>) {
    let mut stack = test_item_stack(42, 1);
    let pages: Vec<String> = pages.into_iter().map(str::to_string).collect();
    let page_filters = vec![None; pages.len()];
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Guide".to_string(),
        title_filter: None,
        author: "Alex".to_string(),
        generation: 0,
        pages,
        page_filters,
        resolved: true,
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    world.apply_open_book(OpenBook {
        hand: InteractionHand::MainHand,
    });
    assert!(world.current_book().is_some());
}

fn test_bundle_stack(
    item_id: i32,
    count: i32,
    bundle_contents_item_count: usize,
) -> ProtocolItemStackSummary {
    let mut stack = test_item_stack(item_id, count);
    stack.component_patch.bundle_contents_item_count = Some(bundle_contents_item_count);
    stack
}

fn test_hashed_item_stack(item_id: i32, count: i32) -> HashedStack {
    HashedStack::Item(HashedItemStack {
        item_id,
        count,
        components: HashedComponentPatch::default(),
    })
}

fn generic_9x1_container_world(
    container_id: i32,
    state_id: i32,
    item: Option<(usize, ProtocolItemStackSummary)>,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: "Chest".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 45];
    if let Some((slot, stack)) = item {
        items[slot] = stack;
    }
    world.apply_container_set_content(ProtocolContainerSetContent {
        container_id,
        state_id,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    world
}

fn open_recipe_book_crafting_table_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ProtocolItemStackSummary::empty(); 46],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world
}

fn open_recipe_book_furnace_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: "Furnace".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ProtocolItemStackSummary::empty(); 39],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        furnace: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world
}

fn recipe_book_shapeless_entry(
    id: i32,
    category_id: i32,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    recipe_book_shapeless_entry_with_group(id, category_id, None, result_item_id)
}

fn recipe_book_shapeless_entry_with_group(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    bbb_protocol::packets::RecipeBookAddEntry {
        contents: bbb_protocol::packets::RecipeDisplayEntry {
            id: bbb_protocol::packets::RecipeDisplayId { index: id },
            display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
                raw_body: Vec::new(),
                crafting: Some(
                    bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                        ingredients: Vec::new(),
                        result: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
                            raw_payload: Vec::new(),
                            item_stack: Some(test_item_stack(result_item_id, 1)),
                            tag: None,
                        },
                        crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                            raw_payload: Vec::new(),
                            item_stack: None,
                            tag: None,
                        },
                    },
                ),
                furnace: None,
            },
            group,
            category_id,
            crafting_requirements: None,
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

fn recipe_book_furnace_entry(
    id: i32,
    category_id: i32,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    bbb_protocol::packets::RecipeBookAddEntry {
        contents: bbb_protocol::packets::RecipeDisplayEntry {
            id: bbb_protocol::packets::RecipeDisplayId { index: id },
            display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::Furnace,
                raw_body: Vec::new(),
                crafting: None,
                furnace: Some(bbb_protocol::packets::FurnaceRecipeDisplaySummary {
                    ingredient: recipe_book_slot_display_item(1),
                    fuel: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
                        raw_payload: vec![1],
                        item_stack: None,
                        tag: None,
                    },
                    result: recipe_book_slot_display_stack(result_item_id, 1),
                    crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                        raw_payload: vec![0],
                        item_stack: None,
                        tag: None,
                    },
                    duration: 200,
                    experience_bits: 0.0_f32.to_bits(),
                }),
            },
            group: None,
            category_id,
            crafting_requirements: None,
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

fn recipe_book_slot_display_item(item_id: i32) -> bbb_protocol::packets::SlotDisplaySummary {
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        raw_payload: Vec::new(),
        item_stack: Some(test_item_stack(item_id, 1)),
        tag: None,
    }
}

fn recipe_book_slot_display_stack(
    item_id: i32,
    count: i32,
) -> bbb_protocol::packets::SlotDisplaySummary {
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
        raw_payload: Vec::new(),
        item_stack: Some(test_item_stack(item_id, count)),
        tag: None,
    }
}

fn open_container_slot_bundle_selection(world: &WorldStore, slot: i16) -> Option<i32> {
    world
        .inventory()
        .open_container
        .as_ref()?
        .slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.local_selected_bundle_item_index)
}

fn anvil_container_world(
    container_id: i32,
    state_id: i32,
    input_item: Option<ProtocolItemStackSummary>,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: "Anvil".to_string(),
        title_styled: Vec::new(),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    if let Some(stack) = input_item {
        items[0] = stack;
    }
    world.apply_container_set_content(ProtocolContainerSetContent {
        container_id,
        state_id,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    world
}

fn write_input_tooltip_item_assets(root: &Path) {
    let assets = input_assets_dir(root);
    write_input_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_input_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "block/",
                    "source": "block"
                }
            ]
        }"#,
    );
    write_input_json(
        &assets.join("items").join("test_combo.json"),
        r#"{
            "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/test_combo"
            }
        }"#,
    );
    write_input_json(
        &assets.join("models").join("item").join("test_combo.json"),
        r#"{
            "textures": {
                "layer0": "minecraft:item/test_combo"
            }
        }"#,
    );
    write_input_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
            "item.minecraft.test_combo": "Test Combo"
        }"#,
    );
    write_input_png(
        &assets.join("textures").join("item").join("test_combo.png"),
        &[80, 120, 160, 255],
    );
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["test_combo"]);
}

fn input_assets_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_pack::MC_VERSION)
        .join("assets")
        .join("minecraft")
}

fn write_input_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn write_input_png(path: &Path, rgba: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::save_buffer(path, rgba, 1, 1, image::ColorType::Rgba8).unwrap();
}

fn unique_input_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("bbb-native-input-{label}-{nanos}"))
}

fn test_player_slot_item(world: &WorldStore, slot: i32) -> ProtocolItemStackSummary {
    world
        .inventory()
        .player_slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.item.clone())
        .unwrap_or_else(ProtocolItemStackSummary::empty)
}

fn simple_probe_entity(world: &mut WorldStore, id: i32, entity_type_id: i32) {
    world.apply_add_entity(AddEntity {
        id,
        uuid: Uuid::from_u128(id as u128),
        entity_type_id,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
}

#[test]
fn villager_local_save_projects_synced_villager_data_and_finalized_flag() {
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 300, VANILLA_ENTITY_TYPE_VILLAGER_ID);
    let villager_data_id =
        bbb_world::vanilla_entity_metadata_villager_data_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
        )
        .expect("villager data accessor");
    let (finalized_id, _) =
        bbb_world::vanilla_entity_metadata_villager_data_finalized_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
        )
        .expect("villager data finalized accessor");
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 300,
        values: vec![
            ProtocolEntityDataValue {
                data_id: villager_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_VILLAGER_DATA_ID,
                // Vanilla registration order: villager_type plains = 2,
                // profession farmer = 5; the store rebinds identities from
                // the holder ids through the registry fallback.
                value: EntityDataValueKind::VillagerData {
                    villager_type: EntityDataRegistryReference::unresolved(2),
                    profession: EntityDataRegistryReference::unresolved(5),
                    level: 2,
                },
            },
            ProtocolEntityDataValue {
                data_id: finalized_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));

    let entity = world.probe_entity(300).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains(
        "VillagerData: {type: \"minecraft:plains\", profession: \"minecraft:farmer\", level: 2}"
    ));
    assert!(snbt.contains("VillagerDataFinalized: 1b"));
    assert!(!snbt.contains("Gossips"));
    assert!(!snbt.contains("FoodLevel"));
}

#[test]
fn zombie_villager_local_save_projects_conversion_and_villager_data() {
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 301, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID);
    let (converting_id, _) =
        bbb_world::vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
        )
        .expect("zombie villager converting accessor");
    let villager_data_id =
        bbb_world::vanilla_entity_metadata_villager_data_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
        )
        .expect("zombie villager data accessor");

    let entity = world.probe_entity(301).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains("ConversionTime: -1"));
    assert!(snbt.contains("IsBaby: 0b"));
    assert!(snbt.contains("VillagerDataFinalized: 0b"));
    assert!(!snbt.contains("VillagerData: {"));

    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 301,
        values: vec![
            ProtocolEntityDataValue {
                data_id: converting_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: villager_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_VILLAGER_DATA_ID,
                // Vanilla registration order: villager_type swamp = 5,
                // profession cleric = 4.
                value: EntityDataValueKind::VillagerData {
                    villager_type: EntityDataRegistryReference::unresolved(5),
                    profession: EntityDataRegistryReference::unresolved(4),
                    level: 1,
                },
            },
        ],
    }));
    let entity = world.probe_entity(301).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains("ConversionTime: 0"));
    assert!(snbt.contains(
        "VillagerData: {type: \"minecraft:swamp\", profession: \"minecraft:cleric\", level: 1}"
    ));
}

#[test]
fn warden_local_save_keeps_server_private_anger_absent() {
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 302, VANILLA_ENTITY_TYPE_WARDEN_ID);

    let entity = world.probe_entity(302).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(!snbt.contains("anger"));
    assert!(!snbt.contains("listener"));
    assert!(snbt.contains("Motion"));
}

#[test]
fn ender_dragon_local_save_projects_synced_phase() {
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 303, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID);
    let (phase_id, _) =
        bbb_world::vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        )
        .expect("ender dragon phase accessor");
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 303,
        values: vec![ProtocolEntityDataValue {
            data_id: phase_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(7),
        }],
    }));

    let entity = world.probe_entity(303).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains("DragonPhase: 7, DragonDeathTime: 0"));
}

#[test]
fn area_effect_cloud_local_save_projects_client_age_and_synced_radius() {
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 304, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID);
    let (radius_id, _) =
        bbb_world::vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        )
        .expect("area effect cloud radius accessor");
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 304,
        values: vec![ProtocolEntityDataValue {
            data_id: radius_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            value: EntityDataValueKind::Float(2.5),
        }],
    }));

    let entity = world.probe_entity(304).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains("Age: 0, Radius: 2.5f"));
    assert!(!snbt.contains("Duration"));
    assert!(!snbt.contains("potion_contents"));
}

fn diamond_item_runtime() -> NativeItemRuntime {
    let registry: ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": ["minecraft:diamond"],
        "protocol_ids": {"minecraft:diamond": 0}
    }))
    .unwrap();
    NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        EquipmentAssetCatalog::default(),
    )
}

fn synced_item_stack_value(data_id: u8, count: i32) -> ProtocolEntityDataValue {
    ProtocolEntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        value: EntityDataValueKind::ItemStack(ProtocolItemStackSummary {
            item_id: Some(0),
            count,
            component_patch: DataComponentPatchSummary::default(),
        }),
    }
}

#[test]
fn item_entity_local_save_projects_client_age_and_synced_stack() {
    let item_runtime = diamond_item_runtime();
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 305, VANILLA_ENTITY_TYPE_ITEM_ID);
    let stack_id = bbb_world::vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )
    .expect("item entity stack accessor");
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 305,
        values: vec![synced_item_stack_value(stack_id, 3)],
    }));

    let entity = world.probe_entity(305).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity).unwrap();
    assert!(snbt.contains("Age: 0s, Item: {id: \"minecraft:diamond\", count: 3}"));
    assert!(!snbt.contains("Health"));
    assert!(!snbt.contains("PickupDelay"));
}

#[test]
fn eye_of_ender_local_save_projects_synced_stack() {
    let item_runtime = diamond_item_runtime();
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 306, VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID);
    let stack_id = bbb_world::vanilla_entity_metadata_eye_of_ender_item_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID,
    )
    .expect("eye of ender item accessor");
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 306,
        values: vec![synced_item_stack_value(stack_id, 1)],
    }));

    let entity = world.probe_entity(306).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity).unwrap();
    assert!(snbt.contains("Item: {id: \"minecraft:diamond\"}"));
}

#[test]
fn ominous_item_spawner_local_save_projects_lowercase_item_field() {
    let item_runtime = diamond_item_runtime();
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 307, VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID);
    let stack_id =
        bbb_world::vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
        )
        .expect("ominous item spawner accessor");

    let entity = world.probe_entity(307).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity);
    assert!(snbt
        .as_deref()
        .map(|text| !text.contains("item:"))
        .unwrap_or(true));

    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 307,
        values: vec![synced_item_stack_value(stack_id, 1)],
    }));
    let entity = world.probe_entity(307).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity).unwrap();
    assert!(snbt.contains("item: {id: \"minecraft:diamond\"}"));
    assert!(!snbt.contains("spawn_item_after_ticks"));
}

fn display_data_id(entity_type_id: i32, java_name: &str) -> u8 {
    bbb_world::vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, java_name)
        .unwrap_or_else(|| panic!("missing {java_name} accessor"))
        .data_id
}

#[test]
fn block_display_local_save_projects_display_defaults_and_synced_block_state() {
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 308, VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID);

    let entity = world.probe_entity(308).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains(
        "transformation: {translation: [0.0f, 0.0f, 0.0f], left_rotation: [0.0f, 0.0f, 0.0f, \
         1.0f], scale: [1.0f, 1.0f, 1.0f], right_rotation: [0.0f, 0.0f, 0.0f, 1.0f]}"
    ));
    assert!(snbt.contains("billboard: \"fixed\""));
    assert!(snbt.contains("interpolation_duration: 0, teleport_duration: 0"));
    assert!(snbt.contains(
        "view_range: 1.0f, shadow_radius: 0.0f, shadow_strength: 1.0f, width: 0.0f, height: 0.0f"
    ));
    assert!(snbt.contains("glow_color_override: -1"));
    assert!(!snbt.contains("brightness"));
    assert!(snbt.contains("block_state: {Name: \"minecraft:air\"}"));

    let slab_properties: BTreeMap<String, String> = [
        ("type".to_string(), "top".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]
    .into_iter()
    .collect();
    let slab_state_id = world
        .registries()
        .block_state_id_by_name_and_properties("minecraft:oak_slab", &slab_properties)
        .expect("oak slab state");
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 308,
        values: vec![
            ProtocolEntityDataValue {
                data_id: display_data_id(VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID, "DATA_BLOCK_STATE_ID"),
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID,
                value: EntityDataValueKind::BlockState(slab_state_id),
            },
            ProtocolEntityDataValue {
                data_id: display_data_id(VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID, "DATA_TRANSLATION_ID"),
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID,
                value: EntityDataValueKind::Vector3f {
                    x: 0.5,
                    y: -1.0,
                    z: 0.25,
                },
            },
            ProtocolEntityDataValue {
                data_id: display_data_id(
                    VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
                    "DATA_BRIGHTNESS_OVERRIDE_ID",
                ),
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                // LightCoordsUtil packing: block << 4 | sky << 20.
                value: EntityDataValueKind::Int((7 << 4) | (15 << 20)),
            },
        ],
    }));
    let entity = world.probe_entity(308).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains("translation: [0.5f, -1.0f, 0.25f]"));
    assert!(snbt.contains("brightness: {block: 7, sky: 15}"));
    assert!(snbt.contains(
        "block_state: {Name: \"minecraft:oak_slab\", Properties: {type: \"top\", waterlogged: \
         \"false\"}}"
    ));
}

#[test]
fn item_display_local_save_projects_synced_stack_and_display_context() {
    let item_runtime = diamond_item_runtime();
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 309, VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID);

    let entity = world.probe_entity(309).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity).unwrap();
    // Vanilla omits the empty display stack but always stores the context.
    assert!(!snbt.contains("item: {"));
    assert!(snbt.contains("item_display: \"none\""));

    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 309,
        values: vec![
            synced_item_stack_value(
                display_data_id(VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID, "DATA_ITEM_STACK_ID"),
                1,
            ),
            ProtocolEntityDataValue {
                data_id: display_data_id(
                    VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID,
                    "DATA_ITEM_DISPLAY_ID"
                ),
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                // ItemDisplayContext GUI = 6.
                value: EntityDataValueKind::Byte(6),
            },
        ],
    }));
    let entity = world.probe_entity(309).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, Some(&item_runtime), &entity).unwrap();
    assert!(snbt.contains("item: {id: \"minecraft:diamond\"}"));
    assert!(snbt.contains("item_display: \"gui\""));
}

#[test]
fn text_display_local_save_projects_text_and_style_flags() {
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 310, VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID);

    let entity = world.probe_entity(310).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains("text: \"\""));
    assert!(snbt.contains("line_width: 200"));
    assert!(snbt.contains("background: 1073741824"));
    assert!(snbt.contains("text_opacity: -1b"));
    assert!(snbt.contains("shadow: 0b, see_through: 0b, default_background: 0b"));
    assert!(snbt.contains("alignment: \"center\""));

    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 310,
        values: vec![
            ProtocolEntityDataValue {
                data_id: display_data_id(VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID, "DATA_TEXT_ID"),
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID,
                value: EntityDataValueKind::Component(bbb_protocol::RichTextComponent::plain(
                    "hello display",
                )),
            },
            ProtocolEntityDataValue {
                data_id: display_data_id(VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID, "DATA_STYLE_FLAGS_ID"),
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                // Vanilla style flags: shadow = 1, align-left = 8.
                value: EntityDataValueKind::Byte(1 | 8),
            },
        ],
    }));
    let entity = world.probe_entity(310).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains("text: \"hello display\""));
    assert!(snbt.contains("shadow: 1b, see_through: 0b, default_background: 0b"));
    assert!(snbt.contains("alignment: \"left\""));
}

#[test]
fn mannequin_local_save_projects_avatar_defaults() {
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 311, VANILLA_ENTITY_TYPE_MANNEQUIN_ID);

    let entity = world.probe_entity(311).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains("profile: {}"));
    assert!(snbt.contains("hidden_layers: []"));
    assert!(snbt.contains("main_hand: \"right\""));
    assert!(snbt.contains("pose: \"standing\""));
    assert!(snbt.contains("immovable: 0b"));
    assert!(!snbt.contains("description"));
}

#[test]
fn mannequin_local_save_projects_synced_profile_layers_pose_and_description() {
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 312, VANILLA_ENTITY_TYPE_MANNEQUIN_ID);
    let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    let profile_id = display_data_id(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, "DATA_PROFILE");
    let layers_id = display_data_id(
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        "DATA_PLAYER_MODE_CUSTOMISATION",
    );
    let main_hand_id = display_data_id(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, "DATA_PLAYER_MAIN_HAND");
    let immovable_id = display_data_id(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, "DATA_IMMOVABLE");
    let description_id = display_data_id(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, "DATA_DESCRIPTION");
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 312,
        values: vec![
            ProtocolEntityDataValue {
                data_id: profile_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_RESOLVABLE_PROFILE_ID,
                value: EntityDataValueKind::ResolvableProfile(ResolvableProfileData {
                    profile: ResolvableProfileKind::Partial {
                        name: Some("Steve".to_string()),
                        id: Some(Uuid::from_u128(1)),
                        properties: 0,
                    },
                    skin_patch: PlayerSkinPatchData {
                        body: None,
                        cape: None,
                        elytra: None,
                        slim_model: None,
                    },
                }),
            },
            ProtocolEntityDataValue {
                data_id: layers_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                // All layers except cape (0x01) and hat (0x40) shown.
                value: EntityDataValueKind::Byte(0x3E),
            },
            ProtocolEntityDataValue {
                data_id: main_hand_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
                value: EntityDataValueKind::HumanoidArm(0),
            },
            ProtocolEntityDataValue {
                data_id: entity_accessors.pose_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                value: EntityDataValueKind::Pose(
                    bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
                ),
            },
            ProtocolEntityDataValue {
                data_id: immovable_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: description_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
                value: EntityDataValueKind::OptionalComponent(Some(
                    bbb_protocol::RichTextComponent::plain("Shop Guide"),
                )),
            },
        ],
    }));

    let entity = world.probe_entity(312).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains("profile: {name: \"Steve\", id: [I; 0, 0, 0, 1]}"));
    assert!(snbt.contains("hidden_layers: [\"cape\", \"hat\"]"));
    assert!(snbt.contains("main_hand: \"left\""));
    assert!(snbt.contains("pose: \"crouching\""));
    assert!(snbt.contains("immovable: 1b"));
    assert!(snbt.contains("description: \"Shop Guide\""));
    assert!(!snbt.contains("hide_description"));
}

#[test]
fn mannequin_local_save_hides_description_and_omits_default_label() {
    let mut world = WorldStore::new();
    simple_probe_entity(&mut world, 313, VANILLA_ENTITY_TYPE_MANNEQUIN_ID);
    let description_id = display_data_id(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, "DATA_DESCRIPTION");

    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 313,
        values: vec![ProtocolEntityDataValue {
            data_id: description_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
            value: EntityDataValueKind::OptionalComponent(None),
        }],
    }));
    let entity = world.probe_entity(313).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(snbt.contains("hide_description: 1b"));
    assert!(!snbt.contains("description: \""));

    // The synced default translatable label is omitted like vanilla's
    // DEFAULT_DESCRIPTION equality check.
    let default_label =
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "translate".to_string(),
                value: NbtSummaryValue::String("entity.minecraft.mannequin.label".to_string()),
            },
        ]))
        .unwrap();
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 313,
        values: vec![ProtocolEntityDataValue {
            data_id: description_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
            value: EntityDataValueKind::OptionalComponent(Some(default_label)),
        }],
    }));
    let entity = world.probe_entity(313).unwrap();
    let snbt = debug_local_entity_pretty_snbt(&world, None, &entity).unwrap();
    assert!(!snbt.contains("hide_description"));
    assert!(!snbt.contains("description: \""));
}
