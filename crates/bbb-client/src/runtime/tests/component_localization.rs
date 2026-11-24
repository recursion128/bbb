use super::*;

#[test]
fn boss_notice_and_advancement_names_use_the_current_language() {
    let root = unique_runtime_temp_dir("component-surface-localization");
    write_runtime_bow_item_assets(&root);
    for (code, boss, ok, title) in [
        ("en_us", "Wither", "Done", "Stone Age"),
        (
            "zh_cn",
            "Localized Wither",
            "Confirm",
            "A long advancement title that still needs multiple lines after translation",
        ),
    ] {
        write_runtime_json(
            &runtime_assets_dir(&root)
                .join("lang")
                .join(format!("{code}.json")),
            &serde_json::json!({
                "entity.minecraft.wither": boss,
                "gui.ok": ok,
                "test.advancement.title": title,
            })
            .to_string(),
        );
    }
    write_runtime_json(
        &root.join("sources/26.1/pack.mcmeta"),
        r#"{"language":{"zh_cn":{"region":"CN","name":"Chinese","bidirectional":false}}}"#,
    );
    let roots = bbb_client_assets::PackRoots::from_root(&root).unwrap();
    let runtimes = [
        NativeItemRuntime::load_with_locale(&roots, "en_us").unwrap(),
        NativeItemRuntime::load_with_locale(&roots, "zh_cn").unwrap(),
    ];
    let translated = |key: &str| {
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "translate".into(),
                value: NbtSummaryValue::string(key),
            },
        ]))
        .unwrap()
    };
    let mut world = WorldStore::new();
    let boss_id = uuid::Uuid::from_u128(19);
    add_boss_bar_with_id_and_progress(&mut world, boss_id, 0.5);
    world.apply_boss_event(ProtocolBossEvent {
        id: boss_id,
        operation: ProtocolBossEventOperation::UpdateName {
            name: Box::new(translated("entity.minecraft.wither")),
        },
    });
    let mut advancement = runtime_displayed_advancement("test:localized", None);
    let display = advancement.display.as_mut().unwrap();
    display.title = translated("test.advancement.title");
    display.icon.item_id = runtimes[0].item_protocol_id("minecraft:bow").unwrap();
    display.show_toast = true;
    advancement.requirements = vec![vec!["done".into()]];
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![advancement],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    world.apply_update_advancements(UpdateAdvancements {
        reset: false,
        added: Vec::new(),
        removed: Vec::new(),
        progress: vec![AdvancementProgressSummary {
            id: "test:localized".to_string(),
            criteria: vec![AdvancementCriterionProgressSummary {
                name: "done".into(),
                obtained_epoch_millis: Some(1),
            }],
        }],
        show_advancements: true,
    });
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    world.apply_show_dialog(bbb_protocol::packets::ShowDialog {
        dialog: bbb_protocol::packets::DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Notice")),
            ]),
        },
    });
    let mut input = ClientInputState::new(true);
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    let now = Instant::now();
    let mut toasts = ClientToastRuntime::default();
    toasts.update(&world, now);
    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    let glyph = bbb_client_render::HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6.0,
        ..Default::default()
    };
    for ch in ' '..='~' {
        glyphs.insert_first_wins(ch, glyph);
    }

    for ((code, boss, ok, title), runtime) in [
        ("en_us", "Wither", "Done", "Stone Age"),
        (
            "zh_cn",
            "Localized Wither",
            "Confirm",
            "A long advancement title that still needs multiple lines after translation",
        ),
    ]
    .into_iter()
    .zip(&runtimes)
    {
        let mut hud = HudRenderRuntime::default();
        let RendererFrameUpdate::Replace(bars) =
            hud.boss_bars_update(&world, Some(runtime), 1, now, now, true)
        else {
            panic!("the first localized projection must be published");
        };
        assert_eq!(
            bars[0]
                .name_runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            boss
        );
        let screen = dialog_screen::hud_dialog_screen(
            &input,
            &world,
            input.current_dialog().unwrap(),
            Some(runtime),
            &TerrainTextureState::default(),
            &dialog_screen::DialogHudLocalState::default(),
            PhysicalSize::new(320, 240),
            0.0,
        )
        .unwrap();
        assert!(screen.text_labels.iter().any(|label| label.text == ok));
        assert!(!screen
            .text_labels
            .iter()
            .any(|label| label.text == "gui.ok"));
        let toast = toasts
            .project_advancements(
                &world,
                Some(runtime),
                &TerrainTextureState::default(),
                ItemModelKeybindContext::default(),
                0.0,
            )
            .remove(0);
        assert_eq!(
            toast
                .title_runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            title
        );
        let lines = bbb_client_render::hud_wrap_styled_text(&toast.title_runs, 125, &glyphs);
        assert_eq!(lines.len() == 1, code == "en_us");
    }
    std::fs::remove_dir_all(root).unwrap();
}
