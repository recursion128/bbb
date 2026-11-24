use crate::explosion::ExplosionSourceEntity;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::minecart::MinecartData;
use crate::entity::save::{optional_float, optional_int};

pub const PRIME_EVENT: i8 = 10;
pub const NO_FUSE: i32 = -1;
pub const DEFAULT_FUSE: i32 = 80;
pub const DEFAULT_EXPLOSION_POWER_BASE: f32 = 4.0;
pub const DEFAULT_EXPLOSION_SPEED_FACTOR: f32 = 1.0;
pub const MAX_EXPLOSION_POWER_COMPONENT: f32 = 128.0;
pub const MIN_COLLISION_SPEED_SQUARED: f64 = 0.01;
pub const MIN_EXPLOSIVE_FALL_DISTANCE: f64 = 3.0;

#[derive(Debug, Clone, PartialEq)]
pub struct TntMinecartIgnitionSource {
    causing_entity: Option<ExplosionSourceEntity>,
}

impl TntMinecartIgnitionSource {
    pub const fn new(causing_entity: Option<ExplosionSourceEntity>) -> Self {
        Self { causing_entity }
    }

    pub const fn causing_entity(&self) -> Option<&ExplosionSourceEntity> {
        self.causing_entity.as_ref()
    }

    pub fn into_causing_entity(self) -> Option<ExplosionSourceEntity> {
        self.causing_entity
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TntMinecartDamageSource {
    pub ignition_source: TntMinecartIgnitionSource,
    pub direct_entity_is_projectile: bool,
    pub direct_projectile_is_on_fire: bool,
    pub is_fire: bool,
    pub is_explosion: bool,
}

impl TntMinecartDamageSource {
    /// `MinecartTNT.java:216-220`.
    pub const fn ignites_tnt(&self) -> bool {
        if self.direct_entity_is_projectile {
            self.direct_projectile_is_on_fire
        } else {
            self.is_fire || self.is_explosion
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TntMinecartPrimeEffects {
    BlockedByGameRule,
    Primed { event: i8, play_sound: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TntMinecartExplosionDecision {
    None,
    Discard,
    Explode {
        speed_squared: f64,
        ignition_source: Option<TntMinecartIgnitionSource>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TntMinecartTickEffects {
    pub smoke: bool,
    pub explosion: TntMinecartExplosionDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TntMinecartDestroyDecision {
    DropTntMinecart,
    ArmShortFuse { prime: TntMinecartPrimeEffects },
    Keep,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TntMinecartData {
    minecart: MinecartData,
    fuse: i32,
    explosion_power_base: f32,
    explosion_speed_factor: f32,
    ignition_source: Option<TntMinecartIgnitionSource>,
}

impl TntMinecartData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.minecart.restore_command_transients(&previous.minecart);
        self.ignition_source = previous.ignition_source.clone();
    }

    /// `MinecartTNT.java:34-40`.
    pub const fn new() -> Self {
        Self {
            minecart: MinecartData::new(),
            fuse: NO_FUSE,
            explosion_power_base: DEFAULT_EXPLOSION_POWER_BASE,
            explosion_speed_factor: DEFAULT_EXPLOSION_SPEED_FACTOR,
            ignition_source: None,
        }
    }

    pub const fn minecart(&self) -> &MinecartData {
        &self.minecart
    }

    pub(crate) const fn minecart_mut(&mut self) -> &mut MinecartData {
        &mut self.minecart
    }

    pub const fn fuse(&self) -> i32 {
        self.fuse
    }

    pub const fn is_primed(&self) -> bool {
        self.fuse > NO_FUSE
    }

    pub const fn explosion_power_base(&self) -> f32 {
        self.explosion_power_base
    }

    pub const fn explosion_speed_factor(&self) -> f32 {
        self.explosion_speed_factor
    }

    pub fn ignition_source(&self) -> Option<TntMinecartIgnitionSource> {
        self.ignition_source.clone()
    }

    /// `MinecartTNT.java:150-163`.
    pub fn prime(
        &mut self,
        tnt_explodes: bool,
        source: Option<TntMinecartIgnitionSource>,
        silent: bool,
    ) -> TntMinecartPrimeEffects {
        if !tnt_explodes {
            return TntMinecartPrimeEffects::BlockedByGameRule;
        }
        self.fuse = DEFAULT_FUSE;
        if self.ignition_source.is_none() {
            self.ignition_source = source;
        }
        TntMinecartPrimeEffects::Primed {
            event: PRIME_EVENT,
            play_sound: !silent,
        }
    }

    /// The fuse branch of `MinecartTNT.tick` (`MinecartTNT.java:52-59`).
    pub fn tick_fuse(
        &mut self,
        tnt_explodes: bool,
        horizontal_speed_squared: f64,
    ) -> TntMinecartTickEffects {
        if self.fuse > 0 {
            self.fuse -= 1;
            return TntMinecartTickEffects {
                smoke: true,
                explosion: TntMinecartExplosionDecision::None,
            };
        }
        TntMinecartTickEffects {
            smoke: false,
            explosion: if self.fuse == 0 {
                self.explosion_decision(
                    tnt_explodes,
                    horizontal_speed_squared,
                    self.ignition_source.clone(),
                )
            } else {
                TntMinecartExplosionDecision::None
            },
        }
    }

    /// `MinecartTNT.java:61-65`.
    pub fn horizontal_collision_decision(
        &self,
        tnt_explodes: bool,
        horizontal_speed_squared: f64,
    ) -> TntMinecartExplosionDecision {
        if horizontal_speed_squared < MIN_COLLISION_SPEED_SQUARED {
            TntMinecartExplosionDecision::None
        } else {
            self.explosion_decision(
                tnt_explodes,
                horizontal_speed_squared,
                self.ignition_source.clone(),
            )
        }
    }

    /// `MinecartTNT.java:125-131`.
    pub fn fall_decision(
        &self,
        tnt_explodes: bool,
        fall_distance: f64,
    ) -> TntMinecartExplosionDecision {
        if fall_distance < MIN_EXPLOSIVE_FALL_DISTANCE {
            return TntMinecartExplosionDecision::None;
        }
        let speed = fall_distance / 10.0;
        self.explosion_decision(tnt_explodes, speed * speed, self.ignition_source.clone())
    }

    /// `MinecartTNT.java:70-76`.
    pub fn burning_arrow_decision(
        &self,
        tnt_explodes: bool,
        arrow_speed_squared: f64,
        source: TntMinecartIgnitionSource,
    ) -> TntMinecartExplosionDecision {
        self.explosion_decision(tnt_explodes, arrow_speed_squared, Some(source))
    }

    /// `MinecartTNT.destroy` (`MinecartTNT.java:80-90`).
    pub fn destroy_decision(
        &mut self,
        tnt_explodes: bool,
        source: TntMinecartDamageSource,
        horizontal_speed_squared: f64,
        first_fuse_draw: i32,
        second_fuse_draw: i32,
        silent: bool,
    ) -> TntMinecartDestroyDecision {
        if !source.ignites_tnt() && horizontal_speed_squared < MIN_COLLISION_SPEED_SQUARED {
            return TntMinecartDestroyDecision::DropTntMinecart;
        }
        if self.is_primed() {
            return TntMinecartDestroyDecision::Keep;
        }
        debug_assert!((0..20).contains(&first_fuse_draw));
        debug_assert!((0..20).contains(&second_fuse_draw));
        let prime = self.prime(tnt_explodes, Some(source.ignition_source.clone()), silent);
        // Vanilla assigns the two-draw fuse even when `primeFuse` was gamerule-gated.
        self.fuse = first_fuse_draw + second_fuse_draw;
        TntMinecartDestroyDecision::ArmShortFuse { prime }
    }

    /// `MinecartTNT.java:135-147`.
    pub fn activator_rail(
        &mut self,
        powered: bool,
        tnt_explodes: bool,
        silent: bool,
    ) -> Option<TntMinecartPrimeEffects> {
        (powered && !self.is_primed()).then(|| self.prime(tnt_explodes, None, silent))
    }

    pub fn handle_entity_event(
        &mut self,
        event: i8,
        tnt_explodes: bool,
        silent: bool,
    ) -> Option<TntMinecartPrimeEffects> {
        (event == PRIME_EVENT).then(|| self.prime(tnt_explodes, None, silent))
    }

    /// `MinecartTNT.explode` (`MinecartTNT.java:102-121`).
    pub fn explosion_decision(
        &self,
        tnt_explodes: bool,
        speed_squared: f64,
        source: Option<TntMinecartIgnitionSource>,
    ) -> TntMinecartExplosionDecision {
        if tnt_explodes {
            TntMinecartExplosionDecision::Explode {
                speed_squared,
                ignition_source: source,
            }
        } else if self.is_primed() {
            TntMinecartExplosionDecision::Discard
        } else {
            TntMinecartExplosionDecision::None
        }
    }

    /// `MinecartTNT.java:102-116`.
    pub fn explosion_radius(&self, speed_squared: f64, random_unit: f64) -> f32 {
        debug_assert!((0.0..1.0).contains(&random_unit));
        let speed = speed_squared.sqrt().min(5.0);
        (f64::from(self.explosion_power_base)
            + f64::from(self.explosion_speed_factor) * random_unit * 1.5 * speed) as f32
    }

    /// `MinecartTNT.java:175-187`.
    pub const fn preserves_rail_during_explosion(
        &self,
        block_is_rail: bool,
        block_above_is_rail: bool,
    ) -> bool {
        self.is_primed() && (block_is_rail || block_above_is_rail)
    }

    /// `MinecartTNT.java:191-209`.
    pub fn from_saved(minecart: MinecartData, entries: &[NbtSummaryEntry]) -> Self {
        let fuse = optional_int(entries, "fuse").unwrap_or(NO_FUSE);
        let explosion_power_base = optional_float(entries, "explosion_power")
            .unwrap_or(DEFAULT_EXPLOSION_POWER_BASE)
            .clamp(0.0, MAX_EXPLOSION_POWER_COMPONENT);
        let explosion_speed_factor = optional_float(entries, "explosion_speed_factor")
            .unwrap_or(DEFAULT_EXPLOSION_SPEED_FACTOR)
            .clamp(0.0, MAX_EXPLOSION_POWER_COMPONENT);
        Self {
            minecart,
            fuse,
            explosion_power_base,
            explosion_speed_factor,
            ignition_source: None,
        }
    }

    pub fn write_save_data(&self, entries: &mut Vec<NbtSummaryEntry>) {
        entries.push(NbtSummaryEntry {
            name: "fuse".into(),
            value: NbtSummaryValue::Int(self.fuse),
        });
        if self.explosion_power_base != DEFAULT_EXPLOSION_POWER_BASE {
            entries.push(NbtSummaryEntry {
                name: "explosion_power".into(),
                value: NbtSummaryValue::Float(self.explosion_power_base.to_bits()),
            });
        }
        if self.explosion_speed_factor != DEFAULT_EXPLOSION_SPEED_FACTOR {
            entries.push(NbtSummaryEntry {
                name: "explosion_speed_factor".into(),
                value: NbtSummaryValue::Float(self.explosion_speed_factor.to_bits()),
            });
        }
    }
}

impl Default for TntMinecartData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explosion::ExplosionSourceEntityKind;
    use bbb_protocol::packets::Vec3d;

    fn ignition(entity_id: i32) -> TntMinecartIgnitionSource {
        TntMinecartIgnitionSource::new(Some(ExplosionSourceEntity {
            custom_name: None,
            entity_id,
            entity_resource_id: "minecraft:pig".to_string(),
            uuid: uuid::Uuid::from_u128(entity_id as u128),
            position: Vec3d::default(),
            kind: ExplosionSourceEntityKind::Mob,
        }))
    }

    fn source(ignites: bool, entity_id: i32) -> TntMinecartDamageSource {
        TntMinecartDamageSource {
            ignition_source: ignition(entity_id),
            direct_entity_is_projectile: false,
            direct_projectile_is_on_fire: false,
            is_fire: ignites,
            is_explosion: false,
        }
    }

    fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    }

    #[test]
    fn defaults_and_prime_follow_the_gamerule_and_keep_the_first_source() {
        let mut data = TntMinecartData::new();
        assert_eq!(data.minecart(), &MinecartData::new());
        assert_eq!(data.fuse(), NO_FUSE);
        assert_eq!(data.explosion_power_base(), 4.0);
        assert_eq!(data.explosion_speed_factor(), 1.0);
        assert_eq!(
            data.prime(false, Some(ignition(1)), false),
            TntMinecartPrimeEffects::BlockedByGameRule
        );
        assert_eq!(data.fuse(), NO_FUSE);
        assert_eq!(data.ignition_source(), None);
        assert_eq!(
            data.prime(true, Some(ignition(2)), false),
            TntMinecartPrimeEffects::Primed {
                event: PRIME_EVENT,
                play_sound: true
            }
        );
        data.prime(true, Some(ignition(3)), true);
        assert_eq!(data.fuse(), DEFAULT_FUSE);
        assert_eq!(data.ignition_source(), Some(ignition(2)));
    }

    #[test]
    fn countdown_waits_one_tick_at_zero_and_collision_uses_its_threshold() {
        let mut data = TntMinecartData::new();
        data.fuse = 1;
        assert_eq!(
            data.tick_fuse(true, 0.0),
            TntMinecartTickEffects {
                smoke: true,
                explosion: TntMinecartExplosionDecision::None
            }
        );
        assert!(matches!(
            data.tick_fuse(true, 0.25).explosion,
            TntMinecartExplosionDecision::Explode {
                speed_squared: 0.25,
                ..
            }
        ));
        let unprimed = TntMinecartData::new();
        assert_eq!(
            unprimed.horizontal_collision_decision(true, 0.009),
            TntMinecartExplosionDecision::None
        );
        assert!(matches!(
            unprimed.horizontal_collision_decision(true, 0.01),
            TntMinecartExplosionDecision::Explode { .. }
        ));
    }

    #[test]
    fn fall_and_burning_arrow_make_explosion_decisions() {
        let data = TntMinecartData::new();
        assert_eq!(
            data.fall_decision(true, 2.999),
            TntMinecartExplosionDecision::None
        );
        let TntMinecartExplosionDecision::Explode { speed_squared, .. } =
            data.fall_decision(true, 3.0)
        else {
            panic!("three-block fall must explode");
        };
        assert!((speed_squared - 0.09).abs() < f64::EPSILON);
        let arrow_source = ignition(7);
        assert_eq!(
            data.burning_arrow_decision(true, 2.25, arrow_source.clone()),
            TntMinecartExplosionDecision::Explode {
                speed_squared: 2.25,
                ignition_source: Some(arrow_source)
            }
        );
    }

    #[test]
    fn disabled_gamerule_discards_only_an_already_primed_cart() {
        let mut data = TntMinecartData::new();
        assert_eq!(
            data.explosion_decision(false, 1.0, None),
            TntMinecartExplosionDecision::None
        );
        data.prime(true, None, true);
        assert_eq!(
            data.explosion_decision(false, 1.0, None),
            TntMinecartExplosionDecision::Discard
        );
    }

    #[test]
    fn destroy_drops_only_an_ordinary_slow_cart_and_arms_other_sources() {
        let mut ordinary = TntMinecartData::new();
        assert_eq!(
            ordinary.destroy_decision(true, source(false, 1), 0.009, 2, 3, false),
            TntMinecartDestroyDecision::DropTntMinecart
        );
        assert_eq!(ordinary.fuse(), NO_FUSE);

        let mut fast = TntMinecartData::new();
        assert!(matches!(
            fast.destroy_decision(true, source(false, 2), 0.01, 2, 3, false),
            TntMinecartDestroyDecision::ArmShortFuse { .. }
        ));
        assert_eq!(fast.fuse(), 5);

        let mut fire = TntMinecartData::new();
        fire.destroy_decision(true, source(true, 3), 0.0, 19, 19, true);
        assert_eq!(fire.fuse(), 38);
        assert_eq!(fire.ignition_source(), Some(ignition(3)));

        let mut explosion = TntMinecartData::new();
        let mut explosion_source = source(false, 4);
        explosion_source.is_explosion = true;
        explosion.destroy_decision(true, explosion_source.clone(), 0.0, 0, 1, true);
        assert_eq!(explosion.fuse(), 1);

        let mut unlit_projectile = TntMinecartData::new();
        explosion_source.direct_entity_is_projectile = true;
        assert_eq!(
            unlit_projectile.destroy_decision(true, explosion_source, 0.0, 0, 1, true),
            TntMinecartDestroyDecision::DropTntMinecart
        );
    }

    #[test]
    fn powered_activator_and_event_ten_prime() {
        let mut data = TntMinecartData::new();
        assert_eq!(data.activator_rail(false, true, false), None);
        assert!(matches!(
            data.activator_rail(true, true, false),
            Some(TntMinecartPrimeEffects::Primed { event: 10, .. })
        ));
        data.fuse = 3;
        assert_eq!(data.activator_rail(true, true, false), None);
        assert_eq!(data.handle_entity_event(9, true, false), None);
        assert!(data.handle_entity_event(10, true, false).is_some());
        assert_eq!(data.fuse(), DEFAULT_FUSE);
    }

    #[test]
    fn radius_is_bounded_by_speed_and_rails_are_preserved_only_when_primed() {
        let mut data = TntMinecartData::new();
        assert_eq!(data.explosion_radius(0.0, 0.999), 4.0);
        let capped = data.explosion_radius(25.0, 0.999_999);
        assert!((4.0..11.5).contains(&capped));
        assert_eq!(data.explosion_radius(10_000.0, 0.999_999), capped);
        assert!(!data.preserves_rail_during_explosion(true, false));
        data.prime(true, None, true);
        assert!(data.preserves_rail_during_explosion(true, false));
        assert!(data.preserves_rail_during_explosion(false, true));
        assert!(!data.preserves_rail_during_explosion(false, false));
    }

    #[test]
    fn save_clamps_power_omits_defaults_and_does_not_persist_ignition() {
        let mut data = TntMinecartData::new();
        data.prime(true, Some(ignition(42)), true);
        let mut defaults = Vec::new();
        data.write_save_data(&mut defaults);
        assert_eq!(field(&defaults, "fuse"), Some(&NbtSummaryValue::Int(80)));
        assert_eq!(field(&defaults, "explosion_power"), None);
        assert_eq!(field(&defaults, "explosion_speed_factor"), None);

        let saved = vec![
            NbtSummaryEntry {
                name: "fuse".into(),
                value: NbtSummaryValue::Int(17),
            },
            NbtSummaryEntry {
                name: "explosion_power".into(),
                value: NbtSummaryValue::Float((-3.0_f32).to_bits()),
            },
            NbtSummaryEntry {
                name: "explosion_speed_factor".into(),
                value: NbtSummaryValue::Float(200.0_f32.to_bits()),
            },
        ];
        let restored = TntMinecartData::from_saved(MinecartData::new(), &saved);
        assert_eq!(restored.fuse(), 17);
        assert_eq!(restored.explosion_power_base(), 0.0);
        assert_eq!(restored.explosion_speed_factor(), 128.0);
        assert_eq!(restored.ignition_source(), None);

        let mut round_trip = Vec::new();
        restored.write_save_data(&mut round_trip);
        let restored_again = TntMinecartData::from_saved(MinecartData::new(), &round_trip);
        assert_eq!(restored_again, restored);
    }
}
