//! Shared language-template tokenization for native Component presenters.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalizedTemplatePart<'a> {
    Literal(&'a str),
    Argument { index: usize, raw: &'a str },
}

/// Visits vanilla `%s`, `%1$s`, and `%%` language-template parts in order.
/// Unknown format syntax remains literal, and callers decide how to present a
/// syntactically valid placeholder whose argument is unavailable.
pub(crate) fn visit_localized_template<'a>(
    template: &'a str,
    mut visitor: impl FnMut(LocalizedTemplatePart<'a>),
) {
    let bytes = template.as_bytes();
    let mut offset = 0;
    let mut sequential = 0_usize;
    while offset < bytes.len() {
        let Some(relative_percent) = bytes[offset..].iter().position(|byte| *byte == b'%') else {
            visitor(LocalizedTemplatePart::Literal(&template[offset..]));
            break;
        };
        let percent = offset + relative_percent;
        if percent > offset {
            visitor(LocalizedTemplatePart::Literal(&template[offset..percent]));
        }
        if bytes.get(percent + 1) == Some(&b'%') {
            visitor(LocalizedTemplatePart::Literal("%"));
            offset = percent + 2;
            continue;
        }
        if bytes.get(percent + 1) == Some(&b's') {
            visitor(LocalizedTemplatePart::Argument {
                index: sequential,
                raw: &template[percent..percent + 2],
            });
            sequential += 1;
            offset = percent + 2;
            continue;
        }

        let mut cursor = percent + 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        let argument_index = (cursor > percent + 1
            && bytes.get(cursor) == Some(&b'$')
            && bytes.get(cursor + 1) == Some(&b's'))
        .then(|| template[percent + 1..cursor].parse::<usize>().ok())
        .flatten()
        .and_then(|index| index.checked_sub(1));
        if let Some(index) = argument_index {
            let next = cursor + 2;
            visitor(LocalizedTemplatePart::Argument {
                index,
                raw: &template[percent..next],
            });
            offset = next;
        } else {
            visitor(LocalizedTemplatePart::Literal("%"));
            offset = percent + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localized_template_tokens_cover_sequential_indexed_escape_and_invalid_forms() {
        let mut parts = Vec::new();
        visit_localized_template("<%s> %2$s %% %q", |part| parts.push(part));
        assert_eq!(
            parts,
            vec![
                LocalizedTemplatePart::Literal("<"),
                LocalizedTemplatePart::Argument {
                    index: 0,
                    raw: "%s"
                },
                LocalizedTemplatePart::Literal("> "),
                LocalizedTemplatePart::Argument {
                    index: 1,
                    raw: "%2$s"
                },
                LocalizedTemplatePart::Literal(" "),
                LocalizedTemplatePart::Literal("%"),
                LocalizedTemplatePart::Literal(" "),
                LocalizedTemplatePart::Literal("%"),
                LocalizedTemplatePart::Literal("q"),
            ]
        );
    }
}
