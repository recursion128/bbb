//! `PlayerAdvancements` (`net/minecraft/server/PlayerAdvancements.java`):
//! session advancement progress, tree layout, visibility, and wire updates.

pub mod display;
mod layout;
pub mod store;
pub mod tree;
pub mod visibility;
pub mod wire;

pub use bbb_server_types::advancements::{
    file, format_obtained_time, parse_obtained_time, player_advancements_dir,
    player_advancements_file, progress, requirements, AdvancementProgress, AdvancementRequirements,
    CriterionProgress, Java26PlayerAdvancements, PlayerAdvancementsParseError,
};

#[cfg(test)]
mod tests;

pub use self::display::{AdvancementDisplay, AdvancementFrame};
pub use self::store::{AdvancementsUpdate, AwardOutcome, PlayerAdvancements};
pub use self::tree::{
    AdvancementNode, AdvancementSummary, AdvancementTree, AdvancementTreeWarning,
};
pub use self::wire::advancements_update_to_wire;
