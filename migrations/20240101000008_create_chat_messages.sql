-- Chat messages for LLM conversation history
CREATE TABLE chat_messages (
    id TEXT PRIMARY KEY,
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    response TEXT NOT NULL,
    actions_taken TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_chat_messages_board_created ON chat_messages(board_id, created_at DESC);
CREATE INDEX idx_chat_messages_user ON chat_messages(user_id);
