use std::collections::BTreeMap;

use crate::random::GameRandom;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID;
use bbb_protocol::entity_types::{vanilla_entity_type, vanilla_entity_type_for_resource_id};
use bbb_protocol::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::Aabb;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::spawn_placement::{
    spawn_dimensions_scale, spawn_placement_by_id, SpawnPlacementProbe, SpawnPredicateReason,
};
use super::{SpawnRequest, SpawnerRefusal, SpawnerSpawnData};
use crate::block_entity::base_spawner::{field, numeric_i32};
use crate::entity::{AttributeModifier, AttributeOperation, Entity, EntityData};

const RANDOM_SPAWN_BONUS: &str = "minecraft:random_spawn_bonus";

/// `DifficultyInstance.calculateDifficulty` (`DifficultyInstance.java:43-61`),
/// which is what `getEffectiveDifficulty` (`:23-25`) answers.
pub fn effective_difficulty(
    difficulty: bbb_protocol::difficulties::Difficulty,
    total_game_time: i64,
    local_game_time: i64,
    moon_brightness: f32,
) -> f32 {
    use bbb_protocol::difficulties::Difficulty;

    if difficulty == Difficulty::Peaceful {
        return 0.0;
    }
    let difficulty_id = match difficulty {
        Difficulty::Peaceful => 0.0,
        Difficulty::Easy => 1.0,
        Difficulty::Normal => 2.0,
        Difficulty::Hard => 3.0,
    };
    let global_scale = (((total_game_time as f32 - 72_000.0) / 1_440_000.0).clamp(0.0, 1.0)) * 0.25;
    let mut local_scale = ((local_game_time as f32 / 3_600_000.0).clamp(0.0, 1.0))
        * if difficulty == Difficulty::Hard {
            1.0
        } else {
            0.75
        };
    local_scale += (moon_brightness * 0.25).clamp(0.0, global_scale);
    if difficulty == Difficulty::Easy {
        local_scale *= 0.5;
    }
    difficulty_id * (0.75 + global_scale + local_scale)
}

/// `DifficultyInstance.getSpecialMultiplier` (`DifficultyInstance.java:35-41`).
pub fn difficulty_special_multiplier(
    difficulty: bbb_protocol::difficulties::Difficulty,
    total_game_time: i64,
    local_game_time: i64,
    moon_brightness: f32,
) -> f32 {
    let effective = effective_difficulty(
        difficulty,
        total_game_time,
        local_game_time,
        moon_brightness,
    );
    if effective < 2.0 {
        0.0
    } else if effective > 4.0 {
        1.0
    } else {
        (effective - 2.0) / 2.0
    }
}

pub const SPAWNER_LEVEL_EVENT_ID: i32 = 2004;
/// `TrialSpawner.java:229`.
pub const TRIAL_SPAWNER_SPAWN_EVENT_ID: i32 = 3011;
/// `TrialSpawner.java:230`.
pub const TRIAL_SPAWNER_SPAWN_MOB_EVENT_ID: i32 = 3012;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InclusiveLightRange {
    pub min_inclusive: u8,
    pub max_inclusive: u8,
}

impl InclusiveLightRange {
    pub const FULL: Self = Self {
        min_inclusive: 0,
        max_inclusive: 15,
    };

    pub const fn contains(self, value: u8) -> bool {
        value >= self.min_inclusive && value <= self.max_inclusive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustomSpawnRules {
    pub block_light_limit: InclusiveLightRange,
    pub sky_light_limit: InclusiveLightRange,
}

impl CustomSpawnRules {
    // `SpawnData.java:53-71`.
    pub fn decode(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::Compound(entries) = value else {
            return None;
        };
        Some(Self {
            block_light_limit: decode_light_range(field(entries, "block_light_limit"))?,
            sky_light_limit: decode_light_range(field(entries, "sky_light_limit"))?,
        })
    }

    pub const fn allows(self, block_light: u8, effective_sky_light: u8) -> bool {
        self.block_light_limit.contains(block_light)
            && self.sky_light_limit.contains(effective_sky_light)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EquipmentSlotDropChance {
    pub slot: EquipmentSlot,
    pub chance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EquipmentPlan {
    pub loot_table: String,
    pub slot_drop_chances: Vec<EquipmentSlotDropChance>,
}

impl EquipmentPlan {
    // `EquipmentTable.java:13-49`.
    pub fn decode(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::Compound(entries) = value else {
            return None;
        };
        let NbtSummaryValue::String(raw_loot_table) = field(entries, "loot_table")? else {
            return None;
        };
        let loot_table =
            bbb_protocol::normalize_resource_location(raw_loot_table.to_string()).ok()?;
        let slot_drop_chances = match field(entries, "slot_drop_chances") {
            None => Vec::new(),
            Some(value) if numeric_f32(value).is_some() => {
                let chance = numeric_f32(value)?;
                VANILLA_EQUIPMENT_SLOTS
                    .iter()
                    .map(|entry| EquipmentSlotDropChance {
                        slot: entry.slot,
                        chance,
                    })
                    .collect()
            }
            Some(NbtSummaryValue::Compound(chances)) => {
                let mut parsed = BTreeMap::new();
                for chance in chances {
                    let slot = VANILLA_EQUIPMENT_SLOTS
                        .iter()
                        .find(|entry| chance.name == entry.serialized_name)
                        .map(|entry| entry.slot)?;
                    if parsed.insert(slot, numeric_f32(&chance.value)?).is_some() {
                        return None;
                    }
                }
                parsed
                    .into_iter()
                    .map(|(slot, chance)| EquipmentSlotDropChance { slot, chance })
                    .collect()
            }
            Some(_) => return None,
        };
        Some(Self {
            loot_table,
            slot_drop_chances,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnEntityTree {
    pub entity_type_id: i32,
    pub resource_id: String,
    pub entity_nbt: Vec<NbtSummaryEntry>,
    pub passengers: Vec<Self>,
}

impl SpawnEntityTree {
    // `EntityType.java:1494-1520`: a bad passenger is skipped without rejecting
    // the already-loaded root, while a bad root rejects the whole attempt.
    pub fn decode_root(entries: &[NbtSummaryEntry]) -> Option<Self> {
        Self::decode_root_with_registry(
            entries,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
    }

    pub fn decode_root_with_registry(
        entries: &[NbtSummaryEntry],
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<Self> {
        decode_entity_node(entries, content_registry)
    }

    pub fn self_and_passengers(&self) -> Box<dyn Iterator<Item = &Self> + '_> {
        Box::new(
            std::iter::once(self).chain(
                self.passengers
                    .iter()
                    .flat_map(SpawnEntityTree::self_and_passengers),
            ),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AtomicSpawnPlan {
    pub tree: SpawnEntityTree,
    pub position: Vec3d,
    pub spawn_block_position: BlockPos,
    pub spawner_position: BlockPos,
    pub collision_box: Aabb,
    pub custom_spawn_rules: Option<CustomSpawnRules>,
    pub placement_probe: Option<SpawnPlacementProbe>,
    pub finalize_root_mob: bool,
    pub equipment: Option<EquipmentPlan>,
    pub nearby_box: Aabb,
    pub max_nearby_entities: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnEffects {
    BaseSpawner,
    TrialSpawner { flame: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnPolicy {
    pub predicate_reason: SpawnPredicateReason,
    pub predicate_with_custom_rules: bool,
    pub refuse_peaceful_custom_rules: bool,
    pub max_nearby_exact_class: bool,
    pub root_mob_spawn_rules: bool,
    pub set_persistence_required: bool,
    pub line_of_sight: Option<SpawnerRefusal>,
    pub effects: SpawnEffects,
}

impl SpawnPolicy {
    pub const BASE_SPAWNER: Self = Self {
        predicate_reason: SpawnPredicateReason::Spawner,
        predicate_with_custom_rules: false,
        refuse_peaceful_custom_rules: true,
        max_nearby_exact_class: true,
        root_mob_spawn_rules: true,
        set_persistence_required: false,
        line_of_sight: None,
        effects: SpawnEffects::BaseSpawner,
    };

    #[must_use]
    pub const fn trial_spawner(flame: i32) -> Self {
        Self {
            predicate_reason: SpawnPredicateReason::TrialSpawner,
            predicate_with_custom_rules: true,
            refuse_peaceful_custom_rules: false,
            max_nearby_exact_class: false,
            root_mob_spawn_rules: false,
            set_persistence_required: true,
            // The sight gate reports the block its ray hit.
            line_of_sight: Some(SpawnerRefusal::Collision),
            effects: SpawnEffects::TrialSpawner { flame },
        }
    }
}

pub fn prepare_spawn_request(request: &SpawnRequest) -> Result<AtomicSpawnPlan, SpawnerRefusal> {
    prepare_spawn_request_with_registry(
        request,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn prepare_spawn_request_with_registry(
    request: &SpawnRequest,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<AtomicSpawnPlan, SpawnerRefusal> {
    let tree =
        SpawnEntityTree::decode_root_with_registry(request.spawn_data.entity(), content_registry)
            .ok_or(SpawnerRefusal::RecursiveEntityLoad)?;
    let (width, height) = if let Some(catalog) = vanilla_entity_type(request.entity_type_id)
        .filter(|entry| entry.resource_id == tree.resource_id)
    {
        (catalog.width, catalog.height)
    } else if let Some(spec) = content_registry
        .entity_by_id(request.entity_type_id)
        .filter(|entry| entry.resource_id == tree.resource_id)
    {
        (spec.width, spec.height)
    } else {
        return Err(SpawnerRefusal::InvalidEntityId);
    };
    let custom_spawn_rules = request
        .spawn_data
        .custom_spawn_rules()
        .map(CustomSpawnRules::decode)
        .transpose_option()
        .ok_or(SpawnerRefusal::CustomSpawnRules)?;
    let equipment = request
        .spawn_data
        .equipment()
        .map(EquipmentPlan::decode)
        .transpose_option()
        .ok_or(SpawnerRefusal::WorldAdd)?;
    let spawn_block_position = containing(request.position);
    // Whether the probe is used is the policy's call, not this one's.
    let placement_probe = Some(SpawnPlacementProbe {
        entity_type_id: request.entity_type_id,
        position: spawn_block_position,
        entry: spawn_placement_by_id(request.entity_type_id),
    });
    let scale = spawn_dimensions_scale(request.entity_type_id);
    let width = width * scale;
    let height = height * scale;
    let collision_box = Aabb::of_entity(request.position, width, height);
    let range = f64::from(request.spawn_range);
    let nearby_box = Aabb {
        min_x: f64::from(request.spawner_position.x) - range,
        min_y: f64::from(request.spawner_position.y) - range,
        min_z: f64::from(request.spawner_position.z) - range,
        max_x: f64::from(request.spawner_position.x) + 1.0 + range,
        max_y: f64::from(request.spawner_position.y) + 1.0 + range,
        max_z: f64::from(request.spawner_position.z) + 1.0 + range,
    };
    Ok(AtomicSpawnPlan {
        tree,
        position: request.position,
        spawn_block_position,
        spawner_position: request.spawner_position,
        collision_box,
        custom_spawn_rules,
        placement_probe,
        finalize_root_mob: has_only_id_configuration(&request.spawn_data),
        equipment,
        nearby_box,
        max_nearby_entities: request.max_nearby_entities,
    })
}

// `Mob.finalizeSpawn` (`Mob.java:1053-1070`). Subclass hooks run from their
// own owners; this is the shared Mob state applied to every spawner-created mob.
pub fn finalize_spawner_mob(entity: &mut Entity, random: &GameRandom) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    if !data.is_mob() {
        return false;
    }
    if let Some(follow_range) = data
        .living_mut()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
    {
        if !follow_range.has_modifier(RANDOM_SPAWN_BONUS) {
            follow_range
                .add_permanent_modifier(AttributeModifier {
                    id: RANDOM_SPAWN_BONUS.to_owned(),
                    amount: 0.11485 * (random.next_double() - random.next_double()),
                    operation: AttributeOperation::AddMultipliedBase,
                })
                .expect("the random spawn bonus id was absent");
        }
    }
    data.set_left_handed(random.next_float() < 0.05);
    true
}

/// `Mob.setPersistenceRequired` (`Mob.java:1068-1070`).
pub fn set_spawner_mob_persistence_required(entity: &mut Entity) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(mob) = data.mob.as_mut() else {
        return false;
    };
    mob.set_persistence_required(true);
    true
}

/// `Mob.dropPreservedEquipment(ServerLevel)` (`Mob.java:893-895` over
/// `:897-913`) as the entity store can serve it: every slot whose
/// `DropChances.isPreserved` (`DropChances.java:46-48`, chance `> 1.0`) holds
/// an item is emptied and handed back in `EquipmentSlot.VALUES` order, for the
/// caller to drop at the mob's own position. Its one caller is
/// `TrialSpawnerStateData.resetAfterBecomingOminous`
/// (`TrialSpawnerStateData.java:180-189`).
pub fn take_preserved_equipment(entity: &mut Entity) -> Vec<(EquipmentSlot, RuntimeItemStack)> {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return Vec::new();
    };
    let Some(mob) = data.mob.as_ref() else {
        return Vec::new();
    };
    let preserved = VANILLA_EQUIPMENT_SLOTS
        .iter()
        .map(|entry| entry.slot)
        .filter(|slot| mob.drop_chance(*slot) > 1.0)
        .collect::<Vec<_>>();
    let mut dropped = Vec::new();
    for slot in preserved {
        if let Some(stack) = data.living_mut().set_item_slot(slot, None) {
            dropped.push((slot, stack));
        }
    }
    dropped
}

/// `EquipmentUser.equip` (`EquipmentUser.java:29-69`).
pub fn equip_spawner_mob(
    entity: &mut Entity,
    stacks: Vec<RuntimeItemStack>,
    equipment: &EquipmentPlan,
) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    if data.mob.is_none() {
        return false;
    }
    let mut inserted = Vec::new();
    for stack in stacks {
        if stack.is_empty() {
            continue;
        }
        let slot = stack
            .equippable_slot()
            .ok()
            .flatten()
            .unwrap_or(EquipmentSlot::MainHand);
        if inserted.contains(&slot) {
            continue;
        }
        // `EquipmentSlot.limit` (`EquipmentSlot.java:13-20`, `:57-59`): non-hand
        // slots clamp stack count to 1.
        let equipped = if matches!(slot, EquipmentSlot::MainHand | EquipmentSlot::OffHand) {
            stack
        } else {
            stack.with_count(1)
        };
        data.living_mut().set_item_slot(slot, Some(equipped));
        if let Some(chance) = equipment
            .slot_drop_chances
            .iter()
            .find(|chance| chance.slot == slot)
        {
            if !data
                .mob
                .as_mut()
                .expect("the root mob was checked")
                .set_drop_chance(slot, chance.chance)
            {
                return false;
            }
        }
        inserted.push(slot);
    }
    true
}

trait OptionTranspose<T> {
    fn transpose_option(self) -> Option<Option<T>>;
}

impl<T> OptionTranspose<T> for Option<Option<T>> {
    fn transpose_option(self) -> Option<Option<T>> {
        match self {
            None => Some(None),
            Some(Some(value)) => Some(Some(value)),
            Some(None) => None,
        }
    }
}

fn decode_light_range(value: Option<&NbtSummaryValue>) -> Option<InclusiveLightRange> {
    let (min, max) = match value {
        None => return Some(InclusiveLightRange::FULL),
        Some(value) if numeric_i32(Some(value)).is_some() => {
            let value = numeric_i32(Some(value))?;
            (value, value)
        }
        Some(NbtSummaryValue::List(values)) if values.len() == 2 => {
            (numeric_i32(values.first())?, numeric_i32(values.get(1))?)
        }
        Some(NbtSummaryValue::Compound(entries)) => (
            numeric_i32(field(entries, "min_inclusive"))?,
            numeric_i32(field(entries, "max_inclusive"))?,
        ),
        Some(_) => return None,
    };
    if !(0..=15).contains(&min) || !(0..=15).contains(&max) || min > max {
        return None;
    }
    Some(InclusiveLightRange {
        min_inclusive: min as u8,
        max_inclusive: max as u8,
    })
}

fn numeric_f32(value: &NbtSummaryValue) -> Option<f32> {
    let parsed = match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(value) => f32::from_bits(*value),
        NbtSummaryValue::Double(value) => f64::from_bits(*value) as f32,
        _ => return None,
    };
    parsed.is_finite().then_some(parsed)
}

fn decode_entity_node(
    entries: &[NbtSummaryEntry],
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<SpawnEntityTree> {
    let NbtSummaryValue::String(raw_id) = field(entries, "id")? else {
        return None;
    };
    let resource_id = bbb_protocol::normalize_resource_location(raw_id.to_string()).ok()?;
    let entity_type_id = vanilla_entity_type_for_resource_id(&resource_id)
        .map(|entry| entry.id)
        .or_else(|| {
            content_registry
                .entity_by_resource_id(&resource_id)
                .map(|spec| spec.id)
        })?;
    let mut entity_nbt = Vec::with_capacity(entries.len());
    let mut passengers = Vec::new();
    for entry in entries {
        if entry.name != "Passengers" {
            entity_nbt.push(entry.clone());
            continue;
        }
        let NbtSummaryValue::List(values) = &entry.value else {
            continue;
        };
        for value in values {
            let NbtSummaryValue::Compound(passenger) = value else {
                continue;
            };
            if let Some(passenger) = decode_entity_node(passenger, content_registry) {
                passengers.push(passenger);
            }
        }
    }
    Some(SpawnEntityTree {
        entity_type_id,
        resource_id,
        entity_nbt,
        passengers,
    })
}

fn has_only_id_configuration(data: &SpawnerSpawnData) -> bool {
    data.entity().len() == 1
        && matches!(field(data.entity(), "id"), Some(NbtSummaryValue::String(_)))
}

fn containing(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    #[test]
    fn custom_light_ranges_default_and_gate() {
        let rules = CustomSpawnRules::decode(&NbtSummaryValue::Compound(vec![
            entry("block_light_limit", NbtSummaryValue::Int(7)),
            entry(
                "sky_light_limit",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(2), NbtSummaryValue::Int(12)]),
            ),
        ]))
        .unwrap();
        assert_eq!(rules.block_light_limit.min_inclusive, 7);
        assert!(rules.allows(7, 12));
        assert!(!rules.allows(6, 12));
    }

    #[test]
    fn recursive_tree_preserves_legal_passengers() {
        let root = vec![
            entry("id", NbtSummaryValue::String("minecraft:pig".into())),
            entry(
                "Passengers",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    entry("id", NbtSummaryValue::String("minecraft:chicken".into())),
                    entry(
                        "Passengers",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![entry(
                            "id",
                            NbtSummaryValue::String("minecraft:cow".into()),
                        )])]),
                    ),
                ])]),
            ),
        ];
        let tree = SpawnEntityTree::decode_root(&root).unwrap();
        assert_eq!(tree.resource_id, "minecraft:pig");
        assert_eq!(tree.passengers.len(), 1);
        assert_eq!(tree.passengers[0].resource_id, "minecraft:chicken");
        assert_eq!(tree.passengers[0].passengers.len(), 1);
        assert_eq!(
            tree.passengers[0].passengers[0].resource_id,
            "minecraft:cow"
        );
        assert!(tree
            .entity_nbt
            .iter()
            .all(|entry| entry.name != "Passengers"));
    }
}

#[test]
fn difficulty_special_multiplier_matches_vanilla_clamps() {
    use bbb_protocol::difficulties::Difficulty;

    assert_eq!(
        difficulty_special_multiplier(Difficulty::Peaceful, i64::MAX, i64::MAX, 1.0),
        0.0
    );
    assert_eq!(
        difficulty_special_multiplier(Difficulty::Easy, 0, 0, 1.0),
        0.0
    );
    assert_eq!(
        difficulty_special_multiplier(Difficulty::Normal, 72_000, 0, 1.0),
        0.0
    );
    assert_eq!(
        difficulty_special_multiplier(Difficulty::Hard, 1_512_000, 3_600_000, 1.0),
        1.0
    );
}
