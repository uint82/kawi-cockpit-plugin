use kawi_plugin_sdk::editor_buffers::BufferInfo;
use kawi_plugin_sdk::types::{DecorationStyle, Error};
use kawi_plugin_sdk::{
    editor_buffers, editor_read, editor_windows, options_read, ui_panel, ui_statusline,
    PANEL_NOT_FOUND,
};

use crate::cargo::{self, Severity};
use crate::dashboard::Dashboard;
use crate::format;
use crate::project;
use crate::state::State;
use crate::workspace;

const NO_LANGUAGE: &str = "text";
const STATUSLINE_PRIORITY: u32 = 20;
const NO_NAME: &str = "[No Name]";

struct PanelContent {
    title: String,
    status: String,
    lines: Vec<ui_panel::PanelLine>,
}

impl PanelContent {
    fn new(dashboard: &Dashboard, height: usize) -> Self {
        Self {
            title: dashboard.title(),
            status: dashboard.status(),
            lines: dashboard.lines(height),
        }
    }
}

pub fn refresh(state: &mut State) -> Result<(), Error> {
    let dashboard = match dashboard(state)? {
        Some(dashboard) => dashboard,
        None => {
            close(state)?;
            return ui_statusline::set(&[]);
        }
    };
    let content = PanelContent::new(&dashboard, state.settings.panel_height as usize);
    if let Some(panel) = state.panel {
        match update(panel, &content) {
            Ok(()) => {}
            Err(error) if error.code == PANEL_NOT_FOUND => {
                state.panel = None;
                open_panel(state, content)?;
            }
            Err(error) => return Err(error),
        }
    } else {
        open_panel(state, content)?;
    }
    set_statusline(state, &dashboard)
}

pub fn dashboard(state: &mut State) -> Result<Option<Dashboard>, Error> {
    let editor = editor_read::get_state(false)?;
    let active_path = editor.path.clone();
    if let Some(path) = active_path.as_deref() {
        state.track_buffer(editor.buffer, path.to_string());
    }
    state.workspace = workspace::detect(active_path.as_deref());
    let Some(root) = state.workspace.clone() else {
        return Ok(None);
    };
    state.tasks = project::tasks(&root);
    state.notes = project::notes_count(&root);
    let buffers = editor_buffers::list_buffers()?;
    for buffer in &buffers {
        if let Some(path) = &buffer.path {
            state.track_buffer(buffer.buffer, path.clone());
        }
    }
    let windows = editor_windows::list_windows()?;
    let buffer_labels = buffer_labels(&buffers, editor.buffer);
    let window_lines = windows
        .iter()
        .map(|window| {
            let label = buffer_labels
                .get(&window.buffer)
                .cloned()
                .unwrap_or_else(|| format!("buffer {}", window.buffer));
            format!(
                "{}{} {}x{} {}",
                if window.active { ">" } else { " " },
                window.window,
                window.width,
                window.height,
                label
            )
        })
        .collect();
    let dashboard = Dashboard {
        root: root.clone(),
        active_path,
        language: editor.language.unwrap_or_else(|| NO_LANGUAGE.to_string()),
        mode: editor.mode,
        cursor: format!(
            "{}:{}",
            editor.cursor.line.saturating_add(1),
            editor.cursor.byte_column.saturating_add(1)
        ),
        buffers: buffer_labels.values().cloned().collect(),
        buffer_count: buffers.len(),
        max_buffers: state.settings.max_buffers,
        windows: window_lines,
        window_count: windows.len(),
        diagnostics: state.all_diagnostics(),
        max_diagnostics: state.settings.max_diagnostics,
        notes: state.notes,
        options: options_summary(),
        job: state.job.summary(),
        tasks: state.tasks.len(),
    };
    Ok(Some(dashboard))
}

fn open_panel(state: &mut State, content: PanelContent) -> Result<(), Error> {
    let result = ui_panel::open(&ui_panel::OpenParams {
        title: content.title,
        status: Some(content.status),
        height: Some(state.settings.panel_height),
        focus: false,
        lines: content.lines,
    })?;
    state.panel = Some(result.panel);
    Ok(())
}

fn update(panel: u64, content: &PanelContent) -> Result<(), Error> {
    ui_panel::update(&ui_panel::UpdateParams {
        panel,
        title: Some(content.title.clone()),
        status: Some(content.status.clone()),
        height: None,
        lines: Some(content.lines.clone()),
    })
    .map(|_| ())
}

pub fn close(state: &mut State) -> Result<(), Error> {
    if let Some(panel) = state.panel.take() {
        ui_panel::close(panel)?;
    }
    Ok(())
}

pub fn focus(state: &mut State) -> Result<(), Error> {
    let panel = state
        .panel
        .ok_or_else(|| crate::error::invalid("cockpit panel is not open"))?;
    ui_panel::focus(panel).map(|_| ())
}

fn buffer_labels(buffers: &[BufferInfo], active: u64) -> std::collections::BTreeMap<u64, String> {
    buffers
        .iter()
        .map(|buffer| {
            let name = buffer
                .path
                .as_deref()
                .map(format::file_name)
                .unwrap_or_else(|| NO_NAME.to_string());
            let marker = if buffer.buffer == active { ">" } else { " " };
            let modified = if buffer.modified { "*" } else { "" };
            (buffer.buffer, format!("{marker}{name}{modified}"))
        })
        .collect()
}

fn options_summary() -> String {
    let number = option_value("number");
    let wrap = option_value("wrap");
    format!("number {number} wrap {wrap}")
}

fn option_value(name: &str) -> String {
    match options_read::get(name) {
        Ok(option) => option.value,
        Err(read_error) => {
            crate::error::log(&format!("cannot read option {name}"), &read_error);
            "?".to_string()
        }
    }
}

fn set_statusline(state: &State, dashboard: &Dashboard) -> Result<(), Error> {
    if !state.settings.show_statusline {
        return ui_statusline::set(&[]);
    }
    let errors = cargo::count(&dashboard.diagnostics, Severity::Error);
    let warnings = cargo::count(&dashboard.diagnostics, Severity::Warning);
    let style = if state.job.is_running() {
        DecorationStyle::Accent
    } else {
        DecorationStyle::Muted
    };
    ui_statusline::set(&[ui_statusline::StatuslineSegment {
        text: format!(" {}  {errors}E {warnings}W ", dashboard.title()),
        style,
        priority: STATUSLINE_PRIORITY,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_modified_and_active_buffers() {
        let buffers = vec![
            BufferInfo {
                buffer: 1,
                revision: 0,
                path: Some("/work/src/main.rs".to_string()),
                language: Some("Rust".to_string()),
                modified: true,
            },
            BufferInfo {
                buffer: 2,
                revision: 0,
                path: None,
                language: None,
                modified: false,
            },
        ];
        let labels = buffer_labels(&buffers, 1);
        assert_eq!(labels.get(&1).map(String::as_str), Some(">main.rs*"));
        assert_eq!(labels.get(&2).map(String::as_str), Some(" [No Name]"));
    }
}
