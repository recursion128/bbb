use super::*;

#[test]
fn young_villagers_chase_and_flee_during_play_and_growing_up_ends_the_game() {
    let mut village = Village::new();
    village.command("time set 3000");
    village.command("gamerule advance_time false");
    village.bed(village.at(1, 3));
    let first = village.villager(village.at(0, 0));
    let second = village.villager(village.at(7, 0));
    for id in [first, second] {
        village.merge(id, vec![entry("Age", NbtSummaryValue::Int(-800))]);
    }
    let ids = [first, second];
    let mut previous = ids.map(|id| village.entity(id).unwrap().position());
    let mut maxima = [0.0_f64; 2];
    let mut steady_ground = [0; 2];
    let mut nearest = 100.0_f64;
    let mut farthest_after_meeting = 0.0_f64;
    village.tick_until(600, |village| {
        for (i, id) in ids.into_iter().enumerate() {
            let entity = village.entity(id).unwrap();
            steady_ground[i] = if entity.on_ground() && entity.position().y == previous[i].y {
                steady_ground[i] + 1
            } else {
                0
            };
            if steady_ground[i] >= 4 {
                maxima[i] = maxima[i].max(village.distance(id, previous[i]));
            }
            previous[i] = entity.position();
        }
        let distance = village.distance(first, previous[1]);
        nearest = nearest.min(distance);
        if nearest < 3.0 {
            farthest_after_meeting = farthest_after_meeting.max(distance);
        }
        false
    });
    assert!(nearest < 1.25);
    assert!(farthest_after_meeting > 3.0);
    assert!(maxima.into_iter().all(|speed| speed > 0.15));
    village.tick_until(240, |_| false);
    for id in ids {
        assert!(
            matches!(village.entity(id).unwrap().data(), EntityData::GeneralLiving(data)
            if data.ageable_retained().is_some_and(|age| !age.is_baby()))
        );
    }
    let mut grown_maxima = [0.0_f64; 2];
    previous = ids.map(|id| village.entity(id).unwrap().position());
    village.tick_until(200, |village| {
        for (i, id) in [first, second].into_iter().enumerate() {
            let entity = village.entity(id).unwrap();
            steady_ground[i] = if entity.on_ground() && entity.position().y == previous[i].y {
                steady_ground[i] + 1
            } else {
                0
            };
            if steady_ground[i] >= 4 {
                grown_maxima[i] = grown_maxima[i].max(village.distance(id, previous[i]));
            }
            previous[i] = entity.position();
        }
        false
    });
    // Ordinary adult walking is slower than tag running, even after the age change.
    assert!(grown_maxima.into_iter().all(|speed| speed < 0.15));
}

#[test]
fn children_only_start_chases_with_visible_young_villagers() {
    for (wall, adult) in [(true, false), (false, false), (false, true)] {
        let mut village = Village::new();
        village.command("time set 3000");
        village.command("gamerule advance_time false");
        if wall {
            village.pen();
        }
        let child = village.villager(village.at(0, 0));
        village.merge(child, vec![entry("Age", NbtSummaryValue::Int(-24_000))]);
        let mut previous = village.entity(child).unwrap().position();
        let mut ordinary_speed = 0.0_f64;
        village.tick_until(250, |village| {
            ordinary_speed = ordinary_speed.max(village.distance(child, previous));
            previous = village.entity(child).unwrap().position();
            false
        });
        assert!(ordinary_speed > 0.05, "the lone child wanders");
        let origin = village.center(village.at(0, 0));
        village.merge(
            child,
            vec![
                entry("Age", NbtSummaryValue::Int(-24_000)),
                entry(
                    "Pos",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Double(origin.x.to_bits()),
                        NbtSummaryValue::Double(origin.y.to_bits()),
                        NbtSummaryValue::Double(origin.z.to_bits()),
                    ]),
                ),
            ],
        );
        let other = village.villager(village.at(if wall { 7 } else { 9 }, 0));
        village.merge(
            other,
            vec![
                entry("Age", NbtSummaryValue::Int(if adult { 0 } else { -24_000 })),
                entry("NoAI", NbtSummaryValue::Byte(1)),
            ],
        );
        if !wall && !adult {
            village.command(&format!(
                "effect give {} minecraft:invisibility infinite",
                village.entity(other).unwrap().uuid()
            ));
        }
        let other_pos = village.entity(other).unwrap().position();
        previous = village.entity(child).unwrap().position();
        let mut hidden_speed = 0.0_f64;
        village.tick_until(300, |village| {
            if !wall && !adult && village.distance(child, other_pos) < 2.5 {
                return true;
            }
            hidden_speed = hidden_speed.max(village.distance(child, previous));
            previous = village.entity(child).unwrap().position();
            false
        });
        assert!(hidden_speed <= ordinary_speed * 1.05);
    }
}

#[test]
fn a_lone_child_follows_visible_adults_cats_and_the_animals_it_watches() {
    for kind in [
        VILLAGER,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
    ] {
        let mut village = Village::new();
        village.command("time set 3000");
        village.command("gamerule advance_time false");
        village.pen();
        let child = village.villager(village.at(0, 0));
        village.merge(child, vec![entry("Age", NbtSummaryValue::Int(-24_000))]);
        let partner = village.mob(kind, village.at(4, -2));
        village.merge(partner, vec![entry("NoAI", NbtSummaryValue::Byte(1))]);
        for at in [village.at(4, -2), village.at(0, 2), village.at(4, 2)] {
            let pos = village.center(at);
            village.command(&format!(
                "tp {} {} {} {}",
                village.entity(partner).unwrap().uuid(),
                pos.x,
                pos.y,
                pos.z
            ));
            assert!(
                village.tick_until(400, |village| {
                    let entity = village.entity(child).unwrap();
                    let from = entity.position();
                    let wanted =
                        ((pos.z - from.z).atan2(pos.x - from.x).to_degrees() - 90.0) as f32;
                    let yaw = entity.living().unwrap().y_head_rot;
                    let difference = (yaw - wanted + 180.0).rem_euclid(360.0) - 180.0;
                    village.distance(child, pos) < 2.5 && difference.abs() < 15.0
                }),
                "the child follows its visible {kind} friend to {pos:?}; child={:?}, partner={:?}",
                village.entity(child).unwrap().position(),
                village.entity(partner).unwrap().position()
            );
        }
    }
}

#[test]
fn a_child_walks_to_a_bed_and_jumps_on_it_but_stops_at_night_or_when_grown() {
    for night in [true, false] {
        let mut village = Village::new();
        village.command("time set 3000");
        village.command("gamerule advance_time false");
        village.pen();
        let bed = village.bed(village.at(1, 0));
        let child = village.villager(village.at(4, 0));
        village.merge(child, vec![entry("Age", NbtSummaryValue::Int(-24_000))]);
        let bed_center = village.center(bed);
        let mut max_height = 0.0_f64;
        assert!(
            village.tick_until(1_200, |village| {
                max_height = max_height.max(village.entity(child).unwrap().position().y - f64::from(bed.y));
                village.distance(child, bed_center) < 1.5
                    && village.entity(child).unwrap().position().y > f64::from(bed.y) + 1.4
            }),
            "a real jump from the bed surface rises above a navigation hop onto it; max={max_height}; pos={:?}", village.entity(child).unwrap().position()
        );
        if night {
            village.command("time set 13000");
        } else {
            village.merge(child, vec![entry("Age", NbtSummaryValue::Int(-40))]);
        }
        village.tick_until(200, |_| false);
        if night {
            assert!(village.sleeping_in(child, bed));
        }
        assert!(
            !village.tick_until(200, |village| village.entity(child).unwrap().position().y
                > f64::from(bed.y) + 1.4)
        );
    }
}

#[test]
fn a_child_wandering_outside_the_village_moves_towards_its_occupied_bed_section() {
    let mut village = Village::new();
    village.command("time set midnight");
    village.command("gamerule advance_time false");
    let bed = village.bed(village.at(-24, 0));
    let resident = village.villager(village.at(-22, 0));
    assert!(village.tick_until(1_200, |village| village.sleeping_in(resident, bed)));
    village.merge(resident, vec![entry("NoAI", NbtSummaryValue::Byte(1))]);
    village.command("time set 3000");
    let child = village.villager(village.at(40, 0));
    village.merge(child, vec![entry("Age", NbtSummaryValue::Int(-24_000))]);
    let home = village.center(bed);
    assert!(village.distance(child, home) > 60.0);
    // The destination stays outside the bed search's 48 blocks. Ground
    // navigation only takes the sampled cells at foot level, so roughly half
    // of the strolls towards the village find no target.
    assert!(village.tick_until(1_200, |village| village.distance(child, home) < 58.0));
}
