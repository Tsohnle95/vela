//! Composer command/skill catalog normalization.
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandKind {
    Builtin,
    Command,
    Skill,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandOption {
    pub name: String,
    pub description: Option<String>,
    pub kind: CommandKind,
}

pub fn builtins() -> Vec<CommandOption> {
    vec![
        CommandOption {
            name: "compact".into(),
            description: Some("Summarize the session to free up context".into()),
            kind: CommandKind::Builtin,
        },
        CommandOption {
            name: "compress".into(),
            description: Some("Alias of /compact — summarize the session to free up context".into()),
            kind: CommandKind::Builtin,
        },
    ]
}

fn rows(value: &Value) -> &[Value] {
    value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn priority(kind: &CommandKind) -> u8 {
    match kind {
        CommandKind::Builtin => 0,
        CommandKind::Command => 1,
        CommandKind::Skill => 2,
    }
}

/// Keep one entry per case-insensitive name, preferring built-ins over runtime
/// commands over skills while preserving stable order within a priority.
pub fn dedupe_catalog(catalog: Vec<CommandOption>) -> Vec<CommandOption> {
    let mut unique: Vec<CommandOption> = Vec::new();
    for item in catalog {
        if let Some(existing) = unique
            .iter_mut()
            .find(|existing| existing.name.eq_ignore_ascii_case(&item.name))
        {
            if priority(&item.kind) < priority(&existing.kind) {
                *existing = item;
            }
        } else {
            unique.push(item);
        }
    }
    unique
}

pub fn contains_command(catalog: &[CommandOption], name: &str) -> bool {
    catalog
        .iter()
        .any(|item| item.name.eq_ignore_ascii_case(name))
}

pub fn parse_catalog(commands: &Value, skills: &Value) -> Vec<CommandOption> {
    let mut catalog = builtins();
    for row in rows(commands) {
        let Some(name) = row.get("name").and_then(Value::as_str).filter(|name| !name.is_empty())
        else {
            continue;
        };
        catalog.push(CommandOption {
            name: name.to_owned(),
            description: row
                .get("description")
                .and_then(Value::as_str)
                .filter(|description| !description.is_empty())
                .map(str::to_owned),
            kind: CommandKind::Command,
        });
    }
    for row in rows(skills) {
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .or_else(|| row.get("id").and_then(Value::as_str))
            .filter(|name| !name.is_empty());
        let Some(name) = name else { continue };
        catalog.push(CommandOption {
            name: name.to_owned(),
            description: row
                .get("description")
                .and_then(Value::as_str)
                .filter(|description| !description.is_empty())
                .map(str::to_owned),
            kind: CommandKind::Skill,
        });
    }
    dedupe_catalog(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn catalog_includes_builtins_and_drops_malformed_rows() {
        let catalog = parse_catalog(
            &json!({"data":[
                {"name":"review","description":"Review changes"},
                {"name":""},
                {"description":"missing"},
                "bad"
            ]}),
            &json!([{"id":"skill_review","name":"review-skill","description":"Skill"}]),
        );
        assert_eq!(catalog.len(), 4);
        assert_eq!(catalog[0].name, "compact");
        assert_eq!(catalog[2].name, "review");
        assert_eq!(catalog[3].name, "review-skill");
        assert_eq!(catalog[3].kind, CommandKind::Skill);
    }

    #[test]
    fn duplicate_names_keep_the_highest_priority_entry() {
        let catalog = parse_catalog(
            &json!({"data":[
                {"name":"review","description":"command review"},
                {"name":"Review","description":"duplicate"},
                {"name":"compact","description":"shadow builtin"}
            ]}),
            &json!([
                {"id":"a","name":"review","description":"skill review"},
                {"id":"b","name":"notes","description":"notes skill"}
            ]),
        );
        assert_eq!(
            catalog.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(),
            vec!["compact", "compress", "review", "notes"]
        );
        assert_eq!(catalog[0].kind, CommandKind::Builtin);
        assert_eq!(catalog[1].kind, CommandKind::Builtin);
        assert_eq!(catalog[2].kind, CommandKind::Command);
        assert_eq!(
            catalog[2].description.as_deref(),
            Some("command review")
        );
        assert_eq!(catalog[3].kind, CommandKind::Skill);
        assert!(contains_command(&catalog, "REVIEW"));
        assert!(contains_command(&catalog, "notes"));
        assert!(!contains_command(&catalog, "missing"));
        assert!(!contains_command(&[], "review"));
    }
}
