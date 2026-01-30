use crate::gfx::{Palette, Snes4BppTile};
use crate::smart_xml;
use anyhow::anyhow;
use bit_field::BitField;
use slotmap::SlotMap;
use std::collections::BTreeMap;
use std::path::Path;

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

#[derive(Copy, Clone)]
pub struct TiletableEntry(pub [TilemapEntry; 4]);

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

type TilesetIndex = u8;
pub struct Tileset {
    handle: TilesetRef,
    index: Option<TilesetIndex>,
    pub name: String,

    pub palette: Palette,
    pub gfx: Vec<Snes4BppTile>,
    pub tiletable: Vec<TiletableEntry>,
}
slotmap::new_key_type! { pub struct TilesetRef; }

impl Tileset {
    pub fn handle(&self) -> TilesetRef {
        self.handle
    }

    #[expect(unused)]
    pub fn index(&self) -> Option<TilesetIndex> {
        self.index
    }

    pub fn title(&self) -> String {
        if let Some(index) = self.index {
            format!("[{index:02X}] {}", self.name)
        } else {
            format!("[??] {}", self.name)
        }
    }
}

#[derive(Default)]
pub struct ProjectData {
    pub tilesets: SlotMap<TilesetRef, Tileset>,
    pub tileset_ids: BTreeMap<TilesetIndex, TilesetRef>,
}

pub fn validate_smart_project_path(project_path: &Path) -> Result<(), String> {
    if !project_path.is_dir() {
        return Err("Not a directory".into());
    }
    if !project_path.join("project.xml").exists() {
        return Err("Does not contain project.xml".into());
    }
    if !project_path.join("Export").exists() {
        return Err("Does not contain a Export/ directory".into());
    }

    Ok(())
}
pub fn load_smart_project(project_path: &Path) -> anyhow::Result<ProjectData> {
    let smart_tilesets = smart_xml::load_project_tilesets(project_path)?;

    let mut project = ProjectData::default();

    for (index, tileset) in smart_tilesets.sce {
        let name = tileset
            .metadata
            .map_or("Unnamed Tileset".into(), |meta| meta.name);

        let mut palette = Palette::from(tileset.palette);
        if let Err(()) = palette.truncate_checked(Palette::LINE_4BPP_LEN * 8) {
            return Err(anyhow!(
                "Tileset {index:02X} palette has too many (non-blank) lines"
            ));
        }

        let (tile_bytes, rest) = tileset.gfx.as_chunks();
        if !rest.is_empty() {
            return Err(anyhow!(
                "Tileset {index:02X} gfx not evenly divisible as tiles"
            ));
        }
        let gfx = tile_bytes.iter().map(Snes4BppTile::from_bytes).collect();

        let (tiletable_entries, rest) = tileset.tiletable.as_chunks::<4>();
        if !rest.is_empty() {
            return Err(anyhow!(
                "Tileset {index:02X} tiletable has truncated trailing entry"
            ));
        }
        let tiletable = tiletable_entries
            .iter()
            .map(|tiles| TiletableEntry(tiles.map(TilemapEntry)))
            .collect();

        // TODO encapsulate the combination of SlotMap + BTreeMap for index
        let tileset_ref = project.tilesets.insert_with_key(|handle| Tileset {
            handle,
            index: Some(index),
            name,
            palette,
            gfx,
            tiletable,
        });
        project.tileset_ids.insert(index, tileset_ref);
    }

    Ok(project)
}
