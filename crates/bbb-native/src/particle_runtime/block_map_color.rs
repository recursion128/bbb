use super::*;

pub(super) fn falling_dust_provider_accepts_block_state(block_state_id: i32) -> bool {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let Some(block_state) = block_states.by_id(block_state_id) else {
        return true;
    };
    !falling_dust_provider_rejects_block_name(&block_state.name)
}

fn falling_dust_provider_rejects_block_name(name: &str) -> bool {
    !matches!(
        name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    ) && block_name_has_invisible_render_shape(name)
}

pub(super) fn falling_dust_color_for_block_state_id(block_state_id: i32) -> Option<[f32; 4]> {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let block_state = block_states.by_id(block_state_id)?;
    falling_dust_color_for_block_name(&block_state.name).map(rgb_particle_color_u32)
}

pub(super) fn falling_dust_map_color_for_block_state_id(block_state_id: i32) -> Option<[f32; 4]> {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let block_state = block_states.by_id(block_state_id)?;
    vanilla_static_map_color_for_block_state(&block_state.name, &block_state.properties)
        .map(rgb_particle_color_u32)
}

fn falling_dust_color_for_block_name(name: &str) -> Option<u32> {
    match name {
        // Vanilla FallingDustParticle.Provider uses FallingBlock#getDustColor first.
        "minecraft:sand" => Some(0xDB_D3_A0),
        "minecraft:red_sand" => Some(0xA9_58_21),
        "minecraft:gravel" => Some(0x80_7C_7B),
        "minecraft:dragon_egg" => Some(0x00_00_00),
        "minecraft:anvil" | "minecraft:chipped_anvil" | "minecraft:damaged_anvil" => {
            Some(MAP_COLOR_METAL)
        }
        name => concrete_powder_map_color(name),
    }
}

fn concrete_powder_map_color(name: &str) -> Option<u32> {
    let color = name
        .strip_prefix("minecraft:")?
        .strip_suffix("_concrete_powder")?;
    dye_color_map_color(color)
}

fn dye_color_map_color(color: &str) -> Option<u32> {
    Some(match color {
        "white" => MAP_COLOR_SNOW,
        "orange" => MAP_COLOR_ORANGE,
        "magenta" => MAP_COLOR_MAGENTA,
        "light_blue" => MAP_COLOR_LIGHT_BLUE,
        "yellow" => MAP_COLOR_YELLOW,
        "lime" => MAP_COLOR_LIGHT_GREEN,
        "pink" => MAP_COLOR_PINK,
        "gray" => MAP_COLOR_GRAY,
        "light_gray" => MAP_COLOR_LIGHT_GRAY,
        "cyan" => MAP_COLOR_CYAN,
        "purple" => MAP_COLOR_PURPLE,
        "blue" => MAP_COLOR_BLUE,
        "brown" => MAP_COLOR_BROWN,
        "green" => MAP_COLOR_GREEN,
        "red" => MAP_COLOR_RED,
        "black" => MAP_COLOR_BLACK,
        _ => return None,
    })
}

fn terracotta_map_color(color: &str) -> Option<u32> {
    Some(match color {
        "white" => MAP_COLOR_TERRACOTTA_WHITE,
        "orange" => MAP_COLOR_TERRACOTTA_ORANGE,
        "magenta" => MAP_COLOR_TERRACOTTA_MAGENTA,
        "light_blue" => MAP_COLOR_TERRACOTTA_LIGHT_BLUE,
        "yellow" => MAP_COLOR_TERRACOTTA_YELLOW,
        "lime" => MAP_COLOR_TERRACOTTA_LIGHT_GREEN,
        "pink" => MAP_COLOR_TERRACOTTA_PINK,
        "gray" => MAP_COLOR_TERRACOTTA_GRAY,
        "light_gray" => MAP_COLOR_TERRACOTTA_LIGHT_GRAY,
        "cyan" => MAP_COLOR_TERRACOTTA_CYAN,
        "purple" => MAP_COLOR_TERRACOTTA_PURPLE,
        "blue" => MAP_COLOR_TERRACOTTA_BLUE,
        "brown" => MAP_COLOR_TERRACOTTA_BROWN,
        "green" => MAP_COLOR_TERRACOTTA_GREEN,
        "red" => MAP_COLOR_TERRACOTTA_RED,
        "black" => MAP_COLOR_TERRACOTTA_BLACK,
        _ => return None,
    })
}

fn colored_family_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    for suffix in [
        "_wool",
        "_carpet",
        "_concrete",
        "_stained_glass",
        "_stained_glass_pane",
        "_glazed_terracotta",
    ] {
        if let Some(color) = name.strip_suffix(suffix) {
            return dye_color_map_color(color);
        }
    }
    if let Some(color) = name.strip_suffix("_terracotta") {
        return terracotta_map_color(color);
    }
    None
}

fn banner_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let color = name
        .strip_suffix("_wall_banner")
        .or_else(|| name.strip_suffix("_banner"))?;
    dye_color_map_color(color).map(|_| MAP_COLOR_WOOD)
}

fn candle_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if name == "candle" {
        return Some(MAP_COLOR_SAND);
    }
    let color = name.strip_suffix("_candle")?;
    if color == "white" {
        Some(MAP_COLOR_WOOL)
    } else {
        dye_color_map_color(color)
    }
}

fn shulker_box_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if name == "shulker_box" {
        return Some(MAP_COLOR_PURPLE);
    }
    let color = name.strip_suffix("_shulker_box")?;
    if color == "purple" {
        Some(MAP_COLOR_TERRACOTTA_PURPLE)
    } else {
        dye_color_map_color(color)
    }
}

fn bed_map_color(
    name: &str,
    properties: &std::collections::BTreeMap<String, String>,
) -> Option<u32> {
    let color = name.strip_prefix("minecraft:")?.strip_suffix("_bed")?;
    match properties.get("part").map(String::as_str) {
        Some("head") => Some(MAP_COLOR_WOOL),
        Some("foot") => dye_color_map_color(color),
        _ => None,
    }
}

fn ore_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    match name {
        "nether_gold_ore" | "nether_quartz_ore" => return Some(MAP_COLOR_NETHER),
        _ => {}
    }
    if name
        .strip_prefix("deepslate_")
        .is_some_and(|ore| ore.ends_with("_ore"))
    {
        return Some(MAP_COLOR_DEEPSLATE);
    }
    if name.ends_with("_ore") {
        return Some(MAP_COLOR_STONE);
    }
    None
}

fn deepslate_family_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if name == "deepslate"
        || name == "cobbled_deepslate"
        || name == "cobbled_deepslate_stairs"
        || name == "cobbled_deepslate_slab"
        || name == "cobbled_deepslate_wall"
        || name == "polished_deepslate"
        || name == "polished_deepslate_stairs"
        || name == "polished_deepslate_slab"
        || name == "polished_deepslate_wall"
        || name == "deepslate_tiles"
        || name == "deepslate_tile_stairs"
        || name == "deepslate_tile_slab"
        || name == "deepslate_tile_wall"
        || name == "deepslate_bricks"
        || name == "deepslate_brick_stairs"
        || name == "deepslate_brick_slab"
        || name == "deepslate_brick_wall"
        || name == "chiseled_deepslate"
        || name == "cracked_deepslate_bricks"
        || name == "cracked_deepslate_tiles"
        || name == "infested_deepslate"
        || name == "reinforced_deepslate"
    {
        return Some(MAP_COLOR_DEEPSLATE);
    }
    None
}

fn copper_weathering_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if name == "raw_copper_block" {
        return Some(MAP_COLOR_ORANGE);
    }
    let name = name.strip_prefix("waxed_").unwrap_or(name);
    if let Some(rest) = name.strip_prefix("exposed_") {
        return copper_weathering_base_suffix(rest).then_some(MAP_COLOR_TERRACOTTA_LIGHT_GRAY);
    }
    if let Some(rest) = name.strip_prefix("weathered_") {
        return copper_weathering_base_suffix(rest).then_some(MAP_COLOR_WARPED_STEM);
    }
    if let Some(rest) = name.strip_prefix("oxidized_") {
        return copper_weathering_base_suffix(rest).then_some(MAP_COLOR_WARPED_NYLIUM);
    }
    copper_weathering_base_suffix(name).then_some(MAP_COLOR_ORANGE)
}

fn copper_weathering_base_suffix(name: &str) -> bool {
    matches!(
        name,
        "copper"
            | "copper_block"
            | "cut_copper"
            | "cut_copper_stairs"
            | "cut_copper_slab"
            | "chiseled_copper"
            | "copper_door"
            | "copper_trapdoor"
            | "copper_grate"
            | "copper_bulb"
            | "copper_chest"
            | "copper_golem_statue"
            | "lightning_rod"
    )
}

fn copper_lantern_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let name = name.strip_prefix("waxed_").unwrap_or(name);
    let name = name
        .strip_prefix("exposed_")
        .or_else(|| name.strip_prefix("weathered_"))
        .or_else(|| name.strip_prefix("oxidized_"))
        .unwrap_or(name);
    (name == "copper_lantern").then_some(MAP_COLOR_METAL)
}

fn wooden_stairs_and_slabs_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let family = name
        .strip_suffix("_stairs")
        .or_else(|| name.strip_suffix("_slab"))?;
    wooden_plank_family_map_color(family)
}

fn wooden_pressure_plate_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let family = name.strip_suffix("_pressure_plate")?;
    wooden_plank_family_map_color(family)
}

fn wooden_door_trapdoor_fence_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let family = name
        .strip_suffix("_fence_gate")
        .or_else(|| name.strip_suffix("_trapdoor"))
        .or_else(|| name.strip_suffix("_door"))
        .or_else(|| name.strip_suffix("_fence"))?;
    wooden_plank_family_map_color(family)
}

fn button_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if matches!(name, "stone_button" | "polished_blackstone_button") {
        return Some(MAP_COLOR_NONE);
    }
    let family = name.strip_suffix("_button")?;
    wooden_plank_family_map_color(family).map(|_| MAP_COLOR_NONE)
}

fn potted_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "flower_pot"
        | "potted_torchflower"
        | "potted_oak_sapling"
        | "potted_spruce_sapling"
        | "potted_birch_sapling"
        | "potted_jungle_sapling"
        | "potted_acacia_sapling"
        | "potted_cherry_sapling"
        | "potted_dark_oak_sapling"
        | "potted_pale_oak_sapling"
        | "potted_mangrove_propagule"
        | "potted_dandelion"
        | "potted_golden_dandelion"
        | "potted_poppy"
        | "potted_blue_orchid"
        | "potted_allium"
        | "potted_azure_bluet"
        | "potted_red_tulip"
        | "potted_orange_tulip"
        | "potted_white_tulip"
        | "potted_pink_tulip"
        | "potted_oxeye_daisy"
        | "potted_cornflower"
        | "potted_lily_of_the_valley"
        | "potted_wither_rose"
        | "potted_red_mushroom"
        | "potted_brown_mushroom"
        | "potted_dead_bush"
        | "potted_cactus"
        | "potted_bamboo"
        | "potted_crimson_fungus"
        | "potted_warped_fungus"
        | "potted_crimson_roots"
        | "potted_warped_roots"
        | "potted_azalea_bush"
        | "potted_flowering_azalea_bush"
        | "potted_open_eyeblossom"
        | "potted_closed_eyeblossom" => MAP_COLOR_NONE,
        _ => return None,
    })
}

fn cake_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if matches!(name, "cake" | "candle_cake") {
        return Some(MAP_COLOR_NONE);
    }
    let color = name.strip_suffix("_candle_cake")?;
    dye_color_map_color(color).map(|_| MAP_COLOR_NONE)
}

fn default_none_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    matches!(
        name,
        "air" | "cave_air" | "void_air" | "nether_portal" | "test_instance_block"
    )
    .then_some(MAP_COLOR_NONE)
}

fn wooden_sign_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if let Some(family) = name.strip_suffix("_wall_hanging_sign") {
        return if family == "spruce" {
            Some(MAP_COLOR_WOOD)
        } else {
            hanging_sign_family_map_color(family)
        };
    }
    if let Some(family) = name.strip_suffix("_hanging_sign") {
        return hanging_sign_family_map_color(family);
    }
    let family = name
        .strip_suffix("_wall_sign")
        .or_else(|| name.strip_suffix("_sign"))?;
    wooden_plank_family_map_color(family)
}

fn wooden_shelf_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let family = name.strip_suffix("_shelf")?;
    wooden_plank_family_map_color(family)
}

fn hanging_sign_family_map_color(family: &str) -> Option<u32> {
    if family == "cherry" {
        return Some(MAP_COLOR_TERRACOTTA_PINK);
    }
    wooden_plank_family_map_color(family)
}

fn wooden_plank_family_map_color(family: &str) -> Option<u32> {
    Some(match family {
        "oak" => MAP_COLOR_WOOD,
        "spruce" => MAP_COLOR_PODZOL,
        "birch" => MAP_COLOR_SAND,
        "jungle" => MAP_COLOR_DIRT,
        "acacia" => MAP_COLOR_ORANGE,
        "cherry" => MAP_COLOR_TERRACOTTA_WHITE,
        "dark_oak" => MAP_COLOR_BROWN,
        "pale_oak" => MAP_COLOR_QUARTZ,
        "mangrove" => MAP_COLOR_RED,
        "bamboo" | "bamboo_mosaic" => MAP_COLOR_YELLOW,
        "crimson" => MAP_COLOR_CRIMSON_STEM,
        "warped" => MAP_COLOR_WARPED_STEM,
        _ => return None,
    })
}

fn vanilla_static_map_color_for_block_state(
    name: &str,
    properties: &std::collections::BTreeMap<String, String>,
) -> Option<u32> {
    if let Some(color) = colored_family_map_color(name) {
        return Some(color);
    }
    if let Some(color) = banner_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = candle_map_color(name) {
        return Some(color);
    }
    if let Some(color) = shulker_box_map_color(name) {
        return Some(color);
    }
    if let Some(color) = bed_map_color(name, properties) {
        return Some(color);
    }
    if let Some(color) = ore_map_color(name) {
        return Some(color);
    }
    if let Some(color) = deepslate_family_map_color(name) {
        return Some(color);
    }
    if let Some(color) = copper_weathering_map_color(name) {
        return Some(color);
    }
    if let Some(color) = copper_lantern_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = wooden_stairs_and_slabs_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = wooden_pressure_plate_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = wooden_door_trapdoor_fence_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = wooden_sign_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = wooden_shelf_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = button_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = potted_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = cake_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = default_none_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = construction_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = resin_and_pale_garden_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = crop_static_map_color(name, properties) {
        return Some(color);
    }
    if let Some(color) = produce_and_fungus_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = natural_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = aquatic_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = utility_static_map_color(name) {
        return Some(color);
    }
    match name {
        "minecraft:stone"
        | "minecraft:andesite"
        | "minecraft:polished_andesite"
        | "minecraft:cobblestone"
        | "minecraft:suspicious_gravel"
        | "minecraft:dispenser"
        | "minecraft:dropper"
        | "minecraft:furnace"
        | "minecraft:piston_head"
        | "minecraft:moving_piston" => Some(MAP_COLOR_STONE),
        "minecraft:granite"
        | "minecraft:polished_granite"
        | "minecraft:dirt"
        | "minecraft:coarse_dirt"
        | "minecraft:dirt_path"
        | "minecraft:jungle_planks"
        | "minecraft:farmland"
        | "minecraft:jukebox" => Some(MAP_COLOR_DIRT),
        "minecraft:diorite" | "minecraft:polished_diorite" | "minecraft:pale_oak_planks" => {
            Some(MAP_COLOR_QUARTZ)
        }
        "minecraft:quartz_block"
        | "minecraft:chiseled_quartz_block"
        | "minecraft:quartz_pillar"
        | "minecraft:quartz_stairs"
        | "minecraft:quartz_slab"
        | "minecraft:smooth_quartz"
        | "minecraft:smooth_quartz_stairs"
        | "minecraft:smooth_quartz_slab"
        | "minecraft:quartz_bricks"
        | "minecraft:sea_lantern" => Some(MAP_COLOR_QUARTZ),
        "minecraft:pale_oak_wood" => Some(MAP_COLOR_STONE),
        "minecraft:oak_planks" => Some(MAP_COLOR_WOOD),
        "minecraft:spruce_planks"
        | "minecraft:podzol"
        | "minecraft:mangrove_roots"
        | "minecraft:muddy_mangrove_roots"
        | "minecraft:spruce_wood"
        | "minecraft:stripped_spruce_wood" => Some(MAP_COLOR_PODZOL),
        "minecraft:birch_planks" => Some(MAP_COLOR_SAND),
        "minecraft:acacia_planks" | "minecraft:terracotta" => Some(MAP_COLOR_ORANGE),
        "minecraft:cherry_planks" => Some(MAP_COLOR_TERRACOTTA_WHITE),
        "minecraft:dark_oak_planks" => Some(MAP_COLOR_BROWN),
        "minecraft:mangrove_planks" => Some(MAP_COLOR_RED),
        "minecraft:bamboo_planks" | "minecraft:bamboo_mosaic" => Some(MAP_COLOR_YELLOW),
        "minecraft:suspicious_sand"
        | "minecraft:sandstone"
        | "minecraft:chiseled_sandstone"
        | "minecraft:cut_sandstone"
        | "minecraft:end_stone"
        | "minecraft:end_stone_bricks"
        | "minecraft:end_stone_brick_stairs"
        | "minecraft:end_stone_brick_slab"
        | "minecraft:end_stone_brick_wall"
        | "minecraft:bone_block" => Some(MAP_COLOR_SAND),
        "minecraft:sponge" | "minecraft:wet_sponge" => Some(MAP_COLOR_YELLOW),
        "minecraft:snow" | "minecraft:snow_block" => Some(MAP_COLOR_SNOW),
        "minecraft:ice"
        | "minecraft:packed_ice"
        | "minecraft:blue_ice"
        | "minecraft:frosted_ice" => Some(MAP_COLOR_ICE),
        "minecraft:clay" => Some(MAP_COLOR_CLAY),
        "minecraft:infested_stone"
        | "minecraft:infested_cobblestone"
        | "minecraft:infested_stone_bricks"
        | "minecraft:infested_mossy_stone_bricks"
        | "minecraft:infested_cracked_stone_bricks"
        | "minecraft:infested_chiseled_stone_bricks" => Some(MAP_COLOR_CLAY),
        "minecraft:lapis_block" => Some(MAP_COLOR_LAPIS),
        "minecraft:diamond_block" => Some(MAP_COLOR_DIAMOND),
        "minecraft:emerald_block" => Some(MAP_COLOR_EMERALD),
        "minecraft:gold_block" | "minecraft:raw_gold_block" => Some(MAP_COLOR_GOLD),
        "minecraft:iron_block" => Some(MAP_COLOR_METAL),
        "minecraft:raw_iron_block" => Some(MAP_COLOR_RAW_IRON),
        "minecraft:coal_block"
        | "minecraft:basalt"
        | "minecraft:polished_basalt"
        | "minecraft:obsidian"
        | "minecraft:crying_obsidian"
        | "minecraft:ancient_debris"
        | "minecraft:netherite_block" => Some(MAP_COLOR_BLACK),
        "minecraft:netherrack"
        | "minecraft:nether_bricks"
        | "minecraft:red_nether_bricks"
        | "minecraft:chiseled_nether_bricks"
        | "minecraft:cracked_nether_bricks"
        | "minecraft:magma_block"
        | "minecraft:crimson_fungus"
        | "minecraft:weeping_vines"
        | "minecraft:weeping_vines_plant"
        | "minecraft:crimson_roots" => Some(MAP_COLOR_NETHER),
        "minecraft:soul_sand" | "minecraft:soul_soil" => Some(MAP_COLOR_BROWN),
        "minecraft:glow_lichen" => Some(MAP_COLOR_GLOW_LICHEN),
        "minecraft:prismarine"
        | "minecraft:prismarine_stairs"
        | "minecraft:prismarine_slab"
        | "minecraft:prismarine_wall" => Some(MAP_COLOR_CYAN),
        "minecraft:prismarine_bricks"
        | "minecraft:prismarine_brick_stairs"
        | "minecraft:prismarine_brick_slab"
        | "minecraft:dark_prismarine"
        | "minecraft:dark_prismarine_stairs"
        | "minecraft:dark_prismarine_slab" => Some(MAP_COLOR_DIAMOND),
        "minecraft:warped_nylium" => Some(MAP_COLOR_WARPED_NYLIUM),
        "minecraft:crimson_nylium" => Some(MAP_COLOR_CRIMSON_NYLIUM),
        "minecraft:warped_wart_block" => Some(MAP_COLOR_WARPED_WART_BLOCK),
        "minecraft:redstone_block" => Some(MAP_COLOR_FIRE),
        "minecraft:slime_block" => Some(MAP_COLOR_GRASS),
        "minecraft:nether_wart_block" | "minecraft:shroomlight" => Some(MAP_COLOR_RED),
        "minecraft:warped_fungus"
        | "minecraft:warped_roots"
        | "minecraft:nether_sprouts"
        | "minecraft:twisting_vines"
        | "minecraft:twisting_vines_plant" => Some(MAP_COLOR_CYAN),
        "minecraft:amethyst_block"
        | "minecraft:budding_amethyst"
        | "minecraft:amethyst_cluster"
        | "minecraft:large_amethyst_bud"
        | "minecraft:medium_amethyst_bud"
        | "minecraft:small_amethyst_bud" => Some(MAP_COLOR_PURPLE),
        "minecraft:chorus_plant" | "minecraft:chorus_flower" => Some(MAP_COLOR_PURPLE),
        "minecraft:purpur_block"
        | "minecraft:purpur_pillar"
        | "minecraft:purpur_stairs"
        | "minecraft:purpur_slab" => Some(MAP_COLOR_MAGENTA),
        "minecraft:end_portal_frame" => Some(MAP_COLOR_GREEN),
        "minecraft:tuff"
        | "minecraft:tuff_slab"
        | "minecraft:tuff_stairs"
        | "minecraft:tuff_wall"
        | "minecraft:polished_tuff"
        | "minecraft:polished_tuff_slab"
        | "minecraft:polished_tuff_stairs"
        | "minecraft:polished_tuff_wall"
        | "minecraft:chiseled_tuff"
        | "minecraft:tuff_bricks"
        | "minecraft:tuff_brick_slab"
        | "minecraft:tuff_brick_stairs"
        | "minecraft:tuff_brick_wall"
        | "minecraft:chiseled_tuff_bricks" => Some(MAP_COLOR_TERRACOTTA_GRAY),
        "minecraft:calcite" => Some(MAP_COLOR_TERRACOTTA_WHITE),
        "minecraft:tinted_glass" => Some(MAP_COLOR_GRAY),
        "minecraft:powder_snow" => Some(MAP_COLOR_SNOW),
        "minecraft:sculk_sensor" | "minecraft:calibrated_sculk_sensor" => Some(MAP_COLOR_CYAN),
        "minecraft:sculk"
        | "minecraft:sculk_vein"
        | "minecraft:sculk_catalyst"
        | "minecraft:sculk_shrieker"
        | "minecraft:smooth_basalt"
        | "minecraft:respawn_anchor"
        | "minecraft:blackstone"
        | "minecraft:blackstone_stairs"
        | "minecraft:blackstone_wall"
        | "minecraft:blackstone_slab"
        | "minecraft:polished_blackstone"
        | "minecraft:polished_blackstone_stairs"
        | "minecraft:polished_blackstone_slab"
        | "minecraft:polished_blackstone_wall"
        | "minecraft:polished_blackstone_pressure_plate"
        | "minecraft:polished_blackstone_bricks"
        | "minecraft:polished_blackstone_brick_slab"
        | "minecraft:polished_blackstone_brick_stairs"
        | "minecraft:polished_blackstone_brick_wall"
        | "minecraft:cracked_polished_blackstone_bricks"
        | "minecraft:chiseled_polished_blackstone"
        | "minecraft:gilded_blackstone" => Some(MAP_COLOR_BLACK),
        "minecraft:ochre_froglight" => Some(MAP_COLOR_SAND),
        "minecraft:verdant_froglight" => Some(MAP_COLOR_GLOW_LICHEN),
        "minecraft:pearlescent_froglight" => Some(MAP_COLOR_PINK),
        "minecraft:crimson_planks" => Some(MAP_COLOR_CRIMSON_STEM),
        "minecraft:warped_planks" => Some(MAP_COLOR_WARPED_STEM),
        "minecraft:oak_wood" | "minecraft:stripped_oak_wood" | "minecraft:petrified_oak_slab" => {
            Some(MAP_COLOR_WOOD)
        }
        "minecraft:birch_wood" | "minecraft:stripped_birch_wood" => Some(MAP_COLOR_SAND),
        "minecraft:jungle_wood" | "minecraft:stripped_jungle_wood" => Some(MAP_COLOR_DIRT),
        "minecraft:acacia_wood" => Some(MAP_COLOR_GRAY),
        "minecraft:stripped_acacia_wood" => Some(MAP_COLOR_ORANGE),
        "minecraft:cherry_wood" => Some(MAP_COLOR_TERRACOTTA_GRAY),
        "minecraft:stripped_cherry_wood" => Some(MAP_COLOR_TERRACOTTA_PINK),
        "minecraft:dark_oak_wood" | "minecraft:stripped_dark_oak_wood" => Some(MAP_COLOR_BROWN),
        "minecraft:stripped_pale_oak_wood" => Some(MAP_COLOR_QUARTZ),
        "minecraft:mangrove_wood"
        | "minecraft:stripped_mangrove_wood"
        | "minecraft:stripped_mangrove_log" => Some(MAP_COLOR_RED),
        "minecraft:stripped_bamboo_block" => Some(MAP_COLOR_YELLOW),
        "minecraft:crimson_stem" | "minecraft:stripped_crimson_stem" => {
            Some(MAP_COLOR_CRIMSON_STEM)
        }
        "minecraft:warped_stem" | "minecraft:stripped_warped_stem" => Some(MAP_COLOR_WARPED_STEM),
        "minecraft:crimson_hyphae" | "minecraft:stripped_crimson_hyphae" => {
            Some(MAP_COLOR_CRIMSON_HYPHAE)
        }
        "minecraft:warped_hyphae" | "minecraft:stripped_warped_hyphae" => {
            Some(MAP_COLOR_WARPED_HYPHAE)
        }
        "minecraft:oak_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_WOOD,
            MAP_COLOR_PODZOL,
        )),
        "minecraft:spruce_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_PODZOL,
            MAP_COLOR_BROWN,
        )),
        "minecraft:birch_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_SAND,
            MAP_COLOR_QUARTZ,
        )),
        "minecraft:jungle_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_DIRT,
            MAP_COLOR_PODZOL,
        )),
        "minecraft:acacia_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_ORANGE,
            MAP_COLOR_STONE,
        )),
        "minecraft:cherry_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_TERRACOTTA_WHITE,
            MAP_COLOR_TERRACOTTA_GRAY,
        )),
        "minecraft:dark_oak_log" => Some(MAP_COLOR_BROWN),
        "minecraft:pale_oak_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_QUARTZ,
            MAP_COLOR_STONE,
        )),
        "minecraft:mangrove_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_RED,
            MAP_COLOR_PODZOL,
        )),
        "minecraft:bamboo_block" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_YELLOW,
            MAP_COLOR_PLANT,
        )),
        "minecraft:stripped_spruce_log" => Some(MAP_COLOR_PODZOL),
        "minecraft:stripped_birch_log" => Some(MAP_COLOR_SAND),
        "minecraft:stripped_jungle_log" => Some(MAP_COLOR_DIRT),
        "minecraft:stripped_acacia_log" => Some(MAP_COLOR_ORANGE),
        "minecraft:stripped_cherry_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_TERRACOTTA_WHITE,
            MAP_COLOR_TERRACOTTA_PINK,
        )),
        "minecraft:stripped_dark_oak_log" => Some(MAP_COLOR_BROWN),
        "minecraft:stripped_pale_oak_log" => Some(MAP_COLOR_QUARTZ),
        "minecraft:stripped_oak_log" => Some(MAP_COLOR_WOOD),
        _ => None,
    }
}

fn construction_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "mossy_cobblestone"
        | "cobblestone_stairs"
        | "cobblestone_slab"
        | "cobblestone_wall"
        | "mossy_cobblestone_stairs"
        | "mossy_cobblestone_slab"
        | "mossy_cobblestone_wall"
        | "stone_bricks"
        | "mossy_stone_bricks"
        | "cracked_stone_bricks"
        | "chiseled_stone_bricks"
        | "stone_brick_stairs"
        | "stone_brick_slab"
        | "stone_brick_wall"
        | "mossy_stone_brick_stairs"
        | "mossy_stone_brick_slab"
        | "mossy_stone_brick_wall"
        | "stone_stairs"
        | "stone_slab"
        | "smooth_stone"
        | "smooth_stone_slab"
        | "andesite_stairs"
        | "andesite_slab"
        | "andesite_wall"
        | "polished_andesite_stairs"
        | "polished_andesite_slab" => MAP_COLOR_STONE,
        "granite_stairs"
        | "granite_slab"
        | "granite_wall"
        | "polished_granite_stairs"
        | "polished_granite_slab" => MAP_COLOR_DIRT,
        "diorite_stairs"
        | "diorite_slab"
        | "diorite_wall"
        | "polished_diorite_stairs"
        | "polished_diorite_slab" => MAP_COLOR_QUARTZ,
        "sandstone_stairs"
        | "sandstone_slab"
        | "sandstone_wall"
        | "cut_sandstone_slab"
        | "smooth_sandstone"
        | "smooth_sandstone_stairs"
        | "smooth_sandstone_slab" => MAP_COLOR_SAND,
        "red_sandstone"
        | "chiseled_red_sandstone"
        | "cut_red_sandstone"
        | "red_sandstone_stairs"
        | "red_sandstone_slab"
        | "red_sandstone_wall"
        | "cut_red_sandstone_slab"
        | "smooth_red_sandstone"
        | "smooth_red_sandstone_stairs"
        | "smooth_red_sandstone_slab" => MAP_COLOR_ORANGE,
        "packed_mud" => MAP_COLOR_DIRT,
        "bricks" | "brick_stairs" | "brick_slab" | "brick_wall" => MAP_COLOR_RED,
        "mud_bricks" | "mud_brick_stairs" | "mud_brick_slab" | "mud_brick_wall" => {
            MAP_COLOR_TERRACOTTA_LIGHT_GRAY
        }
        "nether_brick_stairs"
        | "nether_brick_slab"
        | "nether_brick_wall"
        | "nether_brick_fence"
        | "red_nether_brick_stairs"
        | "red_nether_brick_slab"
        | "red_nether_brick_wall" => MAP_COLOR_NETHER,
        _ => return None,
    })
}

fn resin_and_pale_garden_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "resin_block"
        | "resin_clump"
        | "resin_bricks"
        | "resin_brick_stairs"
        | "resin_brick_slab"
        | "resin_brick_wall"
        | "chiseled_resin_bricks" => MAP_COLOR_TERRACOTTA_ORANGE,
        "pale_moss_block" | "pale_moss_carpet" | "pale_hanging_moss" => MAP_COLOR_LIGHT_GRAY,
        "open_eyeblossom" => MAP_COLOR_ORANGE,
        "closed_eyeblossom" => MAP_COLOR_METAL,
        "firefly_bush" => MAP_COLOR_PLANT,
        _ => return None,
    })
}

fn crop_static_map_color(
    name: &str,
    properties: &std::collections::BTreeMap<String, String>,
) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "wheat" => {
            if properties
                .get("age")
                .and_then(|age| age.parse::<u8>().ok())
                .is_some_and(|age| age >= 6)
            {
                MAP_COLOR_YELLOW
            } else {
                MAP_COLOR_PLANT
            }
        }
        "carrots" | "potatoes" | "beetroots" | "torchflower_crop" | "pitcher_crop"
        | "pitcher_plant" | "cactus" => MAP_COLOR_PLANT,
        "cactus_flower" => MAP_COLOR_PINK,
        "nether_wart" => MAP_COLOR_RED,
        _ => return None,
    })
}

fn produce_and_fungus_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "brown_mushroom" => MAP_COLOR_BROWN,
        "red_mushroom" | "red_mushroom_block" => MAP_COLOR_RED,
        "brown_mushroom_block" => MAP_COLOR_DIRT,
        "mushroom_stem" => MAP_COLOR_WOOL,
        "pumpkin" | "carved_pumpkin" | "jack_o_lantern" => MAP_COLOR_ORANGE,
        "melon" => MAP_COLOR_LIGHT_GREEN,
        "hay_block" => MAP_COLOR_YELLOW,
        "dried_kelp_block" => MAP_COLOR_GREEN,
        _ => return None,
    })
}

fn natural_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "oak_sapling"
        | "spruce_sapling"
        | "birch_sapling"
        | "jungle_sapling"
        | "acacia_sapling"
        | "dark_oak_sapling"
        | "mangrove_propagule"
        | "azalea_leaves"
        | "flowering_azalea_leaves"
        | "cave_vines"
        | "cave_vines_plant"
        | "spore_blossom"
        | "azalea"
        | "flowering_azalea"
        | "big_dripleaf"
        | "big_dripleaf_stem"
        | "small_dripleaf"
        | "bamboo"
        | "sweet_berry_bush"
        | "cocoa"
        | "dandelion"
        | "golden_dandelion"
        | "torchflower"
        | "poppy"
        | "blue_orchid"
        | "allium"
        | "azure_bluet"
        | "red_tulip"
        | "orange_tulip"
        | "white_tulip"
        | "pink_tulip"
        | "oxeye_daisy"
        | "cornflower"
        | "wither_rose"
        | "lily_of_the_valley"
        | "sunflower"
        | "lilac"
        | "rose_bush"
        | "peony" => MAP_COLOR_PLANT,
        "seagrass" | "tall_seagrass" | "kelp" | "kelp_plant" | "frogspawn" => MAP_COLOR_WATER,
        "cherry_sapling" | "cherry_leaves" => MAP_COLOR_PINK,
        "pale_oak_sapling" | "pale_oak_leaves" => MAP_COLOR_METAL,
        "dead_bush" => MAP_COLOR_WOOD,
        "bamboo_sapling" => MAP_COLOR_WOOD,
        "turtle_egg" => MAP_COLOR_SAND,
        "mycelium" => MAP_COLOR_PURPLE,
        "short_dry_grass" | "tall_dry_grass" => MAP_COLOR_YELLOW,
        "pointed_dripstone" | "dripstone_block" => MAP_COLOR_TERRACOTTA_BROWN,
        "moss_carpet" | "moss_block" => MAP_COLOR_GREEN,
        "hanging_roots" | "rooted_dirt" => MAP_COLOR_DIRT,
        "mud" => MAP_COLOR_TERRACOTTA_CYAN,
        "sniffer_egg" => MAP_COLOR_RED,
        "dried_ghast" => MAP_COLOR_GRAY,
        _ => return None,
    })
}

fn aquatic_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "sea_pickle" => MAP_COLOR_GREEN,
        "conduit" => MAP_COLOR_DIAMOND,
        _ => coral_static_map_color(name)?,
    })
}

fn coral_static_map_color(name: &str) -> Option<u32> {
    let (dead, name) = name
        .strip_prefix("dead_")
        .map(|name| (true, name))
        .unwrap_or((false, name));
    let family = name
        .strip_suffix("_coral_wall_fan")
        .or_else(|| name.strip_suffix("_coral_fan"))
        .or_else(|| name.strip_suffix("_coral_block"))
        .or_else(|| name.strip_suffix("_coral"))?;
    if dead {
        return matches!(family, "tube" | "brain" | "bubble" | "fire" | "horn")
            .then_some(MAP_COLOR_GRAY);
    }
    Some(match family {
        "tube" => MAP_COLOR_BLUE,
        "brain" => MAP_COLOR_PINK,
        "bubble" => MAP_COLOR_PURPLE,
        "fire" => MAP_COLOR_RED,
        "horn" => MAP_COLOR_YELLOW,
        _ => return None,
    })
}

fn utility_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if copper_bars_or_chain_default_none(name) {
        return Some(MAP_COLOR_NONE);
    }
    Some(match name {
        "bedrock"
        | "sticky_piston"
        | "piston"
        | "spawner"
        | "crafter"
        | "trial_spawner"
        | "vault"
        | "stone_pressure_plate"
        | "cauldron"
        | "lava_cauldron"
        | "powder_snow_cauldron"
        | "hopper"
        | "smoker"
        | "blast_furnace"
        | "ender_chest"
        | "observer"
        | "stonecutter" => MAP_COLOR_STONE,
        "note_block" | "bookshelf" | "chiseled_bookshelf" | "chest" | "crafting_table" | "loom"
        | "barrel" | "cartography_table" | "fletching_table" | "lectern" | "smithing_table"
        | "composter" | "beehive" | "trapped_chest" | "daylight_detector" => MAP_COLOR_WOOD,
        "scaffolding" => MAP_COLOR_SAND,
        "glowstone" => MAP_COLOR_SAND,
        "campfire" | "soul_campfire" => MAP_COLOR_PODZOL,
        "cobweb" => MAP_COLOR_WOOL,
        "tnt" => MAP_COLOR_FIRE,
        "fire" => MAP_COLOR_FIRE,
        "soul_fire" => MAP_COLOR_LIGHT_BLUE,
        "creaking_heart" => MAP_COLOR_ORANGE,
        "decorated_pot" => MAP_COLOR_TERRACOTTA_RED,
        "honey_block" | "honeycomb_block" => MAP_COLOR_ORANGE,
        "redstone_lamp" => MAP_COLOR_TERRACOTTA_ORANGE,
        "target" => MAP_COLOR_QUARTZ,
        "enchanting_table" => MAP_COLOR_RED,
        "bee_nest" => MAP_COLOR_YELLOW,
        "beacon" => MAP_COLOR_DIAMOND,
        "command_block" => MAP_COLOR_BROWN,
        "repeating_command_block" => MAP_COLOR_PURPLE,
        "chain_command_block" => MAP_COLOR_GREEN,
        "structure_block" | "jigsaw" | "test_block" => MAP_COLOR_LIGHT_GRAY,
        "glass"
        | "glass_pane"
        | "iron_bars"
        | "iron_chain"
        | "ladder"
        | "torch"
        | "wall_torch"
        | "redstone_torch"
        | "redstone_wall_torch"
        | "soul_torch"
        | "soul_wall_torch"
        | "copper_torch"
        | "copper_wall_torch"
        | "end_rod"
        | "powered_rail"
        | "detector_rail"
        | "rail"
        | "lever"
        | "repeater"
        | "tripwire_hook"
        | "tripwire"
        | "comparator"
        | "activator_rail"
        | "skeleton_skull"
        | "skeleton_wall_skull"
        | "wither_skeleton_skull"
        | "wither_skeleton_wall_skull"
        | "zombie_head"
        | "zombie_wall_head"
        | "player_head"
        | "player_wall_head"
        | "creeper_head"
        | "creeper_wall_head"
        | "dragon_head"
        | "dragon_wall_head"
        | "piglin_head"
        | "piglin_wall_head" => MAP_COLOR_NONE,
        "light_weighted_pressure_plate" | "bell" => MAP_COLOR_GOLD,
        "heavy_weighted_pressure_plate"
        | "iron_door"
        | "iron_trapdoor"
        | "brewing_stand"
        | "lantern"
        | "soul_lantern"
        | "grindstone"
        | "lodestone"
        | "heavy_core" => MAP_COLOR_METAL,
        _ => return None,
    })
}

fn copper_bars_or_chain_default_none(name: &str) -> bool {
    let name = name.strip_prefix("waxed_").unwrap_or(name);
    let name = name
        .strip_prefix("exposed_")
        .or_else(|| name.strip_prefix("weathered_"))
        .or_else(|| name.strip_prefix("oxidized_"))
        .unwrap_or(name);
    matches!(name, "copper_bars" | "copper_chain")
}

fn rotated_pillar_map_color(
    properties: &std::collections::BTreeMap<String, String>,
    top_color: u32,
    side_color: u32,
) -> u32 {
    if properties.get("axis").is_some_and(|axis| axis == "y") {
        top_color
    } else {
        side_color
    }
}

pub(super) fn terrain_particle_provider_accepts_block_state(block_state_id: i32) -> bool {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let Some(block_state) = block_states.by_id(block_state_id) else {
        return true;
    };
    !block_name_is_air(&block_state.name)
        && block_state.name != "minecraft:moving_piston"
        && block_name_should_spawn_terrain_particles(&block_state.name)
}

pub(super) fn destroy_block_effect_accepts_block_state(block_state_id: i32) -> bool {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let Some(block_state) = block_states.by_id(block_state_id) else {
        return true;
    };
    !block_name_is_air(&block_state.name)
        && block_name_should_spawn_terrain_particles(&block_state.name)
}

const MAP_COLOR_NONE: u32 = 0;
const MAP_COLOR_GRASS: u32 = 8_368_696;
const MAP_COLOR_SAND: u32 = 16_247_203;
const MAP_COLOR_WOOL: u32 = 13_092_807;
const MAP_COLOR_FIRE: u32 = 16_711_680;
const MAP_COLOR_ICE: u32 = 10_526_975;
const MAP_COLOR_SNOW: u32 = 16_777_215;
const MAP_COLOR_METAL: u32 = 10_987_431;
const MAP_COLOR_PLANT: u32 = 31_744;
const MAP_COLOR_CLAY: u32 = 10_791_096;
const MAP_COLOR_DIRT: u32 = 9_923_917;
const MAP_COLOR_STONE: u32 = 7_368_816;
const MAP_COLOR_WATER: u32 = 4_210_943;
const MAP_COLOR_WOOD: u32 = 9_402_184;
const MAP_COLOR_QUARTZ: u32 = 16_776_437;
const MAP_COLOR_ORANGE: u32 = 14_188_339;
const MAP_COLOR_MAGENTA: u32 = 11_685_080;
const MAP_COLOR_LIGHT_BLUE: u32 = 6_724_056;
const MAP_COLOR_YELLOW: u32 = 15_066_419;
const MAP_COLOR_LIGHT_GREEN: u32 = 8_375_321;
const MAP_COLOR_PINK: u32 = 15_892_389;
const MAP_COLOR_GRAY: u32 = 5_000_268;
const MAP_COLOR_LIGHT_GRAY: u32 = 10_066_329;
const MAP_COLOR_CYAN: u32 = 5_013_401;
const MAP_COLOR_PURPLE: u32 = 8_339_378;
const MAP_COLOR_BLUE: u32 = 3_361_970;
const MAP_COLOR_BROWN: u32 = 6_704_179;
const MAP_COLOR_GREEN: u32 = 6_717_235;
const MAP_COLOR_RED: u32 = 10_040_115;
const MAP_COLOR_BLACK: u32 = 1_644_825;
const MAP_COLOR_GOLD: u32 = 16_445_005;
const MAP_COLOR_DIAMOND: u32 = 6_085_589;
const MAP_COLOR_LAPIS: u32 = 4_882_687;
const MAP_COLOR_EMERALD: u32 = 55_610;
const MAP_COLOR_TERRACOTTA_WHITE: u32 = 13_742_497;
const MAP_COLOR_TERRACOTTA_ORANGE: u32 = 10_441_252;
const MAP_COLOR_TERRACOTTA_MAGENTA: u32 = 9_787_244;
const MAP_COLOR_TERRACOTTA_LIGHT_BLUE: u32 = 7_367_818;
const MAP_COLOR_TERRACOTTA_YELLOW: u32 = 12_223_780;
const MAP_COLOR_TERRACOTTA_LIGHT_GREEN: u32 = 6_780_213;
const MAP_COLOR_TERRACOTTA_GRAY: u32 = 3_746_083;
const MAP_COLOR_TERRACOTTA_PINK: u32 = 10_505_550;
const MAP_COLOR_TERRACOTTA_LIGHT_GRAY: u32 = 8_874_850;
const MAP_COLOR_TERRACOTTA_CYAN: u32 = 5_725_276;
const MAP_COLOR_TERRACOTTA_PURPLE: u32 = 8_014_168;
const MAP_COLOR_TERRACOTTA_BLUE: u32 = 4_996_700;
const MAP_COLOR_TERRACOTTA_BROWN: u32 = 4_993_571;
const MAP_COLOR_TERRACOTTA_GREEN: u32 = 5_001_770;
const MAP_COLOR_TERRACOTTA_RED: u32 = 9_321_518;
const MAP_COLOR_TERRACOTTA_BLACK: u32 = 2_430_480;
const MAP_COLOR_PODZOL: u32 = 8_476_209;
const MAP_COLOR_NETHER: u32 = 7_340_544;
const MAP_COLOR_CRIMSON_NYLIUM: u32 = 12_398_641;
const MAP_COLOR_CRIMSON_STEM: u32 = 9_715_553;
const MAP_COLOR_CRIMSON_HYPHAE: u32 = 6_035_741;
const MAP_COLOR_WARPED_NYLIUM: u32 = 1_474_182;
const MAP_COLOR_WARPED_STEM: u32 = 3_837_580;
const MAP_COLOR_WARPED_HYPHAE: u32 = 5_647_422;
const MAP_COLOR_WARPED_WART_BLOCK: u32 = 1_356_933;
const MAP_COLOR_DEEPSLATE: u32 = 6_579_300;
const MAP_COLOR_RAW_IRON: u32 = 14_200_723;
const MAP_COLOR_GLOW_LICHEN: u32 = 8_365_974;
