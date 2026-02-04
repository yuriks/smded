use crate::smart_xml;
use bit_field::BitField;
use heck::ToTitleCase;

#[derive(Copy, Clone)]
pub struct LevelDataEntry(pub u16);

impl LevelDataEntry {
    /// Tile index into the tiletable.
    pub fn block_id(self) -> u16 {
        self.0.get_bits(0..10)
    }

    pub fn h_flip(self) -> bool {
        self.0.get_bit(11)
    }

    pub fn v_flip(self) -> bool {
        self.0.get_bit(12)
    }

    #[expect(unused)]
    pub fn block_type(self) -> u16 {
        self.0.get_bits(12..)
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

slotmap::new_key_type! { pub struct RoomRef; }
pub type RoomIndex = (u8, u8);

pub struct Room {
    handle: RoomRef,
    index: Option<RoomIndex>,

    pub name: String,
}

impl Room {
    #[expect(unused)]
    pub fn handle(&self) -> RoomRef {
        self.handle
    }

    #[expect(unused)]
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

pub fn load_from_smart(
    index: RoomIndex,
    room_name: String,
    _room: smart_xml::Room,
    handle: RoomRef,
) -> anyhow::Result<Room> {
    Ok(Room {
        handle,
        index: Some(index),
        name: room_name,
    })
}
