//! The one glyph walk behind every text surface: HUD lines, signs, text displays, name tags,
//! map labels and debug billboards.
//!
//! A [`TextPen`] places glyphs (`Font.StringRenderOutput.accept`): it substitutes obfuscated
//! glyphs, resolves the shadow colour and advances the pen. Each [`PlacedGlyph`] then yields its
//! glyph passes and effect bars in font pixels. World surfaces draw a line's glyphs before its
//! effect bars (`PreparedText.visit`) through [`mesh_world_text_line`].

use std::{collections::HashMap, hash::Hash, rc::Rc};

use bbb_render_types::{
    visual_order_indexed, HudAsciiGlyph, HudEffectRect, HudFontGlyphBook, HudFontGlyphMap,
    HudGlyphQuad, HudObfuscatedGlyphPool, HudObfuscatedRandom, HudStyledTextRun, HudSubsequence,
    HudSubsequenceKey, HudTextStyle,
};
use glam::{Mat4, Vec3};

use crate::item_models::{ItemModelMesh, ItemModelVertex};

/// UVs that make the world text shaders skip the atlas and draw solid colour.
pub(crate) const UNTEXTURED_UVS: [[f32; 2]; 4] = [[-1.0, -1.0]; 4];

/// One flattened world-text run. A retained Component font selects its own glyph map.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct WorldTextRun {
    pub text: String,
    pub style: HudTextStyle,
    pub color: Option<u32>,
    pub shadow_color: Option<u32>,
    pub font: Option<String>,
    /// This run's click/hover/insertion identity. Vanilla `Style.equals`
    /// compares those events, so `SubStringSource.substring`
    /// cuts a part when only they differ even
    /// though nothing about the drawn glyphs changes.
    pub event_id: Option<usize>,
}

impl WorldTextRun {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    fn same_style(&self, other: &Self) -> bool {
        self.style == other.style
            && self.color == other.color
            && self.shadow_color == other.shadow_color
            && self.font == other.font
            && self.event_id == other.event_id
    }

    fn glyph(&self, ch: char, fonts: &HudFontGlyphBook) -> HudAsciiGlyph {
        fonts.glyphs(self.font.as_deref()).resolve(ch)
    }
}

/// Appends `text` to `runs`, joining the last run when it has the same style.
pub fn push_world_text(runs: &mut Vec<WorldTextRun>, source: &WorldTextRun, text: &str) {
    if text.is_empty() {
        return;
    }
    match runs.last_mut() {
        Some(last) if last.same_style(source) => last.text.push_str(text),
        _ => runs.push(WorldTextRun {
            text: text.to_string(),
            style: source.style,
            color: source.color,
            shadow_color: source.shadow_color,
            font: source.font.clone(),
            event_id: source.event_id,
        }),
    }
}

/// One visual glyph of a completed line: the character, its run and its local UTF-16 index
/// inside its own `FormattedCharSequence` part (widens effect bars at zero).
pub type VisualGlyph<'a> = (char, &'a WorldTextRun, usize);

/// Applies vanilla `FormattedBidiReorder` to one completed Component line. Parts are cut on
/// `Style`, never on the text a part carries.
pub fn world_text_visual_order<'a>(
    runs: &'a [WorldTextRun],
    fonts: &HudFontGlyphBook,
) -> Vec<VisualGlyph<'a>> {
    let logical = runs
        .iter()
        .flat_map(|run| {
            let key = HudSubsequenceKey {
                style: run.style,
                color: run.color,
                shadow_color: run.shadow_color,
                font: run.font.as_deref(),
                target: run.event_id,
            };
            run.text
                .chars()
                .map(move |ch| (ch, HudSubsequence { payload: run, key }))
        })
        .collect::<Vec<_>>();
    visual_order_indexed(&logical, fonts.default_right_to_left())
        .into_iter()
        .map(|(ch, part, index)| (ch, part.payload, index))
        .collect()
}

/// `Font.width`: the advance sum of the unsubstituted glyphs, rounded up once
pub fn text_width(advances: impl IntoIterator<Item = f32>) -> i32 {
    advances.into_iter().sum::<f32>().ceil() as i32
}

/// [`text_width`] of world-text characters.
pub fn world_text_width<'a>(
    chars: impl IntoIterator<Item = (char, &'a WorldTextRun)>,
    fonts: &HudFontGlyphBook,
) -> i32 {
    text_width(
        chars
            .into_iter()
            .map(|(ch, run)| run.glyph(ch, fonts).styled_advance_f32(run.style)),
    )
}

/// The logical characters of `runs` with their runs.
pub fn world_text_chars(runs: &[WorldTextRun]) -> impl Iterator<Item = (char, &WorldTextRun)> {
    runs.iter()
        .flat_map(|run| run.text.chars().map(move |ch| (ch, run)))
}

/// Style-preserving `StringSplitter.splitLines` over flattened component runs. The width is
/// clamped to one pixel, a space or newline at a break is consumed, the first non-zero-width
/// glyph may exceed the limit, and a terminal newline produces a final empty line.
pub fn split_world_text_lines(
    runs: &[WorldTextRun],
    max_width: i32,
    fonts: &HudFontGlyphBook,
) -> Vec<Vec<WorldTextRun>> {
    let chars: Vec<(char, &WorldTextRun)> = world_text_chars(runs).collect();
    let max_width = max_width.max(1) as f32;
    let mut lines = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let mut width = 0.0_f32;
        let mut had_non_zero_width = false;
        let mut last_space = None;
        let mut split = None;
        for (index, &(ch, run)) in chars.iter().enumerate().skip(start) {
            if ch == '\n' {
                split = Some((index, 1usize, true));
                break;
            }
            if ch == ' ' {
                last_space = Some(index);
            }
            let advance = run.glyph(ch, fonts).styled_advance_f32(run.style);
            width += advance;
            if had_non_zero_width && width > max_width {
                let split_at = last_space.unwrap_or(index);
                split = Some((split_at, usize::from(chars[split_at].0 == ' '), false));
                break;
            }
            had_non_zero_width |= advance != 0.0;
        }
        let (end, skip, newline) = split.unwrap_or((chars.len(), 0, false));
        let mut line = Vec::new();
        for &(ch, run) in &chars[start..end] {
            push_world_text(&mut line, run, ch.encode_utf8(&mut [0; 4]));
        }
        lines.push(line);
        if end == chars.len() {
            break;
        }
        start = end + skip;
        if newline && start == chars.len() {
            lines.push(Vec::new());
        }
    }
    lines
}

/// One glyph placed on the pen: after obfuscation, with its resolved main and shadow colours
/// (`0` shadow means none).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedGlyph {
    pub x: f32,
    pub y: f32,
    pub glyph: HudAsciiGlyph,
    pub style: HudTextStyle,
    pub argb: u32,
    pub shadow_argb: u32,
    pub subsequence_start: bool,
}

impl PlacedGlyph {
    pub fn advance(&self) -> f32 {
        self.glyph.styled_advance_f32(self.style)
    }

    fn has_shadow(&self) -> bool {
        self.shadow_argb != 0
    }

    /// `BakedSheetGlyph.renderChar` passes with their colour and depth: the main pass sits
    /// 0.03 font pixel above its shadow and bold's second quad 0.001 above its first.
    pub fn glyph_quads(&self) -> impl Iterator<Item = (HudGlyphQuad, f32, u32)> + '_ {
        let has_shadow = self.has_shadow();
        self.glyph
            .styled_quads(self.x, self.y, self.style, has_shadow)
            .into_iter()
            .enumerate()
            .map(move |(index, quad)| {
                let z = if has_shadow && !quad.shadow {
                    0.03
                } else {
                    0.0
                } + if self.style.bold && index % 2 == 1 {
                    0.001
                } else {
                    0.0
                };
                let argb = if quad.shadow {
                    self.shadow_argb
                } else {
                    self.argb
                };
                (quad, z, argb)
            })
    }

    /// Underline/strikethrough bars, each as its shadow copy (z 0.01) then the main bar
    /// (z 0.04 over a shadow, else 0.01).
    pub fn effect_rects(&self) -> impl Iterator<Item = (HudEffectRect, f32, u32)> + '_ {
        let has_shadow = self.has_shadow();
        let offset = self.glyph.shadow_offset;
        self.glyph
            .styled_effect_rects(self.x, self.y, self.style, self.subsequence_start)
            .into_iter()
            .flat_map(move |rect| {
                let shadow = has_shadow.then_some({
                    (
                        HudEffectRect {
                            x0: rect.x0 + offset,
                            y0: rect.y0 + offset,
                            x1: rect.x1 + offset,
                            y1: rect.y1 + offset,
                            ..rect
                        },
                        0.01,
                        self.shadow_argb,
                    )
                });
                let main = (rect, if has_shadow { 0.04 } else { 0.01 }, self.argb);
                shadow.into_iter().chain(std::iter::once(main))
            })
    }
}

/// The pen walk shared by every text surface. Obfuscated glyphs draw from one random stream:
/// the same seed draws the same glyphs.
#[derive(Debug, Clone)]
pub struct TextPen {
    pub x: f32,
    pub y: f32,
    /// `None` keeps every glyph unsubstituted.
    random: Option<HudObfuscatedRandom>,
}

impl TextPen {
    pub fn new(x: f32, y: f32, obfuscated_seed: u64) -> Self {
        Self {
            x,
            y,
            random: Some(HudObfuscatedRandom::with_seed(obfuscated_seed)),
        }
    }

    /// A pen that draws obfuscated text with its original glyphs.
    pub fn unobfuscated(x: f32, y: f32) -> Self {
        Self { x, y, random: None }
    }

    /// Places `ch` and advances the pen by the placed glyph's advance, which an obfuscated
    /// substitute may change within its `ceil(advance)` bucket.
    #[allow(clippy::too_many_arguments)]
    pub fn place(
        &mut self,
        ch: char,
        glyphs: &HudFontGlyphMap,
        obfuscated: &HudObfuscatedGlyphPool,
        style: HudTextStyle,
        argb: u32,
        shadow_color: Option<u32>,
        draw_shadow: bool,
        subsequence_start: bool,
    ) -> PlacedGlyph {
        let base = glyphs.resolve(ch);
        let glyph = match self.random.as_mut() {
            Some(random) if style.obfuscated && ch != ' ' => obfuscated
                .random_glyph(base.advance, random)
                .unwrap_or(base),
            _ => base,
        };
        let placed = PlacedGlyph {
            x: self.x,
            y: self.y,
            glyph,
            style,
            argb,
            shadow_argb: HudStyledTextRun::resolve_shadow_argb(argb, shadow_color, draw_shadow),
            subsequence_start,
        };
        self.x += placed.advance();
        placed
    }

    /// [`Self::place`] for one visual world-text glyph.
    pub fn place_world(
        &mut self,
        (ch, run, local_index): VisualGlyph,
        fonts: &HudFontGlyphBook,
        argb: u32,
        draw_shadow: bool,
    ) -> PlacedGlyph {
        let font = run.font.as_deref();
        self.place(
            ch,
            fonts.glyphs(font),
            fonts.obfuscated_pool(font),
            run.style,
            argb,
            run.shadow_color,
            draw_shadow,
            local_index == 0,
        )
    }
}

/// `0xAARRGGBB` to the `[r, g, b, a]` vertex colour.
pub fn argb_color(argb: u32) -> [f32; 4] {
    [16, 8, 0, 24].map(|shift| ((argb >> shift) & 0xFF) as f32 / 255.0)
}

/// The draw colour of a world run: its own colour or the surface's, with the surface alpha.
pub fn world_text_argb(rgb: Option<u32>, alpha: u8) -> u32 {
    u32::from(alpha) << 24 | rgb.unwrap_or(0xFF_FF_FF) & 0xFF_FF_FF
}

/// Font-pixel text geometry: glyph quads sample the font atlas, effect bars and backgrounds are
/// solid. Surfaces keep it in font space and transform it when they draw.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextMesh(pub ItemModelMesh);

impl TextMesh {
    /// Rescales normalized atlas glyph UVs after a dynamic font atlas grows while preserving
    /// untextured effect/background sentinel UVs (`UNTEXTURED_UVS`).
    pub(crate) fn rescaled_for_atlas(&self, scale_x: f32, scale_y: f32) -> Self {
        let mut mesh = self.0.clone();
        for vertex in &mut mesh.vertices {
            if vertex.uv[0] >= 0.0 && vertex.uv[1] >= 0.0 {
                vertex.uv[0] *= scale_x;
                vertex.uv[1] *= scale_y;
            }
        }
        Self(mesh)
    }

    pub fn push_glyph(&mut self, quad: &HudGlyphQuad, z: f32, argb: u32, light: [f32; 2]) {
        let uv = quad.uv;
        self.push(
            quad.corners,
            z,
            [
                [uv.min[0], uv.min[1]],
                [uv.min[0], uv.max[1]],
                [uv.max[0], uv.max[1]],
                [uv.max[0], uv.min[1]],
            ],
            argb,
            light,
        );
    }

    pub fn push_rect(
        &mut self,
        [x0, y0, x1, y1]: [f32; 4],
        z: f32,
        uvs: [[f32; 2]; 4],
        argb: u32,
        light: [f32; 2],
    ) {
        self.push(
            [[x0, y0], [x0, y1], [x1, y1], [x1, y0]],
            z,
            uvs,
            argb,
            light,
        );
    }

    fn push(
        &mut self,
        corners: [[f32; 2]; 4],
        z: f32,
        uvs: [[f32; 2]; 4],
        argb: u32,
        light: [f32; 2],
    ) {
        self.0.append_raw_textured_quad(
            corners.map(|[x, y]| [x, y, z]),
            uvs,
            argb_color(argb),
            light,
        );
    }

    /// The same mesh under `transform`.
    pub fn transformed(&self, transform: Mat4) -> ItemModelMesh {
        ItemModelMesh {
            vertices: self
                .0
                .vertices
                .iter()
                .map(|vertex| ItemModelVertex {
                    position: transform
                        .transform_point3(Vec3::from_array(vertex.position))
                        .to_array(),
                    ..*vertex
                })
                .collect(),
            indices: self.0.indices.clone(),
        }
    }
}

/// Meshes one visual line: every glyph pass, then the deferred effect bars drawn with
/// `effect_uvs`.
#[allow(clippy::too_many_arguments)]
pub fn mesh_world_text_line(
    mesh: &mut TextMesh,
    effects: Option<&mut TextMesh>,
    visual: &[VisualGlyph],
    pen: &mut TextPen,
    fonts: &HudFontGlyphBook,
    argb: impl Fn(&WorldTextRun) -> u32,
    draw_shadow: bool,
    light: [f32; 2],
    effect_uvs: [[f32; 2]; 4],
) {
    let placed: Vec<PlacedGlyph> = visual
        .iter()
        .map(|&glyph| pen.place_world(glyph, fonts, argb(glyph.1), draw_shadow))
        .collect();
    for glyph in &placed {
        for (quad, z, argb) in glyph.glyph_quads() {
            mesh.push_glyph(&quad, z, argb, light);
        }
    }
    let effects = match effects {
        Some(effects) => effects,
        None => mesh,
    };
    for glyph in &placed {
        for (rect, z, argb) in glyph.effect_rects() {
            effects.push_rect(
                [rect.x0, rect.y0, rect.x1, rect.y1],
                z,
                effect_uvs,
                argb,
                light,
            );
        }
    }
}

/// Font-space text layouts of one surface kind, keyed by everything that shapes them. Only
/// layouts drawn in the current or the previous frame are kept, so memory follows what is on
/// screen.
pub(crate) struct TextLayoutCache<K, V> {
    current: HashMap<K, Rc<V>>,
    previous: HashMap<K, Rc<V>>,
}

impl<K, V> Default for TextLayoutCache<K, V> {
    fn default() -> Self {
        Self {
            current: HashMap::new(),
            previous: HashMap::new(),
        }
    }
}

impl<K: Hash + Eq, V> TextLayoutCache<K, V> {
    pub(crate) fn get_or_build(&mut self, key: K, build: impl FnOnce() -> V) -> Rc<V> {
        if let Some(layout) = self.current.get(&key) {
            return layout.clone();
        }
        let layout = self
            .previous
            .remove(&key)
            .unwrap_or_else(|| Rc::new(build()));
        self.current.insert(key, layout.clone());
        layout
    }

    /// Drops every layout the frame that just ended did not draw.
    pub(crate) fn next_frame(&mut self) {
        self.previous = std::mem::take(&mut self.current);
    }
}

/// Starts a new frame for the text display and name tag layout caches. Call once per frame
/// before baking them.
pub fn next_text_layout_frame() {
    crate::item_render::item_models::display_text::next_display_text_layout_frame();
    crate::entity_models::next_name_tag_layout_frame();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_cache_keeps_layouts_drawn_last_frame_and_evicts_the_rest() {
        let mut cache = TextLayoutCache::<u32, u32>::default();
        let mut builds = 0;
        let mut get = |cache: &mut TextLayoutCache<u32, u32>, key| {
            *cache.get_or_build(key, || {
                builds += 1;
                key
            })
        };
        get(&mut cache, 1);
        get(&mut cache, 2);
        cache.next_frame();
        get(&mut cache, 1);
        cache.next_frame();
        get(&mut cache, 1);
        get(&mut cache, 2);
        assert_eq!(builds, 3, "2 was unused for a frame and was rebuilt");
        assert_eq!(cache.current.len() + cache.previous.len(), 2);
    }
}
