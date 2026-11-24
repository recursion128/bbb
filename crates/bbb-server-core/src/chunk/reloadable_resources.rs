use std::sync::Arc;

use bbb_server_datapack::startup::DatapackStartup;

use super::noise_generator::NoiseReloadableResources;
use super::{ChunkSource, ChunkStore, SaveResult};

impl ChunkStore {
    pub fn prepare_noise_reloadable_resources(
        &self,
        startup: &Arc<DatapackStartup>,
    ) -> SaveResult<Option<NoiseReloadableResources>> {
        let ChunkSource::NoiseCarvers(program) = &self.source else {
            return Ok(None);
        };
        let replacement = program.prepare_reloadable_resources(startup)?;
        Ok(Some(replacement))
    }

    pub fn install_noise_reloadable_resources(&mut self, replacement: NoiseReloadableResources) {
        let ChunkSource::NoiseCarvers(program) = &mut self.source else {
            panic!("a prepared noise replacement requires a noise chunk source")
        };
        if let Some(program) = Arc::get_mut(program) {
            program.install_reloadable_resources(replacement);
        } else {
            *program = Arc::new(program.clone_with_reloadable_resources(replacement));
        }
    }

    #[cfg(test)]
    pub fn noise_reloadable_resources_startup_is_for_test(
        &self,
        startup: &Arc<DatapackStartup>,
    ) -> bool {
        match &self.source {
            ChunkSource::NoiseCarvers(program) => {
                program.reloadable_resources_startup_is_for_test(startup)
            }
            ChunkSource::Flat(_) | ChunkSource::Debug(_) => false,
        }
    }

    #[cfg(test)]
    pub fn noise_reloadable_feature_contains_for_test(
        &self,
        biome_id: &str,
        placed_feature_id: &str,
    ) -> bool {
        match &self.source {
            ChunkSource::NoiseCarvers(program) => {
                program.reloadable_feature_contains_for_test(biome_id, placed_feature_id)
            }
            ChunkSource::Flat(_) | ChunkSource::Debug(_) => false,
        }
    }

    #[cfg(test)]
    pub fn noise_reloadable_structure_template_source_for_test(
        &self,
        template_id: &str,
    ) -> Option<&str> {
        match &self.source {
            ChunkSource::NoiseCarvers(program) => {
                program.reloadable_structure_template_source_for_test(template_id)
            }
            ChunkSource::Flat(_) | ChunkSource::Debug(_) => None,
        }
    }
}
