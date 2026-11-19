//! Disclosure policy for a run's activity, independent of the native renderer.
//!
//! Groups and calls describe sibling rows. A call never acquires a new parent
//! when another call arrives or the run settles. Source order is spawn order;
//! status and completion timestamps never participate in identity or ordering.

use std::collections::HashMap;
use std::sync::Arc;

use crate::conversation::Conversation;
use crate::transcript::{Block, BlockKind, ToolStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Exploring,
    Changes,
    Verifying,
    Browser,
    Other,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Exploring => "Exploring",
            Self::Changes => "Changes",
            Self::Verifying => "Verifying",
            Self::Browser => "Browser activity",
            Self::Other => "Activity",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Exploring => "exploring",
            Self::Changes => "changes",
            Self::Verifying => "verifying",
            Self::Browser => "browser",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToolActivity {
    /// Canonical call/block identity, never a position or current status.
    pub id: Arc<str>,
    pub run_id: Arc<str>,
    pub group_id: Arc<str>,
    pub source_index: usize,
    pub category: Category,
    pub summary: Arc<str>,
    pub running: bool,
    pub failed: bool,
    pub warning: bool,
    pub subagent: bool,
    /// Failures, warnings, delegated agents, destructive/external operations
    /// and unknown non-shell tools stay outside collapsed summaries.
    pub mandatory: bool,
    /// A successful read/search/list with a known duration below 300 ms.
    /// It remains in this projection for explicit inspection.
    pub quiet_success: bool,
}

impl ToolActivity {
    /// Policy for a collapsed group. Explicit expansion reveals every call.
    /// Running calls and notable successful work remain visible until settled.
    pub fn visible(&self, expanded: bool) -> bool {
        expanded || self.mandatory || self.running
    }
}

#[derive(Clone, Debug)]
pub struct ActivityGroup {
    pub id: Arc<str>,
    pub run_id: Arc<str>,
    pub category: Category,
    pub first_source_index: usize,
    /// Stable source/spawn order, including quietly completed calls.
    pub tool_ids: Vec<Arc<str>>,
    pub running: usize,
    pub failed: usize,
    pub warnings: usize,
    pub mandatory: usize,
    pub quiet_success: usize,
}

#[derive(Clone, Debug, Default)]
pub struct ActivityProjection {
    /// First occurrence order. Each run/category exists from its first call.
    pub groups: Vec<ActivityGroup>,
    /// Source order, independent of completion order.
    pub tools: Vec<ToolActivity>,
}

impl ActivityProjection {
    pub fn tool_at(&self, source_index: usize) -> Option<&ToolActivity> {
        self.tools
            .binary_search_by_key(&source_index, |tool| tool.source_index)
            .ok()
            .map(|index| &self.tools[index])
    }
}

#[derive(Clone, Debug)]
pub struct ToolPolicy {
    pub category: Category,
    pub summary: Arc<str>,
    pub running: bool,
    pub failed: bool,
    pub warning: bool,
    pub subagent: bool,
    pub mandatory: bool,
    pub quiet_success: bool,
}

/// Retain only the current revision's shared payloads and small policy values.
/// Unchanged successful output is never searched again on a later text delta.
#[derive(Default)]
pub struct ActivityCache {
    tools: HashMap<Arc<str>, CachedTool>,
}

struct CachedTool {
    input: Arc<str>,
    name: Arc<str>,
    output: Arc<str>,
    progress: Arc<str>,
    status: ToolStatus,
    streaming: bool,
    started: i64,
    completed: Option<i64>,
    child: Option<Arc<str>>,
    nested_failed: bool,
    policy: ToolPolicy,
}

impl CachedTool {
    fn matches(&self, block: &Block, nested_failed: bool) -> bool {
        Arc::ptr_eq(&self.input, &block.text)
            && Arc::ptr_eq(&self.name, &block.detail)
            && Arc::ptr_eq(&self.output, &block.output)
            && Arc::ptr_eq(&self.progress, &block.progress)
            && self.status == block.status
            && self.streaming == block.streaming
            && self.started == block.created_at
            && self.completed == block.completed_at
            && self.child == block.child_session_id
            && self.nested_failed == nested_failed
    }
}

impl ActivityCache {
    pub fn project(&mut self, conversation: &Conversation) -> ActivityProjection {
        let mut projection = ActivityProjection::default();
        let mut group_indices = HashMap::new();
        let mut retained = HashMap::new();
        for node in &conversation.nodes {
            let block = &node.block;
            if !matches!(block.kind, BlockKind::Tool | BlockKind::Shell) {
                continue;
            }
            let nested_failed = nested_failure(block);
            let cached = self
                .tools
                .remove(&block.id)
                .filter(|cached| cached.matches(block, nested_failed));
            let cached = cached.unwrap_or_else(|| CachedTool {
                input: block.text.clone(),
                name: block.detail.clone(),
                output: block.output.clone(),
                progress: block.progress.clone(),
                status: block.status,
                streaming: block.streaming,
                started: block.created_at,
                completed: block.completed_at,
                child: block.child_session_id.clone(),
                nested_failed,
                policy: policy_with_nested(block, nested_failed),
            });
            let policy = &cached.policy;
            let run_id = &conversation.turns[node.turn_index].id;
            // The length prefix makes arbitrary run IDs unambiguous.
            let group_id: Arc<str> = format!(
                "activity:{}:{run_id}:{}",
                run_id.len(),
                policy.category.key()
            )
            .into();
            let index = *group_indices.entry(group_id.clone()).or_insert_with(|| {
                let index = projection.groups.len();
                projection.groups.push(ActivityGroup {
                    id: group_id.clone(),
                    run_id: run_id.clone(),
                    category: policy.category,
                    first_source_index: node.source_index,
                    tool_ids: Vec::new(),
                    running: 0,
                    failed: 0,
                    warnings: 0,
                    mandatory: 0,
                    quiet_success: 0,
                });
                index
            });
            let group = &mut projection.groups[index];
            group.tool_ids.push(block.id.clone());
            group.running += usize::from(policy.running);
            group.failed += usize::from(policy.failed);
            group.warnings += usize::from(policy.warning);
            group.mandatory += usize::from(policy.mandatory);
            group.quiet_success += usize::from(policy.quiet_success);
            projection.tools.push(ToolActivity {
                id: block.id.clone(),
                run_id: run_id.clone(),
                group_id,
                source_index: node.source_index,
                category: policy.category,
                summary: policy.summary.clone(),
                running: policy.running,
                failed: policy.failed,
                warning: policy.warning,
                subagent: policy.subagent,
                mandatory: policy.mandatory,
                quiet_success: policy.quiet_success,
            });
            retained.insert(block.id.clone(), cached);
        }
        self.tools = retained;
        projection
    }
}

/// Pure one-shot projection. Long-lived consumers should retain ActivityCache.
pub fn project(conversation: &Conversation) -> ActivityProjection {
    ActivityCache::default().project(conversation)
}

/// Presentation for one call. Never uses raw JSON or accumulated output as its
/// summary. A caller rendering repeatedly should use ActivityCache instead.
pub fn tool(block: &Block) -> ToolPolicy {
    policy_with_nested(block, nested_failure(block))
}

fn nested_failure(block: &Block) -> bool {
    let mut pending = block.sub_calls.iter().collect::<Vec<_>>();
    while let Some(call) = pending.pop() {
        if call.status == ToolStatus::Failed {
            return true;
        }
        pending.extend(call.sub_calls.iter());
    }
    false
}

fn warning(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        [
            "warning:",
            "warning ",
            "warn:",
            "[warning]",
            "[warn]",
            "npm warn",
            "approval required",
            "permission required",
        ]
        .iter()
        .any(|prefix| {
            line.as_bytes()
                .get(..prefix.len())
                .is_some_and(|value| value.eq_ignore_ascii_case(prefix.as_bytes()))
        })
    })
}

fn notable_shell(command: &str, routine: bool) -> bool {
    if routine {
        return false;
    }
    // This controls presentation, never execution permissions. Unsupported
    // scripts/loops still have a visible shared summary and inspectable details;
    // uncertainty alone must not turn every successful probe into a status row.
    let words = command
        .split(|ch: char| ch.is_whitespace() || ";|&()$`".contains(ch))
        .map(|word| word.trim_matches(['\'', '"']))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.iter().any(|word| {
        matches!(
            *word,
            "rm" | "rmdir"
                | "mv"
                | "truncate"
                | "shred"
                | "dd"
                | "chmod"
                | "chown"
                | "deploy"
                | "publish"
                | "send"
                | "send_email"
                | "upload"
                | "-delete"
                | "-exec"
                | "-execdir"
                | "curl"
                | "wget"
                | "ssh"
                | "scp"
                | "sftp"
                | "rsync"
                | "gh"
                | "aws"
                | "gcloud"
                | "az"
                | "kubectl"
                | "tee"
        )
    }) {
        return true;
    }
    if words.contains(&"git")
        && words.iter().any(|word| {
            matches!(
                *word,
                "push" | "reset" | "clean" | "restore" | "checkout" | "revert"
            ) || (words.contains(&"branch") && matches!(*word, "-d" | "-D" | "--delete"))
        })
    {
        return true;
    }
    if words.iter().any(|word| word.starts_with("--output")) {
        return true;
    }
    command.match_indices('>').any(|(index, _)| {
        let target = command[index + 1..].trim_start_matches(['>', ' ', '\t', '\'', '"']);
        !["/dev/null", "&1", "&2"].iter().any(|prefix| {
            target.strip_prefix(prefix).is_some_and(|rest| {
                rest.is_empty() || rest.starts_with([';', '|', '&', ' ', '\n', '\'', '"'])
            })
        })
    })
}

fn brief(text: &str) -> String {
    let mut result = String::new();
    for word in text.split_whitespace() {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(word);
        if result.chars().count() > 160 {
            result = result.chars().take(157).collect();
            result.push('…');
            break;
        }
    }
    result
}

fn policy_with_nested(block: &Block, nested_failed: bool) -> ToolPolicy {
    let name = block.detail.to_ascii_lowercase();
    let name = name.rsplit(['.', '/']).next().unwrap_or(&name);
    // Inputs are small in normal operation. Keep malformed/unbounded streamed
    // input from becoming parser work on every publication. Never parse output.
    let input = (block.text.len() <= 128 * 1024)
        .then(|| serde_json::from_str::<serde_json::Value>(&block.text).ok())
        .flatten();
    let field = |names: &[&str]| -> &str {
        names
            .iter()
            .find_map(|name| input.as_ref()?.get(*name)?.as_str())
            .unwrap_or("")
    };
    let path = field(&["filePath", "file_path", "path", "directory"]);
    let query = field(&["query", "pattern"]);
    let command = field(&["command", "cmd"]);
    let shell = matches!(
        name,
        "bash" | "shell" | "terminal" | "exec_command" | "run_code" | "code"
    );
    let shell_parts = command_parts(command);
    let safe_shell = shell_parts.as_ref().is_some_and(|parts| {
        parts
            .iter()
            .all(|part| verification_command(part) || read_command(part))
    });
    let subagent =
        matches!(name, "task" | "subagent" | "spawn_agent") || block.child_session_id.is_some();
    let category = match name {
        "read" | "read_file" | "read_image" | "grep" | "rg" | "search_code" | "search_files"
        | "glob" | "list" | "list_files" | "ls" => Category::Exploring,
        "write" | "write_file" | "edit" | "edit_file" | "patch" | "apply_patch" => {
            Category::Changes
        }
        "test" | "lint" | "build" | "typecheck" | "check" => Category::Verifying,
        "browser" | "navigate" | "navigation" | "browser_navigate" | "websearch" | "web_search"
        | "webfetch" | "web_fetch" | "search" => Category::Browser,
        "bash" | "shell" | "terminal" | "exec_command" | "run_code" | "code"
            if shell_parts
                .as_ref()
                .is_some_and(|parts| parts.iter().any(|part| verification_command(part))) =>
        {
            Category::Verifying
        }
        "bash" | "shell" | "terminal" | "exec_command" | "run_code" | "code"
            if shell_parts
                .as_ref()
                .is_some_and(|parts| parts.iter().all(|part| read_command(part))) =>
        {
            Category::Exploring
        }
        _ => Category::Other,
    };
    let running =
        block.streaming || matches!(block.status, ToolStatus::Streaming | ToolStatus::Running);
    let failed = block.status == ToolStatus::Failed || nested_failed;
    // During execution the shared activity shows progress. Scan terminal output only
    // when it settles, then reuse that decision on unrelated prose frames.
    let warning = warning(&block.progress) || (!running && warning(&block.output));
    // Routine probes and successful scripts share the work disclosure. Known
    // destructive/external actions, actual diagnostics and failures surface.
    let operation = field(&["operation", "action"]).to_ascii_lowercase();
    let destructive_operation = matches!(
        operation.as_str(),
        "delete" | "remove" | "unlink" | "delete_file" | "remove_file" | "truncate"
    );
    let patch = input
        .as_ref()
        .and_then(serde_json::Value::as_str)
        .filter(|patch| !patch.is_empty())
        .unwrap_or_else(|| field(&["patch", "patchText", "patch_text", "input", "diff"]));
    let raw_patch = if block.text.trim_start().starts_with("*** Begin Patch") {
        block.text.as_ref()
    } else {
        patch
    };
    let deletes_file = raw_patch.lines().any(|line| {
        let line = line.trim_start();
        line.get(.."*** Delete File:".len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("*** Delete File:"))
            || line.starts_with("+++ /dev/null")
            || line.starts_with("deleted file mode ")
    });
    let understood_edit = match name {
        "patch" | "apply_patch" => !raw_patch.is_empty(),
        _ => input.as_ref().is_some_and(serde_json::Value::is_object) && !path.is_empty(),
    };
    let side_effects = if shell {
        notable_shell(command, safe_shell)
    } else {
        match category {
            Category::Exploring => false,
            Category::Verifying => !command.is_empty() && !safe_shell,
            Category::Changes => !understood_edit || deletes_file || destructive_operation,
            // A browser can submit forms or invoke external services; an
            // unknown tool must not be silently treated as a local operation.
            Category::Browser | Category::Other => true,
        }
    };
    let mandatory = failed || warning || side_effects || subagent;
    let quiet_success = category == Category::Exploring
        && block.status == ToolStatus::Success
        && !running
        && !mandatory
        && block.completed_at.is_some_and(|end| {
            end >= block.created_at && end.saturating_sub(block.created_at) < 300
        });
    let detail = match category {
        Category::Exploring if shell => command,
        Category::Exploring if !path.is_empty() => path,
        Category::Exploring => query,
        Category::Changes => path,
        Category::Verifying => command,
        Category::Browser => field(&["url", "query", "address"]),
        Category::Other if subagent => field(&["description", "prompt", "agent", "subagent_type"]),
        Category::Other => command,
    };
    let label = if subagent {
        "Agent"
    } else {
        match category {
            Category::Exploring => match name {
                "grep" | "rg" | "search_code" | "search_files" => "Search",
                "glob" | "list" | "list_files" | "ls" => "List files",
                _ if shell => "Explore",
                _ => "Read",
            },
            Category::Changes => "Change",
            Category::Verifying => "Verify",
            Category::Browser => "Browser",
            Category::Other if command.is_empty() => "Tool activity",
            Category::Other => "Run",
        }
    };
    let detail = brief(detail);
    let summary = if shell {
        if failed {
            "Command failed".to_owned()
        } else if warning {
            "Command reported a warning".to_owned()
        } else {
            match category {
                Category::Exploring => "Read files",
                Category::Verifying => "Ran checks",
                _ if command.trim_start().starts_with("git push") => "Push changes to remote",
                _ if command.trim_start().starts_with("rm ") => "Delete files",
                _ => "Ran a command",
            }
            .to_owned()
        }
    } else if detail.is_empty() {
        label.to_owned()
    } else {
        format!("{label} {detail}")
    };
    ToolPolicy {
        category,
        summary: summary.into(),
        running,
        failed,
        warning,
        subagent,
        mandatory,
        quiet_success,
    }
}

fn verification_command(command: &str) -> bool {
    let command = command.trim_start();
    [
        "cargo test",
        "cargo clippy",
        "cargo check",
        "cargo build",
        "npm test",
        "npm run test",
        "npm run lint",
        "npm run build",
        "npm run typecheck",
        "pnpm test",
        "pnpm run test",
        "pnpm run lint",
        "pnpm run build",
        "pnpm run typecheck",
        "pnpm lint",
        "pnpm build",
        "pnpm typecheck",
        "yarn test",
        "yarn lint",
        "yarn build",
        "yarn typecheck",
        "pytest",
        "python -m pytest",
        "python3 -m pytest",
        "go test",
        "bash scripts/verify.sh",
    ]
    .iter()
    .any(|prefix| {
        command
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.is_empty() || tail.starts_with(char::is_whitespace))
    })
}

/// Presentation classification only; this never authorizes command execution.
/// Keep quoted separators as arguments, and reject expansion/redirection or
/// background jobs instead of pretending to interpret a shell program.
fn command_parts(command: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut chars = command.chars().peekable();
    while let Some(ch) = chars.next() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if quote == Some('\'') {
            if ch == '\'' {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if matches!(ch, '$' | '`') {
            return None;
        }
        if quote == Some('"') {
            if ch == '"' {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            continue;
        }
        if matches!(ch, '<' | '>' | '(' | ')') {
            return None;
        }
        if matches!(ch, ';' | '\n' | '\r' | '|' | '&') {
            if ch == '&' && chars.peek() != Some(&'&') {
                return None;
            }
            if matches!(ch, '&' | '|') && chars.peek() == Some(&ch) {
                chars.next();
            }
            if !current.trim().is_empty() {
                parts.push(current.trim().to_owned());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }
    if quote.is_some() || escaped {
        return None;
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_owned());
    }
    (!parts.is_empty()).then_some(parts)
}

fn read_command(command: &str) -> bool {
    let mut words = command.split_whitespace();
    match words.next() {
        Some("rg") => !words.any(|word| {
            word == "--pre"
                || word.starts_with("--pre=")
                || word == "--hostname-bin"
                || word.starts_with("--hostname-bin=")
        }),
        Some("grep" | "ls" | "cat" | "head" | "tail" | "pwd" | "wc" | "echo" | "printf") => true,
        Some("find") => !words.any(|word| {
            matches!(
                word,
                "-delete"
                    | "-exec"
                    | "-execdir"
                    | "-ok"
                    | "-okdir"
                    | "-fprint"
                    | "-fprint0"
                    | "-fprintf"
                    | "-fls"
            )
        }),
        Some("git") => {
            matches!(
                words.next(),
                Some("status" | "diff" | "log" | "show" | "rev-parse" | "ls-files")
            ) && !words.any(|word| {
                word.starts_with("--output") || matches!(word, "--ext-diff" | "--textconv")
            })
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation;

    fn block(id: &str, name: &str, status: ToolStatus) -> Block {
        Block {
            id: id.into(),
            message_id: "assistant".into(),
            part_id: id.into(),
            kind: BlockKind::Tool,
            text: r#"{"path":"src/lib.rs"}"#.into(),
            detail: name.into(),
            output: "".into(),
            progress: "".into(),
            interrupted: false,
            status,
            streaming: false,
            agent: None,
            model: None,
            created_at: 100,
            completed_at: None,
            child_session_id: None,
            files: Vec::new(),
            sub_calls: Vec::new(),
            attachments: Vec::new(),
        }
    }

    #[test]
    fn hundred_twenty_calls_share_a_group_from_the_first_call_and_keep_spawn_order() {
        let mut blocks = vec![block("call-0", "read", ToolStatus::Running)];
        let mut cache = ActivityCache::default();
        let first = cache.project(&conversation::project(&blocks));
        assert_eq!(first.groups.len(), 1);
        for index in 1..120 {
            blocks.push(block(&format!("call-{index}"), "read", ToolStatus::Running));
        }
        for index in (0..120).rev() {
            blocks[index].status = ToolStatus::Success;
            blocks[index].completed_at = Some(200);
        }
        let settled = cache.project(&conversation::project(&blocks));
        assert_eq!(settled.groups.len(), 1);
        assert_eq!(settled.groups[0].id, first.groups[0].id);
        assert_eq!(settled.groups[0].quiet_success, 120);
        assert_eq!(settled.groups[0].running, 0);
        for (index, tool) in settled.tools.iter().enumerate() {
            assert_eq!(tool.id.as_ref(), format!("call-{index}"));
            assert_eq!(tool.group_id, first.tools[0].group_id);
            assert!(!tool.visible(false));
            assert!(tool.visible(true));
            assert_eq!(settled.tool_at(index).unwrap().id, tool.id);
        }
    }

    #[test]
    fn out_of_order_subagent_completion_updates_existing_spawned_rows() {
        let mut blocks = vec![
            block("spawn-first", "task", ToolStatus::Running),
            block("spawn-second", "task", ToolStatus::Running),
        ];
        blocks[0].child_session_id = Some("child-first".into());
        blocks[1].child_session_id = Some("child-second".into());
        let first = project(&conversation::project(&blocks));
        blocks[1].status = ToolStatus::Success;
        blocks[1].completed_at = Some(150);
        let second = project(&conversation::project(&blocks));
        blocks[0].status = ToolStatus::Success;
        blocks[0].completed_at = Some(250);
        let final_state = project(&conversation::project(&blocks));
        for state in [&second, &final_state] {
            assert_eq!(
                state.tools.iter().map(|tool| &tool.id).collect::<Vec<_>>(),
                first.tools.iter().map(|tool| &tool.id).collect::<Vec<_>>()
            );
            assert!(
                state
                    .tools
                    .iter()
                    .all(|tool| tool.subagent && tool.visible(false))
            );
            assert_eq!(state.groups[0].id, first.groups[0].id);
        }
    }

    #[test]
    fn failures_warnings_and_unknown_side_effects_cannot_be_quiet() {
        let mut warning = block("warning", "read", ToolStatus::Success);
        warning.output = format!(
            "{}\nWARNING: incomplete result\n{}",
            "x".repeat(20_000),
            "x".repeat(20_000)
        )
        .into();
        warning.completed_at = Some(101);
        let mut failed = block("failed", "grep", ToolStatus::Failed);
        failed.completed_at = Some(101);
        for block in [
            warning,
            failed,
            block("external", "send_email", ToolStatus::Success),
            block("browser", "browser_navigate", ToolStatus::Success),
        ] {
            let policy = tool(&block);
            assert!(policy.mandatory, "{} must remain surfaced", block.id);
            assert!(!policy.quiet_success);
        }
    }

    #[test]
    fn quiet_policy_requires_known_success_duration_and_no_warning() {
        let mut read = block("read", "read", ToolStatus::Success);
        assert!(!tool(&read).quiet_success);
        read.completed_at = Some(400);
        assert!(!tool(&read).quiet_success);
        read.completed_at = Some(399);
        assert!(tool(&read).quiet_success);
        read.progress = "Permission required to continue".into();
        assert!(tool(&read).mandatory);
        read.progress = "".into();
        read.status = ToolStatus::Running;
        assert!(!tool(&read).quiet_success);
    }

    #[test]
    fn source_mentions_and_filenames_are_not_runtime_warnings() {
        let mut call = block("read-source", "bash", ToolStatus::Success);
        call.text = r#"{"command":"cat src/app.js; cat src/routes.js"}"#.into();
        call.output = "warnings.md\nconst warning = 'warning: text';\nconsole.warn('warning');\n// warning colors\n".into();
        assert!(!tool(&call).warning);
        assert!(!tool(&call).mandatory);
        call.output = "warning: compilation has an unused import\n".into();
        assert!(tool(&call).warning);
        assert!(tool(&call).mandatory);
    }

    #[test]
    fn cache_reuses_summary_and_releases_removed_payloads() {
        let blocks = vec![block("read", "read", ToolStatus::Success)];
        let conversation = conversation::project(&blocks);
        let mut cache = ActivityCache::default();
        let first = cache.project(&conversation);
        let second = cache.project(&conversation);
        assert!(Arc::ptr_eq(
            &first.tools[0].summary,
            &second.tools[0].summary
        ));
        cache.project(&Conversation::default());
        assert!(cache.tools.is_empty());
    }

    #[test]
    fn category_ids_are_independent_of_count_status_and_separate_turns() {
        let mut user = block("user", "", ToolStatus::None);
        user.kind = BlockKind::User;
        user.message_id = "user-one".into();
        let mut user_two = user.clone();
        user_two.id = "user-two".into();
        user_two.message_id = "user-two".into();
        let blocks = [
            user,
            block("read-one", "read", ToolStatus::Success),
            block("edit", "edit", ToolStatus::Success),
            user_two,
            block("read-two", "read", ToolStatus::Running),
        ];
        let activity = project(&conversation::project(&blocks));
        assert_eq!(activity.groups.len(), 3);
        assert_ne!(activity.tools[0].group_id, activity.tools[2].group_id);
        assert_eq!(activity.tools[0].category, Category::Exploring);
        assert_eq!(activity.tools[1].category, Category::Changes);
        assert_eq!(activity.tools[2].run_id.as_ref(), "user-two");
    }

    #[test]
    fn summaries_never_expose_raw_streamed_json_or_unbounded_lines() {
        let mut call = block("input", "read", ToolStatus::Streaming);
        call.text = r#"{"path":"unfinished"#.into();
        assert_eq!(tool(&call).summary.as_ref(), "Read");
        call.text = serde_json::json!({"path": "a".repeat(500)})
            .to_string()
            .into();
        assert!(tool(&call).summary.chars().count() < 170);
        call.detail = "exec_command".into();
        call.text = r#"{"command":"cargo test -p vela-core"}"#.into();
        assert_eq!(tool(&call).category, Category::Verifying);
        assert!(
            !tool(&call).mandatory,
            "a familiar local check belongs in its summary"
        );
    }

    #[test]
    fn hundred_reversible_edits_collapse_to_one_summary_with_all_details_retained() {
        let blocks = (0..100).map(|index| {
            let mut edit = block(&format!("edit-{index}"), "apply_patch", ToolStatus::Success);
            edit.text = serde_json::json!({"patch": format!(
                "*** Begin Patch\n*** Update File: src/file-{index}.rs\n@@\n-old\n+new\n*** End Patch"
            )}).to_string().into();
            edit
        }).collect::<Vec<_>>();
        let projection = project(&conversation::project(&blocks));
        assert_eq!(projection.groups.len(), 1);
        assert_eq!(projection.groups[0].category, Category::Changes);
        assert_eq!(projection.groups[0].tool_ids.len(), 100);
        assert_eq!(projection.groups[0].mandatory, 0);
        assert_eq!(
            projection
                .tools
                .iter()
                .filter(|tool| tool.visible(false))
                .count(),
            0
        );
        assert!(projection.tools.iter().all(|tool| tool.visible(true)));
    }

    #[test]
    fn familiar_local_commands_fold_but_operators_destructive_and_external_calls_surface() {
        for command in [
            "cargo test -p vela-core",
            "cargo clippy --workspace",
            "npm run build",
            "pnpm run typecheck",
            "bash scripts/verify.sh",
            "rg --files src",
            "git status --short",
            "git diff --stat",
            "find src -name '*.rs'",
            "ls -la back-end docs assets; cat package.json; cat .gitignore",
            "find back-end/src docs/js -maxdepth 3 -type f | head -50; echo '---'; cat package.json",
            "cat back-end/src/app.js; echo '--- routes'; cat back-end/src/routes/chatRoutes.js",
            "find assets -maxdepth 3 | head -40; echo '--- git'; git log --oneline -15; git status --short | head",
            "wc -l docs/js/main.js docs/index.html docs/css/styles.scss; head -40 docs/index.html",
            "grep -rn 'fetch\\|/api/chat\\|EventSource' docs --include='*.js' --include='*.html' | head",
            "cargo test && cargo clippy --workspace",
            "git branch -a",
            "wc -l docs/*.js 2>/dev/null; echo '---'; head -60 docs/main.js",
            "for f in $(find assets/backups -name main.js | head -3); do echo \"$f: $(wc -l < \"$f\") lines\"; grep -c fetch \"$f\"; done",
            "unknown-check --all",
        ] {
            let mut call = block("local", "bash", ToolStatus::Success);
            call.text = serde_json::json!({"command": command}).to_string().into();
            assert!(
                !tool(&call).mandatory,
                "local operation should fold: {command}"
            );
            assert!(
                !tool(&call).summary.contains(command),
                "raw commands are disclosure details"
            );
        }
        for command in [
            "rm -rf generated",
            "git reset --hard",
            "git clean -fd",
            "git push origin main",
            "npm publish",
            "deploy production",
            "send report",
            "cargo test && git push",
            "cargo test; rm data",
            "cargo test $(send report)",
            "rg --files > inventory.txt",
            "find src -delete",
            "find src -exec rm {} +",
            "git diff --output=changed.patch",
            "ls -la; 'rm' -rf generated",
            "find src '-exec' rm '{}' +",
            "cat $(rm source)",
            "curl -X POST https://example.invalid/api -d payload",
            "scp report.txt example.invalid:/reports/",
            "gh release create v1.0",
            "git -C other push origin main",
            "git branch -D obsolete",
        ] {
            let mut call = block("external", "bash", ToolStatus::Success);
            call.text = serde_json::json!({"command": command}).to_string().into();
            assert!(tool(&call).mandatory, "operation must surface: {command}");
        }
        for name in [
            "delete_file",
            "send_email",
            "publish",
            "browser_submit_form",
            "custom_tool",
        ] {
            assert!(
                tool(&block("external", name, ToolStatus::Success)).mandatory,
                "{name}"
            );
        }
        let mut deletion = block("delete", "apply_patch", ToolStatus::Success);
        deletion.text = serde_json::json!({"patchText":
            "*** Begin Patch\n*** Delete File: obsolete.rs\n*** End Patch"
        })
        .to_string()
        .into();
        assert!(tool(&deletion).mandatory);
        deletion.detail = "edit".into();
        deletion.text = r#"{"operation":"delete","path":"obsolete.rs"}"#.into();
        assert!(tool(&deletion).mandatory);
    }
}
