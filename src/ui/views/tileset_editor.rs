use crate::gfx::{Snes4BppTile, SnesColor};
use crate::project::{ProjectData, Tileset, TilesetRef};
use crate::ui::views::EditorWindow;
use crate::ui::{TileCacheKey, TileTextureCache};
use egui::emath::GuiRounding;
use egui::load::SizedTexture;
use egui::{ColorImage, Id, Rect, Sense, TextureHandle, TextureOptions, Ui, Vec2, vec2};

pub struct TilesetEditor {
    tileset: TilesetRef,
    pal_line: usize,
}

impl TilesetEditor {
    pub fn new(tileset: TilesetRef) -> Self {
        Self {
            tileset,
            pal_line: 0,
        }
    }

    fn tileset<'p>(&self, project_data: &'p ProjectData) -> Option<&'p Tileset> {
        project_data.tilesets.get(self.tileset)
    }

    fn tileset_mut<'p>(&self, project_data: &'p mut ProjectData) -> Option<&'p mut Tileset> {
        project_data.tilesets.get_mut(self.tileset)
    }

    fn draw_palette_grid(ui: &mut Ui, palette_lines: &[[SnesColor; 16]]) {
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
    }

    fn get_tileset_texture(
        ctx: &egui::Context,
        tileset: &Tileset,
        palette_line: u8,
    ) -> TextureHandle {
        let cache_key = TileCacheKey::Tileset {
            tileset: tileset.handle(),
            palette_line,
        };
        TileTextureCache::get_or_insert_with(ctx, cache_key, |ctx| {
            let texture_name = format!("tileset[{:?}]-pal[{:X}]", tileset.handle(), palette_line);
            let palette_line = &tileset.palette.as_4bpp_lines()[palette_line as usize];

            let (size, pixels) = Snes4BppTile::tiles_to_image(&tileset.gfx, palette_line, 16);
            let image = ColorImage::new(size, pixels);

            ctx.load_texture(texture_name, image, TextureOptions::NEAREST)
        })
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
            Self::draw_palette_grid(ui, palette_lines);
        });

        ui.group(|ui| {
            ui.label("GFX");
            ui.horizontal(|ui| {
                ui.label("Palette:");
                ui.add(egui::Slider::new(
                    &mut self.pal_line,
                    0..=palette_lines.len() - 1,
                ));
            });

            let tex_handle = Self::get_tileset_texture(ui.ctx(), tileset, self.pal_line as u8);
            let sized_texture = SizedTexture::from_handle(&tex_handle);

            // TODO: Implement a band-limited pixel art resizing shader or similar instead
            let scale_factor = 2.0.round_to_pixels(ui.pixels_per_point());
            ui.add(egui::Image::new(sized_texture).fit_to_original_size(scale_factor));
        });
    }
}
