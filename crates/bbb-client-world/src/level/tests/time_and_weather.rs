use super::*;

#[test]
fn local_player_auto_jump_option_survives_level_replacement_and_timer_resets() {
    let mut store = respawn_state_with_local_player_data();
    store.set_local_player_auto_jump_enabled(false);
    store.arm_local_player_auto_jump();

    store.apply_respawn(&respawn_packet(0));

    assert!(!store.local_player_auto_jump_enabled());
    assert!(!store.local_player_auto_jump_is_armed());
    store.arm_local_player_auto_jump();
    store.clear_client_level();
    assert!(!store.local_player_auto_jump_enabled());
    assert!(!store.local_player_auto_jump_is_armed());

    store.arm_local_player_auto_jump();
    store.apply_login(&login_packet(0, "minecraft:overworld"));
    assert!(!store.local_player_auto_jump_enabled());
    assert!(!store.local_player_auto_jump_is_armed());
}
