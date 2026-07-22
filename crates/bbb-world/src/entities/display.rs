use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID, VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID,
    VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID,
};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, ItemStackSummary};
use bbb_protocol::RichTextComponent;

use super::components::{EntityIdentity, EntityMetadata, EntityTransform};
use super::schema::{
    vanilla_entity_metadata_accessor_for_entity_type, vanilla_entity_metadata_entity_accessors,
};
use super::EntityVec3;
use crate::TerrainLight;

const DEFAULT_TRANSLATION: [f32; 3] = [0.0; 3];
const DEFAULT_SCALE: [f32; 3] = [1.0; 3];
const DEFAULT_ROTATION: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const DEFAULT_VIEW_RANGE: f32 = 1.0;
const DEFAULT_SHADOW_RADIUS: f32 = 0.0;
const DEFAULT_SHADOW_STRENGTH: f32 = 1.0;
const DEFAULT_CULLING_WIDTH: f32 = 0.0;
const DEFAULT_CULLING_HEIGHT: f32 = 0.0;
const DEFAULT_BRIGHTNESS_OVERRIDE: i32 = -1;
const DEFAULT_GLOW_COLOR_OVERRIDE: i32 = -1;
const DEFAULT_TEXT_LINE_WIDTH: i32 = 200;
const DEFAULT_TEXT_BACKGROUND_COLOR: i32 = 1_073_741_824;
const DEFAULT_TEXT_OPACITY: i8 = -1;
const DEFAULT_TEXT_FLAGS: i8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayBillboardConstraints {
    Fixed,
    Vertical,
    Horizontal,
    Center,
}

impl DisplayBillboardConstraints {
    fn from_vanilla_id(id: i8) -> Self {
        match id {
            1 => Self::Vertical,
            2 => Self::Horizontal,
            3 => Self::Center,
            // `ByIdMap.OutOfBoundsStrategy.ZERO` also maps every invalid id to
            // the first enum constant.
            _ => Self::Fixed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayItemContext {
    None,
    ThirdPersonLeftHand,
    ThirdPersonRightHand,
    FirstPersonLeftHand,
    FirstPersonRightHand,
    Head,
    Gui,
    Ground,
    Fixed,
    OnShelf,
}

impl DisplayItemContext {
    fn from_vanilla_id(id: i8) -> Self {
        match id {
            1 => Self::ThirdPersonLeftHand,
            2 => Self::ThirdPersonRightHand,
            3 => Self::FirstPersonLeftHand,
            4 => Self::FirstPersonRightHand,
            5 => Self::Head,
            6 => Self::Gui,
            7 => Self::Ground,
            8 => Self::Fixed,
            9 => Self::OnShelf,
            // `ItemDisplayContext.BY_ID` has the same ZERO fallback.
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayTransformation {
    pub translation: [f32; 3],
    pub left_rotation: [f32; 4],
    pub scale: [f32; 3],
    pub right_rotation: [f32; 4],
}

impl Default for DisplayTransformation {
    fn default() -> Self {
        Self {
            translation: DEFAULT_TRANSLATION,
            left_rotation: DEFAULT_ROTATION,
            scale: DEFAULT_SCALE,
            right_rotation: DEFAULT_ROTATION,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayEntityCullingState {
    pub view_range: f32,
    pub width: f32,
    pub height: f32,
}

impl DisplayEntityCullingState {
    pub fn affected_by_culling(self) -> bool {
        self.width != 0.0 && self.height != 0.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayEntityRenderContent {
    Block {
        block_state_id: i32,
    },
    Item {
        stack: ItemStackSummary,
        context: DisplayItemContext,
    },
    Text {
        text: Option<RichTextComponent>,
        line_width: i32,
        background_color: i32,
        text_opacity: i8,
        flags: i8,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayEntityRenderSourceState {
    pub entity_id: i32,
    pub entity_type_id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub transformation: DisplayTransformation,
    pub billboard: DisplayBillboardConstraints,
    pub brightness_override: i32,
    pub shadow_radius: f32,
    pub shadow_strength: f32,
    pub glow_color_override: i32,
    pub appears_glowing: bool,
    pub light: TerrainLight,
    pub content: DisplayEntityRenderContent,
}

impl DisplayEntityRenderSourceState {
    pub fn light_coords(&self) -> u32 {
        if self.brightness_override != DEFAULT_BRIGHTNESS_OVERRIDE {
            self.brightness_override as u32
        } else {
            (u32::from(self.light.block.min(15)) << 4) | (u32::from(self.light.sky.min(15)) << 20)
        }
    }
}

pub(crate) fn display_entity_culling_state(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<DisplayEntityCullingState> {
    is_display_entity_type(entity_type_id).then(|| DisplayEntityCullingState {
        view_range: metadata_float(
            entity_type_id,
            data_values,
            "DATA_VIEW_RANGE_ID",
            DEFAULT_VIEW_RANGE,
        ),
        width: metadata_float(
            entity_type_id,
            data_values,
            "DATA_WIDTH_ID",
            DEFAULT_CULLING_WIDTH,
        ),
        height: metadata_float(
            entity_type_id,
            data_values,
            "DATA_HEIGHT_ID",
            DEFAULT_CULLING_HEIGHT,
        ),
    })
}

pub(crate) fn display_entity_render_source(
    identity: &EntityIdentity,
    transform: EntityTransform,
    metadata: &EntityMetadata,
    partial_ticks: f32,
) -> Option<DisplayEntityRenderSourceState> {
    let entity_type_id = identity.entity_type_id;
    if !is_display_entity_type(entity_type_id) {
        return None;
    }
    let data_values = &metadata.data_values;
    let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
    let shared_flags = metadata_byte(
        entity_type_id,
        data_values,
        "DATA_SHARED_FLAGS_ID",
        entity_accessors.shared_flags_default,
    );
    let content = match entity_type_id {
        VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID => DisplayEntityRenderContent::Block {
            block_state_id: metadata_block_state(
                entity_type_id,
                data_values,
                "DATA_BLOCK_STATE_ID",
                crate::vanilla_air_block_state_id(),
            ),
        },
        VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID => DisplayEntityRenderContent::Item {
            stack: metadata_item_stack(entity_type_id, data_values, "DATA_ITEM_STACK_ID")
                .cloned()
                .unwrap_or_else(ItemStackSummary::empty),
            context: DisplayItemContext::from_vanilla_id(metadata_byte(
                entity_type_id,
                data_values,
                "DATA_ITEM_DISPLAY_ID",
                0,
            )),
        },
        VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID => DisplayEntityRenderContent::Text {
            text: metadata_component(entity_type_id, data_values, "DATA_TEXT_ID").cloned(),
            line_width: metadata_int(
                entity_type_id,
                data_values,
                "DATA_LINE_WIDTH_ID",
                DEFAULT_TEXT_LINE_WIDTH,
            ),
            background_color: metadata_int(
                entity_type_id,
                data_values,
                "DATA_BACKGROUND_COLOR_ID",
                DEFAULT_TEXT_BACKGROUND_COLOR,
            ),
            text_opacity: metadata_byte(
                entity_type_id,
                data_values,
                "DATA_TEXT_OPACITY_ID",
                DEFAULT_TEXT_OPACITY,
            ),
            flags: metadata_byte(
                entity_type_id,
                data_values,
                "DATA_STYLE_FLAGS_ID",
                DEFAULT_TEXT_FLAGS,
            ),
        },
        _ => return None,
    };
    Some(DisplayEntityRenderSourceState {
        entity_id: identity.id,
        entity_type_id,
        position: transform.render_position(partial_ticks),
        y_rot: transform.render_y_rot(partial_ticks),
        x_rot: transform.render_x_rot(partial_ticks),
        transformation: DisplayTransformation {
            translation: metadata_vector3(
                entity_type_id,
                data_values,
                "DATA_TRANSLATION_ID",
                DEFAULT_TRANSLATION,
            ),
            left_rotation: metadata_quaternion(
                entity_type_id,
                data_values,
                "DATA_LEFT_ROTATION_ID",
                DEFAULT_ROTATION,
            ),
            scale: metadata_vector3(entity_type_id, data_values, "DATA_SCALE_ID", DEFAULT_SCALE),
            right_rotation: metadata_quaternion(
                entity_type_id,
                data_values,
                "DATA_RIGHT_ROTATION_ID",
                DEFAULT_ROTATION,
            ),
        },
        billboard: DisplayBillboardConstraints::from_vanilla_id(metadata_byte(
            entity_type_id,
            data_values,
            "DATA_BILLBOARD_RENDER_CONSTRAINTS_ID",
            0,
        )),
        brightness_override: metadata_int(
            entity_type_id,
            data_values,
            "DATA_BRIGHTNESS_OVERRIDE_ID",
            DEFAULT_BRIGHTNESS_OVERRIDE,
        ),
        shadow_radius: metadata_float(
            entity_type_id,
            data_values,
            "DATA_SHADOW_RADIUS_ID",
            DEFAULT_SHADOW_RADIUS,
        ),
        shadow_strength: metadata_float(
            entity_type_id,
            data_values,
            "DATA_SHADOW_STRENGTH_ID",
            DEFAULT_SHADOW_STRENGTH,
        ),
        glow_color_override: metadata_int(
            entity_type_id,
            data_values,
            "DATA_GLOW_COLOR_OVERRIDE_ID",
            DEFAULT_GLOW_COLOR_OVERRIDE,
        ),
        appears_glowing: shared_flags & entity_accessors.glowing_mask != 0,
        light: TerrainLight::FULL_BRIGHT,
        content,
    })
}

fn is_display_entity_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID
            | VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID
            | VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID
    )
}

fn metadata_value<'a>(
    entity_type_id: i32,
    data_values: &'a [EntityDataValue],
    java_name: &str,
) -> Option<&'a EntityDataValueKind> {
    let data_id =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, java_name)?.data_id;
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .map(|value| &value.value)
}

fn metadata_byte(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    java_name: &str,
    default: i8,
) -> i8 {
    match metadata_value(entity_type_id, data_values, java_name) {
        Some(EntityDataValueKind::Byte(value)) => *value,
        _ => default,
    }
}

fn metadata_int(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    java_name: &str,
    default: i32,
) -> i32 {
    match metadata_value(entity_type_id, data_values, java_name) {
        Some(EntityDataValueKind::Int(value)) => *value,
        _ => default,
    }
}

fn metadata_float(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    java_name: &str,
    default: f32,
) -> f32 {
    match metadata_value(entity_type_id, data_values, java_name) {
        Some(EntityDataValueKind::Float(value)) => *value,
        _ => default,
    }
}

fn metadata_vector3(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    java_name: &str,
    default: [f32; 3],
) -> [f32; 3] {
    match metadata_value(entity_type_id, data_values, java_name) {
        Some(EntityDataValueKind::Vector3f { x, y, z }) => [*x, *y, *z],
        _ => default,
    }
}

fn metadata_quaternion(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    java_name: &str,
    default: [f32; 4],
) -> [f32; 4] {
    match metadata_value(entity_type_id, data_values, java_name) {
        Some(EntityDataValueKind::Quaternionf { x, y, z, w }) => [*x, *y, *z, *w],
        _ => default,
    }
}

fn metadata_block_state(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    java_name: &str,
    default: i32,
) -> i32 {
    match metadata_value(entity_type_id, data_values, java_name) {
        Some(EntityDataValueKind::BlockState(value)) => *value,
        _ => default,
    }
}

fn metadata_item_stack<'a>(
    entity_type_id: i32,
    data_values: &'a [EntityDataValue],
    java_name: &str,
) -> Option<&'a ItemStackSummary> {
    match metadata_value(entity_type_id, data_values, java_name) {
        Some(EntityDataValueKind::ItemStack(value)) => Some(value),
        _ => None,
    }
}

fn metadata_component<'a>(
    entity_type_id: i32,
    data_values: &'a [EntityDataValue],
    java_name: &str,
) -> Option<&'a RichTextComponent> {
    match metadata_value(entity_type_id, data_values, java_name) {
        Some(EntityDataValueKind::Component(value)) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID, VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID,
    };
    use uuid::Uuid;

    use super::*;
    use crate::entities::components::EntityIncarnation;
    use crate::PublicationShared;

    fn value(
        entity_type_id: i32,
        java_name: &str,
        serializer_id: i32,
        value: EntityDataValueKind,
    ) -> EntityDataValue {
        let accessor = vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, java_name)
            .expect("display accessor");
        assert_eq!(accessor.serializer_id, serializer_id);
        EntityDataValue {
            data_id: accessor.data_id,
            serializer_id,
            value,
        }
    }

    fn identity(entity_type_id: i32) -> EntityIdentity {
        EntityIdentity {
            id: 42,
            uuid: Uuid::nil(),
            entity_type_id,
            data: 0,
            incarnation: EntityIncarnation(1),
        }
    }

    fn transform() -> EntityTransform {
        EntityTransform {
            position: EntityVec3 {
                x: 5.0,
                y: 7.0,
                z: 11.0,
            },
            old_position: EntityVec3 {
                x: 3.0,
                y: 5.0,
                z: 9.0,
            },
            position_base: EntityVec3::default(),
            delta_movement: EntityVec3::default(),
            y_rot: 50.0,
            old_y_rot: 30.0,
            x_rot: 20.0,
            old_x_rot: 10.0,
            y_head_rot: 0.0,
            old_y_head_rot: 0.0,
            on_ground: None,
            interpolation: None,
        }
    }

    fn metadata(values: Vec<EntityDataValue>) -> EntityMetadata {
        EntityMetadata {
            data_values: PublicationShared::from(values),
            painting_variant_projection: PublicationShared::default(),
        }
    }

    #[test]
    fn display_defaults_match_vanilla_define_synched_data() {
        let block = display_entity_render_source(
            &identity(VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID),
            transform(),
            &metadata(Vec::new()),
            0.5,
        )
        .expect("block display source");
        assert_eq!(
            block.position,
            EntityVec3 {
                x: 4.0,
                y: 6.0,
                z: 10.0
            }
        );
        assert_eq!(block.y_rot, 40.0);
        assert_eq!(block.x_rot, 15.0);
        assert_eq!(block.transformation, DisplayTransformation::default());
        assert_eq!(block.billboard, DisplayBillboardConstraints::Fixed);
        assert_eq!(block.brightness_override, -1);
        assert_eq!(block.shadow_radius, 0.0);
        assert_eq!(block.shadow_strength, 1.0);
        assert_eq!(block.glow_color_override, -1);
        assert!(!block.appears_glowing);
        assert_eq!(
            block.content,
            DisplayEntityRenderContent::Block {
                block_state_id: crate::vanilla_air_block_state_id()
            }
        );

        for entity_type_id in [
            VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
            VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID,
            VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID,
        ] {
            assert_eq!(
                display_entity_culling_state(entity_type_id, &[]),
                Some(DisplayEntityCullingState {
                    view_range: 1.0,
                    width: 0.0,
                    height: 0.0,
                })
            );
        }
    }

    #[test]
    fn display_common_metadata_projects_exact_current_render_state_and_culling() {
        let entity_type_id = VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID;
        let values = vec![
            value(
                entity_type_id,
                "DATA_TRANSLATION_ID",
                VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID,
                EntityDataValueKind::Vector3f {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
            ),
            value(
                entity_type_id,
                "DATA_SCALE_ID",
                VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID,
                EntityDataValueKind::Vector3f {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                },
            ),
            value(
                entity_type_id,
                "DATA_LEFT_ROTATION_ID",
                VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID,
                EntityDataValueKind::Quaternionf {
                    x: 0.1,
                    y: 0.2,
                    z: 0.3,
                    w: 0.4,
                },
            ),
            value(
                entity_type_id,
                "DATA_RIGHT_ROTATION_ID",
                VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID,
                EntityDataValueKind::Quaternionf {
                    x: 0.5,
                    y: 0.6,
                    z: 0.7,
                    w: 0.8,
                },
            ),
            value(
                entity_type_id,
                "DATA_BILLBOARD_RENDER_CONSTRAINTS_ID",
                VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                EntityDataValueKind::Byte(3),
            ),
            value(
                entity_type_id,
                "DATA_BRIGHTNESS_OVERRIDE_ID",
                VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                EntityDataValueKind::Int((6 << 4) | (12 << 20)),
            ),
            value(
                entity_type_id,
                "DATA_VIEW_RANGE_ID",
                VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                EntityDataValueKind::Float(2.5),
            ),
            value(
                entity_type_id,
                "DATA_WIDTH_ID",
                VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                EntityDataValueKind::Float(3.0),
            ),
            value(
                entity_type_id,
                "DATA_HEIGHT_ID",
                VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                EntityDataValueKind::Float(4.0),
            ),
            value(
                entity_type_id,
                "DATA_BLOCK_STATE_ID",
                VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID,
                EntityDataValueKind::BlockState(123),
            ),
        ];
        let source = display_entity_render_source(
            &identity(entity_type_id),
            transform(),
            &metadata(values.clone()),
            1.0,
        )
        .expect("block display source");
        assert_eq!(source.transformation.translation, [1.0, 2.0, 3.0]);
        assert_eq!(source.transformation.scale, [4.0, 5.0, 6.0]);
        assert_eq!(source.transformation.left_rotation, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(source.transformation.right_rotation, [0.5, 0.6, 0.7, 0.8]);
        assert_eq!(source.billboard, DisplayBillboardConstraints::Center);
        assert_eq!(source.light_coords(), (6 << 4) | (12 << 20));
        assert_eq!(
            source.content,
            DisplayEntityRenderContent::Block {
                block_state_id: 123
            }
        );
        let culling = display_entity_culling_state(entity_type_id, &values).unwrap();
        assert_eq!(culling.view_range, 2.5);
        assert_eq!(culling.width, 3.0);
        assert_eq!(culling.height, 4.0);
        assert!(culling.affected_by_culling());
    }

    #[test]
    fn item_and_text_display_substates_use_source_verified_ids_and_defaults() {
        let item_type = VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID;
        let stack = ItemStackSummary {
            item_id: Some(7),
            count: 2,
            component_patch: Default::default(),
        };
        let item_source = display_entity_render_source(
            &identity(item_type),
            transform(),
            &metadata(vec![
                value(
                    item_type,
                    "DATA_ITEM_STACK_ID",
                    VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                    EntityDataValueKind::ItemStack(stack.clone()),
                ),
                value(
                    item_type,
                    "DATA_ITEM_DISPLAY_ID",
                    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    EntityDataValueKind::Byte(9),
                ),
            ]),
            1.0,
        )
        .unwrap();
        assert_eq!(
            item_source.content,
            DisplayEntityRenderContent::Item {
                stack,
                context: DisplayItemContext::OnShelf,
            }
        );

        let text_type = VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID;
        let text = RichTextComponent::plain("display text");
        let text_source = display_entity_render_source(
            &identity(text_type),
            transform(),
            &metadata(vec![
                value(
                    text_type,
                    "DATA_TEXT_ID",
                    VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID,
                    EntityDataValueKind::Component(text.clone()),
                ),
                value(
                    text_type,
                    "DATA_LINE_WIDTH_ID",
                    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    EntityDataValueKind::Int(64),
                ),
                value(
                    text_type,
                    "DATA_BACKGROUND_COLOR_ID",
                    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    EntityDataValueKind::Int(0x4020_3040),
                ),
                value(
                    text_type,
                    "DATA_TEXT_OPACITY_ID",
                    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    EntityDataValueKind::Byte(127),
                ),
                value(
                    text_type,
                    "DATA_STYLE_FLAGS_ID",
                    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    EntityDataValueKind::Byte(0b1_1111),
                ),
            ]),
            1.0,
        )
        .unwrap();
        assert_eq!(
            text_source.content,
            DisplayEntityRenderContent::Text {
                text: Some(text),
                line_width: 64,
                background_color: 0x4020_3040,
                text_opacity: 127,
                flags: 0b1_1111,
            }
        );
    }
}
