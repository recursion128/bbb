use super::*;

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
    let NbtSummaryValue::Compound(entries) = value else {
        panic!("expected a component compound: {value:?}");
    };
    &entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap()
        .value
}

fn canonical(value: &mut NbtSummaryValue) {
    match value {
        NbtSummaryValue::Compound(entries) => {
            entries.sort_by(|a, b| a.name.cmp(&b.name));
            for entry in entries {
                canonical(&mut entry.value);
            }
        }
        NbtSummaryValue::List(values) => values.iter_mut().for_each(canonical),
        _ => {}
    }
}

#[test]
fn loaded_descriptions_reach_enchant_and_datapack_command_feedback() {
    let dir = std::env::temp_dir().join(format!("bbb-enchantment-description-{}", Uuid::new_v4()));
    let pack = dir.join("datapacks/test");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"\ud800","min_format":[101,1],"max_format":[101,1]}}"#,
    )
    .unwrap();
    let fixtures = [
        (
            "sharpness",
            serde_json::json!("Literal enchantment"),
            "diamond_sword",
            2,
            r#"{text:"Literal enchantment",color:"gray",extra:[" ",{translate:"enchantment.level.2"}]}"#,
        ),
        (
            "unbreaking",
            serde_json::json!({"text":"Styled", "bold":true,"extra":[{"text":" child","italic":true,"color":"blue"}]}),
            "diamond_sword",
            1,
            r#"{text:"Styled",bold:1b,color:"gray",extra:[{text:" child",italic:1b,color:"blue"}," ",{translate:"enchantment.level.1"}]}"#,
        ),
        (
            "binding_curse",
            serde_json::json!({"translate":"custom.enchantment","color":"gold","italic":true,"with":[{"text":"argument","bold":true,"extra":[{"translate":"custom.nested"}]}],"extra":[{"text":" tail","underlined":true}]}),
            "diamond_chestplate",
            1,
            r#"{translate:"custom.enchantment",color:"gold",italic:1b,with:[{text:"argument",bold:1b,extra:[{translate:"custom.nested"}]}],extra:[{text:" tail",underlined:1b}]}"#,
        ),
    ];
    for (name, description, _, _, _) in &fixtures {
        let index = crate::enchantment_facts::builtin_enchantment_registry()
            .iter()
            .position(|id| id == &format!("minecraft:{name}"))
            .unwrap();
        let mut document =
            serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
                .unwrap()
                .clone();
        document["description"] = description.clone();
        std::fs::write(
            pack.join(format!("data/minecraft/enchantment/{name}.json")),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
    }
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:efficiency")
        .unwrap();
    let mut document = crate::enchantment_facts::builtin_enchantment_documents()[index].clone();
    let bbb_registry_codec::RegistryJsonValue::Object(fields) = &mut document else {
        panic!("enchantment object");
    };
    fields
        .iter_mut()
        .find(|(name, _)| name == "description")
        .unwrap()
        .1 = bbb_registry_codec::RegistryJsonValue::String(vec![0xd800].into());
    std::fs::write(
        pack.join("data/minecraft/enchantment/efficiency.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(dir.join("datapacks")),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    let mut config = operator_config(&[("op", 1)]);
    config.datapacks = Some(Arc::new(startup));
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    for (name, _, item, level, expected) in fixtures {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("item replace entity op weapon.mainhand with {item}"),
        );
        let tick = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("enchant op {name} {level}"),
        );
        let feedback = tick
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SystemChat(chat),
                } if chat
                    .content
                    .text()
                    .contains("commands.enchant.success.single") =>
                {
                    Some(chat.content.component())
                }
                _ => None,
            })
            .expect("the enchant command succeeds");
        let NbtSummaryValue::List(args) = field(feedback, "with") else {
            panic!("feedback has arguments");
        };
        let mut actual = args[0].clone();
        let mut expected = bbb_protocol::parse_snbt_compound(expected).unwrap();
        canonical(&mut actual);
        canonical(&mut expected);
        assert_eq!(actual, expected, "{name}");
        let inventory = &world.publication().players[0].inventory;
        let stack = &inventory.slots[usize::from(inventory.selected_slot)];
        assert_eq!(stack.component_patch.enchantments.len(), 1);
        assert_eq!(stack.component_patch.enchantments[0].level, level);
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op weapon.mainhand with diamond_pickaxe",
    );
    let tick = chat_command(&sender, &receiver, &mut world, 1, "enchant op efficiency 2");
    let feedback = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SystemChat(chat),
            } if chat
                .content
                .text()
                .contains("commands.enchant.success.single") =>
            {
                Some(chat.content.component())
            }
            _ => None,
        })
        .expect("the raw description enchantment succeeds");
    let NbtSummaryValue::List(args) = field(feedback, "with") else {
        panic!("feedback arguments");
    };
    let NbtSummaryValue::String(raw) = field(&args[0], "text") else {
        panic!("raw text component");
    };
    assert_eq!(raw.units(), [0xd800]);

    let tick = chat_command(&sender, &receiver, &mut world, 1, "datapack list enabled");
    let feedback = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SystemChat(chat),
            } => Some(chat.content.component()),
            _ => None,
        })
        .expect("datapack list sends feedback");
    fn contains_raw(value: &NbtSummaryValue) -> bool {
        match value {
            NbtSummaryValue::String(value) => value.units() == [0xd800],
            NbtSummaryValue::List(values) => values.iter().any(contains_raw),
            NbtSummaryValue::Compound(entries) => {
                entries.iter().any(|entry| contains_raw(&entry.value))
            }
            _ => false,
        }
    }
    assert!(
        contains_raw(feedback),
        "the pack hover retains its raw description"
    );
}
