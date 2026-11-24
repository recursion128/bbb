//! Beacon beam scanning, pyramid levels, and ordered tick actions.

use bbb_protocol::packets::BlockPos;
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};

use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::{entry, unique_field};

const TAG_PRIMARY: &str = "primary_effect";
const TAG_SECONDARY: &str = "secondary_effect";
const TAG_LEVELS: &str = "Levels";
const TAG_CUSTOM_NAME: &str = "CustomName";
const TAG_LOCK: &str = "lock";

/// `BeaconBlockEntity.BEACON_EFFECTS` flattened (`BeaconBlockEntity.java:53-59`).
const VALID_EFFECTS: &[&str] = &[
    "minecraft:speed",
    "minecraft:haste",
    "minecraft:resistance",
    "minecraft:jump_boost",
    "minecraft:strength",
    "minecraft:regeneration",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeaconBlockRead {
    pub beam_color: Option<u32>,
    pub light_dampening: u8,
    pub is_bedrock: bool,
    pub is_base: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeaconBeamSection {
    pub color: u32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeaconSound {
    Ambient,
    Activate,
    Deactivate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeaconTickAction {
    ApplyEffects {
        levels: i32,
        primary: Option<String>,
        secondary: Option<String>,
    },
    Sound(BeaconSound),
    ConstructBeacon(i32),
}

/// `BeaconBlockEntity` (`:51-117`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BeaconBlockEntity {
    levels: i32,
    primary_power: Option<String>,
    secondary_power: Option<String>,
    custom_name: Option<NbtSummaryValue>,
    lock: Option<NbtSummaryValue>,
    beam_sections: Vec<BeaconBeamSection>,
    checking_beam_sections: Vec<BeaconBeamSection>,
    last_check_y: Option<i32>,
}

impl BeaconBlockEntity {
    #[must_use]
    pub(crate) fn next_check_y(&self, pos_y: i32, min_y: i32) -> i32 {
        let last_check_y = self.last_check_y.unwrap_or(min_y - 1);
        if last_check_y < pos_y {
            pos_y
        } else {
            last_check_y + 1
        }
    }

    // BeaconBlockEntity.java:119-195; the world applies the returned actions in order.
    #[must_use]
    pub fn tick(
        &mut self,
        pos: BlockPos,
        min_y: i32,
        surface_y: i32,
        game_time: i64,
        mut read: impl FnMut(BlockPos) -> BeaconBlockRead,
    ) -> Vec<BeaconTickAction> {
        // BeaconBlockEntity.java:360-363: setLevel starts the walk below minY.
        let mut last_check_y = self.last_check_y.unwrap_or(min_y - 1);
        let mut check_pos = if last_check_y < pos.y {
            self.checking_beam_sections.clear();
            last_check_y = pos.y - 1;
            pos
        } else {
            BlockPos {
                y: last_check_y + 1,
                ..pos
            }
        };
        for _ in 0..10 {
            if check_pos.y > surface_y {
                break;
            }
            let block = read(check_pos);
            if let Some(color) = block.beam_color {
                if self.checking_beam_sections.len() <= 1 {
                    self.checking_beam_sections
                        .push(BeaconBeamSection { color, height: 1 });
                } else {
                    let last = self.checking_beam_sections.last_mut().unwrap();
                    if color == last.color {
                        last.height += 1;
                    } else {
                        let color = average_color(last.color, color);
                        self.checking_beam_sections
                            .push(BeaconBeamSection { color, height: 1 });
                    }
                }
            } else if self.checking_beam_sections.is_empty()
                || block.light_dampening >= 15 && !block.is_bedrock
            {
                self.checking_beam_sections.clear();
                last_check_y = surface_y;
                break;
            } else {
                self.checking_beam_sections.last_mut().unwrap().height += 1;
            }
            check_pos.y += 1;
            last_check_y += 1;
        }

        let previous_levels = self.levels;
        let mut actions = Vec::new();
        if game_time % 80 == 0 {
            if !self.beam_sections.is_empty() {
                self.levels = update_base(pos, min_y, &mut read);
            }
            if self.levels > 0 && !self.beam_sections.is_empty() {
                actions.push(BeaconTickAction::ApplyEffects {
                    levels: self.levels,
                    primary: self.primary_power.clone(),
                    secondary: self.secondary_power.clone(),
                });
                actions.push(BeaconTickAction::Sound(BeaconSound::Ambient));
            }
        }
        if last_check_y >= surface_y {
            last_check_y = min_y - 1;
            self.beam_sections.clone_from(&self.checking_beam_sections);
            if previous_levels <= 0 && self.levels > 0 {
                actions.push(BeaconTickAction::Sound(BeaconSound::Activate));
                actions.push(BeaconTickAction::ConstructBeacon(self.levels));
            } else if previous_levels > 0 && self.levels <= 0 {
                actions.push(BeaconTickAction::Sound(BeaconSound::Deactivate));
            }
        }
        self.last_check_y = Some(last_check_y);
        actions
    }

    // BeaconBlockEntity.java:266-268.
    #[must_use]
    pub fn beam_sections(&self) -> &[BeaconBeamSection] {
        if self.levels == 0 {
            &[]
        } else {
            &self.beam_sections
        }
    }

    // BeaconBlockEntity.java:338-342.
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        self.custom_name = components.read("minecraft:custom_name")?;
        self.lock = components.read("minecraft:lock")?;
        Ok(())
    }

    // BeaconBlockEntity.java:313-315.
    pub(super) fn custom_name(&self) -> Option<&NbtSummaryValue> {
        self.custom_name.as_ref()
    }

    /// `LockCode.canUnlock` (`LockCode.java:25-27`); an unreadable lock is no lock.
    pub(super) fn can_open(
        &self,
        is_spectator: bool,
        key: &bbb_server_types::item_stack::RuntimeItemStack,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> bool {
        is_spectator
            || self.lock.as_ref().is_none_or(|raw| {
                bbb_protocol::packets::decode_persistent_item_predicate_summary(raw).map_or(
                    true,
                    |predicate| {
                        bbb_server_types::item_stack::item_predicate_matches(
                            &predicate, key, resolver,
                        )
                    },
                )
            })
    }

    /// `BeaconBlockEntity.collectImplicitComponents` (`:345-351`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        components.set("minecraft:custom_name", self.custom_name.clone());
        super::container_fields::collect_lock(components, self.lock.as_ref());
    }

    #[must_use]
    pub const fn levels(&self) -> i32 {
        self.levels
    }

    #[must_use]
    pub fn primary_power(&self) -> Option<&str> {
        self.primary_power.as_deref()
    }

    #[must_use]
    pub fn secondary_power(&self) -> Option<&str> {
        self.secondary_power.as_deref()
    }

    /// Whether the last completed scan found a beam, which gates the
    /// power-select sound (`:94-96`).
    #[must_use]
    pub fn has_beam(&self) -> bool {
        !self.beam_sections.is_empty()
    }

    /// `dataAccess.set(1|2, ...)` from `BeaconMenu.updateEffects` (`:97-101`):
    /// anything but a beacon effect is cleared.
    pub fn set_powers(&mut self, primary: Option<&str>, secondary: Option<&str>) {
        let filter = |effect: Option<&str>| {
            effect
                .filter(|effect| VALID_EFFECTS.contains(effect))
                .map(str::to_owned)
        };
        self.primary_power = filter(primary);
        self.secondary_power = filter(secondary);
    }

    /// `BeaconBlockEntity.saveAdditional` (`:299-306`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = Vec::new();
        if let Some(effect) = &self.primary_power {
            entries.push(entry(TAG_PRIMARY, NbtSummaryValue::string(effect.clone())));
        }
        if let Some(effect) = &self.secondary_power {
            entries.push(entry(
                TAG_SECONDARY,
                NbtSummaryValue::string(effect.clone()),
            ));
        }
        entries.push(entry(TAG_LEVELS, NbtSummaryValue::Int(self.levels)));
        if let Some(name) = &self.custom_name {
            entries.push(entry(TAG_CUSTOM_NAME, name.clone()));
        }
        if let Some(lock) = &self.lock {
            entries.push(entry(TAG_LOCK, lock.clone()));
        }
        entries
    }

    /// `BeaconBlockEntity.loadAdditional` (`:290-296`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.primary_power = unique_field(record, TAG_PRIMARY).and_then(load_effect);
        self.secondary_power = unique_field(record, TAG_SECONDARY).and_then(load_effect);
        self.custom_name = unique_field(record, TAG_CUSTOM_NAME).cloned();
        self.lock = unique_field(record, TAG_LOCK).cloned();
    }
}

// BeaconBlockEntity.java:197-223.
fn update_base(
    pos: BlockPos,
    min_y: i32,
    read: &mut impl FnMut(BlockPos) -> BeaconBlockRead,
) -> i32 {
    let mut levels = 0;
    for step in 1..=4 {
        let y = pos.y - step;
        if y < min_y {
            break;
        }
        let mut complete = true;
        for x in pos.x - step..=pos.x + step {
            for z in pos.z - step..=pos.z + step {
                if !read(BlockPos { x, y, z }).is_base {
                    complete = false;
                    break;
                }
            }
            if !complete {
                break;
            }
        }
        if !complete {
            break;
        }
        levels = step;
    }
    levels
}

// ARGB.java:221-223: each channel averages with integer division.
fn average_color(lhs: u32, rhs: u32) -> u32 {
    [0, 8, 16, 24].into_iter().fold(0, |color, shift| {
        color | ((((lhs >> shift) & 255) + ((rhs >> shift) & 255)) / 2) << shift
    })
}

/// `BeaconBlockEntity.loadEffect` (`:285-287`).
fn load_effect(value: &NbtSummaryValue) -> Option<String> {
    let NbtSummaryValue::String(raw) = value else {
        return None;
    };
    let id = raw
        .try_to_string()
        .ok()
        .and_then(|raw| normalize_resource_location(raw).ok())?;
    VALID_EFFECTS.contains(&id.as_str()).then_some(id)
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_BEACON_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::block_entity::{entry, new_block_entity, unique_field, BlockEntityPayload};

    use super::*;

    fn pos() -> BlockPos {
        BlockPos { x: 4, y: 5, z: 6 }
    }

    fn default_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_BEACON_ID)
            .expect("minecraft:beacon has a default state")
    }

    fn upsert(record: &mut Vec<NbtSummaryEntry>, extra: Vec<NbtSummaryEntry>) {
        for field in extra {
            if let Some(existing) = record.iter_mut().find(|entry| entry.name == field.name) {
                existing.value = field.value;
            } else {
                record.push(field);
            }
        }
    }

    #[test]
    fn beacon_save_load_round_trips_owned_fields() {
        let mut beacon = BeaconBlockEntity {
            levels: 4,
            primary_power: Some("minecraft:speed".to_string()),
            secondary_power: Some("minecraft:regeneration".to_string()),
            custom_name: Some(NbtSummaryValue::string("Beam")),
            lock: Some(NbtSummaryValue::Compound(vec![entry(
                "items",
                NbtSummaryValue::List(Vec::new()),
            )])),
            ..BeaconBlockEntity::default()
        };
        let mut record = new_block_entity(pos(), default_state())
            .expect("a beacon builds an object")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        upsert(&mut record, beacon.save_additional());
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture codec succeeds")
            .expect("the record names a beacon");
        let BlockEntityPayload::Beacon(loaded_beacon) = loaded.payload() else {
            panic!("payload is a beacon");
        };
        assert_eq!(loaded_beacon.levels(), 0);
        assert_eq!(loaded_beacon.primary_power(), Some("minecraft:speed"));
        assert_eq!(
            loaded_beacon.secondary_power(),
            Some("minecraft:regeneration")
        );
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(
            unique_field(&saved, TAG_PRIMARY),
            Some(&NbtSummaryValue::string("minecraft:speed"))
        );
        assert_eq!(
            unique_field(&saved, TAG_SECONDARY),
            Some(&NbtSummaryValue::string("minecraft:regeneration"))
        );
        assert_eq!(
            unique_field(&saved, TAG_LEVELS),
            Some(&NbtSummaryValue::Int(0))
        );
        assert_eq!(
            unique_field(&saved, TAG_CUSTOM_NAME),
            Some(&NbtSummaryValue::string("Beam"))
        );
        assert!(unique_field(&saved, TAG_LOCK).is_some());
        let again = crate::block_entity::load_test_fixture(pos(), default_state(), &saved)
            .expect("fixture codec succeeds")
            .expect("the round-trip still names a beacon");
        let BlockEntityPayload::Beacon(again) = again.payload() else {
            panic!("payload is a beacon");
        };
        assert_eq!(again.levels(), 0);
        assert_eq!(again.primary_power(), Some("minecraft:speed"));
        beacon.load_additional(&beacon.save_additional());
        assert_eq!(beacon.levels(), 4);
        assert_eq!(beacon.secondary_power(), Some("minecraft:regeneration"));
    }

    #[test]
    fn beacon_scans_mixed_beam_and_orders_base_effect_actions() {
        let pos = pos();
        let read = |at: BlockPos| BeaconBlockRead {
            beam_color: if at.x == pos.x && at.z == pos.z {
                match at.y - pos.y {
                    0 => Some(0xfff9fffe),
                    1 | 2 => Some(0xffb02e26),
                    3 => Some(0xff3c44aa),
                    _ => None,
                }
            } else {
                None
            },
            light_dampening: if at.y == pos.y + 4 { 15 } else { 0 },
            is_bedrock: at.y == pos.y + 4,
            is_base: (1..=4).contains(&(pos.y - at.y)),
        };
        let mut beacon = BeaconBlockEntity {
            primary_power: Some("minecraft:speed".to_string()),
            secondary_power: Some("minecraft:regeneration".to_string()),
            ..Default::default()
        };
        let surface_y = pos.y + 20;
        for time in [1, 2, 3, 78, 79] {
            assert!(beacon.tick(pos, 0, surface_y, time, read).is_empty());
        }
        assert_eq!(beacon.levels(), 0);
        assert!(beacon.beam_sections().is_empty());
        let actions = beacon.tick(pos, 0, surface_y, 80, read);
        assert!(actions.contains(&BeaconTickAction::ApplyEffects {
            levels: 4,
            primary: Some("minecraft:speed".to_string()),
            secondary: Some("minecraft:regeneration".to_string()),
        }));
        assert!(actions.contains(&BeaconTickAction::Sound(BeaconSound::Activate)));
        assert_eq!(beacon.levels(), 4);
        assert_eq!(
            beacon.beam_sections(),
            &[
                BeaconBeamSection {
                    color: 0xfff9fffe,
                    height: 1
                },
                BeaconBeamSection {
                    color: 0xffb02e26,
                    height: 2
                },
                BeaconBeamSection {
                    color: 0xff763968,
                    height: 18
                },
            ]
        );
        for time in [158, 159] {
            assert!(beacon.tick(pos, 0, surface_y, time, read).is_empty());
        }
        assert_eq!(
            beacon.tick(pos, 0, surface_y, 160, |at| BeaconBlockRead {
                is_base: false,
                ..read(at)
            }),
            vec![BeaconTickAction::Sound(BeaconSound::Deactivate)]
        );
        assert_eq!(beacon.levels(), 0);
        assert!(beacon.beam_sections().is_empty());
    }
}
