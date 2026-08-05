//! Vanilla `ScreenEffectRenderer` world-state queries: the eight-corner
//! view-blocking eye probe, the underwater overlay brightness sample, the
//! local-player sleeping-pose gate that suppresses first-person screen
//! effects, and the item-activation (totem) animation countdown.

use bbb_protocol::block_state_types::vanilla_block_state_support_facts;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID;
use bbb_protocol::item_types::VANILLA_ITEM_TOTEM_OF_UNDYING_ID;
use bbb_protocol::packets::{
    DataComponentPatchSummary, InteractionHand, ItemStackSummary as ProtocolItemStackSummary,
};
use serde::{Deserialize, Serialize};

use super::local_player_collision::LOCAL_PLAYER_HALF_WIDTH;
use crate::entities::vanilla_lightmap_brightness;
use crate::terrain::block_name_has_invisible_render_shape;
use crate::{BlockPos, SoundSeedRandomState, WorldStore};

/// Vanilla `ScreenEffectRenderer.ITEM_ACTIVATION_ANIMATION_LENGTH`.
pub const VANILLA_ITEM_ACTIVATION_ANIMATION_LENGTH_TICKS: u32 = 40;

/// Vanilla seeds `GameRenderer.random` (`RandomSource.create()`) from a wall
/// clock; a fixed local seed keeps `WorldStore` serializable and deterministic
/// while the LCG still advances across draws, so successive activations get
/// distinct offsets.
const ITEM_ACTIVATION_RANDOM_INITIAL_SEED: i64 = 0x707E_3_0FF5E7;

/// The `GameRenderer.random` stream consumed by `displayItemActivation`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemActivationRandomState(SoundSeedRandomState);

impl Default for ItemActivationRandomState {
    fn default() -> Self {
        Self(SoundSeedRandomState::with_seed(
            ITEM_ACTIVATION_RANDOM_INITIAL_SEED,
        ))
    }
}

impl ItemActivationRandomState {
    /// `ScreenEffectRenderer.displayItemActivation`: `random.nextFloat() * 2.0F - 1.0F`.
    fn next_offset(&mut self) -> f32 {
        self.0.next_float() * 2.0 - 1.0
    }
}

/// Vanilla `ScreenEffectRenderer` item-activation animation state
/// (`itemActivationItem`, `itemActivationTicks`, `itemActivationOffX/OffY`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemActivationState {
    pub item: ProtocolItemStackSummary,
    pub ticks_remaining: u32,
    pub off_x: f32,
    pub off_y: f32,
}

impl WorldStore {
    /// Vanilla `ScreenEffectRenderer.getViewBlockingState`: the first of eight
    /// eye-corner samples whose block state both renders (its `RenderShape` is
    /// not `INVISIBLE`) and reports `isViewBlocking`. The corners span
    /// `±0.5 * bbWidth * 0.8` horizontally and `±0.05 * scale` around the eye
    /// vertically, in bit order x, y, z.
    pub fn local_player_view_blocking_state(&self) -> Option<i32> {
        let pose = self.local_player_pose()?;
        let eye_y = pose.position.y + pose.eye_height();
        let scale = self
            .local_player_id()
            .and_then(|id| self.entities.render_scale(id))
            .unwrap_or(1.0);
        let width = LOCAL_PLAYER_HALF_WIDTH * 2.0 * f64::from(scale);
        for corner in 0..8 {
            let x = pose.position.x + (f64::from(corner % 2) - 0.5) * width * 0.8;
            let y = eye_y + (f64::from((corner >> 1) % 2) - 0.5) * 0.1 * f64::from(scale);
            let z = pose.position.z + (f64::from((corner >> 2) % 2) - 0.5) * width * 0.8;
            let pos = BlockPos {
                x: x.floor() as i32,
                y: y.floor() as i32,
                z: z.floor() as i32,
            };
            let Some(probe) = self.probe_block(pos) else {
                continue;
            };
            if probe
                .block_name
                .as_deref()
                .is_some_and(block_name_has_invisible_render_shape)
            {
                continue;
            }
            if vanilla_block_state_support_facts(probe.block_state_id)
                .is_some_and(|facts| facts.view_blocking())
            {
                return Some(probe.block_state_id);
            }
        }
        None
    }

    /// Vanilla `ScreenEffectRenderer.renderWater` brightness:
    /// `Lightmap.getBrightness(dimensionType, level.getMaxLocalRawBrightness(eye))`
    /// where the raw brightness is `max(blockLight, skyLight - skyDarken)` at
    /// the eye position.
    pub fn local_player_eye_brightness(&self, sky_darken: u8) -> Option<f32> {
        let pose = self.local_player_pose()?;
        let eye_y = pose.position.y + pose.eye_height();
        let pos = BlockPos {
            x: pose.position.x.floor() as i32,
            y: eye_y.floor() as i32,
            z: pose.position.z.floor() as i32,
        };
        // A loaded vanilla chunk always carries light; an absent bbb light
        // section degrades to raw brightness 0 rather than suppressing the
        // overlay entirely.
        let raw = self.sample_block_light(pos).map_or(0, |light| {
            light.block.max(light.sky.saturating_sub(sky_darken))
        });
        let ambient_light = self.level_info().map_or(0.0, |level| level.ambient_light());
        Some(vanilla_lightmap_brightness(ambient_light, raw))
    }

    /// Vanilla `ClientPacketListener.findTotem`: the first held stack carrying
    /// `DataComponents.DEATH_PROTECTION` (main hand before off hand), else a
    /// default totem-of-undying stack.
    fn find_totem_for_local_player(&self) -> ProtocolItemStackSummary {
        for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
            if let Some(stack) = self.local_item_in_hand(hand) {
                if self.item_stack_has_effective_component(
                    stack,
                    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
                ) {
                    return stack.clone();
                }
            }
        }
        ProtocolItemStackSummary {
            item_id: Some(VANILLA_ITEM_TOTEM_OF_UNDYING_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        }
    }

    /// Vanilla `ClientPacketListener.handleEntityEvent` case 35 for the local
    /// player: `GameRenderer.displayItemActivation(findTotem(player))`.
    pub(crate) fn display_local_player_item_activation(&mut self) {
        let item = self.find_totem_for_local_player();
        let off_x = self.item_activation_random.next_offset();
        let off_y = self.item_activation_random.next_offset();
        self.item_activation = Some(ItemActivationState {
            item,
            ticks_remaining: VANILLA_ITEM_ACTIVATION_ANIMATION_LENGTH_TICKS,
            off_x,
            off_y,
        });
    }

    /// Vanilla `ScreenEffectRenderer.tick`, run once per client tick where
    /// `tickRateManager().runsNormally()`: the countdown decrements and the
    /// item clears at zero.
    pub fn advance_item_activation_ticks(&mut self, running_ticks: u32) {
        if running_ticks == 0 {
            return;
        }
        if let Some(state) = self.item_activation.as_mut() {
            state.ticks_remaining = state.ticks_remaining.saturating_sub(running_ticks);
            if state.ticks_remaining == 0 {
                self.item_activation = None;
            }
        }
    }

    /// The in-flight vanilla `ScreenEffectRenderer` item-activation animation.
    pub fn item_activation(&self) -> Option<&ItemActivationState> {
        self.item_activation.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::super::local_player::LocalPlayerPoseState;
    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension, WorldStore,
    };
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate, Vec3d};
    use std::collections::BTreeMap;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    crate::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_named_block(world: &mut WorldStore, x: i32, y: i32, z: i32, name: &str) -> i32 {
        let properties = if name == "minecraft:barrier" {
            BTreeMap::from([("waterlogged".to_string(), "false".to_string())])
        } else {
            BTreeMap::new()
        };
        let state = crate::vanilla_block_state_id(name, &properties)
            .unwrap_or_else(|| panic!("block state for {name}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x, y, z },
            block_state_id: state,
        }));
        state
    }

    fn place_player(world: &mut WorldStore, x: f64, z: f64) {
        world.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d { x, y: 0.0, z },
            ..LocalPlayerPoseState::default()
        });
    }

    #[test]
    fn view_blocking_probe_reports_the_block_at_the_eye() {
        let mut world = world_with_air_chunk();
        place_player(&mut world, 0.5, 0.5);
        assert_eq!(world.local_player_view_blocking_state(), None);

        let stone = set_named_block(&mut world, 0, 1, 0, "minecraft:stone");
        assert_eq!(world.local_player_view_blocking_state(), Some(stone));
    }

    #[test]
    fn view_blocking_probe_skips_invisible_render_shapes() {
        // Vanilla `getViewBlockingState` requires `getRenderShape() != INVISIBLE`
        // before `isViewBlocking`; a barrier satisfies the latter but not the
        // former.
        let mut world = world_with_air_chunk();
        place_player(&mut world, 0.5, 0.5);
        set_named_block(&mut world, 0, 1, 0, "minecraft:barrier");
        assert_eq!(world.local_player_view_blocking_state(), None);
    }

    #[test]
    fn view_blocking_probe_reaches_the_eight_eye_corners() {
        // The corner samples span half of `bbWidth * 0.8` (0.24 for a player)
        // around the eye, so a block in the neighboring column still triggers
        // the overlay when the player hugs the cell boundary.
        let mut world = world_with_air_chunk();
        let stone = set_named_block(&mut world, 1, 1, 0, "minecraft:stone");

        place_player(&mut world, 0.5, 0.5);
        assert_eq!(world.local_player_view_blocking_state(), None);

        place_player(&mut world, 0.9, 0.5);
        assert_eq!(world.local_player_view_blocking_state(), Some(stone));
    }

    fn world_with_local_player(player_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&bbb_protocol::packets::PlayLogin {
            player_id,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: bbb_protocol::packets::CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 1,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        world.apply_add_entity(bbb_protocol::packets::AddEntity {
            id: player_id,
            uuid: uuid::Uuid::from_u128(player_id as u128),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world
    }

    fn totem_event(world: &mut WorldStore, entity_id: i32) {
        let mut random = crate::LevelEventSoundRandomState::with_seed(0);
        let leftover = world.apply_play_packet(
            bbb_protocol::packets::PlayClientbound::EntityEvent(
                bbb_protocol::packets::EntityEvent {
                    entity_id,
                    event_id: 35,
                },
            ),
            &mut random,
            &mut crate::play_apply::NoPlayApplyEffects,
        );
        assert!(leftover.is_none());
    }

    #[test]
    fn totem_event_for_the_local_player_starts_the_item_activation() {
        // Vanilla `ClientPacketListener.handleEntityEvent` case 35 calls
        // `displayItemActivation(findTotem(player))` only for the local
        // player; empty hands fall back to a default totem-of-undying stack.
        let mut world = world_with_local_player(42);
        assert!(world.item_activation().is_none());

        totem_event(&mut world, 42);
        let state = world.item_activation().expect("item activation");
        assert_eq!(state.ticks_remaining, 40);
        assert_eq!(
            state.item.item_id,
            Some(bbb_protocol::item_types::VANILLA_ITEM_TOTEM_OF_UNDYING_ID)
        );
        assert!((-1.0..=1.0).contains(&state.off_x));
        assert!((-1.0..=1.0).contains(&state.off_y));

        // The `GameRenderer.random` stream advances across activations.
        let first = (state.off_x, state.off_y);
        totem_event(&mut world, 42);
        let second = world.item_activation().expect("second activation");
        assert_eq!(second.ticks_remaining, 40);
        assert_ne!((second.off_x, second.off_y), first);
    }

    #[test]
    fn totem_event_for_a_remote_entity_does_not_start_the_animation() {
        let mut world = world_with_local_player(42);
        world.apply_add_entity(bbb_protocol::packets::AddEntity {
            id: 9,
            uuid: uuid::Uuid::from_u128(9),
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        totem_event(&mut world, 9);
        assert!(world.item_activation().is_none());
    }

    #[test]
    fn find_totem_prefers_the_held_death_protection_stack() {
        // Vanilla `findTotem` scans main hand before off hand for
        // `DataComponents.DEATH_PROTECTION` and clones the first match.
        let mut world = world_with_local_player(42);
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary::default();
        patch
            .added_type_ids
            .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID);
        let charm = bbb_protocol::packets::ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: patch,
        };
        assert!(world.apply_set_player_inventory(
            bbb_protocol::packets::SetPlayerInventory {
                slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
                item: charm.clone(),
            }
        ));

        totem_event(&mut world, 42);
        let state = world.item_activation().expect("item activation");
        assert_eq!(state.item, charm);
    }

    #[test]
    fn item_activation_ticks_count_down_and_clear_at_zero() {
        // Vanilla `ScreenEffectRenderer.tick` decrements once per running
        // client tick and nulls the item at zero.
        let mut world = world_with_local_player(42);
        totem_event(&mut world, 42);

        world.advance_item_activation_ticks(0);
        assert_eq!(
            world.item_activation().map(|state| state.ticks_remaining),
            Some(40)
        );
        world.advance_item_activation_ticks(39);
        assert_eq!(
            world.item_activation().map(|state| state.ticks_remaining),
            Some(1)
        );
        world.advance_item_activation_ticks(2);
        assert!(world.item_activation().is_none());
    }

    #[test]
    fn clear_client_level_resets_the_item_activation() {
        // Vanilla clearLevel runs `GameRenderer.resetData` ->
        // `resetItemActivation`.
        let mut world = world_with_local_player(42);
        totem_event(&mut world, 42);
        world.clear_client_level();
        assert!(world.item_activation().is_none());
    }
}
