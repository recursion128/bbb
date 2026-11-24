use super::*;

#[cfg(test)]
pub(super) fn hotbar_item_icons(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    partial_tick: f32,
) -> [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] {
    hotbar_item_icons_with_input_context(
        &HudItemModelContext::new(world, item_runtime),
        partial_tick,
        false,
        ItemModelKeybindContext::default(),
    )
}

#[cfg(test)]
pub(super) fn hotbar_item_icons_with_extended_view(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    partial_tick: f32,
    shift_down: bool,
) -> [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] {
    hotbar_item_icons_with_input_context(
        &HudItemModelContext::new(world, item_runtime),
        partial_tick,
        shift_down,
        ItemModelKeybindContext::default(),
    )
}

pub(super) fn hotbar_item_icons_with_input_context(
    item_models: &HudItemModelContext<'_>,
    partial_tick: f32,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
) -> [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] {
    let world = item_models.world;
    let item_runtime = item_models.item_runtime;
    let mut icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] = std::array::from_fn(|_| None);
    if !world.hud_camera_player_is_local_player() {
        return icons;
    }
    let selected_slot = usize::from(world.local_player().selected_hotbar_slot.min(8));
    let using_selected_item = world.local_player().interaction.using_item;
    let pop_times = world.hotbar_pop_times();
    let hotbar = world.inventory().hotbar_item_states();
    for (slot_index, item) in hotbar.into_iter().enumerate() {
        let Some(item) = item else { continue };
        let selected_item = slot_index == selected_slot;
        let fishing_rod_cast =
            local_player_fishing_rod_casts_item(world, item_runtime, item.item, selected_item);
        icons[slot_index] = hud_item_icon(
            item_models,
            item.item,
            HudItemSlotState {
                bundle_selected_item_index: item.local_selected_bundle_item_index(),
                using_item: using_selected_item && selected_item,
                selected_item,
                fishing_rod_cast,
                shift_down,
                keybind: keybind_context,
                seed: i32::try_from(slot_index + 1).unwrap_or(0),
                ..HudItemSlotState::default()
            },
            world.local_player_team_color_rgb(),
            partial_tick,
        );
        // Vanilla `Gui.extractSlot` poses the slot by
        // `itemStack.getPopTime() - getGameTimeDeltaPartialTick(false)`.
        if let Some(icon) = icons[slot_index].as_mut() {
            icon.pop = pop_times
                .get(slot_index)
                .map_or(0.0, |ticks| f32::from(*ticks) - partial_tick);
        }
    }

    icons
}

pub(super) fn hud_offhand_slot_side(world: &WorldStore) -> Option<bool> {
    (world.hud_camera_player_is_local_player()
        && world
            .inventory()
            .offhand_item_state()
            .is_some_and(|state| state.item.count > 0))
    .then(|| !world.local_player_main_arm_left().unwrap_or(false))
}

pub(super) fn hud_offhand_slot(
    item_models: &HudItemModelContext<'_>,
    partial_tick: f32,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
) -> Option<HudOffhandSlot> {
    let world = item_models.world;
    let on_left = hud_offhand_slot_side(world)?;
    let offhand = world.inventory().offhand_item_state()?;
    let icon = hud_item_icon(
        item_models,
        offhand.item,
        HudItemSlotState {
            bundle_selected_item_index: offhand.local_selected_bundle_item_index(),
            shift_down,
            keybind: keybind_context,
            seed: i32::try_from(HUD_HOTBAR_SLOTS + 1).unwrap_or(0),
            ..HudItemSlotState::default()
        },
        world.local_player_team_color_rgb(),
        partial_tick,
    )?;
    Some(HudOffhandSlot { on_left, icon })
}

pub(super) fn hud_offhand_block_item_model(
    item_models: &HudItemModelContext<'_>,
    terrain_textures: &TerrainTextureState,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
) -> Option<HudBlockItemModel> {
    let world = item_models.world;

    hud_offhand_slot_side(world)?;
    let offhand = world.inventory().offhand_item_state()?;
    hud_block_item_model(
        item_models,
        offhand.item,
        HudItemSlotState {
            bundle_selected_item_index: offhand.local_selected_bundle_item_index(),
            shift_down,
            keybind: keybind_context,
            seed: i32::try_from(HUD_HOTBAR_SLOTS + 1).unwrap_or(0),
            ..HudItemSlotState::default()
        },
        world.local_player_team_color_rgb(),
        terrain_textures,
    )
}

pub(super) fn hotbar_block_item_models(
    item_models: &HudItemModelContext<'_>,
    terrain_textures: &TerrainTextureState,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
) -> Vec<Option<HudBlockItemModel>> {
    let world = item_models.world;
    let item_runtime = item_models.item_runtime;
    let mut models: Vec<Option<HudBlockItemModel>> = (0..HUD_HOTBAR_SLOTS).map(|_| None).collect();
    if !world.hud_camera_player_is_local_player() {
        return models;
    }
    let selected_slot = usize::from(world.local_player().selected_hotbar_slot.min(8));
    let using_selected_item = world.local_player().interaction.using_item;
    let hotbar = world.inventory().hotbar_item_states();
    for (slot, item) in hotbar.into_iter().enumerate() {
        let Some(item) = item else { continue };
        let selected_item = slot == selected_slot;
        let fishing_rod_cast =
            local_player_fishing_rod_casts_item(world, item_runtime, item.item, selected_item);
        models[slot] = hud_block_item_model(
            item_models,
            item.item,
            HudItemSlotState {
                bundle_selected_item_index: item.local_selected_bundle_item_index(),
                using_item: using_selected_item && selected_item,
                selected_item,
                fishing_rod_cast,
                shift_down,
                keybind: keybind_context,
                seed: i32::try_from(slot + 1).unwrap_or(0),
                ..HudItemSlotState::default()
            },
            world.local_player_team_color_rgb(),
            terrain_textures,
        );
    }
    models
}
