//! Per-block `BlockBehaviour` tick, placement, shape, neighbor, and
//! interaction dispatch driven by the level schedulers and `block_behavior_owners!` table.

use self::shape_context::block_tag_matches;
use self::shape_context::fluid_tag_matches;
use self::shape_context::BlockShapeCtx;
use crate::block_getter::{BlockGetter, BlockTags};
use crate::world::simulation_level::LevelCtx;
use bbb_server_types::block_queries;

use crate::random::GameRandom;
use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaAffectNeighborsAfterRemovalOverride, VanillaRandomTickOverride,
    VanillaTickOverride, VANILLA_BLOCK_BUBBLE_COLUMN_ID, VANILLA_BLOCK_CREAKING_HEART_ID,
    VANILLA_BLOCK_FARMLAND_ID, VANILLA_BLOCK_ICE_ID, VANILLA_BLOCK_LIGHT_ID,
    VANILLA_BLOCK_NETHER_PORTAL_ID, VANILLA_BLOCK_NOTE_BLOCK_ID, VANILLA_BLOCK_REDSTONE_WIRE_ID,
    VANILLA_BLOCK_SCULK_SHRIEKER_ID, VANILLA_BLOCK_SNOW_ID, VANILLA_BLOCK_TNT_ID,
    VANILLA_BLOCK_TRIPWIRE_ID,
};
use bbb_protocol::packets::{BlockPos, Direction, SoundEvent};

// `above`, `below`, `air_block_state_id`, `block_state_is_solid` and
// `with_property` are [`crate::fluid`]'s own, not copies of them. That module
// was the first consumer of these generated-fact queries and this one is the
// second; a `BlockPos.above()`, a `state.isSolid` and a `state.setValue` mean
// the same thing on both sides, so each gets one implementation and this
// module borrows it.
use crate::fluid::{above, below, with_property};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::fluid::{air_block_state_id, block_state_fluid_state, block_state_is_solid};

pub mod amethyst;
pub mod bamboo;
pub mod banner;
pub mod barrel;
pub mod bed;
pub mod beehive;
pub mod bell;
pub mod big_dripleaf;
pub mod bonemeal;
pub mod brushable;
pub mod bubble_column;
pub mod bucket_pickup;
pub mod button;
pub mod cactus;
pub mod cake;
pub mod campfire;
pub mod candle;
pub mod carpet;
pub mod carved_pumpkin;
pub mod cauldron;
pub mod chain;
pub mod chest;
pub mod chiseled_bookshelf;
pub mod chorus;
pub mod cocoa;
pub mod command_block;
pub mod comparator;
pub mod composter;
pub mod concrete_powder;
pub mod conduit;
pub mod container_openers;
pub mod containers;
pub mod copper_bulb;
pub mod copper_golem_statue;
pub mod coral;
pub mod crafter;
pub mod creaking_heart;
pub mod crop;
pub mod daylight_detector;
pub mod decorated_pot;
pub mod diode;
pub mod dispenser;
pub mod door;
pub mod double_plant;
pub mod dried_ghast;
pub mod ender_chest;
pub mod explosion;
pub mod eyeblossom;
pub mod falling;
pub mod farmland;
pub mod fence;
pub mod fence_gate;
pub mod fire;
pub mod flower_pot;
pub mod footing;
pub mod ground_contact;
pub mod growing_plant;
pub mod honey;
pub mod hopper;
pub mod huge_mushroom;
pub(crate) mod infested;
pub mod inside_contact;
pub mod interaction;
pub mod iron_bars;
pub mod jukebox;
pub mod ladder;
pub mod lantern;
pub mod leaves;
pub mod lectern;
pub mod lever;
pub mod light;
pub mod lightning_rod;
pub mod melting;
pub mod multiface;
pub mod mushroom;
pub mod nether_wart;
pub mod note_block;
pub mod nylium;
pub mod observer;
pub mod piston;
pub mod piston_head;
pub mod pitcher_crop;
pub mod player_break;
pub mod pointed_dripstone;
pub mod pressure_plate;
pub(crate) mod projectile_hit;
pub mod rail;
pub mod redstone_lamp;
pub mod redstone_ore;
pub mod redstone_torch;
pub mod redstone_wire;
pub mod repeater;
pub mod respawn_anchor;
pub mod sapling;
pub mod scaffolding;
pub mod sculk_catalyst;
pub mod sculk_sensor;
pub mod shelf;
pub mod shulker_box;
pub mod sign;
pub mod skull;
pub mod slab;
pub mod sniffer_egg;
pub mod sponge;
pub mod spreading_snowy;
pub mod stair;
pub mod stem;
pub mod sugar_cane;
pub mod sweet_berry_bush;
pub mod target;
pub mod tnt;
pub mod torch;
pub mod trapdoor;
pub mod tripwire;
pub mod tripwire_hook;
pub mod turtle_egg;
pub mod vegetation;
pub mod vine;
pub mod wall;
mod waterlogged;
pub mod weathering;

/// `BlockItem.getPlacementState`, re-exported below.
pub mod placement;

pub mod level;
pub(crate) mod shape_context;

/// The generated-fact queries the owner modules share; re-exported below so
/// every `super::` import in them keeps naming the same items.
pub mod support;

#[cfg(test)]
mod tests;

use bbb_server_types::block_queries::DIRECTION_VALUES;
use bbb_server_types::block_queries::{
    block_type_of, bool_property, can_support_center, can_support_center_at,
    can_support_rigid_block, can_support_rigid_block_at, collision_face_full,
    collision_face_full_at, collision_face_nonempty, collision_face_nonempty_at,
    collision_shape_full_block, direction_serialized_name, face_sturdy, face_sturdy_at,
    int_property, multiface_can_attach_to, multiface_can_attach_to_at, offset,
};
pub use interaction::BlockInteractionPlayer;
pub use level::{
    BlockMobSpawnRequest, BlockPlacedBy, CopperWeatherState, FallingBlockFallRequest,
    FallingBlockHurt, GolemKind, GolemSpawnRequest, InsideEntity, LightLayer,
    SET_BLOCK_UPDATE_CLIENTS, SET_BLOCK_UPDATE_IMMEDIATE, SET_BLOCK_UPDATE_NONE,
};
pub use placement::{
    can_replace_for_placement, can_replace_for_placement_with_registry, state_for_placement,
    state_for_placement_with_view, update_placement_context, BlockPlacementContext,
    BlockPlacementView,
};
pub use support::{
    base_fire_block_type_ids, block_state_with_int, can_survive_override,
    collision_shape_full_block_at, constructor_block_tag, constructor_block_type_id,
    cycle_property, default_block_state_id, direction_property, get_state_for_placement_override,
    light_dampening, may_place_on_override, may_place_on_support_tag, random_horizontal_direction,
    random_tick_override, state_is_block, state_liquid, string_property, tick_override,
};

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// The two shapes a `canSurvive` body answers in: `BlockBehaviour.canSurvive`
/// returns a `boolean`, and an owner that cannot evaluate a state declines
/// with `None` instead. The table's `can_survive:` column takes either.
trait CanSurviveAnswer {
    fn into_answer(self) -> Option<bool>;
}

impl CanSurviveAnswer for bool {
    fn into_answer(self) -> Option<bool> {
        Some(self)
    }
}

impl CanSurviveAnswer for Option<bool> {
    fn into_answer(self) -> Option<bool> {
        self
    }
}

/// The table's `matches:` column, expanded in table order inside
/// [`block_behavior_owner`]. Five kinds, each a generated 26.1 fact: a
/// membership predicate, a resolver that answers for
/// more than one owner at once, the random and scheduled tick class tags, and
/// a `bbb_protocol::block_types` block id.
macro_rules! block_behavior_matches {
    ($state:ident, $owner:expr, predicate $module:ident::$is:ident) => {
        if $module::$is($state) {
            return Some($owner);
        }
    };
    ($state:ident, $owner:expr, resolved_by $module:ident::$resolve:ident) => {
        if $module::$resolve($state) == Some($owner) {
            return Some($owner);
        }
    };
    ($state:ident, $owner:expr, random_tick_override $class:ident) => {
        if block_type_of($state).and_then(random_tick_override)
            == Some(VanillaRandomTickOverride::$class)
        {
            return Some($owner);
        }
    };
    ($state:ident, $owner:expr, tick_override $class:ident) => {
        if block_type_of($state).and_then(tick_override) == Some(VanillaTickOverride::$class) {
            return Some($owner);
        }
    };
    ($state:ident, $owner:expr, block $block_type_id:ident) => {
        if block_type_of($state) == Some($block_type_id) {
            return Some($owner);
        }
    };
}

/// The table's `random_tick:` column.
macro_rules! block_behavior_random_tick {
    ((level, pos, state), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr) => {
        $m::$f($level, $pos, $state)
    };
    ((level, pos), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr) => {
        $m::$f($level, $pos)
    };
}

/// The table's `tick:` column.
macro_rules! block_behavior_tick {
    ((level, pos, state), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr) => {
        $m::$f($level, $pos, $state)
    };
    ((level, pos), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr) => {
        $m::$f($level, $pos)
    };
}

/// The table's `neighbor_changed:` column, whose shapes optionally read the
/// changed source block and experimental redstone orientation.
macro_rules! block_behavior_neighbor_changed {
    ((level, pos, state), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $changed:expr, $orientation:expr) => {
        $m::$f($level, $pos, $state)
    };
    ((level, pos, state, changed_block), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $changed:expr, $orientation:expr) => {
        $m::$f($level, $pos, $state, $changed)
    };
    ((level, pos, state, changed_block, orientation), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $changed:expr, $orientation:expr) => {
        $m::$f($level, $pos, $state, $changed, $orientation)
    };
}

/// The table's `on_place:` column, whose second shape reads the state that was
/// replaced.
macro_rules! block_behavior_on_place {
    ((level, pos, state), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $old:expr) => {
        $m::$f($level, $pos, $state)
    };
    ((level, pos, state, old_state), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $old:expr) => {
        $m::$f($level, $pos, $state, $old)
    };
}

/// The table's `affect_neighbors_after_removal:` column, whose second shape
/// reads the piston flag.
macro_rules! block_behavior_removal {
    ((level, pos, state), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $moved:expr) => {
        $m::$f($level, $pos, $state)
    };
    ((level, pos, state, moved_by_piston), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $moved:expr) => {
        $m::$f($level, $pos, $state, $moved)
    };
}

/// The table's `update_shape:` column, the one hook whose owners read five
/// different slices of the call's arguments.
macro_rules! block_behavior_update_shape {
    ((level, state, direction, neighbour_state), $m:ident, $f:ident, $level:expr, $state:expr, $pos:expr, $dir:expr, $neighbour:expr) => {
        $m::$f($level, $state, $dir, $neighbour)
    };
    ((level, state, pos), $m:ident, $f:ident, $level:expr, $state:expr, $pos:expr, $dir:expr, $neighbour:expr) => {
        $m::$f($level, $state, $pos)
    };
    ((level, state, pos, direction), $m:ident, $f:ident, $level:expr, $state:expr, $pos:expr, $dir:expr, $neighbour:expr) => {
        $m::$f($level, $state, $pos, $dir)
    };
    ((level, state, pos, neighbour_state), $m:ident, $f:ident, $level:expr, $state:expr, $pos:expr, $dir:expr, $neighbour:expr) => {
        $m::$f($level, $state, $pos, $neighbour)
    };
    ((level, state, pos, direction, neighbour_state), $m:ident, $f:ident, $level:expr, $state:expr, $pos:expr, $dir:expr, $neighbour:expr) => {
        $m::$f($level, $state, $pos, $dir, $neighbour)
    };
    ((state, direction, neighbour_state), $m:ident, $f:ident, $level:expr, $state:expr, $pos:expr, $dir:expr, $neighbour:expr) => {
        $m::$f($state, $dir, $neighbour)
    };
    ((state, pos, direction, neighbour_state), $m:ident, $f:ident, $level:expr, $state:expr, $pos:expr, $dir:expr, $neighbour:expr) => {
        $m::$f($state, $pos, $dir, $neighbour)
    };
}

/// The table's `can_survive:` column, through [`CanSurviveAnswer`].
macro_rules! block_behavior_can_survive {
    ((state, pos, at), $m:ident, $f:ident, $state:expr, $pos:expr, $tags:expr, $at:expr) => {
        CanSurviveAnswer::into_answer($m::$f($state, $pos, $at))
    };
    ((pos, at), $m:ident, $f:ident, $state:expr, $pos:expr, $tags:expr, $at:expr) => {
        CanSurviveAnswer::into_answer($m::$f($pos, $at))
    };
    ((state, pos, tags, at), $m:ident, $f:ident, $state:expr, $pos:expr, $tags:expr, $at:expr) => {
        CanSurviveAnswer::into_answer($m::$f($state, $pos, $tags, $at))
    };
    ((pos, tags, at), $m:ident, $f:ident, $state:expr, $pos:expr, $tags:expr, $at:expr) => {
        CanSurviveAnswer::into_answer($m::$f($pos, $tags, $at))
    };
}

/// The table's `can_survive_live:` column: the same `BlockBehaviour.canSurvive`
/// member on the level-taking seam.
macro_rules! block_behavior_can_survive_live {
    ((level, pos), $m:ident, $f:ident, $level:expr, $state:expr, $pos:expr) => {
        $m::$f($level, $pos)
    };
    ((level, state, pos), $m:ident, $f:ident, $level:expr, $state:expr, $pos:expr) => {
        $m::$f($level, $state, $pos)
    };
}

/// The table's `use_item_on:` column.
macro_rules! block_behavior_use_item_on {
    ((state, held), $m:ident, $f:ident, $state:expr, $held:expr, $face:expr) => {
        $m::$f($state, $held)
    };
    ((held, face), $m:ident, $f:ident, $state:expr, $held:expr, $face:expr) => {
        $m::$f($held, $face)
    };
}

/// The table's `use_without_item:` column: the whole acting player, only its
/// `mayBuild`, or nothing of it.
macro_rules! block_behavior_use_without_item {
    ((level, pos, state), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $player:expr) => {
        $m::$f($level, $pos, $state)
    };
    ((level, pos, state, player), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $player:expr) => {
        $m::$f($level, $pos, $state, $player)
    };
    ((level, pos, state, may_build), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $player:expr) => {
        $m::$f($level, $pos, $state, $player.may_build)
    };
}

/// The table's `trigger_event:` column.
macro_rules! block_behavior_trigger_event {
    ((pos, state, random, skull_sound), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $a:expr, $b:expr, $skull:expr) => {
        $m::$f($pos, $state, $level.level_random(), $skull)
    };
    ((level, pos, state, event, data), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $a:expr, $b:expr, $skull:expr) => {
        $m::$f($level, $pos, $state, $a, $b)
    };
}

/// The table's `set_placed_by:` column, whose last shape reads the held stack.
macro_rules! block_behavior_set_placed_by {
    ((level, pos), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $by:expr) => {
        $m::$f($level, $pos)
    };
    ((level, pos, state), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $by:expr) => {
        $m::$f($level, $pos, $state)
    };
    ((level, pos, state, placed_by), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $by:expr) => {
        $m::$f($level, $pos, $state, $by)
    };
}

/// The table's `entity_inside:` column.
macro_rules! block_behavior_entity_inside {
    ((level, pos, state, inside), $m:ident, $f:ident, $level:expr, $pos:expr, $state:expr, $inside:expr) => {
        $m::$f($level, $pos, $state, $inside)
    };
}

/// One row per owner, expanding to [`BlockBehaviorOwner`],
/// [`block_behavior_owner`] and an arm of every dispatcher a row names: a new
/// owner is a new `block_behavior/<owner>.rs` and one row here.
///
/// `matches:` is how a stored state is recognised, and **row order is
/// resolution order**, which is what puts a subclass row in front of its base
/// class. Every other column names the owner module's body for one
/// `BlockBehaviour` member, spelled as the call it becomes; a column left out
/// takes the vanilla default that dispatcher's last arm carries. The
/// per-column macros accept only the argument shapes their hook can hand out,
/// so a wrong call does not compile.
macro_rules! block_behavior_owners {
    ($(
        $(#[$variant_meta:meta])*
        $variant:ident {
            matches: $matcher_kind:ident $($matcher:ident)::+,
            $(random_tick: $rt_m:ident::$rt_f:ident $rt_args:tt,)?
            $(tick: $tk_m:ident::$tk_f:ident $tk_args:tt,)?
            $(neighbor_changed: $nc_m:ident::$nc_f:ident $nc_args:tt,)?
            $(on_place: $op_m:ident::$op_f:ident $op_args:tt,)?
            $(affect_neighbors_after_removal: $rm_m:ident::$rm_f:ident $rm_args:tt,)?
            $(update_shape: $us_m:ident::$us_f:ident $us_args:tt,)?
            $(can_survive: $cs_m:ident::$cs_f:ident $cs_args:tt,)?
            $(can_survive_live: $csl_m:ident::$csl_f:ident $csl_args:tt,)?
            $(use_item_on: $ui_m:ident::$ui_f:ident $ui_args:tt,)?
            $(use_without_item: $uw_m:ident::$uw_f:ident $uw_args:tt,)?
            $(trigger_event: $te_m:ident::$te_f:ident $te_args:tt,)?
            $(entity_inside: $ei_m:ident::$ei_f:ident $ei_args:tt,)?
            $(set_placed_by: $sp_m:ident::$sp_f:ident $sp_args:tt,)?
        }
    ),* $(,)?) => {
        /// The vanilla class that owns a block state's behaviour in this slice.
        ///
        /// One variant per official owner, not one per block: `grass_block` and
        /// `mycelium` share `SpreadingSnowyBlock`, and the five crops share
        /// `CropBlock` with their own overrides folded into
        /// [`crop::CropKind`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum BlockBehaviorOwner {
            $(
                $(#[$variant_meta])*
                $variant,
            )*
        }

        /// The owners whose row carries each generated-catalog-backed column,
        /// paired with the catalog member the column claims to implement.
        /// `bbb-server-world`'s block-use pin reads it through `test-support`
        /// to hold the two use columns against the catalog jointly with its
        /// own interaction table.
        #[cfg(test)]
        pub const BLOCK_BEHAVIOR_OWNER_HOOKS: &[(&str, &[(BlockBehaviorOwner, &str)])] = &[
            (
                "random_tick",
                &[$($((BlockBehaviorOwner::$variant, stringify!($rt_m::$rt_f)),)?)*],
            ),
            (
                "tick",
                &[$($((BlockBehaviorOwner::$variant, stringify!($tk_m::$tk_f)),)?)*],
            ),
            (
                "neighbor_changed",
                &[$($((BlockBehaviorOwner::$variant, stringify!($nc_m::$nc_f)),)?)*],
            ),
            (
                "on_place",
                &[$($((BlockBehaviorOwner::$variant, stringify!($op_m::$op_f)),)?)*],
            ),
            (
                "affect_neighbors_after_removal",
                &[$($((BlockBehaviorOwner::$variant, stringify!($rm_m::$rm_f)),)?)*],
            ),
            (
                "can_survive",
                &[$($((BlockBehaviorOwner::$variant, stringify!($cs_m::$cs_f)),)?)*],
            ),
            (
                "can_survive_live",
                &[$($((BlockBehaviorOwner::$variant, stringify!($csl_m::$csl_f)),)?)*],
            ),
            (
                "use_item_on",
                &[$($((BlockBehaviorOwner::$variant, stringify!($ui_m::$ui_f)),)?)*],
            ),
            (
                "use_without_item",
                &[$($((BlockBehaviorOwner::$variant, stringify!($uw_m::$uw_f)),)?)*],
            ),
        ];

        /// Routes a stored global block-state id to its behaviour owner, or `None`
        /// when this slice has nothing for it.
        ///
        /// Every arm is derived: leaves come from the generated `leaves_block` class
        /// bit, crops and the three survival families from the generated
        /// `can_survive_override` class tag, the two snowy owners from the generated
        /// `get_state_for_placement_override` class tag, and the three single-block
        /// owners from `bbb_protocol::block_types` id constants.
        #[must_use]
        pub fn block_behavior_owner(block_state_id: i32) -> Option<BlockBehaviorOwner> {
            static OWNERS: std::sync::OnceLock<Vec<Option<BlockBehaviorOwner>>> =
                std::sync::OnceLock::new();
            let block_id = usize::try_from(block_type_of(block_state_id)?).ok()?;
            OWNERS.get_or_init(|| {
                bbb_protocol::block_types::VANILLA_BLOCK_TYPES
                    .iter()
                    .map(|block| classify_block_behavior(default_block_state_id(block.id)))
                    .collect()
            }).get(block_id).copied().flatten()
        }

        fn classify_block_behavior(block_state_id: i32) -> Option<BlockBehaviorOwner> {
            $(
                block_behavior_matches!(
                    block_state_id,
                    BlockBehaviorOwner::$variant,
                    $matcher_kind $($matcher)::+
                );
            )*
            None
        }

        pub(crate) fn random_tick_block(
            level: &mut LevelCtx<'_, '_>,
            pos: BlockPos,
            block_state_id: i32,
        ) {
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_random_tick!(
                        $rt_args, $rt_m, $rt_f, level, pos, block_state_id
                    ),
                )?)*
                _ => {}
            }
        }

        pub(crate) fn tick_block(
            level: &mut LevelCtx<'_, '_>,
            pos: BlockPos,
            block_state_id: i32,
        ) {
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_tick!(
                        $tk_args, $tk_m, $tk_f, level, pos, block_state_id
                    ),
                )?)*
                _ => {}
            }
        }

        pub(crate) fn neighbor_changed(
            level: &mut LevelCtx<'_, '_>,
            pos: BlockPos,
            block_state_id: i32,
            changed_block_type_id: i32,
            orientation: Option<redstone_wire::experimental::RedstoneOrientation>,
            moved_by_piston: bool,
        ) {
            let _ = (orientation, moved_by_piston);
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_neighbor_changed!(
                        $nc_args, $nc_m, $nc_f, level, pos, block_state_id, changed_block_type_id, orientation
                    ),
                )?)*
                _ => {}
            }
        }

        ///
        ///
        /// `moved_by_piston` stays in the signature because it is the call's own
        /// argument; no owner here reads it.
        pub(crate) fn on_place(
            level: &mut LevelCtx<'_, '_>,
            pos: BlockPos,
            block_state_id: i32,
            old_state: i32,
            moved_by_piston: bool,
        ) {
            let _ = moved_by_piston;
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_on_place!(
                        $op_args, $op_m, $op_f, level, pos, block_state_id, old_state
                    ),
                )?)*
                _ => {}
            }
        }

        ///
        ///
        /// `block_state_id` is the **old** state, the one that was removed.
        pub(crate) fn affect_neighbors_after_removal(
            level: &mut LevelCtx<'_, '_>,
            pos: BlockPos,
            block_state_id: i32,
            moved_by_piston: bool,
        ) {
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_removal!(
                        $rm_args, $rm_m, $rm_f, level, pos, block_state_id, moved_by_piston
                    ),
                )?)*
                _ => {}
            }
        }

        pub(crate) fn update_block_shape(
            level: &mut BlockShapeCtx<'_, '_, '_>,
            block_state_id: i32,
            pos: BlockPos,
            direction_to_neighbour: Direction,
            neighbour_pos: BlockPos,
            neighbour_state: i32,
        ) -> i32 {
            let _ = neighbour_pos;
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_update_shape!(
                        $us_args,
                        $us_m,
                        $us_f,
                        level,
                        block_state_id,
                        pos,
                        direction_to_neighbour,
                        neighbour_state
                    ),
                )?)*
                _ => block_state_id,
            }
        }

        #[must_use]
        pub fn can_survive(
            block_state_id: i32,
            pos: BlockPos,
            tags: &dyn BlockTags,
            block_state_at: BlockStateAt<'_>,
        ) -> Option<bool> {
            let _ = tags;
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_can_survive!(
                        $cs_args, $cs_m, $cs_f, block_state_id, pos, tags, block_state_at
                    ),
                )?)*
                _ => None,
            }
        }

        /// The same member over a live [`BlockGetter`] instead of a
        /// [`BlockStateAt`], for the owners whose body reads more than the one
        /// stored state below: a shape, a built-in tag or the light.
        ///
        /// A row carrying no `can_survive_live:` column answers through
        /// [`can_survive`] over the same level, so the two columns are one
        /// member with two entry points rather than two behaviours.
        fn can_survive_live<L: BlockGetter + ?Sized>(
            block_state_id: i32,
            pos: BlockPos,
            level: &L,
        ) -> Option<bool> {
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_can_survive_live!(
                        $csl_args, $csl_m, $csl_f, level, block_state_id, pos
                    ),
                )?)*
                _ => can_survive(block_state_id, pos, level.tags(), &mut |at| {
                    level.block_state(at)
                }),
            }
        }

        #[must_use]
        pub fn use_item_on(
            block_state_id: i32,
            held_item_type_id: Option<i32>,
            face: Direction,
        ) -> BlockUseResult {
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_use_item_on!(
                        $ui_args, $ui_m, $ui_f, block_state_id, held_item_type_id, face
                    ),
                )?)*
                _ => BlockUseResult::TryWithEmptyHand,
            }
        }

        ///
        ///
        /// `player` is the `Player` argument; every game event the body
        /// raises carries it as the source entity.
        pub(crate) fn use_without_item(
            level: &mut LevelCtx<'_, '_>,
            pos: BlockPos,
            block_state_id: i32,
            player: BlockInteractionPlayer,
        ) -> bool {
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_use_without_item!(
                        $uw_args, $uw_m, $uw_f, level, pos, block_state_id, player
                    ),
                )?)*
                _ => false,
            }
        }

        pub(crate) fn set_placed_by(
            level: &mut LevelCtx<'_, '_>,
            pos: BlockPos,
            block_state_id: i32,
            placed_by: &BlockPlacedBy<'_>,
        ) {
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_set_placed_by!(
                        $sp_args, $sp_m, $sp_f, level, pos, block_state_id, placed_by
                    ),
                )?)*
                _ => {}
            }
        }

        #[must_use]
        pub(crate) fn trigger_event_with_level(
            level: &mut LevelCtx<'_, '_>,
            pos: BlockPos,
            block_state_id: i32,
            param_a: i32,
            param_b: i32,
            skull_note_block_sound_above: Option<&str>,
        ) -> BlockTriggerOutcome {
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_trigger_event!(
                        $te_args,
                        $te_m,
                        $te_f,
                        level,
                        pos,
                        block_state_id,
                        param_a,
                        param_b,
                        skull_note_block_sound_above
                    ),
                )?)*
                _ => BlockTriggerOutcome::not_triggered(),
            }
        }

        #[must_use]
        pub fn trigger_event(
            pos: BlockPos,
            block_state_id: i32,
            param_a: i32,
            param_b: i32,
            random: &GameRandom,
            skull_note_block_sound_above: Option<&str>,
        ) -> BlockTriggerOutcome {
            let _ = (param_a, param_b);
            match block_behavior_owner(block_state_id) {
                Some(BlockBehaviorOwner::NoteBlock) => note_block::trigger_event(
                    pos,
                    block_state_id,
                    random,
                    skull_note_block_sound_above,
                ),
                _ => BlockTriggerOutcome::not_triggered(),
            }
        }

        #[must_use]
        pub(crate) fn entity_inside(
            level: &mut LevelCtx<'_, '_>,
            pos: BlockPos,
            block_state_id: i32,
            inside: InsideEntity,
        ) -> bool {
            match block_behavior_owner(block_state_id) {
                $($(
                    Some(BlockBehaviorOwner::$variant) => block_behavior_entity_inside!(
                        $ei_args, $ei_m, $ei_f, level, pos, block_state_id, inside
                    ),
                )?)*
                _ => false,
            }
        }
    };
}

block_behavior_owners! {
    Beehive {
        matches: predicate beehive::is_beehive,
        update_shape: beehive::update_shape(level, state, pos, neighbour_state),
    },
    Campfire {
        matches: predicate campfire::is_campfire,
        update_shape: campfire::update_shape(level, state, pos, direction, neighbour_state),
    },
    Leaves {
        matches: predicate block_queries::is_leaves,
        random_tick: leaves::random_tick(level, pos, state),
        tick: leaves::tick(level, pos, state),
        update_shape: leaves::update_shape(level, state, pos, neighbour_state),
    },
    Crop {
        matches: predicate crop::is_crop,
        random_tick: crop::random_tick(level, pos, state),
        update_shape: crop::update_shape(level, state, pos),
        can_survive_live: crop::can_survive_live(level, pos),
    },
    Fire {
        matches: tick_override FireBlock,
        tick: fire::tick(level, pos, state),
        on_place: fire::on_place(level, pos, state, old_state),
        update_shape: fire::update_shape(level, state, pos),
        can_survive: fire::can_survive(state, pos, at),
        can_survive_live: fire::can_survive_live(level, pos),
    },
    NetherPortal {
        matches: block VANILLA_BLOCK_NETHER_PORTAL_ID,
        update_shape: fire::portal_update_shape(level, state, pos, direction, neighbour_state),
    },
    Tnt {
        matches: block VANILLA_BLOCK_TNT_ID,
        neighbor_changed: tnt::neighbor_changed(level, pos, state),
        on_place: tnt::on_place(level, pos, state, old_state),
    },
    CarvedPumpkin {
        matches: predicate carved_pumpkin::is_carved_pumpkin,
        on_place: carved_pumpkin::on_place(level, pos, state, old_state),
    },
    Brushable {
        matches: predicate brushable::is_brushable,
        tick: brushable::tick(level, pos, state),
        on_place: brushable::on_place(level, pos, state),
        update_shape: brushable::update_shape(level, state, pos),
    },

    Sapling {
        matches: random_tick_override SaplingBlock,
        random_tick: sapling::random_tick(level, pos, state),
        update_shape: vegetation::update_shape(level, state, pos),
        can_survive: vegetation::can_survive(state, pos, tags, at),
    },
    SugarCane {
        matches: random_tick_override SugarCaneBlock,
        random_tick: sugar_cane::random_tick(level, pos, state),
        tick: sugar_cane::tick(level, pos, state),
        update_shape: sugar_cane::update_shape(level, state, pos),
        can_survive: sugar_cane::can_survive(state, pos, tags, at),
    },
    Cactus {
        matches: random_tick_override CactusBlock,
        random_tick: cactus::random_tick(level, pos, state),
        tick: cactus::tick(level, pos, state),
        update_shape: cactus::update_shape(level, state, pos),
        can_survive: cactus::can_survive(state, pos, tags, at),
    },
    NetherWart {
        matches: random_tick_override NetherWartBlock,
        random_tick: nether_wart::random_tick(level, pos, state),
        update_shape: vegetation::update_shape(level, state, pos),
        can_survive: vegetation::can_survive(state, pos, tags, at),
        can_survive_live: vegetation::can_survive_live(level, state, pos),
    },
    Cocoa {
        matches: random_tick_override CocoaBlock,
        random_tick: cocoa::random_tick(level, pos, state),
        update_shape: cocoa::update_shape(level, state, pos, direction),
        can_survive: cocoa::can_survive(state, pos, tags, at),
    },
    Stem {
        matches: random_tick_override StemBlock,
        random_tick: stem::random_tick(level, pos, state),
        update_shape: vegetation::update_shape(level, state, pos),
        can_survive: vegetation::can_survive(state, pos, tags, at),
        can_survive_live: vegetation::can_survive_live(level, state, pos),
    },
    BambooStalk {
        matches: random_tick_override BambooStalkBlock,
        random_tick: bamboo::stalk_random_tick(level, pos, state),
        tick: bamboo::stalk_tick(level, pos),
        update_shape: bamboo::stalk_update_shape(level, state, pos, direction, neighbour_state),
        can_survive: bamboo::can_survive(pos, tags, at),
    },
    BambooSapling {
        matches: random_tick_override BambooSaplingBlock,
        random_tick: bamboo::sapling_random_tick(level, pos),
        update_shape: bamboo::sapling_update_shape(level, state, pos, direction, neighbour_state),
        can_survive: bamboo::can_survive(pos, tags, at),
    },
    Vine {
        matches: random_tick_override VineBlock,
        random_tick: vine::random_tick(level, pos, state),
        update_shape: vine::update_shape(level, state, pos, direction),
        can_survive: vine::can_survive(state, pos, at),
        can_survive_live: vine::can_survive_live(level, state, pos),
    },
    Eyeblossom {
        matches: random_tick_override EyeblossomBlock,
        random_tick: eyeblossom::random_tick(level, pos, state),
        tick: eyeblossom::tick(level, pos, state),
        update_shape: vegetation::update_shape(level, state, pos),
        can_survive: vegetation::can_survive(state, pos, tags, at),
        can_survive_live: vegetation::can_survive_live(level, state, pos),
    },
    SweetBerryBush {
        matches: random_tick_override SweetBerryBushBlock,
        random_tick: sweet_berry_bush::random_tick(level, pos, state),
        update_shape: vegetation::update_shape(level, state, pos),
        can_survive: vegetation::can_survive(state, pos, tags, at),
        can_survive_live: vegetation::can_survive_live(level, state, pos),
    },
    Nylium {
        matches: random_tick_override NyliumBlock,
        random_tick: nylium::random_tick(level, pos, state),
    },
    BuddingAmethyst {
        matches: random_tick_override BuddingAmethystBlock,
        random_tick: amethyst::random_tick(level, pos),
    },
    AmethystCluster {
        matches: predicate amethyst::is_amethyst_cluster,
        // No `random_tick:`: only the budding block declares one.
        update_shape: amethyst::update_shape(level, state, pos, direction),
        can_survive: amethyst::can_survive(state, pos, at),
        can_survive_live: amethyst::can_survive_live(level, state, pos),
    },
    Mushroom {
        matches: random_tick_override MushroomBlock,
        random_tick: mushroom::random_tick(level, pos, state),
        update_shape: mushroom::update_shape(level, state, pos),
        can_survive_live: mushroom::can_survive_live(level, pos),
    },
    HugeMushroom {
        matches: predicate huge_mushroom::is_huge_mushroom,
        update_shape: huge_mushroom::update_shape(state, direction, neighbour_state),
    },
    ConcretePowder {
        matches: predicate concrete_powder::is_concrete_powder,
        on_place: falling::on_place(level, pos, state),
        update_shape: concrete_powder::update_shape(level, state, pos),
    },
    /// The seven remaining `FallingBlock` registrations: sand, red sand,
    /// gravel, the dragon egg and the three anvils.
    ///
    Falling {
        matches: predicate falling::is_falling,
        on_place: falling::on_place(level, pos, state),
        update_shape: falling::update_shape(level, state, pos),
    },
    PitcherCrop {
        matches: random_tick_override PitcherCropBlock,
        random_tick: pitcher_crop::random_tick(level, pos, state),
        update_shape: pitcher_crop::update_shape(level, state, pos, direction, neighbour_state),
        can_survive_live: pitcher_crop::can_survive_live(level, state, pos),
    },
    GrowingPlantHead {
        matches: predicate growing_plant::is_head,
        random_tick: growing_plant::head_random_tick(level, pos, state),
        tick: growing_plant::tick(level, pos, state),
        update_shape: growing_plant::head_update_shape(level, state, pos, direction, neighbour_state),
        can_survive: growing_plant::can_survive(state, pos, tags, at),
        can_survive_live: growing_plant::can_survive_live(level, state, pos),
    },
    GrowingPlantBody {
        matches: predicate growing_plant::is_body,
        // No `random_tick:`: only the head half declares one, which is why the
        // four body blocks carry no `random_tick_override` at all.
        tick: growing_plant::tick(level, pos, state),
        update_shape: growing_plant::body_update_shape(level, state, pos, direction, neighbour_state),
        can_survive: growing_plant::can_survive(state, pos, tags, at),
        can_survive_live: growing_plant::can_survive_live(level, state, pos),
    },
    FlowerPot {
        matches: predicate flower_pot::is_flower_pot,
        random_tick: flower_pot::random_tick(level, pos, state),
    },
    ChorusFlower {
        matches: resolved_by chorus::owner,
        random_tick: chorus::flower_random_tick(level, pos, state),
        tick: chorus::flower_tick(level, pos, state),
        update_shape: chorus::flower_update_shape(level, state, pos, direction),
        can_survive: chorus::flower_can_survive(pos, tags, at),
    },
    ChorusPlant {
        matches: resolved_by chorus::owner,
        // No `random_tick:`: only the flower half declares one.
        tick: chorus::plant_tick(level, pos, state),
        update_shape: chorus::plant_update_shape(level, state, pos, direction, neighbour_state),
        can_survive: chorus::plant_can_survive(pos, tags, at),
    },
    DoublePlant {
        matches: predicate double_plant::is_double_plant,
        update_shape: double_plant::plain_update_shape(level, state, pos, direction, neighbour_state),
        can_survive: double_plant::can_survive(state, pos, tags, at),
        set_placed_by: double_plant::set_placed_by(level, pos, state),
    },
    TallSeagrass {
        matches: predicate double_plant::is_tall_seagrass,
        update_shape: double_plant::tall_seagrass_update_shape(level, state, pos, direction, neighbour_state),
        can_survive: double_plant::tall_seagrass_can_survive(state, pos, tags, at),
        set_placed_by: double_plant::set_placed_by(level, pos, state),
    },
    SmallDripleaf {
        matches: predicate vegetation::is_small_dripleaf,
        update_shape: vegetation::small_dripleaf_update_shape(level, state, pos, direction, neighbour_state),
        can_survive: vegetation::small_dripleaf_can_survive(state, pos, tags, at),
        set_placed_by: vegetation::small_dripleaf_set_placed_by(level, pos, state),
    },
    AttachedStem {
        matches: predicate stem::is_attached_stem,
        update_shape: stem::attached_update_shape(level, state, pos, direction, neighbour_state),
        can_survive: vegetation::can_survive(state, pos, tags, at),
    },
    Seagrass {
        matches: predicate vegetation::is_seagrass,
        update_shape: vegetation::seagrass_update_shape(level, state, pos),
        can_survive: vegetation::can_survive(state, pos, tags, at),
        can_survive_live: vegetation::can_survive_live(level, state, pos),
    },
    Vegetation {
        matches: predicate vegetation::is_vegetation,
        // No `random_tick:`: the remaining family members with one require
        // worldgen or owner-specific growth bodies.
        update_shape: vegetation::update_shape(level, state, pos),
        can_survive: vegetation::can_survive(state, pos, tags, at),
        can_survive_live: vegetation::can_survive_live(level, state, pos),
    },
    Carpet {
        matches: predicate carpet::is_carpet,
        update_shape: carpet::update_shape(level, state, pos),
        can_survive: carpet::can_survive(pos, at),
    },
    Candle {
        matches: predicate candle::is_candle,
        update_shape: candle::update_shape(level, state, pos),
        can_survive: candle::can_survive(pos, at),
        can_survive_live: candle::can_survive_live(level, pos),
    },
    Cake {
        matches: predicate cake::is_cake,
        update_shape: cake::update_shape(level, state, pos, direction),
        can_survive: cake::can_survive(pos, at),
    },
    CopperBulb {
        matches: predicate copper_bulb::is_copper_bulb,
        // Before the `Weathering` row: unwaxed bulbs oxidize through the
        // same `changeOverTime` body, but `checkAndFlip` is `CopperBulbBlock`'s
        // and must not be swallowed by it.
        random_tick: copper_bulb::random_tick(level, pos, state),
        neighbor_changed: copper_bulb::neighbor_changed(level, pos, state),
        on_place: copper_bulb::on_place(level, pos, state, old_state),
    },
    Composter {
        matches: predicate composter::is_composter,
        tick: composter::tick(level, pos, state),
        on_place: composter::on_place(level, pos, state),
    },
    CopperChest {
        matches: predicate chest::is_copper_chest,
        // Before the `Weathering` row, for the reason the `CopperBulb` row is:
        // the chest bodies are `CopperChestBlock`'s and must not be swallowed
        // by the bare `changeOverTime` one.
        random_tick: weathering::chest_random_tick(level, pos, state),
        tick: chest::tick(level, pos, state),
        affect_neighbors_after_removal: containers::update_neighbours_after_destroy(level, pos, state),
        update_shape: chest::copper_update_shape(level, state, pos, direction, neighbour_state),
    },
    Chest {
        matches: predicate chest::is_plain_chest,
        // No `random_tick:`: `ChestBlock` declares none and neither block is
        // registered with `randomTicks()`.
        tick: chest::tick(level, pos, state),
        affect_neighbors_after_removal: containers::update_neighbours_after_destroy(level, pos, state),
        update_shape: chest::update_shape(level, state, pos, direction, neighbour_state),
    },
    LightningRod {
        matches: tick_override LightningRodBlock,
        random_tick: weathering::random_tick(level, pos, state),
        tick: lightning_rod::tick(level, pos, state),
        on_place: lightning_rod::on_place(level, pos, state, old_state),
        affect_neighbors_after_removal: lightning_rod::affect_neighbors_after_removal(level, pos, state),
        update_shape: lightning_rod::update_shape(level, state, pos),
    },
    DriedGhast {
        matches: tick_override DriedGhastBlock,
        random_tick: dried_ghast::random_tick(level, pos, state),
        tick: dried_ghast::tick(level, pos, state),
        update_shape: dried_ghast::update_shape(level, state, pos),
        set_placed_by: dried_ghast::set_placed_by(level, pos, state),
    },
    PointedDripstone {
        matches: tick_override PointedDripstoneBlock,
        random_tick: pointed_dripstone::random_tick(level, pos, state),
        tick: pointed_dripstone::tick(level, pos, state),
        update_shape: pointed_dripstone::update_shape(level, state, pos, direction),
        can_survive: pointed_dripstone::can_survive(state, pos, at),
        can_survive_live: pointed_dripstone::can_survive_live(level, state, pos),
    },
    Weathering {
        matches: resolved_by weathering::weathering_owner,
        // Before the `Door` row: all eight copper doors answer
        // `can_survive_override == DoorBlock`, and the four unwaxed ones have
        // a `randomTick` the plain door has not.
        random_tick: weathering::random_tick(level, pos, state),
        // No `can_survive:`: the one base that declares one is `LanternBlock`,
        // and its four copper registrations are taken by the
        // `WeatheringCopperLantern` row instead.
    },
    WeatheringCopperBars {
        matches: resolved_by weathering::weathering_owner,
        random_tick: weathering::random_tick(level, pos, state),
        update_shape: iron_bars::update_shape(level, state, pos, direction, neighbour_state),
    },
    WeatheringCopperChain {
        matches: resolved_by weathering::weathering_owner,
        random_tick: weathering::random_tick(level, pos, state),
        update_shape: waterlogged::update_shape(level, state, pos),
    },
    WeatheringCopperSlab {
        matches: resolved_by weathering::weathering_owner,
        random_tick: weathering::random_tick(level, pos, state),
        update_shape: slab::update_shape(level, state, pos),
    },
    WeatheringCopperStair {
        matches: resolved_by weathering::weathering_owner,
        random_tick: weathering::random_tick(level, pos, state),
        update_shape: stair::update_shape(level, state, pos, direction),
    },
    WeatheringCopperGolemStatue {
        matches: resolved_by weathering::weathering_owner,
        random_tick: weathering::random_tick(level, pos, state),
        affect_neighbors_after_removal: copper_golem_statue::affect_neighbors_after_removal(level, pos, state),
        update_shape: copper_golem_statue::update_shape(level, state, pos),
    },
    WeatheringCopperGrate {
        matches: resolved_by weathering::weathering_owner,
        random_tick: weathering::random_tick(level, pos, state),
        update_shape: waterlogged::update_shape(level, state, pos),
    },
    WeatheringCopperDoor {
        matches: resolved_by weathering::weathering_owner,
        random_tick: weathering::door_random_tick(level, pos, state),
        neighbor_changed: door::neighbor_changed(level, pos, state, changed_block),
        update_shape: door::update_shape(level, state, pos, direction, neighbour_state),
        can_survive: door::can_survive(state, pos, at),
        can_survive_live: door::can_survive_live(level, state, pos),
        use_without_item: door::use_without_item(level, pos, state, player),
        set_placed_by: door::set_placed_by(level, pos, state),
    },
    WeatheringCopperTrapDoor {
        matches: resolved_by weathering::weathering_owner,
        random_tick: weathering::random_tick(level, pos, state),
        neighbor_changed: trapdoor::neighbor_changed(level, pos, state),
        update_shape: trapdoor::update_shape(level, state, pos),
        use_without_item: trapdoor::use_without_item(level, pos, state, player),
    },
    Door {
        matches: predicate door::is_door,
        neighbor_changed: door::neighbor_changed(level, pos, state, changed_block),
        update_shape: door::update_shape(level, state, pos, direction, neighbour_state),
        can_survive: door::can_survive(state, pos, at),
        can_survive_live: door::can_survive_live(level, state, pos),
        use_without_item: door::use_without_item(level, pos, state, player),
        set_placed_by: door::set_placed_by(level, pos, state),
    },
    TrapDoor {
        matches: predicate redstone_wire::is_trapdoor,
        neighbor_changed: trapdoor::neighbor_changed(level, pos, state),
        update_shape: trapdoor::update_shape(level, state, pos),
        use_without_item: trapdoor::use_without_item(level, pos, state, player),
    },
    IronBars {
        matches: predicate iron_bars::is_iron_bars,
        update_shape: iron_bars::update_shape(level, state, pos, direction, neighbour_state),
    },
    Chain {
        matches: predicate chain::is_chain,
        update_shape: waterlogged::update_shape(level, state, pos),
    },
    Slab {
        matches: predicate slab::is_slab,
        update_shape: slab::update_shape(level, state, pos),
    },
    Stair {
        matches: predicate stair::is_stair,
        update_shape: stair::update_shape(level, state, pos, direction),
    },
    CopperGolemStatue {
        matches: predicate copper_golem_statue::is_copper_golem_statue,
        affect_neighbors_after_removal: copper_golem_statue::affect_neighbors_after_removal(level, pos, state),
        update_shape: copper_golem_statue::update_shape(level, state, pos),
    },
    Waterlogged {
        matches: predicate waterlogged::is_waterlogged_only,
        update_shape: waterlogged::update_shape(level, state, pos),
    },
    Conduit {
        matches: predicate conduit::is_conduit,
        update_shape: waterlogged::update_shape(level, state, pos),
    },
    Bed {
        matches: predicate bed::is_bed,
        update_shape: bed::update_shape(state, direction, neighbour_state),
        set_placed_by: bed::set_placed_by(level, pos, state),
    },
    Fence {
        matches: predicate fence::is_fence,
        update_shape: fence::update_shape(level, state, pos, direction, neighbour_state),
    },
    FenceGate {
        matches: predicate footing::is_fence_gate,
        neighbor_changed: fence_gate::neighbor_changed(level, pos, state),
        update_shape: fence_gate::update_shape(level, state, pos, direction, neighbour_state),
        use_without_item: fence_gate::use_without_item(level, pos, state, player),
    },
    Wall {
        matches: predicate wall::is_wall,
        update_shape: wall::update_shape(level, state, pos, direction, neighbour_state),
    },
    SpreadingSnowy {
        matches: predicate spreading_snowy::is_spreading,
        // Before the `Snowy` row it subclasses: the base answers
        // `updateShape`, the subclass adds `randomTick`, which is what keeps
        // `podzol`'s `SNOWY` updating without giving it a spread.
        random_tick: spreading_snowy::random_tick(level, pos, state),
        update_shape: spreading_snowy::update_shape(level, state, direction, neighbour_state),
    },
    Snowy {
        matches: predicate spreading_snowy::is_snowy,
        update_shape: spreading_snowy::update_shape(level, state, direction, neighbour_state),
    },
    Rail {
        matches: predicate rail::is_rail,
        tick: rail::detector_tick(level, pos, state),
        neighbor_changed: rail::neighbor_changed(level, pos, state, changed_block),
        on_place: rail::on_place(level, pos, state, old_state),
        affect_neighbors_after_removal: rail::affect_neighbors_after_removal(level, pos, state, moved_by_piston),
        update_shape: rail::update_shape(level, state, pos),
        can_survive: rail::can_survive(pos, at),
        can_survive_live: rail::can_survive_live(level, pos),
        entity_inside: rail::detector_entity_inside(level, pos, state, inside),
    },
    RedstoneLamp {
        matches: predicate redstone_lamp::is_redstone_lamp,
        tick: redstone_lamp::tick(level, pos, state),
        neighbor_changed: redstone_lamp::neighbor_changed(level, pos, state),
    },
    Observer {
        matches: tick_override ObserverBlock,
        tick: observer::tick(level, pos, state),
        on_place: observer::on_place(level, pos, state, old_state),
        affect_neighbors_after_removal: observer::affect_neighbors_after_removal(level, pos, state),
        update_shape: observer::update_shape(level, state, pos, direction),
    },
    Target {
        matches: tick_override TargetBlock,
        tick: target::tick(level, pos, state),
        on_place: target::on_place(level, pos, state, old_state),
    },
    RedstoneTorch {
        matches: predicate redstone_torch::is_redstone_torch,
        tick: redstone_torch::tick(level, pos, state),
        neighbor_changed: redstone_torch::neighbor_changed(level, pos, state),
        on_place: redstone_torch::on_place(level, pos, state),
        affect_neighbors_after_removal: redstone_torch::affect_neighbors_after_removal(level, pos, state, moved_by_piston),
        update_shape: redstone_torch::update_shape(level, state, pos, direction),
        can_survive: redstone_torch::can_survive(state, pos, at),
        can_survive_live: redstone_torch::can_survive_live(level, state, pos),
    },
    RedstoneOre {
        matches: predicate redstone_ore::is_redstone_ore,
        random_tick: redstone_ore::random_tick(level, pos, state),
    },
    Repeater {
        matches: predicate repeater::is_repeater,
        tick: diode::tick(level, pos, state),
        neighbor_changed: diode::neighbor_changed(level, pos, state),
        on_place: diode::on_place(level, pos, state),
        affect_neighbors_after_removal: diode::affect_neighbors_after_removal(level, pos, state, moved_by_piston),
        update_shape: repeater::update_shape(level, state, pos, direction, neighbour_state),
        can_survive: diode::can_survive(pos, at),
        can_survive_live: diode::can_survive_live(level, pos),
        use_without_item: repeater::use_without_item(level, pos, state, may_build),
        set_placed_by: diode::set_placed_by(level, pos, state),
    },
    Comparator {
        matches: predicate comparator::is_comparator,
        tick: comparator::tick(level, pos, state),
        neighbor_changed: comparator::neighbor_changed(level, pos, state),
        on_place: comparator::on_place(level, pos, state),
        affect_neighbors_after_removal: comparator::affect_neighbors_after_removal(level, pos, state, moved_by_piston),
        update_shape: comparator::update_shape(state, pos, direction, neighbour_state),
        can_survive: comparator::can_survive(pos, at),
        can_survive_live: diode::can_survive_live(level, pos),
        use_without_item: comparator::use_without_item(level, pos, state, player),
        set_placed_by: comparator::set_placed_by(level, pos, state),
    },
    DaylightDetector {
        matches: predicate daylight_detector::is_daylight_detector,
        use_without_item: daylight_detector::use_without_item(level, pos, state, player),
    },
    Hopper {
        matches: predicate hopper::is_hopper,
        neighbor_changed: hopper::neighbor_changed(level, pos, state),
        on_place: hopper::on_place(level, pos, state, old_state),
        affect_neighbors_after_removal: containers::update_neighbours_after_destroy(level, pos, state),
    },
    CommandBlock {
        matches: predicate command_block::is_command_block,
        neighbor_changed: command_block::neighbor_changed(level, pos, state),
    },
    Crafter {
        matches: predicate crafter::is_crafter,
        neighbor_changed: crafter::neighbor_changed(level, pos, state),
        affect_neighbors_after_removal: containers::update_neighbours_after_destroy(level, pos, state),
        set_placed_by: crafter::set_placed_by(level, pos, state),
    },
    /// Dispenser scheduling; World owns the live container transfer.
    Dispenser {
        matches: predicate dispenser::is_dispenser_family,
        neighbor_changed: dispenser::neighbor_changed(level, pos, state),
        affect_neighbors_after_removal: containers::update_neighbours_after_destroy(level, pos, state),
    },
    Lectern {
        matches: predicate lectern::is_lectern,
        tick: lectern::tick(level, pos, state),
        affect_neighbors_after_removal: lectern::affect_neighbors_after_removal(level, pos, state),
    },
    ChiseledBookshelf {
        matches: predicate chiseled_bookshelf::is_chiseled_bookshelf,
        affect_neighbors_after_removal: chiseled_bookshelf::affect_neighbors_after_removal(level, pos, state),
    },
    DecoratedPot {
        matches: predicate decorated_pot::is_decorated_pot,
        affect_neighbors_after_removal: decorated_pot::affect_neighbors_after_removal(level, pos, state),
        update_shape: decorated_pot::update_shape(level, state, pos),
    },
    Shelf {
        matches: predicate shelf::is_shelf,
        neighbor_changed: shelf::neighbor_changed(level, pos, state),
        on_place: shelf::on_place(level, pos, state, old_state),
        affect_neighbors_after_removal: shelf::affect_neighbors_after_removal(level, pos, state),
        update_shape: waterlogged::update_shape(level, state, pos),
    },
    Bell {
        matches: predicate bell::is_bell,
        neighbor_changed: bell::neighbor_changed(level, pos, state),
        update_shape: bell::update_shape(level, state, pos, direction, neighbour_state),
        can_survive: bell::can_survive(state, pos, tags, at),
        can_survive_live: bell::can_survive_live(level, state, pos),
    },
    Skull {
        matches: predicate skull::is_skull,
        neighbor_changed: skull::neighbor_changed(level, pos, state),
        set_placed_by: skull::set_placed_by(level, pos, state, placed_by),
    },
    Sign {
        matches: predicate sign::is_sign,
        update_shape: sign::update_shape(level, state, pos, direction),
        can_survive: sign::can_survive(state, pos, at),
        can_survive_live: sign::can_survive_live(level, state, pos),
    },
    WallHangingSign {
        matches: predicate sign::is_wall_hanging_sign,
        update_shape: sign::wall_hanging_update_shape(level, state, pos),
    },
    Banner {
        matches: predicate banner::is_banner,
        update_shape: banner::update_shape(level, state, pos, direction),
        can_survive: banner::can_survive(state, pos, at),
    },
    Torch {
        matches: predicate torch::is_torch,
        update_shape: torch::update_shape(level, state, pos, direction),
        can_survive: torch::can_survive(pos, at),
        can_survive_live: torch::can_survive_live(level, pos),
    },
    WallTorch {
        matches: predicate torch::is_wall_torch,
        update_shape: torch::wall_update_shape(level, state, pos, direction),
        can_survive: torch::wall_can_survive(state, pos, at),
        can_survive_live: torch::wall_can_survive_live(level, state, pos),
    },
    WeatheringCopperLantern {
        matches: resolved_by weathering::weathering_owner,
        random_tick: weathering::random_tick(level, pos, state),
        update_shape: lantern::update_shape(level, state, pos, direction),
        can_survive: lantern::can_survive(state, pos, tags, at),
        can_survive_live: lantern::can_survive_live(level, state, pos),
    },
    Lantern {
        matches: predicate lantern::is_lantern,
        update_shape: lantern::update_shape(level, state, pos, direction),
        can_survive: lantern::can_survive(state, pos, tags, at),
        can_survive_live: lantern::can_survive_live(level, state, pos),
    },
    Ladder {
        matches: predicate ladder::is_ladder,
        update_shape: ladder::update_shape(level, state, pos, direction),
        can_survive: ladder::can_survive(state, pos, at),
        can_survive_live: ladder::can_survive_live(level, state, pos),
    },
    DeadCoralPlantType {
        matches: predicate coral::is_dead_plant_type,
        update_shape: coral::plant_type_update_shape(level, state, pos, direction),
        can_survive: coral::plant_type_can_survive(pos, at),
        can_survive_live: coral::plant_type_can_survive_live(level, pos),
    },
    CoralPlantType {
        matches: predicate coral::is_living_plant_type,
        tick: coral::tick(level, pos, state),
        on_place: coral::on_place(level, pos, state),
        update_shape: coral::living_plant_type_update_shape(level, state, pos, direction),
        can_survive: coral::plant_type_can_survive(pos, at),
        can_survive_live: coral::plant_type_can_survive_live(level, pos),
    },
    DeadCoralWallFan {
        matches: predicate coral::is_dead_wall_fan,
        update_shape: coral::wall_fan_update_shape(level, state, pos, direction),
        can_survive: coral::wall_fan_can_survive(state, pos, at),
        can_survive_live: coral::wall_fan_can_survive_live(level, state, pos),
    },
    CoralWallFan {
        matches: predicate coral::is_living_wall_fan,
        tick: coral::tick(level, pos, state),
        on_place: coral::on_place(level, pos, state),
        update_shape: coral::living_wall_fan_update_shape(level, state, pos, direction),
        can_survive: coral::wall_fan_can_survive(state, pos, at),
        can_survive_live: coral::wall_fan_can_survive_live(level, state, pos),
    },
    Coral {
        matches: predicate coral::is_coral_block,
        tick: coral::tick(level, pos, state),
        update_shape: coral::coral_block_update_shape(level, state, pos),
    },
    Sponge {
        matches: predicate sponge::is_sponge,
        neighbor_changed: sponge::neighbor_changed(level, pos, state),
        on_place: sponge::on_place(level, pos, state, old_state),
    },
    WetSponge {
        matches: predicate sponge::is_wet_sponge,
        on_place: sponge::wet_sponge_on_place(level, pos, state),
    },
    BubbleColumn {
        matches: block VANILLA_BLOCK_BUBBLE_COLUMN_ID,
        tick: bubble_column::tick(level, pos, state),
        update_shape: bubble_column::update_shape(level, state, pos, direction, neighbour_state),
        can_survive: bubble_column::can_survive(pos, tags, at),
    },
    Cauldron {
        matches: tick_override AbstractCauldronBlock,
        tick: cauldron::tick(level, pos, state),
    },
    CreakingHeart {
        matches: block VANILLA_BLOCK_CREAKING_HEART_ID,
        tick: creaking_heart::tick(level, pos, state),
        affect_neighbors_after_removal: containers::update_neighbours_after_destroy(level, pos, state),
        update_shape: creaking_heart::update_shape(level, state, pos),
    },
    Barrel {
        matches: predicate barrel::is_barrel,
        tick: barrel::tick(level, pos, state),
        affect_neighbors_after_removal: containers::update_neighbours_after_destroy(level, pos, state),
    },
    EnderChest {
        matches: predicate ender_chest::is_ender_chest,
        tick: ender_chest::tick(level, pos, state),
        update_shape: waterlogged::update_shape(level, state, pos),
    },
    /// The ten container classes whose whole `affectNeighborsAfterRemoval` is
    /// `Containers.updateNeighboursAfterDestroy`. One body, because vanilla
    /// gives them one; the six rows above that win some of these blocks for
    /// other reasons carry the same call. See [`containers`].
    Container {
        matches: predicate containers::is_container,
        affect_neighbors_after_removal: containers::update_neighbours_after_destroy(level, pos, state),
        // Only `JukeboxBlock` overrides it here; `CrafterBlock`'s belongs to its ticker.
        set_placed_by: jukebox::set_placed_by(level, pos, state, placed_by),
    },
    RedStoneWire {
        matches: predicate redstone_wire::is_redstone_wire,
        neighbor_changed: redstone_wire::neighbor_changed(level, pos, state, changed_block, orientation),
        on_place: redstone_wire::on_place(level, pos, state, old_state),
        affect_neighbors_after_removal: redstone_wire::affect_neighbors_after_removal(level, pos, state, moved_by_piston),
        update_shape: redstone_wire::update_shape(level, state, pos, direction, neighbour_state),
        can_survive: redstone_wire::can_survive(pos, at),
        can_survive_live: redstone_wire::can_survive_live(level, pos),
        use_without_item: redstone_wire::use_without_item(level, pos, state, may_build),
    },

    CandleCake {
        matches: predicate footing::is_candle_cake,
        update_shape: footing::candle_cake_update_shape(level, state, pos, direction),
        can_survive: footing::candle_cake_can_survive(pos, at),
    },
    SeaPickle {
        matches: predicate footing::is_sea_pickle,
        update_shape: footing::sea_pickle_update_shape(level, state, pos),
        can_survive: footing::sea_pickle_can_survive(pos, at),
        can_survive_live: footing::sea_pickle_can_survive_live(level, pos),
    },
    SporeBlossom {
        matches: predicate footing::is_spore_blossom,
        update_shape: footing::spore_blossom_update_shape(level, state, pos, direction),
        can_survive: footing::spore_blossom_can_survive(pos, tags, at),
        can_survive_live: footing::spore_blossom_can_survive_live(level, pos),
    },
    HangingRoots {
        matches: predicate footing::is_hanging_roots,
        update_shape: footing::hanging_roots_update_shape(level, state, pos, direction),
        can_survive: footing::hanging_roots_can_survive(pos, at),
        can_survive_live: footing::hanging_roots_can_survive_live(level, pos),
    },
    MangrovePropagule {
        matches: predicate footing::is_mangrove_propagule,
        random_tick: sapling::mangrove_random_tick(level, pos, state),
        update_shape: footing::mangrove_propagule_update_shape(level, state, pos, direction),
        can_survive: footing::mangrove_propagule_can_survive(state, pos, tags, at),
    },
    LeafLitter {
        matches: predicate footing::is_leaf_litter,
        update_shape: footing::leaf_litter_update_shape(level, state, pos),
        can_survive: footing::leaf_litter_can_survive(pos, at),
        can_survive_live: footing::leaf_litter_can_survive_live(level, pos),
    },
    DirtPath {
        matches: predicate footing::is_dirt_path,
        tick: footing::dirt_path_tick(level, pos),
        update_shape: footing::dirt_path_update_shape(level, state, pos, direction),
        can_survive: footing::dirt_path_can_survive(pos, at),
    },
    Grindstone {
        matches: predicate footing::is_grindstone,
        can_survive: footing::grindstone_can_survive(pos, at),
    },
    BigDripleaf {
        matches: predicate big_dripleaf::is_leaf,
        tick: big_dripleaf::tick(level, pos, state),
        neighbor_changed: big_dripleaf::neighbor_changed(level, pos, state),
        update_shape: big_dripleaf::update_shape(level, state, pos, direction, neighbour_state),
        can_survive: big_dripleaf::can_survive(pos, tags, at),
    },
    BigDripleafStem {
        matches: predicate big_dripleaf::is_stem,
        tick: big_dripleaf::stem_tick(level, pos),
        update_shape: big_dripleaf::stem_update_shape(level, state, pos, direction),
        can_survive: big_dripleaf::stem_can_survive(pos, tags, at),
    },
    Frogspawn {
        matches: predicate footing::is_frogspawn,
        tick: footing::frogspawn_tick(level, pos, state),
        on_place: footing::frogspawn_on_place(level, pos, state),
        update_shape: footing::frogspawn_update_shape(level, state, pos),
        can_survive: footing::frogspawn_can_survive(pos, tags, at),
    },
    TurtleEgg {
        matches: random_tick_override TurtleEggBlock,
        random_tick: turtle_egg::random_tick(level, pos, state),
        on_place: turtle_egg::on_place(level, pos, state),
    },
    SnifferEgg {
        matches: predicate sniffer_egg::is_sniffer_egg,
        tick: sniffer_egg::tick(level, pos, state),
        on_place: sniffer_egg::on_place(level, pos, state),
    },
    SoulFire {
        matches: predicate footing::is_soul_fire,
        on_place: fire::soul_fire_on_place(level, pos, state, old_state),
        update_shape: footing::soul_fire_update_shape(level, state, pos),
        can_survive: footing::soul_fire_can_survive(pos, tags, at),
    },
    Multiface {
        matches: predicate multiface::is_multiface,
        update_shape: multiface::update_shape(level, state, pos, direction, neighbour_state),
        can_survive: multiface::can_survive(state, pos, at),
        can_survive_live: multiface::can_survive_live(level, state, pos),
    },
    MossyCarpet {
        matches: predicate multiface::is_mossy_carpet,
        update_shape: multiface::mossy_carpet_update_shape(level, state, pos),
        can_survive: multiface::mossy_carpet_can_survive(state, pos, at),
        set_placed_by: multiface::mossy_carpet_set_placed_by(level, pos),
    },
    HangingMoss {
        matches: predicate multiface::is_hanging_moss,
        tick: multiface::hanging_moss_tick(level, pos),
        update_shape: multiface::hanging_moss_update_shape(level, state, pos),
        can_survive: multiface::hanging_moss_can_survive(pos, at),
        can_survive_live: multiface::hanging_moss_can_survive_live(level, pos),
    },
    FrostedIce {
        matches: predicate melting::is_frosted_ice,
        tick: melting::frosted_ice_tick(level, pos, state),
        neighbor_changed: melting::frosted_ice_neighbor_changed(level, pos, state, changed_block),
        on_place: melting::frosted_ice_on_place(level, pos, state),
    },
    Scaffolding {
        matches: predicate scaffolding::is_scaffolding,
        tick: scaffolding::tick(level, pos, state),
        on_place: scaffolding::on_place(level, pos, state),
        update_shape: scaffolding::update_shape(level, state, pos),
        can_survive: scaffolding::can_survive(pos, at),
        can_survive_live: scaffolding::can_survive_live(level, pos),
    },
    SculkCatalyst {
        matches: tick_override SculkCatalystBlock,
        tick: sculk_catalyst::tick(level, pos, state),
    },
    Light {
        matches: block VANILLA_BLOCK_LIGHT_ID,
        update_shape: waterlogged::update_shape(level, state, pos),
    },
    SculkShrieker {
        matches: block VANILLA_BLOCK_SCULK_SHRIEKER_ID,
        update_shape: waterlogged::update_shape(level, state, pos),
    },
    SculkSensor {
        matches: tick_override SculkSensorBlock,
        tick: sculk_sensor::tick(level, pos, state),
        on_place: sculk_sensor::on_place(level, pos, state, old_state),
        affect_neighbors_after_removal: sculk_sensor::affect_neighbors_after_removal(level, pos, state),
        update_shape: sculk_sensor::update_shape(level, state, pos),
    },
    PressurePlate {
        matches: predicate footing::is_pressure_plate,
        tick: pressure_plate::tick(level, pos, state),
        affect_neighbors_after_removal: pressure_plate::affect_neighbors_after_removal(level, pos, state, moved_by_piston),
        update_shape: footing::pressure_plate_update_shape(level, state, pos, direction),
        can_survive: footing::pressure_plate_can_survive(pos, at),
        can_survive_live: footing::pressure_plate_can_survive_live(level, pos),
        entity_inside: pressure_plate::entity_inside(level, pos, state, inside),
    },
    Piston {
        matches: predicate piston::is_piston_base,
        neighbor_changed: piston::neighbor_changed(level, pos, state),
        on_place: piston::on_place(level, pos, state, old_state),
        trigger_event: piston::trigger_event(level, pos, state, event, data),
        set_placed_by: piston::set_placed_by(level, pos, state),
    },
    PistonHead {
        matches: predicate piston_head::is_piston_head,
        neighbor_changed: piston_head::neighbor_changed(level, pos, state, changed_block),
        affect_neighbors_after_removal: piston_head::affect_neighbors_after_removal(level, pos, state),
        update_shape: piston_head::update_shape(level, state, pos, direction),
        can_survive: piston_head::can_survive(state, pos, at),
    },
    Button {
        matches: predicate button::is_button,
        tick: button::tick(level, pos, state),
        affect_neighbors_after_removal: footing::face_attached_affect_neighbors_after_removal(level, pos, state, moved_by_piston),
        update_shape: footing::face_attached_update_shape(level, state, pos, direction),
        can_survive: footing::face_attached_can_survive(state, pos, at),
        can_survive_live: footing::face_attached_can_survive_live(level, state, pos),
        use_without_item: button::use_without_item(level, pos, state, player),
        entity_inside: button::entity_inside(level, pos, state, inside),
    },
    Lever {
        matches: predicate lever::is_lever,
        affect_neighbors_after_removal: footing::face_attached_affect_neighbors_after_removal(level, pos, state, moved_by_piston),
        update_shape: footing::face_attached_update_shape(level, state, pos, direction),
        can_survive: footing::face_attached_can_survive(state, pos, at),
        can_survive_live: footing::face_attached_can_survive_live(level, state, pos),
        use_without_item: lever::use_without_item(level, pos, state),
    },
    TripWireHook {
        matches: predicate footing::is_tripwire_hook,
        tick: tripwire_hook::tick(level, pos, state),
        affect_neighbors_after_removal: tripwire_hook::affect_neighbors_after_removal(level, pos, state, moved_by_piston),
        update_shape: footing::tripwire_hook_update_shape(level, state, pos, direction),
        can_survive: footing::tripwire_hook_can_survive(state, pos, at),
        can_survive_live: footing::tripwire_hook_can_survive_live(level, state, pos),
        set_placed_by: tripwire_hook::set_placed_by(level, pos, state),
    },
    TripWire {
        matches: block VANILLA_BLOCK_TRIPWIRE_ID,
        tick: tripwire::tick(level, pos, state),
        on_place: tripwire::on_place(level, pos, state, old_state),
        affect_neighbors_after_removal: tripwire::affect_neighbors_after_removal(level, pos, state, moved_by_piston),
        update_shape: tripwire::update_shape(state, direction, neighbour_state),
        entity_inside: tripwire::entity_inside(level, pos, state, inside),
    },

    Farmland {
        matches: block VANILLA_BLOCK_FARMLAND_ID,
        random_tick: farmland::random_tick(level, pos, state),
        tick: farmland::tick(level, pos, state),
        update_shape: farmland::update_shape(level, state, pos, direction),
        can_survive: farmland::can_survive_at_state(pos, tags, at),
    },
    SnowLayer {
        matches: block VANILLA_BLOCK_SNOW_ID,
        random_tick: melting::snow_layer_random_tick(level, pos, state),
        update_shape: footing::snow_layer_update_shape(level, state, pos),
        can_survive: footing::snow_layer_can_survive(pos, tags, at),
        can_survive_live: footing::snow_layer_can_survive_live(level, pos),
    },
    Ice {
        matches: block VANILLA_BLOCK_ICE_ID,
        random_tick: melting::ice_random_tick(level, pos, state),
    },
    NoteBlock {
        matches: block VANILLA_BLOCK_NOTE_BLOCK_ID,
        neighbor_changed: note_block::neighbor_changed(level, pos, state),
        update_shape: note_block::update_shape(level, state, pos, direction),
        use_item_on: note_block::use_item_on(held, face),
        trigger_event: note_block::trigger_event(pos, state, random, skull_sound),
    },
}

#[must_use]
pub fn can_survive_at<L: BlockGetter + ?Sized>(
    block_state_id: i32,
    pos: BlockPos,
    level: &L,
) -> bool {
    can_survive_live(block_state_id, pos, level) != Some(false)
}

#[must_use]
pub fn is_base_rail_block(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.affect_neighbors_after_removal_override)
        == Some(VanillaAffectNeighborsAfterRemovalOverride::BaseRailBlock)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndirectNeighbourShapeOwner {
    RedStoneWire,
}

/// [`IndirectNeighbourShapeOwner`] for a stored state.
///
/// There is no generated class tag for this member — the catalog carries the
/// four block-update columns and not this one — so the join is the block id of
/// the single owner, and the test above is what keeps "single" true.
#[must_use]
pub fn indirect_neighbour_shape_owner(block_state_id: i32) -> Option<IndirectNeighbourShapeOwner> {
    (block_type_of(block_state_id) == Some(VANILLA_BLOCK_REDSTONE_WIRE_ID))
        .then_some(IndirectNeighbourShapeOwner::RedStoneWire)
}

pub(crate) fn update_indirect_neighbour_shapes(
    level: &mut LevelCtx<'_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    update_flags: i32,
    update_limit: i32,
) {
    match indirect_neighbour_shape_owner(block_state_id) {
        None => {}
        Some(IndirectNeighbourShapeOwner::RedStoneWire) => {
            redstone_wire::update_indirect_neighbour_shapes(
                level,
                block_state_id,
                pos,
                update_flags,
                update_limit,
            );
        }
    }
}

pub type BlockStateAt<'a> = &'a mut dyn FnMut(BlockPos) -> i32;

#[derive(Debug, Clone, PartialEq)]
pub struct BlockTriggerOutcome {
    pub triggered: bool,
    /// The `Level.playSeededSound` the body performed on its way to that
    /// answer, if any.
    pub sound: Option<SoundEvent>,
}

impl BlockTriggerOutcome {
    /// `BlockBehaviour.triggerEvent`'s own default
    /// (`BlockBehaviour.java:221-223`): no packet, no sound.
    #[must_use]
    pub const fn not_triggered() -> Self {
        Self {
            triggered: false,
            sound: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockUseResult {
    /// `InteractionResult.PASS` or `FAIL`: not consumed, and not a request for
    /// the empty-hand leg.
    Pass,
    TryWithEmptyHand,
    /// Any `InteractionResult.Success`, i.e. `consumesAction()` is true.
    Consumed,
}

pub mod anvil;
