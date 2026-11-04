//! Workspace inventory rows reported by the running OpenCode service.
//!
//! The reference IPC filters malformed rows rather than failing the whole
//! inventory: empty names/ids are dropped and missing status/source values fall
//! back to `unknown`.

use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct McpServerInfo {
    pub name: String,
    pub status: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluginInfo {
    pub id: String,
    pub source: String,
    pub status: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

fn rows(value: &Value) -> Vec<&Value> {
    value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

fn text<'a>(row: &'a Value, key: &str) -> Option<&'a str> {
    row.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn parse_mcp_servers(value: &Value) -> Vec<McpServerInfo> {
    rows(value)
        .into_iter()
        .filter_map(|row| {
            let name = text(row, "name")?;
            Some(McpServerInfo {
                name: name.to_owned(),
                status: text(row, "status").unwrap_or("unknown").to_owned(),
            })
        })
        .collect()
}

pub fn parse_plugins(value: &Value) -> Vec<PluginInfo> {
    rows(value)
        .into_iter()
        .filter_map(|row| {
            let id = text(row, "id")?;
            Some(PluginInfo {
                id: id.to_owned(),
                source: row
                    .get("source")
                    .and_then(|source| source.get("type"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|source| !source.is_empty())
                    .unwrap_or("unknown")
                    .to_owned(),
                status: text(row, "status").unwrap_or("unknown").to_owned(),
            })
        })
        .collect()
}

pub fn parse_skills(value: &Value) -> Vec<SkillInfo> {
    rows(value)
        .into_iter()
        .filter_map(|row| {
            let id = text(row, "id")?;
            let name = text(row, "name")?;
            Some(SkillInfo {
                id: id.to_owned(),
                name: name.to_owned(),
                description: text(row, "description").unwrap_or_default().to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn inventory_parsers_accept_arrays_and_data_wrappers_and_filter_bad_rows() {
        let mcp = json!([
            {"name": "filesystem", "status": "connected"},
            {"name": "  "},
            {"status": "disabled"},
            {"name": "github"}
        ]);
        assert_eq!(
            parse_mcp_servers(&mcp),
            vec![
                McpServerInfo {
                    name: "filesystem".into(),
                    status: "connected".into()
                },
                McpServerInfo {
                    name: "github".into(),
                    status: "unknown".into()
                }
            ]
        );
        assert_eq!(
            parse_mcp_servers(&json!({"data": mcp})).len(),
            2
        );

        let plugins = json!([
            {"id": "alpha", "source": {"type": "npm"}, "status": "loaded"},
            {"id": "", "source": {"type": "npm"}},
            {"id": "beta"},
            {"source": {"type": "local"}}
        ]);
        assert_eq!(
            parse_plugins(&plugins),
            vec![
                PluginInfo {
                    id: "alpha".into(),
                    source: "npm".into(),
                    status: "loaded".into()
                },
                PluginInfo {
                    id: "beta".into(),
                    source: "unknown".into(),
                    status: "unknown".into()
                }
            ]
        );

        let skills = json!({"data": [
            {"id": "review", "name": "review", "description": "Review a change"},
            {"id": "empty"},
            {"name": "nameless"},
            {"id": "docs", "name": "docs"}
        ]});
        assert_eq!(
            parse_skills(&skills),
            vec![
                SkillInfo {
                    id: "review".into(),
                    name: "review".into(),
                    description: "Review a change".into()
                },
                SkillInfo {
                    id: "docs".into(),
                    name: "docs".into(),
                    description: String::new()
                }
            ]
        );
        assert!(parse_mcp_servers(&json!(null)).is_empty());
        assert!(parse_plugins(&json!({"data": "nope"})).is_empty());
    }
}
