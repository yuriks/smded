use crate::gfx::Snes4BppTile;
use crate::project::{ProjectData, Tileset, TilesetRef};
use crate::ui::views::EditorWindow;
use egui::load::SizedTexture;
use egui::{ColorImage, Id, Rect, Sense, TextureHandle, TextureOptions, Ui, Vec2, vec2};
use tracing::info;

pub struct TilesetEditor {
    tileset: TilesetRef,
    texture: Option<TextureHandle>,
    pal_line: usize,
}

impl TilesetEditor {
    pub fn new(tileset: TilesetRef) -> Self {
        Self {
            tileset,
            texture: None,
            pal_line: 0,
        }
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

        let palette_lines = tileset.palette.as_4bpp_lines();
        ui.group(|ui| {
            ui.label("Palette");

            const CELL_SIZE: f32 = 16.0;

            let (res, p) = ui.allocate_painter(
                vec2(16.0, palette_lines.len() as f32) * CELL_SIZE,
                Sense::CLICK,
            );
            let mut rect = Rect::from_min_size(res.rect.min, Vec2::splat(CELL_SIZE));
            for line in palette_lines {
                let mut line_rect = rect;
                for color in line {
                    p.rect_filled(line_rect, 0, *color);
                    line_rect = line_rect.translate(vec2(CELL_SIZE, 0.0));
                }
                rect = rect.translate(vec2(0.0, CELL_SIZE));
            }
        });

        ui.group(|ui| {
            ui.label("GFX");
            ui.horizontal(|ui| {
                ui.label("Palette:");
                if ui.add(egui::Slider::new(&mut self.pal_line, 0..=palette_lines.len()-1)).changed() {
                    self.texture = None;
                }
            });

            let tex_handle = self.texture.get_or_insert_with(|| {
                let (size, pixels) =
                    Snes4BppTile::tiles_to_image(&tileset.gfx, &palette_lines[self.pal_line], 16);
                //info!(?size, "Rendered image");
                let image = ColorImage::new(size, pixels);
                ui.ctx().load_texture(
                    format!("tileset[{:?}]-pal[{:X}]", self.tileset, self.pal_line),
                    image,
                    TextureOptions::NEAREST,
                )
            });

            let mut sized_texture = SizedTexture::from_handle(tex_handle);
            sized_texture.size *= 2.0;
            ui.image(sized_texture);
        });
    }
}
