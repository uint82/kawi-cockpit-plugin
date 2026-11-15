use kawi_plugin_sdk::types::{Argument, Error};
use kawi_plugin_sdk::{ui_message, ui_picker, ui_prompt};

use crate::commands::overlay;
use crate::error;
use crate::panel;
use crate::project;
use crate::state::State;
use crate::tasks::{self, Task};
use crate::worker::Worker;

pub fn run(state: &mut State, worker: &mut Worker) -> Result<Argument, Error> {
    let root = project::root(state)?;
    let tasks = project::tasks(&root);
    if tasks.is_empty() {
        return Err(error::invalid("no tasks are defined"));
    }
    let Some(selected) = pick(&tasks)? else {
        return Ok(Argument::Null);
    };
    let Some(task) = tasks::find(&tasks, &selected).cloned() else {
        return Ok(Argument::Null);
    };
    worker.start(&task, &root, state)?;
    panel::refresh(state)?;
    ui_message::message(&format!("cockpit: running {}", task.name))?;
    Ok(Argument::Null)
}

pub(super) fn items(tasks: &[Task]) -> Vec<ui_picker::PickerItem> {
    tasks
        .iter()
        .map(|task| ui_picker::PickerItem {
            id: task.name.clone(),
            label: task.name.clone(),
            detail: Some(task.command_line()),
            deprecated: false,
        })
        .collect()
}

fn pick(tasks: &[Task]) -> Result<Option<String>, Error> {
    match ui_picker::pick(&ui_picker::PickerParams {
        title: "Run project task".to_string(),
        items: items(tasks),
        selected: None,
    })? {
        ui_picker::PickerOutcome::Selected(name) => Ok(Some(name)),
        ui_picker::PickerOutcome::Canceled => Ok(None),
    }
}

pub fn stop(state: &mut State, worker: &mut Worker) -> Result<Argument, Error> {
    if !worker.is_running() {
        ui_message::message("cockpit: no task is running")?;
        return Ok(Argument::Null);
    }
    match ui_prompt::confirm(
        "Stop the running task?",
        Some("The task will receive a terminate signal."),
    )? {
        ui_prompt::PromptOutcome::Confirmed(true) => {
            if worker.stop(state)? {
                ui_message::message("cockpit: stopping the task")?;
            } else {
                ui_message::message("cockpit: no task is running")?;
            }
        }
        _ => {
            ui_message::message("cockpit: stop canceled")?;
        }
    }
    Ok(Argument::Null)
}

pub fn log(state: &mut State) -> Result<Argument, Error> {
    overlay::show_log(state)
}
