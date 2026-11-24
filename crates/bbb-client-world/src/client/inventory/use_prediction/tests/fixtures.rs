use super::*;

pub(super) fn stack(item_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

pub(super) fn set_main_hand(world: &mut WorldStore, item_id: i32) {
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: stack(item_id),
    });
}

pub(super) fn age_lock_success() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ))
}

pub(super) fn potion_stack(potion_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_POTION_ID),
        count,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
            potion_contents: bbb_protocol::packets::PotionContentsSummary {
                potion_id: Some(potion_id),
                ..Default::default()
            },
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

pub(super) fn direct_instrument_stack(
    use_duration: f32,
    range: f32,
    cooldown_group: Option<&str>,
) -> ItemStackSummary {
    let mut added_type_ids = vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID];
    if cooldown_group.is_some() {
        added_type_ids.push(VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID);
    }
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_GOAT_HORN_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids,
            instrument: bbb_protocol::packets::InstrumentComponentSummary {
                direct: Some(Box::new(InstrumentSummary {
                    sound_event: SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("example:direct_horn".to_string()),
                        fixed_range_bits: Some(24.0_f32.to_bits()),
                    },
                    use_duration_bits: use_duration.to_bits(),
                    range_bits: range.to_bits(),
                    description: "Direct horn".to_string(),
                    description_styled: None,
                    description_nbt: None,
                })),
                ..Default::default()
            },
            use_cooldown_group: cooldown_group.map(str::to_owned),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

pub(super) fn install_convertable_to_mud_tag(world: &mut WorldStore) {
    install_block_tag(
        world,
        "minecraft:convertable_to_mud",
        vec![VANILLA_BLOCK_DIRT_ID],
    );
}

pub(super) fn install_block_tag(world: &mut WorldStore, tag: &str, entries: Vec<i32>) {
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![TagNetworkPayload {
                tag: tag.to_string(),
                entries,
            }],
        }],
    });
}

pub(super) fn install_place_on_water_support_tags(
    world: &mut WorldStore,
    tag: &str,
    fluid_entries: Vec<i32>,
    block_entries: Vec<i32>,
) {
    world.apply_update_tags(UpdateTags {
        registries: vec![
            RegistryTags {
                registry: "minecraft:fluid".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: tag.to_string(),
                    entries: fluid_entries,
                }],
            },
            RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: tag.to_string(),
                    entries: block_entries,
                }],
            },
        ],
    });
}

pub(super) fn set_offhand(world: &mut WorldStore, item: ItemStackSummary) {
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        item,
    });
}

pub(super) fn player_inventory_item(world: &WorldStore, slot: i32) -> &ItemStackSummary {
    &world
        .inventory()
        .player_slots
        .iter()
        .find(|entry| entry.slot == slot)
        .unwrap_or_else(|| panic!("missing player inventory slot {slot}"))
        .item
}

pub(super) fn block_state_id<const N: usize>(name: &str, properties: [(&str, &str); N]) -> i32 {
    crate::vanilla_block_state_id(
        name,
        &properties
            .into_iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect(),
    )
    .unwrap()
}

pub(super) fn tool_use_world(initial_block_state_id: i32) -> WorldStore {
    tool_use_world_with_login(initial_block_state_id, None)
}

pub(super) fn tool_use_world_with_login(
    initial_block_state_id: i32,
    login: Option<(&str, i8)>,
) -> WorldStore {
    let single_value = |domain, entry_count, global_id| crate::PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: crate::PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    };
    let air = block_state_id("minecraft:air", []);
    let mut world = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    if let Some((dimension, game_type)) = login {
        world.apply_login(&PlayLogin {
            player_id: 7,
            hardcore: false,
            levels: vec![dimension.to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: dimension.to_string(),
                seed: 0,
                game_type,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world.apply_set_chunk_cache_center(SetChunkCacheCenter {
            chunk_x: 0,
            chunk_z: 0,
        });
    }
    let section_count = usize::try_from(world.dimension().height.div_euclid(16)).unwrap();
    world.insert_decoded_chunk(crate::ChunkColumn {
        pos: crate::ChunkPos { x: 0, z: 0 },
        state: crate::ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..section_count)
            .map(|_| {
                std::sync::Arc::new(crate::ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value(crate::PaletteDomain::BlockStates, 4096, air),
                    biomes: single_value(crate::PaletteDomain::Biomes, 64, 0),
                })
            })
            .collect(),
        block_entities: std::sync::Arc::default(),
        light: crate::LightData::default(),
    });
    // Scene setup tolerates the vanilla identical-state no-op (an all-air
    // chunk already holds an air initial state).
    world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
        block_state_id: initial_block_state_id,
    });
    assert_eq!(
        world
            .probe_block(BlockPos { x: 0, y: 0, z: 0 })
            .map(|block| block.block_state_id),
        Some(initial_block_state_id)
    );
    world
}

pub(super) fn add_entity(world: &mut WorldStore, id: i32, entity_type_id: i32) {
    add_entity_at(world, id, entity_type_id, Vec3d::default());
}

pub(super) fn set_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
    // Scene setup tolerates the vanilla identical-state no-op; the block
    // must hold the requested state either way.
    world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        block_state_id,
    });
    assert_eq!(
        world.probe_block(pos).map(|block| block.block_state_id),
        Some(block_state_id)
    );
}

pub(super) fn item_use_block_hit(
    pos: BlockPos,
    direction: ProtocolDirection,
) -> LocalItemUsePovResult {
    LocalItemUsePovResult::Block {
        hit: BlockHitResult {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            direction,
            cursor_x: 0.5,
            cursor_y: 0.5,
            cursor_z: 0.5,
            inside: false,
            world_border_hit: false,
        },
        location: Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        },
    }
}

pub(super) fn bucket_success_with(
    sound_action: LocalInteractionSoundAction,
    particle_action: Option<LocalInteractionParticleAction>,
) -> LocalItemUsePrediction {
    LocalItemUsePrediction::Result(LocalInteractionResult::Success(LocalInteractionSuccess {
        swing_source: LocalInteractionSwingSource::Client,
        was_item_interaction: true,
        starts_using_item: false,
        extra_swing: None,
        sound_action: Some(sound_action),
        particle_action,
    }))
}

pub(super) fn add_entity_at(world: &mut WorldStore, id: i32, entity_type_id: i32, position: Vec3d) {
    world.apply_add_entity(AddEntity {
        id,
        uuid: Uuid::from_u128(id as u128),
        entity_type_id,
        position,
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
}

pub(super) fn equippable(can_be_sheared: bool) -> EquippableSummary {
    let sound = || SoundEventSummary {
        registry_id: Some(0),
        sound_id: None,
        fixed_range_bits: None,
    };
    EquippableSummary {
        slot_id: 0,
        equip_sound: sound(),
        asset_id: None,
        camera_overlay: None,
        allowed_entities: None,
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared,
        shearing_sound: sound(),
    }
}

pub(super) fn install_test_enchantment_registry(world: &mut WorldStore) {
    world.record_registry_data(RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![
            RegistryDataEntry {
                id: "minecraft:binding_curse".to_string(),
                raw_data: None,
            },
            RegistryDataEntry {
                id: "minecraft:sharpness".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 0,
    });
}
