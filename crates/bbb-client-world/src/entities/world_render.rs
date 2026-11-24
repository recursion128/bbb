use super::*;

impl WorldStore {
    pub fn probe_entity_camera_pose(&self, id: i32) -> Option<EntityCameraPoseState> {
        self.entities.camera_pose_state(id)
    }

    /// Mirrors `ClientLevel.getPlayerByUUID` for player-list face inversion:
    /// the profile UUID must resolve to a currently loaded player entity.
    pub fn has_loaded_player_entity(&self, profile_id: uuid::Uuid) -> bool {
        let Some(entity_id) = self.entities.protocol_id_for_uuid(profile_id) else {
            return false;
        };
        self.entities.entity_type_id(entity_id) == Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
    }

    pub fn probe_entity_camera_pose_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraPoseState> {
        self.entities
            .camera_pose_state_at_partial_tick(id, partial_ticks)
    }

    pub fn probe_entity_camera_hurt_state(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraHurtState> {
        self.entities.camera_hurt_state(id, partial_ticks)
    }

    pub fn probe_entity_camera_view_bob_state(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraViewBobState> {
        self.entities.camera_view_bob_state(id, partial_ticks)
    }

    /// The entity's canonical relative AABB without the spectator pick filter
    /// that `probe_entity_pick_bounds` applies for ray tracing.
    pub fn entity_pick_bounds(&self, id: i32) -> Option<EntityPickBoundsState> {
        self.entities.pick_bounds(id)
    }

    pub fn entity_pose_eye_height(&self, id: i32) -> Option<f32> {
        self.entities.pose_eye_height(id)
    }

    pub fn probe_entity_pick_bounds(&self, id: i32) -> Option<EntityPickBoundsState> {
        let identity = self.entities.identity(id)?;
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && self
                .player_info_entry(identity.uuid)
                .is_some_and(|info| info.is_spectator())
        {
            return None;
        }
        self.entities.pick_bounds(id)
    }

    pub fn entity_pick_targets(&self) -> Vec<EntityPickTargetState> {
        self.entity_pick_targets_at_partial_tick(1.0)
    }

    pub fn entity_pick_targets_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityPickTargetState> {
        self.entities
            .pick_targets_at_partial_tick_filtered(partial_ticks, |uuid| {
                self.player_info_entry(uuid)
                    .is_some_and(|info| info.is_spectator())
            })
    }

    pub fn entity_pick_targets_in_aabb_at_partial_tick(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
        partial_ticks: f32,
    ) -> Vec<EntityPickTargetState> {
        self.entities.pick_targets_in_aabb_at_partial_tick_filtered(
            query_min,
            query_max,
            partial_ticks,
            |uuid| {
                self.player_info_entry(uuid)
                    .is_some_and(|info| info.is_spectator())
            },
        )
    }

    pub fn entity_debug_hitbox_targets_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityPickTargetState> {
        self.entities
            .debug_hitbox_targets_at_partial_tick_filtered(partial_ticks, |uuid| {
                self.player_info_entry(uuid)
                    .is_some_and(|info| info.is_spectator())
            })
    }

    pub fn entity_debug_hitbox_targets_in_aabb_at_partial_tick(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
        partial_ticks: f32,
    ) -> Vec<EntityPickTargetState> {
        self.entities
            .debug_hitbox_targets_in_aabb_at_partial_tick_filtered(
                query_min,
                query_max,
                partial_ticks,
                |uuid| {
                    self.player_info_entry(uuid)
                        .is_some_and(|info| info.is_spectator())
                },
            )
    }

    /// Performs one narrow ECS query and annotates the spectator flag for
    /// render culling. No full model source, item/effect projection, or
    /// light/fluid sampling is performed here.
    pub fn entity_render_candidates_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityRenderCandidateState> {
        let mut candidates = Vec::new();
        self.fill_entity_render_candidates_at_partial_tick(partial_ticks, &mut candidates);
        candidates
    }

    /// Fills render candidates while retaining the caller's allocation.
    pub fn fill_entity_render_candidates_at_partial_tick(
        &self,
        partial_ticks: f32,
        candidates: &mut Vec<EntityRenderCandidateState>,
    ) {
        let game_time = self.world_time().map_or(0, |time| time.game_time);
        self.entities.fill_model_candidates_at_partial_tick(
            partial_ticks,
            game_time,
            self.is_feature_enabled("minecraft:minecart_improvements"),
            &self.registries,
            candidates,
        );
        for candidate in candidates {
            candidate.is_spectator = candidate.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                && self
                    .player_info_entry(candidate.uuid)
                    .is_some_and(|info| info.is_spectator());
        }
    }

    /// Re-samples candidates from the same world revision at a new partial tick.
    pub fn update_entity_render_candidate_positions(
        &self,
        partial_ticks: f32,
        candidates: &mut [EntityRenderCandidateState],
    ) {
        for candidate in &mut *candidates {
            candidate.position = self
                .entities
                .transform(candidate.entity_id)
                .expect("candidate belongs to this world revision")
                .render_position(partial_ticks);
        }
        self.entities.update_passenger_candidate_positions(
            partial_ticks,
            self.world_time().map_or(0, |time| time.game_time),
            self.is_feature_enabled("minecraft:minecart_improvements"),
            candidates,
            false,
        );
    }

    /// Projects current synchronized Display state only for already-visible
    /// candidates. Camera-dependent billboard transforms remain native-owned;
    /// display transformation interpolation remains in the animation cell.
    pub fn fill_display_entity_render_sources_for_candidates_at_partial_tick(
        &self,
        survivors: &[EntityRenderCandidateState],
        partial_ticks: f32,
        sources: &mut Vec<DisplayEntityRenderSourceState>,
    ) {
        sources.clear();
        sources.reserve(survivors.len());
        sources.extend(survivors.iter().filter_map(|candidate| {
            let mut source = self
                .entities
                .display_render_source_at_partial_tick(candidate.entity_id, partial_ticks)?;
            source.position = candidate.position;
            source.light = self
                .sample_block_light(entity_light_block_pos(source.position))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
            Some(source)
        }));
    }

    /// Full-projects only `survivors`, while retaining the complete narrow
    /// candidate set for relationship rules such as cats searching sleeping
    /// players. Each survivor is projected exactly once.
    pub fn entity_model_sources_for_candidates_at_partial_tick(
        &self,
        survivors: &[EntityRenderCandidateState],
        all_candidates: &[EntityRenderCandidateState],
        partial_ticks: f32,
    ) -> Vec<EntityRenderSource> {
        let mut sources = Vec::new();
        self.fill_entity_model_sources_for_candidates_at_partial_tick(
            survivors,
            all_candidates,
            partial_ticks,
            &mut sources,
        );
        sources
    }

    /// Full-projects survivors while retaining the caller's outer allocation.
    pub fn fill_entity_model_sources_for_candidates_at_partial_tick(
        &self,
        survivors: &[EntityRenderCandidateState],
        all_candidates: &[EntityRenderCandidateState],
        partial_ticks: f32,
        sources: &mut Vec<EntityRenderSource>,
    ) {
        sources.clear();
        sources.reserve(survivors.len());
        sources.extend(survivors.iter().copied().filter_map(|candidate| {
            self.project_entity_model_source(candidate, partial_ticks, all_candidates)
        }));
    }

    /// Extracts vanilla dispatcher shadow pieces for the exact visible model-source generation.
    #[allow(clippy::too_many_arguments)]
    pub fn fill_entity_shadow_render_sources_for_model_sources(
        &self,
        survivors: &[EntityRenderCandidateState],
        model_sources: &[EntityRenderSource],
        partial_ticks: f32,
        camera_position: EntityVec3,
        sky_darken: u8,
        shadows_enabled: bool,
        sources: &mut Vec<EntityShadowRenderSourceState>,
    ) {
        if !shadows_enabled {
            sources.clear();
            return;
        }
        shadows::fill_entity_shadow_render_sources(
            self,
            survivors,
            model_sources,
            partial_ticks,
            camera_position,
            sky_darken,
            sources,
        );
    }

    /// Whether an entity should appear in the rendered model-source set. Vanilla hides spectator
    /// players from the entity render list; every non-player and every non-spectator player is kept.
    pub(crate) fn entity_passes_spectator_filter(&self, entity_id: i32) -> bool {
        let Some(identity) = self.entities.identity(entity_id) else {
            return true;
        };
        identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
            || !self
                .player_info_entry(identity.uuid)
                .is_some_and(|info| info.is_spectator())
    }

    /// Mirrors `EntityGetter.isUnobstructed(null, shape)` for a prospective
    /// single-box block collision shape. Missing canonical entity dimensions
    /// keep the caller's local prediction explicit. The local player is
    /// tested by its pose bounds, which lead its entity record by a frame.
    pub(crate) fn local_block_placement_is_unobstructed(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Option<bool> {
        let obstructing = self
            .entities
            .block_placement_obstruction_ids_intersecting(query_min, query_max)?;
        if obstructing.into_iter().any(|entity_id| {
            Some(entity_id) != self.local_player_id
                && self.entity_passes_spectator_filter(entity_id)
        }) {
            return Some(false);
        }
        self.local_player_bounds_intersect(query_min, query_max)
            .map(|intersects| !intersects)
    }

    /// Mirrors the `getEntities(null, box).isEmpty()` half of
    /// `ArmorStandItem.useOn`, including Vanilla's spectator exclusion.
    pub(crate) fn local_non_spectator_entity_bounds_are_empty(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Option<bool> {
        let intersecting = self
            .entities
            .all_entity_bounds_intersection_ids(query_min, query_max)?;
        if intersecting.into_iter().any(|entity_id| {
            Some(entity_id) != self.local_player_id
                && self.entity_passes_spectator_filter(entity_id)
        }) {
            return Some(false);
        }
        self.local_player_bounds_intersect(query_min, query_max)
            .map(|intersects| !intersects)
    }

    /// Narrow single-entity variant of [`Self::entity_model_sources_at_partial_tick`]. Reuses the
    /// same target list, spectator filter, and per-entity projection helper as the list path, so the
    /// returned state is identical to the list entry for `entity_id` (or `None` when it is filtered
    /// out or absent). Avoids building the full projected `Vec` when only one entity is needed.
    pub fn entity_model_source_at_partial_tick(
        &self,
        entity_id: i32,
        partial_ticks: f32,
    ) -> Option<EntityRenderSource> {
        let candidates = self.entity_render_candidates_at_partial_tick(partial_ticks);
        let candidate = candidates
            .iter()
            .copied()
            .find(|candidate| candidate.entity_id == entity_id)?;
        if candidate.is_spectator {
            return None;
        }
        self.project_entity_model_source(candidate, partial_ticks, &candidates)
    }

    /// Per-entity projection shared by [`Self::entity_model_sources_at_partial_tick`] and
    /// [`Self::entity_model_source_at_partial_tick`]. Turns one model target into a fully populated
    /// [`EntityRenderSource`], sampling light, water overlap, and the
    /// boat/minecart/parrot/sleeping/cat specials. `targets` is the full (unfiltered) target list,
    /// needed by the cat "lying on a sleeping player" search.
    pub(super) fn project_entity_model_source(
        &self,
        target: EntityRenderCandidateState,
        partial_ticks: f32,
        targets: &[EntityRenderCandidateState],
    ) -> Option<EntityRenderSource> {
        let mut source = self.entities.model_source(
            target.entity_id,
            target.position,
            partial_ticks,
            self.is_feature_enabled("minecraft:minecart_improvements"),
            &self.registries,
            &self.items,
        )?;
        let sampled_light = if let Some((width, height)) =
            dimensions::armor_stand_marker_light_probe_dimensions(
                source.entity_type_id,
                &source.data_values,
            ) {
            // ArmorStand.getLightProbePosition (ArmorStand.java:639-660).
            let position = self
                .entities
                .transform(target.entity_id)
                .map_or(target.position, |transform| transform.position);
            self.sample_armor_stand_marker_light(position, width, height)
        } else {
            // Vanilla Entity.getLightProbePosition samples the current pose's eyes.
            let light_probe = EntityVec3 {
                y: target.position.y + f64::from(source.light_probe_eye_height?),
                ..target.position
            };
            self.sample_block_light(entity_light_block_pos(light_probe))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT)
        };
        source.render.light_coords =
            entity_light_coords(source.entity_type_id, &source.data_values, sampled_light);
        if let EntityKindRenderState::EnderDragon(EnderDragonRenderState {
            animation: Some(dragon),
            ..
        }) = &mut source.render.kind
        {
            dragon.distance_to_egg = self.ender_dragon_distance_to_egg_sqr(target.position);
        }
        if matches!(source.render.kind, EntityKindRenderState::Guardian(_)) {
            let eye_tracking = self.guardian_eye_tracking_source(&source, partial_ticks, targets);
            source.render.kind.guardian_mut().eye_tracking = eye_tracking;
        }
        // The rope states need world light samples at both endpoints, so the leash
        // projection lives here on the store-and-light layer rather than in `model_source`.
        source.render.leash_ropes = self
            .entity_leash_render_states_at_partial_tick(target.entity_id, partial_ticks)
            .and_then(|states| {
                let ropes = states
                    .iter()
                    .map(|state| LeashRopeRenderState {
                        offset: state.offset.map(|value| value as f32),
                        delta: [
                            (state.end[0] - state.start[0]) as f32,
                            (state.end[1] - state.start[1]) as f32,
                            (state.end[2] - state.start[2]) as f32,
                        ],
                        start_block_light: state.start_block_light,
                        start_sky_light: state.start_sky_light,
                        end_block_light: state.end_block_light,
                        end_sky_light: state.end_sky_light,
                        slack: state.slack,
                    })
                    .collect::<Vec<_>>();
                EntityLeashRopesRenderState::from_ropes(&ropes)
            });
        if source.render.invisible_to_player
            && super::content_facts::entity_is_living(
                source.entity_type_id,
                self.content_registry(),
            )
            && (self.local_player_is_spectator()
                || self.local_player_can_see_friendly_invisible(&source))
        {
            // Vanilla `Entity.isInvisibleTo(player)`: spectator viewers and same-team
            // viewers whose team enables `canSeeFriendlyInvisibles` can see invisible
            // living entities, so the renderer takes the force-transparent branch instead
            // of the hidden branch.
            source.render.invisible_to_player = false;
        }
        // Vanilla `LivingEntityRenderState.isInWater = entity.isInWater()`: project
        // the `wasTouchingWater` overlap from the entity's interpolated world AABB
        // (`position + EntityDimensions`) against the chunk fluid state.
        // Skip `Interaction` (non-rendered, arbitrary width/height) and use base
        // minecart collision dimensions instead of the `displayOffset` culling box.
        let (aabb_min, aabb_max) = if is_vanilla_minecart_type(source.entity_type_id) {
            let [width, height] =
                vanilla_entity_type_dimensions(source.entity_type_id).unwrap_or([0.98, 0.7]);
            let half_w = f64::from(width) * 0.5;
            (
                [
                    target.position.x - half_w,
                    target.position.y,
                    target.position.z - half_w,
                ],
                [
                    target.position.x + half_w,
                    target.position.y + f64::from(height),
                    target.position.z + half_w,
                ],
            )
        } else {
            (
                [
                    target.position.x + f64::from(target.bounds.min[0]),
                    target.position.y + f64::from(target.bounds.min[1]),
                    target.position.z + f64::from(target.bounds.min[2]),
                ],
                [
                    target.position.x + f64::from(target.bounds.max[0]),
                    target.position.y + f64::from(target.bounds.max[1]),
                    target.position.z + f64::from(target.bounds.max[2]),
                ],
            )
        };
        source.render.in_water = source.entity_type_id
            != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_INTERACTION_ID
            && crate::fluid::world_aabb_in_water(self, aabb_min, aabb_max);
        match &mut source.render.kind {
            // Vanilla `AbstractBoatRenderer.extractRenderState`: `state.isUnderWater`
            // comes from `AbstractBoat.isUnderWater()`, which is a top-slice water
            // surface test, not the broader `Entity.isInWater()` overlap above.
            EntityKindRenderState::Boat(boat) => {
                boat.underwater =
                    crate::fluid::world_aabb_boat_underwater(self, aabb_min, aabb_max);
            }
            EntityKindRenderState::Minecart(minecart) if !minecart.new_render => {
                if let Some(rail) = self.old_minecart_rail_render_state(source.position) {
                    minecart.pos_on_rail = Some(rail.pos_on_rail);
                    minecart.front_pos = Some(rail.front_pos);
                    minecart.back_pos = Some(rail.back_pos);
                    minecart.rail_offset = Some(rail.rail_offset);
                    minecart.rail_direction = Some(rail.rail_direction);
                }
            }
            // Vanilla `LevelEventHandler.playJukeboxSong` notifies nearby living entities,
            // and `Parrot.aiStep` keeps PARTY only while its jukebox block position is within
            // `BlockPos.closerToCenterThan(entity.position(), 3.46)`.
            EntityKindRenderState::Parrot(parrot) => {
                parrot.party = self.parrot_party_near_playing_jukebox(target.position);
            }
            _ => {}
        }
        if source.is_sleeping {
            if let Some((yaw, offset)) = self.resolve_sleeping_bed(target.entity_id) {
                source.sleeping_bed_yaw = Some(yaw);
                source.sleeping_bed_offset = offset;
            }
        }
        if matches!(source.render.kind, EntityKindRenderState::Feline(_)) {
            let lying = self.cat_is_lying_on_top_of_sleeping_player(&source, target, targets);
            source
                .render
                .kind
                .feline_mut()
                .is_lying_on_top_of_sleeping_player = lying;
        }
        if source.render.upside_down_height.is_none()
            && self.resolve_player_upside_down(target.entity_id)
        {
            source.render.upside_down_height = Some(source.render.bounding_box_height);
        }
        source.render.outline_color = self.entity_outline_color(&source);
        source.render.appears_glowing = source.render.outline_color != 0;
        if let EntityKindRenderState::Player(player) = &mut source.render.kind {
            player.show_extra_ears = self.resolve_player_extra_ears(target.entity_id);
        }
        Some(source)
    }

    /// Mirrors `GuardianRenderer.getEntityToLookAt` and its render-state extraction. An active
    /// attack-target id deliberately does not fall back to the camera when the referenced entity is
    /// missing or is not living: Vanilla's `getActiveAttackTarget()` returns `null` in that case.
    pub(super) fn guardian_eye_tracking_source(
        &self,
        source: &EntityRenderSource,
        partial_ticks: f32,
        targets: &[EntityRenderCandidateState],
    ) -> Option<GuardianEyeTrackingRenderState> {
        if !vanilla_entity_metadata_type_is_guardian(source.entity_type_id) {
            return None;
        }
        let guardian =
            self.probe_entity_camera_pose_at_partial_tick(source.entity_id, partial_ticks)?;
        let attack_target_id =
            guardian_attack_target_id(source.entity_type_id, &source.data_values);
        let look_at_id = if attack_target_id != 0 {
            let target = targets
                .iter()
                .find(|target| target.entity_id == attack_target_id)?;
            if !vanilla_living_entity_type(target.entity_type_id) {
                return None;
            }
            attack_target_id
        } else {
            self.camera_entity_id()?
        };
        let look_at = self.probe_entity_camera_pose_at_partial_tick(look_at_id, partial_ticks)?;
        let look_direction = calculate_view_vector(guardian.x_rot, guardian.y_rot);
        Some(GuardianEyeTrackingRenderState {
            eye_position: [
                guardian.position.x,
                guardian.position.y + f64::from(guardian.eye_height),
                guardian.position.z,
            ],
            look_direction: [look_direction.x, look_direction.y, look_direction.z],
            look_at_position: [
                look_at.position.x,
                look_at.position.y + f64::from(look_at.eye_height),
                look_at.position.z,
            ],
        })
    }

    pub(super) fn old_minecart_rail_render_state(
        &self,
        position: EntityVec3,
    ) -> Option<MinecartRailRenderState> {
        let pos_on_rail = self.old_minecart_get_pos(position.x, position.y, position.z)?;
        let front_pos = self
            .old_minecart_get_pos_offs(position.x, position.y, position.z, 0.3)
            .unwrap_or(pos_on_rail);
        let back_pos = self
            .old_minecart_get_pos_offs(position.x, position.y, position.z, -0.3)
            .unwrap_or(pos_on_rail);
        Some(MinecartRailRenderState {
            pos_on_rail: vec3_f64_to_f32_array(pos_on_rail),
            front_pos: vec3_f64_to_f32_array(front_pos),
            back_pos: vec3_f64_to_f32_array(back_pos),
            rail_offset: vec3_f64_to_f32_array([
                pos_on_rail[0] - position.x,
                (front_pos[1] + back_pos[1]) * 0.5 - position.y,
                pos_on_rail[2] - position.z,
            ]),
            rail_direction: vec3_f64_to_f32_array([
                back_pos[0] - front_pos[0],
                back_pos[1] - front_pos[1],
                back_pos[2] - front_pos[2],
            ]),
        })
    }

    /// Vanilla `OldMinecartBehavior.getPosOffs`: sample the current rail, step
    /// along its exit direction by `offs`, adjust the query Y for slopes, then
    /// resolve through `getPos`.
    pub(super) fn old_minecart_get_pos_offs(
        &self,
        mut x: f64,
        mut y: f64,
        mut z: f64,
        offs: f64,
    ) -> Option<[f64; 3]> {
        let xt = floor_to_i32(x);
        let mut yt = floor_to_i32(y);
        let zt = floor_to_i32(z);
        if self
            .rail_shape_at(BlockPos {
                x: xt,
                y: yt - 1,
                z: zt,
            })
            .is_some()
        {
            yt -= 1;
        }

        let shape = self.rail_shape_at(BlockPos {
            x: xt,
            y: yt,
            z: zt,
        })?;
        y = f64::from(yt);
        if shape.is_slope() {
            y = f64::from(yt + 1);
        }
        let (exit0, exit1) = shape.exits();
        let mut x_d = f64::from(exit1[0] - exit0[0]);
        let mut z_d = f64::from(exit1[2] - exit0[2]);
        let dd = (x_d * x_d + z_d * z_d).sqrt();
        x_d /= dd;
        z_d /= dd;
        x += x_d * offs;
        z += z_d * offs;

        if exit0[1] != 0 && floor_to_i32(x) - xt == exit0[0] && floor_to_i32(z) - zt == exit0[2] {
            y += f64::from(exit0[1]);
        } else if exit1[1] != 0
            && floor_to_i32(x) - xt == exit1[0]
            && floor_to_i32(z) - zt == exit1[2]
        {
            y += f64::from(exit1[1]);
        }

        self.old_minecart_get_pos(x, y, z)
    }

    /// Vanilla `OldMinecartBehavior.getPos`: project the minecart's interpolated
    /// entity position onto the current rail segment, including the rail's
    /// `0.0625` Y lift and slope endpoint correction.
    pub(super) fn old_minecart_get_pos(&self, mut x: f64, y: f64, mut z: f64) -> Option<[f64; 3]> {
        let xt = floor_to_i32(x);
        let mut yt = floor_to_i32(y);
        let zt = floor_to_i32(z);
        if self
            .rail_shape_at(BlockPos {
                x: xt,
                y: yt - 1,
                z: zt,
            })
            .is_some()
        {
            yt -= 1;
        }

        let shape = self.rail_shape_at(BlockPos {
            x: xt,
            y: yt,
            z: zt,
        })?;
        let (exit0, exit1) = shape.exits();
        let x0 = f64::from(xt) + 0.5 + f64::from(exit0[0]) * 0.5;
        let y0 = f64::from(yt) + 0.0625 + f64::from(exit0[1]) * 0.5;
        let z0 = f64::from(zt) + 0.5 + f64::from(exit0[2]) * 0.5;
        let x1 = f64::from(xt) + 0.5 + f64::from(exit1[0]) * 0.5;
        let y1 = f64::from(yt) + 0.0625 + f64::from(exit1[1]) * 0.5;
        let z1 = f64::from(zt) + 0.5 + f64::from(exit1[2]) * 0.5;
        let x_d = x1 - x0;
        let y_d = (y1 - y0) * 2.0;
        let z_d = z1 - z0;
        let progress = if x_d == 0.0 {
            z - f64::from(zt)
        } else if z_d == 0.0 {
            x - f64::from(xt)
        } else {
            let xx = x - x0;
            let zz = z - z0;
            (xx * x_d + zz * z_d) * 2.0
        };

        x = x0 + x_d * progress;
        let mut projected_y = y0 + y_d * progress;
        z = z0 + z_d * progress;
        if y_d < 0.0 {
            projected_y += 1.0;
        } else if y_d > 0.0 {
            projected_y += 0.5;
        }
        Some([x, projected_y, z])
    }

    pub(super) fn rail_shape_at(&self, pos: BlockPos) -> Option<MinecartRailShape> {
        let block = self.block_view(pos)?;
        if !block_name_is_rail(block.block_name?) {
            return None;
        }
        MinecartRailShape::from_vanilla_name(block.block_properties.get("shape")?)
    }

    pub(super) fn sample_armor_stand_marker_light(
        &self,
        position: EntityVec3,
        width: f32,
        height: f32,
    ) -> TerrainLight {
        let half_width = f64::from(width) * 0.5;
        let min_x = floor_to_i32(position.x - half_width);
        let min_y = floor_to_i32(position.y);
        let min_z = floor_to_i32(position.z - half_width);
        let max_x = floor_to_i32(position.x + half_width);
        let max_y = floor_to_i32(position.y + f64::from(height));
        let max_z = floor_to_i32(position.z + half_width);
        let mut probe_pos = entity_light_block_pos(position);
        let mut brightest = i32::MIN;
        'scan: for z in min_z..=max_z {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let pos = BlockPos { x, y, z };
                    let light = self
                        .sample_block_light(pos)
                        .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
                    let brightness = i32::from(light.block.max(light.sky));
                    if brightness == 15 {
                        probe_pos = pos;
                        break 'scan;
                    }
                    if brightness > brightest {
                        brightest = brightness;
                        probe_pos = pos;
                    }
                }
            }
        }
        self.sample_block_light(probe_pos)
            .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT)
    }

    pub(super) fn parrot_party_near_playing_jukebox(&self, position: EntityVec3) -> bool {
        const VANILLA_PARROT_PARTY_JUKEBOX_DISTANCE: f64 = 3.46;
        self.playing_jukebox_songs().iter().any(|song| {
            block_pos_closer_to_center_than(
                song.pos,
                position,
                VANILLA_PARROT_PARTY_JUKEBOX_DISTANCE,
            )
        })
    }

    pub(super) fn local_player_can_see_friendly_invisible(
        &self,
        source: &EntityRenderSource,
    ) -> bool {
        let Some(local_player_name) = self.local_player_scoreboard_name() else {
            return false;
        };
        let target_name = self.entity_scoreboard_name(source);
        self.scoreboard
            .same_team_can_see_friendly_invisibles(&local_player_name, &target_name)
    }

    pub fn local_player_scoreboard_name(&self) -> Option<String> {
        let local_player_id = self.local_player_id?;
        let identity = self.entities.identity(local_player_id)?;
        Some(self.scoreboard_name_for_identity(identity.entity_type_id, identity.uuid))
    }

    /// Returns the RGB color supplied by the entity's scoreboard team formatting.
    ///
    /// The caller decides whether the entity is a living item-model owner. Vanilla's
    /// team item tint receives no owner for non-living entities.
    pub fn entity_team_color_rgb(&self, entity_id: i32) -> Option<u32> {
        let identity = self.entities.identity(entity_id)?;
        let scoreboard_name =
            self.scoreboard_name_for_identity(identity.entity_type_id, identity.uuid);
        self.scoreboard
            .team_color_rgb_for_scoreboard_name(&scoreboard_name)
    }

    /// Returns the local player's scoreboard team formatting RGB, when both the
    /// player identity and a color-bearing team formatting are present.
    pub fn local_player_team_color_rgb(&self) -> Option<u32> {
        let scoreboard_name = self.local_player_scoreboard_name()?;
        self.scoreboard
            .team_color_rgb_for_scoreboard_name(&scoreboard_name)
    }

    pub(super) fn entity_scoreboard_name(&self, source: &EntityRenderSource) -> String {
        self.scoreboard_name_for_identity(source.entity_type_id, source.uuid)
    }

    pub(crate) fn scoreboard_name_for_identity(
        &self,
        entity_type_id: i32,
        uuid: uuid::Uuid,
    ) -> String {
        if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            if let Some(player) = self.player_info_entry(uuid) {
                return player.profile.name.clone();
            }
        }

        uuid.to_string()
    }
}
