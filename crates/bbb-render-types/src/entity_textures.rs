/// Vanilla `SkullBlock.Type` values currently rendered by bbb's `CustomHeadLayer` skull branch.
///
/// Static mob heads share the same 8x8x8 mob-head layer and differ only by entity texture. Player
/// heads use the humanoid head layer (base head plus hat). Profileless heads use vanilla's fixed
/// `DefaultPlayerSkin.getDefaultTexture()` skin; profiled heads can use the UUID/default-skin fallback
/// and can now carry a dynamic skin handle/model while live texture upload stays deferred. Dragon and
/// piglin heads use their specialized animated skull models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityCustomHeadSkull {
    Skeleton,
    WitherSkeleton,
    Player(EntityPlayerSkin),
    Zombie,
    Creeper,
    Dragon,
    Piglin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPlayerSkin {
    /// Profileless `player_head` fallback: vanilla `DefaultPlayerSkin.getDefaultTexture()`.
    Default(EntityDefaultPlayerSkin),
    /// A profile-backed player skin that resolves to a built-in default skin for now.
    ProfiledDefault(EntityDefaultPlayerSkin),
    Dynamic(EntityDynamicPlayerSkin),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityDynamicPlayerSkin {
    pub handle: u64,
    pub fallback: EntityDefaultPlayerSkin,
    pub model: EntityPlayerSkinModel,
    pub status: EntityDynamicPlayerSkinStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityDynamicPlayerTexture {
    pub handle: u64,
    pub kind: EntityDynamicPlayerTextureKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDynamicPlayerTextureKind {
    Cape,
    Elytra,
}

/// One equipment-asset layer texture projected onto an entity layer renderer. This mirrors one
/// `EquipmentClientInfo.Layer` after the native side has resolved its pack texture path to a renderer
/// atlas texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityEquipmentLayerTexture {
    pub texture: EntityModelTextureRef,
    pub use_player_texture: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDynamicPlayerSkinStatus {
    Loading,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPlayerSkinModel {
    Slim,
    Wide,
}

impl EntityPlayerSkinModel {
    pub const fn is_slim(self) -> bool {
        matches!(self, Self::Slim)
    }
}

impl EntityPlayerSkin {
    pub const fn fallback(self) -> EntityDefaultPlayerSkin {
        match self {
            Self::Default(skin) => skin,
            Self::ProfiledDefault(skin) => skin,
            Self::Dynamic(skin) => skin.fallback,
        }
    }

    pub const fn model(self) -> EntityPlayerSkinModel {
        match self {
            Self::Default(skin) => skin.model(),
            Self::ProfiledDefault(skin) => skin.model(),
            Self::Dynamic(skin) => skin.model,
        }
    }

    pub const fn default_for_model(slim: bool) -> Self {
        if slim {
            Self::Default(EntityDefaultPlayerSkin::SlimSteve)
        } else {
            Self::Default(EntityDefaultPlayerSkin::WideSteve)
        }
    }

    pub const fn is_slim(self) -> bool {
        self.model().is_slim()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDefaultPlayerSkin {
    SlimAlex,
    SlimAri,
    SlimEfe,
    SlimKai,
    SlimMakena,
    SlimNoor,
    SlimSteve,
    SlimSunny,
    SlimZuri,
    WideAlex,
    WideAri,
    WideEfe,
    WideKai,
    WideMakena,
    WideNoor,
    WideSteve,
    WideSunny,
    WideZuri,
}

impl EntityDefaultPlayerSkin {
    pub fn from_vanilla_index(index: usize) -> Self {
        match index % DEFAULT_PLAYER_SKINS.len() {
            0 => Self::SlimAlex,
            1 => Self::SlimAri,
            2 => Self::SlimEfe,
            3 => Self::SlimKai,
            4 => Self::SlimMakena,
            5 => Self::SlimNoor,
            6 => Self::SlimSteve,
            7 => Self::SlimSunny,
            8 => Self::SlimZuri,
            9 => Self::WideAlex,
            10 => Self::WideAri,
            11 => Self::WideEfe,
            12 => Self::WideKai,
            13 => Self::WideMakena,
            14 => Self::WideNoor,
            15 => Self::WideSteve,
            16 => Self::WideSunny,
            _ => Self::WideZuri,
        }
    }

    pub fn from_texture_path(path: &str) -> Option<Self> {
        let path = path.strip_prefix("minecraft:").unwrap_or(path);
        DEFAULT_PLAYER_SKINS
            .iter()
            .copied()
            .find(|skin| skin.texture_path() == path)
    }

    pub const fn texture_path(self) -> &'static str {
        match self {
            Self::SlimAlex => "textures/entity/player/slim/alex.png",
            Self::SlimAri => "textures/entity/player/slim/ari.png",
            Self::SlimEfe => "textures/entity/player/slim/efe.png",
            Self::SlimKai => "textures/entity/player/slim/kai.png",
            Self::SlimMakena => "textures/entity/player/slim/makena.png",
            Self::SlimNoor => "textures/entity/player/slim/noor.png",
            Self::SlimSteve => "textures/entity/player/slim/steve.png",
            Self::SlimSunny => "textures/entity/player/slim/sunny.png",
            Self::SlimZuri => "textures/entity/player/slim/zuri.png",
            Self::WideAlex => "textures/entity/player/wide/alex.png",
            Self::WideAri => "textures/entity/player/wide/ari.png",
            Self::WideEfe => "textures/entity/player/wide/efe.png",
            Self::WideKai => "textures/entity/player/wide/kai.png",
            Self::WideMakena => "textures/entity/player/wide/makena.png",
            Self::WideNoor => "textures/entity/player/wide/noor.png",
            Self::WideSteve => "textures/entity/player/wide/steve.png",
            Self::WideSunny => "textures/entity/player/wide/sunny.png",
            Self::WideZuri => "textures/entity/player/wide/zuri.png",
        }
    }

    pub const fn model(self) -> EntityPlayerSkinModel {
        match self {
            Self::SlimAlex
            | Self::SlimAri
            | Self::SlimEfe
            | Self::SlimKai
            | Self::SlimMakena
            | Self::SlimNoor
            | Self::SlimSteve
            | Self::SlimSunny
            | Self::SlimZuri => EntityPlayerSkinModel::Slim,
            Self::WideAlex
            | Self::WideAri
            | Self::WideEfe
            | Self::WideKai
            | Self::WideMakena
            | Self::WideNoor
            | Self::WideSteve
            | Self::WideSunny
            | Self::WideZuri => EntityPlayerSkinModel::Wide,
        }
    }
}

const DEFAULT_PLAYER_SKINS: [EntityDefaultPlayerSkin; 18] = [
    EntityDefaultPlayerSkin::SlimAlex,
    EntityDefaultPlayerSkin::SlimAri,
    EntityDefaultPlayerSkin::SlimEfe,
    EntityDefaultPlayerSkin::SlimKai,
    EntityDefaultPlayerSkin::SlimMakena,
    EntityDefaultPlayerSkin::SlimNoor,
    EntityDefaultPlayerSkin::SlimSteve,
    EntityDefaultPlayerSkin::SlimSunny,
    EntityDefaultPlayerSkin::SlimZuri,
    EntityDefaultPlayerSkin::WideAlex,
    EntityDefaultPlayerSkin::WideAri,
    EntityDefaultPlayerSkin::WideEfe,
    EntityDefaultPlayerSkin::WideKai,
    EntityDefaultPlayerSkin::WideMakena,
    EntityDefaultPlayerSkin::WideNoor,
    EntityDefaultPlayerSkin::WideSteve,
    EntityDefaultPlayerSkin::WideSunny,
    EntityDefaultPlayerSkin::WideZuri,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityModelTextureRef {
    pub path: &'static str,
    pub size: [u32; 2],
}
