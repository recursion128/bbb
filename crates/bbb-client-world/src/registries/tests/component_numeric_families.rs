use super::*;

#[derive(Clone, Copy)]
enum NumericField {
    CustomModelDataFloat,
    FoodSaturation,
    MapX,
    MapZ,
    MapRotation,
    MapCatalog(&'static str),
    ToolMiningSpeed,
    DamageReduction(&'static str),
    ItemDamage(&'static str),
    Kinetic(&'static str),
    KineticCondition(&'static str, &'static str),
    SoundRange,
    PredicateAmount(Option<&'static str>),
}

impl NumericField {
    fn component(self) -> &'static str {
        match self {
            Self::CustomModelDataFloat => "minecraft:custom_model_data",
            Self::FoodSaturation => "minecraft:food",
            Self::MapX | Self::MapZ | Self::MapRotation | Self::MapCatalog(_) => {
                "minecraft:map_decorations"
            }
            Self::ToolMiningSpeed => "minecraft:tool",
            Self::DamageReduction(_) | Self::ItemDamage(_) => "minecraft:blocks_attacks",
            Self::Kinetic(_) | Self::KineticCondition(_, _) => "minecraft:kinetic_weapon",
            Self::SoundRange => "minecraft:break_sound",
            Self::PredicateAmount(_) => "minecraft:lock",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::CustomModelDataFloat => "floats",
            Self::FoodSaturation => "saturation",
            Self::MapX | Self::MapCatalog(_) => "x",
            Self::MapZ => "z",
            Self::MapRotation => "rotation",
            Self::ToolMiningSpeed => "default_mining_speed",
            Self::DamageReduction(name) | Self::ItemDamage(name) | Self::Kinetic(name) => name,
            Self::KineticCondition(_, name) => name,
            Self::SoundRange => "range",
            Self::PredicateAmount(_) => "amount",
        }
    }

    fn is_double(self) -> bool {
        matches!(
            self,
            Self::MapX | Self::MapZ | Self::MapCatalog(_) | Self::PredicateAmount(_)
        )
    }

    fn numeric(self, value: f64) -> NbtSummaryValue {
        if self.is_double() {
            NbtSummaryValue::Double(value.to_bits())
        } else {
            NbtSummaryValue::Float((value as f32).to_bits())
        }
    }

    fn value(self, numeric: Option<NbtSummaryValue>) -> NbtSummaryValue {
        match self {
            Self::CustomModelDataFloat => compound(vec![(
                "floats",
                NbtSummaryValue::List(vec![numeric.unwrap()]),
            )]),
            Self::FoodSaturation => {
                let mut fields = vec![("nutrition", NbtSummaryValue::Int(4))];
                if let Some(numeric) = numeric {
                    fields.push(("saturation", numeric));
                }
                compound(fields)
            }
            Self::MapX | Self::MapZ | Self::MapRotation | Self::MapCatalog(_) => {
                let decoration = match self {
                    Self::MapCatalog(resource_id) => resource_id,
                    _ => "player",
                };
                let mut fields = vec![("type", string(decoration))];
                for (name, value) in [
                    ("x", NbtSummaryValue::Double(1.25_f64.to_bits())),
                    ("z", NbtSummaryValue::Double((-2.5_f64).to_bits())),
                    ("rotation", NbtSummaryValue::Float(0.5_f32.to_bits())),
                ] {
                    if name == self.name() {
                        if let Some(numeric) = numeric.clone() {
                            fields.push((name, numeric));
                        }
                    } else {
                        fields.push((name, value));
                    }
                }
                compound(vec![("marker", compound(fields))])
            }
            Self::ToolMiningSpeed => compound(vec![
                ("rules", NbtSummaryValue::List(vec![])),
                (self.name(), numeric.unwrap()),
            ]),
            Self::DamageReduction(_) | Self::ItemDamage(_) => {
                let mut fields = vec![
                    ("base", NbtSummaryValue::Float(1.25_f32.to_bits())),
                    ("factor", NbtSummaryValue::Float(2.5_f32.to_bits())),
                ];
                fields
                    .iter_mut()
                    .find(|(name, _)| *name == self.name())
                    .unwrap()
                    .1 = numeric.unwrap();
                if matches!(self, Self::ItemDamage(_)) {
                    fields.push(("threshold", NbtSummaryValue::Float(1.0_f32.to_bits())));
                    compound(vec![("item_damage", compound(fields))])
                } else {
                    compound(vec![(
                        "damage_reductions",
                        NbtSummaryValue::List(vec![compound(fields)]),
                    )])
                }
            }
            Self::Kinetic(_) => compound(vec![(self.name(), numeric.unwrap())]),
            Self::KineticCondition(condition, name) => compound(vec![(
                condition,
                compound(vec![
                    ("max_duration_ticks", NbtSummaryValue::Int(1)),
                    (name, numeric.unwrap()),
                ]),
            )]),
            Self::SoundRange => compound(vec![
                ("sound_id", string("test:custom")),
                ("range", numeric.unwrap()),
            ]),
            Self::PredicateAmount(bound) => {
                let amount = match bound {
                    Some(bound) => compound(vec![(bound, numeric.unwrap())]),
                    None => numeric.unwrap(),
                };
                compound(vec![(
                    "predicates",
                    compound(vec![(
                        "minecraft:attribute_modifiers",
                        compound(vec![(
                            "modifiers",
                            compound(vec![(
                                "contains",
                                NbtSummaryValue::List(vec![compound(vec![("amount", amount)])]),
                            )]),
                        )]),
                    )]),
                )])
            }
        }
    }
}

fn compound(fields: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        fields
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.into())
}

fn begin_candidate(
    store: &mut WorldStore,
    resource_id: &str,
    target: NumericField,
    numeric: Option<NbtSummaryValue>,
) {
    let trim = compound(vec![
        ("asset_name", string("quartz")),
        (
            "description",
            compound(vec![
                ("text", string("x")),
                (
                    "hover_event",
                    compound(vec![
                        ("action", string("show_item")),
                        ("id", string("stone")),
                        (
                            "components",
                            compound(vec![(target.component(), target.value(numeric))]),
                        ),
                    ]),
                ),
            ]),
        ),
    ]);
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(store, None);
    store.record_registry_entries(
        "minecraft:trim_material",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            resource_id,
            bbb_protocol::encode_nbt_summary(&trim, 1 << 20).unwrap(),
        )],
    );
}

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
    optional_field(value, name).unwrap()
}

fn optional_field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(fields) = value else {
        panic!("expected compound containing {name}");
    };
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn only_element(value: &NbtSummaryValue) -> &NbtSummaryValue {
    let NbtSummaryValue::List(values) = value else {
        panic!("expected a list");
    };
    assert_eq!(values.len(), 1);
    &values[0]
}

fn published_component(store: &WorldStore, target: NumericField) -> NbtSummaryValue {
    let material = store.synchronized_trim_material_presentation(0).unwrap();
    assert_eq!(material.asset_name, "quartz");
    assert_eq!(material.description, "x");
    let hover = field(material.description_nbt.as_ref().unwrap(), "hover_event");
    assert_eq!(field(hover, "action"), &string("show_item"));
    assert_eq!(field(hover, "id"), &string("minecraft:stone"));
    field(field(hover, "components"), target.component()).clone()
}

fn published_numeric(store: &WorldStore, target: NumericField) -> f64 {
    let component = published_component(store, target);
    let numeric = match target {
        NumericField::CustomModelDataFloat => Some(only_element(field(&component, "floats"))),
        NumericField::FoodSaturation => {
            assert_eq!(field(&component, "nutrition"), &NbtSummaryValue::Int(4));
            Some(field(&component, "saturation"))
        }
        NumericField::MapX
        | NumericField::MapZ
        | NumericField::MapRotation
        | NumericField::MapCatalog(_) => {
            let marker = field(&component, "marker");
            let expected = match target {
                NumericField::MapCatalog(resource_id) => resource_id,
                _ => "minecraft:player",
            };
            assert_eq!(field(marker, "type"), &string(expected));
            Some(field(marker, target.name()))
        }
        NumericField::ToolMiningSpeed | NumericField::Kinetic(_) => {
            optional_field(&component, target.name())
        }
        NumericField::DamageReduction(name) => Some(field(
            only_element(field(&component, "damage_reductions")),
            name,
        )),
        NumericField::ItemDamage(name) => Some(field(field(&component, "item_damage"), name)),
        NumericField::KineticCondition(condition, name) => {
            let condition = field(&component, condition);
            assert_eq!(
                field(condition, "max_duration_ticks"),
                &NbtSummaryValue::Int(1)
            );
            optional_field(condition, name)
        }
        NumericField::SoundRange => {
            assert_eq!(field(&component, "sound_id"), &string("test:custom"));
            Some(field(&component, "range"))
        }
        NumericField::PredicateAmount(bound) => {
            let predicate = field(
                field(&component, "predicates"),
                "minecraft:attribute_modifiers",
            );
            let modifier = only_element(field(field(predicate, "modifiers"), "contains"));
            let amount = field(modifier, "amount");
            Some(match bound {
                Some(bound) => field(amount, bound),
                None => amount,
            })
        }
    };
    let Some(numeric) = numeric else {
        return match target {
            NumericField::ToolMiningSpeed | NumericField::Kinetic("damage_multiplier") => 1.0,
            NumericField::Kinetic("forward_movement") | NumericField::KineticCondition(_, _) => 0.0,
            _ => panic!("unexpected omitted numeric field"),
        };
    };
    match (target.is_double(), numeric) {
        (true, NbtSummaryValue::Double(bits)) => f64::from_bits(*bits),
        (false, NbtSummaryValue::Float(bits)) => f64::from(f32::from_bits(*bits)),
        _ => panic!("numeric field must retain its Codec.FLOAT or Codec.DOUBLE writer tag"),
    }
}

fn assert_network_numeric_publishes(target: NumericField, value: f64) {
    let mut store = WorldStore::new();
    begin_candidate(
        &mut store,
        "example:active",
        target,
        Some(target.numeric(1.25)),
    );
    store.finish_registry_configuration().unwrap();
    assert_eq!(published_numeric(&store, target), 1.25);
    let active_revision = store.registry_access_revision();

    begin_candidate(
        &mut store,
        "example:replacement",
        target,
        Some(target.numeric(value)),
    );
    store.finish_registry_configuration().unwrap();
    assert_ne!(store.registry_access_revision(), active_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:trim_material", 0),
        Some("example:replacement")
    );
    let actual = published_numeric(&store, target);
    if value.is_nan() {
        assert!(actual.is_nan());
    } else {
        let expected = if value == 0.0 { 0.0 } else { value };
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

macro_rules! numeric_publication_tests {
    ($target:expr) => {
        #[test]
        fn network_nan_publishes() {
            assert_network_numeric_publishes($target, f64::NAN);
        }

        #[test]
        fn network_positive_infinity_publishes() {
            assert_network_numeric_publishes($target, f64::INFINITY);
        }

        #[test]
        fn network_negative_infinity_publishes() {
            assert_network_numeric_publishes($target, f64::NEG_INFINITY);
        }

        #[test]
        fn network_signed_zero_and_finite_publish_canonically() {
            for value in [-0.0, 0.0, 1.0, 1.25, -2.5] {
                assert_network_numeric_publishes($target, value);
            }
        }
    };
}

macro_rules! numeric_family_tests {
    ($module:ident, $target:ident) => {
        mod $module {
            use super::*;

            numeric_publication_tests!(NumericField::$target);
        }
    };
}

numeric_family_tests!(custom_model_data, CustomModelDataFloat);
numeric_family_tests!(food, FoodSaturation);
numeric_family_tests!(map_x, MapX);
numeric_family_tests!(map_z, MapZ);
numeric_family_tests!(map_rotation, MapRotation);

macro_rules! additional_family_tests {
    ($module:ident, $target:expr) => {
        mod $module {
            use super::*;
            numeric_publication_tests!($target);
        }
    };
}

additional_family_tests!(tool_mining_speed, NumericField::ToolMiningSpeed);
additional_family_tests!(damage_reduction_base, NumericField::DamageReduction("base"));
additional_family_tests!(
    damage_reduction_factor,
    NumericField::DamageReduction("factor")
);
additional_family_tests!(item_damage_base, NumericField::ItemDamage("base"));
additional_family_tests!(item_damage_factor, NumericField::ItemDamage("factor"));
additional_family_tests!(
    kinetic_forward_movement,
    NumericField::Kinetic("forward_movement")
);
additional_family_tests!(
    kinetic_damage_multiplier,
    NumericField::Kinetic("damage_multiplier")
);
additional_family_tests!(
    dismount_min_speed,
    NumericField::KineticCondition("dismount_conditions", "min_speed")
);
additional_family_tests!(
    dismount_min_relative_speed,
    NumericField::KineticCondition("dismount_conditions", "min_relative_speed")
);
additional_family_tests!(
    knockback_min_speed,
    NumericField::KineticCondition("knockback_conditions", "min_speed")
);
additional_family_tests!(
    knockback_min_relative_speed,
    NumericField::KineticCondition("knockback_conditions", "min_relative_speed")
);
additional_family_tests!(
    damage_min_speed,
    NumericField::KineticCondition("damage_conditions", "min_speed")
);
additional_family_tests!(
    damage_min_relative_speed,
    NumericField::KineticCondition("damage_conditions", "min_relative_speed")
);
additional_family_tests!(sound_range, NumericField::SoundRange);
additional_family_tests!(predicate_amount_point, NumericField::PredicateAmount(None));
additional_family_tests!(
    predicate_amount_min,
    NumericField::PredicateAmount(Some("min"))
);
additional_family_tests!(
    predicate_amount_max,
    NumericField::PredicateAmount(Some("max"))
);

#[test]
fn inline_sound_malformed_range_publishes_without_range() {
    for numeric in [string("not a number"), compound(vec![])] {
        let mut store = WorldStore::new();
        begin_candidate(
            &mut store,
            "example:active",
            NumericField::SoundRange,
            Some(NumericField::SoundRange.numeric(2.0)),
        );
        store.finish_registry_configuration().unwrap();
        let active_revision = store.registry_access_revision();
        begin_candidate(
            &mut store,
            "example:lenient",
            NumericField::SoundRange,
            Some(numeric),
        );
        store.finish_registry_configuration().unwrap();
        assert_ne!(store.registry_access_revision(), active_revision);
        assert_eq!(
            store.registry_resource_id_by_holder_id("minecraft:trim_material", 0),
            Some("example:lenient")
        );
        assert_eq!(
            published_component(&store, NumericField::SoundRange),
            compound(vec![("sound_id", string("test:custom"))])
        );
    }
}

#[test]
fn fixed_map_decoration_catalog_publishes_and_roundtrips() {
    let mut store = WorldStore::new();
    for decoration in bbb_render_types::vanilla_map_decoration_types() {
        let target = NumericField::MapCatalog(decoration.resource_id);
        let previous_revision = store.registry_access_revision();
        begin_candidate(
            &mut store,
            "example:catalog",
            target,
            Some(target.numeric(1.25)),
        );
        store.finish_registry_configuration().unwrap();
        assert_ne!(store.registry_access_revision(), previous_revision);
        assert_eq!(published_numeric(&store, target), 1.25);
        assert_eq!(
            store.registry_resource_id_by_holder_id(
                "minecraft:map_decoration_type",
                decoration.type_id
            ),
            Some(decoration.resource_id)
        );
        assert_eq!(
            store.registry_holder_id_by_resource_id(
                "minecraft:map_decoration_type",
                decoration.resource_id
            ),
            Some(decoration.type_id)
        );
    }
}
