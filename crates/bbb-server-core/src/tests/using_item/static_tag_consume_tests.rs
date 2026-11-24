use super::*;
use crate::registry_sync::SynchronizedRegistries;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID;
use bbb_protocol::packets::{
    ConsumableSummary, ConsumeEffectSummary, ItemUseAnimationSummary, SoundEventSummary,
};
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

struct DiskPack(std::path::PathBuf);
impl DiskPack {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "bbb-static-tag-consume-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("custom/data/test/tags/mob_effect")).unwrap();
        std::fs::write(root.join("custom/pack.mcmeta"), r#"{"pack":{"description":"consume tag","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
        Self(root)
    }
    fn write(&self, values: &[&str]) {
        std::fs::write(
            self.0.join("custom/data/test/tags/mob_effect/remove.json"),
            format!(
                r#"{{"replace":true,"values":{},"note":"\ud800"}}"#,
                serde_json::to_string(values).unwrap()
            ),
        )
        .unwrap();
    }
    fn options(&self) -> DatapackStartupOptions {
        DatapackStartupOptions {
            datapacks_dir: Some(self.0.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        }
    }
}
impl Drop for DiskPack {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn fixture(startup: DatapackStartup) -> Fixture {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        force_game_mode: true,
        datapacks: Some(Arc::new(startup)),
        admin_lists: operator_config(&[("bob", 2)]).admin_lists,
        ..ServerWorldConfig::default()
    });
    join_loaded(&sender, &receiver, &mut world, ALICE, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, BOB, "bob", 2);
    world.reset_pending_chunks_for_test(ConnectionId(ALICE), &[]);
    world.reset_pending_chunks_for_test(ConnectionId(BOB), &[]);
    Fixture {
        alice: entity_id_of(&world, ALICE),
        sender,
        receiver,
        world,
    }
}
fn consume(fixture: &mut Fixture) {
    fixture.command("effect give alice minecraft:speed 60");
    fixture.command("effect give alice minecraft:haste 60");
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:speed")
        .is_some());
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:haste")
        .is_some());
    fixture
        .world
        .set_player_food_for_test(ConnectionId(ALICE), 10, 0.);
    let mut food = stack("minecraft:bread", 1);
    food.component_patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID],
        consumable: Some(ConsumableSummary {
            consume_seconds: 0.05,
            animation: ItemUseAnimationSummary::Eat,
        }),
        consumable_sound: Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("minecraft:entity.generic.eat".into()),
            fixed_range_bits: None,
        }),
        consumable_has_consume_particles: Some(false),
        consumable_effects: vec![ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: Vec::new(),
            effect_tag: Some("test:remove".into()),
        }],
        ..DataComponentPatchSummary::default()
    }
    .into();
    fixture.hold(HOTBAR_MENU_SLOT, food);
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..3 {
        fixture.idle();
    }
    assert!(!fixture.use_state().0);
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 0);
}

#[test]
fn real_use_item_remove_effects_follows_disk_startup_reload_and_present_empty_tag() {
    let pack = DiskPack::new();
    pack.write(&["minecraft:speed"]);
    let startup = DatapackStartup::load(&pack.options()).unwrap();
    let original = SynchronizedRegistries::load(Some(&startup))
        .unwrap()
        .registry_tags_shared();
    let mut fixture = fixture(startup.clone());
    consume(&mut fixture);
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:speed")
        .is_none());
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:haste")
        .is_some());
    pack.write(&["minecraft:haste"]);
    let reload = DatapackStartup::reload(&pack.options(), &startup).unwrap();
    let publication = SynchronizedRegistries::load(Some(&reload)).unwrap();
    fixture
        .world
        .install_level_registry_tags(publication.registry_tags_shared());
    consume(&mut fixture);
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:speed")
        .is_some());
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:haste")
        .is_none());
    pack.write(&[]);
    let empty = DatapackStartup::reload(&pack.options(), &reload).unwrap();
    let publication = SynchronizedRegistries::load(Some(&empty)).unwrap();
    assert!(publication
        .registry_tags()
        .members("minecraft:mob_effect", "test:remove")
        .is_empty());
    fixture
        .world
        .install_level_registry_tags(publication.registry_tags_shared());
    consume(&mut fixture);
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:speed")
        .is_some());
    assert!(fixture
        .world
        .player_active_effect_for_test(ConnectionId(ALICE), "minecraft:haste")
        .is_some());
    assert_eq!(fixture.item_used("minecraft:bread"), 3);
    assert_eq!(
        original
            .members("minecraft:mob_effect", "test:remove")
            .iter()
            .map(|member| member.as_ref())
            .collect::<Vec<_>>(),
        ["minecraft:speed"]
    );
}
