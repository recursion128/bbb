//! The save fields and implicit components every container payload shares:
//! `BaseContainerBlockEntity`'s lock and custom name and `RandomizableContainer`'s
//! loot-table pair.

use bbb_protocol::packets::PersistentItemPredicateSummary;
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};

use super::components::ComponentMapBuilder;
use super::value_input::long_or;
use super::{entry, unique_field};

/// `LockCode.TAG_LOCK` (`LockCode.java:13`).
const TAG_LOCK: &str = "lock";
/// `BaseContainerBlockEntity.saveAdditional` (`BaseContainerBlockEntity.java:48`).
const TAG_CUSTOM_NAME: &str = "CustomName";
/// `RandomizableContainer.LOOT_TABLE_TAG` / `LOOT_TABLE_SEED_TAG`
/// (`RandomizableContainer.java:22-23`).
const TAG_LOOT_TABLE: &str = "LootTable";
const TAG_LOOT_TABLE_SEED: &str = "LootTableSeed";

/// `BaseContainerBlockEntity.lockKey` and `name`
/// (`BaseContainerBlockEntity.java:30-31`), as the record stores them.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct ContainerNameAndLock {
    lock: Option<ContainerLock>,
    custom_name: Option<NbtSummaryValue>,
}

#[derive(Debug, Clone, PartialEq)]
struct ContainerLock {
    raw: NbtSummaryValue,
    predicate: PersistentItemPredicateSummary,
}

impl ContainerLock {
    fn read(raw: &NbtSummaryValue) -> Result<Self, String> {
        bbb_protocol::packets::decode_persistent_item_predicate_summary(raw)
            .map(|predicate| Self {
                raw: raw.clone(),
                predicate,
            })
            .map_err(|error| error.to_string())
    }
}

impl ContainerNameAndLock {
    // BaseContainerBlockEntity.java:80-81: the empty predicate is NO_LOCK.
    pub(super) fn is_locked(&self) -> bool {
        self.lock
            .as_ref()
            .is_some_and(|lock| lock.predicate != PersistentItemPredicateSummary::default())
    }

    pub(super) fn apply_components(
        &mut self,
        components: &mut super::components::ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        self.custom_name = components.read("minecraft:custom_name")?;
        self.lock = components
            .read("minecraft:lock")?
            .and_then(|raw| ContainerLock::read(&raw).ok());
        Ok(())
    }

    /// `BaseContainerBlockEntity.loadAdditional` (`BaseContainerBlockEntity.java:38-42`).
    pub(super) fn load(record: &[NbtSummaryEntry]) -> Self {
        Self {
            lock: unique_field(record, TAG_LOCK).and_then(|raw| ContainerLock::read(raw).ok()),
            custom_name: unique_field(record, TAG_CUSTOM_NAME).cloned(),
        }
    }

    /// `BaseContainerBlockEntity.saveAdditional` (`BaseContainerBlockEntity.java:45-49`).
    pub(super) fn save(&self, entries: &mut Vec<NbtSummaryEntry>) {
        if let Some(lock) = &self.lock {
            entries.push(entry(TAG_LOCK, lock.raw.clone()));
        }
        if let Some(name) = &self.custom_name {
            entries.push(entry(TAG_CUSTOM_NAME, name.clone()));
        }
    }

    pub(super) fn custom_name(&self) -> Option<&NbtSummaryValue> {
        self.custom_name.as_ref()
    }

    // LockCode.java:25-27: spectators do not query the key predicate.
    pub(super) fn can_unlock(
        &self,
        is_spectator: bool,
        key: &bbb_server_types::item_stack::RuntimeItemStack,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> bool {
        match &self.lock {
            Some(lock) if !is_spectator => {
                bbb_server_types::item_stack::item_predicate_matches(&lock.predicate, key, resolver)
            }
            _ => true,
        }
    }

    /// `BaseContainerBlockEntity.collectImplicitComponents`
    /// (`BaseContainerBlockEntity.java:157-165`) up to the container, which the
    /// payload adds from its own slots.
    pub(super) fn collect(&self, components: &mut ComponentMapBuilder<'_>) {
        components.set("minecraft:custom_name", self.custom_name.clone());
        collect_lock(components, self.lock.as_ref().map(|lock| &lock.raw));
    }
}

/// `components.set(DataComponents.LOCK, lockKey)` unless the key equals
/// `LockCode.NO_LOCK`, the empty item predicate (`LockCode.java:11`,
/// `BaseContainerBlockEntity.java:81`).
pub(super) fn collect_lock(
    components: &mut ComponentMapBuilder<'_>,
    lock: Option<&NbtSummaryValue>,
) {
    if let Some(lock) = lock.filter(|lock| {
        bbb_protocol::packets::decode_persistent_item_predicate_summary(lock)
            .map_or(true, |predicate| {
                predicate != PersistentItemPredicateSummary::default()
            })
    }) {
        components.set("minecraft:lock", Some(lock.clone()));
    }
}

/// One `unpackLootTable` request: the table key and the seed
/// `LootTable.fill` is handed (`RandomizableContainer.java:72-90`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerLoot {
    pub table_id: String,
    pub seed: i64,
}

/// `RandomizableContainerBlockEntity.lootTable` / `lootTableSeed`
/// (`RandomizableContainerBlockEntity.java:20-21`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ContainerLootTable {
    /// Held as the identifier `LootTable.KEY_CODEC` reads; a value it cannot
    /// read is `null` there and absent here.
    table: Option<String>,
    seed: i64,
}

impl ContainerLootTable {
    pub(super) fn apply_components(
        &mut self,
        components: &mut super::components::ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        let value = components.read("minecraft:container_loot")?;
        if let Some(NbtSummaryValue::Compound(loot)) = value {
            if let Some(table) = unique_field(&loot, "loot_table") {
                self.set(table, long_or(&loot, "seed", 0));
            }
        }
        Ok(())
    }

    /// `RandomizableContainer.tryLoadLootTable` (`RandomizableContainer.java:50-55`).
    pub(super) fn try_load(record: &[NbtSummaryEntry]) -> Self {
        let table = match unique_field(record, TAG_LOOT_TABLE) {
            Some(NbtSummaryValue::String(id)) => id
                .try_to_string()
                .ok()
                .and_then(|raw| normalize_resource_location(raw).ok()),
            _ => None,
        };
        Self {
            table,
            seed: long_or(record, TAG_LOOT_TABLE_SEED, 0),
        }
    }

    /// `RandomizableContainer.trySaveLootTable` (`RandomizableContainer.java:57-70`).
    pub(super) fn try_save(&self, entries: &mut Vec<NbtSummaryEntry>) -> bool {
        let Some(table) = &self.table else {
            return false;
        };
        entries.push(entry(
            TAG_LOOT_TABLE,
            NbtSummaryValue::string(table.clone()),
        ));
        if self.seed != 0 {
            entries.push(entry(TAG_LOOT_TABLE_SEED, NbtSummaryValue::Long(self.seed)));
        }
        true
    }

    #[must_use]
    pub const fn is_set(&self) -> bool {
        self.table.is_some()
    }

    #[must_use]
    pub fn table_id(&self) -> Option<&str> {
        self.table.as_deref()
    }

    pub(crate) const fn seed(&self) -> i64 {
        self.seed
    }

    pub(crate) fn set_seed(&mut self, seed: i64) {
        self.seed = seed;
    }

    pub(crate) fn set_id(&mut self, id: String, seed: i64) -> Result<(), String> {
        self.table = Some(normalize_resource_location(id).map_err(|error| error.to_string())?);
        self.seed = seed;
        Ok(())
    }

    /// `RandomizableContainer.setLootTable(lootTable, seed)`
    /// (`RandomizableContainer.java:29-32`). A value `LootTable.KEY_CODEC`
    /// cannot read leaves the container without a table.
    pub(super) fn set(&mut self, table: &NbtSummaryValue, seed: i64) {
        self.table = match table {
            NbtSummaryValue::String(id) => id
                .try_to_string()
                .ok()
                .and_then(|raw| normalize_resource_location(raw).ok()),
            _ => None,
        };
        self.seed = seed;
    }

    /// `this.setLootTable(null)` ahead of the fill (`RandomizableContainer.java:82`),
    /// answering what the fill reads.
    pub fn take(&mut self) -> Option<ContainerLoot> {
        self.table.take().map(|table_id| ContainerLoot {
            table_id,
            seed: self.seed,
        })
    }

    /// `RandomizableContainerBlockEntity.collectImplicitComponents`
    /// (`RandomizableContainerBlockEntity.java:107-112`) as `SeededContainerLoot.CODEC`
    /// writes it, whose seed field is left out at zero.
    pub(super) fn collect(&self, components: &mut ComponentMapBuilder<'_>) {
        let Some(table) = &self.table else {
            return;
        };
        let mut loot = vec![entry("loot_table", NbtSummaryValue::string(table.clone()))];
        if self.seed != 0 {
            loot.push(entry("seed", NbtSummaryValue::Long(self.seed)));
        }
        let value = Some(NbtSummaryValue::Compound(loot));
        components.set("minecraft:container_loot", value);
    }
}
