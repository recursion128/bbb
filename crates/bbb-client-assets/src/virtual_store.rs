//! Thread-safe in-memory resource file backing for Web/WASM asset bundles and
//! server resource packs, allowing path-based asset consumers to operate
//! without temporary disk extraction.

#[cfg(not(target_arch = "wasm32"))]
use std::sync::RwLock;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Cursor, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

#[cfg(target_arch = "wasm32")]
use bbb_render_types::wasm_sync::{SpinReadGuard, SpinRwLock, SpinWriteGuard};

pub const VIRTUAL_PATH_PREFIX: &str = "/__bbb_mem__";

#[cfg(not(target_arch = "wasm32"))]
type StoreLock<T> = RwLock<T>;
#[cfg(target_arch = "wasm32")]
type StoreLock<T> = SpinRwLock<T>;

fn store() -> &'static StoreLock<HashMap<PathBuf, Arc<[u8]>>> {
    static STORE: OnceLock<StoreLock<HashMap<PathBuf, Arc<[u8]>>>> = OnceLock::new();
    STORE.get_or_init(|| StoreLock::new(HashMap::new()))
}

#[cfg(not(target_arch = "wasm32"))]
fn read_store() -> std::sync::RwLockReadGuard<'static, HashMap<PathBuf, Arc<[u8]>>> {
    store()
        .read()
        .expect("virtual resource store read lock poisoned")
}

#[cfg(target_arch = "wasm32")]
fn read_store() -> SpinReadGuard<'static, HashMap<PathBuf, Arc<[u8]>>> {
    store().read()
}

#[cfg(not(target_arch = "wasm32"))]
fn write_store() -> std::sync::RwLockWriteGuard<'static, HashMap<PathBuf, Arc<[u8]>>> {
    store()
        .write()
        .expect("virtual resource store write lock poisoned")
}

#[cfg(target_arch = "wasm32")]
fn write_store() -> SpinWriteGuard<'static, HashMap<PathBuf, Arc<[u8]>>> {
    store().write()
}

pub(crate) fn normalize_virtual_path(path: &Path) -> Option<PathBuf> {
    let text = path.to_str()?;
    if text == VIRTUAL_PATH_PREFIX {
        return Some(PathBuf::from(VIRTUAL_PATH_PREFIX));
    }
    let rest = text
        .strip_prefix("/__bbb_mem__/")
        .or_else(|| text.strip_prefix("/__bbb_mem__\\"))?;
    if rest.contains('\\') {
        Some(PathBuf::from(format!(
            "/__bbb_mem__/{}",
            rest.replace('\\', "/")
        )))
    } else {
        Some(path.to_path_buf())
    }
}

pub fn is_virtual_path(path: &Path) -> bool {
    normalize_virtual_path(path).is_some()
}

pub fn insert_virtual_file(path: impl Into<PathBuf>, bytes: Arc<[u8]>) {
    let raw = path.into();
    let key = normalize_virtual_path(&raw).unwrap_or(raw);
    let mut guard = write_store();
    guard.insert(key, bytes);
}

pub fn get_virtual_file(path: &Path) -> Option<Arc<[u8]>> {
    let key = normalize_virtual_path(path)?;
    let guard = read_store();
    guard.get(&key).cloned()
}

pub fn has_virtual_file(path: &Path) -> bool {
    let Some(key) = normalize_virtual_path(path) else {
        return false;
    };
    let guard = read_store();
    guard.contains_key(&key)
}

pub fn remove_virtual_prefix(prefix: &Path) {
    let norm = normalize_virtual_path(prefix).unwrap_or_else(|| prefix.to_path_buf());
    let mut guard = write_store();
    guard.retain(|key, _| !key.starts_with(&norm));
    drop(guard);
    crate::zip_pack::remove_lazy_virtual_prefix(&norm);
}

#[derive(Debug)]
pub enum ResourceReader {
    File(File),
    Memory(Cursor<Arc<[u8]>>),
}

impl Read for ResourceReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::File(file) => file.read(buf),
            Self::Memory(cursor) => cursor.read(buf),
        }
    }
}

impl Seek for ResourceReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match self {
            Self::File(file) => file.seek(pos),
            Self::Memory(cursor) => cursor.seek(pos),
        }
    }
}

pub fn open_resource_file(path: &Path) -> io::Result<(ResourceReader, u64)> {
    if is_virtual_path(path) {
        let bytes = get_virtual_file(path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("virtual resource not found: {}", path.display()),
            )
        })?;
        let len = bytes.len() as u64;
        return Ok((ResourceReader::Memory(Cursor::new(bytes)), len));
    }
    let file = File::open(path)?;
    let len = file.metadata()?.len();
    Ok((ResourceReader::File(file), len))
}

pub fn resource_file_len(path: &Path) -> io::Result<u64> {
    if is_virtual_path(path) {
        let bytes = get_virtual_file(path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("virtual resource not found: {}", path.display()),
            )
        })?;
        return Ok(bytes.len() as u64);
    }
    Ok(std::fs::metadata(path)?.len())
}
