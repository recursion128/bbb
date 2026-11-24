use anyhow::{Context, Result};
#[cfg(test)]
use bbb_client_assets::WinScreenTextResources;
use bbb_client_assets::{PackResourceStack, PackRoots, ResourceLocation};
use bbb_client_render::{HudFontGlyphMap, HudStyledTextRun, HudTextStyle};
use serde::Deserialize;

const POEM_WIDTH: u32 = 256;
const LINE_HEIGHT: u32 = 12;
const NAME_PREFIX: &str = "           ";
const OBFUSCATE_TOKEN: &str = "§f§k§a§b";
const MAX_CREDITS_ENTRIES: u64 = 65_536;
const DERIVED_LINE_LOGICAL_BYTES: u64 = 96;
const END_POEM_RESOURCE: &str = "minecraft:texts/end.txt";
const CREDITS_RESOURCE: &str = "minecraft:texts/credits.json";
const POSTCREDITS_RESOURCE: &str = "minecraft:texts/postcredits.txt";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WinScreenResources {
    pub(crate) lines: Vec<WinScreenLine>,
    pub(crate) narration_components: Vec<String>,
    pub(crate) total_scroll_length: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WinScreenLine {
    pub(crate) runs: Vec<HudStyledTextRun>,
    pub(crate) centered: bool,
}

impl WinScreenResources {
    pub(crate) fn load(
        roots: &PackRoots,
        username: &str,
        glyphs: &HudFontGlyphMap,
    ) -> Result<Self> {
        let resources = roots.resource_stack();
        let end_poem = read_optional_text(&resources, END_POEM_RESOURCE, "end poem");
        let credits = read_optional_credits(&resources);
        let postcredits = read_optional_text(&resources, POSTCREDITS_RESOURCE, "postcredits");
        Self::build(
            end_poem,
            credits,
            postcredits,
            username,
            glyphs,
            Some(&resources),
        )
    }

    #[cfg(test)]
    fn from_raw(
        raw: WinScreenTextResources,
        username: &str,
        glyphs: &HudFontGlyphMap,
        resources: Option<&PackResourceStack>,
    ) -> Result<Self> {
        let credits = raw
            .credits_json
            .and_then(|credits| serde_json::from_str::<Vec<CreditsSection>>(&credits).ok());
        Self::build(
            raw.end_poem,
            credits,
            raw.postcredits,
            username,
            glyphs,
            resources,
        )
    }

    fn build(
        end_poem: Option<String>,
        credits: Option<Vec<CreditsSection>>,
        postcredits: Option<String>,
        username: &str,
        glyphs: &HudFontGlyphMap,
        resources: Option<&PackResourceStack>,
    ) -> Result<Self> {
        let mut builder = WinScreenResourcesBuilder::new(resources);
        if let Some(poem) = end_poem {
            builder.add_poem(&poem, username, glyphs)?;
        }
        if let Some(sections) = credits {
            if let Some(resources) = resources {
                admit_credits_counts(resources, &sections)?;
            }
            builder.add_credits(&sections)?;
        }
        if let Some(postcredits) = postcredits {
            builder.add_poem(&postcredits, username, glyphs)?;
        }
        let mut result = builder.finish();
        result.total_scroll_length = u32::try_from(result.lines.len())
            .context("win-screen line count does not fit u32")?
            .checked_mul(LINE_HEIGHT)
            .context("win-screen total scroll length overflow")?;
        Ok(result)
    }
}

fn read_optional_text(resources: &PackResourceStack, id: &str, label: &str) -> Option<String> {
    let location = ResourceLocation::parse(id).expect("static win-screen resource id is valid");
    let resource = resources.get_resource(&location)?;
    resources.read_text_resource(&resource, label).ok()
}

fn read_optional_credits(resources: &PackResourceStack) -> Option<Vec<CreditsSection>> {
    let location =
        ResourceLocation::parse(CREDITS_RESOURCE).expect("static credits resource id is valid");
    let resource = resources.get_resource(&location)?;
    resources.read_json_resource(&resource, "credits").ok()
}

struct WinScreenResourcesBuilder<'a> {
    lines: Vec<WinScreenLine>,
    narration_components: Vec<String>,
    resources: Option<&'a PackResourceStack>,
}

impl<'a> WinScreenResourcesBuilder<'a> {
    fn new(resources: Option<&'a PackResourceStack>) -> Self {
        Self {
            lines: Vec::new(),
            narration_components: Vec::new(),
            resources,
        }
    }

    fn reserve_bytes(&self, bytes: u64) -> Result<()> {
        if let Some(resources) = self.resources {
            resources.reserve_derived_resource_bytes(bytes, "win-screen formatted lines")?;
        }
        Ok(())
    }

    fn push_line(&mut self, runs: Vec<HudStyledTextRun>, centered: bool) -> Result<()> {
        let text_bytes = runs.iter().try_fold(0u64, |acc, run| {
            acc.checked_add(run.text.len() as u64)
                .context("win-screen rendered text logical size overflow")
        })?;
        let line_bytes = DERIVED_LINE_LOGICAL_BYTES
            .checked_add(text_bytes)
            .context("win-screen line logical size overflow")?;
        self.reserve_bytes(line_bytes)?;
        self.lines.push(WinScreenLine { runs, centered });
        Ok(())
    }

    fn push_narration(&mut self, text: String) -> Result<()> {
        if !text.is_empty() {
            self.reserve_bytes(text.len() as u64)?;
        }
        self.narration_components.push(text);
        Ok(())
    }

    fn add_poem(&mut self, document: &str, username: &str, glyphs: &HudFontGlyphMap) -> Result<()> {
        let mut random = LegacyRandom::new(8_124_371);
        for source_line in document.lines() {
            let expanded = if source_line.contains("PLAYERNAME") {
                std::borrow::Cow::Owned(source_line.replace("PLAYERNAME", username))
            } else {
                std::borrow::Cow::Borrowed(source_line)
            };
            let replaced = replace_obfuscate_tokens(&expanded, &mut random);
            let runs = legacy_formatted_runs(&replaced);
            let narration = plain_runs(&runs);
            for wrapped in wrap_runs(&runs, POEM_WIDTH, glyphs) {
                self.push_line(wrapped, false)?;
            }
            self.push_narration(narration)?;
            self.add_empty_line()?;
        }
        for _ in 0..8 {
            self.add_empty_line()?;
        }
        Ok(())
    }

    fn add_credits(&mut self, sections: &[CreditsSection]) -> Result<()> {
        for section in sections {
            self.add_credits_line("============", Some(WHITE), true, false)?;
            self.add_credits_line(&section.section, Some(YELLOW), true, true)?;
            self.add_credits_line("============", Some(WHITE), true, false)?;
            self.add_empty_line()?;
            self.add_empty_line()?;
            for discipline in &section.disciplines {
                if !discipline.discipline.is_empty() {
                    self.add_credits_line(&discipline.discipline, Some(YELLOW), true, true)?;
                    self.add_empty_line()?;
                    self.add_empty_line()?;
                }
                for title in &discipline.titles {
                    self.add_credits_line(&title.title, Some(GRAY), false, true)?;
                    for name in &title.names {
                        self.add_credits_line(
                            &format!("{NAME_PREFIX}{name}"),
                            Some(WHITE),
                            false,
                            true,
                        )?;
                    }
                    self.add_empty_line()?;
                    self.add_empty_line()?;
                }
            }
        }
        Ok(())
    }

    fn add_credits_line(
        &mut self,
        text: &str,
        color: Option<u32>,
        centered: bool,
        narrated: bool,
    ) -> Result<()> {
        self.push_line(
            vec![HudStyledTextRun {
                shadow_color: None,
                text: text.to_string(),
                style: HudTextStyle::default(),
                color,
            }],
            centered,
        )?;
        if narrated {
            self.push_narration(text.to_string())?;
        }
        Ok(())
    }

    fn add_empty_line(&mut self) -> Result<()> {
        self.reserve_bytes(DERIVED_LINE_LOGICAL_BYTES)?;
        self.lines.push(WinScreenLine::default());
        self.narration_components.push(String::new());
        Ok(())
    }

    fn finish(self) -> WinScreenResources {
        WinScreenResources {
            lines: self.lines,
            narration_components: self.narration_components,
            total_scroll_length: 0,
        }
    }
}

#[derive(Debug, Deserialize)]
struct CreditsSection {
    section: String,
    disciplines: Vec<CreditsDiscipline>,
}

#[derive(Debug, Deserialize)]
struct CreditsDiscipline {
    discipline: String,
    titles: Vec<CreditsTitle>,
}

#[derive(Debug, Deserialize)]
struct CreditsTitle {
    title: String,
    names: Vec<String>,
}

fn admit_credits_counts(resources: &PackResourceStack, sections: &[CreditsSection]) -> Result<()> {
    let disciplines = sections
        .iter()
        .try_fold(0u64, |count, section| {
            count.checked_add(section.disciplines.len() as u64)
        })
        .context("credits discipline count overflow")?;
    let titles = sections
        .iter()
        .flat_map(|section| &section.disciplines)
        .try_fold(0u64, |count, discipline| {
            count.checked_add(discipline.titles.len() as u64)
        })
        .context("credits title count overflow")?;
    let names = sections
        .iter()
        .flat_map(|section| &section.disciplines)
        .flat_map(|discipline| &discipline.titles)
        .try_fold(0u64, |count, title| {
            count.checked_add(title.names.len() as u64)
        })
        .context("credits name count overflow")?;
    for (count, label) in [
        (sections.len() as u64, "win-screen credit sections"),
        (disciplines, "win-screen credit disciplines"),
        (titles, "win-screen credit titles"),
        (names, "win-screen credit names"),
    ] {
        resources.admit_resource_collection_count(count, MAX_CREDITS_ENTRIES, label)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StyledChar {
    shadow_color: Option<u32>,
    ch: char,
    style: HudTextStyle,
    color: Option<u32>,
}

fn legacy_formatted_runs(text: &str) -> Vec<HudStyledTextRun> {
    use bbb_render_types::{visit_legacy_text, LegacyTextFormat};
    let mut chars = Vec::new();
    let initial = (HudTextStyle::default(), None);
    visit_legacy_text(
        text,
        initial,
        &initial,
        |(style, color), format| {
            // Style: color clears decorations; reset restores the supplied style.
            match format {
                LegacyTextFormat::Color(value) => {
                    *style = HudTextStyle::default();
                    *color = Some(value);
                }
                LegacyTextFormat::Obfuscated => style.obfuscated = true,
                LegacyTextFormat::Bold => style.bold = true,
                LegacyTextFormat::Strikethrough => style.strikethrough = true,
                LegacyTextFormat::Underlined => style.underlined = true,
                LegacyTextFormat::Italic => style.italic = true,
            }
        },
        |source, &(style, color)| {
            chars.push(StyledChar {
                ch: source.ch,
                style,
                color,
                shadow_color: None,
            });
            true
        },
    );
    chars_to_runs(&chars)
}

fn wrap_runs(
    runs: &[HudStyledTextRun],
    max_width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<Vec<HudStyledTextRun>> {
    let chars = runs
        .iter()
        .flat_map(|run| {
            run.text.chars().map(move |ch| StyledChar {
                shadow_color: run.shadow_color,
                ch,
                style: run.style,
                color: run.color,
            })
        })
        .collect::<Vec<_>>();
    let mut lines = Vec::new();
    let max_width_f = max_width.max(1) as f32;
    let mut start = 0usize;
    while start < chars.len() {
        let remaining = &chars[start..];
        let mut width = 0.0_f32;
        let mut had_nonzero = false;
        let mut last_space = None;
        let mut split = remaining.len();
        for (index, styled) in remaining.iter().enumerate() {
            if styled.ch == ' ' {
                last_space = Some(index);
            }
            let advance = glyphs.styled_char_advance(styled.ch, styled.style);
            width += advance;
            if had_nonzero && width > max_width_f {
                split = last_space.unwrap_or(index);
                break;
            }
            had_nonzero |= advance != 0.0;
        }
        let skip =
            usize::from(split < remaining.len() && matches!(remaining[split].ch, ' ' | '\n'));
        let advance_len = split.saturating_add(skip);
        if advance_len == 0 {
            lines.push(chars_to_runs(&remaining[..1]));
            start += 1;
        } else {
            lines.push(chars_to_runs(&remaining[..split]));
            start += advance_len;
        }
    }
    lines
}

fn chars_to_runs(chars: &[StyledChar]) -> Vec<HudStyledTextRun> {
    let mut runs = Vec::<HudStyledTextRun>::new();
    for styled in chars {
        if let Some(run) = runs.last_mut().filter(|run| {
            run.style == styled.style
                && run.color == styled.color
                && run.shadow_color == styled.shadow_color
        }) {
            run.text.push(styled.ch);
        } else {
            runs.push(HudStyledTextRun {
                shadow_color: styled.shadow_color,
                text: styled.ch.to_string(),
                style: styled.style,
                color: styled.color,
            });
        }
    }
    runs
}

fn plain_runs(runs: &[HudStyledTextRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
}

fn replace_obfuscate_tokens(line: &str, random: &mut LegacyRandom) -> String {
    let mut result = String::with_capacity(line.len());
    let mut cursor = 0usize;
    while let Some(offset) = line[cursor..].find(OBFUSCATE_TOKEN) {
        let position = cursor + offset;
        result.push_str(&line[cursor..position]);
        let count = random.next_int(4) + 3;
        result.push_str("§f§k");
        result.push_str(&"XXXXXXXX"[..count]);
        cursor = position + OBFUSCATE_TOKEN.len();
    }
    result.push_str(&line[cursor..]);
    result
}

struct LegacyRandom {
    seed: u64,
}

impl LegacyRandom {
    fn new(seed: u64) -> Self {
        Self {
            seed: (seed ^ 25_214_903_917) & ((1u64 << 48) - 1),
        }
    }

    fn next(&mut self, bits: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(25_214_903_917).wrapping_add(11) & ((1u64 << 48) - 1);
        (self.seed >> (48 - bits)) as u32
    }

    fn next_int(&mut self, bound: usize) -> usize {
        debug_assert!(bound.is_power_of_two());
        ((bound as u64 * self.next(31) as u64) >> 31) as usize
    }
}

const WHITE: u32 = 0xFF_FF_FF;
const YELLOW: u32 = 0xFF_FF_55;
const GRAY: u32 = 0xAA_AA_AA;

#[cfg(test)]
mod tests {
    use bbb_client_render::{HudAsciiGlyph, HudUvRect};

    use super::*;

    #[test]
    fn win_screen_resources_match_poem_substitution_wrapping_and_credit_styles() {
        let raw = WinScreenTextResources {
            end_poem: Some(
                "§2PLAYERNAME says abc def\n§3secret §f§k§a§b§3 tail".to_string(),
            ),
            credits_json: Some(
                r#"[{"section":"Studio","disciplines":[{"discipline":"Code","titles":[{"title":"Developer","names":["Alex"]}]}]}]"#
                    .to_string(),
            ),
            postcredits: None,
        };
        let glyphs = fixed_width_glyphs(64);

        let resources = WinScreenResources::from_raw(raw, "Player", &glyphs, None).unwrap();

        assert_eq!(resources.lines[0].runs[0].text, "Play");
        assert_eq!(resources.lines[1].runs[0].text, "er");
        assert_eq!(resources.lines[0].runs[0].color, Some(0x00_AA_00));
        assert!(resources
            .lines
            .iter()
            .flat_map(|line| &line.runs)
            .any(|run| run.style.obfuscated && (3..=6).contains(&run.text.len())));
        let section = resources
            .lines
            .iter()
            .find(|line| line.runs.first().is_some_and(|run| run.text == "Studio"))
            .unwrap();
        assert!(section.centered);
        assert_eq!(section.runs[0].color, Some(YELLOW));
        let name = resources
            .lines
            .iter()
            .find(|line| {
                line.runs
                    .first()
                    .is_some_and(|run| run.text.ends_with("Alex"))
            })
            .unwrap();
        assert!(!name.centered);
        assert_eq!(name.runs[0].text, format!("{NAME_PREFIX}Alex"));
        assert_eq!(
            resources.total_scroll_length,
            resources.lines.len() as u32 * LINE_HEIGHT
        );
        assert!(resources
            .narration_components
            .iter()
            .any(|line| line == "Player says abc def"));
    }

    #[test]
    fn win_screen_resources_omit_invalid_credits_but_keep_poem_padding() {
        let resources = WinScreenResources::from_raw(
            WinScreenTextResources {
                end_poem: Some(String::new()),
                credits_json: Some("{".to_string()),
                postcredits: None,
            },
            "Player",
            &fixed_width_glyphs(6),
            None,
        )
        .unwrap();

        assert_eq!(resources.lines.len(), 8);
        assert!(resources.lines.iter().all(|line| line.runs.is_empty()));
        assert_eq!(resources.total_scroll_length, 96);
    }

    #[test]
    fn win_text_wrap_accumulates_fractional_and_negative_advances() {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, advance) in [('A', 0.5), ('B', -2.25), ('C', 6.0), ('Z', 0.0)] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance,
                    ..Default::default()
                },
            );
        }
        let lines = wrap_runs(&[HudStyledTextRun::plain("AAAZ".to_string())], 1, &glyphs);
        let text = |line: &Vec<HudStyledTextRun>| {
            line.iter().map(|run| run.text.as_str()).collect::<String>()
        };
        assert_eq!(lines.iter().map(text).collect::<Vec<_>>(), ["AA", "AZ"]);
        let lines = wrap_runs(&[HudStyledTextRun::plain("BCAZ".to_string())], 5, &glyphs);
        assert_eq!(lines.iter().map(text).collect::<Vec<_>>(), ["BCAZ"]);
    }

    #[test]
    fn win_legacy_text_uses_utf16_consumption_and_color_reset() {
        let runs = legacy_formatted_runs("§lA§aB§rC§😀D§");
        assert_eq!(
            runs.iter().map(|r| r.text.as_str()).collect::<String>(),
            "ABC�D"
        );
        assert!(runs[0].style.bold);
        assert!(!runs[1].style.bold);
        assert_eq!(runs[1].color, Some(0x55ff55));
        assert_eq!(runs[2].color, None);
    }

    #[test]
    fn win_screen_resources_linear_obfuscate_and_wrap_enforce_incremental_budget() {
        let glyphs = fixed_width_glyphs(8);
        let repeated_tokens = OBFUSCATE_TOKEN.repeat(512);
        let mut random = LegacyRandom::new(8_124_371);
        let replaced = replace_obfuscate_tokens(&repeated_tokens, &mut random);
        assert!(!replaced.contains(OBFUSCATE_TOKEN));
        assert!(replaced.len() >= 512 * 7);

        let nanos = bbb_client_clock::SystemTime::now()
            .duration_since(bbb_client_clock::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp = std::env::temp_dir().join(format!(
            "bbb-win-screen-budget-{}-{nanos}",
            std::process::id()
        ));
        let sources = temp.join("sources");
        let base = PackRoots::from_paths(&temp, &sources, sources.join("assets/minecraft"));
        let selected = base.selected_client_packs_or_unmanaged();
        let limited = base
            .with_client_resource_generation_limits(1, selected, 64, 512)
            .unwrap();
        let resources = limited.resource_stack();
        let err = WinScreenResources::from_raw(
            WinScreenTextResources {
                end_poem: Some("abcdefghij ".repeat(500)),
                credits_json: None,
                postcredits: None,
            },
            "Player",
            &glyphs,
            Some(&resources),
        )
        .unwrap_err();
        assert!(
            format!("{err:#}").contains("budget")
                || resources.ensure_resource_data_admitted().is_err()
        );
    }

    fn fixed_width_glyphs(advance: u32) -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        for ch in ' '..='~' {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    uv: HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    width: advance,
                    height: 8,
                    advance: advance as f32,
                    bearing_top: 7.0,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        glyphs
    }
}
