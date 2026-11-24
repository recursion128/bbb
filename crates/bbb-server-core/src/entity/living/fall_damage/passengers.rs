use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{vanilla_entity_type, VANILLA_ENTITY_TYPE_PLAYER_ID};

use crate::entity::{Entity, EntityData, EntityStore};

pub(crate) fn cause_fall_damage(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    distance: f64,
    modifier: f32,
    damage_type: &'static str,
) -> Option<bool> {
    bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == damage_type)?;
    dispatch(entity, store, level, distance, modifier, damage_type)
}

fn dispatch(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    distance: f64,
    modifier: f32,
    damage_type: &'static str,
) -> Option<bool> {
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_PLAYER_ID {
        return None;
    }
    match entity.data() {
        EntityData::Player | EntityData::EnderDragonPart => None,
        EntityData::GeneralLiving(_) => crate::entity::general_living::cause_fall_damage_inner(
            entity,
            store,
            level,
            distance,
            modifier,
            damage_type,
        ),
        EntityData::ArmorStand(_) => crate::entity::armor_stand::cause_fall_damage_inner(
            entity,
            store,
            level,
            distance,
            modifier,
            damage_type,
        ),
        EntityData::FallingBlock(_) => crate::entity::falling_block::cause_fall_damage_inner(
            entity,
            store,
            level,
            distance,
            modifier,
            damage_type,
        ),
        EntityData::TntMinecart(_) => None,
        EntityData::Item(_)
        | EntityData::ExperienceOrb(_)
        | EntityData::Tnt(_)
        | EntityData::Painting(_)
        | EntityData::ItemFrame(_)
        | EntityData::GlowItemFrame(_)
        | EntityData::LeashKnot(_)
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
        | EntityData::SpawnerMinecart(_) => {
            let resource = vanilla_entity_type(entity.entity_type_id())?.resource_id;
            if level.registry_tag_membership(
                "minecraft:entity_type",
                "minecraft:fall_damage_immune",
                resource,
            ) {
                Some(false)
            } else {
                propagate_fall_to_passengers(
                    entity,
                    store,
                    level,
                    distance,
                    modifier,
                    damage_type,
                )?;
                Some(false)
            }
        }
    }
}

pub(crate) fn propagate_fall_to_passengers(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    distance: f64,
    modifier: f32,
    damage_type: &'static str,
) -> Option<()> {
    let passengers = store.passengers(entity.id()).to_vec();
    for id in passengers {
        if store.is_external_player_passenger(id) {
            let player = store.player(id)?.uuid();
            level.fall_player(crate::level_event::PendingPlayerFall {
                player,
                distance,
                modifier,
                damage_type,
            });
            continue;
        }
        if id == entity.id()
            || !store.passenger_relation_matches(id, Some(entity.id()))
            || store.has_indirect_passenger(id, entity.id())
        {
            return None;
        }
        let mut body = store.detach(id)?;
        let result = level.with_detached_entity(entity, |level| {
            dispatch(&mut body, store, level, distance, modifier, damage_type)
        });
        store.attach(body);
        result?;
    }
    Some(())
}
