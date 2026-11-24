use bbb_protocol::{
    entity_types::VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, RichTextComponent,
};
use serde::{Deserialize, Serialize};

use crate::{ScreenRequest, WorldStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecartCommandBlockDataState {
    pub command: bbb_protocol::JavaString,
    pub last_output: Option<RichTextComponent>,
    pub track_output: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecartCommandBlockEditorState {
    pub entity_id: i32,
    pub data: MinecartCommandBlockDataState,
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

    pub fn minecart_command_block_editor_open_count(&self) -> usize {
        self.client_ui.minecart_command_block_editor_open_count
    }

    pub fn try_open_minecart_command_block_editor(&mut self, entity_id: i32) -> bool {
        if !self.local_player_can_use_gamemaster_blocks() {
            return false;
        }
        let Some(data) = self.minecart_command_block_data(entity_id) else {
            return false;
        };
        self.client_ui.replace_screen();
        self.client_ui.minecart_command_block_editor_open_count = self
            .client_ui
            .minecart_command_block_editor_open_count
            .saturating_add(1);
        self.push_screen_request(ScreenRequest::MinecartCommandBlockEditor(
            MinecartCommandBlockEditorState { entity_id, data },
        ));
        true
    }

    pub fn set_minecart_command_block_track_output(
        &mut self,
        entity_id: i32,
        track_output: bool,
    ) -> bool {
        self.entities
            .set_minecart_command_block_track_output(entity_id, track_output)
    }

    pub fn clear_minecart_command_block_last_output(&mut self, entity_id: i32) -> bool {
        self.entities
            .clear_minecart_command_block_last_output(entity_id)
    }

    pub(crate) fn refresh_minecart_command_block_editor_from_authoritative_data(
        &mut self,
        entity_id: i32,
    ) {
        if let Some(data) = self.minecart_command_block_data(entity_id) {
            self.push_screen_request(ScreenRequest::MinecartCommandBlockEditorData(
                MinecartCommandBlockEditorState { entity_id, data },
            ));
        }
    }

    pub(crate) fn close_minecart_command_block_editor_for_replaced_entity(
        &mut self,
        entity_id: i32,
    ) {
        if self.entity_type_id(entity_id) == Some(VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID) {
            self.push_screen_request(ScreenRequest::CloseMinecartCommandBlockEditor(entity_id));
        }
    }

    pub(crate) fn close_minecart_command_block_editor_for_removed_entities(
        &mut self,
        entity_ids: &[i32],
    ) {
        for &entity_id in entity_ids {
            self.close_minecart_command_block_editor_for_replaced_entity(entity_id);
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
            store.minecart_command_block_data(300).unwrap(),
            MinecartCommandBlockDataState {
                command: Default::default(),
                last_output: None,
                track_output: true,
            }
        );

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
        assert!(store.set_minecart_command_block_track_output(300, false));

        assert!(store.apply_set_entity_data(metadata(300, "say authoritative", "previous output")));
        let editor = store
            .take_screen_requests()
            .into_iter()
            .rev()
            .find_map(|request| match request {
                ScreenRequest::MinecartCommandBlockEditorData(editor) => Some(editor),
                _ => None,
            })
            .unwrap();
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

        assert!(store.try_open_minecart_command_block_editor(300));
        assert!(!store.minecart_command_block_data(300).unwrap().track_output);
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
        assert!(store.clear_minecart_command_block_last_output(300));
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
        assert!(store.set_minecart_command_block_track_output(300, false));

        add_entity(
            &mut store,
            300,
            VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        );
        assert!(store
            .take_screen_requests()
            .contains(&ScreenRequest::CloseMinecartCommandBlockEditor(300)));
        assert!(store.try_open_minecart_command_block_editor(300));
        assert!(store.minecart_command_block_data(300).unwrap().track_output);

        assert_eq!(
            store.apply_remove_entities(RemoveEntities {
                entity_ids: vec![300]
            }),
            1
        );
        assert!(store
            .take_screen_requests()
            .contains(&ScreenRequest::CloseMinecartCommandBlockEditor(300)));
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
        assert!(store
            .take_screen_requests()
            .contains(&ScreenRequest::CloseLevelScreens));
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
                    value: EntityDataValueKind::String(command.into()),
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
