//! Credential-free provider catalog projected from OpenCode V2 integrations.
//! Only public connection identifiers and labels cross into presentation state.
use crate::client::{OpencodeError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SUPPORTED_PROVIDER_IDS: &[&str] =
    &["opencode-go", "command-code", "commandcode", "openai"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProviderIntegration {
    pub id: String,
    pub name: String,
    pub key_method: Option<KeyMethod>,
    pub credentials: Vec<Credential>,
    pub environment: Environment,
    pub oauth: Vec<OAuthMethod>,
}
impl ProviderIntegration {
    pub fn connected(&self) -> bool {
        !self.credentials.is_empty() || !self.environment.connected.is_empty()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KeyMethod {
    pub label: Option<String>,
    pub fields: Vec<ProviderField>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Credential {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Environment {
    pub names: Vec<String>,
    pub connected: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OAuthMethod {
    pub id: String,
    pub label: String,
}

#[derive(Clone, PartialEq)]
pub struct OAuthAttempt {
    pub id: String,
    pub url: String,
    pub instructions: String,
    pub code: bool,
}
impl std::fmt::Debug for OAuthAttempt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthAttempt")
            .field("code", &self.code)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OAuthStatus {
    Pending,
    Complete,
    Expired,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    String,
    Number,
    Integer,
    Boolean,
    Multiselect,
    External,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderField {
    pub key: String,
    #[serde(rename = "type")]
    pub kind: FieldKind,
    pub title: Option<String>,
    pub description: Option<String>,
    pub required: Option<bool>,
    pub placeholder: Option<String>,
    pub url: Option<String>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub min_length: Option<i64>,
    pub max_length: Option<i64>,
    pub pattern: Option<String>,
    pub min_items: Option<i64>,
    pub max_items: Option<i64>,
    #[serde(default)]
    pub when: Vec<Condition>,
    pub options: Option<Vec<FieldOption>>,
    pub default: Option<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FieldOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Condition {
    pub key: String,
    pub op: ConditionOp,
    pub value: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConditionOp {
    Eq,
    Neq,
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}
fn rows<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn field(value: &Value) -> Option<ProviderField> {
    let mut value = value.as_object()?.clone();
    // The reference discards malformed optional metadata individually. A bad
    // description or constraint must not hide an otherwise valid form field.
    for key in ["title", "description", "placeholder", "url", "pattern"] {
        if value.get(key).is_some_and(|v| !v.is_string()) {
            value.remove(key);
        }
    }
    if value.get("required").is_some_and(|v| !v.is_boolean()) {
        value.remove("required");
    }
    for key in ["minimum", "maximum"] {
        if value
            .get(key)
            .is_some_and(|v| !v.as_f64().is_some_and(f64::is_finite))
        {
            value.remove(key);
        }
    }
    for key in ["minLength", "maxLength", "minItems", "maxItems"] {
        if let Some(raw) = value.get(key) {
            let number = raw
                .as_f64()
                .filter(|n| n.is_finite() && n.fract() == 0. && n.abs() <= 9_007_199_254_740_991.);
            if let Some(number) = number {
                value.insert(key.into(), Value::from(number as i64));
            } else {
                value.remove(key);
            }
        }
    }
    for key in ["when", "options"] {
        if value.get(key).is_some_and(|v| !v.is_array()) {
            value.remove(key);
        }
    }
    if let Some(conditions) = value.get_mut("when").and_then(Value::as_array_mut) {
        conditions.retain(|condition| {
            condition.get("key").is_some_and(Value::is_string)
                && matches!(
                    condition.get("op").and_then(Value::as_str),
                    Some("eq" | "neq")
                )
                && condition
                    .get("value")
                    .is_some_and(|v| v.is_string() || v.is_number() || v.is_boolean())
        });
    }
    if let Some(options) = value.get_mut("options").and_then(Value::as_array_mut) {
        options.retain(|option| {
            option.get("value").is_some_and(Value::is_string)
                && option.get("label").is_some_and(Value::is_string)
        });
        for option in options {
            if option.get("description").is_some_and(|v| !v.is_string()) {
                option.as_object_mut()?.remove("description");
            }
        }
    }
    if value.get("default").is_some_and(|v| {
        !(v.is_string()
            || v.is_number()
            || v.is_boolean()
            || v.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
    }) {
        value.remove("default");
    }
    serde_json::from_value(Value::Object(value)).ok()
}

pub(crate) fn parse_catalog(value: &Value) -> Result<Vec<ProviderIntegration>> {
    let catalog = value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .ok_or_else(|| OpencodeError::Protocol("integration list is not an array".into()))?;
    let mut providers = Vec::new();
    for id in SUPPORTED_PROVIDER_IDS {
        for row in catalog
            .iter()
            .filter(|row| row.get("id").and_then(Value::as_str) == Some(*id))
        {
            let methods = rows(row, "methods");
            let key_method = methods
                .iter()
                .find(|m| m.get("type").and_then(Value::as_str) == Some("key"))
                .map(|m| KeyMethod {
                    label: text(m, "label"),
                    fields: rows(m, "form").iter().filter_map(field).collect(),
                });
            let mut environment = Environment::default();
            let mut oauth = Vec::new();
            for method in methods {
                match method.get("type").and_then(Value::as_str) {
                    Some("env") => {
                        for name in rows(method, "names").iter().filter_map(Value::as_str) {
                            if !environment.names.iter().any(|existing| existing == name) {
                                environment.names.push(name.into());
                            }
                        }
                    }
                    Some("oauth") => {
                        if let Some(id) = text(method, "id").filter(|id| !id.is_empty()) {
                            oauth.push(OAuthMethod {
                                id,
                                label: text(method, "label").unwrap_or_else(|| "OAuth".into()),
                            });
                        }
                    }
                    _ => {}
                }
            }
            let mut credentials = Vec::new();
            for connection in rows(row, "connections") {
                match connection.get("type").and_then(Value::as_str) {
                    Some("credential") => {
                        if let Some(id) = text(connection, "id").filter(|id| !id.is_empty()) {
                            credentials.push(Credential {
                                id,
                                label: text(connection, "label")
                                    .unwrap_or_else(|| "default".into()),
                            });
                        }
                    }
                    Some("env") => {
                        if let Some(name) = text(connection, "name").filter(|name| !name.is_empty())
                        {
                            environment.connected.push(name);
                        }
                    }
                    _ => {}
                }
            }
            providers.push(ProviderIntegration {
                id: (*id).into(),
                name: text(row, "name").unwrap_or_else(|| (*id).into()),
                key_method,
                credentials,
                environment,
                oauth,
            });
        }
    }
    Ok(providers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn catalog_matches_reference_order_aliases_and_connection_sources_without_secrets() {
        let input = json!({"data":[
            {"id":"openai","name":"OpenAI","methods":[{"type":"key","label":"API key","form":[]},{"id":"login","type":"oauth","label":"ChatGPT"}],"connections":[{"type":"credential","id":"cred-1","label":"Work","key":"must-not-cross-boundary"}]},
            {"id":"unlisted","name":"Hidden provider"},
            {"id":"commandcode","name":"CommandCode","methods":[{"type":"env","names":["CMD_API_KEY","CMD_API_KEY"]}],"connections":[{"type":"env","name":"CMD_API_KEY"}]},
            {"id":"command-code","name":"Command Code"},
            {"id":"opencode-go","name":"OpenCode Go","methods":[{"type":"env","names":["OPENCODE_API_KEY"]}],"connections":[]}
        ]});
        let providers = parse_catalog(&input).unwrap();
        assert_eq!(
            providers.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            SUPPORTED_PROVIDER_IDS
        );
        assert!(!providers[0].connected());
        assert!(providers[2].connected());
        assert_eq!(providers[2].environment.names, ["CMD_API_KEY"]);
        assert!(providers[3].connected());
        assert_eq!(providers[3].credentials[0].label, "Work");
        assert_eq!(providers[3].oauth[0].label, "ChatGPT");
        assert!(!format!("{providers:?}").contains("must-not-cross-boundary"));
        assert!(
            !serde_json::to_string(&providers)
                .unwrap()
                .contains("must-not-cross-boundary")
        );
        assert!(parse_catalog(&json!({"data":null})).is_err());
    }
    #[test]
    fn typed_fields_preserve_conditions_options_defaults_and_constraints() {
        let value = json!({"id":"openai","methods":[{"type":"key","form":[
            {"key":"region","type":"string","required":true,"options":[{"value":"eu","label":"Europe"},{}],"default":"eu","when":[{"key":"enabled","op":"eq","value":true},{"key":"bad","op":"unknown","value":false}]},
            {"key":"count","type":"integer","minimum":1,"maximum":4,"default":2},
            {"key":"models","type":"multiselect","minItems":1,"maxItems":3,"default":["a","b"]},
            {"key":"help","type":"external","url":"https://example.com/help"},
            {"key":"invalid","type":"unknown"}
        ]}]});
        let providers = parse_catalog(&json!([value])).unwrap();
        let fields = &providers[0].key_method.as_ref().unwrap().fields;
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[0].when.len(), 1);
        assert_eq!(fields[0].options.as_ref().unwrap().len(), 1);
        assert_eq!(fields[1].kind, FieldKind::Integer);
        assert_eq!(fields[1].maximum, Some(4.));
        assert_eq!(fields[2].default, Some(json!(["a", "b"])));
        assert_eq!(fields[3].kind, FieldKind::External);
    }
    #[test]
    fn malformed_optional_metadata_does_not_hide_valid_fields() {
        let providers = parse_catalog(&json!([{"id":"openai","methods":[{"type":"key","form":[
            {"key":"region","type":"string","title":42,"description":[],"required":"yes",
             "minimum":"bad","maxLength":1.5,"minLength":-1,"when":"bad","default":{},
             "options":[{"value":"eu","label":"Europe","description":false}]},
            {"key":"count","type":"integer","maximum":4,"maxLength":4.0,"minItems":9007199254740992u64},
            {"key":"bad","type":"unknown"}
        ]}]}]))
        .unwrap();
        let fields = &providers[0].key_method.as_ref().unwrap().fields;
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].title, None);
        assert_eq!(fields[0].required, None);
        assert_eq!(fields[0].min_length, Some(-1));
        assert_eq!(fields[0].max_length, None);
        assert!(fields[0].when.is_empty());
        assert_eq!(fields[0].options.as_ref().unwrap()[0].description, None);
        assert_eq!(fields[1].min_items, None);
        assert_eq!(fields[1].max_length, Some(4));
    }
}
