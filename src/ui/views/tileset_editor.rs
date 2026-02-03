use crate::gfx::{GridModel, Palette, Snes4BppTile, SnesColor, TilemapEntry};
use crate::project::{
    LevelDataEntry, ProjectData, Tileset, TilesetKind, TilesetRef, TiletableEntry,
};
use crate::tileset::TilesetVramLayout;
use crate::ui::views::EditorWindow;
use crate::ui::{TileCacheKey, TileTextureCache};
use crate::util::IteratorArrayExt;
use crate::{gfx, tileset};
use egui::emath::GuiRounding;
use egui::load::SizedTexture;
use egui::{
    Color32, ColorImage, Id, Mesh, Rect, Response, Sense, TextureFilter, TextureHandle,
    TextureOptions, Ui, Vec2, pos2, vec2,
};
use gfx::TILE_SIZE;
use std::{array, mem};

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

    fn get_tileset_gfx_texture(
        ctx: &egui::Context,
        layout: &TilesetVramLayout<&Tileset>,
        palette_line: u8,
    ) -> TextureHandle {
        let cache_key = TileCacheKey::VramLayoutGfx {
            layout: layout.map_values(Tileset::handle),
            palette_line,
        };
        TileTextureCache::get_or_insert_with(ctx, cache_key, |ctx, cache_key| {
            let palette = array::from_fn(|i| {
                if i == 0
                    && let Some(palette) = layout.find_palette()
                {
                    palette.as_4bpp_lines()[usize::from(palette_line)].map(Color32::from)
                } else {
                    [Color32::TRANSPARENT; Palette::LINE_4BPP_LEN]
                }
            });

            let (size, pixels) = Snes4BppTile::tiles_to_image(
                |tile_id| {
                    let (tileset, offset) = layout.lookup(tile_id)?;
                    tileset.gfx.get(offset)
                },
                &palette,
                &FullTilesetGfxModel {
                    len: layout.valid_range().map_or(0, |(_, end)| end),
                    palette_index: 0,
                },
            );
            let image = ColorImage::new(size, pixels);

            ctx.load_texture(
                cache_key.texture_name(),
                image,
                TextureOptions {
                    minification: TextureFilter::Linear,
                    ..TextureOptions::NEAREST
                },
            )
        })
    }

    fn get_tileset_ttb_texture(
        ctx: &egui::Context,
        gfx_layout: &TilesetVramLayout<&Tileset>,
        ttb_layout: &TilesetVramLayout<&Tileset>,
    ) -> TextureHandle {
        let cache_key = TileCacheKey::VramLayoutTtb {
            layout: ttb_layout.map_values(Tileset::handle),
        };
        TileTextureCache::get_or_insert_with(ctx, cache_key, |ctx, cache_key| {
            let texture_name = cache_key.texture_name();
            let (size, pixels) = tiletable_to_image(
                gfx_layout,
                ttb_layout,
                &FullTiletableModel {
                    len: ttb_layout.valid_range().map_or(0, |(_, end)| end),
                },
            );
            let image = ColorImage::new(size, pixels);

            ctx.load_texture(
                texture_name,
                image,
                TextureOptions {
                    minification: TextureFilter::Linear,
                    ..TextureOptions::NEAREST
                },
            )
        })
    }

    #[expect(unused)]
    fn draw_tiletable_grid(
        ui: &mut Ui,
        tileset: &Tileset,
        gfx_layout: TilesetVramLayout<&Tileset>,
        entries_per_row: usize,
        scale: f32,
    ) -> Response {
        const CELL_SIZE: usize = TILE_SIZE * 2;

        let ttb = &tileset.tiletable;
        let num_lines = ttb.len().div_ceil(entries_per_row);

        let mut meshes_per_palette = [const { None }; 8]; // TODO constant for num palette lines

        let (res, p) = ui.allocate_painter(
            (CELL_SIZE as f32) * scale * vec2(entries_per_row as f32, num_lines as f32),
            Sense::CLICK,
        );
        // Required to avoid NEAREST filtering artifacts/shimmer
        let rounded_origin = res.rect.min.round_to_pixels(ui.pixels_per_point());

        for (line, y_pos) in ttb.chunks(entries_per_row).zip((0..).step_by(CELL_SIZE)) {
            for (TiletableEntry(tiles), x_pos) in line.iter().zip((0..).step_by(CELL_SIZE)) {
                for (tile, rect_offset) in tiles.iter().zip(&[
                    (0, 0),
                    (TILE_SIZE, 0),
                    (0, TILE_SIZE),
                    (TILE_SIZE, TILE_SIZE),
                ]) {
                    if tile.tile_id() >= tileset.gfx.len() {
                        continue;
                    }

                    let tile_rect = Rect::from_min_size(
                        pos2(
                            (x_pos + rect_offset.0) as f32,
                            (y_pos + rect_offset.1) as f32,
                        ),
                        Vec2::splat(TILE_SIZE as f32),
                    );

                    let (mesh, texture) =
                        meshes_per_palette[tile.palette()].get_or_insert_with(|| {
                            let texture = Self::get_tileset_gfx_texture(
                                ui.ctx(),
                                &gfx_layout,
                                u8::try_from(tile.palette()).unwrap(),
                            );
                            (Mesh::with_texture(texture.id()), texture)
                        });
                    let tile_row = tile.tile_id() / (texture.size()[0] / TILE_SIZE);
                    let tile_col = tile.tile_id() % (texture.size()[0] / TILE_SIZE);

                    let mut uv = Rect::from_min_size(
                        pos2((tile_col * TILE_SIZE) as f32, (tile_row * TILE_SIZE) as f32),
                        Vec2::splat(TILE_SIZE as f32),
                    );
                    if tile.h_flip() {
                        mem::swap(&mut uv.min.x, &mut uv.max.x);
                    }
                    if tile.v_flip() {
                        mem::swap(&mut uv.min.y, &mut uv.max.y);
                    }

                    mesh.add_rect_with_uv(
                        (tile_rect * scale).translate(rounded_origin.to_vec2()),
                        scale_rect_by_vec2(uv, texture.size_vec2()),
                        Color32::WHITE,
                    );
                }
            }
        }

        for (mesh, _) in meshes_per_palette.into_iter().flatten() {
            p.add(mesh);
        }

        res
    }
}

struct BlockTilemapModel<'tileset, M, F> {
    blocks: &'tileset M,
    tiletable_get: F,
}

impl<M, F> GridModel for BlockTilemapModel<'_, M, F>
where
    M: GridModel<Item = LevelDataEntry>,
    F: Fn(usize) -> Option<TiletableEntry>,
{
    type Item = TilemapEntry;

    fn dimensions(&self) -> [usize; 2] {
        let [block_w, block_h] = self.blocks.dimensions();
        [block_w * 2, block_h * 2]
    }

    fn get(&self, x: usize, y: usize) -> Option<Self::Item> {
        let [block_x, block_y] = [x / 2, y / 2];
        let block = self.blocks.get(block_x, block_y)?;
        let TiletableEntry(subtiles) = (self.tiletable_get)(usize::from(block.block_id()))?;

        let [mut subtile_x, mut subtile_y] = [x % 2, y % 2];
        if block.h_flip() {
            subtile_x ^= 1;
        }
        if block.v_flip() {
            subtile_y ^= 1;
        }

        let mut subtile = subtiles[subtile_y * 2 + subtile_x];
        if block.h_flip() {
            subtile.0 ^= TilemapEntry::H_FLIP_FLAG;
        }
        if block.v_flip() {
            subtile.0 ^= TilemapEntry::V_FLIP_FLAG;
        }

        Some(subtile)
    }
}

pub fn tiletable_to_image(
    gfx_layout: &TilesetVramLayout<&Tileset>,
    ttb_layout: &TilesetVramLayout<&Tileset>,
    model: &impl GridModel<Item = LevelDataEntry>,
) -> ([usize; 2], Vec<Color32>) {
    let palettes_c32: [_; 8] = ttb_layout
        .find_palette()
        .iter()
        .flat_map(|p| p.to_4bpp_color32_lines())
        .collect_to_array_padded(|| [Color32::TRANSPARENT; Palette::LINE_4BPP_LEN]);

    Snes4BppTile::tiles_to_image(
        |tile_id| {
            let (tileset, offset) = gfx_layout.lookup(tile_id)?;
            tileset.gfx.get(offset)
        },
        &palettes_c32,
        &BlockTilemapModel {
            blocks: model,
            tiletable_get: |i| {
                let (tileset, offset) = ttb_layout.lookup(i)?;
                tileset.tiletable.get(offset).copied()
            },
        },
    )
}

struct FullTilesetGfxModel {
    len: usize,
    palette_index: usize,
}

impl FullTilesetGfxModel {
    const TILES_PER_ROW: usize = 16;
}

impl GridModel for FullTilesetGfxModel {
    type Item = TilemapEntry;

    fn dimensions(&self) -> [usize; 2] {
        [Self::TILES_PER_ROW, self.len.div_ceil(Self::TILES_PER_ROW)]
    }

    fn get(&self, x: usize, y: usize) -> Option<Self::Item> {
        let tile_id = Self::TILES_PER_ROW * y + x;
        (tile_id < self.len)
            .then(|| TilemapEntry::for_tile(tile_id).with_palette(self.palette_index))
    }
}

struct FullTiletableModel {
    len: usize,
}

impl FullTiletableModel {
    const BLOCKS_PER_ROW: usize = 32;
}

impl GridModel for FullTiletableModel {
    type Item = LevelDataEntry;

    fn dimensions(&self) -> [usize; 2] {
        [
            Self::BLOCKS_PER_ROW,
            self.len.div_ceil(Self::BLOCKS_PER_ROW),
        ]
    }

    fn get(&self, x: usize, y: usize) -> Option<Self::Item> {
        let tile_id = Self::BLOCKS_PER_ROW * y + x;
        (tile_id < self.len).then(|| LevelDataEntry::for_tile(tile_id as u16))
    }
}

fn scale_rect_by_vec2(rect: Rect, scale: Vec2) -> Rect {
    Rect::from_min_max(
        (rect.min.to_vec2() / scale).to_pos2(),
        (rect.max.to_vec2() / scale).to_pos2(),
    )
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
        let (gfx_layout, ttb_layout) = tileset::detect_sources_layout(tileset, cre_tileset);

        ui.horizontal_centered(|ui| {
            ui.vertical(|ui| {
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

                    egui::ScrollArea::vertical()
                        .max_height(f32::INFINITY)
                        .id_salt("gfx_scrollarea")
                        .show(ui, |ui| {
                            let tex_handle = Self::get_tileset_gfx_texture(
                                ui.ctx(),
                                &gfx_layout,
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
                                Self::get_tileset_ttb_texture(ui.ctx(), &gfx_layout, &ttb_layout);
                            let sized_texture = SizedTexture::from_handle(&tex_handle);

                            let scale_factor = 2.0.round_to_pixels(ui.pixels_per_point());
                            ui.add(
                                egui::Image::new(sized_texture).fit_to_original_size(scale_factor),
                            );
                            //Self::draw_tiletable_grid(ui, tileset, 32, scale_factor);`
                        });
                })
            });
        });
    }
}
