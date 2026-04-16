use std::fs;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

use fd_lock::RwLock;
use jiff::Timestamp;
use serde::Deserialize;
use serde::Serialize;

use crate::error::HemliError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub namespace: String,
    pub secret: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecretIndex {
    pub entries: Vec<IndexEntry>,
}

pub fn index_path() -> PathBuf {
    let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    data_dir.join("hemli").join("index.json")
}

pub fn load_index(path: &Path) -> Result<SecretIndex, HemliError> {
    if !path.exists() {
        return Ok(SecretIndex::default());
    }
    let contents = fs::read_to_string(path)?;
    let index: SecretIndex = serde_json::from_str(&contents)?;
    Ok(index)
}

pub fn save_index(path: &Path, index: &SecretIndex) -> Result<(), HemliError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(index)?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, json)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

fn lock_file_path(index_path: &Path) -> PathBuf {
    index_path.with_extension("json.lock")
}

pub fn update_index(path: &Path, mutate: impl FnOnce(&mut SecretIndex)) -> Result<(), HemliError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let lock_path = lock_file_path(path);
    let file = File::create(&lock_path).map_err(HemliError::LockFailed)?;
    let mut lock = RwLock::new(file);
    let _guard = lock.write().map_err(HemliError::LockFailed)?;

    let mut index = load_index(path)?;
    mutate(&mut index);
    save_index(path, &index)?;

    Ok(())
}

pub fn upsert_entry(index: &mut SecretIndex, namespace: &str, secret: &str, created_at: Timestamp) {
    if let Some(entry) = index
        .entries
        .iter_mut()
        .find(|e| e.namespace == namespace && e.secret == secret)
    {
        entry.created_at = created_at;
    } else {
        index.entries.push(IndexEntry {
            namespace: namespace.to_string(),
            secret: secret.to_string(),
            created_at,
        });
    }
}

pub fn remove_entry(index: &mut SecretIndex, namespace: &str, secret: &str) {
    index
        .entries
        .retain(|e| !(e.namespace == namespace && e.secret == secret));
}

pub fn filter_entries<'a>(index: &'a SecretIndex, namespace: Option<&str>) -> Vec<&'a IndexEntry> {
    match namespace {
        Some(ns) => index.entries.iter().filter(|e| e.namespace == ns).collect(),
        None => index.entries.iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_nonexistent_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let index = load_index(&path).unwrap();
        assert!(index.entries.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");
        let mut index = SecretIndex::default();
        upsert_entry(&mut index, "ns1", "sec1", Timestamp::now());
        save_index(&path, &index).unwrap();

        let loaded = load_index(&path).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].namespace, "ns1");
        assert_eq!(loaded.entries[0].secret, "sec1");
    }

    #[test]
    fn upsert_replaces_existing() {
        let mut index = SecretIndex::default();
        let t1 = Timestamp::from_second(1000).unwrap();
        let t2 = Timestamp::from_second(2000).unwrap();
        upsert_entry(&mut index, "ns", "sec", t1);
        upsert_entry(&mut index, "ns", "sec", t2);
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].created_at, t2);
    }

    #[test]
    fn upsert_adds_different_entries() {
        let mut index = SecretIndex::default();
        let t = Timestamp::now();
        upsert_entry(&mut index, "ns1", "sec1", t);
        upsert_entry(&mut index, "ns2", "sec2", t);
        assert_eq!(index.entries.len(), 2);
    }

    #[test]
    fn remove_entry_works() {
        let mut index = SecretIndex::default();
        let t = Timestamp::now();
        upsert_entry(&mut index, "ns", "sec1", t);
        upsert_entry(&mut index, "ns", "sec2", t);
        remove_entry(&mut index, "ns", "sec1");
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].secret, "sec2");
    }

    #[test]
    fn remove_nonexistent_is_noop() {
        let mut index = SecretIndex::default();
        remove_entry(&mut index, "ns", "sec");
        assert!(index.entries.is_empty());
    }

    #[test]
    fn filter_by_namespace() {
        let mut index = SecretIndex::default();
        let t = Timestamp::now();
        upsert_entry(&mut index, "ns1", "sec1", t);
        upsert_entry(&mut index, "ns2", "sec2", t);
        upsert_entry(&mut index, "ns1", "sec3", t);

        let filtered = filter_entries(&index, Some("ns1"));
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.namespace == "ns1"));
    }

    #[test]
    fn filter_no_namespace_returns_all() {
        let mut index = SecretIndex::default();
        let t = Timestamp::now();
        upsert_entry(&mut index, "ns1", "sec1", t);
        upsert_entry(&mut index, "ns2", "sec2", t);

        let filtered = filter_entries(&index, None);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn parent_directory_creation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("dir").join("index.json");
        let index = SecretIndex::default();
        save_index(&path, &index).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn save_index_leaves_no_tmp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");
        let index = SecretIndex::default();
        save_index(&path, &index).unwrap();

        let tmp_path = path.with_extension("json.tmp");
        assert!(!tmp_path.exists());
        assert!(path.exists());
    }

    #[test]
    fn update_index_creates_and_modifies() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");

        update_index(&path, |idx| {
            upsert_entry(idx, "ns1", "sec1", Timestamp::now());
        })
        .unwrap();

        let loaded = load_index(&path).unwrap();
        assert_eq!(loaded.entries.len(), 1);

        update_index(&path, |idx| {
            upsert_entry(idx, "ns2", "sec2", Timestamp::now());
        })
        .unwrap();

        let loaded = load_index(&path).unwrap();
        assert_eq!(loaded.entries.len(), 2);
    }
}
