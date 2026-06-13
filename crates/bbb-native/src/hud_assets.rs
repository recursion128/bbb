use anyhow::Result;
use bbb_pack::PackRoots;

pub(crate) fn load_hud_textures(renderer: &mut bbb_renderer::Renderer) {
    if let Err(err) = try_load_hud_textures(renderer) {
        tracing::warn!(?err, "continuing without vanilla HUD sprites");
    }
}

fn try_load_hud_textures(renderer: &mut bbb_renderer::Renderer) -> Result<()> {
    let roots = PackRoots::discover()?;
    let crosshair = roots.load_gui_sprite_image("hud/crosshair")?;
    renderer.upload_hud_crosshair(crosshair.width, crosshair.height, &crosshair.rgba)?;
    let hotbar = roots.load_gui_sprite_image("hud/hotbar")?;
    renderer.upload_hud_hotbar(hotbar.width, hotbar.height, &hotbar.rgba)?;
    let hotbar_selection = roots.load_gui_sprite_image("hud/hotbar_selection")?;
    renderer.upload_hud_hotbar_selection(
        hotbar_selection.width,
        hotbar_selection.height,
        &hotbar_selection.rgba,
    )?;
    let experience_background = roots.load_gui_sprite_image("hud/experience_bar_background")?;
    renderer.upload_hud_experience_background(
        experience_background.width,
        experience_background.height,
        &experience_background.rgba,
    )?;
    let experience_progress = roots.load_gui_sprite_image("hud/experience_bar_progress")?;
    renderer.upload_hud_experience_progress(
        experience_progress.width,
        experience_progress.height,
        &experience_progress.rgba,
    )?;
    let heart_container = roots.load_gui_sprite_image("hud/heart/container")?;
    renderer.upload_hud_heart_container(
        heart_container.width,
        heart_container.height,
        &heart_container.rgba,
    )?;
    let heart_full = roots.load_gui_sprite_image("hud/heart/full")?;
    renderer.upload_hud_heart_full(heart_full.width, heart_full.height, &heart_full.rgba)?;
    let heart_half = roots.load_gui_sprite_image("hud/heart/half")?;
    renderer.upload_hud_heart_half(heart_half.width, heart_half.height, &heart_half.rgba)?;
    let food_empty = roots.load_gui_sprite_image("hud/food_empty")?;
    renderer.upload_hud_food_empty(food_empty.width, food_empty.height, &food_empty.rgba)?;
    let food_full = roots.load_gui_sprite_image("hud/food_full")?;
    renderer.upload_hud_food_full(food_full.width, food_full.height, &food_full.rgba)?;
    let food_half = roots.load_gui_sprite_image("hud/food_half")?;
    renderer.upload_hud_food_half(food_half.width, food_half.height, &food_half.rgba)?;
    tracing::info!(
        crosshair = ?(crosshair.width, crosshair.height),
        hotbar = ?(hotbar.width, hotbar.height),
        experience = ?(experience_background.width, experience_background.height),
        heart = ?(heart_full.width, heart_full.height),
        food = ?(food_full.width, food_full.height),
        "loaded vanilla HUD sprites"
    );
    Ok(())
}
