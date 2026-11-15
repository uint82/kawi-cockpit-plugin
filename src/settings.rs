use std::ops::RangeInclusive;

use kawi_plugin_sdk::config_read;
use kawi_plugin_sdk::types::SettingValue;

const DEFAULT_REFRESH_MS: i64 = 2_000;
const DEFAULT_PANEL_HEIGHT: i64 = 12;
const DEFAULT_MAX_BUFFERS: i64 = 8;
const DEFAULT_MAX_DIAGNOSTICS: i64 = 6;
const SHOW_STATUSLINE_DEFAULT: bool = true;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub refresh_ms: u32,
    pub panel_height: u32,
    pub max_buffers: usize,
    pub max_diagnostics: usize,
    pub show_statusline: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_ms: DEFAULT_REFRESH_MS as u32,
            panel_height: DEFAULT_PANEL_HEIGHT as u32,
            max_buffers: DEFAULT_MAX_BUFFERS as usize,
            max_diagnostics: DEFAULT_MAX_DIAGNOSTICS as usize,
            show_statusline: SHOW_STATUSLINE_DEFAULT,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let defaults = Self::default();
        Self {
            refresh_ms: read_integer("refresh_ms", DEFAULT_REFRESH_MS, 1_000..=60_000) as u32,
            panel_height: read_integer("panel_height", DEFAULT_PANEL_HEIGHT, 4..=18) as u32,
            max_buffers: read_integer("max_buffers", DEFAULT_MAX_BUFFERS, 1..=32) as usize,
            max_diagnostics: read_integer("max_diagnostics", DEFAULT_MAX_DIAGNOSTICS, 1..=20)
                as usize,
            show_statusline: read_boolean("show_statusline", defaults.show_statusline),
        }
    }
}

fn read_integer(key: &str, fallback: i64, bounds: RangeInclusive<i64>) -> i64 {
    integer_setting(read_setting(key), fallback, bounds)
}

fn read_boolean(key: &str, fallback: bool) -> bool {
    boolean_setting(read_setting(key), fallback)
}

fn read_setting(key: &str) -> Option<SettingValue> {
    match config_read::get(key) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cockpit: cannot read setting {key}: {}", error.message);
            None
        }
    }
}

fn integer_setting(value: Option<SettingValue>, fallback: i64, bounds: RangeInclusive<i64>) -> i64 {
    match value {
        Some(SettingValue::Integer(value)) => value.clamp(*bounds.start(), *bounds.end()),
        _ => fallback,
    }
}

fn boolean_setting(value: Option<SettingValue>, fallback: bool) -> bool {
    match value {
        Some(SettingValue::Boolean(value)) => value,
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_integers_to_the_allowed_range() {
        assert_eq!(
            integer_setting(Some(SettingValue::Integer(500)), 2_000, 1_000..=60_000),
            1_000
        );
        assert_eq!(
            integer_setting(Some(SettingValue::Integer(90_000)), 2_000, 1_000..=60_000),
            60_000
        );
        assert_eq!(
            integer_setting(Some(SettingValue::Integer(4_000)), 2_000, 1_000..=60_000),
            4_000
        );
    }

    #[test]
    fn falls_back_for_missing_or_wrong_types() {
        assert_eq!(integer_setting(None, 2_000, 1_000..=60_000), 2_000);
        assert_eq!(
            integer_setting(Some(SettingValue::Boolean(true)), 2_000, 1_000..=60_000),
            2_000
        );
        assert!(boolean_setting(None, true));
        assert!(boolean_setting(
            Some(SettingValue::Text("yes".to_string())),
            true
        ));
    }

    #[test]
    fn reads_booleans() {
        assert!(boolean_setting(Some(SettingValue::Boolean(true)), false));
        assert!(!boolean_setting(Some(SettingValue::Boolean(false)), true));
    }
}
