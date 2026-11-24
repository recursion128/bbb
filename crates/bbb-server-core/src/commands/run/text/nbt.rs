//! TextComponentTagVisitor for command NBT contents.

use bbb_protocol::chat_formattings::ChatFormatting;
use bbb_protocol::nbt_summary::canonical_snbt_key_java_string;
use bbb_protocol::{
    canonical_snbt_java_string, component_appended_nbt, component_literal_nbt,
    component_styled_literal_nbt, ComponentStyle, JavaString, NbtSummaryValue,
};

// TextComponentTagVisitor.java:69-287, :334-382.
pub(in crate::commands::run) fn display(value: &NbtSummaryValue, plain: bool) -> NbtSummaryValue {
    let mut visitor = Visitor {
        plain,
        children: Vec::new(),
    };
    visitor.visit(value, 0);
    component_appended_nbt(&ComponentStyle::default(), visitor.children)
        .expect("NBT text fragments are components")
}

struct Visitor {
    plain: bool,
    children: Vec<NbtSummaryValue>,
}
impl Visitor {
    fn literal(&mut self, value: impl Into<JavaString>) {
        self.children.push(component_literal_nbt(value));
    }
    fn colored(&mut self, value: impl Into<JavaString>, color: ChatFormatting) {
        let style = if self.plain {
            ComponentStyle::default()
        } else {
            super::super::super::chat_formatting_style(color)
        };
        self.children
            .push(component_styled_literal_nbt(value, &style).expect("NBT color is representable"));
    }
    fn token(&mut self, text: &str, color: Option<ChatFormatting>) {
        match color {
            Some(color) => self.colored(text, color),
            None => self.literal(text),
        }
    }
    fn quoted(&mut self, value: JavaString, color: ChatFormatting) {
        let units = value.units();
        self.literal(JavaString::from(units[..1].to_vec()));
        self.colored(JavaString::from(units[1..units.len() - 1].to_vec()), color);
        self.literal(JavaString::from(units[units.len() - 1..].to_vec()));
    }
    fn visit(&mut self, value: &NbtSummaryValue, depth: usize) {
        match value {
            NbtSummaryValue::String(_) => {
                self.quoted(canonical_snbt_java_string(value), ChatFormatting::Green)
            }
            NbtSummaryValue::Byte(_)
            | NbtSummaryValue::Short(_)
            | NbtSummaryValue::Long(_)
            | NbtSummaryValue::Float(_)
            | NbtSummaryValue::Double(_) => {
                let text = canonical_snbt_java_string(value);
                let units = text.units();
                self.colored(
                    JavaString::from(units[..units.len() - 1].to_vec()),
                    ChatFormatting::Gold,
                );
                self.colored(
                    JavaString::from(units[units.len() - 1..].to_vec()),
                    ChatFormatting::Red,
                );
            }
            NbtSummaryValue::Int(value) => self.colored(value.to_string(), ChatFormatting::Gold),
            NbtSummaryValue::ByteArray(values) => self.array(
                "B",
                values.len(),
                values
                    .iter()
                    .take(128)
                    .map(|value| (value.to_string(), "b")),
            ),
            NbtSummaryValue::IntArray(values) => self.array(
                "I",
                values.len(),
                values.iter().take(128).map(|value| (value.to_string(), "")),
            ),
            NbtSummaryValue::LongArray(values) => self.array(
                "L",
                values.len(),
                values
                    .iter()
                    .take(128)
                    .map(|value| (value.to_string(), "L")),
            ),
            NbtSummaryValue::List(values) => {
                self.literal("[");
                if !values.is_empty() && depth >= 64 {
                    self.colored("<...>", ChatFormatting::Gray);
                } else {
                    let wrapped = values.len() < 8
                        && values.iter().any(|value| {
                            !matches!(
                                value,
                                NbtSummaryValue::Byte(_)
                                    | NbtSummaryValue::Short(_)
                                    | NbtSummaryValue::Int(_)
                                    | NbtSummaryValue::Long(_)
                                    | NbtSummaryValue::Float(_)
                                    | NbtSummaryValue::Double(_)
                            )
                        });
                    let length = if wrapped {
                        values.len().min(128)
                    } else {
                        values.len()
                    };
                    for (index, value) in values.iter().take(length).enumerate() {
                        if wrapped {
                            self.literal("");
                        }
                        self.visit(value, depth + 1);
                        if index + 1 < values.len() {
                            self.literal(",");
                            self.literal(" ");
                        }
                    }
                    if values.len() > length {
                        self.literal("");
                        self.colored("<...>", ChatFormatting::Gray);
                    }
                }
                self.literal("]");
            }
            NbtSummaryValue::Compound(fields) => {
                self.literal("{");
                if !fields.is_empty() && depth >= 64 {
                    self.colored("<...>", ChatFormatting::Gray);
                } else {
                    for (index, field) in fields.iter().enumerate() {
                        self.literal("");
                        let key = canonical_snbt_key_java_string(&field.name);
                        if key
                            .units()
                            .first()
                            .is_some_and(|unit| *unit == 34 || *unit == 39)
                        {
                            self.quoted(key, ChatFormatting::Aqua);
                        } else {
                            self.colored(key, ChatFormatting::Aqua);
                        }
                        self.literal(":");
                        self.literal(" ");
                        self.visit(&field.value, depth + 1);
                        if index + 1 < fields.len() {
                            self.literal(",");
                            self.literal(" ");
                        }
                    }
                }
                self.literal("}");
            }
        }
    }
    fn array<'a>(
        &mut self,
        prefix: &str,
        total_len: usize,
        values: impl Iterator<Item = (String, &'a str)>,
    ) {
        self.literal("[");
        self.token(prefix, Some(ChatFormatting::Red));
        self.literal(";");
        for (index, (value, suffix)) in values.enumerate() {
            self.literal(" ");
            self.colored(value, ChatFormatting::Gold);
            if !suffix.is_empty() {
                self.colored(suffix, ChatFormatting::Red);
            }
            if index + 1 < total_len {
                self.literal(",");
            }
        }
        if total_len > 128 {
            self.colored("<...>", ChatFormatting::Gray);
        }
        self.literal("]");
    }
}
