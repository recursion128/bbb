use super::*;
use crate::entity::abstract_arrow::AbstractArrowState;
use bbb_server_types::item_stack::RuntimeItemStack;

pub(super) fn write_spectral_arrow(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &SpectralArrowData,
    items: crate::entity::item::ItemResolver<'_>,
) -> Result<(), EntitySaveError> {
    write_abstract_arrow(entries, data, items)?;
    entries.push(NbtSummaryEntry {
        name: "Duration".into(),
        value: NbtSummaryValue::Int(data.duration()),
    });
    Ok(())
}

pub(super) fn write_trident(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &TridentData,
    items: crate::entity::item::ItemResolver<'_>,
) -> Result<(), EntitySaveError> {
    write_abstract_arrow(entries, data, items)?;
    entries.push(NbtSummaryEntry {
        name: "DealtDamage".into(),
        value: byte_bool(data.dealt_damage()),
    });
    Ok(())
}

pub(super) fn write_abstract_arrow(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &crate::entity::abstract_arrow::AbstractArrowState,
    items: crate::entity::item::ItemResolver<'_>,
) -> Result<(), EntitySaveError> {
    write_projectile(entries, data.projectile());
    entries.extend([
        NbtSummaryEntry {
            name: "life".into(),
            value: NbtSummaryValue::Short(data.life() as i16),
        },
        NbtSummaryEntry {
            name: "shake".into(),
            value: NbtSummaryValue::Byte(data.shake_time() as i8),
        },
        NbtSummaryEntry {
            name: "inGround".into(),
            value: byte_bool(data.in_ground()),
        },
        NbtSummaryEntry {
            name: "pickup".into(),
            value: NbtSummaryValue::Byte(data.pickup().to_byte()),
        },
        NbtSummaryEntry {
            name: "damage".into(),
            value: NbtSummaryValue::Double(data.damage().to_bits()),
        },
        NbtSummaryEntry {
            name: "crit".into(),
            value: byte_bool(data.crit()),
        },
        NbtSummaryEntry {
            name: "PierceLevel".into(),
            value: NbtSummaryValue::Byte(data.pierce_level()),
        },
        NbtSummaryEntry {
            name: "SoundEvent".into(),
            value: NbtSummaryValue::string(data.sound_event_resource_id()),
        },
        NbtSummaryEntry {
            name: "item".into(),
            value: crate::entity::item::write_saved_stack(data.item(), items),
        },
    ]);
    if let Some(block_state_id) = data.in_block_state_id() {
        entries.push(NbtSummaryEntry {
            name: "inBlockState".into(),
            value: super::nonliving::block_state_value_with_registry(
                block_state_id,
                "inBlockState",
                items.content_registry(),
            )?,
        });
    }
    if let Some(weapon) = data.weapon() {
        entries.push(NbtSummaryEntry {
            name: "weapon".into(),
            value: crate::entity::item::write_saved_stack(weapon, items),
        });
    }
    Ok(())
}

/// `FishingHook.addAdditionalSaveData` (`FishingHook.java:429-430`) is empty and
/// the type is `noSave()`, so a fishing bobber is never written; this refuses
/// rather than emit a record, matching the `entity_record` dispatch arm.
pub(super) fn write_fishing_bobber(_data: &FishingBobberData) -> Result<(), EntitySaveError> {
    Err(field_error(
        "id",
        "minecraft:fishing_bobber has noSave() and is never serialized",
    ))
}

fn write_projectile(entries: &mut Vec<NbtSummaryEntry>, state: &ProjectileState) {
    if let Some(owner) = state.owner() {
        entries.push(NbtSummaryEntry {
            name: "Owner".into(),
            value: uuid_int_array(owner),
        });
    }
    if state.left_owner() {
        entries.push(NbtSummaryEntry {
            name: "LeftOwner".into(),
            value: byte_bool(true),
        });
    }
    entries.push(NbtSummaryEntry {
        name: "HasBeenShot".into(),
        value: byte_bool(state.has_been_shot()),
    });
}

/// `ShulkerBullet.addAdditionalSaveData` (`ShulkerBullet.java:66-77`).
pub(super) fn write_shulker_bullet(entries: &mut Vec<NbtSummaryEntry>, data: &ShulkerBulletData) {
    write_projectile(entries, data.projectile());
    if let Some(target) = data.target() {
        entries.push(NbtSummaryEntry {
            name: "Target".into(),
            value: uuid_int_array(target),
        });
    }
    if let Some(direction) = data.current_direction() {
        entries.push(NbtSummaryEntry {
            name: "Dir".into(),
            value: NbtSummaryValue::Byte(direction.id() as i8),
        });
    }
    entries.extend([
        NbtSummaryEntry {
            name: "Steps".into(),
            value: NbtSummaryValue::Int(data.flight_steps()),
        },
        NbtSummaryEntry {
            name: "TXD".into(),
            value: NbtSummaryValue::Double(data.target_delta().x.to_bits()),
        },
        NbtSummaryEntry {
            name: "TYD".into(),
            value: NbtSummaryValue::Double(data.target_delta().y.to_bits()),
        },
        NbtSummaryEntry {
            name: "TZD".into(),
            value: NbtSummaryValue::Double(data.target_delta().z.to_bits()),
        },
    ]);
}

pub(super) fn write_hurting_projectile(
    entries: &mut Vec<NbtSummaryEntry>,
    state: &HurtingProjectileState,
) {
    write_projectile(entries, state.projectile());
    entries.push(NbtSummaryEntry {
        name: "acceleration_power".into(),
        value: NbtSummaryValue::Double(bbb_protocol::nbt_summary::canonical_double_tag_bits(
            state.acceleration_power(),
        )),
    });
}

pub(super) fn write_large_fireball(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &LargeFireballData,
    items: crate::entity::item::ItemResolver<'_>,
) {
    write_hurting_projectile(entries, data.projectile());
    entries.push(NbtSummaryEntry {
        name: "Item".into(),
        value: crate::entity::item::write_saved_stack(data.item(), items),
    });
    entries.push(NbtSummaryEntry {
        name: "ExplosionPower".into(),
        value: NbtSummaryValue::Byte(data.explosion_power()),
    });
}

pub(super) fn write_small_fireball(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &SmallFireballData,
    items: crate::entity::item::ItemResolver<'_>,
) {
    write_hurting_projectile(entries, data.projectile());
    entries.push(NbtSummaryEntry {
        name: "Item".into(),
        value: crate::entity::item::write_saved_stack(data.item(), items),
    });
}

pub(super) fn write_wither_skull(entries: &mut Vec<NbtSummaryEntry>, data: &WitherSkullData) {
    write_hurting_projectile(entries, data.projectile());
    entries.push(NbtSummaryEntry {
        name: "dangerous".into(),
        value: byte_bool(data.is_dangerous()),
    });
}

/// `Projectile.addAdditionalSaveData` (`Projectile.java:70-77`) then
/// `ThrowableItemProjectile.addAdditionalSaveData` (`ThrowableItemProjectile.java:56-59`).
pub(super) fn write_throwable_item(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &ThrowableItemData,
    items: crate::entity::item::ItemResolver<'_>,
) {
    write_projectile(entries, data.projectile());
    entries.push(NbtSummaryEntry {
        name: "Item".into(),
        value: crate::entity::item::write_saved_stack(data.item(), items),
    });
}

/// `Projectile.addAdditionalSaveData` (`Projectile.java:70-77`) then
/// `FireworkRocketEntity.addAdditionalSaveData` (`FireworkRocketEntity.java:277-282`).
pub(super) fn write_firework_rocket(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &FireworkRocketData,
    items: crate::entity::item::ItemResolver<'_>,
) {
    write_projectile(entries, data.projectile());
    entries.push(NbtSummaryEntry {
        name: "Life".into(),
        value: NbtSummaryValue::Int(data.life()),
    });
    entries.push(NbtSummaryEntry {
        name: "LifeTime".into(),
        value: NbtSummaryValue::Int(data.lifetime()),
    });
    entries.push(NbtSummaryEntry {
        name: "FireworksItem".into(),
        value: crate::entity::item::write_saved_stack(data.item(), items),
    });
    entries.push(NbtSummaryEntry {
        name: "ShotAtAngle".into(),
        value: byte_bool(data.shot_at_angle()),
    });
}

/// `Projectile.addAdditionalSaveData` (`Projectile.java:70-77`).
pub(super) fn write_llama_spit(entries: &mut Vec<NbtSummaryEntry>, data: &LlamaSpitData) {
    write_projectile(entries, data);
}

/// `EvokerFangs.addAdditionalSaveData` (`EvokerFangs.java:63-66`): `Warmup`
/// always, `Owner` only when the reference is set.
pub(super) fn write_evoker_fangs(entries: &mut Vec<NbtSummaryEntry>, data: &EvokerFangsData) {
    entries.push(NbtSummaryEntry {
        name: "Warmup".into(),
        value: NbtSummaryValue::Int(data.warmup_delay_ticks()),
    });
    if let Some(owner) = data.owner() {
        // `EntityReference.store` (`EntityReference.java:96-100`) writes nothing
        // for a null reference.
        entries.push(NbtSummaryEntry {
            name: "Owner".into(),
            value: uuid_int_array(owner),
        });
    }
}

type Items<'a> = &'a dyn bbb_registry_codec::PersistentItemStackHolderResolver;

fn default_stack(item_resource_id: &str) -> RuntimeItemStack {
    item::plain_stack(item_resource_id, 1).expect("built-in default item")
}

/// `ItemStack.CODEC` refuses an empty stack, so it reads as absent.
fn read_stack(
    entries: &[NbtSummaryEntry],
    name: &str,
    items: Items<'_>,
) -> Option<RuntimeItemStack> {
    optional_field(entries, name)
        .and_then(|value| item::read_saved_stack(value, items))
        .filter(|stack| !stack.is_empty())
}

/// `Projectile.readAdditionalSaveData` (`Projectile.java:84-88`).
pub(super) fn read_projectile(entries: &[NbtSummaryEntry]) -> ProjectileState {
    ProjectileState::from_saved(
        optional_uuid(entries, "Owner"),
        optional_bool(entries, "LeftOwner").unwrap_or(false),
        optional_bool(entries, "HasBeenShot").unwrap_or(false),
    )
}

/// `AbstractArrow.readAdditionalSaveData` (`AbstractArrow.java:616-629`).
fn read_abstract_arrow(
    entries: &[NbtSummaryEntry],
    items: Items<'_>,
    default_pickup_item: &str,
    default_sound: &str,
) -> AbstractArrowState {
    let pickup = match optional_field(entries, "pickup")
        .and_then(bbb_protocol::nbt_summary::codec_int_value)
    {
        Some(ordinal) => ArrowPickup::from_byte(ordinal as i8).unwrap_or(ArrowPickup::Disallowed),
        None => ArrowPickup::Disallowed,
    };
    let sound_event_resource_id = optional_field(entries, "SoundEvent")
        .and_then(|value| match value {
            NbtSummaryValue::String(id) => id
                .try_to_string()
                .ok()
                .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok()),
            _ => None,
        })
        .filter(|id| {
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(id).is_some()
                || items
                    .content_registry()
                    .sound_event_by_resource_id(id)
                    .is_some()
        })
        .unwrap_or_else(|| default_sound.to_string());
    AbstractArrowState::from_saved(
        i32::from(optional_short(entries, "life").unwrap_or(0)),
        optional_field(entries, "inBlockState").and_then(|value| {
            read_block_state_from_value_with_registry(value, items.content_registry())
        }),
        i32::from(optional_byte(entries, "shake").unwrap_or(0) as u8),
        optional_bool(entries, "inGround").unwrap_or(false),
        pickup,
        optional_double(entries, "damage").unwrap_or(2.0),
        optional_bool(entries, "crit").unwrap_or(false),
        optional_byte(entries, "PierceLevel").unwrap_or(0),
        sound_event_resource_id,
        read_stack(entries, "item", items).unwrap_or_else(|| default_stack(default_pickup_item)),
        read_stack(entries, "weapon", items),
        read_projectile(entries),
    )
}

// Arrow.java:122-124 constructs an ordinary arrow as the default pickup.
pub(super) fn read_arrow(entries: &[NbtSummaryEntry], items: Items<'_>) -> ArrowData {
    ArrowData::from_state(read_abstract_arrow(
        entries,
        items,
        "minecraft:arrow",
        "minecraft:entity.arrow.hit",
    ))
}

/// `SpectralArrow.readAdditionalSaveData` (`SpectralArrow.java:50-53`).
pub(super) fn read_spectral_arrow(
    entries: &[NbtSummaryEntry],
    items: Items<'_>,
) -> SpectralArrowData {
    SpectralArrowData::from_state(
        read_abstract_arrow(
            entries,
            items,
            "minecraft:spectral_arrow",
            "minecraft:entity.arrow.hit",
        ),
        optional_int(entries, "Duration")
            .unwrap_or(crate::entity::spectral_arrow::DEFAULT_DURATION),
    )
}

/// `ThrownTrident.readAdditionalSaveData` (`ThrownTrident.java:194-198`).
pub(super) fn read_trident(entries: &[NbtSummaryEntry], items: Items<'_>) -> TridentData {
    TridentData::from_state(
        read_abstract_arrow(
            entries,
            items,
            "minecraft:trident",
            crate::entity::trident::DEFAULT_SOUND_EVENT_RESOURCE_ID,
        ),
        optional_bool(entries, "DealtDamage").unwrap_or(false),
        false,
    )
}

/// `ShulkerBullet.readAdditionalSaveData`: `Dir` is `Direction.LEGACY_ID_CODEC`.
pub(super) fn read_shulker_bullet(entries: &[NbtSummaryEntry]) -> ShulkerBulletData {
    ShulkerBulletData::from_saved(
        read_projectile(entries),
        optional_uuid(entries, "Target"),
        optional_field(entries, "Dir")
            .and_then(bbb_protocol::nbt_summary::codec_int_value)
            .map(|id| crate::entity::hanging::from_3d_id(id as i8)),
        optional_int(entries, "Steps").unwrap_or(0),
        Vec3d {
            x: optional_double(entries, "TXD").unwrap_or(0.0),
            y: optional_double(entries, "TYD").unwrap_or(0.0),
            z: optional_double(entries, "TZD").unwrap_or(0.0),
        },
    )
}

/// `AbstractHurtingProjectile.java:170-173`.
pub(super) fn read_hurting_projectile(entries: &[NbtSummaryEntry]) -> HurtingProjectileState {
    HurtingProjectileState::from_saved(
        read_projectile(entries),
        optional_double(entries, "acceleration_power")
            .unwrap_or(HurtingProjectileState::INITIAL_ACCELERATION_POWER),
    )
}

/// `Fireball.java:34-40, 63-66`: the item is copied with count one.
fn read_fireball_item(entries: &[NbtSummaryEntry], items: Items<'_>) -> RuntimeItemStack {
    read_stack(entries, "Item", items)
        .unwrap_or_else(crate::entity::hurting_projectile::fireball::default_item)
        .with_count(1)
}

/// `LargeFireball.java:59-62`.
pub(super) fn read_large_fireball(
    entries: &[NbtSummaryEntry],
    items: Items<'_>,
) -> LargeFireballData {
    LargeFireballData::new(
        read_hurting_projectile(entries),
        read_fireball_item(entries, items),
        optional_byte(entries, "ExplosionPower").unwrap_or(1),
    )
}

pub(super) fn read_small_fireball(
    entries: &[NbtSummaryEntry],
    items: Items<'_>,
) -> SmallFireballData {
    SmallFireballData::new(
        read_hurting_projectile(entries),
        read_fireball_item(entries, items),
    )
}

/// `WitherSkull.java:126-129`.
pub(super) fn read_wither_skull(entries: &[NbtSummaryEntry]) -> WitherSkullData {
    WitherSkullData::new(
        read_hurting_projectile(entries),
        optional_bool(entries, "dangerous").unwrap_or(false),
    )
}

/// `FishingHook` is `noSave()`; a synthetic record only carries its `Owner`.
pub(super) fn read_fishing_bobber(entries: &[NbtSummaryEntry]) -> FishingBobberData {
    FishingBobberData::new(
        optional_uuid(entries, "Owner").unwrap_or_else(Uuid::nil),
        0,
        0,
    )
}

/// `ThrowableItemProjectile.java:39-41,62-65`: the item is copied with count one.
pub(super) fn read_throwable_item(
    entries: &[NbtSummaryEntry],
    items: Items<'_>,
    default_item: &str,
) -> ThrowableItemData {
    let item = read_stack(entries, "Item", items)
        .unwrap_or_else(|| default_stack(default_item))
        .with_count(1);
    ThrowableItemData::from_saved(item, read_projectile(entries))
}

/// `AbstractThrownPotion.java:25-42` inherits the ThrowableItemProjectile save reader.
pub(super) fn read_thrown_potion(
    entries: &[NbtSummaryEntry],
    items: Items<'_>,
    kind: crate::entity::ThrownPotionKind,
) -> crate::entity::ThrownPotionData {
    read_throwable_item(entries, items, kind.item_resource_id())
}

/// `FireworkRocketEntity.readAdditionalSaveData` (`FireworkRocketEntity.java:286-291`).
pub(super) fn read_firework_rocket(
    entries: &[NbtSummaryEntry],
    items: Items<'_>,
) -> FireworkRocketData {
    let item = read_stack(entries, "FireworksItem", items)
        .unwrap_or_else(|| default_stack("minecraft:firework_rocket"));
    FireworkRocketData::from_saved(
        item,
        optional_int(entries, "Life").unwrap_or(0),
        optional_int(entries, "LifeTime").unwrap_or(0),
        optional_bool(entries, "ShotAtAngle").unwrap_or(false),
        read_projectile(entries),
    )
}

/// `EvokerFangs.readAdditionalSaveData` (`EvokerFangs.java:57-60`).
pub(super) fn read_evoker_fangs(entries: &[NbtSummaryEntry]) -> EvokerFangsData {
    EvokerFangsData::from_saved(
        optional_int(entries, "Warmup").unwrap_or(0),
        optional_uuid(entries, "Owner"),
    )
}

#[cfg(test)]
mod hurting_projectile_default_tests {
    use super::*;

    #[test]
    fn sparse_large_fireball_uses_vanilla_reader_defaults() {
        let resolver = bbb_registry_codec::UnavailableHolderResolver;
        let data = read_large_fireball(&[], &resolver);
        assert_eq!(data.projectile().owner(), None);
        assert!(!data.projectile().projectile().has_been_shot());
        assert_eq!(
            data.projectile().acceleration_power(),
            HurtingProjectileState::INITIAL_ACCELERATION_POWER
        );
        assert_eq!(data.explosion_power(), 1);
        let fire_charge = item::plain_stack("minecraft:fire_charge", 1).unwrap();
        assert_eq!(data.item(), &fire_charge);
    }
}
