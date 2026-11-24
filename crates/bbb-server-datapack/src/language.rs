//! The dedicated server's `Language.getInstance()` (Language.java:34-66) and the
//! `Component.getString` walk (FormattedText.java:86-93) that reads it, which is
//! how the server compares a name without a client to translate it.

use std::collections::HashMap;
use std::io::Read;
use std::sync::OnceLock;

use bbb_protocol::{
    java_double_to_string, java_float_to_string, JavaString, NbtSummaryEntry, NbtSummaryValue,
};
use bbb_registry_codec::RegistryJsonValue as Json;
use flate2::read::GzDecoder;

/// `/assets/minecraft/lang/en_us.json` and `deprecated.json` of the 26.1 server
/// jar, byte for byte.
const EN_US: &[u8] = include_bytes!("../data/vanilla_lang_en_us_26_1.json.gz");
const DEPRECATED: &[u8] = include_bytes!("../data/vanilla_lang_deprecated_26_1.json.gz");

static SERVER_LANGUAGE: OnceLock<ServerLanguage> = OnceLock::new();

/// The table `Language.loadDefault` builds.
#[derive(Debug)]
pub struct ServerLanguage {
    storage: HashMap<JavaString, JavaString>,
}

impl ServerLanguage {
    /// The embedded default language every server-side `getString` reads.
    pub fn get() -> &'static Self {
        SERVER_LANGUAGE.get_or_init(|| {
            Self::load(&gunzip(EN_US), &gunzip(DEPRECATED))
                .expect("the embedded 26.1 language files are well formed")
        })
    }

    /// `Language.loadDefault` over two documents: `parseTranslations`
    /// (Language.java:68-74) with `loadFromJson`'s format rewrite (`:76-83`), then
    /// `DeprecatedTranslationsInfo.applyToMap` (DeprecatedTranslationsInfo.java:48-62).
    pub fn load(en_us: &str, deprecated: &str) -> Result<Self, String> {
        let entries: HashMap<JavaString, Box<serde_json::value::RawValue>> =
            serde_json::from_str(en_us).map_err(|error| error.to_string())?;
        let mut storage = HashMap::with_capacity(entries.len());
        for (key, value) in entries {
            let value = bbb_registry_codec::parse_registry_json(value.get().as_bytes())
                .map_err(|error| error.to_string())?;
            let text = match value {
                Json::String(text) => text,
                Json::Number(number) => number.into(),
                Json::Bool(flag) => flag.to_string().into(),
                other => return Err(format!("{key} is not a primitive: {other:?}")),
            };
            storage.insert(key, rewrite_numeric_formats(text.units()));
        }
        let deprecated: HashMap<String, Box<serde_json::value::RawValue>> =
            serde_json::from_str(deprecated).map_err(|error| error.to_string())?;
        let removed: Vec<JavaString> = serde_json::from_str(
            deprecated
                .get("removed")
                .ok_or("deprecated.json has no removed list")?
                .get(),
        )
        .map_err(|error| error.to_string())?;
        let renamed: HashMap<JavaString, JavaString> = serde_json::from_str(
            deprecated
                .get("renamed")
                .ok_or("deprecated.json has no renamed map")?
                .get(),
        )
        .map_err(|error| error.to_string())?;
        for key in removed {
            storage.remove(key.units());
        }
        for (from, to) in renamed {
            match storage.remove(from.units()) {
                Some(value) => {
                    storage.insert(to, value);
                }
                None => {
                    storage.remove(to.units());
                }
            }
        }
        Ok(Self { storage })
    }

    /// `Language.getOrDefault(elementId, defaultValue)`.
    pub fn get_or_default(&self, key: &str, default: &str) -> JavaString {
        JavaString::with_utf16(key, |units| self.storage.get(units).cloned())
            .unwrap_or_else(|| default.into())
    }

    pub fn has(&self, key: &str) -> bool {
        JavaString::with_utf16(key, |units| self.storage.contains_key(units))
    }
}

fn rewrite_numeric_formats(text: &[u16]) -> JavaString {
    let mut out = Vec::with_capacity(text.len());
    let mut index = 0;
    while index < text.len() {
        if text[index] == u16::from(b'%') {
            let start = index + 1;
            let mut cursor = start;
            while text
                .get(cursor)
                .is_some_and(|unit| (48..=57).contains(unit))
            {
                cursor += 1;
            }
            let group_end = if cursor > start && text.get(cursor) == Some(&u16::from(b'$')) {
                cursor += 1;
                cursor
            } else {
                cursor = start;
                start
            };
            while text
                .get(cursor)
                .is_some_and(|unit| (48..=57).contains(unit) || *unit == 46)
            {
                cursor += 1;
            }
            if text
                .get(cursor)
                .is_some_and(|unit| matches!(*unit, 100 | 102))
            {
                out.push(u16::from(b'%'));
                out.extend_from_slice(&text[start..group_end]);
                out.push(u16::from(b's'));
                index = cursor + 1;
                continue;
            }
        }
        out.push(text[index]);
        index += 1;
    }
    JavaString::from(out)
}

fn gunzip(bytes: &[u8]) -> String {
    let mut text = String::new();
    GzDecoder::new(bytes)
        .read_to_string(&mut text)
        .expect("the embedded language file decompresses to UTF-8");
    text
}

const MAX_PLAIN_TEXT_UNITS: usize = 65_536;
const MAX_PLAIN_TEXT_STEPS: usize = 65_536;
const MAX_PLAIN_TEXT_DEPTH: usize = 128;

struct PlainTextBudget {
    out: Vec<u16>,
    steps_remaining: usize,
    exhausted: bool,
}

impl PlainTextBudget {
    fn new() -> Self {
        Self {
            out: Vec::new(),
            steps_remaining: MAX_PLAIN_TEXT_STEPS,
            exhausted: false,
        }
    }

    fn consume_step(&mut self) -> bool {
        if self.exhausted {
            return false;
        }
        if self.steps_remaining == 0 {
            self.exhausted = true;
            return false;
        }
        self.steps_remaining -= 1;
        true
    }

    fn push_unit(&mut self, unit: u16) {
        if self.exhausted {
            return;
        }
        if self.out.len() >= MAX_PLAIN_TEXT_UNITS {
            self.exhausted = true;
            return;
        }
        self.out.push(unit);
    }

    fn push_units(&mut self, units: &[u16]) {
        if self.exhausted {
            return;
        }
        let remaining = MAX_PLAIN_TEXT_UNITS.saturating_sub(self.out.len());
        if units.len() > remaining {
            self.out.extend_from_slice(&units[..remaining]);
            self.exhausted = true;
        } else {
            self.out.extend_from_slice(units);
        }
    }

    fn push_str(&mut self, text: &str) {
        for unit in text.encode_utf16() {
            self.push_unit(unit);
            if self.exhausted {
                break;
            }
        }
    }
}

/// `Component.getString()` of a component in its codec form, as UTF-16 units so
/// a lone surrogate compares the way a Java `String` does. Parts the component
/// codec would not decode contribute nothing.
pub fn component_plain_text(component: &NbtSummaryValue) -> Vec<u16> {
    let mut budget = PlainTextBudget::new();
    append_component(component, &mut budget, 0);
    budget.out
}

/// `Component.translatable(key, args).getString()` whose arguments are already
/// components (TranslatableContents.java:165-176).
pub fn translatable_plain_text(key: &str, args: &[NbtSummaryValue]) -> Vec<u16> {
    let mut budget = PlainTextBudget::new();
    append_translatable(&JavaString::from(key), None, args, &mut budget, 0);
    budget.out
}

fn append_component(value: &NbtSummaryValue, budget: &mut PlainTextBudget, depth: usize) {
    if depth >= MAX_PLAIN_TEXT_DEPTH {
        budget.exhausted = true;
        return;
    }
    if !budget.consume_step() {
        return;
    }
    match value {
        NbtSummaryValue::String(text) => budget.push_units(text.units()),
        // `createFromList` (ComponentSerialization.java:74-82): the first entry
        // is the root and every other one a sibling appended to it.
        NbtSummaryValue::List(items) => {
            for item in items {
                if budget.exhausted {
                    break;
                }
                append_component(item, budget, depth + 1);
            }
        }
        NbtSummaryValue::Compound(entries) => {
            append_contents(entries, budget, depth);
            if !budget.exhausted {
                if let Some(NbtSummaryValue::List(siblings)) = find(entries, "extra") {
                    for sibling in siblings {
                        if budget.exhausted {
                            break;
                        }
                        append_component(sibling, budget, depth + 1);
                    }
                }
            }
        }
        _ => {}
    }
}

/// `createLegacyComponentMatcher` (ComponentSerialization.java:84-93): the
/// `type` field dispatches when present, and otherwise the first content codec
/// in `bootstrap` order (`:117-125`) whose field is there decodes it.
fn append_contents(entries: &[NbtSummaryEntry], budget: &mut PlainTextBudget, depth: usize) {
    const FUZZY_ORDER: [(&str, &str); 6] = [
        ("text", "text"),
        ("translatable", "translate"),
        ("keybind", "keybind"),
        ("score", "score"),
        ("selector", "selector"),
        ("nbt", "nbt"),
    ];
    let kind = match find(entries, "type") {
        Some(NbtSummaryValue::String(kind)) => match [
            "text",
            "translatable",
            "keybind",
            "score",
            "selector",
            "nbt",
            "object",
            "atlas",
            "player",
        ]
        .into_iter()
        .find(|name| kind == *name)
        {
            Some(kind) => kind,
            None => return,
        },
        Some(_) => return,
        None => FUZZY_ORDER
            .iter()
            .find(|(_, field)| find(entries, field).is_some())
            .map_or("object", |(kind, _)| kind),
    };
    match kind {
        "text" => append_string_field(entries, "text", budget),
        "translatable" => {
            let Some(NbtSummaryValue::String(key)) = find(entries, "translate") else {
                return;
            };
            // `lenientOptionalFieldOf("fallback")`: a malformed fallback is
            // dropped.
            let fallback = match find(entries, "fallback") {
                Some(NbtSummaryValue::String(fallback)) => Some(fallback),
                _ => None,
            };
            let args = match find(entries, "with") {
                Some(NbtSummaryValue::List(args)) => args.as_slice(),
                _ => &[][..],
            };
            append_translatable(key, fallback, args, budget, depth + 1);
        }
        // `KeybindResolver.keyResolver` (KeybindResolver.java:8) is the literal
        // name until a client installs its own.
        "keybind" => append_string_field(entries, "keybind", budget),
        // `SelectorContents.visit` (SelectorContents.java:51-53).
        "selector" => append_string_field(entries, "selector", budget),
        // `ObjectContents.visit` (ObjectContents.java:42-44).
        "object" => match find(entries, "fallback") {
            Some(fallback) => append_component(fallback, budget, depth + 1),
            None => append_object_default_fallback(entries, budget),
        },
        // `ScoreContents` and `NbtContents` keep `ComponentContents.visit`'s
        // empty default.
        _ => {}
    }
}

/// `ObjectInfo.defaultFallback` of the info `ObjectInfos.CODEC` decodes: the
/// `object` discriminator, else the first of `atlas` and `player` whose required
/// field is present (ObjectInfos.java:9-14).
fn append_object_default_fallback(entries: &[NbtSummaryEntry], budget: &mut PlainTextBudget) {
    let kind = match find(entries, "object") {
        Some(NbtSummaryValue::String(kind)) => {
            let Some(kind) = ["atlas", "player"].into_iter().find(|name| kind == *name) else {
                return;
            };
            kind
        }
        None if find(entries, "sprite").is_some() => "atlas",
        None if find(entries, "player").is_some() => "player",
        _ => return,
    };
    match kind {
        // `AtlasSprite.defaultFallback` (AtlasSprite.java:29-37), whose atlas
        // defaults to `minecraft:blocks` (`:10-14`).
        "atlas" => {
            let Some(NbtSummaryValue::String(sprite)) = find(entries, "sprite") else {
                return;
            };
            fn short_name(id: &str) -> &str {
                match id.split_once(':') {
                    Some(("minecraft", path)) => path,
                    _ => id,
                }
            }
            let Ok(sprite) = sprite.try_to_string() else {
                return;
            };
            let atlas = match find(entries, "atlas") {
                Some(NbtSummaryValue::String(atlas))
                    if atlas != "blocks" && atlas != "minecraft:blocks" =>
                {
                    let Ok(atlas) = atlas.try_to_string() else {
                        return;
                    };
                    Some(atlas)
                }
                _ => None,
            };
            budget.push_unit(u16::from(b'['));
            budget.push_str(short_name(&sprite));
            if let Some(atlas) = atlas {
                budget.push_unit(u16::from(b'@'));
                budget.push_str(short_name(&atlas));
            }
            budget.push_unit(u16::from(b']'));
        }
        // `PlayerSprite.defaultFallback` (PlayerSprite.java:23-25) over
        // `ResolvableProfile.name`: a bare name, or the profile's `name` field.
        "player" => {
            let name = match find(entries, "player") {
                Some(NbtSummaryValue::String(name)) => Some(name),
                Some(NbtSummaryValue::Compound(profile)) => match find(profile, "name") {
                    Some(NbtSummaryValue::String(name)) => Some(name),
                    _ => None,
                },
                _ => return,
            };
            match name {
                Some(name) => {
                    budget.push_unit(u16::from(b'['));
                    budget.push_units(name.units());
                    budget.push_str(" head]");
                }
                None => budget.push_str("[unknown player head]"),
            }
        }
        _ => {}
    }
}

fn append_string_field(entries: &[NbtSummaryEntry], field: &str, budget: &mut PlainTextBudget) {
    if let Some(NbtSummaryValue::String(text)) = find(entries, field) {
        budget.push_units(text.units());
    }
}

enum DecomposeOutcome {
    Complete,
    FormatError,
    BudgetExhausted,
}

/// `TranslatableContents.decompose` (TranslatableContents.java:86-100): a
/// template that does not decompose is shown whole.
fn append_translatable(
    key: &JavaString,
    fallback: Option<&JavaString>,
    args: &[NbtSummaryValue],
    budget: &mut PlainTextBudget,
    depth: usize,
) {
    if depth >= MAX_PLAIN_TEXT_DEPTH {
        budget.exhausted = true;
        return;
    }
    if !budget.consume_step() {
        return;
    }
    let language = ServerLanguage::get();
    let template = language
        .storage
        .get(key.units())
        .unwrap_or_else(|| fallback.unwrap_or(key));
    let start_len = budget.out.len();
    match decompose_template(template.units(), args, budget, depth) {
        DecomposeOutcome::Complete | DecomposeOutcome::BudgetExhausted => {}
        DecomposeOutcome::FormatError => {
            budget.out.truncate(start_len);
            budget.push_units(template.units());
        }
    }
}

/// `decomposeTemplate` (`:102-149`) over `FORMAT_PATTERN` (`:53`). `FormatError`
/// is the `TranslatableFormatException` the caller replaces with the template.
fn decompose_template(
    template: &[u16],
    args: &[NbtSummaryValue],
    budget: &mut PlainTextBudget,
    depth: usize,
) -> DecomposeOutcome {
    let mut replacement_index = 0usize;
    let mut current = 0usize;
    while let Some(found) = find_format(template, current) {
        if !budget.consume_step() {
            return DecomposeOutcome::BudgetExhausted;
        }
        if found.start > current {
            let prefix = &template[current..found.start];
            if prefix.contains(&u16::from(b'%')) {
                return DecomposeOutcome::FormatError;
            }
            budget.push_units(prefix);
            if budget.exhausted {
                return DecomposeOutcome::BudgetExhausted;
            }
        }
        let format = &template[found.start..found.end];
        if found.kind == u16::from(b'%') && format == [u16::from(b'%'); 2] {
            budget.push_unit(u16::from(b'%'));
            if budget.exhausted {
                return DecomposeOutcome::BudgetExhausted;
            }
        } else {
            if found.kind != u16::from(b's') {
                return DecomposeOutcome::FormatError;
            }
            let index = match found.position {
                Some(digits) => match digits.iter().try_fold(0_i32, |value, unit| {
                    value
                        .checked_mul(10)?
                        .checked_add(i32::from(*unit - u16::from(b'0')))
                }) {
                    Some(position) => position - 1,
                    None => return DecomposeOutcome::FormatError,
                },
                None => {
                    let index = i32::try_from(replacement_index).unwrap_or(i32::MAX);
                    replacement_index += 1;
                    index
                }
            };
            // `getArgument` (`:151-162`).
            let Some(arg) = usize::try_from(index)
                .ok()
                .and_then(|index| args.get(index))
            else {
                return DecomposeOutcome::FormatError;
            };
            append_argument(arg, budget, depth + 1);
            if budget.exhausted {
                return DecomposeOutcome::BudgetExhausted;
            }
        }
        current = found.end;
    }
    if current < template.len() {
        let tail = &template[current..];
        if tail.contains(&u16::from(b'%')) {
            return DecomposeOutcome::FormatError;
        }
        budget.push_units(tail);
    }
    DecomposeOutcome::Complete
}

/// One `with` entry: `ARG_CODEC` (`:33-37`) keeps a number, boolean or string
/// as the primitive whose `toString` is shown and reads anything else as a
/// component.
fn append_argument(arg: &NbtSummaryValue, budget: &mut PlainTextBudget, depth: usize) {
    let text = match arg {
        NbtSummaryValue::Byte(value) => value.to_string(),
        NbtSummaryValue::Short(value) => value.to_string(),
        NbtSummaryValue::Int(value) => value.to_string(),
        NbtSummaryValue::Long(value) => value.to_string(),
        NbtSummaryValue::Float(bits) => java_float_to_string(f32::from_bits(*bits)),
        NbtSummaryValue::Double(bits) => java_double_to_string(f64::from_bits(*bits)),
        other => return append_component(other, budget, depth),
    };
    budget.push_str(&text);
}

struct FormatMatch<'a> {
    start: usize,
    end: usize,
    position: Option<&'a [u16]>,
    kind: u16,
}

/// `Matcher.find(from)` for `%(?:(\d+)\$)?([A-Za-z%]|$)`.
fn find_format(template: &[u16], from: usize) -> Option<FormatMatch<'_>> {
    let bytes = template;
    for start in (from..bytes.len()).filter(|&index| bytes[index] == u16::from(b'%')) {
        let digits_end = (start + 1..bytes.len())
            .find(|&index| !(u16::from(b'0')..=u16::from(b'9')).contains(&bytes[index]))
            .unwrap_or(bytes.len());
        if digits_end > start + 1 && bytes.get(digits_end) == Some(&u16::from(b'$')) {
            if let Some((kind, end)) = format_kind_at(template, digits_end + 1) {
                return Some(FormatMatch {
                    start,
                    end,
                    position: Some(&template[start + 1..digits_end]),
                    kind,
                });
            }
        }
        if let Some((kind, end)) = format_kind_at(template, start + 1) {
            return Some(FormatMatch {
                start,
                end,
                position: None,
                kind,
            });
        }
    }
    None
}

/// `([A-Za-z%]|$)`, where `$` also matches ahead of one final line terminator.
fn format_kind_at(template: &[u16], at: usize) -> Option<(u16, usize)> {
    let rest = &template[at..];
    match rest.first().copied() {
        Some(unit)
            if (u16::from(b'A')..=u16::from(b'Z')).contains(&unit)
                || (u16::from(b'a')..=u16::from(b'z')).contains(&unit)
                || unit == u16::from(b'%') =>
        {
            Some((unit, at + 1))
        }
        _ if matches!(
            rest,
            [] | [10] | [13, 10] | [13] | [0x85] | [0x2028] | [0x2029]
        ) =>
        {
            Some((0, at))
        }
        _ => None,
    }
}

fn find<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

#[cfg(test)]
mod tests;
