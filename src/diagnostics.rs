use kawi_plugin_sdk::editor_decorations;
use kawi_plugin_sdk::editor_signs;
use kawi_plugin_sdk::types::{
    Decoration, Diagnostic as HostDiagnostic, Error, Highlight, Position, Severity as HostSeverity,
};

use crate::cargo::{Diagnostic, Severity};
use crate::format;

const HIGHLIGHT_WIDTH: u32 = 1;

pub fn from_host(path: &str, diagnostics: Vec<HostDiagnostic>) -> Vec<Diagnostic> {
    // Host positions are zero-based; the panel and cargo output are not.
    diagnostics
        .into_iter()
        .map(|diagnostic| Diagnostic {
            path: path.to_string(),
            line: diagnostic.start.line.saturating_add(1),
            column: diagnostic.start.byte_column.saturating_add(1),
            severity: severity(diagnostic.severity),
            message: format::one_line(&diagnostic.message),
        })
        .collect()
}

pub fn publish(buffer: u64, revision: u64, diagnostics: &[Diagnostic]) -> Result<(), Error> {
    let decorations: Vec<Decoration> = diagnostics.iter().filter_map(highlight).collect();
    editor_decorations::set(buffer, revision, &decorations)?;
    let signs: Vec<editor_signs::Sign> = diagnostics
        .iter()
        .map(|diagnostic| editor_signs::Sign {
            line: diagnostic.line.saturating_sub(1),
            text: diagnostic.severity.sign().to_string(),
            group: diagnostic.severity.decoration_id().to_string(),
        })
        .collect();
    editor_signs::set(buffer, revision, &signs)
}

// One cell is enough. The message belongs in the panel, not on top of the code.
fn highlight(diagnostic: &Diagnostic) -> Option<Decoration> {
    let line = diagnostic.line.checked_sub(1)?;
    let column = diagnostic.column.checked_sub(1)?;
    Some(Decoration::Highlight(Highlight {
        start: Position {
            line,
            byte_column: column,
        },
        end: Position {
            line,
            byte_column: column.saturating_add(HIGHLIGHT_WIDTH),
        },
        group: diagnostic.severity.decoration_id().to_string(),
    }))
}

fn severity(severity: HostSeverity) -> Severity {
    match severity {
        HostSeverity::Error => Severity::Error,
        HostSeverity::Warning => Severity::Warning,
        HostSeverity::Information => Severity::Information,
        HostSeverity::Hint => Severity::Hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(severity: HostSeverity, line: u32, column: u32) -> HostDiagnostic {
        HostDiagnostic {
            start: Position {
                line,
                byte_column: column,
            },
            end: Position {
                line,
                byte_column: column + 1,
            },
            severity,
            message: "message".to_string(),
            source: None,
            code: None,
        }
    }

    #[test]
    fn converts_positions_to_one_based() {
        let converted = from_host("src/main.rs", vec![host(HostSeverity::Warning, 4, 7)]);
        assert_eq!(converted[0].path, "src/main.rs");
        assert_eq!(converted[0].line, 5);
        assert_eq!(converted[0].column, 8);
        assert_eq!(converted[0].severity, Severity::Warning);
    }

    #[test]
    fn flattens_multiline_messages() {
        let mut diagnostic = host(HostSeverity::Error, 0, 0);
        diagnostic.message = "first\nsecond".to_string();
        let converted = from_host("src/main.rs", vec![diagnostic]);
        assert_eq!(converted[0].message, "first second");
    }

    #[test]
    fn maps_every_host_severity() {
        let severities = [
            (HostSeverity::Error, Severity::Error),
            (HostSeverity::Warning, Severity::Warning),
            (HostSeverity::Information, Severity::Information),
            (HostSeverity::Hint, Severity::Hint),
        ];
        for (host, expected) in severities {
            assert_eq!(severity(host), expected);
        }
    }

    #[test]
    fn builds_a_one_character_highlight() {
        let diagnostic = Diagnostic {
            path: "src/main.rs".to_string(),
            line: 3,
            column: 2,
            severity: Severity::Error,
            message: "bad".to_string(),
        };
        match highlight(&diagnostic).expect("highlight") {
            Decoration::Highlight(highlight) => {
                assert_eq!(highlight.start.line, 2);
                assert_eq!(highlight.start.byte_column, 1);
                assert_eq!(highlight.end.byte_column, 2);
                assert_eq!(highlight.group, "diag_error");
            }
            Decoration::VirtualText(_) => panic!("expected highlight"),
        }
    }

    #[test]
    fn ignores_zero_positions() {
        let diagnostic = Diagnostic {
            path: "x".to_string(),
            line: 0,
            column: 0,
            severity: Severity::Hint,
            message: String::new(),
        };
        assert!(highlight(&diagnostic).is_none());
    }
}
