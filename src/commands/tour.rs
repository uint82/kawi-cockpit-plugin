use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use kawi_plugin_sdk::editor_autocmds::{self, AutocmdSpec};
use kawi_plugin_sdk::editor_commands::{self, CommandSpec};
use kawi_plugin_sdk::editor_keymaps::{self, KeyMode, KeymapSpec};
use kawi_plugin_sdk::editor_variables::{self, VariableTarget};
use kawi_plugin_sdk::process_jobs::{self, JobSignal, JobState, OpenParams, StdinMode};
use kawi_plugin_sdk::types::{Argument, Diagnostic, Error, Position, Severity, VariableScope};
use kawi_plugin_sdk::{
    diagnostics_publish, editor_buffers, editor_control, editor_execute, editor_read,
    editor_schedule, editor_windows, options_read, options_write, process_run, ui_message,
};

use crate::error;
use crate::format;
use crate::register_refresh;
use crate::state::State;

const PROBE_COMMAND: &str = "tour-probe";
const PROBE_KEYS: &str = "<leader>cX";
const PROBE_USER_EVENT: &str = "CockpitTourProbe";
const TOUR_TIMER_MS: u32 = 60_000;
const TOUR_NETWORK: &str = "example.com:80";
const PROCESS_TIMEOUT_MS: u32 = 5_000;
const POLL_DELAY_MS: u64 = 20;
const POLL_ATTEMPTS: usize = 20;
const NETWORK_TIMEOUT: Duration = Duration::from_secs(2);

pub fn run(state: &State, tour_name: &str) -> Result<Argument, Error> {
    let editor = editor_read::get_state(false)?;
    exercise_commands(tour_name)?;
    exercise_keymaps(tour_name)?;
    exercise_autocmds()?;
    exercise_variables()?;
    exercise_options()?;
    exercise_timers()?;
    exercise_editor(&editor)?;
    exercise_diagnostics(&editor)?;
    let process = exercise_processes()?;
    let network = probe_network();
    let project = state
        .workspace
        .as_ref()
        .map(|root| format::root_name(root))
        .unwrap_or_else(|| "no project".to_string());
    ui_message::message(&format!(
        "cockpit tour complete: {project}, {process}, net {network}"
    ))?;
    Ok(Argument::Text("tour complete".to_string()))
}

fn exercise_commands(tour_name: &str) -> Result<(), Error> {
    let registered = editor_commands::register(&CommandSpec {
        id: PROBE_COMMAND.to_string(),
        title: "Cockpit tour probe".to_string(),
        description: Some("Temporary command used by the API tour".to_string()),
        aliases: Vec::new(),
    })?;
    let commands = editor_commands::list_registered()?;
    if !commands.iter().any(|command| command.id == PROBE_COMMAND) {
        return Err(error::internal("temporary command is missing"));
    }
    if !commands.iter().any(|command| command.id == tour_name) {
        return Err(error::internal("tour command is missing"));
    }
    editor_commands::unregister(&registered.id)
}

fn exercise_keymaps(tour_name: &str) -> Result<(), Error> {
    editor_keymaps::set(&KeymapSpec {
        mode: KeyMode::Normal,
        keys: PROBE_KEYS.to_string(),
        command: tour_name.to_string(),
        description: Some("Cockpit tour probe".to_string()),
    })?;
    if !editor_keymaps::list_registered()?
        .iter()
        .any(|keymap| keymap.keys == PROBE_KEYS)
    {
        return Err(error::internal("temporary keymap is missing"));
    }
    editor_keymaps::remove(KeyMode::Normal, PROBE_KEYS)
}

fn exercise_autocmds() -> Result<(), Error> {
    let registered = editor_autocmds::register(&AutocmdSpec {
        event: "user".to_string(),
        pattern: Some(PROBE_USER_EVENT.to_string()),
        buffer: None,
        once: true,
    })?;
    if !editor_autocmds::list_registered()?
        .iter()
        .any(|autocmd| autocmd.id == registered.id)
    {
        return Err(error::internal("temporary autocmd is missing"));
    }
    editor_autocmds::emit_user_event(PROBE_USER_EVENT, Some("tour"))?;
    editor_autocmds::remove(registered.id)?;
    // This wipes every autocmd, including ours. That is the point: the tour
    // exercises clear(), then puts the refresh handler back.
    editor_autocmds::clear()?;
    register_refresh()
}

fn exercise_variables() -> Result<(), Error> {
    let target = plugin_target();
    editor_variables::set(target, "tour", &Argument::Text("yes".to_string()))?;
    if !matches!(
        editor_variables::get(target, "tour")?,
        Some(Argument::Text(value)) if value == "yes"
    ) {
        return Err(error::internal("variable did not round trip"));
    }
    if editor_variables::list_variables(target)?.is_empty() {
        return Err(error::internal("variable list is empty"));
    }
    if !editor_variables::delete(target, "tour")? {
        return Err(error::internal("variable was not deleted"));
    }
    Ok(())
}

fn exercise_options() -> Result<(), Error> {
    let original = options_read::get("number")?.value;
    if options_read::list_options(Some("number"))?.is_empty() {
        return Err(error::internal("option list is empty"));
    }
    options_write::set("number", &Argument::Boolean(true))?;
    options_write::reset("number")?;
    options_write::set(
        "number",
        &Argument::Boolean(original.eq_ignore_ascii_case("true")),
    )?;
    Ok(())
}

fn exercise_timers() -> Result<(), Error> {
    let timer = editor_schedule::after(TOUR_TIMER_MS, Some("tour"))?;
    if !editor_schedule::list_timers()?
        .iter()
        .any(|entry| entry.id == timer)
    {
        return Err(error::internal("tour timer is missing"));
    }
    if !editor_schedule::cancel(timer)? {
        return Err(error::internal("tour timer was not canceled"));
    }
    Ok(())
}

fn exercise_editor(editor: &kawi_plugin_sdk::types::State) -> Result<(), Error> {
    editor_buffers::list_buffers()?;
    editor_buffers::get(editor.buffer, false)?;
    editor_execute::list_commands()?;
    let windows = editor_windows::list_windows()?;
    if let Some(window) = windows.first() {
        editor_windows::get(window.window)?;
    }
    editor_control::set_cursor(editor.buffer, editor.cursor)?;
    editor_control::set_mode(editor.buffer, &editor.mode)?;
    Ok(())
}

fn exercise_diagnostics(editor: &kawi_plugin_sdk::types::State) -> Result<(), Error> {
    let position = editor.cursor;
    let diagnostic = Diagnostic {
        start: position,
        end: Position {
            line: position.line,
            byte_column: position.byte_column.saturating_add(1),
        },
        severity: Severity::Warning,
        message: "cockpit tour probe".to_string(),
        source: Some("cockpit".to_string()),
        code: Some("TOUR".to_string()),
    };
    diagnostics_publish::set(
        "cockpit-tour",
        editor.buffer,
        editor.revision,
        &[diagnostic],
    )?;
    diagnostics_publish::set("cockpit-tour", editor.buffer, editor.revision, &[])
}

fn exercise_processes() -> Result<String, Error> {
    let output = process_run::run("git", &["--version".to_string()], PROCESS_TIMEOUT_MS)?;
    if !output.success {
        return Err(error::internal(format!(
            "git --version failed: {}",
            output.stderr.trim()
        )));
    }
    let job = process_jobs::open(&OpenParams {
        command: "cat".to_string(),
        args: Vec::new(),
        cwd: None,
        env: Vec::new(),
        stdin: StdinMode::Pipe,
    })?;
    process_jobs::write(job, "tour\n")?;
    process_jobs::close_stdin(job)?;
    let mut finished = false;
    for _ in 0..POLL_ATTEMPTS {
        process_jobs::poll(job)?;
        if process_jobs::status(job)?.state != JobState::Running {
            finished = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(POLL_DELAY_MS));
    }
    if !finished && process_jobs::status(job)?.state == JobState::Running {
        process_jobs::kill(job)?;
    }
    let signal_job = open_cat()?;
    process_jobs::signal(signal_job, JobSignal::Terminate)?;
    let kill_job = open_cat()?;
    process_jobs::kill(kill_job)?;
    let version = output.stdout.lines().next().unwrap_or("git");
    Ok(version.trim().to_string())
}

fn open_cat() -> Result<u64, Error> {
    process_jobs::open(&OpenParams {
        command: "cat".to_string(),
        args: Vec::new(),
        cwd: None,
        env: Vec::new(),
        stdin: StdinMode::Null,
    })
}

fn probe_network() -> String {
    match connect_probe() {
        Ok(bytes) => format!("{bytes} bytes"),
        Err(message) => format!("unavailable ({message})"),
    }
}

fn connect_probe() -> Result<usize, String> {
    let address = TOUR_NETWORK
        .to_socket_addrs()
        .map_err(|lookup_error| lookup_error.to_string())?
        .next()
        .ok_or_else(|| "no address".to_string())?;
    let mut stream = TcpStream::connect_timeout(&address, NETWORK_TIMEOUT)
        .map_err(|connect_error| connect_error.to_string())?;
    stream
        .set_read_timeout(Some(NETWORK_TIMEOUT))
        .map_err(|timeout_error| timeout_error.to_string())?;
    stream
        .set_write_timeout(Some(NETWORK_TIMEOUT))
        .map_err(|timeout_error| timeout_error.to_string())?;
    stream
        .write_all(b"HEAD / HTTP/1.0\r\nHost: example.com\r\n\r\n")
        .map_err(|write_error| write_error.to_string())?;
    let mut buffer = [0_u8; 32];
    let read = stream
        .read(&mut buffer)
        .map_err(|read_error| read_error.to_string())?;
    Ok(read)
}

fn plugin_target() -> VariableTarget {
    VariableTarget {
        scope: VariableScope::Plugin,
        buffer: None,
        window: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_constants_are_stable() {
        assert!(PROBE_KEYS.contains("cX"));
        assert!(!PROBE_COMMAND.is_empty());
        assert!(PROBE_USER_EVENT.starts_with("Cockpit"));
    }
}
