//! `/spreadplayers`: `SpreadPlayersCommand` (SpreadPlayersCommand.java:46-375).

use crate::block_behavior::state_liquid;
use crate::random::GameRandom;
use crate::tick_rate::java_fixed_decimals;
use bbb_protocol::{component_literal_nbt, packets::BlockPos, NbtSummaryValue};
use bbb_server_types::fluid::block_state_is_air;

use crate::chunk::ChunkStore;
use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::types::TeleportMove;
use crate::commands::{translatable, CommandCx, CommandSource};
use crate::world::ServerWorld;

use super::{float_argument, select_damage_entity_states, vec2_argument, CommandRun, Outcome};
use crate::commands::types::CommandDamageEntityRef;
use crate::commands::CommandOrigin;

/// `SpreadPlayersCommand.MAX_ITERATION_COUNT` (SpreadPlayersCommand.java:35).
const MAX_ITERATION_COUNT: i32 = 10000;

pub(super) fn spread_players(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    max_height: Option<i32>,
) -> Outcome {
    let (moves, report, count) = plan_spread(parse, source, cx.world, &cx.origin, max_height)?;
    for teleport in moves {
        cx.world.teleport_by_command(teleport, cx.events);
    }
    run.succeed(report, true);
    Ok(count)
}

fn plan_spread(
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    max_height: Option<i32>,
) -> Result<(Vec<TeleportMove>, NbtSummaryValue, i32), NbtSummaryValue> {
    let (min_y, max_y) = at.y_bounds(world);
    world.with_entity_reads(|reads| {
        let center = vec2_argument(parse, "center").position(at);
        // `Vec2Argument.getVec2` (Vec2Argument.java:38-41) keeps X/Z as a `Vec2`.
        let center_x = center[0] as f32;
        let center_z = center[2] as f32;
        let spread_distance = float_argument(parse, "spreadDistance");
        let max_range = float_argument(parse, "maxRange");
        let respect_teams = boolean_argument(parse, "respectTeams");
        let targets = entity_targets(parse, source, reads, at)?;
        // SpreadPlayersCommand.java:66, `:111-115`: the bare form uses
        // `getMaxY() + 1`; `under` uses the typed height and refuses below minY.
        let max_height = max_height.unwrap_or(max_y + 1);
        if max_height < min_y {
            return Err(translatable(
                "commands.spreadplayers.failed.invalid.height",
                vec![
                    NbtSummaryValue::Int(max_height),
                    NbtSummaryValue::Int(min_y),
                ],
            ));
        }

        // Vanilla draws `RandomSource.createThreadLocalInstance()`
        // (SpreadPlayersCommand.java:117); borrow the level's live RNG so
        // repeated calls in the same tick do not replay identical layouts.
        let random = reads.random(&at.level);
        let min_x = f64::from(center_x) - f64::from(max_range);
        let min_z = f64::from(center_z) - f64::from(max_range);
        let max_x = f64::from(center_x) + f64::from(max_range);
        let max_z = f64::from(center_z) + f64::from(max_range);
        let count = if respect_teams {
            team_count(&targets, reads.scoreboard)
        } else {
            targets.len()
        };
        let mut positions = create_initial_positions(random, count, min_x, min_z, max_x, max_z);
        let area = SpreadArea {
            min_x,
            min_z,
            max_x,
            max_z,
            min_y,
            max_height,
        };
        spread_positions(
            [center_x, center_z],
            f64::from(spread_distance),
            &reads.blocks(&at.level).expect("served command level"),
            random,
            area,
            &mut positions,
            respect_teams,
        )?;
        let mut moves = Vec::with_capacity(targets.len());
        let distance = set_target_positions(
            &mut moves,
            &reads.blocks(&at.level).expect("served command level"),
            reads.scoreboard,
            at,
            &targets,
            &positions,
            area,
            respect_teams,
        );
        let kind = if respect_teams { "teams" } else { "entities" };
        let count = i32::try_from(positions.len()).unwrap_or(i32::MAX);
        let report = translatable(
            format!("commands.spreadplayers.success.{kind}"),
            vec![
                NbtSummaryValue::Int(count),
                NbtSummaryValue::Float(center_x.to_bits()),
                NbtSummaryValue::Float(center_z.to_bits()),
                component_literal_nbt(java_fixed_decimals(distance, 2)),
            ],
        );
        Ok((moves, report, count))
    })
}

fn entity_targets<'a>(
    parse: &ParseResult,
    source: &CommandSource,
    world: &crate::world::EntityReads<'a>,
    at: &CommandOrigin,
) -> Result<Vec<CommandDamageEntityRef<'a>>, NbtSummaryValue> {
    let Some(ArgumentValue::Players(selector)) = parse.argument("targets") else {
        unreachable!("targets is an entity argument");
    };
    let found = select_damage_entity_states(selector, source, at, world)?;
    if found.is_empty() {
        // EntityArgument.getEntities (EntityArgument.java:63-64).
        return Err(translatable("argument.entity.notfound.entity", Vec::new()));
    }
    Ok(found)
}

fn boolean_argument(parse: &ParseResult, name: &str) -> bool {
    match parse.argument(name) {
        Some(ArgumentValue::Boolean(value)) => *value,
        other => unreachable!("{name} is a boolean argument, not {other:?}"),
    }
}

/// `SpreadPlayersCommand.getNumberOfTeams` (SpreadPlayersCommand.java:140-152).
///
/// The set is a `HashSet<Team>` that a non-player always adds `null` to
/// (`:146-148`), so every stored entity together contributes the single null
/// entry a team-less player would, and the count is the distinct teams among
/// the players plus one when anything carries no team at all.
fn team_count(
    targets: &[CommandDamageEntityRef<'_>],
    board: &crate::world::scoreboard::Scoreboard,
) -> usize {
    let mut teams: Vec<&bbb_protocol::JavaString> = Vec::new();
    let mut untamed = false;
    for target in targets {
        match target {
            CommandDamageEntityRef::Player(player) => {
                match board.players_team(player.name).map(|team| &team.name) {
                    Some(team) => {
                        if !teams.contains(&team) {
                            teams.push(team);
                        }
                    }
                    None => untamed = true,
                }
            }
            CommandDamageEntityRef::Stored(_) => untamed = true,
        }
    }
    teams.len() + usize::from(untamed)
}

/// `SpreadPlayersCommand.createInitialPositions` (SpreadPlayersCommand.java:284-296).
fn create_initial_positions(
    random: &GameRandom,
    count: usize,
    min_x: f64,
    min_z: f64,
    max_x: f64,
    max_z: f64,
) -> Vec<Position> {
    (0..count)
        .map(|_| {
            let mut position = Position::default();
            position.randomize(random, min_x, min_z, max_x, max_z);
            position
        })
        .collect()
}

#[derive(Clone, Copy)]
struct SpreadArea {
    min_x: f64,
    min_z: f64,
    max_x: f64,
    max_z: f64,
    min_y: i32,
    max_height: i32,
}

/// `SpreadPlayersCommand.spreadPositions` (SpreadPlayersCommand.java:154-233).
fn spread_positions(
    [center_x, center_z]: [f32; 2],
    spread_dist: f64,
    blocks: &ChunkStore,
    random: &GameRandom,
    area: SpreadArea,
    positions: &mut [Position],
    respect_teams: bool,
) -> Result<(), NbtSummaryValue> {
    let SpreadArea {
        min_x,
        min_z,
        max_x,
        max_z,
        min_y,
        max_height,
    } = area;
    let mut has_collisions = true;
    let mut min_distance = f64::from(f32::MAX);
    let mut iteration = 0;
    while iteration < MAX_ITERATION_COUNT && has_collisions {
        has_collisions = false;
        min_distance = f64::from(f32::MAX);
        for i in 0..positions.len() {
            let mut neighbour_count = 0;
            let mut average = Position::default();
            for j in 0..positions.len() {
                if i == j {
                    continue;
                }
                let dist = positions[i].dist(positions[j]);
                min_distance = min_distance.min(dist);
                if dist < spread_dist {
                    neighbour_count += 1;
                    average.x += positions[j].x - positions[i].x;
                    average.z += positions[j].z - positions[i].z;
                }
            }
            if neighbour_count > 0 {
                average.x /= f64::from(neighbour_count);
                average.z /= f64::from(neighbour_count);
                let length = average.length();
                if length > 0.0 {
                    average.normalize();
                    positions[i].move_away(average);
                } else {
                    positions[i].randomize(random, min_x, min_z, max_x, max_z);
                }
                has_collisions = true;
            }
            if positions[i].clamp(min_x, min_z, max_x, max_z) {
                has_collisions = true;
            }
        }
        if !has_collisions {
            for position in positions.iter_mut() {
                if !position.is_safe(blocks, min_y, max_height) {
                    position.randomize(random, min_x, min_z, max_x, max_z);
                    has_collisions = true;
                }
            }
        }
        iteration += 1;
    }
    if min_distance == f64::from(f32::MAX) {
        min_distance = 0.0;
    }
    if iteration >= MAX_ITERATION_COUNT {
        let kind = if respect_teams { "teams" } else { "entities" };
        return Err(translatable(
            format!("commands.spreadplayers.failed.{kind}"),
            vec![
                NbtSummaryValue::Int(i32::try_from(positions.len()).unwrap_or(i32::MAX)),
                NbtSummaryValue::Float(center_x.to_bits()),
                NbtSummaryValue::Float(center_z.to_bits()),
                component_literal_nbt(java_fixed_decimals(min_distance, 2)),
            ],
        ));
    }
    Ok(())
}

/// `SpreadPlayersCommand.setPlayerPositions` (SpreadPlayersCommand.java:235-282).
#[allow(clippy::too_many_arguments)]
fn set_target_positions(
    moves: &mut Vec<TeleportMove>,
    blocks: &ChunkStore,
    board: &crate::world::scoreboard::Scoreboard,
    at: &CommandOrigin,
    targets: &[CommandDamageEntityRef<'_>],
    positions: &[Position],
    area: SpreadArea,
    respect_teams: bool,
) -> f64 {
    let SpreadArea {
        min_y, max_height, ..
    } = area;
    let mut avg_distance = 0.0;
    let mut next_index = 0;
    let mut team_slots: rustc_hash::FxHashMap<Option<&bbb_protocol::JavaString>, usize> =
        rustc_hash::FxHashMap::default();
    for target in targets {
        let slot = if respect_teams {
            let team = match target {
                CommandDamageEntityRef::Player(player) => {
                    board.players_team(player.name).map(|team| &team.name)
                }
                CommandDamageEntityRef::Stored(_) => None,
            };
            *team_slots.entry(team).or_insert_with(|| {
                let slot = next_index;
                next_index += 1;
                slot
            })
        } else {
            let slot = next_index;
            next_index += 1;
            slot
        };
        let position = positions[slot];
        let dest = [
            f64::from(mth_floor(position.x)) + 0.5,
            f64::from(position.spawn_y(blocks, min_y, max_height)),
            f64::from(mth_floor(position.z)) + 0.5,
        ];
        moves.push(TeleportMove {
            victim: target.attribute_target(),
            level: at.level.clone(),
            position: dest,
            rotation: target.rotation(),
            relatives_mask: 0,
            look_at: None,
            teleport_command_tail: false,
        });
        if positions.len() >= 2 {
            let mut closest = f64::MAX;
            for (test_slot, test) in positions.iter().enumerate() {
                if slot != test_slot {
                    closest = closest.min(position.dist(*test));
                }
            }
            avg_distance += closest;
        }
    }
    if targets.len() < 2 || positions.len() < 2 {
        0.0
    } else {
        avg_distance / targets.len() as f64
    }
}

/// `Mth.floor(double)` (Mth.java:65-67).
fn mth_floor(value: f64) -> i32 {
    value.floor() as i32
}

/// `Mth.nextDouble` (Mth.java:153-155).
fn next_double(random: &GameRandom, min: f64, max: f64) -> f64 {
    if min >= max {
        min
    } else {
        random.next_double() * (max - min) + min
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Position {
    x: f64,
    z: f64,
}

impl Position {
    fn dist(self, target: Self) -> f64 {
        let dx = self.x - target.x;
        let dz = self.z - target.z;
        (dx * dx + dz * dz).sqrt()
    }

    fn length(self) -> f64 {
        (self.x * self.x + self.z * self.z).sqrt()
    }

    fn normalize(&mut self) {
        let dist = self.length();
        self.x /= dist;
        self.z /= dist;
    }

    fn move_away(&mut self, pos: Self) {
        self.x -= pos.x;
        self.z -= pos.z;
    }

    fn clamp(&mut self, min_x: f64, min_z: f64, max_x: f64, max_z: f64) -> bool {
        let mut changed = false;
        if self.x < min_x {
            self.x = min_x;
            changed = true;
        } else if self.x > max_x {
            self.x = max_x;
            changed = true;
        }
        if self.z < min_z {
            self.z = min_z;
            changed = true;
        } else if self.z > max_z {
            self.z = max_z;
            changed = true;
        }
        changed
    }

    /// `SpreadPlayersCommand.Position.getSpawnY` (SpreadPlayersCommand.java:344-362).
    fn spawn_y(self, blocks: &ChunkStore, min_y: i32, max_height: i32) -> i32 {
        let mut y = max_height + 1;
        let mut air2_above = block_state_is_air(blocks.block_state_at(self.block_pos(y)));
        y -= 1;
        let mut air1_above = block_state_is_air(blocks.block_state_at(self.block_pos(y)));
        while y > min_y {
            y -= 1;
            let current_is_air = block_state_is_air(blocks.block_state_at(self.block_pos(y)));
            if !current_is_air && air1_above && air2_above {
                return y + 1;
            }
            air2_above = air1_above;
            air1_above = current_is_air;
        }
        max_height + 1
    }

    /// `SpreadPlayersCommand.Position.isSafe` (SpreadPlayersCommand.java:364-368).
    fn is_safe(self, blocks: &ChunkStore, min_y: i32, max_height: i32) -> bool {
        let pos = self.block_pos(self.spawn_y(blocks, min_y, max_height) - 1);
        let state = blocks.block_state_at(pos);
        pos.y < max_height
            && !state_liquid(state)
            && !blocks.block_tags().block_state_in("minecraft:fire", state)
    }

    fn randomize(&mut self, random: &GameRandom, min_x: f64, min_z: f64, max_x: f64, max_z: f64) {
        self.x = next_double(random, min_x, max_x);
        self.z = next_double(random, min_z, max_z);
    }

    fn block_pos(self, y: i32) -> BlockPos {
        BlockPos {
            x: mth_floor(self.x),
            y,
            z: mth_floor(self.z),
        }
    }
}
