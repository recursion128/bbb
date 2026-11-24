//! Player documents across the save boundary: restoring saved players, the
//! save patch a session writes, and the command view of that same record.

use bbb_protocol::packets::{BlockPos, PlayerAbilities, PlayerRotation, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::player_schema::ender_pearls::Java26PlayerEnderPearl;
use bbb_server_persistence::{
    Java26CreatedPlayer, Java26PlayerAdvancements, Java26PlayerData, Java26PlayerEnderItems,
    Java26PlayerInventory, Java26PlayerRespawn, Java26PlayerSavePatch, Java26RootVehicle,
    Java26WardenSpawnTracker, Java26WorldCandidate,
};
use bbb_server_types::experience::PlayerExperience as ExperienceState;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::save::{Java26LevelKey, SaveError, SaveResult};
use uuid::Uuid;

use super::player_effects::{attribute_max_health, player_attribute_map};
use super::registry_runtime::PlayerItemStackResolver;
use super::{
    recipe_book_save_flags, PlayerRespawnConfig, PlayerState, RecipeBookTypeSettings,
    RestoredPlayer, ServerWorld,
};
use crate::entity::warden_spawn_tracker::WardenSpawnTracker;
use crate::entity::PackedAttribute;
use crate::inventory::{recipe_book_from_id_lists, PlayerInventoryMenu};

/// A disconnected session's document, written by every save until the
/// profile rejoins and takes it back (`PlayerList.java:302`).
#[derive(Debug, Clone)]
pub(super) struct DepartedPlayerSnapshot {
    pub(super) patch: Java26PlayerSavePatch,
    pub(super) advancements: Java26PlayerAdvancements,
}

/// `WardenSpawnTracker.java:18-25`.
pub(super) fn live_warden_spawn_tracker(saved: &Java26WardenSpawnTracker) -> WardenSpawnTracker {
    WardenSpawnTracker::new(
        saved.ticks_since_last_warning(),
        saved.warning_level(),
        saved.cooldown_ticks(),
    )
}

fn recipe_book_settings_from_save_flags(
    flags: [bool; 8],
) -> [RecipeBookTypeSettings; bbb_protocol::recipe_wire_types::VANILLA_RECIPE_BOOK_TYPES.len()] {
    std::array::from_fn(|index| RecipeBookTypeSettings {
        open: flags[index * 2],
        filtering: flags[index * 2 + 1],
    })
}

fn invalid(detail: String) -> SaveError {
    SaveError::InvalidNbt { path: None, detail }
}

// ItemStackWithSlot.java:9-13; PlayerEnderChestContainer.java:28-38: decode precedes capacity.
fn read_native_player_slot(
    capacity: usize,
    raw: &NbtSummaryValue,
    resolver: &PlayerItemStackResolver,
) -> Option<(usize, RuntimeItemStack)> {
    let NbtSummaryValue::Compound(fields) = raw else {
        return None;
    };
    let slot = fields
        .iter()
        .find(|field| field.name == "Slot")
        .and_then(|field| bbb_protocol::nbt_summary::codec_int_value(&field.value))?
        as i8 as u8 as usize;
    let stack = RuntimeItemStack::try_read(raw, resolver).ok()?;
    (slot < capacity && !stack.is_empty()).then_some((slot, stack))
}

fn read_native_player_slots(
    capacity: usize,
    input: Option<&NbtSummaryValue>,
    resolver: &PlayerItemStackResolver,
) -> SaveResult<Vec<RuntimeItemStack>> {
    let mut slots = vec![RuntimeItemStack::empty(); capacity];
    // TagValueInput.java:79-90; TagValueInput.java:137-139: only an actual ListTag is typed input.
    let Some(NbtSummaryValue::List(items)) = input else {
        return Ok(slots);
    };
    let limit = bbb_registry_codec::PersistentNbtDecodeLimits::DEFAULT.max_collection;
    if items.len() > limit {
        return Err(SaveError::BudgetExceeded {
            resource: "player item input collection",
            actual: items.len() as u64,
            limit: limit as u64,
        });
    }
    for raw in items {
        if let Some((slot, stack)) = read_native_player_slot(capacity, raw, resolver) {
            slots[slot] = stack;
        }
    }
    Ok(slots)
}

fn read_native_player_equipment(
    input: Option<&NbtSummaryValue>,
    resolver: &PlayerItemStackResolver,
    inventory: &mut PlayerInventoryMenu,
) -> SaveResult<()> {
    use bbb_protocol::equipment_slots::vanilla_equipment_slot_for_serialized_name;
    // EntityEquipment.java:11-19; LivingEntity.java:816: retain usable map pairs.
    let Some(NbtSummaryValue::Compound(fields)) = input else {
        return Ok(());
    };
    let limit = bbb_registry_codec::PersistentNbtDecodeLimits::DEFAULT.max_collection;
    if fields.len() > limit {
        return Err(SaveError::BudgetExceeded {
            resource: "player equipment input collection",
            actual: fields.len() as u64,
            limit: limit as u64,
        });
    }
    for entry in fields {
        let Some(slot) = entry
            .name
            .with_utf8(vanilla_equipment_slot_for_serialized_name)
            .flatten()
        else {
            continue;
        };
        let Ok(stack) = RuntimeItemStack::try_read(&entry.value, resolver) else {
            continue;
        };
        // PlayerEquipment.java:16-27: the inherited mainhand entry is not Inventory.selected.
        let Some(index) = slot
            .player_inventory_slot
            .and_then(|index| usize::try_from(index).ok())
        else {
            continue;
        };
        inventory.set_inventory_slot(index, stack);
    }
    Ok(())
}

fn install_ender_chest(
    inventory: &mut PlayerInventoryMenu,
    slots: &[RuntimeItemStack],
) -> SaveResult<()> {
    inventory
        .install_ender_chest_slots(slots)
        .map(|_| ())
        .map_err(|error| invalid(format!("EnderItems: {error}")))
}

pub(super) fn restored_player_from_save_patch(
    patch: &Java26PlayerSavePatch,
) -> SaveResult<RestoredPlayer> {
    let created = &patch.created;
    let mut inventory = PlayerInventoryMenu::empty();
    let slots = created.inventory.runtime_slots();
    let expected = inventory.inventory_slots().len();
    if slots.len() != expected {
        return Err(invalid(format!(
            "live player inventory has {} slots, expected {expected}",
            slots.len(),
        )));
    }
    inventory.install_inventory_slots(&slots);
    inventory.set_selected_slot(created.selected_item_slot);
    inventory.apply_recipe_book(recipe_book_from_id_lists(
        &created.recipe_book_recipes,
        &created.recipe_book_to_be_displayed,
    ));
    // Player.java:641 restores this separate container after the main inventory.
    install_ender_chest(&mut inventory, &created.ender_items.runtime_slots())?;
    Ok(RestoredPlayer {
        portal_cooldown: created.portal_cooldown,
        sleep_timer: created.sleep_timer,
        sleeping_pos: created.sleeping_pos.map(|[x, y, z]| BlockPos { x, y, z }),
        last_hurt_by: created.last_hurt_by,
        spawn_extra_particles_on_fall: created.spawn_extra_particles_on_fall,
        seen_credits: created.seen_credits,
        has_visual_fire: created.has_visual_fire,
        silent: created.silent,
        glowing: created.glowing,
        custom_name: patch.custom_name.clone(),
        custom_data: patch.custom_data.clone(),
        shoulders: super::player_shoulders::PlayerShoulders::from_saved(
            patch.shoulder_entities.clone(),
        ),
        fall_impulse: crate::entity::living::FallImpulseContext {
            reset_grace_time: created.impulse_reset_grace_time,
            impact_position: created.impulse_impact_position,
        },
        fall_distance: created.fall_distance,
        remaining_fire_ticks: created.remaining_fire_ticks,
        air_supply: created.air_supply,
        ticks_frozen: created.ticks_frozen,
        dimension: Some(Java26LevelKey::parse(&created.dimension)?),
        position: Vec3d {
            x: patch.position[0],
            y: patch.position[1],
            z: patch.position[2],
        },
        rotation: PlayerRotation {
            y_rot: patch.rotation[0],
            x_rot: patch.rotation[1],
        },
        on_ground: patch.on_ground,
        game_type: Some(created.game_mode),
        previous_game_type: patch.previous_game_mode,
        entered_nether_position: patch.entered_nether_pos.map(|[x, y, z]| Vec3d { x, y, z }),
        raid_omen_position: patch
            .raid_omen_position
            .map(|[x, y, z]| BlockPos { x, y, z }),
        damage: created.damage,
        // AttributeMap.java:142-149.
        attributes: PackedAttribute::from_saved_list(&created.attributes),
        locator_bar_icon: created.locator_bar_icon.clone(),
        // LivingEntity.java:783-789.
        effects: crate::entity::living::mob_effects::ActiveMobEffects::from_saved_list(
            &created.active_effects,
        ),
        entity_invulnerable: created.entity_invulnerable,
        food: created.food,
        experience: created.experience,
        tags: patch.tags.clone(),
        abilities: created.abilities,
        inventory,
        fall_flying: created.fall_flying,
        recipe_book_settings: recipe_book_settings_from_save_flags(created.recipe_book_settings),
        root_vehicle: patch.root_vehicle.clone(),
        ender_pearls: patch.ender_pearls.clone(),
        respawn: patch.respawn.as_ref().and_then(player_respawn_from_saved),
        last_death_location: patch.last_death_location.clone(),
    })
}

fn player_respawn_from_saved(saved: &Java26PlayerRespawn) -> Option<PlayerRespawnConfig> {
    let dimension = Java26LevelKey::parse(&saved.global_pos().dimension).ok()?;
    Some(PlayerRespawnConfig {
        dimension,
        pos: saved.global_pos().pos,
        yaw: saved.yaw(),
        pitch: saved.pitch(),
        forced: saved.forced(),
    })
}

pub(in crate::world) fn player_save_patch_with_sources(
    player: &PlayerState,
    dimension: &Java26LevelKey,
    resolver: &PlayerItemStackResolver,
    root_vehicle: SaveResult<Option<Java26RootVehicle>>,
    ender_pearls: SaveResult<Vec<Java26PlayerEnderPearl>>,
) -> SaveResult<Java26PlayerSavePatch> {
    use bbb_protocol::nbt_summary::{canonical_double_tag_bits, canonical_float_tag_bits};
    // FloatTag.java:46-48; DoubleTag.java:46-48: output interns signed zero, leaving live fields untouched.
    let output_float = |value| f32::from_bits(canonical_float_tag_bits(value));
    let output_double = |value| f64::from_bits(canonical_double_tag_bits(value));
    let root_vehicle = root_vehicle?;
    let book = player.inventory.recipe_book();
    let recipe_book_recipes = book.known().map(str::to_owned).collect();
    let recipe_book_to_be_displayed = book.to_be_displayed().map(str::to_owned).collect();
    let tracker = player.cold.warden_tracker;
    // ServerPlayer.java:408 writes the session's own tracker.
    let warden_spawn_tracker = Java26WardenSpawnTracker::new(
        tracker.ticks_since_last_warning(),
        tracker.warning_level(),
        tracker.cooldown_ticks(),
    );
    Ok(Java26PlayerSavePatch {
        uuid: player.profile.uuid,
        stats: player.cold.stats.clone(),
        position: [
            player.position().x,
            player.position().y,
            player.position().z,
        ]
        .map(output_double),
        motion: [
            player.delta_movement().x,
            player.delta_movement().y,
            player.delta_movement().z,
        ]
        .map(output_double),
        rotation: [player.rotation().y_rot, player.rotation().x_rot].map(output_float),
        on_ground: player.on_ground(),
        custom_name: player.cold.custom_name.clone(),
        custom_data: player.cold.custom_data.clone(),
        shoulder_entities: player.cold.shoulders.entities.clone(),
        tags: player.cold.tags.clone(),
        root_vehicle,
        ender_pearls: ender_pearls?,
        respawn: player.cold.respawn.as_ref().map(|respawn| {
            Java26PlayerRespawn::new(
                respawn.dimension.resource_id().to_string(),
                [respawn.pos.x, respawn.pos.y, respawn.pos.z],
                output_float(respawn.yaw),
                output_float(respawn.pitch),
                respawn.forced,
            )
        }),
        last_death_location: player.cold.last_death_location.clone(),
        previous_game_mode: player.cold.previous_game_type,
        entered_nether_pos: player
            .cold
            .entered_nether_position
            .map(|pos| [pos.x, pos.y, pos.z].map(output_double)),
        raid_omen_position: player
            .cold
            .raid_omen_position
            .map(|pos| [pos.x, pos.y, pos.z]),
        warden_spawn_tracker: Some(warden_spawn_tracker),
        created: Java26CreatedPlayer {
            portal_cooldown: player.cold.portal_cooldown,
            sleep_timer: player.cold.sleep_timer,
            sleeping_pos: player.cold.sleeping_pos.map(|pos| [pos.x, pos.y, pos.z]),
            last_hurt_by: bbb_server_persistence::Java26PlayerLastHurtBy {
                hurt_by_timestamp: player.cold.last_hurt_by_mob_timestamp,
                player: player
                    .cold
                    .last_hurt_by_player
                    .map(|uuid| (uuid, player.cold.last_hurt_by_player_memory_time)),
                mob: player.cold.last_hurt_by_mob.map(|uuid| {
                    (
                        uuid,
                        player
                            .tick_count
                            .wrapping_sub(player.cold.last_hurt_by_mob_timestamp),
                    )
                }),
            },
            spawn_extra_particles_on_fall: player.cold.spawn_extra_particles_on_fall,
            seen_credits: player.cold.seen_credits,
            has_visual_fire: player.has_visual_fire,
            silent: player.silent,
            glowing: player.cold.glowing_tag,
            fall_distance: output_double(player.block_contact.fall_distance),
            impulse_reset_grace_time: player.cold.fall_impulse.reset_grace_time,
            impulse_impact_position: player.cold.fall_impulse.impact_position.map(|position| {
                Vec3d {
                    x: output_double(position.x),
                    y: output_double(position.y),
                    z: output_double(position.z),
                }
            }),
            remaining_fire_ticks: player.remaining_fire_ticks,
            air_supply: player.air_supply,
            ticks_frozen: player.ticks_frozen,
            dimension: dimension.resource_id().to_string(),
            game_mode: player.game_type,
            attributes: player
                .attributes
                .pack()
                .iter()
                .map(PackedAttribute::to_saved_nbt)
                .collect(),
            active_effects: player.effects.to_saved_list(),
            locator_bar_icon: player.cold.locator_bar_icon.clone(),
            entity_invulnerable: player.entity_invulnerable,
            abilities: PlayerAbilities {
                flying_speed: output_float(player.abilities.flying_speed),
                walking_speed: output_float(player.abilities.walking_speed),
                ..player.abilities
            },
            inventory: Java26PlayerInventory::prepare(
                player.inventory.inventory_items(),
                resolver,
            )?,
            ender_items: Java26PlayerEnderItems::prepare(
                &player.inventory.ender_chest_slots(),
                resolver,
            )?,
            fall_flying: player.shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING,
            ),
            selected_item_slot: player.inventory.selected_slot(),
            recipe_book_settings: recipe_book_save_flags(&player.cold.recipe_book_settings),
            recipe_book_recipes,
            recipe_book_to_be_displayed,
            damage: bbb_server_datapack::damage::PlayerDamageState {
                health: output_float(player.damage.health),
                absorption_amount: output_float(player.damage.absorption_amount),
                ..player.damage
            },
            food: bbb_server_types::food::FoodData::from_saved(
                player.food.food_level(),
                player.food.tick_timer(),
                output_float(player.food.saturation_level()),
                output_float(player.food.exhaustion_level()),
            ),
            experience: ExperienceState::from_saved(
                output_float(player.experience.progress()),
                player.experience.level(),
                player.experience.total(),
            )
            .with_enchantment_seed(player.experience.enchantment_seed())
            .with_score(player.experience.score()),
        },
    })
}

pub(in crate::world) fn player_save_without_id_with_sources(
    player: &PlayerState,
    dimension: &Java26LevelKey,
    item_stack_resolver: &PlayerItemStackResolver,
    root_vehicle: SaveResult<Option<Java26RootVehicle>>,
    ender_pearls: SaveResult<Vec<Java26PlayerEnderPearl>>,
) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
    let fail = |error| {
        bbb_server_types::ServerWorldError::InvalidConfiguration(format!(
            "player command data: {error}"
        ))
    };
    let patch = player_save_patch_with_sources(
        player,
        dimension,
        item_stack_resolver,
        root_vehicle,
        ender_pearls,
    )
    .map_err(fail)?;
    Ok(
        bbb_server_persistence::created_player_document(&patch, item_stack_resolver)
            .map_err(fail)?
            .entries()
            .to_vec(),
    )
}

pub(in crate::world) fn command_player_data_with_sources(
    player: &PlayerState,
    dimension: &Java26LevelKey,
    item_stack_resolver: &PlayerItemStackResolver,
    root_vehicle: SaveResult<Option<Java26RootVehicle>>,
    ender_pearls: SaveResult<Vec<Java26PlayerEnderPearl>>,
) -> bbb_server_types::Result<NbtSummaryValue> {
    let mut entries = player_save_without_id_with_sources(
        player,
        dimension,
        item_stack_resolver,
        root_vehicle,
        ender_pearls,
    )?;
    // Empty hands have no SelectedItem compound.
    let selected = player.inventory.selected_stack();
    if !selected.is_empty() {
        if let Some(value) = selected.write(item_stack_resolver).ok().flatten() {
            entries.push(NbtSummaryEntry {
                name: "SelectedItem".into(),
                value,
            });
        }
    }
    Ok(NbtSummaryValue::Compound(entries))
}

impl ServerWorld {
    /// `ServerPlayer.readAdditionalSaveData` for any pre-populated candidate
    /// player documents, while indexing disk-stored offline playerdata, stats
    /// and advancements for lazy loading on demand.
    pub(super) fn restore_saved_players(
        &mut self,
        candidate: &Java26WorldCandidate,
    ) -> SaveResult<()> {
        self.cold.restored_player_stats = candidate.player_stats.clone();
        self.cold.restored_player_advancements = candidate.player_advancements.clone();
        self.cold.stored_players = candidate.stored_players.clone();
        self.cold.stored_player_stats = candidate.stored_player_stats.clone();
        self.cold.stored_player_advancements = candidate.stored_player_advancements.clone();
        for (uuid, (_source, player)) in &candidate.players {
            self.cold.stored_players.remove(uuid);
            self.restore_single_saved_player(*uuid, player)?;
        }
        // Only inspect offline playerdata when an already-loaded entity chunk
        // actually contains an ender pearl belonging to an offline player.
        for dimension in self.level_ids() {
            self.ensure_offline_pearl_owners_loaded(&dimension);
        }
        Ok(())
    }

    pub(super) fn restore_single_saved_player(
        &mut self,
        uuid: Uuid,
        player: &Java26PlayerData,
    ) -> SaveResult<()> {
        let resolver = self.item_stack_resolver();
        let [x, y, z] = player.position();
        let [y_rot, x_rot] = player.rotation();
        let mut inventory = PlayerInventoryMenu::empty();
        let storage = read_native_player_slots(
            crate::inventory::INVENTORY_SIZE,
            player.inventory_input(),
            &resolver,
        )?;
        inventory.install_inventory_slots(&storage);
        read_native_player_equipment(player.equipment_input(), &resolver, &mut inventory)?;
        let ender_slots = read_native_player_slots(
            crate::inventory::ENDER_CHEST_SIZE,
            player.ender_items_input(),
            &resolver,
        )?;
        install_ender_chest(&mut inventory, &ender_slots)?;
        inventory.set_selected_slot(player.selected_item_slot());
        // LivingEntity.java:779-791: attributes apply before Health is
        // clamped to the max health they define.
        let (packed_attributes, effects) = crate::entity::save::read_player_living_codec_fields(
            player.attributes_input(),
            player.active_effects_input(),
        );
        let attributes = player_attribute_map(&packed_attributes);
        // ServerPlayer.java:390-391; ServerRecipeBook.java:131-137.
        inventory.apply_recipe_book(recipe_book_from_id_lists(
            player.recipe_book().recipes(),
            player.recipe_book().to_be_displayed(),
        ));
        // ServerPlayer.java:387.
        self.cold.restored_warden_trackers.insert(
            uuid,
            live_warden_spawn_tracker(player.warden_spawn_tracker()),
        );
        let restored = RestoredPlayer {
            portal_cooldown: player.portal_cooldown(),
            sleep_timer: player.sleep_timer(),
            sleeping_pos: player.sleeping_pos().map(|[x, y, z]| BlockPos { x, y, z }),
            last_hurt_by: player.last_hurt_by(),
            spawn_extra_particles_on_fall: player.spawn_extra_particles_on_fall(),
            seen_credits: player.seen_credits(),
            has_visual_fire: player.has_visual_fire(),
            silent: player.silent(),
            glowing: player.glowing(),
            custom_name: player.custom_name().cloned(),
            custom_data: player.custom_data().to_vec(),
            shoulders: super::player_shoulders::PlayerShoulders::from_saved(
                player.shoulder_entities().map(<[_]>::to_vec),
            ),
            fall_impulse: crate::entity::living::FallImpulseContext {
                reset_grace_time: player.impulse_reset_grace_time(),
                impact_position: player.impulse_impact_position(),
            },
            fall_distance: player.fall_distance(),
            remaining_fire_ticks: player.remaining_fire_ticks(),
            air_supply: player.air_supply(),
            ticks_frozen: player.ticks_frozen(),
            dimension: Java26LevelKey::parse(player.dimension())
                .ok()
                .filter(|dimension| self.has_level_key(dimension)),
            position: Vec3d { x, y, z },
            rotation: PlayerRotation { y_rot, x_rot },
            on_ground: player.on_ground(),
            // ServerPlayer.java:400,2025-2027: the saved mode applies unless
            // the server forces one.
            game_type: player.game_mode(),
            previous_game_type: player.previous_game_mode(),
            entered_nether_position: player
                .entered_nether_pos()
                .map(|[x, y, z]| Vec3d { x, y, z }),
            raid_omen_position: player
                .raid_omen_position()
                .map(|[x, y, z]| BlockPos { x, y, z }),
            damage: player.damage(attribute_max_health(&attributes)),
            attributes: packed_attributes,
            locator_bar_icon: player.locator_bar_icon().clone(),
            effects,
            entity_invulnerable: player.invulnerable(),
            food: player.food_data(),
            experience: {
                let saved = player.experience();
                // `Player.java:632-635`.
                let seed = match saved.seed() {
                    0 => self.random.next_u64() as i32,
                    seed => seed,
                };
                ExperienceState::from_saved(saved.progress(), saved.level(), saved.total())
                    .with_enchantment_seed(seed)
                    .with_score(saved.score())
            },
            tags: player.tags().to_vec(),
            abilities: PlayerAbilities {
                invulnerable: player.abilities().invulnerable(),
                flying: player.abilities().flying(),
                can_fly: player.abilities().may_fly(),
                instabuild: player.abilities().instant_build(),
                flying_speed: player.abilities().fly_speed(),
                walking_speed: player.abilities().walk_speed(),
            },
            inventory,
            fall_flying: player.fall_flying(),
            recipe_book_settings: recipe_book_settings_from_save_flags(
                player.recipe_book().settings(),
            ),
            root_vehicle: player.root_vehicle().cloned(),
            ender_pearls: player.ender_pearls().to_vec(),
            respawn: player.respawn().and_then(player_respawn_from_saved),
            last_death_location: player.last_death_location().cloned(),
        };
        self.unload_restored_ender_pearls(
            restored
                .ender_pearls
                .iter()
                .map(|pearl| pearl.entity.as_ref().uuid()),
        );
        self.cold.restored_players.insert(uuid, restored);
        Ok(())
    }

    pub(super) fn ensure_restored_player_loaded(&mut self, uuid: Uuid) -> SaveResult<()> {
        if self.cold.restored_players.contains_key(&uuid)
            || !self.cold.stored_players.contains(&uuid)
        {
            return Ok(());
        }
        let Some(reader) = self.cold.stored_data_reader.clone() else {
            return Ok(());
        };
        let loaded = reader.read_player(uuid)?;
        if let Some((_source, player)) = loaded {
            self.restore_single_saved_player(uuid, &player)?;
        }
        self.cold.stored_players.remove(&uuid);
        Ok(())
    }

    pub(super) fn ensure_restored_player_stats_loaded(&mut self, uuid: Uuid) -> SaveResult<()> {
        if self.cold.restored_player_stats.contains_key(&uuid)
            || !self.cold.stored_player_stats.contains(&uuid)
        {
            return Ok(());
        }
        let Some(reader) = self.cold.stored_data_reader.clone() else {
            return Ok(());
        };
        let loaded = reader.read_player_stats(uuid)?;
        self.cold.stored_player_stats.remove(&uuid);
        if let Some(stats) = loaded {
            self.cold.restored_player_stats.insert(uuid, stats);
        }
        Ok(())
    }

    pub(super) fn ensure_restored_player_advancements_loaded(
        &mut self,
        uuid: Uuid,
    ) -> SaveResult<()> {
        if self.cold.restored_player_advancements.contains_key(&uuid)
            || !self.cold.stored_player_advancements.contains(&uuid)
        {
            return Ok(());
        }
        let Some(reader) = self.cold.stored_data_reader.clone() else {
            return Ok(());
        };
        let loaded = reader.read_player_advancements(uuid)?;
        self.cold.stored_player_advancements.remove(&uuid);
        if let Some(advancements) = loaded {
            self.cold
                .restored_player_advancements
                .insert(uuid, advancements);
        }
        Ok(())
    }

    /// Lazily loads offline playerdata only for owners of ender pearls present
    /// in `dimension`, so region duplicates of offline pearls stay unloaded
    /// until their owner joins without scanning unrelated offline players.
    pub(super) fn ensure_offline_pearl_owners_loaded(&mut self, dimension: &super::LevelId) {
        if self.cold.stored_players.is_empty() {
            return;
        }
        let mut owners = Vec::new();
        for entity in self.level_by_id(dimension).entities.bodies() {
            if let crate::entity::EntityData::EnderPearl(data) = entity.data() {
                if let Some(owner) = data.projectile().owner() {
                    if self.cold.stored_players.contains(&owner)
                        && self.connection_of(owner).is_none()
                        && !owners.contains(&owner)
                    {
                        owners.push(owner);
                    }
                }
            }
        }
        for owner in owners {
            if let Err(error) = self.ensure_restored_player_loaded(owner) {
                tracing::warn!(
                    event = "world.offline_pearl_owner_load_failed",
                    uuid = %owner,
                    %error,
                );
            }
        }
    }

    /// PlayerList.java:299-302; PrepareSpawnTask.java:177-183: a rejoin reads
    /// the departed document's items anew.
    pub(super) fn restore_player_items_for_rejoin(
        &self,
        patch: &Java26PlayerSavePatch,
        restored: &mut RestoredPlayer,
    ) -> SaveResult<()> {
        let resolver = self.item_stack_resolver();
        if let Some((storage, equipment)) = patch.created.inventory.prepared_fields() {
            let storage = NbtSummaryValue::List(storage.to_vec());
            let equipment = NbtSummaryValue::Compound(equipment.to_vec());
            let slots = read_native_player_slots(
                crate::inventory::INVENTORY_SIZE,
                Some(&storage),
                &resolver,
            )?;
            restored.inventory.install_inventory_slots(&slots);
            // EntityEquipment.java:57-60 replaces the saved equipment, including omissions.
            for index in slots.len()..restored.inventory.inventory_slots().len() {
                restored
                    .inventory
                    .set_inventory_slot(index, RuntimeItemStack::empty());
            }
            read_native_player_equipment(Some(&equipment), &resolver, &mut restored.inventory)?;
        }
        let Some(output) = patch.created.ender_items.prepared_slots() else {
            return Ok(());
        };
        let mut slots = vec![RuntimeItemStack::empty(); crate::inventory::ENDER_CHEST_SIZE];
        for (_slot, raw) in output {
            if let Some((slot, stack)) =
                read_native_player_slot(crate::inventory::ENDER_CHEST_SIZE, raw, &resolver)
            {
                slots[slot] = stack;
            }
        }
        install_ender_chest(&mut restored.inventory, &slots)
    }

    pub(super) fn player_save_patch(
        &self,
        player: &PlayerState,
    ) -> SaveResult<Java26PlayerSavePatch> {
        let root_vehicle = crate::entity::save::write_external_player_root_vehicle_value(
            &self.level_by_id(&player.dimension).entities,
            player.entity_id(),
        )
        .map_err(|error| {
            invalid(format!(
                "player {} RootVehicle: {error}",
                player.profile.uuid
            ))
        });
        player_save_patch_with_sources(
            player,
            self.level_key(player.dimension),
            &self.item_stack_resolver(),
            root_vehicle,
            self.player_ender_pearls(player.profile.uuid),
        )
    }
}

#[cfg(test)]
mod tests;
