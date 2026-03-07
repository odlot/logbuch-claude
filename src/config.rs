use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Duration of a new session in minutes (default: 45)
    pub session_duration_min: u32,
    /// Path to the SQLite database file
    pub db_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("logbuch");
        Self {
            session_duration_min: 45,
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
            Config::default()
        };

        // Expand `~` in paths supplied via the config file.
        config.db_path = expand_tilde(config.db_path);

        // Apply environment variable overrides.
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply `LOGBUCH_*` environment variable overrides.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("LOGBUCH_DB_PATH") {
            self.db_path = expand_tilde(PathBuf::from(val));
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

    /// Write the config to a TOML file (called by the wizard after the user confirms).
    pub fn write_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating config directory {}", parent.display()))?;
        }
        let content = format!(
            r#"# Logbuch configuration
# Full documentation: https://github.com/odlot/logbuch

# Duration of a new session in minutes (default: 45).
session_duration_min = {duration}

# Path to the SQLite database file.
db_path = "{db}"
"#,
            duration = self.session_duration_min,
            db = self.db_path.display(),
        );
        std::fs::write(path, content)
            .with_context(|| format!("writing config to {}", path.display()))?;
        Ok(())
    }

    /// Print a human-readable summary of the effective configuration to stdout.
    pub fn print_summary(&self, config_path: &Path) {
        println!("Logbuch effective configuration");
        println!("  Config file:      {}", config_path.display());
        println!("  Database:         {}", self.db_path.display());
        println!("  Session duration: {} minutes", self.session_duration_min);
        println!();
        println!("Environment variable overrides (if set):");
        println!("  LOGBUCH_DB_PATH           -> db_path");
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
pub(crate) fn expand_tilde(path: PathBuf) -> PathBuf {
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
