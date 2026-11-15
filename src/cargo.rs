use std::path::Path;

use crate::format;

const ERROR_MARKER: &str = ": error: ";
const WARNING_MARKER: &str = ": warning: ";
const INFO_MARKER: &str = ": info: ";
const NOTE_MARKER: &str = ": note: ";
const HELP_MARKER: &str = ": help: ";
const MAX_RENDERED_MESSAGE: usize = 80;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

impl Severity {
    pub fn decoration_id(self) -> &'static str {
        match self {
            Self::Error => "diag_error",
            Self::Warning => "diag_warning",
            Self::Information => "diag_information",
            Self::Hint => "diag_hint",
        }
    }

    pub fn sign(self) -> &'static str {
        match self {
            Self::Error => "E",
            Self::Warning => "W",
            Self::Information => "I",
            Self::Hint => "H",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn render(&self, root: &Path) -> String {
        format!(
            "{}:{}:{} {} {}",
            format::display_path(root, &self.path),
            self.line,
            self.column,
            self.severity.sign(),
            format::shorten(&self.message, MAX_RENDERED_MESSAGE)
        )
    }
}

pub fn parse(text: &str) -> Vec<Diagnostic> {
    text.lines().filter_map(parse_line).collect()
}

pub fn parse_line(line: &str) -> Option<Diagnostic> {
    let line = line.trim().trim_start_matches("! ");
    // Messages can contain colons, so find the earliest severity marker and
    // parse the location from the right.
    let (index, marker, severity) = [
        (ERROR_MARKER, Severity::Error),
        (WARNING_MARKER, Severity::Warning),
        (INFO_MARKER, Severity::Information),
        (NOTE_MARKER, Severity::Information),
        (HELP_MARKER, Severity::Hint),
    ]
    .into_iter()
    .filter_map(|(marker, severity)| line.find(marker).map(|index| (index, marker, severity)))
    .min_by_key(|(index, _, _)| *index)?;

    let location = &line[..index];
    let message = line[index + marker.len()..].trim();
    let mut parts = location.rsplitn(3, ':');
    let column = parts.next()?.parse().ok()?;
    let line_number = parts.next()?.parse().ok()?;
    let path = parts.next()?.trim();
    if path.is_empty() || line_number == 0 || column == 0 {
        return None;
    }
    Some(Diagnostic {
        path: path.to_string(),
        line: line_number,
        column,
        severity,
        message: format::one_line(message),
    })
}

pub fn matches_path(recorded: &str, active: &str) -> bool {
    let recorded = recorded.strip_prefix("./").unwrap_or(recorded);
    recorded == active || Path::new(active).ends_with(recorded)
}

pub fn for_path<'a>(diagnostics: &'a [Diagnostic], path: &str) -> Vec<&'a Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| matches_path(&diagnostic.path, path))
        .collect()
}

pub fn count(diagnostics: &[Diagnostic], severity: Severity) -> usize {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == severity)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cargo_short_messages() {
        let diagnostic =
            parse_line("src/main.rs:12:5: error: expected semicolon").expect("diagnostic");
        assert_eq!(diagnostic.path, "src/main.rs");
        assert_eq!(diagnostic.line, 12);
        assert_eq!(diagnostic.column, 5);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.message, "expected semicolon");
    }

    #[test]
    fn ignores_blank_lines() {
        assert!(parse("\n  \n").is_empty());
        assert!(parse_line("").is_none());
    }

    #[test]
    fn strips_stderr_prefixes_and_carriage_returns() {
        let diagnostic = parse_line("! src/main.rs:2:2: warning: unused\r").expect("diagnostic");
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(diagnostic.message, "unused");
    }

    #[test]
    fn classifies_warning_info_note_and_help() {
        assert_eq!(
            parse_line("a.rs:1:1: warning: unused").map(|d| d.severity),
            Some(Severity::Warning)
        );
        assert_eq!(
            parse_line("a.rs:1:1: note: see docs").map(|d| d.severity),
            Some(Severity::Information)
        );
        assert_eq!(
            parse_line("a.rs:1:1: help: try this").map(|d| d.severity),
            Some(Severity::Hint)
        );
    }

    #[test]
    fn ignores_lines_without_a_location() {
        assert!(parse_line("error: could not compile").is_none());
        assert!(parse_line("src/main.rs:0:1: error: bad").is_none());
        assert!(parse_line("src/main.rs:x:1: error: bad").is_none());
    }

    #[test]
    fn keeps_colons_in_messages() {
        let diagnostic = parse_line("src/main.rs:3:4: error: expected: type").expect("diagnostic");
        assert_eq!(diagnostic.message, "expected: type");
    }

    #[test]
    fn matches_relative_and_absolute_paths() {
        assert!(matches_path("src/main.rs", "/work/src/main.rs"));
        assert!(matches_path("./src/main.rs", "/work/src/main.rs"));
        assert!(matches_path("/work/src/main.rs", "/work/src/main.rs"));
        assert!(!matches_path("src/lib.rs", "/work/src/main.rs"));
    }

    #[test]
    fn filters_and_counts() {
        let diagnostics = parse(
            "src/main.rs:1:1: error: e\nsrc/main.rs:2:1: warning: w\nsrc/lib.rs:1:1: error: e\n",
        );
        assert_eq!(for_path(&diagnostics, "/work/src/main.rs").len(), 2);
        assert_eq!(count(&diagnostics, Severity::Error), 2);
        assert_eq!(count(&diagnostics, Severity::Hint), 0);
    }
}
