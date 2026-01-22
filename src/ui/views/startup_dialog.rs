use crate::project::validate_smart_project_path;
use crate::ui::measurer::Measurer;
use crate::ui::promise::{EguiWaker, LocalBoxFuture, Promise};
use blocking::{Task, unblock};
use egui::{Align, Button, Context, Layout, Sense, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};
use std::path::PathBuf;
use tracing::error;

pub struct StartupDialog {
    picked_path_new: Promise<LocalBoxFuture<Option<rfd::FileHandle>>>,
    picked_path: PathBuf,

    path_validation_result: Promise<Task<Result<(), String>>>,
}

impl StartupDialog {
    pub fn new(ctx: &Context) -> Self {
        let waker = EguiWaker::for_context(ctx);
        Self {
            picked_path_new: Promise::new(waker.clone()),
            picked_path: PathBuf::new(),
            path_validation_result: Promise::new(waker),
        }
    }

    pub fn with_error_message(ctx: &Context, err: String) -> Self {
        let mut slf = Self::new(ctx);
        slf.path_validation_result.set_response(Err(err));
        slf
    }

    pub fn get_result(self) -> PathBuf {
        self.picked_path
    }

    pub fn show_contents(&mut self, ui: &mut Ui, frame: &eframe::Frame) {
        let mut path_changed = false;

        // Handle events
        if let Some(res) = self.picked_path_new.take_response()
            && let Some(p) = res
        {
            p.path().clone_into(&mut self.picked_path);
            path_changed = true;
        }

        // UI
        ui.vertical_centered(|ui| {
            ui.heading("Super Metroid Disassembly Editor");
        });
        ui.separator();

        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            if ui
                .add_enabled(
                    matches!(self.path_validation_result.response(), Some(Ok(()))),
                    Button::new("Open"),
                )
                .clicked()
            {
                ui.close();
            }
            if ui
                .add_enabled(!self.picked_path_new.is_pending(), Button::new("Browse"))
                .clicked()
            {
                self.picked_path_new.launch(Box::pin(
                    rfd::AsyncFileDialog::new().set_parent(frame).pick_folder(),
                ));
            }

            let mut lossy_path_str = self.picked_path.to_string_lossy().into_owned();
            if ui
                .add(
                    TextEdit::singleline(&mut lossy_path_str)
                        .hint_text("Project Path")
                        .desired_width(f32::INFINITY),
                )
                .changed()
            {
                self.picked_path = PathBuf::from(lossy_path_str);
                path_changed = true;
            }
        });

        if let Some(Err(validation_error)) = self.path_validation_result.response() {
            ui.colored_label(ui.visuals().error_fg_color, validation_error);
        }

        ui.separator();
        ui.label("Recent projects:");

        let button_strip_measurer = Measurer::new(ui);
        const SCROLL_MIN_HEIGHT: f32 = 60.0;
        let scroll_height = if let Some(strip_height) = button_strip_measurer.query_height(ui) {
            (ui.available_height() - ui.spacing().item_spacing.y - strip_height)
                .max(SCROLL_MIN_HEIGHT)
        } else {
            SCROLL_MIN_HEIGHT
        };

        TableBuilder::new(ui)
            .auto_shrink(false)
            .min_scrolled_height(scroll_height)
            .max_scroll_height(scroll_height)
            .striped(true)
            .column(Column::remainder())
            .sense(Sense::CLICK)
            .body(|body| {
                body.rows(18.0, 5, |mut row| {
                    // TODO: Recent projects list
                    let row_index = row.index();
                    row.col(|ui| {
                        ui.label(format!("Test {}", row_index + 1));
                    });
                    if row.response().clicked() {
                        error!("TODO");
                    }
                });
            });

        button_strip_measurer.measure(ui, |ui| {
            ui.separator();
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                let _ = ui.button("Cancel");
            });
        });

        if path_changed {
            let picked_path = self.picked_path.clone();
            self.path_validation_result
                .launch(unblock(move || validate_smart_project_path(&picked_path)));
        }
    }
}
