const HEADING: &str = "# Project Notes";
const ENTRY_PREFIX: &str = "## ";

pub fn count(contents: &str) -> usize {
    contents
        .lines()
        .filter(|line| line.starts_with(ENTRY_PREFIX))
        .count()
}

pub fn render_entry(timestamp: &str, text: &str) -> String {
    format!("{ENTRY_PREFIX}{timestamp}\n{text}\n\n")
}

pub fn with_entry(contents: &str, timestamp: &str, text: &str) -> String {
    let mut output = String::new();
    if contents.trim().is_empty() {
        output.push_str(HEADING);
        output.push_str("\n\n");
    } else {
        output.push_str(contents.trim_end());
        output.push_str("\n\n");
    }
    output.push_str(&render_entry(timestamp, text));
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_entries_not_headings() {
        assert_eq!(count("# Project Notes\n\n## one\n\n## two\n"), 2);
        assert_eq!(count(""), 0);
    }

    #[test]
    fn starts_a_new_file_with_a_heading() {
        let contents = with_entry("", "today", "first");
        assert!(contents.starts_with("# Project Notes\n\n## today\nfirst\n"));
        assert_eq!(count(&contents), 1);
    }

    #[test]
    fn appends_an_entry_with_one_blank_line() {
        let existing = "# Project Notes\n\n## old\nbody\n";
        let contents = with_entry(existing, "new", "second");
        assert!(contents.contains("body\n\n## new\nsecond\n"));
        assert_eq!(count(&contents), 2);
    }
}
