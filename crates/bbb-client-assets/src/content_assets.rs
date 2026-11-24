use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use bbb_protocol::content_registry::{
    ContentRegistry, ModBlockSpec, ModItemSpec, ModParticleSpec, ModSoundEventSpec,
};
use serde::{Deserialize, Serialize};

use crate::{
    block_models::{
        BlockFaceTextures, BlockModelCatalog, BlockModelDisplayTransforms, BlockModelGuiLight,
        BlockModelShape, BlockRenderModel,
    },
    item_cuboid_models::{ItemCuboidModel, ItemCuboidModelCatalog, ItemCuboidTexture},
    item_models::ItemModelCatalog,
    particle_definitions::ParticleDefinitionCatalog,
    resources::{PackResourceStack, ResourceLocation},
    selected_packs::ClientResourceGeneration,
    sound_event_registry::SoundEventRegistry,
    sounds::{SoundCatalog, SoundEventDefinition},
};

pub const MISSING_TEXTURE_ID: &str = "minecraft:missingno";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModAssetModelKind {
    PackBlockstate,
    PackBlockModel,
    PackItemDefinition,
    PackItemModel,
    FallbackCube,
    FallbackFlat,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModAssetTint {
    pub rgb: [f32; 3],
    pub argb: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModBlockAssetProjection {
    pub block_id: i32,
    pub first_state_id: i32,
    pub state_count: u16,
    pub resource_id: String,
    pub kind: ModAssetModelKind,
    pub render_model: BlockRenderModel,
    pub texture_ids: Vec<String>,
    pub fallback_tint: ModAssetTint,
    pub resolved_from_pack: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModItemAssetProjection {
    pub item_id: i32,
    pub resource_id: String,
    pub is_block_item: bool,
    pub kind: ModAssetModelKind,
    pub generated: bool,
    pub texture_ids: Vec<String>,
    pub model: Option<ItemCuboidModel>,
    pub fallback_block_model: Option<BlockRenderModel>,
    pub fallback_tint: ModAssetTint,
    pub resolved_from_pack: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModSoundEventAssetProjection {
    pub sound_event_id: i32,
    pub resource_id: String,
    pub definition: Option<SoundEventDefinition>,
    pub resolved_from_pack: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModParticleAssetProjection {
    pub particle_id: i32,
    pub resource_id: String,
    pub override_limiter: bool,
    pub textures: Vec<String>,
    pub fallback_tint: ModAssetTint,
    pub resolved_from_pack: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContentRegistryAssetProjection {
    pub generation: u64,
    pub blocks: BTreeMap<String, ModBlockAssetProjection>,
    pub items: BTreeMap<String, ModItemAssetProjection>,
    pub sound_events: BTreeMap<String, ModSoundEventAssetProjection>,
    pub particles: BTreeMap<String, ModParticleAssetProjection>,
    pub sound_event_registry: SoundEventRegistry,
}

impl ContentRegistryAssetProjection {
    pub fn block(&self, resource_id: &str) -> Option<&ModBlockAssetProjection> {
        self.blocks.get(resource_id)
    }

    pub fn item(&self, resource_id: &str) -> Option<&ModItemAssetProjection> {
        self.items.get(resource_id)
    }

    pub fn sound_event(&self, resource_id: &str) -> Option<&ModSoundEventAssetProjection> {
        self.sound_events.get(resource_id)
    }

    pub fn particle(&self, resource_id: &str) -> Option<&ModParticleAssetProjection> {
        self.particles.get(resource_id)
    }

    pub fn sound_event_registry(&self) -> &SoundEventRegistry {
        &self.sound_event_registry
    }
}

pub(crate) fn project_content_registry_from_generation(
    generation: &ClientResourceGeneration,
    registry: &ContentRegistry,
) -> Result<ContentRegistryAssetProjection> {
    let stack = generation.resources();
    let cuboid_models = generation.cuboid_models()?;
    let block_catalog = BlockModelCatalog::load_with_models(stack, cuboid_models.clone())?;
    let item_models = ItemModelCatalog::load_resource_stack(stack)?;
    let item_cuboids = ItemCuboidModelCatalog::from_models(cuboid_models);
    let sound_catalog = SoundCatalog::load_resource_stack(stack)?;
    let particle_catalog = ParticleDefinitionCatalog::load_resource_stack(stack)?;

    project_from_catalogs(
        generation.generation(),
        stack,
        &block_catalog,
        &item_models,
        &item_cuboids,
        &sound_catalog,
        &particle_catalog,
        registry,
    )
}

pub(crate) fn project_content_registry_from_stack(
    generation_id: u64,
    stack: &PackResourceStack,
    registry: &ContentRegistry,
) -> Result<ContentRegistryAssetProjection> {
    let block_catalog = BlockModelCatalog::load_resource_stack(stack)?;
    let item_models = ItemModelCatalog::load_resource_stack(stack)?;
    let item_cuboids = ItemCuboidModelCatalog::load_resource_stack(stack)?;
    let sound_catalog = SoundCatalog::load_resource_stack(stack)?;
    let particle_catalog = ParticleDefinitionCatalog::load_resource_stack(stack)?;

    project_from_catalogs(
        generation_id,
        stack,
        &block_catalog,
        &item_models,
        &item_cuboids,
        &sound_catalog,
        &particle_catalog,
        registry,
    )
}

#[allow(clippy::too_many_arguments)]
fn project_from_catalogs(
    generation: u64,
    stack: &PackResourceStack,
    block_catalog: &BlockModelCatalog,
    item_models: &ItemModelCatalog,
    item_cuboids: &ItemCuboidModelCatalog,
    sound_catalog: &SoundCatalog,
    particle_catalog: &ParticleDefinitionCatalog,
    registry: &ContentRegistry,
) -> Result<ContentRegistryAssetProjection> {
    let mut blocks = BTreeMap::new();
    for spec in registry.blocks() {
        let projected = project_block_spec(stack, block_catalog, spec);
        blocks.insert(projected.resource_id.clone(), projected);
    }

    let mut items = BTreeMap::new();
    for spec in registry.items() {
        let projected = project_item_spec(stack, block_catalog, item_models, item_cuboids, spec);
        items.insert(projected.resource_id.clone(), projected);
    }

    let mut sound_events = BTreeMap::new();
    for spec in registry.sound_events() {
        let projected = project_sound_spec(sound_catalog, spec);
        sound_events.insert(projected.resource_id.clone(), projected);
    }

    let mut particles = BTreeMap::new();
    for spec in registry.particles() {
        let projected = project_particle_spec(stack, particle_catalog, spec);
        particles.insert(projected.resource_id.clone(), projected);
    }

    Ok(ContentRegistryAssetProjection {
        generation,
        blocks,
        items,
        sound_events,
        particles,
        sound_event_registry: SoundEventRegistry::with_content_registry(registry),
    })
}

pub(crate) fn project_single_mod_block(
    generation: &ClientResourceGeneration,
    spec: &ModBlockSpec,
) -> Result<ModBlockAssetProjection> {
    let stack = generation.resources();
    let cuboid_models = generation.cuboid_models()?;
    let block_catalog = BlockModelCatalog::load_with_models(stack, cuboid_models)?;
    Ok(project_block_spec(stack, &block_catalog, spec))
}

pub(crate) fn project_single_mod_item(
    generation: &ClientResourceGeneration,
    spec: &ModItemSpec,
) -> Result<ModItemAssetProjection> {
    let stack = generation.resources();
    let cuboid_models = generation.cuboid_models()?;
    let block_catalog = BlockModelCatalog::load_with_models(stack, cuboid_models.clone())?;
    let item_models = ItemModelCatalog::load_resource_stack(stack)?;
    let item_cuboids = ItemCuboidModelCatalog::from_models(cuboid_models);
    Ok(project_item_spec(
        stack,
        &block_catalog,
        &item_models,
        &item_cuboids,
        spec,
    ))
}

pub(crate) fn project_single_mod_sound_event(
    generation: &ClientResourceGeneration,
    spec: &ModSoundEventSpec,
) -> Result<ModSoundEventAssetProjection> {
    let sound_catalog = SoundCatalog::load_resource_stack(generation.resources())?;
    Ok(project_sound_spec(&sound_catalog, spec))
}

pub(crate) fn project_single_mod_particle(
    generation: &ClientResourceGeneration,
    spec: &ModParticleSpec,
) -> Result<ModParticleAssetProjection> {
    let stack = generation.resources();
    let particle_catalog = ParticleDefinitionCatalog::load_resource_stack(stack)?;
    Ok(project_particle_spec(stack, &particle_catalog, spec))
}

fn project_block_spec(
    stack: &PackResourceStack,
    block_catalog: &BlockModelCatalog,
    spec: &ModBlockSpec,
) -> ModBlockAssetProjection {
    let resource_id = spec.resource_id().to_string();
    let fallback_tint = deterministic_tint(&resource_id, spec.dynamic_light_rgb);

    if let Some((kind, render_model)) = resolve_pack_block_model(block_catalog, &resource_id) {
        let texture_ids = collect_block_textures(&render_model);
        return ModBlockAssetProjection {
            block_id: spec.block_id,
            first_state_id: spec.first_state_id,
            state_count: spec.state_count,
            resource_id,
            kind,
            render_model,
            texture_ids,
            fallback_tint,
            resolved_from_pack: true,
        };
    }

    let texture = resolve_pack_texture_or_missing(stack, &resource_id, "block");
    let render_model = fallback_cube_render_model(&texture);
    ModBlockAssetProjection {
        block_id: spec.block_id,
        first_state_id: spec.first_state_id,
        state_count: spec.state_count,
        resource_id,
        kind: ModAssetModelKind::FallbackCube,
        render_model,
        texture_ids: vec![texture],
        fallback_tint,
        resolved_from_pack: false,
    }
}

fn project_item_spec(
    stack: &PackResourceStack,
    block_catalog: &BlockModelCatalog,
    item_models: &ItemModelCatalog,
    item_cuboids: &ItemCuboidModelCatalog,
    spec: &ModItemSpec,
) -> ModItemAssetProjection {
    let resource_id = spec.resource_id().to_string();
    let fallback_tint = deterministic_tint(&resource_id, None);

    if let Some(set) = item_cuboids.models_for_item(item_models, &resource_id) {
        if let Some(model) = set.models.into_iter().next() {
            let texture_ids = model.texture_ids();
            let generated = model.generated;
            return ModItemAssetProjection {
                item_id: spec.id,
                resource_id,
                is_block_item: spec.is_block_item,
                kind: ModAssetModelKind::PackItemDefinition,
                generated,
                texture_ids,
                model: Some(model),
                fallback_block_model: None,
                fallback_tint,
                resolved_from_pack: true,
            };
        }
    }

    if let Some(direct_item_model_id) = prefixed_model_id(&resource_id, "item") {
        if let Some(model) = item_cuboids.model(&direct_item_model_id) {
            let texture_ids = model.texture_ids();
            let generated = model.generated;
            return ModItemAssetProjection {
                item_id: spec.id,
                resource_id,
                is_block_item: spec.is_block_item,
                kind: ModAssetModelKind::PackItemModel,
                generated,
                texture_ids,
                model: Some(model),
                fallback_block_model: None,
                fallback_tint,
                resolved_from_pack: true,
            };
        }
    }

    if spec.is_block_item {
        if let Some((kind, render_model)) = resolve_pack_block_model(block_catalog, &resource_id) {
            let texture_ids = collect_block_textures(&render_model);
            return ModItemAssetProjection {
                item_id: spec.id,
                resource_id,
                is_block_item: true,
                kind,
                generated: false,
                texture_ids,
                model: None,
                fallback_block_model: Some(render_model),
                fallback_tint,
                resolved_from_pack: true,
            };
        }

        let texture = resolve_pack_texture_or_missing(stack, &resource_id, "block");
        return ModItemAssetProjection {
            item_id: spec.id,
            resource_id,
            is_block_item: true,
            kind: ModAssetModelKind::FallbackCube,
            generated: false,
            texture_ids: vec![texture.clone()],
            model: None,
            fallback_block_model: Some(fallback_cube_render_model(&texture)),
            fallback_tint,
            resolved_from_pack: false,
        };
    }

    let texture = resolve_pack_texture_or_missing(stack, &resource_id, "item");
    let model = fallback_flat_item_model(&resource_id, &texture);
    ModItemAssetProjection {
        item_id: spec.id,
        resource_id,
        is_block_item: false,
        kind: ModAssetModelKind::FallbackFlat,
        generated: true,
        texture_ids: vec![texture],
        model: Some(model),
        fallback_block_model: None,
        fallback_tint,
        resolved_from_pack: false,
    }
}

fn project_sound_spec(
    sound_catalog: &SoundCatalog,
    spec: &ModSoundEventSpec,
) -> ModSoundEventAssetProjection {
    let resource_id = spec.resource_id().to_string();
    let definition = sound_catalog.event(&resource_id).cloned();
    ModSoundEventAssetProjection {
        sound_event_id: spec.id,
        resource_id,
        resolved_from_pack: definition.is_some(),
        definition,
    }
}

fn project_particle_spec(
    stack: &PackResourceStack,
    particle_catalog: &ParticleDefinitionCatalog,
    spec: &ModParticleSpec,
) -> ModParticleAssetProjection {
    let resource_id = spec.resource_id().to_string();
    let fallback_tint = deterministic_tint(&resource_id, None);
    if let Some(definition) = particle_catalog.definition(&resource_id) {
        return ModParticleAssetProjection {
            particle_id: spec.id,
            resource_id,
            override_limiter: spec.override_limiter,
            textures: definition.textures.clone(),
            fallback_tint,
            resolved_from_pack: true,
        };
    }

    let fallback_texture = resolve_pack_particle_texture_or_missing(stack, &resource_id);
    ModParticleAssetProjection {
        particle_id: spec.id,
        resource_id,
        override_limiter: spec.override_limiter,
        textures: vec![fallback_texture],
        fallback_tint,
        resolved_from_pack: false,
    }
}

fn resolve_pack_block_model(
    block_catalog: &BlockModelCatalog,
    resource_id: &str,
) -> Option<(ModAssetModelKind, BlockRenderModel)> {
    let empty_props = BTreeMap::new();
    if let Some(model) = block_catalog.block_render_model(resource_id, &empty_props) {
        return Some((ModAssetModelKind::PackBlockstate, model));
    }
    let block_model_id = prefixed_model_id(resource_id, "block")?;
    block_catalog
        .resolve_direct_block_model(&block_model_id)
        .map(|model| (ModAssetModelKind::PackBlockModel, model))
}

fn prefixed_model_id(resource_id: &str, folder: &str) -> Option<String> {
    let loc = ResourceLocation::parse(resource_id).ok()?;
    Some(format!("{}:{folder}/{}", loc.namespace(), loc.path()))
}

fn resolve_pack_texture_or_missing(
    stack: &PackResourceStack,
    resource_id: &str,
    folder: &str,
) -> String {
    if let Ok(loc) = ResourceLocation::parse(resource_id) {
        let texture_path = format!("textures/{folder}/{}.png", loc.path());
        if let Ok(tex_loc) = ResourceLocation::new(loc.namespace(), texture_path) {
            if stack.get_resource(&tex_loc).is_some() {
                return format!("{}:{folder}/{}", loc.namespace(), loc.path());
            }
        }
    }
    MISSING_TEXTURE_ID.to_string()
}

fn resolve_pack_particle_texture_or_missing(
    stack: &PackResourceStack,
    resource_id: &str,
) -> String {
    if let Ok(loc) = ResourceLocation::parse(resource_id) {
        let texture_path = format!("textures/particle/{}.png", loc.path());
        if let Ok(tex_loc) = ResourceLocation::new(loc.namespace(), texture_path) {
            if stack.get_resource(&tex_loc).is_some() {
                return loc.id();
            }
        }
    }
    MISSING_TEXTURE_ID.to_string()
}

fn fallback_cube_render_model(texture_id: &str) -> BlockRenderModel {
    BlockRenderModel {
        face_textures: BlockFaceTextures::uniform(texture_id),
        particle_texture: Some(texture_id.to_string()),
        shape: BlockModelShape::Cube,
        use_ambient_occlusion: true,
        gui_light: BlockModelGuiLight::Side,
        display_transforms: BlockModelDisplayTransforms::default(),
    }
}

fn fallback_flat_item_model(item_id: &str, texture_id: &str) -> ItemCuboidModel {
    let mut texture_slots = BTreeMap::new();
    texture_slots.insert(
        "layer0".to_string(),
        ItemCuboidTexture {
            id: texture_id.to_string(),
            force_translucent: false,
        },
    );
    let model_id = prefixed_model_id(item_id, "item").unwrap_or_else(|| item_id.to_string());
    ItemCuboidModel {
        generated: true,
        id: model_id,
        parent: Some("minecraft:item/generated".to_string()),
        use_ambient_occlusion: true,
        gui_light: BlockModelGuiLight::Front,
        display_transforms: BlockModelDisplayTransforms::default(),
        texture_slots,
        face_textures: None,
        shape: BlockModelShape::default(),
    }
}

fn collect_block_textures(model: &BlockRenderModel) -> Vec<String> {
    let mut seen = BTreeSet::new();
    for texture in &model.face_textures.textures {
        seen.insert(texture.clone());
    }
    if let Some(particle) = &model.particle_texture {
        seen.insert(particle.clone());
    }
    seen.into_iter().collect()
}

fn deterministic_tint(resource_id: &str, explicit_rgb: Option<[f32; 3]>) -> ModAssetTint {
    let rgb = match explicit_rgb {
        Some([r, g, b]) => [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)],
        None => {
            // FNV-1a 64-bit hash gives identical deterministic fallback colors across platforms.
            let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
            for byte in resource_id.as_bytes() {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
            let r = 0.35 + ((hash & 0xff) as f32 / 255.0) * 0.60;
            let g = 0.35 + (((hash >> 8) & 0xff) as f32 / 255.0) * 0.60;
            let b = 0.35 + (((hash >> 16) & 0xff) as f32 / 255.0) * 0.60;
            [r, g, b]
        }
    };
    let r_u8 = (rgb[0] * 255.0).round() as u32;
    let g_u8 = (rgb[1] * 255.0).round() as u32;
    let b_u8 = (rgb[2] * 255.0).round() as u32;
    ModAssetTint {
        rgb,
        argb: 0xff00_0000 | (r_u8 << 16) | (g_u8 << 8) | b_u8,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use bbb_protocol::content_registry::{
        ContentRegistryBuilder, ModBlockRegistration, ModItemRegistration, ModParticleRegistration,
    };

    use super::*;
    use crate::{BlockModelFace, PackRoots, SelectedClientPackStack, MC_VERSION};

    #[test]
    fn resolves_pack_assets_and_deterministic_fallbacks_for_content_registry() {
        let root = unique_temp_dir("content-registry-projection");
        let vanilla = root.join("sources").join(MC_VERSION);
        let mod_pack = root.join("mod_pack");
        write_text(&vanilla.join("assets/minecraft/placeholder.txt"), "ok");
        write_text(
            &mod_pack.join("pack.mcmeta"),
            r#"{"pack":{"description":"mod test pack","min_format":[84,0],"max_format":[84,0]}}"#,
        );

        // Packed mod assets under `examplemod`: blockstate, item definition, sounds.json, particle.
        write_text(
            &mod_pack.join("assets/examplemod/blockstates/ruby_ore.json"),
            r#"{"variants":{"":{"model":"examplemod:block/ruby_ore"}}}"#,
        );
        write_text(
            &mod_pack.join("assets/examplemod/models/block/ruby_ore.json"),
            r#"{"parent":"minecraft:block/cube_all","textures":{"all":"examplemod:block/ruby_ore"}}"#,
        );
        write_text(
            &vanilla.join("assets/minecraft/models/block/cube_all.json"),
            r##"{"elements":[{"from":[0,0,0],"to":[16,16,16],"faces":{"down":{"texture":"#all"},"up":{"texture":"#all"},"north":{"texture":"#all"},"south":{"texture":"#all"},"west":{"texture":"#all"},"east":{"texture":"#all"}}}]}"##,
        );
        write_text(
            &mod_pack.join("assets/examplemod/items/ruby_wand.json"),
            r#"{"model":{"type":"minecraft:model","model":"examplemod:item/ruby_wand"}}"#,
        );
        write_text(
            &vanilla.join("assets/minecraft/models/item/generated.json"),
            r#"{"parent":"builtin/generated","gui_light":"front"}"#,
        );
        write_text(
            &mod_pack.join("assets/examplemod/models/item/ruby_wand.json"),
            r#"{"parent":"minecraft:item/generated","textures":{"layer0":"examplemod:item/ruby_wand"}}"#,
        );
        write_text(
            &mod_pack.join("assets/examplemod/sounds.json"),
            r#"{"crystal.chime":{"sounds":["examplemod:crystal/chime"]}}"#,
        );
        write_text(
            &mod_pack.join("assets/examplemod/sounds/crystal/chime.ogg"),
            "OggS",
        );
        write_text(
            &mod_pack.join("assets/examplemod/particles/sparkle.json"),
            r#"{"textures":["examplemod:sparkle_0","examplemod:sparkle_1"]}"#,
        );

        let mut builder = ContentRegistryBuilder::new();
        builder.register_block(ModBlockRegistration {
            resource_id: "examplemod:ruby_ore".to_string(),
            state_count: 1,
            light_emission: 0,
            destroy_speed: 3.0,
            full_cube_collision: true,
            dynamic_light_rgb: Some([0.8, 0.2, 0.3]),
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "examplemod:unpacked_block".to_string(),
            state_count: 1,
            light_emission: 7,
            destroy_speed: 1.5,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        builder.register_item(ModItemRegistration {
            resource_id: "examplemod:ruby_wand".to_string(),
            max_stack_size: 1,
            is_block_item: false,
        });
        builder.register_item(ModItemRegistration {
            resource_id: "examplemod:unpacked_item".to_string(),
            max_stack_size: 64,
            is_block_item: false,
        });
        builder.register_item(ModItemRegistration {
            resource_id: "examplemod:unpacked_block".to_string(),
            max_stack_size: 64,
            is_block_item: true,
        });
        builder.register_sound_event("examplemod:crystal.chime");
        builder.register_sound_event("examplemod:missing.bell");
        builder.register_particle(ModParticleRegistration {
            resource_id: "examplemod:sparkle".to_string(),
            override_limiter: false,
        });
        builder.register_particle(ModParticleRegistration {
            resource_id: "examplemod:missing_ember".to_string(),
            override_limiter: true,
        });
        let registry = builder.freeze().unwrap();

        let base_roots = PackRoots::from_root(&root).unwrap();
        let selected = SelectedClientPackStack::from_startup_dirs(&base_roots, [mod_pack]).unwrap();
        let roots = base_roots
            .with_client_resource_generation(11, selected)
            .unwrap();
        let generation = roots.client_resource_generation().unwrap();

        let projection = generation.project_content_registry(&registry).unwrap();
        assert_eq!(projection.generation, 11);

        let ruby_ore = projection.block("examplemod:ruby_ore").unwrap();
        assert!(ruby_ore.resolved_from_pack);
        assert_eq!(ruby_ore.kind, ModAssetModelKind::PackBlockstate);
        assert_eq!(
            ruby_ore.render_model.face_textures.get(BlockModelFace::Up),
            "examplemod:block/ruby_ore"
        );

        let missing_block = projection.block("examplemod:unpacked_block").unwrap();
        assert!(!missing_block.resolved_from_pack);
        assert_eq!(missing_block.kind, ModAssetModelKind::FallbackCube);
        assert_eq!(missing_block.render_model.shape, BlockModelShape::Cube);
        assert_eq!(
            missing_block
                .render_model
                .face_textures
                .get(BlockModelFace::North),
            MISSING_TEXTURE_ID
        );

        let ruby_wand = projection.item("examplemod:ruby_wand").unwrap();
        assert!(ruby_wand.resolved_from_pack);
        assert_eq!(ruby_wand.kind, ModAssetModelKind::PackItemDefinition);
        assert!(ruby_wand.generated);
        assert_eq!(
            ruby_wand.texture_ids,
            vec!["examplemod:item/ruby_wand".to_string()]
        );

        let missing_item = projection.item("examplemod:unpacked_item").unwrap();
        assert!(!missing_item.resolved_from_pack);
        assert_eq!(missing_item.kind, ModAssetModelKind::FallbackFlat);
        assert!(missing_item.generated);
        assert_eq!(
            missing_item.texture_ids,
            vec![MISSING_TEXTURE_ID.to_string()]
        );
        assert_eq!(
            missing_item
                .model
                .as_ref()
                .unwrap()
                .texture_slots
                .get("layer0")
                .map(|slot| slot.id.as_str()),
            Some(MISSING_TEXTURE_ID)
        );

        let missing_block_item = projection.item("examplemod:unpacked_block").unwrap();
        assert!(!missing_block_item.resolved_from_pack);
        assert_eq!(missing_block_item.kind, ModAssetModelKind::FallbackCube);
        assert!(!missing_block_item.generated);
        assert_eq!(
            missing_block_item
                .fallback_block_model
                .as_ref()
                .unwrap()
                .shape,
            BlockModelShape::Cube
        );

        let chime = projection.sound_event("examplemod:crystal.chime").unwrap();
        assert!(chime.resolved_from_pack);
        assert_eq!(
            chime.definition.as_ref().unwrap().sounds[0].name,
            "examplemod:crystal/chime"
        );
        let missing_sound = projection.sound_event("examplemod:missing.bell").unwrap();
        assert!(!missing_sound.resolved_from_pack);
        assert!(missing_sound.definition.is_none());
        assert_eq!(
            projection
                .sound_event_registry()
                .event_id(chime.sound_event_id),
            Some("examplemod:crystal.chime")
        );

        let sparkle = projection.particle("examplemod:sparkle").unwrap();
        assert!(sparkle.resolved_from_pack);
        assert_eq!(
            sparkle.textures,
            vec![
                "examplemod:sparkle_0".to_string(),
                "examplemod:sparkle_1".to_string()
            ]
        );
        let missing_particle = projection.particle("examplemod:missing_ember").unwrap();
        assert!(!missing_particle.resolved_from_pack);
        assert_eq!(
            missing_particle.textures,
            vec![MISSING_TEXTURE_ID.to_string()]
        );

        // Single-spec helpers match batch projection cleanly.
        assert_eq!(
            generation
                .project_mod_block(registry.block_by_id(missing_block.block_id).unwrap())
                .unwrap(),
            *missing_block
        );
        assert_eq!(
            generation
                .project_mod_item(registry.item_by_id(missing_item.item_id).unwrap())
                .unwrap(),
            *missing_item
        );

        fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-client-assets-{label}-{nanos}"))
    }

    fn write_text(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
}
