use axum::{
    extract::{multipart::Field, Multipart, Query},
    Extension, Json,
};
use chacha20poly1305::{
    aead::{rand_core::RngCore, stream::EncryptorBE32, OsRng},
    XChaCha20Poly1305,
};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::{
    fs::File,
    io::{self, AsyncRead, AsyncWriteExt},
};
use tokio_util::io::StreamReader;

use crate::{
    errors::{AppError, AppResult},
    repository::{insert_upload, update_stats, InsertUpload},
    utilities::{friendly_id, read_chunk, ENC_CHUNK_SIZE},
    AppContext, STORAGE_PATH,
};

async fn save_encrypted_file<R>(
    file_name: &str,
    key: &[u8; 32],
    nonce: &[u8; 19],
    body: &mut R,
) -> AppResult<usize>
where
    R: AsyncRead + Unpin,
{
    let mut file = File::create(format!("{STORAGE_PATH}{file_name}")).await?;
    let mut encryptor =
        EncryptorBE32::<XChaCha20Poly1305>::new(key.as_ref().into(), nonce.as_ref().into());
    let mut total_bytes = 0;

    loop {
        let chunk = read_chunk(body, ENC_CHUNK_SIZE).await?;
        total_bytes += chunk.len();

        if chunk.len() < ENC_CHUNK_SIZE {
            let ciphertext = encryptor.encrypt_last(chunk.as_slice())?;
            file.write_all(&ciphertext).await?;
            break;
        } else {
            let ciphertext = encryptor.encrypt_next(chunk.as_slice())?;
            file.write_all(&ciphertext).await?;
        }
    }

    Ok(total_bytes)
}

async fn save_file<R>(file_name: &str, body: &mut R) -> AppResult<usize>
where
    R: AsyncRead + Unpin,
{
    let mut file = File::create(format!("{STORAGE_PATH}{file_name}")).await?;
    let mut total_bytes = 0;

    loop {
        let chunk = read_chunk(body, ENC_CHUNK_SIZE).await?;
        total_bytes += chunk.len();

        file.write_all(&chunk).await?;
        if chunk.len() < ENC_CHUNK_SIZE {
            break;
        }
    }

    Ok(total_bytes)
}

async fn handle_upload(
    db: &PgPool,
    field: Field<'_>,
    file_name: String,
    encrypt: bool,
    expiry_hours: Option<u32>,
    expiry_downloads: Option<u32>,
) -> AppResult<UploadResponse> {
    let body = field.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
    let mut body_reader = StreamReader::new(body);

    let id = friendly_id(8);

    let mut key_hex = None;
    let mut nonce_hex = None;

    let total_bytes = if encrypt {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);

        let mut nonce = [0u8; 19];
        OsRng.fill_bytes(&mut nonce);

        key_hex = Some(hex::encode(key));
        nonce_hex = Some(hex::encode(nonce));

        save_encrypted_file(&id, &key, &nonce, &mut body_reader).await?
    } else {
        save_file(&id, &mut body_reader).await?
    };

    if let Err(why) = update_stats(db, total_bytes as u64).await {
        tracing::error!("failed to update stats: {why:?}");
    }

    let delete_key = friendly_id(21);
    insert_upload(
        db,
        InsertUpload {
            id: id.clone(),
            key_hash: key_hex.as_ref().map(sha256::digest),
            delete_key: delete_key.clone(),
            nonce: nonce_hex,
            file_name,
            bytes: total_bytes,
            expiry_hours,
            expiry_downloads,
        },
    )
    .await?;

    Ok(UploadResponse {
        id,
        decryption_key: key_hex,
        delete_key,
    })
}

pub async fn upload_endpoint(
    ctx: Extension<AppContext>,
    query: Query<UploadQuery>,
    mut multipart: Multipart,
) -> AppResult<Json<UploadResponse>> {
    if let Some(_) = query.expiry_downloads {
        if let Some(_) = query.expiry_hours {
            return Err(AppError::BothExpirations);
        }
    }

    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("file") => (),
            _ => continue,
        };

        let file_name = field
            .file_name()
            .ok_or(AppError::InvalidFileName)?
            .to_string();

        if file_name.is_empty() {
            return Err(AppError::InvalidFileName)?;
        }

        let res = handle_upload(&ctx.db, field, file_name, query.encrypt, query.expiry_hours, query.expiry_downloads).await?;
        return Ok(Json(res));
    }

    Err(AppError::EmptyUpload)
}

#[derive(Deserialize)]
pub struct UploadQuery {
    #[serde(default)]
    pub encrypt: bool,
    pub expiry_hours: Option<u32>,
    pub expiry_downloads: Option<u32>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decryption_key: Option<String>,
    pub delete_key: String,
}
