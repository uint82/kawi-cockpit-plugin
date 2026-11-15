use kawi_plugin_sdk::types::{Argument, Error};

use crate::error;
use crate::state::State;
use crate::worker::Worker;

mod dashboard;
mod navigation;
mod notes;
mod overlay;
mod tasks;
mod tour;

pub struct Context<'a> {
    pub state: &'a mut State,
    pub worker: &'a mut Worker,
    pub tour_name: &'a str,
}

pub fn dispatch(id: &str, context: Context<'_>) -> Result<Argument, Error> {
    let Context {
        state,
        worker,
        tour_name,
    } = context;
    match id {
        "open" => dashboard::open(state),
        "refresh" => dashboard::refresh(state),
        "focus-panel" => dashboard::focus_panel(state),
        "tasks" => tasks::run(state, worker),
        "stop" => tasks::stop(state, worker),
        "log" => tasks::log(state),
        "buffers" => navigation::buffers(),
        "windows" => navigation::windows(),
        "split" => navigation::split(),
        "close-window" => navigation::close_window(),
        "mode" => navigation::mode(),
        "next-diagnostic" => navigation::next_diagnostic(state),
        "toggle-numbers" => navigation::toggle_numbers(),
        "save" => navigation::save(),
        "note" => notes::note(state),
        "insert-task" => notes::insert_task(state),
        "report" => notes::report(state),
        "tour" => tour::run(state, tour_name),
        other => Err(error::invalid(format!("unknown command {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn rejects_unknown_commands() {
        let mut state = State::new(Settings::default());
        let mut worker = Worker::default();
        let result = dispatch(
            "missing",
            Context {
                state: &mut state,
                worker: &mut worker,
                tour_name: "cockpit.tour",
            },
        );
        assert!(result
            .expect_err("unknown command")
            .message
            .contains("missing"));
    }
}
