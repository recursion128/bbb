//! Borrowed entity and session queries for one selector pass.

use super::super::sensing::Sensing;
use super::TargetRef;
use crate::entity::living::LivingEntityState;
use crate::entity::{Entity, EntityData, EntityStore, NearbyPlayer};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{vanilla_entity_type, VANILLA_ENTITY_TYPE_PLAYER_ID};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Aabb;
use uuid::Uuid;

#[derive(Clone, Copy)]
enum Subject<'a> {
    Stored(&'a Entity),
    Player(&'a NearbyPlayer),
    Foreign(&'a Entity, &'a EntityStore),
    ForeignPlayer(&'a NearbyPlayer),
}

#[derive(Clone, Copy)]
pub(crate) struct Target<'a, 'l, 'w> {
    subject: Subject<'a>,
    body: &'a Entity,
    detached: Option<&'a Entity>,
    store: &'a EntityStore,
    level: &'a LevelCtx<'l, 'w>,
    sensing: &'a Sensing,
}

enum TargetWorld<'a, 'l, 'w> {
    Shared {
        body_entity: &'a Entity,
        store: &'a EntityStore,
        level: &'a LevelCtx<'l, 'w>,
    },
    Mut {
        body_entity: &'a mut Entity,
        store: &'a mut EntityStore,
        level: &'a mut LevelCtx<'l, 'w>,
        aborted: bool,
    },
}

pub struct TargetView<'a, 'l, 'w> {
    world: TargetWorld<'a, 'l, 'w>,
    detached: Option<&'a Entity>,
    sensing: &'a Sensing,
}

impl<'a, 'l, 'w> TargetView<'a, 'l, 'w> {
    pub(crate) fn new(
        body: &'a Entity,
        store: &'a EntityStore,
        level: &'a LevelCtx<'l, 'w>,
        sensing: &'a Sensing,
    ) -> Self {
        Self {
            world: TargetWorld::Shared {
                body_entity: body,
                store,
                level,
            },
            detached: None,
            sensing,
        }
    }

    pub(crate) fn new_mut(
        body: &'a mut Entity,
        store: &'a mut EntityStore,
        level: &'a mut LevelCtx<'l, 'w>,
        sensing: &'a Sensing,
    ) -> Self {
        Self {
            world: TargetWorld::Mut {
                body_entity: body,
                store,
                level,
                aborted: false,
            },
            detached: None,
            sensing,
        }
    }

    pub(crate) fn valid_shared_target(
        body: &'a Entity,
        store: &'a EntityStore,
        level: &'a LevelCtx<'l, 'w>,
        sensing: &'a Sensing,
        target: TargetRef,
    ) -> Option<Target<'a, 'l, 'w>> {
        let base = Target {
            subject: Subject::Stored(body),
            body,
            detached: None,
            store,
            level,
            sensing,
        };
        let entity = base
            .lookup(target.entity_id)
            .filter(|entity| entity.uuid() == target.uuid)?;
        let view = Self::new(body, store, level, sensing);
        super::conditions::is_valid_target(&view, &entity).then_some(entity)
    }

    #[inline]
    pub(crate) fn body_entity(&self) -> &Entity {
        match &self.world {
            TargetWorld::Shared { body_entity, .. } => body_entity,
            TargetWorld::Mut { body_entity, .. } => body_entity,
        }
    }

    #[inline]
    pub(crate) fn store(&self) -> &EntityStore {
        match &self.world {
            TargetWorld::Shared { store, .. } => store,
            TargetWorld::Mut { store, .. } => store,
        }
    }

    #[inline]
    pub(crate) fn level(&self) -> &LevelCtx<'l, 'w> {
        match &self.world {
            TargetWorld::Shared { level, .. } => level,
            TargetWorld::Mut { level, .. } => level,
        }
    }

    #[inline]
    pub(crate) fn world_mut(
        &mut self,
    ) -> Option<(&mut Entity, &mut EntityStore, &mut LevelCtx<'l, 'w>)> {
        match &mut self.world {
            TargetWorld::Shared { .. } => None,
            TargetWorld::Mut {
                body_entity,
                store,
                level,
                ..
            } => Some((body_entity, store, level)),
        }
    }

    #[inline]
    pub(crate) fn is_aborted(&self) -> bool {
        matches!(self.world, TargetWorld::Mut { aborted: true, .. })
    }

    #[inline]
    pub(crate) fn abort(&mut self) {
        if let TargetWorld::Mut { aborted, .. } = &mut self.world {
            *aborted = true;
        }
    }

    pub(crate) fn set_pose(&mut self, pose_id: i32) {
        if let TargetWorld::Mut {
            body_entity,
            store,
            level,
            aborted,
        } = &mut self.world
        {
            if body_entity
                .set_pose_id_with_level(pose_id, level, Some(&mut **store))
                .is_none()
            {
                *aborted = true;
            }
        }
    }

    pub(crate) fn play_body_sound(
        &mut self,
        resource_id: &'static str,
        source: bbb_protocol::packets::SoundSource,
        volume: f32,
        pitch: f32,
    ) {
        if let Some((entity, store, _)) = self.world_mut() {
            store.play_sound_from_entity_at(
                entity.id(),
                entity.position(),
                resource_id,
                source,
                volume,
                pitch,
            );
        }
    }

    pub(crate) fn swing_main_hand(&mut self) {
        if let Some((entity, _, level)) = self.world_mut() {
            let hand = bbb_protocol::packets::InteractionHand::MainHand;
            if entity.living_mut().is_some_and(|living| living.swing(hand)) {
                level.entity_animation(entity.id(), hand);
            }
        }
    }

    pub(crate) fn set_aggressive(&mut self, aggressive: bool) {
        if let Some((entity, _, _)) = self.world_mut() {
            crate::entity::general_living::set_aggressive(entity, aggressive);
        }
    }

    pub(crate) fn set_using_item(&mut self, hand: Option<bbb_protocol::packets::InteractionHand>) {
        if let Some((entity, _, level)) = self.world_mut() {
            crate::entity::living::item_use::set_using_item(entity, level, hand);
        }
    }

    pub(crate) fn body_level_event(&mut self, event_id: i32) {
        if let Some((entity, _, level)) = self.world_mut() {
            if !entity.is_silent() {
                level.level_event(event_id, entity.block_position(), 0);
            }
        }
    }

    pub(crate) fn body_entity_event(&mut self, event_id: i8) {
        if let Some((entity, store, _)) = self.world_mut() {
            if matches!(event_id, 58 | 59) {
                crate::entity::goat::set_lowering_head(entity, event_id == 58);
            }
            store.broadcast_to_tracking(
                entity.id(),
                crate::connection::ClientboundPlayPacket::EntityEvent(
                    bbb_protocol::packets::EntityEvent {
                        entity_id: entity.id(),
                        event_id,
                    },
                ),
            );
        }
    }

    pub(crate) fn set_body_delta_movement(&mut self, movement: Vec3d) {
        if let Some((entity, _, _)) = self.world_mut() {
            entity.set_delta_movement(movement);
        }
    }

    pub(crate) fn set_body_discard_friction(&mut self, discard: bool) {
        if let Some((entity, _, _)) = self.world_mut() {
            entity.set_discard_friction(discard);
        }
    }

    pub(crate) fn queue_command(&mut self, command: crate::entity::EntityCommand) {
        if let Some((_, store, _)) = self.world_mut() {
            store.queue_command(command);
        }
    }

    pub(crate) fn with_detached(mut self, detached: Option<&'a Entity>) -> Self {
        self.detached = detached;
        self
    }
    fn target<'v>(&'v self, subject: Subject<'v>) -> Target<'v, 'l, 'w> {
        Target {
            subject,
            body: self.body_entity(),
            detached: self.detached,
            store: self.store(),
            level: self.level(),
            sensing: self.sensing,
        }
    }
    pub(crate) fn body(&self) -> Target<'_, 'l, 'w> {
        self.target(Subject::Stored(self.body_entity()))
    }
    pub(crate) fn facts(&self, id: i32) -> Option<Target<'_, 'l, 'w>> {
        self.body().lookup(id)
    }
    pub(crate) fn facts_of(&self, target: TargetRef) -> Option<Target<'_, 'l, 'w>> {
        self.facts(target.entity_id)
            .filter(|entity| entity.uuid() == target.uuid)
    }
    pub(crate) fn by_uuid(&self, uuid: Uuid) -> Option<Target<'_, 'l, 'w>> {
        self.body().lookup_uuid(uuid)
    }
    pub(crate) fn player_by_uuid(&self, uuid: Uuid) -> Option<Target<'_, 'l, 'w>> {
        self.by_uuid(uuid).filter(|target| target.is_player())
    }
    pub(crate) fn local_stored_in(
        &self,
        area: Aabb,
    ) -> impl Iterator<Item = Target<'_, 'l, 'w>> + '_ {
        let body = self.body_entity();
        let store = self.store();
        let level = self.level();
        let sensing = self.sensing;
        let detached = self.detached;
        store.box_section_ids(area).filter_map(move |id| {
            let entity = store.get(id)?;
            (id != body.id() && entity.living().is_some() && entity.bounding_box().intersects(area))
                .then_some(Target {
                    subject: Subject::Stored(entity),
                    detached,
                    body,
                    store,
                    level,
                    sensing,
                })
        })
    }
    pub(crate) fn local_stored(&self) -> impl Iterator<Item = Target<'_, 'l, 'w>> + '_ {
        let body = self.body_entity();
        let range = body.living().map_or(16.0, |living| {
            living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
        });
        let reach = body
            .bounding_box()
            .inflate(range + 1.0, range.max(10.0) + 1.0, range + 1.0);
        self.local_stored_in(reach)
    }
    pub(crate) fn local_players(&self) -> impl Iterator<Item = Target<'_, 'l, 'w>> + '_ {
        let body = self.body_entity();
        let store = self.store();
        let level = self.level();
        let sensing = self.sensing;
        let detached = self.detached;
        level.players().iter().map(move |player| Target {
            subject: Subject::Player(player),
            detached,
            body,
            store,
            level,
            sensing,
        })
    }
    pub(crate) fn local_living(&self) -> impl Iterator<Item = Target<'_, 'l, 'w>> + '_ {
        self.local_stored().chain(self.local_players())
    }
    pub(crate) fn first_living_in_box<'v>(
        &'v self,
        bounds: Aabb,
        mut predicate: impl FnMut(&Target<'v, 'l, 'w>) -> bool,
    ) -> Option<Target<'v, 'l, 'w>> {
        let body = self.body_entity();
        self.store()
            .first_entity_in_box(bounds, |entity| {
                entity.id() != body.id()
                    && entity.living().is_some()
                    && predicate(&self.target(Subject::Stored(entity)))
            })
            .map(|entity| self.target(Subject::Stored(entity)))
            .or_else(|| {
                self.local_players()
                    .find(|target| target.bounding_box().intersects(bounds) && predicate(target))
            })
    }

    pub(crate) fn peaceful(&self) -> bool {
        self.store()
            .difficulty()
            .is_none_or(|difficulty| difficulty == bbb_protocol::difficulties::Difficulty::Peaceful)
    }
    pub(crate) fn game_time(&self) -> i64 {
        self.store().game_time()
    }
    pub(crate) fn universal_anger(&self) -> bool {
        self.store().target_level_facts().universal_anger
    }
    pub(crate) fn pvp(&self) -> bool {
        self.store().target_level_facts().pvp
    }
    pub(crate) fn bright_outside(&self) -> Option<bool> {
        self.store().target_level_facts().bright_outside
    }
    pub(crate) fn has_active_raid(&self) -> bool {
        matches!(self.body_entity().data(), EntityData::GeneralLiving(data) if data.raider_retained().is_some_and(|raider| matches!(raider.raid(),crate::entity::raider::RaiderRaid::Current(_))))
    }
    pub(crate) fn light_magic_value(&self) -> Option<f32> {
        crate::entity::level::light_level_dependent_magic_value(
            self.level(),
            BlockPos {
                x: self.body_entity().position().x.floor() as i32,
                y: self.body().eye_y().floor() as i32,
                z: self.body_entity().position().z.floor() as i32,
            },
        )
    }
    pub(crate) fn within_home(&self, pos: BlockPos) -> bool {
        self.body_entity().mob_home().is_none_or(|(home, radius)| {
            let dx = f64::from(home.x - pos.x);
            let dy = f64::from(home.y - pos.y);
            let dz = f64::from(home.z - pos.z);
            dx * dx + dy * dy + dz * dz < f64::from(radius * radius)
        })
    }
    pub(crate) fn last_hurt_by_mob(&self) -> Option<Target<'_, 'l, 'w>> {
        self.body().last_hurt_by_mob().and_then(|id| self.facts(id))
    }
    pub(crate) fn leash_holder(&self) -> Option<i32> {
        crate::entity::leash::leash_holder(self.body_entity())
    }
    pub(crate) fn bee_has_stung(&self) -> bool {
        matches!(self.body_entity().data(),EntityData::GeneralLiving(data) if data.bee_retained().is_some_and(|bee|bee.has_stung()))
    }
    pub(crate) fn trusts(&self, uuid: Uuid) -> bool {
        matches!(self.body_entity().data(),EntityData::GeneralLiving(data) if data.fox_retained().is_some_and(|fox| fox.trusts_uuid(uuid)))
    }
    pub(crate) fn johnny(&self) -> bool {
        matches!(self.body_entity().data(),EntityData::GeneralLiving(data) if data.raider_retained().and_then(|state|state.johnny())==Some(true))
    }
    pub(crate) fn attach_axis(&self) -> Option<u8> {
        let EntityData::GeneralLiving(data) = self.body_entity().data() else {
            return None;
        };
        Some(match data.shulker_state()?.attach_face() {
            bbb_protocol::packets::Direction::Down | bbb_protocol::packets::Direction::Up => 1,
            bbb_protocol::packets::Direction::North | bbb_protocol::packets::Direction::South => 2,
            bbb_protocol::packets::Direction::West | bbb_protocol::packets::Direction::East => 0,
        })
    }
    pub(crate) fn friendly_fire_allowed(&self, team: u32) -> bool {
        self.level().scoreboard_teams().allows_friendly_fire(team)
    }
}

impl<'a, 'l, 'w> Target<'a, 'l, 'w> {
    pub(crate) fn is_local(self) -> bool {
        matches!(self.subject, Subject::Stored(_) | Subject::Player(_))
    }

    fn with_subject(self, subject: Subject<'a>) -> Self {
        Self { subject, ..self }
    }
    fn lookup(self, id: i32) -> Option<Self> {
        if id == self.body.id() {
            return Some(self.with_subject(Subject::Stored(self.body)));
        }
        if let Some(entity) = self.detached.filter(|entity| entity.id() == id) {
            return Some(self.with_subject(Subject::Stored(entity)));
        }
        if let Some(entity) = self
            .store
            .get(id)
            .filter(|entity| entity.living().is_some())
        {
            return Some(self.with_subject(Subject::Stored(entity)));
        }
        if let Some(player) = self
            .level
            .players()
            .iter()
            .find(|player| player.entity_id == id)
        {
            return Some(self.with_subject(Subject::Player(player)));
        }
        if let Some((entity, store)) = self.level.stored_entity_by_id_in_other_dimension(id) {
            return entity
                .living()
                .is_some()
                .then(|| self.with_subject(Subject::Foreign(entity, store)));
        }
        self.level
            .target_input_by_id_in_other_dimension(id)
            .map(|player| self.with_subject(Subject::ForeignPlayer(player)))
    }
    fn lookup_uuid(self, uuid: Uuid) -> Option<Self> {
        if uuid == self.body.uuid() {
            return Some(self.with_subject(Subject::Stored(self.body)));
        }
        if let Some(id) = self.store.visible_entity_id_for_uuid(uuid) {
            return self.lookup(id);
        }
        if let Some(player) = self
            .level
            .players()
            .iter()
            .find(|player| player.uuid == uuid)
        {
            return Some(self.with_subject(Subject::Player(player)));
        }
        if let Some((entity, store)) = self.level.stored_entity_in_other_dimension(uuid) {
            return entity
                .living()
                .is_some()
                .then(|| self.with_subject(Subject::Foreign(entity, store)));
        }
        self.level
            .target_input_in_other_dimension(uuid)
            .map(|player| self.with_subject(Subject::ForeignPlayer(player)))
    }
    #[cfg(test)]
    pub(crate) fn in_body_level(self) -> bool {
        matches!(self.subject, Subject::Stored(_) | Subject::Player(_))
    }
    pub(crate) fn player(self) -> Option<&'a NearbyPlayer> {
        match self.subject {
            Subject::Player(player) | Subject::ForeignPlayer(player) => Some(player),
            _ => None,
        }
    }
    pub(crate) fn living(self) -> Option<&'a LivingEntityState> {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.living(),
            Subject::Player(_) | Subject::ForeignPlayer(_) => None,
        }
    }
    pub(crate) fn entity_id(self) -> i32 {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.id(),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.entity_id,
        }
    }
    pub(crate) fn uuid(self) -> Uuid {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.uuid(),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.uuid,
        }
    }
    pub(crate) fn entity_type_id(self) -> i32 {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.entity_type_id(),
            Subject::Player(_) | Subject::ForeignPlayer(_) => VANILLA_ENTITY_TYPE_PLAYER_ID,
        }
    }
    pub(crate) fn position(self) -> Vec3d {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.position(),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.position,
        }
    }
    pub(crate) fn delta_movement(self) -> Vec3d {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.delta_movement(),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.delta_movement,
        }
    }
    pub(crate) fn eye_y(self) -> f64 {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => {
                entity.position().y + f64::from(entity.eye_height())
            }
            Subject::Player(player) | Subject::ForeignPlayer(player) => {
                player.position.y + player.eye_height
            }
        }
    }
    pub(crate) fn bounding_box(self) -> Aabb {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.bounding_box(),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.bounding_box(),
        }
    }
    pub(crate) fn alive(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => {
                entity.is_alive()
                    && entity
                        .living()
                        .is_none_or(|living| !living.is_dead_or_dying())
            }
            Subject::Player(player) | Subject::ForeignPlayer(player) => !player.dead_or_dying,
        }
    }
    pub(crate) fn dead_or_dying(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity
                .living()
                .is_some_and(|living| living.is_dead_or_dying()),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.dead_or_dying,
        }
    }
    pub(crate) fn invulnerable(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.invulnerable(),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.invulnerable,
        }
    }
    pub(crate) fn invisible(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.is_invisible(),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.invisible,
        }
    }
    pub(crate) fn discrete(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.get_shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            ),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.discrete,
        }
    }
    pub(crate) fn armor_cover(self) -> f32 {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity
                .living()
                .map_or(0.0, |living| living.armor_cover_percentage()),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.armor_cover,
        }
    }
    pub(crate) fn head_item_id(self) -> Option<i32> {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity
                .living()
                .and_then(|living| living.item_by_slot(EquipmentSlot::Head))
                .and_then(|stack| stack.item_id()),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player
                .head_item_resource_id
                .and_then(bbb_protocol::item_types::vanilla_item_type_for_resource_id)
                .map(|item| item.id),
        }
    }
    pub(crate) fn baby(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => {
                matches!(entity.data(),EntityData::GeneralLiving(data) if data.is_baby())
            }
            Subject::Player(_) | Subject::ForeignPlayer(_) => false,
        }
    }
    pub(crate) fn in_water(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.is_in_water(),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.in_water,
        }
    }
    pub(crate) fn armor_stand_marker(self) -> Option<bool> {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => match entity.data() {
                EntityData::ArmorStand(data) => Some(data.is_marker()),
                _ => None,
            },
            Subject::Player(_) | Subject::ForeignPlayer(_) => None,
        }
    }
    pub(crate) fn playing_dead(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => {
                crate::entity::axolotl::is_playing_dead(entity)
            }
            Subject::Player(_) | Subject::ForeignPlayer(_) => false,
        }
    }
    pub(crate) fn tame(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => {
                matches!(entity.data(),EntityData::GeneralLiving(data) if data.is_tame() || data.abstract_horse_retained().is_some_and(|horse|horse.is_tame()))
            }
            Subject::Player(_) | Subject::ForeignPlayer(_) => false,
        }
    }
    // `Armadillo`: every state except IDLE counts as scared.
    pub(crate) fn armadillo_is_scared(self) -> bool {
        match self.subject {
            Subject::Stored(entity) => crate::entity::armadillo::is_scared(entity),
            _ => false,
        }
    }
    pub(crate) fn ordered_to_sit(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => {
                matches!(entity.data(),EntityData::GeneralLiving(data) if data.tamable_retained().is_some_and(|tame|tame.ordered_to_sit()))
            }
            Subject::Player(_) | Subject::ForeignPlayer(_) => false,
        }
    }
    pub(crate) fn player_created(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => matches!(
                entity.data(),
                EntityData::GeneralLiving(data) if data.iron_golem_player_created() == Some(true)
            ),
            Subject::Player(_) | Subject::ForeignPlayer(_) => false,
        }
    }
    pub(crate) fn aggressive(self) -> bool {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => {
                matches!(entity.data(),EntityData::GeneralLiving(data) if data.mob_flags().is_some_and(|flags|flags & bbb_protocol::entity_synched_data::VANILLA_MOB_FLAG_AGGRESSIVE != 0))
            }
            Subject::Player(_) | Subject::ForeignPlayer(_) => false,
        }
    }
    pub(crate) fn panda_is_aggressive(self) -> bool {
        // Panda: ally alerts use personality, not mob flags.
        match self.subject {
            Subject::Stored(entity) => {
                matches!(entity.data(),EntityData::GeneralLiving(data) if data.panda_retained().is_some_and(|panda|panda.is_aggressive()))
            }
            Subject::Player(_) | Subject::ForeignPlayer(_) | Subject::Foreign(..) => false,
        }
    }
    pub(crate) fn tick_count(self) -> i32 {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.tick_count(),
            Subject::Player(player) | Subject::ForeignPlayer(player) => player.hurt.tick_count,
        }
    }
    pub(crate) fn last_hurt_by_mob_timestamp(self) -> i32 {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity
                .living()
                .map_or(0, |living| living.last_hurt_by_mob_timestamp),
            Subject::Player(player) | Subject::ForeignPlayer(player) => {
                player.hurt.last_hurt_by_mob_timestamp
            }
        }
    }
    pub(crate) fn last_hurt_mob_timestamp(self) -> i32 {
        match self.subject {
            Subject::Stored(_) => 0,
            Subject::Player(player) | Subject::ForeignPlayer(player) => {
                player.hurt.last_hurt_mob_timestamp
            }
            Subject::Foreign(..) => 0,
        }
    }
    pub(crate) fn target_ref(self) -> TargetRef {
        TargetRef {
            entity_id: self.entity_id(),
            uuid: self.uuid(),
        }
    }
    pub(crate) fn is_player(self) -> bool {
        self.entity_type_id() == VANILLA_ENTITY_TYPE_PLAYER_ID
    }
    pub(crate) fn warden_targetable_from(self, attacker: &Target<'_, '_, '_>) -> bool {
        let living = match self.subject {
            Subject::Stored(entity) => entity.living().is_some(),
            Subject::Player(_) | Subject::ForeignPlayer(_) => true,
            Subject::Foreign(..) => false,
        };
        living
            && self.level.world_border().is_some_and(|border| {
                crate::entity::warden::can_target(
                    self.entity_type_id(),
                    self.is_local() && attacker.is_local(),
                    self.creative() || self.spectator(),
                    crate::entity::ScoreboardTeams::allied(attacker.team(), self.team()),
                    self.invulnerable(),
                    !self.alive(),
                    border,
                    self.level.game_time(),
                    self.bounding_box(),
                )
            })
    }
    pub(crate) fn creative(self) -> bool {
        self.player().is_some_and(|player| player.instabuild)
    }
    pub(crate) fn spectator(self) -> bool {
        self.player().is_some_and(|player| player.spectator)
    }
    pub(crate) fn block_position(self) -> BlockPos {
        let pos = self.position();
        BlockPos {
            x: pos.x.floor() as i32,
            y: pos.y.floor() as i32,
            z: pos.z.floor() as i32,
        }
    }
    pub(crate) fn distance_to_sqr(self, x: f64, y: f64, z: f64) -> f64 {
        let p = self.position();
        let dx = p.x - x;
        let dy = p.y - y;
        let dz = p.z - z;
        dx * dx + dy * dy + dz * dz
    }
    pub(crate) fn distance_to_facts_sqr(self, other: &Target<'_, '_, '_>) -> f64 {
        let p = other.position();
        self.distance_to_sqr(p.x, p.y, p.z)
    }
    fn ownable_owner(self) -> Option<Option<Uuid>> {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => entity.ownable_owner(),
            _ => None,
        }
    }
    pub(crate) fn owner(self) -> Option<i32> {
        if let Some(uuid) = self.ownable_owner().flatten() {
            return self.lookup_uuid(uuid).map(|owner| owner.entity_id());
        }
        let uuid = match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => match entity.data() {
                EntityData::GeneralLiving(data) => data.vex_retained().and_then(|vex| vex.owner()),
                _ => None,
            },
            _ => None,
        }?;
        self.lookup_uuid(uuid)
            .filter(|owner| {
                crate::entity::ai::mob::spec::runtime_class(owner.entity_type_id()).is_some()
            })
            .map(|owner| owner.entity_id())
    }
    pub(crate) fn root_owner(self) -> Option<i32> {
        let mut uuid = self.ownable_owner().flatten()?;
        let mut seen = Vec::new();
        loop {
            if seen.contains(&uuid) {
                return None;
            }
            seen.push(uuid);
            let owner = self.lookup_uuid(uuid)?;
            match owner.ownable_owner() {
                None => return Some(owner.entity_id()),
                Some(None) => return None,
                Some(Some(next)) => uuid = next,
            }
        }
    }
    pub(crate) fn team(self) -> Option<u32> {
        let teams = self.level.scoreboard_teams();
        if teams.is_empty() {
            return None;
        }
        let team_of = |target: Self| {
            if let Some(player) = target.player() {
                self.level.player_team(self.store, player.uuid)
            } else {
                teams.entity_team(target.uuid())
            }
        };
        team_of(self).or_else(|| {
            self.tame()
                .then(|| self.root_owner())
                .flatten()
                .and_then(|owner| self.lookup(owner))
                .and_then(team_of)
        })
    }
    pub(crate) fn illager_friend(self) -> bool {
        self.level.registry_tag_membership(
            "minecraft:entity_type",
            "minecraft:illager_friends",
            vanilla_entity_type(self.entity_type_id()).map_or("", |kind| kind.resource_id),
        )
    }
    pub(crate) fn wither_friend(self) -> bool {
        self.level.registry_tag_membership(
            "minecraft:entity_type",
            "minecraft:wither_friends",
            vanilla_entity_type(self.entity_type_id()).map_or("", |kind| kind.resource_id),
        )
    }
    pub(crate) fn rides_body(self) -> Option<bool> {
        self.store
            .checked_has_indirect_passenger(self.body, self.entity_id())
    }
    pub(crate) fn valid_target(self) -> Option<TargetRef> {
        let held = match self.subject {
            Subject::Stored(entity) => self
                .store
                .mob_ai(entity.id())
                .and_then(|runtime| runtime.target_ref()),
            Subject::Foreign(entity, store) => store
                .mob_ai(entity.id())
                .and_then(|runtime| runtime.target_ref()),
            _ => None,
        }?;
        let target = self
            .lookup(held.entity_id)
            .filter(|target| target.uuid() == held.uuid)?;
        super::conditions::as_valid_target(
            self.store.difficulty().is_none_or(|difficulty| {
                difficulty == bbb_protocol::difficulties::Difficulty::Peaceful
            }),
            &self,
            &target,
        )
        .then_some(held)
    }
    pub(crate) fn last_hurt_by_mob(self) -> Option<i32> {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => {
                let reference = entity.living()?.last_hurt_by_mob()?;
                reference
                    .stored_entity_id()
                    .and_then(|id| self.lookup(id))
                    .or_else(|| reference.uuid().and_then(|uuid| self.lookup_uuid(uuid)))
                    .map(|target| target.entity_id())
            }
            Subject::Player(player) | Subject::ForeignPlayer(player) => player
                .hurt
                .last_hurt_by_mob
                .and_then(|uuid| self.lookup_uuid(uuid))
                .map(|target| target.entity_id()),
        }
    }
    pub(crate) fn last_hurt_mob(self) -> Option<i32> {
        self.player()?
            .hurt
            .last_hurt_mob
            .and_then(|uuid| self.lookup_uuid(uuid))
            .map(|target| target.entity_id())
    }
    fn sight_positions(self) -> (Vec3d, Vec3d) {
        (
            Vec3d {
                y: self.body.position().y + f64::from(self.body.eye_height()),
                ..self.body.position()
            },
            Vec3d {
                y: self.eye_y(),
                ..self.position()
            },
        )
    }
    pub(crate) fn line_of_sight(self) -> bool {
        self.sensing.has_line_of_sight(self.entity_id(), || {
            let (from, to) = self.sight_positions();
            let dx = to.x - from.x;
            let dy = to.y - from.y;
            let dz = to.z - from.z;
            dx * dx + dy * dy + dz * dz <= 128.0 * 128.0
                && crate::entity::projectile_hit::collision_clip_misses(self.level, from, to)
        })
    }
    pub(crate) fn direct_line_of_sight(self) -> Option<bool> {
        let (from, to) = self.sight_positions();
        super::super::sensing::body_line_of_sight(self.level, self.body, from, to)
    }
    pub(crate) fn checked_line_of_sight(self) -> Option<bool> {
        self.sensing
            .has_line_of_sight_checked(self.entity_id(), || self.direct_line_of_sight())
    }
    pub(crate) fn reputation_of(self, uuid: Uuid) -> i32 {
        match self.subject {
            Subject::Stored(entity) | Subject::Foreign(entity, _) => {
                gossip_reputation_of(entity, uuid)
            }
            _ => 0,
        }
    }
}

fn gossip_reputation_of(entity: &Entity, uuid: Uuid) -> i32 {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return 0;
    };
    let Some(gossips) = data.villager_gossips() else {
        return 0;
    };
    crate::entity::general_living::gossip_reputation(gossips, uuid)
}
