use bincode::config;
use csv::ReaderBuilder;
use eframe::epaint::Color32;
use egui::{Align, Ui};
use egui_modal::{Icon, Modal, ModalStyle};
use rfd::FileDialog;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::{Read};
use std::time::Instant;

pub fn open_file_explorer() -> Option<String> {
    FileDialog::new()
        .add_filter("trace_set", &["bin"])
        .set_directory("/")
        .pick_file()
        .map(|x| x.to_string_lossy().into_owned())
}

pub fn load_from_file(file_path: &str) -> Result<Vec<Vec<(f64, f64)>>, io::Error> {
    let config = config::standard();

    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Measure the execution time of loading data from a binary file
    let start_bin = Instant::now();
    let decompressed = zstd::decode_all(&buffer[..])?;

    let duration_bin = start_bin.elapsed();
    println!("Time taken to load binary file: {:?}", duration_bin);

    let data: Vec<Vec<(f64, f64)>> = bincode::decode_from_slice(&decompressed, config).unwrap().0;

    Ok(data)
}

#[allow(dead_code)]
pub fn load_csv(file_path: &str) -> Result<Vec<Vec<(f64, f64)>>, Box<dyn Error>> {
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

pub fn dialog_box_ok(ui: &mut Ui, id: &str, message: &str, message_type: Icon) -> Modal {
    let style = ModalStyle {
        body_margin: 30.0,
        frame_margin: 0.0,
        icon_margin: 10.0,
        icon_size: 40.0,
        overlay_color: Default::default(),
        caution_button_fill: Default::default(),
        suggested_button_fill: Default::default(),
        caution_button_text_color: Default::default(),
        suggested_button_text_color: Default::default(),
        dialog_ok_text: "".to_string(),
        info_icon_color: Color32::LIGHT_BLUE,
        warning_icon_color: Color32::YELLOW,
        success_icon_color: Color32::GREEN,
        error_icon_color: Color32::RED,
        default_width: Some(ui.max_rect().max.x / 2.0),
        default_height: Some(ui.max_rect().max.y / 2.0),
        body_alignment: Align::Center,
        ..Default::default()
    };

    let error_dialog = Modal::new(ui.ctx(), id).with_style(&style);

    error_dialog.show(|ui| {
        error_dialog.frame(ui, |ui| {
            error_dialog.body_and_icon(ui, message, message_type);
        });
        error_dialog.buttons(ui, |ui| {
            if ui.button("Ok").clicked() {
                error_dialog.close();
            }
        });
    });

    error_dialog
}
