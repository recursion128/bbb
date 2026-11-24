//! Walking a dimension directory: which region files are there, and which
//! chunks their headers list. Chunk bodies are read when a column is needed.

use super::Java26LoadedDimension;
use crate::atomic_file::io_error;
use crate::region_file::RegionFileCache;
use crate::RegionFileStore;
use bbb_server_types::save::{
    Java26LevelKey, SaveError, SaveResult, WorldSaveLimits, WorldSavePaths,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The dimension's stored chunks, from the region headers alone.
pub fn index_dimension(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
    dimension: &Java26LevelKey,
) -> SaveResult<Java26LoadedDimension> {
    index_dimension_with_cache(paths, limits, dimension, &RegionFileCache::default())
}

pub(super) fn index_dimension_with_cache(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
    dimension: &Java26LevelKey,
    cache: &RegionFileCache,
) -> SaveResult<Java26LoadedDimension> {
    let root = paths.dimension_dir(dimension);
    Ok(Java26LoadedDimension {
        stored_chunks: index_region_kind(&root.join("region"), limits, cache)?,
        stored_entity_chunks: index_region_kind(&root.join("entities"), limits, cache)?,
        stored_poi_chunks: index_region_kind(&root.join("poi"), limits, cache)?,
        ..Java26LoadedDimension::new(dimension.clone())
    })
}

fn index_region_kind(
    directory: &Path,
    limits: WorldSaveLimits,
    _cache: &RegionFileCache,
) -> SaveResult<BTreeSet<(i32, i32)>> {
    let mut result = BTreeSet::new();
    let candidates = discover_region_files(directory)?;
    if candidates.is_empty() {
        return Ok(result);
    }
    // Validate directory ancestry once instead of repeating per `.mca` file, and
    // read headers uncached so startup does not thrash the 256-FD RegionFileCache.
    let _ = RegionFileStore::new(&candidates[0].path, directory, limits)?;
    for candidate in candidates {
        let coords = RegionFileStore::occupied_local_coords_uncached(&candidate.path, limits)?;
        for (local_x, local_z) in coords {
            result.insert((
                global_chunk_coordinate(candidate.region_x, local_x)?,
                global_chunk_coordinate(candidate.region_z, local_z)?,
            ));
        }
    }
    Ok(result)
}

#[derive(Debug)]
struct RegionCandidate {
    path: PathBuf,
    region_x: i32,
    region_z: i32,
}

fn discover_region_files(directory: &Path) -> SaveResult<Vec<RegionCandidate>> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(io_error("reading region directory", directory, source)),
    };
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry
            .map_err(|source| io_error("reading region directory entry", directory, source))?;
        let file_type = entry.file_type().map_err(|source| {
            io_error("reading region directory entry type", &entry.path(), source)
        })?;
        if file_type.is_symlink() {
            return Err(SaveError::InvalidPath {
                path: entry.path(),
                detail: "managed region directory must not contain symbolic links",
            });
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some((region_x, region_z)) = parse_region_file_name(&name) else {
            continue;
        };
        result.push(RegionCandidate {
            path: entry.path(),
            region_x,
            region_z,
        });
    }
    Ok(result)
}

pub fn parse_region_file_name(name: &str) -> Option<(i32, i32)> {
    let body = name.strip_prefix("r.")?.strip_suffix(".mca")?;
    let (x, z) = body.split_once('.')?;
    if z.contains('.') {
        return None;
    }
    let x_value = x.parse::<i32>().ok()?;
    let z_value = z.parse::<i32>().ok()?;
    (x_value.to_string() == x && z_value.to_string() == z).then_some((x_value, z_value))
}

pub fn global_chunk_coordinate(region: i32, local: u8) -> SaveResult<i32> {
    region
        .checked_mul(32)
        .and_then(|base| base.checked_add(i32::from(local)))
        .ok_or_else(|| SaveError::InvalidNbt {
            path: None,
            detail: format!("region coordinate {region} overflows global chunk coordinates"),
        })
}
