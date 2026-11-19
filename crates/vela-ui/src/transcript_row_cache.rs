//! Native view boundaries for settled rows and independent delegated agents.
//!
//! GPUI remains the height authority: an entry first renders without caching,
//! then reuses only a height measured by the owning native list. Measurements
//! are captured before entering its render callback, which holds a list borrow.
use super::*;

const ROWS_PER_SESSION: usize = 512;

#[derive(Default)]
pub(super) struct TranscriptRowCache {
    sessions: HashMap<String, SessionRows>,
    window_size: Option<gpui::Size<Pixels>>,
}

#[derive(Default)]
struct SessionRows {
    entries: HashMap<SharedString, Entry>,
    clock: u64,
    viewport_width: Option<Pixels>,
}

struct Entry {
    entity: gpui::Entity<CachedTranscriptRow>,
    revision: crate::transcript_viewport::RowRevision,
    measured: Option<gpui::Size<Pixels>>,
    pending_measurement: Option<usize>,
    last_used: u64,
    #[cfg(test)]
    renders: Rc<std::cell::Cell<u64>>,
}

struct CachedTranscriptRow {
    owner: gpui::WeakEntity<VelaApp>,
    session: String,
    row: transcript_view::Row,
    #[cfg(test)]
    renders: Rc<std::cell::Cell<u64>>,
}

impl Render for CachedTranscriptRow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        self.renders.set(self.renders.get() + 1);
        self.owner
            .update(cx, |app, cx| {
                app.render_panel_transcript_row(&self.session, &self.row, window, cx)
            })
            .unwrap_or_else(|_| div().into_any_element())
    }
}

impl TranscriptRowCache {
    /// Call before building the list element, never from its item renderer.
    pub(super) fn capture_measurements(
        &mut self,
        session: &str,
        list: &ListState,
        rows: &[transcript_view::Row],
        window: &Window,
    ) {
        let window_size = window.viewport_size();
        if self.window_size != Some(window_size) {
            self.invalidate_all();
            self.window_size = Some(window_size);
        }
        let Some(state) = self.sessions.get_mut(session) else {
            return;
        };
        let viewport_width = list.viewport_bounds().size.width;
        if state.viewport_width != Some(viewport_width) {
            for entry in state.entries.values_mut() {
                entry.measured = None;
            }
            state.viewport_width = Some(viewport_width);
        }

        // Only newly laid-out rows can supply a missing measurement. Walk the
        // bounded cache instead of rehashing the transcript on every wheel frame.
        for (id, entry) in &mut state.entries {
            let Some(index) = entry.pending_measurement.take() else {
                continue;
            };
            // A stream update may splice or revise this row before capture.
            if rows.get(index).is_some_and(|row| row.id == *id && row.revision() == entry.revision) {
                entry.measured = list.bounds_for_item(index)
                    .map(|bounds| bounds.size)
                    .filter(|size| size.width > px(0.) && size.height > px(0.));
            }
        }
    }

    /// Source revisions, Markdown dependencies and explicit UI invalidation
    /// refresh these boundaries. A growing answer keeps its own view while
    /// revisions continue to invalidate its natural height.
    pub(super) fn element(
        &mut self,
        owner: gpui::WeakEntity<VelaApp>,
        session: &str,
        row: &transcript_view::Row,
        index: usize,
        natural_height: bool,
        cx: &mut Context<VelaApp>,
    ) -> AnyElement {
        let state = self.sessions.entry(session.to_owned()).or_default();
        state.clock = state.clock.wrapping_add(1);
        if !state.entries.contains_key(&row.id)
            && state.entries.len() >= ROWS_PER_SESSION
            && let Some(oldest) = state
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(id, _)| id.clone())
        {
            state.entries.remove(&oldest);
        }
        let revision = row.revision();
        let entry = state.entries.entry(row.id.clone()).or_insert_with(|| {
            #[cfg(test)]
            let renders = Rc::new(std::cell::Cell::new(0));
            let entity = cx.new(|_| CachedTranscriptRow {
                owner,
                session: session.to_owned(),
                row: row.clone(),
                #[cfg(test)]
                renders: renders.clone(),
            });
            Entry {
                entity,
                revision: revision.clone(),
                measured: None,
                pending_measurement: None,
                last_used: state.clock,
                #[cfg(test)]
                renders,
            }
        });
        entry.last_used = state.clock;
        if entry.revision != revision {
            entry.entity.update(cx, |view, cx| {
                view.row = row.clone();
                cx.notify();
            });
            entry.revision = revision;
            entry.measured = None;
            entry.pending_measurement = None;
        }
        // GPUI lends unused turn space to the last row. Never carry that
        // artificial height into another row or beyond the reservation.
        if natural_height {
            entry.measured = None;
            entry.pending_measurement = None;
        } else if entry.measured.is_none() {
            entry.pending_measurement = Some(index);
        }
        match entry.measured {
            Some(size) => entry
                .entity
                .clone()
                .cached(gpui::StyleRefinement::default().w_full().h(size.height))
                .into_any_element(),
            None => entry.entity.clone().into_any_element(),
        }
    }

    /// User interaction, theme/layout changes and explicit remeasurement may
    /// alter presentation independently of immutable stream source revisions.
    pub(super) fn invalidate_all(&mut self) {
        for state in self.sessions.values_mut() {
            for entry in state.entries.values_mut() {
                entry.measured = None;
                entry.pending_measurement = None;
            }
        }
    }

    pub(super) fn invalidate_session(&mut self, session: &str) {
        if let Some(state) = self.sessions.get_mut(session) {
            for entry in state.entries.values_mut() {
                entry.measured = None;
                entry.pending_measurement = None;
            }
        }
    }

    pub(super) fn remove_session(&mut self, session: &str) {
        self.sessions.remove(session);
    }

    #[cfg(test)]
    pub(super) fn measured_height(&self, session: &str, id: &str) -> Option<Pixels> {
        self.sessions.get(session)?.entries.get(id)?.measured.map(|size| size.height)
    }

    #[cfg(test)]
    pub(super) fn render_count(&self, session: &str, id: &str) -> Option<u64> {
        self.sessions
            .get(session)?
            .entries
            .get(id)
            .map(|entry| entry.renders.get())
    }
}
