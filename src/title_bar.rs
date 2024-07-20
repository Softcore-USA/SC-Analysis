use egui::{CentralPanel, ComboBox, Context};
use rfd::FileDialog;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
enum FileItems {
    Open,
    Close,
    Exit,
}

pub fn render(ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let mut selected = FileItems::Open;

        ComboBox::from_label("File")
            .selected_text(format!("{:?}", selected))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, FileItems::Open, "Open");
            });

        if ui.button("File").clicked() {
            _ = open_file_explorer();
        }
    });
}

pub fn open_file_explorer() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("trace_set", &["bin"])
        .set_directory("/")
        .pick_file()
}
