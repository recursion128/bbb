//! Deterministic TrueType faces for provider and shared atlas acceptance.

const UNITS_PER_EM: u16 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CmapKind {
    #[default]
    WindowsUnicodeBmp,
    MacintoshRoman,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FixtureSpec {
    /// Emit the `glyf` table; without it the face is not "TrueType" format.
    pub glyf: bool,
    pub cmap: CmapKind,
}

struct FixtureGlyph {
    advance: u16,
    lsb: i16,
    contours: Vec<Vec<[i16; 2]>>,
    instructions: Vec<u8>,
}

/// The standard fixture: `size 10 / oversample 1` at `unitsPerEm 1000` puts
/// `A` on advance 6, bearing left 1, bearing top 7 and an 8x7 bitmap.
pub(crate) fn fixture_ttf() -> Vec<u8> {
    fixture_ttf_with(FixtureSpec {
        glyf: true,
        cmap: CmapKind::WindowsUnicodeBmp,
    })
}

pub(crate) fn fixture_ttf_with(spec: FixtureSpec) -> Vec<u8> {
    assemble_fixture(spec)
}

fn assemble_fixture(spec: FixtureSpec) -> Vec<u8> {
    let glyphs = fixture_glyphs();
    // Glyph ids follow the mapped codepoints so no `glyphIdArray` is needed.
    let mapping: &[(u16, u16)] = match spec.cmap {
        CmapKind::WindowsUnicodeBmp => &[(0x20, 1), (0x41, 2), (0x42, 3), (0x43, 4)],
        CmapKind::MacintoshRoman => &[(0x41, 2)],
    };
    let (glyf, offsets) = glyf_table(&glyphs);
    let mut tables: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"cmap", cmap_table(spec.cmap, mapping)),
        (b"head", head_table()),
        (b"hhea", hhea_table(&glyphs)),
        (b"hmtx", hmtx_table(&glyphs)),
        (b"maxp", maxp_table(glyphs.len())),
    ];
    if spec.glyf {
        tables.push((b"glyf", glyf));
        tables.push((b"loca", loca_table(&offsets)));
    }
    tables.sort_by_key(|(tag, _)| **tag);
    assemble(&tables)
}

fn fixture_glyphs() -> Vec<FixtureGlyph> {
    let point = |x: i16, y: i16| [x, y];
    vec![
        // .notdef: no contours.
        FixtureGlyph {
            instructions: Vec::new(),
            advance: 500,
            lsb: 50,
            contours: Vec::new(),
        },
        // U+0020: no outline, advance only.
        FixtureGlyph {
            instructions: Vec::new(),
            advance: 300,
            lsb: 0,
            contours: Vec::new(),
        },
        // A: triangle reaching 700 units above a baseline at y = 0.
        FixtureGlyph {
            instructions: Vec::new(),
            advance: 600,
            lsb: 100,
            contours: vec![vec![point(100, 0), point(500, 700), point(900, 0)]],
        },
        // B: box from x 50..550, y 0..600.
        FixtureGlyph {
            instructions: Vec::new(),
            advance: 600,
            lsb: 50,
            contours: vec![vec![
                point(50, 0),
                point(550, 0),
                point(550, 600),
                point(50, 600),
            ]],
        },
        // C: box with a negative left side bearing at x = -20.
        FixtureGlyph {
            instructions: Vec::new(),
            advance: 600,
            lsb: -20,
            contours: vec![vec![
                point(-20, 0),
                point(500, 0),
                point(500, 700),
                point(-20, 700),
            ]],
        },
    ]
}

/// Returns the `glyf` body and the byte offset of every glyph entry, with one
/// trailing offset, as `loca` stores them.
fn glyf_table(glyphs: &[FixtureGlyph]) -> (Vec<u8>, Vec<u32>) {
    let mut glyf = Vec::new();
    let mut offsets = Vec::with_capacity(glyphs.len() + 1);
    for glyph in glyphs {
        offsets.push(glyf.len() as u32);
        if glyph.contours.is_empty() {
            // An empty glyph is a zero-length `glyf` entry.
            continue;
        }
        let points = glyph.contours.iter().flatten().copied().collect::<Vec<_>>();
        i16be(&mut glyf, glyph.contours.len() as i16);
        i16be(
            &mut glyf,
            points.iter().map(|point| point[0]).min().unwrap(),
        );
        i16be(
            &mut glyf,
            points.iter().map(|point| point[1]).min().unwrap(),
        );
        i16be(
            &mut glyf,
            points.iter().map(|point| point[0]).max().unwrap(),
        );
        i16be(
            &mut glyf,
            points.iter().map(|point| point[1]).max().unwrap(),
        );
        let mut end = -1_i16;
        for contour in &glyph.contours {
            end += contour.len() as i16;
            u16be(&mut glyf, end as u16);
        }
        u16be(&mut glyf, glyph.instructions.len() as u16);
        glyf.extend_from_slice(&glyph.instructions);
        // Every point is on-curve and carries 16-bit deltas.
        glyf.extend(std::iter::repeat_n(0x01, points.len()));
        // `glyf` stores every x delta, then every y delta.
        for axis in 0..2 {
            let mut previous = 0_i16;
            for point in &points {
                i16be(&mut glyf, point[axis] - previous);
                previous = point[axis];
            }
        }
        if glyf.len() % 2 != 0 {
            glyf.push(0);
        }
    }
    offsets.push(glyf.len() as u32);
    (glyf, offsets)
}

/// Short `loca`: one `uint16` offset per glyph entry, in units of two bytes.
fn loca_table(offsets: &[u32]) -> Vec<u8> {
    let mut loca = Vec::with_capacity(offsets.len() * 2);
    for offset in offsets {
        assert_eq!(offset % 2, 0, "short loca entries are even");
        u16be(&mut loca, (offset / 2) as u16);
    }
    loca
}

fn assemble(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let count = tables.len() as u16;
    let entry_selector = (u16::BITS - 1 - count.leading_zeros()) as u16;
    let search_range = 16 * (1 << entry_selector);
    let mut offsets = Vec::with_capacity(tables.len());
    let mut offset = 12 + 16 * tables.len();
    for (_, body) in tables {
        offsets.push(offset);
        offset += body.len().next_multiple_of(4);
    }
    let mut out = Vec::new();
    u32be(&mut out, 0x0001_0000);
    u16be(&mut out, count);
    u16be(&mut out, search_range);
    u16be(&mut out, entry_selector);
    u16be(&mut out, count * 16 - search_range);
    for ((tag, body), offset) in tables.iter().zip(&offsets) {
        u32be(&mut out, u32::from_be_bytes(**tag));
        u32be(&mut out, table_checksum(body));
        u32be(&mut out, *offset as u32);
        u32be(&mut out, body.len() as u32);
    }
    for (_, body) in tables {
        out.extend_from_slice(body);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    // `head.checkSumAdjustment`: 0xB1B0AFBA minus the whole-file checksum.
    let head = tables
        .iter()
        .position(|(tag, _)| **tag == *b"head")
        .expect("fixture font has a head table");
    let adjustment = 0xB1B0_AFBA_u32.wrapping_sub(table_checksum(&out));
    let head_offset = offsets[head] + 8;
    out[head_offset..head_offset + 4].copy_from_slice(&adjustment.to_be_bytes());
    out
}

pub(crate) fn table_checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0_u32, |sum, chunk| {
        let mut word = [0_u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn cmap_table(kind: CmapKind, mapping: &[(u16, u16)]) -> Vec<u8> {
    let (platform, encoding) = match kind {
        CmapKind::WindowsUnicodeBmp => (3_u16, 1_u16),
        CmapKind::MacintoshRoman => (1, 0),
    };
    let mut subtable = Vec::new();
    match kind {
        CmapKind::WindowsUnicodeBmp => format4_subtable(&mut subtable, mapping),
        CmapKind::MacintoshRoman => {
            u16be(&mut subtable, 0);
            u16be(&mut subtable, 262);
            u16be(&mut subtable, 0);
            let mut glyphs = [0_u8; 256];
            for &(codepoint, glyph) in mapping {
                glyphs[usize::from(codepoint)] = glyph as u8;
            }
            subtable.extend_from_slice(&glyphs);
        }
    }
    let mut out = Vec::new();
    u16be(&mut out, 0);
    u16be(&mut out, 1);
    u16be(&mut out, platform);
    u16be(&mut out, encoding);
    u32be(&mut out, 12);
    out.extend_from_slice(&subtable);
    out
}

fn format4_subtable(out: &mut Vec<u8>, mapping: &[(u16, u16)]) {
    // One segment per run of consecutive codepoints with consecutive glyph
    // ids, plus the mandatory 0xFFFF terminator.
    let mut segments: Vec<(u16, u16, u16)> = Vec::new();
    for &(codepoint, glyph) in mapping {
        match segments.last_mut() {
            Some((start, end, first_glyph))
                if *end + 1 == codepoint && *first_glyph + (codepoint - *start) == glyph =>
            {
                *end = codepoint;
            }
            _ => segments.push((codepoint, codepoint, glyph)),
        }
    }
    segments.sort_by_key(|(start, _, _)| *start);
    segments.push((0xFFFF, 0xFFFF, 0));
    let seg_count = segments.len() as u16;
    u16be(out, 4);
    u16be(out, 16 + 8 * seg_count);
    u16be(out, 0);
    u16be(out, seg_count * 2);
    let entry_selector = (u16::BITS - 1 - seg_count.leading_zeros()) as u16;
    let search_range = 2 * (1 << entry_selector);
    u16be(out, search_range);
    u16be(out, entry_selector);
    u16be(out, seg_count * 2 - search_range);
    for (_, end, _) in &segments {
        u16be(out, *end);
    }
    u16be(out, 0);
    for (start, _, _) in &segments {
        u16be(out, *start);
    }
    for (start, _, first_glyph) in &segments {
        // idDelta maps the segment's range onto consecutive glyph ids; the
        // 0xFFFF terminator's delta 1 wraps to glyph 0.
        u16be(out, first_glyph.wrapping_sub(*start));
    }
    for _ in &segments {
        u16be(out, 0);
    }
}

fn head_table() -> Vec<u8> {
    let mut out = Vec::new();
    u32be(&mut out, 0x0001_0000);
    u32be(&mut out, 0x0001_0000);
    u32be(&mut out, 0);
    u32be(&mut out, 0x5F0F_3CF5);
    u16be(&mut out, 0x0003);
    u16be(&mut out, UNITS_PER_EM);
    out.extend_from_slice(&0_i64.to_be_bytes());
    out.extend_from_slice(&0_i64.to_be_bytes());
    i16be(&mut out, -20);
    i16be(&mut out, 0);
    i16be(&mut out, 900);
    i16be(&mut out, 700);
    u16be(&mut out, 0);
    u16be(&mut out, 8);
    i16be(&mut out, 2);
    i16be(&mut out, 0);
    i16be(&mut out, 0);
    out
}

fn hhea_table(glyphs: &[FixtureGlyph]) -> Vec<u8> {
    let mut out = Vec::new();
    u32be(&mut out, 0x0001_0000);
    i16be(&mut out, 800);
    i16be(&mut out, -200);
    i16be(&mut out, 0);
    u16be(
        &mut out,
        glyphs.iter().map(|glyph| glyph.advance).max().unwrap(),
    );
    i16be(
        &mut out,
        glyphs.iter().map(|glyph| glyph.lsb).min().unwrap(),
    );
    i16be(&mut out, 0);
    i16be(&mut out, 900);
    i16be(&mut out, 1);
    i16be(&mut out, 0);
    i16be(&mut out, 0);
    for _ in 0..4 {
        i16be(&mut out, 0);
    }
    i16be(&mut out, 0);
    u16be(&mut out, glyphs.len() as u16);
    out
}

fn hmtx_table(glyphs: &[FixtureGlyph]) -> Vec<u8> {
    let mut out = Vec::new();
    for glyph in glyphs {
        u16be(&mut out, glyph.advance);
        i16be(&mut out, glyph.lsb);
    }
    out
}

fn maxp_table(glyph_count: usize) -> Vec<u8> {
    let mut out = Vec::new();
    u32be(&mut out, 0x0001_0000);
    u16be(&mut out, glyph_count as u16);
    for value in [8_u16, 1, 0, 0, 2, 0, 0, 0, 0, 16, 16, 0, 0] {
        u16be(&mut out, value);
    }
    out
}

fn u16be(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn i16be(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn u32be(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}
