use super::*;

use bbb_protocol::item_types::vanilla_item_type_for_resource_id;

use crate::block_behavior::scaffolding::STABILITY_MAX_DISTANCE;

const TOWER: BlockPos = at(2, 70, 2);

fn scaffold(distance: i32, bottom: bool, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:scaffolding",
        &[
            ("bottom", if bottom { "true" } else { "false" }),
            ("distance", &distance.to_string()),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn tick(world: &mut NativeWorld, pos: BlockPos) {
    let state = world.state(pos);
    world.with_level(|level| tick_block(level, pos, state));
}

#[test]
fn a_sturdy_floor_is_zero_and_a_tower_carries_its_own_count_up() {
    let mut world = NativeWorld::new();
    world.set(below(TOWER), stone());
    world.set(TOWER, scaffold(7, false, false));
    tick(&mut world, TOWER);
    assert_eq!(world.state(TOWER), scaffold(0, false, false));

    let mut world = NativeWorld::new();
    world.set(below(TOWER), scaffold(3, false, false));
    world.set(TOWER, scaffold(7, false, false));
    tick(&mut world, TOWER);
    assert_eq!(world.state(TOWER), scaffold(3, false, false));
}

#[test]
fn a_sideways_neighbour_costs_one_and_the_cheapest_wins() {
    let mut world = NativeWorld::new();
    world.set(relative(TOWER, Direction::North), scaffold(4, false, false));
    world.set(relative(TOWER, Direction::East), scaffold(1, false, false));
    world.set(TOWER, scaffold(7, false, false));

    tick(&mut world, TOWER);

    assert_eq!(world.state(TOWER), scaffold(2, true, false));
}

#[test]
fn a_tower_seven_steps_out_cannot_survive() {
    let survives = |neighbour: i32, cell: BlockPos| {
        can_survive(
            scaffold(0, true, false),
            TOWER,
            vanilla_tags(),
            &mut |pos| {
                if pos == cell {
                    neighbour
                } else {
                    air()
                }
            },
        )
    };
    assert_eq!(survives(stone(), below(TOWER)), Some(true));
    assert_eq!(
        survives(scaffold(5, false, false), relative(TOWER, Direction::West)),
        Some(true)
    );
    assert_eq!(
        survives(scaffold(6, false, false), relative(TOWER, Direction::West)),
        Some(false)
    );
    assert_eq!(survives(air(), below(TOWER)), Some(false));
}

#[test]
fn a_waterlogged_tower_remeasures_after_a_shape_update() {
    let mut world = NativeWorld::ticking(TOWER);
    world.set(below(TOWER), stone());
    world.set(TOWER, scaffold(0, true, true));
    world.shape_update(TOWER, Direction::North);
    for _ in 0..2 {
        world.run_tick();
    }
    assert_eq!(world.state(TOWER), scaffold(0, false, true));
    assert!(world
        .saved_fluid_ticks()
        .iter()
        .any(|tick| tick.x == TOWER.x
            && tick.y == TOWER.y
            && tick.z == TOWER.z
            && tick.id == "minecraft:water"));
}

#[test]
fn a_tower_cut_at_the_foot_breaks_and_one_already_adrift_falls() {
    let mut world = NativeWorld::new();
    world.set(TOWER, scaffold(0, true, false));
    tick(&mut world, TOWER);
    assert_eq!(world.state(TOWER), air());
    assert!(world
        .items()
        .iter()
        .any(|(name, count)| name == "minecraft:scaffolding" && *count == 1));
    assert!(world.falling_blocks().is_empty());

    let adrift = scaffold(STABILITY_MAX_DISTANCE, true, false);
    let mut world = NativeWorld::new();
    world.set(TOWER, adrift);
    tick(&mut world, TOWER);
    assert!(world
        .falling_blocks()
        .iter()
        .any(|body| body.block_state_id() == adrift));
    assert!(world.items().is_empty());
}

fn scaffolding_item() -> i32 {
    vanilla_item_type_for_resource_id("minecraft:scaffolding")
        .expect("scaffolding item")
        .id
}

fn dirt_item() -> i32 {
    vanilla_item_type_for_resource_id("minecraft:dirt")
        .expect("dirt item")
        .id
}

fn place_context(pos: BlockPos, face: Direction) -> BlockPlacementContext {
    BlockPlacementContext {
        pos,
        clicked_face: face,
        click_x: 0.5,
        click_y: 0.5,
        click_z: 0.5,
        replacing_clicked: true,
        horizontal_direction: Direction::South,
        secondary_use_active: false,
        inside: false,
    }
}

fn update_in(
    context: BlockPlacementContext,
    cells: &[(BlockPos, i32)],
) -> Option<BlockPlacementContext> {
    let mut cell_at = |pos| {
        (
            true,
            cells
                .iter()
                .find(|(at, _)| *at == pos)
                .map(|(_, state)| *state)
                .unwrap_or_else(air),
        )
    };
    update_placement_context(scaffolding_item(), context, &mut cell_at)
}

fn placed_state(pos: BlockPos, cells: &[(BlockPos, i32)]) -> Option<i32> {
    let mut block_state_at = |at| {
        cells
            .iter()
            .find(|(cell, _)| *cell == at)
            .map(|(_, state)| *state)
            .unwrap_or_else(air)
    };
    state_for_placement_with_view(
        scaffolding_item(),
        place_context(pos, Direction::Up),
        BlockPlacementView {
            x_rot: 0.0,
            y_rot: 0.0,
        },
        vanilla_tags(),
        &mut block_state_at,
        &mut |_, _| Some(true),
    )
}

#[test]
fn scaffolding_yields_only_to_another_scaffolding() {
    let occupied = scaffold(0, false, false);
    let context = place_context(TOWER, Direction::Up);
    assert_eq!(
        can_replace_for_placement(scaffolding_item(), occupied, context),
        Some(true)
    );
    assert_eq!(
        can_replace_for_placement(dirt_item(), occupied, context),
        Some(false)
    );
}

#[test]
fn a_sturdy_floor_places_distance_zero_without_a_bottom() {
    assert_eq!(
        placed_state(TOWER, &[(below(TOWER), stone())]),
        Some(scaffold(0, false, false))
    );
}

#[test]
fn a_cell_seven_steps_from_support_is_refused_unless_it_is_already_scaffolding() {
    let air_cell = place_context(TOWER, Direction::Up);
    assert_eq!(update_in(air_cell, &[]), None);
    let on_floor = update_in(air_cell, &[(below(TOWER), stone())]).expect("supported air stays");
    assert_eq!(on_floor.pos, TOWER);
}

#[test]
fn clicking_a_scaffold_side_walks_the_column_up() {
    let context = place_context(TOWER, Direction::South);
    let updated = update_in(
        context,
        &[(below(TOWER), stone()), (TOWER, scaffold(0, false, false))],
    )
    .expect("the cell above is empty");
    assert_eq!(updated.pos, above(TOWER));
    assert_eq!(updated.clicked_face, Direction::Up);
    assert!(!updated.inside);
    assert!(updated.replacing_clicked);
}

#[test]
fn clicking_a_scaffold_top_walks_the_way_the_player_faces() {
    let context = place_context(TOWER, Direction::Up);
    let updated = update_in(
        context,
        &[(below(TOWER), stone()), (TOWER, scaffold(0, false, false))],
    )
    .expect("the south cell is empty");
    assert_eq!(updated.pos, relative(TOWER, Direction::South));
    assert_eq!(updated.clicked_face, Direction::South);
}

#[test]
fn secondary_use_inside_a_scaffold_walks_the_opposite_face() {
    let mut context = place_context(TOWER, Direction::South);
    context.secondary_use_active = true;
    context.inside = true;
    let updated = update_in(context, &[(TOWER, scaffold(0, false, false))])
        .expect("the opposite cell is empty");
    assert_eq!(updated.pos, relative(TOWER, Direction::North));
    assert_eq!(updated.clicked_face, Direction::North);
    assert!(!updated.inside);
}

#[test]
fn secondary_use_without_inside_walks_the_clicked_face() {
    let mut context = place_context(TOWER, Direction::South);
    context.secondary_use_active = true;
    let updated = update_in(context, &[(TOWER, scaffold(0, false, false))])
        .expect("the clicked-face cell is empty");
    assert_eq!(updated.pos, relative(TOWER, Direction::South));
    assert_eq!(updated.clicked_face, Direction::South);
}

#[test]
fn a_walk_through_scaffolding_stops_at_the_first_replaceable_cell() {
    let mid = above(TOWER);
    let top = above(mid);
    let updated = update_in(
        place_context(TOWER, Direction::North),
        &[
            (TOWER, scaffold(0, false, false)),
            (mid, scaffold(0, false, false)),
        ],
    )
    .expect("the column ends in air");
    assert_eq!(updated.pos, top);
}

#[test]
fn a_walk_refuses_when_the_next_cell_cannot_be_replaced() {
    assert_eq!(
        update_in(
            place_context(TOWER, Direction::West),
            &[(TOWER, scaffold(0, false, false)), (above(TOWER), stone()),],
        ),
        None
    );
}

#[test]
fn a_walk_breaks_when_the_next_cell_leaves_the_world() {
    let mut cell_at = |pos| {
        if pos == TOWER {
            (true, scaffold(0, false, false))
        } else {
            (pos != above(TOWER), air())
        }
    };
    assert_eq!(
        update_placement_context(
            scaffolding_item(),
            place_context(TOWER, Direction::East),
            &mut cell_at,
        ),
        None
    );
}
