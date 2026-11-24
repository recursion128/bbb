use super::*;
use bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_TEXTURE_DIFFUSE_COLORS;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID as BANNER_PATTERNS,
    VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID as CONTAINER_LOOT,
    VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID as DYED_COLOR,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

impl Eval<'_> {
    /// `append` keeps the held layers in front.
    pub(super) fn apply_banner_pattern(
        &mut self,
        patterns: &[NbtSummaryValue],
        append: bool,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let mut entries = if append {
            match self.at(path, environment.component_runtime(&stack, BANNER_PATTERNS))? {
                Some(NbtSummaryValue::List(held)) => held,
                None => Vec::new(),
                Some(_) => return Err(self.err(path, malformed("set_banner_pattern", "component"))),
            }
        } else {
            Vec::new()
        };
        entries.extend_from_slice(patterns);
        let value = NbtSummaryValue::List(entries);
        self.at(
            path,
            environment.with_component_runtime(stack, BANNER_PATTERNS, Some(&value)),
        )
        .map(RuntimeItemStack::into_summary)
    }

    pub(super) fn apply_container_loot(
        &mut self,
        name: &str,
        seed: i64,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        if stack_count(&stack) == 0 {
            return Ok(stack);
        }
        let value = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "loot_table".into(),
                value: NbtSummaryValue::string(name),
            },
            NbtSummaryEntry {
                name: "seed".into(),
                value: NbtSummaryValue::Long(seed),
            },
        ]);
        self.write_component(stack, CONTAINER_LOOT, Some(&value), path)
    }

    /// The count is evaluated and every dye drawn before the stack is read.
    pub(super) fn apply_random_dyes(
        &mut self,
        dyes: &NumberProvider,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let rolls = self.number_int(dyes, &format!("{path}.number_of_dyes"))?;
        if rolls <= 0 {
            return Ok(stack);
        }
        let mut channels = [0i32; 3];
        let mut intensity = 0i32;
        for _ in 0..rolls {
            let selected = self
                .random
                .next_int(VANILLA_DYE_COLOR_TEXTURE_DIFFUSE_COLORS.len() as i32);
            accumulate_color(
                VANILLA_DYE_COLOR_TEXTURE_DIFFUSE_COLORS[selected as usize] as i32,
                &mut channels,
                &mut intensity,
            );
        }
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let mut count = rolls;
        if let Some(old) = self.at(path, environment.component_runtime(&stack, DYED_COLOR))? {
            let old = bbb_protocol::nbt_summary::codec_int_value(&old)
                .ok_or_else(|| self.err(path, malformed("set_random_dyes", "component")))?;
            accumulate_color(old, &mut channels, &mut intensity);
            count = count.wrapping_add(1);
        }
        // `DyedItemColor.applyDyes`: channel averages truncate before the
        // float intensity scale.
        let mut channels = channels.map(|channel| channel / count);
        let average = intensity as f32 / count as f32;
        let maximum = *channels.iter().max().expect("RGB channels") as f32;
        for channel in &mut channels {
            *channel = (*channel as f32 * average / maximum) as i32;
        }
        let rgb = ((channels[0] & 255) << 16) | ((channels[1] & 255) << 8) | (channels[2] & 255);
        // `copyWithCount(1)` drops a hidden empty backing.
        let stack = stack.with_count(1);
        self.at(
            path,
            environment.with_component_runtime(stack, DYED_COLOR, Some(&NbtSummaryValue::Int(rgb))),
        )
        .map(RuntimeItemStack::into_summary)
    }
}

fn accumulate_color(color: i32, channels: &mut [i32; 3], intensity: &mut i32) {
    let values = [(color >> 16) & 255, (color >> 8) & 255, color & 255];
    for (channel, value) in channels.iter_mut().zip(values) {
        *channel = channel.wrapping_add(value);
    }
    *intensity = intensity.wrapping_add(*values.iter().max().expect("RGB channels"));
}
