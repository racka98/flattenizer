// Prevents a console window from popping up alongside the GUI on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;

use crate::core::{FlattenOptions, RootNameMode, SeparatorStyle};
use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 700.0])
            .with_min_inner_size([440.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Flattenizer",
        native_options,
        Box::new(|cc| {
            setup_style(&cc.egui_ctx);
            Ok(Box::new(FlattenizerApp::default()))
        }),
    )
}

/// Applies a modern dark theme: custom accent color, generous rounding,
/// roomier spacing, and a slightly larger base font.
fn setup_style(ctx: &egui::Context) {
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();

    // Spacing: airier than egui's cramped defaults.
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(16);
    style.spacing.indent = 18.0;

    // Rounded corners everywhere for a softer, modern look.
    let rounding = egui::CornerRadius::same(8);

    // Dark theme base.
    style.visuals = egui::Visuals::dark();
    style.visuals.window_corner_radius = egui::CornerRadius::same(12);
    style.visuals.widgets.noninteractive.corner_radius = rounding;
    style.visuals.widgets.inactive.corner_radius = rounding;
    style.visuals.widgets.hovered.corner_radius = rounding;
    style.visuals.widgets.active.corner_radius = rounding;
    style.visuals.widgets.open.corner_radius = rounding;

    // Custom accent: warm amber, fitting for a "hive" adjacent tool.
    let accent = egui::Color32::from_rgb(245, 166, 35);
    style.visuals.selection.bg_fill = accent;
    style.visuals.selection.stroke.color = egui::Color32::BLACK;
    style.visuals.widgets.hovered.bg_stroke.color = accent;
    style.visuals.widgets.active.bg_stroke.color = accent;
    style.visuals.widgets.active.fg_stroke.color = accent;

    // Panel / window background: slightly lighter than pure black for depth.
    style.visuals.panel_fill = egui::Color32::from_rgb(24, 24, 27);
    style.visuals.window_fill = egui::Color32::from_rgb(24, 24, 27);
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(18, 18, 20);
    style.visuals.faint_bg_color = egui::Color32::from_rgb(32, 32, 36);

    // Slightly larger base text for readability.
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(14.5, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(14.5, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(24.0, egui::FontFamily::Proportional),
    );

    ctx.set_style_of(egui::Theme::Dark, style);
    ctx.set_theme(egui::Theme::Dark);
}

/// Wraps content in a rounded, slightly raised "card" panel — the main
/// visual grouping device used throughout the UI.
fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(30, 30, 34))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(14))
        .show(ui, add_contents);
}

/// A small uppercase section label used above each card's contents.
fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .small()
            .strong()
            .color(egui::Color32::from_rgb(245, 166, 35)),
    );
    ui.add_space(4.0);
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
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(16, 12))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🐝").size(26.0));
                ui.heading("Flattenizer");
            });
            ui.label(
                egui::RichText::new(
                    "Copy every file from a folder tree into one flat folder, renaming each file to include its original path.",
                )
                .weak(),
            );
            ui.add_space(14.0);

            // --- Source folder ---
            card(ui, |ui| {
                section_label(ui, "Source folder");
                ui.horizontal(|ui| {
                    if ui.button("📁  Choose folder…").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.source_dir = Some(path);
                        }
                    }
                    match &self.source_dir {
                        Some(p) => ui.label(
                            egui::RichText::new(p.display().to_string())
                                .color(egui::Color32::from_rgb(200, 200, 205)),
                        ),
                        None => ui.label(egui::RichText::new("No folder selected").weak().italics()),
                    };
                });
            });

            ui.add_space(10.0);

            // --- Output settings ---
            card(ui, |ui| {
                section_label(ui, "Output");
                ui.label("Folder name (created inside the source folder):");
                ui.text_edit_singleline(&mut self.output_folder_name);

                ui.add_space(10.0);

                ui.label("Prefix root folder name:");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.root_name_choice, RootNameChoice::None, "None");
                    ui.selectable_value(
                        &mut self.root_name_choice,
                        RootNameChoice::FolderName,
                        "Folder name",
                    );
                    ui.selectable_value(&mut self.root_name_choice, RootNameChoice::Custom, "Custom");
                });
                if self.root_name_choice == RootNameChoice::Custom {
                    ui.text_edit_singleline(&mut self.custom_root_name);
                }

                ui.add_space(10.0);

                ui.label("Separator between path segments:");
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.separator_choice,
                        SeparatorChoice::SlashLike,
                        "Slash-like  ∕",
                    );
                    ui.selectable_value(
                        &mut self.separator_choice,
                        SeparatorChoice::Underscore,
                        "Underscore  _",
                    );
                    ui.selectable_value(&mut self.separator_choice, SeparatorChoice::Custom, "Custom");
                });
                if self.separator_choice == SeparatorChoice::Custom {
                    ui.text_edit_singleline(&mut self.custom_separator);
                }

                ui.add_space(6.0);
                let sep = match self.separator_choice {
                    SeparatorChoice::SlashLike => "\u{2215}",
                    SeparatorChoice::Underscore => "_",
                    SeparatorChoice::Custom => {
                        if self.custom_separator.is_empty() {
                            "_"
                        } else {
                            &self.custom_separator
                        }
                    }
                };
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(18, 18, 20))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "components{sep}ble_service{sep}include{sep}ble_service.h"
                            ))
                            .monospace()
                            .color(egui::Color32::from_rgb(245, 166, 35)),
                        );
                    });
            });

            ui.add_space(10.0);

            // --- Ignore rules ---
            card(ui, |ui| {
                section_label(ui, "Ignore rules");
                ui.label("Folders (comma-separated names):");
                ui.text_edit_singleline(&mut self.ignored_folders_text);

                ui.add_space(8.0);
                ui.label("File names (comma-separated):");
                ui.text_edit_singleline(&mut self.ignored_files_text);

                ui.add_space(8.0);
                ui.label("File extensions (comma-separated, no dot):");
                ui.text_edit_singleline(&mut self.ignored_extensions_text);
            });

            ui.add_space(16.0);

            // --- Run ---
            let can_run = self.source_dir.is_some();
            ui.horizontal(|ui| {
                let preview_btn = egui::Button::new("Preview");
                if ui.add_enabled(can_run, preview_btn).clicked() {
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

                let run_label = egui::RichText::new("Run")
                    .strong()
                    .color(egui::Color32::BLACK);
                let run_btn = egui::Button::new(run_label)
                    .fill(egui::Color32::from_rgb(245, 166, 35))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(245, 166, 35)))
                    .min_size(egui::vec2(90.0, 0.0));
                if ui.add_enabled(can_run, run_btn).clicked() {
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

            if !self.status.is_empty() {
                ui.add_space(10.0);
                let is_error = self.status.starts_with("Error:");
                let color = if is_error {
                    egui::Color32::from_rgb(240, 100, 100)
                } else {
                    egui::Color32::from_rgb(140, 220, 150)
                };
                ui.label(egui::RichText::new(&self.status).color(color));
            }

            if !self.last_run_errors.is_empty() {
                ui.add_space(8.0);
                card(ui, |ui| {
                    section_label(ui, &format!("{} error(s)", self.last_run_errors.len()));
                    egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                        for err in &self.last_run_errors {
                            ui.label(egui::RichText::new(err).small().weak());
                        }
                    });
                });
            }

            ui.add_space(8.0);
                    });
            });
    }
}
