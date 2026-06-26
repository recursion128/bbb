use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileTexturesSummary {
    pub skin: Option<ProfileSkinTextureSummary>,
    pub cape: Option<ProfileTextureSummary>,
    pub elytra: Option<ProfileTextureSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSkinTextureSummary {
    pub url: String,
    pub model: PlayerModelTypeSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileTextureSummary {
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerModelTypeSummary {
    Slim,
    Wide,
}

pub fn decode_profile_textures_from_properties<'a>(
    properties: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Option<ProfileTexturesSummary> {
    properties
        .into_iter()
        .filter(|(name, _)| *name == "textures")
        .find_map(|(_, value)| decode_profile_textures_property(value))
}

fn decode_profile_textures_property(value: &str) -> Option<ProfileTexturesSummary> {
    let decoded = decode_base64(value)?;
    let packed: PackedProfileTextures = serde_json::from_slice(&decoded).ok()?;
    let textures = packed.textures?;
    let summary = ProfileTexturesSummary {
        skin: textures
            .skin
            .and_then(ProfileSkinTextureSummary::from_packed),
        cape: textures.cape.and_then(ProfileTextureSummary::from_packed),
        elytra: textures.elytra.and_then(ProfileTextureSummary::from_packed),
    };
    if summary.skin.is_none() && summary.cape.is_none() && summary.elytra.is_none() {
        None
    } else {
        Some(summary)
    }
}

#[derive(Deserialize)]
struct PackedProfileTextures {
    textures: Option<PackedTextureSet>,
}

#[derive(Deserialize)]
struct PackedTextureSet {
    #[serde(rename = "SKIN")]
    skin: Option<PackedTexture>,
    #[serde(rename = "CAPE")]
    cape: Option<PackedTexture>,
    #[serde(rename = "ELYTRA")]
    elytra: Option<PackedTexture>,
}

#[derive(Deserialize)]
struct PackedTexture {
    url: Option<String>,
    metadata: Option<PackedSkinMetadata>,
}

#[derive(Deserialize)]
struct PackedSkinMetadata {
    model: Option<String>,
}

impl ProfileSkinTextureSummary {
    fn from_packed(texture: PackedTexture) -> Option<Self> {
        Some(Self {
            url: texture.url?,
            model: texture.metadata.and_then(|metadata| metadata.model).map_or(
                PlayerModelTypeSummary::Wide,
                |model| {
                    if model == "slim" {
                        PlayerModelTypeSummary::Slim
                    } else {
                        PlayerModelTypeSummary::Wide
                    }
                },
            ),
        })
    }
}

impl ProfileTextureSummary {
    fn from_packed(texture: PackedTexture) -> Option<Self> {
        Some(Self { url: texture.url? })
    }
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut cleaned = Vec::new();
    let mut seen_padding = false;
    for byte in input.bytes() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if byte == b'=' {
            seen_padding = true;
            cleaned.push(byte);
            continue;
        }
        if seen_padding {
            return None;
        }
        cleaned.push(byte);
    }

    let first_padding = cleaned.iter().position(|byte| *byte == b'=');
    let data_len = first_padding.unwrap_or(cleaned.len());
    let padding_len = cleaned.len() - data_len;
    if padding_len > 2 || data_len % 4 == 1 {
        return None;
    }
    if padding_len > 0 {
        if cleaned.len() % 4 != 0 {
            return None;
        }
        let expected_remainder = match padding_len {
            1 => 3,
            2 => 2,
            _ => return None,
        };
        if data_len % 4 != expected_remainder {
            return None;
        }
    }

    let mut output = Vec::with_capacity(data_len / 4 * 3 + 2);
    let mut index = 0;
    while index < data_len {
        let group_len = (data_len - index).min(4);
        let mut values = [0u8; 4];
        for offset in 0..group_len {
            values[offset] = base64_value(cleaned[index + offset])?;
        }
        match group_len {
            4 => {
                output.push((values[0] << 2) | (values[1] >> 4));
                output.push((values[1] << 4) | (values[2] >> 2));
                output.push((values[2] << 6) | values[3]);
            }
            3 => {
                output.push((values[0] << 2) | (values[1] >> 4));
                output.push((values[1] << 4) | (values[2] >> 2));
            }
            2 => {
                output.push((values[0] << 2) | (values[1] >> 4));
            }
            _ => return None,
        }
        index += group_len;
    }
    Some(output)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLIM_TEXTURES_PROPERTY: &str = "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=";
    const WIDE_TEXTURES_PROPERTY: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHBzOi8vdGV4dHVyZXMubWluZWNyYWZ0Lm5ldC90ZXh0dXJlL3dpZGVza2luIn19fQ==";

    #[test]
    fn profile_textures_property_decodes_urls_and_skin_model() {
        let textures =
            decode_profile_textures_from_properties([("textures", SLIM_TEXTURES_PROPERTY)])
                .unwrap();

        assert_eq!(
            textures.skin,
            Some(ProfileSkinTextureSummary {
                url: "https://textures.minecraft.net/texture/skinhash".to_string(),
                model: PlayerModelTypeSummary::Slim,
            })
        );
        assert_eq!(
            textures.cape,
            Some(ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/capehash".to_string(),
            })
        );
        assert_eq!(
            textures.elytra,
            Some(ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/elytrahash".to_string(),
            })
        );
    }

    #[test]
    fn profile_texture_skin_model_defaults_to_wide() {
        let textures =
            decode_profile_textures_from_properties([("textures", WIDE_TEXTURES_PROPERTY)])
                .unwrap();

        assert_eq!(
            textures.skin,
            Some(ProfileSkinTextureSummary {
                url: "https://textures.minecraft.net/texture/wideskin".to_string(),
                model: PlayerModelTypeSummary::Wide,
            })
        );
        assert_eq!(textures.cape, None);
        assert_eq!(textures.elytra, None);
    }

    #[test]
    fn malformed_profile_textures_property_is_ignored() {
        assert_eq!(
            decode_profile_textures_from_properties([
                ("textures", "not base64"),
                ("textures", WIDE_TEXTURES_PROPERTY),
            ]),
            Some(ProfileTexturesSummary {
                skin: Some(ProfileSkinTextureSummary {
                    url: "https://textures.minecraft.net/texture/wideskin".to_string(),
                    model: PlayerModelTypeSummary::Wide,
                }),
                cape: None,
                elytra: None,
            })
        );
        assert_eq!(
            decode_profile_textures_from_properties([("textures", "not base64")]),
            None
        );
    }
}
