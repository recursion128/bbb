#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ParticleFacingCameraMode {
    #[default]
    LookAtXyz,
    LookAtY,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleTextureAtlasKind {
    Particles,
    Terrain,
    Items,
}
