//! The dragon's 24-node flight graph (`EnderDragon.findClosestNode` and
//! `findPath`, `EnderDragon.java:555-714`).

use bbb_server_types::geometry::{mth_cos, mth_sin};

use crate::world::simulation_level::LevelCtx;

const NODE_COUNT: usize = 24;

/// `EnderDragon.nodeAdjacency` (`EnderDragon.java:582-605`).
const ADJACENCY: [u32; NODE_COUNT] = [
    6146, 8197, 8202, 16404, 32808, 32848, 65696, 131392, 131712, 263424, 526848, 525313, 1581057,
    3166214, 2138120, 6373424, 4358208, 12910976, 9044480, 9706496, 15216640, 13688832, 11763712,
    8257536,
];

pub(super) type Node = [i32; 3];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DragonNodes([Node; NODE_COUNT]);

/// `Path` over dragon nodes: `advance` steps `next` along `nodes`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DragonPath {
    nodes: Vec<Node>,
    next: usize,
}

impl DragonPath {
    pub(super) fn is_done(&self) -> bool {
        self.next >= self.nodes.len()
    }

    pub(super) fn advance(&mut self) {
        self.next += 1;
    }

    pub(super) fn next_node(&self) -> Node {
        self.nodes[self.next]
    }
}

fn distance_sqr(a: Node, b: Node) -> f32 {
    let d = |i: usize| (a[i] - b[i]) as f32;
    d(0) * d(0) + d(1) * d(1) + d(2) * d(2)
}

fn distance(a: Node, b: Node) -> f32 {
    distance_sqr(a, b).sqrt()
}

impl DragonNodes {
    /// The lazy node build of `findClosestNode()` (`EnderDragon.java:555-609`):
    /// three rings at radii 60, 40 and 20 over the terrain height.
    pub(super) fn build(level: &LevelCtx<'_, '_>) -> Self {
        Self(std::array::from_fn(|i| {
            let (multiplier, radius, y_adjustment, step) = if i < 12 {
                (i, 60.0, 5, std::f32::consts::PI / 12.0)
            } else if i < 20 {
                (i - 12, 40.0, 15, std::f32::consts::PI / 8.0)
            } else {
                (i - 20, 20.0, 5, std::f32::consts::PI / 4.0)
            };
            let angle = f64::from(2.0 * (-std::f32::consts::PI + step * multiplier as f32));
            let x = (radius * mth_cos(angle)).floor() as i32;
            let z = (radius * mth_sin(angle)).floor() as i32;
            let height = level.motion_blocking_no_leaves_height(x, z).unwrap_or(0);
            [x, 73.max(height + y_adjustment), z]
        }))
    }

    /// `findClosestNode(x, y, z)` (`EnderDragon.java:611-631`).
    pub(super) fn closest(&self, x: f64, y: f64, z: f64, crystals_alive: bool) -> usize {
        let current = [x.floor() as i32, y.floor() as i32, z.floor() as i32];
        let start = if crystals_alive { 0 } else { 12 };
        let mut closest = (10000.0_f32, 0);
        for index in start..NODE_COUNT {
            let dist = distance_sqr(self.0[index], current);
            if dist < closest.0 {
                closest = (dist, index);
            }
        }
        closest.1
    }

    /// `findPath` (`EnderDragon.java:633-714`): A* over the adjacency masks,
    /// falling back to the node closest to the goal, with `final_node`
    /// appended after the last node.
    pub(super) fn find_path(
        &self,
        start: usize,
        end: usize,
        final_node: Option<Node>,
        crystals_alive: bool,
    ) -> Option<DragonPath> {
        let nodes = &self.0;
        let minimum = if crystals_alive { 0 } else { 12 };
        let mut g = [0.0_f32; NODE_COUNT];
        let mut f = [0.0_f32; NODE_COUNT];
        let mut came_from = [None::<usize>; NODE_COUNT];
        let mut closed = [false; NODE_COUNT];
        let mut open: Vec<usize> = vec![start];
        f[start] = distance(nodes[start], nodes[end]);
        let mut closest = start;
        let mut reached = None;
        while let Some(slot) = (0..open.len()).min_by(|&a, &b| f[open[a]].total_cmp(&f[open[b]])) {
            let current = open.swap_remove(slot);
            if nodes[current] == nodes[end] {
                reached = Some(end);
                break;
            }
            if distance(nodes[current], nodes[end]) < distance(nodes[closest], nodes[end]) {
                closest = current;
            }
            closed[current] = true;
            for next in minimum..NODE_COUNT {
                if ADJACENCY[current] & (1 << next) == 0 || closed[next] {
                    continue;
                }
                let tentative = g[current] + distance(nodes[current], nodes[next]);
                let in_open = open.contains(&next);
                if !in_open || tentative < g[next] {
                    came_from[next] = Some(current);
                    g[next] = tentative;
                    f[next] = tentative + distance(nodes[next], nodes[end]);
                    if !in_open {
                        open.push(next);
                    }
                }
            }
        }
        let last = match reached {
            Some(end) => end,
            None if closest == start => return None,
            None => closest,
        };
        let mut path = vec![nodes[last]];
        let mut cursor = last;
        while let Some(previous) = came_from[cursor] {
            path.push(nodes[previous]);
            cursor = previous;
        }
        path.reverse();
        path.extend(final_node);
        Some(DragonPath {
            nodes: path,
            next: 0,
        })
    }
}
