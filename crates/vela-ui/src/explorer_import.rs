//! Explorer external drops retain their captured workspace through asynchronous I/O.
use super::*;
impl VelaApp {
    pub(super) fn start_explorer_import(
        &mut self,
        workspace: String,
        parent: String,
        paths: Vec<std::path::PathBuf>,
        cx: &mut Context<Self>,
    ) {
        if self.workspace_directory() != workspace || paths.is_empty() || self.closing.is_some() {
            return;
        }
        let Some(session) = self.active_id.clone() else {
            return;
        };
        self.file_request += 1;
        let request = vela_engine::imports::Request {
            id: self.file_request,
            session,
            workspace,
            parent,
            sources: paths,
        };
        self.import_requests.insert(request.id, request.clone());
        self.engine.import_external(request);
        cx.notify();
    }
    pub(super) fn finish_explorer_import(
        &mut self,
        request: vela_engine::imports::Request,
        outcomes: Vec<vela_engine::imports::Outcome>,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(owned) = self.import_requests.remove(&request.id) else {
            return;
        };
        if owned.workspace != self.workspace_directory() {
            return;
        }
        if let Some(error) = error {
            self.toasts
                .update(cx, |toasts, cx| toasts.push(error, true, cx));
        }
        for outcome in outcomes {
            if outcome.imported {
                self.toasts.update(cx, |toasts, cx| {
                    toasts.push(format!("Imported {}", outcome.rel), false, cx)
                });
            } else if !matches!(
                outcome.reason.as_deref(),
                Some("already exists" | "already in the workspace")
            ) {
                self.toasts.update(cx, |toasts, cx| {
                    toasts.push(
                        format!(
                            "Could not import {}: {}",
                            outcome.name,
                            outcome.reason.as_deref().unwrap_or("unknown")
                        ),
                        true,
                        cx,
                    )
                });
            }
        }
        if !owned.parent.is_empty() {
            self.expanded.insert(owned.parent.clone());
        }
        self.request_directory(owned.parent);
        self.refresh_files(cx);
        cx.notify();
    }
}
