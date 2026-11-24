use super::*;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;

fn definition() -> FontUnihexProviderDefinition {
    FontUnihexProviderDefinition {
        hex_file: "minecraft:font/missing.zip".into(),
        size_overrides: Vec::new(),
    }
}

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn missing_archive_and_explicit_zip_content_failures_are_typed() {
    let root = std::env::temp_dir().join(format!("bbb-unihex-missing-{}", std::process::id()));
    let roots = PackRoots::from_paths(root.clone(), root.clone(), root);
    assert!(load_unihex_glyph_source(&roots, &definition())
        .unwrap_err()
        .is::<UnihexProviderInvalid>());
    assert!(
        parse_unihex_archive(Cursor::new(b"not a ZIP"), &definition())
            .unwrap_err()
            .is::<UnihexProviderInvalid>()
    );
    for error in [
        zip::result::ZipError::UnsupportedArchive("format"),
        zip::result::ZipError::CompressionMethodNotSupported(999),
    ] {
        assert!(classify_zip_error(error).is::<UnihexProviderInvalid>());
    }
    for error in [
        zip::result::ZipError::FileNotFound,
        zip::result::ZipError::Io(io::Error::new(io::ErrorKind::InvalidData, "CRC")),
    ] {
        assert!(!classify_zip_error(error).is::<UnihexProviderInvalid>());
    }
}

#[test]
fn semantic_prefix_cannot_hide_current_or_later_hex_crc_failure() {
    for later_entry in [false, true] {
        let mut bytes = if later_entry {
            archive(&[("first.hex", b"0041:00\n"), ("second.hex", b"0042:00\n")])
        } else {
            archive(&[("first.hex", b"0041:00\n0042:00\n")])
        };
        let offset = bytes
            .windows(8)
            .position(|window| window == b"0042:00\n")
            .unwrap();
        bytes[offset + 5] = b'1';
        let error = parse_unihex_archive(Cursor::new(bytes), &definition()).unwrap_err();
        assert!(!error.is::<UnihexProviderInvalid>(), "{error:#}");
        assert!(
            error.chain().any(|cause| cause
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.kind() == io::ErrorKind::InvalidData)),
            "{error:#}"
        );
    }
}

#[test]
fn semantic_prefix_cannot_hide_later_entry_or_total_declared_budget() {
    for total in [false, true] {
        let entries = if total { 5 } else { 1 };
        let mut names = vec!["first.hex".to_owned()];
        names.extend((0..entries).map(|i| format!("ignored-{i}.txt")));
        let entries = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                (
                    name.as_str(),
                    if i == 0 { &b"0041:00\n"[..] } else { &b""[..] },
                )
            })
            .collect::<Vec<_>>();
        let mut bytes = archive(&entries);
        let offsets = bytes
            .windows(4)
            .enumerate()
            .filter_map(|(i, window)| (window == b"PK\x01\x02").then_some(i))
            .collect::<Vec<_>>();
        for offset in offsets.into_iter().skip(1) {
            let size = MAX_UNIHEX_ENTRY_BYTES as u32 + u32::from(!total);
            bytes[offset + 24..offset + 28].copy_from_slice(&size.to_le_bytes());
        }
        let error = parse_unihex_archive(Cursor::new(bytes), &definition()).unwrap_err();
        assert!(!error.is::<UnihexProviderInvalid>(), "{error:#}");
        assert!(
            format!("{error:#}").contains(if total {
                "expands to more"
            } else {
                "byte limit"
            }),
            "{error:#}"
        );
    }
}

#[test]
fn semantic_prefix_drain_keeps_line_and_actual_byte_limits_hard() {
    let payload = format!("0041:00\n{}\n", "0".repeat(MAX_UNIHEX_LINE_BYTES + 1));
    let error = parse_unihex_archive(
        Cursor::new(archive(&[("a.hex", payload.as_bytes())])),
        &definition(),
    )
    .unwrap_err();
    assert!(!error.is::<UnihexProviderInvalid>());
    for total_limit in [false, true] {
        let mut total = if total_limit {
            MAX_UNIHEX_TOTAL_UNCOMPRESSED_BYTES - 9
        } else {
            0
        };
        let mut reader = BufReader::with_capacity(
            64 * 1024,
            BoundedEntry {
                reader: Cursor::new(b"0041:00\n0000\n"),
                entry_read: if total_limit {
                    0
                } else {
                    MAX_UNIHEX_ENTRY_BYTES - 9
                },
                total_read: &mut total,
            },
        );
        assert!(parse_hex_stream(&mut reader, &mut BTreeMap::new())
            .unwrap_err()
            .is::<UnihexProviderInvalid>());
        let mut scratch = Vec::new();
        let error = read_until_bounded(&mut reader, b'\n', &mut scratch).unwrap_err();
        assert!(!error.is::<UnihexProviderInvalid>());
        assert!(format!("{error:#}").contains("byte"));
    }
}

#[test]
fn underlying_read_failure_stays_hard() {
    struct Failed;
    impl Read for Failed {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "fixture"))
        }
    }
    let mut reader = BufReader::new(Failed);
    let error = parse_hex_stream(&mut reader, &mut BTreeMap::new()).unwrap_err();
    assert!(!error.is::<UnihexProviderInvalid>());
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::PermissionDenied
    );
}

#[test]
fn retained_derived_budget_is_hard() {
    let nonce = bbb_client_clock::system_now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("bbb-unihex-budget-{}-{nonce}", std::process::id()));
    let assets = root.join("assets/minecraft");
    std::fs::create_dir_all(assets.join("font")).unwrap();
    let bytes = archive(&[("a.hex", format!("D800:{}\n", "80".repeat(16)).as_bytes())]);
    std::fs::write(assets.join("font/missing.zip"), &bytes).unwrap();
    let roots = PackRoots::from_paths(root.clone(), root.clone(), assets);
    let selected =
        bbb_client_assets::SelectedClientPackStack::from_startup_dirs(&roots, std::iter::empty())
            .unwrap();
    let roots = roots
        .with_client_resource_generation_limits(0, selected, 1_000_000, 95)
        .unwrap();
    let error = load_unihex_glyph_source(&roots, &definition()).expect_err("derived byte limit");
    assert!(!error.is::<UnihexProviderInvalid>(), "{error:#}");
    assert!(
        format!("{error:#}").contains("retained unihex glyph catalog"),
        "{error:#}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn atlas_width_budget_is_hard() {
    let mut bits = BTreeMap::new();
    bits.insert(
        65,
        RawUnihexGlyph {
            rows: [0; 16],
            bit_width: 8,
        },
    );
    let mut definition = definition();
    definition
        .size_overrides
        .push(super::super::providers::FontUnihexSizeOverride {
            from: u32::from('A'),
            to: u32::from('A'),
            left: 0,
            right: UNIHEX_PAGE_WIDTH as i32,
        });
    let error = apply_dimensions_and_rasterize(bits, &definition).unwrap_err();
    assert!(!error.is::<UnihexProviderInvalid>());
    assert!(error.to_string().contains("wider than the atlas page"));
}

#[test]
fn unreadable_local_header_cannot_hide_declared_entry_budget() {
    let mut bytes = archive(&[("a.hex", b"0041:00\n")]);
    let central = bytes
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    bytes[central + 24..central + 28]
        .copy_from_slice(&(MAX_UNIHEX_ENTRY_BYTES as u32 + 1).to_le_bytes());
    bytes[0] = 0;
    let error = parse_unihex_archive(Cursor::new(bytes), &definition()).unwrap_err();
    assert!(!error.is::<UnihexProviderInvalid>(), "{error:#}");
    assert!(
        format!("{error:#}").contains("before budget admission"),
        "{error:#}"
    );
}
