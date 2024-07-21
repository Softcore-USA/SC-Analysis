use eframe::epaint::{Color32, Stroke};
use egui::{Area, ComboBox, Context, CursorIcon, Id, Key, Ui, UiKind, Vec2b, Window};
use egui_plot::{
    Legend, Line, Plot, PlotBounds, PlotPoint, PlotPoints,
    PlotUi, Polygon,
};
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct TracePlotter {
    title: String,
    trace: Vec<Vec<(f64, f64)>>,
    selected_plot_range: Range<usize>,
    start_pos: Option<PlotPoint>,
    end_pos: Option<PlotPoint>,
    plot_bounds: PlotBounds,
    default_bounds: PlotBounds,
    zoom_history: Vec<PlotBounds>,
    currently_selected: bool
}

impl TracePlotter {

    pub fn get_selected_data_range_indices(&self) -> Option<Range<usize>> {
        if let (Some(start), Some(end)) = (self.start_pos, self.end_pos) {
            let start_x = start.x.min(end.x);
            let end_x = start.x.max(end.x);

            let selected_trace = &self.trace[self.selected_plot_range.start];

            let start_idx = selected_trace.iter().position(|&(x, _)| x >= start_x)?;
            let end_idx = selected_trace.iter().position(|&(x, _)| x >= end_x)?;

            Some(start_idx..end_idx + 1)
        } else {
            None
        }
    }

    pub fn render(&mut self, ctx: &Context, open: &mut bool) {
        let test = Window::new(&self.title);
        let area = Area::new(Id::new(&self.title)).kind(UiKind::Window);

        let area_layer_id = area.layer();
        self.currently_selected = Some(area_layer_id) == ctx.top_layer_id();


        test.open(open).show(ctx, |ui| {
            // Handle key inputs to change the selected plot range
            if ctx.input(|i| i.key_pressed(Key::ArrowUp)) && self.selected_plot_range.end < self.trace.len() && self.currently_selected {
                self.selected_plot_range =
                    self.selected_plot_range.start + 1..self.selected_plot_range.end + 1;
            }

            if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && self.selected_plot_range.start > 0 && self.currently_selected{
                self.selected_plot_range =
                    self.selected_plot_range.start - 1..self.selected_plot_range.end - 1;
            }

            ui.horizontal(|ui| {
                ui.label("Select Plot:");
                ui.label("Start:");
                ComboBox::from_label("")
                    .selected_text(format!("Plot {}", self.selected_plot_range.start + 1))
                    .show_ui(ui, |ui| {
                        for (i, _) in self.trace.iter().enumerate() {
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
                ComboBox::from_label(" ")
                    .selected_text(format!("Plot {}", self.selected_plot_range.end))
                    .show_ui(ui, |ui| {
                        for (i, _) in self.trace.iter().enumerate() {
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
            });

            self.render_plot(ctx, ui);

            if let Some(range) = self.get_selected_data_range_indices() {
                ui.label(format!(
                    "Selected range start: {:.2}, end: {:.2}, Points: {}",
                    range.start, range.end, range.len()
                ));
            } else {
                ui.label("");
            }

        });

    }

    fn render_plot(&mut self, ctx: &Context, ui: &mut Ui) {
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

        let modifiers = ui.input(|i| i.modifiers);
        let scroll_delta = ui.input(|i| i.raw_scroll_delta);

        let plot_response = plot.show(ui, |plot_ui| {
            plot_ui.set_plot_bounds(self.plot_bounds);
            self.draw_traces(plot_ui, 100_000);

            self.draw_selection_box(plot_ui);

        });

        let plot_transform = plot_response.transform;
        let plot_bounds = plot_transform.bounds();
        let response = plot_response.response;

        if response.hovered() {
            ui.output_mut(|out| out.cursor_icon = CursorIcon::ResizeHorizontal);
        }

        if self.currently_selected {
            if scroll_delta.length_sq() > 0.0 {
                if modifiers.command {
                    // Control key is held, expand or shrink only one side
                    if scroll_delta.y > 0.0 {
                        if self.selected_plot_range.end < self.trace.len() {
                            self.selected_plot_range =
                                self.selected_plot_range.start..self.selected_plot_range.end + 1;
                        }
                    } else if scroll_delta.y < 0.0 && self.selected_plot_range.end > self.selected_plot_range.start + 1 {
                        self.selected_plot_range =
                            self.selected_plot_range.start..self.selected_plot_range.end - 1;
                    }
                } else {
                    // Expand or shrink both sides
                    if scroll_delta.y > 0.0 && self.selected_plot_range.end < self.trace.len() {
                        self.selected_plot_range =
                            self.selected_plot_range.start + 1..self.selected_plot_range.end + 1;
                    } else if scroll_delta.y < 0.0 && self.selected_plot_range.start > 0 {
                        self.selected_plot_range =
                            self.selected_plot_range.start - 1..self.selected_plot_range.end - 1;
                    }
                }
            }

            if response.drag_started() {
                if let Some(pointer_pos) = response.hover_pos() {
                    let plot_pos = plot_transform.value_from_position(pointer_pos);
                    self.start_pos = Some(PlotPoint {
                        x: plot_pos.x
                            .clamp(plot_bounds.min()[0], plot_bounds.max()[0]),
                        y: plot_pos.y
                            .clamp(plot_bounds.min()[1], plot_bounds.max()[1]),
                    });
                    self.end_pos = None;
                }
            }

            if response.dragged() {
                if let Some(pointer_pos) = response.hover_pos() {
                    let plot_pos = plot_transform.value_from_position(pointer_pos);
                    self.end_pos = Some(PlotPoint {
                        x: plot_pos.x
                            .clamp(plot_bounds.min()[0], plot_bounds.max()[0]),
                        y: plot_pos.y
                            .clamp(plot_bounds.min()[1], plot_bounds.max()[1]),
                    });
                }
            }
        }




        // Check for Enter key press and update the plot bounds based on start and end positions
        if ui.input(|input| input.key_pressed(Key::Enter)) && self.currently_selected{
            if let (Some(start), Some(end)) = (self.start_pos, self.end_pos) {
                let (new_min_x, new_max_x) = (start.x.min(end.x), start.x.max(end.x));
                let (min_y, max_y) = (self.plot_bounds.min()[1], self.plot_bounds.max()[1]); // Keep y bounds unchanged

                let new_bound =
                    PlotBounds::from_min_max([new_min_x, min_y], [new_max_x, max_y]);

                // Set new plot bounds
                self.plot_bounds = new_bound;

                self.zoom_history.push(new_bound);

                self.end_pos = None;
                self.start_pos = None;
            }
        }

        if ctx.input(|input| input.key_pressed(Key::Escape)) && self.currently_selected{
            // Get the current plot bounds for comparison
            let current_bounds = plot_transform.bounds();

            // Check if the last recorded bounds are the same as the current bounds
            if let Some(last_bounds) = self.zoom_history.last() {
                if last_bounds == current_bounds {
                    // Remove the last entry if it matches the current view
                    self.zoom_history.pop();
                }
            }

            if let Some(bounds) = self.zoom_history.pop() {
                self.plot_bounds = bounds
            } else {
                self.plot_bounds = self.default_bounds;
            }

            self.start_pos = None;
            self.end_pos = None;
        }
    }

    fn draw_selection_box(&self, plot_ui: &mut PlotUi) {
        if let (Some(start), Some(end)) = (self.start_pos, self.end_pos) {
            let plot_bounds = plot_ui.plot_bounds();
            let min_y = plot_bounds.min()[1];
            let max_y = plot_bounds.max()[1];

            let points = vec![
                [start.x, min_y],
                [end.x, min_y],
                [end.x, max_y],
                [start.x, max_y],
                [start.x, min_y], // close the box by returning to the start point
            ];

            let polygon = Polygon::new(PlotPoints::new(points))
                .stroke(Stroke::new(5.0, Color32::BLUE))
                .fill_color(Color32::from_rgba_premultiplied(100, 100, 255, 5));

            plot_ui.polygon(polygon);
        }
    }

    fn draw_traces(&mut self, plot_ui: &mut PlotUi, max_total_points: usize) {
        let plot_bounds = plot_ui.plot_bounds();

        for i in self.selected_plot_range.clone() {
            let trace = &self.trace[i];

            let min_x = plot_bounds.min()[0];
            let max_x = plot_bounds.max()[0];

            // Filter points that are within the current x-axis bounds
            // Include the next point outside the bounds if available
            let mut visible_points: Vec<[f64; 2]> = Vec::new();
            let mut add_next_point = true;
            let mut add_prev_point = true;

            for &(x, y) in trace {
                if x >= min_x && x <= max_x {
                    visible_points.push([x, y]);
                    add_next_point = true;
                } else if x > max_x && add_next_point {
                    visible_points.push([x, y]);
                    add_next_point = false;
                } else if x < min_x && add_prev_point {
                    visible_points.insert(0, [x, y]);
                    add_prev_point = false;
                }
            }

            let total_points = visible_points.len();
            let max_visible_points_per_trace = max_total_points
                / (self.selected_plot_range.end - self.selected_plot_range.start).max(1);

            let step = if total_points > max_visible_points_per_trace {
                total_points / max_visible_points_per_trace
            } else {
                1
            };

            // Downsampling with spike detection
            let mut values: Vec<[f64; 2]> = Vec::new();
            let mut last_y = visible_points[0][1];
            for (i, &point) in visible_points.iter().enumerate() {
                let (_, y) = (point[0], point[1]);
                if i % step == 0 || (y - last_y).abs() > 0.2 {
                    // Threshold to detect spikes
                    values.push(point);
                    last_y = y;
                }
            }

            let line = Line::new(PlotPoints::new(values));
            plot_ui.line(line);
        }
    }

    fn calculate_bounds(trace_data: &Vec<Vec<(f64, f64)>>) -> PlotBounds {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for trace in trace_data {
            for &(x, y) in trace {
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }

        PlotBounds::from_min_max(
            [min_x, min_y],
            [max_x, max_y],
        )
    }

    pub(crate) fn new(trace_data: Vec<Vec<(f64, f64)>>, title: String) -> Self {
        let bounds = Self::calculate_bounds(&trace_data);

        TracePlotter {
            title,
            trace: trace_data,
            selected_plot_range: 0..1,
            start_pos: None,
            end_pos: None,
            plot_bounds: bounds,
            default_bounds: bounds,
            zoom_history: vec![],
            currently_selected: false,
        }
    }
}
