use bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS;
use bbb_protocol::packets::Vec3d;
use bbb_protocol::RichTextComponent;

use crate::entity::living::{ActiveMobEffect, EntityEquipment};
use crate::entity::{Entity, EntityData};

#[derive(Clone)]
pub(crate) struct CommonMobConversionSnapshot {
    absorption: f32,
    effects: Vec<ActiveMobEffect>,
    left_handed: bool,
    no_ai: bool,
    persistence_required: bool,
    can_pick_up_loot: Option<bool>,
    custom_name_visible: bool,
    on_fire: bool,
    invulnerable: bool,
    no_gravity: bool,
    portal_cooldown: i32,
    silent: bool,
    custom_name: Option<RichTextComponent>,
    zombie_phenotype: Option<(bool, bool)>,
    baby: bool,
    age: Option<(i32, i32)>,
    tags: Vec<bbb_protocol::JavaString>,
    custom_data: Vec<bbb_protocol::NbtSummaryEntry>,
}

impl CommonMobConversionSnapshot {
    // `ConversionType.convertCommon` (`ConversionType.java:94-137`).
    pub(crate) fn capture(source: &Entity, preserve_can_pick_up_loot: bool) -> Self {
        let living = source.living().expect("Mob conversion source is living");
        let EntityData::GeneralLiving(data) = source.data() else {
            unreachable!("Mob conversion source uses GeneralLivingData")
        };
        let mob = data
            .mob_retained()
            .expect("Mob conversion source has Mob state");
        Self {
            absorption: living.absorption_amount(),
            effects: living.active_effects().cloned().collect(),
            left_handed: data.is_left_handed(),
            no_ai: data.is_no_ai(),
            persistence_required: mob.persistence_required(),
            can_pick_up_loot: preserve_can_pick_up_loot.then(|| mob.can_pick_up_loot()),
            custom_name_visible: source.custom_name_visible(),
            on_fire: source.is_on_fire(),
            invulnerable: source.invulnerable(),
            no_gravity: source.no_gravity(),
            portal_cooldown: source.portal_cooldown(),
            silent: source.is_silent(),
            custom_name: source.custom_name().cloned(),
            zombie_phenotype: data
                .zombie_retained()
                .map(|state| (state.is_baby(), state.can_break_doors())),
            baby: data.is_baby(),
            age: data
                .ageable_retained()
                .map(|age| (age.age(), age.forced_age())),
            tags: source.entity_tags().to_vec(),
            custom_data: source.custom_data().to_vec(),
        }
    }

    pub(crate) fn apply(&self, target: &mut Entity) {
        let living = target
            .living_mut()
            .expect("Mob conversion target is living");
        living.set_absorption_amount(self.absorption);
        living.replace_active_effects(self.effects.iter().cloned());
        let zombie_to_zombie = self
            .zombie_phenotype
            .is_some_and(|(baby, can_break_doors)| {
                crate::entity::zombie::apply_conversion_phenotype(target, baby, can_break_doors)
            });
        if !zombie_to_zombie
            && self.baby
            && !crate::entity::zombie::set_baby(target, true)
            && !crate::entity::general_living::set_zoglin_baby(target, true)
        {
            // `AgeableMob.setBaby` (`AgeableMob.java:246-248`).
            let _ = crate::entity::set_spawner_age(target, -24_000);
        }
        let EntityData::GeneralLiving(data) = &mut target.data else {
            unreachable!("Mob conversion target uses GeneralLivingData")
        };
        if let (Some((age, forced_age)), Some(target_age)) = (self.age, data.ageable_retained_mut())
        {
            *target_age = crate::entity::general_living::AgeableMobRetainedState::from_saved(
                age, forced_age, false,
            );
        }
        let mob = data
            .mob
            .as_mut()
            .expect("Mob conversion target has Mob state");
        mob.set_left_handed(self.left_handed);
        mob.set_no_ai(self.no_ai);
        mob.set_persistence_required(self.persistence_required);
        if let Some(can_pick_up_loot) = self.can_pick_up_loot {
            mob.set_can_pick_up_loot(can_pick_up_loot);
        }
        target.set_custom_name_visible(self.custom_name_visible);
        target.set_shared_flag_on_fire(self.on_fire);
        target.set_invulnerable(self.invulnerable);
        target.set_no_gravity(self.no_gravity);
        target.portal_cooldown = self.portal_cooldown;
        target.set_silent(self.silent);
        target.set_custom_name(self.custom_name.clone());
        target.set_entity_tags(self.tags.clone());
        target.load_custom_data(self.custom_data.clone());
    }
}

pub(crate) struct SingleMobConversionSnapshot {
    common: CommonMobConversionSnapshot,
    position: Vec3d,
    delta: Vec3d,
    y_rot: f32,
    x_rot: f32,
    equipment: EntityEquipment,
    drop_chances: Vec<(bbb_protocol::equipment_slots::EquipmentSlot, f32)>,
    fall_distance: f64,
    fall_flying: bool,
    last_hurt_by_player_memory_time: i32,
    hurt_time: i32,
    y_body_rot: f32,
    on_ground: bool,
    sleeping_pos: Option<bbb_protocol::packets::BlockPos>,
}

impl SingleMobConversionSnapshot {
    // `ConversionType.SINGLE` copies equipment and motion before common Mob state
    // (`ConversionType.java:15-62`).
    pub(crate) fn capture(source: &Entity) -> Self {
        Self::capture_with(source, true, true)
    }

    /// `ConversionParams.keepEquipment` and `preserveCanPickUpLoot`.
    pub(crate) fn capture_with(
        source: &Entity,
        keep_equipment: bool,
        preserve_can_pick_up_loot: bool,
    ) -> Self {
        let living = source.living().expect("SINGLE conversion source is living");
        let mob = match source.data() {
            EntityData::GeneralLiving(data) => data
                .mob_retained()
                .expect("SINGLE conversion source has Mob state"),
            _ => unreachable!("SINGLE conversion source uses GeneralLivingData"),
        };
        let equipment = if keep_equipment {
            living.equipment().clone()
        } else {
            EntityEquipment::default()
        };
        let drop_chances = VANILLA_EQUIPMENT_SLOTS
            .iter()
            .filter(|slot| equipment.get(slot.slot).is_some())
            .map(|slot| (slot.slot, mob.drop_chance(slot.slot)))
            .collect();
        Self {
            common: CommonMobConversionSnapshot::capture(source, preserve_can_pick_up_loot),
            position: source.position(),
            delta: source.delta_movement(),
            y_rot: source.y_rot(),
            x_rot: source.x_rot(),
            equipment,
            drop_chances,
            fall_distance: source.fall_distance(),
            fall_flying: source.is_fall_flying(),
            last_hurt_by_player_memory_time: living.last_hurt_by_player_memory_time,
            hurt_time: living.hurt_time,
            y_body_rot: living.y_body_rot,
            on_ground: source.on_ground(),
            sleeping_pos: source.sleeping_pos(),
        }
    }

    pub(crate) const fn position(&self) -> Vec3d {
        self.position
    }

    pub(crate) fn apply(&self, target: &mut Entity) {
        target.set_rot(self.y_rot, self.x_rot);
        target.set_delta_movement(self.delta);
        target
            .living_mut()
            .expect("SINGLE conversion target is living")
            .set_all_equipment(&self.equipment);
        let EntityData::GeneralLiving(data) = &mut target.data else {
            unreachable!("SINGLE conversion target uses GeneralLivingData")
        };
        let mob = data
            .mob
            .as_mut()
            .expect("SINGLE conversion target has Mob state");
        for (slot, chance) in &self.drop_chances {
            let _ = mob.set_drop_chance(*slot, *chance);
        }
        target.fall_distance = self.fall_distance;
        target.set_fall_flying(self.fall_flying);
        let living = target
            .living_mut()
            .expect("SINGLE conversion target is living");
        living.last_hurt_by_player_memory_time = self.last_hurt_by_player_memory_time;
        living.hurt_time = self.hurt_time;
        living.y_body_rot = self.y_body_rot;
        target.on_ground = self.on_ground;
        target.set_sleeping_pos(self.sleeping_pos);
        self.common.apply(target);
    }
}

/// `Mob.setPersistenceRequired` (`Mob.java:1068-1070`).
pub(crate) fn set_persistence_required(entity: &mut Entity) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(mob) = data.mob.as_mut() {
            mob.set_persistence_required(true);
        }
    }
}

/// `ConversionType.SINGLE` moves the first passenger and the vehicle (`ConversionType.java:17-35`).
pub(crate) fn transfer_single_relations(
    store: &mut crate::entity::EntityStore,
    source_id: i32,
    target_id: i32,
) {
    let source_uuid = store.get(source_id).map(Entity::uuid);
    transfer_single_relations_with_identity(store, source_id, source_uuid, target_id);
    if let Some(mut source) = store.detach(source_id) {
        crate::entity::leash::transfer_conversion_leash(&mut source, store, target_id);
        store.attach(source);
    }
}

fn transfer_single_relations_with_identity(
    store: &mut crate::entity::EntityStore,
    source_id: i32,
    source_uuid: Option<uuid::Uuid>,
    target_id: i32,
) {
    if let Some(passenger) = store.passengers(source_id).first().copied() {
        let external_player = store.is_external_player_passenger(passenger);
        store.stop_invalid_riding(passenger);
        if let Some(body) = store.get_mut(passenger) {
            body.set_boarding_cooldown(0);
        }
        if external_player {
            store.queue_command(crate::entity::EntityCommand::StartExternalPlayerRide {
                player_entity_id: passenger,
                vehicle_id: target_id,
            });
        } else {
            store.link_existing_passenger(target_id, passenger);
        }
    }
    if let Some(vehicle) = store.vehicle(source_id) {
        store.stop_invalid_riding(source_id);
        store.link_existing_passenger(vehicle, target_id);
    }
    if let (Some(source), Some(target)) = (source_uuid, store.get(target_id)) {
        store.queue_command(crate::entity::EntityCommand::ConversionTeam {
            source,
            target: target.uuid(),
        });
    }
}

/// SINGLE conversion while its source body is detached for ticking.
pub(crate) fn transfer_single_relations_from(
    source: &mut Entity,
    store: &mut crate::entity::EntityStore,
    target_id: i32,
) {
    transfer_single_relations_with_identity(store, source.id(), Some(source.uuid()), target_id);
    crate::entity::leash::transfer_conversion_leash(source, store, target_id);
}
