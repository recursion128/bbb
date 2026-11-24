//! `net/minecraft/world/entity/Display.java` on the server: the shared
//! `Display` synched rows and save fields, and the `BlockDisplay`,
//! `ItemDisplay` and `TextDisplay` halves on top of them.
//!
//! Everything `Display.tick` does beyond the vehicle check is gated on
//! `level().isClientSide()` (`Display.java:159-184`); the render state,
//! the interpolation handler and the culling box (`:386-395`) are client
//! presentation and have no reader on this side, so none of them is kept.
//! Entity::new owns noPhysics (Display.java:109-113); movement consumes it before collision.
//!
//! The synched-data ids below are declared here because the generated
//! `bbb_protocol::entity_synched_data` catalog carries no `Display`
//! accessor yet.

pub mod save_fields;

use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID, VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID, VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID,
    VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID,
};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, Vec3d};
use bbb_protocol::{
    component_codec_values_equal, NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::display::save_fields::{
    bool_or, byte_bool, byte_or, entry, fixed_float_list, float_list, float_or, int_or,
    numeric_float, numeric_int, optional_field,
};
use crate::entity::item::{empty_stack, read_saved_stack};
use crate::entity::living::SynchedValue;
use crate::entity::save::EntitySaveError;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

/// `Display`'s `SynchedEntityData.defineId` rows in declaration order
/// (`Display.java:46-66`), numbered after the eight `Entity` rows
/// (`Entity.java:311-318`).
pub const DISPLAY_TRANSFORMATION_INTERPOLATION_START_DELTA_TICKS_DATA_ID: u8 = 8;
pub const DISPLAY_TRANSFORMATION_INTERPOLATION_DURATION_DATA_ID: u8 = 9;
pub const DISPLAY_POS_ROT_INTERPOLATION_DURATION_DATA_ID: u8 = 10;
pub const DISPLAY_TRANSLATION_DATA_ID: u8 = 11;
pub const DISPLAY_SCALE_DATA_ID: u8 = 12;
pub const DISPLAY_LEFT_ROTATION_DATA_ID: u8 = 13;
pub const DISPLAY_RIGHT_ROTATION_DATA_ID: u8 = 14;
pub const DISPLAY_BILLBOARD_RENDER_CONSTRAINTS_DATA_ID: u8 = 15;
pub const DISPLAY_BRIGHTNESS_OVERRIDE_DATA_ID: u8 = 16;
pub const DISPLAY_VIEW_RANGE_DATA_ID: u8 = 17;
pub const DISPLAY_SHADOW_RADIUS_DATA_ID: u8 = 18;
pub const DISPLAY_SHADOW_STRENGTH_DATA_ID: u8 = 19;
pub const DISPLAY_WIDTH_DATA_ID: u8 = 20;
pub const DISPLAY_HEIGHT_DATA_ID: u8 = 21;
pub const DISPLAY_GLOW_COLOR_OVERRIDE_DATA_ID: u8 = 22;
/// `Display.BlockDisplay.DATA_BLOCK_STATE_ID` (`Display.java:463-465`).
pub const BLOCK_DISPLAY_BLOCK_STATE_DATA_ID: u8 = 23;
/// `Display.ItemDisplay.DATA_ITEM_STACK_ID`/`DATA_ITEM_DISPLAY_ID`
/// (`Display.java:556-558`).
pub const ITEM_DISPLAY_ITEM_STACK_DATA_ID: u8 = 23;
pub const ITEM_DISPLAY_ITEM_DISPLAY_DATA_ID: u8 = 24;
/// `Display.TextDisplay`'s five rows (`Display.java:675-681`).
pub const TEXT_DISPLAY_TEXT_DATA_ID: u8 = 23;
pub const TEXT_DISPLAY_LINE_WIDTH_DATA_ID: u8 = 24;
pub const TEXT_DISPLAY_BACKGROUND_COLOR_DATA_ID: u8 = 25;
pub const TEXT_DISPLAY_TEXT_OPACITY_DATA_ID: u8 = 26;
pub const TEXT_DISPLAY_STYLE_FLAGS_DATA_ID: u8 = 27;

/// `Display.NO_BRIGHTNESS_OVERRIDE`/`NO_GLOW_COLOR_OVERRIDE` and the
/// `INITIAL_*` defaults (`Display.java:45`, `:77-85`).
pub const NO_BRIGHTNESS_OVERRIDE: i32 = -1;
pub const NO_GLOW_COLOR_OVERRIDE: i32 = -1;
pub const INITIAL_TRANSFORMATION_INTERPOLATION_DURATION: i32 = 0;
pub const INITIAL_TRANSFORMATION_START_INTERPOLATION: i32 = 0;
pub const INITIAL_POS_ROT_INTERPOLATION_DURATION: i32 = 0;
pub const INITIAL_SHADOW_RADIUS: f32 = 0.0;
pub const INITIAL_SHADOW_STRENGTH: f32 = 1.0;
pub const INITIAL_VIEW_RANGE: f32 = 1.0;
pub const INITIAL_WIDTH: f32 = 0.0;
pub const INITIAL_HEIGHT: f32 = 0.0;

/// `TextDisplay.FLAG_*` and its `INITIAL_*` (`Display.java:667-674`).
pub const FLAG_SHADOW: i8 = 1;
pub const FLAG_SEE_THROUGH: i8 = 2;
pub const FLAG_USE_DEFAULT_BACKGROUND: i8 = 4;
pub const FLAG_ALIGN_LEFT: i8 = 8;
pub const FLAG_ALIGN_RIGHT: i8 = 16;
pub const INITIAL_TEXT_OPACITY: i8 = -1;
pub const INITIAL_BACKGROUND: i32 = 1_073_741_824;
pub const INITIAL_LINE_WIDTH: i32 = 200;

/// `Entity.isPickable()` (`Entity.java:1923-1925`), not overridden by any
/// display, and `Entity.canBeHitByProjectile()` (`:1919-1921`) over it.
pub const IS_PICKABLE: bool = false;
pub const CAN_BE_HIT_BY_PROJECTILE: bool = false;

/// `org.joml.Vector3f` as the three synched rows carry it; equality is JOML's
/// `Float.floatToIntBits` comparison per component.
#[derive(Debug, Clone, Copy)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3f {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const fn components(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    fn data_value(self) -> EntityDataValueKind {
        EntityDataValueKind::Vector3f {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

impl PartialEq for Vector3f {
    fn eq(&self, other: &Self) -> bool {
        self.x.to_bits() == other.x.to_bits()
            && self.y.to_bits() == other.y.to_bits()
            && self.z.to_bits() == other.z.to_bits()
    }
}

impl Eq for Vector3f {}

/// `org.joml.Quaternionf` as the two rotation rows carry it; equality is
/// JOML's `Float.floatToIntBits` comparison per component.
#[derive(Debug, Clone, Copy)]
pub struct Quaternionf {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternionf {
    /// `new Quaternionf()`, the identity rotation.
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub const fn components(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }

    /// JOML 1.10.8 `Quaternionf.normalize()` as the 26.1 server's bundled
    /// `joml-1.10.8.jar` compiles it: `invNorm = Math.invsqrt(fma(x, x,
    /// fma(y, y, fma(z, z, w * w))))`, where `Options.USE_MATH_FMA` is off by
    /// default so each `fma(a, b, c)` is `a * b + c` in float, and `invsqrt`
    /// is `1.0F / (float) Math.sqrt((double) value)`.
    pub fn normalized(self) -> Self {
        let sum = self.x * self.x + (self.y * self.y + (self.z * self.z + self.w * self.w));
        let inv_norm = 1.0_f32 / (f64::from(sum).sqrt() as f32);
        Self {
            x: self.x * inv_norm,
            y: self.y * inv_norm,
            z: self.z * inv_norm,
            w: self.w * inv_norm,
        }
    }

    fn data_value(self) -> EntityDataValueKind {
        EntityDataValueKind::Quaternionf {
            x: self.x,
            y: self.y,
            z: self.z,
            w: self.w,
        }
    }
}

impl PartialEq for Quaternionf {
    fn eq(&self, other: &Self) -> bool {
        self.x.to_bits() == other.x.to_bits()
            && self.y.to_bits() == other.y.to_bits()
            && self.z.to_bits() == other.z.to_bits()
            && self.w.to_bits() == other.w.to_bits()
    }
}

impl Eq for Quaternionf {}

/// `com.mojang.math.Transformation`'s decomposed form
/// (`Transformation.java:55-67`): the four values `Display.setTransformation`
/// fans out into synched rows (`Display.java:230-235`) and
/// `createTransformation` gathers back (`:144-150`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transformation {
    pub translation: Vector3f,
    pub left_rotation: Quaternionf,
    pub scale: Vector3f,
    pub right_rotation: Quaternionf,
}

impl Transformation {
    /// `Transformation.IDENTITY` (`Transformation.java:41-49`).
    pub const IDENTITY: Self = Self {
        translation: Vector3f::ZERO,
        left_rotation: Quaternionf::IDENTITY,
        scale: Vector3f::ONE,
        right_rotation: Quaternionf::IDENTITY,
    };
}

/// `Display.BillboardConstraints` (`Display.java:433-459`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillboardConstraints {
    Fixed,
    Vertical,
    Horizontal,
    Center,
}

impl BillboardConstraints {
    pub const ALL: [Self; 4] = [Self::Fixed, Self::Vertical, Self::Horizontal, Self::Center];

    pub const fn id(self) -> i8 {
        match self {
            Self::Fixed => 0,
            Self::Vertical => 1,
            Self::Horizontal => 2,
            Self::Center => 3,
        }
    }

    pub const fn serialized_name(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
            Self::Center => "center",
        }
    }

    pub fn for_serialized_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|constraints| constraints.serialized_name() == name)
    }
}

/// `net.minecraft.world.item.ItemDisplayContext` (`ItemDisplayContext.java:8-18`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemDisplayContext {
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

impl ItemDisplayContext {
    pub const ALL: [Self; 10] = [
        Self::None,
        Self::ThirdPersonLeftHand,
        Self::ThirdPersonRightHand,
        Self::FirstPersonLeftHand,
        Self::FirstPersonRightHand,
        Self::Head,
        Self::Gui,
        Self::Ground,
        Self::Fixed,
        Self::OnShelf,
    ];

    pub const fn id(self) -> i8 {
        match self {
            Self::None => 0,
            Self::ThirdPersonLeftHand => 1,
            Self::ThirdPersonRightHand => 2,
            Self::FirstPersonLeftHand => 3,
            Self::FirstPersonRightHand => 4,
            Self::Head => 5,
            Self::Gui => 6,
            Self::Ground => 7,
            Self::Fixed => 8,
            Self::OnShelf => 9,
        }
    }

    pub const fn serialized_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ThirdPersonLeftHand => "thirdperson_lefthand",
            Self::ThirdPersonRightHand => "thirdperson_righthand",
            Self::FirstPersonLeftHand => "firstperson_lefthand",
            Self::FirstPersonRightHand => "firstperson_righthand",
            Self::Head => "head",
            Self::Gui => "gui",
            Self::Ground => "ground",
            Self::Fixed => "fixed",
            Self::OnShelf => "on_shelf",
        }
    }

    pub fn for_serialized_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|context| context.serialized_name() == name)
    }
}

/// `Display.TextDisplay.Align` (`Display.java:864-880`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Center,
    Left,
    Right,
}

impl TextAlign {
    pub const fn serialized_name(self) -> &'static str {
        match self {
            Self::Center => "center",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    pub fn for_serialized_name(name: &str) -> Option<Self> {
        [Self::Center, Self::Left, Self::Right]
            .into_iter()
            .find(|align| align.serialized_name() == name)
    }

    /// `TextDisplay.getAlign(byte)` (`Display.java:856-862`).
    pub const fn of_flags(flags: i8) -> Self {
        if flags & FLAG_ALIGN_LEFT != 0 {
            Self::Left
        } else if flags & FLAG_ALIGN_RIGHT != 0 {
            Self::Right
        } else {
            Self::Center
        }
    }
}

/// `net.minecraft.util.Brightness` (`Brightness.java:6-20`) over
/// `LightCoordsUtil.pack`/`block`/`sky` (`LightCoordsUtil.java:8-18`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Brightness {
    pub block: i32,
    pub sky: i32,
}

impl Brightness {
    pub const fn pack(self) -> i32 {
        self.block << 4 | self.sky << 20
    }

    pub const fn unpack(packed: i32) -> Self {
        Self {
            block: packed >> 4 & 15,
            sky: packed >> 20 & 15,
        }
    }
}

/// The `Display` synched rows (`Display.java:195-211`) as the server holds
/// them; the client-only fields (`:98-107`) are not here.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayData {
    transformation_interpolation_start_delta_ticks: i32,
    transformation_interpolation_duration: i32,
    pos_rot_interpolation_duration: i32,
    transformation: Transformation,
    billboard_constraints: BillboardConstraints,
    /// `DATA_BRIGHTNESS_OVERRIDE_ID`: the packed `Brightness`, or
    /// [`NO_BRIGHTNESS_OVERRIDE`].
    brightness_override: i32,
    view_range: f32,
    shadow_radius: f32,
    shadow_strength: f32,
    width: f32,
    height: f32,
    glow_color_override: i32,
}

impl Default for DisplayData {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayData {
    /// `Display.defineSynchedData` (`Display.java:195-211`).
    pub const fn new() -> Self {
        Self {
            transformation_interpolation_start_delta_ticks:
                INITIAL_TRANSFORMATION_START_INTERPOLATION,
            transformation_interpolation_duration: INITIAL_TRANSFORMATION_INTERPOLATION_DURATION,
            pos_rot_interpolation_duration: INITIAL_POS_ROT_INTERPOLATION_DURATION,
            transformation: Transformation::IDENTITY,
            billboard_constraints: BillboardConstraints::Fixed,
            brightness_override: NO_BRIGHTNESS_OVERRIDE,
            view_range: INITIAL_VIEW_RANGE,
            shadow_radius: INITIAL_SHADOW_RADIUS,
            shadow_strength: INITIAL_SHADOW_STRENGTH,
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
            glow_color_override: NO_GLOW_COLOR_OVERRIDE,
        }
    }

    pub const fn transformation_interpolation_start_delta_ticks(&self) -> i32 {
        self.transformation_interpolation_start_delta_ticks
    }

    pub const fn transformation_interpolation_duration(&self) -> i32 {
        self.transformation_interpolation_duration
    }

    pub const fn pos_rot_interpolation_duration(&self) -> i32 {
        self.pos_rot_interpolation_duration
    }

    pub const fn transformation(&self) -> Transformation {
        self.transformation
    }

    pub const fn billboard_constraints(&self) -> BillboardConstraints {
        self.billboard_constraints
    }

    /// `Display.getBrightnessOverride()` (`Display.java:310-313`).
    pub const fn brightness_override(&self) -> Option<Brightness> {
        if self.brightness_override != NO_BRIGHTNESS_OVERRIDE {
            Some(Brightness::unpack(self.brightness_override))
        } else {
            None
        }
    }

    pub const fn packed_brightness_override(&self) -> i32 {
        self.brightness_override
    }

    pub const fn view_range(&self) -> f32 {
        self.view_range
    }

    pub const fn shadow_radius(&self) -> f32 {
        self.shadow_radius
    }

    pub const fn shadow_strength(&self) -> f32 {
        self.shadow_strength
    }

    pub const fn width(&self) -> f32 {
        self.width
    }

    pub const fn height(&self) -> f32 {
        self.height
    }

    pub const fn glow_color_override(&self) -> i32 {
        self.glow_color_override
    }

    /// The saved-field setters (`Display.java:214-228`) as one builder, for
    /// a test or a caller constructing the entity's initial state.
    #[allow(clippy::too_many_arguments)]
    pub fn with_saved_fields(
        mut self,
        transformation: Transformation,
        transformation_interpolation_duration: i32,
        transformation_interpolation_start_delta_ticks: i32,
        pos_rot_interpolation_duration: i32,
        billboard_constraints: BillboardConstraints,
        view_range: f32,
        shadow_radius: f32,
        shadow_strength: f32,
        width: f32,
        height: f32,
        glow_color_override: i32,
        brightness_override: Option<Brightness>,
    ) -> Self {
        self.transformation = transformation;
        self.transformation_interpolation_duration = transformation_interpolation_duration;
        self.transformation_interpolation_start_delta_ticks =
            transformation_interpolation_start_delta_ticks;
        // `Mth.clamp(teleportDuration, 0, 59)` (`Display.java:218-219`).
        self.pos_rot_interpolation_duration = pos_rot_interpolation_duration.clamp(0, 59);
        self.billboard_constraints = billboard_constraints;
        self.view_range = view_range;
        self.shadow_radius = shadow_radius;
        self.shadow_strength = shadow_strength;
        self.width = width;
        self.height = height;
        self.glow_color_override = glow_color_override;
        self.brightness_override =
            brightness_override.map_or(NO_BRIGHTNESS_OVERRIDE, Brightness::pack);
        self
    }

    /// The fifteen `Display` rows in id order, each with `isSetToDefault`
    /// (`SynchedEntityData.java:96`) answered the way its value type's
    /// `equals` does.
    fn synched_rows(&self) -> Vec<SynchedValue> {
        let int = |data_id: u8, value: i32, default: i32| SynchedValue {
            value: EntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(value),
            },
            is_default: value == default,
        };
        let float_row = |data_id: u8, value: f32, default: f32| SynchedValue {
            value: EntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(value),
            },
            is_default: value.to_bits() == default.to_bits(),
        };
        let vector = |data_id: u8, value: Vector3f, default: Vector3f| SynchedValue {
            value: EntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID,
                value: value.data_value(),
            },
            is_default: value == default,
        };
        let quaternion = |data_id: u8, value: Quaternionf| SynchedValue {
            value: EntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID,
                value: value.data_value(),
            },
            is_default: value == Quaternionf::IDENTITY,
        };
        let transformation = self.transformation;
        vec![
            int(
                DISPLAY_TRANSFORMATION_INTERPOLATION_START_DELTA_TICKS_DATA_ID,
                self.transformation_interpolation_start_delta_ticks,
                INITIAL_TRANSFORMATION_START_INTERPOLATION,
            ),
            int(
                DISPLAY_TRANSFORMATION_INTERPOLATION_DURATION_DATA_ID,
                self.transformation_interpolation_duration,
                INITIAL_TRANSFORMATION_INTERPOLATION_DURATION,
            ),
            int(
                DISPLAY_POS_ROT_INTERPOLATION_DURATION_DATA_ID,
                self.pos_rot_interpolation_duration,
                INITIAL_POS_ROT_INTERPOLATION_DURATION,
            ),
            vector(
                DISPLAY_TRANSLATION_DATA_ID,
                transformation.translation,
                Vector3f::ZERO,
            ),
            vector(DISPLAY_SCALE_DATA_ID, transformation.scale, Vector3f::ONE),
            quaternion(DISPLAY_LEFT_ROTATION_DATA_ID, transformation.left_rotation),
            quaternion(
                DISPLAY_RIGHT_ROTATION_DATA_ID,
                transformation.right_rotation,
            ),
            SynchedValue {
                value: EntityDataValue {
                    data_id: DISPLAY_BILLBOARD_RENDER_CONSTRAINTS_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(self.billboard_constraints.id()),
                },
                is_default: self.billboard_constraints == BillboardConstraints::Fixed,
            },
            int(
                DISPLAY_BRIGHTNESS_OVERRIDE_DATA_ID,
                self.brightness_override,
                NO_BRIGHTNESS_OVERRIDE,
            ),
            float_row(
                DISPLAY_VIEW_RANGE_DATA_ID,
                self.view_range,
                INITIAL_VIEW_RANGE,
            ),
            float_row(
                DISPLAY_SHADOW_RADIUS_DATA_ID,
                self.shadow_radius,
                INITIAL_SHADOW_RADIUS,
            ),
            float_row(
                DISPLAY_SHADOW_STRENGTH_DATA_ID,
                self.shadow_strength,
                INITIAL_SHADOW_STRENGTH,
            ),
            float_row(DISPLAY_WIDTH_DATA_ID, self.width, INITIAL_WIDTH),
            float_row(DISPLAY_HEIGHT_DATA_ID, self.height, INITIAL_HEIGHT),
            int(
                DISPLAY_GLOW_COLOR_OVERRIDE_DATA_ID,
                self.glow_color_override,
                NO_GLOW_COLOR_OVERRIDE,
            ),
        ]
    }
}

/// `Display.BlockDisplay` (`Display.java:461-517`).
#[derive(Debug, Clone, PartialEq)]
pub struct BlockDisplayData {
    display: DisplayData,
    /// `DATA_BLOCK_STATE_ID` as the global block state id; the default is
    /// `Blocks.AIR.defaultBlockState()` (`:475`).
    block_state_id: i32,
}

impl Default for BlockDisplayData {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockDisplayData {
    pub fn new() -> Self {
        Self {
            display: DisplayData::new(),
            block_state_id: air_state_id(),
        }
    }

    pub fn with_display(mut self, display: DisplayData) -> Self {
        self.display = display;
        self
    }

    pub fn with_block_state(mut self, block_state_id: i32) -> Self {
        self.block_state_id = block_state_id;
        self
    }

    pub const fn display(&self) -> &DisplayData {
        &self.display
    }

    pub const fn block_state_id(&self) -> i32 {
        self.block_state_id
    }
}

/// `Display.ItemDisplay` (`Display.java:553-632`).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemDisplayData {
    display: DisplayData,
    /// `DATA_ITEM_STACK_ID`; `ItemStack.EMPTY` (`:570`) is the empty stack.
    item: RuntimeItemStack,
    item_display: ItemDisplayContext,
}

impl Default for ItemDisplayData {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemDisplayData {
    pub(crate) fn replace_slot_item(&mut self, item: RuntimeItemStack) {
        self.item = item;
    }

    pub fn new() -> Self {
        Self {
            display: DisplayData::new(),
            item: empty_stack().expect("an empty compound is an empty stack"),
            item_display: ItemDisplayContext::None,
        }
    }

    pub fn with_display(mut self, display: DisplayData) -> Self {
        self.display = display;
        self
    }

    pub fn with_item(mut self, item: RuntimeItemStack, item_display: ItemDisplayContext) -> Self {
        self.item = item;
        self.item_display = item_display;
        self
    }

    pub const fn display(&self) -> &DisplayData {
        &self.display
    }

    pub const fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    pub const fn item_display(&self) -> ItemDisplayContext {
        self.item_display
    }
}

/// `Display.TextDisplay` (`Display.java:658-895`).
#[derive(Debug, Clone, PartialEq)]
pub struct TextDisplayData {
    display: DisplayData,
    /// `DATA_TEXT_ID`, carried as the validated component the protocol type
    /// holds; `Component.empty()` (`:695`) is the empty literal.
    text: RichTextComponent,
    line_width: i32,
    background_color: i32,
    text_opacity: i8,
    flags: i8,
}

impl Default for TextDisplayData {
    fn default() -> Self {
        Self::new()
    }
}

impl TextDisplayData {
    /// `TextDisplay.defineSynchedData` (`Display.java:693-700`).
    pub fn new() -> Self {
        Self {
            display: DisplayData::new(),
            text: RichTextComponent::plain(""),
            line_width: INITIAL_LINE_WIDTH,
            background_color: INITIAL_BACKGROUND,
            text_opacity: INITIAL_TEXT_OPACITY,
            flags: 0,
        }
    }

    pub fn with_display(mut self, display: DisplayData) -> Self {
        self.display = display;
        self
    }

    pub fn with_text_fields(
        mut self,
        text: RichTextComponent,
        line_width: i32,
        background_color: i32,
        text_opacity: i8,
        flags: i8,
    ) -> Self {
        self.text = text;
        self.line_width = line_width;
        self.background_color = background_color;
        self.text_opacity = text_opacity;
        self.flags = flags;
        self
    }

    pub const fn display(&self) -> &DisplayData {
        &self.display
    }

    pub const fn text(&self) -> &RichTextComponent {
        &self.text
    }

    pub const fn line_width(&self) -> i32 {
        self.line_width
    }

    pub const fn background_color(&self) -> i32 {
        self.background_color
    }

    pub const fn text_opacity(&self) -> i8 {
        self.text_opacity
    }

    pub const fn flags(&self) -> i8 {
        self.flags
    }

    /// `TextDisplay.getAlign(getFlags())`.
    pub const fn align(&self) -> TextAlign {
        TextAlign::of_flags(self.flags)
    }
}

fn air_state_id() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID)
        .expect("the generated catalog carries minecraft:air")
}

/// `Component.empty().equals(text)`: the component codec values compare
/// equal, which is structural equality over contents, siblings and style.
fn text_is_empty_component(text: &RichTextComponent) -> bool {
    component_codec_values_equal(text.component(), &NbtSummaryValue::string(String::new()))
        .unwrap_or(false)
}

/// The `DisplayData` of any of the three kinds.
pub fn display(entity: &Entity) -> Option<&DisplayData> {
    match entity.data() {
        EntityData::BlockDisplay(data) => Some(&data.display),
        EntityData::ItemDisplay(data) => Some(&data.display),
        EntityData::TextDisplay(data) => Some(&data.display),
        _ => None,
    }
}

pub fn block_display(entity: &Entity) -> Option<&BlockDisplayData> {
    match entity.data() {
        EntityData::BlockDisplay(data) => Some(data),
        _ => None,
    }
}

pub fn item_display(entity: &Entity) -> Option<&ItemDisplayData> {
    match entity.data() {
        EntityData::ItemDisplay(data) => Some(data),
        _ => None,
    }
}

pub fn text_display(entity: &Entity) -> Option<&TextDisplayData> {
    match entity.data() {
        EntityData::TextDisplay(data) => Some(data),
        _ => None,
    }
}

/// `TextDisplay.setText` (`Display.java:714-716`).
pub(crate) fn set_text_display_text(entity: &mut Entity, text: RichTextComponent) {
    let changed = if let EntityData::TextDisplay(data) = &mut entity.data {
        if data.text != text {
            data.text = text;
            true
        } else {
            false
        }
    } else {
        false
    };
    if changed {
        entity.mark_synched_dirty(TEXT_DISPLAY_TEXT_DATA_ID);
    }
}

/// `new Display.BlockDisplay(EntityType.BLOCK_DISPLAY, level)`
/// (`Display.java:468-470` over `:109-113`) added to the level: the UUID draw
/// of the `Entity` constructor (`Entity.java:278`) and nothing else.
pub fn spawn_block_display(
    store: &mut EntityStore,
    position: Vec3d,
    data: BlockDisplayData,
) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
        position,
        EntityData::BlockDisplay(data),
    )
}

/// `new Display.ItemDisplay(EntityType.ITEM_DISPLAY, level)`
/// (`Display.java:563-565` over `:109-113`) added to the level.
pub fn spawn_item_display(
    store: &mut EntityStore,
    position: Vec3d,
    data: ItemDisplayData,
) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID,
        position,
        EntityData::ItemDisplay(data),
    )
}

/// `new Display.TextDisplay(EntityType.TEXT_DISPLAY, level)`
/// (`Display.java:688-690` over `:109-113`) added to the level.
pub fn spawn_text_display(
    store: &mut EntityStore,
    position: Vec3d,
    data: TextDisplayData,
) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID,
        position,
        EntityData::TextDisplay(data),
    )
}

/// `Display.tick()` (`Display.java:153-185`) on the server: the vehicle
/// check (`:154-157`) has nothing to act on because riding has no owner
/// here, and the rest is client side. Neither `baseTick` nor
/// `checkBelowWorld` runs.
pub fn tick(_entity: &mut Entity) {}

/// The `Entity` rows, the fifteen `Display` rows, then the kind's own rows,
/// in id order (`Display.java:195-211`, `:473-476`, `:567-572`,
/// `:693-700`).
pub fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    match entity.data() {
        EntityData::BlockDisplay(data) => {
            rows.extend(data.display.synched_rows());
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: BLOCK_DISPLAY_BLOCK_STATE_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID,
                    value: EntityDataValueKind::BlockState(data.block_state_id),
                },
                is_default: data.block_state_id == air_state_id(),
            });
        }
        EntityData::ItemDisplay(data) => {
            rows.extend(data.display.synched_rows());
            // A stack with no network projection is left out rather than sent
            // wrong, the same fail-closed answer the item entity gives.
            let stack = data.item.summary().clone();
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: ITEM_DISPLAY_ITEM_STACK_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                    value: EntityDataValueKind::ItemStack(stack),
                },
                is_default: data.item.is_empty(),
            });
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: ITEM_DISPLAY_ITEM_DISPLAY_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(data.item_display.id()),
                },
                is_default: data.item_display == ItemDisplayContext::None,
            });
        }
        EntityData::TextDisplay(data) => {
            rows.extend(data.display.synched_rows());
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: TEXT_DISPLAY_TEXT_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID,
                    value: EntityDataValueKind::Component(data.text.clone()),
                },
                is_default: text_is_empty_component(&data.text),
            });
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: TEXT_DISPLAY_LINE_WIDTH_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    value: EntityDataValueKind::Int(data.line_width),
                },
                is_default: data.line_width == INITIAL_LINE_WIDTH,
            });
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: TEXT_DISPLAY_BACKGROUND_COLOR_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    value: EntityDataValueKind::Int(data.background_color),
                },
                is_default: data.background_color == INITIAL_BACKGROUND,
            });
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: TEXT_DISPLAY_TEXT_OPACITY_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(data.text_opacity),
                },
                is_default: data.text_opacity == INITIAL_TEXT_OPACITY,
            });
            rows.push(SynchedValue {
                value: EntityDataValue {
                    data_id: TEXT_DISPLAY_STYLE_FLAGS_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(data.flags),
                },
                is_default: data.flags == 0,
            });
        }
        _ => {}
    }
    rows
}

/// What `readAdditionalSaveData` does beyond the field values on a freshly
/// constructed display: every `entityData.set` of a value away from its
/// `define`d default marks that row dirty (`SynchedEntityData.java:60-68`),
/// and `setTransformationInterpolationDelay` (`Display.java:282-284`) marks
/// its row dirty unconditionally.
pub fn finish_load(entity: &mut Entity) {
    if display(entity).is_none() {
        return;
    }
    // Only the dirty marks are wanted here, and the save reader this runs
    // inside is shared with the world's entity restore, which holds no level
    // and so no publication. A fresh entity's dirty set is read back by
    // nothing: pairing builds its rows from every non-default value and the
    // command-data overlay compares rows itself. The item-stack row is
    // therefore compared resolver-free.
    let dirty: Vec<u8> = synched_values(entity)
        .into_iter()
        .filter(|row| {
            row.value.data_id >= DISPLAY_TRANSFORMATION_INTERPOLATION_START_DELTA_TICKS_DATA_ID
                && !row.is_default
        })
        .map(|row| row.value.data_id)
        .collect();
    for data_id in dirty {
        entity.mark_synched_dirty(data_id);
    }
    entity.mark_synched_dirty(DISPLAY_TRANSFORMATION_INTERPOLATION_START_DELTA_TICKS_DATA_ID);
}

/// `Display.setTransformationInterpolationDelay` (`Display.java:282-284`)
/// unconditionally marks the start-delta row dirty on `/data` reloads even
/// when the value stays `0`.
pub(crate) fn mark_command_load_dirty(entity: &mut Entity) {
    if display(entity).is_some() {
        entity.mark_synched_dirty(DISPLAY_TRANSFORMATION_INTERPOLATION_START_DELTA_TICKS_DATA_ID);
    }
}

/// `Transformation.CODEC` (`Transformation.java:24-32`) written: the four
/// float lists of `ExtraCodecs.VECTOR3F` (`ExtraCodecs.java:94-98`) and
/// `QUATERNIONF_COMPONENTS` (`:110-114`). `EXTENDED_CODEC` encodes through
/// this first alternative (`:33-35`).
fn transformation_record(transformation: Transformation) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry(
            "translation",
            float_list(&transformation.translation.components()),
        ),
        entry(
            "left_rotation",
            float_list(&transformation.left_rotation.components()),
        ),
        entry("scale", float_list(&transformation.scale.components())),
        entry(
            "right_rotation",
            float_list(&transformation.right_rotation.components()),
        ),
    ])
}

/// `ExtraCodecs.QUATERNIONF` (`ExtraCodecs.java:110-120`) read: the four-float
/// list normalized, or the `AXISANGLE4F` record through JOML
/// `Quaternionf(AxisAngle4f)`.
fn read_quaternion(value: &NbtSummaryValue) -> Option<Quaternionf> {
    if let Some(components) = fixed_float_list(value, 4) {
        let [x, y, z, w] = components[..] else {
            unreachable!("fixed_float_list answered four floats")
        };
        return Some(Quaternionf::new(x, y, z, w).normalized());
    }
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let angle = numeric_float(optional_field(entries, "angle")?)?;
    let axis = fixed_float_list(optional_field(entries, "axis")?, 3)?;
    let tau = std::f32::consts::TAU;
    let angle = (if angle < 0.0 {
        tau + angle % tau
    } else {
        angle
    }) % tau;
    let (sin, cos) = (angle * 0.5).sin_cos();
    Some(Quaternionf::new(
        axis[0] * sin,
        axis[1] * sin,
        axis[2] * sin,
        cos,
    ))
}

/// `Transformation.EXTENDED_CODEC` (`Transformation.java:24-35`) read as
/// `Display.readAdditionalSaveData` uses it (`Display.java:215`): either the
/// decomposed record form or the 16-float row-major `MATRIX4F` alternative
/// decomposed via `Transformation.ensureDecomposed` / `MatrixUtil.svdDecompose`,
/// falling back to `IDENTITY` when neither decodes.
pub(crate) fn read_transformation(entries: &[NbtSummaryEntry]) -> Transformation {
    read_transformation_record(entries)
        .or_else(|| read_transformation_matrix(entries))
        .unwrap_or(Transformation::IDENTITY)
}

fn read_transformation_matrix(entries: &[NbtSummaryEntry]) -> Option<Transformation> {
    let values = fixed_float_list(optional_field(entries, "transformation")?, 16)?;
    if !values.iter().all(|value| value.is_finite()) {
        return None;
    }
    decompose_matrix4f(&values)
}

/// Decomposes a row-major 4x4 affine matrix (`ExtraCodecs.MATRIX4F`,
/// `Transformation.ensureDecomposed`, `MatrixUtil.svdDecompose`).
fn decompose_matrix4f(m: &[f32]) -> Option<Transformation> {
    let m33 = m[15];
    if m33 == 0.0 || !m33.is_finite() {
        return None;
    }
    let scale_factor = 1.0_f32 / m33;
    let translation = Vector3f::new(
        m[3] * scale_factor,
        m[7] * scale_factor,
        m[11] * scale_factor,
    );
    // Column-major 3x3: cols[col][row] = m[row * 4 + col] * scale_factor.
    let linear = Mat3([
        [
            m[0] * scale_factor,
            m[4] * scale_factor,
            m[8] * scale_factor,
        ],
        [
            m[1] * scale_factor,
            m[5] * scale_factor,
            m[9] * scale_factor,
        ],
        [
            m[2] * scale_factor,
            m[6] * scale_factor,
            m[10] * scale_factor,
        ],
    ]);
    let (left_rotation, scale, right_rotation) = svd_decompose(linear);
    let all_finite = translation.x.is_finite()
        && translation.y.is_finite()
        && translation.z.is_finite()
        && scale.x.is_finite()
        && scale.y.is_finite()
        && scale.z.is_finite()
        && left_rotation.x.is_finite()
        && left_rotation.y.is_finite()
        && left_rotation.z.is_finite()
        && left_rotation.w.is_finite()
        && right_rotation.x.is_finite()
        && right_rotation.y.is_finite()
        && right_rotation.z.is_finite()
        && right_rotation.w.is_finite();
    all_finite.then_some(Transformation {
        translation,
        left_rotation,
        scale,
        right_rotation,
    })
}

/// Column-major 3x3 matrix matching JOML `Matrix3f` layout (`cols[col][row]`).
#[derive(Clone, Copy)]
struct Mat3([[f32; 3]; 3]);

impl Mat3 {
    fn transpose(self) -> Self {
        let a = self.0;
        Self([
            [a[0][0], a[1][0], a[2][0]],
            [a[0][1], a[1][1], a[2][1]],
            [a[0][2], a[1][2], a[2][2]],
        ])
    }

    fn mul(self, rhs: Self) -> Self {
        let mut out = [[0.0_f32; 3]; 3];
        for (out_col, rhs_col) in out.iter_mut().zip(rhs.0.iter()) {
            for (row, cell) in out_col.iter_mut().enumerate() {
                *cell = self.0[0][row] * rhs_col[0]
                    + self.0[1][row] * rhs_col[1]
                    + self.0[2][row] * rhs_col[2];
            }
        }
        Self(out)
    }

    fn from_quat(q: Quaternionf) -> Self {
        let (x, y, z, w) = (q.x, q.y, q.z, q.w);
        let (xx, yy, zz) = (x * x, y * y, z * z);
        let (xy, xz, yz) = (x * y, x * z, y * z);
        let (wx, wy, wz) = (w * x, w * y, w * z);
        Self([
            [1.0 - 2.0 * (yy + zz), 2.0 * (xy + wz), 2.0 * (xz - wy)],
            [2.0 * (xy - wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz + wx)],
            [2.0 * (xz + wy), 2.0 * (yz - wx), 1.0 - 2.0 * (xx + yy)],
        ])
    }

    fn similarity_transform(&mut self, q: Self) {
        *self = q.transpose().mul(self.mul(q));
    }
}

#[derive(Clone, Copy)]
struct Givens {
    sin_half: f32,
    cos_half: f32,
}

impl Givens {
    fn from_unnormalized(sin_half: f32, cos_half: f32) -> Self {
        let inv = 1.0 / (sin_half * sin_half + cos_half * cos_half).sqrt();
        Self {
            sin_half: inv * sin_half,
            cos_half: inv * cos_half,
        }
    }

    fn pi_4() -> Self {
        let (sin_half, cos_half) = (std::f32::consts::FRAC_PI_4 * 0.5).sin_cos();
        Self { sin_half, cos_half }
    }

    fn inverse(self) -> Self {
        Self {
            sin_half: -self.sin_half,
            cos_half: self.cos_half,
        }
    }

    fn cos(self) -> f32 {
        self.cos_half * self.cos_half - self.sin_half * self.sin_half
    }

    fn sin(self) -> f32 {
        2.0 * self.sin_half * self.cos_half
    }

    fn quat_x(self) -> Quaternionf {
        Quaternionf::new(self.sin_half, 0.0, 0.0, self.cos_half)
    }

    fn quat_y(self) -> Quaternionf {
        Quaternionf::new(0.0, self.sin_half, 0.0, self.cos_half)
    }

    fn quat_z(self) -> Quaternionf {
        Quaternionf::new(0.0, 0.0, self.sin_half, self.cos_half)
    }

    fn mat_x(self) -> Mat3 {
        let (c, s) = (self.cos(), self.sin());
        Mat3([[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]])
    }

    fn mat_y(self) -> Mat3 {
        let (c, s) = (self.cos(), self.sin());
        Mat3([[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]])
    }

    fn mat_z(self) -> Mat3 {
        let (c, s) = (self.cos(), self.sin());
        Mat3([[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]])
    }
}

fn quat_mul(a: Quaternionf, b: Quaternionf) -> Quaternionf {
    Quaternionf::new(
        a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
    )
}

fn quat_normalize(q: Quaternionf) -> Quaternionf {
    let norm = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
    if norm > 0.0 {
        Quaternionf::new(q.x / norm, q.y / norm, q.z / norm, q.w / norm)
    } else {
        Quaternionf::IDENTITY
    }
}

fn approx_givens_quat(a11: f32, a12: f32, a22: f32) -> Givens {
    let g = 3.0_f32 + 2.0_f32 * 2.0_f32.sqrt();
    let ch = 2.0 * (a11 - a22);
    let sh = a12;
    if g * sh * sh < ch * ch {
        Givens::from_unnormalized(sh, ch)
    } else {
        Givens::pi_4()
    }
}

fn qr_givens_quat(a1: f32, a2: f32) -> Givens {
    let p = a1.hypot(a2);
    let mut sh = if p > 1.0e-6 { a2 } else { 0.0 };
    let mut ch = a1.abs() + p.max(1.0e-6);
    if a1 < 0.0 {
        std::mem::swap(&mut sh, &mut ch);
    }
    Givens::from_unnormalized(sh, ch)
}

fn svd_decompose(matrix: Mat3) -> (Quaternionf, Vector3f, Quaternionf) {
    let mut b = matrix.transpose().mul(matrix);
    let mut v = Quaternionf::IDENTITY;
    for _ in 0..5 {
        if b.0[0][1] * b.0[0][1] + b.0[1][0] * b.0[1][0] > 1.0e-6 {
            let p = approx_givens_quat(b.0[0][0], 0.5 * (b.0[0][1] + b.0[1][0]), b.0[1][1]);
            v = quat_mul(v, p.quat_z());
            b.similarity_transform(p.mat_z());
        }
        if b.0[0][2] * b.0[0][2] + b.0[2][0] * b.0[2][0] > 1.0e-6 {
            let p =
                approx_givens_quat(b.0[0][0], 0.5 * (b.0[0][2] + b.0[2][0]), b.0[2][2]).inverse();
            v = quat_mul(v, p.quat_y());
            b.similarity_transform(p.mat_y());
        }
        if b.0[1][2] * b.0[1][2] + b.0[2][1] * b.0[2][1] > 1.0e-6 {
            let p = approx_givens_quat(b.0[1][1], 0.5 * (b.0[1][2] + b.0[2][1]), b.0[2][2]);
            v = quat_mul(v, p.quat_x());
            b.similarity_transform(p.mat_x());
        }
    }
    v = quat_normalize(v);
    let zero_col0 = b.0[0][0] < 1.0e-6;
    let zero_col1 = b.0[1][1] < 1.0e-6;
    let u012s = matrix.mul(Mat3::from_quat(v));
    let mut u = Quaternionf::IDENTITY;
    let p0 = if zero_col0 {
        qr_givens_quat(u012s.0[1][1], -u012s.0[1][0])
    } else {
        qr_givens_quat(u012s.0[0][0], u012s.0[0][1])
    };
    u = quat_mul(u, p0.quat_z());
    let u12s = p0.mat_z().transpose().mul(u012s);
    let p1 = if zero_col0 {
        qr_givens_quat(u12s.0[2][2], -u12s.0[2][0])
    } else {
        qr_givens_quat(u12s.0[0][0], u12s.0[0][2])
    }
    .inverse();
    u = quat_mul(u, p1.quat_y());
    let u2s = p1.mat_y().transpose().mul(u12s);
    let p2 = if zero_col1 {
        qr_givens_quat(u2s.0[2][2], -u2s.0[2][1])
    } else {
        qr_givens_quat(u2s.0[1][1], u2s.0[1][2])
    };
    u = quat_mul(u, p2.quat_x());
    let s = p2.mat_x().transpose().mul(u2s);
    let clean = |v: f32| if v == 0.0 { 0.0 } else { v };
    let clean_q = |q: Quaternionf| Quaternionf::new(clean(q.x), clean(q.y), clean(q.z), clean(q.w));
    let scale = Vector3f::new(clean(s.0[0][0]), clean(s.0[1][1]), clean(s.0[2][2]));
    let right = clean_q(Quaternionf::new(-v.x, -v.y, -v.z, v.w));
    (clean_q(quat_normalize(u)), scale, right)
}

fn read_transformation_record(entries: &[NbtSummaryEntry]) -> Option<Transformation> {
    let NbtSummaryValue::Compound(fields) = optional_field(entries, "transformation")? else {
        return None;
    };
    let vector = |name: &str| fixed_float_list(optional_field(fields, name)?, 3);
    let (translation, scale) = (vector("translation")?, vector("scale")?);
    Some(Transformation {
        translation: Vector3f::new(translation[0], translation[1], translation[2]),
        left_rotation: read_quaternion(optional_field(fields, "left_rotation")?)?,
        scale: Vector3f::new(scale[0], scale[1], scale[2]),
        right_rotation: read_quaternion(optional_field(fields, "right_rotation")?)?,
    })
}

/// `Brightness.CODEC` (`Brightness.java:7-11`) read: both fields required
/// and in `0..=15`, else the record does not decode.
pub(crate) fn read_brightness(value: &NbtSummaryValue) -> Option<Brightness> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let light = |name: &str| {
        let value = numeric_int(optional_field(entries, name)?)?;
        (0..=15).contains(&value).then_some(value)
    };
    Some(Brightness {
        block: light("block")?,
        sky: light("sky")?,
    })
}

/// `Display.addAdditionalSaveData` (`Display.java:238-250`).
fn write_display(entries: &mut Vec<NbtSummaryEntry>, data: &DisplayData) {
    entries.push(entry(
        "transformation",
        transformation_record(data.transformation),
    ));
    entries.push(entry(
        "billboard",
        NbtSummaryValue::string(data.billboard_constraints.serialized_name()),
    ));
    entries.push(entry(
        "interpolation_duration",
        NbtSummaryValue::Int(data.transformation_interpolation_duration),
    ));
    entries.push(entry(
        "teleport_duration",
        NbtSummaryValue::Int(data.pos_rot_interpolation_duration),
    ));
    for (name, value) in [
        ("view_range", data.view_range),
        ("shadow_radius", data.shadow_radius),
        ("shadow_strength", data.shadow_strength),
        ("width", data.width),
        ("height", data.height),
    ] {
        entries.push(entry(
            name,
            NbtSummaryValue::Float(bbb_protocol::nbt_summary::canonical_float_tag_bits(value)),
        ));
    }
    entries.push(entry(
        "glow_color_override",
        NbtSummaryValue::Int(data.glow_color_override),
    ));
    if let Some(brightness) = data.brightness_override() {
        entries.push(entry(
            "brightness",
            NbtSummaryValue::Compound(vec![
                entry("block", NbtSummaryValue::Int(brightness.block)),
                entry("sky", NbtSummaryValue::Int(brightness.sky)),
            ]),
        ));
    }
}

/// `Display.readAdditionalSaveData` (`Display.java:214-228`) as the field
/// values it leaves; [`finish_load`] is the rest of it.
pub(crate) fn read_display(entries: &[NbtSummaryEntry]) -> DisplayData {
    let billboard = match optional_field(entries, "billboard") {
        Some(NbtSummaryValue::String(name)) => name
            .with_utf8(BillboardConstraints::for_serialized_name)
            .flatten(),
        _ => None,
    };
    DisplayData::new().with_saved_fields(
        read_transformation(entries),
        int_or(
            entries,
            "interpolation_duration",
            INITIAL_TRANSFORMATION_INTERPOLATION_DURATION,
        ),
        int_or(
            entries,
            "start_interpolation",
            INITIAL_TRANSFORMATION_START_INTERPOLATION,
        ),
        int_or(
            entries,
            "teleport_duration",
            INITIAL_POS_ROT_INTERPOLATION_DURATION,
        ),
        billboard.unwrap_or(BillboardConstraints::Fixed),
        float_or(entries, "view_range", INITIAL_VIEW_RANGE),
        float_or(entries, "shadow_radius", INITIAL_SHADOW_RADIUS),
        float_or(entries, "shadow_strength", INITIAL_SHADOW_STRENGTH),
        float_or(entries, "width", INITIAL_WIDTH),
        float_or(entries, "height", INITIAL_HEIGHT),
        int_or(entries, "glow_color_override", NO_GLOW_COLOR_OVERRIDE),
        optional_field(entries, "brightness").and_then(read_brightness),
    )
}

/// `BlockState.CODEC` (`BlockState.java:9`) written: the block name and,
/// for a block with properties, every property as a string.
fn block_state_record(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<NbtSummaryValue, EntitySaveError> {
    crate::entity::save::block_state_value_with_registry(
        block_state_id,
        "block_state",
        content_registry,
    )
}

/// `BlockDisplay.addAdditionalSaveData` (`Display.java:501-504`) after the
/// shared fields.
pub fn write_block_display(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &BlockDisplayData,
) -> Result<(), EntitySaveError> {
    write_block_display_with_registry(
        entries,
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
    )
}

pub(crate) fn write_block_display_with_registry(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &BlockDisplayData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<(), EntitySaveError> {
    write_display(entries, &data.display);
    entries.push(entry(
        "block_state",
        block_state_record(data.block_state_id, content_registry)?,
    ));
    Ok(())
}

/// `BlockDisplay.readAdditionalSaveData` (`Display.java:495-498`): an
/// undecodable state is air.
pub fn read_block_display(entries: &[NbtSummaryEntry]) -> BlockDisplayData {
    read_block_display_with_registry(
        entries,
        bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
    )
}

pub(crate) fn read_block_display_with_registry(
    entries: &[NbtSummaryEntry],
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> BlockDisplayData {
    let block_state_id = optional_field(entries, "block_state")
        .and_then(|value| {
            crate::entity::save::read_block_state_from_value_with_registry(value, content_registry)
        })
        .unwrap_or_else(air_state_id);
    BlockDisplayData {
        display: read_display(entries),
        block_state_id,
    }
}

/// `ItemDisplay.addAdditionalSaveData` (`Display.java:606-614`) after the
/// shared fields.
pub fn write_item_display(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &ItemDisplayData,
    items: crate::entity::item::ItemResolver<'_>,
) -> Result<(), EntitySaveError> {
    write_display(entries, &data.display);
    if !data.item.is_empty() {
        entries.push(entry(
            "item",
            crate::entity::item::write_saved_stack(&data.item, items),
        ));
    }
    entries.push(entry(
        "item_display",
        NbtSummaryValue::string(data.item_display.serialized_name()),
    ));
    Ok(())
}

/// `ItemDisplay.readAdditionalSaveData` (`Display.java:599-603`): a stack
/// that does not decode is `ItemStack.EMPTY`, a context that does not is
/// `NONE`.
pub fn read_item_display(
    entries: &[NbtSummaryEntry],
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> ItemDisplayData {
    let item = optional_field(entries, "item")
        .and_then(|value| read_saved_stack(value, items))
        .filter(|stack| !stack.is_empty())
        .unwrap_or_else(|| empty_stack().expect("an empty compound is an empty stack"));
    let item_display = match optional_field(entries, "item_display") {
        Some(NbtSummaryValue::String(name)) => name
            .with_utf8(ItemDisplayContext::for_serialized_name)
            .flatten(),
        _ => None,
    };
    ItemDisplayData {
        display: read_display(entries),
        item,
        item_display: item_display.unwrap_or(ItemDisplayContext::None),
    }
}

/// `TextDisplay.addAdditionalSaveData` (`Display.java:794-805`) after the
/// shared fields. The component is written as the validated NBT it was
/// loaded with, which is the canonical `ComponentSerialization.CODEC` form
/// for every component an official save produces.
pub fn write_text_display(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &TextDisplayData,
) -> Result<(), EntitySaveError> {
    write_display(entries, &data.display);
    entries.push(entry("text", data.text.component().clone()));
    entries.push(entry("line_width", NbtSummaryValue::Int(data.line_width)));
    entries.push(entry(
        "background",
        NbtSummaryValue::Int(data.background_color),
    ));
    entries.push(entry(
        "text_opacity",
        NbtSummaryValue::Byte(data.text_opacity),
    ));
    // `storeFlag` (`Display.java:789-791`) for the three flag bits.
    entries.push(entry("shadow", byte_bool(data.flags & FLAG_SHADOW != 0)));
    entries.push(entry(
        "see_through",
        byte_bool(data.flags & FLAG_SEE_THROUGH != 0),
    ));
    entries.push(entry(
        "default_background",
        byte_bool(data.flags & FLAG_USE_DEFAULT_BACKGROUND != 0),
    ));
    entries.push(entry(
        "alignment",
        NbtSummaryValue::string(data.align().serialized_name()),
    ));
    Ok(())
}

/// `TextDisplay.readAdditionalSaveData` (`Display.java:755-787`) as the
/// field values it leaves; [`finish_load`] is the rest of it. The world
/// resolves the text against the display once it is in its level.
pub fn read_text_display(entries: &[NbtSummaryEntry]) -> TextDisplayData {
    read_text_display_fields(entries).with_display(read_display(entries))
}

fn read_text_display_fields(entries: &[NbtSummaryEntry]) -> TextDisplayData {
    // `loadFlag` (`Display.java:750-752`) for the three flag bits, then the
    // alignment (`:763-770`).
    let mut flags = 0_i8;
    for (name, mask) in [
        ("shadow", FLAG_SHADOW),
        ("see_through", FLAG_SEE_THROUGH),
        ("default_background", FLAG_USE_DEFAULT_BACKGROUND),
    ] {
        if bool_or(entries, name, false) {
            flags |= mask;
        }
    }
    let alignment = match optional_field(entries, "alignment") {
        Some(NbtSummaryValue::String(name)) => {
            name.with_utf8(TextAlign::for_serialized_name).flatten()
        }
        _ => None,
    };
    flags |= match alignment {
        Some(TextAlign::Left) => FLAG_ALIGN_LEFT,
        Some(TextAlign::Right) => FLAG_ALIGN_RIGHT,
        Some(TextAlign::Center) | None => 0,
    };
    // `ComponentSerialization.CODEC` failing to decode leaves the default.
    let text = optional_field(entries, "text")
        .and_then(|value| RichTextComponent::try_from_component_nbt(value.clone()).ok())
        .unwrap_or_else(|| RichTextComponent::plain(""));
    TextDisplayData {
        display: DisplayData::new(),
        text,
        line_width: int_or(entries, "line_width", INITIAL_LINE_WIDTH),
        background_color: int_or(entries, "background", INITIAL_BACKGROUND),
        text_opacity: byte_or(entries, "text_opacity", INITIAL_TEXT_OPACITY),
        flags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_transformation_decodes_16_float_matrix4f_list() {
        // Translation (2, -3, 4), 90 deg rotation around Y, scale (2, 3, 4).
        // Column-major R_y(90)*S: col0=(0,0,-2), col1=(0,3,0), col2=(4,0,0), col3=(2,-3,4)
        // Row-major 16-float list:
        let row_major = [
            0.0_f32, 0.0, 4.0, 2.0, 0.0, 3.0, 0.0, -3.0, -2.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let entries = vec![entry(
            "transformation",
            NbtSummaryValue::List(
                row_major
                    .iter()
                    .map(|v| NbtSummaryValue::Float(v.to_bits()))
                    .collect(),
            ),
        )];
        let decoded = read_transformation(&entries);
        assert!((decoded.translation.x - 2.0).abs() < 1e-4);
        assert!((decoded.translation.y - (-3.0)).abs() < 1e-4);
        assert!((decoded.translation.z - 4.0).abs() < 1e-4);
        // Recompose R(left) * diag(scale) * R(right) and compare against linear 3x3.
        let s = Mat3([
            [decoded.scale.x, 0.0, 0.0],
            [0.0, decoded.scale.y, 0.0],
            [0.0, 0.0, decoded.scale.z],
        ]);
        let recomposed = Mat3::from_quat(decoded.left_rotation)
            .mul(s)
            .mul(Mat3::from_quat(decoded.right_rotation));
        let expected = [[0.0_f32, 0.0, -2.0], [0.0, 3.0, 0.0], [4.0, 0.0, 0.0]];
        for (col, (got_col, exp_col)) in recomposed.0.iter().zip(expected.iter()).enumerate() {
            for (row, (got, exp)) in got_col.iter().zip(exp_col.iter()).enumerate() {
                assert!(
                    (got - exp).abs() < 1e-3,
                    "mismatch at ({col},{row}): got {got}, expected {exp}",
                );
            }
        }
    }
}
