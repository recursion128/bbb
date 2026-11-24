use std::fmt;

use bbb_registry_codec::{RegistryElementValueRef as V, RegistryJsonValue as Value};
type Components = Vec<(bbb_protocol::JavaString, Value)>;

use crate::frozen_lookup::FrozenDatapackRegistryLookup;

use crate::loot::{
    LootCondition, LootDecodeError, LootFunction, NumberProvider as LootNumberProvider,
};

#[derive(Clone, Debug, PartialEq)]
pub struct TradeSet {
    pub trades: TradeHolderSet,
    pub amount: LootNumberProvider,
    pub allow_duplicates: bool,
    pub random_sequence: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TradeHolderSet {
    Tag(String),
    Items(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct VillagerTrade {
    pub wants: TradeCost,
    pub additional_wants: Option<TradeCost>,
    pub gives: TradeItemTemplate,
    pub max_uses: LootNumberProvider,
    pub reputation_discount: LootNumberProvider,
    pub xp: LootNumberProvider,
    pub merchant_predicate: Option<LootCondition>,
    pub given_item_modifiers: Vec<LootFunction>,
    pub double_trade_price_enchantments: Option<TradeHolderSet>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TradeCost {
    pub item_id: String,
    pub count: LootNumberProvider,
    pub components: Components,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TradeItemTemplate {
    pub item_id: String,
    pub count: i32,
    pub components: Components,
}

pub fn load_trade_catalog(
    generation: &crate::dynamic_registry::DatapackGeneration,
    lookup: &crate::frozen_lookup::FrozenDatapackRegistryLookup<'_>,
) -> Result<
    std::collections::BTreeMap<String, VillagerTrade>,
    crate::registry_data_loader::DatapackLoadError,
> {
    let mut trades = std::collections::BTreeMap::new();
    let mut errors = Vec::new();
    if let Some(registry) = generation.registry("minecraft:villager_trade") {
        for raw in &registry.elements {
            let decoded = match raw.decoded_loaded_source(usize::MAX) {
                Ok(decoded) => decoded,
                Err(error) => {
                    errors.push(crate::registry_data_loader::DatapackElementError {
                        registry_id: registry.registry_id.clone(),
                        element_id: Some(raw.id.clone()),
                        pack: raw.source_pack.clone(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            match parse_villager_trade_source_with_lookup(
                decoded.json().expect("trade source is JSON"),
                lookup,
            ) {
                Ok(trade) => {
                    trades.insert(raw.id.clone(), trade);
                }
                Err(error) => errors.push(crate::registry_data_loader::DatapackElementError {
                    registry_id: registry.registry_id.clone(),
                    element_id: Some(raw.id.clone()),
                    pack: raw.source_pack.clone(),
                    message: error.to_string(),
                }),
            }
        }
    }
    if errors.is_empty() {
        Ok(trades)
    } else {
        Err(crate::registry_data_loader::DatapackLoadError { errors })
    }
}

fn parse_villager_trade_source_with_lookup(
    source: &Value,
    lookup: &crate::frozen_lookup::FrozenDatapackRegistryLookup<'_>,
) -> DecodeResult<VillagerTrade> {
    let gives = source
        .get("gives")
        .ok_or_else(|| error("gives", "missing required field"))?;
    let template = bbb_registry_codec::item_stack_template_from_source_with_content_registry(
        V::Json(gives),
        bbb_registry_codec::RegistryElementSource::ServerDatapackJson,
        lookup,
        lookup.content_registry(),
    )
    .map_err(|failure| error("gives", failure.to_string()))?;
    let mut trade = parse_villager_trade_with_lookup(source, Some(lookup))?;
    trade.gives.item_id = bbb_protocol::item_types::vanilla_item_type(template.item_id)
        .map(|entry| entry.resource_id.to_string())
        .or_else(|| {
            lookup
                .content_registry()
                .item_by_id(template.item_id)
                .map(|spec| spec.resource_id.clone())
        })
        .ok_or_else(|| error("gives", "unknown item"))?;
    trade.gives.count = template.count;
    Ok(trade)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeDecodeError {
    pub path: String,
    pub message: String,
}

impl fmt::Display for TradeDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for TradeDecodeError {}

type DecodeResult<T> = Result<T, TradeDecodeError>;

fn error(path: &str, message: impl Into<String>) -> TradeDecodeError {
    TradeDecodeError {
        path: path.into(),
        message: message.into(),
    }
}

fn object<'a>(value: &'a Value, path: &str) -> DecodeResult<&'a Value> {
    value
        .is_object()
        .then_some(value)
        .ok_or_else(|| error(path, "expected object"))
}

fn field<'a>(map: &'a Value, key: &str) -> Option<&'a Value> {
    map.get(key).filter(|value| !value.is_null())
}

fn required<'a>(map: &'a Value, key: &str, path: &str) -> DecodeResult<&'a Value> {
    field(map, key).ok_or_else(|| error(path, "missing required field"))
}

fn id(value: &Value, path: &str) -> DecodeResult<String> {
    let raw = value
        .utf8_string()
        .ok_or_else(|| error(path, "expected resource ID"))?;
    bbb_protocol::normalize_resource_location(raw).map_err(|e| error(path, e.to_string()))
}

fn holder_set(value: &Value, path: &str) -> DecodeResult<TradeHolderSet> {
    if let Some(reference) = value.utf8_string() {
        if let Some(tag) = reference.strip_prefix('#') {
            return id(&Value::String(tag.into()), path).map(TradeHolderSet::Tag);
        }
    }
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .enumerate()
            .map(|(i, v)| id(v, &format!("{path}[{i}]")))
            .collect::<DecodeResult<Vec<_>>>()
            .map(TradeHolderSet::Items);
    }
    id(value, path).map(|v| TradeHolderSet::Items(vec![v]))
}

fn loot_error(path: &str, e: LootDecodeError) -> TradeDecodeError {
    error(&format!("{path}.{}", e.path), e.message)
}

fn number(value: &Value, path: &str) -> DecodeResult<LootNumberProvider> {
    LootNumberProvider::decode(V::Json(value)).map_err(|e| loot_error(path, e))
}

fn constant(value: i32) -> LootNumberProvider {
    LootNumberProvider::Constant(value as f32)
}

fn components(map: &Value, path: &str) -> DecodeResult<Components> {
    field(map, "components")
        .map(|v| {
            v.as_object()
                .map(|entries| entries.to_vec())
                .ok_or_else(|| error(path, "expected components object"))
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

fn cost(value: &Value, path: &str) -> DecodeResult<TradeCost> {
    let map = object(value, path)?;
    let id_path = format!("{path}.id");
    Ok(TradeCost {
        item_id: id(required(map, "id", &id_path)?, &id_path)?,
        count: field(map, "count")
            .map(|v| number(v, &format!("{path}.count")))
            .transpose()?
            .unwrap_or_else(|| constant(1)),
        components: components(map, path)?,
    })
}

fn template(value: &Value, path: &str) -> DecodeResult<TradeItemTemplate> {
    let result = if value.is_string() {
        TradeItemTemplate {
            item_id: id(value, path)?,
            count: 1,
            components: Vec::new(),
        }
    } else {
        let map = object(value, path)?;
        let id_path = format!("{path}.id");
        let count = field(map, "count")
            .map(|v| {
                let path = format!("{path}.count");
                let count = V::Json(v)
                    .codec_i32()
                    .ok_or_else(|| error(&path, "expected number"))?;
                if !(1..=99).contains(&count) {
                    return Err(error(&path, "converted integer must be in 1..=99"));
                }
                Ok(count)
            })
            .transpose()?
            .unwrap_or(1);
        TradeItemTemplate {
            item_id: id(required(map, "id", &id_path)?, &id_path)?,
            count,
            components: components(map, path)?,
        }
    };
    if result.item_id == "minecraft:air" {
        return Err(error(path, "item must be non-empty"));
    }
    Ok(result)
}

/// `TradeSet.java:15-23`.
pub fn parse_trade_set(value: &Value) -> DecodeResult<TradeSet> {
    let map = object(value, "trade_set")?;
    Ok(TradeSet {
        trades: holder_set(required(map, "trades", "trades")?, "trades")?,
        amount: number(required(map, "amount", "amount")?, "amount")?,
        allow_duplicates: field(map, "allow_duplicates")
            .map(|v| {
                v.as_bool()
                    .ok_or_else(|| error("allow_duplicates", "expected boolean"))
            })
            .transpose()?
            .unwrap_or(false),
        random_sequence: field(map, "random_sequence")
            .map(|v| id(v, "random_sequence"))
            .transpose()?,
    })
}

/// `VillagerTrade.java:28-49`.
pub fn parse_villager_trade(value: &Value) -> DecodeResult<VillagerTrade> {
    parse_villager_trade_with_lookup(value, None)
}

fn parse_villager_trade_with_lookup(
    value: &Value,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> DecodeResult<VillagerTrade> {
    let map = object(value, "villager_trade")?;
    let lenient = |key, default| {
        field(map, key)
            .and_then(|v| number(v, key).ok())
            .unwrap_or_else(|| constant(default))
    };
    let given_item_modifiers = field(map, "given_item_modifiers")
        .map(|v| {
            v.as_array()
                .ok_or_else(|| error("given_item_modifiers", "expected array"))?
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    LootFunction::decode(V::Json(v), lookup)
                        .map_err(|e| loot_error(&format!("given_item_modifiers[{i}]"), e))
                })
                .collect::<DecodeResult<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(VillagerTrade {
        wants: cost(required(map, "wants", "wants")?, "wants")?,
        additional_wants: field(map, "additional_wants")
            .map(|v| cost(v, "additional_wants"))
            .transpose()?,
        gives: template(required(map, "gives", "gives")?, "gives")?,
        max_uses: lenient("max_uses", 4),
        reputation_discount: lenient("reputation_discount", 0),
        xp: lenient("xp", 1),
        merchant_predicate: field(map, "merchant_predicate")
            .map(|v| {
                LootCondition::decode(V::Json(v), lookup)
                    .map_err(|e| loot_error("merchant_predicate", e))
            })
            .transpose()?,
        given_item_modifiers,
        double_trade_price_enchantments: field(map, "double_trade_price_enchantments")
            .map(|v| holder_set(v, "double_trade_price_enchantments"))
            .transpose()?,
    })
}

/// Preserves `gives.count` numeric spelling until Java INT conversion.
pub fn parse_villager_trade_source(source: &[u8]) -> DecodeResult<VillagerTrade> {
    parse_villager_trade_remaining_source(source, None)
}

fn parse_villager_trade_remaining_source(
    source: &[u8],
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> DecodeResult<VillagerTrade> {
    let source = bbb_registry_codec::decode_loaded_registry_json(
        source,
        crate::registry_data_loader::DatapackLoadLimits::default().max_element_bytes,
    )
    .map_err(|error| self::error("villager_trade", error.to_string()))?;
    parse_villager_trade_with_lookup(source.json().expect("JSON source"), lookup)
}

#[cfg(test)]
mod tests;
