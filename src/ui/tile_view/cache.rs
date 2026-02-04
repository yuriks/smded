use crate::tileset::{LoadedTilesetLayout, OverlaidLayout, TilesetRef};
use egui::cache::CacheTrait;
use egui::{Context, TextureHandle};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum TileCacheKey {
    // TODO: Figure out how to cleanly handle invalidation
    LoadedGfxLayout {
        gfx_layout: OverlaidLayout<TilesetRef>,
        palette_source: TilesetRef,
        palette_line: u8,
    },
    LoadedTilesetLayout {
        layout: LoadedTilesetLayout<TilesetRef>,
    },
}

impl TileCacheKey {
    /// Returns a descriptive non-unique string to use as a debugging name for the texture
    pub fn texture_name(&self) -> String {
        fn layout_cache_texture_name(tileset: &OverlaidLayout<TilesetRef>) -> String {
            let mut s = String::from("layout");
            for e in &tileset.entries {
                write!(&mut s, "-0x{:X}[{:?}]", e.base, e.tileset).unwrap();
            }

            s
        }

        match self {
            TileCacheKey::LoadedGfxLayout {
                gfx_layout,
                palette_source,
                palette_line,
            } => {
                let mut s = layout_cache_texture_name(gfx_layout);
                write!(s, "-pal{palette_line:X}[{palette_source:?}]").unwrap();
                s
            }
            TileCacheKey::LoadedTilesetLayout { layout } => {
                layout_cache_texture_name(&layout.tiletable) + "-ttb"
            }
        }
    }
}

#[derive(Default)]
pub struct TileTextureCache {
    /// Incremented every eviction pass
    update_counter: u32,
    /// Tuple contains the value of `update_counter` on last use.
    entries: HashMap<TileCacheKey, (u32, TextureHandle)>,
}

impl TileTextureCache {
    fn for_context<T>(ctx: &Context, operation: impl FnOnce(&mut Self) -> T) -> T {
        ctx.memory_mut(|mem| {
            let cache = mem.caches.cache::<Self>();
            operation(cache)
        })
    }

    fn get(&mut self, key: &TileCacheKey) -> Option<&TextureHandle> {
        let (last_use, value) = self.entries.get_mut(key)?;
        *last_use = self.update_counter;
        Some(value)
    }

    fn insert(&mut self, key: TileCacheKey, value: TextureHandle) -> &TextureHandle {
        let (_, value) = self
            .entries
            .entry(key)
            .and_modify(|(last_use, _)| *last_use = self.update_counter)
            .or_insert((self.update_counter, value));
        value
    }

    #[expect(unused)]
    fn invalidate(&mut self, key: &TileCacheKey) {
        self.entries.remove(key);
    }

    pub fn get_or_insert_with(
        ctx: &Context,
        key: TileCacheKey,
        f: impl FnOnce(&Context, &TileCacheKey) -> TextureHandle,
    ) -> TextureHandle {
        if let Some(cached) = Self::for_context(ctx, |cache| cache.get(&key).cloned()) {
            return cached;
        }
        let to_insert = f(ctx, &key);
        Self::for_context(ctx, |cache| cache.insert(key, to_insert).clone())
    }
}

impl CacheTrait for TileTextureCache {
    fn update(&mut self) {
        const MAX_AGE: u32 = 15;

        self.entries
            .retain(|_, (last_use, _)| last_use.wrapping_sub(self.update_counter) < MAX_AGE);
        self.update_counter = self.update_counter.wrapping_add(1);
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
