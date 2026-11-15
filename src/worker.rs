use std::path::Path;

use kawi_plugin_sdk::process_jobs::{
    self, JobEvent, JobSignal, JobState, JobStatus, JobStream, OpenParams, StdinMode,
};
use kawi_plugin_sdk::types::Error;

use crate::state::State;
use crate::tasks::Task;
use crate::workspace;

#[derive(Debug, Default)]
pub struct Worker {
    job: Option<u64>,
}

impl Worker {
    pub fn is_running(&self) -> bool {
        self.job.is_some()
    }

    pub fn start(&mut self, task: &Task, root: &Path, state: &mut State) -> Result<(), Error> {
        if self.is_running() {
            return Err(crate::error::invalid("a cockpit task is already running"));
        }
        let job = process_jobs::open(&OpenParams {
            command: task.program.clone(),
            args: task.args.clone(),
            cwd: Some(workspace::as_string(root)),
            env: Vec::new(),
            stdin: StdinMode::Null,
        })?;
        self.job = Some(job);
        state.job.start(task.name.clone());
        Ok(())
    }

    pub fn poll(&mut self, state: &mut State) -> bool {
        let Some(job) = self.job else {
            return false;
        };
        let events = match process_jobs::poll(job) {
            Ok(events) => events,
            Err(error) => {
                self.finish(state, format!("failed: {}", error.message));
                return true;
            }
        };
        let mut changed = !events.is_empty();
        for event in events {
            self.apply_event(state, event);
        }
        if self.job.is_none() {
            return true;
        }
        let status = match process_jobs::status(job) {
            Ok(status) => status,
            Err(error) => {
                self.finish(state, format!("failed: {}", error.message));
                return true;
            }
        };
        if status.state != JobState::Running {
            self.finish(state, status_text(&status));
            changed = true;
        }
        changed
    }

    fn apply_event(&mut self, state: &mut State, event: JobEvent) {
        match event {
            JobEvent::Output(output) => {
                if output.truncated {
                    state
                        .job
                        .append("stderr", "output truncated at the host limit\n");
                }
                state.job.append(channel_name(output.channel), &output.data);
            }
            JobEvent::Exited(exited) => {
                let status = JobStatus {
                    state: exited.state,
                    exit_code: exited.exit_code,
                    signal: exited.signal,
                };
                self.finish(state, status_text(&status));
            }
        }
    }

    fn finish(&mut self, state: &mut State, status: String) {
        state.job.finish(status);
        self.job = None;
    }

    pub fn stop(&mut self, state: &mut State) -> Result<bool, Error> {
        let Some(job) = self.job else {
            return Ok(false);
        };
        process_jobs::signal(job, JobSignal::Terminate)?;
        state.job.finish("stopping".to_string());
        Ok(true)
    }
}

fn channel_name(channel: JobStream) -> &'static str {
    match channel {
        JobStream::Stdout => "stdout",
        JobStream::Stderr => "stderr",
    }
}

fn status_text(status: &JobStatus) -> String {
    match status.state {
        JobState::Running => "running".to_string(),
        JobState::Exited => match status.exit_code {
            Some(0) => "succeeded".to_string(),
            Some(code) => format!("failed with exit code {code}"),
            None => "exited".to_string(),
        },
        JobState::Killed => match status.signal {
            Some(signal) => format!("killed by signal {signal}"),
            None => "killed".to_string(),
        },
        JobState::Failed => "failed to start".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_other_terminal_states() {
        assert_eq!(
            status_text(&JobStatus {
                state: JobState::Running,
                exit_code: None,
                signal: None,
            }),
            "running"
        );
        assert_eq!(
            status_text(&JobStatus {
                state: JobState::Exited,
                exit_code: None,
                signal: None,
            }),
            "exited"
        );
        assert_eq!(
            status_text(&JobStatus {
                state: JobState::Killed,
                signal: None,
                exit_code: None,
            }),
            "killed"
        );
        assert_eq!(
            status_text(&JobStatus {
                state: JobState::Failed,
                signal: None,
                exit_code: None,
            }),
            "failed to start"
        );
    }

    #[test]
    fn describes_success_and_failure() {
        assert_eq!(
            status_text(&JobStatus {
                state: JobState::Exited,
                exit_code: Some(0),
                signal: None,
            }),
            "succeeded"
        );
        assert_eq!(
            status_text(&JobStatus {
                state: JobState::Exited,
                exit_code: Some(7),
                signal: None,
            }),
            "failed with exit code 7"
        );
        assert_eq!(
            status_text(&JobStatus {
                state: JobState::Killed,
                signal: Some(15),
                exit_code: None,
            }),
            "killed by signal 15"
        );
    }
}
