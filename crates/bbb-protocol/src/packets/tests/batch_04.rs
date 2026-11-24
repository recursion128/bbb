use super::*;

#[test]
fn rejects_invalid_configuration_registry_data_identifiers() {
    for registry in [
        "minecraft:Worldgen/Biome",
        "..:biome",
        "mine/craft:biome",
        "mīnecraft:biome",
        "minecraft:pläins",
    ] {
        let mut invalid_registry = Encoder::new();
        invalid_registry.write_string(registry);
        invalid_registry.write_var_i32(0);
        let err = decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_REGISTRY_DATA,
            &invalid_registry.into_inner(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid resource location"));
    }

    let mut invalid_entry = Encoder::new();
    invalid_entry.write_string("minecraft:worldgen/biome");
    invalid_entry.write_var_i32(1);
    invalid_entry.write_string("minecraft:Plains");
    invalid_entry.write_bool(false);
    let err = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_REGISTRY_DATA,
        &invalid_entry.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn rejects_invalid_configuration_registry_data_nbt() {
    let mut root_end = Encoder::new();
    root_end.write_string("minecraft:worldgen/biome");
    root_end.write_var_i32(1);
    root_end.write_string("minecraft:plains");
    root_end.write_bool(true);
    root_end.write_u8(0);
    assert!(decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_REGISTRY_DATA,
        &root_end.into_inner(),
    )
    .is_err());

    let mut trailing = Encoder::new();
    trailing.write_string("minecraft:worldgen/biome");
    trailing.write_var_i32(0);
    trailing.write_u8(99);
    assert!(decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_REGISTRY_DATA,
        &trailing.into_inner(),
    )
    .is_err());
}

#[test]
fn decodes_custom_report_details_in_configuration_and_play() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_string("Server");
    payload.write_string("bbb test shard");
    payload.write_string("Region");
    payload.write_string("local");
    let payload = payload.into_inner();

    let details = BTreeMap::from([
        ("Region".to_string(), "local".to_string()),
        ("Server".to_string(), "bbb test shard".to_string()),
    ]);
    let expected = CustomReportDetails { details };

    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_CUSTOM_REPORT_DETAILS,
            &payload,
        )
        .unwrap(),
        ConfigurationClientbound::CustomReportDetails(expected.clone())
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_REPORT_DETAILS, &payload).unwrap(),
        PlayClientbound::CustomReportDetails(expected)
    );

    let mut trailing = payload;
    trailing.push(0);
    for error in [
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_CUSTOM_REPORT_DETAILS,
            &trailing,
        )
        .unwrap_err(),
        decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_REPORT_DETAILS, &trailing)
            .unwrap_err(),
    ] {
        assert!(error
            .to_string()
            .contains("trailing bytes after custom report details packet"));
    }

    let mut oversized = Encoder::new();
    oversized.write_var_i32(33);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_CUSTOM_REPORT_DETAILS,
        &oversized.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::PacketTooLarge(33, 32)));
}

#[test]
fn decodes_server_links_in_configuration_and_play() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_bool(true);
    payload.write_var_i32(0);
    payload.write_string("https://example.invalid/bugs");
    payload.write_bool(false);
    payload.write_bytes(&nbt_string_root("Rules"));
    payload.write_string("https://example.invalid/rules");
    let payload = payload.into_inner();

    let expected = ServerLinks {
        links: vec![
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::BugReport),
                url: "https://example.invalid/bugs".to_string(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Custom {
                    label: "Rules".to_string(),
                },
                url: "https://example.invalid/rules".to_string(),
            },
        ],
    };

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_SERVER_LINKS, &payload)
            .unwrap(),
        ConfigurationClientbound::ServerLinks(expected.clone())
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SERVER_LINKS, &payload).unwrap(),
        PlayClientbound::ServerLinks(expected)
    );

    let mut trailing = payload;
    trailing.push(0);
    for error in [
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_SERVER_LINKS, &trailing)
            .unwrap_err(),
        decode_play_clientbound(ids::play::CLIENTBOUND_SERVER_LINKS, &trailing).unwrap_err(),
    ] {
        assert!(error
            .to_string()
            .contains("trailing bytes after server links packet"));
    }

    let mut hostile = Encoder::new();
    hostile.write_var_i32(i32::MAX);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_SERVER_LINKS,
        &hostile.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::PacketTooLarge(_, _)));

    let mut truncated = Encoder::new();
    truncated.write_var_i32(16);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_SERVER_LINKS,
        &truncated.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::UnexpectedEof { .. }));
}

/// The registry-data packet this server writes is one this crate's own reader
/// answers with the same registry key, the same entry ids in the same order,
/// and the same payload bytes.
///
/// The order is the assertion that matters: `CommonPlayerSpawnInfo` writes its
/// dimension type as a bare index into the registry the client rebuilds from
/// these entries (`CommonPlayerSpawnInfo.java:43`, `DimensionType.java:57`),
/// so a writer that reorders them silently renames every dimension.
#[test]
fn a_published_registry_round_trips_through_the_registry_data_reader() {
    let payload = crate::encode_nbt_summary(
        &NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "min_y".into(),
            value: NbtSummaryValue::Int(-64),
        }]),
        4096,
    )
    .unwrap();
    let entries = vec![
        RegistryDataEntry {
            id: "minecraft:overworld".to_string(),
            raw_data: Some(payload.clone()),
        },
        RegistryDataEntry {
            id: "minecraft:the_nether".to_string(),
            raw_data: None,
        },
    ];
    let (packet_id, encoded) =
        encode_configuration_registry_data("minecraft:dimension_type", &entries).unwrap();
    assert_eq!(packet_id, ids::configuration::CLIENTBOUND_REGISTRY_DATA);
    let decoded = decode_configuration_clientbound(packet_id, &encoded).unwrap();
    let ConfigurationClientbound::RegistryData(data) = decoded else {
        panic!("the registry data id decodes as registry data");
    };
    assert_eq!(data.registry, "minecraft:dimension_type");
    assert_eq!(data.entries, entries);
    assert_eq!(data.raw_payload_len, encoded.len());

    // An entry that claims a payload and carries none is refused here rather
    // than written as a truncated tag the reader would reject on arrival.
    let error = encode_configuration_registry_data(
        "minecraft:dimension_type",
        &[RegistryDataEntry {
            id: "minecraft:overworld".to_string(),
            raw_data: Some(Vec::new()),
        }],
    )
    .unwrap_err();
    assert!(error.to_string().contains("empty nbt payload"), "{error}");

    let error = encode_configuration_registry_data(
        "minecraft:Dimension_Type",
        &[RegistryDataEntry {
            id: "minecraft:overworld".to_string(),
            raw_data: None,
        }],
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("invalid resource location"),
        "{error}"
    );
}

#[test]
fn server_configuration_tasks_round_trip_through_the_client_decoders() {
    let links = ServerLinks {
        links: vec![ServerLinkEntry {
            link_type: ServerLinkType::Known(ServerLinkKnownType::BugReport),
            url: "https://example.invalid/bugs".to_string(),
        }],
    };
    let (id, payload) = encode_configuration_server_links(&links).unwrap();
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::ServerLinks(links)
    );

    let (id, payload) = encode_configuration_code_of_conduct("Be kind.").unwrap();
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::CodeOfConduct {
            text: "Be kind.".to_string()
        }
    );

    let pack = uuid::Uuid::from_u128(7);
    let prompt = crate::RichTextComponent::plain("Please accept");
    let (id, payload) = encode_configuration_resource_pack_push(
        pack,
        "https://example.invalid/pack.zip",
        "0123456789abcdef0123456789abcdef01234567",
        true,
        Some(&prompt),
    )
    .unwrap();
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::ResourcePackPush(ResourcePackPush {
            id: pack,
            url: "https://example.invalid/pack.zip".to_string(),
            hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
            required: true,
            prompt: Some(crate::RichTextComponent::plain("Please accept")),
        })
    );

    let (id, payload) =
        encode_configuration_resource_pack_response(pack, ResourcePackResponseAction::Declined);
    assert_eq!(
        decode_configuration_serverbound(id, &payload).unwrap(),
        ConfigurationServerbound::ResourcePack {
            id: pack,
            action: ResourcePackResponseAction::Declined,
        }
    );
    let (id, payload) = encode_configuration_accept_code_of_conduct();
    assert_eq!(
        decode_configuration_serverbound(id, &payload).unwrap(),
        ConfigurationServerbound::AcceptCodeOfConduct
    );
}
