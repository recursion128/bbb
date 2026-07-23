use anyhow::{Context, Result};
use bbb_pack::{PackResourceStack, PackRoots, WinScreenTextResources};
use bbb_renderer::{HudFontGlyphMap, HudStyledTextRun, HudTextStyle};
use serde::Deserialize;

const POEM_WIDTH: u32 = 256;
const LINE_HEIGHT: u32 = 12;
const NAME_PREFIX: &str = "           ";
const OBFUSCATE_TOKEN: &str = "§f§k§a§b";
const MAX_CREDITS_ENTRIES: u64 = 65_536;
const DERIVED_LINE_LOGICAL_BYTES: u64 = 96;

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
        let raw = WinScreenTextResources::load(roots);
        let resources = roots.resource_stack();
        Self::from_raw(raw, username, glyphs, Some(&resources))
    }

    fn from_raw(
        raw: WinScreenTextResources,
        username: &str,
        glyphs: &HudFontGlyphMap,
        resources: Option<&PackResourceStack>,
    ) -> Result<Self> {
        let mut builder = WinScreenResourcesBuilder::default();
        if let Some(poem) = raw.end_poem {
            builder.add_poem(&poem, username, glyphs);
        }
        if let Some(credits) = raw.credits_json {
            if let Ok(sections) = serde_json::from_str::<Vec<CreditsSection>>(&credits) {
                if let Some(resources) = resources {
                    admit_credits_counts(resources, &sections)?;
                }
                builder.add_credits(&sections);
            }
        }
        if let Some(postcredits) = raw.postcredits {
            builder.add_poem(&postcredits, username, glyphs);
        }
        let mut result = builder.finish();
        if let Some(resources) = resources {
            resources.reserve_derived_resource_bytes(
                result.derived_logical_bytes()?,
                "win-screen formatted lines",
            )?;
        }
        result.total_scroll_length = u32::try_from(result.lines.len())
            .context("win-screen line count does not fit u32")?
            .checked_mul(LINE_HEIGHT)
            .context("win-screen total scroll length overflow")?;
        Ok(result)
    }

    fn derived_logical_bytes(&self) -> Result<u64> {
        let line_bytes = u64::try_from(self.lines.len())
            .context("win-screen line count does not fit u64")?
            .checked_mul(DERIVED_LINE_LOGICAL_BYTES)
            .context("win-screen line logical size overflow")?;
        let text_bytes =
            self.lines
                .iter()
                .flat_map(|line| &line.runs)
                .try_fold(0u64, |bytes, run| {
                    bytes
                        .checked_add(run.text.len() as u64)
                        .context("win-screen rendered text logical size overflow")
                })?;
        let narration_bytes =
            self.narration_components
                .iter()
                .try_fold(0u64, |bytes, component| {
                    bytes
                        .checked_add(component.len() as u64)
                        .context("win-screen narration logical size overflow")
                })?;
        line_bytes
            .checked_add(text_bytes)
            .and_then(|bytes| bytes.checked_add(narration_bytes))
            .context("win-screen derived logical size overflow")
    }
}

#[derive(Default)]
struct WinScreenResourcesBuilder {
    lines: Vec<WinScreenLine>,
    narration_components: Vec<String>,
}

impl WinScreenResourcesBuilder {
    fn add_poem(&mut self, document: &str, username: &str, glyphs: &HudFontGlyphMap) {
        let mut random = LegacyRandom::new(8_124_371);
        for source_line in document.lines() {
            let replaced =
                replace_obfuscate_tokens(&source_line.replace("PLAYERNAME", username), &mut random);
            let runs = legacy_formatted_runs(&replaced);
            for wrapped in wrap_runs(&runs, POEM_WIDTH, glyphs) {
                self.lines.push(WinScreenLine {
                    runs: wrapped,
                    centered: false,
                });
            }
            self.narration_components.push(plain_runs(&runs));
            self.add_empty_line();
        }
        for _ in 0..8 {
            self.add_empty_line();
        }
    }

    fn add_credits(&mut self, sections: &[CreditsSection]) {
        for section in sections {
            self.add_credits_line("============", Some(WHITE), true, false);
            self.add_credits_line(&section.section, Some(YELLOW), true, true);
            self.add_credits_line("============", Some(WHITE), true, false);
            self.add_empty_line();
            self.add_empty_line();
            for discipline in &section.disciplines {
                if !discipline.discipline.is_empty() {
                    self.add_credits_line(&discipline.discipline, Some(YELLOW), true, true);
                    self.add_empty_line();
                    self.add_empty_line();
                }
                for title in &discipline.titles {
                    self.add_credits_line(&title.title, Some(GRAY), false, true);
                    for name in &title.names {
                        self.add_credits_line(
                            &format!("{NAME_PREFIX}{name}"),
                            Some(WHITE),
                            false,
                            true,
                        );
                    }
                    self.add_empty_line();
                    self.add_empty_line();
                }
            }
        }
    }

    fn add_credits_line(&mut self, text: &str, color: Option<u32>, centered: bool, narrated: bool) {
        self.lines.push(WinScreenLine {
            runs: vec![HudStyledTextRun {
                text: text.to_string(),
                style: HudTextStyle::default(),
                color,
            }],
            centered,
        });
        if narrated {
            self.narration_components.push(text.to_string());
        }
    }

    fn add_empty_line(&mut self) {
        self.lines.push(WinScreenLine::default());
        self.narration_components.push(String::new());
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
    ch: char,
    style: HudTextStyle,
    color: Option<u32>,
}

fn legacy_formatted_runs(text: &str) -> Vec<HudStyledTextRun> {
    let mut chars = Vec::new();
    let mut style = HudTextStyle::default();
    let mut color = None;
    let mut source = text.chars();
    while let Some(ch) = source.next() {
        if ch == '§' {
            let Some(code) = source.next() else {
                break;
            };
            apply_legacy_format(code, &mut style, &mut color);
            continue;
        }
        chars.push(StyledChar { ch, style, color });
    }
    chars_to_runs(&chars)
}

fn apply_legacy_format(code: char, style: &mut HudTextStyle, color: &mut Option<u32>) {
    if let Some(next_color) = legacy_color(code) {
        *style = HudTextStyle::default();
        *color = Some(next_color);
        return;
    }
    match code.to_ascii_lowercase() {
        'k' => style.obfuscated = true,
        'l' => style.bold = true,
        'm' => style.strikethrough = true,
        'n' => style.underlined = true,
        'o' => style.italic = true,
        'r' => {
            *style = HudTextStyle::default();
            *color = None;
        }
        _ => {}
    }
}

fn legacy_color(code: char) -> Option<u32> {
    Some(match code.to_ascii_lowercase() {
        '0' => 0x00_00_00,
        '1' => 0x00_00_AA,
        '2' => 0x00_AA_00,
        '3' => 0x00_AA_AA,
        '4' => 0xAA_00_00,
        '5' => 0xAA_00_AA,
        '6' => 0xFF_AA_00,
        '7' => 0xAA_AA_AA,
        '8' => 0x55_55_55,
        '9' => 0x55_55_FF,
        'a' => 0x55_FF_55,
        'b' => 0x55_FF_FF,
        'c' => 0xFF_55_55,
        'd' => 0xFF_55_FF,
        'e' => YELLOW,
        'f' => WHITE,
        _ => return None,
    })
}

fn wrap_runs(
    runs: &[HudStyledTextRun],
    max_width: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<Vec<HudStyledTextRun>> {
    let mut remaining = runs
        .iter()
        .flat_map(|run| {
            run.text.chars().map(move |ch| StyledChar {
                ch,
                style: run.style,
                color: run.color,
            })
        })
        .collect::<Vec<_>>();
    let mut lines = Vec::new();
    while !remaining.is_empty() {
        let mut width = 0u32;
        let mut had_nonzero = false;
        let mut last_space = None;
        let mut split = remaining.len();
        for (index, styled) in remaining.iter().enumerate() {
            if styled.ch == ' ' {
                last_space = Some(index);
            }
            let advance = glyphs.styled_char_advance(styled.ch, styled.style);
            width = width.saturating_add(advance);
            if had_nonzero && width > max_width.max(1) {
                split = last_space.unwrap_or(index);
                break;
            }
            had_nonzero |= advance != 0;
        }
        let skip =
            usize::from(split < remaining.len() && matches!(remaining[split].ch, ' ' | '\n'));
        lines.push(chars_to_runs(&remaining[..split]));
        remaining.drain(..split.saturating_add(skip));
    }
    lines
}

fn chars_to_runs(chars: &[StyledChar]) -> Vec<HudStyledTextRun> {
    let mut runs = Vec::<HudStyledTextRun>::new();
    for styled in chars {
        if let Some(run) = runs
            .last_mut()
            .filter(|run| run.style == styled.style && run.color == styled.color)
        {
            run.text.push(styled.ch);
        } else {
            runs.push(HudStyledTextRun {
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
    let mut result = line.to_string();
    while let Some(position) = result.find(OBFUSCATE_TOKEN) {
        let count = random.next_int(4) + 3;
        result.replace_range(
            position..position + OBFUSCATE_TOKEN.len(),
            &format!("§f§k{}", &"XXXXXXXX"[..count]),
        );
    }
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
    use bbb_renderer::{HudAsciiGlyph, HudUvRect};

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
                    advance,
                    ascent: 7,
                },
            );
        }
        glyphs
    }
}
