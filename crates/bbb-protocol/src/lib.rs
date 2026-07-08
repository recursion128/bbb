mod component;

pub use component::{
    decode_sign_block_entity_nbt, decode_styled_component_summary, styled_runs_summary_text,
    ComponentStyle, SignBlockEntityNbt, SignTextNbt, StyledTextRun,
};

pub mod codec;
pub mod entity_types;
pub mod frame;
pub mod ids;
pub mod packets;

pub const MC_VERSION: &str = "26.1";
pub const PROTOCOL_VERSION: i32 = 775;
pub const MC_DATA_VERSION: i32 = 4786;
pub const MC_DATA_VERSION_SERIES: &str = "main";
pub const MC_BUILD_TIME: &str = "2026-03-24T12:09:43+00:00";
pub const MC_RESOURCE_PACK_FORMAT: PackFormatVersion = PackFormatVersion::new(84, 0);
pub const MC_DATA_PACK_FORMAT: PackFormatVersion = PackFormatVersion::new(101, 1);
pub const MC_STABLE: bool = true;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackFormatVersion {
    pub major: u32,
    pub minor: u32,
}

impl PackFormatVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub fn to_vanilla_string(self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn version_constants_match_local_vanilla_version_json() {
        let version_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../mc-code/sources")
            .join(MC_VERSION)
            .join("version.json");
        if !version_path.exists() {
            eprintln!(
                "skipping local vanilla version.json check; missing {}",
                version_path.display()
            );
            return;
        }

        let raw = std::fs::read_to_string(&version_path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();

        assert_eq!(value["id"].as_str(), Some(MC_VERSION));
        assert_eq!(value["name"].as_str(), Some(MC_VERSION));
        assert_eq!(
            value["world_version"].as_i64(),
            Some(MC_DATA_VERSION.into())
        );
        assert_eq!(value["series_id"].as_str(), Some(MC_DATA_VERSION_SERIES));
        assert_eq!(
            value["protocol_version"].as_i64(),
            Some(PROTOCOL_VERSION.into())
        );
        assert_eq!(
            value["pack_version"]["resource_major"].as_u64(),
            Some(MC_RESOURCE_PACK_FORMAT.major.into())
        );
        assert_eq!(
            value["pack_version"]["resource_minor"].as_u64(),
            Some(MC_RESOURCE_PACK_FORMAT.minor.into())
        );
        assert_eq!(
            value["pack_version"]["data_major"].as_u64(),
            Some(MC_DATA_PACK_FORMAT.major.into())
        );
        assert_eq!(
            value["pack_version"]["data_minor"].as_u64(),
            Some(MC_DATA_PACK_FORMAT.minor.into())
        );
        assert_eq!(value["build_time"].as_str(), Some(MC_BUILD_TIME));
        assert_eq!(value["stable"].as_bool(), Some(MC_STABLE));
    }

    #[test]
    fn pack_format_version_string_matches_vanilla_to_string() {
        assert_eq!(MC_RESOURCE_PACK_FORMAT.to_vanilla_string(), "84.0");
        assert_eq!(MC_DATA_PACK_FORMAT.to_vanilla_string(), "101.1");
    }
}
