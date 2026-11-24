use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use bbb_render_types::{
    CoreShaderSourceProjection, ShaderSource, ShaderSourceImport, ShaderSourceKey,
    ShaderSourceLoadFailure, ShaderStage,
};
use regex::Regex;

use crate::{
    resources::{PackResource, PackResourceStack, ResourceLocation},
    roots::PackRoots,
};

const MAX_SHADER_SOURCE_RESOURCES: u64 = 512;
const MAX_SHADER_IMPORT_DEPTH: u32 = 32;
const RETAINED_SHADER_SOURCE_BYTES: u64 = 256;

/// Immutable CPU preparation of Vanilla `ShaderManager.Configs.shaderSources`.
///
/// The catalog enumerates every selected `.vsh`/`.fsh` exactly once and expands
/// `#moj_import` through the same pack stack. GPU compilation and static-pipeline
/// selection remain renderer-owned.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShaderSourceCatalog {
    generation: u64,
    sources: Arc<BTreeMap<ShaderSourceKey, ShaderSource>>,
    load_failures: Arc<Vec<ShaderSourceLoadFailure>>,
    failed_keys: Arc<BTreeMap<ShaderSourceKey, String>>,
}

impl ShaderSourceCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        let generation = roots
            .client_resource_generation()
            .map_or(0, |generation| generation.generation());
        Self::load_resource_stack(generation, &roots.resource_stack())
    }

    pub fn load_resource_stack(generation: u64, stack: &PackResourceStack) -> Result<Self> {
        let mut resources = stack.list_resources("shaders", ".vsh")?;
        resources.extend(stack.list_resources("shaders", ".fsh")?);
        resources.sort_by(|left, right| left.location.cmp(&right.location));
        stack.admit_resource_collection_count(
            u64::try_from(resources.len()).context("shader source count overflow")?,
            MAX_SHADER_SOURCE_RESOURCES,
            "shader stage sources",
        )?;

        let retained_bytes = u64::try_from(resources.len())
            .context("retained shader source count overflow")?
            .checked_mul(RETAINED_SHADER_SOURCE_BYTES)
            .context("retained shader source byte count overflow")?;
        stack.reserve_derived_resource_bytes(retained_bytes, "retained shader source catalog")?;

        let mut sources = BTreeMap::new();
        let mut load_failures = Vec::new();
        let mut failed_keys = BTreeMap::new();
        let mut admitted_resources = BTreeSet::new();
        for resource in resources {
            let key = shader_key_from_resource(&resource.location)?;
            let resource_id = resource.location.id();
            let pack_id = resource.pack_id.clone();
            let prepared = load_top_level_source(stack, &resource).and_then(|source| {
                let prepared = preprocess_shader_source(
                    stack,
                    &resource.location,
                    &source,
                    &mut admitted_resources,
                )
                .with_context(|| format!("preprocess shader source {resource_id}"))?;
                stack.reserve_derived_resource_bytes(
                    u64::try_from(prepared.source.len())
                        .context("preprocessed shader length overflow")?,
                    "preprocessed shader source",
                )?;
                Ok(prepared)
            });
            match prepared {
                Ok(prepared) => {
                    sources.insert(
                        key.clone(),
                        ShaderSource {
                            key,
                            resource_id,
                            pack_id,
                            source: prepared.source,
                            imports: prepared.imports,
                        },
                    );
                }
                Err(error) => {
                    // Propagate generation budget/collection rejections rather than deferring them.
                    stack.ensure_resource_data_admitted()?;
                    let formatted = format!("{error:#}");
                    failed_keys.insert(key, formatted.clone());
                    load_failures.push(ShaderSourceLoadFailure {
                        resource_id,
                        pack_id,
                        error: formatted,
                    });
                }
            }
        }
        stack.ensure_resource_data_admitted()?;
        Ok(Self {
            generation,
            sources: Arc::new(sources),
            load_failures: Arc::new(load_failures),
            failed_keys: Arc::new(failed_keys),
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn source(&self, key: &ShaderSourceKey) -> Option<&ShaderSource> {
        self.sources.get(key)
    }

    pub fn source_by_id(&self, id: &str, stage: ShaderStage) -> Option<&ShaderSource> {
        let id = ResourceLocation::parse(id).ok()?.id();
        self.source(&ShaderSourceKey { id, stage })
    }

    pub fn sources(&self) -> &BTreeMap<ShaderSourceKey, ShaderSource> {
        &self.sources
    }

    pub fn renderer_projection(&self) -> CoreShaderSourceProjection {
        CoreShaderSourceProjection::new(self.generation, Arc::clone(&self.sources))
    }

    pub fn load_failures(&self) -> &[ShaderSourceLoadFailure] {
        &self.load_failures
    }

    pub fn failure_for_key(&self, key: &ShaderSourceKey) -> Option<&str> {
        self.failed_keys.get(key).map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

fn load_top_level_source(stack: &PackResourceStack, resource: &PackResource) -> Result<String> {
    stack
        .read_text_resource(resource, "shader source")
        .with_context(|| format!("read shader source {}", resource.location.id()))
}

struct PreprocessedShaderSource {
    source: String,
    imports: Vec<ShaderSourceImport>,
}

fn preprocess_shader_source(
    stack: &PackResourceStack,
    source_location: &ResourceLocation,
    source: &str,
    admitted_resources: &mut BTreeSet<String>,
) -> Result<PreprocessedShaderSource> {
    admit_shader_resource(stack, admitted_resources, source_location)?;
    let mut imports = Vec::new();
    let mut imported_locations = BTreeSet::new();
    let mut maximum_version = shader_version(source).unwrap_or(0);
    let source = expand_shader_imports(
        stack,
        source_location,
        source,
        false,
        0,
        &mut maximum_version,
        &mut imported_locations,
        admitted_resources,
        &mut imports,
    )?;
    Ok(PreprocessedShaderSource {
        source: set_shader_version(&source, maximum_version),
        imports,
    })
}

#[allow(clippy::too_many_arguments)]
fn expand_shader_imports(
    stack: &PackResourceStack,
    source_location: &ResourceLocation,
    source: &str,
    imported_source: bool,
    depth: u32,
    maximum_version: &mut u32,
    imported_locations: &mut BTreeSet<String>,
    admitted_resources: &mut BTreeSet<String>,
    imports: &mut Vec<ShaderSourceImport>,
) -> Result<String> {
    if depth > MAX_SHADER_IMPORT_DEPTH {
        bail!(
            "shader import depth exceeds {MAX_SHADER_IMPORT_DEPTH} at {}",
            source_location.id()
        );
    }
    if let Some(version) = shader_version(source) {
        *maximum_version = (*maximum_version).max(version);
    }
    let source = if imported_source {
        comment_shader_versions(source)
    } else {
        source.to_string()
    };
    let pattern = shader_import_pattern();
    let mut comments = ShaderCommentCursor::new(&source);
    let mut expanded = String::with_capacity(source.len());
    let mut cursor = 0;
    for captures in pattern.captures_iter(&source) {
        let whole = captures
            .get(0)
            .expect("shader import regex always has a whole match");
        if comments.is_inside_comment_at(whole.start()) {
            continue;
        }
        expanded.push_str(&source[cursor..whole.start()]);
        let (relative, path) = if let Some(path) = captures.get(1) {
            (true, path.as_str())
        } else {
            (
                false,
                captures
                    .get(2)
                    .expect("shader import regex has one path")
                    .as_str(),
            )
        };
        let import_location = resolve_shader_import(source_location, relative, path)
            .with_context(|| format!("resolve shader import {path:?}"))?;
        let import_id = import_location.id();
        if !imported_locations.insert(import_id.clone()) {
            expanded.push_str(&format!("/*{}*/", whole.as_str()));
            cursor = whole.end();
            continue;
        }
        admit_shader_resource(stack, admitted_resources, &import_location)?;
        let resource = stack
            .get_resource(&import_location)
            .with_context(|| format!("missing shader import {}", import_location.id()))?;
        let imported = stack
            .read_text_resource(&resource, "shader import")
            .with_context(|| format!("read shader import {}", import_location.id()))?;
        imports.push(ShaderSourceImport {
            resource_id: import_id,
            pack_id: resource.pack_id,
        });
        let imported = expand_shader_imports(
            stack,
            &import_location,
            &imported,
            true,
            depth + 1,
            maximum_version,
            imported_locations,
            admitted_resources,
            imports,
        )?;
        expanded.push_str(&imported);
        if !imported.ends_with('\n') {
            expanded.push('\n');
        }
        cursor = whole.end();
    }
    expanded.push_str(&source[cursor..]);
    Ok(expanded)
}

fn admit_shader_resource(
    stack: &PackResourceStack,
    admitted_resources: &mut BTreeSet<String>,
    location: &ResourceLocation,
) -> Result<()> {
    admitted_resources.insert(location.id());
    stack.admit_resource_collection_count(
        u64::try_from(admitted_resources.len()).context("shader resource count overflow")?,
        MAX_SHADER_SOURCE_RESOURCES,
        "shader stage and include sources",
    )
}

fn shader_key_from_resource(location: &ResourceLocation) -> Result<ShaderSourceKey> {
    let (path, stage) = if let Some(path) = location.path().strip_suffix(".vsh") {
        (path, ShaderStage::Vertex)
    } else if let Some(path) = location.path().strip_suffix(".fsh") {
        (path, ShaderStage::Fragment)
    } else {
        bail!("shader resource {} has no stage extension", location.id());
    };
    let path = path.strip_prefix("shaders/").with_context(|| {
        format!(
            "shader resource {} is outside the shaders directory",
            location.id()
        )
    })?;
    Ok(ShaderSourceKey {
        id: ResourceLocation::new(location.namespace().to_string(), path.to_string())?.id(),
        stage,
    })
}

fn resolve_shader_import(
    source: &ResourceLocation,
    relative: bool,
    import: &str,
) -> Result<ResourceLocation> {
    if import.is_empty() || import.contains('\\') {
        bail!("invalid shader import path {import:?}");
    }
    if relative {
        if import.split('/').any(|component| component == "..") {
            bail!("shader import path traversal is not allowed: {import:?}");
        }
        let parent = source
            .path()
            .rsplit_once('/')
            .map_or("", |(parent, _)| parent);
        let path = if parent.is_empty() {
            import.to_string()
        } else {
            format!("{parent}/{import}")
        };
        ResourceLocation::new(source.namespace().to_string(), path)
    } else {
        let import = ResourceLocation::parse(import)?;
        ResourceLocation::new(
            import.namespace().to_string(),
            format!("shaders/include/{}", import.path()),
        )
    }
}

fn shader_import_pattern() -> &'static Regex {
    static PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r#"(?m)^(?:[ \t]|/\*[^\r\n]*?\*/)*#(?:[ \t]|/\*[^\r\n]*?\*/)*moj_import(?:[ \t]|/\*[^\r\n]*?\*/)*(?:\"([^\"\r\n]+)\"|<([^>\r\n]+)>)(?:[ \t]|/\*[^\r\n]*?\*/)*(?://[^\r\n]*)?\r?$"#,
        )
        .expect("shader import regex is valid")
    })
}

fn shader_version_pattern() -> &'static Regex {
    static PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"(?m)^(?:[ \t]|/\*[^\r\n]*?\*/)*#(?:[ \t]|/\*[^\r\n]*?\*/)*version(?:[ \t]|/\*[^\r\n]*?\*/)*([0-9]+)\b",
        )
        .expect("shader version regex is valid")
    })
}

fn shader_version(source: &str) -> Option<u32> {
    let mut comments = ShaderCommentCursor::new(source);
    shader_version_pattern()
        .captures_iter(source)
        .find(|captures| {
            captures
                .get(0)
                .is_some_and(|whole| !comments.is_inside_comment_at(whole.start()))
        })
        .and_then(|captures| captures.get(1)?.as_str().parse().ok())
}

fn comment_shader_versions(source: &str) -> String {
    let pattern = shader_version_pattern();
    let mut comments = ShaderCommentCursor::new(source);
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;
    for captures in pattern.captures_iter(source) {
        let whole = captures
            .get(0)
            .expect("shader version regex always has a whole match");
        if comments.is_inside_comment_at(whole.start()) {
            continue;
        }
        result.push_str(&source[cursor..whole.start()]);
        result.push_str("/*");
        result.push_str(whole.as_str());
        result.push_str("*/");
        cursor = whole.end();
    }
    result.push_str(&source[cursor..]);
    result
}

fn set_shader_version(source: &str, version: u32) -> String {
    let pattern = shader_version_pattern();
    let mut comments = ShaderCommentCursor::new(source);
    for captures in pattern.captures_iter(source) {
        let whole = captures
            .get(0)
            .expect("shader version regex always has a whole match");
        if comments.is_inside_comment_at(whole.start()) {
            continue;
        }
        let digits = captures
            .get(1)
            .expect("shader version regex always has digits");
        let Ok(current) = digits.as_str().parse::<u32>() else {
            return source.to_string();
        };
        if version <= current {
            return source.to_string();
        }
        let mut result = String::with_capacity(source.len());
        result.push_str(&source[..digits.start()]);
        result.push_str(&version.to_string());
        result.push_str(&source[digits.end()..]);
        return result;
    }
    source.to_string()
}

/// Forward single-pass GLSL comment/string state tracker so directive checks
/// stay O(L) across monotonically increasing regex match offsets.
struct ShaderCommentCursor<'a> {
    bytes: &'a [u8],
    index: usize,
    block_comment: bool,
    line_comment: bool,
    quoted: Option<u8>,
}

impl<'a> ShaderCommentCursor<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            bytes: source.as_bytes(),
            index: 0,
            block_comment: false,
            line_comment: false,
            quoted: None,
        }
    }

    fn is_inside_comment_at(&mut self, offset: usize) -> bool {
        debug_assert!(offset >= self.index);
        let limit = offset.min(self.bytes.len());
        while self.index < limit {
            let byte = self.bytes[self.index];
            let next = self.bytes.get(self.index + 1).copied();
            if self.block_comment {
                if byte == b'*' && next == Some(b'/') {
                    if self.index + 2 <= limit {
                        self.block_comment = false;
                        self.index += 2;
                        continue;
                    }
                    break;
                }
            } else if self.line_comment {
                if byte == b'\n' || byte == b'\r' {
                    self.line_comment = false;
                }
            } else if let Some(quote) = self.quoted {
                if byte == b'\\' {
                    if self.index + 2 <= limit {
                        self.index += 2;
                        continue;
                    }
                    break;
                }
                if byte == quote {
                    self.quoted = None;
                }
            } else if byte == b'/' && next == Some(b'*') {
                if self.index + 2 <= limit {
                    self.block_comment = true;
                    self.index += 2;
                    continue;
                }
                break;
            } else if byte == b'/' && next == Some(b'/') {
                if self.index + 2 <= limit {
                    self.line_comment = true;
                    self.index += 2;
                    continue;
                }
                break;
            } else if byte == b'"' || byte == b'\'' {
                self.quoted = Some(byte);
            }
            self.index += 1;
        }
        self.block_comment || self.line_comment || self.quoted.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn shader_catalog_uses_pack_precedence_and_preprocesses_imports_once() {
        let root = unique_temp_dir("shader-precedence");
        let vanilla = root.join("vanilla");
        let overlay = root.join("overlay");
        write_text(
            &asset(&vanilla, "demo", "shaders/core/example.vsh"),
            "#version 330\n#moj_import <demo:shared.glsl>\nvoid main() {}\n",
        );
        write_text(
            &asset(&overlay, "demo", "shaders/core/example.vsh"),
            "#version 330\n#moj_import <demo:shared.glsl>\nconst int OVERLAY = 1;\n",
        );
        write_text(
            &asset(&vanilla, "demo", "shaders/include/shared.glsl"),
            "#version 450\nconst int SHARED = 1;\n",
        );
        write_text(
            &asset(&overlay, "demo", "shaders/include/shared.glsl"),
            "#version 450\nconst int SHARED = 2;\n",
        );

        let roots = fixture_roots(&root, &vanilla, [overlay]);
        let catalog = ShaderSourceCatalog::load(&roots).unwrap();
        let source = catalog
            .source_by_id("demo:core/example", ShaderStage::Vertex)
            .unwrap();
        assert_eq!(source.pack_id, "unmanaged/1");
        assert!(source.source.starts_with("#version 450"));
        assert!(source.source.contains("OVERLAY = 1"));
        assert_eq!(source.source.matches("SHARED = 2").count(), 1);
        assert_eq!(
            source.imports,
            vec![ShaderSourceImport {
                resource_id: "demo:shaders/include/shared.glsl".to_string(),
                pack_id: "unmanaged/1".to_string(),
            }]
        );
        assert!(catalog.load_failures().is_empty());
        assert!(roots.resource_data_memory_observation().text_source_bytes > 0);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shader_catalog_filter_can_hide_a_lower_stage_source() {
        let root = unique_temp_dir("shader-filter");
        let vanilla = root.join("vanilla");
        let filter = root.join("filter");
        write_text(
            &asset(&vanilla, "demo", "shaders/core/example.fsh"),
            "#version 330\nvoid main() {}\n",
        );
        write_text(
            &filter.join("pack.mcmeta"),
            r#"{"filter":{"block":[{"namespace":"demo","path":"shaders/core/example\\.fsh"}]}}"#,
        );
        let catalog = ShaderSourceCatalog::load(&fixture_roots(&root, &vanilla, [filter])).unwrap();
        assert!(catalog
            .source_by_id("demo:core/example", ShaderStage::Fragment)
            .is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shader_catalog_defers_missing_and_escaping_imports_per_entry() {
        for (label, import, expected) in [
            ("missing", "<demo:missing.glsl>", "missing shader import"),
            ("traversal", "\"../escape.glsl\"", "path traversal"),
            (
                "backslash",
                "\"bad\\\\path.glsl\"",
                "invalid shader import path",
            ),
        ] {
            let root = unique_temp_dir(label);
            let vanilla = root.join("vanilla");
            write_text(
                &asset(&vanilla, "demo", "shaders/core/example.vsh"),
                &format!("#version 330\n#moj_import {import}\nvoid main() {{}}\n"),
            );
            let catalog = ShaderSourceCatalog::load(&fixture_roots(&root, &vanilla, [])).unwrap();
            let key = ShaderSourceKey {
                id: "demo:core/example".to_string(),
                stage: ShaderStage::Vertex,
            };
            assert!(catalog.source(&key).is_none(), "{label}");
            assert_eq!(catalog.load_failures().len(), 1, "{label}");
            assert!(
                catalog.load_failures()[0].error.contains(expected),
                "{label}: {}",
                catalog.load_failures()[0].error
            );
            assert!(
                catalog
                    .failure_for_key(&key)
                    .is_some_and(|err| err.contains(expected)),
                "{label}"
            );
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn shader_catalog_tolerates_broken_unreferenced_shader_and_keeps_valid_shaders() {
        let root = unique_temp_dir("shader-defer-broken-unreferenced");
        let vanilla = root.join("vanilla");
        write_text(
            &asset(&vanilla, "demo", "shaders/core/valid.vsh"),
            "#version 330\nvoid main() {}\n",
        );
        write_text(
            &asset(&vanilla, "demo", "shaders/core/unused_broken.fsh"),
            "#version 330\n#moj_import <demo:missing.glsl>\nvoid main() {}\n",
        );

        let catalog = ShaderSourceCatalog::load(&fixture_roots(&root, &vanilla, [])).unwrap();
        assert!(catalog
            .source_by_id("demo:core/valid", ShaderStage::Vertex)
            .is_some());
        let broken_key = ShaderSourceKey {
            id: "demo:core/unused_broken".to_string(),
            stage: ShaderStage::Fragment,
        };
        assert!(catalog.source(&broken_key).is_none());
        assert_eq!(catalog.load_failures().len(), 1);
        assert!(catalog
            .failure_for_key(&broken_key)
            .unwrap()
            .contains("missing shader import"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shader_catalog_carries_selected_resource_generation() {
        let root = unique_temp_dir("shader-generation");
        let vanilla = root.join("vanilla");
        write_text(
            &asset(&vanilla, "demo", "shaders/core/example.vsh"),
            "#version 330\nvoid main() {}\n",
        );
        let roots = fixture_roots(&root, &vanilla, []);
        let selected = crate::SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let roots = roots.with_client_resource_generation(47, selected).unwrap();
        let catalog = ShaderSourceCatalog::load(&roots).unwrap();
        assert_eq!(catalog.generation(), 47);
        assert!(catalog.is_empty());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shader_comment_cursor_handles_many_commented_directives_in_linear_time() {
        let root = unique_temp_dir("shader-comment-bomb");
        let vanilla = root.join("vanilla");
        let mut source = String::from("/*\n");
        for _ in 0..20_000 {
            source.push_str("#version 999\n#moj_import <demo:missing.glsl>\n");
        }
        source.push_str("*/\n#version 330\nvoid main() {}\n");
        write_text(
            &asset(&vanilla, "demo", "shaders/core/comment_bomb.vsh"),
            &source,
        );

        let roots = fixture_roots(&root, &vanilla, []);
        let catalog = ShaderSourceCatalog::load(&roots).unwrap();
        let loaded = catalog
            .source_by_id("demo:core/comment_bomb", ShaderStage::Vertex)
            .expect("shader with commented directives should load cleanly");
        assert!(loaded.source.ends_with("#version 330\nvoid main() {}\n"));
        assert!(loaded.imports.is_empty());

        // Also check comment-cursor 2-byte boundary handling (`/*`, `*/`, escaped quote).
        let snippet = "/*x*/\"\\\"\"";
        let mut cursor = ShaderCommentCursor::new(snippet);
        assert!(!cursor.is_inside_comment_at(1));
        assert!(cursor.is_inside_comment_at(2));
        assert!(cursor.is_inside_comment_at(4));
        assert!(!cursor.is_inside_comment_at(5));
        assert!(cursor.is_inside_comment_at(7));
        assert!(cursor.is_inside_comment_at(8));
        assert!(!cursor.is_inside_comment_at(9));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_shader_catalog() {
        let catalog = ShaderSourceCatalog::load(&PackRoots::from_environment().unwrap()).unwrap();
        assert_eq!(catalog.sources().len(), 79);
        assert!(catalog.load_failures().is_empty());
        assert!(catalog
            .source_by_id("minecraft:core/terrain", ShaderStage::Fragment)
            .unwrap()
            .source
            .contains("sampleRGSS"));
    }

    fn fixture_roots(
        root: &Path,
        vanilla: &Path,
        overlays: impl IntoIterator<Item = PathBuf>,
    ) -> PackRoots {
        let mut packs = vec![vanilla.to_path_buf()];
        packs.extend(overlays);
        PackRoots::from_paths(
            root.to_path_buf(),
            root.join("empty-vanilla"),
            root.join("empty-vanilla/assets/minecraft"),
        )
        .with_resource_pack_dirs(packs)
    }

    fn asset(root: &Path, namespace: &str, path: &str) -> PathBuf {
        root.join("assets").join(namespace).join(path)
    }

    fn write_text(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-client-assets-{label}-{nanos}"))
    }
}
