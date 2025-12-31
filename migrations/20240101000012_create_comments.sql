-- Create comments table for card discussions
CREATE TABLE comments (
    id TEXT PRIMARY KEY NOT NULL,
    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_comments_card ON comments(card_id);
CREATE INDEX idx_comments_user ON comments(user_id);
