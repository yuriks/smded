use crate::gfx::{Palette, Snes4BppTile};
use crate::smart_xml;
use anyhow::anyhow;
use slotmap::SlotMap;
use std::collections::BTreeMap;
use std::path::Path;

type TilesetIndex = u8;
pub struct Tileset {
    handle: TilesetRef,
    index: Option<TilesetIndex>,
    pub name: String,

    pub palette: Palette,
    pub gfx: Vec<Snes4BppTile>,
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

        // TODO encapsulate the combination of SlotMap + BTreeMap for index
        let tileset_ref = project.tilesets.insert_with_key(|handle| Tileset {
            handle,
            index: Some(index),
            name,
            palette,
            gfx,
        });
        project.tileset_ids.insert(index, tileset_ref);
    }

    Ok(project)
}
