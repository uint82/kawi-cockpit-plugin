use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::cargo::{matches_path, Diagnostic};
use crate::joblog::JobLog;
use crate::overlay::OverlayView;
use crate::settings::Settings;
use crate::tasks::Task;

#[derive(Debug)]
pub struct State {
    pub settings: Settings,
    pub buffers: BTreeMap<u64, String>,
    pub diagnostics: BTreeMap<u64, Vec<Diagnostic>>,
    pub notes: usize,
    pub workspace: Option<PathBuf>,
    pub tasks: Vec<Task>,
    pub job: JobLog,
    pub panel: Option<u64>,
    pub overlay: Option<OverlayView>,
    pub overlay_id: Option<u64>,
}

impl State {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            buffers: BTreeMap::new(),
            diagnostics: BTreeMap::new(),
            notes: 0,
            workspace: None,
            tasks: Vec::new(),
            job: JobLog::default(),
            panel: None,
            overlay: None,
            overlay_id: None,
        }
    }

    pub fn track_buffer(&mut self, id: u64, path: String) {
        self.buffers.insert(id, path);
    }

    pub fn untrack_buffer(&mut self, id: u64) {
        self.buffers.remove(&id);
        self.diagnostics.remove(&id);
    }

    pub fn set_diagnostics(&mut self, id: u64, diagnostics: Vec<Diagnostic>) -> bool {
        let changed = self.diagnostics.get(&id) != Some(&diagnostics);
        if changed {
            self.diagnostics.insert(id, diagnostics);
        }
        changed
    }

    pub fn all_diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.values().flatten().cloned().collect()
    }

    pub fn active_diagnostics(&self, id: Option<u64>) -> Vec<Diagnostic> {
        id.and_then(|id| self.diagnostics.get(&id).cloned())
            .unwrap_or_default()
    }

    pub fn buffer_for_path(&self, path: &str) -> Option<u64> {
        self.buffers
            .iter()
            .find(|(_, buffer_path)| matches_path(path, buffer_path))
            .map(|(id, _)| *id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo::Severity;

    fn state() -> State {
        let mut state = State::new(Settings::default());
        state.track_buffer(1, "/work/cockpit/src/main.rs".to_string());
        state.track_buffer(2, "/work/cockpit/src/lib.rs".to_string());
        state
    }

    fn diagnostic(path: &str) -> Diagnostic {
        Diagnostic {
            path: path.to_string(),
            line: 1,
            column: 1,
            severity: Severity::Error,
            message: "bad".to_string(),
        }
    }

    #[test]
    fn untracking_a_buffer_drops_its_diagnostics() {
        let mut state = state();
        state.set_diagnostics(1, vec![diagnostic("src/main.rs")]);
        state.untrack_buffer(1);
        assert!(state.diagnostics.is_empty());
        assert_eq!(state.buffer_for_path("src/main.rs"), None);
    }

    #[test]
    fn reports_whether_diagnostics_changed() {
        let mut state = state();
        assert!(state.set_diagnostics(1, vec![diagnostic("src/main.rs")]));
        assert!(!state.set_diagnostics(1, vec![diagnostic("src/main.rs")]));
        assert!(state.set_diagnostics(1, Vec::new()));
    }

    #[test]
    fn resolves_buffers_by_relative_or_absolute_path() {
        let state = state();
        assert_eq!(state.buffer_for_path("src/main.rs"), Some(1));
        assert_eq!(state.buffer_for_path("/work/cockpit/src/lib.rs"), Some(2));
        assert_eq!(state.buffer_for_path("src/other.rs"), None);
    }

    #[test]
    fn active_diagnostics_default_to_empty() {
        let state = state();
        assert!(state.active_diagnostics(None).is_empty());
        assert!(state.active_diagnostics(Some(99)).is_empty());
    }
}
