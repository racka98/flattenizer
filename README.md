# Flattenizer

A small Rust/egui desktop utility that copies every file from a folder tree
into a single new flat folder, renaming each file to embed its original
relative path. Runs natively on Windows and Linux.

Example — this nested file:

```
components/ble_service/include/ble_service.h
```

becomes, with the default "slash-like" separator and the root folder name
prefixed:

```
myproject∕components∕ble_service∕include∕ble_service.h
```

or, with the underscore separator:

```
myproject_components_ble_service_include_ble_service.h
```

Windows and Linux both forbid a literal `/` or `\` inside a filename (they're
path separators), so the default separator uses `∕` (U+2215 DIVISION SLASH)
— a Unicode character that looks like a forward slash but is legal in a
filename. You can also switch to a plain underscore or type any custom
separator string.

Source files are never modified, moved, or deleted — this only copies.

## Features

- **Native folder picker** to choose the source folder.
- **Configurable output folder name**, created inside the source folder.
- **Root folder name prefix**: choose to prepend the source folder's own
  name, a custom string, or nothing, to every flattened filename.
- **Configurable separator**: slash-like (default), underscore, or a custom
  string, with a live preview.
- **Ignore rules**:
  - Respects `.gitignore`, `.git/info/exclude`, and your global gitignore by
    default (toggleable) — same pattern matching engine used by ripgrep, so
    it correctly handles negation, nested `.gitignore` files, and
    directory-only patterns. This works even if the source folder isn't
    inside an actual git repository.
  - Manual comma-separated ignore lists for folder names, specific file
    names, and file extensions.
- **Preview**: shows how many files would be copied without touching disk.
- **Run**: copies files into the output folder. Re-running always produces a
  clean result — the output folder is deleted and recreated from scratch
  each time, so stale files from a previous run with different settings
  never linger.
- Filenames are sanitized to strip characters Windows disallows
  (`\ / : * ? " < > |`).

## Build

Requires Rust (install via https://rustup.rs if you don't have it).

```
cargo build --release
```

The binary will be at `target/release/flattenizer.exe` (Windows) or
`target/release/flattenizer` (Linux).

## Run during development

```
cargo run
```

Faster to iterate with than `--release` since it skips optimizations. Use
`cargo check` for a quick compile check without running, or `cargo test` to
run the unit tests without launching the GUI.

## Usage

1. Click "Choose folder…" and pick the root folder you want to flatten.
2. Set the output folder name (created inside the chosen folder). Note: this
   folder is fully replaced on every run.
3. Choose how the root folder name should be prefixed: none, the folder's
   own name, or a custom string.
4. Choose a separator style, or type a custom one.
5. Optionally toggle `.gitignore` support and/or list folders, file names,
   or file extensions to ignore.
6. Click "Preview" to see how many files would be copied, or "Run" to
   actually do it.

## Packaging for distribution (Windows)

The release profile in `Cargo.toml` is tuned for size (`opt-level = "z"`,
LTO, stripped symbols, panic=abort). A typical release build should land in
the low single-digit MB range with no external runtime dependencies — just
hand out the single `.exe`.

## Project layout

- `src/core.rs` — all traversal, filtering, and rename logic. No GUI
  dependencies, so it's unit tested independently (`cargo test`).
- `src/main.rs` — the `egui`/`eframe` GUI, including the app's dark theme
  and layout.
  