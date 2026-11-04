//! Workspace-owned documents. Tabs and session views borrow these buffers; an
//! asynchronous response can only update the document that issued its request.
use crate::text::TextBuffer;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DocumentKey {
    /// Owning UI workspace/context identity, independent of its disk root.
    pub workspace: String,
    pub directory: String,
    pub rel: String,
}

impl DocumentKey {
    /// Directory-owned compatibility scope. Activation-owned editors use
    /// `in_workspace` so two contexts may retain independent buffers.
    pub fn new(directory: String, rel: String) -> Self {
        Self {
            workspace: directory.clone(),
            directory,
            rel,
        }
    }
    pub fn in_workspace(workspace: String, directory: String, rel: String) -> Self {
        Self {
            workspace,
            directory,
            rel,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Eol {
    #[default]
    Lf,
    Crlf,
}

impl Eol {
    fn detect(text: &str) -> Self {
        if text.contains("\r\n") {
            Self::Crlf
        } else {
            Self::Lf
        }
    }

    fn normalize(text: &str) -> String {
        text.replace("\r\n", "\n")
    }

    fn encode(self, text: &str) -> String {
        match self {
            Self::Lf => text.to_owned(),
            Self::Crlf => text.replace('\n', "\r\n"),
        }
    }
}

pub struct Document {
    pub buffer: TextBuffer,
    saved_text: String,
    disk_saved_text: String,
    eol: Eol,
    pub conflict: Option<ExternalChange>,
    pub deleted: bool,
    external_revision: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExternalChange {
    pub content: Option<String>,
    pub deleted: bool,
    pub read_error: Option<String>,
    pub merging: bool,
}

impl Document {
    fn new(text: String) -> Self {
        let eol = Eol::detect(&text);
        let normalized = Eol::normalize(&text);
        let mut buffer = TextBuffer::new(normalized.clone());
        buffer.move_to(0, false);
        Self {
            buffer,
            saved_text: normalized,
            disk_saved_text: text,
            eol,
            conflict: None,
            deleted: false,
            external_revision: 0,
        }
    }
    pub fn dirty(&self) -> bool {
        self.buffer.text() != self.saved_text
    }
    pub fn is_modified(&self) -> bool {
        self.dirty() || self.conflict.is_some()
    }
    pub fn line_col(&self) -> (usize, usize) {
        let before = &self.buffer.text()[..self.buffer.caret()];
        (
            before.matches('\n').count(),
            before.rsplit('\n').next().unwrap_or("").chars().count(),
        )
    }
}

#[derive(Clone, Debug)]
pub struct DocumentTab {
    pub key: DocumentKey,
    pub pinned: bool,
}

#[derive(Default)]
struct Workspace {
    tabs: Vec<DocumentTab>,
    active: Option<DocumentKey>,
}

#[derive(Clone, Debug)]
pub struct FileRequest {
    pub id: u64,
    pub session: String,
    pub key: DocumentKey,
    pub text: String,
    pub expected: String,
    pub overwrite: bool,
    normalized_text: String,
    external_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RelocationError {
    #[error("Wait for pending document reads and saves to finish")]
    Busy,
    #[error("The destination already has an open document")]
    DestinationOpen,
    #[error("Invalid document relocation")]
    Invalid,
}

fn in_prefix(key: &DocumentKey, prefix: &DocumentKey) -> bool {
    key.directory == prefix.directory
        && (key.rel == prefix.rel
            || key
                .rel
                .strip_prefix(&prefix.rel)
                .is_some_and(|tail| tail.starts_with('/')))
}
fn relocated_key(key: &DocumentKey, from: &DocumentKey, to: &DocumentKey) -> DocumentKey {
    DocumentKey::in_workspace(
        key.workspace.clone(),
        to.directory.clone(),
        format!("{}{}", to.rel, &key.rel[from.rel.len()..]),
    )
}

#[derive(Default)]
pub struct Documents {
    buffers: HashMap<DocumentKey, Document>,
    workspaces: HashMap<String, Workspace>,
    workspace_order: Vec<String>,
    loads: HashMap<u64, DocumentKey>,
    // Only one write per document is dispatched at a time. Merely rejecting a
    // stale acknowledgement would still allow older disk writes to win a race.
    saves: HashMap<DocumentKey, VecDeque<FileRequest>>,
    relocations: HashMap<u64, (DocumentKey, DocumentKey)>,
}

impl Documents {
    pub fn retains_directory(&self, directory: &str) -> bool {
        self.workspaces
            .values()
            .flat_map(|workspace| &workspace.tabs)
            .any(|tab| tab.key.directory == directory)
            || self.buffers.iter().any(|(key, document)| {
                key.directory == directory && (document.is_modified() || self.is_saving(key))
            })
    }
    pub fn keys_at_path(&self, directory: &str, rel: &str) -> Vec<DocumentKey> {
        let mut keys = self
            .retained_keys()
            .into_iter()
            .filter(|key| key.directory == directory && key.rel == rel)
            .collect::<Vec<_>>();
        keys.sort_by(|a, b| a.workspace.cmp(&b.workspace));
        keys
    }
    pub fn unloaded_keys_in_directory(&self, directory: &str) -> Vec<DocumentKey> {
        self.workspaces
            .values()
            .flat_map(|workspace| &workspace.tabs)
            .filter(|tab| tab.key.directory == directory && !self.buffers.contains_key(&tab.key))
            .map(|tab| tab.key.clone())
            .collect()
    }
    pub fn observe_path(
        &mut self,
        directory: &str,
        rel: &str,
        change: ExternalChange,
        write_id: Option<u64>,
    ) -> Vec<DocumentKey> {
        self.keys_at_path(directory, rel)
            .into_iter()
            .filter(|key| self.observe(key, change.clone(), write_id))
            .collect()
    }
    pub fn retains_workspace(&self, workspace: &str) -> bool {
        !self.tabs(workspace).is_empty()
            || self.buffers.iter().any(|(key, document)| {
                key.workspace == workspace && (document.is_modified() || self.is_saving(key))
            })
    }

    pub fn unsaved_keys(&self) -> Vec<DocumentKey> {
        let mut keys = self
            .workspace_order
            .iter()
            .flat_map(|workspace| self.tabs(workspace))
            .filter(|tab| self.get(&tab.key).is_some_and(|document| document.dirty()))
            .map(|tab| tab.key.clone())
            .collect::<Vec<_>>();
        let mut retained = self
            .buffers
            .iter()
            .filter(|(key, document)| document.dirty() && !keys.contains(key))
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        retained.sort_by(|a, b| {
            a.workspace
                .cmp(&b.workspace)
                .then_with(|| a.rel.cmp(&b.rel))
        });
        keys.extend(retained);
        keys
    }

    pub fn relocation_pending(&self) -> bool {
        !self.relocations.is_empty()
    }
    pub fn workspace_busy(&self, workspace: &str) -> bool {
        self.loads.values().any(|key| key.workspace == workspace)
            || self.saves.keys().any(|key| key.workspace == workspace)
            || self.relocations.values().any(|(from, to)| {
                from.workspace == workspace
                    || to.workspace == workspace
                    || self.retained_keys().iter().any(|key| {
                        key.workspace == workspace && (in_prefix(key, from) || in_prefix(key, to))
                    })
            })
    }
    pub fn workspace_modified_keys(&self, workspace: &str) -> Vec<DocumentKey> {
        let mut keys = self
            .buffers
            .iter()
            .filter(|(key, document)| key.workspace == workspace && document.is_modified())
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        keys.sort_by(|a, b| a.rel.cmp(&b.rel));
        keys
    }
    pub fn relocation_contains(&self, key: &DocumentKey) -> bool {
        self.relocations
            .values()
            .any(|(from, to)| in_prefix(key, from) || in_prefix(key, to))
    }
    pub fn pending_io_in_prefix(&self, prefix: &DocumentKey) -> bool {
        self.loads
            .values()
            .chain(self.saves.keys())
            .any(|key| in_prefix(key, prefix))
    }
    pub fn begin_relocation(
        &mut self,
        id: u64,
        from: DocumentKey,
        to: DocumentKey,
    ) -> Result<(), RelocationError> {
        if from.workspace != to.workspace
            || from.directory != to.directory
            || from.rel.is_empty()
            || to.rel.is_empty()
        {
            return Err(RelocationError::Invalid);
        }
        if from != to && (in_prefix(&from, &to) || in_prefix(&to, &from)) {
            return Err(RelocationError::Invalid);
        }
        if self.pending_io_in_prefix(&from)
            || self.pending_io_in_prefix(&to)
            || self.relocations.contains_key(&id)
            || self.relocations.values().any(|(a, b)| {
                [a, b].iter().any(|key| {
                    in_prefix(key, &from)
                        || in_prefix(&from, key)
                        || in_prefix(key, &to)
                        || in_prefix(&to, key)
                })
            })
        {
            return Err(RelocationError::Busy);
        }
        if self
            .retained_keys()
            .iter()
            .any(|key| in_prefix(key, &to) && !in_prefix(key, &from))
        {
            return Err(RelocationError::DestinationOpen);
        }
        self.validate_relocation(&from, &to)?;
        self.relocations.insert(id, (from, to));
        Ok(())
    }
    /// Reserve the source prefix against reads/saves before an explicit Trash
    /// action. The same-prefix reservation shares rename's overlap ownership.
    pub fn begin_removal(&mut self, id: u64, prefix: DocumentKey) -> Result<(), RelocationError> {
        self.begin_relocation(id, prefix.clone(), prefix)
    }
    /// Only a successful OS Trash acknowledgement discards the explicitly
    /// removed tabs and their edits. Failure retains every buffer and undo.
    pub fn finish_removal(
        &mut self,
        id: u64,
        success: bool,
    ) -> Result<Vec<DocumentKey>, RelocationError> {
        self.finish_removals(&[(id, success)])
    }
    /// Detaching retains editors and undo; the old workspace paths become
    /// deleted/conflicted only after the serialized disk move succeeds.
    pub fn finish_detachment(
        &mut self,
        id: u64,
        success: bool,
    ) -> Result<Vec<DocumentKey>, RelocationError> {
        let (from, to) = self
            .relocations
            .get(&id)
            .ok_or(RelocationError::Invalid)?
            .clone();
        if from != to {
            return Err(RelocationError::Invalid);
        }
        self.relocations.remove(&id);
        if !success {
            return Ok(Vec::new());
        }
        let keys = self
            .retained_keys()
            .into_iter()
            .filter(|key| in_prefix(key, &from))
            .collect::<Vec<_>>();
        for key in &keys {
            self.observe(
                key,
                ExternalChange {
                    deleted: true,
                    ..Default::default()
                },
                None,
            );
        }
        Ok(keys)
    }
    /// Apply a multiselection's acknowledgements together, selecting the same
    /// surviving neighbor as Source from the original tab order.
    pub fn finish_removals(
        &mut self,
        results: &[(u64, bool)],
    ) -> Result<Vec<DocumentKey>, RelocationError> {
        let mut removed = Vec::new();
        for (id, success) in results {
            let (from, to) = self.relocations.get(id).ok_or(RelocationError::Invalid)?;
            if from != to {
                return Err(RelocationError::Invalid);
            }
            if *success {
                removed.push(from.clone());
            }
        }
        for (id, _) in results {
            self.relocations.remove(id);
        }
        let within = |key: &DocumentKey| removed.iter().any(|prefix| in_prefix(key, prefix));
        let keys = self
            .retained_keys()
            .into_iter()
            .filter(|key| within(key))
            .collect::<Vec<_>>();
        for workspace in self.workspaces.values_mut() {
            let active_index = workspace
                .active
                .as_ref()
                .filter(|key| within(key))
                .and_then(|key| workspace.tabs.iter().position(|tab| tab.key == *key));
            workspace.tabs.retain(|tab| !within(&tab.key));
            if let Some(index) = active_index {
                workspace.active = workspace
                    .tabs
                    .get(index)
                    .or_else(|| workspace.tabs.last())
                    .map(|tab| tab.key.clone());
            }
        }
        for key in &keys {
            self.buffers.remove(key);
        }
        self.loads.retain(|_, key| !within(key));
        Ok(keys)
    }

    fn retained_keys(&self) -> HashSet<DocumentKey> {
        self.buffers
            .keys()
            .chain(
                self.workspaces
                    .values()
                    .flat_map(|workspace| workspace.tabs.iter().map(|tab| &tab.key)),
            )
            .cloned()
            .collect()
    }
    fn validate_relocation(
        &self,
        from: &DocumentKey,
        to: &DocumentKey,
    ) -> Result<(), RelocationError> {
        let keys = self.retained_keys();
        if keys.iter().filter(|key| in_prefix(key, from)).any(|key| {
            let target = relocated_key(key, from, to);
            target != *key && keys.contains(&target) && !in_prefix(&target, from)
        }) {
            return Err(RelocationError::DestinationOpen);
        }
        Ok(())
    }
    pub fn finish_relocation(
        &mut self,
        id: u64,
        success: bool,
    ) -> Result<Vec<(DocumentKey, DocumentKey)>, RelocationError> {
        let (from, to) = self
            .relocations
            .remove(&id)
            .ok_or(RelocationError::Invalid)?;
        if !success || from == to {
            return Ok(Vec::new());
        }
        self.validate_relocation(&from, &to)?;
        let moves = self
            .retained_keys()
            .into_iter()
            .filter(|key| in_prefix(key, &from))
            .map(|key| {
                let target = relocated_key(&key, &from, &to);
                (key, target)
            })
            .collect::<Vec<_>>();
        for (old, new) in &moves {
            if let Some(document) = self.buffers.remove(old) {
                self.buffers.insert(new.clone(), document);
            }
        }
        for workspace in self.workspaces.values_mut() {
            for tab in &mut workspace.tabs {
                if in_prefix(&tab.key, &from) {
                    tab.key = relocated_key(&tab.key, &from, &to);
                }
            }
            if let Some(active) = &mut workspace.active
                && in_prefix(active, &from)
            {
                *active = relocated_key(active, &from, &to);
            }
        }
        Ok(moves)
    }
    pub fn load_if_missing(&mut self, request: u64, key: DocumentKey) -> bool {
        if self.buffers.contains_key(&key)
            || self.loads.values().any(|pending| *pending == key)
            || self.relocation_contains(&key)
        {
            return false;
        }
        self.loads.insert(request, key);
        true
    }

    pub fn saving_keys(&self) -> Vec<DocumentKey> {
        self.saves.keys().cloned().collect()
    }
    pub fn is_saving(&self, key: &DocumentKey) -> bool {
        self.saves.get(key).is_some_and(|q| !q.is_empty())
    }
    pub fn get(&self, key: &DocumentKey) -> Option<&Document> {
        self.buffers.get(key)
    }
    fn get_mut(&mut self, key: &DocumentKey) -> Option<&mut Document> {
        self.buffers.get_mut(key)
    }
    pub fn edit(&mut self, key: &DocumentKey, edit: impl FnOnce(&mut TextBuffer)) {
        if let Some(document) = self.get_mut(key) {
            let revision = document.buffer.revision();
            edit(&mut document.buffer);
            if document.buffer.revision() != revision {
                self.pin(key);
            }
        }
    }
    pub fn active_key(&self, workspace: &str) -> Option<&DocumentKey> {
        self.workspaces.get(workspace)?.active.as_ref()
    }
    pub fn active(&self, workspace: &str) -> Option<&Document> {
        self.get(self.active_key(workspace)?)
    }
    pub fn tabs(&self, workspace: &str) -> &[DocumentTab] {
        self.workspaces
            .get(workspace)
            .map(|w| w.tabs.as_slice())
            .unwrap_or(&[])
    }
    /// Activate an existing buffer without a disk read, or register a read.
    /// Clean preview tabs may be replaced; edited previews become pinned.
    pub fn open(&mut self, request: u64, key: DocumentKey, pinned: bool) -> bool {
        if !self.workspaces.contains_key(&key.workspace) {
            self.workspace_order.push(key.workspace.clone());
        }
        let workspace = self.workspaces.entry(key.workspace.clone()).or_default();
        for tab in &mut workspace.tabs {
            if self
                .buffers
                .get(&tab.key)
                .is_some_and(Document::is_modified)
            {
                tab.pinned = true;
            }
        }
        if let Some(tab) = workspace.tabs.iter_mut().find(|t| t.key == key) {
            tab.pinned |= pinned;
        } else {
            if let Some(index) = workspace.tabs.iter().position(|t| !t.pinned) {
                let previous = workspace.tabs.remove(index);
                // Cancel this preview's pending read. Its eventual response must
                // not reopen it or replace a later selection.
                self.loads.retain(|_, k| *k != previous.key);
                self.buffers.remove(&previous.key);
            }
            let pinned = pinned || self.buffers.get(&key).is_some_and(Document::is_modified);
            workspace.tabs.push(DocumentTab {
                key: key.clone(),
                pinned,
            });
        }
        workspace.active = Some(key.clone());
        self.load_if_missing(request, key)
    }
    pub fn pin(&mut self, key: &DocumentKey) {
        if let Some(workspace) = self.workspaces.get_mut(&key.workspace)
            && let Some(tab) = workspace.tabs.iter_mut().find(|t| t.key == *key)
        {
            tab.pinned = true;
        }
    }
    pub fn activate(&mut self, key: &DocumentKey) {
        if let Some(workspace) = self.workspaces.get_mut(&key.workspace)
            && workspace.tabs.iter().any(|t| t.key == *key)
        {
            workspace.active = Some(key.clone());
        }
    }
    pub fn finish_load(&mut self, id: u64, rel: &str, text: Option<String>) -> Option<DocumentKey> {
        let key = self.loads.get(&id)?;
        if key.rel != rel {
            return None;
        }
        let key = self.loads.remove(&id)?;
        if let Some(text) = text {
            self.buffers
                .entry(key.clone())
                .or_insert_with(|| Document::new(text));
        }
        Some(key)
    }
    /// Retire a completed UI activation only after all edits and I/O are settled.
    /// Other contexts on the same physical root keep their buffers and undo.
    pub fn close_workspace(&mut self, workspace: &str) -> bool {
        if self.workspace_busy(workspace) || !self.workspace_modified_keys(workspace).is_empty() {
            return false;
        }
        self.workspaces.remove(workspace);
        self.workspace_order.retain(|owner| owner != workspace);
        self.buffers.retain(|key, _| key.workspace != workspace);
        true
    }
    /// Dirty buffers remain owned by the workspace when their view is closed.
    /// Closing a tab never silently destroys undo history or unsaved content.
    pub fn close(&mut self, key: &DocumentKey) {
        if let Some(workspace) = self.workspaces.get_mut(&key.workspace)
            && let Some(index) = workspace.tabs.iter().position(|t| t.key == *key)
        {
            workspace.tabs.remove(index);
            if workspace.active.as_ref() == Some(key) {
                workspace.active = workspace
                    .tabs
                    .get(index.min(workspace.tabs.len().saturating_sub(1)))
                    .map(|t| t.key.clone());
            }
        }
        self.loads.retain(|_, k| k != key);
        if self.buffers.get(key).is_some_and(|d| !d.is_modified()) && !self.saves.contains_key(key)
        {
            self.buffers.remove(key);
        }
    }
    /// Return a snapshot to dispatch now; subsequent saves queue behind it.
    pub fn save(&mut self, id: u64, session: String, key: &DocumentKey) -> Option<FileRequest> {
        self.save_version(id, session, key, false)
    }
    pub fn overwrite(
        &mut self,
        id: u64,
        session: String,
        key: &DocumentKey,
    ) -> Option<FileRequest> {
        self.save_version(id, session, key, true)
    }
    fn save_version(
        &mut self,
        id: u64,
        session: String,
        key: &DocumentKey,
        overwrite: bool,
    ) -> Option<FileRequest> {
        if self.relocation_contains(key) {
            return None;
        }
        let document = self.get(key)?;
        if document.conflict.is_some() && !overwrite {
            return None;
        }
        let normalized_text = document.buffer.text().to_owned();
        let text = document.eol.encode(&normalized_text);
        let expected = document.disk_saved_text.clone();
        let external_revision = document.external_revision;
        self.pin(key);
        let queue = self.saves.entry(key.clone()).or_default();
        let request = FileRequest {
            id,
            session,
            key: key.clone(),
            text,
            expected,
            overwrite,
            normalized_text,
            external_revision,
        };
        let dispatch = queue.is_empty().then(|| request.clone());
        queue.push_back(request);
        dispatch
    }
    /// A successful save acknowledges its exact snapshot, never current text.
    /// The next queued snapshot is released only after this write has finished.
    pub fn finish_save(
        &mut self,
        id: u64,
        rel: &str,
        success: bool,
    ) -> Option<(DocumentKey, Option<FileRequest>)> {
        self.complete_save(id, rel, success, None)
    }
    pub fn complete_save(
        &mut self,
        id: u64,
        rel: &str,
        success: bool,
        conflict: Option<ExternalChange>,
    ) -> Option<(DocumentKey, Option<FileRequest>)> {
        let key = self.saves.iter().find_map(|(key, queue)| {
            queue
                .front()
                .filter(|r| r.id == id && r.key.rel == rel)
                .map(|_| key.clone())
        })?;
        if let Some(conflict) = conflict {
            self.mark_conflict(&key, conflict);
        }
        let queue = self.saves.get_mut(&key)?;
        let saved = queue.pop_front()?;
        if success
            && let Some(document) = self.buffers.get_mut(&key)
            && saved.external_revision == document.external_revision
        {
            document.saved_text = saved.normalized_text;
            document.disk_saved_text = saved.text;
            document.conflict = None;
            document.deleted = false;
            if let Some(next) = queue.front_mut() {
                next.expected = document.disk_saved_text.clone();
            }
        }
        if let Some(document) = self.buffers.get(&key)
            && document.conflict.is_some()
        {
            queue.retain(|request| {
                request.overwrite && request.external_revision == document.external_revision
            });
        }
        let next = queue.front().cloned();
        if queue.is_empty() {
            self.saves.remove(&key);
        }
        if !self.tabs(&key.workspace).iter().any(|tab| tab.key == key)
            && !self.is_saving(&key)
            && self
                .get(&key)
                .is_some_and(|document| !document.is_modified())
        {
            self.buffers.remove(&key);
        }
        Some((key, next))
    }
    pub fn mark_conflict(&mut self, key: &DocumentKey, change: ExternalChange) {
        if let Some(document) = self.get_mut(key) {
            document.external_revision = document.external_revision.wrapping_add(1);
            document.deleted = change.deleted;
            document.conflict = Some(change);
        }
        self.pin(key);
        // The first request is already in flight and must retain its identity.
        // Every unsent save is canceled when a newer external version arrives.
        if let Some(queue) = self.saves.get_mut(key) {
            queue.truncate(1);
        }
    }
    /// Use only for an acknowledged, identified rename receipt. The workspace
    /// observer verified source identity and bytes, so an unchanged saved
    /// baseline is a relocation, not an external edit to a dirty buffer.
    pub fn observe_relocation(
        &mut self,
        key: &DocumentKey,
        change: ExternalChange,
        source_unchanged: bool,
    ) -> bool {
        if source_unchanged
            && !change.deleted
            && change.read_error.is_none()
            && self.get(key).is_some_and(|document| {
                change.content.as_deref() == Some(document.disk_saved_text.as_str())
            })
        {
            return false;
        }
        self.observe(key, change, None)
    }

    /// Only an identified echo of the exact in-flight write bypasses external
    /// change handling. Equal text alone cannot identify a writer.
    pub fn observe(
        &mut self,
        key: &DocumentKey,
        change: ExternalChange,
        write_id: Option<u64>,
    ) -> bool {
        if write_id.is_some_and(|id| {
            self.saves
                .get(key)
                .and_then(|queue| queue.front())
                .is_some_and(|request| {
                    request.id == id
                        && !change.deleted
                        && change.content.as_deref() == Some(request.text.as_str())
                })
        }) {
            return false;
        }
        let Some(document) = self.get_mut(key) else {
            return false;
        };
        if document.dirty() || change.read_error.is_some() {
            self.mark_conflict(key, change);
        } else {
            document.external_revision = document.external_revision.wrapping_add(1);
            if let Some(content) = change.content {
                let normalized = Eol::normalize(&content);
                let caret = document.buffer.caret();
                document.buffer.select_all();
                document.buffer.insert(&normalized);
                document.buffer.move_to(caret, false);
                document.saved_text = normalized;
                document.disk_saved_text = content;
                document.eol = Eol::detect(&document.disk_saved_text);
            }
            document.deleted = change.deleted;
            document.conflict = None;
            if let Some(queue) = self.saves.get_mut(key) {
                queue.truncate(1);
            }
        }
        true
    }
    /// An unambiguous external rename moves buffer, selection, undo and tab
    /// ownership together. Never overwrite another already-open destination.
    pub fn relocate(&mut self, old: &DocumentKey, new: DocumentKey) -> bool {
        if old.workspace != new.workspace
            || old.directory != new.directory
            || old == &new
            || self.buffers.contains_key(&new)
            || self.tabs(&new.workspace).iter().any(|tab| tab.key == new)
        {
            return false;
        }
        let Some(document) = self.buffers.remove(old) else {
            return false;
        };
        self.buffers.insert(new.clone(), document);
        self.loads.retain(|_, key| key != old);
        self.saves.remove(old);
        if let Some(workspace) = self.workspaces.get_mut(&old.workspace) {
            for tab in &mut workspace.tabs {
                if &tab.key == old {
                    tab.key = new.clone();
                }
            }
            if workspace.active.as_ref() == Some(old) {
                workspace.active = Some(new);
            }
        }
        true
    }
    pub fn keep_editing_to_merge(&mut self, key: &DocumentKey) {
        if let Some(document) = self.get_mut(key)
            && let Some(conflict) = &mut document.conflict
        {
            conflict.merging = true;
        }
    }
    pub fn reload_disk(&mut self, key: &DocumentKey) -> bool {
        if self.is_saving(key) {
            return false;
        }
        let Some(document) = self.get_mut(key) else {
            return false;
        };
        let Some(change) = document.conflict.take() else {
            return false;
        };
        if change.read_error.is_some() {
            document.conflict = Some(change);
            return false;
        }
        let content = change.content.unwrap_or_default();
        let normalized = Eol::normalize(&content);
        let caret = document.buffer.caret();
        document.buffer.select_all();
        document.buffer.insert(&normalized);
        document.buffer.move_to(caret, false);
        document.saved_text = normalized;
        document.disk_saved_text = content;
        document.eol = Eol::detect(&document.disk_saved_text);
        document.deleted = change.deleted;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsaved_files_follow_workspace_and_tab_order_including_retained_closed_edits() {
        let mut documents = Documents::default();
        let keys = [
            key("/second", "z.txt"),
            key("/second", "a.txt"),
            key("/first", "b.txt"),
        ];
        for (index, key) in keys.iter().enumerate() {
            load(&mut documents, index as u64, key, "saved", true);
            documents.edit(key, |buffer| buffer.insert("dirty "));
        }
        assert_eq!(documents.unsaved_keys(), keys);
        documents.close(&keys[0]);
        assert_eq!(
            documents.unsaved_keys(),
            vec![keys[1].clone(), keys[2].clone(), keys[0].clone()]
        );
    }

    #[test]
    fn document_workspace_lease_covers_pending_saves_and_reopen_reads_fresh_after_close() {
        let mut documents = Documents::default();
        let key = key("/workspace", "file.txt");
        load(&mut documents, 1, &key, "saved", true);
        assert!(documents.retains_workspace("/workspace"));
        documents.save(2, "session".into(), &key);
        documents.close(&key);
        assert!(documents.retains_workspace("/workspace"));
        documents.finish_save(2, "file.txt", true);
        assert!(!documents.retains_workspace("/workspace"));
        assert!(documents.open(3, key.clone(), true));
        documents.finish_load(3, "file.txt", Some("new disk content".into()));
        assert_eq!(
            documents.get(&key).unwrap().buffer.text(),
            "new disk content"
        );
    }

    #[test]
    fn external_updates_reload_clean_buffers_and_preserve_dirty_deleted_edits() {
        let mut documents = Documents::default();
        let key = key("/workspace", "file.txt");
        load(&mut documents, 1, &key, "before", true);
        documents.observe(
            &key,
            ExternalChange {
                content: Some("external 🌍".into()),
                ..Default::default()
            },
            None,
        );
        assert_eq!(documents.get(&key).unwrap().buffer.text(), "external 🌍");
        assert!(!documents.get(&key).unwrap().dirty());
        documents.edit(&key, |buffer| buffer.insert("local "));
        let edited = documents.get(&key).unwrap().buffer.text().to_owned();
        documents.observe(
            &key,
            ExternalChange {
                deleted: true,
                ..Default::default()
            },
            None,
        );
        assert_eq!(documents.get(&key).unwrap().buffer.text(), edited);
        assert!(
            documents
                .get(&key)
                .unwrap()
                .conflict
                .as_ref()
                .unwrap()
                .deleted
        );
        assert!(documents.save(2, "session".into(), &key).is_none());
        documents.reload_disk(&key);
        assert!(documents.get(&key).unwrap().deleted);
        assert_eq!(documents.get(&key).unwrap().buffer.text(), "");
        documents.get_mut(&key).unwrap().buffer.undo();
        assert_eq!(documents.get(&key).unwrap().buffer.text(), edited);
    }

    #[test]
    fn save_echo_requires_write_identity_and_late_ack_keeps_newer_disk_state() {
        let mut documents = Documents::default();
        let key = key("/workspace", "file.txt");
        load(&mut documents, 1, &key, "before", true);
        documents.edit(&key, |buffer| buffer.insert("saved "));
        let request = documents.save(2, "session".into(), &key).unwrap();
        let echo = ExternalChange {
            content: Some(request.text.clone()),
            ..Default::default()
        };
        assert!(!documents.observe(&key, echo.clone(), Some(2)));
        assert!(documents.get(&key).unwrap().conflict.is_none());
        assert!(documents.observe(&key, echo, None));
        assert!(documents.get(&key).unwrap().conflict.is_some());
        documents.finish_save(2, "file.txt", true);
        assert!(documents.get(&key).unwrap().conflict.is_some());

        let mut documents = Documents::default();
        load(&mut documents, 1, &key, "before", true);
        documents.edit(&key, |buffer| buffer.insert("saved "));
        documents.save(2, "session".into(), &key);
        documents.save(3, "session".into(), &key);
        documents.get_mut(&key).unwrap().buffer.undo();
        assert!(!documents.get(&key).unwrap().dirty());
        documents.observe(
            &key,
            ExternalChange {
                content: Some("newer disk".into()),
                ..Default::default()
            },
            None,
        );
        assert!(
            documents
                .finish_save(2, "file.txt", true)
                .unwrap()
                .1
                .is_none()
        );
        assert_eq!(documents.get(&key).unwrap().buffer.text(), "newer disk");
        assert!(!documents.get(&key).unwrap().dirty());
        assert!(!documents.is_saving(&key));
    }

    #[test]
    fn removal_batch_uses_original_tab_order_and_keeps_failed_prefixes() {
        let mut d = Documents::default();
        let a = key("/one", "a/one");
        let keep = key("/one", "keep");
        let b = key("/one", "b/two");
        let last = key("/one", "last");
        for (id, key) in [(1, &a), (2, &keep), (3, &b), (4, &last)] {
            load(&mut d, id, key, "baseline", true);
        }
        d.edit(&b, |buffer| buffer.insert("keep dirty "));
        d.activate(&a);
        d.begin_removal(10, key("/one", "a")).unwrap();
        d.begin_removal(11, key("/one", "b")).unwrap();
        d.finish_removals(&[(10, true), (11, false)]).unwrap();
        assert_eq!(
            d.tabs("/one")
                .iter()
                .map(|tab| tab.key.clone())
                .collect::<Vec<_>>(),
            vec![keep.clone(), b.clone(), last]
        );
        assert_eq!(d.active_key("/one"), Some(&keep));
        assert!(d.get(&b).unwrap().dirty());
        d.get_mut(&b).unwrap().buffer.undo();
        assert_eq!(d.get(&b).unwrap().buffer.text(), "baseline");
        assert!(!d.relocation_pending());
    }

    #[test]
    fn removal_waits_for_io_blocks_recreation_and_keeps_failure_edits() {
        let mut d = Documents::default();
        let source = key("/one", "folder/open.txt");
        assert!(d.open(1, source.clone(), true));
        assert_eq!(
            d.begin_removal(10, key("/one", "folder")),
            Err(RelocationError::Busy)
        );
        d.finish_load(1, "folder/open.txt", Some("baseline".into()));
        d.edit(&source, |buffer| buffer.insert("dirty "));
        assert!(d.save(2, "session".into(), &source).is_some());
        assert_eq!(
            d.begin_removal(10, key("/one", "folder")),
            Err(RelocationError::Busy)
        );
        d.finish_save(2, "folder/open.txt", true);
        d.edit(&source, |buffer| buffer.insert("more "));
        d.begin_removal(10, key("/one", "folder")).unwrap();
        assert!(d.save(3, "session".into(), &source).is_none());
        assert!(!d.open(4, key("/one", "folder/new.txt"), true));
        assert_eq!(
            d.begin_relocation(11, key("/one", "folder"), key("/one", "renamed")),
            Err(RelocationError::Busy)
        );
        assert!(d.finish_removal(10, false).unwrap().is_empty());
        assert!(d.get(&source).unwrap().dirty());
        d.get_mut(&source).unwrap().buffer.undo();
        assert_eq!(d.get(&source).unwrap().buffer.text(), "dirty baseline");
        let other = key("/two", "folder/open.txt");
        load(&mut d, 5, &other, "other workspace", true);
        let neighbor = key("/one", "folderish/open.txt");
        load(&mut d, 6, &neighbor, "neighbor", true);
        d.begin_removal(12, key("/one", "folder")).unwrap();
        let removed = d.finish_removal(12, true).unwrap();
        assert!(removed.contains(&source));
        assert!(d.get(&source).is_none());
        assert!(d.tabs("/one").iter().all(|tab| tab.key == neighbor));
        assert!(d.get(&other).is_some());
        assert!(d.get(&neighbor).is_some());
        assert!(
            d.finish_load(4, "folder/new.txt", Some("stale read".into()))
                .is_none()
        );
        assert!(d.save(7, "session".into(), &source).is_none());
    }

    #[test]
    fn external_rename_moves_dirty_buffer_undo_and_tabs_without_overwriting_open_destination() {
        let mut documents = Documents::default();
        let old = key("/workspace", "before.txt");
        let new = key("/workspace", "folder/after.txt");
        load(&mut documents, 1, &old, "before", true);
        documents.edit(&old, |buffer| buffer.insert("edited "));
        documents.save(2, "session".into(), &old);
        assert!(documents.relocate(&old, new.clone()));
        assert_eq!(documents.active_key("/workspace"), Some(&new));
        assert_eq!(documents.tabs("/workspace")[0].key, new);
        assert!(documents.finish_save(2, "before.txt", true).is_none());
        documents.get_mut(&new).unwrap().buffer.undo();
        assert_eq!(documents.get(&new).unwrap().buffer.text(), "before");
        load(&mut documents, 3, &old, "destination content", true);
        assert!(!documents.relocate(&new, old.clone()));
        assert_eq!(
            documents.get(&old).unwrap().buffer.text(),
            "destination content"
        );
    }

    #[test]
    fn detachment_retains_tabs_dirty_undo_and_neighbors_and_marks_deleted_only_on_success() {
        let mut documents = Documents::default();
        let dirty = key("/one", "folder/dirty");
        let clean = key("/one", "folder/clean");
        let adjacent = key("/one", "folderish");
        let other = key("/two", "folder/dirty");
        for (id, key) in [(1, &dirty), (2, &clean), (3, &adjacent), (4, &other)] {
            load(&mut documents, id, key, "saved", true);
        }
        documents.edit(&dirty, |buffer| buffer.insert(" edited"));
        documents.begin_removal(10, key("/one", "folder")).unwrap();
        assert!(documents.save(11, "session".into(), &dirty).is_none());
        assert!(documents.finish_detachment(10, false).unwrap().is_empty());
        assert!(documents.get(&dirty).unwrap().dirty());
        assert!(!documents.get(&dirty).unwrap().deleted);
        documents.begin_removal(12, key("/one", "folder")).unwrap();
        let affected = documents.finish_detachment(12, true).unwrap();
        assert_eq!(affected.len(), 2);
        assert_eq!(documents.tabs("/one").len(), 3);
        assert!(documents.get(&dirty).unwrap().deleted);
        assert!(
            documents
                .get(&dirty)
                .unwrap()
                .conflict
                .as_ref()
                .unwrap()
                .deleted
        );
        assert!(documents.get(&clean).unwrap().deleted);
        assert_eq!(documents.get(&clean).unwrap().buffer.text(), "saved");
        assert!(!documents.get(&adjacent).unwrap().deleted);
        assert!(!documents.get(&other).unwrap().deleted);
        documents.get_mut(&dirty).unwrap().buffer.undo();
        assert_eq!(documents.get(&dirty).unwrap().buffer.text(), "saved");
    }

    #[test]
    fn workspace_busy_tracks_load_save_and_reservation_without_blocking_other_roots() {
        let mut documents = Documents::default();
        let a = key("/a", "file");
        let b = key("/b", "file");
        assert!(documents.open(1, a.clone(), true));
        assert!(documents.open(2, b.clone(), true));
        assert!(documents.workspace_busy("/a"));
        assert!(documents.workspace_busy("/b"));
        assert!(!documents.workspace_busy("/c"));
        documents
            .finish_load(1, &a.rel, Some("saved".into()))
            .unwrap();
        assert!(!documents.workspace_busy("/a"));
        documents.edit(&a, |buffer| buffer.insert(" edit"));
        documents.save(3, "owner".into(), &a).unwrap();
        assert!(documents.workspace_busy("/a"));
        documents.finish_save(3, &a.rel, true).unwrap();
        assert!(!documents.workspace_busy("/a"));
        documents.begin_removal(4, a.clone()).unwrap();
        assert!(documents.workspace_busy("/a"));
        documents.finish_removal(4, false).unwrap();
        assert!(!documents.workspace_busy("/a"));
        assert!(documents.workspace_busy("/b"));
        documents
            .finish_load(2, &b.rel, Some("other".into()))
            .unwrap();
        assert!(!documents.workspace_busy("/b"));
    }
    fn key(workspace: &str, rel: &str) -> DocumentKey {
        DocumentKey::new(workspace.into(), rel.into())
    }
    fn load(d: &mut Documents, id: u64, key: &DocumentKey, text: &str, pin: bool) {
        assert!(d.open(id, key.clone(), pin));
        d.finish_load(id, &key.rel, Some(text.into())).unwrap();
    }
    #[test]
    fn crlf_documents_edit_as_lf_and_restore_their_eol_on_save() {
        let mut documents = Documents::default();
        let key = key("/workspace", "file.txt");
        load(&mut documents, 1, &key, "one\r\ntwo\r\n", true);
        assert_eq!(
            documents.get(&key).unwrap().buffer.text(),
            "one\ntwo\n",
            "CRLF files use LF internally so navigation, edits and formatting stay uniform"
        );
        assert!(!documents.get(&key).unwrap().dirty());

        documents.edit(&key, |buffer| {
            let end = buffer.text().len();
            buffer.move_to(end, false);
            buffer.insert("three\n");
        });
        let request = documents.save(2, "session".into(), &key).unwrap();
        assert_eq!(request.text, "one\r\ntwo\r\nthree\r\n");
        assert_eq!(
            request.expected, "one\r\ntwo\r\n",
            "the compare-and-write expectation remains the raw disk text"
        );
        documents.finish_save(2, "file.txt", true);
        assert!(!documents.get(&key).unwrap().dirty());

        documents.observe(
            &key,
            ExternalChange {
                content: Some("next\nline\n".into()),
                ..Default::default()
            },
            None,
        );
        assert_eq!(documents.get(&key).unwrap().buffer.text(), "next\nline\n");
        documents.edit(&key, |buffer| {
            let end = buffer.text().len();
            buffer.move_to(end, false);
            buffer.insert("tail\n");
        });
        let request = documents.save(3, "session".into(), &key).unwrap();
        assert_eq!(
            request.text, "next\nline\ntail\n",
            "an external LF file switches the save EOL back to LF"
        );
    }

    #[test]
    fn same_disk_file_has_independent_context_buffers_tabs_undo_and_save_ownership() {
        let mut d = Documents::default();
        let a = DocumentKey::in_workspace("context-a".into(), "/root".into(), "file.txt".into());
        let b = DocumentKey::in_workspace("context-b".into(), "/root".into(), "file.txt".into());
        load(&mut d, 1, &a, "saved", true);
        load(&mut d, 2, &b, "saved", true);
        d.edit(&a, |buffer| buffer.insert("A "));
        d.edit(&b, |buffer| buffer.insert("B "));
        assert_eq!(d.active_key("context-a"), Some(&a));
        assert_eq!(d.active_key("context-b"), Some(&b));
        assert_eq!(d.get(&a).unwrap().buffer.text(), "A saved");
        assert_eq!(d.get(&b).unwrap().buffer.text(), "B saved");
        d.get_mut(&a).unwrap().buffer.undo();
        assert_eq!(d.get(&a).unwrap().buffer.text(), "saved");
        assert_eq!(d.get(&b).unwrap().buffer.text(), "B saved");
        let request = d.save(3, "runtime-b".into(), &b).unwrap();
        assert_eq!(request.key.workspace, "context-b");
        assert_eq!(request.key.directory, "/root");
        assert!(!d.workspace_busy("context-a"));
        assert!(d.workspace_busy("context-b"));
        assert_eq!(d.finish_save(3, "file.txt", true).unwrap().0, b);
        assert!(d.retains_directory("/root"));
        d.close(&a);
        assert!(d.tabs("context-a").is_empty());
        assert_eq!(d.tabs("context-b").len(), 1);
        assert_eq!(d.get(&b).unwrap().buffer.text(), "B saved");
    }
    #[test]
    fn retiring_context_refuses_pending_io_and_edits_and_preserves_same_root_sibling() {
        let mut d = Documents::default();
        let a = DocumentKey::in_workspace("a".into(), "/root".into(), "file".into());
        let b = DocumentKey::in_workspace("b".into(), "/root".into(), "file".into());
        assert!(d.open(1, a.clone(), true));
        assert!(!d.close_workspace("a"));
        d.finish_load(1, "file", Some("saved".into())).unwrap();
        load(&mut d, 2, &b, "saved", true);
        d.edit(&a, |buffer| buffer.insert("edit "));
        assert!(!d.close_workspace("a"));
        d.save(3, "runtime".into(), &a).unwrap();
        assert!(!d.close_workspace("a"));
        d.finish_save(3, "file", true).unwrap();
        d.edit(&b, |buffer| buffer.insert("sibling "));
        assert!(d.close_workspace("a"));
        assert!(d.get(&a).is_none());
        assert!(d.tabs("a").is_empty());
        assert_eq!(d.active_key("b"), Some(&b));
        assert_eq!(d.get(&b).unwrap().buffer.text(), "sibling saved");
        d.get_mut(&b).unwrap().buffer.undo();
        assert_eq!(d.get(&b).unwrap().buffer.text(), "saved");
        assert!(d.retains_directory("/root"));
    }
    #[test]
    fn physical_save_echo_is_owned_only_by_its_writer_and_updates_other_contexts() {
        let mut d = Documents::default();
        let make = |owner: &str, directory: &str| {
            DocumentKey::in_workspace(owner.into(), directory.into(), "file.txt".into())
        };
        let a = make("a", "/root");
        let b = make("b", "/root");
        let clean = make("clean", "/root");
        let other = make("other", "/other");
        for (id, key) in [(1, &a), (2, &b), (3, &clean), (4, &other)] {
            load(&mut d, id, key, "saved", true);
        }
        d.edit(&a, |buffer| buffer.insert("A "));
        d.edit(&b, |buffer| buffer.insert("B "));
        let request = d.save(5, "runtime-a".into(), &a).unwrap();
        let changed = d.observe_path(
            "/root",
            "file.txt",
            ExternalChange {
                content: Some(request.text),
                ..Default::default()
            },
            Some(5),
        );
        assert!(!changed.contains(&a));
        assert!(changed.contains(&b) && changed.contains(&clean));
        assert!(d.get(&a).unwrap().conflict.is_none());
        assert!(d.get(&b).unwrap().conflict.is_some());
        assert_eq!(d.get(&b).unwrap().buffer.text(), "B saved");
        assert_eq!(d.get(&clean).unwrap().buffer.text(), "A saved");
        assert_eq!(d.get(&other).unwrap().buffer.text(), "saved");
        d.finish_save(5, "file.txt", true).unwrap();
        assert!(!d.get(&a).unwrap().dirty());
        d.get_mut(&b).unwrap().buffer.undo();
        assert_eq!(d.get(&b).unwrap().buffer.text(), "saved");
    }
    #[test]
    fn physical_folder_rename_retains_every_context_identity_and_undo() {
        let mut d = Documents::default();
        let a = DocumentKey::in_workspace("a".into(), "/root".into(), "folder/file.txt".into());
        let b = DocumentKey::in_workspace("b".into(), "/root".into(), "folder/file.txt".into());
        load(&mut d, 1, &a, "saved", true);
        load(&mut d, 2, &b, "saved", true);
        d.edit(&a, |buffer| buffer.insert("A "));
        d.edit(&b, |buffer| buffer.insert("B "));
        d.begin_relocation(3, key("/root", "folder"), key("/root", "renamed"))
            .unwrap();
        assert!(d.workspace_busy("a") && d.workspace_busy("b"));
        assert!(d.save(4, "runtime-a".into(), &a).is_none());
        assert_eq!(d.finish_relocation(3, true).unwrap().len(), 2);
        for (owner, text) in [("a", "A saved"), ("b", "B saved")] {
            let new =
                DocumentKey::in_workspace(owner.into(), "/root".into(), "renamed/file.txt".into());
            assert_eq!(d.active_key(owner), Some(&new));
            assert_eq!(d.get(&new).unwrap().buffer.text(), text);
            d.get_mut(&new).unwrap().buffer.undo();
            assert_eq!(d.get(&new).unwrap().buffer.text(), "saved");
        }
    }
    #[test]
    fn reserved_folder_relocation_keeps_dirty_buffers_selection_undo_and_other_workspaces() {
        let mut d = Documents::default();
        let a = key("/one", "folder/a.txt");
        let closed = key("/one", "folder/closed.txt");
        let adjacent = key("/one", "folderish.txt");
        let other = key("/two", "folder/a.txt");
        for (id, key, text) in [
            (1, &a, "first"),
            (2, &closed, "closed"),
            (3, &adjacent, "adjacent"),
            (4, &other, "other"),
        ] {
            load(&mut d, id, key, text, true);
        }
        d.edit(&a, |buffer| {
            buffer.move_to(0, false);
            buffer.insert("dirty ");
            buffer.select(2, 5);
        });
        d.edit(&closed, |buffer| buffer.insert("retained "));
        d.close(&closed);
        d.activate(&a);
        d.begin_relocation(42, key("/one", "folder"), key("/one", "renamed"))
            .unwrap();
        assert!(d.relocation_pending());
        assert!(d.save(5, "session".into(), &a).is_none());
        assert!(!d.load_if_missing(6, key("/one", "folder/not-loaded.txt")));
        assert!(!d.load_if_missing(7, key("/one", "renamed/not-loaded.txt")));
        let moves = d.finish_relocation(42, true).unwrap();
        assert_eq!(moves.len(), 2);
        let moved = key("/one", "renamed/a.txt");
        let retained = key("/one", "renamed/closed.txt");
        assert_eq!(d.active_key("/one"), Some(&moved));
        assert_eq!(d.get(&moved).unwrap().buffer.selection(), 2..5);
        assert_eq!(d.get(&moved).unwrap().buffer.text(), "dirty first");
        assert!(d.get(&retained).unwrap().dirty());
        assert!(d.get(&closed).is_none());
        assert_eq!(d.get(&adjacent).unwrap().buffer.text(), "adjacent");
        assert_eq!(d.get(&other).unwrap().buffer.text(), "other");
        d.edit(&moved, |buffer| buffer.undo());
        assert_eq!(d.get(&moved).unwrap().buffer.text(), "first");
        assert!(!d.relocation_pending());
        assert!(d.save(8, "session".into(), &moved).is_some());
    }

    #[test]
    fn relocation_waits_for_io_refuses_open_destination_and_failure_releases_reservation() {
        let mut d = Documents::default();
        let old = key("/one", "before.txt");
        let new = key("/one", "after.txt");
        assert!(d.open(1, old.clone(), true));
        assert_eq!(
            d.begin_relocation(10, old.clone(), new.clone()),
            Err(RelocationError::Busy)
        );
        d.finish_load(1, &old.rel, Some("before".into())).unwrap();
        d.edit(&old, |buffer| buffer.insert("edit "));
        let saving = d.save(2, "session".into(), &old).unwrap();
        assert_eq!(
            d.begin_relocation(10, old.clone(), new.clone()),
            Err(RelocationError::Busy)
        );
        d.finish_save(2, &old.rel, true);
        load(&mut d, 3, &new, "retain destination", true);
        assert_eq!(
            d.begin_relocation(10, old.clone(), new.clone()),
            Err(RelocationError::DestinationOpen)
        );
        assert_eq!(d.get(&new).unwrap().buffer.text(), "retain destination");
        d.close(&new);
        d.begin_relocation(10, old.clone(), new.clone()).unwrap();
        assert!(d.save(4, "session".into(), &old).is_none());
        assert!(d.finish_relocation(10, false).unwrap().is_empty());
        assert_eq!(d.get(&old).unwrap().buffer.text(), saving.text);
        assert!(d.get(&new).is_none());
        assert!(d.save(5, "session".into(), &old).is_some());
    }

    #[test]
    fn relocation_acknowledgement_is_idempotent_with_prior_observer_remapping() {
        let mut d = Documents::default();
        let old = key("/one", "before.txt");
        let new = key("/one", "after.txt");
        load(&mut d, 1, &old, "before", true);
        d.edit(&old, |buffer| buffer.insert("dirty "));
        d.begin_relocation(2, old.clone(), new.clone()).unwrap();
        assert!(d.relocate(&old, new.clone()));
        assert!(d.finish_relocation(2, true).unwrap().is_empty());
        assert_eq!(d.tabs("/one").len(), 1);
        assert_eq!(d.active_key("/one"), Some(&new));
        d.edit(&new, |buffer| buffer.undo());
        assert_eq!(d.get(&new).unwrap().buffer.text(), "before");
    }

    #[test]
    fn folder_relocation_can_finish_after_only_some_observer_moves_arrive() {
        let mut d = Documents::default();
        let a = key("/one", "before/a");
        let b = key("/one", "before/b");
        load(&mut d, 1, &a, "first", true);
        load(&mut d, 2, &b, "second", true);
        d.begin_relocation(3, key("/one", "before"), key("/one", "after"))
            .unwrap();
        assert!(d.relocate(&a, key("/one", "after/a")));
        let moved = d.finish_relocation(3, true).unwrap();
        assert_eq!(moved, [(b, key("/one", "after/b"))]);
        assert_eq!(d.tabs("/one").len(), 2);
        assert_eq!(
            d.get(&key("/one", "after/a")).unwrap().buffer.text(),
            "first"
        );
        assert_eq!(
            d.get(&key("/one", "after/b")).unwrap().buffer.text(),
            "second"
        );
    }

    #[test]
    fn verified_rename_receipt_keeps_dirty_undo_but_unidentified_or_changed_sources_conflict() {
        let mut d = Documents::default();
        let old = key("/one", "before.txt");
        let new = key("/one", "after.txt");
        load(&mut d, 1, &old, "baseline", true);
        d.edit(&old, |buffer| buffer.insert("dirty "));
        d.begin_relocation(2, old, new.clone()).unwrap();
        d.finish_relocation(2, true).unwrap();
        let unchanged = ExternalChange {
            content: Some("baseline".into()),
            ..Default::default()
        };
        assert!(!d.observe_relocation(&new, unchanged.clone(), true));
        assert!(d.get(&new).unwrap().conflict.is_none());
        assert_eq!(d.get(&new).unwrap().buffer.text(), "dirty baseline");
        d.edit(&new, |buffer| buffer.undo());
        assert_eq!(d.get(&new).unwrap().buffer.text(), "baseline");
        d.edit(&new, |buffer| buffer.insert("another edit "));
        assert!(d.observe_relocation(&new, unchanged, false));
        assert!(d.get(&new).unwrap().conflict.is_some());
        let changed = ExternalChange {
            content: Some("external".into()),
            ..Default::default()
        };
        assert!(d.observe_relocation(&new, changed, true));
        assert_eq!(
            d.get(&new)
                .unwrap()
                .conflict
                .as_ref()
                .unwrap()
                .content
                .as_deref(),
            Some("external")
        );
        assert_eq!(d.get(&new).unwrap().buffer.text(), "another edit baseline");
    }

    #[test]
    fn editing_pins_a_preview_even_after_undo_but_selection_does_not() {
        let mut d = Documents::default();
        let a = key("/one", "a");
        load(&mut d, 1, &a, "disk", false);
        d.edit(&a, |b| b.select_all());
        assert!(!d.tabs("/one")[0].pinned);
        d.edit(&a, |b| b.insert("changed"));
        d.edit(&a, TextBuffer::undo);
        assert!(!d.get(&a).unwrap().dirty());
        assert!(d.tabs("/one")[0].pinned);
        load(&mut d, 2, &key("/one", "b"), "second", false);
        assert_eq!(d.tabs("/one").len(), 2);
    }
    #[test]
    fn conflicting_save_pauses_queue_and_requires_explicit_resolution() {
        let mut d = Documents::default();
        let a = key("/one", "a");
        load(&mut d, 1, &a, "disk", true);
        d.get_mut(&a).unwrap().buffer.insert("local ");
        d.save(2, "s".into(), &a).unwrap();
        assert!(d.save(3, "s".into(), &a).is_none());
        let external = ExternalChange {
            content: Some("external".into()),
            ..Default::default()
        };
        assert!(
            d.complete_save(2, "a", false, Some(external))
                .unwrap()
                .1
                .is_none()
        );
        assert_eq!(d.get(&a).unwrap().buffer.text(), "local disk");
        assert!(!d.is_saving(&a));
        assert!(d.save(4, "s".into(), &a).is_none());
        d.keep_editing_to_merge(&a);
        assert!(d.get(&a).unwrap().conflict.as_ref().unwrap().merging);
        d.get_mut(&a).unwrap().buffer.insert("merged ");
        let request = d.overwrite(5, "s".into(), &a).unwrap();
        assert!(request.overwrite);
        d.finish_save(5, "a", true);
        assert!(!d.get(&a).unwrap().is_modified());
        assert!(d.get(&a).unwrap().conflict.is_none());
    }
    #[test]
    fn reloading_external_text_preserves_local_edits_in_undo_and_rejects_unreadable_snapshots() {
        let mut d = Documents::default();
        let a = key("/one", "a");
        load(&mut d, 1, &a, "disk", false);
        d.get_mut(&a).unwrap().buffer.insert("local ");
        d.mark_conflict(
            &a,
            ExternalChange {
                content: Some("external".into()),
                ..Default::default()
            },
        );
        assert!(d.reload_disk(&a));
        assert_eq!(d.get(&a).unwrap().buffer.text(), "external");
        assert!(!d.get(&a).unwrap().dirty());
        d.get_mut(&a).unwrap().buffer.undo();
        assert_eq!(d.get(&a).unwrap().buffer.text(), "local disk");
        assert!(d.get(&a).unwrap().dirty());
        d.mark_conflict(
            &a,
            ExternalChange {
                read_error: Some("cannot read disk".into()),
                ..Default::default()
            },
        );
        assert!(!d.reload_disk(&a));
        assert_eq!(d.get(&a).unwrap().buffer.text(), "local disk");
        d.mark_conflict(
            &a,
            ExternalChange {
                deleted: true,
                ..Default::default()
            },
        );
        assert!(d.reload_disk(&a));
        assert!(d.get(&a).unwrap().deleted);
        assert_eq!(d.get(&a).unwrap().buffer.text(), "");
    }
    #[test]
    fn late_overwrite_ack_cannot_clear_a_newer_external_conflict() {
        let mut d = Documents::default();
        let a = key("/one", "a");
        load(&mut d, 1, &a, "disk", true);
        d.get_mut(&a).unwrap().buffer.insert("local ");
        d.mark_conflict(
            &a,
            ExternalChange {
                content: Some("external 1".into()),
                ..Default::default()
            },
        );
        d.overwrite(2, "s".into(), &a).unwrap();
        d.mark_conflict(
            &a,
            ExternalChange {
                content: Some("external 2".into()),
                ..Default::default()
            },
        );
        assert!(d.is_saving(&a));
        d.finish_save(2, "a", true);
        assert_eq!(
            d.get(&a)
                .unwrap()
                .conflict
                .as_ref()
                .unwrap()
                .content
                .as_deref(),
            Some("external 2")
        );
        assert!(d.get(&a).unwrap().is_modified());
    }
    #[test]
    fn switching_files_sessions_and_closing_views_preserves_dirty_buffer_and_undo() {
        let mut d = Documents::default();
        let a = key("/one", "a.txt");
        let b = key("/one", "b.txt");
        load(&mut d, 1, &a, "first", false);
        d.get_mut(&a).unwrap().buffer.insert("edited ");
        load(&mut d, 2, &b, "second", false);
        assert_eq!(d.tabs("/one").len(), 2);
        assert!(d.tabs("/one")[0].pinned);
        load(&mut d, 3, &key("/two", "a.txt"), "other workspace", false);
        d.close(&a);
        assert!(!d.open(4, a.clone(), false));
        assert_eq!(d.get(&a).unwrap().buffer.text(), "edited first");
        d.get_mut(&a).unwrap().buffer.undo();
        assert_eq!(d.get(&a).unwrap().buffer.text(), "first");
        assert!(!d.get(&a).unwrap().dirty());
    }
    #[test]
    fn late_reads_cannot_replace_newer_selection_or_same_name_in_another_workspace() {
        let mut d = Documents::default();
        let a = key("/one", "a");
        let b = key("/one", "b");
        assert!(d.open(1, a.clone(), false));
        assert!(d.open(2, b.clone(), false));
        assert!(d.finish_load(1, "a", Some("stale".into())).is_none());
        d.finish_load(2, "b", Some("new".into()));
        assert_eq!(d.active_key("/one"), Some(&b));
        let other = key("/two", "b");
        load(&mut d, 3, &other, "other", true);
        assert_eq!(d.get(&b).unwrap().buffer.text(), "new");
        assert_eq!(d.get(&other).unwrap().buffer.text(), "other");
    }
    #[test]
    fn save_acknowledges_exact_snapshot_and_serializes_writes_across_navigation() {
        let mut d = Documents::default();
        let a = key("/one", "a");
        load(&mut d, 1, &a, "one", true);
        d.get_mut(&a).unwrap().buffer.insert("2");
        let first = d.save(2, "session-a".into(), &a).unwrap();
        assert_eq!(first.text, "2one");
        d.get_mut(&a).unwrap().buffer.insert("3");
        assert!(d.save(3, "session-a".into(), &a).is_none());
        load(&mut d, 4, &key("/two", "a"), "another", false);
        // No second write has been sent yet; a forged/out-of-order ack is ignored.
        assert!(d.finish_save(3, "a", true).is_none());
        let (_, next) = d.finish_save(2, "a", true).unwrap();
        let next = next.unwrap();
        assert_eq!(next.text, "23one");
        assert_eq!(next.expected, "2one");
        assert!(d.get(&a).unwrap().dirty());
        d.finish_save(3, "a", true);
        assert!(!d.get(&a).unwrap().dirty());
        assert_eq!(d.active("/two").unwrap().buffer.text(), "another");
    }
    #[test]
    fn failed_save_keeps_dirty_state_and_still_releases_the_next_request() {
        let mut d = Documents::default();
        let a = key("/one", "a");
        load(&mut d, 1, &a, "one", false);
        d.get_mut(&a).unwrap().buffer.insert("two");
        d.save(2, "s".into(), &a);
        d.save(3, "s".into(), &a);
        assert!(d.finish_save(2, "a", false).unwrap().1.is_some());
        assert!(d.get(&a).unwrap().dirty());
        assert!(d.finish_save(3, "wrong-name", true).is_none());
        d.finish_save(3, "a", true);
        assert!(!d.get(&a).unwrap().dirty());
    }
}
