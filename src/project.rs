use crate::smart_xml;
use anyhow::anyhow;
use egui::Color32;
use slotmap::SlotMap;
use std::collections::BTreeMap;
use std::path::Path;
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
    const LINE_4BPP_LEN: usize = 16;
    const LINE_2BPP_LEN: usize = 4;

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

type TilesetIndex = u8;
pub struct Tileset {
    index: Option<TilesetIndex>,
    pub name: String,

    pub palette: Palette,
}
slotmap::new_key_type! { pub struct TilesetRef; }

impl Tileset {
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
        let tileset = Tileset {
            index: Some(index),
            name,
            palette,
        };
        // TODO encapsulate the combination of SlotMap + BTreeMap for index
        let tileset_ref = project.tilesets.insert(tileset);
        project.tileset_ids.insert(index, tileset_ref);
    }

    Ok(project)
}
