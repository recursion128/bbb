//! `ShapedRecipe` / `ShapedRecipePattern` (`ShapedRecipe.java`, `ShapedRecipePattern.java`).

use super::ingredient::{test_optional_ingredient, Ingredient, ItemStackTemplate, ItemTagIndex};
use super::input::CraftingInput;
use super::RecipeDecodeError;
use bbb_protocol::JavaString;
use bbb_registry_codec::RegistryElementValueRef;
use bbb_registry_codec::{
    registry_codec::RegistryReferenceLookup, PersistentItemStackHolderResolver,
};

fn decode_error(message: impl Into<String>) -> RecipeDecodeError {
    RecipeDecodeError {
        message: message.into(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapedRecipe {
    pub pattern: ShapedRecipePattern,
    pub result: ItemStackTemplate,
}

impl ShapedRecipe {
    pub(super) fn parse_source(
        root: RegistryElementValueRef<'_>,
        lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
    ) -> Result<Self, RecipeDecodeError> {
        Ok(Self {
            pattern: ShapedRecipePattern::parse_source_with_content_registry(
                root,
                lookup.content_registry(),
            )?,
            result: super::source::template(super::source::required(root, "result")?, lookup)?,
        })
    }

    /// `ShapedRecipe.matches` (`ShapedRecipe.java:66-68`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        self.pattern.matches(input, tags)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapedRecipePattern {
    pub width: i32,
    pub height: i32,
    pub ingredients: Vec<Option<Ingredient>>,
    ingredient_count: i32,
    symmetrical: bool,
}

impl ShapedRecipePattern {
    pub(super) fn map_extending(material: Ingredient, filled: Ingredient) -> Self {
        let key = [(b'#' as u16, material), (b'x' as u16, filled)]
            .into_iter()
            .collect();
        let rows = ["###", "#x#", "###"].map(JavaString::from);
        Self::unpack_units(key, &rows).expect("fixed map extending pattern is valid")
    }
    pub(super) fn parse_source_with_content_registry(
        root: RegistryElementValueRef<'_>,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Result<Self, RecipeDecodeError> {
        use super::source::{error, list, required, string};
        let pattern = list(required(root, "pattern")?, "pattern")?;
        let rows = pattern
            .map(|row| string(row, "pattern row"))
            .collect::<Result<Vec<_>, _>>()?;
        let key_map = required(root, "key")?;
        let len = key_map
            .map_len()
            .ok_or_else(|| error("`key` must be an object"))?;
        let mut key = std::collections::BTreeMap::new();
        for index in 0..len {
            let (symbol, value) = key_map
                .map_entry_java_string_ref(index)
                .expect("map length and indexed entries agree");
            let [unit] = symbol.units() else {
                return Err(decode_error(format!(
                    "invalid key entry: {symbol:?} is an invalid symbol"
                )));
            };
            if *unit == 0x20 {
                return Err(decode_error("invalid key entry: ' ' is a reserved symbol"));
            }
            key.insert(
                *unit,
                Ingredient::parse_source_with_content_registry(value, content_registry)?,
            );
        }
        Self::unpack_units(key, &rows)
    }

    fn unpack_units(
        key: std::collections::BTreeMap<u16, Ingredient>,
        pattern: &[JavaString],
    ) -> Result<Self, RecipeDecodeError> {
        if pattern.is_empty() {
            return Err(decode_error("invalid pattern: empty pattern not allowed"));
        }
        if pattern.len() > 3 {
            return Err(decode_error("invalid pattern: too many rows, 3 is maximum"));
        }
        let source_width = pattern[0].units().len();
        if source_width == 0 || source_width > 3 {
            return Err(decode_error(
                "invalid pattern: each row must be width 1..=3",
            ));
        }
        if pattern.iter().any(|row| row.units().len() != source_width) {
            return Err(decode_error(
                "invalid pattern: each row must be the same width",
            ));
        }
        let shrunk = shrink(pattern);
        if shrunk.is_empty() {
            return Err(decode_error("invalid pattern: empty after shrink"));
        }
        let width = i32::try_from(shrunk[0].len()).expect("pattern width");
        let height = i32::try_from(shrunk.len()).expect("pattern height");
        let mut ingredients = Vec::with_capacity((width * height) as usize);
        let mut unused: std::collections::BTreeSet<u16> = key.keys().copied().collect();
        for line in &shrunk {
            for &ch in line {
                if ch == 0x20 {
                    ingredients.push(None);
                    continue;
                }
                let Some(ingredient) = key.get(&ch) else {
                    return Err(decode_error(format!(
                        "pattern references symbol {ch:#06x} but it's not defined in the key"
                    )));
                };
                unused.remove(&ch);
                ingredients.push(Some(ingredient.clone()));
            }
        }
        if !unused.is_empty() {
            return Err(decode_error(format!(
                "key defines symbols that aren't used in pattern: {unused:?}"
            )));
        }
        let ingredient_count = ingredients.iter().filter(|slot| slot.is_some()).count() as i32;
        let symmetrical = is_symmetrical(width, height, &ingredients);
        Ok(Self {
            width,
            height,
            ingredients,
            ingredient_count,
            symmetrical,
        })
    }

    /// `ShapedRecipePattern.matches` (`ShapedRecipePattern.java:155-171`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        if input.ingredient_count() != self.ingredient_count {
            return false;
        }
        if input.width() != self.width || input.height() != self.height {
            return false;
        }
        if !self.symmetrical && self.matches_oriented(input, true, tags) {
            return true;
        }
        self.matches_oriented(input, false, tags)
    }

    fn matches_oriented(&self, input: &CraftingInput, x_flip: bool, tags: &ItemTagIndex) -> bool {
        for y in 0..self.height {
            for x in 0..self.width {
                let expected = if x_flip {
                    self.ingredients[(self.width - x - 1 + y * self.width) as usize].as_ref()
                } else {
                    self.ingredients[(x + y * self.width) as usize].as_ref()
                };
                if !test_optional_ingredient(expected, input.get_item_at(x, y), tags) {
                    return false;
                }
            }
        }
        true
    }
}

/// `ShapedRecipePattern.shrink` (`ShapedRecipePattern.java:100-133`).
fn shrink(pattern: &[JavaString]) -> Vec<Vec<u16>> {
    let mut left = usize::MAX;
    let mut right = 0usize;
    let mut top = 0usize;
    let mut bottom = 0usize;
    for (i, line) in pattern.iter().enumerate() {
        left = left.min(first_non_empty(line));
        let last = last_non_empty(line);
        right = right.max(last.unwrap_or(0));
        if last.is_none() {
            if top == i {
                top += 1;
            }
            bottom += 1;
        } else {
            bottom = 0;
        }
    }
    if pattern.len() == bottom {
        return Vec::new();
    }
    pattern[top..pattern.len() - bottom]
        .iter()
        .map(|line| line.units()[left..=right].to_vec())
        .collect()
}

fn first_non_empty(line: &JavaString) -> usize {
    line.units()
        .iter()
        .position(|&ch| ch != 0x20)
        .unwrap_or(line.units().len())
}

fn last_non_empty(line: &JavaString) -> Option<usize> {
    line.units().iter().rposition(|&ch| ch != 0x20)
}

/// `Util.isSymmetrical` (`Util.java:463-482`).
fn is_symmetrical(width: i32, height: i32, ingredients: &[Option<Ingredient>]) -> bool {
    if width == 1 {
        return true;
    }
    let center_x = width / 2;
    for y in 0..height {
        for left_x in 0..center_x {
            let right_x = width - 1 - left_x;
            let left = &ingredients[(left_x + y * width) as usize];
            let right = &ingredients[(right_x + y * width) as usize];
            if left != right {
                return false;
            }
        }
    }
    true
}
