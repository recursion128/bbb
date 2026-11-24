use super::*;

/// Vanilla registers the clientbound play bundle delimiter beside the bundle
/// packet (`GameProtocols.java:131-134`). It is a fieldless marker: bundling
/// writes one before and after the contained packets (`BundlerInfo.java:18-24`).
pub fn encode_play_clientbound_bundle_delimiter() -> (i32, Vec<u8>) {
    (ids::play::CLIENTBOUND_BUNDLE_DELIMITER, Vec::new())
}

/// `ClientboundUpdateTagsPacket` (`PlayerList.reloadResources`,
/// `ClientboundUpdateTagsPacket.java:10-14`) reuses the same tag payload in
/// configuration and play, but carries the play-state packet id.
pub fn encode_play_clientbound_update_tags(update_tags: &UpdateTags) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::play::CLIENTBOUND_UPDATE_TAGS,
        encode_update_tags(update_tags)?,
    ))
}

pub fn encode_play_clientbound_login(login: &PlayLogin) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_i32(login.player_id);
    encoder.write_bool(login.hardcore);
    write_len(&mut encoder, login.levels.len(), "play login levels")?;
    for level in &login.levels {
        encoder.write_string(&normalize_resource_location(level.clone())?);
    }
    encoder.write_var_i32(login.max_players);
    encoder.write_var_i32(login.chunk_radius);
    encoder.write_var_i32(login.simulation_distance);
    encoder.write_bool(login.reduced_debug_info);
    encoder.write_bool(login.show_death_screen);
    encoder.write_bool(login.do_limited_crafting);
    encode_common_spawn_info(&mut encoder, &login.common_spawn_info)?;
    encoder.write_bool(login.enforces_secure_chat);
    Ok((ids::play::CLIENTBOUND_LOGIN, encoder.into_inner()))
}

/// `ClientboundRespawnPacket.write` (`ClientboundRespawnPacket.java:20-23`): the
/// same `CommonPlayerSpawnInfo` the play login carries, then the retained-data
/// mask as a single byte.
///
/// The mask is a bit set, not an enum: `shouldKeep` tests it against
/// `KEEP_ATTRIBUTE_MODIFIERS`, `KEEP_ENTITY_DATA` and `KEEP_ALL_DATA`
/// (`ClientboundRespawnPacket.java:12-14`, `:34-36`). The two senders pick
/// opposite ends of it -- `PlayerList.respawn` sends 1 or 0 depending on whether
/// the death kept the player's data (`PlayerList.java:409`, `:412`) while
/// `ServerPlayer.teleport` sends 3 for a dimension change (`ServerPlayer.java:1104`)
/// -- so the encoder carries whatever byte the caller decided rather than
/// re-deriving it.
///
/// The spawn info itself is `ServerPlayer.createCommonSpawnInfo`
/// (`ServerPlayer.java:2151-2164`), read off the destination `ServerLevel`: its
/// dimension type registration and key, the obfuscated seed, the player's game
/// type and previous game type, the debug and flat flags, the last death
/// location, the portal cooldown and the sea level. Encoding it is shared with
/// the play login packet, which is why there is one `encode_common_spawn_info`.
pub fn encode_play_clientbound_respawn(respawn: &Respawn) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encode_common_spawn_info(&mut encoder, &respawn.common_spawn_info)?;
    encoder.write_i8(respawn.data_to_keep);
    Ok((ids::play::CLIENTBOUND_RESPAWN, encoder.into_inner()))
}

pub fn encode_play_clientbound_keep_alive(id: i64) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_i64(id);
    (ids::play::CLIENTBOUND_KEEP_ALIVE, encoder.into_inner())
}

/// `ClientboundSetEntityDataPacket.write`
/// (`ClientboundSetEntityDataPacket.java:15-43`): the entity id, then each
/// dirty value as id + serializer + payload, closed by the `EOF_MARKER`.
///
/// Only the value kinds this server publishes are encodable; anything else
/// fails closed rather than guessing a payload. The decoder is already the full
/// inverse, so this grows one kind at a time as server mechanisms land.
pub fn encode_play_clientbound_set_entity_data(update: &SetEntityData) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_set_entity_data_with_content_registry(
        update,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_set_entity_data_with_content_registry(
    update: &SetEntityData,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    for value in &update.values {
        encoder.write_u8(value.data_id);
        encoder.write_var_i32(value.serializer_id);
        match &value.value {
            EntityDataValueKind::Byte(byte) => encoder.write_i8(*byte),
            EntityDataValueKind::Boolean(value) => encoder.write_bool(*value),
            EntityDataValueKind::BlockPos(pos) => {
                encoder.write_i64(chunks::encode_block_pos(*pos));
            }
            EntityDataValueKind::OptionalBlockPos(value) => {
                encoder.write_bool(value.is_some());
                if let Some(pos) = value {
                    encoder.write_i64(chunks::encode_block_pos(*pos));
                }
            }
            // `EntityDataSerializers.ITEM_STACK`
            // (`EntityDataSerializers.java:62-66`) is
            // `ItemStack.OPTIONAL_STREAM_CODEC`, the same writer every
            // container packet already goes through.
            EntityDataValueKind::ItemStack(item) => {
                serverbound::encode_item_stack_summary_with_content_registry(
                    &mut encoder,
                    item,
                    super::super::data_components::ComponentValueFraming::Direct,
                    registry,
                )?
            }
            EntityDataValueKind::Int(value) => encoder.write_var_i32(*value),
            EntityDataValueKind::Long(value) => encoder.write_var_i64(*value),
            EntityDataValueKind::Float(value) => encoder.write_f32(*value),
            EntityDataValueKind::String(value) => encoder.write_java_string(value, 32767)?,
            EntityDataValueKind::Component(component) => {
                encode_entity_data_component(&mut encoder, component)?
            }
            EntityDataValueKind::OptionalComponent(component) => {
                encoder.write_bool(component.is_some());
                if let Some(component) = component {
                    encode_entity_data_component(&mut encoder, component)?;
                }
            }
            EntityDataValueKind::Rotations { x, y, z } => {
                encoder.write_f32(*x);
                encoder.write_f32(*y);
                encoder.write_f32(*z);
            }
            EntityDataValueKind::Direction(value) => encoder.write_var_i32(*value),
            EntityDataValueKind::OptionalLivingEntityReference(uuid) => {
                encoder.write_bool(uuid.is_some());
                if let Some(uuid) = uuid {
                    encoder.write_uuid(*uuid);
                }
            }
            EntityDataValueKind::BlockState(value) => encoder.write_var_i32(*value),
            // `EntityDataSerializers.OPTIONAL_BLOCK_STATE`
            // (`EntityDataSerializers.java:73-87`) spells the empty optional
            // as id zero, so every present state is written one higher than the
            // registry id the decoder took back off it.
            EntityDataValueKind::OptionalBlockState(value) => {
                encoder.write_var_i32(value.unwrap_or(0))
            }
            EntityDataValueKind::Particle(particle) => {
                world_effects::encode_particle_payload(&mut encoder, particle)
            }
            EntityDataValueKind::Particles(particles) => {
                write_len(&mut encoder, particles.len(), "entity data particles")?;
                for particle in particles {
                    world_effects::encode_particle_payload(&mut encoder, particle);
                }
            }
            EntityDataValueKind::VillagerData {
                villager_type,
                profession,
                level,
            } => {
                encoder.write_var_i32(villager_type.holder_id());
                encoder.write_var_i32(profession.holder_id());
                encoder.write_var_i32(*level);
            }
            // `EntityDataSerializers.OPTIONAL_UNSIGNED_INT`
            // (`EntityDataSerializers.java:106-116`) writes `value + 1` and keeps
            // zero for the empty optional.
            EntityDataValueKind::OptionalUnsignedInt(value) => {
                encoder.write_var_i32(value.map_or(0, |value| value + 1))
            }
            EntityDataValueKind::Pose(value) => encoder.write_var_i32(*value),
            EntityDataValueKind::PaintingVariant(variant) => {
                encode_entity_data_painting_variant(&mut encoder, variant)?
            }
            EntityDataValueKind::Vector3f { x, y, z } => {
                encoder.write_f32(*x);
                encoder.write_f32(*y);
                encoder.write_f32(*z);
            }
            EntityDataValueKind::Quaternionf { x, y, z, w } => {
                encoder.write_f32(*x);
                encoder.write_f32(*y);
                encoder.write_f32(*z);
                encoder.write_f32(*w);
            }
            EntityDataValueKind::RegistryId { value, .. } => {
                encoder.write_var_i32(value.holder_id())
            }
            EntityDataValueKind::EnumId { id, .. } => encoder.write_var_i32(*id),
            EntityDataValueKind::OptionalGlobalPos(value) => {
                encoder.write_bool(value.is_some());
                if let Some(global) = value {
                    encoder.write_string(&normalize_resource_location(global.dimension.clone())?);
                    encoder.write_i64(chunks::encode_block_pos(global.pos));
                }
            }
            // `HumanoidArm.STREAM_CODEC`, an id mapper (`HumanoidArm.java:13`).
            EntityDataValueKind::HumanoidArm(id) => encoder.write_var_i32(*id),
            EntityDataValueKind::ResolvableProfile(profile) => {
                encode_entity_data_resolvable_profile(&mut encoder, profile)?
            }
            EntityDataValueKind::ModCustom(bytes) => {
                if bytes.len() > 65_536 {
                    return Err(ProtocolError::PacketTooLarge(bytes.len(), 65_536));
                }
                encoder.write_var_i32(bytes.len() as i32);
                encoder.write_bytes(bytes);
            }
        }
    }
    encoder.write_u8(ENTITY_DATA_EOF_MARKER);
    Ok((ids::play::CLIENTBOUND_SET_ENTITY_DATA, encoder.into_inner()))
}

/// `ComponentSerialization.TRUSTED_STREAM_CODEC`
/// (`EntityDataSerializers.java:58`), the same NBT component every other
/// clientbound component field goes through.
fn encode_entity_data_component(
    encoder: &mut Encoder,
    component: &RichTextComponent,
) -> Result<()> {
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(component.component()),
        COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    Ok(())
}

/// `EntityDataSerializers.PAINTING_VARIANT`
/// (`EntityDataSerializers.java:134`) is `PaintingVariant.STREAM_CODEC`, a
/// `ByteBufCodecs.holder` (`ByteBufCodecs.java:603-610`): a registry id one
/// higher than the holder, or zero followed by the inline variant
/// (`PaintingVariant.java:28-40`).
fn encode_entity_data_painting_variant(
    encoder: &mut Encoder,
    variant: &PaintingVariantData,
) -> Result<()> {
    match (&variant.registry, &variant.direct) {
        (Some(reference), None) => {
            let holder_id = reference.holder_id();
            if holder_id < 0 || holder_id == i32::MAX {
                return Err(ProtocolError::InvalidData(format!(
                    "painting variant holder id {holder_id} has no wire encoding"
                )));
            }
            encoder.write_var_i32(holder_id + 1);
        }
        (None, Some(direct)) => {
            encoder.write_var_i32(0);
            encoder.write_var_i32(direct.width);
            encoder.write_var_i32(direct.height);
            encoder.write_string(&normalize_resource_location(direct.asset_id.clone())?);
            for component in [&direct.title, &direct.author] {
                encoder.write_bool(component.is_some());
                if let Some(component) = component {
                    encode_entity_data_component(encoder, component)?;
                }
            }
        }
        _ => {
            return Err(ProtocolError::InvalidData(
                "painting variant must be either a registry holder or an inline variant".into(),
            ))
        }
    }
    Ok(())
}

/// `ResolvableProfile.STREAM_CODEC` (`ResolvableProfile.java:36-42`) and
/// `ByteBufCodecs.GAME_PROFILE_PROPERTIES` (`ByteBufCodecs.java:209-234`).
fn encode_entity_data_resolvable_profile(
    encoder: &mut Encoder,
    profile: &super::super::ResolvableProfileData,
) -> Result<()> {
    use super::super::ResolvableProfileKind;
    let properties = match &profile.profile {
        ResolvableProfileKind::GameProfile {
            id,
            name,
            properties,
        } => {
            encoder.write_bool(true);
            encoder.write_uuid(*id);
            encoder.write_string(name);
            properties.as_slice()
        }
        ResolvableProfileKind::Partial {
            name,
            id,
            properties,
        } => {
            encoder.write_bool(false);
            encoder.write_bool(name.is_some());
            if let Some(name) = name {
                encoder.write_string(name);
            }
            encoder.write_bool(id.is_some());
            if let Some(id) = id {
                encoder.write_uuid(*id);
            }
            properties.as_slice()
        }
    };
    if properties.len() > 16 {
        return Err(ProtocolError::PacketTooLarge(properties.len(), 16));
    }
    encoder.write_var_i32(properties.len() as i32);
    for property in properties {
        if property.name.encode_utf16().count() > 64
            || property.value.encode_utf16().count() > 32_767
            || property
                .signature
                .as_ref()
                .is_some_and(|sig| sig.encode_utf16().count() > 1024)
        {
            return Err(ProtocolError::InvalidData(
                "entity data profile property exceeds vanilla string limit".into(),
            ));
        }
        encoder.write_string(&property.name);
        encoder.write_string(&property.value);
        encoder.write_bool(property.signature.is_some());
        if let Some(signature) = &property.signature {
            encoder.write_string(signature);
        }
    }
    let patch = &profile.skin_patch;
    for texture in [&patch.body, &patch.cape, &patch.elytra] {
        encoder.write_bool(texture.is_some());
        if let Some(texture) = texture {
            encoder.write_string(&normalize_resource_location(texture.clone())?);
        }
    }
    encoder.write_bool(patch.slim_model.is_some());
    if let Some(slim) = patch.slim_model {
        encoder.write_bool(slim);
    }
    Ok(())
}

/// `ClientboundSetEntityDataPacket.EOF_MARKER`
/// (`ClientboundSetEntityDataPacket.java:22`).
const ENTITY_DATA_EOF_MARKER: u8 = 0xff;

/// `ClientboundChunkBatchStartPacket.STREAM_CODEC`
/// (`ClientboundChunkBatchStartPacket.java:10`) is a unit codec: the packet is
/// the signal.
pub fn encode_play_clientbound_chunk_batch_start() -> (i32, Vec<u8>) {
    (ids::play::CLIENTBOUND_CHUNK_BATCH_START, Vec::new())
}

/// `ClientboundChunkBatchFinishedPacket.write`
/// (`ClientboundChunkBatchFinishedPacket.java:17-19`).
pub fn encode_play_clientbound_chunk_batch_finished(batch_size: i32) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(batch_size);
    (
        ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED,
        encoder.into_inner(),
    )
}

/// `ClientboundChangeDifficultyPacket.STREAM_CODEC`
/// (`ClientboundChangeDifficultyPacket.java:11-17`): the difficulty id as a
/// var int, then the lock flag.
pub fn encode_play_clientbound_change_difficulty(update: ChangeDifficulty) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.difficulty.id());
    encoder.write_bool(update.locked);
    (
        ids::play::CLIENTBOUND_CHANGE_DIFFICULTY,
        encoder.into_inner(),
    )
}

/// `ClientboundPlayerAbilitiesPacket.write`
/// (`ClientboundPlayerAbilitiesPacket.java:55-67`).
pub fn encode_play_clientbound_player_abilities(update: PlayerAbilities) -> (i32, Vec<u8>) {
    let mut flags = 0u8;
    if update.invulnerable {
        flags |= CLIENTBOUND_PLAYER_ABILITIES_FLAG_INVULNERABLE;
    }
    if update.flying {
        flags |= CLIENTBOUND_PLAYER_ABILITIES_FLAG_FLYING;
    }
    if update.can_fly {
        flags |= CLIENTBOUND_PLAYER_ABILITIES_FLAG_CAN_FLY;
    }
    if update.instabuild {
        flags |= CLIENTBOUND_PLAYER_ABILITIES_FLAG_INSTABUILD;
    }
    let mut encoder = Encoder::new();
    encoder.write_u8(flags);
    encoder.write_f32(update.flying_speed);
    encoder.write_f32(update.walking_speed);
    (
        ids::play::CLIENTBOUND_PLAYER_ABILITIES,
        encoder.into_inner(),
    )
}

pub fn encode_play_clientbound_player_position(update: PlayerPositionUpdate) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    encode_position(&mut encoder, update.position);
    encode_position(&mut encoder, update.delta_movement);
    encoder.write_f32(update.y_rot);
    encoder.write_f32(update.x_rot);
    encoder.write_i32(update.relatives_mask);
    (ids::play::CLIENTBOUND_PLAYER_POSITION, encoder.into_inner())
}

pub fn encode_play_clientbound_block_changed_ack(update: BlockChangedAck) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.sequence);
    (
        ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK,
        encoder.into_inner(),
    )
}

pub fn encode_play_clientbound_block_update(update: BlockUpdate) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_block_update_with_content_registry(
        update,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_block_update_with_content_registry(
    update: BlockUpdate,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    chunks::validate_block_state_id(update.block_state_id, registry)?;
    let mut encoder = Encoder::new();
    encoder.write_i64(chunks::encode_block_pos(update.pos));
    encoder.write_var_i32(update.block_state_id);
    Ok((ids::play::CLIENTBOUND_BLOCK_UPDATE, encoder.into_inner()))
}

/// `ClientboundSectionBlocksUpdatePacket.write`
/// (`ClientboundSectionBlocksUpdatePacket.java:53-60`): `SectionPos.asLong`
/// (`SectionPos.java:216-221`), then each change as `state << 12 | x << 8 | z << 4 | y`.
pub fn encode_play_clientbound_section_blocks_update(
    update: &super::super::SectionBlocksUpdate,
) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_section_blocks_update_with_content_registry(
        update,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_section_blocks_update_with_content_registry(
    update: &super::super::SectionBlocksUpdate,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    let section = ((i64::from(update.section_x) & 0x3f_ffff) << 42)
        | (i64::from(update.section_y) & 0xf_ffff)
        | ((i64::from(update.section_z) & 0x3f_ffff) << 20);
    encoder.write_i64(section);
    encoder.write_var_i32(update.updates.len() as i32);
    for change in &update.updates {
        chunks::validate_block_state_id(change.block_state_id, registry)?;
        let pos = change.pos;
        let relative = ((pos.x & 15) << 8) | ((pos.z & 15) << 4) | (pos.y & 15);
        encoder.write_var_i64((i64::from(change.block_state_id) << 12) | i64::from(relative));
    }
    Ok((
        ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE,
        encoder.into_inner(),
    ))
}

/// `ClientboundBlockEntityDataPacket.STREAM_CODEC`
/// (`ClientboundBlockEntityDataPacket.java:17-25`): the position, the block
/// entity type as a registry id, then the tag as a trusted compound.
pub fn encode_play_clientbound_block_entity_data(
    packet: &BlockEntityData,
) -> Result<(i32, Vec<u8>)> {
    crate::block_entity_types::vanilla_block_entity_type(packet.block_entity_type_id).ok_or_else(
        || {
            ProtocolError::InvalidData(format!(
                "unknown block entity type id {}",
                packet.block_entity_type_id
            ))
        },
    )?;
    let mut encoder = Encoder::new();
    encoder.write_i64(chunks::encode_block_pos(packet.pos));
    encoder.write_var_i32(packet.block_entity_type_id);
    // The tag is written as the producer encoded it, which is what keeps this
    // one codec from being a second owner of block-entity NBT.
    encoder.write_bytes(&packet.raw_nbt);
    Ok((
        ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA,
        encoder.into_inner(),
    ))
}

/// `ClientboundMountScreenOpenPacket.write`
/// (`ClientboundMountScreenOpenPacket.java:28-32`).
pub fn encode_play_clientbound_mount_screen_open(packet: MountScreenOpen) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(packet.container_id);
    encoder.write_var_i32(packet.inventory_columns);
    encoder.write_i32(packet.entity_id);
    (
        ids::play::CLIENTBOUND_MOUNT_SCREEN_OPEN,
        encoder.into_inner(),
    )
}

/// `ClientboundOpenBookPacket.write`: the hand as its enum ordinal.
pub fn encode_play_clientbound_open_book(packet: OpenBook) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(packet.hand.id());
    (ids::play::CLIENTBOUND_OPEN_BOOK, encoder.into_inner())
}

/// `ClientboundOpenSignEditorPacket.write`
/// (`ClientboundOpenSignEditorPacket.java:26-29`).
pub fn encode_play_clientbound_open_sign_editor(packet: OpenSignEditor) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_i64(chunks::encode_block_pos(packet.pos));
    encoder.write_bool(packet.is_front_text);
    (
        ids::play::CLIENTBOUND_OPEN_SIGN_EDITOR,
        encoder.into_inner(),
    )
}

/// `ClientboundBlockDestructionPacket.write`
/// (`ClientboundBlockDestructionPacket.java:29-33`): the breaking entity's id, the
/// block position, and the destroy stage as one byte.
///
/// `ServerLevel.destroyBlockProgress` (`ServerLevel.java:1009-1020`) is the only
/// sender, and it sends to every player in the level except the one whose id the
/// packet carries (`:1011`) and within 32 blocks (`:1015`). The stage is
/// `(int)(destroyProgress * 10.0F)` on the way in
/// (`ServerPlayerGameMode.java:135`, `:211`) and `-1` to clear
/// (`ServerPlayerGameMode.java:123`, `:224`, `:243`, `:247`); vanilla writes it
/// with `writeByte` and reads it back with `readUnsignedByte`
/// (`ClientboundBlockDestructionPacket.java:26`, `:32`), so the field this server
/// carries is the `u8` that round trip lands on and `-1` is the `0xff` the
/// client's own `BlockDestructionProgress` gate rejects as out of stage range.
pub fn encode_play_clientbound_block_destruction(
    update: chunks::BlockDestruction,
) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    encoder.write_i64(chunks::encode_block_pos(update.pos));
    encoder.write_u8(update.progress);
    (
        ids::play::CLIENTBOUND_BLOCK_DESTRUCTION,
        encoder.into_inner(),
    )
}

/// `ClientboundBlockEventPacket.write` (`ClientboundBlockEventPacket.java:35-40`),
/// which is the wire form of the block event `ServerLevel.blockEvent` queues and
/// `ServerLevel.runBlockEvents` broadcasts once `Block.triggerEvent` accepted it.
///
/// The two payload bytes are opaque here for the same reason they are opaque on
/// the wire: their meaning belongs to the block that raised the event, not to the
/// packet. `b0`/`b1` are written with `writeByte` and read back with
/// `readUnsignedByte` (`:30-31`), so the round trip is over `u8`.
pub fn encode_play_clientbound_block_event(update: chunks::BlockEvent) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_block_event_with_content_registry(
        update,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_block_event_with_content_registry(
    update: chunks::BlockEvent,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    if chunks::validate_block_id(update.block_id, registry).is_err() {
        return Err(ProtocolError::InvalidData(format!(
            "unknown vanilla 26.1 block registry id {}",
            update.block_id
        )));
    }
    let mut encoder = Encoder::new();
    encoder.write_i64(chunks::encode_block_pos(update.pos));
    encoder.write_u8(update.b0);
    encoder.write_u8(update.b1);
    encoder.write_var_i32(update.block_id);
    Ok((ids::play::CLIENTBOUND_BLOCK_EVENT, encoder.into_inner()))
}

/// `ClientboundLevelEventPacket.write` (`ClientboundLevelEventPacket.java:32-37`),
/// the wire form of every `Level.levelEvent` and `Level.globalLevelEvent`
/// broadcast.
///
/// All four fields go out raw: the type is a plain `writeInt` rather than a
/// var-int (`:33`), the position is the packed long (`:34`), the data is a
/// second plain `writeInt` (`:35`) and `globalEvent` is one boolean byte
/// (`:36`).
///
/// Nothing is refused here, unlike
/// [`encode_play_clientbound_game_event`]. A `LevelEvent` id is not a registry
/// row -- `LevelEvent` is a bag of `public static final int`s
/// (`net/minecraft/world/level/block/LevelEvent.java`) that the client
/// switches on in `LevelEventHandler.levelEvent`, whose `default` arm does
/// nothing -- so an id vanilla has no case for is dropped by the client rather
/// than being a malformed packet, and the server has no set to check against.
pub fn encode_play_clientbound_level_particles(
    update: super::super::world_effects::LevelParticles,
) -> (i32, Vec<u8>) {
    (
        ids::play::CLIENTBOUND_LEVEL_PARTICLES,
        super::super::world_effects::encode_level_particles(&update),
    )
}

/// `ClientboundExplodePacket` (`ClientboundExplodePacket.java:17-25`).
pub fn encode_play_clientbound_explode(
    update: super::super::world_effects::Explosion,
) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::play::CLIENTBOUND_EXPLODE,
        super::super::world_effects::encode_explosion(&update)?,
    ))
}

pub fn encode_play_clientbound_level_event(update: chunks::LevelEvent) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_i32(update.event_type);
    encoder.write_i64(chunks::encode_block_pos(update.pos));
    encoder.write_i32(update.data);
    encoder.write_bool(update.global);
    (ids::play::CLIENTBOUND_LEVEL_EVENT, encoder.into_inner())
}

/// `ClientboundGameEventPacket.write` (`ClientboundGameEventPacket.java:46-49`):
/// the event id as one byte followed by the float parameter, which the client
/// reads back as `Type.TYPES.get(input.readUnsignedByte())` plus `readFloat`
/// (`:41-44`).
///
/// The id is not a free byte. `Type` registers each instance into the private
/// `TYPES` map at construction (`:68-76`), and the only instances vanilla builds
/// are the fourteen constants the packet declares (`:14-27`) -- exactly the set
/// `game_event_types.rs` is generated from. An id outside that set resolves to a
/// null `Type` on the client and then matches none of the identity comparisons in
/// `ClientPacketListener.handleGameEvent` (`ClientPacketListener.java:1534-1598`),
/// so it is dropped without effect; this encoder refuses it the same way
/// `encode_play_clientbound_block_event` refuses a block id vanilla has no block
/// for.
///
/// The parameter stays a raw `f32` because its meaning belongs to the event, not
/// to the packet: `CHANGE_GAME_MODE` carries a game type id the client rounds with
/// `Mth.floor(param + 0.5F)` (`ClientPacketListener.java:1539`, `:1547`),
/// `DEMO_EVENT` carries one of the `DEMO_PARAM_*` constants
/// (`ClientboundGameEventPacket.java:28-32`), and `RAIN_LEVEL_CHANGE` /
/// `THUNDER_LEVEL_CHANGE` carry the level itself
/// (`ClientPacketListener.java:1581`, `:1583`).
pub fn encode_play_clientbound_game_event(update: GameEvent) -> Result<(i32, Vec<u8>)> {
    crate::game_event_types::vanilla_client_game_event_type(update.event_id).ok_or_else(|| {
        ProtocolError::InvalidData(format!(
            "unknown vanilla 26.1 client game event id {}",
            update.event_id
        ))
    })?;
    let mut encoder = Encoder::new();
    encoder.write_u8(update.event_id);
    encoder.write_f32(update.param);
    Ok((ids::play::CLIENTBOUND_GAME_EVENT, encoder.into_inner()))
}

/// `ClientboundPlayerLookAtPacket.write`
/// (`ClientboundPlayerLookAtPacket.java:61-71`): the anchor the looking entity
/// turns from, the point it turns towards, and — only when the packet was built
/// against an entity — that entity's id and the anchor its position was taken
/// at.
///
/// The position is written whichever constructor built the packet: the
/// entity-facing one resolves `toAnchor.apply(entity)` on the server and stores
/// the result (`:35-44`), so the coordinates are the fallback the client uses
/// when the entity id names nothing it tracks (`getPosition`, `:86-93`).
///
/// `writeEnum` is `writeVarInt(value.ordinal())`, which is the ordinal
/// `entity_anchors.rs` is generated from.
pub fn encode_play_clientbound_player_look_at(update: PlayerLookAt) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.from_anchor.ordinal());
    encode_position(&mut encoder, update.position);
    encoder.write_bool(update.target.is_some());
    if let Some(target) = update.target {
        encoder.write_var_i32(target.entity_id);
        encoder.write_var_i32(target.to_anchor.ordinal());
    }
    (ids::play::CLIENTBOUND_PLAYER_LOOK_AT, encoder.into_inner())
}
