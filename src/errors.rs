use axum::{
    extract::multipart::MultipartError,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use hex::FromHexError;
use serde::Serialize;
use tokio::io;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("You need to upload at least one file.")]
    EmptyUpload,
    #[error("Oops, Looks like the file you tried uploading has invalid name! Change the name and try again.")]
    InvalidFileName,
    #[error("We couldn't find this upload ID! Please re-check and try again.")]
    UploadNotFound,
    #[error("Hmm.. Invalid deletion key, you must've typed that wrong.")]
    InvalidDeleteKey,
    #[error("Invalid decryption key! Please make sure you've entered correct one.")]
    InvalidDecryptionKey,
    #[error("Ouch.. It looks like the uploaded file is corrupted!")]
    CorruptedUpload,
    #[error("This file is encrypted! You need to provide decryption key.")]
    MissingKey,
    #[error("You can only set either expiration hours or expiration downloads! You can't do both at once man ://")]
    BothExpirations,
    #[error("Oops.. Looks like this file expired! What a luck...")]
    UploadExpired,
    #[error("This media file is too big for preview!")]
    MediaTooBig,
    #[error("Preview of this file is not supported yet!")]
    PreviewNotSupported,
    #[error("Failed to upload, file is blacklisted.")]
    FileBlacklisted,
    #[error("Failed to validate your request, {0}")]
    Validation(String),

    #[error("Something went wrong on our side! Please try again later.")]
    Other(#[from] anyhow::Error),
    #[error("Something went wrong on our side! Please try again later.")]
    Crypto(chacha20poly1305::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code = match self {
            Self::UploadExpired => StatusCode::NOT_FOUND,
            Self::PreviewNotSupported => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::Other(_) | Self::Crypto(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };

        let error_code = match self {
            AppError::EmptyUpload => "empty-upload",
            AppError::InvalidFileName => "invalid-file-name",
            AppError::UploadNotFound => "upload-not-found",
            AppError::InvalidDeleteKey => "invalid-delete-key",
            AppError::InvalidDecryptionKey => "invalid-decryption-key",
            AppError::CorruptedUpload => "corrupted-upload",
            AppError::MissingKey => "missing-key",
            AppError::BothExpirations => "both-expirations",
            AppError::UploadExpired => "upload-expired",
            AppError::MediaTooBig => "media-too-big",
            AppError::PreviewNotSupported => "preview-not-supported",
            AppError::FileBlacklisted => "file-blacklist",
            AppError::Validation(_) => "validation",
            AppError::Other(_) | AppError::Crypto(_) => "other",
        };

        // TODO(hito): better error handling, something like color_eyre
        if code == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("{self:?}");
        }

        let res = ErrorResponse {
            error_code: error_code.to_string(),
            error: self.to_string(),
        };
        (code, Json(res)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        Self::Other(value.into())
    }
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Other(value.into())
    }
}

impl From<MultipartError> for AppError {
    fn from(value: MultipartError) -> Self {
        Self::Other(value.into())
    }
}

impl From<FromHexError> for AppError {
    fn from(value: FromHexError) -> Self {
        Self::Other(value.into())
    }
}

impl From<chacha20poly1305::Error> for AppError {
    fn from(value: chacha20poly1305::Error) -> Self {
        Self::Crypto(value)
    }
}

impl From<toml::de::Error> for AppError {
    fn from(value: toml::de::Error) -> Self {
        Self::Other(value.into())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    error_code: String,
    error: String,
}
