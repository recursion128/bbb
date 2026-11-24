use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::connection::ConnectionId;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID;
use bbb_protocol::packets::{DataComponentPatchSummary, ItemStackSummary, Vec3d};

use crate::entity::fishing_bobber::{
    retrieve, spawn_fishing_bobber, FishHookState, FishingBobberData,
};
use crate::entity::save::write_entity_chunk;
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::{
    spawn_armor_stand, tick_entities, tick_one_entity, EntityData, EntityTickOutcome,
};
use bbb_server_types::geometry::{mth_cos, mth_sin, Vec3Ops};

const FISHING_ROD_ITEM_ID: i32 = bbb_protocol::item_types::VANILLA_ITEM_FISHING_ROD_ID;

fn rod_tool() -> crate::loot_runtime::LootTool {
    crate::entity::block_drops::loot_tool_for_stack(&rod_stack(), &[])
}

fn rod_stack() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(FISHING_ROD_ITEM_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn owner_at(
    uuid: Uuid,
    position: Vec3d,
    seed: i64,
) -> (
    EntityStore,
    NativeLevel,
    crate::mailbox::WorldCommandSender,
    crate::mailbox::WorldCommandReceiver,
) {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "owner",
        uuid.as_u128(),
    );
    level
        .world
        .set_player_position_for_test(ConnectionId(1), position);
    sender
        .try_send(crate::world::WorldCommand::Admin {
            envelope: crate::admin::AdminEnvelope {
                operation_id: 1,
                request_id: "rod".into(),
                principal: "fixture".into(),
                scope: crate::admin::AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 0,
            },
            request: crate::admin::AdminRequest::ExecuteCommand {
                command: "give owner minecraft:fishing_rod".into(),
            },
        })
        .unwrap();
    level.world.tick(receiver.drain());
    let counter = EntityIdCounter::new();
    let occupied = level.world.player_entity_id_for_test(ConnectionId(1));
    while counter.next_id() <= occupied {}
    (EntityStore::new(seed, counter), level, sender, receiver)
}

fn bobber_data(store: &EntityStore, id: i32) -> FishingBobberData {
    match store.get(id).expect("entity is live").data() {
        EntityData::FishingBobber(data) => data.clone(),
        other => panic!("expected a fishing bobber, found {other:?}"),
    }
}

const OWNER: uuid::Uuid = uuid::Uuid::from_u128(0x_0b_0b_0b_0b);

#[test]
fn a_spawned_bobber_is_shot_along_the_owners_look_with_a_small_spread() {
    let owner_position = Vec3d {
        x: 10.0,
        y: 64.0,
        z: -3.0,
    };
    let (mut store, mut level, _sender, _receiver) = owner_at(OWNER, owner_position, 4242);
    let (y_rot, x_rot) = (30.0_f32, 0.0_f32);
    let id = level
        .with_level(|_| {
            spawn_fishing_bobber(
                &mut store,
                OWNER,
                owner_position,
                owner_position.y + 1.62,
                y_rot,
                x_rot,
                0,
                0,
            )
        })
        .expect("a fresh UUID");
    let entity = store.get(id).unwrap();
    assert_eq!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID
    );

    let y_cos = mth_cos(-f64::from(y_rot) * (std::f64::consts::PI / 180.0) - std::f64::consts::PI);
    let y_sin = mth_sin(-f64::from(y_rot) * (std::f64::consts::PI / 180.0) - std::f64::consts::PI);
    let x_cos = -mth_cos(-f64::from(x_rot) * (std::f64::consts::PI / 180.0));
    let x_sin = mth_sin(-f64::from(x_rot) * (std::f64::consts::PI / 180.0));
    let base = Vec3d {
        x: f64::from(-y_sin),
        y: f64::from((-(x_sin / x_cos)).clamp(-5.0, 5.0)),
        z: f64::from(-y_cos),
    };
    let dist = base.length_sqr().sqrt();
    let delta = entity.delta_movement();
    for (component, base) in [(delta.x, base.x), (delta.z, base.z)] {
        let factor = component / base;
        assert!((factor - (0.6 / dist + 0.5)).abs() <= 0.0103365, "{factor}");
    }

    assert_eq!(entity.position().y, owner_position.y + 1.62);
    assert!((entity.position().x - (owner_position.x - f64::from(y_sin) * 0.3)).abs() < 1e-9);

    let expected_yaw = delta.x.atan2(delta.z).to_degrees() as f32;
    assert!(
        (entity.y_rot() - expected_yaw).abs() < 0.05,
        "{}",
        entity.y_rot()
    );
}

#[test]
fn a_bobber_over_water_transitions_from_flying_to_bobbing() {
    let (mut store, mut level, _sender, _receiver) = owner_at(
        OWNER,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        7,
    );
    level.set(pos(0, 68, 0), state_id("minecraft:water"));
    let id = level
        .with_level(|_| {
            spawn_fishing_bobber(
                &mut store,
                OWNER,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 0.5,
                },
                68.0,
                0.0,
                0.0,
                0,
                0,
            )
        })
        .expect("a fresh UUID");
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_pos(Vec3d {
            x: 0.5,
            y: 68.5,
            z: 0.5,
        });
        entity.set_delta_movement(Vec3d {
            x: 0.0,
            y: -0.2,
            z: 0.4,
        });
        store.refile(id);
    }
    level.with_level(|level| tick_entities(&mut store, level, true));
    let data = bobber_data(&store, id);
    assert_eq!(data.current_state(), FishHookState::Bobbing);
    assert_eq!(
        store.get(id).unwrap().delta_movement(),
        Vec3d {
            x: 0.0,
            y: -0.2 * 0.2,
            z: 0.4 * 0.3,
        }
    );
}

#[test]
fn a_bobber_hooks_an_entity_in_its_flight_path() {
    let (mut store, mut level, _sender, _receiver) = owner_at(
        OWNER,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        9,
    );
    let hook = level
        .with_level(|_| {
            spawn_fishing_bobber(
                &mut store,
                OWNER,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 0.5,
                },
                68.0,
                0.0,
                0.0,
                0,
                0,
            )
        })
        .expect("a fresh UUID");
    {
        let entity = store.get_mut(hook).unwrap();
        entity.set_pos(Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        });
        entity.set_delta_movement(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        });
        store.refile(hook);
    }
    let target = spawn_armor_stand(
        &mut store,
        Vec3d {
            x: 2.5,
            y: 68.0,
            z: 0.5,
        },
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();

    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(bobber_data(&store, hook).hooked_entity_id(), Some(target));
    assert_eq!(
        bobber_data(&store, hook).current_state(),
        FishHookState::Flying
    );

    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(
        bobber_data(&store, hook).current_state(),
        FishHookState::HookedInEntity
    );
}

#[test]
fn fishing_hook_ignores_portals_and_keeps_moving() {
    let start = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };

    let (mut store, mut level, _sender, _receiver) = owner_at(OWNER, start, 10);
    level.set(
        BlockPos { x: 1, y: 68, z: 0 },
        state_id("minecraft:nether_portal"),
    );
    let id = level
        .with_level(|_| spawn_fishing_bobber(&mut store, OWNER, start, start.y, 0.0, 0.0, 0, 0))
        .unwrap();
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_pos(start);
        entity.set_delta_movement(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        });
        store.refile(id);
    }

    assert!(matches!(
        level.with_level(|level| tick_one_entity(&mut store, level, id)),
        EntityTickOutcome::Complete
    ));

    let entity = store.get(id).unwrap();
    assert!(entity.position().x > 2.0);
    assert_eq!(entity.delta_movement().x, 2.0 * 0.92);
    assert!(entity.portal_process().is_none());
}

#[test]
fn fishing_hook_block_land_carries_the_affected_state_after_contact() {
    let start = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let pos = BlockPos { x: 1, y: 68, z: 0 };
    let stone = state_id("minecraft:stone");

    let (mut store, mut level, _sender, _receiver) = owner_at(OWNER, start, 210);
    level.set(pos, stone);
    let hook = level
        .with_level(|_| spawn_fishing_bobber(&mut store, OWNER, start, start.y, 0.0, 0.0, 0, 0))
        .unwrap();
    store.get_mut(hook).unwrap().set_delta_movement(Vec3d {
        x: 2.0,
        y: 0.0,
        z: 0.0,
    });

    level.with_level(|level| tick_one_entity(&mut store, level, hook));

    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|positioned| {
            let event = &positioned.recorded;
            event.game_event == crate::level_event::VanillaGameEvent::PROJECTILE_LAND
                && event.pos == pos
                && event.source_entity_id == Some(hook)
                && event.affected_state == Some(stone)
        }));
}

#[test]
fn should_stop_fishing_discards_when_the_owner_stops_holding_a_rod() {
    let (mut store, mut level, _sender, _receiver) = owner_at(
        OWNER,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        3,
    );
    let id = level
        .with_level(|_| {
            spawn_fishing_bobber(
                &mut store,
                OWNER,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 0.5,
                },
                68.0,
                0.0,
                0.0,
                0,
                0,
            )
        })
        .expect("a fresh UUID");
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(id).is_some(), "a held rod keeps the hook alive");

    level
        .world
        .stock_menu_slot_for_test(ConnectionId(1), 36, ItemStackSummary::empty());
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(
        store.get(id).is_none(),
        "dropping the rod discards the hook"
    );
}

fn bite_scenario(seed: i64) -> (EntityStore, NativeLevel, i32) {
    let position = Vec3d {
        x: 8.5,
        y: 70.5,
        z: 8.5,
    };
    let (mut store, mut level, _sender, _receiver) = owner_at(OWNER, position, seed);
    for x in 6..=10 {
        for z in 6..=10 {
            for y in 64..=70 {
                level.set(pos(x, y, z), state_id("minecraft:water"));
            }
        }
    }
    let id = level
        .with_level(|_| spawn_fishing_bobber(&mut store, OWNER, position, 70.5, 0.0, 0.0, 0, 0))
        .expect("a fresh UUID");
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_pos(position);
        entity.set_delta_movement(Vec3d {
            x: 0.0,
            y: -0.1,
            z: 0.0,
        });
        store.refile(id);
    }
    for _ in 0..4000 {
        level.with_level(|level| tick_entities(&mut store, level, true));
        let Some(entity) = store.get(id) else {
            panic!("the hook was discarded before it could bite");
        };
        let EntityData::FishingBobber(data) = entity.data() else {
            unreachable!();
        };
        if data.nibble() > 0 && data.biting() {
            return (store, level, id);
        }
    }
    panic!("the timer chain never reached a bite");
}

#[test]
fn the_catching_fish_timer_chain_runs_to_a_bite() {
    let (store, _level, id) = bite_scenario(20260823);
    let data = bobber_data(&store, id);
    assert!(data.nibble() > 0);
    assert!(data.biting());
}

#[test]
fn a_bobber_without_its_owner_is_discarded() {
    let position = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let mut store = EntityStore::new(73, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let id = level
        .with_level(|_| spawn_fishing_bobber(&mut store, OWNER, position, 68.0, 0.0, 0.0, 0, 0))
        .unwrap();
    store.drain_lifecycle_events();

    level.with_level(|level| crate::entity::tick_one_entity(&mut store, level, id));

    assert!(store.get(id).is_none());
}

#[test]
fn retrieving_a_bite_costs_the_vanilla_rod_damage() {
    let (mut store, mut level, id) = bite_scenario(555);
    let owner = level.with_level(|level| level.player(OWNER)).unwrap();
    let mut hook = store.detach(id).expect("the hook is live");
    let outcome = level
        .with_level(|level| retrieve(&mut hook, &mut store, level, &owner, &rod_tool(), &|_| None));
    store.attach(hook);
    assert_eq!(outcome.rod_damage, 1);
    assert_eq!(outcome.pulled_entity_id, None);
    assert!(store.get(id).is_none(), "retrieve discards the hook");
}

#[test]
fn a_hook_lets_go_of_a_dying_entity() {
    let (mut store, mut level, _sender, _receiver) = owner_at(
        OWNER,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        9,
    );
    let hook = level
        .with_level(|_| {
            spawn_fishing_bobber(
                &mut store,
                OWNER,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 0.5,
                },
                68.0,
                0.0,
                0.0,
                0,
                0,
            )
        })
        .expect("a fresh UUID");
    {
        let entity = store.get_mut(hook).unwrap();
        entity.set_pos(Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        });
        entity.set_delta_movement(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        });
        store.refile(hook);
    }
    let target = spawn_armor_stand(
        &mut store,
        Vec3d {
            x: 2.5,
            y: 68.0,
            z: 0.5,
        },
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    level.with_level(|level| tick_entities(&mut store, level, true));
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(
        bobber_data(&store, hook).current_state(),
        FishHookState::HookedInEntity
    );

    crate::entity::living::set_health(store.get_mut(target).unwrap(), 0.0);
    level.with_level(|level| tick_entities(&mut store, level, true));
    let data = bobber_data(&store, hook);
    assert_eq!(data.hooked_entity_id(), None);
    assert_eq!(data.current_state(), FishHookState::Flying);
}

#[test]
fn retrieving_a_hooked_entity_pulls_it_toward_the_owner() {
    let owner_position = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let (mut store, mut level, _sender, _receiver) = owner_at(OWNER, owner_position, 9);
    let hook = level
        .with_level(|_| {
            spawn_fishing_bobber(&mut store, OWNER, owner_position, 68.0, 0.0, 0.0, 0, 0)
        })
        .expect("a fresh UUID");
    {
        let entity = store.get_mut(hook).unwrap();
        entity.set_pos(Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        });
        entity.set_delta_movement(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        });
        store.refile(hook);
    }
    let target = spawn_armor_stand(
        &mut store,
        Vec3d {
            x: 2.5,
            y: 68.0,
            z: 0.5,
        },
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(bobber_data(&store, hook).hooked_entity_id(), Some(target));

    let owner = level.with_level(|level| level.player(OWNER)).unwrap();
    let before = store.get(target).unwrap().delta_movement();
    let hook_position = store.get(hook).unwrap().position();
    let mut detached = store.detach(hook).expect("the hook is live");
    let outcome = level.with_level(|level| {
        retrieve(
            &mut detached,
            &mut store,
            level,
            &owner,
            &rod_tool(),
            &|_| None,
        )
    });
    store.attach(detached);

    assert_eq!(outcome.rod_damage, 5);
    assert_eq!(outcome.pulled_entity_id, Some(target));
    let expected = before.plus(
        Vec3d {
            x: owner_position.x - hook_position.x,
            y: owner_position.y - hook_position.y,
            z: owner_position.z - hook_position.z,
        }
        .scale(0.1),
    );
    assert_eq!(store.get(target).unwrap().delta_movement(), expected);
    assert!(store.get(hook).is_none());
}

#[test]
fn retrieving_a_grounded_hook_deals_two_damage() {
    let owner_position = Vec3d {
        x: 0.5,
        y: 64.2,
        z: 0.5,
    };
    let (mut store, mut level, _sender, _receiver) = owner_at(OWNER, owner_position, 2);
    let id = level
        .with_level(|_| {
            spawn_fishing_bobber(&mut store, OWNER, owner_position, 64.2, 0.0, 0.0, 0, 0)
        })
        .expect("a fresh UUID");
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_pos(Vec3d {
            x: 0.5,
            y: 64.4,
            z: 0.5,
        });
        entity.set_delta_movement(Vec3d {
            x: 0.0,
            y: -0.2,
            z: 0.0,
        });
        store.refile(id);
    }
    let mut landed = false;
    for _ in 0..40 {
        level.with_level(|level| tick_entities(&mut store, level, true));
        if store.get(id).map(|e| e.on_ground()).unwrap_or(false) {
            landed = true;
            break;
        }
    }
    assert!(landed, "the hook never came to rest on the plate");

    let owner = level.with_level(|level| level.player(OWNER)).unwrap();
    let mut hook = store.detach(id).expect("the hook is live");
    let outcome = level
        .with_level(|level| retrieve(&mut hook, &mut store, level, &owner, &rod_tool(), &|_| None));
    store.attach(hook);
    assert_eq!(outcome.rod_damage, 2);
    assert!(outcome.caught.is_empty());
}

#[test]
fn a_fishing_bobber_is_never_written_to_a_region() {
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    level
        .with_level(|_| {
            spawn_fishing_bobber(
                &mut store,
                OWNER,
                Vec3d {
                    x: 0.5,
                    y: 68.0,
                    z: 0.5,
                },
                68.0,
                0.0,
                0.0,
                0,
                0,
            )
        })
        .expect("a fresh UUID");
    let document = write_entity_chunk(&store, (0, 0)).expect("the writer skips it cleanly");
    assert_eq!(document.entities().len(), 0);
}
