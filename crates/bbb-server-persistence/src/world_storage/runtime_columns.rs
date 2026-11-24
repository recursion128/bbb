//! Reading one column's region records when the world first needs them.

use super::dimensions::{DimensionSectionGeometry, Java26DimensionGeometries};
use super::saved_data::{load_java_26_1_map, load_java_26_1_player_stat_with_limits_and_registry};
use super::{
    Java26ChunkContext, Java26ChunkData, Java26EntityChunkData, Java26LevelKey, Java26LoadedChunk,
    Java26LoadedEntityChunk, Java26LoadedPoiChunk, Java26MapSavedData, Java26PlayerAdvancements,
    Java26PlayerData, Java26PoiChunkData, Java26WorldCandidate, Java26WorldStorage, NamedCompound,
    PlayerDataSource, PlayerDataStore, RegionFileStore, SaveError, SaveResult,
};
use crate::player_advancements::load_java_26_1_player_advancement_with_limits;
use crate::region_file::RegionFileCache;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_server_types::save::{WorldSaveLimits, WorldSavePaths};
use bbb_server_types::stats::ServerStatsCounter;
use std::sync::{Arc, PoisonError, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct Java26RuntimeColumn {
    pub chunk: Option<Java26LoadedChunk>,
    pub entities: Option<Java26LoadedEntityChunk>,
    pub poi: Option<Java26LoadedPoiChunk>,
    /// Why the POI record did not read; the column loads without it.
    pub poi_error: Option<String>,
}

/// Reads single chunk records out of an opened save's region files, sharing the
/// storage's bounded `.mca` handle and 8 KiB header cache across reads.
#[derive(Debug, Clone)]
pub struct Java26ColumnReader {
    paths: WorldSavePaths,
    limits: WorldSaveLimits,
    geometries: Java26DimensionGeometries,
    region_io: Arc<RwLock<()>>,
    region_cache: RegionFileCache,
    content_registry: Arc<ContentRegistry>,
}

impl PartialEq for Java26ColumnReader {
    fn eq(&self, other: &Self) -> bool {
        self.paths == other.paths
            && self.limits == other.limits
            && self.geometries == other.geometries
            && (Arc::ptr_eq(&self.content_registry, &other.content_registry)
                || self.content_registry == other.content_registry)
    }
}

impl Java26ColumnReader {
    pub(super) fn new(
        paths: WorldSavePaths,
        limits: WorldSaveLimits,
        geometries: Java26DimensionGeometries,
        region_io: Arc<RwLock<()>>,
        region_cache: RegionFileCache,
        content_registry: Arc<ContentRegistry>,
    ) -> Self {
        Self {
            paths,
            limits,
            geometries,
            region_io,
            region_cache,
            content_registry,
        }
    }

    pub fn content_registry(&self) -> &Arc<ContentRegistry> {
        &self.content_registry
    }

    /// The terrain and POI records of one column. Its entity record is read
    /// on its own, when the column's entities are first needed. A POI chunk
    /// record whose slot bytes fail to decode leaves the terrain alone, as
    /// vanilla's `SectionStorage.tryRead` treats it as empty
    /// (`SectionStorage.java:189-208`); a corrupt `.mca` container header,
    /// symlink, OS I/O error, or mismatched `DataVersion` refuses the column
    /// identically to `region/` and `entities/`.
    pub fn read_column(
        &self,
        dimension: &Java26LevelKey,
        x: i32,
        z: i32,
    ) -> SaveResult<Java26RuntimeColumn> {
        let chunk = self.read_chunk(dimension, x, z)?;
        let (poi, poi_error) = match self.read_poi_slot(dimension, x, z)? {
            Ok(poi) => (poi, None),
            Err(error) => (None, Some(error.to_string())),
        };
        Ok(Java26RuntimeColumn {
            chunk,
            entities: None,
            poi,
            poi_error,
        })
    }

    /// The column's terrain record alone.
    pub fn read_chunk(
        &self,
        dimension: &Java26LevelKey,
        x: i32,
        z: i32,
    ) -> SaveResult<Option<Java26LoadedChunk>> {
        let geometry = self.geometry(dimension)?;
        self.read_record(dimension, "region", x, z, |raw, timestamp| {
            Ok(Java26LoadedChunk {
                timestamp,
                data: Java26ChunkData::parse_with_content_registry(
                    raw,
                    Java26ChunkContext {
                        expected_x: x,
                        expected_z: z,
                        min_section_y: geometry.min_section_y,
                        section_count: geometry.section_count,
                    },
                    &self.content_registry,
                )?,
            })
        })
    }

    pub fn read_poi(
        &self,
        dimension: &Java26LevelKey,
        x: i32,
        z: i32,
    ) -> SaveResult<Option<Java26LoadedPoiChunk>> {
        let geometry = self.geometry(dimension)?;
        self.read_record(dimension, "poi", x, z, |raw, timestamp| {
            Ok(Java26LoadedPoiChunk {
                timestamp,
                data: Java26PoiChunkData::parse(
                    raw,
                    x,
                    z,
                    geometry.min_section_y,
                    geometry.section_count,
                )?,
            })
        })
    }

    pub fn read_entity_chunk(
        &self,
        dimension: &Java26LevelKey,
        x: i32,
        z: i32,
    ) -> SaveResult<Option<Java26LoadedEntityChunk>> {
        self.geometry(dimension)?;
        self.read_record(dimension, "entities", x, z, |raw, timestamp| {
            Ok(Java26LoadedEntityChunk {
                timestamp,
                data: Java26EntityChunkData::parse_with_content_registry(
                    raw,
                    x,
                    z,
                    &self.content_registry,
                )?,
            })
        })
    }

    /// Reads and parses a single `players/data/<uuid>.dat` (falling back to
    /// `.dat_old`) on demand.
    pub fn read_player(
        &self,
        uuid: Uuid,
    ) -> SaveResult<Option<(PlayerDataSource, Java26PlayerData)>> {
        let player_store = PlayerDataStore::new(self.paths.clone(), self.limits)?;
        let Some(loaded) = player_store.load(uuid)? else {
            return Ok(None);
        };
        let player = Java26PlayerData::parse_candidate_with_content_registry(
            loaded.document,
            uuid,
            &self.content_registry,
        )?;
        Ok(Some((loaded.source, player)))
    }

    /// Reads and parses a single `players/stats/<uuid>.json` document on demand.
    pub fn read_player_stats(&self, uuid: Uuid) -> SaveResult<Option<ServerStatsCounter>> {
        load_java_26_1_player_stat_with_limits_and_registry(
            &self.paths,
            self.limits,
            uuid,
            &self.content_registry,
        )
    }

    /// Reads and parses a single `players/advancements/<uuid>.json` document on
    /// demand.
    pub fn read_player_advancements(
        &self,
        uuid: Uuid,
    ) -> SaveResult<Option<Java26PlayerAdvancements>> {
        load_java_26_1_player_advancement_with_limits(&self.paths, self.limits, uuid)
    }

    /// Reads and parses a single `data/minecraft/maps/<id>.dat` document on
    /// demand.
    pub fn read_map(&self, id: i32) -> SaveResult<Option<Java26MapSavedData>> {
        load_java_26_1_map(&self.paths, self.limits, id)
    }

    fn read_poi_slot(
        &self,
        dimension: &Java26LevelKey,
        x: i32,
        z: i32,
    ) -> SaveResult<std::result::Result<Option<Java26LoadedPoiChunk>, SaveError>> {
        let geometry = self.geometry(dimension)?;
        let _reading = self
            .region_io
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        let directory = self.paths.dimension_dir(dimension).join("poi");
        let path = directory.join(format!("r.{}.{}.mca", x >> 5, z >> 5));
        let store =
            RegionFileStore::with_cache(&path, &directory, self.limits, self.region_cache.clone())?;
        let loaded = store
            .read_chunk_with_timestamp_distinguishing_header(x, z)?
            .and_then(|loaded| {
                loaded
                    .map(|(raw, timestamp)| {
                        Ok(Java26LoadedPoiChunk {
                            timestamp,
                            data: Java26PoiChunkData::parse(
                                raw,
                                x,
                                z,
                                geometry.min_section_y,
                                geometry.section_count,
                            )?,
                        })
                    })
                    .transpose()
            });
        match loaded {
            Ok(poi) => Ok(Ok(poi)),
            Err(error) if super::poi_save::malformed_poi_record(&error) => Ok(Err(error)),
            Err(error) => Err(error),
        }
    }

    fn geometry(&self, dimension: &Java26LevelKey) -> SaveResult<DimensionSectionGeometry> {
        self.geometries
            .get(dimension)
            .map(|geometry| geometry.sections())
            .ok_or_else(|| SaveError::InvalidNbt {
                path: Some(self.paths.world_gen_settings_dat()),
                detail: format!(
                    "runtime column names undeclared level {}",
                    dimension.resource_id()
                ),
            })
    }

    fn read_record<T>(
        &self,
        dimension: &Java26LevelKey,
        kind: &str,
        x: i32,
        z: i32,
        parse: impl FnOnce(NamedCompound, u32) -> SaveResult<T>,
    ) -> SaveResult<Option<T>> {
        let _reading = self
            .region_io
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        let directory = self.paths.dimension_dir(dimension).join(kind);
        let path = directory.join(format!("r.{}.{}.mca", x >> 5, z >> 5));
        let store =
            RegionFileStore::with_cache(&path, &directory, self.limits, self.region_cache.clone())?;
        match store.read_chunk_with_timestamp(x, z)? {
            Some((raw, timestamp)) => parse(raw, timestamp).map(Some),
            None => Ok(None),
        }
    }
}

impl Java26WorldStorage {
    /// A reader over this storage's region files, for the thread that reads
    /// columns while the world runs.
    pub fn runtime_reader(
        &self,
        candidate: &Java26WorldCandidate,
    ) -> SaveResult<Java26ColumnReader> {
        self.require_session_lock("world session lock is no longer valid")?;
        Ok(Java26ColumnReader::new(
            self.paths.clone(),
            self.limits,
            candidate.dimension_geometries.clone(),
            Arc::clone(&self.region_io),
            self.region_cache.clone(),
            Arc::clone(candidate.content_registry()),
        ))
    }

    /// One column with its entity record, read through this storage's lock.
    pub fn read_runtime_column(
        &self,
        candidate: &Java26WorldCandidate,
        dimension: &Java26LevelKey,
        x: i32,
        z: i32,
    ) -> SaveResult<Java26RuntimeColumn> {
        let reader = self.runtime_reader(candidate)?;
        let mut column = reader.read_column(dimension, x, z)?;
        column.entities = reader.read_entity_chunk(dimension, x, z)?;
        Ok(column)
    }

    pub fn read_runtime_entity_chunk(
        &self,
        candidate: &Java26WorldCandidate,
        dimension: &Java26LevelKey,
        x: i32,
        z: i32,
    ) -> SaveResult<Option<Java26LoadedEntityChunk>> {
        self.runtime_reader(candidate)?
            .read_entity_chunk(dimension, x, z)
    }
}

#[cfg(test)]
mod tests;
