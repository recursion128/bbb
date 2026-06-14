use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityPickBoundsState {
    pub width: f32,
    pub height: f32,
    pub pick_radius: f32,
}

pub(crate) fn vanilla_pick_bounds_for_type(entity_type_id: i32) -> Option<EntityPickBoundsState> {
    VANILLA_ENTITY_PICK_BOUNDS
        .binary_search_by_key(&entity_type_id, |(id, _)| *id)
        .ok()
        .map(|index| VANILLA_ENTITY_PICK_BOUNDS[index].1)
}

const fn pick(width: f32, height: f32, pick_radius: f32) -> EntityPickBoundsState {
    EntityPickBoundsState {
        width,
        height,
        pick_radius,
    }
}

// IDs are the vanilla 26.1 EntityType registry order from EntityType.java.
const VANILLA_ENTITY_PICK_BOUNDS: &[(i32, EntityPickBoundsState)] = &[
    (0, pick(1.375, 0.5625, 0.0)),    // minecraft:acacia_boat
    (1, pick(1.375, 0.5625, 0.0)),    // minecraft:acacia_chest_boat
    (2, pick(0.35, 0.6, 0.0)),        // minecraft:allay
    (4, pick(0.7, 0.65, 0.0)),        // minecraft:armadillo
    (5, pick(0.5, 1.975, 0.0)),       // minecraft:armor_stand
    (7, pick(0.75, 0.42, 0.0)),       // minecraft:axolotl
    (8, pick(1.375, 0.5625, 0.0)),    // minecraft:bamboo_chest_raft
    (9, pick(1.375, 0.5625, 0.0)),    // minecraft:bamboo_raft
    (10, pick(0.5, 0.9, 0.0)),        // minecraft:bat
    (11, pick(0.7, 0.6, 0.0)),        // minecraft:bee
    (12, pick(1.375, 0.5625, 0.0)),   // minecraft:birch_boat
    (13, pick(1.375, 0.5625, 0.0)),   // minecraft:birch_chest_boat
    (14, pick(0.6, 1.8, 0.0)),        // minecraft:blaze
    (16, pick(0.6, 1.99, 0.0)),       // minecraft:bogged
    (17, pick(0.6, 1.77, 0.0)),       // minecraft:breeze
    (18, pick(0.3125, 0.3125, 1.0)),  // minecraft:breeze_wind_charge
    (19, pick(1.7, 2.375, 0.0)),      // minecraft:camel
    (20, pick(1.7, 2.375, 0.0)),      // minecraft:camel_husk
    (21, pick(0.6, 0.7, 0.0)),        // minecraft:cat
    (22, pick(0.7, 0.5, 0.0)),        // minecraft:cave_spider
    (23, pick(1.375, 0.5625, 0.0)),   // minecraft:cherry_boat
    (24, pick(1.375, 0.5625, 0.0)),   // minecraft:cherry_chest_boat
    (25, pick(0.98, 0.7, 0.0)),       // minecraft:chest_minecart
    (26, pick(0.4, 0.7, 0.0)),        // minecraft:chicken
    (27, pick(0.5, 0.3, 0.0)),        // minecraft:cod
    (28, pick(0.49, 0.98, 0.0)),      // minecraft:copper_golem
    (29, pick(0.98, 0.7, 0.0)),       // minecraft:command_block_minecart
    (30, pick(0.9, 1.4, 0.0)),        // minecraft:cow
    (31, pick(0.9, 2.7, 0.0)),        // minecraft:creaking
    (32, pick(0.6, 1.7, 0.0)),        // minecraft:creeper
    (33, pick(1.375, 0.5625, 0.0)),   // minecraft:dark_oak_boat
    (34, pick(1.375, 0.5625, 0.0)),   // minecraft:dark_oak_chest_boat
    (35, pick(0.9, 0.6, 0.0)),        // minecraft:dolphin
    (36, pick(1.3964844, 1.5, 0.0)),  // minecraft:donkey
    (38, pick(0.6, 1.95, 0.0)),       // minecraft:drowned
    (40, pick(1.9975, 1.9975, 0.0)),  // minecraft:elder_guardian
    (41, pick(0.6, 2.9, 0.0)),        // minecraft:enderman
    (42, pick(0.4, 0.3, 0.0)),        // minecraft:endermite
    (45, pick(2.0, 2.0, 0.0)),        // minecraft:end_crystal
    (46, pick(0.6, 1.95, 0.0)),       // minecraft:evoker
    (51, pick(0.98, 0.98, 0.0)),      // minecraft:falling_block
    (52, pick(1.0, 1.0, 1.0)),        // minecraft:fireball
    (54, pick(0.6, 0.7, 0.0)),        // minecraft:fox
    (55, pick(0.5, 0.5, 0.0)),        // minecraft:frog
    (56, pick(0.98, 0.7, 0.0)),       // minecraft:furnace_minecart
    (57, pick(4.0, 4.0, 0.0)),        // minecraft:ghast
    (58, pick(4.0, 4.0, 0.0)),        // minecraft:happy_ghast
    (59, pick(3.6, 12.0, 0.0)),       // minecraft:giant
    (61, pick(0.8, 0.8, 0.0)),        // minecraft:glow_squid
    (62, pick(0.9, 1.3, 0.0)),        // minecraft:goat
    (63, pick(0.85, 0.85, 0.0)),      // minecraft:guardian
    (64, pick(1.3964844, 1.4, 0.0)),  // minecraft:hoglin
    (65, pick(0.98, 0.7, 0.0)),       // minecraft:hopper_minecart
    (66, pick(1.3964844, 1.6, 0.0)),  // minecraft:horse
    (67, pick(0.6, 1.95, 0.0)),       // minecraft:husk
    (68, pick(0.6, 1.95, 0.0)),       // minecraft:illusioner
    (70, pick(1.4, 2.7, 0.0)),        // minecraft:iron_golem
    (74, pick(1.375, 0.5625, 0.0)),   // minecraft:jungle_boat
    (75, pick(1.375, 0.5625, 0.0)),   // minecraft:jungle_chest_boat
    (78, pick(0.9, 1.87, 0.0)),       // minecraft:llama
    (80, pick(0.52, 0.52, 0.0)),      // minecraft:magma_cube
    (81, pick(1.375, 0.5625, 0.0)),   // minecraft:mangrove_boat
    (82, pick(1.375, 0.5625, 0.0)),   // minecraft:mangrove_chest_boat
    (83, pick(0.6, 1.8, 0.0)),        // minecraft:mannequin
    (85, pick(0.98, 0.7, 0.0)),       // minecraft:minecart
    (86, pick(0.9, 1.4, 0.0)),        // minecraft:mooshroom
    (87, pick(1.3964844, 1.6, 0.0)),  // minecraft:mule
    (88, pick(0.875, 0.95, 0.0)),     // minecraft:nautilus
    (89, pick(1.375, 0.5625, 0.0)),   // minecraft:oak_boat
    (90, pick(1.375, 0.5625, 0.0)),   // minecraft:oak_chest_boat
    (91, pick(0.6, 0.7, 0.0)),        // minecraft:ocelot
    (94, pick(1.375, 0.5625, 0.0)),   // minecraft:pale_oak_boat
    (95, pick(1.375, 0.5625, 0.0)),   // minecraft:pale_oak_chest_boat
    (96, pick(1.3, 1.25, 0.0)),       // minecraft:panda
    (97, pick(0.6, 1.99, 0.0)),       // minecraft:parched
    (98, pick(0.5, 0.9, 0.0)),        // minecraft:parrot
    (99, pick(0.9, 0.5, 0.0)),        // minecraft:phantom
    (100, pick(0.9, 0.9, 0.0)),       // minecraft:pig
    (101, pick(0.6, 1.95, 0.0)),      // minecraft:piglin
    (102, pick(0.6, 1.95, 0.0)),      // minecraft:piglin_brute
    (103, pick(0.6, 1.95, 0.0)),      // minecraft:pillager
    (104, pick(1.4, 1.4, 0.0)),       // minecraft:polar_bear
    (107, pick(0.7, 0.7, 0.0)),       // minecraft:pufferfish
    (108, pick(0.49, 0.6, 0.0)),      // minecraft:rabbit
    (109, pick(1.95, 2.2, 0.0)),      // minecraft:ravager
    (110, pick(0.7, 0.4, 0.0)),       // minecraft:salmon
    (111, pick(0.9, 1.3, 0.0)),       // minecraft:sheep
    (112, pick(1.0, 1.0, 0.0)),       // minecraft:shulker
    (113, pick(0.3125, 0.3125, 1.0)), // minecraft:shulker_bullet
    (114, pick(0.4, 0.3, 0.0)),       // minecraft:silverfish
    (115, pick(0.6, 1.99, 0.0)),      // minecraft:skeleton
    (116, pick(1.3964844, 1.6, 0.0)), // minecraft:skeleton_horse
    (117, pick(0.52, 0.52, 0.0)),     // minecraft:slime
    (119, pick(1.9, 1.75, 0.0)),      // minecraft:sniffer
    (121, pick(0.7, 1.9, 0.0)),       // minecraft:snow_golem
    (122, pick(0.98, 0.7, 0.0)),      // minecraft:spawner_minecart
    (124, pick(1.4, 0.9, 0.0)),       // minecraft:spider
    (125, pick(1.375, 0.5625, 0.0)),  // minecraft:spruce_boat
    (126, pick(1.375, 0.5625, 0.0)),  // minecraft:spruce_chest_boat
    (127, pick(0.8, 0.8, 0.0)),       // minecraft:squid
    (128, pick(0.6, 1.99, 0.0)),      // minecraft:stray
    (129, pick(0.9, 1.7, 0.0)),       // minecraft:strider
    (130, pick(0.4, 0.3, 0.0)),       // minecraft:tadpole
    (132, pick(0.98, 0.98, 0.0)),     // minecraft:tnt
    (133, pick(0.98, 0.7, 0.0)),      // minecraft:tnt_minecart
    (134, pick(0.9, 1.87, 0.0)),      // minecraft:trader_llama
    (136, pick(0.5, 0.4, 0.0)),       // minecraft:tropical_fish
    (137, pick(1.2, 0.4, 0.0)),       // minecraft:turtle
    (138, pick(0.4, 0.8, 0.0)),       // minecraft:vex
    (139, pick(0.6, 1.95, 0.0)),      // minecraft:villager
    (140, pick(0.6, 1.95, 0.0)),      // minecraft:vindicator
    (141, pick(0.6, 1.95, 0.0)),      // minecraft:wandering_trader
    (142, pick(0.9, 2.9, 0.0)),       // minecraft:warden
    (143, pick(0.3125, 0.3125, 1.0)), // minecraft:wind_charge
    (144, pick(0.6, 1.95, 0.0)),      // minecraft:witch
    (145, pick(0.9, 3.5, 0.0)),       // minecraft:wither
    (146, pick(0.7, 2.4, 0.0)),       // minecraft:wither_skeleton
    (148, pick(0.6, 0.85, 0.0)),      // minecraft:wolf
    (149, pick(1.3964844, 1.4, 0.0)), // minecraft:zoglin
    (150, pick(0.6, 1.95, 0.0)),      // minecraft:zombie
    (151, pick(1.3964844, 1.6, 0.0)), // minecraft:zombie_horse
    (152, pick(0.875, 0.95, 0.0)),    // minecraft:zombie_nautilus
    (153, pick(0.6, 1.95, 0.0)),      // minecraft:zombie_villager
    (154, pick(0.6, 1.95, 0.0)),      // minecraft:zombified_piglin
    (155, pick(0.6, 1.8, 0.0)),       // minecraft:player
];
