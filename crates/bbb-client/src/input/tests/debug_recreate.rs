use super::*;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_ITEM_ID};
use bbb_protocol::item_types::{
    VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_DIAMOND_SWORD_ID, VANILLA_ITEM_IRON_HELMET_ID,
};

fn add_entity(world: &mut WorldStore, id: i32, entity_type_id: i32, position: [f64; 3]) {
    world.apply_add_entity(AddEntity {
        id,
        uuid: Uuid::from_u128(id as u128),
        entity_type_id,
        position: ProtocolVec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        delta_movement: ProtocolVec3d {
            x: 0.25,
            y: -0.5,
            z: 0.75,
        },
        x_rot: 10.0,
        y_rot: 45.0,
        y_head_rot: 45.0,
        data: 0,
    });
}

fn synched(
    entity_type_id: i32,
    java_name: &str,
    value: EntityDataValueKind,
) -> ProtocolEntityDataValue {
    let accessor = bbb_client_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity_type_id,
        java_name,
    )
    .unwrap_or_else(|| panic!("missing {java_name} accessor"));
    ProtocolEntityDataValue {
        data_id: accessor.data_id,
        serializer_id: accessor.serializer_id,
        value,
    }
}

fn item(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

/// Shift+F3+I with gamemaster permission, looking from the origin along +Z.
fn shift_f3_i_client_copy(world: &mut WorldStore) -> Option<String> {
    grant_debug_recreate_nbt_permission(world);
    world.set_local_player_pose(LocalPlayerPoseState::default());
    let mut input = ClientInputState::new(true);
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);
    for key in [KeyCode::F3, KeyCode::KeyI] {
        assert!(input.handle_debug_overlay_key_with_clipboard(
            PhysicalKey::Code(key),
            ElementState::Pressed,
            Some(world),
            None,
            Some(&mut clipboard)
        ));
    }
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    assert_eq!(
        world
            .client_chat()
            .messages
            .last()
            .map(|message| message.content.as_str()),
        Some("[Debug]: Copied client-side entity data to clipboard")
    );
    clipboard.text
}

#[test]
fn shift_f3_i_copies_named_mob_with_received_data_and_equipment() {
    let mut world = world_with_debug_player(false);
    // Raised so the short baby zombie meets the crosshair at eye height.
    add_entity(
        &mut world,
        50,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [0.0, 1.0, 3.0],
    );
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            synched(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                "DATA_CUSTOM_NAME",
                EntityDataValueKind::OptionalComponent(Some(RichTextComponent::plain(
                    "Bob \"Prime\"",
                ))),
            ),
            synched(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                "DATA_CUSTOM_NAME_VISIBLE",
                EntityDataValueKind::Boolean(true),
            ),
            synched(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                "DATA_HEALTH_ID",
                EntityDataValueKind::Float(15.0),
            ),
            synched(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                "DATA_BABY_ID",
                EntityDataValueKind::Boolean(true),
            ),
        ],
    }));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 50,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: item(VANILLA_ITEM_DIAMOND_SWORD_ID, 1),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: item(VANILLA_ITEM_IRON_HELMET_ID, 1),
            },
        ],
    }));

    assert_eq!(
        shift_f3_i_client_copy(&mut world).as_deref(),
        Some(
            "/summon minecraft:zombie 0.00 1.00 3.00 \
             {Motion: [0.25d, -0.5d, 0.75d], Rotation: [45.0f, 10.0f], \
             CustomName: 'Bob \"Prime\"', CustomNameVisible: 1b, Health: 15.0f, IsBaby: 1b, \
             equipment: {mainhand: {id: \"minecraft:diamond_sword\", count: 1}, \
             head: {id: \"minecraft:iron_helmet\", count: 1}}}"
        )
    );
}

#[test]
fn shift_f3_i_copies_another_player_with_received_data() {
    let mut world = world_with_debug_player(false);
    add_entity(
        &mut world,
        51,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [0.0, 0.0, 3.0],
    );
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![
            synched(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                "DATA_HEALTH_ID",
                EntityDataValueKind::Float(20.0),
            ),
            synched(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                "DATA_PLAYER_ABSORPTION_ID",
                EntityDataValueKind::Float(6.0),
            ),
            synched(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                "DATA_SCORE_ID",
                EntityDataValueKind::Int(7),
            ),
        ],
    }));

    assert_eq!(
        shift_f3_i_client_copy(&mut world).as_deref(),
        Some(
            "/summon minecraft:player 0.00 0.00 3.00 \
             {Motion: [0.25d, -0.5d, 0.75d], Rotation: [45.0f, 10.0f], \
             Health: 20.0f, AbsorptionAmount: 6.0f, Score: 7}"
        )
    );
}

#[test]
fn client_copy_of_item_entity_writes_the_received_stack() {
    // Item entities are not pickable, so this goes through the serializer directly.
    let mut world = world_with_debug_player(false);
    add_entity(
        &mut world,
        52,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        [1.0, 64.0, -2.5],
    );
    let mut stack = item(VANILLA_ITEM_APPLE_ID, 3);
    stack.component_patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID],
        custom_name: Some("Lunch".to_string()),
        custom_name_styled: Some(vec![StyledTextRun {
            text: "Lunch".to_string(),
            style: ComponentStyle::default(),
        }]),
        custom_name_nbt: Some(bbb_protocol::NbtSummaryValue::string("Lunch")),
        ..DataComponentPatchSummary::default()
    }
    .into();
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 52,
        values: vec![synched(
            VANILLA_ENTITY_TYPE_ITEM_ID,
            "DATA_ITEM",
            EntityDataValueKind::ItemStack(stack),
        )],
    }));

    let copy = super::super::debug_recreate::client_entity_command(&world, 52, true)
        .expect("item entity copy");
    assert_eq!(
        copy.command,
        "/summon minecraft:item 1.00 64.00 -2.50 \
         {Motion: [0.25d, -0.5d, 0.75d], Rotation: [45.0f, 10.0f], \
         Item: {id: \"minecraft:apple\", count: 3, components: {\"minecraft:custom_name\": \"Lunch\"}}}"
    );
    let copy = super::super::debug_recreate::client_entity_command(&world, 52, false)
        .expect("item entity copy without permission");
    assert_eq!(copy.command, "/summon minecraft:item 1.00 64.00 -2.50");
}

#[test]
fn client_copy_of_vehicle_writes_passengers_with_their_pos_and_uuid() {
    let mut world = world_with_debug_player(false);
    add_entity(
        &mut world,
        53,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [2.0, 64.0, -2.0],
    );
    add_entity(
        &mut world,
        54,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [2.5, 64.5, -2.5],
    );
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 54,
        values: vec![synched(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            "DATA_BABY_ID",
            EntityDataValueKind::Boolean(true),
        )],
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 53,
        passenger_ids: vec![54],
    }));

    let copy = super::super::debug_recreate::client_entity_command(&world, 53, true)
        .expect("vehicle copy");
    assert_eq!(
        copy.command,
        "/summon minecraft:chicken 2.00 64.00 -2.00 \
         {Motion: [0.25d, -0.5d, 0.75d], Rotation: [45.0f, 10.0f], \
         Passengers: [{id: \"minecraft:zombie\", Pos: [2.0d, 64.5d, -2.0d], \
         Motion: [0.25d, -0.5d, 0.75d], Rotation: [45.0f, 10.0f], \
         UUID: [I; 0, 0, 0, 54], IsBaby: 1b}]}"
    );
}

#[test]
fn shift_f3_i_copies_mannequin_profile_layers_pose_and_hidden_description() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID as MANNEQUIN;
    use bbb_protocol::packets::{
        PlayerSkinPatchData, ResolvableProfileData, ResolvableProfileKind,
    };
    let mut world = world_with_debug_player(false);
    // Raised so the crouching (1.5 high) mannequin meets the crosshair at eye height.
    add_entity(&mut world, 53, MANNEQUIN, [0.0, 0.5, 3.0]);
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 53,
        values: vec![
            synched(
                MANNEQUIN,
                "DATA_PROFILE",
                EntityDataValueKind::ResolvableProfile(ResolvableProfileData {
                    profile: ResolvableProfileKind::Partial {
                        name: Some("Steve".to_string()),
                        id: None,
                        properties: Vec::new(),
                    },
                    skin_patch: PlayerSkinPatchData {
                        body: None,
                        cape: None,
                        elytra: None,
                        slim_model: Some(true),
                    },
                }),
            ),
            synched(
                MANNEQUIN,
                "DATA_PLAYER_MODE_CUSTOMISATION",
                EntityDataValueKind::Byte(0x7F & !0x01 & !0x40),
            ),
            synched(
                MANNEQUIN,
                "DATA_POSE",
                EntityDataValueKind::Pose(
                    bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
                ),
            ),
            synched(
                MANNEQUIN,
                "DATA_DESCRIPTION",
                EntityDataValueKind::OptionalComponent(None),
            ),
        ],
    }));

    assert_eq!(
        shift_f3_i_client_copy(&mut world).as_deref(),
        Some(
            "/summon minecraft:mannequin 0.00 0.50 3.00 \
             {Motion: [0.25d, -0.5d, 0.75d], Rotation: [45.0f, 10.0f], \
             pose: \"crouching\", hidden_layers: [\"cape\", \"hat\"], \
             profile: {name: \"Steve\", model: \"slim\"}, hide_description: 1b}"
        )
    );
}
