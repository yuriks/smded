use bit_field::BitField;
use egui::Color32;
use std::iter;
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

type PaletteLine4Bpp<Color> = [Color; Palette::LINE_4BPP_LEN];

impl Palette {
    pub const LINE_4BPP_LEN: usize = 16;

    pub fn as_4bpp_lines(&self) -> &[PaletteLine4Bpp<SnesColor>] {
        let (lines, rest) = self.0.as_chunks();
        if !rest.is_empty() {
            warn!("Palette contains {} leftover entries", rest.len());
        }
        lines
    }

    pub fn to_4bpp_color32_lines(&self) -> impl Iterator<Item = PaletteLine4Bpp<Color32>> {
        self.as_4bpp_lines()
            .iter()
            .map(|line| line.map(Color32::from))
    }

    pub fn truncate_checked(&mut self, new_len: usize) -> Result<(), ()> {
        if new_len > self.0.len() || self.0[new_len..].iter().any(|&SnesColor(x)| x != 0) {
            Err(())
        } else {
            self.0.truncate(new_len);
            Ok(())
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Vec<u16>> for Palette {
    fn from(v: Vec<u16>) -> Self {
        Self(v.into_iter().map(SnesColor).collect())
    }
}

#[derive(Copy, Clone)]
pub struct TilemapEntry(pub u16);

// TODO: Replace with bitfields! macro?
impl TilemapEntry {
    pub fn tile_id(self) -> usize {
        usize::from(self.0.get_bits(0..10))
    }

    pub const H_FLIP_FLAG: u16 = 1 << 14;
    pub fn h_flip(self) -> bool {
        self.0.get_bit(14)
    }

    pub const V_FLIP_FLAG: u16 = 1 << 15;
    pub fn v_flip(self) -> bool {
        self.0.get_bit(15)
    }

    #[expect(unused)]
    pub fn priority(self) -> bool {
        self.0.get_bit(13)
    }

    pub const ADDRESSABLE_PALETTES: usize = 1 << 3;
    pub fn palette(self) -> usize {
        usize::from(self.0.get_bits(10..13))
    }

    // TODO: Silently discards overflow
    pub fn for_tile(tile: usize) -> Self {
        Self((tile & ((1 << 10) - 1)) as u16)
    }

    pub fn with_palette(mut self, pal: usize) -> Self {
        self.0.set_bits(10..13, pal as u16);
        self
    }
}

fn spread_u8_x4(x: u8) -> u32 {
    let mut x = u32::from(x);
    x = 0x000F_000F & (x | x << 12);
    x = 0x0303_0303 & (x | x << 6);
    x = 0x1111_1111 & (x | x << 3);
    x
}

fn decode_bitplanes(bitplanes: [u8; 4]) -> u32 {
    let spread = bitplanes.map(spread_u8_x4);
    spread[0] | spread[1] << 1 | spread[2] << 2 | spread[3] << 3
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Snes4BppTile(pub [u8; TILE_SIZE * 4]);

pub const TILE_SIZE: usize = 8;

impl Snes4BppTile {
    pub fn from_bytes(data: &[u8; 32]) -> Self {
        Self(*data)
    }

    fn bitplane_sets(&self) -> impl Iterator<Item = [u8; 4]> {
        let (pairs, _) = self.0.as_chunks::<2>();
        let (bp01, bp23) = pairs.split_at(TILE_SIZE);
        iter::zip(bp01, bp23).map(|(&[bp0, bp1], &[bp2, bp3])| [bp0, bp1, bp2, bp3])
    }

    pub fn write_to_image<'p, const H_FLIP: bool, const USE_TRANSPARENCY: bool>(
        &self,
        palette: &PaletteLine4Bpp<Color32>,
        output: impl Iterator<Item = &'p mut [Color32; TILE_SIZE]>,
    ) {
        for (mut bp, out_row) in self.bitplane_sets().map(decode_bitplanes).zip(output) {
            for out_p in out_row {
                let index;
                if H_FLIP {
                    index = bp & 0xF;
                    bp >>= 4;
                } else {
                    index = bp >> (32 - 4);
                    bp <<= 4;
                }
                if USE_TRANSPARENCY && index == 0 {
                    continue;
                }
                *out_p = palette[index as usize];
            }
        }
    }

    pub fn write_to_image_flippable<'p, const USE_TRANSPARENCY: bool>(
        &self,
        palette: &PaletteLine4Bpp<Color32>,
        output_slivers: impl DoubleEndedIterator<Item = &'p mut [Color32; TILE_SIZE]>,
        flips: [bool; 2],
    ) {
        match flips {
            [false, false] => self.write_to_image::<false, false>(palette, output_slivers),
            [true, false] => self.write_to_image::<true, false>(palette, output_slivers),
            [false, true] => self.write_to_image::<false, false>(palette, output_slivers.rev()),
            [true, true] => self.write_to_image::<true, false>(palette, output_slivers.rev()),
        }
    }

    pub fn tiles_to_image<'p>(
        mut get_tile: impl FnMut(usize) -> Option<&'p Snes4BppTile>,
        palette: &[PaletteLine4Bpp<Color32>; TilemapEntry::ADDRESSABLE_PALETTES],
        model: &impl GridModel<Item = TilemapEntry>,
    ) -> ([usize; 2], Vec<Color32>) {
        let [tiles_per_row, n_rows] = model.dimensions();
        let [width, height] = [tiles_per_row * TILE_SIZE, n_rows * TILE_SIZE];
        let mut pixels = vec![Color32::TRANSPARENT; width * height];
        let slivers = pixels.as_chunks_mut::<TILE_SIZE>().0;

        for (tile_y, row_slivers) in slivers
            .chunks_exact_mut(tiles_per_row * TILE_SIZE)
            .enumerate()
        {
            for tile_x in 0..tiles_per_row {
                let Some(tile) = model.get(tile_x, tile_y) else {
                    continue;
                };
                let Some(tile_gfx) = get_tile(tile.tile_id()) else {
                    continue;
                };

                let output_slivers = row_slivers[tile_x..].iter_mut().step_by(tiles_per_row);
                tile_gfx.write_to_image_flippable::<false>(
                    &palette[tile.palette()],
                    output_slivers,
                    [tile.h_flip(), tile.v_flip()],
                );
            }
        }

        ([width, height], pixels)
    }
}

pub trait GridModel {
    type Item;

    fn dimensions(&self) -> [usize; 2];
    fn get(&self, x: usize, y: usize) -> Option<Self::Item>;
}
