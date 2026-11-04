//! Bundled Dart Sass compiler process. Project commands are never evaluated.
use crate::validation::Diagnostic;
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) struct Program {
    node: PathBuf,
    script: PathBuf,
}
#[derive(Deserialize)]
pub(crate) struct Output {
    pub source: PathBuf,
    pub output: PathBuf,
    pub css: Option<String>,
    #[serde(rename = "sourceMap")]
    pub source_map: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub error: Option<String>,
}
#[derive(Deserialize)]
pub(crate) struct Batch {
    #[serde(default)]
    pub convention: String,
    #[serde(default)]
    pub outputs: Vec<Output>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct ValidationReport {
    #[serde(default)]
    diagnostics: Vec<ValidationDiagnostic>,
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidationDiagnostic {
    line: u32,
    column: u32,
    end_line: u32,
    end_column: u32,
    message: String,
    severity: String,
}
impl Program {
    pub fn discover() -> Result<Self, String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let bundled = executable
            .parent()
            .and_then(Path::parent)
            .map(|path| path.join("Resources/preview"));
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let candidates = bundled
            .into_iter()
            .map(|root| Self {
                node: root.join("node/bin/node"),
                script: root.join("sass-compile.mjs"),
            })
            .chain(std::iter::once(Self {
                node: source.join(".tools/preview/node/bin/node"),
                script: source.join("tools/preview/sass-compile.mjs"),
            }));
        candidates.into_iter().find(|program| program.node.is_file() && program.script.is_file())
            .ok_or_else(|| "Vela's bundled Sass tools are missing. Run scripts/setup-preview-toolchain.sh and rebuild the app.".into())
    }
    pub async fn compile(&self, root: &Path, cancel: &CancellationToken) -> Result<Batch, String> {
        if cancel.is_cancelled() {
            return Err("Sass compilation cancelled".into());
        }
        let mut command = tokio::process::Command::new(&self.node);
        command
            .arg(&self.script)
            .arg(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let output = tokio::select! {
            _ = cancel.cancelled() => return Err("Sass compilation cancelled".into()),
            output = command.output() => output.map_err(|error| error.to_string())?,
        };
        let batch: Batch = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "Sass compiler returned invalid output: {error}: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        if let Some(error) = batch.error {
            return Err(error);
        }
        if !output.status.success() {
            return Err(format!("Sass compiler exited with {}", output.status));
        }
        Ok(batch)
    }

    /// Validate an unsaved SCSS/Sass buffer through the pinned compiler without
    /// writing output. Relative imports resolve from the real source path and
    /// workspace root, so diagnostics match the on-disk project context.
    #[cfg(test)]
    pub async fn validate(
        &self,
        root: &Path,
        source: &Path,
        content: &str,
    ) -> Result<Vec<Diagnostic>, String> {
        self.validate_with_cancel(root, source, content, &CancellationToken::new())
            .await
    }

    pub async fn validate_with_cancel(
        &self,
        root: &Path,
        source: &Path,
        content: &str,
        cancel: &CancellationToken,
    ) -> Result<Vec<Diagnostic>, String> {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let source = if source.is_absolute() {
            source.to_path_buf()
        } else {
            root.join(source)
        };
        let mut command = tokio::process::Command::new(&self.node);
        command
            .arg(&self.script)
            .arg("--validate")
            .arg(&root)
            .arg(&source)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn().map_err(|error| error.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(content.as_bytes())
                .await
                .map_err(|error| error.to_string())?;
            drop(stdin);
        }
        let output = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err("Sass validation cancelled".into()),
            output = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                child.wait_with_output(),
            ) => output
                .map_err(|_| "Sass validation timed out".to_string())?
                .map_err(|error| error.to_string())?,
        };
        let report: ValidationReport = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "Sass validator returned invalid output: {error}: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        if let Some(error) = report.error {
            return Err(error);
        }
        Ok(report
            .diagnostics
            .into_iter()
            .map(|diagnostic| Diagnostic {
                line: diagnostic.line.max(1),
                column: diagnostic.column.max(1),
                end_line: diagnostic.end_line.max(diagnostic.line.max(1)),
                end_column: diagnostic.end_column.max(diagnostic.column.max(1)),
                message: diagnostic.message,
                severity: if diagnostic.severity == "warning" {
                    "warning".into()
                } else {
                    "error".into()
                },
                source: "scss".into(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn pinned_compiler_returns_modern_sass_maps_and_diagnostics_without_writing_outputs() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("main.scss"),
            "@use 'sass:math'; .a { width: math.div(10px, 2); }",
        )
        .unwrap();
        std::fs::write(
            temp.path().join("vela-sass.json"),
            "{entries:{'main.scss':'public/main.css'},sourceMap:true}",
        )
        .unwrap();
        let result = Program::discover()
            .unwrap()
            .compile(temp.path(), &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(result.convention, "vela-sass.json");
        assert_eq!(result.outputs.len(), 1);
        assert!(
            result.outputs[0]
                .css
                .as_deref()
                .unwrap()
                .contains("width: 5px")
        );
        assert!(result.outputs[0].source_map.is_some());
        assert!(!temp.path().join("public").exists());
    }

    #[tokio::test]
    async fn pinned_compiler_validates_unsaved_scss_with_imports_and_warnings() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("vars.scss"), "$accent: red;\n").unwrap();
        let source = temp.path().join("main.scss");
        let program = Program::discover().unwrap();
        let valid = program
            .validate(
                temp.path(),
                &source,
                "@use 'vars';\n.a { color: vars.$accent; }\n",
            )
            .await
            .unwrap();
        assert!(
            valid.iter().all(|diagnostic| diagnostic.severity == "warning"),
            "valid scss should not produce errors: {valid:?}"
        );

        let invalid = program
            .validate(temp.path(), &source, ".a { color: ; }\n")
            .await
            .unwrap();
        let error = invalid
            .iter()
            .find(|diagnostic| diagnostic.severity == "error")
            .expect("invalid scss returns a compile error");
        assert_eq!(error.source, "scss");
        assert!(error.line >= 1);
        assert!(!error.message.is_empty());

        let warnings = program
            .validate(
                temp.path(),
                &source,
                "@warn \"careful\";\n.a { color: red; }\n",
            )
            .await
            .unwrap();
        assert!(
            warnings
                .iter()
                .any(|diagnostic| diagnostic.severity == "warning"
                    && diagnostic.message.contains("careful")),
            "Sass warnings must be preserved: {warnings:?}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cancelling_sass_validation_kills_the_validation_process() {
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join("held.mjs");
        std::fs::write(&script, "import fs from 'node:fs'; fs.writeFileSync(process.argv[3]+'/pid',String(process.pid)); setInterval(()=>{},1000);").unwrap();
        let mut program = Program::discover().unwrap();
        program.script = script;
        let root = temp.path().to_owned();
        let source = temp.path().join("main.scss");
        let cancel = CancellationToken::new();
        let task_cancel = cancel.clone();
        let task = tokio::spawn(async move {
            program
                .validate_with_cancel(&root, &source, ".a { color: red; }", &task_cancel)
                .await
        });
        let pid = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if let Ok(pid) = std::fs::read_to_string(temp.path().join("pid"))
                    && let Ok(pid) = pid.parse::<i32>()
                {
                    break pid;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        cancel.cancel();
        assert_eq!(task.await.unwrap(), Err("Sass validation cancelled".into()));
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            // SAFETY: signal zero probes only the PID written by our own child.
            while unsafe { libc::kill(pid, 0) } == 0 {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("cancelled validation process remained alive");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cancellation_kills_the_owned_compiler_process() {
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join("held.mjs");
        std::fs::write(&script, "import fs from 'node:fs'; fs.writeFileSync(process.argv[2]+'/pid',String(process.pid)); setInterval(()=>{},1000);").unwrap();
        let mut program = Program::discover().unwrap();
        program.script = script;
        let root = temp.path().to_owned();
        let cancel = CancellationToken::new();
        let task_cancel = cancel.clone();
        let task = tokio::spawn(async move { program.compile(&root, &task_cancel).await });
        let pid = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if let Ok(pid) = std::fs::read_to_string(temp.path().join("pid"))
                    && let Ok(pid) = pid.parse::<i32>()
                {
                    break pid;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        cancel.cancel();
        assert!(task.await.unwrap().is_err());
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            // SAFETY: signal zero probes only the PID written by our own child.
            while unsafe { libc::kill(pid, 0) } == 0 {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("cancelled compiler remained alive");
    }
}
