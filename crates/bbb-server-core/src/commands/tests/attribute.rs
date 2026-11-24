//! `/attribute` get, base, and modifier arms
//! (`AttributeCommand.java:42-223`, `:252-351`).

use super::*;

#[test]
fn attribute_get_reports_the_player_supplier_max_health() {
    let players = [user("alice", 1)];
    let run = ran("attribute alice minecraft:max_health get", world(&players));
    assert_eq!(run.result, Some(20));
    assert_eq!(keys(&run), ["commands.attribute.value.get.success"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            translatable("attribute.name.max_health", Vec::new()),
            display_name("alice", 1),
            NbtSummaryValue::Double(20.0_f64.to_bits()),
        ]
    );
}

#[test]
fn attribute_get_scale_multiplies_only_the_return_value() {
    let players = [user("alice", 1)];
    let run = ran(
        "attribute alice minecraft:max_health get 2",
        world(&players),
    );
    assert_eq!(run.result, Some(40));
    assert_eq!(
        arguments(&run, 0),
        vec![
            translatable("attribute.name.max_health", Vec::new()),
            display_name("alice", 1),
            NbtSummaryValue::Double(20.0_f64.to_bits()),
        ]
    );
}

#[test]
fn attribute_get_refuses_an_attribute_the_player_supplier_lacks() {
    let players = [user("alice", 1)];
    let run = ran(
        "attribute alice minecraft:flying_speed get",
        world(&players),
    );
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["commands.attribute.failed.no_attribute"]);
}

#[test]
fn attribute_get_refuses_a_non_living_stored_entity() {
    let entities = [stored_entity(
        9,
        99,
        [1.0, 64.0, 1.0],
        crate::entity::EntityData::ExperienceOrb(crate::entity::ExperienceOrbData::new(3)),
    )];
    let run = ran(
        "attribute 0-0-0-0-9 minecraft:max_health get",
        CommandTestWorld::default().with_entities(&entities),
    );
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["commands.attribute.failed.entity"]);
}

#[test]
fn attribute_get_reads_a_stored_living_entity() {
    let zombie = crate::entity::GeneralLivingData::new(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    )
    .expect("zombie is living");
    let entities = [stored_entity(
        9,
        99,
        [1.0, 64.0, 0.0],
        crate::entity::EntityData::GeneralLiving(zombie),
    )];
    let run = ran(
        "attribute 0-0-0-0-9 minecraft:max_health get",
        CommandTestWorld::default().with_entities(&entities),
    );
    assert_eq!(keys(&run), ["commands.attribute.value.get.success"]);
    assert_eq!(run.result, Some(20));
}

#[test]
fn attribute_get_needs_the_gamemaster_level() {
    let refusal = refused(
        "attribute alice minecraft:max_health get",
        CommandPermission::Moderator,
    );
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}

#[test]
fn attribute_base_get_reports_the_supplier_base() {
    let players = [user("alice", 1)];
    let run = ran(
        "attribute alice minecraft:max_health base get",
        world(&players),
    );
    assert_eq!(run.result, Some(20));
    assert_eq!(keys(&run), ["commands.attribute.base_value.get.success"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            translatable("attribute.name.max_health", Vec::new()),
            display_name("alice", 1),
            NbtSummaryValue::Double(20.0_f64.to_bits()),
        ]
    );
}

#[test]
fn attribute_base_get_scale_multiplies_only_the_return_value() {
    let players = [user("alice", 1)];
    let run = ran(
        "attribute alice minecraft:max_health base get 2",
        world(&players),
    );
    assert_eq!(run.result, Some(40));
    assert_eq!(
        arguments(&run, 0),
        vec![
            translatable("attribute.name.max_health", Vec::new()),
            display_name("alice", 1),
            NbtSummaryValue::Double(20.0_f64.to_bits()),
        ]
    );
}

#[test]
fn attribute_get_and_base_get_read_the_projected_player_map() {
    let players = [user("alice", 1)];
    let get = ran(
        "attribute alice minecraft:max_health get",
        world(&players).with_player_attributes("alice", |attributes| {
            attributes
                .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
                .expect("player supplies max health")
                .set_base_value(30.0);
        }),
    );
    assert_eq!(get.result, Some(30));
    assert_eq!(keys(&get), ["commands.attribute.value.get.success"]);
    assert_eq!(
        arguments(&get, 0),
        vec![
            translatable("attribute.name.max_health", Vec::new()),
            display_name("alice", 1),
            NbtSummaryValue::Double(30.0_f64.to_bits()),
        ]
    );
    let base = ran(
        "attribute alice minecraft:max_health base get",
        world(&players).with_player_attributes("alice", |attributes| {
            attributes
                .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
                .expect("player supplies max health")
                .set_base_value(30.0);
        }),
    );
    assert_eq!(base.result, Some(30));
    assert_eq!(keys(&base), ["commands.attribute.base_value.get.success"]);
    assert_eq!(
        arguments(&base, 0),
        vec![
            translatable("attribute.name.max_health", Vec::new()),
            display_name("alice", 1),
            NbtSummaryValue::Double(30.0_f64.to_bits()),
        ]
    );
}

#[test]
fn attribute_base_get_reads_a_stored_living_base_distinct_from_the_default() {
    let mut zombie = crate::entity::GeneralLivingData::new(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    )
    .expect("zombie is living");
    zombie
        .living_mut()
        .attributes_mut()
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
        .expect("zombie supplies max health")
        .set_base_value(30.0);
    let entities = [stored_entity(
        9,
        99,
        [1.0, 64.0, 0.0],
        crate::entity::EntityData::GeneralLiving(zombie),
    )];
    let mut world = CommandTestWorld::default().with_entities(&entities);
    let base = ran(
        "attribute 0-0-0-0-9 minecraft:max_health base get",
        &mut world,
    );
    assert_eq!(keys(&base), ["commands.attribute.base_value.get.success"]);
    assert_eq!(base.result, Some(30));
    let value = ran("attribute 0-0-0-0-9 minecraft:max_health get", &mut world);
    assert_eq!(value.result, Some(30));
}

#[test]
fn attribute_base_set_targets_a_stored_living_entity() {
    let zombie = crate::entity::GeneralLivingData::new(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    )
    .expect("zombie is living");
    let entities = [stored_entity(
        9,
        99,
        [1.0, 64.0, 0.0],
        crate::entity::EntityData::GeneralLiving(zombie),
    )];
    let mut world = CommandTestWorld::default().with_entities(&entities);
    let run = ran(
        "attribute 0-0-0-0-9 minecraft:max_health base set 40",
        &mut world,
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.attribute.base_value.set.success"]);
}

#[test]
fn attribute_base_set_writes_the_requested_base() {
    let players = [user("alice", 1)];
    let mut set = world(&players);
    let run = ran("attribute alice minecraft:max_health base set 30", &mut set);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.attribute.base_value.set.success"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            translatable("attribute.name.max_health", Vec::new()),
            display_name("alice", 1),
            NbtSummaryValue::Double(30.0_f64.to_bits()),
        ]
    );
    assert_eq!(
        ran("attribute alice minecraft:max_health base get", &mut set).result,
        Some(30)
    );
}

#[test]
fn attribute_base_reset_restores_the_supplier_base() {
    let players = [user("alice", 1)];
    let mut reset = world(&players).with_player_attributes("alice", |attributes| {
        attributes
            .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .expect("player supplies max health")
            .set_base_value(30.0);
    });
    let run = ran(
        "attribute alice minecraft:max_health base reset",
        &mut reset,
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.attribute.base_value.reset.success"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            translatable("attribute.name.max_health", Vec::new()),
            display_name("alice", 1),
            NbtSummaryValue::Double(20.0_f64.to_bits()),
        ]
    );
    assert_eq!(
        ran("attribute alice minecraft:max_health base get", &mut reset).result,
        Some(20)
    );
}

#[test]
fn attribute_base_set_refuses_an_attribute_the_player_supplier_lacks() {
    let players = [user("alice", 1)];
    let run = ran(
        "attribute alice minecraft:flying_speed base set 1",
        world(&players),
    );
    assert_eq!(keys(&run), ["commands.attribute.failed.no_attribute"]);
}

#[test]
fn attribute_modifier_add_writes_a_permanent_modifier() {
    let players = [user("alice", 1)];
    let mut added = world(&players);
    let run = ran(
        "attribute alice minecraft:max_health modifier add foo 4 add_value",
        &mut added,
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.attribute.modifier.add.success"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            text("minecraft:foo"),
            translatable("attribute.name.max_health", Vec::new()),
            display_name("alice", 1),
        ]
    );
    assert_eq!(
        ran(
            "attribute alice minecraft:max_health modifier value get foo",
            &mut added
        )
        .result,
        Some(4)
    );
}

#[test]
fn attribute_modifier_add_refuses_a_duplicate_id() {
    let players = [user("alice", 1)];
    let run = ran(
        "attribute alice minecraft:max_health modifier add foo 4 add_value",
        world(&players).with_player_attributes("alice", |attributes| {
            attributes
                .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
                .expect("player supplies max health")
                .add_permanent_modifier(crate::entity::AttributeModifier {
                    id: "minecraft:foo".to_string(),
                    amount: 1.0,
                    operation: crate::entity::AttributeOperation::AddValue,
                })
                .expect("first add");
        }),
    );
    assert_eq!(
        keys(&run),
        ["commands.attribute.failed.modifier_already_present"]
    );
}

#[test]
fn attribute_modifier_remove_and_value_get_read_the_live_map() {
    let players = [user("alice", 1)];
    let get = ran(
        "attribute alice minecraft:max_health modifier value get foo",
        world(&players).with_player_attributes("alice", |attributes| {
            attributes
                .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
                .expect("player supplies max health")
                .add_permanent_modifier(crate::entity::AttributeModifier {
                    id: "minecraft:foo".to_string(),
                    amount: 4.0,
                    operation: crate::entity::AttributeOperation::AddValue,
                })
                .expect("add");
        }),
    );
    assert_eq!(get.result, Some(4));
    assert_eq!(
        keys(&get),
        ["commands.attribute.modifier.value.get.success"]
    );
    let mut removing = world(&players).with_player_attributes("alice", |attributes| {
        attributes
            .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .expect("player supplies max health")
            .add_permanent_modifier(crate::entity::AttributeModifier {
                id: "minecraft:foo".to_string(),
                amount: 4.0,
                operation: crate::entity::AttributeOperation::AddValue,
            })
            .expect("add");
    });
    let remove = ran(
        "attribute alice minecraft:max_health modifier remove foo",
        &mut removing,
    );
    assert_eq!(remove.result, Some(1));
    assert_eq!(
        keys(&remove),
        ["commands.attribute.modifier.remove.success"]
    );
    assert_eq!(
        keys(&ran(
            "attribute alice minecraft:max_health modifier value get foo",
            &mut removing
        )),
        ["commands.attribute.failed.no_modifier"]
    );
}

#[test]
fn attribute_modifier_value_get_refuses_a_missing_id() {
    let players = [user("alice", 1)];
    let run = ran(
        "attribute alice minecraft:max_health modifier value get foo",
        world(&players),
    );
    assert_eq!(keys(&run), ["commands.attribute.failed.no_modifier"]);
}

#[test]
fn attribute_resource_outside_the_catalog_is_refused() {
    let players = [user("alice", 1)];
    let run = ran(
        "attribute alice minecraft:not_an_attribute get",
        world(&players),
    );
    assert_eq!(
        failure_messages(&run)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["argument.resource.not_found"]
    );
}
