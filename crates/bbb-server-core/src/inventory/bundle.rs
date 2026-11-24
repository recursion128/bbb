//! `BundleItem`'s click overrides over `BundleContents` (`BundleItem.java`,
//! `BundleContents.java`), and the selected-item index the client picks with
//! `SelectBundleItem`.
//!
//! Vanilla keeps the selected index inside the in-memory `BundleContents`
//! (neither codec carries it); here it lives on the menu, keyed by the slot
//! the client named.

use crate::world::maps::ServerMapStore;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BEES_TYPE_ID, VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID as BUNDLE,
};
use bbb_protocol::item_types::{vanilla_item_type, VanillaItemUseOverride};
use bbb_protocol::packets::DataComponentPatchSummary;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_registry_codec::UnavailableHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::item_rules::is_stackable;
use super::menu_layout::item_places_a_shulker_box;
use super::PlayerInventoryMenu;

/// The `BundleItem` sounds a click plays (`BundleItem.java:257-267`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleSound {
    Insert,
    InsertFail,
    RemoveOne,
}

/// A non-negative rational, `org.apache.commons.lang3.math.Fraction` as the
/// weights use it; `None` from an operation is Vanilla's
/// "Excessive total bundle weight".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fraction {
    num: i128,
    den: i128,
}

impl Fraction {
    const ZERO: Self = Self { num: 0, den: 1 };
    const ONE: Self = Self { num: 1, den: 1 };

    fn new(num: i128, den: i128) -> Option<Self> {
        if den == 0 {
            return None;
        }
        let gcd = gcd(num.abs(), den.abs()).max(1);
        Some(Self {
            num: num / gcd,
            den: den / gcd,
        })
    }

    fn add(self, other: Self) -> Option<Self> {
        Self::new(
            self.num
                .checked_mul(other.den)?
                .checked_add(other.num.checked_mul(self.den)?)?,
            self.den.checked_mul(other.den)?,
        )
    }

    fn sub(self, other: Self) -> Option<Self> {
        self.add(Self {
            num: -other.num,
            den: other.den,
        })
    }

    fn times(self, count: i32) -> Option<Self> {
        Self::new(self.num.checked_mul(i128::from(count))?, self.den)
    }
}

fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn is_bundle(item: &RuntimeItemStack) -> bool {
    item.item_id()
        .and_then(vanilla_item_type)
        .is_some_and(|entry| entry.use_override == Some(VanillaItemUseOverride::BundleItem))
}

/// `stack.get(BUNDLE_CONTENTS)`.
fn bundle_items(item: &RuntimeItemStack) -> Option<Vec<RuntimeItemStack>> {
    let patch = item
        .effective_component(BUNDLE, &UnavailableHolderResolver)
        .ok()??;
    Some(
        patch
            .bundle_contents_items
            .iter()
            .map(RuntimeItemStack::from_template)
            .collect(),
    )
}

/// `BundleContents.getWeight` (`BundleContents.java:69-77`).
fn item_weight(item: &RuntimeItemStack) -> Option<Fraction> {
    if let Some(nested) = bundle_items(item) {
        return contents_weight(&nested)?.add(Fraction::new(1, 16)?);
    }
    let bees = item
        .component_nbt(
            VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
            &UnavailableHolderResolver,
        )
        .ok()?;
    if matches!(bees, Some(NbtSummaryValue::List(bees)) if !bees.is_empty()) {
        return Some(Fraction::ONE);
    }
    Fraction::new(1, i128::from(item.max_stack_size().ok()?))
}

/// `BundleContents.computeContentWeight` (`BundleContents.java:50-67`).
fn contents_weight(items: &[RuntimeItemStack]) -> Option<Fraction> {
    items.iter().try_fold(Fraction::ZERO, |weight, item| {
        weight.add(item_weight(item)?.times(item.count())?)
    })
}

/// `BundleContents.canItemBeInBundle` (`BundleContents.java:79-81`).
fn can_be_in_bundle(item: &RuntimeItemStack) -> bool {
    !item.is_empty() && !item_places_a_shulker_box(item.summary())
}

/// `BundleContents.Mutable`.
struct Contents {
    items: Vec<RuntimeItemStack>,
    weight: Fraction,
}

impl Contents {
    /// `None` for a stack without the component, or one whose weight cannot
    /// be computed (Vanilla would empty it).
    fn read(bundle: &RuntimeItemStack) -> Option<Self> {
        let items = bundle_items(bundle)?;
        let weight = contents_weight(&items)?;
        Some(Self { items, weight })
    }

    /// `getMaxAmountToAdd` (`BundleContents.java:182-185`).
    fn max_amount_to_add(&self, item_weight: Fraction) -> i32 {
        let Some(remaining) = Fraction::ONE.sub(self.weight) else {
            return 0;
        };
        if remaining.num <= 0 {
            return 0;
        }
        let (Some(num), Some(den)) = (
            remaining.num.checked_mul(item_weight.den),
            remaining.den.checked_mul(item_weight.num),
        ) else {
            return 0;
        };
        if den <= 0 {
            return 0;
        }
        let amount = num / den;
        i32::try_from(amount).unwrap_or(i32::MAX)
    }

    /// `tryInsert` (`BundleContents.java:187-215`): how many it took out of
    /// `add`.
    fn try_insert(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        add: &mut RuntimeItemStack,
    ) -> Result<i32> {
        if !can_be_in_bundle(add) {
            return Ok(0);
        }
        let Some(item_weight) = item_weight(add) else {
            return Ok(0);
        };
        let amount = add.count().min(self.max_amount_to_add(item_weight));
        if amount == 0 {
            return Ok(0);
        }
        let Some(weight) = item_weight
            .times(amount)
            .and_then(|added| self.weight.add(added))
        else {
            return Ok(0);
        };
        self.weight = weight;
        let mut index = None;
        if is_stackable(add.summary())? {
            for (candidate, item) in self.items.iter().enumerate() {
                if item.same_item_same_effective_components(add, resolver)? {
                    index = Some(candidate);
                    break;
                }
            }
        }
        let inserted = match index {
            Some(index) => {
                let merged = self.items.remove(index);
                add.shrink(amount);
                merged.with_count(merged.count() + amount)
            }
            None => add.split(amount),
        };
        self.items.insert(0, inserted);
        Ok(amount)
    }

    /// `removeOne` (`BundleContents.java:236-246`).
    fn remove_one(&mut self, selected: Option<i32>) -> Option<RuntimeItemStack> {
        if self.items.is_empty() {
            return None;
        }
        let index = selected
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < self.items.len())
            .unwrap_or(0);
        let stack = self.items.remove(index);
        self.weight = item_weight(&stack)
            .and_then(|weight| weight.times(stack.count()))
            .and_then(|removed| self.weight.sub(removed))
            .unwrap_or(Fraction::ZERO);
        Some(stack)
    }

    /// `stack.set(BUNDLE_CONTENTS, contents.toImmutable())`.
    fn write(&self, bundle: &RuntimeItemStack) -> Result<RuntimeItemStack> {
        let templates = self
            .items
            .iter()
            .filter_map(RuntimeItemStack::to_template)
            .collect::<Vec<_>>();
        let patch = DataComponentPatchSummary {
            added_type_ids: vec![BUNDLE],
            bundle_contents_items: templates,
            ..Default::default()
        };
        bundle
            .clone()
            .with_component_patch(&patch, &UnavailableHolderResolver)
    }
}

/// `BundleItem.removeOneItemFromBundle` (`BundleItem.java:207-217`) on a
/// bundle in hand: the bundle left behind and the stack that came out.
pub fn bundle_remove_one(
    bundle: &RuntimeItemStack,
) -> Result<Option<(RuntimeItemStack, RuntimeItemStack)>> {
    let Some(mut contents) = Contents::read(bundle) else {
        return Ok(None);
    };
    let Some(removed) = contents.remove_one(None) else {
        return Ok(None);
    };
    Ok(Some((contents.write(bundle)?, removed)))
}

impl PlayerInventoryMenu {
    /// `AbstractContainerMenu.setSelectedBundleItemIndex`
    /// (`AbstractContainerMenu.java:311-316`) and
    /// `BundleContents.Mutable.toggleSelectedItem` (`BundleContents.java:228-230`).
    pub fn select_bundle_item(&mut self, slot: i16, index: i32) {
        if slot < 0 || !self.addressable(slot) {
            return;
        }
        let stack = self.slot_item(slot);
        let Some(size) = bundle_items(&stack).map(|items| items.len()) else {
            return;
        };
        let selected_stack = stack.into_owned();
        let current = self.selected_bundle_item(slot);
        let in_bounds = usize::try_from(index).is_ok_and(|index| index < size);
        self.menu.bundle_selection =
            (current != Some(index) && in_bounds).then_some((slot, index, selected_stack));
    }

    fn selected_bundle_item(&self, slot: i16) -> Option<i32> {
        self.menu
            .bundle_selection
            .as_ref()
            .filter(|(selected_slot, _, selected_stack)| {
                *selected_slot == slot && self.slot_item(slot).as_ref() == selected_stack
            })
            .map(|(_, index, _)| *index)
    }

    pub(super) fn clear_bundle_selection(&mut self, slot: i16) {
        if self
            .menu
            .bundle_selection
            .as_ref()
            .is_some_and(|(selected_slot, _, _)| *selected_slot == slot)
        {
            self.menu.bundle_selection = None;
        }
    }

    pub(super) fn clear_bundle_selection_for_inventory_index(&mut self, index: usize) {
        if let Some((selected_slot, _, _)) = &self.menu.bundle_selection {
            if self.menu.layout.slot(*selected_slot) == Some(super::MenuSlot::Inventory(index)) {
                self.menu.bundle_selection = None;
            }
        }
    }

    pub fn take_bundle_sounds(&mut self) -> Vec<BundleSound> {
        std::mem::take(&mut self.menu.bundle_sounds)
    }

    /// `AbstractContainerMenu.tryItemClickBehaviourOverride`
    /// (`AbstractContainerMenu.java:560-567`); `BundleItem` is the only item
    /// overriding either hook.
    pub(super) fn try_bundle_click_override(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        primary: bool,
        creative: bool,
    ) -> Result<bool> {
        let carried = self.menu.carried.clone();
        if is_bundle(&carried)
            && self.bundle_stacked_on_other(
                resolver,
                maps.as_deref_mut(),
                slot,
                primary,
                &carried,
                creative,
            )?
        {
            return Ok(true);
        }
        if is_bundle(&self.slot_item(slot)) {
            return self.bundle_other_stacked_on_me(resolver, maps, slot, primary, creative);
        }
        Ok(false)
    }

    /// `BundleItem.overrideStackedOnOther` (`BundleItem.java:59-94`): the
    /// carried bundle clicked onto `slot`.
    fn bundle_stacked_on_other(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        primary: bool,
        bundle: &RuntimeItemStack,
        creative: bool,
    ) -> Result<bool> {
        let Some(mut contents) = Contents::read(bundle) else {
            return Ok(false);
        };
        let other = self.slot_item(slot).into_owned();
        if primary && !other.is_empty() {
            // `Mutable.tryTransfer` (`BundleContents.java:217-226`).
            let mut inserted = 0;
            if let Some(weight) = item_weight(&other).filter(|_| can_be_in_bundle(&other)) {
                let max = contents.max_amount_to_add(weight);
                let mut taken = self.slot_safe_take(
                    resolver,
                    maps.as_deref_mut(),
                    slot,
                    other.count(),
                    max,
                    creative,
                )?;
                inserted = contents.try_insert(resolver, &mut taken)?;
            }
            self.menu.bundle_sounds.push(if inserted > 0 {
                BundleSound::Insert
            } else {
                BundleSound::InsertFail
            });
        } else if !primary && other.is_empty() {
            if let Some(removed) = contents.remove_one(None) {
                let mut remainder =
                    self.slot_safe_insert(resolver, maps, slot, removed.clone(), removed.count())?;
                if remainder.count() > 0 {
                    contents.try_insert(resolver, &mut remainder)?;
                } else {
                    self.menu.bundle_sounds.push(BundleSound::RemoveOne);
                }
            }
        } else {
            return Ok(false);
        }
        self.menu.carried = contents.write(bundle)?;
        Ok(true)
    }

    /// `BundleItem.overrideOtherStackedOnMe` (`BundleItem.java:97-137`): the
    /// carried stack clicked onto the bundle in `slot`.
    fn bundle_other_stacked_on_me(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
        slot: i16,
        primary: bool,
        creative: bool,
    ) -> Result<bool> {
        let other_empty = self.menu.carried.is_empty();
        if primary && other_empty {
            self.clear_bundle_selection(slot);
            return Ok(false);
        }
        let bundle = self.slot_item(slot).into_owned();
        let Some(mut contents) = Contents::read(&bundle) else {
            return Ok(false);
        };
        let allow_modification =
            self.slot_may_pickup(slot, creative) && self.slot_may_place(slot, &bundle)?;
        if primary {
            let mut carried = std::mem::take(&mut self.menu.carried);
            let inserted = if allow_modification {
                contents.try_insert(resolver, &mut carried)?
            } else {
                0
            };
            self.menu.carried = carried;
            self.menu.bundle_sounds.push(if inserted > 0 {
                BundleSound::Insert
            } else {
                BundleSound::InsertFail
            });
        } else if other_empty {
            if allow_modification {
                if let Some(removed) = contents.remove_one(self.selected_bundle_item(slot)) {
                    self.clear_bundle_selection(slot);
                    self.menu.bundle_sounds.push(BundleSound::RemoveOne);
                    self.menu.carried = removed;
                }
            }
        } else {
            self.clear_bundle_selection(slot);
            return Ok(false);
        }
        self.set_slot_item_in_place(resolver, maps, slot, contents.write(&bundle)?)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use bbb_facts::container_inputs::ContainerInput;
    use bbb_protocol::item_types::{
        VANILLA_ITEM_BUNDLE_ID, VANILLA_ITEM_DIRT_ID, VANILLA_ITEM_ENDER_PEARL_ID,
        VANILLA_ITEM_STONE_ID,
    };
    use bbb_protocol::packets::ItemStackSummary;
    use bbb_protocol::NbtSummaryEntry;

    use super::*;

    /// Main-inventory menu slots of the player's own menu.
    const A: i16 = 9;
    const B: i16 = 10;
    const C: i16 = 11;

    fn stack(item_id: i32, count: i32) -> RuntimeItemStack {
        RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        })
    }

    fn contents(bundle: &RuntimeItemStack) -> Vec<(Option<i32>, i32)> {
        bundle_items(bundle)
            .unwrap()
            .iter()
            .map(|item| (item.item_id(), item.count()))
            .collect()
    }

    fn click(menu: &mut PlayerInventoryMenu, slot: i16, button: i8) {
        menu.clicked(
            crate::inventory::test_item_resolver(),
            None,
            slot,
            button,
            ContainerInput::Pickup,
            false,
        )
        .unwrap();
    }

    #[test]
    fn a_carried_bundle_takes_stacks_until_its_weight_reaches_one() {
        let mut menu = PlayerInventoryMenu::empty();
        menu.set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            A,
            stack(VANILLA_ITEM_STONE_ID, 48),
        )
        .unwrap();
        menu.set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            B,
            stack(VANILLA_ITEM_ENDER_PEARL_ID, 16),
        )
        .unwrap();
        menu.set_carried(stack(VANILLA_ITEM_BUNDLE_ID, 1));

        click(&mut menu, A, 0);
        assert!(menu.slot_item(A).is_empty());
        // 48/64 used: a quarter left, which is four sixteenths of pearls.
        click(&mut menu, B, 0);
        assert_eq!(menu.slot_item(B).count(), 12);
        assert_eq!(
            contents(menu.carried()),
            vec![
                (Some(VANILLA_ITEM_ENDER_PEARL_ID), 4),
                (Some(VANILLA_ITEM_STONE_ID), 48)
            ]
        );
        // Full: the next click fails and leaves the slot alone.
        click(&mut menu, B, 0);
        assert_eq!(menu.slot_item(B).count(), 12);
        assert_eq!(
            menu.take_bundle_sounds(),
            vec![
                BundleSound::Insert,
                BundleSound::Insert,
                BundleSound::InsertFail
            ]
        );
    }

    #[test]
    fn a_bundle_in_a_slot_takes_the_carried_stack_and_merges_it() {
        let mut menu = PlayerInventoryMenu::empty();
        menu.set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            A,
            stack(VANILLA_ITEM_BUNDLE_ID, 1),
        )
        .unwrap();
        menu.set_carried(stack(VANILLA_ITEM_DIRT_ID, 10));
        click(&mut menu, A, 0);
        menu.set_carried(stack(VANILLA_ITEM_STONE_ID, 5));
        click(&mut menu, A, 0);
        menu.set_carried(stack(VANILLA_ITEM_DIRT_ID, 6));
        click(&mut menu, A, 0);

        assert!(menu.carried().is_empty());
        assert_eq!(
            contents(&menu.slot_item(A)),
            vec![
                (Some(VANILLA_ITEM_DIRT_ID), 16),
                (Some(VANILLA_ITEM_STONE_ID), 5)
            ]
        );

        for (component, value, merges) in [
            (
                "minecraft:rarity",
                NbtSummaryValue::String("common".into()),
                true,
            ),
            (
                "minecraft:rarity",
                NbtSummaryValue::String("uncommon".into()),
                false,
            ),
            (
                "!minecraft:rarity",
                NbtSummaryValue::Compound(vec![]),
                false,
            ),
        ] {
            menu.set_by_player(
                crate::inventory::test_item_resolver(),
                None,
                B,
                stack(VANILLA_ITEM_BUNDLE_ID, 1),
            )
            .unwrap();
            menu.set_carried(stack(VANILLA_ITEM_STONE_ID, 5));
            click(&mut menu, B, 0);
            menu.set_carried(
                RuntimeItemStack::try_read(
                    &NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "id".into(),
                            value: NbtSummaryValue::String("minecraft:stone".into()),
                        },
                        NbtSummaryEntry {
                            name: "count".into(),
                            value: NbtSummaryValue::Int(6),
                        },
                        NbtSummaryEntry {
                            name: "components".into(),
                            value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                                name: component.into(),
                                value,
                            }]),
                        },
                    ]),
                    crate::inventory::test_item_resolver(),
                )
                .unwrap(),
            );
            click(&mut menu, B, 0);
            assert!(menu.carried().is_empty());
            assert_eq!(
                contents(&menu.slot_item(B)),
                if merges {
                    vec![(Some(VANILLA_ITEM_STONE_ID), 11)]
                } else {
                    vec![
                        (Some(VANILLA_ITEM_STONE_ID), 6),
                        (Some(VANILLA_ITEM_STONE_ID), 5),
                    ]
                },
            );
        }
    }

    #[test]
    fn right_clicks_take_out_the_selected_stack_or_else_the_first() {
        let mut menu = PlayerInventoryMenu::empty();
        menu.set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            A,
            stack(VANILLA_ITEM_BUNDLE_ID, 1),
        )
        .unwrap();
        for item_id in [VANILLA_ITEM_STONE_ID, VANILLA_ITEM_DIRT_ID] {
            menu.set_carried(stack(item_id, 3));
            click(&mut menu, A, 0);
        }

        // Dirt went in last, so it is first; select the stone behind it.
        menu.select_bundle_item(A, 1);
        click(&mut menu, A, 1);
        assert_eq!(menu.carried().item_id(), Some(VANILLA_ITEM_STONE_ID));
        assert_eq!(
            contents(&menu.slot_item(A)),
            vec![(Some(VANILLA_ITEM_DIRT_ID), 3)]
        );

        // A carried bundle right-clicked onto an empty slot empties into it.
        menu.set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            B,
            menu.carried().clone(),
        )
        .unwrap();
        let bundle = menu.slot_item(A).into_owned();
        menu.set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            A,
            RuntimeItemStack::empty(),
        )
        .unwrap();
        menu.set_carried(bundle);
        click(&mut menu, C, 1);
        assert_eq!(menu.slot_item(C).item_id(), Some(VANILLA_ITEM_DIRT_ID));
        assert_eq!(menu.slot_item(C).count(), 3);
        assert!(contents(menu.carried()).is_empty());
    }

    #[test]
    fn a_bundle_inside_a_bundle_weighs_its_contents_plus_a_sixteenth() {
        let mut menu = PlayerInventoryMenu::empty();
        menu.set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            A,
            stack(VANILLA_ITEM_BUNDLE_ID, 1),
        )
        .unwrap();
        menu.set_carried(stack(VANILLA_ITEM_STONE_ID, 32));
        click(&mut menu, A, 0);
        let inner = menu.slot_item(A).into_owned();
        assert_eq!(item_weight(&inner), Fraction::new(9, 16));

        menu.set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            B,
            stack(VANILLA_ITEM_BUNDLE_ID, 1),
        )
        .unwrap();
        menu.set_carried(stack(VANILLA_ITEM_STONE_ID, 32));
        click(&mut menu, B, 0);
        // 1/2 + 9/16 does not fit.
        menu.set_carried(inner);
        click(&mut menu, B, 0);
        assert_eq!(menu.carried().item_id(), Some(VANILLA_ITEM_BUNDLE_ID));
        assert_eq!(contents(&menu.slot_item(B)).len(), 1);
    }

    #[test]
    fn swapping_bundle_slots_clears_bundle_selection() {
        let mut menu = PlayerInventoryMenu::empty();
        // Put bundle 1 (dirt on top, stone at index 1) in hotbar slot 0 (menu slot 36)
        // and bundle 2 (pearl on top, stone at index 1) in hotbar slot 1 (menu slot 37).
        for (slot, second_item) in [
            (36_i16, VANILLA_ITEM_DIRT_ID),
            (37_i16, VANILLA_ITEM_ENDER_PEARL_ID),
        ] {
            menu.set_by_player(
                crate::inventory::test_item_resolver(),
                None,
                slot,
                stack(VANILLA_ITEM_BUNDLE_ID, 1),
            )
            .unwrap();
            menu.set_carried(stack(VANILLA_ITEM_STONE_ID, 2));
            click(&mut menu, slot, 0);
            menu.set_carried(stack(second_item, 1));
            click(&mut menu, slot, 0);
        }

        // Select index 1 (stone) on slot 36, then swap hotbar 1 into slot 36.
        menu.select_bundle_item(36, 1);
        menu.clicked(
            crate::inventory::test_item_resolver(),
            None,
            36,
            1,
            ContainerInput::Swap,
            false,
        )
        .unwrap();

        // Right-clicking slot 36 must now remove bundle 2's top item (ender pearl),
        // not index 1 leaked from bundle 1.
        click(&mut menu, 36, 1);
        assert_eq!(menu.carried().item_id(), Some(VANILLA_ITEM_ENDER_PEARL_ID));
    }
}
