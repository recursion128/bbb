//! `StructureTemplate.placeEntities` for FEATURES proto-entity owners.

use bbb_protocol::{packets::BlockPos, NbtSummaryValue};

use crate::noise::ruined_portal::{transform, RuinedPortalMirror};
use crate::noise::structure_pieces::{JigsawBoundingBox, JigsawRotation};
use crate::WorldGenLevel;
use bbb_server_datapack::structure_template::StructureTemplateEntity;

/// `StructureTemplate.placeEntities` (`StructureTemplate.java:472-508`).
pub fn place_template_entities<L: WorldGenLevel>(
    level: &mut L,
    entities: &[StructureTemplateEntity],
    origin: BlockPos,
    rotation: JigsawRotation,
    pivot: BlockPos,
    chunk_bounds: JigsawBoundingBox,
) {
    place_template_entities_with_settings(
        level,
        entities,
        origin,
        rotation,
        pivot,
        RuinedPortalMirror::None,
        chunk_bounds,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn place_template_entities_with_settings<L: WorldGenLevel>(
    level: &mut L,
    entities: &[StructureTemplateEntity],
    origin: BlockPos,
    rotation: JigsawRotation,
    pivot: BlockPos,
    mirror: RuinedPortalMirror,
    chunk_bounds: JigsawBoundingBox,
    finalize: bool,
) {
    for entity in entities {
        let block = wrapping_add(
            origin,
            transform(entity.block_position, mirror, rotation, pivot),
        );
        if !chunk_bounds.contains(block) {
            continue;
        }
        let relative = transform_vec3(entity_pos(entity), mirror, rotation, pivot);
        let position = [
            relative[0] + f64::from(origin.x),
            relative[1] + f64::from(origin.y),
            relative[2] + f64::from(origin.z),
        ];
        let loaded = template_rotation(&entity.nbt);
        let y_rot = placed_y_rot_with_mirror(loaded[0], rotation, mirror);
        level.add_template_entity_with_finalize(
            &entity.nbt,
            position,
            [y_rot, loaded[1]],
            finalize,
            (
                match rotation {
                    JigsawRotation::None => 0,
                    JigsawRotation::Clockwise90 => 1,
                    JigsawRotation::Clockwise180 => 2,
                    JigsawRotation::CounterClockwise90 => 3,
                },
                match mirror {
                    RuinedPortalMirror::None => {
                        bbb_protocol::block_state_types::VanillaBlockStateMirror::None
                    }
                    RuinedPortalMirror::LeftRight => {
                        bbb_protocol::block_state_types::VanillaBlockStateMirror::LeftRight
                    }
                    RuinedPortalMirror::FrontBack => {
                        bbb_protocol::block_state_types::VanillaBlockStateMirror::FrontBack
                    }
                },
            ),
        );
    }
}

fn entity_pos(entity: &StructureTemplateEntity) -> [f64; 3] {
    entity.position_bits.map(f64::from_bits)
}

fn template_rotation(nbt: &NbtSummaryValue) -> [f32; 2] {
    let NbtSummaryValue::Compound(entries) = nbt else {
        return [0.0, 0.0];
    };
    let Some(NbtSummaryValue::List(values)) = entries
        .iter()
        .find(|entry| entry.name == "Rotation")
        .map(|entry| &entry.value)
    else {
        return [0.0, 0.0];
    };
    let yaw = values.first().and_then(float_value).unwrap_or(0.0);
    let pitch = values.get(1).and_then(float_value).unwrap_or(0.0);
    [yaw, pitch]
}

fn float_value(value: &NbtSummaryValue) -> Option<f32> {
    match value {
        NbtSummaryValue::Float(bits) => Some(f32::from_bits(*bits)),
        NbtSummaryValue::Double(bits) => Some(f64::from_bits(*bits) as f32),
        _ => None,
    }
}

/// `Entity.rotate` then the `placeEntities` mirror adjustment with the configured mirror
/// (`Entity.java:3398-3416`, `StructureTemplate.java:495-496`).
fn placed_y_rot_with_mirror(
    loaded_y_rot: f32,
    rotation: JigsawRotation,
    mirror: RuinedPortalMirror,
) -> f32 {
    let wrapped = wrap_degrees(loaded_y_rot);
    let rotated = match rotation {
        JigsawRotation::Clockwise180 => wrapped + 180.0,
        JigsawRotation::CounterClockwise90 => wrapped + 270.0,
        JigsawRotation::Clockwise90 => wrapped + 90.0,
        JigsawRotation::None => wrapped,
    };
    let mirrored = match mirror {
        RuinedPortalMirror::None => wrapped,
        RuinedPortalMirror::FrontBack => -wrapped,
        RuinedPortalMirror::LeftRight => 180.0 - wrapped,
    };
    rotated + mirrored - loaded_y_rot
}

/// `StructureTemplate.transform(Vec3, mirror, rotation, pivot)` (`StructureTemplate.java:558-586`).
fn transform_vec3(
    pos: [f64; 3],
    mirror: RuinedPortalMirror,
    rotation: JigsawRotation,
    pivot: BlockPos,
) -> [f64; 3] {
    let [mut x, y, mut z] = pos;
    match mirror {
        RuinedPortalMirror::None => {}
        RuinedPortalMirror::LeftRight => z = 1.0 - z,
        RuinedPortalMirror::FrontBack => x = 1.0 - x,
    }
    let pivot_x = f64::from(pivot.x);
    let pivot_z = f64::from(pivot.z);
    match rotation {
        JigsawRotation::CounterClockwise90 => {
            [pivot_x - pivot_z + z, y, pivot_x + pivot_z + 1.0 - x]
        }
        JigsawRotation::Clockwise90 => [pivot_x + pivot_z + 1.0 - z, y, pivot_z - pivot_x + x],
        JigsawRotation::Clockwise180 => {
            [pivot_x + pivot_x + 1.0 - x, y, pivot_z + pivot_z + 1.0 - z]
        }
        JigsawRotation::None => [x, y, z],
    }
}

fn wrapping_add(left: BlockPos, right: BlockPos) -> BlockPos {
    BlockPos {
        x: left.x.wrapping_add(right.x),
        y: left.y.wrapping_add(right.y),
        z: left.z.wrapping_add(right.z),
    }
}

/// `Mth.wrapDegrees(float)` (`Mth.java:215-225`).
fn wrap_degrees(angle: f32) -> f32 {
    let mut normalized = angle % 360.0;
    if normalized >= 180.0 {
        normalized -= 360.0;
    }
    if normalized < -180.0 {
        normalized += 360.0;
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::NbtSummaryEntry;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    #[test]
    fn vec3_clock_90_matches_vanilla() {
        let pivot = BlockPos { x: 3, y: 6, z: 7 };
        assert_eq!(
            transform_vec3(
                [2.5, 1.0, 1.5],
                RuinedPortalMirror::None,
                JigsawRotation::Clockwise90,
                pivot
            ),
            [3.0 + 7.0 + 1.0 - 1.5, 1.0, 7.0 - 3.0 + 2.5]
        );
    }

    #[test]
    fn none_rotation_keeps_loaded_yaw() {
        assert_eq!(
            placed_y_rot_with_mirror(-4.3276978, JigsawRotation::None, RuinedPortalMirror::None),
            -4.3276978
        );
    }

    struct CaptureLevel {
        added: Vec<(NbtSummaryValue, [f64; 3], [f32; 2])>,
    }

    impl WorldGenLevel for CaptureLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }
        fn get_block(&self, _pos: BlockPos) -> i32 {
            0
        }
        fn set_block(&mut self, _pos: BlockPos, _state: i32, _update_flags: i32) -> bool {
            true
        }
        fn get_min_y(&self) -> i32 {
            -64
        }
        fn get_max_y(&self) -> i32 {
            319
        }
        fn get_sea_level(&self) -> i32 {
            63
        }
        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}
        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}
        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}
        fn add_template_entity(
            &mut self,
            nbt: &NbtSummaryValue,
            position: [f64; 3],
            rotation: [f32; 2],
        ) {
            self.added.push((nbt.clone(), position, rotation));
        }
    }

    #[test]
    fn places_owned_entity_inside_chunk_bounds() {
        let entity = StructureTemplateEntity {
            position_bits: [2.5_f64.to_bits(), 1.0_f64.to_bits(), 1.5_f64.to_bits()],
            block_position: BlockPos { x: 2, y: 1, z: 1 },
            nbt: NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::string("minecraft:villager")),
                entry(
                    "Rotation",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Float((-4.3276978_f32).to_bits()),
                        NbtSummaryValue::Float(0.0_f32.to_bits()),
                    ]),
                ),
            ]),
        };
        let mut level = CaptureLevel { added: Vec::new() };
        place_template_entities(
            &mut level,
            &[entity],
            BlockPos {
                x: 10,
                y: 64,
                z: 20,
            },
            JigsawRotation::None,
            BlockPos { x: 3, y: 6, z: 7 },
            JigsawBoundingBox {
                min: BlockPos { x: 0, y: 0, z: 0 },
                max: BlockPos {
                    x: 32,
                    y: 128,
                    z: 32,
                },
            },
        );
        assert_eq!(level.added.len(), 1);
        assert_eq!(level.added[0].1, [12.5, 65.0, 21.5]);
        assert_eq!(level.added[0].2[0], -4.3276978);
    }

    #[test]
    fn skips_entities_outside_the_chunk_bb() {
        let entity = StructureTemplateEntity {
            position_bits: [0; 3],
            block_position: BlockPos { x: 8, y: 1, z: 8 },
            nbt: NbtSummaryValue::Compound(vec![entry(
                "id",
                NbtSummaryValue::string("minecraft:villager"),
            )]),
        };
        let mut level = CaptureLevel { added: Vec::new() };
        place_template_entities(
            &mut level,
            &[entity],
            BlockPos { x: 0, y: 0, z: 0 },
            JigsawRotation::None,
            BlockPos { x: 0, y: 0, z: 0 },
            JigsawBoundingBox {
                min: BlockPos { x: 0, y: 0, z: 0 },
                max: BlockPos { x: 4, y: 4, z: 4 },
            },
        );
        assert!(level.added.is_empty());
    }
}
