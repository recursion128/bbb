//! A session's using-item state (`LivingEntity.java:3353-3562`) driven by real
//! `UseItem`, `PlayerAction` and `SetCarriedItem` packets, as the using player
//! and a second, tracking player see it.

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID;
use bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_FLAGS_DATA_ID;
use bbb_protocol::potion_types::vanilla_potion_for_resource_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

use super::*;

mod brush;
mod consume;
mod equipment;
mod held;
mod projectile_block_hit;
mod ranged;
mod shield;
mod spear;
mod static_tag_consume_tests;

pub(super) const ALICE: u64 = 1;
pub(super) const BOB: u64 = 2;
/// `InventoryMenu` slots of the first hotbar slot and of the offhand.
pub(super) const HOTBAR_MENU_SLOT: i16 = 36;
pub(super) const OFFHAND_MENU_SLOT: i16 = 45;
/// `Inventory` indices of the same two slots.
pub(super) const HOTBAR_SLOT: usize = 0;
pub(super) const OFFHAND_SLOT: usize = 40;

pub(super) fn stack(resource_id: &str, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id(resource_id)),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

/// `PotionContents.createItemStack(Items.POTION, potion)`.
pub(super) fn potion(resource_id: &str) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id("minecraft:potion")),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
            potion_contents: bbb_protocol::packets::PotionContentsSummary {
                potion_id: Some(
                    vanilla_potion_for_resource_id(resource_id)
                        .expect("a built-in potion")
                        .id,
                ),
                ..Default::default()
            },
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

pub(super) struct Fixture {
    pub(super) sender: WorldCommandSender,
    pub(super) receiver: WorldCommandReceiver,
    pub(super) world: ServerWorld,
    pub(super) alice: i32,
}

impl Fixture {
    /// Alice and Bob in `game_type`, each tracking the other, with Bob an
    /// operator who can `/damage` Alice.
    pub(super) fn new(game_type: GameType) -> Self {
        Self::with_config(game_type, datapack_config())
    }

    pub(super) fn with_config(game_type: GameType, config: ServerWorldConfig) -> Self {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(ServerWorldConfig {
            game_type,
            force_game_mode: true,
            // Bob's operator entry would otherwise put Alice's block uses
            // under spawn protection.
            spawn_protection_radius: 0,
            admin_lists: operator_config(&[("bob", 2)]).admin_lists,
            ..config
        });
        join_loaded(&sender, &receiver, &mut world, ALICE, "alice", 1);
        join_loaded(&sender, &receiver, &mut world, BOB, "bob", 2);
        world.reset_pending_chunks_for_test(ConnectionId(ALICE), &[]);
        world.reset_pending_chunks_for_test(ConnectionId(BOB), &[]);
        let mut fixture = Self {
            alice: entity_id_of(&world, ALICE),
            sender,
            receiver,
            world,
        };
        for connection_id in [ALICE, BOB] {
            fixture.play(
                connection_id,
                PlayServerbound::MovePlayer(MovePlayer {
                    position: None,
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                }),
            );
        }
        assert!(fixture
            .world
            .player_tracking_viewers_for_test(Java26BuiltinDimension::Overworld, fixture.alice)
            .contains(&ConnectionId(BOB)));
        fixture
    }

    pub(super) fn play(&mut self, connection_id: u64, packet: PlayServerbound) -> TickResult {
        let tick = play(
            &self.sender,
            &self.receiver,
            &mut self.world,
            connection_id,
            packet,
        );
        self.answer_keep_alives(tick)
    }

    pub(super) fn idle(&mut self) -> TickResult {
        let tick = self.world.tick(self.receiver.drain());
        self.answer_keep_alives(tick)
    }

    /// Both clients answer every keep-alive in the next tick, which is what
    /// keeps a minute-long use from timing them out.
    fn answer_keep_alives(&self, tick: TickResult) -> TickResult {
        for event in &tick.events {
            if let WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::KeepAlive { id },
            } = event
            {
                enqueue(
                    &self.sender,
                    WorldCommand::Play {
                        connection_id: *connection_id,
                        packet: PlayServerbound::KeepAlive { id: *id },
                    },
                )
                .unwrap();
            }
        }
        tick
    }

    pub(super) fn hold(&mut self, menu_slot: i16, item: ItemStackSummary) {
        self.world
            .stock_menu_slot_for_test(ConnectionId(ALICE), menu_slot, item);
    }

    /// Bob runs `command` from chat.
    pub(super) fn command(&mut self, command: &str) -> TickResult {
        self.play(
            BOB,
            PlayServerbound::ChatCommand {
                command: command.to_string(),
            },
        )
    }

    pub(super) fn use_item(&mut self, hand: InteractionHand) -> TickResult {
        self.play(
            ALICE,
            PlayServerbound::UseItem(UseItem {
                hand,
                sequence: 1,
                y_rot: 0.0,
                x_rot: 0.0,
            }),
        )
    }

    pub(super) fn action(&mut self, action: PlayerActionKind) -> TickResult {
        self.play(
            ALICE,
            PlayServerbound::PlayerAction(PlayerAction {
                action,
                pos: BlockPos { x: 0, y: 0, z: 0 },
                direction: Direction::Down,
                sequence: 0,
            }),
        )
    }

    pub(super) fn use_state(&self) -> (bool, InteractionHand, i32, ItemStackSummary) {
        self.world.player_use_state_for_test(ConnectionId(ALICE))
    }

    pub(super) fn slot(&self, index: usize) -> ItemStackSummary {
        self.world
            .player_inventory_slot_for_test(ConnectionId(ALICE), index)
    }

    pub(super) fn item_used(&self, resource_id: &str) -> i32 {
        self.world.player_stat_for_test(
            ConnectionId(ALICE),
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id(resource_id)),
        )
    }
}

pub(super) fn packets_to(
    tick: &TickResult,
    connection_id: u64,
) -> impl Iterator<Item = &ClientboundPlayPacket> {
    tick.events
        .iter()
        .filter_map(move |event| event.packet_for(ConnectionId(connection_id)))
}

/// Every `DATA_LIVING_ENTITY_FLAGS` value `viewer` was sent for `entity_id`.
pub(super) fn living_flags(tick: &TickResult, viewer: u64, entity_id: i32) -> Vec<i8> {
    packets_to(tick, viewer)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::SetEntityData(data) if data.id == entity_id => Some(data),
            _ => None,
        })
        .flat_map(|data| data.values.iter())
        .filter(|value| value.data_id == VANILLA_LIVING_ENTITY_FLAGS_DATA_ID)
        .map(|value| match value.value {
            EntityDataValueKind::Byte(flags) => flags,
            ref other => panic!("living flags are a byte: {other:?}"),
        })
        .collect()
}

pub(super) fn entity_events(tick: &TickResult, viewer: u64, entity_id: i32) -> Vec<i8> {
    packets_to(tick, viewer)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::EntityEvent(event) if event.entity_id == entity_id => {
                Some(event.event_id)
            }
            _ => None,
        })
        .collect()
}

/// The resource ids of the positioned sounds `viewer` was sent.
/// The entity-bound sounds a viewer heard, with the entity each follows.
pub(super) fn entity_sounds(tick: &TickResult, viewer: u64) -> Vec<(&'static str, i32)> {
    packets_to(tick, viewer)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::SoundEntity(sound) => match sound.sound {
                SoundEventHolder::Reference { registry_id } => Some((
                    vanilla_sound_event_resource_id(registry_id)?,
                    sound.entity_id,
                )),
                SoundEventHolder::Direct { .. } => None,
            },
            _ => None,
        })
        .collect()
}

pub(super) fn sounds(tick: &TickResult, viewer: u64) -> Vec<&'static str> {
    packets_to(tick, viewer)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::Sound(sound) => match sound.sound {
                SoundEventHolder::Reference { registry_id } => {
                    vanilla_sound_event_resource_id(registry_id)
                }
                SoundEventHolder::Direct { .. } => None,
            },
            _ => None,
        })
        .collect()
}
