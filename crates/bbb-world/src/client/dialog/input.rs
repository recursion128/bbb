use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

use super::{
    compound_entries, decode_component, find_entry, optional_bool, optional_i32, required_entry,
    required_string, string_value, DialogDecodeContext, DialogTextState,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogInputState {
    pub key: String,
    pub control: DialogInputControlState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogInputControlState {
    Boolean {
        label: DialogTextState,
        initial: bool,
        on_true: String,
        on_false: String,
    },
    NumberRange {
        width: i32,
        label: DialogTextState,
        label_format: String,
        range: DialogNumberRangeState,
    },
    SingleOption {
        width: i32,
        options: Vec<DialogSingleOptionEntryState>,
        label: DialogTextState,
        label_visible: bool,
    },
    Text {
        width: i32,
        label: DialogTextState,
        label_visible: bool,
        initial: String,
        max_length: i32,
        #[serde(default)]
        multiline: Option<DialogTextMultilineState>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogNumberRangeState {
    pub start_bits: u32,
    pub end_bits: u32,
    #[serde(default)]
    pub initial_bits: Option<u32>,
    #[serde(default)]
    pub step_bits: Option<u32>,
}

impl DialogNumberRangeState {
    pub fn start(&self) -> f32 {
        f32::from_bits(self.start_bits)
    }

    pub fn end(&self) -> f32 {
        f32::from_bits(self.end_bits)
    }

    pub fn initial(&self) -> Option<f32> {
        self.initial_bits.map(f32::from_bits)
    }

    pub fn step(&self) -> Option<f32> {
        self.step_bits.map(f32::from_bits)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogSingleOptionEntryState {
    pub id: String,
    #[serde(default)]
    pub display: Option<DialogTextState>,
    pub initial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogTextMultilineState {
    #[serde(default)]
    pub max_lines: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
}

#[cfg(test)]
pub(super) fn decode_dialog_inputs(
    values: &[NbtSummaryValue],
) -> Result<Vec<DialogInputState>, String> {
    decode_dialog_inputs_with_context(values, DialogDecodeContext::new(None, 0))
}

pub(super) fn decode_dialog_inputs_with_context(
    values: &[NbtSummaryValue],
    context: DialogDecodeContext<'_>,
) -> Result<Vec<DialogInputState>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| decode_dialog_input(value, index, context))
        .collect()
}

fn decode_dialog_input(
    value: &NbtSummaryValue,
    index: usize,
    context: DialogDecodeContext<'_>,
) -> Result<DialogInputState, String> {
    let what = format!("dialog input[{index}]");
    let entries = compound_entries(value, &what)?;
    let key = required_string(entries, "key", &what)?;
    if !is_valid_input_name(key) {
        return Err(format!("{what} key {key:?} is not a valid input name"));
    }
    Ok(DialogInputState {
        key: key.to_string(),
        control: decode_input_control(entries, &what, context)?,
    })
}

fn decode_input_control(
    entries: &[NbtSummaryEntry],
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogInputControlState, String> {
    match required_string(entries, "type", what)? {
        "boolean" | "minecraft:boolean" => Ok(DialogInputControlState::Boolean {
            label: decode_component(
                required_entry(entries, "label", what)?,
                "boolean input label",
                context,
            )?,
            initial: optional_bool(entries, "initial", false)?,
            on_true: optional_string(entries, "on_true", "true", what)?.to_string(),
            on_false: optional_string(entries, "on_false", "false", what)?.to_string(),
        }),
        "number_range" | "minecraft:number_range" => {
            let width = optional_i32(entries, "width", 200, what)?;
            validate_range(width, 1, 1024, &format!("{what} width"))?;
            Ok(DialogInputControlState::NumberRange {
                width,
                label: decode_component(
                    required_entry(entries, "label", what)?,
                    "number-range input label",
                    context,
                )?,
                label_format: optional_string(
                    entries,
                    "label_format",
                    "options.generic_value",
                    what,
                )?
                .to_string(),
                range: decode_number_range(entries, what)?,
            })
        }
        "single_option" | "minecraft:single_option" => {
            let width = optional_i32(entries, "width", 200, what)?;
            validate_range(width, 1, 1024, &format!("{what} width"))?;
            let options =
                decode_single_options(required_entry(entries, "options", what)?, what, context)?;
            Ok(DialogInputControlState::SingleOption {
                width,
                options,
                label: decode_component(
                    required_entry(entries, "label", what)?,
                    "single-option input label",
                    context,
                )?,
                label_visible: optional_bool(entries, "label_visible", true)?,
            })
        }
        "text" | "minecraft:text" => {
            let width = optional_i32(entries, "width", 200, what)?;
            validate_range(width, 1, 1024, &format!("{what} width"))?;
            let initial = optional_string(entries, "initial", "", what)?.to_string();
            let max_length = optional_i32(entries, "max_length", 32, what)?;
            if max_length <= 0 {
                return Err(format!("{what} max_length {max_length} must be positive"));
            }
            if initial.encode_utf16().count() > max_length as usize {
                return Err(format!(
                    "{what} initial text length exceeds max_length {max_length}"
                ));
            }
            Ok(DialogInputControlState::Text {
                width,
                label: decode_component(
                    required_entry(entries, "label", what)?,
                    "text input label",
                    context,
                )?,
                label_visible: optional_bool(entries, "label_visible", true)?,
                initial,
                max_length,
                multiline: find_entry(entries, "multiline")
                    .map(|value| decode_multiline(value, what))
                    .transpose()?,
            })
        }
        input_type => Err(format!("unsupported dialog input type {input_type}")),
    }
}

fn decode_number_range(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<DialogNumberRangeState, String> {
    let start_bits = required_f32_bits(entries, "start", what)?;
    let end_bits = required_f32_bits(entries, "end", what)?;
    let initial_bits = optional_f32_bits(entries, "initial", what)?;
    let step_bits = optional_f32_bits(entries, "step", what)?;
    if let Some(bits) = step_bits {
        let step = f32::from_bits(bits);
        if !(step > 0.0 && step <= f32::MAX) {
            return Err(format!(
                "{what} step {step} must be a positive finite float"
            ));
        }
    }
    if let Some(bits) = initial_bits {
        let start = f32::from_bits(start_bits);
        let end = f32::from_bits(end_bits);
        let initial = f32::from_bits(bits);
        let (min, max) = if start.is_nan() || end.is_nan() {
            (f32::NAN, f32::NAN)
        } else if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        if initial < min || initial > max {
            return Err(format!(
                "{what} initial {initial} is outside range [{min}, {max}]"
            ));
        }
    }
    Ok(DialogNumberRangeState {
        start_bits,
        end_bits,
        initial_bits,
        step_bits,
    })
}

fn decode_single_options(
    value: &NbtSummaryValue,
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<Vec<DialogSingleOptionEntryState>, String> {
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} options is not a list"));
    };
    if values.is_empty() {
        return Err(format!("{what} options must not be empty"));
    }
    let options = values
        .iter()
        .enumerate()
        .map(|(index, value)| decode_single_option(value, index, what, context))
        .collect::<Result<Vec<_>, _>>()?;
    if options.iter().filter(|option| option.initial).count() > 1 {
        return Err(format!("{what} has multiple initial options"));
    }
    Ok(options)
}

fn decode_single_option(
    value: &NbtSummaryValue,
    index: usize,
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogSingleOptionEntryState, String> {
    let value = unwrap_list_element(value);
    match value {
        NbtSummaryValue::String(id) => Ok(DialogSingleOptionEntryState {
            id: id.clone(),
            display: None,
            initial: false,
        }),
        NbtSummaryValue::Compound(entries) => Ok(DialogSingleOptionEntryState {
            id: required_string(entries, "id", &format!("{what} option[{index}]"))?.to_string(),
            display: find_entry(entries, "display")
                .map(|value| decode_component(value, "single-option display", context))
                .transpose()?,
            initial: optional_bool(entries, "initial", false)?,
        }),
        _ => Err(format!(
            "{what} option[{index}] is neither an id string nor a compound"
        )),
    }
}

fn unwrap_list_element(value: &NbtSummaryValue) -> &NbtSummaryValue {
    let NbtSummaryValue::Compound(entries) = value else {
        return value;
    };
    match entries.as_slice() {
        [NbtSummaryEntry { name, value }] if name.is_empty() => value,
        _ => value,
    }
}

fn decode_multiline(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogTextMultilineState, String> {
    let entries = compound_entries(value, &format!("{what} multiline"))?;
    let max_lines = optional_positive_i32(entries, "max_lines", &format!("{what} multiline"))?;
    let height = optional_i32_value(entries, "height", &format!("{what} multiline"))?;
    if let Some(height) = height {
        validate_range(height, 1, 512, &format!("{what} multiline height"))?;
    }
    Ok(DialogTextMultilineState { max_lines, height })
}

fn optional_string<'a>(
    entries: &'a [NbtSummaryEntry],
    key: &str,
    default: &'a str,
    what: &str,
) -> Result<&'a str, String> {
    find_entry(entries, key)
        .map(|value| string_value(value, &format!("{what} field {key}")))
        .unwrap_or(Ok(default))
}

fn required_f32_bits(entries: &[NbtSummaryEntry], key: &str, what: &str) -> Result<u32, String> {
    f32_bits(
        required_entry(entries, key, what)?,
        &format!("{what} field {key}"),
    )
}

fn optional_f32_bits(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<Option<u32>, String> {
    find_entry(entries, key)
        .map(|value| f32_bits(value, &format!("{what} field {key}")))
        .transpose()
}

fn f32_bits(value: &NbtSummaryValue, what: &str) -> Result<u32, String> {
    let value = match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => return Ok(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return Err(format!("{what} is not numeric")),
    };
    Ok(value.to_bits())
}

fn optional_i32_value(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<Option<i32>, String> {
    find_entry(entries, key)
        .map(|_| optional_i32(entries, key, 0, what))
        .transpose()
}

fn optional_positive_i32(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<Option<i32>, String> {
    let value = optional_i32_value(entries, key, what)?;
    if let Some(value) = value {
        if value <= 0 {
            return Err(format!("{what} field {key} must be positive"));
        }
    }
    Ok(value)
}

fn validate_range(value: i32, min: i32, max: i32, what: &str) -> Result<(), String> {
    if !(min..=max).contains(&value) {
        return Err(format!("{what} {value} is outside {min}..={max}"));
    }
    Ok(())
}

fn is_valid_input_name(value: &str) -> bool {
    value.chars().all(|character| {
        character == '_' || (character.len_utf16() == 1 && character.is_alphanumeric())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_boolean_and_number_range_inputs_with_defaults() {
        let inputs = decode_dialog_inputs(&[
            compound(vec![
                string_entry("key", "enabled_1"),
                string_entry("type", "minecraft:boolean"),
                value_entry("label", NbtSummaryValue::String("Enabled".to_string())),
                value_entry("initial", NbtSummaryValue::Byte(1)),
                string_entry("on_true", "yes"),
            ]),
            compound(vec![
                string_entry("key", "amount"),
                string_entry("type", "number_range"),
                value_entry("label", NbtSummaryValue::String("Amount".to_string())),
                value_entry("start", NbtSummaryValue::Float(10.0f32.to_bits())),
                value_entry("end", NbtSummaryValue::Int(0)),
                value_entry("initial", NbtSummaryValue::Double(4.0f64.to_bits())),
                value_entry("step", NbtSummaryValue::Float(0.5f32.to_bits())),
            ]),
        ])
        .unwrap();

        assert!(matches!(
            &inputs[0].control,
            DialogInputControlState::Boolean {
                initial: true,
                on_true,
                on_false,
                ..
            } if on_true == "yes" && on_false == "false"
        ));
        let DialogInputControlState::NumberRange {
            width,
            label_format,
            range,
            ..
        } = &inputs[1].control
        else {
            panic!("expected number-range input");
        };
        assert_eq!(*width, 200);
        assert_eq!(label_format, "options.generic_value");
        assert_eq!(range.start(), 10.0);
        assert_eq!(range.end(), 0.0);
        assert_eq!(range.initial(), Some(4.0));
        assert_eq!(range.step(), Some(0.5));
    }

    #[test]
    fn validates_number_range_initial_step_and_width() {
        for entries in [
            vec![
                value_entry("start", NbtSummaryValue::Float(0.0f32.to_bits())),
                value_entry("end", NbtSummaryValue::Float(1.0f32.to_bits())),
                value_entry("initial", NbtSummaryValue::Float(2.0f32.to_bits())),
            ],
            vec![
                value_entry("start", NbtSummaryValue::Int(0)),
                value_entry("end", NbtSummaryValue::Int(1)),
                value_entry("step", NbtSummaryValue::Float(0.0f32.to_bits())),
            ],
            vec![
                value_entry("start", NbtSummaryValue::Int(0)),
                value_entry("end", NbtSummaryValue::Int(1)),
                value_entry("width", NbtSummaryValue::Int(1025)),
            ],
        ] {
            let mut input = vec![
                string_entry("key", "range"),
                string_entry("type", "number_range"),
                value_entry("label", NbtSummaryValue::String("Range".to_string())),
            ];
            input.extend(entries);
            assert!(decode_dialog_inputs(&[compound(input)]).is_err());
        }
    }

    #[test]
    fn decodes_single_option_alternatives_and_validates_initials() {
        let inputs = decode_dialog_inputs(&[compound(vec![
            string_entry("key", "mode"),
            string_entry("type", "single_option"),
            value_entry("label", NbtSummaryValue::String("Mode".to_string())),
            value_entry("label_visible", NbtSummaryValue::Byte(0)),
            value_entry(
                "options",
                NbtSummaryValue::List(vec![
                    compound(vec![value_entry(
                        "",
                        NbtSummaryValue::String("compact".to_string()),
                    )]),
                    compound(vec![
                        string_entry("id", "full"),
                        value_entry("display", NbtSummaryValue::String("Full".to_string())),
                        value_entry("initial", NbtSummaryValue::Byte(1)),
                    ]),
                ]),
            ),
        ])])
        .unwrap();

        let DialogInputControlState::SingleOption {
            width,
            options,
            label_visible,
            ..
        } = &inputs[0].control
        else {
            panic!("expected single-option input");
        };
        assert_eq!(*width, 200);
        assert!(!label_visible);
        assert_eq!(options[0].id, "compact");
        assert_eq!(options[0].display, None);
        assert_eq!(options[1].display.as_ref().unwrap().text, "Full");
        assert!(options[1].initial);

        for options in [
            Vec::new(),
            vec![option("first", true), option("second", true)],
        ] {
            assert!(decode_dialog_inputs(&[compound(vec![
                string_entry("key", "mode"),
                string_entry("type", "single_option"),
                value_entry("label", NbtSummaryValue::String("Mode".to_string())),
                value_entry("options", NbtSummaryValue::List(options)),
            ])])
            .is_err());
        }
    }

    #[test]
    fn decodes_text_multiline_and_validates_utf16_length_and_keys() {
        let inputs = decode_dialog_inputs(&[compound(vec![
            string_entry("key", "message_文本"),
            string_entry("type", "text"),
            value_entry("label", NbtSummaryValue::String("Message".to_string())),
            string_entry("initial", "a😀"),
            value_entry("max_length", NbtSummaryValue::Int(3)),
            value_entry(
                "multiline",
                compound(vec![
                    value_entry("max_lines", NbtSummaryValue::Int(4)),
                    value_entry("height", NbtSummaryValue::Int(80)),
                ]),
            ),
        ])])
        .unwrap();
        let DialogInputControlState::Text {
            width,
            label_visible,
            initial,
            max_length,
            multiline,
            ..
        } = &inputs[0].control
        else {
            panic!("expected text input");
        };
        assert_eq!(*width, 200);
        assert!(label_visible);
        assert_eq!(initial, "a😀");
        assert_eq!(*max_length, 3);
        assert_eq!(multiline.as_ref().unwrap().max_lines, Some(4));
        assert_eq!(multiline.as_ref().unwrap().height, Some(80));

        for input in [
            compound(vec![
                string_entry("key", "bad-name"),
                string_entry("type", "text"),
                value_entry("label", NbtSummaryValue::String("Bad".to_string())),
            ]),
            compound(vec![
                string_entry("key", "text"),
                string_entry("type", "example:unknown"),
                value_entry("label", NbtSummaryValue::String("Bad".to_string())),
            ]),
            compound(vec![
                string_entry("key", "text"),
                string_entry("type", "text"),
                value_entry("label", NbtSummaryValue::String("Bad".to_string())),
                string_entry("initial", "😀"),
                value_entry("max_length", NbtSummaryValue::Int(1)),
            ]),
            compound(vec![
                string_entry("key", "text"),
                string_entry("type", "text"),
                value_entry("label", NbtSummaryValue::String("Bad".to_string())),
                value_entry(
                    "multiline",
                    compound(vec![value_entry("height", NbtSummaryValue::Int(513))]),
                ),
            ]),
        ] {
            assert!(decode_dialog_inputs(&[input]).is_err());
        }
    }

    fn option(id: &str, initial: bool) -> NbtSummaryValue {
        compound(vec![
            string_entry("id", id),
            value_entry("initial", NbtSummaryValue::Byte(i8::from(initial))),
        ])
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }

    fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
        value_entry(name, NbtSummaryValue::String(value.to_string()))
    }
}
