use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_synched_data::VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
    VANILLA_ENTITY_TYPE_WIND_CHARGE_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use super::{synched_values, WitherRetainedState};
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::{tick_one_entity, EntityData, EntityIdCounter, EntityStore};

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

fn default_record() -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0x145, EntityIdCounter::new());
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WITHER_ID, position())
        .expect("Wither is GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

fn chunk_document(entries: Vec<NbtSummaryEntry>) -> NamedCompound {
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
    )
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(chunk_document(entries), 0, 0).unwrap()
}

fn set_field(entries: &mut [NbtSummaryEntry], name: &str, value: NbtSummaryValue) {
    entries
        .iter_mut()
        .find(|field| field.name == name)
        .expect("writer emits owned Wither field")
        .value = value;
}

fn restored_store(entries: Vec<NbtSummaryEntry>, seed: i64) -> (EntityStore, i32) {
    let mut store = EntityStore::new(seed, EntityIdCounter::new());
    let id = read_entity_chunk(&mut store, &chunk(entries)).unwrap()[0];
    (store, id)
}

fn wither_state(store: &EntityStore, id: i32) -> WitherRetainedState {
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!("Wither uses GeneralLiving")
    };
    data.wither_retained().unwrap()
}

#[test]
fn wither_runtime_census_save_restore_and_tracking_are_exact() {
    let owned = VANILLA_ENTITY_TYPES
        .iter()
        .filter_map(|facts| {
            crate::entity::GeneralLivingData::new(facts.id)
                .and_then(|data| data.wither_retained())
                .map(|_| facts.id)
        })
        .collect::<Vec<_>>();
    assert_eq!(owned, vec![VANILLA_ENTITY_TYPE_WITHER_ID]);

    let record = default_record();
    assert_eq!(
        record
            .iter()
            .filter(|field| field.name == "Invul")
            .map(|field| &field.value)
            .collect::<Vec<_>>(),
        vec![&NbtSummaryValue::Int(0)]
    );
    let metadata = synched_values(WitherRetainedState::from_saved(0));
    assert_eq!(
        metadata.map(|row| (row.value.data_id, row.is_default)),
        [(16, true), (17, true), (18, true), (19, true)]
    );

    let mut missing = record.clone();
    missing.retain(|field| field.name != "Invul");
    let (missing, id) = restored_store(missing, 0x146);
    assert_eq!(wither_state(&missing, id).invulnerable_ticks(), 0);

    let mut active = record.clone();
    set_field(&mut active, "Invul", NbtSummaryValue::Int(17));
    let (active, id) = restored_store(active, 0x147);
    assert_eq!(wither_state(&active, id).invulnerable_ticks(), 17);
    assert!(
        crate::entity::tracking::non_default_synched_values(active.get(id).unwrap())
            .iter()
            .any(|row| row.data_id == VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID)
    );

    let mut negative = record.clone();
    set_field(&mut negative, "Invul", NbtSummaryValue::Int(-1));
    let (negative, id) = restored_store(negative, 0x14A);
    assert_eq!(wither_state(&negative, id).invulnerable_ticks(), -1);
}

#[test]
fn wither_despawns_in_peaceful_and_completes_invulnerability_with_explosion_and_event() {
    let mut peaceful = EntityStore::new(0x150, EntityIdCounter::new());
    let id = peaceful
        .spawn_general_living(VANILLA_ENTITY_TYPE_WITHER_ID, position())
        .unwrap();
    peaceful.set_difficulty(Difficulty::Peaceful);
    NativeLevel::new().with_level(|level| {
        tick_one_entity(&mut peaceful, level, id);
    });
    assert!(peaceful.get(id).is_none());

    let mut invulnerable = default_record();
    set_field(&mut invulnerable, "Invul", NbtSummaryValue::Int(1));
    let (mut invulnerable, id) = restored_store(invulnerable, 0x151);
    let mut level = NativeLevel::new();
    level.with_level(|level| {
        tick_one_entity(&mut invulnerable, level, id);
    });
    assert_eq!(wither_state(&invulnerable, id).invulnerable_ticks(), 0);
    assert_eq!(invulnerable.drain_explosions().len(), 1);
    assert!(invulnerable
        .get_mut(id)
        .unwrap()
        .take_synched_dirty()
        .contains(&VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID));
}

#[test]
fn powered_wither_fixed_projectile_gate_excludes_breeze_wind_charge_sibling() {
    let mut store = EntityStore::new(0x152, EntityIdCounter::new());
    let wither = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WITHER_ID, position())
        .unwrap();
    let max_health = store.get(wither).unwrap().living().unwrap().max_health();
    store
        .get_mut(wither)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(max_health / 2.0);
    let wind = store.spawn(
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        position(),
        EntityData::WindCharge(crate::entity::WindChargeData::new(
            crate::entity::hurting_projectile::HurtingProjectileState::new(None, 0.0),
            0,
        )),
    );
    let breeze = store.spawn(
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
        position(),
        EntityData::BreezeWindCharge(crate::entity::WindChargeData::new(
            crate::entity::hurting_projectile::HurtingProjectileState::new(None, 0.0),
            0,
        )),
    );
    let attack = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|kind| kind.resource_id == "minecraft:generic")
        .unwrap();
    let mut native = NativeLevel::new();
    let mut probe_wind = store.get(wither).unwrap().clone();
    let wind_effects = native.with_level(|level| {
        crate::entity::general_living::hurt_server_with_runtime_in_level(
            &mut probe_wind,
            &mut store,
            level,
            crate::entity::damage::StoredEntityDamageSource::without_attacker_views(
                attack,
                Some(wind),
                None,
            ),
            1.0,
            crate::entity::damage::StoredEntityDamageRules::default(),
        )
    });
    assert!(!wind_effects.outcome.succeeded);

    let mut probe_breeze = store.get(wither).unwrap().clone();
    let breeze_effects = native.with_level(|level| {
        crate::entity::general_living::hurt_server_with_runtime_in_level(
            &mut probe_breeze,
            &mut store,
            level,
            crate::entity::damage::StoredEntityDamageSource::without_attacker_views(
                attack,
                Some(breeze),
                None,
            ),
            1.0,
            crate::entity::damage::StoredEntityDamageRules::default(),
        )
    });
    assert!(breeze_effects.outcome.succeeded);
}

const AIR_ORIGIN: Vec3d = Vec3d {
    x: 8.5,
    y: 100.0,
    z: 8.5,
};

fn live_fixture(seed: i64) -> (NativeLevel, EntityStore, i32) {
    let mut native = NativeLevel::new();
    for x in -1..=1 {
        for z in -1..=1 {
            native
                .world
                .load_entity_chunk_for_test(crate::entity::tests::native_level::DIMENSION, (x, z))
                .unwrap();
        }
    }
    let mut store = EntityStore::new(seed, EntityIdCounter::new());
    store.set_difficulty(Difficulty::Normal);
    let wither = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WITHER_ID, AIR_ORIGIN)
        .unwrap();
    (native, store, wither)
}

fn skulls_owned_by(store: &EntityStore, owner: uuid::Uuid) -> usize {
    store
        .ids()
        .into_iter()
        .filter(|&id| {
            matches!(store.get(id).unwrap().data(), EntityData::WitherSkull(data)
                if data.projectile().owner() == Some(owner))
        })
        .count()
}

#[test]
fn wither_heads_lock_on_to_a_nearby_mob_and_shoot_skulls_at_it() {
    let (mut native, mut store, wither) = live_fixture(0x160);
    let pig = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
            Vec3d {
                x: AIR_ORIGIN.x + 6.0,
                ..AIR_ORIGIN
            },
        )
        .unwrap();
    if let EntityData::GeneralLiving(data) = &mut store.get_mut(pig).unwrap().data {
        data.set_no_ai(true);
    }
    store.get_mut(pig).unwrap().set_no_gravity(true);
    let owner = store.get(wither).unwrap().uuid();
    for _ in 0..60 {
        native.with_level(|level| {
            tick_one_entity(&mut store, level, wither);
            tick_one_entity(&mut store, level, pig);
        });
    }
    let targets = wither_state(&store, wither).targets();
    assert!(targets.contains(&pig), "{targets:?}");
    assert!(skulls_owned_by(&store, owner) > 0);
}

#[test]
fn hurt_wither_breaks_surrounding_blocks_except_wither_immune_ones() {
    let (mut native, mut store, wither) = live_fixture(0x161);
    let stone = crate::entity::tests::native_level::state("minecraft:stone");
    let bedrock = crate::entity::tests::native_level::state("minecraft:bedrock");
    let origin = bbb_protocol::packets::BlockPos { x: 8, y: 100, z: 8 };
    let mut cells = Vec::new();
    for x in -1..=1 {
        for y in 0..=3 {
            for z in -1..=1 {
                cells.push(bbb_protocol::packets::BlockPos {
                    x: origin.x + x,
                    y: origin.y + y,
                    z: origin.z + z,
                });
            }
        }
    }
    for &pos in &cells {
        native.set(pos, stone);
    }
    let immune = cells[0];
    native.set(immune, bedrock);

    let attack = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|kind| kind.resource_id == "minecraft:generic")
        .unwrap();
    let mut entity = store.detach(wither).unwrap();
    let effects = native.with_level(|level| {
        crate::entity::general_living::hurt_server_with_runtime_in_level(
            &mut entity,
            &mut store,
            level,
            crate::entity::damage::StoredEntityDamageSource::without_attacker_views(
                attack, None, None,
            ),
            1.0,
            crate::entity::damage::StoredEntityDamageRules::default(),
        )
    });
    assert!(effects.outcome.succeeded);
    store.attach(entity);
    for _ in 0..20 {
        native.with_level(|level| {
            tick_one_entity(&mut store, level, wither);
        });
    }
    native.with_level(|level| {
        assert_eq!(level.block_state(immune), bedrock);
        let left = cells
            .iter()
            .filter(|&&pos| level.block_state(pos) == stone)
            .count();
        // The wither drifts while it hovers, so only part of the shell is in reach.
        assert!(left < cells.len() - 1, "{left}");
    });
}
