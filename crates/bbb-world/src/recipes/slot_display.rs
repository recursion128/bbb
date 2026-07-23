use bbb_protocol::{
    builtin_enum_component_values::{
        vanilla_dye_color_texture_diffuse_color, VANILLA_DYE_COLOR_WHITE_ID,
    },
    data_component_types::{
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
    },
    packets::{
        DataComponentPatchSummary, ItemStackSummary, SlotDisplaySummary,
        SlotDisplayTrimPatternSummary, SlotDisplayValue, TrimMaterialSummary,
    },
    potion_types::VANILLA_POTIONS,
};

use crate::{direct_trim_material_presentation, direct_trim_pattern_presentation, WorldStore};

const ITEM_REGISTRY: &str = "minecraft:item";
const MAX_RESOLVED_SLOT_DISPLAY_STACKS: usize = 65_536;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SlotDisplayResolveError {
    #[error("invalid slot display payload: {0}")]
    InvalidPayload(String),
    #[error("slot display resolved {actual} candidates, maximum is 65536")]
    TooManyCandidates { actual: usize },
    #[error("trim pattern holder {holder_id} is unavailable in the current registry revision")]
    MissingTrimPattern { holder_id: i32 },
    #[error("direct trim pattern is invalid")]
    InvalidDirectTrimPattern,
    #[error("trim material holder {holder_id} is unavailable in the current registry revision")]
    MissingTrimMaterial { holder_id: i32 },
    #[error("direct trim material is invalid")]
    InvalidDirectTrimMaterial,
}

impl WorldStore {
    pub fn resolve_slot_display_stacks(
        &self,
        display: &SlotDisplaySummary,
    ) -> Result<Vec<ItemStackSummary>, SlotDisplayResolveError> {
        let value = display
            .decode_value()
            .map_err(|error| SlotDisplayResolveError::InvalidPayload(error.to_string()))?;
        SlotDisplayResolver { world: self }.resolve(&value, fnv1a64(&display.raw_payload))
    }

    pub fn resolve_slot_display_first_stack(
        &self,
        display: &SlotDisplaySummary,
    ) -> Result<Option<ItemStackSummary>, SlotDisplayResolveError> {
        let value = display
            .decode_value()
            .map_err(|error| SlotDisplayResolveError::InvalidPayload(error.to_string()))?;
        SlotDisplayResolver { world: self }.resolve_first(&value, fnv1a64(&display.raw_payload))
    }
}

pub(super) fn slot_display_holders_are_resolved(
    world: &WorldStore,
    display: &SlotDisplaySummary,
) -> bool {
    display
        .decode_value()
        .ok()
        .is_some_and(|value| validate_slot_display_holders(world, &value))
}

fn validate_slot_display_holders(world: &WorldStore, display: &SlotDisplayValue) -> bool {
    match display {
        SlotDisplayValue::Empty | SlotDisplayValue::AnyFuel | SlotDisplayValue::Tag { .. } => true,
        SlotDisplayValue::Item { stack } | SlotDisplayValue::ItemStack { stack } => {
            validate_item_stack_trim_holders(world, stack)
        }
        SlotDisplayValue::WithAnyPotion { display } => {
            validate_slot_display_holders(world, display)
        }
        SlotDisplayValue::OnlyWithComponent { source, .. } => {
            validate_slot_display_holders(world, source)
        }
        SlotDisplayValue::Dyed { dye, target } => {
            validate_slot_display_holders(world, dye)
                && validate_slot_display_holders(world, target)
        }
        SlotDisplayValue::SmithingTrim {
            base,
            material,
            pattern,
        } => {
            validate_slot_display_holders(world, base)
                && validate_slot_display_holders(world, material)
                && validate_trim_pattern(world, pattern).is_ok()
        }
        SlotDisplayValue::WithRemainder { input, remainder } => {
            validate_slot_display_holders(world, input)
                && validate_slot_display_holders(world, remainder)
        }
        SlotDisplayValue::Composite { contents } => contents
            .iter()
            .all(|display| validate_slot_display_holders(world, display)),
    }
}

fn validate_item_stack_trim_holders(world: &WorldStore, stack: &ItemStackSummary) -> bool {
    let patch = &stack.component_patch;
    let provides_trim_material_is_valid = !patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID)
        || validate_trim_material(
            world,
            patch.provides_trim_material_id,
            patch.provides_trim_material_direct.as_deref(),
        )
        .is_ok();
    let trim_is_valid = !patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_TRIM_TYPE_ID)
        || (validate_trim_material(
            world,
            patch.armor_trim_material_id,
            patch.armor_trim_material_direct.as_deref(),
        )
        .is_ok()
            && validate_trim_pattern(
                world,
                &SlotDisplayTrimPatternSummary {
                    registry_id: patch.armor_trim_pattern_id,
                    direct: patch.armor_trim_pattern_direct.as_deref().cloned(),
                },
            )
            .is_ok());
    provides_trim_material_is_valid && trim_is_valid
}

struct SlotDisplayResolver<'a> {
    world: &'a WorldStore,
}

impl SlotDisplayResolver<'_> {
    fn resolve_first(
        &self,
        display: &SlotDisplayValue,
        seed: u64,
    ) -> Result<Option<ItemStackSummary>, SlotDisplayResolveError> {
        match display {
            SlotDisplayValue::Empty => Ok(None),
            SlotDisplayValue::AnyFuel => Ok(self
                .world
                .furnace_fuel_item_ids()
                .first()
                .map(|item_id| self.effective_item_stack(*item_id))),
            SlotDisplayValue::WithAnyPotion { display } => {
                let Some(stack) = self.resolve_first(display, child_seed(seed, 0, 0))? else {
                    return Ok(None);
                };
                Ok(VANILLA_POTIONS
                    .first()
                    .map(|potion| with_potion(stack, potion.id)))
            }
            SlotDisplayValue::OnlyWithComponent {
                source,
                component_type_id,
            } => Ok(self
                .resolve(source, child_seed(seed, 1, 0))?
                .into_iter()
                .find(|stack| {
                    self.world
                        .item_stack_has_effective_component(stack, *component_type_id)
                })),
            SlotDisplayValue::Item { stack } | SlotDisplayValue::ItemStack { stack } => {
                Ok(Some(self.world.effective_item_stack(stack).into_owned()))
            }
            SlotDisplayValue::Tag { tag } => {
                let Ok(holder_set) = self
                    .world
                    .frozen_registry_access()
                    .holder_set_by_tag(ITEM_REGISTRY, tag)
                else {
                    return Ok(None);
                };
                Ok(holder_set
                    .entries
                    .first()
                    .map(|entry| self.effective_item_stack(entry.holder_id)))
            }
            SlotDisplayValue::Dyed { dye, target } => {
                let Some(mut target) = self.resolve_first(target, child_seed(seed, 2, 0))? else {
                    return Ok(None);
                };
                let Some(dye) = self.resolve_first(dye, child_seed(seed, 2, 1))? else {
                    return Ok(None);
                };
                let dye_color_id = self.dye_color_id(&dye);
                target.count = 1;
                let color = apply_dye_color(target.component_patch.dyed_color, dye_color_id);
                set_component(
                    &mut target.component_patch,
                    VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
                );
                target.component_patch.dyed_color = Some(color);
                Ok(Some(target))
            }
            SlotDisplayValue::SmithingTrim {
                base,
                material,
                pattern,
            } => {
                let bases = self.resolve(base, child_seed(seed, 3, 0))?;
                let materials = self.resolve(material, child_seed(seed, 3, 1))?;
                if bases.is_empty() || materials.is_empty() {
                    return Ok(None);
                }
                validate_trim_pattern(self.world, pattern)?;
                let mut random = JavaRandom::new(seed as i64);
                for _ in 0..256 {
                    let base = &bases[random.next_int(bases.len())];
                    let material = &materials[random.next_int(materials.len())];
                    if let Some(trimmed) = self.apply_smithing_trim(base, material, pattern)? {
                        return Ok(Some(trimmed));
                    }
                }
                Ok(None)
            }
            SlotDisplayValue::WithRemainder { input, .. } => {
                self.resolve_first(input, child_seed(seed, 4, 0))
            }
            SlotDisplayValue::Composite { contents } => {
                for (index, content) in contents.iter().enumerate() {
                    if let Some(stack) =
                        self.resolve_first(content, child_seed(seed, 5, index as u64))?
                    {
                        return Ok(Some(stack));
                    }
                }
                Ok(None)
            }
        }
    }

    fn resolve(
        &self,
        display: &SlotDisplayValue,
        seed: u64,
    ) -> Result<Vec<ItemStackSummary>, SlotDisplayResolveError> {
        match display {
            SlotDisplayValue::Empty => Ok(Vec::new()),
            SlotDisplayValue::AnyFuel => {
                self.resolve_item_ids(self.world.furnace_fuel_item_ids().iter().copied())
            }
            SlotDisplayValue::WithAnyPotion { display } => {
                let source = self.resolve(display, child_seed(seed, 0, 0))?;
                let candidate_count = source
                    .len()
                    .checked_mul(VANILLA_POTIONS.len())
                    .unwrap_or(usize::MAX);
                ensure_candidate_count(candidate_count)?;
                let mut resolved = Vec::with_capacity(candidate_count);
                for potion in VANILLA_POTIONS {
                    for stack in &source {
                        resolved.push(with_potion(stack.clone(), potion.id));
                    }
                }
                Ok(resolved)
            }
            SlotDisplayValue::OnlyWithComponent {
                source,
                component_type_id,
            } => Ok(self
                .resolve(source, child_seed(seed, 1, 0))?
                .into_iter()
                .filter(|stack| {
                    self.world
                        .item_stack_has_effective_component(stack, *component_type_id)
                })
                .collect()),
            SlotDisplayValue::Item { stack } | SlotDisplayValue::ItemStack { stack } => {
                Ok(vec![self.world.effective_item_stack(stack).into_owned()])
            }
            SlotDisplayValue::Tag { tag } => {
                let Ok(holder_set) = self
                    .world
                    .frozen_registry_access()
                    .holder_set_by_tag(ITEM_REGISTRY, tag)
                else {
                    return Ok(Vec::new());
                };
                self.resolve_item_ids(holder_set.entries.iter().map(|entry| entry.holder_id))
            }
            SlotDisplayValue::Dyed { dye, target } => {
                let targets = self.resolve(target, child_seed(seed, 2, 0))?;
                let dyes = self.resolve(dye, child_seed(seed, 2, 1))?;
                let mut resolved = Vec::with_capacity(16);
                for dye in &dyes {
                    let dye_color_id = self.dye_color_id(dye);
                    for target in &targets {
                        let mut target = target.clone();
                        target.count = 1;
                        let color =
                            apply_dye_color(target.component_patch.dyed_color, dye_color_id);
                        set_component(
                            &mut target.component_patch,
                            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
                        );
                        target.component_patch.dyed_color = Some(color);
                        resolved.push(target);
                        if resolved.len() == 16 {
                            return Ok(resolved);
                        }
                    }
                }
                Ok(resolved)
            }
            SlotDisplayValue::SmithingTrim {
                base,
                material,
                pattern,
            } => {
                let bases = self.resolve(base, child_seed(seed, 3, 0))?;
                let materials = self.resolve(material, child_seed(seed, 3, 1))?;
                if bases.is_empty() || materials.is_empty() {
                    return Ok(Vec::new());
                }
                validate_trim_pattern(self.world, pattern)?;
                let mut random = JavaRandom::new(seed as i64);
                let mut resolved = Vec::with_capacity(16);
                for _ in 0..256 {
                    let base = &bases[random.next_int(bases.len())];
                    let material = &materials[random.next_int(materials.len())];
                    if let Some(trimmed) = self.apply_smithing_trim(base, material, pattern)? {
                        resolved.push(trimmed);
                        if resolved.len() == 16 {
                            break;
                        }
                    }
                }
                Ok(resolved)
            }
            SlotDisplayValue::WithRemainder { input, .. } => {
                self.resolve(input, child_seed(seed, 4, 0))
            }
            SlotDisplayValue::Composite { contents } => {
                let mut resolved = Vec::new();
                for (index, content) in contents.iter().enumerate() {
                    let candidates = self.resolve(content, child_seed(seed, 5, index as u64))?;
                    let candidate_count = resolved
                        .len()
                        .checked_add(candidates.len())
                        .unwrap_or(usize::MAX);
                    ensure_candidate_count(candidate_count)?;
                    resolved.extend(candidates);
                }
                Ok(resolved)
            }
        }
    }

    fn resolve_item_ids(
        &self,
        item_ids: impl IntoIterator<Item = i32>,
    ) -> Result<Vec<ItemStackSummary>, SlotDisplayResolveError> {
        let mut resolved = Vec::new();
        for item_id in item_ids {
            ensure_candidate_count(resolved.len().saturating_add(1))?;
            resolved.push(self.effective_item_stack(item_id));
        }
        Ok(resolved)
    }

    fn effective_item_stack(&self, item_id: i32) -> ItemStackSummary {
        self.world
            .effective_item_stack(&ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            })
            .into_owned()
    }

    fn dye_color_id(&self, dye: &ItemStackSummary) -> i32 {
        if self
            .world
            .item_stack_has_effective_component(dye, VANILLA_DATA_COMPONENT_DYE_TYPE_ID)
        {
            dye.component_patch
                .dye_color_id
                .unwrap_or(VANILLA_DYE_COLOR_WHITE_ID)
        } else {
            VANILLA_DYE_COLOR_WHITE_ID
        }
    }

    fn apply_smithing_trim(
        &self,
        base: &ItemStackSummary,
        material: &ItemStackSummary,
        pattern: &SlotDisplayTrimPatternSummary,
    ) -> Result<Option<ItemStackSummary>, SlotDisplayResolveError> {
        if !self.world.item_stack_has_effective_component(
            material,
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        ) {
            return Ok(None);
        }
        let material_holder = trim_material_holder(self.world, &material.component_patch)?;
        if trim_matches(base, &material_holder, pattern) {
            return Ok(None);
        }

        let mut trimmed = base.clone();
        trimmed.count = 1;
        set_component(
            &mut trimmed.component_patch,
            VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        );
        trimmed.component_patch.armor_trim_material_id = material_holder.registry_id;
        trimmed.component_patch.armor_trim_material_direct = material_holder.direct;
        trimmed.component_patch.armor_trim_pattern_id = pattern.registry_id;
        trimmed.component_patch.armor_trim_pattern_direct = pattern.direct.clone().map(Box::new);
        Ok(Some(trimmed))
    }
}

fn with_potion(mut stack: ItemStackSummary, potion_id: i32) -> ItemStackSummary {
    set_component(
        &mut stack.component_patch,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    );
    stack.component_patch.potion_id = Some(potion_id);
    stack.component_patch.potion_custom_color = None;
    stack.component_patch.potion_custom_effect_count = Some(0);
    stack.component_patch.potion_custom_effects.clear();
    stack.component_patch.potion_custom_name = None;
    stack
}

struct TrimMaterialHolder {
    registry_id: Option<i32>,
    direct: Option<Box<TrimMaterialSummary>>,
}

fn trim_material_holder(
    world: &WorldStore,
    patch: &DataComponentPatchSummary,
) -> Result<TrimMaterialHolder, SlotDisplayResolveError> {
    validate_trim_material(
        world,
        patch.provides_trim_material_id,
        patch.provides_trim_material_direct.as_deref(),
    )
}

fn validate_trim_material(
    world: &WorldStore,
    registry_id: Option<i32>,
    direct: Option<&TrimMaterialSummary>,
) -> Result<TrimMaterialHolder, SlotDisplayResolveError> {
    if let Some(direct) = direct {
        if direct_trim_material_presentation(direct).is_none() {
            return Err(SlotDisplayResolveError::InvalidDirectTrimMaterial);
        }
        return Ok(TrimMaterialHolder {
            registry_id: None,
            direct: Some(Box::new(direct.clone())),
        });
    }
    let Some(holder_id) = registry_id else {
        return Err(SlotDisplayResolveError::InvalidDirectTrimMaterial);
    };
    if world
        .synchronized_trim_material_presentation(holder_id)
        .map_err(|_| SlotDisplayResolveError::MissingTrimMaterial { holder_id })?
        .is_none()
    {
        return Err(SlotDisplayResolveError::MissingTrimMaterial { holder_id });
    }
    Ok(TrimMaterialHolder {
        registry_id: Some(holder_id),
        direct: None,
    })
}

fn validate_trim_pattern(
    world: &WorldStore,
    pattern: &SlotDisplayTrimPatternSummary,
) -> Result<(), SlotDisplayResolveError> {
    if let Some(direct) = pattern.direct.as_ref() {
        return if direct_trim_pattern_presentation(direct).is_some() {
            Ok(())
        } else {
            Err(SlotDisplayResolveError::InvalidDirectTrimPattern)
        };
    }
    let Some(holder_id) = pattern.registry_id else {
        return Err(SlotDisplayResolveError::InvalidDirectTrimPattern);
    };
    if world
        .synchronized_trim_pattern_presentation(holder_id)
        .map_err(|_| SlotDisplayResolveError::MissingTrimPattern { holder_id })?
        .is_none()
    {
        return Err(SlotDisplayResolveError::MissingTrimPattern { holder_id });
    }
    Ok(())
}

fn trim_matches(
    base: &ItemStackSummary,
    material: &TrimMaterialHolder,
    pattern: &SlotDisplayTrimPatternSummary,
) -> bool {
    !base
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_TRIM_TYPE_ID)
        && base
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_TRIM_TYPE_ID)
        && base.component_patch.armor_trim_material_id == material.registry_id
        && base.component_patch.armor_trim_material_direct == material.direct
        && base.component_patch.armor_trim_pattern_id == pattern.registry_id
        && base.component_patch.armor_trim_pattern_direct.as_deref() == pattern.direct.as_ref()
}

fn set_component(patch: &mut DataComponentPatchSummary, component_type_id: i32) {
    patch
        .removed_type_ids
        .retain(|removed| *removed != component_type_id);
    if !patch.added_type_ids.contains(&component_type_id) {
        patch.added_type_ids.push(component_type_id);
    }
    patch.added = patch.added_type_ids.len();
}

fn apply_dye_color(current: Option<i32>, dye_color_id: i32) -> i32 {
    let mut red_total = 0;
    let mut green_total = 0;
    let mut blue_total = 0;
    let mut intensity_total = 0;
    let mut color_count = 0;
    for color in [
        current.map(|color| color as u32),
        Some(vanilla_dye_color_texture_diffuse_color(dye_color_id)),
    ]
    .into_iter()
    .flatten()
    {
        let red = ((color >> 16) & 0xff) as i32;
        let green = ((color >> 8) & 0xff) as i32;
        let blue = (color & 0xff) as i32;
        intensity_total += red.max(green).max(blue);
        red_total += red;
        green_total += green;
        blue_total += blue;
        color_count += 1;
    }

    let mut red = red_total / color_count;
    let mut green = green_total / color_count;
    let mut blue = blue_total / color_count;
    let average_intensity = intensity_total as f32 / color_count as f32;
    let result_intensity = red.max(green).max(blue) as f32;
    red = (red as f32 * average_intensity / result_intensity) as i32;
    green = (green as f32 * average_intensity / result_intensity) as i32;
    blue = (blue as f32 * average_intensity / result_intensity) as i32;
    (red << 16) | (green << 8) | blue
}

fn ensure_candidate_count(actual: usize) -> Result<(), SlotDisplayResolveError> {
    if actual > MAX_RESOLVED_SLOT_DISPLAY_STACKS {
        return Err(SlotDisplayResolveError::TooManyCandidates { actual });
    }
    Ok(())
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

fn child_seed(seed: u64, branch: u64, index: u64) -> u64 {
    [branch.to_le_bytes(), index.to_le_bytes()]
        .into_iter()
        .flatten()
        .fold(seed, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
        })
}

struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    const MASK: u64 = (1u64 << 48) - 1;
    const MULTIPLIER: u64 = 25_214_903_917;
    const INCREMENT: u64 = 11;

    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    fn next(&mut self, bits: u8) -> i32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT)
            & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }

    fn next_int(&mut self, bound: usize) -> usize {
        let bound = i32::try_from(bound).expect("slot display candidate bound fits i32");
        if (bound & (bound - 1)) == 0 {
            return ((i64::from(bound) * i64::from(self.next(31))) >> 31) as usize;
        }
        loop {
            let sample = self.next(31);
            let modulo = sample % bound;
            if sample.wrapping_sub(modulo).wrapping_add(bound - 1) >= 0 {
                return modulo as usize;
            }
        }
    }
}

#[cfg(test)]
mod tests;
