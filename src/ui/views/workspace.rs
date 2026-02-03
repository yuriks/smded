use crate::project::ProjectData;
use crate::ui::views::EditorWindow;
use crate::ui::views::tileset_editor::TilesetEditor;
use egui::{LayerId, Order};

pub struct Workspace {
    project_data: ProjectData,

    open_editors: Vec<Box<dyn EditorWindow>>,
}

impl Workspace {
    pub fn new(project_data: ProjectData) -> Self {
        Self {
            project_data,
            open_editors: Vec::new(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("editor_list").show(ctx, |ui| {
            ui.collapsing("Tilesets", |ui| {
                for (tileset_ref, tileset) in &self.project_data.tilesets {
                    if tileset.palette.is_empty() {
                        continue;
                    }
                    if ui
                        .add(egui::Button::new(tileset.title()).frame_when_inactive(false))
                        .clicked()
                    {
                        let editor = TilesetEditor::new(ctx, tileset_ref, &self.project_data);

                        // If there's an existing editor open, bring that to front instead
                        let editor_id = editor.stable_id();
                        if let Some(existing_id) = self
                            .open_editors
                            .iter()
                            .map(|e| e.stable_id())
                            .find(|id| *id == editor_id)
                        {
                            let layer_id = LayerId::new(Order::Middle, existing_id);
                            ctx.move_to_top(layer_id);
                        } else {
                            self.open_editors.push(Box::new(editor));
                        }
                    }
                }
            });

            ui.allocate_space(ui.available_size());
        });

        self.open_editors.retain_mut(|editor| {
            let response = editor.show_window(&mut self.project_data, ctx);
            let should_close = response.is_none_or(|r| r.should_close());
            !should_close
        });
    }
}
