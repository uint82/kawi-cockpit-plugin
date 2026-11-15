use std::path::Path;

pub fn root_name(root: &Path) -> String {
    root.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string())
}

pub fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

pub fn display_path(root: &Path, path: &str) -> String {
    let path = Path::new(path);
    match path.strip_prefix(root) {
        Ok(relative) if relative.as_os_str().is_empty() => root_name(root),
        Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
        Err(_) => shorten(path.to_string_lossy().as_ref(), 40),
    }
}

pub fn shorten(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return value.chars().take(max_chars).collect();
    }
    let kept: String = value.chars().take(max_chars - 3).collect();
    format!("{kept}...")
}

pub fn one_line(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for word in value.split_whitespace() {
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(word);
    }
    output
}

pub fn count_label(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortens_only_when_needed() {
        assert_eq!(shorten("abcd", 4), "abcd");
        assert_eq!(shorten("abcde", 4), "a...");
        assert_eq!(shorten("abcdef", 3), "abc");
        assert_eq!(shorten("abc", 0), "");
    }

    #[test]
    fn strips_the_workspace_root() {
        let root = Path::new("/tmp/project");
        assert_eq!(
            display_path(root, "/tmp/project/src/main.rs"),
            "src/main.rs"
        );
        assert_eq!(display_path(root, "/tmp/project"), "project");
    }

    #[test]
    fn shortens_paths_outside_the_workspace() {
        let root = Path::new("/tmp/project");
        let outside = format!("/elsewhere/{}", "x".repeat(60));
        let shown = display_path(root, &outside);
        assert!(shown.ends_with("..."));
        assert_eq!(shown.chars().count(), 40);
    }

    #[test]
    fn extracts_file_names_from_paths_and_bare_names() {
        assert_eq!(file_name("/work/src/main.rs"), "main.rs");
        assert_eq!(file_name("README.md"), "README.md");
    }

    #[test]
    fn flattens_multiline_messages() {
        assert_eq!(one_line("first\nsecond\tthird"), "first second third");
        assert_eq!(one_line("  kept   apart  "), "kept apart");
        assert_eq!(one_line(""), "");
    }

    #[test]
    fn pluralizes_counts() {
        assert_eq!(count_label(0, "error"), "0 errors");
        assert_eq!(count_label(1, "error"), "1 error");
        assert_eq!(count_label(2, "error"), "2 errors");
    }
}
