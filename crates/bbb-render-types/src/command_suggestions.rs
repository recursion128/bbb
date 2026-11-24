use crate::HudStyledTextRun;

/// The parse facts `CommandSuggestions.formatText` colours the edit box with:
/// the parsed argument ranges in Brigadier context order and the region the
/// parser did not read. Both are UTF-16 units of the raw edit-box value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HudCommandInputSyntax {
    pub arguments: Vec<HudCommandInputRange>,
    pub unread: Option<HudCommandInputRange>,
}

/// One half-open UTF-16 range of the raw input value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudCommandInputRange {
    pub start: usize,
    pub end: usize,
}

/// One suggestion row. `runs` is the already-resolved
/// `FormattedText.of(suggestion.getText())` list the list width was measured
/// from, so the row draw cannot re-decide which codes were consumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudCommandSuggestionEntry {
    pub runs: Vec<HudStyledTextRun>,
    pub selected: bool,
}
