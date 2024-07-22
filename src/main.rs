mod title_bar;
mod trace_plotter;
mod wave;
mod math;

use crate::trace_plotter::TracePlotter;
use bincode::config;
use csv::ReaderBuilder;
use eframe::egui::Frame;
use egui::{CentralPanel, Color32};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::process::exit;
use std::time::Instant;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use zstd::encode_all;

struct App {
    trace_plotters: Vec<(TracePlotter, bool)>,
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
            title_bar::custom_title_bar(ui);

            CentralPanel::default().frame(content_panel).show_inside(ui, |ui| {
                ui.label("Hello from the root viewport");

                if ui.button("Open new Trace Plotter").clicked() {
                    let file_path = "./data.bin";
                    let loaded_data = match load_from_file(file_path) {
                        Ok(data) => data,
                        Err(_) => {
                            println!(
                                "Could not find file specified : \"{}\" Not found",
                                file_path
                            );
                            exit(1)
                        }
                    };

                    self.open_trace_plotter(loaded_data, generate_random_string(10));
                }
            });

            self.trace_plotters.retain(|(_, show)| *show);

            for (ref mut trace_plotter, ref mut show) in &mut self.trace_plotters {
                trace_plotter.render(ctx, show);
                //println!("{:?}",trace_plotter.get_selected_data_range_indices())
            }
        });
    }
}

fn generate_random_string(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}
impl App {
    fn new() -> Self {
        App {
            trace_plotters: vec![],
        }
    }

    fn open_trace_plotter(&mut self, trace_data: Vec<Vec<(f64, f64)>>, title: String) {
        let start_csv = Instant::now();
        let shifts = math::compute_static_alignment(
            0,
            &trace_data,
            580..690,
            200,
            0.50
        );

        let trace_plotter = TracePlotter::new(trace_data, "second".to_string());

        self.trace_plotters.push((trace_plotter, true));

        //println!("Shifts: {:?}", max_alignments);
        let trace_plotter = TracePlotter::new(shifts, title);

        self.trace_plotters.push((trace_plotter, true));


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



    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();


    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
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

fn load_from_file(file_path: &str) -> io::Result<Vec<Vec<(f64, f64)>>> {
    let config = config::standard();

    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Measure the execution time of loading data from a binary file
    let start_bin = Instant::now();
    let decompressed = zstd::decode_all(&buffer[..]).unwrap();

    let duration_bin = start_bin.elapsed();
    println!("Time taken to load binary file: {:?}", duration_bin);

    let data: Vec<Vec<(f64, f64)>> = bincode::decode_from_slice(&decompressed, config)
        .unwrap()
        .0;
    Ok(data)
}

fn write_to_file(data: &[Vec<(f64, f64)>], file_path: &str) -> io::Result<()> {
    let config = config::standard();


    let encoded: Vec<u8> = bincode::encode_to_vec(data, config).unwrap();
    let mut file = File::create(file_path)?;
    let compressed = encode_all(&encoded[..], 0).unwrap(); // Default compression level
    file.write_all(&compressed)?;
    Ok(())
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
