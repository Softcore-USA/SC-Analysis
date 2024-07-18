use eframe::emath::{Rect, Vec2};
use eframe::epaint::{Color32, Stroke};
use egui::{CentralPanel, ComboBox, Context, Event, Key, Pos2, Rounding, Shape, Vec2b};
use egui::epaint::RectShape;
use egui_plot::{BoxElem, BoxPlot, BoxSpread, Legend, Line, Plot, PlotBounds, PlotItem, PlotPoint, PlotPoints, Polygon, VLine};

#[derive(Clone, Debug)]
pub struct TracePlotter{
    trace: Vec<Vec<(f64, f64)>>,
    selected_plot: usize,
    start_pos: Option<PlotPoint>,
    end_pos: Option<PlotPoint>,
    pointer_down: bool,
    auto_bound: bool,
    bounds: (f64, f64, f64, f64),
    zoom_history: Vec<PlotBounds>
}

impl TracePlotter {
    pub fn render(&mut self, ctx: &Context,){
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) &&
            self.selected_plot + 1 < self.trace.len() {
                self.selected_plot += 1;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && self.selected_plot - 1 < self.trace.len() {
            self.selected_plot -= 1;
        }
        CentralPanel::default().show(ctx, |ui| {



            ui.horizontal(|ui| {
                ui.label("Select Plot:");
                ComboBox::from_label("")
                    .selected_text(format!("Plot {}", self.selected_plot + 1))
                    .show_ui(ui, |ui| {
                        for (i, _) in self.trace.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_plot, i, format!("Plot {}", i + 1));
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
                    }

                    plot_ui.line(self.create_line());

                    if self.pointer_down != pointer_down {
                        if !self.pointer_down {
                            self.start_pos = Some(plot_ui.pointer_coordinate().unwrap());
                        }

                        self.pointer_down = pointer_down;
                    }
                    if self.pointer_down {
                        self.end_pos = plot_ui.pointer_coordinate();
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

                    if let Some(mut scroll) = scroll {
                        let zoomed_factor =plot_ui.plot_bounds().width() as f32;

                        if modifiers.mac_cmd {
                            let zoom_factor = Vec2::from([
                                ((scroll.y * zoomed_factor) / 10.0).exp(),
                                1.0,
                            ]);

                            plot_ui.zoom_bounds_around_hovered(zoom_factor)

                        } else {
                            scroll = Vec2::new(scroll.x * zoomed_factor, 0.0);


                            let delta_pos = 0.001 * scroll;
                            plot_ui.translate_bounds(delta_pos);
                        }

                    }

                });
            });



        });
    }

    fn create_line(&self) -> Line {
        let selected_data = &self.trace[self.selected_plot];
        let values: PlotPoints = selected_data.iter().map(|&(x, y)| [x, y]).collect();
        Line::new(values)
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

        (min_x + min_x * 0.02, max_x+ max_x * 0.02, min_y+ min_y * 0.02, max_y+ max_y * 0.02)
    }

    pub(crate) fn new(trace_data: Vec<Vec<(f64, f64)>>) -> Self {

        let bounds = Self::calculate_bounds(&trace_data);

        TracePlotter{
            trace: trace_data,
            selected_plot: 0,
            start_pos: None,
            end_pos: None,
            pointer_down: false,
            auto_bound: true,
            bounds,
            zoom_history: vec![],
        }
    }
}