use sqlx::PgPool;

use crate::models::Upload;

pub async fn fetch_upload(db: &PgPool, id: &str) -> sqlx::Result<Option<Upload>> {
    let res = sqlx::query_as!(Upload, "SELECT * FROM uploads WHERE id = $1", id)
        .fetch_optional(db)
        .await?;
    Ok(res)
}

pub async fn insert_upload(db: &PgPool, insert: InsertUpload) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO uploads
            (id, key_hash, delete_key, nonce, file_name, bytes, expiry_hours, expiry_downloads)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        insert.id,
        insert.key_hash,
        insert.delete_key,
        insert.nonce,
        insert.file_name,
        insert.bytes as i64,
        insert.expiry_hours.map(|n| n as i32),
        insert.expiry_downloads.map(|n| n as i32),
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete_upload(db: &PgPool, id: &str) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM uploads WHERE id = $1", id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn add_download(db: &PgPool, id: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE uploads SET downloads = downloads + 1 WHERE id = $1",
        id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_stats(db: &PgPool, bytes: u64) -> sqlx::Result<()> {
    sqlx::query!("UPDATE stats SET files_uploaded = files_uploaded + 1, bytes_uploaded = bytes_uploaded + $1 WHERE id = 1", bytes as i64)
        .execute(db)
        .await?;
    Ok(())
}

pub struct InsertUpload {
    pub id: String,
    pub key_hash: Option<String>,
    pub delete_key: String,
    pub nonce: Option<String>,
    pub file_name: String,
    pub bytes: usize,
    pub expiry_hours: Option<u32>,
    pub expiry_downloads: Option<u32>,
}
