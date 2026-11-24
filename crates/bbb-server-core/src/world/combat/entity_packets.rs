//! Serverbound entity attack, entity interact, and block pick-item handlers.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::packets::{
    AttackEntity, InteractEntity, InteractionHand, PickItemFromBlock, SetHeldSlot,
};
use bbb_server_types::interaction::{
    is_within_block_interaction_range, is_within_entity_interaction_range, AttackRange,
};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::{MeleeTarget, PlayerHurtSource, SourceEntity};
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::projectile_action::{StoredHurtRequest, StoredHurtResult};
use crate::entity::{
    ArmorStandInteraction, EntityRegistryTagView, InteractingPlayer, StoredDamageCausingEntity,
    StoredEntityDamageContext, StoredEntityDamageRules, StoredEntityDamageSource,
};
use crate::explosion::{ExplosionSourceEntity, ExplosionSourceEntityKind};
use crate::inventory::{attack_range, has_piercing_weapon, PlayerInventoryMenu};
use crate::level_event::VanillaGameEvent;
use crate::world::{
    block_position, send, LevelId, ServerWorld, WorldCloseReason, WorldEvent,
    ATTACK_CHARGE_TOLERANCE, BLOCK_INTERACTION_BUFFER, ENTITY_INTERACTION_BUFFER,
};

impl ServerWorld {
    /// `ServerGamePacketListenerImpl.handleAttack` (:1784-1810).
    ///
    /// `resetLastActionTime` is the only line without a counterpart here: it
    /// feeds the idle-kick clock, which this server does not run.
    ///
    /// `Entity.isRemoved` cannot fail for either live index and
    /// `ItemStack.isItemEnabled` is true because this server enables no feature
    /// flags beyond the defaults. Items, experience orbs, self-attacks, and an
    /// `AbstractArrow` absent from the captured redirectable-projectile tag
    /// disconnect; the three fixed `isAttackable == false` stored types stop
    /// inside `Player.cannotAttack`.
    pub(in crate::world) fn apply_attack(
        &mut self,
        connection_id: ConnectionId,
        packet: AttackEntity,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        self.attack_outcome(connection_id, packet, events)
            // A missing generated fact is this server's fault, not the
            // client's, so the swing is declined rather than the connection
            // closed.
            .unwrap_or_default()
    }

    pub(in crate::world) fn attack_outcome(
        &mut self,
        connection_id: ConnectionId,
        packet: AttackEntity,
        events: &mut Vec<WorldEvent>,
    ) -> Result<Option<WorldCloseReason>> {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        if !player.client_loaded() {
            return Ok(None);
        }
        let registry_tags = self
            .level_by_id(&player.dimension)
            .chunks
            .registry_tags_shared()
            .map(EntityRegistryTagView::Frozen)
            .unwrap_or(EntityRegistryTagView::Builtin26_1);
        #[derive(Clone, Copy)]
        enum Target {
            Player(ConnectionId),
            Stored(i32),
            Hanging(i32),
            /// `Interaction.skipAttackInteraction` (`Interaction.java:100-111`)
            /// runs inside `Player.cannotAttack` (`Player.java:1015-1017`).
            Interaction(i32),
            InvalidStored,
            UnattackableStored,
        }
        // `ServerLevel.getEntityOrPart`: a dragon part is hit through its dragon.
        let dragon_part = crate::entity::ender_dragon::part_by_id(
            &self.level_by_id(&player.dimension).entities,
            packet.entity_id,
        );
        let (target, target_block, target_bounds) = if let Some((dragon, _, bounds)) = dragon_part {
            let center = bounds.center();
            (Target::Stored(dragon), block_position(center), bounds)
        } else if let Some(target_id) = self
            .connection_by_entity_id(packet.entity_id)
            .filter(|&target_id| self.levels.player_at(target_id).dimension == player.dimension)
        {
            let target = self.levels.player_at(target_id);
            (
                Target::Player(target_id),
                block_position(target.position()),
                target.bounding_box()?,
            )
        } else {
            let Some(entity) = self
                .level_by_id(&player.dimension)
                .entities
                .get(packet.entity_id)
            else {
                return Ok(None);
            };
            let target = match entity.data() {
                crate::entity::EntityData::Player | crate::entity::EntityData::EnderDragonPart => {
                    Target::InvalidStored
                }
                crate::entity::EntityData::Item(_)
                | crate::entity::EntityData::ExperienceOrb(_) => Target::InvalidStored,
                crate::entity::EntityData::Interaction(_) => {
                    Target::Interaction(packet.entity_id)
                }
                crate::entity::EntityData::FallingBlock(_)
                | crate::entity::EntityData::EyeOfEnder(_)
                | crate::entity::EntityData::FireworkRocket(_) => Target::UnattackableStored,
                crate::entity::EntityData::Arrow(_)
                | crate::entity::EntityData::SpectralArrow(_)
                | crate::entity::EntityData::Trident(_) => {
                    if crate::entity::hurting_projectile::can_deflect_by_player_attack_with_registry_tag_view(
                        entity,
                        &registry_tags,
                    ) {
                        Target::Stored(packet.entity_id)
                    } else {
                        Target::InvalidStored
                    }
                }
                crate::entity::EntityData::ArmorStand(_)
                | crate::entity::EntityData::EndCrystal(_)
                | crate::entity::EntityData::Marker(_)
                | crate::entity::EntityData::AreaEffectCloud(_)
                | crate::entity::EntityData::OminousItemSpawner(_)
                | crate::entity::EntityData::BlockDisplay(_)
                | crate::entity::EntityData::ItemDisplay(_)
                | crate::entity::EntityData::TextDisplay(_)
                | crate::entity::EntityData::Tnt(_)
                | crate::entity::EntityData::GeneralLiving(_)
                | crate::entity::EntityData::FishingBobber(_)
                | crate::entity::EntityData::Snowball(_)
                | crate::entity::EntityData::Egg(_)
                | crate::entity::EntityData::EnderPearl(_)
                | crate::entity::EntityData::ExperienceBottle(_)
                | crate::entity::EntityData::SplashPotion(_)
                | crate::entity::EntityData::LingeringPotion(_)
                | crate::entity::EntityData::LlamaSpit(_)
                | crate::entity::EntityData::EvokerFangs(_)
                | crate::entity::EntityData::LightningBolt(_)
                | crate::entity::EntityData::ShulkerBullet(_)
                | crate::entity::EntityData::LargeFireball(_)
                | crate::entity::EntityData::WindCharge(_)
                | crate::entity::EntityData::BreezeWindCharge(_)
                | crate::entity::EntityData::SmallFireball(_)
                | crate::entity::EntityData::DragonFireball(_)
                | crate::entity::EntityData::WitherSkull(_) => Target::Stored(packet.entity_id),
                crate::entity::EntityData::Painting(_)
                | crate::entity::EntityData::ItemFrame(_)
                | crate::entity::EntityData::GlowItemFrame(_)
                | crate::entity::EntityData::LeashKnot(_) => Target::Hanging(packet.entity_id),
                data @ (crate::entity::EntityData::Boat(_)
                | crate::entity::EntityData::ChestBoat(_)) => {
                    debug_assert!(player_attack_reaches_vehicle(data));
                    Target::Stored(packet.entity_id)
                }
                data @ (crate::entity::EntityData::Minecart(_)
                | crate::entity::EntityData::FurnaceMinecart(_)
                | crate::entity::EntityData::ChestMinecart(_)
                | crate::entity::EntityData::HopperMinecart(_)
                | crate::entity::EntityData::CommandBlockMinecart(_)
                | crate::entity::EntityData::SpawnerMinecart(_)
                | crate::entity::EntityData::TntMinecart(_)) => {
                    debug_assert!(player_attack_reaches_vehicle(data));
                    Target::Stored(packet.entity_id)
                }
            };
            (target, entity.block_position(), entity.bounding_box())
        };
        if !self
            .level_by_id(&player.dimension)
            .world_border
            .contains_block_at(target_block, self.game_time())
        {
            return Ok(None);
        }
        let main_hand = player.inventory.selected_item()?.clone();
        let eye = player.eye_position()?;
        let range = match attack_range(&main_hand)? {
            Some(declared) => declared,
            None => AttackRange::default_for_player(player.entity_interaction_range()),
        };
        if !range.is_in_range(
            eye,
            player.abilities.instabuild,
            &target_bounds,
            ENTITY_INTERACTION_BUFFER,
        ) {
            return Ok(None);
        }
        if has_piercing_weapon(&main_hand)? {
            return Ok(None);
        }
        match target {
            Target::InvalidStored => {
                return Ok(Some(WorldCloseReason::InvalidEntityAttacked));
            }
            Target::Player(id) if id == connection_id => {
                return Ok(Some(WorldCloseReason::InvalidEntityAttacked));
            }
            Target::Player(_)
            | Target::Stored(_)
            | Target::Hanging(_)
            | Target::Interaction(_)
            | Target::UnattackableStored => {}
        }
        if player.cannot_attack_with_item(&main_hand, ATTACK_CHARGE_TOLERANCE)? {
            return Ok(None);
        }
        if matches!(target, Target::UnattackableStored) {
            return Ok(None);
        }
        match target {
            Target::Player(target_id) => self.player_melee_attack(
                connection_id,
                MeleeTarget::Session(target_id),
                &registry_tags,
                events,
            )?,
            // `ArmorStand.skipAttackInteraction` (ArmorStand.java:577-579).
            Target::Stored(entity_id)
                if self
                    .level_by_id(&player.dimension)
                    .entities
                    .get(entity_id)
                    .is_some_and(|entity| {
                        matches!(entity.data(), crate::entity::EntityData::ArmorStand(_))
                            && !self
                                .player_may_interact_block(connection_id, entity.block_position())
                    }) => {}
            Target::Stored(entity_id) => {
                let dimension = player.dimension;
                let set_part = |world: &mut Self, part| {
                    if let Some(dragon) = world
                        .level_mut_by_id(&dimension)
                        .entities
                        .get_mut(entity_id)
                    {
                        crate::entity::ender_dragon::set_hit_part(dragon, part);
                    }
                };
                if let Some((_, part, _)) = dragon_part {
                    set_part(self, Some(part));
                }
                let attacked = self.player_melee_attack(
                    connection_id,
                    MeleeTarget::Stored(entity_id),
                    &registry_tags,
                    events,
                );
                if dragon_part.is_some() {
                    set_part(self, None);
                }
                attacked?
            }
            Target::Hanging(entity_id) => {
                if !self.player_attack_hanging_skip(connection_id, entity_id, &registry_tags)? {
                    self.player_melee_attack(
                        connection_id,
                        MeleeTarget::Stored(entity_id),
                        &registry_tags,
                        events,
                    )?;
                }
            }
            Target::Interaction(entity_id) => {
                self.player_attack_interaction(connection_id, entity_id, &registry_tags, events)?
            }
            Target::InvalidStored | Target::UnattackableStored => {
                unreachable!("stored refusals returned above")
            }
        }
        Ok(None)
    }

    pub(super) fn player_attack_hanging_skip(
        &mut self,
        connection_id: ConnectionId,
        entity_id: i32,
        registry_tags: &EntityRegistryTagView,
    ) -> Result<bool> {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let player_entity_id = player.entity_id();
        let player_uuid = player.profile.uuid;
        let instabuild = player.abilities.instabuild;
        let may_build = player.may_build();
        let spectator = player.game_type == bbb_protocol::game_types::GameType::Spectator;
        let player_position = player.position();
        let dimension = player.dimension;
        // `BlockAttachedEntity.skipAttackInteraction` (BlockAttachedEntity.java:60-62).
        if self
            .level_by_id(&dimension)
            .entities
            .get(entity_id)
            .is_some_and(|entity| {
                !self.player_may_interact_block(connection_id, entity.block_position())
            })
        {
            return Ok(true);
        }
        let Some(damage_type) = self
            .cold
            .damage_types
            .iter()
            .find(|entry| entry.resource_id == "minecraft:player_attack")
            .cloned()
        else {
            return Ok(true);
        };
        let request = StoredHurtRequest {
            direct_entity_snapshot: None,
            target_entity_id: entity_id,
            damage_type_resource_id: "minecraft:player_attack",
            amount: 0.0,
            direct_resource_id: Some("minecraft:player"),
            registry_tags,
            causing_entity_snapshot: Some(ExplosionSourceEntity {
                custom_name: None,
                entity_id: player_entity_id,
                entity_resource_id: "minecraft:player".to_string(),
                uuid: player_uuid,
                position: player_position,
                kind: ExplosionSourceEntityKind::Player {
                    instabuild,
                    may_build,
                    spectator,
                },
            }),
            source_position: Some(player_position),
            explicit_source_position: None,
        };
        let source = StoredEntityDamageSource {
            damage_type: &damage_type,
            direct_entity_id: Some(player_entity_id),
            causing_entity: Some(StoredDamageCausingEntity::Player {
                uuid: player_uuid,
                entity_id: player_entity_id,
                instabuild,
            }),
            attacker_views: Default::default(),
        };
        let rules = StoredEntityDamageRules {
            mob_griefing: self.game_rule_bool("minecraft:mob_griefing", true),
            entity_drops: self.game_rule_bool("minecraft:entity_drops", true),
            tnt_explodes: self.game_rule_bool("minecraft:tnt_explodes", true),
            fall_damage_immune: false,
        };
        let context = StoredEntityDamageContext {
            block_drops: self.game_rule_bool("minecraft:block_drops", true),
            attacker_may_build: may_build,
            attacker_spectator: Some(spectator),
        };
        let result = self.with_stored_hurt_level(&dimension, |entities, level| {
            crate::entity::projectile_action::hurt_stored_entity(
                request, None, source, rules, context, entities, level,
            )
        });
        Ok(match result {
            StoredHurtResult::Applied { outcome, .. } => outcome.succeeded,
            StoredHurtResult::DropNoop => true,
        })
    }

    /// `Player.attack` (`Player.java:951-1004`) on an `Interaction`:
    /// `cannotAttack` (`:1015-1017`) runs `Interaction.skipAttackInteraction`
    /// (`Interaction.java:100-111`) first, which records the attack against the
    /// level game time and ends the attack unless `response` is set; the rest
    /// is the spine above, ending in the no-damage sound because
    /// `Interaction.hurtServer` is false (`:113-116`).
    fn player_attack_interaction(
        &mut self,
        connection_id: ConnectionId,
        entity_id: i32,
        registry_tags: &EntityRegistryTagView,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let player_uuid = player.profile.uuid;
        let dimension = player.dimension;
        let game_time = self.game_time();
        let Some(interaction) = self.level_by_id(&dimension).entities.get(entity_id) else {
            return Ok(());
        };
        let crate::entity::EntityData::Interaction(_) = interaction.data() else {
            return Ok(());
        };
        let skipped = {
            let entities = &mut self.level_mut_by_id(&dimension).entities;
            let Some(mut entity) = entities.detach(entity_id) else {
                return Ok(());
            };
            let skipped = crate::entity::interaction::skip_attack_interaction(
                &mut entity,
                player_uuid,
                game_time,
            );
            entities.attach(entity);
            skipped
        };
        // `Interaction.java:103-105`.
        let damage_types = std::sync::Arc::clone(&self.cold.damage_types);
        if let Some(generic) = damage_types
            .iter()
            .find(|facts| facts.resource_id == "minecraft:generic")
        {
            self.trigger_player_hurt_entity(
                connection_id,
                SourceEntity::Stored(entity_id),
                PlayerHurtSource {
                    damage_type: generic,
                    causing: None,
                    direct: None,
                },
                crate::world::advancement_triggers::HurtTriggerDetails {
                    dealt: 1.0,
                    taken: 1.0,
                    blocked: false,
                },
                events,
            );
        }
        if skipped {
            return Ok(());
        }
        self.player_melee_attack(
            connection_id,
            MeleeTarget::Stored(entity_id),
            registry_tags,
            events,
        )
    }

    /// `ServerGamePacketListenerImpl.handlePickItemFromBlock` (:690-708).
    ///
    /// The reach test and the loaded test are the whole gate: neither
    /// `hasClientLoaded` nor an outstanding teleport is consulted, so this one
    /// does not go through `player_can_interact`.
    pub(in crate::world) fn apply_pick_item_from_block(
        &mut self,
        connection_id: ConnectionId,
        packet: PickItemFromBlock,
        events: &mut Vec<WorldEvent>,
    ) {
        // A missing generated fact is this server's fault, not the client's,
        // so the pick is declined rather than the connection closed.
        let _ = self.pick_item_from_block_outcome(connection_id, packet, events);
    }

    pub(in crate::world) fn pick_item_from_block_outcome(
        &mut self,
        connection_id: ConnectionId,
        packet: PickItemFromBlock,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let eye = player.eye_position()?;
        // `:698`: `includeData` needs `Player.hasInfiniteMaterials`.
        let include_data = packet.include_data && player.abilities.instabuild;
        if !is_within_block_interaction_range(
            eye,
            packet.pos,
            player.block_interaction_range(),
            BLOCK_INTERACTION_BUFFER,
        ) {
            return Ok(());
        }
        // `Level.isLoaded`: the authoritative store answers for the loaded
        // column and for nothing else.
        let Some(block_state_id) =
            self.block_state(self.player_level_id(connection_id), packet.pos)
        else {
            return Ok(());
        };
        let dimension = self.player_level_id(connection_id);
        let item = if let Some(block_type) =
            vanilla_block_state_block_type_id(block_state_id).and_then(vanilla_block_type)
        {
            self.clone_item_stack(
                &dimension,
                packet.pos,
                block_state_id,
                block_type,
                include_data,
            )
        } else {
            let content_registry = self.registries.content_registry();
            content_registry
                .block_by_state_id(block_state_id)
                .and_then(|(block, _)| content_registry.item_by_resource_id(block.resource_id()))
                .map(|item| {
                    RuntimeItemStack::from_summary(bbb_protocol::packets::ItemStackSummary {
                        item_id: Some(item.id),
                        count: 1,
                        component_patch: Default::default(),
                    })
                })
        };
        let Some(item) = item else {
            return Ok(());
        };
        self.try_pick_item(connection_id, item, events)
    }

    /// `ServerGamePacketListenerImpl.tryPickItem` (:740-756).
    ///
    /// `ItemStack.isItemEnabled` is true for every item this world can name:
    /// the gate serves no feature packs, so the level's enabled features are
    /// the vanilla set every built-in item requires. The held-slot answer and
    /// the menu broadcast are outside the branch that moved the stack, so a
    /// pick that changed nothing still reports the slot the player is on.
    pub(in crate::world) fn try_pick_item(
        &mut self,
        connection_id: ConnectionId,
        item: RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        let resolver = self.item_stack_resolver();
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        match player.inventory.find_slot_matching_item(&resolver, &item)? {
            Some(slot) if PlayerInventoryMenu::is_hotbar_slot(slot) => player
                .inventory
                .set_selected_slot(u8::try_from(slot).expect("hotbar slots fit a byte")),
            Some(slot) => player.inventory.pick_slot(slot),
            // `Player.hasInfiniteMaterials` (Player.java:1293-1295).
            None if player.abilities.instabuild => player.inventory.add_and_pick_item(item),
            None => {}
        }
        let slot = i32::from(player.inventory.selected_slot());
        events.push(send(
            connection_id,
            ClientboundPlayPacket::SetHeldSlot(SetHeldSlot { slot }),
        ));
        self.broadcast_player_menu(
            connection_id,
            crate::inventory::MenuBroadcastMode::Changes,
            events,
        );
        Ok(())
    }

    /// `ServerGamePacketListenerImpl.handleInteract` (:1813-1839).
    pub(in crate::world) fn apply_interact(
        &mut self,
        connection_id: ConnectionId,
        packet: InteractEntity,
        events: &mut Vec<WorldEvent>,
    ) {
        let Ok(()) = self.interact_outcome(connection_id, packet, events) else {
            return;
        };
    }

    pub(in crate::world) fn interact_outcome(
        &mut self,
        connection_id: ConnectionId,
        packet: InteractEntity,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        if !player.client_loaded() {
            return Ok(());
        }
        // The shift key is written before the target is resolved, so a client
        // that aims at nothing still moves its own crouch state.
        player.set_shared_flag(
            VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            packet.using_secondary_action,
        );
        let dimension = self.levels[connection_id].dimension;
        if let Some(target_id) = self
            .connection_by_entity_id(packet.entity_id)
            .filter(|&target_id| self.levels.player_at(target_id).dimension == dimension)
        {
            let target = self.levels.player_at(target_id);
            let border = self.level_by_id(&dimension).world_border;
            if !border.contains_block_at(block_position(target.position()), self.game_time()) {
                return Ok(());
            }
            let target_bounds = target.bounding_box()?;
            let player = self.levels.player_at(connection_id);
            if !is_within_entity_interaction_range(
                player.eye_position()?,
                &target_bounds,
                player.entity_interaction_range(),
                ENTITY_INTERACTION_BUFFER,
            ) {
                return Ok(());
            }
            return Ok(());
        }
        let Some(entity) = self.level_by_id(&dimension).entities.get(packet.entity_id) else {
            return Ok(());
        };
        let minecart = matches!(
            entity.entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
        );
        let vehicle_container = crate::entity::item_slots::container_size(entity).is_some();
        let interaction = matches!(entity.data(), crate::entity::EntityData::Interaction(_));
        let armor_stand = matches!(entity.data(), crate::entity::EntityData::ArmorStand(_));
        let item_frame = matches!(
            entity.data(),
            crate::entity::EntityData::ItemFrame(_) | crate::entity::EntityData::GlowItemFrame(_)
        );
        // The stored kinds whose `Entity.interact` leash branches have an
        // owner (`Entity.java:2168-2242`); a boat target was already taken by
        // `apply_external_player_boat_interact`.
        let leash_target = matches!(
            entity.data(),
            crate::entity::EntityData::GeneralLiving(_) | crate::entity::EntityData::LeashKnot(_)
        );
        if !interaction
            && !armor_stand
            && !item_frame
            && !leash_target
            && !minecart
            && !vehicle_container
        {
            return Ok(());
        }
        if !self
            .level_by_id(&dimension)
            .world_border
            .contains_block_at(entity.block_position(), self.game_time())
        {
            return Ok(());
        }
        let target_bounds = entity.bounding_box();
        let player = self.levels.player_at(connection_id);
        if !is_within_entity_interaction_range(
            player.eye_position()?,
            &target_bounds,
            player.entity_interaction_range(),
            ENTITY_INTERACTION_BUFFER,
        ) {
            return Ok(());
        }
        if interaction {
            if player.game_type == bbb_protocol::game_types::GameType::Spectator {
                return Ok(());
            }
            let player_uuid = player.profile.uuid;
            let original =
                crate::world::using_item::runtime_item_in_hand(player, packet.hand).clone();
            let game_time = self.game_time();
            let target = self
                .level_mut_by_id(&dimension)
                .entities
                .get_mut(packet.entity_id)
                .expect("interaction target was live");
            crate::entity::interaction::interact(target, player_uuid, game_time);
            self.trigger_player_entity_interaction(
                connection_id,
                "minecraft:player_interacted_with_entity",
                dimension,
                packet.entity_id,
                original.summary(),
                events,
            );
            return Ok(());
        }
        if vehicle_container {
            let original =
                crate::world::using_item::runtime_item_in_hand(player, packet.hand).clone();
            let spectator = player.game_type == bbb_protocol::game_types::GameType::Spectator;
            self.open_vehicle_container(connection_id, packet.entity_id, events);
            if !spectator {
                self.trigger_player_entity_interaction(
                    connection_id,
                    "minecraft:player_interacted_with_entity",
                    dimension,
                    packet.entity_id,
                    original.summary(),
                    events,
                );
            }
            return Ok(());
        }
        if minecart {
            self.apply_minecart_interact(connection_id, packet, events);
            return Ok(());
        }
        if leash_target {
            // `Player.interactOn` returns before invoking the entity for a
            // spectator (`Player.java:824-834`).
            if self.levels[connection_id].game_type != bbb_protocol::game_types::GameType::Spectator
            {
                self.interact_with_stored_leash_target(connection_id, packet, dimension, events);
            }
            return Ok(());
        }
        if armor_stand {
            return self.interact_with_armor_stand(connection_id, packet, dimension, events);
        }
        // `Player.interactOn` returns before invoking the entity (`Player.java:824-834`).
        if self.levels[connection_id].game_type == bbb_protocol::game_types::GameType::Spectator {
            return Ok(());
        }
        let hand = packet.hand;
        let (stack, map_id) = {
            let player = self.levels.player_at(connection_id);
            let stack = crate::world::using_item::runtime_item_in_hand(player, hand);
            (stack.clone(), stack.summary().component_patch.map_id)
        };
        // The stack names the map; `MapItemSavedData` owns its tracked count (`ItemFrame.java:369-371`).
        let map_admission = map_id
            .map_or(crate::entity::FramedMapAdmission::NotFramedMap, |map_id| {
                self.maps.admission(map_id)
            });
        let original = stack.clone();
        let held = Some(stack);
        let Some(entity) = self.level_by_id(&dimension).entities.get(packet.entity_id) else {
            return Ok(());
        };
        let plan = crate::entity::prepare_hanging_interact(entity, held, map_admission);
        let (attached, consume_one) = match &plan {
            crate::entity::HangInteract::Pass | crate::entity::HangInteract::Fail => return Ok(()),
            crate::entity::HangInteract::Add { attached, .. } => (*attached, true),
            crate::entity::HangInteract::Rotate { attached, .. } => (*attached, false),
        };
        let mut entity = self
            .level_mut_by_id(&dimension)
            .entities
            .detach(packet.entity_id)
            .expect("entity was live");
        let applied = {
            let entities = &mut self.level_mut_by_id(&dimension).entities;
            crate::entity::apply_hanging_interact(&mut entity, &plan, |_, sound| {
                entities.play_entity_sound(sound);
            })
        };
        if !applied {
            self.level_mut_by_id(&dimension).entities.attach(entity);
            return Ok(());
        }
        let event_pos = entity.block_position();
        self.level_mut_by_id(&dimension).entities.attach(entity);
        self.item_frame_output_changed_by_key(&dimension, attached);
        let player_entity_id = self.levels[connection_id].entity_id();
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::BLOCK_CHANGE,
                event_pos,
                Some(player_entity_id),
                None,
            );
        if consume_one {
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("connection presence was checked");
            if !player.abilities.instabuild {
                let stack = player.inventory.item_in_hand_mut(hand);
                if stack.count() > 0 {
                    *stack = stack.with_count(stack.count() - 1);
                }
            }
        }
        self.trigger_player_entity_interaction(
            connection_id,
            "minecraft:player_interacted_with_entity",
            dimension,
            packet.entity_id,
            original.summary(),
            events,
        );
        Ok(())
    }

    fn interact_with_armor_stand(
        &mut self,
        connection_id: ConnectionId,
        packet: InteractEntity,
        dimension: LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        let hand = packet.hand;
        let (spectator, infinite_materials, mut hand_stack) = {
            let player = self
                .levels
                .player(connection_id)
                .expect("connection presence was checked");
            (
                player.game_type == bbb_protocol::game_types::GameType::Spectator,
                player.abilities.instabuild,
                crate::world::using_item::runtime_item_in_hand(player, hand).clone(),
            )
        };
        let original = hand_stack.clone();
        let mut entity = self
            .level_mut_by_id(&dimension)
            .entities
            .detach(packet.entity_id)
            .expect("entity was live");
        let result = {
            let entities = &mut self.level_mut_by_id(&dimension).entities;
            crate::entity::armor_stand::interact(
                &mut entity,
                entities,
                InteractingPlayer {
                    hand_stack: &mut hand_stack,
                    spectator,
                    infinite_materials,
                },
                packet.location,
            )
        };
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let index = match hand {
            InteractionHand::MainHand => usize::from(player.inventory.selected_slot()),
            InteractionHand::OffHand => 40,
        };
        player
            .inventory
            .set_inventory_slot(index, hand_stack.clone());
        self.level_mut_by_id(&dimension).entities.attach(entity);
        if !spectator
            && result == ArmorStandInteraction::Pass
            && hand_stack.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_NAME_TAG_ID)
        {
            self.name_living_entity(
                connection_id,
                packet,
                dimension,
                &hand_stack,
                infinite_materials,
                events,
            );
        }

        if !spectator && result == ArmorStandInteraction::SuccessServer {
            self.trigger_player_entity_interaction(
                connection_id,
                "minecraft:player_interacted_with_entity",
                dimension,
                packet.entity_id,
                original.summary(),
                events,
            );
            // `ArmorStand.interact` mutates equipment before the listener
            // swings (`ServerGamePacketListenerImpl.java:1828-1833`).
            self.apply_level_entity_lifecycle_by_key(&dimension, events);
            let player = self
                .levels
                .player(connection_id)
                .expect("connection presence was checked");
            let user = crate::admin::NameAndId {
                id: player.profile.uuid,
                name: (&player.profile.name).into(),
            };
            self.swing_arm_by_command(
                &crate::commands::CommandPlayerTarget {
                    profile: user.clone(),
                },
                hand == InteractionHand::OffHand,
                events,
            );
        }
        Ok(())
    }

    /// `Player.interactOn` falling through to `NameTagItem.interactLivingEntity`
    /// (`Player.java:841-850`) for a target whose `interact` passed.
    fn name_living_entity(
        &mut self,
        connection_id: ConnectionId,
        packet: InteractEntity,
        dimension: LevelId,
        hand_stack: &RuntimeItemStack,
        infinite_materials: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let mut tag = hand_stack.summary().clone();
        let entities = &mut self.level_mut_by_id(&dimension).entities;
        if crate::entity::name_tag::interact_living_entity(entities, packet.entity_id, &mut tag)
            != crate::entity::name_tag::NameTagInteraction::Success
        {
            return;
        }
        let Some(position) = entities
            .get(packet.entity_id)
            .map(|entity| entity.position())
        else {
            return;
        };
        self.player_game_event_at(
            connection_id,
            VanillaGameEvent::ENTITY_INTERACT,
            position,
            None,
        );
        if !infinite_materials {
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("connection presence was checked");
            *player.inventory.item_in_hand_mut(packet.hand) = RuntimeItemStack::from_summary(tag);
        }
        self.trigger_player_entity_interaction(
            connection_id,
            "minecraft:player_interacted_with_entity",
            dimension,
            packet.entity_id,
            hand_stack.summary(),
            events,
        );
    }
}

/// `Entity.isAttackable` and `skipAttackInteraction` (`Entity.java:2904-2910`).
fn player_attack_reaches_vehicle(data: &crate::entity::EntityData) -> bool {
    crate::entity::boat::is_boat(data) || crate::entity::minecart::is_minecart(data)
}

#[cfg(test)]
mod vehicle_attack_tests {
    use super::*;

    #[test]
    fn player_attack_routes_minecart_containers_and_other_vehicles_to_stored_damage() {
        assert!(player_attack_reaches_vehicle(
            &crate::entity::EntityData::Boat(crate::entity::BoatData::new())
        ));
        assert!(player_attack_reaches_vehicle(
            &crate::entity::EntityData::ChestBoat(crate::entity::ChestBoatData::new())
        ));
        assert!(player_attack_reaches_vehicle(
            &crate::entity::EntityData::Minecart(crate::entity::MinecartData::new())
        ));
        assert!(player_attack_reaches_vehicle(
            &crate::entity::EntityData::ChestMinecart(
                crate::entity::MinecartContainerData::new_chest()
            )
        ));
        assert!(player_attack_reaches_vehicle(
            &crate::entity::EntityData::HopperMinecart(
                crate::entity::MinecartContainerData::new_hopper()
            )
        ));
        assert!(!player_attack_reaches_vehicle(
            &crate::entity::EntityData::Marker(crate::entity::MarkerData::new())
        ));
    }
}
