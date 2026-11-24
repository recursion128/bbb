use super::DialogTextMeasurer;
use bbb_protocol::JavaString;

pub(super) struct DialogTextRaw<'a> {
    units: &'a [u16],
    pub(super) display: String,
    boundaries: Vec<usize>,
}

impl<'a> DialogTextRaw<'a> {
    pub(super) fn new(value: &'a JavaString) -> Self {
        Self::from_units(value.units())
    }

    fn from_units(units: &'a [u16]) -> Self {
        let mut boundaries = vec![0];
        let mut cursor = 0;
        while cursor < units.len() {
            cursor += if is_high(units[cursor])
                && units.get(cursor + 1).is_some_and(|&unit| is_low(unit))
            {
                2
            } else {
                1
            };
            boundaries.push(cursor);
        }
        Self {
            units,
            display: JavaString::from(units.to_vec()).to_display_string(),
            boundaries,
        }
    }

    pub(super) fn unit_cursor(&self, cursor: usize) -> usize {
        self.boundaries[cursor.min(self.boundaries.len() - 1)]
    }

    pub(super) fn display_start(
        &self,
        cursor: usize,
        width: i32,
        measurer: &dyn DialogTextMeasurer,
    ) -> usize {
        let prefix = Self::from_units(&self.units[..cursor.min(self.units.len())]);
        prefix.unit_cursor(measurer.display_start(
            &prefix.display,
            prefix.boundaries.len() - 1,
            width,
        ))
    }

    pub(super) fn cursor_for_width(
        &self,
        start: usize,
        width: i32,
        measurer: &dyn DialogTextMeasurer,
    ) -> usize {
        let start = start.min(self.units.len());
        let suffix = Self::from_units(&self.units[start..]);
        start + suffix.unit_cursor(measurer.cursor_for_width(&suffix.display, 0, width))
    }

    pub(super) fn slice(&self, begin: usize, end: usize) -> String {
        let begin = begin.min(self.units.len());
        JavaString::from(self.units[begin..end.clamp(begin, self.units.len())].to_vec())
            .to_display_string()
    }
}

fn is_high(unit: u16) -> bool {
    (0xd800..=0xdbff).contains(&unit)
}
fn is_low(unit: u16) -> bool {
    (0xdc00..=0xdfff).contains(&unit)
}
