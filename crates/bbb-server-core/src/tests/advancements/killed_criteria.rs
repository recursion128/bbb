use super::*;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

#[test]
fn pvp_kill_criteria_read_the_other_players_equipment() {
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::AdvancementElement;
    use serde_json::json;

    let mut startup = (*startup_generation()).clone();
    for (id, trigger, entity) in [
        (
            "test:killed_by_helmet",
            "minecraft:entity_killed_player",
            json!({"equipment":{"head":{"items":"minecraft:iron_helmet"}}}),
        ),
        (
            "test:killed_player",
            "minecraft:player_killed_entity",
            json!({"type":"minecraft:player"}),
        ),
    ] {
        startup.advancements.advancements.push(AdvancementElement {
            id: id.into(),
            source_pack: "test".into(),
            advancement: parse_advancement(&json!({
                "criteria":{"kill":{"trigger":trigger,"conditions":{
                    "entity":[{"condition":"minecraft:entity_properties","entity":"this",
                        "predicate":entity}]
                }}}
            }))
            .unwrap(),
        });
    }
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        datapacks: Some(Arc::new(startup)),
        ..operator_config(&[("alice", 1)])
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity alice armor.head with minecraft:iron_helmet",
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(1), 39)
            .item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_HELMET_ID)
    );
    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 100 minecraft:player_attack by alice",
    );
    assert_eq!(world.player_health_for_test(ConnectionId(2)), 0.0);
    for (player, id) in [(2, "test:killed_by_helmet"), (1, "test:killed_player")] {
        assert!(
            world.player_advancement_is_done(ConnectionId(player), id),
            "{id}"
        );
        assert!(
            update_advancements(&killed, player).iter().any(|update| {
                update.progress.iter().any(|progress| {
                    progress.id == id
                        && progress.criteria.iter().any(|criterion| {
                            criterion.name == "kill" && criterion.obtained_epoch_millis.is_some()
                        })
                })
            }),
            "{id}"
        );
    }
}

#[test]
fn a_players_melee_kill_matches_captured_entity_components() {
    use crate::tests::parse_advancement;
    use bbb_facts::temperature_animal_variants::FrogModelVariant;
    use bbb_facts::zombie_nautilus_variants::ZombieNautilusModelVariant;
    use bbb_protocol::builtin_enum_component_values::{
        builtin_enum_component_value_name, VANILLA_AXOLOTL_VARIANT_BLUE_ID,
        VANILLA_AXOLOTL_VARIANT_LUCY_ID, VANILLA_DYE_COLOR_RED_ID, VANILLA_LLAMA_VARIANT_CREAMY_ID,
        VANILLA_LLAMA_VARIANT_GRAY_ID, VANILLA_PARROT_VARIANT_GRAY_ID,
        VANILLA_PARROT_VARIANT_RED_BLUE_ID,
    };
    use bbb_protocol::data_component_types::{
        vanilla_data_component_type,
        VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID as AXOLOTL_VARIANT,
        VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID as LLAMA_VARIANT,
        VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID as PARROT_VARIANT,
    };
    use bbb_server_datapack::advancement::AdvancementElement;
    use serde_json::json;

    const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
    let chicken_sound =
        bbb_protocol::animal_sound_variant_types::ChickenSoundVariantIdentity::Picky.resource_id();
    let pig_sound =
        bbb_protocol::animal_sound_variant_types::PigSoundVariantIdentity::Big.resource_id();
    let enum_components = |component, variant| {
        json!({(vanilla_data_component_type(component).unwrap().resource_id):
            builtin_enum_component_value_name(component, variant).unwrap()})
    };
    let rows = [
        (
            "test:kill_large_salmon",
            "minecraft:salmon",
            true,
            json!({"minecraft:salmon/size":"large"}),
        ),
        (
            "test:kill_small_salmon",
            "minecraft:salmon",
            false,
            json!({"minecraft:salmon/size":"small"}),
        ),
        (
            "test:kill_clayfish",
            "minecraft:tropical_fish",
            true,
            json!({
                "minecraft:tropical_fish/pattern":"clayfish",
                "minecraft:tropical_fish/base_color":"blue",
                "minecraft:tropical_fish/pattern_color":"red"
            }),
        ),
        (
            "test:kill_kob",
            "minecraft:tropical_fish",
            false,
            json!({
                "minecraft:tropical_fish/pattern":"kob",
                "minecraft:tropical_fish/base_color":"blue",
                "minecraft:tropical_fish/pattern_color":"red"
            }),
        ),
        (
            "test:kill_cold_chicken",
            "minecraft:chicken",
            true,
            json!({
                "minecraft:chicken/variant":"minecraft:cold",
                "minecraft:chicken/sound_variant":chicken_sound
            }),
        ),
        (
            "test:kill_warm_chicken",
            "minecraft:chicken",
            false,
            json!({
                "minecraft:chicken/variant":"minecraft:warm",
                "minecraft:chicken/sound_variant":chicken_sound
            }),
        ),
        (
            "test:kill_cold_pig",
            "minecraft:pig",
            true,
            json!({
                "minecraft:pig/variant":"minecraft:cold",
                "minecraft:pig/sound_variant":pig_sound
            }),
        ),
        (
            "test:kill_warm_pig",
            "minecraft:pig",
            false,
            json!({
                "minecraft:pig/variant":"minecraft:warm",
                "minecraft:pig/sound_variant":pig_sound
            }),
        ),
        (
            "test:kill_gray_llama",
            "minecraft:llama",
            true,
            enum_components(LLAMA_VARIANT, VANILLA_LLAMA_VARIANT_GRAY_ID),
        ),
        (
            "test:kill_creamy_llama",
            "minecraft:llama",
            false,
            enum_components(LLAMA_VARIANT, VANILLA_LLAMA_VARIANT_CREAMY_ID),
        ),
        (
            "test:kill_cold_frog",
            "minecraft:frog",
            true,
            json!({"minecraft:frog/variant":FrogModelVariant::Cold.resource_id()}),
        ),
        (
            "test:kill_warm_frog",
            "minecraft:frog",
            false,
            json!({"minecraft:frog/variant":FrogModelVariant::Warm.resource_id()}),
        ),
        (
            "test:kill_warm_zombie_nautilus",
            "minecraft:zombie_nautilus",
            true,
            json!({"minecraft:zombie_nautilus/variant":ZombieNautilusModelVariant::Warm.resource_id()}),
        ),
        (
            "test:kill_temperate_zombie_nautilus",
            "minecraft:zombie_nautilus",
            false,
            json!({"minecraft:zombie_nautilus/variant":ZombieNautilusModelVariant::Temperate.resource_id()}),
        ),
        (
            "test:kill_blue_axolotl",
            "minecraft:axolotl",
            true,
            enum_components(AXOLOTL_VARIANT, VANILLA_AXOLOTL_VARIANT_BLUE_ID),
        ),
        (
            "test:kill_lucy_axolotl",
            "minecraft:axolotl",
            false,
            enum_components(AXOLOTL_VARIANT, VANILLA_AXOLOTL_VARIANT_LUCY_ID),
        ),
        (
            "test:kill_gray_parrot",
            "minecraft:parrot",
            true,
            enum_components(PARROT_VARIANT, VANILLA_PARROT_VARIANT_GRAY_ID),
        ),
        (
            "test:kill_red_blue_parrot",
            "minecraft:parrot",
            false,
            enum_components(PARROT_VARIANT, VANILLA_PARROT_VARIANT_RED_BLUE_ID),
        ),
        (
            "test:kill_colored_shulker",
            "minecraft:shulker",
            true,
            json!({"minecraft:shulker/color":"red"}),
        ),
        (
            "test:kill_colored_shulker_wrong",
            "minecraft:shulker",
            false,
            json!({"minecraft:shulker/color":"blue"}),
        ),
        (
            "test:kill_uncolored_shulker",
            "minecraft:shulker",
            true,
            json!({"minecraft:shulker/color":"red"}),
        ),
        (
            "test:kill_profile_mannequin",
            "minecraft:mannequin",
            true,
            json!({"minecraft:profile":{"name":"capture_probe"}}),
        ),
        (
            "test:kill_wrong_profile_mannequin",
            "minecraft:mannequin",
            false,
            json!({"minecraft:profile":{"name":"other_probe"}}),
        ),
    ];
    let mut startup = (*startup_generation()).clone();
    for (id, species, _, components) in &rows {
        let entity_conditions = if *id == "test:kill_uncolored_shulker" {
            json!([
                {"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"type":species}},
                {"condition":"minecraft:inverted","term":{
                    "condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"components":components}
                }}
            ])
        } else {
            json!([{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"type":species,"components":components}}])
        };
        startup.advancements.advancements.push(AdvancementElement {
            id: (*id).into(),
            source_pack: "test".into(),
            advancement: parse_advancement(&json!({
                "criteria":{"kill":{
                    "trigger":"minecraft:player_killed_entity",
                    "conditions":{"entity":entity_conditions}
                }}
            }))
            .unwrap(),
        });
    }
    let mut config = operator_config(&[("alice", 1)]);
    config.game_type = GameType::Survival;
    config.datapacks = Some(Arc::new(startup));
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config.clone());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
    let obtained = |tick: &TickResult, id: &str| {
        update_advancements(tick, 1).iter().any(|update| {
            update.progress.iter().any(|progress| {
                progress.id == id
                    && progress.criteria.iter().any(|criterion| {
                        criterion.name == "kill" && criterion.obtained_epoch_millis.is_some()
                    })
            })
        })
    };
    for (species, variant_fields, criterion_prefix) in [
        (
            "minecraft:salmon",
            vec![("type", NbtSummaryValue::String("large".into()))],
            "",
        ),
        (
            "minecraft:tropical_fish",
            vec![("Variant", NbtSummaryValue::Int(0x0e0b0501))],
            "",
        ),
        (
            "minecraft:chicken",
            vec![
                ("variant", NbtSummaryValue::String("minecraft:cold".into())),
                (
                    "sound_variant",
                    NbtSummaryValue::String(chicken_sound.into()),
                ),
            ],
            "",
        ),
        (
            "minecraft:pig",
            vec![
                ("variant", NbtSummaryValue::String("minecraft:cold".into())),
                ("sound_variant", NbtSummaryValue::String(pig_sound.into())),
            ],
            "",
        ),
        (
            "minecraft:llama",
            vec![(
                "Variant",
                NbtSummaryValue::Int(VANILLA_LLAMA_VARIANT_GRAY_ID),
            )],
            "",
        ),
        (
            "minecraft:frog",
            vec![(
                "variant",
                NbtSummaryValue::String(FrogModelVariant::Cold.resource_id().into()),
            )],
            "",
        ),
        (
            "minecraft:zombie_nautilus",
            vec![(
                "variant",
                NbtSummaryValue::String(ZombieNautilusModelVariant::Warm.resource_id().into()),
            )],
            "",
        ),
        (
            "minecraft:axolotl",
            vec![(
                "Variant",
                NbtSummaryValue::Int(VANILLA_AXOLOTL_VARIANT_BLUE_ID),
            )],
            "",
        ),
        (
            "minecraft:parrot",
            vec![(
                "Variant",
                NbtSummaryValue::Int(VANILLA_PARROT_VARIANT_GRAY_ID),
            )],
            "",
        ),
        (
            "minecraft:shulker",
            vec![(
                "Color",
                NbtSummaryValue::Byte(VANILLA_DYE_COLOR_RED_ID as i8),
            )],
            "test:kill_colored_shulker",
        ),
        (
            "minecraft:shulker",
            vec![("Color", NbtSummaryValue::Byte(16))],
            "test:kill_uncolored_shulker",
        ),
        (
            "minecraft:mannequin",
            vec![(
                "profile",
                NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                    name: "name".into(),
                    value: NbtSummaryValue::String("capture_probe".into()),
                }]),
            )],
            "",
        ),
    ] {
        for _ in 0..15 {
            world.tick(receiver.drain());
        }
        world.set_player_ground_for_test(ConnectionId(1), true, 0.0);
        let kind =
            bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(species).unwrap();
        let position = world.publication().players[0].position;
        let (id, _) = world.spawn_stored_entity_for_test(
            DIM,
            kind,
            Vec3d {
                z: position.z + 1.5,
                ..position
            },
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(kind).unwrap(),
            ),
        );
        let NbtSummaryValue::Compound(mut record) =
            world.stored_entity_command_data_for_test(DIM, id).unwrap()
        else {
            panic!("the entity saves a compound")
        };
        for (name, value) in variant_fields.into_iter().chain([
            ("Health", NbtSummaryValue::Float(1.0_f32.to_bits())),
            ("NoAI", NbtSummaryValue::Byte(1)),
            ("NoGravity", NbtSummaryValue::Byte(1)),
        ]) {
            record.retain(|entry| entry.name != name);
            record.push(bbb_protocol::NbtSummaryEntry {
                name: name.into(),
                value,
            });
        }
        world.load_stored_entity_record_for_test(DIM, id, &record);
        if criterion_prefix == "test:kill_uncolored_shulker" {
            let Some(NbtSummaryValue::Compound(record)) =
                world.stored_entity_command_data_for_test(DIM, id)
            else {
                panic!("the shulker saves a compound")
            };
            assert!(record
                .iter()
                .any(|entry| entry.name == "Color" && entry.value == NbtSummaryValue::Byte(16)));
        }
        let attack = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Attack(AttackEntity { entity_id: id }),
        );
        assert_eq!(
            world
                .stored_entity_for_test(DIM, id)
                .unwrap()
                .living()
                .unwrap()
                .health(),
            0.0
        );
        for (advancement, target, expected, _) in &rows {
            if *target == species && advancement.starts_with(criterion_prefix) {
                assert_eq!(
                    world.player_advancement_is_done(ConnectionId(1), advancement),
                    *expected,
                    "{advancement}"
                );
                assert_eq!(obtained(&attack, advancement), *expected, "{advancement}");
            }
        }
    }
    let written = world.player_advancements_to_data(ConnectionId(1));
    for (id, _, expected, _) in &rows {
        assert_eq!(
            written_criteria(&written, id).unwrap_or_default(),
            if *expected {
                vec!["kill".to_owned()]
            } else {
                Vec::new()
            },
            "{id}"
        );
    }
    let mut restored = spawn_pinned_world(config);
    restored.seed_restored_player_advancements(profile("alice", 1).uuid, written);
    let (joined, handshake) = join_loaded_ticks(&sender, &receiver, &mut restored, 1, "alice", 1);
    for (id, _, expected, _) in &rows {
        assert_eq!(
            restored.player_advancement_is_done(ConnectionId(1), id),
            *expected,
            "{id}"
        );
        assert_eq!(
            obtained(&joined, id) || obtained(&handshake, id),
            *expected,
            "{id}"
        );
    }
}

#[test]
fn a_captains_kill_requires_both_full_banner_components() {
    const ADVANCEMENT: &str = "minecraft:adventure/voluntary_exile";
    let (sender, receiver, mut world) = built_in_advancement_world();
    let startup = startup_generation();
    let items = FrozenDatapackRegistryLookup::from_startup(&startup);
    let banner =
        crate::entity::general_living::spawn_finalize_equipment::ominous_banner_stack(&items);
    let wrong_patterns = banner
        .clone()
        .with_component_nbt(
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            Some(&NbtSummaryValue::List(Vec::new())),
            &items,
        )
        .unwrap();
    let wrong_name = banner
        .clone()
        .with_component_nbt(VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID, None, &items)
        .unwrap();
    for (head, expected) in [(wrong_patterns, false), (wrong_name, false), (banner, true)] {
        let mut data =
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_PILLAGER_ID).unwrap();
        data.set_no_ai(true);
        data.mob
            .as_mut()
            .unwrap()
            .set_guaranteed_drop(EquipmentSlot::Head);
        let position = world.publication().players[0].position;
        let (id, uuid) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
            Vec3d {
                x: position.x + 1.5,
                ..position
            },
            crate::entity::EntityData::GeneralLiving(data),
        );
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
            .living_mut()
            .unwrap()
            .set_item_slot(EquipmentSlot::Head, Some(head));
        let result = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("damage {uuid} 100 minecraft:generic by alice"),
        );
        assert_eq!(
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .unwrap()
                .living()
                .unwrap()
                .health(),
            0.0
        );
        assert!(world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::Head)
            .is_none());
        assert_eq!(
            world.player_advancement_is_done(ConnectionId(1), ADVANCEMENT),
            expected
        );
        assert_eq!(
            update_advancements(&result, 1)
                .iter()
                .any(|update| update.progress.iter().any(|progress| {
                    progress.id == ADVANCEMENT
                        && progress.criteria.iter().any(|criterion| {
                            criterion.name == "voluntary_exile"
                                && criterion.obtained_epoch_millis.is_some()
                        })
                })),
            expected
        );
    }
    assert_eq!(
        written_criteria(
            &world.player_advancements_to_data(ConnectionId(1)),
            ADVANCEMENT
        ),
        Some(vec!["voluntary_exile".into()])
    );
}
