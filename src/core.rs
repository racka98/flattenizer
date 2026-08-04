use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Preset separator styles for joining path segments into a flat filename.
/// Windows forbids literal '/' and '\' in filenames, so `SlashLike` uses a
/// visually similar Unicode character (U+2215 DIVISION SLASH) that is legal
/// in a filename on both Windows and Linux.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// If true, honor .gitignore files (and .git/info/exclude, global
    /// gitignore) found in the source tree, skipping anything they exclude.
    pub respect_gitignore: bool,
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
            respect_gitignore: true,
        }
    }
}

/// Filename for a per-folder config file, saved in the root of the folder
/// being flattened. Distinct name so it's unambiguous if checked into git.
pub const CONFIG_FILE_NAME: &str = ".flattenizerrc";

/// The persistable subset of `FlattenOptions` — everything except
/// `source_dir`, which is implicit (the config lives inside that folder).
/// Serialized as JSON to `.flattenizerrc` in the root of the target folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default = "default_output_folder_name")]
    pub output_folder_name: String,
    #[serde(default = "default_root_name_mode")]
    pub root_name_mode: RootNameMode,
    #[serde(default)]
    pub ignored_folder_names: HashSet<String>,
    #[serde(default)]
    pub ignored_file_names: HashSet<String>,
    #[serde(default)]
    pub ignored_extensions: HashSet<String>,
    #[serde(default = "default_separator_style")]
    pub separator_style: SeparatorStyle,
    #[serde(default = "default_respect_gitignore")]
    pub respect_gitignore: bool,
}

fn default_output_folder_name() -> String {
    "flattened".to_string()
}
fn default_root_name_mode() -> RootNameMode {
    RootNameMode::UseFolderName
}
fn default_separator_style() -> SeparatorStyle {
    SeparatorStyle::SlashLike
}
fn default_respect_gitignore() -> bool {
    true
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            output_folder_name: default_output_folder_name(),
            root_name_mode: default_root_name_mode(),
            ignored_folder_names: HashSet::new(),
            ignored_file_names: HashSet::new(),
            ignored_extensions: HashSet::new(),
            separator_style: default_separator_style(),
            respect_gitignore: default_respect_gitignore(),
        }
    }
}

impl ProjectConfig {
    /// Builds a config from a full `FlattenOptions`, dropping `source_dir`.
    pub fn from_options(opts: &FlattenOptions) -> Self {
        Self {
            output_folder_name: opts.output_folder_name.clone(),
            root_name_mode: opts.root_name_mode.clone(),
            ignored_folder_names: opts.ignored_folder_names.clone(),
            ignored_file_names: opts.ignored_file_names.clone(),
            ignored_extensions: opts.ignored_extensions.clone(),
            separator_style: opts.separator_style.clone(),
            respect_gitignore: opts.respect_gitignore,
        }
    }

    /// Applies this config onto a `FlattenOptions`, keeping `source_dir` as-is.
    #[allow(dead_code)]
    pub fn apply_to(&self, opts: &mut FlattenOptions) {
        opts.output_folder_name = self.output_folder_name.clone();
        opts.root_name_mode = self.root_name_mode.clone();
        opts.ignored_folder_names = self.ignored_folder_names.clone();
        opts.ignored_file_names = self.ignored_file_names.clone();
        opts.ignored_extensions = self.ignored_extensions.clone();
        opts.separator_style = self.separator_style.clone();
        opts.respect_gitignore = self.respect_gitignore;
    }
}

/// Path to the config file for a given source folder.
pub fn config_path(source_dir: &Path) -> PathBuf {
    source_dir.join(CONFIG_FILE_NAME)
}

/// Loads `.flattenizerrc` from the root of `source_dir`, if present.
/// Returns `Ok(None)` if the file doesn't exist. A malformed file is
/// reported as an `Err` rather than silently ignored, so the user finds
/// out their config didn't load instead of quietly getting defaults.
pub fn load_config(source_dir: &Path) -> Result<Option<ProjectConfig>, String> {
    let path = config_path(source_dir);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let config: ProjectConfig = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;
    Ok(Some(config))
}

/// Saves the given config as `.flattenizerrc` in the root of `source_dir`,
/// overwriting any existing file there.
pub fn save_config(source_dir: &Path, config: &ProjectConfig) -> Result<(), String> {
    let path = config_path(source_dir);
    let text = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    Ok(())
}


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

    let ignored_folder_names = opts.ignored_folder_names.clone();

    let mut builder = WalkBuilder::new(&opts.source_dir);
    builder
        // Master switch: when false, disables every gitignore-family source
        // (.gitignore, .git/info/exclude, and the user's global gitignore).
        .git_ignore(opts.respect_gitignore)
        .git_exclude(opts.respect_gitignore)
        .git_global(opts.respect_gitignore)
        // Honor .gitignore files even if the source folder isn't inside an
        // actual git repository (no .git directory) — we just want the
        // pattern matching, not git repo detection.
        .require_git(false)
        // We do our own explicit ignore-list filtering below; don't also
        // apply .ignore files or hidden-file skipping implicitly.
        .ignore(false)
        .hidden(false)
        .parents(opts.respect_gitignore)
        .filter_entry(move |entry| {
            // Never descend into the output folder itself (avoids feedback
            // loops on repeated runs) or into ignored folders.
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let name = lower(&entry.file_name().to_string_lossy());
                if entry.path() == output_dir {
                    return false;
                }
                if ignored_folder_names.contains(&name) {
                    return false;
                }
            }
            true
        });

    let walker = builder.build();

    for entry in walker {
        let entry = entry.map_err(|e| format!("Walk error: {e}"))?;
        let is_file = entry.file_type().map(|ft| ft.is_file()).unwrap_or(false);
        if !is_file {
            continue;
        }

        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if is_file_ignored(&file_name, opts) {
            continue;
        }

        // Skip our own config file at the source root — it's tool metadata,
        // not project content, so it shouldn't end up in the flattened output.
        if file_name == CONFIG_FILE_NAME && path.parent() == Some(opts.source_dir.as_path()) {
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

/// Executes a flatten run: wipes any existing output folder, recreates it,
/// and copies every planned file into it under its new flattened name.
/// Source files are never modified or deleted — this is a copy, not a
/// move. Re-running always produces a clean result: the previous output
/// folder's contents are fully replaced rather than merged with stale
/// files from an earlier run (e.g. with different ignore rules).
pub fn run(opts: &FlattenOptions) -> Result<FlattenSummary, String> {
    let planned = plan(opts)?;
    let output_dir = opts.source_dir.join(opts.output_folder_name.trim());

    if output_dir.is_dir() {
        fs::remove_dir_all(&output_dir)
            .map_err(|e| format!("Failed to clear existing output folder: {e}"))?;
    }

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
    fn config_roundtrips_through_save_and_load() {
        let dir = tempfile::tempdir().unwrap();

        let mut config = ProjectConfig::default();
        config.output_folder_name = "out".to_string();
        config.root_name_mode = RootNameMode::Custom("hivetrace".to_string());
        config.separator_style = SeparatorStyle::Underscore;
        config.respect_gitignore = false;
        config.ignored_folder_names.insert("build".to_string());

        save_config(dir.path(), &config).unwrap();
        assert!(config_path(dir.path()).is_file());

        let loaded = load_config(dir.path()).unwrap().expect("config should load");
        assert_eq!(loaded.output_folder_name, "out");
        assert_eq!(loaded.root_name_mode, RootNameMode::Custom("hivetrace".to_string()));
        assert_eq!(loaded.separator_style, SeparatorStyle::Underscore);
        assert!(!loaded.respect_gitignore);
        assert!(loaded.ignored_folder_names.contains("build"));
    }

    #[test]
    fn load_config_returns_none_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_config(dir.path()).unwrap().is_none());
    }

    #[test]
    fn config_file_is_excluded_from_flattened_output() {
        let dir = make_test_tree();
        save_config(dir.path(), &ProjectConfig::default()).unwrap();

        let opts = FlattenOptions {
            source_dir: dir.path().to_path_buf(),
            root_name_mode: RootNameMode::None,
            separator_style: SeparatorStyle::Underscore,
            ..Default::default()
        };
        let planned = plan(&opts).unwrap();
        let names: Vec<String> = planned.iter().map(|p| p.new_file_name.clone()).collect();
        assert!(!names.iter().any(|n| n.contains(CONFIG_FILE_NAME)));
    }

    #[test]
    fn gitignore_rules_are_respected_by_default() {
        let dir = make_test_tree();
        fs::write(
            dir.path().join(".gitignore"),
            "debug.log\nnode_modules/\n",
        )
        .unwrap();

        let opts = FlattenOptions {
            source_dir: dir.path().to_path_buf(),
            root_name_mode: RootNameMode::None,
            separator_style: SeparatorStyle::Underscore,
            ..Default::default() // respect_gitignore: true
        };
        let planned = plan(&opts).unwrap();
        let names: Vec<String> = planned.iter().map(|p| p.new_file_name.clone()).collect();

        assert!(names.contains(&"readme.md".to_string()));
        assert!(!names.iter().any(|n| n.contains("debug.log")));
        assert!(!names.iter().any(|n| n.contains("node_modules")));
        // The .gitignore file itself shouldn't be swept up as a "content" file
        // by mistake (it's not excluded by its own rules, but confirm it's
        // present as ordinary content since we didn't ignore dotfiles).
    }

    #[test]
    fn gitignore_can_be_disabled() {
        let dir = make_test_tree();
        fs::write(dir.path().join(".gitignore"), "debug.log\n").unwrap();

        let opts = FlattenOptions {
            source_dir: dir.path().to_path_buf(),
            root_name_mode: RootNameMode::None,
            separator_style: SeparatorStyle::Underscore,
            respect_gitignore: false,
            ..Default::default()
        };
        let mut opts = opts;
        opts.ignored_folder_names.insert("node_modules".to_string());

        let planned = plan(&opts).unwrap();
        let names: Vec<String> = planned.iter().map(|p| p.new_file_name.clone()).collect();

        // debug.log would normally be gitignored, but with respect_gitignore
        // false it should come through since it isn't in our manual ignore lists.
        assert!(names.contains(&"debug.log".to_string()));
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
    fn rerun_wipes_stale_output_from_previous_run() {
        let dir = make_test_tree();

        // First run: underscore separator, root name prefix included.
        let opts_v1 = FlattenOptions {
            source_dir: dir.path().to_path_buf(),
            root_name_mode: RootNameMode::UseFolderName,
            separator_style: SeparatorStyle::Underscore,
            ..Default::default()
        };
        run(&opts_v1).unwrap();

        let output_dir = dir.path().join("flattened");
        let root_name = dir.path().file_name().unwrap().to_string_lossy().to_string();
        let stale_name = format!("{root_name}_readme.md");
        assert!(output_dir.join(&stale_name).exists());

        // Second run: no root name prefix, different separator. The old
        // "<root>_readme.md" file must be gone, not just left alongside
        // the newly-named output.
        let opts_v2 = FlattenOptions {
            source_dir: dir.path().to_path_buf(),
            root_name_mode: RootNameMode::None,
            separator_style: SeparatorStyle::Underscore,
            ..Default::default()
        };
        run(&opts_v2).unwrap();

        assert!(
            !output_dir.join(&stale_name).exists(),
            "stale file from previous run should have been wiped"
        );
        assert!(output_dir.join("readme.md").exists());
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
