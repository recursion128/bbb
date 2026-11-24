use crate::connection::ConnectionId;
use crate::loot_runtime::{matches_conditions, param_sets};
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::registry_codec::canonical_advancement_player_predicate_from_source;
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::IntBounds;
use bbb_server_datapack::loot::{parse_loot_condition_nbt, EntityPredicate, LootCondition};

use super::{int_bounds, ServerWorld, WorldEvent};

pub(super) const CONSTRUCT_BEACON: &str = "minecraft:construct_beacon";

pub(super) struct BeaconCondition {
    level: IntBounds,
    player: Vec<LootCondition>,
}

impl BeaconCondition {
    // ConstructBeaconTrigger.java:20-27; MinMaxBounds.java:103-115,268-272.
    pub(super) fn parse(
        value: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        if !value.is_compound()
            && !matches!(value, V::Json(bbb_registry_codec::RegistryJsonValue::Null))
        {
            return Err("construct_beacon conditions must be an object".into());
        }
        Ok(Self {
            level: int_bounds(value.field("level"))?,
            player: player_conditions(value.field("player"), CONSTRUCT_BEACON, lookup)?,
        })
    }
}

pub(super) fn player_conditions(
    value: Option<V<'_>>,
    trigger: &str,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<Vec<LootCondition>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let canonical = canonical_advancement_player_predicate_from_source(
        value,
        bbb_server_datapack::loot::codec::source(value),
    )
    .map_err(|error| error.to_string())?;
    player_conditions_from_canonical(Some(&canonical), trigger, lookup)
}

pub(super) fn player_conditions_from_canonical(
    value: Option<&NbtSummaryValue>,
    trigger: &str,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<Vec<LootCondition>, String> {
    let conditions = match value {
        None => Vec::new(),
        Some(NbtSummaryValue::List(values)) => values
            .iter()
            .map(|value| parse_loot_condition_nbt(value, lookup))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?,
        Some(predicate @ NbtSummaryValue::Compound(_)) => {
            vec![LootCondition::EntityProperties {
                entity: bbb_registry_codec::loot_context_types::VanillaLootContextParam::ThisEntity,
                predicate: Some(Box::new(
                    EntityPredicate::decode(V::Nbt(predicate))
                        .map_err(|error| error.to_string())?,
                )),
            }]
        }
        Some(_) => return Err("canonical player predicate is not a compound or list".into()),
    };
    super::warn_condition_check(trigger, &conditions, param_sets::ADVANCEMENT_ENTITY);
    Ok(conditions)
}

impl ServerWorld {
    // ConstructBeaconTrigger.java:16-18,37-39; SimpleCriterionTrigger.java:43-70.
    pub(in crate::world) fn trigger_construct_beacon(
        &mut self,
        connection_id: ConnectionId,
        levels: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(CONSTRUCT_BEACON)
            .next()
            .is_none()
        {
            return;
        }
        let Some(startup) = self.cold.config.datapacks.as_ref() else {
            return;
        };
        let Some(matching) = self.with_advancement_player_params(player, |level, params| {
            let advancement_triggers = &self.cold.advancement_triggers;
            let mut matching = Vec::new();
            for (index, criterion) in player.cold.advancements.trigger_listeners(CONSTRUCT_BEACON) {
                let Some(condition) = advancement_triggers
                    .construct_beacon
                    .get(&index)
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    continue;
                };
                if !condition.level.matches(levels) {
                    continue;
                }
                if matches_conditions(
                    &startup.loot,
                    CONSTRUCT_BEACON,
                    &condition.player,
                    &params,
                    &level.random,
                ) == Ok(true)
                {
                    matching.push((index, criterion.clone()));
                }
            }
            matching
        }) else {
            return;
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, OnceLock};

    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
    use serde_json::{json, Value};

    use super::*;
    use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
    use crate::world::ServerWorldConfig;

    const PLAYER: ConnectionId = ConnectionId(1);

    fn world(definitions: &[(&str, Value)]) -> ServerWorld {
        static BUILTIN: OnceLock<DatapackStartup> = OnceLock::new();
        let mut startup = BUILTIN
            .get_or_init(|| {
                DatapackStartup::load(&DatapackStartupOptions {
                    datapacks_dir: None,
                    saved_data_packs: None,
                    safe_mode: false,
                    limits: Default::default(),
                })
                .unwrap()
            })
            .clone();
        startup.advancements = AdvancementSet {
            advancements: definitions
                .iter()
                .map(|(id, body)| AdvancementElement {
                    id: (*id).into(),
                    source_pack: "test".into(),
                    advancement: parse_advancement(body).unwrap(),
                })
                .collect(),
            warnings: Vec::new(),
        };
        let mut world = ServerWorld::new(ServerWorldConfig {
            datapacks: Some(Arc::new(startup)),
            ..Default::default()
        })
        .unwrap();
        let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
        {
            let mut position = world.levels[PLAYER].position();
            position.y = 64.0;
            world.levels[PLAYER].set_position(position);
        }
        world
    }

    fn advancement(conditions: Value, experience: i32) -> Value {
        json!({"criteria":{"built":{"trigger":"construct_beacon","conditions":conditions}},
            "rewards":{"experience":experience}})
    }

    fn fire(world: &mut ServerWorld, levels: i32) {
        world.trigger_construct_beacon(PLAYER, levels, &mut Vec::new());
    }

    #[test]
    fn construct_beacon_level_bounds_award_matching_listeners_once() {
        let mut world = world(&[
            ("test:any", advancement(json!({}), 5)),
            (
                "test:range",
                advancement(json!({"level":{"min":2,"max":3}}), 3),
            ),
            ("test:four", advancement(json!({"level":4}), 2)),
        ]);
        fire(&mut world, 1);
        assert_eq!(world.levels[PLAYER].experience.total(), 5);
        assert!(!world.player_advancement_is_done(PLAYER, "test:range"));
        fire(&mut world, 3);
        assert_eq!(world.levels[PLAYER].experience.total(), 8);
        fire(&mut world, 4);
        fire(&mut world, 4);
        assert_eq!(world.levels[PLAYER].experience.total(), 10);
        assert_eq!(
            world.levels[PLAYER]
                .cold
                .advancements
                .trigger_listeners(CONSTRUCT_BEACON)
                .count(),
            0
        );
    }

    #[test]
    fn construct_beacon_player_object_and_list_use_real_session_predicates() {
        let predicate = json!({"type":"minecraft:player","distance":{"absolute":{"max":0}},
            "nbt":"{SelectedItem:{id:'minecraft:stick',count:1}}",
            "location":{"position":{"y":64}},"flags":{"is_on_ground":true}});
        let mut world = world(&[
            (
                "test:object",
                advancement(json!({"level":4,"player":predicate}), 2),
            ),
            (
                "test:list",
                advancement(
                    json!({"level":4,"player":[{
                        "condition":"entity_properties","entity":"this","predicate":predicate
                    }]}),
                    3,
                ),
            ),
        ]);
        let stick = crate::entity::item::plain_stack("minecraft:stick", 1).unwrap();
        world.levels[PLAYER].inventory.set_item_in_hand(
            bbb_protocol::packets::InteractionHand::MainHand,
            stick.clone(),
        );
        world.levels[PLAYER].set_on_ground(false);
        fire(&mut world, 4);
        assert_eq!(world.levels[PLAYER].experience.total(), 0);
        world.levels[PLAYER].set_on_ground(true);
        fire(&mut world, 3);
        assert_eq!(world.levels[PLAYER].experience.total(), 0);
        world.levels[PLAYER].inventory.set_item_in_hand(
            bbb_protocol::packets::InteractionHand::MainHand,
            crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
        );
        fire(&mut world, 4);
        assert_eq!(world.levels[PLAYER].experience.total(), 0);
        world.levels[PLAYER]
            .inventory
            .set_item_in_hand(bbb_protocol::packets::InteractionHand::MainHand, stick);
        fire(&mut world, 4);
        fire(&mut world, 4);
        assert_eq!(world.levels[PLAYER].experience.total(), 5);
        assert!(world.player_advancement_is_done(PLAYER, "test:object"));
        assert!(world.player_advancement_is_done(PLAYER, "test:list"));
    }
}
