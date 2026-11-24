use super::*;

pub(super) fn append_redstone_torch_animate_effect(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    kind: RedstoneTorchAnimateKind,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("lit").map(String::as_str) != Some("true") {
        return;
    }
    let direction = match kind {
        RedstoneTorchAnimateKind::Floor => [0, 0, 0],
        RedstoneTorchAnimateKind::Wall => {
            let Some([x, 0, z]) = properties
                .get("facing")
                .and_then(|facing| block_state_direction(facing))
            else {
                return;
            };
            [-x, 0, -z]
        }
    };
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::dust(REDSTONE_PARTICLE_COLOR, 1.0),
        position: [
            f64::from(pos.x)
                + 0.5
                + (random.next_double() - 0.5) * 0.2
                + 0.27 * f64::from(direction[0]),
            f64::from(pos.y)
                + 0.7
                + (random.next_double() - 0.5) * 0.2
                + if kind == RedstoneTorchAnimateKind::Wall {
                    0.22
                } else {
                    0.0
                },
            f64::from(pos.z)
                + 0.5
                + (random.next_double() - 0.5) * 0.2
                + 0.27 * f64::from(direction[2]),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_repeater_animate_effect(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("powered").map(String::as_str) != Some("true") {
        return;
    }
    let Some([step_x, 0, step_z]) = properties
        .get("facing")
        .and_then(|facing| block_state_direction(facing))
    else {
        return;
    };
    let Some(delay) = properties
        .get("delay")
        .and_then(|delay| delay.parse::<i32>().ok())
        .filter(|delay| (1..=4).contains(delay))
    else {
        return;
    };

    let x = f64::from(pos.x) + 0.5 + (random.next_double() - 0.5) * 0.2;
    let y = f64::from(pos.y) + 0.4 + (random.next_double() - 0.5) * 0.2;
    let z = f64::from(pos.z) + 0.5 + (random.next_double() - 0.5) * 0.2;
    let mut offset = -5.0_f32;
    if random.next_boolean() {
        offset = (delay * 2 - 1) as f32;
    }
    offset /= 16.0;

    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::dust(REDSTONE_PARTICLE_COLOR, 1.0),
        position: [
            x + f64::from(offset * step_x as f32),
            y,
            z + f64::from(offset * step_z as f32),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_redstone_wire_animate_effects(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let Some(power) = properties
        .get("power")
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|power| *power <= 15)
    else {
        return;
    };
    if power == 0 {
        return;
    }
    let [Some(north), Some(east), Some(south), Some(west)] =
        REDSTONE_WIRE_DIRECTIONS.map(|direction| {
            redstone_wire_side(properties.get(direction.property).map(String::as_str))
        })
    else {
        return;
    };
    let sides = [north, east, south, west];

    let color = redstone_wire_color(power);
    for (direction, side) in REDSTONE_WIRE_DIRECTIONS.into_iter().zip(sides) {
        if side == RedstoneWireSide::Up {
            append_redstone_wire_particle_along_line(
                pos,
                color,
                direction.step,
                [0, 1, 0],
                -0.5,
                0.5,
                random,
                spawns,
            );
        }
        let (from, to) = if side == RedstoneWireSide::None {
            (0.0, 0.3)
        } else {
            (0.0, 0.5)
        };
        append_redstone_wire_particle_along_line(
            pos,
            color,
            [0, -1, 0],
            direction.step,
            from,
            to,
            random,
            spawns,
        );
    }
}

pub(super) fn redstone_wire_side(value: Option<&str>) -> Option<RedstoneWireSide> {
    match value {
        Some("up") => Some(RedstoneWireSide::Up),
        Some("side") => Some(RedstoneWireSide::Side),
        Some("none") => Some(RedstoneWireSide::None),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn append_redstone_wire_particle_along_line(
    pos: BlockPos,
    color: u32,
    side: [i32; 3],
    along: [i32; 3],
    from: f32,
    to: f32,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let span = to - from;
    if random.next_float() >= 0.2 * span {
        return;
    }
    let position_on_line = from + span * random.next_float();
    let coordinate = |axis: usize| {
        0.5 + f64::from(0.4375_f32 * side[axis] as f32)
            + f64::from(position_on_line * along[axis] as f32)
    };
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::dust(color, 1.0),
        position: [
            f64::from(pos.x) + coordinate(0),
            f64::from(pos.y) + coordinate(1),
            f64::from(pos.z) + coordinate(2),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn redstone_wire_color(power: u8) -> u32 {
    let power = f32::from(power) / 15.0;
    let red = power * 0.6 + if power > 0.0 { 0.4 } else { 0.3 };
    let green = (power * power * 0.7 - 0.5).clamp(0.0, 1.0);
    let blue = (power * power * 0.6 - 0.7).clamp(0.0, 1.0);
    let channel = |value: f32| (value * 255.0).floor() as u32;
    (channel(1.0) << 24) | (channel(red) << 16) | (channel(green) << 8) | channel(blue)
}

pub(super) fn append_redstone_ore_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    level_random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("lit").map(String::as_str) != Some("true") {
        return;
    }
    let solid_render_faces = redstone_ore_solid_render_faces(world, pos);
    for position in
        redstone_ore_particle_positions(pos, solid_render_faces, || level_random.next_float())
            .into_iter()
            .flatten()
    {
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::dust(REDSTONE_PARTICLE_COLOR, 1.0),
            position,
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
}
