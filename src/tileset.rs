use crate::gfx::Palette;
use crate::project::Tileset;

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct TilesetVramLayoutEntry<T> {
    pub base: usize,
    pub size: usize,
    pub tileset: T,
}

/// Stack of tileset objects that can be edited together. e.g. SCE+CRE
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct TilesetVramLayout<T>(pub Vec<TilesetVramLayoutEntry<T>>);

impl<T> Default for TilesetVramLayout<T> {
    fn default() -> Self {
        TilesetVramLayout(Vec::new())
    }
}

impl<T> TilesetVramLayout<T>
where
    T: Copy,
{
    pub fn lookup(&self, i: usize) -> Option<(T, usize)> {
        self.0
            .iter()
            .rfind(|e| i >= e.base && (i - e.base) <= e.size)
            .map(|e| (e.tileset, i - e.base))
    }

    pub fn valid_range(&self) -> Option<(usize, usize)> {
        self.0
            .iter()
            .map(|e| (e.base, e.base + e.size))
            .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
    }

    pub fn map_values<U>(&self, f: impl Fn(T) -> U) -> TilesetVramLayout<U> {
        TilesetVramLayout(
            self.0
                .iter()
                .map(|e| TilesetVramLayoutEntry {
                    base: e.base,
                    size: e.size,
                    tileset: f(e.tileset),
                })
                .collect(),
        )
    }
}

impl TilesetVramLayout<&Tileset> {
    pub fn find_palette(&self) -> Option<&Palette> {
        self.0
            .iter()
            .map(|e| &e.tileset.palette)
            .rfind(|p| !p.is_empty())
    }
}

pub fn detect_sources_layout<'p>(
    selected_sce: &'p Tileset,
    selected_cre: Option<&'p Tileset>,
) -> (
    TilesetVramLayout<&'p Tileset>,
    TilesetVramLayout<&'p Tileset>,
) {
    // A tiletable with more than 0x300 entries would overflow the vanilla buffer, so it's a good
    // guess on if it's expecting the Ceres tileset loading code.
    let is_ceres_tileset = selected_sce.tiletable.len() > 0x300;

    let mut gfx_layout = TilesetVramLayout::default();
    if let Some(selected_cre) = selected_cre {
        gfx_layout.0.push(TilesetVramLayoutEntry {
            base: 0x280,
            size: selected_cre.gfx.len(),
            tileset: selected_cre,
        });
    }
    gfx_layout.0.push(TilesetVramLayoutEntry {
        base: 0x0,
        size: selected_sce.gfx.len(),
        tileset: selected_sce,
    });

    let mut ttb_layout = TilesetVramLayout::default();
    if let Some(selected_cre) = selected_cre
        && !is_ceres_tileset
    {
        ttb_layout.0.push(TilesetVramLayoutEntry {
            base: 0x0,
            size: selected_cre.tiletable.len(),
            tileset: selected_cre,
        });
    }
    ttb_layout.0.push(TilesetVramLayoutEntry {
        base: if is_ceres_tileset { 0x0 } else { 0x100 },
        size: selected_sce.tiletable.len(),
        tileset: selected_sce,
    });

    (gfx_layout, ttb_layout)
}
