//! `OverworldBiomeBuilder`: the Overworld preset's climate parameter points,
//! emitted in Vanilla's order (the order fixes R-tree ties and the possible
//! biome order).

use std::sync::OnceLock;

/// `[min, max]` of one quantized climate parameter.
type Span = [i64; 2];

pub struct OverworldPoint {
    /// Temperature, humidity, continentalness, erosion, depth, weirdness.
    pub ranges: [Span; 6],
    pub biome: &'static str,
}

macro_rules! b {
    ($name:ident) => {
        concat!("minecraft:", stringify!($name))
    };
}

/// `Climate.quantizeCoord`.
fn q(value: f32) -> i64 {
    (value * 10000.0) as i64
}

fn span(min: f32, max: f32) -> Span {
    [q(min), q(max)]
}

fn point(value: f32) -> Span {
    span(value, value)
}

const fn join(min: Span, max: Span) -> Span {
    [min[0], max[1]]
}

type Grid = [[Option<&'static str>; 5]; 5];

#[rustfmt::skip]
const MIDDLE: [[&str; 5]; 5] = [
    [b!(snowy_plains), b!(snowy_plains), b!(snowy_plains), b!(snowy_taiga), b!(taiga)],
    [b!(plains), b!(plains), b!(forest), b!(taiga), b!(old_growth_spruce_taiga)],
    [b!(flower_forest), b!(plains), b!(forest), b!(birch_forest), b!(dark_forest)],
    [b!(savanna), b!(savanna), b!(forest), b!(jungle), b!(jungle)],
    [b!(desert), b!(desert), b!(desert), b!(desert), b!(desert)],
];
#[rustfmt::skip]
const MIDDLE_VARIANT: Grid = [
    [Some(b!(ice_spikes)), None, Some(b!(snowy_taiga)), None, None],
    [None, None, None, None, Some(b!(old_growth_pine_taiga))],
    [Some(b!(sunflower_plains)), None, None, Some(b!(old_growth_birch_forest)), None],
    [None, None, Some(b!(plains)), Some(b!(sparse_jungle)), Some(b!(bamboo_jungle))],
    [None, None, None, None, None],
];
#[rustfmt::skip]
const PLATEAU: [[&str; 5]; 5] = [
    [b!(snowy_plains), b!(snowy_plains), b!(snowy_plains), b!(snowy_taiga), b!(snowy_taiga)],
    [b!(meadow), b!(meadow), b!(forest), b!(taiga), b!(old_growth_spruce_taiga)],
    [b!(meadow), b!(meadow), b!(meadow), b!(meadow), b!(pale_garden)],
    [b!(savanna_plateau), b!(savanna_plateau), b!(forest), b!(forest), b!(jungle)],
    [b!(badlands), b!(badlands), b!(badlands), b!(wooded_badlands), b!(wooded_badlands)],
];
#[rustfmt::skip]
const PLATEAU_VARIANT: Grid = [
    [Some(b!(ice_spikes)), None, None, None, None],
    [Some(b!(cherry_grove)), None, Some(b!(meadow)), Some(b!(meadow)), Some(b!(old_growth_pine_taiga))],
    [Some(b!(cherry_grove)), Some(b!(cherry_grove)), Some(b!(forest)), Some(b!(birch_forest)), None],
    [None, None, None, None, None],
    [Some(b!(eroded_badlands)), Some(b!(eroded_badlands)), None, None, None],
];
#[rustfmt::skip]
const SHATTERED: Grid = [
    [Some(b!(windswept_gravelly_hills)), Some(b!(windswept_gravelly_hills)), Some(b!(windswept_hills)), Some(b!(windswept_forest)), Some(b!(windswept_forest))],
    [Some(b!(windswept_gravelly_hills)), Some(b!(windswept_gravelly_hills)), Some(b!(windswept_hills)), Some(b!(windswept_forest)), Some(b!(windswept_forest))],
    [Some(b!(windswept_hills)), Some(b!(windswept_hills)), Some(b!(windswept_hills)), Some(b!(windswept_forest)), Some(b!(windswept_forest))],
    [None, None, None, None, None],
    [None, None, None, None, None],
];
#[rustfmt::skip]
const OCEANS: [[&str; 5]; 2] = [
    [b!(deep_frozen_ocean), b!(deep_cold_ocean), b!(deep_ocean), b!(deep_lukewarm_ocean), b!(warm_ocean)],
    [b!(frozen_ocean), b!(cold_ocean), b!(ocean), b!(lukewarm_ocean), b!(warm_ocean)],
];

struct Builder {
    out: Vec<OverworldPoint>,
    full: Span,
    temperatures: [Span; 5],
    humidities: [Span; 5],
    erosions: [Span; 7],
    coast: Span,
    inland: Span,
    near_inland: Span,
    mid_inland: Span,
    far_inland: Span,
}

impl Builder {
    fn new() -> Self {
        Self {
            out: Vec::new(),
            full: span(-1.0, 1.0),
            temperatures: [
                span(-1.0, -0.45),
                span(-0.45, -0.15),
                span(-0.15, 0.2),
                span(0.2, 0.55),
                span(0.55, 1.0),
            ],
            humidities: [
                span(-1.0, -0.35),
                span(-0.35, -0.1),
                span(-0.1, 0.1),
                span(0.1, 0.3),
                span(0.3, 1.0),
            ],
            erosions: [
                span(-1.0, -0.78),
                span(-0.78, -0.375),
                span(-0.375, -0.2225),
                span(-0.2225, 0.05),
                span(0.05, 0.45),
                span(0.45, 0.55),
                span(0.55, 1.0),
            ],
            coast: span(-0.19, -0.11),
            inland: span(-0.11, 0.55),
            near_inland: span(-0.11, 0.03),
            mid_inland: span(0.03, 0.3),
            far_inland: span(0.3, 1.0),
        }
    }

    fn push(&mut self, ranges: [Span; 6], biome: &'static str) {
        self.out.push(OverworldPoint { ranges, biome });
    }

    /// `addSurfaceBiome`: one point at the surface and one at depth 1.
    fn surface(&mut self, t: Span, h: Span, c: Span, e: Span, w: Span, biome: &'static str) {
        self.push([t, h, c, e, point(0.0), w], biome);
        self.push([t, h, c, e, point(1.0), w], biome);
    }

    /// `OverworldBiomeBuilder.java:194-224`.
    fn add_biomes(&mut self) {
        let full = self.full;
        self.surface(
            full,
            full,
            span(-1.2, -1.05),
            full,
            full,
            b!(mushroom_fields),
        );
        for (t_index, t) in self.temperatures.into_iter().enumerate() {
            self.surface(t, full, span(-1.05, -0.455), full, full, OCEANS[0][t_index]);
            self.surface(t, full, span(-0.455, -0.19), full, full, OCEANS[1][t_index]);
        }

        self.mid_slice(span(-1.0, -0.933_333_34));
        self.high_slice(span(-0.933_333_34, -0.766_666_7));
        self.peaks(span(-0.766_666_7, -0.566_666_66));
        self.high_slice(span(-0.566_666_66, -0.4));
        self.mid_slice(span(-0.4, -0.266_666_68));
        self.low_slice(span(-0.266_666_68, -0.05));
        self.valleys(span(-0.05, 0.05));
        self.low_slice(span(0.05, 0.266_666_68));
        self.mid_slice(span(0.266_666_68, 0.4));
        self.high_slice(span(0.4, 0.566_666_66));
        self.peaks(span(0.566_666_66, 0.766_666_7));
        self.high_slice(span(0.766_666_7, 0.933_333_34));
        self.mid_slice(span(0.933_333_34, 1.0));

        let e = self.erosions;
        self.push(
            [full, full, span(0.8, 1.0), full, span(0.2, 0.9), full],
            b!(dripstone_caves),
        );
        self.push(
            [full, span(0.7, 1.0), full, full, span(0.2, 0.9), full],
            b!(lush_caves),
        );
        self.push(
            [full, full, full, join(e[0], e[1]), point(1.1), full],
            b!(deep_dark),
        );
    }

    fn for_each_climate(&mut self, mut add: impl FnMut(&mut Self, usize, usize, Span, Span)) {
        for (t_index, t) in self.temperatures.into_iter().enumerate() {
            for (h_index, h) in self.humidities.into_iter().enumerate() {
                add(self, t_index, h_index, t, h);
            }
        }
    }

    fn peaks(&mut self, w: Span) {
        let (e, coast, near, mid, far) = self.continents();
        self.for_each_climate(|s, ti, hi, t, h| {
            let middle = middle_biome(ti, hi, w);
            let middle_or_badlands = middle_or_badlands_if_hot(ti, hi, w);
            let middle_or_badlands_or_slope = middle_or_badlands_or_slope(ti, hi, w);
            let plateau = plateau_biome(ti, hi, w);
            let shattered = shattered_biome(ti, hi, w);
            let shattered_or_savanna = windswept_savanna_or(ti, hi, w, shattered);
            let peak = peak_biome(ti, hi, w);
            s.surface(t, h, join(coast, far), e[0], w, peak);
            s.surface(
                t,
                h,
                join(coast, near),
                e[1],
                w,
                middle_or_badlands_or_slope,
            );
            s.surface(t, h, join(mid, far), e[1], w, peak);
            s.surface(t, h, join(coast, near), join(e[2], e[3]), w, middle);
            s.surface(t, h, join(mid, far), e[2], w, plateau);
            s.surface(t, h, mid, e[3], w, middle_or_badlands);
            s.surface(t, h, far, e[3], w, plateau);
            s.surface(t, h, join(coast, far), e[4], w, middle);
            s.surface(t, h, join(coast, near), e[5], w, shattered_or_savanna);
            s.surface(t, h, join(mid, far), e[5], w, shattered);
            s.surface(t, h, join(coast, far), e[6], w, middle);
        });
    }

    fn high_slice(&mut self, w: Span) {
        let (e, coast, near, mid, far) = self.continents();
        self.for_each_climate(|s, ti, hi, t, h| {
            let middle = middle_biome(ti, hi, w);
            let middle_or_badlands = middle_or_badlands_if_hot(ti, hi, w);
            let middle_or_badlands_or_slope = middle_or_badlands_or_slope(ti, hi, w);
            let plateau = plateau_biome(ti, hi, w);
            let shattered = shattered_biome(ti, hi, w);
            let middle_or_savanna = windswept_savanna_or(ti, hi, w, middle);
            let slope = slope_biome(ti, hi, w);
            let peak = peak_biome(ti, hi, w);
            s.surface(t, h, coast, join(e[0], e[1]), w, middle);
            s.surface(t, h, near, e[0], w, slope);
            s.surface(t, h, join(mid, far), e[0], w, peak);
            s.surface(t, h, near, e[1], w, middle_or_badlands_or_slope);
            s.surface(t, h, join(mid, far), e[1], w, slope);
            s.surface(t, h, join(coast, near), join(e[2], e[3]), w, middle);
            s.surface(t, h, join(mid, far), e[2], w, plateau);
            s.surface(t, h, mid, e[3], w, middle_or_badlands);
            s.surface(t, h, far, e[3], w, plateau);
            s.surface(t, h, join(coast, far), e[4], w, middle);
            s.surface(t, h, join(coast, near), e[5], w, middle_or_savanna);
            s.surface(t, h, join(mid, far), e[5], w, shattered);
            s.surface(t, h, join(coast, far), e[6], w, middle);
        });
    }

    fn swamps(&mut self, w: Span, from: Span) {
        let (t, e6, far, full) = (
            self.temperatures,
            self.erosions[6],
            self.far_inland,
            self.full,
        );
        self.surface(join(t[1], t[2]), full, join(from, far), e6, w, b!(swamp));
        self.surface(
            join(t[3], t[4]),
            full,
            join(from, far),
            e6,
            w,
            b!(mangrove_swamp),
        );
    }

    fn mid_slice(&mut self, w: Span) {
        let (e, coast, near, mid, far) = self.continents();
        let full = self.full;
        self.surface(full, full, coast, join(e[0], e[2]), w, b!(stony_shore));
        self.swamps(w, near);
        let negative = w[1] < 0;
        self.for_each_climate(|s, ti, hi, t, h| {
            let middle = middle_biome(ti, hi, w);
            let middle_or_badlands = middle_or_badlands_if_hot(ti, hi, w);
            let middle_or_badlands_or_slope = middle_or_badlands_or_slope(ti, hi, w);
            let shattered = shattered_biome(ti, hi, w);
            let plateau = plateau_biome(ti, hi, w);
            let beach = beach_biome(ti);
            let middle_or_savanna = windswept_savanna_or(ti, hi, w, middle);
            let shattered_coast = shattered_coast_biome(ti, hi, w);
            let slope = slope_biome(ti, hi, w);
            s.surface(t, h, join(near, far), e[0], w, slope);
            s.surface(t, h, join(near, mid), e[1], w, middle_or_badlands_or_slope);
            s.surface(t, h, far, e[1], w, if ti == 0 { slope } else { plateau });
            s.surface(t, h, near, e[2], w, middle);
            s.surface(t, h, mid, e[2], w, middle_or_badlands);
            s.surface(t, h, far, e[2], w, plateau);
            s.surface(t, h, join(coast, near), e[3], w, middle);
            s.surface(t, h, join(mid, far), e[3], w, middle_or_badlands);
            if negative {
                s.surface(t, h, coast, e[4], w, beach);
                s.surface(t, h, join(near, far), e[4], w, middle);
            } else {
                s.surface(t, h, join(coast, far), e[4], w, middle);
            }
            s.surface(t, h, coast, e[5], w, shattered_coast);
            s.surface(t, h, near, e[5], w, middle_or_savanna);
            s.surface(t, h, join(mid, far), e[5], w, shattered);
            s.surface(t, h, coast, e[6], w, if negative { beach } else { middle });
            if ti == 0 {
                s.surface(t, h, join(near, far), e[6], w, middle);
            }
        });
    }

    fn low_slice(&mut self, w: Span) {
        let (e, coast, near, mid, far) = self.continents();
        let full = self.full;
        self.surface(full, full, coast, join(e[0], e[2]), w, b!(stony_shore));
        self.swamps(w, near);
        self.for_each_climate(|s, ti, hi, t, h| {
            let middle = middle_biome(ti, hi, w);
            let middle_or_badlands = middle_or_badlands_if_hot(ti, hi, w);
            let middle_or_badlands_or_slope = middle_or_badlands_or_slope(ti, hi, w);
            let beach = beach_biome(ti);
            let middle_or_savanna = windswept_savanna_or(ti, hi, w, middle);
            let shattered_coast = shattered_coast_biome(ti, hi, w);
            s.surface(t, h, near, join(e[0], e[1]), w, middle_or_badlands);
            s.surface(
                t,
                h,
                join(mid, far),
                join(e[0], e[1]),
                w,
                middle_or_badlands_or_slope,
            );
            s.surface(t, h, near, join(e[2], e[3]), w, middle);
            s.surface(
                t,
                h,
                join(mid, far),
                join(e[2], e[3]),
                w,
                middle_or_badlands,
            );
            s.surface(t, h, coast, join(e[3], e[4]), w, beach);
            s.surface(t, h, join(near, far), e[4], w, middle);
            s.surface(t, h, coast, e[5], w, shattered_coast);
            s.surface(t, h, near, e[5], w, middle_or_savanna);
            s.surface(t, h, join(mid, far), e[5], w, middle);
            s.surface(t, h, coast, e[6], w, beach);
            if ti == 0 {
                s.surface(t, h, join(near, far), e[6], w, middle);
            }
        });
    }

    fn valleys(&mut self, w: Span) {
        let (e, coast, near, mid, far) = self.continents();
        let (full, inland) = (self.full, self.inland);
        let frozen = self.temperatures[0];
        let unfrozen = join(self.temperatures[1], self.temperatures[4]);
        let negative = w[1] < 0;
        let (frozen_coast, unfrozen_coast) = if negative {
            (b!(stony_shore), b!(stony_shore))
        } else {
            (b!(frozen_river), b!(river))
        };
        self.surface(frozen, full, coast, join(e[0], e[1]), w, frozen_coast);
        self.surface(unfrozen, full, coast, join(e[0], e[1]), w, unfrozen_coast);
        self.surface(frozen, full, near, join(e[0], e[1]), w, b!(frozen_river));
        self.surface(unfrozen, full, near, join(e[0], e[1]), w, b!(river));
        self.surface(
            frozen,
            full,
            join(coast, far),
            join(e[2], e[5]),
            w,
            b!(frozen_river),
        );
        self.surface(
            unfrozen,
            full,
            join(coast, far),
            join(e[2], e[5]),
            w,
            b!(river),
        );
        self.surface(frozen, full, coast, e[6], w, b!(frozen_river));
        self.surface(unfrozen, full, coast, e[6], w, b!(river));
        self.swamps(w, inland);
        self.surface(frozen, full, join(inland, far), e[6], w, b!(frozen_river));
        self.for_each_climate(|s, ti, hi, t, h| {
            let biome = middle_or_badlands_if_hot(ti, hi, w);
            s.surface(t, h, join(mid, far), join(e[0], e[1]), w, biome);
        });
    }

    fn continents(&self) -> ([Span; 7], Span, Span, Span, Span) {
        (
            self.erosions,
            self.coast,
            self.near_inland,
            self.mid_inland,
            self.far_inland,
        )
    }
}

fn middle_biome(t: usize, h: usize, w: Span) -> &'static str {
    let base = MIDDLE[t][h];
    if w[1] < 0 {
        base
    } else {
        MIDDLE_VARIANT[t][h].unwrap_or(base)
    }
}

fn middle_or_badlands_if_hot(t: usize, h: usize, w: Span) -> &'static str {
    if t == 4 {
        badlands_biome(h, w)
    } else {
        middle_biome(t, h, w)
    }
}

fn middle_or_badlands_or_slope(t: usize, h: usize, w: Span) -> &'static str {
    if t == 0 {
        slope_biome(t, h, w)
    } else {
        middle_or_badlands_if_hot(t, h, w)
    }
}

fn windswept_savanna_or(t: usize, h: usize, w: Span, other: &'static str) -> &'static str {
    if t > 1 && h < 4 && w[1] >= 0 {
        b!(windswept_savanna)
    } else {
        other
    }
}

fn shattered_coast_biome(t: usize, h: usize, w: Span) -> &'static str {
    let base = if w[1] >= 0 {
        middle_biome(t, h, w)
    } else {
        beach_biome(t)
    };
    windswept_savanna_or(t, h, w, base)
}

fn beach_biome(t: usize) -> &'static str {
    match t {
        0 => b!(snowy_beach),
        4 => b!(desert),
        _ => b!(beach),
    }
}

fn badlands_biome(h: usize, w: Span) -> &'static str {
    if h < 2 {
        if w[1] < 0 {
            b!(badlands)
        } else {
            b!(eroded_badlands)
        }
    } else if h < 3 {
        b!(badlands)
    } else {
        b!(wooded_badlands)
    }
}

fn plateau_biome(t: usize, h: usize, w: Span) -> &'static str {
    match PLATEAU_VARIANT[t][h] {
        Some(variant) if w[1] >= 0 => variant,
        _ => PLATEAU[t][h],
    }
}

fn peak_biome(t: usize, h: usize, w: Span) -> &'static str {
    match t {
        0..=2 if w[1] < 0 => b!(jagged_peaks),
        0..=2 => b!(frozen_peaks),
        3 => b!(stony_peaks),
        _ => badlands_biome(h, w),
    }
}

fn slope_biome(t: usize, h: usize, w: Span) -> &'static str {
    if t >= 3 {
        plateau_biome(t, h, w)
    } else if h <= 1 {
        b!(snowy_slopes)
    } else {
        b!(grove)
    }
}

fn shattered_biome(t: usize, h: usize, w: Span) -> &'static str {
    SHATTERED[t][h].unwrap_or_else(|| middle_biome(t, h, w))
}

pub struct OverworldPreset {
    pub points: Vec<OverworldPoint>,
    /// Distinct biomes in first-appearance order.
    pub biome_order: Vec<&'static str>,
}

pub fn overworld_preset() -> &'static OverworldPreset {
    static PRESET: OnceLock<OverworldPreset> = OnceLock::new();
    PRESET.get_or_init(|| {
        let mut builder = Builder::new();
        builder.add_biomes();
        let mut biome_order = Vec::new();
        for point in &builder.out {
            if !biome_order.contains(&point.biome) {
                biome_order.push(point.biome);
            }
        }
        OverworldPreset {
            points: builder.out,
            biome_order,
        }
    })
}
