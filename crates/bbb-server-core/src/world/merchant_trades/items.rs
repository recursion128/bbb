use super::super::registry_runtime::PlayerItemStackResolver;
use super::TradeResult;
use bbb_protocol::data_component_types::vanilla_data_component_type_for_resource_id;
use bbb_protocol::NbtSummaryEntry;
use bbb_registry_codec::RegistryJsonValue as Value;
use bbb_registry_codec::{validate_persistent_component_nbt, PersistentNbtDecodeContext};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_datapack::trades::TradeItemTemplate;

pub(super) struct TradeItemEnvironment<'a> {
    pub startup: &'a DatapackStartup,
    pub tables: &'a super::super::datapack_install::DatapackRuntimeTables,
    pub registry_tags: Option<std::sync::Arc<crate::registry_tags::RegistryTagSnapshot>>,
    pub resolver: &'a PlayerItemStackResolver,
    pub enchantments: &'a [String],
}

impl TradeItemEnvironment<'_> {
    fn context(&self) -> PersistentNbtDecodeContext<'_> {
        PersistentNbtDecodeContext::with_resolver(self.resolver)
    }

    pub(super) fn template(
        &self,
        template: &TradeItemTemplate,
    ) -> TradeResult<bbb_server_types::item_stack::RuntimeItemStack> {
        let value = bbb_registry_codec::json_to_nbt(&Value::Object(vec![
            ("id".into(), Value::String(template.item_id.as_str().into())),
            ("count".into(), Value::Number(template.count.to_string())),
            (
                "components".into(),
                Value::Object(template.components.clone()),
            ),
        ]))?;
        bbb_server_types::item_stack::RuntimeItemStack::try_read(&value, self.resolver)
            .map_err(|error| error.to_string())
    }

    pub(super) fn cost_components(
        &self,
        components: &[(bbb_protocol::JavaString, Value)],
    ) -> TradeResult<Vec<NbtSummaryEntry>> {
        let mut decoded = Vec::new();
        for (name, value) in components {
            let id = bbb_protocol::normalize_resource_location(
                name.try_to_string().map_err(|_| "invalid component id")?,
            )
            .map_err(|error| error.to_string())?;
            let component = vanilla_data_component_type_for_resource_id(&id)
                .ok_or_else(|| format!("unknown exact cost component {id}"))?;
            let value = bbb_registry_codec::json_to_nbt(value)?;
            validate_persistent_component_nbt(component.type_id, &id, &value, self.context())
                .map_err(|error| error.to_string())?;
            if decoded
                .iter()
                .any(|entry: &NbtSummaryEntry| entry.name == id)
            {
                return Err(format!("duplicate cost component {id}"));
            }
            decoded.push(NbtSummaryEntry {
                name: id.into(),
                value,
            });
        }
        Ok(decoded)
    }
}
