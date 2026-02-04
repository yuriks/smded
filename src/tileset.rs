use crate::gfx::{Palette, Snes4BppTile, TilemapEntry};
use crate::smart_xml;
use anyhow::anyhow;
use std::cmp::Ordering;

#[derive(Copy, Clone)]
pub struct TiletableEntry(pub [TilemapEntry; 4]);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum TilesetKind {
    /// Main, area-specific tileset data
    Sce,
    /// Common room elements. Shared set of tiles that are loaded together with an SCE.
    Cre,
}

slotmap::new_key_type! { pub struct TilesetRef; }
pub type TilesetIndex = u8;

pub struct Tileset {
    handle: TilesetRef,
    index: Option<TilesetIndex>,
    pub name: String,
    pub kind: TilesetKind,

    pub palette: Palette,
    pub gfx: Vec<Snes4BppTile>,
    pub tiletable: Vec<TiletableEntry>,
}

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

    pub fn display_cmp(&self, o: &Self) -> Ordering {
        self.kind
            .cmp(&o.kind)
            .then(self.index.cmp(&o.index))
            .then_with(|| self.name.cmp(&o.name))
            .then(self.handle.cmp(&o.handle))
    }
}

pub fn load_from_smart(
    kind: TilesetKind,
    index: u8,
    tileset: smart_xml::Tileset,
    handle: TilesetRef,
) -> anyhow::Result<Tileset> {
    let name = tileset
        .metadata
        .map_or("Unnamed Tileset".into(), |meta| meta.name);

    let mut palette = Palette::from(tileset.palette);
    if !palette.is_empty()
        && let Err(()) =
            palette.truncate_checked(Palette::LINE_4BPP_LEN * TilemapEntry::ADDRESSABLE_PALETTES)
    {
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

    Ok(Tileset {
        handle,
        index: Some(index),
        kind,
        name,
        palette,
        gfx,
        tiletable,
    })
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct OverlaidLayoutEntry<Ref> {
    pub base: usize,
    pub size: usize,
    pub tileset: Ref,
}

/// Stack of tileset objects that can be edited together. e.g. SCE+CRE
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct OverlaidLayout<Ref> {
    pub entries: Vec<OverlaidLayoutEntry<Ref>>,
}

impl<Ref> Default for OverlaidLayout<Ref> {
    fn default() -> Self {
        OverlaidLayout {
            entries: Vec::new(),
        }
    }
}

impl<Ref> OverlaidLayout<Ref>
where
    Ref: Copy,
{
    pub fn lookup(&self, i: usize) -> Option<(Ref, usize)> {
        self.entries
            .iter()
            .rfind(|e| i >= e.base && (i - e.base) <= e.size)
            .map(|e| (e.tileset, i - e.base))
    }

    pub fn valid_range(&self) -> Option<(usize, usize)> {
        self.entries
            .iter()
            .map(|e| (e.base, e.base + e.size))
            .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
    }

    pub fn map_ref<T>(&self, mut f: impl FnMut(Ref) -> T) -> OverlaidLayout<T> {
        OverlaidLayout {
            entries: self
                .entries
                .iter()
                .map(|e| OverlaidLayoutEntry {
                    base: e.base,
                    size: e.size,
                    tileset: f(e.tileset),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct LoadedTilesetLayout<Ref> {
    pub gfx: OverlaidLayout<Ref>,
    pub tiletable: OverlaidLayout<Ref>,
    // palette_source could be made an OverlaidLayout too, but it's not necessary right now.
    pub palette_source: Ref,
}

impl<Ref> LoadedTilesetLayout<Ref>
where
    Ref: Copy,
{
    pub fn map_refs<T>(&self, mut f: impl FnMut(Ref) -> T) -> LoadedTilesetLayout<T> {
        LoadedTilesetLayout {
            gfx: self.gfx.map_ref(&mut f),
            tiletable: self.tiletable.map_ref(&mut f),
            palette_source: f(self.palette_source),
        }
    }
}

pub fn detect_sources_layout<'p>(
    selected_sce: &'p Tileset,
    selected_cre: Option<&'p Tileset>,
) -> LoadedTilesetLayout<&'p Tileset> {
    // A tiletable with more than 0x300 entries would overflow the vanilla buffer, so it's a good
    // guess on if it's expecting the Ceres tileset loading code.
    let is_ceres_tileset = selected_sce.tiletable.len() > 0x300;

    let mut gfx_layout = OverlaidLayout::default();
    if let Some(selected_cre) = selected_cre {
        gfx_layout.entries.push(OverlaidLayoutEntry {
            base: 0x280,
            size: selected_cre.gfx.len(),
            tileset: selected_cre,
        });
    }
    gfx_layout.entries.push(OverlaidLayoutEntry {
        base: 0x0,
        size: selected_sce.gfx.len(),
        tileset: selected_sce,
    });

    let mut ttb_layout = OverlaidLayout::default();
    if let Some(selected_cre) = selected_cre
        && !is_ceres_tileset
    {
        ttb_layout.entries.push(OverlaidLayoutEntry {
            base: 0x0,
            size: selected_cre.tiletable.len(),
            tileset: selected_cre,
        });
    }
    ttb_layout.entries.push(OverlaidLayoutEntry {
        base: if is_ceres_tileset { 0x0 } else { 0x100 },
        size: selected_sce.tiletable.len(),
        tileset: selected_sce,
    });

    LoadedTilesetLayout {
        gfx: gfx_layout,
        tiletable: ttb_layout,
        palette_source: selected_sce,
    }
}
