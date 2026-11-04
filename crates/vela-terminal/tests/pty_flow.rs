use std::time::Duration;

use vela_terminal::emulator::Emulator;
use vela_terminal::pty;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pty_output_reaches_the_emulator() {
    let directory = std::env::temp_dir();
    let pty::SpawnedPty {
        session,
        mut output,
        mut exit,
    } = pty::spawn(&directory.to_string_lossy(), 80, 24).expect("spawn shell");
    // The command text contains `6*7`, not `42`, so a match proves the shell
    // executed it rather than merely echoing the typed input.
    session.write(b"echo $((6*7))\n").expect("write command");

    let mut emulator = Emulator::new(80, 24);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let mut matched = false;
    while tokio::time::Instant::now() < deadline && !matched {
        tokio::select! {
            chunk = output.recv() => {
                let Some(bytes) = chunk else { break };
                emulator.feed(&bytes);
                for row in 0..emulator.rows() {
                    let line: String = emulator.line(row).iter().map(|cell| cell.ch).collect();
                    if line.contains("42") {
                        matched = true;
                        break;
                    }
                }
            }
            code = exit.recv() => {
                let _ = code;
                break;
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }
    session.kill();
    assert!(
        matched,
        "expected shell arithmetic output in the emulator grid"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_terminal_commands_receive_environment_and_arguments() {
    let pty::SpawnedPty {
        session,
        mut output,
        mut exit,
    } = pty::spawn_command(
        &std::env::temp_dir().to_string_lossy(),
        "/bin/sh",
        &[
            "-c".into(),
            "printf 'TUI:%s\\n' \"$VELA_TUI_FIXTURE\"".into(),
        ],
        &[("VELA_TUI_FIXTURE".into(), "ready".into())],
        80,
        24,
    )
    .expect("spawn explicit command");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut bytes = Vec::new();
    while tokio::time::Instant::now() < deadline {
        tokio::select! {
            chunk = output.recv() => {
                if let Some(chunk) = chunk { bytes.extend(chunk); }
            }
            _ = exit.recv() => break,
            _ = tokio::time::sleep(Duration::from_millis(20)) => {}
        }
        if bytes.windows(9).any(|window| window == b"TUI:ready") {
            break;
        }
    }
    session.kill();
    assert!(
        bytes.windows(9).any(|window| window == b"TUI:ready"),
        "explicit command did not receive args/env: {:?}",
        String::from_utf8_lossy(&bytes)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kernel_resize_matches_the_emulator_minimum_and_requested_viewport() {
    struct KillOnDrop(std::sync::Arc<pty::PtySession>);
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            self.0.kill();
        }
    }
    let pty::SpawnedPty {
        session,
        mut output,
        ..
    } = pty::spawn(&std::env::temp_dir().to_string_lossy(), 80, 24).unwrap();
    let _cleanup = KillOnDrop(session.clone());
    for (cols, rows) in [(37, 9), (2, 1)] {
        session.resize(cols, rows).unwrap();
        session
            .write(b"printf 'SIZE:%s\\n' \"$(stty size)\"\n")
            .unwrap();
        let expected = format!("SIZE:{rows} {cols}").into_bytes();
        let result = tokio::time::timeout(Duration::from_secs(8), async {
            let mut bytes = Vec::new();
            while let Some(chunk) = output.recv().await {
                bytes.extend(chunk);
                if bytes
                    .windows(expected.len())
                    .any(|window| window == expected)
                {
                    return true;
                }
                assert!(
                    bytes.len() < 1024 * 1024,
                    "unexpected unbounded terminal output"
                );
            }
            false
        })
        .await
        .unwrap_or(false);
        assert!(result, "PTY did not apply {cols} columns × {rows} rows");
    }
}
