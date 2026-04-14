use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub scanner_running: bool,
    pub scanner_paused: bool,
    pub hasher_running: bool,
    pub hasher_paused: bool,
    pub grouper_running: bool,
    pub total_images_discovered: usize,
    pub total_images_hashed: usize,
    pub total_images_to_hash: usize,
    pub total_duplicate_groups: usize,
    pub hashing_speed: f64, // images per second
    pub scanner_paths: Vec<String>,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub last_update: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub hash: String,
    pub perceptual_hash: String,
    pub images: Vec<ImageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub id: String,
    pub path: String,
    pub size: u64,
    pub content_hash: String,
    pub perceptual_hash: String,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            scanner_running: false,
            scanner_paused: false,
            hasher_running: false,
            hasher_paused: false,
            grouper_running: false,
            total_images_discovered: 0,
            total_images_hashed: 0,
            total_images_to_hash: 0,
            total_duplicate_groups: 0,
            hashing_speed: 0.0,
            scanner_paths: vec![],
            duplicate_groups: vec![],
            last_update: SystemTime::now(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState::new()
    }
}
