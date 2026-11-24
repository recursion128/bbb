use bbb_protocol::packets::ItemStackSummary;
use uuid::Uuid;

use bbb_server_types::item_stack::RuntimeItemStack;

pub(super) const DEFAULT_LOOT_TABLE: &str = "minecraft:chests/trial_chambers/reward";

#[derive(Debug, Clone, PartialEq)]
pub struct VaultConfig {
    pub(super) loot_table: String,
    pub(super) activation_range: f64,
    pub(super) deactivation_range: f64,
    pub(super) key_item: RuntimeItemStack,
    pub(super) override_loot_table_to_display: Option<String>,
}

// VaultConfig.java:38-47; a decoded config's absent key is EMPTY instead.
impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            loot_table: DEFAULT_LOOT_TABLE.into(),
            activation_range: 4.0,
            deactivation_range: 4.5,
            key_item: RuntimeItemStack::from_summary(Self::default_key_summary()),
            override_loot_table_to_display: None,
        }
    }
}

impl VaultConfig {
    pub fn loot_table(&self) -> &str {
        &self.loot_table
    }
    pub const fn activation_range(&self) -> f64 {
        self.activation_range
    }
    pub const fn deactivation_range(&self) -> f64 {
        self.deactivation_range
    }
    pub fn override_loot_table_to_display(&self) -> Option<&str> {
        self.override_loot_table_to_display.as_deref()
    }
    pub fn key_item(&self) -> &RuntimeItemStack {
        &self.key_item
    }
    pub(super) fn key_is_empty(&self) -> bool {
        self.key_item.is_empty()
    }
    pub(super) fn default_key_summary() -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_TRIAL_KEY_ID),
            count: 1,
            component_patch: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VaultSharedData {
    pub(super) display_item: RuntimeItemStack,
    pub(super) connected_players: Vec<Uuid>,
    pub(super) connected_particles_range: f64,
    pub(super) dirty: bool,
}

impl Default for VaultSharedData {
    fn default() -> Self {
        Self {
            display_item: RuntimeItemStack::empty(),
            connected_players: Vec::new(),
            connected_particles_range: 4.5,
            dirty: false,
        }
    }
}

impl VaultSharedData {
    pub fn display_item(&self) -> &RuntimeItemStack {
        &self.display_item
    }
    pub fn connected_players(&self) -> &[Uuid] {
        &self.connected_players
    }
    pub const fn connected_particles_range(&self) -> f64 {
        self.connected_particles_range
    }
    pub(super) fn set_display(&mut self, item: RuntimeItemStack) {
        if !self.display_item.matches(&item) {
            self.display_item = item;
            self.dirty = true;
        }
    }
    // VaultSharedData.java:85-89: loading does not reset an existing dirty flag.
    pub(super) fn set(&mut self, from: Self) {
        self.display_item = from.display_item;
        self.connected_players = from.connected_players;
        self.connected_particles_range = from.connected_particles_range;
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VaultServerData {
    pub(super) rewarded_players: Vec<Uuid>,
    pub(super) state_updating_resumes_at: i64,
    pub(super) items_to_eject: Vec<RuntimeItemStack>,
    pub(super) total_ejections_needed: i32,
    pub(super) last_insert_fail_timestamp: i64,
    pub(super) dirty: bool,
}

impl VaultServerData {
    pub fn rewarded_players(&self) -> &[Uuid] {
        &self.rewarded_players
    }
    pub const fn state_updating_resumes_at(&self) -> i64 {
        self.state_updating_resumes_at
    }
    pub fn items_to_eject(&self) -> &[RuntimeItemStack] {
        &self.items_to_eject
    }
    pub const fn total_ejections_needed(&self) -> i32 {
        self.total_ejections_needed
    }
    pub(super) fn next_item(&self) -> RuntimeItemStack {
        self.items_to_eject
            .last()
            .cloned()
            .unwrap_or_else(RuntimeItemStack::empty)
    }
    pub(super) fn pause_until(&mut self, time: i64) {
        self.state_updating_resumes_at = time;
        self.dirty = true;
    }
    // VaultServerData.java:115-128: set deliberately retains the live total and fail clock.
    pub(super) fn set(&mut self, from: Self) {
        self.rewarded_players = from.rewarded_players;
        self.state_updating_resumes_at = from.state_updating_resumes_at;
        self.items_to_eject = from.items_to_eject;
    }
    pub(super) fn ejection_progress(&self) -> f32 {
        if self.total_ejections_needed == 1 {
            1.0
        } else {
            1.0 - (self.items_to_eject.len() as f32 - 1.0)
                / (self.total_ejections_needed as f32 - 1.0)
        }
    }
}
