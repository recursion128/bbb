//! The player's `WardenSpawnTracker` across the session boundary: the counters
//! a restore seeds from the document, the ones a
//! tick advances while the session is online, and the live value a
//! save writes back.

use super::*;

use crate::block_entity::vibration::GameEventTag;
use crate::entity::warden_spawn_tracker::WardenSpawnTracker;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const SHRIEKER: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
/// Three blocks from the shrieker, inside the sensor's listener radius and
/// inside `WardenSpawnTracker.PLAYER_SEARCH_RADIUS` (`:27`) of the player.
const SOURCE: BlockPos = BlockPos { x: 3, y: 64, z: 0 };
const STAND: Vec3d = Vec3d {
    x: 8.5,
    y: 64.0,
    z: 0.5,
};

fn counters(live: WardenSpawnTracker) -> [i32; 3] {
    [
        live.ticks_since_last_warning(),
        live.warning_level(),
        live.cooldown_ticks(),
    ]
}

fn saved_counters(saved: &bbb_server_persistence::Java26WardenSpawnTracker) -> [i32; 3] {
    [
        saved.ticks_since_last_warning(),
        saved.warning_level(),
        saved.cooldown_ticks(),
    ]
}

fn shrieker_state(shrieking: bool) -> i32 {
    helpers::shrieker_state(shrieking, true)
}

/// `SculkShriekerBlockEntity.warningLevel` off the live object.
fn shrieker_warning_level(world: &ServerWorld) -> i32 {
    world
        .get_block_entity(DIM, SHRIEKER)
        .and_then(|entity| entity.sculk_shrieker())
        .expect("the shrieker's block entity")
        .warning_level()
}

/// The `warden_spawn_tracker` compound one session's `/data get entity`
/// projection carries.
fn command_counters(world: &ServerWorld, uuid: Uuid) -> [i32; 3] {
    let snapshot = world.command_entity_data_snapshot_for_test();
    let data = &snapshot
        .iter()
        .find(|entry| entry.uuid == uuid)
        .expect("the session's command projection")
        .data;
    let NbtSummaryValue::Compound(entries) = data else {
        panic!("a projected entity is a compound: {data:?}");
    };
    let value = entries
        .iter()
        .find(|entry| entry.name == "warden_spawn_tracker")
        .map(|entry| &entry.value)
        .expect("`ServerPlayer.addAdditionalSaveData` writes the tracker");
    let NbtSummaryValue::Compound(tracker) = value else {
        panic!("the tracker is a compound: {value:?}");
    };
    let field = |name: &str| match tracker
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
    {
        Some(NbtSummaryValue::Int(value)) => *value,
        other => panic!("{name} is an int: {other:?}"),
    };
    [
        field("ticks_since_last_warning"),
        field("warning_level"),
        field("cooldown_ticks"),
    ]
}

/// A save on disk whose single player document carries the given counters,
/// opened as a world, plus the save barrier the runtime runs.
struct WardenSave {
    root: std::path::PathBuf,
    storage: Java26WorldStorage,
    candidate: Java26WorldCandidate,
    uuid: Uuid,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    world: ServerWorld,
}

impl WardenSave {
    /// `document` is the counters the save holds for `alice`; `None` is a save
    /// that has never held a document for her.
    fn open(name: &str, document: Option<[i32; 3]>) -> Self {
        let root = std::env::temp_dir().join(format!("bbb-warden-{name}-{}", std::process::id()));
        let storage = created_fixture_save(&root, name);
        let mut candidate = storage.load_candidate().unwrap();
        let uuid = profile("alice", 1).uuid;
        if let Some(counters) = document {
            candidate.players.insert(
                uuid,
                (
                    PlayerDataSource::Primary,
                    restore_fixture_player_with_warden_spawn_tracker(
                        uuid,
                        [STAND.x, STAND.y, STAND.z],
                        [0.0, 0.0],
                        true,
                        counters,
                    ),
                ),
            );
        }
        let (sender, receiver) = mailbox();
        let world = spawn_pinned_restore(datapack_config(), &candidate);
        Self {
            root,
            storage,
            candidate,
            uuid,
            sender,
            receiver,
            world,
        }
    }

    fn live(&self) -> Option<[i32; 3]> {
        self.world
            .warden_spawn_tracker_for_test(self.uuid)
            .map(counters)
    }

    fn join(&mut self) {
        join_loaded(&self.sender, &self.receiver, &mut self.world, 1, "alice", 1);
    }

    /// The session in play next to the shrieker, in survival because
    /// `WardenSpawnTracker.tryWarn` only searches non-spectating, living
    /// players (`:100-103`).
    fn join_and_stand(&mut self) {
        self.join();
        stand_at(&self.sender, &self.receiver, &mut self.world, 1, STAND);
        self.world.set_player_mode_and_abilities_for_test(
            ConnectionId(1),
            GameType::Survival,
            PlayerAbilities::for_game_type(GameType::Survival),
        );
    }

    fn tick(&mut self) {
        let events = self.receiver.drain();
        self.world.tick(events);
    }

    /// A real block change four blocks away travels to the shrieker and
    /// reaches `WardenSpawnTracker.tryWarn`.
    fn warn_via_shrieker(&mut self) {
        let placed =
            self.world
                .set_block(DIM, SHRIEKER, shrieker_state(false), SET_BLOCK_UPDATE_ALL);
        assert!(placed, "the shrieker is placed");
        self.world
            .level_block_entity_for_test(DIM, SHRIEKER)
            .and_then(|entity| entity.sculk_shrieker_mut())
            .expect("the placement created the shrieker's block entity")
            .set_listenable_events_for_test(GameEventTag::Vibrations);
        self.tick();
        let player = self.world.player_entity_id_for_test(ConnectionId(1));
        self.world.record_game_event_for_test(
            DIM,
            VanillaGameEvent::BLOCK_CHANGE,
            SOURCE,
            Some(player),
            None,
        );
        for _ in 0..24 {
            self.tick();
            if self.world.block_state(DIM, SHRIEKER) == Some(shrieker_state(true)) {
                return;
            }
        }
        panic!("the shrieker never warned");
    }

    fn save(&mut self) {
        let request = self
            .world
            .save_request(1_786_000_000_500, 210, RegionCompression::Zlib)
            .unwrap();
        self.storage
            .save_snapshot(&self.candidate, &request)
            .unwrap();
    }

    /// The counters the file holds for `alice` after a save.
    fn saved(&self) -> [i32; 3] {
        saved_counters(
            self.storage
                .load_candidate()
                .unwrap()
                .load_player(self.uuid)
                .unwrap()
                .expect("the save holds her document")
                .1
                .warden_spawn_tracker(),
        )
    }

    fn reload(&self) -> ServerWorld {
        spawn_pinned_restore(datapack_config(), &self.storage.load_candidate().unwrap())
    }

    fn close(self) {
        self.storage.close().unwrap();
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

/// `ServerPlayer.readAdditionalSaveData` hands the
/// session the document's counters, so a login after a restart continues from
/// them; an idle tick with nobody online must not drop what the save holds.
#[test]
fn a_join_seeds_the_live_tracker_from_the_documents_counters() {
    let mut save = WardenSave::open("seed", Some([11, 2, 7]));
    assert_eq!(save.live(), Some([11, 2, 7]), "the restore seeds the map");
    save.tick();
    assert_eq!(
        save.live(),
        Some([11, 2, 7]),
        "a tick with nobody online leaves the loaded counters alone"
    );
    save.join();
    // The join's two ticks advance the session's own counter
    // which is the only thing that moves it.
    assert_eq!(save.live(), Some([13, 2, 5]));
    assert_eq!(command_counters(&save.world, save.uuid), [13, 2, 5]);
    save.close();
}

/// `ServerPlayer`: a fresh tracker ticks before its first warning.
#[test]
fn a_fresh_session_ticks_its_tracker_before_the_first_warning() {
    let mut save = WardenSave::open("fresh", None);
    assert_eq!(save.live(), None, "a save with no document seeds nothing");
    save.join();
    assert_eq!(save.live(), Some([2, 0, 0]));
    save.close();
}

#[test]
fn a_shrieker_warning_continues_from_the_seeded_tracker() {
    for (saved_level, expected_level) in [(2, 3), (4, 4)] {
        let mut save = WardenSave::open(&format!("warn-{saved_level}"), Some([0, saved_level, 0]));
        save.join_and_stand();
        save.warn_via_shrieker();
        assert_eq!(shrieker_warning_level(&save.world), expected_level);
        let live = save.live().expect("the session has a tracker");
        assert_eq!(live[1], expected_level);
        assert!(live[2] > 0);
        assert_eq!(command_counters(&save.world, save.uuid), live);
        save.close();
    }
}

/// `ServerPlayer.addAdditionalSaveData` writes the
/// tracker the session has been ticking, and a reload continues from it.
#[test]
fn a_warned_session_saves_its_live_tracker_and_the_reload_continues_from_it() {
    let mut save = WardenSave::open("round-trip", Some([0, 2, 0]));
    save.join_and_stand();
    save.warn_via_shrieker();
    let live = save.live().expect("the warning made a live tracker");
    assert_eq!(live, [1, 3, 199]);
    save.save();
    assert_eq!(
        save.saved(),
        live,
        "the file holds the session's own counters, not the ones it joined with"
    );
    let restored = save.reload();
    assert_eq!(
        restored
            .warden_spawn_tracker_for_test(save.uuid)
            .map(counters),
        Some(live),
        "the reloaded world continues from the saved counters"
    );
    save.close();
}

/// `ServerPlayer`: saving reads the body's tracker rather than its online index.
#[test]
fn direct_warning_uses_precise_warden_bounds_and_keeps_unremoved_dead_wardens() {
    for (x, health, nearby) in [(25.0, 500.0, false), (24.8, 500.0, true), (24.0, 0.0, true)] {
        let mut save = WardenSave::open(&format!("warden-bounds-{x}-{health}"), None);
        save.join_and_stand();
        assert_eq!(save.live(), Some([3, 0, 0]));
        assert!(save
            .world
            .set_block(DIM, SHRIEKER, shrieker_state(false), SET_BLOCK_UPDATE_ALL));
        let data = crate::entity::GeneralLivingData::new(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID,
        )
        .unwrap();
        let (id, _) = save.world.spawn_stored_entity_for_test(
            DIM,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID,
            Vec3d { x, y: 64.0, z: 0.5 },
            crate::entity::EntityData::GeneralLiving(data),
        );
        save.world
            .stored_entity_mut_for_test(DIM, id)
            .unwrap()
            .living_mut()
            .unwrap()
            .set_health(health);
        let mut events = Vec::new();
        save.world
            .step_on_sculk_shrieker_for_test(&DIM.into(), SHRIEKER, save.uuid, &mut events);
        assert_eq!(
            save.world.block_state(DIM, SHRIEKER),
            Some(shrieker_state(!nearby))
        );
        assert_eq!(
            save.live(),
            if nearby {
                Some([3, 0, 0])
            } else {
                Some([0, 1, 200])
            }
        );
        assert_eq!(
            shrieker_warning_level(&save.world),
            if nearby { 0 } else { 1 }
        );
        save.close();
    }
}

#[test]
fn a_stored_item_step_warns_its_thrower_through_the_real_world_tick() {
    let mut save = WardenSave::open("stored-item-step", None);
    save.join_and_stand();
    assert!(save
        .world
        .set_block(DIM, SHRIEKER, shrieker_state(false), SET_BLOCK_UPDATE_ALL));
    let stack = crate::entity::item::plain_stack("minecraft:stone", 1).unwrap();
    let mut item = crate::entity::ItemEntityData::new(stack, 0.0);
    item.set_thrower(Some(save.uuid));
    let (id, _) = save.world.spawn_stored_entity_for_test(
        DIM,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 0.5,
            y: 64.5,
            z: 0.5,
        },
        crate::entity::EntityData::Item(item),
    );
    save.world
        .stored_entity_mut_for_test(DIM, id)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: -0.01,
            z: 0.0,
        });
    assert_eq!(save.live(), Some([3, 0, 0]));
    save.tick();
    assert_eq!(
        save.world.block_state(DIM, SHRIEKER),
        Some(shrieker_state(true))
    );
    assert_eq!(shrieker_warning_level(&save.world), 1);
    assert_eq!(save.live(), Some([1, 1, 199]));
    let player = save.world.player_entity_id_for_test(ConnectionId(1));
    let shriek = save
        .world
        .positioned_game_events_for_test(DIM)
        .into_iter()
        .find(|event| event.recorded.game_event == VanillaGameEvent::SHRIEK)
        .expect("the actual stored contact raised SHRIEK");
    assert_eq!(shriek.recorded.source_entity_id, Some(player));
    assert_eq!(shriek.recorded.pos, SHRIEKER);
    assert_eq!(
        shriek.position,
        Vec3d {
            x: 0.5,
            y: 64.5,
            z: 0.5
        }
    );
    save.close();
}
