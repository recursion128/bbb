use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
use bbb_protocol::entity_synched_data::VANILLA_PHANTOM_SIZE_DATA_ID;
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::ai::mob::phantom::{PhantomAiStepResult, PhantomAttack};
use crate::entity::environment::{fluid_contact, FluidContact, FluidTag};
use crate::entity::living::{
    apply_immobile, apply_input, apply_movement_dead_band, travel_flying, FlyingContact,
    SynchedValue,
};
use crate::entity::save::optional_block_pos;
use crate::entity::save::optional_int;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

/// `LevelEvent.SOUND_PHANTOM_BITE` (`LevelEvent.java:33`).
const PHANTOM_BITE_LEVEL_EVENT: i32 = 1039;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PhantomRetainedState {
    size: i32,
    anchor: Option<BlockPos>,
}

impl PhantomRetainedState {
    pub(in crate::entity) const fn from_saved(size: i32, anchor: Option<BlockPos>) -> Self {
        Self {
            size: clamp_size(size),
            anchor,
        }
    }

    pub(crate) const fn size(self) -> i32 {
        self.size
    }

    pub(crate) const fn anchor(self) -> Option<BlockPos> {
        self.anchor
    }

    fn set_size(&mut self, size: i32) -> bool {
        let size = clamp_size(size);
        let changed = self.size != size;
        self.size = size;
        changed
    }

    fn set_anchor(&mut self, anchor: Option<BlockPos>) {
        self.anchor = anchor;
    }
}

pub(crate) fn set_size(entity: &mut Entity, size: i32) -> bool {
    let actual_size = clamp_size(size);
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let Some(state) = data.phantom_retained_mut() else {
                return false;
            };
            state.set_size(actual_size)
        }
        _ => return false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_PHANTOM_SIZE_DATA_ID);
        apply_size_derived_state(entity, actual_size);
    }
    true
}

pub(crate) fn finalize_spawn_state(entity: &mut Entity) -> bool {
    let position = entity.block_position();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.phantom_retained_mut() else {
        return false;
    };
    state.set_anchor(Some(BlockPos {
        x: position.x,
        y: position.y.wrapping_add(5),
        z: position.z,
    }));
    set_size(entity, 0)
}

pub(crate) fn finish_load(
    entity: &mut Entity,
    previous: Option<&Entity>,
    level: Option<&LevelCtx<'_, '_>>,
) {
    let size = match entity.data() {
        EntityData::GeneralLiving(data) => data.phantom_retained().map(PhantomRetainedState::size),
        _ => None,
    };
    if let Some(size) = size {
        let previous_size = previous
            .and_then(|entity| match entity.data() {
                EntityData::GeneralLiving(data) => {
                    data.phantom_retained().map(PhantomRetainedState::size)
                }
                _ => None,
            })
            .unwrap_or(0);
        if size != previous_size {
            let _ = apply_size_derived_state_with(entity, size, |entity| {
                let _ = crate::entity::living::refresh_dimensions_with_context(entity, level, None);
                Some(())
            });
        }
    }
}

fn apply_size_derived_state(entity: &mut Entity, size: i32) {
    let _ = apply_size_derived_state_with(entity, size, |entity| {
        crate::entity::living::refresh_dimensions(entity);
        Some(())
    });
}

fn apply_size_derived_state_with(
    entity: &mut Entity,
    size: i32,
    refresh: impl FnOnce(&mut Entity) -> Option<()>,
) -> Option<()> {
    refresh(entity)?;
    if let Some(attribute) = entity.living_mut().and_then(|living| {
        living
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
    }) {
        attribute.set_base_value(f64::from(6 + size));
    }
    Some(())
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: PhantomRetainedState) {
    if let Some(anchor) = state.anchor() {
        entries.push(block_pos_entry("anchor_pos", anchor));
    }
    entries.push(NbtSummaryEntry {
        name: "size".into(),
        value: NbtSummaryValue::Int(state.size()),
    });
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> PhantomRetainedState {
    PhantomRetainedState::from_saved(
        optional_int(entries, "size").unwrap_or(0),
        optional_block_pos(entries, "anchor_pos"),
    )
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "anchor_pos" | "size")
}

pub(crate) fn synched_values(state: PhantomRetainedState) -> [SynchedValue; 1] {
    [SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_PHANTOM_SIZE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(state.size()),
        },
        is_default: state.size() == 0,
    }]
}

pub(crate) fn ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    contact: FlyingContact,
) -> Result<bool, ()> {
    let is_phantom = matches!(
        entity.data(),
        EntityData::GeneralLiving(data) if data.phantom_retained().is_some()
    );
    if !is_phantom {
        return Ok(false);
    }
    if contact == FlyingContact::Unknown {
        return Err(());
    }
    apply_movement_dead_band(entity);
    let (runs_ai, immobile, input) = {
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!("Phantom uses GeneralLiving")
        };
        let runs_ai = !data.is_no_ai();
        let living = data.living_mut();
        apply_input(living);
        let immobile = living.is_immobile();
        if immobile {
            apply_immobile(living);
        }
        let input = Vec3d {
            x: f64::from(living.xxa),
            y: f64::from(living.yya),
            z: f64::from(living.zza),
        };
        (runs_ai, immobile, input)
    };
    if runs_ai && !immobile {
        if let Some(living) = entity.living_mut() {
            living.no_action_time = living.no_action_time.wrapping_add(1);
        }
        let id = entity.id();
        let result = store
            .with_mob_ai_control(id, id, |runtime, _, store| {
                runtime.phantom_step(entity, store, level)
            })
            .flatten();
        if let Some(result) = result {
            let attack = apply_ai_step(entity, result);
            if !apply_attack(entity, store, level, attack) {
                return Err(());
            }
        }
    }
    if runs_ai {
        travel_flying(entity, store, level, input, contact, 0.2, 0.02)?;
    }
    Ok(true)
}

/// The level writes of the attack goals: the swoop cry
/// (`Phantom.java:284`), the hiss of the cats that scare the phantom off
/// (`Phantom.java:472-474`, `Cat.hiss`) and the sweep's hit
/// (`Phantom.java:495-500`).
fn apply_attack(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    attack: PhantomAttack,
) -> bool {
    if let Some(pitch) = attack.swoop_pitch.filter(|_| !entity.is_silent()) {
        store.play_entity_sound(crate::entity::PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.phantom.swoop".to_owned(),
            source: crate::entity::living::living_sound_source(entity),
            volume: 10.0,
            pitch,
        });
    }
    for cat_id in attack.hissing_cats {
        let Some(cat) = store.get(cat_id).filter(|cat| !cat.is_silent()) else {
            continue;
        };
        let (position, source) = (
            cat.position(),
            crate::entity::living::living_sound_source(cat),
        );
        let baby = matches!(cat.data(), EntityData::GeneralLiving(data)
            if data.ageable_retained().is_some_and(|age| age.is_baby()));
        let Some(sound) = super::cat_sounds::resolve(store, cat, super::cat_sounds::CatSound::Hiss)
        else {
            continue;
        };
        let pitch = crate::entity::living::voice_pitch(store, baby);
        super::sound_variants::play(store, sound, position, source, 1.0, pitch);
    }
    if let Some(target_id) = attack.hurt_target {
        if !crate::entity::ai::mob::do_hurt_target(entity, store, level, target_id) {
            return false;
        }
        if !entity.is_silent() {
            level.level_event(PHANTOM_BITE_LEVEL_EVENT, entity.block_position(), 0);
        }
    }
    true
}

fn apply_ai_step(entity: &mut Entity, result: PhantomAiStepResult) -> PhantomAttack {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(state) = data.phantom_retained_mut() {
            state.set_anchor(result.anchor);
        }
    }
    entity.set_delta_movement(result.delta_movement);
    entity.set_rot(result.y_rot, result.x_rot);
    if let (Some(living), Some(y_body_rot)) = (entity.living_mut(), result.move_body_rot) {
        living.y_body_rot = y_body_rot;
    }
    result.attack
}

pub(crate) fn body_rotation(entity: &mut Entity) -> bool {
    let is_phantom = matches!(
        entity.data(),
        EntityData::GeneralLiving(data) if data.phantom_retained().is_some()
    );
    if !is_phantom {
        return false;
    }
    let y_rot = entity.y_rot();
    if let Some(living) = entity.living_mut() {
        living.y_head_rot = living.y_body_rot;
        living.y_body_rot = y_rot;
    }
    true
}

pub(crate) fn flying_contact(entity: &Entity, level: &LevelCtx<'_, '_>) -> FlyingContact {
    match fluid_contact(entity, level, FluidTag::Water) {
        FluidContact::Known(true) => return FlyingContact::Water,
        FluidContact::Unknown => return FlyingContact::Unknown,
        FluidContact::Known(false) => {}
    }
    match fluid_contact(entity, level, FluidTag::Lava).known() {
        Some(true) => FlyingContact::Lava,
        Some(false) => FlyingContact::Air,
        None => FlyingContact::Unknown,
    }
}

const fn clamp_size(size: i32) -> i32 {
    if size < 0 {
        0
    } else if size > 64 {
        64
    } else {
        size
    }
}

fn block_pos_entry(name: &str, pos: BlockPos) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z]),
    }
}

#[cfg(test)]
mod tests;
