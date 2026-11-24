use crate::random::GameRandom;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_PANDA_EAT_COUNTER_DATA_ID, VANILLA_PANDA_FLAGS_DATA_ID,
    VANILLA_PANDA_HIDDEN_GENE_DATA_ID, VANILLA_PANDA_MAIN_GENE_DATA_ID,
    VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID, VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID,
};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, SoundSource, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::{mth_cos, mth_sin};

use crate::entity::living::SynchedValue;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

const FLAG_SNEEZE: i8 = 2;
const FLAG_ROLL: i8 = 4;
const FLAG_SIT: i8 = 8;
const FLAG_ON_BACK: i8 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub(crate) enum PandaGene {
    Normal,
    Lazy,
    Worried,
    Playful,
    Brown,
    Weak,
    Aggressive,
}

impl PandaGene {
    const fn serialized_name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Lazy => "lazy",
            Self::Worried => "worried",
            Self::Playful => "playful",
            Self::Brown => "brown",
            Self::Weak => "weak",
            Self::Aggressive => "aggressive",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "normal" => Self::Normal,
            "lazy" => Self::Lazy,
            "worried" => Self::Worried,
            "playful" => Self::Playful,
            "brown" => Self::Brown,
            "weak" => Self::Weak,
            "aggressive" => Self::Aggressive,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PandaRetainedState {
    unhappy_counter: i32,
    sneeze_counter: i32,
    eat_counter: i32,
    main_gene: PandaGene,
    hidden_gene: PandaGene,
    flags: i8,
    roll_counter: i32,
    roll_delta: Vec3d,
    retaliation_satisfied: bool,
}

impl PandaRetainedState {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.unhappy_counter = previous.unhappy_counter;
        self.sneeze_counter = previous.sneeze_counter;
        self.eat_counter = previous.eat_counter;
        self.flags = previous.flags;
        self.roll_counter = previous.roll_counter;
        self.roll_delta = previous.roll_delta;
        self.retaliation_satisfied = previous.retaliation_satisfied;
    }

    fn from_saved(main_gene: PandaGene, hidden_gene: PandaGene) -> Self {
        Self {
            main_gene,
            hidden_gene,
            ..Self::default()
        }
    }

    fn has_flag(&self, flag: i8) -> bool {
        self.flags & flag != 0
    }

    pub(crate) fn is_worried(&self) -> bool {
        self.main_gene == PandaGene::Worried
    }

    pub(crate) fn is_lazy(&self) -> bool {
        self.main_gene == PandaGene::Lazy
    }

    pub(crate) fn is_weak(&self) -> bool {
        self.main_gene == PandaGene::Weak && self.hidden_gene == PandaGene::Weak
    }

    pub(crate) fn is_unhappy(&self) -> bool {
        self.unhappy_counter > 0
    }

    pub(crate) fn is_playful(&self) -> bool {
        self.main_gene == PandaGene::Playful
    }

    pub(crate) fn is_aggressive(&self) -> bool {
        self.main_gene == PandaGene::Aggressive
    }

    pub(crate) fn retaliation_satisfied(&self) -> bool {
        self.retaliation_satisfied
    }

    pub(crate) fn is_sitting(&self) -> bool {
        self.has_flag(FLAG_SIT)
    }

    // Panda.java:391,689-691.
    pub(crate) fn can_perform_action(&self, thundering: bool) -> bool {
        !self.has_flag(FLAG_ON_BACK)
            && !(self.is_worried() && thundering)
            && self.eat_counter <= 0
            && !self.has_flag(FLAG_ROLL)
            && !self.is_sitting()
    }

    fn set_flag(&mut self, flag: i8, value: bool) -> bool {
        let next = if value {
            self.flags | flag
        } else {
            self.flags & !flag
        };
        let changed = self.flags != next;
        self.flags = next;
        changed
    }

    #[cfg(test)]
    pub(crate) fn set_counters_and_flags(
        &mut self,
        unhappy: i32,
        sneeze: i32,
        eat: i32,
        flags: i8,
    ) {
        self.unhappy_counter = unhappy;
        self.sneeze_counter = sneeze;
        self.eat_counter = eat;
        self.flags = flags;
    }
}

impl Default for PandaRetainedState {
    fn default() -> Self {
        Self {
            unhappy_counter: 0,
            sneeze_counter: 0,
            eat_counter: 0,
            main_gene: PandaGene::Normal,
            hidden_gene: PandaGene::Normal,
            flags: 0,
            roll_counter: 0,
            retaliation_satisfied: false,
            roll_delta: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PandaGenes {
    main: PandaGene,
    hidden: PandaGene,
}

pub(crate) fn offspring_genes(
    first: &Entity,
    second: &Entity,
    random: &GameRandom,
) -> Option<PandaGenes> {
    let pick = |entity: &Entity| {
        let EntityData::GeneralLiving(data) = entity.data() else {
            return None;
        };
        let state = data.panda_retained()?;
        Some(if random.next_boolean() {
            state.main_gene
        } else {
            state.hidden_gene
        })
    };
    // Panda.java:561-594: one allele from each parent, with independent mutations.
    let mut genes = PandaGenes {
        main: pick(first)?,
        hidden: pick(second)?,
    };
    if random.next_boolean() {
        std::mem::swap(&mut genes.main, &mut genes.hidden);
    }
    if random.next_int(32) == 0 {
        genes.main = random_gene(random.next_int(16) as u32);
    }
    if random.next_int(32) == 0 {
        genes.hidden = random_gene(random.next_int(16) as u32);
    }
    Some(genes)
}

pub(crate) fn finalize_spawn_genes(entity: &mut Entity, random: &GameRandom) -> bool {
    apply_spawn_genes(
        entity,
        PandaGenes {
            main: random_gene(random.next_int(16) as u32),
            hidden: random_gene(random.next_int(16) as u32),
        },
    )
}

pub(crate) fn apply_spawn_genes(entity: &mut Entity, genes: PandaGenes) -> bool {
    let Some((main_changed, hidden_changed)) = (|| {
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            return None;
        };
        let state = data.panda_retained_mut()?;
        let main_gene = genes.main;
        let hidden_gene = genes.hidden;
        let old_main = state.main_gene;
        let old_hidden = state.hidden_gene;
        state.main_gene = genes.main;
        state.hidden_gene = genes.hidden;
        let phenotype = if matches!(main_gene, PandaGene::Brown | PandaGene::Weak) {
            (main_gene == hidden_gene).then_some(main_gene)
        } else {
            Some(main_gene)
        };
        // Panda.java:591-599: recessive weakness and dominant laziness affect attributes.
        let living = data.living_mut();
        if phenotype == Some(PandaGene::Weak) {
            living
                .attributes_mut()
                .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
                .expect("Panda has max health")
                .set_base_value(10.0);
        }
        if phenotype == Some(PandaGene::Lazy) {
            living
                .attributes_mut()
                .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
                .expect("Panda has movement speed")
                .set_base_value(0.07_f32.into());
        }
        Some((old_main != main_gene, old_hidden != hidden_gene))
    })() else {
        return false;
    };
    if main_changed {
        entity.mark_synched_dirty(VANILLA_PANDA_MAIN_GENE_DATA_ID);
    }
    if hidden_changed {
        entity.mark_synched_dirty(VANILLA_PANDA_HIDDEN_GENE_DATA_ID);
    }
    true
}

pub(crate) fn unhappy_counter(entity: &Entity) -> Option<i32> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    Some(data.panda_retained()?.unhappy_counter)
}

pub(crate) fn set_breeding_unhappy(entity: &mut Entity) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.panda_retained_mut() else {
        return;
    };
    state.unhappy_counter = 32;
    entity.mark_synched_dirty(VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID);
}

const fn random_gene(value: u32) -> PandaGene {
    match value {
        0 => PandaGene::Lazy,
        1 => PandaGene::Worried,
        2 => PandaGene::Playful,
        4 => PandaGene::Aggressive,
        3 | 5..=8 => PandaGene::Weak,
        9..=10 => PandaGene::Brown,
        _ => PandaGene::Normal,
    }
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: &PandaRetainedState) {
    // `Panda.addAdditionalSaveData` (`Panda.java:234-245`).
    entries.extend([
        NbtSummaryEntry {
            name: "MainGene".into(),
            value: NbtSummaryValue::string(state.main_gene.serialized_name()),
        },
        NbtSummaryEntry {
            name: "HiddenGene".into(),
            value: NbtSummaryValue::string(state.hidden_gene.serialized_name()),
        },
    ]);
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> PandaRetainedState {
    let main = read_gene(entries, "MainGene");
    let hidden = read_gene(entries, "HiddenGene");
    PandaRetainedState::from_saved(main, hidden)
}

// Panda.java:243-244: an undecodable gene reads as NORMAL.
fn read_gene(entries: &[NbtSummaryEntry], name: &str) -> PandaGene {
    match crate::entity::save::optional_field(entries, name) {
        Some(NbtSummaryValue::String(value)) => value
            .with_utf8(PandaGene::parse)
            .flatten()
            .unwrap_or(PandaGene::Normal),
        _ => PandaGene::Normal,
    }
}

pub(crate) fn synched_values(state: &PandaRetainedState) -> [SynchedValue; 6] {
    // `Panda.defineSynchedData` (`Panda.java:210-218`).
    [
        int_value(VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID, state.unhappy_counter),
        int_value(VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID, state.sneeze_counter),
        int_value(VANILLA_PANDA_EAT_COUNTER_DATA_ID, state.eat_counter),
        byte_value(VANILLA_PANDA_MAIN_GENE_DATA_ID, state.main_gene as i8),
        byte_value(VANILLA_PANDA_HIDDEN_GENE_DATA_ID, state.hidden_gene as i8),
        byte_value(VANILLA_PANDA_FLAGS_DATA_ID, state.flags),
    ]
}

fn int_value(data_id: u8, value: i32) -> SynchedValue {
    SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(value),
        },
        is_default: value == 0,
    }
}

fn byte_value(data_id: u8, value: i8) -> SynchedValue {
    SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(value),
        },
        is_default: value == 0,
    }
}

pub(crate) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut crate::entity::leash::LeashInteractingPlayer<'_>,
    tags: &crate::entity::level::EntityRegistryTagView,
    thundering: bool,
    growth: Option<&crate::entity::living::dimensions_growth::SizeGrowthCollisionView<'_>>,
) -> Option<crate::entity::leash::LeashInteraction> {
    use crate::entity::animal_feeding::{self, FeedingPlayer};
    use crate::entity::leash::LeashInteraction;
    use bbb_protocol::packets::EquipmentSlot;
    let target = store.get(target_id)?;
    let EntityData::GeneralLiving(data) = target.data() else {
        return None;
    };
    let state = data.panda_retained()?;
    // `Panda.mobInteract` (`Panda.java:610-657`).
    if state.main_gene == PandaGene::Worried && thundering {
        return Some(LeashInteraction::Pass);
    }
    if state.has_flag(FLAG_ON_BACK) {
        let target = store.get_mut(target_id)?;
        set_on_back(target, false);
        return Some(LeashInteraction::PandaStoodUp);
    }
    let food = player
        .hand_stack
        .item_id
        .and_then(bbb_protocol::item_types::vanilla_item_type)
        .filter(|item| tags.contains("minecraft:item", "minecraft:panda_food", item.resource_id));
    let Some(food) = food else {
        let outcome = animal_feeding::super_mob_interact(
            store,
            target_id,
            &mut FeedingPlayer {
                hand_stack: player.hand_stack,
                uuid: player.uuid,
                infinite_materials: player.infinite_materials,
            },
            tags,
        );
        return Some(match outcome {
            animal_feeding::AnimalFeeding::AgeLockToggled => LeashInteraction::AgeLockToggled,
            _ => LeashInteraction::Pass,
        });
    };
    let ageable = data.ageable_retained()?;
    let grows = ageable.can_age_up();
    let age = ageable.age();
    let loves = age == 0 && crate::entity::animal_breeding::can_fall_in_love(target);
    let refuses_food = !grows
        && (ageable.is_baby() || (!loves && (state.has_flag(FLAG_SIT) || target.is_in_water())));
    // Panda.java:621-624 pacifies before the age, love and sitting branches.
    if store
        .mob_ai(target_id)
        .is_some_and(|ai| ai.target_ref().is_some())
    {
        if let Some(target) = store.get_mut(target_id) {
            if let EntityData::GeneralLiving(data) = &mut target.data {
                data.panda_retained_mut()?.retaliation_satisfied = true;
            }
        }
    }
    if refuses_food {
        return Some(LeashInteraction::Pass);
    }
    let mut target = store.detach(target_id)?;
    if grows {
        if !animal_feeding::age_up(
            &mut target,
            animal_feeding::speed_up_seconds_when_feeding(age.wrapping_neg()),
            true,
            growth,
        ) {
            store.attach(target);
            return Some(LeashInteraction::Refused);
        }
    } else if loves {
        crate::entity::animal_breeding::set_in_love(&mut target, store, Some(player.uuid));
    } else {
        if let EntityData::GeneralLiving(data) = &mut target.data {
            data.living_mut().zza = 0.0;
            let state = data.panda_retained_mut().expect("Panda state");
            state.set_flag(FLAG_SIT, true);
            state.eat_counter = 1;
        }
        store.stop_mob_navigation(target_id);
        let old = target
            .living_mut()
            .expect("Panda living state")
            .set_item_slot(
                EquipmentSlot::MainHand,
                crate::entity::item::plain_stack(food.resource_id, 1),
            );
        if !player.infinite_materials {
            if let Some(old) = old {
                crate::entity::item::spawn_dropped_item_entity(store, target.position(), old);
            }
        }
        target.mark_synched_dirty(VANILLA_PANDA_FLAGS_DATA_ID);
        target.mark_synched_dirty(VANILLA_PANDA_EAT_COUNTER_DATA_ID);
    }
    animal_feeding::use_mob_player_item(
        &mut FeedingPlayer {
            hand_stack: player.hand_stack,
            uuid: player.uuid,
            infinite_materials: player.infinite_materials,
        },
        &mut player.mob_item_used,
    );
    store.attach(target);
    Some(LeashInteraction::AnimalFedByServer)
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut crate::world::simulation_level::LevelCtx<'_, '_>,
) {
    let id = entity.id();
    let position = entity.position();
    let silent = entity.is_silent();
    let y_rot = entity.y_rot();
    let movement = entity.delta_movement();
    let on_ground = entity.on_ground();
    let in_water = entity.is_in_water();
    let mut sounds = Vec::new();
    let mut dirty = Vec::new();
    let mut next_movement = None;
    let mut zero_pitch = false;
    let mut finished_sneeze = false;
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let is_baby = data.ageable_retained().is_some_and(|age| age.is_baby());
    let Some(state) = data.panda_retained_mut() else {
        return;
    };

    // Panda.java:346-350: both pacification causes end with the target.
    if !store.mob_ai(id).is_some_and(|ai| ai.target_ref().is_some()) {
        state.retaliation_satisfied = false;
    }
    if state.main_gene == PandaGene::Worried {
        if level.is_thundering() && !in_water {
            if state.set_flag(FLAG_SIT, true) {
                dirty.push(VANILLA_PANDA_FLAGS_DATA_ID);
            }
            if state.eat_counter != 0 {
                state.eat_counter = 0;
                dirty.push(VANILLA_PANDA_EAT_COUNTER_DATA_ID);
            }
        } else if state.eat_counter == 0 && state.set_flag(FLAG_SIT, false) {
            dirty.push(VANILLA_PANDA_FLAGS_DATA_ID);
        }
    }
    // `Panda.tick` follows its inherited tick (`Panda.java:335-388`).
    if state.unhappy_counter > 0 {
        if matches!(state.unhappy_counter, 29 | 14) {
            sounds.push("minecraft:entity.panda.cant_breed");
        }
        state.unhappy_counter -= 1;
        dirty.push(VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID);
    }
    if state.has_flag(FLAG_SNEEZE) {
        state.sneeze_counter += 1;
        dirty.push(VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID);
        if state.sneeze_counter > 20 {
            state.sneeze_counter = 0;
            state.set_flag(FLAG_SNEEZE, false);
            dirty.push(VANILLA_PANDA_FLAGS_DATA_ID);
            sounds.push("minecraft:entity.panda.sneeze");
            finished_sneeze = true;
        } else if state.sneeze_counter == 1 {
            sounds.push("minecraft:entity.panda.pre_sneeze");
        }
    }
    if state.has_flag(FLAG_ROLL) {
        state.roll_counter += 1;
        if state.roll_counter > 32 {
            state.set_flag(FLAG_ROLL, false);
            dirty.push(VANILLA_PANDA_FLAGS_DATA_ID);
        } else if state.roll_counter == 1 {
            let angle = f64::from(y_rot * (std::f32::consts::PI / 180.0));
            let multiplier = if is_baby { 0.1_f32 } else { 0.2_f32 };
            state.roll_delta = Vec3d {
                x: movement.x + f64::from(-mth_sin(angle) * multiplier),
                y: 0.0,
                z: movement.z + f64::from(mth_cos(angle) * multiplier),
            };
            next_movement = Some(Vec3d {
                y: 0.27,
                ..state.roll_delta
            });
        } else if matches!(state.roll_counter, 7 | 15 | 23) {
            next_movement = Some(Vec3d {
                x: 0.0,
                y: if on_ground { 0.27 } else { movement.y },
                z: 0.0,
            });
        } else {
            next_movement = Some(Vec3d {
                x: state.roll_delta.x,
                y: movement.y,
                z: state.roll_delta.z,
            });
        }
    } else {
        state.roll_counter = 0;
    }
    if state.has_flag(FLAG_SIT) {
        zero_pitch = true;
    }

    for data_id in dirty {
        entity.mark_synched_dirty(data_id);
    }
    if let Some(movement) = next_movement {
        entity.set_delta_movement(movement);
    }
    if zero_pitch {
        entity.set_rot(y_rot, 0.0);
    }
    if !silent {
        for resource_id in sounds {
            store.play_entity_sound(PendingEntitySound {
                position,
                resource_id: resource_id.to_string(),
                source: SoundSource::Neutral,
                volume: 1.0,
                pitch: 1.0,
            });
        }
    }
    if finished_sneeze {
        after_sneeze(entity, store, level);
    }
    tick_eating(entity, store, level);
}

// Panda.java:517-525.
fn after_sneeze(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut crate::world::simulation_level::LevelCtx<'_, '_>,
) {
    let bounds = entity.bounding_box().inflate(10.0, 10.0, 10.0);
    let thundering = level.is_thundering();
    for id in store.ids_in_box_sections(bounds) {
        let Some(other) = store.get_mut(id) else {
            continue;
        };
        if other.bounding_box().intersects(bounds) && can_jump_at_sneeze(other, thundering) {
            crate::entity::living::jump_from_ground(other, level);
        }
    }
    if can_jump_at_sneeze(entity, thundering) {
        crate::entity::living::jump_from_ground(entity, level);
    }
    if level.mob_drops() {
        crate::entity::gift_loot::drop_gift(
            entity,
            store,
            level,
            "minecraft:gameplay/panda_sneeze",
            entity.position(),
            true,
        );
    }
}

fn can_jump_at_sneeze(entity: &Entity, thundering: bool) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data)
        if !data.is_baby() && entity.on_ground() && !entity.is_in_water()
            && data.panda_retained().is_some_and(|panda| panda.can_perform_action(thundering)))
}

fn tick_eating(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut crate::world::simulation_level::LevelCtx<'_, '_>,
) {
    use bbb_protocol::packets::EquipmentSlot;
    let Some(hand) = entity
        .living()
        .map(|living| living.equipment().get(EquipmentSlot::MainHand))
    else {
        return;
    };
    let empty_hand = hand.is_none();
    let edible = hand
        .and_then(|stack| stack.summary().item_id)
        .and_then(bbb_protocol::item_types::vanilla_item_type)
        .is_some_and(|item| {
            level.registry_tag_membership(
                "minecraft:item",
                "minecraft:panda_eats_from_ground",
                item.resource_id,
            )
        });
    let random = level.level_random();
    let silent = entity.is_silent();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.panda_retained_mut() else {
        return;
    };
    let previous_counter = state.eat_counter;
    let previous_flags = state.flags;
    // `Panda.handleEating` (`Panda.java:394-423`).
    if state.eat_counter == 0
        && state.has_flag(FLAG_SIT)
        && !(state.main_gene == PandaGene::Worried && level.is_thundering())
        && !empty_hand
        && random.next_int(80) == 1
    {
        state.eat_counter = 1;
    } else if empty_hand || !state.has_flag(FLAG_SIT) {
        state.eat_counter = 0;
    }
    let mut sound = None;
    let mut consumed = false;
    if state.eat_counter > 0 {
        // Level.addParticle is empty on the server (`Level.java:492-493`).
        if state.eat_counter % 5 == 0 && !silent {
            sound = Some((
                0.5 + 0.5 * random.next_int(2) as f32,
                (random.next_float() - random.next_float()) * 0.2 + 1.0,
            ));
        }
        if state.eat_counter > 80 && random.next_int(20) == 1 {
            if state.eat_counter > 100 && edible {
                consumed = true;
                state.set_flag(FLAG_SIT, false);
            }
            state.eat_counter = 0;
        } else {
            state.eat_counter += 1;
        }
    }
    let counter_changed = previous_counter != state.eat_counter;
    let flags_changed = previous_flags != state.flags;
    if consumed {
        data.living_mut()
            .set_item_slot(EquipmentSlot::MainHand, None);
        level.game_event_at(
            crate::level_event::VanillaGameEvent::EAT,
            entity.position(),
            Some(entity.id()),
        );
    }
    if counter_changed {
        entity.mark_synched_dirty(VANILLA_PANDA_EAT_COUNTER_DATA_ID);
    }
    if flags_changed {
        entity.mark_synched_dirty(VANILLA_PANDA_FLAGS_DATA_ID);
    }
    if let Some((volume, pitch)) = sound {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.panda.eat".into(),
            source: SoundSource::Neutral,
            volume,
            pitch,
        });
    }
}

pub(crate) fn start_rolling(entity: &mut Entity) {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .panda_retained_mut()
            .is_some_and(|state| state.set_flag(FLAG_ROLL, true)),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_PANDA_FLAGS_DATA_ID);
    }
}

pub(crate) fn before_melee_attempt(entity: &mut Entity) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(state) = data.panda_retained_mut() {
            if !state.is_aggressive() {
                state.retaliation_satisfied = true;
            }
        }
    }
}

// Panda.java:601-607.
pub(crate) fn try_sit(entity: &mut Entity) {
    if !entity.is_in_water() {
        entity.living_mut().expect("Panda is living").zza = 0.0;
        set_sitting(entity, true);
    }
}

// Panda.java:1096-1106.
pub(crate) fn stop_sitting(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut crate::world::simulation_level::LevelCtx<'_, '_>,
) -> bool {
    let held = entity
        .living()
        .and_then(|living| {
            living
                .equipment()
                .get(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
        })
        .cloned();
    if let Some(stack) = held {
        if crate::entity::item::spawn_dropped_item_entity(store, entity.position(), stack).is_none()
        {
            return false;
        }
        if crate::entity::item_slots::set_living_item_in_tick(
            entity,
            store,
            level,
            bbb_protocol::equipment_slots::EquipmentSlot::MainHand,
            None,
        )
        .is_err()
        {
            return false;
        }
    }
    set_sitting(entity, false);
    true
}

pub(crate) fn before_hurt(entity: &mut Entity) {
    // `Panda.hurtServer` clears sitting before the inherited damage gate
    // (`Panda.java:541-544`).
    set_sitting(entity, false);
}

pub(crate) fn start_sneezing(entity: &mut Entity) {
    set_flag_published(entity, FLAG_SNEEZE, true);
}

pub(crate) fn set_on_back(entity: &mut Entity, on_back: bool) {
    set_flag_published(entity, FLAG_ON_BACK, on_back);
}

fn set_sitting(entity: &mut Entity, sitting: bool) {
    set_flag_published(entity, FLAG_SIT, sitting);
}

fn set_flag_published(entity: &mut Entity, flag: i8, value: bool) {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .panda_retained_mut()
            .is_some_and(|state| state.set_flag(flag, value)),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_PANDA_FLAGS_DATA_ID);
    }
}

#[cfg(test)]
mod tests;
