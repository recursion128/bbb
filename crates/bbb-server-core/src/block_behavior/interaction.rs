//! The acting `Player` a real click or dig hands a block body, and the
//! `GameEvent.Context.sourceEntity` it implies. Automatic callers (scheduled
//! ticks, redstone, explosions) have no player and raise a `None` source.

use bbb_protocol::packets::Direction;

/// The facts block bodies read off the `Player` argument of
/// `BlockBehaviour.useWithoutItem` / `useItemOn` and `Block.playerWillDestroy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockInteractionPlayer {
    /// `Entity.getId()`, the source entity of every game event the
    /// interaction raises.
    pub entity_id: i32,
    /// `player.getAbilities().mayBuild`.
    pub may_build: bool,
    /// `Entity.getDirection()`.
    pub horizontal_direction: Direction,
}

impl BlockInteractionPlayer {
    /// The `GameEvent.Context.sourceEntity` this player supplies.
    #[must_use]
    pub const fn source_entity_id(self) -> Option<i32> {
        Some(self.entity_id)
    }
}
