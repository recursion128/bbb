//! `minecraft:location_changed` (`Enchantment.java:528-576`) and
//! `minecraft:tick` (`:384-390`) enchantment effects on a session and on
//! stored mobs.

use super::*;

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
};

fn default_state(block: &str) -> i32 {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(block).unwrap();
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap()
}

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const SOUL_SPEED: &str = "minecraft:enchantment.soul_speed/feet";
const ALICE: ConnectionId = ConnectionId(1);

struct Fixture {
    world: ServerWorld,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    base: BlockPos,
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
        let position = world.publication().players[0].position;
        let base = BlockPos {
            x: position.x.floor() as i32,
            y: position.y.floor() as i32,
            z: position.z.floor() as i32,
        };
        let mut fixture = Self {
            world,
            sender,
            receiver,
            base,
        };
        // A water floor under clear air, 13 wide.
        for x in -6..=6 {
            for z in -6..=6 {
                fixture.set(x, -1, z, "minecraft:water");
                fixture.set(x, 0, z, "minecraft:air");
                fixture.set(x, 1, z, "minecraft:air");
            }
        }
        fixture
    }

    fn at(&self, x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos {
            x: self.base.x + x,
            y: self.base.y + y,
            z: self.base.z + z,
        }
    }

    fn set(&mut self, x: i32, y: i32, z: i32, block: &str) {
        let pos = self.at(x, y, z);
        self.world
            .set_block(OVERWORLD, pos, default_state(block), 2);
    }

    fn block(&mut self, x: i32, y: i32, z: i32) -> i32 {
        let pos = self.at(x, y, z);
        self.world.block_state(OVERWORLD, pos).unwrap()
    }

    fn run(&mut self, command: &str) {
        chat_command(&self.sender, &self.receiver, &mut self.world, 1, command);
        self.world.tick(self.receiver.drain());
    }

    /// Alice steps onto the centre of cell `(x, 0, z)`, on the ground.
    fn step(&mut self, x: i32, z: i32) -> TickResult {
        let position = Vec3d {
            x: f64::from(self.base.x + x) + 0.5,
            y: f64::from(self.base.y),
            z: f64::from(self.base.z + z) + 0.5,
        };
        play(
            &self.sender,
            &self.receiver,
            &mut self.world,
            1,
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(position),
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        )
    }

    fn player_has(&self, attribute: i32, id: &str) -> bool {
        self.world
            .player_attributes_for_test(ALICE)
            .has_modifier(attribute, id)
    }

    /// A NoAI husk on the ground at the centre of cell `(x, 0, z)`.
    fn spawn_husk(&mut self, x: i32, z: i32) -> i32 {
        self.spawn_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HUSK_ID,
            x,
            z,
        )
    }

    fn spawn_living(&mut self, type_id: i32, x: i32, z: i32) -> i32 {
        let position = Vec3d {
            x: f64::from(self.base.x + x) + 0.5,
            y: f64::from(self.base.y),
            z: f64::from(self.base.z + z) + 0.5,
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
            &[
                NbtSummaryEntry {
                    name: "Pos".into(),
                    value: NbtSummaryValue::List(vec![
                        NbtSummaryValue::Double(position.x.to_bits()),
                        NbtSummaryValue::Double(position.y.to_bits()),
                        NbtSummaryValue::Double(position.z.to_bits()),
                    ]),
                },
                NbtSummaryEntry {
                    name: "NoAI".into(),
                    value: NbtSummaryValue::Byte(1),
                },
                NbtSummaryEntry {
                    name: "OnGround".into(),
                    value: NbtSummaryValue::Byte(1),
                },
            ],
        );
        id
    }

    fn husk_has(&self, id: i32, attribute: i32, modifier: &str) -> bool {
        self.world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .living()
            .unwrap()
            .attributes()
            .has_modifier(attribute, modifier)
    }

    fn boots_damage(&self) -> i32 {
        // Inventory slot 36 is `EquipmentSlot.FEET` (`Inventory.java:36-39`).
        let boots = self.world.player_inventory_slot_for_test(ALICE, 36);
        crate::item_durability::damage_and_max(&boots)
            .expect("iron boots are damageable")
            .0
    }

    fn assert_no_failures(&self) {}
}

/// Frost Walker II freezes the water under a session that changes block on the
/// ground: radius `3 + 1` (`ReplaceDisk.java:43-50`), strict `< 16`.
#[test]
fn frost_walker_freezes_water_around_a_walking_player() {
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:frost_walker")
        .unwrap();
    let mut effects =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()["effects"]
            .clone();
    let predicate = &mut effects["minecraft:location_changed"][0]["effect"]["predicate"];
    predicate["unused"] = serde_json::json!([null]);
    predicate["predicates"][0]["offset"] = serde_json::json!([4294967296_u64, 1, 0]);
    let mut fixture = Fixture::with_config(datapack_config_with_enchantment_effects(
        "minecraft:frost_walker",
        effects,
    ));
    let water = default_state("minecraft:water");
    let ice = default_state("minecraft:frosted_ice");
    fixture.step(1, 0);
    assert_eq!(fixture.block(1, -1, 0), water, "no boots, no ice");

    // Equipping runs the effects at once (`LivingEntity.java:2915`), off the
    // ground this client tick reports: water holds nobody up, so the tick's
    // own travel took the last report's ground away again.
    enqueue(
        &fixture.sender,
        WorldCommand::Play {
            connection_id: ALICE,
            packet: PlayServerbound::ChatCommand {
                command: "item replace entity alice armor.feet with leather_boots[enchantments={frost_walker:2}]".to_string(),
            },
        },
    )
    .unwrap();
    fixture.step(1, 0);
    assert_eq!(fixture.block(1, -1, 0), ice);
    assert_eq!(fixture.block(5, -1, 0), water, "distance 4 is outside");
    fixture.step(2, 0);
    assert_eq!(fixture.block(5, -1, 0), ice);
    assert_eq!(fixture.block(2, -1, 3), ice);
    assert_eq!(fixture.block(6, -1, 0), water);
    assert_eq!(fixture.block(-3, -1, 0), water);

    // Spectators are skipped (`ServerPlayer.java:1270-1274`).
    fixture.run("gamemode spectator");
    fixture.step(-4, 0);
    assert_eq!(fixture.block(-4, -1, 0), water);
    fixture.assert_no_failures();
}

/// Soul Speed III on soul sand: both attribute modifiers while active, the
/// wear on soul sand, and removal once the session leaves it.
#[test]
fn soul_speed_toggles_its_modifiers_and_wears_the_boots() {
    let mut fixture = Fixture::new();
    for x in 1..=2 {
        fixture.set(x, -1, 0, "minecraft:soul_sand");
    }
    fixture.set(3, -1, 0, "minecraft:stone");
    fixture
        .run("item replace entity alice armor.feet with iron_boots[enchantments={soul_speed:3}]");
    fixture.step(1, 0);
    assert!(fixture.player_has(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED));
    assert!(fixture.player_has(VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID, SOUL_SPEED));
    let speed = fixture
        .world
        .player_attribute_value_for_test(ALICE, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
    assert!(
        (speed - (0.1 + 0.0405 + 2.0 * 0.0105)).abs() < 1e-6,
        "{speed}"
    );

    fixture.run("tick freeze");
    let before = fixture.boots_damage();
    // Each move on soul sand wears the boots at 4% (`Enchantments.java:523-535`);
    // 800 moves all miss with odds of about 6e-15.
    for step in 0..800 {
        if fixture.boots_damage() != before {
            break;
        }
        fixture.step(2 - step % 2, 0);
    }
    fixture.assert_no_failures();
    assert_eq!(fixture.boots_damage(), before + 1);
    assert!(fixture.player_has(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED));

    fixture.step(3, 0);
    assert!(!fixture.player_has(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED));
    assert!(!fixture.player_has(VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID, SOUL_SPEED));
    fixture.assert_no_failures();
}

/// Stored mobs run the same effects from `LivingEntity.baseTick`
/// (`LivingEntity.java:441-445`) and `collectEquipmentChanges`
/// (`:2883-2922`).
#[test]
fn stored_mobs_run_location_effects_and_stop_them_on_unequip() {
    let mut fixture = Fixture::new();
    let water = default_state("minecraft:water");
    let ice = default_state("minecraft:frosted_ice");
    fixture.set(-4, -1, 0, "minecraft:soul_sand");
    fixture.set(4, -1, 0, "minecraft:stone");
    let soul = fixture.spawn_husk(-4, 0);
    let frost = fixture.spawn_husk(4, 0);
    fixture.run(&format!(
        "item replace entity @e[type=husk,x={},y={},z={},distance=..1] armor.feet with iron_boots[enchantments={{soul_speed:1}}]",
        fixture.base.x - 4,
        fixture.base.y,
        fixture.base.z,
    ));
    fixture.run(&format!(
        "item replace entity @e[type=husk,x={},y={},z={},distance=..1] armor.feet with iron_boots[enchantments={{frost_walker:1}}]",
        fixture.base.x + 4,
        fixture.base.y,
        fixture.base.z,
    ));
    fixture.world.tick(Vec::new());
    fixture.assert_no_failures();
    assert!(fixture.husk_has(soul, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED));
    assert_eq!(fixture.block(5, -1, 0), ice);
    assert_eq!(fixture.block(4, -1, 2), ice);
    assert_eq!(fixture.block(4, -1, 3), water, "Frost Walker I reaches 3");
    assert!(!fixture.husk_has(frost, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED));

    fixture.run(&format!(
        "item replace entity @e[type=husk,x={},y={},z={},distance=..1] armor.feet with iron_boots",
        fixture.base.x - 4,
        fixture.base.y,
        fixture.base.z,
    ));
    fixture.world.tick(Vec::new());
    assert!(!fixture.husk_has(soul, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED));
    fixture.assert_no_failures();
}

#[test]
fn player_and_mob_location_and_tick_effects_read_the_named_clock() {
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:soul_speed")
        .unwrap();
    let effect =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()["effects"]["minecraft:location_changed"][0]["effect"]
            .clone();
    let requirements = serde_json::json!({"condition":"minecraft:all_of","terms":[
        {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107},
        {"condition":"minecraft:time_check","clock":"minecraft:the_end","period":100,"value":7}
    ]});
    let mut fixture = Fixture::with_config(datapack_config_with_enchantment_effects(
        "minecraft:soul_speed",
        serde_json::json!({
            "minecraft:location_changed":[{"effect":effect,"requirements":requirements}],
            "minecraft:tick":[{"effect":{"type":"minecraft:change_item_damage","amount":1},
                "requirements":requirements}]
        }),
    ));
    for z in 0..=2 {
        fixture.set(0, -1, z, "minecraft:stone");
        fixture.set(4, -1, z, "minecraft:stone");
    }
    set_paused_test_clocks(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        108,
        0,
    );
    let husk = fixture.spawn_husk(4, 0);
    for target in ["alice", "@e[type=husk,limit=1]"] {
        let result = console_command(&fixture.sender, &fixture.receiver, &mut fixture.world, 1,
            &format!("item replace entity {target} armor.feet with iron_boots[enchantments={{soul_speed:1}}]"));
        assert_eq!(super::entities::console_command_answer(&result).0, Some(1));
    }
    let mob_wear = |fixture: &Fixture| {
        fixture
            .world
            .stored_entity_for_test(OVERWORLD, husk)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::Feet)
            .unwrap()
            .summary()
            .component_patch
            .damage
            .unwrap_or(0)
    };
    assert!(!fixture.player_has(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED));
    assert!(!fixture.husk_has(husk, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED));
    assert_eq!((fixture.boots_damage(), mob_wear(&fixture)), (0, 0));
    for (time, z, active) in [(107, 1, true), (108, 2, false)] {
        set_paused_test_clocks(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            time,
            z as u64,
        );
        let before = (fixture.boots_damage(), mob_wear(&fixture));
        fixture.step(0, z);
        let result = console_command(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            1,
            &format!(
                "tp @e[type=husk,limit=1] {} {} {}",
                f64::from(fixture.base.x + 4) + 0.5,
                fixture.base.y,
                f64::from(fixture.base.z + z) + 0.5
            ),
        );
        assert_eq!(super::entities::console_command_answer(&result).0, Some(1));
        fixture.world.tick(Vec::new());
        assert_eq!(
            fixture.player_has(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED),
            active
        );
        assert_eq!(
            fixture.husk_has(husk, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED),
            active
        );
        let after = (fixture.boots_damage(), mob_wear(&fixture));
        if active {
            assert!(
                after.0 > before.0 && after.1 > before.1,
                "{before:?} -> {after:?}"
            );
        } else {
            assert_eq!(after, before);
        }
    }
    let mut config = datapack_config();
    let mut document: serde_json::Value = serde_json::from_slice(
        &config
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", "minecraft:soul_speed")
            .unwrap()
            .source_json,
    )
    .unwrap();
    document["effects"] = serde_json::json!({"minecraft:tick":[{
        "effect":{"type":"minecraft:change_item_damage","amount":1},
        "requirements":{"condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"type":"minecraft:player","vehicle":{
                "type":"minecraft:oak_chest_boat",
                "slots":{"container.*":{"items":"minecraft:diamond","count":1}}
            }}}
    }]});
    document["effects"]["minecraft:tick"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "effect":{"type":"minecraft:change_item_damage","amount":1},
            "requirements":{"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"type":"minecraft:husk","vehicle":{
                    "type":"minecraft:oak_chest_boat",
                    "slots":{"container.*":{"items":"minecraft:diamond","count":1}}
                }}}
        }));
    let cargo = serde_json::json!({"type":"minecraft:chest","pools":[
        {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:location_check",
            "predicate":{"dimension":"minecraft:overworld"}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties",
            "entity":"this","predicate":{}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}
    ]});
    let root = std::env::temp_dir().join(format!("bbb-session-tick-cargo-{}", Uuid::new_v4()));
    let pack = root.join("cargo");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"Session tick cargo","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for (path, value) in [
        ("data/minecraft/enchantment/soul_speed.json", document),
        ("data/test/loot_table/tick_vehicle.json", cargo),
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
    let equipped = console_command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        1,
        "item replace entity alice armor.feet with iron_boots[enchantments={soul_speed:1}]",
    );
    assert_eq!(
        super::entities::console_command_answer(&equipped).0,
        Some(1)
    );
    fixture.world.tick(Vec::new());
    assert_eq!(fixture.boots_damage(), 0);
    let at = fixture.world.publication().players[0].position;
    let summoned = console_command(
        &fixture.sender, &fixture.receiver, &mut fixture.world, 2,
        &format!("summon minecraft:oak_chest_boat {} {} {} {{UUID:[I;0,0,0,6101],NoGravity:1b,LootTable:'test:tick_vehicle',LootTableSeed:37L}}", at.x + 2.0, at.y, at.z),
    );
    assert_eq!(
        super::entities::console_command_answer(&summoned).0,
        Some(1)
    );
    let boat_uuid = Uuid::from_u128(6101);
    let boat = fixture
        .world
        .live_entity_id_for_uuid_for_test(OVERWORLD, boat_uuid)
        .unwrap();
    let body = fixture
        .world
        .stored_entity_for_test(OVERWORLD, boat)
        .unwrap();
    assert_eq!(body.uuid(), boat_uuid);
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
    );
    assert!(crate::entity::item_slots::container_has_loot(body));
    for slot in 0..crate::entity::item_slots::container_size(body).unwrap() {
        assert!(crate::entity::item_slots::read(body, slot as i32)
            .unwrap()
            .is_some_and(|stack| stack.is_empty()));
    }
    let NbtSummaryValue::Compound(before) = fixture
        .world
        .stored_entity_command_data_for_test(OVERWORLD, boat)
        .unwrap()
    else {
        panic!("pending boat")
    };
    assert!(before.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::String("test:tick_vehicle".into())));
    assert!(before
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(37)));
    assert!(!before.iter().any(|entry| entry.name == "Items"));
    assert_eq!(fixture.boots_damage(), 0);
    let mounted = console_command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        3,
        &format!("ride alice mount {boat_uuid}"),
    );
    assert_eq!(super::entities::console_command_answer(&mounted).0, Some(1));
    assert_eq!(
        fixture
            .world
            .external_player_vehicle_state_for_test(ALICE)
            .2,
        Some(boat)
    );
    fixture.world.tick(Vec::new());
    assert!(fixture.boots_damage() > 0);
    let worn = fixture.boots_damage();
    fixture.world.tick(Vec::new());
    assert!(fixture.boots_damage() > worn);
    let body = fixture
        .world
        .stored_entity_for_test(OVERWORLD, boat)
        .unwrap();
    assert_eq!(body.uuid(), boat_uuid);
    assert!(!crate::entity::item_slots::container_has_loot(body));
    let mut counts = BTreeMap::new();
    for slot in 0..crate::entity::item_slots::container_size(body).unwrap() {
        if let Some(stack) = crate::entity::item_slots::read(body, slot as i32)
            .unwrap()
            .filter(|stack| !stack.is_empty())
        {
            *counts
                .entry(stack.item_resource_id().unwrap().to_owned())
                .or_insert(0) += stack.count();
        }
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("minecraft:apple".to_string(), 1),
            ("minecraft:diamond".to_string(), 1)
        ])
    );
    let NbtSummaryValue::Compound(after) = fixture
        .world
        .stored_entity_command_data_for_test(OVERWORLD, boat)
        .unwrap()
    else {
        panic!("unpacked boat")
    };
    assert!(!after
        .iter()
        .any(|entry| entry.name == "LootTable" || entry.name == "LootTableSeed"));
    assert!(after.iter().any(|entry| entry.name == "Items"));
    let dismounted = console_command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        4,
        "ride alice dismount",
    );
    assert_eq!(
        super::entities::console_command_answer(&dismounted).0,
        Some(1)
    );
    assert_eq!(
        fixture
            .world
            .external_player_vehicle_state_for_test(ALICE)
            .2,
        None
    );
    let worn = fixture.boots_damage();
    for _ in 0..2 {
        fixture.world.tick(Vec::new());
    }
    assert_eq!(fixture.boots_damage(), worn);

    let husk_uuid = Uuid::from_u128(6103);
    let boat_uuid = Uuid::from_u128(6102);
    for (operation, command) in [
        (5, format!("summon minecraft:husk {} {} {} {{UUID:[I;0,0,0,6103],NoAI:1b,NoGravity:1b}}", at.x + 7.0, at.y, at.z)),
        (6, format!("summon minecraft:oak_chest_boat {} {} {} {{UUID:[I;0,0,0,6102],NoGravity:1b,LootTable:'test:tick_vehicle',LootTableSeed:37L}}", at.x + 4.0, at.y, at.z)),
        (7, format!("item replace entity {husk_uuid} armor.feet with iron_boots[enchantments={{soul_speed:1}}]")),
    ] {
        let result = console_command(
            &fixture.sender, &fixture.receiver, &mut fixture.world, operation, &command,
        );
        assert_eq!(super::entities::console_command_answer(&result).0, Some(1));
    }
    let husk = fixture
        .world
        .live_entity_id_for_uuid_for_test(OVERWORLD, husk_uuid)
        .unwrap();
    let boat = fixture
        .world
        .live_entity_id_for_uuid_for_test(OVERWORLD, boat_uuid)
        .unwrap();
    let stored_wear = |fixture: &Fixture| {
        fixture
            .world
            .stored_entity_for_test(OVERWORLD, husk)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::Feet)
            .unwrap()
            .summary()
            .component_patch
            .damage
            .unwrap_or(0)
    };
    for _ in 0..2 {
        fixture.world.tick(Vec::new());
    }
    assert_eq!(stored_wear(&fixture), 0);
    assert!(fixture
        .world
        .stored_passengers_for_test(OVERWORLD, boat)
        .is_empty());
    let body = fixture
        .world
        .stored_entity_for_test(OVERWORLD, boat)
        .unwrap();
    assert_eq!(body.uuid(), boat_uuid);
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
    );
    assert!(crate::entity::item_slots::container_has_loot(body));
    for slot in 0..crate::entity::item_slots::container_size(body).unwrap() {
        assert!(crate::entity::item_slots::read(body, slot as i32)
            .unwrap()
            .is_some_and(|stack| stack.is_empty()));
    }
    let NbtSummaryValue::Compound(before) = fixture
        .world
        .stored_entity_command_data_for_test(OVERWORLD, boat)
        .unwrap()
    else {
        panic!("pending stored vehicle")
    };
    assert!(before.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::String("test:tick_vehicle".into())));
    assert!(before
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(37)));
    assert!(!before.iter().any(|entry| entry.name == "Items"));
    let mounted = console_command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        8,
        &format!("ride {husk_uuid} mount {boat_uuid}"),
    );
    assert_eq!(super::entities::console_command_answer(&mounted).0, Some(1));
    assert_eq!(
        fixture.world.stored_passengers_for_test(OVERWORLD, boat),
        &[husk]
    );
    for _ in 0..20 {
        fixture.world.tick(Vec::new());
        if stored_wear(&fixture) > 0 {
            break;
        }
    }
    assert!(stored_wear(&fixture) > 0);
    let worn = stored_wear(&fixture);
    for _ in 0..20 {
        fixture.world.tick(Vec::new());
        if stored_wear(&fixture) > worn {
            break;
        }
    }
    assert!(stored_wear(&fixture) > worn);
    let body = fixture
        .world
        .stored_entity_for_test(OVERWORLD, boat)
        .unwrap();
    assert_eq!(body.uuid(), boat_uuid);
    assert!(!crate::entity::item_slots::container_has_loot(body));
    let mut counts = BTreeMap::new();
    for slot in 0..crate::entity::item_slots::container_size(body).unwrap() {
        if let Some(stack) = crate::entity::item_slots::read(body, slot as i32)
            .unwrap()
            .filter(|stack| !stack.is_empty())
        {
            *counts
                .entry(stack.item_resource_id().unwrap().to_owned())
                .or_insert(0) += stack.count();
        }
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("minecraft:apple".to_string(), 1),
            ("minecraft:diamond".to_string(), 1),
        ])
    );
    let NbtSummaryValue::Compound(after) = fixture
        .world
        .stored_entity_command_data_for_test(OVERWORLD, boat)
        .unwrap()
    else {
        panic!("unpacked stored vehicle")
    };
    assert!(!after
        .iter()
        .any(|entry| entry.name == "LootTable" || entry.name == "LootTableSeed"));
    assert!(after.iter().any(|entry| entry.name == "Items"));
    let dismounted = console_command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        9,
        &format!("ride {husk_uuid} dismount"),
    );
    assert_eq!(
        super::entities::console_command_answer(&dismounted).0,
        Some(1)
    );
    assert!(fixture
        .world
        .stored_passengers_for_test(OVERWORLD, boat)
        .is_empty());
    let worn = stored_wear(&fixture);
    for _ in 0..2 {
        fixture.world.tick(Vec::new());
    }
    assert_eq!(stored_wear(&fixture), worn);
}

#[test]
fn player_and_mob_equipment_conditions_read_live_scores_teams_and_player_nbt() {
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:soul_speed")
        .unwrap();
    let effect =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()["effects"]["minecraft:location_changed"][0]["effect"]
            .clone();
    let requirements = serde_json::json!({"condition":"minecraft:all_of","terms":[
        {"condition":"minecraft:entity_properties","entity":"this","predicate":{"team":"walkers"}},
        {"condition":"minecraft:entity_scores","entity":"this","scores":{"stride":2}},
        {"condition":"minecraft:value_check","value":{"type":"minecraft:score",
            "target":{"type":"minecraft:fixed","name":"#enabled"},"score":"stride"},"range":1},
        {"condition":"minecraft:any_of","terms":[
            {"condition":"minecraft:entity_properties","entity":"this","predicate":{"type":"minecraft:husk"}},
            {"condition":"minecraft:entity_properties","entity":"this","predicate":{
                "type":"minecraft:player",
                "nbt":"{SelectedItem:{id:'minecraft:stone',count:3,components:{'minecraft:custom_data':{stride:1}}}}"}}
        ]}
    ]});
    let mut fixture = Fixture::with_config(datapack_config_with_enchantment_effects(
        "minecraft:soul_speed",
        serde_json::json!({
            "minecraft:location_changed":[{"effect":effect,"requirements":requirements}],
            "minecraft:tick":[{"effect":{"type":"minecraft:change_item_damage","amount":1},
                "requirements":requirements}]
        }),
    ));
    for z in 0..=5 {
        fixture.set(0, -1, z, "minecraft:stone");
        fixture.set(4, -1, z, "minecraft:stone");
    }
    let husk = fixture.spawn_husk(4, 0);
    for input in ["team add walkers", "scoreboard objectives add stride dummy",
        "item replace entity alice weapon.mainhand with stone[custom_data={stride:1}] 3",
        "item replace entity alice armor.feet with iron_boots[enchantments={soul_speed:1}]",
        "item replace entity @e[type=husk,limit=1] armor.feet with iron_boots[enchantments={soul_speed:1}]"] {
        fixture.run(input);
    }
    let mob_wear = |fixture: &Fixture| {
        fixture
            .world
            .stored_entity_for_test(OVERWORLD, husk)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::Feet)
            .unwrap()
            .summary()
            .component_patch
            .damage
            .unwrap_or(0)
    };
    for (z, team, score, enabled, active) in [
        (1, true, 2, 1, true),
        (2, false, 2, 1, false),
        (3, true, 1, 1, false),
        (4, true, 2, 0, false),
        (5, true, 2, 1, true),
    ] {
        for target in ["alice", "@e[type=husk,limit=1]"] {
            fixture.run(&format!("team leave {target}"));
            fixture.run(&format!("scoreboard players set {target} stride {score}"));
        }
        fixture.run(&format!("scoreboard players set #enabled stride {enabled}"));
        if team {
            fixture.run("team join walkers alice");
            fixture.run("team join walkers @e[type=husk,limit=1]");
        }
        let before = (fixture.boots_damage(), mob_wear(&fixture));
        fixture.step(0, z);
        fixture.run(&format!(
            "tp @e[type=husk,limit=1] {} {} {}",
            f64::from(fixture.base.x + 4) + 0.5,
            fixture.base.y,
            f64::from(fixture.base.z + z) + 0.5
        ));
        assert_eq!(
            fixture.player_has(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED),
            active
        );
        assert_eq!(
            fixture.husk_has(husk, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED),
            active
        );
        let after = (fixture.boots_damage(), mob_wear(&fixture));
        if active {
            assert!(
                after.0 > before.0 && after.1 > before.1,
                "z={z}: {before:?} -> {after:?}"
            );
        } else {
            assert_eq!(after, before, "z={z}");
        }
    }
    for (z, selected, active) in [
        (1, "stone[custom_data={stride:2}] 3", false),
        (2, "stone[custom_data={stride:1}] 3", true),
    ] {
        fixture.run(&format!(
            "item replace entity alice weapon.mainhand with {selected}"
        ));
        let before = (fixture.boots_damage(), mob_wear(&fixture));
        fixture.step(0, z);
        fixture.world.tick(Vec::new());
        assert_eq!(
            fixture.player_has(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED),
            active,
            "selected={selected}"
        );
        assert!(fixture.husk_has(husk, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, SOUL_SPEED));
        let after = (fixture.boots_damage(), mob_wear(&fixture));
        if active {
            assert!(
                after.0 > before.0,
                "selected={selected}: {before:?} -> {after:?}"
            );
        } else {
            assert_eq!(after.0, before.0, "selected={selected}");
        }
        assert!(
            after.1 > before.1,
            "selected={selected}: {before:?} -> {after:?}"
        );
    }
}

/// Soul particles sent to alice in `result`, with the escape sounds beside
/// them.
fn soul_effects(result: &TickResult) -> (usize, usize) {
    let escape = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:particle.soul_escape",
    )
    .unwrap();
    let mut counts = (0, 0);
    for event in &result.events {
        match event {
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::LevelParticles(particles),
            } if particles.particle.particle_type_id
                == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_SOUL_ID =>
            {
                assert_eq!(particles.count, 0);
                counts.0 += 1;
            }
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::Sound(sound),
            } if matches!(
                sound.sound,
                bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                    if registry_id == escape
            ) =>
            {
                counts.1 += 1;
            }
            _ => {}
        }
    }
    counts
}

/// Soul Speed's `minecraft:tick` effects: every fifth tick of moving on soul
/// sand a soul particle, and a 35% escape sound, for a session and a stored
/// mob alike.
#[test]
fn soul_speed_ticks_soul_particles_while_moving_on_soul_sand() {
    let mut fixture = Fixture::new();
    for x in 1..=2 {
        fixture.set(x, -1, 0, "minecraft:soul_sand");
    }
    fixture
        .run("item replace entity alice armor.feet with iron_boots[enchantments={soul_speed:3}]");
    let (mut particles, mut sounds) = (0, 0);
    for step in 0..40 {
        let (p, s) = soul_effects(&fixture.step(1 + step % 2, 0));
        particles += p;
        sounds += s;
    }
    assert_eq!(particles, 8, "one tick in five");
    assert!(sounds > 0 && sounds < particles);

    // Alice steps off the soul sand: her known movement outlives the step.
    fixture.step(0, 0);
    // A stored husk on soul sand only ticks them while it moves.
    fixture.set(-4, -1, 0, "minecraft:soul_sand");
    let husk = fixture.spawn_husk(-4, 0);
    fixture.run(&format!(
        "item replace entity @e[type=husk,x={},y={},z={},distance=..1] armor.feet with iron_boots[enchantments={{soul_speed:3}}]",
        fixture.base.x - 4,
        fixture.base.y,
        fixture.base.z,
    ));
    let mut still = 0;
    for _ in 0..10 {
        still += soul_effects(&fixture.world.tick(Vec::new())).0;
    }
    assert_eq!(still, 0);
    let mut moving = 0;
    for _ in 0..10 {
        let body = fixture
            .world
            .stored_entity_mut_for_test(OVERWORLD, husk)
            .unwrap();
        let resting = body.delta_movement();
        body.set_delta_movement(Vec3d { x: 0.05, ..resting });
        moving += soul_effects(&fixture.world.tick(Vec::new())).0;
    }
    assert_eq!(moving, 2);
    fixture.assert_no_failures();
}

fn move_before_mount(fixture: &mut Fixture) {
    let start = fixture.world.publication().players[0].position;
    play(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: start.x + 0.25,
                ..start
            }),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
}

#[test]
fn stored_camel_soul_particles_follow_its_players_movement_only_with_a_saddle() {
    let mut fixture = Fixture::new();
    fixture.set(0, -1, 0, "minecraft:soul_sand");
    let camel = fixture.spawn_living(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID,
        0,
        0,
    );
    fixture.run("data merge entity @e[type=camel,limit=1] {NoGravity:1b,Motion:[0.0d,0.0d,0.0d]}");
    fixture.run("item replace entity @e[type=camel,limit=1] armor.feet with iron_boots[enchantments={soul_speed:3}]");
    fixture.run("item replace entity @e[type=camel,limit=1] saddle with saddle");
    move_before_mount(&mut fixture);
    assert!(fixture.world.attach_external_player_for_test(ALICE, camel));
    let mut moving = 0;
    for _ in 0..15 {
        moving += soul_effects(&fixture.world.tick(Vec::new())).0;
    }
    assert!(
        moving > 0,
        "a stationary camel uses its controlling player's motion"
    );
    assert_eq!(
        fixture
            .world
            .stored_entity_for_test(OVERWORLD, camel)
            .unwrap()
            .delta_movement(),
        Vec3d::default()
    );
    fixture.run("item replace entity @e[type=camel,limit=1] saddle with air");
    assert_eq!(
        fixture
            .world
            .external_player_vehicle_state_for_test(ALICE)
            .2,
        Some(camel)
    );
    let mut stationary = 0;
    for _ in 0..15 {
        stationary += soul_effects(&fixture.world.tick(Vec::new())).0;
    }
    assert_eq!(stationary, 0, "an unsaddled camel has no player controller");
}

#[test]
fn nested_riding_fall_protection_reads_the_camel_controllers_movement() {
    let dir =
        std::env::temp_dir().join(format!("bbb-stored-movement-protection-{}", Uuid::new_v4()));
    let pack = dir.join("datapacks/test");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"movement protection","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
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
        "requirements":{"condition":"minecraft:entity_properties","entity":"this",
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
    let mut losses = Vec::new();
    for saddled in [false, true] {
        let mut fixture = Fixture::with_config(ServerWorldConfig {
            datapacks: Some(Arc::new(startup.clone())),
            ..datapack_config()
        });
        fixture.set(0, -1, 0, "minecraft:stone");
        fixture.spawn_husk(0, 0);
        let camel = fixture.spawn_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID,
            0,
            0,
        );
        fixture.run("item replace entity @e[type=camel,limit=1] armor.feet with iron_boots[enchantments={protection:1}]");
        if saddled {
            fixture.run("item replace entity @e[type=camel,limit=1] saddle with saddle");
        }
        fixture.run("ride @e[type=camel,limit=1] mount @e[type=husk,limit=1]");
        move_before_mount(&mut fixture);
        assert!(fixture.world.attach_external_player_for_test(ALICE, camel));
        let before = fixture
            .world
            .stored_entity_for_test(OVERWORLD, camel)
            .unwrap()
            .living()
            .unwrap()
            .health();
        fixture.run("data merge entity @e[type=husk,limit=1] {NoAI:0b,fall_distance:10.0d,OnGround:0b,Motion:[0.0d,-0.1d,0.0d]}");
        losses.push(
            before
                - fixture
                    .world
                    .stored_entity_for_test(OVERWORLD, camel)
                    .unwrap()
                    .living()
                    .unwrap()
                    .health(),
        );
    }
    assert!(
        losses[0] > 0.0,
        "the fall reaches the nested camel: {losses:?}"
    );
    assert!(
        (losses[1] - losses[0] * 0.2).abs() < 1e-4,
        "fall damage with and without player control: {losses:?}"
    );
}

#[test]
fn stored_equipment_runs_location_and_tick_functions_in_the_world() {
    let dir = std::env::temp_dir().join(format!("bbb-location-functions-{}", Uuid::new_v4()));
    let pack = dir.join("test");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/function")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"location functions","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:frost_walker")
        .unwrap();
    let mut document =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()
            .clone();
    let [x, y, z] = ServerWorld::fixture_spawn_block();
    let chest = BlockPos {
        x: x + 4,
        y,
        z: z + 1,
    };
    let requirements = serde_json::json!({
        "condition":"minecraft:location_check", "offsetZ":1,
        "predicate":{"block":{"nbt":format!(
            "{{id:'minecraft:chest',x:{},y:{},z:{},Items:[{{Slot:0b,id:'minecraft:diamond',count:3}}]}}",
            chest.x, chest.y, chest.z
        )}}
    });
    document["effects"] = serde_json::json!({
        "minecraft:location_changed":[{"effect":{"type":"minecraft:run_function","function":"test:location"},"requirements":requirements}],
        "minecraft:tick":[
            {"effect":{"type":"minecraft:run_function","function":"test:tick"},"requirements":requirements},
            {"effect":{"type":"minecraft:run_function","function":"test:vehicle"},
                "requirements":{"condition":"minecraft:entity_properties","entity":"this", "predicate":{
                    "type":"minecraft:player", "vehicle":{"type":"minecraft:camel", "flags":{"is_baby":false},
                        "equipment":{"mainhand":{"items":"minecraft:emerald"}},
                        "components":{"minecraft:custom_name":"Carrier"}}
                }}}
        ]
    });
    std::fs::write(
        pack.join("data/minecraft/enchantment/frost_walker.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    std::fs::write(
        pack.join("data/test/function/location.mcfunction"),
        "effect give @s minecraft:glowing 30 0 true\nsetblock ~ ~2 ~ minecraft:gold_block",
    )
    .unwrap();
    std::fs::write(
        pack.join("data/test/function/tick.mcfunction"),
        "setblock ~1 ~2 ~ minecraft:diamond_block",
    )
    .unwrap();
    std::fs::write(
        pack.join("data/test/function/vehicle.mcfunction"),
        format!("setblock {x} {} {} minecraft:emerald_block", y + 2, z - 3),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(dir.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    let mut fixture = Fixture::with_config(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..datapack_config()
    });
    fixture.base = BlockPos { x, y, z };
    fixture.set(4, -1, 0, "minecraft:stone");
    let setup = console_command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        1,
        &format!(
            "setblock {} {} {} chest{{Items:[{{Slot:0b,id:\"minecraft:diamond\",count:3}}]}}",
            chest.x, chest.y, chest.z
        ),
    );
    assert_eq!(super::entities::console_command_answer(&setup).0, Some(1));
    let husk = fixture.spawn_husk(4, 0);
    fixture.run("item replace entity @e[type=husk,limit=1] armor.feet with iron_boots[enchantments={frost_walker:1}]");
    assert_eq!(
        fixture.block(4, 2, 0),
        default_state("minecraft:gold_block")
    );
    assert!(fixture
        .world
        .stored_entity_for_test(OVERWORLD, husk)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:glowing")
        .is_some());
    assert_eq!(
        fixture.block(5, 2, 0),
        default_state("minecraft:diamond_block")
    );
    fixture.set(5, 2, 0, "minecraft:air");
    fixture.world.tick(Vec::new());
    assert_eq!(
        fixture.block(5, 2, 0),
        default_state("minecraft:diamond_block")
    );
    fixture.run("item replace entity @e[type=husk,limit=1] armor.feet with air");
    fixture.set(4, 2, 0, "minecraft:air");
    fixture.set(5, 2, 0, "minecraft:air");
    let moved = console_command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        2,
        &format!("tp alice {} {y} {}", f64::from(x) + 4.5, f64::from(z) + 0.5),
    );
    assert_eq!(super::entities::console_command_answer(&moved).0, Some(1));
    fixture
        .run("item replace entity alice armor.feet with iron_boots[enchantments={frost_walker:1}]");
    assert_eq!(
        fixture.block(4, 2, 0),
        default_state("minecraft:gold_block")
    );
    assert!(fixture
        .world
        .player_active_effect_for_test(ALICE, "minecraft:glowing")
        .is_some());
    assert_eq!(
        fixture.block(5, 2, 0),
        default_state("minecraft:diamond_block")
    );
    fixture.set(5, 2, 0, "minecraft:air");
    fixture.world.tick(Vec::new());
    assert_eq!(
        fixture.block(5, 2, 0),
        default_state("minecraft:diamond_block")
    );
    assert_eq!(fixture.block(0, 2, -3), default_state("minecraft:air"));
    fixture.set(-4, -1, 0, "minecraft:stone");
    fixture.spawn_living(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID,
        -4,
        0,
    );
    fixture.run("data merge entity @e[type=camel,limit=1] {NoGravity:1b,CustomName:\"Carrier\"}");
    fixture.run("item replace entity @e[type=camel,limit=1] weapon.mainhand with emerald");
    fixture.run("ride alice mount @e[type=camel,limit=1]");
    assert_eq!(
        fixture.block(0, 2, -3),
        default_state("minecraft:emerald_block")
    );
    fixture.run("item replace entity @e[type=camel,limit=1] weapon.mainhand with stone");
    fixture.set(0, 2, -3, "minecraft:air");
    fixture.world.tick(Vec::new());
    assert_eq!(fixture.block(0, 2, -3), default_state("minecraft:air"));
    std::fs::remove_dir_all(dir).unwrap();
}

/// A datapack enchantment's `tick` effects on a stored wearer: a mob effect
/// (`ApplyMobEffect.java:39-54`), dust particles with their options
/// (`SpawnParticlesEffect.java:30`) and a sound whose volume is a trapezoid
/// float provider (`FloatProviders.java:33-37`).
#[test]
fn a_worn_datapack_enchantment_ticks_mob_effects_particles_and_sounds_on_a_mob() {
    let dir = std::env::temp_dir().join(format!("bbb-worn-tick-{}", Uuid::new_v4()));
    let pack = dir.join("test");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"worn tick","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:frost_walker")
        .unwrap();
    let mut document =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()
            .clone();
    document["effects"] = serde_json::json!({
        "tick": [{"effect": {"type": "minecraft:all_of", "effects": [
            {"type": "minecraft:apply_mob_effect", "to_apply": "minecraft:glowing",
                "min_duration": 30, "max_duration": 30, "min_amplifier": 0, "max_amplifier": 0},
            {"type": "minecraft:spawn_particles",
                "particle": {"type": "minecraft:dust", "color": 16711680, "scale": 1.0},
                "horizontal_position": {"type": "entity_position"},
                "vertical_position": {"type": "entity_position"},
                "horizontal_velocity": {}, "vertical_velocity": {},
                "speed": {"type": "minecraft:clamped_normal", "mean": 0.5, "deviation": 0.1, "min": 0.0, "max": 1.0}},
            {"type": "minecraft:spawn_particles",
                "particle": {"type": "minecraft:flame"},
                "horizontal_position": {"type": "entity_position"}, "vertical_position": {"type": "entity_position"},
                "horizontal_velocity": {"base":null}, "vertical_velocity": {"base":null}, "speed":null},
            {"type": "minecraft:play_sound", "sound": "minecraft:entity.blaze.shoot",
                "volume": {"type": "minecraft:trapezoid", "min": 0.5, "max": 1.0, "plateau": 0.2},
                "pitch": 1.0}
        ]}}]
    });
    std::fs::write(
        pack.join("data/minecraft/enchantment/frost_walker.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(dir.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    let mut fixture = Fixture::with_config(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..datapack_config()
    });
    fixture.set(-4, -1, 0, "minecraft:stone");
    let husk = fixture.spawn_husk(-4, 0);
    fixture.run("item replace entity @e[type=husk,limit=1] armor.feet with iron_boots[enchantments={frost_walker:1}]");
    let shoot = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.blaze.shoot",
    )
    .unwrap();
    let result = fixture.world.tick(Vec::new());
    let dust = result.events.iter().any(|event| {
        matches!(event, WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::LevelParticles(particles),
        } if particles.particle.particle_type_id == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_DUST_ID
            && !particles.particle.raw_options.is_empty()
            && (0.0..=1.0).contains(&particles.max_speed))
    });
    assert!(dust, "the dust particle is sent with its options");
    assert!(result.events.iter().any(|event| matches!(event, WorldEvent::Send {
        connection_id: ALICE, packet: ClientboundPlayPacket::LevelParticles(particles),
    } if particles.particle.particle_type_id == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_FLAME_ID && particles.max_speed == 0.0)), "null optional providers use zero");
    let sound = result.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::Sound(sound),
        } if matches!(
            sound.sound,
            bbb_protocol::packets::SoundEventHolder::Reference { registry_id } if registry_id == shoot
        ) => Some(sound.volume),
        _ => None,
    });
    assert!(
        sound.is_some_and(|volume| (0.5..=1.0).contains(&volume)),
        "the sound plays at a trapezoid volume: {sound:?}"
    );
    assert!(fixture
        .world
        .stored_entity_for_test(OVERWORLD, husk)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:glowing")
        .is_some());
    std::fs::remove_dir_all(dir).unwrap();
}
