CREATE TABLE provision_links (
    id BLOB PRIMARY KEY NOT NULL CHECK(length(id) = 16),
    created_at DATETIME NOT NULL,
    expires_at DATETIME NOT NULL,
    max_uses INTEGER,
    use_count INTEGER NOT NULL DEFAULT 0
);
