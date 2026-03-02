use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::game::GameSnapshotV1;

const APP_DIR_NAME: &str = "terminal-snake";
const SESSION_FILE_NAME: &str = "sessions.json";
const LEGACY_SESSION_FILE_NAME: &str = "session.json";
const STORAGE_VERSION: u32 = 1;
const SNAPSHOT_VERSION: u32 = 1;
const RESUME_HASH_LEN: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFile {
    pub snapshot_version: u32,
    pub resume_hash: String,
    pub saved_at_unix_ms: u64,
    pub snapshot: GameSnapshotV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionStore {
    version: u32,
    sessions: Vec<SessionFile>,
}

#[must_use]
pub fn session_path() -> PathBuf {
    let mut base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.push(APP_DIR_NAME);
    base.push(SESSION_FILE_NAME);
    base
}

fn legacy_session_path() -> PathBuf {
    let mut base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.push(APP_DIR_NAME);
    base.push(LEGACY_SESSION_FILE_NAME);
    base
}

pub fn load_active_sessions() -> io::Result<Vec<SessionFile>> {
    load_store().map(|store| store.sessions)
}

pub fn load_latest_session() -> io::Result<Option<SessionFile>> {
    let sessions = load_active_sessions()?;
    Ok(sessions.into_iter().next())
}

pub fn find_session_by_hash(candidate: &str) -> io::Result<Option<SessionFile>> {
    let sessions = load_active_sessions()?;
    Ok(sessions
        .into_iter()
        .find(|session| matches_resume_hash(session, candidate)))
}

pub fn save_active_session(snapshot: &GameSnapshotV1) -> io::Result<SessionFile> {
    let mut sessions = load_active_sessions()?;
    let saved = SessionFile {
        snapshot_version: SNAPSHOT_VERSION,
        resume_hash: compute_resume_hash(snapshot)?,
        saved_at_unix_ms: now_unix_ms(),
        snapshot: snapshot.clone(),
    };

    sessions.retain(|existing| !matches_resume_hash(existing, &saved.resume_hash));
    sessions.insert(0, saved.clone());
    write_store(
        &session_path(),
        &SessionStore {
            version: STORAGE_VERSION,
            sessions,
        },
    )?;

    Ok(saved)
}

pub fn remove_session_by_hash(candidate: &str) -> io::Result<bool> {
    let mut sessions = load_active_sessions()?;
    let before = sessions.len();
    sessions.retain(|session| !matches_resume_hash(session, candidate));
    let removed = sessions.len() != before;

    if removed {
        write_store(
            &session_path(),
            &SessionStore {
                version: STORAGE_VERSION,
                sessions,
            },
        )?;
    }

    Ok(removed)
}

pub fn compute_resume_hash(snapshot: &GameSnapshotV1) -> io::Result<String> {
    let canonical = serde_json::to_vec(snapshot)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    let digest = Sha256::digest(canonical);
    let mut full_hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        full_hex.push_str(&format!("{byte:02x}"));
    }

    Ok(full_hex.chars().take(RESUME_HASH_LEN).collect())
}

#[must_use]
pub fn matches_resume_hash(session: &SessionFile, candidate: &str) -> bool {
    session.resume_hash.eq_ignore_ascii_case(candidate.trim())
}

fn load_store() -> io::Result<SessionStore> {
    let path = session_path();
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return load_legacy_store();
        }
        Err(error) => return Err(error),
    };

    let mut store = serde_json::from_str::<SessionStore>(&raw)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if store.version != STORAGE_VERSION {
        return Ok(SessionStore {
            version: STORAGE_VERSION,
            sessions: Vec::new(),
        });
    }

    store
        .sessions
        .retain(|entry| entry.snapshot_version == SNAPSHOT_VERSION);
    store
        .sessions
        .sort_by(|a, b| b.saved_at_unix_ms.cmp(&a.saved_at_unix_ms));
    Ok(store)
}

fn load_legacy_store() -> io::Result<SessionStore> {
    let legacy_path = legacy_session_path();
    let raw = match fs::read_to_string(&legacy_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(SessionStore {
                version: STORAGE_VERSION,
                sessions: Vec::new(),
            });
        }
        Err(error) => return Err(error),
    };

    let legacy = serde_json::from_str::<SessionFile>(&raw)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    let store = if legacy.snapshot_version == SNAPSHOT_VERSION {
        SessionStore {
            version: STORAGE_VERSION,
            sessions: vec![legacy],
        }
    } else {
        SessionStore {
            version: STORAGE_VERSION,
            sessions: Vec::new(),
        }
    };

    write_store(&session_path(), &store)?;
    let _ = fs::remove_file(legacy_path);
    Ok(store)
}

fn write_store(path: &Path, store: &SessionStore) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let payload = serde_json::to_string_pretty(store)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(path, payload)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::{compute_resume_hash, matches_resume_hash};
    use crate::config::GridSize;
    use crate::game::GameState;

    #[test]
    fn resume_hash_is_stable_for_same_snapshot() {
        let state = GameState::new_with_seed(
            GridSize {
                width: 20,
                height: 20,
            },
            42,
        );
        let snapshot = state.to_snapshot_v1();

        let a = compute_resume_hash(&snapshot).expect("hash should compute");
        let b = compute_resume_hash(&snapshot).expect("hash should compute");

        assert_eq!(a, b);
    }

    #[test]
    fn resume_hash_compare_is_case_insensitive() {
        let file = crate::session::SessionFile {
            snapshot_version: 1,
            resume_hash: "abcdef123456".to_owned(),
            saved_at_unix_ms: 0,
            snapshot: GameState::new_with_seed(
                GridSize {
                    width: 8,
                    height: 8,
                },
                1,
            )
            .to_snapshot_v1(),
        };

        assert!(matches_resume_hash(&file, "ABCDEF123456"));
    }
}
