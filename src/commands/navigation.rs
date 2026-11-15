use kawi_plugin_sdk::types::{Argument, Error, Position, SplitDirection};
use kawi_plugin_sdk::{
    editor_buffers, editor_buffers_write, editor_control, editor_execute, editor_read,
    editor_windows, options_read, options_write, ui_message, ui_picker, ui_prompt,
};

use crate::cargo::Diagnostic;
use crate::error;
use crate::format;
use crate::state::State;

pub fn buffers() -> Result<Argument, Error> {
    let buffers = editor_buffers::list_buffers()?;
    let active = editor_read::get_state(false)?.buffer;
    let items: Vec<_> = buffers
        .iter()
        .filter_map(|buffer| {
            let path = buffer.path.as_deref()?;
            let marker = if buffer.buffer == active { ">" } else { " " };
            Some(ui_picker::PickerItem {
                id: buffer.buffer.to_string(),
                label: format!("{marker}{}", format::file_name(path)),
                detail: Some(path.to_string()),
                deprecated: false,
            })
        })
        .collect();
    if items.is_empty() {
        ui_message::message("cockpit: no named buffers")?;
        return Ok(Argument::Null);
    }
    match ui_picker::pick(&ui_picker::PickerParams {
        title: "Switch buffer".to_string(),
        items,
        selected: None,
    })? {
        ui_picker::PickerOutcome::Selected(id) => {
            let buffer = buffers
                .iter()
                .find(|buffer| buffer.buffer.to_string() == id)
                .ok_or_else(|| error::invalid("buffer disappeared"))?;
            let path = buffer
                .path
                .clone()
                .ok_or_else(|| error::invalid("buffer has no path"))?;
            editor_buffers_write::open(&path)?;
        }
        ui_picker::PickerOutcome::Canceled => {}
    }
    Ok(Argument::Null)
}

pub fn windows() -> Result<Argument, Error> {
    let windows = editor_windows::list_windows()?;
    let items: Vec<_> = windows
        .iter()
        .map(|window| ui_picker::PickerItem {
            id: window.window.to_string(),
            label: format!(
                "{}{} {}x{}",
                if window.active { ">" } else { " " },
                window.window,
                window.width,
                window.height
            ),
            detail: Some(format!("buffer {}", window.buffer)),
            deprecated: false,
        })
        .collect();
    if items.is_empty() {
        ui_message::message("cockpit: no windows")?;
        return Ok(Argument::Null);
    }
    if let ui_picker::PickerOutcome::Selected(id) = ui_picker::pick(&ui_picker::PickerParams {
        title: "Focus window".to_string(),
        items,
        selected: None,
    })? {
        let window = id
            .parse()
            .map_err(|_| error::invalid("invalid window id"))?;
        editor_windows::focus(window)?;
    }
    Ok(Argument::Null)
}

pub fn split() -> Result<Argument, Error> {
    let state = editor_read::get_state(false)?;
    editor_windows::open(state.buffer, SplitDirection::Vertical)?;
    Ok(Argument::Null)
}

pub fn close_window() -> Result<Argument, Error> {
    let windows = editor_windows::list_windows()?;
    let Some(active) = windows.iter().find(|window| window.active) else {
        return Err(error::invalid("no active window"));
    };
    if windows.len() == 1 {
        return Err(error::invalid("cannot close the last window"));
    }
    if let ui_prompt::PromptOutcome::Confirmed(true) =
        ui_prompt::confirm("Close the active window?", None)?
    {
        editor_windows::close(active.window, false)?;
    }
    Ok(Argument::Null)
}

pub fn mode() -> Result<Argument, Error> {
    let state = editor_read::get_state(false)?;
    let value = match ui_prompt::input("Set mode", Some("normal"), Some(state.mode.as_str()), None)?
    {
        ui_prompt::PromptOutcome::Text(value) => value,
        _ => return Ok(Argument::Null),
    };
    if value.trim().is_empty() {
        return Ok(Argument::Null);
    }
    editor_control::set_mode(state.buffer, value.trim())?;
    Ok(Argument::Null)
}

pub fn next_diagnostic(state: &State) -> Result<Argument, Error> {
    let editor = editor_read::get_state(false)?;
    let mut diagnostics = state.active_diagnostics(Some(editor.buffer));
    if diagnostics.is_empty() {
        ui_message::message("cockpit: no diagnostics for this buffer")?;
        return Ok(Argument::Null);
    }
    sort_diagnostics(&mut diagnostics);
    let next = next_after(&diagnostics, editor.cursor.line)
        .ok_or_else(|| error::invalid("no diagnostics"))?;
    editor_control::set_cursor(
        editor.buffer,
        Position {
            line: next.line.saturating_sub(1),
            byte_column: next.column.saturating_sub(1),
        },
    )?;
    Ok(Argument::Null)
}

pub fn toggle_numbers() -> Result<Argument, Error> {
    let option = options_read::get("number")?;
    let option = options_write::set("number", &Argument::Boolean(!option_bool(&option.value)?))?;
    ui_message::message(&format!("cockpit: number {}", option.value))?;
    Ok(Argument::Null)
}

pub fn save() -> Result<Argument, Error> {
    editor_execute::execute("file.save")?;
    ui_message::message("cockpit: buffer saved")?;
    Ok(Argument::Null)
}

fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by_key(|diagnostic| (diagnostic.line, diagnostic.column));
}

fn next_after(diagnostics: &[Diagnostic], cursor_line: u32) -> Option<&Diagnostic> {
    let current = cursor_line.saturating_add(1);
    diagnostics
        .iter()
        .find(|diagnostic| diagnostic.line > current)
        .or_else(|| diagnostics.first())
}

fn option_bool(value: &str) -> Result<bool, Error> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(error::invalid(format!("unexpected number value {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo::Severity;

    fn diagnostic(line: u32, column: u32) -> Diagnostic {
        Diagnostic {
            path: "src/main.rs".to_string(),
            line,
            column,
            severity: Severity::Error,
            message: "bad".to_string(),
        }
    }

    #[test]
    fn sorts_diagnostics_by_position() {
        let mut diagnostics = vec![diagnostic(4, 2), diagnostic(1, 9), diagnostic(1, 3)];
        sort_diagnostics(&mut diagnostics);
        let positions: Vec<_> = diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.line, diagnostic.column))
            .collect();
        assert_eq!(positions, [(1, 3), (1, 9), (4, 2)]);
    }

    #[test]
    fn moves_to_the_next_diagnostic_and_wraps() {
        let diagnostics = vec![diagnostic(1, 1), diagnostic(3, 1), diagnostic(5, 1)];
        assert_eq!(next_after(&diagnostics, 0).map(|d| d.line), Some(3));
        assert_eq!(next_after(&diagnostics, 2).map(|d| d.line), Some(5));
        assert_eq!(next_after(&diagnostics, 4).map(|d| d.line), Some(1));
    }

    #[test]
    fn returns_none_for_empty_diagnostics() {
        assert!(next_after(&[], 0).is_none());
    }

    #[test]
    fn parses_boolean_option_values() {
        assert!(option_bool("true").expect("true"));
        assert!(!option_bool("false").expect("false"));
        assert!(option_bool("maybe").is_err());
    }
}
