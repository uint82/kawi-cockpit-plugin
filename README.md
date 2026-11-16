# Kawi Cockpit

Project Cockpit is a reference WASM plugin for Kawi. It shows a project dashboard, caches diagnostics, runs project tasks in the background, appends notes, and writes a Markdown report.

The plugin targets `kawi:plugin@0.13.0` and is built with `kawi-plugin-sdk`.

## Features

- Panel dashboard with the active file, cursor, mode, buffers, windows, diagnostics, task status, notes, and options.
- Cached diagnostics for every open buffer, with gutter signs and highlights for the active buffer.
- One background task at a time, using `process-jobs`. `cargo check` and `clippy` output is parsed into diagnostics.
- Task picker from `<root>/.cockpit/tasks`, falling back to cargo and git defaults.
- Buffer and window pickers, a mode prompt, split and close helpers, and next-diagnostic navigation.
- Notes appended to `<root>/.cockpit/notes.md`.
- Report written to `<root>/.cockpit/report.md`.
- A hidden API tour that exercises commands, keymaps, autocmds, timers, variables, options, diagnostics, process jobs, process runs, and WASI TCP.

## Build

Install the WASI target and component linker once:

```sh
rustup target add wasm32-wasip2
```

Build the component and update the manifest hash:

```sh
./build.sh
```

The script uses `kawi plugin build` when `kawi` is on `PATH`. Otherwise set `KAWI_MANIFEST` to a Kawi checkout:

```sh
KAWI_MANIFEST=/path/to/kawi/Cargo.toml ./build.sh
```

The built `cockpit.wasm` is intentionally ignored. Run the build before inspecting or installing.

`kawi plugin inspect .` prints the parsed manifest and component imports.

## Local install

Copy this directory under the Kawi development plugin directory:

```sh
mkdir -p "$XDG_CONFIG_HOME/kawi/plugins"
cp -R . "$XDG_CONFIG_HOME/kawi/plugins/cockpit"
```

Without `XDG_CONFIG_HOME`, the default is `~/.config/kawi/plugins`. Kawi marks development plugins with a startup warning. Restart Kawi after rebuilding the component.

## Commands

| Command | Default binding | Action |
| --- | --- | --- |
| `cockpit.open` | `<leader>co` | Open, refresh, and focus the panel |
| `cockpit.refresh` | none | Refresh the panel |
| `cockpit.focus-panel` | none | Focus the panel |
| `cockpit.tasks` | `<leader>ct` | Pick and run a project task |
| `cockpit.buffers` | none | Pick a named buffer |
| `cockpit.windows` | none | Pick a window |
| `cockpit.split` | none | Split the active window vertically |
| `cockpit.close-window` | none | Close the active window after confirmation |
| `cockpit.mode` | none | Set the active buffer mode |
| `cockpit.next-diagnostic` | none | Move to the next cached diagnostic |
| `cockpit.toggle-numbers` | none | Toggle the `number` option |
| `cockpit.save` | none | Save the active buffer |
| `cockpit.note` | none | Append a note and open the notes file |
| `cockpit.insert-task` | none | Insert a task command line at the cursor |
| `cockpit.report` | `<leader>cR` | Write and open the project report |
| `cockpit.log` | none | Open the task log overlay |
| `cockpit.stop` | none | Stop the running task after confirmation |

The API tour is registered at runtime as `cockpit.tour` on `<leader>cK`. It is intentionally absent from `plugin.toml`.

## Typical workflow

The walkthrough assumes the plugin is installed and a project file is open. A project root is the nearest ancestor with `.cockpit/`, `Cargo.toml`, or `.git`.

The panel can dock automatically on the first refresh after a project file becomes active. `cockpit.open` focuses it.

### 1. Open a project file

Open a file under the root, for example `src/main.rs`.

Expect:

- The status line to gain a segment like ` my-project  1E 2W `.
- The panel to dock above the task area, possibly without focus.
- The panel title to be the project directory name.

If no root is found, no panel opens. `cockpit.open` then reports `no project found for the active buffer`.

### 2. Open or focus the panel

Run `:cockpit.open` or `<leader>co`.

Expect the panel to receive focus and the message `cockpit opened`. The panel status shows counts like `2 buffers | 3 diagnostics | 6 tasks`, or the current job summary while a task runs.

A small panel can look like this:

```text
my-project  [normal]
File  src/main.rs
Lang  Rust   Cursor 12:5
Buffers (1/1)
  >main.rs*
Windows (1)
  >1 120x40 >main.rs*
Diagnostics (1E 2W)
  src/main.rs:12:5 E expected `;`
Job
  succeeded
Notes 2   number true wrap false
```

The `Diagnostics` header counts all cached diagnostics; the list below it shows the active buffer only.

The panel is read-only. The host owns scrolling and closing. While focused, `Esc` returns focus to the editor and `q` closes the panel. The panel reserves at least four editor rows and hides when the editor is too small. `panel_height` defaults to 12 and is clamped to 4 through 18; content beyond the height is clipped. If another plugin already owns the panel, expect `UI_BUSY`; run `:plugin panel close` first.

### 3. Pick a task

Run `:cockpit.tasks` or `<leader>ct`.

Expect a picker titled `Run project task`, with each task name and its command line.

- Cancel: nothing runs.
- Select a task: the message `cockpit: running check`, an accent-colored status line, and a panel status like `running check ... | 3 diagnostics`.
- Select while another task runs: `a cockpit task is already running`. The existing task continues.
- Empty task list: `no tasks are defined`.

Tasks run in the project root. Defaults and `.cockpit/tasks` are described under Tasks. The command must be allowed by `process.spawn`; the defaults are `cargo`, `git`, and `cat`. A disallowed command fails to start with the host error.

### 4. Watch the job

The panel updates every `refresh_ms`, default 2 seconds, and on editor events. The editor stays usable.

Expect a `Job` section with a status and the last output line:

- `running check`
- `succeeded`
- `failed with exit code 101`
- `exited`
- `killed by signal 15`
- `failed to start`
- `failed: <error>` when polling the host fails
- `stopping`

The job log keeps the newest 300 lines. Stderr lines are stored with an internal `! ` marker and shown red in the log overlay. Cargo diagnostics are parsed when the job finishes, not while it runs. Use `--message-format short` for parsing.

Stop with `:cockpit.stop`. Confirm `Stop the running task?`; expect `cockpit: stopping the task` and a `stopping` status. If no task runs, expect `cockpit: no task is running`.

### 5. Read the full output

Run `:cockpit.log`.

Expect a centered `Cockpit task log` overlay with a status like `42 lines`. The last line is selected and the view starts at the bottom. Stderr lines are red. Use `j`/`k`, arrows, PageUp/PageDown, `g`/`G`, or Home/End. `q` or `Esc` closes.

The overlay is a snapshot. Reopen it to see output that arrived later. If the job produced no output, expect `cockpit: the task log is empty`.

### 6. Fix diagnostics

For the active buffer, expect gutter signs `E`, `W`, `I`, and `H`, plus a one-character highlight. Host diagnostics update through events. Cargo diagnostics appear after a short-format task finishes.

Run `:cockpit.next-diagnostic`.

Expect the cursor to move to the next diagnostic on a later line, wrapping to the first when none remain after the cursor. If the active buffer has no cached diagnostics, expect `cockpit: no diagnostics for this buffer`.

The panel header counts all cached diagnostics; the list below it shows the active buffer only. Old diagnostics remain until the host replaces them or the buffer closes.

### 7. Append a note

Run `:cockpit.note`.

Expect a `Project note` prompt with `What should be remembered?` and a 500-character limit.

- Cancel or submit empty text: nothing is written.
- Submit text: cockpit appends to `.cockpit/notes.md`, opens the file, refreshes the panel, and reports `cockpit: note appended`.

The first entry looks like:

```markdown
# Project Notes

## note 1760000000
first note
```

The heading uses epoch seconds. Later entries are appended with one blank line between sections. The active file changes to the notes file; use `:cockpit.buffers` to switch back.

### 8. Write a report

Run `:cockpit.report` or `<leader>cR`.

Expect a `Cockpit report` progress display with `Writing report`, then `.cockpit/report.md` opens and `cockpit: report written` appears.

The report includes the project root, mode, open buffers, windows, all cached diagnostics, task count, note count, and the last job summary. It is overwritten on each run. It does not stop a running task.

Example:

```markdown
# Project Cockpit: my-project

Generated by the Kawi cockpit plugin.

## Project

- Root: `/home/you/my-project`
- Mode: `normal`

## Buffers

- Open: 2 (2 listed)
- `>main.rs*`
- ` lib.rs`

## Windows

- Open: 1
- `>1 120x40 >main.rs*`

## Diagnostics

- 3 total, 1 error, 2 warnings
- `src/main.rs:12:5 E expected ;`

## Tasks and notes

- Tasks: 6
- Notes: 2
- Last job: succeeded
```

The active file changes to the report file; use `:cockpit.buffers` to switch back.

## Workflow expectations

- The panel may open automatically; `cockpit.open` focuses it.
- One panel exists across all plugins. A second open returns `UI_BUSY`.
- One task runs at a time.
- The panel updates every 2 seconds and on editor events.
- The log overlay is a snapshot, not a live stream.
- Cargo diagnostics are parsed after a task finishes.
- Notes and reports open in the editor and change the active file.
- `cockpit.tour` is for API testing only and is intentionally not in `plugin.toml`.

## Tasks

Create `<root>/.cockpit/tasks`:

```text
check = cargo check --message-format short
test = cargo test
fmt = cargo fmt --check
clippy = cargo clippy --message-format short
status = git status --short
serve = cargo run -- --port 8080
```

Each line is `name = command arguments`. Blank lines and `#` comments are ignored. Single and double quotes group words, and backslash escapes the next character. Names cannot contain whitespace, and duplicate names are rejected.

A task file replaces the defaults; it is not merged. An empty or comment-only file defines no tasks, and `cockpit.tasks` reports `no tasks are defined`. An invalid file falls back to the defaults and logs the parse error to the plugin log.

Without a task file the plugin offers:

- `cargo check`
- `cargo test`
- `cargo build`
- `cargo fmt --check`
- `cargo clippy`
- `git status --short`

To parse cargo diagnostics, use `--message-format short` in the task command, as in the example above.

A task must resolve to a command listed under `process.spawn` in `plugin.toml`. The default list is `cargo`, `git`, and `cat`. Add another command only when you extend the manifest and rebuild.

Jobs run with the workspace as their working directory. Only one job runs at a time.

## Diagnostics

The plugin caches host diagnostics by buffer. For the active buffer it publishes one-cell signs `E`, `W`, `I`, and `H`, plus a one-character highlight. When a `cargo check` or `cargo clippy` task finishes with short-format output, matching diagnostics are also parsed into the cache. Old diagnostics remain until the host replaces them or the buffer closes.

## Report and notes

`cockpit.note` appends a timestamped section to `.cockpit/notes.md` and opens the file. `cockpit.report` writes a Markdown snapshot to `.cockpit/report.md` and opens it. Both files use relative-to-workspace paths inside `editor.buffers.write` and `fs.write` scopes.

## Settings

Settings are stored in `$XDG_CONFIG_HOME/kawi/plugins.toml` under `[cockpit]`.

| Setting | Type | Default | Range |
| --- | --- | --- | --- |
| `refresh_ms` | integer | `2000` | `1000` to `60000` |
| `panel_height` | integer | `12` | `4` to `18` |
| `max_buffers` | integer | `8` | `1` to `32` |
| `max_diagnostics` | integer | `6` | `1` to `20` |
| `show_statusline` | boolean | `true` | |

Edit them with `:plugin settings cockpit`.

## Capabilities

The manifest declares all 28 host capabilities to keep this repository useful as a reference. The notable scopes are:

- `editor.buffers.write`, `fs.read`, and `fs.write`: `$WORKSPACE`
- `editor.execute`: `file.save`
- `process.spawn`: `cargo`, `git`, `cat`
- `net.connect`: `example.com:80`

The main workflow uses the editor, UI, diagnostics, filesystem, and process capabilities. The network scope is used only by the hidden tour. Remove capabilities and code together if you fork this plugin for production.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo check --target wasm32-wasip2
./build.sh
```

The source separates pure logic from host calls so most behavior is covered by native tests. Files stay below 300 lines and each module has one responsibility.
