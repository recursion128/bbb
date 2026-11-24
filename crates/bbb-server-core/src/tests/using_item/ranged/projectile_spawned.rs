use super::*;

fn config_with_birth_effects(
    effects: serde_json::Value,
) -> (ServerWorldConfig, std::path::PathBuf) {
    let mut config = datapack_config();
    let mut document: serde_json::Value = serde_json::from_slice(
        &config
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", "minecraft:flame")
            .unwrap()
            .source_json,
    )
    .unwrap();
    document["effects"] = serde_json::json!({"minecraft:projectile_spawned":effects});
    let root = std::env::temp_dir().join(format!("bbb-projectile-birth-{}", uuid::Uuid::new_v4()));
    let pack = root.join("birth");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"projectile birth","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(
        pack.join("data/minecraft/enchantment/flame.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
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

fn entity_condition(entity_type: &str) -> serde_json::Value {
    serde_json::json!({
        "condition":"minecraft:entity_properties", "entity":"this",
        "predicate":{"type":entity_type}
    })
}

fn bell() -> serde_json::Value {
    serde_json::json!({
        "type":"minecraft:play_sound", "sound":"minecraft:block.note_block.bell",
        "volume":1, "pitch":1
    })
}

#[test]
fn projectile_birth_conditions_use_the_arrow_and_damage_only_its_carried_weapon() {
    let max_damage = crate::item_durability::damage_and_max(&stack("minecraft:bow", 1))
        .unwrap()
        .1;
    for amount in [7, max_damage] {
        let (config, root) = config_with_birth_effects(serde_json::json!([
            {"requirements":entity_condition("minecraft:player"),
             "effect":{"type":"minecraft:play_sound", "sound":"minecraft:block.note_block.bass", "volume":1,"pitch":1}},
            {"requirements":entity_condition("minecraft:arrow"), "effect":{
                "type":"minecraft:all_of", "effects":[
                    {"type":"minecraft:ignite", "duration":100}, bell(),
                    {"type":"minecraft:change_item_damage", "amount":amount}
                ]
            }}
        ]));
        let mut fixture = archer_with_config(config);
        let bow = enchanted(
            &fixture,
            stack("minecraft:bow", 1),
            &[("minecraft:flame", 1)],
        );
        fixture.hold(HOTBAR_MENU_SLOT, bow);
        fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 2));
        let released = draw_and_release(&mut fixture, 20);
        let arrows = entities_of(&fixture, VANILLA_ENTITY_TYPE_ARROW_ID);
        assert_eq!(arrows.len(), 1);
        let arrow = &arrows[0];
        assert!(arrow.remaining_fire_ticks() > 0);
        let state = crate::entity::abstract_arrow::state(arrow.data()).unwrap();
        if amount == max_damage {
            assert!(state.weapon().is_none());
        } else {
            assert_eq!(damage(state.weapon().unwrap().summary()), amount);
        }
        assert_eq!(damage(&fixture.slot(HOTBAR_SLOT)), 1);
        assert_eq!(fixture.slot(HOTBAR_SLOT + 1).count, 1);
        assert!(sounds(&released, BOB).contains(&"minecraft:block.note_block.bell"));
        assert!(!sounds(&released, BOB).contains(&"minecraft:block.note_block.bass"));
        assert!(packets_to(&released, BOB).any(|packet| matches!(
            packet, ClientboundPlayPacket::AddEntity(add) if add.id == arrow.id()
        )));
        let next = fixture.idle();
        assert!([&released, &next].into_iter().any(|tick| packets_to(tick, BOB).any(|packet| {
            matches!(packet, ClientboundPlayPacket::SetEntityData(data)
                if data.id == arrow.id() && data.values.iter().any(|value| {
                    value.data_id == bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAGS_DATA_ID
                        && matches!(value.value, bbb_protocol::packets::EntityDataValueKind::Byte(flags)
                            if flags & bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_ON_FIRE != 0)
                }))
        })));
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn projectile_birth_inputs_keep_a_rods_wear_and_run_for_thrown_and_dispensed_items() {
    let (config, root) = config_with_birth_effects(serde_json::json!([
        {"requirements":entity_condition("minecraft:snowball"), "effect":bell()},
        {"requirements":entity_condition("minecraft:fishing_bobber"),
         "effect":{"type":"minecraft:change_item_damage", "amount":7}}
    ]));
    let mut fixture = archer_with_config(config);
    let snowball = enchanted(
        &fixture,
        stack("minecraft:snowball", 2),
        &[("minecraft:flame", 1)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, snowball);
    let thrown = fixture.use_item(InteractionHand::MainHand);
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 1);
    assert!(sounds(&thrown, BOB).contains(&"minecraft:block.note_block.bell"));
    let rod = enchanted(
        &fixture,
        stack("minecraft:fishing_rod", 1),
        &[("minecraft:flame", 1)],
    );
    fixture.hold(HOTBAR_MENU_SLOT, rod);
    fixture.use_item(InteractionHand::MainHand);
    assert_eq!(damage(&fixture.slot(HOTBAR_SLOT)), 7);
    fixture.command(r#"setblock 3 64 0 dispenser[facing=east]{Items:[{Slot:0b,id:"minecraft:snowball",count:1,components:{"minecraft:enchantments":{"minecraft:flame":1}}}]}"#);
    fixture.command("setblock 3 64 1 redstone_block");
    let ticks: Vec<_> = (0..6).map(|_| fixture.idle()).collect();
    assert!(ticks
        .iter()
        .any(|tick| sounds(tick, BOB).contains(&"minecraft:block.note_block.bell")));
    let snowballs = entities_of(
        &fixture,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNOWBALL_ID,
    );
    assert!(snowballs
        .iter()
        .any(|body| body.data().projectile_owner().is_none()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn projectile_birth_fireworks_use_the_ammo_effects_and_leave_crossbow_effects_on_arrows() {
    let (config, root) = config_with_birth_effects(serde_json::json!([{
        "requirements":entity_condition("minecraft:firework_rocket"), "effect":bell()
    }]));
    for enchanted_ammo in [false, true] {
        let mut fixture = archer_with_config(config.clone());
        let weapon = enchanted(
            &fixture,
            stack("minecraft:crossbow", 1),
            &[("minecraft:flame", 1)],
        );
        let ammo = if enchanted_ammo {
            enchanted(
                &fixture,
                stack("minecraft:firework_rocket", 1),
                &[("minecraft:flame", 1)],
            )
        } else {
            stack("minecraft:firework_rocket", 1)
        };
        fixture.hold(HOTBAR_MENU_SLOT, weapon);
        fixture.hold(OFFHAND_MENU_SLOT, ammo);
        fixture.use_item(InteractionHand::MainHand);
        for _ in 0..30 {
            fixture.idle();
        }
        fixture.action(PlayerActionKind::ReleaseUseItem);
        let fired = fixture.use_item(InteractionHand::MainHand);
        assert_eq!(
            entities_of(
                &fixture,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID
            )
            .len(),
            1
        );
        assert_eq!(
            sounds(&fired, BOB).contains(&"minecraft:block.note_block.bell"),
            enchanted_ammo
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
