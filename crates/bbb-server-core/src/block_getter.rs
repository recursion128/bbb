//! Read-only block and light access shared by gameplay and generation hosts.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::registry_data_types::VanillaFluidIdentity;

use crate::registry_tags::RegistryTagSnapshot;

/// `Holder.is(TagKey)` against the tags of one datapack generation, with the
/// `BlockState.is` and `FluidState.is` shorthands block code reads.
pub trait BlockTags {
    fn contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool;

    fn content_registry(&self) -> &ContentRegistry {
        ContentRegistry::vanilla()
    }

    fn block_in_tag(&self, block_state_id: i32, tag_id: &str) -> bool {
        if let Some(block) =
            vanilla_block_state_block_type_id(block_state_id).and_then(vanilla_block_type)
        {
            return self.contains("minecraft:block", tag_id, block.resource_id);
        }
        self.content_registry()
            .block_by_state_id(block_state_id)
            .is_some_and(|(block, _)| self.contains("minecraft:block", tag_id, block.resource_id()))
    }

    fn fluid_in_tag(&self, fluid: VanillaFluidIdentity, tag_id: &str) -> bool {
        self.contains("minecraft:fluid", tag_id, fluid.resource_id())
    }
}

pub trait BlockGetter {
    fn block_state(&self, pos: BlockPos) -> i32;

    fn raw_brightness(&self, _pos: BlockPos, _sky_darkening: i32) -> Option<i32> {
        None
    }

    fn tags(&self) -> &dyn BlockTags {
        RegistryTagSnapshot::builtin()
    }

    fn content_registry(&self) -> &ContentRegistry {
        self.tags().content_registry()
    }
}

/// [`BlockTags`] over a bare holder-membership lookup, for generation hosts
/// that answer tags without a snapshot.
pub struct TagLookup<F>(pub F);

impl<F: Fn(&str, &str, &str) -> bool> BlockTags for TagLookup<F> {
    fn contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        (self.0)(registry_id, tag_id, element_id)
    }
}
