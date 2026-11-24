//! Real session digs through the subclass `playerWillDestroy`, `attack` and
//! `playerDestroy` bodies, and the `GameMasterBlock` refusal in front of them.

use super::*;

use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_ID;

pub(super) const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
pub(super) const MINER: ConnectionId = ConnectionId(1);
const OBSERVER: ConnectionId = ConnectionId(2);

pub(super) fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let block = vanilla_block_type_for_resource_id(name).expect("block type");
    let default = vanilla_block_default_state_id(block.id).expect("default state");
    properties.iter().fold(default, |state, (property, value)| {
        with_property(state, property, value).expect("block state property")
    })
}

pub(super) struct Dig {
    pub(super) sender: WorldCommandSender,
    pub(super) receiver: WorldCommandReceiver,
    pub(super) world: ServerWorld,
    sequence: i32,
}

impl Dig {
    pub(super) fn new(game_type: GameType, operator: bool) -> Self {
        Self::with_config(
            ServerWorldConfig {
                game_type,
                ..datapack_config()
            },
            operator,
        )
    }

    pub(super) fn with_config(mut config: ServerWorldConfig, operator: bool) -> Self {
        let (sender, receiver) = mailbox();
        if operator {
            config.admin_lists = operator_config(&[("miner", 1)]).admin_lists;
        }
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
        join_loaded(&sender, &receiver, &mut world, 2, "observer", 2);
        world.set_player_position_for_test(
            OBSERVER,
            Vec3d {
                x: 8.5,
                y: 64.0,
                z: -1.5,
            },
        );
        world.tick(receiver.drain());
        Self {
            sender,
            receiver,
            world,
            sequence: 0,
        }
    }

    pub(super) fn set(&mut self, pos: BlockPos, block_state_id: i32) {
        assert!(self
            .world
            .set_block(DIM, pos, block_state_id, SET_BLOCK_UPDATE_ALL));
    }

    pub(super) fn hold(&mut self, item: ItemStackSummary) {
        self.world.set_inventory_slot_for_test(MINER, 0, item);
    }

    /// Stands the miner three blocks north of `pos` and sends
    /// `START_DESTROY_BLOCK` for it.
    pub(super) fn dig(&mut self, pos: BlockPos) -> TickResult {
        self.world.set_player_position_for_test(
            MINER,
            Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: 64.0,
                z: f64::from(pos.z) - 2.5,
            },
        );
        self.world.tick(self.receiver.drain());
        self.sequence += 1;
        play(
            &self.sender,
            &self.receiver,
            &mut self.world,
            1,
            PlayServerbound::PlayerAction(PlayerAction {
                action: PlayerActionKind::StartDestroyBlock,
                pos,
                direction: Direction::Up,
                sequence: self.sequence,
            }),
        )
    }

    pub(super) fn command(&mut self, command: &str) -> TickResult {
        chat_command(&self.sender, &self.receiver, &mut self.world, 1, command)
    }

    pub(super) fn place(&mut self, support: BlockPos) -> TickResult {
        self.sequence += 1;
        play(
            &self.sender,
            &self.receiver,
            &mut self.world,
            1,
            PlayServerbound::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos: support,
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 1.0,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: self.sequence,
            }),
        )
    }

    pub(super) fn block(&mut self, pos: BlockPos) -> Option<i32> {
        self.world.block_state(DIM, pos)
    }

    fn items(&self) -> Vec<(String, i32)> {
        self.world
            .stored_items_for_test(DIM)
            .into_iter()
            .map(|(_, _, id, count, _, _)| (id, count))
            .collect()
    }

    fn game_events_at(&self, pos: BlockPos) -> Vec<RecordedGameEvent> {
        self.world
            .game_events(DIM)
            .0
            .into_iter()
            .filter(|event| event.pos == pos)
            .collect()
    }

    pub(super) fn enchanted(
        &self,
        mut stack: ItemStackSummary,
        enchantment: &str,
    ) -> ItemStackSummary {
        let holder_id = self
            .world
            .datapacks()
            .expect("datapack generation")
            .registries
            .registry("minecraft:enchantment")
            .expect("enchantment registry")
            .elements
            .iter()
            .position(|entry| entry.id == enchantment)
            .expect("enchantment holder");
        stack
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
        stack.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: i32::try_from(holder_id).unwrap(),
            level: 1,
        }];
        stack
    }
}

pub(super) fn item(resource_id: &str) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)
                .expect("item type")
                .id,
        ),
        count: 1,
        ..ItemStackSummary::empty()
    }
}

/// A stack whose `minecraft:tool` breaks `blocks` on the first swing, and
/// counts as the correct tool for their drops only when `correct` is set.
pub(super) fn fast_tool(resource_id: &str, blocks: &[&str], correct: bool) -> ItemStackSummary {
    let mut tool = item(resource_id);
    tool.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_TOOL_TYPE_ID);
    tool.component_patch.tool = Some(bbb_protocol::packets::ToolSummary {
        rules: vec![bbb_protocol::packets::ToolRuleSummary {
            block_registry_ids: blocks
                .iter()
                .map(|name| vanilla_block_type_for_resource_id(name).expect("block").id)
                .collect(),
            block_tag: None,
            speed_bits: 1_000.0_f32.to_bits().into(),
            correct_for_drops: Some(correct),
        }],
        default_mining_speed_bits: 1.0_f32.to_bits(),
        damage_per_block: 0,
        can_destroy_blocks_in_creative: true,
    });
    tool
}

fn level_events_at(tick: &TickResult, pos: BlockPos) -> Vec<(ConnectionId, i32, i32)> {
    level_events_sent(tick)
        .into_iter()
        .filter(|(_, event)| event.pos == pos)
        .map(|(connection_id, event)| (connection_id, event.event_type, event.data))
        .collect()
}

/// Both block packets: a lone change or one of a section's batch.
fn block_updates_at(tick: &TickResult, pos: BlockPos) -> Vec<(ConnectionId, i32)> {
    tick.events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .flat_map(|(connection_id, packet)| match packet {
            ClientboundPlayPacket::BlockUpdate(update) => vec![(connection_id, *update)],
            ClientboundPlayPacket::SectionBlocksUpdate(section) => section
                .updates
                .iter()
                .map(|update| (connection_id, *update))
                .collect(),
            _ => Vec::new(),
        })
        .filter(|(_, update)| update.pos == pos)
        .map(|(connection_id, update)| (connection_id, update.block_state_id))
        .collect()
}

/// `ServerPlayerGameMode.destroyBlock` (`ServerPlayerGameMode.java:270-273`):
/// a creative session below `Permissions.COMMANDS_GAMEMASTER` has the command
/// block resent to every tracker instead of broken, and an operator breaks it.
#[test]
fn a_game_master_block_refuses_a_creative_non_operator_and_breaks_for_an_operator() {
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    let command_block = state("minecraft:command_block", &[]);

    let mut refused = Dig::new(GameType::Creative, false);
    refused.set(pos, command_block);
    let tick = refused.dig(pos);
    assert_eq!(refused.block(pos), Some(command_block));
    let updates = block_updates_at(&tick, pos);
    assert!(updates.contains(&(MINER, command_block)), "{updates:?}");
    assert!(updates.contains(&(OBSERVER, command_block)), "{updates:?}");
    assert!(level_events_at(&tick, pos).is_empty());
    assert!(refused.game_events_at(pos).is_empty());

    let mut operator = Dig::new(GameType::Creative, true);
    operator.set(pos, command_block);
    let tick = operator.dig(pos);
    assert_eq!(operator.block(pos), Some(block_state_id("minecraft:air")));
    assert_eq!(
        level_events_at(&tick, pos),
        vec![(OBSERVER, 2001, command_block)]
    );
}

/// `DoorBlock.playerWillDestroy` (`DoorBlock.java:122-128`) removes the lower
/// half without drops for a creative breaker. No 26.1 door requires the
/// correct tool, so a survival break leaves the lower half to drop itself.
#[test]
fn a_creative_door_upper_half_break_removes_the_lower_half_without_a_drop() {
    let lower = BlockPos { x: 2, y: 64, z: 2 };
    let upper = BlockPos { x: 2, y: 65, z: 2 };
    let door = |half| state("minecraft:iron_door", &[("half", half)]);
    let air = block_state_id("minecraft:air");
    for (game_type, drops) in [(GameType::Creative, 0), (GameType::Survival, 1)] {
        let mut dig = Dig::new(game_type, false);
        dig.set(lower, door("lower"));
        dig.set(upper, door("upper"));
        dig.hold(fast_tool(
            "minecraft:iron_pickaxe",
            &["minecraft:iron_door"],
            true,
        ));
        let tick = dig.dig(upper);
        assert_eq!(
            (dig.block(lower), dig.block(upper)),
            (Some(air), Some(air)),
            "{game_type:?}"
        );
        assert_eq!(
            dig.items(),
            vec![("minecraft:iron_door".to_owned(), 1); drops],
            "{game_type:?}"
        );
        let mut lower_events = level_events_at(&tick, lower);
        lower_events.sort_by_key(|(connection_id, _, _)| connection_id.0);
        if drops == 0 {
            // `DoublePlantBlock.java:129`: past the breaker, naming the lower
            // half's own state.
            assert_eq!(lower_events, vec![(OBSERVER, 2001, door("lower"))]);
        } else {
            // The lower half's own `Level.destroyBlock` reaches everyone.
            assert_eq!(
                lower_events,
                vec![
                    (MINER, 2001, door("lower")),
                    (OBSERVER, 2001, door("lower"))
                ]
            );
        }
    }
}

/// `DoublePlantBlock.playerWillDestroy` (`DoublePlantBlock.java:98-104`) and
/// `preventDropFromBottomPart` (`:121-132`): a creative break of the upper half
/// writes the lower half's water back and drops nothing, and a survival break
/// of the lower half drops once, from `playerWillDestroy`, while
/// `playerDestroy` (`:110-119`) still counts the mined plant.
#[test]
fn double_plants_replace_the_lower_half_with_its_water_and_drop_once_in_survival() {
    let air = block_state_id("minecraft:air");
    let water = state("minecraft:water", &[]);

    let mut creative = Dig::new(GameType::Creative, false);
    let lower = BlockPos { x: 2, y: 64, z: 2 };
    let upper = BlockPos { x: 2, y: 65, z: 2 };
    let seagrass = |half| state("minecraft:tall_seagrass", &[("half", half)]);
    creative.set(lower, seagrass("lower"));
    creative.set(upper, seagrass("upper"));
    let tick = creative.dig(upper);
    assert_eq!(creative.block(lower), Some(water));
    assert_eq!(creative.block(upper), Some(water));
    assert_eq!(
        level_events_at(&tick, lower),
        vec![(OBSERVER, 2001, seagrass("lower"))]
    );
    assert!(block_updates_at(&tick, lower).contains(&(OBSERVER, water)));
    assert!(creative.items().is_empty());

    let flower_lower = BlockPos { x: 5, y: 64, z: 2 };
    let flower_upper = BlockPos { x: 5, y: 65, z: 2 };
    let sunflower = |half| state("minecraft:sunflower", &[("half", half)]);
    creative.set(
        BlockPos {
            y: 63,
            ..flower_lower
        },
        state("minecraft:dirt", &[]),
    );
    creative.set(flower_lower, sunflower("lower"));
    creative.set(flower_upper, sunflower("upper"));
    let tick = creative.dig(flower_upper);
    assert_eq!(
        (creative.block(flower_lower), creative.block(flower_upper)),
        (Some(air), Some(air))
    );
    assert_eq!(
        level_events_at(&tick, flower_lower),
        vec![(OBSERVER, 2001, sunflower("lower"))]
    );
    assert!(creative.items().is_empty(), "{:?}", creative.items());

    let mut survival = Dig::new(GameType::Survival, false);
    survival.set(
        BlockPos {
            y: 63,
            ..flower_lower
        },
        state("minecraft:dirt", &[]),
    );
    survival.set(flower_lower, sunflower("lower"));
    survival.set(flower_upper, sunflower("upper"));
    survival.dig(flower_lower);
    assert_eq!(
        (survival.block(flower_lower), survival.block(flower_upper)),
        (Some(air), Some(air))
    );
    assert_eq!(
        survival.items(),
        vec![("minecraft:sunflower".to_owned(), 1)]
    );
    let sunflower_block = vanilla_block_type_for_resource_id("minecraft:sunflower")
        .expect("sunflower")
        .id;
    assert_eq!(
        survival.world.player_stat_for_test(
            MINER,
            bbb_server_types::stats::StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                sunflower_block,
            ),
        ),
        1
    );
}

/// `BedBlock.playerWillDestroy` (`BedBlock.java:180-190`) and
/// `PistonHeadBlock.playerWillDestroy` (`PistonHeadBlock.java:71-76`): the
/// creative break takes the other half with it and nothing drops.
#[test]
fn creative_bed_and_piston_head_breaks_take_the_other_half_without_drops() {
    let air = block_state_id("minecraft:air");
    let mut dig = Dig::new(GameType::Creative, false);

    let foot = BlockPos { x: 2, y: 64, z: 2 };
    let head = BlockPos { x: 2, y: 64, z: 3 };
    let bed = |part| state("minecraft:red_bed", &[("facing", "south"), ("part", part)]);
    dig.set(foot, bed("foot"));
    dig.set(head, bed("head"));
    let tick = dig.dig(foot);
    assert_eq!((dig.block(foot), dig.block(head)), (Some(air), Some(air)));
    assert_eq!(
        level_events_at(&tick, head),
        vec![(OBSERVER, 2001, bed("head"))]
    );
    assert!(dig.items().is_empty(), "{:?}", dig.items());

    let base = BlockPos { x: 6, y: 64, z: 2 };
    let piston_head = BlockPos { x: 7, y: 64, z: 2 };
    dig.set(
        BlockPos { x: 5, ..base },
        state("minecraft:redstone_block", &[]),
    );
    dig.set(
        base,
        state(
            "minecraft:piston",
            &[("facing", "east"), ("extended", "true")],
        ),
    );
    dig.set(
        piston_head,
        state(
            "minecraft:piston_head",
            &[("facing", "east"), ("type", "normal")],
        ),
    );
    dig.dig(piston_head);
    assert_eq!(
        (dig.block(base), dig.block(piston_head)),
        (Some(air), Some(air))
    );
    assert!(dig.items().is_empty(), "{:?}", dig.items());
}

/// `ShulkerBoxBlock.playerWillDestroy` (`ShulkerBoxBlock.java:108-123`): a
/// creative break of a box with contents spawns the box item carrying them.
#[test]
fn a_creative_break_of_a_filled_shulker_box_drops_it_with_its_contents() {
    let mut dig = Dig::new(GameType::Creative, false);
    let filled = BlockPos { x: 2, y: 64, z: 2 };
    let empty = BlockPos { x: 5, y: 64, z: 2 };
    let shulker = state("minecraft:shulker_box", &[]);
    dig.set(filled, shulker);
    dig.set(empty, shulker);
    dig.world
        .level_block_entity_for_test(DIM, filled)
        .and_then(crate::block_entity::BlockEntity::shulker_box_mut)
        .expect("shulker box block entity")
        .items_mut()[3] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 7,
        ..ItemStackSummary::empty()
    });

    dig.dig(empty);
    assert!(dig.items().is_empty());
    dig.dig(filled);
    assert_eq!(dig.block(filled), Some(block_state_id("minecraft:air")));
    let dropped = dig.world.stored_items_for_test(DIM);
    assert_eq!(dropped.len(), 1, "{dropped:?}");
    let (id, position, resource_id, count, pickup_delay, _) = dropped[0].clone();
    // The default delay of 10, less the entity tick of the tick that spawned it.
    assert_eq!(
        (resource_id.as_str(), count, pickup_delay),
        ("minecraft:shulker_box", 1, 9)
    );
    // Spawned at the centre, then moved once by its initial throw.
    assert!(
        (position.x - 2.5).abs() <= 0.1 && (position.z - 2.5).abs() <= 0.1 && position.y >= 64.5,
        "{position:?}"
    );
    let stack = dig
        .world
        .stored_item_stack_for_test(DIM, id)
        .expect("dropped stack");
    let container = crate::tests::saved_item_component(&dig.world, &stack, "minecraft:container")
        .expect("container component");
    let bbb_protocol::NbtSummaryValue::List(slots) = container else {
        panic!("container contents are a slot list: {container:?}");
    };
    assert_eq!(slots.len(), 1, "{slots:?}");
    let rendered = format!("{slots:?}");
    assert!(
        rendered.contains("minecraft:stone")
            && rendered.contains("Int(7)")
            && rendered.contains("Int(3)"),
        "{rendered}"
    );
}

/// `TntBlock.playerWillDestroy` (`TntBlock.java:66-72`) primes unstable TNT for
/// a survival breaker only, and `BaseFireBlock.playerWillDestroy`
/// (`BaseFireBlock.java:176-185`) swaps the destroy particles for the
/// extinguish sound every session hears.
#[test]
fn survival_breaks_prime_unstable_tnt_and_fire_breaks_play_the_extinguish_event() {
    let unstable = state("minecraft:tnt", &[("unstable", "true")]);
    let primed = |dig: &Dig| {
        dig.world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .filter(|&id| {
                dig.world
                    .stored_entity_for_test(DIM, id)
                    .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_TNT_ID)
            })
            .count()
    };
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    let mut survival = Dig::new(GameType::Survival, false);
    survival.set(pos, unstable);
    survival.dig(pos);
    assert_eq!(survival.block(pos), Some(block_state_id("minecraft:air")));
    assert_eq!(primed(&survival), 1);
    assert!(survival.items().is_empty());

    let mut creative = Dig::new(GameType::Creative, false);
    creative.set(pos, unstable);
    creative.dig(pos);
    assert_eq!(primed(&creative), 0);

    let fire_pos = BlockPos { x: 5, y: 64, z: 2 };
    survival.set(fire_pos, state("minecraft:fire", &[]));
    let tick = survival.dig(fire_pos);
    assert_eq!(
        survival.block(fire_pos),
        Some(block_state_id("minecraft:air"))
    );
    let mut heard = level_events_at(&tick, fire_pos);
    heard.sort_by_key(|(connection_id, _, _)| connection_id.0);
    assert_eq!(heard, vec![(MINER, 1009, 0), (OBSERVER, 1009, 0)]);
}

/// `TripWireBlock.playerWillDestroy` (`TripWireBlock.java:115-122`): shears
/// disarm the wire and raise `GameEvent.SHEAR` from the breaker before the base
/// break event; a bare hand does neither.
#[test]
fn shears_disarm_a_tripwire_with_a_shear_game_event_from_the_breaker() {
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    let wire = state("minecraft:tripwire", &[]);
    let mut dig = Dig::new(GameType::Survival, false);
    let miner_id = entity_id_of(&dig.world, 1);
    dig.set(pos, wire);
    dig.hold(item("minecraft:shears"));
    dig.dig(pos);
    assert_eq!(dig.block(pos), Some(block_state_id("minecraft:air")));
    let events: Vec<_> = dig
        .game_events_at(pos)
        .into_iter()
        .map(|event| {
            (
                event.game_event,
                event.source_entity_id,
                event.affected_state,
            )
        })
        .collect();
    let shear = events
        .iter()
        .position(|event| *event == (VanillaGameEvent::SHEAR, Some(miner_id), None))
        .expect("SHEAR from the breaker");
    let destroyed = events
        .iter()
        .position(|event| *event == (VanillaGameEvent::BLOCK_DESTROY, Some(miner_id), Some(wire)))
        .expect("BLOCK_DESTROY with the undisarmed wire");
    assert!(shear < destroyed, "{events:?}");

    let bare = BlockPos { x: 5, y: 64, z: 2 };
    dig.set(bare, wire);
    dig.hold(ItemStackSummary::empty());
    dig.dig(bare);
    assert!(!dig
        .game_events_at(bare)
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::SHEAR));
}

/// `DecoratedPotBlock.playerWillDestroy` (`DecoratedPotBlock.java:196-204`):
/// an item in `#breaks_decorated_pots` without a shattering-preventing
/// enchantment cracks the pot, and the base break names the cracked state.
#[test]
fn a_pickaxe_cracks_a_decorated_pot_unless_its_enchantment_prevents_shattering() {
    let intact = state("minecraft:decorated_pot", &[]);
    let cracked = with_property(intact, "cracked", "true").expect("cracked pot");
    let mut dig = Dig::new(GameType::Survival, false);
    let pickaxe = item("minecraft:diamond_pickaxe");
    let silk_pickaxe = dig.enchanted(pickaxe.clone(), "minecraft:silk_touch");
    for (x, held, broken_state) in [
        (2, pickaxe, cracked),
        (5, silk_pickaxe, intact),
        (8, ItemStackSummary::empty(), intact),
    ] {
        let pos = BlockPos { x, y: 64, z: 2 };
        dig.set(pos, intact);
        dig.hold(held);
        let tick = dig.dig(pos);
        assert_eq!(dig.block(pos), Some(block_state_id("minecraft:air")));
        assert_eq!(
            level_events_at(&tick, pos),
            vec![(OBSERVER, 2001, broken_state)],
            "x={x}"
        );
        assert!(dig.game_events_at(pos).iter().any(|event| {
            event.game_event == VanillaGameEvent::BLOCK_DESTROY
                && event.affected_state == Some(broken_state)
        }));
    }
}

/// `BlockBehaviour.attack` (`ServerPlayerGameMode.java:197`) on the swing that
/// starts a survival break: `NoteBlock.attack` (`NoteBlock.java:136-141`),
/// `RedStoneOreBlock.attack` (`RedStoneOreBlock.java:39-42`) and
/// `DragonEggBlock.attack` (`DragonEggBlock.java:47-49`).
#[test]
fn starting_a_survival_break_attacks_note_blocks_redstone_ore_and_dragon_eggs() {
    let mut dig = Dig::new(GameType::Survival, false);
    let miner_id = entity_id_of(&dig.world, 1);

    let note = BlockPos { x: 2, y: 64, z: 2 };
    let note_state = state("minecraft:note_block", &[]);
    dig.set(note, note_state);
    let tick = dig.dig(note);
    assert_eq!(dig.block(note), Some(note_state));
    assert!(tick.events.iter().any(|event| matches!(
        event.packet_for(OBSERVER),
        Some(ClientboundPlayPacket::BlockEvent(block_event)) if block_event.pos == note
    )));
    assert!(dig.game_events_at(note).iter().any(|event| {
        event.game_event == VanillaGameEvent::NOTE_BLOCK_PLAY
            && event.source_entity_id == Some(miner_id)
    }));
    assert_eq!(
        dig.world.player_stat_for_test(
            MINER,
            bbb_server_types::stats::custom_stat("minecraft:play_noteblock")
        ),
        1
    );

    let ore = BlockPos { x: 5, y: 64, z: 2 };
    dig.set(ore, state("minecraft:redstone_ore", &[]));
    dig.dig(ore);
    assert_eq!(
        dig.block(ore),
        Some(state("minecraft:redstone_ore", &[("lit", "true")]))
    );

    let egg = BlockPos { x: 8, y: 64, z: 2 };
    let egg_state = state("minecraft:dragon_egg", &[]);
    dig.set(egg, egg_state);
    dig.dig(egg);
    assert_eq!(dig.block(egg), Some(block_state_id("minecraft:air")));

    // A creative swing destroys before the attack is reached (`:172-175`).
    let mut creative = Dig::new(GameType::Creative, false);
    creative.set(note, note_state);
    creative.dig(note);
    assert_eq!(
        creative.world.player_stat_for_test(
            MINER,
            bbb_server_types::stats::custom_stat("minecraft:play_noteblock")
        ),
        0
    );
}

/// `IceBlock.playerDestroy` (`IceBlock.java:36-56`) melts the broken ice over a
/// solid floor unless the tool's enchantment prevents it, and
/// `TurtleEggBlock.playerDestroy` (`TurtleEggBlock.java:142-152`) writes one egg
/// fewer back.
#[test]
fn player_destroy_melts_ice_and_takes_one_turtle_egg() {
    let mut dig = Dig::new(GameType::Survival, false);
    let ice = BlockPos { x: 2, y: 64, z: 2 };
    dig.set(ice, state("minecraft:ice", &[]));
    dig.hold(fast_tool(
        "minecraft:iron_pickaxe",
        &["minecraft:ice"],
        true,
    ));
    let tick = dig.dig(ice);
    let water = state("minecraft:water", &[]);
    assert_eq!(dig.block(ice), Some(water));
    assert!(block_updates_at(&tick, ice).contains(&(OBSERVER, water)));
    assert!(dig.items().is_empty());

    let silk_ice = BlockPos { x: 5, y: 64, z: 2 };
    dig.set(silk_ice, state("minecraft:ice", &[]));
    let silk = dig.enchanted(
        fast_tool("minecraft:iron_pickaxe", &["minecraft:ice"], true),
        "minecraft:silk_touch",
    );
    dig.hold(silk);
    dig.dig(silk_ice);
    assert_eq!(dig.block(silk_ice), Some(block_state_id("minecraft:air")));
    assert_eq!(dig.items(), vec![("minecraft:ice".to_owned(), 1)]);

    let eggs = BlockPos { x: 8, y: 64, z: 2 };
    let three = state("minecraft:turtle_egg", &[("eggs", "3")]);
    let two = state("minecraft:turtle_egg", &[("eggs", "2")]);
    dig.set(eggs, three);
    dig.hold(fast_tool(
        "minecraft:iron_pickaxe",
        &["minecraft:turtle_egg"],
        true,
    ));
    let miner_id = entity_id_of(&dig.world, 1);
    let tick = dig.dig(eggs);
    assert_eq!(dig.block(eggs), Some(two));
    let sound_id = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.turtle.egg_break",
    )
    .expect("egg break sound");
    assert!(sounds_sent(&tick).iter().any(|sound| matches!(
        sound.sound,
        bbb_protocol::packets::SoundEventHolder::Reference { registry_id } if registry_id == sound_id
    )));
    // The base particles skip the breaker; `decreaseEggs`' own reach everyone.
    let heard = level_events_at(&tick, eggs);
    let heard_by = |connection_id| {
        heard
            .iter()
            .filter(|(to, _, _)| *to == connection_id)
            .map(|(_, event_type, data)| (*event_type, *data))
            .collect::<Vec<_>>()
    };
    assert_eq!(heard_by(OBSERVER), vec![(2001, three), (2001, three)]);
    assert_eq!(heard_by(MINER), vec![(2001, three)]);
    let destroys: Vec<_> = dig
        .game_events_at(eggs)
        .into_iter()
        .filter(|event| event.game_event == VanillaGameEvent::BLOCK_DESTROY)
        .map(|event| (event.source_entity_id, event.affected_state))
        .collect();
    assert_eq!(
        destroys,
        vec![(Some(miner_id), Some(three)), (None, Some(three))]
    );

    let single = BlockPos { x: 11, y: 64, z: 2 };
    dig.set(single, state("minecraft:turtle_egg", &[("eggs", "1")]));
    dig.dig(single);
    assert_eq!(dig.block(single), Some(block_state_id("minecraft:air")));
}

/// `SculkBlock` is a `DropExperienceBlock` of `ConstantInt.of(1)`
/// (`SculkBlock.java:23-25`), paid by `spawnAfterBreak` after a player break.
#[test]
fn breaking_sculk_pays_one_experience_point() {
    let mut dig = Dig::new(GameType::Survival, false);
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    dig.set(pos, state("minecraft:sculk", &[]));
    dig.hold(fast_tool("minecraft:iron_hoe", &["minecraft:sculk"], true));
    dig.dig(pos);
    assert_eq!(dig.block(pos), Some(block_state_id("minecraft:air")));
    let orbs = dig.world.stored_experience_orbs_for_test(DIM);
    assert_eq!(
        orbs.iter()
            .map(|(_, _, value, count)| value * count)
            .sum::<i32>(),
        1,
        "{orbs:?}"
    );
}

/// `Block.playerWillDestroy` asks `BlockTags.GUARDED_BY_PIGLINS`
/// (`Block.java:520`) of the state the subclass handed it.
#[test]
fn the_piglin_anger_hook_reads_the_guarded_by_piglins_tag() {
    let dig = Dig::new(GameType::Survival, false);
    for (name, guarded) in [
        ("minecraft:gold_block", true),
        ("minecraft:chest", true),
        ("minecraft:shulker_box", true),
        ("minecraft:stone", false),
    ] {
        assert_eq!(
            dig.world.player_break_angers_piglins(DIM, state(name, &[])),
            guarded,
            "{name}"
        );
    }
}

#[test]
fn breaking_a_loaded_guarded_tag_member_provokes_a_piglin_without_sight() {
    use crate::entity::ai::{MemoryModuleType, MemoryValue};
    use crate::entity::{EntityData, GeneralLivingData};
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID;
    use bbb_protocol::equipment_slots::EquipmentSlot;

    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    Arc::make_mut(config.datapacks.as_mut().unwrap())
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:block")
        .unwrap()
        .tags
        .insert(
            "minecraft:guarded_by_piglins".into(),
            vec!["minecraft:torch".into()],
        );
    let mut dig = Dig::with_config(config, true);
    dig.command("fill -2 63 -5 9 63 2 minecraft:stone");
    dig.command("fill -2 64 -5 9 68 2 minecraft:air");
    dig.command("fill 3 64 -5 3 68 2 minecraft:stone");
    dig.command("item replace entity @a armor.head with minecraft:golden_helmet");
    let at = Vec3d {
        x: 6.5,
        y: 64.0,
        z: -2.5,
    };
    let (id, _) = dig.world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        at,
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_PIGLIN_ID).unwrap()),
    );
    let NbtSummaryValue::Compound(mut record) = dig
        .world
        .stored_entity_command_data_for_test(DIM, id)
        .unwrap()
    else {
        unreachable!()
    };
    record.extend([
        nbt("IsImmuneToZombification", NbtSummaryValue::Byte(1)),
        nbt("PersistenceRequired", NbtSummaryValue::Byte(1)),
    ]);
    dig.world
        .load_stored_entity_record_for_test(DIM, id, &record);
    let body = dig.world.stored_entity_mut_for_test(DIM, id).unwrap();
    body.set_pos(at);
    body.living_mut().unwrap().set_item_slot(
        EquipmentSlot::MainHand,
        Some(crate::entity::item::plain_stack("minecraft:golden_sword", 1).unwrap()),
    );
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    dig.set(pos, state("minecraft:torch", &[]));
    assert!(!dig
        .world
        .player_break_angers_piglins(DIM, state("minecraft:gold_block", &[])));
    dig.dig(pos);
    assert_eq!(
        dig.world.block_state(DIM, pos),
        Some(state("minecraft:air", &[]))
    );
    assert!(
        crate::entity::brain_packed(dig.world.stored_entity_for_test(DIM, id).unwrap())
            .unwrap()
            .memories
            .iter()
            .any(|memory| memory.memory_type == MemoryModuleType::AngryAt
                && memory.value == MemoryValue::Uuid(Uuid::from_u128(1)))
    );
    dig.command("fill 3 64 -5 3 68 2 minecraft:air");
    for _ in 0..100 {
        dig.world.tick(dig.receiver.drain());
        if dig.world.player_health_for_test(MINER) < 20.0 {
            break;
        }
    }
    assert!(dig.world.player_health_for_test(MINER) < 20.0);
}

fn with_adventure_predicate(
    mut stack: ItemStackSummary,
    can_break: bool,
    blocks: bbb_protocol::packets::HolderSetSummary,
) -> ItemStackSummary {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
    };
    let predicate = Some(bbb_protocol::packets::AdventureModePredicateDataSummary {
        predicates: vec![bbb_protocol::packets::BlockPredicateSummary {
            blocks: Some(blocks),
            ..Default::default()
        }],
    });
    let patch = &mut stack.component_patch;
    if can_break {
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID);
        patch.can_break_persistent = predicate;
    } else {
        patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID);
        patch.can_place_on_persistent = predicate;
    }
    stack
}

/// `Player.blockActionRestricted` (Player.java:188-203) and `ItemStack.useOn`
/// (ItemStack.java:358-363) let an adventure player break and place only where
/// the held stack's `can_break` / `can_place_on` admits the block.
#[test]
fn an_adventure_player_breaks_and_places_only_where_the_held_stack_allows() {
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    let dandelion = state("minecraft:dandelion", &[]);
    let by_tag = bbb_protocol::packets::HolderSetSummary {
        registry_ids: Vec::new(),
        tag: Some("minecraft:small_flowers".into()),
    };
    let poppy_only = bbb_protocol::packets::HolderSetSummary {
        registry_ids: vec![
            vanilla_block_type_for_resource_id("minecraft:poppy")
                .unwrap()
                .id,
        ],
        tag: None,
    };
    for (blocks, breaks) in [(by_tag, true), (poppy_only, false)] {
        let mut dig = Dig::new(GameType::Adventure, false);
        dig.hold(with_adventure_predicate(
            item("minecraft:stick"),
            true,
            blocks,
        ));
        dig.set(pos, dandelion);
        dig.dig(pos);
        let expected = if breaks {
            block_state_id("minecraft:air")
        } else {
            dandelion
        };
        assert_eq!(dig.block(pos), Some(expected), "breaks: {breaks}");
    }

    let support = BlockPos { x: 2, y: 64, z: 2 };
    let above = BlockPos { x: 2, y: 65, z: 2 };
    let stone = state("minecraft:stone", &[]);
    for (support_state, places) in [(stone, true), (state("minecraft:dirt", &[]), false)] {
        let mut dig = Dig::new(GameType::Adventure, false);
        let stone_id = vanilla_block_type_for_resource_id("minecraft:stone")
            .unwrap()
            .id;
        let mut held = with_adventure_predicate(
            item("minecraft:stone"),
            false,
            bbb_protocol::packets::HolderSetSummary {
                registry_ids: vec![stone_id],
                tag: None,
            },
        );
        held.count = 4;
        dig.hold(held);
        dig.set(support, support_state);
        dig.world.set_player_position_for_test(
            MINER,
            Vec3d {
                x: 2.5,
                y: 64.0,
                z: 0.5,
            },
        );
        dig.place(support);
        let expected = if places {
            stone
        } else {
            block_state_id("minecraft:air")
        };
        assert_eq!(dig.block(above), Some(expected), "places: {places}");
    }
}
