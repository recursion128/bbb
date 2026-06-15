use std::collections::BTreeMap;

use bbb_protocol::packets::{
    BossBarColor as ProtocolBossBarColor, BossBarOverlay as ProtocolBossBarOverlay,
    BossEvent as ProtocolBossEvent, BossEventOperation as ProtocolBossEventOperation,
    ChangeDifficulty as ProtocolChangeDifficulty, ClearTitles as ProtocolClearTitles,
    Difficulty as ProtocolDifficulty, SetActionBarText as ProtocolSetActionBarText,
    SetSubtitleText as ProtocolSetSubtitleText, SetTitleText as ProtocolSetTitleText,
    SetTitlesAnimation as ProtocolSetTitlesAnimation, SystemChat as ProtocolSystemChat,
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
    #[serde(default)]
    pub system_chat: Option<SystemChatLineState>,
    #[serde(default)]
    pub action_bar: Option<ActionBarState>,
    #[serde(default)]
    pub title: HudTitleState,
}

impl Default for ClientHudState {
    fn default() -> Self {
        Self {
            boss_bars: BTreeMap::new(),
            tab_list: TabListState::default(),
            difficulty: DifficultyState::default(),
            system_chat: None,
            action_bar: None,
            title: HudTitleState::default(),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemChatLineState {
    pub content: String,
    pub overlay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBarState {
    pub content: String,
    pub display_ticks: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HudTitleState {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
    pub title_time: i32,
}

impl Default for HudTitleState {
    fn default() -> Self {
        Self {
            title: None,
            subtitle: None,
            fade_in: 10,
            stay: 70,
            fade_out: 20,
            title_time: 0,
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
                let removed = self.client_hud.boss_bars.remove(&packet.id).is_some();
                if !removed {
                    self.counters.boss_events_ignored += 1;
                }
                removed
            }
            ProtocolBossEventOperation::UpdateProgress { progress } => {
                let Some(bar) = self.client_hud.boss_bars.get_mut(&packet.id) else {
                    self.counters.boss_events_ignored += 1;
                    self.update_boss_bar_count();
                    return false;
                };
                bar.progress = progress;
                true
            }
            ProtocolBossEventOperation::UpdateName { name } => {
                let Some(bar) = self.client_hud.boss_bars.get_mut(&packet.id) else {
                    self.counters.boss_events_ignored += 1;
                    self.update_boss_bar_count();
                    return false;
                };
                bar.name = name;
                true
            }
            ProtocolBossEventOperation::UpdateStyle { color, overlay } => {
                let Some(bar) = self.client_hud.boss_bars.get_mut(&packet.id) else {
                    self.counters.boss_events_ignored += 1;
                    self.update_boss_bar_count();
                    return false;
                };
                bar.color = boss_bar_color_name(color).to_string();
                bar.overlay = boss_bar_overlay_name(overlay).to_string();
                true
            }
            ProtocolBossEventOperation::UpdateProperties { flags } => {
                let Some(bar) = self.client_hud.boss_bars.get_mut(&packet.id) else {
                    self.counters.boss_events_ignored += 1;
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

    pub fn apply_system_chat(&mut self, packet: ProtocolSystemChat) {
        self.counters.system_chat_packets += 1;
        let line = SystemChatLineState {
            content: packet.content,
            overlay: packet.overlay,
        };
        if line.overlay {
            self.set_overlay_message(line.content.clone());
        }
        self.client_hud.system_chat = Some(line);
    }

    pub fn apply_action_bar_text(&mut self, packet: ProtocolSetActionBarText) {
        self.counters.action_bar_packets += 1;
        self.set_overlay_message(packet.content);
    }

    pub fn apply_title_text(&mut self, packet: ProtocolSetTitleText) {
        self.counters.title_text_packets += 1;
        self.client_hud.title.title = Some(packet.content);
        self.client_hud.title.title_time = title_total_ticks(&self.client_hud.title);
    }

    pub fn apply_subtitle_text(&mut self, packet: ProtocolSetSubtitleText) {
        self.counters.subtitle_text_packets += 1;
        self.client_hud.title.subtitle = Some(packet.content);
    }

    pub fn apply_clear_titles(&mut self, packet: ProtocolClearTitles) {
        self.counters.clear_titles_packets += 1;
        self.client_hud.title.title = None;
        self.client_hud.title.subtitle = None;
        self.client_hud.title.title_time = 0;
        if packet.reset_times {
            let defaults = HudTitleState::default();
            self.client_hud.title.fade_in = defaults.fade_in;
            self.client_hud.title.stay = defaults.stay;
            self.client_hud.title.fade_out = defaults.fade_out;
        }
    }

    pub fn apply_titles_animation(&mut self, packet: ProtocolSetTitlesAnimation) {
        self.counters.titles_animation_packets += 1;
        if packet.fade_in >= 0 {
            self.client_hud.title.fade_in = packet.fade_in;
        }
        if packet.stay >= 0 {
            self.client_hud.title.stay = packet.stay;
        }
        if packet.fade_out >= 0 {
            self.client_hud.title.fade_out = packet.fade_out;
        }
        if self.client_hud.title.title_time > 0 {
            self.client_hud.title.title_time = title_total_ticks(&self.client_hud.title);
        }
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

    pub fn system_chat(&self) -> Option<&SystemChatLineState> {
        self.client_hud.system_chat.as_ref()
    }

    pub fn action_bar(&self) -> Option<&ActionBarState> {
        self.client_hud.action_bar.as_ref()
    }

    pub fn title(&self) -> &HudTitleState {
        &self.client_hud.title
    }

    fn update_boss_bar_count(&mut self) {
        self.counters.boss_bars_tracked = self.client_hud.boss_bars.len();
    }

    fn set_overlay_message(&mut self, content: String) {
        self.client_hud.action_bar = Some(ActionBarState {
            content,
            display_ticks: 60,
        });
    }
}

fn title_total_ticks(title: &HudTitleState) -> i32 {
    title
        .fade_in
        .saturating_add(title.stay)
        .saturating_add(title.fade_out)
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
    use bbb_protocol::packets::{
        BossEventFlags as ProtocolBossEventFlags, ClearTitles as ProtocolClearTitles,
        SetActionBarText as ProtocolSetActionBarText, SetSubtitleText as ProtocolSetSubtitleText,
        SetTitleText as ProtocolSetTitleText, SetTitlesAnimation as ProtocolSetTitlesAnimation,
        SystemChat as ProtocolSystemChat,
    };

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
        assert_eq!(store.counters().boss_events_ignored, 1);
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

    #[test]
    fn hud_text_packets_update_canonical_state() {
        let mut store = WorldStore::new();

        store.apply_system_chat(ProtocolSystemChat {
            content: "Server restart soon".to_string(),
            overlay: false,
        });
        assert_eq!(
            store.system_chat(),
            Some(&SystemChatLineState {
                content: "Server restart soon".to_string(),
                overlay: false,
            })
        );
        assert_eq!(store.action_bar(), None);

        store.apply_system_chat(ProtocolSystemChat {
            content: "Overlay warning".to_string(),
            overlay: true,
        });
        assert_eq!(
            store.action_bar(),
            Some(&ActionBarState {
                content: "Overlay warning".to_string(),
                display_ticks: 60,
            })
        );

        store.apply_action_bar_text(ProtocolSetActionBarText {
            content: "Action ready".to_string(),
        });
        assert_eq!(
            store.action_bar(),
            Some(&ActionBarState {
                content: "Action ready".to_string(),
                display_ticks: 60,
            })
        );

        let counters = store.counters();
        assert_eq!(counters.system_chat_packets, 2);
        assert_eq!(counters.action_bar_packets, 1);
    }

    #[test]
    fn title_packets_match_vanilla_timing_rules() {
        let mut store = WorldStore::new();
        assert_eq!(
            store.title(),
            &HudTitleState {
                title: None,
                subtitle: None,
                fade_in: 10,
                stay: 70,
                fade_out: 20,
                title_time: 0,
            }
        );

        store.apply_titles_animation(ProtocolSetTitlesAnimation {
            fade_in: 5,
            stay: -1,
            fade_out: 15,
        });
        assert_eq!(store.title().fade_in, 5);
        assert_eq!(store.title().stay, 70);
        assert_eq!(store.title().fade_out, 15);
        assert_eq!(store.title().title_time, 0);

        store.apply_title_text(ProtocolSetTitleText {
            content: "Quest complete".to_string(),
        });
        store.apply_subtitle_text(ProtocolSetSubtitleText {
            content: "Return to camp".to_string(),
        });
        assert_eq!(store.title().title.as_deref(), Some("Quest complete"));
        assert_eq!(store.title().subtitle.as_deref(), Some("Return to camp"));
        assert_eq!(store.title().title_time, 90);

        store.apply_titles_animation(ProtocolSetTitlesAnimation {
            fade_in: -1,
            stay: 40,
            fade_out: -1,
        });
        assert_eq!(store.title().fade_in, 5);
        assert_eq!(store.title().stay, 40);
        assert_eq!(store.title().fade_out, 15);
        assert_eq!(store.title().title_time, 60);

        store.apply_clear_titles(ProtocolClearTitles { reset_times: false });
        assert_eq!(store.title().title, None);
        assert_eq!(store.title().subtitle, None);
        assert_eq!(store.title().title_time, 0);
        assert_eq!(store.title().fade_in, 5);
        assert_eq!(store.title().stay, 40);
        assert_eq!(store.title().fade_out, 15);

        store.apply_clear_titles(ProtocolClearTitles { reset_times: true });
        assert_eq!(store.title().fade_in, 10);
        assert_eq!(store.title().stay, 70);
        assert_eq!(store.title().fade_out, 20);

        let counters = store.counters();
        assert_eq!(counters.titles_animation_packets, 2);
        assert_eq!(counters.title_text_packets, 1);
        assert_eq!(counters.subtitle_text_packets, 1);
        assert_eq!(counters.clear_titles_packets, 2);
    }
}
