use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Preset separator styles for joining path segments into a flat filename.
/// Windows forbids literal '/' and '\' in filenames, so `SlashLike` uses a
/// visually similar Unicode character (U+2215 DIVISION SLASH) that is legal
/// in a filename on both Windows and Linux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeparatorStyle {
    /// Plain underscore: `components_ble_service_include_ble_service.h`
    Underscore,
    /// Unicode division slash, visually close to '/':
    /// `components∕ble_service∕include∕ble_service.h`
    SlashLike,
    /// User-supplied string.
    Custom(String),
}

impl SeparatorStyle {
    pub fn as_str(&self) -> &str {
        match self {
            SeparatorStyle::Underscore => "_",
            SeparatorStyle::SlashLike => "\u{2215}", // ∕ DIVISION SLASH
            SeparatorStyle::Custom(s) => {
                if s.is_empty() {
                    "_"
                } else {
                    s.as_str()
                }
            }
        }
    }
}

/// How to handle the root folder's own name when building the prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootNameMode {
    /// Don't include the root folder name at all.
    None,
    /// Use the root folder's actual name.
    UseFolderName,
    /// Use a user-supplied custom string instead.
    Custom(String),
}

/// User-configurable options for a flatten run.
#[derive(Debug, Clone)]
pub struct FlattenOptions {
    /// The folder whose contents will be flattened.
    pub source_dir: PathBuf,
    /// Name of the new folder created inside `source_dir` to hold the output.
    pub output_folder_name: String,
    /// How to prefix the root folder name onto every path.
    pub root_name_mode: RootNameMode,
    /// Folder names to skip entirely (exact match, case-insensitive), e.g. "node_modules", ".git".
    pub ignored_folder_names: HashSet<String>,
    /// File names to skip entirely (exact match, case-insensitive), e.g. ".DS_Store".
    pub ignored_file_names: HashSet<String>,
    /// File extensions to skip (without the dot, case-insensitive), e.g. "tmp", "log".
    pub ignored_extensions: HashSet<String>,
    /// Separator used when joining path segments into the new filename.
    pub separator_style: SeparatorStyle,
}

impl Default for FlattenOptions {
    fn default() -> Self {
        Self {
            source_dir: PathBuf::new(),
            output_folder_name: "flattened".to_string(),
            root_name_mode: RootNameMode::UseFolderName,
            ignored_folder_names: HashSet::new(),
            ignored_file_names: HashSet::new(),
            ignored_extensions: HashSet::new(),
            separator_style: SeparatorStyle::SlashLike,
        }
    }
}

/// A single planned rename: original file path -> new flat file name.
#[derive(Debug, Clone)]
pub struct PlannedRename {
    pub source_path: PathBuf,
    pub new_file_name: String,
}

/// Result of a flatten run.
#[derive(Debug, Default)]
pub struct FlattenSummary {
    pub copied: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

/// Returns true if a given path component (folder or file name) should be ignored.
fn is_folder_ignored(name: &str, opts: &FlattenOptions) -> bool {
    opts.ignored_folder_names.contains(&lower(name))
}

fn is_file_ignored(file_name: &str, opts: &FlattenOptions) -> bool {
    if opts.ignored_file_names.contains(&lower(file_name)) {
        return true;
    }
    if let Some(ext) = Path::new(file_name).extension().and_then(|e| e.to_str()) {
        if opts.ignored_extensions.contains(&lower(ext)) {
            return true;
        }
    }
    false
}

/// Sanitize a single path component so it's safe to embed in a flat filename.
/// Windows disallows: \ / : * ? " < > |
fn sanitize_component(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            other => other,
        })
        .collect()
}

/// Walks `opts.source_dir`, applying ignore rules, and builds the list of
/// planned renames without touching the filesystem. Useful for previewing
/// and for unit testing.
pub fn plan(opts: &FlattenOptions) -> Result<Vec<PlannedRename>, String> {
    if !opts.source_dir.is_dir() {
        return Err(format!(
            "Source path does not exist or is not a directory: {}",
            opts.source_dir.display()
        ));
    }

    let output_folder_name = opts.output_folder_name.trim();
    if output_folder_name.is_empty() {
        return Err("Output folder name cannot be empty.".to_string());
    }
    let output_dir = opts.source_dir.join(output_folder_name);

    let root_name_lower = opts
        .source_dir
        .file_name()
        .map(|n| lower(&n.to_string_lossy()));

    let mut planned = Vec::new();

    let walker = WalkDir::new(&opts.source_dir).into_iter().filter_entry(|entry| {
        // Never descend into the output folder itself (avoids feedback loops
        // on repeated runs) or into ignored folders.
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy();
            if entry.path() == output_dir {
                return false;
            }
            if is_folder_ignored(&name, opts) {
                return false;
            }
        }
        true
    });

    for entry in walker {
        let entry = entry.map_err(|e| format!("Walk error: {e}"))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if is_file_ignored(&file_name, opts) {
            continue;
        }

        let rel_path = path
            .strip_prefix(&opts.source_dir)
            .map_err(|e| format!("Path prefix error: {e}"))?;

        // Build the list of path segments (parent folders + file name).
        let mut segments: Vec<String> = rel_path
            .components()
            .map(|c| sanitize_component(&c.as_os_str().to_string_lossy()))
            .collect();

        // Prepend the root folder name if requested.
        match &opts.root_name_mode {
            RootNameMode::None => {}
            RootNameMode::UseFolderName => {
                if let Some(name) = &opts.source_dir.file_name() {
                    segments.insert(0, sanitize_component(&name.to_string_lossy()));
                }
            }
            RootNameMode::Custom(custom) => {
                let trimmed = custom.trim();
                if !trimmed.is_empty() {
                    segments.insert(0, sanitize_component(trimmed));
                }
            }
        }

        // Guard: if root_name_lower matches the first real path segment already
        // (rare edge case), we still just join everything; duplication is
        // harmless and predictable rather than "clever."
        let _ = &root_name_lower;

        let new_file_name = segments.join(opts.separator_style.as_str());

        planned.push(PlannedRename {
            source_path: path.to_path_buf(),
            new_file_name,
        });
    }

    Ok(planned)
}

/// Executes a flatten run: creates the output folder and copies every
/// planned file into it under its new flattened name. Source files are
/// never modified or deleted — this is a copy, not a move, so re-running
/// is always safe.
pub fn run(opts: &FlattenOptions) -> Result<FlattenSummary, String> {
    let planned = plan(opts)?;
    let output_dir = opts.source_dir.join(opts.output_folder_name.trim());

    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output folder: {e}"))?;

    let mut summary = FlattenSummary::default();

    for item in planned {
        let dest = output_dir.join(&item.new_file_name);
        match fs::copy(&item.source_path, &dest) {
            Ok(_) => summary.copied += 1,
            Err(e) => {
                summary.skipped += 1;
                summary.errors.push(format!(
                    "{}: {}",
                    item.source_path.display(),
                    e
                ));
            }
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_test_tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("components/ble_service/include")).unwrap();
        fs::write(
            root.join("components/ble_service/include/ble_service.h"),
            "content",
        )
        .unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("node_modules/pkg/index.js"), "content").unwrap();
        fs::write(root.join("readme.md"), "content").unwrap();
        fs::write(root.join("debug.log"), "content").unwrap();
        dir
    }

    #[test]
    fn flattens_nested_path_with_root_name() {
        let dir = make_test_tree();
        let root_name = dir.path().file_name().unwrap().to_string_lossy().to_string();

        let mut opts = FlattenOptions {
            source_dir: dir.path().to_path_buf(),
            root_name_mode: RootNameMode::UseFolderName,
            separator_style: SeparatorStyle::Underscore,
            ..Default::default()
        };
        opts.ignored_folder_names.insert("node_modules".to_string());
        opts.ignored_extensions.insert("log".to_string());

        let planned = plan(&opts).unwrap();
        let names: Vec<String> = planned.iter().map(|p| p.new_file_name.clone()).collect();

        let expected = format!(
            "{}_components_ble_service_include_ble_service.h",
            root_name
        );
        assert!(names.contains(&expected), "names: {:?}", names);
        assert!(!names.iter().any(|n| n.contains("node_modules")));
        assert!(!names.iter().any(|n| n.ends_with(".log")));
    }

    #[test]
    fn root_name_mode_none_omits_prefix() {
        let dir = make_test_tree();
        let opts = FlattenOptions {
            source_dir: dir.path().to_path_buf(),
            root_name_mode: RootNameMode::None,
            separator_style: SeparatorStyle::Underscore,
            ..Default::default()
        };
        let planned = plan(&opts).unwrap();
        let names: Vec<String> = planned.iter().map(|p| p.new_file_name.clone()).collect();
        assert!(names.contains(&"components_ble_service_include_ble_service.h".to_string()));
    }

    #[test]
    fn custom_root_name_is_used() {
        let dir = make_test_tree();
        let opts = FlattenOptions {
            source_dir: dir.path().to_path_buf(),
            root_name_mode: RootNameMode::Custom("hivetrace".to_string()),
            separator_style: SeparatorStyle::Underscore,
            ..Default::default()
        };
        let planned = plan(&opts).unwrap();
        let names: Vec<String> = planned.iter().map(|p| p.new_file_name.clone()).collect();
        assert!(names
            .contains(&"hivetrace_components_ble_service_include_ble_service.h".to_string()));
    }

    #[test]
    fn slash_like_separator_is_used_by_default() {
        let dir = make_test_tree();
        let opts = FlattenOptions {
            source_dir: dir.path().to_path_buf(),
            root_name_mode: RootNameMode::None,
            ..Default::default() // default separator_style is SlashLike
        };
        let planned = plan(&opts).unwrap();
        let names: Vec<String> = planned.iter().map(|p| p.new_file_name.clone()).collect();
        let expected = "components\u{2215}ble_service\u{2215}include\u{2215}ble_service.h";
        assert!(names.contains(&expected.to_string()), "names: {:?}", names);
    }

    #[test]
    fn output_folder_is_excluded_from_walk_on_rerun() {
        let dir = make_test_tree();
        let mut opts = FlattenOptions {
            source_dir: dir.path().to_path_buf(),
            root_name_mode: RootNameMode::None,
            ..Default::default()
        };
        opts.ignored_folder_names.insert("node_modules".to_string());
        opts.ignored_extensions.insert("log".to_string());

        run(&opts).unwrap();
        // Run again; output folder must not be walked into itself.
        let summary = run(&opts).unwrap();
        assert!(summary.errors.is_empty());
    }
}
