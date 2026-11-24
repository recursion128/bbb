//! `CraftingInput` shrink-to-positioned (`CraftingInput.java:32-84`).

use bbb_protocol::packets::ItemStackSummary;

use super::ingredient::stack_is_empty;

/// Positioned crafting grid after empty-border shrink.
#[derive(Debug, Clone, PartialEq)]
pub struct CraftingInput {
    width: i32,
    height: i32,
    items: Vec<ItemStackSummary>,
    ingredient_count: i32,
}

/// `CraftingInput.Positioned` (`CraftingInput.java:146-148`).
#[derive(Debug, Clone, PartialEq)]
pub struct CraftingInputPositioned {
    pub input: CraftingInput,
    pub left: i32,
    pub top: i32,
}

impl CraftingInput {
    pub fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            items: Vec::new(),
            ingredient_count: 0,
        }
    }

    /// `CraftingInput.of` (`CraftingInput.java:32-34`).
    pub fn of(width: i32, height: i32, items: Vec<ItemStackSummary>) -> Self {
        Self::of_positioned(width, height, items).input
    }

    /// `CraftingInput.ofPositioned` (`CraftingInput.java:36-84`).
    pub fn of_positioned(
        width: i32,
        height: i32,
        items: Vec<ItemStackSummary>,
    ) -> CraftingInputPositioned {
        if width <= 0 || height <= 0 {
            return CraftingInputPositioned::empty();
        }
        let mut left = width - 1;
        let mut right = 0;
        let mut top = height - 1;
        let mut bottom = 0;
        for y in 0..height {
            let mut row_empty = true;
            for x in 0..width {
                let index = usize::try_from(x + y * width).expect("crafting input index");
                if !stack_is_empty(&items[index]) {
                    left = left.min(x);
                    right = right.max(x);
                    row_empty = false;
                }
            }
            if !row_empty {
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
        let new_width = right - left + 1;
        let new_height = bottom - top + 1;
        if new_width <= 0 || new_height <= 0 {
            return CraftingInputPositioned::empty();
        }
        if new_width == width && new_height == height {
            return CraftingInputPositioned {
                input: Self::from_cells(width, height, items),
                left,
                top,
            };
        }
        let mut shrunk = Vec::with_capacity((new_width * new_height) as usize);
        for y in 0..new_height {
            for x in 0..new_width {
                let index = usize::try_from(x + left + (y + top) * width).expect("shrink index");
                shrunk.push(items[index].clone());
            }
        }
        CraftingInputPositioned {
            input: Self::from_cells(new_width, new_height, shrunk),
            left,
            top,
        }
    }

    fn from_cells(width: i32, height: i32, items: Vec<ItemStackSummary>) -> Self {
        let ingredient_count = items.iter().filter(|item| !stack_is_empty(item)).count() as i32;
        Self {
            width,
            height,
            items,
            ingredient_count,
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn size(&self) -> i32 {
        i32::try_from(self.items.len()).unwrap_or(0)
    }

    pub fn ingredient_count(&self) -> i32 {
        self.ingredient_count
    }

    pub fn is_empty(&self) -> bool {
        self.ingredient_count == 0
    }

    pub fn get_item(&self, index: i32) -> &ItemStackSummary {
        &self.items[usize::try_from(index).expect("item index")]
    }

    pub fn get_item_at(&self, x: i32, y: i32) -> &ItemStackSummary {
        self.get_item(x + y * self.width)
    }

    pub fn items(&self) -> &[ItemStackSummary] {
        &self.items
    }

    /// Occupied cells, each counted as one (`CraftingInput` accounts with count 1).
    pub fn occupied(&self) -> Vec<&ItemStackSummary> {
        self.items
            .iter()
            .filter(|item| !stack_is_empty(item))
            .collect()
    }
}

impl CraftingInputPositioned {
    pub fn empty() -> Self {
        Self {
            input: CraftingInput::empty(),
            left: 0,
            top: 0,
        }
    }
}
