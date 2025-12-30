-- Create boards table
CREATE TABLE IF NOT EXISTS boards (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_boards_owner_id ON boards(owner_id);

-- Create board permissions table
CREATE TABLE IF NOT EXISTS board_permissions (
    id TEXT PRIMARY KEY NOT NULL,
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'reader',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(board_id, user_id)
);

CREATE INDEX idx_board_permissions_board_id ON board_permissions(board_id);
CREATE INDEX idx_board_permissions_user_id ON board_permissions(user_id);
