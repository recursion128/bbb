//! Raid leader banner pursuit over the mob's canonical navigation.

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_RAVAGER_ID, VANILLA_ENTITY_TYPE_WITCH_ID};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::geometry::Vec3Ops;
use rustc_hash::FxHashMap;
use uuid::Uuid;

use crate::entity::general_living::spawn_finalize_equipment::is_ominous_banner;
use crate::entity::raider::RaiderRaid;
use crate::entity::store::EntityBodyReadView;
use crate::entity::{Entity, EntityData};

use super::navigation::{containing_block, GroundNavigation, MobPathingSnapshot};
use super::target::{TargetRef, TargetView};
use super::{Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(super) struct RaiderBannerState {
    pursued: Option<TargetRef>,
    failed_until: FxHashMap<i32, (Uuid, i64)>,
    search: bool,
    pickup: Option<TargetRef>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct RaiderBannerGoal;

fn eligible(targets: &TargetView<'_, '_, '_>) -> bool {
    let body = targets.body_entity();
    if matches!(
        body.entity_type_id(),
        VANILLA_ENTITY_TYPE_WITCH_ID | VANILLA_ENTITY_TYPE_RAVAGER_ID
    ) {
        return false;
    }
    let EntityData::GeneralLiving(data) = body.data() else {
        return false;
    };
    let Some(state) = data.raider_retained() else {
        return false;
    };
    let RaiderRaid::Current(raid_id) = state.raid() else {
        return false;
    };
    let Some(wave) = targets.level().raider_wave(raid_id, state.wave()) else {
        return false;
    };
    if !wave.active || wave.is_over {
        return false;
    }
    if body
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::Head))
        .is_some_and(|stack| is_ominous_banner(stack, targets.level().item_stack_resolver()))
    {
        return false;
    }
    let bodies = EntityBodyReadView::new(
        targets.store(),
        Some(body),
        targets.level().detached_bodies(),
    );
    !wave
        .leader
        .and_then(|uuid| bodies.get_by_uuid(uuid))
        .is_some_and(|leader| leader.is_alive_with_health())
}

fn pursued_item<'a>(targets: &'a TargetView<'_, '_, '_>, target: TargetRef) -> Option<&'a Entity> {
    EntityBodyReadView::new(
        targets.store(),
        Some(targets.body_entity()),
        targets.level().detached_bodies(),
    )
    .get(target.entity_id)
    .filter(|item| {
        item.uuid() == target.uuid && item.is_alive() && matches!(item.data(), EntityData::Item(_))
    })
}

impl Goal for RaiderBannerGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !eligible(targets) {
            return false;
        }
        let Some(state) = context.species.raider_banner.as_mut() else {
            return false;
        };
        state.pursued = None;
        state.search = true;
        true
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if context.navigation_done() || !eligible(targets) {
            return false;
        }
        let Some(target) = context
            .species
            .raider_banner
            .as_ref()
            .and_then(|state| state.pursued)
        else {
            return false;
        };
        pursued_item(targets, target).is_some()
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        if let Some(state) = context.species.raider_banner.as_mut() {
            state.pursued = None;
            state.search = false;
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(state) = context.species.raider_banner.as_mut() else {
            return;
        };
        let Some(target) = state.pursued else {
            return;
        };
        let Some(item) = pursued_item(targets, target) else {
            return;
        };
        if targets
            .body_entity()
            .position()
            .distance_to_sqr(item.position())
            < 1.414 * 1.414
        {
            state.pickup = Some(target);
        }
    }
}

pub(super) fn resolve_path(
    context: &mut MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    navigation: &mut GroundNavigation,
    mob: Option<MobPathingSnapshot>,
) -> Option<bool> {
    if !context.species.raider_banner.as_ref()?.search {
        return None;
    }
    let mob = mob.unwrap_or_else(|| MobPathingSnapshot::from_context(context));
    let state = context.species.raider_banner.as_mut()?;
    state.search = false;
    let now = targets.level().game_time();
    let body = targets.body_entity();
    let bodies = EntityBodyReadView::new(
        targets.store(),
        Some(body),
        targets.level().detached_bodies(),
    );
    state.failed_until.retain(|id, (uuid, until)| {
        now < *until && bodies.get(*id).is_some_and(|item| item.uuid() == *uuid)
    });
    let range = body
        .living()?
        .attribute_value(VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID);
    let bounds = body.bounding_box().inflate(range, 8.0, range);
    for id in bodies.ids_in_box_sections(bounds) {
        let Some(item) = bodies.get(id) else {
            continue;
        };
        if !item.is_alive() || !item.bounding_box().intersects(bounds) {
            continue;
        }
        let EntityData::Item(data) = item.data() else {
            continue;
        };
        if data.has_pickup_delay()
            || !is_ominous_banner(data.item(), targets.level().item_stack_resolver())
            || state
                .failed_until
                .get(&id)
                .is_some_and(|(uuid, until)| *uuid == item.uuid() && now < *until)
        {
            continue;
        }
        if let Some(path) = navigation
            .create_path_to(
                targets.level(),
                mob,
                containing_block(item.position()),
                1,
                None,
            )
            .filter(|path| path.reached())
        {
            state.pursued = Some(TargetRef {
                entity_id: id,
                uuid: item.uuid(),
            });
            // A reached path can already be done while the banner is within pickup range.
            navigation.start_path(targets.level(), path, mob, 1.15);
            context.nav.navigation_done = navigation.path().is_none_or(super::GroundPath::is_done);
            return Some(true);
        }
        state
            .failed_until
            .insert(id, (item.uuid(), now.saturating_add(600)));
    }
    Some(false)
}

pub(super) fn take_pickup(state: &mut RaiderBannerState) -> Option<TargetRef> {
    state.pickup.take()
}
