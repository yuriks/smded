mod cache;

use crate::gfx::{GridModel, Palette, Snes4BppTile, TILE_SIZE, TilemapEntry};
use crate::project::{LevelDataEntry, Tileset, TiletableEntry};
use crate::tileset::TilesetVramLayout;
use crate::ui::tile_view::cache::{TileCacheKey, TileTextureCache};
use crate::util::IteratorArrayExt;
use egui::emath::GuiRounding;
use egui::{
    Color32, ColorImage, Mesh, Rect, Response, Sense, TextureFilter, TextureHandle, TextureOptions,
    Ui, Vec2, pos2, vec2,
};
use std::{array, mem};

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

pub fn get_tileset_gfx_texture(
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

fn tiletable_to_image(
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

pub fn get_tileset_ttb_texture(
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
    fn scale_rect_by_vec2(rect: Rect, scale: Vec2) -> Rect {
        Rect::from_min_max(
            (rect.min.to_vec2() / scale).to_pos2(),
            (rect.max.to_vec2() / scale).to_pos2(),
        )
    }

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

                let (mesh, texture) = meshes_per_palette[tile.palette()].get_or_insert_with(|| {
                    let texture = get_tileset_gfx_texture(
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
