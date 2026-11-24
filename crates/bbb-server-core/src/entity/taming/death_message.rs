//! Combat history needed by an owned tamable's death notification.

use bbb_protocol::{NbtSummaryValue, RichTextComponent};
use bbb_server_datapack::damage::{
    CombatEntry, CombatTracker, DamageTypeFacts, FallLocation, HurtSource,
};

use crate::entity::general_living::TamableRetainedState;
use crate::entity::{Entity, EntityCommand, EntityData, EntityStore, StoredEntityDamageSource};
use crate::world::simulation_level::LevelCtx;

#[derive(Debug, Clone, Default, PartialEq)]
pub(in crate::entity) struct PetCombat {
    tracker: CombatTracker,
    source: Option<(
        DamageTypeFacts,
        Option<NbtSummaryValue>,
        Option<NbtSummaryValue>,
    )>,
}

impl Entity {
    pub(crate) fn record_pet_death_damage(
        &mut self,
        entry: CombatEntry,
        tick_count: i32,
        enters_combat: bool,
        damage_type: DamageTypeFacts,
        causing: Option<NbtSummaryValue>,
        direct: Option<NbtSummaryValue>,
    ) {
        let EntityData::GeneralLiving(data) = &mut self.data else {
            return;
        };
        let Some(tamable) = data
            .tamable_retained_mut()
            .filter(|state| state.owner().is_some())
        else {
            return;
        };
        let combat = tamable.death_combat.get_or_insert_with(Default::default);
        combat
            .tracker
            .record_damage(entry, tick_count, enters_combat);
        combat.source = Some((damage_type, causing, direct));
    }
}

impl TamableRetainedState {
    pub(crate) fn death_message(
        &self,
        tick_count: i32,
        death_source: &DamageTypeFacts,
        victim: NbtSummaryValue,
        kill_credit: Option<NbtSummaryValue>,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> RichTextComponent {
        let mut tracker = self
            .death_combat
            .as_ref()
            .map_or_else(CombatTracker::default, |combat| combat.tracker.clone());
        tracker.recheck_status(tick_count, true);
        let source = self
            .death_combat
            .as_ref()
            .and_then(|combat| combat.source.as_ref());
        let (damage_type, causing, direct) = source.map_or(
            (death_source, None, None),
            |(damage_type, causing, direct)| (damage_type, causing.clone(), direct.clone()),
        );
        bbb_server_datapack::damage::death_message_with_registry(
            &tracker,
            &HurtSource {
                damage_type,
                causing_player: None,
                direct_player: None,
                source_position: None,
                arrow_owner_player: false,
                team_refuses_harm: false,
            },
            victim,
            kill_credit,
            causing,
            direct,
            content_registry,
        )
    }
}

#[derive(Clone, Copy)]
pub(in crate::entity) struct PetDamageSourceSnapshots<'a> {
    pub(in crate::entity) causing: Option<&'a crate::explosion::ExplosionSourceEntity>,
    pub(in crate::entity) direct: Option<&'a crate::explosion::ExplosionSourceEntity>,
}

pub(in crate::entity) fn record_damage(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
    snapshots: PetDamageSourceSnapshots<'_>,
    damage: f32,
    enters_combat: bool,
) {
    let PetDamageSourceSnapshots {
        causing: causing_entity_snapshot,
        direct: direct_entity_snapshot,
    } = snapshots;
    if !has_owner(entity) {
        return;
    }
    // `FallLocation.getCurrentFallLocation` (FallLocation.java:35-43).
    let fall_location = match entity
        .living()
        .and_then(|living| living.last_climbable_pos())
    {
        Some(pos) => {
            let state = level.block_state(pos);
            bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
                .and_then(bbb_protocol::block_types::vanilla_block_type)
                .map(|block| {
                    FallLocation::of_climbable_block(
                        block.resource_id,
                        level.block_state_in_tag(state, "minecraft:trapdoors"),
                    )
                })
        }
        None => entity.was_touching_water().then_some(FallLocation::Water),
    };
    store.queue_command(EntityCommand::PetDamage {
        entity_id: entity.id(),
        tick_count: entity.tick_count(),
        enters_combat,
        entry: CombatEntry {
            is_fall: level.registry_tag_membership(
                "minecraft:damage_type",
                "minecraft:is_fall",
                &source.damage_type.resource_id,
            ),
            always_most_significant_fall: level.registry_tag_membership(
                "minecraft:damage_type",
                "minecraft:always_most_significant_fall",
                &source.damage_type.resource_id,
            ),
            attacker: None,
            attacker_weapon: None,
            damage,
            fall_location,
            fall_distance: entity.fall_distance() as f32,
        },
        damage_type: source.damage_type.clone(),
        causing_entity: source.causing_entity,
        direct_entity_id: source.direct_entity_id,
        causing_entity_snapshot: causing_entity_snapshot.cloned(),
        direct_entity_snapshot: direct_entity_snapshot.cloned(),
    });
}

pub(in crate::entity) fn queue_death(
    entity: &Entity,
    store: &mut EntityStore,
    source: StoredEntityDamageSource<'_>,
) {
    if has_owner(entity) {
        store.queue_command(EntityCommand::PetDeath {
            entity_id: entity.id(),
            damage_type: source.damage_type.clone(),
        });
    }
}

fn has_owner(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.tamable_retained().is_some_and(|state| state.owner().is_some()))
}
