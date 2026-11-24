use crate::random::WorldgenRandom;
use bbb_protocol::packets::BlockPos;

use super::{qualify_template, EndCityPiece, END_CITY_TEMPLATES};
use crate::noise::structure_pieces::JigsawRotation;
use crate::noise::woodland_mansion::direction::{compose_rotation, rotate_block_pos};

const TOWER_BRIDGES: [(JigsawRotation, BlockPos); 4] = [
    (JigsawRotation::None, at(1, -1, 0)),
    (JigsawRotation::Clockwise90, at(6, -1, 1)),
    (JigsawRotation::CounterClockwise90, at(0, -1, 5)),
    (JigsawRotation::Clockwise180, at(5, -1, 6)),
];
const FAT_TOWER_BRIDGES: [(JigsawRotation, BlockPos); 4] = [
    (JigsawRotation::None, at(4, -1, 0)),
    (JigsawRotation::Clockwise90, at(12, -1, 4)),
    (JigsawRotation::CounterClockwise90, at(0, -1, 8)),
    (JigsawRotation::Clockwise180, at(8, -1, 12)),
];

#[derive(Clone, Copy)]
enum Section {
    HouseTower,
    Tower,
    TowerBridge,
    FatTower,
}

pub fn start_house_tower(
    origin: BlockPos,
    rotation: JigsawRotation,
    random: &mut WorldgenRandom,
    mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
) -> Option<Vec<EndCityPiece>> {
    let mut sizes = [[0; 3]; END_CITY_TEMPLATES.len()];
    for (size, name) in sizes.iter_mut().zip(END_CITY_TEMPLATES) {
        *size = template_size(&qualify_template(name))?;
    }
    let mut generator = Generator {
        sizes,
        ship_created: false,
    };
    let mut pieces = Vec::new();
    let mut last = generator.piece(origin, "base_floor", rotation, true);
    push(&mut pieces, &last);
    last = generator.add_piece(&last, at(-1, 0, -1), "second_floor_1", rotation, false);
    push(&mut pieces, &last);
    last = generator.add_piece(&last, at(-1, 4, -1), "third_floor_1", rotation, false);
    push(&mut pieces, &last);
    last = generator.add_piece(&last, at(-1, 8, -1), "third_roof", rotation, true);
    push(&mut pieces, &last);
    // `EndCityPieces.startHouseTower` initializes one recursion owner per start
    // and caps recursive generations at eight (`EndCityPieces.java:31-351`).
    generator.recursive_children(Section::Tower, 1, &last, at(0, 0, 0), &mut pieces, random);
    Some(pieces)
}

struct Generator {
    sizes: [[i32; 3]; END_CITY_TEMPLATES.len()],
    ship_created: bool,
}

impl Generator {
    fn piece(
        &mut self,
        position: BlockPos,
        name: &str,
        rotation: JigsawRotation,
        overwrite: bool,
    ) -> EndCityPiece {
        let index = END_CITY_TEMPLATES
            .iter()
            .position(|template| *template == name)
            .expect("End City pieces only use built-in templates");
        EndCityPiece::new(
            qualify_template(name),
            position,
            rotation,
            overwrite,
            self.sizes[index],
        )
    }

    fn add_piece(
        &mut self,
        parent: &EndCityPiece,
        offset: BlockPos,
        name: &str,
        rotation: JigsawRotation,
        overwrite: bool,
    ) -> EndCityPiece {
        // `calculateConnectedPosition` subtracts transformed ZERO here
        // (`StructureTemplate.java:239-249`).
        let relative = rotate_block_pos(offset, parent.rotation());
        self.piece(
            add(parent.template_position(), relative),
            name,
            rotation,
            overwrite,
        )
    }

    fn recursive_children(
        &mut self,
        section: Section,
        depth: i32,
        parent: &EndCityPiece,
        offset: BlockPos,
        pieces: &mut Vec<EndCityPiece>,
        random: &mut WorldgenRandom,
    ) -> bool {
        if depth > 8 {
            return false;
        }
        let mut children = Vec::new();
        if !self.generate(section, depth, parent, offset, &mut children, random) {
            return false;
        }
        let child_tag = random.next_bits(32) as i32;
        for child in &mut children {
            child.set_gen_depth(child_tag);
            let child_bounds = child.bounding_box();
            let collision = pieces
                .iter()
                .find(|existing| existing.bounding_box().intersects(child_bounds));
            if let Some(existing) = collision {
                if existing.gen_depth() != parent.gen_depth() {
                    return false;
                }
            }
        }
        pieces.extend(children);
        true
    }

    fn generate(
        &mut self,
        section: Section,
        depth: i32,
        parent: &EndCityPiece,
        offset: BlockPos,
        pieces: &mut Vec<EndCityPiece>,
        random: &mut WorldgenRandom,
    ) -> bool {
        match section {
            Section::HouseTower => self.house_tower(depth, parent, offset, pieces, random),
            Section::Tower => self.tower(depth, parent, pieces, random),
            Section::TowerBridge => self.tower_bridge(depth, parent, pieces, random),
            Section::FatTower => self.fat_tower(depth, parent, pieces, random),
        }
    }

    fn house_tower(
        &mut self,
        depth: i32,
        parent: &EndCityPiece,
        offset: BlockPos,
        pieces: &mut Vec<EndCityPiece>,
        random: &mut WorldgenRandom,
    ) -> bool {
        if depth > 8 {
            return false;
        }
        let rotation = parent.rotation();
        let mut last = self.add_piece(parent, offset, "base_floor", rotation, true);
        push(pieces, &last);
        match random.next_int(3) {
            0 => {
                last = self.add_piece(&last, at(-1, 4, -1), "base_roof", rotation, true);
                push(pieces, &last);
            }
            1 => {
                last = self.add_piece(&last, at(-1, 0, -1), "second_floor_2", rotation, false);
                push(pieces, &last);
                last = self.add_piece(&last, at(-1, 8, -1), "second_roof", rotation, false);
                push(pieces, &last);
                self.recursive_children(
                    Section::Tower,
                    depth + 1,
                    &last,
                    at(0, 0, 0),
                    pieces,
                    random,
                );
            }
            _ => {
                last = self.add_piece(&last, at(-1, 0, -1), "second_floor_2", rotation, false);
                push(pieces, &last);
                last = self.add_piece(&last, at(-1, 4, -1), "third_floor_2", rotation, false);
                push(pieces, &last);
                last = self.add_piece(&last, at(-1, 8, -1), "third_roof", rotation, true);
                push(pieces, &last);
                self.recursive_children(
                    Section::Tower,
                    depth + 1,
                    &last,
                    at(0, 0, 0),
                    pieces,
                    random,
                );
            }
        }
        true
    }

    fn tower(
        &mut self,
        depth: i32,
        parent: &EndCityPiece,
        pieces: &mut Vec<EndCityPiece>,
        random: &mut WorldgenRandom,
    ) -> bool {
        let rotation = parent.rotation();
        let mut last = self.add_piece(
            parent,
            at(3 + random.next_int(2), -3, 3 + random.next_int(2)),
            "tower_base",
            rotation,
            true,
        );
        push(pieces, &last);
        last = self.add_piece(&last, at(0, 7, 0), "tower_piece", rotation, true);
        push(pieces, &last);
        let mut bridge = (random.next_int(3) == 0).then(|| last.clone());
        let tower_height = 1 + random.next_int(3);
        for index in 0..tower_height {
            last = self.add_piece(&last, at(0, 4, 0), "tower_piece", rotation, true);
            push(pieces, &last);
            if index < tower_height - 1 && random.next_boolean() {
                bridge = Some(last.clone());
            }
        }
        if let Some(bridge) = bridge {
            for (applied, offset) in TOWER_BRIDGES {
                if random.next_boolean() {
                    let bridge_start = self.add_piece(
                        &bridge,
                        offset,
                        "bridge_end",
                        compose_rotation(rotation, applied),
                        true,
                    );
                    push(pieces, &bridge_start);
                    self.recursive_children(
                        Section::TowerBridge,
                        depth + 1,
                        &bridge_start,
                        at(0, 0, 0),
                        pieces,
                        random,
                    );
                }
            }
            last = self.add_piece(&last, at(-1, 4, -1), "tower_top", rotation, true);
            push(pieces, &last);
        } else if depth != 7 {
            return self.recursive_children(
                Section::FatTower,
                depth + 1,
                &last,
                at(0, 0, 0),
                pieces,
                random,
            );
        } else {
            last = self.add_piece(&last, at(-1, 4, -1), "tower_top", rotation, true);
            push(pieces, &last);
        }
        true
    }

    fn tower_bridge(
        &mut self,
        depth: i32,
        parent: &EndCityPiece,
        pieces: &mut Vec<EndCityPiece>,
        random: &mut WorldgenRandom,
    ) -> bool {
        let rotation = parent.rotation();
        let bridge_length = random.next_int(4) + 1;
        let mut last = self.add_piece(parent, at(0, 0, -4), "bridge_piece", rotation, true);
        last.set_gen_depth(-1);
        push(pieces, &last);
        let mut next_y = 0;
        for _ in 0..bridge_length {
            let (name, z, following_y) = if random.next_boolean() {
                ("bridge_piece", -4, 0)
            } else if random.next_boolean() {
                ("bridge_steep_stairs", -4, 4)
            } else {
                ("bridge_gentle_stairs", -8, 4)
            };
            last = self.add_piece(&last, at(0, next_y, z), name, rotation, true);
            push(pieces, &last);
            next_y = following_y;
        }
        if !self.ship_created && random.next_int(10 - depth) == 0 {
            let ship = self.add_piece(
                &last,
                at(-8 + random.next_int(8), next_y, -70 + random.next_int(10)),
                "ship",
                rotation,
                true,
            );
            push(pieces, &ship);
            self.ship_created = true;
        } else if !self.recursive_children(
            Section::HouseTower,
            depth + 1,
            &last,
            at(-3, next_y + 1, -11),
            pieces,
            random,
        ) {
            return false;
        }
        last = self.add_piece(
            &last,
            at(4, next_y, 0),
            "bridge_end",
            compose_rotation(rotation, JigsawRotation::Clockwise180),
            true,
        );
        last.set_gen_depth(-1);
        push(pieces, &last);
        true
    }

    fn fat_tower(
        &mut self,
        depth: i32,
        parent: &EndCityPiece,
        pieces: &mut Vec<EndCityPiece>,
        random: &mut WorldgenRandom,
    ) -> bool {
        let rotation = parent.rotation();
        let mut last = self.add_piece(parent, at(-3, 4, -3), "fat_tower_base", rotation, true);
        push(pieces, &last);
        last = self.add_piece(&last, at(0, 4, 0), "fat_tower_middle", rotation, true);
        push(pieces, &last);
        for _ in 0..2 {
            if random.next_int(3) == 0 {
                break;
            }
            last = self.add_piece(&last, at(0, 8, 0), "fat_tower_middle", rotation, true);
            push(pieces, &last);
            for (applied, offset) in FAT_TOWER_BRIDGES {
                if random.next_boolean() {
                    let bridge_start = self.add_piece(
                        &last,
                        offset,
                        "bridge_end",
                        compose_rotation(rotation, applied),
                        true,
                    );
                    push(pieces, &bridge_start);
                    self.recursive_children(
                        Section::TowerBridge,
                        depth + 1,
                        &bridge_start,
                        at(0, 0, 0),
                        pieces,
                        random,
                    );
                }
            }
        }
        last = self.add_piece(&last, at(-2, 8, -2), "fat_tower_top", rotation, true);
        push(pieces, &last);
        true
    }
}

fn push(pieces: &mut Vec<EndCityPiece>, piece: &EndCityPiece) {
    pieces.push(piece.clone());
}

const fn at(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

fn add(left: BlockPos, right: BlockPos) -> BlockPos {
    at(left.x + right.x, left.y + right.y, left.z + right.z)
}
