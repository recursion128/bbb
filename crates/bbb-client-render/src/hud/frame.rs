use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudSoundSubtitlePlacement {
    BeforeScreen,
    Deferred,
}
impl Renderer {
    pub(crate) fn append_hud_block_item_residency(&self, frame: &mut ItemModelResidencyFrame) {
        if self.hud_screen_blur {
            self.append_hud_block_item_residency_for_stratum(
                frame,
                HudScreenStratum::BeforeBlur,
                ItemModelDrawTarget::GuiBeforeBlur,
            );
            self.append_hud_block_item_residency_for_stratum(
                frame,
                HudScreenStratum::AfterBlur,
                ItemModelDrawTarget::GuiAfterBlur,
            );
        } else {
            self.append_hud_block_item_residency_for_stratum(
                frame,
                HudScreenStratum::All,
                ItemModelDrawTarget::Gui,
            );
        }
    }

    fn append_hud_block_item_residency_for_stratum(
        &self,
        frame: &mut ItemModelResidencyFrame,
        stratum: HudScreenStratum,
        target: ItemModelDrawTarget,
    ) {
        let surface_size = self.gui_surface_size();
        self.visit_hud_block_item_models_for_stratum(stratum, |model, placement, scissor| {
            match scissor {
                None => {
                    append_hud_block_item_model_residency(frame, model, placement, target, None);
                }
                Some(scissor) => {
                    if let Some(scissor) = hud_item_model_scissor_rect(surface_size, scissor) {
                        append_hud_block_item_model_residency(
                            frame,
                            model,
                            placement,
                            target,
                            Some(scissor),
                        );
                    }
                }
            }
        });
    }

    pub(super) fn visit_hud_block_item_models_for_stratum(
        &self,
        stratum: HudScreenStratum,
        mut visit: impl FnMut(&HudBlockItemModel, glam::Mat4, Option<HudRect>),
    ) {
        let surface_size = self.gui_surface_size();
        let mut append_model =
            |model: &HudBlockItemModel, placement: glam::Mat4, scissor: Option<HudRect>| {
                visit(model, placement, scissor);
            };
        if stratum != HudScreenStratum::AfterBlur
            && !self.hide_gui
            && matches!(&self.hud_hotbar_mode, HudHotbarMode::Ordinary)
        {
            for (slot, model) in self.hud_hotbar_block_item_models.iter().enumerate() {
                if let Some(model) = model {
                    let rect = hotbar_item_hud_rect(surface_size, slot);
                    let pop = self
                        .hud_hotbar_item_icons
                        .get(slot)
                        .and_then(|icon| icon.as_ref())
                        .map_or(0.0, |icon| icon.pop);
                    let placement =
                        hud_item_pop_placement(rect, pop) * gui_item_slot_placement(rect);
                    append_model(model, placement, None);
                }
            }
            if let (Some(offhand), Some(model)) =
                (&self.hud_offhand_slot, &self.hud_offhand_block_item_model)
            {
                let placement = gui_item_slot_placement(hotbar_offhand_item_hud_rect(
                    surface_size,
                    offhand.on_left,
                ));
                append_model(model, placement, None);
            }
        }
        if stratum != HudScreenStratum::BeforeBlur {
            for toast in &self.hud_toasts {
                match toast {
                    HudToast::Advancement(toast) => {
                        if let Some(model) = &toast.block_model {
                            append_model(
                                model,
                                gui_item_slot_placement(hud_advancement_toast_icon_rect(
                                    surface_size,
                                    toast,
                                )),
                                None,
                            );
                        }
                    }
                    HudToast::Recipe(toast) => {
                        if let Some(model) = &toast.category_block_model {
                            append_model(
                                model,
                                gui_item_slot_placement(hud_recipe_toast_category_icon_rect(
                                    surface_size,
                                    toast,
                                )),
                                None,
                            );
                        }
                        if let Some(model) = &toast.unlocked_block_model {
                            append_model(
                                model,
                                gui_item_slot_placement(hud_recipe_toast_unlocked_icon_rect(
                                    surface_size,
                                    toast,
                                )),
                                None,
                            );
                        }
                    }
                    HudToast::System(_) | HudToast::Tutorial(_) => {}
                }
            }
        }
        // The open inventory screen's block items (container slots + the cursor / floating item) render as
        // 3D icons in the same pass, seated in their slot pixel rects.
        if stratum != HudScreenStratum::BeforeBlur {
            if let Some(screen) = &self.hud_inventory_screen {
                for slot in &screen.slots {
                    if let Some(model) = &slot.block_model {
                        let rect = inventory_slot_item_hud_rect(
                            surface_size,
                            screen.width,
                            screen.height,
                            slot.x,
                            slot.y,
                        );
                        append_model(model, gui_item_slot_placement(rect), None);
                    }
                }
                for item in &screen.floating_items {
                    if let Some(model) = &item.block_model {
                        let item_rect =
                            inventory_floating_item_hud_rect(surface_size, screen, item);
                        let scissor_rect =
                            inventory_floating_item_scissor_hud_rect(surface_size, screen, item);
                        append_model(model, gui_item_slot_placement(item_rect), scissor_rect);
                    }
                }
                for item in &screen.foreground_items {
                    if let Some(model) = &item.block_model {
                        let item_rect =
                            inventory_floating_item_hud_rect(surface_size, screen, item);
                        let scissor_rect =
                            inventory_floating_item_scissor_hud_rect(surface_size, screen, item);
                        append_model(model, gui_item_slot_placement(item_rect), scissor_rect);
                    }
                }
            }
        }
        if stratum != HudScreenStratum::AfterBlur {
            if let Some(screen) = &self.hud_stats_screen {
                if screen.active_tab == 1 {
                    for (row, item) in screen.item_rows.iter().enumerate() {
                        if let Some(model) = &item.block_model {
                            let rect = hud_stats_item_icon_rect(surface_size, screen, row);
                            append_model(
                                model,
                                gui_item_slot_placement(rect),
                                Some(hud_stats_list_scissor_rect(surface_size)),
                            );
                        }
                    }
                }
            }
        }
        if stratum != HudScreenStratum::AfterBlur {
            if let Some(switcher) = self
                .hud_debug_overlay
                .as_ref()
                .and_then(|overlay| overlay.game_mode_switcher.as_ref())
            {
                for slot in &switcher.slots {
                    if let Some(model) = &slot.block_model {
                        if let Some(rect) = hud_debug_game_mode_switcher_icon_rect(slot) {
                            append_model(model, gui_item_slot_placement(rect), None);
                        }
                    }
                }
            }
        }
    }

    pub(super) fn hud_has_any_screen(&self) -> bool {
        self.hud_screen_present
            || self.hud_chat_context.focused
            || self.hud_code_of_conduct_screen.is_some()
    }

    pub(super) fn hud_has_ingame_ui_screen(&self) -> bool {
        self.hud_ingame_ui_present
    }

    pub(super) fn hud_sound_subtitle_placement(&self) -> Option<HudSoundSubtitlePlacement> {
        if self.hud_level_loading_screen.is_some() {
            return None;
        }
        if self.hide_gui && !self.hud_has_ingame_ui_screen() {
            return None;
        }
        Some(if self.hud_has_any_screen() {
            HudSoundSubtitlePlacement::BeforeScreen
        } else {
            HudSoundSubtitlePlacement::Deferred
        })
    }

    /// Vanilla `GameRenderer.render` extracts `Gui.extractDebugOverlay` after
    /// the screen, so the F3 overlay also draws above `LevelLoadingScreen`,
    /// the win screen and `DisconnectedScreen`.
    pub(super) fn push_hud_debug_overlay_block<'a>(
        &'a self,
        vertices: &mut Vec<HudVertex>,
        commands: &mut Vec<HudDrawCommand<'a>>,
        post_gui_item_commands: &mut Vec<HudDrawCommand<'a>>,
    ) {
        let surface_size = self.gui_surface_size();
        if let Some(debug_overlay) = &self.hud_debug_overlay {
            push_hud_debug_overlay(
                vertices,
                commands,
                post_gui_item_commands,
                &self.hud_white_pixel,
                self.hud_item_atlas(),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_digit_glyphs,
                self.gui_texture("textures/gui/container/gamemode_switcher.png"),
                self.gui_sprite("gamemode_switcher/slot"),
                self.gui_sprite("gamemode_switcher/selection"),
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                debug_overlay,
            );
            if debug_overlay.show_lightmap_preview {
                push_hud_debug_lightmap_preview(vertices, post_gui_item_commands, surface_size);
            }
        }
    }

    /// Vanilla `GameRenderer.render` extracts the `ToastManager` after the
    /// screen, so every toast family also draws above `LevelLoadingScreen`,
    /// the win screen and `DisconnectedScreen`.
    pub(super) fn push_hud_toast_block<'a>(
        &'a self,
        vertices: &mut Vec<HudVertex>,
        commands: &mut Vec<HudDrawCommand<'a>>,
        post_gui_item_commands: &mut Vec<HudDrawCommand<'a>>,
    ) {
        if self.hide_gui {
            return;
        }
        let surface_size = self.gui_surface_size();
        for toast in &self.hud_toasts {
            match toast {
                HudToast::System(toast) => push_hud_system_toast(
                    vertices,
                    post_gui_item_commands,
                    &self.hud_white_pixel,
                    self.gui_sprite("toast/system"),
                    self.pack.hud_font_atlas.as_ref(),
                    &self.pack.hud_font_glyphs,
                    &self.pack.hud_obfuscated_glyph_pool,
                    crate::text_obfuscation_seed(),
                    surface_size,
                    &toast.toast,
                    toast.slot,
                ),
                HudToast::Advancement(toast) => push_hud_advancement_toasts(
                    vertices,
                    commands,
                    post_gui_item_commands,
                    &self.hud_white_pixel,
                    self.hud_item_atlas(),
                    self.pack.hud_font_atlas.as_ref(),
                    &self.pack.hud_digit_glyphs,
                    self.gui_sprite("toast/advancement"),
                    self.pack.hud_font_atlas.as_ref(),
                    &self.pack.hud_font_glyphs,
                    &self.pack.hud_obfuscated_glyph_pool,
                    crate::text_obfuscation_seed(),
                    surface_size,
                    std::slice::from_ref(toast),
                ),
                HudToast::Recipe(toast) => push_hud_recipe_toasts(
                    vertices,
                    commands,
                    post_gui_item_commands,
                    &self.hud_white_pixel,
                    self.hud_item_atlas(),
                    self.pack.hud_font_atlas.as_ref(),
                    &self.pack.hud_digit_glyphs,
                    self.gui_sprite("toast/recipe"),
                    self.pack.hud_font_atlas.as_ref(),
                    &self.pack.hud_font_glyphs,
                    &self.pack.hud_obfuscated_glyph_pool,
                    crate::text_obfuscation_seed(),
                    surface_size,
                    std::slice::from_ref(toast),
                ),
                HudToast::Tutorial(toast) => push_hud_tutorial_toasts(
                    vertices,
                    commands,
                    post_gui_item_commands,
                    &self.hud_white_pixel,
                    self.gui_sprite("toast/tutorial"),
                    &self.pack.gui_sprites,
                    self.pack.hud_font_atlas.as_ref(),
                    &self.pack.hud_font_glyphs,
                    &self.pack.hud_obfuscated_glyph_pool,
                    crate::text_obfuscation_seed(),
                    surface_size,
                    std::slice::from_ref(toast),
                ),
            }
        }
        if let Some(toast) = self
            .hud_now_playing_toast
            .as_ref()
            .filter(|toast| toast.placement == HudNowPlayingToastPlacement::SlidingToast)
        {
            push_hud_now_playing_toast(
                vertices,
                post_gui_item_commands,
                &self.hud_white_pixel,
                self.gui_sprite("toast/now_playing"),
                &self.pack.gui_sprites,
                self.pack.hud_font_atlas.as_ref(),
                &self.pack.hud_font_glyphs,
                &self.pack.hud_obfuscated_glyph_pool,
                crate::text_obfuscation_seed(),
                surface_size,
                toast,
            );
        }
    }
}
