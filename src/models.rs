use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Upload {
    pub id: String,
    pub key_hash: Option<String>,
    pub delete_key: String,
    pub nonce: Option<String>,
    pub file_name: String,
    pub bytes: i64,
    pub downloads: i32,
    pub expiry_hours: Option<i32>,
    pub expiry_downloads: Option<i32>,
    pub created_at: DateTime<Utc>,
}
