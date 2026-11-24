use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::entity::{EntityData, GeneralLivingData};
use crate::world::{TickResult, WorldCommand};
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, MoveVehicleCommand, PlayServerbound, Vec3d};
use bbb_server_types::save::Java26BuiltinDimension;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const PLAYER: ConnectionId = ConnectionId(1);

fn packet(world: &mut ServerWorld, packet: PlayServerbound) -> TickResult {
    world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
        world.last_command_sequence + 1,
        crate::mailbox::WorldProducerClass::Network,
        WorldCommand::Play {
            connection_id: PLAYER,
            packet,
        },
    )])
}

fn mount(world: &mut ServerWorld, vehicle: i32) {
    let mut events = Vec::new();
    world.start_external_player_ride(PLAYER, DIM, vehicle, 0.0, 0.0, &mut events);
    let teleport = events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .expect("mounting places the rider");
    packet(world, PlayServerbound::AcceptTeleportation { id: teleport });
}

fn drive(world: &mut ServerWorld, vehicle: i32, distance: f64) {
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(PLAYER);
    let target = Vec3d {
        x: baseline.x + distance,
        ..baseline
    };
    packet(
        world,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: target,
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        }),
    );
    assert_eq!(
        world.level(DIM).entities.get(vehicle).unwrap().position().x,
        target.x,
        "the real vehicle movement was accepted"
    );
}

fn pool(world: &mut ServerWorld, lava: bool) {
    let state = |name| {
        vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id)
            .unwrap()
    };
    for x in -1..=15 {
        for z in -1..=1 {
            world.set_raw_block_state_for_test(
                DIM,
                BlockPos { x, y: 63, z },
                state("minecraft:stone"),
            );
            world.set_raw_block_state_for_test(
                DIM,
                BlockPos { x, y: 64, z },
                state(if lava {
                    "minecraft:lava"
                } else {
                    "minecraft:air"
                }),
            );
        }
    }
}

#[test]
fn lava_ride_uses_vehicle_fluid_and_restarts_after_exiting_or_dismounting() {
    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    let conditions = json!({
        "player": {"location": {"dimension": "minecraft:overworld"}, "vehicle": {"type": "minecraft:strider"}},
        "start_position": {"position": {"x": {"min": 0, "max": 15}}},
        "distance": {"horizontal": {"min": 2}}
    });
    let mut definitions = vec![("test:ride", conditions.clone())];
    let mut wrong_start = conditions.clone();
    wrong_start["start_position"]["position"]["x"] = json!({"min": 100});
    definitions.push(("test:wrong_start", wrong_start));
    let mut wrong_dimension = conditions.clone();
    wrong_dimension["player"]["location"]["dimension"] = json!("minecraft:the_nether");
    definitions.push(("test:wrong_dimension", wrong_dimension));
    let mut wrong_vehicle = conditions;
    wrong_vehicle["player"]["vehicle"]["type"] = json!("minecraft:pig");
    definitions.push(("test:wrong_vehicle", wrong_vehicle));
    startup.advancements = AdvancementSet {
        advancements: definitions.into_iter().map(|(id, conditions)| AdvancementElement {
            id: id.into(), source_pack: "test".into(), advancement: parse_advancement(&json!({
                "criteria": {"ride": {"trigger": "minecraft:ride_entity_in_lava", "conditions": conditions}},
                "rewards": {"experience": 7}
            })).unwrap(),
        }).collect(), warnings: Vec::new(),
    };
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "rider", 1);
    pool(&mut world, false);
    let vehicle = world
        .spawn_stored_entity_for_test(
            DIM,
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            Vec3d {
                x: 0.5,
                y: 64.5,
                z: 0.5,
            },
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_STRIDER_ID).unwrap(),
            ),
        )
        .0;
    let saddle = crate::entity::item::plain_stack("minecraft:saddle", 1).unwrap();
    world
        .level_mut(DIM)
        .entities
        .get_mut(vehicle)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::Saddle, Some(saddle));
    world.set_inventory_slot_for_test(
        PLAYER,
        0,
        crate::entity::item::plain_stack("minecraft:warped_fungus_on_a_stick", 1)
            .unwrap()
            .into_summary(),
    );
    mount(&mut world, vehicle);
    drive(&mut world, vehicle, 2.25);
    assert!(!world.player_advancement_is_done(PLAYER, "test:ride"));

    pool(&mut world, true);
    world.tick(Vec::new());
    assert!(world.level(DIM).entities.get(vehicle).unwrap().is_in_lava());
    assert!(
        !world.levels[PLAYER]
            .fluid_interaction
            .is_in_fluid(crate::entity::FluidTag::Lava),
        "the rider is above the lava, while its vehicle touches it"
    );
    drive(&mut world, vehicle, 1.25);
    assert!(!world.player_advancement_is_done(PLAYER, "test:ride"));
    pool(&mut world, false);
    world.tick(Vec::new());
    pool(&mut world, true);
    world.tick(Vec::new());
    drive(&mut world, vehicle, 1.25);
    assert!(
        !world.player_advancement_is_done(PLAYER, "test:ride"),
        "leaving lava resets the distance"
    );

    let player_id = world.levels[PLAYER].entity_id();
    world
        .level_mut(DIM)
        .entities
        .detach_external_player(player_id);
    world.tick(Vec::new());
    mount(&mut world, vehicle);
    drive(&mut world, vehicle, 1.25);
    assert!(
        !world.player_advancement_is_done(PLAYER, "test:ride"),
        "dismounting resets the distance"
    );
    drive(&mut world, vehicle, 1.25);
    assert!(world.player_advancement_is_done(PLAYER, "test:ride"));
    assert_eq!(world.levels[PLAYER].experience.total(), 7);
    drive(&mut world, vehicle, 0.25);
    assert_eq!(
        world.levels[PLAYER].experience.total(),
        7,
        "the reward is granted once"
    );
    for id in [
        "test:wrong_start",
        "test:wrong_dimension",
        "test:wrong_vehicle",
    ] {
        assert!(!world.player_advancement_is_done(PLAYER, id), "{id}");
    }
}
