use super::*;
/// Every specialized [`EntityData`] variant, with an exhaustive match that makes
/// adding a variant without classifying it a compile error rather than a silent gap.
fn every_specialized_entity_data_variant() -> Vec<EntityData> {
    let mut variants = vec![
        EntityData::FallingBlock(FallingBlockData::new(
            state_id("minecraft:sand"),
            pos(0, 1, 0),
        )),
        EntityData::Item(ItemEntityData::new(stack_of("minecraft:dirt", 1), 0.0)),
        EntityData::ExperienceOrb(ExperienceOrbData::new(7)),
        EntityData::Tnt(crate::entity::TntData::new()),
        EntityData::Painting(crate::entity::PaintingData::new(
            pos(0, 5, 0),
            bbb_protocol::packets::Direction::South,
            crate::entity::hanging::PaintingVariant::vanilla("minecraft:kebab"),
        )),
        EntityData::ItemFrame(crate::entity::ItemFrameData::new(
            pos(0, 5, 0),
            bbb_protocol::packets::Direction::North,
        )),
        EntityData::GlowItemFrame(crate::entity::ItemFrameData::new(
            pos(0, 5, 0),
            bbb_protocol::packets::Direction::East,
        )),
        EntityData::LeashKnot(crate::entity::LeashKnotData::new(pos(0, 5, 0))),
        EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
        EntityData::Marker(crate::entity::MarkerData::new()),
        EntityData::Interaction(crate::entity::InteractionData::new()),
        EntityData::BlockDisplay(crate::entity::BlockDisplayData::new()),
        EntityData::ItemDisplay(crate::entity::ItemDisplayData::new()),
        EntityData::TextDisplay(crate::entity::TextDisplayData::new()),
        EntityData::Arrow(crate::entity::ArrowData::new(
            plain_stack("minecraft:arrow", 1).expect("arrow is a built-in stack"),
        )),
        EntityData::SpectralArrow(crate::entity::SpectralArrowData::new(
            plain_stack("minecraft:spectral_arrow", 1).expect("spectral arrow is a built-in stack"),
        )),
        EntityData::Trident(crate::entity::TridentData::new(
            plain_stack("minecraft:trident", 1).expect("trident is a built-in stack"),
        )),
        EntityData::FishingBobber(crate::entity::FishingBobberData::new(
            uuid::Uuid::from_u128(0x_f1_5b_00_00),
            0,
            0,
        )),
        EntityData::Snowball(crate::entity::SnowballData::new(
            plain_stack("minecraft:snowball", 1).expect("snowball is a built-in stack"),
        )),
        EntityData::Egg(crate::entity::EggData::new(
            plain_stack("minecraft:egg", 1).expect("egg is a built-in stack"),
        )),
        EntityData::EnderPearl(crate::entity::EnderPearlData::new(
            plain_stack("minecraft:ender_pearl", 1).expect("ender pearl is a built-in stack"),
        )),
        EntityData::EndCrystal(crate::entity::EndCrystalData::new(0, true)),
        EntityData::EyeOfEnder(crate::entity::EyeOfEnderData::new(
            plain_stack("minecraft:ender_eye", 1).expect("ender eye is a built-in stack"),
        )),
        EntityData::ExperienceBottle(crate::entity::ExperienceBottleData::new(
            plain_stack("minecraft:experience_bottle", 1)
                .expect("experience bottle is a built-in stack"),
        )),
        EntityData::FireworkRocket(crate::entity::FireworkRocketData::new(
            plain_stack("minecraft:firework_rocket", 1)
                .expect("firework rocket is a built-in stack"),
        )),
        EntityData::LlamaSpit(crate::entity::LlamaSpitData::default()),
        EntityData::EvokerFangs(crate::entity::EvokerFangsData::new(0, None)),
        EntityData::LightningBolt(crate::entity::LightningBoltData::new(0, 1)),
        EntityData::LargeFireball(crate::entity::LargeFireballData::new(
            crate::entity::HurtingProjectileState::new(None, 0.1),
            plain_stack("minecraft:fire_charge", 1).expect("fire charge is a built-in stack"),
            1,
        )),
        EntityData::WindCharge(crate::entity::WindChargeData::new(
            crate::entity::HurtingProjectileState::new(None, 0.0),
            5,
        )),
        EntityData::BreezeWindCharge(crate::entity::WindChargeData::new(
            crate::entity::HurtingProjectileState::new(None, 0.0),
            0,
        )),
        EntityData::AreaEffectCloud(crate::entity::AreaEffectCloudData::new()),
        EntityData::OminousItemSpawner(crate::entity::OminousItemSpawnerData::from_saved(
            plain_stack("minecraft:stick", 1).unwrap(),
            60,
        )),
        EntityData::SplashPotion(crate::entity::ThrownPotionData::new(
            plain_stack("minecraft:splash_potion", 1).unwrap(),
        )),
        EntityData::LingeringPotion(crate::entity::ThrownPotionData::new(
            plain_stack("minecraft:lingering_potion", 1).unwrap(),
        )),
        EntityData::Minecart(crate::entity::MinecartData::new()),
        EntityData::FurnaceMinecart(crate::entity::FurnaceMinecartData::new()),
        EntityData::ChestMinecart(crate::entity::MinecartContainerData::new_chest()),
        EntityData::HopperMinecart(crate::entity::MinecartContainerData::new_hopper()),
        EntityData::CommandBlockMinecart(crate::entity::CommandBlockMinecartData::new()),
        EntityData::SpawnerMinecart(crate::entity::SpawnerMinecartData::new()),
        EntityData::TntMinecart(crate::entity::TntMinecartData::new()),
    ];
    variants.extend(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPES
            .iter()
            .filter_map(|entry| {
                crate::entity::BoatData::for_entity_type(entry.id)
                    .map(EntityData::Boat)
                    .or_else(|| {
                        crate::entity::ChestBoatData::for_entity_type(entry.id)
                            .map(EntityData::ChestBoat)
                    })
            }),
    );
    for variant in &variants {
        match variant {
            // General living save coverage needs a writer-validated subtype record.
            EntityData::GeneralLiving(_) => unreachable!("covered by general_living tests"),
            EntityData::Player => unreachable!("players are saved as playerdata"),
            EntityData::EnderDragonPart => unreachable!("dragon parts are not saved separately"),
            EntityData::FallingBlock(_)
            | EntityData::Item(_)
            | EntityData::ExperienceOrb(_)
            | EntityData::Tnt(_)
            | EntityData::Painting(_)
            | EntityData::ItemFrame(_)
            | EntityData::GlowItemFrame(_)
            | EntityData::LeashKnot(_)
            | EntityData::ArmorStand(_)
            | EntityData::Marker(_)
            | EntityData::Interaction(_)
            | EntityData::BlockDisplay(_)
            | EntityData::ItemDisplay(_)
            | EntityData::TextDisplay(_)
            | EntityData::Arrow(_)
            | EntityData::SpectralArrow(_)
            | EntityData::Trident(_)
            | EntityData::FishingBobber(_)
            | EntityData::Snowball(_)
            | EntityData::Egg(_)
            | EntityData::EnderPearl(_)
            | EntityData::EndCrystal(_)
            | EntityData::EyeOfEnder(_)
            | EntityData::ExperienceBottle(_)
            | EntityData::FireworkRocket(_)
            | EntityData::LlamaSpit(_)
            | EntityData::EvokerFangs(_)
            | EntityData::LightningBolt(_)
            | EntityData::LargeFireball(_)
            | EntityData::SmallFireball(_)
            | EntityData::DragonFireball(_)
            | EntityData::WitherSkull(_)
            | EntityData::WindCharge(_)
            | EntityData::BreezeWindCharge(_)
            | EntityData::AreaEffectCloud(_)
            | EntityData::OminousItemSpawner(_)
            | EntityData::SplashPotion(_)
            | EntityData::LingeringPotion(_)
            | EntityData::ShulkerBullet(_)
            | EntityData::Boat(_)
            | EntityData::ChestBoat(_)
            | EntityData::Minecart(_)
            | EntityData::FurnaceMinecart(_)
            | EntityData::ChestMinecart(_)
            | EntityData::HopperMinecart(_)
            | EntityData::CommandBlockMinecart(_)
            | EntityData::SpawnerMinecart(_)
            | EntityData::TntMinecart(_) => {}
        }
    }
    variants
}

#[test]
fn every_specialized_entity_data_variant_names_a_built_in_type_and_survives_a_save() {
    for variant in every_specialized_entity_data_variant() {
        let resource_id = variant.entity_resource_id();
        let entity_type_id = vanilla_entity_type_id_for_resource_id(resource_id)
            .unwrap_or_else(|| panic!("{resource_id} is a built-in 26.1 entity type"));

        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = store.spawn(
            entity_type_id,
            Vec3d {
                x: 0.5,
                y: 5.0,
                z: 0.5,
            },
            variant.clone(),
        );
        assert!(store.get(id).is_some(), "{resource_id} was not filed");

        if !vanilla_entity_type_is_serializable(entity_type_id) {
            // A `noSave()` type is skipped by the writer (`Entity.saveAsPassenger`
            // returns false), so its chunk carries no record and there is
            // nothing to read back. Removal is still the shared shape.
            let document = write_entity_chunk(&store, (0, 0))
                .unwrap_or_else(|error| panic!("{resource_id} has no writer: {error}"));
            assert!(
                document.is_empty(),
                "{resource_id} is noSave and must not be written"
            );
            let uuid = store.get(id).expect("entity is live").uuid();
            assert!(store.contains_uuid(uuid));
            assert!(store.remove(id, RemovalReason::Discarded).is_some());
            assert!(!store.contains_uuid(uuid));
            assert!(store.is_empty());
            continue;
        }

        let document = write_entity_chunk(&store, (0, 0))
            .unwrap_or_else(|error| panic!("{resource_id} has no writer: {error}"));
        assert_eq!(document.entities().len(), 1, "{resource_id}");
        assert_eq!(document.entities().next().unwrap().id(), resource_id);

        let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut reloaded, &document)
            .unwrap_or_else(|error| panic!("{resource_id} has no reader: {error}"));
        assert_eq!(ids.len(), 1, "{resource_id}");
        let after = reloaded.get(ids[0]).expect("entity is live");
        assert_eq!(after.data().entity_resource_id(), resource_id);
        assert_eq!(after.entity_type_id(), entity_type_id);

        // Removal is the same shape for every variant: the slot goes, the UUID
        // stops being known, and the section index empties.
        let uuid = after.uuid();
        assert!(reloaded.contains_uuid(uuid));
        assert!(reloaded.remove(ids[0], RemovalReason::Discarded).is_some());
        assert!(!reloaded.contains_uuid(uuid));
        assert!(reloaded.is_empty());
    }
}
