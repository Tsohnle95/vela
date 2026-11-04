use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use vela_engine::{Engine, EngineConfig, Settings};
use vela_ui::run;

fn workspace_arg() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--workspace" {
            return args.next().map(|path| normalize(PathBuf::from(path)));
        }
        if let Some(path) = arg.strip_prefix("--workspace=") {
            return Some(normalize(PathBuf::from(path)));
        }
        if !arg.starts_with('-') {
            return Some(normalize(PathBuf::from(arg)));
        }
    }
    std::env::var_os("VELA_WORKSPACE").map(|path| normalize(PathBuf::from(path)))
}

fn normalize(path: PathBuf) -> String {
    path.canonicalize()
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn ensure_comparison_isolation(
    executable: &Path,
    env: impl Fn(&str) -> Option<OsString>,
) -> anyhow::Result<()> {
    if !matches!(
        executable.file_name().and_then(|name| name.to_str()),
        Some("Vela Native QA" | "Vela Native QA.exe")
    ) {
        return Ok(());
    }
    let enabled = env("VELA_PARITY_COMPARISON").as_deref() == Some(std::ffi::OsStr::new("1"));
    let skip_import = env("VELA_SKIP_LEGACY_IMPORT").as_deref() == Some(std::ffi::OsStr::new("1"));
    let profiles_set = [
        "VELA_DATA_DIR",
        "XDG_STATE_HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
    ]
    .iter()
    .all(|key| env(key).is_some_and(|value| !value.is_empty()));
    anyhow::ensure!(
        enabled && skip_import && profiles_set,
        "Vela Native QA requires the isolated launcher: bash scripts/run-parity.sh --comparison. No app state or OpenCode service has been opened."
    );
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // A GUI automation rebind can launch a stopped app without shell variables.
    // Refuse that QA launch before reading settings, importing data or starting
    // any service. Normal developer/installed app launches retain their behavior.
    ensure_comparison_isolation(&std::env::current_exe()?, |key| std::env::var_os(key))?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vela=info,vela_engine=info,vela_opencode=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?;

    let data_dir = vela_engine::persist::default_data_dir();
    let mut settings = Settings::load(data_dir.as_deref());
    let migration = vela_engine::migration::import_legacy(&mut settings);
    if migration.ambiguous > 0 {
        tracing::warn!(
            count = migration.ambiguous,
            "workspace bookmarks have ambiguous relocation matches; stored paths kept"
        );
    }
    if let Some(directory) = data_dir.as_deref()
        && let Err(error) = vela_engine::migration::import_sass_preferences(directory)
    {
        tracing::warn!(%error, "could not import legacy Sass preferences (source preserved)");
    }
    if let (Some((width, height)), Some(directory)) = (migration.window, data_dir.as_deref())
        && let Some(size) =
            vela_engine::window_state::WindowSize::new(f64::from(width), f64::from(height))
        && let Err(error) = size.import_if_absent(directory)
    {
        tracing::warn!(%error, "could not import legacy session window dimensions");
    }
    if migration.imported > 0 || migration.pins_scanned {
        if let Some(directory) = data_dir.as_deref()
            && let Err(error) = settings.save(Some(directory))
        {
            tracing::warn!(%error, "could not persist imported legacy Vela data");
        }
        tracing::info!(
            workspaces = migration.imported,
            duplicates = migration.duplicates,
            pins = migration.pins_imported,
            source = ?migration.source,
            "imported legacy Vela data (read-only)"
        );
    }
    let warm_sessions = std::env::var("VELA_NAV_WARM")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value >= 2)
        .unwrap_or(settings.warm_sessions);
    let config = EngineConfig {
        warm_sessions,
        warm_bytes: settings.warm_bytes,
        data_dir: data_dir.clone(),
        service_file: None,
        opencode_binary: std::env::var_os("VELA_OPENCODE_BINARY").map(PathBuf::from),
    };
    let engine = Engine::spawn(config, runtime.handle());
    let workspace = workspace_arg();

    run(engine.clone(), settings, workspace);

    runtime.block_on(engine.shutdown_and_wait());
    runtime.shutdown_timeout(Duration::from_secs(2));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qa_requires_comparison_flag_and_complete_isolated_environment() {
        let values = [
            ("VELA_PARITY_COMPARISON", "1"),
            ("VELA_SKIP_LEGACY_IMPORT", "1"),
            ("VELA_DATA_DIR", "/private/tmp/qa-profile"),
            ("XDG_STATE_HOME", "/private/tmp/qa-state"),
            ("XDG_CONFIG_HOME", "/private/tmp/qa-config"),
            ("XDG_DATA_HOME", "/private/tmp/qa-data"),
        ];
        let executable = Path::new("/private/tmp/Vela Native QA");
        assert!(ensure_comparison_isolation(executable, |_| None).is_err());
        assert!(
            ensure_comparison_isolation(executable, |key| values
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| OsString::from(value)))
            .is_ok()
        );
        for (missing, _) in values {
            assert!(
                ensure_comparison_isolation(executable, |key| {
                    values
                        .iter()
                        .find(|(name, _)| *name == key && *name != missing)
                        .map(|(_, value)| OsString::from(value))
                })
                .is_err(),
                "missing {missing} must refuse the QA launch"
            );
        }
    }
    #[test]
    fn ordinary_app_does_not_require_comparison_configuration() {
        for name in ["vela", "Vela Native"] {
            assert!(
                ensure_comparison_isolation(Path::new(name), |_| panic!(
                    "ordinary launch must not inspect QA configuration"
                ))
                .is_ok()
            );
        }
    }
}
