use super::*;
use crate::entity::{EntityData, GeneralLivingData};
use crate::explosion::{ExplosionDamageProfile, ExplosionPresentation, PendingExplosion};
use crate::mailbox::{WorldCommandReceiver, WorldCommandSender};
use crate::tests::{chat_command, join_loaded, mailbox, operator_config};
use crate::world::{ServerWorldConfig, TickResult};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
    VANILLA_ENTITY_TYPE_WITHER_ID, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
    VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::Java26BuiltinDimension;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const ALICE: ConnectionId = ConnectionId(1);

fn world_with_alice() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        datapacks: crate::tests::datapack_config().datapacks,
        ..operator_config(&[("alice", 1)])
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick(receiver.drain());
    (sender, receiver, world)
}

fn alice_position(world: &ServerWorld) -> Vec3d {
    world.publication().players[0].position
}

fn spawn_living(world: &mut ServerWorld, type_id: i32, position: Vec3d) -> (i32, Uuid) {
    world.spawn_stored_entity_for_test(
        DIM,
        type_id,
        position,
        EntityData::GeneralLiving(GeneralLivingData::new(type_id).unwrap()),
    )
}

fn health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIM, id)
        .and_then(|entity| entity.living())
        .map_or(0.0, |living| living.health())
}

fn merge_record(world: &mut ServerWorld, id: i32, merged: Vec<NbtSummaryEntry>) {
    let NbtSummaryValue::Compound(mut entries) =
        world.stored_entity_command_data_for_test(DIM, id).unwrap()
    else {
        panic!("an entity record is a compound");
    };
    entries.retain(|entry| !merged.iter().any(|merge| merge.name == entry.name));
    entries.extend(merged);
    world.load_stored_entity_record_for_test(DIM, id, &entries);
}

fn no_ai(world: &mut ServerWorld, id: i32) {
    merge_record(
        world,
        id,
        vec![NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(1),
        }],
    );
}

fn uuid_ints(uuid: Uuid) -> NbtSummaryValue {
    let bits = uuid.as_u128();
    NbtSummaryValue::IntArray(
        (0..4)
            .map(|word| (bits >> (96 - 32 * word)) as u32 as i32)
            .collect(),
    )
}

fn explosion(center: Vec3d, radius: f32, profile: ExplosionDamageProfile) -> PendingExplosion {
    PendingExplosion {
        source: None,
        center,
        radius,
        fire: false,
        interaction: ExplosionInteraction::Mob,
        damage_source: None,
        damage_profile: profile,
        presentation: ExplosionPresentation::Default,
    }
}

fn block(name: &str) -> i32 {
    vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

/// `Level.getEntities` yields the dragon's parts: a blast at its head deals the
/// head's full damage, not the body's quartered share.
#[test]
fn explosion_at_the_dragon_head_deals_full_head_damage() {
    let (_sender, receiver, mut world) = world_with_alice();
    let at = alice_position(&world);
    let (dragon, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        Vec3d {
            y: at.y + 30.0,
            ..at
        },
        EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID).unwrap(),
        ),
    );
    world.tick(receiver.drain());
    let head = crate::entity::ender_dragon::projectile_parts(
        world.stored_entity_for_test(DIM, dragon).unwrap(),
    )
    .next()
    .map(|(_, bounds, _)| bounds.center())
    .expect("an admitted dragon has parts");
    let before = health(&world, dragon);
    world.explode(
        DIM,
        explosion(head, 4.0, ExplosionDamageProfile::Default),
        &mut Vec::new(),
    );
    // A body-only hit is capped at `57 / 4 + 1`.
    assert!(before - health(&world, dragon) > 20.0);
}

/// `LivingEntity.onBelowWorld` (`LivingEntity.java:2141-2143`).
#[test]
fn a_mob_below_the_world_takes_out_of_world_damage() {
    let (_sender, receiver, mut world) = world_with_alice();
    let at = alice_position(&world);
    let (cow, _) = spawn_living(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: at.x + 2.0,
            y: -64.0 - 70.0,
            z: at.z,
        },
    );
    world.tick(receiver.drain());
    assert_eq!(health(&world, cow), 6.0);
    assert!(world.stored_entity_for_test(DIM, cow).is_some());
}

/// `WitherSkull.getBlockExplosionResistance` (`WitherSkull.java:51-55`).
#[test]
fn a_dangerous_wither_skull_breaks_obsidian() {
    for (profile, broken) in [
        (ExplosionDamageProfile::Default, false),
        (ExplosionDamageProfile::DangerousWitherSkull, true),
    ] {
        let (_sender, _receiver, mut world) = world_with_alice();
        let at = alice_position(&world);
        let pos = BlockPos {
            x: at.x.floor() as i32 + 3,
            y: at.y.floor() as i32 + 3,
            z: at.z.floor() as i32,
        };
        assert!(world.set_block(DIM, pos, block("minecraft:obsidian"), SET_BLOCK_UPDATE_ALL));
        let center = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        };
        world.explode(DIM, explosion(center, 1.0, profile), &mut Vec::new());
        assert_eq!(
            world.block_state(DIM, pos) == Some(block("minecraft:air")),
            broken
        );
    }
}

/// `WitherSkull.onHitEntity` (`WitherSkull.java:66-71`): a kill heals the owner 5.
#[test]
fn a_wither_skull_kill_heals_its_wither() {
    let (_sender, receiver, mut world) = world_with_alice();
    let at = alice_position(&world);
    let (wither, wither_uuid) = spawn_living(
        &mut world,
        VANILLA_ENTITY_TYPE_WITHER_ID,
        Vec3d {
            x: at.x + 12.0,
            ..at
        },
    );
    no_ai(&mut world, wither);
    crate::entity::living::set_health(
        world.stored_entity_mut_for_test(DIM, wither).unwrap(),
        100.0,
    );
    let cow_at = Vec3d {
        x: at.x + 3.0,
        ..at
    };
    let (cow, _) = spawn_living(&mut world, VANILLA_ENTITY_TYPE_COW_ID, cow_at);
    no_ai(&mut world, cow);
    crate::entity::living::set_health(world.stored_entity_mut_for_test(DIM, cow).unwrap(), 1.0);
    let (skull, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        Vec3d {
            x: cow_at.x + 1.0,
            y: cow_at.y + 0.5,
            ..cow_at
        },
        EntityData::WitherSkull(
            crate::entity::hurting_projectile::wither_skull::WitherSkullData::new(
                crate::entity::HurtingProjectileState::new(Some(wither_uuid), 0.0),
                false,
            ),
        ),
    );
    world
        .stored_entity_mut_for_test(DIM, skull)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: -0.8,
            y: 0.0,
            z: 0.0,
        });
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    assert_eq!(health(&world, cow), 0.0);
    assert_eq!(health(&world, wither), 105.0);
}

/// `EvokerFangs.dealDamageTo` (`EvokerFangs.java:110-112`): the owner's allies
/// are spared.
#[test]
fn evoker_fangs_spare_the_owners_tamed_wolf() {
    let (_sender, receiver, mut world) = world_with_alice();
    let at = Vec3d {
        x: alice_position(&world).x + 4.0,
        ..alice_position(&world)
    };
    let (wolf, _) = spawn_living(&mut world, VANILLA_ENTITY_TYPE_WOLF_ID, at);
    merge_record(
        &mut world,
        wolf,
        vec![NbtSummaryEntry {
            name: "Owner".into(),
            value: uuid_ints(Uuid::from_u128(1)),
        }],
    );
    no_ai(&mut world, wolf);
    assert!(matches!(
        world
            .stored_entity_for_test(DIM, wolf)
            .unwrap()
            .tamable_owner(),
        Some((true, Some(_)))
    ));
    let (cow, _) = spawn_living(&mut world, VANILLA_ENTITY_TYPE_COW_ID, at);
    no_ai(&mut world, cow);
    let (wolf_before, cow_before) = (health(&world, wolf), health(&world, cow));
    world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
        at,
        EntityData::EvokerFangs(crate::entity::EvokerFangsData::new(
            0,
            Some(Uuid::from_u128(1)),
        )),
    );
    for _ in 0..8 {
        world.tick(receiver.drain());
    }
    assert_eq!(health(&world, wolf), wolf_before);
    assert_eq!(health(&world, cow), cow_before - 6.0);
}

/// `LivingEntity.resolveMobResponsibleForDamage` (`LivingEntity.java:1329-1335`).
#[test]
fn no_anger_damage_does_not_record_the_mob_that_hurt_a_player() {
    let (sender, receiver, mut world) = world_with_alice();
    let at = alice_position(&world);
    let (_, cow) = spawn_living(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: at.x + 2.0,
            ..at
        },
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage alice 1 minecraft:mob_attack_no_aggro by {cow}"),
    );
    assert_eq!(world.levels.player_at(ALICE).cold.last_hurt_by_mob, None);
    for _ in 0..20 {
        world.tick(receiver.drain());
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage alice 1 minecraft:mob_attack by {cow}"),
    );
    assert_eq!(
        world.levels.player_at(ALICE).cold.last_hurt_by_mob,
        Some(cow)
    );
}

/// Explosion knockback goes through `Entity.push`, which a stuck arrow
/// (`AbstractArrow.java:412-416`) and a wind charge (`AbstractWindCharge.java:96`)
/// refuse.
#[test]
fn explosion_knockback_skips_stuck_arrows_and_wind_charges() {
    let (_sender, _receiver, mut world) = world_with_alice();
    let at = alice_position(&world);
    let center = Vec3d {
        y: at.y + 6.0,
        ..at
    };
    let arrow = |stuck: bool| {
        let mut data = crate::entity::ArrowData::new(
            crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
        );
        data.set_in_ground(stuck);
        EntityData::Arrow(data)
    };
    let beside = Vec3d {
        x: center.x + 1.5,
        ..center
    };
    let (flying, _) =
        world.spawn_stored_entity_for_test(DIM, VANILLA_ENTITY_TYPE_ARROW_ID, beside, arrow(false));
    let (stuck, _) =
        world.spawn_stored_entity_for_test(DIM, VANILLA_ENTITY_TYPE_ARROW_ID, beside, arrow(true));
    let (wind, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        beside,
        EntityData::WindCharge(crate::entity::WindChargeData::new(
            crate::entity::HurtingProjectileState::new(None, 0.0),
            5,
        )),
    );
    world.explode(
        DIM,
        explosion(center, 3.0, ExplosionDamageProfile::Default),
        &mut Vec::new(),
    );
    let motion = |id| {
        world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .delta_movement()
    };
    assert!(motion(flying).x > 0.0);
    assert_eq!(motion(stuck), Vec3d::default());
    assert_eq!(motion(wind), Vec3d::default());
}

/// `MaceItem.getItemDamageSource` / `getAttackDamageBonus` reach a mob
/// attacker through `Mob.doHurtTarget` (`Mob.java:1358-1360`).
#[test]
fn a_falling_zombie_with_a_mace_smashes() {
    let mut store =
        crate::entity::EntityStore::new(7, crate::entity::store::EntityIdCounter::new());
    let at = Vec3d {
        x: 8.5,
        y: 64.0,
        z: 8.5,
    };
    let zombie = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, at)
        .unwrap();
    let cow = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: at.x + 1.0,
                ..at
            },
        )
        .unwrap();
    let mut body = store.detach(zombie).unwrap();
    body.living_mut().unwrap().set_item_slot(
        bbb_protocol::equipment_slots::EquipmentSlot::MainHand,
        Some(crate::entity::item::plain_stack("minecraft:mace", 1).unwrap()),
    );
    body.set_fall_distance(5.0);
    let mut native = crate::entity::tests::native_level::NativeLevel::new();
    native.with_level(|level| {
        crate::entity::ai::mob::do_hurt_target(&mut body, &mut store, level, cow)
    });
    let cow = store.get(cow).unwrap().living().unwrap();
    assert_eq!(
        cow.last_damage_type.as_deref(),
        Some("minecraft:mace_smash")
    );
    // 3 attack damage + the `12 + 2 * (5 - 3)` smash bonus.
    assert!(cow.health() <= 0.0);
    assert_eq!(body.delta_movement().y, f64::from(0.01_f32));
}

/// `LivingEntity.java:1222-1224`: a zero-damage hit still marks the body hurt, so
/// its motion is synced at once.
#[test]
fn a_zero_damage_hit_syncs_the_mob_motion() {
    let (sender, receiver, mut world) = world_with_alice();
    let at = alice_position(&world);
    let (cow, cow_uuid) = spawn_living(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: at.x + 2.0,
            ..at
        },
    );
    no_ai(&mut world, cow);
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    let tick: TickResult = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {cow_uuid} 0 minecraft:generic"),
    );
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetEntityMotion(motion),
            ..
        } if motion.id == cow
    )));
}

/// `SmallFireball.onHitBlock` places `BaseFireBlock.getState` (`SmallFireball.java:55-57`).
#[test]
fn a_small_fireball_lights_soul_fire_on_soul_sand() {
    let (_sender, receiver, mut world) = world_with_alice();
    let at = alice_position(&world);
    let base = BlockPos {
        x: at.x.floor() as i32 + 3,
        y: at.y.floor() as i32,
        z: at.z.floor() as i32,
    };
    assert!(world.set_block(
        DIM,
        base,
        block("minecraft:soul_sand"),
        SET_BLOCK_UPDATE_ALL
    ));
    let pos = BlockPos {
        y: base.y + 1,
        ..base
    };
    world.queue_projectile_action_for_test(
        DIM,
        crate::entity::projectile_action::PendingProjectileAction::PlaceFire { pos },
    );
    world.tick(receiver.drain());
    assert_eq!(
        world.block_state(DIM, pos),
        Some(block("minecraft:soul_fire"))
    );
}
