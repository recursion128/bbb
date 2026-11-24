use super::*;
use crate::world::natural_spawn::fuzzy_biome_resource_id;
use bbb_protocol::packets::BlockPos;
use bbb_server_worldgen::noise::biome::biome_zoom_seed;

#[test]
fn location_uses_current_datapack_biome_tags_and_rewards_only_members_once() {
    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    let tags = startup
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/biome")
        .unwrap();
    tags.tags
        .insert("test:meadow".into(), vec!["minecraft:plains".into()]);
    tags.tags
        .insert("test:hot".into(), vec!["minecraft:nether_wastes".into()]);
    startup.advancements = AdvancementSet {
        advancements: [("test:meadow", "#test:meadow"), ("test:hot", "#test:hot")]
            .into_iter()
            .map(|(id, biomes)| AdvancementElement {
                id: id.into(),
                source_pack: "test".into(),
                advancement: parse_advancement(&json!({
                    "criteria": {"located": {
                        "trigger": "minecraft:location",
                        "conditions": {"player": {"location": {"biomes": biomes}}}
                    }},
                    "rewards": {"experience": 7}
                }))
                .unwrap(),
            })
            .collect(),
        warnings: Vec::new(),
    };
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "explorer", 1);
    let player = ConnectionId(1);
    let position = world.levels[player].position();
    assert_eq!(
        fuzzy_biome_resource_id(
            &world.registries,
            &world
                .levels
                .get(&world.levels[player].dimension)
                .unwrap()
                .chunks,
            biome_zoom_seed(world.cold.seed),
            BlockPos {
                x: position.x.floor() as i32,
                y: position.y.floor() as i32,
                z: position.z.floor() as i32,
            },
        ),
        Some("minecraft:plains")
    );
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert!(world.player_advancement_is_done(player, "test:meadow"));
    assert!(!world.player_advancement_is_done(player, "test:hot"));
    assert_eq!(world.levels[player].experience.total(), 7);
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert!(!world.player_advancement_is_done(player, "test:hot"));
    assert_eq!(world.levels[player].experience.total(), 7);
}
