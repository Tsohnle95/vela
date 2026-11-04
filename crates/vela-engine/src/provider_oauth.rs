//! One cancellable lease owns an authorization attempt, including a start reply
//! that arrives after its view closed. No UI timer or current workspace lookup
//! may retarget polling, code submission or cleanup to another workspace.
use crate::EngineUpdate;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use vela_opencode::{
    Client,
    providers::{OAuthAttempt, OAuthStatus},
};

pub struct ProviderOAuthRequest {
    pub flow_id: u64,
    pub session_id: String,
    pub directory: String,
    pub integration_id: String,
    pub method_id: String,
}
#[derive(Clone, Debug)]
pub enum ProviderOAuthState {
    Pending(OAuthAttempt),
    Confirming,
    CodeRejected(String),
    Complete,
    Failed(String),
}
pub(crate) struct Lease {
    pub session_id: String,
    pub cancel: CancellationToken,
    pub code: mpsc::UnboundedSender<String>,
}
pub(crate) fn spawn(
    client: Client,
    request: ProviderOAuthRequest,
    cancel: CancellationToken,
    updates: broadcast::Sender<EngineUpdate>,
    finished: impl FnOnce() + Send + 'static,
) -> Lease {
    let (code, mut codes) = mpsc::unbounded_channel::<String>();
    let task_cancel = cancel.clone();
    let session_id = request.session_id.clone();
    tokio::spawn(async move {
        let publish = |state| {
            let _ = updates.send(EngineUpdate::ProviderOAuth {
                flow_id: request.flow_id,
                session_id: request.session_id.clone(),
                integration_id: request.integration_id.clone(),
                state,
            });
        };
        // Do not discard an in-flight start response on cancellation. It owns
        // the attempt ID needed to clean up the newly created server attempt.
        match client
            .start_provider_oauth(
                &request.directory,
                &request.integration_id,
                &request.method_id,
            )
            .await
        {
            Err(error) => {
                if !task_cancel.is_cancelled() {
                    publish(ProviderOAuthState::Failed(error.to_string()));
                }
            }
            Ok(attempt) => {
                let mut complete = false;
                if !task_cancel.is_cancelled() {
                    publish(ProviderOAuthState::Pending(attempt.clone()));
                    if attempt.code {
                        loop {
                            tokio::select! {
                                biased;
                                _ = task_cancel.cancelled() => break,
                                code = codes.recv() => {
                                    let Some(code) = code else { break; };
                                    publish(ProviderOAuthState::Confirming);
                                    match client.complete_provider_oauth(&request.directory, &request.integration_id, &attempt.id, code).await {
                                        Ok(()) => { complete = true; break; }
                                        Err(error) => if !task_cancel.is_cancelled() { publish(ProviderOAuthState::CodeRejected(error.to_string())); },
                                    }
                                }
                            }
                        }
                    } else {
                        let mut interval = tokio::time::interval(Duration::from_secs(2));
                        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                        loop {
                            tokio::select! {
                                biased;
                                _ = task_cancel.cancelled() => break,
                                _ = interval.tick() => {
                                    let poll = client.poll_provider_oauth(&request.directory, &request.integration_id, &attempt.id);
                                    let result = tokio::select! {
                                        biased;
                                        _ = task_cancel.cancelled() => break,
                                        result = poll => result,
                                    };
                                    match result {
                                        Ok(OAuthStatus::Pending) => {},
                                        Ok(OAuthStatus::Complete) => { complete = true; break; }
                                        Ok(OAuthStatus::Expired) => { publish(ProviderOAuthState::Failed("Authorization request expired".into())); break; }
                                        Ok(OAuthStatus::Failed) => { publish(ProviderOAuthState::Failed("Authorization failed".into())); break; }
                                        Err(error) => { publish(ProviderOAuthState::Failed(error.to_string())); break; }
                                    }
                                }
                            }
                        }
                    }
                }
                if complete {
                    if !task_cancel.is_cancelled() {
                        publish(ProviderOAuthState::Complete);
                    }
                } else {
                    let _ = tokio::time::timeout(
                        Duration::from_secs(5),
                        client.cancel_provider_oauth(
                            &request.directory,
                            &request.integration_id,
                            &attempt.id,
                        ),
                    )
                    .await;
                }
            }
        }
        finished();
    });
    Lease {
        session_id,
        cancel,
        code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        extract::{Path, Query, State},
        routing::{get, post},
    };
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};
    use tokio::sync::{Notify, oneshot};
    use vela_opencode::service::Endpoint;

    #[derive(Clone, Default)]
    struct Fixture {
        starts: Arc<Mutex<Vec<String>>>,
        codes: Arc<Mutex<Vec<String>>>,
        cancels: Arc<Mutex<Vec<String>>>,
        release_start: Arc<Notify>,
    }
    fn check_directory(query: &std::collections::HashMap<String, String>) {
        assert_eq!(
            query.get("location[directory]").map(String::as_str),
            Some("/captured/workspace")
        );
    }
    async fn start(
        State(state): State<Fixture>,
        Query(query): Query<std::collections::HashMap<String, String>>,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        check_directory(&query);
        let method = body["methodID"].as_str().unwrap().to_owned();
        state.starts.lock().unwrap().push(method.clone());
        if method == "slow" {
            state.release_start.notified().await;
        }
        Json(
            json!({"data":{"attemptID":method,"mode":if method == "code" {"code"} else {"auto"},"url":"http://127.0.0.1/fixture-auth","instructions":"Fixture authorization"}}),
        )
    }
    async fn poll(
        Path((_, attempt)): Path<(String, String)>,
        Query(query): Query<std::collections::HashMap<String, String>>,
    ) -> Json<Value> {
        check_directory(&query);
        Json(json!({"data":{"status":if attempt == "expired" {"expired"} else {"complete"}}}))
    }
    async fn complete(
        State(state): State<Fixture>,
        Query(query): Query<std::collections::HashMap<String, String>>,
        Json(body): Json<Value>,
    ) -> (axum::http::StatusCode, Json<Value>) {
        check_directory(&query);
        let code = body["code"].as_str().unwrap().to_owned();
        state.codes.lock().unwrap().push(code.clone());
        if code == "reject-code-fixture" {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(json!({"message":format!("echoed {code}")})),
            )
        } else {
            (axum::http::StatusCode::NO_CONTENT, Json(Value::Null))
        }
    }
    async fn cancel(
        State(state): State<Fixture>,
        Path((_, attempt)): Path<(String, String)>,
        Query(query): Query<std::collections::HashMap<String, String>>,
    ) -> axum::http::StatusCode {
        check_directory(&query);
        state.cancels.lock().unwrap().push(attempt);
        axum::http::StatusCode::NO_CONTENT
    }
    async fn fixture() -> (Client, Fixture) {
        let state = Fixture::default();
        let router = Router::new()
            .route("/api/integration/{id}/connect/oauth", post(start))
            .route(
                "/api/integration/{id}/connect/oauth/{attempt}",
                get(poll).delete(cancel),
            )
            .route(
                "/api/integration/{id}/connect/oauth/{attempt}/complete",
                post(complete),
            )
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (
            Client::new(Endpoint {
                url: format!("http://{address}"),
                auth: None,
                version: None,
                pid: None,
            })
            .unwrap(),
            state,
        )
    }
    fn begin(
        client: Client,
        method: &str,
    ) -> (
        Lease,
        broadcast::Receiver<EngineUpdate>,
        oneshot::Receiver<()>,
    ) {
        let (updates, rx) = broadcast::channel(16);
        let (finished, done) = oneshot::channel();
        let lease = spawn(
            client,
            ProviderOAuthRequest {
                flow_id: 91,
                session_id: "session-fixture".into(),
                directory: "/captured/workspace".into(),
                integration_id: "openai".into(),
                method_id: method.into(),
            },
            CancellationToken::new(),
            updates,
            move || {
                let _ = finished.send(());
            },
        );
        (lease, rx, done)
    }
    async fn next(rx: &mut broadcast::Receiver<EngineUpdate>) -> ProviderOAuthState {
        match tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            EngineUpdate::ProviderOAuth {
                flow_id,
                session_id,
                integration_id,
                state,
            } => {
                assert_eq!(flow_id, 91);
                assert_eq!(session_id, "session-fixture");
                assert_eq!(integration_id, "openai");
                state
            }
            other => panic!("unexpected update: {other:?}"),
        }
    }
    #[tokio::test]
    async fn code_authorization_retries_redacted_errors_then_finishes() {
        let (client, fixture) = fixture().await;
        let (lease, mut updates, done) = begin(client, "code");
        assert!(
            matches!(next(&mut updates).await, ProviderOAuthState::Pending(attempt) if attempt.code)
        );
        lease.code.send("reject-code-fixture".into()).unwrap();
        assert!(matches!(
            next(&mut updates).await,
            ProviderOAuthState::Confirming
        ));
        assert!(
            matches!(next(&mut updates).await, ProviderOAuthState::CodeRejected(message) if !message.contains("reject-code-fixture"))
        );
        lease.code.send("fixture-success".into()).unwrap();
        assert!(matches!(
            next(&mut updates).await,
            ProviderOAuthState::Confirming
        ));
        assert!(matches!(
            next(&mut updates).await,
            ProviderOAuthState::Complete
        ));
        tokio::time::timeout(Duration::from_secs(5), done)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            fixture.codes.lock().unwrap().as_slice(),
            ["reject-code-fixture", "fixture-success"]
        );
        assert!(fixture.cancels.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn cancelled_late_start_is_cleaned_up_without_publishing_an_attempt() {
        let (client, fixture) = fixture().await;
        let (lease, mut updates, done) = begin(client, "slow");
        tokio::time::timeout(Duration::from_secs(5), async {
            while fixture.starts.lock().unwrap().is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        lease.cancel.cancel();
        fixture.release_start.notify_one();
        tokio::time::timeout(Duration::from_secs(5), done)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fixture.cancels.lock().unwrap().as_slice(), ["slow"]);
        assert!(updates.try_recv().is_err());
    }
    #[tokio::test]
    async fn automatic_authorization_finishes_or_cleans_up_expiry() {
        for method in ["auto", "expired"] {
            let (client, fixture) = fixture().await;
            let (_lease, mut updates, done) = begin(client, method);
            assert!(
                matches!(next(&mut updates).await, ProviderOAuthState::Pending(attempt) if !attempt.code)
            );
            let state = next(&mut updates).await;
            if method == "auto" {
                assert!(matches!(state, ProviderOAuthState::Complete));
            } else {
                assert!(
                    matches!(state, ProviderOAuthState::Failed(message) if message == "Authorization request expired")
                );
            }
            tokio::time::timeout(Duration::from_secs(5), done)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                fixture.cancels.lock().unwrap().len(),
                usize::from(method == "expired")
            );
        }
    }
}
