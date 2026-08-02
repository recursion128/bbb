//! Bounded loader for Vanilla `UnihexProvider`: reads every `.hex` entry in a
//! selected ZIP resource, applies ordered size overrides, and rasterizes the
//! retained 16-row glyphs into one deterministic page.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek},
};

use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation};
use zip::ZipArchive;

use super::providers::FontUnihexProviderDefinition;

const MAX_UNIHEX_ARCHIVE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_UNIHEX_ENTRIES: usize = 64;
const MAX_UNIHEX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_UNIHEX_TOTAL_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_UNIHEX_LINE_BYTES: usize = 160;
const MAX_UNIHEX_GLYPHS: usize = 1_114_112;
pub(super) const UNIHEX_PAGE_WIDTH: u32 = 8_192;
const UNIHEX_GLYPH_HEIGHT: u32 = 16;
const UNIHEX_GLYPH_GAP: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawUnihexGlyph {
    rows: [u32; 16],
    bit_width: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UnihexAtlasGlyph {
    pub codepoint: char,
    pub x: u32,
    pub y: u32,
    pub pixel_width: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UnihexAtlasPage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub glyphs: Vec<UnihexAtlasGlyph>,
}

pub(super) fn load_unihex_atlas_page(
    roots: &PackRoots,
    definition: &FontUnihexProviderDefinition,
) -> Result<UnihexAtlasPage> {
    let location = ResourceLocation::parse(&definition.hex_file)?;
    let stack = roots.resource_stack();
    let resource = stack
        .get_resource(&location)
        .with_context(|| format!("missing unihex archive {}", location.id()))?;
    stack.validate_resource(&resource, "unihex archive")?;
    let file = File::open(&resource.path)
        .with_context(|| format!("open unihex archive {}", resource.path.display()))?;
    let size = file
        .metadata()
        .with_context(|| format!("read unihex archive metadata {}", resource.path.display()))?
        .len();
    if size > MAX_UNIHEX_ARCHIVE_BYTES {
        bail!(
            "unihex archive {} is {size} bytes, exceeding the {MAX_UNIHEX_ARCHIVE_BYTES} byte limit",
            resource.path.display()
        );
    }
    let page = parse_unihex_archive(file, definition)
        .with_context(|| format!("load unihex archive {}", resource.path.display()))?;
    let retained_bytes = u64::try_from(page.glyphs.len())
        .context("unihex glyph count overflow")?
        .checked_mul(96)
        .context("unihex retained byte count overflow")?;
    stack.reserve_derived_resource_bytes(retained_bytes, "retained unihex glyph catalog")?;
    Ok(page)
}

fn parse_unihex_archive<R: Read + Seek>(
    reader: R,
    definition: &FontUnihexProviderDefinition,
) -> Result<UnihexAtlasPage> {
    let mut archive = ZipArchive::new(reader).context("parse unihex ZIP")?;
    if archive.len() > MAX_UNIHEX_ENTRIES {
        bail!(
            "unihex ZIP has {} entries, exceeding the {MAX_UNIHEX_ENTRIES} entry limit",
            archive.len()
        );
    }
    let mut total_uncompressed = 0_u64;
    let mut total_read = 0_u64;
    let mut glyphs = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("open unihex ZIP entry {index}"))?;
        let entry_size = entry.size();
        if entry_size > MAX_UNIHEX_ENTRY_BYTES {
            bail!(
                "unihex ZIP entry {:?} is {entry_size} bytes, exceeding the {MAX_UNIHEX_ENTRY_BYTES} byte limit",
                entry.name()
            );
        }
        total_uncompressed = total_uncompressed
            .checked_add(entry_size)
            .context("unihex ZIP total uncompressed size overflow")?;
        if total_uncompressed > MAX_UNIHEX_TOTAL_UNCOMPRESSED_BYTES {
            bail!("unihex ZIP expands to more than {MAX_UNIHEX_TOTAL_UNCOMPRESSED_BYTES} bytes");
        }
        if entry.name().ends_with(".hex") {
            let read = parse_hex_stream(&mut entry, &mut glyphs)
                .with_context(|| format!("parse unihex entry {:?}", entry.name()))?;
            if read > MAX_UNIHEX_ENTRY_BYTES {
                bail!(
                    "unihex ZIP entry {:?} produced more than {MAX_UNIHEX_ENTRY_BYTES} bytes",
                    entry.name()
                );
            }
            total_read = total_read
                .checked_add(read)
                .context("unihex ZIP actual byte count overflow")?;
            if total_read > MAX_UNIHEX_TOTAL_UNCOMPRESSED_BYTES {
                bail!("unihex ZIP produced more than {MAX_UNIHEX_TOTAL_UNCOMPRESSED_BYTES} bytes");
            }
        }
    }
    apply_dimensions_and_rasterize(glyphs, definition)
}

fn parse_hex_stream<R: Read>(
    reader: &mut R,
    glyphs: &mut BTreeMap<u32, RawUnihexGlyph>,
) -> Result<u64> {
    let mut line_number = 0_u64;
    let mut bytes_read = 0_u64;
    loop {
        let (codepoint_bytes, found_colon, consumed) = read_until_bounded(reader, b':')?;
        bytes_read = bytes_read
            .checked_add(consumed)
            .context("unihex byte count overflow")?;
        if bytes_read > MAX_UNIHEX_ENTRY_BYTES {
            bail!("unihex data exceeds the {MAX_UNIHEX_ENTRY_BYTES} byte entry limit");
        }
        if codepoint_bytes.is_empty() && !found_colon {
            return Ok(bytes_read);
        }
        if !found_colon || !matches!(codepoint_bytes.len(), 4 | 5 | 6) {
            bail!(
                "invalid entry at line {line_number}: expected 4, 5 or 6 hex digits followed by a colon"
            );
        }
        let codepoint = decode_hex_number(&codepoint_bytes, line_number)?;
        if char::from_u32(codepoint).is_none() {
            bail!("invalid Unicode scalar U+{codepoint:06X} at line {line_number}");
        }
        let (bitmap_bytes, _, consumed) = read_until_bounded(reader, b'\n')?;
        bytes_read = bytes_read
            .checked_add(consumed)
            .context("unihex byte count overflow")?;
        if bytes_read > MAX_UNIHEX_ENTRY_BYTES {
            bail!("unihex data exceeds the {MAX_UNIHEX_ENTRY_BYTES} byte entry limit");
        }
        let bit_width = match bitmap_bytes.len() {
            32 => 8,
            64 => 16,
            96 => 24,
            128 => 32,
            _ => bail!("invalid entry at line {line_number}: expected an (8,16,24,32) x 16 bitmap"),
        };
        let digits_per_row = usize::from(bit_width / 4);
        let mut rows = [0_u32; 16];
        for (row_index, row) in rows.iter_mut().enumerate() {
            let start = row_index * digits_per_row;
            let value =
                decode_hex_number(&bitmap_bytes[start..start + digits_per_row], line_number)?;
            *row = value << (32 - bit_width);
        }
        // `CodepointMap.put`: a later occurrence replaces the earlier glyph.
        glyphs.insert(codepoint, RawUnihexGlyph { rows, bit_width });
        if glyphs.len() > MAX_UNIHEX_GLYPHS {
            bail!("unihex glyph count exceeds the {MAX_UNIHEX_GLYPHS} scalar limit");
        }
        line_number = line_number
            .checked_add(1)
            .context("unihex line number overflow")?;
    }
}

fn read_until_bounded<R: Read>(reader: &mut R, delimiter: u8) -> Result<(Vec<u8>, bool, u64)> {
    let mut bytes = Vec::new();
    let mut consumed = 0_u64;
    let mut byte = [0_u8; 1];
    loop {
        match reader.read(&mut byte).context("read unihex data")? {
            0 => return Ok((bytes, false, consumed)),
            1 if byte[0] == delimiter => {
                return Ok((bytes, true, consumed + 1));
            }
            1 => {
                consumed += 1;
                if bytes.len() >= MAX_UNIHEX_LINE_BYTES {
                    bail!("unihex line exceeds the {MAX_UNIHEX_LINE_BYTES} byte limit");
                }
                bytes.push(byte[0]);
            }
            _ => unreachable!("one-byte read returned more than one byte"),
        }
    }
}

fn decode_hex_number(bytes: &[u8], line: u64) -> Result<u32> {
    bytes.iter().try_fold(0_u32, |value, byte| {
        let digit = match byte {
            b'0'..=b'9' => u32::from(byte - b'0'),
            b'A'..=b'F' => u32::from(byte - b'A' + 10),
            _ => bail!(
                "invalid entry at line {line}: expected uppercase hex digit, got {:?}",
                char::from(*byte)
            ),
        };
        value
            .checked_mul(16)
            .and_then(|value| value.checked_add(digit))
            .context("unihex number overflow")
    })
}

fn apply_dimensions_and_rasterize(
    mut bits: BTreeMap<u32, RawUnihexGlyph>,
    definition: &FontUnihexProviderDefinition,
) -> Result<UnihexAtlasPage> {
    let mut sized = BTreeMap::<u32, (RawUnihexGlyph, i32, i32)>::new();
    for range in &definition.size_overrides {
        // Vanilla loops the inclusive scalar range and removes matching
        // entries. Iterating only the present BTreeMap keys is result- and
        // order-equivalent while bounding work for adversarial wide ranges.
        let matched = bits
            .range(u32::from(range.from)..=u32::from(range.to))
            .map(|(&codepoint, _)| codepoint)
            .collect::<Vec<_>>();
        for codepoint in matched {
            let glyph = bits
                .remove(&codepoint)
                .expect("matched unihex glyph remains until this ordered override");
            sized.insert(codepoint, (glyph, range.left, range.right));
        }
    }
    for (codepoint, glyph) in bits {
        let mask = glyph.rows.iter().fold(0_u32, |mask, row| mask | row);
        let (left, right) = if mask == 0 {
            (0, i32::from(glyph.bit_width))
        } else {
            (
                i32::try_from(mask.leading_zeros()).expect("leading zeros fit i32"),
                i32::try_from(31 - mask.trailing_zeros()).expect("bit column fits i32"),
            )
        };
        sized.insert(codepoint, (glyph, left, right));
    }

    let mut placements = Vec::with_capacity(sized.len());
    let mut x = 0_u32;
    let mut y = 0_u32;
    for (&codepoint, &(_, left, right)) in &sized {
        let pixel_width = u32::try_from(right - left + 1)
            .context("unihex glyph has non-positive retained width")?;
        if pixel_width > UNIHEX_PAGE_WIDTH {
            bail!("unihex glyph U+{codepoint:04X} is wider than the atlas page");
        }
        if x > 0 && x + pixel_width > UNIHEX_PAGE_WIDTH {
            x = 0;
            y = y
                .checked_add(UNIHEX_GLYPH_HEIGHT + UNIHEX_GLYPH_GAP)
                .context("unihex atlas height overflow")?;
        }
        let codepoint = char::from_u32(codepoint).context("unihex scalar changed after parse")?;
        placements.push(UnihexAtlasGlyph {
            codepoint,
            x,
            y,
            pixel_width,
        });
        x = x
            .checked_add(pixel_width + UNIHEX_GLYPH_GAP)
            .context("unihex atlas row overflow")?;
    }
    let height = if placements.is_empty() {
        0
    } else {
        y.checked_add(UNIHEX_GLYPH_HEIGHT)
            .context("unihex atlas height overflow")?
    };
    if height > 8_192 {
        bail!("unihex atlas height {height} exceeds the 8192 pixel texture limit");
    }
    let rgba_len = usize::try_from(UNIHEX_PAGE_WIDTH)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .context("unihex atlas RGBA size overflow")?;
    let mut rgba = vec![0_u8; rgba_len];
    for placement in &placements {
        let codepoint = u32::from(placement.codepoint);
        let (glyph, left, right) = sized[&codepoint];
        for row in 0..UNIHEX_GLYPH_HEIGHT {
            for column in left..=right {
                let set = (0..32).contains(&column)
                    && (glyph.rows[row as usize] & (1_u32 << (31 - column))) != 0;
                if set {
                    let dst_x = placement.x + u32::try_from(column - left).unwrap();
                    let dst_y = placement.y + row;
                    let offset = usize::try_from((dst_y * UNIHEX_PAGE_WIDTH + dst_x) * 4)
                        .context("unihex atlas pixel offset overflow")?;
                    rgba[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
                }
            }
        }
    }
    Ok(UnihexAtlasPage {
        width: UNIHEX_PAGE_WIDTH,
        height,
        rgba,
        glyphs: placements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    fn definition(
        overrides: Vec<super::super::providers::FontUnihexSizeOverride>,
    ) -> FontUnihexProviderDefinition {
        FontUnihexProviderDefinition {
            hex_file: "minecraft:font/test.zip".to_string(),
            size_overrides: overrides,
        }
    }

    fn archive(entries: &[(&str, &[u8])]) -> Cursor<Vec<u8>> {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut bytes);
            for (name, payload) in entries {
                zip.start_file(*name, SimpleFileOptions::default()).unwrap();
                zip.write_all(payload).unwrap();
            }
            zip.finish().unwrap();
        }
        bytes.set_position(0);
        bytes
    }

    fn repeated_rows(codepoint: &str, row: &str) -> String {
        format!("{codepoint}:{}\n", row.repeat(16))
    }

    #[test]
    fn parses_all_codepoint_and_bitmap_widths_and_duplicate_last_wins() {
        let payload = [
            repeated_rows("0041", "80"),
            repeated_rows("10000", "8000"),
            repeated_rows("010001", "800000"),
            repeated_rows("0041", "80000000"),
        ]
        .concat();
        let page = parse_unihex_archive(
            archive(&[("ignored.txt", b"x"), ("glyphs.hex", payload.as_bytes())]),
            &definition(Vec::new()),
        )
        .unwrap();
        assert_eq!(page.glyphs.len(), 3);
        let a = page
            .glyphs
            .iter()
            .find(|glyph| glyph.codepoint == 'A')
            .unwrap();
        assert_eq!(a.pixel_width, 1);
        let cjk_ext = page
            .glyphs
            .iter()
            .find(|glyph| glyph.codepoint == '\u{10000}')
            .unwrap();
        assert_eq!(cjk_ext.pixel_width, 1);
        assert_eq!(
            page.rgba.iter().filter(|byte| **byte == 255).count(),
            3 * 16 * 4
        );
    }

    #[test]
    fn automatic_blank_bounds_and_ordered_overrides_match_vanilla() {
        let payload = [repeated_rows("4E00", "0000"), repeated_rows("4E01", "0180")].concat();
        let page = parse_unihex_archive(
            archive(&[("glyphs.hex", payload.as_bytes())]),
            &definition(vec![
                super::super::providers::FontUnihexSizeOverride {
                    from: '一',
                    to: '丁',
                    left: 0,
                    right: 15,
                },
                super::super::providers::FontUnihexSizeOverride {
                    from: '一',
                    to: '七',
                    left: 2,
                    right: 12,
                },
            ]),
        )
        .unwrap();
        assert_eq!(page.glyphs[0].pixel_width, 16);
        // The first inclusive range removes both glyphs before the overlapping
        // second range and before automatic bounds.
        assert_eq!(page.glyphs[1].pixel_width, 16);

        let blank = parse_unihex_archive(
            archive(&[("glyphs.hex", repeated_rows("0041", "00").as_bytes())]),
            &definition(Vec::new()),
        )
        .unwrap();
        assert_eq!(blank.glyphs[0].pixel_width, 9);
    }

    #[test]
    fn rejects_lowercase_malformed_width_invalid_scalar_and_long_lines() {
        for (payload, expected) in [
            (
                "00af:00000000000000000000000000000000\n".to_string(),
                "uppercase",
            ),
            ("0041:00\n".to_string(), "expected an (8,16,24,32)"),
            (repeated_rows("D800", "00"), "invalid Unicode scalar"),
            (
                format!("0041:{}\n", "0".repeat(MAX_UNIHEX_LINE_BYTES + 1)),
                "line exceeds",
            ),
        ] {
            let error = parse_unihex_archive(
                archive(&[("glyphs.hex", payload.as_bytes())]),
                &definition(Vec::new()),
            )
            .unwrap_err();
            let error = format!("{error:#}");
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn rejects_archive_entry_count_over_budget() {
        let names = (0..=MAX_UNIHEX_ENTRIES)
            .map(|index| format!("ignored-{index}.txt"))
            .collect::<Vec<_>>();
        let entries = names
            .iter()
            .map(|name| (name.as_str(), &[][..]))
            .collect::<Vec<_>>();
        let error = parse_unihex_archive(archive(&entries), &definition(Vec::new()))
            .unwrap_err()
            .to_string();
        assert!(error.contains("entry limit"), "{error}");
    }
}
