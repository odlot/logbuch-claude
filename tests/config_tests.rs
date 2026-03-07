use std::path::PathBuf;
use std::sync::Mutex;

use logbuch::config::Config;

/// Serialises env-var mutation so parallel tests cannot interfere.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Config::default
// ---------------------------------------------------------------------------

#[test]
fn config_default_session_duration_is_45_minutes() {
    // Act
    let config = Config::default();

    // Assert
    assert_eq!(config.session_duration_min, 45);
}

// ---------------------------------------------------------------------------
// Config::load
// ---------------------------------------------------------------------------

#[test]
fn config_load_returns_defaults_when_config_file_does_not_exist() {
    // Arrange
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.toml");

    // Act
    let config = Config::load(Some(&path)).unwrap();

    // Assert
    assert_eq!(config.session_duration_min, 45);
}

#[test]
fn config_load_reads_session_duration_from_toml_file() {
    // Arrange
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "session_duration_min = 50\n").unwrap();

    // Act
    let config = Config::load(Some(&path)).unwrap();

    // Assert
    assert_eq!(config.session_duration_min, 50);
}

#[test]
fn config_load_reads_db_path_from_toml_file() {
    // Arrange
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "db_path = \"/tmp/mydb.sqlite\"\n").unwrap();

    // Act
    let config = Config::load(Some(&path)).unwrap();

    // Assert
    assert_eq!(config.db_path, PathBuf::from("/tmp/mydb.sqlite"));
}

// ---------------------------------------------------------------------------
// Config::write_to + Config::load roundtrip
// ---------------------------------------------------------------------------

#[test]
fn config_write_to_creates_a_file_that_can_be_loaded_back() {
    // Arrange
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    let original = Config {
        session_duration_min: 45,
        db_path: dir.path().join("data.db"),
    };

    // Act
    original.write_to(&path).unwrap();
    let loaded = Config::load(Some(&path)).unwrap();

    // Assert
    assert_eq!(loaded.session_duration_min, 45);
    assert_eq!(loaded.db_path, dir.path().join("data.db"));
}

#[test]
fn config_write_to_creates_parent_directories_if_missing() {
    // Arrange
    let dir = tempfile::TempDir::new().unwrap();
    let nested_path = dir.path().join("a").join("b").join("config.toml");
    let config = Config::default();

    // Act
    config.write_to(&nested_path).unwrap();

    // Assert
    assert!(nested_path.exists());
}

// ---------------------------------------------------------------------------
// Config::apply_env_overrides
// ---------------------------------------------------------------------------

#[test]
fn config_env_override_sets_db_path() {
    let _guard = ENV_MUTEX.lock().unwrap();

    // Arrange
    std::env::set_var("LOGBUCH_DB_PATH", "/env/override.db");
    let mut config = Config::default();

    // Act
    config.apply_env_overrides();

    // Assert
    assert_eq!(config.db_path, PathBuf::from("/env/override.db"));

    std::env::remove_var("LOGBUCH_DB_PATH");
}

#[test]
fn config_env_override_sets_session_duration() {
    let _guard = ENV_MUTEX.lock().unwrap();

    // Arrange
    std::env::set_var("LOGBUCH_SESSION_DURATION", "35");
    let mut config = Config::default();

    // Act
    config.apply_env_overrides();

    // Assert
    assert_eq!(config.session_duration_min, 35);

    std::env::remove_var("LOGBUCH_SESSION_DURATION");
}

#[test]
fn config_env_override_ignores_non_numeric_session_duration() {
    let _guard = ENV_MUTEX.lock().unwrap();

    // Arrange
    std::env::set_var("LOGBUCH_SESSION_DURATION", "not_a_number");
    let mut config = Config::default();

    // Act
    config.apply_env_overrides();

    // Assert: unchanged from default
    assert_eq!(config.session_duration_min, 45);

    std::env::remove_var("LOGBUCH_SESSION_DURATION");
}

// ---------------------------------------------------------------------------
// Config::print_summary
// ---------------------------------------------------------------------------

#[test]
fn config_print_summary_does_not_panic() {
    // Arrange
    let config = Config::default();
    let path = PathBuf::from("/tmp/config.toml");

    // Act / Assert: just verify it does not panic
    config.print_summary(&path);
}
