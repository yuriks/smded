use crate::tileset::{Tileset, TilesetIndex, TilesetKind, TilesetRef};
use crate::{smart_xml, tileset};
use bit_field::BitField;
use slotmap::SlotMap;
use std::collections::BTreeMap;
use std::path::Path;

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

#[derive(Default)]
pub struct ProjectData {
    pub tilesets: SlotMap<TilesetRef, Tileset>,
    pub tileset_ids: BTreeMap<TilesetIndex, TilesetRef>,
    pub cre_tileset_ids: BTreeMap<TilesetIndex, TilesetRef>,
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
        // TODO encapsulate the combination of SlotMap + BTreeMap for index
        let tileset_ref = project.tilesets.try_insert_with_key(|handle| {
            tileset::load_from_smart(TilesetKind::Sce, index, tileset, handle)
        })?;
        project.tileset_ids.insert(index, tileset_ref);
    }

    for (index, tileset) in smart_tilesets.cre {
        // TODO encapsulate the combination of SlotMap + BTreeMap for index
        let tileset_ref = project.tilesets.try_insert_with_key(|handle| {
            tileset::load_from_smart(TilesetKind::Cre, index, tileset, handle)
        })?;
        project.cre_tileset_ids.insert(index, tileset_ref);
    }

    Ok(project)
}
