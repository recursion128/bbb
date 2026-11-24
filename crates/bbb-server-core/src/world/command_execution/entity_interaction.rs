use super::*;
use crate::commands::CommandPlayerTarget;
use crate::world::level_blocks::{LoadedBlocks, LootChunks};
use crate::world::{block_position, LevelId};

use crate::world::simulation_level::mob_death_loot::LevelLootEnvironment;
use crate::world::ServerLevel;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
use bbb_server_types::item_stack::RuntimeItemStack;

mod allay;
mod living_item;
#[cfg(test)]
mod tests;

impl ServerWorld {
    pub(crate) fn mount_command_entity(
        &mut self,
        target: &CommandAttributeTarget,
        vehicle: &CommandAttributeTarget,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let CommandAttributeTarget::Stored {
            dimension,
            entity_id: vehicle_id,
        } = vehicle
        else {
            return false;
        };
        let dimension = self.require_level_id(dimension);
        let mounted = match target {
            CommandAttributeTarget::Player(player) => {
                let Some(connection) = self.command_player_connection(player) else {
                    return false;
                };
                self.start_external_player_ride_checked(
                    connection,
                    &dimension,
                    *vehicle_id,
                    crate::entity::passenger::RelationAdmission::Forced,
                    None,
                    events,
                )
                .unwrap_or(false)
            }
            CommandAttributeTarget::Stored { entity_id, .. } => {
                let level = self.level_mut_by_id(&dimension);
                if !level
                    .entities
                    .link_existing_passenger_forced(*vehicle_id, *entity_id)
                {
                    return false;
                }
                level
                    .entities
                    .get_mut(*entity_id)
                    .expect("mounted passenger exists")
                    .set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID);
                let position = level
                    .entities
                    .get(*vehicle_id)
                    .expect("mounted vehicle exists")
                    .position();
                level.event_sink.game_event_with_source_at(
                    crate::level_event::VanillaGameEvent::ENTITY_MOUNT,
                    position,
                    Some(*entity_id),
                    None,
                );
                for rider in self.sessions_riding(&dimension, *vehicle_id) {
                    self.fire_player_trigger(rider, "minecraft:started_riding", events);
                }
                true
            }
        };
        self.apply_level_entity_lifecycle_by_key(&dimension, events);
        mounted
    }

    pub(crate) fn dismount_command_entity(
        &mut self,
        target: &CommandAttributeTarget,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = match target {
            CommandAttributeTarget::Player(player) => {
                let Some(connection) = self.command_player_connection(player) else {
                    return;
                };
                let dimension = self.levels.player_at(connection).dimension;
                let _ = self.stop_external_player_ride(
                    &dimension,
                    connection,
                    crate::world::external_player_ride_events::DismountLanding::Search,
                    events,
                );
                dimension
            }
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => {
                let dimension = self.require_level_id(dimension);
                self.stop_stored_command_ride(&dimension, *entity_id);
                dimension
            }
        };
        self.apply_level_entity_lifecycle_by_key(&dimension, events);
    }

    pub(in crate::world) fn interact_with_stored_leash_target(
        &mut self,
        connection_id: crate::connection::ConnectionId,
        packet: bbb_protocol::packets::InteractEntity,
        dimension: LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.interact_with_stored_leash_target_checked(
            connection_id,
            packet,
            dimension,
            events,
        );
    }

    pub(in crate::world) fn interact_with_stored_leash_target_checked(
        &mut self,
        connection_id: crate::connection::ConnectionId,
        packet: bbb_protocol::packets::InteractEntity,
        dimension: LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let resolver = self.item_stack_resolver();
        let hand = packet.hand;
        let (original_hand, player_entity_id, player_uuid, player_position, infinite_materials) = {
            let player = self.levels.player_at(connection_id);
            let stack = match hand {
                InteractionHand::MainHand => player.inventory.selected_stack().clone(),
                InteractionHand::OffHand => player.inventory.off_hand_stack().clone(),
            };
            (
                stack,
                player.entity_id(),
                player.profile.uuid,
                player.position(),
                player.abilities.instabuild,
            )
        };
        let Some((target_position_before, target_block_pos, may_remove_target)) = self
            .level_by_id(&dimension)
            .entities
            .get(packet.entity_id)
            .map(|target| {
                (
                    target.position(),
                    target.block_position(),
                    matches!(target.data(), crate::entity::EntityData::LeashKnot(_))
                        || (target.entity_type_id()
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MOOSHROOM_ID
                            && original_hand.item_id()
                                == Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID)),
                )
            })
        else {
            return Some(());
        };
        let captured_target = (may_remove_target
            && self.levels[connection_id]
                .cold
                .advancements
                .trigger_listeners("minecraft:player_interacted_with_entity")
                .next()
                .is_some())
        .then(|| self.killed_entity_of_stored(&dimension, packet.entity_id));
        let original_summary = original_hand.summary().clone();
        let mut hand_stack = original_summary.clone();
        let enchantment_registry = Arc::clone(&self.datapack_tables.enchantment_registry);
        let registry_tags = self
            .level_by_id(&dimension)
            .chunks
            .registry_tags_shared()
            .map(crate::entity::EntityRegistryTagView::Frozen)
            .unwrap_or(crate::entity::EntityRegistryTagView::Builtin26_1);
        let game_time = self.game_time();
        let interacting_player = (original_summary.item_id
            == Some(bbb_protocol::item_types::VANILLA_ITEM_BRUSH_ID))
        .then(|| {
            let player = self.levels.player_at(connection_id);
            (self.loot_view_for_player(player), player.inventory.clone())
        });
        let spawn_egg_level = crate::entity::held_is_spawn_egg(&original_summary).then(|| {
            let chunks = &self.level_by_id(&dimension).chunks;
            let pos = target_block_pos;
            let environment = self.cold.dimension_sky_lights.spawn_context(
                self.cold
                    .level_dimension_types
                    .get(&dimension)
                    .map(String::as_str),
                &self.registries,
                &self.world_clocks,
            );
            crate::entity::SpawnEggLevel {
                biome: chunks
                    .biome_id_at(pos.x, pos.y, pos.z)
                    .and_then(|id| {
                        self.registries
                            .registry_element_id("minecraft:worldgen/biome", id)
                    })
                    .map(str::to_owned),
                special_multiplier: environment
                    .overworld_clock_time()
                    .and_then(|clock| {
                        environment.special_multiplier(
                            chunks,
                            pos,
                            self.admin.settings().difficulty,
                            clock,
                        )
                    })
                    .unwrap_or(0.0),
            }
        });
        let mob_item_used;
        let outcome = {
            let (level, drop_tables, loot_level_facts, _structure_services) =
                self.level_and_drop_tables(&dimension);
            let ServerLevel {
                chunks,
                entities,
                random: world_random,
                world_border,
                weather,
                ..
            } = level;
            let chunks = std::cell::RefCell::new(chunks);

            let player_sources = crate::world::loot_context::PlayerLootNbtSources {
                snapshot: loot_level_facts.entity_nbt.as_ref(),
                resolver: loot_level_facts.item_stack_resolver.as_ref(),
                dimension: loot_level_facts.dimension.as_ref(),
                bodies: None,
                other_levels: loot_level_facts.other_levels.map_or(
                    crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(&[]),
                    crate::world::loot_context::entity_scoreboard::LootEntityLevels::Other,
                ),
            };
            let player_slots = |uuid, slot| {
                if let Some((_, inventory)) =
                    interacting_player.as_ref().filter(|_| uuid == player_uuid)
                {
                    Ok(crate::world::loot_context::player_nbt::read_inventory_slot(
                        inventory,
                        slot,
                        loot_level_facts
                            .enchantment_slots
                            .as_deref()
                            .map_or(&[], Vec::as_slice),
                    ))
                } else {
                    player_sources.slot(uuid, slot)
                }
            };
            let environment = LevelLootEnvironment::for_level(
                drop_tables.recipes,
                LootChunks::Chunks(&chunks),
                &loot_level_facts,
                None,
            )
            .map(|environment| environment.with_player_slots(&player_slots));
            let collisions = |entity: &crate::entity::Entity, query| {
                Some(
                    bbb_server_types::geometry::collect_block_collision_shapes_checked(
                        query,
                        |pos| {
                            crate::world::movement_collision::moving_piston_at(
                                &chunks.borrow(),
                                pos,
                            )
                        },
                        |pos| {
                            crate::world::movement_collision::stored_growth_collision_shape(
                                &chunks.borrow(),
                                entity,
                                pos,
                                None,
                            )
                        },
                    )?
                    .into_iter()
                    .map(|(_, shape)| shape)
                    .collect(),
                )
            };
            let border = || Some(*world_border);
            let clock = || game_time;
            let growth = crate::entity::living::dimensions_growth::SizeGrowthCollisionView {
                collisions: &collisions,
                border: &border,
                game_time: &clock,
            };
            let mut player = crate::entity::leash::LeashInteractingPlayer {
                mob_item_used: false,
                hand_stack: &mut hand_stack,
                entity_id: player_entity_id,
                uuid: player_uuid,
                position: player_position,
                secondary_use_active: packet.using_secondary_action,
                infinite_materials,
            };
            let outcome = crate::entity::leash::interact_leash_with_loot(
                entities,
                packet.entity_id,
                &mut player,
                crate::entity::leash::EntityInteractionLoot {
                    dimension: loot_level_facts
                        .dimension_key
                        .as_deref()
                        .map(|key| key.as_str()),
                    weather: crate::loot_runtime::LootWeather {
                        raining: weather.is_raining(),
                        thundering: weather.is_thundering(),
                    },
                    interacting_entity: interacting_player
                        .as_ref()
                        .map_or(crate::loot_runtime::LootEntity::Absent, |(facts, _)| {
                            crate::loot_runtime::LootEntity::Facts(facts)
                        }),
                    tables: drop_tables,
                    level: Some(&|pos| {
                        LoadedBlocks {
                            chunks: &chunks.borrow(),
                        }
                        .loaded_block_state(pos)
                    }),
                    block_entity_data: Some(&|pos| {
                        let resolver =
                            loot_level_facts
                                .item_stack_resolver
                                .as_ref()
                                .ok_or_else(|| {
                                    crate::loot_runtime::LootRuntimeReason::Catalog(
                                        "block entity NBT publication is unavailable".into(),
                                    )
                                })?;
                        chunks
                            .borrow()
                            .block_entity_loot_data(pos, Some(resolver))
                            .map_err(|error| {
                                crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                            })
                    }),
                    item_environment: environment.as_ref().map(|environment| {
                        environment as &dyn crate::loot_runtime::LootItemEnvironment
                    }),
                    enchantment_registry: &enchantment_registry,
                    registry_tags: &registry_tags,
                    level_random: Some(&*world_random),
                    growth: Some(&growth),
                    spawn_egg: spawn_egg_level.as_ref(),
                },
            );
            mob_item_used = player.mob_item_used;
            outcome
        };
        let mut leftovers = Vec::new();
        let admitted = if let crate::entity::leash::LeashInteraction::MooshroomStew { item }
        | crate::entity::leash::LeashInteraction::BucketedMob { item } = &outcome
        {
            if !self.create_filled_result_with(
                connection_id,
                packet.hand,
                RuntimeItemStack::from_summary(item.clone()),
                false,
            ) {
                return Some(());
            }
            Ok(())
        } else {
            (|| -> bbb_server_types::Result<()> {
                let inventory = &mut self.levels[connection_id].inventory;
                if infinite_materials
                    && !matches!(
                        &outcome,
                        crate::entity::leash::LeashInteraction::MountEquipped { .. }
                    )
                    && hand_stack.count < original_summary.count
                    && (hand_stack.item_id == original_summary.item_id
                        || hand_stack.item_id.is_none())
                {
                    hand_stack = original_summary.clone();
                }
                let replaced = hand_stack.item_id != original_summary.item_id
                    && hand_stack.item_id.is_some()
                    && hand_stack.count > 0;
                let hand = RuntimeItemStack::from_summary(hand_stack.clone());
                if replaced {
                    inventory.consume_in_hand(packet.hand, 1, infinite_materials);
                    inventory.set_item_in_hand(packet.hand, hand);
                } else {
                    *inventory.item_in_hand_mut(packet.hand) = hand;
                }
                let remainder = match &outcome {
                    crate::entity::leash::LeashInteraction::Tamed { remainder, .. }
                    | crate::entity::leash::LeashInteraction::TamingFailed { remainder, .. } => {
                        remainder.clone()
                    }
                    crate::entity::leash::LeashInteraction::FedWithRemainder { item, .. } => {
                        Some(item.clone())
                    }
                    crate::entity::leash::LeashInteraction::Milked
                        if infinite_materials
                            || inventory.item_in_hand_mut(packet.hand).item_id()
                                != Some(bbb_protocol::item_types::VANILLA_ITEM_MILK_BUCKET_ID) =>
                    {
                        Some(ItemStackSummary {
                            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_MILK_BUCKET_ID),
                            count: 1,
                            component_patch: Default::default(),
                        })
                    }
                    _ => None,
                };
                if let Some(remainder) = remainder {
                    let mut extra = RuntimeItemStack::from_summary(remainder);
                    if matches!(&outcome, crate::entity::leash::LeashInteraction::Milked)
                        && !infinite_materials
                    {
                        let empty = {
                            let held = inventory.item_in_hand_mut(packet.hand);
                            let count = held.count() - 1;
                            held.set_count(count);
                            held.is_empty()
                        };
                        if empty {
                            inventory.set_item_in_hand(packet.hand, extra);
                            return Ok(());
                        }
                    }
                    let mut already_owned = false;
                    if infinite_materials {
                        for owned in inventory.inventory_slots() {
                            if !owned.is_empty()
                                && owned.same_item_same_effective_components(&extra, &resolver)?
                            {
                                already_owned = true;
                                break;
                            }
                        }
                    }
                    if !already_owned {
                        inventory.add_anywhere(&resolver, &mut extra, infinite_materials)?;
                        if !infinite_materials && !extra.is_empty() {
                            leftovers.push(extra);
                        }
                    }
                }
                Ok(())
            })()
        };
        if let Err(error) = admitted {
            tracing::warn!("entity interaction item admission: {error}");
            return Some(());
        }
        if mob_item_used {
            let used = RuntimeItemStack::from_summary(hand_stack.clone());
            match self.apply_use_remainder(connection_id, &used, &original_hand) {
                Ok(crate::world::using_item::HeldItem::Replaced(stack)) => {
                    self.set_player_item_in_hand(connection_id, hand, stack, events);
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!("entity interaction use remainder: {error}");
                    return Some(());
                }
            }
        }
        let succeeded = !matches!(
            outcome,
            crate::entity::leash::LeashInteraction::Pass
                | crate::entity::leash::LeashInteraction::Refused
        );
        let removes_target = matches!(
            outcome,
            crate::entity::leash::LeashInteraction::BucketedMob { .. }
                | crate::entity::leash::LeashInteraction::ParrotPoisoned { .. }
        );
        let without_item = matches!(
            outcome,
            crate::entity::leash::LeashInteraction::BoundHeld
                | crate::entity::leash::LeashInteraction::Unleashed
        ) || matches!(
            outcome,
            crate::entity::leash::LeashInteraction::SitToggled
        ) && self
            .level_by_id(&dimension)
            .entities
            .get(packet.entity_id)
            .is_some_and(|target| {
                target.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID
            });
        let awarded_item = if without_item {
            &ItemStackSummary::empty()
        } else {
            &original_summary
        };
        if matches!(
            outcome,
            crate::entity::leash::LeashInteraction::BucketedMob { .. }
        ) {
            self.trigger_player_entity_interaction(
                connection_id,
                "minecraft:player_interacted_with_entity",
                dimension,
                packet.entity_id,
                awarded_item,
                events,
            );
        }
        if let crate::entity::leash::LeashInteraction::BucketedMob { item } = &outcome {
            self.trigger_item_criterion(
                connection_id,
                crate::world::advancement_triggers::FILLED_BUCKET,
                item,
                events,
            );
            crate::entity::bucketable::finish_capture(
                &mut self.level_mut_by_id(&dimension).entities,
                packet.entity_id,
            );
        }
        for leftover in leftovers {
            self.drop_player_original_stack(connection_id, leftover, false)?;
        }
        if let crate::entity::leash::LeashInteraction::ParrotPoisoned { entity_id } = &outcome {
            let entity_id = *entity_id;
            let creative = self.levels.player_at(connection_id).game_type == GameType::Creative;
            let hurt = creative
                || self
                    .level_by_id(&dimension)
                    .entities
                    .get(entity_id)
                    .is_some_and(|target| !target.invulnerable());
            let poison = crate::entity::ActiveMobEffect::new("minecraft:poison", 900, 0)?;
            let _ = self.add_stored_mob_effect(&dimension, entity_id, poison);
            self.trigger_player_entity_interaction(
                connection_id,
                "minecraft:player_interacted_with_entity",
                dimension,
                entity_id,
                awarded_item,
                events,
            );
            if hurt {
                let source = self.pending_hurt_source_entity(
                    &dimension,
                    crate::level_event::PendingHurtEntity::Player { uuid: player_uuid },
                )?;
                let _ = self.hurt_stored_entity_by_command(
                    dimension,
                    entity_id,
                    "minecraft:player_attack",
                    f32::MAX,
                    Some(source.clone()),
                    Some(source),
                    None,
                    events,
                );
            }
        }
        // Match the emission after interaction; removal keeps the last live origin.
        let target_position = self
            .level_by_id(&dimension)
            .entities
            .get(packet.entity_id)
            .map_or(target_position_before, crate::entity::Entity::position);
        if matches!(outcome, crate::entity::leash::LeashInteraction::Pass)
            && self.is_merchant_target(&dimension, packet.entity_id)
        {
            self.apply_level_entity_lifecycle_by_key(&dimension, events);
            if self.merchant_mob_interact(connection_id, dimension, packet.entity_id, hand, events)
            {
                self.trigger_player_entity_interaction(
                    connection_id,
                    "minecraft:player_interacted_with_entity",
                    dimension,
                    packet.entity_id,
                    &original_summary,
                    events,
                );
                return Some(());
            }
        }
        if matches!(outcome, crate::entity::leash::LeashInteraction::Pass) {
            self.interact_with_living_item(
                connection_id,
                dimension,
                &LivingItemInteractArgs {
                    target_id: packet.entity_id,
                    hand,
                    original: &original_hand,
                    tags: &registry_tags,
                },
                events,
            )?;
        }
        self.finish_leash_interaction(
            connection_id,
            dimension,
            LeashInteractionFinish {
                outcome,
                target_position,
                target_block_pos,
                off_hand: hand == InteractionHand::OffHand,
            },
            events,
        )?;
        if succeeded && !removes_target {
            if let Some(target) = captured_target.filter(|_| {
                self.level_by_id(&dimension)
                    .entities
                    .get(packet.entity_id)
                    .is_none()
            }) {
                self.trigger_captured_entity_interaction(
                    connection_id,
                    target,
                    awarded_item,
                    events,
                );
            } else {
                self.trigger_player_entity_interaction(
                    connection_id,
                    "minecraft:player_interacted_with_entity",
                    dimension,
                    packet.entity_id,
                    awarded_item,
                    events,
                );
            }
        }
        Some(())
    }
}

#[derive(Clone, Copy)]
pub(super) struct LivingItemInteractArgs<'a> {
    pub(super) target_id: i32,
    pub(super) hand: InteractionHand,
    pub(super) original: &'a RuntimeItemStack,
    pub(super) tags: &'a crate::entity::EntityRegistryTagView,
}

#[derive(Clone)]
pub(in crate::world) struct LeashInteractionFinish {
    pub(in crate::world) outcome: crate::entity::leash::LeashInteraction,
    pub(in crate::world) target_position: Vec3d,
    pub(in crate::world) target_block_pos: BlockPos,
    pub(in crate::world) off_hand: bool,
}

impl ServerWorld {
    pub(in crate::world) fn finish_leash_interaction(
        &mut self,
        connection_id: crate::connection::ConnectionId,
        dimension: LevelId,
        finish: LeashInteractionFinish,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let LeashInteractionFinish {
            outcome,
            target_position,
            target_block_pos,
            off_hand,
        } = finish;
        use crate::level_event::VanillaGameEvent;
        self.apply_level_entity_lifecycle_by_key(&dimension, events);
        let player = self.levels.player_at(connection_id);
        let user = NameAndId {
            id: player.profile.uuid,
            name: (&player.profile.name).into(),
        };
        match outcome {
            crate::entity::leash::LeashInteraction::Pass
            | crate::entity::leash::LeashInteraction::Refused
            | crate::entity::leash::LeashInteraction::Consumed => {}
            crate::entity::leash::LeashInteraction::KnotTransferred => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::BLOCK_ATTACH,
                    target_position,
                    None,
                );
            }
            crate::entity::leash::LeashInteraction::BoundHeld => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_ACTION,
                    Vec3d {
                        x: f64::from(target_block_pos.x) + 0.5,
                        y: f64::from(target_block_pos.y) + 0.5,
                        z: f64::from(target_block_pos.z) + 0.5,
                    },
                    None,
                );
                self.swing_arm_by_command(
                    &crate::commands::CommandPlayerTarget {
                        profile: user.clone(),
                    },
                    off_hand,
                    events,
                );
            }
            crate::entity::leash::LeashInteraction::Unleashed
            | crate::entity::leash::LeashInteraction::Named => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
            }
            crate::entity::leash::LeashInteraction::SpawnedOffspring
            | crate::entity::leash::LeashInteraction::FellInLove
            | crate::entity::leash::LeashInteraction::MountFed
            | crate::entity::leash::LeashInteraction::AnimalFedByServer => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
                self.swing_arm_by_command(
                    &crate::commands::CommandPlayerTarget {
                        profile: user.clone(),
                    },
                    off_hand,
                    events,
                );
            }
            crate::entity::leash::LeashInteraction::Mounted {
                vehicle_entity_id,
                rotation,
            } => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
                self.start_external_player_ride_checked(
                    connection_id,
                    &dimension,
                    vehicle_entity_id,
                    crate::entity::passenger::RelationAdmission::Normal,
                    rotation,
                    events,
                )?;
            }
            crate::entity::leash::LeashInteraction::OpenMountInventory { vehicle_entity_id } => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
                self.open_mount_inventory(connection_id, vehicle_entity_id, events);
            }
            crate::entity::leash::LeashInteraction::MadeMad
            | crate::entity::leash::LeashInteraction::GrewUp
            | crate::entity::leash::LeashInteraction::DolphinFed
            | crate::entity::leash::LeashInteraction::AgeLockToggled
            | crate::entity::leash::LeashInteraction::CollarDyed
            | crate::entity::leash::LeashInteraction::AnimalFed
            | crate::entity::leash::LeashInteraction::Repaired
            | crate::entity::leash::LeashInteraction::CopperGolemReturnedItem
            | crate::entity::leash::LeashInteraction::ParrotPoisoned { .. }
            | crate::entity::leash::LeashInteraction::SitToggled
            | crate::entity::leash::LeashInteraction::PandaStoodUp => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
            }
            crate::entity::leash::LeashInteraction::MountEquipped { entity_id, effects } => {
                let level = self.level_mut_by_id(&dimension);
                if let Some(sound) = effects.sound {
                    level.entities.play_entity_sound(sound);
                }
                if let Some(event) = effects.equip_event {
                    level.event_sink.game_event_with_source_at(
                        event,
                        target_position,
                        Some(entity_id),
                        None,
                    );
                }
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
                self.apply_level_entity_lifecycle_by_key(&dimension, events);
            }
            // `SUCCESS_SERVER` adds a server swing to the interaction event.
            crate::entity::leash::LeashInteraction::Tamed { server_swing, .. }
            | crate::entity::leash::LeashInteraction::TamingFailed { server_swing, .. }
            | crate::entity::leash::LeashInteraction::FedWithRemainder { server_swing, .. }
            | crate::entity::leash::LeashInteraction::ZombieVillagerCure {
                started: server_swing,
            } => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
                if server_swing {
                    self.swing_arm_by_command(
                        &crate::commands::CommandPlayerTarget {
                            profile: user.clone(),
                        },
                        off_hand,
                        events,
                    );
                }
            }
            crate::entity::leash::LeashInteraction::ShearedEquipment { entity_id, item } => {
                let hand = if off_hand {
                    InteractionHand::OffHand
                } else {
                    InteractionHand::MainHand
                };
                self.hurt_and_break_in_hand(connection_id, hand, 1, events);
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::SHEAR,
                    target_position,
                    None,
                );
                self.trigger_player_entity_interaction(
                    connection_id,
                    "minecraft:player_sheared_equipment",
                    dimension,
                    entity_id,
                    item.summary(),
                    events,
                );
            }
            crate::entity::leash::LeashInteraction::Sheared => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::SHEAR,
                    target_position,
                    None,
                );
                let hand = if off_hand {
                    InteractionHand::OffHand
                } else {
                    InteractionHand::MainHand
                };
                self.hurt_and_break_in_hand(connection_id, hand, 1, events);
            }
            crate::entity::leash::LeashInteraction::CopperGolemWeatherChanged(change) => {
                use crate::entity::copper_golem::WeatherChange;
                // CopperGolem.java:231-253.
                self.level_mut_by_id(&dimension).event_sink.level_event(
                    match change {
                        WeatherChange::Waxed => 3003,
                        WeatherChange::Unwaxed => 3004,
                        WeatherChange::Scraped => 3005,
                    },
                    target_block_pos,
                    0,
                );
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
                if change != WeatherChange::Waxed {
                    let hand = if off_hand {
                        InteractionHand::OffHand
                    } else {
                        InteractionHand::MainHand
                    };
                    self.hurt_and_break_in_hand(connection_id, hand, 1, events);
                }
                self.swing_arm_by_command(
                    &crate::commands::CommandPlayerTarget {
                        profile: user.clone(),
                    },
                    off_hand,
                    events,
                );
            }
            crate::entity::leash::LeashInteraction::IgnitedCreeper { damage_tool } => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
                if damage_tool {
                    let hand = if off_hand {
                        InteractionHand::OffHand
                    } else {
                        InteractionHand::MainHand
                    };
                    self.hurt_and_break_in_hand(connection_id, hand, 1, events);
                }
            }
            crate::entity::leash::LeashInteraction::BrushedArmadillo => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
                let hand = if off_hand {
                    InteractionHand::OffHand
                } else {
                    InteractionHand::MainHand
                };
                self.hurt_and_break_in_hand(
                    connection_id,
                    hand,
                    crate::entity::armadillo::BRUSH_DAMAGE,
                    events,
                );
            }
            crate::entity::leash::LeashInteraction::ShearedMob { server_swing } => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::SHEAR,
                    target_position,
                    None,
                );
                let hand = if off_hand {
                    InteractionHand::OffHand
                } else {
                    InteractionHand::MainHand
                };
                self.hurt_and_break_in_hand(connection_id, hand, 1, events);
                if server_swing {
                    self.swing_arm_by_command(
                        &crate::commands::CommandPlayerTarget {
                            profile: user.clone(),
                        },
                        off_hand,
                        events,
                    );
                }
            }
            crate::entity::leash::LeashInteraction::AttachedToPlayer => {
                self.swing_arm_by_command(
                    &crate::commands::CommandPlayerTarget {
                        profile: user.clone(),
                    },
                    off_hand,
                    events,
                );
            }
            crate::entity::leash::LeashInteraction::Milked
            | crate::entity::leash::LeashInteraction::MooshroomStew { .. }
            | crate::entity::leash::LeashInteraction::BucketedMob { .. }
            | crate::entity::leash::LeashInteraction::MooshroomFed => {
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::ENTITY_INTERACT,
                    target_position,
                    None,
                );
            }
        }
        Some(())
    }
}

impl ServerWorld {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn teleport_stored_entity_by_command(
        &mut self,
        dimension: &LevelId,
        level: &LevelId,
        entity_id: i32,
        position: [f64; 3],
        rotation: [f32; 2],
        relatives_mask: i32,
        look_at: Option<CommandLookAt>,
        teleport_command_tail: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let [x, y, z] = position;
        let [y_rot, x_rot] = rotation;
        let destination = PlayerPositionState {
            position: Vec3d { x, y, z },
            delta_movement: Vec3d::default(),
            y_rot,
            x_rot,
        };
        let transition = crate::entity::StoredEntityTransition {
            position: destination.position,
            delta_movement: destination.delta_movement,
            y_rot,
            x_rot,
            relatives_mask,
        };
        let Some(source) = self.levels.get(dimension) else {
            return;
        };
        if !self.has_level_id(level) {
            return;
        }
        let Ok(Some(tree)) = super::super::entity_transfer::resolved_tree(
            &source.entities,
            entity_id,
            transition,
            true,
        ) else {
            return;
        };
        let chunks = tree
            .iter()
            .map(|(_, position, _)| {
                let pos = block_position(*position);
                (pos.x >> 4, pos.z >> 4)
            })
            .collect::<Vec<_>>();
        if self.install_full_chunks_by_key(level, &chunks).is_err() {
            return;
        }
        let Ok(Some(entity_id)) =
            self.teleport_entity_subtree(*dimension, *level, entity_id, transition, false, events)
        else {
            return;
        };
        let look_at_point = look_at
            .as_ref()
            .and_then(|look_at| self.command_look_at_point(look_at));
        let level = self.level_mut_by_id(level);
        let chunks = &level.chunks;
        let Some(entity) = level.entities.get_mut(entity_id) else {
            return;
        };
        // `:253-255`.
        if let Some((from_anchor, point, _)) = look_at_point {
            entity.look_at(from_anchor, point);
        }
        if !teleport_command_tail {
            return;
        }
        // `:257-260`. The delta movement the transition left is already zero on
        // every axis the relative set did not carry over.
        if !entity.is_fall_flying() {
            let delta = entity.delta_movement();
            entity.set_delta_movement(Vec3d { y: 0.0, ..delta });
            set_spawn_on_ground_with_moving_pistons(
                entity,
                &|pos| {
                    crate::world::movement_collision::static_collision_shape_with_live_piston(
                        chunks,
                        pos,
                        stored_block_state(chunks, pos),
                    )
                },
                &|pos| {
                    crate::world::movement_collision::moving_piston_at(chunks, pos).unwrap_or(true)
                },
            );
        }
        // `:262-264`.
        level.entities.stop_mob_navigation(entity_id);
    }

    fn command_look_at_point(
        &self,
        look_at: &CommandLookAt,
    ) -> Option<(EntityAnchor, Vec3d, Option<PlayerLookAtTarget>)> {
        match look_at {
            CommandLookAt::Position {
                position: [x, y, z],
                from_anchor,
            } => Some((
                *from_anchor,
                Vec3d {
                    x: *x,
                    y: *y,
                    z: *z,
                },
                None,
            )),
            CommandLookAt::Entity {
                target,
                anchor,
                from_anchor,
            } => {
                let reads = self.entity_reads();
                let entity = match target {
                    CommandAttributeTarget::Player(target) => {
                        crate::commands::types::CommandDamageEntityRef::Player(
                            reads
                                .players()
                                .find(|player| player.id == target.profile.id)?,
                        )
                    }
                    CommandAttributeTarget::Stored {
                        dimension,
                        entity_id,
                    } => reads.command_entity_by_id(dimension, *entity_id)?,
                };
                let [x, y, z] = entity.anchored_position(*anchor);
                let entity_id = match entity {
                    crate::commands::types::CommandDamageEntityRef::Player(player) => {
                        player.entity_id
                    }
                    crate::commands::types::CommandDamageEntityRef::Stored(stored) => {
                        stored.entity.id()
                    }
                };
                Some((
                    *from_anchor,
                    Vec3d { x, y, z },
                    Some(PlayerLookAtTarget {
                        entity_id,
                        to_anchor: *anchor,
                    }),
                ))
            }
        }
    }

    pub(crate) fn look_at_command_entity(
        &mut self,
        target: &CommandAttributeTarget,
        look_at: CommandLookAt,
        events: &mut Vec<WorldEvent>,
    ) {
        match target {
            CommandAttributeTarget::Player(player) => {
                if let Some(connection) = self.command_player_connection(player) {
                    self.look_at_by_command(connection, look_at, events);
                }
            }
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => {
                let Some((from_anchor, position, _)) = self.command_look_at_point(&look_at) else {
                    return;
                };
                let level = self.require_level_id(dimension);
                if let Some(entity) = self.level_mut_by_id(&level).entities.get_mut(*entity_id) {
                    entity.look_at(from_anchor, position);
                }
            }
        }
    }

    pub(in crate::world) fn teleport_entity_by_command(
        &mut self,
        player: &CommandPlayerTarget,
        level: LevelId,
        teleport: &crate::commands::TeleportMove,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(target) = self.command_player_connection(player) else {
            return;
        };
        if self
            .levels
            .player(target)
            .is_some_and(|player| player.cold.sleeping_pos.is_some())
        {
            self.stop_player_sleep(target, true, true, events);
        }
        self.set_player_camera(target, None, events);
        let connection_id = target;
        let [x, y, z] = teleport.position;
        let [y_rot, x_rot] = teleport.rotation;
        let moved = self.teleport_player_id(
            connection_id,
            TeleportTransition {
                new_level: level,
                position: Vec3d { x, y, z },
                y_rot,
                x_rot,
                relatives_mask: teleport.relatives_mask,
            },
            events,
        );
        // `:252`'s `if`: everything below runs only for a teleport that landed.
        if !moved {
            return;
        }
        // `:253-255`.
        if let Some(look_at) = teleport.look_at.clone() {
            self.look_at_by_command(connection_id, look_at, events);
        }
        // TeleportCommand.java:257-260.
        let player = &mut self.levels[connection_id];
        if teleport.teleport_command_tail
            && !player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING)
        {
            player.set_delta_movement(Vec3d {
                y: 0.0,
                ..player.delta_movement()
            });
            player.set_on_ground(true);
        }
    }

    pub(crate) fn swing_arm_by_command(
        &mut self,
        player: &CommandPlayerTarget,
        off_hand: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(target) = self.command_player_connection(player) else {
            return;
        };
        let hand = if off_hand {
            InteractionHand::OffHand
        } else {
            InteractionHand::MainHand
        };
        self.player_swing(target, hand, true, events);
    }

    pub(crate) fn swing_entity_by_command(
        &mut self,
        target: &crate::commands::CommandAttributeTarget,
        off_hand: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        match target {
            crate::commands::CommandAttributeTarget::Player(player) => {
                self.swing_arm_by_command(player, off_hand, events);
            }
            crate::commands::CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => {
                let dimension = self.require_level_id(dimension);
                let store = &mut self.level_mut_by_id(&dimension).entities;
                let hand = if off_hand {
                    InteractionHand::OffHand
                } else {
                    InteractionHand::MainHand
                };
                let swung = store
                    .get_mut(*entity_id)
                    .and_then(crate::entity::Entity::living_mut)
                    .is_some_and(|living| living.swing(hand));
                if swung {
                    store.broadcast_to_tracking(
                        *entity_id,
                        ClientboundPlayPacket::EntityAnimation(EntityAnimation {
                            id: *entity_id,
                            action: if off_hand {
                                ANIMATE_SWING_OFF_HAND
                            } else {
                                ANIMATE_SWING_MAIN_HAND
                            },
                        }),
                    );
                    self.apply_level_entity_lifecycle_by_key(&dimension, events);
                }
            }
        }
    }

    pub(in crate::world) fn player_swing(
        &mut self,
        target: ConnectionId,
        hand: InteractionHand,
        send_to_self: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(state) = self.levels.player_mut(target) else {
            return;
        };
        if state.swinging
            && state.swing_time < state.current_swing_duration() / 2
            && state.swing_time >= 0
        {
            return;
        }
        state.swing_time = -1;
        state.swinging = true;
        state.swinging_arm = hand;
        let entity_id = state.entity_id();
        // `:1999`: `MAIN_HAND` is action 0 and `OFF_HAND` is action 3.
        let packet = ClientboundPlayPacket::EntityAnimation(EntityAnimation {
            id: entity_id,
            action: match hand {
                InteractionHand::MainHand => ANIMATE_SWING_MAIN_HAND,
                InteractionHand::OffHand => ANIMATE_SWING_OFF_HAND,
            },
        });
        if send_to_self {
            self.send_to_player_tracking_and_self(target, entity_id, packet, events);
        } else {
            self.send_to_player_tracking(target, entity_id, &packet, events);
        }
    }

    /// `Entity.forceSetRotation` (`Entity.java:3091-3100`).
    /// The base method resolves the two relative flags against the entity's
    /// current rotation (`:3092-3095`, `PositionMoveRotation.calculateAbsolute`)
    /// and stores the absolute result; the override then sends the client the
    /// *unresolved* four values, so the client applies the same arithmetic to
    /// the rotation it holds.
    ///
    /// **Not carried.** `setYHeadRot` (`:3097`) and `setOldRot` (`:3099`) write
    /// the head yaw and the previous-tick rotation, neither of which this world
    /// keeps for a player -- the same two `look_at_by_command` leaves out.
    pub(crate) fn force_set_rotation_by_command(
        &mut self,
        target: &CommandAttributeTarget,
        y_rot: f32,
        relative_y: bool,
        x_rot: f32,
        relative_x: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let absolute = |old_yaw, old_pitch| PlayerRotation {
            y_rot: wrap_degrees(if relative_y { old_yaw + y_rot } else { y_rot }),
            x_rot: wrap_degrees(if relative_x { old_pitch + x_rot } else { x_rot }),
        };
        let player = match target {
            CommandAttributeTarget::Player(player) => player,
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => {
                let level = self.require_level_id(dimension);
                if let Some(entity) = self.level_mut_by_id(&level).entities.get_mut(*entity_id) {
                    let rotation = absolute(entity.y_rot(), entity.x_rot());
                    entity.set_rot(rotation.y_rot, rotation.x_rot);
                    entity.set_y_head_rot(rotation.y_rot);
                }
                return;
            }
        };
        let Some(connection_id) = self.command_player_connection(player) else {
            return;
        };
        let state = self
            .levels
            .player_mut(connection_id)
            .expect("the connection was just resolved");
        state.set_rotation(absolute(state.rotation().y_rot, state.rotation().x_rot));
        events.push(send(
            state.connection_id,
            ClientboundPlayPacket::PlayerRotation(PlayerRotationUpdate {
                y_rot,
                relative_y,
                x_rot,
                relative_x,
            }),
        ));
    }

    pub(in crate::world) fn look_at_by_command(
        &mut self,
        connection_id: ConnectionId,
        look_at: CommandLookAt,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some((from_anchor, position, target)) = self.command_look_at_point(&look_at) else {
            return;
        };
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        let Vec3d { x, y, z } = position;
        let from = match from_anchor {
            EntityAnchor::Feet => player.position(),
            EntityAnchor::Eyes => {
                let Ok(position) = player.eye_position() else {
                    return;
                };
                position
            }
        };
        let xd = x - from.x;
        let yd = y - from.y;
        let zd = z - from.z;
        let sd = (xd * xd + zd * zd).sqrt();
        // `180.0F / (float)Math.PI` is the float-rounded pi widened back to a
        // double, not the double constant, so the divisor is spelled that way.
        let degrees_per_radian = 180.0 / f64::from(std::f32::consts::PI);
        player.set_x_rot(wrap_degrees(-(yd.atan2(sd) * degrees_per_radian) as f32));
        player.set_y_rot(wrap_degrees(
            (zd.atan2(xd) * degrees_per_radian) as f32 - 90.0,
        ));
        events.push(send(
            player.connection_id,
            ClientboundPlayPacket::PlayerLookAt(PlayerLookAt {
                from_anchor,
                position: Vec3d { x, y, z },
                target,
            }),
        ));
    }
}
