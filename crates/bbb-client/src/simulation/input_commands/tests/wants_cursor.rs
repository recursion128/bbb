use super::*;
use crate::input::{AdvancementsScreen, ClientInputState, Screen, StatsScreen};
use bbb_client_world::{BlockEntityRecord, BlockPos, ChunkColumn, ChunkPos, ChunkState, LightData};
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, CommonPlayerSpawnInfo, EntityEvent as ProtocolEntityEvent,
    OpenSignEditor, PlayLogin,
};
use winit::keyboard::{KeyCode, PhysicalKey};

#[test]
fn wants_cursor_for_sign_editor() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();

    assert!(!input.wants_cursor());

    insert_empty_sign(&mut world, BlockPos { x: 1, y: 2, z: 3 });
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut Default::default(), &None);

    assert!(input.wants_cursor());
}

#[test]
fn wants_cursor_for_advancements_screen() {
    let mut input = ClientInputState::new(true);

    assert!(!input.wants_cursor());
    input.set_screen_for_test(Screen::Advancements(AdvancementsScreen::default()));
    assert!(input.wants_cursor());
}

#[test]
fn wants_cursor_for_stats_screen() {
    let mut input = ClientInputState::new(true);

    assert!(!input.wants_cursor());
    input.set_screen_for_test(Screen::Stats(StatsScreen::default()));
    assert!(input.wants_cursor());
}

#[test]
fn wants_cursor_for_social_interactions_screen() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();

    assert!(!input.wants_cursor());
    input.open_social_interactions_screen(&mut world, &mut Default::default(), &None);
    assert!(input.wants_cursor());
}

#[test]
fn wants_cursor_for_debug_game_mode_switcher() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 12345,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: 42,
        event_id: 26,
    }));

    assert!(!input.wants_cursor());
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.wants_cursor());
}

#[test]
fn wants_cursor_for_debug_options_screen() {
    let mut input = ClientInputState::new(true);

    assert!(!input.wants_cursor());
    input.open_debug_options_screen(&mut Default::default(), &None);
    assert!(input.wants_cursor());
}

#[test]
fn wants_cursor_for_debug_pause_screen() {
    let mut input = ClientInputState::new(true);

    assert!(!input.wants_cursor());
    input.open_debug_pause_screen_without_menu_for_test();
    assert!(input.wants_cursor());
}

fn insert_empty_sign(world: &mut WorldStore, pos: BlockPos) {
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: Vec::new(),
        block_entities: std::sync::Arc::new(vec![BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).unwrap(),
            pos.z.rem_euclid(16) as u8,
            bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
        )
        .with_raw_nbt(vec![0])
        .with_payload(bbb_client_world::BlockEntityPayload::SignText(
            Box::default(),
        ))]),
        light: LightData::default(),
    });
}
