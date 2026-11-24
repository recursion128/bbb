use super::*;

impl WorldStore {
    pub fn local_player_id(&self) -> Option<i32> {
        self.local_player_id
    }

    /// The `Entity.DATA_SHARED_FLAGS_ID` byte this client last received for an
    /// entity, or the field's default while the server has published none.
    ///
    /// `ServerEntity.sendDirtyEntityData` (ServerEntity.java:336-342) is the
    /// only thing that publishes it, so this is everything a client knows
    /// about another player's sneak, sprint and glide state. `None` means the
    /// entity itself is unknown.
    pub fn entity_shared_flags(&self, id: i32) -> Option<i8> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        self.entities.metadata_byte_for_entity(
            id,
            accessors.shared_flags_data_id,
            accessors.shared_flags_default,
        )
    }

    pub fn local_player_main_arm_left(&self) -> Option<bool> {
        self.entities.main_arm_left(self.local_player_id?)
    }

    pub fn local_player_attack_swing(
        &self,
        partial_ticks: f32,
    ) -> Option<LocalPlayerAttackSwingState> {
        self.entities
            .attack_swing_state(self.local_player_id?, partial_ticks)
    }

    pub fn local_player_ticks_since_kinetic_hit_feedback(&self, partial_ticks: f32) -> f32 {
        self.local_player_id
            .and_then(|id| {
                self.entities
                    .ticks_since_kinetic_hit_feedback(id, partial_ticks)
            })
            .unwrap_or(0.0)
    }

    /// The fishing bobber currently owned by the local player, if any. Vanilla
    /// `FishingHook.getAddEntityPacket` stores the owner id in
    /// `ClientboundAddEntityPacket.data`, and `FishingHook.setOwner` writes it
    /// back to `Player.fishing` on the client.
    pub fn local_player_fishing_bobber_id(&self) -> Option<i32> {
        let local_player_id = self.local_player_id?;
        self.entities.first_entity_id_with_type_and_data(
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            local_player_id,
        )
    }

    /// Vanilla `FishingHookRenderer.extractRenderState` inputs for one tracked
    /// fishing bobber and its player owner, sampled at the same partial tick.
    pub fn fishing_hook_render_source_at_partial_tick(
        &self,
        hook_id: i32,
        partial_ticks: f32,
    ) -> Option<FishingHookRenderSourceState> {
        let partial_ticks = partial_ticks.clamp(0.0, 1.0);
        let mut source = self
            .entities
            .fishing_hook_render_source_at_partial_tick(hook_id, partial_ticks)?;
        let game_time = self.world_time().map_or(0, |time| time.game_time);
        let minecart_improvements_enabled =
            self.is_feature_enabled("minecraft:minecart_improvements");
        source.hook_position = self.entities.render_position_at_partial_tick(
            hook_id,
            partial_ticks,
            game_time,
            minecart_improvements_enabled,
        )?;
        let owner_transform = self.entities.transform(source.owner_id)?;
        let owner_position = owner_transform.render_position(partial_ticks);
        let projected_owner_position = self.entities.render_position_at_partial_tick(
            source.owner_id,
            partial_ticks,
            game_time,
            minecart_improvements_enabled,
        )?;
        source.owner_eye_position.x += projected_owner_position.x - owner_position.x;
        source.owner_eye_position.y += projected_owner_position.y - owner_position.y;
        source.owner_eye_position.z += projected_owner_position.z - owner_position.z;
        Some(source)
    }

    pub fn local_player_vehicle_id(&self) -> Option<i32> {
        self.local_player_vehicle_id
    }

    pub fn removed_player_vehicle_id(&self) -> Option<i32> {
        self.removed_player_vehicle_id
    }

    /// Vanilla `Gui.getPlayerVehicleWithHealth` + the `extractVehicleHealth` /
    /// `getVehicleMaxHearts` inputs (Gui.java:709-737,974-979): the local
    /// player's direct vehicle when it is a `LivingEntity` (which is exactly
    /// vanilla's `showVehicleHealth()` — the `Entity` base returns
    /// `this instanceof LivingEntity` (Entity.java:2349-2351) and 26.1 has no
    /// overrides). `health` is the synced `DATA_HEALTH_ID` float
    /// (`LivingEntity.getHealth`); `max_health` is the synced MAX_HEALTH
    /// attribute folded through `AttributeInstance.calculateValue`
    /// (`vehicle.getMaxHealth()`, Gui.java:727). Boats/minecarts (non-living
    /// vehicles) yield `None`, matching vanilla's hidden vehicle bar.
    pub fn local_player_vehicle_health(&self) -> Option<LocalPlayerVehicleHealth> {
        self.vehicle_health(self.local_player_vehicle_id?)
    }

    pub(super) fn vehicle_health(&self, vehicle_id: i32) -> Option<LocalPlayerVehicleHealth> {
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if !super::content_facts::entity_is_living(entity_type_id, self.content_registry()) {
            return None;
        }
        let health = self.entities.living_entity_health(vehicle_id)?;
        // Vanilla reads the attribute map; every living entity's syncable
        // MAX_HEALTH instance is sent on tracking start
        // (`ServerEntity.sendPairingData`, ServerEntity.java:282-284). The
        // 20.0 fallback is the `Attributes.MAX_HEALTH` `RangedAttribute`
        // default (Attributes.java:58-60) for the not-yet-synced window.
        let max_health = self
            .entities
            .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .unwrap_or(VANILLA_DEFAULT_MAX_HEALTH);
        Some(LocalPlayerVehicleHealth { health, max_health })
    }

    /// Vanilla `player.getAttributeValue(Attributes.MAX_HEALTH)`, the max the
    /// heart row scales its container count / row count against
    /// (`Gui.extractPlayerHealth`, Gui.java:768). Reads the synced MAX_HEALTH
    /// attribute (registry index 19) folded through
    /// `AttributeInstance.calculateValue`, falling back to the
    /// `Attributes.MAX_HEALTH` `RangedAttribute` default 20.0 while unsynced.
    pub fn local_player_max_health(&self) -> f32 {
        self.local_player_id
            .map_or(VANILLA_DEFAULT_MAX_HEALTH as f32, |id| {
                self.player_max_health(id)
            })
    }

    pub fn player_max_health(&self, id: i32) -> f32 {
        self.entities
            .attribute_value(id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .unwrap_or(VANILLA_DEFAULT_MAX_HEALTH) as f32
    }

    /// Vanilla `player.isFullyFrozen()` (`getTicksFrozen() >= 140`), the
    /// `Gui.HeartType.forPlayer` frozen-heart gate (Gui.java:1444). Reads the
    /// local player's synced `DATA_TICKS_FROZEN_ID`.
    pub fn local_player_is_fully_frozen(&self) -> bool {
        self.local_player_id
            .is_some_and(|id| self.player_is_fully_frozen(id))
    }

    pub fn player_is_fully_frozen(&self, id: i32) -> bool {
        self.entities.is_fully_frozen(id)
    }

    /// Vanilla `LivingEntity.getHealth()` for any camera player: the synced
    /// `DATA_HEALTH_ID` float. The local player's canonical health stays the
    /// `ClientboundSetHealthPacket` value instead.
    pub fn player_synced_health(&self, id: i32) -> Option<f32> {
        self.entities.living_entity_health(id)
    }

    pub fn hurting_projectile(&self, id: i32) -> Option<HurtingProjectileState> {
        self.entities.hurting_projectile(id)
    }

    pub fn last_projectile_power_update(&self) -> Option<&ProjectilePowerUpdateState> {
        self.last_projectile_power.as_ref()
    }

    pub(crate) fn update_entity_count(&mut self) {
        // The counter is the server-tracked set; the local player is not one.
        let local_player = self
            .local_player_id
            .is_some_and(|id| self.entities.contains(id));
        self.counters.entities_tracked = self.entities.len() - usize::from(local_player);
    }
}
