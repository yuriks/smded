use crate::smart_xml;
use slotmap::SlotMap;
use std::collections::BTreeMap;
use std::path::Path;

type TilesetIndex = u8;
pub struct Tileset {
    index: Option<TilesetIndex>,
    pub name: String,
}
slotmap::new_key_type! { pub struct TilesetRef; }

impl Tileset {
    pub fn index(&self) -> Option<TilesetIndex> {
        self.index
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
        let tileset = Tileset { index: Some(index), name };
        // TODO encapsulate the combination of SlotMap + BTreeMap for index
        let tileset_ref = project.tilesets.insert(tileset);
        project.tileset_ids.insert(index, tileset_ref);
    }

    Ok(project)
}
