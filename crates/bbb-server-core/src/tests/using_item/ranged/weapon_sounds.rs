use super::*;
use bbb_protocol::equipment_slots::EquipmentSlot;

fn config_with_weapon_sounds() -> (ServerWorldConfig, std::path::PathBuf) {
    let mut config = datapack_config();
    let root = std::env::temp_dir().join(format!("bbb-weapon-sounds-{}", uuid::Uuid::new_v4()));
    let pack = root.join("sounds");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"weapon sounds","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for (id, replacements) in [
        (
            "quick_charge",
            serde_json::json!({
                "minecraft:crossbow_charging_sounds": [{"start":null,"mid":null,"end":null}, {
                    "start": {"sound_id":"minecraft:item.crossbow.loading_start", "range":32},
                    "mid": {"sound_id":"test:loading_mid", "range":31},
                    "end": {"sound_id":"test:loading_end", "range":30}
                }]
            }),
        ),
        (
            "loyalty",
            serde_json::json!({
                "minecraft:trident_sound": [{"sound_id":"test:throw", "range":29}]
            }),
        ),
        (
            "riptide",
            serde_json::json!({
                "minecraft:trident_sound": [{"sound_id":"test:spin", "range":28}]
            }),
        ),
    ] {
        let mut document: serde_json::Value = serde_json::from_slice(
            &config
                .datapacks
                .as_ref()
                .unwrap()
                .registries
                .element("minecraft:enchantment", &format!("minecraft:{id}"))
                .unwrap()
                .source_json,
        )
        .unwrap();
        for (key, value) in replacements.as_object().unwrap() {
            document["effects"][key] = value.clone();
        }
        std::fs::write(
            pack.join(format!("data/minecraft/enchantment/{id}.json")),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
    }
    config.datapacks = Some(std::sync::Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(root.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            },
        )
        .unwrap(),
    ));
    (config, root)
}

fn direct(id: &str, range: f32) -> SoundEventHolder {
    SoundEventHolder::Direct {
        location: id.to_owned(),
        fixed_range: Some(range),
    }
}

fn position_sounds(tick: &TickResult) -> Vec<SoundEventHolder> {
    packets_to(tick, BOB)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::Sound(sound) => Some(sound.sound.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn inline_weapon_sounds_reach_player_and_mob_packets() {
    let (config, root) = config_with_weapon_sounds();
    let mut fixture = archer_with_config(config);
    let crossbow = enchanted(
        &fixture,
        stack("minecraft:crossbow", 1),
        &[("minecraft:quick_charge", 3)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, crossbow.clone());
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 2));
    let mut heard = position_sounds(&fixture.use_item(InteractionHand::MainHand));
    for _ in 0..12 {
        heard.extend(position_sounds(&fixture.idle()));
    }
    fixture.action(PlayerActionKind::ReleaseUseItem);
    let charging = [
        direct("minecraft:item.crossbow.loading_start", 32.0),
        direct("test:loading_mid", 31.0),
        direct("test:loading_end", 30.0),
    ];
    for sound in &charging {
        assert!(
            heard.contains(sound),
            "player charge: {sound:?} missing from {heard:?}"
        );
    }
    let trident = enchanted(
        &fixture,
        stack("minecraft:trident", 1),
        &[("minecraft:loyalty", 1)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, trident);
    let thrown = draw_and_release(&mut fixture, 10);
    let trident_id = entities_of(&fixture, VANILLA_ENTITY_TYPE_TRIDENT_ID)[0].id();
    assert!(packets_to(&thrown, BOB).any(|packet| matches!(packet,
        ClientboundPlayPacket::SoundEntity(sound)
            if sound.entity_id == trident_id && sound.sound == direct("test:throw", 29.0))));

    let trident = enchanted(
        &fixture,
        stack("minecraft:trident", 1),
        &[("minecraft:riptide", 1)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, trident);
    fixture.command("setblock 0 64 0 minecraft:water");
    let spin = draw_and_release(&mut fixture, 10);
    assert!(packets_to(&spin, BOB).any(|packet| matches!(packet,
        ClientboundPlayPacket::SoundEntity(sound)
            if sound.entity_id == fixture.alice && sound.sound == direct("test:spin", 28.0))));

    fixture.command("time set midnight");
    fixture.command("summon minecraft:pillager 0.5 64 6.5");
    let pillager = entities_of(
        &fixture,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID,
    )[0]
    .id();
    fixture
        .world
        .stored_entity_mut_for_test(OVERWORLD, pillager)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(
            EquipmentSlot::MainHand,
            Some(crate::tests::native_stack(crossbow)),
        );
    let mut heard = Vec::new();
    for _ in 0..200 {
        heard.extend(position_sounds(&fixture.idle()));
        if charging.iter().all(|sound| heard.contains(sound)) {
            break;
        }
    }
    for sound in &charging {
        assert!(
            heard.contains(sound),
            "mob charge: {sound:?} missing from {heard:?}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
