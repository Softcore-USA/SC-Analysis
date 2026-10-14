use crate::loaders::LoadedTraces;
use crate::trace_plotter::plot_selection::PlotSelection;
use crate::trace_plotter::trace_plot::TracePlot;
use crate::trace_plotter::util::calculate_bounds;
use egui::{
    vec2, Area, ComboBox, Context, Id, Key, Rect, RichText, ScrollArea, Ui, UiKind, Vec2b, Window,
};
use egui_plot::{Legend, Plot, PlotResponse, PlotUi};
use std::ops::Range;

const MAX_NUMB_OF_POINTS: usize = 100_000;
/// Trace sets up to this size open in Overlay mode with every trace shown
const OVERLAY_BY_DEFAULT_MAX_TRACES: usize = 50;
/// Largest / smallest plot width in points (height is half the width)
const MAX_PLOT_WIDTH: f32 = 1000.0;
const MIN_PLOT_WIDTH: f32 = 400.0;
/// Width reserved for the trace checklist in Overlay mode
const TRACE_LIST_WIDTH: f32 = 200.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum ViewMode {
    /// Show a contiguous range of traces (scroll / arrow keys to move it)
    Range,
    /// Show any combination of traces, toggled individually, overlapped
    Overlay,
}

#[derive(Clone, Debug)]
pub struct TracePlotter {
    title: String,
    traces: Vec<TracePlot>,
    selected_plot_range: Range<usize>,
    plot_selection: PlotSelection,
    currently_selected: bool,
    view_mode: ViewMode,
    /// Plot width for the current frame, fitted to the space available
    plot_width: f32,
    /// Color index for the next added trace (never reused, so colors stay stable)
    next_color_index: usize,
}

impl TracePlotter {
    /// Draws the plotter window, kept inside `area` (the space beside the navigator).
    /// `index` staggers where new windows first appear.
    pub fn render(&mut self, ctx: &Context, open: &mut bool, area: Rect, index: usize) {
        let offset = 10.0 + 30.0 * (index % 8) as f32;
        let window = Window::new(&self.title)
            .default_pos(area.min + vec2(offset, offset))
            .constrain_to(area);

        // Fit the plot (and the trace list beside it) to the available width
        let list_width = if self.view_mode == ViewMode::Overlay {
            TRACE_LIST_WIDTH
        } else {
            0.0
        };
        self.plot_width = (area.width() - list_width - 40.0).clamp(MIN_PLOT_WIDTH, MAX_PLOT_WIDTH);
        let area = Area::new(Id::new(&self.title)).kind(UiKind::Window);

        let area_layer_id = area.layer();
        self.currently_selected = Some(area_layer_id) == ctx.top_layer_id();

        window.open(open).show(ctx, |ui| {
            // Handle key inputs to change the selected plot range
            let range_mode = self.view_mode == ViewMode::Range;

            if range_mode
                && ctx.input(|i| i.key_pressed(Key::ArrowUp))
                && self.selected_plot_range.end < self.traces.len()
                && self.currently_selected
            {
                self.selected_plot_range =
                    self.selected_plot_range.start + 1..self.selected_plot_range.end + 1;
            }

            if range_mode
                && ctx.input(|i| i.key_pressed(Key::ArrowDown))
                && self.selected_plot_range.start > 0
                && self.currently_selected
            {
                self.selected_plot_range =
                    self.selected_plot_range.start - 1..self.selected_plot_range.end - 1;
            }

            let mut should_scroll = range_mode;

            ui.horizontal(|ui| {
                ui.label("View:");
                let previous_mode = self.view_mode;
                ui.selectable_value(&mut self.view_mode, ViewMode::Range, "Range")
                    .on_hover_text("Show a range of consecutive traces");
                ui.selectable_value(&mut self.view_mode, ViewMode::Overlay, "Overlay")
                    .on_hover_text("Pick which traces to overlap");
                if previous_mode == ViewMode::Range
                    && self.view_mode == ViewMode::Overlay
                    && !self.traces.iter().any(|t| t.visible)
                {
                    // Start the overlay from whatever the range view was showing
                    for i in self.selected_plot_range.clone() {
                        self.traces[i].visible = true;
                    }
                }
                ui.separator();
            });

            if self.view_mode == ViewMode::Range {
                ui.horizontal(|ui| {
                    ui.label("Select Plot:");
                    ui.label("Start:");
                    let start_response = ComboBox::from_label("")
                        .selected_text(format!("Plot {}", self.selected_plot_range.start + 1))
                        .show_ui(ui, |ui| {
                            for (i, _) in self.traces.iter().enumerate() {
                                if ui
                                    .selectable_value(
                                        &mut self.selected_plot_range.start,
                                        i,
                                        format!("Plot {}", i + 1),
                                    )
                                    .clicked()
                                {
                                    self.selected_plot_range.start = i;
                                    if self.selected_plot_range.start > self.selected_plot_range.end
                                    {
                                        self.selected_plot_range.end =
                                            self.selected_plot_range.start + 1;
                                    }
                                }
                            }
                        });

                    ui.label("End:");
                    let end_response = ComboBox::from_label(" ")
                        .selected_text(format!("Plot {}", self.selected_plot_range.end))
                        .show_ui(ui, |ui| {
                            for (i, _) in self.traces.iter().enumerate() {
                                if ui
                                    .selectable_value(
                                        &mut self.selected_plot_range.end,
                                        i + 1,
                                        format!("Plot {}", i + 1),
                                    )
                                    .clicked()
                                {
                                    self.selected_plot_range.end = i + 1;
                                    if self.selected_plot_range.end < self.selected_plot_range.start
                                    {
                                        self.selected_plot_range.start =
                                            self.selected_plot_range.end - 1;
                                    }
                                }
                            }
                        });

                    should_scroll = !ComboBox::is_open(ctx, start_response.response.id)
                        && !ComboBox::is_open(ctx, end_response.response.id);
                });
            } else {
                ui.horizontal(|ui| {
                    let shown = self.traces.iter().filter(|t| t.visible).count();
                    ui.label(format!("Showing {} of {} traces", shown, self.traces.len()));
                    if ui.button("Show all").clicked() {
                        self.traces.iter_mut().for_each(|t| t.visible = true);
                    }
                    if ui.button("Hide all").clicked() {
                        self.traces.iter_mut().for_each(|t| t.visible = false);
                    }
                });
            }

            ui.horizontal_top(|ui| {
                self.render_plot(ui);
                if self.view_mode == ViewMode::Overlay {
                    self.render_trace_list(ui);
                }
            });
            if should_scroll {
                self.update_selected_plot_range(ui);
            }

            if let Some(range) = self.traces.first().and_then(|first| {
                self.plot_selection
                    .get_selected_data_range_indices(&first.trace)
            }) {
                ui.label(format!(
                    "Selected range start: {:.2}, end: {:.2}, Points: {}",
                    range.start,
                    range.end,
                    range.len()
                ));
            } else {
                ui.label("");
            }
        });
    }

    fn render_plot(&mut self, ui: &mut Ui) {
        let plot = Plot::new((&self.title, "trace_plot"))
            .width(self.plot_width)
            .legend(Legend::default())
            .view_aspect(2.0)
            .allow_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .auto_bounds(Vec2b::FALSE)
            .allow_double_click_reset(false)
            .allow_boxed_zoom(false);

        self.process_zoom_input(ui);

        let plot_responce: PlotResponse<()> = plot.show(ui, |plot_ui| {
            plot_ui.set_plot_bounds(self.plot_selection.get_plot_bounds());

            self.plot_traces(plot_ui);
            self.plot_selection.draw_selection_box(plot_ui);
        });
        self.plot_selection.update_selection(plot_responce);
    }

    fn plot_traces(&mut self, plot_ui: &mut PlotUi) {
        let shown: Vec<usize> = match self.view_mode {
            ViewMode::Range => self.selected_plot_range.clone().collect(),
            ViewMode::Overlay => (0..self.traces.len())
                .filter(|&i| self.traces[i].visible)
                .collect(),
        };
        let max_visible_points_per_trace = MAX_NUMB_OF_POINTS / shown.len().max(1);

        for i in shown {
            self.traces[i].draw_trace(plot_ui, max_visible_points_per_trace);
        }
    }

    /// Checkbox list (one per trace) shown beside the plot in Overlay mode.
    fn render_trace_list(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.set_min_width(180.0);
            ui.strong("Traces");
            let row_height = ui.spacing().interact_size.y;
            ScrollArea::vertical()
                .id_source((&self.title, "trace_list"))
                .max_height(self.plot_width / 2.0 - 30.0)
                .auto_shrink([false, true])
                .show_rows(ui, row_height, self.traces.len(), |ui, rows| {
                    for i in rows {
                        let trace = &mut self.traces[i];
                        let label = RichText::new(&trace.name).color(trace.color);
                        ui.checkbox(&mut trace.visible, label);
                    }
                });
        });
    }
    fn process_zoom_input(&mut self, ui: &Ui) {
        if ui.input(|input| input.key_pressed(Key::Enter)) {
            self.plot_selection.select_zoom();
        }

        if ui.input(|input| input.key_pressed(Key::Escape)) {
            self.plot_selection.revert_zoom();
        }
    }

    fn update_selected_plot_range(&mut self, ui: &Ui) {
        let modifiers = ui.input(|i| i.modifiers);
        let scroll_delta = ui.input(|i| i.raw_scroll_delta);

        if scroll_delta.length_sq() > 0.0 {
            if modifiers.command {
                // Control key is held, expand or shrink only one side
                if scroll_delta.y > 0.0 {
                    if self.selected_plot_range.end < self.traces.len() {
                        self.selected_plot_range =
                            self.selected_plot_range.start..self.selected_plot_range.end + 1;
                    }
                } else if scroll_delta.y < 0.0
                    && self.selected_plot_range.end > self.selected_plot_range.start + 1
                {
                    self.selected_plot_range =
                        self.selected_plot_range.start..self.selected_plot_range.end - 1;
                }
            } else {
                // Expand or shrink both sides
                if scroll_delta.y > 0.0 && self.selected_plot_range.end < self.traces.len() {
                    self.selected_plot_range =
                        self.selected_plot_range.start + 1..self.selected_plot_range.end + 1;
                } else if scroll_delta.y < 0.0 && self.selected_plot_range.start > 0 {
                    self.selected_plot_range =
                        self.selected_plot_range.start - 1..self.selected_plot_range.end - 1;
                }
            }
        }
    }

    pub(crate) fn new(loaded: LoadedTraces, title: String) -> Self {
        let mut plotter = TracePlotter {
            title,
            traces: Vec::new(),
            selected_plot_range: 0..1,
            plot_selection: PlotSelection::new(calculate_bounds(&loaded.traces)),
            currently_selected: false,
            view_mode: ViewMode::Range,
            plot_width: MAX_PLOT_WIDTH,
            next_color_index: 0,
        };
        plotter.push_traces(loaded);

        // Small sets (e.g. a handful of files) open overlapped with every trace shown;
        // large sets keep the range view.
        if plotter.traces.len() <= OVERLAY_BY_DEFAULT_MAX_TRACES {
            plotter.traces.iter_mut().for_each(|t| t.visible = true);
            plotter.view_mode = ViewMode::Overlay;
        }
        plotter
    }

    /// Adds traces to this plotter's stack and shows them overlapped with the
    /// traces already shown. Files that are already loaded are just made visible.
    pub(crate) fn add_traces(&mut self, loaded: LoadedTraces) {
        let mut new_traces = LoadedTraces {
            traces: Vec::new(),
            names: Vec::new(),
            sources: Vec::new(),
        };
        for ((trace, name), source) in loaded
            .traces
            .into_iter()
            .zip(loaded.names)
            .zip(loaded.sources)
        {
            if self.contains_source(&source) {
                self.traces
                    .iter_mut()
                    .filter(|t| t.source == source)
                    .for_each(|t| t.visible = true);
            } else {
                new_traces.traces.push(trace);
                new_traces.names.push(name);
                new_traces.sources.push(source);
            }
        }

        // Coming from Range view: keep showing what was on screen
        if self.view_mode == ViewMode::Range && !self.traces.iter().any(|t| t.visible) {
            for i in self.selected_plot_range.clone() {
                if let Some(t) = self.traces.get_mut(i) {
                    t.visible = true;
                }
            }
        }

        let first_new = self.traces.len();
        let added = new_traces.traces.len();
        self.push_traces(new_traces);

        // Show the new traces (just the first one if a file holds a huge set)
        let show_count = if added <= OVERLAY_BY_DEFAULT_MAX_TRACES { added } else { 1 };
        for t in self.traces.iter_mut().skip(first_new).take(show_count) {
            t.visible = true;
        }

        self.view_mode = ViewMode::Overlay;
        self.update_bounds();
    }

    /// Removes every trace that was loaded from `source`.
    pub(crate) fn remove_source(&mut self, source: &str) {
        self.traces.retain(|t| t.source != source);

        // Keep the range view pointing at traces that still exist
        let len = self.traces.len();
        let end = self.selected_plot_range.end.min(len).max(len.min(1));
        let start = self.selected_plot_range.start.min(end.saturating_sub(1));
        self.selected_plot_range = start..end;

        self.update_bounds();
    }

    /// Refits the full-view bounds to the traces currently loaded.
    fn update_bounds(&mut self) {
        if !self.traces.is_empty() {
            self.plot_selection
                .set_default_bounds(calculate_bounds(self.traces.iter().map(|t| &t.trace)));
        }
    }

    fn push_traces(&mut self, loaded: LoadedTraces) {
        let start = self.traces.len();
        for (i, trace) in loaded.traces.into_iter().enumerate() {
            let name = loaded
                .names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("Trace {}", start + i + 1));
            let source = loaded.sources.get(i).cloned().unwrap_or_default();
            self.traces
                .push(TracePlot::new(trace, name, source, self.next_color_index));
            self.next_color_index += 1;
        }
    }

    /// Whether any trace in this plotter was loaded from `source`.
    pub(crate) fn contains_source(&self, source: &str) -> bool {
        self.traces.iter().any(|t| t.source == source)
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// True when this plotter's window is the frontmost one.
    pub(crate) fn is_focused(&self) -> bool {
        self.currently_selected
    }
}
