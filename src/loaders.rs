use bincode::config;
use eframe::epaint::Color32;
use egui::{Align, Ui};
use egui_modal::{Icon, Modal, ModalStyle};
use rfd::FileDialog;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::{Read};
use std::path::Path;
use std::time::Instant;

#[allow(dead_code)]
pub fn open_file_explorer() -> Option<String> {
    FileDialog::new()
        .add_filter("trace_set", &["bin", "csv", "txt"])
        .set_directory("/")
        .pick_file()
        .map(|x| x.to_string_lossy().into_owned())
}

/// Opens a file picker that allows selecting several trace files at once.
pub fn open_file_explorer_multi() -> Option<Vec<String>> {
    FileDialog::new()
        .add_filter("trace_set", &["bin", "csv", "txt"])
        .set_directory("/")
        .pick_files()
        .map(|paths| {
            paths
                .into_iter()
                .map(|x| x.to_string_lossy().into_owned())
                .collect()
        })
}

/// Traces loaded from one or more files, with a display name and source file for each trace.
pub struct LoadedTraces {
    pub traces: Vec<Vec<(f64, f64)>>,
    pub names: Vec<String>,
    pub sources: Vec<String>,
}

/// Loads every file in `file_paths` and combines their traces into one set,
/// naming traces after their file name.
pub fn load_trace_files(file_paths: &[String]) -> Result<LoadedTraces, Box<dyn Error>> {
    let labelled: Vec<(String, String)> = file_paths
        .iter()
        .map(|path| {
            let label = Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            (path.clone(), label)
        })
        .collect();
    load_trace_files_labelled(&labelled)
}

/// Loads `(path, label)` pairs and combines their traces into one set.
/// A file holding a single trace is named by its label; a file holding several
/// traces gets numbered names ("label #1", "label #2", ...).
pub fn load_trace_files_labelled(
    files: &[(String, String)],
) -> Result<LoadedTraces, Box<dyn Error>> {
    let mut loaded = LoadedTraces {
        traces: Vec::new(),
        names: Vec::new(),
        sources: Vec::new(),
    };

    for (file_path, label) in files {
        let file_traces = load_trace_file(file_path).map_err(|e| format!("{}: {}", label, e))?;

        let count = file_traces.len();
        for (i, trace) in file_traces.into_iter().enumerate() {
            loaded.names.push(if count == 1 {
                label.clone()
            } else {
                format!("{} #{}", label, i + 1)
            });
            loaded.sources.push(file_path.clone());
            loaded.traces.push(trace);
        }
    }

    Ok(loaded)
}

/// Opens a folder picker for choosing a workspace directory.
pub fn open_folder_explorer() -> Option<String> {
    FileDialog::new()
        .pick_folder()
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

/// Loads a trace set from either a compressed .bin file or a .csv file,
/// picking the loader based on the file extension.
pub fn load_trace_file(file_path: &str) -> Result<Vec<Vec<(f64, f64)>>, Box<dyn Error>> {
    let extension = Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());

    match extension.as_deref() {
        Some("csv") | Some("txt") => load_csv(file_path),
        _ => Ok(load_from_file(file_path)?),
    }
}

/// Loads a CSV-style text file. Two layouts are supported:
/// - a single column of values (one sample per line): loaded as one trace,
///   using the sample index as the x value
/// - a time column followed by one or more trace columns
/// Also handles:
/// - comma, semicolon, tab or whitespace separators (auto-detected)
/// - any number of non-numeric header / metadata lines before the data
/// - a UTF-8 byte-order mark, quoted values, and decimal commas (when not comma-separated)
/// - out-of-range ("∞"/"inf") or unreadable cells are skipped with a warning
pub fn load_csv(file_path: &str) -> Result<Vec<Vec<(f64, f64)>>, Box<dyn Error>> {
    let start_csv = Instant::now();
    let bytes = std::fs::read(file_path)?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim_start_matches('\u{feff}');

    let delimiter = detect_delimiter(text);
    let split_row = |line: &str| -> Vec<String> {
        let fields: Vec<&str> = match delimiter {
            Some(d) => line.split(d).collect(),
            None => line.split_whitespace().collect(),
        };
        fields
            .into_iter()
            .map(|f| {
                let f = f.trim().trim_matches('"').trim();
                if delimiter != Some(',') {
                    f.replace(',', ".")
                } else {
                    f.to_string()
                }
            })
            .collect()
    };

    let mut columns: Vec<Vec<(f64, f64)>> = Vec::new();
    // Values from single-column rows, x = sample index
    let mut single_column: Vec<(f64, f64)> = Vec::new();
    let mut data_started = false;
    let mut skipped_cells = 0usize;
    let mut skipped_rows = 0usize;

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_row(line);
        let time = match parse_number(&fields[0]) {
            Some(t) if t.is_finite() => t,
            _ => {
                // Header / metadata lines before the data are skipped silently
                if data_started {
                    skipped_rows += 1;
                }
                continue;
            }
        };
        data_started = true;

        if fields.len() == 1 {
            single_column.push((single_column.len() as f64, time));
            continue;
        }

        for (i, value) in fields[1..].iter().enumerate() {
            if value.is_empty() {
                continue;
            }
            match parse_number(value) {
                Some(data) if data.is_finite() => {
                    if i >= columns.len() {
                        columns.resize_with(i + 1, Vec::new);
                    }
                    columns[i].push((time, data));
                }
                _ => skipped_cells += 1,
            }
        }
    }

    if skipped_rows > 0 || skipped_cells > 0 {
        log::warn!(
            "{}: skipped {} unreadable rows and {} unreadable values",
            file_path, skipped_rows, skipped_cells
        );
    }

    columns.retain(|c| !c.is_empty());
    if columns.is_empty() && !single_column.is_empty() {
        columns.push(single_column);
    }
    if columns.is_empty() {
        let sep = match delimiter {
            Some('\t') => "tab".to_string(),
            Some(d) => format!("'{}'", d),
            None => "whitespace".to_string(),
        };
        return Err(format!(
            "No trace data found. Expected one value per line, or a time column \
             followed by trace columns (detected separator: {}).",
            sep
        )
        .into());
    }

    println!(
        "Time taken to load CSV file: {:?} ({} traces)",
        start_csv.elapsed(),
        columns.len()
    );

    Ok(columns)
}

/// Picks the separator that appears most consistently in the first lines of the file.
fn detect_delimiter(text: &str) -> Option<char> {
    let sample: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(50)
        .collect();
    [',', ';', '\t']
        .into_iter()
        .map(|d| (d, sample.iter().filter(|l| l.contains(d)).count()))
        .filter(|(_, n)| *n > 0)
        .max_by_key(|(_, n)| *n)
        .map(|(d, _)| d)
}

fn parse_number(s: &str) -> Option<f64> {
    if let Ok(v) = s.parse::<f64>() {
        return Some(v);
    }
    match s {
        "∞" | "+∞" => Some(f64::INFINITY),
        "-∞" | "−∞" => Some(f64::NEG_INFINITY),
        _ => None,
    }
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
