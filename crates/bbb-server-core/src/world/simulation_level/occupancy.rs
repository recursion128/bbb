use super::*;

use crate::block_behavior::level::DetectorRailMinecarts;
use crate::entity::store::EntityBodyReadView;
use crate::entity::MinecartContainerData;
use bbb_server_types::item_stack::RuntimeItemStack;

fn is_minecart(data: &EntityData) -> bool {
    // Every `AbstractMinecart` subclass, which is what
    // `getInteractingMinecartOfType(.., AbstractMinecart.class, ..)`
    // (`DetectorRailBlock.java:86`) matches.
    matches!(
        data,
        EntityData::Minecart(_)
            | EntityData::FurnaceMinecart(_)
            | EntityData::ChestMinecart(_)
            | EntityData::HopperMinecart(_)
            | EntityData::CommandBlockMinecart(_)
            | EntityData::SpawnerMinecart(_)
            | EntityData::TntMinecart(_)
    )
}

/// Counts live entities and non-spectator players whose bounding boxes strictly
/// intersect `area`, querying the entity store spatially without copying.
pub(super) fn count_in_box(
    bodies: EntityBodyReadView<'_>,
    players: &[NearbyPlayer],
    area: Aabb,
    query: BlockEntityQuery,
) -> usize {
    let stored = bodies
        .store()
        .all_box_section_ids(area)
        .filter_map(|id| bodies.get(id))
        .filter(|entity| entity.is_alive() && entity.bounding_box().intersects(area))
        .filter(|entity| match query {
            BlockEntityQuery::BlockTriggerAll => !entity.is_ignoring_block_triggers(),
            BlockEntityQuery::BlockTriggerLiving => {
                entity.living().is_some() && !entity.is_ignoring_block_triggers()
            }
            BlockEntityQuery::Minecart => is_minecart(entity.data()),
        })
        .count();
    let online = match query {
        BlockEntityQuery::BlockTriggerAll | BlockEntityQuery::BlockTriggerLiving => players
            .iter()
            .filter(|player| !player.spectator && player.bounding_box().intersects(area))
            .count(),
        BlockEntityQuery::Minecart => 0,
    };
    stored + online
}

/// `DetectorRailBlock.getAnalogOutputSignal`'s two queries
/// (`DetectorRailBlock.java:147-155`), answered spatially from `bodies`.
pub(super) fn detector_rail_minecarts(
    bodies: EntityBodyReadView<'_>,
    area: Aabb,
) -> DetectorRailMinecarts {
    let mut found = DetectorRailMinecarts::default();
    for id in bodies.store().all_box_section_ids(area) {
        let Some(entity) = bodies.get(id) else {
            continue;
        };
        if !entity.is_alive() || !entity.bounding_box().intersects(area) {
            continue;
        }
        match entity.data() {
            EntityData::CommandBlockMinecart(minecart)
                if found.command_block_success_count.is_none() =>
            {
                // `MinecartCommandBlock.getCommandBlock().getSuccessCount()`
                // (`DetectorRailBlock.java:149`).
                found.command_block_success_count = Some(minecart.success_count());
            }
            EntityData::ChestMinecart(minecart) | EntityData::HopperMinecart(minecart)
                if found.container_signal.is_none() =>
            {
                // `getRedstoneSignalFromContainer` over a minecart
                // `EntitySelector.CONTAINER_ENTITY_SELECTOR` keeps (`:152-154`).
                found.container_signal = Some(container_minecart_signal(minecart));
            }
            _ => {}
        }
        if found.command_block_success_count.is_some() && found.container_signal.is_some() {
            break;
        }
    }
    found
}

/// `ComparatorBlock.getItemFrame` (`ComparatorBlock.java:104-114`) and
/// `ItemFrame.getAnalogOutput` (`ItemFrame.java:396-398`), queried spatially.
pub(super) fn comparator_item_frame_output(
    bodies: EntityBodyReadView<'_>,
    pos: BlockPos,
    facing: Direction,
) -> Option<i32> {
    let area = Aabb::of_block(pos);
    let mut matches = bodies
        .store()
        .all_box_section_ids(area)
        .filter_map(|id| bodies.get(id))
        .filter(|entity| entity.is_alive() && entity.bounding_box().intersects(area))
        .filter_map(|entity| match entity.data() {
            EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame)
                if frame.pose().direction() == facing =>
            {
                Some(frame.item().map_or(0, |_| frame.rotation() % 8 + 1))
            }
            _ => None,
        });
    let output = matches.next()?;
    matches.next().is_none().then_some(output)
}

/// `AbstractContainerMenu.getRedstoneSignalFromContainer`
/// (`AbstractContainerMenu.java:751-767`) over a container minecart, whose
/// slots the entity keeps in the persistent stack form.
fn container_minecart_signal(minecart: &MinecartContainerData) -> Option<i32> {
    let items = minecart.items();
    let empty = ItemStackSummary::empty();
    crate::inventory::redstone_signal_from_container_access(
        items.len(),
        |slot| {
            Ok::<_, bbb_server_types::ServerWorldError>(
                items[slot]
                    .as_ref()
                    .map_or(&empty, RuntimeItemStack::summary),
            )
        },
        |error| error,
    )
    .ok()
}

#[cfg(test)]
mod entity_occupancy_tests {
    use super::*;
    use crate::entity::{
        spawn_armor_stand, spawn_marker, spawn_minecart, EntityIdCounter, MarkerData,
    };

    fn player(entity_id: i32, spectator: bool, dead_or_dying: bool) -> NearbyPlayer {
        NearbyPlayer {
            dimensions: bbb_protocol::entity_types::vanilla_entity_type_dimensions(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            )
            .unwrap(),
            collision: None,
            in_water: false,
            armor_cover: 0.0,
            uuid: Uuid::from_u128(entity_id as u128),
            position: Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            known_movement: Vec3d::default(),
            delta_movement: Vec3d::default(),
            eye_height: 1.62,
            spectator,
            dead_or_dying,
            health: if dead_or_dying { 0.0 } else { 20.0 },
            has_slowness: false,
            has_poison: false,
            has_weakness: false,
            luck: 0.0,
            invulnerable: false,
            abilities_invulnerable: false,
            discrete: false,
            sleeping: false,
            invisible: false,
            entity_id,
            instabuild: false,
            may_build: true,
            main_hand_item_id: None,
            off_hand_item_id: None,
            head_item_resource_id: None,
            y_rot: 0.0,
            x_rot: 0.0,
            hurt: Default::default(),
        }
    }

    fn stone_stack(count: i32) -> bbb_server_types::item_stack::RuntimeItemStack {
        let summary = ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count,
            component_patch: Default::default(),
        };
        bbb_server_types::item_stack::RuntimeItemStack::from_summary(summary)
    }

    /// `DetectorRailBlock.getAnalogOutputSignal` (`DetectorRailBlock.java:145-159`)
    /// reads two different things off the minecarts in the rail's box, and the
    /// command-block one wins whichever way round they stand.
    #[test]
    fn a_detector_rails_box_answers_both_minecart_analog_inputs() {
        let pos = BlockPos { x: 0, y: 64, z: 0 };
        let center = Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        };
        let area = Aabb::of_block(pos);
        let mut entities = EntityStore::new(7, EntityIdCounter::new());

        // Nine full stacks of a twenty-seven slot chest minecart:
        // `Mth.lerpDiscrete(9 / 27, 0, 15)` is five.
        let mut chest = MinecartContainerData::new_chest();
        for slot in 0..9 {
            chest
                .set_item(slot, Some(stone_stack(64)))
                .expect("a chest minecart slot");
        }
        let uuid = entities.next_uuid();
        entities
            .restore_initialized(
                uuid,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
                center,
                EntityData::ChestMinecart(chest),
                |entity| entity.set_rot(0.0, 0.0),
            )
            .expect("a fresh UUID");
        let bodies = EntityBodyReadView::new(&entities, None, None);
        assert_eq!(
            detector_rail_minecarts(bodies, area),
            DetectorRailMinecarts {
                command_block_success_count: None,
                container_signal: Some(Some(5)),
            }
        );
        // The container minecarts are `AbstractMinecart`s the press check
        // counts too.
        assert_eq!(
            count_in_box(bodies, &[], area, BlockEntityQuery::Minecart),
            1
        );

        let mut command_block = crate::entity::CommandBlockMinecartData::new();
        command_block.set_success_count(3);
        let uuid = entities.next_uuid();
        entities
            .restore_initialized(
                uuid,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
                center,
                EntityData::CommandBlockMinecart(command_block),
                |entity| entity.set_rot(0.0, 0.0),
            )
            .expect("a fresh UUID");
        let bodies = EntityBodyReadView::new(&entities, None, None);
        assert_eq!(
            detector_rail_minecarts(bodies, area),
            DetectorRailMinecarts {
                command_block_success_count: Some(3),
                container_signal: Some(Some(5)),
            }
        );
        assert_eq!(
            count_in_box(bodies, &[], area, BlockEntityQuery::Minecart),
            2
        );

        // A box that reaches neither leaves both queries empty.
        let elsewhere = Aabb::of_block(BlockPos { x: 8, ..pos });
        assert_eq!(
            detector_rail_minecarts(bodies, elsewhere),
            DetectorRailMinecarts::default()
        );
    }

    #[test]
    fn entity_occupancy_uses_strict_boxes_and_query_specific_filters() {
        let pos = BlockPos { x: 0, y: 64, z: 0 };
        let center = Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        };
        let mut entities = EntityStore::new(7, EntityIdCounter::new());
        spawn_armor_stand(&mut entities, center).expect("a fresh UUID");
        spawn_marker(&mut entities, center, MarkerData::new()).expect("a fresh UUID");
        let minecart = spawn_minecart(&mut entities, center, 0.0).expect("a fresh UUID");
        let players = [
            player(100, false, false),
            player(101, false, true),
            player(102, true, false),
        ];
        let bodies = EntityBodyReadView::new(&entities, None, None);
        let area = Aabb::of_block(pos);

        assert_eq!(
            count_in_box(bodies, &players, area, BlockEntityQuery::BlockTriggerAll),
            4
        );
        assert_eq!(
            count_in_box(bodies, &players, area, BlockEntityQuery::BlockTriggerLiving),
            3
        );
        assert_eq!(
            count_in_box(bodies, &players, area, BlockEntityQuery::Minecart),
            1
        );

        // `AABB.intersects` is strict (`AABB.java:237-242`): a query ending
        // exactly at the minecart's minimum x does not reach it.
        let cart = entities.get(minecart).unwrap().bounding_box();
        let touching = Aabb {
            min_x: cart.min_x - 1.0,
            min_y: cart.min_y,
            min_z: cart.min_z,
            max_x: cart.min_x,
            max_y: cart.max_y,
            max_z: cart.max_z,
        };
        assert_eq!(
            count_in_box(bodies, &players, touching, BlockEntityQuery::Minecart),
            0
        );
    }
}
