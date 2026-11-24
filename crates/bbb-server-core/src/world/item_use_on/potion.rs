//! `PotionItem.useOn` (`PotionItem.java:35-70`): a water bottle turns
//! `convertable_to_mud` into mud.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_MUD_ID;
use bbb_protocol::item_types::VANILLA_ITEM_GLASS_BOTTLE_ID;
use bbb_protocol::packets::{
    BlockPos, Direction, ItemStackSummary, LevelParticles, ParticlePayload, Vec3d,
};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_SPLASH_ID;

use super::ItemUseOnContext;
use crate::block_behavior::default_block_state_id;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::world::block_use::is_water_potion;
use crate::world::{ServerWorld, WorldEvent};

const CONVERTABLE_TO_MUD: &str = "minecraft:convertable_to_mud";

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let pos = context.pos();
    let state = world.block_state_or_air(context.dimension, pos);
    let convertable = vanilla_block_state_block_type_id(state).is_some_and(|block| {
        world
            .level_by_id(&context.dimension)
            .chunks
            .registry_tags()
            .is_some_and(|tags| tags.contains_block(CONVERTABLE_TO_MUD, block))
    });
    if context.update.hit.direction == Direction::Down
        || !convertable
        || !is_water_potion(context.stack)
    {
        return false;
    }
    world.play_block_sound(
        context.dimension,
        pos,
        "minecraft:entity.generic.splash",
        events,
    );
    let bottle = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_GLASS_BOTTLE_ID),
        count: 1,
        ..ItemStackSummary::empty()
    };
    world.create_filled_result(context.connection_id, context.hand(), bottle);
    world.send_mud_splash(&context.dimension, pos);
    world.play_block_sound(
        context.dimension,
        pos,
        "minecraft:item.bottle.empty",
        events,
    );
    world
        .level_mut_by_id(&context.dimension)
        .event_sink
        .game_event(VanillaGameEvent::FLUID_PLACE, pos, None);
    world.set_block(
        context.dimension,
        pos,
        default_block_state_id(VANILLA_BLOCK_MUD_ID),
        SET_BLOCK_UPDATE_ALL,
    );
    true
}

impl ServerWorld {
    /// The five `SPLASH` particles a water bottle sheds over `pos`
    /// (`PotionItem.java:48-60`, `DispenseItemBehavior.java:430-436`).
    pub(crate) fn send_mud_splash(&mut self, dimension: &crate::world::LevelId, pos: BlockPos) {
        let level = self.level_mut_by_id(dimension);
        for _ in 0..5 {
            let x = f64::from(pos.x) + level.random.next_double();
            let z = f64::from(pos.z) + level.random.next_double();
            level.entities.send_particles(LevelParticles {
                override_limiter: false,
                always_show: false,
                position: Vec3d {
                    x,
                    y: f64::from(pos.y + 1),
                    z,
                },
                offset: Vec3d::default(),
                max_speed: 1.0,
                count: 1,
                particle: ParticlePayload {
                    particle_type_id: VANILLA_PARTICLE_TYPE_SPLASH_ID,
                    raw_options: Vec::new(),
                },
            });
        }
    }
}
