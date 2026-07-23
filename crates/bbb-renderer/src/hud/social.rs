//! Stable value types shared by native Social Interactions input and HUD.
//!
//! Draw and hit geometry is added by the renderer stage; keeping semantic
//! targets here lets native reject stale generations and revalidate player
//! actions without owning renderer coordinates.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSocialInteractionsPage {
    All,
    Hidden,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSocialInteractionsPlayerAction {
    Hide,
    Show,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSocialInteractionsFocus {
    Search,
    Tab(HudSocialInteractionsPage),
    Player {
        profile_id: [u8; 16],
        action: HudSocialInteractionsPlayerAction,
    },
    BlockingHelp,
    Done,
    LinkOpen,
    LinkCopy,
    LinkCancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSocialInteractionsHitTargetKind {
    Focus(HudSocialInteractionsFocus),
    Scrollbar { scroll_amount: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSocialInteractionsHitTarget {
    pub generation: u64,
    pub kind: HudSocialInteractionsHitTargetKind,
}
