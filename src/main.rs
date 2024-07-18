mod wave;
mod trace_plotter;

use std::fs::File;
use std::error::Error;
use std::io;
use std::io::{Read, Write};
use std::time::Instant;
use csv::ReaderBuilder;
use eframe::Frame;
use egui::{Context};
use splines::{Interpolation, Key, Spline};
use bincode;
use bincode::config;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use zstd::encode_all;
use crate::trace_plotter::TracePlotter;

struct App {
    traces: Vec<TracePlotter>,

}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        if let Some(&mut ref mut trace) = self.traces.first_mut() {
            trace.render(ctx);
        }
    }
}

impl App {


    fn new(data: Vec<Vec<(f64, f64)>>) -> Self {
        App {
            traces: vec![TracePlotter::new(data)]
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
    // Measure the execution time of loading data from CSV
    // let start_csv = Instant::now();
    // let data = load_csv("data/EMAcquisition_4thQuadranthotspot+StaticAlign.csv").unwrap();
    // let duration_csv = start_csv.elapsed();
    // println!("Time taken to load CSV: {:?}", duration_csv);
    //
    // write_to_file(&data, "data2.bin").unwrap();

    // Measure the execution time of loading data from a binary file
    let start_bin = Instant::now();
    let loaded_data = load_from_file("data2.bin").unwrap();
    let duration_bin = start_bin.elapsed();
    println!("Time taken to load binary file: {:?}", duration_bin);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1250.0, 750.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Ground Station",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(loaded_data)))),
    )

}






fn load_from_file(file_path: &str) -> io::Result<Vec<Vec<(f64, f64)>>> {
    let config = config::standard().with_limit::<10000000000>();

    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let decompressed = zstd::decode_all(&buffer[..]).unwrap();

    // Split the buffer into chunks for parallel decompression
    let chunks: Vec<_> = decompressed.chunks(decompressed.len() / num_cpus::get()).collect();
    let decoded_chunks: Result<Vec<Vec<Vec<(f64, f64)>>>, _> = chunks.into_par_iter().map(|chunk| {
        bincode::decode_from_slice(chunk, config).map(|(data, _)| data)
    }).collect();

    let data: Vec<Vec<(f64, f64)>> = decoded_chunks.unwrap().into_iter().flatten().collect();
    Ok(data)
}

fn write_to_file(data: &Vec<Vec<(f64, f64)>>, file_path: &str) -> io::Result<()> {
    let config = config::standard().with_limit::<10000000000>();

    // Split the data into chunks for parallel compression
    let chunks: Vec<_> = data.chunks(data.len() / num_cpus::get()).collect();
    let encoded_chunks: Vec<Vec<u8>> = chunks.into_par_iter().map(|chunk| {
        bincode::encode_to_vec(chunk, config).unwrap()
    }).collect();

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
