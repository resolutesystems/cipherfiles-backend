use axum::{body::Body, extract::Path, http::header::CONTENT_DISPOSITION, response::IntoResponse, Extension};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{errors::{AppError, AppResult}, repository::fetch_upload, AppContext};

pub async fn preview_endpoint(ctx: Extension<AppContext>, Path(upload_id): Path<String>) -> AppResult<impl IntoResponse> {
    let upload = fetch_upload(&ctx.db, &upload_id)
        .await?
        .ok_or(AppError::UploadNotFound)?;

    if let Some(_) = upload.nonce {
        return Err(AppError::PreviewNotSupported);
    }

    if upload.bytes > ctx.cfg.general.max_preview_bytes as i64 {
        return Err(AppError::MediaTooBig);
    }

    let file_path = format!("{}{upload_id}", ctx.cfg.general.storage_dir);
    let file = File::open(file_path).await?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok((
        [(
            CONTENT_DISPOSITION,
            format!(r#"attachment; filename="{}""#, upload.file_name),
        )],
        body,
    ))
}
