//! `Level.registryAccess()` as generation reaches it.
//!
//! Vanilla hands the same provider to every block-entity seam a generating
//! level runs: `StructureTemplate.placeInWorld` loads a template's block
//! entities through `level.registryAccess()` (`StructureTemplate.java:305-306`),
//! its writer takes one through `TagValueOutput.createWithContext` (`:118`),
//! and `ProtoChunk.getBlockEntityNbtForSaving` is handed one outright
//! (`ProtoChunk.java:277-279`).
//!
//! The publication those three read belongs to the world, which sits above the
//! generation crates, so generation cannot reach up for it: the owner that
//! builds a level pushes it down, and a generator handed none reads the way it
//! always did.

use std::sync::Arc;

use bbb_registry_codec::PersistentItemStackHolderResolver;

/// One generating level's registry publication.
#[derive(Clone)]
pub struct GenerationItemPublication {
    resolver: Arc<dyn PersistentItemStackHolderResolver + Send + Sync>,
}

impl std::fmt::Debug for GenerationItemPublication {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GenerationItemPublication")
            .finish_non_exhaustive()
    }
}

impl GenerationItemPublication {
    #[must_use]
    pub fn new(resolver: Arc<dyn PersistentItemStackHolderResolver + Send + Sync>) -> Self {
        Self { resolver }
    }

    pub(crate) fn resolver(&self) -> &dyn PersistentItemStackHolderResolver {
        self.resolver.as_ref()
    }
}
