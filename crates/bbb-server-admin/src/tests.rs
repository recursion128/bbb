mod catalog {
    use std::collections::BTreeSet;

    use bbb_protocol::difficulties::Difficulty;
    use bbb_protocol::game_rules::GameRuleValue;
    use bbb_protocol::game_types::GameType;
    use bbb_server_core::admin::{
        AdminRequest, AdminScope, AdminSetting, AdminSettings, PlayerRef, ADMIN_SETTING_KEYS,
        MAX_REQUEST_ENTRIES,
    };
    use bbb_server_core::commands::CommandPermission;
    use serde_json::{json, Value};
    use uuid::Uuid;

    use crate::{
        catalog, resource_by_uri, resources, resources_changed_by, setting_entry,
        setting_from_json, tool_by_name,
    };

    /// The `action()` match arms in the world's own request surface. Reading
    /// them from the source keeps the catalog from silently missing a request
    /// that was added there, which no in-crate test could notice.
    fn vanilla_owned_actions() -> BTreeSet<String> {
        const PROTOCOL: &str = include_str!("../../bbb-server-core/src/admin/protocol.rs");
        let body = PROTOCOL
            .split_once("pub fn action(&self) -> &'static str {")
            .expect("the request surface declares its audit actions")
            .1
            .split_once("\n    }")
            .expect("the action match ends")
            .0;
        body.lines()
            .filter_map(|line| line.split_once("=> \""))
            .filter_map(|(_, rest)| rest.split_once('"'))
            .map(|(action, _)| action.to_string())
            .collect()
    }

    #[test]
    fn every_request_the_world_owns_has_exactly_one_tool() {
        let actions = vanilla_owned_actions();
        assert!(actions.len() > 30, "{} actions parsed", actions.len());
        let tools = catalog()
            .iter()
            .map(|tool| tool.name.to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(tools, actions);
        assert_eq!(tools.len(), catalog().len(), "tool names are unique");
        for tool in catalog() {
            assert_eq!(tool.action(), tool.name, "{}", tool.name);
        }
    }

    #[test]
    fn a_tool_name_is_addressable_and_shaped_the_way_the_protocol_requires() {
        for tool in catalog() {
            assert!(
                tool.name
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric()
                        || matches!(character, '_' | '.' | '-')),
                "{}",
                tool.name
            );
            assert!(tool.name.len() <= 128, "{}", tool.name);
            assert!(std::ptr::eq(
                tool_by_name(tool.name).expect("addressable"),
                tool
            ));
            let descriptor = tool.descriptor();
            assert_eq!(descriptor["name"], tool.name);
            assert_eq!(descriptor["inputSchema"]["type"], "object");
            assert_eq!(descriptor["annotations"]["readOnlyHint"], tool.read_only());
            assert_eq!(descriptor["_meta"]["bbb/scope"], tool.scope().name());
        }
        assert!(tool_by_name("allowlist.destroy").is_none());
    }

    #[test]
    fn authority_comes_from_the_request_and_not_from_the_annotation() {
        // A read-only annotation and a read scope have to agree, in both
        // directions, or an annotation would be describing an authority the
        // server does not enforce.
        for tool in catalog() {
            assert_eq!(
                tool.read_only(),
                tool.scope() == AdminScope::Read,
                "{}",
                tool.name
            );
            assert!(
                !(tool.read_only() && tool.destructive),
                "{} cannot be a destructive read",
                tool.name
            );
        }
        assert_eq!(
            tool_by_name("command.execute").expect("tool").scope(),
            AdminScope::Command
        );
        assert_eq!(
            tool_by_name("server.stop").expect("tool").scope(),
            AdminScope::Manage
        );
    }

    fn decode(name: &str, arguments: Value) -> AdminRequest {
        (tool_by_name(name).expect("tool").decode)(&arguments)
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    fn refuse(name: &str, arguments: Value) -> String {
        (tool_by_name(name).expect("tool").decode)(&arguments)
            .err()
            .unwrap_or_else(|| panic!("{name} must refuse {arguments}"))
            .to_string()
    }

    #[test]
    fn readouts_with_no_parameters_accept_empty_arguments() {
        for tool in catalog() {
            if !tool.read_only() || (tool.schema)()["properties"] != json!({}) {
                continue;
            }
            assert_eq!((tool.decode)(&json!({})).ok(), Some((tool.example)()));
            assert_eq!((tool.decode)(&Value::Null).ok(), Some((tool.example)()));
            assert!(
                (tool.decode)(&json!({ "player": "x" })).is_err(),
                "{}",
                tool.name
            );
        }
    }

    #[test]
    fn world_queries_decode_their_position_arguments() {
        let dimension = "minecraft:overworld";
        let point = bbb_protocol::packets::BlockPos { x: -3, y: 70, z: 4 };
        assert_eq!(
            decode(
                "world.blocks",
                json!({ "dimension": dimension, "positions": [point] }),
            ),
            AdminRequest::WorldBlocks {
                dimension: dimension.to_owned(),
                positions: vec![point],
            }
        );
        assert_eq!(
            decode(
                "world.surface",
                json!({
                    "dimension": dimension, "minX": -3, "minZ": 4, "maxX": 9, "maxZ": 12,
                }),
            ),
            AdminRequest::WorldSurface {
                dimension: dimension.to_owned(),
                min_x: -3,
                min_z: 4,
                max_x: 9,
                max_z: 12,
            }
        );
        assert!(refuse(
            "world.blocks",
            json!({
                "dimension": dimension,
                "positions": [{ "x": 0, "y": 70, "z": i64::from(i32::MAX) + 1 }],
            }),
        )
        .contains("z is out of range"));
    }

    #[test]
    fn player_arguments_decode_by_id_by_name_or_by_both() {
        let id = Uuid::from_u128(9);
        assert_eq!(
            decode(
                "allowlist.add",
                json!({ "add": [{ "name": "bbb-client" }] })
            ),
            AdminRequest::AllowlistAdd(vec![PlayerRef::by_name("bbb-client")])
        );
        assert_eq!(
            decode(
                "allowlist.remove",
                json!({ "remove": [{ "id": id.to_string() }] })
            ),
            AdminRequest::AllowlistRemove(vec![PlayerRef::by_id(id)])
        );
        assert_eq!(
            decode(
                "allowlist.set",
                json!({ "players": [{ "id": id.to_string(), "name": "bbb-client" }] })
            ),
            AdminRequest::AllowlistSet(vec![PlayerRef {
                id: Some(id),
                name: Some("bbb-client".into()),
            }])
        );
        assert!(refuse("allowlist.add", json!({ "add": [{}] })).contains("named by id"));
        assert!(refuse("allowlist.add", json!({ "add": [{ "id": "nope" }] })).contains("UUID"));
        assert!(refuse("allowlist.add", json!({ "players": [] })).contains("no \"players\""));
        assert!(refuse("allowlist.add", json!({})).contains("required"));
    }

    #[test]
    fn a_ban_carries_its_reason_source_and_iso_instant() {
        assert_eq!(
            decode(
                "bans.add",
                json!({
                    "add": [{
                        "player": { "name": "bbb-client" },
                        "reason": "griefing",
                        "source": "ops",
                        "expires": "2026-08-17T10:12:27Z",
                    }],
                })
            ),
            AdminRequest::BansAdd(vec![bbb_server_core::admin::UserBanRequest {
                player: PlayerRef::by_name("bbb-client"),
                reason: Some("griefing".to_string()),
                source: Some("ops".to_string()),
                expires: Some(1_786_961_547_000),
            }])
        );
        assert!(refuse(
            "bans.add",
            json!({ "add": [{ "player": { "name": "a" }, "expires": "soon" }] })
        )
        .contains("instant"));
    }

    #[test]
    fn an_address_ban_names_a_player_or_a_literal() {
        assert_eq!(
            decode("ip_bans.add", json!({ "add": [{ "ip": "203.0.113.4" }] })),
            AdminRequest::IpBansAdd(vec![bbb_server_core::admin::IncomingIpBanRequest {
                player: None,
                ip: Some("203.0.113.4".to_string()),
                reason: None,
                source: None,
                expires: None,
            }])
        );
        assert!(
            refuse("ip_bans.add", json!({ "add": [{ "reason": "x" }] })).contains("names a player")
        );
        assert_eq!(
            decode("ip_bans.remove", json!({ "ip": ["203.0.113.4"] })),
            AdminRequest::IpBansRemove(vec!["203.0.113.4".to_string()])
        );
    }

    #[test]
    fn an_operator_entry_defaults_both_optional_fields() {
        assert_eq!(
            decode(
                "operators.add",
                json!({ "add": [{ "player": { "name": "a" } }] })
            ),
            AdminRequest::OperatorsAdd(vec![bbb_server_core::admin::OperatorRequest {
                player: PlayerRef::by_name("a"),
                permission_level: None,
                bypasses_player_limit: None,
            }])
        );
        assert_eq!(
            decode(
                "operators.set",
                json!({
                    "operators": [{
                        "player": { "name": "a" },
                        "permissionLevel": 4,
                        "bypassesPlayerLimit": true,
                    }],
                })
            ),
            AdminRequest::OperatorsSet(vec![bbb_server_core::admin::OperatorRequest {
                player: PlayerRef::by_name("a"),
                permission_level: Some(CommandPermission::Owner),
                bypasses_player_limit: Some(true),
            }])
        );
        assert!(refuse(
            "operators.add",
            json!({ "add": [{ "player": { "name": "a" }, "permissionLevel": 9 }] })
        )
        .contains("permission level"));
    }

    #[test]
    fn a_kick_and_a_system_message_share_the_message_shape() {
        assert_eq!(
            decode(
                "players.kick",
                json!({
                    "kick": [{
                        "player": { "name": "a" },
                        "message": { "translatable": "multiplayer.disconnect.kicked" },
                    }],
                })
            ),
            AdminRequest::KickPlayers(vec![bbb_server_core::admin::KickRequest {
                player: PlayerRef::by_name("a"),
                message: Some(bbb_server_core::admin::AdminMessage {
                    literal: None,
                    translatable: Some("multiplayer.disconnect.kicked".to_string()),
                    translatable_params: None,
                }),
            }])
        );
        assert_eq!(
            decode(
                "server.system_message",
                json!({
                    "message": {
                        "message": { "literal": "restarting" },
                        "overlay": true,
                        "receivingPlayers": [{ "name": "a" }],
                    },
                })
            ),
            AdminRequest::SystemMessage(bbb_server_core::admin::SystemMessageRequest {
                message: bbb_server_core::admin::AdminMessage::literal("restarting"),
                overlay: true,
                receiving_players: Some(vec![PlayerRef::by_name("a")]),
            })
        );
        assert!(refuse(
            "server.system_message",
            json!({ "message": { "message": { "literal": "x" } } })
        )
        .contains("overlay"));
    }

    #[test]
    fn save_stop_and_command_decode_their_own_arguments() {
        assert_eq!(
            decode("server.save", json!({})),
            AdminRequest::Save { flush: false }
        );
        assert_eq!(
            decode("server.save", json!({ "flush": true })),
            AdminRequest::Save { flush: true }
        );
        assert_eq!(
            decode("command.execute", json!({ "command": "list uuids" })),
            AdminRequest::ExecuteCommand {
                command: "list uuids".to_string()
            }
        );
        assert!(refuse("command.execute", json!({})).contains("required"));
        assert!(refuse("server.save", json!({ "flush": 1 })).contains("boolean"));
    }

    #[test]
    fn a_settings_write_round_trips_through_every_key_it_owns() {
        let settings = AdminSettings {
            difficulty: Difficulty::Hard,
            game_mode: GameType::Creative,
            enforce_allowlist: true,
            use_allowlist: true,
            max_players: 20,
            motd: "A BBB Server".to_string(),
            spawn_protection_radius: 16,
            operator_user_permission_level: CommandPermission::Admin,
            view_distance: 10,
            simulation_distance: 8,
            hide_online_players: true,
            status_replies: false,
            accepts_transfers: true,
            allow_flight: true,
            pause_when_empty_seconds: 0,
            entity_broadcast_range_percentage: 250,
        };
        for setting in settings.all() {
            let entry = setting_entry(&setting);
            assert_eq!(entry["key"], setting.key());
            assert_eq!(
                setting_from_json(setting.key(), &entry["value"]),
                Ok(setting.clone()),
                "{}",
                setting.key()
            );
            assert_eq!(
                decode(
                    "serversettings.set",
                    json!({ "key": setting.key(), "value": entry["value"] })
                ),
                AdminRequest::SetSetting(setting)
            );
        }
        assert_eq!(settings.all().len(), ADMIN_SETTING_KEYS.len());
        assert!(refuse(
            "serversettings.set",
            json!({ "key": "unknown_setting", "value": true })
        )
        .contains("not a setting"));
        assert!(refuse(
            "serversettings.set",
            json!({ "key": "difficulty", "value": "sunday" })
        )
        .contains("not a difficulty"));
        assert!(refuse(
            "serversettings.set",
            json!({ "key": "game_mode", "value": "hardcore" })
        )
        .contains("not a game mode"));
        assert!(refuse(
            "serversettings.set",
            json!({ "key": "max_players", "value": "many" })
        )
        .contains("integer"));
    }

    #[test]
    fn a_spawn_protection_radius_write_is_read_back_through_management() {
        use bbb_server_core::admin::AdminEnvelope;
        use bbb_server_core::mailbox::{world_command_mailbox, WorldMailboxLimits};
        use bbb_server_core::world::{ServerWorld, ServerWorldConfig, WorldCommand, WorldEvent};

        let mut world = ServerWorld::new(ServerWorldConfig {
            spawn_protection_radius: 12,
            ..ServerWorldConfig::default()
        })
        .unwrap();
        let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
        let mut call = |name: &str, arguments: Value, operation_id: u64| {
            let tool = tool_by_name(name).unwrap();
            sender
                .try_send(WorldCommand::Admin {
                    envelope: AdminEnvelope {
                        operation_id,
                        request_id: format!("radius-{operation_id}"),
                        principal: "agent".into(),
                        scope: tool.scope(),
                        deadline_tick: u64::MAX,
                        now: 0,
                    },
                    request: (tool.decode)(&arguments).unwrap(),
                })
                .unwrap();
            let tick = world.tick(receiver.drain());
            let result = tick
                .events
                .iter()
                .find_map(|event| match event {
                    WorldEvent::Admin(outcome) => Some(outcome.result.as_ref().unwrap()),
                    _ => None,
                })
                .unwrap();
            crate::result_value(result)
        };
        let radius = |answer: Value| {
            answer["settings"]
                .as_array()
                .unwrap()
                .iter()
                .find(|setting| setting["key"] == "spawn_protection_radius")
                .unwrap()["value"]
                .clone()
        };
        assert_eq!(radius(call("serversettings", json!({}), 1)), 12);
        assert_eq!(
            call(
                "serversettings.set",
                json!({ "key": "spawn_protection_radius", "value": 5 }),
                2
            ),
            json!({ "setting": { "key": "spawn_protection_radius", "value": 5 } })
        );
        assert_eq!(radius(call("serversettings", json!({}), 3)), 5);
    }

    #[test]
    fn a_game_mode_write_updates_default_game_mode_and_saved_gamemode() {
        use bbb_server_core::admin::AdminEnvelope;
        use bbb_server_core::mailbox::{world_command_mailbox, WorldMailboxLimits};
        use bbb_server_core::world::{ServerWorld, ServerWorldConfig, WorldCommand, WorldEvent};

        let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
        let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
        let call = |world: &mut ServerWorld, name: &str, arguments: Value, operation_id: u64| {
            let tool = tool_by_name(name).unwrap();
            sender
                .try_send(WorldCommand::Admin {
                    envelope: AdminEnvelope {
                        operation_id,
                        request_id: format!("gamemode-{operation_id}"),
                        principal: "agent".into(),
                        scope: tool.scope(),
                        deadline_tick: u64::MAX,
                        now: 0,
                    },
                    request: (tool.decode)(&arguments).unwrap(),
                })
                .unwrap();
            let tick = world.tick(receiver.drain());
            let result = tick
                .events
                .iter()
                .find_map(|event| match event {
                    WorldEvent::Admin(outcome) => Some(outcome.result.as_ref().unwrap()),
                    _ => None,
                })
                .unwrap();
            crate::result_value(result)
        };
        let game_mode = |answer: Value| {
            answer["settings"]
                .as_array()
                .unwrap()
                .iter()
                .find(|setting| setting["key"] == "game_mode")
                .unwrap()["value"]
                .clone()
        };
        assert_eq!(
            game_mode(call(&mut world, "serversettings", json!({}), 1)),
            "survival"
        );
        assert_eq!(
            call(
                &mut world,
                "serversettings.set",
                json!({ "key": "game_mode", "value": "adventure" }),
                2
            ),
            json!({ "setting": { "key": "game_mode", "value": "adventure" } })
        );
        assert_eq!(
            game_mode(call(&mut world, "serversettings", json!({}), 3)),
            "adventure"
        );
        assert_eq!(
            world.persisted_settings().gamemode,
            Some(GameType::Adventure)
        );

        call(
            &mut world,
            "command.execute",
            json!({ "command": "defaultgamemode spectator" }),
            4,
        );
        assert_eq!(
            game_mode(call(&mut world, "serversettings", json!({}), 5)),
            "spectator"
        );
        assert_eq!(
            world.persisted_settings().gamemode,
            Some(GameType::Adventure)
        );
    }

    #[test]
    fn a_game_rule_update_takes_the_type_the_rule_declares() {
        assert_eq!(
            decode(
                "gamerules.update",
                json!({ "gamerule": { "key": "minecraft:advance_time", "value": false } })
            ),
            AdminRequest::UpdateGameRule {
                key: "minecraft:advance_time".to_string(),
                value: GameRuleValue::Boolean(false),
            }
        );
        assert_eq!(
            decode(
                "gamerules.update",
                json!({ "gamerule": { "key": "minecraft:random_tick_speed", "value": 7 } })
            ),
            AdminRequest::UpdateGameRule {
                key: "minecraft:random_tick_speed".to_string(),
                value: GameRuleValue::Integer(7),
            }
        );
        assert!(refuse(
            "gamerules.update",
            json!({ "gamerule": { "key": "minecraft:random_tick_speed", "value": true } })
        )
        .contains("integer"));
        assert!(refuse(
            "gamerules.update",
            json!({ "gamerule": { "key": "minecraft:not_a_rule", "value": true } })
        )
        .contains("not a game rule"));
    }

    #[test]
    fn an_over_bounded_list_is_refused_before_it_reaches_the_world() {
        let players = (0..=MAX_REQUEST_ENTRIES)
            .map(|index| json!({ "name": format!("player{index}") }))
            .collect::<Vec<_>>();
        assert!(refuse("allowlist.add", json!({ "add": players })).contains("at most"));
    }

    #[test]
    fn every_resource_reads_through_a_read_scope_request() {
        for resource in resources() {
            let request = (resource.request)();
            assert_eq!(
                request.required_scope(),
                AdminScope::Read,
                "{}",
                resource.uri
            );
            assert!(resource.uri.starts_with("bbb://server/"));
            assert!(std::ptr::eq(
                resource_by_uri(resource.uri).expect("addressable"),
                resource
            ));
            assert_eq!(resource.descriptor()["mimeType"], "application/json");
            let tool = tool_by_name(request.action()).expect("corresponding tool");
            assert_eq!((tool.example)(), request);
            assert_eq!((tool.schema)()["properties"], json!({}));
        }
        assert!(resource_by_uri("bbb://server/secrets").is_none());
    }

    #[test]
    fn an_applied_mutation_names_the_resources_it_may_have_changed() {
        assert_eq!(
            resources_changed_by("allowlist.add"),
            vec!["bbb://server/allowlist", "bbb://server/diagnostics"]
        );
        assert_eq!(
            resources_changed_by("players.kick"),
            vec![
                "bbb://server/players",
                "bbb://server/status",
                "bbb://server/diagnostics",
            ]
        );
        assert_eq!(
            resources_changed_by("server.save"),
            vec!["bbb://server/diagnostics"]
        );
        // A command can reach anything a typed tool can, so it invalidates
        // every resource rather than guessing which one it wrote.
        assert_eq!(
            resources_changed_by("command.execute").len(),
            resources().len()
        );
        for tool in catalog() {
            let changed = resources_changed_by(tool.name);
            assert!(!changed.is_empty(), "{}", tool.name);
            for uri in changed {
                assert!(resource_by_uri(uri).is_some(), "{uri}");
            }
        }
    }

    #[test]
    fn the_answer_shape_carries_the_vanilla_field_names() {
        use bbb_server_core::admin::{DiagnosticsReport, ServerStateReport};

        let status = crate::result_value(&bbb_server_core::admin::AdminResult::Status(
            ServerStateReport {
                started: true,
                players: vec![PlayerRef::by_name("a")],
                version_name: "26.1",
                version_protocol: 800,
            },
        ));
        assert_eq!(status["status"]["started"], true);
        assert_eq!(status["status"]["players"][0]["name"], "a");
        assert_eq!(status["status"]["version"]["protocol"], 800);

        let diagnostics = crate::result_value(&bbb_server_core::admin::AdminResult::Diagnostics(
            DiagnosticsReport {
                tick: 5,
                online_players: 1,
                max_players: 20,
                loaded_chunks: 9,
                changed_blocks: 0,
                allowlist_entries: 0,
                operator_entries: 1,
                player_ban_entries: 0,
                ip_ban_entries: 0,
                processed_admin_operations: 3,
                stopping: false,
            },
        ));
        assert_eq!(diagnostics["diagnostics"]["tick"], 5);
        assert_eq!(diagnostics["diagnostics"]["processedAdminOperations"], 3);

        let setting = crate::result_value(&bbb_server_core::admin::AdminResult::Setting(
            AdminSetting::Difficulty(Difficulty::Peaceful),
        ));
        assert_eq!(setting["setting"]["key"], "difficulty");
        assert_eq!(setting["setting"]["value"], "peaceful");

        let mut ban = bbb_server_core::admin::UserBanReport {
            player: PlayerRef::by_name("bob"),
            ban: bbb_server_core::admin::BanRecord {
                created: 0,
                source: "alice".into(),
                expires: None,
                reason: Some("blocked alice".into()),
            },
        };
        assert_eq!(crate::user_ban_value(&ban)["reason"], "blocked alice");
        let raw = "griefing Stand "
            .encode_utf16()
            .chain([0xd800])
            .collect::<Vec<_>>();
        ban.ban.reason = Some(raw.clone().into());
        ban.ban.source = raw.into();
        assert_eq!(
            crate::user_ban_value(&ban)["source"],
            "griefing Stand \u{fffd}"
        );
        assert_eq!(
            crate::user_ban_value(&ban)["reason"],
            "griefing Stand \u{fffd}"
        );
        let component = bbb_protocol::RichTextComponent::try_from_component_nbt(
            ban.ban.reason.as_ref().unwrap().to_nbt(),
        )
        .unwrap();
        let value = crate::component_value(&component);
        assert_eq!(value["text"], "griefing Stand \u{fffd}");
        assert_eq!(
            value["component"],
            serde_json::json!({"type": "String", "value": "griefing Stand \u{fffd}"})
        );
        assert_eq!(
            ban.ban.reason.as_ref().unwrap().units().last(),
            Some(&0xd800)
        );
    }
}

mod transport {
    use std::{
        net::SocketAddr,
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc, Mutex,
        },
        time::Duration,
    };

    use bbb_server_core::admin::{
        AdminError, AdminOutcome, AdminRequest, AdminResult, AdminScope, PlayerRef,
    };
    use serde_json::{json, Value};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        sync::{broadcast, oneshot},
        time::timeout,
    };

    use crate::{
        AdminCall, AdminGateway, AuthConfig, GatewayError, McpServer, McpServerConfig,
        PendingOperation, TokenGrant, OPERATION_URI_PREFIX,
    };
    use bbb_mcp::{
        ResourceChange, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND,
        PROTOCOL_VERSION, RESOURCE_NOT_FOUND, SESSION_HEADER,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Mode {
        Answer,
        Withhold,
        Busy,
    }

    struct TestGateway {
        mode: Mode,
        next_operation: AtomicU64,
        calls: Mutex<Vec<AdminCall>>,
        withheld: Mutex<Vec<(u64, oneshot::Sender<AdminOutcome>)>>,
        changes: broadcast::Sender<ResourceChange>,
    }

    impl TestGateway {
        fn new(mode: Mode) -> Arc<Self> {
            Arc::new(Self {
                mode,
                next_operation: AtomicU64::new(1),
                calls: Mutex::new(Vec::new()),
                withheld: Mutex::new(Vec::new()),
                changes: broadcast::channel(8).0,
            })
        }

        fn outcome(operation_id: u64, call: &AdminCall) -> AdminOutcome {
            let required = call.request.required_scope();
            let result = if call.scope < required {
                Err(AdminError::ScopeDenied {
                    required,
                    granted: call.scope,
                })
            } else {
                Ok(AdminResult::Players(vec![PlayerRef::by_name("bbb-client")]))
            };
            AdminOutcome {
                operation_id,
                request_id: call.request_id.clone(),
                principal: call.principal.clone(),
                scope: call.scope,
                action: call.request.action(),
                tick: 42,
                replayed: false,
                result,
            }
        }

        fn answer_withheld(&self) {
            let withheld = std::mem::take(&mut *self.withheld.lock().expect("withheld"));
            for (operation_id, sender) in withheld {
                let call = self.calls.lock().expect("calls")[0].clone();
                let _ = sender.send(Self::outcome(operation_id, &call));
            }
        }
    }

    impl AdminGateway for TestGateway {
        fn submit(&self, call: AdminCall) -> Result<PendingOperation, GatewayError> {
            if self.mode == Mode::Busy {
                return Err(GatewayError::Busy);
            }
            let operation_id = self.next_operation.fetch_add(1, Ordering::Relaxed);
            self.calls.lock().expect("calls").push(call.clone());
            let (sender, receiver) = oneshot::channel();
            match self.mode {
                Mode::Answer => {
                    let _ = sender.send(Self::outcome(operation_id, &call));
                }
                Mode::Withhold => self
                    .withheld
                    .lock()
                    .expect("withheld")
                    .push((operation_id, sender)),
                Mode::Busy => unreachable!("a busy gateway never submits"),
            }
            Ok(PendingOperation {
                operation_id,
                answer: receiver,
            })
        }

        fn now(&self) -> i64 {
            1_755_000_000_000
        }

        fn subscribe(&self) -> broadcast::Receiver<ResourceChange> {
            self.changes.subscribe()
        }
    }

    fn loopback() -> SocketAddr {
        "127.0.0.1:52000".parse().expect("address")
    }

    fn remote() -> SocketAddr {
        "203.0.113.7:52000".parse().expect("address")
    }

    fn server(gateway: Arc<TestGateway>) -> Arc<McpServer> {
        McpServer::new(McpServerConfig::default(), gateway)
    }

    struct RawResponse {
        status: u16,
        headers: Vec<(String, String)>,
        body: String,
    }

    impl RawResponse {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(header, _)| header.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
        }

        fn json(&self) -> Value {
            serde_json::from_str(&self.body)
                .unwrap_or_else(|error| panic!("body is JSON: {error}; body was {:?}", self.body))
        }

        fn result(&self) -> Value {
            let body = self.json();
            assert!(
                body.get("error").is_none(),
                "expected a result, got {}",
                self.body
            );
            body["result"].clone()
        }

        fn error_code(&self) -> i64 {
            self.json()["error"]["code"]
                .as_i64()
                .unwrap_or_else(|| panic!("expected an error, got {}", self.body))
        }
    }

    async fn exchange(server: &Arc<McpServer>, peer: SocketAddr, raw: String) -> RawResponse {
        let (mut client, service) = tokio::io::duplex(256 * 1024);
        let handler = tokio::spawn(Arc::clone(server).handle_connection(service, peer));
        client.write_all(raw.as_bytes()).await.expect("write");
        let mut received = Vec::new();
        timeout(Duration::from_secs(5), client.read_to_end(&mut received))
            .await
            .expect("the answer arrives")
            .expect("read");
        handler.await.expect("the connection task finishes");
        parse_response(&received)
    }

    fn parse_response(bytes: &[u8]) -> RawResponse {
        let text = String::from_utf8_lossy(bytes).to_string();
        let (head, body) = text.split_once("\r\n\r\n").expect("a response head");
        let mut lines = head.split("\r\n");
        let status_line = lines.next().expect("a status line");
        let status = status_line
            .split(' ')
            .nth(1)
            .and_then(|status| status.parse::<u16>().ok())
            .expect("a status code");
        let headers = lines
            .filter_map(|line| line.split_once(": "))
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        RawResponse {
            status,
            headers,
            body: body.to_string(),
        }
    }

    fn post(body: &Value, session: Option<&str>, extra: &[(&str, &str)]) -> String {
        let body = body.to_string();
        let mut request = String::from("POST /mcp HTTP/1.1\r\nhost: 127.0.0.1\r\n");
        request.push_str("accept: application/json, text/event-stream\r\n");
        request.push_str("content-type: application/json\r\n");
        request.push_str("connection: close\r\n");
        if let Some(session) = session {
            request.push_str(&format!("{SESSION_HEADER}: {session}\r\n"));
        }
        for (name, value) in extra {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str(&format!("content-length: {}\r\n\r\n", body.len()));
        request.push_str(&body);
        request
    }

    fn call(id: i64, method: &str, params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
    }

    async fn initialize(server: &Arc<McpServer>) -> String {
        let response = exchange(
            server,
            loopback(),
            post(
                &call(
                    1,
                    "initialize",
                    json!({
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {},
                        "clientInfo": { "name": "bbb-admin", "version": "0.1.0" },
                    }),
                ),
                None,
                &[],
            ),
        )
        .await;
        assert_eq!(response.status, 200, "{}", response.body);
        let session = response
            .header(SESSION_HEADER)
            .expect("a session id")
            .to_string();
        let ready = exchange(
            server,
            loopback(),
            post(
                &json!({ "jsonrpc": "2.0", "method": "notifications/initialized", "params": {} }),
                Some(&session),
                &[],
            ),
        )
        .await;
        assert_eq!(ready.status, 202, "{}", ready.body);
        session
    }

    #[tokio::test]
    async fn the_handshake_answers_the_pinned_revision_and_assigns_a_session() {
        let server = server(TestGateway::new(Mode::Answer));
        let response = exchange(
            &server,
            loopback(),
            post(
                &call(
                    1,
                    "initialize",
                    json!({ "protocolVersion": "1999-01-01", "capabilities": {} }),
                ),
                None,
                &[],
            ),
        )
        .await;
        let result = response.result();
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(result["capabilities"]["resources"]["subscribe"], true);
        assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
        assert_eq!(result["serverInfo"]["name"], "bbb-server");
        assert_eq!(result["_meta"]["bbb/scope"], "manage");
        assert_eq!(
            result["_meta"]["bbb/requestedProtocolVersion"],
            "1999-01-01"
        );
        let session = response.header(SESSION_HEADER).expect("a session id");
        assert_eq!(session.len(), 32, "128 bits of session id");
        assert!(session
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
        assert_eq!(server.sessions().len(), 1);
    }

    #[tokio::test]
    async fn a_session_is_required_known_and_owned_by_its_principal() {
        let server = server(TestGateway::new(Mode::Answer));
        let session = initialize(&server).await;

        let missing = exchange(
            &server,
            loopback(),
            post(&call(2, "ping", json!({})), None, &[]),
        )
        .await;
        assert_eq!(missing.status, 400);

        let unknown = exchange(
            &server,
            loopback(),
            post(&call(2, "ping", json!({})), Some("deadbeef"), &[]),
        )
        .await;
        assert_eq!(unknown.status, 404);

        let wrong_version = exchange(
            &server,
            loopback(),
            post(
                &call(2, "ping", json!({})),
                Some(&session),
                &[("mcp-protocol-version", "2024-11-05")],
            ),
        )
        .await;
        assert_eq!(wrong_version.status, 400);

        let accepted = exchange(
            &server,
            loopback(),
            post(
                &call(2, "ping", json!({})),
                Some(&session),
                &[("mcp-protocol-version", PROTOCOL_VERSION)],
            ),
        )
        .await;
        assert_eq!(accepted.result(), json!({}));
    }

    #[tokio::test]
    async fn ending_a_session_makes_every_later_request_start_over() {
        let server = server(TestGateway::new(Mode::Answer));
        let session = initialize(&server).await;
        let mut request = String::from("DELETE /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n");
        request.push_str(&format!("{SESSION_HEADER}: {session}\r\n\r\n"));
        let deleted = exchange(&server, loopback(), request).await;
        assert_eq!(deleted.status, 204);
        assert!(server.sessions().is_empty());
        let after = exchange(
            &server,
            loopback(),
            post(&call(3, "ping", json!({})), Some(&session), &[]),
        )
        .await;
        assert_eq!(after.status, 404);
    }

    #[tokio::test]
    async fn a_tool_call_reaches_the_gateway_and_answers_structured_content() {
        let gateway = TestGateway::new(Mode::Answer);
        let server = server(Arc::clone(&gateway));
        let session = initialize(&server).await;
        let response = exchange(
            &server,
            loopback(),
            post(
                &call(
                    2,
                    "tools/call",
                    json!({ "name": "players", "arguments": {} }),
                ),
                Some(&session),
                &[],
            ),
        )
        .await;
        let result = response.result();
        assert_eq!(result["isError"], false);
        assert_eq!(
            result["structuredContent"]["players"][0]["name"],
            "bbb-client"
        );
        assert_eq!(result["content"][0]["type"], "text");
        assert_eq!(result["_meta"]["bbb/tick"], 42);
        let calls = gateway.calls.lock().expect("calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].request, AdminRequest::Players);
        assert_eq!(calls[0].principal, "loopback");
        assert_eq!(calls[0].scope, AdminScope::Manage);
        assert_eq!(calls[0].request_id, format!("{session}:2"));
    }

    #[tokio::test]
    async fn a_caller_supplied_request_id_is_what_makes_a_retry_the_same_operation() {
        let gateway = TestGateway::new(Mode::Answer);
        let server = server(Arc::clone(&gateway));
        let session = initialize(&server).await;
        for id in 2..4 {
            let response = exchange(
                &server,
                loopback(),
                post(
                    &call(
                        id,
                        "tools/call",
                        json!({
                            "name": "players",
                            "arguments": {},
                            "_meta": { "bbb/requestId": "restart-1" },
                        }),
                    ),
                    Some(&session),
                    &[],
                ),
            )
            .await;
            assert_eq!(response.result()["isError"], false);
        }
        let calls = gateway.calls.lock().expect("calls");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].request_id, "restart-1");
        assert_eq!(calls[1].request_id, "restart-1");
    }

    #[tokio::test]
    async fn a_refusal_the_world_decided_is_content_and_a_protocol_fault_is_an_error() {
        let gateway = TestGateway::new(Mode::Answer);
        let config = McpServerConfig {
            auth: AuthConfig {
                loopback_scope: Some(AdminScope::Read),
                ..AuthConfig::default()
            },
            ..McpServerConfig::default()
        };
        let server = McpServer::new(config, gateway);
        let session = initialize(&server).await;

        let refused = exchange(
            &server,
            loopback(),
            post(
                &call(
                    2,
                    "tools/call",
                    json!({ "name": "allowlist.clear", "arguments": {} }),
                ),
                Some(&session),
                &[],
            ),
        )
        .await;
        let result = refused.result();
        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .expect("text")
                .contains("does not satisfy"),
            "{result}"
        );

        let unknown_tool = exchange(
            &server,
            loopback(),
            post(
                &call(3, "tools/call", json!({ "name": "allowlist.destroy" })),
                Some(&session),
                &[],
            ),
        )
        .await;
        assert_eq!(unknown_tool.error_code(), i64::from(INVALID_PARAMS));

        let bad_arguments = exchange(
            &server,
            loopback(),
            post(
                &call(
                    4,
                    "tools/call",
                    json!({ "name": "allowlist.add", "arguments": { "add": [{}] } }),
                ),
                Some(&session),
                &[],
            ),
        )
        .await;
        assert_eq!(bad_arguments.error_code(), i64::from(INVALID_PARAMS));

        let unknown_method = exchange(
            &server,
            loopback(),
            post(&call(5, "prompts/list", json!({})), Some(&session), &[]),
        )
        .await;
        assert_eq!(unknown_method.error_code(), i64::from(METHOD_NOT_FOUND));
    }

    #[tokio::test]
    async fn the_listings_are_the_catalog_and_the_resource_table() {
        let server = server(TestGateway::new(Mode::Answer));
        let session = initialize(&server).await;
        let tools = exchange(
            &server,
            loopback(),
            post(&call(2, "tools/list", json!({})), Some(&session), &[]),
        )
        .await;
        let listed = tools.result()["tools"].as_array().expect("tools").len();
        assert_eq!(listed, crate::catalog().len());

        let resources = exchange(
            &server,
            loopback(),
            post(&call(3, "resources/list", json!({})), Some(&session), &[]),
        )
        .await;
        assert_eq!(
            resources.result()["resources"]
                .as_array()
                .expect("resources")
                .len(),
            crate::resources().len()
        );

        let templates = exchange(
            &server,
            loopback(),
            post(
                &call(4, "resources/templates/list", json!({})),
                Some(&session),
                &[],
            ),
        )
        .await;
        assert_eq!(
            templates.result()["resourceTemplates"][0]["uriTemplate"],
            format!("{OPERATION_URI_PREFIX}{{operationId}}")
        );
    }

    #[tokio::test]
    async fn reading_a_resource_goes_through_the_same_gateway_a_tool_does() {
        let gateway = TestGateway::new(Mode::Answer);
        let server = server(Arc::clone(&gateway));
        let session = initialize(&server).await;
        let response = exchange(
            &server,
            loopback(),
            post(
                &call(
                    2,
                    "resources/read",
                    json!({ "uri": "bbb://server/players" }),
                ),
                Some(&session),
                &[],
            ),
        )
        .await;
        let contents = response.result();
        assert_eq!(contents["contents"][0]["uri"], "bbb://server/players");
        assert_eq!(contents["contents"][0]["mimeType"], "application/json");
        assert!(contents["contents"][0]["text"]
            .as_str()
            .expect("text")
            .contains("bbb-client"));
        assert_eq!(
            gateway.calls.lock().expect("calls")[0].request,
            AdminRequest::Players
        );

        let missing = exchange(
            &server,
            loopback(),
            post(
                &call(
                    3,
                    "resources/read",
                    json!({ "uri": "bbb://server/secrets" }),
                ),
                Some(&session),
                &[],
            ),
        )
        .await;
        assert_eq!(missing.error_code(), i64::from(RESOURCE_NOT_FOUND));
    }

    #[tokio::test]
    async fn a_deadline_leaves_the_work_running_and_the_answer_readable_by_id() {
        let gateway = TestGateway::new(Mode::Withhold);
        let config = McpServerConfig {
            request_timeout: Duration::from_millis(50),
            ..McpServerConfig::default()
        };
        let server = McpServer::new(config, Arc::clone(&gateway) as Arc<dyn AdminGateway>);
        let session = initialize(&server).await;
        let deferred = exchange(
            &server,
            loopback(),
            post(
                &call(2, "tools/call", json!({ "name": "players" })),
                Some(&session),
                &[],
            ),
        )
        .await;
        let result = deferred.result();
        assert_eq!(result["isError"], true);
        assert_eq!(
            result["_meta"]["bbb/deferred"]["operationUri"],
            format!("{OPERATION_URI_PREFIX}1")
        );

        let pending = exchange(
            &server,
            loopback(),
            post(
                &call(
                    3,
                    "resources/read",
                    json!({ "uri": format!("{OPERATION_URI_PREFIX}1") }),
                ),
                Some(&session),
                &[],
            ),
        )
        .await;
        let record: Value = serde_json::from_str(
            pending.result()["contents"][0]["text"]
                .as_str()
                .expect("text"),
        )
        .expect("the record is JSON");
        assert_eq!(record["state"], "pending");
        assert_eq!(record["action"], "players");

        gateway.answer_withheld();
        tokio::task::yield_now().await;
        let applied = exchange(
            &server,
            loopback(),
            post(
                &call(
                    4,
                    "resources/read",
                    json!({ "uri": format!("{OPERATION_URI_PREFIX}1") }),
                ),
                Some(&session),
                &[],
            ),
        )
        .await;
        let record: Value = serde_json::from_str(
            applied.result()["contents"][0]["text"]
                .as_str()
                .expect("text"),
        )
        .expect("the record is JSON");
        assert_eq!(record["state"], "applied");
        assert_eq!(record["outcome"]["tick"], 42);
        assert_eq!(
            record["outcome"]["result"]["players"][0]["name"],
            "bbb-client"
        );
    }

    #[tokio::test]
    async fn a_full_mailbox_is_an_attributable_refusal_rather_than_a_wait() {
        let server = server(TestGateway::new(Mode::Busy));
        let session = initialize(&server).await;
        let response = exchange(
            &server,
            loopback(),
            post(
                &call(2, "tools/call", json!({ "name": "players" })),
                Some(&session),
                &[],
            ),
        )
        .await;
        assert_eq!(response.error_code(), i64::from(INTERNAL_ERROR));
        assert_eq!(response.json()["error"]["data"]["retryable"], true);
    }

    #[tokio::test]
    async fn a_subscription_delivers_the_change_the_runtime_published() {
        let gateway = TestGateway::new(Mode::Answer);
        let server = server(Arc::clone(&gateway));
        let fanout = server.spawn_change_fanout();
        let session = initialize(&server).await;
        let subscribed = exchange(
            &server,
            loopback(),
            post(
                &call(
                    2,
                    "resources/subscribe",
                    json!({ "uri": "bbb://server/players" }),
                ),
                Some(&session),
                &[],
            ),
        )
        .await;
        assert_eq!(subscribed.result(), json!({}));

        let (mut client, service) = tokio::io::duplex(64 * 1024);
        let stream_server = Arc::clone(&server);
        let stream = tokio::spawn(stream_server.handle_connection(service, loopback()));
        let mut request =
            String::from("GET /mcp HTTP/1.1\r\nhost: h\r\naccept: text/event-stream\r\n");
        request.push_str(&format!("{SESSION_HEADER}: {session}\r\n\r\n"));
        client.write_all(request.as_bytes()).await.expect("write");

        let mut received = String::new();
        let mut buffer = [0u8; 4096];
        // The first read drains the response head, the second the event.
        loop {
            let read = timeout(Duration::from_secs(5), client.read(&mut buffer))
                .await
                .expect("the stream answers")
                .expect("read");
            received.push_str(&String::from_utf8_lossy(&buffer[..read]));
            if received.contains("content-type: text/event-stream") {
                break;
            }
        }
        gateway
            .changes
            .send(ResourceChange {
                uri: "bbb://server/players",
            })
            .expect("publish");
        loop {
            let read = timeout(Duration::from_secs(5), client.read(&mut buffer))
                .await
                .expect("the notification arrives")
                .expect("read");
            received.push_str(&String::from_utf8_lossy(&buffer[..read]));
            if received.contains("notifications/resources/updated") {
                break;
            }
        }
        assert!(received.contains("bbb://server/players"), "{received}");
        assert!(received.contains("event: message"), "{received}");

        // A second stream on one session is refused rather than duplicated.
        let second = exchange(&server, loopback(), {
            let mut request =
                String::from("GET /mcp HTTP/1.1\r\nhost: h\r\naccept: text/event-stream\r\n");
            request.push_str("connection: close\r\n");
            request.push_str(&format!("{SESSION_HEADER}: {session}\r\n\r\n"));
            request
        })
        .await;
        assert_eq!(second.status, 409);

        server.sessions().remove(&session);
        drop(client);
        let _ = timeout(Duration::from_secs(5), stream).await;
        fanout.abort();
    }

    #[tokio::test]
    async fn a_credential_is_required_off_the_loopback_and_names_the_principal() {
        let gateway = TestGateway::new(Mode::Answer);
        let config = McpServerConfig {
            auth: AuthConfig {
                loopback_scope: None,
                tokens: vec![TokenGrant {
                    principal: "release-agent".to_string(),
                    scope: AdminScope::Read,
                    secret: "s3cret-token".to_string(),
                }],
                ..AuthConfig::default()
            },
            ..McpServerConfig::default()
        };
        let server = McpServer::new(config, Arc::clone(&gateway) as Arc<dyn AdminGateway>);

        let anonymous = exchange(
            &server,
            remote(),
            post(&call(1, "initialize", json!({})), None, &[]),
        )
        .await;
        assert_eq!(anonymous.status, 401);
        assert_eq!(anonymous.header("www-authenticate"), Some("Bearer"));

        let wrong = exchange(
            &server,
            remote(),
            post(
                &call(1, "initialize", json!({})),
                None,
                &[("authorization", "Bearer wrong-token")],
            ),
        )
        .await;
        assert_eq!(wrong.status, 401);

        let accepted = exchange(
            &server,
            remote(),
            post(
                &call(1, "initialize", json!({})),
                None,
                &[("authorization", "Bearer s3cret-token")],
            ),
        )
        .await;
        assert_eq!(accepted.status, 200);
        assert_eq!(accepted.result()["_meta"]["bbb/principal"], "release-agent");
        assert_eq!(accepted.result()["_meta"]["bbb/scope"], "read");
        let session = accepted
            .header(SESSION_HEADER)
            .expect("session")
            .to_string();

        // Another principal cannot borrow the session.
        let stolen = exchange(
            &server,
            remote(),
            post(&call(2, "ping", json!({})), Some(&session), &[]),
        )
        .await;
        assert_eq!(stolen.status, 401);
    }

    #[tokio::test]
    async fn a_browser_origin_is_refused_unless_it_was_declared() {
        let server = server(TestGateway::new(Mode::Answer));
        let refused = exchange(
            &server,
            loopback(),
            post(
                &call(1, "initialize", json!({})),
                None,
                &[("origin", "https://evil.example")],
            ),
        )
        .await;
        assert_eq!(refused.status, 403);

        let config = McpServerConfig {
            auth: AuthConfig {
                allowed_origins: vec!["https://console.example".to_string()],
                ..AuthConfig::default()
            },
            ..McpServerConfig::default()
        };
        let server = McpServer::new(config, TestGateway::new(Mode::Answer));
        let accepted = exchange(
            &server,
            loopback(),
            post(
                &call(1, "initialize", json!({})),
                None,
                &[("origin", "https://console.example")],
            ),
        )
        .await;
        assert_eq!(accepted.status, 200);
    }

    #[tokio::test]
    async fn the_endpoint_refuses_what_it_does_not_serve() {
        let server = server(TestGateway::new(Mode::Answer));
        let elsewhere = exchange(
            &server,
            loopback(),
            "GET /admin HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n".to_string(),
        )
        .await;
        assert_eq!(elsewhere.status, 404);

        let put = exchange(
            &server,
            loopback(),
            "PUT /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n".to_string(),
        )
        .await;
        assert_eq!(put.status, 405);
        assert_eq!(put.header("allow"), Some("GET, POST, DELETE, OPTIONS"));

        let options = exchange(
            &server,
            loopback(),
            "OPTIONS /mcp HTTP/1.1\r\nhost: h\r\nconnection: close\r\n\r\n".to_string(),
        )
        .await;
        assert_eq!(options.status, 204);

        let batch = exchange(&server, loopback(), {
            let body = "[{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}]";
            format!(
                "POST /mcp HTTP/1.1\r\nhost: h\r\ncontent-type: application/json\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            )
        })
        .await;
        assert_eq!(batch.status, 400);
        assert_eq!(batch.error_code(), i64::from(INVALID_REQUEST));

        let wrong_media = exchange(&server, loopback(), {
            let body = "{}";
            format!(
                "POST /mcp HTTP/1.1\r\nhost: h\r\ncontent-type: text/plain\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            )
        })
        .await;
        assert_eq!(wrong_media.status, 415);

        let notification = exchange(
            &server,
            loopback(),
            post(
                &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
                None,
                &[],
            ),
        )
        .await;
        assert_eq!(notification.status, 202);
    }
}
