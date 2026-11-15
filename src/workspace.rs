use std::path::{Path, PathBuf};

const COCKPIT_DIR: &str = ".cockpit";
const TASKS_FILE: &str = "tasks";
const NOTES_FILE: &str = "notes.md";
const REPORT_FILE: &str = "report.md";
const CARGO_MANIFEST: &str = "Cargo.toml";
const GIT_DIR: &str = ".git";

pub fn detect(active_path: Option<&str>) -> Option<PathBuf> {
    let active = Path::new(active_path?);
    let start = if active.is_dir() {
        active
    } else {
        active.parent()?
    };
    start
        .ancestors()
        .find(|directory| is_root(directory))
        .map(Path::to_path_buf)
}

// Nearest marker wins: .cockpit, Cargo.toml, or .git.
pub fn is_root(directory: &Path) -> bool {
    directory.join(COCKPIT_DIR).is_dir()
        || directory.join(CARGO_MANIFEST).is_file()
        || directory.join(GIT_DIR).exists()
}

pub fn tasks_path(root: &Path) -> PathBuf {
    root.join(COCKPIT_DIR).join(TASKS_FILE)
}

pub fn notes_path(root: &Path) -> PathBuf {
    root.join(COCKPIT_DIR).join(NOTES_FILE)
}

pub fn report_path(root: &Path) -> PathBuf {
    root.join(COCKPIT_DIR).join(REPORT_FILE)
}

pub fn as_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn finds_a_cargo_root_from_a_nested_file() {
        let temp = TempDir::new("cargo");
        let source = temp.path().join("src");
        std::fs::create_dir_all(&source).expect("source dir");
        std::fs::write(temp.path().join(CARGO_MANIFEST), "").expect("manifest");
        let file = source.join("main.rs");
        std::fs::write(&file, "").expect("file");
        assert_eq!(
            detect(Some(&as_string(&file))),
            Some(temp.path().to_path_buf())
        );
    }

    #[test]
    fn prefers_the_nearest_root() {
        let temp = TempDir::new("nearest");
        std::fs::write(temp.path().join(CARGO_MANIFEST), "").expect("manifest");
        let nested = temp.path().join("tools");
        std::fs::create_dir_all(nested.join(COCKPIT_DIR)).expect("nested cockpit");
        assert_eq!(
            detect(Some(&as_string(&nested.join("notes.txt")))),
            Some(nested)
        );
    }

    #[test]
    fn accepts_directories_and_rejects_missing_roots() {
        let temp = TempDir::new("direct");
        std::fs::create_dir_all(temp.path().join(GIT_DIR)).expect("git dir");
        assert_eq!(
            detect(Some(&as_string(temp.path()))),
            Some(temp.path().to_path_buf())
        );
        assert_eq!(detect(Some("/definitely/not/a/project/file.rs")), None);
    }

    #[test]
    fn accepts_a_git_file_for_worktrees() {
        let temp = TempDir::new("worktree");
        std::fs::write(temp.path().join(GIT_DIR), "gitdir: elsewhere").expect("git file");
        assert_eq!(
            detect(Some(&as_string(temp.path()))),
            Some(temp.path().to_path_buf())
        );
    }

    #[test]
    fn builds_cockpit_paths() {
        let root = Path::new("/tmp/project");
        assert_eq!(tasks_path(root), root.join(".cockpit/tasks"));
        assert_eq!(notes_path(root), root.join(".cockpit/notes.md"));
        assert_eq!(report_path(root), root.join(".cockpit/report.md"));
    }
}
