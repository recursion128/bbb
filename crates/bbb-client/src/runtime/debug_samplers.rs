use super::*;

#[derive(Debug, Clone, Default)]
pub(crate) struct HudDebugFpsSampler {
    window_started_at: Option<Instant>,
    last_frame_at: Option<Instant>,
    frames_in_window: u32,
    last_reported_fps: u32,
    frame_time_nanos: Vec<u64>,
}

impl HudDebugFpsSampler {
    pub(crate) fn record_frame(&mut self, now: Instant) {
        if let Some(last_frame_at) = self.last_frame_at {
            let nanos = now
                .saturating_duration_since(last_frame_at)
                .as_nanos()
                .min(u128::from(u64::MAX)) as u64;
            self.frame_time_nanos.push(nanos);
            if self.frame_time_nanos.len() > HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY {
                let overflow = self.frame_time_nanos.len() - HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY;
                self.frame_time_nanos.drain(0..overflow);
            }
        }
        self.last_frame_at = Some(now);

        let Some(started_at) = self.window_started_at else {
            self.window_started_at = Some(now);
            self.frames_in_window = 1;
            return;
        };

        self.frames_in_window = self.frames_in_window.saturating_add(1);
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed < Duration::from_secs(1) {
            return;
        }

        let fps = self.frames_in_window as f64 / elapsed.as_secs_f64();
        self.last_reported_fps = fps.round().clamp(0.0, u32::MAX as f64) as u32;
        self.window_started_at = Some(now);
        self.frames_in_window = 0;
    }

    pub(crate) fn fps(&self) -> u32 {
        self.last_reported_fps
    }

    pub(crate) fn frame_time_nanos(&self) -> Vec<u64> {
        self.frame_time_nanos.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HudDebugNetworkSampler {
    last_sample_at: Option<Instant>,
    pending_bandwidth_bytes: u64,
    inbound_metrics: Option<std::sync::Arc<bbb_client_net::InboundPacketMetrics>>,
    ping_millis: Vec<u64>,
    bandwidth_bytes_per_tick: Vec<u64>,
}

impl HudDebugNetworkSampler {
    pub(super) fn record_net_event(&mut self, event: &NetEvent) {
        match event {
            NetEvent::InboundMetrics(metrics) => {
                self.inbound_metrics = Some(std::sync::Arc::clone(metrics));
            }
            NetEvent::Play(bbb_protocol::packets::PlayClientbound::PongResponse(response)) => {
                self.record_pong_response(response.time, wall_clock_millis_i64());
            }
            NetEvent::Disconnected { .. } => {
                self.reset();
            }
            _ => {}
        }
    }

    pub(super) fn drain_inbound_metrics(&mut self) -> (u64, u64) {
        let Some(metrics) = self.inbound_metrics.as_ref() else {
            return (0, 0);
        };
        let (packets, bytes) = metrics.take_delta();
        if bytes > 0 {
            self.pending_bandwidth_bytes = self.pending_bandwidth_bytes.saturating_add(bytes);
        }
        (packets, bytes)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn record_received_packet(&mut self, len: usize) {
        self.pending_bandwidth_bytes = self
            .pending_bandwidth_bytes
            .saturating_add(u64::try_from(len).unwrap_or(u64::MAX));
    }

    pub(super) fn record_pong_response(&mut self, sent_millis: i64, received_millis: i64) {
        let latency = received_millis.saturating_sub(sent_millis).max(0) as u64;
        push_capped_hud_debug_sample(
            &mut self.ping_millis,
            latency,
            HUD_DEBUG_NETWORK_SAMPLE_CAPACITY,
        );
    }

    pub(super) fn advance_tick(
        &mut self,
        input: &ClientInputState,
        net_counters: &NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        now: Instant,
        now_millis: i64,
    ) {
        if !net_counters.connected {
            self.reset();
            return;
        }

        let ticks = match self.last_sample_at {
            Some(last_sample_at) => {
                let elapsed = now.saturating_duration_since(last_sample_at);
                let ticks = elapsed.as_millis() / HUD_DEBUG_NETWORK_SAMPLE_INTERVAL.as_millis();
                if ticks == 0 {
                    return;
                }
                ticks.min(HUD_DEBUG_NETWORK_SAMPLE_CAPACITY as u128) as usize
            }
            None => 1,
        };
        self.last_sample_at = Some(now);

        for tick in 0..ticks {
            let bytes = if tick == 0 {
                std::mem::take(&mut self.pending_bandwidth_bytes)
            } else {
                0
            };
            push_capped_hud_debug_sample(
                &mut self.bandwidth_bytes_per_tick,
                bytes,
                HUD_DEBUG_NETWORK_SAMPLE_CAPACITY,
            );
            if input.debug_network_charts_visible() && net_counters.state.as_deref() == Some("Play")
            {
                if let Some(tx) = net_commands {
                    let _ = tx.try_send(NetCommand::PingRequest(now_millis));
                }
            }
        }
    }

    pub(super) fn reset(&mut self) {
        self.last_sample_at = None;
        self.pending_bandwidth_bytes = 0;
        self.inbound_metrics = None;
        self.ping_millis.clear();
        self.bandwidth_bytes_per_tick.clear();
    }

    pub(super) fn ping_millis(&self) -> Vec<u64> {
        self.ping_millis.clone()
    }

    pub(super) fn bandwidth_bytes_per_tick(&self) -> Vec<u64> {
        self.bandwidth_bytes_per_tick.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HudDebugTpsSampler {
    samples: Vec<HudDebugTpsSample>,
}

pub(super) const DEBUG_COLLISION_REFRESH_INTERVAL: Duration = Duration::from_millis(100);
pub(super) const DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL: Duration = Duration::from_millis(100);
pub(super) const DEBUG_SUPPORTING_BLOCK_QUERY_RADIUS: f64 = 16.0;

#[derive(Debug, Default)]
pub(crate) struct DebugCollisionCache {
    last_update_at: Option<Instant>,
    lines: Vec<DebugLine>,
}

#[derive(Debug, Default)]
pub(crate) struct DebugSupportingBlockCache {
    last_update_at: Option<Instant>,
    entity_ids: Vec<i32>,
}

impl DebugCollisionCache {
    pub(super) fn lines(
        &mut self,
        input: &ClientInputState,
        world: &WorldStore,
        now: Instant,
    ) -> Vec<DebugLine> {
        if !debug_collision_entry_enabled(input, world) {
            self.reset();
            return Vec::new();
        }
        let refresh = self.last_update_at.is_none_or(|last_update_at| {
            now.saturating_duration_since(last_update_at) > DEBUG_COLLISION_REFRESH_INTERVAL
        });
        if refresh {
            self.last_update_at = Some(now);
            self.lines = collect_debug_collision_lines(world);
        }
        self.lines.clone()
    }

    pub(super) fn reset(&mut self) {
        self.last_update_at = None;
        self.lines.clear();
    }
}

impl DebugSupportingBlockCache {
    pub(super) fn lines(
        &mut self,
        input: &ClientInputState,
        world: &WorldStore,
        now: Instant,
    ) -> Vec<DebugLine> {
        if !debug_supporting_block_entry_enabled(input, world) {
            self.reset();
            return Vec::new();
        }
        let refresh = self.last_update_at.is_none_or(|last_update_at| {
            now.saturating_duration_since(last_update_at) > DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL
        });
        if refresh {
            self.last_update_at = Some(now);
            self.entity_ids =
                world.debug_supporting_block_entity_ids(DEBUG_SUPPORTING_BLOCK_QUERY_RADIUS);
        }
        collect_debug_supporting_block_lines(world, &self.entity_ids)
    }

    pub(super) fn reset(&mut self) {
        self.last_update_at = None;
        self.entity_ids.clear();
    }
}

impl HudDebugTpsSampler {
    pub(super) fn record_net_event(&mut self, event: &NetEvent) {
        match event {
            NetEvent::Play(bbb_protocol::packets::PlayClientbound::DebugSample(sample))
                if sample.sample_type == RemoteDebugSampleType::TickTime =>
            {
                self.record_debug_sample(&sample.sample);
            }
            NetEvent::Disconnected { .. } => {
                self.reset();
            }
            _ => {}
        }
    }

    pub(super) fn record_debug_sample(&mut self, sample: &[i64]) {
        let value =
            |index: usize| -> u64 { sample.get(index).copied().unwrap_or_default().max(0) as u64 };
        push_capped_hud_debug_sample(
            &mut self.samples,
            HudDebugTpsSample {
                full_tick_nanos: value(0),
                tick_server_method_nanos: value(1),
                scheduled_tasks_nanos: value(2),
                idle_nanos: value(3),
            },
            HUD_DEBUG_TPS_SAMPLE_CAPACITY,
        );
    }

    pub(super) fn sync_subscriptions_and_tick(
        &mut self,
        input: &ClientInputState,
        world: &mut WorldStore,
        net_counters: &NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        if !net_counters.connected {
            self.reset();
            world.clear_debug_subscriber();
            return;
        }
        if net_counters.state.as_deref() != Some("Play") {
            return;
        }

        let desired = input
            .remote_debug_renderers()
            .subscription_ids(input.debug_fps_charts_visible());
        if world
            .requested_debug_subscriptions()
            .iter()
            .copied()
            .eq(desired.iter().copied())
        {
            // `ClientDebugSubscriber.tick` changes the request set before
            // purging against the current pre-increment level game time.
            let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
            world.tick_debug_subscriber(game_time);
            return;
        }
        if let Some(tx) = net_commands {
            if tx
                .try_send(NetCommand::DebugSubscriptionRequest {
                    subscription_ids: desired.clone(),
                })
                .is_ok()
            {
                world
                    .set_requested_debug_subscriptions(&desired)
                    .expect("startup-derived debug subscriptions must use generated ids");
            }
        }
        let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
        world.tick_debug_subscriber(game_time);
    }

    pub(super) fn chart(&self, world: &WorldStore) -> Option<HudDebugTpsChart> {
        (!self.samples.is_empty()).then(|| HudDebugTpsChart {
            samples: self.samples.clone(),
            milliseconds_per_tick: world_ticking_milliseconds_per_tick(world),
        })
    }

    #[cfg(test)]
    pub(super) fn samples(&self) -> Vec<HudDebugTpsSample> {
        self.samples.clone()
    }

    pub(super) fn reset(&mut self) {
        self.samples.clear();
    }
}
