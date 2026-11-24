//! The container menu session the `server-container-menu-surface` differential
//! is driven by.
//!
//! Each session opens a staged chest from its block, quick-moves the stack its
//! first container slot holds into the player half of the same menu, and closes
//! it again, publishing the whole menu after each of the three. What makes the
//! result comparable rather than a race is the freeze the harness took before
//! the session connected plus a round that is one tick. A frozen server answers
//! a packet where it reads it (`processPacketsAndTick`,
//! MinecraftServer), but everything on the tick loop -- the block
//! event a lid crossing queues among it -- waits for a tick this session steps
//! itself, and a chat command is not read there at all, so each round closes on
//! [`Session::tick_barrier`] rather than on its fence's own answer.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use bbb_client_world::{BlockEventRecord, BlockPos, ContainerState, InventoryState};
use bbb_protocol::{
    item_types::vanilla_item_type,
    menu_types::vanilla_menu_type,
    packets::{
        self, BlockHitResult, ContainerClick, ContainerCloseRequest, ContainerInput, Direction,
        HashedStack, InteractionHand, ItemStackSummary, UseItemOn,
    },
};

use crate::types::{
    CommandMessageObservation, ContainerMenuReport, ContainerMenuSessionContract,
    ContainerMenuSessionReport, ContainerMenuStepReport, CONTAINER_MENU_QUICK_MOVED_SLOT,
    CONTAINER_MENU_SESSIONS, CONTAINER_MENU_STEPPED_TICKS, CONTAINER_MENU_STEP_LABELS,
    OPERATOR_PROBE_FENCE_COMMAND, OPERATOR_PROBE_FENCE_TRANSLATION_KEY, OPERATOR_PROBE_USERNAME,
};
use bbb_client_net::{ConnectionState, PlayPacketBundleAssembler};

use super::{
    operator::{observe_command_message, ObservedCommandMessage},
    send_encoded, tick_barrier_request, ProbeContext, ProbeProgress, TICK_BARRIER_COMMAND,
};

/// `/tick step` with no argument steps a single tick (`TickCommand.register`,
/// TickCommand.java:34-35), and only while the game is frozen
/// (`TickCommand.step`, `:131-141`).
const STEP_COMMAND: &str = "tick step";
const STEP_SUCCESS_TRANSLATION_KEY: &str = "commands.tick.step.success";

/// How the differential spells an empty slot and an empty cursor.
const EMPTY_CELL: &str = "<empty>";

pub(super) async fn run(
    probe: &mut ProbeContext,
    packets_seen: &mut usize,
    progress: &ProbeProgress,
) -> Result<ContainerMenuReport> {
    let mut session = Session {
        bundle: PlayPacketBundleAssembler::default(),
        seen_message: probe
            .world
            .client_chat()
            .messages
            .iter()
            .map(|message| message.display_sequence)
            .max()
            .unwrap_or(0),
        block_events_read: probe.world.block_events().len(),
        published_lids: BTreeMap::new(),
        sequence: 0,
    };
    // One fenced round before the first open, so the login's own traffic --
    // the teleport this client still has to accept among it -- is behind the
    // session rather than racing its first packet.
    progress.note("the container menu session's login to settle");
    session.fence(probe, packets_seen, None).await?;
    let mut sessions = Vec::with_capacity(CONTAINER_MENU_SESSIONS.len());
    for contract in &CONTAINER_MENU_SESSIONS {
        sessions.push(
            session
                .replay(probe, packets_seen, progress, contract)
                .await?,
        );
    }
    Ok(ContainerMenuReport {
        stepped_ticks: CONTAINER_MENU_STEPPED_TICKS,
        sessions,
    })
}

/// One connection's cursor into everything the steps are read out of.
struct Session {
    bundle: PlayPacketBundleAssembler,
    /// The chat feed position the fence is read from.
    seen_message: u64,
    /// How much of the world's block event list earlier steps already took.
    block_events_read: usize,
    /// The last `(b0, b1)` published for each block, which is what turns the
    /// republished counts into the crossings the report names.
    published_lids: BTreeMap<BlockPos, (u8, u8)>,
    /// The block-change sequence the next open acknowledges with, counted up
    /// the way a client counts its own.
    sequence: i32,
}

impl Session {
    async fn replay(
        &mut self,
        probe: &mut ProbeContext,
        packets_seen: &mut usize,
        progress: &ProbeProgress,
        contract: &ContainerMenuSessionContract,
    ) -> Result<ContainerMenuSessionReport> {
        let name = contract.name;
        let [x, y, z] = contract.block;
        self.sequence += 1;
        send_encoded(
            &mut probe.conn,
            packets::encode_play_use_item_on(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos: BlockPos { x, y, z },
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 1.0,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: self.sequence,
            }),
        )
        .await?;
        self.settle(
            probe,
            packets_seen,
            progress,
            name,
            CONTAINER_MENU_STEP_LABELS[0],
        )
        .await?;
        let (menu_type, title_key, container_id, state_id) = self.opened_menu(probe, name)?;
        let mut steps = Vec::with_capacity(CONTAINER_MENU_STEP_LABELS.len());
        steps.push(self.open_step(probe, CONTAINER_MENU_STEP_LABELS[0], container_id, name)?);

        send_encoded(
            &mut probe.conn,
            packets::encode_play_container_click(ContainerClick {
                container_id,
                state_id,
                slot_num: CONTAINER_MENU_QUICK_MOVED_SLOT,
                button_num: 0,
                input: ContainerInput::QuickMove,
                // Nothing is predicted locally: what the quick move did is what
                // the server broadcast back, which is the half being compared.
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::empty(),
            })?,
        )
        .await?;
        self.settle(
            probe,
            packets_seen,
            progress,
            name,
            CONTAINER_MENU_STEP_LABELS[1],
        )
        .await?;
        steps.push(self.open_step(probe, CONTAINER_MENU_STEP_LABELS[1], container_id, name)?);

        send_encoded(
            &mut probe.conn,
            packets::encode_play_container_close(ContainerCloseRequest { container_id }),
        )
        .await?;
        probe.world.close_local_container(container_id);
        self.settle(
            probe,
            packets_seen,
            progress,
            name,
            CONTAINER_MENU_STEP_LABELS[2],
        )
        .await?;
        steps.push(self.inventory_step(probe, CONTAINER_MENU_STEP_LABELS[2])?);

        Ok(ContainerMenuSessionReport {
            name: name.to_owned(),
            block: contract.block,
            menu_type,
            title_key,
            container_id,
            steps,
        })
    }

    /// What the open said about the menu itself, before any step reads it.
    fn opened_menu(&self, probe: &ProbeContext, name: &str) -> Result<(String, String, i32, i32)> {
        let container = probe
            .world
            .inventory()
            .open_container
            .as_ref()
            .with_context(|| format!("the {name} session opened no menu from its block"))?;
        let menu_type_id = container
            .menu_type_id
            .with_context(|| format!("the {name} menu was opened without a menu type"))?;
        let menu_type = vanilla_menu_type(menu_type_id)
            .with_context(|| format!("the {name} menu type {menu_type_id} is not a vanilla one"))?
            .resource_id
            .to_owned();
        let title = container
            .title
            .as_ref()
            .with_context(|| format!("the {name} menu was opened without a title"))?;
        let title_key = observe_command_message(title.component())
            .with_context(|| format!("the {name} menu title is not a valid component"))?
            .observation
            .translate
            .with_context(|| format!("the {name} menu title names no translation key"))?;
        Ok((
            menu_type,
            title_key,
            container.container_id,
            container.state_id,
        ))
    }

    fn open_step(
        &mut self,
        probe: &ProbeContext,
        label: &str,
        container_id: i32,
        name: &str,
    ) -> Result<ContainerMenuStepReport> {
        let inventory = probe.world.inventory();
        let container = inventory
            .open_container
            .as_ref()
            .filter(|container| container.container_id == container_id)
            .with_context(|| {
                format!("the {name} session holds no container {container_id} at the {label} step")
            })?;
        step_report(
            label,
            inventory,
            container,
            &inventory.cursor_item,
            self.lid_events(probe),
        )
    }

    /// The player's own menu, which is what a closed session falls back to
    /// (`doCloseContainer`).
    fn inventory_step(
        &mut self,
        probe: &ProbeContext,
        label: &str,
    ) -> Result<ContainerMenuStepReport> {
        let inventory = probe.world.inventory();
        step_report(
            label,
            inventory,
            &inventory.inventory_menu,
            &inventory.cursor_item,
            self.lid_events(probe),
        )
    }

    /// Steps the ticks one packet's answer needs, then one more fenced round so
    /// the last of them has been broadcast.
    ///
    /// A round's barrier is answered by the packet phase that opens the tick
    /// the round's step armed, so the round that reads a tick is always the one
    /// after the round that asked for it.
    async fn settle(
        &mut self,
        probe: &mut ProbeContext,
        packets_seen: &mut usize,
        progress: &ProbeProgress,
        name: &str,
        label: &str,
    ) -> Result<()> {
        for step in 0..CONTAINER_MENU_STEPPED_TICKS {
            progress.note(format!("the {name} session's {label} stepped tick {step}"));
            let window = self.fence(probe, packets_seen, Some(STEP_COMMAND)).await?;
            if !window
                .iter()
                .any(|message| message.mentions_translation_key(STEP_SUCCESS_TRANSLATION_KEY))
            {
                bail!(
                    "/{STEP_COMMAND} {step} of the {name} session's {label} step was not answered \
                     with {STEP_SUCCESS_TRANSLATION_KEY}: check that the server is frozen and that \
                     its ops.json lists {OPERATOR_PROBE_USERNAME} at level 4 ({:?})",
                    observations(&window)
                );
            }
        }
        self.fence(probe, packets_seen, None).await?;
        Ok(())
    }

    /// Runs `command`, then the fence, and pumps until the fence is answered.
    ///
    /// The fence is [`OPERATOR_PROBE_FENCE_COMMAND`], whose refusal is what
    /// says the command ahead of it has finished being answered, and
    /// [`Session::tick_barrier`] is what says the packet phase has run and a
    /// tick with it.
    async fn fence(
        &mut self,
        probe: &mut ProbeContext,
        packets_seen: &mut usize,
        command: Option<&str>,
    ) -> Result<Vec<ObservedCommandMessage>> {
        let named = command.unwrap_or(OPERATOR_PROBE_FENCE_COMMAND);
        if let Some(command) = command {
            send_command(probe, command).await?;
        }
        send_command(probe, OPERATOR_PROBE_FENCE_COMMAND).await?;

        let mut messages = Vec::new();
        loop {
            let arrived = probe
                .world
                .client_chat()
                .messages
                .iter()
                .filter(|message| message.display_sequence > self.seen_message)
                .map(|message| (message.display_sequence, message.content_component.clone()))
                .collect::<Vec<_>>();
            for (sequence, component) in arrived {
                self.seen_message = self.seen_message.max(sequence);
                let component = component.with_context(|| {
                    format!("/{named} was answered by a message with no component")
                })?;
                let observed = observe_command_message(&component)?;
                if observed.mentions_translation_key(OPERATOR_PROBE_FENCE_TRANSLATION_KEY) {
                    self.tick_barrier(probe, packets_seen, named).await?;
                    return Ok(messages);
                }
                messages.push(observed);
            }
            self.pump_once(probe, packets_seen, named).await?;
        }
    }

    /// Pumps until the server has begun its next tick, so that a round really
    /// is behind the packet the leg sent ahead of it.
    ///
    /// [`tick_barrier_request`] names why the chat fence alone is behind
    /// nothing. Without this the open the round fences can still be sitting in
    /// the packet processor when the round reads the menu it was supposed to
    /// have produced, and a round that reads no menu is indistinguishable from
    /// a server that opened none.
    async fn tick_barrier(
        &mut self,
        probe: &mut ProbeContext,
        packets_seen: &mut usize,
        named: &str,
    ) -> Result<()> {
        let request = probe
            .world
            .next_command_suggestion_request(TICK_BARRIER_COMMAND);
        let request_id = request.id;
        send_encoded(&mut probe.conn, tick_barrier_request(request)?).await?;
        while !probe
            .world
            .last_command_suggestions()
            .is_some_and(|answer| answer.id == request_id)
        {
            self.pump_once(probe, packets_seen, named).await?;
        }
        Ok(())
    }

    async fn pump_once(
        &mut self,
        probe: &mut ProbeContext,
        packets_seen: &mut usize,
        named: &str,
    ) -> Result<()> {
        let (packet_id, payload) = probe.session.read_packet_or_send_play_tick().await?;
        *packets_seen = packets_seen.saturating_add(1);
        if probe.state != ConnectionState::Play {
            bail!("the container menu session left play while running /{named}");
        }
        let packet = probe.session.decode_play_packet(packet_id, &payload)?;
        if let Some(batch) = self.bundle.push(packet)? {
            for packet in batch {
                probe.apply_observed_play_packet(packet).await?;
            }
        }
        Ok(())
    }

    /// The lid crossings the block events that arrived since the last step
    /// carry.
    fn lid_events(&mut self, probe: &ProbeContext) -> Vec<String> {
        let events = probe.world.block_events();
        let crossings = lid_crossings(&events[self.block_events_read..], &mut self.published_lids);
        self.block_events_read = events.len();
        crossings
    }
}

/// Keeps only the events that moved a block's event pair.
///
/// `ContainerOpenersCounter.recheckOpeners`
/// calls `openerCountChanged` whether or not the count moved and reschedules
/// itself every five ticks through `ChestBlock.tick`,
/// so an open chest republishes its count for as long as it is open and only
/// the first of those is the crossing the differential names.
fn lid_crossings(
    events: &[BlockEventRecord],
    published: &mut BTreeMap<BlockPos, (u8, u8)>,
) -> Vec<String> {
    let mut crossings = Vec::new();
    for event in events {
        if published.insert(event.pos, (event.b0, event.b1)) == Some((event.b0, event.b1)) {
            continue;
        }
        crossings.push(format!(
            "{},{},{}={}/{}",
            event.pos.x, event.pos.y, event.pos.z, event.b0, event.b1
        ));
    }
    crossings
}

fn step_report(
    label: &str,
    inventory: &InventoryState,
    container: &ContainerState,
    carried: &ItemStackSummary,
    lid_events: Vec<String>,
) -> Result<ContainerMenuStepReport> {
    Ok(ContainerMenuStepReport {
        label: label.to_owned(),
        state_id: container.state_id,
        container_id: container.container_id,
        slots: cells(inventory, container)?,
        carried: stack(carried)?,
        lid_events,
    })
}

/// Every slot the menu holds, in slot order, however many the server sent.
fn cells(inventory: &InventoryState, container: &ContainerState) -> Result<Vec<String>> {
    let size = inventory
        .container_slots(container)
        .map(|slot| i32::from(slot.slot) + 1)
        .max()
        .unwrap_or(0);
    (0..size)
        .map(|slot| {
            let item = inventory
                .container_slot(container, slot as i16)
                .map(|candidate| candidate.item);
            let cell = match item {
                Some(item) => stack(item)?,
                None => EMPTY_CELL.to_owned(),
            };
            Ok(format!("{slot}={cell}"))
        })
        .collect()
}

fn stack(item: &ItemStackSummary) -> Result<String> {
    let Some(item_id) = item.item_id.filter(|_| item.count > 0) else {
        return Ok(EMPTY_CELL.to_owned());
    };
    let entry = vanilla_item_type(item_id)
        .with_context(|| format!("a menu slot holds item {item_id}, which no vanilla item has"))?;
    Ok(format!("{} x{}", entry.resource_id, item.count))
}

fn observations(window: &[ObservedCommandMessage]) -> Vec<&CommandMessageObservation> {
    window.iter().map(|message| &message.observation).collect()
}

async fn send_command(probe: &mut ProbeContext, command: &str) -> Result<()> {
    send_encoded(
        &mut probe.conn,
        packets::encode_play_chat_command(&packets::ChatCommand {
            command: command.to_owned(),
        }),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_world::WorldStore;

    fn stone(count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count,
            ..ItemStackSummary::empty()
        }
    }

    fn event(pos: BlockPos, b1: u8) -> BlockEventRecord {
        BlockEventRecord {
            pos,
            b0: 1,
            b1,
            block_id: 0,
        }
    }

    #[test]
    fn container_menu_cells_are_slot_ordered_and_name_their_gaps() {
        let mut world = WorldStore::new();
        for (slot, item) in [(2, stone(16)), (0, ItemStackSummary::empty())] {
            world.apply_container_set_slot(packets::ContainerSetSlot {
                container_id: 0,
                state_id: 0,
                slot,
                item,
            });
        }
        let inventory = world.inventory();
        assert_eq!(
            cells(inventory, &inventory.inventory_menu).unwrap(),
            ["0=<empty>", "1=<empty>", "2=minecraft:stone x16"]
        );
        assert_eq!(
            cells(&InventoryState::default(), &ContainerState::default()).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn container_menu_counts_a_countless_stack_as_an_empty_slot() {
        assert_eq!(stack(&stone(0)).unwrap(), EMPTY_CELL);
        assert_eq!(stack(&stone(1)).unwrap(), "minecraft:stone x1");
    }

    #[test]
    fn container_menu_lid_events_are_crossings_rather_than_republished_counts() {
        let first = BlockPos { x: 2, y: -60, z: 0 };
        let second = BlockPos {
            x: -2,
            y: -60,
            z: 0,
        };
        let mut published = BTreeMap::new();
        assert_eq!(
            lid_crossings(&[event(first, 1), event(second, 1)], &mut published),
            ["2,-60,0=1/1", "-2,-60,0=1/1"]
        );
        assert!(lid_crossings(&[event(first, 1), event(second, 1)], &mut published).is_empty());
        assert_eq!(
            lid_crossings(&[event(first, 0), event(first, 0)], &mut published),
            ["2,-60,0=1/0"]
        );
    }

    #[test]
    fn container_menu_contract_drives_two_sessions_and_three_steps() {
        assert_eq!(CONTAINER_MENU_SESSIONS.len(), 2);
        assert_eq!(CONTAINER_MENU_STEP_LABELS, ["opened", "clicked", "closed"]);
    }
}
