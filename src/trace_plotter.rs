use std::ops::Range;
use std::time::{Duration, Instant};
use eframe::emath::{Rect, Vec2};
use eframe::epaint::{Color32, Stroke};
use egui::{CentralPanel, ComboBox, Context, Event, Key, Pos2, Rounding, Shape, Vec2b};
use egui::epaint::RectShape;
use egui_plot::{BoxElem, BoxPlot, BoxSpread, Legend, Line, Plot, PlotBounds, PlotItem, PlotPoint, PlotPoints, PlotUi, Polygon, VLine};

#[derive(Clone, Debug)]
pub struct TracePlotter{
    trace: Vec<Vec<(f64, f64)>>,
    selected_plot_range: Range<usize>,
    start_pos: Option<PlotPoint>,
    end_pos: Option<PlotPoint>,
    pointer_down: bool,
    auto_bound: bool,
    bounds: (f64, f64, f64, f64),
    zoom_history: Vec<PlotBounds>
}

impl TracePlotter {
    pub fn render(&mut self, ctx: &Context,){
        // Handle key inputs to change the selected plot range
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            if self.selected_plot_range.end < self.trace.len() {
                self.selected_plot_range = self.selected_plot_range.start + 1..self.selected_plot_range.end + 1;
            }
        }

        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            if self.selected_plot_range.start > 0 {
                self.selected_plot_range = self.selected_plot_range.start - 1..self.selected_plot_range.end - 1;
            }
        }

        CentralPanel::default().show(ctx, |ui| {


            ui.horizontal(|ui| {
                ui.label("Select Plot:");
                ComboBox::from_label("")
                    .selected_text(format!(
                        "Plot {}",
                        self.selected_plot_range.start + 1
                    ))
                    .show_ui(ui, |ui| {
                        for (i, _) in self.trace.iter().enumerate() {
                            if ui
                                .selectable_value(
                                    &mut self.selected_plot_range,
                                    i..i + 1,
                                    format!("Plot {}", i + 1),
                                )
                                .clicked()
                            {
                                self.selected_plot_range = i..i + 1;
                            }
                        }
                    });
            });

            ui.vertical(|ui|{
                let (scroll, pointer_down, modifiers) = ui.input(|i| {
                    let scroll = i.events.iter().find_map(|e| match e {
                        Event::MouseWheel {
                            unit: _,
                            delta,
                            modifiers: _,
                        } => Some(*delta),
                        Event::Zoom(zoom) => {
                            None
                        }
                        _ => None,
                    });
                    (scroll, i.pointer.primary_down(), i.modifiers)
                });

                if let Some(scroll) = scroll {
                    if modifiers.command {
                        // Control key is held, expand or shrink only one side
                        if scroll.y > 0.0 {
                            if self.selected_plot_range.end < self.trace.len() {
                                self.selected_plot_range = self.selected_plot_range.start..self.selected_plot_range.end + 1;
                            }
                        } else if scroll.y < 0.0 {
                            if self.selected_plot_range.end > self.selected_plot_range.start + 1 {
                                self.selected_plot_range = self.selected_plot_range.start..self.selected_plot_range.end - 1;
                            }
                        }
                    } else {
                        // Expand or shrink both sides
                        if scroll.y > 0.0 && self.selected_plot_range.end < self.trace.len() {
                            self.selected_plot_range = self.selected_plot_range.start + 1..self.selected_plot_range.end + 1;
                        } else if scroll.y < 0.0 && self.selected_plot_range.start > 0 {
                            self.selected_plot_range = self.selected_plot_range.start - 1..self.selected_plot_range.end - 1;
                        }
                    }
                }




                let mut plot = Plot::new("trace_plot")
                    .width(1000.0)
                    .legend(Legend::default())
                    .view_aspect(2.0)
                    .allow_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .auto_bounds(Vec2b::FALSE);

                plot.show(ui, |plot_ui| {

                    if self.auto_bound  {
                        plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                            [self.bounds.0, self.bounds.2],
                            [self.bounds.1, self.bounds.3],
                        ));
                        self.auto_bound = false;
                    }
                    let plot_bounds = plot_ui.plot_bounds();

                    // // Create and draw lines for each trace in the selected range
                    // for i in self.selected_plot_range.clone() {
                    //     let line = self.create_line(&self.trace[i], &plot_bounds, 100_000);
                    //     plot_ui.line(line);
                    // }
                    self.draw_traces(plot_ui, 100_000);


                    if self.pointer_down != pointer_down {
                        if !self.pointer_down {
                            if let Some(coord) = plot_ui.pointer_coordinate() {
                                self.start_pos = Some(PlotPoint {
                                    x: coord.x.clamp(plot_bounds.min()[0], plot_bounds.max()[0]),
                                    y: coord.y.clamp(plot_bounds.min()[1], plot_bounds.max()[1]),
                                });
                            }
                        }
                        self.pointer_down = pointer_down;
                    }

                    if self.pointer_down {
                        if let Some(coord) = plot_ui.pointer_coordinate() {
                            self.end_pos = Some(PlotPoint {
                                x: coord.x.clamp(plot_bounds.min()[0], plot_bounds.max()[0]),
                                y: coord.y.clamp(plot_bounds.min()[1], plot_bounds.max()[1]),
                            });
                        }
                    }


                    // Check for Enter key press and update the plot bounds based on start and end positions
                    if ctx.input(|input| input.key_pressed(Key::Enter)) {
                        if let (Some(start), Some(end)) = (self.start_pos, self.end_pos) {
                            let (new_min_x, new_max_x) = (start.x.min(end.x), start.x.max(end.x));
                            let (min_y, max_y) = (self.bounds.2, self.bounds.3); // Keep y bounds unchanged

                            let new_bound = PlotBounds::from_min_max(
                                [new_min_x, min_y],
                                [new_max_x, max_y],
                            );

                            // Set new plot bounds
                            plot_ui.set_plot_bounds(new_bound);

                            self.zoom_history.push(new_bound);

                            self.end_pos = None;
                            self.start_pos = None;
                        }
                    }

                    if ctx.input(|input| input.key_pressed(Key::Escape)) {
                        // Get the current plot bounds for comparison
                        let current_bounds = plot_ui.plot_bounds();

                        // Check if the last recorded bounds are the same as the current bounds
                        if let Some(last_bounds) = self.zoom_history.last() {
                            if last_bounds == &current_bounds {
                                // Remove the last entry if it matches the current view
                                self.zoom_history.pop();
                            }
                        }

                        if let Some(bounds) = self.zoom_history.pop() {
                            plot_ui.set_plot_bounds(bounds);
                        } else {
                            plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                                [self.bounds.0, self.bounds.2],
                                [self.bounds.1, self.bounds.3],
                            ));
                        }

                        self.start_pos = None;
                        self.end_pos = None;
                    }

                    self.draw_selection_box(plot_ui);



                });
            });



        });
    }

    fn draw_selection_box(&self, mut plot_ui: &mut PlotUi){
        if let (Some(start), Some(end)) = (self.start_pos, self.end_pos){

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
            let visible_points: Vec<[f64; 2]> = trace
                .iter()
                .filter(|&&(x, _)| x >= min_x && x <= max_x)
                .map(|&(x, y)| [x, y])
                .collect();

            let total_points = visible_points.len();
            let max_visible_points_per_trace = max_total_points / (self.selected_plot_range.end - self.selected_plot_range.start).max(1);

            let step = if total_points > max_visible_points_per_trace {
                total_points / max_visible_points_per_trace
            } else {
                1
            };

            // Downsampling with spike detection
            let mut values: Vec<[f64; 2]> = Vec::new();
            let mut last_y = visible_points[0][1];
            for (i, &point) in visible_points.iter().enumerate() {
                let (x, y) = (point[0], point[1]);
                if i % step == 0 || (y - last_y).abs() > 0.2 { // Threshold to detect spikes
                    values.push(point);
                    last_y = y;
                }
            }

            let line = Line::new(PlotPoints::new(values));
            plot_ui.line(line);
        }

        if self.auto_bound {
            plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                [self.bounds.0, self.bounds.2],
                [self.bounds.1, self.bounds.3],
            ));
            self.auto_bound = false;
        }
    }


    fn calculate_bounds(trace_data: &Vec<Vec<(f64, f64)>>) -> (f64, f64, f64, f64) {
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

        (min_x, max_x, min_y, max_y)
    }

    pub(crate) fn new(trace_data: Vec<Vec<(f64, f64)>>) -> Self {

        let bounds = Self::calculate_bounds(&trace_data);

        TracePlotter{
            trace: trace_data,
            selected_plot_range: 0..1,
            start_pos: None,
            end_pos: None,
            pointer_down: false,
            auto_bound: true,
            bounds,
            zoom_history: vec![],
        }
    }
}