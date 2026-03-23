-- Migration 007: Session file contexts
-- Stores references to files attached during a conversation session.
-- Enables persistent file context: once a file is attached, it stays
-- in the LLM context for the entire conversation (like ChatGPT, Gemini, etc.).

CREATE TABLE IF NOT EXISTS session_file_contexts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    source TEXT NOT NULL,       -- 'inline' or 'local_storage'
    file_path TEXT,             -- OS path (inline only)
    all_files_id INTEGER,       -- DB ID in all_files table (local_storage only)
    size_bytes INTEGER,
    attached_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(session_id, file_name, source)
);

CREATE INDEX IF NOT EXISTS idx_sfc_session_id ON session_file_contexts(session_id);
