use crate::project::{ProjectData, Tileset, TilesetRef};
use crate::ui::views::EditorWindow;
use egui::{Id, Rect, Sense, Ui, Vec2, vec2};

pub struct TilesetEditor {
    tileset: TilesetRef,
}

impl TilesetEditor {
    pub fn new(tileset: TilesetRef) -> Self {
        Self { tileset }
    }

    fn tileset<'p>(&self, project_data: &'p ProjectData) -> Option<&'p Tileset> {
        project_data.tilesets.get(self.tileset)
    }

    fn tileset_mut<'p>(&self, project_data: &'p mut ProjectData) -> Option<&'p mut Tileset> {
        project_data.tilesets.get_mut(self.tileset)
    }
}

impl EditorWindow for TilesetEditor {
    fn title(&self, project_data: &ProjectData) -> String {
        let tileset = self.tileset(project_data);
        format!(
            "Tileset: {}",
            tileset.map_or("<UNKNOWN>".into(), |t| t.title())
        )
    }

    fn stable_id(&self) -> Id {
        Id::new(concat!(module_path!(), "::TilesetEditor")).with(self.tileset)
    }

    fn show_contents(&mut self, project_data: &mut ProjectData, ui: &mut Ui) {
        let Some(tileset) = self.tileset_mut(project_data) else {
            ui.close();
            return;
        };

        ui.heading("Palette:");

        const CELL_SIZE: f32 = 16.0;
        let lines = tileset.palette.as_4bpp_lines();

        let (res, p) =
            ui.allocate_painter(vec2(16.0, lines.len() as f32) * CELL_SIZE, Sense::CLICK);
        let mut rect = Rect::from_min_size(res.rect.min, Vec2::splat(CELL_SIZE));
        for line in lines {
            let mut line_rect = rect;
            for color in line {
                p.rect_filled(line_rect, 0, *color);
                line_rect = line_rect.translate(vec2(CELL_SIZE, 0.0));
            }
            rect = rect.translate(vec2(0.0, CELL_SIZE));
        }
    }
}
