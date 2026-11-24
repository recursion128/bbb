//! `Projectile.shoot` and `shootFromRotation`: the motion a thrown or fired
//! projectile leaves its source with, for every spawner that files one.

use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};
use bbb_server_types::geometry::{mth_cos, mth_sin, Vec3Ops};

/// The `LivingEntity` facts `Projectile.shootFromRotation`
/// (`Projectile.java:152-159`) reads off its source besides the two angles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotSource {
    /// `Entity.getKnownMovement()`, which for a `ServerPlayer` is the last
    /// client-reported movement (`ServerPlayer.java:2179-2182`).
    pub known_movement: Vec3d,
    /// `Entity.onGround()`.
    pub on_ground: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotParameters {
    pub x_rot: f32,
    pub y_rot: f32,
    pub y_offset: f32,
    pub pow: f32,
    pub uncertainty: f32,
}

/// `Projectile.getMovementToShoot` (`Projectile.java:130-139`): the direction
/// normalised, blurred by three `random.triangle(0, 0.0172275 * uncertainty)`
/// draws in axis order, then scaled by `pow`.
///
/// The draws come off the store's entity stream, which stands in for the
/// projectile's own `Entity.random` (see [`EntityStore::next_uuid`]).
pub fn movement_to_shoot(
    store: &mut EntityStore,
    direction: Vec3d,
    pow: f32,
    uncertainty: f32,
) -> Vec3d {
    let spread = 0.0172275 * f64::from(uncertainty);
    let normalized = direction.normalize();
    let x = normalized.x + triangle(store, spread);
    let y = normalized.y + triangle(store, spread);
    let z = normalized.z + triangle(store, spread);
    Vec3d { x, y, z }.scale(f64::from(pow))
}

/// `RandomSource.triangle(mean, spread)` (`RandomSource.java:59-61`) with a
/// zero mean: `spread * (nextDouble() - nextDouble())`, first draw first.
fn triangle(store: &mut EntityStore, spread: f64) -> f64 {
    let first = store.random().next_double();
    let second = store.random().next_double();
    spread * (first - second)
}

/// The motion `Projectile.shoot` (`Projectile.java:141-149`) installs, and
/// what `shootFromRotation` (`:152-159`) adds after it.
///
/// A spawner takes one of these and runs [`ProjectileShot::install`] inside
/// the store's initializer, which is how the filed entity -- and the
/// `EntityLifecycleEvent::Added` snapshot the trackers pair from -- already
/// carry the shot, the way `spawnProjectile` (`:225-232`) shoots before
/// `addFreshEntity`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectileShot {
    /// `shoot`'s `movement`: the velocity the rotation is derived from.
    pub movement: Vec3d,
    /// `shootFromRotation`'s `sourceMovement` term (`:157-158`), added to the
    /// velocity after `shoot`; zero for a bare `shoot`.
    pub source_movement: Vec3d,
}

impl ProjectileShot {
    /// A bare `shoot` (`Projectile.java:141-149`) with nothing added after.
    pub const fn new(movement: Vec3d) -> Self {
        Self {
            movement,
            source_movement: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        }
    }

    /// `Projectile.shoot` (`Projectile.java:141-149`) -- the velocity, the sync
    /// flag and the rotation `Mth.atan2` derives from that velocity -- then the
    /// `setDeltaMovement(getDeltaMovement().add(..))` of `shootFromRotation`
    /// (`:158`).
    ///
    /// `yRotO`/`xRotO` (`:148-149`) are the previous-tick rotation, which no
    /// entity here keeps yet.
    pub fn install(&self, entity: &mut Entity) {
        let movement = self.movement;
        entity.set_delta_movement(movement);
        entity.set_needs_sync(true);
        let horizontal = movement.horizontal_distance_sqr().sqrt();
        let y_rot = (mth_atan2(movement.x, movement.z) * 180.0 / std::f64::consts::PI) as f32;
        let x_rot = (mth_atan2(movement.y, horizontal) * 180.0 / std::f64::consts::PI) as f32;
        entity.set_rot(y_rot, x_rot);
        entity.set_delta_movement(movement.plus(self.source_movement));
    }
}

/// `Projectile.spawnProjectileFromRotation` (`Projectile.java:175-190`) down
/// to its `shootFromRotation` (`:152-159`): the direction the source's angles
/// describe goes through [`movement_to_shoot`], the source's known movement is
/// added on top -- its vertical part only while the source is airborne -- and
/// `spawn` files the projectile carrying that [`ProjectileShot`].
///
/// The blur is drawn before the spawner draws the UUID, where vanilla's
/// constructor draws the UUID first: both come off one store stream here that
/// stands in for the projectile's own unseeded `Entity.random`, and nothing
/// observable orders the two draws against each other.
pub fn shoot_from_rotation(
    store: &mut EntityStore,
    source: ShotSource,
    params: ShotParameters,
    spawn: impl FnOnce(&mut EntityStore, ProjectileShot) -> Option<i32>,
) -> Option<i32> {
    let ShotParameters {
        x_rot,
        y_rot,
        y_offset,
        pow,
        uncertainty,
    } = params;
    let to_radians = std::f32::consts::PI / 180.0;
    let xd = -mth_sin(f64::from(y_rot * to_radians)) * mth_cos(f64::from(x_rot * to_radians));
    let yd = -mth_sin(f64::from((x_rot + y_offset) * to_radians));
    let zd = mth_cos(f64::from(y_rot * to_radians)) * mth_cos(f64::from(x_rot * to_radians));
    let direction = Vec3d {
        x: f64::from(xd),
        y: f64::from(yd),
        z: f64::from(zd),
    };
    let movement = movement_to_shoot(store, direction, pow, uncertainty);
    let source_movement = Vec3d {
        x: source.known_movement.x,
        y: if source.on_ground {
            0.0
        } else {
            source.known_movement.y
        },
        z: source.known_movement.z,
    };
    spawn(
        store,
        ProjectileShot {
            movement,
            source_movement,
        },
    )
}

/// `Mth.atan2(double, double)` (`Mth.java:373-420`): the table approximation
/// every projectile angle goes through, not the exact arc tangent.
pub fn mth_atan2(y: f64, x: f64) -> f64 {
    let d2 = x * x + y * y;
    if d2.is_nan() {
        return f64::NAN;
    }
    let neg_y = y < 0.0;
    let mut y = if neg_y { -y } else { y };
    let neg_x = x < 0.0;
    let mut x = if neg_x { -x } else { x };
    let steep = y > x;
    if steep {
        std::mem::swap(&mut x, &mut y);
    }
    let rinv = fast_inv_sqrt(d2);
    x *= rinv;
    y *= rinv;
    let yp = FRAC_BIAS + y;
    let index = (yp.to_bits() as i32) as usize;
    let (asin_tab, cos_tab) = atan2_tables();
    let phi = asin_tab[index];
    let c_phi = cos_tab[index];
    let s_phi = yp - FRAC_BIAS;
    let sd = y * c_phi - x * s_phi;
    let d = (6.0 + sd * sd) * sd * 0.166_666_666_666_666_66;
    let mut theta = phi + d;
    if steep {
        theta = std::f64::consts::FRAC_PI_2 - theta;
    }
    if neg_x {
        theta = std::f64::consts::PI - theta;
    }
    if neg_y {
        theta = -theta;
    }
    theta
}

/// `Mth.FRAC_BIAS` (`Mth.java:45`).
const FRAC_BIAS: f64 = f64::from_bits(4_805_340_802_404_319_232);

/// `Mth.fastInvSqrt(double)` (`Mth.java:431-437`).
fn fast_inv_sqrt(x: f64) -> f64 {
    let xhalf = 0.5 * x;
    let bits = 6_910_469_410_427_058_090_i64 - ((x.to_bits() as i64) >> 1);
    let x = f64::from_bits(bits as u64);
    x * (1.5 - xhalf * x * x)
}

/// `Mth.ASIN_TAB` and `Mth.COS_TAB` (`Mth.java:745-751`).
fn atan2_tables() -> &'static ([f64; 257], [f64; 257]) {
    static TABLES: std::sync::OnceLock<([f64; 257], [f64; 257])> = std::sync::OnceLock::new();
    TABLES.get_or_init(|| {
        let mut asin_tab = [0.0_f64; 257];
        let mut cos_tab = [0.0_f64; 257];
        for index in 0..257 {
            let v = index as f64 / 256.0;
            let asinv = v.asin();
            cos_tab[index] = asinv.cos();
            asin_tab[index] = asinv;
        }
        (asin_tab, cos_tab)
    })
}

impl EntityData {
    /// `Projectile.getOwner()`'s stored half: the owner UUID a projectile
    /// carries, which `Projectile.getAddEntityPacket`
    /// (`Projectile.java:346-349`) resolves to an entity id.
    ///
    /// A kind that is not a `Projectile` answers `None`, which is the `data: 0`
    /// of `Entity.getAddEntityPacket`. The arms are exactly the kinds
    /// [`Entity::is_projectile_class`] names, which is every concrete
    /// `Projectile` subclass 26.1 declares.
    pub fn projectile_owner(&self) -> Option<Uuid> {
        match self {
            Self::Snowball(data) => data.projectile().owner(),
            Self::Egg(data) => data.projectile().owner(),
            Self::EnderPearl(data) => data.projectile().owner(),
            Self::ExperienceBottle(data) => data.projectile().owner(),
            Self::Arrow(data) => data.projectile().owner(),
            Self::SpectralArrow(data) => data.projectile().owner(),
            Self::Trident(data) => data.projectile().owner(),
            Self::FireworkRocket(data) => data.projectile().owner(),
            Self::LlamaSpit(data) => data.owner(),
            Self::LargeFireball(data) => data.projectile().owner(),
            Self::SmallFireball(data) => data.projectile().owner(),
            Self::DragonFireball(data) => data.projectile().owner(),
            Self::WitherSkull(data) => data.projectile().owner(),
            Self::WindCharge(data) | Self::BreezeWindCharge(data) => data.projectile().owner(),
            Self::SplashPotion(data) | Self::LingeringPotion(data) => data.projectile().owner(),
            Self::ShulkerBullet(data) => data.projectile().owner(),
            // `FishingHook` takes `Projectile.getOwner()` unchanged
            // (`FishingHook.java:51`, `:530-531` reads it back), so its stored
            // half is the same field.
            Self::FishingBobber(data) => data.owner(),
            _ => None,
        }
    }

    /// The owner UUID of an `AbstractArrow`, whose owner
    /// `ServerPlayer.hurtServer` tests (`ServerPlayer.java:978-980`).
    pub fn abstract_arrow_owner(&self) -> Option<Uuid> {
        match self {
            Self::Arrow(_) | Self::SpectralArrow(_) | Self::Trident(_) => self.projectile_owner(),
            _ => None,
        }
    }
}
