use std::cmp::Ordering;

use anyhow::{bail, Context, Result};
use bbb_client_world::{ChunkPos, WorldStore};
use bbb_protocol::{
    canonical_snbt_string, decode_nbt_summary,
    packets::{self, RegistryData, UpdateTags},
    registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES,
};
use sha2::{Digest, Sha256};

use crate::types::{
    ConfigurationCatalogReport, ConfigurationRegistryEntryReport, ConfigurationRegistryReport,
    ConfigurationRegistryTagsReport, ConfigurationTagReport, DimensionLevelReport,
    DimensionTransitionLegReport, DimensionTransitionsReport,
};
use bbb_client_net::{ConnectionState, DecodedClientboundPacket, PlayPacketBundleAssembler};

use super::ProbeContext;

const TRANSITION_TARGETS: [(&str, &str); 3] = [
    (
        "minecraft:the_nether",
        "execute in minecraft:the_nether run tp @s 0 80 0",
    ),
    (
        "minecraft:the_end",
        "execute in minecraft:the_end run tp @s 0 80 0",
    ),
    (
        "minecraft:overworld",
        "execute in minecraft:overworld run tp @s 0 80 0",
    ),
];

#[derive(Default)]
pub(super) struct ConfigurationCapture {
    selected_known_packs: Option<usize>,
    packet_order: Vec<String>,
    registries: Vec<RegistryData>,
    tags: Vec<UpdateTags>,
}

impl ConfigurationCapture {
    pub(super) fn select_known_packs(&mut self, selected: usize) -> Result<()> {
        if self.selected_known_packs.replace(selected).is_some() {
            bail!("dimension probe received duplicate SelectKnownPacks packets");
        }
        self.packet_order.push("select_known_packs".to_string());
        Ok(())
    }

    pub(super) fn registry_data(&mut self, data: &RegistryData) {
        self.packet_order
            .push(format!("registry_data:{}", data.registry));
        self.registries.push(data.clone());
    }

    pub(super) fn update_tags(&mut self, tags: &UpdateTags) {
        self.packet_order.push("update_tags".to_string());
        self.tags.push(tags.clone());
    }

    pub(super) fn finish(&mut self, world: &WorldStore) -> Result<ConfigurationCatalogReport> {
        self.packet_order.push("finish".to_string());
        self.build(|registry, holder_id| {
            world
                .registry_resource_id_by_holder_id(registry, holder_id)
                .map(str::to_string)
        })
    }

    fn build(
        &self,
        mut resolve_holder: impl FnMut(&str, i32) -> Option<String>,
    ) -> Result<ConfigurationCatalogReport> {
        let selected_known_packs = self
            .selected_known_packs
            .context("dimension probe did not receive SelectKnownPacks")?;
        if selected_known_packs != 0 {
            bail!("dimension probe requires zero selected known packs, got {selected_known_packs}");
        }

        let expected = VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES
            .iter()
            .map(|entry| entry.resource_id)
            .collect::<Vec<_>>();
        let actual = self
            .registries
            .iter()
            .map(|registry| registry.registry.as_str())
            .collect::<Vec<_>>();
        if actual != expected {
            bail!("dimension probe registry order mismatch: expected {expected:?}, got {actual:?}");
        }
        let expected_packet_order = std::iter::once("select_known_packs".to_string())
            .chain(
                expected
                    .iter()
                    .map(|registry| format!("registry_data:{registry}")),
            )
            .chain(["update_tags".to_string(), "finish".to_string()])
            .collect::<Vec<_>>();
        if self.packet_order != expected_packet_order {
            bail!(
                "dimension probe configuration packet order mismatch: {:?}",
                self.packet_order
            );
        }

        let mut registries = Vec::with_capacity(self.registries.len());
        for registry in &self.registries {
            let mut previous = None;
            let mut entries = Vec::with_capacity(registry.entries.len());
            for entry in &registry.entries {
                if previous.is_some_and(|previous| {
                    resource_location_order(previous, &entry.id) != Ordering::Less
                }) {
                    bail!(
                        "dimension probe registry {} entries are not strictly path/namespace ordered at {}",
                        registry.registry,
                        entry.id
                    );
                }
                previous = Some(entry.id.as_str());
                let raw = entry.raw_data.as_deref().with_context(|| {
                    format!(
                        "dimension probe registry {} entry {} omitted NBT",
                        registry.registry, entry.id
                    )
                })?;
                let nbt = decode_nbt_summary(raw)
                    .with_context(|| {
                        format!(
                            "dimension probe registry {} entry {} has invalid NBT",
                            registry.registry, entry.id
                        )
                    })?
                    .with_context(|| {
                        format!(
                            "dimension probe registry {} entry {} has an NBT end root",
                            registry.registry, entry.id
                        )
                    })?;
                entries.push(ConfigurationRegistryEntryReport {
                    resource_id: entry.id.clone(),
                    canonical_nbt_sha256: canonical_nbt_sha256(&nbt),
                });
            }
            registries.push(ConfigurationRegistryReport {
                registry: registry.registry.clone(),
                entries,
            });
        }

        if self.tags.len() != 1 {
            bail!(
                "dimension probe expected one UpdateTags packet, got {}",
                self.tags.len()
            );
        }
        let tags = self.tags[0]
            .registries
            .iter()
            .map(|registry| {
                let tags = registry
                    .tags
                    .iter()
                    .map(|tag| {
                        let entries = tag
                            .entries
                            .iter()
                            .map(|holder_id| {
                                resolve_holder(&registry.registry, *holder_id).with_context(|| {
                                    format!(
                                        "dimension probe could not resolve holder {holder_id} in {} tag {}",
                                        registry.registry, tag.tag
                                    )
                                })
                            })
                            .collect::<Result<Vec<_>>>()?;
                        Ok(ConfigurationTagReport {
                            tag: tag.tag.clone(),
                            entries,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(ConfigurationRegistryTagsReport {
                    registry: registry.registry.clone(),
                    tags,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        // `serializeTagsToNetwork` collects into a `HashMap` and each registry's
        // payload is a second one, so the
        // wire order is a hash order rather than a contract; only the membership
        // is one. Publish it in a canonical order so two servers can be compared.
        let mut tags = tags;
        for registry in &mut tags {
            registry
                .tags
                .sort_by(|left, right| left.tag.cmp(&right.tag));
        }
        tags.sort_by(|left, right| left.registry.cmp(&right.registry));

        Ok(ConfigurationCatalogReport {
            selected_known_packs,
            packet_order: self.packet_order.clone(),
            registries,
            tags,
        })
    }
}

fn resource_location_order(left: &str, right: &str) -> Ordering {
    let (left_namespace, left_path) = split_resource_location(left);
    let (right_namespace, right_path) = split_resource_location(right);
    left_path
        .cmp(right_path)
        .then_with(|| left_namespace.cmp(right_namespace))
}

fn split_resource_location(value: &str) -> (&str, &str) {
    value.split_once(':').unwrap_or(("minecraft", value))
}

fn canonical_nbt_sha256(value: &bbb_protocol::NbtSummaryValue) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(canonical_snbt_string(value).as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub(super) async fn run(
    probe: &mut ProbeContext,
    packets_seen: &mut usize,
) -> Result<DimensionTransitionsReport> {
    settle_initial_batch(probe, packets_seen).await?;
    let initial = DimensionLevelReport::from_world(&probe.world)
        .context("dimension probe reached chunks without a level")?;
    if initial.session.dimension != "minecraft:overworld" {
        bail!(
            "dimension probe must start in minecraft:overworld, got {}",
            initial.session.dimension
        );
    }

    let mut legs = Vec::with_capacity(TRANSITION_TARGETS.len());
    for (target, command) in TRANSITION_TARGETS {
        send_command(probe, command).await?;
        legs.push(run_leg(probe, target, packets_seen).await?);
    }
    Ok(DimensionTransitionsReport { initial, legs })
}

async fn settle_initial_batch(probe: &mut ProbeContext, packets_seen: &mut usize) -> Result<()> {
    if probe.chunk_delivery.report().completed_a_batch && probe.player_loaded_sent {
        return Ok(());
    }
    let mut bundle = PlayPacketBundleAssembler::default();
    while !(probe.chunk_delivery.report().completed_a_batch && probe.player_loaded_sent) {
        pump_without_observation(probe, &mut bundle, packets_seen).await?;
    }
    Ok(())
}

async fn send_command(probe: &mut ProbeContext, command: &str) -> Result<()> {
    let (id, payload) = packets::encode_play_chat_command(&packets::ChatCommand {
        command: command.to_string(),
    });
    probe.conn.send_packet(id, &payload).await
}

async fn run_leg(
    probe: &mut ProbeContext,
    target: &str,
    packets_seen: &mut usize,
) -> Result<DimensionTransitionLegReport> {
    let respawns_before = probe.world.counters().respawns_received;
    let expected_chunk = ChunkPos { x: 0, z: 0 };
    let mut observer = LegObserver::new(target, expected_chunk);
    let mut bundle = PlayPacketBundleAssembler::default();
    while !observer.complete {
        let (packet_id, payload) = probe.session.read_packet_or_send_play_tick().await?;
        *packets_seen = packets_seen.saturating_add(1);
        if probe.state != ConnectionState::Play {
            bail!("dimension probe left play while transitioning to {target}");
        }
        let packet = probe.session.decode_play_packet(packet_id, &payload)?;
        if let Some(batch) = bundle.push(packet)? {
            for packet in batch {
                apply_leg_packet(probe, &mut observer, packet).await?;
            }
        }
    }

    let respawns_after = probe.world.counters().respawns_received;
    let level = DimensionLevelReport::from_world(&probe.world)
        .with_context(|| format!("dimension probe lost the level for {target}"))?;
    if level.session.dimension != target {
        bail!(
            "dimension probe completed {target} with level {}",
            level.session.dimension
        );
    }
    let respawn_count_delta = respawns_after.saturating_sub(respawns_before);
    if respawn_count_delta != 1 {
        bail!("dimension probe {target} saw {respawn_count_delta} respawns");
    }
    observer.report(respawn_count_delta, level)
}

async fn apply_leg_packet(
    probe: &mut ProbeContext,
    observer: &mut LegObserver,
    packet: packets::PlayClientbound,
) -> Result<()> {
    observer.before(probe, &packet)?;
    let inserted = probe.apply_observed_play_packet(packet.clone()).await?;
    observer.after(probe, &packet, inserted)
}

async fn pump_without_observation(
    probe: &mut ProbeContext,
    bundle: &mut PlayPacketBundleAssembler,
    packets_seen: &mut usize,
) -> Result<()> {
    let (packet_id, payload) = probe.session.read_packet_or_send_play_tick().await?;
    *packets_seen = packets_seen.saturating_add(1);
    match probe.session.decode_packet(packet_id, &payload)? {
        DecodedClientboundPacket::Login(packet) => {
            probe.handle_login_packet(packet).await?;
        }
        DecodedClientboundPacket::Configuration(packet) => {
            probe.handle_configuration_packet(packet).await?;
        }
        DecodedClientboundPacket::Play(packet) => {
            if let Some(batch) = bundle.push(packet)? {
                for packet in batch {
                    probe.apply_observed_play_packet(packet).await?;
                }
            }
        }
    }
    Ok(())
}

struct LegObserver {
    target: String,
    expected_chunk: ChunkPos,
    milestones: Vec<String>,
    data_to_keep: Option<i8>,
    target_chunk: Option<ChunkPos>,
    first_chunk: Option<ChunkPos>,
    player_position: Option<[f64; 3]>,
    open_batch: bool,
    target_in_open_batch: bool,
    complete: bool,
}

impl LegObserver {
    fn new(target: &str, expected_chunk: ChunkPos) -> Self {
        Self {
            target: target.to_string(),
            expected_chunk,
            milestones: Vec::new(),
            data_to_keep: None,
            target_chunk: None,
            first_chunk: None,
            player_position: None,
            open_batch: false,
            target_in_open_batch: false,
            complete: false,
        }
    }

    fn before(
        &mut self,
        probe: &mut ProbeContext,
        packet: &packets::PlayClientbound,
    ) -> Result<()> {
        if let packets::PlayClientbound::Respawn(respawn) = packet {
            self.observe_respawn(&respawn.common_spawn_info.dimension, respawn.data_to_keep)?;
            probe.player_loaded_sent = false;
        }
        Ok(())
    }

    fn observe_respawn(&mut self, dimension: &str, data_to_keep: i8) -> Result<()> {
        if self.data_to_keep.is_some() {
            bail!("dimension probe {} received a second Respawn", self.target);
        }
        if dimension != self.target {
            bail!(
                "dimension probe expected Respawn for {}, got {dimension}",
                self.target
            );
        }
        // ServerPlayer.java:1102-1105; ClientboundRespawnPacket.java:8-22
        if data_to_keep != 3 {
            bail!(
                "dimension probe {} Respawn data_to_keep was {data_to_keep}, expected 3",
                self.target
            );
        }
        self.data_to_keep = Some(data_to_keep);
        self.milestones.push("respawn".to_string());
        Ok(())
    }

    fn after(
        &mut self,
        probe: &ProbeContext,
        packet: &packets::PlayClientbound,
        inserted: Option<ChunkPos>,
    ) -> Result<()> {
        if self.data_to_keep.is_none() {
            return Ok(());
        }
        match packet {
            packets::PlayClientbound::PlayerPosition(_) => {
                let position = probe
                    .world
                    .local_player_pose()
                    .context("dimension probe applied PlayerPosition without a canonical pose")?
                    .position;
                let position = [position.x, position.y, position.z];
                // `Vec3Argument.vec3()` center-corrects,
                // so the whole `0 80 0` the leg teleports to lands on the block
                // centre in x and z.
                if position != [0.5, 80.0, 0.5] {
                    bail!(
                        "dimension probe {} player position was {position:?}, expected [0.5, 80.0, 0.5]",
                        self.target
                    );
                }
                self.player_position = Some(position);
                self.push_once("player_position")?;
            }
            packets::PlayClientbound::SetChunkCacheCenter(center) => {
                let chunk = ChunkPos {
                    x: center.chunk_x,
                    z: center.chunk_z,
                };
                if chunk != self.expected_chunk {
                    bail!(
                        "dimension probe {} cache center was {chunk:?}, expected {:?}",
                        self.target,
                        self.expected_chunk
                    );
                }
                let position_chunk = self
                    .player_position
                    .map(chunk_at_position)
                    .context("dimension probe received cache center before PlayerPosition")?;
                if chunk != position_chunk {
                    bail!(
                        "dimension probe {} cache center {chunk:?} does not contain player position {:?}",
                        self.target,
                        self.player_position
                    );
                }
                self.target_chunk = Some(chunk);
                self.push_once("set_chunk_cache_center")?;
            }
            packets::PlayClientbound::GameEvent(event)
                if event.event_id
                    == bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID =>
            {
                self.push_once("level_chunks_load_start")?;
            }
            packets::PlayClientbound::ChunkBatchStart => {
                if self.open_batch {
                    bail!("dimension probe {} nested chunk batches", self.target);
                }
                self.open_batch = true;
                self.target_in_open_batch = false;
                self.push_once("chunk_batch_start")?;
            }
            packets::PlayClientbound::LevelChunkWithLight(_) => {
                let chunk = inserted.context("dimension probe applied a chunk without its position")?;
                if self.first_chunk.is_none() {
                    self.first_chunk = Some(chunk);
                    self.milestones.push("first_chunk".to_string());
                }
                if chunk == self.expected_chunk && !self.target_in_open_batch {
                    if !self.open_batch {
                        bail!("dimension probe {} target chunk was outside a batch", self.target);
                    }
                    self.target_in_open_batch = true;
                    self.milestones.push("target_chunk".to_string());
                }
            }
            packets::PlayClientbound::ChunkBatchFinished { .. } => {
                if !self.open_batch {
                    bail!("dimension probe {} finished an unopened chunk batch", self.target);
                }
                self.open_batch = false;
                if self.target_in_open_batch {
                    self.milestones.push("chunk_batch_finished".to_string());
                    self.complete = true;
                }
            }
            _ => {}
        }
        if self.complete && !probe.player_loaded_sent {
            bail!(
                "dimension probe {} completed chunks before sending PlayerLoaded",
                self.target
            );
        }
        Ok(())
    }

    fn push_once(&mut self, marker: &str) -> Result<()> {
        if self.milestones.iter().any(|seen| seen == marker) {
            bail!("dimension probe {} repeated {marker}", self.target);
        }
        self.milestones.push(marker.to_string());
        Ok(())
    }

    fn report(
        self,
        respawn_count_delta: usize,
        level: DimensionLevelReport,
    ) -> Result<DimensionTransitionLegReport> {
        // ServerPlayer.java:1102-1129; ChunkMap.java:1037-1048; ChunkMap.java:1113-1122
        // PlayerList.java:644-656; PlayerChunkSender.java:50-78
        // ServerGamePacketListenerImpl.java:1246-1255
        const EXPECTED: [&str; 8] = [
            "respawn",
            "player_position",
            "set_chunk_cache_center",
            "level_chunks_load_start",
            "chunk_batch_start",
            "first_chunk",
            "target_chunk",
            "chunk_batch_finished",
        ];
        if self
            .milestones
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != EXPECTED
        {
            bail!(
                "dimension probe {} milestone order mismatch: {:?}",
                self.target,
                self.milestones
            );
        }
        Ok(DimensionTransitionLegReport {
            target_dimension: self.target,
            respawn_count_delta,
            data_to_keep: self
                .data_to_keep
                .context("dimension probe missed Respawn")?,
            level,
            player_position: self
                .player_position
                .context("dimension probe missed PlayerPosition")?,
            milestone_order: self.milestones,
            target_chunk: self
                .target_chunk
                .context("dimension probe missed cache center")?,
            first_chunk: self
                .first_chunk
                .context("dimension probe missed first chunk")?,
            target_chunk_completed: self.complete,
        })
    }
}

fn chunk_at_position(position: [f64; 3]) -> ChunkPos {
    ChunkPos {
        x: (position[0].floor() as i32).div_euclid(16),
        z: (position[2].floor() as i32).div_euclid(16),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_net::RawConnection;
    use bbb_protocol::{
        codec::Encoder,
        encode_nbt_summary,
        packets::{
            CommonPlayerSpawnInfo, GameEvent, LevelChunkData, LevelChunkWithLight, LightUpdateData,
            PlayClientbound, PlayLogin, PlayerPositionUpdate, RegistryDataEntry, RegistryTags,
            Respawn, SetChunkCacheCenter, TagNetworkPayload, Vec3d,
        },
        NbtSummaryEntry, NbtSummaryValue,
    };
    use tokio::net::TcpListener;

    fn full_capture() -> ConfigurationCapture {
        let mut capture = ConfigurationCapture::default();
        capture.select_known_packs(0).unwrap();
        for registry in VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
            capture.registry_data(&RegistryData {
                registry: registry.resource_id.to_string(),
                entries: vec![RegistryDataEntry {
                    id: "minecraft:base".to_string(),
                    raw_data: Some(
                        encode_nbt_summary(
                            &NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                                name: "value".into(),
                                value: NbtSummaryValue::Int(1),
                            }]),
                            1024,
                        )
                        .unwrap(),
                    ),
                }],
                raw_payload_len: 1,
            });
        }
        capture.update_tags(&UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:test".to_string(),
                    entries: vec![1, 0],
                }],
            }],
        });
        capture.packet_order.push("finish".to_string());
        capture
    }

    /// `serializeTagsToNetwork` and each registry's payload are both `HashMap`s
    /// so the wire order is a hash order
    /// two servers need not share.
    #[test]
    fn configuration_capture_publishes_tags_in_a_canonical_order() {
        let mut capture = full_capture();
        capture.tags.clear();
        capture
            .packet_order
            .retain(|step| step != "update_tags" && step != "finish");
        capture.update_tags(&UpdateTags {
            registries: vec![
                RegistryTags {
                    registry: "minecraft:item".to_string(),
                    tags: vec![
                        TagNetworkPayload {
                            tag: "minecraft:second".to_string(),
                            entries: vec![0],
                        },
                        TagNetworkPayload {
                            tag: "minecraft:first".to_string(),
                            entries: vec![1],
                        },
                    ],
                },
                RegistryTags {
                    registry: "minecraft:block".to_string(),
                    tags: vec![TagNetworkPayload {
                        tag: "minecraft:only".to_string(),
                        entries: vec![0],
                    }],
                },
            ],
        });
        capture.packet_order.push("finish".to_string());
        let report = capture
            .build(|_, holder| {
                Some(["minecraft:stone", "minecraft:dirt"][holder as usize].to_string())
            })
            .unwrap();
        assert_eq!(
            report
                .tags
                .iter()
                .map(|registry| registry.registry.as_str())
                .collect::<Vec<_>>(),
            ["minecraft:block", "minecraft:item"]
        );
        assert_eq!(
            report.tags[1]
                .tags
                .iter()
                .map(|tag| tag.tag.as_str())
                .collect::<Vec<_>>(),
            ["minecraft:first", "minecraft:second"]
        );
    }

    #[test]
    fn configuration_capture_requires_exact_registry_order_full_data_and_resolves_tags() {
        let capture = full_capture();
        let report = capture
            .build(|registry, holder| {
                (registry == "minecraft:item")
                    .then(|| ["minecraft:stone", "minecraft:dirt"][holder as usize].to_string())
            })
            .unwrap();
        assert_eq!(report.registries.len(), 28);
        assert_eq!(
            report
                .registries
                .iter()
                .map(|registry| registry.registry.as_str())
                .collect::<Vec<_>>(),
            [
                "minecraft:worldgen/biome",
                "minecraft:chat_type",
                "minecraft:trim_pattern",
                "minecraft:trim_material",
                "minecraft:wolf_variant",
                "minecraft:wolf_sound_variant",
                "minecraft:pig_variant",
                "minecraft:pig_sound_variant",
                "minecraft:frog_variant",
                "minecraft:cat_variant",
                "minecraft:cat_sound_variant",
                "minecraft:cow_sound_variant",
                "minecraft:cow_variant",
                "minecraft:chicken_sound_variant",
                "minecraft:chicken_variant",
                "minecraft:zombie_nautilus_variant",
                "minecraft:painting_variant",
                "minecraft:dimension_type",
                "minecraft:damage_type",
                "minecraft:banner_pattern",
                "minecraft:enchantment",
                "minecraft:jukebox_song",
                "minecraft:instrument",
                "minecraft:test_environment",
                "minecraft:test_instance",
                "minecraft:dialog",
                "minecraft:world_clock",
                "minecraft:timeline",
            ]
        );
        assert_eq!(
            report.tags[0].tags[0].entries,
            ["minecraft:dirt", "minecraft:stone"]
        );

        let mut stub = full_capture();
        stub.registries[0].entries[0].raw_data = None;
        assert!(stub.build(|_, _| None).is_err());

        let mut wrong_order = full_capture();
        wrong_order.registries.swap(0, 1);
        assert!(wrong_order.build(|_, _| None).is_err());

        let unresolved = full_capture();
        assert!(unresolved.build(|_, _| None).is_err());
    }

    #[test]
    fn configuration_capture_checks_entry_order_by_path_then_namespace() {
        let mut capture = full_capture();
        let data = capture.registries[0].entries[0].raw_data.clone().unwrap();
        capture.registries[0].entries = vec![
            RegistryDataEntry {
                id: "zeta:a".to_string(),
                raw_data: Some(data.clone()),
            },
            RegistryDataEntry {
                id: "alpha:b".to_string(),
                raw_data: Some(data),
            },
        ];
        assert!(capture
            .build(|registry, holder| {
                (registry == "minecraft:item")
                    .then(|| ["minecraft:stone", "minecraft:dirt"][holder as usize].to_string())
            })
            .is_ok());
        capture.registries[0].entries.swap(0, 1);
        assert!(capture.build(|_, _| None).is_err());
    }

    #[test]
    fn configuration_capture_canonical_nbt_digest_ignores_compound_entry_order() {
        let left = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "z".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "a".into(),
                value: NbtSummaryValue::string("same"),
            },
        ]);
        let right = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "a".into(),
                value: NbtSummaryValue::string("same"),
            },
            NbtSummaryEntry {
                name: "z".into(),
                value: NbtSummaryValue::Int(1),
            },
        ]);
        assert_eq!(canonical_nbt_sha256(&left), canonical_nbt_sha256(&right));
    }

    #[tokio::test]
    async fn dimension_transition_packet_path_reports_pose_order_and_frozen_initial_chunk() {
        let (mut probe, _server) = transition_probe().await;
        let initial_chunk = ChunkPos { x: 2, z: 0 };
        let initial_dimension = probe.world.dimension();
        probe
            .apply_observed_play_packet(PlayClientbound::LevelChunkWithLight(chunk_packet(
                initial_chunk,
                initial_dimension,
            )))
            .await
            .unwrap();

        let mut observer = LegObserver::new("minecraft:the_nether", ChunkPos { x: 0, z: 0 });
        apply_leg_packet(&mut probe, &mut observer, respawn_packet(3))
            .await
            .unwrap();
        apply_leg_packet(&mut probe, &mut observer, position_packet([0.5, 80.0, 0.5]))
            .await
            .unwrap();
        apply_leg_packet(&mut probe, &mut observer, cache_center_packet(0, 0))
            .await
            .unwrap();
        apply_leg_packet(&mut probe, &mut observer, load_start_packet())
            .await
            .unwrap();
        apply_leg_packet(&mut probe, &mut observer, PlayClientbound::ChunkBatchStart)
            .await
            .unwrap();
        let dimension = probe.world.dimension();
        apply_leg_packet(
            &mut probe,
            &mut observer,
            PlayClientbound::LevelChunkWithLight(chunk_packet(ChunkPos { x: 0, z: 0 }, dimension)),
        )
        .await
        .unwrap();
        apply_leg_packet(
            &mut probe,
            &mut observer,
            PlayClientbound::ChunkBatchFinished { batch_size: 1 },
        )
        .await
        .unwrap();

        let level = DimensionLevelReport::from_world(&probe.world).unwrap();
        let report = observer.report(1, level).unwrap();
        assert_eq!(report.data_to_keep, 3);
        assert_eq!(report.player_position, [0.5, 80.0, 0.5]);
        assert!(report.target_chunk_completed);

        let report = probe.finish(8, initial_chunk, Vec::new(), None, None);
        assert_eq!(report.first_chunk_summary.unwrap().pos, initial_chunk);
    }

    #[tokio::test]
    async fn dimension_transition_packet_path_rejects_invalid_sequences() {
        let (mut probe, _server) = transition_probe().await;
        let mut observer = LegObserver::new("minecraft:the_nether", ChunkPos { x: 0, z: 0 });
        assert!(
            apply_leg_packet(&mut probe, &mut observer, respawn_packet(2))
                .await
                .is_err()
        );

        let (mut probe, _server) = transition_probe().await;
        let mut observer = LegObserver::new("minecraft:the_nether", ChunkPos { x: 0, z: 0 });
        apply_leg_packet(&mut probe, &mut observer, respawn_packet(3))
            .await
            .unwrap();
        assert!(apply_leg_packet(
            &mut probe,
            &mut observer,
            position_packet([16.0, 80.0, 0.0]),
        )
        .await
        .is_err());

        let (mut probe, _server) = transition_probe().await;
        let mut observer = LegObserver::new("minecraft:the_nether", ChunkPos { x: 0, z: 0 });
        apply_leg_packet(&mut probe, &mut observer, respawn_packet(3))
            .await
            .unwrap();
        apply_leg_packet(&mut probe, &mut observer, position_packet([0.5, 80.0, 0.5]))
            .await
            .unwrap();
        assert!(
            apply_leg_packet(&mut probe, &mut observer, cache_center_packet(1, 0))
                .await
                .is_err()
        );

        let (mut probe, _server) = transition_probe().await;
        let mut observer = LegObserver::new("minecraft:the_nether", ChunkPos { x: 0, z: 0 });
        apply_leg_packet(&mut probe, &mut observer, respawn_packet(3))
            .await
            .unwrap();
        assert!(
            apply_leg_packet(&mut probe, &mut observer, cache_center_packet(0, 0))
                .await
                .is_err()
        );

        let (mut probe, _server) = transition_probe().await;
        let mut observer = LegObserver::new("minecraft:the_nether", ChunkPos { x: 0, z: 0 });
        apply_leg_packet(&mut probe, &mut observer, respawn_packet(3))
            .await
            .unwrap();
        apply_leg_packet(&mut probe, &mut observer, position_packet([0.5, 80.0, 0.5]))
            .await
            .unwrap();
        apply_leg_packet(&mut probe, &mut observer, cache_center_packet(0, 0))
            .await
            .unwrap();
        apply_leg_packet(&mut probe, &mut observer, load_start_packet())
            .await
            .unwrap();
        let dimension = probe.world.dimension();
        assert!(apply_leg_packet(
            &mut probe,
            &mut observer,
            PlayClientbound::LevelChunkWithLight(chunk_packet(ChunkPos { x: 0, z: 0 }, dimension,)),
        )
        .await
        .is_err());

        let (mut probe, _server) = transition_probe().await;
        let mut observer = LegObserver::new("minecraft:the_nether", ChunkPos { x: 0, z: 0 });
        apply_leg_packet(&mut probe, &mut observer, respawn_packet(3))
            .await
            .unwrap();
        assert!(
            apply_leg_packet(&mut probe, &mut observer, respawn_packet(3))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn dimension_transition_packet_path_requires_target_chunk_in_batch() {
        let (mut probe, _server) = transition_probe().await;
        let mut observer = LegObserver::new("minecraft:the_nether", ChunkPos { x: 0, z: 0 });
        for packet in [
            respawn_packet(3),
            position_packet([0.5, 80.0, 0.5]),
            cache_center_packet(0, 0),
            load_start_packet(),
            PlayClientbound::ChunkBatchStart,
        ] {
            apply_leg_packet(&mut probe, &mut observer, packet)
                .await
                .unwrap();
        }
        let dimension = probe.world.dimension();
        apply_leg_packet(
            &mut probe,
            &mut observer,
            PlayClientbound::LevelChunkWithLight(chunk_packet(ChunkPos { x: 1, z: 0 }, dimension)),
        )
        .await
        .unwrap();
        apply_leg_packet(
            &mut probe,
            &mut observer,
            PlayClientbound::ChunkBatchFinished { batch_size: 1 },
        )
        .await
        .unwrap();
        assert!(!observer.complete);
        let level = DimensionLevelReport::from_world(&probe.world).unwrap();
        assert!(observer.report(1, level).is_err());
    }

    async fn transition_probe() -> (ProbeContext, RawConnection) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = tokio::spawn(async move {
            RawConnection::connect(&address.to_string(), None)
                .await
                .unwrap()
        });
        let (server, _) = listener.accept().await.unwrap();
        let client = client.await.unwrap();
        let mut probe = ProbeContext::new(client);
        probe
            .apply_observed_play_packet(PlayClientbound::Login(login_packet()))
            .await
            .unwrap();
        (probe, RawConnection::from_stream(server, None))
    }

    fn login_packet() -> PlayLogin {
        PlayLogin {
            player_id: 1,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 2,
            simulation_distance: 2,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: spawn_info("minecraft:overworld", 0, 63),
            enforces_secure_chat: false,
        }
    }

    fn respawn_packet(data_to_keep: i8) -> PlayClientbound {
        PlayClientbound::Respawn(Respawn {
            common_spawn_info: spawn_info("minecraft:the_nether", 1, 32),
            data_to_keep,
        })
    }

    fn spawn_info(
        dimension: &str,
        dimension_type_id: i32,
        sea_level: i32,
    ) -> CommonPlayerSpawnInfo {
        CommonPlayerSpawnInfo {
            dimension_type_id,
            dimension: dimension.to_string(),
            seed: 0,
            game_type: 1,
            previous_game_type: 1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level,
        }
    }

    fn position_packet(position: [f64; 3]) -> PlayClientbound {
        PlayClientbound::PlayerPosition(PlayerPositionUpdate {
            id: 1,
            position: Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            delta_movement: Vec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        })
    }

    fn cache_center_packet(x: i32, z: i32) -> PlayClientbound {
        PlayClientbound::SetChunkCacheCenter(SetChunkCacheCenter {
            chunk_x: x,
            chunk_z: z,
        })
    }

    fn load_start_packet() -> PlayClientbound {
        PlayClientbound::GameEvent(GameEvent {
            event_id:
                bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
            param: 0.0,
        })
    }

    fn chunk_packet(
        pos: ChunkPos,
        dimension: bbb_client_world::WorldDimension,
    ) -> LevelChunkWithLight {
        let mut sections = Encoder::new();
        for _ in 0..dimension.section_count() {
            sections.write_i16(0);
            sections.write_i16(0);
            sections.write_u8(0);
            sections.write_var_i32(0);
            sections.write_u8(0);
            sections.write_var_i32(0);
        }
        LevelChunkWithLight {
            x: pos.x,
            z: pos.z,
            chunk_data: LevelChunkData {
                heightmaps: Vec::new(),
                section_data: sections.into_inner(),
                block_entities: Vec::new(),
            },
            light_data: LightUpdateData {
                sky_y_mask: Vec::new(),
                block_y_mask: Vec::new(),
                empty_sky_y_mask: Vec::new(),
                empty_block_y_mask: Vec::new(),
                sky_updates: Vec::new(),
                block_updates: Vec::new(),
            },
        }
    }
}
