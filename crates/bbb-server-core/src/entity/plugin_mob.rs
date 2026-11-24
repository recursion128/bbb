//! Server-side plugin mob state, goal specification, and behavior hooks.

use std::sync::Arc;

use bbb_protocol::content_registry::{ModEntityTypeSpec, ModSynchedFieldSpec};
use bbb_protocol::entity_data_serializers::*;
use bbb_protocol::packets::entities::MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID;
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};
use crate::world::simulation_level::LevelCtx;

/// Built-in goal descriptors available to server plugin mobs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MobGoalSpec {
    Float,
    MeleeAttack {
        speed_modifier: f64,
        following_target_even_if_not_seen: bool,
    },
    WaterAvoidingRandomStroll {
        speed_modifier: f64,
    },
    LookAtPlayer {
        look_distance: f32,
    },
    RandomLookAround,
    HurtByTarget {
        alert_others: bool,
    },
    NearestAttackablePlayer {
        must_see: bool,
    },
}

/// Goal and target registration list for one custom mob type.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MobBehaviorSpec {
    pub goals: Vec<(i32, MobGoalSpec)>,
    pub targets: Vec<(i32, MobGoalSpec)>,
}

/// Per-instance runtime state stored on every plugin-defined entity inside
/// `GeneralLivingData`.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginMobState {
    pub type_id: i32,
    pub resource_id: &'static str,
    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
    pub fire_immune: bool,
    pub client_tracking_range: i32,
    pub update_interval: i32,
    pub track_deltas: bool,
    pub timers: [i32; 4],
    pub scalars: [f32; 4],
    pub flags: u64,
    pub synched_values: Vec<EntityDataValue>,
    pub default_synched_values: Vec<EntityDataValue>,
    pub custom_nbt: Vec<NbtSummaryEntry>,
}

impl PluginMobState {
    pub fn from_spec(spec: &ModEntityTypeSpec, behavior: &dyn PluginMobBehavior) -> Self {
        let default_synched_values = default_synched_values_for_spec(spec);
        let mut synched_values = behavior.initial_synched_values(spec);
        for default in &default_synched_values {
            if !synched_values
                .iter()
                .any(|existing| existing.data_id == default.data_id)
            {
                synched_values.push(default.clone());
            }
        }
        synched_values.sort_by_key(|row| row.data_id);
        Self {
            type_id: spec.id,
            resource_id: spec.resource_id_static(),
            width: spec.width,
            height: spec.height,
            eye_height: spec.eye_height,
            fire_immune: spec.fire_immune,
            client_tracking_range: spec.client_tracking_range,
            update_interval: spec.update_interval,
            track_deltas: true,
            timers: [0; 4],
            scalars: [0.0; 4],
            flags: 0,
            synched_values,
            default_synched_values,
            custom_nbt: Vec::new(),
        }
    }

    pub fn synched_value(&self, data_id: u8) -> Option<&EntityDataValue> {
        self.synched_values
            .iter()
            .find(|entry| entry.data_id == data_id)
    }

    pub fn set_synched_value(&mut self, value: EntityDataValue) {
        if let Some(existing) = self
            .synched_values
            .iter_mut()
            .find(|entry| entry.data_id == value.data_id)
        {
            *existing = value;
        } else {
            self.synched_values.push(value);
            self.synched_values.sort_by_key(|row| row.data_id);
        }
    }

    pub fn write_save_entry(&self) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: "bbb_plugin".into(),
            value: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "timers".into(),
                    value: NbtSummaryValue::IntArray(self.timers.to_vec()),
                },
                NbtSummaryEntry {
                    name: "scalars".into(),
                    value: NbtSummaryValue::IntArray(
                        self.scalars.iter().map(|v| v.to_bits() as i32).collect(),
                    ),
                },
                NbtSummaryEntry {
                    name: "flags".into(),
                    value: NbtSummaryValue::Long(self.flags as i64),
                },
                NbtSummaryEntry {
                    name: "custom".into(),
                    value: NbtSummaryValue::Compound(self.custom_nbt.clone()),
                },
            ]),
        }
    }

    pub fn read_save_entries(&mut self, entries: &[NbtSummaryEntry]) {
        let Some(NbtSummaryValue::Compound(plugin_entries)) = entries
            .iter()
            .find(|entry| entry.name == "bbb_plugin")
            .map(|entry| &entry.value)
        else {
            return;
        };
        for entry in plugin_entries {
            match entry.name.to_string().as_str() {
                "timers" => {
                    if let NbtSummaryValue::IntArray(vals) = &entry.value {
                        for (dst, &src) in self.timers.iter_mut().zip(vals.iter()) {
                            *dst = src;
                        }
                    }
                }
                "scalars" => {
                    if let NbtSummaryValue::IntArray(vals) = &entry.value {
                        for (dst, &src) in self.scalars.iter_mut().zip(vals.iter()) {
                            *dst = f32::from_bits(src as u32);
                        }
                    }
                }
                "flags" => {
                    if let NbtSummaryValue::Long(val) = entry.value {
                        self.flags = val as u64;
                    }
                }
                "custom" => {
                    if let NbtSummaryValue::Compound(custom) = &entry.value {
                        self.custom_nbt = custom.clone();
                    }
                }
                _ => {}
            }
        }
    }
}

/// Server-side behavior hooks invoked for each registered plugin entity.
pub trait PluginMobBehavior: Send + Sync + std::fmt::Debug {
    fn initial_synched_values(&self, _spec: &ModEntityTypeSpec) -> Vec<EntityDataValue> {
        Vec::new()
    }

    fn on_finalize_spawn(&self, _entity: &mut Entity, _store: &mut EntityStore) {}

    fn on_tick(
        &self,
        _entity: &mut Entity,
        _store: &mut EntityStore,
        _level: &mut LevelCtx<'_, '_>,
    ) {
    }

    fn on_hurt(
        &self,
        _entity: &mut Entity,
        _store: &mut EntityStore,
        _level: &mut LevelCtx<'_, '_>,
        _amount: f32,
    ) {
    }

    fn on_death(
        &self,
        _entity: &mut Entity,
        _store: &mut EntityStore,
        _level: &mut LevelCtx<'_, '_>,
    ) {
    }

    fn boss_bar_progress(&self, entity: &Entity) -> Option<f32> {
        entity
            .living()
            .map(|living| (living.health() / living.max_health()).clamp(0.0, 1.0))
    }
}

#[derive(Debug, Default)]
pub struct DefaultPluginMobBehavior;

impl PluginMobBehavior for DefaultPluginMobBehavior {}

pub(crate) fn default_synched_values_for_spec(spec: &ModEntityTypeSpec) -> Vec<EntityDataValue> {
    spec.synched_fields
        .iter()
        .filter_map(default_synched_value_for_field)
        .collect()
}

fn default_synched_value_for_field(field: &ModSynchedFieldSpec) -> Option<EntityDataValue> {
    let value = match field.serializer_id {
        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID => EntityDataValueKind::Byte(0),
        VANILLA_ENTITY_DATA_SERIALIZER_INT_ID => EntityDataValueKind::Int(0),
        VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID => EntityDataValueKind::Long(0),
        VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID => EntityDataValueKind::Float(0.0),
        VANILLA_ENTITY_DATA_SERIALIZER_STRING_ID => {
            EntityDataValueKind::String(String::new().into())
        }
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID => EntityDataValueKind::Boolean(false),
        VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID => EntityDataValueKind::Rotations {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_POS_ID => {
            EntityDataValueKind::BlockPos(BlockPos { x: 0, y: 0, z: 0 })
        }
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID => {
            EntityDataValueKind::OptionalBlockPos(None)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID => EntityDataValueKind::Direction(0),
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID => {
            EntityDataValueKind::OptionalLivingEntityReference(None)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID => {
            EntityDataValueKind::OptionalUnsignedInt(None)
        }
        VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID => EntityDataValueKind::Pose(0),
        VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID => EntityDataValueKind::Vector3f {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID => EntityDataValueKind::Quaternionf {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
        MOD_CUSTOM_ENTITY_DATA_SERIALIZER_ID => EntityDataValueKind::ModCustom(Vec::new()),
        _ => return None,
    };
    Some(EntityDataValue {
        data_id: field.data_id,
        serializer_id: field.serializer_id,
        value,
    })
}

impl Entity {
    pub fn plugin_mob(&self) -> Option<&PluginMobState> {
        match &self.data {
            EntityData::GeneralLiving(data) => data.plugin_mob(),
            _ => None,
        }
    }

    pub fn plugin_mob_mut(&mut self) -> Option<&mut PluginMobState> {
        match &mut self.data {
            EntityData::GeneralLiving(data) => data.plugin_mob_mut(),
            _ => None,
        }
    }

    /// Updates one modded entity synched data entry and marks its slot dirty
    /// for tracker broadcast.
    pub fn set_plugin_synched_value(&mut self, value: EntityDataValue) {
        let data_id = value.data_id;
        if let Some(plugin) = self.plugin_mob_mut() {
            plugin.set_synched_value(value);
            self.mark_synched_dirty(data_id);
        }
    }
}

pub(crate) fn shared_default_behavior() -> Arc<dyn PluginMobBehavior> {
    Arc::new(DefaultPluginMobBehavior)
}

#[cfg(test)]
mod tests {
    use super::*;

    use bbb_facts::boss_bar_styles::{
        VANILLA_BOSS_BAR_COLOR_RED_ORDINAL, VANILLA_BOSS_BAR_OVERLAY_NOTCHED_10_ORDINAL,
    };
    use bbb_facts::entity_types::VanillaEntityMobCategory;
    use bbb_protocol::attribute_types::{
        VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID,
        VANILLA_ATTRIBUTE_MAX_HEALTH_ID, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
    };
    use bbb_protocol::content_registry::{ModBossBarSpec, ModEntityRegistration};
    use bbb_protocol::game_types::GameType;
    use bbb_protocol::packets::{
        decode_play_clientbound, decode_play_clientbound_with_content_registry,
        encode_play_clientbound_boss_event, encode_play_clientbound_system_chat,
        BossEventOperation, PlayClientbound,
    };
    use bbb_server_types::save::Java26BuiltinDimension;

    use crate::connection::{ClientboundPlayPacket, ConnectionId};
    use crate::entity::save::{read_entity_chunk, write_entity_chunk};
    use crate::entity::store::EntityIdCounter;
    use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
    use crate::plugin::{MobBehaviorSpec, MobGoalSpec, ServerPluginRegistryBuilder};
    use crate::tests::{chat_command, join_loaded, operator_config, settle_joined_bodies};
    use crate::world::{ServerWorld, ServerWorldConfig, WorldEvent};

    #[derive(Debug)]
    struct TestBossBehavior;

    impl PluginMobBehavior for TestBossBehavior {
        fn on_tick(
            &self,
            entity: &mut Entity,
            _store: &mut EntityStore,
            _level: &mut LevelCtx<'_, '_>,
        ) {
            if let Some(plugin) = entity.plugin_mob_mut() {
                plugin.timers[0] += 1;
                plugin.scalars[0] = 2.5;
                plugin.flags |= 1;
                plugin.custom_nbt = vec![NbtSummaryEntry {
                    name: "phase".into(),
                    value: NbtSummaryValue::Int(2),
                }];
            }
            entity.set_plugin_synched_value(EntityDataValue {
                data_id: 16,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(1),
            });
        }
    }

    #[test]
    fn plugin_mob_summon_ai_boss_bar_synched_data_and_nbt_round_trip() {
        let mut builder = ServerPluginRegistryBuilder::new();
        builder.register_mob(
            ModEntityRegistration {
                resource_id: "bbb_demo:inferno_warden".to_string(),
                width: 1.2,
                height: 3.2,
                eye_height: 2.8,
                client_tracking_range: 10,
                update_interval: 2,
                category: VanillaEntityMobCategory::Monster,
                is_living: true,
                is_mob: true,
                fire_immune: true,
                can_summon: true,
                serializable: true,
                allowed_in_peaceful: false,
                default_attributes: vec![
                    (VANILLA_ATTRIBUTE_MAX_HEALTH_ID, 250.0),
                    (VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, 14.0),
                    (VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, 0.30),
                    (VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID, 40.0),
                ],
                synched_fields: vec![ModSynchedFieldSpec {
                    data_id: 16,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                }],
                boss_bar: Some(ModBossBarSpec {
                    color_id: VANILLA_BOSS_BAR_COLOR_RED_ORDINAL,
                    overlay_id: VANILLA_BOSS_BAR_OVERLAY_NOTCHED_10_ORDINAL,
                    darken_screen: true,
                    play_boss_music: true,
                    create_world_fog: false,
                }),
                ..Default::default()
            },
            MobBehaviorSpec {
                goals: vec![(
                    2,
                    MobGoalSpec::MeleeAttack {
                        speed_modifier: 1.0,
                        following_target_even_if_not_seen: true,
                    },
                )],
                targets: vec![(2, MobGoalSpec::NearestAttackablePlayer { must_see: false })],
            },
            Arc::new(TestBossBehavior),
        );
        let plugins = Arc::new(builder.freeze().expect("plugin registry freezes"));
        let expected_type_id = plugins
            .content_registry()
            .entity_by_resource_id("bbb_demo:inferno_warden")
            .expect("registered boss")
            .id;
        assert_eq!(expected_type_id, 157);

        let config = ServerWorldConfig {
            game_type: GameType::Survival,
            plugins: Arc::clone(&plugins),
            ..operator_config(&[("alice", 1)])
        };
        let mut world = ServerWorld::new(config).expect("fixture world");
        world.set_test_respawn_radius(0);
        let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        settle_joined_bodies(&mut world);

        let summon_tick = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "summon bbb_demo:inferno_warden ~ ~ ~3",
        );
        let mut packets = Vec::new();
        for event in summon_tick.events {
            if let WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet,
            } = event
            {
                packets.push(packet);
            }
        }
        for _ in 0..3 {
            let tick = world.tick(Vec::new());
            for event in tick.events {
                if let WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet,
                } = event
                {
                    packets.push(packet);
                }
            }
        }

        let uuid = packets
            .iter()
            .find_map(|packet| match packet {
                ClientboundPlayPacket::AddEntity(add) if add.entity_type_id == expected_type_id => {
                    Some(add.uuid)
                }
                _ => None,
            })
            .expect("summoned boss spawn");
        let message = packets
            .iter()
            .find_map(|packet| match packet {
                ClientboundPlayPacket::SystemChat(chat)
                    if chat.content.text().contains("commands.summon.success") =>
                {
                    Some(encode_play_clientbound_system_chat(chat).unwrap())
                }
                _ => None,
            })
            .expect("summon feedback");
        let boss_bar = packets
            .iter()
            .find_map(|packet| match packet {
                ClientboundPlayPacket::BossEvent(event)
                    if matches!(event.operation, BossEventOperation::Add { .. }) =>
                {
                    Some(encode_play_clientbound_boss_event(event).unwrap())
                }
                _ => None,
            })
            .expect("summoned boss bar");
        for (id, payload) in [message, boss_bar] {
            assert!(decode_play_clientbound(id, &payload).is_err());
            let component = match decode_play_clientbound_with_content_registry(
                id,
                &payload,
                plugins.content_registry(),
            )
            .unwrap()
            {
                PlayClientbound::SystemChat(chat) => chat.content,
                PlayClientbound::BossEvent(bbb_protocol::packets::BossEvent {
                    operation: BossEventOperation::Add { name, .. },
                    ..
                }) => name,
                _ => unreachable!(),
            };
            let hover = component
                .styled()
                .iter()
                .find_map(|run| run.style.hover_event.as_ref());
            let Some(bbb_protocol::ComponentHoverEvent::ShowEntity {
                id,
                uuid: target,
                name_component: Some(name),
                ..
            }) = hover
            else {
                panic!("expected summoned boss in entity tooltip")
            };
            assert_eq!(id, "bbb_demo:inferno_warden");
            assert_eq!(bbb_protocol::component_hover_uuid(target), Some(uuid));
            assert_eq!(
                bbb_protocol::nbt_summary::value_field(name, "translate"),
                Some(&NbtSummaryValue::string("entity.bbb_demo.inferno_warden"))
            );
        }
        assert!(
            packets.iter().any(|packet| matches!(
                packet,
                ClientboundPlayPacket::SetEntityData(update)
                    if update.values.iter().any(|entry| {
                        entry.data_id == 16 && matches!(entry.value, EntityDataValueKind::Byte(1))
                    })
            )),
            "expected SetEntityData(data_id=16, Byte(1)) after plugin on_tick"
        );

        let chunk = write_entity_chunk(
            world.level_entities(Java26BuiltinDimension::Overworld),
            (0, 0),
        )
        .expect("write modded boss chunk");
        let mut restored_store = EntityStore::new(99, EntityIdCounter::new());
        restored_store.set_plugins(Arc::clone(&plugins));
        let restored_ids = read_entity_chunk(&mut restored_store, &chunk).expect("restore chunk");
        assert_eq!(restored_ids.len(), 1);
        let restored = restored_store.get(restored_ids[0]).expect("restored boss");
        let state = restored.plugin_mob().expect("plugin mob state");
        assert!(state.timers[0] >= 1);
        assert_eq!(state.scalars[0], 2.5);
        assert_eq!(state.flags, 1);
        assert!(state.custom_nbt.iter().any(|entry| {
            entry.name == "phase" && matches!(entry.value, NbtSummaryValue::Int(2))
        }));
        let teleport = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("tp {uuid} ~ ~ ~3"),
        );
        assert!(crate::tests::system_chats(&teleport)
            .iter()
            .any(|(connection, line)| *connection == ConnectionId(1)
                && line.starts_with("commands.teleport.success.location.single")));
    }

    #[test]
    fn plugin_mob_supports_nbt_summon_spawner_natural_spawn_loot_xp_and_peaceful_despawn() {
        let mut builder = ServerPluginRegistryBuilder::new();
        builder.register_mob(
            ModEntityRegistration {
                resource_id: "bbb_demo:inferno_warden".to_string(),
                width: 1.2,
                height: 3.2,
                eye_height: 2.8,
                client_tracking_range: 10,
                update_interval: 2,
                category: VanillaEntityMobCategory::Monster,
                is_living: true,
                is_mob: true,
                fire_immune: true,
                can_summon: true,
                serializable: true,
                allowed_in_peaceful: false,
                default_loot_table: Some("minecraft:entities/zombie".to_string()),
                xp_reward: 42,
                ambient_sound: Some("minecraft:entity.warden.ambient".to_string()),
                hurt_sound: Some("minecraft:entity.warden.hurt".to_string()),
                death_sound: Some("minecraft:entity.warden.death".to_string()),
                default_attributes: vec![(VANILLA_ATTRIBUTE_MAX_HEALTH_ID, 250.0)],
                ..Default::default()
            },
            MobBehaviorSpec::default(),
            Arc::new(TestBossBehavior),
        );
        let plugins = Arc::new(builder.freeze().expect("plugin registry freezes"));
        let content_registry = plugins.content_registry();
        let type_id = content_registry
            .entity_by_resource_id("bbb_demo:inferno_warden")
            .expect("registered boss")
            .id;

        // SpawnerSpawnData & prepare_spawn_request resolve custom mod entities (R2-53 / R13-12).
        let spawn_data = crate::block_entity::base_spawner::SpawnerSpawnData::new(
            vec![NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("bbb_demo:inferno_warden"),
            }],
            None,
            None,
        );
        assert_eq!(
            spawn_data.entity_type_id_with_registry(content_registry),
            Some(type_id)
        );
        let request = crate::entity::SpawnRequest {
            entity_type_id: type_id,
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            spawn_data,
            spawner_position: bbb_protocol::packets::BlockPos { x: 0, y: 64, z: 0 },
            spawn_range: 4,
            max_nearby_entities: 6,
        };
        assert!(
            crate::entity::minecart_spawner::spawner_runtime::prepare_spawn_request_with_registry(
                &request,
                content_registry,
            )
            .is_ok()
        );

        let config = ServerWorldConfig {
            game_type: GameType::Survival,
            plugins: Arc::clone(&plugins),
            ..operator_config(&[("alice", 1)])
        };
        let mut world = ServerWorld::new(config).expect("fixture world");
        world.set_test_respawn_radius(0);
        let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        settle_joined_bodies(&mut world);

        // Summon with explicit NBT (R13-12).
        let summon = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "summon bbb_demo:inferno_warden ~ ~ ~3 {}",
        );
        assert!(
            crate::tests::system_chats(&summon)
                .iter()
                .any(|(c, line)| *c == ConnectionId(1)
                    && line.starts_with("commands.summon.success")),
            "expected /summon with NBT to succeed for custom mod entity"
        );

        // Tick across tick_count % 5 == 0 boundary without update_control_flags refusing (R16-07).
        for _ in 0..8 {
            world.tick(Vec::new());
        }
        let store = world.level_entities(Java26BuiltinDimension::Overworld);
        let entity = store
            .bodies()
            .find(|body| body.entity_type_id() == type_id)
            .expect("boss survives across 8 ticks");
        assert!(
            entity.plugin_mob().expect("plugin mob").timers[0] >= 6,
            "expected plugin mob to keep ticking past tick 5"
        );
        // Natural spawn existing facts must recognize custom mod entity category (R16-07).
        let existing = crate::entity::natural_spawn::natural_spawn_existing_facts(store, entity)
            .expect("custom mod entity must not abort natural spawn scan");
        assert_eq!(existing.category, VanillaEntityMobCategory::Monster);
        // Default loot table and XP reward from ModEntityTypeSpec (R2-53 / R10-11).
        assert_eq!(
            crate::entity::mob_death_loot::entity_loot_table_with_registry(
                entity,
                content_registry,
            ),
            Some("minecraft:entities/zombie")
        );
        assert_eq!(
            crate::entity::experience::experience_reward_with_registry(
                entity,
                &crate::random::GameRandom::with_seed(1),
                content_registry,
            ),
            42
        );
        assert!(
            crate::entity::experience::should_drop_experience_with_registry(
                entity,
                content_registry,
            )
        );
        // Peaceful difficulty despawns non-peaceful mod mobs (R2-53).
        let _ = chat_command(&sender, &receiver, &mut world, 1, "difficulty peaceful");
        world.tick(Vec::new());
        assert!(
            world
                .level_entities(Java26BuiltinDimension::Overworld)
                .bodies()
                .all(|body| body.entity_type_id() != type_id),
            "non-peaceful mod mob should despawn on peaceful difficulty"
        );
    }
}
