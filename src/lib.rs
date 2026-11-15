//! Project Cockpit: a Kawi WASM plugin that keeps a project dashboard,
//! caches diagnostics, and runs one task at a time.
//!
//! Host calls live in `commands`, `panel`, `diagnostics`, and `worker`.
//! Parsing, formatting, and panel layout stay pure so they are cheap to test.

use std::collections::BTreeMap;
use std::sync::Mutex;

use kawi_plugin_sdk::editor_autocmds::{self, AutocmdSpec};
use kawi_plugin_sdk::editor_commands::{self, CommandSpec};
use kawi_plugin_sdk::editor_keymaps::{self, KeyMode, KeymapSpec};
use kawi_plugin_sdk::editor_schedule;
use kawi_plugin_sdk::exports::kawi::plugin::callback_handler::{
    Callback, CallbackEvent, Guest as CallbackGuest,
};
use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::types::{Argument, Error, Event};
use kawi_plugin_sdk::INVALID_PARAMS;

mod cargo;
mod commands;
mod dashboard;
mod diagnostics;
mod error;
mod format;
mod joblog;
mod notes;
mod overlay;
mod overlay_ui;
mod panel;
mod project;
mod report;
mod settings;
mod state;
mod tasks;
mod worker;
mod workspace;

#[cfg(test)]
mod test_support;

use commands::dispatch;
use settings::Settings;
use state::State;
use worker::Worker;

const PROTOCOL_VERSION: u32 = 1;
const TOUR_COMMAND: &str = "tour";
const TOUR_KEYS: &str = "<leader>cK";
const REFRESH_EVENT: &str = "CockpitRefresh";
const REFRESH_PAYLOAD: &str = "refresh";
const TOUR_TITLE: &str = "Cockpit API tour";
const TOUR_DESCRIPTION: &str = "Exercise the Kawi plugin API";

static PLUGIN: Mutex<Option<Plugin>> = Mutex::new(None);

struct Plugin {
    state: State,
    worker: Worker,
    tour_name: String,
}

impl Guest for Plugin {
    fn init(host_protocol: u32) -> Result<(), Error> {
        if host_protocol != PROTOCOL_VERSION {
            return Err(protocol_error());
        }
        let settings = Settings::load();
        let mut plugin = Self {
            state: State::new(settings),
            worker: Worker::default(),
            tour_name: String::new(),
        };
        register_refresh()?;
        editor_schedule::every(settings.refresh_ms, Some(REFRESH_PAYLOAD))?;
        let tour = editor_commands::register(&CommandSpec {
            id: TOUR_COMMAND.to_string(),
            title: TOUR_TITLE.to_string(),
            description: Some(TOUR_DESCRIPTION.to_string()),
            aliases: Vec::new(),
        })?;
        editor_keymaps::set(&KeymapSpec {
            mode: KeyMode::Normal,
            keys: TOUR_KEYS.to_string(),
            command: tour.name.clone(),
            description: Some(TOUR_DESCRIPTION.to_string()),
        })?;
        plugin.tour_name = tour.name;
        let mut guard = PLUGIN
            .lock()
            .map_err(|_| error::internal("plugin state lock is poisoned"))?;
        *guard = Some(plugin);
        Ok(())
    }

    fn invoke_command(id: String, _args: Vec<Argument>) -> Result<Argument, Error> {
        with_plugin(|plugin| {
            let Plugin {
                state,
                worker,
                tour_name,
            } = plugin;
            dispatch(
                &id,
                commands::Context {
                    state,
                    worker,
                    tour_name,
                },
            )
        })
    }

    fn on_event(event: Event) {
        if let Err(event_error) = with_plugin(|plugin| plugin.handle_event(event)) {
            error::log("event handling failed", &event_error);
        }
    }
}

impl Plugin {
    fn handle_event(&mut self, event: Event) -> Result<(), Error> {
        self.poll_job()?;
        match event {
            Event::BufferOpened(event) => {
                if let Some(path) = event.path {
                    self.state.track_buffer(event.buffer, path);
                }
                panel::refresh(&mut self.state)
            }
            Event::FileOpened(event) => {
                self.state.track_buffer(event.buffer, event.path);
                panel::refresh(&mut self.state)
            }
            Event::BufferClosed(event) => {
                self.state.untrack_buffer(event.buffer);
                panel::refresh(&mut self.state)
            }
            Event::DiagnosticsChanged(event) => self.handle_diagnostics(event),
            Event::BufferChanged(_) | Event::FileSaved(_) | Event::ModeChanged(_) => {
                panel::refresh(&mut self.state)
            }
            _ => Ok(()),
        }
    }

    fn poll_job(&mut self) -> Result<(), Error> {
        let was_running = self.worker.is_running();
        let changed = self.worker.poll(&mut self.state);
        // Cargo output only makes sense once the job stops. Reading a moving
        // target gives you half a diagnostic and a headache.
        if changed && was_running && !self.worker.is_running() {
            self.collect_cargo_diagnostics()?;
        }
        Ok(())
    }

    fn collect_cargo_diagnostics(&mut self) -> Result<(), Error> {
        let diagnostics = cargo::parse(&self.state.job.lines().join("\n"));
        if diagnostics.is_empty() {
            return Ok(());
        }
        let grouped = self.group_cargo_diagnostics(diagnostics);
        if grouped.is_empty() {
            return Ok(());
        }
        let active = kawi_plugin_sdk::editor_read::get_state(false)?;
        for (buffer, found) in grouped {
            if buffer == active.buffer {
                self.state.set_diagnostics(buffer, found.clone());
                diagnostics::publish(buffer, active.revision, &found)?;
            } else {
                self.state.set_diagnostics(buffer, found);
            }
        }
        Ok(())
    }

    fn group_cargo_diagnostics(
        &self,
        diagnostics: Vec<cargo::Diagnostic>,
    ) -> BTreeMap<u64, Vec<cargo::Diagnostic>> {
        let mut grouped = BTreeMap::new();
        for diagnostic in diagnostics {
            if let Some(buffer) = self.state.buffer_for_path(&diagnostic.path) {
                grouped
                    .entry(buffer)
                    .or_insert_with(Vec::new)
                    .push(diagnostic);
            }
        }
        grouped
    }

    fn handle_diagnostics(
        &mut self,
        event: kawi_plugin_sdk::types::DiagnosticsChanged,
    ) -> Result<(), Error> {
        let Some(buffer) = event.buffer else {
            return Ok(());
        };
        self.state.track_buffer(buffer, event.path.clone());
        let diagnostics = diagnostics::from_host(&event.path, event.diagnostics);
        if !self.state.set_diagnostics(buffer, diagnostics.clone()) {
            return Ok(());
        }
        let active = kawi_plugin_sdk::editor_read::get_state(false)?;
        if active.buffer == buffer {
            diagnostics::publish(buffer, event.revision, &diagnostics)?;
        }
        panel::refresh(&mut self.state)
    }
}

impl CallbackGuest for Plugin {
    fn on_callback(callback: Callback) -> Result<(), Error> {
        match callback {
            Callback::Timer(timer) => {
                if timer.payload.as_deref() == Some(REFRESH_PAYLOAD) {
                    with_plugin(|plugin| {
                        plugin.poll_job()?;
                        panel::refresh(&mut plugin.state)
                    })?;
                }
            }
            Callback::Autocmd(autocmd) => {
                if let CallbackEvent::User(event) = autocmd.event {
                    if event.name == REFRESH_EVENT {
                        with_plugin(|plugin| {
                            plugin.poll_job()?;
                            panel::refresh(&mut plugin.state)
                        })?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn register_refresh() -> Result<(), Error> {
    editor_autocmds::register(&AutocmdSpec {
        event: "user".to_string(),
        pattern: Some(REFRESH_EVENT.to_string()),
        buffer: None,
        once: false,
    })?;
    Ok(())
}

fn with_plugin<T>(action: impl FnOnce(&mut Plugin) -> Result<T, Error>) -> Result<T, Error> {
    let mut guard = PLUGIN
        .lock()
        .map_err(|_| error::internal("plugin state lock is poisoned"))?;
    let plugin = guard
        .as_mut()
        .ok_or_else(|| error::internal("plugin is not initialized"))?;
    action(plugin)
}

fn protocol_error() -> Error {
    error::error(INVALID_PARAMS, "unsupported host protocol version")
}

kawi_plugin_sdk::export!(Plugin);
