//! `LivingEntity.actuallyHurt` on stored mobs: worn enchantments, the weapon's
//! armour effectiveness and the wolf's body armour (`LivingEntity.java:1849-1940`).

use super::*;

use bbb_protocol::equipment_slots::EquipmentSlot;

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

struct Fixture {
    world: ServerWorld,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
}

impl Fixture {
    fn new() -> Self {
        Self::with_config(datapack_config())
    }

    fn with_config(config: ServerWorldConfig) -> Self {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type: GameType::Survival,
            admin_lists: operator_config(&[("alice", 1)]).admin_lists,
            ..config
        })
        .unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        Self {
            world,
            sender,
            receiver,
        }
    }

    fn run(&mut self, command: &str) {
        chat_command(&self.sender, &self.receiver, &mut self.world, 1, command);
        self.world.tick(self.receiver.drain());
    }

    /// A NoAI mob at `offset` (x, z) from the player.
    fn spawn_mob(&mut self, type_id: i32, offset: [f64; 2]) -> i32 {
        let player = self.world.publication().players[0].position;
        let position = Vec3d {
            x: player.x + offset[0],
            z: player.z + offset[1],
            ..player
        };
        let (id, _) = self.world.spawn_stored_entity_for_test(
            OVERWORLD,
            type_id,
            position,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(type_id).unwrap(),
            ),
        );
        self.world.load_stored_entity_record_for_test(
            OVERWORLD,
            id,
            &[NbtSummaryEntry {
                name: "NoAI".into(),
                value: NbtSummaryValue::Byte(1),
            }],
        );
        self.world
            .stored_entity_mut_for_test(OVERWORLD, id)
            .unwrap()
            .set_pos(position);
        id
    }

    fn health(&self, id: i32) -> f32 {
        self.world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .living()
            .unwrap()
            .health()
    }

    /// Each hit lands on full health outside the last one's window.
    fn hit(&mut self, id: i32, command: &str) -> f32 {
        let full = self
            .world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .living()
            .unwrap()
            .max_health();
        self.world
            .stored_entity_mut_for_test(OVERWORLD, id)
            .unwrap()
            .living_mut()
            .unwrap()
            .set_health(full);
        for _ in 0..11 {
            self.world.tick(Vec::new());
        }
        self.run(command);
        full - self.health(id)
    }

    fn worn_damage(&self, id: i32, slot: EquipmentSlot) -> Option<i32> {
        let stack = self
            .world
            .stored_entity_for_test(OVERWORLD, id)?
            .living()?
            .item_by_slot(slot)?
            .clone();
        stack.summary().component_patch.damage
    }
}

fn assert_close(taken: f32, expected: f32) {
    assert!(
        (taken - expected).abs() < 1e-4,
        "took {taken}, expected {expected}"
    );
}

/// `CombatRules.getDamageAfterAbsorb` (`CombatRules.java:16-32`) with the
/// weapon's modified fraction.
fn after_armor(damage: f32, armor: f32, fraction_bonus: f32) -> f32 {
    let real_armor = (armor - damage / 2.0).max(armor * 0.2).min(20.0);
    damage * (1.0 - (real_armor / 25.0 + fraction_bonus).clamp(0.0, 1.0))
}

#[test]
fn player_and_stored_damage_conditions_read_the_named_world_clock() {
    let mut config = datapack_config();
    let mut document: serde_json::Value = serde_json::from_slice(
        &config
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", "minecraft:protection")
            .unwrap()
            .source_json,
    )
    .unwrap();
    document["effects"] = serde_json::json!({"minecraft:damage_protection":[{
        "effect":{"type":"minecraft:add","value":10},
        "requirements":{"condition":"minecraft:all_of","terms":[
            {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107},
            {"condition":"minecraft:time_check","clock":"minecraft:the_end",
                "period":100,"value":7}
        ]}
    }]});
    let dir =
        std::env::temp_dir().join(format!("bbb-stored-enchantment-clocks-{}", Uuid::new_v4()));
    let pack = dir.join("datapacks/test");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"stored enchantment clocks","pack_format":1000,"supported_formats":[0,10000]}}"#,
    ).unwrap();
    std::fs::write(
        pack.join("data/minecraft/enchantment/protection.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    config.datapacks = Some(Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(dir.join("datapacks")),
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    ));
    std::fs::remove_dir_all(dir).unwrap();
    let mut fixture = Fixture::with_config(config.clone());
    let command = |fixture: &mut Fixture, input: &str, feedback: &str| {
        let mut tick = chat_command(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            1,
            input,
        );
        assert!(
            system_chats(&tick)
                .iter()
                .any(|(connection, line)| *connection == ConnectionId(1) && line.contains(feedback)),
            "{input}: {:?}",
            system_chats(&tick)
        );
        tick.events
            .extend(fixture.world.tick(fixture.receiver.drain()).events);
        tick
    };
    for (input, feedback) in [
        ("time of minecraft:the_end pause", "commands.time.pause"),
        ("time of minecraft:overworld pause", "commands.time.pause"),
        (
            "time of minecraft:the_end set 107",
            "commands.time.set.absolute",
        ),
        (
            "time of minecraft:overworld set 6000",
            "commands.time.set.absolute",
        ),
    ] {
        command(&mut fixture, input, feedback);
    }
    let husks = [[0.0, 4.0], [4.0, 0.0]].map(|offset| {
        fixture.spawn_mob(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HUSK_ID,
            offset,
        )
    });
    command(
        &mut fixture,
        "item replace entity @e[type=husk] armor.chest with iron_chestplate[enchantments={protection:1}]",
        "commands.item.entity.set.success.multiple",
    );
    for (husk, time, multiplier) in [(husks[0], 107, 0.6), (husks[1], 108, 1.0)] {
        command(
            &mut fixture,
            &format!("time of minecraft:the_end set {time}"),
            "commands.time.set.absolute",
        );
        assert_eq!(
            fixture.world.clock_total_ticks("minecraft:the_end"),
            Some(time)
        );
        assert_eq!(
            fixture.world.clock_total_ticks("minecraft:overworld"),
            Some(6000)
        );
        let uuid = fixture
            .world
            .stored_entity_for_test(OVERWORLD, husk)
            .unwrap()
            .uuid();
        let full = fixture.health(husk);
        assert_eq!(full, 20.0);
        command(
            &mut fixture,
            &format!("damage {uuid} 10 minecraft:cactus"),
            "commands.damage.success",
        );
        assert_close(
            full - fixture.health(husk),
            after_armor(10.0, 8.0, 0.0) * multiplier,
        );
    }
    for (time, multiplier) in [(107, 0.6), (108, 1.0)] {
        let mut fixture = Fixture::with_config(config.clone());
        set_paused_test_clocks(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            time,
            0,
        );
        command(&mut fixture,
            "item replace entity alice armor.chest with iron_chestplate[enchantments={protection:1}]",
            "commands.item.entity.set.success.single");
        let result = command(
            &mut fixture,
            "damage alice 10 minecraft:cactus",
            "commands.damage.success",
        );
        let health = 20.0 - after_armor(10.0, 6.0, 0.0) * multiplier;
        assert_close(
            fixture.world.player_health_for_test(ConnectionId(1)),
            health,
        );
        assert!(result.events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetHealth(update) }
                if (update.health - health).abs() < 1e-4)));
    }

    use crate::tests::entities::{console_command, console_command_answer};

    let mut config = datapack_config();
    let mut document: serde_json::Value = serde_json::from_slice(
        &config
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", "minecraft:protection")
            .unwrap()
            .source_json,
    )
    .unwrap();
    let vehicle_condition = |item| {
        serde_json::json!({
            "condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"type":"minecraft:player","vehicle":{
                "type":"minecraft:oak_chest_boat",
                "slots":{"container.*":{"items":item,"count":1}}
            }}
        })
    };
    document["effects"] = serde_json::json!({"minecraft:damage_protection":[
        {"effect":{"type":"minecraft:add","value":5}},
        {"effect":{"type":"minecraft:add","value":10},
            "requirements":vehicle_condition("minecraft:diamond")},
        {"effect":{"type":"minecraft:add","value":5},
            "requirements":vehicle_condition("minecraft:stone")}
    ]});
    let cargo = serde_json::json!({"type":"minecraft:chest","pools":[{
        "rolls":1,"conditions":[{"condition":"minecraft:inverted","term":{
            "condition":"minecraft:entity_properties","entity":"this","predicate":{}
        }}],"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]
    }]});
    let root = std::env::temp_dir().join(format!(
        "bbb-world-protection-vehicle-slots-{}",
        Uuid::new_v4()
    ));
    let pack = root.join("cargo");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"World protection vehicle slots","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for (path, value) in [
        ("data/minecraft/enchantment/protection.json", document),
        ("data/test/loot_table/pending_vehicle.json", cargo),
    ] {
        std::fs::write(pack.join(path), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    config.datapacks = Some(Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(root.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    ));
    std::fs::remove_dir_all(root).unwrap();
    let mut fixture = Fixture::with_config(config);
    let at = fixture.world.publication().players[0].position;
    let player_uuid = fixture.world.publication().players[0].uuid;
    assert_eq!(player_uuid, Uuid::from_u128(1));
    for (index, command) in [
        "item replace entity alice armor.chest with iron_chestplate[enchantments={protection:1}]".to_owned(),
        format!("summon minecraft:oak_chest_boat {} {} {} {{UUID:[I;0,0,0,4001],NoGravity:1b,LootTable:'test:pending_vehicle',LootTableSeed:37L}}", at.x + 2.0, at.y, at.z),
        "ride alice mount 00000000-0000-0000-0000-000000000fa1".to_owned(),
    ].into_iter().enumerate() {
        let tick = console_command(&fixture.sender, &fixture.receiver, &mut fixture.world,
            index as u64 + 1, &command);
        assert_eq!(console_command_answer(&tick).0, Some(1), "{command}");
    }
    let boat_uuid = Uuid::from_u128(4001);
    let boat = fixture
        .world
        .live_entity_id_for_uuid_for_test(OVERWORLD, boat_uuid)
        .unwrap();
    assert_eq!(
        fixture
            .world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat)
    );
    let boat_body = fixture
        .world
        .stored_entity_for_test(OVERWORLD, boat)
        .unwrap();
    assert_eq!(boat_body.uuid(), boat_uuid);
    assert!(crate::entity::item_slots::container_has_loot(boat_body));
    for slot in 0..crate::entity::item_slots::container_size(boat_body).unwrap() {
        assert!(crate::entity::item_slots::read(boat_body, slot as i32)
            .unwrap()
            .is_some_and(|stack| stack.is_empty()));
    }
    let NbtSummaryValue::Compound(before) = fixture
        .world
        .stored_entity_command_data_for_test(OVERWORLD, boat)
        .unwrap()
    else {
        panic!("the pending boat has canonical entity data");
    };
    assert!(before.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::String("test:pending_vehicle".into())));
    assert!(before
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(37)));
    assert!(!before.iter().any(|entry| entry.name == "Items"));
    let full = fixture.world.player_health_for_test(ConnectionId(1));
    let tick = console_command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        4,
        "damage alice 10 minecraft:cactus",
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    let health = full - after_armor(10.0, 6.0, 0.0) * (1.0 - 15.0 / 25.0);
    assert_close(
        fixture.world.player_health_for_test(ConnectionId(1)),
        health,
    );
    assert!(tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetHealth(update) }
            if (update.health - health).abs() < 1e-4)));
    assert_eq!(fixture.world.publication().players[0].uuid, player_uuid);
    assert_eq!(
        fixture
            .world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat)
    );
    let boat_body = fixture
        .world
        .stored_entity_for_test(OVERWORLD, boat)
        .unwrap();
    assert_eq!(boat_body.uuid(), boat_uuid);
    assert_eq!(boat_body.removal_reason(), None);
    assert!(!crate::entity::item_slots::container_has_loot(boat_body));
    let cargo: Vec<_> = (0..crate::entity::item_slots::container_size(boat_body).unwrap())
        .filter_map(|slot| crate::entity::item_slots::read(boat_body, slot as i32).unwrap())
        .filter(|stack| !stack.is_empty())
        .map(|stack| (stack.item_resource_id().unwrap().to_owned(), stack.count()))
        .collect();
    assert_eq!(cargo, vec![("minecraft:diamond".into(), 1)]);
    let NbtSummaryValue::Compound(record) = fixture
        .world
        .stored_entity_command_data_for_test(OVERWORLD, boat)
        .unwrap()
    else {
        panic!("the unpacked boat has canonical entity data");
    };
    assert!(!record.iter().any(|entry| entry.name == "LootTable"));
    let Some(NbtSummaryValue::List(items)) = record
        .iter()
        .find(|entry| entry.name == "Items")
        .map(|entry| &entry.value)
    else {
        panic!("the unpacked boat saves its cargo");
    };
    let [NbtSummaryValue::Compound(stack)] = items.as_slice() else {
        panic!("the boat saves exactly one cargo stack: {items:?}");
    };
    assert!(stack.iter().any(|entry| entry.name == "id"
        && entry.value == NbtSummaryValue::String("minecraft:diamond".into())));
    assert!(stack
        .iter()
        .any(|entry| entry.name == "count" && entry.value == NbtSummaryValue::Int(1)));
}

/// A husk (base armour 2, no daylight burn) in an iron chestplate: armour,
/// Protection IV, Frost Walker's immunity, the helmet quarter and a Breach
/// weapon, with no armour wear on a mob.
#[test]
fn worn_enchantments_and_the_weapon_reduce_a_stored_mobs_damage() {
    let mut fixture = Fixture::new();
    let husk = fixture.spawn_mob(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HUSK_ID,
        [0.0, 4.0],
    );
    fixture.spawn_mob(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [4.0, 0.0],
    );
    fixture.run("item replace entity @e[type=husk] armor.chest with iron_chestplate");

    let taken = fixture.hit(husk, "damage @e[type=husk,limit=1] 10 minecraft:cactus");
    assert_close(taken, after_armor(10.0, 8.0, 0.0));
    assert_eq!(fixture.worn_damage(husk, EquipmentSlot::Chest), None);

    fixture.run(
        "item replace entity @e[type=husk] armor.chest with iron_chestplate[enchantments={protection:4}]",
    );
    let taken = fixture.hit(husk, "damage @e[type=husk,limit=1] 10 minecraft:cactus");
    assert_close(taken, after_armor(10.0, 8.0, 0.0) * (1.0 - 4.0 / 25.0));

    fixture.run(
        "item replace entity @e[type=husk] armor.feet with leather_boots[enchantments={frost_walker:2}]",
    );
    let taken = fixture.hit(husk, "damage @e[type=husk,limit=1] 5 minecraft:hot_floor");
    assert_close(taken, 0.0);

    // Breach IV takes 0.6 off the armour fraction of the zombie's own swing.
    fixture.run("item replace entity @e[type=husk] armor.chest with iron_chestplate");
    fixture.run(
        "item replace entity @e[type=zombie] weapon.mainhand with iron_sword[enchantments={breach:4}]",
    );
    let taken = fixture.hit(
        husk,
        "damage @e[type=husk,limit=1] 10 minecraft:mob_attack by @e[type=zombie,limit=1]",
    );
    assert_close(taken, after_armor(10.0, 9.0, -0.6));

    // A falling anvil takes a quarter off before the armour pass; the boots
    // and helmet bring the armour to 11.
    fixture.run("item replace entity @e[type=husk] armor.head with iron_helmet");
    let taken = fixture.hit(
        husk,
        "damage @e[type=husk,limit=1] 8 minecraft:falling_anvil",
    );
    assert_close(taken, after_armor(6.0, 11.0, 0.0));

    // Resistance II takes 10/25 of what the armour let through, credited to
    // the player who caused the hit (`LivingEntity.java:1888-1895`).
    fixture.run("effect give @e[type=husk] minecraft:resistance 60 1");
    let taken = fixture.hit(
        husk,
        "damage @e[type=husk,limit=1] 10 minecraft:player_attack by alice",
    );
    let armored = after_armor(10.0, 11.0, 0.0);
    assert_close(taken, armored * 15.0 / 25.0);
    assert_eq!(
        fixture.world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:damage_dealt_resisted"),
        ),
        ((armored - armored * 15.0 / 25.0) * 10.0).round() as i32
    );

    use crate::tests::entities::{console_command, console_command_answer};

    let mut config = datapack_config();
    let mut document: serde_json::Value = serde_json::from_slice(
        &config
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", "minecraft:protection")
            .unwrap()
            .source_json,
    )
    .unwrap();
    let vehicle_condition = |item| {
        serde_json::json!({
            "condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"type":"minecraft:cow","vehicle":{
                "type":"minecraft:oak_chest_boat",
                "slots":{"container.*":{"items":item,"count":1}}
            }}
        })
    };
    document["effects"] = serde_json::json!({"minecraft:damage_protection":[
        {"effect":{"type":"minecraft:add","value":5}},
        {"effect":{"type":"minecraft:add","value":10},
            "requirements":vehicle_condition("minecraft:diamond")},
        {"effect":{"type":"minecraft:add","value":5},
            "requirements":vehicle_condition("minecraft:stone")}
    ]});
    let cargo = serde_json::json!({"type":"minecraft:chest","pools":[{
        "rolls":1,"conditions":[{"condition":"minecraft:inverted","term":{
            "condition":"minecraft:entity_properties","entity":"this","predicate":{}
        }}],"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]
    }]});
    let root =
        std::env::temp_dir().join(format!("bbb-protection-vehicle-slots-{}", Uuid::new_v4()));
    let pack = root.join("cargo");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"Protection vehicle slots","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for (path, value) in [
        ("data/minecraft/enchantment/protection.json", document),
        ("data/test/loot_table/pending_vehicle.json", cargo),
    ] {
        std::fs::write(pack.join(path), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    config.datapacks = Some(Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(root.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    ));
    std::fs::remove_dir_all(root).unwrap();
    let mut fixture = Fixture::with_config(config);
    let at = fixture.world.publication().players[0].position;
    for (index, command) in [
        format!("summon minecraft:oak_chest_boat {} {} {} {{UUID:[I;0,0,0,4001],NoGravity:1b,LootTable:'test:pending_vehicle',LootTableSeed:37L}}", at.x + 2.0, at.y, at.z),
        format!("summon minecraft:cow {} {} {} {{UUID:[I;0,0,0,4002],NoAI:1b,NoGravity:1b}}", at.x + 5.0, at.y, at.z),
        "ride 00000000-0000-0000-0000-000000000fa2 mount 00000000-0000-0000-0000-000000000fa1".into(),
        "item replace entity 00000000-0000-0000-0000-000000000fa2 armor.chest with iron_chestplate[enchantments={protection:1}]".into(),
    ].into_iter().enumerate() {
        let tick = console_command(&fixture.sender, &fixture.receiver, &mut fixture.world,
            index as u64 + 1, &command);
        assert_eq!(console_command_answer(&tick).0, Some(1), "{command}");
    }
    let boat_uuid = Uuid::from_u128(4001);
    let cow_uuid = Uuid::from_u128(4002);
    let boat = fixture
        .world
        .live_entity_id_for_uuid_for_test(OVERWORLD, boat_uuid)
        .unwrap();
    let cow = fixture
        .world
        .live_entity_id_for_uuid_for_test(OVERWORLD, cow_uuid)
        .unwrap();
    assert_eq!(
        fixture.world.stored_passengers_for_test(OVERWORLD, boat),
        &[cow]
    );
    let cow_body = fixture
        .world
        .stored_entity_for_test(OVERWORLD, cow)
        .unwrap();
    assert_eq!(cow_body.uuid(), cow_uuid);
    assert!(cow_body.is_alive_with_health());
    assert_eq!(
        cow_body
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::Chest)
            .unwrap()
            .item_resource_id(),
        Some("minecraft:iron_chestplate")
    );
    let boat_body = fixture
        .world
        .stored_entity_for_test(OVERWORLD, boat)
        .unwrap();
    assert!(crate::entity::item_slots::container_has_loot(boat_body));
    for slot in 0..crate::entity::item_slots::container_size(boat_body).unwrap() {
        assert!(crate::entity::item_slots::read(boat_body, slot as i32)
            .unwrap()
            .is_some_and(|stack| stack.is_empty()));
    }
    let NbtSummaryValue::Compound(before) = fixture
        .world
        .stored_entity_command_data_for_test(OVERWORLD, boat)
        .unwrap()
    else {
        panic!("the pending boat has canonical entity data");
    };
    assert!(before.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::String("test:pending_vehicle".into())));
    assert!(before
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(37)));
    assert!(!before.iter().any(|entry| entry.name == "Items"));
    let full = fixture.health(cow);
    let tick = console_command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        5,
        &format!("damage {cow_uuid} 10 minecraft:cactus"),
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    assert_close(
        full - fixture.health(cow),
        after_armor(10.0, 6.0, 0.0) * (1.0 - 15.0 / 25.0),
    );
    let cow_body = fixture
        .world
        .stored_entity_for_test(OVERWORLD, cow)
        .unwrap();
    assert_eq!(cow_body.uuid(), cow_uuid);
    assert!(cow_body.is_alive_with_health());
    assert_eq!(
        fixture.world.stored_passengers_for_test(OVERWORLD, boat),
        &[cow]
    );
    let boat_body = fixture
        .world
        .stored_entity_for_test(OVERWORLD, boat)
        .unwrap();
    assert_eq!(boat_body.uuid(), boat_uuid);
    assert_eq!(boat_body.removal_reason(), None);
    assert!(!crate::entity::item_slots::container_has_loot(boat_body));
    let cargo: Vec<_> = (0..crate::entity::item_slots::container_size(boat_body).unwrap())
        .filter_map(|slot| crate::entity::item_slots::read(boat_body, slot as i32).unwrap())
        .filter(|stack| !stack.is_empty())
        .map(|stack| (stack.item_resource_id().unwrap().to_owned(), stack.count()))
        .collect();
    assert_eq!(cargo, vec![("minecraft:diamond".into(), 1)]);
    let NbtSummaryValue::Compound(record) = fixture
        .world
        .stored_entity_command_data_for_test(OVERWORLD, boat)
        .unwrap()
    else {
        panic!("the unpacked boat has canonical entity data");
    };
    assert!(!record.iter().any(|entry| entry.name == "LootTable"));
    let Some(NbtSummaryValue::List(items)) = record
        .iter()
        .find(|entry| entry.name == "Items")
        .map(|entry| &entry.value)
    else {
        panic!("the unpacked boat saves its cargo");
    };
    let [NbtSummaryValue::Compound(stack)] = items.as_slice() else {
        panic!("the boat saves exactly one cargo stack: {items:?}");
    };
    assert!(stack.iter().any(|entry| entry.name == "id"
        && entry.value == NbtSummaryValue::String("minecraft:diamond".into())));
    assert!(stack
        .iter()
        .any(|entry| entry.name == "count" && entry.value == NbtSummaryValue::Int(1)));
}

/// An arrow's `getWeaponItem` is the weapon that fired it
/// (`AbstractArrow.java:565-567`), for a stored victim and a session alike.
#[test]
fn an_arrows_firing_weapon_breaches_armour() {
    let mut fixture = Fixture::new();
    let husk = fixture.spawn_mob(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HUSK_ID,
        [0.0, 4.0],
    );
    let player = fixture.world.publication().players[0].position;
    let (arrow, _) = fixture.world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
        Vec3d {
            y: player.y + 10.0,
            ..player
        },
        crate::entity::EntityData::Arrow(crate::entity::ArrowData::new(
            crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
        )),
    );
    let compound = |entries: Vec<(&str, NbtSummaryValue)>| {
        NbtSummaryValue::Compound(
            entries
                .into_iter()
                .map(|(name, value)| NbtSummaryEntry {
                    name: name.into(),
                    value,
                })
                .collect(),
        )
    };
    let weapon = compound(vec![
        ("id", NbtSummaryValue::string("minecraft:crossbow")),
        ("count", NbtSummaryValue::Int(1)),
        (
            "components",
            compound(vec![(
                "minecraft:enchantments",
                compound(vec![("minecraft:breach", NbtSummaryValue::Int(4))]),
            )]),
        ),
    ]);
    fixture.world.load_stored_entity_record_for_test(
        OVERWORLD,
        arrow,
        &[
            NbtSummaryEntry {
                name: "NoGravity".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "weapon".into(),
                value: weapon,
            },
        ],
    );
    assert!(fixture
        .world
        .stored_entity_for_test(OVERWORLD, arrow)
        .is_some_and(|entity| crate::entity::enchanted_damage::weapon_item(entity).is_some()));
    fixture.run("item replace entity @e[type=husk] armor.chest with iron_chestplate");
    let taken = fixture.hit(
        husk,
        "damage @e[type=husk,limit=1] 10 minecraft:arrow by @e[type=arrow,limit=1]",
    );
    assert_close(taken, after_armor(10.0, 8.0, -0.6));

    let alice = ConnectionId(1);
    fixture.run("item replace entity alice armor.chest with iron_chestplate");
    fixture.world.set_player_health_for_test(alice, 20.0);
    for _ in 0..11 {
        fixture.world.tick(Vec::new());
    }
    fixture.run("damage alice 10 minecraft:arrow by @e[type=arrow,limit=1]");
    // The player has no base armour.
    assert_close(
        20.0 - fixture.world.player_health_for_test(alice),
        after_armor(10.0, 6.0, -0.6),
    );
}

/// `Wolf.actuallyHurt` (`Wolf.java:403-418`): wolf armour takes the whole hit
/// as durability, and a source that bypasses it wears the armour through
/// `hurtArmor` (`Wolf.java:435-437`) instead.
#[test]
fn wolf_armour_absorbs_and_wears() {
    let mut fixture = Fixture::new();
    let wolf = fixture.spawn_mob(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID,
        [0.0, 4.0],
    );
    fixture.run("item replace entity @e[type=wolf] armor.body with wolf_armor");

    let taken = fixture.hit(wolf, "damage @e[type=wolf,limit=1] 4.5 minecraft:cactus");
    assert_close(taken, 0.0);
    assert_eq!(
        fixture.worn_damage(wolf, EquipmentSlot::Body),
        Some(5),
        "Mth.ceil(4.5)"
    );

    // Thorns bypasses wolf armour but not armour; `(int) max(1, 8 / 4)`.
    fixture.hit(wolf, "damage @e[type=wolf,limit=1] 8 minecraft:thorns");
    assert_eq!(fixture.worn_damage(wolf, EquipmentSlot::Body), Some(7));
}

/// `ServerPlayer.hurtServer` (`ServerPlayer.java:971-984`) under `/damage`
/// (`DamageCommand.java:103-109`): an arrow's player owner must be able to
/// harm the target, and fire resistance (`LivingEntity.java:1164-1166`) and
/// worn immunity (`:3933-3935`) answer the command the way the live hurt does.
#[test]
fn damage_command_answers_with_the_arrow_owner_and_live_immunity() {
    let mut fixture = Fixture::new();
    join_loaded(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        2,
        "bob",
        2,
    );
    let alice = ConnectionId(1);
    let player = fixture.world.publication().players[0].position;
    let (arrow, _) = fixture.world.spawn_stored_entity_for_test(
        OVERWORLD,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
        Vec3d {
            y: player.y + 10.0,
            ..player
        },
        crate::entity::EntityData::Arrow(crate::entity::ArrowData::new(
            crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
        )),
    );
    fixture.world.load_stored_entity_record_for_test(
        OVERWORLD,
        arrow,
        &[
            NbtSummaryEntry {
                name: "NoGravity".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "Owner".into(),
                value: NbtSummaryValue::IntArray(vec![0, 0, 0, 2]),
            },
        ],
    );
    // (refused, damage taken)
    let hurt_alice = |fixture: &mut Fixture, command: &str| {
        fixture.world.set_player_health_for_test(alice, 20.0);
        for _ in 0..11 {
            fixture.world.tick(Vec::new());
        }
        let tick = chat_command(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            1,
            command,
        );
        let refused = system_chats(&tick)
            .iter()
            .any(|(_, line)| line.contains("commands.damage.invulnerable"));
        (refused, 20.0 - fixture.world.player_health_for_test(alice))
    };
    let by_arrow = "damage alice 2 minecraft:arrow by @e[type=arrow,limit=1]";

    let (refused, taken) = hurt_alice(&mut fixture, by_arrow);
    assert!(!refused && taken > 0.0);
    fixture.run("gamerule pvp false");
    assert_eq!(hurt_alice(&mut fixture, by_arrow), (true, 0.0));
    fixture.run("gamerule pvp true");
    fixture.run("team add allies");
    fixture.run("team modify allies friendlyFire false");
    fixture.run("team join allies @a");
    assert_eq!(hurt_alice(&mut fixture, by_arrow), (true, 0.0));
    fixture.run("team remove allies");

    // Frost Walker's `damage_immunity` covers `burn_from_stepping`; hot floor
    // is also fire, so the boots are tried before fire resistance.
    fixture.run(
        "item replace entity alice armor.feet with leather_boots[enchantments={frost_walker:1}]",
    );
    assert_eq!(
        hurt_alice(&mut fixture, "damage alice 2 minecraft:hot_floor"),
        (true, 0.0)
    );
    fixture.run("effect give alice minecraft:fire_resistance 60");
    assert_eq!(
        hurt_alice(&mut fixture, "damage alice 2 minecraft:in_fire"),
        (true, 0.0)
    );

    let husk = fixture.spawn_mob(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HUSK_ID,
        [0.0, 4.0],
    );
    let husk_refused = |fixture: &mut Fixture, command: &str| {
        let tick = chat_command(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            1,
            command,
        );
        system_chats(&tick)
            .iter()
            .any(|(_, line)| line.contains("commands.damage.invulnerable"))
    };
    fixture.run(
        "item replace entity @e[type=husk] armor.feet with leather_boots[enchantments={frost_walker:1}]",
    );
    assert!(husk_refused(
        &mut fixture,
        "damage @e[type=husk,limit=1] 2 minecraft:hot_floor"
    ));
    fixture.run("effect give @e[type=husk] minecraft:fire_resistance 60");
    assert!(husk_refused(
        &mut fixture,
        "damage @e[type=husk,limit=1] 2 minecraft:in_fire"
    ));
    assert_eq!(fixture.health(husk), 20.0);
    assert!(!husk_refused(
        &mut fixture,
        "damage @e[type=husk,limit=1] 2 minecraft:cactus"
    ));
}

#[test]
fn worn_protection_reads_a_rear_passenger_projectile_owners_controller_movement() {
    let dir = std::env::temp_dir().join(format!("bbb-riding-protection-{}", Uuid::new_v4()));
    let pack = dir.join("datapacks/test");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"movement protection","pack_format":1000,"supported_formats":[0,10000]}}"#,
    ).unwrap();
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:protection")
        .unwrap();
    let mut document =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()
            .clone();
    document["effects"]["minecraft:damage_protection"] = serde_json::json!([{
        "effect":{"type":"minecraft:add","value":20},
        "requirements":{"condition":"minecraft:entity_properties","entity":"attacker",
            "predicate":{"movement":{"x":{"min":4,"max":6}}}}
    }]);
    std::fs::write(
        pack.join("data/minecraft/enchantment/protection.json"),
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
    for (motion, multiplier) in [(0.0f64, 1.0), (0.25, 0.2)] {
        let mut fixture = Fixture::with_config(ServerWorldConfig {
            datapacks: Some(Arc::new(startup.clone())),
            ..datapack_config()
        });
        join_loaded(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            2,
            "bob",
            2,
        );
        let cow = fixture.spawn_mob(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [0.0, 4.0],
        );
        fixture.run("item replace entity @e[type=cow] armor.chest with iron_chestplate[enchantments={protection:1}]");
        let target = fixture
            .world
            .stored_entity_for_test(OVERWORLD, cow)
            .unwrap()
            .position();
        let start = Vec3d {
            x: target.x + 10.0,
            y: target.y + 30.0,
            z: target.z,
        };
        for connection in [ConnectionId(1), ConnectionId(2)] {
            fixture
                .world
                .set_player_position_for_test(connection, start);
        }
        play(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            1,
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: start.x + motion,
                    ..start
                }),
                rotation: None,
                on_ground: false,
                horizontal_collision: false,
            }),
        );
        let (boat, _) = fixture.world.spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            start,
            crate::entity::EntityData::Boat(crate::entity::BoatData::new()),
        );
        assert!(fixture
            .world
            .attach_external_player_for_test(ConnectionId(1), boat));
        assert!(fixture
            .world
            .attach_external_player_for_test(ConnectionId(2), boat));
        let target = fixture
            .world
            .stored_entity_for_test(OVERWORLD, cow)
            .unwrap()
            .position();
        let (arrow, _) = fixture.world.spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
            Vec3d {
                y: target.y + 0.7,
                z: target.z - 2.0,
                ..target
            },
            crate::entity::EntityData::Arrow(crate::entity::ArrowData::new(
                crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
            )),
        );
        fixture.world.load_stored_entity_record_for_test(
            OVERWORLD,
            arrow,
            &[
                NbtSummaryEntry {
                    name: "Owner".into(),
                    value: NbtSummaryValue::IntArray(vec![0, 0, 0, 2]),
                },
                NbtSummaryEntry {
                    name: "NoGravity".into(),
                    value: NbtSummaryValue::Byte(1),
                },
                NbtSummaryEntry {
                    name: "LeftOwner".into(),
                    value: NbtSummaryValue::Byte(1),
                },
                NbtSummaryEntry {
                    name: "Motion".into(),
                    value: NbtSummaryValue::List(vec![
                        NbtSummaryValue::Double(0.0f64.to_bits()),
                        NbtSummaryValue::Double(0.0f64.to_bits()),
                        NbtSummaryValue::Double(2.0f64.to_bits()),
                    ]),
                },
            ],
        );
        fixture
            .world
            .stored_entity_mut_for_test(OVERWORLD, arrow)
            .unwrap()
            .set_pos(Vec3d {
                y: target.y + 0.7,
                z: target.z - 2.0,
                ..target
            });
        let full = fixture.health(cow);
        fixture.world.tick(Vec::new());
        assert_close(
            full - fixture.health(cow),
            after_armor(4.0, 6.0, 0.0) * multiplier,
        );
    }
}

#[test]
fn protection_reads_the_attackers_live_player_data_for_mobs_and_players() {
    use crate::tests::entities::{console_command, console_command_answer};

    let [x, y, z] = ServerWorld::fixture_spawn_block();
    let locations: Vec<_> = [z, z + 4]
        .into_iter()
        .map(|z| serde_json::json!({
            "condition":"minecraft:location_check", "offsetX":1,
            "predicate":{"block":{"nbt":format!(
                "{{id:'minecraft:chest',x:{},y:{y},z:{z},Items:[{{Slot:0b,id:'minecraft:diamond',count:3}}]}}",
                x + 1
            )}}
        }))
        .collect();
    let dir = std::env::temp_dir().join(format!("bbb-player-slot-protection-{}", Uuid::new_v4()));
    let pack = dir.join("datapacks/test");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"player slot protection","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:protection")
        .unwrap();
    let mut document =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()
            .clone();
    document["effects"]["minecraft:damage_protection"] = serde_json::json!([{
        "effect":{"type":"minecraft:add","value":20},
        "requirements":{"condition":"minecraft:all_of","terms":[
            {"condition":"minecraft:entity_properties","entity":"attacker",
                "predicate":{"slots":{"inventory.0":{"items":"minecraft:diamond",
                    "components":{"minecraft:custom_data":{"marked":17}}}}}},
            {"condition":"minecraft:entity_properties","entity":"attacker","predicate":{"team":"guards"}},
            {"condition":"minecraft:entity_properties","entity":"attacker",
                "predicate":{"nbt":"{SelectedItem:{id:'minecraft:diamond',count:1}}"}},
            {"condition":"minecraft:entity_scores","entity":"attacker","scores":{"guard":2}},
            {"condition":"minecraft:value_check","value":{"type":"minecraft:score",
                "target":{"type":"minecraft:fixed","name":"#enabled"},"score":"guard"},"range":1},
            {"condition":"minecraft:value_check","value":{"type":"minecraft:storage",
                "storage":"test:protection","path":"enabled"},"range":1},
            {"condition":"minecraft:any_of","terms":locations}
        ]}
    }]);
    std::fs::write(
        pack.join("data/minecraft/enchantment/protection.json"),
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
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        datapacks: Some(Arc::new(startup)),
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let mut fixture = Fixture {
        world,
        sender,
        receiver,
    };
    for z in [z, z + 4] {
        let at = format!("{} {y} {z}", x + 1);
        for command in [
            format!("setblock {at} chest"),
            format!("item replace block {at} container.0 with diamond 3"),
        ] {
            let result = console_command(
                &fixture.sender,
                &fixture.receiver,
                &mut fixture.world,
                1,
                &command,
            );
            assert_eq!(console_command_answer(&result).0, Some(1), "{command}");
        }
    }
    let cow = fixture.spawn_mob(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        [0.0, 4.0],
    );
    fixture.run("item replace entity @e[type=cow] armor.chest with iron_chestplate[enchantments={protection:1}]");
    fixture.run(
        "item replace entity alice armor.chest with iron_chestplate[enchantments={protection:1}]",
    );
    fixture.run("team add guards");
    fixture.run("scoreboard objectives add guard dummy");
    for (item, team, score, enabled, selected_count, storage, factor) in [
        ("diamond[custom_data={marked:16b}]", true, 2, 1, 1, 1, 1.0),
        ("diamond[custom_data={marked:17b}]", false, 2, 1, 1, 1, 1.0),
        ("diamond[custom_data={marked:17b}]", true, 1, 1, 1, 1, 1.0),
        ("diamond[custom_data={marked:17b}]", true, 2, 0, 1, 1, 1.0),
        ("diamond[custom_data={marked:17b}]", true, 2, 1, 2, 1, 1.0),
        ("diamond[custom_data={marked:17b}]", true, 2, 1, 1, 2, 1.0),
        ("diamond[custom_data={marked:17b}]", true, 2, 1, 1, 1, 0.2),
    ] {
        fixture.run("team leave alice");
        if team {
            fixture.run("team join guards alice");
        }
        fixture.run(&format!("scoreboard players set alice guard {score}"));
        fixture.run(&format!("scoreboard players set #enabled guard {enabled}"));
        fixture.run(&format!(
            "data modify storage test:protection enabled set value {storage}"
        ));
        fixture.run(&format!(
            "item replace entity alice inventory.0 with {item}"
        ));
        fixture.run(&format!(
            "item replace entity alice hotbar.0 with diamond {selected_count}"
        ));
        let taken = fixture.hit(
            cow,
            "damage @e[type=cow,limit=1] 5 minecraft:player_attack by alice",
        );
        assert_close(taken, after_armor(5.0, 6.0, 0.0) * factor);
        let alice = ConnectionId(1);
        fixture.world.set_player_health_for_test(alice, 20.0);
        for _ in 0..11 {
            fixture.world.tick(Vec::new());
        }
        fixture.run("damage alice 5 minecraft:player_attack by alice");
        assert_close(
            20.0 - fixture.world.player_health_for_test(alice),
            after_armor(5.0, 6.0, 0.0) * factor,
        );
    }
}
