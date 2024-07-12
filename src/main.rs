#[macro_use]
extern crate diesel;

mod wave;
pub mod schema;
use std::env;
use std::fs::File;
use std::error::Error;
use csv::ReaderBuilder;
use diesel::dsl::max;
use eframe::Frame;
use egui::{CentralPanel, ComboBox, Context};
use egui_plot::{BoxPlot, Legend, Line, Plot, PlotPoints};
use plotters::prelude::*;
use diesel::prelude::*;
use crate::schema::{trace_sets, traces, voltage_readings};
use dotenv::dotenv;

struct App {
    conn: SqliteConnection,
    data: Vec<(f64, f64)>,  // Store only the currently selected trace
    selected_trace_id: i32,  // ID of the selected trace
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Refresh Data").clicked() {
                    self.load_data();
                }
                ui.label("Select Trace ID:");
                if ComboBox::from_label("")
                    .selected_text(format!("Trace ID {}", self.selected_trace_id))
                    .show_ui(ui, |ui| {
                        for id in 1..=2000 {  // Example ID range
                            if ui.selectable_value(&mut self.selected_trace_id, id, format!("Trace ID {}", id)).clicked() {
                                self.load_data();  // Load data when a new trace ID is selected
                            }
                        }
                    }).response.changed()
                {
                    self.load_data();  // Optionally load data again if needed
                }
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

#[derive(Queryable, Selectable)]
struct VoltageReading {
    id: Option<i32>,
    trace_id: i32,
    timestep: f32,
    voltage_value: f32,
}


impl App {
    fn create_plot(&self) -> Line {
        let selected_data = &self.data;
        let values: PlotPoints = selected_data.iter().map(|&(x, y)| [x, y]).collect();
        Line::new(values)
    }

    // Function to load trace data from the database for the selected trace ID
    pub fn load_data(&mut self) {
        use crate::schema::voltage_readings::dsl::*;

        self.data.clear();  // Clear existing data

        let results = voltage_readings
            .filter(trace_id.eq(self.selected_trace_id))
            .load::<VoltageReading>(&mut self.conn)
            .expect("Error loading voltage readings");

        // Collect data for the current trace
        self.data = results
            .iter()
            .map(|reading| (reading.timestep as f64, reading.voltage_value as f64))
            .collect();
    }


    fn new(conn: SqliteConnection) -> Self {
        App {
            conn,
            data: vec![],
            selected_trace_id: 0

        }
    }
}


fn main() -> Result<(), eframe::Error> {

    //let data = load_csv("data/EMAcquisition_4thQuadranthotspot+StaticAlign.csv").unwrap();


    let connection = establish_connection();

    //insert_data(&mut connection, data.clone()).ok();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1250.0, 750.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Ground Station",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(connection)))),
    )

}

fn insert_data(conn: &mut SqliteConnection, data: Vec<Vec<(f64, f64)>>) -> QueryResult<()> {
    conn.transaction(|conn| {  // Use the closure parameter `conn` instead of the outer `conn`
        // Create a new trace set
        diesel::insert_into(trace_sets::table)
            .default_values()
            .execute(conn)?;

        // Retrieve the last inserted ID for the trace set
        let set_id_result = trace_sets::table
            .select(diesel::dsl::max(trace_sets::id))
            .first::<Option<i32>>(conn)?
            .unwrap();

        // Iterate over the columns which represent different traces
        for column in data {
            // Create a new trace
            diesel::insert_into(traces::table)
                .values(traces::set_id.eq(set_id_result))
                .execute(conn)?;

            let trace_id_result = traces::table
                .select(diesel::dsl::max(traces::id))
                .first::<Option<i32>>(conn)?
                .unwrap();

            // Insert voltage readings
            for &(time, voltage) in &column {
                diesel::insert_into(voltage_readings::table)
                    .values((
                        voltage_readings::trace_id.eq(trace_id_result),
                        voltage_readings::timestep.eq(time as f32),
                        voltage_readings::voltage_value.eq(voltage as f32),
                    ))
                    .execute(conn)?;
            }
        }
        Ok(())
    })
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
