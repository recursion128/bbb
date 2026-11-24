use super::*;
use crate::registries::BlockStateInfo;

impl WorldStore {
    pub fn enderman_carried_block_state(&self, entity_id: i32) -> Option<&'static BlockStateInfo> {
        let state_id = self.entities.enderman_carried_block_state_id(entity_id)??;
        self.entity_block_model_state(state_id)
    }

    /// Resolves one synchronized block-state id into the model identity and
    /// properties consumed by entity-owned block submissions such as
    /// `BlockDisplayRenderer`.
    pub fn entity_block_model_state(&self, state_id: i32) -> Option<&'static BlockStateInfo> {
        self.registries.block_state(state_id)
    }

    pub fn minecart_display_block_state(
        &self,
        entity_id: i32,
    ) -> Option<MinecartDisplayBlockState> {
        self.entities
            .minecart_display_block_state(entity_id, &self.registries)
    }

    pub fn falling_block_state(&self, entity_id: i32) -> Option<FallingBlockModelState> {
        self.entities
            .falling_block_state(entity_id, &self.registries)
    }

    pub fn primed_tnt_block_state(&self, entity_id: i32) -> Option<&'static BlockStateInfo> {
        self.entities
            .primed_tnt_block_state(entity_id, &self.registries)
    }

    pub fn primed_tnt_fuse_remaining_in_ticks(
        &self,
        entity_id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        self.entities
            .primed_tnt_fuse_remaining_in_ticks(entity_id, partial_ticks)
    }

    pub fn primed_tnt_smoke_particle_states(&self) -> Vec<PrimedTntSmokeParticleState> {
        self.entities.primed_tnt_smoke_particle_states()
    }

    pub fn take_ravager_stun_particle_states(&mut self) -> Vec<RavagerStunParticleState> {
        self.entities.take_ravager_stun_particle_states()
    }

    pub fn take_evoker_fangs_crit_particle_states(&mut self) -> Vec<EvokerFangsCritParticleState> {
        self.entities.take_evoker_fangs_crit_particle_states()
    }

    pub fn drain_entity_client_movement_effect_states(
        &mut self,
    ) -> std::vec::Drain<'_, EntityClientMovementEffectState> {
        self.entities.drain_entity_client_movement_effect_states()
    }

    pub fn take_entity_client_movement_effect_states(
        &mut self,
    ) -> Vec<EntityClientMovementEffectState> {
        self.entities.take_entity_client_movement_effect_states()
    }

    pub fn recycle_entity_client_movement_effect_states(
        &mut self,
        effects: Vec<EntityClientMovementEffectState>,
    ) {
        self.entities
            .recycle_entity_client_movement_effect_states(effects);
    }

    pub fn take_ominous_item_spawner_particle_states(
        &mut self,
    ) -> Vec<OminousItemSpawnerParticleState> {
        self.entities.take_ominous_item_spawner_particle_states()
    }

    /// Resolves vanilla `AbstractClientPlayer.showExtraEars`: only a real player
    /// whose GameProfile name is exactly lowercase `deadmau5` enables
    /// `Deadmau5EarsLayer`.
    pub(super) fn resolve_player_extra_ears(&self, entity_id: i32) -> bool {
        let Some(identity) = self.entities.identity(entity_id) else {
            return false;
        };
        identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && self
                .player_info_entry(identity.uuid)
                .is_some_and(|entry| entry.profile.name == "deadmau5")
    }

    /// Resolves the vanilla `LivingEntityRenderer` sleeping bed orientation and head
    /// offset for an entity sleeping in a bed: `BedBlock.getBedOrientation` looks up
    /// the bed block at the synced sleeping position and reads its `FACING`. Returns
    /// the `sleepDirectionToRotation` yaw (degrees) and the world-space bed offset,
    /// or `None` when the sleeping position is not a bed (the renderer then falls
    /// back to the body yaw with no offset).
    pub(super) fn resolve_sleeping_bed(&self, entity_id: i32) -> Option<(f32, [f32; 2])> {
        let bed_pos = self.entities.sleeping_pos(entity_id)?;
        let probe = self.block_view(bed_pos)?;
        let block_name = probe.block_name?;
        let standing_eye_height = self.entities.standing_eye_height(entity_id)?;
        sleeping_bed_yaw_and_offset(block_name, probe.block_properties, standing_eye_height)
    }

    /// Vanilla `Camera.alignWithEntity` for a sleeping first-person camera:
    /// `getBedOrientation().toYRot() - 180`, or 0 off a bed (Camera.java:281-284).
    pub fn sleeping_camera_y_rot(&self, entity_id: i32) -> Option<f32> {
        let bed_pos = self.entities.sleeping_pos(entity_id)?;
        let to_y_rot = self
            .block_view(bed_pos)
            .filter(|bed| bed.block_name.is_some_and(block_name_is_bed))
            .and_then(|bed| match bed.block_properties.get("facing")?.as_str() {
                "south" => Some(0.0),
                "west" => Some(90.0),
                "north" => Some(180.0),
                "east" => Some(270.0),
                _ => None,
            });
        Some(to_y_rot.map_or(0.0, |y_rot: f32| y_rot - 180.0))
    }

    pub fn entity_transforms(&self) -> Vec<EntityTransformState> {
        self.entities.transform_states()
    }

    pub fn entity_type_name(&self, id: i32) -> Option<&'static str> {
        let type_id = self.entity_type_id(id)?;
        bbb_protocol::entity_types::vanilla_entity_type(type_id).map(|entry| entry.resource_id)
    }

    /// Lists entities sorted by distance from the local player, including
    /// non-pickable dropped items with their resolved item names.
    pub fn nearby_entities(
        &self,
        max_distance: f64,
        limit: usize,
        include_local_player: bool,
    ) -> Vec<NearbyEntityProbe> {
        let max_distance = if max_distance.is_finite() {
            max_distance.max(0.0)
        } else {
            0.0
        };
        if limit == 0 {
            return Vec::new();
        }
        let origin = self
            .local_player_pose()
            .map(|pose| [pose.position.x, pose.position.y, pose.position.z])
            .unwrap_or([0.0, 0.0, 0.0]);
        let local_id = self.local_player_id();
        let max_dist_sq = max_distance * max_distance;
        let mut results = Vec::new();
        for transform in self.entities.transform_states() {
            if !include_local_player && local_id == Some(transform.id) {
                continue;
            }
            let dx = transform.position.x - origin[0];
            let dy = transform.position.y - origin[1];
            let dz = transform.position.z - origin[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq > max_dist_sq {
                continue;
            }
            let entity_type =
                bbb_protocol::entity_types::vanilla_entity_type(transform.entity_type_id)
                    .map(|entry| entry.resource_id.to_owned());
            let eye_height = self
                .probe_entity_camera_pose_at_partial_tick(transform.id, 1.0)
                .map(|pose| pose.eye_height)
                .or_else(|| {
                    bbb_protocol::entity_types::vanilla_entity_type_eye_height(
                        transform.entity_type_id,
                    )
                })
                .unwrap_or(0.0);
            let item = self
                .entities
                .item_stack_for_entity(transform.id)
                .filter(|stack| stack.count > 0)
                .map(|stack| NearbyEntityItemProbe {
                    item_id: stack.item_id,
                    item_name: stack.item_id.and_then(|id| {
                        bbb_protocol::item_types::vanilla_item_type(id)
                            .map(|entry| entry.resource_id.to_owned())
                    }),
                    count: stack.count,
                });
            results.push(NearbyEntityProbe {
                id: transform.id,
                uuid: transform.uuid,
                entity_type_id: transform.entity_type_id,
                entity_type,
                position: transform.position,
                delta_movement: transform.delta_movement,
                distance: dist_sq.sqrt(),
                y_rot: transform.y_rot,
                x_rot: transform.x_rot,
                eye_height,
                on_ground: transform.on_ground,
                pickable: self.probe_entity_pick_bounds(transform.id).is_some(),
                item,
            });
        }
        results.sort_by(|a, b| {
            a.distance
                .total_cmp(&b.distance)
                .then_with(|| a.id.cmp(&b.id))
        });
        results.truncate(limit);
        results
    }

    pub fn lightning_bolt_render_sources(&self) -> Vec<LightningBoltRenderSourceState> {
        self.entities.lightning_bolt_render_sources()
    }

    pub fn item_entity_stacks(&self) -> impl Iterator<Item = ItemEntityStackState<'_>> {
        self.item_stacks_with_sampled_light(self.entities.item_entity_stacks())
    }

    /// Projects one dropped or thrown item after renderer visibility has selected its entity.
    pub fn entity_item_stack(&self, id: i32) -> Option<ItemEntityStackState<'_>> {
        self.item_stacks_with_sampled_light(self.entities.item_stack(id).into_iter())
            .next()
    }

    pub fn item_frame_render_state(&self, id: i32) -> Option<ItemFrameRenderState<'_>> {
        let mut state = self.entities.item_frame_render_state(id)?;
        state.light = self
            .sample_block_light(entity_light_block_pos(state.center))
            .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
        Some(state)
    }

    pub fn firework_rocket_item_state(&self, id: i32) -> Option<FireworkRocketItemState<'_>> {
        let mut state = self.entities.firework_rocket_item_state(id)?;
        state.light = self
            .sample_block_light(entity_light_block_pos(state.position))
            .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
        Some(state)
    }

    /// The render state of every item-frame / glow-item-frame entity (center, facing, sampled light, item
    /// rotation, glow/invisible flags, framed item, map id). Drives the 3D item-frame render (vanilla
    /// `ItemFrameRenderer`).
    pub fn item_frame_render_states(&self) -> impl Iterator<Item = ItemFrameRenderState<'_>> {
        self.entities.item_frame_render_states().map(|mut state| {
            state.light = self
                .sample_block_light(entity_light_block_pos(state.center))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
            state
        })
    }

    /// The item a humanoid entity holds in its main (`off_hand = false`) or off hand, or `None` for an
    /// empty hand. Drives the third-person held-item 3D render.
    pub fn held_item(&self, id: i32, off_hand: bool) -> Option<&ProtocolItemStackSummary> {
        self.entities.held_item(id, off_hand)
    }

    /// The item in an arbitrary vanilla equipment slot, or `None` for an empty / absent slot. This is
    /// used by entity render layers whose source slot is not a hand, such as the copper golem's antenna
    /// block in `EquipmentSlot.SADDLE`.
    pub fn equipment_item(
        &self,
        id: i32,
        slot: ProtocolEquipmentSlot,
    ) -> Option<&ProtocolItemStackSummary> {
        self.entities.equipment_item(id, slot)
    }

    /// Collects the `DATA_ITEM_STACK` carried by every entity whose type id is in `type_ids`. The
    /// thrown-item projectiles (snowball, egg, ender pearl, potions, …) that vanilla's
    /// `ThrownItemRenderer` draws as an item sprite share the dropped item's data layout, so this feeds
    /// the same billboard path. The caller (which owns the vanilla type ids) passes the projectile set.
    pub fn item_stacks_for_entity_types<'a>(
        &'a self,
        type_ids: &'a [i32],
    ) -> impl Iterator<Item = ItemEntityStackState<'a>> + 'a {
        self.item_stacks_with_sampled_light(self.entities.item_stacks_for_entity_types(type_ids))
    }

    /// The item-model render state for firework rocket entities (vanilla `FireworkEntityRenderer`).
    /// Attached elytra-boost rockets intentionally do not render, matching
    /// `FireworkRocketEntity.shouldRender` / `shouldRenderAtSqrDistance`.
    pub fn firework_rocket_item_states(&self) -> impl Iterator<Item = FireworkRocketItemState<'_>> {
        self.entities.firework_rocket_item_states().map(|mut item| {
            item.light = self
                .sample_block_light(entity_light_block_pos(item.position))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
            item
        })
    }

    /// Firework rocket event `17` uses `ClientLevel.createFireworks` with the
    /// rocket's current position, delta movement, and decoded `minecraft:fireworks`
    /// explosion list.
    pub fn firework_rocket_explosion_particle_state(
        &self,
        id: i32,
    ) -> Option<FireworkRocketExplosionParticleState> {
        self.entities.firework_rocket_explosion_particle_state(id)
    }

    /// Fox entity event `45` breaks the item held in the fox's main hand at the
    /// mouth, using the current body pitch/yaw for the velocity fan.
    pub fn fox_eat_particle_state(&self, id: i32) -> Option<FoxEatParticleState> {
        self.entities.fox_eat_particle_state(id)
    }

    /// The item-cluster render state for ominous item spawner entities (vanilla
    /// `OminousItemSpawnerRenderer`), projected with entity `ageInTicks` for scale-in and spin.
    pub fn ominous_item_spawner_item_states_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> impl Iterator<Item = OminousItemSpawnerItemState<'_>> {
        self.entities
            .ominous_item_spawner_item_states_at_partial_tick(partial_ticks)
    }

    fn item_stacks_with_sampled_light<'a>(
        &'a self,
        items: impl Iterator<Item = ItemEntityStackState<'a>> + 'a,
    ) -> impl Iterator<Item = ItemEntityStackState<'a>> + 'a {
        items.map(move |mut item| {
            item.light = self
                .sample_block_light(entity_light_block_pos(item.position))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
            item
        })
    }
}
