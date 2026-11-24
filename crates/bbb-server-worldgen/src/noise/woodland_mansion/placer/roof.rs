//! `MansionPiecePlacer.createRoof` (`WoodlandMansionPieces.java:660-871`).

use bbb_protocol::packets::BlockPos;

use super::super::direction::{above, step_relative, MansionDirection};
use super::super::grid::{MansionGrid, SimpleGrid};
use super::super::{qualify_template, WoodlandMansionPiece};
use crate::noise::ruined_portal::RuinedPortalMirror;
use crate::noise::structure_pieces::JigsawRotation;

fn is_house(grid: &SimpleGrid, x: i32, y: i32) -> bool {
    MansionGrid::is_house(grid, x, y)
}

fn push(
    pieces: &mut Vec<WoodlandMansionPiece>,
    name: &str,
    pos: BlockPos,
    rotation: JigsawRotation,
) {
    pieces.push(WoodlandMansionPiece::new(
        qualify_template(name),
        pos,
        rotation,
        RuinedPortalMirror::None,
    ));
}

fn rotated(rotation: JigsawRotation, applied: JigsawRotation) -> JigsawRotation {
    super::super::direction::compose_rotation(rotation, applied)
}

fn base_position(
    roof_origin: BlockPos,
    rotation: JigsawRotation,
    start_x: i32,
    start_y: i32,
    x: i32,
    y: i32,
) -> BlockPos {
    let position = step_relative(
        roof_origin,
        rotation,
        MansionDirection::South,
        8 + (y - start_y) * 8,
    );
    step_relative(
        position,
        rotation,
        MansionDirection::East,
        (x - start_x) * 8,
    )
}

pub fn create_roof(
    pieces: &mut Vec<WoodlandMansionPiece>,
    roof_origin: BlockPos,
    rotation: JigsawRotation,
    grid: &SimpleGrid,
    above_grid: Option<&SimpleGrid>,
    start_x: i32,
    start_y: i32,
) {
    use MansionDirection::{East, North, South, West};

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let position = base_position(roof_origin, rotation, start_x, start_y, x, y);
            let is_above = above_grid.is_some_and(|above_grid| is_house(above_grid, x, y));
            if is_house(grid, x, y) && !is_above {
                push(pieces, "roof", above(position, 3), rotation);
                if !is_house(grid, x + 1, y) {
                    let p2 = step_relative(position, rotation, East, 6);
                    push(pieces, "roof_front", p2, rotation);
                }
                if !is_house(grid, x - 1, y) {
                    let p2 = step_relative(
                        step_relative(position, rotation, East, 0),
                        rotation,
                        South,
                        7,
                    );
                    push(
                        pieces,
                        "roof_front",
                        p2,
                        rotated(rotation, JigsawRotation::Clockwise180),
                    );
                }
                if !is_house(grid, x, y - 1) {
                    let p2 = step_relative(position, rotation, West, 1);
                    push(
                        pieces,
                        "roof_front",
                        p2,
                        rotated(rotation, JigsawRotation::CounterClockwise90),
                    );
                }
                if !is_house(grid, x, y + 1) {
                    let p2 = step_relative(
                        step_relative(position, rotation, East, 6),
                        rotation,
                        South,
                        6,
                    );
                    push(
                        pieces,
                        "roof_front",
                        p2,
                        rotated(rotation, JigsawRotation::Clockwise90),
                    );
                }
            }
        }
    }

    if let Some(above_grid) = above_grid {
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                let position = base_position(roof_origin, rotation, start_x, start_y, x, y);
                let is_above = is_house(above_grid, x, y);
                if is_house(grid, x, y) && is_above {
                    if !is_house(grid, x + 1, y) {
                        let p2 = step_relative(position, rotation, East, 7);
                        push(pieces, "small_wall", p2, rotation);
                    }
                    if !is_house(grid, x - 1, y) {
                        let p2 = step_relative(
                            step_relative(position, rotation, West, 1),
                            rotation,
                            South,
                            6,
                        );
                        push(
                            pieces,
                            "small_wall",
                            p2,
                            rotated(rotation, JigsawRotation::Clockwise180),
                        );
                    }
                    if !is_house(grid, x, y - 1) {
                        let p2 = step_relative(
                            step_relative(position, rotation, West, 0),
                            rotation,
                            North,
                            1,
                        );
                        push(
                            pieces,
                            "small_wall",
                            p2,
                            rotated(rotation, JigsawRotation::CounterClockwise90),
                        );
                    }
                    if !is_house(grid, x, y + 1) {
                        let p2 = step_relative(
                            step_relative(position, rotation, East, 6),
                            rotation,
                            South,
                            7,
                        );
                        push(
                            pieces,
                            "small_wall",
                            p2,
                            rotated(rotation, JigsawRotation::Clockwise90),
                        );
                    }
                    if !is_house(grid, x + 1, y) {
                        if !is_house(grid, x, y - 1) {
                            let p2 = step_relative(
                                step_relative(position, rotation, East, 7),
                                rotation,
                                North,
                                2,
                            );
                            push(pieces, "small_wall_corner", p2, rotation);
                        }
                        if !is_house(grid, x, y + 1) {
                            let p2 = step_relative(
                                step_relative(position, rotation, East, 8),
                                rotation,
                                South,
                                7,
                            );
                            push(
                                pieces,
                                "small_wall_corner",
                                p2,
                                rotated(rotation, JigsawRotation::Clockwise90),
                            );
                        }
                    }
                    if !is_house(grid, x - 1, y) {
                        if !is_house(grid, x, y - 1) {
                            let p2 = step_relative(
                                step_relative(position, rotation, West, 2),
                                rotation,
                                North,
                                1,
                            );
                            push(
                                pieces,
                                "small_wall_corner",
                                p2,
                                rotated(rotation, JigsawRotation::CounterClockwise90),
                            );
                        }
                        if !is_house(grid, x, y + 1) {
                            let p2 = step_relative(
                                step_relative(position, rotation, West, 1),
                                rotation,
                                South,
                                8,
                            );
                            push(
                                pieces,
                                "small_wall_corner",
                                p2,
                                rotated(rotation, JigsawRotation::Clockwise180),
                            );
                        }
                    }
                }
            }
        }
    }

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let position = base_position(roof_origin, rotation, start_x, start_y, x, y);
            let is_above = above_grid.is_some_and(|above_grid| is_house(above_grid, x, y));
            if is_house(grid, x, y) && !is_above {
                if !is_house(grid, x + 1, y) {
                    let p2 = step_relative(position, rotation, East, 6);
                    if !is_house(grid, x, y + 1) {
                        let p3 = step_relative(p2, rotation, South, 6);
                        push(pieces, "roof_corner", p3, rotation);
                    } else if is_house(grid, x + 1, y + 1) {
                        let p3 = step_relative(p2, rotation, South, 5);
                        push(pieces, "roof_inner_corner", p3, rotation);
                    }
                    if !is_house(grid, x, y - 1) {
                        push(
                            pieces,
                            "roof_corner",
                            p2,
                            rotated(rotation, JigsawRotation::CounterClockwise90),
                        );
                    } else if is_house(grid, x + 1, y - 1) {
                        let p3 = step_relative(
                            step_relative(position, rotation, East, 9),
                            rotation,
                            North,
                            2,
                        );
                        push(
                            pieces,
                            "roof_inner_corner",
                            p3,
                            rotated(rotation, JigsawRotation::Clockwise90),
                        );
                    }
                }
                if !is_house(grid, x - 1, y) {
                    let p2 = step_relative(
                        step_relative(position, rotation, East, 0),
                        rotation,
                        South,
                        0,
                    );
                    if !is_house(grid, x, y + 1) {
                        let p3 = step_relative(p2, rotation, South, 6);
                        push(
                            pieces,
                            "roof_corner",
                            p3,
                            rotated(rotation, JigsawRotation::Clockwise90),
                        );
                    } else if is_house(grid, x - 1, y + 1) {
                        let p3 =
                            step_relative(step_relative(p2, rotation, South, 8), rotation, West, 3);
                        push(
                            pieces,
                            "roof_inner_corner",
                            p3,
                            rotated(rotation, JigsawRotation::CounterClockwise90),
                        );
                    }
                    if !is_house(grid, x, y - 1) {
                        push(
                            pieces,
                            "roof_corner",
                            p2,
                            rotated(rotation, JigsawRotation::Clockwise180),
                        );
                    } else if is_house(grid, x - 1, y - 1) {
                        let p3 = step_relative(p2, rotation, South, 1);
                        push(
                            pieces,
                            "roof_inner_corner",
                            p3,
                            rotated(rotation, JigsawRotation::Clockwise180),
                        );
                    }
                }
            }
        }
    }
}
