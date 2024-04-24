use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    errors::{AppError, AppResult},
    repository::fetch_upload,
    AppContext,
};

pub async fn info_endpoint(
    ctx: Extension<AppContext>,
    Path(upload_id): Path<String>,
    Query(query): Query<InfoQuery>,
) -> AppResult<Json<InfoResponse>> {
    let upload = fetch_upload(&ctx.db, &upload_id)
        .await?
        .ok_or(AppError::UploadNotFound)?;

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
