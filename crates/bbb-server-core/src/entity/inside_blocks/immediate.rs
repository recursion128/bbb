use super::{Entity, LivingInsideBlockCallback};
use crate::block_behavior::honey;
use crate::block_behavior::inside_contact::{contact, ContactBody, ImmediateContact};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::BlockPos;

/// The block's immediate `entityInside` effect; `Some(false)` leaves the
/// block to the remaining contact dispatch.
pub(super) fn apply<F>(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    store: Option<&mut crate::entity::EntityStore>,
    state: i32,
    pos: BlockPos,
    callback: &mut F,
) -> Option<bool>
where
    F: FnMut(
        &mut Entity,
        &mut LevelCtx<'_, '_>,
        Option<&mut crate::entity::EntityStore>,
        LivingInsideBlockCallback,
    ) -> bool,
{
    let body = ContactBody {
        entity_type_id: entity.entity_type_id(),
        living: entity.living().is_some(),
        weaving: entity
            .living()
            .is_some_and(|living| living.active_effect("minecraft:weaving").is_some()),
    };
    let request = match contact(state, body)? {
        ImmediateContact::Unhandled => return Some(false),
        ImmediateContact::Resolved => return Some(true),
        ImmediateContact::RavagerCrop => {
            if level.mob_griefing() {
                level.destroy_block_by_entity(pos, true, entity, store.as_deref());
            }
            return Some(true);
        }
        ImmediateContact::LilyPad => {
            if crate::entity::boat::is_boat(entity.data()) {
                level.destroy_block_by_entity(pos, true, entity, store.as_deref());
            }
            return Some(true);
        }
        ImmediateContact::DestroyWithoutDrops => {
            level.destroy_block(pos, false);
            return Some(true);
        }
        ImmediateContact::Stuck(multiplier) => {
            // `Spider.makeStuckInBlock` (`Spider.java:118-122`) ignores cobwebs.
            if !entity.is_spider() {
                entity.make_stuck_in_block(multiplier);
            }
            return Some(true);
        }
        ImmediateContact::Hurt(damage_type, amount) => LivingInsideBlockCallback::Damage {
            damage_type,
            amount,
        },
        ImmediateContact::SweetBerry { .. } => LivingInsideBlockCallback::SweetBerryBush { state },
        ImmediateContact::WitherRose(duration) => {
            LivingInsideBlockCallback::WitherRose { duration }
        }
        ImmediateContact::BeeEyeblossom(duration) => {
            LivingInsideBlockCallback::BeeEyeblossom { state, duration }
        }
        ImmediateContact::Honey => {
            if !honey::is_sliding_down(
                pos,
                entity.position(),
                entity.bb_width(),
                entity.on_ground(),
                entity.delta_movement(),
            ) {
                return Some(true);
            }
            entity.set_delta_movement(honey::slide_movement(entity.delta_movement()));
            entity.reset_fall_distance();
            if !(entity.living().is_some()
                || crate::entity::minecart::is_minecart(entity.data())
                || crate::entity::boat::is_boat(entity.data())
                || matches!(entity.data(), crate::entity::EntityData::Tnt(_)))
            {
                return Some(true);
            }
            LivingInsideBlockCallback::HoneySlideEffects
        }
    };
    callback(entity, level, store, request).then_some(true)
}
