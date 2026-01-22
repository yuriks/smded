use crate::project::ProjectData;

pub struct Workspace {
    project_data: ProjectData,
}

impl Workspace {
    pub fn new(project_data: ProjectData) -> Self {
        Self { project_data }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("editor_list").show(ctx, |ui| {
            ui.collapsing("Tilesets", |ui| {
                for tileset in self.project_data.tilesets.values() {
                    let text = if let Some(index) = tileset.index() {
                        format!("{index:02X} - {}", tileset.name)
                    } else {
                        format!("?? - {}", tileset.name)
                    };
                    ui.add(egui::Button::new(text).frame_when_inactive(false));
                }
            });

            ui.allocate_space(ui.available_size());
        });
    }
}
