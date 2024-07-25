use crate::trace_plotter::plot_selection::PlotSelection;
use crate::trace_plotter::trace_plot::TracePlot;
use crate::trace_plotter::util::calculate_bounds;
use egui::{Area, ComboBox, Context, Id, Key, Ui, UiKind, Vec2b, Window};
use egui_plot::{
    Legend, Plot, PlotResponse, PlotUi,
};
use std::ops::Range;
use egui::debug_text::print;

const MAX_NUMB_OF_POINTS: usize = 100_000;

#[derive(Clone, Debug)]
pub struct TracePlotter {
    title: String,
    traces: Vec<TracePlot>,
    selected_plot_range: Range<usize>,
    plot_selection: PlotSelection,
    currently_selected: bool,
}

impl TracePlotter {
    pub fn render(&mut self, ctx: &Context, open: &mut bool) {
        let window = Window::new(&self.title);
        let area = Area::new(Id::new(&self.title)).kind(UiKind::Window);

        let area_layer_id = area.layer();
        self.currently_selected = Some(area_layer_id) == ctx.top_layer_id();

        window.open(open).show(ctx, |ui| {
            // Handle key inputs to change the selected plot range
            if ctx.input(|i| i.key_pressed(Key::ArrowUp))
                && self.selected_plot_range.end < self.traces.len()
                && self.currently_selected
            {
                self.selected_plot_range =
                    self.selected_plot_range.start + 1..self.selected_plot_range.end + 1;
            }

            if ctx.input(|i| i.key_pressed(Key::ArrowDown))
                && self.selected_plot_range.start > 0
                && self.currently_selected
            {
                self.selected_plot_range =
                    self.selected_plot_range.start - 1..self.selected_plot_range.end - 1;
            }

            let mut should_scroll = true;

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
                                if self.selected_plot_range.start > self.selected_plot_range.end {
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
                                if self.selected_plot_range.end < self.selected_plot_range.start {
                                    self.selected_plot_range.start =
                                        self.selected_plot_range.end - 1;
                                }
                            }
                        }
                    });
                
                should_scroll = !ComboBox::is_open(ctx, start_response.response.id) && !ComboBox::is_open(ctx, end_response.response.id);

            });


            self.render_plot(ui);
            if should_scroll {
                self.update_selected_plot_range(ui);
            }


            if let Some(range) = self
                .plot_selection
                .get_selected_data_range_indices(&self.traces.first().unwrap().trace)
            {
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
        let plot = Plot::new("trace_plot")
            .width(1000.0)
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
        let num_of_shown_traces = self.selected_plot_range.len();
        let max_visible_points_per_trace = MAX_NUMB_OF_POINTS / num_of_shown_traces.max(1);

        for i in self.selected_plot_range.clone() {
            self.traces[i].draw_trace(plot_ui, max_visible_points_per_trace);
        }
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

    pub(crate) fn new(trace_data: Vec<Vec<(f64, f64)>>, title: String) -> Self {
        let bounds = calculate_bounds(&trace_data);

        let traces: Vec<TracePlot> = trace_data.into_iter().map(TracePlot::new).collect();

        TracePlotter {
            title,
            traces,
            selected_plot_range: 0..1,
            plot_selection: PlotSelection::new(bounds),
            currently_selected: false,
        }
    }
}
