use std::collections::BTreeMap;

use bbb_protocol::packets::{
    BossBarColor as ProtocolBossBarColor, BossBarOverlay as ProtocolBossBarOverlay,
    BossEvent as ProtocolBossEvent, BossEventOperation as ProtocolBossEventOperation,
    ChangeDifficulty as ProtocolChangeDifficulty, Difficulty as ProtocolDifficulty,
    TabList as ProtocolTabList,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WorldStore;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientHudState {
    #[serde(default)]
    pub boss_bars: BTreeMap<Uuid, BossBarState>,
    #[serde(default)]
    pub tab_list: TabListState,
    #[serde(default)]
    pub difficulty: DifficultyState,
}

impl Default for ClientHudState {
    fn default() -> Self {
        Self {
            boss_bars: BTreeMap::new(),
            tab_list: TabListState::default(),
            difficulty: DifficultyState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossBarState {
    pub name: String,
    pub progress: f32,
    pub color: String,
    pub overlay: String,
    pub darken_screen: bool,
    pub play_music: bool,
    pub create_world_fog: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabListState {
    pub header: Option<String>,
    pub footer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifficultyState {
    pub difficulty: String,
    pub difficulty_locked: bool,
}

impl Default for DifficultyState {
    fn default() -> Self {
        Self {
            difficulty: difficulty_name(ProtocolDifficulty::Normal).to_string(),
            difficulty_locked: false,
        }
    }
}

impl WorldStore {
    pub fn apply_boss_event(&mut self, packet: ProtocolBossEvent) -> bool {
        self.counters.boss_event_packets += 1;
        let applied = match packet.operation {
            ProtocolBossEventOperation::Add {
                name,
                progress,
                color,
                overlay,
                flags,
            } => {
                self.client_hud.boss_bars.insert(
                    packet.id,
                    BossBarState {
                        name,
                        progress,
                        color: boss_bar_color_name(color).to_string(),
                        overlay: boss_bar_overlay_name(overlay).to_string(),
                        darken_screen: flags.darken_screen,
                        play_music: flags.play_music,
                        create_world_fog: flags.create_world_fog,
                    },
                );
                true
            }
            ProtocolBossEventOperation::Remove => {
                self.client_hud.boss_bars.remove(&packet.id).is_some()
            }
            ProtocolBossEventOperation::UpdateProgress { progress } => {
                let Some(bar) = self.client_hud.boss_bars.get_mut(&packet.id) else {
                    self.update_boss_bar_count();
                    return false;
                };
                bar.progress = progress;
                true
            }
            ProtocolBossEventOperation::UpdateName { name } => {
                let Some(bar) = self.client_hud.boss_bars.get_mut(&packet.id) else {
                    self.update_boss_bar_count();
                    return false;
                };
                bar.name = name;
                true
            }
            ProtocolBossEventOperation::UpdateStyle { color, overlay } => {
                let Some(bar) = self.client_hud.boss_bars.get_mut(&packet.id) else {
                    self.update_boss_bar_count();
                    return false;
                };
                bar.color = boss_bar_color_name(color).to_string();
                bar.overlay = boss_bar_overlay_name(overlay).to_string();
                true
            }
            ProtocolBossEventOperation::UpdateProperties { flags } => {
                let Some(bar) = self.client_hud.boss_bars.get_mut(&packet.id) else {
                    self.update_boss_bar_count();
                    return false;
                };
                bar.darken_screen = flags.darken_screen;
                bar.play_music = flags.play_music;
                bar.create_world_fog = flags.create_world_fog;
                true
            }
        };
        self.update_boss_bar_count();
        applied
    }

    pub fn apply_tab_list(&mut self, packet: ProtocolTabList) {
        self.counters.tab_list_packets += 1;
        self.client_hud.tab_list.header = non_empty_component_string(packet.header);
        self.client_hud.tab_list.footer = non_empty_component_string(packet.footer);
    }

    pub fn apply_change_difficulty(&mut self, packet: ProtocolChangeDifficulty) {
        self.counters.change_difficulty_packets += 1;
        self.client_hud.difficulty = DifficultyState {
            difficulty: difficulty_name(packet.difficulty).to_string(),
            difficulty_locked: packet.locked,
        };
    }

    pub fn client_hud(&self) -> &ClientHudState {
        &self.client_hud
    }

    pub fn boss_bars(&self) -> &BTreeMap<Uuid, BossBarState> {
        &self.client_hud.boss_bars
    }

    pub fn tab_list(&self) -> &TabListState {
        &self.client_hud.tab_list
    }

    pub fn difficulty(&self) -> &DifficultyState {
        &self.client_hud.difficulty
    }

    fn update_boss_bar_count(&mut self) {
        self.counters.boss_bars_tracked = self.client_hud.boss_bars.len();
    }
}

fn non_empty_component_string(component: Option<String>) -> Option<String> {
    component.filter(|value| !value.is_empty())
}

fn boss_bar_color_name(color: ProtocolBossBarColor) -> &'static str {
    match color {
        ProtocolBossBarColor::Pink => "pink",
        ProtocolBossBarColor::Blue => "blue",
        ProtocolBossBarColor::Red => "red",
        ProtocolBossBarColor::Green => "green",
        ProtocolBossBarColor::Yellow => "yellow",
        ProtocolBossBarColor::Purple => "purple",
        ProtocolBossBarColor::White => "white",
    }
}

fn boss_bar_overlay_name(overlay: ProtocolBossBarOverlay) -> &'static str {
    match overlay {
        ProtocolBossBarOverlay::Progress => "progress",
        ProtocolBossBarOverlay::Notched6 => "notched_6",
        ProtocolBossBarOverlay::Notched10 => "notched_10",
        ProtocolBossBarOverlay::Notched12 => "notched_12",
        ProtocolBossBarOverlay::Notched20 => "notched_20",
    }
}

fn difficulty_name(difficulty: ProtocolDifficulty) -> &'static str {
    match difficulty {
        ProtocolDifficulty::Peaceful => "peaceful",
        ProtocolDifficulty::Easy => "easy",
        ProtocolDifficulty::Normal => "normal",
        ProtocolDifficulty::Hard => "hard",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::BossEventFlags as ProtocolBossEventFlags;

    #[test]
    fn boss_events_add_update_remove_and_ignore_unknown_updates() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(1);
        let missing_id = Uuid::from_u128(2);

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::Add {
                name: "Ender Dragon".to_string(),
                progress: 0.75,
                color: ProtocolBossBarColor::Purple,
                overlay: ProtocolBossBarOverlay::Progress,
                flags: ProtocolBossEventFlags {
                    darken_screen: true,
                    play_music: false,
                    create_world_fog: true,
                },
            },
        }));
        assert_eq!(
            store.boss_bars().get(&id),
            Some(&BossBarState {
                name: "Ender Dragon".to_string(),
                progress: 0.75,
                color: "purple".to_string(),
                overlay: "progress".to_string(),
                darken_screen: true,
                play_music: false,
                create_world_fog: true,
            })
        );

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::UpdateProgress { progress: 0.5 },
        }));
        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::UpdateName {
                name: "Wither".to_string(),
            },
        }));
        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::UpdateStyle {
                color: ProtocolBossBarColor::Red,
                overlay: ProtocolBossBarOverlay::Notched10,
            },
        }));
        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::UpdateProperties {
                flags: ProtocolBossEventFlags {
                    darken_screen: false,
                    play_music: true,
                    create_world_fog: false,
                },
            },
        }));

        assert!(!store.apply_boss_event(ProtocolBossEvent {
            id: missing_id,
            operation: ProtocolBossEventOperation::UpdateProgress { progress: 1.0 },
        }));
        assert_eq!(store.boss_bars().len(), 1);
        assert_eq!(
            store.boss_bars().get(&id),
            Some(&BossBarState {
                name: "Wither".to_string(),
                progress: 0.5,
                color: "red".to_string(),
                overlay: "notched_10".to_string(),
                darken_screen: false,
                play_music: true,
                create_world_fog: false,
            })
        );

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::Remove,
        }));
        assert!(store.boss_bars().is_empty());
        assert_eq!(store.counters().boss_event_packets, 7);
        assert_eq!(store.counters().boss_bars_tracked, 0);
    }

    #[test]
    fn tab_list_empty_components_clear_header_and_footer() {
        let mut store = WorldStore::new();

        store.apply_tab_list(ProtocolTabList {
            header: Some("Welcome".to_string()),
            footer: Some("Online".to_string()),
        });
        assert_eq!(store.tab_list().header.as_deref(), Some("Welcome"));
        assert_eq!(store.tab_list().footer.as_deref(), Some("Online"));

        store.apply_tab_list(ProtocolTabList {
            header: None,
            footer: Some("Still online".to_string()),
        });
        assert_eq!(store.tab_list().header, None);
        assert_eq!(store.tab_list().footer.as_deref(), Some("Still online"));

        store.apply_tab_list(ProtocolTabList {
            header: Some("Players".to_string()),
            footer: None,
        });
        assert_eq!(store.tab_list().header.as_deref(), Some("Players"));
        assert_eq!(store.tab_list().footer, None);
        assert_eq!(store.counters().tab_list_packets, 3);
    }

    #[test]
    fn change_difficulty_updates_client_level_data_state() {
        let mut store = WorldStore::new();

        assert_eq!(store.difficulty().difficulty, "normal");
        assert!(!store.difficulty().difficulty_locked);

        store.apply_change_difficulty(ProtocolChangeDifficulty {
            difficulty: ProtocolDifficulty::Hard,
            locked: true,
        });
        assert_eq!(store.difficulty().difficulty, "hard");
        assert!(store.difficulty().difficulty_locked);

        store.apply_change_difficulty(ProtocolChangeDifficulty {
            difficulty: ProtocolDifficulty::Peaceful,
            locked: false,
        });
        assert_eq!(store.difficulty().difficulty, "peaceful");
        assert!(!store.difficulty().difficulty_locked);
        assert_eq!(store.counters().change_difficulty_packets, 2);
    }
}
