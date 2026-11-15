use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use kawi_plugin_sdk::editor_read;
use kawi_plugin_sdk::types::Error;

use crate::error;
use crate::notes;
use crate::state::State;
use crate::tasks::{self, Task};
use crate::workspace;

pub fn root(state: &mut State) -> Result<PathBuf, Error> {
    let editor = editor_read::get_state(false)?;
    let root = workspace::detect(editor.path.as_deref())
        .ok_or_else(|| error::invalid("open a file inside a project first"))?;
    state.workspace = Some(root.clone());
    Ok(root)
}

pub fn tasks(root: &Path) -> Vec<Task> {
    let path = workspace::tasks_path(root);
    let contents = match read_optional(&path) {
        Ok(contents) => contents,
        Err(read_error) => {
            error::log("cannot read tasks", &read_error);
            None
        }
    };
    match tasks::load(contents.as_deref()) {
        Ok(tasks) => tasks,
        Err(message) => {
            eprintln!("cockpit: {}: {message}", path.display());
            tasks::defaults()
        }
    }
}

pub fn notes_count(root: &Path) -> usize {
    match read_optional(&workspace::notes_path(root)) {
        Ok(Some(contents)) => notes::count(&contents),
        Ok(None) => 0,
        Err(read_error) => {
            error::log("cannot read notes", &read_error);
            0
        }
    }
}

pub fn read_optional(path: &Path) -> Result<Option<String>, Error> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(read_error) if read_error.kind() == ErrorKind::NotFound => Ok(None),
        Err(read_error) => Err(error::internal(format!(
            "cannot read {}: {read_error}",
            path.display()
        ))),
    }
}

pub fn ensure_dir(root: &Path) -> Result<(), Error> {
    fs::create_dir_all(root.join(".cockpit")).map_err(|write_error| {
        error::internal(format!(
            "cannot create {}: {write_error}",
            root.join(".cockpit").display()
        ))
    })
}

pub fn write(path: &Path, contents: &str) -> Result<(), Error> {
    fs::write(path, contents).map_err(|write_error| {
        error::internal(format!("cannot write {}: {write_error}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn reads_optional_files() {
        let temp = TempDir::new("project-read");
        let path = temp.path().join("missing.txt");
        assert!(read_optional(&path).expect("read").is_none());
        write(&path, "content").expect("write");
        assert_eq!(
            read_optional(&path).expect("read").as_deref(),
            Some("content")
        );
    }

    #[test]
    fn creates_the_cockpit_directory() {
        let temp = TempDir::new("project-dir");
        ensure_dir(temp.path()).expect("ensure dir");
        assert!(temp.path().join(".cockpit").is_dir());
    }

    #[test]
    fn counts_notes_and_handles_missing_files() {
        let temp = TempDir::new("project-notes");
        assert_eq!(notes_count(temp.path()), 0);
        ensure_dir(temp.path()).expect("ensure dir");
        write(
            &workspace::notes_path(temp.path()),
            "# Project Notes\n\n## one\n",
        )
        .expect("write notes");
        assert_eq!(notes_count(temp.path()), 1);
    }

    #[test]
    fn falls_back_when_the_task_file_is_invalid() {
        let temp = TempDir::new("bad-tasks");
        ensure_dir(temp.path()).expect("ensure dir");
        write(&workspace::tasks_path(temp.path()), "not a task\n").expect("write tasks");
        assert_eq!(tasks(temp.path()), tasks::defaults());
    }

    #[test]
    fn loads_custom_tasks_and_falls_back_to_defaults() {
        let temp = TempDir::new("project-tasks");
        assert_eq!(tasks(temp.path()), tasks::defaults());
        ensure_dir(temp.path()).expect("ensure dir");
        write(
            &workspace::tasks_path(temp.path()),
            "check = cargo check --all-targets\n",
        )
        .expect("write tasks");
        let loaded = tasks(temp.path());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "check");
    }
}
