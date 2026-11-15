use std::collections::VecDeque;

pub const MAX_LINES: usize = 300;
const SUMMARY_TAIL: usize = 160;

#[derive(Clone, Debug, Default)]
pub struct JobLog {
    pub name: Option<String>,
    pub status: String,
    running: bool,
    lines: VecDeque<String>,
}

impl JobLog {
    pub fn start(&mut self, name: String) {
        self.name = Some(name.clone());
        self.status = format!("running {name}");
        self.running = true;
        self.lines.clear();
    }

    pub fn append(&mut self, channel: &str, text: &str) {
        for line in text.lines() {
            if self.lines.len() == MAX_LINES {
                self.lines.pop_front();
            }
            let line = line.trim_end_matches('\r');
            // The overlay needs no extra metadata to paint stderr red.
            if channel == "stderr" {
                self.lines.push_back(format!("! {line}"));
            } else {
                self.lines.push_back(line.to_string());
            }
        }
    }

    pub fn finish(&mut self, status: String) {
        self.status = crate::format::one_line(&status);
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn tail(&self, count: usize) -> Vec<String> {
        let skip = self.lines.len().saturating_sub(count);
        self.lines.iter().skip(skip).cloned().collect()
    }

    pub fn lines(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }

    pub fn summary(&self) -> Option<String> {
        let status = self.name.as_ref().map(|_| self.status.as_str())?;
        match self.lines.back() {
            Some(last) => Some(format!(
                "{} {}",
                status,
                super::format::shorten(last, SUMMARY_TAIL)
            )),
            None => Some(status.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_newest_lines() {
        let mut log = JobLog::default();
        for index in 0..(MAX_LINES + 5) {
            log.append("stdout", &format!("{index}\n"));
        }
        assert_eq!(log.lines().len(), MAX_LINES);
        assert_eq!(log.lines().first().map(String::as_str), Some("5"));
    }

    #[test]
    fn trims_carriage_returns_and_marks_stderr() {
        let mut log = JobLog::default();
        log.append("stdout", "ok\r\n");
        log.append("stderr", "warn\n");
        assert_eq!(log.lines(), ["ok", "! warn"]);
    }

    #[test]
    fn reports_running_state_and_summary() {
        let mut log = JobLog::default();
        assert!(!log.is_running());
        log.start("test".to_string());
        log.append("stdout", "compiling\n");
        assert!(log.is_running());
        assert_eq!(log.summary().as_deref(), Some("running test compiling"));
        log.finish("succeeded in 120ms".to_string());
        assert!(!log.is_running());
    }

    #[test]
    fn flattens_multiline_status() {
        let mut log = JobLog::default();
        log.start("test".to_string());
        log.finish("failed:\nsecond line".to_string());
        assert_eq!(log.summary().as_deref(), Some("failed: second line"));
    }

    #[test]
    fn summary_needs_a_named_job() {
        let mut log = JobLog::default();
        log.append("stdout", "orphan output\n");
        assert_eq!(log.summary(), None);
    }

    #[test]
    fn tail_is_clamped_to_the_available_lines() {
        let mut log = JobLog::default();
        log.append("stdout", "only\n");
        assert_eq!(log.tail(10), ["only"]);
    }

    #[test]
    fn tail_keeps_the_newest_lines() {
        let mut log = JobLog::default();
        log.append("stdout", "a\nb\nc\n");
        assert_eq!(log.tail(2), ["b", "c"]);
    }
}
