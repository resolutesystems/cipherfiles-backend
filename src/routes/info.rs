use axum::{
    Extension, Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    errors::{AppError, AppResult}, extractors, repository::fetch_upload, AppContext
};

use super::delete::delete_upload;

pub async fn info_endpoint(
    ctx: Extension<AppContext>,
    extractors::Path(upload_id): extractors::Path<String>,
    extractors::Query(query): extractors::Query<InfoQuery>,
) -> AppResult<Json<InfoResponse>> {
    let upload = fetch_upload(&ctx.db, &upload_id)
        .await?
        .ok_or(AppError::UploadNotFound)?;

    // TODO(hito): actually nice and better way of handling expired uploads
    // because right now they are only removed IF someone tries to download them
    // thus files that never get requested will stay in database and storage forever
    if let Some(expiry_hours) = upload.expiry_hours {
        if Utc::now() >= upload.created_at + Duration::hours(expiry_hours as _) {
            if let Err(why) = delete_upload(&ctx.db, &ctx.cfg.general.storage_dir, &upload_id).await {
                tracing::error!("Failed to remove expired upload with id {upload_id}: {why:?}");
            }
            return Err(AppError::UploadExpired);
        }
    }

    if let Some(key_hash) = upload.key_hash {
        let query_key = query.key.ok_or(AppError::MissingKey)?;

        if sha256::digest(query_key) != key_hash {
            return Err(AppError::InvalidDecryptionKey)?;
        }
    }

    Ok(Json(InfoResponse {
        file_name: upload.file_name,
        bytes: upload.bytes,
        downloads: upload.downloads,
    }))
}

#[derive(Deserialize)]
pub struct InfoQuery {
    key: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    file_name: String,
    bytes: i64,
    downloads: i32,
}
