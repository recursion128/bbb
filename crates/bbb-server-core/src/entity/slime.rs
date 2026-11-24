use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
    VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
    VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
use bbb_protocol::entity_synched_data::VANILLA_SLIME_SIZE_DATA_ID;
use bbb_protocol::entity_types::{
    vanilla_entity_type_eye_height, VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, VANILLA_ENTITY_TYPE_SLIME_ID,
};
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind, SoundSource};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use crate::entity::ai::mob::slime::SlimeJumpSound;
use crate::entity::conversion::CommonMobConversionSnapshot;
use crate::entity::level::fluid_state_tag_membership;
use crate::entity::living::{entity_block_jump_factor, SynchedValue};
use crate::entity::save::{optional_bool, optional_int};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData, GeneralLivingData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlimeRetainedState {
    size: i32,
    was_on_ground: bool,
}

impl SlimeRetainedState {
    pub(in crate::entity) const fn from_saved(size: i32, was_on_ground: bool) -> Self {
        Self {
            size,
            was_on_ground,
        }
    }

    pub(crate) const fn size(self) -> i32 {
        self.size
    }

    pub(crate) const fn was_on_ground(self) -> bool {
        self.was_on_ground
    }
}

pub(crate) const fn is_slime(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_SLIME_ID | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
    )
}

pub(crate) fn initialize_spawn(entity: &mut Entity) {
    let _ = set_size(entity, 1, true);
}

pub(crate) fn finish_load(entity: &mut Entity, level: Option<&LevelCtx<'_, '_>>) {
    let size = match entity.data() {
        EntityData::GeneralLiving(data) => data.slime_retained().map(SlimeRetainedState::size),
        _ => None,
    };
    if let Some(size) = size {
        let _ = apply_size_derived_state_with(entity, size, false, |entity| {
            let _ = crate::entity::living::refresh_dimensions_with_context(entity, level, None);
            Some(())
        });
    }
}

pub(crate) fn set_size(entity: &mut Entity, size: i32, update_health: bool) -> bool {
    let actual_size = size.clamp(1, 127);
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let Some(state) = data.slime_retained_mut() else {
                return false;
            };
            let changed = state.size != actual_size;
            state.size = actual_size;
            changed
        }
        _ => return false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_SLIME_SIZE_DATA_ID);
    }
    apply_size_derived_state(entity, actual_size, update_health);
    true
}

pub(crate) fn split_on_removal(source: &Entity, store: &mut EntityStore) {
    if source
        .living()
        .is_none_or(|living| !living.is_dead_or_dying())
    {
        return;
    }
    let size = match source.data() {
        EntityData::GeneralLiving(data) => match data.slime_retained() {
            Some(state) if state.size() > 1 => state.size(),
            _ => return,
        },
        _ => return,
    };
    let Some(count) = store
        .mob_ai_mut(source.id())
        .map(|runtime| runtime.slime_split_count())
    else {
        return;
    };
    let width_offset = source.bb_width() / 2.0;
    let half_size = size / 2;
    for index in 0..count {
        let uuid = store.next_uuid();
        let data = GeneralLivingData::new(source.entity_type_id()).expect("slime entity type");
        let y_rot = store
            .mob_ai_mut(source.id())
            .expect("splitting slime runtime")
            .slime_split_y_rot();
        let xd = (index as i32 % 2) as f32 - 0.5;
        let zd = (index as i32 / 2) as f32 - 0.5;
        let position = bbb_protocol::packets::Vec3d {
            x: source.position().x + f64::from(xd * width_offset),
            y: source.position().y + 0.5,
            z: source.position().z + f64::from(zd * width_offset),
        };
        store
            .restore_initialized(
                uuid,
                source.entity_type_id(),
                position,
                EntityData::GeneralLiving(data),
                |child| {
                    CommonMobConversionSnapshot::capture(source, false).apply(child);
                    let _ = set_size(child, half_size, true);
                    child.set_rot(y_rot, 0.0);
                },
            )
            .expect("fresh split child is admitted");
    }
}

fn apply_size_derived_state(entity: &mut Entity, size: i32, update_health: bool) {
    let _ = apply_size_derived_state_with(entity, size, update_health, |entity| {
        crate::entity::living::refresh_dimensions(entity);
        Some(())
    });
}

fn apply_size_derived_state_with(
    entity: &mut Entity,
    size: i32,
    update_health: bool,
    refresh: impl FnOnce(&mut Entity) -> Option<()>,
) -> Option<()> {
    refresh(entity)?;
    let magma_cube = entity.entity_type_id() == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID;
    let Some(living) = entity.living_mut() else {
        return Some(());
    };
    let size_f32 = size as f32;
    for (attribute_id, value) in [
        (
            VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
            f64::from((size * size) as f32),
        ),
        (
            VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            f64::from(0.2_f32 + 0.1_f32 * size_f32),
        ),
        (VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, f64::from(size_f32)),
    ] {
        if let Some(attribute) = living.attributes_mut().get_instance(attribute_id) {
            attribute.set_base_value(value);
        }
    }
    if magma_cube {
        if let Some(armor) = living
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_ARMOR_ID)
        {
            armor.set_base_value(f64::from(size * 3));
        }
    }
    if update_health {
        living.set_health(living.max_health());
    }
    Some(())
}

pub(crate) fn eye_y(entity: &Entity) -> Option<f64> {
    let size = match entity.data() {
        EntityData::GeneralLiving(data) => data.slime_retained()?.size(),
        _ => return None,
    };
    vanilla_entity_type_eye_height(entity.entity_type_id())
        .map(|height| entity.position().y + f64::from(height * size as f32))
}

/// `Slime.playerTouch` over `dealDamage` (`Slime.java:228-242`): a slime
/// the session touches hits it for `getAttackDamage`.
pub(crate) fn player_touch(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    slime_id: i32,
    player: &crate::entity::level::NearbyPlayer,
) {
    let Some(slime) = store.get(slime_id) else {
        return;
    };
    let EntityData::GeneralLiving(data) = slime.data() else {
        return;
    };
    let Some(size) = data.slime_retained().map(SlimeRetainedState::size) else {
        return;
    };
    let magma_cube = slime.entity_type_id() == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID;
    // `isDealsDamage` (`:249-251`, `MagmaCube.java:88-90`).
    let deals_damage = (magma_cube || size > 1) && !data.is_no_ai();
    let alive = slime.is_alive() && !data.living().is_dead_or_dying();
    let bounds = bbb_server_types::geometry::Aabb::of_entity(
        player.position,
        player.dimensions[0],
        player.dimensions[1],
    );
    if !deals_damage
        || !alive
        || !crate::entity::ai::mob::melee_attack_box(slime.bounding_box()).intersects(bounds)
    {
        return;
    }
    let from = bbb_protocol::packets::Vec3d {
        y: eye_y(slime).unwrap_or(slime.position().y),
        ..slime.position()
    };
    let to = bbb_protocol::packets::Vec3d {
        y: player.position.y + player.eye_height,
        ..player.position
    };
    if !crate::entity::projectile_hit::collision_clip_misses(level, from, to) {
        return;
    }
    let damage = data
        .living()
        .attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID) as f32
        + if magma_cube { 2.0 } else { 0.0 };
    let attacker = crate::level_event::PendingHurtEntity::Stored {
        entity_id: slime_id,
    };
    level.hurt_player(crate::level_event::PendingPlayerHurt {
        player: player.uuid,
        damage_type: crate::entity::ai::MOB_ATTACK_DAMAGE_TYPE,
        amount: damage,
        causing: Some(attacker.clone()),
        direct: Some(attacker),
        post_attack: true,
        impulse: None,
    });
    if player.dead_or_dying || player.spectator || player.invulnerable || slime.is_silent() {
        return;
    }
    let position = slime.position();
    let random = store.random();
    let pitch = (random.next_float() - random.next_float()) * 0.2 + 1.0;
    store.play_entity_sound(PendingEntitySound {
        position,
        resource_id: "minecraft:entity.slime.attack".to_owned(),
        source: SoundSource::Hostile,
        volume: 1.0,
        pitch,
    });
}

/// `Slime.push` over `dealDamage` (`Slime.java:220-242`): a damaging slime
/// that bumps an iron golem hits it. Vanilla runs this from the golem's push
/// sweep; the slime's own sweep meets the same pair.
pub(crate) fn push_iron_golem(
    slime: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    golem_id: i32,
) {
    let EntityData::GeneralLiving(data) = slime.data() else {
        return;
    };
    let Some(size) = data.slime_retained().map(SlimeRetainedState::size) else {
        return;
    };
    let Some(golem) = store.get(golem_id).filter(|golem| {
        golem.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
    }) else {
        return;
    };
    let magma_cube = slime.entity_type_id() == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID;
    // `isDealsDamage` (`:249-251`, `MagmaCube.java:88-90`).
    let deals_damage = (magma_cube || size > 1) && !data.is_no_ai();
    if !deals_damage
        || !slime.is_alive()
        || data.living().is_dead_or_dying()
        || !crate::entity::ai::mob::melee_attack_box(slime.bounding_box())
            .intersects(golem.bounding_box())
    {
        return;
    }
    let from = bbb_protocol::packets::Vec3d {
        y: eye_y(slime).unwrap_or(slime.position().y),
        ..slime.position()
    };
    let to = bbb_protocol::packets::Vec3d {
        y: golem.position().y
            + f64::from(vanilla_entity_type_eye_height(golem.entity_type_id()).unwrap_or(0.0)),
        ..golem.position()
    };
    if !crate::entity::projectile_hit::collision_clip_misses(level, from, to) {
        return;
    }
    let damage = data
        .living()
        .attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID) as f32
        + if magma_cube { 2.0 } else { 0.0 };
    let hurt = crate::entity::ai::mob::brain_runtime::hurt_entity_from_body(
        slime,
        store,
        level,
        golem_id,
        crate::entity::ai::MOB_ATTACK_DAMAGE_TYPE,
        damage,
    );
    if hurt == Some(true) && !slime.is_silent() {
        let random = store.random();
        let pitch = (random.next_float() - random.next_float()) * 0.2 + 1.0;
        store.play_entity_sound(PendingEntitySound {
            position: slime.position(),
            resource_id: "minecraft:entity.slime.attack".to_owned(),
            source: SoundSource::Hostile,
            volume: 1.0,
            pitch,
        });
    }
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: SlimeRetainedState) {
    entries.extend([
        int_entry("Size", state.size() - 1),
        NbtSummaryEntry {
            name: "wasOnGround".into(),
            value: NbtSummaryValue::Byte(i8::from(state.was_on_ground())),
        },
    ]);
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> SlimeRetainedState {
    let stored_size = optional_int(entries, "Size").unwrap_or(0);
    let size = stored_size.wrapping_add(1).clamp(1, 127);
    let was_on_ground = optional_bool(entries, "wasOnGround").unwrap_or(false);
    SlimeRetainedState::from_saved(size, was_on_ground)
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "Size" | "wasOnGround")
}

pub(crate) fn synched_values(state: SlimeRetainedState) -> [SynchedValue; 1] {
    [SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_SLIME_SIZE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(state.size()),
        },
        is_default: state.size() == 1,
    }]
}

pub(crate) fn after_super(entity: &mut Entity, store: &mut EntityStore) {
    let (size, was_on_ground) = match entity.data() {
        EntityData::GeneralLiving(data) => match data.slime_retained() {
            Some(state) => (state.size(), state.was_on_ground()),
            None => return,
        },
        _ => return,
    };
    let on_ground = entity.on_ground();
    let sound = store.mob_ai_mut(entity.id()).and_then(|runtime| {
        runtime.slime_after_super(
            on_ground,
            was_on_ground,
            size,
            entity.entity_type_id() == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
        )
    });
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(state) = data.slime_retained_mut() {
            state.was_on_ground = on_ground;
        }
    }
    if let Some(sound) = sound {
        play_sound(entity, store, sound);
    }
}

pub(crate) fn apply_ai_step(entity: &mut Entity, store: &mut EntityStore, result: SlimeJumpSound) {
    play_sound(entity, store, result);
}

pub(crate) fn apply_control_rotation(entity: &mut Entity, y_rot: f32) {
    entity.set_rot(y_rot, entity.x_rot());
}

pub(crate) fn tick_jump(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
    contact: SlimeFluidContact,
) -> bool {
    let size = match entity.data() {
        EntityData::GeneralLiving(data) => match data.slime_retained() {
            Some(state) => state.size(),
            None => return false,
        },
        _ => return false,
    };
    let jumping = entity
        .living_mut()
        .expect("Slime is living")
        .begin_jump_step();
    if !jumping {
        return true;
    }
    let threshold = if eye_y(entity).is_some_and(|eye_y| eye_y - entity.position().y < 0.4) {
        0.0
    } else {
        0.4
    };
    let liquid = match contact {
        SlimeFluidContact::Water(height) => {
            (!entity.on_ground() || height > threshold).then_some(FluidFamily::Water)
        }
        SlimeFluidContact::Lava(height) => {
            (!entity.on_ground() || height > threshold).then_some(FluidFamily::Lava)
        }
        SlimeFluidContact::Dry => None,
        SlimeFluidContact::Unknown => return true,
    };
    if let Some(fluid) = liquid {
        let mut movement = entity.delta_movement();
        if entity.entity_type_id() == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
            && fluid == FluidFamily::Lava
        {
            movement.y = f64::from(0.22_f32 + size as f32 * 0.05_f32);
            entity.set_needs_sync(true);
        } else {
            movement.y += f64::from(0.04_f32);
        }
        entity.set_delta_movement(movement);
        return true;
    }
    if entity.on_ground()
        && entity
            .living()
            .is_some_and(|living| living.no_jump_delay() == 0)
    {
        let jump_power = entity
            .living()
            .expect("Slime is living")
            .attribute_value(VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID) as f32
            * entity_block_jump_factor(entity, level);
        let mut movement = entity.delta_movement();
        movement.y = f64::from(
            if entity.entity_type_id() == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID {
                jump_power + size as f32 * 0.1_f32
            } else {
                jump_power
            },
        );
        entity.set_delta_movement(movement);
        entity.set_needs_sync(true);
        entity
            .living_mut()
            .expect("Slime is living")
            .set_no_jump_delay(10);
    }
    true
}

pub(crate) fn blocks_unowned_fluid_travel(contact: SlimeFluidContact) -> bool {
    !matches!(contact, SlimeFluidContact::Dry)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SlimeFluidContact {
    Water(f64),
    Lava(f64),
    Dry,
    Unknown,
}

pub(crate) fn fluid_contact(
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
) -> Option<SlimeFluidContact> {
    if !is_slime(entity.entity_type_id()) {
        return None;
    }
    match fluid_depth(entity, level, FluidFamily::Water) {
        FluidDepth::Present(height) => return Some(SlimeFluidContact::Water(height)),
        FluidDepth::Unknown => return Some(SlimeFluidContact::Unknown),
        FluidDepth::Absent => {}
    }
    Some(match fluid_depth(entity, level, FluidFamily::Lava) {
        FluidDepth::Present(height) => SlimeFluidContact::Lava(height),
        FluidDepth::Absent => SlimeFluidContact::Dry,
        FluidDepth::Unknown => SlimeFluidContact::Unknown,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FluidDepth {
    Present(f64),
    Absent,
    Unknown,
}

fn fluid_depth(entity: &Entity, level: &LevelCtx<'_, '_>, family: FluidFamily) -> FluidDepth {
    let tag_id = match family {
        FluidFamily::Water => "minecraft:water",
        FluidFamily::Lava => "minecraft:lava",
    };
    let bb = entity.bounding_box();
    let min_x = (bb.min_x + 0.001).floor() as i32;
    let min_y = (bb.min_y + 0.001).floor() as i32;
    let min_z = (bb.min_z + 0.001).floor() as i32;
    let max_x = (bb.max_x - 0.001).ceil() as i32 - 1;
    let max_y = (bb.max_y - 0.001).ceil() as i32 - 1;
    let max_z = (bb.max_z - 0.001).ceil() as i32 - 1;
    let mut max_depth = 0.0_f64;
    let mut unknown = false;
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                let pos = BlockPos { x, y, z };
                let state_id = level.block_state(pos);
                if vanilla_block_state_block_type_id(state_id).is_none() {
                    if level
                        .content_registry()
                        .block_by_state_id(state_id)
                        .is_none()
                    {
                        unknown = true;
                    }
                    continue;
                }
                let fluid = block_state_fluid_state(state_id);
                if !fluid_state_tag_membership(level, fluid, tag_id) {
                    continue;
                }
                let above_id = level.block_state(BlockPos { y: y + 1, ..pos });
                if vanilla_block_state_block_type_id(above_id).is_none()
                    && level
                        .content_registry()
                        .block_by_state_id(above_id)
                        .is_none()
                {
                    unknown = true;
                    continue;
                }
                let height = if block_state_fluid_state(above_id).family() == fluid.family() {
                    1.0
                } else {
                    f64::from(fluid.own_height())
                };
                max_depth = max_depth.max(f64::from(y) + height - bb.min_y);
            }
        }
    }
    if unknown {
        FluidDepth::Unknown
    } else if max_depth > 0.0 {
        FluidDepth::Present(max_depth)
    } else {
        FluidDepth::Absent
    }
}

fn play_sound(entity: &Entity, store: &mut EntityStore, sound: SlimeJumpSound) {
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: sound.resource_id.to_string(),
            source: SoundSource::Hostile,
            volume: sound.volume,
            pitch: sound.pitch,
        });
    }
}

fn int_entry(name: &str, value: i32) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Int(value),
    }
}

#[cfg(test)]
#[path = "slime/tests.rs"]
mod tests;
