use bbb_protocol::{packets::GameProfilePropertySummary, JavaString};
use serde::{
    de::{Error, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::value::RawValue;
use std::fmt;

pub(super) struct SessionProfileResponse {
    pub(super) id: String,
    /// Gson reads a `\\uXXXX` escape as one char without pairing it, so the
    /// session server's name arrives with whatever units it was written with.
    pub(super) name: JavaString,
    pub(super) properties: Vec<GameProfilePropertySummary>,
}

#[derive(Deserialize)]
struct JsonString(#[serde(deserialize_with = "JavaString::deserialize_json_string")] JavaString);

struct RawObject(Vec<(JavaString, Box<RawValue>)>);

impl RawObject {
    fn field(&self, name: &str) -> Option<&RawValue> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_ref())
    }
}

impl<'de> Deserialize<'de> for RawObject {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = RawObject;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a profile property object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut entries: Vec<(JavaString, Box<RawValue>)> = Vec::new();
                while let Some((JsonString(name), value)) =
                    map.next_entry::<JsonString, Box<RawValue>>()?
                {
                    if let Some((_, previous)) = entries.iter_mut().find(|(key, _)| key == &name) {
                        *previous = value;
                    } else {
                        entries.push((name, value));
                    }
                }
                Ok(RawObject(entries))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

impl<'de> Deserialize<'de> for SessionProfileResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ResponseVisitor;
        impl<'de> Visitor<'de> for ResponseVisitor {
            type Value = SessionProfileResponse;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a session profile")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (mut id, mut name, mut properties) = (None, None, None);
                while let Some(JsonString(key)) = map.next_key::<JsonString>()? {
                    if key == "id" {
                        id = map.next_value::<Option<String>>()?;
                    } else if key == "name" {
                        name = map
                            .next_value::<Option<JsonString>>()?
                            .map(|JsonString(name)| name);
                    } else if key == "properties" {
                        let raw = map.next_value::<Box<RawValue>>()?;
                        properties = if raw.get() == "null" {
                            None
                        } else {
                            Some(decode_properties(&raw).map_err(A::Error::custom)?)
                        };
                    } else {
                        let _ = map.next_value::<Box<RawValue>>()?;
                    }
                }
                Ok(SessionProfileResponse {
                    id: id.ok_or_else(|| A::Error::missing_field("id"))?,
                    name: name.ok_or_else(|| A::Error::missing_field("name"))?,
                    properties: properties.ok_or_else(|| A::Error::missing_field("properties"))?,
                })
            }
        }
        deserializer.deserialize_map(ResponseVisitor)
    }
}

fn primitive_string(
    raw: &RawValue,
    singleton_arrays: bool,
) -> Result<JavaString, serde_json::Error> {
    let mut raw = raw.to_owned();
    loop {
        match raw.get().as_bytes().first() {
            Some(b'"') => return JavaString::from_json_string(raw.get()),
            Some(b't' | b'f' | b'-' | b'0'..=b'9') => return Ok(raw.get().into()),
            Some(b'[') if singleton_arrays => {
                let mut values: Vec<Box<RawValue>> = serde_json::from_str(raw.get())?;
                if values.len() != 1 {
                    return Err(serde_json::Error::custom(
                        "property value array must contain one element",
                    ));
                }
                raw = values.pop().unwrap();
            }
            _ => {
                return Err(serde_json::Error::custom(
                    "profile property value is not a primitive",
                ))
            }
        }
    }
}

fn decode_properties(raw: &RawValue) -> Result<Vec<GameProfilePropertySummary>, serde_json::Error> {
    let mut properties = Vec::new();
    match raw.get().as_bytes().first() {
        Some(b'[') => {
            let values: Vec<Box<RawValue>> = serde_json::from_str(raw.get())?;
            for value in values {
                if !value.get().starts_with('{') {
                    continue;
                }
                let object: RawObject = serde_json::from_str(value.get())?;
                let required = |key| {
                    object
                        .field(key)
                        .ok_or_else(|| serde_json::Error::custom(format!("property lacks {key}")))
                };
                properties.push(GameProfilePropertySummary {
                    name: primitive_string(required("name")?, false)?,
                    value: primitive_string(required("value")?, false)?,
                    signature: object
                        .field("signature")
                        .map(|value| primitive_string(value, false))
                        .transpose()?,
                });
            }
        }
        Some(b'{') => {
            let object: RawObject = serde_json::from_str(raw.get())?;
            for (name, values) in object.0 {
                if !values.get().starts_with('[') {
                    continue;
                }
                let values: Vec<Box<RawValue>> = serde_json::from_str(values.get())?;
                for value in values {
                    properties.push(GameProfilePropertySummary {
                        name: name.clone(),
                        value: primitive_string(&value, true)?,
                        signature: None,
                    });
                }
            }
        }
        _ => {}
    }
    let mut groups: Vec<(JavaString, Vec<GameProfilePropertySummary>)> = Vec::new();
    for property in properties {
        if let Some((_, values)) = groups.iter_mut().find(|(name, _)| name == &property.name) {
            values.push(property);
        } else {
            groups.push((property.name.clone(), vec![property]));
        }
    }
    Ok(groups.into_iter().flat_map(|(_, values)| values).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(properties: &str) -> Result<SessionProfileResponse, serde_json::Error> {
        serde_json::from_str(&format!(
            r#"{{"id":"00000000000000000000000000000001","name":"Alex","properties":{properties}}}"#
        ))
    }

    #[test]
    fn http_profile_properties_follow_authlib_shapes_and_primitive_coercion() {
        let decoded = response(r#"[null,4,[],{"name":"a","value":1e2},{"name":"b","value":true,"signature":7},{"name":"a","value":"last"}]"#).unwrap();
        assert_eq!(
            decoded
                .properties
                .iter()
                .map(|p| p.value.clone())
                .collect::<Vec<_>>(),
            vec![JavaString::from("1e2"), "last".into(), "true".into()]
        );
        assert_eq!(decoded.properties[2].signature, Some("7".into()));
        let decoded = response(r#"{"a":["old"],"ignored":3,"b":[[[false]]],"a":[1e2]}"#).unwrap();
        assert_eq!(
            decoded
                .properties
                .iter()
                .map(|p| p.value.clone())
                .collect::<Vec<_>>(),
            vec![JavaString::from("1e2"), "false".into()]
        );
        for empty in ["false", "42", r#""ignored""#] {
            assert!(response(empty).unwrap().properties.is_empty());
        }
        for invalid in [
            "null",
            r#"[{"name":"a","value":"v","signature":null}]"#,
            r#"[{"name":[],"value":"v"}]"#,
            r#"[{"name":"a"}]"#,
            r#"{"a":[[]]}"#,
            r#"{"a":[null]}"#,
        ] {
            assert!(response(invalid).is_err(), "{invalid}");
        }
        assert!(
            serde_json::from_str::<SessionProfileResponse>(r#"{"id":"1","name":"Alex"}"#).is_err()
        );
    }

    #[test]
    fn http_profile_duplicate_root_fields_do_not_hide_prior_decode_failure() {
        let valid = r#"{"id":"1","name":"Alex","properties":[],"properties":{"a":["last"]}}"#;
        assert_eq!(
            serde_json::from_str::<SessionProfileResponse>(valid)
                .unwrap()
                .properties[0]
                .value,
            "last"
        );
        let invalid = r#"{"id":"1","name":"Alex","properties":[{"name":null,"value":"bad"}],"properties":[]}"#;
        assert!(serde_json::from_str::<SessionProfileResponse>(invalid).is_err());
    }
}
