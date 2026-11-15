use std::path::PathBuf;

use kawi_plugin_sdk::types::DecorationStyle;
use kawi_plugin_sdk::ui_panel::PanelLine;

use crate::cargo::{self, Diagnostic, Severity};
use crate::format;

const SECTION_STYLE: DecorationStyle = DecorationStyle::Muted;

struct PanelLines {
    lines: Vec<PanelLine>,
    maximum: usize,
}

impl PanelLines {
    fn new(maximum: usize) -> Self {
        Self {
            lines: Vec::with_capacity(maximum),
            maximum,
        }
    }

    fn push(&mut self, text: String, style: Option<DecorationStyle>) {
        if self.lines.len() < self.maximum {
            self.lines.push(PanelLine { text, style });
        }
    }

    fn into_lines(self) -> Vec<PanelLine> {
        self.lines
    }
}

#[derive(Clone, Debug)]
pub struct Dashboard {
    pub root: PathBuf,
    pub active_path: Option<String>,
    pub language: String,
    pub mode: String,
    pub cursor: String,
    pub buffers: Vec<String>,
    pub buffer_count: usize,
    pub max_buffers: usize,
    pub windows: Vec<String>,
    pub window_count: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub max_diagnostics: usize,
    pub notes: usize,
    pub options: String,
    pub job: Option<String>,
    pub tasks: usize,
}

impl Dashboard {
    pub fn title(&self) -> String {
        format::root_name(&self.root)
    }

    pub fn status(&self) -> String {
        let diagnostics = self.diagnostics.len();
        let diagnostic_label = format::count_label(diagnostics, "diagnostic");
        match &self.job {
            Some(job) => format!("{job} | {diagnostic_label}"),
            None => format!(
                "{} | {diagnostic_label} | {}",
                format::count_label(self.buffer_count, "buffer"),
                format::count_label(self.tasks, "task")
            ),
        }
    }

    pub fn lines(&self, maximum: usize) -> Vec<PanelLine> {
        let mut lines = PanelLines::new(maximum);
        lines.push(
            format!("{}  [{}]", self.title(), self.mode),
            Some(DecorationStyle::Accent),
        );
        lines.push(format!("File  {}", self.active_label()), None);
        lines.push(
            format!("Lang  {}   Cursor {}", self.language, self.cursor),
            Some(DecorationStyle::Muted),
        );
        lines.push(
            format!(
                "Buffers ({}/{})",
                self.buffers.len().min(self.buffer_count),
                self.buffer_count
            ),
            Some(SECTION_STYLE),
        );
        for buffer in self.buffers.iter().take(self.max_buffers) {
            lines.push(format!("  {buffer}"), None);
        }
        lines.push(
            format!("Windows ({})", self.window_count),
            Some(SECTION_STYLE),
        );
        for window in &self.windows {
            lines.push(format!("  {window}"), None);
        }
        self.push_diagnostics(&mut lines);
        if let Some(job) = &self.job {
            lines.push("Job".to_string(), Some(SECTION_STYLE));
            lines.push(format!("  {job}"), None);
        }
        lines.push(
            format!("Notes {}   {}", self.notes, self.options),
            Some(DecorationStyle::Muted),
        );
        lines.into_lines()
    }

    fn push_diagnostics(&self, lines: &mut PanelLines) {
        let active = self
            .active_path
            .as_deref()
            .map(|path| cargo::for_path(&self.diagnostics, path));
        let shown = active.as_deref().unwrap_or(&[]);
        let errors = cargo::count(&self.diagnostics, Severity::Error);
        let warnings = cargo::count(&self.diagnostics, Severity::Warning);
        lines.push(
            format!("Diagnostics ({errors}E {warnings}W)"),
            Some(SECTION_STYLE),
        );
        if shown.is_empty() {
            lines.push(
                "  No diagnostics for this buffer".to_string(),
                Some(DecorationStyle::Muted),
            );
            return;
        }
        for diagnostic in shown.iter().take(self.max_diagnostics) {
            lines.push(
                format!("  {}", diagnostic.render(&self.root)),
                Some(style_for(diagnostic.severity)),
            );
        }
    }

    fn active_label(&self) -> String {
        match &self.active_path {
            Some(path) => format::display_path(&self.root, path),
            None => "[No Name]".to_string(),
        }
    }
}

fn style_for(severity: Severity) -> DecorationStyle {
    match severity {
        Severity::Error => DecorationStyle::Error,
        Severity::Warning => DecorationStyle::Warning,
        Severity::Information => DecorationStyle::Info,
        Severity::Hint => DecorationStyle::Muted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo::parse;

    fn sample() -> Dashboard {
        Dashboard {
            root: PathBuf::from("/work/cockpit"),
            active_path: Some("/work/cockpit/src/main.rs".to_string()),
            language: "Rust".to_string(),
            mode: "normal".to_string(),
            cursor: "2:4".to_string(),
            buffers: vec!["main.rs".to_string(), "lib.rs".to_string()],
            buffer_count: 3,
            max_buffers: 8,
            windows: vec!["1 active 80x24 main.rs".to_string()],
            window_count: 1,
            diagnostics: parse("src/main.rs:2:4: error: bad\nsrc/lib.rs:1:1: warning: unused\n"),
            max_diagnostics: 6,
            notes: 5,
            options: "number yes".to_string(),
            job: Some("running test".to_string()),
            tasks: 6,
        }
    }

    #[test]
    fn renders_the_header_and_active_file() {
        let dashboard = sample();
        let text: Vec<_> = dashboard
            .lines(20)
            .into_iter()
            .map(|line| line.text)
            .collect();
        assert_eq!(text[0], "cockpit  [normal]");
        assert_eq!(text[1], "File  src/main.rs");
        assert!(text.iter().any(|line| line.contains("1E 1W")));
        assert!(text.iter().any(|line| line.contains("main.rs:2:4 E bad")));
        assert!(!text.iter().any(|line| line.contains("lib.rs:1:1 W")));
    }

    #[test]
    fn clips_to_the_maximum_height() {
        let dashboard = sample();
        assert!(dashboard.lines(5).len() <= 5);
        assert!(dashboard.lines(0).is_empty());
    }

    #[test]
    fn summarizes_buffers_and_job() {
        let dashboard = sample();
        assert_eq!(dashboard.status(), "running test | 2 diagnostics");
        let text: Vec<_> = dashboard
            .lines(40)
            .into_iter()
            .map(|line| line.text)
            .collect();
        assert!(text.iter().any(|line| line == "Buffers (2/3)"));
        assert!(text.iter().any(|line| line == "Job"));
    }

    #[test]
    fn uses_singular_counts_without_a_job() {
        let mut dashboard = sample();
        dashboard.job = None;
        dashboard.buffers = vec!["main.rs".to_string()];
        dashboard.buffer_count = 1;
        dashboard.diagnostics = parse("src/main.rs:2:4: error: bad\n");
        dashboard.tasks = 1;
        assert_eq!(dashboard.status(), "1 buffer | 1 diagnostic | 1 task");
    }

    #[test]
    fn reports_no_active_diagnostics() {
        let mut dashboard = sample();
        dashboard.active_path = Some("/work/cockpit/src/other.rs".to_string());
        let text: Vec<_> = dashboard
            .lines(40)
            .into_iter()
            .map(|line| line.text)
            .collect();
        assert!(text
            .iter()
            .any(|line| line.contains("No diagnostics for this buffer")));
    }
}
