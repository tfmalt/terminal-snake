use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const LEGACY_APP_DIR_NAME: &str = "snake";
const APP_DIR_NAME: &str = "terminal-snake";
const SCORE_FILE_NAME: &str = "scores.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ScoreFile {
    high_score: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    theme_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    theme_name: Option<String>,
}

/// Returns the platform-correct score file path.
#[must_use]
pub fn scores_path() -> PathBuf {
    score_path_for_app(APP_DIR_NAME)
}

fn legacy_scores_path() -> PathBuf {
    score_path_for_app(LEGACY_APP_DIR_NAME)
}

fn score_path_for_app(app_dir_name: &str) -> PathBuf {
    let mut base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.push(app_dir_name);
    base.push(SCORE_FILE_NAME);
    base
}

/// Loads high score from disk.
///
/// Returns `Ok(0)` when the score file does not yet exist (first run).
/// Returns `Err` when the file exists but cannot be read or parsed, so the
/// caller can surface a warning before entering raw terminal mode.
pub fn load_high_score() -> io::Result<u32> {
    load_score_file().map(|f| f.high_score)
}

/// Saves high score to disk, preserving any existing theme name.
pub fn save_high_score(score: u32) -> io::Result<()> {
    let path = scores_path();
    let mut file = load_score_file().unwrap_or_default();
    file.high_score = score;
    write_score_file_to_path(&path, &file)
}

/// Loads the saved theme name from disk, or `None` when not set.
pub fn load_theme_name() -> io::Result<Option<String>> {
    load_score_file().map(|f| f.theme_name.or(f.theme_id))
}

/// Persists the selected theme name to disk, preserving the high score.
pub fn save_theme_name(name: &str) -> io::Result<()> {
    let path = scores_path();
    let mut file = load_score_file().unwrap_or_default();
    file.theme_id = Some(name.to_owned());
    file.theme_name = Some(name.to_owned());
    write_score_file_to_path(&path, &file)
}

/// Loads the saved theme id from disk, falling back to legacy `theme_name`.
pub fn load_theme_selection() -> io::Result<Option<String>> {
    load_score_file().map(|f| f.theme_id.or(f.theme_name))
}

/// Persists the selected theme id and display name.
pub fn save_theme_selection(theme_id: &str, display_name: &str) -> io::Result<()> {
    let path = scores_path();
    let mut file = load_score_file().unwrap_or_default();
    file.theme_id = Some(theme_id.to_owned());
    file.theme_name = Some(display_name.to_owned());
    write_score_file_to_path(&path, &file)
}

fn load_score_file() -> io::Result<ScoreFile> {
    let current = scores_path();
    if current.exists() {
        return load_score_file_from_path(&current);
    }

    let legacy = legacy_scores_path();
    if legacy.exists() {
        return load_score_file_from_path(&legacy);
    }

    Ok(ScoreFile::default())
}

fn load_score_file_from_path(path: &Path) -> io::Result<ScoreFile> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(ScoreFile::default()),
        Err(e) => return Err(e),
    };

    serde_json::from_str::<ScoreFile>(&raw)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn write_score_file_to_path(path: &Path, file: &ScoreFile) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(file)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ScoreFile, load_score_file, load_score_file_from_path, write_score_file_to_path};

    #[test]
    fn score_serialization_round_trip() {
        let path = unique_test_path("round_trip");

        let file = ScoreFile {
            high_score: 42,
            theme_id: None,
            theme_name: None,
        };
        write_score_file_to_path(&path, &file).expect("score save should succeed");
        let loaded = load_score_file_from_path(&path).expect("load should succeed");

        assert_eq!(loaded.high_score, 42);
        cleanup_test_path(&path);
    }

    #[test]
    fn missing_score_file_returns_zero() {
        let path = unique_test_path("missing");
        // Deliberately do not create the file.
        let loaded =
            load_score_file_from_path(&path).expect("missing file should return Ok(default)");
        assert_eq!(loaded.high_score, 0);
    }

    #[test]
    fn malformed_score_file_returns_error() {
        let path = unique_test_path("malformed");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("test parent directory should be creatable");
        }
        fs::write(&path, "not-json").expect("test file write should succeed");

        assert!(
            load_score_file_from_path(&path).is_err(),
            "malformed file should return Err"
        );

        cleanup_test_path(&path);
    }

    #[test]
    fn theme_name_survives_round_trip() {
        let path = unique_test_path("theme_round_trip");

        let file = ScoreFile {
            high_score: 10,
            theme_id: None,
            theme_name: Some("Ocean".to_owned()),
        };
        write_score_file_to_path(&path, &file).expect("save should succeed");
        let loaded = load_score_file_from_path(&path).expect("load should succeed");

        assert_eq!(loaded.theme_name.as_deref(), Some("Ocean"));
        cleanup_test_path(&path);
    }

    #[test]
    fn legacy_score_file_is_loaded_when_new_path_missing() {
        let root = unique_test_root("legacy_fallback");
        let legacy = root.join("snake").join("scores.json");
        let new = root.join("terminal-snake").join("scores.json");

        if let Some(parent) = legacy.parent() {
            fs::create_dir_all(parent).expect("legacy parent should be creatable");
        }

        let legacy_file = ScoreFile {
            high_score: 77,
            theme_id: Some("opencode".to_owned()),
            theme_name: Some("Opencode".to_owned()),
        };
        write_score_file_to_path(&legacy, &legacy_file).expect("legacy save should succeed");

        let original_home = std::env::var_os("XDG_DATA_HOME");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", &root);
        }

        let loaded = load_score_file().expect("load should succeed from legacy path");
        assert_eq!(loaded.high_score, 77);
        assert_eq!(loaded.theme_id.as_deref(), Some("opencode"));
        assert!(
            !new.exists(),
            "loading legacy data should not create the new score file"
        );

        if let Some(value) = original_home {
            unsafe {
                std::env::set_var("XDG_DATA_HOME", value);
            }
        } else {
            unsafe {
                std::env::remove_var("XDG_DATA_HOME");
            }
        }

        cleanup_test_dir(&root);
    }

    fn unique_test_path(label: &str) -> PathBuf {
        unique_test_root(label).join("scores.json")
    }

    fn unique_test_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();

        std::env::temp_dir()
            .join("snake-score-tests")
            .join(format!("{label}-{nanos}"))
    }

    fn cleanup_test_path(path: &PathBuf) {
        let _ = fs::remove_file(path);
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir(parent);
        }
    }

    fn cleanup_test_dir(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }
}
