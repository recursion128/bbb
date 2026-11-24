use super::*;

#[test]
fn client_chat_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_disguised_chat(ProtocolDisguisedChat {
            message: "server notice".to_string(),
            message_styled: Vec::new(),
            message_component: None,
            chat_type: ChatTypeBound {
                chat_type: ChatTypeHolder::Registry { id: 0 },
                name: "Server".into(),
                name_styled: Vec::new(),
                name_component: None,
                target_name: None,
                target_name_styled: None,
                target_name_component: None,
            },
        });
        store.push_client_system_chat_message("[Debug]: Client version info:");
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_chat".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let chat = response.result.unwrap();
    assert_eq!(chat["messages"][0]["kind"], "Disguised");
    assert_eq!(chat["messages"][0]["content"], "server notice");
    assert_eq!(chat["messages"][0]["sender_name"], "Server");
    assert_eq!(chat["messages"][0]["chat_type"]["registry_id"], 0);
    assert_eq!(chat["messages"][0]["validation_state"], "Unsigned");
    assert_eq!(chat["messages"][1]["kind"], "ClientSystem");
    assert_eq!(
        chat["messages"][1]["content"],
        "[Debug]: Client version info:"
    );
    assert_eq!(
        chat["messages"][1]["styled_content"],
        json!([{
            "text": "[Debug]: Client version info:",
            "style": {
                "bold": null,
                "italic": null,
                "underlined": null,
                "strikethrough": null,
                "obfuscated": null,
                "color": null,
                "click_event": null
            }
        }])
    );
    assert_eq!(chat["messages"][1]["sender_name"], "");
    assert_eq!(
        chat["messages"][1]["chat_type"]["registry_id"],
        serde_json::Value::Null
    );
    assert_eq!(chat["messages"][1]["validation_state"], "Unsigned");
    assert_eq!(chat["deleted_messages"].as_array().unwrap().len(), 0);
}

#[test]
fn client_combat_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_player_combat_kill(PlayerCombatKill {
            player_id: 123,
            message: bbb_protocol::RichTextComponent::plain("You died"),
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_combat".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let combat = response.result.unwrap();
    assert_eq!(combat["last_combat"]["kind"], "kill");
    assert_eq!(combat["last_combat"]["duration"], serde_json::Value::Null);
    assert_eq!(combat["last_combat"]["player_id"], 123);
    assert_eq!(combat["last_combat"]["message"]["text"], "You died");
}

#[test]
fn client_cooldowns_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_cooldown(Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 20,
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_cooldowns".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let cooldowns = response.result.unwrap();
    assert_eq!(
        cooldowns["minecraft:ender_pearl"]["cooldown_group"],
        "minecraft:ender_pearl"
    );
    assert_eq!(cooldowns["minecraft:ender_pearl"]["duration"], 20);

    let mut cleared_world = (*state.snapshot.load().world_view.world).clone();
    cleared_world.apply_cooldown(Cooldown {
        cooldown_group: "minecraft:ender_pearl".to_string(),
        duration: 0,
    });
    publish_test_world(&state, cleared_world);
    let response = dispatch(
        ControlRequest {
            method: "world.client_cooldowns".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert!(response
        .result
        .unwrap()
        .get("minecraft:ender_pearl")
        .is_none());
}

#[test]
fn client_effects_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_explosion(ProtocolExplosion {
            center: ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            radius: 4.5,
            block_count: 7,
            player_knockback: Some(ProtocolVec3d {
                x: 0.25,
                y: -0.5,
                z: 1.5,
            }),
            explosion_particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
                raw_options: Vec::new(),
            },
            explosion_sound: SoundEventHolder::Reference { registry_id: 41 },
            block_particles: vec![WeightedExplosionParticle {
                particle: ParticlePayload {
                    particle_type_id: VANILLA_PARTICLE_TYPE_CLOUD_ID,
                    raw_options: Vec::new(),
                },
                scaling: 0.5,
                speed: 1.25,
                weight: 3,
            }],
        });
        store.apply_level_particles(ProtocolLevelParticles {
            override_limiter: true,
            always_show: false,
            position: ProtocolVec3d {
                x: 10.0,
                y: 64.5,
                z: -3.25,
            },
            offset: ProtocolVec3d {
                x: f64::from(0.1_f32),
                y: f64::from(0.2_f32),
                z: f64::from(0.3_f32),
            },
            max_speed: 1.5,
            count: 16,
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_HEART_ID,
                raw_options: vec![0xaa, 0xbb],
            },
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_effects".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let effects = response.result.unwrap();
    assert_eq!(effects["last_explosion"]["center"]["x"], 1.0);
    assert_eq!(effects["last_explosion"]["center"]["y"], 2.0);
    assert_eq!(effects["last_explosion"]["center"]["z"], 3.0);
    assert_eq!(effects["last_explosion"]["radius"], 4.5);
    assert_eq!(effects["last_explosion"]["block_count"], 7);
    assert_eq!(effects["last_explosion"]["player_knockback"]["x"], 0.25);
    assert_eq!(effects["last_explosion"]["player_knockback"]["y"], -0.5);
    assert_eq!(effects["last_explosion"]["player_knockback"]["z"], 1.5);
    assert_eq!(
        effects["last_explosion"]["explosion_particle"]["particle_type_id"],
        VANILLA_PARTICLE_TYPE_EXPLOSION_ID
    );
    assert_eq!(
        effects["last_explosion"]["explosion_sound"]["Reference"]["registry_id"],
        41
    );
    assert_eq!(
        effects["last_explosion"]["block_particles"][0]["particle"]["particle_type_id"],
        VANILLA_PARTICLE_TYPE_CLOUD_ID
    );
    assert_eq!(effects["last_explosion"]["block_particles"][0]["weight"], 3);
    assert_eq!(effects["last_level_particles"]["override_limiter"], true);
    assert_eq!(effects["last_level_particles"]["always_show"], false);
    assert_eq!(effects["last_level_particles"]["position"]["x"], 10.0);
    assert_eq!(effects["last_level_particles"]["position"]["y"], 64.5);
    assert_eq!(effects["last_level_particles"]["position"]["z"], -3.25);
    assert_eq!(effects["last_level_particles"]["max_speed"], 1.5);
    assert_eq!(effects["last_level_particles"]["count"], 16);
    assert_eq!(
        effects["last_level_particles"]["particle_type_id"],
        VANILLA_PARTICLE_TYPE_HEART_ID
    );
    assert_eq!(effects["last_level_particles"]["raw_options_len"], 2);
}

#[test]
fn client_stats_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_award_stats(AwardStats {
            stats: vec![
                StatUpdate {
                    stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                    value_id: 10,
                    amount: 3,
                },
                StatUpdate {
                    stat_type_id: VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                    value_id: 4,
                    amount: 11,
                },
                StatUpdate {
                    stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,
                    value_id: 10,
                    amount: 5,
                },
            ],
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_stats".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let stats = response.result.unwrap();
    assert_eq!(
        stats["values"][0]["stat_type_id"],
        VANILLA_STAT_TYPE_BLOCK_MINED_ID
    );
    assert_eq!(stats["values"][0]["value_id"], 4);
    assert_eq!(stats["values"][0]["amount"], 11);
    assert_eq!(
        stats["values"][1]["stat_type_id"],
        VANILLA_STAT_TYPE_CUSTOM_ID
    );
    assert_eq!(stats["values"][1]["value_id"], 10);
    assert_eq!(stats["values"][1]["amount"], 5);
    assert_eq!(
        stats["last_update"]["entries"][0]["stat_type_id"],
        VANILLA_STAT_TYPE_CUSTOM_ID
    );
    assert_eq!(stats["last_update"]["entries"][0]["value_id"], 10);
    assert_eq!(stats["last_update"]["entries"][0]["amount"], 3);
    assert_eq!(
        stats["last_update"]["entries"][2]["stat_type_id"],
        VANILLA_STAT_TYPE_CUSTOM_ID
    );
    assert_eq!(stats["last_update"]["entries"][2]["value_id"], 10);
    assert_eq!(stats["last_update"]["entries"][2]["amount"], 5);
}

#[test]
fn client_waypoints_reads_canonical_world_state() {
    let state = test_control_state();
    let waypoint_id = uuid::Uuid::from_u128(0x00112233445566778899aabbccddeeff);
    {
        let mut store = WorldStore::new();
        store.apply_waypoint(TrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: TrackedWaypoint {
                identifier: WaypointIdentifier::Uuid(waypoint_id),
                icon: WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: Some(0x112233),
                },
                data: WaypointData::Position(WaypointVec3i {
                    x: 10,
                    y: 64,
                    z: -5,
                }),
            },
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_waypoints".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let waypoints = response.result.unwrap();
    let key = format!("uuid:{waypoint_id}");
    assert_eq!(
        waypoints["tracked"][key.as_str()]["identifier_kind"],
        "uuid"
    );
    assert_eq!(
        waypoints["tracked"][key.as_str()]["data"]["position"]["x"],
        10
    );
    assert_eq!(waypoints["last_event"]["operation"], "track");
    assert_eq!(waypoints["last_event"]["applied"], true);
    assert_eq!(
        waypoints["last_event"]["waypoint"]["icon_color_rgb"],
        0x112233
    );
}

#[test]
fn client_ui_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_low_disk_space_warning();
        store.apply_code_of_conduct("Respect the realm.".to_string());
        store.apply_add_entity(protocol_add_entity(
            42,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
        ));
        store.apply_mount_screen_open(MountScreenOpen {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        });
        store.apply_open_book(OpenBook {
            hand: InteractionHand::OffHand,
        });
        store.apply_open_sign_editor(OpenSignEditor {
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
        });
        store.apply_place_ghost_recipe(PlaceGhostRecipe {
            container_id: 9,
            recipe_display: RecipeDisplaySummary {
                display_type: RecipeDisplayType::Stonecutter,
                raw_body: vec![3, 4, 100, 4, 101, 4, 102],
                crafting: None,
                furnace: None,
                stonecutter: None,
                smithing: None,
            },
        });
        store.apply_show_dialog(ShowDialog {
            dialog: DialogHolder::Reference { registry_id: 7 },
        });
        store.apply_pong_response(PongResponse { time: 123456789 });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_ui".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let ui = response.result.unwrap();
    assert_eq!(ui["low_disk_space_warning_count"], 1);
    assert_eq!(ui["last_code_of_conduct"]["text"], "Respect the realm.");
    assert_eq!(
        ui["last_code_of_conduct"]["text_hash"],
        bbb_client_world::code_of_conduct_text_hash("Respect the realm.")
    );
    assert_eq!(ui["last_mount_screen"]["container_id"], 11);
    assert_eq!(ui["last_mount_screen"]["inventory_columns"], 5);
    assert_eq!(ui["last_mount_screen"]["entity_id"], 42);
    assert_eq!(ui["last_open_book"]["hand"], "off_hand");
    assert_eq!(ui["last_ghost_recipe"], serde_json::Value::Null);
    assert_eq!(ui["current_dialog"]["holder_kind"], "reference");
    assert_eq!(ui["current_dialog"]["registry_id"], 7);
    assert_eq!(ui["last_pong_response"]["time"], 123456789);
}

#[test]
fn client_audio_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_sound_event(SoundEvent {
            sound: SoundEventHolder::Reference { registry_id: 41 },
            source: SoundSource::Blocks,
            position: ProtocolVec3d {
                x: 2.5,
                y: -1.0,
                z: 0.0,
            },
            volume: 0.75,
            pitch: 1.25,
            seed: 123456789,
        });
        store.record_local_sound(bbb_client_world::LocalSoundEventState {
            sound: bbb_client_world::SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some("minecraft:block.portal.travel".to_string()),
                fixed_range: None,
            },
            source: "ambient".to_string(),
            volume: 0.25,
            pitch: 1.0,
            seed: 0,
        });
        store.apply_level_event(LevelEvent {
            event_type: 1010,
            pos: bbb_protocol::packets::BlockPos { x: 4, y: 65, z: -7 },
            data: 12,
            global: false,
        });
        store.apply_stop_sound(StopSound {
            source: Some(SoundSource::Music),
            name: Some("minecraft:music.menu".to_string()),
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_audio".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let audio = response.result.unwrap();
    assert_eq!(audio["last_sound"]["sound"]["kind"], "reference");
    assert_eq!(audio["last_sound"]["sound"]["registry_id"], 41);
    assert_eq!(audio["last_sound"]["position"]["x"], 2.5);
    assert_eq!(audio["last_sound"]["position"]["y"], -1.0);
    assert_eq!(audio["last_sound"]["position"]["z"], 0.0);
    assert_eq!(
        audio["last_local_sound"]["sound"]["location"],
        "minecraft:block.portal.travel"
    );
    assert_eq!(audio["last_local_sound"]["source"], "ambient");
    assert_eq!(audio["last_local_sound"]["volume"], 0.25);
    assert_eq!(audio["playing_jukebox_songs"][0]["song_registry_id"], 12);
    assert_eq!(audio["playing_jukebox_songs"][0]["pos"]["x"], 4);
    assert_eq!(audio["last_jukebox_event"]["action"], "start");
    assert_eq!(audio["last_jukebox_event"]["song_registry_id"], 12);
    assert_eq!(audio["last_stop_sound"]["source"], "music");
    assert_eq!(audio["last_stop_sound"]["name"], "minecraft:music.menu");
}

#[test]
fn client_debug_game_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_debug_block_value(DebugBlockValue {
            pos: ProtocolBlockPos { x: 4, y: 65, z: -9 },
            update: bbb_protocol::packets::DebugSubscriptionUpdate {
                subscription_id: 6,
                value: Some(
                    bbb_protocol::packets::DebugSubscriptionValue::EntityBlockIntersection(
                        bbb_protocol::packets::DebugEntityBlockIntersection::InAir,
                    ),
                ),
            },
        });
        store.apply_game_rule_values(GameRuleValues {
            values: vec![
                GameRuleValue {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                GameRuleValue {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "12".to_string(),
                },
            ],
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_debug_game".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let debug_game = response.result.unwrap();
    assert_eq!(debug_game["last_debug_block_value"]["pos"]["x"], 4);
    assert_eq!(debug_game["last_debug_block_value"]["pos"]["y"], 65);
    assert_eq!(debug_game["last_debug_block_value"]["pos"]["z"], -9);
    assert_eq!(debug_game["last_debug_block_value"]["subscription_id"], 6);
    assert_eq!(debug_game["last_debug_block_value"]["value_present"], true);
    assert_eq!(
        debug_game["last_game_rule_values"]["values"],
        json!([
            {
                "rule": "minecraft:do_daylight_cycle",
                "value": "false"
            },
            {
                "rule": "minecraft:random_tick_speed",
                "value": "12"
            }
        ])
    );
}

#[test]
fn client_debug_query_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_tag_query(TagQuery {
            transaction_id: 12,
            tag_present: true,
            raw_nbt: vec![10, 0],
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_debug_query".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let debug_query = response.result.unwrap();
    assert_eq!(debug_query["last_tag_query"]["transaction_id"], 12);
    assert_eq!(debug_query["last_tag_query"]["tag_present"], true);
    assert_eq!(debug_query["last_tag_query"]["raw_nbt"], json!([10, 0]));
}

#[test]
fn client_command_suggestions_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_custom_chat_completions(CustomChatCompletions {
            action: CustomChatCompletionsAction::Set,
            entries: vec!["/warp".to_string(), "/spawn".to_string()],
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_command_suggestions".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let suggestions = response.result.unwrap();
    assert_eq!(
        suggestions["custom_completions"],
        json!(["/spawn", "/warp"])
    );
    assert_eq!(
        suggestions["last_custom_completion_update"]["action"],
        "set"
    );
    assert_eq!(suggestions["last_custom_completion_update"]["entries"], 2);
}

#[test]
fn client_features_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_update_enabled_features(UpdateEnabledFeatures {
            features: vec![
                "minecraft:unknown".to_string(),
                "minecraft:vanilla".to_string(),
                "minecraft:trade_rebalance".to_string(),
            ],
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_features".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(
        response.result.unwrap(),
        json!(["minecraft:trade_rebalance", "minecraft:vanilla"])
    );
}

#[test]
fn client_known_packs_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_select_known_packs(
            vec![KnownPack {
                namespace: "minecraft".to_string(),
                id: "core".to_string(),
                version: "26.1".to_string(),
            }],
            Vec::new(),
        );
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_known_packs".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let known_packs = response.result.unwrap();
    assert_eq!(known_packs["offered"][0]["namespace"], "minecraft");
    assert_eq!(known_packs["offered"][0]["id"], "core");
    assert_eq!(known_packs["offered"][0]["version"], "26.1");
    assert_eq!(known_packs["selected"], json!([]));
}

#[test]
fn apply_diagnostics_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.record_apply_error("light_update", "invalid light payload");
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.apply_diagnostics".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let diagnostics = response.result.unwrap();
    assert_eq!(diagnostics["apply_errors"][0]["source"], "light_update");
    assert_eq!(
        diagnostics["apply_errors"][0]["message"],
        "invalid light payload"
    );
    assert_eq!(
        diagnostics["last_apply_error"]["message"],
        "invalid light payload"
    );
}

#[test]
fn command_tree_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_commands(command_tree("say"));
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.command_tree".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let commands = response.result.unwrap();
    assert_eq!(commands["root_index"], 0);
    assert_eq!(commands["nodes"][1]["name"], "say");
    assert_eq!(commands["nodes"][2]["parser"]["name"], "brigadier:string");
    assert_eq!(commands["nodes"][2]["suggestions"], "minecraft:ask_server");
    assert_eq!(commands["nodes"][2]["executable"], true);
    assert_eq!(commands["nodes"][2]["restricted"], true);
}

#[test]
fn last_block_changed_ack_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_block_changed_ack(BlockChangedAck { sequence: 17 });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.last_block_changed_ack".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    assert_eq!(response.result.unwrap()["sequence"], 17);
}

#[test]
fn client_block_events_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        assert!(store.apply_block_destruction(BlockDestruction {
            id: 7,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5
            },
            progress: 3,
        }));
        store.apply_block_event(BlockEvent {
            pos: ProtocolBlockPos {
                x: 13,
                y: 65,
                z: -6,
            },
            b0: 1,
            b1: 5,
            block_id: 123,
        });
        store.apply_level_event(LevelEvent {
            event_type: 2001,
            pos: ProtocolBlockPos {
                x: 14,
                y: 66,
                z: -7,
            },
            data: 9,
            global: true,
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_block_events".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let events = response.result.unwrap();
    assert_eq!(events["destructions"][0]["id"], 7);
    assert_eq!(events["destructions"][0]["pos"]["x"], 12);
    assert_eq!(events["destructions"][0]["pos"]["z"], -5);
    assert_eq!(events["destructions"][0]["progress"], 3);
    assert_eq!(events["block_events"][0]["pos"]["x"], 13);
    assert_eq!(events["block_events"][0]["b0"], 1);
    assert_eq!(events["block_events"][0]["b1"], 5);
    assert_eq!(events["block_events"][0]["block_id"], 123);
    assert_eq!(events["level_events"][0]["event_type"], 2001);
    assert_eq!(events["level_events"][0]["pos"]["y"], 66);
    assert_eq!(events["level_events"][0]["data"], 9);
    assert_eq!(events["level_events"][0]["global"], true);
}

#[test]
fn world_border_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_initialize_border(InitializeBorder {
            new_center_x: 10.5,
            new_center_z: -20.25,
            old_size: 100.0,
            new_size: 200.0,
            lerp_time: 0,
            new_absolute_max_size: 400,
            warning_blocks: 6,
            warning_time: 7,
        });
        store.apply_set_border_center(SetBorderCenter {
            new_center_x: 30.0,
            new_center_z: -40.0,
        });
        store.apply_set_border_lerp_size(SetBorderLerpSize {
            old_size: 200.0,
            new_size: 300.0,
            lerp_time: 50,
        });
        store.apply_set_border_warning_delay(SetBorderWarningDelay { warning_delay: 11 });
        store.apply_set_border_warning_distance(SetBorderWarningDistance { warning_blocks: 12 });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.world_border".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let border = response.result.unwrap();
    assert_eq!(border["center_x"], 30.0);
    assert_eq!(border["center_z"], -40.0);
    assert_eq!(border["size"], 200.0);
    assert_eq!(border["lerp_target"], 300.0);
    assert_eq!(border["lerp_time"], 50);
    assert_eq!(border["absolute_max_size"], 400);
    assert_eq!(border["warning_blocks"], 12);
    assert_eq!(border["warning_time"], 11);
}

#[test]
fn client_advancements_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![AdvancementSummary {
                id: "minecraft:story/root".to_string(),
                parent: None,
                display: None,
                requirements: Vec::new(),
                sends_telemetry_event: false,
            }],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });
        store.apply_select_advancements_tab(SelectAdvancementsTab {
            tab: Some("minecraft:story/root".to_string()),
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_advancements".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let advancements = response.result.unwrap();
    assert_eq!(advancements["selected_tab"], "minecraft:story/root");
    assert!(advancements["advancements"]
        .as_object()
        .unwrap()
        .contains_key("minecraft:story/root"));
}

#[test]
fn client_local_player_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_player_abilities(PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        store.apply_player_health(PlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        });
        store.apply_player_experience(PlayerExperience {
            progress: 0.75,
            level: 8,
            total: 123,
        });
        store.apply_default_spawn_position(SetDefaultSpawnPosition {
            dimension: "minecraft:overworld".to_string(),
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        });
        store.apply_simulation_distance(SetSimulationDistance { distance: 12 });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_local_player".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let local = response.result.unwrap();
    assert_eq!(local["abilities"]["can_fly"], true);
    assert_eq!(local["health"]["food"], 16);
    assert_eq!(local["experience"]["total"], 123);
    assert_eq!(local["default_spawn"]["pos"]["x"], -5);
    assert_eq!(local["default_spawn"]["yaw"], 90.0);
    assert_eq!(local["simulation_distance"], 12);
}

#[test]
fn client_player_info_reads_canonical_world_state() {
    let state = test_control_state();
    let profile_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    {
        let mut store = WorldStore::new();
        let applied = store.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![
                PlayerInfoAction::AddPlayer,
                PlayerInfoAction::UpdateGameMode,
                PlayerInfoAction::UpdateListed,
                PlayerInfoAction::UpdateLatency,
                PlayerInfoAction::UpdateDisplayName,
                PlayerInfoAction::UpdateHat,
                PlayerInfoAction::UpdateListOrder,
            ],
            entries: vec![PlayerInfoEntry {
                profile_id,
                profile: Some(GameProfile {
                    uuid: profile_id,
                    name: "Ada".into(),
                    properties: vec![GameProfileProperty {
                        name: "textures".into(),
                        value: "skin-payload".to_string(),
                        signature: Some("skin-signature".to_string()),
                    }],
                }),
                listed: true,
                latency: 42,
                game_mode: GameType::Creative,
                display_name: Some(bbb_protocol::RichTextComponent::plain(
                    "{\"text\":\"Ada Lovelace\"}",
                )),
                show_hat: true,
                list_order: 7,
                chat_session: None,
            }],
        });
        assert_eq!(applied, 1);
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_player_info".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let player_info = response.result.unwrap();
    let key = profile_id.to_string();
    let entry = &player_info["entries"][&key];
    assert_eq!(entry["profile"]["uuid"], key);
    assert_eq!(entry["profile"]["name"], "Ada");
    assert_eq!(entry["profile"]["properties"][0]["name"], "textures");
    assert_eq!(entry["listed"], true);
    assert_eq!(entry["latency"], 42);
    assert_eq!(entry["game_mode"], "creative");
    assert_eq!(entry["display_name"]["text"], "{\"text\":\"Ada Lovelace\"}");
    assert_eq!(
        entry["display_name"]["component"],
        json!({"type": "String", "value": "{\"text\":\"Ada Lovelace\"}"})
    );
    assert_eq!(entry["show_hat"], true);
    assert_eq!(entry["list_order"], 7);
    assert_eq!(entry["chat_session_present"], false);
    assert_eq!(
        player_info["listed_players"],
        json!([profile_id.to_string()])
    );
}

#[test]
fn client_scoreboard_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        assert!(store.apply_set_objective(SetObjective {
            objective_name: "kills".into(),
            method: SetObjectiveMethod::Add.into(),
            parameters: Some(SetObjectiveParameters {
                display_name: bbb_protocol::RichTextComponent::plain("Kills"),
                render_type: ObjectiveRenderType::Hearts,
                number_format: Some(bbb_protocol::packets::NumberFormatSummary::Blank),
            }),
        }));
        assert!(store.apply_set_display_objective(SetDisplayObjective {
            slot: ScoreboardDisplaySlot::Sidebar,
            objective_name: Some("kills".into()),
        }));
        assert!(store.apply_set_score(SetScore {
            owner: "Alice".into(),
            objective_name: "kills".into(),
            score: 4,
            display: Some(bbb_protocol::RichTextComponent::plain("Alice")),
            number_format: Some(bbb_protocol::packets::NumberFormatSummary::Blank),
        }));
        assert!(store.apply_set_player_team(SetPlayerTeam {
            name: "red".into(),
            method: PlayerTeamMethod::Add.into(),
            parameters: Some(PlayerTeamParameters {
                display_name: bbb_protocol::RichTextComponent::plain("Red Team"),
                options: 3,
                nametag_visibility: TeamVisibility::Always,
                collision_rule: TeamCollisionRule::PushOtherTeams,
                color: ChatFormatting::Red,
                player_prefix: bbb_protocol::RichTextComponent::plain("[R] "),
                player_suffix: bbb_protocol::RichTextComponent::plain("!"),
            }),
            players: vec!["Alice".into(), "Bob".into()],
        }));
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_scoreboard".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let scoreboard = response.result.unwrap();
    assert_eq!(scoreboard["objectives"]["kills"]["display_name"], "Kills");
    assert_eq!(scoreboard["objectives"]["kills"]["render_type"], "hearts");
    assert_eq!(
        scoreboard["objectives"]["kills"]["number_format"],
        json!({"type": "blank"})
    );
    assert_eq!(scoreboard["display_slots"]["sidebar"], "kills");
    assert_eq!(scoreboard["scores"]["Alice"]["kills"]["value"], 4);
    assert_eq!(scoreboard["scores"]["Alice"]["kills"]["display"], "Alice");
    assert_eq!(
        scoreboard["scores"]["Alice"]["kills"]["number_format"],
        json!({"type": "blank"})
    );
    assert_eq!(
        scoreboard["teams"]["red"]["parameters"]["display_name"],
        "Red Team"
    );
    assert_eq!(scoreboard["teams"]["red"]["parameters"]["color"], "red");
    assert_eq!(
        scoreboard["teams"]["red"]["parameters"]["collision_rule"],
        "pushOtherTeams"
    );
    assert_eq!(
        scoreboard["teams"]["red"]["players"],
        json!(["Alice", "Bob"])
    );
}

#[test]
fn client_maps_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        assert!(store.apply_map_item_data(MapItemData {
            map_id: 42,
            scale: 2,
            locked: true,
            decorations: Some(vec![MapDecoration {
                type_id: 4,
                x: -20,
                y: 30,
                rot: 7,
                name: Some(bbb_protocol::RichTextComponent::plain("Village")),
            }]),
            color_patch: Some(MapColorPatch {
                start_x: 3,
                start_y: 4,
                width: 2,
                height: 2,
                colors: vec![1, 2, 3, 4],
            }),
        }));
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_maps".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let maps = response.result.unwrap();
    let map = &maps["42"];
    assert_eq!(map["id"], 42);
    assert_eq!(map["scale"], 2);
    assert_eq!(map["locked"], true);
    assert_eq!(map["decorations"][0]["type_id"], 4);
    assert_eq!(map["decorations"][0]["x"], -20);
    assert_eq!(map["decorations"][0]["name"]["text"], "Village");
    assert_eq!(
        map["decorations"][0]["name"]["component"],
        json!({"type": "String", "value": "Village"})
    );
    assert_eq!(map["last_color_patch"]["start_x"], 3);
    assert_eq!(map["last_color_patch"]["height"], 2);
    let colors = map["colors"].as_array().unwrap();
    assert_eq!(colors[3 + 4 * 128], 1);
    assert_eq!(colors[4 + 4 * 128], 2);
    assert_eq!(colors[3 + 5 * 128], 3);
    assert_eq!(colors[4 + 5 * 128], 4);
}

#[test]
fn last_map_color_patch_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_map_item_data(MapItemData {
            map_id: 42,
            scale: 2,
            locked: true,
            decorations: Some(vec![MapDecoration {
                type_id: 4,
                x: -20,
                y: 30,
                rot: 7,
                name: Some(bbb_protocol::RichTextComponent::plain("Village")),
            }]),
            color_patch: Some(MapColorPatch {
                start_x: 3,
                start_y: 4,
                width: 2,
                height: 2,
                colors: vec![1, 2, 3, 4],
            }),
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.last_map_color_patch".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let patch = response.result.unwrap();
    assert_eq!(patch["map_id"], 42);
    assert_eq!(patch["start_x"], 3);
    assert_eq!(patch["start_y"], 4);
    assert_eq!(patch["width"], 2);
    assert_eq!(patch["height"], 2);
}

#[test]
fn level_clock_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_world_time(PlayTime {
            game_time: 123,
            clock_updates: vec![bbb_protocol::packets::ClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.0,
                rate: 1.0,
            }],
        });
        store.apply_game_event(GameEvent {
            event_id: 7,
            param: 0.5,
        });
        store.apply_ticking_state(TickingState {
            tick_rate: 0.25,
            frozen: true,
        });
        store.apply_ticking_step(TickingStep { tick_steps: 7 });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.level_clock".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let clock = response.result.unwrap();
    assert_eq!(clock["world_time"]["game_time"], 123);
    assert_eq!(clock["world_time"]["day_time"], 6000);
    assert_eq!(clock["world_time"]["clock_updates"][0]["total_ticks"], 6000);
    assert_eq!(clock["weather"]["raining"], true);
    assert_eq!(clock["weather"]["rain_level"], 0.5);
    assert_eq!(clock["ticking"]["tick_rate"], 1.0);
    assert_eq!(clock["ticking"]["frozen"], true);
    assert_eq!(clock["ticking"]["frozen_ticks_to_run"], 7);
}

#[test]
fn server_presentation_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_transfer(Transfer {
            host: "next.example.com".into(),
            port: 25566,
        });
        store.apply_custom_report_details(CustomReportDetails {
            details: BTreeMap::from([
                ("Region".to_string(), "local".to_string()),
                ("Server".to_string(), "bbb test shard".to_string()),
            ]),
        });
        store.apply_store_cookie("bbb:session", 3, 1);
        store.apply_custom_payload(CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: CustomPayloadBody::Brand {
                brand: "vanilla".to_string(),
            },
        });
        store.apply_server_links(ServerLinks {
            links: vec![ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                url: "https://example.invalid/support".to_string(),
            }],
        });
        let pack_id = Uuid::from_u128(0x11111111111111111111111111111111);
        store.apply_resource_pack_push(bbb_protocol::packets::ResourcePackPush {
            id: pack_id,
            url: "https://example.invalid/server-pack.zip".to_string(),
            hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            required: false,
            prompt: None,
        });
        let push_sequence = store.resource_pack(pack_id).unwrap().push_sequence;
        store.update_resource_pack_runtime_state(
            pack_id,
            push_sequence,
            ResourcePackRuntimeState {
                phase: ResourcePackRuntimePhase::Failed,
                generation: Some(7),
                failure: Some(ResourcePackRuntimeFailure {
                    stage: ResourcePackFailureStage::Reload,
                    generation: Some(7),
                    error: "missing custom model".to_string(),
                }),
            },
        );
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.server_presentation".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let presentation = response.result.unwrap();
    assert_eq!(presentation["last_transfer"]["host"], "next.example.com");
    assert_eq!(presentation["last_transfer"]["port"], 25566);
    assert_eq!(presentation["custom_report_details"]["Region"], "local");
    assert_eq!(
        presentation["custom_report_details"]["Server"],
        "bbb test shard"
    );
    assert_eq!(presentation["server_cookies"]["last_key"], "bbb:session");
    assert_eq!(presentation["server_cookies"]["stored_count"], 1);
    assert_eq!(presentation["server_brand"], "vanilla");
    assert_eq!(presentation["last_custom_payload"]["id"], "minecraft:brand");
    assert_eq!(presentation["last_custom_payload"]["kind"], "brand");
    assert_eq!(presentation["last_custom_payload"]["brand"], "vanilla");
    assert_eq!(
        presentation["server_links"][0]["label"],
        "known_server_link.support"
    );
    assert_eq!(
        presentation["server_links"][0]["url"],
        "https://example.invalid/support"
    );
    assert_eq!(presentation["server_links"][0]["known_type"], "support");
    let pack = &presentation["resource_packs"]["11111111-1111-1111-1111-111111111111"];
    assert_eq!(pack["runtime"]["phase"], "failed");
    assert_eq!(pack["runtime"]["generation"], 7);
    assert_eq!(pack["runtime"]["failure"]["stage"], "reload");
    assert_eq!(pack["runtime"]["failure"]["generation"], 7);
    assert_eq!(pack["runtime"]["failure"]["error"], "missing custom model");
}

#[test]
fn probes_chunk_and_block_from_world_store() {
    let state = test_control_state();
    {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store.insert_decoded_chunk(single_section_chunk());

        publish_test_world(&state, store);
    }

    let chunk_response = dispatch(
        ControlRequest {
            method: "world.probe_chunk".to_string(),
            params: json!({"x": 1, "z": -2}),
        },
        &state,
    );
    assert!(chunk_response.ok);
    let chunk = chunk_response.result.unwrap();
    assert_eq!(chunk["state"], "Decoded");
    assert_eq!(chunk["sections"], 1);
    assert_eq!(chunk["heightmaps"], 1);

    let block_response = dispatch(
        ControlRequest {
            method: "world.probe_block".to_string(),
            params: json!({"x": 17, "y": 0, "z": -31}),
        },
        &state,
    );
    assert!(block_response.ok);
    let block = block_response.result.unwrap();
    assert_eq!(block["block_state_id"], 42);
    assert_eq!(block["block_name"], "minecraft:dark_oak_sapling");
    assert_eq!(block["block_properties"]["stage"], "1");
    assert_eq!(block["block_palette_kind"], "SingleValue");
    assert_eq!(block["biome_id"], 3);

    let terrain_response = dispatch(
        ControlRequest {
            method: "world.terrain_chunk_summary".to_string(),
            params: json!({"x": 1, "z": -2}),
        },
        &state,
    );
    assert!(terrain_response.ok);
    let terrain = terrain_response.result.unwrap();
    assert_eq!(terrain["total_blocks"], 4096);
    assert_eq!(terrain["opaque_blocks"], 4096);
    assert_eq!(terrain["cutout_blocks"], 0);
    assert_eq!(terrain["empty_blocks"], 0);

    let missing_response = dispatch(
        ControlRequest {
            method: "world.probe_block".to_string(),
            params: json!({"x": 17, "y": 16, "z": -31}),
        },
        &state,
    );
    assert!(missing_response.ok);
    assert!(missing_response.result.unwrap().is_null());
}

#[test]
fn chunk_view_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.insert_decoded_chunk(single_section_chunk());
        store.apply_set_chunk_cache_center(SetChunkCacheCenter {
            chunk_x: -4,
            chunk_z: 7,
        });
        store.apply_set_chunk_cache_radius(SetChunkCacheRadius { radius: 10 });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.chunk_view".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );

    assert!(response.ok);
    let view = response.result.unwrap();
    assert_eq!(view["first_chunk"]["x"], 1);
    assert_eq!(view["first_chunk"]["z"], -2);
    assert_eq!(view["center"]["x"], -4);
    assert_eq!(view["center"]["z"], 7);
    assert_eq!(view["radius"], 10);
}

#[test]
fn probes_entity_transforms_from_world_store_components() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(7, 3));
        store.apply_add_entity(protocol_add_entity(9, 85));
        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 9,
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 70.0,
                    z: -8.0,
                },
                delta_movement: ProtocolVec3d::default(),
                y_rot: 45.0,
                x_rot: -15.0,
                on_ground: true,
            })
        );
        assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
            id: 9,
            delta_movement: ProtocolVec3d {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            },
        }));

        publish_test_world(&state, store);
    }

    let one_response = dispatch(
        ControlRequest {
            method: "world.probe_entity_transform".to_string(),
            params: json!({"id": 9}),
        },
        &state,
    );
    assert!(one_response.ok);
    let one = one_response.result.unwrap();
    assert_eq!(one["id"], 9);
    assert_eq!(one["entity_type_id"], 85);
    assert_eq!(one["position"]["x"], 5.0);
    assert_eq!(one["delta_movement"]["z"], 0.3);
    assert_eq!(one["y_rot"], 45.0);
    assert_eq!(one["x_rot"], -15.0);
    assert_eq!(one["on_ground"], true);

    let all_response = dispatch(
        ControlRequest {
            method: "world.entity_transforms".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(all_response.ok);
    let all = all_response.result.unwrap();
    let ids: Vec<i64> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|entity| entity["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, vec![7, 9]);

    let missing_response = dispatch(
        ControlRequest {
            method: "world.probe_entity_transform".to_string(),
            params: json!({"id": 999}),
        },
        &state,
    );
    assert!(missing_response.ok);
    assert!(missing_response.result.unwrap().is_null());
}

#[test]
fn last_projectile_power_reads_canonical_world_state() {
    let state = test_control_state();

    let empty_response = dispatch(
        ControlRequest {
            method: "world.last_projectile_power".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(empty_response.ok);
    assert!(empty_response.result.unwrap().is_null());

    {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(10, 52));
        store.apply_add_entity(protocol_add_entity(20, 7));
        assert!(store.apply_projectile_power(ProjectilePower {
            entity_id: 10,
            acceleration_power: 0.75,
        }));
        publish_test_world(&state, store);
    }

    let applied_response = dispatch(
        ControlRequest {
            method: "world.last_projectile_power".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(applied_response.ok);
    let applied = applied_response.result.unwrap();
    assert_eq!(applied["entity_id"], 10);
    assert_eq!(applied["acceleration_power"], 0.75);
    assert_eq!(applied["applied"], true);

    let mut ignored_world = (*state.snapshot.load().world_view.world).clone();
    assert!(!ignored_world.apply_projectile_power(ProjectilePower {
        entity_id: 20,
        acceleration_power: 0.25,
    }));
    publish_test_world(&state, ignored_world);

    let ignored_response = dispatch(
        ControlRequest {
            method: "world.last_projectile_power".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(ignored_response.ok);
    let ignored = ignored_response.result.unwrap();
    assert_eq!(ignored["entity_id"], 20);
    assert_eq!(ignored["acceleration_power"], 0.25);
    assert_eq!(ignored["applied"], false);
}

#[test]
fn probe_entity_status_reads_canonical_world_state() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(7, 7));
        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: 7,
            event_id: 35,
        }));
        assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 7, yaw: 45.5 }));
        assert!(store.apply_damage_event(ProtocolDamageEvent {
            entity_id: 7,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: 42,
            source_position: Some(ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }),
        }));
        assert!(store.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 7,
            effect_id: 3,
            amplifier: 2,
            duration_ticks: 400,
            flags: MobEffectFlags {
                raw: 0b1011,
                ambient: true,
                visible: true,
                show_icon: false,
                blend: true,
            },
        }));
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.probe_entity_status".to_string(),
            params: json!({"id": 7}),
        },
        &state,
    );

    assert!(response.ok);
    let status = response.result.unwrap();
    assert_eq!(status["id"], 7);
    assert_eq!(status["entity_type_id"], 7);
    assert_eq!(status["last_event_id"], 35);
    assert_eq!(status["last_hurt_yaw"], 45.5);
    assert_eq!(status["mob_effects"]["3"]["effect_id"], 3);
    assert_eq!(status["mob_effects"]["3"]["amplifier"], 2);
    assert_eq!(status["mob_effects"]["3"]["duration_ticks"], 400);
    assert_eq!(status["mob_effects"]["3"]["ambient"], true);
    assert_eq!(status["mob_effects"]["3"]["visible"], true);
    assert_eq!(status["mob_effects"]["3"]["show_icon"], false);
    assert_eq!(status["mob_effects"]["3"]["blend"], true);
    assert_eq!(status["last_damage"]["source_type_id"], 5);
    assert_eq!(status["last_damage"]["source_cause_id"], -1);
    assert_eq!(status["last_damage"]["source_direct_id"], 42);
    assert_eq!(status["last_damage"]["source_position"]["x"], 1.0);
    assert_eq!(status["last_damage"]["source_position"]["y"], 2.0);
    assert_eq!(status["last_damage"]["source_position"]["z"], 3.0);

    let missing_response = dispatch(
        ControlRequest {
            method: "world.probe_entity_status".to_string(),
            params: json!({"id": 999}),
        },
        &state,
    );
    assert!(missing_response.ok);
    assert!(missing_response.result.unwrap().is_null());
}

#[test]
fn entity_pick_targets_probe_exposes_ender_dragon_part_targets() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(100, 43));
        store.advance_entity_client_animations(1);
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.entity_pick_targets".to_string(),
            params: json!({"partial_tick": 1.0}),
        },
        &state,
    );

    assert!(response.ok);
    let targets = response.result.unwrap();
    let targets = targets.as_array().unwrap();
    let ids = targets
        .iter()
        .map(|target| target["entity_id"].as_i64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![101, 102, 103, 104, 105, 106, 107, 108]);
    assert!(!ids.contains(&100));

    let head = &targets[0];
    assert!(head["position"]["x"].is_number());
    assert!(head["position"]["y"].is_number());
    assert!(head["position"]["z"].is_number());
    assert_eq!(head["bounds"]["max"][0], 0.5);
    assert_eq!(head["bounds"]["max"][1], 1.0);

    let wing = &targets[6];
    assert_eq!(wing["entity_id"], 107);
    assert_eq!(wing["bounds"]["min"][0], -2.0);
    assert_eq!(wing["bounds"]["max"][0], 2.0);
    assert_eq!(wing["bounds"]["max"][1], 2.0);
}

#[test]
fn probe_blocks_and_surface_return_loaded_and_unloaded_states_and_validate_bounds() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        let air = bbb_client_world::vanilla_air_block_state_id();
        let empty = BTreeMap::new();
        let stone = bbb_client_world::vanilla_block_state_id("minecraft:stone", &empty).unwrap();
        let dirt = bbb_client_world::vanilla_block_state_id("minecraft:dirt", &empty).unwrap();
        let water_props = BTreeMap::from([("level".to_string(), "0".to_string())]);
        let water =
            bbb_client_world::vanilla_block_state_id("minecraft:water", &water_props).unwrap();
        let mut sections = Vec::new();
        for _ in 0..store.dimension().section_count() {
            sections.push(Arc::new(ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(PaletteDomain::BlockStates, air, 4096),
                biomes: single_value_container(PaletteDomain::Biomes, 0, 64),
            }));
        }
        store.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections,
            block_entities: Arc::new(Vec::new()),
            light: LightData::default(),
        });
        store.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: ProtocolBlockPos { x: 1, y: 64, z: 1 },
            block_state_id: stone,
        });
        store.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: ProtocolBlockPos { x: 1, y: 70, z: 1 },
            block_state_id: dirt,
        });
        store.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: ProtocolBlockPos { x: 2, y: 63, z: 1 },
            block_state_id: water,
        });
        publish_test_world(&state, store);
    }

    let blocks_resp = dispatch(
        ControlRequest {
            method: "world.probe_blocks".to_string(),
            params: json!({
                "positions": [
                    {"x": 1, "y": 64, "z": 1},
                    {"x": 2, "y": 63, "z": 1},
                    {"x": 100, "y": 64, "z": 100}
                ]
            }),
        },
        &state,
    );
    assert!(blocks_resp.ok, "{:?}", blocks_resp.error);
    let blocks = blocks_resp.result.unwrap();
    assert_eq!(blocks["blocks"][0]["loaded"], true);
    assert_eq!(blocks["blocks"][0]["block_name"], "minecraft:stone");
    assert_eq!(blocks["blocks"][1]["block_name"], "minecraft:water");
    assert!(blocks["blocks"][1]["fluid"].is_object());
    assert_eq!(blocks["blocks"][2]["loaded"], false);
    assert!(blocks["blocks"][2]["block_name"].is_null());

    // Empty and oversized batches are rejected cleanly.
    assert!(
        !dispatch(
            ControlRequest {
                method: "world.probe_blocks".to_string(),
                params: json!({"positions": []}),
            },
            &state
        )
        .ok
    );

    // Surface without max_y hits the upper roof dirt at y=70; with max_y=68 it finds stone at y=64.
    let surface_resp = dispatch(
        ControlRequest {
            method: "world.probe_surface".to_string(),
            params: json!({"min_x": 1, "min_z": 1, "max_x": 2, "max_z": 1}),
        },
        &state,
    );
    assert!(surface_resp.ok, "{:?}", surface_resp.error);
    let surface = surface_resp.result.unwrap();
    assert_eq!(surface["columns"][0]["loaded"], true);
    assert_eq!(surface["columns"][0]["top_y"], 70);
    assert_eq!(surface["columns"][0]["block_name"], "minecraft:dirt");
    assert_eq!(surface["columns"][0]["is_fluid"], false);
    assert_eq!(surface["columns"][1]["top_y"], 63);
    assert_eq!(surface["columns"][1]["block_name"], "minecraft:water");
    assert_eq!(surface["columns"][1]["is_fluid"], true);

    let under_roof = dispatch(
        ControlRequest {
            method: "world.probe_surface".to_string(),
            params: json!({"min_x": 1, "min_z": 1, "max_x": 1, "max_z": 1, "max_y": 68}),
        },
        &state,
    );
    assert!(under_roof.ok);
    assert_eq!(under_roof.result.unwrap()["columns"][0]["top_y"], 64);

    // Unloaded column and inverted/oversized bounds.
    let unloaded = dispatch(
        ControlRequest {
            method: "world.probe_surface".to_string(),
            params: json!({"min_x": 64, "min_z": 64, "max_x": 64, "max_z": 64}),
        },
        &state,
    );
    assert!(unloaded.ok);
    assert_eq!(unloaded.result.unwrap()["columns"][0]["loaded"], false);
    assert!(
        !dispatch(
            ControlRequest {
                method: "world.probe_surface".to_string(),
                params: json!({"min_x": 5, "min_z": 0, "max_x": 1, "max_z": 0}),
            },
            &state
        )
        .ok
    );
    assert!(
        !dispatch(
            ControlRequest {
                method: "world.probe_surface".to_string(),
                params: json!({"min_x": 0, "min_z": 0, "max_x": 16, "max_z": 16}),
            },
            &state
        )
        .ok
    );
}

#[test]
fn crosshair_target_and_nearby_entities_resolve_blocks_mobs_and_dropped_items() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        let air = bbb_client_world::vanilla_air_block_state_id();
        let empty = BTreeMap::new();
        let stone = bbb_client_world::vanilla_block_state_id("minecraft:stone", &empty).unwrap();
        let mut sections = Vec::new();
        for _ in 0..store.dimension().section_count() {
            sections.push(Arc::new(ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(PaletteDomain::BlockStates, air, 4096),
                biomes: single_value_container(PaletteDomain::Biomes, 0, 64),
            }));
        }
        store.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections,
            block_entities: Arc::new(Vec::new()),
            light: LightData::default(),
        });
        store.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 65, z: 2 },
            block_state_id: stone,
        });
        store.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        // Add a dropped item entity near the player and a zombie further away.
        let mut drop_entity =
            protocol_add_entity(201, bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID);
        drop_entity.position = ProtocolVec3d {
            x: 1.5,
            y: 64.0,
            z: 0.5,
        };
        store.apply_add_entity(drop_entity);
        let oak_log_id =
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:oak_log")
                .unwrap()
                .id;
        let item_data_id =
            bbb_client_world::vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
            )
            .unwrap();
        store.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 201,
            values: vec![bbb_protocol::packets::EntityDataValue {
                data_id: item_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                value: bbb_protocol::packets::EntityDataValueKind::ItemStack(ItemStackSummary {
                    item_id: Some(oak_log_id),
                    count: 4,
                    component_patch: Default::default(),
                }),
            }],
        });
        let mut zombie = protocol_add_entity(
            202,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        );
        zombie.position = ProtocolVec3d {
            x: 6.5,
            y: 64.0,
            z: 0.5,
        };
        store.apply_add_entity(zombie);
        publish_test_world(&state, store);
    }

    let crosshair = dispatch(
        ControlRequest {
            method: "world.crosshair_target".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(crosshair.ok, "{:?}", crosshair.error);
    let target = crosshair.result.unwrap();
    assert_eq!(target["kind"], "block");
    assert_eq!(target["block"]["pos"], json!({"x": 0, "y": 65, "z": 2}));
    assert_eq!(target["block"]["face"], "north");
    assert_eq!(
        target["block"]["adjacent_pos"],
        json!({"x": 0, "y": 65, "z": 1})
    );
    assert_eq!(target["block"]["block_name"], "minecraft:stone");
    let distance = target["block"]["distance"].as_f64().unwrap();
    assert!(
        (1.0..=2.5).contains(&distance),
        "expected world-space distance near 1.5, got {distance}"
    );

    let nearby = dispatch(
        ControlRequest {
            method: "world.nearby_entities".to_string(),
            params: json!({"max_distance": 10.0, "limit": 10}),
        },
        &state,
    );
    assert!(nearby.ok, "{:?}", nearby.error);
    let entities = nearby.result.unwrap();
    let list = entities["entities"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["id"], 201);
    assert_eq!(list[0]["entity_type"], "minecraft:item");
    assert_eq!(list[0]["pickable"], false);
    assert_eq!(list[0]["item"]["item_name"], "minecraft:oak_log");
    assert_eq!(list[0]["item"]["count"], 4);
    assert_eq!(list[1]["id"], 202);
    assert_eq!(list[1]["entity_type"], "minecraft:zombie");
    assert_eq!(list[1]["pickable"], true);
}

#[test]
fn client_inventory_annotates_item_and_menu_resource_names() {
    let state = test_control_state();
    {
        let mut store = WorldStore::new();
        let diamond_id =
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:diamond")
                .unwrap()
                .id;
        store.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(diamond_id),
                count: 5,
                component_patch: Default::default(),
            },
        });
        store.apply_open_screen(OpenScreen {
            container_id: 2,
            menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
            title: bbb_protocol::RichTextComponent::plain("Chest"),
        });
        publish_test_world(&state, store);
    }

    let response = dispatch(
        ControlRequest {
            method: "world.client_inventory".to_string(),
            params: serde_json::Value::Null,
        },
        &state,
    );
    assert!(response.ok);
    let inv = response.result.unwrap();
    assert_eq!(
        inv["player_slots"][0]["item"]["item_name"],
        "minecraft:diamond"
    );
    assert_eq!(inv["player_slots"][0]["item"]["component_patch"], json!({}));
    assert_eq!(inv["open_container"]["menu_type"], "minecraft:generic_9x3");
}

#[test]
fn step2_control_writes_validate_and_queue_look_scroll_hold_and_wait() {
    let state = test_control_state();

    assert!(
        dispatch(
            ControlRequest {
                method: "input.focus".to_string(),
                params: json!({"focused": true, "capture_cursor": true}),
            },
            &state
        )
        .ok
    );
    assert!(
        dispatch(
            ControlRequest {
                method: "input.key".to_string(),
                params: json!({"key": "forward", "pressed": true, "hold_ticks": 12}),
            },
            &state
        )
        .ok
    );
    assert!(
        !dispatch(
            ControlRequest {
                method: "input.key".to_string(),
                params: json!({"key": "forward", "pressed": false, "hold_ticks": 12}),
            },
            &state
        )
        .ok
    );
    assert!(
        !dispatch(
            ControlRequest {
                method: "input.key".to_string(),
                params: json!({"key": "forward", "pressed": true, "hold_ticks": 0}),
            },
            &state
        )
        .ok
    );
    assert!(
        dispatch(
            ControlRequest {
                method: "input.mouse_button".to_string(),
                params: json!({"button": "left", "pressed": true, "hold_ticks": 20}),
            },
            &state
        )
        .ok
    );
    assert!(
        dispatch(
            ControlRequest {
                method: "input.scroll".to_string(),
                params: json!({"dy": -1.0}),
            },
            &state
        )
        .ok
    );
    assert!(
        dispatch(
            ControlRequest {
                method: "input.look".to_string(),
                params: json!({"yaw": 90.0, "pitch": -15.0}),
            },
            &state
        )
        .ok
    );
    assert!(
        dispatch(
            ControlRequest {
                method: "input.look_at".to_string(),
                params: json!({"x": 1.5, "y": 65.5, "z": 2.5}),
            },
            &state
        )
        .ok
    );
    assert!(
        dispatch(
            ControlRequest {
                method: "input.look_at".to_string(),
                params: json!({"entity_id": 77, "anchor": "eyes"}),
            },
            &state
        )
        .ok
    );
    assert!(
        !dispatch(
            ControlRequest {
                method: "input.look_at".to_string(),
                params: json!({"x": 1.0, "y": 2.0, "z": 3.0, "entity_id": 77}),
            },
            &state
        )
        .ok
    );
    assert!(
        dispatch(
            ControlRequest {
                method: "sim.wait_ticks".to_string(),
                params: json!({"ticks": 5}),
            },
            &state
        )
        .ok
    );
    assert!(
        !dispatch(
            ControlRequest {
                method: "sim.wait_ticks".to_string(),
                params: json!({"ticks": 101}),
            },
            &state
        )
        .ok
    );

    let recorded = state.requests.lock().unwrap();
    assert_eq!(
        recorded.input_requests,
        vec![
            InputControlRequest::FocusCapture {
                focused: true,
                capture_cursor: true,
            },
            InputControlRequest::KeyHold {
                key: InputKeyControl::Forward,
                hold_ticks: 12,
            },
            InputControlRequest::MouseButtonHold {
                button: InputMouseButtonControl::Left,
                hold_ticks: 20,
            },
            InputControlRequest::Scroll { dx: 0.0, dy: -1.0 },
            InputControlRequest::Look {
                yaw: 90.0,
                pitch: -15.0,
            },
            InputControlRequest::LookAt {
                target: LookAtTargetControl::Position {
                    x: 1.5,
                    y: 65.5,
                    z: 2.5,
                },
            },
            InputControlRequest::LookAt {
                target: LookAtTargetControl::Entity {
                    entity_id: 77,
                    anchor: LookAtAnchorControl::Eyes,
                },
            },
        ]
    );
    assert_eq!(recorded.wait_ticks_requests, vec![5]);
}
