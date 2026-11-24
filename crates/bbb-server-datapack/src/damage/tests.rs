use super::*;

/// The whole vanilla `minecraft:damage_type` registry, with no datapack
/// generation behind it — which is the state every world built without a
/// runtime is in.
fn registry() -> &'static [DamageTypeFacts] {
    builtin_damage_type_registry()
}

fn damage_type<'a>(registry: &'a [DamageTypeFacts], resource_id: &str) -> &'a DamageTypeFacts {
    registry
        .iter()
        .find(|entry| entry.resource_id == resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is in the damage type registry"))
}

fn hit(damage: f32) -> CombatEntry {
    CombatEntry {
        is_fall: false,
        always_most_significant_fall: false,
        attacker: None,
        attacker_weapon: None,
        damage,
        fall_location: None,
        fall_distance: 0.0,
    }
}

/// A tracker holding the one blow being processed.
fn struck() -> CombatTracker {
    let mut combat = CombatTracker::default();
    combat.record_damage(hit(1.0), 0, false);
    combat
}

fn survival_target(state: PlayerDamageState) -> HurtTarget {
    HurtTarget {
        state,
        abilities_invulnerable: false,
        entity_invulnerable: false,
        client_loaded: true,
        changing_dimension: false,
        max_health: player_max_health(),
        max_absorption: player_max_absorption(),
    }
}

fn generic<'a>(registry: &'a [DamageTypeFacts]) -> HurtSource<'a> {
    HurtSource {
        damage_type: damage_type(registry, "minecraft:generic"),
        causing_player: None,
        direct_player: None,
        source_position: None,
        arrow_owner_player: false,
        team_refuses_harm: false,
    }
}

#[test]
fn the_registry_reads_the_tags_the_hurt_path_tests() {
    let registry = registry();
    // `DamageTypeTags.BYPASSES_INVULNERABILITY` holds exactly the two types
    // that reach a creative player, which is what `/kill` uses.
    let bypassing = registry
        .iter()
        .filter(|entry| entry.bypasses_invulnerability)
        .map(|entry| entry.resource_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        bypassing,
        ["minecraft:generic_kill", "minecraft:out_of_world"],
        // The tag file lists `out_of_world` first; this is the registry's own
        // element order, which `RegistryDataLoader` takes from the
        // `damage_type/` directory listing and which the wire ids follow.
        "the registry's own element order",
    );
    // `minecraft:generic` is in neither cooldown nor invulnerability bypass,
    // which is what makes `/damage <target> <amount>` obey both.
    let generic = damage_type(registry, "minecraft:generic");
    assert!(!generic.bypasses_invulnerability);
    assert!(!generic.bypasses_cooldown);
    assert_eq!(generic.scaling, DamageScaling::WhenCausedByLivingNonPlayer);
    assert_eq!(generic.effects, DamageEffect::Hurt);
    assert_eq!(generic.death_message_type, DeathMessageType::Default);
    // The tags the four `Player.isInvulnerableTo` game rules read.
    assert!(damage_type(registry, "minecraft:fall").is_fall);
    assert!(damage_type(registry, "minecraft:in_fire").is_fire);
    assert!(damage_type(registry, "minecraft:drown").is_drowning);
    assert!(damage_type(registry, "minecraft:freeze").is_freezing);
    // `DamageTypeTags.BYPASSES_COOLDOWN` (DamageTypeTags.java:12) is the tag
    // that would let a second hit inside the twenty-tick window take its full
    // damage. Vanilla 26.1 declares the key and ships no tag file for it, so
    // nothing bypasses the window and every source obeys it. The day the
    // built-in pack grows one, this is what says so.
    assert!(
        registry.iter().all(|entry| !entry.bypasses_cooldown),
        "the built-in pack ships no bypasses_cooldown members",
    );

    let data = decode_direct_registry_json(
        br#"{"message_id":"onFire","scaling":"never","exhaustion":0.0}"#,
    )
    .unwrap();
    let codec = synchronized_registry_element_codec(DAMAGE_TYPE_REGISTRY, data.source).unwrap();
    let element = decode_damage_type(data.root(), codec).unwrap();
    let overlay = damage_type_registry(
        [("minecraft:on_fire".into(), element)],
        builtin_damage_type_tag_members,
    );
    assert_eq!(overlay[0].effects, DamageEffect::Hurt);
    assert_eq!(
        overlay[0].effects.sound_resource_id(),
        "minecraft:entity.player.hurt"
    );
}

/// `Player.createAttributes` declares no base for either attribute, so the two
/// registry defaults are what a player carrying no modifier has.
///
/// They are the base, not the cap. `setHealth` and `setAbsorptionAmount`
/// (LivingEntity.java:1146-1148, `:3333-3335`) clamp against `getMaxHealth()`
/// and `getMaxAbsorption()` (`:1954-1960`), which are the attributes with their
/// modifiers -- and `MobEffects.HEALTH_BOOST` and `MobEffects.ABSORPTION` each
/// carry one of `4.0 * (amplifier + 1)` (`MobEffects.java:78-88`).
#[test]
fn the_health_clamps_follow_the_attribute_rather_than_its_base() {
    assert_eq!(player_max_health(), 20.0);
    assert_eq!(player_max_absorption(), 0.0);

    let mut state = PlayerDamageState::created();
    state.set_health(25.0, player_max_health());
    assert_eq!(state.health, 20.0);
    state.set_health(25.0, 24.0);
    assert_eq!(state.health, 24.0);

    // A hit against a target whose absorption cap the effect raised is taken
    // out of the absorption first (`Player.actuallyHurt`, Player.java:752-755).
    let absorbed = hurt_player(
        HurtTarget {
            state: PlayerDamageState {
                absorption_amount: 4.0,
                ..PlayerDamageState::created()
            },
            max_absorption: 4.0,
            ..survival_target(PlayerDamageState::created())
        },
        generic(registry()),
        3.0,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert!(absorbed.succeeded);
    assert_eq!(absorbed.state.health, 20.0);
    assert_eq!(absorbed.state.absorption_amount, 1.0);
}

/// The two quantities `Player.actuallyHurt` awards its statistics from
/// (Player.java:756-758, `:764-766`). The health delta cannot stand in for the
/// applied one, because `setHealth` clamps at zero and the statistic does not.
#[test]
fn a_hurt_reports_what_it_applied_and_what_absorption_ate() {
    let registry = registry();
    let rules = DamageGameRules::default();

    let plain = hurt_player(
        survival_target(PlayerDamageState::created()),
        generic(registry),
        7.0,
        Difficulty::Normal,
        rules,
    );
    assert_eq!(plain.applied_damage, 7.0);
    assert_eq!(plain.absorbed_damage, 0.0);
    assert_eq!(plain.state.health, 13.0);

    // Absorption takes part of the hit, and what it took is not what landed.
    let split = hurt_player(
        HurtTarget {
            state: PlayerDamageState {
                absorption_amount: 4.0,
                ..PlayerDamageState::created()
            },
            max_absorption: 4.0,
            ..survival_target(PlayerDamageState::created())
        },
        generic(registry),
        7.0,
        Difficulty::Normal,
        rules,
    );
    assert_eq!(split.absorbed_damage, 4.0);
    assert_eq!(split.applied_damage, 3.0);
    assert_eq!(split.state.health, 17.0);

    // Absorption swallows it whole, which is vanilla's `dmg != 0.0F` guard on
    // `Stats.DAMAGE_TAKEN` reading false.
    let swallowed = hurt_player(
        HurtTarget {
            state: PlayerDamageState {
                absorption_amount: 9.0,
                ..PlayerDamageState::created()
            },
            max_absorption: 9.0,
            ..survival_target(PlayerDamageState::created())
        },
        generic(registry),
        7.0,
        Difficulty::Normal,
        rules,
    );
    assert_eq!(swallowed.absorbed_damage, 7.0);
    assert_eq!(swallowed.applied_damage, 0.0);
    assert_eq!(swallowed.state.absorption_amount, 2.0);
    assert_eq!(swallowed.state.health, 20.0);

    // The overkill the health delta would under-report.
    let overkill = hurt_player(
        survival_target(PlayerDamageState {
            health: 5.0,
            ..PlayerDamageState::created()
        }),
        generic(registry),
        40.0,
        Difficulty::Normal,
        rules,
    );
    assert_eq!(overkill.applied_damage, 40.0);
    assert_eq!(overkill.state.health, 0.0);
    assert!(overkill.died);

    // A refused hit never reaches `actuallyHurt` and reports neither.
    let refused = hurt_player(
        HurtTarget {
            abilities_invulnerable: true,
            ..survival_target(PlayerDamageState::created())
        },
        generic(registry),
        7.0,
        Difficulty::Normal,
        rules,
    );
    assert!(!refused.succeeded);
    assert_eq!(refused.applied_damage, 0.0);
    assert_eq!(refused.absorbed_damage, 0.0);
}

/// `LivingEntity.actuallyHurt` subtracts the applied damage from the absorption
/// a second time (LivingEntity.java:1936); `Player.actuallyHurt` does not, and
/// a player runs `Player`'s body. The two agree in exact arithmetic -- the
/// first subtraction has already emptied the absorption whenever any damage
/// gets through -- and stop agreeing when the hit is large enough that
/// `originalDamage - absorption` rounds back to `originalDamage`.
#[test]
fn a_player_keeps_the_absorption_that_the_living_entity_body_would_take_twice() {
    let huge = hurt_player(
        HurtTarget {
            state: PlayerDamageState {
                absorption_amount: 0.5,
                ..PlayerDamageState::created()
            },
            max_absorption: 0.5,
            ..survival_target(PlayerDamageState::created())
        },
        generic(registry()),
        1.0e9,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert_eq!(huge.absorbed_damage, 0.0, "1.0e9 - 0.5 is 1.0e9 in f32");
    assert_eq!(huge.state.absorption_amount, 0.5);
    assert!(huge.died);
}

/// `Player.actuallyHurt` charges `source.getFoodExhaustion()` on a hit that
/// put damage through (`Player.java:760-761`), which is the damage type's own
/// `exhaustion` (DamageType.java:12, DamageSource.java:26-28).
#[test]
fn a_hurt_reports_the_hunger_the_damage_type_costs() {
    let registry = registry();
    let rules = DamageGameRules::default();
    // Every built-in type carries one of two values, and the thirty-one that
    // cost anything all cost the same.
    assert_eq!(damage_type(registry, "minecraft:generic").exhaustion, 0.0);
    assert_eq!(damage_type(registry, "minecraft:cactus").exhaustion, 0.1);
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.exhaustion != 0.0)
            .count(),
        31,
    );

    let cactus = HurtSource {
        damage_type: damage_type(registry, "minecraft:cactus"),
        ..generic(registry)
    };
    let landed = hurt_player(
        survival_target(PlayerDamageState::created()),
        cactus,
        1.0,
        Difficulty::Normal,
        rules,
    );
    assert_eq!(landed.applied_damage, 1.0);
    assert_eq!(landed.food_exhaustion, 0.1);

    // `:760`'s guard: absorption swallowing the hit means nothing is charged.
    let swallowed = hurt_player(
        HurtTarget {
            state: PlayerDamageState {
                absorption_amount: 4.0,
                ..PlayerDamageState::created()
            },
            max_absorption: 4.0,
            ..survival_target(PlayerDamageState::created())
        },
        cactus,
        1.0,
        Difficulty::Normal,
        rules,
    );
    assert_eq!(swallowed.applied_damage, 0.0);
    assert_eq!(swallowed.food_exhaustion, 0.0);

    // So does a hit that never reaches `actuallyHurt` at all.
    let refused = hurt_player(
        HurtTarget {
            abilities_invulnerable: true,
            ..survival_target(PlayerDamageState::created())
        },
        cactus,
        1.0,
        Difficulty::Normal,
        rules,
    );
    assert!(!refused.succeeded);
    assert_eq!(refused.food_exhaustion, 0.0);
}

#[test]
fn a_creative_target_refuses_a_source_that_does_not_bypass_invulnerability() {
    let registry = registry();
    let target = HurtTarget {
        abilities_invulnerable: true,
        ..survival_target(PlayerDamageState::created())
    };
    let refused = hurt_player(
        target,
        generic(registry),
        5.0,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert!(!refused.succeeded);
    assert_eq!(refused.state, PlayerDamageState::created());

    // `minecraft:out_of_world` is in `BYPASSES_INVULNERABILITY`, so the same
    // target takes it.
    let killed = hurt_player(
        target,
        HurtSource {
            damage_type: damage_type(registry, "minecraft:out_of_world"),
            ..generic(registry)
        },
        5.0,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert!(killed.succeeded);
    assert_eq!(killed.state.health, 15.0);
}

#[test]
fn the_twenty_tick_window_only_charges_the_difference() {
    let registry = registry();
    let source = generic(registry);
    let first = hurt_player(
        survival_target(PlayerDamageState::created()),
        source,
        4.0,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert!(first.succeeded);
    assert_eq!(first.state.health, 16.0);
    assert_eq!(first.state.invulnerable_time, INVULNERABLE_TICKS);
    assert_eq!(first.state.last_hurt, 4.0);
    assert_eq!(first.state.hurt_time, HURT_DURATION);

    // `LivingEntity.hurtServer` (LivingEntity.java:1197-1198): a second hit no
    // bigger than the last one inside the window does nothing at all.
    let ignored = hurt_player(
        survival_target(first.state),
        source,
        4.0,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert!(!ignored.succeeded);
    assert_eq!(ignored.state, first.state);

    // `:1201-1203`: a bigger one takes only the difference, and — unlike the
    // `else` arm at `:1204-1210` — writes neither `invulnerableTime` nor
    // `hurtTime`. Both counters have to have moved off the value the first hit
    // left them at before the difference can be told from the whole, so the
    // window is walked partway down first: five ticks put it at fifteen, which
    // is still above the ten `:1196` tests.
    let mut inside_window = first.state;
    for _ in 0..5 {
        inside_window = tick_damage_state(inside_window);
    }
    assert_eq!(inside_window.invulnerable_time, INVULNERABLE_TICKS - 5);
    assert_eq!(inside_window.hurt_time, HURT_DURATION - 5);
    let topped_up = hurt_player(
        survival_target(inside_window),
        source,
        6.0,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert!(topped_up.succeeded);
    assert_eq!(topped_up.state.health, 14.0);
    assert_eq!(topped_up.state.last_hurt, 6.0);
    // Not reopened to twenty, and not replayed to ten.
    assert_eq!(topped_up.state.invulnerable_time, INVULNERABLE_TICKS - 5);
    assert_eq!(topped_up.state.hurt_time, HURT_DURATION - 5);
    assert!(!topped_up.broadcast_damage_event);
    assert_eq!(topped_up.sound, None);

    // Once the window has fallen to ten or below, the next hit is a full one
    // again (`:1196`).
    let mut state = first.state;
    for _ in 0..(INVULNERABLE_TICKS - 10) {
        state = tick_damage_state(state);
    }
    assert_eq!(state.invulnerable_time, 10);
    let full = hurt_player(
        survival_target(state),
        source,
        1.0,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert!(full.succeeded);
    assert_eq!(full.state.health, 15.0);
    assert_eq!(full.state.invulnerable_time, INVULNERABLE_TICKS);
}

#[test]
fn a_lethal_hit_bottoms_the_health_out_and_reports_the_death_sound() {
    let registry = registry();
    let killed = hurt_player(
        survival_target(PlayerDamageState::created()),
        generic(registry),
        40.0,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert!(killed.succeeded);
    assert!(killed.died);
    // `setHealth` clamps at zero rather than going negative.
    assert_eq!(killed.state.health, 0.0);
    assert!(killed.state.is_dead_or_dying());
    assert_eq!(killed.sound, Some(PLAYER_DEATH_SOUND));
    assert!(killed.broadcast_damage_event);

    // `Player.hurtServer` (Player.java:689-691) refuses a target that is
    // already dying.
    let again = hurt_player(
        survival_target(killed.state),
        generic(registry),
        1.0,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert!(!again.succeeded);
}

#[test]
fn the_death_counter_stops_where_vanilla_removes_the_body() {
    let mut state = PlayerDamageState {
        health: 0.0,
        ..PlayerDamageState::created()
    };
    for expected in 1..=DEATH_TICKS {
        state = tick_damage_state(state);
        assert_eq!(state.death_time, expected);
    }
    state = tick_damage_state(state);
    assert_eq!(state.death_time, DEATH_TICKS);
}

#[test]
fn a_restored_absorption_takes_the_hit_before_the_health_does() {
    let registry = registry();
    // `readAdditionalSaveData` (LivingEntity.java:778) writes the saved
    // absorption through `internalSetAbsorptionAmount`, which does not clamp,
    // so a document from an official save arrives with a non-zero one even
    // though nothing here could have produced it.
    let restored = PlayerDamageState {
        absorption_amount: 6.0,
        ..PlayerDamageState::created()
    };
    let absorbed = hurt_player(
        survival_target(restored),
        generic(registry),
        4.0,
        Difficulty::Normal,
        DamageGameRules::default(),
    );
    assert!(absorbed.succeeded);
    assert_eq!(absorbed.state.health, 20.0);
    // `setAbsorptionAmount` clamps to the player's max absorption of zero, so
    // the rest of the shield is gone with the hit it stopped.
    assert_eq!(absorbed.state.absorption_amount, 0.0);
}

#[test]
fn only_the_always_scaled_types_follow_the_difficulty() {
    let registry = registry();
    // `minecraft:generic` scales `WHEN_CAUSED_BY_LIVING_NON_PLAYER`, and no
    // source this server builds is a living non-player, so every difficulty
    // takes the same bite out of it.
    for difficulty in [
        Difficulty::Peaceful,
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
    ] {
        let hurt = hurt_player(
            survival_target(PlayerDamageState::created()),
            generic(registry),
            4.0,
            difficulty,
            DamageGameRules::default(),
        );
        assert!(hurt.succeeded, "{difficulty:?}");
        assert_eq!(hurt.state.health, 16.0, "{difficulty:?}");
    }

    // `minecraft:explosion` is one of the four `ALWAYS` types the built-in pack
    // ships, which is the arm `Player.hurtServer` (Player.java:694-706) scales:
    // peaceful zeroes it out and refuses, easy halves it and adds one, hard
    // multiplies by one and a half.
    let explosion = damage_type(registry, "minecraft:explosion");
    assert_eq!(explosion.scaling, DamageScaling::Always);
    let scaled = |difficulty| {
        hurt_player(
            survival_target(PlayerDamageState::created()),
            HurtSource {
                damage_type: explosion,
                ..generic(registry)
            },
            4.0,
            difficulty,
            DamageGameRules::default(),
        )
    };
    assert!(!scaled(Difficulty::Peaceful).succeeded);
    assert_eq!(scaled(Difficulty::Easy).state.health, 17.0);
    assert_eq!(scaled(Difficulty::Normal).state.health, 16.0);
    assert_eq!(scaled(Difficulty::Hard).state.health, 14.0);
}

#[test]
fn the_four_damage_game_rules_and_the_pvp_one_refuse_their_own_sources() {
    let registry = registry();
    let rules = |rule: fn(&mut DamageGameRules)| {
        let mut rules = DamageGameRules::default();
        rule(&mut rules);
        rules
    };
    for (resource_id, off) in [
        (
            "minecraft:fall",
            rules(|rules| rules.fall_damage = false) as DamageGameRules,
        ),
        (
            "minecraft:drown",
            rules(|rules| rules.drowning_damage = false),
        ),
        (
            "minecraft:in_fire",
            rules(|rules| rules.fire_damage = false),
        ),
        (
            "minecraft:freeze",
            rules(|rules| rules.freeze_damage = false),
        ),
    ] {
        let source = HurtSource {
            damage_type: damage_type(registry, resource_id),
            ..generic(registry)
        };
        assert!(
            hurt_player(
                survival_target(PlayerDamageState::created()),
                source,
                4.0,
                Difficulty::Normal,
                DamageGameRules::default(),
            )
            .succeeded,
            "{resource_id} lands while its rule is on"
        );
        assert!(
            !hurt_player(
                survival_target(PlayerDamageState::created()),
                source,
                4.0,
                Difficulty::Normal,
                off,
            )
            .succeeded,
            "{resource_id} is refused while its rule is off"
        );
    }

    // `ServerPlayer.hurtServer` (ServerPlayer.java:976-981) through
    // `canHarmPlayer`: a source naming a player is refused with pvp off.
    let by_player = HurtSource {
        causing_player: Some(HurtEntity {
            entity_id: 7,
            instabuild: false,
        }),
        direct_player: Some(HurtEntity {
            entity_id: 7,
            instabuild: false,
        }),
        ..generic(registry)
    };
    assert!(
        hurt_player(
            survival_target(PlayerDamageState::created()),
            by_player,
            4.0,
            Difficulty::Normal,
            DamageGameRules::default(),
        )
        .succeeded
    );
    assert!(
        !hurt_player(
            survival_target(PlayerDamageState::created()),
            by_player,
            4.0,
            Difficulty::Normal,
            DamageGameRules {
                pvp: false,
                ..DamageGameRules::default()
            },
        )
        .succeeded
    );
}

#[test]
fn a_creative_source_reaches_an_entity_invulnerable_target() {
    let registry = registry();
    // `Entity.isInvulnerableToBase` (Entity.java:2943): the `Invulnerable`
    // playerdata byte refuses everything except a bypassing type and a source
    // whose causing player is in creative.
    let target = HurtTarget {
        entity_invulnerable: true,
        ..survival_target(PlayerDamageState::created())
    };
    assert!(
        !hurt_player(
            target,
            generic(registry),
            4.0,
            Difficulty::Normal,
            DamageGameRules::default(),
        )
        .succeeded
    );
    assert!(
        hurt_player(
            target,
            HurtSource {
                causing_player: Some(HurtEntity {
                    entity_id: 7,
                    instabuild: true,
                }),
                direct_player: Some(HurtEntity {
                    entity_id: 7,
                    instabuild: true,
                }),
                ..generic(registry)
            },
            4.0,
            Difficulty::Normal,
            DamageGameRules::default(),
        )
        .succeeded
    );
}

#[test]
fn a_session_that_has_not_loaded_takes_nothing() {
    let registry = registry();
    // `ServerPlayer.isInvulnerableTo` (ServerPlayer.java:1266): the join window
    // before the client reports itself ready.
    let target = HurtTarget {
        client_loaded: false,
        changing_dimension: false,
        ..survival_target(PlayerDamageState::created())
    };
    assert!(
        !hurt_player(
            target,
            generic(registry),
            4.0,
            Difficulty::Normal,
            DamageGameRules::default(),
        )
        .succeeded
    );
}

/// The other arm of the same override: a session between the
/// `isChangingDimension = true` at `ServerPlayer.java:1102` and the
/// acknowledgement that clears it (`ServerGamePacketListenerImpl.java:544`)
/// takes nothing, however loaded its client is.
#[test]
fn a_session_in_the_middle_of_a_dimension_change_takes_nothing() {
    let registry = registry();
    let target = HurtTarget {
        client_loaded: true,
        changing_dimension: true,
        ..survival_target(PlayerDamageState::created())
    };
    assert!(
        !hurt_player(
            target,
            generic(registry),
            4.0,
            Difficulty::Normal,
            DamageGameRules::default(),
        )
        .succeeded
    );
}

#[test]
fn the_death_message_is_the_killing_blow_own_source() {
    let registry = registry();
    let victim = component_literal_nbt("alice");
    let attacker = component_literal_nbt("bob");

    // `DamageSource.getLocalizedDeathMessage` (DamageSource.java:73-78) with no
    // entity on the source and no kill credit remembered.
    let alone = death_message(
        &struck(),
        &generic(registry),
        victim.clone(),
        None,
        None,
        None,
    );
    assert_eq!(
        alone.component(),
        &translatable_with("death.attack.generic", vec![victim.clone()])
    );

    // The same source once a player is still remembered as the kill credit.
    let credited = death_message(
        &struck(),
        &generic(registry),
        victim.clone(),
        Some(attacker.clone()),
        None,
        None,
    );
    assert_eq!(
        credited.component(),
        &translatable_with(
            "death.attack.generic.player",
            vec![victim.clone(), attacker.clone()]
        )
    );

    // `:79-84` with the attacker on the source itself.
    let by_player = death_message(
        &struck(),
        &HurtSource {
            damage_type: damage_type(registry, "minecraft:player_attack"),
            ..generic(registry)
        },
        victim.clone(),
        None,
        Some(attacker.clone()),
        None,
    );
    assert_eq!(
        by_player.component(),
        &translatable_with(
            "death.attack.player",
            vec![victim.clone(), attacker.clone()]
        )
    );

    // `:80-83`: a custom-named weapon in the attacker's main hand.
    let sword = component_literal_nbt("[Excalibur]");
    let mut armed = CombatTracker::default();
    armed.record_damage(
        CombatEntry {
            attacker: Some(attacker.clone()),
            attacker_weapon: Some(sword.clone()),
            ..hit(4.0)
        },
        0,
        true,
    );
    let with_item = death_message(
        &armed,
        &HurtSource {
            damage_type: damage_type(registry, "minecraft:player_attack"),
            ..generic(registry)
        },
        victim.clone(),
        None,
        Some(attacker.clone()),
        None,
    );
    assert_eq!(
        with_item.component(),
        &translatable_with("death.attack.player.item", vec![victim, attacker, sword])
    );
    let data =
        bbb_registry_codec::RegistryElementData::from_network_nbt(NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "message_id".into(),
                value: NbtSummaryValue::string_from_utf16_slice(&[0xd800]),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "scaling".into(),
                value: NbtSummaryValue::string("never"),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "exhaustion".into(),
                value: NbtSummaryValue::Float(0.0f32.to_bits()),
            },
        ]));
    let codec = synchronized_registry_element_codec(DAMAGE_TYPE_REGISTRY, data.source).unwrap();
    let element = decode_damage_type(data.root(), codec).unwrap();
    let raw = damage_type_registry([("test:raw".into(), element)], |_| Vec::new())
        .pop()
        .unwrap();
    let raw_message = death_message(
        &struck(),
        &HurtSource {
            damage_type: &raw,
            ..generic(registry)
        },
        component_literal_nbt("alice"),
        Some(component_literal_nbt("bob")),
        None,
        None,
    );
    let NbtSummaryValue::Compound(fields) = raw_message.component() else {
        panic!("death component")
    };
    let mut units: Vec<u16> = "death.attack.".encode_utf16().collect();
    units.push(0xd800);
    units.extend(".player".encode_utf16());
    assert_eq!(
        fields
            .iter()
            .find(|field| field.name == "translate")
            .map(|field| &field.value),
        Some(&NbtSummaryValue::string_from_utf16(units))
    );
}

#[test]
fn the_intentional_game_design_message_carries_the_bracketed_bug_link() {
    let registry = registry();
    // `CombatTracker.getDeathMessage` (CombatTracker.java:98-101) for the one
    // type whose `death_message_type` is `intentional_game_design`.
    let bad_respawn = damage_type(registry, "minecraft:bad_respawn_point");
    assert_eq!(
        bad_respawn.death_message_type,
        DeathMessageType::IntentionalGameDesign
    );
    let victim = component_literal_nbt("alice");
    let message = death_message(
        &struck(),
        &HurtSource {
            damage_type: bad_respawn,
            ..generic(registry)
        },
        victim.clone(),
        None,
        None,
        None,
    );
    let NbtSummaryValue::Compound(entries) = message.component() else {
        panic!("a component is a compound");
    };
    let field = |name: &str| {
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    };
    assert_eq!(
        field("translate"),
        Some(&NbtSummaryValue::string(
            "death.attack.badRespawnPoint.message"
        ))
    );
    let Some(NbtSummaryValue::List(arguments)) = field("with") else {
        panic!("the message carries the victim and the link: {entries:?}");
    };
    assert_eq!(arguments.len(), 2);
    assert_eq!(arguments[0], victim);
    let NbtSummaryValue::Compound(link) = &arguments[1] else {
        panic!("the link is a component");
    };
    assert!(link.iter().any(|entry| entry.name == "translate"
        && entry.value == NbtSummaryValue::string("chat.square_brackets")));
    assert!(link.iter().any(|entry| entry.name == "click_event"));
    assert!(link.iter().any(|entry| entry.name == "hover_event"));
}

#[test]
fn invulnerability_query_preserves_state_and_excludes_hurt_only_gates() {
    let registry = registry();
    let rules = DamageGameRules::default();
    let base = survival_target(PlayerDamageState::created());
    let targets = [
        HurtTarget {
            abilities_invulnerable: true,
            ..base
        },
        HurtTarget {
            state: PlayerDamageState {
                health: 0.0,
                death_time: 12,
                ..base.state
            },
            ..base
        },
        HurtTarget {
            state: PlayerDamageState {
                invulnerable_time: INVULNERABLE_TICKS,
                last_hurt: 10.0,
                hurt_time: HURT_DURATION,
                ..base.state
            },
            ..base
        },
    ];
    for target in targets {
        let before = target;
        for resource_id in ["minecraft:wither", "minecraft:in_fire"] {
            let source = HurtSource {
                damage_type: damage_type(registry, resource_id),
                ..generic(registry)
            };
            assert!(!player_is_invulnerable_to(&target, &source, rules));
            assert!(!hurt_player(target, source, 1.0, Difficulty::Normal, rules).succeeded);
            assert_eq!(target, before);
        }
    }
    let source = HurtSource {
        causing_player: Some(HurtEntity {
            entity_id: 7,
            instabuild: false,
        }),
        ..generic(registry)
    };
    let no_pvp = DamageGameRules {
        pvp: false,
        ..rules
    };
    assert!(!player_is_invulnerable_to(&base, &source, no_pvp));
    assert!(!hurt_player(base, source, 1.0, Difficulty::Normal, no_pvp).succeeded);
}

#[test]
fn invulnerability_query_entity_flag_uses_causing_creative_player_and_type_bypass() {
    let registry = registry();
    let target = HurtTarget {
        entity_invulnerable: true,
        ..survival_target(PlayerDamageState::created())
    };
    let creative = HurtEntity {
        entity_id: 7,
        instabuild: true,
    };
    let rules = DamageGameRules::default();
    for (resource_id, causing_player, direct_player, immune) in [
        ("minecraft:generic", None, None, true),
        ("minecraft:generic", None, Some(creative), true),
        ("minecraft:generic", Some(creative), None, false),
        ("minecraft:generic_kill", None, None, false),
    ] {
        let source = HurtSource {
            damage_type: damage_type(registry, resource_id),
            causing_player,
            direct_player,
            source_position: None,
            arrow_owner_player: false,
            team_refuses_harm: false,
        };
        assert_eq!(player_is_invulnerable_to(&target, &source, rules), immune);
        assert_eq!(
            hurt_player(target, source, 1.0, Difficulty::Normal, rules).succeeded,
            !immune
        );
    }
}

#[test]
fn invulnerability_query_uses_first_damage_tag_before_server_player_gates() {
    let registry = registry();
    let target = survival_target(PlayerDamageState::created());
    for first in 0..4 {
        let mut damage_type = damage_type(registry, "minecraft:generic").clone();
        damage_type.is_drowning = first == 0;
        damage_type.is_fall = first <= 1;
        damage_type.is_fire = first <= 2;
        damage_type.is_freezing = true;
        let source = HurtSource {
            damage_type: &damage_type,
            ..generic(registry)
        };
        let rules = DamageGameRules {
            drowning_damage: first == 0,
            fall_damage: first == 1,
            fire_damage: first == 2,
            freeze_damage: first == 3,
            pvp: true,
        };
        assert!(!player_is_invulnerable_to(&target, &source, rules));
        assert!(hurt_player(target, source, 1.0, Difficulty::Normal, rules).succeeded);
        let disabled = DamageGameRules {
            drowning_damage: false,
            fall_damage: false,
            fire_damage: false,
            freeze_damage: false,
            ..rules
        };
        assert!(player_is_invulnerable_to(&target, &source, disabled));
        assert!(!hurt_player(target, source, 1.0, Difficulty::Normal, disabled).succeeded);
        let unloaded = HurtTarget {
            client_loaded: false,
            ..target
        };
        assert!(player_is_invulnerable_to(&unloaded, &source, rules));
    }
}

#[test]
fn ender_pearl_only_bypasses_dimension_change_in_query_and_hurt() {
    let registry = registry();
    for resource_id in ["minecraft:generic", "minecraft:ender_pearl"] {
        let source = HurtSource {
            damage_type: damage_type(registry, resource_id),
            ..generic(registry)
        };
        for changing_dimension in [false, true] {
            for client_loaded in [false, true] {
                for entity_invulnerable in [false, true] {
                    let target = HurtTarget {
                        changing_dimension,
                        client_loaded,
                        entity_invulnerable,
                        ..survival_target(PlayerDamageState::created())
                    };
                    let immune = entity_invulnerable
                        || !client_loaded
                        || (changing_dimension && resource_id != "minecraft:ender_pearl");
                    let rules = DamageGameRules::default();
                    assert_eq!(player_is_invulnerable_to(&target, &source, rules), immune);
                    let outcome = hurt_player(target, source, 5.0, Difficulty::Normal, rules);
                    assert_eq!(outcome.succeeded, !immune);
                    assert_eq!(outcome.state.health, if immune { 20.0 } else { 15.0 });
                }
            }
        }
    }
    let mut pearl = damage_type(registry, "minecraft:ender_pearl").clone();
    pearl.is_fall = true;
    let source = HurtSource {
        damage_type: &pearl,
        ..generic(registry)
    };
    let target = HurtTarget {
        changing_dimension: true,
        ..survival_target(PlayerDamageState::created())
    };
    let rules = DamageGameRules {
        fall_damage: false,
        ..DamageGameRules::default()
    };
    assert!(player_is_invulnerable_to(&target, &source, rules));
    assert!(!hurt_player(target, source, 5.0, Difficulty::Normal, rules).succeeded);
}

#[test]
fn a_fall_death_names_the_most_significant_fall() {
    let registry = registry();
    let fall = HurtSource {
        damage_type: damage_type(registry, "minecraft:fall"),
        ..generic(registry)
    };
    let victim = component_literal_nbt("alice");
    let bob = component_literal_nbt("bob");
    let carol = component_literal_nbt("carol");
    let fell = |distance: f32, location: Option<FallLocation>| CombatEntry {
        is_fall: true,
        fall_location: location,
        fall_distance: distance,
        ..hit(distance - 3.0)
    };
    let message = |entries: Vec<CombatEntry>, killer: Option<NbtSummaryValue>| {
        let mut combat = CombatTracker::default();
        for entry in entries {
            combat.record_damage(entry, 0, false);
        }
        death_message(&combat, &fall, victim.clone(), None, killer, None)
            .component()
            .clone()
    };

    // CombatTracker.java:128-140: three blocks is not significant.
    assert_eq!(
        message(vec![fell(3.0, None)], None),
        translatable_with("death.attack.fall", vec![victim.clone()])
    );
    assert_eq!(
        message(vec![fell(20.0, Some(FallLocation::Ladder))], None),
        translatable_with("death.fell.accident.ladder", vec![victim.clone()])
    );
    assert_eq!(
        message(vec![fell(20.0, None)], None),
        translatable_with("death.fell.accident.generic", vec![victim.clone()])
    );
    // The blow before the fall knocked the victim off.
    let knocked = CombatEntry {
        attacker: Some(bob.clone()),
        ..hit(2.0)
    };
    assert_eq!(
        message(vec![knocked.clone(), fell(20.0, None)], None),
        translatable_with("death.fell.assist", vec![victim.clone(), bob.clone()])
    );
    assert_eq!(
        message(vec![knocked.clone(), fell(20.0, None)], Some(bob.clone())),
        translatable_with("death.fell.finish", vec![victim.clone(), bob.clone()])
    );
    assert_eq!(
        message(vec![knocked.clone(), fell(20.0, None)], Some(carol.clone())),
        translatable_with("death.fell.assist", vec![victim.clone(), bob.clone()])
    );
    let sword = component_literal_nbt("[Excalibur]");
    let armed = CombatEntry {
        attacker_weapon: Some(sword.clone()),
        ..knocked
    };
    assert_eq!(
        message(vec![armed, fell(20.0, None)], None),
        translatable_with("death.fell.assist.item", vec![victim.clone(), bob, sword])
    );
    assert_eq!(
        message(vec![hit(2.0), fell(20.0, None)], None),
        translatable_with("death.fell.killer", vec![victim])
    );
}

#[test]
fn death_message_with_registry_accepts_mod_entity_hover_and_vanilla_falls_back_without_panicking() {
    use bbb_protocol::content_registry::{ContentRegistryBuilder, ModEntityRegistration};
    use bbb_protocol::{component_styled_literal_nbt, ComponentHoverEvent, ComponentStyle};

    let mut builder = ContentRegistryBuilder::new();
    builder.register_entity(ModEntityRegistration {
        resource_id: "bbb_demo:inferno_warden".to_string(),
        ..Default::default()
    });
    let content_registry = builder.freeze_for_session().unwrap();

    let registry = registry();
    let mob_attack = HurtSource {
        damage_type: damage_type(registry, "minecraft:mob_attack"),
        ..generic(registry)
    };
    let mut combat = CombatTracker::default();
    combat.record_damage(hit(20.0), 0, false);

    let victim = component_literal_nbt("Steve");
    let killer = component_styled_literal_nbt(
        "Inferno Warden",
        &ComponentStyle {
            hover_event: Some(ComponentHoverEvent::ShowEntity {
                id: "bbb_demo:inferno_warden".into(),
                uuid: NbtSummaryValue::string("00000000-0000-0000-0000-000000000001"),
                name: None,
                name_component: Some(component_literal_nbt("Inferno Warden")),
            }),
            ..ComponentStyle::default()
        },
    )
    .unwrap();

    let rich = death_message_with_registry(
        &combat,
        &mob_attack,
        victim.clone(),
        None,
        Some(killer.clone()),
        None,
        &content_registry,
    );
    assert_eq!(
        rich.component(),
        &translatable_with("death.attack.mob", vec![victim.clone(), killer.clone()])
    );

    // Calling the vanilla-only wrapper must not panic even when the killer hover names a mod entity.
    let fallback = death_message(&combat, &mob_attack, victim, None, Some(killer), None);
    assert_eq!(fallback.text(), "Steve was slain by Inferno Warden");
}
