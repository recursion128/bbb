//! Ground navigation tuning parameters stored on `MobSpec`.

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct GroundTuning {
    pub(super) can_float: bool,
    pub(super) can_walk_over_fences: bool,
    pub(super) required_path_length: Option<f32>,
    /// `Creeper.getMaxFallDistance`.
    pub(super) creeper_fall: bool,
    /// `Sniffer.onPathfindingStart`.
    pub(super) water_while_burning_or_wet: bool,
}

impl GroundTuning {
    pub(crate) const DEFAULT: Self = Self {
        can_float: false,
        can_walk_over_fences: false,
        required_path_length: None,
        creeper_fall: false,
        water_while_burning_or_wet: false,
    };

    pub(crate) const FLOAT: Self = Self {
        can_float: true,
        ..Self::DEFAULT
    };

    pub(crate) const SNIFFER: Self = Self {
        water_while_burning_or_wet: true,
        ..Self::FLOAT
    };

    pub(crate) const CAMEL: Self = Self {
        can_walk_over_fences: true,
        ..Self::FLOAT
    };

    pub(crate) const VILLAGER: Self = Self {
        required_path_length: Some(48.0),
        ..Self::FLOAT
    };

    pub(crate) const CREEPER: Self = Self {
        creeper_fall: true,
        ..Self::DEFAULT
    };

    pub(crate) const fn required(path_length: f32) -> Self {
        Self {
            required_path_length: Some(path_length),
            ..Self::DEFAULT
        }
    }
}
