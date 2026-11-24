//! Source-decoded noise settings, parameters and spawn climate ranges.

use super::density::DensityHolder;
use super::json::{self, fail, At, Obj, Result};
use super::surface::SurfaceRule;
use super::{value, Holder, Refs};
use bbb_registry_codec::registry_codec::DimensionTypeScalars;
use bbb_registry_codec::RegistryJsonValue as Json;

pub const NOISE_REGISTRY: &str = "minecraft:worldgen/noise";
pub const NOISE_SETTINGS_REGISTRY: &str = "minecraft:worldgen/noise_settings";
pub const ROUTER_FIELDS: [&str; 15] = [
    "barrier",
    "fluid_level_floodedness",
    "fluid_level_spread",
    "lava",
    "temperature",
    "vegetation",
    "continents",
    "erosion",
    "depth",
    "ridges",
    "preliminary_surface_level",
    "final_density",
    "vein_toggle",
    "vein_ridged",
    "vein_gap",
];
pub type NoiseHolder = Holder<NoiseParameters>;

#[derive(Debug, Clone, PartialEq)]
pub struct NoiseParameters {
    pub first_octave: i32,
    pub amplitudes: Vec<f64>,
}
impl NoiseParameters {
    pub fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        let amplitudes = obj.req("amplitudes", |v| json::bounded_list(v, 1, 64, json::double))?;
        Ok(Self {
            first_octave: obj.req("firstOctave", json::int)?,
            amplitudes,
        })
    }
}
impl NoiseHolder {
    pub fn decode_noise(value: &Json) -> Result<Self> {
        Self::decode(value, NoiseParameters::decode)
    }
    pub(super) fn refs(&self, refs: &mut Refs) {
        if let Self::Named(id) = self {
            refs.element(NOISE_REGISTRY, id);
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseSettings {
    pub min_y: i32,
    pub height: i32,
    pub size_horizontal: i32,
    pub size_vertical: i32,
}
impl NoiseSettings {
    fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        let settings = Self {
            min_y: obj.req("min_y", |v| {
                json::int_in(v, DimensionTypeScalars::MIN_Y, DimensionTypeScalars::MAX_Y)
            })?,
            height: obj.req("height", |v| {
                json::int_in(v, 0, DimensionTypeScalars::Y_SIZE)
            })?,
            size_horizontal: obj.req("size_horizontal", |v| json::int_in(v, 1, 4))?,
            size_vertical: obj.req("size_vertical", |v| json::int_in(v, 1, 4))?,
        };
        if settings.min_y + settings.height > DimensionTypeScalars::MAX_Y + 1 {
            return fail("min_y + height exceeds dimension maximum");
        }
        if settings.height % 16 != 0 || settings.min_y % 16 != 0 {
            return fail("height and min_y must be multiples of 16");
        }
        Ok(settings)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnTarget {
    pub ranges: [[f32; 2]; 6],
    pub offset: f32,
}
impl SpawnTarget {
    // `Climate.java:98-108` and `Climate.java:218-228` decode source floats before quantization.
    fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        let mut ranges = [[0.0; 2]; 6];
        for (range, field) in ranges.iter_mut().zip([
            "temperature",
            "humidity",
            "continentalness",
            "erosion",
            "depth",
            "weirdness",
        ]) {
            *range = obj.req(field, climate_parameter)?;
        }
        Ok(Self {
            ranges,
            offset: obj.req("offset", |v| json::float_in(v, 0.0, 1.0))?,
        })
    }
}
fn climate_parameter(value: &Json) -> Result<[f32; 2]> {
    let number = |v: &Json| json::float_in(v, -2.0, 2.0);
    let range = match value {
        Json::Number(_) => {
            let point = number(value)?;
            [point, point]
        }
        Json::Array(values) if values.len() == 2 => [
            number(&values[0]).at_index(0)?,
            number(&values[1]).at_index(1)?,
        ],
        _ => {
            let obj = Obj::of(value)?;
            [obj.req("min", number)?, obj.req("max", number)?]
        }
    };
    if range[0].total_cmp(&range[1]).is_gt() {
        return fail("climate interval minimum exceeds maximum");
    }
    Ok(range)
}
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseGeneratorSettings {
    pub noise: NoiseSettings,
    pub default_block: i32,
    pub default_fluid: i32,
    pub noise_router: [DensityHolder; 15],
    pub surface_rule: SurfaceRule,
    pub spawn_target: Vec<SpawnTarget>,
    pub sea_level: i32,
    pub disable_mob_generation: bool,
    pub aquifers_enabled: bool,
    pub ore_veins_enabled: bool,
    pub legacy_random_source: bool,
}
impl NoiseGeneratorSettings {
    // `NoiseGeneratorSettings.java:32-48`: eleven fields, including the entire router, are required.
    pub fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        let noise_router = obj.req("noise_router", |v| {
            let router = Obj::of(v)?;
            ROUTER_FIELDS
                .iter()
                .map(|field| router.req(field, DensityHolder::decode_density))
                .collect::<Result<Vec<_>>>()?
                .try_into()
                .map_err(|_| unreachable!())
        })?;
        Ok(Self {
            noise: obj.req("noise", NoiseSettings::decode)?,
            default_block: obj.req("default_block", value::block_state)?,
            default_fluid: obj.req("default_fluid", value::block_state)?,
            noise_router,
            surface_rule: obj.req("surface_rule", SurfaceRule::decode)?,
            spawn_target: obj.req("spawn_target", |v| json::list(v, SpawnTarget::decode))?,
            sea_level: obj.req("sea_level", json::int)?,
            disable_mob_generation: obj.req("disable_mob_generation", json::boolean)?,
            aquifers_enabled: obj.req("aquifers_enabled", json::boolean)?,
            ore_veins_enabled: obj.req("ore_veins_enabled", json::boolean)?,
            legacy_random_source: obj.req("legacy_random_source", json::boolean)?,
        })
    }
    pub(super) fn refs(&self, refs: &mut Refs) {
        for holder in &self.noise_router {
            holder.refs(refs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NoiseParameters;

    #[test]
    fn noise_parameters_reject_empty_amplitudes() {
        let empty =
            bbb_registry_codec::parse_registry_json(br#"{"firstOctave":-4,"amplitudes":[]}"#)
                .unwrap();
        assert!(NoiseParameters::decode(&empty).is_err());

        let zero =
            bbb_registry_codec::parse_registry_json(br#"{"firstOctave":-4,"amplitudes":[0.0]}"#)
                .unwrap();
        assert_eq!(
            NoiseParameters::decode(&zero).unwrap().amplitudes,
            vec![0.0]
        );
    }
}
