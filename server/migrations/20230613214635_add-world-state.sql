CREATE TABLE IF NOT EXISTS world_saves
(
    id         BIGSERIAL PRIMARY KEY,
    state      JSONB, 
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

