//! `ThrownTrident`: a thrown `minecraft:trident`, an `AbstractArrow` that
//! hits for a fixed eight and bounces off what it hits.

use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID, VANILLA_TRIDENT_FOIL_DATA_ID,
    VANILLA_TRIDENT_LOYALTY_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRIDENT_ID;
use bbb_protocol::packets::{SoundSource, Vec3d};
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::enchantment_weapon_effects::{process, WeaponEffectTable};
use crate::entity::abstract_arrow::{AbstractArrowState, ArrowPickup};
use crate::entity::living::SynchedValue;
use crate::entity::store::EntityStore;
use crate::entity::store::PendingEntitySound;
use crate::entity::{Entity, EntityData};
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;

pub use crate::entity::abstract_arrow::DEFAULT_GRAVITY;

/// `ThrownTrident.getDefaultHitGroundSoundEvent()` (`ThrownTrident.java:182-184`).
pub const DEFAULT_SOUND_EVENT_RESOURCE_ID: &str = "minecraft:item.trident.hit_ground";

/// The `AbstractArrow` and `ThrownTrident` fields this slice carries.
#[derive(Debug, Clone, PartialEq)]
pub struct TridentData {
    base: AbstractArrowState,
    dealt_damage: bool,
    loyalty: Option<i8>,
    foil: bool,
}

impl std::ops::Deref for TridentData {
    type Target = AbstractArrowState;

    fn deref(&self) -> &AbstractArrowState {
        &self.base
    }
}

impl std::ops::DerefMut for TridentData {
    fn deref_mut(&mut self) -> &mut AbstractArrowState {
        &mut self.base
    }
}

impl TridentData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.base.restore_command_transients(&previous.base);
        self.foil = previous.foil;
    }

    pub fn new(item: RuntimeItemStack) -> Self {
        let foil = item_has_foil(&item);
        Self::from_state(
            AbstractArrowState::new(item, DEFAULT_SOUND_EVENT_RESOURCE_ID),
            false,
            foil,
        )
    }

    pub(crate) fn from_state(base: AbstractArrowState, dealt_damage: bool, foil: bool) -> Self {
        Self {
            base,
            dealt_damage,
            loyalty: None,
            foil,
        }
    }

    pub const fn dealt_damage(&self) -> bool {
        self.dealt_damage
    }

    pub fn loyalty(&self) -> i8 {
        self.loyalty.unwrap_or_default()
    }

    pub const fn foil(&self) -> bool {
        self.foil
    }

    pub(crate) fn mark_dealt_damage(&mut self) {
        self.dealt_damage = true;
    }

    /// `ThrownTrident.getLoyaltyFromItem` (`ThrownTrident.java:206-210`).
    fn loyalty_from_item(
        &self,
        table: &WeaponEffectTable,
        trident: crate::item_durability::ValueContext<'_>,
        random: &GameRandom,
        value_level: &crate::world::ValueEffectLevel<'_>,
    ) -> i8 {
        let item = self.base.item().summary();
        let acceleration = process(
            table,
            item,
            trident,
            |effects| effects.trident_return_acceleration.as_ref(),
            0.0,
            random,
            value_level,
        );
        (acceleration as i32).clamp(0, 127) as i8
    }
}

/// The owner a loyal trident flies back to: where its eyes are, how wide it
/// is, whether it is a player and `isAcceptibleReturnOwner`.
struct ReturnOwner {
    eye: Vec3d,
    width: f64,
    player: bool,
    acceptable: bool,
}

fn return_owner(
    owner: uuid::Uuid,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<ReturnOwner> {
    if let Some(player) = level.player(owner) {
        return Some(ReturnOwner {
            eye: Vec3d {
                y: player.position.y + player.eye_height,
                ..player.position
            },
            width: f64::from(player.dimensions[0]),
            player: true,
            acceptable: !player.dead_or_dying && !player.spectator,
        });
    }
    let entity = store.get(store.entity_id_for_uuid(owner)?)?;
    let bounds = entity.bounding_box();
    let position = entity.position();
    Some(ReturnOwner {
        eye: Vec3d {
            y: position.y + f64::from(entity.eye_height()),
            ..position
        },
        width: bounds.max_x - bounds.min_x,
        player: false,
        acceptable: entity.is_alive(),
    })
}

/// The loyalty half of `ThrownTrident.tick` (`ThrownTrident.java:67-93`);
/// answers whether the shared arrow tick still runs.
pub(crate) fn return_to_owner(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if data(entity).loyalty.is_none() {
        let loyalty = data(entity).loyalty_from_item(
            store.weapon_effects(),
            crate::item_durability::ValueContext::stored(entity, None),
            store.random(),
            &level.value_effect_level(),
        );
        data_mut(entity).loyalty = Some(loyalty);
        if loyalty != 0 {
            entity.mark_synched_dirty(VANILLA_TRIDENT_LOYALTY_DATA_ID);
        }
    }
    let (loyalty, should_return, owner) = {
        let data = data_mut(entity);
        let loyalty = data.loyalty();
        if data.in_ground_time() > 4 {
            data.mark_dealt_damage();
        }
        (
            loyalty,
            loyalty > 0 && (data.dealt_damage || data.no_physics()),
            data.projectile().owner(),
        )
    };
    if !should_return {
        return true;
    }
    let Some(owner) = owner.and_then(|owner| return_owner(owner, store, level)) else {
        return true;
    };
    let position = entity.position();
    if !owner.acceptable {
        let state = data(entity);
        if state.pickup() == ArrowPickup::Allowed {
            level.spawn_item(
                Vec3d {
                    y: position.y + 0.1,
                    ..position
                },
                state.item().summary(),
            );
        }
        entity.discard();
        return false;
    }
    if !owner.player && position.distance_to_sqr(owner.eye).sqrt() < owner.width + 1.0 {
        entity.discard();
        return false;
    }
    let state = data_mut(entity);
    let returning = state.no_physics();
    if !returning {
        state.set_no_physics(true);
        entity.mark_synched_dirty(VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID);
    }
    let to_owner = Vec3d {
        x: owner.eye.x - position.x,
        y: owner.eye.y - position.y,
        z: owner.eye.z - position.z,
    };
    let loyalty = f64::from(loyalty);
    entity.set_pos(Vec3d {
        y: position.y + to_owner.y * 0.015 * loyalty,
        ..position
    });
    let pull = to_owner.normalize().scale(0.05 * loyalty);
    entity.set_delta_movement(entity.delta_movement().scale(0.95).plus(pull));
    // `clientSideReturnTridentTickCount == 0`: the first tick it turns back.
    if !returning && !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:item.trident.return".to_string(),
            source: SoundSource::Neutral,
            volume: 10.0,
            pitch: 1.0,
        });
    }
    true
}

fn data(entity: &Entity) -> &TridentData {
    let EntityData::Trident(data) = &entity.data else {
        unreachable!("expected a trident");
    };
    data
}

fn data_mut(entity: &mut Entity) -> &mut TridentData {
    let EntityData::Trident(data) = &mut entity.data else {
        unreachable!("expected a trident");
    };
    data
}

pub fn spawn_trident(
    store: &mut EntityStore,
    position: Vec3d,
    delta_movement: Vec3d,
    item: RuntimeItemStack,
) -> Option<i32> {
    crate::entity::abstract_arrow::spawn(
        store,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
        position,
        delta_movement,
        EntityData::Trident(TridentData::new(item)),
    )
}

/// `new ThrownTrident(level, owner, item)` (`ThrownTrident.java:42-46`)
/// shot as `Projectile.spawnProjectileFromRotation` shoots it.
pub(crate) fn spawn_thrown(
    store: &mut EntityStore,
    owner: uuid::Uuid,
    position: Vec3d,
    item: RuntimeItemStack,
    pickup: ArrowPickup,
    shot: crate::entity::projectile_shoot::ProjectileShot,
    value_level: &crate::world::ValueEffectLevel<'_>,
) -> Option<i32> {
    let spawned_item = item.clone();
    let mut trident = TridentData::new(item);
    trident.projectile.set_owner(Some(owner));
    trident.set_pickup(pickup);
    let effects = store.weapon_effects_shared();
    let random = store.random().clone();
    let uuid = store.next_uuid();
    let entity_id = store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
        position,
        EntityData::Trident(trident),
        |entity| {
            shot.install(entity);
            let loyalty = data(entity).loyalty_from_item(
                &effects,
                crate::item_durability::ValueContext::stored(entity, None),
                &random,
                value_level,
            );
            data_mut(entity).loyalty = Some(loyalty);
        },
    )?;
    crate::entity::projectile_weapon::queue_spawn_effects(store, entity_id, spawned_item);
    Some(entity_id)
}

/// `AbstractArrow` and `ThrownTrident` synched rows after the `Entity` rows.
pub fn synched_values(entity: &Entity, data: &TridentData) -> Vec<SynchedValue> {
    use bbb_protocol::entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    };
    use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};

    let row = |data_id: u8, serializer_id: i32, value: EntityDataValueKind, is_default: bool| {
        SynchedValue {
            value: EntityDataValue {
                data_id,
                serializer_id,
                value,
            },
            is_default,
        }
    };
    let mut rows = crate::entity::abstract_arrow::synched_values(entity, data);
    rows.push(row(
        VANILLA_TRIDENT_LOYALTY_DATA_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        EntityDataValueKind::Byte(data.loyalty()),
        data.loyalty() == 0,
    ));
    rows.push(row(
        VANILLA_TRIDENT_FOIL_DATA_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        EntityDataValueKind::Boolean(data.foil),
        !data.foil,
    ));
    rows
}

fn item_has_foil(item: &RuntimeItemStack) -> bool {
    bbb_server_types::item_stack::static_item_component(
        item.summary(),
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
    )
    .expect("validated item component")
    .map(|patch| {
        patch
            .enchantment_glint_override
            .expect("validated glint override")
    })
    .unwrap_or_else(|| crate::entity::item_components::enchantment_count(item) > 0)
}
