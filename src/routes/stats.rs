use axum::{Extension, Json};
use serde::Serialize;

use crate::{errors::AppResult, AppContext};

pub async fn service_stats(ctx: Extension<AppContext>) -> AppResult<Json<Stats>> {
    let row = sqlx::query!("SELECT * FROM stats WHERE id = 1")
        .fetch_one(&ctx.db)
        .await?;

    Ok(Json(Stats {
        uploads: row.files_uploaded as u32,
        bytes: row.bytes_uploaded as u64,
    }))
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    uploads: u32,
    bytes: u64,
}
