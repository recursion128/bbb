use super::*;

fn tick_until(world: &mut ServerWorld, limit: usize, done: impl Fn(&ServerWorld) -> bool) -> bool {
    for _ in 0..limit {
        world.tick(Vec::new());
        if done(world) {
            return true;
        }
    }
    false
}

#[test]
fn villagers_gather_between_waves_hide_during_combat_and_resume_rest_after_a_loss() {
    let dim = Java26BuiltinDimension::Overworld;
    let (mut world, origin) = hero_world_in_a_village();
    let (sender, receiver) = crate::tests::mailbox();
    let mut operation_id = 0;
    let mut command = |world: &mut ServerWorld, text: &str| {
        operation_id += 1;
        crate::tests::console_command(&sender, &receiver, world, operation_id, text);
    };
    command(&mut world, "gamerule spawn_mobs false");
    command(&mut world, "gamerule advance_time false");
    command(&mut world, "time set 3000");
    let meeting = BlockPos {
        x: origin.x + 1,
        ..origin
    };
    let home = BlockPos {
        x: origin.x + 16,
        ..origin
    };
    for (x, part) in [(home.x - 1, "foot"), (home.x, "head")] {
        command(
            &mut world,
            &format!(
                "setblock {x} {} {} minecraft:red_bed[part={part},facing=east]",
                home.y, home.z,
            ),
        );
    }
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID;
    let (villager, _) = world.spawn_stored_entity_for_test(
        dim,
        kind,
        Vec3d {
            x: f64::from(origin.x + 20) + 0.5,
            y: f64::from(origin.y),
            z: f64::from(origin.z) + 0.5,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(kind).unwrap(),
        ),
    );
    let row = Java26Raid {
        center: origin,
        groups_spawned: 0,
        cooldown_ticks: 10_000,
        ..raid(Java26RaidStatus::Ongoing)
    };
    world.level_mut(dim).raids = manager_rows(&[(3, row)]);
    let near = |world: &ServerWorld, pos: BlockPos, distance: f64| {
        let at = world
            .stored_entity_for_test(dim, villager)
            .unwrap()
            .position();
        let dx = at.x - (f64::from(pos.x) + 0.5);
        let dz = at.z - (f64::from(pos.z) + 0.5);
        dx * dx + dz * dz < distance * distance
    };
    assert!(
        tick_until(&mut world, 600, |world| near(world, meeting, 2.5)),
        "villagers gather at their bell between waves"
    );

    command(
        &mut world,
        "attribute @e[type=minecraft:villager,limit=1] minecraft:movement_speed base set 0",
    );
    assert!(
        tick_until(&mut world, 400, |world| world
            .get_block_entity(dim, meeting)
            .and_then(crate::block_entity::BlockEntity::bell)
            .is_some_and(|bell| bell.shaking())),
        "villagers ring the meeting bell while gathering"
    );
    command(
        &mut world,
        "attribute @e[type=minecraft:villager,limit=1] minecraft:movement_speed base set 0.5",
    );

    let raider = pillager_with_raid_id(
        &mut world,
        f64::from(origin.x + 40) + 0.5,
        Some(3),
        1,
        false,
    );
    raid_manager_mut(&mut world).add_wave_mob(3, 1, raider);
    command(
        &mut world,
        &format!("data merge entity {raider} {{NoAI:1b}}"),
    );
    let row = raid_manager_mut(&mut world)
        .saved
        .raids
        .get_mut(&3)
        .unwrap();
    row.groups_spawned = 1;
    row.cooldown_ticks = 0;
    assert!(
        tick_until(&mut world, 600, |world| near(world, home, 2.0)),
        "villagers leave the meeting point to hide at home during a wave"
    );
    assert!(world
        .stored_entity_for_test(dim, villager)
        .unwrap()
        .sleeping_pos()
        .is_none());

    raid_manager_mut(&mut world)
        .saved
        .raids
        .get_mut(&3)
        .unwrap()
        .status = Java26RaidStatus::Loss;
    command(&mut world, "time set 13000");
    assert!(
        tick_until(&mut world, 400, |world| world
            .stored_entity_for_test(dim, villager)
            .unwrap()
            .sleeping_pos()
            == Some(home)),
        "a lost raid returns villagers to their ordinary night schedule"
    );
}

#[test]
fn victorious_villagers_leave_cover_launch_burst_fireworks_and_stop_after_the_raid() {
    use crate::connection::ClientboundPlayPacket;
    use crate::entity::{EntityData, GeneralLivingData};
    use bbb_protocol::item_component_value_types::FireworkExplosionShapeSummary;
    use bbb_protocol::packets::SoundEventHolder;

    let dim = Java26BuiltinDimension::Overworld;
    let (mut world, origin) = hero_world_in_a_village();
    let (sender, receiver) = crate::tests::mailbox();
    let mut operation_id = 0;
    let mut command = |world: &mut ServerWorld, text: &str| {
        operation_id += 1;
        crate::tests::console_command(&sender, &receiver, world, operation_id, text);
    };
    command(&mut world, "gamerule spawn_mobs false");
    command(&mut world, "gamerule advance_time false");
    command(&mut world, "time set 3000");
    command(
        &mut world,
        &format!(
            "fill {} {} {} {} {} {} minecraft:stone",
            origin.x - 3,
            origin.y + 3,
            origin.z - 3,
            origin.x + 3,
            origin.y + 3,
            origin.z + 3
        ),
    );
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID;
    let mut villagers = BTreeSet::new();
    for dx in [-2, 0, 2] {
        for dz in [-2, 0, 2] {
            let (_, uuid) = world.spawn_stored_entity_for_test(
                dim,
                kind,
                Vec3d {
                    x: f64::from(origin.x + dx) + 0.5,
                    y: f64::from(origin.y),
                    z: f64::from(origin.z + dz) + 0.5,
                },
                EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
            );
            villagers.insert(uuid);
        }
    }
    command(&mut world, "execute as @e[type=minecraft:villager] run attribute @s minecraft:movement_speed base set 0");
    world.level_mut(dim).raids = manager_rows(&[(3, raid_at(Java26RaidStatus::Victory, origin))]);
    for _ in 0..100 {
        super::scene::tick(&sender, &receiver, &mut world);
    }
    assert!(
        world
            .level(dim)
            .entities
            .bodies()
            .all(|entity| !matches!(entity.data(), EntityData::FireworkRocket(_))),
        "villagers do not launch fireworks underneath a roof"
    );
    command(&mut world, "execute as @e[type=minecraft:villager] run attribute @s minecraft:movement_speed base set 0.5");
    let celebrate_sound = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.villager.celebrate",
    )
    .unwrap();
    let mut heard_celebration = false;
    let mut saw_firework = false;
    let mut left_cover = false;
    for _ in 0..500 {
        let output = super::scene::tick(&sender, &receiver, &mut world);
        heard_celebration |= output.events.iter().any(|event| {
            matches!(event, crate::world::WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(sound), ..
        } if sound.sound == SoundEventHolder::Reference { registry_id: celebrate_sound })
        });
        for entity in world.level(dim).entities.bodies() {
            if villagers.contains(&entity.uuid()) {
                let pos = entity.block_position();
                left_cover |= (pos.x - origin.x).abs() > 3 || (pos.z - origin.z).abs() > 3;
            }
            let EntityData::FireworkRocket(data) = entity.data() else {
                continue;
            };
            saw_firework = true;
            assert!(data
                .projectile()
                .owner()
                .is_some_and(|owner| villagers.contains(&owner)));
            let patch = &data.item().summary().component_patch;
            assert!(matches!(patch.fireworks_flight_duration, Some(0..=2)));
            assert_eq!(patch.fireworks_explosions.len(), 1);
            let burst = &patch.fireworks_explosions[0];
            assert_eq!(burst.shape, FireworkExplosionShapeSummary::Burst);
            assert_eq!(burst.colors.len(), 1);
            assert!(burst.fade_colors.is_empty() && !burst.has_trail && !burst.has_twinkle);
        }
    }
    assert!(left_cover, "victorious villagers walk outside");
    assert!(heard_celebration, "villagers celebrate audibly");
    assert!(
        saw_firework,
        "victorious villagers launch actual firework entities"
    );
    for _ in 0..100 {
        super::scene::tick(&sender, &receiver, &mut world);
    }
    assert!(
        world
            .level(dim)
            .entities
            .bodies()
            .all(|entity| !matches!(entity.data(), EntityData::FireworkRocket(_))),
        "celebration stops when the raid expires"
    );
}

#[test]
fn victorious_villagers_keep_cheering_under_a_roof_built_after_they_started() {
    use crate::connection::ClientboundPlayPacket;
    use crate::entity::{EntityData, GeneralLivingData};
    use bbb_protocol::packets::SoundEventHolder;

    let dim = Java26BuiltinDimension::Overworld;
    let (mut world, origin) = hero_world_in_a_village();
    let (sender, receiver) = crate::tests::mailbox();
    let mut operation_id = 0;
    let mut command = |world: &mut ServerWorld, text: &str| {
        operation_id += 1;
        crate::tests::console_command(&sender, &receiver, world, operation_id, text);
    };
    command(&mut world, "gamerule spawn_mobs false");
    command(&mut world, "gamerule advance_time false");
    command(&mut world, "time set 3000");
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID;
    for dx in [-2, 0, 2] {
        for dz in [-2, 0, 2] {
            world.spawn_stored_entity_for_test(
                dim,
                kind,
                Vec3d {
                    x: f64::from(origin.x + dx) + 0.5,
                    y: f64::from(origin.y),
                    z: f64::from(origin.z + dz) + 0.5,
                },
                EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
            );
        }
    }
    command(&mut world, "execute as @e[type=minecraft:villager] run attribute @s minecraft:movement_speed base set 0");
    world.level_mut(dim).raids = manager_rows(&[(3, raid_at(Java26RaidStatus::Victory, origin))]);
    for _ in 0..5 {
        super::scene::tick(&sender, &receiver, &mut world);
    }
    command(
        &mut world,
        &format!(
            "fill {} {} {} {} {} {} minecraft:stone",
            origin.x - 3,
            origin.y + 3,
            origin.z - 3,
            origin.x + 3,
            origin.y + 3,
            origin.z + 3
        ),
    );
    let fireworks = |world: &ServerWorld| {
        world
            .level(dim)
            .entities
            .bodies()
            .filter(|entity| matches!(entity.data(), EntityData::FireworkRocket(_)))
            .map(|entity| entity.uuid())
            .collect::<BTreeSet<_>>()
    };
    let before = fireworks(&world);
    let celebrate_sound = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.villager.celebrate",
    )
    .unwrap();
    let mut heard_celebration = false;
    for _ in 0..400 {
        let output = super::scene::tick(&sender, &receiver, &mut world);
        heard_celebration |= output.events.iter().any(|event| {
            matches!(event, crate::world::WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(sound), ..
        } if sound.sound == SoundEventHolder::Reference { registry_id: celebrate_sound })
        });
        assert!(
            fireworks(&world).is_subset(&before),
            "no fireworks are launched under the roof"
        );
    }
    assert!(
        heard_celebration,
        "the running celebration keeps cheering under the roof"
    );
}
