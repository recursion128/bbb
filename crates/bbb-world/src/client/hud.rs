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

use bbb_protocol::{NbtSummaryValue, RichTextComponent, StyledTextRun};

use crate::WorldStore;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientHudState {
    #[serde(default)]
    pub boss_bars: Vec<BossBarState>,
    #[serde(default)]
    pub boss_bar_instance_revision: u64,
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
            boss_bars: Vec::new(),
            boss_bar_instance_revision: 0,
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
    pub id: Uuid,
    pub name: RichTextComponent,
    pub progress: f32,
    #[serde(default)]
    pub instance_revision: u64,
    #[serde(default)]
    pub progress_revision: u64,
    pub color: String,
    pub overlay: String,
    pub darken_screen: bool,
    pub play_music: bool,
    pub create_world_fog: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabListState {
    pub header: Option<RichTextComponent>,
    pub footer: Option<RichTextComponent>,
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
    #[serde(default)]
    pub styled_content: Vec<StyledTextRun>,
    #[serde(default)]
    pub content_component: Option<NbtSummaryValue>,
    pub overlay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBarState {
    pub content: RichTextComponent,
    pub display_ticks: i32,
    /// Vanilla `Gui.animateOverlayMessageColor` (`setOverlayMessage(component,
    /// animate)`): only the jukebox now-playing path (`Gui.setNowPlaying`)
    /// passes `true`; both packet paths modeled here pass `false`
    /// (`ChatListener` overlay chat and `ClientPacketListener`
    /// `handleSetActionBarText`).
    #[serde(default)]
    pub animate_color: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HudTitleState {
    pub title: Option<RichTextComponent>,
    pub subtitle: Option<RichTextComponent>,
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
        if !self.preflight_packet_components(&packet, "boss event") {
            self.counters.boss_events_ignored += 1;
            return false;
        }
        let applied = match packet.operation {
            ProtocolBossEventOperation::Add {
                name,
                progress,
                color,
                overlay,
                flags,
            } => {
                self.client_hud.boss_bar_instance_revision =
                    self.client_hud.boss_bar_instance_revision.saturating_add(1);
                let bar = BossBarState {
                    id: packet.id,
                    name: *name,
                    progress,
                    instance_revision: self.client_hud.boss_bar_instance_revision,
                    progress_revision: 0,
                    color: boss_bar_color_name(color).to_string(),
                    overlay: boss_bar_overlay_name(overlay).to_string(),
                    darken_screen: flags.darken_screen,
                    play_music: flags.play_music,
                    create_world_fog: flags.create_world_fog,
                };
                if let Some(existing) = self
                    .client_hud
                    .boss_bars
                    .iter_mut()
                    .find(|existing| existing.id == packet.id)
                {
                    *existing = bar;
                } else {
                    self.client_hud.boss_bars.push(bar);
                }
                true
            }
            ProtocolBossEventOperation::Remove => {
                let previous_len = self.client_hud.boss_bars.len();
                self.client_hud.boss_bars.retain(|bar| bar.id != packet.id);
                let removed = self.client_hud.boss_bars.len() != previous_len;
                if !removed {
                    self.counters.boss_events_ignored += 1;
                }
                removed
            }
            ProtocolBossEventOperation::UpdateProgress { progress } => {
                let Some(bar) = self
                    .client_hud
                    .boss_bars
                    .iter_mut()
                    .find(|bar| bar.id == packet.id)
                else {
                    self.counters.boss_events_ignored += 1;
                    self.update_boss_bar_count();
                    return false;
                };
                bar.progress = progress;
                bar.progress_revision = bar.progress_revision.saturating_add(1);
                true
            }
            ProtocolBossEventOperation::UpdateName { name } => {
                let Some(bar) = self
                    .client_hud
                    .boss_bars
                    .iter_mut()
                    .find(|bar| bar.id == packet.id)
                else {
                    self.counters.boss_events_ignored += 1;
                    self.update_boss_bar_count();
                    return false;
                };
                bar.name = *name;
                true
            }
            ProtocolBossEventOperation::UpdateStyle { color, overlay } => {
                let Some(bar) = self
                    .client_hud
                    .boss_bars
                    .iter_mut()
                    .find(|bar| bar.id == packet.id)
                else {
                    self.counters.boss_events_ignored += 1;
                    self.update_boss_bar_count();
                    return false;
                };
                bar.color = boss_bar_color_name(color).to_string();
                bar.overlay = boss_bar_overlay_name(overlay).to_string();
                true
            }
            ProtocolBossEventOperation::UpdateProperties { flags } => {
                let Some(bar) = self
                    .client_hud
                    .boss_bars
                    .iter_mut()
                    .find(|bar| bar.id == packet.id)
                else {
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
        if !self.preflight_packet_components(&packet, "tab list") {
            return;
        }
        self.client_hud.tab_list.header = non_empty_tab_list_component(packet.header);
        self.client_hud.tab_list.footer = non_empty_tab_list_component(packet.footer);
        self.advance_player_list_display_revision();
    }

    pub fn apply_change_difficulty(&mut self, packet: ProtocolChangeDifficulty) {
        self.counters.change_difficulty_packets += 1;
        self.client_hud.difficulty = DifficultyState {
            difficulty: difficulty_name(packet.difficulty).to_string(),
            difficulty_locked: packet.locked,
        };
    }

    pub fn apply_system_chat(&mut self, packet: ProtocolSystemChat) {
        self.apply_system_chat_at(packet, 0);
    }

    pub(crate) fn apply_system_chat_at(
        &mut self,
        packet: ProtocolSystemChat,
        received_epoch_millis: i64,
    ) {
        self.counters.system_chat_packets += 1;
        if !self.preflight_packet_components(&packet, "system chat") {
            return;
        }
        let line = SystemChatLineState {
            content: packet.content.text().to_string(),
            styled_content: packet.content.styled().to_vec(),
            content_component: Some(packet.content.component().clone()),
            overlay: packet.overlay,
        };
        if line.overlay {
            self.set_overlay_message((*packet.content).clone());
            self.push_overlay_chat_narration(
                line.content.clone(),
                if line.styled_content.is_empty() {
                    vec![StyledTextRun {
                        text: line.content.clone(),
                        style: bbb_protocol::ComponentStyle::default(),
                    }]
                } else {
                    line.styled_content.clone()
                },
            );
        } else {
            self.push_rich_server_system_chat_message(
                line.content.clone(),
                line.styled_content.clone(),
                line.content_component.clone(),
                received_epoch_millis,
            );
        }
        self.client_hud.system_chat = Some(line);
    }

    pub fn apply_action_bar_text(&mut self, packet: ProtocolSetActionBarText) {
        self.counters.action_bar_packets += 1;
        if !self.preflight_packet_components(&packet, "action bar") {
            return;
        }
        self.set_overlay_message(packet.content);
    }

    pub fn apply_title_text(&mut self, packet: ProtocolSetTitleText) {
        self.counters.title_text_packets += 1;
        if !self.preflight_packet_components(&packet, "title") {
            return;
        }
        self.client_hud.title.title = Some(packet.content);
        self.client_hud.title.title_time = title_total_ticks(&self.client_hud.title);
    }

    pub fn apply_subtitle_text(&mut self, packet: ProtocolSetSubtitleText) {
        self.counters.subtitle_text_packets += 1;
        if !self.preflight_packet_components(&packet, "subtitle") {
            return;
        }
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

    pub fn boss_bars(&self) -> &[BossBarState] {
        &self.client_hud.boss_bars
    }

    pub fn boss_bar(&self, id: Uuid) -> Option<&BossBarState> {
        self.client_hud.boss_bars.iter().find(|bar| bar.id == id)
    }

    pub fn boss_overlay_should_darken_screen(&self) -> bool {
        self.client_hud
            .boss_bars
            .iter()
            .any(|bar| bar.darken_screen)
    }

    pub fn boss_overlay_should_create_world_fog(&self) -> bool {
        self.client_hud
            .boss_bars
            .iter()
            .any(|bar| bar.create_world_fog)
    }

    pub fn boss_overlay_should_play_music(&self) -> bool {
        self.client_hud.boss_bars.iter().any(|bar| bar.play_music)
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

    pub(crate) fn clear_hud_for_level_teardown(&mut self) {
        self.client_hud.tab_list = TabListState::default();
        self.client_hud.boss_bars.clear();
        self.update_boss_bar_count();
        self.client_hud.title = HudTitleState::default();
    }

    pub(crate) fn set_mount_onboard_overlay(&mut self) {
        // Vanilla `ClientPacketListener.handleSetEntityPassengersPacket` builds
        // `mount.onboard` with `Options.keyShift`. BBB currently has one fixed
        // sneak binding (`Left Shift`), so keep the localized English projection
        // beside the HUD owner until configurable key bindings/localization exist.
        let content = "Press Left Shift to Dismount".to_string();
        self.set_overlay_message(RichTextComponent::plain(content.clone()));
        self.push_interrupting_system_narration(
            content.clone(),
            vec![StyledTextRun {
                text: content,
                style: bbb_protocol::ComponentStyle::default(),
            }],
        );
    }

    /// Vanilla `CreativeModeInventoryScreen.handleHotbarLoadOrSave` save
    /// branch overlays and interrupt-narrates `inventory.hotbarSaved` built
    /// from the load-activator and hotbar-slot key names. BBB has fixed
    /// bindings (`X` + digit keys), so keep the localized English projection
    /// beside the HUD owner until configurable key bindings/localization
    /// exist.
    pub fn set_creative_hotbar_preset_saved_overlay(&mut self, slot_index: usize) {
        let content = format!(
            "Item hotbar saved (restore with X + {})",
            slot_index.saturating_add(1)
        );
        self.set_overlay_message(RichTextComponent::plain(content.clone()));
        self.push_interrupting_system_narration(
            content.clone(),
            vec![StyledTextRun {
                text: content,
                style: bbb_protocol::ComponentStyle::default(),
            }],
        );
    }

    fn set_overlay_message(&mut self, content: RichTextComponent) {
        self.client_hud.action_bar = Some(ActionBarState {
            content,
            display_ticks: 60,
            animate_color: false,
        });
    }

    pub(crate) fn set_jukebox_now_playing(&mut self, description: RichTextComponent) {
        let message = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "translate".to_string(),
                value: NbtSummaryValue::String("record.nowPlaying".to_string()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "with".to_string(),
                value: NbtSummaryValue::List(vec![description.component().clone()]),
            },
        ]))
        .expect("jukebox now-playing wrapper is a valid Component");
        self.client_hud.action_bar = Some(ActionBarState {
            content: message.clone(),
            display_ticks: 60,
            animate_color: true,
        });
        self.push_interrupting_system_component_narration(&message);
    }

    /// Vanilla `Gui.tick` (Gui.java:1152-1166, run from `Minecraft.tick` once
    /// per client tick): `overlayMessageTime` counts down while positive, and
    /// `titleTime` counts down while positive, clearing `title`/`subtitle`
    /// when it reaches zero. Fade times are untouched.
    pub fn advance_hud_text_ticks(&mut self, ticks: u32) {
        if ticks == 0 {
            return;
        }
        let ticks = i32::try_from(ticks).unwrap_or(i32::MAX);
        if let Some(action_bar) = self.client_hud.action_bar.as_mut() {
            if action_bar.display_ticks > 0 {
                action_bar.display_ticks = action_bar.display_ticks.saturating_sub(ticks).max(0);
            }
        }
        let title = &mut self.client_hud.title;
        if title.title_time > 0 {
            title.title_time = title.title_time.saturating_sub(ticks).max(0);
            if title.title_time <= 0 {
                title.title = None;
                title.subtitle = None;
            }
        }
    }
}

fn title_total_ticks(title: &HudTitleState) -> i32 {
    title
        .fade_in
        .saturating_add(title.stay)
        .saturating_add(title.fade_out)
}

fn non_empty_tab_list_component(component: RichTextComponent) -> Option<RichTextComponent> {
    (!component.is_empty()).then_some(component)
}

fn boss_bar_color_name(color: ProtocolBossBarColor) -> &'static str {
    color.as_str()
}

fn boss_bar_overlay_name(overlay: ProtocolBossBarOverlay) -> &'static str {
    overlay.as_str()
}

fn difficulty_name(difficulty: ProtocolDifficulty) -> &'static str {
    difficulty.serialized_name()
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

    fn plain_component(text: &str) -> Box<bbb_protocol::RichTextComponent> {
        Box::new(bbb_protocol::RichTextComponent::plain(text))
    }

    fn styled_component(text: &str) -> Box<bbb_protocol::RichTextComponent> {
        Box::new(
            bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(
                vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "text".to_string(),
                        value: NbtSummaryValue::String(text.to_string()),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "bold".to_string(),
                        value: NbtSummaryValue::Byte(1),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "color".to_string(),
                        value: NbtSummaryValue::String("red".to_string()),
                    },
                ],
            ))
            .unwrap(),
        )
    }

    fn rich_overlay_component(text: &str) -> RichTextComponent {
        RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String(text.to_string()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "font".to_string(),
                value: NbtSummaryValue::String("example:title".to_string()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "click_event".to_string(),
                value: NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "action".to_string(),
                        value: NbtSummaryValue::String("run_command".to_string()),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "command".to_string(),
                        value: NbtSummaryValue::String("/spawn".to_string()),
                    },
                ]),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "hover_event".to_string(),
                value: NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "action".to_string(),
                        value: NbtSummaryValue::String("show_text".to_string()),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "value".to_string(),
                        value: NbtSummaryValue::String("Details".to_string()),
                    },
                ]),
            },
        ]))
        .unwrap()
    }

    #[test]
    fn boss_events_add_update_remove_and_ignore_unknown_updates() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(1);
        let missing_id = Uuid::from_u128(2);

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::Add {
                name: styled_component("Ender Dragon"),
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
        let bar = store.boss_bar(id).unwrap();
        assert_eq!(bar.id, id);
        assert_eq!(bar.name.text(), "Ender Dragon");
        assert_eq!(bar.name.styled().len(), 1);
        assert_eq!(bar.name.styled()[0].style.bold, Some(true));
        assert_eq!(bar.name.styled()[0].style.color, Some(0xff5555));
        assert!(matches!(bar.name.component(), NbtSummaryValue::Compound(_)));
        assert_eq!(bar.progress, 0.75);
        assert_eq!(bar.instance_revision, 1);
        assert_eq!(bar.progress_revision, 0);
        assert_eq!(bar.color, "purple");
        assert_eq!(bar.overlay, "progress");
        assert!(bar.darken_screen);
        assert!(bar.create_world_fog);

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::UpdateProgress { progress: 0.5 },
        }));
        assert_eq!(store.boss_bar(id).unwrap().progress_revision, 1);
        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::UpdateProgress { progress: 0.5 },
        }));
        assert_eq!(store.boss_bar(id).unwrap().progress_revision, 2);
        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::UpdateName {
                name: plain_component("Wither"),
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
        let bar = store.boss_bar(id).unwrap();
        assert_eq!(bar.name.text(), "Wither");
        assert_eq!(bar.progress, 0.5);
        assert_eq!(bar.progress_revision, 2);
        assert_eq!(bar.color, "red");
        assert_eq!(bar.overlay, "notched_10");
        assert!(!bar.darken_screen);
        assert!(bar.play_music);
        assert!(!bar.create_world_fog);

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::Remove,
        }));
        assert!(store.boss_bars().is_empty());
        assert_eq!(store.counters().boss_event_packets, 8);
        assert_eq!(store.counters().boss_bars_tracked, 0);
        assert_eq!(store.counters().boss_events_ignored, 1);
    }

    #[test]
    fn boss_overlay_queries_fold_any_active_boss_bar_property() {
        let mut store = WorldStore::new();
        let darkening_id = Uuid::from_u128(1);
        let fog_id = Uuid::from_u128(2);

        assert!(!store.boss_overlay_should_darken_screen());
        assert!(!store.boss_overlay_should_create_world_fog());
        assert!(!store.boss_overlay_should_play_music());

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id: darkening_id,
            operation: ProtocolBossEventOperation::Add {
                name: plain_component("Dark"),
                progress: 1.0,
                color: ProtocolBossBarColor::Purple,
                overlay: ProtocolBossBarOverlay::Progress,
                flags: ProtocolBossEventFlags {
                    darken_screen: true,
                    play_music: false,
                    create_world_fog: false,
                },
            },
        }));
        assert!(store.apply_boss_event(ProtocolBossEvent {
            id: fog_id,
            operation: ProtocolBossEventOperation::Add {
                name: plain_component("Fog"),
                progress: 1.0,
                color: ProtocolBossBarColor::Red,
                overlay: ProtocolBossBarOverlay::Progress,
                flags: ProtocolBossEventFlags {
                    darken_screen: false,
                    play_music: false,
                    create_world_fog: true,
                },
            },
        }));

        assert!(store.boss_overlay_should_darken_screen());
        assert!(store.boss_overlay_should_create_world_fog());

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id: darkening_id,
            operation: ProtocolBossEventOperation::UpdateProperties {
                flags: ProtocolBossEventFlags {
                    darken_screen: false,
                    play_music: true,
                    create_world_fog: false,
                },
            },
        }));
        assert!(!store.boss_overlay_should_darken_screen());
        assert!(store.boss_overlay_should_create_world_fog());
        assert!(store.boss_overlay_should_play_music());

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id: fog_id,
            operation: ProtocolBossEventOperation::Remove,
        }));
        assert!(!store.boss_overlay_should_create_world_fog());
        assert!(store.boss_overlay_should_play_music());
    }

    #[test]
    fn boss_event_adds_keep_linked_hash_map_order_and_round_trip_exact_names() {
        let mut store = WorldStore::new();
        let first = Uuid::from_u128(7);
        let second = Uuid::from_u128(2);
        let add = |id, name| ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::Add {
                name,
                progress: 1.0,
                color: ProtocolBossBarColor::White,
                overlay: ProtocolBossBarOverlay::Progress,
                flags: ProtocolBossEventFlags::default(),
            },
        };

        assert!(store.apply_boss_event(add(first, styled_component("First"))));
        assert!(store.apply_boss_event(add(second, plain_component("Second"))));
        assert_eq!(
            store
                .boss_bars()
                .iter()
                .map(|bar| bar.id)
                .collect::<Vec<_>>(),
            vec![first, second]
        );

        let first_revision = store.boss_bar(first).unwrap().instance_revision;
        assert!(store.apply_boss_event(add(first, plain_component("Replaced"))));
        assert_eq!(
            store
                .boss_bars()
                .iter()
                .map(|bar| bar.id)
                .collect::<Vec<_>>(),
            vec![first, second]
        );
        assert!(store.boss_bar(first).unwrap().instance_revision > first_revision);
        assert_eq!(store.boss_bar(first).unwrap().name.text(), "Replaced");

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id: first,
            operation: ProtocolBossEventOperation::Remove,
        }));
        assert!(store.apply_boss_event(add(first, styled_component("Appended"))));
        assert_eq!(
            store
                .boss_bars()
                .iter()
                .map(|bar| bar.id)
                .collect::<Vec<_>>(),
            vec![second, first]
        );

        let encoded = serde_json::to_value(&store).unwrap();
        let restored: WorldStore = serde_json::from_value(encoded).unwrap();
        assert_eq!(restored.boss_bars(), store.boss_bars());
        assert_eq!(
            restored.boss_bar(first).unwrap().name.styled()[0]
                .style
                .bold,
            Some(true)
        );
        assert_eq!(
            restored.boss_bar(first).unwrap().name.component(),
            store.boss_bar(first).unwrap().name.component()
        );
    }

    #[test]
    fn tab_list_preserves_components_and_empty_text_clears_each_field() {
        let mut store = WorldStore::new();

        let styled_header = *styled_component("Welcome");
        let styled_header_nbt = styled_header.component().clone();
        store.apply_tab_list(ProtocolTabList {
            header: styled_header,
            footer: RichTextComponent::plain("Online"),
        });
        assert_eq!(store.scoreboard_display_revision(), 0);
        assert_eq!(store.player_list_display_revision(), 1);
        let header = store.tab_list().header.as_ref().unwrap();
        assert_eq!(header.text(), "Welcome");
        assert_eq!(header.styled()[0].style.bold, Some(true));
        assert_eq!(header.styled()[0].style.color, Some(0xFF_55_55));
        assert_eq!(header.component(), &styled_header_nbt);
        assert_eq!(
            store
                .tab_list()
                .footer
                .as_ref()
                .map(RichTextComponent::text),
            Some("Online")
        );
        let restored: WorldStore =
            serde_json::from_value(serde_json::to_value(&store).unwrap()).unwrap();
        assert_eq!(restored.tab_list(), store.tab_list());

        store.apply_tab_list(ProtocolTabList {
            header: RichTextComponent::plain(""),
            footer: RichTextComponent::plain("Still online"),
        });
        assert_eq!(store.tab_list().header, None);
        assert_eq!(
            store
                .tab_list()
                .footer
                .as_ref()
                .map(RichTextComponent::text),
            Some("Still online")
        );

        store.apply_tab_list(ProtocolTabList {
            header: RichTextComponent::plain("Players"),
            footer: RichTextComponent::plain(""),
        });
        assert_eq!(
            store
                .tab_list()
                .header
                .as_ref()
                .map(RichTextComponent::text),
            Some("Players")
        );
        assert_eq!(store.tab_list().footer, None);
        assert_eq!(store.counters().tab_list_packets, 3);
        assert_eq!(store.scoreboard_display_revision(), 0);
        assert_eq!(store.player_list_display_revision(), 3);
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

        store.apply_system_chat(ProtocolSystemChat::plain("Server restart soon", false));
        assert_eq!(
            store.system_chat(),
            Some(&SystemChatLineState {
                content: "Server restart soon".to_string(),
                styled_content: vec![StyledTextRun {
                    text: "Server restart soon".to_string(),
                    style: bbb_protocol::ComponentStyle::default(),
                }],
                content_component: Some(
                    NbtSummaryValue::String("Server restart soon".to_string(),)
                ),
                overlay: false,
            })
        );
        assert_eq!(store.action_bar(), None);
        assert_eq!(store.client_chat().messages.len(), 1);
        assert_eq!(
            store.client_chat().messages[0].kind,
            crate::ChatMessageKind::System
        );
        assert_eq!(
            store.client_chat().messages[0].content,
            "Server restart soon"
        );

        store.apply_system_chat(ProtocolSystemChat::plain("Overlay warning", true));
        assert_eq!(
            store.action_bar(),
            Some(&ActionBarState {
                content: RichTextComponent::plain("Overlay warning"),
                display_ticks: 60,
                animate_color: false,
            })
        );
        assert_eq!(store.client_chat().messages.len(), 1);

        store.apply_action_bar_text(ProtocolSetActionBarText::plain("Action ready"));
        assert_eq!(
            store.action_bar(),
            Some(&ActionBarState {
                content: RichTextComponent::plain("Action ready"),
                display_ticks: 60,
                animate_color: false,
            })
        );

        let counters = store.counters();
        assert_eq!(counters.system_chat_packets, 2);
        assert_eq!(counters.action_bar_packets, 1);
    }

    #[test]
    fn hud_text_components_remain_exact_across_all_overlay_producers() {
        let mut store = WorldStore::new();
        let component = rich_overlay_component("Styled");

        store.apply_system_chat(ProtocolSystemChat {
            content: Box::new(component.clone()),
            overlay: true,
        });
        assert_eq!(
            store.action_bar().map(|state| &state.content),
            Some(&component)
        );

        store.apply_action_bar_text(ProtocolSetActionBarText {
            content: component.clone(),
        });
        store.apply_title_text(ProtocolSetTitleText {
            content: component.clone(),
        });
        store.apply_subtitle_text(ProtocolSetSubtitleText {
            content: component.clone(),
        });

        assert_eq!(
            store.action_bar().map(|state| &state.content),
            Some(&component)
        );
        assert_eq!(store.title().title.as_ref(), Some(&component));
        assert_eq!(store.title().subtitle.as_ref(), Some(&component));
        assert_eq!(
            store.title().title.as_ref().unwrap().styled()[0]
                .style
                .font
                .as_deref(),
            Some("example:title")
        );

        let encoded = serde_json::to_value(&store).unwrap();
        let restored: WorldStore = serde_json::from_value(encoded).unwrap();
        assert_eq!(restored.action_bar(), store.action_bar());
        assert_eq!(restored.title(), store.title());
    }

    #[test]
    fn jukebox_now_playing_wraps_one_exact_component_for_overlay_and_narration() {
        let mut store = WorldStore::new();
        let description = rich_overlay_component("Custom song");
        let expected_component = NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "translate".to_string(),
                value: NbtSummaryValue::String("record.nowPlaying".to_string()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "with".to_string(),
                value: NbtSummaryValue::List(vec![description.component().clone()]),
            },
        ]);

        store.set_jukebox_now_playing(description);

        let action_bar = store.action_bar().unwrap();
        assert_eq!(action_bar.display_ticks, 60);
        assert!(action_bar.animate_color);
        assert_eq!(action_bar.content.component(), &expected_component);
        assert_eq!(
            action_bar
                .content
                .styled()
                .last()
                .unwrap()
                .style
                .font
                .as_deref(),
            Some("example:title")
        );
        let narrations = store.take_pending_chat_narrations();
        assert_eq!(narrations.len(), 1);
        assert_eq!(narrations[0].kind, crate::ChatNarrationKind::SystemNow);
        assert_eq!(
            narrations[0].content_component.as_ref(),
            Some(&expected_component)
        );
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

        store.apply_title_text(ProtocolSetTitleText::plain("Quest complete"));
        store.apply_subtitle_text(ProtocolSetSubtitleText::plain("Return to camp"));
        assert_eq!(
            store.title().title.as_ref().map(RichTextComponent::text),
            Some("Quest complete")
        );
        assert_eq!(
            store.title().subtitle.as_ref().map(RichTextComponent::text),
            Some("Return to camp")
        );
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

    #[test]
    fn level_teardown_resets_the_tab_list_boss_bars_and_titles_like_gui_on_disconnected() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(7);
        store.apply_tab_list(ProtocolTabList {
            header: RichTextComponent::plain("Welcome"),
            footer: RichTextComponent::plain("Online"),
        });
        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::Add {
                name: plain_component("Ender Dragon"),
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
        store.apply_titles_animation(ProtocolSetTitlesAnimation {
            fade_in: 5,
            stay: 40,
            fade_out: 15,
        });
        store.apply_title_text(ProtocolSetTitleText {
            content: *plain_component("Chapter One"),
        });
        store.apply_action_bar_text(ProtocolSetActionBarText {
            content: *plain_component("Now playing"),
        });
        let instance_revision = store.boss_bar(id).unwrap().instance_revision;
        assert_eq!(store.counters().boss_bars_tracked, 1);

        store.clear_client_level();

        assert_eq!(store.tab_list(), &TabListState::default());
        assert!(store.boss_bar(id).is_none());
        assert_eq!(store.counters().boss_bars_tracked, 0);
        assert_eq!(store.title(), &HudTitleState::default());
        assert!(store.action_bar().is_some());

        assert!(store.apply_boss_event(ProtocolBossEvent {
            id,
            operation: ProtocolBossEventOperation::Add {
                name: plain_component("Wither"),
                progress: 1.0,
                color: ProtocolBossBarColor::Blue,
                overlay: ProtocolBossBarOverlay::Progress,
                flags: ProtocolBossEventFlags {
                    darken_screen: false,
                    play_music: false,
                    create_world_fog: false,
                },
            },
        }));
        assert!(store.boss_bar(id).unwrap().instance_revision > instance_revision);
    }

    #[test]
    fn only_a_new_connection_chain_clears_the_displayed_chat_lines() {
        let mut store = WorldStore::new();
        store.push_client_system_chat_message("hello");
        assert_eq!(store.client_chat().messages.len(), 1);

        store.clear_client_level();

        assert_eq!(store.client_chat().messages.len(), 1);

        store.begin_connection_session(false);

        assert!(store.client_chat().messages.is_empty());
    }

    #[test]
    fn hud_text_ticks_count_down_like_vanilla_gui_tick() {
        let mut store = WorldStore::new();

        // No state: ticking is a no-op.
        store.advance_hud_text_ticks(3);
        assert_eq!(store.action_bar(), None);
        assert_eq!(store.title().title_time, 0);

        store.apply_action_bar_text(ProtocolSetActionBarText::plain("Action ready"));
        store.apply_title_text(ProtocolSetTitleText::plain("Quest complete"));
        store.apply_subtitle_text(ProtocolSetSubtitleText::plain("Return to camp"));
        assert_eq!(store.action_bar().unwrap().display_ticks, 60);
        assert_eq!(store.title().title_time, 100);

        store.advance_hud_text_ticks(1);
        assert_eq!(store.action_bar().unwrap().display_ticks, 59);
        assert_eq!(store.title().title_time, 99);

        // Vanilla `Gui.tick` clears title and subtitle when titleTime hits 0;
        // the overlay message string is kept with its timer floored at 0.
        store.advance_hud_text_ticks(99);
        assert_eq!(
            store.action_bar(),
            Some(&ActionBarState {
                content: RichTextComponent::plain("Action ready"),
                display_ticks: 0,
                animate_color: false,
            })
        );
        assert_eq!(store.title().title, None);
        assert_eq!(store.title().subtitle, None);
        assert_eq!(store.title().title_time, 0);
        // Fade times survive the countdown (only ClearTitles(reset) resets them).
        assert_eq!(store.title().fade_in, 10);
        assert_eq!(store.title().stay, 70);
        assert_eq!(store.title().fade_out, 20);

        // Timers never go negative on further ticks.
        store.advance_hud_text_ticks(5);
        assert_eq!(store.action_bar().unwrap().display_ticks, 0);
        assert_eq!(store.title().title_time, 0);
    }

    #[test]
    fn hud_text_tick_zero_and_partial_advances_keep_title_visible() {
        let mut store = WorldStore::new();
        store.apply_title_text(ProtocolSetTitleText::plain("Quest complete"));

        store.advance_hud_text_ticks(0);
        assert_eq!(store.title().title_time, 100);

        store.advance_hud_text_ticks(40);
        assert_eq!(
            store.title().title.as_ref().map(RichTextComponent::text),
            Some("Quest complete")
        );
        assert_eq!(store.title().title_time, 60);
    }
}
