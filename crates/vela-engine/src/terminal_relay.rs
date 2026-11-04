//! Order final PTY output before the exit notification, regardless of thread order.
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Coalesce a short burst into one UI update without adding more than one
/// display-frame of latency to interactive echo. The reader already supplies
/// 16 KiB chunks; the byte cap additionally bounds a pathological stream of
/// tiny writes.
const COALESCE_WINDOW: Duration = Duration::from_millis(16);
const MAX_BATCH_BYTES: usize = 64 * 1024;
const MAX_BATCH_CHUNKS: usize = 64;

pub(crate) async fn relay(
    mut output: mpsc::Receiver<Vec<u8>>,
    mut exit: mpsc::Receiver<Option<i32>>,
    cancel: CancellationToken,
    mut emit: impl FnMut(Vec<u8>),
) -> Option<i32> {
    // A descendant may keep the slave open after the shell exits. Bound this
    // final drain rather than retaining the terminal forever for that process.
    const EXIT_DRAIN: Duration = Duration::from_millis(500);
    let mut output_open = true;
    let mut exit_code = None;
    let mut exited = false;
    let mut deadline = None;
    loop {
        if exited && !output_open {
            return exit_code;
        }
        tokio::select! {
            // Once the exit deadline is set it must win over a continuous
            // output burst; otherwise a noisy child can delay terminal cleanup
            // indefinitely.
            biased;
            _ = cancel.cancelled() => return exit_code,
            _ = async {
                if let Some(deadline) = deadline {
                    tokio::time::sleep_until(deadline).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                // Deliver anything already queued before closing the relay.
                for _ in 0..output.len() {
                    if let Ok(bytes) = output.try_recv() {
                        emit(bytes);
                    }
                }
                return exit_code;
            },
            code = exit.recv(), if !exited => {
                exit_code = code.flatten();
                exited = true;
                deadline = Some(tokio::time::Instant::now() + EXIT_DRAIN);
            },
            chunk = output.recv(), if output_open => match chunk {
                Some(bytes) => {
                    let (batch, open) = coalesce(&mut output, bytes).await;
                    if !open {
                        output_open = false;
                    }
                    emit(batch);
                }
                None => output_open = false,
            },
        }
    }
}

async fn coalesce(output: &mut mpsc::Receiver<Vec<u8>>, first: Vec<u8>) -> (Vec<u8>, bool) {
    let mut batch = first;
    let mut chunks = 1;
    let deadline = tokio::time::Instant::now() + COALESCE_WINDOW;
    while batch.len() < MAX_BATCH_BYTES && chunks < MAX_BATCH_CHUNKS {
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        match tokio::time::timeout_at(deadline, output.recv()).await {
            Ok(Some(next)) => {
                if next.is_empty() {
                    continue;
                }
                batch.extend_from_slice(&next);
                chunks += 1;
            }
            Ok(None) => return (batch, false),
            Err(_) => break,
        }
    }
    (batch, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn exit_cannot_overtake_queued_final_output() {
        let (data_tx, data_rx) = mpsc::channel(8);
        let (exit_tx, exit_rx) = mpsc::channel(1);
        exit_tx.send(Some(7)).await.unwrap();
        let mut task = tokio::spawn(async move {
            let mut bytes = Vec::new();
            let code = relay(data_rx, exit_rx, CancellationToken::new(), |chunk| {
                bytes.extend(chunk)
            })
            .await;
            (bytes, code)
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut task)
                .await
                .is_err(),
            "exit must wait for the reader's final data"
        );
        data_tx.send(b"first".to_vec()).await.unwrap();
        data_tx.send(b"last".to_vec()).await.unwrap();
        drop(data_tx);
        let (bytes, code) = task.await.unwrap();
        assert_eq!(code, Some(7));
        assert_eq!(bytes, b"firstlast");
    }

    #[tokio::test]
    async fn output_eof_still_waits_for_exit_and_cancellation_is_bounded() {
        let (data_tx, data_rx) = mpsc::channel(1);
        let (exit_tx, exit_rx) = mpsc::channel(1);
        drop(data_tx);
        let task = tokio::spawn(relay(data_rx, exit_rx, CancellationToken::new(), |_| {}));
        tokio::task::yield_now().await;
        assert!(
            !task.is_finished(),
            "output EOF must not drop the exit receiver"
        );
        exit_tx.send(Some(19)).await.unwrap();
        assert_eq!(task.await.unwrap(), Some(19));
        let (_data_tx, data_rx) = mpsc::channel(1);
        let (_exit_tx, exit_rx) = mpsc::channel(1);
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(1),
                relay(data_rx, exit_rx, cancel, |_| {})
            )
            .await
            .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn burst_output_is_coalesced_without_reordering() {
        let (data_tx, data_rx) = mpsc::channel(256);
        let (exit_tx, exit_rx) = mpsc::channel(1);
        let expected = (0..100u8)
            .map(|index| b'a' + index % 26)
            .collect::<Vec<_>>();
        for byte in &expected {
            data_tx.send(vec![*byte]).await.unwrap();
        }
        exit_tx.send(Some(0)).await.unwrap();
        drop(data_tx);
        let mut batches = Vec::new();
        let code = relay(data_rx, exit_rx, CancellationToken::new(), |chunk| {
            batches.push(chunk)
        })
        .await;
        assert_eq!(code, Some(0));
        assert!(
            batches.len() >= 2,
            "100 tiny writes must stay within the chunk bound"
        );
        assert!(batches.len() < 100, "tiny writes should coalesce");
        assert_eq!(batches.concat(), expected);
        assert!(batches.iter().all(|batch| batch.len() <= MAX_BATCH_CHUNKS));
    }

    #[tokio::test]
    async fn exit_deadline_wins_over_a_continuous_output_burst() {
        let (data_tx, data_rx) = mpsc::channel(2);
        let (exit_tx, exit_rx) = mpsc::channel(1);
        let sender = tokio::spawn(async move {
            loop {
                if data_tx.send(vec![b'x'; 1024]).await.is_err() {
                    break;
                }
            }
        });
        tokio::task::yield_now().await;
        exit_tx.send(Some(9)).await.unwrap();
        let code = tokio::time::timeout(
            Duration::from_secs(2),
            relay(data_rx, exit_rx, CancellationToken::new(), |_| {}),
        )
        .await
        .expect("a continuous output burst starved the exit deadline");
        assert_eq!(code, Some(9));
        sender.abort();
    }

    #[tokio::test]
    async fn inherited_open_slave_cannot_hold_an_exited_terminal_forever() {
        let (data_tx, data_rx) = mpsc::channel(1);
        let (exit_tx, exit_rx) = mpsc::channel(1);
        data_tx.send(b"tail".to_vec()).await.unwrap();
        exit_tx.send(Some(4)).await.unwrap();
        let mut bytes = Vec::new();
        let code = tokio::time::timeout(
            Duration::from_secs(2),
            relay(data_rx, exit_rx, CancellationToken::new(), |chunk| {
                bytes.extend(chunk)
            }),
        )
        .await
        .expect("open slave kept an exited terminal alive");
        assert_eq!(code, Some(4));
        assert_eq!(bytes, b"tail");
        drop(data_tx);
    }
}
