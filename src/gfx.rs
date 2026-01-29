use eframe::epaint::Color32;
use std::{array, iter};
use tracing::warn;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct SnesColor(pub u16);

impl SnesColor {
    pub fn as_rgb_5bpc(self) -> [u16; 3] {
        [(self.0) & 0x1F, (self.0 >> 5) & 0x1F, (self.0 >> 10) & 0x1F]
    }

    /// Returns color as a PC RGB triplet in 0-255 (i.e. the range is expanded)
    pub fn as_rgb_8bpc(self) -> [u8; 3] {
        self.as_rgb_5bpc()
            .map(|x| { (x * 0xFF + (0x1F / 2)) / 0x1F } as u8)
    }
}

impl From<SnesColor> for Color32 {
    fn from(value: SnesColor) -> Self {
        let [r, g, b] = value.as_rgb_8bpc();
        Color32::from_rgb(r, g, b)
    }
}

pub struct Palette(pub Vec<SnesColor>);

impl Palette {
    pub const LINE_4BPP_LEN: usize = 16;
    pub const LINE_2BPP_LEN: usize = 4;

    pub fn as_4bpp_lines(&self) -> &[[SnesColor; Self::LINE_4BPP_LEN]] {
        let (lines, rest) = self.0.as_chunks();
        if !rest.is_empty() {
            warn!("Palette contains {} leftover entries", rest.len());
        }
        lines
    }

    #[expect(unused)]
    pub fn as_2bpp_lines(&self) -> &[[SnesColor; Self::LINE_2BPP_LEN]] {
        let (lines, rest) = self.0.as_chunks();
        if !rest.is_empty() {
            warn!("Palette contains {} leftover entries", rest.len());
        }
        lines
    }

    pub fn truncate_checked(&mut self, new_len: usize) -> Result<(), ()> {
        if new_len > self.0.len() || self.0[new_len..].iter().any(|&SnesColor(x)| x != 0) {
            Err(())
        } else {
            self.0.truncate(new_len);
            Ok(())
        }
    }
}

impl From<Vec<u16>> for Palette {
    fn from(v: Vec<u16>) -> Self {
        Self(v.into_iter().map(SnesColor).collect())
    }
}

fn spread_u16_x4(x: u16) -> u64 {
    let mut x = u64::from(x);
    x = 0x0000_00FF_0000_00FF & (x | x << 24);
    x = 0x000F_000F_000F_000F & (x | x << 12);
    x = 0x0303_0303_0303_0303 & (x | x << 6);
    x = 0x1111_1111_1111_1111 & (x | x << 3);
    x
}

fn decode_bitplane_pair([bp01, bp23]: [u16; 2]) -> u32 {
    let bp0_bp1 = spread_u16_x4(bp01);
    let bp2_bp3 = spread_u16_x4(bp23);
    (bp0_bp1 | bp0_bp1 >> (32 - 1)) as u32 | (bp2_bp3 << 2 | bp2_bp3 >> (32 - 3)) as u32
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Snes4BppTile(pub [u16; TILE_SIZE * 2]);

pub const TILE_SIZE: usize = 8;

impl Snes4BppTile {
    pub fn from_bytes(data: &[u8; 32]) -> Self {
        let (pairs, _) = data.as_chunks::<2>();
        Self(array::from_fn(|i| u16::from_le_bytes(pairs[i])))
    }

    fn bitplane_pairs(&self) -> [[u16; 2]; TILE_SIZE] {
        array::from_fn(|i| [self.0[i], self.0[i + TILE_SIZE]])
    }

    fn interleaved_rows(&self) -> [u32; TILE_SIZE] {
        self.bitplane_pairs().map(decode_bitplane_pair)
    }

    pub fn write_to_image<'p, const USE_TRANSPARENCY: bool>(
        &self,
        palette: &[Color32; Palette::LINE_4BPP_LEN],
        output: impl Iterator<Item = &'p mut [Color32; TILE_SIZE]>,
    ) {
        for (mut bp, out_row) in iter::zip(self.interleaved_rows(), output) {
            for out_p in out_row {
                let index = bp >> (32 - 4);
                bp <<= 4;
                if USE_TRANSPARENCY && index == 0 {
                    continue;
                }
                *out_p = palette[index as usize];
            }
        }
    }

    pub fn tiles_to_image(
        tiles: &[Snes4BppTile],
        palette: &[SnesColor; Palette::LINE_4BPP_LEN],
        tiles_per_row: usize,
    ) -> ([usize; 2], Vec<Color32>) {
        let n_rows = tiles.len().div_ceil(tiles_per_row);
        let [width, height] = [tiles_per_row * TILE_SIZE, n_rows * TILE_SIZE];
        let mut pixels = vec![Color32::TRANSPARENT; width * height];
        let slivers = pixels.as_chunks_mut::<TILE_SIZE>().0;

        let palette_c32 = palette.map(Color32::from);

        for (tiles, row_slivers) in iter::zip(
            tiles.chunks(tiles_per_row),
            slivers.chunks_mut(tiles_per_row * TILE_SIZE),
        ) {
            for (column, tile) in tiles.iter().enumerate() {
                let output_slivers = row_slivers[column..].iter_mut().step_by(tiles_per_row);
                tile.write_to_image::<false>(&palette_c32, output_slivers);
            }
        }

        ([width, height], pixels)
    }
}
