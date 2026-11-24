use crate::{
    content_registry::ContentRegistry, entity_types::vanilla_entity_type_for_resource_id,
    nbt_summary::parse_snbt_compound_prefix_utf16, JavaString,
};

mod generated_26_1;

pub use generated_26_1::VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEntitySelectorOptionKind {
    Name,
    Once,
    Limit,
    Sort,
    Gamemode,
    Team,
    Type,
    Repeatable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandEntitySelectorOption {
    pub name: &'static str,
    pub kind: CommandEntitySelectorOptionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntitySelectorPrefix {
    pub consumed: usize,
    pub multiple: bool,
    pub includes_entities: bool,
    pub self_selector: bool,
    /// Range of the `advancements={...}` payload this parser accepted, braces
    /// included, measured the way the entry point measured `consumed`. This
    /// parser only validates, so a consumer that needs the ids reads them back
    /// out of the span rather than out of a list this type would have to
    /// carry.
    pub advancements_span: Option<(usize, usize)>,
}

/// Every position is a byte index into `value`.
pub(super) fn parse_entity_selector_prefix(value: &str) -> Option<EntitySelectorPrefix> {
    parse_entity_selector_prefix_with_registry(value, ContentRegistry::vanilla())
}

pub(super) fn parse_entity_selector_prefix_with_registry(
    value: &str,
    registry: &ContentRegistry,
) -> Option<EntitySelectorPrefix> {
    let units: Vec<u16> = value.encode_utf16().collect();
    let parsed = parse_entity_selector_prefix_utf16_with_registry(&units, registry)?;
    let byte = |units: usize| super::syntax::byte_cursor(value, units);
    Some(EntitySelectorPrefix {
        consumed: byte(parsed.consumed),
        advancements_span: parsed
            .advancements_span
            .map(|(start, end)| (byte(start), byte(end))),
        ..parsed
    })
}

/// [`parse_entity_selector_prefix`] over a buffer's own units, with every
/// position in units, so a name or a condition value made of units no `String`
/// can hold reads the way Vanilla's `StringReader` reads it.
pub(super) fn parse_entity_selector_prefix_utf16(value: &[u16]) -> Option<EntitySelectorPrefix> {
    parse_entity_selector_prefix_utf16_with_registry(value, ContentRegistry::vanilla())
}

pub(super) fn parse_entity_selector_prefix_utf16_with_registry(
    value: &[u16],
    registry: &ContentRegistry,
) -> Option<EntitySelectorPrefix> {
    SelectorParser::new(value, registry).parse()
}

#[derive(Default)]
struct SelectorState {
    multiple: bool,
    includes_entities: bool,
    current_entity: bool,
    type_limited: bool,
    type_inverse: bool,
    name_equals: bool,
    name_not_equals: bool,
    distance: bool,
    level: bool,
    x: bool,
    y: bool,
    z: bool,
    delta_x: bool,
    delta_y: bool,
    delta_z: bool,
    rot_x: bool,
    rot_y: bool,
    limited: bool,
    sorted: bool,
    gamemode_equals: bool,
    gamemode_not_equals: bool,
    team_equals: bool,
    team_not_equals: bool,
    scores: bool,
    advancements: bool,
    advancements_span: Option<(usize, usize)>,
}

struct SelectorParser<'a> {
    input: &'a [u16],
    cursor: usize,
    state: SelectorState,
    registry: &'a ContentRegistry,
}

impl<'a> SelectorParser<'a> {
    fn new(input: &'a [u16], registry: &'a ContentRegistry) -> Self {
        Self {
            input,
            cursor: 0,
            state: SelectorState::default(),
            registry,
        }
    }

    fn parse(mut self) -> Option<EntitySelectorPrefix> {
        let valid = if self.peek() == Some(AT) {
            self.bump();
            self.parse_selector()
        } else {
            self.parse_name_or_uuid()
        };
        valid.then_some(EntitySelectorPrefix {
            consumed: self.cursor,
            multiple: self.state.multiple,
            includes_entities: self.state.includes_entities,
            self_selector: self.state.current_entity,
            advancements_span: self.state.advancements_span,
        })
    }

    fn parse_selector(&mut self) -> bool {
        match self.bump() {
            Some(LETTER_A) => {
                self.state.multiple = true;
                self.state.type_limited = true;
            }
            Some(LETTER_P | LETTER_R) => self.state.type_limited = true,
            Some(LETTER_S) => {
                self.state.includes_entities = true;
                self.state.current_entity = true;
            }
            Some(LETTER_E) => {
                self.state.multiple = true;
                self.state.includes_entities = true;
            }
            Some(LETTER_N) => self.state.includes_entities = true,
            _ => return false,
        }
        self.peek() != Some(OPEN_BRACKET) || self.parse_options()
    }

    fn parse_name_or_uuid(&mut self) -> bool {
        let Some(name) = self.read_string() else {
            return false;
        };
        if is_java_uuid_string(&ascii_word(name.units())) {
            self.state.includes_entities = true;
            true
        } else {
            !name.units().is_empty() && name.units().len() <= 16
        }
    }

    fn parse_options(&mut self) -> bool {
        self.consume(OPEN_BRACKET);
        self.skip_whitespace();
        while self.peek().is_some_and(|next| next != CLOSE_BRACKET) {
            self.skip_whitespace();
            let Some(key) = self.read_string() else {
                return false;
            };
            self.skip_whitespace();
            if !self.consume(EQUALS) {
                return false;
            }
            self.skip_whitespace();
            if !self.parse_option(&ascii_word(key.units())) {
                return false;
            }
            self.skip_whitespace();
            match self.peek() {
                Some(COMMA) => {
                    self.bump();
                }
                Some(CLOSE_BRACKET) => break,
                _ => return false,
            }
        }
        self.consume(CLOSE_BRACKET)
    }

    fn parse_option(&mut self, key: &str) -> bool {
        match key {
            "name" => self.parse_name_option(),
            "distance" => self.parse_distance_option(),
            "level" => self.parse_level_option(),
            "x" => self.parse_double_once(SelectorField::X),
            "y" => self.parse_double_once(SelectorField::Y),
            "z" => self.parse_double_once(SelectorField::Z),
            "dx" => self.parse_double_once(SelectorField::DeltaX),
            "dy" => self.parse_double_once(SelectorField::DeltaY),
            "dz" => self.parse_double_once(SelectorField::DeltaZ),
            "x_rotation" => self.parse_rotation_option(true),
            "y_rotation" => self.parse_rotation_option(false),
            "limit" => self.parse_limit_option(),
            "sort" => self.parse_sort_option(),
            "gamemode" => self.parse_gamemode_option(),
            "team" => self.parse_team_option(),
            "type" => self.parse_type_option(),
            "tag" => self.parse_tag_option(),
            "nbt" => self.parse_nbt_option(),
            "scores" => self.parse_scores_option(),
            "advancements" => self.parse_advancements_option(),
            "predicate" => self.parse_predicate_option(),
            _ => false,
        }
    }

    fn parse_name_option(&mut self) -> bool {
        if self.state.name_equals {
            return false;
        }
        let inverted = self.read_inversion();
        let Some(_) = self.read_string() else {
            return false;
        };
        if self.state.name_not_equals && !inverted {
            return false;
        }
        if inverted {
            self.state.name_not_equals = true;
        } else {
            self.state.name_equals = true;
        }
        true
    }

    fn parse_distance_option(&mut self) -> bool {
        if self.state.distance {
            return false;
        }
        let Some(range) = self.read_double_range(true) else {
            return false;
        };
        if range.min.is_some_and(|value| value < 0.0) || range.max.is_some_and(|value| value < 0.0)
        {
            return false;
        }
        self.state.distance = true;
        true
    }

    fn parse_level_option(&mut self) -> bool {
        if self.state.level {
            return false;
        }
        let Some(range) = self.read_int_range() else {
            return false;
        };
        if range.min.is_some_and(|value| value < 0) || range.max.is_some_and(|value| value < 0) {
            return false;
        }
        self.state.level = true;
        // `EntitySelectorOptions.java:123`, on the same branch that accepts the
        // bounds -- a negative bound throws before reaching it.
        self.state.includes_entities = false;
        true
    }

    fn parse_double_once(&mut self, field: SelectorField) -> bool {
        let already_set = match field {
            SelectorField::X => self.state.x,
            SelectorField::Y => self.state.y,
            SelectorField::Z => self.state.z,
            SelectorField::DeltaX => self.state.delta_x,
            SelectorField::DeltaY => self.state.delta_y,
            SelectorField::DeltaZ => self.state.delta_z,
        };
        if already_set || self.read_double().is_none() {
            return false;
        }
        match field {
            SelectorField::X => self.state.x = true,
            SelectorField::Y => self.state.y = true,
            SelectorField::Z => self.state.z = true,
            SelectorField::DeltaX => self.state.delta_x = true,
            SelectorField::DeltaY => self.state.delta_y = true,
            SelectorField::DeltaZ => self.state.delta_z = true,
        }
        true
    }

    fn parse_rotation_option(&mut self, x_axis: bool) -> bool {
        let already_set = if x_axis {
            self.state.rot_x
        } else {
            self.state.rot_y
        };
        if already_set || self.read_float_range().is_none() {
            return false;
        }
        if x_axis {
            self.state.rot_x = true;
        } else {
            self.state.rot_y = true;
        }
        true
    }

    fn parse_limit_option(&mut self) -> bool {
        if self.state.current_entity || self.state.limited {
            return false;
        }
        let Some(limit) = self.read_int() else {
            return false;
        };
        if limit < 1 {
            return false;
        }
        self.state.multiple = limit > 1;
        self.state.limited = true;
        true
    }

    fn parse_sort_option(&mut self) -> bool {
        if self.state.current_entity || self.state.sorted {
            return false;
        }
        if !matches!(
            ascii_word(self.read_unquoted_string().units()).as_str(),
            "nearest" | "furthest" | "random" | "arbitrary"
        ) {
            return false;
        }
        self.state.sorted = true;
        true
    }

    fn parse_gamemode_option(&mut self) -> bool {
        if self.state.gamemode_equals {
            return false;
        }
        let inverted = self.read_inversion();
        if self.state.gamemode_not_equals && !inverted {
            return false;
        }
        if !matches!(
            ascii_word(self.read_unquoted_string().units()).as_str(),
            "survival" | "creative" | "adventure" | "spectator"
        ) {
            return false;
        }
        if inverted {
            self.state.gamemode_not_equals = true;
        } else {
            self.state.gamemode_equals = true;
        }
        // `EntitySelectorOptions.java:235`, past the name check and regardless
        // of the inversion.
        self.state.includes_entities = false;
        true
    }

    fn parse_team_option(&mut self) -> bool {
        if self.state.team_equals {
            return false;
        }
        let inverted = self.read_inversion();
        self.read_unquoted_string();
        if inverted {
            self.state.team_not_equals = true;
        } else {
            self.state.team_equals = true;
        }
        true
    }

    fn parse_type_option(&mut self) -> bool {
        if self.state.type_limited {
            return false;
        }
        let inverted = self.read_inversion();
        if self.state.type_inverse && !inverted {
            return false;
        }
        if inverted {
            self.state.type_inverse = true;
        }
        let tagged = self.read_tag_prefix();
        let Some(resource_id) = self.read_identifier() else {
            return false;
        };
        if !tagged
            && vanilla_entity_type_for_resource_id(&resource_id).is_none()
            && self.registry.entity_by_resource_id(&resource_id).is_none()
        {
            return false;
        }
        if !tagged && !inverted {
            self.state.type_limited = true;
        }
        // `EntitySelectorOptions.java:296` narrows to players only for the
        // uninverted player type, and for no other type and no tag.
        if !tagged && !inverted && resource_id == "minecraft:player" {
            self.state.includes_entities = false;
        }
        true
    }

    fn parse_tag_option(&mut self) -> bool {
        self.read_inversion();
        self.read_unquoted_string();
        true
    }

    fn parse_nbt_option(&mut self) -> bool {
        self.read_inversion();
        let Ok((consumed, _)) = parse_snbt_compound_prefix_utf16(&self.input[self.cursor..]) else {
            return false;
        };
        self.cursor += consumed;
        true
    }

    fn parse_scores_option(&mut self) -> bool {
        if self.state.scores || !self.consume(OPEN_BRACE) {
            return false;
        }
        self.skip_whitespace();
        while self.peek().is_some_and(|next| next != CLOSE_BRACE) {
            self.skip_whitespace();
            self.read_unquoted_string();
            self.skip_whitespace();
            if !self.consume(EQUALS) {
                return false;
            }
            self.skip_whitespace();
            if self.read_int_range().is_none() {
                return false;
            }
            self.skip_whitespace();
            if self.peek() == Some(COMMA) {
                self.bump();
            }
        }
        if !self.consume(CLOSE_BRACE) {
            return false;
        }
        self.state.scores = true;
        true
    }

    fn parse_advancements_option(&mut self) -> bool {
        let start = self.cursor;
        if self.state.advancements || !self.consume(OPEN_BRACE) {
            return false;
        }
        let mut any_entry = false;
        self.skip_whitespace();
        while self.peek().is_some_and(|next| next != CLOSE_BRACE) {
            any_entry = true;
            self.skip_whitespace();
            if self.read_identifier().is_none() {
                return false;
            }
            self.skip_whitespace();
            if !self.consume(EQUALS) {
                return false;
            }
            self.skip_whitespace();
            if self.peek() == Some(OPEN_BRACE) {
                if !self.parse_criteria_map() {
                    return false;
                }
                self.skip_whitespace();
            } else if self.read_boolean().is_none() {
                return false;
            }
            self.skip_whitespace();
            if self.peek() == Some(COMMA) {
                self.bump();
            }
        }
        if !self.consume(CLOSE_BRACE) {
            return false;
        }
        self.state.advancements = true;
        // `EntitySelectorOptions.java:450` sits inside the `!expected.isEmpty()`
        // guard at `:432`, so an empty map adds no predicate and leaves the
        // selector reaching entities.
        if any_entry {
            self.state.includes_entities = false;
        }
        self.state.advancements_span = Some((start, self.cursor));
        true
    }

    fn parse_criteria_map(&mut self) -> bool {
        if !self.consume(OPEN_BRACE) {
            return false;
        }
        self.skip_whitespace();
        while self.peek().is_some_and(|next| next != CLOSE_BRACE) {
            self.skip_whitespace();
            self.read_unquoted_string();
            self.skip_whitespace();
            if !self.consume(EQUALS) {
                return false;
            }
            self.skip_whitespace();
            if self.read_boolean().is_none() {
                return false;
            }
            self.skip_whitespace();
            if self.peek() == Some(COMMA) {
                self.bump();
            }
        }
        self.consume(CLOSE_BRACE)
    }

    fn parse_predicate_option(&mut self) -> bool {
        self.read_inversion();
        self.read_identifier().is_some()
    }

    fn read_inversion(&mut self) -> bool {
        self.skip_whitespace();
        if self.consume(BANG) {
            self.skip_whitespace();
            true
        } else {
            false
        }
    }

    fn read_tag_prefix(&mut self) -> bool {
        self.skip_whitespace();
        if self.consume(HASH) {
            self.skip_whitespace();
            true
        } else {
            false
        }
    }

    fn read_double_range(&mut self, reject_swapped: bool) -> Option<NumericRange<f64>> {
        self.read_bounds(
            |raw| raw.parse::<f64>(),
            |min, max| reject_swapped && min.total_cmp(&max).is_gt(),
        )
    }

    fn read_float_range(&mut self) -> Option<NumericRange<f32>> {
        self.read_bounds(|raw| raw.parse::<f32>(), |_, _| false)
    }

    fn read_int_range(&mut self) -> Option<NumericRange<i32>> {
        self.read_bounds(|raw| raw.parse::<i32>(), |min, max| min > max)
    }

    fn read_bounds<T: Copy, E>(
        &mut self,
        parse_number: impl Fn(&str) -> Result<T, E>,
        swapped: impl Fn(T, T) -> bool,
    ) -> Option<NumericRange<T>> {
        let (end, min, max) = super::syntax::parse_command_bounds_utf16(
            self.input,
            self.cursor,
            parse_number,
            swapped,
        )
        .ok()?;
        self.cursor = end;
        Some(NumericRange { min, max })
    }

    fn read_double(&mut self) -> Option<f64> {
        let (end, raw) =
            super::syntax::scan_command_number_lexeme_utf16(self.input, self.cursor).ok()?;
        self.cursor = end;
        ascii_word(raw).parse().ok()
    }

    fn read_int(&mut self) -> Option<i32> {
        let (end, raw) =
            super::syntax::scan_command_number_lexeme_utf16(self.input, self.cursor).ok()?;
        self.cursor = end;
        ascii_word(raw).parse().ok()
    }

    fn read_boolean(&mut self) -> Option<bool> {
        match ascii_word(self.read_string()?.units()).as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    fn read_identifier(&mut self) -> Option<String> {
        let (end, id) =
            super::syntax::scan_command_resource_location_utf16(self.input, self.cursor).ok()?;
        self.cursor = end;
        Some(id)
    }

    fn read_string(&mut self) -> Option<JavaString> {
        let (end, value) =
            super::syntax::read_command_string_value_utf16(self.input, self.cursor).ok()?;
        self.cursor = end;
        Some(value)
    }

    fn read_unquoted_string(&mut self) -> JavaString {
        let end = super::syntax::scan_command_unquoted_string_utf16(self.input, self.cursor);
        let value = self.input[self.cursor..end].to_vec().into();
        self.cursor = end;
        value
    }

    fn skip_whitespace(&mut self) {
        self.cursor = super::syntax::scan_command_whitespace_utf16(self.input, self.cursor);
    }

    fn consume(&mut self, expected: u16) -> bool {
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u16> {
        self.input.get(self.cursor).copied()
    }

    fn bump(&mut self) -> Option<u16> {
        let next = self.peek()?;
        self.cursor += 1;
        Some(next)
    }
}

const AT: u16 = b'@' as u16;
const BANG: u16 = b'!' as u16;
const HASH: u16 = b'#' as u16;
const COMMA: u16 = b',' as u16;
const EQUALS: u16 = b'=' as u16;
const OPEN_BRACKET: u16 = b'[' as u16;
const CLOSE_BRACKET: u16 = b']' as u16;
const OPEN_BRACE: u16 = b'{' as u16;
const CLOSE_BRACE: u16 = b'}' as u16;
const LETTER_A: u16 = b'a' as u16;
const LETTER_E: u16 = b'e' as u16;
const LETTER_N: u16 = b'n' as u16;
const LETTER_P: u16 = b'p' as u16;
const LETTER_R: u16 = b'r' as u16;
const LETTER_S: u16 = b's' as u16;

/// The option names, keywords and numeric runs this parser compares are ASCII,
/// and every other unit -- an unpaired surrogate included -- projects to
/// something no ASCII literal equals, so the comparison stays exact.
fn ascii_word(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

#[derive(Clone, Copy)]
struct NumericRange<T> {
    min: Option<T>,
    max: Option<T>,
}

#[derive(Clone, Copy)]
enum SelectorField {
    X,
    Y,
    Z,
    DeltaX,
    DeltaY,
    DeltaZ,
}

/// `java.util.UUID.fromString`, which `EntitySelectorParser.parseNameOrUUID`
/// (EntitySelectorParser.java:270) tries before it reads the word as a player
/// name.
///
/// Java's parser is looser than the canonical 8-4-4-4-12 spelling: it cuts the
/// string at the four hyphens and reads each group as a positive hexadecimal
/// `long`, so `1-2-3-4-5` parses. The five groups are then packed into the two
/// longs `java.util.UUID` holds, most significant first, which is the value a
/// lookup compares against an online player's id.
///
/// Both the yes/no the selector grammar needs and the value the resolution
/// needs come from this one parse.
pub(super) fn java_uuid_from_string(value: &str) -> Option<u128> {
    if value.len() > 36 {
        return None;
    }
    let mut parts = value.split('-');
    let mut groups = [0u64; 5];
    for group in &mut groups {
        *group = java_positive_hex_long(parts.next()?)?;
    }
    if parts.next().is_some() {
        return None;
    }
    let most =
        ((groups[0] & 0xffff_ffff) << 32) | ((groups[1] & 0xffff) << 16) | (groups[2] & 0xffff);
    let least = ((groups[3] & 0xffff) << 48) | (groups[4] & 0xffff_ffff_ffff);
    Some((u128::from(most) << 64) | u128::from(least))
}

pub(super) fn is_java_uuid_string(value: &str) -> bool {
    java_uuid_from_string(value).is_some()
}

/// One hyphen-separated group as `Long.parseLong(.., 16)` reads it: an optional
/// `+`, at least one hexadecimal digit, and no more than a positive `long`
/// holds.
fn java_positive_hex_long(part: &str) -> Option<u64> {
    let digits = part.strip_prefix('+').unwrap_or(part);
    if digits.is_empty()
        || digits.len() > 16
        || !digits.bytes().all(|byte| byte.is_ascii_hexdigit())
        || (digits.len() == 16 && digits.as_bytes()[0] > b'7')
    {
        return None;
    }
    u64::from_str_radix(digits, 16).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        java_uuid_from_string, parse_entity_selector_prefix, parse_entity_selector_prefix_utf16,
        VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS,
    };

    fn is_valid_entity_selector(value: &str) -> bool {
        parse_entity_selector_prefix(value).is_some_and(|parsed| parsed.consumed == value.len())
    }

    /// `UUID.fromString` packs the five hyphen-separated groups into the two
    /// longs `java.util.UUID` holds, so the lenient spelling names the same
    /// identity the canonical one does.
    #[test]
    fn a_java_uuid_packs_its_five_groups_the_way_the_canonical_spelling_reads() {
        let canonical = "f81d4fae-7dec-11d0-a765-00a0c91e6bf6";
        let expected = uuid::Uuid::parse_str(canonical).unwrap().as_u128();
        assert_eq!(java_uuid_from_string(canonical), Some(expected));
        // Every group is read as a positive hexadecimal `long`, so leading
        // zeroes may be dropped and a `+` is accepted.
        assert_eq!(
            java_uuid_from_string("f81d4fae-7dec-11d0-a765-a0c91e6bf6"),
            java_uuid_from_string("f81d4fae-7dec-11d0-a765-00a0c91e6bf6")
        );
        assert_eq!(
            java_uuid_from_string("1-2-3-4-5"),
            Some((1u128 << 96) | (2u128 << 80) | (3u128 << 64) | (4u128 << 48) | 5u128)
        );
        assert_eq!(
            java_uuid_from_string("+1-2-3-4-5"),
            java_uuid_from_string("1-2-3-4-5")
        );
        // Four groups, six groups, an empty one, a non-hexadecimal digit and a
        // group past the positive `long` range are all the
        // `IllegalArgumentException` arm.
        for refused in [
            "1-2-3-4",
            "1-2-3-4-5-6",
            "1-2--4-5",
            "1-2-3-4-g",
            "8000000000000000-2-3-4-5",
        ] {
            assert_eq!(java_uuid_from_string(refused), None, "{refused}");
        }
    }

    #[test]
    fn accepts_names_selector_types_and_every_registered_option_shape() {
        for selector in [
            "Steve",
            "'Player One'",
            "'😀😀😀😀😀😀😀😀'",
            "1-2-3-4-5",
            "@p",
            "@a[]",
            "@r",
            "@s[tag=local]",
            "@e[ ]",
            "@n[limit=1]",
            "@e[name=!Cow,name=!Pig,distance=1..5,level=0..30,x=1.5,y=-2,z=3,dx=.5,dy=0,dz=-.5,x_rotation=-90..90,y_rotation=..180,limit=2,sort=nearest,gamemode=!creative,team=!red,type=!#minecraft:undead,type=!minecraft:pig,tag=!boss,nbt=!{CustomName:'Pig'},scores={kills=1..,deaths=..5},advancements={minecraft:story/root=true,minecraft:adventure/root={criterion=false}},predicate=!minecraft:test]",
            "@e[tag=,predicate=]",
            "@e[type=#]",
            "@e[\"type\"=pig]",
            "@e[team=!red,team=blue]",
            "@e[scores={a=1 b=2,},advancements={minecraft:test={a=true b=false,},}]",
            "@e[advancements={minecraft:test='true'}]",
            "@e[x_rotation=1...2]",
        ] {
            assert!(is_valid_entity_selector(selector), "rejected {selector:?}");
        }
    }

    #[test]
    fn completion_option_catalog_is_unique_and_accepted_by_the_selector_owner() {
        let names = VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS
            .iter()
            .map(|option| option.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(names.len(), VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS.len());
        for option in VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS {
            let value = match option.name {
                "name" => "Pig",
                "distance" => "1..2",
                "level" => "1..2",
                "x" | "y" | "z" | "dx" | "dy" | "dz" => "1",
                "x_rotation" | "y_rotation" => "-90..90",
                "limit" => "1",
                "sort" => "nearest",
                "gamemode" => "creative",
                "team" => "red",
                "type" => "minecraft:pig",
                "tag" => "boss",
                "nbt" => "{}",
                "scores" => "{kills=1}",
                "advancements" => "{minecraft:story/root=true}",
                "predicate" => "minecraft:test",
                unknown => panic!("missing selector option fixture for {unknown}"),
            };
            let selector = format!("@e[{}={value}]", option.name);
            assert!(is_valid_entity_selector(&selector), "rejected {selector:?}");
        }
    }

    #[test]
    fn rejects_unknown_inapplicable_or_malformed_selector_options() {
        for selector in [
            "",
            "12345678901234567",
            "'😀😀😀😀😀😀😀😀😀'",
            "@",
            "@x",
            "@e trailing",
            "@e[unknown=1]",
            "@e[type]",
            "@e[type=pig",
            "@e[type=missing]",
            "@a[type=player]",
            "@e[type=!pig,type=cow]",
            "@e[name=Pig,name=!Cow]",
            "@e[name=!Pig,name=Cow]",
            "@e[distance=-1]",
            "@e[distance=2..1]",
            "@e[distance=0..-0]",
            "@e[distance=1,distance=2]",
            "@e[level=-1]",
            "@e[level=2..1]",
            "@e[x=1e2]",
            "@s[limit=1]",
            "@s[sort=nearest]",
            "@e[limit=0]",
            "@e[sort=closest]",
            "@e[gamemode=builder]",
            "@e[gamemode=!creative,gamemode=survival]",
            "@e[nbt={broken=1}]",
            "@e[scores={kills=..}]",
            "@e[advancements={minecraft:test=TRUE}]",
            "@e[predicate=Bad]",
            "@e[,]",
        ] {
            assert!(!is_valid_entity_selector(selector), "accepted {selector:?}");
        }
    }

    #[test]
    fn the_four_options_that_narrow_to_players_clear_includes_entities() {
        // `EntitySelectorOptions` calls `setIncludesEntities(false)` from
        // exactly four places: `level` (:123), `gamemode` (:235), `type` when
        // it names the uninverted player type (:296) and `advancements` (:450).
        for selector in [
            "@e[level=1..2]",
            "@e[gamemode=creative]",
            "@e[gamemode=!creative]",
            "@e[type=minecraft:player]",
            "@e[advancements={minecraft:story/root=true}]",
        ] {
            let prefix = parse_entity_selector_prefix(selector).unwrap();
            assert!(
                !prefix.includes_entities,
                "{selector} still includes entities"
            );
        }
        // An empty advancements map adds no predicate, so `:450` -- which sits
        // inside the `!expected.isEmpty()` guard at `:432` -- never runs.
        assert!(
            parse_entity_selector_prefix("@e[advancements={}]")
                .unwrap()
                .includes_entities
        );
        // `scores` is not one of them, an inverted `type` is not, and neither
        // is a type that is not the player.
        for selector in [
            "@e[scores={kills=1}]",
            "@e[type=!minecraft:player]",
            "@e[type=minecraft:pig]",
        ] {
            let prefix = parse_entity_selector_prefix(selector).unwrap();
            assert!(
                prefix.includes_entities,
                "{selector} stopped including entities"
            );
        }
    }

    #[test]
    fn the_advancements_span_covers_its_payload_in_any_option_list() {
        for selector in [
            "@a[advancements={minecraft:story/root=true}]",
            // `@a` already carries the player type limit, so the combined
            // case has to come off `@e`.
            "@e[type=minecraft:player,advancements={minecraft:story/root=true},limit=3]",
            "@a[limit=3,advancements={minecraft:story/root=true},sort=nearest]",
            "@a[advancements={minecraft:story/root={crafting_table=false}}]",
        ] {
            let prefix = parse_entity_selector_prefix(selector).unwrap();
            let (start, end) = prefix.advancements_span.expect("the option was accepted");
            let payload = &selector[start..end];
            assert!(
                payload.starts_with('{') && payload.ends_with('}'),
                "{payload}"
            );
            assert!(payload.contains("minecraft:story/root"), "{payload}");
        }
        // A list without the option carries no span.
        assert!(parse_entity_selector_prefix("@a[limit=3]")
            .unwrap()
            .advancements_span
            .is_none());
    }

    /// `EntitySelectorParser` reads the buffer with `StringReader`, so a name
    /// or a condition value made of units no `String` can hold is read unit by
    /// unit. Every count is the pinned client-26.1 jar's `EntityArgument`
    /// cursor for the same buffer.
    #[test]
    fn selector_conditions_are_read_in_the_buffers_own_units() {
        fn units(prefix: &str, unit: u16, suffix: &str) -> Vec<u16> {
            prefix
                .encode_utf16()
                .chain([unit])
                .chain(suffix.encode_utf16())
                .collect()
        }

        for (prefix, unit, suffix, consumed) in [
            ("@e[name='a", 0xd800, "']", 13),
            ("@e[name='a", 0xd800, "'] hi", 13),
            ("@p[name='a", 0xd800, "',limit=1]", 21),
            ("@a[name=!'a", 0xd800, "',limit=2]", 22),
            ("@e[nbt={a:'x", 0xd800, "'}] x", 16),
            // The unquoted charset is ASCII, so the surrogate ends the name and
            // the option list is then unterminated.
            ("@e[tag='a", 0xd800, "']", 0),
            // A bare name stops at the first unit the unquoted charset rejects.
            ("a", 0xd800, "", 1),
            ("", 0xd800, "", 0),
        ] {
            let buffer = units(prefix, unit, suffix);
            let parsed = parse_entity_selector_prefix_utf16(&buffer);
            assert_eq!(
                parsed.map(|parsed| parsed.consumed),
                (consumed != 0).then_some(consumed),
                "{prefix:?}+{unit:04x}+{suffix:?}"
            );
        }

        // A name outside ASCII needs the quotes the unquoted charset denies.
        for (selector, consumed) in [
            ("@e[name='\u{65e5}\u{672c}\u{8a9e}']", Some(14)),
            ("@e[scores={a=1},name='\u{65e5}\u{672c}\u{8a9e}']", Some(27)),
            ("@e[name='\u{1f600}']", Some(13)),
            ("@e[limit=1,name='\u{1f600}x']", Some(22)),
            ("@e[name=\u{65e5}\u{672c}\u{8a9e}]", None),
            ("\u{65e5}\u{672c}\u{8a9e}", None),
        ] {
            let buffer: Vec<u16> = selector.encode_utf16().collect();
            assert_eq!(
                parse_entity_selector_prefix_utf16(&buffer).map(|parsed| parsed.consumed),
                consumed,
                "{selector}"
            );
        }

        // The `&str` entry answers the same parse in bytes.
        let selector = "@e[name='\u{65e5}\u{672c}\u{8a9e}']";
        assert_eq!(
            parse_entity_selector_prefix(selector).map(|parsed| parsed.consumed),
            Some(selector.len())
        );

        // A compound key is the value side's string alternative
        // (`SnbtGrammar.java:581-585`), so it holds any unit: the jar reads
        // `@e[nbt={'a<D800>':1}]` to cursor 16, the whole selector.
        let key = units("@e[nbt={'a", 0xd800, "':1}]");
        assert_eq!(
            parse_entity_selector_prefix_utf16(&key).map(|parsed| parsed.consumed),
            Some(16)
        );
    }

    #[test]
    fn reports_prefix_and_entity_argument_constraints() {
        let all = parse_entity_selector_prefix("@e[type=minecraft:pig] trailing").unwrap();
        assert_eq!(all.consumed, "@e[type=minecraft:pig]".len());
        assert!(all.multiple);
        assert!(all.includes_entities);
        assert!(!all.self_selector);

        let nearest_player = parse_entity_selector_prefix("@p").unwrap();
        assert!(!nearest_player.multiple);
        assert!(!nearest_player.includes_entities);

        let current = parse_entity_selector_prefix("@s").unwrap();
        assert!(current.includes_entities);
        assert!(current.self_selector);

        let limited = parse_entity_selector_prefix("@e[limit=1]").unwrap();
        assert!(!limited.multiple);
        assert!(limited.includes_entities);
    }
}
