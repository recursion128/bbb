//! What a break costs and what it pays: tool durability, exhaustion and food,
//! experience orbs, and the loot the block leaves on the ground.

use super::*;

/// A `DataComponents.TOOL` that mines the fixture floor in a single tick.
///
/// `Tool.getMiningSpeed` (`Tool.java:41`) takes the first rule whose blocks
/// hold the state, so one rule naming `minecraft:stone` is the whole tool;
/// `correct_for_drops` is the `Tool.isCorrectForDrops` answer
/// (`Tool.java:51`) that `Player.hasCorrectToolForDrops` (`Player.java:618-620`)
/// ends up reading, which is exactly the gate at
/// `ServerPlayerGameMode.java:297-299`.
pub(super) fn instant_stone_tool(item: &str, correct_for_drops: bool) -> ItemStackSummary {
    instant_tool_for(
        item,
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
        correct_for_drops,
    )
}

/// The same tool, aimed at one named block rather than at stone.
fn instant_tool_for(
    item: &str,
    block_registry_id: i32,
    correct_for_drops: bool,
) -> ItemStackSummary {
    let mut tool = ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(item)
                .unwrap_or_else(|| panic!("the generated registry carries {item}"))
                .id,
        ),
        count: 1,
        ..ItemStackSummary::empty()
    };
    tool.component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID);
    tool.component_patch.tool = Some(bbb_protocol::packets::ToolSummary {
        rules: vec![bbb_protocol::packets::ToolRuleSummary {
            block_registry_ids: vec![block_registry_id],
            block_tag: None,
            speed_bits: 1_000.0_f32.to_bits().into(),
            correct_for_drops: Some(correct_for_drops),
        }],
        default_mining_speed_bits: 1.0_f32.to_bits(),
        damage_per_block: 1,
        can_destroy_blocks_in_creative: true,
    });
    tool
}

/// Puts `holder_id`-resolved enchantments on a stack the way a decoded
/// `minecraft:enchantments` component carries them.
fn enchant(world: &ServerWorld, tool: &mut ItemStackSummary, enchantments: &[(&str, i32)]) {
    let registry: Vec<String> = world
        .datapacks()
        .expect("the mining world has a datapack generation")
        .registries
        .registry("minecraft:enchantment")
        .expect("the generation carries the enchantment registry")
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect();
    tool.component_patch.enchantments = enchantments
        .iter()
        .map(
            |(id, level)| bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: i32::try_from(
                    registry
                        .iter()
                        .position(|entry| entry == id)
                        .unwrap_or_else(|| panic!("the loaded registry carries {id}")),
                )
                .unwrap(),
                level: *level,
            },
        )
        .collect();
    tool.component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
}

/// Joins one session holding `tool` and, unless it stays creative, moves it to
/// survival the one way this server can while keeping the inventory:
/// `MinecraftServer.enforceGameTypeForPlayers` (`MinecraftServer.java:1389-1403`).
pub(super) fn miner_holding(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    tool: ItemStackSummary,
    survival: bool,
) {
    join_loaded(sender, receiver, world, 1, "miner", 1);
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: tool,
        }),
    );
    if survival {
        chat_command(sender, receiver, world, 1, "defaultgamemode survival");
    }
}

/// The one `StartDestroyBlock` that finishes the break, because the tool is
/// fast enough for `:201-202` to insta-mine.
fn swing_at(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pos: BlockPos,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos,
            direction: Direction::Up,
            sequence: 1,
        }),
    );
}

/// The item entities one save request writes, as `(item id, count)`.
fn dropped_items(world: &mut ServerWorld) -> Vec<(String, i32)> {
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    saved_entities(&request)
        .iter()
        .map(|entity| {
            assert_eq!(
                field(entity, "id"),
                &NbtSummaryValue::string("minecraft:item"),
                "every entity a break leaves behind is an item entity"
            );
            let NbtSummaryValue::Compound(stack) = field(entity, "Item") else {
                panic!("Item is a compound");
            };
            let NbtSummaryValue::String(id) = field(stack, "id") else {
                panic!("an item stack names its item");
            };
            let NbtSummaryValue::Int(count) = field(stack, "count") else {
                panic!("an item stack carries a count");
            };
            (id.try_to_string().unwrap(), *count)
        })
        .collect()
}

/// Puts a `minecraft:damage` on a stack the way a used tool carries it.
fn damaged(mut tool: ItemStackSummary, damage: i32) -> ItemStackSummary {
    tool.component_patch.damage = Some(damage);
    tool.component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    tool
}

/// The stack the menu broadcast last published for the held hotbar slot.
///
/// `ServerPlayer.tick`'s `containerMenu.broadcastChanges` is the only thing
/// that tells a client its own inventory moved, so this is what a durability
/// change looks like from the outside.
fn republished_held_stack(tick: &TickResult) -> Option<ItemStackSummary> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
                ..
            } if update.slot == 36 => Some(update.item.clone()),
            _ => None,
        })
        .next_back()
}

/// Every statistic the session has marked dirty, keyed `"<type>/<value>"`.
fn requested_stats(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> std::collections::BTreeMap<String, i32> {
    let stats = play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestStats),
    );
    let update = stats
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::AwardStats(update),
                ..
            } => Some(update),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{stats:?}"));
    update
        .stats
        .iter()
        .map(|update| {
            let key = bbb_server_types::stats::StatKey::new(update.stat_type_id, update.value_id);
            let (stat_type, value) = key
                .resource_ids()
                .expect("a reported statistic is nameable");
            (format!("{stat_type}/{value}"), update.amount)
        })
        .collect()
}

/// `ItemStack.mineBlock` (`ItemStack.java:557-562`) over `Item.mineBlock`
/// (`Item.java:257-268`): the break costs the tool `Tool.damagePerBlock`
/// durability and counts one `Stats.ITEM_USED`, and `Block.playerDestroy`
/// (`Block.java:478`) counts one `Stats.BLOCK_MINED` for the block itself.
#[test]
fn a_survival_break_wears_the_held_tool_and_counts_the_use_and_the_block() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    miner_holding(
        &sender,
        &receiver,
        &mut world,
        instant_stone_tool("minecraft:diamond_pickaxe", true),
        true,
    );
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    let broke = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: floor,
            direction: Direction::Up,
            sequence: 1,
        }),
    );

    let held = republished_held_stack(&broke).expect("the worn tool is republished");
    assert_eq!(
        held.component_patch.damage,
        Some(1),
        "one break spends one point of `Tool.damagePerBlock`: {held:?}"
    );
    assert_eq!(held.count, 1, "the tool is nowhere near broken: {held:?}");

    let reported = requested_stats(&sender, &receiver, &mut world);
    assert_eq!(
        reported.get("minecraft:used/minecraft:diamond_pickaxe"),
        Some(&1),
        "{reported:?}"
    );
    assert_eq!(
        reported.get("minecraft:mined/minecraft:stone"),
        Some(&1),
        "{reported:?}"
    );
    assert_eq!(
        reported.get("minecraft:broken/minecraft:diamond_pickaxe"),
        None,
        "nothing broke: {reported:?}"
    );
}

/// `applyDamage` (`ItemStack.java:462-473`): the damage that reaches
/// `getMaxDamage` shrinks the stack by one, which for a tool empties it, and
/// the empty hand is what the client is told about.
///
/// The break then runs `ServerPlayer.onEquippedItemBroken`
/// (`ServerPlayer.java:2200-2203`): entity event `47`
/// (`LivingEntity.java:3768-3782`) to every tracking session and itself, and
/// one `Stats.ITEM_BROKEN`.
#[test]
fn the_last_point_of_durability_snaps_the_tool_and_tells_every_session() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    // `minecraft:diamond_pickaxe` carries `max_damage` in its prototype, so the
    // last usable point is the one below it.
    let max_damage = crate::inventory::item_prototype_int_component(
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:diamond_pickaxe")
            .expect("the generated registry carries the pickaxe")
            .id,
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    )
    .expect("a pickaxe is damageable");
    miner_holding(
        &sender,
        &receiver,
        &mut world,
        damaged(
            instant_stone_tool("minecraft:diamond_pickaxe", true),
            max_damage - 1,
        ),
        true,
    );
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    let broke = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: floor,
            direction: Direction::Up,
            sequence: 1,
        }),
    );

    let held = republished_held_stack(&broke).expect("the emptied slot is republished");
    assert_eq!(
        held,
        ItemStackSummary::empty(),
        "the tool is gone from the hand: {held:?}"
    );
    let breaks: Vec<i8> = broke
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityEvent(update),
                ..
            } => Some(update.event_id),
            _ => None,
        })
        .collect();
    assert_eq!(
        breaks,
        vec![crate::item_durability::ENTITY_EVENT_MAINHAND_ITEM_BROKEN],
        "the mainhand break event is announced once"
    );
    // `:297-299` still ran: the copy the loot table reads was taken before the
    // damage (`:294`), so a tool that broke on its last block still drops it.
    assert_eq!(
        dropped_items(&mut world),
        vec![("minecraft:cobblestone".to_string(), 1)]
    );

    let reported = requested_stats(&sender, &receiver, &mut world);
    assert_eq!(
        reported.get("minecraft:broken/minecraft:diamond_pickaxe"),
        Some(&1),
        "{reported:?}"
    );
    assert_eq!(
        reported.get("minecraft:used/minecraft:diamond_pickaxe"),
        Some(&1),
        "a tool that broke still counted the use that broke it: {reported:?}"
    );
}

/// `processDurabilityChange` (`ItemStack.java:455-456`) refuses on
/// `hasInfiniteMaterials`, and `destroyBlock` never even gets there: it returns
/// at `:289-291`. Either way a creative break costs no durability and counts no
/// use, because `mineBlock` is below the return.
#[test]
fn a_creative_break_leaves_the_held_tool_whole() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    miner_holding(
        &sender,
        &receiver,
        &mut world,
        instant_stone_tool("minecraft:diamond_pickaxe", true),
        false,
    );
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    let broke = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: floor,
            direction: Direction::Up,
            sequence: 1,
        }),
    );

    assert_eq!(
        republished_held_stack(&broke),
        None,
        "an untouched slot costs no packet"
    );
    let reported = requested_stats(&sender, &receiver, &mut world);
    assert_eq!(
        reported.get("minecraft:used/minecraft:diamond_pickaxe"),
        None,
        "{reported:?}"
    );
    assert_eq!(
        reported.get("minecraft:mined/minecraft:stone"),
        None,
        "a creative break earns no `Stats.BLOCK_MINED` either: {reported:?}"
    );
}

/// `minecraft:unbreaking` is the only element of the built-in enchantment
/// registry that declares an `EnchantmentEffectComponents.ITEM_DAMAGE`, and its
/// non-armour arm removes each point with probability `level / (level + 1)`
/// (`unbreaking.json`, `RemoveBinomial.java:14-31`).
///
/// The draw is a real one from `Level.random`, so the assertion is over a run
/// of breaks rather than over a single outcome: at level three, most of the
/// points are removed and some are not. One row of six is short enough that
/// every point surviving is a plain coin flip, so the row is broken and
/// restored three times over.
#[test]
fn unbreaking_removes_most_of_the_durability_a_run_of_breaks_would_cost() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    let mut tool = instant_stone_tool("minecraft:diamond_pickaxe", true);
    enchant(&world, &mut tool, &[("minecraft:unbreaking", 3)]);
    miner_holding(&sender, &receiver, &mut world, tool, true);

    // Six blocks in a row is as far as the session can reach:
    // `ServerGamePacketListenerImpl` closes a connection that asks for a break
    // outside its interaction range. The row is put back between passes, which
    // is what makes the run long enough to measure the enchantment.
    const PASSES: i32 = 3;
    const ROW: i32 = 6;
    let breaks = PASSES * ROW;
    let row: Vec<BlockPos> = (0..ROW).map(|x| BlockPos { x, y: 63, z: 0 }).collect();
    let floor: Vec<i32> = row
        .iter()
        .map(|&pos| live_block(&mut world, pos).expect("the fixture floor stands here"))
        .collect();
    let mut damage = None;
    let mut sequence = 0;
    for _ in 0..PASSES {
        for (&pos, &state) in row.iter().zip(floor.iter()) {
            world.set_block(
                Java26BuiltinDimension::Overworld,
                pos,
                state,
                SET_BLOCK_UPDATE_ALL,
            );
            sequence += 1;
            let broke = play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::PlayerAction(PlayerAction {
                    action: PlayerActionKind::StartDestroyBlock,
                    pos,
                    direction: Direction::Up,
                    sequence,
                }),
            );
            if let Some(held) = republished_held_stack(&broke) {
                damage = held.component_patch.damage;
            }
        }
    }

    let damage = damage.expect("the run wears the tool at least once");
    // `Stats.ITEM_USED` counts one per `Item.mineBlock` that saw a tool
    // (`ItemStack.java:559-560`), which is the number of points the same run
    // would have cost with no enchantment on the stack.
    let reported = requested_stats(&sender, &receiver, &mut world);
    let used = reported
        .get("minecraft:used/minecraft:diamond_pickaxe")
        .copied()
        .expect("the run mined with the tool");
    assert_eq!(used, breaks, "every swing broke its block: {reported:?}");
    assert!(
        damage > 0 && damage < used,
        "unbreaking removed some but not all of the {used} points: {damage}"
    );
}

/// The same fixture document with its four `FoodData` fields set.
///
/// `FoodData.readAdditionalSaveData` (`FoodData.java:74-79`) is four raw field
/// assignments, so a document is the only way to put a session on a hunger
/// state that is not the default: nothing this server accepts from a client
/// moves the food bar, which is the point -- vanilla's does not either.
fn with_food(
    player: Java26PlayerData,
    level: i32,
    tick_timer: i32,
    saturation: f32,
    exhaustion: f32,
) -> Java26PlayerData {
    let uuid = player.uuid();
    let mut raw = player.into_raw();
    for entry in raw.entries_mut().iter_mut() {
        match entry.name.to_jdk_string().as_str() {
            "foodLevel" => entry.value = NbtSummaryValue::Int(level),
            "foodTickTimer" => entry.value = NbtSummaryValue::Int(tick_timer),
            "foodSaturationLevel" => entry.value = nbt_float(saturation),
            "foodExhaustionLevel" => entry.value = nbt_float(exhaustion),
            _ => {}
        }
    }
    Java26PlayerData::parse(raw, uuid).unwrap()
}

/// A survival world holding one session whose hunger the save decided.
fn hungry_world(level: i32, tick_timer: i32, saturation: f32, exhaustion: f32) -> ServerWorld {
    let uuid = profile("miner", 1).uuid;
    let candidate = restore_candidate(
        0,
        None,
        vec![(
            uuid,
            with_food(
                restore_fixture_player(uuid, [0.5, 65.0, 0.5], [0.0, 0.0], true),
                level,
                tick_timer,
                saturation,
                exhaustion,
            ),
        )],
    );
    ServerWorld::restore(mining_config(), &candidate).unwrap()
}

/// `Block.playerDestroy` (`Block.java:479`) charges
/// `player.causeFoodExhaustion(0.005F)` for every block a survival session
/// breaks, and `Player.causeFoodExhaustion` (`Player.java:1561-1567`) gates it
/// on `abilities.invulnerable` -- which is what keeps a creative session's bar
/// still.
#[test]
fn every_survival_break_charges_the_exhaustion_player_destroy_pays() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    let tool = instant_stone_tool("minecraft:diamond_pickaxe", true);
    miner_holding(&sender, &receiver, &mut world, tool, true);
    assert_eq!(live_food(&mut world).exhaustion_level(), 0.0);

    let breaks = 4;
    for x in 0..breaks {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: BlockPos { x, y: 63, z: 0 },
                direction: Direction::Up,
                sequence: 1 + x,
            }),
        );
    }
    // Four breaks well under the four points one cascade step costs, so the
    // charge is still standing on the bar rather than already spent.
    assert_eq!(
        live_food(&mut world).exhaustion_level(),
        breaks as f32 * bbb_server_types::food::EXHAUSTION_MINE
    );

    // The same run in creative: `ServerPlayerGameMode` never reaches
    // `playerDestroy` for an instant-build session, and the ability gate would
    // refuse the charge even if it did.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    let tool = instant_stone_tool("minecraft:diamond_pickaxe", true);
    miner_holding(&sender, &receiver, &mut world, tool, false);
    for x in 0..breaks {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: BlockPos { x, y: 63, z: 0 },
                direction: Direction::Up,
                sequence: 1 + x,
            }),
        );
    }
    assert_eq!(live_food(&mut world).exhaustion_level(), 0.0);
}

/// The third term of `ServerPlayer.doTick`'s health comparison
/// (`ServerPlayer.java:673`) is whether the saturation *is zero*, not the
/// saturation, so a saturation that reaches zero sends
/// `ClientboundSetHealthPacket` on its own -- with the health and the food
/// standing exactly where they were.
#[test]
fn a_saturation_reaching_zero_sends_set_health_on_its_own() {
    let (sender, receiver) = mailbox();
    // Exactly the four points `FoodData.tick`'s cascade needs, which its `>`
    // (`FoodData.java:35`) does not fire on, so the break below is what tips
    // it -- and nothing before the break can.
    let mut world = hungry_world(20, 0, 0.5, bbb_server_types::food::EXHAUSTION_DROP);
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            // The restored document selects hotbar slot 1, so container slot
            // 37 is the hand.
            slot_num: 37,
            item: instant_stone_tool("minecraft:diamond_pickaxe", true),
        }),
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    let settled = world.tick(receiver.drain());
    assert!(set_healths(&settled).is_empty(), "{settled:?}");

    let broke = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: BlockPos { x: 0, y: 63, z: 0 },
            direction: Direction::Up,
            sequence: 1,
        }),
    );
    let food = live_food(&mut world);
    // `:37-38`: the cascade spends the saturation before it touches the food.
    assert_eq!(food.saturation_level(), 0.0);
    assert_eq!(food.food_level(), 20);
    let sent = broke
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SetHealth(update),
            } => Some(*update),
            _ => None,
        })
        .expect("the saturation bit moved, so the session is told");
    assert_eq!(
        sent.health,
        bbb_server_datapack::damage::player_max_health()
    );
    assert_eq!(sent.food, 20);
    assert_eq!(sent.saturation, 0.0);
}

/// The second term of the same comparison (`ServerPlayer.java:672`) is the food
/// level, which the cascade moves once the saturation is gone
/// (`FoodData.java:39`).
#[test]
fn a_food_level_the_cascade_moved_sends_set_health() {
    let (sender, receiver) = mailbox();
    // The saturation is already gone, so the same tipping charge takes a point
    // of food instead (`FoodData.java:39`).
    let mut world = hungry_world(20, 0, 0.0, bbb_server_types::food::EXHAUSTION_DROP);
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            // The restored document selects hotbar slot 1, so container slot
            // 37 is the hand.
            slot_num: 37,
            item: instant_stone_tool("minecraft:diamond_pickaxe", true),
        }),
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    world.tick(receiver.drain());

    let broke = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: BlockPos { x: 0, y: 63, z: 0 },
            direction: Direction::Up,
            sequence: 1,
        }),
    );
    assert_eq!(live_food(&mut world).food_level(), 19);
    let sent = broke
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SetHealth(update),
            } => Some(*update),
            _ => None,
        })
        .expect("the food level moved, so the session is told");
    assert_eq!(sent.food, 19);
    assert_eq!(
        sent.health,
        bbb_server_datapack::damage::player_max_health()
    );
}

/// `FoodData.tick`'s starvation arm (`FoodData.java:60-68`) waits eighty ticks
/// on an empty bar and then asks for
/// `player.hurtServer(level, damageSources().starve(), 1.0F)`, which this world
/// runs through the same hurt pipeline `/damage` does.
#[test]
fn an_empty_food_bar_starves_the_session_after_eighty_ticks() {
    let (sender, receiver) = mailbox();
    let mut world = hungry_world(0, 0, 0.0, 0.0);
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    // The join and the command already ran the hunger tick, so the counter is
    // read off the save rather than assumed.
    let already = live_food(&mut world).tick_timer();
    let mut ticks = 0;
    let starved = loop {
        let tick = world.tick(receiver.drain());
        ticks += 1;
        if !set_healths(&tick).is_empty() {
            break tick;
        }
        assert!(ticks < 200, "the empty bar never starved");
    };
    assert_eq!(ticks + already, 80, "`FoodData.java:62`'s counter");
    assert_eq!(
        set_healths(&starved),
        [(
            ConnectionId(1),
            bbb_server_datapack::damage::player_max_health()
                - bbb_server_types::food::STARVE_DAMAGE
        )]
    );
    // `:63` floors the damage on the two lower difficulties, and the timer
    // restarts either way (`:62`).
    assert_eq!(live_food(&mut world).tick_timer(), 0);
    assert_eq!(live_food(&mut world).food_level(), 0);
}

/// A hunger state that is not the default has to survive the save and the
/// restart, which is `FoodData.addAdditionalSaveData` (`FoodData.java:81-86`)
/// against `readAdditionalSaveData` (`:74-79`).
#[test]
fn the_food_a_run_of_breaks_left_survives_a_save_and_a_restart() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-food-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-food");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_restore(mining_config(), &candidate);
    let tool = instant_stone_tool("minecraft:diamond_pickaxe", true);
    miner_holding(&sender, &receiver, &mut world, tool, true);
    let breaks = 3;
    for x in 0..breaks {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos: BlockPos { x, y: 63, z: 0 },
                direction: Direction::Up,
                sequence: 1 + x,
            }),
        );
    }
    let earned = live_food(&mut world);
    assert_eq!(
        earned.exhaustion_level(),
        breaks as f32 * bbb_server_types::food::EXHAUSTION_MINE
    );

    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let (_, saved) = reloaded
        .load_player(profile("miner", 1).uuid)
        .unwrap()
        .expect("the mining session's save wrote its playerdata");
    assert_eq!(saved.food_data(), earned);

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(mining_config(), &reloaded).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    assert_eq!(live_food(&mut world), earned);
    storage.close().unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

/// Every entity one save request writes, as `(entity id, its own fields)`.
fn saved_entity_records(world: &mut ServerWorld) -> Vec<(String, Vec<NbtSummaryEntry>)> {
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    saved_entities(&request)
        .iter()
        .map(|entity| {
            let NbtSummaryValue::String(id) = field(entity, "id") else {
                panic!("an entity record names its type");
            };
            (id.try_to_string().unwrap(), entity.clone())
        })
        .collect()
}

/// Puts a nether quartz ore where the miner can reach it.
///
/// It is the ore this suite breaks because its provider is `UniformInt.of(2, 5)`
/// (`Blocks.java`, the `nether_quartz_ore` registration): every sample is
/// positive, so the `experience > 0` gate at `Block.java:622` never turns the
/// test into a coin flip the way `coal_ore`'s `UniformInt.of(0, 2)` would.
fn quartz_ore_in_reach(world: &mut ServerWorld) -> BlockPos {
    // Three blocks out: inside the session's interaction range but outside the
    // pickup box `Player.aiStep` inflates around it (`Player.java:462`), so the
    // orbs the break leaves are still lying there on the tick it happened.
    let pos = BlockPos { x: 3, y: 63, z: 0 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        block_state_id("minecraft:nether_quartz_ore"),
        SET_BLOCK_UPDATE_ALL,
    ));
    pos
}

fn quartz_ore_pickaxe(correct_for_drops: bool) -> ItemStackSummary {
    instant_tool_for(
        "minecraft:diamond_pickaxe",
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(
            "minecraft:nether_quartz_ore",
        )
        .expect("the generated catalog carries nether quartz ore")
        .id,
        correct_for_drops,
    )
}

/// `Block.dropResources`' second line (`Block.java:413`) through
/// `DropExperienceBlock.spawnAfterBreak` (`DropExperienceBlock.java:30-35`),
/// `Block.tryDropExperience` (`Block.java:620-625`) and
/// `Block.popExperience` (`:450-454`): a nether quartz ore pays
/// `UniformInt.of(2, 5)` experience, and the orbs it becomes are real entities
/// the save can hold.
#[test]
fn breaking_an_ore_leaves_experience_orbs_the_save_can_hold() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    miner_holding(
        &sender,
        &receiver,
        &mut world,
        quartz_ore_pickaxe(true),
        true,
    );
    let ore = quartz_ore_in_reach(&mut world);
    swing_at(&sender, &receiver, &mut world, ore);

    let records = saved_entity_records(&mut world);
    let orbs: Vec<&Vec<NbtSummaryEntry>> = records
        .iter()
        .filter(|(id, _)| id == "minecraft:experience_orb")
        .map(|(_, record)| record)
        .collect();
    assert!(!orbs.is_empty(), "the break paid experience: {records:?}");
    let mut total = 0;
    for orb in &orbs {
        // `ExperienceOrb.addAdditionalSaveData` (`ExperienceOrb.java:259-264`),
        // which the entity-region schema also requires.
        let NbtSummaryValue::Short(value) = field(orb, "Value") else {
            panic!("an orb carries a short Value: {orb:?}");
        };
        let NbtSummaryValue::Int(count) = field(orb, "Count") else {
            panic!("an orb carries an int Count: {orb:?}");
        };
        let NbtSummaryValue::Short(health) = field(orb, "Health") else {
            panic!("an orb carries a short Health: {orb:?}");
        };
        let NbtSummaryValue::Short(age) = field(orb, "Age") else {
            panic!("an orb carries a short Age: {orb:?}");
        };
        assert_eq!(
            *health, 5,
            "the field initialiser (`ExperienceOrb.java:42`)"
        );
        assert!(*age >= 0, "{age}");
        assert!(*count > 0, "the schema refuses a non-positive Count");
        total += i32::from(*value) * count;
    }
    // `UniformInt.of(2, 5)` bounds what one break can pay, and the split into
    // orbs (`ExperienceOrb.awardWithDirection`, `:191-199`) conserves it.
    assert!((2..=5).contains(&total), "{total} from {orbs:?}");
}

/// `processPacketsAndTick` drains the inbound packet queue before
/// `tickServer` (`MinecraftServer.java:1000-1010`), while entity additions
/// synchronously enter `ChunkMap` from the level callback. An instant break
/// therefore publishes the entities made by that one mailbox command in the
/// same `TickResult`; the later phase drains must not pair them again.
#[test]
fn one_mailbox_break_publishes_its_entities_once_in_the_same_tick_result() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    miner_holding(
        &sender,
        &receiver,
        &mut world,
        quartz_ore_pickaxe(true),
        true,
    );
    let ore = quartz_ore_in_reach(&mut world);
    let broken = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: ore,
            direction: Direction::Up,
            sequence: 1,
        }),
    );

    let added = broken
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::AddEntity(add),
            } => Some((add.id, add.entity_type_id)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        added.iter().any(|(_, entity_type_id)| *entity_type_id
            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID),
        "the command's experience orb is in its own result: {broken:?}"
    );
    let mut unique_ids = added.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    unique_ids.sort_unstable();
    unique_ids.dedup();
    assert_eq!(
        unique_ids.len(),
        added.len(),
        "later phase drains do not repeat a pairing: {added:?}"
    );

    let settled = world.tick(receiver.drain());
    assert!(
        !settled.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::AddEntity(add),
                ..
            } if unique_ids.contains(&add.id)
        )),
        "the command journal was fully consumed in its own tick: {settled:?}"
    );
}

/// `EnchantmentHelper.processBlockExperience` (`EnchantmentHelper.java:102-106`)
/// with silk touch on the tool: `silk_touch.json` declares a
/// `minecraft:block_experience` effect of `{"type": "minecraft:set", "value":
/// 0.0}`, so the amount reaches zero and `Block.java:622` drops it.
#[test]
fn a_silk_touched_ore_pays_no_experience() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    let mut tool = quartz_ore_pickaxe(true);
    enchant(&world, &mut tool, &[("minecraft:silk_touch", 1)]);
    miner_holding(&sender, &receiver, &mut world, tool, true);
    let ore = quartz_ore_in_reach(&mut world);
    swing_at(&sender, &receiver, &mut world, ore);

    let records = saved_entity_records(&mut world);
    assert!(
        records
            .iter()
            .all(|(id, _)| id != "minecraft:experience_orb"),
        "silk touch left no orb: {records:?}"
    );
    // The drop itself is still there, and it is the ore rather than its loot.
    assert!(
        records.iter().any(|(id, _)| id == "minecraft:item"),
        "{records:?}"
    );
}

/// `Player.preventsBlockDrops` returns at `ServerPlayerGameMode.java:289-291`,
/// which is above `playerDestroy` -- so a creative break reaches neither
/// `dropResources` nor the `spawnAfterBreak` inside it.
#[test]
fn a_creative_break_pays_no_experience() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    miner_holding(
        &sender,
        &receiver,
        &mut world,
        quartz_ore_pickaxe(true),
        false,
    );
    let ore = quartz_ore_in_reach(&mut world);
    swing_at(&sender, &receiver, &mut world, ore);

    assert_eq!(saved_entity_records(&mut world), Vec::new());
}

/// `Player.aiStep`'s experience-orb arm (`Player.java:468-478`) over
/// `ExperienceOrb.playerTouch` (`ExperienceOrb.java:275-291`): the orb reaches
/// the session, the bar moves and the client is told
/// (`ServerPlayer.java:710-713`).
#[test]
fn an_orb_in_reach_is_absorbed_and_moves_the_players_bar() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    miner_holding(
        &sender,
        &receiver,
        &mut world,
        quartz_ore_pickaxe(true),
        true,
    );
    let ore = quartz_ore_in_reach(&mut world);
    swing_at(&sender, &receiver, &mut world, ore);
    // Stand in the pit the break leaves, so every orb is within the pickup
    // box whichever spawn impulse it drew.
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: f64::from(ore.x) + 0.5,
            y: f64::from(ore.y),
            z: f64::from(ore.z) + 0.5,
        },
    );

    // Tick until the orb has been absorbed, which takes the walk it does
    // towards the session plus the two-tick `takeXpDelay` between orbs.
    let before = saved_entity_records(&mut world)
        .iter()
        .filter(|(id, _)| id == "minecraft:experience_orb")
        .count();
    let mut experience = Vec::new();
    for _ in 0..200 {
        let tick = world.tick(receiver.drain());
        experience.extend(tick.events.iter().filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetExperience(update),
                ..
            } => Some(*update),
            _ => None,
        }));
        if saved_entity_records(&mut world)
            .iter()
            .all(|(id, _)| id != "minecraft:experience_orb")
        {
            break;
        }
    }

    let last = experience
        .last()
        .copied()
        .expect("the absorbed orb moved the bar");
    assert!(last.total > 0, "{experience:?}");
    // A quartz ore pays at most five, and the first level costs seven
    // (`Player.getXpNeededForNextLevel`, `Player.java:1553-1559`), so the bar
    // is still on level zero with a fraction of it filled.
    assert_eq!(last.level, 0, "{last:?}");
    assert_eq!(last.progress, last.total as f32 / 7.0, "{last:?}");
    // The absorbed orbs are gone.
    let after = saved_entity_records(&mut world)
        .iter()
        .filter(|(id, _)| id == "minecraft:experience_orb")
        .count();
    assert!(
        after < before,
        "absorbed orbs are gone: {before} -> {after}"
    );
}

/// `ExperienceOrb.repairPlayerItems` (`ExperienceOrb.java:293-311`) over
/// `EnchantmentHelper.getRandomItemWith` (`EnchantmentHelper.java:482-502`):
/// mending on a damaged held tool spends the orb before the bar ever sees it.
///
/// `mending.json` declares its `repair_with_xp` effect as
/// `{"type": "minecraft:multiply", "factor": 2.0}`, so one point of experience
/// is two of durability -- and with `remaining = amount - repair * amount /
/// toRepairFromXpAmount` (`:301`) reaching zero, `giveExperiencePoints` is
/// never called at all.
#[test]
fn mending_spends_the_orb_on_the_held_tool_before_the_bar_sees_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    let mut tool = damaged(quartz_ore_pickaxe(true), 100);
    enchant(&world, &mut tool, &[("minecraft:mending", 1)]);
    miner_holding(&sender, &receiver, &mut world, tool, true);
    let ore = quartz_ore_in_reach(&mut world);
    swing_at(&sender, &receiver, &mut world, ore);
    // Stand in the pit the break leaves, so every orb is within the pickup
    // box whichever spawn impulse it drew.
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: f64::from(ore.x) + 0.5,
            y: f64::from(ore.y),
            z: f64::from(ore.z) + 0.5,
        },
    );

    let mut experience = Vec::new();
    let mut damage = None;
    for _ in 0..200 {
        let tick = world.tick(receiver.drain());
        experience.extend(tick.events.iter().filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetExperience(update),
                ..
            } => Some(*update),
            _ => None,
        }));
        if let Some(held) = republished_held_stack(&tick) {
            damage = held.component_patch.damage;
        }
    }

    // The break itself spent one point (`Item.mineBlock`), and every orb since
    // has paid two back.
    let damage = damage.expect("the mended tool is republished");
    assert!(damage < 101, "mending repaired the tool: {damage}");
    assert!(
        damage >= 101 - 2 * 5,
        "no more than the ore was worth: {damage}"
    );
    assert!(
        experience.iter().all(|update| update.total == 0),
        "the bar never moved: {experience:?}"
    );
}

/// The whole point of `ServerPlayerGameMode.destroyBlock` (`:262-302`): a
/// survival break with the right tool leaves the block's loot on the ground.
///
/// `blocks/stone` with an unenchanted tool is the second alternative, so the
/// floor becomes one cobblestone (`Block.playerDestroy`, `Block.java:470-481`,
/// reaching the six-argument `dropResources` at `:480`).
#[test]
fn a_survival_break_leaves_the_blocks_loot_on_the_ground() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    miner_holding(
        &sender,
        &receiver,
        &mut world,
        instant_stone_tool("minecraft:diamond_pickaxe", true),
        true,
    );
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    swing_at(&sender, &receiver, &mut world, floor);

    assert_eq!(
        published_block(&world, floor),
        Some(world.fixture().air_state_id()),
        "the block still goes"
    );
    assert_eq!(
        dropped_items(&mut world),
        vec![("minecraft:cobblestone".to_string(), 1)]
    );
}

/// `Player.preventsBlockDrops` (`Player.java:1297-1299`) is
/// `abilities.instabuild`, and `destroyBlock` returns at `:289-291` before it
/// reads the tool at all — so a creative break removes the block and leaves
/// nothing behind, with the very same tool that drops cobblestone in survival.
#[test]
fn a_creative_break_leaves_the_ground_empty() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    miner_holding(
        &sender,
        &receiver,
        &mut world,
        instant_stone_tool("minecraft:diamond_pickaxe", true),
        false,
    );
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    swing_at(&sender, &receiver, &mut world, floor);

    assert_eq!(
        published_block(&world, floor),
        Some(world.fixture().air_state_id()),
        "creative still breaks it"
    );
    assert_eq!(dropped_items(&mut world), Vec::new());
}

/// `:297-299` runs `playerDestroy` only when `hasCorrectToolForDrops` said yes,
/// and stone is a `requiresCorrectToolForDrops()` block (`Blocks.java:54-57`).
///
/// The tool here mines just as fast; the only thing that changed is
/// `Tool.isCorrectForDrops` (`Tool.java:51`), and that alone empties the
/// ground.
#[test]
fn the_wrong_tool_breaks_the_stone_and_drops_nothing() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    miner_holding(
        &sender,
        &receiver,
        &mut world,
        instant_stone_tool("minecraft:diamond_shovel", false),
        true,
    );
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    swing_at(&sender, &receiver, &mut world, floor);

    assert_eq!(
        published_block(&world, floor),
        Some(world.fixture().air_state_id()),
        "a wrong tool still breaks the block"
    );
    assert_eq!(dropped_items(&mut world), Vec::new());
}

/// The six-argument `Block.getDrops` (`Block.java:373-386`) puts the breaking
/// stack in `LootContextParams.TOOL`, and `blocks/stone`'s first alternative
/// asks that stack for `silk_touch`.
///
/// So the enchantment on the held item is what decides whether the floor comes
/// back as stone or as cobblestone — reached here through the whole chain, from
/// the packet to the saved entity.
#[test]
fn a_silk_touched_break_drops_the_block_itself() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    let mut tool = instant_stone_tool("minecraft:diamond_pickaxe", true);
    enchant(&world, &mut tool, &[("minecraft:silk_touch", 1)]);
    miner_holding(&sender, &receiver, &mut world, tool, true);
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    swing_at(&sender, &receiver, &mut world, floor);

    assert_eq!(
        dropped_items(&mut world),
        vec![("minecraft:stone".to_string(), 1)]
    );
}

/// `Block.popResource` builds nothing when `GameRules.BLOCK_DROPS`
/// (`gamerules/GameRules.java:29`) is off (`Block.java:443`), and the block is
/// destroyed all the same: the rule suppresses the drop, not the break.
#[test]
fn block_drops_off_leaves_the_ground_clear() {
    let mut candidate = restore_candidate(10, None, Vec::new());
    candidate.game_rules.insert(
        "minecraft:block_drops".to_string(),
        bbb_protocol::game_rules::GameRuleValue::Boolean(false),
    );
    let mut world = ServerWorld::restore(datapack_config(), &candidate).unwrap();
    let plant = dandelion_on_dirt(&mut world);
    remove_the_support(&mut world);
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, plant),
        Some(block_state_id("minecraft:air")),
        "the plant is still destroyed; only the drop is suppressed"
    );
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    assert!(saved_entities(&request).is_empty());
}

/// A world with no datapack generation has no loot registry, so the drop is skipped.
#[test]
fn a_world_without_loot_tables_drops_nothing() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    dandelion_on_dirt(&mut world);
    remove_the_support(&mut world);
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    assert!(saved_entities(&request).is_empty());
}

/// A patched `minecraft:tool` whose rules all name block tags, the shape
/// `ToolMaterial.applyToolProperties` gives every vanilla tool
/// (`ToolMaterial.java:50-51`): a drop-denying `incorrect_for_*` rule ahead of
/// the `mineable/*` rule that mines and drops.
fn tagged_tool(item: &str, rules: &[(&str, Option<f32>, Option<bool>)]) -> ItemStackSummary {
    let mut tool = instant_stone_tool(item, true);
    tool.component_patch.tool.as_mut().unwrap().rules = rules
        .iter()
        .map(
            |(tag, speed, correct_for_drops)| bbb_protocol::packets::ToolRuleSummary {
                block_registry_ids: Vec::new(),
                block_tag: Some((*tag).to_string()),
                speed_bits: speed.map(|speed| speed.to_bits()),
                correct_for_drops: *correct_for_drops,
            },
        )
        .collect();
    tool
}

fn tag_mining_world(
    tags: &[(&str, &[&str], bool)],
    block: &str,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    BlockPos,
) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        force_game_mode: true,
        admin_lists: operator_config(&[("miner", 1)]).admin_lists,
        ..datapack_config_with_block_tags(tags)
    });
    let pos = BlockPos { x: 3, y: 63, z: 0 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        block_state_id(block),
        SET_BLOCK_UPDATE_ALL,
    ));
    (sender, receiver, world, pos)
}

/// `Tool.getMiningSpeed` (`Tool.java:39-47`) reads the rule's tag as loaded:
/// a pack that adds dirt to `#minecraft:mineable/pickaxe` turns a one-tick
/// break of the default-speed miss into an instant one.
#[test]
fn a_datapack_mineable_tag_decides_the_tool_speed() {
    let tool = tagged_tool(
        "minecraft:diamond_pickaxe",
        &[("minecraft:mineable/pickaxe", Some(1_000.0), Some(true))],
    );
    for (tags, broken) in [
        (&[][..], false),
        (
            &[("minecraft:mineable/pickaxe", &["minecraft:dirt"][..], false)][..],
            true,
        ),
    ] {
        let (sender, receiver, mut world, pos) = tag_mining_world(tags, "minecraft:dirt");
        miner_holding(&sender, &receiver, &mut world, tool.clone(), true);
        swing_at(&sender, &receiver, &mut world, pos);
        assert_eq!(
            published_block(&world, pos) == Some(world.fixture().air_state_id()),
            broken,
            "{tags:?}"
        );
    }
}

/// `Tool.isCorrectForDrops` (`Tool.java:49-57`) takes the first rule whose
/// loaded tag holds the block: emptying `#minecraft:incorrect_for_wooden_tool`
/// lets the `mineable` rule behind it drop iron ore's loot, and the `mineable`
/// rule itself names a tag only the pack defines.
#[test]
fn a_datapack_incorrect_for_tool_tag_decides_the_drops() {
    let tool = tagged_tool(
        "minecraft:wooden_pickaxe",
        &[
            ("minecraft:incorrect_for_wooden_tool", None, Some(false)),
            ("test:ores", Some(1_000.0), Some(true)),
        ],
    );
    let ores: (&str, &[&str], bool) = ("test:ores", &["minecraft:iron_ore"], false);
    for (tags, drops) in [
        (vec![ores], Vec::new()),
        (
            vec![ores, ("minecraft:incorrect_for_wooden_tool", &[][..], true)],
            vec![("minecraft:raw_iron".to_string(), 1)],
        ),
    ] {
        let (sender, receiver, mut world, pos) = tag_mining_world(&tags, "minecraft:iron_ore");
        miner_holding(&sender, &receiver, &mut world, tool.clone(), true);
        swing_at(&sender, &receiver, &mut world, pos);
        assert_eq!(
            published_block(&world, pos),
            Some(world.fixture().air_state_id())
        );
        assert_eq!(dropped_items(&mut world), drops, "{tags:?}");
    }
}
