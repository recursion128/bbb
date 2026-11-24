use super::super::ServerWorld;
use crate::connection::ConnectionId;
use crate::mailbox::{
    world_command_mailbox, WorldCommandReceiver, WorldCommandSender, WorldMailboxLimits,
};
use crate::world::{ServerWorldConfig, WorldCommand};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_ON_FIRE;
use bbb_protocol::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::game_types::GameType;
use bbb_protocol::item_types::VANILLA_ITEM_TURTLE_HELMET_ID;
use bbb_protocol::packets::{
    ClientCommandAction, EquippableSummary, ItemStackSummary, PlayServerbound, SoundEventSummary,
};
use bbb_server_persistence::RegionCompression;
use bbb_server_persistence::{Java26PlayerData, PlayerDataSource};

const PLAYER: ConnectionId = ConnectionId(1);
const WATER_BREATHING: &str = "minecraft:water_breathing";

fn fixture() -> (ServerWorld, WorldCommandSender, WorldCommandReceiver) {
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player = world.levels.player_mut(PLAYER).unwrap();
    {
        let mut position = player.position();
        position.y = 80.0;
        player.set_position(position);
    }
    {
        let mut position = player.position();
        position.x = player.position().x.floor() + 0.5;
        player.set_position(position);
    }
    {
        let mut position = player.position();
        position.z = player.position().z.floor() + 0.5;
        player.set_position(position);
    }
    (world, sender, receiver)
}

fn state(world: &ServerWorld, id: ConnectionId) -> (i32, i32, i32) {
    let player = world.levels.player_at(id);
    (
        player.remaining_fire_ticks,
        player.air_supply,
        player.ticks_frozen,
    )
}

fn set_state(world: &mut ServerWorld, fire: i32, air: i32, frozen: i32) {
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.remaining_fire_ticks = fire;
    player.set_air_supply(air);
    player.set_ticks_frozen(frozen);
}

fn parsed_state(player: &Java26PlayerData) -> (i32, i32, i32) {
    (
        player.remaining_fire_ticks(),
        player.air_supply(),
        player.ticks_frozen(),
    )
}

#[test]
fn player_environment_lifecycle_live_save_documents_restore_typed_state() {
    let (mut world, _, _) = fixture();
    set_state(&mut world, 87, -11, 193);
    world.levels[PLAYER].has_visual_fire = true;
    world.levels[PLAYER].cold.glowing_tag = true;
    let save = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let patch = &save.player_patches[0];
    assert_eq!(
        (
            patch.created.remaining_fire_ticks,
            patch.created.air_supply,
            patch.created.ticks_frozen
        ),
        (87, -11, 193)
    );
    let new =
        bbb_server_persistence::created_player_document(patch, save.item_stack_resolver.as_dyn())
            .unwrap();
    let new = Java26PlayerData::parse(new, patch.uuid).unwrap();
    assert_eq!(parsed_state(&new), (87, -11, 193));
    assert!(new.has_visual_fire());
    assert!(new.glowing());

    set_state(&mut world, -7, 333, 71);
    let save = world
        .save_request(1_786_000_000_001, 2, RegionCompression::Zlib)
        .unwrap();
    let patch = &save.player_patches[0];
    let retained = new
        .raw_with_player_save_patch(patch, save.item_stack_resolver.as_dyn())
        .unwrap();
    let retained = Java26PlayerData::parse(retained, patch.uuid).unwrap();
    assert_eq!(parsed_state(&retained), (-7, 333, 71));

    let root = std::env::temp_dir().join(format!(
        "bbb-player-environment-restore-{}",
        std::process::id()
    ));
    let storage = crate::tests::created_fixture_save(&root, "environment-restore");
    let mut candidate = storage.load_candidate().unwrap();
    candidate
        .players
        .insert(patch.uuid, (PlayerDataSource::Primary, retained));
    let mut restored = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let player = &restored.cold.restored_players[&patch.uuid];
    assert_eq!(
        (
            player.remaining_fire_ticks,
            player.air_supply,
            player.ticks_frozen
        ),
        (-7, 333, 71)
    );
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
    assert_eq!(state(&restored, PLAYER), (-20, 333, 67));
    assert!(restored.levels[PLAYER].has_visual_fire);
    assert!(restored.levels[PLAYER].shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE));
    assert!(restored.levels[PLAYER]
        .shared_flag(bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_GLOWING));
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn player_environment_lifecycle_disconnect_snapshot_survives_rejoin() {
    let (mut world, sender, receiver) = fixture();
    set_state(&mut world, -7, 333, 169);
    sender
        .try_send(WorldCommand::Disconnect {
            connection_id: PLAYER,
        })
        .unwrap();
    world.tick(receiver.drain());
    assert!(!world.levels.contains_player(PLAYER));
    let save = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let saved = &save.player_patches[0].created;
    assert_eq!(
        (
            saved.remaining_fire_ticks,
            saved.air_supply,
            saved.ticks_frozen
        ),
        (-7, 333, 169)
    );
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "alice", 1);
    assert_eq!(state(&world, ConnectionId(2)), (-20, 333, 165));
}

#[test]
fn player_environment_lifecycle_death_clears_fire_and_freeze_respawn_resets_air() {
    let (mut world, sender, receiver) = fixture();
    set_state(&mut world, 200, -8, 140);
    world.levels[PLAYER].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE, true);
    assert!(world.hurt_connected_player(
        PLAYER,
        "minecraft:generic_kill",
        40.0,
        None,
        None,
        None,
        &mut Vec::new()
    ));
    assert_eq!(state(&world, PLAYER), (0, -8, 0));
    assert!(!world.levels[PLAYER].shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE));
    sender
        .try_send(WorldCommand::Play {
            connection_id: PLAYER,
            packet: PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
        })
        .unwrap();
    world.tick(receiver.drain());
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.air_supply, 300);
    assert_eq!(player.ticks_frozen, 0);
    assert!(player.remaining_fire_ticks <= 0);
    assert!(!player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE));
    assert!(player.damage.health > 0.0);
}

fn turtle() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_TURTLE_HELMET_ID),
        count: 1,
        component_patch: Default::default(),
    }
}

fn equip(world: &mut ServerWorld, slot: EquipmentSlot, item: ItemStackSummary) {
    let player = world.levels.player_mut(PLAYER).unwrap();
    let index = player
        .inventory
        .equipment_slots()
        .into_iter()
        .find(|(candidate, _)| *candidate == slot)
        .unwrap()
        .1;
    assert!(player
        .inventory
        .set_inventory_slot(index, crate::tests::native_stack(item)));
}

fn pool(world: &mut ServerWorld) {
    let player = world.levels.player_at(PLAYER);
    let pos = player.position();
    let dimension = player.dimension;
    let water =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:water").unwrap();
    let water = bbb_protocol::block_state_types::vanilla_block_default_state_id(water.id).unwrap();
    let chunks = &mut world.level_mut_by_id(&dimension).chunks;
    for x in (pos.x.floor() as i32 - 2)..=(pos.x.floor() as i32 + 2) {
        for z in (pos.z.floor() as i32 - 2)..=(pos.z.floor() as i32 + 2) {
            for y in 80..83 {
                assert!(chunks.set_block_state(x, y, z, water).is_some());
            }
        }
    }
}

#[test]
fn player_environment_lifecycle_turtle_effect_expires_after_last_protected_air_tick() {
    let (mut world, _, _) = fixture();
    equip(&mut world, EquipmentSlot::Head, turtle());
    world.tick(Vec::new());
    let effect = world.levels[PLAYER].active_effect(WATER_BREATHING).unwrap();
    assert_eq!((effect.duration(), effect.amplifier()), (200, 0));
    assert!(!effect.ambient());
    assert!(!effect.show_particles());
    assert!(effect.show_icon());
    pool(&mut world);
    for _ in 0..199 {
        world.tick(Vec::new());
    }
    assert_eq!(
        world.levels[PLAYER]
            .active_effect(WATER_BREATHING)
            .unwrap()
            .duration(),
        1
    );
    world.levels[PLAYER].set_air_supply(-23);
    let health = world.levels[PLAYER].damage.health;
    world.tick(Vec::new());
    assert_eq!(world.levels[PLAYER].air_supply, -19);
    assert_eq!(world.levels[PLAYER].damage.health, health);
    assert!(world.levels[PLAYER]
        .active_effect(WATER_BREATHING)
        .is_none());
    world.tick(Vec::new());
    assert_eq!(world.levels[PLAYER].air_supply, 0);
    assert_eq!(world.levels[PLAYER].damage.health, health - 2.0);
}

#[test]
fn player_environment_lifecycle_turtle_requires_matching_live_equippable_component() {
    let (mut world, _, _) = fixture();
    equip(&mut world, EquipmentSlot::OffHand, turtle());
    world.tick(Vec::new());
    assert!(world.levels[PLAYER]
        .active_effect(WATER_BREATHING)
        .is_none());

    let mut patched = turtle();
    patched
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
    let sound = SoundEventSummary {
        registry_id: Some(0),
        sound_id: None,
        fixed_range_bits: None,
    };
    patched.component_patch.equippable = Some(EquippableSummary {
        slot_id: VANILLA_EQUIPMENT_SLOTS
            .iter()
            .find(|slot| slot.slot == EquipmentSlot::OffHand)
            .unwrap()
            .id,
        equip_sound: sound.clone(),
        asset_id: None,
        camera_overlay: None,
        allowed_entities: None,
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared: false,
        shearing_sound: sound,
    });
    equip(&mut world, EquipmentSlot::OffHand, patched);
    world.tick(Vec::new());
    assert_eq!(
        world.levels[PLAYER]
            .active_effect(WATER_BREATHING)
            .unwrap()
            .duration(),
        200
    );

    equip(
        &mut world,
        EquipmentSlot::OffHand,
        ItemStackSummary::empty(),
    );
    world
        .remove_player_mob_effect(PLAYER, WATER_BREATHING, &mut Vec::new())
        .unwrap();
    let mut removed = turtle();
    removed
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
    equip(&mut world, EquipmentSlot::Head, removed);
    world.tick(Vec::new());
    assert!(world.levels[PLAYER]
        .active_effect(WATER_BREATHING)
        .is_none());
}

#[test]
fn player_environment_lifecycle_visual_fire_is_not_a_damage_clock_and_survives_death() {
    let (mut world, _, _) = fixture();
    world.levels[PLAYER].has_visual_fire = true;
    world.tick(Vec::new());
    assert_eq!(world.levels[PLAYER].damage.health, 20.0);
    assert!(world.levels[PLAYER].shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE));
    assert!(world.hurt_connected_player(
        PLAYER,
        "minecraft:generic_kill",
        40.0,
        None,
        None,
        None,
        &mut Vec::new()
    ));
    assert_eq!(world.levels[PLAYER].remaining_fire_ticks, -20);
    assert!(world.levels[PLAYER].shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE));
}
