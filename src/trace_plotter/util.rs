use egui_plot::PlotBounds;

pub(crate) fn calculate_bounds(trace_data: &Vec<Vec<(f64, f64)>>) -> PlotBounds {
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

    PlotBounds::from_min_max([min_x, min_y], [max_x, max_y])
}
