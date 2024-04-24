#[cfg(test)]
mod tests {
    use std::path::Path;

    use axum::http::StatusCode;
    use axum_test::{
        multipart::{MultipartForm, Part},
        TestServer,
    };
    use sqlx::PgPool;
    use tokio::fs::File;

    use crate::{errors::AppResult, router, upload::UploadResponse, STORAGE_PATH};

    const BASIC_FILE: &[u8] = include_bytes!("./storage/basic");

    type TestResult = anyhow::Result<()>;

    #[sqlx::test]
    async fn upload(db: PgPool) -> TestResult {
        let router = router(db);
        let server = TestServer::new(router)?;

        let multipart_form = MultipartForm::new()
            .add_part("file", Part::bytes(BASIC_FILE).file_name("hello_world.txt"));
        let response = server.post("/upload").multipart(multipart_form).await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let body: UploadResponse = response.json();
        assert!(body.decryption_key.is_none());

        // TODO: remove uploaded file
        Ok(())
    }

    #[sqlx::test]
    async fn upload_encrypted(db: PgPool) -> TestResult {
        let router = router(db);
        let server = TestServer::new(router)?;

        let multipart_form = MultipartForm::new()
            .add_part("file", Part::bytes(BASIC_FILE).file_name("hello_world.txt"));
        let response = server
            .post("/upload")
            .add_query_param("encrypt", true)
            .multipart(multipart_form)
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let body: UploadResponse = response.json();
        assert!(body.decryption_key.is_some());

        // TODO: remove uploaded file
        Ok(())
    }

    #[sqlx::test]
    async fn download(db: PgPool) -> AppResult<()> {
        sqlx::query!(
            "INSERT INTO uploads (id, delete_key, file_name, bytes) VALUES ('basic', '', 'basic', 0)"
        )
        .execute(&db)
        .await?;

        let router = router(db);
        let server = TestServer::new(router)?;

        let response = server.get("/download/basic").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let body = response.as_bytes();
        assert_eq!(body, BASIC_FILE);

        Ok(())
    }

    #[sqlx::test]
    async fn download_encrypted(db: PgPool) -> AppResult<()> {
        sqlx::query!("INSERT INTO uploads (id, key_hash, delete_key, nonce, file_name, bytes) VALUES ('basic_ec', '8882d9c8f120896dd013f528362bac298fc8f14c2f6608c6c5db5fa8e14f2e8e', '', '5561039d74dbe779e061d3731c19d3ca93b92a', 'basic', 0)")
            .execute(&db)
            .await?;

        let router = router(db);
        let server = TestServer::new(router)?;

        let response = server
            .get("/download/basic_ec")
            .add_query_param(
                "key",
                "7888acd752f412fd1736861405041fa0d4be99733715bf7a3185a698454bbeb6",
            )
            .await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let body = response.as_bytes();
        assert_eq!(body, BASIC_FILE);

        Ok(())
    }

    #[sqlx::test]
    async fn download_encrypted_invalid_key(db: PgPool) -> AppResult<()> {
        let router = router(db);
        let server = TestServer::new(router)?;

        let response = server
            .get("/download/basic_ec")
            .add_query_param("key", "abc")
            .await;
        assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[sqlx::test]
    async fn delete_upload(db: PgPool) -> AppResult<()> {
        sqlx::query!("INSERT INTO uploads (id, delete_key, file_name, bytes) VALUES ('useless', 'MOjql910y1nyViKuJvFUx', 'useless', 0)")
            .execute(&db)
            .await?;

        let router = router(db);
        let server = TestServer::new(router)?;

        File::create(format!("{STORAGE_PATH}useless")).await?;

        let response = server
            .delete("/delete/useless")
            .add_query_param("key", "MOjql910y1nyViKuJvFUx")
            .await;

        assert_eq!(response.status_code(), StatusCode::NO_CONTENT);
        assert!(response.text().is_empty());
        assert!(!Path::new("/storage/useless").exists());

        Ok(())
    }
}
