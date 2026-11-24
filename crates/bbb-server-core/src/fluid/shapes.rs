use std::sync::{Arc, OnceLock};

use bbb_protocol::packets::BlockPos;
use bbb_server_types::geometry::Aabb;

use super::{checked_fluid_shape_with_registry, FluidFamily, FluidState};

#[derive(Debug, Clone)]
pub struct SharedFluidShapeCache(Arc<[OnceLock<Aabb>; 36]>);

impl Default for SharedFluidShapeCache {
    fn default() -> Self {
        Self(Arc::new(std::array::from_fn(|_| OnceLock::new())))
    }
}

impl SharedFluidShapeCache {
    pub fn shape(
        &self,
        state: FluidState,
        pos: BlockPos,
        block_state_at: impl FnMut(BlockPos) -> Option<i32>,
    ) -> Option<Option<&Aabb>> {
        self.shape_with_registry(
            state,
            pos,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
            block_state_at,
        )
    }

    pub fn shape_with_registry(
        &self,
        state: FluidState,
        pos: BlockPos,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
        block_state_at: impl FnMut(BlockPos) -> Option<i32>,
    ) -> Option<Option<&Aabb>> {
        if state.is_empty() {
            return Some(None);
        }
        let cell = &self.0[state_key(state)?];
        if let Some(shape) = cell.get() {
            return Some(Some(shape));
        }
        let shape =
            checked_fluid_shape_with_registry(state, pos, content_registry, block_state_at)??;
        Some(Some(cell.get_or_init(|| shape)))
    }
}

fn state_key(state: FluidState) -> Option<usize> {
    let (family, slot) = match state {
        FluidState::Empty => return None,
        FluidState::Source { family, falling } => (family, usize::from(falling)),
        FluidState::Flowing {
            family,
            amount,
            falling,
        } if (1..=8).contains(&amount) => (
            family,
            2 + usize::from(amount - 1) * 2 + usize::from(falling),
        ),
        FluidState::Flowing { .. } => return None,
    };
    Some(
        slot + match family {
            FluidFamily::Water => 0,
            FluidFamily::Lava => 18,
        },
    )
}
