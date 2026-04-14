use crate::database::{Database, Image};
use crate::shared_state::AppState;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use walkdir::WalkDir;
use chrono::Utc;

pub struct Scanner {
    db: Arc<Database>,
    state: Arc<RwLock<AppState>>,
    batch_size: usize,
}

impl Scanner {
    pub fn new(db: Arc<Database>, state: Arc<RwLock<AppState>>, batch_size: usize) -> Self {
        Scanner {
            db,
            state,
            batch_size,
        }
    }

    pub async fn scan_paths(&self, paths: &[String]) -> Result<()> {
        let mut state = self.state.write().await;
        state.scanner_running = true;
        drop(state);

        let valid_extensions = ["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff"];
        let mut images = Vec::new();

        for path in paths {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
            {
                let file_path = entry.path();
                if let Some(ext) = file_path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if valid_extensions.contains(&ext_str.to_lowercase().as_str()) {
                            if let Ok(metadata) = entry.metadata() {
                                let image = Image {
                                    id: Uuid::new_v4().to_string(),
                                    path: file_path.to_string_lossy().to_string(),
                                    size: metadata.len(),
                                    content_hash: None,
                                    perceptual_hash: None,
                                    created_at: Utc::now().to_rfc3339(),
                                };
                                images.push(image);

                                if images.len() >= self.batch_size {
                                    self.db.insert_images_batch(&images)?;
                                    let mut state = self.state.write().await;
                                    state.total_images_discovered += images.len();
                                    drop(state);
                                    images.clear();
                                }
                            }
                        }
                    }
                }
            }
        }

        if !images.is_empty() {
            self.db.insert_images_batch(&images)?;
            let mut state = self.state.write().await;
            state.total_images_discovered += images.len();
            drop(state);
        }

        let mut state = self.state.write().await;
        state.scanner_running = false;

        Ok(())
    }

    pub async fn start(&self, paths: &[String]) -> Result<()> {
        self.scan_paths(paths).await
    }

    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        state.scanner_running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scanner_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Arc::new(Database::new(db_path.to_str().unwrap()).unwrap());
        let state = Arc::new(RwLock::new(AppState::new()));

        let scanner = Scanner::new(db, state, 10);
        assert_eq!(scanner.batch_size, 10);
    }

    #[tokio::test]
    async fn test_scanner_scan_empty_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Arc::new(Database::new(db_path.to_str().unwrap()).unwrap());
        let state = Arc::new(RwLock::new(AppState::new()));

        let scanner = Scanner::new(db.clone(), state.clone(), 10);

        let scan_dir = temp_dir.path().join("scan");
        std::fs::create_dir(&scan_dir).unwrap();

        scanner
            .start(&[scan_dir.to_string_lossy().to_string()])
            .await
            .unwrap();

        let state_guard = state.read().await;
        assert_eq!(state_guard.total_images_discovered, 0);
        assert!(!state_guard.scanner_running);
    }

    #[tokio::test]
    async fn test_scanner_finds_images() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Arc::new(Database::new(db_path.to_str().unwrap()).unwrap());
        let state = Arc::new(RwLock::new(AppState::new()));

        let scanner = Scanner::new(db.clone(), state.clone(), 10);

        let scan_dir = temp_dir.path().join("scan");
        std::fs::create_dir(&scan_dir).unwrap();

        // Create test image files
        std::fs::write(scan_dir.join("test1.jpg"), b"fake jpg data").unwrap();
        std::fs::write(scan_dir.join("test2.png"), b"fake png data").unwrap();
        std::fs::write(scan_dir.join("test3.txt"), b"not an image").unwrap();

        scanner
            .start(&[scan_dir.to_string_lossy().to_string()])
            .await
            .unwrap();

        let images = db.get_all_images().unwrap();
        assert_eq!(images.len(), 2);
        assert!(!images.iter().any(|img| img.path.contains("test3.txt")));
    }
}
