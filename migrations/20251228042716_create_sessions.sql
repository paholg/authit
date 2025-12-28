CREATE TABLE sessions (
    id BLOB PRIMARY KEY NOT NULL CHECK(length(id) = 16),
    user_data TEXT NOT NULL
);
