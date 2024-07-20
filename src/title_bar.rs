use egui::{Button, CentralPanel, ComboBox, Context, Direction, Id, Layout, PointerButton, RichText, Sense, Ui, ViewportCommand};
use rfd::FileDialog;
use std::path::PathBuf;
use eframe::emath::Align;

#[derive(Debug, PartialEq)]
enum FileItems {
    Open,
    Close,
    Exit,
}

pub fn custom_title_bar(ui: &mut Ui) {
    let side_margin = 10.0;
    let title_bar_height = 40.0;

    let title_bar_rect = {
        let mut r = ui.max_rect();
        r.max.y = r.min.y + title_bar_height;
        r
    };

    let drag_response = ui.interact(title_bar_rect, Id::new("drag_bar"), Sense::click_and_drag());

    if drag_response.double_clicked() {
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    if drag_response.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    ui.allocate_ui_at_rect(title_bar_rect, |ui| {
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(side_margin);

                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.menu_button("File", |ui| {
                        ui.button("Open").clicked();
                        ui.button("Close").clicked();
                        ui.button("Exit").clicked();
                    });
                    ui.menu_button("View", |ui| {
                        ui.button("Side Bar").clicked();
                    })
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(side_margin);

                    minimize_maximize_close(ui);
                });
            });
        });
    });

    // CentralPanel::default().show(ctx, |ui| {
    //     let mut selected = FileItems::Open;
    //
    //     ComboBox::from_label("File")
    //         .selected_text(format!("{:?}", selected))
    //         .show_ui(ui, |ui| {
    //             ui.selectable_value(&mut selected, FileItems::Open, "Open");
    //         });
    //
    //     if ui.button("File").clicked() {
    //         _ = open_file_explorer();
    //     }
    // });
}

pub fn open_file_explorer() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("trace_set", &["bin"])
        .set_directory("/")
        .pick_file()
}

fn minimize_maximize_close(ui: &mut Ui) {
    let close_response = ui
        .add(Button::new(RichText::new("❌")))
        .on_hover_text("Close Window");

    if close_response.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }

    let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));

    if is_maximized {
        let maximized_response = ui
            .add(Button::new(RichText::new("🗗")))
            .on_hover_text("Restore Window");
        if maximized_response.clicked() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Maximized(false));
        }
    } else {
        let maximized_response = ui
            .add(Button::new(RichText::new("🗗")))
            .on_hover_text("Maximize Window");
        if maximized_response.clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(true));
        }
    }

    let minimized_response = ui
        .add(Button::new(RichText::new("🗕")))
        .on_hover_text("Minimize Window");
    if minimized_response.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}