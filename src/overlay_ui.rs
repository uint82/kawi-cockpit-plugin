use kawi_plugin_sdk::types::{DecorationStyle, Error};
use kawi_plugin_sdk::ui_overlay::{
    self, Key, OverlayAnchor, OverlayEvent, OverlayLine, OverlayOpenParams, OverlaySpan,
    OverlayUpdateParams,
};

use crate::joblog;
use crate::overlay::OverlayView;
use crate::state::State;

const WIDTH: u32 = 72;
const MIN_HEIGHT: u32 = 3;
const MAX_HEIGHT: u32 = 14;
const STDOUT_STYLE: Option<DecorationStyle> = None;
const STDERR_STYLE: Option<DecorationStyle> = Some(DecorationStyle::Error);

pub fn show(state: &mut State, title: &str) -> Result<(), Error> {
    close_stale(state)?;
    let lines = state.job.tail(joblog::MAX_LINES);
    if lines.is_empty() {
        return kawi_plugin_sdk::ui_message::message("cockpit: the task log is empty");
    }
    let height = (lines.len() as u32)
        .saturating_add(1)
        .clamp(MIN_HEIGHT, MAX_HEIGHT);
    let mut view = OverlayView::new(lines.clone(), height as usize);
    view.go_bottom();
    let overlay = open(title, &lines, height)?;
    state.overlay_id = Some(overlay);
    state.overlay = Some(view);
    let result = run(state, overlay);
    state.overlay_id = None;
    state.overlay = None;
    if result.is_err() {
        if let Err(close_error) = ui_overlay::close(overlay) {
            eprintln!(
                "cockpit: cannot close the task log: {}",
                close_error.message
            );
        }
    }
    result
}

fn run(state: &mut State, overlay: u64) -> Result<(), Error> {
    while let Some(view) = state.overlay.as_ref() {
        let selected = view.selected.map(|index| index as u32);
        let scroll = Some(view.scroll as u32);
        update(overlay, selected, scroll)?;
        let event = ui_overlay::wait(overlay)?;
        let Some(mut view) = state.overlay.take() else {
            break;
        };
        let close = handle(event, &mut view)?;
        state.overlay = Some(view);
        if close {
            ui_overlay::close(overlay)?;
            break;
        }
    }
    Ok(())
}

fn open(title: &str, lines: &[String], height: u32) -> Result<u64, Error> {
    let result = ui_overlay::open(&OverlayOpenParams {
        title: title.to_string(),
        status: Some(format!("{} lines", lines.len())),
        anchor: OverlayAnchor::Center,
        width: Some(WIDTH),
        height: Some(height),
        focus: true,
        selected: Some(lines.len().saturating_sub(1) as u32),
        scroll: Some(lines.len().saturating_sub(height as usize) as u32),
        lines: lines.iter().map(|line| overlay_line(line)).collect(),
    })?;
    Ok(result.overlay)
}

fn update(overlay: u64, selected: Option<u32>, scroll: Option<u32>) -> Result<(), Error> {
    ui_overlay::update(&OverlayUpdateParams {
        overlay,
        title: None,
        status: None,
        anchor: None,
        width: None,
        height: None,
        focus: None,
        selected,
        scroll,
        lines: None,
    })
}

fn handle(event: OverlayEvent, view: &mut OverlayView) -> Result<bool, Error> {
    match event {
        OverlayEvent::Key(event) => match event.key {
            Key::Escape | Key::Character('q') => Ok(true),
            Key::Up | Key::Character('k') => {
                view.move_selection(-1);
                Ok(false)
            }
            Key::Down | Key::Character('j') => {
                view.move_selection(1);
                Ok(false)
            }
            Key::PageUp => {
                view.page(-1);
                Ok(false)
            }
            Key::PageDown => {
                view.page(1);
                Ok(false)
            }
            Key::Home | Key::Character('g') => {
                view.go_top();
                Ok(false)
            }
            Key::End | Key::Character('G') => {
                view.go_bottom();
                Ok(false)
            }
            _ => Ok(false),
        },
        OverlayEvent::Paste(_) => Ok(false),
        OverlayEvent::Closed(_) => Ok(true),
    }
}

fn close_stale(state: &mut State) -> Result<(), Error> {
    if let Some(overlay) = state.overlay_id.take() {
        if let Err(error) = ui_overlay::close(overlay) {
            eprintln!(
                "cockpit: cannot close the previous overlay: {}",
                error.message
            );
        }
    }
    state.overlay = None;
    Ok(())
}

fn overlay_line(line: &str) -> OverlayLine {
    let style = if line.starts_with('!') {
        STDERR_STYLE
    } else {
        STDOUT_STYLE
    };
    OverlayLine {
        spans: vec![OverlaySpan {
            text: line.to_string(),
            style,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_stderr_lines() {
        let line = overlay_line("! warning");
        assert_eq!(line.spans[0].style, Some(DecorationStyle::Error));
    }

    #[test]
    fn q_and_escape_close_the_view() {
        let mut view = OverlayView::new(vec!["a".to_string()], 3);
        let key = |key| {
            OverlayEvent::Key(ui_overlay::KeyEvent {
                key,
                modifiers: Vec::new(),
                text: None,
            })
        };
        assert!(handle(key(Key::Character('q')), &mut view).expect("handle"));
        assert!(handle(key(Key::Escape), &mut view).expect("handle"));
        assert!(!handle(key(Key::Down), &mut view).expect("handle"));
    }
}
