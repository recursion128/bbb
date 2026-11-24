use super::catalog::*;
use crate::terrain::TerrainUvRect;
pub use bbb_render_types::{
    EntityRenderState, SleepingPose, SpearKineticUseParams, SpearKineticWeapon,
    ENTITY_DEFAULT_OUTLINE_COLOR, ENTITY_FULL_BRIGHT_LIGHT_COORDS,
};

mod builders;

/// Vanilla `ModelFeatureRenderer.CrumblingOverlay` projected for one
/// block-entity model instance. `origin` is the block position used by the
/// wrapper's inverse camera pose; `uv` resolves the selected destroy-stage
/// texture in bbb's terrain atlas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockEntityCrumbling {
    pub origin: [f32; 3],
    pub uv: TerrainUvRect,
}

/// Per-frame inputs that only block-entity renderers project.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockEntityRenderState {
    /// Vanilla `ChestRenderState.open` before easing: the combined raw lid
    /// openness (`ChestBlock.opennessCombiner` over the lerped
    /// `ChestLidController.getOpenness(partialTick)` of both joined halves).
    /// `ChestModel::setup_anim` applies the `1 - (1 - o)^3` easing and the
    /// `-o * π/2` lid rotation. `0.0` for every non-chest instance and a
    /// resting chest.
    pub chest_openness: f32,
    /// Vanilla `BellRenderState.ticks` (`blockEntity.ticks + partialTicks`): the
    /// shake counter `BellModel.setupAnim` feeds into
    /// `sin(ticks / π) / (4 + ticks / 3)`. `0.0` (plus the frame's partial tick)
    /// for a resting bell and every non-bell instance — with a `None` shake
    /// direction the angle is zero regardless.
    pub bell_ticks: f32,
    /// Vanilla `BellRenderState.shakeDirection` (`blockEntity.shaking ?
    /// blockEntity.clickDirection : null`): the ring direction selecting the
    /// `BellModel.setupAnim` swing axis, `None` while the bell is not shaking.
    pub bell_shake_direction: Option<BellShakeDirection>,
    /// Vanilla `ShulkerBoxRenderState.progress`
    /// (`blockEntity.getProgress(partialTicks)` — the lerped
    /// `ShulkerBoxBlockEntity` open progress): the raw `0..=1` value
    /// `ShulkerBoxModel.setupAnim` turns into the lid lift
    /// (`24 - progress·8`) and the `270°·progress` lid twist. `0.0` for every
    /// non-shulker-box instance and a resting closed box.
    pub shulker_box_progress: f32,
    /// Vanilla `DecoratedPotRenderState.wobbleStyle`/`wobbleProgress`: the
    /// running pot wobble folded into the pot root transform. `None` for
    /// every non-pot instance and a pot at rest.
    pub decorated_pot_wobble: Option<DecoratedPotWobble>,
    /// Vanilla `BannerRenderState.phase` (`(floorMod(x*7 + y*9 + z*13 +
    /// gameTime, 100L) + partialTicks) / 100`,
    /// `BannerRenderer.extractRenderState`): the per-position flag swing
    /// phase `BannerFlagModel.setupAnim` turns into the flag
    /// `xRot = (-0.0125 + 0.01·cos(2π·phase))·π`. `0.0` for every non-banner
    /// instance.
    pub banner_flag_phase: f32,
    /// Vanilla `BookModel.State.forAnimation`'s `progress` argument (the block
    /// entity's `time + partialTicks`; the fixed `0.0` for the lectern):
    /// `BookModel.setup_anim` folds it into the derived openness
    /// `(sin(progress·0.02)·0.1 + 1.25)·book_open`. `0.0` for every non-book
    /// instance (a closed book). Shared by the enchanting-table and lectern
    /// books.
    pub book_progress: f32,
    /// Vanilla `BookModel.State.forAnimation`'s `openness` argument (the lerped
    /// enchanting-table `open` in `[0, 1]`; the fixed `1.2` for the lectern):
    /// the base book openness before the `progress` bob. `0.0` for every
    /// non-book instance (a closed book).
    pub book_open: f32,
    /// Vanilla `BookModel.State.pageFlip1` (`clamp(frac(flip + 0.25)·1.6 − 0.3,
    /// 0, 1)` in `EnchantTableRenderer.submit`; the fixed `0.1` for the
    /// lectern): the first flip page's turn fraction. `0.0` for non-book
    /// instances.
    pub book_page_flip_1: f32,
    /// Vanilla `BookModel.State.pageFlip2` (`clamp(frac(flip + 0.75)·1.6 − 0.3,
    /// 0, 1)`; the fixed `0.9` for the lectern): the second flip page's turn
    /// fraction. `0.0` for non-book instances.
    pub book_page_flip_2: f32,
    /// Vanilla `EnchantTableRenderer.submit`'s hover offset `0.1 +
    /// sin(time·0.1)·0.01` folded into the enchanting-table book root
    /// transform's vertical translate. `0.0` for the lectern book (a static
    /// transform) and every non-book instance.
    pub book_float_y: f32,
    /// Vanilla `ConduitRenderState.animTime` (`tickCount + partialTick`):
    /// drives the active conduit bob (`sin(t*0.1)`) and the wind phase
    /// (`floor(t / 66) % 3`). `0.0` for every non-conduit instance and for a
    /// fresh conduit before its client tick advances.
    pub conduit_anim_time: f32,
    /// Vanilla `ConduitRenderState.activeRotation` from
    /// `ConduitBlockEntity.getActiveRotation(partialTick)`, in radians
    /// (`(activeRotation + partialTick) * -0.0375`). The active cage consumes
    /// it directly; the inactive shell reproduces vanilla's
    /// `Axis.YP.rotation(state.activeRotation * PI / 180)` quirk. `0.0` for
    /// every non-conduit instance.
    pub conduit_active_rotation: f32,
    /// Vanilla `BlockEntityRenderState.breakProgress`: the highest active
    /// destruction stage at this block position and the block-origin pose used
    /// by `SheetedDecalTextureGenerator`. `None` for ordinary entities and for
    /// block-entity renderers that do not pass the overlay to their model.
    pub block_entity_crumbling: Option<BlockEntityCrumbling>,
    /// Vanilla `SpawnerRenderer.submitEntityInSpawner` wrapper transform:
    /// centered at `(0.5, 0.4, 0.5)`, spun around Y, dropped by `0.2`,
    /// pitched by `-30` degrees, and scaled by `0.53125/maxLength`. `None`
    /// for ordinary world entities.
    pub spawner_display: Option<SpawnerDisplayRenderState>,
    /// Vanilla `EndGatewayRenderState` beam fields for spawning/cooling end gateways. `None` for
    /// end portals, inactive gateways, and every ordinary entity.
    pub end_gateway_beam: Option<EndGatewayBeamRenderState>,
    /// Vanilla `BeaconRenderState` fields for one beam section. `None` for every
    /// model that is not a beacon beam.
    pub beacon_beam: Option<BeaconBeamRenderState>,
}

impl Default for BlockEntityRenderState {
    fn default() -> Self {
        Self {
            chest_openness: 0.0,
            bell_ticks: 0.0,
            bell_shake_direction: None,
            shulker_box_progress: 0.0,
            decorated_pot_wobble: None,
            banner_flag_phase: 0.0,
            book_progress: 0.0,
            book_open: 0.0,
            book_page_flip_1: 0.0,
            book_page_flip_2: 0.0,
            book_float_y: 0.0,
            conduit_anim_time: 0.0,
            conduit_active_rotation: 0.0,
            block_entity_crumbling: None,
            spawner_display: None,
            end_gateway_beam: None,
            beacon_beam: None,
        }
    }
}

impl EntityModelInstance {
    pub(in crate::entity_models) fn illusioner_body_visible_when_invisible(&self) -> bool {
        self.render_state.invisible
            && matches!(
                self.kind,
                EntityModelKind::Illager {
                    family: IllagerModelFamily::Illusioner
                }
            )
    }

    pub(in crate::entity_models) fn illusioner_clone_offset(&self, index: usize) -> [f32; 3] {
        let offset = self.render_state.kind.illager().clone_offsets[index];
        let i = index as f32;
        let age = self.render_state.age_in_ticks;
        [
            offset[0] + (i + age * 0.5).cos() * 0.025,
            offset[1] + (i + age * 0.75).cos() * 0.0125,
            offset[2] + (i + age * 0.7).cos() * 0.025,
        ]
    }

    /// Sets the vanilla `LivingEntityRenderState.isInvisible` flag. A true value
    /// also selects the ordinary non-spectator `isInvisibleToPlayer` baseline;
    /// callers can then set `with_invisible_to_player(false)` for spectator/self-visible
    /// translucent rendering.
    pub fn with_invisible(mut self, invisible: bool) -> Self {
        self.render_state.invisible = invisible;
        self.render_state.invisible_to_player = invisible;
        self
    }

    /// Sets the legacy boolean glowing flag and mirrors vanilla's default white
    /// outline color for callers that do not supply scoreboard-derived color.
    pub fn with_appears_glowing(mut self, appears_glowing: bool) -> Self {
        self.render_state.appears_glowing = appears_glowing;
        self.render_state.outline_color = if appears_glowing {
            ENTITY_DEFAULT_OUTLINE_COLOR
        } else {
            0
        };
        self
    }

    /// Sets vanilla `EntityRenderState.outlineColor` and derives
    /// `appearsGlowing()` from whether the color is non-zero.
    pub fn with_outline_color(mut self, outline_color: u32) -> Self {
        self.render_state.outline_color = outline_color;
        self.render_state.appears_glowing = outline_color != 0;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelInstance {
    pub entity_id: i32,
    pub kind: EntityModelKind,
    pub position: [f32; 3],
    /// Optional runtime texture for the model's base pass. Native only publishes this after the
    /// corresponding image has been uploaded, so geometry and custom texture selection switch as
    /// one render-frame value.
    pub dynamic_texture: Option<EntityDynamicTexture>,
    pub render_state: EntityRenderState,
    pub block_entity: BlockEntityRenderState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_coords_select_vanilla_red_row_when_hurt() {
        let calm = EntityModelInstance::zombie(1, [0.0, 0.0, 0.0], 0.0, false);
        // NO_WHITE_U = 0, WHITE_OVERLAY_V = 10 (no overlay).
        assert_eq!(calm.render_state.overlay_coords(), [0.0, 10.0]);

        let hurt = calm.with(|s| s.has_red_overlay = true);
        // NO_WHITE_U = 0, RED_OVERLAY_V = 3.
        assert_eq!(hurt.render_state.overlay_coords(), [0.0, 3.0]);

        // White swelling overlay drives the u column: u = (int)(progress * 15).
        let swelling = calm.with(|s| s.white_overlay_progress = 0.8);
        assert_eq!(swelling.render_state.overlay_coords(), [12.0, 10.0]);
        // Red overlay still wins the v row when both are active.
        let both = swelling.with(|s| s.has_red_overlay = true);
        assert_eq!(both.render_state.overlay_coords(), [12.0, 3.0]);
    }

    #[test]
    fn shader_light_normalizes_packed_block_and_sky() {
        // Full bright packs block 15, sky 15 -> [1.0, 1.0].
        let bright = EntityModelInstance::sheep(1, [0.0, 0.0, 0.0], 0.0, false);
        assert_eq!(bright.render_state.shader_light(), [1.0, 1.0]);

        // pack(block 7, sky 0) = 7 << 4 = 112; pack(block 0, sky 15) = 15 << 20.
        let block_only = bright.with(|s| s.light_coords = 7 << 4);
        assert_eq!(block_only.render_state.shader_light(), [7.0 / 15.0, 0.0]);
        let sky_only = EntityModelInstance::sheep(2, [0.0, 0.0, 0.0], 0.0, false)
            .with(|s| s.light_coords = 15 << 20);
        assert_eq!(sky_only.render_state.shader_light(), [0.0, 1.0]);
    }
}
