use std::cmp::Ordering;

use bbb_protocol::{
    block_entity_types::vanilla_block_entity_type,
    data_component_types::{
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
    },
    decode_nbt_summary,
    packets::{
        AdventureModePredicateDataSummary, BlockPredicateSummary, DataComponentPatchSummary,
        HolderSetSummary, InteractionHand, NbtSummaryEntry, NbtSummaryValue,
        StatePropertyMatcherSummary, StatePropertyValueMatcherSummary,
    },
};

use crate::{BlockPos, BlockView, WorldStore};

const BLOCK_REGISTRY: &str = "minecraft:block";

#[derive(Debug, Clone, Copy)]
enum AdventureModeComponent {
    CanPlaceOn,
    CanBreak,
}

impl AdventureModeComponent {
    fn type_id(self) -> i32 {
        match self {
            Self::CanPlaceOn => VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
            Self::CanBreak => VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
        }
    }

    fn value(
        self,
        patch: &DataComponentPatchSummary,
    ) -> Option<&AdventureModePredicateDataSummary> {
        match self {
            Self::CanPlaceOn => patch.can_place_on_persistent.as_ref(),
            Self::CanBreak => patch.can_break_persistent.as_ref(),
        }
    }
}

impl WorldStore {
    pub fn local_selected_main_hand_can_place_on_block_in_adventure_mode(
        &self,
        pos: BlockPos,
    ) -> bool {
        self.local_item_in_hand_adventure_mode_predicate_matches(
            AdventureModeComponent::CanPlaceOn,
            InteractionHand::MainHand,
            pos,
        )
    }

    pub(crate) fn local_item_in_hand_can_place_on_block_in_adventure_mode(
        &self,
        hand: InteractionHand,
        pos: BlockPos,
    ) -> bool {
        self.local_item_in_hand_adventure_mode_predicate_matches(
            AdventureModeComponent::CanPlaceOn,
            hand,
            pos,
        )
    }

    pub fn local_selected_main_hand_can_break_block_in_adventure_mode(
        &self,
        pos: BlockPos,
    ) -> bool {
        self.local_item_in_hand_adventure_mode_predicate_matches(
            AdventureModeComponent::CanBreak,
            InteractionHand::MainHand,
            pos,
        )
    }

    fn local_item_in_hand_adventure_mode_predicate_matches(
        &self,
        component: AdventureModeComponent,
        hand: InteractionHand,
        pos: BlockPos,
    ) -> bool {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return false;
        };
        let patch = &stack.component_patch;
        let type_id = component.type_id();
        if patch.removed_type_ids.contains(&type_id) || !patch.added_type_ids.contains(&type_id) {
            return false;
        }
        let Some(predicate) = component.value(patch) else {
            return false;
        };
        let Some(block) = self.block_view(pos) else {
            return false;
        };
        predicate
            .predicates
            .iter()
            .any(|predicate| self.adventure_mode_block_predicate_matches(predicate, &block))
    }

    fn adventure_mode_block_predicate_matches(
        &self,
        predicate: &BlockPredicateSummary,
        block: &BlockView,
    ) -> bool {
        if predicate
            .blocks
            .as_ref()
            .is_some_and(|blocks| !self.adventure_mode_holder_set_matches(blocks, block))
        {
            return false;
        }
        if predicate.state.as_ref().is_some_and(|state| {
            !state
                .iter()
                .all(|matcher| self.adventure_mode_state_matcher_matches(matcher, block))
        }) {
            return false;
        }
        if predicate.nbt.as_ref().is_some_and(|expected| {
            !self
                .block_entity_nbt_for_adventure_mode(block.pos)
                .is_some_and(|actual| nbt_matches(expected, &actual))
        }) {
            return false;
        }

        // Vanilla's BlockPredicate.matches(BlockInWorld) intentionally does
        // not evaluate the exact/partial DataComponentMatchers branch.
        true
    }

    fn adventure_mode_holder_set_matches(
        &self,
        holders: &HolderSetSummary,
        block: &BlockView,
    ) -> bool {
        let Some(block_name) = block.block_name else {
            return false;
        };
        let Some(holder_id) = self.registry_holder_id_by_resource_id(BLOCK_REGISTRY, block_name)
        else {
            return false;
        };
        match (holders.registry_ids.as_slice(), holders.tag.as_deref()) {
            (ids, None) => ids.contains(&holder_id),
            ([], Some(tag)) => self
                .registry_holder_ids_for_tag(BLOCK_REGISTRY, tag)
                .is_some_and(|ids| ids.contains(&holder_id)),
            (_, Some(_)) => false,
        }
    }

    fn adventure_mode_state_matcher_matches(
        &self,
        matcher: &StatePropertyMatcherSummary,
        block: &BlockView,
    ) -> bool {
        let Some(block_name) = block.block_name else {
            return false;
        };
        let Ok(name) = String::from_utf16(matcher.name.units()) else {
            return false;
        };
        let Some(actual) = block.block_properties.get(&name) else {
            return false;
        };
        let compare = |expected: &bbb_protocol::JavaString| {
            let expected = String::from_utf16(expected.units()).ok()?;
            self.registries
                .block_states
                .compare_block_property_values(block_name, &name, actual, &expected)
        };
        match &matcher.value {
            StatePropertyValueMatcherSummary::Exact(expected) => {
                compare(expected) == Some(Ordering::Equal)
            }
            StatePropertyValueMatcherSummary::Range { min, max } => {
                min.as_ref()
                    .is_none_or(|min| compare(min).is_some_and(|order| order != Ordering::Less))
                    && max.as_ref().is_none_or(|max| {
                        compare(max).is_some_and(|order| order != Ordering::Greater)
                    })
            }
        }
    }

    fn block_entity_nbt_for_adventure_mode(&self, pos: BlockPos) -> Option<NbtSummaryValue> {
        let type_id = self.block_entity_type_id_at(pos)?;
        let block_entity_type = vanilla_block_entity_type(type_id)?;
        let raw = self.block_entity_raw_nbt_at(pos)?;
        let mut entries = match (!raw.is_empty())
            .then(|| decode_nbt_summary(raw))
            .transpose()
        {
            Ok(Some(Some(NbtSummaryValue::Compound(entries)))) => entries,
            Ok(Some(None)) | Ok(None) => Vec::new(),
            Ok(Some(Some(_))) | Err(_) => return None,
        };

        if !entries.iter().any(|entry| entry.name == "components") {
            entries.push(NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(Vec::new()),
            });
        }
        replace_compound_entry(
            &mut entries,
            "id",
            NbtSummaryValue::string(block_entity_type.resource_id),
        );
        replace_compound_entry(&mut entries, "x", NbtSummaryValue::Int(pos.x));
        replace_compound_entry(&mut entries, "y", NbtSummaryValue::Int(pos.y));
        replace_compound_entry(&mut entries, "z", NbtSummaryValue::Int(pos.z));
        Some(NbtSummaryValue::Compound(entries))
    }
}

fn replace_compound_entry(
    entries: &mut Vec<NbtSummaryEntry>,
    name: &'static str,
    value: NbtSummaryValue,
) {
    if let Some(entry) = entries.iter_mut().find(|entry| entry.name == name) {
        entry.value = value;
    } else {
        entries.push(NbtSummaryEntry {
            name: name.into(),
            value,
        });
    }
}

fn nbt_matches(expected: &NbtSummaryValue, actual: &NbtSummaryValue) -> bool {
    match (expected, actual) {
        (NbtSummaryValue::Compound(expected), NbtSummaryValue::Compound(actual)) => {
            if actual.len() < expected.len() {
                return false;
            }
            expected.iter().all(|expected| {
                actual
                    .iter()
                    .find(|actual| actual.name == expected.name)
                    .is_some_and(|actual| nbt_matches(&expected.value, &actual.value))
            })
        }
        (NbtSummaryValue::List(expected), NbtSummaryValue::List(actual)) => {
            if expected.is_empty() {
                return actual.is_empty();
            }
            actual.len() >= expected.len()
                && expected
                    .iter()
                    .all(|expected| actual.iter().any(|actual| nbt_matches(expected, actual)))
        }
        _ => expected == actual,
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
        block_types::vanilla_block_type_for_resource_id,
        codec::Encoder,
        data_component_types::{
            VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        },
        packets::{
            BlockEntityData, BlockPos as ProtocolBlockPos, DataComponentPredicateTypeSummary,
            ItemStackSummary, PartialDataComponentPredicateSummary, RegistryTags,
            SetPlayerInventory, TagNetworkPayload, UpdateTags,
        },
    };

    use crate::{
        vanilla_block_state_id, BlockEntityRecord, ChunkColumn, ChunkPos, ChunkSection, ChunkState,
        LightData, PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };

    use super::*;

    #[test]
    fn adventure_mode_predicates_match_direct_tags_state_ranges_and_ignore_component_matchers() {
        let pos = BlockPos { x: 1, y: 0, z: 2 };
        let state_id = vanilla_block_state_id(
            "minecraft:iron_chain",
            &[
                ("axis".to_string(), "y".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();
        let mut world = world_with_single_state(state_id);
        let iron_chain = vanilla_block_type_for_resource_id("minecraft:iron_chain")
            .unwrap()
            .id;
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: BLOCK_REGISTRY.to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:test/adventure_chains".to_string(),
                    entries: vec![iron_chain],
                }],
            }],
        });

        set_main_hand_predicates(
            &mut world,
            Some(AdventureModePredicateDataSummary {
                predicates: vec![
                    BlockPredicateSummary {
                        blocks: Some(HolderSetSummary {
                            registry_ids: vec![iron_chain],
                            tag: None,
                        }),
                        state: Some(vec![StatePropertyMatcherSummary {
                            name: "axis".into(),
                            value: StatePropertyValueMatcherSummary::Range {
                                min: Some("x".into()),
                                max: Some("z".into()),
                            },
                        }]),
                        exact_components: Some(Box::new(DataComponentPatchSummary {
                            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                            max_stack_size: Some(64),
                            ..DataComponentPatchSummary::default()
                        })),
                        partial_components: vec![PartialDataComponentPredicateSummary {
                            predicate_type: DataComponentPredicateTypeSummary::AnyComponent(
                                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                            ),
                            wire_value: Some(NbtSummaryValue::Compound(Vec::new())),
                        }],
                        ..BlockPredicateSummary::default()
                    },
                    BlockPredicateSummary {
                        blocks: Some(HolderSetSummary {
                            registry_ids: Vec::new(),
                            tag: Some("minecraft:missing".to_string()),
                        }),
                        ..BlockPredicateSummary::default()
                    },
                ],
            }),
            Some(AdventureModePredicateDataSummary {
                predicates: vec![BlockPredicateSummary {
                    blocks: Some(HolderSetSummary {
                        registry_ids: Vec::new(),
                        tag: Some("minecraft:test/adventure_chains".to_string()),
                    }),
                    state: Some(vec![StatePropertyMatcherSummary {
                        name: "waterlogged".into(),
                        value: StatePropertyValueMatcherSummary::Exact("false".into()),
                    }]),
                    ..BlockPredicateSummary::default()
                }],
            }),
        );

        assert!(world.local_selected_main_hand_can_place_on_block_in_adventure_mode(pos));
        assert!(world.local_selected_main_hand_can_break_block_in_adventure_mode(pos));
    }

    #[test]
    fn adventure_mode_predicate_rejects_invalid_state_values_and_explicit_removal() {
        let pos = BlockPos { x: 1, y: 0, z: 2 };
        let state_id = vanilla_block_state_id(
            "minecraft:iron_chain",
            &[
                ("axis".to_string(), "y".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();
        let mut world = world_with_single_state(state_id);
        set_main_hand_predicates(
            &mut world,
            None,
            Some(AdventureModePredicateDataSummary {
                predicates: vec![BlockPredicateSummary {
                    state: Some(vec![StatePropertyMatcherSummary {
                        name: "axis".into(),
                        value: StatePropertyValueMatcherSummary::Range {
                            min: Some("invalid".into()),
                            max: None,
                        },
                    }]),
                    ..BlockPredicateSummary::default()
                }],
            }),
        );
        assert!(!world.local_selected_main_hand_can_break_block_in_adventure_mode(pos));

        let held = world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .clone();
        let mut removed = held;
        removed
            .component_patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID);
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: removed,
        }));
        assert!(!world.local_selected_main_hand_can_break_block_in_adventure_mode(pos));
    }

    #[test]
    fn adventure_mode_raw_property_name_and_value_do_not_match_replacement_text() {
        let pos = BlockPos { x: 1, y: 0, z: 2 };
        let state_id = vanilla_block_state_id(
            "minecraft:iron_chain",
            &[
                ("axis".to_string(), "y".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();
        let mut world = world_with_single_state(state_id);
        for matcher in [
            StatePropertyMatcherSummary {
                name: vec![0xd800].into(),
                value: StatePropertyValueMatcherSummary::Exact("y".into()),
            },
            StatePropertyMatcherSummary {
                name: "axis".into(),
                value: StatePropertyValueMatcherSummary::Exact(vec![0xd800].into()),
            },
            StatePropertyMatcherSummary {
                name: "axis".into(),
                value: StatePropertyValueMatcherSummary::Range {
                    min: Some(vec![0xd801].into()),
                    max: None,
                },
            },
        ] {
            set_main_hand_predicates(
                &mut world,
                None,
                Some(AdventureModePredicateDataSummary {
                    predicates: vec![BlockPredicateSummary {
                        state: Some(vec![matcher]),
                        ..BlockPredicateSummary::default()
                    }],
                }),
            );
            assert!(!world.local_selected_main_hand_can_break_block_in_adventure_mode(pos));
        }
    }

    #[test]
    fn adventure_mode_nbt_uses_partial_compounds_lists_and_full_metadata() {
        let pos = BlockPos { x: 1, y: 0, z: 2 };
        let chest_state = vanilla_block_state_id(
            "minecraft:chest",
            &[
                ("facing".to_string(), "north".to_string()),
                ("type".to_string(), "single".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();
        let mut world = world_with_single_state(chest_state);
        let raw_nbt = compound_with_items();
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
                raw_nbt,
            })
            .unwrap());
        set_main_hand_predicates(
            &mut world,
            None,
            Some(AdventureModePredicateDataSummary {
                predicates: vec![BlockPredicateSummary {
                    nbt: Some(NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "id".into(),
                            value: NbtSummaryValue::string("minecraft:chest"),
                        },
                        NbtSummaryEntry {
                            name: "x".into(),
                            value: NbtSummaryValue::Int(1),
                        },
                        NbtSummaryEntry {
                            name: "components".into(),
                            value: NbtSummaryValue::Compound(Vec::new()),
                        },
                        NbtSummaryEntry {
                            name: "Items".into(),
                            value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                                NbtSummaryEntry {
                                    name: "slot".into(),
                                    value: NbtSummaryValue::Byte(3),
                                },
                            ])]),
                        },
                    ])),
                    ..BlockPredicateSummary::default()
                }],
            }),
        );

        assert!(world.local_selected_main_hand_can_break_block_in_adventure_mode(pos));
    }

    #[test]
    fn vanilla_property_comparison_uses_typed_comparable_order() {
        let registry = &WorldStore::new().registries.block_states;
        assert_eq!(
            registry.compare_block_property_values("minecraft:repeater", "delay", "2", "10"),
            None
        );
        assert_eq!(
            registry.compare_block_property_values(
                "minecraft:iron_chain",
                "waterlogged",
                "false",
                "true"
            ),
            Some(Ordering::Less)
        );
        assert_eq!(
            registry.compare_block_property_values("minecraft:observer", "facing", "down", "north"),
            Some(Ordering::Less)
        );
        assert_eq!(
            registry.compare_block_property_values("minecraft:oak_log", "axis", "y", "z"),
            Some(Ordering::Less)
        );
    }

    fn world_with_single_state(state_id: i32) -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.chunks.push(
            ChunkColumn {
                pos: ChunkPos { x: 0, z: 0 },
                state: ChunkState::Decoded,
                heightmaps: Vec::new(),
                sections: vec![std::sync::Arc::new(ChunkSection {
                    non_empty_block_count: 1,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        state_id,
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })],
                block_entities: std::sync::Arc::new(vec![BlockEntityRecord::empty(
                    1,
                    0,
                    2,
                    VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
                )]),
                light: LightData::default(),
            }
            .into(),
        );
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

    fn set_main_hand_predicates(
        world: &mut WorldStore,
        can_place_on: Option<AdventureModePredicateDataSummary>,
        can_break: Option<AdventureModePredicateDataSummary>,
    ) {
        let mut patch = DataComponentPatchSummary::default();
        if let Some(value) = can_place_on {
            patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID);
            patch.can_place_on_persistent = Some(value);
        }
        if let Some(value) = can_break {
            patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID);
            patch.can_break_persistent = Some(value);
        }
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(1),
                count: 1,
                component_patch: patch.into(),
            },
        }));
    }

    fn compound_with_items() -> Vec<u8> {
        let mut out = Encoder::new();
        out.write_u8(10);
        out.write_u8(9);
        write_nbt_string(&mut out, "Items");
        out.write_u8(10);
        out.write_i32(2);
        for slot in [3, 5] {
            out.write_u8(1);
            write_nbt_string(&mut out, "slot");
            out.write_i8(slot);
            out.write_u8(8);
            write_nbt_string(&mut out, "name");
            write_nbt_string(&mut out, "extra");
            out.write_u8(0);
        }
        out.write_u8(0);
        out.into_inner()
    }

    fn write_nbt_string(out: &mut Encoder, value: &str) {
        out.write_u16(value.len() as u16);
        out.write_bytes(value.as_bytes());
    }
}
