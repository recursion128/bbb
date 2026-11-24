use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_HUNGER_ID, VANILLA_MOB_EFFECT_POISON_ID, VANILLA_MOB_EFFECT_REGENERATION_ID,
    VANILLA_MOB_EFFECT_WITHER_ID,
};

use crate::entities::LocalPlayerVehicleHealth;
use crate::WorldStore;

/// Vanilla `FoodData`'s construction defaults (FoodData.java:14-15). A remote
/// player's food data is never synchronized, so `Gui.extractFood` draws these
/// for a camera set to another player.
const VANILLA_FOOD_DATA_DEFAULT_FOOD_LEVEL: i32 = 20;
const VANILLA_FOOD_DATA_DEFAULT_SATURATION_LEVEL: f32 = 5.0;

/// Everything `Gui` reads off `getCameraPlayer()` (Gui.java:705-707) to draw
/// the vitals rows: `extractPlayerHealth`, `extractArmor`, `extractHearts`,
/// `extractFood`, `extractAirBubbles` and `getPlayerVehicleWithHealth`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudCameraPlayerVitals {
    pub entity_id: Option<i32>,
    pub is_local_player: bool,
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
    pub max_health: f32,
    pub absorption: f32,
    pub armor: i32,
    pub hungry: bool,
    pub poisoned: bool,
    pub withered: bool,
    pub fully_frozen: bool,
    pub regenerating: bool,
    pub invulnerable_time: i32,
    pub air: i32,
    pub max_air: i32,
    pub eye_in_water: bool,
    pub vehicle: Option<LocalPlayerVehicleHealth>,
}

impl WorldStore {
    pub fn hud_camera_player_vitals(&self) -> Option<HudCameraPlayerVitals> {
        if self.hud_camera_player_is_local_player() {
            return self.local_camera_player_vitals();
        }
        self.remote_camera_player_vitals(self.hud_camera_player_id()?)
    }

    fn local_camera_player_vitals(&self) -> Option<HudCameraPlayerVitals> {
        let entity_id = self.local_player_id();
        let health = self.local_player().health?;
        Some(HudCameraPlayerVitals {
            entity_id,
            is_local_player: true,
            health: health.health,
            food: health.food,
            saturation: health.saturation,
            max_health: self.local_player_max_health(),
            absorption: self.local_player_absorption(),
            armor: self.local_player_armor_value(),
            hungry: self.local_player_has_effect(VANILLA_MOB_EFFECT_HUNGER_ID),
            poisoned: self.local_player_has_effect(VANILLA_MOB_EFFECT_POISON_ID),
            withered: self.local_player_has_effect(VANILLA_MOB_EFFECT_WITHER_ID),
            fully_frozen: self.local_player_is_fully_frozen(),
            regenerating: self.local_player_has_effect(VANILLA_MOB_EFFECT_REGENERATION_ID),
            invulnerable_time: entity_id
                .and_then(|id| self.entity_damage_invulnerable_time(id))
                .unwrap_or(0),
            air: self.local_player_air_supply(),
            max_air: self.local_player_max_air_supply(),
            eye_in_water: self.local_player_eye_in_water(),
            vehicle: self.local_player_vehicle_health(),
        })
    }

    fn remote_camera_player_vitals(&self, entity_id: i32) -> Option<HudCameraPlayerVitals> {
        Some(HudCameraPlayerVitals {
            entity_id: Some(entity_id),
            is_local_player: false,
            health: self.player_synced_health(entity_id)?,
            food: VANILLA_FOOD_DATA_DEFAULT_FOOD_LEVEL,
            saturation: VANILLA_FOOD_DATA_DEFAULT_SATURATION_LEVEL,
            max_health: self.player_max_health(entity_id),
            absorption: self.player_absorption_amount(entity_id),
            armor: self.player_armor_value(entity_id),
            hungry: self
                .entity_effect(entity_id, VANILLA_MOB_EFFECT_HUNGER_ID)
                .is_some(),
            poisoned: self
                .entity_effect(entity_id, VANILLA_MOB_EFFECT_POISON_ID)
                .is_some(),
            withered: self
                .entity_effect(entity_id, VANILLA_MOB_EFFECT_WITHER_ID)
                .is_some(),
            fully_frozen: self.player_is_fully_frozen(entity_id),
            regenerating: self
                .entity_effect(entity_id, VANILLA_MOB_EFFECT_REGENERATION_ID)
                .is_some(),
            invulnerable_time: self.entity_damage_invulnerable_time(entity_id).unwrap_or(0),
            air: self.player_air_supply(entity_id),
            max_air: self.local_player_max_air_supply(),
            eye_in_water: self.entity_eye_in_water(entity_id),
            vehicle: self.player_vehicle_health(entity_id),
        })
    }

    fn local_player_has_effect(&self, effect_id: i32) -> bool {
        self.local_player_id()
            .and_then(|id| self.entity_effect(id, effect_id))
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::attribute_types::{
        VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
    };
    use bbb_protocol::entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
    };
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
    use bbb_protocol::packets::{
        AddEntity, AttributeSnapshot, EntityDataValue, EntityDataValueKind, PlayerHealth,
        SetCamera, SetEntityData, UpdateAttributes, Vec3d,
    };
    use uuid::Uuid;

    use super::*;

    const LOCAL_ID: i32 = 7;
    const REMOTE_ID: i32 = 55;

    fn player_entity(id: i32) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn set_entity_data(
        world: &mut WorldStore,
        id: i32,
        data_id: u8,
        serializer_id: i32,
        value: EntityDataValueKind,
    ) {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![EntityDataValue {
                data_id,
                serializer_id,
                value,
            }],
        }));
    }

    fn two_player_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.local_player_id = Some(LOCAL_ID);
        world.apply_add_entity(player_entity(LOCAL_ID));
        world.apply_add_entity(player_entity(REMOTE_ID));
        world.apply_player_health(PlayerHealth {
            health: 6.0,
            food: 3,
            saturation: 0.0,
        });
        world
    }

    #[test]
    fn the_camera_player_defaults_to_the_local_player_until_set_camera_retargets_it() {
        let mut world = two_player_world();
        assert_eq!(world.camera_entity_id(), Some(LOCAL_ID));
        assert_eq!(world.hud_camera_player_id(), Some(LOCAL_ID));
        assert!(world.hud_camera_player_is_local_player());

        assert!(world.apply_set_camera(SetCamera {
            camera_id: REMOTE_ID
        }));
        assert_eq!(world.camera_entity_id(), Some(REMOTE_ID));
        assert_eq!(world.hud_camera_player_id(), Some(REMOTE_ID));
        assert!(!world.hud_camera_player_is_local_player());
    }

    #[test]
    fn a_remote_camera_player_projects_its_own_synced_vitals_and_vanilla_food_defaults() {
        let mut world = two_player_world();
        let living =
            crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
            )
            .expect("generated LivingEntity accessors");
        let (absorption_data_id, _) =
            crate::entities::vanilla_entity_metadata_player_absorption_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
            )
            .expect("generated Player absorption accessor");
        let entity_accessors = crate::vanilla_entity_metadata_entity_accessors()
            .expect("generated base Entity accessors");

        set_entity_data(
            &mut world,
            REMOTE_ID,
            living.health_data_id,
            VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            EntityDataValueKind::Float(13.0),
        );
        set_entity_data(
            &mut world,
            REMOTE_ID,
            absorption_data_id,
            VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            EntityDataValueKind::Float(4.0),
        );
        set_entity_data(
            &mut world,
            REMOTE_ID,
            entity_accessors.air_supply_data_id,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(120),
        );
        assert!(world.apply_update_attributes(UpdateAttributes {
            entity_id: REMOTE_ID,
            attributes: vec![
                AttributeSnapshot {
                    attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                    base: 24.0,
                    modifiers: Vec::new(),
                },
                AttributeSnapshot {
                    attribute_id: VANILLA_ATTRIBUTE_ARMOR_ID,
                    base: 9.0,
                    modifiers: Vec::new(),
                },
            ],
        }));

        let local = world.hud_camera_player_vitals().expect("local vitals");
        assert!(local.is_local_player);
        assert_eq!(local.health, 6.0);
        assert_eq!(local.food, 3);
        assert_eq!(local.saturation, 0.0);
        assert_eq!(local.air, 300);

        assert!(world.apply_set_camera(SetCamera {
            camera_id: REMOTE_ID
        }));
        let remote = world.hud_camera_player_vitals().expect("remote vitals");
        assert!(!remote.is_local_player);
        assert_eq!(remote.entity_id, Some(REMOTE_ID));
        assert_eq!(remote.health, 13.0);
        assert_eq!(remote.max_health, 24.0);
        assert_eq!(remote.absorption, 4.0);
        assert_eq!(remote.armor, 9);
        assert_eq!(remote.air, 120);
        assert_eq!(remote.max_air, 300);
        assert_eq!(remote.food, VANILLA_FOOD_DATA_DEFAULT_FOOD_LEVEL);
        assert_eq!(
            remote.saturation,
            VANILLA_FOOD_DATA_DEFAULT_SATURATION_LEVEL
        );
        assert!(remote.vehicle.is_none());
        assert!(!remote.eye_in_water);
    }

    #[test]
    fn a_non_player_camera_entity_leaves_the_vitals_unresolved() {
        let mut world = two_player_world();
        world.apply_add_entity(AddEntity {
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            ..player_entity(90)
        });
        assert!(world.apply_set_camera(SetCamera { camera_id: 90 }));
        assert_eq!(world.camera_entity_id(), Some(90));
        assert_eq!(world.hud_camera_player_id(), None);
        assert!(world.hud_camera_player_vitals().is_none());
    }
}
