pub const HUD_SOUND_SUBTITLE_MAX_ROWS: usize = 64;
const HUD_SOUND_SUBTITLE_MAX_TEXT_CHARS: usize = 256;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HudSoundSubtitles {
    pub rows: Vec<HudSoundSubtitleRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudSoundSubtitleRow {
    pub text: String,
    pub brightness: u8,
    pub direction: HudSoundSubtitleDirection,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HudSoundSubtitleDirection {
    #[default]
    None,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HudSoundSubtitleLayout {
    pub(super) rows: Vec<HudSoundSubtitleRowLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HudSoundSubtitleRowLayout {
    pub(super) background: HudSoundSubtitleRect,
    pub(super) text_origin: (i32, i32),
    pub(super) arrow: Option<HudSoundSubtitleArrowLayout>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudSoundSubtitleArrowLayout {
    pub(super) text: &'static str,
    pub(super) origin: (i32, i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudSoundSubtitleRect {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: u32,
    pub(super) height: u32,
}

pub(super) fn sanitize_hud_sound_subtitles(mut subtitles: HudSoundSubtitles) -> HudSoundSubtitles {
    let mut kept = 0usize;
    subtitles.rows.retain_mut(|row| {
        if kept >= HUD_SOUND_SUBTITLE_MAX_ROWS {
            return false;
        }
        let Some(text) = sanitize_subtitle_text(std::mem::take(&mut row.text)) else {
            return false;
        };
        row.text = text;
        kept += 1;
        true
    });
    subtitles
}

pub(super) fn hud_sound_subtitle_layout(
    subtitles: &HudSoundSubtitles,
    surface_width: u32,
    surface_height: u32,
    mut measure: impl FnMut(&str) -> i32,
) -> HudSoundSubtitleLayout {
    let shared_width = subtitles
        .rows
        .iter()
        .map(|row| measure(&row.text))
        .fold(0, i32::max)
        .saturating_add(measure("<"))
        .saturating_add(measure(" "))
        .saturating_add(measure(">"))
        .saturating_add(measure(" "));
    let half_width = i64::from(shared_width / 2);
    let surface_width = i64::from(surface_width);
    let surface_height = i64::from(surface_height);
    let translate_x = surface_width - half_width - 2;
    let rows = subtitles
        .rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            let translate_y = surface_height
                - 35
                - i64::try_from(row_index)
                    .unwrap_or(i64::MAX)
                    .saturating_mul(10);
            let text_width = i64::from(measure(&row.text));
            let arrow = match row.direction {
                HudSoundSubtitleDirection::None => None,
                HudSoundSubtitleDirection::Left => Some(HudSoundSubtitleArrowLayout {
                    text: "<",
                    origin: (
                        clamp_i64_to_i32(translate_x - half_width),
                        clamp_i64_to_i32(translate_y - 4),
                    ),
                }),
                HudSoundSubtitleDirection::Right => {
                    let arrow_width = i64::from(measure(">"));
                    Some(HudSoundSubtitleArrowLayout {
                        text: ">",
                        origin: (
                            clamp_i64_to_i32(translate_x + half_width - arrow_width),
                            clamp_i64_to_i32(translate_y - 4),
                        ),
                    })
                }
            };
            HudSoundSubtitleRowLayout {
                background: HudSoundSubtitleRect {
                    x: clamp_i64_to_i32(translate_x - half_width - 1),
                    y: clamp_i64_to_i32(translate_y - 5),
                    width: u32::try_from(half_width.saturating_mul(2).saturating_add(2).max(0))
                        .unwrap_or(u32::MAX),
                    height: 10,
                },
                text_origin: (
                    clamp_i64_to_i32(translate_x - text_width / 2),
                    clamp_i64_to_i32(translate_y - 4),
                ),
                arrow,
            }
        })
        .collect();
    HudSoundSubtitleLayout { rows }
}

fn sanitize_subtitle_text(text: String) -> Option<String> {
    let (mut text, _) =
        super::sanitize_hud_chars_with_count(text, HUD_SOUND_SUBTITLE_MAX_TEXT_CHARS, false);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.len() == text.len() {
        return Some(text);
    }
    let start = trimmed.as_ptr() as usize - text.as_ptr() as usize;
    let end = start + trimmed.len();
    text.truncate(end);
    if start > 0 {
        text.drain(..start);
    }
    Some(text)
}

fn clamp_i64_to_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: impl Into<String>, direction: HudSoundSubtitleDirection) -> HudSoundSubtitleRow {
        HudSoundSubtitleRow {
            text: text.into(),
            brightness: 255,
            direction,
        }
    }

    #[test]
    fn sound_subtitle_sanitizer_bounds_rows_and_plain_text() {
        let mut rows = vec![row("\n Note \u{0}", HudSoundSubtitleDirection::None)];
        rows.extend(
            (0..HUD_SOUND_SUBTITLE_MAX_ROWS)
                .map(|index| row(format!("row {index}"), HudSoundSubtitleDirection::None)),
        );
        rows.push(row("discarded", HudSoundSubtitleDirection::None));

        let sanitized = sanitize_hud_sound_subtitles(HudSoundSubtitles { rows });

        assert_eq!(sanitized.rows.len(), HUD_SOUND_SUBTITLE_MAX_ROWS);
        assert_eq!(sanitized.rows[0].text, "Note");
        assert_eq!(
            sanitize_hud_sound_subtitles(HudSoundSubtitles {
                rows: vec![row("\n\u{0}", HudSoundSubtitleDirection::None)],
            })
            .rows,
            Vec::new()
        );
    }

    #[test]
    fn sound_subtitle_layout_matches_vanilla_shared_width_geometry() {
        let subtitles = HudSoundSubtitles {
            rows: vec![
                row("wide", HudSoundSubtitleDirection::Left),
                row("n", HudSoundSubtitleDirection::Right),
            ],
        };
        let layout = hud_sound_subtitle_layout(&subtitles, 320, 240, |text| match text {
            "wide" => 24,
            "n" => 6,
            "<" | ">" => 6,
            " " => 4,
            _ => 0,
        });

        assert_eq!(
            layout.rows[0],
            HudSoundSubtitleRowLayout {
                background: HudSoundSubtitleRect {
                    x: 273,
                    y: 200,
                    width: 46,
                    height: 10,
                },
                text_origin: (284, 201),
                arrow: Some(HudSoundSubtitleArrowLayout {
                    text: "<",
                    origin: (274, 201),
                }),
            }
        );
        assert_eq!(layout.rows[1].background.y, 190);
        assert_eq!(
            layout.rows[1].arrow,
            Some(HudSoundSubtitleArrowLayout {
                text: ">",
                origin: (312, 191),
            })
        );
    }
}
