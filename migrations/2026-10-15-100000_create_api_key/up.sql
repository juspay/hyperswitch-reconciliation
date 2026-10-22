CREATE TYPE "ApiKeyStatus" AS ENUM ('active', 'revoked');

CREATE TABLE api_key (
    id VARCHAR(64) PRIMARY KEY,
    merchant_id VARCHAR(64) NOT NULL,
    name VARCHAR(64) NOT NULL,
    description VARCHAR(256),
    hashed_api_key VARCHAR(128) NOT NULL UNIQUE,
    prefix VARCHAR(24) NOT NULL,
    created_by VARCHAR(64) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP,
    last_modified_at TIMESTAMP NOT NULL,
    status "ApiKeyStatus" NOT NULL
);

CREATE INDEX idx_api_key_merchant_id_active ON api_key (merchant_id, created_at DESC) WHERE status = 'active';
