//! Shulker attachment, relocation, shell metadata and covered armor.

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ARMOR_ID;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_SHULKER_ATTACH_FACE_DATA_ID, VANILLA_SHULKER_COLOR_DATA_ID,
    VANILLA_SHULKER_DEFAULT_COLOR, VANILLA_SHULKER_DEFAULT_PEEK, VANILLA_SHULKER_PEEK_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID;
use bbb_protocol::packets::{
    BlockPos, Direction, EntityDataValue, EntityDataValueKind, SoundSource, Vec3d,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::geometry::{
    collision_face_full, mth_sin, no_block_collision_with_moving_pistons, Aabb,
};

use super::attributes::{AttributeModifier, AttributeOperation};
use super::living::{LivingEntityState, SynchedValue};
use super::save::{optional_byte, optional_field};
use super::{Entity, EntityData, EntityStore};
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;

const DEFAULT_ATTACH_FACE: Direction = Direction::Down;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShulkerState {
    attach_face: Direction,
    peek: i8,
    color: i8,
    current_peek: f32,
}

impl ShulkerState {
    pub(crate) fn new() -> Self {
        Self {
            attach_face: DEFAULT_ATTACH_FACE,
            peek: VANILLA_SHULKER_DEFAULT_PEEK,
            color: VANILLA_SHULKER_DEFAULT_COLOR,
            current_peek: 0.0,
        }
    }

    // Shulker.java:155-167: preserve the byte values saved by Vanilla.
    pub(crate) fn read_save(entries: &[NbtSummaryEntry]) -> Self {
        Self {
            attach_face: optional_field(entries, "AttachFace")
                .and_then(bbb_protocol::nbt_summary::codec_int_value)
                .map_or(DEFAULT_ATTACH_FACE, |id| {
                    super::hanging::from_3d_id(id as i8)
                }),
            peek: optional_byte(entries, "Peek").unwrap_or(VANILLA_SHULKER_DEFAULT_PEEK),
            color: optional_byte(entries, "Color").unwrap_or(VANILLA_SHULKER_DEFAULT_COLOR),
            current_peek: 0.0,
        }
    }

    pub(crate) fn write_save(self, entries: &mut Vec<NbtSummaryEntry>) {
        entries.extend(
            [
                ("AttachFace", self.attach_face.id() as i8),
                ("Peek", self.peek),
                ("Color", self.color),
            ]
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value: NbtSummaryValue::Byte(value),
            }),
        );
    }

    pub(crate) const fn color(self) -> i8 {
        self.color
    }

    pub(crate) const fn attach_face(self) -> Direction {
        self.attach_face
    }

    // Shulker.java:436-437: only a zero raw peek byte closes the shell.
    pub(crate) const fn is_closed(self) -> bool {
        self.peek == 0
    }

    // Shulker.java:155-160,200-240: currentPeek is runtime animation state not written to NBT.
    pub(crate) fn restore_command_transients(&mut self, previous: Self) {
        self.current_peek = previous.current_peek;
    }

    // Shulker.java:139-143: attachment Direction, raw peek byte and color byte.
    pub(crate) fn synched_values(self) -> [SynchedValue; 3] {
        [
            SynchedValue {
                value: EntityDataValue {
                    data_id: VANILLA_SHULKER_ATTACH_FACE_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
                    value: EntityDataValueKind::Direction(i32::from(self.attach_face.id())),
                },
                is_default: self.attach_face == DEFAULT_ATTACH_FACE,
            },
            SynchedValue {
                value: EntityDataValue {
                    data_id: VANILLA_SHULKER_PEEK_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(self.peek),
                },
                is_default: self.peek == VANILLA_SHULKER_DEFAULT_PEEK,
            },
            SynchedValue {
                value: EntityDataValue {
                    data_id: VANILLA_SHULKER_COLOR_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(self.color),
                },
                is_default: self.color == VANILLA_SHULKER_DEFAULT_COLOR,
            },
        ]
    }
}

pub(crate) fn set_color(entity: &mut Entity, color: i8) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.shulker_state_mut() else {
        return false;
    };
    if state.color != color {
        state.color = color;
        entity.mark_synched_dirty(VANILLA_SHULKER_COLOR_DATA_ID);
    }
    true
}

// Shulker.java:540-542: large shulkers cap their effective scale.
pub(crate) fn sanitize_scale(scale: f32) -> f32 {
    scale.min(3.0)
}

// Shulker.java:200-207: physical shell geometry is derived from the current opening.
pub(crate) fn bounding_box(data: &EntityData, position: Vec3d, size: f32) -> Option<Aabb> {
    let EntityData::GeneralLiving(data) = data else {
        return None;
    };
    let state = data.shulker_state()?;
    Some(progress_box(
        position,
        size,
        opposite(state.attach_face),
        physical_peek(state.current_peek),
    ))
}

fn physical_peek(amount: f32) -> f32 {
    0.5 - mth_sin(f64::from((0.5 + amount) * std::f32::consts::PI)) * 0.5
}

// Shulker.java:246-262: the closed cube extends in the opening direction.
fn progress_box(position: Vec3d, size: f32, direction: Direction, progress: f32) -> Aabb {
    let size = f64::from(size);
    let distance = f64::from(progress) * size;
    Aabb {
        min_x: position.x - size * 0.5,
        min_y: position.y,
        min_z: position.z - size * 0.5,
        max_x: position.x + size * 0.5,
        max_y: position.y + size,
        max_z: position.z + size * 0.5,
    }
    .expand_towards(direction_delta(direction, distance))
}

fn direction_delta(direction: Direction, distance: f64) -> Vec3d {
    let step = relative(BlockPos { x: 0, y: 0, z: 0 }, direction);
    Vec3d {
        x: f64::from(step.x) * distance,
        y: f64::from(step.y) * distance,
        z: f64::from(step.z) * distance,
    }
}

// Shulker.java:210-240: only the newly opened band pushes entities outward.
fn advance_shell(entity: &mut Entity) -> Option<(Aabb, Vec3d)> {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return None;
    };
    let state = data.shulker_state_mut()?;
    let target = f32::from(state.peek) * 0.01;
    if state.current_peek == target {
        return None;
    }
    let old = physical_peek(state.current_peek);
    state.current_peek = if state.current_peek > target {
        let next = state.current_peek - 0.05;
        if next < target {
            target
        } else {
            next.min(1.0)
        }
    } else {
        let next = state.current_peek + 0.05;
        if next < 0.0 {
            0.0
        } else {
            next.min(target)
        }
    };
    let new = physical_peek(state.current_peek);
    let direction = opposite(state.attach_face);
    let position = entity.position();
    let size = entity.bb_width();
    entity.set_pos(position);
    let distance = f64::from((new - old) * size);
    if distance <= 0.0 {
        return None;
    }
    let before = progress_box(position, size, direction, old);
    let mut band = entity.bounding_box();
    match direction {
        Direction::Down => band.max_y = before.min_y,
        Direction::Up => band.min_y = before.max_y,
        Direction::North => band.max_z = before.min_z,
        Direction::South => band.min_z = before.max_z,
        Direction::West => band.max_x = before.min_x,
        Direction::East => band.min_x = before.max_x,
    }
    Some((band, direction_delta(direction, distance)))
}

pub(crate) fn update_armor(living: &mut LivingEntityState, peek: i8) {
    let Some(armor) = living
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_ARMOR_ID)
    else {
        return;
    };
    // Shulker.java:63-64,483-489: only the covered modifier belongs to the shell.
    if peek == 0 {
        armor.add_or_replace_permanent_modifier(AttributeModifier {
            id: "minecraft:covered".to_owned(),
            amount: 20.0,
            operation: AttributeOperation::AddValue,
        });
    } else {
        armor.remove_modifier("minecraft:covered");
    }
}

pub(crate) fn restore_armor(entity: &mut Entity) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(state) = data.shulker_state() {
            update_armor(data.living_mut(), state.peek);
        }
    }
}

pub(crate) fn set_peek(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    amount: i8,
) {
    if !set_raw_peek(entity, amount) {
        return;
    }
    // Shulker.java:483-496: shell sounds have fixed volume and pitch.
    if !entity.is_silent() {
        store.play_entity_sound(super::store::PendingEntitySound {
            position: entity.position(),
            resource_id: if amount == 0 {
                "minecraft:entity.shulker.close"
            } else {
                "minecraft:entity.shulker.open"
            }
            .to_owned(),
            source: SoundSource::Hostile,
            volume: 1.0,
            pitch: 1.0,
        });
    }
    level.game_event_at(
        if amount == 0 {
            VanillaGameEvent::CONTAINER_CLOSE
        } else {
            VanillaGameEvent::CONTAINER_OPEN
        },
        entity.position(),
        Some(entity.id()),
    );
}

fn set_raw_peek(entity: &mut Entity, amount: i8) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.shulker_state_mut() else {
        return false;
    };
    state.peek = amount;
    update_armor(data.living_mut(), amount);
    entity.mark_synched_dirty(VANILLA_SHULKER_PEEK_DATA_ID);
    true
}

fn set_attach_face(entity: &mut Entity, face: Direction) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(state) = data.shulker_state_mut() {
            state.attach_face = face;
            entity.mark_synched_dirty(VANILLA_SHULKER_ATTACH_FACE_DATA_ID);
            entity.set_pos(entity.position());
        }
    }
}

// Shulker.java:171-197: losing support first tries every face at the current position.
pub(crate) fn tick(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_SHULKER_ID {
        return;
    }
    // Shulker.java:266-273: a riding shulker sits on its floor face.
    if store.vehicle(entity.id()).is_some()
        && matches!(entity.data(), EntityData::GeneralLiving(data)
            if data.shulker_state().is_some_and(|state| state.attach_face() != Direction::Down))
    {
        set_attach_face(entity, Direction::Down);
    }
    if store.vehicle(entity.id()).is_none() && can_stay_at(entity, store, level) == Some(false) {
        if let Some(face) = find_attachable_surface(entity, store, level, entity.block_position()) {
            set_attach_face(entity, face);
        } else {
            teleport(entity, store, level);
        }
    }
    if let Some((band, delta)) = advance_shell(entity) {
        push_opening_band(entity, store, level, band, delta);
    }
}

fn push_opening_band(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    band: Aabb,
    delta: Vec3d,
) {
    let ids = super::store::EntityBodyReadView::new(store, Some(entity), level.detached_bodies())
        .ids_in_box_sections_with_players(band);
    for id in ids {
        if id == entity.id()
            || super::store::EntityBodyReadView::new(store, Some(entity), level.detached_bodies())
                .same_vehicle(entity.id(), id)
                != Some(false)
        {
            continue;
        }
        if let Some(player) = store.player(id) {
            if !player.avatar_removed()
                && !player.is_spectator()
                && !player.body().no_physics()
                && player.body().bounding_box().intersects(band)
            {
                store.queue_command(super::EntityCommand::MovePlayerByShulker {
                    player: player.uuid(),
                    delta,
                });
            }
            continue;
        }
        let Some(other) = store.get(id) else {
            continue;
        };
        if other.removal_reason().is_some()
            || other.entity_type_id() == VANILLA_ENTITY_TYPE_SHULKER_ID
            || other.no_physics()
            || !other.bounding_box().intersects(band)
        {
            continue;
        }
        level.with_detached_entity(entity, |level| {
            let _ = store.with_entity_mut(id, |other, store| {
                super::movement::move_living_entity(
                    other,
                    store,
                    level,
                    super::MoverType::Shulker,
                    delta,
                )
            });
        });
    }
}

// Shulker.java:424-431: low-health relocation precedes a projectile-triggered split.
pub(crate) fn after_hurt(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    hit_by_shulker_bullet: bool,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_SHULKER_ID {
        return;
    }
    if entity
        .living()
        .is_some_and(|living| living.health() < living.max_health() * 0.5)
        && level.level_random().next_int(4) == 0
    {
        teleport(entity, store, level);
    } else if hit_by_shulker_bullet {
        split_after_bullet_hit(entity, store, level);
    }
}

// Shulker.java:440-455: an open shell relocates before density permits a fresh, same-colour shell.
fn split_after_bullet_hit(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some(state) = data.shulker_state().filter(|state| !state.is_closed()) else {
        return;
    };
    let color = state.color();
    let position = entity.position();
    let bounds = entity.bounding_box().inflate(8.0, 8.0, 8.0);
    if !teleport(entity, store, level) {
        return;
    }
    let bodies =
        super::store::EntityBodyReadView::new(store, Some(entity), level.detached_bodies());
    let count = bodies
        .ids_in_box_sections(bounds)
        .into_iter()
        .filter_map(|id| bodies.get(id))
        .filter(|body| {
            body.entity_type_id() == VANILLA_ENTITY_TYPE_SHULKER_ID
                && body.is_alive_with_health()
                && body.bounding_box().intersects(bounds)
        })
        .count();
    if level.level_random().next_float() < (count as f32 - 1.0) / 5.0 {
        return;
    }
    let Some(child_id) = store.spawn_general_living(VANILLA_ENTITY_TYPE_SHULKER_ID, position)
    else {
        return;
    };
    if let Some(child) = store.get_mut(child_id) {
        if let EntityData::GeneralLiving(data) = &mut child.data {
            if let Some(state) = data.shulker_state_mut() {
                state.color = color;
                child.mark_synched_dirty(VANILLA_SHULKER_COLOR_DATA_ID);
            }
        }
    }
    store.queue_command(super::EntityCommand::FinalizeShulkerSplit {
        entity_id: child_id,
    });
}

fn find_attachable_surface(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
) -> Option<Direction> {
    [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ]
    .into_iter()
    .find(|face| can_stay_at_face(entity, store, level, pos, *face) == Some(true))
}

// Shulker.java:374-404: five nearby air candidates, without a landing search.
fn teleport(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) -> bool {
    if !entity.is_alive_with_health()
        || matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai())
    {
        return false;
    }
    let origin = entity.block_position();
    for _ in 0..5 {
        let target = BlockPos {
            x: origin.x + level.level_random().next_int(17) - 8,
            y: origin.y + level.level_random().next_int(17) - 8,
            z: origin.z + level.level_random().next_int(17) - 8,
        };
        if target.y <= level.min_y()
            || !level
                .block_state_checked(target)
                .and_then(bbb_protocol::block_state_types::vanilla_block_state_path_facts)
                .is_some_and(|facts| facts.is_air())
            || !level
                .world_border()
                .is_some_and(|border| border.contains_block_at(target, level.game_time()))
            || no_collision(
                entity,
                store,
                level,
                Aabb {
                    min_x: f64::from(target.x),
                    min_y: f64::from(target.y),
                    min_z: f64::from(target.z),
                    max_x: f64::from(target.x) + 1.0,
                    max_y: f64::from(target.y) + 1.0,
                    max_z: f64::from(target.z) + 1.0,
                }
                .inflate(-1.0e-6, -1.0e-6, -1.0e-6),
            ) != Some(true)
        {
            continue;
        }
        let Some(face) = find_attachable_surface(entity, store, level, target) else {
            continue;
        };
        store.unride_body(entity);
        set_attach_face(entity, face);
        if !entity.is_silent() {
            store.play_entity_sound(super::store::PendingEntitySound {
                position: entity.position(),
                resource_id: "minecraft:entity.shulker.teleport".to_owned(),
                source: SoundSource::Hostile,
                volume: 1.0,
                pitch: 1.0,
            });
        }
        entity.set_pos(Vec3d {
            x: f64::from(target.x) + 0.5,
            y: f64::from(target.y),
            z: f64::from(target.z) + 0.5,
        });
        entity.set_needs_sync(true);
        level.game_event(VanillaGameEvent::TELEPORT, origin, Some(entity.id()));
        set_raw_peek(entity, 0);
        super::ai::mob::target::sense::set_body_target(entity, store, level, None);
        return true;
    }
    false
}

// Shulker.java:350-362: a supported attachment needs room for the fully opened shell.
pub(crate) fn can_stay_at(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<bool> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let state = data.shulker_state()?;
    can_stay_at_face(
        entity,
        store,
        level,
        entity.block_position(),
        state.attach_face(),
    )
}

fn can_stay_at_face(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    face: Direction,
) -> Option<bool> {
    let here = level.block_state_checked(pos)?;
    let here_is_air = if let Some(facts) =
        bbb_protocol::block_state_types::vanilla_block_state_path_facts(here)
    {
        facts.is_air()
    } else {
        level.content_registry().block_by_state_id(here)?;
        false
    };
    if !here_is_air
        && !(pos == entity.block_position()
            && bbb_server_types::block_queries::block_type_of(here)
                == Some(bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID))
    {
        return Some(false);
    }
    let opening = opposite(face);
    let support = relative(pos, face);
    if !level.is_in_world_bounds(support) {
        return Some(false);
    }
    let support_state = level.block_state_checked(support)?;
    let full = match bbb_server_types::block_queries::collision_face_full_with_registry(
        support_state,
        opening,
        level.content_registry(),
    ) {
        Some(full) => full,
        None => collision_face_full(
            &level.block_collision_boxes_for_entity(entity, support)?,
            opening,
        )?,
    };
    if !full {
        return Some(false);
    }
    let bounds = progress_box(
        Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y),
            z: f64::from(pos.z) + 0.5,
        },
        entity.bb_width(),
        opening,
        1.0,
    )
    .inflate(-1.0e-6, -1.0e-6, -1.0e-6);
    no_collision(entity, store, level, bounds)
}

fn no_collision(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    bounds: Aabb,
) -> Option<bool> {
    if !no_block_collision_with_moving_pistons(
        bounds,
        |at| {
            level.block_state_checked(at)?;
            level.is_moving_piston_at(at)
        },
        |at| {
            level.block_state_checked(at)?;
            level.block_collision_boxes_for_entity(entity, at)
        },
    )? {
        return Some(false);
    }
    for id in store.ids_in_box_sections(bounds.inflate(1.0e-7, 1.0e-7, 1.0e-7)) {
        let other = store.get(id)?;
        if id != entity.id()
            && other.removal_reason().is_none()
            && other.can_be_collided_with()
            && other.bounding_box().intersects(bounds)
            && !store.checked_same_vehicle(entity, id)?
        {
            return Some(false);
        }
    }
    Some(
        level
            .world_border()?
            .no_collision_at(entity.position(), bounds, level.game_time()),
    )
}
