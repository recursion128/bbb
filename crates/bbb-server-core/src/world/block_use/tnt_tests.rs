use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::world::{ServerWorld, WorldCommand, WorldEvent};
use bbb_protocol::packets::{
    BlockHitResult, BlockPos, Direction, InteractionHand, ItemStackSummary, PlayServerbound,
    UseItemOn, Vec3d,
};
use bbb_server_types::save::Java26BuiltinDimension;

const PLAYER: ConnectionId = ConnectionId(1);

/// TntBlock.java:123-126: with `tnt_explodes` off, lighting TNT only shows
/// the clicking player an action-bar message.
#[test]
fn lighting_tnt_with_the_rule_off_tells_the_player_and_keeps_the_block() {
    let (sender, receiver) = crate::tests::mailbox();
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        game_type: bbb_protocol::game_types::GameType::Survival,
        ..crate::tests::datapack_config()
    })
    .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        PLAYER,
        Vec3d {
            x: 0.5,
            y: 101.0,
            z: 0.5,
        },
    );
    let tnt = BlockPos { x: 2, y: 101, z: 0 };
    let tnt_state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_TNT_ID,
    )
    .unwrap();
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        tnt,
        tnt_state,
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    use crate::admin::{AdminEnvelope, AdminRequest, AdminScope};
    sender
        .try_send(WorldCommand::Admin {
            envelope: AdminEnvelope {
                operation_id: 2,
                request_id: uuid::Uuid::new_v4().to_string(),
                principal: "fixture".into(),
                scope: AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 0,
            },
            request: AdminRequest::ExecuteCommand {
                command: "gamerule tnt_explodes false".into(),
            },
        })
        .unwrap();
    assert!(world
        .tick(receiver.drain())
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::Admin(outcome) if outcome.result.is_ok())));
    world.stock_menu_slot_for_test(
        PLAYER,
        36,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_FLINT_AND_STEEL_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    sender
        .try_send(WorldCommand::Play {
            connection_id: PLAYER,
            packet: PlayServerbound::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos: tnt,
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 0.5,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 1,
            }),
        })
        .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SystemChat(chat),
            ..
        } if chat.overlay && format!("{:?}", chat.content).contains("block.minecraft.tnt.disabled")
    )));
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, tnt),
        Some(tnt_state)
    );
}
