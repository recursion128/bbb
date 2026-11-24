use super::*;

pub(super) fn hud_inventory_entity_previews(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    local_state: &InventoryHudLocalState,
    enchantment_book: Option<EnchantmentBookHudState>,
    partial_tick: f32,
) -> Vec<HudEntityPreview> {
    match background {
        InventoryScreenBackground::LocalInventory => {
            hud_local_inventory_entity_preview(world, item_runtime, local_state, partial_tick)
                .into_iter()
                .collect()
        }
        InventoryScreenBackground::Mount { .. } => {
            hud_mount_inventory_entity_preview(world, item_runtime, local_state, partial_tick)
                .into_iter()
                .collect()
        }
        InventoryScreenBackground::Smithing => {
            vec![hud_smithing_entity_preview(world, item_runtime)]
        }
        InventoryScreenBackground::EnchantmentTable => enchantment_book
            .map(hud_enchantment_book_preview)
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn hud_local_inventory_entity_preview(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    local_state: &InventoryHudLocalState,
    partial_tick: f32,
) -> Option<HudEntityPreview> {
    const X0: i32 = 26;
    const Y0: i32 = 8;
    const X1: i32 = 75;
    const Y1: i32 = 78;
    const SCALE: f32 = 30.0;
    const OFFSET_Y: f32 = 0.0625;

    let local_player_id = world.local_player_id()?;
    hud_entity_in_inventory_follows_mouse_preview(
        world,
        item_runtime,
        local_player_id,
        local_state.cursor_position,
        partial_tick,
        [X0, Y0, X1, Y1],
        SCALE,
        OFFSET_Y,
    )
}

fn hud_mount_inventory_entity_preview(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    local_state: &InventoryHudLocalState,
    partial_tick: f32,
) -> Option<HudEntityPreview> {
    const X0: i32 = 26;
    const Y0: i32 = 18;
    const X1: i32 = 78;
    const Y1: i32 = 70;
    const SCALE: f32 = 17.0;
    const OFFSET_Y: f32 = 0.25;

    let mount_entity_id = world.inventory().open_container.as_ref()?.mount?.entity_id;
    hud_entity_in_inventory_follows_mouse_preview(
        world,
        item_runtime,
        mount_entity_id,
        local_state.cursor_position,
        partial_tick,
        [X0, Y0, X1, Y1],
        SCALE,
        OFFSET_Y,
    )
}

pub(super) fn hud_enchantment_book_preview(state: EnchantmentBookHudState) -> HudEntityPreview {
    const ENTITY_ID: i32 = -1;
    const GUI_BOOK_SCALE: f32 = 40.0;
    const GUI_BOOK_Y: f32 = 17.0;
    const WORLD_BOOK_ROOT_Z_ROTATION_DEGREES: f32 = 80.0;

    let open = state.open.clamp(0.0, 1.0);
    let page_flip_1 = ((fraction(state.flip + 0.25) * 1.6) - 0.3).clamp(0.0, 1.0);
    let page_flip_2 = ((fraction(state.flip + 0.75) * 1.6) - 0.3).clamp(0.0, 1.0);
    let mut entity = EntityModelInstance::enchanting_book(ENTITY_ID, [-0.5, -0.75, -0.5], 0.0)
        .with_block_entity(|s| {
            s.book_open = open;
            s.book_page_flip_1 = page_flip_1;
            s.book_page_flip_2 = page_flip_2;
        });
    entity.render_state.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
    entity.render_state.outline_color = 0;
    entity.render_state.appears_glowing = false;

    // `EntityModelKind::EnchantingBook` carries the in-world `Rz(80)` root.
    // Cancel that final rotation and fold `GuiBookModelRenderer`'s exact rigid
    // pose into the existing entity-PIP camera.
    let gui_base =
        Quat::from_rotation_y(std::f32::consts::PI) * Quat::from_rotation_x(25.0_f32.to_radians());
    let closed = 1.0 - open;
    let gui_translation = gui_base * Vec3::new(closed * 0.2, closed * 0.1, closed * 0.25);
    let rotation = gui_base
        * Quat::from_rotation_y((-(closed * 90.0) - 90.0).to_radians())
        * Quat::from_rotation_x(std::f32::consts::PI)
        * Quat::from_rotation_z(-WORLD_BOOK_ROOT_Z_ROTATION_DEGREES.to_radians());

    HudEntityPreview {
        entity,
        lighting: GuiItemLightingEntry::EntityInUi,
        rect: HudEntityPreviewRect {
            x: 14,
            y: 14,
            width: 38,
            height: 31,
        },
        scissor: None,
        translation: (gui_translation
            + Vec3::new(0.0, (GUI_BOOK_Y - 31.0 / 2.0) / GUI_BOOK_SCALE, 0.0))
        .to_array(),
        rotation: rotation.to_array(),
        override_camera_rotation: None,
        scale: GUI_BOOK_SCALE,
        depth_isolated: true,
        after_background_layer: Some(0),
        item_layers: Vec::new(),
    }
}

fn fraction(value: f32) -> f32 {
    value - value.floor()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn hud_entity_in_inventory_follows_mouse_preview(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    cursor_position: Option<(i32, i32)>,
    partial_tick: f32,
    rect: [i32; 4],
    scale: f32,
    offset_y: f32,
) -> Option<HudEntityPreview> {
    const MOUSE_FOLLOW_DIVISOR: f32 = 40.0;
    const ROTATION_SCALE_DEGREES: f32 = 20.0;

    let [x0, y0, x1, y1] = rect;
    let mut entity = entity_model_instance_from_world_entity_at_partial_tick(
        world,
        item_runtime,
        entity_id,
        partial_tick,
    )?;
    let bounds = world.probe_entity_pick_bounds(entity_id)?;
    let height = bounds.max[1] - bounds.min[1];
    let center_x = (x0 + x1) as f32 / 2.0;
    let center_y = (y0 + y1) as f32 / 2.0;
    let (mouse_x, mouse_y) = cursor_position
        .map(|(x, y)| (x as f32, y as f32))
        .unwrap_or((center_x, center_y));
    let x_angle = ((center_x - mouse_x) / MOUSE_FOLLOW_DIVISOR).atan();
    let y_angle = ((center_y - mouse_y) / MOUSE_FOLLOW_DIVISOR).atan();
    let yaw_degrees = x_angle * ROTATION_SCALE_DEGREES;
    let pitch_degrees = y_angle * ROTATION_SCALE_DEGREES;
    let camera_x_rotation = quaternion_x(pitch_degrees.to_radians());

    // GuiEntityRenderer.renderToTexture submits the entity at the preview origin.
    entity.position = [0.0; 3];
    entity.render_state.body_rot = 180.0 + yaw_degrees;
    entity.render_state.head_yaw = yaw_degrees;
    entity.render_state.head_pitch = -pitch_degrees;
    entity.render_state.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
    entity.render_state.outline_color = 0;
    entity.render_state.appears_glowing = false;

    Some(HudEntityPreview {
        entity,
        lighting: GuiItemLightingEntry::EntityInUi,
        rect: HudEntityPreviewRect {
            x: x0,
            y: y0,
            width: u32::try_from(x1 - x0).ok()?,
            height: u32::try_from(y1 - y0).ok()?,
        },
        scissor: None,
        translation: [0.0, height / 2.0 + offset_y, 0.0],
        rotation: quaternion_mul([0.0, 0.0, 1.0, 0.0], camera_x_rotation),
        override_camera_rotation: Some(camera_x_rotation),
        scale,
        depth_isolated: true,
        after_background_layer: None,
        item_layers: Vec::new(),
    })
}

fn hud_smithing_entity_preview(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> HudEntityPreview {
    const ENTITY_ID: i32 = -1;
    const X0: i32 = 121;
    const Y0: i32 = 20;
    const X1: i32 = 161;
    const Y1: i32 = 80;
    const SCALE: f32 = 25.0;
    const X_ROT_DEGREES: f32 = 25.0;
    const BODY_ROT_DEGREES: f32 = 210.0;
    const ARMOR_STAND_X_ROT_RADIANS: f32 = 0.43633232;

    let mut entity = EntityModelInstance::armor_stand(
        ENTITY_ID,
        [0.0, 0.0, 0.0],
        BODY_ROT_DEGREES,
        false,
        true,
        false,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (0.0, X_ROT_DEGREES));
    entity.render_state.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
    entity.render_state.outline_color = 0;
    entity.render_state.appears_glowing = false;
    let item_layers = apply_smithing_result_equipment(&mut entity, world, item_runtime);

    HudEntityPreview {
        entity,
        lighting: GuiItemLightingEntry::EntityInUi,
        rect: HudEntityPreviewRect {
            x: X0,
            y: Y0,
            width: u32::try_from(X1 - X0).unwrap_or_default(),
            height: u32::try_from(Y1 - Y0).unwrap_or_default(),
        },
        scissor: None,
        translation: [0.0, 1.0, 0.0],
        rotation: quaternion_mul(
            quaternion_x(ARMOR_STAND_X_ROT_RADIANS),
            quaternion_z(std::f32::consts::PI),
        ),
        override_camera_rotation: None,
        scale: SCALE,
        depth_isolated: true,
        after_background_layer: None,
        item_layers,
    }
}

fn apply_smithing_result_equipment(
    entity: &mut EntityModelInstance,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudEntityPreviewItemLayer> {
    const SMITHING_RESULT_SLOT: i16 = 3;
    const ARMOR_STAND_ITEM_IN_HAND_LAYER_SEQUENCE: u32 = 1;
    const ARMOR_STAND_CUSTOM_HEAD_LAYER_SEQUENCE: u32 = 2;

    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let Some(stack) = open_container_slot_item(world, SMITHING_RESULT_SLOT) else {
        return Vec::new();
    };
    if item_stack_is_empty(stack) {
        return Vec::new();
    }
    let Some(item_id) = stack.item_id else {
        return Vec::new();
    };

    let equipment = world.item_stack_equipment(stack);
    let equipment_asset = equipment.and_then(|(_, asset)| asset);
    let equipment_asset_present = equipment_asset.is_some();
    match equipment.map(|(slot, _)| slot) {
        Some(EquipmentSlot::Head) if equipment_asset_present => {
            entity.render_state.head_equipment_asset = equipment_asset;
            entity.render_state.head_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.head_armor_foil = stack.has_foil();
        }
        Some(EquipmentSlot::Head) => {
            if smithing_result_is_custom_head_skull(item_runtime, item_id) {
                entity.render_state.custom_head_skull =
                    item_runtime.custom_head_skull_for_stack(stack);
            } else {
                return vec![smithing_preview_item_layer(
                    HudEntityPreviewItemSlot::Head,
                    HudEntityPreviewItemDisplayContext::Head,
                    stack,
                    ARMOR_STAND_CUSTOM_HEAD_LAYER_SEQUENCE,
                )];
            }
        }
        Some(EquipmentSlot::Chest) => {
            // The world entity path derives the wings layer from the same
            // resolved chest asset; the renderer resolves each layer kind.
            entity.render_state.chest_equipment_asset = equipment_asset;
            entity.render_state.chest_wings_equipment_asset = equipment_asset;
            entity.render_state.chest_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.chest_armor_foil = equipment_asset_present && stack.has_foil();
        }
        Some(EquipmentSlot::Legs) => {
            entity.render_state.legs_equipment_asset = equipment_asset;
            entity.render_state.legs_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.legs_armor_foil = equipment_asset_present && stack.has_foil();
        }
        Some(EquipmentSlot::Feet) => {
            entity.render_state.feet_equipment_asset = equipment_asset;
            entity.render_state.feet_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.feet_armor_foil = equipment_asset_present && stack.has_foil();
        }
        Some(
            EquipmentSlot::MainHand
            | EquipmentSlot::OffHand
            | EquipmentSlot::Body
            | EquipmentSlot::Saddle,
        )
        | None => {
            return vec![smithing_preview_item_layer(
                HudEntityPreviewItemSlot::LeftHand,
                HudEntityPreviewItemDisplayContext::ThirdPersonLeftHand,
                stack,
                ARMOR_STAND_ITEM_IN_HAND_LAYER_SEQUENCE,
            )];
        }
    }
    Vec::new()
}

fn smithing_preview_item_layer(
    slot: HudEntityPreviewItemSlot,
    display_context: HudEntityPreviewItemDisplayContext,
    stack: &ItemStackSummary,
    submit_sequence: u32,
) -> HudEntityPreviewItemLayer {
    HudEntityPreviewItemLayer {
        slot,
        display_context,
        item_id: stack.item_id.unwrap_or_default(),
        count: stack.count,
        foil: stack.has_foil(),
        light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
        overlay: ITEM_MODEL_NO_OVERLAY,
        order: 0,
        submit_sequence,
    }
}

fn smithing_result_is_custom_head_skull(item_runtime: &NativeItemRuntime, item_id: i32) -> bool {
    item_runtime
        .item_resource_id(item_id)
        .is_some_and(|resource_id| {
            matches!(
                resource_id,
                "minecraft:skeleton_skull"
                    | "minecraft:wither_skeleton_skull"
                    | "minecraft:zombie_head"
                    | "minecraft:creeper_head"
                    | "minecraft:piglin_head"
                    | "minecraft:dragon_head"
                    | "minecraft:player_head"
            )
        })
}

pub(super) fn quaternion_x(angle_radians: f32) -> [f32; 4] {
    let half = angle_radians / 2.0;
    [half.sin(), 0.0, 0.0, half.cos()]
}

pub(super) fn quaternion_z(angle_radians: f32) -> [f32; 4] {
    let half = angle_radians / 2.0;
    [0.0, 0.0, half.sin(), half.cos()]
}

pub(super) fn quaternion_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}
