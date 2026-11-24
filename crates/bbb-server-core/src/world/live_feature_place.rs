//! Live `/place`, end platform and gateway placement through the level's
//! `WorldGenLevel`.

mod structures;
pub(in crate::world) use structures::{
    place_live_materialized_template, template_world_chunks, LiveTemplateSettings,
};

use crate::random::GameRandom;
use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::worldgen::{ConfiguredFeature, EndGatewayConfig};
use bbb_server_worldgen::place_configured_feature;

use crate::world::simulation_level::LevelCtx;
use crate::world::{LevelId, ServerWorld};

impl ServerWorld {
    pub(super) fn create_end_spawn_platform(&mut self, dimension: LevelId, origin: BlockPos) {
        self.with_level(dimension, |level| {
            bbb_server_worldgen::create_end_platform(level, origin, true)
        })
    }

    pub(super) fn place_gateway_island(&mut self, dimension: LevelId, origin: BlockPos) {
        let Some(feature) = self.configured_feature("minecraft:end_island") else {
            return;
        };
        let packed = ((i64::from(origin.x) & 0x3ffffff) << 38)
            | ((i64::from(origin.z) & 0x3ffffff) << 12)
            | (i64::from(origin.y) & 0xfff);
        let random = GameRandom::with_seed(packed);
        self.with_level(dimension, |level| {
            place_configured_feature(&feature, level, origin, &mut &random)
        });
    }

    pub(super) fn create_gateway_return(
        &mut self,
        dimension: LevelId,
        origin: BlockPos,
        exit: BlockPos,
    ) {
        let feature = ConfiguredFeature::EndGateway(EndGatewayConfig {
            exit: Some(exit),
            exact: false,
        });
        self.with_live_placement(dimension, |level, random| {
            place_configured_feature(&feature, level, origin, random)
        });
    }

    pub(crate) fn place_live_configured_feature(
        &mut self,
        dimension: LevelId,
        feature_id: &str,
        origin: BlockPos,
    ) -> bool {
        let Some(feature) = self.configured_feature(feature_id) else {
            return false;
        };
        self.with_live_placement(dimension, |level, random| {
            place_configured_feature(&feature, level, origin, random)
        })
    }

    fn configured_feature(&self, id: &str) -> Option<std::sync::Arc<ConfiguredFeature>> {
        let worldgen = &self.datapacks()?.registries.worldgen;
        worldgen.configured_features.get(id).cloned()
    }

    /// One level frame placing with the level's own random.
    fn with_live_placement<T>(
        &mut self,
        dimension: LevelId,
        place: impl FnOnce(&mut LevelCtx<'_, '_>, &mut dyn RandomSource) -> T,
    ) -> T {
        self.with_level(dimension, |level| {
            let mut random = level.level_random();
            place(level, &mut random)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::{AdminEnvelope, AdminRequest, AdminResult, AdminScope, CommandReport};
    use crate::world::level_blocks::stored_block_state;
    use crate::world::WorldEvent;
    use bbb_protocol::block_state_types::vanilla_block_state_identity;
    use bbb_protocol::packets::Vec3d;
    use bbb_server_types::save::Java26BuiltinDimension;
    use std::sync::Arc;

    use crate::world::{ServerWorld, ServerWorldConfig};

    fn startup() -> Arc<bbb_server_datapack::startup::DatapackStartup> {
        Arc::new(
            bbb_server_datapack::startup::DatapackStartup::load(
                &bbb_server_datapack::startup::DatapackStartupOptions {
                    datapacks_dir: None,
                    saved_data_packs: None,
                    safe_mode: false,
                    limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(
                    ),
                },
            )
            .expect("the embedded vanilla snapshot loads"),
        )
    }

    fn world() -> ServerWorld {
        ServerWorld::new(ServerWorldConfig {
            datapacks: Some(startup()),
            ..ServerWorldConfig::default()
        })
        .unwrap()
    }

    fn load_place_neighborhood(world: &mut ServerWorld, center_chunk_x: i32) {
        for chunk_x in (center_chunk_x - 1)..=(center_chunk_x + 1) {
            for chunk_z in -1..=1 {
                let _ = world.block_state(
                    Java26BuiltinDimension::Overworld,
                    BlockPos {
                        x: chunk_x * 16,
                        y: 80,
                        z: chunk_z * 16,
                    },
                );
            }
        }
    }

    #[test]
    fn place_commands_publish_blocks_loot_and_saved_entities() {
        let mut world = world();
        load_place_neighborhood(&mut world, 0);
        let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
        let template = world
            .cold
            .structure_templates
            .get(
                "minecraft:igloo/bottom",
                Some(&world.datapacks().unwrap().structures),
            )
            .unwrap();
        let chest = template.materialized().palettes[0]
            .blocks
            .iter()
            .find(|block| {
                vanilla_block_state_identity(block.state)
                    .is_some_and(|state| state.name == "minecraft:chest")
            })
            .unwrap();
        let chest_pos = BlockPos {
            x: chest.position.x,
            y: 80 + chest.position.y,
            z: chest.position.z,
        };
        let completions = crate::commands::completion_suggestions_in_world(
            "place template minecraft:igloo/",
            &crate::commands::CommandSource::server(
                "test",
                crate::commands::CommandPermission::Owner,
            ),
            &world,
            &ServerWorld::console_origin(),
        );
        assert!(completions
            .suggestions
            .iter()
            .any(|suggestion| suggestion.text == "minecraft:igloo/bottom"));
        let before = world.level_by_id(&dimension).entities.len();
        let mut events = Vec::new();
        world.apply_admin(
            admin_envelope("place-template"),
            AdminRequest::ExecuteCommand {
                command: "place template minecraft:igloo/bottom 0 80 0 none none 1 42 strict"
                    .into(),
            },
            &mut events,
        );
        let (report, _) = command_report(&events);
        assert_eq!(report.result, Some(1), "{:?}", report.messages);
        let level = world.level_by_id(&dimension);
        assert_eq!(stored_block_state(&level.chunks, chest_pos), chest.state);
        let chest = level
            .chunks
            .block_entities()
            .get(chest_pos)
            .unwrap()
            .command_data()
            .expect("fixture chest data");
        let bbb_protocol::NbtSummaryValue::Compound(entries) = chest else {
            panic!("chest compound");
        };
        assert!(entries.iter().any(|entry| entry.name == "Items"));
        assert_eq!(
            level.entities.len() - before,
            template.materialized().entities.len()
        );
        let snapshot = world.entity_nbt_read_snapshot();
        let saved = crate::entity::save::write_entity_chunk(&level.entities, (0, 0)).unwrap();
        assert_eq!(
            saved.entities().len(),
            template.materialized().entities.len()
        );
        let mut restored =
            crate::entity::EntityStore::new(0, crate::entity::EntityIdCounter::new());
        let ids = crate::entity::save::read_entity_chunk_with_context(
            &mut restored,
            &saved,
            snapshot.context(),
        )
        .unwrap();
        assert_eq!(ids.len(), template.materialized().entities.len());

        for x in 0..=4 {
            for z in -2..=2 {
                let _ = world.block_state(
                    Java26BuiltinDimension::Overworld,
                    BlockPos {
                        x: x * 16,
                        y: 80,
                        z: z * 16,
                    },
                );
            }
        }
        events.clear();
        world.apply_admin(
            admin_envelope("place-structure"),
            AdminRequest::ExecuteCommand {
                command: "place structure minecraft:desert_pyramid 32 80 0".into(),
            },
            &mut events,
        );
        let (report, _) = command_report(&events);
        assert_eq!(report.result, Some(1), "{:?}", report.messages);
        let chests = (0..=4)
            .flat_map(|x| (-2..=2).map(move |z| (x, z)))
            .flat_map(|chunk| {
                world
                    .level_by_id(&dimension)
                    .chunks
                    .block_entities()
                    .chunk(chunk)
            })
            .filter(|entity| {
                let bbb_protocol::NbtSummaryValue::Compound(entries) = entity
                    .command_data_with_resolver(
                        crate::world::registry_runtime::item_stack_resolver_for_test(),
                    )
                    .expect("fixture block entity data")
                else {
                    return false;
                };
                entries.iter().any(|entry| {
                    entry.name == "LootTable"
                        && entry.value
                            == bbb_protocol::NbtSummaryValue::String(
                                "minecraft:chests/desert_pyramid".into(),
                            )
                })
            })
            .count();
        assert_eq!(chests, 4);

        let base = world
            .cold
            .structure_templates
            .get(
                "minecraft:pillager_outpost/base_plate",
                Some(&world.datapacks().unwrap().structures),
            )
            .unwrap();
        let target = &base.materialized().palettes[0].jigsaws[0].name;
        events.clear();
        world.apply_admin(
            admin_envelope("place-jigsaw"),
            AdminRequest::ExecuteCommand {
                command: format!(
                    "place jigsaw minecraft:pillager_outpost/base_plates {target} 1 48 80 0"
                ),
            },
            &mut events,
        );
        let (report, _) = command_report(&events);
        assert_eq!(report.result, Some(1), "{:?}", report.messages);
        assert!((32..=64).any(|x| (-16..=16).any(|z| (72..=88).any(|y| {
            vanilla_block_state_identity(stored_block_state(
                &world.level_by_id(&dimension).chunks,
                BlockPos { x, y, z },
            ))
            .is_some_and(|state| {
                state.name == "minecraft:cobblestone" || state.name == "minecraft:mossy_cobblestone"
            })
        }))));

        world.with_live_placement(dimension, |level, _| {
            bbb_server_worldgen::noise::mineshaft::MineshaftPlacementLevel::add_mineshaft_chest_minecart(level, [0.5, 80.0, 0.5], 42);
            true
        });
        let snapshot = world.entity_nbt_read_snapshot();
        let saved = crate::entity::save::write_entity_chunk(
            &world.level_by_id(&dimension).entities,
            (0, 0),
        )
        .unwrap();
        let mut restored =
            crate::entity::EntityStore::new(0, crate::entity::EntityIdCounter::new());
        crate::entity::save::read_entity_chunk_with_context(
            &mut restored,
            &saved,
            snapshot.context(),
        )
        .unwrap();
        let pending = restored
            .ids()
            .into_iter()
            .filter_map(|id| {
                crate::entity::minecart_container::minecart_container(restored.get(id)?.data())
            })
            .find(|container| container.loot_table_id().is_some())
            .unwrap();
        assert_eq!(
            pending.loot_table_id().as_deref(),
            Some("minecraft:chests/abandoned_mineshaft")
        );
        assert_eq!(pending.loot_table_seed(), 42);

        let template = bbb_server_datapack::structure_template::StructureTemplate::from_materialized(
            bbb_server_datapack::structure_template::MaterializedStructureTemplate {
                size: [1, 1, 1],
                palettes: vec![bbb_server_datapack::structure_template::StructureTemplatePalette {
                    blocks: vec![bbb_server_datapack::structure_template::StructureTemplateBlock {
                        position: BlockPos { x: 0, y: 0, z: 0 },
                        state: bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
                            "minecraft:stone", &Default::default(),
                        ).unwrap(),
                        nbt: None,
                    }],
                    jigsaws: Vec::new(),
                }],
                entities: vec![bbb_server_datapack::structure_template::StructureTemplateEntity {
                    position_bits: [0.5_f64.to_bits(), 1.0_f64.to_bits(), 0.5_f64.to_bits()],
                    block_position: BlockPos { x: 0, y: 1, z: 0 },
                    nbt: bbb_protocol::nbt_summary::parse_snbt_compound(
                        "{id:'minecraft:cow',NoAI:1b,NoGravity:1b,Team:'template_team',Tags:['template_team_probe']}",
                    ).unwrap(),
                }],
            },
        ).unwrap();
        assert!(world
            .cold
            .structure_templates
            .put("test:team_probe", template));
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 81, "template_watcher", 8001);
        events.clear();
        for command in [
            "team add template_team",
            "place template test:team_probe 8 100 8",
        ] {
            let sequence = world.last_command_sequence + 1;
            let tick = world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
                sequence,
                crate::mailbox::WorldProducerClass::Admin,
                crate::world::WorldCommand::Admin {
                    envelope: AdminEnvelope {
                        operation_id: sequence,
                        request_id: format!("template-team-{sequence}"),
                        ..admin_envelope("template-team")
                    },
                    request: AdminRequest::ExecuteCommand {
                        command: command.into(),
                    },
                },
            )]);
            assert_eq!(command_report(&tick.events).0.result, Some(1));
            events.extend(tick.events);
        }
        let cow = world
            .level_by_id(&dimension)
            .entities
            .ids()
            .into_iter()
            .find(|id| {
                let body = world.level_by_id(&dimension).entities.get(*id).unwrap();
                body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
                    && body
                        .entity_tags()
                        .iter()
                        .any(|tag| tag == "template_team_probe")
            })
            .unwrap();
        let uuid = world
            .level_by_id(&dimension)
            .entities
            .get(cow)
            .unwrap()
            .uuid();
        let bbb_protocol::NbtSummaryValue::Compound(record) = world
            .level_by_id(&dimension)
            .entities
            .command_data(cow)
            .unwrap()
        else {
            panic!("template cow record");
        };
        assert!(record.iter().any(|field| field.name == "UUID"
            && field.value == crate::entity::save::uuid_int_array(uuid)));
        assert!(!record.iter().any(|field| field.name == "Team"));
        for flag in ["NoAI", "NoGravity"] {
            assert!(record
                .iter()
                .any(|field| field.name == flag
                    && field.value == bbb_protocol::NbtSummaryValue::Byte(1)));
        }
        assert!(record.iter().any(|field| field.name == "Tags"
            && matches!(&field.value, bbb_protocol::NbtSummaryValue::List(tags)
                if tags.contains(&bbb_protocol::NbtSummaryValue::String("template_team_probe".into())))));
        for command in [
            "execute if entity @e[type=minecraft:cow,tag=template_team_probe,team=template_team]",
            "execute unless entity @e[type=minecraft:cow,tag=template_team_probe,team=!template_team]",
        ] {
            let sequence = world.last_command_sequence + 1;
            let tick = world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
                sequence,
                crate::mailbox::WorldProducerClass::Admin,
                crate::world::WorldCommand::Admin {
                    envelope: AdminEnvelope {
                        operation_id: sequence,
                        request_id: format!("template-team-{sequence}"),
                        ..admin_envelope("template-team")
                    },
                    request: AdminRequest::ExecuteCommand { command: command.into() },
                },
            )]);
            assert_eq!(command_report(&tick.events).0.result, Some(1));
            events.extend(tick.events);
        }
        let member = uuid.to_string();
        assert!(events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: crate::connection::ConnectionId(81),
                packet: crate::connection::ClientboundPlayPacket::SetPlayerTeam(team) }
                if team.name == "template_team" && team.players.iter().any(|player| player == &member)
        )));
        assert!(world
            .scoreboard
            .pack()
            .teams
            .iter()
            .any(|team| team.name == "template_team"
                && team.players.iter().any(|player| player == &member)));
    }

    #[test]
    fn place_end_island_writes_end_stone_on_a_live_level() {
        let mut world = world();
        let origin = BlockPos { x: 0, y: 80, z: 0 };
        let _ = world.block_state(Java26BuiltinDimension::Overworld, origin);
        assert!(
            world.place_live_configured_feature(
                Java26BuiltinDimension::Overworld.into(),
                "minecraft:end_island",
                origin,
            ),
            "end_island places"
        );
        let state = world
            .block_state(Java26BuiltinDimension::Overworld, origin)
            .expect("the origin is in the world");
        assert_eq!(
            vanilla_block_state_identity(state)
                .expect("end stone is a catalog state")
                .name,
            "minecraft:end_stone"
        );
    }

    #[test]
    fn retained_feature_that_returns_false_fails() {
        let mut world = world();
        let origin = BlockPos { x: 0, y: 80, z: 0 };
        let _ = world.block_state(Java26BuiltinDimension::Overworld, origin);
        assert!(
            !world.place_live_configured_feature(
                Java26BuiltinDimension::Overworld.into(),
                "minecraft:blue_ice",
                origin,
            ),
            "blue ice refuses an origin above sea level"
        );
    }

    #[test]
    fn place_unknown_feature_fails() {
        let mut world = world();
        let origin = BlockPos { x: 0, y: 80, z: 0 };
        assert!(
            !world.place_live_configured_feature(
                Java26BuiltinDimension::Overworld.into(),
                "minecraft:not_a_configured_feature",
                origin,
            ),
            "an unknown retained element fails"
        );
    }

    #[test]
    fn command_minecart_commits_the_settled_place_result() {
        let mut world = world();
        load_place_neighborhood(&mut world, 0);
        load_place_neighborhood(&mut world, 4);
        let failure = world.spawn_command_minecart_for_test(
            Java26BuiltinDimension::Overworld,
            Vec3d {
                x: 0.5,
                y: 80.0,
                z: 0.5,
            },
        );
        let success = world.spawn_command_minecart_for_test(
            Java26BuiltinDimension::Overworld,
            Vec3d {
                x: 0.5,
                y: 80.0,
                z: 0.5,
            },
        );
        world.edit_command_minecart_for_test(
            Java26BuiltinDimension::Overworld,
            failure,
            "place feature minecraft:blue_ice 64 80 0",
            true,
        );
        world.edit_command_minecart_for_test(
            Java26BuiltinDimension::Overworld,
            success,
            "place feature minecraft:end_island 0 80 0",
            true,
        );
        world.queue_command_minecart_execution_for_test(Java26BuiltinDimension::Overworld, failure);
        world.queue_command_minecart_execution_for_test(Java26BuiltinDimension::Overworld, success);
        world.run_pending_command_minecart_executions_for_test(
            Java26BuiltinDimension::Overworld,
            &mut Vec::new(),
        );

        let data = |id| {
            let entity = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .expect("command minecart remains live");
            let crate::entity::EntityData::CommandBlockMinecart(data) = entity.data() else {
                panic!("command minecart data")
            };
            data
        };
        assert_eq!(data(failure).success_count(), 0);
        assert!(data(failure)
            .last_output()
            .is_some_and(|output| output.text().contains("commands.place.feature.failed")));
        assert_eq!(data(success).success_count(), 1);
        assert!(data(success)
            .last_output()
            .is_some_and(|output| output.text().contains("commands.place.feature.success")));
    }

    fn admin_envelope(request_id: &str) -> AdminEnvelope {
        AdminEnvelope {
            operation_id: 1,
            request_id: request_id.to_string(),
            principal: "agent".to_string(),
            scope: AdminScope::Command,
            deadline_tick: u64::MAX,
            now: 1_755_000_000_000,
        }
    }

    fn command_report(events: &[WorldEvent]) -> (&CommandReport, bool) {
        events
            .iter()
            .find_map(|event| {
                let WorldEvent::Admin(outcome) = event else {
                    return None;
                };
                let Ok(AdminResult::Command(report)) = &outcome.result else {
                    return None;
                };
                Some((report, outcome.replayed))
            })
            .expect("command.execute produces one command report")
    }

    #[test]
    fn admin_command_and_replay_report_the_settled_place_result() {
        for (feature, chunk_x, expected) in [
            ("minecraft:end_island", 0, Some(1)),
            ("minecraft:blue_ice", 4, None),
        ] {
            let mut world = world();
            load_place_neighborhood(&mut world, chunk_x);
            let envelope = admin_envelope(feature);
            let request = AdminRequest::ExecuteCommand {
                command: format!("place feature {feature} {} 80 0", chunk_x * 16),
            };
            let mut first = Vec::new();
            world.apply_admin(envelope.clone(), request.clone(), &mut first);
            let (report, replayed) = command_report(&first);
            assert!(!replayed);
            assert_eq!(report.result, expected);
            assert_eq!(report.messages.len(), 1);

            let mut replay = Vec::new();
            world.apply_admin(envelope, request, &mut replay);
            let (replay_report, replayed) = command_report(&replay);
            assert!(replayed);
            assert_eq!(replay_report, report);
        }
    }
}
