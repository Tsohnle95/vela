//! Real adapter/engine publication recovery after a subscriber misses events.
//! All sessions, service registration, executable and workspace are disposable.

use std::{collections::HashMap, convert::Infallible};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::serve::ListenerExt;
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::sync::broadcast;
use vela_engine::{ConnectionState, Engine, EngineConfig, EngineUpdate};

#[derive(Clone)]
struct ServerState {
    events: broadcast::Sender<Value>,
    directory: String,
    prompts: Arc<Mutex<Vec<Value>>>,
    appended_messages: Arc<Mutex<HashMap<String, Vec<Value>>>>,
    forms: Arc<Mutex<Vec<Value>>>,
    ids: Vec<String>,
    history: usize,
}

fn summary(id: &str, directory: &str) -> Value {
    json!({
        "id":id,"title":"Publication fixture","projectID":"fixture",
        "time":{"created":1,"updated":2},"location":{"directory":directory}
    })
}

async fn info() -> Json<Value> {
    Json(json!({"version":"2.1.4","pid":4242}))
}

async fn sessions(State(state): State<ServerState>) -> Json<Value> {
    let rows: Vec<_> = state.ids.iter().map(|id| summary(id, &state.directory)).collect();
    Json(json!({"data":rows,"cursor":{}}))
}

async fn session(Path(id): Path<String>, State(state): State<ServerState>) -> Json<Value> {
    Json(json!({"data":summary(&id, &state.directory)}))
}

async fn messages(Path(id): Path<String>, State(state): State<ServerState>) -> Json<Value> {
    let mut entries = Vec::new();
    if state.history > 0 {
        entries.extend((0..state.history).map(|index| {
            let id = format!("{id}-history-{index}");
            if index % 2 == 0 {
                json!({"id":id,"type":"user","time":{"created":1},"text":"What changed in the synthetic workspace?"})
            } else {
                json!({"id":id,"type":"assistant","time":{"created":1,"completed":2},"finish":"stop",
                    "content":[{"id":format!("{id}-text"),"type":"text",
                    "text":"The synthetic workspace has **updated files** and `inline code` in this historical answer."}]})
            }
        }));
    } else if id == "ses_noise" {
        entries.extend((0..32).map(|index| json!({
            "id":format!("history-{index}"),"type":"user",
            "time":{"created":1},"text":"Earlier prompt"
        })));
    }
    entries.extend([
        json!({"id":"user","type":"user","time":{"created":1},"text":"Fixture request"}),
        json!({"id":"seed","type":"assistant","time":{"created":2,"completed":3},
         "finish":"stop","content":[{"id":"seed_text","type":"text","text":"Initial response"}]}),
    ]);
    if let Some(appended) = state.appended_messages.lock().unwrap().get(&id) {
        entries.extend(appended.iter().cloned());
    }
    Json(json!({"data":entries,"cursor":{}}))
}

async fn prompt(State(state): State<ServerState>, Json(body): Json<Value>) -> (StatusCode, Json<Value>) {
    let fails = body["text"] == "recover queued prompt" && body["delivery"] == "queue";
    state.prompts.lock().unwrap().push(body);
    if fails {
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"message":"Disposable prompt failure"})))
    } else {
        (StatusCode::OK, Json(json!({"id":"fixture-inbox"})))
    }
}

async fn permissions() -> Json<Value> {
    Json(json!([]))
}

async fn forms(Path(id): Path<String>, State(state): State<ServerState>) -> Json<Value> {
    if id == "ses_stream" {
        Json(json!(state.forms.lock().unwrap().as_slice()))
    } else {
        Json(json!([]))
    }
}

async fn event_stream(
    State(state): State<ServerState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let stream =
        futures_util::stream::unfold(state.events.subscribe(), |mut receiver| async move {
            match receiver.recv().await {
                Ok(value) => Some((Ok(Event::default().data(value.to_string())), receiver)),
                Err(broadcast::error::RecvError::Closed) => None,
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    panic!(
                        "fixture lost {count} source events before the real adapter received them"
                    )
                }
            }
        });
    Sse::new(stream)
}

pub struct Fixture {
    pub engine: Engine,
    events: broadcast::Sender<Value>,
    server: tokio::task::JoinHandle<()>,
    _profile: tempfile::TempDir,
    // The engine-only overflow test does not issue prompts.
    #[allow(dead_code)]
    pub prompts: Arc<Mutex<Vec<Value>>>,
    #[allow(dead_code)]
    appended_messages: Arc<Mutex<HashMap<String, Vec<Value>>>>,
    #[allow(dead_code)]
    pub forms: Arc<Mutex<Vec<Value>>>,
}

impl Fixture {
    pub async fn start() -> Self {
        Self::start_with_directory(false).await
    }

    #[allow(dead_code)]
    pub async fn start_with_workspace() -> Self {
        Self::start_with_directory(true).await
    }

    #[allow(dead_code)]
    pub async fn stress(ids: Vec<String>, history: usize) -> Self {
        Self::start_config(false, ids, history).await
    }

    async fn start_with_directory(with_workspace: bool) -> Self {
        Self::start_config(with_workspace, ["ses_stream", "ses_noise", "ses_flood"].map(str::to_owned).to_vec(), 0).await
    }

    async fn start_config(with_workspace: bool, ids: Vec<String>, history: usize) -> Self {
        let profile = tempfile::tempdir().unwrap();
        let workspace = profile.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();
        let (events, _) = broadcast::channel(4_096);
        let prompts = Arc::new(Mutex::new(Vec::new()));
        let appended_messages = Arc::new(Mutex::new(HashMap::new()));
        let pending_forms = Arc::new(Mutex::new(Vec::new()));
        let state = ServerState {
            events: events.clone(),
            directory: if with_workspace { workspace.to_string_lossy().into_owned() } else { String::new() },
            prompts: prompts.clone(),
            appended_messages: appended_messages.clone(),
            forms: pending_forms.clone(),
            ids: ids.clone(),
            history,
        };
        let app = Router::new()
            .route("/api/info", get(info))
            .route("/api/session", get(sessions))
            .route("/api/session/{id}", get(session))
            .route("/api/session/{id}/message", get(messages))
            .route("/api/session/{id}/prompt", post(prompt))
            .route("/api/session/{id}/inbox", get(permissions))
            .route("/api/session/{id}/form", get(forms))
            .route("/api/form", get(permissions))
            .route("/api/permission/request", get(permissions))
            .route("/api/event", get(event_stream))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let listener = listener.tap_io(|socket| socket.set_nodelay(true).expect("fixture TCP_NODELAY"));
            axum::serve(listener, app).await.unwrap()
        });
        let service_file = profile.path().join("service.json");
        std::fs::write(
            &service_file,
            serde_json::to_vec(&json!({
                "url":format!("http://{address}"),"pid":4242,"version":"2.1.4"
            }))
            .unwrap(),
        )
        .unwrap();
        let executable = profile.path().join("opencode-fixture");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(&executable, "#!/bin/sh\nprintf 'opencode v2.1.4\\n'\n").unwrap();
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let engine = Engine::spawn(
            EngineConfig {
                data_dir: Some(profile.path().join("profile")),
                service_file: Some(service_file),
                opencode_binary: Some(executable),
                ..Default::default()
            },
            &tokio::runtime::Handle::current(),
        );
        let mut startup_updates = engine.subscribe();
        wait_for(|| {
            engine.connection() == ConnectionState::Ready
                && engine.sessions().len() == ids.len()
                && events.receiver_count() > 0
        })
        .await;
        for id in &ids {
            engine.open_session(id.clone());
            wait_for(|| {
                engine.view(id).is_some_and(|view| view.presentation().blocks.len() >= history + 2)
            })
            .await;
        }
        // The sessions share an otherwise idle workspace. Drain its initial
        // observation startup before GPUI's same-thread test scheduler starts
        // listening. Native watch setup can finish after history hydration.
        if with_workspace {
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    if let EngineUpdate::WorkspaceFiles { tree_changed, error, .. } =
                        startup_updates.recv().await.unwrap()
                        && (tree_changed || error.is_some())
                    {
                        break;
                    }
                }
            })
            .await
            .expect("fixture file watcher did not report startup");
        }
        Self {
            engine,
            events,
            server,
            _profile: profile,
            prompts,
            appended_messages,
            forms: pending_forms,
        }
    }

    pub fn send(&self, kind: &str, data: Value) {
        self.events.send(json!({"type":kind,"data":data})).unwrap();
    }

    #[allow(dead_code)]
    pub fn send_identified(&self, id: &str, kind: &str, data: Value) {
        self.events.send(json!({"id":id,"type":kind,"created":1000,"data":data})).unwrap();
    }

    #[allow(dead_code)]
    pub fn journal_path(&self, session: &str) -> std::path::PathBuf {
        for instance in std::fs::read_dir(self._profile.path().join("profile/stream-journal")).unwrap() {
            for file in std::fs::read_dir(instance.unwrap().path()).unwrap() {
                let path = file.unwrap().path();
                let records = vela_engine::stream_journal::read_records(&path).unwrap();
                if records.iter().any(|record| record.session_id() == Some(session)) { return path; }
            }
        }
        panic!("missing synthetic session journal");
    }

    #[allow(dead_code)]
    pub fn user_arrived(&self, session: &str, id: &str, text: &str, at: i64) {
        let message = json!({"sessionID":session,"id":id,"type":"user","text":text,"time":{"created":at}});
        self.appended_messages.lock().unwrap().entry(session.into()).or_default().push(message.clone());
        self.send("message.updated", message);
    }

    /// Targeted todo notifications overflow a genuinely lagged subscriber.
    /// Transcript execution events intentionally publish only once per batch.
    /// Stay below the fixture's bounded source-event capacity.
    pub fn overflow_notifications(&self, session: &str) {
        for index in 0..1_152 {
            self.send(
                "todo.updated",
                json!({"sessionID":session,"todos":[{"id":"overflow","content":format!("Synthetic update {index}"),"status":"pending"}]}),
            );
        }
    }

    pub fn finish_text(&self, session: &str, text: &str) {
        self.send(
            "session.step.started",
            json!({"sessionID":session,"assistantMessageID":"live"}),
        );
        self.send(
            "session.text.delta",
            json!({"sessionID":session,"assistantMessageID":"live","ordinal":0,"delta":text}),
        );
        self.send(
            "session.step.ended",
            json!({"sessionID":session,"assistantMessageID":"live","finish":"stop"}),
        );
        // Flush the ordinary text/step events and settle the actual run.
        self.send("session.execution.succeeded", json!({"sessionID":session}));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.engine.shutdown();
        self.server.abort();
    }
}

pub async fn wait_for(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("publication fixture did not reach its expected state");
}

