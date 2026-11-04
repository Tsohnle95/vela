use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::sync::broadcast;

use vela_engine::{Engine, EngineConfig, EngineUpdate, Settings};

#[derive(Clone)]
struct RestartState {
    sessions: Arc<AtomicUsize>,
    permissions: Arc<AtomicUsize>,
    events: Arc<AtomicUsize>,
    disconnect: broadcast::Sender<()>,
}

impl RestartState {
    fn new() -> Self {
        let (disconnect, _) = broadcast::channel(1);
        Self {
            sessions: Arc::default(),
            permissions: Arc::default(),
            events: Arc::default(),
            disconnect,
        }
    }
}

async fn restart_info() -> Json<Value> {
    Json(json!({"version":"2.1.4","pid":4242}))
}

async fn restart_sessions(State(state): State<RestartState>) -> Json<Value> {
    state.sessions.fetch_add(1, Ordering::SeqCst);
    Json(json!({"data":[],"cursor":{}}))
}

async fn restart_permissions(State(state): State<RestartState>) -> Json<Value> {
    state.permissions.fetch_add(1, Ordering::SeqCst);
    Json(json!([]))
}

async fn restart_events(
    State(state): State<RestartState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    state.events.fetch_add(1, Ordering::SeqCst);
    let receiver = state.disconnect.subscribe();
    let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
        loop {
            match receiver.recv().await {
                Ok(()) | Err(broadcast::error::RecvError::Closed) => return None,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    });
    Sse::new(stream)
}

async fn start_restart_server(state: RestartState) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/api/info", get(restart_info))
        .route("/api/session", get(restart_sessions))
        .route("/api/permission/request", get(restart_permissions))
        .route("/api/event", get(restart_events))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (address, server)
}

fn write_service_registration(path: &std::path::Path, address: std::net::SocketAddr) {
    std::fs::write(
        path,
        serde_json::to_vec(&json!({
            "url": format!("http://{address}"),
            "pid": 4242,
            "version": "2.1.4"
        }))
        .unwrap(),
    )
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn engine_reconnects_after_service_moves_ports_and_rehydrates() {
    let first = RestartState::new();
    let second = RestartState::new();
    let (first_address, first_server) = start_restart_server(first.clone()).await;
    let (second_address, second_server) = start_restart_server(second.clone()).await;

    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("opencode")).unwrap();
    let service_file = temp.path().join("opencode/service.json");
    write_service_registration(&service_file, first_address);

    let fake_cli = temp.path().join("opencode-fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&fake_cli, "#!/bin/sh\nsleep 0.1\nprintf 'opencode v2.1.4\\n'\n")
            .unwrap();
        std::fs::set_permissions(&fake_cli, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let engine = Engine::spawn(
        EngineConfig {
            data_dir: Some(temp.path().to_path_buf()),
            service_file: Some(service_file.clone()),
            opencode_binary: Some(fake_cli),
            ..Default::default()
        },
        &tokio::runtime::Handle::current(),
    );
    wait_for(Duration::from_secs(10), || {
        engine.connection() == vela_engine::ConnectionState::Ready
            && first.sessions.load(Ordering::SeqCst) >= 1
            && first.permissions.load(Ordering::SeqCst) >= 1
    })
    .await;
    assert_eq!(first.events.load(Ordering::SeqCst), 1);

    write_service_registration(&service_file, second_address);
    first.disconnect.send(()).unwrap();
    first_server.abort();
    wait_for(Duration::from_secs(15), || {
        engine.connection() == vela_engine::ConnectionState::Ready
            && second.sessions.load(Ordering::SeqCst) >= 1
            && second.permissions.load(Ordering::SeqCst) >= 1
            && second.events.load(Ordering::SeqCst) >= 1
    })
    .await;
    assert_eq!(
        second.events.load(Ordering::SeqCst),
        1,
        "the replaced client opens exactly one replacement event stream"
    );

    // A second outage at the same endpoint also re-discovers and rehydrates
    // instead of only retrying the old transport forever.
    second.disconnect.send(()).unwrap();
    wait_for(Duration::from_secs(15), || {
        engine.connection() == vela_engine::ConnectionState::Ready
            && second.sessions.load(Ordering::SeqCst) >= 2
            && second.permissions.load(Ordering::SeqCst) >= 2
            && second.events.load(Ordering::SeqCst) >= 2
    })
    .await;

    engine.shutdown_and_wait().await;
    second_server.abort();
}

#[tokio::test]
async fn canonical_history_client_accepts_raw_arrays_and_continuation_pages() {
    async fn history(
        Path(id): Path<String>,
        Query(query): Query<std::collections::HashMap<String, String>>,
    ) -> Json<Value> {
        if id == "raw" {
            assert_eq!(query.get("order").map(String::as_str), Some("asc"));
            return Json(json!([{"info":{"id":"raw-message","role":"user"},"parts":[]}]));
        }
        match query.get("cursor").map(String::as_str) {
            None => {
                assert_eq!(query.get("order").map(String::as_str), Some("asc"));
                Json(json!({"data":[{"id":"first"}], "cursor":{"next":"next 🌍 & page"}}))
            }
            Some("next 🌍 & page") => {
                assert!(!query.contains_key("order"));
                Json(json!({"data":[{"id":"second"}], "cursor":{"next":""}}))
            }
            other => panic!("unexpected continuation: {other:?}"),
        }
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/api/session/{id}/message", get(history)),
        )
        .await
        .unwrap();
    });
    let client = vela_opencode::Client::new(vela_opencode::Endpoint {
        url: format!("http://{address}"),
        auth: None,
        version: None,
        pid: None,
    })
    .unwrap();
    assert_eq!(
        client.load_messages("raw").await.unwrap()[0]["info"]["id"],
        "raw-message"
    );
    assert_eq!(
        client.load_messages("paged").await.unwrap(),
        json!([{"id":"first"},{"id":"second"}])
    );
    server.abort();
}

#[tokio::test]
async fn settings_handoff_persists_and_updates_engine_warm_limits() {
    let temp = tempfile::tempdir().unwrap();
    let engine = Engine::spawn(
        EngineConfig {
            data_dir: Some(temp.path().to_path_buf()),
            ..Default::default()
        },
        &tokio::runtime::Handle::current(),
    );
    let mut updates = engine.subscribe();
    let mut settings = engine.settings();
    settings.appearance = "classic".into();
    settings.shell = "classic".into();
    settings.warm_sessions = 3;
    settings.warm_bytes = 9 * 1024 * 1024;
    settings.favorite_models.insert("provider:model".into());
    engine.apply_settings(settings.clone());
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if matches!(updates.recv().await, Ok(EngineUpdate::SettingsApplied { .. })) {
                break;
            }
        }
    })
    .await
    .expect("settings handoff finishes");

    assert_eq!(engine.settings().appearance, "classic");
    assert_eq!(engine.settings().shell, "classic");
    assert_eq!(engine.warm_limits(), (3, 9 * 1024 * 1024));
    let loaded = Settings::load(Some(temp.path()));
    assert_eq!(loaded.appearance, "classic");
    assert_eq!(loaded.warm_sessions, 3);
    assert!(loaded.favorite_models.contains("provider:model"));

    let mut clamped = engine.settings();
    clamped.warm_sessions = 0;
    clamped.warm_bytes = 1;
    engine.apply_settings(clamped);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if matches!(updates.recv().await, Ok(EngineUpdate::SettingsApplied { .. })) {
                break;
            }
        }
    })
    .await
    .expect("clamped handoff finishes");
    assert_eq!(engine.warm_limits(), (6, 96 * 1024 * 1024));
    assert_eq!(Settings::load(Some(temp.path())).warm_sessions, 6);
    engine.shutdown_and_wait().await;
}

#[derive(Clone)]
struct TestState {
    events: broadcast::Sender<Value>,
    prompts: Arc<Mutex<Vec<Value>>>,
    prompts_to_fail: Arc<AtomicUsize>,
    replies: Arc<Mutex<Vec<Value>>>,
    message_requests: Arc<AtomicUsize>,
    block_next_history: Arc<AtomicBool>,
    history_release: Arc<tokio::sync::Notify>,
    session_deletes: Arc<Mutex<Vec<String>>>,
    deletes_to_fail: Arc<AtomicUsize>,
    switches: Arc<Mutex<Vec<(String, Value)>>>,
    inbox_actions: Arc<Mutex<Vec<(String, Value)>>>,
    provider_keys: Arc<Mutex<Vec<Value>>>,
    provider_removals: Arc<Mutex<Vec<String>>>,
    created_workspaces: Arc<Mutex<Vec<String>>>,
    revert_stages: Arc<Mutex<Vec<Value>>>,
    revert_commits: Arc<AtomicUsize>,
    revert_clears: Arc<AtomicUsize>,
    revert_fail_commit: Arc<AtomicBool>,
    forms: Arc<Mutex<Vec<Value>>>,
    location_forms: Arc<Mutex<Vec<Value>>>,
    form_replies: Arc<Mutex<Vec<Value>>>,
    form_cancels: Arc<Mutex<Vec<String>>>,
    compacts: Arc<AtomicUsize>,
    commands: Arc<Mutex<Vec<Value>>>,
    skill_activations: Arc<Mutex<Vec<String>>>,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn end_to_end_replay_stream_prompt_and_permission() {
    let (events, _) = broadcast::channel(64);
    let state = TestState {
        events,
        prompts: Arc::default(),
        prompts_to_fail: Arc::default(),
        replies: Arc::default(),
        message_requests: Arc::default(),
        block_next_history: Arc::default(),
        history_release: Arc::default(),
        session_deletes: Arc::default(),
        deletes_to_fail: Arc::new(AtomicUsize::new(1)),
        switches: Arc::default(),
        inbox_actions: Arc::default(),
        provider_keys: Arc::default(),
        provider_removals: Arc::default(),
        created_workspaces: Arc::default(),
        revert_stages: Arc::default(),
        revert_commits: Arc::default(),
        revert_clears: Arc::default(),
        revert_fail_commit: Arc::default(),
        forms: Arc::new(Mutex::new(vec![json!({
            "id":"form_fixture",
            "title":"Fixture form",
            "fields":[
                {"key":"name","type":"string","title":"Name","required":true},
                {"key":"targets","type":"multiselect","options":[{"value":"a","label":"A"}]},
                {"key":"enabled","type":"boolean","default":false}
            ]
        })])),
        form_replies: Arc::default(),
        form_cancels: Arc::default(),
        location_forms: Arc::new(Mutex::new(vec![json!({
            "id":"form_global",
            "sessionID":"global",
            "title":"Location form",
            "fields":[{"key":"choice","type":"string","required":true}]
        })])),
        compacts: Arc::default(),
        commands: Arc::default(),
        skill_activations: Arc::default(),
    };
    let app = Router::new()
        .route("/api/info", get(info))
        .route("/api/session", get(sessions).post(create_workspace))
        .route("/api/session/{id}", get(session).delete(remove_session))
        .route("/api/session/{id}/message", get(messages))
        .route("/api/session/{id}/prompt", post(prompt))
        .route("/api/session/{id}/interrupt", post(interrupt))
        .route("/api/session/{id}/revert/stage", post(stage_revert))
        .route("/api/session/{id}/revert/commit", post(commit_revert))
        .route("/api/session/{id}/revert", delete(clear_revert))
        .route("/api/session/{id}/form", get(list_forms))
        .route("/api/form", get(list_location_forms))
        .route("/api/session/{id}/form/{fid}/reply", post(reply_form))
        .route("/api/session/{id}/form/{fid}", delete(cancel_form))
        .route("/api/session/{id}/compact", post(compact_session))
        .route("/api/session/{id}/command", post(run_command))
        .route("/api/skill", get(list_skills))
        .route("/api/command", get(list_commands))
        .route("/api/fs/find", get(find_files))
        .route(
            "/api/experimental/session/{id}/skill",
            post(activate_skill),
        )
        .route("/api/session/{id}/permission/{rid}/reply", post(reply))
        .route("/api/permission/request", get(permissions))
        .route("/api/model", get(models))
        .route("/api/model/default", get(default_model))
        .route("/api/agent", get(agents))
        .route("/api/integration", get(integrations))
        .route("/api/credential/{id}", delete(remove_provider_credential))
        .route(
            "/api/integration/{id}/connect/key",
            post(connect_provider_key),
        )
        .route("/api/session/{id}/model", post(switch_model))
        .route("/api/session/{id}/agent", post(switch_agent))
        .route("/api/session/{id}/inbox", get(inbox))
        .route(
            "/api/session/{id}/inbox/{iid}",
            delete(cancel_inbox).patch(steer_inbox),
        )
        .route("/api/event", get(event_stream))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all("/tmp/vela-test").unwrap();
    std::fs::write("/tmp/vela-test/README.md", "# test\n").unwrap();
    std::fs::create_dir(temp.path().join("opencode")).unwrap();
    let service_file = temp.path().join("opencode/service.json");
    std::fs::write(
        &service_file,
        serde_json::to_vec(&json!({
            "url": format!("http://{address}"),
            "pid": 4242,
            "version": "2.1.4"
        }))
        .unwrap(),
    )
    .unwrap();

    let fake_cli = temp.path().join("opencode-fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(
            &fake_cli,
            "#!/bin/sh\nsleep 0.1\nprintf 'opencode v2.1.4\\n'\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake_cli, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let config = EngineConfig {
        warm_sessions: 2,
        warm_bytes: 8 * 1024 * 1024,
        data_dir: Some(temp.path().to_path_buf()),
        service_file: Some(service_file),
        opencode_binary: Some(fake_cli),
    };
    let engine = Engine::spawn(config, &tokio::runtime::Handle::current());
    let mut updates = engine.subscribe();
    wait_for(Duration::from_secs(10), || {
        engine.connection() == vela_engine::ConnectionState::Ready
    })
    .await;

    // Initial list must arrive from the engine, not just from a UI poll.
    let mut saw_sessions = false;
    for _ in 0..50 {
        if let Ok(update) = updates.try_recv()
            && matches!(update, EngineUpdate::Sessions)
        {
            saw_sessions = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(saw_sessions, "engine should publish the session index");

    engine.open_session("ses_test");
    wait_for(Duration::from_secs(5), || {
        engine
            .view("ses_test")
            .is_some_and(|view| view.blocks().len() >= 2)
    })
    .await;
    let view = engine.view("ses_test").unwrap();
    let blocks = view.blocks();
    assert_eq!(blocks[0].text.as_ref(), "hello from replay");
    assert_eq!(blocks[1].text.as_ref(), "assistant replay");

    // Standalone external documents read outside the workspace root without
    // creating workspace recovery beside the file.
    let external = temp.path().join("standalone 界 🌍.txt");
    std::fs::write(&external, "outside bytes\n").unwrap();
    let mut external_updates = engine.subscribe();
    engine.read_document(
        7001,
        vela_core::document::DocumentKey::in_workspace(
            "standalone-scope".into(),
            temp.path().to_string_lossy().into_owned(),
            external.to_string_lossy().into_owned(),
        ),
    );
    let external_content = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let EngineUpdate::FileContent {
                request_id: 7001,
                content,
                error,
                ..
            } = external_updates.recv().await.unwrap()
            {
                break (content, error);
            }
        }
    })
    .await
    .expect("standalone read did not complete");
    assert_eq!(external_content.0.as_deref(), Some("outside bytes\n"));
    assert_eq!(external_content.1, None);
    assert!(!temp.path().join(".vela-recovery").exists());

    // Revert stages with files, commits, then reloads the authoritative history.
    let before_revert_history = state.message_requests.load(Ordering::SeqCst);
    let mut revert_updates = engine.subscribe();
    engine.revert_session(7002, "ses_test", "user-1");
    let revert_error = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let EngineUpdate::RevertFinished {
                request_id: 7002,
                error,
                ..
            } = revert_updates.recv().await.unwrap()
            {
                break error;
            }
        }
    })
    .await
    .expect("revert did not finish");
    assert_eq!(revert_error, None);
    assert_eq!(state.revert_commits.load(Ordering::SeqCst), 1);
    assert_eq!(state.revert_clears.load(Ordering::SeqCst), 0);
    assert_eq!(
        state.revert_stages.lock().unwrap()[0],
        json!({"messageID":"user-1","files":true})
    );
    wait_for(Duration::from_secs(3), || {
        state.message_requests.load(Ordering::SeqCst) > before_revert_history
    })
    .await;

    // A commit failure retains the staged boundary and announces it so the UI
    // can offer Commit undo/Discard instead of silently clearing the stage.
    state.revert_fail_commit.store(true, Ordering::SeqCst);
    let mut revert_failure_updates = engine.subscribe();
    engine.revert_session(7003, "ses_test", "user-2");
    let (revert_failure, staged_seen) = tokio::time::timeout(Duration::from_secs(5), async {
        let mut staged_seen = false;
        loop {
            match revert_failure_updates.recv().await.unwrap() {
                EngineUpdate::RevertStaged { session_id } if session_id == "ses_test" => {
                    staged_seen = true;
                }
                EngineUpdate::RevertFinished {
                    request_id: 7003,
                    error,
                    ..
                } => break (error, staged_seen),
                _ => {}
            }
        }
    })
    .await
    .expect("failed revert did not finish");
    assert!(revert_failure.is_some());
    assert!(staged_seen);
    assert_eq!(state.revert_clears.load(Ordering::SeqCst), 0);
    state.revert_fail_commit.store(false, Ordering::SeqCst);

    // Commit undo consumes the retained stage without clearing it first.
    let mut commit_updates = engine.subscribe();
    engine.commit_revert(7004, "ses_test");
    let commit_error = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let EngineUpdate::RevertFinished {
                request_id: 7004,
                error,
                ..
            } = commit_updates.recv().await.unwrap()
            {
                break error;
            }
        }
    })
    .await
    .expect("staged revert commit did not finish");
    assert_eq!(commit_error, None);
    assert_eq!(state.revert_clears.load(Ordering::SeqCst), 0);

    // Discard remains available for a staged boundary and clears it explicitly.
    state.revert_fail_commit.store(true, Ordering::SeqCst);
    let mut discard_stage_updates = engine.subscribe();
    engine.revert_session(7005, "ses_test", "user-3");
    let discard_stage_error = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let EngineUpdate::RevertFinished {
                request_id: 7005,
                error,
                ..
            } = discard_stage_updates.recv().await.unwrap()
            {
                break error;
            }
        }
    })
    .await
    .expect("discard setup revert did not finish");
    assert!(discard_stage_error.is_some());
    state.revert_fail_commit.store(false, Ordering::SeqCst);
    let mut discard_updates = engine.subscribe();
    engine.clear_revert(7006, "ses_test");
    let discard_error = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let EngineUpdate::RevertFinished {
                request_id: 7006,
                error,
                ..
            } = discard_updates.recv().await.unwrap()
            {
                break error;
            }
        }
    })
    .await
    .expect("staged revert discard did not finish");
    assert_eq!(discard_error, None);
    assert_eq!(state.revert_clears.load(Ordering::SeqCst), 1);

    // Pending forms are normalized from the session endpoint and replies use the
    // exact `answer` object expected by the runtime.
    let mut form_updates = engine.subscribe();
    engine.refresh_forms("ses_test");
    let forms = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let EngineUpdate::Forms {
                session_id,
                forms,
                error,
            } = form_updates.recv().await.unwrap()
                && session_id == "ses_test"
            {
                break (forms, error);
            }
        }
    })
    .await
    .expect("forms refresh did not finish");
    assert_eq!(forms.1, None);
    assert_eq!(forms.0.len(), 2);
    assert_eq!(forms.0[0].fields[0].key, "name");
    assert!(forms.0[0].fields[0].required);
    assert_eq!(forms.0[0].fields[1].options[0].value, "a");
    assert_eq!(forms.0[0].fields[2].default, Some(json!(false)));
    assert_eq!(forms.0[1].id, "form_global");
    assert_eq!(forms.0[1].session_id, "global");

    let answers = json!({"name":"api","targets":["a"],"enabled":true})
        .as_object()
        .unwrap()
        .clone();
    engine.reply_form(7004, "ses_test", "ses_test", "form_fixture", answers);
    let form_finished = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let EngineUpdate::FormFinished {
                request_id: 7004,
                error,
                ..
            } = form_updates.recv().await.unwrap()
            {
                break error;
            }
        }
    })
    .await
    .expect("form reply did not finish");
    assert_eq!(form_finished, None);
    wait_for(Duration::from_secs(3), || {
        state.form_replies.lock().unwrap().len() == 1
    })
    .await;
    let reply = state.form_replies.lock().unwrap()[0].clone();
    assert_eq!(reply["formID"], "form_fixture");
    assert_eq!(
        reply["body"],
        json!({"answer":{"name":"api","targets":["a"],"enabled":true}})
    );
    assert_eq!(reply["directory"], Value::Null);

    // Location-global forms are attributed as global and replies carry the
    // encoded owning directory header.
    let global_answers = json!({"choice":"yes"}).as_object().unwrap().clone();
    engine.reply_form(7006, "ses_test", "global", "form_global", global_answers);
    let global_finished = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let EngineUpdate::FormFinished {
                request_id: 7006,
                error,
                ..
            } = form_updates.recv().await.unwrap()
            {
                break error;
            }
        }
    })
    .await
    .expect("global form reply did not finish");
    assert_eq!(global_finished, None);
    wait_for(Duration::from_secs(3), || {
        state.form_replies.lock().unwrap().len() == 2
    })
    .await;
    let global_reply = state.form_replies.lock().unwrap()[1].clone();
    assert_eq!(global_reply["formID"], "form_global");
    assert_eq!(
        global_reply["directory"],
        "%2Ftmp%2Fvela-test"
    );

    // Slash commands route built-ins, runtime commands and skills separately.
    let mut slash_updates = engine.subscribe();
    engine.run_slash_command(7007, "ses_test", "compact", "");
    engine.run_slash_command(7008, "ses_test", "deploy", "prod");
    engine.run_slash_command(7009, "ses_test", "review", "");
    let mut finished = [false; 3];
    tokio::time::timeout(Duration::from_secs(5), async {
        while finished.iter().any(|done| !done) {
            if let EngineUpdate::SlashCommandFinished {
                request_id,
                error,
                ..
            } = slash_updates.recv().await.unwrap()
            {
                assert_eq!(error, None);
                if (7007..=7009).contains(&request_id) {
                    finished[(request_id - 7007) as usize] = true;
                }
            }
        }
    })
    .await
    .expect("slash commands did not finish");
    assert_eq!(state.compacts.load(Ordering::SeqCst), 1);
    wait_for(Duration::from_secs(3), || {
        state.commands.lock().unwrap().len() == 1
            && state.skill_activations.lock().unwrap().len() == 1
    })
    .await;
    assert_eq!(
        state.commands.lock().unwrap()[0],
        json!({"name":"deploy","text":"prod"})
    );
    assert_eq!(
        state.skill_activations.lock().unwrap().as_slice(),
        ["skill_review"]
    );

    let mut catalog_updates = engine.subscribe();
    engine.refresh_commands("ses_test");
    let catalog = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let EngineUpdate::CommandCatalog {
                session_id,
                commands,
                error,
            } = catalog_updates.recv().await.unwrap()
                && session_id == "ses_test"
            {
                break (commands, error);
            }
        }
    })
    .await
    .expect("command catalog did not finish");
    assert_eq!(catalog.1, None);
    let names = catalog
        .0
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["compact", "compress", "deploy", "review"]);

    let mut reference_updates = engine.subscribe();
    engine.find_references(7010, "ses_test", "ma");
    let references = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let EngineUpdate::References {
                request_id: 7010,
                files,
                error,
                ..
            } = reference_updates.recv().await.unwrap()
            {
                break (files, error);
            }
        }
    })
    .await
    .expect("reference search did not finish");
    assert_eq!(references.1, None);
    assert_eq!(references.0, ["src/main.rs", "README.md"]);

    engine.cancel_form(7005, "ses_test", "ses_test", "form_fixture");
    let cancel_finished = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let EngineUpdate::FormFinished {
                request_id: 7005,
                error,
                ..
            } = form_updates.recv().await.unwrap()
            {
                break error;
            }
        }
    })
    .await
    .expect("form cancel did not finish");
    assert_eq!(cancel_finished, None);
    wait_for(Duration::from_secs(3), || {
        state.form_cancels.lock().unwrap().as_slice() == ["form_fixture"]
    })
    .await;

    // Live delta updates must append to the authoritative replay part.
    state
        .events
        .send(json!({
            "type": "session.text.delta",
            "data": {
                "sessionID": "ses_test",
                "assistantMessageID": "msg_a",
                "ordinal": 0,
                "delta": " + live"
            }
        }))
        .unwrap();
    wait_for(Duration::from_secs(5), || {
        engine.view("ses_test").is_some_and(|view| {
            view.blocks()
                .iter()
                .any(|block| block.text.contains("+ live"))
        })
    })
    .await;

    engine.queue_prompt("ses_test", "ship it");
    wait_for(Duration::from_secs(5), || {
        state.prompts.lock().unwrap().len() == 1
    })
    .await;
    let sent = state.prompts.lock().unwrap()[0].clone();
    assert_eq!(sent["text"], "ship it");
    assert_eq!(sent["delivery"], "queue");

    // Staged paths must survive both immediate file-only delivery and queueing.
    let attached = temp.path().join("Unicode 🌍.png");
    std::fs::write(&attached, b"disposable attachment").unwrap();
    let attached_uri = url::Url::from_file_path(&attached).unwrap().to_string();
    for queued in [false, true] {
        let expected_count = if queued { 3 } else { 2 };
        engine.prompt_with_files(
            "ses_test",
            "",
            vec![vela_opencode::PromptFile {
                path: attached.to_string_lossy().into_owned(),
                name: None,
                mention: None,
            }],
            queued,
        );
        wait_for(Duration::from_secs(5), || {
            state.prompts.lock().unwrap().len() == expected_count
        })
        .await;
        let sent = state.prompts.lock().unwrap()[expected_count - 1].clone();
        assert_eq!(sent["text"], "");
        assert_eq!(sent["files"][0]["uri"], attached_uri);
        assert_eq!(sent["files"][0]["name"], "Unicode 🌍.png");
        assert!(sent["files"][0].get("path").is_none());
        assert_eq!(
            sent.get("delivery").and_then(Value::as_str),
            queued.then_some("queue")
        );
    }

    // Failed queue delivery must recover the complete prompt durably, with no
    // cross-workspace leakage, then remove it only after a successful retry.
    state.prompts_to_fail.store(1, Ordering::SeqCst);
    engine.prompt_with_files(
        "ses_test",
        "Recover this\n🌍",
        vec![vela_opencode::PromptFile {
            path: attached.to_string_lossy().into_owned(),
            name: None,
            mention: None,
        }],
        true,
    );
    wait_for(Duration::from_secs(5), || {
        !engine.local_queue("/tmp/vela-test", "ses_test").is_empty()
    })
    .await;
    let recovered = engine.local_queue("/tmp/vela-test", "ses_test");
    assert_eq!(recovered[0].text, "Recover this\n🌍");
    assert_eq!(recovered[0].files[0].path, attached.to_string_lossy());
    assert!(engine.local_queue("/other", "ses_test").is_empty());
    let disk = std::fs::read_to_string(temp.path().join("message-queues.json")).unwrap();
    assert!(disk.contains("Unicode 🌍.png"));
    let count = state.prompts.lock().unwrap().len();
    state.prompts_to_fail.store(1, Ordering::SeqCst);
    engine.retry_queued(
        "/tmp/vela-test".into(),
        "ses_test".into(),
        recovered[0].id.clone(),
    );
    wait_for(Duration::from_secs(5), || {
        state.prompts.lock().unwrap().len() == count + 1
            && !engine.local_queue("/tmp/vela-test", "ses_test").is_empty()
    })
    .await;
    let retry_id = engine.local_queue("/tmp/vela-test", "ses_test")[0]
        .id
        .clone();
    assert_eq!(
        engine.local_queue("/tmp/vela-test", "ses_test")[0].files[0].path,
        recovered[0].files[0].path
    );
    engine.retry_queued("/tmp/vela-test".into(), "ses_test".into(), retry_id);
    wait_for(Duration::from_secs(5), || {
        state.prompts.lock().unwrap().len() == count + 2
            && engine.local_queue("/tmp/vela-test", "ses_test").is_empty()
    })
    .await;
    let retry = state.prompts.lock().unwrap().last().unwrap().clone();
    assert_eq!(retry["text"], "Recover this\n🌍");
    assert_eq!(retry["files"][0]["uri"], attached_uri);
    assert!(retry.get("delivery").is_none());

    // Current V2 execution lifecycle events may arrive before any step event.
    // They must drive the public view used by the composer's Send/Stop state.
    for (event_type, expected) in [
        (
            "session.execution.succeeded",
            vela_core::session::SessionPhase::Idle,
        ),
        (
            "session.execution.started",
            vela_core::session::SessionPhase::Busy,
        ),
        (
            "session.execution.interrupted",
            vela_core::session::SessionPhase::Error,
        ),
        (
            "session.execution.succeeded",
            vela_core::session::SessionPhase::Idle,
        ),
    ] {
        state
            .events
            .send(json!({"type": event_type, "data": {"sessionID":"ses_test"}}))
            .unwrap();
        wait_for(Duration::from_secs(5), || {
            engine
                .view("ses_test")
                .is_some_and(|view| view.phase() == expected)
        })
        .await;
    }

    // Retry and failure records must reach the same retained transcript used
    // by the UI, and clear both the retry notice and Stop state on failure.
    for (kind, data) in [
        (
            "session.step.started",
            json!({"sessionID":"ses_test","assistantMessageID":"lifecycle"}),
        ),
        (
            "session.text.delta",
            json!({"sessionID":"ses_test","assistantMessageID":"lifecycle","ordinal":0,"delta":"Partial response"}),
        ),
        (
            "session.retry.scheduled",
            json!({"sessionID":"ses_test","assistantMessageID":"lifecycle","attempt":2,"at":9999999999999i64,
            "error":{"code":"provider.limit","message":"Try later"}}),
        ),
    ] {
        state.events.send(json!({"type":kind,"data":data})).unwrap();
    }
    wait_for(Duration::from_secs(5), || {
        engine.view("ses_test").unwrap().phase() == vela_core::session::SessionPhase::Retry
    })
    .await;
    let view = engine.view("ses_test").unwrap();
    view.with_transcript(|transcript| {
        let retry = transcript
            .message("lifecycle")
            .unwrap()
            .retry
            .as_ref()
            .unwrap();
        assert_eq!(retry.next, Some(9999999999999));
        assert_eq!(retry.message, "[PROVIDER.LIMIT] Try later");
    });
    state.events.send(json!({"type":"session.step.failed","data":{"sessionID":"ses_test","assistantMessageID":"lifecycle",
        "error":{"code":"provider.failed","message":"Failure fixture"}}})).unwrap();
    wait_for(Duration::from_secs(5), || {
        view.phase() == vela_core::session::SessionPhase::Error
    })
    .await;
    view.with_transcript(|transcript| {
        let message = transcript.message("lifecycle").unwrap();
        assert!(message.retry.is_none());
        assert!(message.completed_at.is_some());
        assert_eq!(
            message.error.as_deref(),
            Some("[PROVIDER.FAILED] Failure fixture")
        );
    });
    let blocks = view.blocks();
    assert!(
        !blocks
            .iter()
            .any(|block| block.kind == vela_core::transcript::BlockKind::Retry)
    );
    assert!(blocks.iter().any(
        |block| block.kind == vela_core::transcript::BlockKind::Error
            && block.text.as_ref() == "[PROVIDER.FAILED] Failure fixture"
    ));
    state
        .events
        .send(json!({"type":"session.execution.succeeded","data":{"sessionID":"ses_test"}}))
        .unwrap();
    wait_for(Duration::from_secs(5), || {
        view.phase() == vela_core::session::SessionPhase::Idle
    })
    .await;

    state.events.send(json!({"type":"session.step.started","data":{"sessionID":"ses_test","assistantMessageID":"idle-settlement"}})).unwrap();
    state.events.send(json!({"type":"session.text.delta","data":{"sessionID":"ses_test","assistantMessageID":"idle-settlement","ordinal":0,"delta":"Quiet incomplete response"}})).unwrap();
    state.events.send(json!({"type":"session.status","data":{"sessionID":"ses_test","status":{"type":"idle"}}})).unwrap();
    wait_for(Duration::from_secs(5), || {
        view.with_transcript(|transcript| {
            transcript
                .message("idle-settlement")
                .is_some_and(|message| message.completed_at.is_some())
        })
    })
    .await;
    assert_eq!(view.phase(), vela_core::session::SessionPhase::Idle);

    let mut permission_events = engine.subscribe();
    state
        .events
        .send(json!({
            "type": "permission.asked",
            "data": {
                "id": "req_1",
                "sessionID": "ses_test",
                "action": "bash",
                "resources": ["git status"]
            }
        }))
        .unwrap();
    wait_for(Duration::from_secs(5), || {
        engine.permissions("ses_test").len() == 1
    })
    .await;
    let mut new_requests = 0;
    wait_for(Duration::from_secs(5), || {
        while let Ok(update) = permission_events.try_recv() {
            if let EngineUpdate::PermissionAsked(request) = update {
                assert_eq!(request.id, "req_1");
                assert_eq!(request.session_id, "ses_test");
                new_requests += 1;
            }
        }
        new_requests == 1
    })
    .await;
    state
        .events
        .send(json!({"type":"permission.asked", "data": {
            "id":"req_1", "sessionID":"ses_test", "action":"bash", "resources":["updated duplicate"]
        }}))
        .unwrap();
    wait_for(Duration::from_secs(5), || {
        engine
            .permissions("ses_test")
            .first()
            .is_some_and(|request| request.resources == ["updated duplicate"])
    })
    .await;
    while let Ok(update) = permission_events.try_recv() {
        assert!(
            !matches!(update, EngineUpdate::PermissionAsked(_)),
            "a repeated pending request must not trigger automatic approval twice"
        );
    }

    engine.reply_permission(
        "ses_test",
        "req_1",
        vela_opencode::client::PermissionReply::Allow,
    );
    wait_for(Duration::from_secs(5), || {
        state.replies.lock().unwrap().len() == 1
    })
    .await;
    assert_eq!(state.replies.lock().unwrap()[0]["decision"], "allow");

    // A terminal execution event immediately clears busy state.
    state
        .events
        .send(json!({
            "type": "session.execution.succeeded",
            "data": { "sessionID": "ses_test" }
        }))
        .unwrap();
    wait_for(Duration::from_secs(5), || {
        engine
            .view("ses_test")
            .is_some_and(|view| view.phase() == vela_core::session::SessionPhase::Idle)
    })
    .await;

    // Warm navigation: closing retains the transcript, and reopening promotes
    // it synchronously. Background terminal events must update the warm copy,
    // not reopen a closed view.
    let loaded = engine.view("ses_test").expect("loaded view");
    assert!(loaded.blocks().len() >= 2);
    engine.close_session("ses_test");
    wait_for(Duration::from_secs(5), || engine.view("ses_test").is_none()).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        engine.view("ses_test").is_none(),
        "a warm session must not be reopened by background events"
    );
    engine.open_session("ses_test");
    wait_for(Duration::from_secs(2), || {
        engine
            .view("ses_test")
            .is_some_and(|view| view.blocks().len() >= 2)
    })
    .await;
    let promoted = engine.view("ses_test").unwrap();
    assert!(
        promoted
            .blocks()
            .iter()
            .any(|block| block.text.contains("assistant replay"))
    );
    let requests_before = state.message_requests.load(Ordering::SeqCst);
    engine.open_session("ses_test");
    wait_for(Duration::from_secs(2), || {
        engine
            .view("ses_test")
            .is_some_and(|view| view.blocks().len() >= 2)
    })
    .await;
    let promoted = engine.view("ses_test").unwrap();
    assert!(
        promoted
            .blocks()
            .iter()
            .any(|block| block.text.contains("assistant replay"))
    );
    // Reconciliation may run, but it must not be required for the retained
    // transcript to be visible.
    let _ = requests_before;

    // Confined file access through the engine command boundary.
    let mut listing_updates = engine.subscribe();
    engine.list_dir(1, "ses_test", "");
    wait_for(Duration::from_secs(5), || {
        std::fs::read_to_string("/tmp/vela-test/README.md").is_ok()
    })
    .await;
    let mut listed = false;
    for _ in 0..50 {
        if let Ok(EngineUpdate::Directory { entries, .. }) = listing_updates.try_recv() {
            listed = entries.iter().any(|entry| entry.name == "README.md");
            if listed {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(listed, "file listing should include README.md");
    engine.read_file(2, "ses_test", "README.md");
    wait_for(Duration::from_secs(5), || {
        let mut found = false;
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::FileContent { content, .. } = update
                && content.as_deref() == Some("# test\n")
            {
                found = true;
            }
        }
        found
    })
    .await;
    // Create operations acknowledge scope failures and collisions, not just success.
    let create_sandbox = tempfile::tempdir_in("/tmp/vela-test").unwrap();
    let parent = create_sandbox
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let mut create_updates = engine.subscribe();
    for (id, request_parent, name, is_dir, workspace, succeeds) in [
        (
            8000,
            parent.clone(),
            "new folder",
            true,
            "/tmp/vela-test",
            true,
        ),
        (
            8001,
            format!("{parent}/new folder"),
            "界🌍.txt",
            false,
            "/tmp/vela-test",
            true,
        ),
        (
            8002,
            format!("{parent}/new folder"),
            "界🌍.txt",
            false,
            "/tmp/vela-test",
            false,
        ),
        (
            8003,
            parent.clone(),
            "wrong-scope.txt",
            false,
            "/tmp/other-workspace",
            false,
        ),
    ] {
        engine.create_entry(vela_engine::files::CreateEntryRequest {
            id,
            session: "ses_test".into(),
            workspace: workspace.into(),
            parent: request_parent,
            name: name.into(),
            is_dir,
        });
        let (rel, error) = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let EngineUpdate::EntryCreated {
                    request,
                    rel,
                    error,
                } = create_updates.recv().await.unwrap()
                    && request.id == id
                {
                    break (rel, error);
                }
            }
        })
        .await
        .expect("create operation never acknowledged");
        assert_eq!(error.is_none(), succeeds, "{id}: {error:?}");
        assert_eq!(rel.is_some(), succeeds);
        if id == 8001 {
            std::fs::write(
                create_sandbox.path().join("new folder/界🌍.txt"),
                "retain existing source",
            )
            .unwrap();
        }
    }
    assert_eq!(
        std::fs::read_to_string(create_sandbox.path().join("new folder/界🌍.txt")).unwrap(),
        "retain existing source"
    );
    assert!(!create_sandbox.path().join("wrong-scope.txt").exists());
    let mut rename_updates = engine.subscribe();
    let old = format!("{parent}/new folder/界🌍.txt");
    let renamed = format!("{parent}/new folder/renamed.txt");
    std::fs::write(
        create_sandbox.path().join("new folder/occupied.txt"),
        "do not replace",
    )
    .unwrap();
    let mut rename_read = engine.subscribe();
    engine.read_file(8099, "ses_test", old.clone());
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let EngineUpdate::FileContent {
                request_id: 8099,
                content,
                ..
            } = rename_read.recv().await.unwrap()
            {
                assert_eq!(content.as_deref(), Some("retain existing source"));
                break;
            }
        }
    })
    .await
    .unwrap();
    for (id, rel, name, workspace, succeeds) in [
        (8100, old.clone(), "renamed.txt", "/tmp/vela-test", true),
        (
            8101,
            renamed.clone(),
            "occupied.txt",
            "/tmp/vela-test",
            false,
        ),
        (
            8102,
            renamed.clone(),
            "wrong.txt",
            "/tmp/other-workspace",
            false,
        ),
        (
            8103,
            format!("{parent}/new folder"),
            "moved folder",
            "/tmp/vela-test",
            true,
        ),
    ] {
        engine.rename_entry(vela_engine::files::RenameEntryRequest {
            id,
            target_parent: None,
            session: "ses_test".into(),
            workspace: workspace.into(),
            rel,
            name: name.into(),
        });
        let (new_rel, snapshots, error) = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let EngineUpdate::EntryRenamed {
                    request,
                    new_rel,
                    snapshots,
                    error,
                } = rename_updates.recv().await.unwrap()
                    && request.id == id
                {
                    break (new_rel, snapshots, error);
                }
            }
        })
        .await
        .expect("rename operation never acknowledged");
        assert_eq!(error.is_none(), succeeds, "{id}: {error:?}");
        assert_eq!(new_rel.is_some(), succeeds);
        if id == 8100 || id == 8103 {
            assert_eq!(snapshots.len(), 1);
            assert!(snapshots[0].source_unchanged);
            assert_eq!(
                snapshots[0].change.content.as_deref(),
                Some("retain existing source")
            );
        }
    }
    assert_eq!(
        std::fs::read_to_string(create_sandbox.path().join("moved folder/renamed.txt")).unwrap(),
        "retain existing source"
    );
    assert_eq!(
        std::fs::read_to_string(create_sandbox.path().join("moved folder/occupied.txt")).unwrap(),
        "do not replace"
    );
    assert!(!create_sandbox.path().join("new folder").exists());

    for (id, rel, destination, scope, succeeds) in [
        (
            8200,
            format!("{parent}/moved folder/renamed.txt"),
            parent.clone(),
            "/tmp/vela-test",
            true,
        ),
        (
            8201,
            format!("{parent}/renamed.txt"),
            parent.clone(),
            "/tmp/vela-test",
            false,
        ),
        (
            8202,
            format!("{parent}/renamed.txt"),
            format!("{parent}/moved folder"),
            "/tmp/other-workspace",
            false,
        ),
        (
            8203,
            format!("{parent}/renamed.txt"),
            format!("{parent}/moved folder"),
            "/tmp/vela-test",
            true,
        ),
    ] {
        engine.rename_entry(vela_engine::files::RenameEntryRequest {
            id,
            target_parent: Some(destination),
            session: "ses_test".into(),
            workspace: scope.into(),
            rel,
            name: "ignored-for-move".into(),
        });
        let (new_rel, snapshots, error) = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let EngineUpdate::EntryRenamed {
                    request,
                    new_rel,
                    snapshots,
                    error,
                } = rename_updates.recv().await.unwrap()
                    && request.id == id
                {
                    break (new_rel, snapshots, error);
                }
            }
        })
        .await
        .expect("move operation never acknowledged");
        assert_eq!(error.is_none(), succeeds, "{id}: {error:?}");
        assert_eq!(new_rel.is_some(), succeeds);
        if succeeds {
            assert_eq!(snapshots.len(), 1);
            assert!(snapshots[0].source_unchanged);
        }
    }
    assert_eq!(
        std::fs::read_to_string(create_sandbox.path().join("moved folder/renamed.txt")).unwrap(),
        "retain existing source"
    );

    let mut remove_updates = engine.subscribe();
    for (id, rel, scope) in [
        (
            8300,
            format!("{parent}/moved folder/renamed.txt"),
            "/tmp/other-workspace",
        ),
        (8301, format!("{parent}/missing.txt"), "/tmp/vela-test"),
        (8302, String::new(), "/tmp/vela-test"),
    ] {
        engine.remove_entry(vela_engine::files::RemoveEntryRequest {
            id,
            batch: id,
            session: "ses_test".into(),
            workspace: scope.into(),
            rel,
        });
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let EngineUpdate::EntryRemoved { request, error } =
                    remove_updates.recv().await.unwrap()
                    && request.id == id
                {
                    assert!(error.is_some());
                    break;
                }
            }
        })
        .await
        .expect("failed Trash request never acknowledged");
    }
    assert_eq!(
        std::fs::read_to_string(create_sandbox.path().join("moved folder/renamed.txt")).unwrap(),
        "retain existing source"
    );

    let mut reveal_updates = engine.subscribe();
    for (id, rel, scope, success) in [
        (
            8400,
            format!("{parent}/moved folder/renamed.txt"),
            "/tmp/vela-test",
            true,
        ),
        (8401, String::new(), "/tmp/vela-test", true),
        (
            8402,
            format!("{parent}/missing.txt"),
            "/tmp/vela-test",
            false,
        ),
        (
            8403,
            format!("{parent}/moved folder/renamed.txt"),
            "/tmp/other-workspace",
            false,
        ),
        (8404, "../outside".into(), "/tmp/vela-test", false),
    ] {
        engine.resolve_entry(id, "ses_test".into(), scope.into(), rel.clone());
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let EngineUpdate::EntryResolved {
                    request_id,
                    path,
                    error,
                } = reveal_updates.recv().await.unwrap()
                    && request_id == id
                {
                    assert_eq!(error.is_none(), success, "{id}: {error:?}");
                    if success {
                        assert_eq!(
                            path,
                            Some(
                                std::fs::canonicalize(std::path::Path::new(scope).join(&rel))
                                    .unwrap()
                            )
                        );
                    } else {
                        assert!(path.is_none());
                    }
                    break;
                }
            }
        })
        .await
        .expect("reveal path request never acknowledged");
    }

    for (id, rel, success) in [
        (8500, format!("{parent}/moved folder"), true),
        (8501, format!("{parent}/moved folder/renamed.txt"), false),
        (8502, "../outside".into(), false),
    ] {
        engine.resolve_terminal_directory(id, "ses_test".into(), "/tmp/vela-test".into(), rel);
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let EngineUpdate::EntryResolved {
                    request_id,
                    path,
                    error,
                } = reveal_updates.recv().await.unwrap()
                    && request_id == id
                {
                    assert_eq!(error.is_none(), success);
                    assert_eq!(path.is_some(), success);
                    break;
                }
            }
        })
        .await
        .expect("terminal directory request never acknowledged");
    }

    let mut detach_updates = engine.subscribe();
    for (id, rel, scope) in [
        (
            8700,
            format!("{parent}/moved folder/renamed.txt"),
            "/tmp/other-workspace",
        ),
        (8701, format!("{parent}/missing"), "/tmp/vela-test"),
        (8702, String::new(), "/tmp/vela-test"),
    ] {
        engine.detach_entry(vela_engine::detach::Request {
            id,
            session: "ses_test".into(),
            workspace: scope.into(),
            rel,
        });
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let EngineUpdate::EntryDetached {
                    request,
                    path,
                    error,
                } = detach_updates.recv().await.unwrap()
                    && request.id == id
                {
                    assert!(error.is_some());
                    assert!(path.is_none());
                    break;
                }
            }
        })
        .await
        .expect("failed detach request never acknowledged");
    }
    assert_eq!(
        std::fs::read_to_string(create_sandbox.path().join("moved folder/renamed.txt")).unwrap(),
        "retain existing source"
    );
    let external_import = tempfile::tempdir().unwrap();
    std::fs::write(
        external_import.path().join("imported.txt"),
        "external source preserved",
    )
    .unwrap();
    let mut import_updates = engine.subscribe();
    for (id, scope, success) in [
        (8600, "/tmp/other-workspace", false),
        (8601, "/tmp/vela-test", true),
    ] {
        engine.import_external(vela_engine::imports::Request {
            id,
            session: "ses_test".into(),
            workspace: scope.into(),
            parent: parent.clone(),
            sources: vec![external_import.path().join("imported.txt")],
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let EngineUpdate::EntriesImported {
                    request,
                    outcomes,
                    error,
                } = import_updates.recv().await.unwrap()
                    && request.id == id
                {
                    assert_eq!(error.is_none(), success, "{id}: {error:?}");
                    if success {
                        assert_eq!(outcomes.len(), 1);
                        assert!(outcomes[0].imported);
                        assert_eq!(outcomes[0].files, [format!("{parent}/imported.txt")]);
                    } else {
                        assert!(outcomes.is_empty());
                    }
                    break;
                }
            }
        })
        .await
        .expect("import request did not acknowledge");
    }
    assert_eq!(
        std::fs::read_to_string(create_sandbox.path().join("imported.txt")).unwrap(),
        "external source preserved"
    );
    assert_eq!(
        std::fs::read_to_string(external_import.path().join("imported.txt")).unwrap(),
        "external source preserved"
    );

    engine.write_file(3, "ses_test", "README.md", "# updated\n", "# test\n", false);
    wait_for(Duration::from_secs(5), || {
        std::fs::read_to_string("/tmp/vela-test/README.md")
            .is_ok_and(|content| content == "# updated\n")
    })
    .await;

    // Queue chips: list, cancel, and steer all round-trip through the engine.
    engine.refresh_inbox("ses_test");
    wait_for(Duration::from_secs(5), || {
        let mut seen = false;
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::Inbox { items, .. } = update
                && items.iter().any(|item| item.id == "inbox-1")
            {
                seen = true;
            }
        }
        seen
    })
    .await;
    engine.steer_inbox("ses_test", "inbox-1");
    wait_for(Duration::from_secs(5), || {
        state
            .inbox_actions
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, value)| kind == "steer" && value["delivery"] == "steer")
    })
    .await;
    engine.cancel_inbox("ses_test", "inbox-1");
    wait_for(Duration::from_secs(5), || {
        state
            .inbox_actions
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, _)| kind == "cancel")
    })
    .await;

    // Catalog and selection commands use the nested `location[directory]`
    // query shape and report the chosen model/agent back to the UI.
    engine.load_catalog("ses_test");
    let mut catalog_seen = false;
    let mut model = None;
    for _ in 0..50 {
        if let Ok(EngineUpdate::Catalog {
            models,
            agents,
            default_model,
            ..
        }) = updates.try_recv()
            && !models.is_empty()
            && !agents.is_empty()
        {
            let selected =
                default_model.expect("session selection should supersede the runtime default");
            assert_eq!(selected.id, "mock-model");
            assert_eq!(selected.name, "Mock Model");
            assert_eq!(selected.variant.as_deref(), Some("high"));
            assert_eq!(selected.context_limit, Some(100000));
            assert_eq!(
                models.len(),
                1,
                "disabled and incomplete catalog rows are excluded"
            );
            assert_eq!(selected.variants, ["low", "high"]);
            model = Some(selected);
            catalog_seen = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(catalog_seen, "catalog should include a model and agent");
    engine.load_providers(81, "ses_test");
    let mut provider_catalog_seen = false;
    for _ in 0..50 {
        if let Ok(EngineUpdate::Providers {
            request_id,
            session_id,
            providers,
            error,
        }) = updates.try_recv()
        {
            assert_eq!(request_id, 81);
            assert_eq!(session_id, "ses_test");
            assert!(error.is_none(), "{error:?}");
            assert_eq!(providers.len(), 2);
            assert_eq!(providers[0].id, "opencode-go");
            assert!(!providers[0].connected());
            assert!(providers[1].connected());
            assert!(!format!("{providers:?}").contains("not-public"));
            provider_catalog_seen = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        provider_catalog_seen,
        "provider data must arrive with its request and session identity"
    );
    for (request_id, directory, key, expected_calls, succeeds) in [
        (82, "/wrong-workspace", "must-not-send", 0, false),
        (83, "/tmp/vela-test", "fixture-test-key", 1, true),
        (84, "/tmp/vela-test", "reject-secret-fixture", 2, false),
    ] {
        engine.connect_provider_key(vela_engine::ProviderKeyConnection {
            request_id,
            session_id: "ses_test".into(),
            directory: directory.into(),
            integration_id: "openai".into(),
            key: key.into(),
            label: "Fixture".into(),
            answers: json!({"region":"eu"}).as_object().unwrap().clone(),
        });
        let mut completion = None;
        wait_for(Duration::from_secs(5), || {
            while let Ok(update) = updates.try_recv() {
                if let EngineUpdate::ProviderConnected {
                    request_id: returned,
                    session_id,
                    integration_id,
                    error,
                } = update
                    && returned == request_id
                {
                    assert_eq!(session_id, "ses_test");
                    assert_eq!(integration_id, "openai");
                    completion = Some(error);
                }
            }
            completion.is_some()
        })
        .await;
        let error = completion.unwrap();
        assert_eq!(error.is_none(), succeeds);
        if request_id == 84 {
            assert!(
                error
                    .as_deref()
                    .is_some_and(|error| error.contains("HTTP 400") && !error.contains(key)),
                "credential error body must be redacted"
            );
        }
        assert_eq!(state.provider_keys.lock().unwrap().len(), expected_calls);
    }
    let connected = state.provider_keys.lock().unwrap()[0].clone();
    assert_eq!(connected["key"], "fixture-test-key");
    assert_eq!(connected["label"], "Fixture");
    assert_eq!(connected["answer"]["region"], "eu");
    for (request_id, directory, credential, succeeds, expected_calls) in [
        (85, "/wrong-workspace", "credential-1", false, 0),
        (86, "/tmp/vela-test", "not-in-catalog", false, 0),
        (87, "/tmp/vela-test", "credential-1", true, 1),
    ] {
        engine.remove_provider_credential(vela_engine::ProviderCredentialRemoval {
            request_id,
            session_id: "ses_test".into(),
            directory: directory.into(),
            integration_id: "openai".into(),
            credential_id: credential.into(),
        });
        let mut completion = None;
        wait_for(Duration::from_secs(5), || {
            while let Ok(update) = updates.try_recv() {
                if let EngineUpdate::ProviderCredentialRemoved {
                    request_id: returned,
                    session_id,
                    integration_id,
                    credential_id,
                    error,
                } = update
                    && returned == request_id
                {
                    assert_eq!(session_id, "ses_test");
                    assert_eq!(integration_id, "openai");
                    assert_eq!(credential_id, credential);
                    completion = Some(error);
                }
            }
            completion.is_some()
        })
        .await;
        assert_eq!(completion.unwrap().is_none(), succeeds);
        assert_eq!(
            state.provider_removals.lock().unwrap().len(),
            expected_calls
        );
    }
    engine.start_provider_oauth(vela_engine::ProviderOAuthRequest {
        flow_id: 88,
        session_id: "ses_test".into(),
        directory: "/wrong-workspace".into(),
        integration_id: "openai".into(),
        method_id: "must-not-start".into(),
    });
    let mut rejected_oauth_scope = false;
    wait_for(Duration::from_secs(5), || {
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::ProviderOAuth {
                flow_id: 88,
                state: vela_engine::ProviderOAuthState::Failed(message),
                ..
            } = update
            {
                assert!(message.contains("workspace changed"));
                rejected_oauth_scope = true;
            }
        }
        rejected_oauth_scope
    })
    .await;
    let model = model.unwrap();
    engine.switch_model("ses_test", model.clone());
    wait_for(Duration::from_secs(5), || {
        let mut found = false;
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::Selection { model, .. } = update
                && model.is_some_and(|model| model.id == "mock-model")
            {
                found = true;
            }
        }
        found
    })
    .await;
    let mut automatic = model.clone();
    automatic.variant = None;
    engine.switch_model("ses_test", automatic);
    wait_for(Duration::from_secs(5), || {
        engine
            .view("ses_test")
            .and_then(|view| view.selection().0)
            .is_some_and(|selected| {
                selected.variant.is_none() && selected.variants == ["low", "high"]
            })
    })
    .await;
    assert!(
        state
            .switches
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, body)| kind == "model" && body.pointer("/model/variant").is_none())
    );
    engine.switch_model("ses_test", model);
    wait_for(Duration::from_secs(5), || {
        engine
            .view("ses_test")
            .and_then(|view| view.selection().0)
            .is_some_and(|selected| selected.variant.as_deref() == Some("high"))
    })
    .await;
    engine.switch_agent("ses_test", "build");
    wait_for(Duration::from_secs(5), || {
        let mut found = false;
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::Selection { agent, .. } = update
                && agent.as_deref() == Some("build")
            {
                found = true;
            }
        }
        found
    })
    .await;
    assert!(
        state
            .switches
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, body)| kind == "model"
                && body.pointer("/model/variant").and_then(Value::as_str) == Some("high"))
    );
    assert!(
        state
            .switches
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, _)| kind == "agent")
    );

    let mut rejected = engine.view("ses_test").unwrap().selection().0.unwrap();
    rejected.id = "reject-selection".into();
    engine.switch_model("ses_test", rejected);
    let mut rejection_seen = false;
    wait_for(Duration::from_secs(5), || {
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::Error(message) = update {
                rejection_seen |= message.contains("Deliberate model rejection");
            }
        }
        rejection_seen
    })
    .await;
    let retained = engine.view("ses_test").unwrap().selection().0.unwrap();
    assert_eq!(retained.id, "mock-model");
    assert_eq!(retained.variant.as_deref(), Some("high"));

    // Native Sass watcher: saved SCSS compiles with pinned bundled Dart Sass.
    let scss = std::path::Path::new("/tmp/vela-test/sass_probe.scss");
    let css = std::path::Path::new("/tmp/vela-test/sass_probe.css");
    let _ = std::fs::remove_file(css);
    std::fs::write(scss, "$ink: #123456;\n.probe { color: $ink; }\n").unwrap();
    engine.set_sass_watch("ses_test", true);
    wait_for(Duration::from_secs(20), || {
        std::fs::read_to_string(css)
            .map(|content| content.contains("color: #123456"))
            .unwrap_or(false)
    })
    .await;
    assert!(engine.sass_watch_enabled("ses_test"));
    // Disabling or closing another context must leave the owner running.
    let mut sass_updates = engine.subscribe();
    engine.set_sass_watch("not-the-sass-owner", false);
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let EngineUpdate::Sass {
                session_id,
                state,
            } = sass_updates.recv().await.unwrap()
                && session_id == "not-the-sass-owner" && !state.enabled
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert!(engine.sass_watch_enabled("ses_test"));
    engine.close_session("not-the-sass-owner");
    engine.set_sass_watch("ses_test", false);
    wait_for(Duration::from_secs(3), || {
        !engine.sass_watch_enabled("ses_test")
    })
    .await;
    let _ = std::fs::remove_file(scss);
    let _ = std::fs::remove_file(css);

    // Real compiler workers share a canonical root and keep other roots alive.
    let sass_roots = tempfile::tempdir().unwrap();
    let first_root = sass_roots.path().join("one");
    let second_root = sass_roots.path().join("two");
    for root in [&first_root, &second_root] {
        std::fs::create_dir(root).unwrap();
        std::fs::write(root.join("entry.scss"), ".first { color: #123456; }").unwrap();
    }
    let nested = first_root.join("nested");
    std::fs::create_dir(&nested).unwrap();
    std::fs::write(nested.join("_ink.scss"), "$ink: #112233;").unwrap();
    std::fs::write(nested.join("main.scss"), "@import 'ink'; .nested { color: $ink; }").unwrap();
    let first_owner = open_test_workspace(&engine, &first_root).await;
    let shared_owner = open_test_workspace(&engine, &first_root).await;
    let other_owner = open_test_workspace(&engine, &second_root).await;
    let started = engine.update_sass_watch(first_owner.clone(), true).await.unwrap();
    assert!(started.enabled);
    assert_eq!(started.status, vela_engine::SassStatus::Starting);
    assert_eq!(started.directory, first_root.canonicalize().unwrap().to_string_lossy());
    assert!(engine.update_sass_watch("missing-sass-context".into(), true).await.is_err());
    engine.set_sass_watch(other_owner.clone(), true);
    wait_for(Duration::from_secs(20), || {
        first_root.join("entry.css").is_file() && second_root.join("entry.css").is_file()
            && std::fs::read_to_string(nested.join("main.css")).is_ok_and(|css| css.contains("#112233"))
    }).await;
    std::fs::write(nested.join("_ink.scss"), "$ink: #445566;").unwrap();
    wait_for(Duration::from_secs(20), || std::fs::read_to_string(nested.join("main.css")).is_ok_and(|css| css.contains("#445566"))).await;
    assert!(!nested.join("_ink.css").exists());
    std::fs::remove_file(nested.join("_ink.scss")).unwrap();
    wait_for(Duration::from_secs(20), || engine.sass_watch_lines(&shared_owner).iter().any(|line| line.level == vela_engine::SassLogLevel::Error && line.message.contains("nested/main.scss"))).await;
    assert_eq!(engine.sass_watch_state(&shared_owner).status, vela_engine::SassStatus::Error);
    assert!(engine.sass_watch_state(&shared_owner).error.is_some());
    assert!(std::fs::read_to_string(nested.join("main.css")).unwrap().contains("#445566"));
    std::fs::write(nested.join("_ink.scss"), "$ink: #778899;").unwrap();
    wait_for(Duration::from_secs(20), || std::fs::read_to_string(nested.join("main.css")).is_ok_and(|css| css.contains("#778899"))).await;
    wait_for(Duration::from_secs(3), || engine.sass_watch_state(&shared_owner).status == vela_engine::SassStatus::On).await;
    assert!(engine.sass_watch_state(&shared_owner).error.is_none());
    // Config-only changes redirect real compiler output and create relative
    // source maps; removing the config restores sibling output without a save.
    let sibling_modified = std::fs::metadata(first_root.join("entry.css")).unwrap().modified().unwrap();
    std::fs::write(first_root.join("vela-sass.json"), "{entries:{'entry.scss':'compiled/entry.min.css'},style:'compressed',sourceMap:true}").unwrap();
    wait_for(Duration::from_secs(20), || first_root.join("compiled/entry.min.css.map").is_file()
        && std::fs::read_to_string(first_root.join("compiled/entry.min.css")).is_ok_and(|css| css.starts_with(".first{color:#123456}") && css.contains("sourceMappingURL=entry.min.css.map"))).await;
    let map: Value = serde_json::from_str(&std::fs::read_to_string(first_root.join("compiled/entry.min.css.map")).unwrap()).unwrap();
    assert!(map["sources"].as_array().unwrap().iter().any(|source| source == "../entry.scss"));
    std::fs::remove_file(first_root.join("vela-sass.json")).unwrap();
    wait_for(Duration::from_secs(20), || std::fs::metadata(first_root.join("entry.css")).unwrap().modified().unwrap() != sibling_modified).await;
    assert!(engine.sass_watch_enabled(&shared_owner));
    engine.close_session(first_owner.clone());
    wait_for(Duration::from_secs(3), || engine.view(&first_owner).is_none()).await;
    assert!(engine.sass_watch_enabled(&shared_owner));
    assert!(engine.sass_watch_enabled(&other_owner));
    for root in [&first_root, &second_root] {
        std::fs::write(root.join("entry.scss"), "@warn 'migration warning';\n.second { color: #654321; }").unwrap();
    }
    wait_for(Duration::from_secs(20), || {
        [&first_root, &second_root].iter().all(|root| {
            std::fs::read_to_string(root.join("entry.css")).is_ok_and(|css| css.contains("#654321"))
        })
    }).await;
    assert!(engine.sass_watch_lines(&shared_owner).iter().any(|line| line.message.contains("entry.scss → entry.css")));
    wait_for(Duration::from_secs(3), || engine.sass_watch_lines(&shared_owner).iter().any(|line| {
        line.level == vela_engine::SassLogLevel::Warning && line.message.contains("migration warning") && line.time_ms > 0
    })).await;
    assert_eq!(engine.sass_watch_state(&shared_owner).status, vela_engine::SassStatus::On);

    let stopped = engine.update_sass_watch(shared_owner.clone(), false).await.unwrap();
    assert!(!stopped.enabled);
    assert_eq!(stopped.status, vela_engine::SassStatus::Off);
    wait_for(Duration::from_secs(3), || !engine.sass_watch_enabled(&shared_owner)).await;
    assert!(engine.sass_watch_enabled(&other_owner));
    engine.close_session(shared_owner.clone());
    engine.close_session(other_owner.clone());
    wait_for(Duration::from_secs(3), || !engine.sass_watch_enabled(&other_owner)).await;
    // Explicit close unregisters resources, while the enabled folder preference
    // survives and starts a fresh worker when its context is reopened.
    engine.open_session(other_owner.clone());
    wait_for(Duration::from_secs(3), || engine.sass_watch_enabled(&other_owner)).await;
    engine.close_session(other_owner.clone());
    wait_for(Duration::from_secs(3), || !engine.sass_watch_enabled(&other_owner)).await;
    engine.open_session("ses_test");
    wait_for(Duration::from_secs(3), || engine.active_session().as_deref() == Some("ses_test")).await;

    // A selected file prepares its canonical parent without stealing active
    // context. Reject bad targets before creating any runtime conversation.
    let file_root = tempfile::tempdir().unwrap();
    let selected = file_root.path().join("界 🌍.txt");
    std::fs::write(&selected, "selected bytes stay on disk\n").unwrap();
    let before_file_open = state.created_workspaces.lock().unwrap().len();
    for invalid in ["relative.txt".to_owned(), file_root.path().to_string_lossy().into_owned(), file_root.path().join("missing.txt").to_string_lossy().into_owned()] {
        assert!(engine.open_file_workspace(invalid).await.is_err());
    }
    assert_eq!(state.created_workspaces.lock().unwrap().len(), before_file_open);
    let opened = engine.open_file_workspace(selected.to_string_lossy().into_owned()).await.unwrap();
    assert_eq!(opened.path, "界 🌍.txt");
    assert_eq!(opened.directory, file_root.path().canonicalize().unwrap().to_string_lossy());
    assert_eq!(state.created_workspaces.lock().unwrap().len(), before_file_open + 1);
    assert_eq!(engine.active_session().as_deref(), Some("ses_test"));
    assert!(engine.view(&opened.session_id).is_none());
    // Editor access must not wait behind a slow conversation replay.
    state.block_next_history.store(true, Ordering::SeqCst);
    let file_history_requests = state.message_requests.load(Ordering::SeqCst);
    let mut file_updates = engine.subscribe();
    engine.open_session(opened.session_id.clone());
    wait_for(Duration::from_secs(3), || state.message_requests.load(Ordering::SeqCst) > file_history_requests).await;
    engine.read_file(12001, opened.session_id.clone(), opened.path.clone());
    let (content, error) = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match file_updates.recv().await.unwrap() {
                EngineUpdate::FileContent { request_id: 12001, content, error, .. } => break (content, error),
                EngineUpdate::Error(error) => panic!("selected-file read failed before history replay: {error}"),
                _ => {}
            }
        }
    }).await.unwrap();
    // Missing files are deleted tabs, not an error or a permanently loading editor.
    engine.read_file(12002, opened.session_id.clone(), "removed.txt");
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let EngineUpdate::FileContent { request_id: 12002, deleted, content, error, .. } = file_updates.recv().await.unwrap() {
                assert!(deleted); assert!(content.is_none()); assert!(error.is_none()); break;
            }
        }
    }).await.unwrap();
    state.history_release.notify_one();
    assert!(error.is_none(), "file access waited for history ownership: {error:?}");
    assert_eq!(content.as_deref(), Some("selected bytes stay on disk\n"));
    engine.close_session(opened.session_id.clone());
    wait_for(Duration::from_secs(3), || engine.view(&opened.session_id).is_none()).await;
    engine.open_session("ses_test");
    wait_for(Duration::from_secs(3), || engine.active_session().as_deref() == Some("ses_test")).await;

    // Native PTY wiring: output flows through the engine and input reaches the
    // shell. `6*7` in the command means a literal "42" proves execution.
    engine.start_terminal(
        "term_test",
        temp.path().to_string_lossy().to_string(),
        80,
        24,
    );
    let mut terminal_bytes = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while tokio::time::Instant::now() < deadline && terminal_bytes.is_empty() {
        if let Ok(EngineUpdate::TerminalData { id, data }) = updates.try_recv()
            && id == "term_test"
        {
            terminal_bytes.extend_from_slice(&data);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(
        !terminal_bytes.is_empty(),
        "terminal should produce a prompt"
    );
    let mut input_errors = engine.subscribe();
    engine.terminal_input(
        "term_test",
        vec![b'x'; vela_terminal::input::MAX_INPUT_BYTES + 1],
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let EngineUpdate::Error(error) = input_errors.recv().await.unwrap()
                && error == "invalid or oversized terminal input"
            {
                break;
            }
        }
    })
    .await
    .expect("oversized terminal input was not rejected");
    assert!(engine.has_terminal("term_test"));
    engine.terminal_input("term_test", b"echo $((6*7))\n".to_vec());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut matched = false;
    while tokio::time::Instant::now() < deadline && !matched {
        if let Ok(EngineUpdate::TerminalData { id, data }) = updates.try_recv()
            && id == "term_test"
        {
            terminal_bytes.extend_from_slice(&data);
            matched = terminal_bytes.windows(2).any(|window| window == b"42");
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let mut duplicate_updates = engine.subscribe();
    engine.start_terminal(
        "term_test",
        temp.path().to_string_lossy().to_string(),
        80,
        24,
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let EngineUpdate::Error(error) = duplicate_updates.recv().await.unwrap()
                && error == "terminal already exists or is unavailable"
            {
                break;
            }
        }
    })
    .await
    .expect("duplicate terminal ID was not rejected");
    assert!(
        engine.has_terminal("term_test"),
        "duplicate start must retain the original PTY"
    );
    engine.stop_terminal("term_test");
    assert!(
        matched,
        "terminal command output should round-trip through the engine"
    );

    // The process can exit before its reader thread drains the final write.
    // Require the full burst and computed sentinel before the exit event.
    let mut reused_updates = engine.subscribe();
    engine.start_terminal(
        "term_test",
        temp.path().to_string_lossy().to_string(),
        80,
        24,
    );
    engine.terminal_input(
        "term_test",
        b"printf 'REUSED:%d\\n' $((8*9)); exit 29\n".to_vec(),
    );
    let (reused_bytes, reused_code) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut bytes = Vec::new();
        loop {
            match reused_updates.recv().await.unwrap() {
                EngineUpdate::TerminalData { id, data } if id == "term_test" => {
                    bytes.extend_from_slice(&data)
                }
                EngineUpdate::TerminalExit { id, code } if id == "term_test" => {
                    break (bytes, code);
                }
                _ => {}
            }
        }
    })
    .await
    .expect("replacement terminal was lost to retired cleanup");
    assert_eq!(reused_code, Some(29));
    assert!(reused_bytes.windows(9).any(|window| window == b"REUSED:72"));
    let mut final_updates = engine.subscribe();
    engine.start_terminal(
        "term_final_output",
        temp.path().to_string_lossy().to_string(),
        80,
        24,
    );
    engine.terminal_input("term_final_output",b"stty -echo; python3 -c 'import sys;sys.stdout.write(\"\".join(chr(65+i%26) for i in range(180000))+\"FINAL:%d\"%(6*7))'; exit 23\n".to_vec());
    let (final_bytes, final_code) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut bytes = Vec::new();
        loop {
            match final_updates.recv().await.unwrap() {
                EngineUpdate::TerminalData { id, data } if id == "term_final_output" => {
                    bytes.extend_from_slice(&data)
                }
                EngineUpdate::TerminalExit { id, code } if id == "term_final_output" => {
                    break (bytes, code);
                }
                _ => {}
            }
        }
    })
    .await
    .expect("final-output terminal never announced exit");
    assert_eq!(final_code, Some(23));
    let expected: Vec<u8> = (0..180000).map(|i| b'A' + (i % 26) as u8).collect();
    assert!(
        final_bytes
            .windows(expected.len())
            .any(|window| window == expected),
        "exit overtook the complete terminal output burst"
    );
    assert!(final_bytes.windows(8).any(|window| window == b"FINAL:42"));
    assert!(!engine.has_terminal("term_final_output"));

    // Resizes must be serialized by the engine rather than racing background
    // writes or touching the UI's settings snapshot. The final request wins.
    let final_size = vela_engine::window_state::WindowSize::new(921., 641.).unwrap();
    for width in 900..921 {
        engine.save_window_size(
            vela_engine::window_state::WindowSize::new(f64::from(width), 640.).unwrap(),
        );
    }
    engine.save_window_size(final_size);
    wait_for(Duration::from_secs(5), || {
        vela_engine::window_state::WindowSize::load(temp.path()) == Some(final_size)
    })
    .await;

    // A runtime rebind keeps the same retained view, draft, scroll and model,
    // rematerializes messages, and refuses an overlapping service operation.
    engine.set_draft("ses_test", "unfinished draft 🌍");
    engine.set_scroll("ses_test", 123., false);
    let retained = engine.view("ses_test").unwrap();
    let previous_selection = retained.selection();
    let requests_before = state.message_requests.load(Ordering::SeqCst);
    engine.manage_runtime(901, vela_engine::runtime::RuntimeAction::Sync);
    engine.manage_runtime(902, vela_engine::runtime::RuntimeAction::Update);
    let mut runtime_results = std::collections::HashMap::new();
    wait_for(Duration::from_secs(5), || {
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::RuntimeFinished { request_id, result } = update {
                runtime_results.insert(request_id, result);
            }
        }
        runtime_results.len() == 2
    })
    .await;
    assert_eq!(
        runtime_results.remove(&901).unwrap().unwrap().version,
        "2.1.4"
    );
    assert!(
        runtime_results
            .remove(&902)
            .unwrap()
            .unwrap_err()
            .contains("already in progress")
    );
    wait_for(Duration::from_secs(5), || {
        state.message_requests.load(Ordering::SeqCst) > requests_before
    })
    .await;
    assert!(Arc::ptr_eq(&retained, &engine.view("ses_test").unwrap()));
    assert_eq!(retained.draft(), "unfinished draft 🌍");
    assert_eq!(retained.scroll(), (123., false));
    assert_eq!(
        retained.selection().0.map(|model| model.id),
        previous_selection.0.map(|model| model.id)
    );

    // Editor authority follows its workspace, independently of the transcript
    // warm cache. A background save must keep the captured root and must not
    // silently reopen the session or authorize an unrelated workspace.
    use vela_core::document::{DocumentKey, Documents};
    let root = std::path::Path::new("/tmp/vela-test");
    std::fs::write(root.join("background-save.txt"), "background disk\n").unwrap();
    let key = DocumentKey::in_workspace("retained-editor-context".into(), "/tmp/vela-test".into(), "background-save.txt".into());
    let mut documents = Documents::default();
    documents.open(1, key.clone(), true);
    documents.finish_load(1, &key.rel, Some("background disk\n".into()));
    documents.edit(&key, |buffer| buffer.insert("edited "));
    let request = documents.save(951, "ses_test".into(), &key).unwrap();
    engine.retain_editor_workspace("ses_test".into(), key.directory.clone());
    engine.close_session("ses_test");
    wait_for(Duration::from_secs(3), || engine.view("ses_test").is_none()).await;
    engine.write_document(request);
    let mut saved = None;
    wait_for(Duration::from_secs(5), || {
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::FileSaved {
                request_id: 951,
                error,
                ..
            } = update
            {
                saved = Some(error);
            }
        }
        saved.is_some()
    })
    .await;
    assert!(saved.unwrap().is_none());
    assert_eq!(
        std::fs::read_to_string(root.join(&key.rel)).unwrap(),
        "edited background disk\n"
    );
    assert!(engine.view("ses_test").is_none());
    documents.finish_save(951, &key.rel, true);
    std::fs::write(root.join(&key.rel), "external while warm\n").unwrap();
    engine.read_document(952, key.clone());
    let mut read = None;
    wait_for(Duration::from_secs(5), || {
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::FileContent {
                request_id: 952,
                content,
                error,
                ..
            } = update
            {
                assert!(error.is_none());
                read = content;
            }
        }
        read.is_some()
    })
    .await;
    assert_eq!(read.as_deref(), Some("external while warm\n"));
    assert!(engine.view("ses_test").is_none());
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("file.txt"), "unrelated disk").unwrap();
    let other = DocumentKey::new(
        outside.path().to_string_lossy().into_owned(),
        "file.txt".into(),
    );
    documents.open(2, other.clone(), true);
    documents.finish_load(2, "file.txt", Some("unrelated disk".into()));
    documents.edit(&other, |buffer| buffer.insert("forged "));
    engine.write_document(documents.save(953, "ses_test".into(), &other).unwrap());
    let mut rejected = false;
    wait_for(Duration::from_secs(3), || {
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::FileSaved {
                request_id: 953,
                error,
                ..
            } = update
            {
                rejected = error.is_some();
            }
        }
        rejected
    })
    .await;
    assert_eq!(
        std::fs::read_to_string(outside.path().join("file.txt")).unwrap(),
        "unrelated disk"
    );
    engine.release_editor_workspace(key.directory.clone());
    documents.edit(&key, |buffer| buffer.insert("late "));
    engine.write_document(documents.save(954, "ses_test".into(), &key).unwrap());
    let mut released = false;
    wait_for(Duration::from_secs(3), || {
        while let Ok(update) = updates.try_recv() {
            if let EngineUpdate::FileSaved {
                request_id: 954,
                error,
                ..
            } = update
            {
                released = error.is_some();
            }
        }
        released
    })
    .await;
    assert_eq!(
        std::fs::read_to_string(root.join(&key.rel)).unwrap(),
        "external while warm\n"
    );
    // Opening another folder must publish an explicit navigation completion.
    // A normal Session update also describes background stream changes, so the
    // UI cannot use it to decide which workspace the user just requested.
    let mut navigation = engine.subscribe();
    for name in ["first-workspace", "second-workspace"] {
        let directory = temp.path().join(name);
        std::fs::create_dir(&directory).unwrap();
        let directory = directory.to_string_lossy().to_string();
        engine.open_directory(directory.clone());
        let id = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let EngineUpdate::WorkspaceOpened {
                    directory: opened,
                    result,
                } = navigation.recv().await.unwrap()
                    && opened == directory
                {
                    break result.unwrap();
                }
            }
        })
        .await
        .expect("workspace navigation never completed");
        assert_eq!(engine.active_session().as_deref(), Some(id.as_str()));
        assert_eq!(engine.view(&id).unwrap().summary().directory, directory);
    }
    let deleted_active = engine.active_session().unwrap();
    let mut delete_updates = engine.subscribe();
    engine.delete_session(9000, "ses_test");
    let failed = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let EngineUpdate::SessionDeleteFinished {
                request_id: 9000,
                error,
                ..
            } = delete_updates.recv().await.unwrap()
            {
                break error;
            }
        }
    })
    .await
    .unwrap();
    assert!(failed.is_some());
    assert!(
        engine
            .sessions()
            .iter()
            .any(|summary| summary.id == "ses_test")
    );
    assert_eq!(
        engine.active_session().as_deref(),
        Some(deleted_active.as_str())
    );

    // Hold an actual replay response in flight while deletion is acknowledged.
    engine.open_session("ses_test");
    wait_for(Duration::from_secs(3), || engine.view("ses_test").is_some()).await;
    engine.set_sass_watch("ses_test", true);
    wait_for(Duration::from_secs(3), || engine.sass_watch_enabled("ses_test")).await;
    engine.close_session("ses_test");
    wait_for(Duration::from_secs(3), || engine.view("ses_test").is_none()).await;
    assert!(!engine.sass_watch_enabled("ses_test"));
    // A history response already in flight must not reopen a closed context
    // or restart its persisted Sass watcher after unregistering it.
    state.block_next_history.store(true, Ordering::SeqCst);
    let closing_requests = state.message_requests.load(Ordering::SeqCst);
    engine.open_session("ses_test");
    wait_for(Duration::from_secs(3), || state.message_requests.load(Ordering::SeqCst) > closing_requests).await;
    engine.close_session("ses_test");
    wait_for(Duration::from_secs(3), || engine.view("ses_test").is_none()).await;
    state.history_release.notify_one();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(engine.view("ses_test").is_none(), "late history reopened a closed context");
    assert!(!engine.sass_watch_enabled("ses_test"), "late history restarted a closed Sass owner");
    state.block_next_history.store(true, Ordering::SeqCst);
    let requests = state.message_requests.load(Ordering::SeqCst);
    engine.open_session("ses_test");
    wait_for(Duration::from_secs(3), || {
        state.message_requests.load(Ordering::SeqCst) > requests
    })
    .await;
    engine.delete_session(9001, "ses_test");
    let succeeded = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let EngineUpdate::SessionDeleteFinished {
                request_id: 9001,
                error,
                ..
            } = delete_updates.recv().await.unwrap()
            {
                break error;
            }
        }
    })
    .await
    .unwrap();
    assert!(succeeded.is_none());
    assert!(engine.view("ses_test").is_none());
    assert_eq!(
        *state.session_deletes.lock().unwrap(),
        ["ses_test", "ses_test"]
    );
    state.history_release.notify_one();
    engine.open_session(deleted_active.clone());
    wait_for(Duration::from_secs(3), || {
        engine.active_session().as_deref() == Some(deleted_active.as_str())
    })
    .await;
    // Runtime deletion must clear active/warm ownership. Late history and
    // stale inventory must never resurrect a session that was deleted.
    state
        .events
        .send(json!({"type":"session.deleted", "data":{"sessionID":deleted_active}}))
        .unwrap();
    wait_for(Duration::from_secs(3), || {
        engine.active_session().is_none()
            && engine.view(&deleted_active).is_none()
            && !engine
                .sessions()
                .iter()
                .any(|summary| summary.id == deleted_active || summary.id == "ses_test")
    })
    .await;
    let prompts_before_retired_action = state.prompts.lock().unwrap().len();
    engine.prompt("ses_test", "stale UI action must never reach runtime");
    engine.queue_prompt(deleted_active.clone(), "stale queued action");
    engine.open_session(deleted_active.clone());
    engine.open_session("ses_test");
    engine.refresh_sessions();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        state.prompts.lock().unwrap().len(),
        prompts_before_retired_action
    );
    assert!(engine.active_session().is_none());
    assert!(engine.view(&deleted_active).is_none());
    assert!(engine.view("ses_test").is_none());
    assert!(
        !engine
            .sessions()
            .iter()
            .any(|summary| summary.id == "ses_test")
    );

    tokio::time::timeout(Duration::from_secs(3), engine.shutdown_and_wait())
        .await
        .expect("engine did not finish application shutdown");
}

async fn open_test_workspace(engine: &Engine, root: &std::path::Path) -> String {
    let directory = root.to_string_lossy().into_owned();
    let mut updates = engine.subscribe();
    engine.open_directory(directory.clone());
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let EngineUpdate::WorkspaceOpened { directory: opened, result } = updates.recv().await.unwrap()
                && opened == directory { break result.unwrap(); }
        }
    }).await.expect("fixture workspace did not open")
}

async fn wait_for(timeout: Duration, mut condition: impl FnMut() -> bool) {
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() < deadline {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("condition not met within {timeout:?}");
}

async fn info() -> Json<Value> {
    Json(json!({ "version": "2.1.4", "pid": 4242 }))
}

async fn sessions() -> Json<Value> {
    Json(json!({ "data": [session_json()], "cursor": {} }))
}

async fn session(Path(id): Path<String>, State(state): State<TestState>) -> Json<Value> {
    let mut summary = session_json();
    if let Some(index) = id
        .strip_prefix("ses_workspace_")
        .and_then(|n| n.parse::<usize>().ok())
    {
        summary["id"] = json!(id);
        summary["location"]["directory"] = json!(state.created_workspaces.lock().unwrap()[index]);
    }
    Json(json!({ "data": summary }))
}

async fn remove_session(
    Path(id): Path<String>,
    State(state): State<TestState>,
) -> impl IntoResponse {
    state.session_deletes.lock().unwrap().push(id);
    if state
        .deletes_to_fail
        .try_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
            count.checked_sub(1)
        })
        .is_ok()
    {
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"message":"Deliberate delete rejection"})),
        )
    } else {
        (axum::http::StatusCode::NO_CONTENT, Json(Value::Null))
    }
}

async fn create_workspace(State(state): State<TestState>, Json(body): Json<Value>) -> Json<Value> {
    let mut workspaces = state.created_workspaces.lock().unwrap();
    let id = format!("ses_workspace_{}", workspaces.len());
    let directory = body["location"]["directory"].as_str().unwrap().to_string();
    workspaces.push(directory.clone());
    let mut summary = session_json();
    summary["id"] = json!(id);
    summary["location"]["directory"] = json!(directory);
    Json(json!({ "data": summary }))
}

fn session_json() -> Value {
    json!({
        "id": "ses_test",
        "title": "Test session",
        "projectID": "proj",
        "model": { "modelID": "mock-model", "providerID": "mock", "variant": "high" },
        "cost": 0,
        "tokens": { "input": 0, "output": 0, "reasoning": 0, "cache": { "read": 0, "write": 0 } },
        "time": { "created": 1, "updated": 2 },
        "location": { "directory": "/tmp/vela-test" }
    })
}

async fn models() -> Json<Value> {
    Json(json!({ "data": [
            { "id": "mock-model", "providerID": "mock", "name": "Mock Model", "limit": { "context": 100000 }, "variants":[{"id":"low"}, "high"] },
            { "id": "disabled", "providerID": "mock", "enabled":false },
            { "id":"missing-provider" }
        ] }))
}

async fn default_model() -> Json<Value> {
    Json(
        json!({ "data": { "id": "default-model", "providerID": "mock", "name": "Default Model" } }),
    )
}

async fn integrations(
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    assert_eq!(
        query.get("location[directory]").map(String::as_str),
        Some("/tmp/vela-test")
    );
    Json(json!({"data": [
        {"id":"openai", "name":"OpenAI", "methods":[{"type":"key","form":[]}],
            "connections":[{"type":"credential","id":"credential-1","label":"Fixture", "value":"not-public"}]},
        {"id":"opencode-go", "name":"OpenCode Go", "methods":[], "connections":[]},
        {"id":"unlisted", "name":"Unsupported"}
    ]}))
}

async fn connect_provider_key(
    Path(id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
    State(state): State<TestState>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    assert_eq!(id, "openai");
    assert_eq!(
        query.get("location[directory]").map(String::as_str),
        Some("/tmp/vela-test")
    );
    state.provider_keys.lock().unwrap().push(body.clone());
    if body["key"] == "reject-secret-fixture" {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"message":"echoed reject-secret-fixture must not reach the UI"})),
        )
            .into_response()
    } else {
        axum::http::StatusCode::NO_CONTENT.into_response()
    }
}

async fn remove_provider_credential(
    Path(id): Path<String>,
    State(state): State<TestState>,
) -> axum::http::StatusCode {
    state.provider_removals.lock().unwrap().push(id);
    axum::http::StatusCode::NO_CONTENT
}

async fn agents() -> Json<Value> {
    Json(json!({ "data": [ { "id": "build", "name": "Build" } ] }))
}

async fn switch_model(
    State(state): State<TestState>,
    Path(_id): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let rejected = body.pointer("/model/id").and_then(Value::as_str) == Some("reject-selection");
    state.switches.lock().unwrap().push(("model".into(), body));
    if rejected {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"message":"Deliberate model rejection"})),
        )
    } else {
        (axum::http::StatusCode::OK, Json(json!({})))
    }
}

async fn switch_agent(
    State(state): State<TestState>,
    Path(_id): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    state.switches.lock().unwrap().push(("agent".into(), body));
    Json(json!({}))
}

async fn inbox() -> Json<Value> {
    Json(json!({
        "data": [{
            "id": "inbox-1",
            "sessionID": "ses_test",
            "time": { "created": 1 },
            "type": "user",
            "payload": { "text": "queued follow-up" },
            "delivery": "queue"
        }]
    }))
}

async fn cancel_inbox(
    State(state): State<TestState>,
    Path((_id, iid)): Path<(String, String)>,
) -> impl IntoResponse {
    state
        .inbox_actions
        .lock()
        .unwrap()
        .push(("cancel".into(), json!({ "id": iid })));
    Json(json!({}))
}

async fn steer_inbox(
    State(state): State<TestState>,
    Path((_id, iid)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    state
        .inbox_actions
        .lock()
        .unwrap()
        .push(("steer".into(), body));
    Json(json!({ "id": iid }))
}

async fn messages(State(state): State<TestState>) -> Json<Value> {
    let blocked = state.block_next_history.swap(false, Ordering::SeqCst);
    state.message_requests.fetch_add(1, Ordering::SeqCst);
    if blocked {
        state.history_release.notified().await;
    }
    Json(json!({
        "data": [
            { "id": "msg_u", "type": "user", "time": { "created": 1 }, "text": "hello from replay" },
            { "id": "msg_a", "type": "assistant", "agent": "build", "model": { "id": "demo" },
              "time": { "created": 2, "completed": 3 }, "finish": "stop",
              "content": [ { "type": "text", "text": "assistant replay" } ] }
        ],
        "cursor": {}
    }))
}

async fn prompt(
    State(state): State<TestState>,
    Path(_id): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    state.prompts.lock().unwrap().push(body);
    if state
        .prompts_to_fail
        .try_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
            count.checked_sub(1)
        })
        .is_ok()
    {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"message":"Disposable prompt failure"})),
        )
            .into_response();
    }
    Json(json!({ "id": "inbox" })).into_response()
}

async fn interrupt(Path(_id): Path<String>) -> impl IntoResponse {
    Json(json!({}))
}

async fn stage_revert(
    State(state): State<TestState>,
    Path(_id): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    state.revert_stages.lock().unwrap().push(body.clone());
    Json(json!({
        "data": {
            "messageID": body.get("messageID").cloned().unwrap_or(Value::Null),
            "files": []
        }
    }))
}

async fn commit_revert(State(state): State<TestState>, Path(_id): Path<String>) -> impl IntoResponse {
    state.revert_commits.fetch_add(1, Ordering::SeqCst);
    if state.revert_fail_commit.load(Ordering::SeqCst) {
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"message":"Disposable revert commit failure"})),
        )
            .into_response()
    } else {
        axum::http::StatusCode::NO_CONTENT.into_response()
    }
}

async fn clear_revert(State(state): State<TestState>, Path(_id): Path<String>) -> impl IntoResponse {
    state.revert_clears.fetch_add(1, Ordering::SeqCst);
    axum::http::StatusCode::NO_CONTENT
}

async fn list_forms(State(state): State<TestState>, Path(_id): Path<String>) -> Json<Value> {
    Json(json!({ "data": state.forms.lock().unwrap().clone() }))
}

async fn list_location_forms(State(state): State<TestState>) -> Json<Value> {
    Json(json!({ "data": state.location_forms.lock().unwrap().clone() }))
}

async fn reply_form(
    State(state): State<TestState>,
    Path((_id, form_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    state.form_replies.lock().unwrap().push(json!({
        "formID": form_id,
        "body": body,
        "directory": headers.get("x-opencode-directory").and_then(|value| value.to_str().ok())
    }));
    axum::http::StatusCode::NO_CONTENT
}

async fn cancel_form(
    State(state): State<TestState>,
    Path((_id, form_id)): Path<(String, String)>,
) -> impl IntoResponse {
    state.form_cancels.lock().unwrap().push(form_id);
    axum::http::StatusCode::NO_CONTENT
}

async fn compact_session(
    State(state): State<TestState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    state.compacts.fetch_add(1, Ordering::SeqCst);
    Json(json!({"data":{"compaction":{"status":"running"}}}))
}

async fn run_command(
    State(state): State<TestState>,
    Path(_id): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    state.commands.lock().unwrap().push(body);
    axum::http::StatusCode::NO_CONTENT
}

async fn list_skills(_: axum::extract::Query<std::collections::HashMap<String, String>>) -> Json<Value> {
    Json(json!({"data":[{"id":"skill_review","name":"review","description":"Review code"}]}))
}

async fn list_commands(
    _: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    Json(json!({"data":[{"name":"deploy","description":"Deploy the app"}]}))
}

async fn find_files(
    _: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    Json(json!({"data":[
        {"path":"src/main.rs","type":"file"},
        {"path":".vela-recovery/secret","type":"file"},
        {"path":"README.md","type":"file"}
    ]}))
}

async fn activate_skill(
    State(state): State<TestState>,
    Path(_id): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    state
        .skill_activations
        .lock()
        .unwrap()
        .push(body["id"].as_str().unwrap_or_default().to_owned());
    axum::http::StatusCode::NO_CONTENT
}

async fn permissions() -> Json<Value> {
    Json(json!([]))
}

async fn reply(
    State(state): State<TestState>,
    Path((_id, _rid)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    state.replies.lock().unwrap().push(body);
    Json(json!({}))
}

async fn event_stream(
    State(state): State<TestState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.events.subscribe();
    let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
        loop {
            match receiver.recv().await {
                Ok(value) => {
                    return Some((Ok(Event::default().data(value.to_string())), receiver));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => return None,
            }
        }
    });
    Sse::new(stream)
}
