# Kawi Cockpit

Project Cockpit is a reference WASM plugin for Kawi. It shows a project dashboard, caches diagnostics, runs project tasks in the background, appends notes, and writes a Markdown report.

The plugin targets `kawi:plugin@0.12.0` and is built with `kawi-plugin-sdk`.

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
| `cockpit.open` | `<leader>co` | Open or refresh the panel |
| `cockpit.refresh` | none | Refresh the panel and task state |
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

Each line is `name = command arguments`. Blank lines and `#` comments are ignored. Single and double quotes group words, and backslash escapes the next character.

Without a task file the plugin offers:

- `cargo check`
- `cargo test`
- `cargo build`
- `cargo fmt --check`
- `cargo clippy`
- `git status --short`

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
