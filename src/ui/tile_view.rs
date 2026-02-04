mod cache;

use crate::gfx::{GridModel, Palette, Snes4BppTile, TILE_SIZE, TilemapEntry};
use crate::room::LevelDataEntry;
use crate::tileset::{LoadedTilesetLayout, OverlaidLayout, Tileset, TiletableEntry};
use crate::ui::tile_view::cache::{TileCacheKey, TileTextureCache};
use crate::util::IteratorArrayExt;
use egui::emath::GuiRounding;
use egui::{
    Color32, ColorImage, Mesh, Rect, Response, Sense, TextureFilter, TextureHandle, TextureOptions,
    Ui, Vec2, pos2,
};
use std::{iter, mem};

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
    gfx_layout: &OverlaidLayout<&Tileset>,
    palette_source: &Tileset,
    palette_line: u8,
) -> TextureHandle {
    let cache_key = TileCacheKey::LoadedGfxLayout {
        gfx_layout: gfx_layout.map_ref(Tileset::handle),
        palette_source: palette_source.handle(),
        palette_line,
    };
    TileTextureCache::get_or_insert_with(ctx, cache_key, |ctx, cache_key| {
        let palette_line = &palette_source.palette.as_4bpp_lines()[usize::from(palette_line)];
        let palette = iter::once(palette_line.map(Color32::from))
            .collect_to_array_padded(|| [Color32::MAGENTA; Palette::LINE_4BPP_LEN]);

        let (size, pixels) = Snes4BppTile::tiles_to_image(
            |tile_id| {
                let (tileset, offset) = gfx_layout.lookup(tile_id)?;
                tileset.gfx.get(offset)
            },
            &palette,
            &FullTilesetGfxModel {
                len: gfx_layout.valid_range().map_or(0, |(_, end)| end),
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

struct BlockTilemapModel<'tileset, Model, F> {
    blocks: &'tileset Model,
    tiletable_get: F,
}

impl<Model, F> GridModel for BlockTilemapModel<'_, Model, F>
where
    Model: GridModel<Item = LevelDataEntry>,
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
    layout: &LoadedTilesetLayout<&Tileset>,
    model: &impl GridModel<Item = LevelDataEntry>,
) -> ([usize; 2], Vec<Color32>) {
    let palettes_c32: [_; TilemapEntry::ADDRESSABLE_PALETTES] = layout
        .palette_source
        .palette
        .to_4bpp_color32_lines()
        .collect_to_array_padded(|| [Color32::MAGENTA; Palette::LINE_4BPP_LEN]);

    Snes4BppTile::tiles_to_image(
        |tile_id| {
            let (tileset, offset) = layout.gfx.lookup(tile_id)?;
            tileset.gfx.get(offset)
        },
        &palettes_c32,
        &BlockTilemapModel {
            blocks: model,
            tiletable_get: |i| {
                let (tileset, offset) = layout.tiletable.lookup(i)?;
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
    layout: &LoadedTilesetLayout<&Tileset>,
) -> TextureHandle {
    let cache_key = TileCacheKey::LoadedTilesetLayout {
        layout: layout.map_refs(Tileset::handle),
    };
    TileTextureCache::get_or_insert_with(ctx, cache_key, |ctx, cache_key| {
        let texture_name = cache_key.texture_name();
        let (size, pixels) = tiletable_to_image(
            layout,
            &FullTiletableModel {
                len: layout.tiletable.valid_range().map_or(0, |(_, end)| end),
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
pub fn draw_tiletable_grid(
    ui: &mut Ui,
    layout: &LoadedTilesetLayout<&Tileset>,
    scale: f32,
) -> Response {
    fn scale_rect_by_vec2(rect: Rect, scale: Vec2) -> Rect {
        Rect::from_min_max(
            (rect.min.to_vec2() / scale).to_pos2(),
            (rect.max.to_vec2() / scale).to_pos2(),
        )
    }

    fn rect_for_grid_cell(cell_x: usize, cell_y: usize, cell_size: f32) -> Rect {
        Rect::from_min_size(
            pos2(cell_x as f32, cell_y as f32) * cell_size,
            Vec2::splat(cell_size),
        )
    }

    let model = BlockTilemapModel {
        blocks: &FullTiletableModel {
            len: layout.tiletable.valid_range().map_or(0, |(_, end)| end),
        },
        tiletable_get: |i| {
            let (tileset, offset) = layout.tiletable.lookup(i)?;
            tileset.tiletable.get(offset).copied()
        },
    };

    let mut meshes_per_palette = [const { None }; TilemapEntry::ADDRESSABLE_PALETTES];

    let (res, p) = ui.allocate_painter(
        scale * Vec2::from(model.dimensions().map(|x| (x * TILE_SIZE) as f32)),
        Sense::CLICK,
    );
    // Required to avoid NEAREST filtering artifacts/shimmer
    let rounded_origin = res.rect.min.round_to_pixels(ui.pixels_per_point());

    for tile_y in 0..model.dimensions()[1] {
        for tile_x in 0..model.dimensions()[0] {
            let Some(tile) = model.get(tile_x, tile_y) else {
                continue;
            };

            let tile_rect = rect_for_grid_cell(tile_x, tile_y, TILE_SIZE as f32);

            let (mesh, texture) = meshes_per_palette[tile.palette()].get_or_insert_with(|| {
                let texture = get_tileset_gfx_texture(
                    ui.ctx(),
                    &layout.gfx,
                    layout.palette_source,
                    u8::try_from(tile.palette()).unwrap(),
                );
                (Mesh::with_texture(texture.id()), texture)
            });
            let texture_row = tile.tile_id() / (texture.size()[0] / TILE_SIZE);
            let texture_col = tile.tile_id() % (texture.size()[0] / TILE_SIZE);

            let mut uv = rect_for_grid_cell(texture_col, texture_row, TILE_SIZE as f32);
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

    for (mesh, _) in meshes_per_palette.into_iter().flatten() {
        p.add(mesh);
    }

    res
}
