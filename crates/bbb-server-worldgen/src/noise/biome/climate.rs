//! Seven-dimensional `Climate.ParameterList` search.

use std::cmp::Ordering;

const DIMENSIONS: usize = 7;
pub const CHILDREN_PER_NODE: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub min: i64,
    pub max: i64,
}

impl Parameter {
    pub const fn point(value: i64) -> Self {
        Self {
            min: value,
            max: value,
        }
    }

    pub fn span(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    fn distance(self, target: i64) -> i64 {
        let above = target - self.max;
        let below = self.min - target;
        if above > 0 {
            above
        } else {
            below.max(0)
        }
    }

    pub fn center(self) -> i64 {
        (self.min + self.max) / 2
    }
}

#[derive(Clone, Debug)]
pub struct ParameterPoint {
    pub space: [Parameter; DIMENSIONS],
    pub biome: i32,
}

impl ParameterPoint {
    pub fn new(ranges: [[i64; 2]; 6], offset: i64, biome: i32) -> Self {
        Self {
            space: [
                Parameter {
                    min: ranges[0][0],
                    max: ranges[0][1],
                },
                Parameter {
                    min: ranges[1][0],
                    max: ranges[1][1],
                },
                Parameter {
                    min: ranges[2][0],
                    max: ranges[2][1],
                },
                Parameter {
                    min: ranges[3][0],
                    max: ranges[3][1],
                },
                Parameter {
                    min: ranges[4][0],
                    max: ranges[4][1],
                },
                Parameter {
                    min: ranges[5][0],
                    max: ranges[5][1],
                },
                Parameter::point(offset),
            ],
            biome,
        }
    }

    pub fn fitness(&self, target: &[i64; DIMENSIONS]) -> i64 {
        self.space
            .iter()
            .zip(target)
            .map(|(parameter, target)| parameter.distance(*target).pow(2))
            .sum()
    }
}

pub enum NodeKind {
    Leaf(usize),
    Branch(Vec<usize>),
}

pub struct Node {
    space: [Parameter; DIMENSIONS],
    kind: NodeKind,
}

/// Exact `Climate.RTree` ownership, including its stable build order and strict
/// pruning. The caller owns the task-local last-result state.
pub struct ParameterTree {
    nodes: Vec<Node>,
    root: usize,
}

impl ParameterTree {
    pub fn new(points: &[ParameterPoint]) -> Self {
        assert!(
            !points.is_empty(),
            "parameter list is validated as non-empty"
        );
        let mut nodes = points
            .iter()
            .enumerate()
            .map(|(index, point)| Node {
                space: point.space,
                kind: NodeKind::Leaf(index),
            })
            .collect::<Vec<_>>();
        let children = (0..nodes.len()).collect();
        let root = build(&mut nodes, children);
        Self { nodes, root }
    }

    pub fn search(
        &self,
        points: &[ParameterPoint],
        target: &[i64; DIMENSIONS],
        last_result: &mut Option<usize>,
    ) -> usize {
        let candidate = last_result.map(|point| point);
        let winner = self.search_node(self.root, points, target, candidate);
        *last_result = Some(winner);
        winner
    }

    fn search_node(
        &self,
        node_id: usize,
        points: &[ParameterPoint],
        target: &[i64; DIMENSIONS],
        candidate: Option<usize>,
    ) -> usize {
        let node = &self.nodes[node_id];
        match &node.kind {
            NodeKind::Leaf(point) => *point,
            NodeKind::Branch(children) => {
                let mut closest = candidate;
                let mut min_distance = candidate
                    .map(|point| points[point].fitness(target))
                    .unwrap_or(i64::MAX);
                for child_id in children {
                    let child = &self.nodes[*child_id];
                    let child_distance = space_distance(&child.space, target);
                    if min_distance > child_distance {
                        let leaf = self.search_node(*child_id, points, target, closest);
                        let leaf_distance = match child.kind {
                            NodeKind::Leaf(point) if point == leaf => child_distance,
                            _ => points[leaf].fitness(target),
                        };
                        if min_distance > leaf_distance {
                            min_distance = leaf_distance;
                            closest = Some(leaf);
                        }
                    }
                }
                closest.expect("a non-empty parameter tree always finds a leaf")
            }
        }
    }
}

pub fn build(nodes: &mut Vec<Node>, mut children: Vec<usize>) -> usize {
    if children.len() == 1 {
        return children[0];
    }
    if children.len() <= CHILDREN_PER_NODE {
        children.sort_by_key(|child| magnitude(&nodes[*child].space));
        return push_branch(nodes, children);
    }

    let mut min_cost = i64::MAX;
    let mut min_dimension = 0;
    let mut min_buckets = Vec::new();
    for dimension in 0..DIMENSIONS {
        sort_nodes(nodes, &mut children, dimension, false);
        let buckets = bucketize(&children);
        let cost = buckets
            .iter()
            .map(|bucket| space_cost(&combined_space(nodes, bucket)))
            .sum();
        if min_cost > cost {
            min_cost = cost;
            min_dimension = dimension;
            min_buckets = buckets;
        }
    }
    min_buckets.sort_by(|first, second| {
        compare_space(
            &combined_space(nodes, first),
            &combined_space(nodes, second),
            min_dimension,
            true,
        )
    });
    let child_branches = min_buckets
        .into_iter()
        .map(|bucket| build(nodes, bucket))
        .collect();
    push_branch(nodes, child_branches)
}

pub fn push_branch(nodes: &mut Vec<Node>, children: Vec<usize>) -> usize {
    let space = combined_space(nodes, &children);
    let id = nodes.len();
    nodes.push(Node {
        space,
        kind: NodeKind::Branch(children),
    });
    id
}

pub fn bucketize(nodes: &[usize]) -> Vec<Vec<usize>> {
    let expected = 6_usize.pow(((nodes.len() as f64 - 0.01).ln() / 6_f64.ln()).floor() as u32);
    nodes.chunks(expected).map(<[usize]>::to_vec).collect()
}

pub fn sort_nodes(nodes: &[Node], children: &mut [usize], dimension: usize, absolute: bool) {
    children.sort_by(|first, second| {
        compare_space(
            &nodes[*first].space,
            &nodes[*second].space,
            dimension,
            absolute,
        )
    });
}

pub fn compare_space(
    first: &[Parameter; DIMENSIONS],
    second: &[Parameter; DIMENSIONS],
    dimension: usize,
    absolute: bool,
) -> Ordering {
    for offset in 0..DIMENSIONS {
        let index = (dimension + offset) % DIMENSIONS;
        let first = first[index].center();
        let second = second[index].center();
        let first = if absolute { first.abs() } else { first };
        let second = if absolute { second.abs() } else { second };
        match first.cmp(&second) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    Ordering::Equal
}

pub fn combined_space(nodes: &[Node], children: &[usize]) -> [Parameter; DIMENSIONS] {
    let mut result = nodes[children[0]].space;
    for child in &children[1..] {
        for (result, parameter) in result.iter_mut().zip(nodes[*child].space) {
            *result = result.span(parameter);
        }
    }
    result
}

pub fn magnitude(space: &[Parameter; DIMENSIONS]) -> i64 {
    space.iter().map(|parameter| parameter.center().abs()).sum()
}

pub fn space_cost(space: &[Parameter; DIMENSIONS]) -> i64 {
    space
        .iter()
        .map(|parameter| (parameter.max - parameter.min).abs())
        .sum()
}

fn space_distance(space: &[Parameter; DIMENSIONS], target: &[i64; DIMENSIONS]) -> i64 {
    space
        .iter()
        .zip(target)
        .map(|(parameter, target)| parameter.distance(*target).pow(2))
        .sum()
}

/// Noise settings `spawn_target`, already validated at datapack load.
/// Each climate field is an interval codec: a point, `[min, max]` or
/// `{min, max}` (`ExtraCodecs.java:271-295`); `offset` is a scalar.
pub fn spawn_targets(
    values: &[bbb_server_datapack::worldgen::noise::SpawnTarget],
) -> Vec<ParameterPoint> {
    values
        .iter()
        .map(|point| {
            ParameterPoint::new(
                point.ranges.map(|range| range.map(quantize_coord)),
                quantize_coord(point.offset),
                0,
            )
        })
        .collect()
}

/// `Climate.quantizeCoord` (`Climate.java:77-79`).
fn quantize_coord(value: f32) -> i64 {
    (value * 10_000.0_f32) as i64
}

/// `Climate.SpawnFinder` (`Climate.java:503-556`): the block column whose
/// zero-depth climate best fits a spawn target, biased toward the origin.
/// `sample` answers the climate target at a block column.
pub fn find_spawn_position(
    targets: &[ParameterPoint],
    mut sample: impl FnMut(i32, i32) -> [i64; DIMENSIONS],
) -> [i32; 2] {
    if targets.is_empty() {
        return [0, 0];
    }
    let mut fitness = |x: i32, z: i32| {
        let mut target = sample(x, z);
        target[4] = 0;
        let fit = targets
            .iter()
            .map(|point| point.fitness(&target))
            .min()
            .unwrap_or(i64::MAX);
        let bias = i64::from(x).pow(2) + i64::from(z).pow(2);
        fit.wrapping_mul(2048 * 2048).wrapping_add(bias)
    };
    let mut best = ([0, 0], fitness(0, 0));
    for (max_radius, increment) in [(2048.0_f32, 512.0_f32), (512.0, 32.0)] {
        let origin = best.0;
        let mut angle = 0.0_f32;
        let mut radius = increment;
        while radius <= max_radius {
            let x = origin[0] + (f64::from(angle).sin() * f64::from(radius)) as i32;
            let z = origin[1] + (f64::from(angle).cos() * f64::from(radius)) as i32;
            let candidate = fitness(x, z);
            if candidate < best.1 {
                best = ([x, z], candidate);
            }
            angle += increment / radius;
            if f64::from(angle) > std::f64::consts::PI * 2.0 {
                angle = 0.0;
                radius += increment;
            }
        }
    }
    best.0
}

#[cfg(test)]
mod tests {
    use super::{ParameterPoint, ParameterTree};

    #[test]
    fn strict_ties_keep_the_task_local_last_result() {
        let points = vec![
            ParameterPoint::new([[0, 0]; 6], 0, 10),
            ParameterPoint::new([[0, 0]; 6], 0, 20),
        ];
        let tree = ParameterTree::new(&points);
        let mut cold = None;
        assert_eq!(tree.search(&points, &[0; 7], &mut cold), 0);
        let mut warm = Some(1);
        assert_eq!(tree.search(&points, &[0; 7], &mut warm), 1);
    }

    #[test]
    fn interval_distance_and_offset_use_all_seven_dimensions() {
        let points = [
            ParameterPoint::new([[0, 10]; 6], 5, 10),
            ParameterPoint::new([[20, 30]; 6], 0, 20),
        ];
        assert_eq!(points[0].fitness(&[5, 5, 5, 5, 5, 5, 0]), 25);
        assert_eq!(points[1].fitness(&[15, 15, 15, 15, 15, 15, 0]), 150);
    }
}
