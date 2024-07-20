use bincode::config;
use criterion::{black_box, criterion_group, criterion_main, Criterion, SamplingMode};
use csv::ReaderBuilder;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::time::Duration;
use zstd::encode_all;

fn benchmark_functions(c: &mut Criterion) {
    //let data = load_csv("data/EMAcquisition_4thQuadranthotspot+StaticAlign.csv").unwrap();

    let mut group = c.benchmark_group("flat-sampling");
    group.sampling_mode(SamplingMode::Flat);

    // group.bench_function("write_to_file", |b| {
    //     b.iter(|| write_to_file(black_box(&data), black_box("data.bin")).unwrap())
    // });
    //
    // group.bench_function("load_from_file", |b| {
    //     b.iter(|| load_from_file(black_box("data.bin")).unwrap())
    // });

    group.bench_function("load_csv", |b| {
        b.iter(|| {
            load_csv(black_box(
                "data/EMAcquisition_4thQuadranthotspot+StaticAlign.csv",
            ))
            .unwrap()
        })
    });
}
fn custom_criterion() -> Criterion {
    Criterion::default()
        .sample_size(10) // Set the number of samples here
        .measurement_time(Duration::from_secs(59)) // Increase the target time to 5 seconds
        .configure_from_args() // This allows further configuration from CLI
}

criterion_group! {
    name = benches;
    config = custom_criterion();
    targets = benchmark_functions
}
criterion_main!(benches);

fn load_from_file(file_path: &str) -> io::Result<Vec<Vec<(f64, f64)>>> {
    let config = config::standard().with_limit::<10000000000>();

    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let decompressed = zstd::decode_all(&buffer[..]).unwrap();

    // Split the buffer into chunks for parallel decompression
    let chunks: Vec<_> = decompressed
        .chunks(decompressed.len() / num_cpus::get())
        .collect();
    let decoded_chunks: Result<Vec<Vec<Vec<(f64, f64)>>>, _> = chunks
        .into_par_iter()
        .map(|chunk| bincode::decode_from_slice(chunk, config).map(|(data, _)| data))
        .collect();

    let data: Vec<Vec<(f64, f64)>> = decoded_chunks.unwrap().into_iter().flatten().collect();
    Ok(data)
}

fn write_to_file(data: &Vec<Vec<(f64, f64)>>, file_path: &str) -> io::Result<()> {
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
