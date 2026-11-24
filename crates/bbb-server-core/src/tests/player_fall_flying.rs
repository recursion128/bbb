use super::*;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_FALL_FLYING_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;

/// `Inventory.EQUIPMENT_SLOT_MAPPING`'s `CHEST` slot.
const CHEST_SLOT: usize = 38;

struct Glider {
    world: ServerWorld,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
}

impl Glider {
    fn packet(&mut self, connection_id: u64, packet: PlayServerbound) -> TickResult {
        play(
            &self.sender,
            &self.receiver,
            &mut self.world,
            connection_id,
            packet,
        )
    }

    fn command(&mut self, command: &str) {
        chat_command(&self.sender, &self.receiver, &mut self.world, 1, command);
    }

    fn move_to(&mut self, y: f64, on_ground: bool) -> TickResult {
        let at = self.position();
        self.packet(
            1,
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: at.x,
                    y,
                    z: at.z,
                }),
                rotation: None,
                on_ground,
                horizontal_collision: false,
            }),
        )
    }

    fn position(&self) -> Vec3d {
        self.world
            .publication()
            .players
            .iter()
            .find(|player| player.connection_id == ConnectionId(1))
            .unwrap()
            .position
    }

    fn start_fall_flying(&mut self) -> TickResult {
        let entity_id = entity_id_of(&self.world, 1);
        self.packet(
            1,
            PlayServerbound::PlayerCommand(PlayerCommand {
                entity_id,
                action: PlayerCommandAction::StartFallFlying,
                data: 0,
            }),
        )
    }

    fn ticks(&mut self, count: usize) {
        for _ in 0..count {
            self.world.tick(self.receiver.drain());
        }
    }

    fn fall_flying(&mut self) -> bool {
        self.world
            .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
            .unwrap()
            .player_patches
            .iter()
            .find(|patch| patch.uuid == Uuid::from_u128(1))
            .unwrap()
            .created
            .fall_flying
    }

    fn elytra_damage(&self) -> Option<i32> {
        self.world
            .player_inventory_slot_for_test(ConnectionId(1), CHEST_SLOT)
            .component_patch
            .damage
    }

    fn glide_events(&self) -> usize {
        let entity_id = entity_id_of(&self.world, 1);
        self.world
            .positioned_game_events_for_test(Java26BuiltinDimension::Overworld)
            .iter()
            .filter(|event| {
                event.recorded.game_event == VanillaGameEvent::ELYTRA_GLIDE
                    && event.recorded.source_entity_id == Some(entity_id)
            })
            .count()
    }
}

/// Fall flying held by a fixture rather than started: `updateFallFlying`
/// clears the flag on the next tick unless the session wears a glider and is
/// off the ground, so this provides both.
pub(super) fn wear_an_elytra_in_the_air(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
) -> Vec3d {
    world.set_inventory_slot_for_test(
        ConnectionId(connection_id),
        CHEST_SLOT,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_ELYTRA_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    // Three blocks up, where the tick's travel finds no ground under the
    // glider (`LivingEntity.canGlide` reads `onGround`).
    let start = world
        .publication()
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(connection_id))
        .unwrap()
        .position;
    let aloft = Vec3d {
        y: start.y + 3.0,
        ..start
    };
    play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(aloft),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    aloft
}

/// Whom one tick told alice's shared-flags byte, and with which value.
fn shared_flags_sent(tick: &TickResult, entity_id: i32) -> Vec<(u64, i8)> {
    tick.events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(ConnectionId(recipient), packet)| match packet {
            ClientboundPlayPacket::SetEntityData(update) if update.id == entity_id => {
                update.values.iter().find_map(|value| match value.value {
                    EntityDataValueKind::Byte(flags)
                        if value.data_id == VANILLA_ENTITY_SHARED_FLAGS_DATA_ID =>
                    {
                        Some((recipient, flags))
                    }
                    _ => None,
                })
            }
            _ => None,
        })
        .collect()
}

/// `Player.tryToStartFallFlying` (`Player.java:1440-1447`) and
/// `LivingEntity.updateFallFlying` (`LivingEntity.java:3117-3137`) on a real
/// session wearing an elytra: the glide starts in the air, publishes flag and
/// pose, wears the elytra on the 20th glide tick and raises `ELYTRA_GLIDE` on
/// every 10th, ends on landing and at the last durability point, and the wear
/// survives a save and a restart.
#[test]
fn an_elytra_session_glides_wears_its_elytra_and_stops_on_landing_and_last_durability() {
    let root = std::env::temp_dir().join(format!(
        "bbb-server-world-elytra-glide-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-elytra-glide");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let world = ServerWorld::restore(operator_config(&[("alice", 1)]), &candidate).unwrap();
    let mut glider = Glider {
        world,
        sender,
        receiver,
    };
    join_loaded(
        &glider.sender,
        &glider.receiver,
        &mut glider.world,
        1,
        "alice",
        1,
    );
    join_loaded(
        &glider.sender,
        &glider.receiver,
        &mut glider.world,
        2,
        "bob",
        2,
    );
    glider.command("gamemode survival alice");
    // Max damage 432: 429 leaves two wear steps before `nextDamageWillBreak`.
    glider.command("item replace entity alice armor.chest with elytra[damage=429]");
    // The created save's floor tops out at y 64 here.
    let ground = 64.0;
    place_player(
        &glider.sender,
        &glider.receiver,
        &mut glider.world,
        1,
        Vec3d {
            x: 5.5,
            y: ground,
            z: 6.5,
        },
        None,
    );
    let alice = entity_id_of(&glider.world, 1);

    let started = glider.start_fall_flying();
    assert!(
        !glider.fall_flying(),
        "a grounded session cannot start gliding"
    );
    assert!(shared_flags_sent(&started, alice)
        .iter()
        .all(|(_, flags)| flags & VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING == 0));

    glider.move_to(ground + 2.0, false);
    let started = glider.start_fall_flying();
    assert!(glider.fall_flying());
    let sent = shared_flags_sent(&started, alice);
    for recipient in [1, 2] {
        assert!(
            sent.iter().any(|(to, flags)| *to == recipient
                && flags & VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING != 0),
            "session {recipient} is told about the glide: {sent:?}"
        );
    }
    assert_eq!(
        glider.world.player_pose_id_for_test(ConnectionId(1)),
        VANILLA_ENTITY_POSE_FALL_FLYING_ID
    );

    // The start tick was glide tick 1: tick 10 raises the event, tick 20 also
    // wears the elytra.
    glider.ticks(8);
    assert_eq!(
        (glider.glide_events(), glider.elytra_damage()),
        (0, Some(429))
    );
    glider.ticks(1);
    assert_eq!(
        (glider.glide_events(), glider.elytra_damage()),
        (1, Some(429))
    );
    glider.ticks(10);
    assert_eq!(
        (glider.glide_events(), glider.elytra_damage()),
        (2, Some(430))
    );

    glider.move_to(ground, true);
    assert!(!glider.fall_flying(), "landing ends the glide");

    glider.move_to(ground + 2.0, false);
    glider.start_fall_flying();
    assert!(glider.fall_flying());
    glider.ticks(19);
    assert_eq!(
        (glider.glide_events(), glider.elytra_damage()),
        (4, Some(431))
    );
    assert!(glider.fall_flying());
    // 431 is `nextDamageWillBreak`, so the elytra is no glider any more.
    glider.ticks(1);
    assert!(!glider.fall_flying());
    glider.start_fall_flying();
    assert!(!glider.fall_flying());
    assert_eq!(glider.elytra_damage(), Some(431));

    let request = glider
        .world
        .save_request(1_786_000_106_000, 203, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(operator_config(&[("alice", 1)]), &reloaded).unwrap();
    join_loaded(&glider.sender, &glider.receiver, &mut world, 1, "alice", 1);
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(1), CHEST_SLOT)
            .component_patch
            .damage,
        Some(431)
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `travelFallFlying` (`LivingEntity.java:2520-2522`) stops the glide as soon
/// as the glider is on a climbable.
#[test]
fn gliding_into_a_ladder_stops_fall_flying() {
    let root = std::env::temp_dir().join(format!(
        "bbb-server-world-elytra-ladder-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-elytra-ladder");
    let candidate = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let world = ServerWorld::restore(operator_config(&[("alice", 1)]), &candidate).unwrap();
    let mut glider = Glider {
        world,
        sender,
        receiver,
    };
    join_loaded(
        &glider.sender,
        &glider.receiver,
        &mut glider.world,
        1,
        "alice",
        1,
    );
    glider.command("gamemode survival alice");
    glider.command("item replace entity alice armor.chest with elytra");
    let ground = 64.0;
    place_player(
        &glider.sender,
        &glider.receiver,
        &mut glider.world,
        1,
        Vec3d {
            x: 5.5,
            y: ground,
            z: 6.5,
        },
        None,
    );
    glider.move_to(ground + 2.0, false);
    glider.start_fall_flying();
    glider.ticks(2);
    assert!(glider.fall_flying());
    glider.command("setblock 5 66 7 minecraft:stone");
    glider.command("setblock 5 66 6 minecraft:ladder[facing=north]");
    glider.ticks(1);
    assert!(!glider.fall_flying(), "the ladder ends the glide");
    drop(storage);
    let _ = std::fs::remove_dir_all(&root);
}
