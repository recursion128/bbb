use std::cmp::Ordering;

use crate::{
    entity_types::vanilla_entity_type_for_resource_id, nbt_summary::validate_snbt_compound_prefix,
    packets::normalize_resource_location,
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
}

pub(super) fn parse_entity_selector_prefix(value: &str) -> Option<EntitySelectorPrefix> {
    SelectorParser::new(value).parse()
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
}

struct SelectorParser<'a> {
    input: &'a str,
    cursor: usize,
    state: SelectorState,
}

impl<'a> SelectorParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            cursor: 0,
            state: SelectorState::default(),
        }
    }

    fn parse(mut self) -> Option<EntitySelectorPrefix> {
        let valid = if self.peek() == Some('@') {
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
        })
    }

    fn parse_selector(&mut self) -> bool {
        match self.bump() {
            Some('a') => {
                self.state.multiple = true;
                self.state.type_limited = true;
            }
            Some('p' | 'r') => self.state.type_limited = true,
            Some('s') => {
                self.state.includes_entities = true;
                self.state.current_entity = true;
            }
            Some('e') => {
                self.state.multiple = true;
                self.state.includes_entities = true;
            }
            Some('n') => self.state.includes_entities = true,
            _ => return false,
        }
        self.peek() != Some('[') || self.parse_options()
    }

    fn parse_name_or_uuid(&mut self) -> bool {
        let Some(name) = self.read_string() else {
            return false;
        };
        if is_java_uuid_string(&name) {
            self.state.includes_entities = true;
            true
        } else {
            !name.is_empty() && name.encode_utf16().count() <= 16
        }
    }

    fn parse_options(&mut self) -> bool {
        self.consume('[');
        self.skip_whitespace();
        while self.peek().is_some_and(|next| next != ']') {
            self.skip_whitespace();
            let Some(key) = self.read_string() else {
                return false;
            };
            self.skip_whitespace();
            if !self.consume('=') {
                return false;
            }
            self.skip_whitespace();
            if !self.parse_option(&key) {
                return false;
            }
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => break,
                _ => return false,
            }
        }
        self.consume(']')
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
            self.read_unquoted_string().as_str(),
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
            self.read_unquoted_string().as_str(),
            "survival" | "creative" | "adventure" | "spectator"
        ) {
            return false;
        }
        if inverted {
            self.state.gamemode_not_equals = true;
        } else {
            self.state.gamemode_equals = true;
        }
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
        if !tagged && vanilla_entity_type_for_resource_id(&resource_id).is_none() {
            return false;
        }
        if !tagged && !inverted {
            self.state.type_limited = true;
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
        let Ok(consumed) = validate_snbt_compound_prefix(&self.input[self.cursor..]) else {
            return false;
        };
        self.cursor += consumed;
        true
    }

    fn parse_scores_option(&mut self) -> bool {
        if self.state.scores || !self.consume('{') {
            return false;
        }
        self.skip_whitespace();
        while self.peek().is_some_and(|next| next != '}') {
            self.skip_whitespace();
            self.read_unquoted_string();
            self.skip_whitespace();
            if !self.consume('=') {
                return false;
            }
            self.skip_whitespace();
            if self.read_int_range().is_none() {
                return false;
            }
            self.skip_whitespace();
            if self.peek() == Some(',') {
                self.bump();
            }
        }
        if !self.consume('}') {
            return false;
        }
        self.state.scores = true;
        true
    }

    fn parse_advancements_option(&mut self) -> bool {
        if self.state.advancements || !self.consume('{') {
            return false;
        }
        self.skip_whitespace();
        while self.peek().is_some_and(|next| next != '}') {
            self.skip_whitespace();
            if self.read_identifier().is_none() {
                return false;
            }
            self.skip_whitespace();
            if !self.consume('=') {
                return false;
            }
            self.skip_whitespace();
            if self.peek() == Some('{') {
                if !self.parse_criteria_map() {
                    return false;
                }
                self.skip_whitespace();
            } else if self.read_boolean().is_none() {
                return false;
            }
            self.skip_whitespace();
            if self.peek() == Some(',') {
                self.bump();
            }
        }
        if !self.consume('}') {
            return false;
        }
        self.state.advancements = true;
        true
    }

    fn parse_criteria_map(&mut self) -> bool {
        if !self.consume('{') {
            return false;
        }
        self.skip_whitespace();
        while self.peek().is_some_and(|next| next != '}') {
            self.skip_whitespace();
            self.read_unquoted_string();
            self.skip_whitespace();
            if !self.consume('=') {
                return false;
            }
            self.skip_whitespace();
            if self.read_boolean().is_none() {
                return false;
            }
            self.skip_whitespace();
            if self.peek() == Some(',') {
                self.bump();
            }
        }
        self.consume('}')
    }

    fn parse_predicate_option(&mut self) -> bool {
        self.read_inversion();
        self.read_identifier().is_some()
    }

    fn read_inversion(&mut self) -> bool {
        self.skip_whitespace();
        if self.consume('!') {
            self.skip_whitespace();
            true
        } else {
            false
        }
    }

    fn read_tag_prefix(&mut self) -> bool {
        self.skip_whitespace();
        if self.consume('#') {
            self.skip_whitespace();
            true
        } else {
            false
        }
    }

    fn read_double_range(&mut self, reject_swapped: bool) -> Option<NumericRange<f64>> {
        let min = self.read_optional_range_number(|raw| raw.parse::<f64>().ok())?;
        let max = if self.remaining().starts_with("..") {
            self.cursor += 2;
            self.read_optional_range_number(|raw| raw.parse::<f64>().ok())?
        } else {
            min
        };
        if min.is_none() && max.is_none() {
            return None;
        }
        if reject_swapped
            && matches!((min, max), (Some(min), Some(max)) if min.total_cmp(&max) == Ordering::Greater)
        {
            return None;
        }
        Some(NumericRange { min, max })
    }

    fn read_float_range(&mut self) -> Option<NumericRange<f32>> {
        let min = self.read_optional_range_number(|raw| raw.parse::<f32>().ok())?;
        let max = if self.remaining().starts_with("..") {
            self.cursor += 2;
            self.read_optional_range_number(|raw| raw.parse::<f32>().ok())?
        } else {
            min
        };
        (min.is_some() || max.is_some()).then_some(NumericRange { min, max })
    }

    fn read_int_range(&mut self) -> Option<NumericRange<i32>> {
        let min = self.read_optional_range_number(|raw| raw.parse::<i32>().ok())?;
        let max = if self.remaining().starts_with("..") {
            self.cursor += 2;
            self.read_optional_range_number(|raw| raw.parse::<i32>().ok())?
        } else {
            min
        };
        if min.is_none() && max.is_none() {
            return None;
        }
        if matches!((min, max), (Some(min), Some(max)) if min > max) {
            return None;
        }
        Some(NumericRange { min, max })
    }

    fn read_optional_range_number<T>(
        &mut self,
        parse: impl FnOnce(&str) -> Option<T>,
    ) -> Option<Option<T>> {
        let start = self.cursor;
        while let Some(next) = self.peek() {
            if next.is_ascii_digit() || next == '-' {
                self.bump();
            } else if next == '.' && !self.remaining().starts_with("..") {
                self.bump();
            } else {
                break;
            }
        }
        if self.cursor == start {
            Some(None)
        } else {
            parse(&self.input[start..self.cursor]).map(Some)
        }
    }

    fn read_double(&mut self) -> Option<f64> {
        let start = self.cursor;
        self.read_brigadier_number();
        self.input[start..self.cursor].parse().ok()
    }

    fn read_int(&mut self) -> Option<i32> {
        let start = self.cursor;
        self.read_brigadier_number();
        self.input[start..self.cursor].parse().ok()
    }

    fn read_brigadier_number(&mut self) {
        while self
            .peek()
            .is_some_and(|next| next.is_ascii_digit() || matches!(next, '.' | '-'))
        {
            self.bump();
        }
    }

    fn read_boolean(&mut self) -> Option<bool> {
        match self.read_string()?.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    fn read_identifier(&mut self) -> Option<String> {
        let start = self.cursor;
        while self.peek().is_some_and(is_identifier_character) {
            self.bump();
        }
        normalize_resource_location(self.input[start..self.cursor].to_string()).ok()
    }

    fn read_string(&mut self) -> Option<String> {
        match self.peek() {
            Some(quote @ ('\'' | '"')) => {
                self.bump();
                let mut output = String::new();
                loop {
                    match self.bump()? {
                        next if next == quote => return Some(output),
                        '\\' => {
                            let escaped = self.bump()?;
                            if escaped != quote && escaped != '\\' {
                                return None;
                            }
                            output.push(escaped);
                        }
                        next => output.push(next),
                    }
                }
            }
            _ => Some(self.read_unquoted_string()),
        }
    }

    fn read_unquoted_string(&mut self) -> String {
        let start = self.cursor;
        while self.peek().is_some_and(is_unquoted_string_character) {
            self.bump();
        }
        self.input[start..self.cursor].to_string()
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(is_java_whitespace) {
            self.bump();
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.cursor..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let next = self.peek()?;
        self.cursor += next.len_utf8();
        Some(next)
    }
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

fn is_unquoted_string_character(next: char) -> bool {
    next.is_ascii_alphanumeric() || matches!(next, '_' | '-' | '.' | '+')
}

fn is_identifier_character(next: char) -> bool {
    next.is_ascii_digit()
        || next.is_ascii_lowercase()
        || matches!(next, '_' | ':' | '/' | '.' | '-')
}

fn is_java_whitespace(next: char) -> bool {
    matches!(
        next,
        '\u{0009}'..='\u{000d}'
            | '\u{001c}'..='\u{0020}'
            | '\u{1680}'
            | '\u{2000}'..='\u{2006}'
            | '\u{2008}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{205f}'
            | '\u{3000}'
    )
}

pub(super) fn is_java_uuid_string(value: &str) -> bool {
    if value.len() > 36 {
        return false;
    }
    let mut parts = value.split('-');
    (0..5).all(|_| parts.next().is_some_and(is_java_positive_hex_long)) && parts.next().is_none()
}

fn is_java_positive_hex_long(part: &str) -> bool {
    let digits = part.strip_prefix('+').unwrap_or(part);
    !digits.is_empty()
        && digits.len() <= 16
        && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
        && (digits.len() < 16 || digits.as_bytes()[0] <= b'7')
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{parse_entity_selector_prefix, VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS};

    fn is_valid_entity_selector(value: &str) -> bool {
        parse_entity_selector_prefix(value).is_some_and(|parsed| parsed.consumed == value.len())
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
