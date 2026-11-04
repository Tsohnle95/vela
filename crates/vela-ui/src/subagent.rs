//! Delegated-agent records embedded in transcript text.
//!
//! OpenCode emits both `<subagent ...>` completion tags and older
//! `agent=...`/`prompt=...` notices. Vela turns a resolvable record into the
//! same clickable card shape as a task tool so a child session can be reopened.

use vela_core::session::SessionSummary;
use vela_core::transcript::ToolStatus;
use vela_opencode::client::AgentInfo;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SubagentRef {
    pub id: String,
    pub agent: String,
    pub description: String,
    pub state: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolDispatch {
    pub agent: String,
    pub description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Working,
    Complete,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Card {
    pub child_id: String,
    pub title: String,
    pub agent: String,
    pub description: String,
    pub state: String,
    pub tone: Tone,
}

fn attribute(attributes: &str, name: &str) -> String {
    let lower = attributes.to_ascii_lowercase();
    let needle = format!("{}=\"", name.to_ascii_lowercase());
    let Some(start) = lower.find(&needle).map(|index| index + needle.len()) else {
        return String::new();
    };
    let Some(length) = attributes[start..].find('"') else {
        return String::new();
    };
    attributes[start..start + length].to_string()
}

/// Parse the current `<subagent id="..." agent="..." ...>` record.
pub(crate) fn parse_tag(text: &str) -> Option<SubagentRef> {
    let lower = text.to_ascii_lowercase();
    let start = lower.find("<subagent")?;
    let after = start + "<subagent".len();
    let boundary = text[after..].chars().next()?;
    if !(boundary.is_whitespace() || boundary == '>' || boundary == '/') {
        return None;
    }
    let close = text[after..].find('>')? + after;
    let id = attribute(&text[after..close], "id");
    if id.is_empty() {
        return None;
    }
    let attributes = &text[after..close];
    Some(SubagentRef {
        id,
        agent: attribute(attributes, "agent"),
        description: attribute(attributes, "description"),
        state: attribute(attributes, "state"),
    })
}

/// Parse the legacy two-line `agent=...` / `prompt=...` dispatch notice.
pub(crate) fn parse_legacy(text: &str) -> Option<SubagentRef> {
    let mut agent = String::new();
    let mut prompt = String::new();
    for line in text.lines() {
        let line = line.trim_start();
        if agent.is_empty()
            && let Some(value) = assignment(line, "agent")
        {
            agent = value
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string();
            continue;
        }
        if prompt.is_empty()
            && let Some(value) = assignment(line, "prompt")
        {
            prompt = value.trim().to_string();
        }
    }
    if agent.is_empty() {
        return None;
    }
    Some(SubagentRef {
        id: String::new(),
        agent,
        description: prompt,
        state: String::new(),
    })
}

fn assignment<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let rest = line.get(..name.len())?;
    if !rest.eq_ignore_ascii_case(name) {
        return None;
    }
    let rest = line[name.len()..].trim_start();
    rest.strip_prefix('=').map(str::trim_start)
}

pub(crate) fn parse(text: &str) -> Option<SubagentRef> {
    parse_tag(text).or_else(|| parse_legacy(text))
}

fn dispatch_signature(agent: &str, description: &str) -> Option<String> {
    let agent = agent.trim().to_ascii_lowercase();
    let description = description
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if agent.is_empty() && description.is_empty() {
        None
    } else {
        Some(format!("dispatch:{agent}:{description}"))
    }
}

pub(crate) fn ref_dispatch_signature(reference: &SubagentRef) -> Option<String> {
    dispatch_signature(&reference.agent, &reference.description)
}

pub(crate) fn is_task_tool(name: &str) -> bool {
    let key = name
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>();
    key == "task" || key == "subagent"
}

/// A task tool's requested agent and the human description used for child
/// session matching.
pub(crate) fn tool_dispatch(name: &str, input: &str) -> Option<ToolDispatch> {
    if !is_task_tool(name) {
        return None;
    }
    let input = serde_json::from_str::<serde_json::Value>(input).ok()?;
    let text = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| input.get(*name).and_then(serde_json::Value::as_str))
            .unwrap_or("")
    };
    let agent = text(&["agent", "subagent_type"]).to_string();
    let description = text(&["description"]);
    let description = if description.is_empty() {
        text(&["prompt"]).trim().to_string()
    } else {
        description.to_string()
    };
    Some(ToolDispatch { agent, description })
}

/// A `task`/`subagent` tool's dispatch identity, used to collapse the generic
/// tool row and its later synthetic completion record into one card.
pub(crate) fn tool_dispatch_signature(name: &str, input: &str) -> Option<String> {
    let dispatch = tool_dispatch(name, input)?;
    dispatch_signature(&dispatch.agent, &dispatch.description)
}

/// Apply a synthetic completion state to an existing task tool snapshot.
pub(crate) fn settled_status(current: ToolStatus, state: &str) -> ToolStatus {
    match state.to_ascii_lowercase().as_str() {
        "failed" | "error" | "cancelled" => ToolStatus::Failed,
        "complete" | "completed" | "success" => ToolStatus::Success,
        _ => current,
    }
}

/// Build the card for a non-synthetic task tool. The card still renders when
/// no child session can be resolved; it is just not clickable, matching Vela.
pub(crate) fn tool_card(
    explicit_child: &str,
    name: &str,
    input: &str,
    status: ToolStatus,
    sessions: &[SessionSummary],
    agents: &[AgentInfo],
    parent: &str,
) -> Option<Card> {
    let dispatch = tool_dispatch(name, input)?;
    let child_id = if explicit_child.is_empty() {
        match_child(sessions, parent, &dispatch.agent, &dispatch.description)
    } else {
        explicit_child.to_string()
    };
    let resolved = sessions.iter().find(|session| session.id == child_id);
    let requested = configured_agent(agents, &dispatch.agent)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| dispatch.agent.clone());
    let agent = requested;
    let title = resolved
        .map(|session| session.title.trim())
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if agent.is_empty() {
                "Subagent".into()
            } else {
                title_case(&agent)
            }
        });
    let description = if dispatch.description.is_empty() {
        child_id.clone()
    } else {
        dispatch.description
    };
    let (state, tone) = match status {
        ToolStatus::Running | ToolStatus::Streaming => ("running", Tone::Working),
        ToolStatus::Failed => ("failed", Tone::Failed),
        _ => ("completed", Tone::Complete),
    };
    Some(Card {
        child_id,
        title,
        agent: if agent.is_empty() {
            "Agent".into()
        } else {
            agent
        },
        description,
        state: state.into(),
        tone,
    })
}

/// Source's internal system reminders are prompt scaffolding, never transcript
/// content. Hide them before they become rows.
pub(crate) fn is_internal_reminder(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let Some(start) = lower.find("<system-reminder") else {
        return false;
    };
    let after = start + "<system-reminder".len();
    let boundary = text[after..].chars().next();
    if !boundary.is_some_and(|c| c.is_whitespace() || c == '>') {
        return false;
    }
    let Some(close) = text[after..].find('>') else {
        return false;
    };
    lower[after + close + 1..].contains("</system-reminder>")
}

fn match_child(
    sessions: &[SessionSummary],
    parent: &str,
    agent: &str,
    description: &str,
) -> String {
    if parent.is_empty() {
        return String::new();
    }
    let mut candidates = sessions
        .iter()
        .filter(|session| session.parent_id.as_deref() == Some(parent))
        .collect::<Vec<_>>();
    if !description.is_empty() {
        let described = candidates
            .iter()
            .copied()
            .filter(|session| session.title.starts_with(description))
            .collect::<Vec<_>>();
        if !described.is_empty() {
            candidates = described;
        } else if agent.is_empty() {
            return String::new();
        }
    }
    if !agent.is_empty() {
        let key = agent.to_ascii_lowercase();
        let mention = format!("@{agent}");
        let by_agent = candidates
            .iter()
            .copied()
            .filter(|session| {
                session
                    .agent
                    .as_deref()
                    .is_some_and(|name| name.to_ascii_lowercase() == key)
                    || session.title.contains(&mention)
            })
            .collect::<Vec<_>>();
        if by_agent.is_empty() {
            return String::new();
        }
        candidates = by_agent;
    }
    candidates
        .into_iter()
        .max_by_key(|session| session.updated_at)
        .map(|session| session.id.clone())
        .unwrap_or_default()
}

/// Explicit ids always resolve; legacy notices match a child under the active
/// parent by description and agent, preferring the most recently updated one.
pub(crate) fn child_id(
    reference: &SubagentRef,
    sessions: &[SessionSummary],
    parent: &str,
) -> String {
    if !reference.id.is_empty() {
        return reference.id.clone();
    }
    let description = reference.description.chars().take(60).collect::<String>();
    match_child(sessions, parent, &reference.agent, &description)
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn configured_agent(agents: &[AgentInfo], name: &str) -> Option<String> {
    if name.is_empty() {
        return None;
    }
    let key = name.to_ascii_lowercase();
    agents
        .iter()
        .find(|agent| {
            agent.id.to_ascii_lowercase() == key || agent.name.to_ascii_lowercase() == key
        })
        .map(|agent| agent.name.clone())
}

/// Build the delegated-agent card for a resolvable record. Unresolved records
/// stay in the ordinary session-event presentation, matching Vela.
pub(crate) fn card(
    reference: &SubagentRef,
    sessions: &[SessionSummary],
    agents: &[AgentInfo],
    parent: &str,
) -> Option<Card> {
    let child_id = child_id(reference, sessions, parent);
    if child_id.is_empty() {
        return None;
    }
    let resolved = sessions.iter().find(|session| session.id == child_id);
    let agent = resolved
        .and_then(|session| session.agent.as_deref())
        .and_then(|name| configured_agent(agents, name))
        .or_else(|| configured_agent(agents, &reference.agent))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| reference.agent.clone());
    let title = resolved
        .map(|session| session.title.trim())
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if !reference.id.is_empty() {
                if reference.description.is_empty() {
                    "Subagent".into()
                } else {
                    reference.description.clone()
                }
            } else if agent.is_empty() {
                "Subagent".into()
            } else {
                title_case(&agent)
            }
        });
    let description = if reference.description.chars().count() > 100 {
        format!(
            "{}…",
            reference.description.chars().take(100).collect::<String>()
        )
    } else {
        reference.description.clone()
    };
    let state = if reference.state.is_empty() {
        "running".into()
    } else {
        reference.state.clone()
    };
    let lower = state.to_ascii_lowercase();
    let tone = if lower == "error" || lower == "cancelled" || lower == "failed" {
        Tone::Failed
    } else if lower.is_empty() || lower == "running" {
        Tone::Working
    } else {
        Tone::Complete
    };
    Some(Card {
        child_id,
        title,
        agent: if agent.is_empty() {
            "Agent".into()
        } else {
            agent
        },
        description,
        state,
        tone,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, title: &str, agent: Option<&str>, updated: i64) -> SessionSummary {
        SessionSummary {
            id: id.into(),
            title: title.into(),
            directory: "/workspace".into(),
            updated_at: updated,
            parent_id: Some("parent".into()),
            agent: agent.map(str::to_string),
        }
    }

    #[test]
    fn tag_parsing_requires_an_id_and_reads_case_insensitive_attributes() {
        let tag = parse_tag(
            "<SUBAGENT ID=\"ses_child\" Agent=\"build\" description=\"Run the tests\" state=\"completed\">\n</SUBAGENT>",
        )
        .unwrap();
        assert_eq!(
            tag,
            SubagentRef {
                id: "ses_child".into(),
                agent: "build".into(),
                description: "Run the tests".into(),
                state: "completed".into(),
            }
        );
        assert!(parse_tag("<subagent agent=\"build\"></subagent>").is_none());
        assert!(parse_tag("<subagents id=\"ses\">").is_none());
    }

    #[test]
    fn legacy_dispatch_parses_agent_and_prompt_lines() {
        let legacy =
            parse_legacy("agent=build\r\nprompt=Run the test suite and report failures\r\n")
                .unwrap();
        assert_eq!(legacy.agent, "build");
        assert_eq!(legacy.description, "Run the test suite and report failures");
        assert_eq!(legacy.id, "");
        assert!(parse_legacy("prompt=Missing agent").is_none());
    }

    #[test]
    fn explicit_child_ids_win_over_session_matching() {
        let reference = SubagentRef {
            id: "ses_explicit".into(),
            agent: String::new(),
            description: String::new(),
            state: String::new(),
        };
        assert_eq!(
            child_id(
                &reference,
                &[session("ses_other", "Other", None, 5)],
                "parent"
            ),
            "ses_explicit"
        );
    }

    #[test]
    fn legacy_child_matching_filters_parent_description_and_agent() {
        let reference = SubagentRef {
            id: String::new(),
            agent: "build".into(),
            description: "Run the tests".into(),
            state: String::new(),
        };
        let sessions = vec![
            session("older", "Run the tests first", Some("build"), 100),
            session("newer", "Run the tests again", Some("build"), 200),
            session("wrong-agent", "Run the tests", Some("plan"), 300),
            SessionSummary {
                parent_id: Some("elsewhere".into()),
                ..session("foreign", "Run the tests", Some("build"), 400)
            },
        ];
        assert_eq!(child_id(&reference, &sessions, "parent"), "newer");
        assert_eq!(child_id(&reference, &sessions, ""), "");
        let unknown = SubagentRef {
            agent: "review".into(),
            ..reference
        };
        assert_eq!(child_id(&unknown, &sessions, "parent"), "");
    }

    #[test]
    fn dispatch_signatures_match_tool_inputs_to_completion_records() {
        let reference = SubagentRef {
            id: "ses_child".into(),
            agent: "Build".into(),
            description: "Run   the tests".into(),
            state: "completed".into(),
        };
        let signature = ref_dispatch_signature(&reference).unwrap();
        assert_eq!(
            tool_dispatch_signature(
                "subagent",
                r#"{"agent":"build","description":"Run the tests"}"#
            )
            .as_deref(),
            Some(signature.as_str())
        );
        assert_eq!(
            tool_dispatch_signature(
                "task",
                r#"{"subagent_type":"build","prompt":"Run   the tests"}"#
            )
            .as_deref(),
            Some(signature.as_str())
        );
        assert!(tool_dispatch_signature("bash", r#"{"agent":"build"}"#).is_none());
        assert!(tool_dispatch_signature("task", "{").is_none());
    }

    #[test]
    fn task_tool_cards_prefer_metadata_and_fall_back_to_child_matching() {
        let agents = vec![
            AgentInfo {
                id: "build".into(),
                name: "Builder".into(),
            },
            AgentInfo {
                id: "review".into(),
                name: "Reviewer".into(),
            },
        ];
        let sessions = vec![
            session("ses_meta", "Metadata child", Some("review"), 10),
            session("ses_match", "Run the tests now", Some("build"), 20),
        ];
        let by_metadata = tool_card(
            "ses_meta",
            "task",
            r#"{"agent":"build","description":"Run the tests"}"#,
            ToolStatus::Running,
            &sessions,
            &agents,
            "parent",
        )
        .unwrap();
        assert_eq!(by_metadata.child_id, "ses_meta");
        assert_eq!(by_metadata.title, "Metadata child");
        assert_eq!(by_metadata.agent, "Builder");
        assert_eq!(by_metadata.tone, Tone::Working);

        let by_match = tool_card(
            "",
            "subagent",
            r#"{"subagent_type":"build","prompt":"Run the tests"}"#,
            ToolStatus::Success,
            &sessions,
            &agents,
            "parent",
        )
        .unwrap();
        assert_eq!(by_match.child_id, "ses_match");
        assert_eq!(by_match.title, "Run the tests now");
        assert_eq!(by_match.description, "Run the tests");
        assert_eq!(by_match.tone, Tone::Complete);
    }

    #[test]
    fn task_tool_cards_without_a_child_stay_visible_but_unlinked() {
        let card = tool_card(
            "",
            "task",
            r#"{"agent":"build","description":"Unknown child"}"#,
            ToolStatus::Failed,
            &[],
            &[],
            "parent",
        )
        .unwrap();
        assert_eq!(card.child_id, "");
        assert_eq!(card.title, "Build");
        assert_eq!(card.description, "Unknown child");
        assert_eq!(card.tone, Tone::Failed);
        assert!(
            tool_card(
                "",
                "bash",
                r#"{"command":"pwd"}"#,
                ToolStatus::Running,
                &[],
                &[],
                "parent"
            )
            .is_none()
        );
    }

    #[test]
    fn settled_task_status_prefers_terminal_synthetic_states() {
        assert_eq!(
            settled_status(ToolStatus::Running, "completed"),
            ToolStatus::Success
        );
        assert_eq!(
            settled_status(ToolStatus::Running, "cancelled"),
            ToolStatus::Failed
        );
        assert_eq!(
            settled_status(ToolStatus::Success, "running"),
            ToolStatus::Success
        );
    }

    #[test]
    fn card_prefers_resolved_titles_and_configured_agent_names() {
        let reference = SubagentRef {
            id: "ses_child".into(),
            agent: "build".into(),
            description: "Inspect the renderer".into(),
            state: "completed".into(),
        };
        let agents = vec![AgentInfo {
            id: "build".into(),
            name: "Builder".into(),
        }];
        let resolved_card = card(
            &reference,
            &[session(
                "ses_child",
                "Inspect the renderer",
                Some("build"),
                1,
            )],
            &agents,
            "parent",
        )
        .unwrap();
        assert_eq!(resolved_card.title, "Inspect the renderer");
        assert_eq!(resolved_card.agent, "Builder");
        assert_eq!(resolved_card.description, "Inspect the renderer");
        assert_eq!(resolved_card.tone, Tone::Complete);

        let stale = card(&reference, &[], &agents, "parent").unwrap();
        assert_eq!(stale.title, "Inspect the renderer");
        assert_eq!(stale.agent, "Builder");
        assert_eq!(stale.state, "completed");
    }

    #[test]
    fn unresolved_records_do_not_become_cards() {
        let reference = SubagentRef {
            id: String::new(),
            agent: "build".into(),
            description: "Unknown child".into(),
            state: String::new(),
        };
        assert!(card(&reference, &[], &[], "parent").is_none());
    }

    #[test]
    fn internal_system_reminders_are_hidden() {
        assert!(is_internal_reminder(
            "<system-reminder>\nprivate context\n</system-reminder>"
        ));
        assert!(is_internal_reminder(
            "<SYSTEM-REMINDER source=\"x\">\nprivate\n</SYSTEM-REMINDER>"
        ));
        assert!(!is_internal_reminder("<system-reminder>unterminated"));
        assert!(!is_internal_reminder(
            "<system-reminders>no</system-reminders>"
        ));
        assert!(!is_internal_reminder("ordinary text"));
    }
}
