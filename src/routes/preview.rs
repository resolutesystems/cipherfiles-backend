use axum::{body::Body, http::header::{CONTENT_DISPOSITION, CONTENT_TYPE}, response::IntoResponse, Extension};
use infer::MatcherType;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{errors::{AppError, AppResult}, extractors, repository::fetch_upload, AppContext};

pub async fn preview_endpoint(
    ctx: Extension<AppContext>,
    extractors::Path(upload_id): extractors::Path<String>,
) -> AppResult<impl IntoResponse> {
    let upload = fetch_upload(&ctx.db, &upload_id)
        .await?
        .ok_or(AppError::UploadNotFound)?;

    if let Some(_) = upload.nonce {
        return Err(AppError::PreviewNotSupported);
    }

    let file_path = format!("{}{upload_id}", ctx.cfg.general.storage_dir);
    let kind = infer::get_from_path(&file_path)?.ok_or(AppError::PreviewNotSupported)?;

    if kind.matcher_type() != MatcherType::Image && kind.matcher_type() != MatcherType::Video {
        return Err(AppError::PreviewNotSupported);
    }

    if upload.bytes > ctx.cfg.general.max_preview_bytes as i64 {
        return Err(AppError::MediaTooBig);
    }

    let file = File::open(file_path).await?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok((
        [
            (CONTENT_TYPE, kind.mime_type().to_string()),
            (CONTENT_DISPOSITION, format!(r#"attachment; filename="{}""#, upload.file_name)),
        ],
        body,
    ))
}
