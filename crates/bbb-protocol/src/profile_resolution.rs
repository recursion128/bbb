use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::{
    packets::{
        decode_profile_textures_from_properties, GameProfilePropertySummary,
        PlayerSkinPatchSummary, ResolvableProfileKindSummary, ResolvableProfileSummary,
    },
    player_names::valid_player_name,
    JavaString,
};

pub const MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES: usize = 1024 * 1024;
pub const MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PROFILE_TEXTURE_URL_BYTES: usize = 2_048;

/// Authlib `TextureUrlChecker` plus `MinecraftProfileTexture.getHash()`.
/// The checker accepts only HTTP(S) on the exact texture CDN host; the hash
/// is the final path basename with its final extension removed.
pub fn vanilla_profile_texture_hash(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str()? != "textures.minecraft.net"
    {
        return None;
    }
    let filename = parsed.path().rsplit('/').next().unwrap_or_default();
    let basename = filename
        .rfind('.')
        .map_or(filename, |extension| &filename[..extension]);
    Some(basename.to_string())
}

/// Guava `Hashing.sha1().hashUnencodedChars`: UTF-16 code units in the
/// primitive sink's little-endian order.
pub fn vanilla_profile_texture_cache_hash(texture_hash: &str) -> String {
    let mut digest = Sha1::new();
    for code_unit in texture_hash.encode_utf16() {
        digest.update(code_unit.to_le_bytes());
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn profile_texture_handle(url: &str) -> u64 {
    if let Some(texture_hash) = vanilla_profile_texture_hash(url) {
        let cache_hash = vanilla_profile_texture_cache_hash(&texture_hash);
        if let Ok(handle) = u64::from_str_radix(&cache_hash[..16], 16) {
            return handle;
        }
    }
    let mut hash = 0xcbf29ce484222325u64;
    for byte in url.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn is_dynamic_resolvable_profile(profile: &ResolvableProfileSummary) -> bool {
    profile.kind == ResolvableProfileKindSummary::Partial
        && profile.properties.is_empty()
        && profile.name.is_some() != profile.uuid.is_some()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProfileResolutionKey {
    Name(String),
    Uuid(Uuid),
}

impl ProfileResolutionKey {
    pub fn from_profile(profile: &ResolvableProfileSummary) -> Option<Self> {
        if !is_dynamic_resolvable_profile(profile) || profile.profile_textures.is_some() {
            return None;
        }
        match (profile.name.as_deref(), profile.uuid) {
            (Some(name), None) if valid_player_name(name) => Some(Self::Name(name.to_string())),
            (None, Some(uuid)) => Some(Self::Uuid(uuid)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGameProfile {
    pub uuid: Uuid,
    pub name: JavaString,
    pub properties: Vec<GameProfilePropertySummary>,
}

impl ResolvedGameProfile {
    pub fn into_resolvable(self, skin_patch: PlayerSkinPatchSummary) -> ResolvableProfileSummary {
        let profile_textures = decode_profile_textures_from_properties(
            self.properties
                .iter()
                .map(|property| (&property.name, &property.value)),
        );
        ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::GameProfile,
            uuid: Some(self.uuid),
            // `ExtraCodecs.PLAYER_NAME` is `Codec.string(0, 16)` plus
            // `StringUtil.isValidPlayerName`, so this field holds only printable
            // ASCII: a name with an unpaired unit is not one, and is carried as
            // absent rather than as a `U+FFFD` name that would compare equal to
            // a real one.
            name: String::from_utf16(self.name.units()).ok(),
            properties: self.properties,
            profile_textures,
            skin_patch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileResolutionResult {
    pub key: ProfileResolutionKey,
    pub resolved: Option<ResolvedGameProfile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicPlayerTextureKind {
    Cape,
    Elytra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicTextureFetchKind {
    Skin,
    Cape,
    Elytra,
}

impl From<DynamicPlayerTextureKind> for DynamicTextureFetchKind {
    fn from(kind: DynamicPlayerTextureKind) -> Self {
        match kind {
            DynamicPlayerTextureKind::Cape => Self::Cape,
            DynamicPlayerTextureKind::Elytra => Self::Elytra,
        }
    }
}

impl DynamicTextureFetchKind {
    pub const fn max_png_bytes(self) -> usize {
        match self {
            Self::Skin => MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
            Self::Cape | Self::Elytra => MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES,
        }
    }

    pub const fn profile_kind(self) -> Option<DynamicPlayerTextureKind> {
        match self {
            Self::Skin => None,
            Self::Cape => Some(DynamicPlayerTextureKind::Cape),
            Self::Elytra => Some(DynamicPlayerTextureKind::Elytra),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DynamicTextureFetchRequest {
    pub kind: DynamicTextureFetchKind,
    pub url: String,
    pub handle: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicTextureFetchResult {
    pub kind: DynamicTextureFetchKind,
    pub url: String,
    pub handle: u64,
    pub png_bytes: Result<Vec<u8>, String>,
    pub from_network: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_texture_url_and_cache_identity_match_authlib_and_guava() {
        assert_eq!(
            vanilla_profile_texture_hash(
                "https://textures.minecraft.net/texture/skinhash?ignored=yes"
            ),
            Some("skinhash".to_string())
        );
        assert_eq!(
            vanilla_profile_texture_hash("http://textures.minecraft.net/path/abc.png"),
            Some("abc".to_string())
        );
        assert_eq!(
            vanilla_profile_texture_hash("ftp://textures.minecraft.net/texture/skin"),
            None
        );
        assert_eq!(
            vanilla_profile_texture_hash("https://textures.minecraft.net.example/texture/skin"),
            None
        );
        assert_eq!(
            vanilla_profile_texture_hash("https://example.com/texture/skin"),
            None
        );
        assert_eq!(
            vanilla_profile_texture_cache_hash("skinhash"),
            "18aa6cdcc5d1ab4d4952af5973d144d5e69d0881"
        );
        assert_eq!(
            vanilla_profile_texture_cache_hash("abc"),
            "9f04f41a848514162050e3d68c1a7abb441dc2b5"
        );
    }
}
