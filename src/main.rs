extern crate core;

mod nav_bar;
mod trace_plotter;
mod wave;
mod math;

use crate::trace_plotter::TracePlotter;
use bincode::config;
use csv::ReaderBuilder;
use eframe::Frame;
use egui::CentralPanel;
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello from the root viewport");

            if ui.button("Open new Trace Plotter").clicked() {
                let file_path = "./data/100x100XYAquisition.bin";
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
        }
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
        let shifts = math::static_align(
            100,
            &trace_data,
            100..1000,
            1000,
            0.50
        );
        println!("Shifts: {:?}", shifts);

        let trace_plotter = TracePlotter::new(trace_data, title);

        self.trace_plotters.push((trace_plotter, true));
    }
}

fn main() -> Result<(), eframe::Error> {
    // Measure the execution time of loading data from CSV
    // let start_csv = Instant::now();
    // let data = load_csv("data/EMAcquisition_4thQuadranthotspot+StaticAlign.csv").unwrap();
    // let duration_csv = start_csv.elapsed();
    // println!("Time taken to load CSV: {:?}", duration_csv);
    //
    // write_to_file(&data, "data2.bin").unwrap();

    // Measure the execution time of loading data from a binary file
    let start_bin = Instant::now();

    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();

    let duration_bin = start_bin.elapsed();
    println!("Time taken to load binary file: {:?}", duration_bin);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1250.0, 750.0]),
        ..Default::default()
    };
    eframe::run_native(
        "SC-Analysis",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

fn load_from_file(file_path: &str) -> io::Result<Vec<Vec<(f64, f64)>>> {
    let config = config::standard().with_limit::<10000000000>();

    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let decompressed = zstd::decode_all(&buffer[..]).unwrap();

    let data: Vec<Vec<(f64, f64)>> = bincode::decode_from_slice(&decompressed, config)
        .unwrap()
        .0;
    Ok(data)
}

fn write_to_file(data: &[Vec<(f64, f64)>], file_path: &str) -> io::Result<()> {
    let config = config::standard().with_limit::<10000000000>();

    // Split the data into chunks for parallel compression
    let chunks: Vec<_> = data.chunks(data.len() / num_cpus::get()).collect();
    let encoded_chunks: Vec<Vec<u8>> = chunks
        .into_par_iter()
        .map(|chunk| bincode::encode_to_vec(chunk, config).unwrap())
        .collect();

    let encoded: Vec<u8> = encoded_chunks.into_iter().flatten().collect();
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
