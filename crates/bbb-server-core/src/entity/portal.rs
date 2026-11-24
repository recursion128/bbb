use bbb_protocol::packets::BlockPos;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalKind {
    Nether,
    End,
    EndGateway,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalProcessor {
    kind: PortalKind,
    entry_position: BlockPos,
    portal_time: i32,
    inside_this_tick: bool,
}

impl PortalProcessor {
    // `PortalProcessor.java:15-19`.
    pub const fn new(kind: PortalKind, entry_position: BlockPos) -> Self {
        Self {
            kind,
            entry_position,
            portal_time: 0,
            inside_this_tick: true,
        }
    }

    pub const fn kind(&self) -> PortalKind {
        self.kind
    }

    pub const fn entry_position(&self) -> BlockPos {
        self.entry_position
    }

    pub const fn portal_time(&self) -> i32 {
        self.portal_time
    }

    pub const fn is_inside_this_tick(&self) -> bool {
        self.inside_this_tick
    }

    // `PortalProcessor.java:21-28`; `PortalProcessor.java:39-45`.
    pub fn process_portal_teleportation(&mut self, transition_time: i32, allowed: bool) -> bool {
        if !self.inside_this_tick {
            self.portal_time = self.portal_time.wrapping_sub(4).max(0);
            return false;
        }
        self.inside_this_tick = false;
        if !allowed {
            return false;
        }
        let time_before_increment = self.portal_time;
        self.portal_time = self.portal_time.wrapping_add(1);
        time_before_increment >= transition_time
    }

    pub const fn has_expired(&self) -> bool {
        self.portal_time <= 0
    }

    pub fn is_same_portal(&self, kind: PortalKind) -> bool {
        self.kind == kind
    }

    pub fn mark_inside(&mut self, entry_position: BlockPos) {
        self.entry_position = entry_position;
        self.inside_this_tick = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalTransitionRequest {
    pub kind: PortalKind,
    pub entry: BlockPos,
}

/// `Player.getDimensionChangingDelay` (`Player.java:392-394`).
pub const PLAYER_PORTAL_COOLDOWN: i32 = 10;

/// `Entity.getDimensionChangingDelay` (`Entity.java:580-581`).
pub const ENTITY_PORTAL_COOLDOWN: i32 = 300;

/// `Entity.setAsInsidePortal` (`Entity.java:2530-2538`).
pub fn record_portal_contact(
    portal_cooldown: &mut i32,
    portal_process: &mut Option<PortalProcessor>,
    kind: PortalKind,
    entry_position: BlockPos,
    cooldown_on_busy: i32,
) {
    if *portal_cooldown > 0 {
        *portal_cooldown = cooldown_on_busy;
        return;
    }
    match portal_process.as_mut() {
        Some(processor) if processor.is_same_portal(kind) => {
            if !processor.is_inside_this_tick() {
                processor.mark_inside(entry_position);
            }
        }
        _ => *portal_process = Some(PortalProcessor::new(kind, entry_position)),
    }
}

/// `Entity.handlePortal` (`Entity.java:2540-2561`) over an active `PortalProcessor`.
pub fn advance_portal_process(
    portal_cooldown: &mut i32,
    portal_process: &mut Option<PortalProcessor>,
    transition_time: i32,
    allowed: bool,
    cooldown_on_teleport: i32,
) -> Option<PortalTransitionRequest> {
    let processor = portal_process.as_mut()?;
    if processor.process_portal_teleportation(transition_time, allowed) {
        *portal_cooldown = cooldown_on_teleport;
        return Some(PortalTransitionRequest {
            kind: processor.kind(),
            entry: processor.entry_position(),
        });
    }
    if processor.has_expired() {
        *portal_process = None;
    }
    None
}
