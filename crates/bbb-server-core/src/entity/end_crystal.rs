//! Server-owned `EndCrystal` state and lifecycle hooks.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID, VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_END_CRYSTAL_ID;
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::damage::{
    removed_or_invulnerable_to_damage, StoredEntityDamageRules, StoredEntityDamageSource,
    StoredEntityHurtOutcome,
};
use crate::entity::living::SynchedValue;
use crate::entity::store::EntityStore;
use crate::entity::{handle_portal, Entity, EntityData, RemovalReason};
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    PendingExplosion,
};

pub const DEFAULT_GRAVITY: f64 = 0.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndCrystalData {
    beam_target: Option<BlockPos>,
    show_bottom: bool,
    time: i32,
    /// The player behind the damage that destroyed it (`EnderDragon.java:802-808`).
    destroyed_by: Option<uuid::Uuid>,
}

impl EndCrystalData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.time = previous.time;
    }

    pub const fn new(time: i32, show_bottom: bool) -> Self {
        Self {
            beam_target: None,
            show_bottom,
            time,
            destroyed_by: None,
        }
    }

    pub const fn beam_target(&self) -> Option<BlockPos> {
        self.beam_target
    }

    pub const fn shows_bottom(&self) -> bool {
        self.show_bottom
    }

    pub const fn time(&self) -> i32 {
        self.time
    }
}

pub fn spawn_end_crystal(
    store: &mut EntityStore,
    position: Vec3d,
    show_bottom: bool,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let time = store.random().next_int(100_000);
    store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        position,
        EntityData::EndCrystal(EndCrystalData::new(time, show_bottom)),
    )
}

pub fn set_beam_target(entity: &mut Entity, target: Option<BlockPos>) {
    let EntityData::EndCrystal(data) = &mut entity.data else {
        return;
    };
    if data.beam_target != target {
        data.beam_target = target;
        entity.mark_synched_dirty(VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID);
    }
}

pub fn set_show_bottom(entity: &mut Entity, show_bottom: bool) {
    let EntityData::EndCrystal(data) = &mut entity.data else {
        return;
    };
    if data.show_bottom != show_bottom {
        data.show_bottom = show_bottom;
        entity.mark_synched_dirty(VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID);
    }
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    matches!(tick_inner(entity, store, level, false), Ok(None))
}

pub(crate) fn tick_pausing(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Result<Option<crate::entity::inside_blocks::NonlivingInsidePause>, ()> {
    tick_inner(entity, store, level, true)
}

fn tick_inner(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    world_callbacks: bool,
) -> Result<Option<crate::entity::inside_blocks::NonlivingInsidePause>, ()> {
    let EntityData::EndCrystal(data) = &mut entity.data else {
        return Ok(None);
    };
    data.time = data.time.wrapping_add(1);
    if world_callbacks {
        let movements = entity.take_effect_movements();
        if let Some(inside) = crate::entity::inside_blocks::start_nonliving_inside_pausing(
            entity, store, level, movements,
        )? {
            return Ok(Some(inside));
        }
    } else if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(store)) {
        return Err(());
    }
    handle_portal(entity, level).then_some(None).ok_or(())
}

pub(crate) fn destroyed_by(entity: &Entity) -> Option<uuid::Uuid> {
    match entity.data() {
        EntityData::EndCrystal(data) => data.destroyed_by,
        _ => None,
    }
}

pub fn synched_values(entity: &Entity, data: &EndCrystalData) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    rows.extend([
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                value: EntityDataValueKind::OptionalBlockPos(data.beam_target()),
            },
            is_default: data.beam_target().is_none(),
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(data.shows_bottom()),
            },
            is_default: data.shows_bottom(),
        },
    ]);
    rows
}

pub fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, data: &EndCrystalData) {
    if let Some(target) = data.beam_target() {
        entries.push(NbtSummaryEntry {
            name: "beam_target".into(),
            value: NbtSummaryValue::IntArray(vec![target.x, target.y, target.z]),
        });
    }
    entries.push(NbtSummaryEntry {
        name: "ShowBottom".into(),
        value: NbtSummaryValue::Byte(i8::from(data.shows_bottom())),
    });
}

pub fn read_save_data(entries: &[NbtSummaryEntry], time: i32) -> EndCrystalData {
    EndCrystalData {
        beam_target: crate::entity::save::optional_block_pos(entries, "beam_target"),
        show_bottom: crate::entity::display::save_fields::bool_or(entries, "ShowBottom", true),
        time,
        destroyed_by: None,
    }
}

pub struct HurtDecision {
    pub outcome: StoredEntityHurtOutcome,
    pub pending_explosion: Option<PendingExplosion>,
}

pub fn hurt_decision(
    entity: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
) -> HurtDecision {
    hurt_decision_with_causing_type(entity, source, rules, false, None)
}

pub(crate) fn hurt_decision_with_causing_type(
    entity: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    causing_is_ender_dragon: bool,
    causing_entity_snapshot: Option<crate::explosion::ExplosionSourceEntity>,
) -> HurtDecision {
    if removed_or_invulnerable_to_damage(entity, source)
        || source.damage_type.is_fire
        || source.damage_type.is_fall && rules.fall_damage_immune
    {
        return HurtDecision {
            outcome: StoredEntityHurtOutcome {
                applied: false,
                succeeded: false,
            },
            pending_explosion: None,
        };
    }

    if causing_is_ender_dragon {
        return HurtDecision {
            outcome: StoredEntityHurtOutcome {
                applied: false,
                succeeded: false,
            },
            pending_explosion: None,
        };
    }

    let direct = crate::entity::damage::explosion_source_entity(entity);
    if let EntityData::EndCrystal(data) = &mut entity.data {
        data.destroyed_by = match source.causing_entity {
            Some(crate::entity::StoredDamageCausingEntity::Player { uuid, .. }) => Some(uuid),
            _ => None,
        };
    }
    entity.set_removed(RemovalReason::Killed);
    let pending_explosion = (!source.damage_type.is_explosion).then(|| PendingExplosion {
        source: Some(ExplosionSource {
            direct,
            indirect: None,
            causing: causing_entity_snapshot,
            in_water: entity.was_touching_water(),
            wind_charge: false,
        }),
        center: entity.position(),
        radius: 6.0,
        fire: false,
        interaction: ExplosionInteraction::Block,
        damage_source: None,
        damage_profile: ExplosionDamageProfile::Default,
        presentation: ExplosionPresentation::Default,
    });
    HurtDecision {
        outcome: StoredEntityHurtOutcome {
            applied: true,
            succeeded: true,
        },
        pending_explosion,
    }
}
