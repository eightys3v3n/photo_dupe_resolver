use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub db_path: String,
    pub scanner_batch_size: usize,
    pub scanner_threads: usize,
    pub hasher_worker_threads: usize,
    pub hasher_batch_queue_size: usize,
    pub hasher_batch_size: usize,
    pub hash_algorithm: String,
    pub perceptual_hash_algorithm: String,
    pub scan_paths: Vec<String>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    pub conditions: Vec<String>,
    pub actions: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            db_path: "photos.db".to_string(),
            scanner_batch_size: 100,
            scanner_threads: 4,
            hasher_worker_threads: 4,
            hasher_batch_queue_size: 10,
            hasher_batch_size: 50,
            hash_algorithm: "sha256".to_string(),
            perceptual_hash_algorithm: "phash".to_string(),
            scan_paths: vec![],
            scenarios: vec![],
        }
    }
}

impl Config {
    pub fn load(config_path: &str) -> Result<Self> {
        if Path::new(config_path).exists() {
            let content = std::fs::read_to_string(config_path)
                .context("Failed to read config file")?;
            let config: Config = toml::from_str(&content)
                .context("Failed to parse config file")?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self, config_path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    pub fn apply_cli_overrides(&mut self, args: &crate::cli::Args) {
        if let Some(db_path) = &args.db_path {
            self.db_path = db_path.clone();
        }
        if let Some(scanner_threads) = args.scanner_threads {
            self.scanner_threads = scanner_threads;
        }
        if let Some(hasher_threads) = args.hasher_threads {
            self.hasher_worker_threads = hasher_threads;
        }
        if !args.scan_paths.is_empty() {
            self.scan_paths = args.scan_paths.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.db_path, "photos.db");
        assert_eq!(config.scanner_threads, 4);
        assert_eq!(config.hasher_worker_threads, 4);
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("test.toml");
        let config_path_str = config_path.to_str().unwrap();

        let mut original = Config::default();
        original.scanner_threads = 8;
        original.scan_paths = vec!["/home/photos".to_string()];

        original.save(config_path_str).unwrap();

        let loaded = Config::load(config_path_str).unwrap();
        assert_eq!(loaded.scanner_threads, 8);
        assert_eq!(loaded.scan_paths, vec!["/home/photos".to_string()]);
    }

    #[test]
    fn test_partial_config_uses_defaults() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("partial.toml");

        std::fs::write(&config_path, "db_path = \"custom.db\"").unwrap();

        let loaded = Config::load(config_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded.db_path, "custom.db");
        assert_eq!(loaded.scanner_batch_size, 100);
        assert!(loaded.scenarios.is_empty());
    }
}
