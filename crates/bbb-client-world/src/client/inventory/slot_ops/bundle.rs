use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID, VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    },
    item_types::{vanilla_item_type, VanillaItemUseOverride},
    packets::ItemStackTemplateSummary,
};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BundleFraction {
    num: i128,
    den: i128,
}

impl BundleFraction {
    const ZERO: Self = Self { num: 0, den: 1 };
    const ONE: Self = Self { num: 1, den: 1 };

    fn new(num: i128, den: i128) -> Option<Self> {
        if den == 0 {
            return None;
        }
        let divisor = gcd(num.abs(), den.abs()).max(1);
        Some(Self {
            num: num / divisor,
            den: den / divisor,
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

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a
}

pub(in crate::client::inventory) fn is_bundle_item(stack: &ProtocolItemStackSummary) -> bool {
    !item_stack_is_empty(stack)
        && stack.item_id.is_some_and(|item_id| {
            vanilla_item_type(item_id).and_then(|entry| entry.use_override)
                == Some(VanillaItemUseOverride::BundleItem)
        })
}

fn effective_bundle_contents(
    stack: &ProtocolItemStackSummary,
) -> Option<&[ItemStackTemplateSummary]> {
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID)
    {
        return None;
    }
    if let Some(contents) = stack.component_patch.explicit_bundle_contents() {
        return Some(contents);
    }
    is_bundle_item(stack).then_some(&[])
}

fn stack_from_template(template: &ItemStackTemplateSummary) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(template.item_id),
        count: template.count,
        component_patch: template.component_patch.clone(),
    }
}

fn stack_to_template(stack: &ProtocolItemStackSummary) -> Option<ItemStackTemplateSummary> {
    if item_stack_is_empty(stack) {
        return None;
    }
    Some(ItemStackTemplateSummary {
        item_id: stack.item_id?,
        count: stack.count,
        component_patch: stack.component_patch.clone(),
    })
}

/// `BundleContents.getWeight` (`BundleContents.java:69-77`).
fn bundle_item_weight(
    stack: &ProtocolItemStackSummary,
    profiles: &ItemProfiles,
) -> Option<BundleFraction> {
    if let Some(nested) = effective_bundle_contents(stack) {
        let nested_items = nested.iter().map(stack_from_template).collect::<Vec<_>>();
        return bundle_contents_weight(&nested_items, profiles)?.add(BundleFraction::new(1, 16)?);
    }
    if !stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_BEES_TYPE_ID)
        && stack.component_patch.bees_present
        && !stack.component_patch.bees.is_empty()
    {
        return Some(BundleFraction::ONE);
    }
    let max_stack = profiles.item_stack_max_stack_size(stack);
    (max_stack > 0)
        .then(|| BundleFraction::new(1, i128::from(max_stack)))
        .flatten()
}

/// `BundleContents.computeContentWeight` (`BundleContents.java:50-67`).
fn bundle_contents_weight(
    items: &[ProtocolItemStackSummary],
    profiles: &ItemProfiles,
) -> Option<BundleFraction> {
    items.iter().try_fold(BundleFraction::ZERO, |weight, item| {
        weight.add(bundle_item_weight(item, profiles)?.times(item.count)?)
    })
}

/// `BundleContents.canItemBeInBundle` (`BundleContents.java:79-81`).
fn can_item_be_in_bundle(stack: &ProtocolItemStackSummary) -> bool {
    !item_stack_is_empty(stack) && item_can_fit_inside_container_items(stack)
}

/// `BundleContents.Mutable` (`BundleContents.java:138-261`).
struct MutableBundleContents {
    items: Vec<ProtocolItemStackSummary>,
    weight: BundleFraction,
}

impl MutableBundleContents {
    fn from_bundle(bundle: &ProtocolItemStackSummary, profiles: &ItemProfiles) -> Option<Self> {
        let raw = effective_bundle_contents(bundle)?;
        let items = raw.iter().map(stack_from_template).collect::<Vec<_>>();
        match bundle_contents_weight(&items, profiles) {
            Some(weight) => Some(Self { items, weight }),
            None => Some(Self {
                items: Vec::new(),
                weight: BundleFraction::ZERO,
            }),
        }
    }

    fn max_amount_to_add(&self, item_weight: BundleFraction) -> i32 {
        let Some(remaining) = BundleFraction::ONE.sub(self.weight) else {
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
        i32::try_from(num / den).unwrap_or(i32::MAX)
    }

    fn try_insert(&mut self, add: &mut ProtocolItemStackSummary, profiles: &ItemProfiles) -> i32 {
        if !can_item_be_in_bundle(add) {
            return 0;
        }
        let Some(item_weight) = bundle_item_weight(add, profiles) else {
            return 0;
        };
        let amount = add.count.min(self.max_amount_to_add(item_weight));
        if amount <= 0 {
            return 0;
        }
        let Some(updated_weight) = item_weight
            .times(amount)
            .and_then(|added| self.weight.add(added))
        else {
            return 0;
        };
        self.weight = updated_weight;
        let existing_index = if profiles.item_stack_max_stack_size(add) > 1 {
            self.items
                .iter()
                .position(|candidate| same_item_same_components(candidate, add))
        } else {
            None
        };
        let inserted = match existing_index {
            Some(index) => {
                let mut merged = self.items.remove(index);
                merged.count += amount;
                add.count -= amount;
                normalize_item_stack(add);
                merged
            }
            None => {
                let mut split = ProtocolItemStackSummary::empty();
                move_stack_count(add, &mut split, amount);
                split
            }
        };
        self.items.insert(0, inserted);
        amount
    }

    fn remove_one(
        &mut self,
        selected_index: i32,
        profiles: &ItemProfiles,
    ) -> Option<ProtocolItemStackSummary> {
        if self.items.is_empty() {
            return None;
        }
        let index = usize::try_from(selected_index)
            .ok()
            .filter(|index| *index < self.items.len())
            .unwrap_or(0);
        let removed = self.items.remove(index);
        self.weight = bundle_item_weight(&removed, profiles)
            .and_then(|weight| weight.times(removed.count))
            .and_then(|sub| self.weight.sub(sub))
            .unwrap_or(BundleFraction::ZERO);
        Some(removed)
    }

    fn write_to_bundle(self, bundle: &mut ProtocolItemStackSummary) {
        let templates = self
            .items
            .iter()
            .filter_map(stack_to_template)
            .collect::<Vec<_>>();
        let is_bundle = is_bundle_item(bundle);
        let patch = &mut bundle.component_patch;
        patch
            .removed_type_ids
            .retain(|type_id| *type_id != VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
        patch.bundle_contents_items = templates;
        if patch.bundle_contents_items.is_empty() && is_bundle {
            patch
                .added_type_ids
                .retain(|type_id| *type_id != VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
        } else if !patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID)
        {
            patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client::inventory) enum BundleClickSound {
    Insert,
    RemoveOne,
    InsertFail,
}

impl BundleClickSound {
    pub(in crate::client::inventory) fn into_action(self) -> crate::LocalInteractionSoundAction {
        match self {
            Self::Insert => crate::LocalInteractionSoundAction::BundleInsert,
            Self::RemoveOne => crate::LocalInteractionSoundAction::BundleRemoveOne,
            Self::InsertFail => crate::LocalInteractionSoundAction::BundleInsertFail,
        }
    }
}

/// Vanilla `AbstractContainerMenu.tryItemClickBehaviourOverride`
/// (`AbstractContainerMenu.java:560-567`) for `BundleItem`
/// (`BundleItem.java:59-137`).
pub(in crate::client::inventory) fn try_apply_bundle_pickup_override(
    slot: &mut ContainerSlot,
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
    slot_access: LocalContainerSlotAccess,
    profiles: &ItemProfiles,
    mut slot_access_for_stack: impl FnMut(&ProtocolItemStackSummary) -> Option<LocalContainerSlotAccess>,
) -> Option<(bool, Option<BundleClickSound>)> {
    if !matches!(button_num, 0 | 1) {
        return Some((false, None));
    }
    let primary = button_num == 0;

    // `BundleItem.overrideStackedOnOther` (`BundleItem.java:59-94`): carried bundle onto slot.
    if is_bundle_item(cursor) {
        if let Some(mut contents) = MutableBundleContents::from_bundle(cursor, profiles) {
            if primary && !item_stack_is_empty(&slot.item) {
                let mut inserted = 0;
                if slot_access.may_pickup && can_item_be_in_bundle(&slot.item) {
                    if let Some(weight) = bundle_item_weight(&slot.item, profiles) {
                        let max_amount = contents.max_amount_to_add(weight);
                        let to_take = slot.item.count.min(max_amount);
                        if to_take > 0 {
                            // `BundleContents.Mutable.tryTransfer` (`BundleContents.java:217-226`)
                            // calls `Slot.safeTake` -> `Slot.tryRemove` (`Slot.java:97-104`,
                            // `Slot.java:151-153`), which refuses partial takes whenever
                            // `!allowModification(player)` (`!(mayPickup && mayPlace(slot.getItem()))`),
                            // e.g. `FurnaceResultSlot`.
                            let self_access = slot_access_for_stack(&slot.item)?;
                            let allow_modification =
                                self_access.may_pickup && self_access.may_place;
                            if allow_modification || to_take == slot.item.count {
                                let mut taken = ProtocolItemStackSummary::empty();
                                move_stack_count(&mut slot.item, &mut taken, to_take);
                                inserted = contents.try_insert(&mut taken, profiles);
                                slot.normalize_selection();
                            }
                        }
                    }
                }
                let sound = if inserted > 0 {
                    BundleClickSound::Insert
                } else {
                    BundleClickSound::InsertFail
                };
                contents.write_to_bundle(cursor);
                return Some((true, Some(sound)));
            }
            if !primary && item_stack_is_empty(&slot.item) {
                let mut sound = None;
                if let Some(mut removed) =
                    contents.remove_one(NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX, profiles)
                {
                    let place_access = slot_access_for_stack(&removed)?;
                    if place_access.may_place && place_access.max_stack_size > 0 {
                        let place_count = removed.count.min(place_access.max_stack_size);
                        move_stack_count(&mut removed, &mut slot.item, place_count);
                        slot.normalize_selection();
                    }
                    if !item_stack_is_empty(&removed) {
                        contents.try_insert(&mut removed, profiles);
                    } else {
                        sound = Some(BundleClickSound::RemoveOne);
                    }
                }
                contents.write_to_bundle(cursor);
                return Some((true, sound));
            }
        }
    }

    // `BundleItem.overrideOtherStackedOnMe` (`BundleItem.java:97-137`): carried stack onto bundle slot.
    if is_bundle_item(&slot.item) {
        if primary && item_stack_is_empty(cursor) {
            slot.local_selected_bundle_item_index = NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
            return Some((false, None));
        }
        let Some(mut contents) = MutableBundleContents::from_bundle(&slot.item, profiles) else {
            return Some((false, None));
        };
        let self_access = slot_access_for_stack(&slot.item)?;
        let allow_modification = self_access.may_pickup && self_access.may_place;
        if primary && !item_stack_is_empty(cursor) {
            let inserted = if allow_modification {
                contents.try_insert(cursor, profiles)
            } else {
                0
            };
            let sound = if inserted > 0 {
                BundleClickSound::Insert
            } else {
                BundleClickSound::InsertFail
            };
            contents.write_to_bundle(&mut slot.item);
            slot.normalize_selection();
            return Some((true, Some(sound)));
        }
        if !primary && item_stack_is_empty(cursor) {
            let mut sound = None;
            if allow_modification {
                if let Some(removed) =
                    contents.remove_one(slot.local_selected_bundle_item_index, profiles)
                {
                    *cursor = removed;
                    slot.local_selected_bundle_item_index = NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
                    sound = Some(BundleClickSound::RemoveOne);
                }
            }
            contents.write_to_bundle(&mut slot.item);
            slot.normalize_selection();
            return Some((true, sound));
        }
        slot.local_selected_bundle_item_index = NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
        return Some((false, None));
    }

    Some((false, None))
}
