use bbb_protocol::{
    entity_types::VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, RichTextComponent,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecartCommandBlockDataState {
    pub command: String,
    pub last_output: Option<RichTextComponent>,
    pub track_output: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecartCommandBlockEditorState {
    pub entity_id: i32,
    pub data: MinecartCommandBlockDataState,
    #[serde(default)]
    pub authoritative_revision: usize,
    #[serde(default)]
    target_incarnation: u64,
}

impl WorldStore {
    pub fn minecart_command_block_data(
        &self,
        entity_id: i32,
    ) -> Option<MinecartCommandBlockDataState> {
        if self.entity_type_id(entity_id) != Some(VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID) {
            return None;
        }
        let command_block = self.entities.minecart_command_block(entity_id)?;
        Some(MinecartCommandBlockDataState {
            command: command_block.command,
            last_output: command_block.last_output,
            track_output: command_block.track_output,
        })
    }

    pub fn minecart_command_block_editor(&self) -> Option<&MinecartCommandBlockEditorState> {
        self.client_ui
            .current_minecart_command_block_editor
            .as_ref()
    }

    pub fn minecart_command_block_editor_open_count(&self) -> usize {
        self.client_ui.minecart_command_block_editor_open_count
    }

    pub fn try_open_minecart_command_block_editor(&mut self, entity_id: i32) -> bool {
        if !self.local_player_can_use_gamemaster_blocks() {
            return false;
        }
        let Some(target_incarnation) = self.entities.incarnation(entity_id) else {
            return false;
        };
        let Some(data) = self.minecart_command_block_data(entity_id) else {
            return false;
        };
        self.client_ui.replace_screen();
        self.client_ui.minecart_command_block_editor_open_count = self
            .client_ui
            .minecart_command_block_editor_open_count
            .saturating_add(1);
        self.client_ui.current_minecart_command_block_editor =
            Some(MinecartCommandBlockEditorState {
                entity_id,
                data,
                authoritative_revision: 0,
                target_incarnation,
            });
        true
    }

    pub fn close_minecart_command_block_editor(&mut self) -> bool {
        self.client_ui
            .current_minecart_command_block_editor
            .take()
            .is_some()
    }

    pub fn set_minecart_command_block_editor_track_output(&mut self, track_output: bool) -> bool {
        let Some(entity_id) = self
            .minecart_command_block_editor()
            .map(|editor| editor.entity_id)
        else {
            return false;
        };
        if !self
            .entities
            .set_minecart_command_block_track_output(entity_id, track_output)
        {
            self.close_minecart_command_block_editor();
            return false;
        }
        if let Some(editor) = &mut self.client_ui.current_minecart_command_block_editor {
            editor.data.track_output = track_output;
        }
        true
    }

    pub fn clear_minecart_command_block_editor_last_output(&mut self) -> bool {
        let Some(entity_id) = self
            .minecart_command_block_editor()
            .map(|editor| editor.entity_id)
        else {
            return false;
        };
        if !self
            .entities
            .clear_minecart_command_block_last_output(entity_id)
        {
            self.close_minecart_command_block_editor();
            return false;
        }
        if let Some(editor) = &mut self.client_ui.current_minecart_command_block_editor {
            editor.data.last_output = None;
        }
        true
    }

    pub(crate) fn refresh_minecart_command_block_editor_from_authoritative_data(
        &mut self,
        entity_id: i32,
    ) {
        let Some(editor) = self.minecart_command_block_editor() else {
            return;
        };
        if editor.entity_id != entity_id {
            return;
        }
        if self.entities.incarnation(entity_id) != Some(editor.target_incarnation) {
            self.close_minecart_command_block_editor();
            return;
        }
        let Some(data) = self.minecart_command_block_data(entity_id) else {
            self.close_minecart_command_block_editor();
            return;
        };
        if let Some(editor) = &mut self.client_ui.current_minecart_command_block_editor {
            editor.data = data;
            editor.authoritative_revision = editor.authoritative_revision.saturating_add(1);
        }
    }

    pub(crate) fn close_minecart_command_block_editor_for_replaced_entity(
        &mut self,
        entity_id: i32,
    ) {
        if self
            .minecart_command_block_editor()
            .is_some_and(|editor| editor.entity_id == entity_id)
        {
            self.close_minecart_command_block_editor();
        }
    }

    pub(crate) fn close_minecart_command_block_editor_for_removed_entities(
        &mut self,
        entity_ids: &[i32],
    ) {
        if self
            .minecart_command_block_editor()
            .is_some_and(|editor| entity_ids.contains(&editor.entity_id))
        {
            self.close_minecart_command_block_editor();
        }
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        entity_data_serializers::{
            VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID, VANILLA_ENTITY_DATA_SERIALIZER_STRING_ID,
        },
        entity_types::{
            VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_MINECART_ID,
        },
        packets::{
            AddEntity, CommonPlayerSpawnInfo, EntityDataValue, EntityDataValueKind, EntityEvent,
            PlayLogin, RemoveEntities, SetEntityData, Vec3d,
        },
    };
    use uuid::Uuid;

    use super::*;
    use crate::{
        VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
        VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID,
    };

    #[test]
    fn editor_requires_permission_and_command_minecart_type() {
        let mut store = WorldStore::new();
        add_entity(
            &mut store,
            300,
            VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        );
        assert!(!store.try_open_minecart_command_block_editor(300));

        grant_gamemaster_block_access(&mut store);
        assert!(store.try_open_minecart_command_block_editor(300));
        assert_eq!(store.minecart_command_block_editor_open_count(), 1);
        assert_eq!(
            store.minecart_command_block_editor().unwrap().data,
            MinecartCommandBlockDataState {
                command: String::new(),
                last_output: None,
                track_output: true,
            }
        );

        store.close_minecart_command_block_editor();
        add_entity(&mut store, 301, VANILLA_ENTITY_TYPE_MINECART_ID);
        assert!(!store.try_open_minecart_command_block_editor(301));
        assert!(!store.try_open_minecart_command_block_editor(999));
    }

    #[test]
    fn metadata_refreshes_command_and_output_but_preserves_local_track_output() {
        let mut store = WorldStore::new();
        add_entity(
            &mut store,
            300,
            VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        );
        grant_gamemaster_block_access(&mut store);
        assert!(store.try_open_minecart_command_block_editor(300));
        assert!(store.set_minecart_command_block_editor_track_output(false));

        assert!(store.apply_set_entity_data(metadata(300, "say authoritative", "previous output")));
        let editor = store.minecart_command_block_editor().unwrap();
        assert_eq!(editor.authoritative_revision, 1);
        assert_eq!(editor.data.command, "say authoritative");
        assert_eq!(
            editor
                .data
                .last_output
                .as_ref()
                .map(RichTextComponent::text),
            Some("previous output")
        );
        assert!(!editor.data.track_output);

        assert!(store.close_minecart_command_block_editor());
        assert!(store.try_open_minecart_command_block_editor(300));
        assert!(
            !store
                .minecart_command_block_editor()
                .unwrap()
                .data
                .track_output
        );
        let cloned = store.clone();
        assert!(
            !cloned
                .minecart_command_block_data(300)
                .unwrap()
                .track_output
        );
        let encoded = serde_json::to_vec(&store).unwrap();
        let restored: WorldStore = serde_json::from_slice(&encoded).unwrap();
        assert!(
            !restored
                .minecart_command_block_data(300)
                .unwrap()
                .track_output
        );
        assert!(store.clear_minecart_command_block_editor_last_output());
        assert!(store
            .minecart_command_block_data(300)
            .unwrap()
            .last_output
            .is_none());
    }

    #[test]
    fn removal_and_same_id_replacement_close_editor_and_reset_local_state() {
        let mut store = WorldStore::new();
        add_entity(
            &mut store,
            300,
            VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        );
        grant_gamemaster_block_access(&mut store);
        assert!(store.try_open_minecart_command_block_editor(300));
        assert!(store.set_minecart_command_block_editor_track_output(false));

        add_entity(
            &mut store,
            300,
            VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        );
        assert!(store.minecart_command_block_editor().is_none());
        assert!(store.try_open_minecart_command_block_editor(300));
        assert!(
            store
                .minecart_command_block_editor()
                .unwrap()
                .data
                .track_output
        );

        assert_eq!(
            store.apply_remove_entities(RemoveEntities {
                entity_ids: vec![300]
            }),
            1
        );
        assert!(store.minecart_command_block_editor().is_none());
    }

    #[test]
    fn level_login_closes_minecart_command_block_editor() {
        let mut store = WorldStore::new();
        add_entity(
            &mut store,
            300,
            VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        );
        grant_gamemaster_block_access(&mut store);
        assert!(store.try_open_minecart_command_block_editor(300));

        store.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        assert!(store.minecart_command_block_editor().is_none());
    }

    fn grant_gamemaster_block_access(store: &mut WorldStore) {
        store.local_player_id = Some(7);
        assert!(store.apply_entity_event(EntityEvent {
            entity_id: 7,
            event_id: 26,
        }));
        store.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
    }

    fn add_entity(store: &mut WorldStore, id: i32, entity_type_id: i32) {
        store.apply_add_entity(AddEntity {
            id,
            uuid: Uuid::from_u128(u128::try_from(id).unwrap()),
            entity_type_id,
            position: Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
    }

    fn metadata(entity_id: i32, command: &str, output: &str) -> SetEntityData {
        SetEntityData {
            id: entity_id,
            values: vec![
                EntityDataValue {
                    data_id: VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_STRING_ID,
                    value: EntityDataValueKind::String(command.to_string()),
                },
                EntityDataValue {
                    data_id: VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID,
                    value: EntityDataValueKind::Component(RichTextComponent::plain(output)),
                },
            ],
        }
    }
}
