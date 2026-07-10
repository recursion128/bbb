//! Vanilla 26.1 BlockEntityType registry ids.
//!
//! The order follows the register calls in
//! net.minecraft.world.level.block.entity.BlockEntityType.

const VANILLA_BLOCK_ENTITY_TYPE_RESOURCE_IDS: &[&str] = &[
    "minecraft:furnace",
    "minecraft:chest",
    "minecraft:trapped_chest",
    "minecraft:ender_chest",
    "minecraft:jukebox",
    "minecraft:dispenser",
    "minecraft:dropper",
    "minecraft:sign",
    "minecraft:hanging_sign",
    "minecraft:mob_spawner",
    "minecraft:creaking_heart",
    "minecraft:piston",
    "minecraft:brewing_stand",
    "minecraft:enchanting_table",
    "minecraft:end_portal",
    "minecraft:beacon",
    "minecraft:skull",
    "minecraft:daylight_detector",
    "minecraft:hopper",
    "minecraft:comparator",
    "minecraft:banner",
    "minecraft:structure_block",
    "minecraft:end_gateway",
    "minecraft:command_block",
    "minecraft:shulker_box",
    "minecraft:bed",
    "minecraft:conduit",
    "minecraft:barrel",
    "minecraft:smoker",
    "minecraft:blast_furnace",
    "minecraft:lectern",
    "minecraft:bell",
    "minecraft:jigsaw",
    "minecraft:campfire",
    "minecraft:beehive",
    "minecraft:sculk_sensor",
    "minecraft:calibrated_sculk_sensor",
    "minecraft:sculk_catalyst",
    "minecraft:sculk_shrieker",
    "minecraft:chiseled_bookshelf",
    "minecraft:shelf",
    "minecraft:brushable_block",
    "minecraft:decorated_pot",
    "minecraft:crafter",
    "minecraft:trial_spawner",
    "minecraft:vault",
    "minecraft:test_block",
    "minecraft:test_instance_block",
    "minecraft:copper_golem_statue",
];

pub fn vanilla_block_entity_resource_id_for_type_id(type_id: i32) -> Option<&'static str> {
    usize::try_from(type_id)
        .ok()
        .and_then(|index| VANILLA_BLOCK_ENTITY_TYPE_RESOURCE_IDS.get(index).copied())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn block_entity_resource_ids_follow_vanilla_registration_order() {
        assert_eq!(
            vanilla_block_entity_resource_id_for_type_id(0),
            Some("minecraft:furnace")
        );
        assert_eq!(
            vanilla_block_entity_resource_id_for_type_id(9),
            Some("minecraft:mob_spawner")
        );
        assert_eq!(
            vanilla_block_entity_resource_id_for_type_id(45),
            Some("minecraft:vault")
        );
        assert_eq!(
            vanilla_block_entity_resource_id_for_type_id(48),
            Some("minecraft:copper_golem_statue")
        );
        assert_eq!(vanilla_block_entity_resource_id_for_type_id(-1), None);
        assert_eq!(vanilla_block_entity_resource_id_for_type_id(49), None);
    }

    #[test]
    fn block_entity_resource_ids_match_local_vanilla_registration_order() {
        let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../mc-code/sources/26.1/net/minecraft/world/level/block/entity")
            .join("BlockEntityType.java");
        if !source_path.exists() {
            eprintln!(
                "skipping local BlockEntityType source check; missing {}",
                source_path.display()
            );
            return;
        }

        let source = std::fs::read_to_string(source_path).unwrap();
        let mut remaining = source.as_str();
        let mut registered = Vec::new();
        while let Some(index) = remaining.find("= register(") {
            remaining = &remaining[index + "= register(".len()..];
            let start = remaining.find('"').unwrap() + 1;
            let tail = &remaining[start..];
            let end = tail.find('"').unwrap();
            registered.push(format!("minecraft:{}", &tail[..end]));
            remaining = &tail[end + 1..];
        }

        assert_eq!(registered, VANILLA_BLOCK_ENTITY_TYPE_RESOURCE_IDS);
    }
}
