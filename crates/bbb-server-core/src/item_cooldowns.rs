//! `ItemCooldowns` as `ServerItemCooldowns` runs it for one session: the
//! per-group timers a `use_cooldown` component starts, the tick that expires
//! them, and the `ClientboundCooldownPacket` each edge owes the client.

use std::collections::BTreeMap;

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID;
use bbb_protocol::packets::{Cooldown, ItemStackSummary};
use bbb_server_types::item_stack::static_item_component;
use bbb_server_types::Result;

/// `ItemCooldowns.CooldownInstance` (`ItemCooldowns.java:74-75`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CooldownInstance {
    start_time: i32,
    end_time: i32,
}

/// `ItemCooldowns` (`ItemCooldowns.java:13-76`) with `ServerItemCooldowns`'s
/// two hooks (`ServerItemCooldowns.java:15-24`) answered as the packets they
/// send rather than sent, so the world can route them to the session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemCooldowns {
    cooldowns: BTreeMap<String, CooldownInstance>,
    tick_count: i32,
}

impl ItemCooldowns {
    /// `ItemCooldowns.isOnCooldown` (`ItemCooldowns.java:17-19`).
    pub fn is_on_cooldown(&self, item: &ItemStackSummary) -> Result<bool> {
        Ok(self.cooldown_percent(item, 0.0)? > 0.0)
    }

    /// `ItemCooldowns.getCooldownPercent` (`ItemCooldowns.java:21-31`).
    pub fn cooldown_percent(&self, item: &ItemStackSummary, partial_tick: f32) -> Result<f32> {
        let group = cooldown_group(item)?;
        Ok(match self.cooldowns.get(group) {
            Some(cooldown) => {
                let duration = (cooldown.end_time - cooldown.start_time) as f32;
                let remaining = cooldown.end_time as f32 - (self.tick_count as f32 + partial_tick);
                (remaining / duration).clamp(0.0, 1.0)
            }
            None => 0.0,
        })
    }

    /// `ItemCooldowns.tick` (`ItemCooldowns.java:33-46`): the counter climbs,
    /// and every group whose end has come is dropped and ended, which on the
    /// server is a zero-duration `ClientboundCooldownPacket` each
    /// (`ServerItemCooldowns.onCooldownEnded`, `ServerItemCooldowns.java:21-24`).
    pub fn tick(&mut self) -> Vec<Cooldown> {
        self.tick_count += 1;
        let tick_count = self.tick_count;
        let mut ended = Vec::new();
        self.cooldowns.retain(|group, cooldown| {
            if cooldown.end_time <= tick_count {
                ended.push(Cooldown {
                    cooldown_group: group.clone(),
                    duration: 0,
                });
                false
            } else {
                true
            }
        });
        ended
    }

    /// `ItemCooldowns.addCooldown(ItemStack, int)` (`ItemCooldowns.java:54-61`):
    /// the group's timer restarts at `tickCount` and the start is announced
    /// (`ServerItemCooldowns.onCooldownStarted`, `ServerItemCooldowns.java:15-18`).
    pub fn add_cooldown(&mut self, item: &ItemStackSummary, ticks: i32) -> Result<Cooldown> {
        Ok(self.add_group_cooldown(cooldown_group(item)?.to_owned(), ticks))
    }

    /// `ItemCooldowns.addCooldown(Identifier, int)` (`ItemCooldowns.java:58-61`).
    pub fn add_group_cooldown(&mut self, group: String, ticks: i32) -> Cooldown {
        self.cooldowns.insert(
            group.clone(),
            CooldownInstance {
                start_time: self.tick_count,
                end_time: self.tick_count + ticks,
            },
        );
        Cooldown {
            cooldown_group: group,
            duration: ticks,
        }
    }
}

/// `ItemCooldowns.getCooldownGroup` (`ItemCooldowns.java:48-52`): the
/// `use_cooldown` component's group, else the item's own registry key.
pub fn cooldown_group(item: &ItemStackSummary) -> Result<&str> {
    Ok(
        static_item_component(item, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID)?
            .and_then(|cooldown| cooldown.use_cooldown_group.as_deref())
            .unwrap_or_else(|| {
                item.item_id
                    .and_then(bbb_protocol::item_types::vanilla_item_type)
                    .map(|entry| entry.resource_id)
                    // `BuiltInRegistries.ITEM.getKey(Items.AIR)` for the empty stack.
                    .unwrap_or("minecraft:air")
            }),
    )
}

#[cfg(test)]
mod tests {
    use bbb_protocol::item_types::{VANILLA_ITEM_ENDER_PEARL_ID, VANILLA_ITEM_SNOWBALL_ID};
    use bbb_protocol::packets::DataComponentPatchSummary;

    use super::*;

    fn stack(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }
    }

    #[test]
    fn a_cooldown_runs_for_its_ticks_and_ends_with_a_zero_duration_packet() {
        let mut cooldowns = ItemCooldowns::default();
        let pearl = stack(VANILLA_ITEM_ENDER_PEARL_ID);
        assert!(!cooldowns.is_on_cooldown(&pearl).unwrap());
        let started = cooldowns.add_cooldown(&pearl, 20).unwrap();
        assert_eq!(
            started,
            Cooldown {
                cooldown_group: "minecraft:ender_pearl".to_string(),
                duration: 20,
            }
        );
        assert!(cooldowns.is_on_cooldown(&pearl).unwrap());
        // The group is the pearl's own, so a snowball is not held up by it.
        assert!(!cooldowns
            .is_on_cooldown(&stack(VANILLA_ITEM_SNOWBALL_ID))
            .unwrap());
        for tick in 1..20 {
            assert!(cooldowns.tick().is_empty(), "tick {tick}");
            assert!(cooldowns.is_on_cooldown(&pearl).unwrap(), "tick {tick}");
            let percent = cooldowns.cooldown_percent(&pearl, 0.0).unwrap();
            assert!(
                (percent - (20 - tick) as f32 / 20.0).abs() < 1e-6,
                "tick {tick}"
            );
        }
        // `endTime <= tickCount` on the twentieth tick drops it and ends it.
        assert_eq!(
            cooldowns.tick(),
            vec![Cooldown {
                cooldown_group: "minecraft:ender_pearl".to_string(),
                duration: 0,
            }]
        );
        assert!(!cooldowns.is_on_cooldown(&pearl).unwrap());
        assert!(cooldowns.tick().is_empty());
    }

    #[test]
    fn restarting_a_group_moves_its_end_and_announces_the_new_duration() {
        let mut cooldowns = ItemCooldowns::default();
        let pearl = stack(VANILLA_ITEM_ENDER_PEARL_ID);
        cooldowns.add_cooldown(&pearl, 20).unwrap();
        for _ in 0..10 {
            cooldowns.tick();
        }
        let restarted = cooldowns.add_cooldown(&pearl, 5).unwrap();
        assert_eq!(restarted.duration, 5);
        // `(endTime - (tickCount + a)) / (endTime - startTime)` off the new
        // instance: five of five remain.
        assert_eq!(cooldowns.cooldown_percent(&pearl, 0.0).unwrap(), 1.0);
        for _ in 0..4 {
            assert!(cooldowns.tick().is_empty());
        }
        assert_eq!(cooldowns.tick().len(), 1);
    }
}
