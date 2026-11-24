//! `MinecraftServer.getScheduledEvents()` (TimerQueue.java:22-118).

use bbb_protocol::JavaString;
use bbb_server_persistence::scheduled_events_schema::Java26ScheduledEvent;
use std::collections::BTreeMap;

use crate::commands::{CommandScheduledCallback, CommandScheduledEvent};

/// `TimerQueue<MinecraftServer>` (TimerQueue.java:22-118).
#[derive(Clone, Debug, Default)]
pub(crate) struct ScheduledEvents {
    /// `TimerQueue.events` (TimerQueue.java:29): row is the schedule id, column is trigger time.
    events: BTreeMap<JavaString, BTreeMap<i64, u64>>,
    /// `TimerQueue.queue` (TimerQueue.java:27), ordered by trigger time then sequential id.
    queue: BTreeMap<(i64, u64), Java26ScheduledEvent>,
    sequential_id: u64,
}

impl ScheduledEvents {
    /// `TimerQueue.schedule` (TimerQueue.java:65-73).
    pub(crate) fn schedule(
        &mut self,
        id: JavaString,
        trigger_time: i64,
        callback: CommandScheduledCallback,
    ) {
        if self
            .events
            .get(&id)
            .is_some_and(|row| row.contains_key(&trigger_time))
        {
            return;
        }
        self.sequential_id = self.sequential_id.wrapping_add(1);
        let sequential_id = self.sequential_id;
        self.events
            .entry(id.clone())
            .or_default()
            .insert(trigger_time, sequential_id);
        self.queue.insert(
            (trigger_time, sequential_id),
            Java26ScheduledEvent {
                trigger_time,
                id,
                callback,
            },
        );
    }

    /// `TimerQueue.remove` (TimerQueue.java:75-82).
    pub(crate) fn remove(&mut self, id: &JavaString) -> i32 {
        let Some(row) = self.events.remove(id) else {
            return 0;
        };
        for (&time, &sequence) in &row {
            self.queue.remove(&(time, sequence));
        }
        i32::try_from(row.len()).unwrap_or(i32::MAX)
    }

    /// `TimerQueue.java:40-45`: packed order reconstructs sequential IDs.
    pub(crate) fn from_packed(events: &[Java26ScheduledEvent]) -> Self {
        let mut queue = Self::default();
        for event in events {
            queue.schedule(event.id.clone(), event.trigger_time, event.callback.clone());
        }
        queue
    }

    /// `TimerQueue.java:89-92`: equal-time events retain insertion order.
    pub(crate) fn pack(&self) -> Vec<Java26ScheduledEvent> {
        self.queue.values().cloned().collect()
    }

    /// `TimerQueue.java:51-62`: invoke each callback before polling the next.
    pub(crate) fn pop_due(&mut self, current_tick: i64) -> Option<CommandScheduledCallback> {
        let (&(trigger_time, _), _) = self.queue.first_key_value()?;
        if trigger_time > current_tick {
            return None;
        }
        let (_, event) = self.queue.pop_first()?;
        if let Some(row) = self.events.get_mut(&event.id) {
            // `:59` removes the current tick, which may differ after a clock jump.
            row.remove(&current_tick);
            if row.is_empty() {
                self.events.remove(&event.id);
            }
        }
        Some(event.callback)
    }

    pub(crate) fn project(&self) -> Vec<CommandScheduledEvent> {
        let mut projected: Vec<CommandScheduledEvent> = self
            .events
            .iter()
            .flat_map(|(id, times)| {
                times.keys().map(|&trigger_time| CommandScheduledEvent {
                    id: id.clone(),
                    trigger_time,
                })
            })
            .collect();
        projected.sort_by(|left, right| {
            left.trigger_time
                .cmp(&right.trigger_time)
                .then_with(|| left.id.cmp(&right.id))
        });
        projected
    }
}

impl super::ServerWorld {
    /// `ScheduleCommand.schedule`: `replace`
    /// drops the id's earlier triggers first.
    pub(crate) fn schedule_event(
        &mut self,
        id: String,
        trigger_time: i64,
        callback: CommandScheduledCallback,
        replace: bool,
    ) {
        let id: JavaString = id.into();
        if replace {
            let _ = self.cold.scheduled_events.remove(&id);
        }
        self.cold
            .scheduled_events
            .schedule(id, trigger_time, callback);
    }

    /// `TimerQueue.remove`'s count, which `/schedule clear` reports.
    pub(crate) fn clear_scheduled_events(&mut self, id: &JavaString) -> i32 {
        self.cold.scheduled_events.remove(id)
    }

    #[cfg(test)]
    pub(crate) fn projected_scheduled_events_for_test(&self) -> Vec<CommandScheduledEvent> {
        self.cold.scheduled_events.project()
    }
}
