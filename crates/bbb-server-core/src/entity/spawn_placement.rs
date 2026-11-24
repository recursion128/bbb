use crate::random::GameRandom;
use bbb_protocol::packets::BlockPos;

use crate::world::SpawnPredicateLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnPlacementType {
    NoRestrictions,
    InWater,
    InLava,
    OnGround,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnHeightmap {
    MotionBlocking,
    MotionBlockingNoLeaves,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnPredicateKind {
    Unrestricted,
    Animal,
    Mob,
    Monster,
    AnyLightMonster,
    SurfaceMonster,
    SurfaceWaterAnimal,
    SurfaceAgeableWaterCreature,
    Axolotl,
    Drowned,
    Guardian,
    TropicalFish,
    Armadillo,
    Bat,
    Camel,
    Endermite,
    Frog,
    Ghast,
    GlowSquid,
    Goat,
    MagmaCube,
    MushroomCow,
    Nautilus,
    Ocelot,
    Parrot,
    Hoglin,
    Piglin,
    PatrollingMonster,
    PolarBear,
    Rabbit,
    Silverfish,
    SkeletonHorse,
    Slime,
    Stray,
    Strider,
    Turtle,
    Wolf,
    ZombifiedPiglin,
    Fox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnPredicateReason {
    Natural,
    ChunkGeneration,
    Spawner,
    TrialSpawner,
    Reinforcement,
    Event,
    Patrol,
}

impl SpawnPredicateReason {
    const fn is_spawner(self) -> bool {
        matches!(self, Self::Spawner | Self::TrialSpawner)
    }

    const fn ignores_light_requirements(self) -> bool {
        matches!(self, Self::TrialSpawner)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnPlacementEntry {
    pub placement: SpawnPlacementType,
    pub heightmap: SpawnHeightmap,
    pub predicate: SpawnPredicateKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnFluid {
    Empty,
    Water,
    Lava,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnPlacementEnvironment {
    pub inside_world_border: bool,
    pub fluid_at: SpawnFluid,
    pub above_is_redstone_conductor: bool,
    pub below_is_valid_spawn: bool,
    pub at_is_valid_empty_spawn_block: bool,
    pub above_is_valid_empty_spawn_block: bool,
}

impl Default for SpawnPlacementEnvironment {
    fn default() -> Self {
        Self {
            inside_world_border: true,
            fluid_at: SpawnFluid::Empty,
            above_is_redstone_conductor: false,
            below_is_valid_spawn: true,
            at_is_valid_empty_spawn_block: true,
            above_is_valid_empty_spawn_block: true,
        }
    }
}

// `SpawnPlacementTypes.java:11-49`.
pub const fn is_spawn_position_ok(
    placement: SpawnPlacementType,
    environment: SpawnPlacementEnvironment,
) -> bool {
    if matches!(placement, SpawnPlacementType::NoRestrictions) {
        return true;
    }
    if !environment.inside_world_border {
        return false;
    }
    match placement {
        SpawnPlacementType::NoRestrictions => unreachable!(),
        SpawnPlacementType::InWater => {
            matches!(environment.fluid_at, SpawnFluid::Water)
                && !environment.above_is_redstone_conductor
        }
        SpawnPlacementType::InLava => matches!(environment.fluid_at, SpawnFluid::Lava),
        SpawnPlacementType::OnGround => {
            environment.below_is_valid_spawn
                && environment.at_is_valid_empty_spawn_block
                && environment.above_is_valid_empty_spawn_block
        }
    }
}

pub fn spawn_placement(resource_id: &str) -> SpawnPlacementEntry {
    crate::entity::natural_spawn_facts_26_1::vanilla_natural_spawn_type_facts_26_1(resource_id)
        .map_or(
            SpawnPlacementEntry {
                placement: SpawnPlacementType::NoRestrictions,
                heightmap: SpawnHeightmap::MotionBlockingNoLeaves,
                predicate: SpawnPredicateKind::Unrestricted,
            },
            |facts| facts.placement,
        )
}

pub fn spawn_placement_by_id(entity_type_id: i32) -> SpawnPlacementEntry {
    crate::entity::natural_spawn_facts_26_1::vanilla_natural_spawn_type_facts_by_id_26_1(
        entity_type_id,
    )
    .map_or(
        SpawnPlacementEntry {
            placement: SpawnPlacementType::NoRestrictions,
            heightmap: SpawnHeightmap::MotionBlockingNoLeaves,
            predicate: SpawnPredicateKind::Unrestricted,
        },
        |facts| facts.placement,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnPlacementProbe {
    pub entity_type_id: i32,
    pub position: BlockPos,
    pub entry: SpawnPlacementEntry,
}

/// `SpawnPlacements.checkSpawnRules` invokes the registered predicate, or
/// accepts an unregistered type (`SpawnPlacements.java:74-79`).
pub(crate) fn check_spawn_placement_predicate(
    environment: &SpawnPredicateLevel<'_>,
    probe: SpawnPlacementProbe,
    reason: SpawnPredicateReason,
    random: &GameRandom,
) -> bool {
    use SpawnPredicateKind::*;
    let pos = probe.position;
    match probe.entry.predicate {
        Unrestricted => true,
        Mob => check_mob_spawn_rules(environment, probe.entity_type_id, pos, reason),
        Animal => {
            check_animal_floor_and_light(environment, pos, reason, "minecraft:animals_spawnable_on")
        }
        Monster => {
            check_monster_spawn_rules(environment, probe.entity_type_id, pos, reason, random)
        }
        AnyLightMonster => {
            check_any_light_monster_spawn_rules(environment, probe.entity_type_id, pos, reason)
        }
        SurfaceMonster => {
            check_monster_spawn_rules(environment, probe.entity_type_id, pos, reason, random)
                && (reason.is_spawner() || can_see_sky(environment, pos))
        }
        // `AgeableWaterCreature.java:66-79`, `WaterAnimal.java:70-83`.
        SurfaceWaterAnimal | SurfaceAgeableWaterCreature => {
            check_surface_water_spawn_rules(environment, pos, 13, 0)
        }
        // `Axolotl.java:558-566`, `Guardian.java:304-311`,
        // `TropicalFish.java:259-268`, `GlowSquid.java:110-118`,
        // `AbstractNautilus.java:134-147`.
        Axolotl => environment.block_matches_tag(below(pos), "minecraft:axolotls_spawnable_on"),
        Drowned => check_drowned_spawn_rules(environment, pos, reason, random),
        Guardian => {
            (random.next_int(20) == 0 || !can_see_sky_from_below_water(environment, pos))
                && !environment.is_peaceful()
                && (reason.is_spawner() || environment.fluid_is_water(pos))
                && environment.fluid_is_water(below(pos))
        }
        TropicalFish => {
            environment.fluid_is_water(below(pos))
                && environment.block_is(above(pos), "minecraft:water")
                && (environment
                    .biome_matches_tag(pos, "minecraft:allows_tropical_fish_spawns_at_any_height")
                    || check_surface_water_spawn_rules(environment, pos, 13, 0))
        }
        GlowSquid => {
            pos.y <= environment.sea_level() - 33
                && raw_brightness(environment, pos, 0) == 0
                && environment.block_is(pos, "minecraft:water")
        }
        Nautilus => check_surface_water_spawn_rules(environment, pos, 25, 5),
        // `Strider.java:96-110`.
        Strider => check_strider_spawn_rules(environment, pos),
        // `Armadillo.java:229-233`, `Camel.java:139-143`, `Frog.java:337-341`,
        // `Goat.java:348-352`, `MushroomCow.java:69-73`, `Parrot.java:306-310`,
        // `Rabbit.java:434-438`, `Wolf.java:649-653`, `Fox.java:324-328`.
        Armadillo => check_floor_and_light(environment, pos, "minecraft:armadillo_spawnable_on"),
        Camel => check_floor_and_light(environment, pos, "minecraft:camels_spawnable_on"),
        Frog => check_floor_and_light(environment, pos, "minecraft:frogs_spawnable_on"),
        Goat => check_floor_and_light(environment, pos, "minecraft:goats_spawnable_on"),
        MushroomCow => check_floor_and_light(environment, pos, "minecraft:mooshrooms_spawnable_on"),
        // `Ocelot.java:225-229`.
        Ocelot => random.next_int(3) != 0,
        Parrot => check_floor_and_light(environment, pos, "minecraft:parrots_spawnable_on"),
        PolarBear => {
            if environment.biome_matches_tag(pos, "minecraft:polar_bears_spawn_on_alternate_blocks")
            {
                check_floor_and_light(
                    environment,
                    pos,
                    "minecraft:polar_bears_spawnable_on_alternate",
                )
            } else {
                check_animal_floor_and_light(
                    environment,
                    pos,
                    reason,
                    "minecraft:animals_spawnable_on",
                )
            }
        }
        Rabbit => check_floor_and_light(environment, pos, "minecraft:rabbits_spawnable_on"),
        // `SkeletonHorse.java:50-56`, `Turtle.java:139-143`.
        SkeletonHorse => {
            if reason.is_spawner() {
                reason.ignores_light_requirements() || bright_enough(environment, pos)
            } else {
                check_animal_floor_and_light(
                    environment,
                    pos,
                    reason,
                    "minecraft:animals_spawnable_on",
                )
            }
        }
        Turtle => {
            pos.y < environment.sea_level() + 4
                && environment.block_matches_tag(below(pos), "minecraft:sand")
                && bright_enough(environment, pos)
        }
        Wolf => check_floor_and_light(environment, pos, "minecraft:wolves_spawnable_on"),
        Fox => check_floor_and_light(environment, pos, "minecraft:foxes_spawnable_on"),
        Bat => check_bat_spawn_rules(environment, probe.entity_type_id, pos, reason, random),
        // `Endermite.java:131-144`, `Silverfish.java:112-125`.
        Endermite | Silverfish => {
            check_any_light_monster_spawn_rules(environment, probe.entity_type_id, pos, reason)
                && (reason.is_spawner() || !environment.has_nearby_alive_player(pos, 5.0))
        }
        // `Ghast.java:149-153`, `MagmaCube.java:31-35`, `Hoglin.java:172-176`,
        // `Piglin.java:167-171`, `PatrollingMonster.java:87-95`,
        // `ZombifiedPiglin.java:170-174`.
        Ghast => {
            !environment.is_peaceful()
                && random.next_int(20) == 0
                && check_mob_spawn_rules(environment, probe.entity_type_id, pos, reason)
        }
        MagmaCube => !environment.is_peaceful(),
        Hoglin | Piglin => !environment.block_is(below(pos), "minecraft:nether_wart_block"),
        PatrollingMonster => {
            environment.block_light(pos) <= 8
                && check_any_light_monster_spawn_rules(
                    environment,
                    probe.entity_type_id,
                    pos,
                    reason,
                )
        }
        Slime => check_slime_spawn_rules(environment, probe.entity_type_id, pos, reason, random),
        Stray => check_stray_spawn_rules(environment, probe.entity_type_id, pos, reason, random),
        ZombifiedPiglin => {
            !environment.is_peaceful()
                && !environment.block_is(below(pos), "minecraft:nether_wart_block")
        }
    }
}

const fn below(pos: BlockPos) -> BlockPos {
    BlockPos {
        y: pos.y - 1,
        ..pos
    }
}

const fn above(pos: BlockPos) -> BlockPos {
    BlockPos {
        y: pos.y + 1,
        ..pos
    }
}

fn raw_brightness(environment: &SpawnPredicateLevel<'_>, pos: BlockPos, darkening: u8) -> u8 {
    environment
        .block_light(pos)
        .max(environment.sky_light(pos).saturating_sub(darkening))
}

fn bright_enough(environment: &SpawnPredicateLevel<'_>, pos: BlockPos) -> bool {
    raw_brightness(environment, pos, 0) > 8
}

fn check_floor_and_light(environment: &SpawnPredicateLevel<'_>, pos: BlockPos, tag: &str) -> bool {
    environment.block_matches_tag(below(pos), tag) && bright_enough(environment, pos)
}

// `Animal.java:110-119`; the species-specific copies do not apply the
// spawn-reason light exemption (`Armadillo.java:229-233`, `Wolf.java:649-653`).
fn check_animal_floor_and_light(
    environment: &SpawnPredicateLevel<'_>,
    pos: BlockPos,
    reason: SpawnPredicateReason,
    tag: &str,
) -> bool {
    environment.block_matches_tag(below(pos), tag)
        && (reason.ignores_light_requirements() || bright_enough(environment, pos))
}

fn check_mob_spawn_rules(
    environment: &SpawnPredicateLevel<'_>,
    entity_type_id: i32,
    pos: BlockPos,
    reason: SpawnPredicateReason,
) -> bool {
    reason.is_spawner() || environment.mob_spawn_floor_is_valid(below(pos), entity_type_id)
}

// `Monster.java:86-119`.
fn check_monster_spawn_rules(
    environment: &SpawnPredicateLevel<'_>,
    entity_type_id: i32,
    pos: BlockPos,
    reason: SpawnPredicateReason,
    random: &GameRandom,
) -> bool {
    if environment.is_peaceful() {
        return false;
    }
    if !reason.ignores_light_requirements() {
        if environment.sky_light(pos) as i32 > random.next_int(32) {
            return false;
        }
        let Some(block_limit) = environment.monster_spawn_block_light_limit() else {
            return false;
        };
        if block_limit < 15 && environment.block_light(pos) > block_limit {
            return false;
        }
        let darkening = if environment.is_thundering() {
            10
        } else {
            environment.sky_darken()
        };
        let brightness = raw_brightness(environment, pos, darkening);
        let Some(light_test) = environment.sample_monster_spawn_light_test(random) else {
            return false;
        };
        if brightness > light_test {
            return false;
        }
    }
    check_mob_spawn_rules(environment, entity_type_id, pos, reason)
}

fn check_any_light_monster_spawn_rules(
    environment: &SpawnPredicateLevel<'_>,
    entity_type_id: i32,
    pos: BlockPos,
    reason: SpawnPredicateReason,
) -> bool {
    !environment.is_peaceful() && check_mob_spawn_rules(environment, entity_type_id, pos, reason)
}

fn check_surface_water_spawn_rules(
    environment: &SpawnPredicateLevel<'_>,
    pos: BlockPos,
    below_sea: i32,
    below_top: i32,
) -> bool {
    let sea = environment.sea_level();
    pos.y >= sea - below_sea
        && pos.y <= sea - below_top
        && environment.fluid_is_water(below(pos))
        && environment.block_is(above(pos), "minecraft:water")
}

// `Drowned.java:136-158`.
fn check_drowned_spawn_rules(
    environment: &SpawnPredicateLevel<'_>,
    pos: BlockPos,
    reason: SpawnPredicateReason,
    random: &GameRandom,
) -> bool {
    if !environment.fluid_is_water(below(pos)) && !reason.is_spawner() {
        return false;
    }
    let can_spawn = !environment.is_peaceful()
        && (reason.ignores_light_requirements()
            || check_dark_enough_to_spawn(environment, pos, random))
        && (reason.is_spawner() || environment.fluid_is_water(pos));
    if !can_spawn || (!reason.is_spawner() && reason != SpawnPredicateReason::Reinforcement) {
        if environment.biome_matches_tag(pos, "minecraft:more_frequent_drowned_spawns") {
            random.next_int(15) == 0 && can_spawn
        } else {
            random.next_int(40) == 0 && pos.y < environment.sea_level() - 5 && can_spawn
        }
    } else {
        true
    }
}

fn check_dark_enough_to_spawn(
    environment: &SpawnPredicateLevel<'_>,
    pos: BlockPos,
    random: &GameRandom,
) -> bool {
    if environment.sky_light(pos) as i32 > random.next_int(32) {
        return false;
    }
    let Some(block_limit) = environment.monster_spawn_block_light_limit() else {
        return false;
    };
    if block_limit < 15 && environment.block_light(pos) > block_limit {
        return false;
    }
    let darkening = if environment.is_thundering() {
        10
    } else {
        environment.sky_darken()
    };
    environment
        .sample_monster_spawn_light_test(random)
        .is_some_and(|limit| raw_brightness(environment, pos, darkening) <= limit)
}

// `Bat.java:222-234`.
fn check_bat_spawn_rules(
    environment: &SpawnPredicateLevel<'_>,
    entity_type_id: i32,
    pos: BlockPos,
    reason: SpawnPredicateReason,
    random: &GameRandom,
) -> bool {
    pos.y < environment.world_surface_height(pos)
        && !random.next_boolean()
        && raw_brightness(environment, pos, environment.sky_darken()) as i32 <= random.next_int(4)
        && environment.block_matches_tag(below(pos), "minecraft:bats_spawnable_on")
        && check_mob_spawn_rules(environment, entity_type_id, pos, reason)
}

// `Slime.java:271-298`.
fn check_slime_spawn_rules(
    environment: &SpawnPredicateLevel<'_>,
    entity_type_id: i32,
    pos: BlockPos,
    reason: SpawnPredicateReason,
    random: &GameRandom,
) -> bool {
    if environment.is_peaceful() {
        return false;
    }
    if reason.is_spawner() {
        return check_mob_spawn_rules(environment, entity_type_id, pos, reason);
    }
    if environment.biome_matches_tag(pos, "minecraft:allows_surface_slime_spawns")
        && pos.y > 50
        && pos.y < 70
        && environment
            .surface_slime_spawn_chance(pos)
            .is_some_and(|chance| random.next_float() < chance)
        && raw_brightness(environment, pos, environment.sky_darken()) as i32 <= random.next_int(8)
        && check_mob_spawn_rules(environment, entity_type_id, pos, reason)
    {
        return true;
    }
    random.next_int(10) == 0
        && environment.is_slime_chunk(pos.x.div_euclid(16), pos.z.div_euclid(16))
        && pos.y < 40
        && check_mob_spawn_rules(environment, entity_type_id, pos, reason)
}

/// Scale applied to the entity type's base dimensions when testing natural,
/// chunk-generation, and spawner clearance (`NaturalSpawner.java`,
/// `BaseSpawner.java`, `Slime.java`).
pub(crate) const fn spawn_dimensions_scale(entity_type_id: i32) -> f32 {
    if matches!(
        entity_type_id,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SLIME_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
    ) {
        4.0_f32
    } else {
        1.0_f32
    }
}

// `Stray.java:26-37`.
fn check_stray_spawn_rules(
    environment: &SpawnPredicateLevel<'_>,
    entity_type_id: i32,
    pos: BlockPos,
    reason: SpawnPredicateReason,
    random: &GameRandom,
) -> bool {
    let mut sky_pos = pos;
    loop {
        let Some(next_y) = sky_pos.y.checked_add(1) else {
            return false;
        };
        sky_pos.y = next_y;
        if next_y >= environment.max_y_exclusive()
            || !environment.block_is(sky_pos, "minecraft:powder_snow")
        {
            break;
        }
    }
    check_monster_spawn_rules(environment, entity_type_id, pos, reason, random)
        && (reason.is_spawner() || can_see_sky(environment, below(sky_pos)))
}

// `Strider.checkStriderSpawnRules` (`Strider.java:155-169`).
fn check_strider_spawn_rules(environment: &SpawnPredicateLevel<'_>, pos: BlockPos) -> bool {
    let mut scan = pos;
    loop {
        let Some(next_y) = scan.y.checked_add(1) else {
            return false;
        };
        if next_y >= environment.max_y_exclusive() {
            return true;
        }
        scan.y = next_y;
        if !environment.fluid_is_lava(scan) {
            return environment.block_is(scan, "minecraft:air")
                || environment.block_is(scan, "minecraft:cave_air")
                || environment.block_is(scan, "minecraft:void_air");
        }
    }
}

fn can_see_sky(environment: &SpawnPredicateLevel<'_>, pos: BlockPos) -> bool {
    environment.sky_light(pos) >= 15
}

fn can_see_sky_from_below_water(environment: &SpawnPredicateLevel<'_>, pos: BlockPos) -> bool {
    if pos.y >= environment.sea_level() {
        return can_see_sky(environment, pos);
    }
    let surface = BlockPos {
        y: environment.sea_level(),
        ..pos
    };
    if !can_see_sky(environment, surface) {
        return false;
    }
    ((pos.y + 1)..environment.sea_level()).all(|y| {
        let scan = BlockPos { y, ..pos };
        environment.light_dampening(scan) == 0 || environment.has_liquid(scan)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_preserves_registered_placement_and_predicate_families() {
        for facts in crate::entity::natural_spawn_facts_26_1::VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1
        {
            assert_eq!(spawn_placement(facts.resource_id), facts.placement);
        }
        assert_eq!(
            spawn_placement("minecraft:cod"),
            SpawnPlacementEntry {
                placement: SpawnPlacementType::InWater,
                heightmap: SpawnHeightmap::MotionBlockingNoLeaves,
                predicate: SpawnPredicateKind::SurfaceWaterAnimal,
            }
        );
        assert_eq!(
            spawn_placement("minecraft:ocelot").heightmap,
            SpawnHeightmap::MotionBlocking
        );
        assert_eq!(
            spawn_placement("minecraft:strider").placement,
            SpawnPlacementType::InLava
        );
        assert_eq!(
            spawn_placement("minecraft:future_entity").predicate,
            SpawnPredicateKind::Unrestricted
        );
    }

    #[test]
    fn placement_types_apply_world_border_medium_and_ground_shape() {
        let water = SpawnPlacementEnvironment {
            fluid_at: SpawnFluid::Water,
            ..Default::default()
        };
        assert!(is_spawn_position_ok(SpawnPlacementType::InWater, water));
        assert!(!is_spawn_position_ok(
            SpawnPlacementType::InWater,
            SpawnPlacementEnvironment {
                above_is_redstone_conductor: true,
                ..water
            }
        ));
        assert!(!is_spawn_position_ok(
            SpawnPlacementType::OnGround,
            SpawnPlacementEnvironment {
                at_is_valid_empty_spawn_block: false,
                ..Default::default()
            }
        ));
        assert!(is_spawn_position_ok(
            SpawnPlacementType::NoRestrictions,
            SpawnPlacementEnvironment {
                inside_world_border: false,
                ..Default::default()
            }
        ));
    }
}
