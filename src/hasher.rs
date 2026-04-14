use crate::database::Database;
use crate::shared_state::AppState;
use anyhow::Result;
use sha2::{Sha256, Digest};
use std::fs;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

pub struct Hasher {
    db: Arc<Database>,
    state: Arc<RwLock<AppState>>,
    worker_threads: usize,
    batch_queue_size: usize,
    batch_size: usize,
}

impl Hasher {
    pub fn new(
        db: Arc<Database>,
        state: Arc<RwLock<AppState>>,
        worker_threads: usize,
        batch_queue_size: usize,
        batch_size: usize,
    ) -> Self {
        Hasher {
            db,
            state,
            worker_threads,
            batch_queue_size,
            batch_size,
        }
    }

    pub async fn hash_images(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.hasher_running = true;
        drop(state);

        let start_time = Instant::now();
        let mut total_hashed = 0;

        loop {
            let images = self.db.get_images_without_content_hash(self.batch_size)?;
            if images.is_empty() {
                break;
            }

            let batch_len = images.len();

            // Process hashing
            for image in images {
                let content_hash = self.compute_content_hash(&image.path)?;
                let perceptual_hash = self.compute_perceptual_hash(&image.path).unwrap_or_default();
                self.db
                    .update_image_hashes(&image.id, &content_hash, &perceptual_hash)?;
            }

            total_hashed += batch_len;

            // Update state
            let mut state = self.state.write().await;
            state.total_images_hashed = total_hashed;
            let elapsed = start_time.elapsed().as_secs_f64();
            state.hashing_speed = if elapsed > 0.0 {
                total_hashed as f64 / elapsed
            } else {
                0.0
            };
            drop(state);
        }

        let mut state = self.state.write().await;
        state.hasher_running = false;

        Ok(())
    }

    pub fn compute_content_hash(&self, path: &str) -> Result<String> {
        let data = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    pub fn compute_perceptual_hash(&self, path: &str) -> Result<String> {
        // Simple perceptual hash using file size and first few bytes
        let data = fs::read(path)?;
        let mut hasher = Sha256::new();

        // Use first 1KB and last 1KB of file
        let first_part = if data.len() > 1024 {
            &data[..1024]
        } else {
            &data
        };

        let last_part = if data.len() > 1024 {
            &data[data.len() - 1024..]
        } else {
            &data
        };

        hasher.update(first_part);
        hasher.update(last_part);
        hasher.update(data.len().to_le_bytes());

        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    pub async fn start(&self) -> Result<()> {
        self.hash_images().await
    }

    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        state.hasher_running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn build_hasher() -> Hasher {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Arc::new(Database::new(db_path.to_str().unwrap()).unwrap());
        let state = Arc::new(RwLock::new(AppState::new()));
        Hasher::new(db, state, 4, 10, 50)
    }

    fn fixture_path(name: &str) -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join("hasher")
            .join(name)
            .to_string_lossy()
            .to_string()
    }

    #[tokio::test]
    async fn test_hasher_creation() {
        let hasher = build_hasher();
        assert_eq!(hasher.worker_threads, 4);
        assert_eq!(hasher.batch_size, 50);
    }

    #[tokio::test]
    async fn test_compute_content_hash() {
        let hasher = build_hasher();

        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.bin");
        fs::write(&test_file, b"test data").unwrap();

        let hash = hasher
            .compute_content_hash(test_file.to_str().unwrap())
            .unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64);
    }

    #[tokio::test]
    async fn test_compute_perceptual_hash() {
        let hasher = build_hasher();

        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.bin");
        fs::write(&test_file, b"test data").unwrap();

        let hash = hasher
            .compute_perceptual_hash(test_file.to_str().unwrap())
            .unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64);
    }

    #[tokio::test]
    async fn test_same_content_produces_same_hash() {
        let hasher = build_hasher();

        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.bin");
        fs::write(&test_file, b"test data").unwrap();

        let hash1 = hasher
            .compute_content_hash(test_file.to_str().unwrap())
            .unwrap();
        let hash2 = hasher
            .compute_content_hash(test_file.to_str().unwrap())
            .unwrap();

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_real_duplicate_images_have_same_hashes() {
        let hasher = build_hasher();
        let first = fixture_path("sample_a.png");
        let copy = fixture_path("sample_a_copy.png");

        let content_hash_1 = hasher.compute_content_hash(&first).unwrap();
        let content_hash_2 = hasher.compute_content_hash(&copy).unwrap();
        let perceptual_hash_1 = hasher.compute_perceptual_hash(&first).unwrap();
        let perceptual_hash_2 = hasher.compute_perceptual_hash(&copy).unwrap();

        assert_eq!(content_hash_1, content_hash_2);
        assert_eq!(perceptual_hash_1, perceptual_hash_2);
    }

    #[tokio::test]
    async fn test_real_different_images_produce_different_content_hashes() {
        let hasher = build_hasher();
        let first = fixture_path("sample_a.png");
        let second = fixture_path("sample_b.png");

        let hash1 = hasher.compute_content_hash(&first).unwrap();
        let hash2 = hasher.compute_content_hash(&second).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_missing_fixture_returns_error() {
        let hasher = build_hasher();
        let missing = fixture_path("does_not_exist.png");

        assert!(hasher.compute_content_hash(&missing).is_err());
        assert!(hasher.compute_perceptual_hash(&missing).is_err());
    }
}
