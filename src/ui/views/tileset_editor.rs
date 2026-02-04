use crate::gfx::SnesColor;
use crate::project::ProjectData;
use crate::tileset;
use crate::tileset::{Tileset, TilesetKind, TilesetRef};
use crate::ui::tile_view;
use crate::ui::views::EditorWindow;
use egui::emath::GuiRounding;
use egui::load::SizedTexture;
use egui::{Id, Rect, Response, Sense, Ui, Vec2, vec2};

const ID_SALT: &str = concat!(module_path!(), "::TilesetEditor");

pub struct TilesetEditor {
    /// Main tileset being edited. Used for things like title.
    tileset: TilesetRef,
    /// Currently selected CRE to edit together with `tileset`.
    cre_tileset: Option<TilesetRef>,
    /// Current palette line to preview GFX with.
    pal_line: usize,
}

const LAST_USED_CRE_KEY: &str = "last_used_cre";

fn find_default_cre<'p>(ctx: &egui::Context, project_data: &'p ProjectData) -> Option<&'p Tileset> {
    let id = Id::new(ID_SALT).with(LAST_USED_CRE_KEY);
    if let Some(last_used_cre) = ctx.data_mut(|data| data.get_persisted::<TilesetRef>(id))
        && let Some(tileset) = project_data.tilesets.get(last_used_cre)
    {
        return Some(tileset);
    }

    project_data
        .tilesets
        .values()
        .filter(|tileset| tileset.kind == TilesetKind::Cre)
        .min_by(|a, b| a.display_cmp(b))
}

impl TilesetEditor {
    pub fn new(ctx: &egui::Context, tileset: TilesetRef, project_data: &ProjectData) -> Self {
        Self {
            tileset,
            cre_tileset: find_default_cre(ctx, project_data).map(Tileset::handle),
            pal_line: 0,
        }
    }

    fn tileset<'p>(&self, project_data: &'p ProjectData) -> Option<&'p Tileset> {
        project_data.tilesets.get(self.tileset)
    }

    fn draw_palette_grid(ui: &mut Ui, palette_lines: &[[SnesColor; 16]]) -> Response {
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

        res
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
        Id::new(ID_SALT).with(self.tileset)
    }

    fn show_contents(&mut self, project_data: &mut ProjectData, ui: &mut Ui) {
        let Some(tileset) = self.tileset(project_data) else {
            ui.close();
            return;
        };

        let cre_tileset = self
            .cre_tileset
            .and_then(|hnd| project_data.tilesets.get(hnd))
            .or_else(|| find_default_cre(ui.ctx(), project_data));
        let tileset_layout = tileset::detect_sources_layout(tileset, cre_tileset);

        ui.horizontal_centered(|ui| {
            ui.vertical(|ui| {
                let palette_lines = tileset_layout.palette_source.palette.as_4bpp_lines();
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

                    egui::ScrollArea::vertical()
                        .max_height(f32::INFINITY)
                        .id_salt("gfx_scrollarea")
                        .show(ui, |ui| {
                            let tex_handle = tile_view::get_tileset_gfx_texture(
                                ui.ctx(),
                                &tileset_layout.gfx,
                                tileset_layout.palette_source,
                                self.pal_line as u8,
                            );
                            let sized_texture = SizedTexture::from_handle(&tex_handle);

                            // TODO: Implement a band-limited pixel art resizing shader or similar instead
                            let scale_factor = 2.0.round_to_pixels(ui.pixels_per_point());
                            ui.add(
                                egui::Image::new(sized_texture).fit_to_original_size(scale_factor),
                            );
                        });
                });
            });

            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.label("Tiletable");
                    egui::ScrollArea::both()
                        .max_width(f32::INFINITY)
                        .max_height(f32::INFINITY)
                        .id_salt("tiletable_scrollarea")
                        .show(ui, |ui| {
                            let tex_handle =
                                tile_view::get_tileset_ttb_texture(ui.ctx(), &tileset_layout);
                            let sized_texture = SizedTexture::from_handle(&tex_handle);

                            let scale_factor = 2.0.round_to_pixels(ui.pixels_per_point());
                            ui.add(
                                egui::Image::new(sized_texture).fit_to_original_size(scale_factor),
                            );
                            // tile_view::draw_tiletable_grid(ui, &tileset_layout, scale_factor);
                        });
                })
            });
        });
    }
}
