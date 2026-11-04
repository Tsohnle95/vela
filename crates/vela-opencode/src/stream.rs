use std::time::Duration;

use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::client::Client;

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(30);
const RETRY_BASE: Duration = Duration::from_millis(250);
const RETRY_CAP: Duration = Duration::from_secs(5);

/// Spawn a reconnecting SSE reader. The receiver is bounded; when the engine
/// falls behind, reads apply backpressure to the socket instead of growing an
/// unbounded queue in memory.
pub fn spawn_event_stream(client: Client, cancel: CancellationToken) -> mpsc::Receiver<Value> {
    let (tx, rx) = mpsc::channel(2_048);
    tokio::spawn(async move {
        let mut retry = RETRY_BASE;
        let mut connected_once = false;
        loop {
            if cancel.is_cancelled() {
                break;
            }
            let mut opened = false;
            match read_once(&client, &tx, &mut connected_once, &mut opened, &cancel).await {
                Ok(()) => {
                    retry = RETRY_BASE;
                }
                Err(error) => {
                    if cancel.is_cancelled() {
                        break;
                    }
                    warn!(%error, "OpenCode event stream disconnected");
                }
            }
            if opened
                && !cancel.is_cancelled()
                && tx
                    .send(serde_json::json!({ "type": "server.disconnected", "data": {} }))
                    .await
                    .is_err()
            {
                break;
            }
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(retry) => {}
            }
            retry = (retry * 2).min(RETRY_CAP);
        }
    });
    rx
}

async fn read_once(
    client: &Client,
    tx: &mpsc::Sender<Value>,
    connected_once: &mut bool,
    opened: &mut bool,
    cancel: &CancellationToken,
) -> Result<(), crate::client::OpencodeError> {
    let response = tokio::select! {
        _ = cancel.cancelled() => return Ok(()),
        response = client.open_event_stream() => response?,
    };
    *opened = true;
    if *connected_once {
        // Re-materialize open sessions after a gap, exactly as the original
        // transport does: terminal events missed during the outage cannot be
        // recovered from deltas alone.
        let _ = tx
            .send(serde_json::json!({ "type": "server.connected", "data": {} }))
            .await;
    }
    *connected_once = true;

    let mut stream = response.bytes_stream();
    let mut decoder = EventDecoder::default();
    loop {
        let next = tokio::select! {
            _ = cancel.cancelled() => return Ok(()),
            next = tokio::time::timeout(HEARTBEAT_TIMEOUT, stream.next()) => next,
        };
        let chunk = match next {
            Ok(Some(Ok(chunk))) => chunk,
            Ok(Some(Err(error))) => {
                debug!(%error, "SSE chunk error");
                return Ok(());
            }
            Ok(None) => return Ok(()),
            Err(_) => {
                debug!("SSE heartbeat timed out");
                return Ok(());
            }
        };
        for data in decoder.push(&chunk) {
            if data == "[DONE]" {
                continue;
            }
            match serde_json::from_str::<Value>(&data) {
                Ok(value) => {
                    if tokio::select! {
                        _ = cancel.cancelled() => return Ok(()),
                        result = tx.send(value) => result.is_err(),
                    } {
                        return Ok(());
                    }
                }
                Err(error) => debug!(%error, "invalid SSE JSON frame"),
            }
        }
    }
}

/// Decode complete UTF-8 lines, never individual transport chunks. CR, LF
/// and CRLF can all straddle chunks. A blank line dispatches joined data fields;
/// comments and non-data fields do not become JSON payloads.
#[derive(Default)]
struct EventDecoder {
    line: Vec<u8>,
    data: Vec<String>,
    skip_lf: bool,
    saw_line: bool,
}
impl EventDecoder {
    fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut events = Vec::new();
        for &byte in bytes {
            if self.skip_lf {
                self.skip_lf = false;
                if byte == b'\n' {
                    continue;
                }
            }
            if byte == b'\r' || byte == b'\n' {
                if let Some(event) = self.finish_line() {
                    events.push(event);
                }
                self.skip_lf = byte == b'\r';
            } else {
                self.line.push(byte);
            }
        }
        events
    }
    fn finish_line(&mut self) -> Option<String> {
        let line = String::from_utf8_lossy(&self.line);
        let line = if self.saw_line {
            line.as_ref()
        } else {
            line.strip_prefix('\u{feff}').unwrap_or(&line)
        };
        self.saw_line = true;
        let event = if line.is_empty() {
            (!self.data.is_empty()).then(|| std::mem::take(&mut self.data).join("\n"))
        } else {
            let (field, value) = line.split_once(':').unwrap_or((line, ""));
            if field == "data" {
                self.data
                    .push(value.strip_prefix(' ').unwrap_or(value).to_owned());
            }
            None
        };
        self.line.clear();
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unicode_delta_survives_http_chunks_that_split_every_utf8_codepoint() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let expected = serde_json::json!({"type":"session.text.delta","data":{"sessionID":"fixture","delta":"こんにちは 🌍 e\u{301} é"}});
        let body = format!("data: {expected}\n\n");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            let length = socket.read(&mut request).await.unwrap();
            assert!(request[..length].starts_with(b"GET /api/event "));
            let mut response = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
            for byte in body.bytes() {
                response.extend_from_slice(&[b'1', b'\r', b'\n', byte, b'\r', b'\n']);
            }
            response.extend_from_slice(b"0\r\n\r\n");
            socket.write_all(&response).await.unwrap();
        });
        let client = Client::new(crate::Endpoint {
            url: format!("http://{address}"),
            auth: None,
            version: None,
            pid: None,
        })
        .unwrap();
        let (sender, mut receiver) = mpsc::channel(4);
        let mut connected = false;
        let mut opened = false;
        tokio::time::timeout(
            Duration::from_secs(2),
            read_once(
                &client,
                &sender,
                &mut connected,
                &mut opened,
                &CancellationToken::new(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(receiver.try_recv().unwrap(), expected);
        server.await.unwrap();
    }

    #[test]
    fn parses_multiline_frames_with_comments() {
        let mut decoder = EventDecoder::default();
        assert_eq!(
            decoder.push(b": keepalive\ndata: {\"type\":\"a\"}\n\n"),
            vec!["{\"type\":\"a\"}"]
        );
        assert!(decoder.line.is_empty());
    }

    #[test]
    fn handles_crlf_and_multiple_frames() {
        let mut decoder = EventDecoder::default();
        assert_eq!(
            decoder.push(b"data: one\r\n\r\ndata: two\n\npartial"),
            vec!["one", "two"]
        );
        assert_eq!(decoder.line, b"partial");
    }

    #[test]
    fn handles_bom_mixed_line_endings_and_preserves_data_whitespace_at_every_boundary() {
        let stream = "\u{feff}: comment\rdata: first\r\ndata:  second\r\rid: ignored\nevent: delta\ndata\n\n";
        for width in 1..=stream.len() {
            let mut decoder = EventDecoder::default();
            let events = stream
                .as_bytes()
                .chunks(width)
                .flat_map(|chunk| decoder.push(chunk))
                .collect::<Vec<_>>();
            assert_eq!(events, vec!["first\n second", ""], "chunk width {width}");
        }
    }
}
