use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use kawi_plugin_sdk::types::{Argument, Edit, Error};
use kawi_plugin_sdk::{
    editor_buffers_write, editor_read, editor_write, ui_message, ui_picker, ui_progress, ui_prompt,
};

use crate::error;
use crate::notes;
use crate::panel;
use crate::project;
use crate::report::{self, ReportData};
use crate::state::State;
use crate::tasks;
use crate::workspace;

const NOTE_MAX_LENGTH: u32 = 500;
const REPORT_TITLE: &str = "Cockpit report";

pub fn note(state: &mut State) -> Result<Argument, Error> {
    let root = project::root(state)?;
    let path = workspace::notes_path(&root);
    let existing = project::read_optional(&path)?.unwrap_or_default();
    let text = match ui_prompt::input(
        "Project note",
        Some("What should be remembered?"),
        None,
        Some(NOTE_MAX_LENGTH),
    )? {
        ui_prompt::PromptOutcome::Text(value) => value,
        _ => return Ok(Argument::Null),
    };
    if text.trim().is_empty() {
        return Ok(Argument::Null);
    }
    let contents = notes::with_entry(&existing, &timestamp(), text.trim());
    project::ensure_dir(&root)?;
    project::write(&path, &contents)?;
    editor_buffers_write::open(&workspace::as_string(&path))?;
    panel::refresh(state)?;
    ui_message::message("cockpit: note appended")?;
    Ok(Argument::Null)
}

pub fn insert_task(state: &mut State) -> Result<Argument, Error> {
    let root = project::root(state)?;
    let tasks = project::tasks(&root);
    if tasks.is_empty() {
        return Err(error::invalid("no tasks are defined"));
    }
    let selected = match ui_picker::pick(&ui_picker::PickerParams {
        title: "Insert task".to_string(),
        items: super::tasks::items(&tasks),
        selected: None,
    })? {
        ui_picker::PickerOutcome::Selected(name) => name,
        ui_picker::PickerOutcome::Canceled => return Ok(Argument::Null),
    };
    let Some(task) = tasks::find(&tasks, &selected) else {
        return Ok(Argument::Null);
    };
    let editor = editor_read::get_state(false)?;
    editor_write::apply_edit(
        editor.buffer,
        editor.revision,
        &[Edit {
            start: editor.cursor,
            end: editor.cursor,
            text: format!("{}\n", task.command_line()),
        }],
    )?;
    Ok(Argument::Null)
}

pub fn report(state: &mut State) -> Result<Argument, Error> {
    let root = project::root(state)?;
    let Some(dashboard) = panel::dashboard(state)? else {
        return Err(error::invalid("no active project"));
    };
    let progress = ui_progress::start(REPORT_TITLE, Some("Writing report"), None)?;
    let result = write_report(&root, &dashboard);
    ui_progress::finish(progress)?;
    let path = result?;
    editor_buffers_write::open(&workspace::as_string(&path))?;
    ui_message::message("cockpit: report written")?;
    Ok(Argument::Null)
}

fn write_report(
    root: &Path,
    dashboard: &crate::dashboard::Dashboard,
) -> Result<std::path::PathBuf, Error> {
    let path = workspace::report_path(root);
    let data = ReportData::from(dashboard);
    project::ensure_dir(root)?;
    project::write(&path, &report::render(&data))?;
    Ok(path)
}

fn timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("note {seconds}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_deterministic_in_shape() {
        let stamp = timestamp();
        assert!(stamp.starts_with("note "));
        assert!(stamp[5..]
            .chars()
            .all(|character| character.is_ascii_digit()));
    }
}
