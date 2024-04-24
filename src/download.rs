use std::io::SeekFrom;

use axum::{
    body::Body,
    extract::{Path, Query},
    http::header::CONTENT_DISPOSITION,
    response::IntoResponse,
    Extension,
};
use chacha20poly1305::{aead::stream::DecryptorBE32, XChaCha20Poly1305};
use serde::Deserialize;
use tokio::{
    fs::{self, File},
    io::{AsyncSeekExt, AsyncWriteExt},
};
use tokio_util::io::ReaderStream;

use crate::{
    errors::{AppError, AppResult},
    repository::{add_download, fetch_upload},
    utilities::{read_chunk, temp_file, DEC_CHUNK_SIZE},
    AppContext, STORAGE_PATH,
};

pub async fn download_endpoint(
    ctx: Extension<AppContext>,
    Path(upload_id): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> AppResult<impl IntoResponse> {
    let upload = fetch_upload(&ctx.db, &upload_id)
        .await?
        .ok_or(AppError::UploadNotFound)?;

    let mut file = File::open(format!("{STORAGE_PATH}{upload_id}")).await?;

    if let Some(nonce) = upload.nonce {
        let nonce_bytes = hex::decode(nonce)?;
        let key = query.key.ok_or(AppError::MissingKey)?;
        let key_hash = upload.key_hash.ok_or(AppError::CorruptedUpload)?;

        if sha256::digest(&key) != key_hash {
            return Err(AppError::InvalidDecryptionKey);
        }

        let key_bytes = hex::decode(key)?;

        let (mut temp_file, temp_path) = temp_file().await?;
        let mut decryptor = DecryptorBE32::<XChaCha20Poly1305>::new(
            key_bytes.as_slice().into(),
            nonce_bytes.as_slice().into(),
        );
        loop {
            let chunk = read_chunk(&mut file, DEC_CHUNK_SIZE).await?;

            if chunk.len() < DEC_CHUNK_SIZE {
                let plaintext = decryptor.decrypt_last(chunk.as_slice())?;
                temp_file.write_all(&plaintext).await?;
                break;
            } else {
                let plaintext = decryptor.decrypt_next(chunk.as_slice())?;
                temp_file.write_all(&plaintext).await?;
            }
        }

        if let Err(why) = add_download(&ctx.db, &upload_id).await {
            tracing::warn!("failed to update download count for `{upload_id}`: {why:?}");
        }

        temp_file.seek(SeekFrom::Start(0)).await?;
        let stream = ReaderStream::new(temp_file);
        let body = Body::from_stream(stream);

        if let Err(why) = fs::remove_file(&temp_path).await {
            tracing::warn!("failed to remove decryption temp file ({temp_path}): {why:?}");
        }

        Ok((
            [(
                CONTENT_DISPOSITION,
                format!(r#"attachment; filename="{}""#, upload.file_name),
            )],
            body,
        ))
    } else {
        if let Err(why) = add_download(&ctx.db, &upload_id).await {
            tracing::warn!("failed to update download count for `{upload_id}`: {why:?}");
        }

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
}

#[derive(Deserialize)]
pub struct DownloadQuery {
    key: Option<String>,
}
