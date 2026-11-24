use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_ON_FIRE, VANILLA_SLIME_SIZE_DATA_ID,
};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, VANILLA_ENTITY_TYPE_SLIME_ID};
use bbb_protocol::packets::{BlockPos, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use crate::entity::tests::native_level::{state, NativeLevel};
use crate::registry_tags::RegistryTagSnapshot;

use super::{fluid_contact, set_size, synched_values, tick_jump, SlimeFluidContact};
use crate::entity::living::ActiveMobEffect;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::{tick_entities, EntityData, EntityIdCounter, EntityStore, RemovalReason};

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(
        NamedCompound::new(
            "",
            vec![
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                entry(
                    "Entities",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entries)]),
                ),
            ],
        ),
        0,
        0,
    )
    .unwrap()
}

fn default_record(entity_type_id: i32) -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    store
        .spawn_general_living(entity_type_id, position())
        .unwrap();
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

#[test]
fn slime_autonomous_runtime_census_size_save_tracking_and_attributes_are_exact() {
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    assert!(set_size(entity, 4, true));
    assert_eq!(entity.bb_width(), 2.08);
    assert_eq!(entity.living().unwrap().max_health(), 16.0);
    assert_eq!(
        entity
            .living()
            .unwrap()
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,),
        f64::from(0.2_f32 + 0.1_f32 * 4.0)
    );
    assert_eq!(
        entity
            .living()
            .unwrap()
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ARMOR_ID,),
        12.0
    );
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_SLIME_SIZE_DATA_ID]
    );
    let state = match entity.data() {
        EntityData::GeneralLiving(data) => data.slime_retained().unwrap(),
        _ => unreachable!(),
    };
    let tracked = synched_values(state)[0].clone();
    assert_eq!(tracked.value.value, EntityDataValueKind::Int(4));
    assert!(!tracked.is_default);
}

#[test]
fn slime_autonomous_runtime_restore_clamps() {
    let mut record = default_record(VANILLA_ENTITY_TYPE_SLIME_ID);
    record
        .iter_mut()
        .find(|entry| entry.name == "Size")
        .unwrap()
        .value = NbtSummaryValue::Int(126);
    record
        .iter_mut()
        .find(|entry| entry.name == "wasOnGround")
        .unwrap()
        .value = NbtSummaryValue::Byte(1);
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let id = read_entity_chunk(&mut store, &chunk(record)).unwrap()[0];
    let entity = store.get(id).unwrap();
    assert_eq!(entity.bb_width(), 0.52 * 127.0);
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert_eq!(data.slime_retained().unwrap().size(), 127);
    assert!(data.slime_retained().unwrap().was_on_ground());
}

#[test]
fn slime_autonomous_runtime_targetless_hop_and_no_ai_gate() {
    let mut native = NativeLevel::new();
    let mut store = EntityStore::new(5, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_SLIME_ID, position())
        .unwrap();
    store.get_mut(id).unwrap().on_ground = true;
    native.with_level(|level| tick_entities(&mut store, level, true));
    let entity = store.get(id).unwrap();
    assert!(entity.delta_movement().y > 0.0);
    assert!(entity.needs_sync());

    let mut record = default_record(VANILLA_ENTITY_TYPE_SLIME_ID);
    record
        .iter_mut()
        .find(|entry| entry.name == "OnGround")
        .unwrap()
        .value = NbtSummaryValue::Byte(1);
    record.push(entry("NoAI", NbtSummaryValue::Byte(1)));
    let mut no_ai = EntityStore::new(6, EntityIdCounter::new());
    let id = read_entity_chunk(&mut no_ai, &chunk(record)).unwrap()[0];
    native.with_level(|level| tick_entities(&mut no_ai, level, true));
    assert!(no_ai.get(id).unwrap().delta_movement().y <= 0.0);
}

#[test]
fn overlapping_live_water_and_lava_tags_take_the_water_jump_branch() {
    let mut store = EntityStore::new(0x51_1A, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, position())
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    entity.living_mut().unwrap().jumping = true;
    let mut native = NativeLevel::new();
    native.set(BlockPos { x: 0, y: 64, z: 0 }, state("minecraft:water"));
    native.install_tags(RegistryTagSnapshot::new([
        (
            "minecraft:fluid".into(),
            "minecraft:water".into(),
            vec!["minecraft:water".into()],
        ),
        (
            "minecraft:fluid".into(),
            "minecraft:lava".into(),
            vec!["minecraft:water".into()],
        ),
    ]));
    native.with_level(|level| {
        let contact = fluid_contact(&entity, level).unwrap();
        assert!(matches!(contact, SlimeFluidContact::Water(_)));
        assert!(tick_jump(&mut entity, level, contact));
    });
    assert_eq!(entity.delta_movement().y, f64::from(0.04_f32));
}

#[test]
fn slime_death_split_preserves_children_and_removes_parent() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_SLIME_ID, position())
        .unwrap();
    let source_width = {
        let entity = store.get_mut(id).unwrap();
        assert!(set_size(entity, 4, true));
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!()
        };
        data.set_no_ai(true);
        data.set_left_handed(true);
        let mob = data.mob.as_mut().unwrap();
        mob.set_persistence_required(true);
        mob.set_can_pick_up_loot(true);
        let living = entity.living_mut().unwrap();
        assert!(
            living.add_active_effect(ActiveMobEffect::new("minecraft:levitation", 200, 1).unwrap())
        );
        living.set_health(0.0);
        entity.set_remaining_fire_ticks(20);
        entity.set_shared_flag_on_fire(true);
        entity.set_invulnerable(true);
        entity.set_no_gravity(true);
        entity.portal_cooldown = 42;
        entity.set_silent(true);
        entity.set_custom_name_visible(true);
        entity.set_custom_name(Some(RichTextComponent::plain("split slime")));
        entity.set_delta_movement(Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        });
        entity.bb_width()
    };
    store.drain_lifecycle_events();
    let mut source = store.detach(id).unwrap();
    source.set_removed(RemovalReason::Killed);
    store.attach(source);

    let count = store.len();
    assert!((2..=4).contains(&count));
    assert!(store.get(id).is_none());
    let mut positions = Vec::new();
    for child_id in store.ids() {
        let child = store.get(child_id).unwrap();
        positions.push((child.position().x, child.position().y, child.position().z));
        assert_eq!(child.entity_type_id(), VANILLA_ENTITY_TYPE_SLIME_ID);
        assert_eq!(
            (child.position().x - position().x).abs(),
            f64::from(source_width / 4.0)
        );
        assert_eq!(
            (child.position().z - position().z).abs(),
            f64::from(source_width / 4.0)
        );
        assert_eq!(child.position().y, position().y + 0.5);
        assert!((0.0..360.0).contains(&child.y_rot()));
        assert_eq!(child.x_rot(), 0.0);
        assert_eq!(child.delta_movement(), Vec3d::default());
        assert_eq!(child.bb_width(), 0.52 * 2.0);
        assert_eq!(child.living().unwrap().health(), 4.0);
        assert_eq!(
            child
                .living()
                .unwrap()
                .active_effect("minecraft:levitation")
                .unwrap()
                .duration(),
            200
        );
        let EntityData::GeneralLiving(data) = child.data() else {
            unreachable!()
        };
        assert_eq!(data.slime_retained().unwrap().size(), 2);
        assert!(data.is_no_ai());
        assert!(data.is_left_handed());
        let mob = data.mob_retained().unwrap();
        assert!(mob.persistence_required());
        assert!(!mob.can_pick_up_loot());
        assert!(child.custom_name_visible());
        assert_eq!(
            child.custom_name(),
            Some(&RichTextComponent::plain("split slime"))
        );
        assert!(child.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE));
        assert!(child.invulnerable());
        assert!(child.no_gravity());
        assert_eq!(child.portal_cooldown(), 42);
        assert!(child.is_silent());
    }
    positions.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then(a.2.total_cmp(&b.2))
    });
    positions.dedup();
    assert_eq!(positions.len(), count);
}

#[test]
fn slime_death_split_preserves_magma_subclass_and_uses_health_for_removal() {
    let mut magma = EntityStore::new(8, EntityIdCounter::new());
    let magma_id = magma
        .spawn_general_living(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, position())
        .unwrap();
    let entity = magma.get_mut(magma_id).unwrap();
    assert!(set_size(entity, 4, true));
    entity.living_mut().unwrap().set_health(0.0);
    magma.drain_lifecycle_events();
    magma.remove(magma_id, RemovalReason::Killed).unwrap();
    assert!(matches!(magma.len(), 2..=4));
    for id in magma.ids() {
        let child = magma.get(id).unwrap();
        assert_eq!(child.entity_type_id(), VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID);
        let EntityData::GeneralLiving(data) = child.data() else {
            unreachable!()
        };
        assert_eq!(data.slime_retained().unwrap().size(), 2);
    }

    for (seed, size, dead, reason) in [
        (9, 1, true, RemovalReason::Killed),
        (10, 4, false, RemovalReason::Killed),
    ] {
        let mut store = EntityStore::new(seed, EntityIdCounter::new());
        let id = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_SLIME_ID, position())
            .unwrap();
        let entity = store.get_mut(id).unwrap();
        assert!(set_size(entity, size, true));
        if dead {
            entity.living_mut().unwrap().set_health(0.0);
        }
        store.drain_lifecycle_events();
        store.remove(id, reason).unwrap();
        assert!(store.is_empty());
    }

    let mut discarded = EntityStore::new(11, EntityIdCounter::new());
    let discarded_id = discarded
        .spawn_general_living(VANILLA_ENTITY_TYPE_SLIME_ID, position())
        .unwrap();
    let entity = discarded.get_mut(discarded_id).unwrap();
    assert!(set_size(entity, 4, true));
    entity.living_mut().unwrap().set_health(0.0);
    discarded.drain_lifecycle_events();
    discarded
        .remove(discarded_id, RemovalReason::Discarded)
        .unwrap();
    assert!(matches!(discarded.len(), 2..=4));
    assert!(discarded.get(discarded_id).is_none());
}
