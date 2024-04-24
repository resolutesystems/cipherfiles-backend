use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension,
};
use serde::Deserialize;
use tokio::fs;

use crate::{
    errors::{AppError, AppResult},
    models::Upload,
    repository, AppContext, STORAGE_PATH,
};

pub async fn delete_upload(
    ctx: Extension<AppContext>,
    Path(upload_id): Path<String>,
    Query(query): Query<DeleteQuery>,
) -> AppResult<StatusCode> {
    // check if upload exists
    let upload = sqlx::query_as!(Upload, "SELECT * FROM uploads WHERE id = $1", upload_id)
        .fetch_optional(&ctx.db)
        .await?
        .ok_or(AppError::UploadNotFound)?;

    // check if delete key matches
    if upload.delete_key != query.key {
        return Err(AppError::InvalidDeleteKey);
    }

    // if so then delete
    repository::delete_upload(&ctx.db, &upload_id).await?;

    let file_path = format!("{STORAGE_PATH}{upload_id}");
    fs::remove_file(file_path).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct DeleteQuery {
    key: String,
}
