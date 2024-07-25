use crate::trace_plotter::Trace;
use egui_plot::{Line, PlotPoints, PlotUi};

#[derive(Clone, Debug)]
pub(crate) struct TracePlot {
    pub trace: Trace,
}

impl TracePlot {
    pub(crate) fn draw_trace(&mut self, plot_ui: &mut PlotUi, max_visible_points_per_trace: usize) {
        let plot_bounds = plot_ui.plot_bounds();
        let trace = &self.trace;

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

    pub(crate) fn new(trace: Trace) -> Self {
        TracePlot { trace }
    }
}
