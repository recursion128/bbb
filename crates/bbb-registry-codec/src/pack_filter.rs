//! Identifier patterns used in pack metadata and atlas sources.
//!
//! Vanilla 26.1 anchors: `ResourceFilterSection.java:9-27`, `IdentifierPattern.java:10-43`.

mod depth;

use java_regex::Regex as JavaRegex;
use regex::RegexBuilder;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct IdentifierPatternSource {
    pub namespace: Option<String>,
    pub path: Option<String>,
}

impl IdentifierPatternSource {
    pub fn compile(&self) -> Result<IdentifierPattern, String> {
        Ok(IdentifierPattern {
            namespace: self.namespace.as_deref().map(compile_pattern).transpose()?,
            path: self.path.as_deref().map(compile_pattern).transpose()?,
        })
    }
}

#[derive(Debug, Clone)]
enum Pattern {
    Literal(String),
    LinearRegex(regex::Regex),
    JavaRegex(JavaRegex),
}

const MAX_PATTERN_BYTES: usize = 16 * 1024;
const LINEAR_REGEX_SIZE_LIMIT: usize = 256 * 1024;

fn compile_pattern(source: &str) -> Result<Pattern, String> {
    if source.len() > MAX_PATTERN_BYTES {
        return Err(format!(
            "pack filter pattern length {} exceeds {MAX_PATTERN_BYTES} bytes",
            source.len()
        ));
    }
    if source
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'/' | b'-'))
    {
        return Ok(Pattern::Literal(source.to_owned()));
    }
    let summary = depth::validate(source)?;
    if summary.linear_regex_eligible {
        if let Ok(compiled) = RegexBuilder::new(source)
            .size_limit(LINEAR_REGEX_SIZE_LIMIT)
            .dfa_size_limit(LINEAR_REGEX_SIZE_LIMIT)
            .build()
        {
            return Ok(Pattern::LinearRegex(compiled));
        }
    }
    if summary.variable_quantifiers > depth::MAX_BACKTRACKING_VARIABLE_QUANTIFIERS {
        return Err(format!(
            "pack regex fallback variable quantifier count exceeds {}",
            depth::MAX_BACKTRACKING_VARIABLE_QUANTIFIERS
        ));
    }
    JavaRegex::new(source)
        .map(Pattern::JavaRegex)
        .map_err(|error| error.to_string())
}

#[derive(Debug, Clone)]
pub struct IdentifierPattern {
    namespace: Option<Pattern>,
    path: Option<Pattern>,
}

impl IdentifierPattern {
    pub fn matches(&self, namespace: &str, path: &str) -> bool {
        finds(&self.namespace, namespace) && finds(&self.path, path)
    }
}

fn finds(pattern: &Option<Pattern>, text: &str) -> bool {
    pattern.as_ref().is_none_or(|pattern| match pattern {
        Pattern::Literal(literal) => text.contains(literal),
        Pattern::LinearRegex(regex) => regex.is_match(text),
        Pattern::JavaRegex(regex) => regex.find_at(text, 0).is_some(),
    })
}

#[derive(Debug, Clone, Default)]
pub struct ResourceFilter {
    block: Vec<IdentifierPattern>,
}

impl ResourceFilter {
    pub fn new(block: Vec<IdentifierPattern>) -> Self {
        Self { block }
    }

    pub fn is_namespace_filtered(&self, namespace: &str) -> bool {
        self.block
            .iter()
            .any(|rule| finds(&rule.namespace, namespace))
    }

    // Each namespace has a resource manager that installs the pack's combined path filter.
    pub fn is_path_blocked(&self, namespace: &str, path: &str) -> bool {
        self.is_namespace_filtered(namespace)
            && self.block.iter().any(|rule| finds(&rule.path, path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_filter_accepts_flat_alternatives_and_literal_delimiters() {
        let path = format!(
            r"(?x)\Q{}\E|{}|# {}{}",
            "[".repeat(300),
            vec![r"(?:\(|[a-z&&[^b]]|a&b)"; 251].join("|"),
            "(".repeat(300),
            "\nstone",
        );
        let pattern = IdentifierPatternSource {
            namespace: None,
            path: Some(path),
        }
        .compile()
        .unwrap();
        assert!(pattern.matches("minecraft", "stone"));
    }

    #[test]
    fn pack_filter_rejects_excessive_nesting() {
        let path = format!("{}a{}", "(".repeat(251), ")".repeat(251));
        let error = IdentifierPatternSource {
            namespace: None,
            path: Some(path),
        }
        .compile()
        .unwrap_err();
        assert!(error.contains("nesting"));
    }

    #[test]
    fn pack_filter_rejects_redos_patterns_and_oversized_sources() {
        for (bad_pattern, expected_substr) in [
            ("(a+)+$", "nested repetition"),
            ("((a*))+", "nested repetition"),
            ("(a{2,5}){2,5}", "nested repetition"),
            ("(?:.?){50}z", "nested repetition"),
            ("(?:.{0,1}){50}z", "nested repetition"),
            ("(?:a{1,2}){50}z", "nested repetition"),
            ("(a|aa)+$", "alternation group"),
            ("((a|aa))+$", "alternation group"),
            ("((?:a|aa))+", "alternation group"),
            (".*.*.*.*.*.*.*.*.*z", "variable quantifier count"),
            ("(?=a).*.*.*.*.*z", "fallback variable quantifier count"),
            ("a{1000}", "quantifier bound"),
            (r"(a)\1", "backreference"),
        ] {
            let err = IdentifierPatternSource {
                namespace: None,
                path: Some(bad_pattern.to_string()),
            }
            .compile()
            .unwrap_err();
            assert!(
                err.contains(expected_substr),
                "expected `{expected_substr}` in `{err}` for `{bad_pattern}`"
            );
        }

        let too_long = "a".repeat(MAX_PATTERN_BYTES + 1);
        let err = IdentifierPatternSource {
            namespace: None,
            path: Some(too_long),
        }
        .compile()
        .unwrap_err();
        assert!(err.contains("length"));

        let valid = IdentifierPatternSource {
            namespace: Some("^(?:minecraft|custom)$".to_string()),
            path: Some(r"^(?:recipes|loot_table)/(?:wood/)?[a-z0-9_/-]+\.json$".to_string()),
        }
        .compile()
        .unwrap();
        assert!(matches!(valid.namespace, Some(Pattern::LinearRegex(_))));
        assert!(matches!(valid.path, Some(Pattern::LinearRegex(_))));
        assert!(valid.matches("minecraft", "recipes/oak_planks.json"));
        assert!(valid.matches("minecraft", "recipes/wood/oak_planks.json"));
        assert!(!valid.matches("minecraft", "textures/stone.png"));

        // Java-specific lookahead stays supported via `java_regex` fallback within tight bounds.
        let lookahead = IdentifierPatternSource {
            namespace: None,
            path: Some(r"^(?=recipes/)[a-z0-9_/-]+\.json$".to_string()),
        }
        .compile()
        .unwrap();
        assert!(matches!(lookahead.path, Some(Pattern::JavaRegex(_))));
        assert!(lookahead.matches("minecraft", "recipes/stone.json"));
        assert!(!lookahead.matches("minecraft", "loot_table/stone.json"));
    }
}
