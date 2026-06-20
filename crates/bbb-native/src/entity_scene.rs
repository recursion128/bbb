use bbb_protocol::packets::EntityDataValueKind;
use bbb_renderer::{
    EntityModelInstance, EntityModelKind, HumanoidModelFamily, QuadrupedModelFamily, SelectionBox,
    SelectionOutline,
};
use bbb_world::{EntityModelSourceState, EntityPickTargetState, WorldStore};

const VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID: i32 = 0;
const VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID: i32 = 1;
const VANILLA_ENTITY_TYPE_ALLAY_ID: i32 = 2;
const VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID: i32 = 3;
const VANILLA_ENTITY_TYPE_ARMADILLO_ID: i32 = 4;
const VANILLA_ENTITY_TYPE_ARMOR_STAND_ID: i32 = 5;
const VANILLA_ENTITY_TYPE_ARROW_ID: i32 = 6;
const VANILLA_ENTITY_TYPE_AXOLOTL_ID: i32 = 7;
const VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID: i32 = 8;
const VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID: i32 = 9;
const VANILLA_ENTITY_TYPE_BAT_ID: i32 = 10;
const VANILLA_ENTITY_TYPE_BEE_ID: i32 = 11;
const VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID: i32 = 12;
const VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID: i32 = 13;
const VANILLA_ENTITY_TYPE_BLAZE_ID: i32 = 14;
const VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID: i32 = 15;
const VANILLA_ENTITY_TYPE_BOGGED_ID: i32 = 16;
const VANILLA_ENTITY_TYPE_BREEZE_ID: i32 = 17;
const VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID: i32 = 18;
const VANILLA_ENTITY_TYPE_CAMEL_ID: i32 = 19;
const VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID: i32 = 20;
const VANILLA_ENTITY_TYPE_CAT_ID: i32 = 21;
const VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID: i32 = 22;
const VANILLA_ENTITY_TYPE_CHERRY_BOAT_ID: i32 = 23;
const VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID: i32 = 24;
const VANILLA_ENTITY_TYPE_CHEST_MINECART_ID: i32 = 25;
const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
const VANILLA_ENTITY_TYPE_COD_ID: i32 = 27;
const VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID: i32 = 28;
const VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID: i32 = 29;
const VANILLA_ENTITY_TYPE_COW_ID: i32 = 30;
const VANILLA_ENTITY_TYPE_CREAKING_ID: i32 = 31;
const VANILLA_ENTITY_TYPE_CREEPER_ID: i32 = 32;
const VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID: i32 = 33;
const VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID: i32 = 34;
const VANILLA_ENTITY_TYPE_DOLPHIN_ID: i32 = 35;
const VANILLA_ENTITY_TYPE_DONKEY_ID: i32 = 36;
const VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID: i32 = 37;
const VANILLA_ENTITY_TYPE_DROWNED_ID: i32 = 38;
const VANILLA_ENTITY_TYPE_EGG_ID: i32 = 39;
const VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID: i32 = 40;
const VANILLA_ENTITY_TYPE_ENDERMAN_ID: i32 = 41;
const VANILLA_ENTITY_TYPE_ENDERMITE_ID: i32 = 42;
const VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID: i32 = 43;
const VANILLA_ENTITY_TYPE_ENDER_PEARL_ID: i32 = 44;
const VANILLA_ENTITY_TYPE_END_CRYSTAL_ID: i32 = 45;
const VANILLA_ENTITY_TYPE_EVOKER_ID: i32 = 46;
const VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID: i32 = 47;
const VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID: i32 = 48;
const VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID: i32 = 49;
const VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID: i32 = 50;
const VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID: i32 = 51;
const VANILLA_ENTITY_TYPE_FIREBALL_ID: i32 = 52;
const VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID: i32 = 53;
const VANILLA_ENTITY_TYPE_FOX_ID: i32 = 54;
const VANILLA_ENTITY_TYPE_FROG_ID: i32 = 55;
const VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID: i32 = 56;
const VANILLA_ENTITY_TYPE_GHAST_ID: i32 = 57;
const VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID: i32 = 58;
const VANILLA_ENTITY_TYPE_GIANT_ID: i32 = 59;
const VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID: i32 = 60;
const VANILLA_ENTITY_TYPE_GLOW_SQUID_ID: i32 = 61;
const VANILLA_ENTITY_TYPE_GOAT_ID: i32 = 62;
const VANILLA_ENTITY_TYPE_GUARDIAN_ID: i32 = 63;
const VANILLA_ENTITY_TYPE_HOGLIN_ID: i32 = 64;
const VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID: i32 = 65;
const VANILLA_ENTITY_TYPE_HORSE_ID: i32 = 66;
const VANILLA_ENTITY_TYPE_HUSK_ID: i32 = 67;
const VANILLA_ENTITY_TYPE_ILLUSIONER_ID: i32 = 68;
const VANILLA_ENTITY_TYPE_INTERACTION_ID: i32 = 69;
const VANILLA_ENTITY_TYPE_IRON_GOLEM_ID: i32 = 70;
const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;
const VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID: i32 = 72;
const VANILLA_ENTITY_TYPE_ITEM_FRAME_ID: i32 = 73;
const VANILLA_ENTITY_TYPE_JUNGLE_BOAT_ID: i32 = 74;
const VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID: i32 = 75;
const VANILLA_ENTITY_TYPE_LEASH_KNOT_ID: i32 = 76;
const VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID: i32 = 77;
const VANILLA_ENTITY_TYPE_LLAMA_ID: i32 = 78;
const VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID: i32 = 79;
const VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID: i32 = 80;
const VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID: i32 = 81;
const VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID: i32 = 82;
const VANILLA_ENTITY_TYPE_MANNEQUIN_ID: i32 = 83;
const VANILLA_ENTITY_TYPE_MARKER_ID: i32 = 84;
const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;
const VANILLA_ENTITY_TYPE_MOOSHROOM_ID: i32 = 86;
const VANILLA_ENTITY_TYPE_MULE_ID: i32 = 87;
const VANILLA_ENTITY_TYPE_NAUTILUS_ID: i32 = 88;
const VANILLA_ENTITY_TYPE_OAK_BOAT_ID: i32 = 89;
const VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID: i32 = 90;
const VANILLA_ENTITY_TYPE_OCELOT_ID: i32 = 91;
const VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID: i32 = 92;
const VANILLA_ENTITY_TYPE_PAINTING_ID: i32 = 93;
const VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID: i32 = 94;
const VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID: i32 = 95;
const VANILLA_ENTITY_TYPE_PANDA_ID: i32 = 96;
const VANILLA_ENTITY_TYPE_PARCHED_ID: i32 = 97;
const VANILLA_ENTITY_TYPE_PARROT_ID: i32 = 98;
const VANILLA_ENTITY_TYPE_PHANTOM_ID: i32 = 99;
const VANILLA_ENTITY_TYPE_PIG_ID: i32 = 100;
const VANILLA_ENTITY_TYPE_PIGLIN_ID: i32 = 101;
const VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID: i32 = 102;
const VANILLA_ENTITY_TYPE_PILLAGER_ID: i32 = 103;
const VANILLA_ENTITY_TYPE_POLAR_BEAR_ID: i32 = 104;
const VANILLA_ENTITY_TYPE_SPLASH_POTION_ID: i32 = 105;
const VANILLA_ENTITY_TYPE_LINGERING_POTION_ID: i32 = 106;
const VANILLA_ENTITY_TYPE_PUFFERFISH_ID: i32 = 107;
const VANILLA_ENTITY_TYPE_RABBIT_ID: i32 = 108;
const VANILLA_ENTITY_TYPE_RAVAGER_ID: i32 = 109;
const VANILLA_ENTITY_TYPE_SALMON_ID: i32 = 110;
const VANILLA_ENTITY_TYPE_SHEEP_ID: i32 = 111;
const VANILLA_ENTITY_TYPE_SHULKER_ID: i32 = 112;
const VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID: i32 = 113;
const VANILLA_ENTITY_TYPE_SILVERFISH_ID: i32 = 114;
const VANILLA_ENTITY_TYPE_SKELETON_ID: i32 = 115;
const VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID: i32 = 116;
const VANILLA_ENTITY_TYPE_SLIME_ID: i32 = 117;
const VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID: i32 = 118;
const VANILLA_ENTITY_TYPE_SNIFFER_ID: i32 = 119;
const VANILLA_ENTITY_TYPE_SNOWBALL_ID: i32 = 120;
const VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID: i32 = 121;
const VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID: i32 = 122;
const VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID: i32 = 123;
const VANILLA_ENTITY_TYPE_SPIDER_ID: i32 = 124;
const VANILLA_ENTITY_TYPE_SPRUCE_BOAT_ID: i32 = 125;
const VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID: i32 = 126;
const VANILLA_ENTITY_TYPE_SQUID_ID: i32 = 127;
const VANILLA_ENTITY_TYPE_STRAY_ID: i32 = 128;
const VANILLA_ENTITY_TYPE_STRIDER_ID: i32 = 129;
const VANILLA_ENTITY_TYPE_TADPOLE_ID: i32 = 130;
const VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID: i32 = 131;
const VANILLA_ENTITY_TYPE_TNT_ID: i32 = 132;
const VANILLA_ENTITY_TYPE_TNT_MINECART_ID: i32 = 133;
const VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID: i32 = 134;
const VANILLA_ENTITY_TYPE_TRIDENT_ID: i32 = 135;
const VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID: i32 = 136;
const VANILLA_ENTITY_TYPE_TURTLE_ID: i32 = 137;
const VANILLA_ENTITY_TYPE_VEX_ID: i32 = 138;
const VANILLA_ENTITY_TYPE_VILLAGER_ID: i32 = 139;
const VANILLA_ENTITY_TYPE_VINDICATOR_ID: i32 = 140;
const VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID: i32 = 141;
const VANILLA_ENTITY_TYPE_WARDEN_ID: i32 = 142;
const VANILLA_ENTITY_TYPE_WIND_CHARGE_ID: i32 = 143;
const VANILLA_ENTITY_TYPE_WITCH_ID: i32 = 144;
const VANILLA_ENTITY_TYPE_WITHER_ID: i32 = 145;
const VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID: i32 = 146;
const VANILLA_ENTITY_TYPE_WITHER_SKULL_ID: i32 = 147;
const VANILLA_ENTITY_TYPE_WOLF_ID: i32 = 148;
const VANILLA_ENTITY_TYPE_ZOGLIN_ID: i32 = 149;
const VANILLA_ENTITY_TYPE_ZOMBIE_ID: i32 = 150;
const VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID: i32 = 151;
const VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID: i32 = 152;
const VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID: i32 = 153;
const VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID: i32 = 154;
const VANILLA_ENTITY_TYPE_PLAYER_ID: i32 = 155;
const VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID: i32 = 156;
const AGEABLE_MOB_BABY_DATA_ID: u8 = 16;
const ZOMBIE_BABY_DATA_ID: u8 = 16;
const PIGLIN_BABY_DATA_ID: u8 = 17;

pub(crate) fn entity_scene_outline_from_world_at_partial_tick(
    world: &WorldStore,
    entity_partial_tick: f32,
) -> Option<SelectionOutline> {
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    let boxes: Vec<_> = world
        .entity_pick_targets_at_partial_tick(entity_partial_tick.clamp(0.0, 1.0))
        .into_iter()
        .filter(|target| {
            local_player_id != Some(target.entity_id) && camera_entity_id != Some(target.entity_id)
        })
        .map(entity_pick_target_box)
        .collect();
    (!boxes.is_empty()).then(|| SelectionOutline::from_boxes(boxes))
}

pub(crate) fn entity_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    entity_partial_tick: f32,
) -> Vec<EntityModelInstance> {
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    world
        .entity_model_sources_at_partial_tick(entity_partial_tick.clamp(0.0, 1.0))
        .into_iter()
        .filter(|source| {
            local_player_id != Some(source.entity_id) && camera_entity_id != Some(source.entity_id)
        })
        .filter_map(entity_model_instance)
        .collect()
}

fn entity_pick_target_box(target: EntityPickTargetState) -> SelectionBox {
    SelectionBox {
        min: [
            (target.position.x + f64::from(target.bounds.min[0])) as f32,
            (target.position.y + f64::from(target.bounds.min[1])) as f32,
            (target.position.z + f64::from(target.bounds.min[2])) as f32,
        ],
        max: [
            (target.position.x + f64::from(target.bounds.max[0])) as f32,
            (target.position.y + f64::from(target.bounds.max[1])) as f32,
            (target.position.z + f64::from(target.bounds.max[2])) as f32,
        ],
    }
}

fn entity_model_instance(source: EntityModelSourceState) -> Option<EntityModelInstance> {
    let kind = entity_model_kind(source.entity_type_id, &source.data_values);
    Some(EntityModelInstance::new(
        source.entity_id,
        kind,
        [
            source.position.x as f32,
            source.position.y as f32,
            source.position.z as f32,
        ],
        source.y_rot,
    ))
}

fn entity_model_kind(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityModelKind {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHICKEN_ID => EntityModelKind::Chicken {
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_PLAYER_ID | VANILLA_ENTITY_TYPE_MANNEQUIN_ID => {
            humanoid(HumanoidModelFamily::Player, false)
        }
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID => humanoid(HumanoidModelFamily::ArmorStand, false),
        VANILLA_ENTITY_TYPE_ZOMBIE_ID => EntityModelKind::Zombie {
            baby: zombie_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_DROWNED_ID
        | VANILLA_ENTITY_TYPE_HUSK_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
        | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => {
            humanoid(HumanoidModelFamily::Zombie, zombie_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_PIGLIN_ID => {
            humanoid(HumanoidModelFamily::Zombie, piglin_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_SKELETON_ID => EntityModelKind::Skeleton,
        VANILLA_ENTITY_TYPE_STRAY_ID
        | VANILLA_ENTITY_TYPE_BOGGED_ID
        | VANILLA_ENTITY_TYPE_PARCHED_ID
        | VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID => humanoid(HumanoidModelFamily::Skeleton, false),
        VANILLA_ENTITY_TYPE_VILLAGER_ID | VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID => {
            humanoid(HumanoidModelFamily::Villager, ageable_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_EVOKER_ID
        | VANILLA_ENTITY_TYPE_ILLUSIONER_ID
        | VANILLA_ENTITY_TYPE_PILLAGER_ID
        | VANILLA_ENTITY_TYPE_VINDICATOR_ID
        | VANILLA_ENTITY_TYPE_WITCH_ID => humanoid(HumanoidModelFamily::Illager, false),
        VANILLA_ENTITY_TYPE_ENDERMAN_ID
        | VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID
        | VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
        | VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID => humanoid(HumanoidModelFamily::Player, false),
        VANILLA_ENTITY_TYPE_CREEPER_ID => EntityModelKind::Creeper,
        VANILLA_ENTITY_TYPE_PIG_ID => {
            quadruped(QuadrupedModelFamily::Pig, ageable_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_COW_ID => EntityModelKind::Cow {
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID
        | VANILLA_ENTITY_TYPE_GOAT_ID
        | VANILLA_ENTITY_TYPE_HOGLIN_ID
        | VANILLA_ENTITY_TYPE_ZOGLIN_ID
        | VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
        | VANILLA_ENTITY_TYPE_PANDA_ID
        | VANILLA_ENTITY_TYPE_SNIFFER_ID
        | VANILLA_ENTITY_TYPE_RAVAGER_ID => {
            quadruped(QuadrupedModelFamily::Cow, ageable_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_SHEEP_ID => EntityModelKind::Sheep {
            baby: ageable_baby(data_values),
        },
        VANILLA_ENTITY_TYPE_HORSE_ID
        | VANILLA_ENTITY_TYPE_DONKEY_ID
        | VANILLA_ENTITY_TYPE_MULE_ID
        | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
        | VANILLA_ENTITY_TYPE_LLAMA_ID
        | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
        | VANILLA_ENTITY_TYPE_CAMEL_ID
        | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
        | VANILLA_ENTITY_TYPE_NAUTILUS_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
            quadruped(QuadrupedModelFamily::Horse, ageable_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_WOLF_ID
        | VANILLA_ENTITY_TYPE_CAT_ID
        | VANILLA_ENTITY_TYPE_OCELOT_ID
        | VANILLA_ENTITY_TYPE_FOX_ID
        | VANILLA_ENTITY_TYPE_RABBIT_ID => {
            quadruped(QuadrupedModelFamily::Wolf, ageable_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_MINECART_ID
        | VANILLA_ENTITY_TYPE_CHEST_MINECART_ID
        | VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID
        | VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
        | VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID
        | VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID
        | VANILLA_ENTITY_TYPE_TNT_MINECART_ID => EntityModelKind::Minecart,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID => {
            placeholder("todo_area_effect_cloud_bounds", 1.0, 0.5, 1.0)
        }
        VANILLA_ENTITY_TYPE_ARROW_ID | VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID => {
            placeholder("todo_arrow_bounds", 0.5, 0.5, 0.5)
        }
        VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID => {
            placeholder("todo_block_display_bounds", 1.0, 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID => {
            placeholder("todo_dragon_fireball_bounds", 1.0, 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_EGG_ID | VANILLA_ENTITY_TYPE_SNOWBALL_ID => {
            placeholder("todo_thrown_item_projectile_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => {
            placeholder("todo_ender_dragon_bounds", 16.0, 8.0, 16.0)
        }
        VANILLA_ENTITY_TYPE_ENDER_PEARL_ID => {
            placeholder("todo_ender_pearl_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID
        | VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID
        | VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID
        | VANILLA_ENTITY_TYPE_CHERRY_BOAT_ID
        | VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID
        | VANILLA_ENTITY_TYPE_JUNGLE_BOAT_ID
        | VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID
        | VANILLA_ENTITY_TYPE_OAK_BOAT_ID
        | VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID
        | VANILLA_ENTITY_TYPE_SPRUCE_BOAT_ID => EntityModelKind::Boat { chest: false },
        VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID
        | VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID
        | VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID
        | VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID
        | VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID
        | VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID
        | VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID
        | VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
        | VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID
        | VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID => EntityModelKind::Boat { chest: true },
        VANILLA_ENTITY_TYPE_ALLAY_ID => placeholder("todo_allay_bounds", 0.35, 0.6, 0.35),
        VANILLA_ENTITY_TYPE_ARMADILLO_ID => placeholder("todo_armadillo_bounds", 0.7, 0.65, 0.7),
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => placeholder("todo_axolotl_bounds", 0.75, 0.42, 0.75),
        VANILLA_ENTITY_TYPE_BAT_ID => placeholder("todo_bat_bounds", 0.5, 0.9, 0.5),
        VANILLA_ENTITY_TYPE_BEE_ID => placeholder("todo_bee_bounds", 0.7, 0.6, 0.7),
        VANILLA_ENTITY_TYPE_BLAZE_ID => placeholder("todo_blaze_bounds", 0.6, 1.8, 0.6),
        VANILLA_ENTITY_TYPE_BREEZE_ID => placeholder("todo_breeze_bounds", 0.6, 1.77, 0.6),
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID => {
            placeholder("todo_breeze_wind_charge_bounds", 0.3125, 0.3125, 0.3125)
        }
        VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => placeholder("todo_cave_spider_bounds", 0.7, 0.5, 0.7),
        VANILLA_ENTITY_TYPE_COD_ID => placeholder("todo_cod_bounds", 0.5, 0.3, 0.5),
        VANILLA_ENTITY_TYPE_CREAKING_ID => placeholder("todo_creaking_bounds", 0.9, 2.7, 0.9),
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => placeholder("todo_dolphin_bounds", 0.9, 0.6, 0.9),
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => {
            placeholder("todo_elder_guardian_bounds", 1.9975, 1.9975, 1.9975)
        }
        VANILLA_ENTITY_TYPE_ENDERMITE_ID => placeholder("todo_endermite_bounds", 0.4, 0.3, 0.4),
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID => placeholder("todo_end_crystal_bounds", 2.0, 2.0, 2.0),
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID => {
            placeholder("todo_evoker_fangs_bounds", 0.5, 0.8, 0.5)
        }
        VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID
        | VANILLA_ENTITY_TYPE_SPLASH_POTION_ID
        | VANILLA_ENTITY_TYPE_LINGERING_POTION_ID => {
            placeholder("todo_thrown_bottle_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID => {
            placeholder("todo_experience_orb_bounds", 0.5, 0.5, 0.5)
        }
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID => {
            placeholder("todo_eye_of_ender_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID => {
            placeholder("todo_falling_block_bounds", 0.98, 0.98, 0.98)
        }
        VANILLA_ENTITY_TYPE_FIREBALL_ID => placeholder("todo_fireball_bounds", 1.0, 1.0, 1.0),
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID => {
            placeholder("todo_firework_rocket_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID => {
            placeholder("todo_fishing_bobber_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_FROG_ID => placeholder("todo_frog_bounds", 0.5, 0.5, 0.5),
        VANILLA_ENTITY_TYPE_GHAST_ID => placeholder("todo_ghast_bounds", 4.0, 4.0, 4.0),
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => placeholder("todo_happy_ghast_bounds", 4.0, 4.0, 4.0),
        VANILLA_ENTITY_TYPE_GIANT_ID => placeholder("todo_giant_bounds", 3.6, 12.0, 3.6),
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID => {
            placeholder("todo_glow_item_frame_bounds", 0.75, 0.75, 0.0625)
        }
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => placeholder("todo_glow_squid_bounds", 0.8, 0.8, 0.8),
        VANILLA_ENTITY_TYPE_GUARDIAN_ID => placeholder("todo_guardian_bounds", 0.85, 0.85, 0.85),
        VANILLA_ENTITY_TYPE_INTERACTION_ID => placeholder("todo_interaction_bounds", 1.0, 1.0, 1.0),
        VANILLA_ENTITY_TYPE_ITEM_ID => placeholder("todo_item_entity_bounds", 0.25, 0.25, 0.25),
        VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID => {
            placeholder("todo_item_display_bounds", 1.0, 1.0, 1.0)
        }
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID => {
            placeholder("todo_item_frame_bounds", 0.75, 0.75, 0.0625)
        }
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID => {
            placeholder("todo_leash_knot_bounds", 0.375, 0.5, 0.375)
        }
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID => {
            placeholder("todo_lightning_bolt_bounds", 0.5, 2.0, 0.5)
        }
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID => {
            placeholder("todo_llama_spit_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => {
            placeholder("todo_magma_cube_bounds", 0.52, 0.52, 0.52)
        }
        VANILLA_ENTITY_TYPE_MARKER_ID => placeholder("todo_marker_bounds", 0.0625, 0.0625, 0.0625),
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID => {
            placeholder("todo_ominous_item_spawner_bounds", 0.25, 0.25, 0.25)
        }
        VANILLA_ENTITY_TYPE_PAINTING_ID => placeholder("todo_painting_bounds", 1.0, 1.0, 0.0625),
        VANILLA_ENTITY_TYPE_PARROT_ID => placeholder("todo_parrot_bounds", 0.5, 0.9, 0.5),
        VANILLA_ENTITY_TYPE_PHANTOM_ID => placeholder("todo_phantom_bounds", 0.9, 0.5, 0.9),
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => humanoid(HumanoidModelFamily::Zombie, false),
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID => placeholder("todo_pufferfish_bounds", 0.7, 0.7, 0.7),
        VANILLA_ENTITY_TYPE_SALMON_ID => placeholder("todo_salmon_bounds", 0.7, 0.4, 0.7),
        VANILLA_ENTITY_TYPE_SHULKER_ID => placeholder("todo_shulker_bounds", 1.0, 1.0, 1.0),
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID => {
            placeholder("todo_shulker_bullet_bounds", 0.3125, 0.3125, 0.3125)
        }
        VANILLA_ENTITY_TYPE_SILVERFISH_ID => placeholder("todo_silverfish_bounds", 0.4, 0.3, 0.4),
        VANILLA_ENTITY_TYPE_SLIME_ID => placeholder("todo_slime_bounds", 0.52, 0.52, 0.52),
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID => {
            placeholder("todo_small_fireball_bounds", 0.3125, 0.3125, 0.3125)
        }
        VANILLA_ENTITY_TYPE_SPIDER_ID => EntityModelKind::Spider,
        VANILLA_ENTITY_TYPE_SQUID_ID => placeholder("todo_squid_bounds", 0.8, 0.8, 0.8),
        VANILLA_ENTITY_TYPE_STRIDER_ID => {
            quadruped(QuadrupedModelFamily::Horse, ageable_baby(data_values))
        }
        VANILLA_ENTITY_TYPE_TADPOLE_ID => placeholder("todo_tadpole_bounds", 0.4, 0.3, 0.4),
        VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID => {
            placeholder("todo_text_display_bounds", 1.0, 0.5, 0.0625)
        }
        VANILLA_ENTITY_TYPE_TNT_ID => placeholder("todo_tnt_bounds", 0.98, 0.98, 0.98),
        VANILLA_ENTITY_TYPE_TRIDENT_ID => placeholder("todo_trident_bounds", 0.5, 0.5, 0.5),
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => {
            placeholder("todo_tropical_fish_bounds", 0.5, 0.4, 0.5)
        }
        VANILLA_ENTITY_TYPE_TURTLE_ID => placeholder("todo_turtle_bounds", 1.2, 0.4, 1.2),
        VANILLA_ENTITY_TYPE_VEX_ID => placeholder("todo_vex_bounds", 0.4, 0.8, 0.4),
        VANILLA_ENTITY_TYPE_WARDEN_ID => placeholder("todo_warden_bounds", 0.9, 2.9, 0.9),
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID => {
            placeholder("todo_wind_charge_bounds", 0.3125, 0.3125, 0.3125)
        }
        VANILLA_ENTITY_TYPE_WITHER_ID => placeholder("todo_wither_bounds", 0.9, 3.5, 0.9),
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID => {
            placeholder("todo_wither_skull_bounds", 0.3125, 0.3125, 0.3125)
        }
        _ => placeholder("todo_unknown_entity_type_bounds", 0.75, 0.75, 0.75),
    }
}

fn humanoid(family: HumanoidModelFamily, baby: bool) -> EntityModelKind {
    EntityModelKind::Humanoid { family, baby }
}

fn quadruped(family: QuadrupedModelFamily, baby: bool) -> EntityModelKind {
    EntityModelKind::Quadruped { family, baby }
}

fn placeholder(name: &'static str, width: f32, height: f32, depth: f32) -> EntityModelKind {
    EntityModelKind::Placeholder {
        name,
        bounds: bbb_renderer::EntityModelBounds {
            width,
            height,
            depth,
        },
    }
}

fn ageable_baby(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_data_bool(values, AGEABLE_MOB_BABY_DATA_ID, false)
}

fn zombie_baby(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_data_bool(values, ZOMBIE_BABY_DATA_ID, false)
}

fn piglin_baby(values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    entity_data_bool(values, PIGLIN_BABY_DATA_ID, false)
}

fn entity_data_bool(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: bool,
) -> bool {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        AddEntity, CommonPlayerSpawnInfo, EntityDataValue, PlayLogin, SetCamera, SetEntityData,
        Vec3d,
    };
    use bbb_world::{EntityPickBoundsState, EntityVec3};
    use uuid::Uuid;

    const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;

    #[test]
    fn entity_scene_outline_is_none_without_visible_entity_targets() {
        assert_eq!(
            entity_scene_outline_from_world_at_partial_tick(&WorldStore::new(), 1.0),
            None
        );
    }

    #[test]
    fn entity_scene_outline_projects_pick_bounds_for_all_visible_targets() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            11,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [2.0, 1.0, 3.0],
        ));

        let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.5)
            .expect("expected entity scene outline");

        assert_eq!(outline.boxes.len(), 2);
        assert_selection_box_close(outline.boxes[0].min, [-0.49, 1.0, 2.51]);
        assert_selection_box_close(outline.boxes[0].max, [0.49, 1.7, 3.49]);
        assert_selection_box_close(outline.boxes[1].min, [1.51, 1.0, 2.51]);
        assert_selection_box_close(outline.boxes[1].max, [2.49, 1.7, 3.49]);
    }

    #[test]
    fn entity_scene_outline_uses_bounds_without_pick_radius_inflation() {
        let outline_box = entity_pick_target_box(EntityPickTargetState {
            entity_id: 7,
            position: EntityVec3 {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
            bounds: EntityPickBoundsState::from_centered_size(2.0, 4.0, 6.0, 1.5),
        });

        assert_selection_box_close(outline_box.min, [9.0, 18.0, 27.0]);
        assert_selection_box_close(outline_box.max, [11.0, 22.0, 33.0]);
    }

    #[test]
    fn entity_scene_outline_filters_local_player_and_camera_entity() {
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(10));
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            11,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [2.0, 1.0, 3.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [4.0, 1.0, 3.0],
        ));
        assert!(world.apply_set_camera(SetCamera { camera_id: 11 }));

        let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
            .expect("expected non-camera entity scene outline");

        assert_eq!(outline.boxes.len(), 1);
        assert_selection_box_close(outline.boxes[0].min, [3.51, 1.0, 2.51]);
        assert_selection_box_close(outline.boxes[0].max, [4.49, 1.7, 3.49]);
    }

    #[test]
    fn entity_model_instances_project_chicken_adult_and_baby_models() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            26,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            27,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [3.0, 64.0, -2.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            85,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [5.0, 64.0, -2.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 27,
            values: vec![protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)],
        }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

        assert_eq!(
            instances,
            vec![
                EntityModelInstance::chicken(26, [1.0, 64.0, -2.0], 0.0, false),
                EntityModelInstance::chicken(27, [3.0, 64.0, -2.0], 0.0, true),
                EntityModelInstance::new(85, EntityModelKind::Minecart, [5.0, 64.0, -2.0], 0.0),
            ]
        );
    }

    #[test]
    fn entity_model_instances_do_not_drop_pick_target_entity_types() {
        let mut world = WorldStore::new();
        for (index, entity_type_id) in VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS
            .iter()
            .copied()
            .enumerate()
        {
            world.apply_add_entity(protocol_add_entity(
                1000 + index as i32,
                entity_type_id,
                [index as f64 * 2.0, 64.0, 0.0],
            ));
        }

        let sources = world.entity_model_sources_at_partial_tick(1.0);
        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

        assert_eq!(
            sources.len(),
            VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS.len()
        );
        assert_eq!(instances.len(), sources.len());
    }

    #[test]
    fn entity_model_kind_maps_all_vanilla_registry_ids() {
        for entity_type_id in 0..=VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID {
            let kind = entity_model_kind(entity_type_id, &[]);
            assert!(
                placeholder_name(kind) != Some("todo_unknown_entity_type_bounds"),
                "vanilla type id {entity_type_id} fell through to unknown renderer entity model fallback"
            );
        }
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_base_zombie_and_skeleton() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_ID, &[]),
            EntityModelKind::Zombie { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                &[protocol_bool_data(ZOMBIE_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Zombie { baby: true }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SKELETON_ID, &[]),
            EntityModelKind::Skeleton
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_HUSK_ID, &[]),
            humanoid(HumanoidModelFamily::Zombie, false)
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_STRAY_ID, &[]),
            humanoid(HumanoidModelFamily::Skeleton, false)
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_models_for_base_cow_and_sheep() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[]),
            EntityModelKind::Cow { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_COW_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Cow { baby: true }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SHEEP_ID, &[]),
            EntityModelKind::Sheep { baby: false }
        );
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
                &[protocol_bool_data(AGEABLE_MOB_BABY_DATA_ID, true)]
            ),
            EntityModelKind::Sheep { baby: true }
        );
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, &[]),
            quadruped(QuadrupedModelFamily::Cow, false)
        );
    }

    #[test]
    fn entity_model_kind_uses_exact_model_for_base_spider() {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_SPIDER_ID, &[]),
            EntityModelKind::Spider
        );
        assert_eq!(
            placeholder_name(entity_model_kind(VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, &[])),
            Some("todo_cave_spider_bounds")
        );
    }

    #[test]
    fn entity_model_kind_uses_explicit_unknown_future_fallback() {
        let kind = entity_model_kind(9999, &[]);

        assert_eq!(
            placeholder_name(kind),
            Some("todo_unknown_entity_type_bounds")
        );
    }

    #[test]
    fn entity_model_instances_filter_local_player_and_camera_entity() {
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(10));
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [0.0, 64.0, 0.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            11,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [1.0, 64.0, 0.0],
        ));
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            [2.0, 64.0, 0.0],
        ));
        assert!(world.apply_set_camera(SetCamera { camera_id: 11 }));

        let instances = entity_model_instances_from_world_at_partial_tick(&world, 1.0);

        assert_eq!(
            instances,
            vec![EntityModelInstance::chicken(
                12,
                [2.0, 64.0, 0.0],
                0.0,
                false
            )]
        );
    }

    fn protocol_add_entity(id: i32, entity_type_id: i32, position: [f64; 3]) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678 + id as u128),
            entity_type_id,
            position: Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn protocol_bool_data(data_id: u8, value: bool) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 8,
            value: EntityDataValueKind::Boolean(value),
        }
    }

    fn protocol_play_login(player_id: i32) -> PlayLogin {
        PlayLogin {
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
        }
    }

    fn assert_selection_box_close(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "expected {expected}, got {actual}"
            );
        }
    }

    const VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS: &[i32] = &[
        0, 1, 2, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
        28, 29, 30, 31, 32, 33, 34, 35, 36, 38, 40, 41, 42, 45, 46, 51, 52, 54, 55, 56, 57, 58, 59,
        61, 62, 63, 64, 65, 66, 67, 68, 70, 74, 75, 78, 80, 81, 82, 83, 85, 86, 87, 88, 89, 90, 91,
        94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 107, 108, 109, 110, 111, 112, 113, 114,
        115, 116, 117, 119, 121, 122, 124, 125, 126, 127, 128, 129, 130, 132, 133, 134, 136, 137,
        138, 139, 140, 141, 142, 143, 144, 145, 146, 148, 149, 150, 151, 152, 153, 154, 155,
    ];

    fn placeholder_name(kind: EntityModelKind) -> Option<&'static str> {
        match kind {
            EntityModelKind::Placeholder { name, .. } => Some(name),
            _ => None,
        }
    }
}
