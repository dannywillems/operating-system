-- Migration: Standalone Cards
-- Cards can now exist independently from boards and be assigned to multiple boards
-- Tags can be global (user-scoped) or board-specific

-- Disable foreign keys during migration
PRAGMA foreign_keys = OFF;

-- ============================================================================
-- 1. Create card_boards junction table (for multi-board card assignments)
-- ============================================================================
CREATE TABLE IF NOT EXISTS card_boards (
    id TEXT PRIMARY KEY NOT NULL,
    card_id TEXT NOT NULL,
    board_id TEXT NOT NULL,
    column_id TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(card_id, board_id)
);

CREATE INDEX IF NOT EXISTS idx_card_boards_card ON card_boards(card_id);
CREATE INDEX IF NOT EXISTS idx_card_boards_board ON card_boards(board_id);
CREATE INDEX IF NOT EXISTS idx_card_boards_column ON card_boards(column_id);

-- ============================================================================
-- 2. Recreate cards table with nullable column_id, status, and owner_id
-- ============================================================================
CREATE TABLE cards_new (
    id TEXT PRIMARY KEY NOT NULL,
    column_id TEXT REFERENCES columns(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    body TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    visibility TEXT NOT NULL DEFAULT 'restricted',
    status TEXT NOT NULL DEFAULT 'open',
    start_date TEXT,
    end_date TEXT,
    due_date TEXT,
    owner_id TEXT REFERENCES users(id),
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Copy existing data, setting owner_id = created_by and status = 'open'
INSERT INTO cards_new (id, column_id, title, body, position, visibility, status, start_date, end_date, due_date, owner_id, created_by, created_at, updated_at)
SELECT id, column_id, title, body, position, visibility, 'open', start_date, end_date, due_date, created_by, created_by, created_at, updated_at
FROM cards;

-- Create card_boards entries for existing cards (from their current column assignments)
INSERT INTO card_boards (id, card_id, board_id, column_id, position, created_at)
SELECT
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab',abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))) as id,
    c.id as card_id,
    col.board_id as board_id,
    c.column_id as column_id,
    c.position as position,
    c.created_at as created_at
FROM cards c
JOIN columns col ON c.column_id = col.id;

-- Drop old table and rename new one
DROP TABLE cards;
ALTER TABLE cards_new RENAME TO cards;

-- Recreate indexes for cards
CREATE INDEX idx_cards_column_id ON cards(column_id);
CREATE INDEX idx_cards_position ON cards(column_id, position);
CREATE INDEX idx_cards_created_by ON cards(created_by);
CREATE INDEX idx_cards_due_date ON cards(due_date);
CREATE INDEX idx_cards_owner_id ON cards(owner_id);
CREATE INDEX idx_cards_status ON cards(status);

-- ============================================================================
-- 3. Recreate tags table with nullable board_id and owner_id
-- ============================================================================
CREATE TABLE tags_new (
    id TEXT PRIMARY KEY NOT NULL,
    board_id TEXT REFERENCES boards(id) ON DELETE CASCADE,
    owner_id TEXT REFERENCES users(id),
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '#6c757d',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (board_id IS NOT NULL OR owner_id IS NOT NULL)
);

-- Copy existing tags, setting owner_id from board owner
INSERT INTO tags_new (id, board_id, owner_id, name, color, created_at)
SELECT t.id, t.board_id, b.owner_id, t.name, t.color, t.created_at
FROM tags t
JOIN boards b ON t.board_id = b.id;

-- Drop old table and rename new one
DROP TABLE tags;
ALTER TABLE tags_new RENAME TO tags;

-- Recreate indexes for tags
CREATE INDEX idx_tags_board_id ON tags(board_id);
CREATE INDEX idx_tags_owner_id ON tags(owner_id);

-- ============================================================================
-- 4. Add foreign key constraints to card_boards (after cards table is ready)
-- ============================================================================
-- SQLite doesn't support adding foreign keys to existing tables,
-- so we rely on application-level enforcement and the indexes above

-- Re-enable foreign keys
PRAGMA foreign_keys = ON;
