// Prevents a console window from popping up alongside the GUI on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;

use crate::core::{FlattenOptions, RootNameMode, SeparatorStyle};
use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 620.0])
            .with_min_inner_size([420.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Flattenizer",
        native_options,
        Box::new(|_cc| Ok(Box::new(FlattenizerApp::default()))),
    )
}

#[derive(PartialEq, Clone, Copy)]
enum RootNameChoice {
    None,
    FolderName,
    Custom,
}

#[derive(PartialEq, Clone, Copy)]
enum SeparatorChoice {
    SlashLike,
    Underscore,
    Custom,
}

struct FlattenizerApp {
    source_dir: Option<PathBuf>,
    output_folder_name: String,
    root_name_choice: RootNameChoice,
    custom_root_name: String,
    ignored_folders_text: String,
    ignored_files_text: String,
    ignored_extensions_text: String,
    separator_choice: SeparatorChoice,
    custom_separator: String,
    status: String,
    last_run_errors: Vec<String>,
}

impl Default for FlattenizerApp {
    fn default() -> Self {
        Self {
            source_dir: None,
            output_folder_name: "flattened".to_string(),
            root_name_choice: RootNameChoice::FolderName,
            custom_root_name: String::new(),
            ignored_folders_text: "node_modules, .git, target, build, .venv".to_string(),
            ignored_files_text: ".DS_Store, Thumbs.db".to_string(),
            ignored_extensions_text: "log, tmp".to_string(),
            separator_choice: SeparatorChoice::SlashLike,
            custom_separator: String::new(),
            status: String::new(),
            last_run_errors: Vec::new(),
        }
    }
}

/// Splits a comma-separated text field into a trimmed, lowercased set,
/// dropping empty entries.
fn parse_list(text: &str) -> std::collections::HashSet<String> {
    text.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

impl FlattenizerApp {
    fn build_options(&self) -> Option<FlattenOptions> {
        let source_dir = self.source_dir.clone()?;

        let root_name_mode = match self.root_name_choice {
            RootNameChoice::None => RootNameMode::None,
            RootNameChoice::FolderName => RootNameMode::UseFolderName,
            RootNameChoice::Custom => RootNameMode::Custom(self.custom_root_name.clone()),
        };

        let separator_style = match self.separator_choice {
            SeparatorChoice::SlashLike => SeparatorStyle::SlashLike,
            SeparatorChoice::Underscore => SeparatorStyle::Underscore,
            SeparatorChoice::Custom => SeparatorStyle::Custom(self.custom_separator.clone()),
        };

        Some(FlattenOptions {
            source_dir,
            output_folder_name: self.output_folder_name.clone(),
            root_name_mode,
            ignored_folder_names: parse_list(&self.ignored_folders_text),
            ignored_file_names: parse_list(&self.ignored_files_text),
            ignored_extensions: parse_list(&self.ignored_extensions_text),
            separator_style,
        })
    }
}

impl eframe::App for FlattenizerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Flattenizer");
            ui.label("Copy every file from a folder tree into one flat folder, renaming each file to include its original path.");
            ui.add_space(12.0);

            // --- Source folder picker ---
            ui.horizontal(|ui| {
                if ui.button("Choose folder…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.source_dir = Some(path);
                    }
                }
                match &self.source_dir {
                    Some(p) => ui.label(p.display().to_string()),
                    None => ui.label("(no folder selected)"),
                };
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // --- Output folder name ---
            ui.label("Output folder name (created inside the source folder):");
            ui.text_edit_singleline(&mut self.output_folder_name);

            ui.add_space(8.0);

            // --- Root name mode ---
            ui.label("Prefix root folder name:");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.root_name_choice, RootNameChoice::None, "None");
                ui.radio_value(
                    &mut self.root_name_choice,
                    RootNameChoice::FolderName,
                    "Use folder name",
                );
                ui.radio_value(&mut self.root_name_choice, RootNameChoice::Custom, "Custom");
            });
            if self.root_name_choice == RootNameChoice::Custom {
                ui.text_edit_singleline(&mut self.custom_root_name);
            }

            ui.add_space(8.0);

            // --- Separator ---
            ui.label("Separator between path segments:");
            ui.horizontal(|ui| {
                ui.radio_value(
                    &mut self.separator_choice,
                    SeparatorChoice::SlashLike,
                    "Slash-like ( ∕ )",
                );
                ui.radio_value(
                    &mut self.separator_choice,
                    SeparatorChoice::Underscore,
                    "Underscore ( _ )",
                );
                ui.radio_value(&mut self.separator_choice, SeparatorChoice::Custom, "Custom");
            });
            if self.separator_choice == SeparatorChoice::Custom {
                ui.text_edit_singleline(&mut self.custom_separator);
            }
            ui.label(
                egui::RichText::new(format!(
                    "Preview: components{sep}ble_service{sep}include{sep}ble_service.h",
                    sep = match self.separator_choice {
                        SeparatorChoice::SlashLike => "\u{2215}",
                        SeparatorChoice::Underscore => "_",
                        SeparatorChoice::Custom =>
                            if self.custom_separator.is_empty() {
                                "_"
                            } else {
                                &self.custom_separator
                            },
                    }
                ))
                .weak()
                .small(),
            );

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // --- Ignore lists ---
            ui.label("Ignore folders (comma-separated names):");
            ui.text_edit_singleline(&mut self.ignored_folders_text);

            ui.add_space(6.0);
            ui.label("Ignore specific file names (comma-separated):");
            ui.text_edit_singleline(&mut self.ignored_files_text);

            ui.add_space(6.0);
            ui.label("Ignore file extensions (comma-separated, no dot):");
            ui.text_edit_singleline(&mut self.ignored_extensions_text);

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // --- Run ---
            let can_run = self.source_dir.is_some();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_run, egui::Button::new("Preview"))
                    .clicked()
                {
                    if let Some(opts) = self.build_options() {
                        match core::plan(&opts) {
                            Ok(planned) => {
                                self.status = format!("{} file(s) would be copied.", planned.len());
                                self.last_run_errors.clear();
                            }
                            Err(e) => {
                                self.status = format!("Error: {e}");
                            }
                        }
                    }
                }

                if ui
                    .add_enabled(can_run, egui::Button::new("Run"))
                    .clicked()
                {
                    if let Some(opts) = self.build_options() {
                        match core::run(&opts) {
                            Ok(summary) => {
                                self.status = format!(
                                    "Done. Copied {} file(s), {} skipped.",
                                    summary.copied, summary.skipped
                                );
                                self.last_run_errors = summary.errors;
                            }
                            Err(e) => {
                                self.status = format!("Error: {e}");
                                self.last_run_errors.clear();
                            }
                        }
                    }
                }
            });

            ui.add_space(8.0);
            if !self.status.is_empty() {
                ui.label(&self.status);
            }

            if !self.last_run_errors.is_empty() {
                ui.add_space(6.0);
                ui.label(format!("{} error(s):", self.last_run_errors.len()));
                egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                    for err in &self.last_run_errors {
                        ui.label(err);
                    }
                });
            }
        });
    }
}
