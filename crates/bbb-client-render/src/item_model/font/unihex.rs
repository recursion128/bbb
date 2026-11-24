//! Bounded loader for Vanilla `UnihexProvider`: reads every `.hex` entry in a
//! selected ZIP resource, applies ordered size overrides, and retains compact
//! 1-bit glyph bitmaps for lazy on-demand rasterization.

use std::{
    collections::BTreeMap,
    io::{self, BufRead, BufReader, Read, Seek},
};

use anyhow::{bail, Context, Result};
use bbb_client_assets::{PackResourceStack, PackRoots, ResourceLocation};
use zip::ZipArchive;

use super::providers::FontUnihexProviderDefinition;

const MAX_UNIHEX_ARCHIVE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_UNIHEX_ENTRIES: usize = 64;
const MAX_UNIHEX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_UNIHEX_TOTAL_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_UNIHEX_LINE_BYTES: usize = 160;
const MAX_UNIHEX_GLYPHS: usize = 1_114_112;
pub(super) const UNIHEX_PAGE_WIDTH: u32 = 8_192;
pub(super) const UNIHEX_GLYPH_HEIGHT: u32 = 16;

#[derive(Debug)]
pub(super) struct UnihexProviderInvalid(anyhow::Error);

impl std::fmt::Display for UnihexProviderInvalid {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
    }
}

impl std::error::Error for UnihexProviderInvalid {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

fn invalid(message: String) -> anyhow::Error {
    UnihexProviderInvalid(anyhow::anyhow!(message)).into()
}

fn classify_zip_error(error: zip::result::ZipError) -> anyhow::Error {
    match error {
        zip::result::ZipError::InvalidArchive(_)
        | zip::result::ZipError::UnsupportedArchive(_)
        | zip::result::ZipError::CompressionMethodNotSupported(_) => {
            UnihexProviderInvalid(error.into()).into()
        }
        _ => error.into(),
    }
}

struct BoundedEntry<'a, R> {
    reader: R,
    entry_read: u64,
    total_read: &'a mut u64,
}

impl<R: Read> Read for BoundedEntry<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let entry_remaining = MAX_UNIHEX_ENTRY_BYTES.saturating_sub(self.entry_read);
        let total_remaining = MAX_UNIHEX_TOTAL_UNCOMPRESSED_BYTES.saturating_sub(*self.total_read);
        let remaining = entry_remaining.min(total_remaining);
        // Cap buffered reads at the remaining budget so a valid prefix never overshoots into
        // an early limit error before the boundary byte is actually consumed.
        let max_len = if remaining == 0 {
            1
        } else {
            buffer
                .len()
                .min(usize::try_from(remaining).unwrap_or(usize::MAX))
        };
        let read = self.reader.read(&mut buffer[..max_len])?;
        self.entry_read = self
            .entry_read
            .checked_add(read as u64)
            .ok_or_else(|| io::Error::other("unihex entry byte count overflow"))?;
        *self.total_read = self
            .total_read
            .checked_add(read as u64)
            .ok_or_else(|| io::Error::other("unihex ZIP actual byte count overflow"))?;
        if self.entry_read > MAX_UNIHEX_ENTRY_BYTES {
            return Err(io::Error::other("unihex data exceeds the byte entry limit"));
        }
        if *self.total_read > MAX_UNIHEX_TOTAL_UNCOMPRESSED_BYTES {
            return Err(io::Error::other(
                "unihex ZIP produced more than the total byte limit",
            ));
        }
        Ok(read)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawUnihexGlyph {
    rows: [u32; 16],
    bit_width: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UnihexGlyphEntry {
    pub codepoint: u32,
    pub pixel_width: u32,
    /// Offset into `UnihexGlyphSource::bit_rows` (`16 * bytes_per_row` bytes).
    data_offset: u32,
    bytes_per_row: u8,
}

/// Compact 1-bit catalog for a `UnihexProvider`; individual glyphs are expanded
/// to RGBA8 only when first drawn into the dynamic font atlas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UnihexGlyphSource {
    pub glyphs: Vec<UnihexGlyphEntry>,
    bit_rows: Vec<u8>,
}

impl UnihexGlyphSource {
    #[cfg(test)]
    pub(super) fn from_test_glyphs(glyphs: &[(u32, u32)]) -> Self {
        let mut entries = Vec::with_capacity(glyphs.len());
        let mut bit_rows = Vec::new();
        for &(codepoint, pixel_width) in glyphs {
            let bytes_per_row = pixel_width.div_ceil(8).max(1) as usize;
            let data_offset = bit_rows.len() as u32;
            let start = bit_rows.len();
            bit_rows.resize(start + UNIHEX_GLYPH_HEIGHT as usize * bytes_per_row, 0);
            // Set top-left pixel (0, 0) to match previous test helper behavior.
            if pixel_width > 0 {
                bit_rows[start] = 0x80;
            }
            entries.push(UnihexGlyphEntry {
                codepoint,
                pixel_width,
                data_offset,
                bytes_per_row: bytes_per_row as u8,
            });
        }
        Self {
            glyphs: entries,
            bit_rows,
        }
    }

    pub(super) fn rasterize_glyph_rgba(&self, index: usize) -> Vec<u8> {
        let entry = self.glyphs[index];
        let width = entry.pixel_width as usize;
        let height = UNIHEX_GLYPH_HEIGHT as usize;
        let mut rgba = vec![0_u8; width * height * 4];
        let bytes_per_row = usize::from(entry.bytes_per_row);
        let base = entry.data_offset as usize;
        for row in 0..height {
            let row_bits =
                &self.bit_rows[base + row * bytes_per_row..base + (row + 1) * bytes_per_row];
            let dst_row = row * width * 4;
            for col in 0..width {
                if (row_bits[col >> 3] & (0x80 >> (col & 7))) != 0 {
                    let px = dst_row + col * 4;
                    rgba[px..px + 4].copy_from_slice(&[255, 255, 255, 255]);
                }
            }
        }
        rgba
    }

    #[cfg(test)]
    pub(super) fn glyph_pixel_is_set(&self, index: usize, x: u32, y: u32) -> bool {
        let entry = self.glyphs[index];
        if x >= entry.pixel_width || y >= UNIHEX_GLYPH_HEIGHT {
            return false;
        }
        let bytes_per_row = usize::from(entry.bytes_per_row);
        let row_start = entry.data_offset as usize + y as usize * bytes_per_row;
        (self.bit_rows[row_start + (x as usize >> 3)] & (0x80 >> (x & 7))) != 0
    }

    #[cfg(test)]
    pub(super) fn set_pixel_count(&self) -> usize {
        self.bit_rows
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum()
    }
}

pub(super) fn load_unihex_glyph_source(
    roots: &PackRoots,
    definition: &FontUnihexProviderDefinition,
) -> Result<UnihexGlyphSource> {
    let location = ResourceLocation::parse(&definition.hex_file)?;
    let stack = roots.resource_stack();
    let resource = stack
        .get_resource(&location)
        .ok_or_else(|| invalid(format!("missing unihex archive {}", location.id())))?;
    stack.validate_resource(&resource, "unihex archive")?;
    let source = if resource.is_archive_backed() {
        let bytes = resource.read_bounded_bytes(MAX_UNIHEX_ARCHIVE_BYTES, "unihex archive")?;
        parse_unihex_archive_with_budget(std::io::Cursor::new(bytes), definition, Some(&stack))
            .with_context(|| format!("load unihex archive {}", resource.path.display()))?
    } else {
        let (file, size) = bbb_client_assets::open_resource_file(&resource.path)
            .with_context(|| format!("open unihex archive {}", resource.path.display()))?;
        if size > MAX_UNIHEX_ARCHIVE_BYTES {
            bail!(
                "unihex archive {} is {size} bytes, exceeding the {MAX_UNIHEX_ARCHIVE_BYTES} byte limit",
                resource.path.display()
            );
        }
        parse_unihex_archive_with_budget(
            BufReader::with_capacity(64 * 1024, file),
            definition,
            Some(&stack),
        )
        .with_context(|| format!("load unihex archive {}", resource.path.display()))?
    };
    let retained_bytes = u64::try_from(source.glyphs.len())
        .context("unihex glyph count overflow")?
        .checked_mul(96)
        .and_then(|bytes| bytes.checked_add(source.bit_rows.len() as u64))
        .context("unihex retained byte count overflow")?;
    stack.reserve_derived_resource_bytes(retained_bytes, "retained unihex glyph catalog")?;
    Ok(source)
}

#[cfg(test)]
fn parse_unihex_archive<R: Read + Seek>(
    reader: R,
    definition: &FontUnihexProviderDefinition,
) -> Result<UnihexGlyphSource> {
    parse_unihex_archive_with_budget(reader, definition, None)
}

fn parse_unihex_archive_with_budget<R: Read + Seek>(
    reader: R,
    definition: &FontUnihexProviderDefinition,
    budget: Option<&PackResourceStack>,
) -> Result<UnihexGlyphSource> {
    let mut archive = ZipArchive::new(reader)
        .map_err(classify_zip_error)
        .context("parse unihex ZIP")?;
    if archive.len() > MAX_UNIHEX_ENTRIES {
        bail!(
            "unihex ZIP has {} entries, exceeding the {MAX_UNIHEX_ENTRIES} entry limit",
            archive.len()
        );
    }
    let mut total_uncompressed = 0_u64;
    let mut total_read = 0_u64;
    let mut glyphs = BTreeMap::new();
    let mut first_invalid = None;
    for index in 0..archive.len() {
        // Unreadable local headers cannot bypass the following declared-size checks.
        let entry = archive
            .by_index_raw(index)
            .with_context(|| format!("inspect unihex ZIP entry {index} before budget admission"))?;
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
        drop(entry);
        let mut entry = match archive.by_index(index).map_err(classify_zip_error) {
            Ok(entry) => entry,
            Err(error) if error.is::<UnihexProviderInvalid>() => {
                first_invalid.get_or_insert(error);
                continue;
            }
            Err(error) => return Err(error.context(format!("open unihex ZIP entry {index}"))),
        };
        if entry.name().ends_with(".hex") {
            let name = entry.name().to_owned();
            let mut reader = BufReader::with_capacity(
                64 * 1024,
                BoundedEntry {
                    reader: &mut entry,
                    entry_read: 0,
                    total_read: &mut total_read,
                },
            );
            if let Err(error) = parse_hex_stream(&mut reader, &mut glyphs)
                .with_context(|| format!("parse unihex entry {name:?}"))
            {
                if !error.is::<UnihexProviderInvalid>() {
                    return Err(error);
                }
                first_invalid.get_or_insert(error);
                // ZIP CRC and actual expansion failures must outrank bad HEX content.
                let mut drain_buf = Vec::with_capacity(MAX_UNIHEX_LINE_BYTES);
                loop {
                    let (newline, consumed) =
                        read_until_bounded(&mut reader, b'\n', &mut drain_buf)?;
                    if consumed == 0 && !newline {
                        break;
                    }
                }
            }
        }
    }
    if let Some(error) = first_invalid {
        return Err(error);
    }
    apply_dimensions_and_rasterize_with_budget(glyphs, definition, budget)
}

fn parse_hex_stream<R: BufRead>(
    reader: &mut R,
    glyphs: &mut BTreeMap<u32, RawUnihexGlyph>,
) -> Result<u64> {
    let mut line_number = 0_u64;
    let mut bytes_read = 0_u64;
    let mut codepoint_bytes = Vec::with_capacity(16);
    let mut bitmap_bytes = Vec::with_capacity(MAX_UNIHEX_LINE_BYTES);
    loop {
        let (found_colon, consumed) = read_until_bounded(reader, b':', &mut codepoint_bytes)?;
        bytes_read = bytes_read
            .checked_add(consumed)
            .context("unihex byte count overflow")?;
        if bytes_read > MAX_UNIHEX_ENTRY_BYTES {
            bail!("unihex data exceeds the {MAX_UNIHEX_ENTRY_BYTES} byte entry limit");
        }
        if codepoint_bytes.is_empty() && !found_colon {
            return Ok(bytes_read);
        }
        if !found_colon || !matches!(codepoint_bytes.len(), 4..=6) {
            return Err(invalid(format!(
                "invalid entry at line {line_number}: expected 4, 5 or 6 hex digits followed by a colon"
            )));
        }
        let codepoint = decode_hex_number(&codepoint_bytes, line_number)?;
        if codepoint > 0x10ffff {
            return Err(invalid(format!(
                "codepoint U+{codepoint:06X} exceeds CodepointMap range at line {line_number}"
            )));
        }
        let (_, consumed) = read_until_bounded(reader, b'\n', &mut bitmap_bytes)?;
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
            _ => {
                return Err(invalid(format!(
                    "invalid entry at line {line_number}: expected an (8,16,24,32) x 16 bitmap"
                )))
            }
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

fn read_until_bounded<R: BufRead>(
    reader: &mut R,
    delimiter: u8,
    bytes: &mut Vec<u8>,
) -> Result<(bool, u64)> {
    bytes.clear();
    let mut consumed = 0_u64;
    loop {
        let available = reader.fill_buf().context("read unihex data")?;
        if available.is_empty() {
            return Ok((false, consumed));
        }
        if let Some(pos) = available.iter().position(|&b| b == delimiter) {
            if bytes.len() + pos > MAX_UNIHEX_LINE_BYTES {
                bail!("unihex line exceeds the {MAX_UNIHEX_LINE_BYTES} byte limit");
            }
            bytes.extend_from_slice(&available[..pos]);
            let take = pos + 1;
            reader.consume(take);
            consumed += take as u64;
            return Ok((true, consumed));
        }
        let chunk_len = available.len();
        if bytes.len() + chunk_len > MAX_UNIHEX_LINE_BYTES {
            bail!("unihex line exceeds the {MAX_UNIHEX_LINE_BYTES} byte limit");
        }
        bytes.extend_from_slice(available);
        reader.consume(chunk_len);
        consumed += chunk_len as u64;
    }
}

fn decode_hex_number(bytes: &[u8], line: u64) -> Result<u32> {
    bytes.iter().try_fold(0_u32, |value, byte| {
        let digit = match byte {
            b'0'..=b'9' => u32::from(byte - b'0'),
            b'A'..=b'F' => u32::from(byte - b'A' + 10),
            _ => {
                return Err(invalid(format!(
                    "invalid entry at line {line}: expected uppercase hex digit, got {:?}",
                    char::from(*byte)
                )))
            }
        };
        value
            .checked_mul(16)
            .and_then(|value| value.checked_add(digit))
            .context("unihex number overflow")
    })
}

#[cfg(test)]
fn apply_dimensions_and_rasterize(
    bits: BTreeMap<u32, RawUnihexGlyph>,
    definition: &FontUnihexProviderDefinition,
) -> Result<UnihexGlyphSource> {
    apply_dimensions_and_rasterize_with_budget(bits, definition, None)
}

fn apply_dimensions_and_rasterize_with_budget(
    mut bits: BTreeMap<u32, RawUnihexGlyph>,
    definition: &FontUnihexProviderDefinition,
    _budget: Option<&PackResourceStack>,
) -> Result<UnihexGlyphSource> {
    let mut sized = Vec::<(u32, RawUnihexGlyph, i32, i32)>::with_capacity(bits.len());
    for range in &definition.size_overrides {
        // Vanilla loops the inclusive codepoint range and removes matching
        // entries. Iterating only the present BTreeMap keys is result- and
        // order-equivalent while bounding work for adversarial wide ranges.
        let matched = bits
            .range(range.from..=range.to)
            .map(|(&codepoint, _)| codepoint)
            .collect::<Vec<_>>();
        for codepoint in matched {
            let glyph = bits
                .remove(&codepoint)
                .expect("matched unihex glyph remains until this ordered override");
            sized.push((codepoint, glyph, range.left, range.right));
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
        sized.push((codepoint, glyph, left, right));
    }
    sized.sort_unstable_by_key(|&(codepoint, _, _, _)| codepoint);

    let mut entries = Vec::with_capacity(sized.len());
    let mut bit_rows = Vec::with_capacity(sized.len().saturating_mul(24));
    for &(codepoint, glyph, left, right) in &sized {
        let pixel_width = u32::try_from(right - left + 1)
            .context("unihex glyph has non-positive retained width")?;
        if pixel_width > UNIHEX_PAGE_WIDTH {
            bail!("unihex glyph U+{codepoint:04X} is wider than the atlas page");
        }
        let bytes_per_row = pixel_width.div_ceil(8).max(1);
        let bytes_per_row_u8 =
            u8::try_from(bytes_per_row).context("unihex glyph row byte width overflow")?;
        let data_offset =
            u32::try_from(bit_rows.len()).context("unihex packed bitmap buffer overflow")?;
        let row_bytes = bytes_per_row as usize;
        let base = bit_rows.len();
        bit_rows.resize(base + UNIHEX_GLYPH_HEIGHT as usize * row_bytes, 0);
        for row in 0..UNIHEX_GLYPH_HEIGHT {
            let row_bits = glyph.rows[row as usize];
            if row_bits == 0 {
                continue;
            }
            let dst_row = base + row as usize * row_bytes;
            for column in left..=right {
                if (0..32).contains(&column) && (row_bits & (1_u32 << (31 - column))) != 0 {
                    let dst_x = (column - left) as usize;
                    bit_rows[dst_row + (dst_x >> 3)] |= 0x80 >> (dst_x & 7);
                }
            }
        }
        entries.push(UnihexGlyphEntry {
            codepoint,
            pixel_width,
            data_offset,
            bytes_per_row: bytes_per_row_u8,
        });
    }
    entries.shrink_to_fit();
    bit_rows.shrink_to_fit();
    Ok(UnihexGlyphSource {
        glyphs: entries,
        bit_rows,
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
            .find(|glyph| glyph.codepoint == u32::from('A'))
            .unwrap();
        assert_eq!(a.pixel_width, 1);
        let cjk_ext = page
            .glyphs
            .iter()
            .find(|glyph| glyph.codepoint == 0x10000)
            .unwrap();
        assert_eq!(cjk_ext.pixel_width, 1);
        assert_eq!(page.set_pixel_count(), 3 * 16);
    }

    #[test]
    fn automatic_blank_bounds_and_ordered_overrides_match_vanilla() {
        let payload = [repeated_rows("4E00", "0000"), repeated_rows("4E01", "0180")].concat();
        let page = parse_unihex_archive(
            archive(&[("glyphs.hex", payload.as_bytes())]),
            &definition(vec![
                super::super::providers::FontUnihexSizeOverride {
                    from: u32::from('一'),
                    to: u32::from('丁'),
                    left: 0,
                    right: 15,
                },
                super::super::providers::FontUnihexSizeOverride {
                    from: u32::from('一'),
                    to: u32::from('七'),
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
    fn surrogate_catalog_keeps_duplicates_overrides_and_full_codepoint_range() {
        let first = [
            repeated_rows("D7FF", "80"),
            repeated_rows("D800", "80"),
            repeated_rows("D800", "40"),
            repeated_rows("DFFF", "80"),
            repeated_rows("E000", "80"),
            repeated_rows("10FFFF", "80"),
        ]
        .concat();
        let last = repeated_rows("D800", "20");
        let page = parse_unihex_archive(
            archive(&[("one.hex", first.as_bytes()), ("two.hex", last.as_bytes())]),
            &definition(vec![
                super::super::providers::FontUnihexSizeOverride {
                    from: u32::from('\u{d7ff}'),
                    to: u32::from('\u{e000}'),
                    left: 0,
                    right: 7,
                },
                super::super::providers::FontUnihexSizeOverride {
                    from: u32::from('\u{d7ff}'),
                    to: u32::from('\u{e000}'),
                    left: 0,
                    right: 15,
                },
            ]),
        )
        .unwrap();
        assert_eq!(
            page.glyphs.iter().map(|g| g.codepoint).collect::<Vec<_>>(),
            [0xd7ff, 0xd800, 0xdfff, 0xe000, 0x10ffff]
        );
        for glyph in &page.glyphs {
            assert_eq!(
                glyph.pixel_width,
                if glyph.codepoint == 0x10ffff { 1 } else { 8 }
            );
        }
        assert!(!page.glyph_pixel_is_set(1, 0, 0));
        assert!(!page.glyph_pixel_is_set(1, 1, 0));
        assert!(page.glyph_pixel_is_set(1, 2, 0));
    }

    #[test]
    fn rejects_lowercase_malformed_width_out_of_range_codepoint_and_long_lines() {
        for (payload, expected) in [
            ("0041".to_string(), "followed by a colon"),
            ("041:00\n".to_string(), "4, 5 or 6"),
            (repeated_rows("110000", "00"), "exceeds CodepointMap range"),
            (
                "00af:00000000000000000000000000000000\n".to_string(),
                "uppercase",
            ),
            ("0041:00\n".to_string(), "expected an (8,16,24,32)"),
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
            assert_eq!(
                error.is::<UnihexProviderInvalid>(),
                expected != "line exceeds"
            );
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

#[cfg(test)]
mod failures;
