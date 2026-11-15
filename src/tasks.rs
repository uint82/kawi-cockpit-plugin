use std::collections::BTreeSet;

const ASSIGNMENT: char = '=';
const COMMENT: char = '#';

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
}

impl Task {
    fn parse(name: &str, command: &str) -> Result<Self, String> {
        let mut words =
            split_words(command).map_err(|message| format!("task {name}: {message}"))?;
        if words.is_empty() {
            return Err(format!("task {name}: command is empty"));
        }
        let program = words.remove(0);
        Ok(Self {
            name: name.to_string(),
            program,
            args: words,
        })
    }

    pub fn command_line(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub fn parse(contents: &str) -> Result<Vec<Task>, String> {
    let mut tasks = Vec::new();
    let mut names = BTreeSet::new();
    for (index, raw) in contents.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let (name, command) = line
            .split_once(ASSIGNMENT)
            .ok_or_else(|| format!("line {}: expected name = command", index + 1))?;
        let name = name.trim();
        if name.is_empty() || name.chars().any(char::is_whitespace) {
            return Err(format!("line {}: invalid task name", index + 1));
        }
        if !names.insert(name.to_string()) {
            return Err(format!("line {}: duplicate task {name}", index + 1));
        }
        let task = Task::parse(name, command.trim())
            .map_err(|message| format!("line {}: {message}", index + 1))?;
        tasks.push(task);
    }
    Ok(tasks)
}

pub fn load(contents: Option<&str>) -> Result<Vec<Task>, String> {
    match contents {
        Some(contents) => parse(contents),
        None => Ok(defaults()),
    }
}

pub fn find<'a>(tasks: &'a [Task], name: &str) -> Option<&'a Task> {
    tasks.iter().find(|task| task.name == name)
}

pub fn defaults() -> Vec<Task> {
    vec![
        default_task("check", "cargo", &["check"]),
        default_task("test", "cargo", &["test"]),
        default_task("build", "cargo", &["build"]),
        default_task("fmt", "cargo", &["fmt", "--check"]),
        default_task("clippy", "cargo", &["clippy"]),
        default_task("status", "git", &["status", "--short"]),
    ]
}

fn default_task(name: &str, program: &str, args: &[&str]) -> Task {
    Task {
        name: name.to_string(),
        program: program.to_string(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
    }
}

pub fn split_words(command: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut chars = command.chars();
    while let Some(character) = chars.next() {
        // Backslashes escape the next character, inside or outside quotes.
        if character == '\\' {
            if let Some(escaped) = chars.next() {
                word.push(escaped);
            } else {
                word.push(character);
            }
            started = true;
            continue;
        }
        if let Some(active) = quote {
            if character == active {
                quote = None;
            } else {
                word.push(character);
            }
            continue;
        }
        match character {
            '\'' | '"' => {
                quote = Some(character);
                started = true;
            }
            value if value.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            value => {
                word.push(value);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".to_string());
    }
    if started {
        words.push(word);
    }
    Ok(words)
}

fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '\'' | '"' if quote.is_none() => quote = Some(character),
            value if Some(value) == quote => quote = None,
            // A '#' inside quotes is a hashtag, not a comment.
            COMMENT if quote.is_none() => return &line[..index],
            _ => {}
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tasks_and_quotes() {
        let tasks = parse(
            r#"
            # comment
            check = cargo check
            run = cargo run -- "hello world" 'x y'
        "#,
        )
        .expect("valid tasks");
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].program, "cargo");
        assert_eq!(tasks[1].args, ["run", "--", "hello world", "x y"]);
    }

    #[test]
    fn keeps_tasks_in_file_order() {
        let tasks = parse("z = cat\na = cat\nm = cat\n").expect("valid tasks");
        let names: Vec<_> = tasks.iter().map(|task| task.name.as_str()).collect();
        assert_eq!(names, ["z", "a", "m"]);
    }

    #[test]
    fn rejects_duplicates_and_bad_lines() {
        assert!(parse("a = cat\na = cat").is_err());
        assert!(parse("a cat").is_err());
        assert!(parse("a = ").is_err());
        assert!(parse("bad name = cat").is_err());
    }

    #[test]
    fn keeps_empty_quoted_arguments() {
        assert_eq!(split_words("cat \"\"").expect("words"), ["cat", ""]);
    }

    #[test]
    fn detects_unterminated_quotes() {
        assert!(split_words("cat \"oops").is_err());
    }

    #[test]
    fn escapes_quotes_inside_quoted_words() {
        assert_eq!(
            split_words(r#"cat "a \" b" 'c \' d'"#).expect("words"),
            ["cat", "a \" b", "c ' d"]
        );
    }

    #[test]
    fn escaped_hashes_are_not_comments() {
        let line = strip_comment(r"run = echo \#tag # trailing");
        assert_eq!(
            split_words(line).expect("words"),
            ["run", "=", "echo", "#tag"]
        );
    }

    #[test]
    fn comments_inside_quotes_are_kept() {
        let line = strip_comment("cat '#x' # y");
        assert_eq!(split_words(line).expect("words"), ["cat", "#x"]);
    }

    #[test]
    fn defaults_have_unique_names() {
        let tasks = defaults();
        let names: std::collections::BTreeSet<_> =
            tasks.iter().map(|task| task.name.as_str()).collect();
        assert_eq!(names.len(), tasks.len());
        assert_eq!(tasks[0].command_line(), "cargo check");
        assert!(find(&tasks, "test").is_some());
    }

    #[test]
    fn formats_a_command_line() {
        let task = Task::parse("run", "cargo run -- x").expect("task");
        assert_eq!(task.command_line(), "cargo run -- x");
    }
}
