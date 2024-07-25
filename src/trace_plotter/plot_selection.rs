use crate::trace_plotter::Trace;
use eframe::epaint::{Color32, Stroke};
use egui_plot::{PlotBounds, PlotPoint, PlotPoints, PlotResponse, PlotUi, Polygon};
use std::ops::Range;

#[derive(Clone, Debug)]
pub(crate) struct PlotSelection {
    start_pos: Option<PlotPoint>,
    end_pos: Option<PlotPoint>,
    default_bounds: PlotBounds,
    plot_bounds: PlotBounds,
    zoom_history: Vec<PlotBounds>,
}

impl PlotSelection {
    pub(crate) fn update_selection(&mut self, plot_response: PlotResponse<()>) {
        let plot_transform = plot_response.transform;
        let plot_bounds = plot_transform.bounds();

        let response = &plot_response.response;

        if response.drag_started() {
            self.start_pos = generate_plot_points(&plot_response, plot_bounds);
            self.end_pos = None;
        }

        if response.dragged() {
            self.end_pos = generate_plot_points(&plot_response, plot_bounds);
        }
    }

    pub(crate) fn select_zoom(&mut self) {
        if let (Some(start), Some(end)) = (self.start_pos, self.end_pos) {
            let (new_min_x, new_max_x) = (start.x.min(end.x), start.x.max(end.x));
            let (min_y, max_y) = (self.plot_bounds.min()[1], self.plot_bounds.max()[1]); // Keep y bounds unchanged

            let new_bound = PlotBounds::from_min_max([new_min_x, min_y], [new_max_x, max_y]);

            // Set new plot bounds
            self.plot_bounds = new_bound;

            self.zoom_history.push(new_bound);

            self.end_pos = None;
            self.start_pos = None;
        }
    }

    /// Reverts the zoom level to the previous state or to the default bounds if no history exists.
    ///
    /// Clears `start_pos` and `end_pos` if both are set. Otherwise, it checks the zoom history:
    /// - Removes the last entry if it matches the current plot bounds.
    /// - Sets the plot bounds to the last recorded bounds in the history, or to `default_bound` if history is empty.
    ///
    /// # Parameters
    /// - `default_bound`: The default plot bounds to revert to if the zoom history is empty.
    ///
    /// # Example
    /// ```
    /// plot.step_back_zoom(default_bounds);
    /// ```
    pub(crate) fn revert_zoom(&mut self) {
        if self.start_pos.is_some() && self.end_pos.is_some() {
            self.start_pos = None;
            self.end_pos = None;
        } else {
            // Get the current plot bounds for comparison
            let current_bounds = &self.plot_bounds;

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

    pub fn get_selected_data_range_indices(&self, selected_trace: &Trace) -> Option<Range<usize>> {
        if let (Some(start), Some(end)) = (self.start_pos, self.end_pos) {
            let start_x = start.x.min(end.x);
            let end_x = start.x.max(end.x);

            let start_idx = selected_trace.iter().position(|&(x, _)| x >= start_x)?;
            let end_idx = selected_trace.iter().position(|&(x, _)| x >= end_x)?;

            Some(start_idx..end_idx + 1)
        } else {
            None
        }
    }

    pub(crate) fn get_plot_bounds(&self) -> PlotBounds {
        self.plot_bounds
    }

    pub(crate) fn draw_selection_box(&self, plot_ui: &mut PlotUi) {
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

    pub(crate) fn new(plot_bounds: PlotBounds) -> Self {
        PlotSelection {
            start_pos: None,
            end_pos: None,
            default_bounds: plot_bounds,
            plot_bounds,
            zoom_history: vec![],
        }
    }
}

fn generate_plot_points(
    response: &PlotResponse<()>,
    plot_bounds: &PlotBounds,
) -> Option<PlotPoint> {
    if let Some(pointer_pos) = response.response.hover_pos() {
        let plot_pos = response.transform.value_from_position(pointer_pos);

        return Some(PlotPoint {
            x: plot_pos.x.clamp(plot_bounds.min()[0], plot_bounds.max()[0]),
            y: plot_pos.y.clamp(plot_bounds.min()[1], plot_bounds.max()[1]),
        });
    }

    None
}
