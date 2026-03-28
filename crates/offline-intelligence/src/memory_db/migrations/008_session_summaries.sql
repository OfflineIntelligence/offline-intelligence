-- Migration 008: Add session summaries table for pre-clear conversation compression

-- Stores a compact summary generated just before each KV cache clear.
-- When the same session continues after a clear, only this summary is re-fed
-- to the model instead of the full conversation history, minimizing recomputation cost.
-- One row per session. Replaced (not appended) on every cache clear.
-- Each replacement accumulates the full conversation history into a single summary.
CREATE TABLE IF NOT EXISTS session_summaries (
    session_id TEXT PRIMARY KEY,
    summary_text TEXT NOT NULL,
    token_count INTEGER NOT NULL DEFAULT 0,
    total_message_count INTEGER NOT NULL DEFAULT 0,
    clear_count INTEGER NOT NULL DEFAULT 1,
    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- session_id is the PRIMARY KEY, so lookup is already O(1) — no extra index needed.
