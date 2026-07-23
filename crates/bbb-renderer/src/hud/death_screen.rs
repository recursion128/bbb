//! Shared interaction vocabulary for Vanilla's death and confirmation screens.
//!
//! The renderer owns construction of these generation-tagged hit targets from
//! its exact draw layout. Native input revalidates them against the canonical
//! death-screen generation before acting.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudDeathScreenButton {
    Respawn,
    TitleScreen,
    QuitTitleScreen,
    QuitRespawn,
    LinkOpen,
    LinkCopy,
    LinkCancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudDeathScreenHitTargetKind {
    Button(HudDeathScreenButton),
    CauseLink { run_index: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudDeathScreenHitTarget {
    pub generation: u64,
    pub kind: HudDeathScreenHitTargetKind,
}
