mod cache;

use crate::gfx::{GridModel, Palette, Snes4BppTile, TilemapEntry};
use crate::room::{BLOCK_SIZE_PX, LevelBlock};
use crate::tileset::{LoadedTilesetLayout, OverlaidLayout, Tileset, TiletableEntry};
use crate::ui::tile_view::cache::{TileCacheKey, TileTextureCache};
use crate::util::IteratorArrayExt;
use egui::emath::TSTransform;
use egui::{
    Color32, ColorImage, Mesh, Rangef, Rect, Response, Sense, TextureFilter, TextureHandle,
    TextureOptions, Ui, Vec2, pos2, vec2,
};
use std::ops::Range;
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
    Model: GridModel<Item = LevelBlock>,
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
        let TiletableEntry(subtiles) = (self.tiletable_get)(block.block_id())?;

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
    model: &impl GridModel<Item = LevelBlock>,
    with_transparency: bool,
) -> ([usize; 2], Vec<Color32>) {
    let mut palettes_c32: [_; TilemapEntry::ADDRESSABLE_PALETTES] = layout
        .palette_source
        .palette
        .to_4bpp_color32_lines()
        .collect_to_array_padded(|| [Color32::MAGENTA; Palette::LINE_4BPP_LEN]);
    if with_transparency {
        for line in &mut palettes_c32 {
            line[0] = Color32::TRANSPARENT;
        }
    }

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
    type Item = LevelBlock;

    fn dimensions(&self) -> [usize; 2] {
        [
            Self::BLOCKS_PER_ROW,
            self.len.div_ceil(Self::BLOCKS_PER_ROW),
        ]
    }

    fn get(&self, x: usize, y: usize) -> Option<Self::Item> {
        let tile_id = Self::BLOCKS_PER_ROW * y + x;
        (tile_id < self.len).then(|| LevelBlock::for_tile(tile_id as u16))
    }
}

pub fn get_tileset_ttb_texture(
    ctx: &egui::Context,
    layout: &LoadedTilesetLayout<&Tileset>,
    with_transparency: bool,
) -> TextureHandle {
    let cache_key = TileCacheKey::LoadedTilesetLayout {
        layout: layout.map_refs(Tileset::handle),
        with_transparency,
    };
    TileTextureCache::get_or_insert_with(ctx, cache_key, |ctx, cache_key| {
        let texture_name = cache_key.texture_name();
        let (size, pixels) = tiletable_to_image(
            layout,
            &FullTiletableModel {
                len: layout.tiletable.valid_range().map_or(0, |(_, end)| end),
            },
            with_transparency,
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

pub fn draw_each_screen(
    room_dimensions: [usize; 2],
    cell_size: f32,
    ui: &mut Ui,
    mut add_contents: impl FnMut(&mut Ui, [usize; 2], Rect),
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::from(room_dimensions.map(|x| x as f32)) * cell_size,
        Sense::empty(),
    );

    let screen_transform = TSTransform::new(rect.min.to_vec2(), cell_size);
    let clip_rect_screens = screen_transform.inverse() * ui.clip_rect();

    fn integer_range_containing(r: Rangef, clamp: Range<usize>) -> Range<usize> {
        (r.min.floor() as usize).max(clamp.start)..(r.max.ceil() as usize).min(clamp.end)
    }
    let x_range = integer_range_containing(clip_rect_screens.x_range(), 0..room_dimensions[0]);
    let y_range = integer_range_containing(clip_rect_screens.y_range(), 0..room_dimensions[1]);

    for cell_y in y_range {
        for cell_x in x_range.clone() {
            let cell_rect = Rect::from_min_size(
                screen_transform * pos2(cell_x as f32, cell_y as f32),
                Vec2::splat(cell_size),
            );
            add_contents(ui, [cell_x, cell_y], cell_rect);
        }
    }

    resp
}

pub fn draw_tiletable_grid(
    ui: &mut Ui,
    layout: &LoadedTilesetLayout<&Tileset>,
    model: impl GridModel<Item = LevelBlock>,
    zoom: f32,
    with_transparency: bool,
) -> Response {
    let texture = get_tileset_ttb_texture(ui.ctx(), layout, with_transparency);
    let mut mesh = Mesh::with_texture(texture.id());

    let resp = draw_each_screen(
        model.dimensions(),
        zoom * BLOCK_SIZE_PX as f32,
        ui,
        |_ui, [tile_x, tile_y], rect| {
            let Some(block) = model.get(tile_x, tile_y) else {
                return;
            };

            let texture_row = block.block_id() / FullTiletableModel::BLOCKS_PER_ROW;
            let texture_col = block.block_id() % FullTiletableModel::BLOCKS_PER_ROW;

            let tile_size_in_uv = Vec2::splat(BLOCK_SIZE_PX as f32) / texture.size_vec2();
            let mut uv = Rect::from_min_size(
                (vec2(texture_col as f32, texture_row as f32) * tile_size_in_uv).to_pos2(),
                tile_size_in_uv,
            );
            if block.h_flip() {
                mem::swap(&mut uv.min.x, &mut uv.max.x);
            }
            if block.v_flip() {
                mem::swap(&mut uv.min.y, &mut uv.max.y);
            }

            mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
        },
    );

    ui.painter().add(mesh);

    resp
}
