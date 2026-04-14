use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Result as SqliteResult};
use uuid::Uuid;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Image {
    pub id: String,
    pub path: String,
    pub size: u64,
    pub content_hash: Option<String>,
    pub perceptual_hash: Option<String>,
    pub created_at: String,
}

pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::new(manager)?;

        let db = Database { pool };
        db.initialize_schema()?;
        Ok(db)
    }

    fn initialize_schema(&self) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS images (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                size INTEGER NOT NULL,
                content_hash TEXT,
                perceptual_hash TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_images_content_hash ON images(content_hash);
            CREATE INDEX IF NOT EXISTS idx_images_perceptual_hash ON images(perceptual_hash);",
        )?;
        Ok(())
    }

    pub fn insert_images_batch(&self, images: &[Image]) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO images (id, path, size, content_hash, perceptual_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?;

        for image in images {
            stmt.execute(params![
                &image.id,
                &image.path,
                image.size,
                &image.content_hash,
                &image.perceptual_hash,
                &image.created_at
            ])?;
        }
        Ok(())
    }

    pub fn get_all_images(&self) -> Result<Vec<Image>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, path, size, content_hash, perceptual_hash, created_at FROM images ORDER BY created_at"
        )?;

        let images = stmt.query_map([], |row| {
            Ok(Image {
                id: row.get(0)?,
                path: row.get(1)?,
                size: row.get(2)?,
                content_hash: row.get(3)?,
                perceptual_hash: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;

        Ok(images)
    }

    pub fn get_images_without_content_hash(&self, limit: usize) -> Result<Vec<Image>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, path, size, content_hash, perceptual_hash, created_at
             FROM images WHERE content_hash IS NULL LIMIT ?"
        )?;

        let images = stmt.query_map([limit as i64], |row| {
            Ok(Image {
                id: row.get(0)?,
                path: row.get(1)?,
                size: row.get(2)?,
                content_hash: row.get(3)?,
                perceptual_hash: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;

        Ok(images)
    }

    pub fn update_image_hashes(&self, id: &str, content_hash: &str, perceptual_hash: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE images SET content_hash = ?1, perceptual_hash = ?2 WHERE id = ?3",
            params![content_hash, perceptual_hash, id],
        )?;
        Ok(())
    }

    pub fn get_image_count(&self) -> Result<usize> {
        let conn = self.pool.get()?;
        let count: usize = conn.query_row(
            "SELECT COUNT(*) FROM images",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn get_hashed_image_count(&self) -> Result<usize> {
        let conn = self.pool.get()?;
        let count: usize = conn.query_row(
            "SELECT COUNT(*) FROM images WHERE content_hash IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn get_image_by_id(&self, id: &str) -> Result<Option<Image>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, path, size, content_hash, perceptual_hash, created_at FROM images WHERE id = ?1"
        )?;

        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Image {
                id: row.get(0)?,
                path: row.get(1)?,
                size: row.get(2)?,
                content_hash: row.get(3)?,
                perceptual_hash: row.get(4)?,
                created_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_image(&self, id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM images WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn get_duplicate_groups(&self) -> Result<Vec<(String, Vec<Image>)>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT content_hash, id, path, size, content_hash, perceptual_hash, created_at
             FROM images WHERE content_hash IS NOT NULL AND content_hash IN (
                SELECT content_hash FROM images WHERE content_hash IS NOT NULL GROUP BY content_hash HAVING COUNT(*) > 1
             ) ORDER BY content_hash"
        )?;

        let mut groups: std::collections::HashMap<String, Vec<Image>> = std::collections::HashMap::new();

        let images = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, // content_hash
                Image {
                    id: row.get(1)?,
                    path: row.get(2)?,
                    size: row.get(3)?,
                    content_hash: row.get(4)?,
                    perceptual_hash: row.get(5)?,
                    created_at: row.get(6)?,
                }
            ))
        })?
        .collect::<SqliteResult<Vec<_>>>()?;

        for (hash, image) in images {
            groups.entry(hash).or_insert_with(Vec::new).push(image);
        }

        let result: Vec<(String, Vec<Image>)> = groups.into_iter().collect();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let result = Database::new(db_path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_insert_and_get_images() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path.to_str().unwrap()).unwrap();

        let images = vec![
            Image {
                id: Uuid::new_v4().to_string(),
                path: "/path/to/photo1.jpg".to_string(),
                size: 1024,
                content_hash: None,
                perceptual_hash: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            },
            Image {
                id: Uuid::new_v4().to_string(),
                path: "/path/to/photo2.jpg".to_string(),
                size: 2048,
                content_hash: None,
                perceptual_hash: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            },
        ];

        db.insert_images_batch(&images).unwrap();
        let retrieved = db.get_all_images().unwrap();
        assert_eq!(retrieved.len(), 2);
    }

    #[test]
    fn test_update_image_hashes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path.to_str().unwrap()).unwrap();

        let image_id = Uuid::new_v4().to_string();
        let images = vec![Image {
            id: image_id.clone(),
            path: "/path/to/photo.jpg".to_string(),
            size: 1024,
            content_hash: None,
            perceptual_hash: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        db.insert_images_batch(&images).unwrap();
        db.update_image_hashes(&image_id, "hash123", "phash456").unwrap();

        let images = db.get_all_images().unwrap();
        assert_eq!(images[0].content_hash, Some("hash123".to_string()));
        assert_eq!(images[0].perceptual_hash, Some("phash456".to_string()));
    }

    #[test]
    fn test_get_hashed_image_count() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path.to_str().unwrap()).unwrap();

        let image_id = Uuid::new_v4().to_string();
        let images = vec![Image {
            id: image_id.clone(),
            path: "/path/to/photo.jpg".to_string(),
            size: 1024,
            content_hash: None,
            perceptual_hash: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        db.insert_images_batch(&images).unwrap();
        assert_eq!(db.get_hashed_image_count().unwrap(), 0);

        db.update_image_hashes(&image_id, "hash123", "phash456").unwrap();
        assert_eq!(db.get_hashed_image_count().unwrap(), 1);
    }
}
