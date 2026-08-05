//! `BlockBehaviour.getMenuProvider` and the entity-side `instanceof
//! MenuProvider` test, projected onto the client's world state.
//!
//! `Gui.canRenderCrosshairForSpectator` and
//! `GameRenderer.shouldRenderBlockOutline` are the two consumers: both ask
//! whether the current hit result names something a spectator can still open,
//! which is `entity instanceof MenuProvider` for entity hits and
//! `state.getMenuProvider(level, pos) != null` for block hits. The override
//! owner per block and the `MenuProvider` membership per block-entity/entity
//! type are generated vanilla facts, so a new override fails the match arm
//! rather than silently answering `false`.

use bbb_protocol::{
    block_entity_types::vanilla_block_entity_type_provides_menu,
    block_types::{vanilla_block_type_for_resource_id, VanillaBlockMenuProviderOverride},
};

use crate::{
    chest_blocks::ChestCombineResult, entities::vanilla_entity_metadata_type_is_menu_provider,
    BlockPos, WorldStore,
};

impl WorldStore {
    pub fn block_provides_menu(&self, pos: BlockPos) -> bool {
        self.resolve_block_menu_provider(pos).unwrap_or(false)
    }

    pub fn entity_provides_menu(&self, entity_id: i32) -> bool {
        self.entity_type_id(entity_id)
            .is_some_and(vanilla_entity_metadata_type_is_menu_provider)
    }

    fn resolve_block_menu_provider(&self, pos: BlockPos) -> Option<bool> {
        let block = self.probe_block(pos)?;
        let entry = vanilla_block_type_for_resource_id(block.block_name.as_deref()?)?;
        Some(match entry.menu_provider_override? {
            VanillaBlockMenuProviderOverride::BaseEntityBlock => {
                self.block_entity_provides_menu(pos)
            }
            VanillaBlockMenuProviderOverride::ChestBlock => {
                self.chest_combine(pos, &block, false)? != ChestCombineResult::None
            }
            VanillaBlockMenuProviderOverride::LecternBlock => {
                block.block_properties.get("has_book").map(String::as_str) == Some("true")
                    && self.block_entity_provides_menu(pos)
            }
            VanillaBlockMenuProviderOverride::AnvilBlock
            | VanillaBlockMenuProviderOverride::CartographyTableBlock
            | VanillaBlockMenuProviderOverride::CraftingTableBlock
            | VanillaBlockMenuProviderOverride::EnchantingTableBlock
            | VanillaBlockMenuProviderOverride::GrindstoneBlock
            | VanillaBlockMenuProviderOverride::LoomBlock
            | VanillaBlockMenuProviderOverride::SmithingTableBlock
            | VanillaBlockMenuProviderOverride::StonecutterBlock => true,
        })
    }

    fn block_entity_provides_menu(&self, pos: BlockPos) -> bool {
        self.block_entity_type_id_at(pos)
            .is_some_and(vanilla_block_entity_type_provides_menu)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::{
        block_entity_types::{
            VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID, VANILLA_BLOCK_ENTITY_TYPE_FURNACE_ID,
            VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID, VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
        },
        entity_types::{VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_PIG_ID},
        packets::{AddEntity, BlockPos as ProtocolBlockPos, BlockUpdate, Vec3d},
    };
    use uuid::Uuid;

    use crate::{
        BlockEntityRecord, BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension, WorldStore,
    };

    #[test]
    fn blocks_without_a_menu_provider_override_never_open_a_menu() {
        let mut world = world_with_air_chunk(Vec::new());
        set_block(
            &mut world,
            pos(1, 1, 1),
            "minecraft:grass_block",
            &[("snowy", "false")],
        );
        set_block(&mut world, pos(2, 1, 1), "minecraft:stone", &[]);

        assert!(!world.block_provides_menu(pos(1, 1, 1)));
        assert!(!world.block_provides_menu(pos(2, 1, 1)));
    }

    #[test]
    fn unconditional_menu_provider_blocks_open_without_a_block_entity() {
        let mut world = world_with_air_chunk(Vec::new());
        set_block(&mut world, pos(1, 1, 1), "minecraft:crafting_table", &[]);
        set_block(&mut world, pos(2, 1, 1), "minecraft:enchanting_table", &[]);

        assert!(world.block_provides_menu(pos(1, 1, 1)));
        assert!(world.block_provides_menu(pos(2, 1, 1)));
    }

    #[test]
    fn base_entity_blocks_defer_to_the_block_entity_at_the_position() {
        let mut world = world_with_air_chunk(Vec::new());
        set_block(
            &mut world,
            pos(1, 1, 1),
            "minecraft:furnace",
            &[("facing", "north"), ("lit", "false")],
        );

        assert_eq!(
            world.block_entity_type_id_at(pos(1, 1, 1)),
            Some(VANILLA_BLOCK_ENTITY_TYPE_FURNACE_ID)
        );
        assert!(world.block_provides_menu(pos(1, 1, 1)));
    }

    #[test]
    fn non_menu_block_entities_do_not_open_a_menu() {
        let mut world = world_with_air_chunk(vec![BlockEntityRecord::empty(
            1,
            1,
            1,
            VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
        )]);
        set_block(
            &mut world,
            pos(1, 1, 1),
            "minecraft:oak_sign",
            &[("rotation", "0"), ("waterlogged", "false")],
        );

        assert!(!world.block_provides_menu(pos(1, 1, 1)));
    }

    #[test]
    fn lecterns_only_open_a_menu_while_they_hold_a_book() {
        let mut world = world_with_air_chunk(vec![BlockEntityRecord::empty(
            1,
            1,
            1,
            VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID,
        )]);
        let properties = |has_book| {
            [
                ("facing", "north"),
                ("has_book", has_book),
                ("powered", "false"),
            ]
        };
        set_block(
            &mut world,
            pos(1, 1, 1),
            "minecraft:lectern",
            &properties("false"),
        );
        assert!(!world.block_provides_menu(pos(1, 1, 1)));

        set_block(
            &mut world,
            pos(1, 1, 1),
            "minecraft:lectern",
            &properties("true"),
        );
        assert!(world.block_provides_menu(pos(1, 1, 1)));
    }

    #[test]
    fn a_blocked_chest_reports_no_menu_provider() {
        let mut world = world_with_air_chunk(Vec::new());
        let chest = [
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ];
        set_block(&mut world, pos(1, 1, 1), "minecraft:chest", &chest);
        assert_eq!(
            world.block_entity_type_id_at(pos(1, 1, 1)),
            Some(VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID)
        );
        assert!(world.block_provides_menu(pos(1, 1, 1)));

        set_block(&mut world, pos(1, 2, 1), "minecraft:stone", &[]);
        assert!(!world.block_provides_menu(pos(1, 1, 1)));
    }

    #[test]
    fn a_double_chest_loses_its_menu_when_the_other_half_is_blocked() {
        let mut world = world_with_air_chunk(Vec::new());
        set_block(
            &mut world,
            pos(1, 1, 1),
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "left"),
                ("waterlogged", "false"),
            ],
        );
        set_block(
            &mut world,
            pos(2, 1, 1),
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "right"),
                ("waterlogged", "false"),
            ],
        );
        assert!(world.block_provides_menu(pos(1, 1, 1)));

        set_block(&mut world, pos(2, 2, 1), "minecraft:stone", &[]);
        assert!(!world.block_provides_menu(pos(1, 1, 1)));
        assert!(!world.block_provides_menu(pos(2, 1, 1)));
    }

    #[test]
    fn only_container_entities_report_an_entity_menu_provider() {
        let mut world = world_with_air_chunk(Vec::new());
        add_entity(&mut world, 41, VANILLA_ENTITY_TYPE_CHEST_MINECART_ID);
        add_entity(&mut world, 42, VANILLA_ENTITY_TYPE_PIG_ID);

        assert!(world.entity_provides_menu(41));
        assert!(!world.entity_provides_menu(42));
        assert!(!world.entity_provides_menu(43));
    }

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    fn world_with_air_chunk(block_entities: Vec<BlockEntityRecord>) -> WorldStore {
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
                block_states: PalettedContainerData {
                    domain: PaletteDomain::BlockStates,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![crate::vanilla_air_block_state_id()],
                    packed_data: Vec::new(),
                    entry_count: 4096,
                },
                biomes: PalettedContainerData {
                    domain: PaletteDomain::Biomes,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 64,
                },
            }],
            block_entities,
            light: LightData::default(),
        });
        world
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str, properties: &[(&str, &str)]) {
        let properties: BTreeMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        let block_state_id = crate::vanilla_block_state_id(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name} {properties:?}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        }));
    }

    fn add_entity(world: &mut WorldStore, entity_id: i32, entity_type_id: i32) {
        world.apply_add_entity(AddEntity {
            id: entity_id,
            uuid: Uuid::from_u128(u128::try_from(entity_id).unwrap()),
            entity_type_id,
            position: Vec3d {
                x: 0.5,
                y: 1.0,
                z: 0.5,
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
}
