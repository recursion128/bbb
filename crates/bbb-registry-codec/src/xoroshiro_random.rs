//! `Xoroshiro128PlusPlus` and `XoroshiroRandomSource` (`RandomSupport` seed
//! arithmetic + generator used by loot tables and `RandomSequence`).

/// `RandomSupport.GOLDEN_RATIO_64` (`RandomSupport.java:11-12`).
pub const GOLDEN_RATIO_64: i64 = -7046029254386353131;
/// `RandomSupport.SILVER_RATIO_64` (`RandomSupport.java:11-12`).
pub const SILVER_RATIO_64: i64 = 7640891576956012809;

/// The two multipliers of `RandomSupport.mixStafford13`
/// (`RandomSupport.java:17-21`), written the way the Java literals are so the
/// transcription reads against the source.
const MIX_STAFFORD_13_MULTIPLIER_1: i64 = -4658895280553007687;
const MIX_STAFFORD_13_MULTIPLIER_2: i64 = -7723592293110705685;

/// `RandomSupport.Seed128bit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seed128bit {
    pub seed_lo: i64,
    pub seed_hi: i64,
}

impl Seed128bit {
    /// `RandomSupport.Seed128bit.xor(long, long)` (`RandomSupport.java:44-56`).
    #[must_use]
    pub const fn xor(self, lo: i64, hi: i64) -> Self {
        Self {
            seed_lo: self.seed_lo ^ lo,
            seed_hi: self.seed_hi ^ hi,
        }
    }

    /// `RandomSupport.Seed128bit.mixed()` (`RandomSupport.java:44-56`).
    #[must_use]
    pub const fn mixed(self) -> Self {
        Self {
            seed_lo: mix_stafford13(self.seed_lo),
            seed_hi: mix_stafford13(self.seed_hi),
        }
    }
}

/// `RandomSupport.mixStafford13` (`RandomSupport.java:17-21`).
///
/// Java's `>>>` is an unsigned shift, so the state travels as `u64` and only
/// the result is read back as the signed value the Java expression produces.
#[must_use]
pub const fn mix_stafford13(z: i64) -> i64 {
    let mut z = z as u64;
    z = (z ^ (z >> 30)).wrapping_mul(MIX_STAFFORD_13_MULTIPLIER_1 as u64);
    z = (z ^ (z >> 27)).wrapping_mul(MIX_STAFFORD_13_MULTIPLIER_2 as u64);
    (z ^ (z >> 31)) as i64
}

/// `RandomSupport.upgradeSeedTo128bitUnmixed` (`RandomSupport.java:23-27`).
#[must_use]
pub const fn upgrade_seed_to_128bit_unmixed(legacy_seed: i64) -> Seed128bit {
    let seed_lo = legacy_seed ^ SILVER_RATIO_64;
    let seed_hi = seed_lo.wrapping_add(GOLDEN_RATIO_64);
    Seed128bit { seed_lo, seed_hi }
}

/// `RandomSupport.upgradeSeedTo128bit` (`RandomSupport.java:29-31`).
#[must_use]
pub const fn upgrade_seed_to_128bit(legacy_seed: i64) -> Seed128bit {
    upgrade_seed_to_128bit_unmixed(legacy_seed).mixed()
}

/// `RandomSupport.seedFromHashOf`.
///
/// Guava's `Hashing.md5().hashString(input, UTF_8).asBytes()` is the plain MD5
/// digest, and `Longs.fromBytes` composes big-endian, so the two halves are the
/// digest's first and second eight bytes read big-endian.
#[must_use]
pub fn seed_from_hash_of(input: &str) -> Seed128bit {
    let digest = md5::compute(input.as_bytes());
    let (low, high) = digest.0.split_at(8);
    Seed128bit {
        seed_lo: i64::from_be_bytes(low.try_into().expect("an MD5 digest is sixteen bytes")),
        seed_hi: i64::from_be_bytes(high.try_into().expect("an MD5 digest is sixteen bytes")),
    }
}

/// `Xoroshiro128PlusPlus` (`Xoroshiro128PlusPlus.java:17-34`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xoroshiro128PlusPlus {
    seed_lo: i64,
    seed_hi: i64,
}

impl Xoroshiro128PlusPlus {
    /// `Xoroshiro128PlusPlus(long, long)` (`Xoroshiro128PlusPlus.java:17-24`),
    /// including the substitution that keeps the all-zero state — which
    /// xoroshiro128++ is a fixed point of — out of the generator.
    #[must_use]
    pub const fn new(seed_lo: i64, seed_hi: i64) -> Self {
        if (seed_lo | seed_hi) == 0 {
            return Self {
                seed_lo: GOLDEN_RATIO_64,
                seed_hi: SILVER_RATIO_64,
            };
        }
        Self { seed_lo, seed_hi }
    }

    /// `Xoroshiro128PlusPlus(RandomSupport.Seed128bit)`
    /// (`Xoroshiro128PlusPlus.java:17-24` through the delegating constructor).
    #[must_use]
    pub const fn from_seed_128(seed: Seed128bit) -> Self {
        Self::new(seed.seed_lo, seed.seed_hi)
    }

    /// The `(seedLo, seedHi)` pair `Xoroshiro128PlusPlus.CODEC` writes as its
    /// two-element long array (`Xoroshiro128PlusPlus.java:10-11`).
    #[must_use]
    pub const fn state(self) -> (i64, i64) {
        (self.seed_lo, self.seed_hi)
    }

    /// `Xoroshiro128PlusPlus.nextLong` (`Xoroshiro128PlusPlus.java:26-34`).
    pub fn next_long(&mut self) -> i64 {
        let s0 = self.seed_lo;
        let mut s1 = self.seed_hi;
        let result = s0.wrapping_add(s1).rotate_left(17).wrapping_add(s0);
        s1 ^= s0;
        self.seed_lo = s0.rotate_left(49) ^ s1 ^ (s1 << 21);
        self.seed_hi = s1.rotate_left(28);
        result
    }
}

/// `XoroshiroRandomSource.FLOAT_UNIT` (`XoroshiroRandomSource.java:9-10`).
const FLOAT_UNIT: f32 = 5.960_464_5E-8;

/// `XoroshiroRandomSource.DOUBLE_UNIT` (`XoroshiroRandomSource.java:9-10`).
///
/// The field is a `double` initialised from a `float` literal, so its value is
/// the widening of the nearest `float` and the constant `javac` inlines into
/// `nextDouble` is `1.1102230246251565E-16`. The decompiled body reads
/// `this.nextBits(53) * 1.110223E-16F`, which would make the multiply a *float*
/// multiply under Java's binary numeric promotion; the shipped bytecode is
/// `l2d; ldc2_w 1.1102230246251565E-16d; dmul`, so the `F` suffix in the
/// decompiled expression is an artifact and the arithmetic is `f64`.
const DOUBLE_UNIT: f64 = 1.110_223E-16_f32 as f64;

/// `XoroshiroRandomSource`.
///
/// See the module docs for the members this deliberately does not carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XoroshiroRandomSource {
    generator: Xoroshiro128PlusPlus,
}

impl XoroshiroRandomSource {
    /// `XoroshiroRandomSource(long seed)` (`XoroshiroRandomSource.java:16-18`).
    #[must_use]
    pub const fn with_seed(seed: i64) -> Self {
        Self::from_seed_128(upgrade_seed_to_128bit(seed))
    }

    /// `XoroshiroRandomSource(RandomSupport.Seed128bit)`
    /// (`XoroshiroRandomSource.java:20-22`).
    #[must_use]
    pub const fn from_seed_128(seed: Seed128bit) -> Self {
        Self {
            generator: Xoroshiro128PlusPlus::from_seed_128(seed),
        }
    }

    /// `XoroshiroRandomSource(long seedLo, long seedHi)`
    /// (`XoroshiroRandomSource.java:24-26`) — the constructor
    /// `XoroshiroRandomSource.CODEC` restores a saved generator through, which
    /// is why a persisted all-zero pair is substituted rather than kept.
    #[must_use]
    pub const fn from_state(seed_lo: i64, seed_hi: i64) -> Self {
        Self {
            generator: Xoroshiro128PlusPlus::new(seed_lo, seed_hi),
        }
    }

    /// The generator state `XoroshiroRandomSource.CODEC` writes
    /// (`XoroshiroRandomSource.java:11-12`).
    #[must_use]
    pub const fn state(&self) -> (i64, i64) {
        self.generator.state()
    }

    /// `XoroshiroRandomSource.setSeed`.
    pub fn set_seed(&mut self, seed: i64) {
        self.generator = Xoroshiro128PlusPlus::from_seed_128(upgrade_seed_to_128bit(seed));
    }

    /// `XoroshiroRandomSource.nextLong`.
    pub fn next_long(&mut self) -> i64 {
        self.generator.next_long()
    }

    /// `XoroshiroRandomSource.nextInt()`.
    pub fn next_int(&mut self) -> i32 {
        self.generator.next_long() as i32
    }

    /// `XoroshiroRandomSource.nextInt(int bound)`
    ///
    /// This is Lemire's multiply-shift sampler and *not* the modulo rejection
    /// loop `java.util.Random.nextInt(bound)` runs: a 32-bit draw is multiplied
    /// by the bound into 64 bits, the high half is the answer and the low half
    /// decides rejection, so the accepted draw count differs from
    /// [`crate::legacy_random::LegacyRandomSource::next_int`] for the same
    /// bound. There is no power-of-two fast path either — every bound takes the
    /// same route.
    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "Bound must be positive");
        let bound_wide = i64::from(bound);
        // `Integer.toUnsignedLong(this.nextInt())`.
        let mut random_bits = i64::from(self.next_int() as u32);
        let mut multiplied_random_bits = random_bits.wrapping_mul(bound_wide);
        let mut fractional_part = multiplied_random_bits & 0xFFFF_FFFF;
        if fractional_part < bound_wide {
            // `Integer.remainderUnsigned(~bound + 1, bound)`: the number of
            // low buckets that would be over-represented, i.e. `2^32 % bound`.
            let unbiased_buckets_start_index =
                i64::from((!bound).wrapping_add(1) as u32 % bound as u32);
            while fractional_part < unbiased_buckets_start_index {
                random_bits = i64::from(self.next_int() as u32);
                multiplied_random_bits = random_bits.wrapping_mul(bound_wide);
                fractional_part = multiplied_random_bits & 0xFFFF_FFFF;
            }
        }
        (multiplied_random_bits >> 32) as i32
    }

    /// `XoroshiroRandomSource.nextBoolean`
    pub fn next_boolean(&mut self) -> bool {
        (self.generator.next_long() & 1) != 0
    }

    /// `XoroshiroRandomSource.nextFloat`.
    pub fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 * FLOAT_UNIT
    }

    /// `XoroshiroRandomSource.nextDouble`.
    pub fn next_double(&mut self) -> f64 {
        self.next_bits(53) as f64 * DOUBLE_UNIT
    }

    /// `XoroshiroRandomSource.nextBits`:
    /// the top `bits` bits of one draw, taken with Java's unsigned shift.
    fn next_bits(&mut self, bits: u32) -> i64 {
        ((self.generator.next_long() as u64) >> (64 - bits)) as i64
    }
}

#[cfg(test)]
mod tests {
    //! Every expected value below was produced twice, independently:
    //!
    //! 1. by running the shipped `net.minecraft.world.level.levelgen`
    //!    `Xoroshiro128PlusPlus` / `XoroshiroRandomSource` / `RandomSupport`
    //!    classes out of `server-26.1.jar` on a JDK, and
    //! 2. by a from-scratch re-implementation of the Java semantics in Python,
    //!    written from the decompiled source alone.
    //!
    //! The two agreed byte for byte, so the numbers here are the official
    //! server's output rather than this module's.

    use super::*;

    /// `Xoroshiro128PlusPlus.nextLong` (`Xoroshiro128PlusPlus.java:26-34`) from
    /// a state set directly, including the all-zero pair the constructor
    /// substitutes (`Xoroshiro128PlusPlus.java:17-24`).
    #[test]
    fn the_raw_generator_walks_the_official_stream() {
        const CASES: &[((i64, i64), [i64; 8])] = &[
            (
                (0, 0),
                [
                    6807859099481836695,
                    5275285228792843439,
                    -1883134111310439721,
                    -7481282880567689833,
                    -7884262219761809303,
                    8358756903579477222,
                    -8273970421895100078,
                    -645759475717512899,
                ],
            ),
            (
                (1, 2),
                [
                    393217,
                    669327710093319,
                    1732421326133921491,
                    -7051953992050424633,
                    -8891291296936358940,
                    3586421180005889563,
                    1691397964866707553,
                    -7711117276956439919,
                ],
            ),
            (
                (-1, 1),
                [
                    -1,
                    -35734131965952,
                    -1152402534833585527,
                    1011031301013903998,
                    -4433826628979058053,
                    283823977746095602,
                    -5175604816124799629,
                    7276592279597412226,
                ],
            ),
            (
                (SILVER_RATIO_64, GOLDEN_RATIO_64),
                [
                    3048035857114651019,
                    2222872858311155711,
                    -6486004625602579455,
                    -2664169022942708869,
                    5191993041567473679,
                    -6182465853508579789,
                    -6201627850474679329,
                    -4465213839879552123,
                ],
            ),
        ];
        for ((seed_lo, seed_hi), expected) in CASES {
            let mut generator = Xoroshiro128PlusPlus::new(*seed_lo, *seed_hi);
            let drawn: Vec<i64> = (0..expected.len()).map(|_| generator.next_long()).collect();
            assert_eq!(drawn, expected, "raw state {seed_lo},{seed_hi}");
        }
        // The all-zero pair is the fixed point the constructor replaces, so it
        // draws the same stream as the pair it is replaced with.
        assert_eq!(
            Xoroshiro128PlusPlus::new(0, 0).state(),
            (GOLDEN_RATIO_64, SILVER_RATIO_64)
        );
    }

    /// `RandomSupport.upgradeSeedTo128bitUnmixed`
    /// and `upgradeSeedTo128bit`.
    #[test]
    fn the_seed_upgrade_matches_the_official_output() {
        // seed, unmixed lo/hi, mixed lo/hi.
        const CASES: &[(i64, i64, i64, i64, i64)] = &[
            (
                0,
                7640891576956012809,
                594862322569659678,
                3847398142028685078,
                7192185014346937746,
            ),
            (
                1,
                7640891576956012808,
                594862322569659677,
                5272463233947570727,
                1927618558350093866,
            ),
            (
                -1,
                -7640891576956012810,
                3759823242367185675,
                -110783831392733308,
                2932223646667407290,
            ),
            (
                42,
                7640891576956012835,
                594862322569659704,
                6720814022939733433,
                -2851323883594622011,
            ),
            (
                1234567890123456789,
                8875458604272273436,
                1829429349885920305,
                8769092525673015319,
                -7481273819660931731,
            ),
            (
                i64::MIN,
                -1582480459898762999,
                -8628509714285116130,
                -6382634648412944878,
                5448932524140013571,
            ),
        ];
        for (seed, unmixed_lo, unmixed_hi, mixed_lo, mixed_hi) in CASES {
            let raw = upgrade_seed_to_128bit_unmixed(*seed);
            assert_eq!(
                (raw.seed_lo, raw.seed_hi),
                (*unmixed_lo, *unmixed_hi),
                "unmixed({seed})"
            );
            let upgraded = upgrade_seed_to_128bit(*seed);
            assert_eq!(
                (upgraded.seed_lo, upgraded.seed_hi),
                (*mixed_lo, *mixed_hi),
                "mixed({seed})"
            );
        }
    }

    /// `RandomSupport.seedFromHashOf`, the hash
    /// `RandomSequence.seedForKey` folds a
    /// sequence key in with.
    #[test]
    fn the_key_hash_matches_the_official_md5_split() {
        const CASES: &[(&str, (i64, i64))] = &[
            ("", (-3162216497309240828, -1621285313438006658)),
            (
                "minecraft:blocks/stone",
                (8630522255710752552, -1263817869132723299),
            ),
            (
                "minecraft:blocks/diamond_ore",
                (3341782701134066133, 1707541829296416187),
            ),
            (
                "minecraft:blocks/oak_leaves",
                (-1196680148576723403, 2241300332399962973),
            ),
            (
                "minecraft:entities/zombie",
                (-768814030213158141, 5495999711084782798),
            ),
        ];
        for (input, expected) in CASES {
            let seed = seed_from_hash_of(input);
            assert_eq!((seed.seed_lo, seed.seed_hi), *expected, "hash of {input:?}");
        }
    }

    /// `XoroshiroRandomSource(long seed)`
    /// followed by `nextLong`.
    #[test]
    fn a_seeded_source_walks_the_official_stream() {
        const CASES: &[(i64, (i64, i64), [i64; 8])] = &[
            (
                0,
                (3847398142028685078, 7192185014346937746),
                [
                    3038984756725240190,
                    -3694039286755638414,
                    4633751808701151732,
                    2160572957309072155,
                    1839370574944072389,
                    -4488466507718817201,
                    -4199796579929588030,
                    -1069045159880208415,
                ],
            ),
            (
                1,
                (5272463233947570727, 1927618558350093866),
                [
                    -1033667707219518978,
                    6451672561743293322,
                    -1821890263888393630,
                    890086654470169703,
                    8094835630745194324,
                    2779418831538184155,
                    -2153570570747265786,
                    2631759950516672506,
                ],
            ),
            (
                -1,
                (-110783831392733308, 2932223646667407290),
                [
                    -8676505878415342125,
                    -868585888688873692,
                    -6331679347063163302,
                    -2068491455652362927,
                    -5626054917968568837,
                    350347487066691045,
                    5757290794395940,
                    -3423761802310783585,
                ],
            ),
            (
                42,
                (6720814022939733433, -2851323883594622011),
                [
                    -4695948378737616609,
                    7341713790291473579,
                    -7542733514721318211,
                    4888889476139319686,
                    8419651034331256779,
                    -6491934549477179079,
                    8279452174680803839,
                    6246239634032559210,
                ],
            ),
            (
                1234567890123456789,
                (8769092525673015319, -7481273819660931731),
                [
                    -412491574613767211,
                    995339481696482301,
                    -9182186815809476885,
                    -3026495172682072589,
                    3164943235357423725,
                    6058643449839064071,
                    523021270511442169,
                    -5347956871874582672,
                ],
            ),
        ];
        for (seed, state, expected) in CASES {
            let mut source = XoroshiroRandomSource::with_seed(*seed);
            assert_eq!(source.state(), *state, "initial state for seed {seed}");
            let drawn: Vec<i64> = (0..expected.len()).map(|_| source.next_long()).collect();
            assert_eq!(drawn, expected, "seed {seed}");
        }
    }

    /// `XoroshiroRandomSource.nextInt(int bound)`
    /// for the same seed across bounds,
    /// including `1` (which still draws) and `Integer.MAX_VALUE`.
    #[test]
    fn bounded_next_int_matches_the_official_unbiased_sampler() {
        const CASES: &[(i32, [i32; 12])] = &[
            (1, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (2, [0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1]),
            (3, [1, 0, 2, 1, 1, 0, 2, 1, 2, 2, 2, 2]),
            (7, [2, 2, 6, 3, 4, 2, 6, 2, 5, 5, 5, 6]),
            (16, [6, 5, 13, 7, 10, 4, 14, 6, 11, 12, 12, 14]),
            (100, [41, 31, 85, 48, 66, 28, 93, 40, 73, 75, 76, 91]),
            (
                1000,
                [413, 316, 857, 482, 664, 286, 932, 408, 732, 751, 765, 918],
            ),
            (
                i32::MAX,
                [
                    887286927, 678722133, 1842156893, 1035403458, 1426607076, 615112860,
                    2002232574, 876413492, 1573617618, 1614283821, 1643194575, 1972085646,
                ],
            ),
        ];
        for (bound, expected) in CASES {
            let mut source = XoroshiroRandomSource::with_seed(42);
            let drawn: Vec<i32> = (0..expected.len())
                .map(|_| source.next_int_bounded(*bound))
                .collect();
            assert_eq!(drawn, expected, "bound {bound}");
        }
    }

    /// `XoroshiroRandomSource.nextFloat` / `nextDouble` / `nextBoolean`
    /// compared as raw bits so a rounding
    /// difference cannot hide behind a printed decimal.
    ///
    /// The `nextDouble` row is also what proves the multiply is a `double`
    /// multiply: under the decompiled `1.110223E-16F` expression these values
    /// come out different (see [`DOUBLE_UNIT`]).
    #[test]
    fn the_scaled_draws_match_the_official_bits() {
        const FLOAT_CASES: &[(i64, [u32; 8])] = &[
            (
                0,
                [
                    1042854544, 1061993506, 1048616136, 1039130408, 1036793352, 1061270979,
                    1061533523, 1064380925,
                ],
            ),
            (
                42,
                [
                    1061082275, 1053541890, 1058493139, 1049080230, 1055502648, 1059448835,
                    1055247626, 1051549234,
                ],
            ),
        ];
        for (seed, expected) in FLOAT_CASES {
            let mut source = XoroshiroRandomSource::with_seed(*seed);
            let drawn: Vec<u32> = (0..expected.len())
                .map(|_| source.next_float().to_bits())
                .collect();
            assert_eq!(drawn, expected, "nextFloat bits for seed {seed}");
        }

        const DOUBLE_CASES: &[(i64, [u64; 8])] = &[
            (
                0,
                [
                    4595103537393514408,
                    4605378688679531256,
                    4598196768168590638,
                    4593104158777653488,
                    4591849461971540208,
                    4604990784763045329,
                    4605131736876223663,
                    4606660424093044649,
                ],
            ),
            (
                42,
                [
                    4604889475255711931,
                    4600841262291237436,
                    4603499443451032389,
                    4598445926046948224,
                    4601893935381120036,
                    4604012528883280504,
                    4601757022432242642,
                    4599771463310515840,
                ],
            ),
        ];
        for (seed, expected) in DOUBLE_CASES {
            let mut source = XoroshiroRandomSource::with_seed(*seed);
            let drawn: Vec<u64> = (0..expected.len())
                .map(|_| source.next_double().to_bits())
                .collect();
            assert_eq!(drawn, expected, "nextDouble bits for seed {seed}");
        }

        const BOOLEAN_CASES: &[(i64, [bool; 16])] = &[
            (
                0,
                [
                    false, false, false, true, true, true, false, true, true, false, true, false,
                    true, true, false, true,
                ],
            ),
            (
                42,
                [
                    true, true, true, false, true, true, true, false, true, true, true, true,
                    false, false, false, true,
                ],
            ),
        ];
        for (seed, expected) in BOOLEAN_CASES {
            let mut source = XoroshiroRandomSource::with_seed(*seed);
            let drawn: Vec<bool> = (0..expected.len()).map(|_| source.next_boolean()).collect();
            assert_eq!(drawn, expected, "nextBoolean for seed {seed}");
        }
    }

    #[test]
    #[should_panic(expected = "Bound must be positive")]
    fn a_non_positive_bound_is_refused_like_the_official_source() {
        XoroshiroRandomSource::with_seed(0).next_int_bounded(0);
    }
}
