//! Jigsaw-owned structure terrain density before NOISE integration.

use std::sync::OnceLock;

use bbb_protocol::packets::BlockPos;

use bbb_server_datapack::structure_pool::StructurePoolProjection;

use super::structure_pieces::{element_projection, JigsawBoundingBox, JigsawGraphJunction};
use super::structure_starts::JigsawStructureStart;
use super::structures::StructureTerrainAdjustment;

const BEARD_KERNEL_RADIUS: i32 = 12;
const BEARD_KERNEL_SIZE: i32 = 24;
const AFFECTED_BOX_INFLATION: i32 = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerrainAdjustingStart {
    Jigsaw {
        start: JigsawStructureStart,
    },
    Rigid {
        terrain_adjustment: StructureTerrainAdjustment,
        pieces: Vec<JigsawBoundingBox>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rigid {
    bounding_box: JigsawBoundingBox,
    terrain_adjustment: StructureTerrainAdjustment,
    ground_level_delta: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JigsawBeardifier {
    rigids: Vec<Rigid>,
    junctions: Vec<JigsawGraphJunction>,
    affected_box: Option<JigsawBoundingBox>,
}

impl JigsawBeardifier {
    /// Consumes the starts already selected by the target chunk's structure
    /// references. Rigid pieces and junctions follow Vanilla's distinct edge
    /// tests (`Beardifier.java:38-87`, `StructurePiece.java:122-125`).
    pub fn from_referenced_starts<'a>(
        chunk_x: i32,
        chunk_z: i32,
        starts: impl IntoIterator<Item = &'a JigsawStructureStart>,
    ) -> Self {
        let starts = starts
            .into_iter()
            .map(|start| TerrainAdjustingStart::Jigsaw {
                start: start.clone(),
            })
            .collect::<Vec<_>>();
        Self::from_ordered_terrain_starts(chunk_x, chunk_z, &starts)
    }

    /// Builds the ordered Pool/non-Pool input selected by StructureManager
    /// (`StructureManager.java:50-80`, `Beardifier.java:38-87`).
    pub fn from_ordered_terrain_starts<'a>(
        chunk_x: i32,
        chunk_z: i32,
        starts: impl IntoIterator<Item = &'a TerrainAdjustingStart>,
    ) -> Self {
        let close_min_x = chunk_x * 16 - BEARD_KERNEL_RADIUS;
        let close_min_z = chunk_z * 16 - BEARD_KERNEL_RADIUS;
        let close_max_x = chunk_x * 16 + 15 + BEARD_KERNEL_RADIUS;
        let close_max_z = chunk_z * 16 + 15 + BEARD_KERNEL_RADIUS;

        let mut rigids = Vec::new();
        let mut junctions = Vec::new();
        let mut affected_box = None;
        for start in starts {
            match start {
                TerrainAdjustingStart::Jigsaw { start } => {
                    let terrain_adjustment = start.terrain_adjustment();
                    if terrain_adjustment == StructureTerrainAdjustment::None {
                        continue;
                    }
                    for (piece_index, piece) in start.pieces().iter().enumerate() {
                        if !piece_is_close(
                            piece.bounding_box,
                            close_min_x,
                            close_min_z,
                            close_max_x,
                            close_max_z,
                        ) {
                            continue;
                        }
                        if element_projection(&piece.element) == StructurePoolProjection::Rigid {
                            rigids.push(Rigid {
                                bounding_box: piece.bounding_box,
                                terrain_adjustment,
                                ground_level_delta: piece.ground_level_delta,
                            });
                            affected_box = Some(include_box(affected_box, piece.bounding_box));
                        }
                        for junction in start
                            .junctions()
                            .iter()
                            .filter(|junction| junction.piece_index == piece_index)
                        {
                            if !junction_is_close(
                                junction,
                                close_min_x,
                                close_min_z,
                                close_max_x,
                                close_max_z,
                            ) {
                                continue;
                            }
                            junctions.push(junction.clone());
                            let point = JigsawBoundingBox {
                                min: BlockPos {
                                    x: junction.source_x,
                                    y: junction.source_ground_y,
                                    z: junction.source_z,
                                },
                                max: BlockPos {
                                    x: junction.source_x,
                                    y: junction.source_ground_y,
                                    z: junction.source_z,
                                },
                            };
                            affected_box = Some(include_box(affected_box, point));
                        }
                    }
                }
                TerrainAdjustingStart::Rigid {
                    terrain_adjustment,
                    pieces,
                } => {
                    if *terrain_adjustment == StructureTerrainAdjustment::None {
                        continue;
                    }
                    for &bounding_box in pieces {
                        if !piece_is_close(
                            bounding_box,
                            close_min_x,
                            close_min_z,
                            close_max_x,
                            close_max_z,
                        ) {
                            continue;
                        }
                        rigids.push(Rigid {
                            bounding_box,
                            terrain_adjustment: *terrain_adjustment,
                            ground_level_delta: 0,
                        });
                        affected_box = Some(include_box(affected_box, bounding_box));
                    }
                }
            }
        }
        Self {
            rigids,
            junctions,
            affected_box: affected_box.map(inflate_affected),
        }
    }

    /// `Beardifier.compute` (`Beardifier.java:111-155`).
    pub fn sample(&self, block_x: i32, block_y: i32, block_z: i32) -> f64 {
        if !self.affected_box.is_some_and(|bounds| {
            bounds.contains(BlockPos {
                x: block_x,
                y: block_y,
                z: block_z,
            })
        }) {
            return 0.0;
        }
        let mut noise_value = 0.0;
        for rigid in &self.rigids {
            let bounds = rigid.bounding_box;
            let dx = 0.max(
                bounds
                    .min
                    .x
                    .wrapping_sub(block_x)
                    .max(block_x.wrapping_sub(bounds.max.x)),
            );
            let dz = 0.max(
                bounds
                    .min
                    .z
                    .wrapping_sub(block_z)
                    .max(block_z.wrapping_sub(bounds.max.z)),
            );
            let ground_y = bounds.min.y.wrapping_add(rigid.ground_level_delta);
            let dy_to_ground = block_y.wrapping_sub(ground_y);
            let dy = match rigid.terrain_adjustment {
                StructureTerrainAdjustment::None => 0,
                StructureTerrainAdjustment::Bury | StructureTerrainAdjustment::BeardThin => {
                    dy_to_ground
                }
                StructureTerrainAdjustment::BeardBox => 0.max(
                    ground_y
                        .wrapping_sub(block_y)
                        .max(block_y.wrapping_sub(bounds.max.y)),
                ),
                StructureTerrainAdjustment::Encapsulate => 0.max(
                    bounds
                        .min
                        .y
                        .wrapping_sub(block_y)
                        .max(block_y.wrapping_sub(bounds.max.y)),
                ),
            };
            noise_value += match rigid.terrain_adjustment {
                StructureTerrainAdjustment::None => 0.0,
                StructureTerrainAdjustment::Bury => {
                    bury_contribution(f64::from(dx), f64::from(dy) / 2.0, f64::from(dz))
                }
                StructureTerrainAdjustment::BeardThin | StructureTerrainAdjustment::BeardBox => {
                    beard_contribution(dx, dy, dz, dy_to_ground) * 0.8
                }
                StructureTerrainAdjustment::Encapsulate => {
                    bury_contribution(
                        f64::from(dx) / 2.0,
                        f64::from(dy) / 2.0,
                        f64::from(dz) / 2.0,
                    ) * 0.8
                }
            };
        }
        for junction in &self.junctions {
            let dx = block_x.wrapping_sub(junction.source_x);
            let dy = block_y.wrapping_sub(junction.source_ground_y);
            let dz = block_z.wrapping_sub(junction.source_z);
            noise_value += beard_contribution(dx, dy, dz, dy) * 0.4;
        }
        noise_value
    }
}

fn piece_is_close(
    bounds: JigsawBoundingBox,
    min_x: i32,
    min_z: i32,
    max_x: i32,
    max_z: i32,
) -> bool {
    bounds.min.x <= max_x && bounds.max.x >= min_x && bounds.min.z <= max_z && bounds.max.z >= min_z
}

fn junction_is_close(
    junction: &JigsawGraphJunction,
    min_x: i32,
    min_z: i32,
    max_x: i32,
    max_z: i32,
) -> bool {
    junction.source_x > min_x
        && junction.source_z > min_z
        && junction.source_x < max_x
        && junction.source_z < max_z
}

fn include_box(
    encompassing: Option<JigsawBoundingBox>,
    other: JigsawBoundingBox,
) -> JigsawBoundingBox {
    let Some(mut encompassing) = encompassing else {
        return other;
    };
    encompassing.min.x = encompassing.min.x.min(other.min.x);
    encompassing.min.y = encompassing.min.y.min(other.min.y);
    encompassing.min.z = encompassing.min.z.min(other.min.z);
    encompassing.max.x = encompassing.max.x.max(other.max.x);
    encompassing.max.y = encompassing.max.y.max(other.max.y);
    encompassing.max.z = encompassing.max.z.max(other.max.z);
    encompassing
}

fn inflate_affected(bounds: JigsawBoundingBox) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: bounds.min.x - AFFECTED_BOX_INFLATION,
            y: bounds.min.y - AFFECTED_BOX_INFLATION,
            z: bounds.min.z - AFFECTED_BOX_INFLATION,
        },
        max: BlockPos {
            x: bounds.max.x + AFFECTED_BOX_INFLATION,
            y: bounds.max.y + AFFECTED_BOX_INFLATION,
            z: bounds.max.z + AFFECTED_BOX_INFLATION,
        },
    }
}

/// `Beardifier.getBuryContribution` (`Beardifier.java:168-171`).
fn bury_contribution(dx: f64, dy: f64, dz: f64) -> f64 {
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    if distance <= 0.0 {
        1.0
    } else if distance >= 6.0 {
        0.0
    } else {
        1.0 - distance / 6.0
    }
}

/// `Beardifier.getBeardContribution` (`Beardifier.java:173-185`).
fn beard_contribution(dx: i32, dy: i32, dz: i32, y_to_ground: i32) -> f64 {
    let Some(xi) = dx.checked_add(BEARD_KERNEL_RADIUS) else {
        return 0.0;
    };
    let Some(yi) = dy.checked_add(BEARD_KERNEL_RADIUS) else {
        return 0.0;
    };
    let Some(zi) = dz.checked_add(BEARD_KERNEL_RADIUS) else {
        return 0.0;
    };
    if !(0..BEARD_KERNEL_SIZE).contains(&xi)
        || !(0..BEARD_KERNEL_SIZE).contains(&yi)
        || !(0..BEARD_KERNEL_SIZE).contains(&zi)
    {
        return 0.0;
    }
    let dy_with_offset = f64::from(y_to_ground) + 0.5;
    let distance_squared = f64::from(dx) * f64::from(dx)
        + dy_with_offset * dy_with_offset
        + f64::from(dz) * f64::from(dz);
    let value = -dy_with_offset * fast_inv_sqrt(distance_squared / 2.0) / 2.0;
    let index =
        usize::try_from(zi * BEARD_KERNEL_SIZE * BEARD_KERNEL_SIZE + xi * BEARD_KERNEL_SIZE + yi)
            .expect("kernel coordinates are non-negative and bounded");
    value * f64::from(beard_kernel()[index])
}

fn beard_kernel() -> &'static [f32] {
    static KERNEL: OnceLock<Box<[f32]>> = OnceLock::new();
    KERNEL.get_or_init(|| {
        let mut kernel = vec![0.0; usize::try_from(BEARD_KERNEL_SIZE.pow(3)).unwrap()];
        for zi in 0..BEARD_KERNEL_SIZE {
            for xi in 0..BEARD_KERNEL_SIZE {
                for yi in 0..BEARD_KERNEL_SIZE {
                    let dx = xi - BEARD_KERNEL_RADIUS;
                    let dy = f64::from(yi - BEARD_KERNEL_RADIUS) + 0.5;
                    let dz = zi - BEARD_KERNEL_RADIUS;
                    let distance_squared = f64::from(dx * dx + dz * dz) + dy * dy;
                    let index = usize::try_from(
                        zi * BEARD_KERNEL_SIZE * BEARD_KERNEL_SIZE + xi * BEARD_KERNEL_SIZE + yi,
                    )
                    .unwrap();
                    kernel[index] = (-distance_squared / 16.0).exp() as f32;
                }
            }
        }
        kernel.into_boxed_slice()
    })
}

/// `Mth.fastInvSqrt(double)` (`Mth.java:431-437`).
fn fast_inv_sqrt(value: f64) -> f64 {
    let half = 0.5 * value;
    let bits = 6_910_469_410_427_058_090_i64 - ((value.to_bits() as i64) >> 1);
    let estimate = f64::from_bits(bits as u64);
    estimate * (1.5 - half * estimate * estimate)
}

#[cfg(test)]
mod tests {
    use crate::noise::structure_pieces::JigsawHeightAccess;
    use crate::noise::structures::{
        NoiseStructurePlan, StructureHeightmap, StructureTerrainAdjustment,
    };
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;

    struct FlatHeight;

    impl JigsawHeightAccess for FlatHeight {
        fn first_free_height(
            &self,
            _x: i32,
            _z: i32,
            _heightmap: StructureHeightmap,
        ) -> Option<i32> {
            Some(64)
        }
    }

    fn box_at(min: [i32; 3], max: [i32; 3]) -> JigsawBoundingBox {
        JigsawBoundingBox {
            min: BlockPos {
                x: min[0],
                y: min[1],
                z: min[2],
            },
            max: BlockPos {
                x: max[0],
                y: max[1],
                z: max[2],
            },
        }
    }

    fn single_rigid(adjustment: StructureTerrainAdjustment) -> JigsawBeardifier {
        JigsawBeardifier {
            rigids: vec![Rigid {
                bounding_box: box_at([0, 10, 0], [2, 14, 2]),
                terrain_adjustment: adjustment,
                ground_level_delta: 2,
            }],
            junctions: Vec::new(),
            affected_box: Some(box_at([-24, -14, -24], [26, 38, 26])),
        }
    }

    #[test]
    fn empty_and_affected_box_short_circuit_to_zero() {
        assert_eq!(JigsawBeardifier::default().sample(0, 0, 0), 0.0);
        let bury = single_rigid(StructureTerrainAdjustment::Bury);
        assert_eq!(bury.sample(27, 12, 0), 0.0);
        assert_eq!(bury.sample(0, 12, 0), 1.0);
        assert_eq!(bury.sample(0, 24, 0), 0.0);
    }

    #[test]
    fn piece_and_junction_edges_keep_their_distinct_inclusivity() {
        assert!(piece_is_close(
            box_at([-12, 0, 27], [-12, 0, 27]),
            -12,
            -12,
            27,
            27
        ));
        assert!(!piece_is_close(
            box_at([-13, 0, 28], [-13, 0, 28]),
            -12,
            -12,
            27,
            27
        ));
        let junction = |x, z| JigsawGraphJunction {
            piece_index: 0,
            source_x: x,
            source_ground_y: 0,
            source_z: z,
            delta_y: 0,
            destination_projection: StructurePoolProjection::Rigid,
        };
        assert!(!junction_is_close(&junction(-12, 0), -12, -12, 27, 27));
        assert!(!junction_is_close(&junction(27, 0), -12, -12, 27, 27));
        assert!(junction_is_close(&junction(-11, 26), -12, -12, 27, 27));
    }

    #[test]
    fn every_terrain_adjustment_and_junction_uses_its_vanilla_weight() {
        assert_eq!(
            single_rigid(StructureTerrainAdjustment::Encapsulate).sample(0, 12, 0),
            0.8
        );
        let thin = single_rigid(StructureTerrainAdjustment::BeardThin).sample(0, 12, 0);
        let boxed = single_rigid(StructureTerrainAdjustment::BeardBox).sample(0, 12, 0);
        assert_eq!(thin.to_bits(), boxed.to_bits());
        // Java 26.1 formula probes lock the f32 kernel promotion and fast inverse sqrt.
        assert_eq!(thin.to_bits(), 13_826_562_246_658_327_255);
        assert_eq!(
            single_rigid(StructureTerrainAdjustment::BeardThin)
                .sample(0, 11, 0)
                .to_bits(),
            4_603_190_209_803_551_447
        );

        let junction = JigsawGraphJunction {
            piece_index: 0,
            source_x: 0,
            source_ground_y: 12,
            source_z: 0,
            delta_y: 7,
            destination_projection: StructurePoolProjection::TerrainMatching,
        };
        let component = JigsawBeardifier {
            rigids: Vec::new(),
            junctions: vec![junction],
            affected_box: Some(box_at([-24, -12, -24], [24, 36, 24])),
        };
        assert_eq!(
            component.sample(0, 12, 0).to_bits(),
            13_822_058_647_030_956_759
        );
    }

    #[test]
    fn built_in_village_start_collects_rigids_and_ordered_junctions() {
        let startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap();
        let plan = NoiseStructurePlan::from_startup(&startup, &["minecraft:plains"]).unwrap();
        let graph = plan
            .jigsaw_piece_graph(
                &startup,
                "minecraft:village_plains",
                0,
                0,
                0,
                -64,
                384,
                &FlatHeight,
            )
            .unwrap();
        let start = JigsawStructureStart::from_graph(&plan, graph, 0).unwrap();
        let component = JigsawBeardifier::from_referenced_starts(0, 0, [&start]);
        let close = |bounds| piece_is_close(bounds, -12, -12, 27, 27);
        let mut expected_affected = None;
        let expected_rigids = start
            .pieces()
            .iter()
            .filter(|piece| {
                let retained = close(piece.bounding_box)
                    && element_projection(&piece.element) == StructurePoolProjection::Rigid;
                if retained {
                    expected_affected = Some(include_box(expected_affected, piece.bounding_box));
                }
                retained
            })
            .count();
        let expected_junctions = start
            .junctions()
            .iter()
            .filter(|junction| {
                let retained = close(start.pieces()[junction.piece_index].bounding_box)
                    && junction_is_close(junction, -12, -12, 27, 27);
                if retained {
                    let point = box_at(
                        [
                            junction.source_x,
                            junction.source_ground_y,
                            junction.source_z,
                        ],
                        [
                            junction.source_x,
                            junction.source_ground_y,
                            junction.source_z,
                        ],
                    );
                    expected_affected = Some(include_box(expected_affected, point));
                }
                retained
            })
            .count();
        assert_eq!(component.rigids.len(), expected_rigids);
        assert_eq!(component.junctions.len(), expected_junctions);
        assert!(component.junctions.iter().any(|junction| {
            element_projection(&start.pieces()[junction.piece_index].element)
                == StructurePoolProjection::TerrainMatching
        }));
        assert_eq!(
            component.affected_box,
            Some(inflate_affected(expected_affected.unwrap()))
        );
        assert!(component.sample(8, 64, 8).is_finite());
    }
}
