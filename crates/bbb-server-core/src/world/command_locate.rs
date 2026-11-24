//! Live registry-target lookup for deferred command execution.

use std::collections::BTreeSet;

use crate::commands::{LocateKind, LocateRequest, LocateSearchResult};

use super::ServerWorld;

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";

impl ServerWorld {
    pub(crate) fn locate_live_target(&mut self, request: &LocateRequest) -> LocateSearchResult {
        if request.allowed_ids.is_empty() {
            return LocateSearchResult::NotFound;
        }

        match request.kind {
            LocateKind::Structure => {
                return match self
                    .level_mut_by_id(&self.require_level_id(&request.level))
                    .chunks
                    .find_nearest_map_structure(request.origin, &request.allowed_ids, 100, false)
                {
                    Ok(Some((pos, resource_id))) => LocateSearchResult::Found { pos, resource_id },
                    Ok(None) => LocateSearchResult::NotFound,
                    Err(_) => LocateSearchResult::Unavailable,
                };
            }
            LocateKind::Poi => {
                let found = self
                    .level_mut_by_id(&self.require_level_id(&request.level))
                    .chunks
                    .poi_mut()
                    .find_closest_with_type(
                        [request.origin.x, request.origin.y, request.origin.z],
                        256,
                        crate::chunk::poi::PoiOccupancy::Any,
                        |record| request.allowed_ids.iter().any(|id| id == record.type_id),
                    );
                return found.map_or(LocateSearchResult::NotFound, |record| {
                    LocateSearchResult::Found {
                        pos: bbb_protocol::packets::BlockPos {
                            x: record.pos[0],
                            y: record.pos[1],
                            z: record.pos[2],
                        },
                        resource_id: record.type_id.to_string(),
                    }
                });
            }
            LocateKind::Biome => {}
        }

        let mut allowed = BTreeSet::new();
        for biome_id in &request.allowed_ids {
            let Some(registry_id) = self
                .registries
                .registry_element_index(BIOME_REGISTRY, biome_id)
            else {
                return LocateSearchResult::Unavailable;
            };
            allowed.insert(registry_id);
        }

        let Some((pos, registry_id)) = self
            .level_by_id(&self.require_level_id(&request.level))
            .chunks
            .find_closest_biome_3d(request.origin, &allowed, 6400, 32, 64)
        else {
            return LocateSearchResult::NotFound;
        };
        let Some(biome_id) = self
            .registries
            .registry_element_id(BIOME_REGISTRY, registry_id)
        else {
            return LocateSearchResult::Unavailable;
        };

        LocateSearchResult::Found {
            pos,
            resource_id: biome_id.to_string(),
        }
    }
}
