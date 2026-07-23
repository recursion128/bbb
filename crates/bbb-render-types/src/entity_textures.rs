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

impl EntityDynamicPlayerTexture {
    pub const fn dynamic_texture(self) -> EntityDynamicTexture {
        EntityDynamicTexture {
            handle: self.handle,
        }
    }
}

/// A renderer-owned variable-size entity texture addressed independently from static pack atlases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityDynamicTexture {
    pub handle: u64,
}

/// One resolved armor-trim atlas sprite. `texture` addresses the shared runtime entity atlas;
/// `decal` selects vanilla's equal-depth, no-depth-write armor decal pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityArmorTrim {
    pub texture: EntityDynamicTexture,
    pub decal: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub const ALL: [Self; 18] = [
        Self::SlimAlex,
        Self::SlimAri,
        Self::SlimEfe,
        Self::SlimKai,
        Self::SlimMakena,
        Self::SlimNoor,
        Self::SlimSteve,
        Self::SlimSunny,
        Self::SlimZuri,
        Self::WideAlex,
        Self::WideAri,
        Self::WideEfe,
        Self::WideKai,
        Self::WideMakena,
        Self::WideNoor,
        Self::WideSteve,
        Self::WideSunny,
        Self::WideZuri,
    ];

    pub fn from_vanilla_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    pub fn from_texture_path(path: &str) -> Option<Self> {
        let path = path.strip_prefix("minecraft:").unwrap_or(path);
        Self::ALL
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityModelTextureRef {
    pub path: &'static str,
    /// Logical model UV extent from the baked LayerDefinition. Pack images
    /// may use any positive physical resolution and are stretched across
    /// this extent just as Vanilla's NativeImageBackedTexture is.
    pub size: [u32; 2],
}
