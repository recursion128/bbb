//! Screen-local state and pure presentation inputs for `EnchantmentScreen`.
//!
//! The synchronized container, player experience/abilities, and enchantment
//! registry remain canonical in `bbb-world`. Vanilla keeps the GUI book cursor
//! and its process-local `RandomSource` on the screen object, so this module
//! keeps the equivalent transient state on `ClientInputState` and publishes
//! only interpolated values to the renderer.

use bbb_protocol::{menu_types::VANILLA_MENU_TYPE_ENCHANTMENT_ID, packets::ItemStackSummary};
use bbb_renderer::HudFontGlyphBook;
use bbb_world::{LevelEventSoundRandomState, WorldStore};

use super::ClientInputState;

pub(crate) const ENCHANTMENT_OPTION_COUNT: usize = 3;
pub(crate) const ENCHANTMENT_SEED_DATA_ID: i16 = 3;
pub(crate) const ENCHANTMENT_CLUE_DATA_ID: i16 = 4;
pub(crate) const ENCHANTMENT_LEVEL_CLUE_DATA_ID: i16 = 7;
pub(crate) const ENCHANTMENT_ALT_FONT_ID: &str = "minecraft:alt";

const SCREEN_RANDOM_SEED: i64 = 0x454e_4348_414e_54;
pub(crate) const ENCHANTING_TABLE_OPTION_X: i32 = 60;
pub(crate) const ENCHANTING_TABLE_OPTION_Y: i32 = 14;
pub(crate) const ENCHANTING_TABLE_OPTION_WIDTH: u32 = 108;
pub(crate) const ENCHANTING_TABLE_OPTION_HEIGHT: u32 = 19;
pub(crate) const ENCHANTING_TABLE_OPTION_TOOLTIP_HEIGHT: u32 = 17;
pub(crate) const ENCHANTING_TABLE_OPTION_SPACING: i32 = 19;
pub(crate) const ENCHANTING_TABLE_OPTION_TEXT_WIDTH: u32 = 86;

const ENCHANTMENT_WORDS: [&str; 62] = [
    "the",
    "elder",
    "scrolls",
    "klaatu",
    "berata",
    "niktu",
    "xyzzy",
    "bless",
    "curse",
    "light",
    "darkness",
    "fire",
    "air",
    "earth",
    "water",
    "hot",
    "dry",
    "cold",
    "wet",
    "ignite",
    "snuff",
    "embiggen",
    "twist",
    "shorten",
    "stretch",
    "fiddle",
    "destroy",
    "imbue",
    "galvanize",
    "enchant",
    "free",
    "limited",
    "range",
    "of",
    "towards",
    "inside",
    "sphere",
    "cube",
    "self",
    "other",
    "ball",
    "mental",
    "physical",
    "grow",
    "shrink",
    "demon",
    "elemental",
    "spirit",
    "animal",
    "creature",
    "beast",
    "humanoid",
    "undead",
    "fresh",
    "stale",
    "phnglui",
    "mglwnafh",
    "cthulhu",
    "rlyeh",
    "wgahnagl",
    "fhtagn",
    "baguette",
];

#[derive(Debug, Clone)]
pub(crate) struct EnchantmentScreenState {
    container_id: i32,
    random: LevelEventSoundRandomState,
    last_input: ItemStackSummary,
    flip: f32,
    old_flip: f32,
    flip_target: f32,
    flip_acceleration: f32,
    open: f32,
    old_open: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EnchantmentBookHudState {
    pub(crate) open: f32,
    pub(crate) flip: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EnchantmentOptionHudState {
    pub(crate) index: usize,
    pub(crate) cost: i16,
    pub(crate) enabled: bool,
    pub(crate) highlighted: bool,
    pub(crate) tooltip_hovered: bool,
    pub(crate) name: String,
    pub(crate) name_max_width: u32,
    pub(crate) clue_holder_id: Option<i32>,
    pub(crate) clue_level: Option<i32>,
}

impl EnchantmentScreenState {
    fn new(container_id: i32) -> Self {
        Self {
            container_id,
            random: LevelEventSoundRandomState::with_seed(SCREEN_RANDOM_SEED),
            last_input: ItemStackSummary::empty(),
            flip: 0.0,
            old_flip: 0.0,
            flip_target: 0.0,
            flip_acceleration: 0.0,
            open: 0.0,
            old_open: 0.0,
        }
    }

    fn tick(&mut self, current_input: &ItemStackSummary, should_open: bool) {
        // Vanilla `EnchantmentScreen.tickBook`.
        if current_input != &self.last_input {
            self.last_input.clone_from(current_input);
            loop {
                self.flip_target +=
                    (self.random.next_int_bound(4) - self.random.next_int_bound(4)) as f32;
                if !(self.flip <= self.flip_target + 1.0 && self.flip >= self.flip_target - 1.0) {
                    break;
                }
            }
        }

        self.old_flip = self.flip;
        self.old_open = self.open;
        self.open += if should_open { 0.2 } else { -0.2 };
        self.open = self.open.clamp(0.0, 1.0);
        let diff = ((self.flip_target - self.flip) * 0.4).clamp(-0.2, 0.2);
        self.flip_acceleration += (diff - self.flip_acceleration) * 0.9;
        self.flip += self.flip_acceleration;
    }

    fn projection(&self, partial_tick: f32) -> EnchantmentBookHudState {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        EnchantmentBookHudState {
            open: lerp(partial_tick, self.old_open, self.open),
            flip: lerp(partial_tick, self.old_flip, self.flip),
        }
    }
}

impl ClientInputState {
    /// Synchronizes the active enchantment screen and advances its local book
    /// animation once per raw client tick, matching `Screen.containerTick`.
    pub(crate) fn advance_enchantment_screen(&mut self, world: &WorldStore, client_ticks: u32) {
        let Some((container_id, input, should_open)) = enchantment_screen_tick_source(world) else {
            self.enchantment_screen = None;
            return;
        };
        if self
            .enchantment_screen
            .as_ref()
            .is_none_or(|state| state.container_id != container_id)
        {
            self.enchantment_screen = Some(EnchantmentScreenState::new(container_id));
        }
        let state = self
            .enchantment_screen
            .as_mut()
            .expect("enchantment screen initialized above");
        for _ in 0..client_ticks {
            state.tick(&input, should_open);
        }
    }

    pub(crate) fn enchantment_book_hud_state(
        &self,
        partial_tick: f32,
    ) -> Option<EnchantmentBookHudState> {
        self.enchantment_screen
            .as_ref()
            .map(|state| state.projection(partial_tick))
    }
}

/// Projects the three screen rows from synchronized menu/player state and the
/// exact default/alt font advances. Name generation resets to property 3 once,
/// then consumes rows in Vanilla order.
pub(crate) fn enchantment_option_hud_states(
    world: &WorldStore,
    cursor_position: Option<(i32, i32)>,
    fonts: &HudFontGlyphBook,
) -> Option<[EnchantmentOptionHudState; ENCHANTMENT_OPTION_COUNT]> {
    let container = world.inventory().open_container.as_ref()?;
    (container.menu_type_id == Some(VANILLA_MENU_TYPE_ENCHANTMENT_ID)).then_some(())?;

    let seed = i64::from(
        world
            .open_container_data_value(ENCHANTMENT_SEED_DATA_ID)
            .unwrap_or_default(),
    );
    let mut random = LevelEventSoundRandomState::with_seed(seed);
    let default_font = fonts.glyphs(None);
    let alt_font = fonts.glyphs(Some(ENCHANTMENT_ALT_FONT_ID));
    let lapis_count = container
        .slots
        .iter()
        .find(|slot| slot.slot == 1)
        .filter(|slot| slot.item.item_id.is_some() && slot.item.count > 0)
        .map(|slot| slot.item.count)
        .unwrap_or_default();
    let infinite_materials = world
        .local_player()
        .abilities
        .is_some_and(|abilities| abilities.instabuild);
    let experience_level = world
        .local_player()
        .experience
        .map(|experience| experience.level)
        .unwrap_or_default();

    Some(std::array::from_fn(|index| {
        let data_index = i16::try_from(index).expect("three options fit i16");
        let cost = world
            .open_container_data_value(data_index)
            .unwrap_or_default();
        let required_lapis = i32::try_from(index + 1).expect("three options fit i32");
        let enabled = cost > 0
            && (infinite_materials
                || (lapis_count >= required_lapis && experience_level >= i32::from(cost)));
        let y = ENCHANTING_TABLE_OPTION_Y
            + i32::try_from(index).expect("three options fit i32")
                * ENCHANTING_TABLE_OPTION_SPACING;
        let highlighted = enabled
            && cursor_in_rect(
                cursor_position,
                ENCHANTING_TABLE_OPTION_X,
                y,
                ENCHANTING_TABLE_OPTION_WIDTH,
                ENCHANTING_TABLE_OPTION_HEIGHT,
            );
        let tooltip_hovered = cursor_in_rect(
            cursor_position,
            ENCHANTING_TABLE_OPTION_X,
            y,
            ENCHANTING_TABLE_OPTION_WIDTH,
            ENCHANTING_TABLE_OPTION_TOOLTIP_HEIGHT,
        );
        let cost_text = cost.to_string();
        let name_max_width = ENCHANTING_TABLE_OPTION_TEXT_WIDTH
            .saturating_sub(default_font.plain_text_width(&cost_text));
        let name = if cost > 0 {
            random_enchantment_name(&mut random, alt_font, name_max_width)
        } else {
            String::new()
        };
        let clue_holder_id = world
            .open_container_data_value(ENCHANTMENT_CLUE_DATA_ID + data_index)
            .filter(|holder_id| *holder_id >= 0)
            .map(i32::from);
        let clue_level = world
            .open_container_data_value(ENCHANTMENT_LEVEL_CLUE_DATA_ID + data_index)
            .filter(|level| *level >= 0)
            .map(i32::from);
        EnchantmentOptionHudState {
            index,
            cost,
            enabled,
            highlighted,
            tooltip_hovered,
            name,
            name_max_width,
            clue_holder_id,
            clue_level,
        }
    }))
}

fn enchantment_screen_tick_source(world: &WorldStore) -> Option<(i32, ItemStackSummary, bool)> {
    let container = world.inventory().open_container.as_ref()?;
    (container.menu_type_id == Some(VANILLA_MENU_TYPE_ENCHANTMENT_ID)).then_some(())?;
    let input = container
        .slots
        .iter()
        .find(|slot| slot.slot == 0)
        .map(|slot| slot.item.clone())
        .unwrap_or_else(ItemStackSummary::empty);
    let should_open = (0..ENCHANTMENT_OPTION_COUNT).any(|index| {
        world
            .open_container_data_value(i16::try_from(index).expect("three options fit i16"))
            .is_some_and(|cost| cost != 0)
    });
    Some((container.container_id, input, should_open))
}

fn random_enchantment_name(
    random: &mut LevelEventSoundRandomState,
    alt_font: &bbb_renderer::HudFontGlyphMap,
    max_width: u32,
) -> String {
    let word_count = random.next_int_bound(2) + 3;
    let mut result = String::new();
    for index in 0..word_count {
        if index != 0 {
            result.push(' ');
        }
        let word_index = usize::try_from(
            random.next_int_bound(i32::try_from(ENCHANTMENT_WORDS.len()).expect("word catalog")),
        )
        .expect("bounded random is nonnegative");
        result.push_str(ENCHANTMENT_WORDS[word_index]);
    }
    let cursor = alt_font
        .plain_text_cursor_for_width_from(&result, 0, max_width)
        .unwrap_or_else(|| result.chars().count());
    result.chars().take(cursor).collect()
}

fn cursor_in_rect(cursor: Option<(i32, i32)>, x: i32, y: i32, width: u32, height: u32) -> bool {
    let width = i32::try_from(width).expect("enchanting option width fits i32");
    let height = i32::try_from(height).expect("enchanting option height fits i32");
    cursor.is_some_and(|(cursor_x, cursor_y)| {
        cursor_x >= x && cursor_x < x + width && cursor_y >= y && cursor_y < y + height
    })
}

fn lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::{
        packets::{
            ContainerSetContent, ContainerSetData, OpenScreen, PlayerAbilities, PlayerExperience,
        },
        RichTextComponent,
    };
    use bbb_renderer::{HudAsciiGlyph, HudFontGlyphMap, HudUvRect};

    use super::*;

    fn test_fonts(default_advance: u32, alt_advance: u32) -> HudFontGlyphBook {
        fn glyphs(advance: u32) -> HudFontGlyphMap {
            let mut glyphs = HudFontGlyphMap::new();
            for byte in b' '..=b'z' {
                glyphs.insert_first_wins(
                    char::from(byte),
                    HudAsciiGlyph {
                        uv: HudUvRect {
                            min: [0.0, 0.0],
                            max: [1.0, 1.0],
                        },
                        width: advance,
                        height: 8,
                        advance,
                        ascent: 7,
                        ..HudAsciiGlyph::default()
                    },
                );
            }
            glyphs
        }
        HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([
                ("minecraft:default".to_string(), glyphs(default_advance)),
                (ENCHANTMENT_ALT_FONT_ID.to_string(), glyphs(alt_advance)),
            ]),
        )
    }

    fn enchantment_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
            title: RichTextComponent::plain("Enchanting Table"),
        });
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 1,
            items: vec![ItemStackSummary::empty(); 38],
            carried_item: ItemStackSummary::empty(),
        });
        world
    }

    fn set_data(world: &mut WorldStore, id: i16, value: i16) {
        world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id,
            value,
        });
    }

    #[test]
    fn enchanting_screen_book_ticks_and_interpolates_vanilla_state() {
        let mut world = enchantment_world();
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[0] = ItemStackSummary {
            item_id: Some(42),
            count: 1,
            ..ItemStackSummary::empty()
        };
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 2,
            items,
            carried_item: ItemStackSummary::empty(),
        });
        set_data(&mut world, 0, 5);

        let mut input = ClientInputState::default();
        input.advance_enchantment_screen(&world, 1);
        let state = input.enchantment_screen.as_ref().unwrap();
        assert_eq!(state.open, 0.2);
        assert_eq!(state.old_open, 0.0);
        assert!(state.flip_target.abs() > 1.0);
        assert_eq!(
            input.enchantment_book_hud_state(0.5),
            Some(EnchantmentBookHudState {
                open: 0.1,
                flip: state.flip * 0.5,
            })
        );

        input.advance_enchantment_screen(&world, 4);
        let state = input.enchantment_screen.as_ref().unwrap();
        assert_eq!(state.open, 1.0);
        assert_eq!(state.old_open, 0.8);
        assert!(state.flip.is_finite());
        assert!(state.flip_acceleration.abs() <= 0.2);
    }

    #[test]
    fn enchanting_screen_state_resets_on_container_change_and_close() {
        let mut world = enchantment_world();
        set_data(&mut world, 0, 5);
        let mut input = ClientInputState::default();
        input.advance_enchantment_screen(&world, 3);
        assert_eq!(input.enchantment_screen.as_ref().unwrap().container_id, 7);

        world.apply_open_screen(OpenScreen {
            container_id: 8,
            menu_type_id: VANILLA_MENU_TYPE_ENCHANTMENT_ID,
            title: RichTextComponent::plain("Other Table"),
        });
        input.advance_enchantment_screen(&world, 0);
        let state = input.enchantment_screen.as_ref().unwrap();
        assert_eq!(state.container_id, 8);
        assert_eq!(state.open, 0.0);

        world.apply_container_close(bbb_protocol::packets::ContainerClose { container_id: 8 });
        input.advance_enchantment_screen(&world, 0);
        assert!(input.enchantment_screen.is_none());
    }

    #[test]
    fn enchanting_names_follow_seed_row_order_and_alt_font_width() {
        let mut world = enchantment_world();
        set_data(&mut world, ENCHANTMENT_SEED_DATA_ID, 1234);
        set_data(&mut world, 0, 5);
        set_data(&mut world, 1, 12);
        set_data(&mut world, 2, 30);
        let fonts = test_fonts(2, 4);

        let first = enchantment_option_hud_states(&world, None, &fonts).unwrap();
        let second = enchantment_option_hud_states(&world, None, &fonts).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first
                .iter()
                .map(|option| option.name.as_str())
                .collect::<Vec<_>>(),
            [
                "of cold xyzzy towards",
                "enchant elder shorte",
                "animal rlyeh wgahnag"
            ]
        );
        assert_eq!(
            first
                .iter()
                .map(|option| option.name_max_width)
                .collect::<Vec<_>>(),
            [84, 82, 82]
        );
        assert!(first.iter().all(|option| fonts
            .glyphs(Some(ENCHANTMENT_ALT_FONT_ID))
            .plain_text_width(&option.name)
            <= option.name_max_width));
    }

    #[test]
    fn enchanting_word_catalog_matches_vanilla_26_1_source_fingerprint() {
        // FNV-1a over each UTF-8 word plus a NUL separator. The expected
        // fingerprint was generated directly from `EnchantmentNames.words`;
        // changing order, spelling, or cardinality requires re-verifying the
        // local Vanilla 26.1 source named in the engineering contract.
        let mut fingerprint = 0xcbf2_9ce4_8422_2325_u64;
        for word in ENCHANTMENT_WORDS {
            for byte in word.bytes().chain(std::iter::once(0)) {
                fingerprint ^= u64::from(byte);
                fingerprint = fingerprint.wrapping_mul(0x100_0000_01b3);
            }
        }
        assert_eq!(fingerprint, 0x5492_dcb6_43da_3f11);
    }

    #[test]
    fn option_projection_distinguishes_highlight_tooltip_and_affordability() {
        let mut world = enchantment_world();
        let mut items = vec![ItemStackSummary::empty(); 38];
        items[1] = ItemStackSummary {
            item_id: Some(42),
            count: 2,
            ..ItemStackSummary::empty()
        };
        world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 2,
            items,
            carried_item: ItemStackSummary::empty(),
        });
        set_data(&mut world, 0, 4);
        set_data(&mut world, 1, 12);
        set_data(&mut world, 2, 30);
        set_data(&mut world, ENCHANTMENT_CLUE_DATA_ID + 1, 13);
        set_data(&mut world, ENCHANTMENT_LEVEL_CLUE_DATA_ID + 1, 3);
        world.apply_player_experience(PlayerExperience {
            progress: 0.0,
            level: 12,
            total: 0,
        });

        let states =
            enchantment_option_hud_states(&world, Some((60, 33 + 18)), &test_fonts(2, 4)).unwrap();
        assert!(states[0].enabled);
        assert!(states[1].enabled);
        assert!(!states[2].enabled);
        assert!(states[1].highlighted);
        assert!(!states[1].tooltip_hovered);
        assert_eq!(states[1].clue_holder_id, Some(13));
        assert_eq!(states[1].clue_level, Some(3));

        world.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let states =
            enchantment_option_hud_states(&world, Some((60, 33)), &test_fonts(2, 4)).unwrap();
        assert!(states.iter().all(|option| option.enabled));
        assert!(states[1].tooltip_hovered);
    }
}
