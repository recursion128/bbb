//! Bounds recursive groups, character-class trees, and catastrophic-backtracking
//! quantifier patterns before entering regex compilation.

const MAX_DEPTH: usize = 250;
pub(super) const MAX_VARIABLE_QUANTIFIERS: usize = 8;
pub(super) const MAX_BACKTRACKING_VARIABLE_QUANTIFIERS: usize = 4;
const MAX_QUANTIFIER_BOUND: u32 = 256;

#[derive(Clone, Copy, Debug)]
pub(super) struct ValidationSummary {
    pub(super) linear_regex_eligible: bool,
    pub(super) variable_quantifiers: usize,
}

enum Scope {
    Group {
        outer_comments: bool,
        has_repetition: bool,
        has_variable_length: bool,
        has_alternation: bool,
    },
    Class {
        first: bool,
    },
    Intersection,
}

#[derive(Clone, Copy)]
struct ClosedGroupSummary {
    has_repetition: bool,
    has_variable_length: bool,
    has_alternation: bool,
}

pub(super) fn validate(mut rest: &str) -> Result<ValidationSummary, String> {
    let mut scopes = Vec::new();
    let mut comments = false;
    let mut variable_quantifiers = 0usize;
    let mut linear_regex_eligible = true;
    let mut last_closed_group: Option<ClosedGroupSummary> = None;

    while let Some(c) = take_char(&mut rest) {
        if comments {
            if c == '#' {
                rest = rest.split_once('\n').map_or("", |(_, tail)| tail);
                continue;
            }
            if matches!(c, ' ' | '\n' | '\t' | '\r') {
                continue;
            }
        }
        let class = match scopes.last_mut() {
            Some(Scope::Class { first }) => {
                if c == ']' {
                    if *first {
                        *first = false;
                    } else {
                        scopes.pop();
                    }
                    last_closed_group = None;
                    continue;
                }
                *first = false;
                true
            }
            Some(Scope::Intersection) if c == ']' => {
                while matches!(scopes.last(), Some(Scope::Intersection)) {
                    scopes.pop();
                }
                scopes.pop();
                last_closed_group = None;
                continue;
            }
            Some(Scope::Intersection) => true,
            _ => false,
        };
        match c {
            '\\' => {
                let (single, escape_linear) = skip_escape(&mut rest, class)?;
                linear_regex_eligible &= escape_linear;
                if class && single {
                    let range_linear = skip_range_end(&mut rest)?;
                    linear_regex_eligible &= range_linear;
                }
                last_closed_group = None;
            }
            '[' => {
                if class {
                    // Nested character class (`[a-z[0-9]]`) is Java-specific syntax.
                    linear_regex_eligible = false;
                }
                rest = rest.strip_prefix('^').unwrap_or(rest);
                scopes.push(Scope::Class { first: true });
                last_closed_group = None;
            }
            '&' if class && rest.starts_with('&') => {
                // Bare intersection operands recurse; bracketed operands still form a nested
                // intersection tree when the Java engine evaluates the compiled character class.
                linear_regex_eligible = false;
                rest = &rest[1..];
                scopes.push(Scope::Intersection);
                last_closed_group = None;
            }
            '(' if !class => {
                let outer_comments = comments;
                if let Some(after_q) = rest.strip_prefix('?') {
                    let end = after_q
                        .find(|c| !matches!(c, 'i' | 'm' | 's' | 'x' | 'U' | 'd' | 'u' | '-'))
                        .unwrap_or(after_q.len());
                    let (flags, body) = after_q.split_at(end);
                    if body.starts_with([':', ')']) {
                        // Java `U`/`d`/`u` flags differ from Rust `regex` (e.g. `(?U)` means
                        // ungreedy in Rust `regex` vs `UNICODE_CHARACTER_CLASS` in Java).
                        if flags.contains(['U', 'd', 'u']) {
                            linear_regex_eligible = false;
                        }
                        let (set, clear) = flags.split_once('-').unwrap_or((flags, ""));
                        comments = (comments || set.contains('x')) && !clear.contains('x');
                        rest = &body[1..];
                        if body.starts_with(')') {
                            continue;
                        }
                    } else {
                        // Lookarounds (`(?=`, `(?!`, `(?<=`, `(?<!`), atomic groups (`(?>`),
                        // named groups (`(?<...>`), etc. fall back to `java_regex`.
                        linear_regex_eligible = false;
                    }
                }
                scopes.push(Scope::Group {
                    outer_comments,
                    has_repetition: false,
                    has_variable_length: false,
                    has_alternation: false,
                });
                last_closed_group = None;
            }
            ')' if !class => {
                if let Some(Scope::Group {
                    outer_comments,
                    has_repetition,
                    has_variable_length,
                    has_alternation,
                }) = scopes.pop()
                {
                    comments = outer_comments;
                    if let Some(Scope::Group {
                        has_repetition: parent_rep,
                        has_variable_length: parent_var,
                        has_alternation: parent_alt,
                        ..
                    }) = scopes.last_mut()
                    {
                        *parent_rep |= has_repetition;
                        *parent_var |= has_variable_length;
                        *parent_alt |= has_alternation;
                    }
                    last_closed_group = Some(ClosedGroupSummary {
                        has_repetition,
                        has_variable_length,
                        has_alternation,
                    });
                } else {
                    last_closed_group = None;
                }
            }
            '|' if !class => {
                if let Some(Scope::Group {
                    has_alternation, ..
                }) = scopes.last_mut()
                {
                    *has_alternation = true;
                }
                last_closed_group = None;
            }
            '*' | '+' if !class => {
                if consume_quantifier_modifier(&mut rest) {
                    linear_regex_eligible = false;
                }
                record_quantifier(
                    &mut scopes,
                    &mut variable_quantifiers,
                    last_closed_group.take(),
                    true,
                    true,
                )?;
            }
            '?' if !class => {
                if consume_quantifier_modifier(&mut rest) {
                    linear_regex_eligible = false;
                }
                record_quantifier(
                    &mut scopes,
                    &mut variable_quantifiers,
                    last_closed_group.take(),
                    false,
                    true,
                )?;
            }
            '{' if !class => {
                if let Some((can_repeat, is_variable)) = parse_brace_quantifier(&mut rest)? {
                    if consume_quantifier_modifier(&mut rest) {
                        linear_regex_eligible = false;
                    }
                    if can_repeat || is_variable {
                        record_quantifier(
                            &mut scopes,
                            &mut variable_quantifiers,
                            last_closed_group.take(),
                            can_repeat,
                            is_variable,
                        )?;
                    } else {
                        last_closed_group = None;
                    }
                } else {
                    last_closed_group = None;
                }
            }
            _ if class => {
                let range_linear = skip_range_end(&mut rest)?;
                linear_regex_eligible &= range_linear;
                last_closed_group = None;
            }
            _ => {
                last_closed_group = None;
            }
        }
        if scopes.len() > MAX_DEPTH {
            return Err(format!("pack regex nesting exceeds {MAX_DEPTH}"));
        }
    }
    Ok(ValidationSummary {
        linear_regex_eligible,
        variable_quantifiers,
    })
}

fn record_quantifier(
    scopes: &mut [Scope],
    variable_quantifiers: &mut usize,
    closed_group: Option<ClosedGroupSummary>,
    can_repeat: bool,
    is_variable: bool,
) -> Result<(), String> {
    if can_repeat {
        if let Some(summary) = closed_group {
            if summary.has_repetition || summary.has_variable_length {
                return Err("pack regex contains nested repetition quantifier".to_string());
            }
            if summary.has_alternation {
                return Err("pack regex repeats an alternation group".to_string());
            }
        }
    }
    if is_variable {
        *variable_quantifiers += 1;
        if *variable_quantifiers > MAX_VARIABLE_QUANTIFIERS {
            return Err(format!(
                "pack regex variable quantifier count exceeds {MAX_VARIABLE_QUANTIFIERS}"
            ));
        }
    }
    if let Some(Scope::Group {
        has_repetition,
        has_variable_length,
        ..
    }) = scopes.last_mut()
    {
        *has_repetition |= can_repeat;
        *has_variable_length |= is_variable;
    }
    Ok(())
}

/// Consumes an optional reluctant (`?`) or possessive (`+`) modifier.
/// Returns `true` when the modifier is possessive (`+`), which Rust `regex`
/// does not support.
fn consume_quantifier_modifier(rest: &mut &str) -> bool {
    if let Some(tail) = rest.strip_prefix('?') {
        *rest = tail;
        false
    } else if let Some(tail) = rest.strip_prefix('+') {
        *rest = tail;
        true
    } else {
        false
    }
}

/// Parses a `{m}`, `{m,}`, or `{m,n}` quantifier after the opening `{`.
/// Returns `Ok(None)` when `{` starts a literal brace rather than a quantifier,
/// `Ok(Some((can_repeat, is_variable)))` when it is a valid quantifier, or `Err`
/// if a bound exceeds `MAX_QUANTIFIER_BOUND`.
fn parse_brace_quantifier(rest: &mut &str) -> Result<Option<(bool, bool)>, String> {
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return Ok(None);
    }
    let min_str = &rest[..i];
    if i < bytes.len() && bytes[i] == b'}' {
        let min = parse_quantifier_bound(min_str)?;
        *rest = &rest[i + 1..];
        return Ok(Some((min > 1, false)));
    }
    if i >= bytes.len() || bytes[i] != b',' {
        return Ok(None);
    }
    let min = parse_quantifier_bound(min_str)?;
    let max_start = i + 1;
    let mut j = max_start;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j >= bytes.len() || bytes[j] != b'}' {
        return Ok(None);
    }
    let max_str = &rest[max_start..j];
    let (can_repeat, is_variable) = if max_str.is_empty() {
        (true, true)
    } else {
        let max = parse_quantifier_bound(max_str)?;
        (max > 1 || min > 1, min != max)
    };
    *rest = &rest[j + 1..];
    Ok(Some((can_repeat, is_variable)))
}

fn parse_quantifier_bound(digits: &str) -> Result<u32, String> {
    let value = digits.parse::<u32>().unwrap_or(u32::MAX);
    if value > MAX_QUANTIFIER_BOUND {
        return Err(format!(
            "pack regex quantifier bound {digits} exceeds {MAX_QUANTIFIER_BOUND}"
        ));
    }
    Ok(value)
}

fn take_char(rest: &mut &str) -> Option<char> {
    let mut chars = rest.chars();
    let c = chars.next()?;
    *rest = chars.as_str();
    Some(c)
}

// A range endpoint is read as an item, including a literal '#' in comments mode.
fn skip_range_end(rest: &mut &str) -> Result<bool, String> {
    if let Some(end) = rest.strip_prefix('-') {
        if !end.is_empty() && !end.starts_with(']') {
            *rest = end;
            if !rest.starts_with('[') && take_char(rest) == Some('\\') {
                let (_, linear) = skip_escape(rest, true)?;
                return Ok(linear);
            }
        }
    }
    Ok(true)
}

/// Advances past an escape sequence after `\`.
/// Returns `(is_single_char_for_class_range, is_linear_regex_eligible)`.
fn skip_escape(rest: &mut &str, in_class: bool) -> Result<(bool, bool), String> {
    match take_char(rest) {
        Some('1'..='9') if !in_class => {
            Err("pack regex backreferences are not permitted".to_string())
        }
        Some('k') if !in_class && rest.starts_with('<') => {
            Err("pack regex backreferences are not permitted".to_string())
        }
        Some('Q') => {
            let (quoted, tail) = rest.split_once(r"\E").unwrap_or((*rest, ""));
            *rest = tail;
            Ok((quoted.chars().count() == 1, false))
        }
        Some('p' | 'P') => {
            if rest.starts_with('{') {
                *rest = rest.split_once('}').map_or("", |(_, tail)| tail);
            } else {
                take_char(rest);
            }
            Ok((false, false))
        }
        Some('h' | 'H' | 'v' | 'V') => Ok((false, false)),
        Some('d' | 'D' | 'w' | 'W' | 's' | 'S') => Ok((false, true)),
        Some('x') if rest.starts_with('{') => {
            *rest = rest.split_once('}').map_or("", |(_, tail)| tail);
            Ok((true, false))
        }
        Some('c') => {
            take_char(rest);
            Ok((true, false))
        }
        Some('G' | 'Z' | 'R' | 'X' | 'u' | '0' | 'e') => Ok((true, false)),
        _ => Ok((true, true)),
    }
}
