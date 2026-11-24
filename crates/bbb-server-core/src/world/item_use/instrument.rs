//! `InstrumentItem`'s use and the `minecraft:instrument` holder it plays.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::connection::ClientboundPlayPacket;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::packets::{SoundEntityEvent, SoundEvent, SoundEventHolder, SoundSource};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_registry_codec::RegistryElementValueRef;
use bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot;
use bbb_server_types::stats::StatKey;

use super::{ItemUseContext, ItemUseResult};
use crate::world::player_packets::sound_broadcast_range;
use crate::world::using_item::{instrument_component, InstrumentRef};
use crate::world::{send, ServerWorld, WorldEvent};

const INSTRUMENT_REGISTRY: &str = "minecraft:instrument";

/// `Instrument` (`Instrument.java:16`) without its description.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::world) struct InstrumentFacts {
    sound: SoundEventHolder,
    use_duration: f32,
    range: f32,
}

impl InstrumentFacts {
    /// `Mth.floor(instrument.useDuration() * 20.0F)` (`InstrumentItem.java:37`, `:48`).
    pub(in crate::world) fn use_ticks(&self) -> i32 {
        (self.use_duration * 20.0).floor() as i32
    }
}

/// `InstrumentItem.use` (`InstrumentItem.java:30-43`).
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    let Ok(Some(instrument)) = world.instrument_facts(&context.item) else {
        return ItemUseResult::Fail;
    };
    world.start_using_item(context.connection_id, context.hand);
    world.play_instrument(context, &instrument, events);
    let player = world
        .levels
        .player_mut(context.connection_id)
        .expect("connection presence was checked");
    if let Ok(packet) = player
        .cold
        .cooldowns
        .add_cooldown(&context.item, instrument.use_ticks())
    {
        events.push(send(
            context.connection_id,
            ClientboundPlayPacket::Cooldown(packet),
        ));
    }
    if let Some(item_id) = context.item.item_id {
        player.award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    ItemUseResult::Consume
}

impl ServerWorld {
    /// `stack.get(DataComponents.INSTRUMENT)` resolved to its `Instrument`.
    pub(in crate::world) fn instrument_facts(
        &self,
        stack: &bbb_protocol::packets::ItemStackSummary,
    ) -> Result<Option<InstrumentFacts>, String> {
        let Some(reference) = instrument_component(stack)? else {
            return Ok(None);
        };
        let id = match reference {
            InstrumentRef::Direct {
                sound,
                use_duration,
                range,
            } => {
                return Ok(Some(InstrumentFacts {
                    sound,
                    use_duration,
                    range,
                }))
            }
            InstrumentRef::Named(id) => id,
            InstrumentRef::Holder(index) => self
                .registries
                .registry_element_id(INSTRUMENT_REGISTRY, index)
                .ok_or_else(|| format!("instrument holder {index} is not registered"))?,
        };
        let facts = match self.datapacks() {
            Some(startup) => {
                let document = startup
                    .registries
                    .element(INSTRUMENT_REGISTRY, id)
                    .and_then(|element| element.codec_value.as_ref())
                    .ok_or_else(|| format!("instrument {id} has no loaded value"))?;
                parse_instrument(RegistryElementValueRef::Nbt(&document.network_nbt))
            }
            None => builtin_instruments().get(id).cloned(),
        };
        facts
            .map(Some)
            .ok_or_else(|| format!("instrument {id} does not decode"))
    }

    /// `InstrumentItem.play` (`InstrumentItem.java:61-66`).
    fn play_instrument(
        &mut self,
        context: &ItemUseContext,
        instrument: &InstrumentFacts,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self.levels.player_at(context.connection_id);
        let (entity_id, position, dimension) =
            (player.entity_id(), player.position(), player.dimension);
        let volume = instrument.range / 16.0;
        // `ServerLevel.playSeededSound(except, sourceEntity, ..)`
        // (`ServerLevel.java:1048-1068`).
        let sound = SoundEntityEvent {
            sound: instrument.sound.clone(),
            source: SoundSource::Records,
            entity_id,
            volume,
            pitch: 1.0,
            seed: self.random.next_long(),
        };
        let range = sound_broadcast_range(&SoundEvent {
            sound: sound.sound.clone(),
            source: sound.source,
            position,
            volume,
            pitch: sound.pitch,
            seed: sound.seed,
        });
        for (&listener_id, listener) in self.levels.player_entries() {
            if listener_id == context.connection_id || listener.dimension != dimension {
                continue;
            }
            let dx = position.x - listener.position().x;
            let dy = position.y - listener.position().y;
            let dz = position.z - listener.position().z;
            if dx * dx + dy * dy + dz * dz < range * range {
                events.push(send(
                    listener_id,
                    ClientboundPlayPacket::SoundEntity(sound.clone()),
                ));
            }
        }
        self.player_game_event_at(
            context.connection_id,
            VanillaGameEvent::INSTRUMENT_PLAY,
            position,
            None,
        );
    }
}

/// `Instrument.DIRECT_CODEC` (`Instrument.java:17-25`) down to the fields a
/// server reads.
fn parse_instrument(document: RegistryElementValueRef<'_>) -> Option<InstrumentFacts> {
    let use_duration = document.field("use_duration")?.codec_f32()?;
    let range = document.field("range")?.codec_f32()?;
    if !(use_duration > 0.0 && range > 0.0) {
        return None;
    }
    let sound = document.field("sound_event")?;
    let sound = match sound.java_string_ref() {
        Some(id) => id
            .with_utf8(bbb_protocol::sound_event_types::vanilla_sound_event_registry_id)
            .flatten()
            .map(|registry_id| SoundEventHolder::Reference { registry_id })?,
        None => SoundEventHolder::Direct {
            location: sound.field("sound_id")?.utf8_string()?,
            fixed_range: sound
                .field("range")
                .and_then(RegistryElementValueRef::codec_f32),
        },
    };
    Some(InstrumentFacts {
        sound,
        use_duration,
        range,
    })
}

/// The built-in pack's instruments, for a world started without a loaded
/// datapack generation.
fn builtin_instruments() -> &'static BTreeMap<String, InstrumentFacts> {
    static BUILTIN: OnceLock<BTreeMap<String, InstrumentFacts>> = OnceLock::new();
    BUILTIN.get_or_init(|| {
        let snapshot = VanillaDatapackSnapshot::embedded_shared()
            .expect("the embedded built-in datapack decodes");
        snapshot
            .paths()
            .filter_map(|path| {
                let name = path.strip_prefix("instrument/")?.strip_suffix(".json")?;
                let document = bbb_registry_codec::decode_direct_registry_json(
                    snapshot
                        .read_path(path)
                        .expect("an embedded instrument exists"),
                )
                .expect("the embedded instrument JSON decodes");
                let facts = parse_instrument(document.root())
                    .expect("the embedded instrument playback fields decode");
                Some((format!("minecraft:{name}"), facts))
            })
            .collect()
    })
}
