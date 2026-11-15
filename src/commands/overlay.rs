use kawi_plugin_sdk::types::{Argument, Error};

use crate::overlay_ui;
use crate::state::State;

pub fn show_log(state: &mut State) -> Result<Argument, Error> {
    overlay_ui::show(state, "Cockpit task log")?;
    Ok(Argument::Null)
}
