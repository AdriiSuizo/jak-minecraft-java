//! Chunk persistence for one dimension: vanilla 26.3 region files under
//! `<world>/dimensions/<namespace>/<path>/region`, chunks in the
//! `SerializableChunkData` layout (`minecraftoss_core::chunk_nbt`).
//!
//! Chunks of every status are saved, as vanilla saves proto chunks: a
//! chunk that was decorated keeps its decoration and is never decorated
//! again after a reload. FULL chunks' entities live in the entity storage,
//! `<dimension>/entities`, which the level writes as its entities leave
//! (`EntityStorage`); a FULL chunk saved before the level ever wrote its
//! entities seeds the storage with the ones generation made.

use minecraftoss_core::anvil::{region_of, region_path, RegionFile, StoredChunk};
use minecraftoss_core::chunk::ChunkStatus;
use minecraftoss_core::chunk_nbt::{read_chunk, write_chunk, write_entities};
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::{Chunk, ChunkPos, Registries};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct ChunkStorage {
    dir: PathBuf,
    entities: EntityRegions,
    registries: Arc<Registries>,
    min_y: i32,
    height: i32,
    regions: Mutex<Regions>,
    /// Held while region files are written, so two flushes never write the
    /// same file at once; loads and saves only take the region caches.
    writing: Mutex<()>,
}

/// Region files of the entity storage.
struct EntityRegions {
    dir: PathBuf,
    regions: Mutex<Regions>,
}

#[derive(Default)]
struct Regions {
    loaded: HashMap<(i32, i32), CachedRegion>,
    dirty: HashSet<(i32, i32)>,
    writing: HashSet<(i32, i32)>,
    clock: u64,
    bytes: usize,
}

struct CachedRegion {
    file: RegionFile,
    touched: u64,
    bytes: usize,
}

// A long drive must not retain every region ever visited. Dirty regions and
// snapshots still being written stay resident even when over this budget.
const CACHE_REGIONS: usize = 32;
const CACHE_BYTES: usize = 64 * 1024 * 1024;

impl Regions {
    fn trim(&mut self, max_regions: usize, max_bytes: usize) {
        while self.loaded.len() > max_regions || self.bytes > max_bytes {
            let oldest = self.loaded.iter()
                .filter(|(pos, _)| !self.dirty.contains(pos) && !self.writing.contains(pos))
                .min_by_key(|(_, region)| region.touched)
                .map(|(&pos, _)| pos);
            let Some(pos) = oldest else { break };
            self.bytes -= self.loaded.remove(&pos).expect("cached region").bytes;
        }
    }
}

impl ChunkStorage {
    /// Storage for a dimension (`minecraft:overworld`) of the world at `world`.
    pub fn new(world: &Path, dimension: &str, registries: Arc<Registries>, min_y: i32, height: i32) -> Self {
        let (namespace, path) = dimension.split_once(':').unwrap_or(("minecraft", dimension));
        let base = world.join("dimensions").join(namespace).join(path);
        Self {
            dir: base.join("region"),
            entities: EntityRegions { dir: base.join("entities"), regions: Mutex::default() },
            registries,
            min_y,
            height,
            regions: Mutex::default(),
            writing: Mutex::default(),
        }
    }

    fn with_region<T>(&self, region: (i32, i32), f: impl FnOnce(&mut RegionFile) -> T) -> Result<T, String> {
        with_region(&self.dir, &self.regions, region, false, f)
    }

    fn with_entity_region<T>(&self, region: (i32, i32), f: impl FnOnce(&mut RegionFile) -> T) -> Result<T, String> {
        with_region(&self.entities.dir, &self.entities.regions, region, false, f)
    }

    /// A saved chunk, if there is one. Only the lookup holds the region
    /// cache; the chunk is decompressed and read outside it.
    pub fn load(&self, pos: ChunkPos) -> Result<Option<Chunk>, String> {
        let (region, index) = region_of(pos);
        let Some(stored) = self.with_region(region, |file| file.stored(index))? else { return Ok(None) };
        let mut chunk = read_chunk(&stored.parse()?, &self.registries, self.min_y, self.height)?;
        if chunk.status == ChunkStatus::Full {
            let entities = self.with_entity_region(region, |file| file.stored(index))?.map(|stored| stored.parse()).transpose()?;
            chunk.generation.entities = entities.and_then(|t| t.get("Entities").and_then(Tag::as_list).map(<[Tag]>::to_vec)).unwrap_or_default();
        }
        Ok(Some(chunk))
    }

    /// Queues a chunk for the next `flush`, compressed before the region
    /// cache is taken.
    pub fn save(&self, chunk: &Chunk) -> Result<(), String> {
        let started = std::time::Instant::now();
        let tag = write_chunk(chunk, &self.registries, 0);
        crate::chunk_map::stage_times::add(crate::chunk_map::stage_times::SAVE_NBT, started);
        let started = std::time::Instant::now();
        let stored = StoredChunk::encode(&tag);
        crate::chunk_map::stage_times::add(crate::chunk_map::stage_times::SAVE_COMPRESS, started);
        let (region, index) = region_of(chunk.pos);
        store(&self.dir, &self.regions, region, index, stored)?;
        // The level owns a FULL chunk's entities once it has saved them.
        if chunk.status == ChunkStatus::Full && !self.with_entity_region(region, |file| file.contains(index))? {
            // The check and insertion share the lock: generation must never
            // overwrite newer entities saved concurrently by the live level.
            let stored = StoredChunk::encode(&write_entities(chunk.pos, &chunk.generation.entities));
            with_region(&self.entities.dir, &self.entities.regions, region, true, |file| {
                if !file.contains(index) { file.set_stored(index, stored, timestamp()); }
            })?;
        }
        Ok(())
    }

    /// A chunk's saved entities (`EntityStorage.loadEntities`): `None` when
    /// the storage has no entry for it yet.
    pub fn load_entities(&self, pos: ChunkPos) -> Result<Option<Vec<Tag>>, String> {
        let (region, index) = region_of(pos);
        let tag = self.with_entity_region(region, |file| file.stored(index))?.map(|stored| stored.parse()).transpose()?;
        Ok(tag.map(|t| t.get("Entities").and_then(Tag::as_list).map(<[Tag]>::to_vec).unwrap_or_default()))
    }

    /// Queues a chunk's entities for the next `flush` (`EntityStorage.storeEntities`).
    pub fn save_entities(&self, pos: ChunkPos, entities: &[Tag]) -> Result<(), String> {
        let (region, index) = region_of(pos);
        let stored = StoredChunk::encode(&write_entities(pos, entities));
        store(&self.entities.dir, &self.entities.regions, region, index, stored)
    }

    /// Writes every region with unsaved chunks: each is copied (its chunks
    /// are shared) under the region cache and written outside it.
    pub fn flush(&self) -> Result<(), String> {
        let _writing = self.writing.lock().expect("region writer");
        let mut error = None;
        for (dir, regions) in [(&self.dir, &self.regions), (&self.entities.dir, &self.entities.regions)] {
            if let Err(e) = flush_regions(dir, regions, |file, path| file.write(path)) {
                error.get_or_insert(e);
            }
        }
        error.map_or(Ok(()), Err)
    }
}

fn timestamp() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as u32)
}

fn flush_regions(
    dir: &Path,
    cache: &Mutex<Regions>,
    mut write: impl FnMut(&RegionFile, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let snapshots: Vec<_> = {
        let mut regions = cache.lock().expect("region cache");
        let dirty: Vec<_> = regions.dirty.drain().collect();
        regions.writing.extend(dirty.iter().copied());
        dirty.into_iter().map(|pos| (pos, regions.loaded[&pos].file.clone())).collect()
    };
    let mut error = None;
    for (pos, file) in snapshots {
        let result = write(&file, &region_path(dir, pos));
        let mut regions = cache.lock().expect("region cache");
        regions.writing.remove(&pos);
        if let Err(e) = result {
            // Retain failed writes for the next flush. Do not skip the rest
            // of the batch or clear edits made while this snapshot was out.
            regions.dirty.insert(pos);
            error.get_or_insert(e);
        }
        regions.trim(CACHE_REGIONS, CACHE_BYTES);
    }
    error.map_or(Ok(()), Err)
}

/// Sets a stored chunk and marks its region for the next flush.
fn store(dir: &Path, regions: &Mutex<Regions>, region: (i32, i32), index: usize, stored: StoredChunk) -> Result<(), String> {
    with_region(dir, regions, region, true, |file| file.set_stored(index, stored, timestamp()))
}

fn with_region<T>(dir: &Path, regions: &Mutex<Regions>, region: (i32, i32), dirty: bool, f: impl FnOnce(&mut RegionFile) -> T) -> Result<T, String> {
    let mut regions = regions.lock().expect("region cache");
    if !regions.loaded.contains_key(&region) {
        let path = region_path(dir, region);
        let file = if path.exists() { RegionFile::read(&path)? } else { RegionFile::new() };
        let bytes = file.cached_bytes();
        regions.bytes += bytes;
        regions.loaded.insert(region, CachedRegion { file, bytes, touched: 0 });
    }
    regions.clock += 1;
    let clock = regions.clock;
    let cached = regions.loaded.get_mut(&region).expect("inserted");
    cached.touched = clock;
    let result = f(&mut cached.file);
    if dirty {
        let old_bytes = cached.bytes;
        cached.bytes = cached.file.cached_bytes();
        let new_bytes = cached.bytes;
        regions.bytes = regions.bytes - old_bytes + new_bytes;
        regions.dirty.insert(region);
    }
    regions.trim(CACHE_REGIONS, CACHE_BYTES);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let dir = std::env::temp_dir().join(format!("iw4l-storage40-{}-{}-{}", std::process::id(),
                timestamp(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }
    fn put(dir: &Path, cache: &Mutex<Regions>, region: (i32, i32), value: i32) {
        store(dir, cache, region, 0, StoredChunk::encode(&Tag::Int(value))).unwrap();
    }
    fn read(dir: &Path, cache: &Mutex<Regions>, region: (i32, i32)) -> Tag {
        with_region(dir, cache, region, false, |file| file.stored(0)).unwrap().unwrap().parse().unwrap()
    }

    #[test]
    fn long_exploration_releases_old_regions_and_revisits_preserve_edits() {
        let dir = Temp::new();
        let cache = Mutex::new(Regions::default());
        for x in 0..96 {
            put(&dir.0, &cache, (x, -2), x);
            flush_regions(&dir.0, &cache, |file, path| file.write(path)).unwrap();
            let regions = cache.lock().unwrap();
            assert!(regions.loaded.len() <= CACHE_REGIONS);
            assert!(regions.bytes <= CACHE_BYTES);
        }
        assert!(!cache.lock().unwrap().loaded.contains_key(&(0, -2)));
        // Travel back across every evicted region; disk remains authoritative.
        for x in (0..96).rev() { assert_eq!(read(&dir.0, &cache, (x, -2)), Tag::Int(x)); }
        put(&dir.0, &cache, (0, -2), 1000);
        flush_regions(&dir.0, &cache, |file, path| file.write(path)).unwrap();
        let reopened = Mutex::new(Regions::default());
        assert_eq!(read(&dir.0, &reopened, (0, -2)), Tag::Int(1000));
        assert_eq!(read(&dir.0, &reopened, (95, -2)), Tag::Int(95));
    }

    #[test]
    fn failed_region_writes_are_retried_without_losing_the_rest_of_the_batch() {
        let dir = Temp::new();
        let cache = Mutex::new(Regions::default());
        put(&dir.0, &cache, (0, 0), 17);
        put(&dir.0, &cache, (1, 0), 29);
        let blocked = region_path(&dir.0, (0, 0));
        std::fs::create_dir(&blocked).unwrap(); // Rename to this directory must fail.
        assert!(flush_regions(&dir.0, &cache, |file, path| file.write(path)).is_err());
        {
            let mut regions = cache.lock().unwrap();
            assert_eq!(regions.dirty, [(0, 0)].into_iter().collect());
            regions.trim(0, 0);
            assert!(regions.loaded.contains_key(&(0, 0)), "unsaved changes cannot be evicted");
        }
        assert_eq!(RegionFile::read(&region_path(&dir.0, (1, 0))).unwrap().get(0).unwrap(), Some(Tag::Int(29)));
        std::fs::remove_dir(blocked).unwrap();
        flush_regions(&dir.0, &cache, |file, path| file.write(path)).unwrap();
        let reopened = Mutex::new(Regions::default());
        assert_eq!(read(&dir.0, &reopened, (0, 0)), Tag::Int(17));
        assert!(cache.lock().unwrap().dirty.is_empty());
    }

    #[test]
    fn edits_during_a_flush_survive_its_completion_and_cache_pressure() {
        let dir = Temp::new();
        let cache = Mutex::new(Regions::default());
        put(&dir.0, &cache, (0, 0), 1);
        flush_regions(&dir.0, &cache, |file, path| {
            {
                let mut regions = cache.lock().unwrap();
                regions.trim(0, 0);
                assert!(regions.loaded.contains_key(&(0, 0)), "in-flight snapshot must remain pinned");
            }
            put(&dir.0, &cache, (0, 0), 2);
            file.write(path)
        }).unwrap();
        assert_eq!(RegionFile::read(&region_path(&dir.0, (0, 0))).unwrap().get(0).unwrap(), Some(Tag::Int(1)));
        assert_eq!(read(&dir.0, &cache, (0, 0)), Tag::Int(2));
        assert!(cache.lock().unwrap().dirty.contains(&(0, 0)));
        flush_regions(&dir.0, &cache, |file, path| file.write(path)).unwrap();
        assert_eq!(RegionFile::read(&region_path(&dir.0, (0, 0))).unwrap().get(0).unwrap(), Some(Tag::Int(2)));
    }

    #[test]
    fn byte_budget_evicts_least_recent_clean_region() {
        let dir = Temp::new();
        let cache = Mutex::new(Regions::default());
        for x in 0..3 { put(&dir.0, &cache, (x, 0), x); }
        flush_regions(&dir.0, &cache, |file, path| file.write(path)).unwrap();
        read(&dir.0, &cache, (0, 0)); // Region 0 is most recently used now.
        let mut regions = cache.lock().unwrap();
        let one_region = regions.loaded[&(0, 0)].bytes;
        regions.trim(CACHE_REGIONS, one_region);
        assert_eq!(regions.loaded.len(), 1);
        assert!(regions.loaded.contains_key(&(0, 0)));
        assert_eq!(regions.bytes, one_region);
    }
}
