pub(super) fn char_len(text: &str) -> usize {
    text.chars().count()
}

pub(super) fn byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

pub(super) fn word_position(text: &str, cursor: usize, direction: i32) -> usize {
    let mut result = cursor.min(char_len(text));
    let reverse = direction < 0;

    for _ in 0..direction.unsigned_abs() {
        if reverse {
            while result > 0 && is_word_separator(char_before(text, result)) {
                result -= 1;
            }
            while result > 0 && !is_word_separator(char_before(text, result)) {
                result -= 1;
            }
        } else {
            let length = char_len(text);
            while result < length && !is_word_separator(char_at(text, result)) {
                result += 1;
            }
            while result < length && is_word_separator(char_at(text, result)) {
                result += 1;
            }
        }
    }

    result
}

pub(super) fn remove_word_before_cursor(current: &mut String, cursor: &mut usize) {
    *cursor = (*cursor).min(char_len(current));
    let start = word_position(current, *cursor, -1);
    if start == *cursor {
        return;
    }
    let start_byte = byte_index(current, start);
    let end_byte = byte_index(current, *cursor);
    current.replace_range(start_byte..end_byte, "");
    *cursor = start;
}

pub(super) fn remove_word_at_cursor(current: &mut String, cursor: usize) {
    let start = cursor.min(char_len(current));
    let end = word_position(current, start, 1);
    if start == end {
        return;
    }
    let start_byte = byte_index(current, start);
    let end_byte = byte_index(current, end);
    current.replace_range(start_byte..end_byte, "");
}

fn char_before(text: &str, char_index: usize) -> char {
    text.chars()
        .nth(char_index.saturating_sub(1))
        .unwrap_or('\0')
}

fn char_at(text: &str, char_index: usize) -> char {
    text.chars().nth(char_index).unwrap_or('\0')
}

fn is_word_separator(ch: char) -> bool {
    matches!(ch, ' ' | '\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_position_matches_vanilla_space_steps() {
        let text = "alpha  beta gamma";

        assert_eq!(word_position(text, char_len(text), -1), 12);
        assert_eq!(word_position(text, 12, -1), 7);
        assert_eq!(word_position(text, 7, -1), 0);
        assert_eq!(word_position(text, 0, 1), 7);
        assert_eq!(word_position(text, 5, 1), 7);
        assert_eq!(word_position(text, 7, 1), 12);
        assert_eq!(word_position(text, 12, 1), char_len(text));
    }

    #[test]
    fn word_deletes_include_adjacent_space_like_vanilla() {
        let mut text = "alpha beta gamma".to_string();
        let mut cursor = 11;

        remove_word_before_cursor(&mut text, &mut cursor);
        assert_eq!(text, "alpha gamma");
        assert_eq!(cursor, 6);

        remove_word_at_cursor(&mut text, cursor);
        assert_eq!(text, "alpha ");
        assert_eq!(cursor, 6);
    }
}
