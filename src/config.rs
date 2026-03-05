use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Duration of a new session in minutes (default: 25)
    pub session_duration_min: u32,
    /// Directory where summary reports are written
    pub summary_export_dir: PathBuf,
    /// Path to the SQLite database file
    pub db_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("logbuch");
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            session_duration_min: 25,
            summary_export_dir: home.join("logbuch-reports"),
            db_path: data_dir.join("logbuch.db"),
        }
    }
}

impl Config {
    /// Load config from the given path (or the default XDG location).
    ///
    /// Precedence (highest → lowest):
    ///   1. Values set by the caller after this function returns (CLI flags)
    ///   2. Environment variables  (`LOGBUCH_DB_PATH`, `LOGBUCH_SUMMARY_DIR`,
    ///      `LOGBUCH_SESSION_DURATION`)
    ///   3. Config file values
    ///   4. Built-in defaults
    ///
    /// If the config file does not exist it is created automatically with all
    /// options commented out so the user can see what is available.
    pub fn load(path: Option<&PathBuf>) -> Result<Self> {
        let config_path = match path {
            Some(p) => p.clone(),
            None => default_config_path(),
        };

        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("reading config file {}", config_path.display()))?;
            let cfg: Config = toml::from_str(&content)
                .with_context(|| format!("parsing config file {}", config_path.display()))?;
            cfg
        } else {
            // First run: write a commented-out default config so the user can
            // discover the available options without reading documentation.
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating config directory {}", parent.display()))?;
            }
            if let Err(e) = Self::write_default_config(&config_path) {
                // Non-fatal: warn but continue with in-memory defaults.
                eprintln!(
                    "Warning: could not write default config to {}: {}",
                    config_path.display(),
                    e
                );
            }
            Config::default()
        };

        // Expand `~` in paths supplied via the config file.
        config.db_path = expand_tilde(config.db_path);
        config.summary_export_dir = expand_tilde(config.summary_export_dir);

        // Apply environment variable overrides.
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply `LOGBUCH_*` environment variable overrides.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("LOGBUCH_DB_PATH") {
            self.db_path = expand_tilde(PathBuf::from(val));
        }
        if let Ok(val) = std::env::var("LOGBUCH_SUMMARY_DIR") {
            self.summary_export_dir = expand_tilde(PathBuf::from(val));
        }
        if let Ok(val) = std::env::var("LOGBUCH_SESSION_DURATION") {
            if let Ok(mins) = val.parse::<u32>() {
                self.session_duration_min = mins;
            } else {
                eprintln!(
                    "Warning: LOGBUCH_SESSION_DURATION='{}' is not a valid number, ignoring",
                    val
                );
            }
        }
    }

    /// Write a fully-commented default config file so the user can see every
    /// available option with its default value.
    fn write_default_config(path: &Path) -> Result<()> {
        let defaults = Config::default();
        let content = format!(
            r#"# Logbuch configuration
# Generated automatically on first run. Uncomment and edit any value.
# Full documentation: https://github.com/odlot/logbuch

# Duration of a new Pomodoro session in minutes.
# session_duration_min = {duration}

# Directory where daily/weekly summary reports are written (Markdown).
# summary_export_dir = "{summary}"

# Path to the SQLite database file.
# db_path = "{db}"
"#,
            duration = defaults.session_duration_min,
            summary = defaults.summary_export_dir.display(),
            db = defaults.db_path.display(),
        );
        std::fs::write(path, content)
            .with_context(|| format!("writing default config to {}", path.display()))?;
        Ok(())
    }

    /// Print a human-readable summary of the effective configuration to stdout.
    pub fn print_summary(&self, config_path: &Path) {
        println!("Logbuch effective configuration");
        println!("  Config file:      {}", config_path.display());
        println!("  Database:         {}", self.db_path.display());
        println!("  Reports dir:      {}", self.summary_export_dir.display());
        println!("  Session duration: {} minutes", self.session_duration_min);
        println!();
        println!("Environment variable overrides (if set):");
        println!("  LOGBUCH_DB_PATH           -> db_path");
        println!("  LOGBUCH_SUMMARY_DIR       -> summary_export_dir");
        println!("  LOGBUCH_SESSION_DURATION  -> session_duration_min");
    }
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("logbuch")
        .join("config.toml")
}

/// Expand a leading `~/` or bare `~` to the user's home directory.
fn expand_tilde(path: PathBuf) -> PathBuf {
    let s = match path.to_str() {
        Some(s) => s,
        None => return path,
    };
    if s == "~" {
        return dirs::home_dir().unwrap_or(path);
    }
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    path
}
