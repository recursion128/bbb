use bbb_world::WorldStore;

pub(super) fn apply_system_chat_update(
    world: &mut WorldStore,
    chat: bbb_protocol::packets::SystemChat,
) {
    world.apply_system_chat(chat);
}

pub(super) fn apply_action_bar_update(
    world: &mut WorldStore,
    text: bbb_protocol::packets::SetActionBarText,
) {
    world.apply_action_bar_text(text);
}

pub(super) fn apply_title_text_update(
    world: &mut WorldStore,
    text: bbb_protocol::packets::SetTitleText,
) {
    world.apply_title_text(text);
}

pub(super) fn apply_subtitle_text_update(
    world: &mut WorldStore,
    text: bbb_protocol::packets::SetSubtitleText,
) {
    world.apply_subtitle_text(text);
}

pub(super) fn apply_clear_titles_update(
    world: &mut WorldStore,
    clear: bbb_protocol::packets::ClearTitles,
) {
    world.apply_clear_titles(clear);
}

pub(super) fn apply_titles_animation_update(
    world: &mut WorldStore,
    animation: bbb_protocol::packets::SetTitlesAnimation,
) {
    world.apply_titles_animation(animation);
}

pub(super) fn apply_player_position_update(
    world: &mut WorldStore,
    update: bbb_protocol::packets::PlayerPositionUpdate,
) {
    world.apply_player_position(update);
}

pub(super) fn apply_player_rotation_update(
    world: &mut WorldStore,
    update: bbb_protocol::packets::PlayerRotationUpdate,
) {
    world.apply_player_rotation(update);
}

pub(super) fn apply_player_look_at_update(
    world: &mut WorldStore,
    update: bbb_protocol::packets::PlayerLookAt,
) {
    world.apply_player_look_at(update);
}

#[cfg(test)]
mod tests;
