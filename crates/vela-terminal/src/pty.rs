use std::io::{Read, Write};
use std::sync::Mutex;

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::sync::mpsc;

const READ_CHUNK: usize = 16 * 1024;

pub struct PtySession {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    cwd: String,
}

impl std::fmt::Debug for PtySession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PtySession")
            .field("cwd", &self.cwd)
            .finish_non_exhaustive()
    }
}

pub struct SpawnedPty {
    pub session: std::sync::Arc<PtySession>,
    pub output: mpsc::Receiver<Vec<u8>>,
    pub exit: mpsc::Receiver<Option<i32>>,
}

/// Spawn the user's login shell in `cwd`. Reader/wait threads are detached;
/// both terminate when the child exits or the PTY is dropped.
pub fn spawn(cwd: &str, cols: u16, rows: u16) -> anyhow::Result<SpawnedPty> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    spawn_command(cwd, &shell, &[], &[], cols, rows)
}

/// Spawn an explicit command for the integrated Agent TUI or similar tools.
pub fn spawn_command(
    cwd: &str,
    program: &str,
    args: &[String],
    envs: &[(String, String)],
    cols: u16,
    rows: u16,
) -> anyhow::Result<SpawnedPty> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: rows.max(1),
        cols: cols.max(2),
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let mut command = CommandBuilder::new(program);
    command.args(args);
    command.cwd(cwd);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = pair.slave.spawn_command(command)?;
    drop(pair.slave);

    let reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;
    let killer = child.clone_killer();

    let (tx, rx) = mpsc::channel::<Vec<u8>>(256);
    std::thread::Builder::new()
        .name("vela-pty-reader".into())
        .spawn(move || {
            let mut reader = reader;
            let mut buffer = vec![0u8; READ_CHUNK];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if tx.blocking_send(buffer[..count].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        tracing::debug!(%error, "pty read ended");
                        break;
                    }
                }
            }
        })?;

    let (exit_tx, exit_rx) = mpsc::channel::<Option<i32>>(1);
    let killer_for_wait = child.clone_killer();
    std::thread::Builder::new()
        .name("vela-pty-wait".into())
        .spawn(move || {
            let status = child.wait();
            let code = status.ok().map(|status| status.exit_code() as i32);
            let _ = exit_tx.blocking_send(code);
            drop(killer_for_wait);
        })?;

    Ok(SpawnedPty {
        session: std::sync::Arc::new(PtySession {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            killer: Mutex::new(killer),
            cwd: cwd.to_string(),
        }),
        output: rx,
        exit: exit_rx,
    })
}

impl PtySession {
    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    pub fn write(&self, bytes: &[u8]) -> anyhow::Result<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("pty writer poisoned"))?;
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> anyhow::Result<()> {
        let master = self
            .master
            .lock()
            .map_err(|_| anyhow::anyhow!("pty master poisoned"))?;
        master.resize(PtySize {
            rows: rows.max(1),
            cols: cols.max(2),
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    pub fn kill(&self) {
        if let Ok(mut killer) = self.killer.lock() {
            let _ = killer.kill();
        }
    }
}
