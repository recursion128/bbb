//! Entity pick results and frame-content preservation.

use bbb_facts::{vanilla_entity_pick_result, VanillaEntityPickResult};
use bbb_protocol::item_types::vanilla_item_type;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::{Entity, EntityData};

#[derive(Debug, Clone, PartialEq)]
pub enum EntityPickResult {
    Existing(RuntimeItemStack),
    Fresh { item_id: i32 },
}

impl Entity {
    /// `Entity.getPickResult` (Entity.java:3786-3788), overridden by entity owners.
    pub fn pick_result(&self) -> Option<EntityPickResult> {
        let item_id = match vanilla_entity_pick_result(self.entity_type_id())? {
            VanillaEntityPickResult::None => return None,
            // `Mob.getPickResult` (Mob.java:1410-1412) creates the default egg.
            VanillaEntityPickResult::Mob { item_id } => item_id?,
            VanillaEntityPickResult::Item { item_id } => item_id,
            VanillaEntityPickResult::ItemFrame { empty_item_id } => {
                let frame = match self.data() {
                    EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => frame,
                    _ => return None,
                };
                // `ItemFrame.getPickResult` (ItemFrame.java:412-418) copies the entire stack.
                if let Some(stack) = frame.item().filter(|stack| !stack.is_empty()) {
                    return Some(EntityPickResult::Existing(stack.clone()));
                }
                empty_item_id
            }
        };
        vanilla_item_type(item_id)?;
        Some(EntityPickResult::Fresh { item_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id;
    use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};

    use uuid::Uuid;

    use crate::entity::hanging::{ItemFrameData, PaintingData};
    use crate::entity::item::{plain_stack, ItemEntityData};
    use crate::entity::{BoatData, ChestBoatData, GeneralLivingData, SpawnerMinecartData};

    fn entity(resource: &str, data: EntityData) -> Entity {
        Entity::new(
            1,
            Uuid::from_u128(1),
            vanilla_entity_type_id_for_resource_id(resource).unwrap(),
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            data,
        )
    }

    fn assert_plain_pick(entity: &Entity, item: &str) {
        let expected = plain_stack(item, 1).expect("expected item is registered");
        assert_eq!(
            entity.pick_result(),
            Some(EntityPickResult::Fresh {
                item_id: expected.item_id().unwrap(),
            })
        );
    }

    #[test]
    fn pick_mob_returns_default_egg_without_custom_name_or_entity_data() {
        for (resource, expected) in [
            ("minecraft:cow", "minecraft:cow_spawn_egg"),
            ("minecraft:ender_dragon", "minecraft:ender_dragon_spawn_egg"),
            ("minecraft:camel_husk", "minecraft:camel_husk_spawn_egg"),
        ] {
            let type_id = vanilla_entity_type_id_for_resource_id(resource).unwrap();
            let mut mob = entity(
                resource,
                EntityData::GeneralLiving(GeneralLivingData::new(type_id).unwrap()),
            );
            mob.set_custom_name(Some(RichTextComponent::plain("Named mob")));
            assert_plain_pick(&mob, expected);
        }
    }

    #[test]
    fn pick_mobs_without_eggs_and_avatar_have_no_result() {
        for resource in [
            "minecraft:giant",
            "minecraft:illusioner",
            "minecraft:mannequin",
        ] {
            let type_id = vanilla_entity_type_id_for_resource_id(resource).unwrap();
            let mob = entity(
                resource,
                EntityData::GeneralLiving(GeneralLivingData::new(type_id).unwrap()),
            );
            assert_eq!(mob.pick_result(), None, "{resource}");
        }
    }

    #[test]
    fn pick_default_entity_does_not_return_its_carried_item() {
        let dropped = entity(
            "minecraft:item",
            EntityData::Item(ItemEntityData::new(
                plain_stack("minecraft:diamond", 3).unwrap(),
                0.0,
            )),
        );
        assert_eq!(dropped.pick_result(), None);
    }

    fn frame(glow: bool, item: Option<RuntimeItemStack>) -> Entity {
        let data = ItemFrameData::new(BlockPos { x: 0, y: 0, z: 0 }, Direction::North)
            .with_saved_fields(item, 3, 0.5, true, true);
        if glow {
            entity("minecraft:glow_item_frame", EntityData::GlowItemFrame(data))
        } else {
            entity("minecraft:item_frame", EntityData::ItemFrame(data))
        }
    }

    #[test]
    fn pick_empty_frames_use_their_own_item_for_absent_and_empty_stacks() {
        for glow in [false, true] {
            let item = if glow {
                "minecraft:glow_item_frame"
            } else {
                "minecraft:item_frame"
            };
            assert_plain_pick(&frame(glow, None), item);
            let empty = bbb_server_types::item_stack::RuntimeItemStack::try_read(
                &NbtSummaryValue::Compound(Vec::new()),
                &bbb_registry_codec::UnavailableHolderResolver,
            )
            .unwrap();
            assert_plain_pick(&frame(glow, Some(empty)), item);
        }
    }

    #[test]
    fn pick_populated_frames_preserve_full_stack_and_do_not_remove_it() {
        let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String("minecraft:filled_map".into()),
                },
                NbtSummaryEntry {
                    name: "count".into(),
                    value: NbtSummaryValue::Int(3),
                },
                NbtSummaryEntry {
                    name: "components".into(),
                    value: NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "minecraft:map_id".into(),
                            value: NbtSummaryValue::Int(42),
                        },
                        NbtSummaryEntry {
                            name: "minecraft:custom_data".into(),
                            value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                                name: "preserved".into(),
                                value: NbtSummaryValue::String("payload".into()),
                            }]),
                        },
                    ]),
                },
            ]),
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .unwrap();
        for glow in [false, true] {
            let entity = frame(glow, Some(stack.clone()));
            assert_eq!(
                entity.pick_result(),
                Some(EntityPickResult::Existing(stack.clone()))
            );
            let (EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame)) = entity.data()
            else {
                unreachable!()
            };
            assert_eq!(frame.item(), Some(&stack));
        }
    }

    #[test]
    fn pick_vehicles_use_registered_items_and_spawner_uses_plain_minecart() {
        for (entity, item) in [
            (
                entity("minecraft:oak_boat", EntityData::Boat(BoatData::new())),
                "minecraft:oak_boat",
            ),
            (
                entity(
                    "minecraft:bamboo_chest_raft",
                    EntityData::ChestBoat(
                        ChestBoatData::for_entity_type(
                            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID,
                        )
                        .unwrap(),
                    ),
                ),
                "minecraft:bamboo_chest_raft",
            ),
            (
                entity(
                    "minecraft:spawner_minecart",
                    EntityData::SpawnerMinecart(SpawnerMinecartData::new()),
                ),
                "minecraft:minecart",
            ),
        ] {
            assert_plain_pick(&entity, item);
        }
    }

    #[test]
    fn pick_painting_does_not_copy_variant_or_custom_name() {
        let mut painting = entity(
            "minecraft:painting",
            EntityData::Painting(PaintingData::new(
                BlockPos { x: 0, y: 0, z: 0 },
                Direction::North,
                crate::entity::hanging::PaintingVariant::vanilla("minecraft:kebab"),
            )),
        );
        painting.set_custom_name(Some(RichTextComponent::plain("Named painting")));
        assert_plain_pick(&painting, "minecraft:painting");
    }
}
