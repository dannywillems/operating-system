-- Make board_id optional to support global chat messages
-- SQLite requires table recreation to change column constraints

-- Create new table with optional board_id
CREATE TABLE chat_messages_new (
    id TEXT PRIMARY KEY,
    board_id TEXT REFERENCES boards(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    response TEXT NOT NULL,
    actions_taken TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Copy existing data
INSERT INTO chat_messages_new SELECT * FROM chat_messages;

-- Drop old table
DROP TABLE chat_messages;

-- Rename new table
ALTER TABLE chat_messages_new RENAME TO chat_messages;

-- Recreate indexes
CREATE INDEX idx_chat_messages_board_created ON chat_messages(board_id, created_at DESC);
CREATE INDEX idx_chat_messages_user ON chat_messages(user_id);

-- New index for global messages
CREATE INDEX idx_chat_messages_global ON chat_messages(user_id, created_at DESC) WHERE board_id IS NULL;
