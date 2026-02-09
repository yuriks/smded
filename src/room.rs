use crate::gfx::TILE_SIZE;
use crate::hex_types::{HexU8, HexU16};
use crate::smart_xml;
use crate::tileset::TilesetIndex;
use anyhow::anyhow;
use bit_field::BitField;
use heck::ToTitleCase;
use std::array;
use std::iter::Peekable;
use tracing::{error, warn};

/// Width/height of a screen, in pixels
pub const SCREEN_SIZE_PX: usize = TILE_SIZE * 32;
/// Width/height of a block/metatile, in pixels
pub const BLOCK_SIZE_PX: usize = TILE_SIZE * 2;
/// Width/height of a screen, in 16x16 blocks.
pub const SCREEN_SIZE_BLOCKS: usize = SCREEN_SIZE_PX / BLOCK_SIZE_PX;
pub const SCREEN_AREA_BLOCKS: usize = SCREEN_SIZE_BLOCKS.pow(2);

#[derive(Copy, Clone)]
pub struct LevelBlock(pub u16);

impl LevelBlock {
    /// Tile index into the tiletable.
    pub fn block_id(self) -> usize {
        usize::from(self.0.get_bits(0..10))
    }

    pub fn h_flip(self) -> bool {
        self.0.get_bit(10)
    }

    pub fn v_flip(self) -> bool {
        self.0.get_bit(11)
    }

    #[expect(unused)]
    pub fn block_type(self) -> u8 {
        self.0.get_bits(12..) as u8
    }

    // TODO: Silently discards overflow
    pub fn for_tile(tile: u16) -> Self {
        Self(tile & ((1 << 10) - 1))
    }

    #[expect(unused)]
    pub fn with_flips(mut self, h_flip: bool, v_flip: bool) -> Self {
        self.0.set_bit(11, h_flip);
        self.0.set_bit(12, v_flip);
        self
    }
}

pub struct LayerScreenGrid<T> {
    /// Width of the layer, in screens
    width: u8,
    /// Height of the layer, in screens
    height: u8,
    screens: Vec<T>,
}

fn row_order_coords(width: u8, height: u8) -> impl Iterator<Item = (u8, u8)> {
    (0..height).flat_map(move |y| (0..width).map(move |x| (x, y)))
}

impl<T> LayerScreenGrid<T> {
    #[expect(clippy::type_complexity)]
    fn from_iter(
        [width, height]: [u8; 2],
        mut default_fn: impl FnMut() -> T,
        iter: impl IntoIterator<Item = ((u8, u8), T)>,
    ) -> (Self, Peekable<impl Iterator<Item = ((u8, u8), T)>>) {
        let mut screens = Vec::from_iter(iter);
        screens.sort_unstable_by_key(|((x, y), _)| (*y, *x));

        let mut iter = screens.into_iter().peekable();
        let screens = row_order_coords(width, height)
            .map(|expected_pos| {
                iter.next_if(|(pos, _)| *pos == expected_pos)
                    .map(|(_, screen)| screen)
                    .unwrap_or_else(&mut default_fn)
            })
            .collect();

        (
            LayerScreenGrid {
                width,
                height,
                screens,
            },
            iter,
        )
    }

    pub fn dimensions(&self) -> [usize; 2] {
        [usize::from(self.width), usize::from(self.height)]
    }

    pub fn get_screen(&self, x: usize, y: usize) -> Option<&T> {
        if x >= usize::from(self.width) {
            return None;
        }
        self.screens.get(y * self.width as usize + x)
    }
}

pub struct BlockScreenLayer {
    pub blocks: LayerScreenGrid<[LevelBlock; SCREEN_AREA_BLOCKS]>,
}

impl BlockScreenLayer {}

#[expect(unused)]
pub struct CollisionScreenLayer {
    blocks: LayerScreenGrid<[u8; SCREEN_AREA_BLOCKS]>,
}

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ScrollType {
    // If anyone ever figures out how to name these other than the colors, I'm all ears.
    Red = 0,
    Blue = 1,
    Green = 2,
}

impl From<ScrollType> for u8 {
    fn from(value: ScrollType) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for ScrollType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ScrollType::Red),
            1 => Ok(ScrollType::Blue),
            2 => Ok(ScrollType::Green),
            _ => Err(()),
        }
    }
}

pub struct ScrollsLayer {
    pub scrolls: LayerScreenGrid<u8>,
}

pub struct RoomState {
    pub tileset_index: TilesetIndex,

    // Layers
    /// Foreground block layer. Also stores main collision type in `block_type`.
    pub layer1: BlockScreenLayer,
    /// Aka "BTS". Additional sub-type or parameter used by most block types.
    #[expect(unused)]
    pub extended_block_type: CollisionScreenLayer,
    /// Background block layer. `block_type` bits are unused.
    pub layer2: Option<BlockScreenLayer>,
    /// Camera scrolling type for each scroll.
    pub scrolls: ScrollsLayer,
    // TODO: BgData, Scroll doorcode/PLMs (as "attached layers"), save stations, ...
}

slotmap::new_key_type! { pub struct RoomRef; }
pub type RoomIndex = (u8, u8);

pub struct Room {
    handle: RoomRef,
    index: Option<RoomIndex>,

    pub name: String,
    pub width: u8,
    pub height: u8,

    pub states: Vec<RoomState>,
}

impl Room {
    #[expect(unused)]
    pub fn handle(&self) -> RoomRef {
        self.handle
    }

    pub fn index(&self) -> Option<RoomIndex> {
        self.index
    }

    pub fn title(&self) -> String {
        let print_name = self.name.to_title_case();
        if let Some((area, room)) = self.index {
            format!("[{area:02X},{room:02X}] {print_name}")
        } else {
            format!("[??,??] {print_name}")
        }
    }
}

fn convert_block_layer<T, U>(
    dimensions: [u8; 2],
    default_fn: impl Fn() -> U,
    mut map_fn: impl FnMut(T) -> U,
    layer: smart_xml::LevelDataLayer<T>,
) -> Result<LayerScreenGrid<[U; SCREEN_AREA_BLOCKS]>, ()> {
    let (blocks, mut remaining) = LayerScreenGrid::from_iter(
        dimensions,
        || array::from_fn(|_| default_fn()),
        layer.screens.into_iter().map(|screen| {
            let mut data: Vec<_> = screen.data.into_iter().map(&mut map_fn).collect();
            data.resize_with(SCREEN_AREA_BLOCKS, &default_fn);

            ((screen.x.0, screen.y.0), data.try_into().ok().unwrap())
        }),
    );

    if remaining.peek().is_none() {
        Ok(blocks)
    } else {
        for (pos, _) in remaining {
            error!("Out-of-bounds screen @ {pos:?}");
        }
        Err(())
    }
}

pub fn load_from_smart(
    index: RoomIndex,
    room_name: String,
    room: smart_xml::Room,
    handle: RoomRef,
) -> anyhow::Result<Room> {
    let width = room.width.into();
    let height = room.height.into();

    let mut states = Vec::new();
    for smart_state in room.states.into_iter() {
        let smart_xml::RoomState {
            level_data,
            gfx_set: HexU8(tileset_index),
            scroll_data,
            ..
        } = smart_state;

        let dimensions = [level_data.width, level_data.height].map(u8::from);
        let layer1 = {
            let blocks = convert_block_layer(
                dimensions,
                || LevelBlock(0),
                |HexU16(b)| LevelBlock(b),
                level_data.layer1,
            );
            BlockScreenLayer {
                blocks: blocks.map_err(|()| anyhow!("Out-of-bounds screens in Layer1"))?,
            }
        };
        let extended_block_type = {
            let blocks = convert_block_layer(dimensions, || 0, |HexU8(b)| b, level_data.bts);
            CollisionScreenLayer {
                blocks: blocks
                    .map_err(|()| anyhow!("Out-of-bounds screens in layer ExtendedBlockType"))?,
            }
        };
        let layer2 = if let Some(layer2) = level_data.layer2 {
            let blocks = convert_block_layer(
                dimensions,
                || LevelBlock(0),
                |HexU16(b)| LevelBlock(b),
                layer2,
            );
            Some(BlockScreenLayer {
                blocks: blocks.map_err(|()| anyhow!("Out-of-bounds screens in Layer2"))?,
            })
        } else {
            None
        };

        let scrolls = {
            let desired_len = usize::from(width) * usize::from(height);

            let scrolls = if let Some(HexU16(const_val)) = scroll_data.const_ {
                let mut scrolls = vec![ScrollType::Green as u8; desired_len];
                let last_row = scrolls.len() - usize::from(width);
                scrolls[last_row..].fill((const_val + 1) as u8);

                scrolls
            } else {
                let mut scrolls: Vec<_> = scroll_data.data.into_iter().map(|HexU8(b)| b).collect();
                if scrolls.len() > desired_len {
                    return Err(anyhow!("Out-of-bounds scroll data"));
                } else if scrolls.len() < desired_len {
                    warn!(
                        "Truncated scroll data (of len {:#X}) will be expanded (to {:#X})",
                        scrolls.len(),
                        desired_len
                    );
                    scrolls.resize(desired_len, ScrollType::Red.into());
                }

                scrolls
            };

            ScrollsLayer {
                scrolls: LayerScreenGrid {
                    width,
                    height,
                    screens: scrolls,
                },
            }
        };

        states.push(RoomState {
            tileset_index,
            layer1,
            extended_block_type,
            layer2,
            scrolls,
        });
    }

    Ok(Room {
        handle,
        index: Some(index),
        name: room_name,
        width,
        height,
        states,
    })
}
