use kawi_plugin_sdk::types::Error;
use kawi_plugin_sdk::{INTERNAL_ERROR, INVALID_PARAMS};

pub fn invalid(message: impl Into<String>) -> Error {
    error(INVALID_PARAMS, message)
}

pub fn internal(message: impl Into<String>) -> Error {
    error(INTERNAL_ERROR, message)
}

pub fn error(code: i32, message: impl Into<String>) -> Error {
    Error {
        code,
        message: message.into(),
        capability: None,
    }
}

pub fn log(context: &str, error: &Error) {
    eprintln!("cockpit: {context}: {}", error.message);
}
