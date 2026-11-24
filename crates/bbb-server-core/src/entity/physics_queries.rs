use bbb_protocol::packets::BlockPos;

use super::{
    area_effect_cloud, arrow, boat, egg, end_crystal, ender_pearl, evoker_fangs, experience_bottle,
    experience_orb, eye_of_ender, falling_block, item, lightning_bolt, llama_spit, minecart,
    shulker_bullet, snowball, supporting_block, thrown_potion, tnt, trident, Entity, EntityData,
    MovementEmission, SupportingBlockFacts,
};
use crate::world::simulation_level::LevelCtx;

impl Entity {
    pub fn is_ignoring_block_triggers(&self) -> bool {
        match self.data() {
            EntityData::GeneralLiving(data) => data.entity_resource_id() == "minecraft:bat",
            EntityData::ArmorStand(data) => data.is_marker(),
            EntityData::Marker(_)
            | EntityData::Interaction(_)
            | EntityData::BlockDisplay(_)
            | EntityData::ItemDisplay(_)
            | EntityData::TextDisplay(_)
            | EntityData::OminousItemSpawner(_) => true,
            _ => false,
        }
    }

    pub fn is_pickable(&self) -> bool {
        match self.data() {
            EntityData::Player | EntityData::EnderDragonPart => true,
            EntityData::GeneralLiving(data) => {
                data.entity_resource_id() != "minecraft:ender_dragon"
            }
            EntityData::FallingBlock(_)
            | EntityData::Painting(_)
            | EntityData::ItemFrame(_)
            | EntityData::GlowItemFrame(_)
            | EntityData::LeashKnot(_) => true,
            EntityData::Tnt(_) => self.removal_reason.is_none(),
            EntityData::EndCrystal(_) => true,
            EntityData::LargeFireball(_)
            | EntityData::SmallFireball(_)
            | EntityData::DragonFireball(_)
            | EntityData::WitherSkull(_)
            | EntityData::WindCharge(_)
            | EntityData::BreezeWindCharge(_) => true,
            EntityData::ArmorStand(stand) => !stand.is_marker(),
            EntityData::Interaction(_) => true,
            EntityData::Marker(_)
            | EntityData::BlockDisplay(_)
            | EntityData::ItemDisplay(_)
            | EntityData::TextDisplay(_)
            | EntityData::ExperienceOrb(_)
            | EntityData::Arrow(_)
            | EntityData::SpectralArrow(_)
            | EntityData::Trident(_)
            | EntityData::Snowball(_)
            | EntityData::Egg(_)
            | EntityData::EnderPearl(_)
            | EntityData::EyeOfEnder(_)
            | EntityData::ExperienceBottle(_)
            | EntityData::FireworkRocket(_)
            | EntityData::LlamaSpit(_)
            | EntityData::FishingBobber(_)
            | EntityData::Item(_)
            | EntityData::EvokerFangs(_)
            | EntityData::LightningBolt(_)
            | EntityData::AreaEffectCloud(_)
            | EntityData::OminousItemSpawner(_)
            | EntityData::SplashPotion(_)
            | EntityData::LingeringPotion(_) => false,
            EntityData::ShulkerBullet(_) => true,
            EntityData::Boat(_) | EntityData::ChestBoat(_) => self.removal_reason.is_none(),
            EntityData::Minecart(_)
            | EntityData::FurnaceMinecart(_)
            | EntityData::ChestMinecart(_)
            | EntityData::HopperMinecart(_)
            | EntityData::CommandBlockMinecart(_)
            | EntityData::SpawnerMinecart(_)
            | EntityData::TntMinecart(_) => self.removal_reason.is_none(),
        }
    }

    pub fn blocks_building(&self) -> bool {
        matches!(
            self.data(),
            EntityData::GeneralLiving(_)
                | EntityData::FallingBlock(_)
                | EntityData::Tnt(_)
                | EntityData::EndCrystal(_)
                | EntityData::Boat(_)
                | EntityData::ChestBoat(_)
                | EntityData::Minecart(_)
                | EntityData::FurnaceMinecart(_)
                | EntityData::ChestMinecart(_)
                | EntityData::HopperMinecart(_)
                | EntityData::CommandBlockMinecart(_)
                | EntityData::SpawnerMinecart(_)
                | EntityData::TntMinecart(_)
        ) || matches!(self.data(), EntityData::ArmorStand(data) if !data.is_marker())
    }

    #[must_use]
    pub fn is_persistence_required(&self) -> bool {
        match self.data() {
            EntityData::GeneralLiving(data) => data
                .mob_retained()
                .is_some_and(|retained| retained.persistence_required()),
            _ => false,
        }
    }

    pub fn can_be_collided_with(&self) -> bool {
        match self.data() {
            EntityData::GeneralLiving(data) => {
                let alive = self.is_alive() && data.living().health() > 0.0;
                if let Some(state) = data.happy_ghast_state() {
                    return alive && !data.is_baby() && state.still_timeout > 0;
                }
                alive && data.shulker_state().is_some()
            }
            EntityData::Boat(_) | EntityData::ChestBoat(_) => true,
            _ => false,
        }
    }

    pub fn can_be_hit_by_projectile(&self) -> bool {
        if self.is_projectile_class() {
            return self.projectile_pickable_for_membership(
                crate::entity::level::builtin_registry_tag_membership(
                    "minecraft:entity_type",
                    "minecraft:redirectable_projectile",
                    self.data().entity_resource_id(),
                ),
            );
        }
        self.is_alive() && self.is_pickable() && !matches!(self.data(), EntityData::Interaction(_))
    }

    pub(crate) fn can_be_hit_by_projectile_in_level(&self, level: &LevelCtx<'_, '_>) -> bool {
        if self.is_projectile_class() {
            return self.projectile_pickable_for_membership(level.registry_tag_membership(
                "minecraft:entity_type",
                "minecraft:redirectable_projectile",
                self.data().entity_resource_id(),
            ));
        }
        self.is_alive() && self.is_pickable() && !matches!(self.data(), EntityData::Interaction(_))
    }

    fn projectile_pickable_for_membership(&self, membership: bool) -> bool {
        if !self.is_alive() {
            return false;
        }
        if matches!(self.data(), EntityData::ShulkerBullet(_)) {
            return true;
        }
        if membership {
            match self.data() {
                EntityData::Arrow(data) => !data.in_ground(),
                EntityData::SpectralArrow(data) => !data.in_ground(),
                EntityData::Trident(data) => !data.in_ground(),
                _ => true,
            }
        } else {
            false
        }
    }

    pub fn can_be_hit_by_hurting_projectile(&self) -> bool {
        if !self.can_be_hit_by_projectile() {
            return false;
        }
        match self.data() {
            EntityData::Interaction(_)
            | EntityData::Marker(_)
            | EntityData::BlockDisplay(_)
            | EntityData::ItemDisplay(_)
            | EntityData::TextDisplay(_) => false,
            EntityData::ShulkerBullet(_) => false,
            EntityData::ArmorStand(stand) => stand.has_physics(self.no_gravity()),
            _ => true,
        }
    }

    pub(crate) fn can_be_hit_by_hurting_projectile_in_level(
        &self,
        level: &LevelCtx<'_, '_>,
    ) -> bool {
        if !self.can_be_hit_by_projectile_in_level(level) {
            return false;
        }
        match self.data() {
            EntityData::Interaction(_)
            | EntityData::Marker(_)
            | EntityData::BlockDisplay(_)
            | EntityData::ItemDisplay(_)
            | EntityData::TextDisplay(_)
            | EntityData::ShulkerBullet(_) => false,
            EntityData::ArmorStand(stand) => stand.has_physics(self.no_gravity()),
            _ => true,
        }
    }

    #[must_use]
    pub fn is_projectile_class(&self) -> bool {
        matches!(
            self.data(),
            EntityData::Arrow(_)
                | EntityData::SpectralArrow(_)
                | EntityData::Trident(_)
                | EntityData::Snowball(_)
                | EntityData::Egg(_)
                | EntityData::EnderPearl(_)
                | EntityData::ExperienceBottle(_)
                | EntityData::FireworkRocket(_)
                | EntityData::LlamaSpit(_)
                | EntityData::FishingBobber(_)
                | EntityData::SplashPotion(_)
                | EntityData::LingeringPotion(_)
                | EntityData::LargeFireball(_)
                | EntityData::SmallFireball(_)
                | EntityData::DragonFireball(_)
                | EntityData::WitherSkull(_)
                | EntityData::WindCharge(_)
                | EntityData::BreezeWindCharge(_)
                | EntityData::ShulkerBullet(_)
        )
    }

    pub fn hurting_projectile_acceleration_power(&self) -> Option<f64> {
        match self.data() {
            EntityData::LargeFireball(data) => Some(data.projectile().acceleration_power()),
            EntityData::SmallFireball(data) => Some(data.projectile().acceleration_power()),
            EntityData::DragonFireball(data) => Some(data.projectile().acceleration_power()),
            EntityData::WitherSkull(data) => Some(data.projectile().acceleration_power()),
            EntityData::WindCharge(data) | EntityData::BreezeWindCharge(data) => {
                Some(data.projectile().acceleration_power())
            }
            _ => None,
        }
    }

    pub fn gravity(&self) -> f64 {
        if self.no_gravity {
            return 0.0;
        }
        match &self.data {
            EntityData::Player | EntityData::EnderDragonPart => 0.0,
            EntityData::GeneralLiving(_) => 0.0,
            EntityData::FallingBlock(_) => falling_block::DEFAULT_GRAVITY,
            EntityData::Item(_) => item::DEFAULT_GRAVITY,
            EntityData::ExperienceOrb(_) => experience_orb::DEFAULT_GRAVITY,
            EntityData::Tnt(_) => tnt::DEFAULT_GRAVITY,
            EntityData::Painting(_) | EntityData::ItemFrame(_) | EntityData::GlowItemFrame(_) => {
                0.08
            }
            EntityData::LeashKnot(_) => 0.0,
            EntityData::ArmorStand(data) => data
                .living()
                .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_GRAVITY_ID),
            EntityData::Marker(_)
            | EntityData::Interaction(_)
            | EntityData::BlockDisplay(_)
            | EntityData::ItemDisplay(_)
            | EntityData::TextDisplay(_) => 0.0,
            EntityData::Arrow(_) | EntityData::SpectralArrow(_) => arrow::DEFAULT_GRAVITY,
            EntityData::Trident(_) => trident::DEFAULT_GRAVITY,
            // `Entity.getDefaultGravity()` (`Entity.java`): the hook has no
            // override and applies its own `-0.03` in tick, never `apply_gravity`.
            EntityData::FishingBobber(_) => 0.0,
            EntityData::Snowball(_) => snowball::DEFAULT_GRAVITY,
            EntityData::Egg(_) => egg::DEFAULT_GRAVITY,
            EntityData::EnderPearl(_) => ender_pearl::DEFAULT_GRAVITY,
            EntityData::EndCrystal(_) => end_crystal::DEFAULT_GRAVITY,
            EntityData::EyeOfEnder(_) => eye_of_ender::DEFAULT_GRAVITY,
            EntityData::ExperienceBottle(_) => experience_bottle::DEFAULT_GRAVITY,
            EntityData::FireworkRocket(_) => 0.0,
            EntityData::LlamaSpit(_) => llama_spit::DEFAULT_GRAVITY,
            EntityData::EvokerFangs(_) => evoker_fangs::DEFAULT_GRAVITY,
            EntityData::LightningBolt(_) => lightning_bolt::DEFAULT_GRAVITY,
            EntityData::LargeFireball(_)
            | EntityData::SmallFireball(_)
            | EntityData::DragonFireball(_)
            | EntityData::WitherSkull(_)
            | EntityData::WindCharge(_)
            | EntityData::BreezeWindCharge(_) => 0.0,
            EntityData::AreaEffectCloud(_) => area_effect_cloud::DEFAULT_GRAVITY,
            EntityData::OminousItemSpawner(_) => 0.0,
            EntityData::SplashPotion(_) | EntityData::LingeringPotion(_) => {
                thrown_potion::DEFAULT_GRAVITY
            }
            EntityData::ShulkerBullet(_) => shulker_bullet::DEFAULT_GRAVITY,
            EntityData::Boat(_) | EntityData::ChestBoat(_) => boat::DEFAULT_GRAVITY,
            EntityData::Minecart(_)
            | EntityData::FurnaceMinecart(_)
            | EntityData::ChestMinecart(_)
            | EntityData::HopperMinecart(_)
            | EntityData::CommandBlockMinecart(_)
            | EntityData::SpawnerMinecart(_)
            | EntityData::TntMinecart(_)
                if self.was_touching_water() =>
            {
                minecart::WATER_GRAVITY
            }
            EntityData::Minecart(_)
            | EntityData::FurnaceMinecart(_)
            | EntityData::ChestMinecart(_)
            | EntityData::HopperMinecart(_)
            | EntityData::CommandBlockMinecart(_)
            | EntityData::SpawnerMinecart(_)
            | EntityData::TntMinecart(_) => minecart::DEFAULT_GRAVITY,
        }
    }

    #[must_use]
    pub fn is_supported_by(&self, pos: BlockPos) -> bool {
        self.main_supporting_block_pos == Some(pos)
    }

    fn on_pos(&self, offset: f32) -> BlockPos {
        supporting_block::body_on_pos(
            self.position,
            self.main_supporting_block_pos,
            offset,
            |_| SupportingBlockFacts {
                state_id: -1,
                is_fence: false,
                is_wall: false,
            },
        )
    }

    pub(crate) fn on_pos_legacy_in(&self, level: &LevelCtx<'_, '_>) -> BlockPos {
        supporting_block::body_on_pos(
            self.position,
            self.main_supporting_block_pos,
            0.2_f32,
            |pos| {
                let state_id = level.block_state(pos);
                let block =
                    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state_id)
                        .and_then(bbb_protocol::block_types::vanilla_block_type);
                let tagged = |tag| {
                    block.is_some_and(|block| {
                        level.registry_tag_membership("minecraft:block", tag, block.resource_id)
                    })
                };
                SupportingBlockFacts {
                    state_id,
                    is_fence: tagged("minecraft:fences"),
                    is_wall: tagged("minecraft:walls"),
                }
            },
        )
    }

    pub fn on_pos_legacy(&self) -> BlockPos {
        self.on_pos(0.2)
    }

    #[must_use]
    pub fn get_on_pos(&self) -> BlockPos {
        supporting_block::body_get_on_pos(self.position, self.main_supporting_block_pos)
    }

    #[must_use]
    pub fn is_on_rails(&self) -> bool {
        match &self.data {
            EntityData::Minecart(data) => data.on_rails(),
            EntityData::FurnaceMinecart(data) => data.minecart().on_rails(),
            EntityData::ChestMinecart(data) | EntityData::HopperMinecart(data) => {
                data.minecart().on_rails()
            }
            EntityData::CommandBlockMinecart(data) => data.minecart().on_rails(),
            EntityData::SpawnerMinecart(data) => data.minecart().on_rails(),
            EntityData::TntMinecart(data) => data.minecart().on_rails(),
            _ => false,
        }
    }

    #[must_use]
    pub fn movement_emission(&self) -> MovementEmission {
        match self.data.entity_resource_id() {
            "minecraft:shulker"
            | "minecraft:item"
            | "minecraft:tnt"
            | "minecraft:falling_block"
            | "minecraft:experience_orb"
            | "minecraft:fishing_bobber"
            | "minecraft:arrow"
            | "minecraft:spectral_arrow"
            | "minecraft:trident"
            | "minecraft:end_crystal" => MovementEmission::None,
            "minecraft:squid"
            | "minecraft:glow_squid"
            | "minecraft:endermite"
            | "minecraft:breeze"
            | "minecraft:silverfish"
            | "minecraft:guardian"
            | "minecraft:elder_guardian"
            | "minecraft:bat"
            | "minecraft:oak_boat"
            | "minecraft:spruce_boat"
            | "minecraft:birch_boat"
            | "minecraft:jungle_boat"
            | "minecraft:acacia_boat"
            | "minecraft:cherry_boat"
            | "minecraft:dark_oak_boat"
            | "minecraft:pale_oak_boat"
            | "minecraft:mangrove_boat"
            | "minecraft:bamboo_raft"
            | "minecraft:oak_chest_boat"
            | "minecraft:spruce_chest_boat"
            | "minecraft:birch_chest_boat"
            | "minecraft:jungle_chest_boat"
            | "minecraft:acacia_chest_boat"
            | "minecraft:cherry_chest_boat"
            | "minecraft:dark_oak_chest_boat"
            | "minecraft:pale_oak_chest_boat"
            | "minecraft:mangrove_chest_boat"
            | "minecraft:bamboo_chest_raft"
            | "minecraft:minecart"
            | "minecraft:chest_minecart"
            | "minecraft:furnace_minecart"
            | "minecraft:hopper_minecart"
            | "minecraft:command_block_minecart"
            | "minecraft:spawner_minecart"
            | "minecraft:tnt_minecart" => MovementEmission::Events,
            _ => MovementEmission::All,
        }
    }

    pub fn block_pos_below_that_affects_my_movement(&self) -> BlockPos {
        match &self.data {
            EntityData::Player | EntityData::EnderDragonPart => self.on_pos(0.500001),
            EntityData::GeneralLiving(_) => self.on_pos(0.500001),
            EntityData::FallingBlock(_) => self.on_pos(0.500001),
            EntityData::Item(_) => self.on_pos(item::BLOCK_POS_BELOW_OFFSET),
            EntityData::ExperienceOrb(_) => self.on_pos(experience_orb::BLOCK_POS_BELOW_OFFSET),
            EntityData::Tnt(_) => self.on_pos(0.500001),
            EntityData::Painting(_)
            | EntityData::ItemFrame(_)
            | EntityData::GlowItemFrame(_)
            | EntityData::LeashKnot(_) => self.on_pos(0.500001),
            EntityData::ArmorStand(_) => self.on_pos(0.500001),
            EntityData::Marker(_)
            | EntityData::Interaction(_)
            | EntityData::BlockDisplay(_)
            | EntityData::ItemDisplay(_)
            | EntityData::TextDisplay(_) => self.on_pos(0.500001),
            EntityData::Arrow(_) | EntityData::SpectralArrow(_) | EntityData::Trident(_) => {
                self.on_pos(0.500001)
            }
            EntityData::FishingBobber(_) => self.on_pos(0.500001),
            EntityData::Snowball(_)
            | EntityData::Egg(_)
            | EntityData::EnderPearl(_)
            | EntityData::EndCrystal(_)
            | EntityData::EyeOfEnder(_)
            | EntityData::ExperienceBottle(_) => self.on_pos(0.500001),
            EntityData::FireworkRocket(_) => self.on_pos(0.500001),
            EntityData::LlamaSpit(_) => self.on_pos(0.500001),
            EntityData::EvokerFangs(_) | EntityData::LightningBolt(_) => self.on_pos(0.500001),
            EntityData::LargeFireball(_)
            | EntityData::SmallFireball(_)
            | EntityData::DragonFireball(_)
            | EntityData::WitherSkull(_)
            | EntityData::WindCharge(_)
            | EntityData::BreezeWindCharge(_) => self.on_pos(0.500001),
            EntityData::AreaEffectCloud(_) => self.on_pos(0.500001),
            EntityData::OminousItemSpawner(_) => self.on_pos(0.500001),
            EntityData::SplashPotion(_) | EntityData::LingeringPotion(_) => self.on_pos(0.500001),
            EntityData::ShulkerBullet(_) => self.on_pos(0.500001),
            EntityData::Boat(_) | EntityData::ChestBoat(_) => self.on_pos(0.500001),
            EntityData::Minecart(_)
            | EntityData::FurnaceMinecart(_)
            | EntityData::ChestMinecart(_)
            | EntityData::HopperMinecart(_)
            | EntityData::CommandBlockMinecart(_)
            | EntityData::SpawnerMinecart(_)
            | EntityData::TntMinecart(_) => self.on_pos(0.500001),
        }
    }
}
