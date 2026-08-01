# Flattenizer

Copies every file from a folder tree into a single new flat folder, renaming
each file to embed its original relative path. Example:

```
components/ble_service/include/ble_service.h
```

becomes (default "slash-like" separator):

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
separator string in the GUI.

Source files are never modified, moved, or deleted — this only copies.
Re-running is safe; the output folder is automatically excluded from the
scan so it won't recursively flatten itself.

## Build

Requires Rust (install via https://rustup.rs if you don't have it).

```
cargo build --release
```

The binary will be at `target/release/flattenizer.exe` (Windows) or
`target/release/flattenizer` (Linux).

## Run tests

```
cargo test
```

## Usage

1. Click "Choose folder…" and pick the root folder you want to flatten.
2. Set the output folder name (created inside the chosen folder).
3. Choose how the root folder name should be prefixed: None, use the
   folder's own name, or a custom string.
4. Optionally list folders, file names, or file extensions to ignore
   (comma-separated).
5. Click "Preview" to see how many files would be copied, or "Run" to
   actually do it.

## Packaging for distribution (Windows)

For a smaller final binary, the release profile in `Cargo.toml` is already
tuned for size (`opt-level = "z"`, LTO, stripped symbols, panic=abort).
A typical release build should land in the 3-6 MB range with no external
runtime dependencies — just hand out the single `.exe`.
