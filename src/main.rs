mod gfx;
mod hex_types;
mod project;
#[allow(unused)]
mod smart_xml;
mod tileset;
mod ui;
mod util;

use crate::project::{ProjectData, load_smart_project};
use crate::ui::promise::{EguiWaker, Promise};
use crate::ui::views::{StartupDialog, Workspace};
use blocking::{Task, unblock};
use eframe::egui;
use egui::{Color32, Context, Frame, Id, StrokeKind, ViewportBuilder, Visuals};
use std::path::PathBuf;
use std::{env, mem};

const APP_ID: &str = "SMDEd";

fn configure_tracing() {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
}

fn main() -> eframe::Result {
    //let cmdline_options = config::cmdline_options().run();
    configure_tracing();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([1920.0, 1080.0]),
        ..Default::default()
    };
    eframe::run_native(
        APP_ID,
        native_options,
        Box::new(|cc| {
            let project_path = env::args_os().nth(1).map(PathBuf::from);
            let app = if let Some(project_path) = project_path {
                Application::with_opened_project(cc, project_path)
            } else {
                Application::new(cc)
            };
            Ok(Box::new(app))
        }),
    )
}

enum ApplicationUiState {
    NoOpenProject(StartupDialog),
    LoadingProject(Promise<Task<anyhow::Result<ProjectData>>>),
    ProjectLoaded(Workspace),
    Invalid, // Used to facilitate state transitions
}

impl ApplicationUiState {
    fn load_project(ctx: &Context, project_path: PathBuf) -> Self {
        ApplicationUiState::LoadingProject(Promise::launched(
            EguiWaker::for_context(ctx),
            unblock(move || load_smart_project(&project_path)),
        ))
    }
}

struct Application {
    state: ApplicationUiState,
}

impl Application {
    fn new(cc: &eframe::CreationContext) -> Self {
        Application {
            state: ApplicationUiState::NoOpenProject(StartupDialog::new(&cc.egui_ctx)),
        }
    }

    fn with_opened_project(cc: &eframe::CreationContext, project_path: PathBuf) -> Self {
        Application {
            state: ApplicationUiState::load_project(&cc.egui_ctx, project_path),
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        ctx.options_mut(|opt| opt.max_passes = 2.try_into().unwrap());

        //_debug_focus(ctx);

        self.state = match mem::replace(&mut self.state, ApplicationUiState::Invalid) {
            ApplicationUiState::NoOpenProject(mut startup_dialog) => {
                let modal_response = egui::Modal::new(Id::new("load_project_modal"))
                    .frame(Frame::window(&ctx.style()))
                    .show(ctx, |ui| startup_dialog.show_contents(ui, frame));

                if modal_response.response.should_close() {
                    ApplicationUiState::load_project(ctx, startup_dialog.get_result())
                } else {
                    ApplicationUiState::NoOpenProject(startup_dialog)
                }
            }
            ApplicationUiState::LoadingProject(mut promise) => {
                egui::Modal::new(Id::new("loading_project_spinner")).show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                        ui.label("Loading project...");
                    });
                });
                if let Some(project) = promise.take_response() {
                    match project {
                        Ok(project) => ApplicationUiState::ProjectLoaded(Workspace::new(project)),
                        Err(e) => {
                            let message = format!("Error loading project: {e}");
                            ApplicationUiState::NoOpenProject(StartupDialog::with_error_message(
                                ctx, message,
                            ))
                        }
                    }
                } else {
                    ApplicationUiState::LoadingProject(promise)
                }
            }
            ApplicationUiState::ProjectLoaded(mut workspace) => {
                workspace.show(ctx);
                ApplicationUiState::ProjectLoaded(workspace)
            }
            ApplicationUiState::Invalid => unreachable!(),
        }
    }

    fn clear_color(&self, visuals: &Visuals) -> [f32; 4] {
        visuals
            .window_fill
            .gamma_multiply(0.5)
            .to_normalized_gamma_f32()
    }
}

fn _debug_focus(ctx: &Context) {
    let Some(focused_id) = ctx.memory(|mem| mem.focused()) else {
        return;
    };

    let Some((mut focused_rect, focused_layer)) = ctx.viewport(|viewport| {
        let focused_info = viewport.prev_pass.widgets.get(focused_id)?;
        Some((focused_info.rect, focused_info.layer_id))
    }) else {
        return;
    };

    let painter = ctx.debug_painter();
    if let Some(layer_transform) = ctx.layer_transform_to_global(focused_layer) {
        focused_rect = layer_transform.mul_rect(focused_rect);
    }
    painter.rect_stroke(
        focused_rect,
        0.0,
        (1.0, Color32::DEBUG_COLOR),
        StrokeKind::Middle,
    );
}
