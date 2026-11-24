use crate::random::GameRandom;
use bbb_protocol::entity_types::{
    vanilla_entity_type_eye_height, VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use super::*;
use crate::entity::ai::mob::MobAiContext;
use crate::entity::general_living::{AnimalRetainedState, GeneralLivingData};
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::EntityData;

fn position(x: f64) -> Vec3d {
    Vec3d { x, y: 64.0, z: 0.5 }
}

/// One animal with the love timer a save's `InLove` writes
/// (`Animal.readAdditionalSaveData`).
fn animal_at(
    store: &mut EntityStore,
    uuid: u128,
    entity_type_id: i32,
    x: f64,
    in_love: i32,
) -> i32 {
    let id = store
        .restore(
            Uuid::from_u128(uuid),
            entity_type_id,
            position(x),
            EntityData::GeneralLiving(
                GeneralLivingData::new(entity_type_id).expect("the type is general living"),
            ),
        )
        .expect("the animal restores");
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).expect("stored").data else {
        unreachable!("an animal is general living")
    };
    *data.animal_retained_mut().expect("an animal is an Animal") =
        AnimalRetainedState::from_saved(in_love, None);
    id
}

/// One tamable animal with the love timer and the tame state a save writes
/// (`TamableAnimal.readAdditionalSaveData`).
fn tamable_at(
    store: &mut EntityStore,
    uuid: u128,
    entity_type_id: i32,
    x: f64,
    owner: Option<Uuid>,
    ordered_to_sit: bool,
) -> i32 {
    let id = animal_at(store, uuid, entity_type_id, x, 600);
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).expect("stored").data else {
        unreachable!("a tamable animal is general living")
    };
    *data.tamable_retained_mut().unwrap() =
        crate::entity::general_living::TamableRetainedState::from_saved(owner, ordered_to_sit);
    id
}

fn admits(store: &mut EntityStore, body_id: i32) -> bool {
    let mut goal = BreedGoal::new(1.0);
    let body = store.get(body_id).expect("the body is stored");
    let living = body.living().expect("the body is living");
    let mut context = MobAiContext::from_living_body(
        body_id,
        1,
        body.position(),
        body.bounding_box(),
        body.position().y
            + f64::from(vanilla_entity_type_eye_height(body.entity_type_id()).unwrap()),
        body.y_rot(),
        living.y_body_rot,
        body.on_ground,
        false,
        false,
        living,
        GameRandom::with_seed(7),
    );
    let sensing = crate::entity::ai::mob::Sensing::default();
    let mut native = crate::entity::tests::native_level::NativeLevel::new();
    native.with_level(|level| {
        let mut targets =
            crate::entity::ai::mob::target::TargetView::new(body, store, level, &sensing);
        goal.can_use(&mut context, &mut targets)
    })
}

/// `Cat.canMate` and `Wolf.canMate`
/// both refuse an untamed half of the pair, and only the wolf also refuses a
/// partner still in the sitting pose -- which is exactly the state
/// `tryToTame` leaves a freshly tamed wolf in.
#[test]
fn the_tamable_mate_gates_want_two_tame_parents_and_a_wolf_partner_on_its_feet() {
    let owner = Uuid::from_u128(0xfeed);
    for entity_type_id in [VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_WOLF_ID] {
        let mut store = EntityStore::new(3, EntityIdCounter::new());
        let body = tamable_at(&mut store, 1, entity_type_id, 0.5, None, false);
        let partner = tamable_at(&mut store, 2, entity_type_id, 2.0, None, false);
        assert!(!admits(&mut store, body), "neither half is tame");

        let EntityData::GeneralLiving(data) = &mut store.get_mut(body).expect("stored").data else {
            unreachable!()
        };
        *data.tamable_retained_mut().unwrap() =
            crate::entity::general_living::TamableRetainedState::from_saved(Some(owner), false);
        assert!(!admits(&mut store, body), "the partner is not tame");

        let EntityData::GeneralLiving(data) = &mut store.get_mut(partner).expect("stored").data
        else {
            unreachable!()
        };
        *data.tamable_retained_mut().unwrap() =
            crate::entity::general_living::TamableRetainedState::from_saved(Some(owner), false);
        assert!(admits(&mut store, body), "two tame parents mate");

        // `readAdditionalSaveData` couples `Sitting` to the pose bit
        // which is what a freshly tamed wolf
        // carries.
        let EntityData::GeneralLiving(data) = &mut store.get_mut(partner).expect("stored").data
        else {
            unreachable!()
        };
        *data.tamable_retained_mut().unwrap() =
            crate::entity::general_living::TamableRetainedState::from_saved(Some(owner), true);
        assert_eq!(
            admits(&mut store, body),
            entity_type_id == VANILLA_ENTITY_TYPE_CAT_ID,
            "only `Wolf.canMate` reads the partner's sitting pose"
        );
    }
}

#[test]
fn camel_mating_requires_full_health_and_no_passenger_or_vehicle() {
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_PIG_ID};
    for refusal in 0..4 {
        let mut store = EntityStore::new(7, EntityIdCounter::new());
        let first = animal_at(&mut store, 1, VANILLA_ENTITY_TYPE_CAMEL_ID, 0.5, 600);
        let second = animal_at(&mut store, 2, VANILLA_ENTITY_TYPE_CAMEL_ID, 1.5, 600);
        match refusal {
            1 => {
                crate::entity::living::set_health(store.get_mut(second).unwrap(), 10.0);
            }
            2 | 3 => {
                let pig = animal_at(&mut store, 3, VANILLA_ENTITY_TYPE_PIG_ID, 1.5, 0);
                let (vehicle, passenger) = if refusal == 2 {
                    (second, pig)
                } else {
                    (pig, second)
                };
                assert!(store.link_existing_passenger_forced(vehicle, passenger));
            }
            _ => {}
        }
        assert_eq!(admits(&mut store, first), refusal == 0);
    }
}

/// `Sniffer.canMate`: a digging sniffer is no mate.
#[test]
fn a_digging_sniffer_does_not_mate() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID;
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let body = animal_at(&mut store, 1, VANILLA_ENTITY_TYPE_SNIFFER_ID, 0.5, 600);
    let partner = animal_at(&mut store, 2, VANILLA_ENTITY_TYPE_SNIFFER_ID, 2.5, 600);
    assert!(admits(&mut store, body));
    crate::entity::sniffer::set_digging_for_test(store.get_mut(partner).unwrap());
    assert!(!admits(&mut store, body));
}
