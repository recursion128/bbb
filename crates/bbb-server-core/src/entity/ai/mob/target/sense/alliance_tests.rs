use super::*;
use crate::entity::ai::mob::target::{TargetRef, TargetView};
use crate::entity::ScoreboardTeams;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::Vec3d;

use crate::entity::ai::mob::target::conditions::{is_allied_to, TargetingConditions};
use crate::entity::general_living::VexRetainedState;
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::EntityIdCounter;
use crate::world::simulation_level::LevelCtx;
use bbb_facts::scoreboard_team_options::TeamCollisionRule;

fn at(x: f64) -> Vec3d {
    Vec3d { x, y: 64.0, z: 0.5 }
}

fn store() -> EntityStore {
    let mut store = EntityStore::new(97, EntityIdCounter::new());
    store.set_difficulty(Difficulty::Normal);
    store
}

fn spawn(store: &mut EntityStore, kind: i32, x: f64) -> i32 {
    store.spawn_general_living(kind, at(x)).unwrap()
}

fn vex_owner(store: &mut EntityStore, vex: i32, owner: i32) {
    let uuid = store.get(owner).unwrap().uuid();
    let EntityData::GeneralLiving(data) = &mut store.get_mut(vex).unwrap().data else {
        unreachable!()
    };
    *data.vex_retained_mut().unwrap() = VexRetainedState::from_saved(Some(uuid), None, None);
}

fn ownable_owner(store: &mut EntityStore, id: i32, owner: Option<Uuid>) {
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
        unreachable!()
    };
    if let Some(tamable) = data.tamable_retained_mut() {
        tamable.set_owner(owner);
        tamable.set_tame(owner.is_some());
    } else {
        data.abstract_horse_retained_mut().unwrap().set_owner(owner);
    }
}

fn teams(native: &mut NativeLevel, store: &EntityStore, groups: &[&[i32]]) {
    let mut teams = ScoreboardTeams::default();
    for group in groups {
        let names = group
            .iter()
            .map(|id| store.get(*id).unwrap().uuid().to_string().into())
            .collect::<Vec<_>>();
        teams.add_team(&names, TeamCollisionRule::Always);
    }
    native.set_scoreboard_teams(teams);
}

fn collect<'a, 'l, 'w>(
    store: &'a EntityStore,
    body: i32,
    level: &'a LevelCtx<'l, 'w>,
) -> Option<TargetView<'a, 'l, 'w>> {
    Some(
        store
            .mob_ai(body)?
            .target_view(store.get(body)?, store, level),
    )
}

#[test]
fn alliance_recursive_evoker_terminal_same_and_different_teams() {
    let mut native = NativeLevel::new();
    for allied in [true, false] {
        let mut store = store();
        let evoker = spawn(&mut store, VANILLA_ENTITY_TYPE_EVOKER_ID, 0.5);
        let first = spawn(&mut store, VANILLA_ENTITY_TYPE_VEX_ID, 2.5);
        let second = spawn(&mut store, VANILLA_ENTITY_TYPE_VEX_ID, 128.5);
        let terminal = spawn(&mut store, VANILLA_ENTITY_TYPE_EVOKER_ID, 256.5);
        vex_owner(&mut store, first, second);
        vex_owner(&mut store, second, terminal);
        if allied {
            teams(
                &mut native,
                &store,
                &[&[evoker, terminal], &[first, second]],
            );
        } else {
            teams(
                &mut native,
                &store,
                &[&[evoker], &[first, second, terminal]],
            );
        }
        native.with_level(|level| {
            let world = collect(&store, evoker, level).unwrap();
            assert_eq!(
                is_allied_to(&world, &world.body(), &world.facts(first).unwrap()),
                allied
            );
            assert_eq!(
                TargetingConditions::for_combat()
                    .ignore_line_of_sight()
                    .test(&world, &world.facts(first).unwrap(), |_| true),
                !allied
            );
        });
    }
}

#[test]
fn alliance_ownable_known_null_differs_from_nonownable_terminal() {
    let mut native = NativeLevel::new();
    for kind in [
        VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        VANILLA_ENTITY_TYPE_COW_ID,
    ] {
        let mut store = store();
        let wolf = spawn(&mut store, VANILLA_ENTITY_TYPE_WOLF_ID, 0.5);
        let terminal = spawn(&mut store, kind, 128.5);
        let uuid = store.get(terminal).unwrap().uuid();
        ownable_owner(&mut store, wolf, Some(uuid));
        teams(&mut native, &store, &[&[terminal]]);
        native.with_level(|level| {
            let world = collect(&store, wolf, level).unwrap();
            assert_eq!(world.body().owner(), Some(terminal));
            if kind == VANILLA_ENTITY_TYPE_COW_ID {
                assert_eq!(world.body().root_owner(), Some(terminal));
                assert_eq!(world.body().team(), world.facts(terminal).unwrap().team());
                assert!(is_allied_to(
                    &world,
                    &world.body(),
                    &world.facts(terminal).unwrap()
                ));
            } else {
                assert_eq!(world.body().root_owner(), None);
                assert_eq!(world.body().team(), None);
                assert!(!is_allied_to(
                    &world,
                    &world.body(),
                    &world.facts(terminal).unwrap()
                ));
            }
        });
    }
}

#[test]
fn alliance_tamable_delegates_to_outside_evoker_and_vex_chain() {
    let mut native = NativeLevel::new();
    let mut store = store();
    let wolf = spawn(&mut store, VANILLA_ENTITY_TYPE_WOLF_ID, 0.5);
    let first = spawn(&mut store, VANILLA_ENTITY_TYPE_VEX_ID, 2.5);
    let second = spawn(&mut store, VANILLA_ENTITY_TYPE_VEX_ID, 128.5);
    let evoker = spawn(&mut store, VANILLA_ENTITY_TYPE_EVOKER_ID, 256.5);
    let uuid = store.get(evoker).unwrap().uuid();
    ownable_owner(&mut store, wolf, Some(uuid));
    vex_owner(&mut store, first, second);
    vex_owner(&mut store, second, evoker);
    teams(&mut native, &store, &[&[evoker], &[first, second]]);
    native.with_level(|level| {
        let world = collect(&store, wolf, level).unwrap();
        assert_eq!(world.body().root_owner(), Some(evoker));
        assert_eq!(world.body().team(), world.facts(evoker).unwrap().team());
        assert!(is_allied_to(
            &world,
            &world.body(),
            &world.facts(first).unwrap()
        ));
    });
}

#[test]
fn alliance_vex_owner_requires_actual_mob_class() {
    let mut native = NativeLevel::new();
    let mut store = store();
    let evoker = spawn(&mut store, VANILLA_ENTITY_TYPE_EVOKER_ID, 0.5);
    let vex = spawn(&mut store, VANILLA_ENTITY_TYPE_VEX_ID, 2.5);
    let mannequin = spawn(
        &mut store,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        128.5,
    );
    vex_owner(&mut store, vex, mannequin);
    teams(&mut native, &store, &[&[evoker, mannequin], &[vex]]);
    native.with_level(|level| {
        let world = collect(&store, evoker, level).unwrap();
        assert_eq!(world.facts(vex).unwrap().owner(), None);
        assert!(!is_allied_to(
            &world,
            &world.body(),
            &world.facts(vex).unwrap()
        ));
    });
}

#[test]
fn an_enderman_accepts_a_live_target_and_refuses_it_after_death() {
    let mut native = NativeLevel::new();
    let mut store = store();
    let id = spawn(&mut store, VANILLA_ENTITY_TYPE_ENDERMAN_ID, 0.5);
    let cow = spawn(&mut store, VANILLA_ENTITY_TYPE_COW_ID, 2.5);
    let target = TargetRef {
        entity_id: cow,
        uuid: store.get(cow).unwrap().uuid(),
    };
    native.with_level(|level| set_mob_target(&mut store, level, id, Some(target)));
    assert_eq!(store.mob_ai(id).unwrap().target_ref(), Some(target));
    store
        .get_mut(cow)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    native.with_level(|level| set_mob_target(&mut store, level, id, Some(target)));
    assert_eq!(store.mob_ai(id).unwrap().target_ref(), None);
}

#[test]
fn a_cold_enderman_resolves_its_saved_anger_target() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut store = store();
        let cow = spawn(&mut store, VANILLA_ENTITY_TYPE_COW_ID, 2.5);
        let target_uuid = store.get(cow).unwrap().uuid();
        let id = store
            .restore_initialized(
                Uuid::from_u128(98765),
                VANILLA_ENTITY_TYPE_ENDERMAN_ID,
                at(0.5),
                EntityData::GeneralLiving(
                    crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ENDERMAN_ID).unwrap(),
                ),
                |body| {
                    super::super::anger::write(
                        body,
                        NeutralAnger {
                            end_time: 1000,
                            target: Some(target_uuid),
                        },
                    )
                },
            )
            .unwrap();
        let mut body = store.detach(id).unwrap();
        assert!(before_ai_step(&mut body, &mut store, level));
        let runtime = store.mob_ai(id).unwrap();
        assert_eq!(runtime.target_ref().unwrap().uuid, target_uuid);
    });
}

#[test]
fn enderman_target_keeps_indirect_stored_chain_when_body_is_detached() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut store = store();
        let id = spawn(&mut store, VANILLA_ENTITY_TYPE_ENDERMAN_ID, 0.5);
        let cow = spawn(&mut store, VANILLA_ENTITY_TYPE_COW_ID, 2.5);
        let nested = spawn(&mut store, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 3.5);
        let other = spawn(&mut store, VANILLA_ENTITY_TYPE_HORSE_ID, 4.5);
        assert!(store.link_existing_passenger_forced(id, cow));
        assert!(store.link_existing_passenger_forced(cow, nested));
        let body = store.detach(id).unwrap();
        let world = store.mob_ai(id).unwrap().target_view(&body, &store, level);
        assert_eq!(world.body().rides_body(), Some(false));
        assert_eq!(world.facts(cow).unwrap().rides_body(), Some(true));
        assert_eq!(world.facts(nested).unwrap().rides_body(), Some(true));
        assert_eq!(world.facts(other).unwrap().rides_body(), Some(false));
    });
}
