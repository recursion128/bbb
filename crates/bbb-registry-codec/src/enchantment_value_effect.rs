//! Shared enchantment value-effect projection and calculation.

use bbb_protocol::{nbt_summary::compound_field as nbt_field, NbtSummaryValue};

use crate::{
    legacy_random::LegacyRandomSource, level_based_value::LevelBasedValue,
    random_source::RandomSource, RegistryJsonValue,
};

/// Vanilla `EnchantmentValueEffect` variants.
#[derive(Debug, Clone, PartialEq)]
pub enum EnchantmentValueEffect {
    Add(LevelBasedValue),
    AllOf(Vec<Self>),
    Multiply(LevelBasedValue),
    RemoveBinomial(LevelBasedValue),
    Exponential {
        base: LevelBasedValue,
        exponent: LevelBasedValue,
    },
    Set(LevelBasedValue),
}

impl EnchantmentValueEffect {
    pub fn parse(effect: &RegistryJsonValue) -> Result<Self, String> {
        let effect_type = effect
            .get("type")
            .and_then(RegistryJsonValue::utf8_string)
            .map(bbb_protocol::normalize_resource_location)
            .transpose()
            .map_err(|error| error.to_string())?;
        let level_value = |kind: &str, name: &str| {
            LevelBasedValue::parse(
                effect
                    .get(name)
                    .ok_or_else(|| format!("{kind} has no {name}"))?,
            )
        };
        match effect_type.as_deref() {
            Some("minecraft:add") => Ok(Self::Add(level_value("add", "value")?)),
            Some("minecraft:remove_binomial") => Ok(Self::RemoveBinomial(level_value(
                "remove_binomial",
                "chance",
            )?)),
            Some("minecraft:set") => Ok(Self::Set(level_value("set", "value")?)),
            Some("minecraft:multiply") => Ok(Self::Multiply(level_value("multiply", "factor")?)),
            Some("minecraft:exponential") => Ok(Self::Exponential {
                base: level_value("exponential", "base")?,
                exponent: level_value("exponential", "exponent")?,
            }),
            Some("minecraft:all_of") => Ok(Self::AllOf(
                effect
                    .get("effects")
                    .and_then(RegistryJsonValue::as_array)
                    .ok_or_else(|| "all_of has no effects list".to_string())?
                    .iter()
                    .map(Self::parse)
                    .collect::<Result<_, _>>()?,
            )),
            other => Err(format!("effect type {other:?} has no owner in this server")),
        }
    }

    /// Projects an effect produced by the exact value-effect source codec.
    pub fn from_canonical(value: &NbtSummaryValue) -> Self {
        let NbtSummaryValue::Compound(fields) = value else {
            unreachable!("canonical value effect");
        };
        let field = |name| nbt_field(fields, name).expect("canonical value effect field");
        let NbtSummaryValue::String(effect_type) = field("type") else {
            unreachable!("canonical value effect type");
        };
        effect_type
            .with_utf8(|effect_type| match effect_type {
                "minecraft:add" => Self::Add(LevelBasedValue::from_canonical(field("value"))),
                "minecraft:all_of" => {
                    let NbtSummaryValue::List(effects) = field("effects") else {
                        unreachable!("canonical value effects");
                    };
                    Self::AllOf(effects.iter().map(Self::from_canonical).collect())
                }
                "minecraft:multiply" => {
                    Self::Multiply(LevelBasedValue::from_canonical(field("factor")))
                }
                "minecraft:remove_binomial" => {
                    Self::RemoveBinomial(LevelBasedValue::from_canonical(field("chance")))
                }
                "minecraft:exponential" => Self::Exponential {
                    base: LevelBasedValue::from_canonical(field("base")),
                    exponent: LevelBasedValue::from_canonical(field("exponent")),
                },
                "minecraft:set" => Self::Set(LevelBasedValue::from_canonical(field("value"))),
                _ => unreachable!("canonical value effect type"),
            })
            .expect("canonical value effect identifier")
    }

    pub fn requires_random(&self) -> bool {
        match self {
            Self::RemoveBinomial(_) => true,
            Self::AllOf(effects) => effects.iter().any(Self::requires_random),
            _ => false,
        }
    }

    /// Refuses effects requiring a victim random when the caller lacks one.
    pub fn process_without_draw(&self, level: i32, input: f32) -> Result<f32, String> {
        if self.requires_random() {
            return Err(
                "remove_binomial needs the victim's random, which this caller lacks".into(),
            );
        }
        Ok(self.process::<LegacyRandomSource>(level, input, &mut None))
    }

    #[inline]
    pub fn process<R: RandomSource + ?Sized>(
        &self,
        level: i32,
        input: f32,
        random: &mut Option<&mut R>,
    ) -> f32 {
        match self {
            Self::Add(value) => input + value.calculate(level),
            Self::AllOf(effects) => effects
                .iter()
                .fold(input, |value, effect| effect.process(level, value, random)),
            Self::Multiply(factor) => input * factor.calculate(level),
            Self::RemoveBinomial(chance) => random.as_deref_mut().map_or(input, |random| {
                remove_binomial(chance.calculate(level), input, random)
            }),
            // `ScaleExponentially.process`.
            Self::Exponential { base, exponent } => {
                (f64::from(input)
                    * f64::from(base.calculate(level)).powf(f64::from(exponent.calculate(level))))
                    as f32
            }
            Self::Set(value) => value.calculate(level),
        }
    }
}

const MAX_EXACT_BINOMIAL_TRIALS: f32 = 4096.0;
const MAX_POISSON_ITERATIONS: u32 = 1024;

/// `RemoveBinomial.process` (`RemoveBinomial.java:14-31`).
#[inline]
fn remove_binomial<R: RandomSource + ?Sized>(chance: f32, n: f32, random: &mut R) -> f32 {
    if !n.is_finite() || n <= 0.0 || chance.is_nan() || chance <= 0.0 {
        return n;
    }
    if chance >= 1.0 {
        return n - n.ceil();
    }
    let nf = f64::from(n);
    let pf = f64::from(chance);
    let mean_drop = nf * pf;
    let mean_keep = nf * (1.0 - pf);
    if n > 128.0 && mean_drop >= 20.0 && mean_keep >= 20.0 {
        let mean = mean_drop.floor();
        let sigma = (mean_drop * (1.0 - pf)).sqrt();
        let drop = (mean + random.next_gaussian() * sigma + 0.5)
            .floor()
            .clamp(0.0, nf.floor());
        return (nf - drop) as f32;
    }
    if n <= MAX_EXACT_BINOMIAL_TRIALS {
        let trials = n.ceil() as u32;
        let mut drop = 0_u32;
        for _ in 0..trials {
            drop += u32::from(random.next_float() < chance);
        }
        return n - drop as f32;
    }
    // Large `n` with small tail mean (< 20): approximate Binomial(n, p) with Poisson(lambda).
    if mean_drop < 20.0 {
        let drop = sample_poisson_small(mean_drop, random);
        (nf - f64::from(drop)).max(0.0) as f32
    } else {
        let keep = sample_poisson_small(mean_keep, random);
        (nf - nf.ceil() + f64::from(keep)).min(nf) as f32
    }
}

fn sample_poisson_small<R: RandomSource + ?Sized>(lambda: f64, random: &mut R) -> u32 {
    if lambda <= 0.0 {
        return 0;
    }
    let threshold = (-lambda).exp();
    let mut k = 0_u32;
    let mut product = 1.0_f64;
    while k < MAX_POISSON_ITERATIONS {
        product *= f64::from(random.next_float());
        if product <= threshold {
            break;
        }
        k += 1;
    }
    k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_binomial_handles_huge_n_and_degenerate_chances_without_overflow() {
        let mut rng = LegacyRandomSource::with_seed(42);
        assert_eq!(remove_binomial(0.0, 1e10, &mut rng), 1e10);
        assert_eq!(remove_binomial(-0.5, 1e10, &mut rng), 1e10);
        assert_eq!(remove_binomial(f32::NAN, 1e10, &mut rng), 1e10);
        assert_eq!(remove_binomial(1.0, 1e10, &mut rng), 0.0);
        assert_eq!(remove_binomial(2.0, 1e10, &mut rng), 0.0);
        assert_eq!(remove_binomial(f32::INFINITY, 1e10, &mut rng), 0.0);
        assert_eq!(remove_binomial(0.5, 0.0, &mut rng), 0.0);
        assert_eq!(remove_binomial(0.5, -3.0, &mut rng), -3.0);
        assert!(remove_binomial(0.5, f32::NAN, &mut rng).is_nan());
        assert_eq!(remove_binomial(0.5, f32::INFINITY, &mut rng), f32::INFINITY);

        // Large n with rare event (n * chance < 20) must finish fast via Poisson approximation.
        let rare_drop = remove_binomial(1e-10, 1e10, &mut rng);
        assert!((0.0..=1e10).contains(&rare_drop));

        // Large n with chance near 1 (n * (1 - chance) < 20) must finish fast via tail Poisson.
        let rare_keep = remove_binomial(1.0 - 1e-7, 1e8, &mut rng);
        assert!((0.0..=1e8).contains(&rare_keep));

        // Large n > i32::MAX with mid chance must not truncate drop at i32::MAX.
        let mid = remove_binomial(0.5, 1e10, &mut rng);
        assert!((4.9e9..=5.1e9).contains(&mid), "unexpected mid={mid}");
    }
}
