use crate::gfx;
use crate::gfx::{GridModel, Palette, Snes4BppTile, SnesColor};
use crate::project::{
    LevelDataEntry, ProjectData, TilemapEntry, Tileset, TilesetRef, TiletableEntry,
};
use crate::ui::views::EditorWindow;
use crate::ui::{TileCacheKey, TileTextureCache};
use crate::util::IteratorArrayExt;
use egui::emath::GuiRounding;
use egui::load::SizedTexture;
use egui::{
    Color32, ColorImage, Id, Mesh, Rect, Response, Sense, TextureFilter, TextureHandle,
    TextureOptions, Ui, Vec2, pos2, vec2,
};
use gfx::TILE_SIZE;
use std::{array, mem};

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
        tileset: &Tileset,
        palette_line: u8,
    ) -> TextureHandle {
        let cache_key = TileCacheKey::TilesetGfx {
            tileset: tileset.handle(),
            palette_line,
        };
        TileTextureCache::get_or_insert_with(ctx, cache_key, |ctx| {
            let texture_name = format!("tileset[{:?}]-pal[{:X}]", tileset.handle(), palette_line);

            let palette = array::from_fn(|i| {
                if i == 0 {
                    tileset.palette.as_4bpp_lines()[palette_line as usize].map(Color32::from)
                } else {
                    [Color32::TRANSPARENT; Palette::LINE_4BPP_LEN]
                }
            });

            let (size, pixels) = Snes4BppTile::tiles_to_image(
                &tileset.gfx,
                &palette,
                &FullTilesetGfxModel {
                    len: tileset.gfx.len(),
                    palette_index: 0,
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

    fn get_tileset_ttb_texture(ctx: &egui::Context, tileset: &Tileset) -> TextureHandle {
        let cache_key = TileCacheKey::TilesetTtb {
            tileset: tileset.handle(),
        };
        TileTextureCache::get_or_insert_with(ctx, cache_key, |ctx| {
            let texture_name = format!("tileset[{:?}]-ttb", tileset.handle());

            let (size, pixels) = tiletable_to_image(
                tileset,
                &FullTiletableModel {
                    len: tileset.tiletable.len(),
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
                                tileset,
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

struct BlockTilemapModel<'tileset, M>
where
    M: GridModel<Item = LevelDataEntry>,
{
    blocks: &'tileset M,
    tiletable: &'tileset [TiletableEntry],
}

impl<M> GridModel for BlockTilemapModel<'_, M>
where
    M: GridModel<Item = LevelDataEntry>,
{
    type Item = TilemapEntry;

    fn dimensions(&self) -> [usize; 2] {
        let [block_w, block_h] = self.blocks.dimensions();
        [block_w * 2, block_h * 2]
    }

    fn get(&self, x: usize, y: usize) -> Option<Self::Item> {
        let [block_x, block_y] = [x / 2, y / 2];
        let block = self.blocks.get(block_x, block_y)?;
        let TiletableEntry(subtiles) = self.tiletable.get(usize::from(block.block_id()))?;

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
    tileset: &Tileset,
    model: &impl GridModel<Item = LevelDataEntry>,
) -> ([usize; 2], Vec<Color32>) {
    let palettes_c32: [_; 8] = tileset
        .palette
        .to_4bpp_color32_lines()
        .collect_to_array_padded(|| [Color32::TRANSPARENT; Palette::LINE_4BPP_LEN]);

    Snes4BppTile::tiles_to_image(
        &tileset.gfx,
        &palettes_c32,
        &BlockTilemapModel {
            blocks: model,
            tiletable: &tileset.tiletable,
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
        Id::new(concat!(module_path!(), "::TilesetEditor")).with(self.tileset)
    }

    fn show_contents(&mut self, project_data: &mut ProjectData, ui: &mut Ui) {
        let Some(tileset) = self.tileset_mut(project_data) else {
            ui.close();
            return;
        };

        ui.horizontal(|ui| {
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

                    let tex_handle =
                        Self::get_tileset_gfx_texture(ui.ctx(), tileset, self.pal_line as u8);
                    let sized_texture = SizedTexture::from_handle(&tex_handle);

                    // TODO: Implement a band-limited pixel art resizing shader or similar instead
                    let scale_factor = 2.0.round_to_pixels(ui.pixels_per_point());
                    ui.add(egui::Image::new(sized_texture).fit_to_original_size(scale_factor));
                });
            });

            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.label("Tiletable");
                    let tex_handle = Self::get_tileset_ttb_texture(ui.ctx(), tileset);
                    let sized_texture = SizedTexture::from_handle(&tex_handle);

                    let scale_factor = 2.0.round_to_pixels(ui.pixels_per_point());
                    ui.add(egui::Image::new(sized_texture).fit_to_original_size(scale_factor));
                    //Self::draw_tiletable_grid(ui, tileset, 32, scale_factor);`
                })
            });
        });
    }
}
