use super::*;

fn collect_compiled<T>(out: &mut BTreeSet<(usize, JavaString)>, table: &Compiled<T>) {
    for (index, named) in table {
        for name in named.keys() {
            assert!(
                out.insert((*index, name.clone())),
                "duplicate compiled criterion {index}/{name}"
            );
        }
    }
}

fn collect_keyed<T>(
    out: &mut BTreeSet<(usize, JavaString)>,
    tables: &BTreeMap<String, Compiled<T>>,
) {
    for table in tables.values() {
        collect_compiled(out, table);
    }
}

#[test]
fn all_built_in_26_1_advancements_load_and_compile_every_criterion() {
    let startup = built_in_startup();
    assert!(
        startup.advancements.warnings.is_empty(),
        "{:?}",
        startup.advancements.warnings
    );
    assert_eq!(startup.advancements.advancements.len(), 1617);

    let tree = AdvancementTree::from_loaded(&startup.advancements);
    let compiled = AdvancementTriggers::compile(&tree, Some(startup));

    let mut expected = BTreeSet::new();
    for element in &startup.advancements.advancements {
        let index = tree
            .index_of(&element.id)
            .expect("built-in advancement in tree");
        for criterion in &element.advancement.criteria {
            if criterion.trigger != "minecraft:impossible" {
                expected.insert((index, criterion.name.clone()));
            }
        }
    }

    let mut actual = BTreeSet::new();
    for entries in compiled.recipes.values() {
        for (index, name, _) in entries {
            assert!(
                actual.insert((*index, name.clone())),
                "duplicate recipe_unlocked criterion {index}/{name}"
            );
        }
    }
    collect_compiled(&mut actual, &compiled.inventory);
    collect_compiled(&mut actual, &compiled.dimension_changes);
    collect_keyed(&mut actual, &compiled.distance);
    collect_keyed(&mut actual, &compiled.item_used_on_location);
    collect_keyed(&mut actual, &compiled.player);
    collect_compiled(&mut actual, &compiled.effects_changed);
    collect_compiled(&mut actual, &compiled.levitation);
    collect_keyed(&mut actual, &compiled.killed);
    collect_compiled(&mut actual, &compiled.tamed);
    collect_compiled(&mut actual, &compiled.summoned);
    collect_compiled(&mut actual, &compiled.bred);
    collect_compiled(&mut actual, &compiled.brewed_potion);
    collect_compiled(&mut actual, &compiled.cured_zombie_villager);
    collect_compiled(&mut actual, &compiled.lightning_strike);
    collect_compiled(&mut actual, &compiled.channeled_lightning);
    collect_compiled(&mut actual, &compiled.killed_by_arrow);
    collect_compiled(&mut actual, &compiled.target_hit);
    collect_compiled(&mut actual, &compiled.entity_hurt_player);
    collect_compiled(&mut actual, &compiled.player_hurt_entity);
    collect_compiled(&mut actual, &compiled.traded);
    collect_keyed(&mut actual, &compiled.interacted);
    collect_keyed(&mut actual, &compiled.item_criteria);
    collect_compiled(&mut actual, &compiled.generate_loot);
    collect_keyed(&mut actual, &compiled.block_contact);
    collect_compiled(&mut actual, &compiled.construct_beacon);
    collect_keyed(&mut actual, &compiled.block_use);
    collect_keyed(&mut actual, &compiled.recipe_crafted);
    collect_compiled(&mut actual, &compiled.picked_up_item);
    collect_compiled(&mut actual, &compiled.enchanted_item);
    collect_compiled(&mut actual, &compiled.item_durability);
    collect_compiled(&mut actual, &compiled.bee_nest_destroyed);
    collect_compiled(&mut actual, &compiled.used_ender_eye);
    collect_compiled(&mut actual, &compiled.spear_mobs);
    collect_compiled(&mut actual, &compiled.fishing_rod_hooked);

    assert_eq!(actual.len(), expected.len());
    assert_eq!(actual, expected);
}

#[test]
fn datapack_with_malformed_criterion_drops_advancement_and_descendant_in_world() {
    let root = std::env::temp_dir().join(format!("bbb-criterion-validation-{}", Uuid::new_v4()));
    let pack_dir = root.join("datapacks/custom");
    let adv_dir = pack_dir.join("data/test/advancement");
    std::fs::create_dir_all(&adv_dir).unwrap();
    std::fs::write(
        pack_dir.join("pack.mcmeta"),
        br#"{"pack":{"description":"criterion validation","min_format":101,"max_format":101}}"#,
    )
    .unwrap();
    std::fs::write(
        adv_dir.join("valid.json"),
        br#"{"criteria":{"seen":{"trigger":"minecraft:tick"}}}"#,
    )
    .unwrap();
    std::fs::write(
        adv_dir.join("half_broken.json"),
        br#"{"criteria":{"seen":{"trigger":"minecraft:tick"},"broken":{"trigger":"minecraft:construct_beacon","conditions":{"level":"nope"}}}}"#,
    )
    .unwrap();
    std::fs::write(
        adv_dir.join("child_of_broken.json"),
        br#"{"parent":"test:half_broken","criteria":{"seen":{"trigger":"minecraft:tick"}}}"#,
    )
    .unwrap();

    let startup = Arc::new(
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: Some(root.join("datapacks")),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        })
        .unwrap(),
    );
    std::fs::remove_dir_all(&root).unwrap();

    assert!(startup.advancements.advancement("test:valid").is_some());
    assert!(startup
        .advancements
        .advancement("test:half_broken")
        .is_none());
    assert!(startup
        .advancements
        .advancement("test:child_of_broken")
        .is_none());
    assert!(startup
        .advancements
        .warnings
        .iter()
        .any(|warning| warning.element_id == "test:half_broken"));
    assert!(startup
        .advancements
        .warnings
        .iter()
        .any(|warning| warning.element_id == "test:child_of_broken"));

    let mut world = world(&[]);
    world.cold.config.datapacks = Some(startup);
    world.reload_player_advancement_stores();

    assert!(world.cold.advancement_tree.index_of("test:valid").is_some());
    assert!(world
        .cold
        .advancement_tree
        .index_of("test:half_broken")
        .is_none());
    assert!(world
        .cold
        .advancement_tree
        .index_of("test:child_of_broken")
        .is_none());

    world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
        world.last_command_sequence + 1,
        crate::mailbox::WorldProducerClass::Network,
        WorldCommand::Play {
            connection_id: CONNECTION,
            packet: PlayServerbound::PlayerLoaded,
        },
    )]);
    world.tick(vec![]);

    assert!(world.player_advancement_is_done(CONNECTION, "test:valid"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:half_broken"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:child_of_broken"));
}
