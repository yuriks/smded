use crate::project::TilesetRef;
use egui::cache::CacheTrait;
use egui::{Context, TextureHandle};
use std::any::Any;
use std::collections::HashMap;

mod measurer;
pub mod promise;
pub mod views;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum TileCacheKey {
    // TODO: Figure out how to cleanly handle invalidation
    TilesetGfx {
        tileset: TilesetRef,
        palette_line: u8,
    },
    TilesetTtb {
        tileset: TilesetRef,
    },
}

#[derive(Default)]
pub struct TileTextureCache {
    /// Incremented every eviction pass
    update_counter: u32,
    /// Tuple contains the value of `update_counter` on last use.
    entries: HashMap<TileCacheKey, (u32, TextureHandle)>,
}

impl TileTextureCache {
    pub fn for_context<T>(ctx: &Context, operation: impl FnOnce(&mut Self) -> T) -> T {
        ctx.memory_mut(|mem| {
            let cache = mem.caches.cache::<Self>();
            operation(cache)
        })
    }

    pub fn get(&mut self, key: &TileCacheKey) -> Option<&TextureHandle> {
        let (last_use, value) = self.entries.get_mut(key)?;
        *last_use = self.update_counter;
        Some(value)
    }

    pub fn insert(&mut self, key: TileCacheKey, value: TextureHandle) -> &TextureHandle {
        let (_, value) = self
            .entries
            .entry(key)
            .and_modify(|(last_use, _)| *last_use = self.update_counter)
            .or_insert((self.update_counter, value));
        value
    }

    #[expect(unused)]
    pub fn invalidate(&mut self, key: &TileCacheKey) {
        self.entries.remove(key);
    }

    pub fn get_or_insert_with(
        ctx: &Context,
        key: TileCacheKey,
        f: impl FnOnce(&Context) -> TextureHandle,
    ) -> TextureHandle {
        if let Some(cached) = Self::for_context(ctx, |cache| cache.get(&key).cloned()) {
            return cached;
        }
        let to_insert = f(ctx);
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
