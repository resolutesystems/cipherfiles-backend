CREATE TABLE uploads (
    id VARCHAR(8) NOT NULL PRIMARY KEY,
    key_hash VARCHAR(64),
    delete_key VARCHAR(21) NOT NULL,
    nonce VARCHAR(38),
    file_name VARCHAR(255) NOT NULL,
    bytes BIGINT NOT NULL,
    downloads INT NOT NULL DEFAULT 0,
    expiry_hours INT,
    expiry_downloads INT,
    embedded BOOL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
