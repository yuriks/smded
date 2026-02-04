use crate::project::ProjectData;
use crate::room::RoomRef;
use crate::ui::views::EditorWindow;
use egui::{Id, Ui};

const ID_SALT: &str = concat!(module_path!(), "::RoomEditor");

pub struct RoomEditor {
    room: RoomRef,
}

impl RoomEditor {
    pub fn new(room: RoomRef) -> Self {
        Self { room }
    }
}

impl EditorWindow for RoomEditor {
    fn title(&self, project_data: &ProjectData) -> String {
        if let Some(room) = project_data.rooms.get(self.room) {
            format!("Room: {}", &room.name)
        } else {
            format!("Room: <{:?}>", self.room)
        }
    }

    fn stable_id(&self) -> Id {
        Id::new(ID_SALT).with(self.room)
    }

    fn show_contents(&mut self, project_data: &mut ProjectData, ui: &mut Ui) {
        let Some(_room) = project_data.rooms.get(self.room) else {
            ui.close();
            return;
        };
    }
}
