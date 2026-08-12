mod advancements;
mod banner_blocks;
mod beacon_blocks;
mod bed_blocks;
mod bell_blocks;
mod block_entity_client_effects;
mod block_entity_item_displays;
mod block_events;
mod bounding_box_blocks;
mod chest_blocks;
mod chest_lids;
mod chunks;
mod client;
mod climate;
mod command_blocks;
mod commands;
mod comparator_blocks;
mod conduit_blocks;
mod copper_golem_statue_blocks;
mod counters;
mod creative_inventory;
mod decorated_pot_blocks;
mod diagnostics;
mod enchanting_table_books;
mod end_portal_blocks;
mod entities;
mod error;
mod exports;
mod fluid;
mod internal;
mod item_profiles;
mod jigsaw_blocks;
mod lectern_books;
mod level;
mod maps;
mod menu_providers;
mod minecart_command_blocks;
mod note_blocks;
mod painting_variants;
mod piston_blocks;
mod play_apply;
mod position;
mod publication_snapshot;
mod recipe_book;
mod recipes;
mod registries;
mod resolvable_profile;
mod shulker_box_blocks;
mod sign_blocks;
mod skull_blocks;
mod spawner_blocks;
mod store;
mod structure_blocks;
mod terrain;
mod terrain_state;
mod test_blocks;
mod test_instance_blocks;
mod toasts;
mod trial_spawner_blocks;
mod tutorial;
mod world_state;

pub use creative_inventory::*;
pub use exports::*;
pub(crate) use internal::*;
pub(crate) use publication_snapshot::PublicationShared;

#[cfg(test)]
pub(crate) struct RsaTestRng(rand::rngs::StdRng);

#[cfg(test)]
impl RsaTestRng {
    pub(crate) fn seeded(seed: u64) -> Self {
        use rand::SeedableRng;
        Self(rand::rngs::StdRng::seed_from_u64(seed))
    }
}

#[cfg(test)]
impl rsa::rand_core::RngCore for RsaTestRng {
    fn next_u32(&mut self) -> u32 {
        rand::Rng::next_u32(&mut self.0)
    }

    fn next_u64(&mut self) -> u64 {
        rand::Rng::next_u64(&mut self.0)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand::Rng::fill_bytes(&mut self.0, dest);
    }

    fn try_fill_bytes(
        &mut self,
        dest: &mut [u8],
    ) -> std::result::Result<(), rsa::rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

#[cfg(test)]
impl rsa::rand_core::CryptoRng for RsaTestRng {}
