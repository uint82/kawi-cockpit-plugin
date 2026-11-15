use kawi_plugin_sdk::types::{Argument, Error};
use kawi_plugin_sdk::ui_message;

use crate::error;
use crate::panel;
use crate::state::State;

pub fn open(state: &mut State) -> Result<Argument, Error> {
    focus_panel(state)?;
    ui_message::message("cockpit opened")?;
    Ok(Argument::Null)
}

pub fn refresh(state: &mut State) -> Result<Argument, Error> {
    ensure_panel(state)?;
    ui_message::message("cockpit refreshed")?;
    Ok(Argument::Null)
}

pub fn focus_panel(state: &mut State) -> Result<Argument, Error> {
    ensure_panel(state)?;
    panel::focus(state)?;
    Ok(Argument::Null)
}

fn ensure_panel(state: &mut State) -> Result<(), Error> {
    panel::refresh(state)?;
    if state.panel.is_none() {
        return Err(error::invalid("no project found for the active buffer"));
    }
    Ok(())
}
