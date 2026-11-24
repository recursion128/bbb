//! `ServerLevel.tickThunder` (`ServerLevel.java:527-561`) in a running world:
//! the storm the weather cycle brings strikes a rained-on column and the bolt
//! reaches the session.

use super::*;

use crate::random::GameRandom;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID;

/// A seed whose first `nextInt(100000)` is 0.
fn winning_thunder_roll_seed() -> i64 {
    (0..)
        .find(|&seed| GameRandom::with_seed(seed).next_int(100_000) == 0)
        .expect("a winning seed")
}

/// The bolts `LightningBolt`'s spawn published to the session.
fn bolt_spawns(events: &[WorldEvent]) -> Vec<&bbb_protocol::packets::AddEntity> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::AddEntity(add),
                ..
            } if add.entity_type_id == VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID => Some(add),
            _ => None,
        })
        .collect()
}

/// The whole path, end to end: a generated overworld with a player in it, taken
/// from clear sky to `Level.isThundering` (`Level.java:937-939`) by the weather
/// cycle, strikes a rained-on column and publishes the bolt to the session,
/// and strikes nothing before the storm reaches `isThundering`.
#[test]
fn a_thundering_world_tick_strikes_a_rained_on_column() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..ServerWorldConfig::default()
    })
    .unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    // A bolt is only paired with a session that was sent its chunk.
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);

    let command = "weather thunder 40000t";
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, command, AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: command.to_string(),
            },
        },
    )
    .unwrap();
    // `advanceWeatherCycle` interpolates both levels a hundredth at a time
    // (`ServerLevel.java:740-755`), so the storm takes ~95 ticks to cross the
    // `0.9` `isThundering` reads. Nothing strikes while it climbs.
    let mut climbing = 0;
    while !world
        .level_weather(Java26BuiltinDimension::Overworld)
        .is_thundering()
    {
        let tick = world.tick(receiver.drain());
        assert!(
            bolt_spawns(&tick.events).is_empty(),
            "struck while climbing"
        );
        climbing += 1;
        assert!(climbing < 200, "the storm never reached isThundering");
    }

    // The `1/100000` roll (`ServerLevel.java:534`) is staged to win on the
    // next draw of the player's own chunk rather than waited out.
    let player = world.publication().players[0].position;
    let chunk = (
        (player.x.floor() as i32).div_euclid(16),
        (player.z.floor() as i32).div_euclid(16),
    );
    world.random.reseed(winning_thunder_roll_seed());
    let mut events = world.tick_thunder_for_test(Java26BuiltinDimension::Overworld, chunk);
    events.extend(world.tick(receiver.drain()).events);
    let bolts = bolt_spawns(&events);
    assert!(!bolts.is_empty(), "the storm never struck");
    let bolt = bolts[0];
    // `Vec3.atBottomCenterOf` (`ServerLevel.java:553`) puts the bolt on the
    // floor of the block `findLightningTargetAround` picked.
    let pos = BlockPos {
        x: bolt.position.x.floor() as i32,
        y: bolt.position.y.floor() as i32,
        z: bolt.position.z.floor() as i32,
    };
    assert_eq!(bolt.position.x, f64::from(pos.x) + 0.5);
    assert_eq!(bolt.position.z, f64::from(pos.z) + 0.5);
    assert_eq!(bolt.position.y, f64::from(pos.y));
    // `isRainingAt` (`ServerLevel.java:536`) is the last gate the strike passed,
    // so the column it landed in is one the world agrees is being rained on.
    assert!(
        world.is_raining_at(Java26BuiltinDimension::Overworld, pos),
        "{pos:?}"
    );
}

mod strike {
    use super::super::using_item::{Fixture, ALICE, BOB};
    use super::*;
    use crate::entity::item::{plain_stack, ItemEntityData};
    use crate::entity::{Entity, EntityData};
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
        VANILLA_ENTITY_TYPE_WITCH_ID, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
    };
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    use bbb_server_types::block_queries::{bool_property, int_property};

    const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

    /// Alice and Bob stand well away from where the bolts fall.
    fn bystanders() -> Fixture {
        bystanders_with_config(datapack_config())
    }

    fn bystanders_with_config(config: ServerWorldConfig) -> Fixture {
        let mut fixture = Fixture::with_config(GameType::Survival, config);
        for (connection_id, x, z) in [(ALICE, 0.5, 0.5), (BOB, 8.5, -8.5)] {
            fixture
                .world
                .set_player_position_for_test(ConnectionId(connection_id), Vec3d { x, y: 64.0, z });
        }
        fixture
    }

    fn living(fixture: &Fixture, entity_type_id: i32) -> Vec<Entity> {
        fixture
            .world
            .stored_entity_ids_for_test(OVERWORLD)
            .into_iter()
            .filter_map(|id| fixture.world.stored_entity_for_test(OVERWORLD, id))
            .filter(|entity| {
                entity.entity_type_id() == entity_type_id && entity.is_alive_with_health()
            })
            .cloned()
            .collect()
    }

    fn record(fixture: &Fixture, entity_id: i32) -> Vec<NbtSummaryEntry> {
        let Some(NbtSummaryValue::Compound(entries)) = fixture
            .world
            .stored_entity_command_data_for_test(OVERWORLD, entity_id)
        else {
            panic!("entity {entity_id} has a record")
        };
        entries
    }

    fn field(entries: &[NbtSummaryEntry], name: &str) -> Option<NbtSummaryValue> {
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.value.clone())
    }

    fn block(fixture: &mut Fixture, x: i32, y: i32, z: i32) -> i32 {
        fixture
            .world
            .block_state(OVERWORLD, BlockPos { x, y, z })
            .expect("a loaded block")
    }

    /// `LightningBolt.tick`, `spawnFire` and the `thunderHit` overrides: on
    /// normal difficulty the strike lights its floor, charges a creeper,
    /// turns a pig into a zombified piglin and a villager into a witch,
    /// recolours a mooshroom once, kills a turtle and burns an item.
    #[test]
    fn a_strike_burns_its_floor_and_transforms_what_it_hits() {
        let mut fixture = bystanders();
        fixture.command("difficulty normal");
        for mob in ["creeper", "pig", "villager", "mooshroom", "turtle"] {
            fixture.command(&format!("summon minecraft:{mob} -10.5 64 10.5"));
        }
        let (item, _) = fixture.world.spawn_stored_entity_for_test(
            OVERWORLD,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            Vec3d {
                x: -10.5,
                y: 64.0,
                z: 11.5,
            },
            EntityData::Item(ItemEntityData::new(
                plain_stack("minecraft:stick", 1).unwrap(),
                0.0,
            )),
        );
        let creeper = living(
            &fixture,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREEPER_ID,
        )[0]
        .id();
        let mooshroom = living(
            &fixture,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
        )[0]
        .id();
        let turtle = living(
            &fixture,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID,
        )[0]
        .id();
        let colour = field(&record(&fixture, mooshroom), "Type");

        fixture.command("summon minecraft:lightning_bolt -10.5 64 10.5");
        for _ in 0..40 {
            fixture.idle();
        }

        assert!(crate::block_behavior::fire::is_fire(block(
            &mut fixture,
            -11,
            64,
            10
        )));
        assert_eq!(
            field(&record(&fixture, creeper), "powered"),
            Some(NbtSummaryValue::Byte(1))
        );
        assert!(living(&fixture, VANILLA_ENTITY_TYPE_PIG_ID).is_empty());
        let piglins = living(&fixture, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID);
        assert_eq!(piglins.len(), 1);
        let weapon = piglins[0]
            .living()
            .and_then(|living| {
                living.item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
            })
            .and_then(|stack| stack.item_id());
        assert!(
            weapon == Some(item_id("minecraft:golden_sword"))
                || weapon == Some(item_id("minecraft:golden_spear"))
        );
        assert!(living(&fixture, VANILLA_ENTITY_TYPE_VILLAGER_ID).is_empty());
        assert_eq!(living(&fixture, VANILLA_ENTITY_TYPE_WITCH_ID).len(), 1);
        let recoloured = field(&record(&fixture, mooshroom), "Type");
        assert_ne!(recoloured, colour);
        assert!(fixture
            .world
            .stored_entity_for_test(OVERWORLD, turtle)
            .is_none_or(|turtle| !turtle.is_alive_with_health()));
        assert!(fixture
            .world
            .stored_entity_for_test(OVERWORLD, item)
            .is_none_or(|item| !item.is_alive()));
    }

    /// `LightningRodBlock.onLightningStrike`: the rod a bolt lands on powers
    /// the dust beside it, and a bolt that set nothing alight with a villager
    /// looking on is the Surge Protector advancement.
    #[test]
    fn a_struck_rod_powers_dust_and_a_spared_villager_is_surge_protector() {
        let mut config = datapack_config();
        let mut startup = (*config.datapacks.take().unwrap()).clone();
        startup
            .advancements
            .advancements
            .push(bbb_server_datapack::advancement::AdvancementElement {
            id: "test:storm_bystander".into(),
            source_pack: "test".into(),
            advancement: crate::tests::parse_advancement(
                &serde_json::json!({"criteria":{"strike":{
                    "trigger":"minecraft:lightning_strike", "conditions":{"bystander":[{
                        "condition":"minecraft:entity_properties", "entity":"this", "predicate":{
                            "type":"minecraft:villager", "team":"witnesses",
                            "nbt":"{Tags:[\"witness\"],data:{marked:19b}}",
                            "components":{"minecraft:custom_data":{"marked":19}}
                        }
                    }]}
                }}}),
            )
            .unwrap(),
        });
        config.datapacks = Some(std::sync::Arc::new(startup));
        let mut fixture = bystanders_with_config(config);
        fixture.command("difficulty easy");
        fixture.command("setblock 12 64 12 minecraft:lightning_rod");
        fixture.command("setblock 13 64 12 minecraft:redstone_wire");
        fixture.command(
            "summon minecraft:villager 12.5 64 18.5 {NoAI:1b,Tags:[\"witness\"],data:{marked:19b}}",
        );
        let villager = living(&fixture, VANILLA_ENTITY_TYPE_VILLAGER_ID)[0].uuid();
        fixture.command("team add witnesses");
        fixture.command(&format!("team join witnesses {villager}"));
        fixture.command("summon minecraft:lightning_bolt 12.5 65 12.5");

        let mut powered = false;
        for _ in 0..3 {
            fixture.idle();
            let rod = block(&mut fixture, 12, 64, 12);
            let dust = block(&mut fixture, 13, 64, 12);
            powered |= bool_property(rod, "powered") == Some(true)
                && int_property(dust, "power") == Some(15);
        }
        assert!(powered);
        for _ in 0..40 {
            fixture.idle();
        }
        assert_eq!(
            bool_property(block(&mut fixture, 12, 64, 12), "powered"),
            Some(false)
        );
        assert!(fixture.world.player_advancement_is_done(
            ConnectionId(ALICE),
            "minecraft:adventure/lightning_rod_with_villager_no_fire"
        ));
        assert!(fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), "test:storm_bystander"));
    }

    /// `LightningBolt.getHitEntities` (`LightningBolt.java:259-261`) keeps
    /// struck players, so `entity_struck` (`LightningBoltPredicate.java:31-36`)
    /// sees them.
    #[test]
    fn a_struck_player_is_in_the_bolts_entity_struck() {
        let mut config = datapack_config();
        let mut startup = (*config.datapacks.take().unwrap()).clone();
        startup
            .advancements
            .advancements
            .push(bbb_server_datapack::advancement::AdvancementElement {
            id: "test:struck_player".into(),
            source_pack: "test".into(),
            advancement: crate::tests::parse_advancement(
                &serde_json::json!({"criteria":{"strike":{
                    "trigger":"minecraft:lightning_strike", "conditions":{"lightning":[{
                        "condition":"minecraft:entity_properties", "entity":"this", "predicate":{
                            "type_specific":{"type":"minecraft:lightning",
                                "entity_struck":{"type":"minecraft:player"}}
                        }
                    }]}
                }}}),
            )
            .unwrap(),
        });
        config.datapacks = Some(std::sync::Arc::new(startup));
        let mut fixture = bystanders_with_config(config);
        fixture.command("difficulty easy");
        fixture.command("summon minecraft:lightning_bolt 8.5 64 -8.5");
        for _ in 0..40 {
            fixture.idle();
        }
        assert!(fixture
            .world
            .player_advancement_is_done(ConnectionId(ALICE), "test:struck_player"));
    }

    fn present(fixture: &Fixture, entity_type_id: i32) -> Vec<Entity> {
        fixture
            .world
            .stored_entity_ids_for_test(OVERWORLD)
            .into_iter()
            .filter_map(|id| fixture.world.stored_entity_for_test(OVERWORLD, id))
            .filter(|entity| entity.entity_type_id() == entity_type_id && entity.is_alive())
            .cloned()
            .collect()
    }

    /// `VehicleEntity.hurtServer` and `EndCrystal.hurtServer` under
    /// `Entity.thunderHit`: a bolt breaks a boat and a minecart into their
    /// items and sets off an end crystal.
    #[test]
    fn a_strike_wrecks_vehicles_and_detonates_end_crystals() {
        use bbb_protocol::entity_types::{
            VANILLA_ENTITY_TYPE_END_CRYSTAL_ID, VANILLA_ENTITY_TYPE_MINECART_ID,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        };
        let mut fixture = bystanders();
        fixture.command("difficulty peaceful");
        for (entity_type_id, x, z, data) in [
            (
                VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
                -10.5,
                10.5,
                EntityData::Boat(crate::entity::BoatData::new()),
            ),
            (
                VANILLA_ENTITY_TYPE_MINECART_ID,
                -11.5,
                10.5,
                EntityData::Minecart(crate::entity::MinecartData::new()),
            ),
            (
                VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
                -10.5,
                26.5,
                EntityData::EndCrystal(crate::entity::EndCrystalData::new(0, false)),
            ),
        ] {
            fixture.world.spawn_stored_entity_for_test(
                OVERWORLD,
                entity_type_id,
                Vec3d { x, y: 64.0, z },
                data,
            );
            assert_eq!(present(&fixture, entity_type_id).len(), 1);
        }

        let mut ticks = vec![
            fixture.command("summon minecraft:lightning_bolt -11 64 10.5"),
            fixture.command("summon minecraft:lightning_bolt -10.5 64 26.5"),
        ];
        for _ in 0..5 {
            ticks.push(fixture.idle());
        }

        assert!(present(&fixture, VANILLA_ENTITY_TYPE_OAK_BOAT_ID).is_empty());
        assert!(present(&fixture, VANILLA_ENTITY_TYPE_MINECART_ID).is_empty());
        assert!(present(&fixture, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID).is_empty());
        // Each wreck drops its item (the crystal's blast drops blocks well
        // away); it lands inside the strike box, where the
        // next strike tick burns it as it would any item.
        let dropped = ticks
            .iter()
            .flat_map(|tick| &tick.events)
            .filter(|event| {
                matches!(
                    event.packet_for(ConnectionId(ALICE)),
                    Some(ClientboundPlayPacket::AddEntity(add))
                        if add.entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID
                            && (add.position.z - 10.5).abs() < 2.0
                )
            })
            .count();
        assert_eq!(dropped, 2);
    }
}
