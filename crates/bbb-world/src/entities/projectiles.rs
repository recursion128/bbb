use bbb_protocol::packets::ProjectilePower as ProtocolProjectilePower;

use crate::{HurtingProjectileState, ProjectilePowerUpdateState, WorldStore};

use super::{
    EntityHurtingProjectile, VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
    VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID, VANILLA_ENTITY_TYPE_FIREBALL_ID,
    VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
    VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
};

const DEFAULT_HURTING_PROJECTILE_ACCELERATION_POWER: f64 = 0.1;
const DEFAULT_WIND_CHARGE_ACCELERATION_POWER: f64 = 0.0;

impl WorldStore {
    pub fn apply_projectile_power(&mut self, packet: ProtocolProjectilePower) -> bool {
        self.counters.projectile_power_packets += 1;
        let applied = self
            .entities
            .with_hurting_projectile_mut(packet.entity_id, |projectile| {
                projectile.acceleration_power = packet.acceleration_power;
            })
            .is_some();

        if applied {
            self.counters.projectile_power_updates_applied += 1;
        } else {
            self.counters.projectile_power_updates_ignored += 1;
        }

        self.last_projectile_power = Some(ProjectilePowerUpdateState {
            entity_id: packet.entity_id,
            acceleration_power: packet.acceleration_power,
            applied,
        });
        applied
    }
}

pub(crate) fn initial_hurting_projectile_state(
    entity_type_id: i32,
) -> Option<HurtingProjectileState> {
    if !is_vanilla_hurting_projectile_type(entity_type_id) {
        return None;
    }
    let acceleration_power = if is_vanilla_wind_charge_type(entity_type_id) {
        DEFAULT_WIND_CHARGE_ACCELERATION_POWER
    } else {
        DEFAULT_HURTING_PROJECTILE_ACCELERATION_POWER
    };
    Some(HurtingProjectileState { acceleration_power })
}

pub(crate) fn entity_hurting_projectile_from_state(
    entity_type_id: i32,
    state: Option<HurtingProjectileState>,
) -> Option<EntityHurtingProjectile> {
    if !is_vanilla_hurting_projectile_type(entity_type_id) {
        return None;
    }
    state
        .or_else(|| initial_hurting_projectile_state(entity_type_id))
        .map(EntityHurtingProjectile::from)
}

fn is_vanilla_hurting_projectile_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID
            | VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID
            | VANILLA_ENTITY_TYPE_FIREBALL_ID
            | VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID
            | VANILLA_ENTITY_TYPE_WIND_CHARGE_ID
            | VANILLA_ENTITY_TYPE_WITHER_SKULL_ID
    )
}

fn is_vanilla_wind_charge_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID | VANILLA_ENTITY_TYPE_WIND_CHARGE_ID
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorldStore;
    use bbb_protocol::packets::{AddEntity as ProtocolAddEntity, Vec3d as ProtocolVec3d};
    use uuid::Uuid;

    #[test]
    fn applies_projectile_power_to_hurting_projectiles_only() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(
            10,
            VANILLA_ENTITY_TYPE_FIREBALL_ID,
        ));
        store.apply_add_entity(protocol_add_entity_with_type(20, 7));

        assert_eq!(
            store.hurting_projectile(10),
            Some(HurtingProjectileState {
                acceleration_power: DEFAULT_HURTING_PROJECTILE_ACCELERATION_POWER,
            })
        );
        assert_eq!(store.hurting_projectile(20), None);

        assert!(store.apply_projectile_power(ProtocolProjectilePower {
            entity_id: 10,
            acceleration_power: 0.75,
        }));
        assert_eq!(
            store.hurting_projectile(10),
            Some(HurtingProjectileState {
                acceleration_power: 0.75,
            })
        );
        assert_eq!(
            store
                .probe_entity(10)
                .and_then(|entity| entity.hurting_projectile),
            Some(HurtingProjectileState {
                acceleration_power: 0.75,
            })
        );
        assert_eq!(
            store.clone().hurting_projectile(10),
            Some(HurtingProjectileState {
                acceleration_power: 0.75,
            })
        );

        assert!(!store.apply_projectile_power(ProtocolProjectilePower {
            entity_id: 20,
            acceleration_power: 0.25,
        }));
        assert!(!store.apply_projectile_power(ProtocolProjectilePower {
            entity_id: 404,
            acceleration_power: 0.5,
        }));

        let counters = store.counters();
        assert_eq!(counters.projectile_power_packets, 3);
        assert_eq!(counters.projectile_power_updates_applied, 1);
        assert_eq!(counters.projectile_power_updates_ignored, 2);
        assert_eq!(
            store.last_projectile_power_update(),
            Some(&ProjectilePowerUpdateState {
                entity_id: 404,
                acceleration_power: 0.5,
                applied: false,
            })
        );
    }

    #[test]
    fn wind_charge_projectiles_start_without_acceleration() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(
            10,
            VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        ));

        assert_eq!(
            store.hurting_projectile(10),
            Some(HurtingProjectileState {
                acceleration_power: DEFAULT_WIND_CHARGE_ACCELERATION_POWER,
            })
        );
    }

    fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }
}
