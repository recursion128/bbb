use super::*;

#[test]
fn name_component_descriptors_reject_noncanonical_projections_atomically() {
    let valid_custom = DataComponentPatchSummary {
        custom_name: Some("Custom".to_string()),
        custom_name_styled: Some(vec![StyledTextRun {
            text: "Custom".to_string(),
            style: crate::ComponentStyle::default(),
        }]),
        custom_name_nbt: Some(NbtSummaryValue::String("Custom".into())),
        ..DataComponentPatchSummary::default()
    };
    let valid_item = DataComponentPatchSummary {
        item_name: Some("Item".to_string()),
        item_name_styled: Some(vec![StyledTextRun {
            text: "Item".to_string(),
            style: crate::ComponentStyle::default(),
        }]),
        item_name_nbt: Some(NbtSummaryValue::String("Item".into())),
        ..DataComponentPatchSummary::default()
    };
    let cases = [
        (
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            DataComponentPatchSummary::default(),
            "text is missing",
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            DataComponentPatchSummary {
                custom_name: Some("Custom".to_string()),
                ..DataComponentPatchSummary::default()
            },
            "missing component nbt",
        ),
        (
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            DataComponentPatchSummary {
                custom_name: Some("stale".to_string()),
                ..valid_custom
            },
            "noncanonical component projections",
        ),
        (
            VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
            DataComponentPatchSummary::default(),
            "text is missing",
        ),
        (
            VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
            DataComponentPatchSummary {
                item_name_styled: Some(Vec::new()),
                ..valid_item
            },
            "noncanonical component projections",
        ),
    ];
    for (type_id, summary, expected_error) in cases {
        let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
        assert!(
            error.to_string().contains(expected_error),
            "{error} did not contain {expected_error:?}"
        );
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn lore_descriptor_round_trips_canonical_components_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LORE_TYPE_ID).unwrap();
    let styled_nbt = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String("Styled".into()),
        },
        NbtSummaryEntry {
            name: "italic".into(),
            value: NbtSummaryValue::Byte(0),
        },
        NbtSummaryEntry {
            name: "color".into(),
            value: NbtSummaryValue::String("gold".into()),
        },
    ]);
    let summary = DataComponentPatchSummary {
        lore: vec!["Plain".to_string(), "Styled".to_string()],
        lore_styled: vec![
            vec![StyledTextRun {
                text: "Plain".to_string(),
                style: crate::ComponentStyle::default(),
            }],
            vec![StyledTextRun {
                text: "Styled".to_string(),
                style: crate::ComponentStyle {
                    italic: Some(false),
                    color: Some(0xFF_AA_00),
                    ..Default::default()
                },
            }],
        ],
        lore_nbt: vec![NbtSummaryValue::String("Plain".into()), styled_nbt],
        ..DataComponentPatchSummary::default()
    };

    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let encoded = encoded.into_inner();
    assert_eq!(encoded[0], 2);
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.lore, summary.lore);
    assert_eq!(decoded.lore_styled, summary.lore_styled);
    assert_eq!(decoded.lore_nbt, summary.lore_nbt);

    let mut empty = Encoder::new();
    descriptor
        .encode(&DataComponentPatchSummary::default(), &mut empty)
        .unwrap();
    assert_eq!(empty.into_inner(), [0]);

    let maximal = DataComponentPatchSummary {
        lore: vec!["x".to_string(); MAX_LORE_LINES],
        lore_styled: vec![
            vec![StyledTextRun {
                text: "x".to_string(),
                style: crate::ComponentStyle::default(),
            }];
            MAX_LORE_LINES
        ],
        lore_nbt: vec![NbtSummaryValue::String("x".into()); MAX_LORE_LINES],
        ..DataComponentPatchSummary::default()
    };
    descriptor.encode(&maximal, &mut Encoder::new()).unwrap();
}

#[test]
fn lore_descriptor_rejects_limits_and_noncanonical_projections() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LORE_TYPE_ID).unwrap();
    let valid = DataComponentPatchSummary {
        lore: vec!["line".to_string()],
        lore_styled: vec![vec![StyledTextRun {
            text: "line".to_string(),
            style: crate::ComponentStyle::default(),
        }]],
        lore_nbt: vec![NbtSummaryValue::String("line".into())],
        ..DataComponentPatchSummary::default()
    };
    let cases = [
        (
            DataComponentPatchSummary {
                lore: valid.lore.clone(),
                ..DataComponentPatchSummary::default()
            },
            "projections are not aligned",
        ),
        (
            DataComponentPatchSummary {
                lore: vec!["stale".to_string()],
                ..valid.clone()
            },
            "noncanonical component projections",
        ),
        (
            DataComponentPatchSummary {
                lore_styled: vec![vec![StyledTextRun {
                    text: "stale".to_string(),
                    style: crate::ComponentStyle::default(),
                }]],
                ..valid.clone()
            },
            "noncanonical component projections",
        ),
        (
            DataComponentPatchSummary {
                lore: vec!["component nbt".to_string()],
                lore_styled: vec![Vec::new()],
                lore_nbt: vec![NbtSummaryValue::Int(1)],
                ..DataComponentPatchSummary::default()
            },
            "is not a component",
        ),
    ];
    for (summary, expected_error) in cases {
        let mut encoded = Encoder::new();
        let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
        assert!(
            error.to_string().contains(expected_error),
            "{error} did not contain {expected_error:?}"
        );
    }

    let oversized = DataComponentPatchSummary {
        lore: vec!["x".to_string(); MAX_LORE_LINES + 1],
        lore_styled: vec![
            vec![StyledTextRun {
                text: "x".to_string(),
                style: crate::ComponentStyle::default(),
            }];
            MAX_LORE_LINES + 1
        ],
        lore_nbt: vec![NbtSummaryValue::String("x".into()); MAX_LORE_LINES + 1],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    assert!(matches!(
        descriptor.encode(&oversized, &mut encoded),
        Err(ProtocolError::PacketTooLarge(count, max))
            if count == MAX_LORE_LINES + 1 && max == MAX_LORE_LINES
    ));
}

#[test]
fn writable_book_descriptor_round_trips_filterable_strings_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
            .unwrap();
    let summary = DataComponentPatchSummary {
        writable_book_pages: vec!["raw page".into(), "😀".repeat(512).into()],
        writable_book_page_filters: vec![Some("filtered page".into()), None],
        ..DataComponentPatchSummary::default()
    };

    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let encoded = encoded.into_inner();
    let mut expected = Encoder::new();
    expected.write_var_i32(2);
    expected.write_string("raw page");
    expected.write_bool(true);
    expected.write_string("filtered page");
    expected.write_string(&"😀".repeat(512));
    expected.write_bool(false);
    assert_eq!(encoded, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.writable_book_pages, summary.writable_book_pages);
    assert_eq!(
        decoded.writable_book_page_filters,
        summary.writable_book_page_filters
    );

    let mut empty = Encoder::new();
    descriptor
        .encode(&DataComponentPatchSummary::default(), &mut empty)
        .unwrap();
    assert_eq!(empty.into_inner(), [0]);

    let maximal = DataComponentPatchSummary {
        writable_book_pages: vec![Default::default(); MAX_BOOK_PAGES],
        writable_book_page_filters: vec![None; MAX_BOOK_PAGES],
        ..DataComponentPatchSummary::default()
    };
    descriptor.encode(&maximal, &mut Encoder::new()).unwrap();
}

#[test]
fn writable_book_descriptor_enforces_page_and_java_string_bounds() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
            .unwrap();
    let cases = [
        (
            DataComponentPatchSummary {
                writable_book_pages: vec![Default::default()],
                ..DataComponentPatchSummary::default()
            },
            "projections are not aligned",
        ),
        (
            DataComponentPatchSummary {
                writable_book_pages: vec!["x".repeat(MAX_WRITABLE_BOOK_PAGE_CHARS + 1).into()],
                writable_book_page_filters: vec![None],
                ..DataComponentPatchSummary::default()
            },
            "string has 1025 bytes, max is 1024",
        ),
        (
            DataComponentPatchSummary {
                writable_book_pages: vec![Default::default()],
                writable_book_page_filters: vec![Some(
                    "😀".repeat(MAX_WRITABLE_BOOK_PAGE_CHARS / 2 + 1).into(),
                )],
                ..DataComponentPatchSummary::default()
            },
            "string has 1026 bytes, max is 1024",
        ),
    ];
    for (summary, expected_error) in cases {
        let mut encoded = Encoder::new();
        let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
        assert!(
            error.to_string().contains(expected_error),
            "{error} did not contain {expected_error:?}"
        );
    }

    let oversized = DataComponentPatchSummary {
        writable_book_pages: vec![Default::default(); MAX_BOOK_PAGES + 1],
        writable_book_page_filters: vec![None; MAX_BOOK_PAGES + 1],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    assert!(matches!(
        descriptor.encode(&oversized, &mut encoded),
        Err(ProtocolError::PacketTooLarge(count, max))
            if count == MAX_BOOK_PAGES + 1 && max == MAX_BOOK_PAGES
    ));
}

#[test]
fn written_book_descriptor_round_trips_filterable_components_in_wire_order() {
    let raw_page = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String("Raw page".into()),
        },
        NbtSummaryEntry {
            name: "italic".into(),
            value: NbtSummaryValue::Byte(1),
        },
    ]);
    let filtered_page = NbtSummaryValue::String("Filtered page".into());
    let summary = DataComponentPatchSummary {
        written_book: Some(WrittenBookContentSummary {
            title: "Guide".into(),
            title_filter: Some("Safe Guide".into()),
            author: "Alex".into(),
            generation: 2,
            pages: vec!["Raw page".to_string(), "Second page".to_string()],
            page_filters: vec![Some("Filtered page".to_string()), None],
            resolved: true,
        }),
        written_book_pages_nbt: vec![
            Some(raw_page.clone()),
            Some(NbtSummaryValue::String("Second page".into())),
        ],
        written_book_page_filters_nbt: vec![Some(filtered_page.clone()), None],
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)
            .unwrap();
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let encoded = encoded.into_inner();

    let mut expected = Encoder::new();
    expected.write_string("Guide");
    expected.write_bool(true);
    expected.write_string("Safe Guide");
    expected.write_string("Alex");
    expected.write_var_i32(2);
    expected.write_var_i32(2);
    expected.write_bytes(
        &encode_nbt_summary_bounded(Some(&raw_page), i32::MAX as usize, MAX_NBT_DEPTH).unwrap(),
    );
    expected.write_bool(true);
    expected.write_bytes(
        &encode_nbt_summary_bounded(Some(&filtered_page), i32::MAX as usize, MAX_NBT_DEPTH)
            .unwrap(),
    );
    expected.write_bytes(
        &encode_nbt_summary_bounded(
            Some(&NbtSummaryValue::String("Second page".into())),
            i32::MAX as usize,
            MAX_NBT_DEPTH,
        )
        .unwrap(),
    );
    expected.write_bool(false);
    expected.write_bool(true);
    assert_eq!(encoded, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.written_book, summary.written_book);
    assert_eq!(
        decoded.written_book_pages_nbt,
        summary.written_book_pages_nbt
    );
    assert_eq!(
        decoded.written_book_page_filters_nbt,
        summary.written_book_page_filters_nbt
    );
}

#[test]
fn written_book_descriptor_enforces_java_bounds_and_canonical_alignment() {
    let valid = DataComponentPatchSummary {
        written_book: Some(WrittenBookContentSummary {
            title: "😀".repeat(16).into(),
            title_filter: None,
            author: "Author".into(),
            generation: 0,
            pages: vec!["Page".to_string()],
            page_filters: vec![None],
            resolved: false,
        }),
        written_book_pages_nbt: vec![Some(NbtSummaryValue::String("Page".into()))],
        written_book_page_filters_nbt: vec![None],
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)
            .unwrap();
    let mut boundary = Encoder::new();
    descriptor.encode(&valid, &mut boundary).unwrap();
    let mut decoded = DataComponentPatchSummary::default();
    descriptor
        .decode(&mut Decoder::new(&boundary.into_inner()), &mut decoded)
        .unwrap();
    assert_eq!(decoded.written_book, valid.written_book);
    let mut author_boundary = valid.clone();
    author_boundary.written_book.as_mut().unwrap().author =
        format!("{}a", "😀".repeat(16_383)).into();
    descriptor
        .encode(&author_boundary, &mut Encoder::new())
        .unwrap();

    let mut cases = Vec::new();
    cases.push((
        DataComponentPatchSummary::default(),
        "missing written_book_content",
    ));
    let mut invalid_generation = valid.clone();
    invalid_generation.written_book.as_mut().unwrap().generation = 4;
    cases.push((invalid_generation, "outside 0..=3"));
    let mut oversized_title = valid.clone();
    oversized_title.written_book.as_mut().unwrap().title = "😀".repeat(17).into();
    cases.push((oversized_title, "string has 34 bytes, max is 32"));
    let mut oversized_filtered_title = valid.clone();
    oversized_filtered_title
        .written_book
        .as_mut()
        .unwrap()
        .title_filter = Some("😀".repeat(17).into());
    cases.push((oversized_filtered_title, "string has 34 bytes, max is 32"));
    let mut oversized_author = valid.clone();
    oversized_author.written_book.as_mut().unwrap().author = "😀".repeat(16_384).into();
    cases.push((oversized_author, "string has 32768 bytes, max is 32767"));
    let mut misaligned_pages = valid.clone();
    misaligned_pages.written_book_pages_nbt.clear();
    cases.push((misaligned_pages, "not aligned"));
    let mut missing_raw_page = valid.clone();
    missing_raw_page.written_book_pages_nbt[0] = None;
    cases.push((missing_raw_page, "missing raw component nbt"));
    let mut stale_page = valid.clone();
    stale_page.written_book.as_mut().unwrap().pages[0] = "Stale".to_string();
    cases.push((stale_page, "noncanonical component projections"));
    let mut malformed_page = valid.clone();
    malformed_page.written_book_pages_nbt[0] = Some(NbtSummaryValue::Int(1));
    cases.push((malformed_page, "is not a component"));
    let mut mismatched_filter = valid.clone();
    mismatched_filter
        .written_book
        .as_mut()
        .unwrap()
        .page_filters[0] = Some("Filtered".to_string());
    cases.push((mismatched_filter, "filtered projections are not aligned"));

    for (summary, expected_error) in cases {
        let mut encoded = Encoder::new();
        let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }

    let oversized_page_count = MAX_DATA_COMPONENT_LIST_ITEMS + 1;
    let oversized = DataComponentPatchSummary {
        written_book: Some(WrittenBookContentSummary {
            title: "Title".into(),
            title_filter: None,
            author: "Author".into(),
            generation: 0,
            pages: vec!["Page".to_string(); oversized_page_count],
            page_filters: vec![None; oversized_page_count],
            resolved: false,
        }),
        written_book_pages_nbt: vec![
            Some(NbtSummaryValue::String("Page".into()));
            oversized_page_count
        ],
        written_book_page_filters_nbt: vec![None; oversized_page_count],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    let error = descriptor.encode(&oversized, &mut encoded).unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(actual, max)
            if actual == oversized_page_count && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));

    for (wire, expected_error) in [
        (
            {
                let mut wire = Encoder::new();
                wire.write_string(&"😀".repeat(17));
                wire.write_bool(false);
                wire.write_string("Author");
                wire.write_var_i32(0);
                wire.write_var_i32(0);
                wire.write_bool(false);
                wire.into_inner()
            },
            "string has 34 bytes, max is 32",
        ),
        (
            {
                let mut wire = Encoder::new();
                wire.write_string("Title");
                wire.write_bool(false);
                wire.write_string("Author");
                wire.write_var_i32(-1);
                wire.write_var_i32(0);
                wire.write_bool(false);
                wire.into_inner()
            },
            "outside 0..=3",
        ),
    ] {
        let error = descriptor
            .decode(
                &mut Decoder::new(&wire),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }
}

#[test]
fn descriptor_handler_projects_component_value() {
    let mut payload = Encoder::new();
    payload.write_var_i32(99);
    let payload = payload.into_inner();
    let mut decoder = Decoder::new(&payload);
    let mut summary = DataComponentPatchSummary::default();

    vanilla_data_component_descriptor(1)
        .unwrap()
        .decode(&mut decoder, &mut summary)
        .unwrap();

    assert_eq!(summary.max_stack_size, Some(99));
    assert!(decoder.is_empty());
}

#[test]
fn foundational_descriptors_encode_exact_network_values() {
    let summary = DataComponentPatchSummary {
        max_stack_size: Some(i32::MIN),
        max_damage: Some(0),
        damage: Some(-1),
        unbreakable: true,
        ..DataComponentPatchSummary::default()
    };
    for (type_id, expected_value) in [
        (
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            Some(i32::MIN),
        ),
        (VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, Some(0)),
        (VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, Some(-1)),
        (VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, None),
    ] {
        let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        if let Some(expected_value) = expected_value {
            let mut decoder = Decoder::new(&encoded);
            assert_eq!(decoder.read_var_i32().unwrap(), expected_value);
            assert!(decoder.is_empty());
        } else {
            assert!(encoded.is_empty());
        }
    }
}

#[test]
fn custom_data_descriptor_round_trips_exact_compound_nbt() {
    let custom_data = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "answer".into(),
            value: NbtSummaryValue::Int(42),
        },
        NbtSummaryEntry {
            name: "owner".into(),
            value: NbtSummaryValue::String("Alex".into()),
        },
    ]);
    let summary = DataComponentPatchSummary {
        custom_data: Some(custom_data.clone()),
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let encoded = encoded.into_inner();
    assert_eq!(
        encoded,
        [
            10, // unnamed compound root
            3, 0, 6, b'a', b'n', b's', b'w', b'e', b'r', 0, 0, 0, 42, 8, 0, 5, b'o', b'w', b'n',
            b'e', b'r', 0, 4, b'A', b'l', b'e', b'x', 0, // compound end
        ]
    );

    let mut decoder = Decoder::new(&encoded);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.custom_data, Some(custom_data));
}

#[test]
fn custom_data_descriptor_rejects_missing_null_and_scalar_roots() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID).unwrap();
    for custom_data in [None, Some(NbtSummaryValue::Int(42))] {
        let summary = DataComponentPatchSummary {
            custom_data,
            ..DataComponentPatchSummary::default()
        };
        let error = descriptor
            .encode(&summary, &mut Encoder::new())
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains(if summary.custom_data.is_some() {
                    "must be a non-null compound tag"
                } else {
                    "missing custom_data"
                }),
            "{error}"
        );
    }

    for wire in [&[0][..], &[3, 0, 0, 0, 42][..]] {
        let mut decoder = Decoder::new(wire);
        let error = descriptor
            .decode(&mut decoder, &mut DataComponentPatchSummary::default())
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must be a non-null compound tag"),
            "{error}"
        );
    }
}

#[test]
fn entity_data_descriptors_encode_exact_holder_and_compound_values() {
    let entity_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "Health".into(),
        value: NbtSummaryValue::Short(5),
    }]);
    let bucket_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "Variant".into(),
        value: NbtSummaryValue::Int(2),
    }]);
    let block_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "Delay".into(),
        value: NbtSummaryValue::Short(20),
    }]);
    let summary = DataComponentPatchSummary {
        entity_data_entity_type_id: Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        entity_data_tag: Some(entity_tag),
        codec_backed_nbt: BTreeMap::from([(
            VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
            bucket_tag,
        )]),
        block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID),
        block_entity_data_tag: Some(block_tag),
        ..DataComponentPatchSummary::default()
    };

    for (type_id, expected) in [
        (
            VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
            vec![
                0x96, 0x01, // zombie holder 150
                10, 2, 0, 6, b'H', b'e', b'a', b'l', b't', b'h', 0, 5, 0,
            ],
        ),
        (
            VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
            vec![
                10, 3, 0, 7, b'V', b'a', b'r', b'i', b'a', b'n', b't', 0, 0, 0, 2, 0,
            ],
        ),
        (
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            vec![
                9, // mob spawner holder
                10, 2, 0, 5, b'D', b'e', b'l', b'a', b'y', 0, 20, 0,
            ],
        ),
    ] {
        let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        assert_eq!(encoded.into_inner(), expected);
    }
}

#[test]
fn typed_entity_data_decode_strips_id_and_rejects_invalid_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID).unwrap();
    let mut wire = Encoder::new();
    wire.write_var_i32(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    wire.write_bytes(&[
        10, // compound
        8, 0, 2, b'i', b'd', 0, 16, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':',
        b'z', b'o', b'm', b'b', b'i', b'e', 2, 0, 6, b'H', b'e', b'a', b'l', b't', b'h', 0, 5, 0,
    ]);
    let wire = wire.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut summary = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut summary).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(
        summary.entity_data_tag,
        Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "Health".into(),
            value: NbtSummaryValue::Short(5),
        }]))
    );

    for (type_id, wire, expected_error) in [
        (
            VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
            {
                let mut wire = Encoder::new();
                wire.write_var_i32(VANILLA_ENTITY_TYPES.len() as i32);
                wire.write_bytes(&[10, 0]);
                wire.into_inner()
            },
            "invalid vanilla entity type holder",
        ),
        (
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            {
                let mut wire = Encoder::new();
                wire.write_var_i32(VANILLA_BLOCK_ENTITY_TYPES.len() as i32);
                wire.write_bytes(&[10, 0]);
                wire.into_inner()
            },
            "invalid vanilla block entity type holder",
        ),
        (
            VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
            vec![3, 0, 0, 0, 42],
            "must be a non-null compound tag",
        ),
        (
            VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
            {
                let mut wire = Encoder::new();
                wire.write_var_i32(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
                wire.write_u8(0);
                wire.into_inner()
            },
            "must be a non-null compound tag",
        ),
        (
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            {
                let mut wire = Encoder::new();
                wire.write_var_i32(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID);
                wire.write_bytes(&[3, 0, 0, 0, 42]);
                wire.into_inner()
            },
            "must be a non-null compound tag",
        ),
    ] {
        let error = vanilla_data_component_descriptor(type_id)
            .unwrap()
            .decode(
                &mut Decoder::new(&wire),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }
}

#[test]
fn entity_data_descriptors_reject_missing_scalar_and_embedded_id_summaries() {
    let compound = NbtSummaryValue::Compound(Vec::new());
    let cases = [
        (
            VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
            DataComponentPatchSummary {
                entity_data_entity_type_id: Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
                ..DataComponentPatchSummary::default()
            },
            "missing entity_data component value",
        ),
        (
            VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
            DataComponentPatchSummary {
                codec_backed_nbt: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                    NbtSummaryValue::Int(42),
                )]),
                ..DataComponentPatchSummary::default()
            },
            "must be a non-null compound tag",
        ),
        (
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            DataComponentPatchSummary {
                block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID),
                block_entity_data_tag: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String("minecraft:mob_spawner".into()),
                }])),
                ..DataComponentPatchSummary::default()
            },
            "must not contain top-level id",
        ),
    ];
    for (type_id, summary, expected_error) in cases {
        let mut encoded = Encoder::new();
        let error = vanilla_data_component_descriptor(type_id)
            .unwrap()
            .encode(&summary, &mut encoded)
            .unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }

    let valid_entity = DataComponentPatchSummary {
        entity_data_entity_type_id: Some(VANILLA_ENTITY_TYPES.len() as i32),
        entity_data_tag: Some(compound),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    let error = vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID)
        .unwrap()
        .encode(&valid_entity, &mut encoded)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("invalid vanilla entity type holder"),
        "{error}"
    );
}

#[test]
fn profile_descriptor_encodes_exact_game_and_partial_values() {
    let uuid = Uuid::from_u128(1);
    let game_profile = DataComponentPatchSummary {
        profile: Some(ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::GameProfile,
            uuid: Some(uuid),
            name: Some("Alex".to_string()),
            properties: vec![GameProfilePropertySummary {
                name: "p".into(),
                value: "v".into(),
                signature: None,
            }],
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary {
                model: Some(PlayerModelTypeSummary::Wide),
                ..PlayerSkinPatchSummary::default()
            },
        }),
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    descriptor.encode(&game_profile, &mut encoded).unwrap();
    assert_eq!(
        encoded.into_inner(),
        [
            1, // Either.left(GameProfile)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // UUID
            4, b'A', b'l', b'e', b'x', // name
            1,    // property count
            1, b'p', 1, b'v', 0, // property and absent signature
            0, 0, 0, // absent body, cape, and elytra
            1, 0, // present wide model
        ]
    );

    let partial_profile = DataComponentPatchSummary {
        profile: Some(ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid),
            name: None,
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary {
                body: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/wide/steve".to_string(),
                    texture_path: "minecraft:textures/entity/player/wide/steve.png".to_string(),
                }),
                cape: None,
                elytra: Some(ResourceTextureSummary {
                    asset_id: "minecraft:entity/player/elytra".to_string(),
                    texture_path: "minecraft:textures/entity/player/elytra.png".to_string(),
                }),
                model: Some(PlayerModelTypeSummary::Slim),
            },
        }),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&partial_profile, &mut encoded).unwrap();
    assert_eq!(
        encoded.into_inner(),
        [
            0, // Either.right(Partial)
            0, // absent name
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // UUID
            0, // property count
            1, 34, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b'e', b'n', b't',
            b'i', b't', b'y', b'/', b'p', b'l', b'a', b'y', b'e', b'r', b'/', b'w', b'i', b'd',
            b'e', b'/', b's', b't', b'e', b'v', b'e', // body
            0,    // absent cape
            1, 30, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b'e', b'n', b't',
            b'i', b't', b'y', b'/', b'p', b'l', b'a', b'y', b'e', b'r', b'/', b'e', b'l', b'y',
            b't', b'r', b'a', // elytra
            1, 1, // present slim model
        ]
    );
}

#[test]
fn profile_descriptor_round_trips_java_utf16_name_boundary() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID).unwrap();
    let name = "😀".repeat(8);
    let summary = DataComponentPatchSummary {
        profile: Some(ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: None,
            name: Some(name.clone()),
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        }),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let encoded = encoded.into_inner();
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.profile, summary.profile);

    let mut oversized = encoded;
    oversized[1] = 1;
    oversized[2] = 36;
    oversized.splice(3..35, "😀".repeat(9).into_bytes());
    let error = descriptor
        .decode(
            &mut Decoder::new(&oversized),
            &mut DataComponentPatchSummary::default(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::StringTooLong {
            actual: 18,
            max: MAX_PLAYER_NAME_CHARS
        }
    ));

    let mut too_many_properties = Encoder::new();
    too_many_properties.write_bool(false);
    too_many_properties.write_bool(false);
    too_many_properties.write_bool(false);
    too_many_properties.write_var_i32((MAX_PROFILE_PROPERTIES + 1) as i32);
    let too_many_properties = too_many_properties.into_inner();
    let error = descriptor
        .decode(
            &mut Decoder::new(&too_many_properties),
            &mut DataComponentPatchSummary::default(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(
            actual,
            MAX_PROFILE_PROPERTIES
        ) if actual == MAX_PROFILE_PROPERTIES + 1
    ));

    let mut oversized_property_name = Encoder::new();
    oversized_property_name.write_bool(false);
    oversized_property_name.write_bool(false);
    oversized_property_name.write_bool(false);
    oversized_property_name.write_var_i32(1);
    oversized_property_name.write_string(&"😀".repeat(33));
    let oversized_property_name = oversized_property_name.into_inner();
    let error = descriptor
        .decode(
            &mut Decoder::new(&oversized_property_name),
            &mut DataComponentPatchSummary::default(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::StringTooLong {
            actual: 66,
            max: MAX_PROFILE_PROPERTY_NAME_CHARS
        }
    ));
}

#[test]
fn profile_descriptor_rejects_noncanonical_and_oversized_summaries() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID).unwrap();
    let valid = || ResolvableProfileSummary {
        kind: ResolvableProfileKindSummary::Partial,
        uuid: None,
        name: Some("Alex".to_string()),
        properties: Vec::new(),
        profile_textures: None,
        skin_patch: PlayerSkinPatchSummary::default(),
    };
    let mut cases = Vec::new();

    cases.push((
        DataComponentPatchSummary::default(),
        "missing profile component value",
    ));

    let mut missing_game_uuid = valid();
    missing_game_uuid.kind = ResolvableProfileKindSummary::GameProfile;
    cases.push((
        DataComponentPatchSummary {
            profile: Some(missing_game_uuid),
            ..DataComponentPatchSummary::default()
        },
        "missing its UUID",
    ));

    let mut missing_game_name = valid();
    missing_game_name.kind = ResolvableProfileKindSummary::GameProfile;
    missing_game_name.uuid = Some(Uuid::from_u128(1));
    missing_game_name.name = None;
    cases.push((
        DataComponentPatchSummary {
            profile: Some(missing_game_name),
            ..DataComponentPatchSummary::default()
        },
        "missing its name",
    ));

    let mut oversized_name = valid();
    oversized_name.name = Some("😀".repeat(9));
    cases.push((
        DataComponentPatchSummary {
            profile: Some(oversized_name),
            ..DataComponentPatchSummary::default()
        },
        "string has 18 bytes",
    ));

    let mut too_many_properties = valid();
    too_many_properties.properties = (0..=MAX_PROFILE_PROPERTIES)
        .map(|index| GameProfilePropertySummary {
            name: format!("p{index}").into(),
            value: "value".into(),
            signature: None,
        })
        .collect();
    cases.push((
        DataComponentPatchSummary {
            profile: Some(too_many_properties),
            ..DataComponentPatchSummary::default()
        },
        "exceeds configured maximum 16",
    ));

    for (property, expected_error) in [
        (
            GameProfilePropertySummary {
                name: "p".repeat(MAX_PROFILE_PROPERTY_NAME_CHARS + 1).into(),
                value: "value".into(),
                signature: None,
            },
            "max is 64",
        ),
        (
            GameProfilePropertySummary {
                name: "p".into(),
                value: "v".repeat(MAX_STRING_CHARS + 1).into(),
                signature: None,
            },
            "max is 32767",
        ),
        (
            GameProfilePropertySummary {
                name: "p".into(),
                value: "value".into(),
                signature: Some("s".repeat(MAX_PROFILE_SIGNATURE_CHARS + 1).into()),
            },
            "max is 1024",
        ),
    ] {
        let mut oversized_property = valid();
        oversized_property.properties.push(property);
        cases.push((
            DataComponentPatchSummary {
                profile: Some(oversized_property),
                ..DataComponentPatchSummary::default()
            },
            expected_error,
        ));
    }

    let mut stale_profile_textures = valid();
    stale_profile_textures.profile_textures = Some(ProfileTexturesSummary {
        skin: None,
        cape: None,
        elytra: None,
    });
    cases.push((
        DataComponentPatchSummary {
            profile: Some(stale_profile_textures),
            ..DataComponentPatchSummary::default()
        },
        "noncanonical derived texture values",
    ));

    let mut stale_path = valid();
    stale_path.skin_patch.body = Some(ResourceTextureSummary {
        asset_id: "minecraft:entity/player/wide/steve".to_string(),
        texture_path: "minecraft:wrong.png".to_string(),
    });
    cases.push((
        DataComponentPatchSummary {
            profile: Some(stale_path),
            ..DataComponentPatchSummary::default()
        },
        "noncanonical derived texture values",
    ));

    let mut invalid_asset = valid();
    invalid_asset.skin_patch.body = Some(ResourceTextureSummary {
        asset_id: "minecraft:Bad Asset".to_string(),
        texture_path: "minecraft:textures/Bad Asset.png".to_string(),
    });
    cases.push((
        DataComponentPatchSummary {
            profile: Some(invalid_asset),
            ..DataComponentPatchSummary::default()
        },
        "invalid resource location",
    ));

    for (summary, expected_error) in cases {
        let mut encoded = Encoder::new();
        let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
        assert!(
            error
                .to_string()
                .to_ascii_lowercase()
                .contains(&expected_error.to_ascii_lowercase()),
            "{error}"
        );
    }
}

#[test]
fn component_patch_loop_dispatches_through_catalog_descriptor() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_var_i32(1);
    payload.write_var_i32(1);
    payload.write_var_i32(64);
    payload.write_var_i32(20);

    let payload = payload.into_inner();
    let mut decoder = Decoder::new(&payload);
    let summary = decode_data_component_patch_summary(&mut decoder).unwrap();

    assert_eq!(summary.added_type_ids, vec![1]);
    assert_eq!(summary.max_stack_size, Some(64));
    assert_eq!(summary.removed_type_ids, vec![20]);
    assert!(decoder.is_empty());
}

#[test]
fn value_level_encoder_delegates_to_scalar_object_list_and_float_descriptors() {
    assert_eq!(
        encode_data_component_value_summary(
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            &DataComponentPatchSummary {
                max_stack_size: Some(16),
                ..DataComponentPatchSummary::default()
            }
        )
        .unwrap(),
        [16]
    );
    assert_eq!(
        encode_data_component_value_summary(
            VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
            &DataComponentPatchSummary {
                potion_duration_scale_bits: Some(0.125_f32.to_bits()),
                ..DataComponentPatchSummary::default()
            }
        )
        .unwrap(),
        0.125_f32.to_be_bytes()
    );

    let custom_data = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "values".into(),
        value: NbtSummaryValue::List(vec![NbtSummaryValue::Int(1), NbtSummaryValue::Int(2)]),
    }]);
    let encoded = encode_data_component_value_summary(
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        &DataComponentPatchSummary {
            custom_data: Some(custom_data.clone()),
            ..DataComponentPatchSummary::default()
        },
    )
    .unwrap();
    assert_eq!(
        crate::decode_nbt_summary(&encoded).unwrap(),
        Some(custom_data)
    );
}

#[test]
fn writable_book_java_utf8_decodes_raw_and_filtered_wire_strings() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
            .unwrap();
    let bytes = [1, 3, 0xed, 0xa0, 0x80, 1, 2, 0xed, 0xb0];
    let mut patch = DataComponentPatchSummary::default();
    let mut reader = Decoder::new(&bytes);
    descriptor.decode(&mut reader, &mut patch).unwrap();
    assert!(reader.is_empty());
    assert_eq!(patch.writable_book_pages, ["�"]);
    assert_eq!(patch.writable_book_page_filters, [Some("�".into())]);
    let bytes = [1, 2, 0x3f, 0xac, 0];
    descriptor
        .decode(&mut Decoder::new(&bytes), &mut patch)
        .unwrap();
    assert_eq!(patch.writable_book_pages, ["?�"]);
}
