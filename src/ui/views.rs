mod startup_dialog;
mod tileset_editor;
mod workspace;

use crate::project::ProjectData;
use egui::{Context, Id, Response, Ui};

pub use startup_dialog::StartupDialog;
pub use workspace::Workspace;

trait EditorWindow {
    fn title(&self, project_data: &ProjectData) -> String;
    fn stable_id(&self) -> Id;
    fn show_contents(&mut self, project_data: &mut ProjectData, ui: &mut Ui);

    fn show_window(&mut self, project_data: &mut ProjectData, ctx: &Context) -> Option<Response> {
        let mut stay_open = true;
        egui::Window::new(self.title(project_data))
            .id(self.stable_id())
            .open(&mut stay_open)
            .show(ctx, |ui| self.show_contents(project_data, ui))
            .filter(|_| stay_open)
            .map(|inner_r| inner_r.response)
    }
}
