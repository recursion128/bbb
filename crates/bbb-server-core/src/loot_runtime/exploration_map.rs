use super::*;
use bbb_protocol::item_types::VANILLA_ITEM_MAP_ID;

impl Eval<'_> {
    pub(super) fn apply_exploration_map(
        &mut self,
        options: &LootExplorationMap,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        // Every miss returns the very same stack.
        let item_id = (stack_count(&stack) > 0).then_some(stack.item_id).flatten();
        if item_id != Some(VANILLA_ITEM_MAP_ID) {
            return Ok(stack);
        }
        let Some(origin) = self.context.origin() else {
            return Ok(stack);
        };
        let environment = self.environment(path)?;
        Ok(
            match self.at(path, environment.exploration_map(origin, options))? {
                Some(map) => map.into_summary(),
                None => stack,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_server_datapack::{
        frozen_lookup::FrozenDatapackRegistryLookup,
        loot::{parse_loot_function, parse_loot_table, LootElementValue},
        recipe::{parse_recipe_source_with_lookup, CookingKind},
        startup::{DatapackStartup, DatapackStartupOptions},
    };
    use serde_json::json;
    use std::cell::{Cell, RefCell};

    struct Environment {
        output: RuntimeItemStack,
        hit: Cell<bool>,
        calls: RefCell<Vec<(LootOrigin, LootExplorationMap)>>,
    }

    impl LootItemEnvironment for Environment {
        fn exploration_map(
            &self,
            origin: LootOrigin,
            options: &LootExplorationMap,
        ) -> std::result::Result<Option<RuntimeItemStack>, LootRuntimeReason> {
            self.calls.borrow_mut().push((origin, options.clone()));
            Ok(self.hit.get().then(|| self.output.clone()))
        }

        fn enchant_with_levels(
            &self,
            _: ItemStackSummary,
            _: i32,
            _: Option<&bbb_server_datapack::loot::HolderSet>,
            _: &GameRandom,
        ) -> std::result::Result<ItemStackSummary, LootRuntimeReason> {
            unreachable!()
        }

        fn matches_item(
            &self,
            _: &bbb_server_datapack::loot::ItemPredicate,
            _: &ItemStackSummary,
        ) -> std::result::Result<bool, LootRuntimeReason> {
            unreachable!()
        }

        fn smelt_result(
            &self,
            _: &ItemStackSummary,
        ) -> std::result::Result<Option<RuntimeItemStack>, LootRuntimeReason> {
            unreachable!()
        }
    }

    #[test]
    fn exploration_map_gates_preserve_original_and_hits_replace_the_stack() {
        let startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        })
        .unwrap();
        let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);
        let native = |result| {
            let bytes = serde_json::to_vec(&json!({
                "type": "smelting", "ingredient": "stone", "result": result,
            }))
            .unwrap();
            let decoded = bbb_registry_codec::decode_direct_registry_json(&bytes).unwrap();
            let held = parse_recipe_source_with_lookup(decoded.root(), &lookup)
                .unwrap()
                .cooking_of_kind(CookingKind::Smelting)
                .unwrap()
                .assemble();
            RuntimeItemStack::from_summary(held)
        };
        let input = native(json!({
            "id": "map", "count": 4,
            "components": {"custom_data": {"input_only": 1},
                "lock": {"predicates": {"custom_name": {}}}},
        }));
        let output = native(json!({"id": "filled_map", "components": {"map_id": 17}}));
        let environment = Environment {
            output: output.clone(),
            hit: Cell::new(false),
            calls: RefCell::new(Vec::new()),
        };
        let tables = LootRegistries {
            registries: Vec::new(),
            warnings: Vec::new(),
            tag_warnings: Vec::new(),
        };
        let origin = LootOrigin {
            x: -1.5,
            y: 64.0,
            z: 23.0,
        };
        let function = parse_loot_function(&json!({"function": "exploration_map"}), None).unwrap();
        let blocked = parse_loot_function(
            &json!({
                "function": "exploration_map",
                "conditions": [{"condition": "random_chance", "chance": 0}],
            }),
            None,
        )
        .unwrap();
        let random = crate::random::GameRandom::with_seed(0);
        let apply = |function: &LootFunction, stack: LootStack, origin: Option<LootOrigin>| {
            let params = LootParams {
                origin,
                item_environment: Some(&environment),
                ..LootParams::new(if origin.is_some() {
                    param_sets::COMMAND
                } else {
                    param_sets::EMPTY
                })
            };
            let mut eval = Eval::new("test:map", &tables, &params, &random).unwrap();
            eval.apply_loot_function(function, stack, "function")
                .unwrap()
        };
        let original = input.clone().into_summary();
        assert_eq!(apply(&blocked, original.clone(), Some(origin)), original);
        assert_eq!(apply(&function, original.clone(), None), original);
        let non_map = output.clone().into_summary();
        assert_eq!(apply(&function, non_map.clone(), Some(origin)), non_map);
        assert!(environment.calls.borrow().is_empty());
        let mut interact_tables = tables.clone();
        interact_tables.registries.push(bbb_server_datapack::loot::LoadedLootRegistry {
            registry_id: "minecraft:loot_table".into(),
            tags: None,
            elements: vec![bbb_server_datapack::loot::LootElement {
                id: "test:interact".into(), source_pack: "test".into(),
                value: LootElementValue::Table(parse_loot_table(&json!({
                    "pools": [{"rolls": 1, "entries": [{"type": "item", "name": "map", "functions": [{"function": "exploration_map"}]}]}]
                }), None).unwrap()),
            }],
        });
        let drops = named_table_drops(
            &interact_tables,
            "test:interact",
            &BlockDropParams {
                dimension: None,
                block_state_id: 0,
                pos: BlockPos { x: 0, y: 64, z: 0 },
                level: None,
                block_entity_data: None,
                tool: &LootTool::empty(),
                breaker: LootEntity::Absent,
                explosion_radius: None,
                block_entity: None,
                item_environment: Some(&environment),
            },
            &random,
        )
        .unwrap();
        assert_eq!(drops.len(), 1);
        assert!(drops[0].item_id == Some(VANILLA_ITEM_MAP_ID));
        assert!(environment.calls.borrow().is_empty());
        assert_eq!(apply(&function, original.clone(), Some(origin)), original);
        let fresh = ItemStackSummary {
            item_id: Some(VANILLA_ITEM_MAP_ID),
            count: 4,
            component_patch: Default::default(),
        };
        assert_eq!(apply(&function, fresh.clone(), Some(origin)), fresh);
        environment.hit.set(true);
        let ordered = parse_loot_function(
            &json!([
                {"function": "exploration_map",
                    "conditions": [{"condition": "random_chance", "chance": 1}]},
                {"function": "set_count", "count": 2},
            ]),
            None,
        )
        .unwrap();
        let replaced = RuntimeItemStack::from_summary(apply(&ordered, original, Some(origin)));
        assert_eq!(replaced, output.with_count(2));
        let configured = parse_loot_function(
            &json!({
                "function": "exploration_map", "destination": "test:maps",
                "decoration": "red_x", "zoom": 1, "search_radius": 32,
                "skip_existing_chunks": false,
            }),
            None,
        )
        .unwrap();
        assert_eq!(
            apply(&configured, fresh, Some(origin)),
            output.into_summary()
        );
        let defaults = LootExplorationMap {
            destination: "minecraft:on_treasure_maps".into(),
            decoration: "minecraft:mansion".into(),
            zoom: 2,
            search_radius: 50,
            skip_known_structures: true,
        };
        let mut expected_calls = vec![(origin, defaults); 3];
        expected_calls.push((
            origin,
            LootExplorationMap {
                destination: "test:maps".into(),
                decoration: "minecraft:red_x".into(),
                zoom: 1,
                search_radius: 32,
                skip_known_structures: false,
            },
        ));
        assert_eq!(*environment.calls.borrow(), expected_calls);
    }
}
