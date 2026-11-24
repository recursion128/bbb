use super::*;

impl WorldStore {
    /// Mirrors `BlockState.isFaceSturdy(level, pos, direction,
    /// SupportType.FULL)` for `SolidFaceRenderer`'s vanilla direction order:
    /// west, south, east, north, down, up.
    pub fn debug_block_sturdy_faces(&self, block: &BlockProbe) -> [bool; 6] {
        self.block_sturdy_faces(&block.view())
    }

    pub(crate) fn block_sturdy_faces(&self, block: &BlockView) -> [bool; 6] {
        let Some(block_name) = block.block_name else {
            return [false; 6];
        };
        if is_soul_sand_or_mud_block_name(block_name) {
            return [true; 6];
        }
        if is_leaves_block_name(block_name)
            || block_name == "minecraft:chorus_flower"
            || is_hanging_sign_block_name(block_name)
            || is_fence_gate_block_name(block_name)
        {
            return [false; 6];
        }
        if block_name == "minecraft:snow" {
            let full = block
                .block_properties
                .get("layers")
                .and_then(|value| value.parse::<u8>().ok())
                == Some(8);
            return if full {
                [true; 6]
            } else {
                [false, false, false, false, true, false]
            };
        }
        if is_shulker_box_block_name(block_name) {
            let open = self
                .shulker_box_lids
                .iter()
                .find(|lid| lid.pos == block.pos)
                .is_some_and(|lid| lid.status != crate::ShulkerBoxAnimationStatus::Closed);
            if !open {
                return [true; 6];
            }
            let Some(support_direction) = block
                .block_properties
                .get("facing")
                .and_then(|facing| BlockDirection::parse(facing))
                .map(BlockDirection::opposite)
            else {
                return [false; 6];
            };
            return DEBUG_SOLID_FACE_DIRECTIONS.map(|direction| direction == support_direction);
        }

        block_collision_shape_empty_context(block, block.pos)
            .map(BlockCollisionShape::full_face_flags)
            .unwrap_or([false; 6])
    }

    pub(crate) fn block_has_full_sturdy_face(
        &self,
        block: &BlockView,
        direction: ProtocolDirection,
    ) -> bool {
        let direction = block_direction(direction);
        DEBUG_SOLID_FACE_DIRECTIONS
            .into_iter()
            .position(|candidate| candidate == direction)
            .is_some_and(|index| self.block_sturdy_faces(block)[index])
    }

    /// Mirrors cached `BlockState.isFaceSturdy(direction, SupportType.FULL)`;
    /// dynamic states fall back to their live empty-context support shape.
    pub(crate) fn block_full_face_sturdy(
        &self,
        block: &BlockView,
        direction: ProtocolDirection,
    ) -> Option<bool> {
        self.block_full_face_sturdy_faces(block)
            .map(|faces| faces[protocol_direction_index(direction)])
    }

    /// All cached FULL support results in Vanilla `Direction.values()` order.
    pub(crate) fn block_full_face_sturdy_faces(&self, block: &BlockView) -> Option<[bool; 6]> {
        let facts = vanilla_block_state_support_facts(block.block_state_id)?;
        if facts.dynamic_shape() {
            let [west, south, east, north, down, up] = self.block_sturdy_faces(block);
            Some([down, up, north, south, west, east])
        } else {
            Some(VANILLA_PROTOCOL_DIRECTIONS.map(|direction| {
                facts.face_sturdy(
                    vanilla_support_direction(direction),
                    VanillaBlockSupportType::Full,
                )
            }))
        }
    }

    /// Mirrors `SupportType.CENTER` for the vertical support queries used by
    /// candles, lanterns, and ceiling hanging signs. Dynamic shulker support
    /// is its one full attachment face; the remaining dynamic blocks use
    /// their live empty-context block-support shape.
    pub(crate) fn block_has_vertical_center_sturdy_face(
        &self,
        block: &BlockView,
        direction: ProtocolDirection,
    ) -> bool {
        if !matches!(direction, ProtocolDirection::Down | ProtocolDirection::Up) {
            return false;
        }
        if block.block_name.is_some_and(is_shulker_box_block_name) {
            return self.block_has_full_sturdy_face(block, direction);
        }
        block_collision_shape_empty_context(block, block.pos)
            .is_some_and(|shape| shape.has_center_face(block_direction(direction)))
    }

    /// Mirrors the vertical `SupportType.RIGID` query used by rail survival.
    pub(crate) fn block_has_vertical_rigid_sturdy_face(
        &self,
        block: &BlockView,
        direction: ProtocolDirection,
    ) -> bool {
        if !matches!(direction, ProtocolDirection::Down | ProtocolDirection::Up) {
            return false;
        }
        if block.block_name.is_some_and(is_shulker_box_block_name) {
            return self.block_has_full_sturdy_face(block, direction);
        }
        block_collision_shape_empty_context(block, block.pos)
            .is_some_and(|shape| shape.has_rigid_face(block_direction(direction)))
    }

    pub(crate) fn block_has_nonempty_collision_face(
        &self,
        block: &BlockView,
        direction: ProtocolDirection,
    ) -> bool {
        block_collision_shape_empty_context(block, block.pos)
            .is_some_and(|shape| shape.has_nonempty_face(block_direction(direction)))
    }

    /// Whether the empty-context collision face covers one exact local
    /// rectangle. Wall placement uses this for the vanilla post and side
    /// probes against the block above.
    pub(crate) fn block_empty_context_collision_face_covers_rectangle(
        &self,
        block: &BlockView,
        direction: ProtocolDirection,
        target: [f64; 4],
    ) -> bool {
        block_collision_shape_empty_context(block, block.pos)
            .is_some_and(|shape| shape.covers_face_rectangle(block_direction(direction), target))
    }

    /// Mirrors `MultifaceBlock.canAttachTo`: either the block-support shape or
    /// the empty-context collision shape may provide the requested full face.
    pub(crate) fn block_has_full_support_or_collision_face(
        &self,
        block: &BlockView,
        face_index: usize,
    ) -> bool {
        self.block_sturdy_faces(block)
            .get(face_index)
            .copied()
            .unwrap_or(false)
            || block_collision_shape_empty_context(block, block.pos).is_some_and(|shape| {
                shape
                    .full_face_flags()
                    .get(face_index)
                    .copied()
                    .unwrap_or(false)
            })
    }

    /// Mirrors `level.getEntities(cameraEntity,
    /// cameraEntity.getBoundingBox().inflate(radius))` for
    /// `SupportBlockRenderer`'s cached nearby-entity membership.
    pub fn debug_supporting_block_entity_ids(&self, radius: f64) -> Vec<i32> {
        if !radius.is_finite() || radius < 0.0 {
            return Vec::new();
        }
        let Some(camera) = self.debug_camera_collision_query() else {
            return Vec::new();
        };
        let query = camera.bounds.inflated(radius);
        self.entity_transforms()
            .into_iter()
            .filter(|transform| camera.source_entity_id != Some(transform.id))
            .filter_map(|transform| {
                let bounds = self.probe_entity_pick_bounds(transform.id)?;
                world_collision_boxes_intersect(
                    WorldCollisionBox {
                        min: [
                            transform.position.x + f64::from(bounds.min[0]),
                            transform.position.y + f64::from(bounds.min[1]),
                            transform.position.z + f64::from(bounds.min[2]),
                        ],
                        max: [
                            transform.position.x + f64::from(bounds.max[0]),
                            transform.position.y + f64::from(bounds.max[1]),
                            transform.position.z + f64::from(bounds.max[2]),
                        ],
                    },
                    query,
                )
                .then_some(transform.id)
            })
            .collect()
    }

    pub fn entity_main_supporting_block_pos(&self, entity_id: i32) -> Option<BlockPos> {
        self.entities.supporting_block(entity_id)?.main
    }

    pub fn debug_entity_supporting_blocks(
        &self,
        entity_id: i32,
    ) -> Option<DebugEntitySupportingBlocks> {
        if self.local_player_id() == Some(entity_id) {
            return None;
        }
        let supporting_pos = self.entity_main_supporting_block_pos(entity_id)?;
        let entity_y = self.entities.transform(entity_id)?.position.y;
        let supporting = self.debug_supporting_block_highlight(supporting_pos);
        let legacy_pos = self.legacy_on_pos(
            Some(supporting_pos),
            crate::EntityVec3 {
                x: f64::from(supporting_pos.x),
                y: entity_y,
                z: f64::from(supporting_pos.z),
            },
        );
        let legacy_effect = (legacy_pos != supporting_pos)
            .then(|| self.debug_supporting_block_highlight(legacy_pos));
        Some(DebugEntitySupportingBlocks {
            entity_id,
            supporting,
            legacy_effect,
        })
    }

    /// Mirrors the `Entity.setOnGround` -> `checkSupportingBlock(onGround,
    /// null)` call made by vanilla's three remote-entity position packet
    /// handlers. The source-specific collision-context refinements are kept at
    /// this single world boundary for later expansion.
    pub(crate) fn update_entity_supporting_block_from_packet(
        &mut self,
        entity_id: i32,
        on_ground: bool,
    ) {
        if self.local_player_id() == Some(entity_id) {
            return;
        }
        let Some(previous) = self.entities.supporting_block(entity_id) else {
            return;
        };
        if !on_ground {
            let _ = self
                .entities
                .with_supporting_block_mut(entity_id, |support| {
                    support.clear_physical();
                });
            return;
        }
        let Some(transform) = self.entities.transform(entity_id) else {
            return;
        };
        let Some(bounds) = self.entities.pick_bounds(entity_id) else {
            return;
        };
        let world_bounds = WorldCollisionBox {
            min: [
                transform.position.x + f64::from(bounds.min[0]),
                transform.position.y + f64::from(bounds.min[1]),
                transform.position.z + f64::from(bounds.min[2]),
            ],
            max: [
                transform.position.x + f64::from(bounds.max[0]),
                transform.position.y + f64::from(bounds.max[1]),
                transform.position.z + f64::from(bounds.max[2]),
            ],
        };
        let current = find_supporting_block(
            self,
            [
                transform.position.x,
                transform.position.y,
                transform.position.z,
            ],
            world_bounds,
            LocalPlayerCollisionContext::for_remote_entity(
                world_bounds.min[1],
                self.entities.is_descending(entity_id).unwrap_or(false),
                self.remote_entity_can_walk_on_powder_snow(entity_id),
                self.entity_type_id(entity_id) == Some(VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID),
            )
            .with_can_stand_on_lava(
                self.entity_type_id(entity_id) == Some(VANILLA_ENTITY_TYPE_STRIDER_ID),
            ),
        );
        let main = if current.is_some() || previous.on_ground_no_blocks {
            current
        } else {
            previous.main
        };
        let _ = self
            .entities
            .with_supporting_block_mut(entity_id, |support| {
                support.main = main;
                support.on_ground_no_blocks = current.is_none();
            });
    }

    pub(super) fn remote_entity_can_walk_on_powder_snow(&self, entity_id: i32) -> bool {
        let Some(entity_type_id) = self.entity_type_id(entity_id) else {
            return false;
        };
        if vanilla_entity_type_is_powder_snow_walkable(entity_type_id) {
            return true;
        }
        vanilla_living_entity_type(entity_type_id)
            && self
                .entities
                .equipment_item(entity_id, ProtocolEquipmentSlot::Feet)
                .filter(|item| item.count > 0)
                .and_then(|item| item.item_id)
                .is_some_and(|item_id| {
                    self.items
                        .powder_snow_walkable_foot_item_ids
                        .contains(&item_id)
                })
    }

    /// Projects the local-player half of vanilla `SupportBlockRenderer`.
    ///
    /// `Entity.checkSupportingBlock` probes the one-microunit slice directly
    /// below the entity and selects the intersecting block nearest to the
    /// entity position, breaking exact ties toward the greatest `(y, z, x)`
    /// `BlockPos`. The debug renderer draws this state only while
    /// `mainSupportingBlockPos` is present.
    pub fn debug_local_player_supporting_blocks(&self) -> Option<DebugLocalSupportingBlocks> {
        let pose = self.local_player_pose()?;
        if !pose.on_ground {
            return None;
        }
        let bounds = self.local_player_bounds(pose);
        let supporting_pos = find_supporting_block(
            self,
            [pose.position.x, pose.position.y, pose.position.z],
            bounds,
            LocalPlayerCollisionContext::for_pose(self, pose, false),
        )?;
        let supporting = self.debug_supporting_block_highlight(supporting_pos);
        let legacy_pos = self.legacy_on_pos(
            Some(supporting_pos),
            crate::EntityVec3 {
                x: pose.position.x,
                y: pose.position.y,
                z: pose.position.z,
            },
        );
        let legacy_effect = (legacy_pos != supporting_pos)
            .then(|| self.debug_supporting_block_highlight(legacy_pos));
        Some(DebugLocalSupportingBlocks {
            supporting,
            legacy_effect,
        })
    }

    /// Mirrors `Entity.getOnPos(0.2F)`, including the main-supporting-block
    /// override and the fence/wall/gate Y exception used by entity effects.
    pub(crate) fn legacy_on_pos(
        &self,
        supporting_pos: Option<BlockPos>,
        position: crate::EntityVec3,
    ) -> BlockPos {
        let Some(supporting_pos) = supporting_pos else {
            return BlockPos {
                x: block_floor(position.x),
                y: block_floor(position.y - 0.2),
                z: block_floor(position.z),
            };
        };
        let keeps_support_y = self
            .block_view(supporting_pos)
            .and_then(|block| block.block_name)
            .is_some_and(|name| {
                is_fence_block_name(name)
                    || is_wall_block_name(name)
                    || is_fence_gate_block_name(name)
            });
        if keeps_support_y {
            supporting_pos
        } else {
            BlockPos {
                y: block_floor(position.y - 0.2),
                ..supporting_pos
            }
        }
    }

    pub(super) fn debug_supporting_block_highlight(
        &self,
        pos: BlockPos,
    ) -> DebugSupportingBlockHighlight {
        let collision_boxes = self
            .block_view(pos)
            .and_then(|block| block_collision_shape_empty_context(&block, pos))
            .map(BlockCollisionShape::to_aabbs)
            .unwrap_or_default()
            .into_iter()
            .map(|shape_box| world_collision_box(pos, shape_box))
            .collect();
        DebugSupportingBlockHighlight {
            pos,
            collision_boxes,
        }
    }
}
