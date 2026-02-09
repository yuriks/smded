use crate::project::ProjectData;
use crate::ui::views::EditorWindow;
use crate::ui::views::room_editor::RoomEditor;
use crate::ui::views::tileset_editor::TilesetEditor;
use egui::{LayerId, Order};
use heck::ToTitleCase;
use itertools::Itertools;

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

    fn open_editor(&mut self, ctx: &egui::Context, editor: Box<dyn EditorWindow>) {
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
            self.open_editors.push(editor);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        let mut new_editor: Option<Box<dyn EditorWindow>> = None;

        egui::SidePanel::left("editor_list").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.collapsing("Rooms", |ui| {
                    let rooms_by_area = self
                        .project_data
                        .rooms
                        .iter()
                        .chunk_by(|(_, room)| room.index().map(|(area, _)| area));
                    for (area, area_rooms) in &rooms_by_area {
                        let area_str = if let Some(area) = area {
                            format!("Area {area}")
                        } else {
                            "Area ?".into()
                        };

                        ui.collapsing(area_str, |ui| {
                            for (room_ref, room) in area_rooms {
                                let print_name = room.name.to_title_case();
                                let room_str = if let Some((_, room)) = room.index() {
                                    format!("[{room:02X}] {print_name}")
                                } else {
                                    format!("[??] {print_name}")
                                };
                                if ui
                                    .add(egui::Button::new(room_str).frame_when_inactive(false))
                                    .clicked()
                                {
                                    new_editor = Some(Box::new(RoomEditor::new(room_ref)));
                                }
                            }
                        });
                    }
                });
                ui.collapsing("Tilesets", |ui| {
                    for (tileset_ref, tileset) in &self.project_data.tilesets {
                        if tileset.palette.is_empty() {
                            continue;
                        }
                        if ui
                            .add(egui::Button::new(tileset.title()).frame_when_inactive(false))
                            .clicked()
                        {
                            new_editor = Some(Box::new(TilesetEditor::new(
                                ctx,
                                tileset_ref,
                                &self.project_data,
                            )));
                        }
                    }
                });

                ui.allocate_space(ui.available_size());
            });
        });

        if let Some(new_editor) = new_editor {
            self.open_editor(ctx, new_editor);
        }
        self.open_editors.retain_mut(|editor| {
            let response = editor.show_window(&mut self.project_data, ctx);
            let should_close = response.is_none_or(|r| r.should_close());
            !should_close
        });
    }
}
