use axum::{Extension, Json};
use serde::Serialize;
use tokio::{fs, io};

use crate::{errors::AppResult, AppContext, STORAGE_PATH};

async fn stat_dir(dir: &str) -> io::Result<(u32, u64)> {
    let mut files = 0;
    let mut bytes = 0;

    let mut dir = fs::read_dir(dir).await?;
    loop {
        let entry = dir.next_entry().await?;

        if let Some(entry) = entry {
            if entry.file_name() != ".gitkeep" {
                let metadata = entry.metadata().await?;
                bytes += metadata.len();
                files += 1;
            }
        } else {
            break;
        }
    }

    Ok((files, bytes))
}

pub async fn service_stats(ctx: Extension<AppContext>) -> AppResult<Json<Stats>> {
    let (files, bytes) = stat_dir(STORAGE_PATH).await?;

    let row = sqlx::query!("SELECT * FROM stats WHERE id = 1")
        .fetch_one(&ctx.db)
        .await?;

    Ok(Json(Stats {
        uploads: files,
        bytes,
        total_uploads: row.files_uploaded as u32,
        total_bytes: row.bytes_uploaded as u64,
    }))
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    uploads: u32,
    bytes: u64,
    total_uploads: u32,
    total_bytes: u64,
}
