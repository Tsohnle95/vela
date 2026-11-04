//! Provider usage snapshots written by the desktop credential bridge.
//!
//! Vela does not read provider OAuth tokens here. It consumes the short-lived
//! snapshot document that the usage bridge writes after refreshing live
//! windows/credits, mirroring the reference fallback path and 15-minute age cap.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SNAPSHOT_MAX_AGE_MS: u64 = 15 * 60 * 1000;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub used_percent: f64,
    #[serde(default)]
    pub window_minutes: Option<u64>,
    #[serde(default)]
    pub resets_at: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageCredits {
    #[serde(default)]
    pub has_credits: bool,
    #[serde(default)]
    pub unlimited: bool,
    #[serde(default)]
    pub balance: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub total: Option<f64>,
    #[serde(default)]
    pub used: Option<f64>,
    #[serde(default)]
    pub remaining: Option<f64>,
    #[serde(default)]
    pub overage_permitted: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageSnapshot {
    #[serde(default)]
    pub windows: Vec<UsageWindow>,
    #[serde(default)]
    pub credits: Option<ProviderUsageCredits>,
    #[serde(default)]
    pub plan_type: Option<String>,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub retryable: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageResult {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub snapshot: Option<ProviderUsageSnapshot>,
    #[serde(default)]
    pub error: Option<ProviderUsageError>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotFile {
    #[serde(default)]
    generated_at: u64,
    #[serde(default)]
    results: Vec<ProviderUsageResult>,
}

pub fn snapshot_candidates() -> Vec<PathBuf> {
    snapshot_candidates_from(
        std::env::var_os("VELA_USAGE_SNAPSHOT"),
        std::env::var_os("XDG_DATA_HOME"),
        std::env::var_os("HOME"),
    )
}

fn snapshot_candidates_from(
    override_path: Option<std::ffi::OsString>,
    xdg_data_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Vec<PathBuf> {
    if let Some(path) = override_path.filter(|path| !path.is_empty()) {
        return vec![PathBuf::from(path)];
    }
    let data = xdg_data_home
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|home| Path::new(home).join(".local/share")));
    let mut candidates = Vec::new();
    if let Some(data) = data {
        candidates.push(data.join("opencode/vela-usage.json"));
    }
    if let Some(home) = home {
        candidates.push(
            Path::new(&home)
                .join("Library/Application Support/ai.opencode.desktop/opencode/vela-usage.json"),
        );
    }
    candidates
}

/// Return the newest usable snapshot. Unreadable, malformed and stale files are
/// skipped so the next candidate can still supply usage.
pub fn read_provider_usage_snapshot(
    candidates: &[PathBuf],
    now_ms: u64,
) -> Vec<ProviderUsageResult> {
    let mut newest: Option<(u64, Vec<ProviderUsageResult>)> = None;
    for candidate in candidates {
        let Ok(raw) = std::fs::read_to_string(candidate) else {
            continue;
        };
        let Ok(snapshot) = serde_json::from_str::<SnapshotFile>(&raw) else {
            continue;
        };
        if snapshot.generated_at == 0
            || now_ms.saturating_sub(snapshot.generated_at) > SNAPSHOT_MAX_AGE_MS
        {
            continue;
        }
        let results = snapshot
            .results
            .into_iter()
            .filter(|result| !result.provider.trim().is_empty())
            .collect::<Vec<_>>();
        if results.is_empty() {
            continue;
        }
        if newest
            .as_ref()
            .is_none_or(|(generated_at, _)| snapshot.generated_at > *generated_at)
        {
            newest = Some((snapshot.generated_at, results));
        }
    }
    newest.map(|(_, results)| results).unwrap_or_default()
}

pub fn read_default_provider_usage_snapshot() -> Vec<ProviderUsageResult> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64;
    read_provider_usage_snapshot(&snapshot_candidates(), now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write_snapshot(root: &Path, generated_at: u64, results: serde_json::Value) -> PathBuf {
        let path = root.join("vela-usage.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&json!({
                "generatedAt": generated_at,
                "results": results
            }))
            .unwrap(),
        )
        .unwrap();
        path
    }

    #[test]
    fn snapshot_candidates_honor_override_and_environment_fallbacks() {
        use std::ffi::OsString;
        let override_path = PathBuf::from("/tmp/custom-usage.json");
        assert_eq!(
            snapshot_candidates_from(
                Some(override_path.clone().into_os_string()),
                Some(OsString::from("/xdg")),
                Some(OsString::from("/home")),
            ),
            vec![override_path],
            "the explicit snapshot override replaces the default candidates"
        );
        assert_eq!(
            snapshot_candidates_from(
                None,
                Some(OsString::from("/xdg")),
                Some(OsString::from("/home")),
            ),
            vec![
                PathBuf::from("/xdg/opencode/vela-usage.json"),
                PathBuf::from(
                    "/home/Library/Application Support/ai.opencode.desktop/opencode/vela-usage.json"
                ),
            ]
        );
        assert_eq!(
            snapshot_candidates_from(None, None, Some(OsString::from("/home"))),
            vec![
                PathBuf::from("/home/.local/share/opencode/vela-usage.json"),
                PathBuf::from(
                    "/home/Library/Application Support/ai.opencode.desktop/opencode/vela-usage.json"
                ),
            ]
        );
        assert_eq!(
            snapshot_candidates_from(
                Some(OsString::new()),
                Some(OsString::from("/xdg")),
                None,
            ),
            vec![PathBuf::from("/xdg/opencode/vela-usage.json")],
            "an empty override falls back to the environment candidates"
        );
        assert!(snapshot_candidates_from(None, None, None).is_empty());
    }

    #[test]
    fn snapshot_parser_reads_windows_credits_and_provider_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_snapshot(
            dir.path(),
            1_000,
            json!([
                {
                    "provider": "openai",
                    "displayName": "OpenAI",
                    "status": "ok",
                    "snapshot": {
                        "windows": [
                            {"id":"5h","label":"5h","usedPercent":42.5,"windowMinutes":300,"resetsAt":1234},
                            {"id":"weekly","label":"Weekly","usedPercent":91,"windowMinutes":10080,"resetsAt":null}
                        ],
                        "credits": {
                            "hasCredits": true,
                            "unlimited": false,
                            "balance": null,
                            "label": "Credits",
                            "total": 1000,
                            "used": 250,
                            "remaining": 750
                        },
                        "planType": "self_serve_business",
                        "updatedAt": 999
                    }
                },
                {
                    "provider": "copilot",
                    "displayName": "GitHub Copilot",
                    "status": "unauthenticated",
                    "snapshot": null,
                    "error": {"code":"missing_oauth","message":"GitHub Copilot is not authenticated.","retryable":false}
                },
                {"provider": "  ", "displayName": "ignored"},
                {"displayName": "no provider"}
            ]),
        );
        let results = read_provider_usage_snapshot(&[path], 1_000 + 60_000);
        assert_eq!(results.len(), 2);
        let openai = &results[0];
        assert_eq!(openai.display_name, "OpenAI");
        let snapshot = openai.snapshot.as_ref().unwrap();
        assert_eq!(snapshot.windows.len(), 2);
        assert_eq!(snapshot.windows[0].used_percent, 42.5);
        assert_eq!(snapshot.windows[1].window_minutes, Some(10080));
        assert_eq!(snapshot.credits.as_ref().unwrap().remaining, Some(750.0));
        assert_eq!(snapshot.plan_type.as_deref(), Some("self_serve_business"));
        assert_eq!(
            results[1].error.as_ref().unwrap().code,
            "missing_oauth"
        );
    }

    #[test]
    fn snapshot_parser_selects_the_newest_valid_candidate() {
        let older_dir = tempfile::tempdir().unwrap();
        let older = write_snapshot(
            older_dir.path(),
            100,
            json!([{"provider": "older", "displayName": "Older"}]),
        );
        let newer_dir = tempfile::tempdir().unwrap();
        let newer = write_snapshot(
            newer_dir.path(),
            200,
            json!([{"provider": "newer", "displayName": "Newer"}]),
        );
        for candidates in [
            vec![older.clone(), newer.clone()],
            vec![newer.clone(), older.clone()],
        ] {
            let results = read_provider_usage_snapshot(&candidates, 300);
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].provider, "newer");
        }

        // A malformed or empty candidate never displaces a valid older one.
        let malformed_dir = tempfile::tempdir().unwrap();
        let malformed = malformed_dir.path().join("vela-usage.json");
        std::fs::write(&malformed, "{broken").unwrap();
        let empty_dir = tempfile::tempdir().unwrap();
        let empty = write_snapshot(empty_dir.path(), 400, json!([]));
        let results = read_provider_usage_snapshot(&[malformed, empty, older], 500);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "older");
    }

    #[test]
    fn snapshot_parser_ignores_stale_malformed_and_empty_documents() {
        let dir = tempfile::tempdir().unwrap();
        let stale = write_snapshot(
            dir.path(),
            SNAPSHOT_MAX_AGE_MS,
            json!([{"provider": "stale", "displayName": "Stale"}]),
        );
        assert!(
            read_provider_usage_snapshot(
                std::slice::from_ref(&stale),
                SNAPSHOT_MAX_AGE_MS * 2 + 1,
            )
            .is_empty()
        );

        let malformed = dir.path().join("bad.json");
        std::fs::write(&malformed, "not json").unwrap();
        assert!(read_provider_usage_snapshot(&[malformed], 1).is_empty());

        let empty = write_snapshot(dir.path(), 100, json!([]));
        assert!(read_provider_usage_snapshot(&[empty], 100).is_empty());

        let valid = write_snapshot(
            dir.path(),
            100,
            json!([{"provider": "ok", "displayName": "OK", "status": "ok"}]),
        );
        assert_eq!(
            read_provider_usage_snapshot(&[stale, valid], 100).len(),
            1,
            "a later candidate supplies usage when the first is stale"
        );
    }
}
