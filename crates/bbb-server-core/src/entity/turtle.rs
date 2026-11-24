use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_TURTLE_HAS_EGG_DATA_ID, VANILLA_TURTLE_LAYING_EGG_DATA_ID,
};
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::environment::{fluid_contact, FluidContact, FluidTag};
use crate::entity::level::block_state_tag_membership;
use crate::entity::living::{move_relative, SynchedValue};
use crate::entity::save::{optional_block_pos, optional_bool};
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData, GeneralLivingData, MoverType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TurtleRetainedState {
    home_pos: BlockPos,
    has_egg: bool,
    laying_egg: bool,
    lay_egg_counter: i32,
    going_home: bool,
    lay_target: Option<BlockPos>,
    retry_ticks: i32,
    close_to_home_ticks: i32,
}

impl TurtleRetainedState {
    pub(crate) const fn going_home(&self) -> bool {
        self.going_home
    }

    /// `Turtle.hasEgg()` (`Turtle.java:93-95`), the extra term
    /// `Turtle.canFallInLove` adds (`:203-206`).
    pub(crate) const fn has_egg(&self) -> bool {
        self.has_egg
    }

    pub(crate) fn set_has_egg(&mut self, has_egg: bool) {
        self.has_egg = has_egg;
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.laying_egg = previous.laying_egg;
        self.lay_egg_counter = previous.lay_egg_counter;
        self.going_home = previous.going_home;
        self.lay_target = previous.lay_target;
        self.retry_ticks = previous.retry_ticks;
        self.close_to_home_ticks = previous.close_to_home_ticks;
    }

    pub(crate) fn new() -> Self {
        Self {
            home_pos: BlockPos { x: 0, y: 0, z: 0 },
            has_egg: false,
            laying_egg: false,
            lay_egg_counter: 0,
            going_home: false,
            lay_target: None,
            retry_ticks: 0,
            close_to_home_ticks: 0,
        }
    }

    fn from_saved(home_pos: BlockPos, has_egg: bool) -> Self {
        Self {
            home_pos,
            has_egg,
            ..Self::new()
        }
    }

    #[cfg(test)]
    pub(crate) const fn home_pos(&self) -> BlockPos {
        self.home_pos
    }

    fn set_laying_egg(&mut self, value: bool) -> bool {
        self.lay_egg_counter = if value { 1 } else { 0 };
        let changed = self.laying_egg != value;
        self.laying_egg = value;
        changed
    }

    #[cfg(test)]
    fn set_homeward(&mut self, home_pos: BlockPos) {
        self.home_pos = home_pos;
        self.going_home = true;
    }
}

pub(crate) fn initialize_hatch(data: &mut GeneralLivingData, home_pos: BlockPos) -> bool {
    let Some(turtle) = data.turtle_retained_mut() else {
        return false;
    };
    turtle.home_pos = home_pos;
    let Some(ageable) = data.ageable_retained_mut() else {
        return false;
    };
    let _baby_changed = ageable.set_age(-24_000);
    true
}

pub(crate) fn set_spawn_home(entity: &mut Entity, home_pos: BlockPos) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.turtle_retained_mut() else {
        return false;
    };
    state.home_pos = home_pos;
    true
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "home_pos" | "has_egg")
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: &TurtleRetainedState) {
    entries.extend([
        block_pos_entry("home_pos", state.home_pos),
        boolean_entry("has_egg", state.has_egg),
    ]);
}

pub(crate) fn read_save_data(
    entries: &[NbtSummaryEntry],
    position: [f64; 3],
) -> TurtleRetainedState {
    TurtleRetainedState::from_saved(
        optional_block_pos(entries, "home_pos").unwrap_or_else(|| block_position(position)),
        optional_bool(entries, "has_egg").unwrap_or(false),
    )
}

pub(crate) fn synched_values(state: &TurtleRetainedState) -> [SynchedValue; 2] {
    [
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_TURTLE_HAS_EGG_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(state.has_egg),
            },
            is_default: !state.has_egg,
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_TURTLE_LAYING_EGG_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(state.laying_egg),
            },
            is_default: !state.laying_egg,
        },
    ]
}

// Turtle.java:337-345; Turtle.java:437-474: adults return home, then dig on sand and lay eggs.
pub(crate) fn custom_server_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some(state) = data.turtle_retained() else {
        return;
    };
    let before = (state.has_egg, state.laying_egg);
    let position = entity.position();
    let block_pos = entity.block_position();
    let in_water = entity.is_in_water();
    let id = entity.id();
    let tick_count = entity.tick_count();
    let Some(runtime) = store.mob_ai(id) else {
        return;
    };
    let Some(mob) = crate::entity::ai::mob::navigation::MobPathingSnapshot::from_entity(
        entity,
        false,
        store.vehicle(id).is_some(),
        store.difficulty(),
        runtime.pathfinding_malus(),
    ) else {
        return;
    };
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    let baby = data.ageable_retained().is_some_and(|age| age.is_baby());
    let in_love = data
        .animal_retained()
        .is_some_and(|animal| animal.in_love() > 0);
    let state = data.turtle_retained_mut().expect("turtle retained state");
    let mut laid = false;
    (|| {
        if state.retry_ticks > 0 {
            state.retry_ticks -= 1;
        }
        if baby || (!state.has_egg && in_love) {
            if state.going_home {
                store
                    .mob_ai_mut(id)
                    .expect("turtle runtime")
                    .stop_navigation();
            }
            state.going_home = false;
            return;
        }
        if state.has_egg && closer_to_center(state.home_pos, position, 9.0) {
            state.going_home = false;
            if state
                .lay_target
                .is_some_and(|target| !valid_lay_target(level, target))
            {
                state.lay_target = None;
                state.set_laying_egg(false);
            }
            if state.lay_target.is_none() && state.retry_ticks == 0 {
                state.lay_target = find_lay_target(level, block_pos);
                state.retry_ticks = 200 + level.level_random().next_int(200);
            }
            if let Some(target) = state.lay_target {
                let egg_pos = BlockPos {
                    y: target.y + 1,
                    ..target
                };
                let runtime = store.mob_ai_mut(id).expect("turtle runtime");
                if !in_water && closer_to_center(egg_pos, position, 1.0) {
                    runtime.stop_navigation();
                    if state.lay_egg_counter < 1 {
                        state.set_laying_egg(true);
                    } else if state.lay_egg_counter > 200 {
                        use bbb_protocol::block_state_types::vanilla_block_default_state_id;
                        use bbb_protocol::block_types::VANILLA_BLOCK_TURTLE_EGG_ID;
                        let eggs = level.level_random().next_int(4) + 1;
                        let egg = crate::fluid::with_property(
                            vanilla_block_default_state_id(VANILLA_BLOCK_TURTLE_EGG_ID)
                                .expect("turtle egg"),
                            "eggs",
                            &eggs.to_string(),
                        )
                        .expect("turtle egg count");
                        if level.set_block(
                            egg_pos,
                            egg,
                            crate::fluid::SET_BLOCK_UPDATE_ALL,
                            crate::neighbor_updater::UPDATE_LIMIT,
                        ) {
                            let pitch = 0.9 + level.level_random().next_float() * 0.2;
                            level.play_block_sound_with_volume_pitch(
                                block_pos,
                                "minecraft:entity.turtle.lay_egg",
                                0.3,
                                pitch,
                            );
                            level.game_event(VanillaGameEvent::BLOCK_PLACE, egg_pos, Some(id));
                            state.has_egg = false;
                            state.set_laying_egg(false);
                            state.lay_target = None;
                            laid = true;
                        }
                    }
                    if state.laying_egg {
                        state.lay_egg_counter += 1;
                    }
                } else if tick_count % 40 == 0 {
                    runtime.navigate_to(level, mob, egg_pos, 0, 1.0);
                }
                return;
            }
        } else {
            state.lay_target = None;
            state.set_laying_egg(false);
        }
        if state.going_home {
            if closer_to_center(state.home_pos, position, 16.0) {
                state.close_to_home_ticks += 1;
            }
            if closer_to_center(state.home_pos, position, 7.0) || state.close_to_home_ticks > 600 {
                state.going_home = false;
                state.retry_ticks = 200;
                store
                    .mob_ai_mut(id)
                    .expect("turtle runtime")
                    .stop_navigation();
                return;
            }
        } else if state.retry_ticks == 0
            && (state.has_egg
                || (!closer_to_center(state.home_pos, position, 64.0)
                    && level.level_random().next_int(700) == 0))
        {
            state.going_home = true;
            state.close_to_home_ticks = 0;
        }
        if state.going_home && tick_count % 40 == 0 {
            let dx = f64::from(state.home_pos.x) + 0.5 - position.x;
            let dz = f64::from(state.home_pos.z) + 0.5 - position.z;
            let fraction = (12.0 / dx.hypot(dz)).min(1.0);
            let target = BlockPos {
                x: (position.x + dx * fraction).floor() as i32,
                y: (position.y + (f64::from(state.home_pos.y) - position.y).clamp(-3.0, 3.0))
                    .floor() as i32,
                z: (position.z + dz * fraction).floor() as i32,
            };
            if !store
                .mob_ai_mut(id)
                .expect("turtle runtime")
                .navigate_to(level, mob, target, 1, 1.0)
            {
                state.going_home = false;
                state.retry_ticks = 200;
            }
        }
    })();
    let after = (state.has_egg, state.laying_egg);
    if laid {
        if let Some(animal) = data.animal_retained_mut() {
            animal.set_in_love(600, None);
        }
    }
    if before.0 != after.0 {
        entity.mark_synched_dirty(VANILLA_TURTLE_HAS_EGG_DATA_ID);
    }
    if before.1 != after.1 {
        entity.mark_synched_dirty(VANILLA_TURTLE_LAYING_EGG_DATA_ID);
    }
}

fn valid_lay_target(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let above = BlockPos {
        y: pos.y + 1,
        ..pos
    };
    level.has_ticking_column_at(pos)
        && level.has_ticking_column_at(above)
        && block_state_tag_membership(level, level.block_state(pos), "minecraft:sand")
        && bbb_protocol::block_state_types::vanilla_block_state_path_facts(level.block_state(above))
            .is_some_and(|facts| facts.is_air())
}

fn find_lay_target(level: &LevelCtx<'_, '_>, origin: BlockPos) -> Option<BlockPos> {
    for radius in 0_i32..16 {
        for dy in [0, 1, -1] {
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    if dx.abs().max(dz.abs()) != radius {
                        continue;
                    }
                    let target = BlockPos {
                        x: origin.x + dx,
                        y: origin.y - 1 + dy,
                        z: origin.z + dz,
                    };
                    if valid_lay_target(level, target) {
                        return Some(target);
                    }
                }
            }
        }
    }
    None
}

pub(crate) fn after_ai_step(entity: &mut Entity, level: &mut LevelCtx<'_, '_>) -> bool {
    let state = match entity.data() {
        EntityData::GeneralLiving(data) => data.turtle_retained(),
        _ => None,
    };
    let Some(state) = state else {
        return true;
    };
    if !entity.is_alive()
        || !state.laying_egg
        || state.lay_egg_counter < 1
        || state.lay_egg_counter % 5 != 0
    {
        return true;
    }
    let pos = entity.block_position();
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    let state_id = level.block_state(below);
    if !block_state_tag_membership(level, state_id, "minecraft:sand") {
        return true;
    }
    level.level_event(2001, pos, state_id);
    level.game_event(VanillaGameEvent::ENTITY_ACTION, pos, Some(entity.id()));
    true
}

pub(crate) fn travel(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    input: Vec3d,
) -> Result<bool, ()> {
    let retained = match entity.data() {
        EntityData::GeneralLiving(data) => data.turtle_retained(),
        _ => None,
    };
    let Some(retained) = retained else {
        return Ok(false);
    };
    match fluid_contact(entity, level, FluidTag::Water) {
        FluidContact::Known(false) => Ok(false),
        FluidContact::Unknown => Err(()),
        FluidContact::Known(true) => {
            let home_pos = retained.home_pos;
            let going_home = retained.going_home;
            if let Some(node) = store
                .mob_ai(entity.id())
                .and_then(|runtime| runtime.ground_path())
                .and_then(|path| path.nodes().get(path.next_node_index()))
            {
                let desired = Vec3d {
                    x: f64::from(node.pos.x) + 0.5,
                    y: f64::from(node.pos.y),
                    z: f64::from(node.pos.z) + 0.5,
                };
                let direction = Vec3d {
                    x: desired.x - entity.position().x,
                    y: desired.y - entity.position().y,
                    z: desired.z - entity.position().z,
                };
                let distance = (direction.x * direction.x
                    + direction.y * direction.y
                    + direction.z * direction.z)
                    .sqrt();
                let vertical = if distance > 1.0e-5 {
                    f64::from(entity.living().expect("turtle living").speed()) * direction.y
                        / distance
                        * 0.1
                } else {
                    0.0
                };
                entity.set_delta_movement(entity.delta_movement().plus(Vec3d {
                    x: 0.0,
                    y: 0.005 + vertical,
                    z: 0.0,
                }));
            }
            move_relative(entity, 0.1, input);
            let requested = entity.delta_movement();
            crate::entity::movement::move_living_entity(
                entity,
                store,
                level,
                MoverType::Self_,
                requested,
            )?;
            let movement = entity.delta_movement().scale(0.9);
            let near_home = closer_to_center(home_pos, entity.position(), 20.0);
            entity.set_delta_movement(if !going_home || !near_home {
                movement.plus(Vec3d {
                    x: 0.0,
                    y: -0.005,
                    z: 0.0,
                })
            } else {
                movement
            });
            Ok(true)
        }
    }
}

fn closer_to_center(home: BlockPos, position: Vec3d, distance: f64) -> bool {
    let dx = f64::from(home.x) + 0.5 - position.x;
    let dy = f64::from(home.y) + 0.5 - position.y;
    let dz = f64::from(home.z) + 0.5 - position.z;
    dx * dx + dy * dy + dz * dz < distance * distance
}

fn block_position([x, y, z]: [f64; 3]) -> BlockPos {
    BlockPos {
        x: x.floor() as i32,
        y: y.floor() as i32,
        z: z.floor() as i32,
    }
}

fn block_pos_entry(name: &str, pos: BlockPos) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z]),
    }
}

fn boolean_entry(name: &str, value: bool) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Byte(i8::from(value)),
    }
}

#[cfg(test)]
mod tests;
