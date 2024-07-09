mod wave;

use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::fs::File;
use std::error::Error;
use csv::ReaderBuilder;
use eframe::Frame;
use egui::{CentralPanel, ComboBox, Context};
use egui_plot::{BoxPlot, Legend, Line, Plot, PlotPoints};
use plotters::prelude::*;
use splines::{Interpolation, Key, Spline};



struct App {
    data: Vec<Vec<(f64, f64)>>,
    selected_plot: usize,

}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Select Plot:");
                ComboBox::from_label("")
                    .selected_text(format!("Plot {}", self.selected_plot + 1))
                    .show_ui(ui, |ui| {
                        for (i, _) in self.data.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_plot, i, format!("Plot {}", i + 1));
                        }
                    });
            });

            let plot = Plot::new("trace_plot")
                .width(1000.0)
                .legend(Legend::default())
                .view_aspect(2.0)
                .allow_zoom(true);
            plot.show(ui, |plot_ui| {
                plot_ui.line(self.create_plot());
            });
        });
    }
}

impl App {
    fn create_plot(&self) -> Line {
        let selected_data = &self.data[self.selected_plot];
        let values: PlotPoints = selected_data.iter().map(|&(x, y)| [x, y]).collect();
        Line::new(values)
    }

    fn new(data: Vec<Vec<(f64, f64)>>) -> Self {
        App {
            data,
            selected_plot: 0

        }
    }
}

fn smooth_line(data: &[(f64, f64)], num_points: usize) -> Vec<(f64, f64)> {
    let keys: Vec<_> = data.iter().map(|&(x, y)| Key::new(x, y, Interpolation::Linear)).collect();
    let spline = Spline::from_vec(keys);

    let x_min = data.first().unwrap().0;
    let x_max = data.last().unwrap().0;
    let step = (x_max - x_min) / (num_points as f64 - 1.0);

    (0..num_points)
        .map(|i| {
            let x = x_min + i as f64 * step;
            let y = spline.clamped_sample(x).unwrap_or(0.0);
            (x, y)
        })
        .collect()
}

fn main() -> Result<(), eframe::Error> {

    let data = load_csv("data/EMAcquisition_4thQuadranthotspot+StaticAlign.csv").unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1250.0, 750.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Ground Station",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(data)))),
    )

}


fn load_csv(file_path: &str) -> Result<Vec<Vec<(f64, f64)>>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut rdr = ReaderBuilder::new().has_headers(false).from_reader(file);

    // Initialize a vector to hold all columns
    let mut columns: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut first_row = true;

    for result in rdr.records() {
        let record = result?;
        let mut iter = record.iter();

        // Read the time value
        let time: f64 = iter.next().unwrap().parse()?;

        // Read the data values and organize them into columns
        for (i, value) in iter.enumerate() {
            let data: f64 = value.parse()?;
            if first_row {
                // Initialize column vectors on the first row
                columns.push(Vec::new());
            }
            columns[i].push((time, data));
        }
        first_row = false;
    }

    Ok(columns)
}
