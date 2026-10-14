mod loaders;
mod math;
mod title_bar;
mod trace_plotter;
mod wave;
mod workspace;

use crate::loaders::{
    dialog_box_ok, load_trace_files, load_trace_files_labelled, open_file_explorer_multi,
    open_folder_explorer, LoadedTraces,
};
use crate::title_bar::TitleBarAction;
use crate::trace_plotter::trace_plotter::TracePlotter;
use crate::workspace::{Workspace, WorkspaceAction};
use bincode::config;
use eframe::egui::Frame;
use egui::{CentralPanel, Color32, RichText, SidePanel};
use egui_modal::{Icon};
use log::{error, LevelFilter};
use rand::{rng, Rng};
use simple_logger::SimpleLogger;
use std::fs::File;
use std::io;
use std::io::{Write};
use std::path::PathBuf;
use std::time::Instant;
use rand::distr::Alphanumeric;
use zstd::encode_all;

struct App {
    trace_plotters: Vec<(TracePlotter, bool)>,
    file_error: String,
    workspace: Option<Workspace>,
    show_navigator: bool,
    /// Title of the plotter that navigator clicks add traces to (the last one in front)
    active_plotter: Option<String>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let root_panel = Frame {
            inner_margin: 0.0.into(),
            fill: ctx.style().visuals.window_fill(),
            rounding: 15.0.into(),
            stroke: ctx.style().visuals.widgets.noninteractive.fg_stroke,
            ..Default::default()
        };

        let content_panel = Frame {
            inner_margin: 10.0.into(),
            fill: Color32::TRANSPARENT,
            ..Default::default()
        };

        CentralPanel::default().frame(root_panel).show(ctx, |ui| {
            let title_bar_action = title_bar::custom_title_bar(ui);

            let err = dialog_box_ok(ui, "file_error", &self.file_error, Icon::Warning);

            match title_bar_action {
                TitleBarAction::OpenWorkspace => self.pick_workspace(),
                TitleBarAction::ToggleSideBar => self.show_navigator = !self.show_navigator,
                TitleBarAction::None => {}
            }

            // Workspace navigator on the left
            let mut workspace_action = None;
            if self.show_navigator {
                SidePanel::left("workspace_navigator")
                    .resizable(true)
                    .default_width(240.0)
                    .width_range(150.0..=600.0)
                    .frame(content_panel)
                    .show_inside(ui, |ui| {
                        workspace_action = self.render_navigator(ui);
                    });
            }

            let central = CentralPanel::default()
                .frame(content_panel)
                .show_inside(ui, |ui| {
                    ui.label("Hello from the root viewport");

                    if ui.button("Open new Trace Plotter").clicked() {
                        // Let the user pick one or more .bin / .csv trace files
                        if let Some(file_paths) = open_file_explorer_multi() {
                            match load_trace_files(&file_paths) {
                                Ok(loaded) => {
                                    let label = if file_paths.len() == 1 {
                                        loaded.names.first().cloned().unwrap_or_default()
                                    } else {
                                        format!("{} files", file_paths.len())
                                    };
                                    self.open_trace_plotter(loaded, label)
                                }
                                Err(e) => {
                                    error!("Failed to open files {:?}: {:?}", file_paths, e);
                                    self.file_error =
                                        format!("Could not open the file:\n{}", e);
                                    err.open();
                                }
                            };
                        }
                    }
                });

            if let Some(action) = workspace_action {
                if let Err(e) = self.handle_workspace_action(action) {
                    error!("Failed to open trace from workspace: {:?}", e);
                    self.file_error = format!("Could not open the file:\n{}", e);
                    err.open();
                }
            }

            self.trace_plotters.retain(|(_, show)| *show);

            let plot_area = central.response.rect;
            for (index, (ref mut trace_plotter, ref mut show)) in
                self.trace_plotters.iter_mut().enumerate()
            {
                trace_plotter.render(ctx, show, plot_area, index);
                //println!("{:?}",trace_plotter.get_selected_data_range_indices())
            }

            // Remember which plotter is in front so navigator clicks go there
            if let Some((plotter, _)) = self.trace_plotters.iter().find(|(p, _)| p.is_focused()) {
                self.active_plotter = Some(plotter.title().to_string());
            }
        });
    }
}

fn generate_random_string(length: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}
impl App {
    fn new() -> Self {
        App {
            trace_plotters: vec![],
            file_error: String::new(),
            workspace: None,
            show_navigator: true,
            active_plotter: None,
        }
    }

    fn pick_workspace(&mut self) {
        if let Some(folder) = open_folder_explorer() {
            self.workspace = Some(Workspace::open(PathBuf::from(folder)));
            self.show_navigator = true;
        }
    }

    /// Index of the plotter that navigator clicks should add traces to.
    fn active_plotter_index(&self) -> Option<usize> {
        self.active_plotter
            .as_ref()
            .and_then(|title| {
                self.trace_plotters
                    .iter()
                    .position(|(p, _)| p.title() == title)
            })
            .or_else(|| self.trace_plotters.len().checked_sub(1))
    }

    fn render_navigator(&mut self, ui: &mut egui::Ui) -> Option<WorkspaceAction> {
        let mut action = None;
        let active = self.active_plotter_index();

        ui.horizontal(|ui| {
            ui.strong("Workspace");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.workspace.is_some() {
                    if ui.small_button("✖").on_hover_text("Close workspace").clicked() {
                        self.workspace = None;
                    }
                    if ui.small_button("⟳").on_hover_text("Refresh").clicked() {
                        if let Some(ws) = &mut self.workspace {
                            ws.refresh();
                        }
                    }
                }
            });
        });

        let Some(workspace) = &mut self.workspace else {
            ui.add_space(8.0);
            ui.label("No workspace open.");
            if ui.button("Open Workspace…").clicked() {
                self.pick_workspace();
            }
            return None;
        };

        ui.label(RichText::new(workspace.name()).weak())
            .on_hover_text(workspace.root().to_string_lossy());
        let target = match active {
            Some(i) => format!("Click a file to add it to: {}", self.trace_plotters[i].0.title()),
            None => "Click a file to open it in a new plotter".to_string(),
        };
        ui.small(target);
        ui.separator();

        let plotters = &self.trace_plotters;
        let is_loaded = |path: &std::path::Path| {
            active
                .map(|i| plotters[i].0.contains_source(&path.to_string_lossy()))
                .unwrap_or(false)
        };
        if let Some(a) = workspace.render(ui, &is_loaded) {
            action = Some(a);
        }
        action
    }

    fn handle_workspace_action(
        &mut self,
        action: WorkspaceAction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(workspace) = &self.workspace else {
            return Ok(());
        };

        let (paths, new_plotter) = match action {
            WorkspaceAction::AddToPlotter(paths) => (paths, false),
            WorkspaceAction::OpenInNewPlotter(paths) => (paths, true),
            WorkspaceAction::RemoveFromPlotter(paths) => {
                if let Some(i) = self.active_plotter_index() {
                    for path in &paths {
                        self.trace_plotters[i]
                            .0
                            .remove_source(&path.to_string_lossy());
                    }
                }
                return Ok(());
            }
        };

        // Name traces by their path inside the workspace (e.g. "run1/trace0.csv")
        let files: Vec<(String, String)> = paths
            .iter()
            .map(|p| (p.to_string_lossy().into_owned(), workspace.relative_name(p)))
            .collect();
        let label = if files.len() == 1 {
            files[0].1.clone()
        } else {
            format!("{} files", files.len())
        };

        let loaded = load_trace_files_labelled(&files)?;

        match self.active_plotter_index() {
            Some(i) if !new_plotter => self.trace_plotters[i].0.add_traces(loaded),
            _ => self.open_trace_plotter(loaded, label),
        }
        Ok(())
    }

    fn open_trace_plotter(&mut self, loaded: LoadedTraces, label: String) {
        let start_time = Instant::now();

        // let _ = math::static_align(
        //     0,
        //     &trace_data,
        //     867..992,
        //     200,
        //     0.50
        // );

        println!("Time Taken: {:?}", start_time.elapsed());

        // Random suffix keeps window IDs unique
        let title = format!("{} ({})", label, generate_random_string(4));
        let trace_plotter = TracePlotter::new(loaded, title.clone());

        self.trace_plotters.push((trace_plotter, true));
        self.active_plotter = Some(title);

        //println!("Shifts: {:?}", max_alignments);
        // let trace_plotter = TracePlotter::new(shifts, title);
        //
        // self.trace_plotters.push((trace_plotter, true));
    }
}

fn main() -> Result<(), eframe::Error> {
    // // Measure the execution time of loading data from CSV
    // let start_csv = Instant::now();
    // let data = load_csv("data/100x100XYAquisition.txt").unwrap();
    // let duration_csv = start_csv.elapsed();
    //
    //  println!("Time taken to load CSV: {:?} - {}", duration_csv,data.len());
    // //
    // write_to_file(&data, "data.bin").unwrap();

    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(true)
            .with_inner_size([1250.0, 750.0])
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "SC-Analysis",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

#[allow(dead_code)]
fn write_to_file(data: &[Vec<(f64, f64)>], file_path: &str) -> io::Result<()> {
    let config = config::standard();

    let encoded: Vec<u8> = bincode::encode_to_vec(data, config).unwrap();
    let mut file = File::create(file_path)?;
    let compressed = encode_all(&encoded[..], 0).unwrap(); // Default compression level
    file.write_all(&compressed)?;
    Ok(())
}
