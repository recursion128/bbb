use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct Filter {
    uniform: Option<bool>,
    jp: Option<bool>,
}

impl Filter {
    // A referencing provider overrides the referenced provider’s options.
    pub(super) fn merge(self, inner: Self) -> Self {
        Self {
            uniform: self.uniform.or(inner.uniform),
            jp: self.jp.or(inner.jp),
        }
    }

    pub(super) fn enabled(self, options: FontOptions) -> bool {
        self.uniform.is_none_or(|value| value == options.uniform)
            && self
                .jp
                .is_none_or(|value| value == options.japanese_variants)
    }
}

#[derive(Clone)]
pub(super) struct Conditional {
    pub(super) provider: Definition,
    pub(super) filter: Filter,
}

#[derive(Clone)]
pub(super) enum Definition {
    Raster(std::sync::Arc<FontProviderDefinition>),
    Reference(String),
}

pub(super) fn decode(font_id: &str, root: &Value) -> Result<Vec<Conditional>> {
    root.get("providers")
        .and_then(Value::as_array)
        .with_context(|| format!("font definition {font_id} has no \"providers\" array"))?
        .iter()
        .map(|value| {
            let mut filter = Filter::default();
            if let Some(value) = value.get("filter") {
                for (name, value) in value.as_object().context("font filter must be an object")? {
                    let expected = value.as_bool().context("font option must be boolean")?;
                    match name.as_str() {
                        Some("uniform") => filter.uniform = Some(expected),
                        Some("jp") => filter.jp = Some(expected),
                        _ => bail!("unknown font option {name:?}"),
                    }
                }
            }
            let provider = match value
                .get("type")
                .and_then(Value::as_str)
                .context("font provider has no type")?
            {
                "bitmap" => Definition::Raster(std::sync::Arc::new(
                    FontProviderDefinition::Bitmap(parse_bitmap_provider(font_id, value)?),
                )),
                "space" => Definition::Raster(std::sync::Arc::new(FontProviderDefinition::Space(
                    parse_space_provider(font_id, value)?,
                ))),
                "unihex" => Definition::Raster(std::sync::Arc::new(
                    FontProviderDefinition::Unihex(parse_unihex_provider(font_id, value)?),
                )),
                "reference" => Definition::Reference(resource_id(value, "id")?),
                "ttf" => Definition::Raster(std::sync::Arc::new(FontProviderDefinition::TrueType(
                    parse_ttf_provider(font_id, value)?,
                ))),
                unknown => bail!("unknown font provider type {unknown:?}"),
            };
            Ok(Conditional { provider, filter })
        })
        .collect()
}

fn resource_id(value: &Value, field: &str) -> Result<String> {
    let id = value
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("font provider requires {field}"))?;
    Ok(ResourceLocation::parse(id)?.id())
}
