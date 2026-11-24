//! `GameEvent.Context.sourceEntity` for real session clicks and digs, read off
//! the game events the level raised.

use super::*;

use crate::block_entity::game_event::GameEventContext;
use crate::block_entity::vibration::VibrationData;
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, PendingExplosion,
};
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const SENSOR: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const TARGET: BlockPos = BlockPos { x: 3, y: 64, z: 0 };

/// A session within reach of [`TARGET`] beside an idle sensor, and its entity id.
fn listener_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 2.5,
        },
    );
    let sensor = placement_state(
        "minecraft:sculk_sensor",
        &[
            ("sculk_sensor_phase", "inactive"),
            ("power", "0"),
            ("waterlogged", "false"),
        ],
    );
    assert!(world.set_block(DIMENSION, SENSOR, sensor, SET_BLOCK_UPDATE_ALL));
    world.tick(receiver.drain());
    *world
        .level_block_entity_for_test(DIMENSION, SENSOR)
        .and_then(|entity| entity.sculk_sensor_mut())
        .expect("the sensor's block entity")
        .vibration_data_mut_for_test() = VibrationData::default();
    let player = world.player_entity_id_for_test(ConnectionId(1));
    (sender, receiver, world, player)
}

/// The context of the latest `event` raised at `pos`.
fn raised(world: &ServerWorld, event: VanillaGameEvent, pos: BlockPos) -> GameEventContext {
    world
        .game_events(DIMENSION)
        .0
        .into_iter()
        .rev()
        .find(|raised| raised.game_event == event && raised.pos == pos)
        .map(|raised| GameEventContext {
            source_entity_id: raised.source_entity_id,
            affected_state: raised.affected_state,
        })
        .unwrap_or_else(|| panic!("no {event:?} was raised at {pos:?}"))
}

const fn from_player(player: i32, affected_state: Option<i32>) -> GameEventContext {
    GameEventContext {
        source_entity_id: Some(player),
        affected_state,
    }
}

const fn sourceless(affected_state: Option<i32>) -> GameEventContext {
    GameEventContext {
        source_entity_id: None,
        affected_state,
    }
}

fn stone_button() -> i32 {
    placement_state(
        "minecraft:stone_button",
        &[("face", "floor"), ("facing", "north"), ("powered", "false")],
    )
}

fn set(world: &mut ServerWorld, pos: BlockPos, state: i32) {
    assert!(world.set_block(DIMENSION, pos, state, SET_BLOCK_UPDATE_ALL));
}

/// `ButtonBlock.press` (`ButtonBlock.java:108-114`) carries the clicking
/// player, which the sensor records as the vibration source; the scheduled
/// release (`ButtonBlock.java:147-178`) raises its event with no source.
#[test]
fn a_button_click_carries_the_player_and_its_scheduled_release_does_not() {
    let (sender, receiver, mut world, player) = listener_world();
    set(&mut world, TARGET, stone_button());
    right_click(&sender, &receiver, &mut world, 1, TARGET, 1);

    assert_eq!(
        raised(&world, VanillaGameEvent::BLOCK_ACTIVATE, TARGET),
        from_player(player, None)
    );
    let vibration = world
        .level_block_entity_for_test(DIMENSION, SENSOR)
        .and_then(|entity| entity.sculk_sensor())
        .expect("the sensor's block entity")
        .vibration_data();
    let candidate = vibration
        .current_vibration()
        .copied()
        .or_else(|| {
            vibration
                .selection_strategy()
                .current()
                .map(|(info, _)| info)
        })
        .expect("the click scheduled a vibration");
    assert_eq!(candidate.game_event, "minecraft:block_activate");
    assert_eq!(candidate.source, Some(Uuid::from_u128(1)));

    for _ in 0..21 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        raised(&world, VanillaGameEvent::BLOCK_DEACTIVATE, TARGET),
        sourceless(None)
    );
}

#[test]
fn an_arrow_holds_a_wooden_button_until_it_is_removed_but_does_not_press_stone() {
    for (button, responds) in [
        ("minecraft:oak_button", true),
        ("minecraft:stone_button", false),
    ] {
        let (_sender, receiver, mut world, _) = listener_world();
        world.set_block(
            DIMENSION,
            BlockPos {
                y: TARGET.y - 1,
                ..TARGET
            },
            placement_state("minecraft:stone", &[]),
            SET_BLOCK_UPDATE_ALL,
        );
        set(
            &mut world,
            TARGET,
            placement_state(
                button,
                &[("face", "floor"), ("facing", "north"), ("powered", "false")],
            ),
        );
        let arrow = world
            .spawn_stored_entity_for_test(
                DIMENSION,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
                Vec3d {
                    x: f64::from(TARGET.x) + 0.5,
                    y: f64::from(TARGET.y) + 2.0,
                    z: f64::from(TARGET.z) + 0.5,
                },
                crate::entity::EntityData::Arrow(crate::entity::ArrowData::new(
                    crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
                )),
            )
            .0;
        world
            .stored_entity_mut_for_test(DIMENSION, arrow)
            .unwrap()
            .set_delta_movement(Vec3d {
                x: 0.0,
                y: -1.5,
                z: 0.0,
            });
        for _ in 0..5 {
            world.tick(receiver.drain());
        }
        let pressed = world.block_state(DIMENSION, TARGET).unwrap();
        assert_eq!(
            crate::block_behavior::footing::face_attached_signal(pressed),
            if responds { 15 } else { 0 }
        );
        if !responds {
            continue;
        }
        assert_eq!(
            raised(&world, VanillaGameEvent::BLOCK_ACTIVATE, TARGET),
            from_player(arrow, None)
        );
        for _ in 0..65 {
            world.tick(receiver.drain());
        }
        assert_eq!(
            crate::block_behavior::footing::face_attached_signal(
                world.block_state(DIMENSION, TARGET).unwrap()
            ),
            15
        );
        world
            .stored_entity_mut_for_test(DIMENSION, arrow)
            .unwrap()
            .discard();
        for _ in 0..31 {
            world.tick(receiver.drain());
        }
        assert_eq!(
            crate::block_behavior::footing::face_attached_signal(
                world.block_state(DIMENSION, TARGET).unwrap()
            ),
            0
        );
        assert_eq!(
            raised(&world, VanillaGameEvent::BLOCK_DEACTIVATE, TARGET),
            sourceless(None)
        );
    }
}

/// Every `block_behavior` use owner whose override hands the player to its game
/// event, plus the lever, whose server arm pulls with a null player
/// (`LeverBlock.java:66-72`).
#[test]
fn shared_use_bodies_carry_the_clicking_player() {
    let lower_door = placement_state(
        "minecraft:oak_door",
        &[
            ("facing", "north"),
            ("half", "lower"),
            ("hinge", "left"),
            ("open", "false"),
            ("powered", "false"),
        ],
    );
    let upper_door = with_property(lower_door, "half", "upper").unwrap();
    let detector = placement_state(
        "minecraft:daylight_detector",
        &[("inverted", "false"), ("power", "0")],
    );
    let inverted = with_property(detector, "inverted", "true").unwrap();
    type InteractionCase<'a> = (
        &'a str,
        Vec<(BlockPos, i32)>,
        VanillaGameEvent,
        bool,
        Option<i32>,
    );
    let cases: [InteractionCase<'_>; 6] = [
        (
            "door",
            vec![
                (TARGET, lower_door),
                (BlockPos { y: 65, ..TARGET }, upper_door),
            ],
            VanillaGameEvent::BLOCK_OPEN,
            true,
            None,
        ),
        (
            "trapdoor",
            vec![(
                TARGET,
                placement_state(
                    "minecraft:oak_trapdoor",
                    &[
                        ("facing", "north"),
                        ("half", "bottom"),
                        ("open", "false"),
                        ("powered", "false"),
                        ("waterlogged", "false"),
                    ],
                ),
            )],
            VanillaGameEvent::BLOCK_OPEN,
            true,
            None,
        ),
        (
            "fence gate",
            vec![(
                TARGET,
                placement_state(
                    "minecraft:oak_fence_gate",
                    &[
                        ("facing", "north"),
                        ("in_wall", "false"),
                        ("open", "false"),
                        ("powered", "false"),
                    ],
                ),
            )],
            VanillaGameEvent::BLOCK_OPEN,
            true,
            None,
        ),
        (
            "daylight detector",
            vec![(TARGET, detector)],
            VanillaGameEvent::BLOCK_CHANGE,
            true,
            Some(inverted),
        ),
        (
            "note block",
            vec![(TARGET, note_block_state("harp", 0))],
            VanillaGameEvent::NOTE_BLOCK_PLAY,
            true,
            None,
        ),
        (
            "lever",
            vec![(
                TARGET,
                placement_state(
                    "minecraft:lever",
                    &[("face", "floor"), ("facing", "north"), ("powered", "false")],
                ),
            )],
            VanillaGameEvent::BLOCK_ACTIVATE,
            false,
            None,
        ),
    ];
    for (name, cells, event, carries_player, affected_state) in cases {
        let (sender, receiver, mut world, player) = listener_world();
        for (pos, state) in cells {
            set(&mut world, pos, state);
        }
        right_click(&sender, &receiver, &mut world, 1, TARGET, 1);
        let expected = if carries_player {
            from_player(player, affected_state)
        } else {
            sourceless(affected_state)
        };
        assert_eq!(raised(&world, event, TARGET), expected, "{name}");
    }
}

/// World-owned use legs: `CakeBlock.eat` (`CakeBlock.java:102`),
/// `CopperGolemStatueBlock.updatePose` (`CopperGolemStatueBlock.java:122`),
/// `DecoratedPotBlock.useItemOn` (`DecoratedPotBlock.java:131`),
/// `BeehiveBlock.useItemOn` (`BeehiveBlock.java:168`) and the `BlockItem.place`
/// the fixture's held stone falls through to (`BlockItem.java:88`).
#[test]
fn world_owned_use_legs_carry_the_clicking_player() {
    let (sender, receiver, mut world, player) = listener_world();
    set(&mut world, TARGET, cake_state(0));
    right_click(&sender, &receiver, &mut world, 1, TARGET, 1);
    assert_eq!(
        raised(&world, VanillaGameEvent::EAT, TARGET),
        from_player(player, None)
    );

    let (sender, receiver, mut world, player) = listener_world();
    set(
        &mut world,
        TARGET,
        copper_golem_statue_state("minecraft:waxed_copper_golem_statue", "standing"),
    );
    right_click(&sender, &receiver, &mut world, 1, TARGET, 1);
    assert_eq!(
        raised(&world, VanillaGameEvent::BLOCK_CHANGE, TARGET),
        from_player(player, None)
    );

    let (sender, receiver, mut world, player) = listener_world();
    set(
        &mut world,
        TARGET,
        placement_state(
            "minecraft:decorated_pot",
            &[
                ("cracked", "false"),
                ("facing", "north"),
                ("waterlogged", "false"),
            ],
        ),
    );
    right_click(&sender, &receiver, &mut world, 1, TARGET, 1);
    assert_eq!(
        raised(&world, VanillaGameEvent::BLOCK_CHANGE, TARGET),
        from_player(player, None)
    );

    let (sender, receiver, mut world, player) = listener_world();
    set(&mut world, TARGET, bee_nest_state(5));
    give_named(&sender, &receiver, &mut world, "minecraft:shears");
    right_click(&sender, &receiver, &mut world, 1, TARGET, 1);
    assert_eq!(
        raised(&world, VanillaGameEvent::SHEAR, TARGET),
        from_player(player, None)
    );

    let (sender, receiver, mut world, player) = listener_world();
    let floor = BlockPos { y: 63, ..TARGET };
    give_named(&sender, &receiver, &mut world, "minecraft:stone");
    right_click(&sender, &receiver, &mut world, 1, floor, 1);
    let stone = world.fixture().stone_state_id();
    assert_eq!(world.block_state(DIMENSION, TARGET), Some(stone));
    assert_eq!(
        raised(&world, VanillaGameEvent::BLOCK_PLACE, TARGET),
        from_player(player, Some(stone))
    );
}

/// `Block.playerWillDestroy` (`Block.java:524`): the breaking player and the
/// state it broke.
#[test]
fn a_player_dig_carries_the_player_and_the_broken_state() {
    let (sender, receiver, mut world, player) = listener_world();
    let broken = BlockPos { y: 63, ..TARGET };
    let stone = world.fixture().stone_state_id();
    assert_eq!(world.block_state(DIMENSION, broken), Some(stone));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: broken,
            direction: Direction::Up,
            sequence: 1,
        }),
    );
    assert_ne!(world.block_state(DIMENSION, broken), Some(stone));
    assert_eq!(
        raised(&world, VanillaGameEvent::BLOCK_DESTROY, broken),
        from_player(player, Some(stone))
    );
}

/// A TRIGGER explosion presses with a null player (`ButtonBlock.java:98-103`),
/// and a redstone edge opens a door with none (`DoorBlock.java:225-238`).
#[test]
fn explosion_triggers_and_redstone_edges_carry_no_source() {
    let (_sender, receiver, mut world, _player) = listener_world();
    set(&mut world, TARGET, stone_button());
    world.explode(
        DIMENSION,
        PendingExplosion {
            source: None,
            center: Vec3d {
                x: 3.5,
                y: 64.5,
                z: 0.5,
            },
            radius: 1.0,
            fire: false,
            interaction: ExplosionInteraction::Trigger,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut Vec::new(),
    );
    world.tick(receiver.drain());
    assert_eq!(
        raised(&world, VanillaGameEvent::BLOCK_ACTIVATE, TARGET),
        sourceless(None)
    );

    let (_sender, receiver, mut world, _player) = listener_world();
    let lower_door = placement_state(
        "minecraft:oak_door",
        &[
            ("facing", "north"),
            ("half", "lower"),
            ("hinge", "left"),
            ("open", "false"),
            ("powered", "false"),
        ],
    );
    set(&mut world, TARGET, lower_door);
    set(
        &mut world,
        BlockPos { y: 65, ..TARGET },
        with_property(lower_door, "half", "upper").unwrap(),
    );
    set(
        &mut world,
        BlockPos { x: 4, ..TARGET },
        block_state_id("minecraft:redstone_block"),
    );
    world.tick(receiver.drain());
    assert_eq!(
        raised(&world, VanillaGameEvent::BLOCK_OPEN, TARGET),
        sourceless(None)
    );
}
