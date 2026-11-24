//! Shared environment attribute values.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedAvailability {
    Always,
    WhenDark,
    Never,
}

impl BedAvailability {
    // BedRule.java:51-57.
    pub fn allows(self, is_dark: bool) -> bool {
        match self {
            Self::Always => true,
            Self::WhenDark => is_dark,
            Self::Never => false,
        }
    }

    // BedRule.java:39-42.
    pub fn from_serialized_name(value: &str) -> Option<Self> {
        match value {
            "always" => Some(Self::Always),
            "when_dark" => Some(Self::WhenDark),
            "never" => Some(Self::Never),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BedRule {
    pub can_sleep: BedAvailability,
    pub can_set_spawn: BedAvailability,
    pub explodes: bool,
    pub error_message: Option<bbb_protocol::NbtSummaryValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriState {
    True,
    False,
    Default,
}

impl TriState {
    pub fn from_bool(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }

    pub fn from_serialized_name(value: &str) -> Option<Self> {
        match value {
            "true" => Some(Self::True),
            "false" => Some(Self::False),
            "default" => Some(Self::Default),
            _ => None,
        }
    }

    // TriState.java:30-36.
    pub fn to_boolean(self, default: bool) -> bool {
        match self {
            Self::True => true,
            Self::False => false,
            Self::Default => default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoonPhase(u8);

impl MoonPhase {
    pub const FULL_MOON: Self = Self(0);

    // MoonPhase.java:6-14.
    pub fn from_serialized_name(value: &str) -> Option<Self> {
        Some(match value {
            "full_moon" => Self::FULL_MOON,
            "waning_gibbous" => Self(1),
            "third_quarter" => Self(2),
            "waning_crescent" => Self(3),
            "new_moon" => Self(4),
            "waxing_crescent" => Self(5),
            "first_quarter" => Self(6),
            "waxing_gibbous" => Self(7),
            _ => return None,
        })
    }

    // DimensionType.java:58.
    pub fn brightness(self) -> f32 {
        [1.0, 0.75, 0.5, 0.25, 0.0, 0.25, 0.5, 0.75][usize::from(self.0)]
    }
}
