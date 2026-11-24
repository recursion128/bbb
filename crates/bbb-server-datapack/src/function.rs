//! `ServerFunctionLibrary` surface (`ServerFunctionLibrary.java:66-127`): `.mcfunction`
//! files from the `function/` tree, winner-take-all, each compiled line-wise.
//!
//! D4 keeps the `CommandFunction.fromLines` line surface only
//! (`CommandFunction.java:24-94`): trimmed lines, `\` continuation, `#` comments,
//! `/` rejection, `$` macro marking and the 2,000,000-character command length cap.
//! Brigadier compilation against the server command tree is the other
//! half of `fromLines` and lives where a command tree exists to compile
//! against: `world::server_functions::compile_library` runs it once per library
//! and drops the functions vanilla's loader would have dropped. Macro
//! substitution stays a load-time property of the held lines. Function tags stop
//! at their entries here, because vanilla builds them only once the compiled
//! function map is in place (`ServerFunctionLibrary.java:76-78`, `:113-114`).

use std::fmt;
use std::sync::Arc;

use super::registry_data_loader::DatapackLoadLimits;
use super::resource_overlay::{MergedServerDataView, PackOverlayInput};
use super::tag_loader::{load_tag_entries_for_registry, LoadedTagEntries, TagLoadWarning};

/// `CommandFunction.checkCommandLineLength` (`CommandFunction.java:89-94`).
pub const MAX_COMMAND_LINE_CHARS: usize = 2_000_000;

/// One held command line after the `fromLines` line surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionLine {
    /// The trimmed, continuation-folded line.
    pub line: Arc<[u16]>,
}

impl FunctionLine {
    /// `$`-prefixed lines are macro lines (`CommandFunction.java:74`).
    pub fn is_macro(&self) -> bool {
        self.line.first() == Some(&u16::from(b'$'))
    }
}

/// The held line surface of one `.mcfunction` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFunctionSource {
    pub lines: Vec<FunctionLine>,
}

impl CommandFunctionSource {
    /// `MacroFunction`-equivalent marker: the function uses `$` variables.
    pub fn has_macros(&self) -> bool {
        self.lines.iter().any(FunctionLine::is_macro)
    }
}

/// A line-surface parse failure (`fromLines` throws `IllegalArgumentException`/
/// `IllegalStateException`; the library drops the function warn-only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParseError {
    pub line_number: usize,
    pub message: String,
}

impl fmt::Display for FunctionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line_number, self.message)
    }
}

impl std::error::Error for FunctionParseError {}

/// `CommandFunction.fromLines` (`CommandFunction.java:29-86`) minus the dispatcher
/// parse: line trimming, `\` continuation folding, comment skipping, `/` rejection
/// and `$` macro marking.
pub fn parse_function_source(text: &str) -> Result<CommandFunctionSource, FunctionParseError> {
    let mut raw = text.lines().enumerate();
    let mut lines = Vec::new();
    while let Some((index, input)) = raw.next() {
        let line_number = index + 1;
        let mut line: Vec<u16> = trim_java(input).encode_utf16().collect();
        while line.last() == Some(&u16::from(b'\\')) {
            let Some((_, next)) = raw.next() else {
                return Err(FunctionParseError {
                    line_number,
                    message: "line continuation at end of file".to_string(),
                });
            };
            line.pop();
            line.extend(trim_java(next).encode_utf16());
            check_length(&line, line_number)?;
        }
        check_length(&line, line_number)?;
        let Some(&first) = line.first() else {
            continue;
        };
        if first == u16::from(b'#') {
            continue;
        }
        if first == u16::from(b'/') {
            let double_slash = line.get(1) == Some(&u16::from(b'/'));
            let line = String::from_utf16_lossy(&line);
            let message = if double_slash {
                format!("unknown or invalid command {line:?} (if you intended to make a comment, use '#' not '//')")
            } else {
                format!(
                    "unknown or invalid command {line:?} (do not use a preceding forwards slash)"
                )
            };
            return Err(FunctionParseError {
                line_number,
                message,
            });
        }
        lines.push(FunctionLine { line: line.into() });
    }
    Ok(CommandFunctionSource { lines })
}

/// Java `String.trim` removes `<= ' '` characters.
fn trim_java(line: &str) -> &str {
    line.trim_matches(|c: char| c <= ' ')
}

fn check_length(line: &[u16], line_number: usize) -> Result<(), FunctionParseError> {
    if line.len() > MAX_COMMAND_LINE_CHARS {
        return Err(FunctionParseError {
            line_number,
            message: format!("command too long: {} characters", line.len()),
        });
    }
    Ok(())
}

/// One loaded `.mcfunction`, attributed to the pack that won the overlay merge.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionElement {
    pub id: String,
    pub source_pack: String,
    pub source: CommandFunctionSource,
}

/// One warn-only function loading event (`ServerFunctionLibrary` logs and drops).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionLoadWarning {
    pub element_id: String,
    pub pack: String,
    pub message: String,
}

/// The function library for one datapack view.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionLibrary {
    /// Functions sorted by id (`TreeMap` iteration order of the merged listing).
    pub functions: Vec<FunctionElement>,
    /// The unresolved `tags/function` entries, built by whoever holds the
    /// compiled function map. `None` when no `tags/function` tree supplied any.
    pub tags: Option<LoadedTagEntries>,
    pub warnings: Vec<FunctionLoadWarning>,
    pub tag_warnings: Vec<TagLoadWarning>,
}

impl FunctionLibrary {
    pub fn function(&self, id: &str) -> Option<&FunctionElement> {
        self.functions.iter().find(|function| function.id == id)
    }
}

/// The 26.1 function registry id (`Registries.FUNCTION`).
pub const FUNCTION_REGISTRY_ID: &str = "minecraft:function";

/// `ServerFunctionLibrary.reload`: winner-take-all `.mcfunction` scan, warn-only
/// line-surface parse, and the `tags/function` entries `tagsLoader.load` answers.
pub fn load_function_library(
    view: &MergedServerDataView,
    packs: &[PackOverlayInput<'_>],
    limits: DatapackLoadLimits,
) -> FunctionLibrary {
    let mut functions = Vec::new();
    let mut warnings = Vec::new();
    for namespace in view.namespaces() {
        for path in view.list(namespace, "function") {
            let Some(element_path) = path
                .strip_prefix("function/")
                .and_then(|path| path.strip_suffix(".mcfunction"))
            else {
                continue;
            };
            let element_id = format!("{namespace}:{element_path}");
            let Some((pack_id, resources)) = view.winner(packs, namespace, path) else {
                continue;
            };
            let Some(bytes) = resources.read(namespace, path) else {
                continue;
            };
            if bytes.len() > limits.max_element_bytes {
                warnings.push(FunctionLoadWarning {
                    element_id,
                    pack: pack_id.to_string(),
                    message: format!("function file exceeds the budget ({} bytes)", bytes.len()),
                });
                continue;
            }
            let text = String::from_utf8_lossy(&bytes);
            match parse_function_source(&text) {
                Ok(source) => functions.push(FunctionElement {
                    id: element_id,
                    source_pack: pack_id.to_string(),
                    source,
                }),
                Err(parse) => warnings.push(FunctionLoadWarning {
                    element_id,
                    pack: pack_id.to_string(),
                    message: format!("failed to load function: {parse}"),
                }),
            }
        }
    }
    functions.sort_by(|a, b| a.id.cmp(&b.id));
    let mut tag_warnings = Vec::new();
    let tags =
        load_tag_entries_for_registry(packs, FUNCTION_REGISTRY_ID, limits, &mut tag_warnings);
    FunctionLibrary {
        functions,
        tags,
        warnings,
        tag_warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_folds_continuations_and_skips_comments() {
        let parsed =
            parse_function_source("  say hello  \\\n  world\n# comment\n\nsay second\n").unwrap();
        assert_eq!(parsed.lines.len(), 2);
        assert_eq!(
            parsed.lines[0].line.as_ref(),
            "say hello  world".encode_utf16().collect::<Vec<_>>()
        );
        assert!(!parsed.lines[0].is_macro());
        assert_eq!(
            parsed.lines[1].line.as_ref(),
            "say second".encode_utf16().collect::<Vec<_>>()
        );
        assert!(!parsed.has_macros());
    }

    #[test]
    fn marks_macro_lines_and_caps_length() {
        let parsed = parse_function_source("$say $(message)\nsay plain").unwrap();
        assert!(parsed.lines[0].is_macro());
        assert!(parsed.has_macros());
        let long = "a".repeat(MAX_COMMAND_LINE_CHARS + 1);
        assert!(parse_function_source(&long).is_err());
        let supplementary = "😀".repeat(MAX_COMMAND_LINE_CHARS / 2);
        assert!(parse_function_source(&supplementary).is_ok());
        assert!(parse_function_source(&format!("{supplementary}a")).is_err());
        assert!(parse_function_source(&format!("{supplementary}\\\na")).is_err());
    }
}
