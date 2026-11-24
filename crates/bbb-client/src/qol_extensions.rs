//! CLI-configured client quality-of-life extensions (`--ext <slug>` / `?ext=<slug>`).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClientExtension {
    Zoom,
    HudInfo,
    ItemInspector,
}

pub(crate) fn parse_client_extension_slug(raw: &str) -> Result<ClientExtension, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "zoom" => Ok(ClientExtension::Zoom),
        "hud_info" => Ok(ClientExtension::HudInfo),
        "item_inspector" => Ok(ClientExtension::ItemInspector),
        other => Err(format!(
            "unknown extension '{other}' (expected 'zoom', 'hud_info', or 'item_inspector')"
        )),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ClientExtensionSet {
    pub(crate) zoom: bool,
    pub(crate) hud_info: bool,
    pub(crate) item_inspector: bool,
}

impl ClientExtensionSet {
    pub(crate) fn from_extensions(extensions: impl IntoIterator<Item = ClientExtension>) -> Self {
        let mut set = Self::default();
        for ext in extensions {
            match ext {
                ClientExtension::Zoom => set.zoom = true,
                ClientExtension::HudInfo => set.hud_info = true,
                ClientExtension::ItemInspector => set.item_inspector = true,
            }
        }
        set
    }

    #[cfg(test)]
    pub(crate) fn is_empty(self) -> bool {
        !self.zoom && !self.hud_info && !self.item_inspector
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qol_extension_list_parses_duplicates_and_rejects_unknown() {
        let parsed: Vec<ClientExtension> = ["zoom", " HUD_INFO ", "zoom", "item_inspector"]
            .into_iter()
            .map(|raw| parse_client_extension_slug(raw).expect("valid slug"))
            .collect();
        let set = ClientExtensionSet::from_extensions(parsed);
        assert_eq!(
            set,
            ClientExtensionSet {
                zoom: true,
                hud_info: true,
                item_inspector: true,
            }
        );
        assert!(ClientExtensionSet::default().is_empty());
        assert!(!set.is_empty());
        assert!(parse_client_extension_slug("minimap").is_err());
    }
}
