//! The `minecraft:weather` saved data (`WeatherData.java:8-21`).
//!
//! `MinecraftServer` builds it once for the whole save --
//! `savedDataStorage.computeIfAbsent(WeatherData.TYPE)`
//! (`MinecraftServer.java:344`) over the storage rooted at
//! `storageSource.getLevelPath(LevelResource.DATA)` (`:330`) -- so the document
//! is `data/minecraft/weather.dat` beside `random_sequences.dat` rather than
//! one file per dimension. `ServerLevel.getWeatherData` returns the server's
//! (`ServerLevel.java:1682-1683`), which is why three levels share one cycle.
//!
//! Every one of the five fields is a `fieldOf` (`WeatherData.java:11-15`), so
//! the writer emits all five; the reader takes the constructor's zero for one
//! that is missing.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::schema_support::{bool_or, compound_or_empty, int_or, require_java_26_1_root};
use bbb_server_types::save::{
    NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};

const SCHEMA: &str = "weather";

/// `WeatherData` (`WeatherData.java:22-26`).
///
/// [`Default`] is the no-argument constructor (`:28-29`) `computeIfAbsent`
/// falls back to, which leaves every field at the JVM's own zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Java26WeatherData {
    pub clear_weather_time: i32,
    pub rain_time: i32,
    pub thunder_time: i32,
    pub raining: bool,
    pub thundering: bool,
}

fn weather_dat(paths: &WorldSavePaths) -> std::path::PathBuf {
    paths.saved_data_dir().join("minecraft").join("weather.dat")
}

/// The weather saved data one save carries.
///
/// A missing file is a default [`Java26WeatherData`], the way
/// `computeIfAbsent` builds one.
pub fn load_java_26_1_weather(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<Java26WeatherData> {
    let path = weather_dat(paths);
    let Some((document, _)) = crate::level_file::read_optional_gzip_compound(&path, limits)? else {
        return Ok(Java26WeatherData::default());
    };
    let data = compound_or_empty(require_java_26_1_root(&document, SCHEMA)?, "data");
    Ok(Java26WeatherData {
        clear_weather_time: int_or(data, "clear_weather_time", 0),
        rain_time: int_or(data, "rain_time", 0),
        thunder_time: int_or(data, "thunder_time", 0),
        raining: bool_or(data, "raining", false),
        thundering: bool_or(data, "thundering", false),
    })
}

/// The `data/minecraft/weather.dat` document one save is written with, the
/// inverse of [`load_java_26_1_weather`].
///
/// Field order is the codec's group order (`WeatherData.java:11-15`).
pub fn java_26_1_weather_document(weather: Java26WeatherData) -> NamedCompound {
    let int = |name: &str, value: i32| NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Int(value),
    };
    let flag = |name: &str, value: bool| NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Byte(i8::from(value)),
    };
    NamedCompound::new(
        "",
        vec![
            NbtSummaryEntry {
                name: "DataVersion".into(),
                value: NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
            },
            NbtSummaryEntry {
                name: "data".into(),
                value: NbtSummaryValue::Compound(vec![
                    int("clear_weather_time", weather.clear_weather_time),
                    int("rain_time", weather.rain_time),
                    int("thunder_time", weather.thunder_time),
                    flag("raining", weather.raining),
                    flag("thundering", weather.thundering),
                ]),
            },
        ],
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::level_file::encode_gzip_named_compound;
    use super::*;

    fn scratch(name: &str) -> WorldSavePaths {
        let root = std::env::temp_dir().join(format!("bbb-weather-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let paths = WorldSavePaths::new(&root).expect("the scratch root is a usable save path");
        fs::create_dir_all(paths.saved_data_dir().join("minecraft"))
            .expect("the data folder is creatable");
        paths
    }

    fn write(paths: &WorldSavePaths, document: &NamedCompound) {
        let limits = WorldSaveLimits::default();
        let path = weather_dat(paths);
        fs::write(
            &path,
            encode_gzip_named_compound(&path, document, limits).expect("the document encodes"),
        )
        .expect("the file is writable");
    }

    #[test]
    fn a_save_without_the_file_starts_from_the_field_initialisers() {
        let paths = scratch("absent");
        let loaded = load_java_26_1_weather(&paths, WorldSaveLimits::default())
            .expect("a missing file is not an error");
        assert_eq!(loaded, Java26WeatherData::default());
        assert_eq!(
            loaded,
            Java26WeatherData {
                clear_weather_time: 0,
                rain_time: 0,
                thunder_time: 0,
                raining: false,
                thundering: false,
            }
        );
    }

    #[test]
    fn a_thundering_storm_round_trips() {
        let paths = scratch("round-trip");
        let weather = Java26WeatherData {
            clear_weather_time: 0,
            rain_time: 8123,
            thunder_time: 4096,
            raining: true,
            thundering: true,
        };
        write(&paths, &java_26_1_weather_document(weather));
        assert_eq!(
            load_java_26_1_weather(&paths, WorldSaveLimits::default())
                .expect("the document reads back"),
            weather
        );
    }

    #[test]
    fn a_clear_sky_with_a_countdown_round_trips() {
        let paths = scratch("clear");
        let weather = Java26WeatherData {
            clear_weather_time: 12345,
            rain_time: 1,
            thunder_time: 1,
            raining: false,
            thundering: false,
        };
        write(&paths, &java_26_1_weather_document(weather));
        assert_eq!(
            load_java_26_1_weather(&paths, WorldSaveLimits::default())
                .expect("the document reads back"),
            weather
        );
    }
}
