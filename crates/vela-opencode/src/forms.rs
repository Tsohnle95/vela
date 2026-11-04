//! Pending agent form normalization. Malformed optional metadata is dropped
//! field-by-field so one bad field never hides the rest of a form.
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct FormOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FormKind {
    Text,
    Number,
    Integer,
    Boolean,
    MultiSelect,
    External,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormField {
    pub key: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub required: bool,
    pub kind: FormKind,
    pub placeholder: Option<String>,
    pub url: Option<String>,
    pub options: Vec<FormOption>,
    pub default: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormRequest {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub fields: Vec<FormField>,
}

fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn options(value: &Value) -> Vec<FormOption> {
    value
        .get("options")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    let value = string(option, "value")?;
                    let label = string(option, "label")?;
                    Some(FormOption {
                        value,
                        label,
                        description: string(option, "description"),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn field(value: &Value) -> Option<FormField> {
    let key = string(value, "key")?;
    let kind = match string(value, "type").as_deref() {
        Some("number") => FormKind::Number,
        Some("integer") => FormKind::Integer,
        Some("boolean") => FormKind::Boolean,
        Some("multiselect") => FormKind::MultiSelect,
        Some("external") => FormKind::External,
        Some(_) => FormKind::Text,
        None => return None,
    };
    Some(FormField {
        key,
        title: string(value, "title"),
        description: string(value, "description"),
        required: value.get("required").and_then(Value::as_bool) == Some(true),
        kind,
        placeholder: string(value, "placeholder"),
        url: string(value, "url"),
        options: options(value),
        default: value.get("default").cloned(),
    })
}

pub fn parse_form(value: &Value, fallback_session: &str) -> Option<FormRequest> {
    let id = string(value, "id").filter(|id| !id.is_empty())?;
    let session_id = string(value, "sessionID")
        .filter(|session| !session.is_empty())
        .unwrap_or_else(|| fallback_session.to_owned());
    let fields = value
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(field).collect::<Vec<_>>())
        .unwrap_or_default();
    Some(FormRequest {
        id,
        session_id,
        title: string(value, "title")
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| "Agent request".into()),
        fields,
    })
}

pub fn parse_forms(value: &Value, fallback_session: &str) -> Vec<FormRequest> {
    let rows = value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.as_array());
    rows.map(|rows| {
        rows.iter()
            .filter_map(|row| parse_form(row, fallback_session))
            .collect()
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_typed_fields_options_defaults_and_session_fallback() {
        let value = json!({"data":[{
            "id":"form-1",
            "title":"Deployment",
            "fields":[
                {"key":"name","type":"string","title":"Name","required":true,"placeholder":"service","default":"api"},
                {"key":"count","type":"integer","required":true},
                {"key":"ratio","type":"number","minimum":0},
                {"key":"enabled","type":"boolean","default":true},
                {"key":"targets","type":"multiselect","options":[
                    {"value":"a","label":"A","description":"first"},
                    {"value":"b","label":"B"},
                    {"value":3,"label":"bad"}
                ]},
                {"key":"docs","type":"external","url":"https://example.com"},
                {"type":"string"},
                {"key":"unknown","type":"future"}
            ]
        }]});
        let forms = parse_forms(&value, "session-1");
        assert_eq!(forms.len(), 1);
        let form = &forms[0];
        assert_eq!(form.session_id, "session-1");
        assert_eq!(form.fields.len(), 7);
        assert_eq!(form.fields[0].default, Some(json!("api")));
        assert_eq!(form.fields[1].kind, FormKind::Integer);
        assert_eq!(form.fields[3].default, Some(json!(true)));
        assert_eq!(form.fields[4].options.len(), 2);
        assert_eq!(form.fields[4].options[0].description.as_deref(), Some("first"));
        assert_eq!(form.fields[5].kind, FormKind::External);
        assert_eq!(form.fields[5].url.as_deref(), Some("https://example.com"));
        assert_eq!(form.fields[6].kind, FormKind::Text);
    }

    #[test]
    fn malformed_forms_and_fields_do_not_hide_valid_rows() {
        let value = json!([
            {"id":"","fields":[]},
            {"id":"ok","sessionID":"true-session","fields":[{"key":"x","type":"string"}]},
            {"id":"no-fields","fields":"bad"}
        ]);
        let forms = parse_forms(&value, "fallback");
        assert_eq!(forms.len(), 2);
        assert_eq!(forms[0].session_id, "true-session");
        assert_eq!(forms[0].fields.len(), 1);
        assert!(parse_forms(&json!({"data":"bad"}), "fallback").is_empty());
    }
}
