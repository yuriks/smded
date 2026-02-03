use crate::project::Tileset;

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
